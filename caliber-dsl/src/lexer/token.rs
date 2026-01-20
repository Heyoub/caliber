//! Lexer token types

use std::fmt;

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

