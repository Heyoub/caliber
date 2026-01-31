//! Config parser for Markdown fence blocks
//! Uses serde_yaml for ALL parsing (no custom mini-syntax)

use crate::pack::PackError;
use crate::parser::ast::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Debug)]
pub enum ConfigError {
    YamlParse(String),
    NameConflict(String),
    MissingName(String),
    InvalidValue(String),
    UnknownProvider(String),
    UnknownAdapter(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::YamlParse(msg) => write!(f, "YAML parse error: {}", msg),
            ConfigError::NameConflict(msg) => write!(f, "Name conflict: {}", msg),
            ConfigError::MissingName(msg) => write!(f, "Missing name: {}", msg),
            ConfigError::InvalidValue(msg) => write!(f, "Invalid value: {}", msg),
            ConfigError::UnknownProvider(msg) => write!(f, "Unknown provider: {}", msg),
            ConfigError::UnknownAdapter(msg) => write!(f, "Unknown adapter: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<ConfigError> for PackError {
    fn from(err: ConfigError) -> Self {
        PackError::Validation(err.to_string())
    }
}

// ============================================================================
// CONFIG STRUCTS (The Schema)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AdapterConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub adapter_type: String,
    pub connection: String,
    #[serde(default)]
    pub options: Vec<(String, String)>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MemoryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub memory_type: String,
    #[serde(default)]
    pub schema: Vec<FieldConfig>,
    pub retention: String,
    pub lifecycle: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub indexes: Vec<IndexConfig>,
    #[serde(default)]
    pub inject_on: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FieldConfig {
    pub name: String,
    /// Field type (accepts both "field_type" and "type" for compatibility)
    #[serde(alias = "type")]
    pub field_type: String,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IndexConfig {
    pub field: String,
    /// Index type (accepts both "index_type" and "type" for compatibility)
    #[serde(alias = "type")]
    pub index_type: String,
    #[serde(default)]
    pub options: Vec<(String, String)>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub rules: Vec<PolicyRuleConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicyRuleConfig {
    pub trigger: String,
    pub actions: Vec<ActionConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum ActionConfig {
    #[serde(rename = "summarize")]
    Summarize { target: String },
    #[serde(rename = "checkpoint")]
    Checkpoint { target: String },
    #[serde(rename = "extract_artifacts")]
    ExtractArtifacts { target: String },
    #[serde(rename = "notify")]
    Notify { target: String },
    #[serde(rename = "inject")]
    Inject { target: String, mode: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InjectionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub source: String,
    pub target: String,
    pub mode: String,
    pub priority: i32,
    #[serde(default)]
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub provider_type: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub options: Vec<(String, String)>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CacheConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub backend: String,
    #[serde(default)]
    pub path: Option<String>,
    pub size_mb: i32,
    pub default_freshness: FreshnessConfig,
    #[serde(default)]
    pub max_entries: Option<i32>,
    #[serde(default)]
    pub ttl: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum FreshnessConfig {
    #[serde(rename = "best_effort")]
    BestEffort { max_staleness: String },
    #[serde(rename = "strict")]
    Strict,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TrajectoryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub agent_type: String,
    pub token_budget: i32,
    #[serde(default)]
    pub memory_refs: Vec<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub capabilities: Vec<String>,
    pub constraints: AgentConstraintsConfig,
    pub permissions: PermissionMatrixConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentConstraintsConfig {
    pub max_concurrent: i32,
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct PermissionMatrixConfig {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub write: Vec<String>,
    #[serde(default)]
    pub lock: Vec<String>,
}

// ============================================================================
// PARSER FUNCTIONS (serde_yaml does the heavy lifting)
// ============================================================================

pub fn parse_adapter_block(
    header_name: Option<String>,
    content: &str,
) -> Result<AdapterDef, ConfigError> {
    // Step 1: Deserialize YAML into config struct
    let config: AdapterConfig = serde_yaml::from_str(content)
        .map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    // Step 2: Enforce name precedence rule
    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict(
                "Name in both fence header and YAML payload".to_string(),
            ));
        }
        (None, Some(payload_name)) => payload_name.clone(),
        (None, None) => {
            return Err(ConfigError::MissingName(
                "Adapter requires name".to_string(),
            ));
        }
    };

    // Step 3: Validate and convert to AST
    validate_adapter_config(&config)?;

    let adapter_type = parse_adapter_type(&config.adapter_type)?;

    Ok(AdapterDef {
        name,
        adapter_type,
        connection: config.connection,
        options: config.options,
    })
}

pub fn parse_memory_block(
    header_name: Option<String>,
    content: &str,
) -> Result<MemoryDef, ConfigError> {
    let config: MemoryConfig = serde_yaml::from_str(content)
        .map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict(
                "Name conflict".to_string(),
            ));
        }
        (None, Some(payload_name)) => payload_name.clone(),
        (None, None) => {
            return Err(ConfigError::MissingName(
                "Memory requires name".to_string(),
            ));
        }
    };

    // Convert to AST
    let memory_type = parse_memory_type(&config.memory_type)?;
    let retention = parse_retention(&config.retention)?;
    let lifecycle = parse_lifecycle(&config.lifecycle)?;
    let schema = config
        .schema
        .into_iter()
        .map(parse_field_def)
        .collect::<Result<Vec<_>, _>>()?;
    let indexes = config
        .indexes
        .into_iter()
        .map(parse_index_def)
        .collect::<Result<Vec<_>, _>>()?;
    let inject_on = config
        .inject_on
        .into_iter()
        .map(|s| parse_trigger(&s))
        .collect::<Result<Vec<_>, _>>()?;
    let modifiers = config
        .modifiers
        .into_iter()
        .map(parse_modifier)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MemoryDef {
        name,
        memory_type,
        schema,
        retention,
        lifecycle,
        parent: config.parent,
        indexes,
        inject_on,
        artifacts: config.artifacts,
        modifiers,
    })
}

pub fn parse_policy_block(
    header_name: Option<String>,
    content: &str,
) -> Result<PolicyDef, ConfigError> {
    let config: PolicyConfig = serde_yaml::from_str(content)
        .map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict(
                "Name conflict".to_string(),
            ));
        }
        (None, Some(payload_name)) => payload_name.clone(),
        (None, None) => {
            return Err(ConfigError::MissingName(
                "Policy requires name".to_string(),
            ));
        }
    };

    let rules = config
        .rules
        .into_iter()
        .map(parse_policy_rule)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PolicyDef { name, rules })
}

