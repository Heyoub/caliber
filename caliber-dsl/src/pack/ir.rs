//! Pack IR and validation

use crate::compiler::{
    CompiledInjectionMode, CompiledPackAgentConfig, CompiledPackInjectionConfig,
    CompiledPackRoutingConfig, CompiledToolConfig, CompiledToolKind, CompiledToolsetConfig,
};
use crate::config::*;
use crate::parser::ast::{Action, InjectionMode, MemoryDef, Trigger};
use crate::parser::AdapterDef as AstAdapterDef;
use crate::parser::InjectionDef as AstInjectionDef;
use crate::parser::{AdapterType, CaliberAst, Definition, PolicyDef, PolicyRule};
use crate::parser::{EnvValue, ProviderDef as AstProviderDef, ProviderType};
use std::collections::HashSet;

use super::markdown::{FenceKind, MarkdownDoc};
use super::schema::*;

#[derive(Debug, Clone)]
pub struct PackIr {
    pub manifest: PackManifest,
    pub markdown: Vec<MarkdownDoc>,
    pub adapters: Vec<AstAdapterDef>,
    pub policies: Vec<PolicyDef>,
    pub injections: Vec<AstInjectionDef>,
    pub providers: Vec<AstProviderDef>,
    pub memories: Vec<MemoryDef>,
}

