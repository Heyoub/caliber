//! DSL Compiler - Transform AST to Runtime Configuration
//!
//! This module takes a parsed AST and compiles it into runtime-usable configuration
//! structs. The compiler validates semantic rules that can't be checked during parsing.
//!
//! # Pipeline
//!
//! ```text
//! DSL Source → Lexer → Parser → AST → Compiler → CompiledConfig → Deploy
//!                                                 ↓
//!                                           Validation (semantic)
//! ```

use crate::parser::ast::*;
use caliber_core::MemoryCategory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

// ============================================================================
// COMPILE ERRORS
// ============================================================================

/// Errors that can occur during DSL compilation.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum CompileError {
    /// Reference to undefined entity
    #[error("undefined reference: {kind} '{name}' is not defined")]
    UndefinedReference { kind: String, name: String },

    /// Duplicate definition
    #[error("duplicate definition: {kind} '{name}' is already defined")]
    DuplicateDefinition { kind: String, name: String },

    /// Invalid configuration value
    #[error("invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },

    /// Missing required field
    #[error("missing required field: {field}")]
    MissingField { field: String },

    /// Circular dependency detected
    #[error("circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },

    /// Type mismatch
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    /// Invalid duration format
    #[error("invalid duration format: {value}")]
    InvalidDuration { value: String },

    /// Semantic validation error
    #[error("semantic error: {message}")]
    SemanticError { message: String },
}

pub type CompileResult<T> = Result<T, CompileError>;

// ============================================================================
// COMPILED CONFIGURATION TYPES
// ============================================================================

/// Compiled adapter configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub name: String,
    pub adapter_type: CompiledAdapterType,
    pub connection: String,
    pub options: HashMap<String, String>,
}

/// Compiled adapter types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledAdapterType {
    Postgres,
    Redis,
    Memory,
}

/// Compiled memory configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub name: String,
    /// Memory category (uses `caliber_core::MemoryCategory`).
    pub memory_type: MemoryCategory,
    pub schema: Vec<FieldConfig>,
    /// Retention policy (uses `caliber_core::TTL`).
    pub retention: caliber_core::TTL,
    pub lifecycle: CompiledLifecycle,
    pub parent: Option<String>,
    pub indexes: Vec<IndexConfig>,
    pub inject_on: Vec<CompiledTrigger>,
    pub artifacts: Vec<String>,
    pub modifiers: MemoryModifiers,
}

/// Compiled field configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldConfig {
    pub name: String,
    /// Field type (uses `caliber_core::FieldType`).
    pub field_type: caliber_core::FieldType,
    pub nullable: bool,
    /// Security configuration for PII fields.
    pub security: Option<CompiledFieldSecurity>,
}

/// Compiled PII classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CompiledPIIClassification {
    #[default]
    Public,
    Internal,
    Confidential,
    Restricted,
    Secret,
}

/// Compiled field security configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledFieldSecurity {
    /// Sensitivity classification
    pub classification: CompiledPIIClassification,
    /// Agent can pass but not read content
    pub opaque: bool,
    /// Cannot be modified after creation
    pub immutable: bool,
    /// All access is logged
    pub audited: bool,
    /// Redact in logs and error messages
    pub redact_in_logs: bool,
    /// Source from environment variable
    pub env_source: Option<String>,
}

// NOTE: Field types use `caliber_core::FieldType`.
// The former `CompiledFieldType` was removed as it was identical.

// NOTE: Retention uses `caliber_core::TTL`.
// The former `CompiledRetention` was removed - Duration is converted to milliseconds.

/// Compiled lifecycle configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledLifecycle {
    /// Explicit lifecycle management (manual control)
    Explicit,
    /// Auto-close on trigger
    AutoClose(CompiledTrigger),
}

/// Compiled trigger types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledTrigger {
    TaskStart,
    TaskEnd,
    ScopeClose,
    TurnEnd,
    Manual,
    Schedule(String),
    DosageReached { threshold: i32 },
    TurnCount { count: i32 },
    ArtifactCount { count: i32 },
}

/// Compiled lifecycle actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledAction {
    Summarize { target: String },
    ExtractArtifacts { target: String },
    Checkpoint { target: String },
    Prune { target: String, criteria: CompiledFilter },
    Notify { target: String },
    Inject { target: String, mode: CompiledInjectionMode },
    AutoSummarize {
        source_level: CompiledAbstractionLevel,
        target_level: CompiledAbstractionLevel,
        create_edges: bool,
    },
}

/// Compiled index configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexConfig {
    pub field: String,
    pub index_type: CompiledIndexType,
    pub options: HashMap<String, String>,
}

/// Compiled index types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledIndexType {
    Btree,
    Hash,
    Gin,
    Hnsw,
    Ivfflat,
}

/// Memory modifiers (embeddable, summarizable, lockable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryModifiers {
    pub embeddable: Option<EmbeddableConfig>,
    pub summarizable: Option<SummarizableConfig>,
    pub lockable: Option<LockableConfig>,
}

/// Summarizable modifier configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummarizableConfig {
    pub style: SummarizeStyle,
    pub on_triggers: Vec<CompiledTrigger>,
}

/// Embeddable modifier configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddableConfig {
    pub provider: String,
}

/// Summary styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SummarizeStyle {
    Brief,
    Detailed,
}

/// Lockable modifier configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockableConfig {
    pub mode: CompiledLockMode,
}