pub fn parse_injection_block(
    header_name: Option<String>,
    content: &str,
) -> Result<InjectionDef, ConfigError> {
    let config: InjectionConfig = serde_yaml::from_str(content)
        .map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    // Note: Injection might not need a name (it's identified by source/target)
    // But we'll enforce name precedence if a name is provided
    if let (Some(_), Some(_)) = (&header_name, &config.name) {
        return Err(ConfigError::NameConflict(
            "Name conflict".to_string(),
        ));
    }

    let mode = parse_injection_mode(&config.mode)?;

    Ok(InjectionDef {
        source: config.source,
        target: config.target,
        mode,
        priority: config.priority,
        max_tokens: config.max_tokens,
        filter: None, // Not supported in YAML yet
    })
}

pub fn parse_provider_block(
    header_name: Option<String>,
    content: &str,
) -> Result<ProviderDef, ConfigError> {
    let config: ProviderConfig = serde_yaml::from_str(content)
        .map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict(
                "Name conflict".to_string(),
            ));
        }
        (None, Some(payload_name)) => payload_name.clone(),
        (None, None) => {
            return Err(ConfigError::MissingName(
                "Provider requires name".to_string(),
            ));
        }
    };

    let provider_type = parse_provider_type(&config.provider_type)?;
    let api_key = parse_env_value(&config.api_key);

    Ok(ProviderDef {
        name,
        provider_type,
        api_key,
        model: config.model,
        options: config.options,
    })
}

