//! Abstract Syntax Tree types

use serde::{Deserialize, Serialize};

// ============================================================================
// AST TYPES (Task 4.1)
// ============================================================================

/// The root AST node for a CALIBER configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaliberAst {
    pub version: String,
    pub definitions: Vec<Definition>,
}

/// A top-level definition in the DSL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Definition {
    Adapter(AdapterDef),
    Memory(MemoryDef),
    Policy(PolicyDef),
    Injection(InjectionDef),
    // Battle Intel Feature 3: Evolution mode
    Evolution(EvolutionDef),
    // Battle Intel Feature 4: Summarization policies
    SummarizationPolicy(SummarizationPolicyDef),
    // DSL-first architecture: New definitions
    Trajectory(TrajectoryDef),
    Agent(AgentDef),
    Cache(CacheDef),
    Provider(ProviderDef),
}

/// Adapter definition for storage backends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdapterDef {
    pub name: String,
    pub adapter_type: AdapterType,
    pub connection: String,
    pub options: Vec<(String, String)>,
}

/// Supported adapter types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterType {
    Postgres,
    Redis,
    Memory,
}

/// Memory definition for memory types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryDef {
    pub name: String,
    pub memory_type: MemoryType,
    pub schema: Vec<FieldDef>,
    pub retention: Retention,
    pub lifecycle: Lifecycle,
    pub parent: Option<String>,
    pub indexes: Vec<IndexDef>,
    pub inject_on: Vec<Trigger>,
    pub artifacts: Vec<String>,
    /// DSL-first: Memory modifiers (embeddable, summarizable, lockable)
    pub modifiers: Vec<ModifierDef>,
}

/// Memory type categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    Ephemeral,
    Working,
    Episodic,
    Semantic,
    Procedural,
    Meta,
}

/// Field definition in a schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub nullable: bool,
    pub default: Option<String>,
    /// Optional security configuration for PII fields.
    pub security: Option<FieldSecurity>,
}

/// Field types supported in schemas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    Uuid,
    Text,
    Int,
    Float,
    Bool,
    Timestamp,
    Json,
    Embedding(Option<usize>),
    Enum(Vec<String>),
    Array(Box<FieldType>),
}

/// Retention policy for memory entries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Retention {
    Persistent,
    Session,
    Scope,
    Duration(String),
    Max(usize),
}

/// Lifecycle management for memory entries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Lifecycle {
    Explicit,
    AutoClose(Trigger),
}

/// Trigger events for policies and lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Trigger {
    TaskStart,
    TaskEnd,
    ScopeClose,
    TurnEnd,
    Manual,
    Schedule(String),
}

/// Index definition for memory fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexDef {
    pub field: String,
    pub index_type: IndexType,
    pub options: Vec<(String, String)>,
}

/// Supported index types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    Btree,
    Hash,
    Gin,
    Hnsw,
    Ivfflat,
}

/// Policy definition with trigger-action rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyDef {
    pub name: String,
    pub rules: Vec<PolicyRule>,
}

/// A single policy rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyRule {
    pub trigger: Trigger,
    pub actions: Vec<Action>,
}

/// Actions that can be triggered by policies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Summarize(String),
    ExtractArtifacts(String),
    Checkpoint(String),
    Prune {
        target: String,
        criteria: FilterExpr,
    },
    Notify(String),
    Inject {
        target: String,
        mode: InjectionMode,
    },
    // Battle Intel Feature 4: Auto-summarization action
    AutoSummarize {
        source_level: AbstractionLevelDsl,
        target_level: AbstractionLevelDsl,
        create_edges: bool,
    },
}

/// Injection definition for context assembly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InjectionDef {
    pub source: String,
    pub target: String,
    pub mode: InjectionMode,
    pub priority: i32,
    pub max_tokens: Option<i32>,
    pub filter: Option<FilterExpr>,
}

/// Injection modes for context assembly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InjectionMode {
    Full,
    Summary,
    TopK(usize),
    Relevant(f32),
}

/// Filter expression for queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterExpr {
    Comparison {
        field: String,
        op: CompareOp,
        value: FilterValue,
    },
    And(Vec<FilterExpr>),
    Or(Vec<FilterExpr>),
    Not(Box<FilterExpr>),
}

/// Comparison operators for filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
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

/// Values that can be used in filter expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    CurrentTrajectory,
    CurrentScope,
    Now,
    Array(Vec<FilterValue>),
}

// ============================================================================
// BATTLE INTEL FEATURE 2: ABSTRACTION LEVELS
// ============================================================================

/// Abstraction level for DSL (mirrors caliber_core::AbstractionLevel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbstractionLevelDsl {
    Raw,       // L0: Direct observation
    Summary,   // L1: Synthesized from L0s
    Principle, // L2: High-level abstraction
}