/// Compiled lock modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledLockMode {
    Exclusive,
    Shared,
}

/// Compiled policy configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub name: String,
    pub rules: Vec<CompiledPolicyRule>,
}

/// Compiled policy rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledPolicyRule {
    pub trigger: CompiledTrigger,
    pub actions: Vec<CompiledAction>,
}

/// Compiled injection configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InjectionConfig {
    pub source: String,
    pub target: String,
    pub mode: CompiledInjectionMode,
    pub priority: i32,
    pub max_tokens: Option<i32>,
    pub filter: Option<CompiledFilter>,
}

/// Compiled injection modes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledInjectionMode {
    Full,
    Summary,
    TopK { k: i32 },
    Relevant { threshold: f32 },
}

/// Compiled filter expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledFilter {
    Comparison {
        field: String,
        op: CompiledOperator,
        value: CompiledFilterValue,
    },
    And(Vec<CompiledFilter>),
    Or(Vec<CompiledFilter>),
    Not(Box<CompiledFilter>),
}

/// Compiled filter values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledFilterValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    CurrentTrajectory,
    CurrentScope,
    Now,
    Array(Vec<CompiledFilterValue>),
}

/// Compiled operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledOperator {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    Contains,
    Regex,
    In,
}

/// Compiled trajectory configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryConfig {
    pub name: String,
    pub description: Option<String>,
    pub agent_type: String,
    pub token_budget: i32,
    pub memory_refs: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Compiled agent configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub capabilities: Vec<String>,
    pub constraints: CompiledAgentConstraints,
    pub permissions: CompiledPermissionMatrix,
}

/// Compiled agent constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledAgentConstraints {
    pub max_concurrent: i32,
    pub timeout: Duration,
}

/// Compiled permission matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledPermissionMatrix {
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub lock: Vec<String>,
}

/// Compiled cache configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheConfig {
    pub backend: CompiledCacheBackend,
    pub path: Option<String>,
    pub size_mb: i32,
    pub default_freshness: CompiledFreshness,
    pub max_entries: Option<i32>,
    pub ttl: Option<Duration>,
}

/// Compiled cache backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledCacheBackend {
    Lmdb,
    Memory,
}

/// Compiled freshness configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompiledFreshness {
    BestEffort { max_staleness: Duration },
    Strict,
}

/// Compiled provider configuration from DSL.
/// Note: Different from `caliber_core::config::ProviderConfig` (system bootstrap config).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledProviderConfig {
    pub name: String,
    pub provider_type: CompiledProviderType,
    pub api_key: String,
    pub model: String,
    pub options: HashMap<String, String>,
}

/// Compiled provider types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledProviderType {
    OpenAI,
    Anthropic,
    Custom,
}

/// Compiled evolution configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub name: String,
    pub baseline: String,
    pub candidates: Vec<String>,
    pub benchmark_queries: i32,
    pub metrics: Vec<String>,
}

/// Compiled summarization policy configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummarizationPolicyConfig {
    pub name: String,
    pub triggers: Vec<CompiledTrigger>,
    pub source_level: CompiledAbstractionLevel,
    pub target_level: CompiledAbstractionLevel,
    pub max_sources: i32,
    pub create_edges: bool,
}

/// Compiled abstraction levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledAbstractionLevel {
    Raw,
    Summary,
    Principle,
}

// ============================================================================
// TOOL REGISTRY (PACK-DRIVEN RUNTIME CAPABILITIES)
// ============================================================================

/// Compiled tool kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledToolKind {
    Exec,
    Prompt,
}

/// Compiled tool definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledToolConfig {
    /// Fully-qualified tool id (e.g., "tools.prompts.search")
    pub id: String,
    pub kind: CompiledToolKind,
    pub cmd: Option<String>,
    pub prompt_md: Option<String>,
    /// Path to contract JSON Schema file (original reference).
    pub contract: Option<String>,
    /// Compiled JSON Schema for runtime input validation.
    /// Populated during pack compilation if contract is specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiled_schema: Option<serde_json::Value>,
    pub result_format: Option<String>,
    pub timeout_ms: Option<i32>,
    pub allow_network: Option<bool>,
    pub allow_fs: Option<bool>,
    pub allow_subprocess: Option<bool>,
}

/// Compiled toolset definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledToolsetConfig {
    pub name: String,
    pub tools: Vec<String>,
}

/// Pack agent-to-toolset bindings with extracted markdown metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledPackAgentConfig {
    pub name: String,
    pub toolsets: Vec<String>,
    /// Constraints extracted from ```constraints block in agent markdown.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extracted_constraints: Vec<String>,
    /// Tool references extracted from ```tools block (validated against TOML).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extracted_tool_refs: Vec<String>,
    /// RAG configuration extracted from ```rag block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extracted_rag_config: Option<String>,
}

/// Pack injection metadata for runtime RAG wiring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledPackInjectionConfig {
    pub source: String,
    pub target: String,
    /// Explicit entity type for injection targeting (e.g., "note", "artifact").
    pub entity_type: Option<String>,
    pub mode: CompiledInjectionMode,
    pub priority: i32,
    pub max_tokens: Option<i32>,
}

/// Pack provider routing hints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledPackRoutingConfig {
    pub strategy: Option<String>,
    pub embedding_provider: Option<String>,
    pub summarization_provider: Option<String>,
}

// ============================================================================
// COMPILED CONFIG (THE OUTPUT)
// ============================================================================

