//! Lexer token types


// ============================================================================
// LEXER TYPES (Task 3.1, 3.2)
// ============================================================================

/// Token kinds for the CALIBER DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Caliber,
    Memory,
    Policy,
    Adapter,
    Inject,
    Into,
    On,
    Context,
    Type,
    Schema,
    Retention,
    Index,
    Lifecycle,
    Artifacts,
    Parent,
    InjectOn,
    Connection,
    Options,
    Mode,
    Priority,
    MaxTokens,
    Filter,
    Schedule,

    // Memory types
    Ephemeral,
    Working,
    Episodic,
    Semantic,
    Procedural,
    Meta,

    // Field types
    Uuid,
    Text,
    Int,
    Float,
    Bool,
    Timestamp,
    Json,
    Embedding,
    Enum,

    // Retention types
    Persistent,
    Session,
    Scope,

    // Injection modes
    Full,
    Summary,
    TopK,
    Relevant,

    // Lifecycle triggers
    TaskStart,
    TaskEnd,
    ScopeClose,
    TurnEnd,
    Manual,
    Explicit,

    // Policy actions
    Summarize,
    ExtractArtifacts,
    Checkpoint,
    Prune,
    Notify,
    AutoSummarize,  // Battle Intel Feature 4

    // Battle Intel Feature 2: Abstraction levels
    AbstractionLevel,
    Raw,
    Principle,  // Note: Summary already exists above

    // Battle Intel Feature 3: Evolution mode keywords
    Freeze,
    Snapshot,
    Benchmark,
    Evolve,
    Compare,
    Baseline,
    Candidates,
    Metrics,

    // Battle Intel Feature 4: Summarization triggers and policy
    DosageReached,
    CreateEdges,
    SourceLevel,
    TargetLevel,
    MaxSources,
    TurnCount,             // turn_count(N) trigger
    ArtifactCount,         // artifact_count(N) trigger
    SummarizationPolicy,   // top-level definition keyword
    Triggers,              // triggers: field
    BenchmarkQueries,      // benchmark_queries: field

    // DSL-first architecture: New top-level definitions
    Trajectory,            // trajectory definition
    Agent,                 // agent definition
    Cache,                 // cache configuration
    Provider,              // LLM provider definition

    // Agent definition keywords
    Capabilities,          // agent capabilities list
    Constraints,           // agent constraints block
    Permissions,           // agent permissions block
    MaxConcurrent,         // constraint: max concurrent tasks
    TimeoutMs,             // constraint: timeout in milliseconds
    Read,                  // permission: read access
    Write,                 // permission: write access
    Lock,                  // permission: lock access (also memory modifier)

    // Cache configuration keywords
    Backend,               // cache backend type
    Lmdb,                  // LMDB backend
    MaxStaleness,          // cache max staleness duration
    PollInterval,          // cache poll interval
    Prefetch,              // cache prefetch setting
    MaxEntries,            // cache max entries
    Ttl,                   // cache TTL
    SizeMb,                // cache size in MB
    DefaultFreshness,      // cache default freshness
    BestEffort,            // freshness mode
    Strict,                // freshness mode

    // Modifier keywords
    Modifiers,             // memory modifiers list
    Embeddable,            // embeddable modifier
    Summarizable,          // summarizable modifier
    Lockable,              // lockable modifier
    Style,                 // summarization style
    Brief,                 // summary style: brief
    Detailed,              // summary style: detailed

    // Lock mode keywords
    Exclusive,             // exclusive lock mode
    Shared,                // shared lock mode

    // Provider keywords
    ApiKey,                // provider API key
    Model,                 // provider model name
    Openai,                // OpenAI provider type
    Anthropic,             // Anthropic provider type

    // Utility keywords
    Env,                   // environment variable reference
    Description,           // trajectory/agent description
    AgentType,             // trajectory agent type reference
    TokenBudget,           // trajectory token budget
    MemoryRefs,            // trajectory memory references

    // Index types
    Btree,
    Hash,
    Gin,
    Hnsw,
    Ivfflat,

    // Operators
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    Contains,
    Regex,
    And,
    Or,
    Not,
    In,

    // Delimiters
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Colon,
    Comma,
    Dot,
    Arrow,

    // Literals
    String(String),
    Number(f64),
    Duration(String),
    Identifier(String),

    // Special
    Eof,
    Error(String),

    // ========================================================================
    // PII & SECURITY TOKENS (Phase 3)
    // ========================================================================

    /// Mark field/value as opaque to agents (can pass but not read)
    Opaque,
    /// Sensitive data classification
    Sensitive,
    /// Secret value (encrypted at rest, redacted in logs)
    Secret,
    /// Redaction policy marker
    Redact,
    /// Write protection (immutable after creation)
    Immutable,
    /// Audit requirement (all access logged)
    Audited,

    // Sensitivity/classification levels
    /// Public - no restrictions
    Public,
    /// Internal use only
    Internal,
    /// Limited access - confidential
    Confidential,
    /// Highly restricted
    Restricted,
}

/// Source location span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        }
    }
}

/// A token with its kind and source location.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