// ============================================================================
// BATTLE INTEL FEATURE 3: EVOLUTION MODE (MemEvolve-inspired)
// ============================================================================

/// Evolution definition for DSL config benchmarking.
///
/// DSL syntax:
/// ```text
/// evolution "memory_optimization" {
///     baseline: "current_prod"
///     candidates: ["hybrid_search", "aggressive_summarize"]
///     benchmark_queries: 100
///     metrics: ["retrieval_accuracy", "token_efficiency"]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionDef {
    pub name: String,
    /// Snapshot name to compare against
    pub baseline: String,
    /// Candidate config names to test
    pub candidates: Vec<String>,
    /// Number of queries to benchmark
    pub benchmark_queries: i32,
    /// Metrics to track
    pub metrics: Vec<String>,
}

// ============================================================================
// BATTLE INTEL FEATURE 4: SUMMARIZATION POLICIES
// ============================================================================

/// Summarization trigger types (mirrors caliber_core::SummarizationTrigger).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SummarizationTriggerDsl {
    /// Trigger when token usage reaches threshold percent
    DosageThreshold { percent: u8 },
    /// Trigger when scope closes
    ScopeClose,
    /// Trigger every N turns
    TurnCount { count: i32 },
    /// Trigger every N artifacts
    ArtifactCount { count: i32 },
    /// Manual trigger only
    Manual,
}

/// Summarization policy definition.
///
/// DSL syntax:
/// ```text
/// summarization_policy "auto_abstract" {
///     triggers: [dosage_reached(80), scope_close]
///     source_level: raw
///     target_level: summary
///     max_sources: 10
///     create_edges: true
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummarizationPolicyDef {
    pub name: String,
    pub triggers: Vec<SummarizationTriggerDsl>,
    pub source_level: AbstractionLevelDsl,
    pub target_level: AbstractionLevelDsl,
    pub max_sources: i32,
    pub create_edges: bool,
}

// ============================================================================
// DSL-FIRST ARCHITECTURE: TRAJECTORY DEFINITIONS
// ============================================================================

/// Trajectory definition for multi-turn interaction templates.
///
/// DSL syntax:
/// ```text
/// trajectory "customer_support" {
///     description: "Multi-turn customer support interaction"
///     agent_type: "support_agent"
///     token_budget: 8000
///     memory_refs: [artifacts, notes, scopes]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryDef {
    pub name: String,
    pub description: Option<String>,
    pub agent_type: String,
    pub token_budget: i32,
    pub memory_refs: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// DSL-FIRST ARCHITECTURE: AGENT DEFINITIONS
// ============================================================================

/// Agent definition for agent types and capabilities.
///
/// DSL syntax:
/// ```text
/// agent "support_agent" {
///     capabilities: ["classify_issue", "search_kb", "escalate"]
///     constraints: {
///         max_concurrent: 5
///         timeout_ms: 30000
///     }
///     permissions: {
///         read: [artifacts, notes, scopes]
///         write: [notes, scopes]
///         lock: [scopes]
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub capabilities: Vec<String>,
    pub constraints: AgentConstraints,
    pub permissions: PermissionMatrix,
}

/// Agent runtime constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentConstraints {
    pub max_concurrent: i32,
    pub timeout_ms: i64,
}

impl Default for AgentConstraints {
    fn default() -> Self {
        Self {
            max_concurrent: 1,
            timeout_ms: 30000,
        }
    }
}

/// Permission matrix for agent access control.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PermissionMatrix {
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub lock: Vec<String>,
}

// ============================================================================
// DSL-FIRST ARCHITECTURE: CACHE CONFIGURATION
// ============================================================================

/// Cache configuration for the Three Dragons architecture.
///
/// DSL syntax:
/// ```text
/// cache {
///     backend: lmdb
///     path: "/var/caliber/cache"
///     size_mb: 1024
///     default_freshness: best_effort { max_staleness: 60s }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheDef {
    pub backend: CacheBackendType,
    pub path: Option<String>,
    pub size_mb: i32,
    pub default_freshness: FreshnessDef,
    pub max_entries: Option<i32>,
    pub ttl: Option<String>,
}

/// Cache backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CacheBackendType {
    Lmdb,
    Memory,
}

/// Freshness policy definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FreshnessDef {
    /// Best-effort freshness with max staleness tolerance
    BestEffort { max_staleness: String },
    /// Strict freshness - always fetch from source
    Strict,
}

impl Default for FreshnessDef {
    fn default() -> Self {
        FreshnessDef::BestEffort {
            max_staleness: "60s".to_string(),
        }
    }
}

// ============================================================================
// DSL-FIRST ARCHITECTURE: PROVIDER DEFINITIONS
// ============================================================================

/// LLM provider definition for embeddings and summarization.
///
/// DSL syntax:
/// ```text
/// provider "openai" {
///     type: openai
///     api_key: env("OPENAI_API_KEY")
///     model: "text-embedding-3-small"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderDef {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: EnvValue,
    pub model: String,
    pub options: Vec<(String, String)>,
}