pub fn parse_cache_block(
    header_name: Option<String>,
    content: &str,
) -> Result<CacheDef, ConfigError> {
    let config: CacheConfig = serde_yaml::from_str(content)
        .map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    // Cache might use header name or be singleton
    if let (Some(_), Some(_)) = (&header_name, &config.name) {
        return Err(ConfigError::NameConflict(
            "Name conflict".to_string(),
        ));
    }

    let backend = parse_cache_backend(&config.backend)?;
    let default_freshness = parse_freshness_def(config.default_freshness)?;

    Ok(CacheDef {
        backend,
        path: config.path,
        size_mb: config.size_mb,
        default_freshness,
        max_entries: config.max_entries,
        ttl: config.ttl,
    })
}

pub fn parse_trajectory_block(
    header_name: Option<String>,
    content: &str,
) -> Result<TrajectoryDef, ConfigError> {
    let config: TrajectoryConfig = serde_yaml::from_str(content)
        .map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict(
                "Name conflict".to_string(),
            ));
        }
        (None, Some(payload_name)) => payload_name.clone(),
        (None, None) => {
            return Err(ConfigError::MissingName(
                "Trajectory requires name".to_string(),
            ));
        }
    };

    Ok(TrajectoryDef {
        name,
        description: config.description,
        agent_type: config.agent_type,
        token_budget: config.token_budget,
        memory_refs: config.memory_refs,
        metadata: config.metadata,
    })
}

pub fn parse_agent_block(
    header_name: Option<String>,
    content: &str,
) -> Result<AgentDef, ConfigError> {
    let config: AgentConfig = serde_yaml::from_str(content)
        .map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict(
                "Name conflict".to_string(),
            ));
        }
        (None, Some(payload_name)) => payload_name.clone(),
        (None, None) => {
            return Err(ConfigError::MissingName(
                "Agent requires name".to_string(),
            ));
        }
    };

    Ok(AgentDef {
        name,
        capabilities: config.capabilities,
        constraints: AgentConstraints {
            max_concurrent: config.constraints.max_concurrent,
            timeout_ms: config.constraints.timeout_ms,
        },
        permissions: PermissionMatrix {
            read: config.permissions.read,
            write: config.permissions.write,
            lock: config.permissions.lock,
        },
    })
}

// ============================================================================
// VALIDATION LAYER (refinement types via runtime checks)
// ============================================================================

fn validate_adapter_config(config: &AdapterConfig) -> Result<(), ConfigError> {
    // Validate adapter_type is known
    parse_adapter_type(&config.adapter_type)?;
    Ok(())
}

// ============================================================================
// TYPE CONVERSION HELPERS
// ============================================================================

fn parse_adapter_type(s: &str) -> Result<AdapterType, ConfigError> {
    match s.to_lowercase().as_str() {
        "postgres" => Ok(AdapterType::Postgres),
        "redis" => Ok(AdapterType::Redis),
        "memory" => Ok(AdapterType::Memory),
        other => Err(ConfigError::UnknownAdapter(other.to_string())),
    }
}

fn parse_memory_type(s: &str) -> Result<MemoryType, ConfigError> {
    match s.to_lowercase().as_str() {
        "ephemeral" => Ok(MemoryType::Ephemeral),
        "working" => Ok(MemoryType::Working),
        "episodic" => Ok(MemoryType::Episodic),
        "semantic" => Ok(MemoryType::Semantic),
        "procedural" => Ok(MemoryType::Procedural),
        "meta" => Ok(MemoryType::Meta),
        other => Err(ConfigError::InvalidValue(format!(
            "Unknown memory type '{}'",
            other
        ))),
    }
}

