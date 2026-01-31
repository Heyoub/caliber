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
    /// Formats a ConfigError into a human-readable message.
    ///
    /// Each variant is rendered as "<prefix>: <message>", for example
    /// "YAML parse error: ..." or "Missing name: ...".
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fmt::Display;
    /// let err = crate::ConfigError::MissingName("memory".into());
    /// assert_eq!(format!("{}", err), "Missing name: memory");
    /// ```
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
    /// Convert a `ConfigError` into a `PackError` by mapping it to `PackError::Validation`
    /// containing the error's string representation.
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg_err = ConfigError::MissingName("adapter".into());
    /// let pack_err: PackError = cfg_err.into();
    /// assert!(matches!(pack_err, PackError::Validation(_)));
    /// ```
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

/// Parses an adapter YAML fence block into an `AdapterDef`.
///
/// The function deserializes `content` as YAML, determines the adapter name using
/// the fence header if present (erroring on conflicts or absence), validates the
/// adapter configuration, and converts it into an `AdapterDef`.
///
/// # Errors
///
/// Returns a `ConfigError` when:
/// - YAML deserialization fails (`ConfigError::YamlParse`),
/// - both a header name and a payload name are provided (`ConfigError::NameConflict`),
/// - no name is provided (`ConfigError::MissingName`),
/// - the adapter type is unrecognized (`ConfigError::UnknownAdapter`),
/// - or other validation failures occur.
///
/// # Examples
///
/// ```
/// #[test]
/// fn parse_with_header_name() {
///     let header = Some("my_adapter".to_string());
///     let yaml = r#"
/// adapter_type: "postgres"
/// connection: "postgres://user:pass@localhost/db"
/// options: []
/// "#;
///     let def = parse_adapter_block(header, yaml).expect("should parse");
///     assert_eq!(def.name, "my_adapter");
/// }
/// ```
pub fn parse_adapter_block(
    header_name: Option<String>,
    content: &str,
) -> Result<AdapterDef, ConfigError> {
    // Step 1: Deserialize YAML into config struct
    let config: AdapterConfig =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

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

/// Parses a YAML memory block (fence content) into a MemoryDef.
///
/// The header-provided name takes precedence when the payload omits a name.
/// If both header and payload provide names, or if neither provides a name, an error is returned.
/// Returns an error for invalid YAML or any invalid/unknown values found during conversion.
///
/// # Errors
///
/// Returns `ConfigError::YamlParse` if the content is not valid YAML; `ConfigError::NameConflict`
/// if both header and payload specify different names; `ConfigError::MissingName` if no name is provided;
/// or other `ConfigError` variants produced by value parsing helpers when fields contain invalid values.
///
/// # Examples
///
/// ```
/// let yaml = r#"
/// memory_type: episodic
/// retention: persistent
/// lifecycle: explicit
/// schema:
///   - name: id
///     field_type: uuid
///   - name: content
///     field_type: text
/// indexes: []
/// inject_on: []
/// modifiers: []
/// "#;
///
/// let def = parse_memory_block(Some("my_memory".to_string()), yaml).unwrap();
/// assert_eq!(def.name, "my_memory");
/// ```
pub fn parse_memory_block(
    header_name: Option<String>,
    content: &str,
) -> Result<MemoryDef, ConfigError> {
    let config: MemoryConfig =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict("Name conflict".to_string()));
        }
        (None, Some(payload_name)) => payload_name.clone(),
        (None, None) => {
            return Err(ConfigError::MissingName("Memory requires name".to_string()));
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

/// Parses a policy YAML block from a Markdown fence into a PolicyDef.
///
/// The function deserializes `content` as YAML into a PolicyConfig, applies name precedence
/// (the fence `header_name` is used when present; an explicit payload name conflicts with the
/// header; an absent name in both locations is an error), converts each policy rule into the
/// AST via `parse_policy_rule`, and returns a `PolicyDef` with the resolved name and rules.
///
/// # Errors
///
/// Returns a `ConfigError::YamlParse` if YAML deserialization fails, `ConfigError::NameConflict`
/// if both header and payload provide different names, `ConfigError::MissingName` if no name is
/// supplied, or any `ConfigError` produced while parsing policy rules.
///
/// # Examples
///
/// ```
/// let yaml = r#"
/// name: example-policy
/// rules:
///   - trigger: task_end
///     actions:
///       - summarize:
///           target: "summary-target"
/// "#;
/// let def = parse_policy_block(None, yaml).unwrap();
/// assert_eq!(def.name, "example-policy");
/// assert_eq!(def.rules.len(), 1);
/// ```
pub fn parse_policy_block(
    header_name: Option<String>,
    content: &str,
) -> Result<PolicyDef, ConfigError> {
    let config: PolicyConfig =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict("Name conflict".to_string()));
        }
        (None, Some(payload_name)) => payload_name.clone(),
        (None, None) => {
            return Err(ConfigError::MissingName("Policy requires name".to_string()));
        }
    };

    let rules = config
        .rules
        .into_iter()
        .map(parse_policy_rule)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PolicyDef { name, rules })
}