/// The complete compiled configuration from a DSL file.
/// This is the output of the compiler and can be used to configure the runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CompiledConfig {
    /// DSL version string
    pub version: String,

    /// Storage adapters
    pub adapters: Vec<AdapterConfig>,

    /// Memory definitions
    pub memories: Vec<MemoryConfig>,

    /// Lifecycle policies
    pub policies: Vec<PolicyConfig>,

    /// Context injections
    pub injections: Vec<InjectionConfig>,

    /// Trajectory templates
    pub trajectories: Vec<TrajectoryConfig>,

    /// Agent type definitions
    pub agents: Vec<AgentConfig>,

    /// Evolution experiments
    pub evolutions: Vec<EvolutionConfig>,

    /// Summarization policies
    pub summarization_policies: Vec<SummarizationPolicyConfig>,

    /// Cache configuration (optional, only one allowed)
    pub cache: Option<CacheConfig>,

    /// LLM providers
    pub providers: Vec<CompiledProviderConfig>,

    /// Pack tool registry (optional; empty when compiling raw DSL)
    #[serde(default)]
    pub tools: Vec<CompiledToolConfig>,

    /// Pack toolsets (optional; empty when compiling raw DSL)
    #[serde(default)]
    pub toolsets: Vec<CompiledToolsetConfig>,

    /// Pack agent bindings (optional; empty when compiling raw DSL)
    #[serde(default)]
    pub pack_agents: Vec<CompiledPackAgentConfig>,

    /// Pack injection metadata (optional; empty when compiling raw DSL)
    #[serde(default)]
    pub pack_injections: Vec<CompiledPackInjectionConfig>,

    /// Pack routing hints (optional)
    #[serde(default)]
    pub pack_routing: Option<CompiledPackRoutingConfig>,

    /// File hashes for artifact determinism (lockfile support).
    /// Maps relative file paths to their SHA-256 hashes.
    /// Used to detect when pack sources change without recompilation.
    #[serde(default)]
    pub file_hashes: HashMap<String, String>,
}

// ============================================================================
// DSL COMPILER
// ============================================================================

/// The DSL Compiler transforms a parsed AST into runtime configuration.
///
/// # Example
///
/// ```ignore
/// let source = r#"caliber "1.0" { ... }"#;
/// let ast = caliber_dsl::parse(source)?;
/// let config = DslCompiler::compile(&ast)?;
/// ```
pub struct DslCompiler {
    /// Current configuration being built
    config: CompiledConfig,

    /// Name registry for duplicate detection
    names: NameRegistry,
}

/// Registry for tracking defined names to detect duplicates.
#[derive(Debug, Default)]
struct NameRegistry {
    adapters: HashMap<String, bool>,
    memories: HashMap<String, bool>,
    policies: HashMap<String, bool>,
    trajectories: HashMap<String, bool>,
    agents: HashMap<String, bool>,
    evolutions: HashMap<String, bool>,
    summarization_policies: HashMap<String, bool>,
    providers: HashMap<String, bool>,
}

impl NameRegistry {
    fn register(&mut self, kind: &str, name: &str) -> CompileResult<()> {
        let map = match kind {
            "adapter" => &mut self.adapters,
            "memory" => &mut self.memories,
            "policy" => &mut self.policies,
            "trajectory" => &mut self.trajectories,
            "agent" => &mut self.agents,
            "evolution" => &mut self.evolutions,
            "summarization_policy" => &mut self.summarization_policies,
            "provider" => &mut self.providers,
            _ => return Ok(()), // Unknown kind, skip
        };

        if map.contains_key(name) {
            return Err(CompileError::DuplicateDefinition {
                kind: kind.to_string(),
                name: name.to_string(),
            });
        }

        map.insert(name.to_string(), true);
        Ok(())
    }
}

impl DslCompiler {
    /// Create a new compiler instance.
    pub fn new() -> Self {
        Self {
            config: CompiledConfig::default(),
            names: NameRegistry::default(),
        }
    }

    /// Compile an AST into a runtime configuration.
    pub fn compile(ast: &CaliberAst) -> CompileResult<CompiledConfig> {
        let mut compiler = Self::new();
        compiler.config.version = ast.version.clone();

        // First pass: register all names for reference checking
        for def in &ast.definitions {
            compiler.register_definition(def)?;
        }

        // Second pass: compile each definition
        for def in &ast.definitions {
            compiler.compile_definition(def)?;
        }

        // Final validation pass
        compiler.validate()?;

        Ok(compiler.config)
    }

    /// Register a definition's name for duplicate detection.
    fn register_definition(&mut self, def: &Definition) -> CompileResult<()> {
        match def {
            Definition::Adapter(d) => self.names.register("adapter", &d.name),
            Definition::Memory(d) => self.names.register("memory", &d.name),
            Definition::Policy(d) => self.names.register("policy", &d.name),
            Definition::Trajectory(d) => self.names.register("trajectory", &d.name),
            Definition::Agent(d) => self.names.register("agent", &d.name),
            Definition::Evolution(d) => self.names.register("evolution", &d.name),
            Definition::SummarizationPolicy(d) => {
                self.names.register("summarization_policy", &d.name)
            }
            Definition::Provider(d) => self.names.register("provider", &d.name),
            Definition::Injection(_) | Definition::Cache(_) => Ok(()), // Anonymous or singleton
        }
    }