fn parse_retention(s: &str) -> Result<Retention, ConfigError> {
    match s.to_lowercase().as_str() {
        "persistent" => Ok(Retention::Persistent),
        "session" => Ok(Retention::Session),
        "scope" => Ok(Retention::Scope),
        other if other.starts_with("duration:") => {
            Ok(Retention::Duration(other["duration:".len()..].to_string()))
        }
        other if other.starts_with("max:") => {
            let count = other["max:".len()..]
                .parse()
                .map_err(|_| ConfigError::InvalidValue("Invalid max count".to_string()))?;
            Ok(Retention::Max(count))
        }
        other => Err(ConfigError::InvalidValue(format!(
            "Unknown retention '{}'",
            other
        ))),
    }
}

fn parse_lifecycle(s: &str) -> Result<Lifecycle, ConfigError> {
    match s.to_lowercase().as_str() {
        "explicit" => Ok(Lifecycle::Explicit),
        other if other.starts_with("autoclose:") => {
            let trigger = parse_trigger(other["autoclose:".len()..].trim())?;
            Ok(Lifecycle::AutoClose(trigger))
        }
        other => Err(ConfigError::InvalidValue(format!(
            "Unknown lifecycle '{}'",
            other
        ))),
    }
}

fn parse_trigger(s: &str) -> Result<Trigger, ConfigError> {
    match s.to_lowercase().as_str() {
        "task_start" => Ok(Trigger::TaskStart),
        "task_end" => Ok(Trigger::TaskEnd),
        "scope_close" => Ok(Trigger::ScopeClose),
        "turn_end" => Ok(Trigger::TurnEnd),
        "manual" => Ok(Trigger::Manual),
        other if other.starts_with("schedule:") => {
            Ok(Trigger::Schedule(other["schedule:".len()..].to_string()))
        }
        other => Err(ConfigError::InvalidValue(format!(
            "Unknown trigger '{}'",
            other
        ))),
    }
}

fn parse_field_def(config: FieldConfig) -> Result<FieldDef, ConfigError> {
    let field_type = parse_field_type(&config.field_type)?;
    Ok(FieldDef {
        name: config.name,
        field_type,
        nullable: config.nullable,
        default: config.default,
        security: None, // Not supported in YAML yet
    })
}

fn parse_field_type(s: &str) -> Result<FieldType, ConfigError> {
    match s.to_lowercase().as_str() {
        "uuid" => Ok(FieldType::Uuid),
        "text" => Ok(FieldType::Text),
        "int" => Ok(FieldType::Int),
        "float" => Ok(FieldType::Float),
        "bool" => Ok(FieldType::Bool),
        "timestamp" => Ok(FieldType::Timestamp),
        "json" => Ok(FieldType::Json),
        other if other.starts_with("embedding:") => {
            let dim = other["embedding:".len()..]
                .parse()
                .ok();
            Ok(FieldType::Embedding(dim))
        }
        other => Err(ConfigError::InvalidValue(format!(
            "Unknown field type '{}'",
            other
        ))),
    }
}

fn parse_index_def(config: IndexConfig) -> Result<IndexDef, ConfigError> {
    let index_type = parse_index_type(&config.index_type)?;
    Ok(IndexDef {
        field: config.field,
        index_type,
        options: config.options,
    })
}

fn parse_index_type(s: &str) -> Result<IndexType, ConfigError> {
    match s.to_lowercase().as_str() {
        "btree" => Ok(IndexType::Btree),
        "hash" => Ok(IndexType::Hash),
        "gin" => Ok(IndexType::Gin),
        "hnsw" => Ok(IndexType::Hnsw),
        "ivfflat" => Ok(IndexType::Ivfflat),
        other => Err(ConfigError::InvalidValue(format!(
            "Unknown index type '{}'",
            other
        ))),
    }
}

fn parse_modifier(s: String) -> Result<ModifierDef, ConfigError> {
    // Placeholder - proper parsing needed
    Ok(ModifierDef::Embeddable {
        provider: s,
    })
}

