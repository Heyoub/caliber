//! Pack IR and validation

use crate::parser::ast::{Action, InjectionMode, Trigger};
use crate::parser::{AdapterType, CaliberAst, Definition, PolicyDef, PolicyRule};
use crate::parser::{InjectionDef as AstInjectionDef};
use crate::parser::{AdapterDef as AstAdapterDef};
use std::collections::HashSet;

use super::markdown::MarkdownDoc;
use super::schema::*;

#[derive(Debug, Clone)]
pub struct PackIr {
    pub manifest: PackManifest,
    pub markdown: Vec<MarkdownDoc>,
    pub adapters: Vec<AstAdapterDef>,
    pub policies: Vec<PolicyDef>,
    pub injections: Vec<AstInjectionDef>,
}

impl PackIr {
    pub fn new(manifest: PackManifest, markdown: Vec<MarkdownDoc>) -> Result<Self, PackError> {
        validate_profiles(&manifest)?;
        validate_toolsets(&manifest)?;
        validate_agents(&manifest, &markdown)?;
        let adapters = build_adapters(&manifest)?;
        let policies = build_policies(&manifest)?;
        let injections = build_injections(&manifest)?;
        Ok(Self {
            manifest,
            markdown,
            adapters,
            policies,
            injections,
        })
    }
}

#[derive(Debug, Clone)]
pub enum PackError {
    Toml(String),
    Validation(String),
    Markdown(MarkdownError),
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::Toml(msg) => write!(f, "TOML error: {}", msg),
            PackError::Validation(msg) => write!(f, "Validation error: {}", msg),
            PackError::Markdown(err) => write!(f, "Markdown error: {}", err),
        }
    }
}

impl std::error::Error for PackError {}

impl From<MarkdownError> for PackError {
    fn from(err: MarkdownError) -> Self {
        PackError::Markdown(err)
    }
}