    /// Compile a single definition.
    fn compile_definition(&mut self, def: &Definition) -> CompileResult<()> {
        match def {
            Definition::Adapter(d) => {
                let config = Self::compile_adapter(d)?;
                self.config.adapters.push(config);
            }
            Definition::Memory(d) => {
                let config = Self::compile_memory(d)?;
                self.config.memories.push(config);
            }
            Definition::Policy(d) => {
                let config = Self::compile_policy(d)?;
                self.config.policies.push(config);
            }
            Definition::Injection(d) => {
                let config = Self::compile_injection(d)?;
                self.config.injections.push(config);
            }
            Definition::Evolution(d) => {
                let config = Self::compile_evolution(d)?;
                self.config.evolutions.push(config);
            }
            Definition::SummarizationPolicy(d) => {
                let config = Self::compile_summarization_policy(d)?;
                self.config.summarization_policies.push(config);
            }
            Definition::Trajectory(d) => {
                let config = Self::compile_trajectory(d)?;
                self.config.trajectories.push(config);
            }
            Definition::Agent(d) => {
                let config = Self::compile_agent(d)?;
                self.config.agents.push(config);
            }
            Definition::Cache(d) => {
                if self.config.cache.is_some() {
                    return Err(CompileError::DuplicateDefinition {
                        kind: "cache".to_string(),
                        name: "cache".to_string(),
                    });
                }
                let config = Self::compile_cache(d)?;
                self.config.cache = Some(config);
            }
            Definition::Provider(d) => {
                let config = Self::compile_provider(d)?;
                self.config.providers.push(config);
            }
        }
        Ok(())
    }

    /// Compile an adapter definition.
    fn compile_adapter(def: &AdapterDef) -> CompileResult<AdapterConfig> {
        let adapter_type = match def.adapter_type {
            AdapterType::Postgres => CompiledAdapterType::Postgres,
            AdapterType::Redis => CompiledAdapterType::Redis,
            AdapterType::Memory => CompiledAdapterType::Memory,
        };

        let options: HashMap<String, String> = def.options.iter().cloned().collect();

        Ok(AdapterConfig {
            name: def.name.clone(),
            adapter_type,
            connection: def.connection.clone(),
            options,
        })
    }

    /// Compile a memory definition.
    fn compile_memory(def: &MemoryDef) -> CompileResult<MemoryConfig> {
        let memory_type = Self::compile_memory_type(&def.memory_type)?;
        let schema = def
            .schema
            .iter()
            .map(Self::compile_field)
            .collect::<CompileResult<Vec<_>>>()?;
        let retention = Self::compile_retention(&def.retention)?;
        let lifecycle = Self::compile_lifecycle(&def.lifecycle)?;
        let indexes = def
            .indexes
            .iter()
            .map(Self::compile_index)
            .collect::<CompileResult<Vec<_>>>()?;
        let inject_on = def
            .inject_on
            .iter()
            .map(Self::compile_trigger)
            .collect::<CompileResult<Vec<_>>>()?;
        let modifiers = Self::compile_modifiers(&def.modifiers)?;

        Ok(MemoryConfig {
            name: def.name.clone(),
            memory_type,
            schema,
            retention,
            lifecycle,
            parent: def.parent.clone(),
            indexes,
            inject_on,
            artifacts: def.artifacts.clone(),
            modifiers,
        })
    }

    fn compile_memory_type(mt: &MemoryType) -> CompileResult<MemoryCategory> {
        Ok(match mt {
            MemoryType::Ephemeral => MemoryCategory::Ephemeral,
            MemoryType::Working => MemoryCategory::Working,
            MemoryType::Episodic => MemoryCategory::Episodic,
            MemoryType::Semantic => MemoryCategory::Semantic,
            MemoryType::Procedural => MemoryCategory::Procedural,
            MemoryType::Meta => MemoryCategory::Meta,
        })
    }

    fn compile_field(def: &FieldDef) -> CompileResult<FieldConfig> {
        let field_type = Self::compile_field_type(&def.field_type)?;
        let security = def.security.as_ref().map(Self::compile_field_security);
        Ok(FieldConfig {
            name: def.name.clone(),
            field_type,
            nullable: def.nullable,
            security,
        })
    }

    fn compile_field_security(sec: &FieldSecurity) -> CompiledFieldSecurity {
        use crate::parser::ast::PIIClassification;
        CompiledFieldSecurity {
            classification: match sec.classification {
                PIIClassification::Public => CompiledPIIClassification::Public,
                PIIClassification::Internal => CompiledPIIClassification::Internal,
                PIIClassification::Confidential => CompiledPIIClassification::Confidential,
                PIIClassification::Restricted => CompiledPIIClassification::Restricted,
                PIIClassification::Secret => CompiledPIIClassification::Secret,
            },
            opaque: sec.opaque,
            immutable: sec.immutable,
            audited: sec.audited,
            redact_in_logs: sec.redact_in_logs,
            env_source: sec.env_source.clone(),
        }
    }

    fn compile_field_type(ft: &FieldType) -> CompileResult<caliber_core::FieldType> {
        use caliber_core::FieldType as CoreFieldType;
        Ok(match ft {
            FieldType::Uuid => CoreFieldType::Uuid,
            FieldType::Text => CoreFieldType::Text,
            FieldType::Int => CoreFieldType::Int,
            FieldType::Float => CoreFieldType::Float,
            FieldType::Bool => CoreFieldType::Bool,
            FieldType::Timestamp => CoreFieldType::Timestamp,
            FieldType::Json => CoreFieldType::Json,
            FieldType::Embedding(dimensions) => CoreFieldType::Embedding {
                dimensions: *dimensions,
            },
            FieldType::Enum(variants) => CoreFieldType::Enum {
                variants: variants.clone(),
            },
            FieldType::Array(inner) => {
                let compiled_inner = Self::compile_field_type(inner)?;
                CoreFieldType::Array(Box::new(compiled_inner))
            }
        })
    }