fn parse_policy_rule(config: PolicyRuleConfig) -> Result<PolicyRule, ConfigError> {
    let trigger = parse_trigger(&config.trigger)?;
    let actions = config
        .actions
        .into_iter()
        .map(parse_action)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PolicyRule { trigger, actions })
}

fn parse_action(config: ActionConfig) -> Result<Action, ConfigError> {
    match config {
        ActionConfig::Summarize { target } => Ok(Action::Summarize(target)),
        ActionConfig::Checkpoint { target } => Ok(Action::Checkpoint(target)),
        ActionConfig::ExtractArtifacts { target } => Ok(Action::ExtractArtifacts(target)),
        ActionConfig::Notify { target } => Ok(Action::Notify(target)),
        ActionConfig::Inject { target, mode } => {
            let injection_mode = parse_injection_mode(&mode)?;
            Ok(Action::Inject {
                target,
                mode: injection_mode,
            })
        }
    }
}

fn parse_injection_mode(s: &str) -> Result<InjectionMode, ConfigError> {
    match s.to_lowercase().as_str() {
        "full" => Ok(InjectionMode::Full),
        "summary" => Ok(InjectionMode::Summary),
        other if other.starts_with("topk:") => {
            let k = other["topk:".len()..]
                .parse()
                .map_err(|_| ConfigError::InvalidValue("Invalid topk value".to_string()))?;
            Ok(InjectionMode::TopK(k))
        }
        other if other.starts_with("relevant:") => {
            let threshold = other["relevant:".len()..]
                .parse()
                .map_err(|_| ConfigError::InvalidValue("Invalid threshold".to_string()))?;
            Ok(InjectionMode::Relevant(threshold))
        }
        other => Err(ConfigError::InvalidValue(format!(
            "Unknown injection mode '{}'",
            other
        ))),
    }
}

fn parse_provider_type(s: &str) -> Result<ProviderType, ConfigError> {
    match s.to_lowercase().as_str() {
        "openai" => Ok(ProviderType::OpenAI),
        "anthropic" => Ok(ProviderType::Anthropic),
        "custom" => Ok(ProviderType::Custom),
        other => Err(ConfigError::UnknownProvider(other.to_string())),
    }
}

fn parse_env_value(s: &str) -> EnvValue {
    if let Some(rest) = s.strip_prefix("env:") {
        EnvValue::Env(rest.trim().to_string())
    } else {
        EnvValue::Literal(s.to_string())
    }
}

fn parse_cache_backend(s: &str) -> Result<CacheBackendType, ConfigError> {
    match s.to_lowercase().as_str() {
        "lmdb" => Ok(CacheBackendType::Lmdb),
        "memory" => Ok(CacheBackendType::Memory),
        other => Err(ConfigError::InvalidValue(format!(
            "Unknown cache backend '{}'",
            other
        ))),
    }
}

