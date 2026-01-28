//! Abstract Syntax Tree types

use crate::lexer::{Token, TokenKind};
use crate::parser::parser::escape_string;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

// ============================================================================
// PARSER (Task 4.2 - 4.7)
// ============================================================================

/// Parser for the CALIBER DSL.
pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
}

impl Parser {
    /// Create a new parser from a vector of tokens.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse the tokens into a CaliberAst.
    pub fn parse(&mut self) -> Result<CaliberAst, ParseError> {
        if let Some(token) = self
            .tokens
            .iter()
            .find(|t| matches!(t.kind, TokenKind::Error(_)))
        {
            let message = match &token.kind {
                TokenKind::Error(msg) => format!("Lexer error: {}", msg),
                _ => "Lexer error".to_string(),
            };
            return Err(ParseError {
                message,
                line: token.span.line,
                column: token.span.column,
            });
        }

        // Expect: caliber: "version" { definitions... }
        self.expect(TokenKind::Caliber)?;
        self.expect(TokenKind::Colon)?;

        let version = match &self.current().kind {
            TokenKind::String(s) => s.clone(),
            _ => return Err(self.error("Expected version string")),
        };
        self.advance();

        self.expect(TokenKind::LBrace)?;

        let mut definitions = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            definitions.push(self.parse_definition()?);
        }

        self.expect(TokenKind::RBrace)?;