impl PackIr {
    pub fn new(manifest: PackManifest, markdown: Vec<MarkdownDoc>) -> Result<Self, PackError> {
        validate_profiles(&manifest)?;
        validate_toolsets(&manifest)?;
        validate_agents(&manifest, &markdown)?;
        validate_injections(&manifest)?;
        validate_routing(&manifest)?;

        // Build from TOML manifest (legacy)
        let mut adapters = build_adapters(&manifest)?;
        let mut policies = build_policies(&manifest)?;
        let mut injections = build_injections(&manifest)?;
        let mut providers = build_providers(&manifest)?;

        // Extract configs from Markdown fence blocks (NEW)
        let md_adapters = extract_adapters_from_markdown(&markdown)?;
        let md_policies = extract_policies_from_markdown(&markdown)?;
        let md_injections = extract_injections_from_markdown(&markdown)?;
        let md_providers = extract_providers_from_markdown(&markdown)?;
        let md_memories = extract_memories_from_markdown(&markdown)?;

        // Check for duplicates within Markdown configs
        check_markdown_duplicates(&md_adapters, &md_policies, &md_injections, &md_providers)?;

        // Merge: Check for duplicates before merging
        // Default behavior: ERROR on duplicates (no silent override)

        // Check adapter duplicates
        let toml_adapter_names: HashSet<_> = adapters.iter().map(|a| &a.name).collect();
        for md_adapter in &md_adapters {
            if toml_adapter_names.contains(&md_adapter.name) {
                return Err(PackError::Validation(format!(
                    "Duplicate adapter name '{}' found in both TOML and Markdown",
                    md_adapter.name
                )));
            }
        }

        // Check policy duplicates
        let toml_policy_names: HashSet<_> = policies.iter().map(|p| &p.name).collect();
        for md_policy in &md_policies {
            if toml_policy_names.contains(&md_policy.name) {
                return Err(PackError::Validation(format!(
                    "Duplicate policy name '{}' found in both TOML and Markdown",
                    md_policy.name
                )));
            }
        }

        // Check provider duplicates
        let toml_provider_names: HashSet<_> = providers.iter().map(|p| &p.name).collect();
        for md_provider in &md_providers {
            if toml_provider_names.contains(&md_provider.name) {
                return Err(PackError::Validation(format!(
                    "Duplicate provider name '{}' found in both TOML and Markdown",
                    md_provider.name
                )));
            }
        }

        // Check injection duplicates (by source, target tuple since no name field)
        let toml_injection_keys: HashSet<_> =
            injections.iter().map(|i| (&i.source, &i.target)).collect();
        for md_injection in &md_injections {
            let key = (&md_injection.source, &md_injection.target);
            if toml_injection_keys.contains(&key) {
                return Err(PackError::Validation(format!(
                    "Duplicate injection (source: '{}', target: '{}') found in both TOML and Markdown",
                    md_injection.source, md_injection.target
                )));
            }
        }

        // All clear - merge configs
        adapters.extend(md_adapters);
        policies.extend(md_policies);
        injections.extend(md_injections);
        providers.extend(md_providers);

        Ok(Self {
            manifest,
            markdown,
            adapters,
            policies,
            injections,
            providers,
            memories: md_memories,
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
        write!(
            f,
            "{}:{}:{}: {}",
            self.file, self.line, self.column, self.message
        )
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
        allowed.insert(profile_key(
            &p.retention,
            &p.index,
            &p.embeddings,
            &p.format,
        ));
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
    let profiles: HashSet<String> = manifest.profiles.keys().cloned().collect();
    let adapters: HashSet<String> = manifest.adapters.keys().cloned().collect();
    let formats: HashSet<String> = manifest.formats.keys().cloned().collect();
    let md_paths: HashSet<String> = markdown.iter().map(|m| m.file.clone()).collect();

    for (name, agent) in &manifest.agents {
        // Validate profile reference exists
        if !profiles.contains(&agent.profile) {
            return Err(PackError::Validation(format!(
                "agent '{}' references unknown profile '{}'. Available profiles: {:?}",
                name,
                agent.profile,
                profiles.iter().collect::<Vec<_>>()
            )));
        }

        // Validate adapter reference exists (if specified)
        if let Some(ref adapter_name) = agent.adapter {
            if !adapters.contains(adapter_name) {
                return Err(PackError::Validation(format!(
                    "agent '{}' references unknown adapter '{}'. Available adapters: {:?}",
                    name,
                    adapter_name,
                    adapters.iter().collect::<Vec<_>>()
                )));
            }
        }

        // Validate format reference exists (if specified)
        if let Some(ref format_name) = agent.format {
            if !formats.contains(format_name) {
                return Err(PackError::Validation(format!(
                    "agent '{}' references unknown format '{}'. Available formats: {:?}",
                    name,
                    format_name,
                    formats.iter().collect::<Vec<_>>()
                )));
            }
        }

        // Validate token_budget is positive (if specified)
        if let Some(budget) = agent.token_budget {
            if budget <= 0 {
                return Err(PackError::Validation(format!(
                    "agent '{}' has invalid token_budget '{}'. Must be greater than 0.",
                    name, budget
                )));
            }
        }

        // Validate toolset references
        for toolset in &agent.toolsets {
            if !toolsets.contains(toolset) {
                return Err(PackError::Validation(format!(
                    "agent '{}' references unknown toolset '{}'",
                    name, toolset
                )));
            }
        }

        // Validate prompt markdown path
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

/// Maximum priority allowed for pack injections.
/// Priorities 900+ are reserved for platform-level injections.
const MAX_PACK_INJECTION_PRIORITY: i32 = 899;

fn validate_injections(manifest: &PackManifest) -> Result<(), PackError> {
    for (name, injection) in &manifest.injections {
        // Validate entity type if specified
        if let Some(entity_type) = injection.entity_type.as_deref() {
            let normalized = entity_type.to_lowercase();
            let valid = matches!(
                normalized.as_str(),
                "note" | "notes" | "artifact" | "artifacts"
            );
            if !valid {
                return Err(PackError::Validation(format!(
                    "injections.{}: invalid entity_type '{}' (expected 'note' or 'artifact')",
                    name, entity_type
                )));
            }
        }

        // Validate priority is within pack range (0-899)
        if injection.priority > MAX_PACK_INJECTION_PRIORITY {
            return Err(PackError::Validation(format!(
                "injections.{}: priority {} exceeds pack maximum ({}). Priorities {}+ are reserved for platform.",
                name, injection.priority, MAX_PACK_INJECTION_PRIORITY, MAX_PACK_INJECTION_PRIORITY + 1
            )));
        }
    }
    Ok(())
}

fn validate_routing(manifest: &PackManifest) -> Result<(), PackError> {
    let Some(routing) = manifest.routing.as_ref() else {
        return Ok(());
    };

    if let Some(strategy) = routing.strategy.as_deref() {
        let strategy = strategy.to_lowercase();
        let valid = matches!(
            strategy.as_str(),
            "first" | "round_robin" | "roundrobin" | "random" | "least_latency" | "leastlatency"
        );
        if !valid {
            return Err(PackError::Validation(format!(
                "routing.strategy: invalid value '{}' (expected first|round_robin|random|least_latency)",
                strategy
            )));
        }
    }

    if let Some(provider) = routing.embedding_provider.as_deref() {
        if !manifest.providers.contains_key(provider) {
            return Err(PackError::Validation(format!(
                "routing.embedding_provider: unknown provider '{}'",
                provider
            )));
        }
    }

    if let Some(provider) = routing.summarization_provider.as_deref() {
        if !manifest.providers.contains_key(provider) {
            return Err(PackError::Validation(format!(
                "routing.summarization_provider: unknown provider '{}'",
                provider
            )));
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

/// Contract files loaded from the pack for schema compilation.
pub type ContractFiles = std::collections::HashMap<String, String>;

/// Validate that a tool command is a safe executable path.
///
/// Commands must be:
/// - A relative path starting with `./` or an absolute path starting with `/`
/// - Free of shell metacharacters that could enable injection
/// - Free of path traversal sequences (`..`)
///
/// This prevents shell injection attacks like `cmd = "rm -rf / && echo hacked"`
/// and path traversal attacks like `cmd = "./tools/../../../bin/sh"`.
fn is_valid_executable_path(cmd: &str) -> bool {
    // Forbidden shell metacharacters
    const FORBIDDEN: &[char] = &[
        ';', '|', '&', '$', '`', '(', ')', '{', '}', '<', '>', '!', '\n', '\r',
    ];

    // Must not contain forbidden characters
    if cmd.chars().any(|c| FORBIDDEN.contains(&c)) {
        return false;
    }

    // Block path traversal attempts (e.g., "../../../etc/passwd")
    if cmd.contains("..") {
        return false;
    }

    // Must be a path (relative with ./ or absolute with /)
    // This prevents bare commands like "bash" or "rm"
    cmd.starts_with("./") || cmd.starts_with("/")
}

/// Compile pack tool registry into runtime tool configs.
/// `contracts` maps contract paths to their JSON content.
pub fn compile_tools(
    manifest: &PackManifest,
    contracts: &ContractFiles,
) -> Result<Vec<CompiledToolConfig>, PackError> {
    let mut tools = Vec::new();

    for (name, def) in &manifest.tools.bin {
        // Validate command is a safe executable path, not arbitrary shell
        if !is_valid_executable_path(&def.cmd) {
            return Err(PackError::Validation(format!(
                "tools.bin.{}: cmd must be a path to executable (starting with ./ or /), \
                and must not contain shell metacharacters. Got: '{}'",
                name, def.cmd
            )));
        }

        tools.push(CompiledToolConfig {
            id: format!("tools.bin.{}", name),
            kind: CompiledToolKind::Exec,
            cmd: Some(def.cmd.clone()),
            prompt_md: None,
            contract: None,
            compiled_schema: None,
            result_format: None,
            timeout_ms: def.timeout_ms,
            allow_network: def.allow_network,
            allow_fs: def.allow_fs,
            allow_subprocess: def.allow_subprocess,
        });
    }

    for (name, def) in &manifest.tools.prompts {
        // If contract specified, compile the schema
        let compiled_schema = if let Some(contract_path) = &def.contract {
            let json_str = contracts.get(contract_path).ok_or_else(|| {
                PackError::Validation(format!(
                    "tools.prompts.{}: contract file '{}' not found",
                    name, contract_path
                ))
            })?;
            let schema: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
                PackError::Validation(format!(
                    "tools.prompts.{}: contract '{}' is invalid JSON: {}",
                    name, contract_path, e
                ))
            })?;
            Some(schema)
        } else {
            None
        };

        tools.push(CompiledToolConfig {
            id: format!("tools.prompts.{}", name),
            kind: CompiledToolKind::Prompt,
            cmd: None,
            prompt_md: Some(def.prompt_md.clone()),
            contract: def.contract.clone(),
            compiled_schema,
            result_format: def.result_format.clone(),
            timeout_ms: def.timeout_ms,
            allow_network: None,
            allow_fs: None,
            allow_subprocess: None,
        });
    }

    Ok(tools)
}

/// Compile pack toolsets into runtime toolset configs.
pub fn compile_toolsets(manifest: &PackManifest) -> Vec<CompiledToolsetConfig> {
    manifest
        .toolsets
        .iter()
        .map(|(name, set)| CompiledToolsetConfig {
            name: name.clone(),
            tools: set.tools.clone(),
        })
        .collect()
}

/// Compile pack agent bindings to toolsets with extracted markdown metadata.
///
/// This function transforms manifest agent definitions into runtime-ready
/// configurations, combining TOML settings with markdown-extracted metadata.
/// All reference validations (profile, adapter, format, toolsets) have been
/// performed during the IR validation phase.
pub fn compile_pack_agents(
    manifest: &PackManifest,
    markdown_docs: &[super::MarkdownDoc],
) -> Vec<CompiledPackAgentConfig> {
    // Build lookup from markdown file path to extracted data
    let md_by_path: std::collections::HashMap<&str, &super::MarkdownDoc> =
        markdown_docs.iter().map(|m| (m.file.as_str(), m)).collect();

    manifest
        .agents
        .iter()
        .map(|(name, agent)| {
            // Find matching markdown doc by prompt_md path
            let md: Option<&super::MarkdownDoc> = md_by_path
                .get(agent.prompt_md.as_str())
                .copied()
                .or_else(|| {
                    // Try suffix match for relative paths
                    md_by_path
                        .iter()
                        .find(|(path, _)| path.ends_with(&agent.prompt_md))
                        .map(|(_, doc)| *doc)
                });

            let (constraints, tool_refs, rag_config) = match md {
                Some(doc) => (
                    doc.extracted_constraints.clone(),
                    doc.extracted_tool_refs.clone(),
                    doc.extracted_rag_config.clone(),
                ),
                None => (Vec::new(), Vec::new(), None),
            };

            // PROFILE INHERITANCE: Resolve format from agent or profile
            let profile_def = manifest.profiles.get(&agent.profile);
            let resolved_format = agent
                .format
                .clone()
                .or_else(|| profile_def.map(|p| p.format.clone()))
                .unwrap_or_else(|| "markdown".to_string());

            CompiledPackAgentConfig {
                name: name.clone(),
                enabled: agent.enabled.unwrap_or(true),
                profile: agent.profile.clone(),
                adapter: agent.adapter.clone(),
                format: agent.format.clone(),
                resolved_format,
                token_budget: agent.token_budget,
                prompt_md: agent.prompt_md.clone(),
                toolsets: agent.toolsets.clone(),
                extracted_constraints: constraints,
                extracted_tool_refs: tool_refs,
                extracted_rag_config: rag_config,
            }
        })
        .collect()
}

/// Compile pack injection metadata for runtime wiring.
pub fn compile_pack_injections(
    manifest: &PackManifest,
) -> Result<Vec<CompiledPackInjectionConfig>, PackError> {
    let mut out = Vec::new();
    for def in manifest.injections.values() {
        let mode = compile_injection_mode_compiled(def)?;
        out.push(CompiledPackInjectionConfig {
            source: def.source.clone(),
            target: def.target.clone(),
            entity_type: def.entity_type.clone().map(|s| s.to_lowercase()),
            mode,
            priority: def.priority,
            max_tokens: def.max_tokens,
        });
    }
    Ok(out)
}

/// Compile pack provider routing hints.
pub fn compile_pack_routing(manifest: &PackManifest) -> Option<CompiledPackRoutingConfig> {
    manifest
        .routing
        .as_ref()
        .map(|routing| CompiledPackRoutingConfig {
            strategy: routing.strategy.clone().map(|s| s.to_lowercase()),
            embedding_provider: routing.embedding_provider.clone(),
            summarization_provider: routing.summarization_provider.clone(),
        })
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

fn build_providers(manifest: &PackManifest) -> Result<Vec<AstProviderDef>, PackError> {
    let mut providers = Vec::new();
    for (name, def) in &manifest.providers {
        let provider_type = match def.provider_type.to_lowercase().as_str() {
            "openai" => ProviderType::OpenAI,
            "anthropic" => ProviderType::Anthropic,
            "custom" => ProviderType::Custom,
            other => {
                return Err(PackError::Validation(format!(
                    "provider '{}' has invalid type '{}'",
                    name, other
                )))
            }
        };

        let api_key = parse_env_value(&def.api_key);
        let options = def
            .options
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();

        providers.push(AstProviderDef {
            name: name.clone(),
            provider_type,
            api_key,
            model: def.model.clone(),
            options,
        });
    }
    Ok(providers)
}

fn parse_env_value(value: &str) -> EnvValue {
    if let Some(rest) = value.strip_prefix("env:") {
        EnvValue::Env(rest.trim().to_string())
    } else {
        EnvValue::Literal(value.to_string())
    }
}

fn parse_trigger(value: &str) -> Result<Trigger, PackError> {
    match value.to_lowercase().as_str() {
        "task_start" => Ok(Trigger::TaskStart),
        "task_end" => Ok(Trigger::TaskEnd),
        "scope_close" => Ok(Trigger::ScopeClose),
        "turn_end" => Ok(Trigger::TurnEnd),
        "manual" => Ok(Trigger::Manual),
        other if other.starts_with("schedule:") => Ok(Trigger::Schedule(
            other["schedule:".len()..].trim().to_string(),
        )),
        other => Err(PackError::Validation(format!(
            "invalid trigger '{}'",
            other
        ))),
    }
}

fn parse_action(action: &PolicyActionDef) -> Result<Action, PackError> {
    let typ = action.action_type.to_lowercase();
    match typ.as_str() {
        "summarize" => Ok(Action::Summarize(action.target.clone().ok_or_else(
            || PackError::Validation("summarize action missing target".into()),
        )?)),
        "checkpoint" => Ok(Action::Checkpoint(action.target.clone().ok_or_else(
            || PackError::Validation("checkpoint action missing target".into()),
        )?)),
        "extract_artifacts" => Ok(Action::ExtractArtifacts(action.target.clone().ok_or_else(
            || PackError::Validation("extract_artifacts action missing target".into()),
        )?)),
        "notify" => Ok(Action::Notify(action.target.clone().ok_or_else(|| {
            PackError::Validation("notify action missing target".into())
        })?)),
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

fn compile_injection_mode_compiled(def: &InjectionDef) -> Result<CompiledInjectionMode, PackError> {
    match def.mode.to_lowercase().as_str() {
        "full" => Ok(CompiledInjectionMode::Full),
        "summary" => Ok(CompiledInjectionMode::Summary),
        "topk" => {
            let k = def
                .top_k
                .ok_or_else(|| PackError::Validation("topk mode requires top_k".into()))?;
            let k =
                i32::try_from(k).map_err(|_| PackError::Validation("top_k out of range".into()))?;
            Ok(CompiledInjectionMode::TopK { k })
        }
        "relevant" => {
            let threshold = def
                .threshold
                .ok_or_else(|| PackError::Validation("relevant mode requires threshold".into()))?;
            Ok(CompiledInjectionMode::Relevant { threshold })
        }
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
    for provider in &ir.providers {
        defs.push(Definition::Provider(provider.clone()));
    }
    for memory in &ir.memories {
        defs.push(Definition::Memory(memory.clone()));
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

// ============================================================================
// MARKDOWN CONFIG EXTRACTION (NEW)
// ============================================================================

/// Check for duplicate definitions within Markdown configs
fn check_markdown_duplicates(
    adapters: &[AstAdapterDef],
    policies: &[PolicyDef],
    injections: &[AstInjectionDef],
    providers: &[AstProviderDef],
) -> Result<(), PackError> {
    // Check for duplicate adapter names
    let mut adapter_names = HashSet::new();
    for adapter in adapters {
        if !adapter_names.insert(&adapter.name) {
            return Err(PackError::Validation(format!(
                "Duplicate adapter name '{}' found in Markdown configs",
                adapter.name
            )));
        }
    }

    // Check for duplicate policy names
    let mut policy_names = HashSet::new();
    for policy in policies {
        if !policy_names.insert(&policy.name) {
            return Err(PackError::Validation(format!(
                "Duplicate policy name '{}' found in Markdown configs",
                policy.name
            )));
        }
    }

    // Check for duplicate provider names
    let mut provider_names = HashSet::new();
    for provider in providers {
        if !provider_names.insert(&provider.name) {
            return Err(PackError::Validation(format!(
                "Duplicate provider name '{}' found in Markdown configs",
                provider.name
            )));
        }
    }

    // Check for duplicate injection (source, target) tuples
    let mut injection_keys = HashSet::new();
    for injection in injections {
        let key = (&injection.source, &injection.target);
        if !injection_keys.insert(key) {
            return Err(PackError::Validation(format!(
                "Duplicate injection (source: '{}', target: '{}') found in Markdown configs",
                injection.source, injection.target
            )));
        }
    }

    Ok(())
}

/// Extract adapter definitions from Markdown fence blocks
fn extract_adapters_from_markdown(markdown: &[MarkdownDoc]) -> Result<Vec<AstAdapterDef>, PackError> {
    let mut adapters = Vec::new();

    for doc in markdown {
        for user in &doc.users {
            for block in &user.blocks {
                if block.kind == FenceKind::Adapter {
                    let adapter = parse_adapter_block(
                        block.header_name.clone(),
                        &block.content,
                    )?;
                    adapters.push(adapter);
                }
            }
        }
    }

    Ok(adapters)
}

/// Extract policy definitions from Markdown fence blocks
fn extract_policies_from_markdown(markdown: &[MarkdownDoc]) -> Result<Vec<PolicyDef>, PackError> {
    let mut policies = Vec::new();

    for doc in markdown {
        for user in &doc.users {
            for block in &user.blocks {
                if block.kind == FenceKind::Policy {
                    let policy = parse_policy_block(
                        block.header_name.clone(),
                        &block.content,
                    )?;
                    policies.push(policy);
                }
            }
        }
    }

    Ok(policies)
}

/// Extract injection definitions from Markdown fence blocks
fn extract_injections_from_markdown(markdown: &[MarkdownDoc]) -> Result<Vec<AstInjectionDef>, PackError> {
    let mut injections = Vec::new();

    for doc in markdown {
        for user in &doc.users {
            for block in &user.blocks {
                if block.kind == FenceKind::Injection {
                    let injection = parse_injection_block(
                        block.header_name.clone(),
                        &block.content,
                    )?;
                    injections.push(injection);
                }
            }
        }
    }

    Ok(injections)
}

/// Extract provider definitions from Markdown fence blocks
fn extract_providers_from_markdown(markdown: &[MarkdownDoc]) -> Result<Vec<AstProviderDef>, PackError> {
    let mut providers = Vec::new();

    for doc in markdown {
        for user in &doc.users {
            for block in &user.blocks {
                if block.kind == FenceKind::Provider {
                    let provider = parse_provider_block(
                        block.header_name.clone(),
                        &block.content,
                    )?;
                    providers.push(provider);
                }
            }
        }
    }

    Ok(providers)
}

/// Extract memory definitions from Markdown fence blocks
fn extract_memories_from_markdown(markdown: &[MarkdownDoc]) -> Result<Vec<MemoryDef>, PackError> {
    let mut memories = Vec::new();

    for doc in markdown {
        for user in &doc.users {
            for block in &user.blocks {
                if block.kind == FenceKind::Memory {
                    let memory = parse_memory_block(
                        block.header_name.clone(),
                        &block.content,
                    )?;
                    memories.push(memory);
                }
            }
        }
    }

    Ok(memories)
}