fn parse_freshness_def(config: FreshnessConfig) -> Result<FreshnessDef, ConfigError> {
    match config {
        FreshnessConfig::BestEffort { max_staleness } => Ok(FreshnessDef::BestEffort { max_staleness }),
        FreshnessConfig::Strict => Ok(FreshnessDef::Strict),
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_parse_with_header_name() {
        let yaml = r#"
adapter_type: postgres
connection: "postgresql://localhost/test"
"#;
        let result = parse_adapter_block(Some("postgres_main".to_string()), yaml);
        assert!(result.is_ok(), "Failed to parse adapter: {:?}", result.err());

        let adapter = result.unwrap();
        assert_eq!(adapter.name, "postgres_main");
        assert_eq!(adapter.adapter_type, AdapterType::Postgres);
        assert_eq!(adapter.connection, "postgresql://localhost/test");
    }

    #[test]
    fn test_adapter_parse_with_payload_name() {
        let yaml = r#"
name: postgres_main
adapter_type: postgres
connection: "postgresql://localhost/test"
"#;
        let result = parse_adapter_block(None, yaml);
        assert!(result.is_ok(), "Failed to parse adapter: {:?}", result.err());

        let adapter = result.unwrap();
        assert_eq!(adapter.name, "postgres_main");
    }

    #[test]
    fn test_adapter_deny_unknown_fields() {
        let yaml = r#"
adapter_type: postgres
connection: "postgresql://localhost/test"
unknown_field: bad
"#;
        let result = parse_adapter_block(Some("test".to_string()), yaml);
        assert!(result.is_err(), "Should reject unknown field");

        let err = result.unwrap_err();
        match err {
            ConfigError::YamlParse(msg) => {
                assert!(msg.contains("unknown field"), "Expected 'unknown field' error, got: {}", msg);
            }
            _ => panic!("Expected YamlParse error, got: {:?}", err),
        }
    }

    #[test]
    fn test_adapter_name_conflict() {
        let yaml = r#"
name: payload_name
adapter_type: postgres
connection: "postgresql://localhost/test"
"#;
        let result = parse_adapter_block(Some("header_name".to_string()), yaml);
        assert!(result.is_err(), "Should reject name conflict");

        let err = result.unwrap_err();
        match err {
            ConfigError::NameConflict(_) => {
                // Expected
            }
            _ => panic!("Expected NameConflict error, got: {:?}", err),
        }
    }

    #[test]
    fn test_adapter_missing_name() {
        let yaml = r#"
adapter_type: postgres
connection: "postgresql://localhost/test"
"#;
        let result = parse_adapter_block(None, yaml);
        assert!(result.is_err(), "Should require name");

        let err = result.unwrap_err();
        match err {
            ConfigError::MissingName(_) => {
                // Expected
            }
            _ => panic!("Expected MissingName error, got: {:?}", err),
        }
    }

    #[test]
    fn test_adapter_case_preservation() {
        let yaml = r#"
adapter_type: PostgreS
connection: "PostgreSQL://LocalHost/Test"
"#;
        let result = parse_adapter_block(Some("MyAdapter".to_string()), yaml);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let adapter = result.unwrap();
        assert_eq!(adapter.name, "MyAdapter");
        // Note: adapter_type is normalized to lowercase in parsing, but connection preserves case
        assert_eq!(adapter.connection, "PostgreSQL://LocalHost/Test");
    }

    #[test]
    fn test_provider_parse_with_env_key() {
        let yaml = r#"
provider_type: openai
api_key: env:OPENAI_API_KEY
model: "gpt-4"
"#;
        let result = parse_provider_block(Some("my_provider".to_string()), yaml);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let provider = result.unwrap();
        assert_eq!(provider.name, "my_provider");
        assert_eq!(provider.provider_type, ProviderType::OpenAI);
        match provider.api_key {
            EnvValue::Env(var) => assert_eq!(var, "OPENAI_API_KEY"),
            _ => panic!("Expected Env variant"),
        }
    }

    #[test]
    fn test_provider_deny_unknown_fields() {
        let yaml = r#"
provider_type: openai
api_key: "secret"
model: "gpt-4"
invalid_option: true
"#;
        let result = parse_provider_block(Some("test".to_string()), yaml);
        assert!(result.is_err(), "Should reject unknown field");
    }

    #[test]
    fn test_injection_mode_parsing() {
        let yaml = r#"
source: "memories.episodic"
target: "context.main"
mode: full
priority: 100
"#;
        let result = parse_injection_block(None, yaml);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let injection = result.unwrap();
        assert_eq!(injection.mode, InjectionMode::Full);
        assert_eq!(injection.priority, 100);
    }

    #[test]
    fn test_cache_freshness_parsing() {
        let yaml = r#"
backend: lmdb
path: "/var/cache"
size_mb: 1024
default_freshness:
  type: best_effort
  max_staleness: "60s"
"#;
        let result = parse_cache_block(Some("main".to_string()), yaml);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let cache = result.unwrap();
        match cache.default_freshness {
            FreshnessDef::BestEffort { max_staleness } => {
                assert_eq!(max_staleness, "60s");
            }
            _ => panic!("Expected BestEffort variant"),
        }
    }
}