    fn compile_retention(ret: &Retention) -> CompileResult<caliber_core::TTL> {
        use caliber_core::TTL;
        Ok(match ret {
            Retention::Persistent => TTL::Persistent,
            Retention::Session => TTL::Session,
            Retention::Scope => TTL::Scope,
            Retention::Duration(s) => {
                let duration = Self::parse_duration(s)?;
                // Convert Duration to milliseconds for TTL
                TTL::Duration(duration.as_millis() as i64)
            }
            Retention::Max(n) => TTL::Max(*n),
        })
    }

    fn compile_lifecycle(lc: &Lifecycle) -> CompileResult<CompiledLifecycle> {
        Ok(match lc {
            Lifecycle::Explicit => CompiledLifecycle::Explicit,
            Lifecycle::AutoClose(trigger) => {
                let compiled_trigger = Self::compile_trigger(trigger)?;
                CompiledLifecycle::AutoClose(compiled_trigger)
            }
        })
    }

    fn compile_trigger(trigger: &Trigger) -> CompileResult<CompiledTrigger> {
        Ok(match trigger {
            Trigger::TaskStart => CompiledTrigger::TaskStart,
            Trigger::TaskEnd => CompiledTrigger::TaskEnd,
            Trigger::ScopeClose => CompiledTrigger::ScopeClose,
            Trigger::TurnEnd => CompiledTrigger::TurnEnd,
            Trigger::Manual => CompiledTrigger::Manual,
            Trigger::Schedule(s) => CompiledTrigger::Schedule(s.clone()),
        })
    }

    fn compile_action(action: &Action) -> CompileResult<CompiledAction> {
        Ok(match action {
            Action::Summarize(target) => CompiledAction::Summarize {
                target: target.clone(),
            },
            Action::ExtractArtifacts(target) => CompiledAction::ExtractArtifacts {
                target: target.clone(),
            },
            Action::Checkpoint(target) => CompiledAction::Checkpoint {
                target: target.clone(),
            },
            Action::Prune { target, criteria } => CompiledAction::Prune {
                target: target.clone(),
                criteria: Self::compile_filter(criteria)?,
            },
            Action::Notify(target) => CompiledAction::Notify {
                target: target.clone(),
            },
            Action::Inject { target, mode } => CompiledAction::Inject {
                target: target.clone(),
                mode: Self::compile_injection_mode(mode)?,
            },
            Action::AutoSummarize {
                source_level,
                target_level,
                create_edges,
            } => CompiledAction::AutoSummarize {
                source_level: Self::compile_abstraction_level(source_level)?,
                target_level: Self::compile_abstraction_level(target_level)?,
                create_edges: *create_edges,
            },
        })
    }

    fn compile_index(def: &IndexDef) -> CompileResult<IndexConfig> {
        let index_type = match def.index_type {
            IndexType::Btree => CompiledIndexType::Btree,
            IndexType::Hash => CompiledIndexType::Hash,
            IndexType::Gin => CompiledIndexType::Gin,
            IndexType::Hnsw => CompiledIndexType::Hnsw,
            IndexType::Ivfflat => CompiledIndexType::Ivfflat,
        };

        Ok(IndexConfig {
            field: def.field.clone(),
            index_type,
            options: def.options.iter().cloned().collect(),
        })
    }

    fn compile_modifiers(modifiers: &[ModifierDef]) -> CompileResult<MemoryModifiers> {
        let mut result = MemoryModifiers::default();

        for modifier in modifiers {
            match modifier {
                ModifierDef::Embeddable { provider } => {
                    result.embeddable = Some(EmbeddableConfig {
                        provider: provider.clone(),
                    });
                }
                ModifierDef::Summarizable { style, on_triggers } => {
                    let compiled_style = match style {
                        SummaryStyle::Brief => SummarizeStyle::Brief,
                        SummaryStyle::Detailed => SummarizeStyle::Detailed,
                    };
                    let triggers = on_triggers
                        .iter()
                        .map(Self::compile_trigger)
                        .collect::<CompileResult<Vec<_>>>()?;
                    result.summarizable = Some(SummarizableConfig {
                        style: compiled_style,
                        on_triggers: triggers,
                    });
                }
                ModifierDef::Lockable { mode } => {
                    let compiled_mode = match mode {
                        LockMode::Exclusive => CompiledLockMode::Exclusive,
                        LockMode::Shared => CompiledLockMode::Shared,
                    };
                    result.lockable = Some(LockableConfig {
                        mode: compiled_mode,
                    });
                }
            }
        }

        Ok(result)
    }

    /// Compile a policy definition.
    fn compile_policy(def: &PolicyDef) -> CompileResult<PolicyConfig> {
        let rules = def
            .rules
            .iter()
            .map(Self::compile_policy_rule)
            .collect::<CompileResult<Vec<_>>>()?;
        Ok(PolicyConfig {
            name: def.name.clone(),
            rules,
        })
    }