impl From<crate::compiler::CompileError> for PackError {
    fn from(err: crate::compiler::CompileError) -> Self {
        PackError::Validation(err.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct MarkdownError {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl std::fmt::Display for MarkdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}: {}", self.file, self.line, self.column, self.message)
    }
}

fn validate_profiles(manifest: &PackManifest) -> Result<(), PackError> {
    let Some(settings) = &manifest.settings else {
        return Ok(());
    };
    let Some(matrix) = &settings.matrix else {
        return Ok(());
    };
    let mut allowed = HashSet::new();
    for p in &matrix.allowed {
        allowed.insert(profile_key(&p.retention, &p.index, &p.embeddings, &p.format));
    }
    for (name, p) in &manifest.profiles {
        let key = profile_key(&p.retention, &p.index, &p.embeddings, &p.format);
        if !allowed.contains(&key) {
            return Err(PackError::Validation(format!(
                "profile '{}' does not satisfy settings.matrix.allowed",
                name
            )));
        }
    }
    Ok(())
}

fn validate_toolsets(manifest: &PackManifest) -> Result<(), PackError> {
    let tool_ids = collect_tool_ids(&manifest.tools);
    for (set_name, set) in &manifest.toolsets {
        for tool in &set.tools {
            if !tool_ids.contains(tool) {
                return Err(PackError::Validation(format!(
                    "toolset '{}' references unknown tool id '{}'",
                    set_name, tool
                )));
            }
        }
    }
    Ok(())
}

fn validate_agents(manifest: &PackManifest, markdown: &[MarkdownDoc]) -> Result<(), PackError> {
    let toolsets: HashSet<String> = manifest.toolsets.keys().cloned().collect();
    let md_paths: HashSet<String> = markdown.iter().map(|m| m.file.clone()).collect();
    for (name, agent) in &manifest.agents {
        for toolset in &agent.toolsets {
            if !toolsets.contains(toolset) {
                return Err(PackError::Validation(format!(
                    "agent '{}' references unknown toolset '{}'",
                    name, toolset
                )));
            }
        }
        if !md_paths.contains(&agent.prompt_md) {
            // allow relative path match by suffix
            let found = md_paths.iter().any(|p| p.ends_with(&agent.prompt_md));
            if !found {
                return Err(PackError::Validation(format!(
                    "agent '{}' prompt_md '{}' not found in pack markdowns",
                    name, agent.prompt_md
                )));
            }
        }
    }
    Ok(())
}

fn collect_tool_ids(tools: &ToolsSection) -> HashSet<String> {
    let mut ids = HashSet::new();
    for name in tools.bin.keys() {
        ids.insert(format!("tools.bin.{}", name));
    }
    for name in tools.prompts.keys() {
        ids.insert(format!("tools.prompts.{}", name));
    }
    ids
}

fn build_adapters(manifest: &PackManifest) -> Result<Vec<AstAdapterDef>, PackError> {
    let mut adapters = Vec::new();
    for (name, def) in &manifest.adapters {
        let adapter_type = match def.adapter_type.to_lowercase().as_str() {
            "postgres" => AdapterType::Postgres,
            "redis" => AdapterType::Redis,
            "memory" => AdapterType::Memory,
            other => {
                return Err(PackError::Validation(format!(
                    "adapter '{}' has invalid type '{}'",
                    name, other
                )))
            }
        };
        let options = def
            .options
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        adapters.push(AstAdapterDef {
            name: name.clone(),
            adapter_type,
            connection: def.connection.clone(),
            options,
        });
    }
    Ok(adapters)
}

fn build_policies(manifest: &PackManifest) -> Result<Vec<PolicyDef>, PackError> {
    let mut policies = Vec::new();
    for (name, def) in &manifest.policies {
        let trigger = parse_trigger(&def.trigger)?;
        let mut actions = Vec::new();
        for action in &def.actions {
            actions.push(parse_action(action)?);
        }
        policies.push(PolicyDef {
            name: name.clone(),
            rules: vec![PolicyRule { trigger, actions }],
        });
    }
    Ok(policies)
}

fn build_injections(manifest: &PackManifest) -> Result<Vec<AstInjectionDef>, PackError> {
    let mut injections = Vec::new();
    for def in manifest.injections.values() {
        let mode = parse_injection_mode(def)?;
        injections.push(AstInjectionDef {
            source: def.source.clone(),
            target: def.target.clone(),
            mode,
            priority: def.priority,
            max_tokens: def.max_tokens,
            filter: None,
        });
    }
    Ok(injections)
}

fn parse_trigger(value: &str) -> Result<Trigger, PackError> {
    match value.to_lowercase().as_str() {
        "task_start" => Ok(Trigger::TaskStart),
        "task_end" => Ok(Trigger::TaskEnd),
        "scope_close" => Ok(Trigger::ScopeClose),
        "turn_end" => Ok(Trigger::TurnEnd),
        "manual" => Ok(Trigger::Manual),
        other if other.starts_with("schedule:") => {
            Ok(Trigger::Schedule(other["schedule:".len()..].trim().to_string()))
        }
        other => Err(PackError::Validation(format!(
            "invalid trigger '{}'",
            other
        ))),
    }
}

fn parse_action(action: &PolicyActionDef) -> Result<Action, PackError> {
    let typ = action.action_type.to_lowercase();
    match typ.as_str() {
        "summarize" => Ok(Action::Summarize(
            action
                .target
                .clone()
                .ok_or_else(|| PackError::Validation("summarize action missing target".into()))?,
        )),
        "checkpoint" => Ok(Action::Checkpoint(
            action
                .target
                .clone()
                .ok_or_else(|| PackError::Validation("checkpoint action missing target".into()))?,
        )),
        "extract_artifacts" => Ok(Action::ExtractArtifacts(
            action
                .target
                .clone()
                .ok_or_else(|| PackError::Validation("extract_artifacts action missing target".into()))?,
        )),
        "notify" => Ok(Action::Notify(
            action
                .target
                .clone()
                .ok_or_else(|| PackError::Validation("notify action missing target".into()))?,
        )),
        "inject" => Ok(Action::Inject {
            target: action
                .target
                .clone()
                .ok_or_else(|| PackError::Validation("inject action missing target".into()))?,
            mode: InjectionMode::Full,
        }),
        other => Err(PackError::Validation(format!(
            "unsupported action type '{}'",
            other
        ))),
    }
}

fn parse_injection_mode(def: &InjectionDef) -> Result<InjectionMode, PackError> {
    match def.mode.to_lowercase().as_str() {
        "full" => Ok(InjectionMode::Full),
        "summary" => Ok(InjectionMode::Summary),
        "topk" => Ok(InjectionMode::TopK(def.top_k.ok_or_else(|| {
            PackError::Validation("topk mode requires top_k".into())
        })?)),
        "relevant" => Ok(InjectionMode::Relevant(def.threshold.ok_or_else(|| {
            PackError::Validation("relevant mode requires threshold".into())
        })?)),
        other => Err(PackError::Validation(format!(
            "invalid injection mode '{}'",
            other
        ))),
    }
}

fn profile_key(ret: &str, idx: &str, emb: &str, fmt: &str) -> String {
    format!(
        "{}|{}|{}|{}",
        ret.to_lowercase(),
        idx.to_lowercase(),
        emb.to_lowercase(),
        fmt.to_lowercase()
    )
}

pub fn ast_from_ir(ir: &PackIr) -> CaliberAst {
    let mut defs: Vec<Definition> = Vec::new();
    for a in &ir.adapters {
        defs.push(Definition::Adapter(a.clone()));
    }
    for p in &ir.policies {
        defs.push(Definition::Policy(p.clone()));
    }
    for i in &ir.injections {
        defs.push(Definition::Injection(i.clone()));
    }
    CaliberAst {
        version: ir
            .manifest
            .meta
            .as_ref()
            .and_then(|m| m.version.clone())
            .unwrap_or_else(|| "1.0".to_string()),
        definitions: defs,
    }
}