/// LLM provider types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Custom,
}

/// Environment variable reference or literal value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnvValue {
    /// Reference to environment variable: env("VAR_NAME")
    Env(String),
    /// Literal string value
    Literal(String),
}

// ============================================================================
// DSL-FIRST ARCHITECTURE: MEMORY MODIFIERS
// ============================================================================

/// Memory modifier types for embeddable, summarizable, lockable.
///
/// DSL syntax in memory definition:
/// ```text
/// memory artifacts {
///     modifiers: [
///         embeddable { provider: "openai" },
///         summarizable { style: brief, on: [scope_close] }
///     ]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModifierDef {
    /// Embeddable modifier - enables vector embeddings
    Embeddable { provider: String },
    /// Summarizable modifier - enables auto-summarization
    Summarizable {
        style: SummaryStyle,
        on_triggers: Vec<Trigger>,
    },
    /// Lockable modifier - enables distributed locking
    Lockable { mode: LockMode },
}

/// Summary style for summarizable modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SummaryStyle {
    Brief,
    Detailed,
}

/// Lock mode for lockable modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockMode {
    Exclusive,
    Shared,
}

// ============================================================================
// PII & SECURITY AST TYPES (Phase 3)
// ============================================================================

/// PII/sensitivity classification for a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PIIClassification {
    /// No restrictions
    #[default]
    Public,
    /// Internal use only
    Internal,
    /// Limited access
    Confidential,
    /// Highly restricted
    Restricted,
    /// Secret - encrypted, redacted in logs
    Secret,
}

/// Field security modifiers.
///
/// Controls how agents and systems can interact with sensitive fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldSecurity {
    /// Sensitivity classification
    pub classification: PIIClassification,
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

impl Default for FieldSecurity {
    fn default() -> Self {
        Self {
            classification: PIIClassification::Public,
            opaque: false,
            immutable: false,
            audited: false,
            redact_in_logs: false,
            env_source: None,
        }
    }
}

impl FieldSecurity {
    /// Create a public field with no restrictions.
    pub fn public() -> Self {
        Self::default()
    }

    /// Create a secret field with full protection.
    pub fn secret() -> Self {
        Self {
            classification: PIIClassification::Secret,
            opaque: true,
            immutable: false,
            audited: true,
            redact_in_logs: true,
            env_source: None,
        }
    }

    /// Create a sensitive field.
    pub fn sensitive() -> Self {
        Self {
            classification: PIIClassification::Confidential,
            opaque: false,
            immutable: false,
            audited: false,
            redact_in_logs: true,
            env_source: None,
        }
    }

    /// Mark as opaque (agent can pass but not read).
    pub fn with_opaque(mut self) -> Self {
        self.opaque = true;
        self
    }

    /// Mark as immutable.
    pub fn with_immutable(mut self) -> Self {
        self.immutable = true;
        self
    }

    /// Mark as audited.
    pub fn with_audited(mut self) -> Self {
        self.audited = true;
        self
    }

    /// Set environment source.
    pub fn with_env(mut self, var_name: impl Into<String>) -> Self {
        self.env_source = Some(var_name.into());
        self
    }
}

/// Enhanced field definition with security modifiers.
///
/// Extends the basic FieldDef with optional security configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecureFieldDef {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Security configuration (optional)
    pub security: Option<FieldSecurity>,
    /// Default value (if any)
    pub default: Option<String>,
}

// ============================================================================
// PARSE ERROR (Task 4.8)
// ============================================================================

/// Parse error with line/column information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Parse error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// Collector for accumulating multiple parse errors.
#[derive(Debug, Clone, Default)]
pub struct ErrorCollector {
    errors: Vec<ParseError>,
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self { errors: vec![] }
    }

    pub fn add(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn into_errors(self) -> Vec<ParseError> {
        self.errors
    }

    /// Convert multiple errors into a single ParseError for backwards compatibility.
    /// Returns the first error with all other errors appended to the message.
    pub fn into_single_error(self) -> Option<ParseError> {
        if self.errors.is_empty() {
            return None;
        }

        if self.errors.len() == 1 {
            return Some(
                self.errors
                    .into_iter()
                    .next()
                    .expect("length verified as 1"),
            );
        }

        // Multiple errors: create combined message
        let first = &self.errors[0];
        let mut message = format!(
            "{} (and {} more errors):\n",
            first.message,
            self.errors.len() - 1
        );

        for (i, err) in self.errors.iter().enumerate() {
            message.push_str(&format!(
                "  {}. Line {}, col {}: {}\n",
                i + 1,
                err.line,
                err.column,
                err.message
            ));
        }

        Some(ParseError {
            message,
            line: first.line,
            column: first.column,
        })
    }
}