/// Parses an InjectionConfig YAML block and converts it into an InjectionDef.
///
/// If both a header name and a `name` field in the YAML payload are present, this returns
/// a `ConfigError::NameConflict`. The returned `InjectionDef` contains the parsed
/// source, target, injection mode, priority, and optional max_tokens. `filter` is unset.
///
/// # Examples
///
/// ```
/// let yaml = r#"
/// source: "memory_a"
/// target: "agent_b"
/// mode: "full"
/// priority: 10
/// "#;
/// let def = parse_injection_block(None, yaml).unwrap();
/// assert_eq!(def.source, "memory_a");
/// assert_eq!(def.target, "agent_b");
/// assert_eq!(def.priority, 10);
/// ```
pub fn parse_injection_block(
    header_name: Option<String>,
    content: &str,
) -> Result<InjectionDef, ConfigError> {
    let config: InjectionConfig =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    // Note: Injection might not need a name (it's identified by source/target)
    // But we'll enforce name precedence if a name is provided
    if let (Some(_), Some(_)) = (&header_name, &config.name) {
        return Err(ConfigError::NameConflict("Name conflict".to_string()));
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

/// Parse a provider fence block from YAML content into a ProviderDef.
///
/// Parses `content` as YAML into a `ProviderConfig`, applies name precedence between
/// the optional `header_name` and the payload `name` (header wins; conflict is an error),
/// resolves the `provider_type`, and converts `api_key` into an environment-aware `EnvValue`.
///
/// # Errors
///
/// Returns `ConfigError::YamlParse` if the YAML is invalid, `ConfigError::NameConflict` if
/// both header and payload names are provided, `ConfigError::MissingName` if no name is present,
/// or other `ConfigError` variants produced while parsing `provider_type` or `api_key`.
///
/// # Examples
///
/// ```
/// let yaml = r#"
/// provider_type: openai
/// api_key: env:OPENAI_KEY
/// model: gpt-4
/// options: []
/// "#;
///
/// let def = parse_provider_block(Some("my-provider".to_string()), yaml).unwrap();
/// assert_eq!(def.name, "my-provider");
/// ```
pub fn parse_provider_block(
    header_name: Option<String>,
    content: &str,
) -> Result<ProviderDef, ConfigError> {
    let config: ProviderConfig =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict("Name conflict".to_string()));
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

/// Parses a cache configuration YAML block and converts it into a CacheDef.
///
/// Returns an error if YAML parsing fails, if the header and payload both provide a name,
/// or if backend/freshness values are invalid.
///
/// # Examples
///
/// ```
/// let yaml = r#"
/// backend: memory
/// size_mb: 100
/// default_freshness: Strict
/// "#;
/// let def = parse_cache_block(None, yaml).unwrap();
/// assert_eq!(def.size_mb, 100);
/// ```
pub fn parse_cache_block(
    header_name: Option<String>,
    content: &str,
) -> Result<CacheDef, ConfigError> {
    let config: CacheConfig =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    // Cache might use header name or be singleton
    if let (Some(_), Some(_)) = (&header_name, &config.name) {
        return Err(ConfigError::NameConflict("Name conflict".to_string()));
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

/// Parse a Markdown fence trajectory YAML block into a TrajectoryDef, resolving the block name from the header or payload.
///
/// The resulting `TrajectoryDef` is populated from the deserialized YAML payload. The trajectory `name` is taken from `header_name` when provided; if `header_name` is `None`, the payload `name` is used. If both are present the function returns an error, and if neither is present it returns an error.
///
/// # Errors
///
/// Returns `ConfigError::YamlParse` if the YAML cannot be deserialized, `ConfigError::NameConflict` if both a header name and a payload name are supplied, or `ConfigError::MissingName` if no name is provided.
///
/// # Examples
///
/// ```
/// let yaml = r#"
/// name: my_trajectory
/// description: "A sample trajectory"
/// agent_type: "assistant"
/// token_budget: 1000
/// memory_refs:
///   - mem1
///   - mem2
/// "#;
///
/// let def = parse_trajectory_block(None, yaml).expect("parse");
/// assert_eq!(def.name, "my_trajectory");
/// assert_eq!(def.agent_type, "assistant");
/// assert_eq!(def.token_budget, 1000);
/// assert_eq!(def.memory_refs.len(), 2);
/// ```
pub fn parse_trajectory_block(
    header_name: Option<String>,
    content: &str,
) -> Result<TrajectoryDef, ConfigError> {
    let config: TrajectoryConfig =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict("Name conflict".to_string()));
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

/// Parses an agent YAML block and converts it into an AgentDef.
///
/// The function deserializes `content` as an `AgentConfig`, resolves the agent name by
/// preferring `header_name` when present (and returns an error on conflict), and maps
/// capabilities, constraints, and permissions into an `AgentDef`.
///
/// # Errors
///
/// Returns `ConfigError::YamlParse` if the YAML cannot be deserialized,
/// `ConfigError::NameConflict` if both `header_name` and the payload specify a name,
/// and `ConfigError::MissingName` if no name is provided in either place.
///
/// # Examples
///
/// ```
/// let yaml = r#"
/// name: my-agent
/// capabilities:
///   - read
///   - write
/// constraints:
///   max_concurrent: 4
///   timeout_ms: 10000
/// permissions:
///   read: [ "resource_a" ]
///   write: [ "resource_b" ]
///   lock: []
/// "#;
///
/// let def = parse_agent_block(None, yaml).unwrap();
/// assert_eq!(def.name, "my-agent");
/// assert_eq!(def.capabilities, vec!["read".into(), "write".into()]);
/// assert_eq!(def.constraints.max_concurrent, 4);
/// ```
pub fn parse_agent_block(
    header_name: Option<String>,
    content: &str,
) -> Result<AgentDef, ConfigError> {
    let config: AgentConfig =
        serde_yaml::from_str(content).map_err(|e| ConfigError::YamlParse(e.to_string()))?;

    let name = match (header_name, &config.name) {
        (Some(header), None) => header,
        (Some(_), Some(_)) => {
            return Err(ConfigError::NameConflict("Name conflict".to_string()));
        }
        (None, Some(payload_name)) => payload_name.clone(),
        (None, None) => {
            return Err(ConfigError::MissingName("Agent requires name".to_string()));
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

/// Validates that the adapter configuration specifies a known adapter type.
///
/// This checks the `adapter_type` field of the provided `AdapterConfig` and
/// returns an error if it does not map to a supported adapter.
///
/// # Returns
///
/// `Err(ConfigError::UnknownAdapter)` if `adapter_type` is not recognized, `Ok(())` otherwise.
///
/// # Examples
///
/// ```
/// let cfg = AdapterConfig {
///     name: Some("main".into()),
///     adapter_type: "postgres".into(),
///     connection: "postgres://user:pass@localhost/db".into(),
///     options: vec![],
/// };
/// assert!(validate_adapter_config(&cfg).is_ok());
/// ```
fn validate_adapter_config(config: &AdapterConfig) -> Result<(), ConfigError> {
    // Validate adapter_type is known
    parse_adapter_type(&config.adapter_type)?;
    Ok(())
}

// ============================================================================
// TYPE CONVERSION HELPERS
// ============================================================================

/// Parse a string into an AdapterType.
///
/// # Returns
///
/// `Ok(AdapterType)` corresponding to the input string (case-insensitive), or
/// `Err(ConfigError::UnknownAdapter(_))` if the input does not match a known adapter.
///
/// # Examples
///
/// ```
/// let a = parse_adapter_type("Postgres").unwrap();
/// assert_eq!(a, AdapterType::Postgres);
/// assert!(matches!(parse_adapter_type("unknown"), Err(ConfigError::UnknownAdapter(_))));
/// ```
fn parse_adapter_type(s: &str) -> Result<AdapterType, ConfigError> {
    match s.to_lowercase().as_str() {
        "postgres" | "postgresql" => Ok(AdapterType::Postgres),
        "redis" => Ok(AdapterType::Redis),
        "memory" => Ok(AdapterType::Memory),
        other => Err(ConfigError::UnknownAdapter(other.to_string())),
    }
}

/// Parse a memory type name into its corresponding MemoryType.
///
/// Accepts a case-insensitive string and returns the matching MemoryType variant.
/// If the input does not match a known memory type, returns `ConfigError::InvalidValue`.
///
/// # Examples
///
/// ```
/// let t = parse_memory_type("Ephemeral").unwrap();
/// assert_eq!(t, MemoryType::Ephemeral);
///
/// assert!(matches!(
///     parse_memory_type("unknown"),
///     Err(ConfigError::InvalidValue(_))
/// ));
/// ```
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

/// Parse a retention specifier string into a `Retention` value.
///
/// Recognizes the literals `persistent`, `session`, and `scope`, the
/// `duration:<value>` form which yields `Retention::Duration(<value>)`,
/// and the `max:<n>` form which yields `Retention::Max(n)`.
/// If `max:<n>` contains a non-integer `n`, returns `ConfigError::InvalidValue`.
///
/// # Examples
///
/// ```
/// assert_eq!(parse_retention("persistent").unwrap(), Retention::Persistent);
/// assert_eq!(parse_retention("duration:7d").unwrap(), Retention::Duration("7d".into()));
/// assert_eq!(parse_retention("max:5").unwrap(), Retention::Max(5));
/// ```
fn parse_retention(s: &str) -> Result<Retention, ConfigError> {
    match s.to_lowercase().as_str() {
        "persistent" => Ok(Retention::Persistent),
        "session" => Ok(Retention::Session),
        "scope" => Ok(Retention::Scope),
        other => {
            if let Some(duration) = other.strip_prefix("duration:") {
                Ok(Retention::Duration(duration.to_string()))
            } else if let Some(max_str) = other.strip_prefix("max:") {
                let count = max_str
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue("Invalid max count".to_string()))?;
                Ok(Retention::Max(count))
            } else {
                Err(ConfigError::InvalidValue(format!(
                    "Unknown retention '{}'",
                    other
                )))
            }
        }
    }
}

/// Parse a lifecycle descriptor string into a `Lifecycle`.
///
/// Accepts the literal "explicit" or the form "autoclose:<trigger-expression>"; the latter produces
/// `Lifecycle::AutoClose(trigger)` where `<trigger-expression>` is parsed into a `Trigger`.
///
/// # Returns
///
/// `Lifecycle::Explicit` for "explicit", `Lifecycle::AutoClose(...)` for "autoclose:<...>", or a
/// `ConfigError::InvalidValue` if the input does not match a known lifecycle.
///
/// # Examples
///
/// ```
/// let explicit = parse_lifecycle("explicit").unwrap();
/// assert_eq!(explicit, Lifecycle::Explicit);
///
/// let auto = parse_lifecycle("autoclose: task_end").unwrap();
/// match auto {
///     Lifecycle::AutoClose(_) => {}
///     _ => panic!("expected AutoClose"),
/// }
/// ```
fn parse_lifecycle(s: &str) -> Result<Lifecycle, ConfigError> {
    match s.to_lowercase().as_str() {
        "explicit" => Ok(Lifecycle::Explicit),
        other => {
            if let Some(autoclose_str) = other.strip_prefix("autoclose:") {
                let trigger = parse_trigger(autoclose_str.trim())?;
                Ok(Lifecycle::AutoClose(trigger))
            } else {
                Err(ConfigError::InvalidValue(format!(
                    "Unknown lifecycle '{}'",
                    other
                )))
            }
        }
    }
}

/// Parses a trigger specifier string into a `Trigger`.
///
/// Recognizes the literal values `task_start`, `task_end`, `scope_close`, `turn_end`, and `manual` (case-insensitive),
/// and `schedule:<expr>` which produces `Trigger::Schedule` with `<expr>` as its payload.
///
/// # Examples
///
/// ```
/// let t = parse_trigger("task_start").unwrap();
/// assert!(matches!(t, Trigger::TaskStart));
///
/// let s = parse_trigger("schedule:0 0 * * *").unwrap();
/// assert!(matches!(s, Trigger::Schedule(expr) if expr == "0 0 * * *"));
/// ```
///
/// # Returns
///
/// `Ok(Trigger)` when the input matches a known trigger; `Err(ConfigError::InvalidValue)` when it does not.
fn parse_trigger(s: &str) -> Result<Trigger, ConfigError> {
    match s.to_lowercase().as_str() {
        "task_start" => Ok(Trigger::TaskStart),
        "task_end" => Ok(Trigger::TaskEnd),
        "scope_close" => Ok(Trigger::ScopeClose),
        "turn_end" => Ok(Trigger::TurnEnd),
        "manual" => Ok(Trigger::Manual),
        other => {
            if let Some(schedule_str) = other.strip_prefix("schedule:") {
                Ok(Trigger::Schedule(schedule_str.to_string()))
            } else {
                Err(ConfigError::InvalidValue(format!(
                    "Unknown trigger '{}'",
                    other
                )))
            }
        }
    }
}

/// Converts a YAML-deserialized FieldConfig into the internal FieldDef.
///
/// The function parses the textual `field_type` into a `FieldType` and
/// constructs a `FieldDef` preserving `name`, `nullable`, and `default`.
/// The `security` field is set to `None` because YAML-based security is not supported yet.
///
/// # Returns
///
/// `FieldDef` built from the given config; `Err(ConfigError)` if the `field_type` is invalid.
///
/// # Examples
///
/// ```
/// let cfg = FieldConfig {
///     name: "id".to_string(),
///     field_type: "uuid".to_string(),
///     nullable: false,
///     default: None,
/// };
/// let def = parse_field_def(cfg).unwrap();
/// assert_eq!(def.name, "id");
/// ```
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

/// Parses a field type identifier into a `FieldType`.
///
/// Accepts case-insensitive names: `uuid`, `text`, `int`, `float`, `bool`,
/// `timestamp`, and `json`. Recognizes `embedding:<dim>` where `<dim>` is an
/// integer; if the dimension fails to parse the embedding's dimension will be
/// `None` (the embedding type is still returned). Returns `Err(ConfigError::InvalidValue(_))`
/// for unknown type strings.
///
/// # Examples
///
/// ```
/// assert_eq!(parse_field_type("text").unwrap(), FieldType::Text);
/// assert_eq!(parse_field_type("EMBEDDING:128").unwrap(), FieldType::Embedding(Some(128)));
/// assert_eq!(parse_field_type("embedding:bad").unwrap(), FieldType::Embedding(None));
/// assert!(matches!(parse_field_type("unknown"), Err(ConfigError::InvalidValue(_))));
/// ```
fn parse_field_type(s: &str) -> Result<FieldType, ConfigError> {
    match s.to_lowercase().as_str() {
        "uuid" => Ok(FieldType::Uuid),
        "text" => Ok(FieldType::Text),
        "int" => Ok(FieldType::Int),
        "float" => Ok(FieldType::Float),
        "bool" => Ok(FieldType::Bool),
        "timestamp" => Ok(FieldType::Timestamp),
        "json" => Ok(FieldType::Json),
        other => {
            if let Some(dim_str) = other.strip_prefix("embedding:") {
                let dim = dim_str.parse().ok();
                Ok(FieldType::Embedding(dim))
            } else {
                Err(ConfigError::InvalidValue(format!(
                    "Unknown field type '{}'",
                    other
                )))
            }
        }
    }
}

/// Converts an IndexConfig into an IndexDef by parsing the configured index type.
///
/// Parses the `index_type` string and returns an `IndexDef` preserving the `field` and `options` from the input config.
///
/// # Returns
///
/// `Ok(IndexDef)` on success; `Err(ConfigError::InvalidValue)` if the `index_type` text is not a known index type.
///
/// # Examples
///
/// ```
/// let cfg = IndexConfig {
///     field: "title".into(),
///     index_type: "btree".into(),
///     options: vec![],
/// };
/// let def = parse_index_def(cfg).unwrap();
/// assert_eq!(def.field, "title");
/// assert_eq!(def.options.len(), 0);
/// ```
fn parse_index_def(config: IndexConfig) -> Result<IndexDef, ConfigError> {
    let index_type = parse_index_type(&config.index_type)?;
    Ok(IndexDef {
        field: config.field,
        index_type,
        options: config.options,
    })
}

/// Parses a string representation of an index type into an `IndexType`.
///
/// Recognized values (case-insensitive): `"btree"`, `"hash"`, `"gin"`, `"hnsw"`, `"ivfflat"`.
///
/// # Returns
///
/// `Ok(IndexType)` for a recognized value, or `Err(ConfigError::InvalidValue)` if the value is unknown.
///
/// # Examples
///
/// ```
/// let t = parse_index_type("hnsw").unwrap();
/// assert_eq!(t, IndexType::Hnsw);
/// ```
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

/// Parses a modifier specification string into a `ModifierDef`.
///
/// Currently this is a placeholder implementation that treats the entire input
/// string as the name of an embeddable provider.
///
/// # Parameters
///
/// - `s`: Modifier specification string.
///
/// # Returns
///
/// `Ok(ModifierDef::Embeddable { provider })` where `provider` is the original
/// input string.
///
/// # Examples
///
/// ```
/// let def = parse_modifier("openai".to_string()).unwrap();
/// match def {
///     ModifierDef::Embeddable { provider } => assert_eq!(provider, "openai"),
///     _ => panic!("unexpected modifier variant"),
/// }
/// ```
fn parse_modifier(s: String) -> Result<ModifierDef, ConfigError> {
    let s_lower = s.to_lowercase();

    if let Some(provider) = s_lower.strip_prefix("embeddable:") {
        Ok(ModifierDef::Embeddable {
            provider: provider.to_string(),
        })
    } else if let Some(style_str) = s_lower.strip_prefix("summarizable:") {
        let style = match style_str {
            "brief" => SummaryStyle::Brief,
            "detailed" => SummaryStyle::Detailed,
            other => {
                return Err(ConfigError::InvalidValue(format!(
                    "invalid summary style '{}', expected 'brief' or 'detailed'",
                    other
                )))
            }
        };
        Ok(ModifierDef::Summarizable {
            style,
            on_triggers: vec![], // Default to empty triggers
        })
    } else if let Some(mode_str) = s_lower.strip_prefix("lockable:") {
        let mode = match mode_str {
            "exclusive" => LockMode::Exclusive,
            "shared" => LockMode::Shared,
            other => {
                return Err(ConfigError::InvalidValue(format!(
                    "invalid lock mode '{}', expected 'exclusive' or 'shared'",
                    other
                )))
            }
        };
        Ok(ModifierDef::Lockable { mode })
    } else if s_lower == "embeddable" {
        // Default embeddable with empty provider
        Ok(ModifierDef::Embeddable {
            provider: String::new(),
        })
    } else if s_lower == "summarizable" {
        // Default summarizable with brief style
        Ok(ModifierDef::Summarizable {
            style: SummaryStyle::Brief,
            on_triggers: vec![],
        })
    } else if s_lower == "lockable" {
        // Default lockable with exclusive mode
        Ok(ModifierDef::Lockable {
            mode: LockMode::Exclusive,
        })
    } else {
        Err(ConfigError::InvalidValue(format!(
            "invalid modifier '{}', expected 'embeddable', 'summarizable', or 'lockable'",
            s
        )))
    }
}

/// Converts a deserialized PolicyRuleConfig into a validated PolicyRule.
///
/// Returns an error if the rule's trigger or any contained action is invalid.
///
/// # Examples
///
/// ```
/// let cfg = PolicyRuleConfig {
///     trigger: "task_end".to_string(),
///     actions: vec![ActionConfig::Summarize { target: "log".to_string() }],
/// };
/// let rule = parse_policy_rule(cfg).expect("valid policy rule");
/// assert_eq!(rule.trigger.to_string(), "task_end");
/// assert_eq!(rule.actions.len(), 1);
/// ```
fn parse_policy_rule(config: PolicyRuleConfig) -> Result<PolicyRule, ConfigError> {
    let trigger = parse_trigger(&config.trigger)?;
    let actions = config
        .actions
        .into_iter()
        .map(parse_action)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PolicyRule { trigger, actions })
}

/// Converts an ActionConfig (parsed from YAML) into the corresponding internal Action.
///
/// # Returns
///
/// `Ok(Action)` when conversion succeeds; `Err(ConfigError)` if a value is invalid (for example, an unrecognized injection mode).
///
/// # Examples
///
/// ```
/// let cfg = ActionConfig::Summarize { target: String::from("doc") };
/// let action = parse_action(cfg).unwrap();
/// assert_eq!(action, Action::Summarize(String::from("doc")));
/// ```
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

/// Parses an injection mode string into an InjectionMode.
///
/// Accepts (case-insensitive) "full", "summary", "topk:<n>" and "relevant:<n>".
/// Returns `Err(ConfigError::InvalidValue(_))` for unknown modes or invalid numeric arguments.
///
/// # Examples
///
/// ```
/// let m = parse_injection_mode("full").unwrap();
/// assert_eq!(m, InjectionMode::Full);
///
/// let m = parse_injection_mode("TopK:3").unwrap();
/// assert_eq!(m, InjectionMode::TopK(3));
/// ```
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

/// Parse a provider identifier string into the corresponding ProviderType.
///
/// # Returns
///
/// `Ok(ProviderType::OpenAI | ProviderType::Anthropic | ProviderType::Custom)` on recognized input,
/// `Err(ConfigError::UnknownProvider(_))` if the input is not a known provider.
///
/// # Examples
///
/// ```
/// let p = parse_provider_type("OpenAI").unwrap();
/// assert_eq!(p, ProviderType::OpenAI);
/// ```
fn parse_provider_type(s: &str) -> Result<ProviderType, ConfigError> {
    match s.to_lowercase().as_str() {
        "openai" => Ok(ProviderType::OpenAI),
        "anthropic" => Ok(ProviderType::Anthropic),
        "custom" => Ok(ProviderType::Custom),
        other => Err(ConfigError::UnknownProvider(other.to_string())),
    }
}

/// Parses a string into an EnvValue, interpreting values that start with `env:` as environment variable references.
///
/// The prefix `env:` (case-sensitive) is removed and the remainder is trimmed; if the prefix is present the result is `EnvValue::Env(var)`, otherwise the original string is returned as `EnvValue::Literal`.
///
/// # Examples
///
/// ```
/// // env reference
/// assert_eq!(parse_env_value("env:API_KEY"), EnvValue::Env("API_KEY".to_string()));
/// // with whitespace after prefix
/// assert_eq!(parse_env_value("env:  VAR  "), EnvValue::Env("VAR".to_string()));
/// // literal value
/// assert_eq!(parse_env_value("plain"), EnvValue::Literal("plain".to_string()));
/// ```
fn parse_env_value(s: &str) -> EnvValue {
    if let Some(rest) = s.strip_prefix("env:") {
        EnvValue::Env(rest.trim().to_string())
    } else {
        EnvValue::Literal(s.to_string())
    }
}

/// Parse a string into a CacheBackendType.
///
/// Accepts case-insensitive names for supported cache backends and fails for unknown values.
///
/// # Returns
///
/// `CacheBackendType::Lmdb` for "lmdb", `CacheBackendType::Memory` for "memory", or a `ConfigError::InvalidValue` for any other input.
///
/// # Examples
///
/// ```
/// let be = parse_cache_backend("LMDB").unwrap();
/// assert_eq!(be, CacheBackendType::Lmdb);
/// let mem = parse_cache_backend("memory").unwrap();
/// assert_eq!(mem, CacheBackendType::Memory);
/// assert!(parse_cache_backend("unknown").is_err());
/// ```
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

/// Convert a freshness configuration into its AST representation.
///
/// # Returns
///
/// `Ok(FreshnessDef::BestEffort { max_staleness })` when the input is `BestEffort`, `Ok(FreshnessDef::Strict)` when the input is `Strict`.
///
/// # Examples
///
/// ```
/// let cfg = FreshnessConfig::BestEffort { max_staleness: "5m".into() };
/// let def = parse_freshness_def(cfg).unwrap();
/// match def {
///     FreshnessDef::BestEffort { max_staleness } => assert_eq!(max_staleness, "5m"),
///     _ => panic!("unexpected variant"),
/// }
/// ```
fn parse_freshness_def(config: FreshnessConfig) -> Result<FreshnessDef, ConfigError> {
    match config {
        FreshnessConfig::BestEffort { max_staleness } => {
            Ok(FreshnessDef::BestEffort { max_staleness })
        }
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
        assert!(
            result.is_ok(),
            "Failed to parse adapter: {:?}",
            result.err()
        );

        let adapter = result.expect("adapter parsing verified above");
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
        assert!(
            result.is_ok(),
            "Failed to parse adapter: {:?}",
            result.err()
        );

        let adapter = result.expect("adapter parsing verified above");
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
                assert!(
                    msg.contains("unknown field"),
                    "Expected 'unknown field' error, got: {}",
                    msg
                );
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

        let adapter = result.expect("adapter parsing verified above");
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

        let provider = result.expect("provider parsing verified above");
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

        let injection = result.expect("injection parsing verified above");
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

        let cache = result.expect("cache parsing verified above");
        match cache.default_freshness {
            FreshnessDef::BestEffort { max_staleness } => {
                assert_eq!(max_staleness, "60s");
            }
            _ => panic!("Expected BestEffort variant"),
        }
    }
}