        Ok(CaliberAst {
            version,
            definitions,
        })
    }

    /// Parse a single definition.
    fn parse_definition(&mut self) -> Result<Definition, ParseError> {
        match &self.current().kind {
            TokenKind::Adapter => self.parse_adapter().map(Definition::Adapter),
            TokenKind::Memory => self.parse_memory().map(Definition::Memory),
            TokenKind::Policy => self.parse_policy().map(Definition::Policy),
            TokenKind::Inject => self.parse_injection().map(Definition::Injection),
            // Battle Intel Feature 3: Evolution mode
            TokenKind::Evolve => self.parse_evolution().map(Definition::Evolution),
            // Battle Intel Feature 4: Summarization policy
            TokenKind::SummarizationPolicy => {
                self.parse_summarization_policy().map(Definition::SummarizationPolicy)
            }
            // DSL-first architecture: New definitions
            TokenKind::Trajectory => self.parse_trajectory().map(Definition::Trajectory),
            TokenKind::Agent => self.parse_agent().map(Definition::Agent),
            TokenKind::Cache => self.parse_cache().map(Definition::Cache),
            TokenKind::Provider => self.parse_provider().map(Definition::Provider),
            _ => Err(self.error(
                "Expected definition (adapter, memory, policy, inject, evolve, summarization_policy, trajectory, agent, cache, provider)",
            )),
        }
    }

    /// Parse an adapter definition (Task 4.3).
    /// Requires: type, connection (no defaults per REQ-5)
    fn parse_adapter(&mut self) -> Result<AdapterDef, ParseError> {
        self.expect(TokenKind::Adapter)?;

        let name = self.expect_name()?;

        self.expect(TokenKind::LBrace)?;

        let mut adapter_type: Option<AdapterType> = None;
        let mut connection: Option<String> = None;
        let mut options = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            let field = self.expect_field_name()?;
            self.expect(TokenKind::Colon)?;

            match field.as_str() {
                "type" => {
                    adapter_type = Some(match &self.current().kind {
                        TokenKind::Identifier(s) if s == "postgres" => AdapterType::Postgres,
                        TokenKind::Identifier(s) if s == "redis" => AdapterType::Redis,
                        TokenKind::Identifier(s) if s == "memory" => AdapterType::Memory,
                        // Also handle keywords that match adapter types
                        TokenKind::Memory => AdapterType::Memory,
                        _ => {
                            return Err(
                                self.error("Expected adapter type (postgres, redis, memory)")
                            )
                        }
                    });
                    self.advance();
                }
                "connection" => {
                    let conn = self.expect_string()?;
                    if conn.is_empty() {
                        return Err(self.error("connection string cannot be empty"));
                    }
                    connection = Some(conn);
                }
                "options" => {
                    self.expect(TokenKind::LBrace)?;
                    while !self.check(&TokenKind::RBrace) {
                        let key = self.expect_string()?;
                        self.expect(TokenKind::Colon)?;
                        let value = self.expect_string_or_number()?;
                        options.push((key, value));
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                _ => return Err(self.error(&format!("unknown field: {}", field))),
            }
        }

        self.expect(TokenKind::RBrace)?;

        // Validate required fields - no defaults allowed
        let adapter_type =
            adapter_type.ok_or_else(|| self.error("missing required field: type"))?;
        let connection =
            connection.ok_or_else(|| self.error("missing required field: connection"))?;

        Ok(AdapterDef {
            name,
            adapter_type,
            connection,
            options,
        })
    }

    /// Parse a memory definition (Task 4.4).
    /// Requires: type, retention (no defaults per REQ-5)
    fn parse_memory(&mut self) -> Result<MemoryDef, ParseError> {
        self.expect(TokenKind::Memory)?;

        let name = self.expect_name()?;

        self.expect(TokenKind::LBrace)?;

        let mut memory_type: Option<MemoryType> = None;
        let mut schema = Vec::new();
        let mut retention: Option<Retention> = None;
        let mut lifecycle = Lifecycle::Explicit;
        let mut parent = None;
        let mut indexes = Vec::new();
        let mut inject_on = Vec::new();
        let mut artifacts = Vec::new();
        let mut modifiers = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            let field = self.expect_field_name()?;
            self.expect(TokenKind::Colon)?;

            match field.as_str() {
                "type" => {
                    memory_type = Some(self.parse_memory_type()?);
                }
                "schema" => {
                    self.expect(TokenKind::LBrace)?;
                    while !self.check(&TokenKind::RBrace) {
                        schema.push(self.parse_field_def()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                "retention" => {
                    retention = Some(self.parse_retention()?);
                }
                "lifecycle" => {
                    lifecycle = self.parse_lifecycle()?;
                }
                "parent" => {
                    parent = Some(self.expect_name()?);
                }
                "index" => {
                    self.expect(TokenKind::LBrace)?;
                    while !self.check(&TokenKind::RBrace) {
                        indexes.push(self.parse_index_def()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                "inject_on" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(&TokenKind::RBracket) {
                        inject_on.push(self.parse_trigger()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                "artifacts" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(&TokenKind::RBracket) {
                        artifacts.push(self.expect_string()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                "modifiers" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(&TokenKind::RBracket) {
                        modifiers.push(self.parse_modifier()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                _ => return Err(self.error(&format!("unknown field: {}", field))),
            }
        }

        self.expect(TokenKind::RBrace)?;

        // Validate required fields - no defaults allowed
        let memory_type = memory_type.ok_or_else(|| self.error("missing required field: type"))?;
        let retention = retention.ok_or_else(|| self.error("missing required field: retention"))?;

        Ok(MemoryDef {
            name,
            memory_type,
            schema,
            retention,
            lifecycle,
            parent,
            indexes,
            inject_on,
            artifacts,
            modifiers,
        })
    }

    /// Parse a field definition.
    fn parse_field_def(&mut self) -> Result<FieldDef, ParseError> {
        let name = self.expect_field_name()?;
        self.expect(TokenKind::Colon)?;
        let field_type = self.parse_field_type()?;

        // Check for optional nullable marker
        let nullable = if let TokenKind::Identifier(s) = &self.current().kind {
            if s == "optional" {
                self.advance();
                true
            } else {
                false
            }
        } else {
            false
        };

        // Check for security modifiers [modifier, ...]
        let security = if self.check(&TokenKind::LBracket) {
            Some(self.parse_field_security()?)
        } else {
            None
        };

        // Check for default value (literal)
        let default = if self.check(&TokenKind::Eq) {
            self.advance();
            Some(self.parse_default_literal()?)
        } else {
            None
        };

        Ok(FieldDef {
            name,
            field_type,
            nullable,
            default,
            security,
        })
    }

    /// Parse field security modifiers: [modifier, modifier, ...]
    fn parse_field_security(&mut self) -> Result<FieldSecurity, ParseError> {
        self.expect(TokenKind::LBracket)?;
        let mut security = FieldSecurity::default();

        loop {
            match &self.current().kind {
                // Classification levels
                TokenKind::Public => {
                    security.classification = PIIClassification::Public;
                    self.advance();
                }
                TokenKind::Internal => {
                    security.classification = PIIClassification::Internal;
                    self.advance();
                }
                TokenKind::Confidential => {
                    security.classification = PIIClassification::Confidential;
                    self.advance();
                }
                TokenKind::Restricted => {
                    security.classification = PIIClassification::Restricted;
                    self.advance();
                }
                TokenKind::Secret => {
                    security.classification = PIIClassification::Secret;
                    security.redact_in_logs = true; // Secret implies redaction
                    self.advance();
                }
                // Boolean modifiers
                TokenKind::Opaque => {
                    security.opaque = true;
                    self.advance();
                }
                TokenKind::Sensitive => {
                    security.classification = PIIClassification::Confidential;
                    security.redact_in_logs = true;
                    self.advance();
                }
                TokenKind::Redact => {
                    security.redact_in_logs = true;
                    self.advance();
                }
                TokenKind::Immutable => {
                    security.immutable = true;
                    self.advance();
                }
                TokenKind::Audited => {
                    security.audited = true;
                    self.advance();
                }
                // env("VAR_NAME") syntax
                TokenKind::Env => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    if let TokenKind::String(var_name) = &self.current().kind {
                        security.env_source = Some(var_name.clone());
                        self.advance();
                    } else {
                        return Err(self.error("Expected environment variable name string"));
                    }
                    self.expect(TokenKind::RParen)?;
                }
                TokenKind::RBracket => {
                    break;
                }
                TokenKind::Comma => {
                    self.advance();
                }
                _ => {
                    return Err(self.error("Expected security modifier or ']'"));
                }
            }
        }

        self.expect(TokenKind::RBracket)?;
        Ok(security)
    }

    fn parse_default_literal(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let value = format!("\"{}\"", escape_string(s));
                self.advance();
                Ok(value)
            }
            TokenKind::Number(n) => {
                let value = n.to_string();
                self.advance();
                Ok(value)
            }
            TokenKind::Identifier(s) if s == "true" || s == "false" => {
                let value = s.clone();
                self.advance();
                Ok(value)
            }
            _ => Err(self.error("Expected default literal (string, number, or boolean)")),
        }
    }

    /// Parse a field type.
    fn parse_field_type(&mut self) -> Result<FieldType, ParseError> {
        match &self.current().kind {
            TokenKind::Uuid => {
                self.advance();
                Ok(FieldType::Uuid)
            }
            TokenKind::Text => {
                self.advance();
                Ok(FieldType::Text)
            }
            TokenKind::Int => {
                self.advance();
                Ok(FieldType::Int)
            }
            TokenKind::Float => {
                self.advance();
                Ok(FieldType::Float)
            }
            TokenKind::Bool => {
                self.advance();
                Ok(FieldType::Bool)
            }
            TokenKind::Timestamp => {
                self.advance();
                Ok(FieldType::Timestamp)
            }
            TokenKind::Json => {
                self.advance();
                Ok(FieldType::Json)
            }
            TokenKind::Embedding => {
                self.advance();
                // Check for optional dimension
                let dim = if self.check(&TokenKind::LParen) {
                    self.advance();
                    let n = self.expect_number()? as usize;
                    self.expect(TokenKind::RParen)?;
                    Some(n)
                } else {
                    None
                };
                Ok(FieldType::Embedding(dim))
            }
            TokenKind::Enum => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let mut variants = Vec::new();
                while !self.check(&TokenKind::RParen) {
                    variants.push(self.expect_string()?);
                    self.optional_comma();
                }
                self.expect(TokenKind::RParen)?;
                Ok(FieldType::Enum(variants))
            }
            _ => Err(self.error("Expected field type")),
        }
    }

    /// Parse a memory type.
    fn parse_memory_type(&mut self) -> Result<MemoryType, ParseError> {
        match &self.current().kind {
            TokenKind::Ephemeral => {
                self.advance();
                Ok(MemoryType::Ephemeral)
            }
            TokenKind::Working => {
                self.advance();
                Ok(MemoryType::Working)
            }
            TokenKind::Episodic => {
                self.advance();
                Ok(MemoryType::Episodic)
            }
            TokenKind::Semantic => {
                self.advance();
                Ok(MemoryType::Semantic)
            }
            TokenKind::Procedural => {
                self.advance();
                Ok(MemoryType::Procedural)
            }
            TokenKind::Meta => {
                self.advance();
                Ok(MemoryType::Meta)
            }
            _ => Err(self.error("Expected memory type")),
        }
    }

    /// Parse a retention policy.
    fn parse_retention(&mut self) -> Result<Retention, ParseError> {
        match &self.current().kind {
            TokenKind::Persistent => {
                self.advance();
                Ok(Retention::Persistent)
            }
            TokenKind::Session => {
                self.advance();
                Ok(Retention::Session)
            }
            TokenKind::Scope => {
                self.advance();
                Ok(Retention::Scope)
            }
            TokenKind::Duration(d) => {
                let d = d.clone();
                self.advance();
                Ok(Retention::Duration(d))
            }
            TokenKind::Number(n) => {
                let n = *n as usize;
                self.advance();
                Ok(Retention::Max(n))
            }
            _ => Err(self.error("Expected retention type")),
        }
    }
}