    fn compile_policy_rule(rule: &PolicyRule) -> CompileResult<CompiledPolicyRule> {
        let trigger = Self::compile_trigger(&rule.trigger)?;
        let actions = rule
            .actions
            .iter()
            .map(Self::compile_action)
            .collect::<CompileResult<Vec<_>>>()?;
        Ok(CompiledPolicyRule {
            trigger,
            actions,
        })
    }

    /// Compile an injection definition.
    fn compile_injection(def: &InjectionDef) -> CompileResult<InjectionConfig> {
        let mode = Self::compile_injection_mode(&def.mode)?;
        let filter = def
            .filter
            .as_ref()
            .map(Self::compile_filter)
            .transpose()?;

        Ok(InjectionConfig {
            source: def.source.clone(),
            target: def.target.clone(),
            mode,
            priority: def.priority,
            max_tokens: def.max_tokens,
            filter,
        })
    }

    fn compile_injection_mode(mode: &InjectionMode) -> CompileResult<CompiledInjectionMode> {
        Ok(match mode {
            InjectionMode::Full => CompiledInjectionMode::Full,
            InjectionMode::Summary => CompiledInjectionMode::Summary,
            InjectionMode::TopK(k) => CompiledInjectionMode::TopK { k: *k as i32 },
            InjectionMode::Relevant(threshold) => CompiledInjectionMode::Relevant {
                threshold: *threshold,
            },
        })
    }

    fn compile_filter(expr: &FilterExpr) -> CompileResult<CompiledFilter> {
        Ok(match expr {
            FilterExpr::Comparison { field, op, value } => {
                let compiled_op = Self::compile_compare_op(op)?;
                let compiled_value = Self::compile_filter_value(value)?;
                CompiledFilter::Comparison {
                    field: field.clone(),
                    op: compiled_op,
                    value: compiled_value,
                }
            }
            FilterExpr::And(exprs) => {
                let compiled = exprs
                    .iter()
                    .map(Self::compile_filter)
                    .collect::<CompileResult<Vec<_>>>()?;
                CompiledFilter::And(compiled)
            }
            FilterExpr::Or(exprs) => {
                let compiled = exprs
                    .iter()
                    .map(Self::compile_filter)
                    .collect::<CompileResult<Vec<_>>>()?;
                CompiledFilter::Or(compiled)
            }
            FilterExpr::Not(inner) => {
                let compiled_inner = Self::compile_filter(inner)?;
                CompiledFilter::Not(Box::new(compiled_inner))
            }
        })
    }

    fn compile_compare_op(op: &CompareOp) -> CompileResult<CompiledOperator> {
        Ok(match op {
            CompareOp::Eq => CompiledOperator::Eq,
            CompareOp::Ne => CompiledOperator::Ne,
            CompareOp::Gt => CompiledOperator::Gt,
            CompareOp::Lt => CompiledOperator::Lt,
            CompareOp::Ge => CompiledOperator::Ge,
            CompareOp::Le => CompiledOperator::Le,
            CompareOp::Contains => CompiledOperator::Contains,
            CompareOp::Regex => CompiledOperator::Regex,
            CompareOp::In => CompiledOperator::In,
        })
    }

    fn compile_filter_value(value: &FilterValue) -> CompileResult<CompiledFilterValue> {
        Ok(match value {
            FilterValue::String(s) => CompiledFilterValue::String(s.clone()),
            FilterValue::Number(n) => CompiledFilterValue::Number(*n),
            FilterValue::Bool(b) => CompiledFilterValue::Bool(*b),
            FilterValue::Null => CompiledFilterValue::Null,
            FilterValue::CurrentTrajectory => CompiledFilterValue::CurrentTrajectory,
            FilterValue::CurrentScope => CompiledFilterValue::CurrentScope,
            FilterValue::Now => CompiledFilterValue::Now,
            FilterValue::Array(arr) => {
                let compiled = arr
                    .iter()
                    .map(Self::compile_filter_value)
                    .collect::<CompileResult<Vec<_>>>()?;
                CompiledFilterValue::Array(compiled)
            }
        })
    }

