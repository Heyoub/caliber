//! Pack manifest schema (cal.toml)

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    pub meta: Option<MetaSection>,
    pub defaults: Option<DefaultsSection>,
    pub settings: Option<SettingsSection>,
    #[serde(default)]
    pub profiles: HashMap<String, ProfileDef>,
    #[serde(default)]
    pub adapters: HashMap<String, AdapterDef>,
    #[serde(default)]
    pub formats: HashMap<String, FormatDef>,
    #[serde(default)]
    pub policies: HashMap<String, PolicyDef>,
    #[serde(default)]
    pub injections: HashMap<String, InjectionDef>,
    #[serde(default)]
    pub tools: ToolsSection,
    #[serde(default)]
    pub toolsets: HashMap<String, ToolsetDef>,
    #[serde(default)]
    pub agents: HashMap<String, AgentDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetaSection {
    pub version: Option<String>,
    pub project: Option<String>,
    pub env: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultsSection {
    pub context_format: Option<String>,
    pub token_budget: Option<i32>,
    pub strict_markdown: Option<bool>,
    pub strict_refs: Option<bool>,
    pub secrets_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsSection {
    pub matrix: Option<SettingsMatrix>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsMatrix {
    pub allowed: Vec<ProfileBinding>,
    #[serde(default)]
    pub enforce_profiles_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileBinding {
    pub name: String,
    pub retention: String,
    pub index: String,
    pub embeddings: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDef {
    pub retention: String,
    pub index: String,
    pub embeddings: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterDef {
    #[serde(rename = "type")]
    pub adapter_type: String,
    pub connection: String,
    #[serde(default)]
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormatDef {
    #[serde(rename = "type")]
    pub format_type: String,
    pub include_audit: Option<bool>,
    pub include_sources: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDef {
    pub trigger: String,
    #[serde(default)]
    pub actions: Vec<PolicyActionDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyActionDef {
    #[serde(rename = "type")]
    pub action_type: String,
    pub target: Option<String>,
    pub max_tokens: Option<i32>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InjectionDef {
    pub source: String,
    pub target: String,
    pub mode: String,
    #[serde(default)]
    pub priority: i32,
    pub max_tokens: Option<i32>,
    pub top_k: Option<usize>,
    pub threshold: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ToolsSection {
    #[serde(default)]
    pub bin: HashMap<String, ToolExecDef>,
    #[serde(default)]
    pub prompts: HashMap<String, ToolPromptDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolExecDef {
    #[serde(rename = "kind")]
    pub kind: Option<String>,
    pub cmd: String,
    pub timeout_ms: Option<i32>,
    pub allow_network: Option<bool>,
    pub allow_fs: Option<bool>,
    pub allow_subprocess: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolPromptDef {
    #[serde(rename = "kind")]
    pub kind: Option<String>,
    pub prompt_md: String,
    pub contract: Option<String>,
    pub result_format: Option<String>,
    pub timeout_ms: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolsetDef {
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDef {
    pub enabled: Option<bool>,
    pub profile: String,
    pub adapter: Option<String>,
    pub format: Option<String>,
    pub token_budget: Option<i32>,
    pub prompt_md: String,
    #[serde(default)]
    pub toolsets: Vec<String>,
}

pub fn parse_manifest(toml_source: &str) -> Result<PackManifest, PackError> {
    toml::from_str(toml_source).map_err(|e| PackError::Toml(e.to_string()))
}

// Error lives in ir.rs
use super::ir::PackError;