    /// Compile a trajectory definition.
    fn compile_trajectory(def: &TrajectoryDef) -> CompileResult<TrajectoryConfig> {
        if def.token_budget <= 0 {
            return Err(CompileError::InvalidValue {
                field: "token_budget".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        Ok(TrajectoryConfig {
            name: def.name.clone(),
            description: def.description.clone(),
            agent_type: def.agent_type.clone(),
            token_budget: def.token_budget,
            memory_refs: def.memory_refs.clone(),
            metadata: def.metadata.clone(),
        })
    }

    /// Compile an agent definition.
    fn compile_agent(def: &AgentDef) -> CompileResult<AgentConfig> {
        let constraints = CompiledAgentConstraints {
            max_concurrent: def.constraints.max_concurrent,
            timeout: Duration::from_millis(def.constraints.timeout_ms as u64),
        };

        let permissions = CompiledPermissionMatrix {
            read: def.permissions.read.clone(),
            write: def.permissions.write.clone(),
            lock: def.permissions.lock.clone(),
        };

        Ok(AgentConfig {
            name: def.name.clone(),
            capabilities: def.capabilities.clone(),
            constraints,
            permissions,
        })
    }

    /// Compile a cache definition.
    fn compile_cache(def: &CacheDef) -> CompileResult<CacheConfig> {
        let backend = match def.backend {
            CacheBackendType::Lmdb => CompiledCacheBackend::Lmdb,
            CacheBackendType::Memory => CompiledCacheBackend::Memory,
        };

        let default_freshness = Self::compile_freshness(&def.default_freshness)?;

        let ttl = def
            .ttl
            .as_ref()
            .map(|s| Self::parse_duration(s))
            .transpose()?;

        Ok(CacheConfig {
            backend,
            path: def.path.clone(),
            size_mb: def.size_mb,
            default_freshness,
            max_entries: def.max_entries,
            ttl,
        })
    }

    fn compile_freshness(def: &FreshnessDef) -> CompileResult<CompiledFreshness> {
        Ok(match def {
            FreshnessDef::BestEffort { max_staleness } => {
                let duration = Self::parse_duration(max_staleness)?;
                CompiledFreshness::BestEffort {
                    max_staleness: duration,
                }
            }
            FreshnessDef::Strict => CompiledFreshness::Strict,
        })
    }

    /// Compile a provider definition.
    fn compile_provider(def: &ProviderDef) -> CompileResult<CompiledProviderConfig> {
        let provider_type = match def.provider_type {
            ProviderType::OpenAI => CompiledProviderType::OpenAI,
            ProviderType::Anthropic => CompiledProviderType::Anthropic,
            ProviderType::Custom => CompiledProviderType::Custom,
        };

        let api_key = Self::resolve_env_value(&def.api_key)?;
        let options: HashMap<String, String> = def.options.iter().cloned().collect();

        Ok(CompiledProviderConfig {
            name: def.name.clone(),
            provider_type,
            api_key,
            model: def.model.clone(),
            options,
        })
    }

    fn resolve_env_value(env: &EnvValue) -> CompileResult<String> {
        // At compile time, we just return a placeholder or the literal
        // Actual env var resolution happens at runtime
        Ok(match env {
            EnvValue::Env(var_name) => format!("${{env:{}}}", var_name),
            EnvValue::Literal(value) => value.clone(),
        })
    }

    /// Compile an evolution definition.
    fn compile_evolution(def: &EvolutionDef) -> CompileResult<EvolutionConfig> {
        if def.benchmark_queries <= 0 {
            return Err(CompileError::InvalidValue {
                field: "benchmark_queries".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        Ok(EvolutionConfig {
            name: def.name.clone(),
            baseline: def.baseline.clone(),
            candidates: def.candidates.clone(),
            benchmark_queries: def.benchmark_queries,
            metrics: def.metrics.clone(),
        })
    }

    /// Compile a summarization policy definition.
    fn compile_summarization_policy(
        def: &SummarizationPolicyDef,
    ) -> CompileResult<SummarizationPolicyConfig> {
        let triggers = def
            .triggers
            .iter()
            .map(Self::compile_summarization_trigger)
            .collect::<CompileResult<Vec<_>>>()?;
        let source_level = Self::compile_abstraction_level(&def.source_level)?;
        let target_level = Self::compile_abstraction_level(&def.target_level)?;

        if def.max_sources <= 0 {
            return Err(CompileError::InvalidValue {
                field: "max_sources".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        Ok(SummarizationPolicyConfig {
            name: def.name.clone(),
            triggers,
            source_level,
            target_level,
            max_sources: def.max_sources,
            create_edges: def.create_edges,
        })
    }

    fn compile_summarization_trigger(
        trigger: &SummarizationTriggerDsl,
    ) -> CompileResult<CompiledTrigger> {
        Ok(match trigger {
            SummarizationTriggerDsl::DosageThreshold { percent } => {
                CompiledTrigger::DosageReached {
                    threshold: *percent as i32,
                }
            }
            SummarizationTriggerDsl::ScopeClose => CompiledTrigger::ScopeClose,
            SummarizationTriggerDsl::TurnCount { count } => CompiledTrigger::TurnCount { count: *count },
            SummarizationTriggerDsl::ArtifactCount { count } => {
                CompiledTrigger::ArtifactCount { count: *count }
            }
            SummarizationTriggerDsl::Manual => CompiledTrigger::Manual,
        })
    }

    fn compile_abstraction_level(
        level: &AbstractionLevelDsl,
    ) -> CompileResult<CompiledAbstractionLevel> {
        Ok(match level {
            AbstractionLevelDsl::Raw => CompiledAbstractionLevel::Raw,
            AbstractionLevelDsl::Summary => CompiledAbstractionLevel::Summary,
            AbstractionLevelDsl::Principle => CompiledAbstractionLevel::Principle,
        })
    }

    /// Parse a duration string (e.g., "30s", "5m", "1h", "24h").
    fn parse_duration(s: &str) -> CompileResult<Duration> {
        let s = s.trim();
        if s.is_empty() {
            return Err(CompileError::InvalidDuration {
                value: s.to_string(),
            });
        }

        // Find where the number ends and unit begins
        let num_end = s
            .chars()
            .position(|c| !c.is_ascii_digit() && c != '.')
            .unwrap_or(s.len());

        let (num_str, unit) = s.split_at(num_end);
        let num: f64 = num_str.parse().map_err(|_| CompileError::InvalidDuration {
            value: s.to_string(),
        })?;

        let multiplier = match unit.trim() {
            "ms" => 1,
            "s" => 1000,
            "m" => 60 * 1000,
            "h" => 60 * 60 * 1000,
            "d" => 24 * 60 * 60 * 1000,
            _ => {
                return Err(CompileError::InvalidDuration {
                    value: s.to_string(),
                })
            }
        };

        Ok(Duration::from_millis((num * multiplier as f64) as u64))
    }

    /// Final validation pass - check cross-references.
    fn validate(&self) -> CompileResult<()> {
        // Validate agent references in trajectories
        for trajectory in &self.config.trajectories {
            if !self.names.agents.contains_key(&trajectory.agent_type) {
                return Err(CompileError::UndefinedReference {
                    kind: "agent".to_string(),
                    name: trajectory.agent_type.clone(),
                });
            }
        }

        // Validate memory references in trajectories
        for trajectory in &self.config.trajectories {
            for memory_ref in &trajectory.memory_refs {
                if !self.names.memories.contains_key(memory_ref) {
                    return Err(CompileError::UndefinedReference {
                        kind: "memory".to_string(),
                        name: memory_ref.clone(),
                    });
                }
            }
        }

        // Validate agent permission references to memories
        for agent in &self.config.agents {
            for mem in &agent.permissions.read {
                if !self.names.memories.contains_key(mem) {
                    return Err(CompileError::UndefinedReference {
                        kind: "memory".to_string(),
                        name: mem.clone(),
                    });
                }
            }
            for mem in &agent.permissions.write {
                if !self.names.memories.contains_key(mem) {
                    return Err(CompileError::UndefinedReference {
                        kind: "memory".to_string(),
                        name: mem.clone(),
                    });
                }
            }
            for mem in &agent.permissions.lock {
                if !self.names.memories.contains_key(mem) {
                    return Err(CompileError::UndefinedReference {
                        kind: "memory".to_string(),
                        name: mem.clone(),
                    });
                }
            }
        }

        // Validate injection source/target references
        for injection in &self.config.injections {
            if !self.names.memories.contains_key(&injection.source) {
                return Err(CompileError::UndefinedReference {
                    kind: "memory".to_string(),
                    name: injection.source.clone(),
                });
            }
            // Target could be a context slot, not necessarily a memory
        }

        // Validate evolution baseline/candidate references
        // (These reference snapshot names which are runtime entities, skip for now)

        // Validate parent memory references
        for memory in &self.config.memories {
            if let Some(ref parent) = memory.parent {
                if !self.names.memories.contains_key(parent) {
                    return Err(CompileError::UndefinedReference {
                        kind: "memory".to_string(),
                        name: parent.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

impl Default for DslCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_parse_duration() {
        assert_eq!(
            DslCompiler::parse_duration("30s").unwrap(),
            Duration::from_secs(30)
        );
        assert_eq!(
            DslCompiler::parse_duration("5m").unwrap(),
            Duration::from_secs(300)
        );
        assert_eq!(
            DslCompiler::parse_duration("1h").unwrap(),
            Duration::from_secs(3600)
        );
        assert_eq!(
            DslCompiler::parse_duration("100ms").unwrap(),
            Duration::from_millis(100)
        );
        assert_eq!(
            DslCompiler::parse_duration("1d").unwrap(),
            Duration::from_secs(86400)
        );
    }

    #[test]
    fn test_duplicate_detection() {
        let mut registry = NameRegistry::default();
        registry.register("adapter", "pg").unwrap();
        let err = registry.register("adapter", "pg").unwrap_err();
        assert!(matches!(err, CompileError::DuplicateDefinition { .. }));
    }

    #[test]
    fn test_parse_duration_invalid() {
        let err = DslCompiler::parse_duration("10weeks").unwrap_err();
        assert!(matches!(err, CompileError::InvalidDuration(_)));
    }

    #[test]
    fn test_compile_undefined_agent_reference() {
        let source = r#"
            caliber: "1.0" {
                trajectory "t1" {
                    agent_type: "missing"
                    token_budget: 1000
                }
            }
        "#;
        let ast = parse(source).unwrap();
        let err = DslCompiler::compile(&ast).unwrap_err();
        assert!(matches!(
            err,
            CompileError::UndefinedReference { kind, .. } if kind == "agent"
        ));
    }

    #[test]
    fn test_compile_undefined_memory_reference() {
        let source = r#"
            caliber: "1.0" {
                agent "worker" {
                }
                trajectory "t1" {
                    agent_type: "worker"
                    token_budget: 1000
                    memory_refs: [notes]
                }
            }
        "#;
        let ast = parse(source).unwrap();
        let err = DslCompiler::compile(&ast).unwrap_err();
        assert!(matches!(
            err,
            CompileError::UndefinedReference { kind, .. } if kind == "memory"
        ));
    }

    #[test]
    fn test_compile_invalid_token_budget() {
        let source = r#"
            caliber: "1.0" {
                agent "worker" {
                }
                trajectory "t1" {
                    agent_type: "worker"
                    token_budget: 0
                }
            }
        "#;
        let ast = parse(source).unwrap();
        let err = DslCompiler::compile(&ast).unwrap_err();
        assert!(matches!(
            err,
            CompileError::InvalidValue { field, .. } if field == "token_budget"
        ));
    }

    #[test]
    fn test_compile_invalid_evolution_benchmark_queries() {
        let source = r#"
            caliber: "1.0" {
                evolve "config" {
                    baseline: "base"
                    candidates: ["c1"]
                    benchmark_queries: 0
                    metrics: ["latency"]
                }
            }
        "#;
        let ast = parse(source).unwrap();
        let err = DslCompiler::compile(&ast).unwrap_err();
        assert!(matches!(
            err,
            CompileError::InvalidValue { field, .. } if field == "benchmark_queries"
        ));
    }
}
