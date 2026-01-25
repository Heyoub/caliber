//! Lexer implementation

use super::token::*;
use std::iter::Peekable;
use std::str::CharIndices;

// ============================================================================
// LEXER IMPLEMENTATION (Task 3.3, 3.4)
// ============================================================================

/// Lexer for the CALIBER DSL.
pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    line: usize,
    column: usize,
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given source.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
            pos: 0,
        }
    }

    /// Tokenize the entire source into a vector of tokens.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        tokens
    }

    /// Get the next token from the source.
    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let start_pos = self.pos;
        let start_line = self.line;
        let start_col = self.column;

        let kind = match self.peek_char() {
            None => TokenKind::Eof,
            Some(c) => match c {
                '{' => {
                    self.advance();
                    TokenKind::LBrace
                }
                '}' => {
                    self.advance();
                    TokenKind::RBrace
                }
                '(' => {
                    self.advance();
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    TokenKind::RParen
                }
                '[' => {
                    self.advance();
                    TokenKind::LBracket
                }
                ']' => {
                    self.advance();
                    TokenKind::RBracket
                }
                ':' => {
                    self.advance();
                    TokenKind::Colon
                }
                ',' => {
                    self.advance();
                    TokenKind::Comma
                }
                '.' => {
                    self.advance();
                    TokenKind::Dot
                }

                '=' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                    }
                    TokenKind::Eq
                }

                '!' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        TokenKind::Ne
                    } else {
                        TokenKind::Not
                    }
                }

                '>' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        TokenKind::Ge
                    } else {
                        TokenKind::Gt
                    }
                }

                '<' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        TokenKind::Le
                    } else {
                        TokenKind::Lt
                    }
                }

                '~' => {
                    self.advance();
                    TokenKind::Regex
                }

                '-' => {
                    self.advance();
                    if self.peek_char() == Some('>') {
                        self.advance();
                        TokenKind::Arrow
                    } else if self.peek_char().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                        self.scan_number_from_pos(start_pos)
                    } else {
                        TokenKind::Error("Unexpected character: -".to_string())
                    }
                }

                '"' => self.scan_string(),

                c if c.is_ascii_digit() => self.scan_number_or_duration(),

                c if c.is_ascii_alphabetic() || c == '_' => self.scan_identifier(),

                c => {
                    self.advance();
                    TokenKind::Error(format!("Unexpected character: {}", c))
                }
            },
        };

        Token {
            kind,
            span: Span {
                start: start_pos,
                end: self.pos,
                line: start_line,
                column: start_col,
            },
        }
    }

    /// Scan an identifier or keyword.
    fn scan_identifier(&mut self) -> TokenKind {
        let start = self.pos;

        while let Some(c) = self.peek_char() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let ident = &self.source[start..self.pos];

        // Check keywords (case-insensitive)
        match ident.to_lowercase().as_str() {
            "caliber" => TokenKind::Caliber,
            "memory" => TokenKind::Memory,
            "policy" => TokenKind::Policy,
            "adapter" => TokenKind::Adapter,
            "inject" => TokenKind::Inject,
            "into" => TokenKind::Into,
            "on" => TokenKind::On,
            "context" => TokenKind::Context,
            "type" => TokenKind::Type,
            "schema" => TokenKind::Schema,
            "retention" => TokenKind::Retention,
            "index" => TokenKind::Index,
            "lifecycle" => TokenKind::Lifecycle,
            "artifacts" => TokenKind::Artifacts,
            "parent" => TokenKind::Parent,
            "inject_on" => TokenKind::InjectOn,
            "connection" => TokenKind::Connection,
            "options" => TokenKind::Options,
            "mode" => TokenKind::Mode,
            "priority" => TokenKind::Priority,
            "max_tokens" => TokenKind::MaxTokens,
            "filter" => TokenKind::Filter,
            "schedule" => TokenKind::Schedule,

            // Memory types
            "ephemeral" => TokenKind::Ephemeral,
            "working" => TokenKind::Working,
            "episodic" => TokenKind::Episodic,
            "semantic" => TokenKind::Semantic,
            "procedural" => TokenKind::Procedural,
            "meta" => TokenKind::Meta,

            // Field types
            "uuid" => TokenKind::Uuid,
            "text" => TokenKind::Text,
            "int" => TokenKind::Int,
            "float" => TokenKind::Float,
            "bool" => TokenKind::Bool,
            "timestamp" => TokenKind::Timestamp,
            "json" => TokenKind::Json,
            "embedding" => TokenKind::Embedding,
            "enum" => TokenKind::Enum,

            // Retention
            "persistent" => TokenKind::Persistent,
            "session" => TokenKind::Session,
            "scope" => TokenKind::Scope,

            // Injection modes
            "full" => TokenKind::Full,
            "summary" => TokenKind::Summary,
            "top_k" => TokenKind::TopK,
            "relevant" => TokenKind::Relevant,

            // Lifecycle
            "task_start" => TokenKind::TaskStart,
            "task_end" => TokenKind::TaskEnd,
            "scope_close" => TokenKind::ScopeClose,
            "turn_end" => TokenKind::TurnEnd,
            "manual" => TokenKind::Manual,
            "explicit" => TokenKind::Explicit,

            // Actions
            "summarize" => TokenKind::Summarize,
            "extract_artifacts" => TokenKind::ExtractArtifacts,
            "checkpoint" => TokenKind::Checkpoint,
            "prune" => TokenKind::Prune,
            "notify" => TokenKind::Notify,
            "auto_summarize" => TokenKind::AutoSummarize,

            // Battle Intel Feature 2: Abstraction levels
            "abstraction_level" => TokenKind::AbstractionLevel,
            "raw" => TokenKind::Raw,
            "principle" => TokenKind::Principle,

            // Battle Intel Feature 3: Evolution mode
            "freeze" => TokenKind::Freeze,
            "snapshot" => TokenKind::Snapshot,
            "benchmark" => TokenKind::Benchmark,
            "evolve" => TokenKind::Evolve,
            "compare" => TokenKind::Compare,
            "baseline" => TokenKind::Baseline,
            "candidates" => TokenKind::Candidates,
            "metrics" => TokenKind::Metrics,

            // Battle Intel Feature 4: Summarization triggers and policy
            "dosage_reached" => TokenKind::DosageReached,
            "create_edges" => TokenKind::CreateEdges,
            "source_level" => TokenKind::SourceLevel,
            "target_level" => TokenKind::TargetLevel,
            "max_sources" => TokenKind::MaxSources,
            "turn_count" => TokenKind::TurnCount,
            "artifact_count" => TokenKind::ArtifactCount,
            "summarization_policy" => TokenKind::SummarizationPolicy,
            "triggers" => TokenKind::Triggers,
            "benchmark_queries" => TokenKind::BenchmarkQueries,

            // DSL-first architecture: New top-level definitions
            "trajectory" => TokenKind::Trajectory,
            "agent" => TokenKind::Agent,
            "cache" => TokenKind::Cache,
            "provider" => TokenKind::Provider,

            // Agent definition keywords
            "capabilities" => TokenKind::Capabilities,
            "constraints" => TokenKind::Constraints,
            "permissions" => TokenKind::Permissions,
            "max_concurrent" => TokenKind::MaxConcurrent,
            "timeout_ms" => TokenKind::TimeoutMs,
            "read" => TokenKind::Read,
            "write" => TokenKind::Write,
            "lock" => TokenKind::Lock,

            // Cache configuration keywords
            "backend" => TokenKind::Backend,
            "lmdb" => TokenKind::Lmdb,
            "max_staleness" => TokenKind::MaxStaleness,
            "poll_interval" => TokenKind::PollInterval,
            "prefetch" => TokenKind::Prefetch,
            "max_entries" => TokenKind::MaxEntries,
            "ttl" => TokenKind::Ttl,
            "size_mb" => TokenKind::SizeMb,
            "default_freshness" => TokenKind::DefaultFreshness,
            "best_effort" => TokenKind::BestEffort,
            "strict" => TokenKind::Strict,

            // Modifier keywords
            "modifiers" => TokenKind::Modifiers,
            "embeddable" => TokenKind::Embeddable,
            "summarizable" => TokenKind::Summarizable,
            "lockable" => TokenKind::Lockable,
            "style" => TokenKind::Style,
            "brief" => TokenKind::Brief,
            "detailed" => TokenKind::Detailed,

            // PII & Security keywords
            "opaque" => TokenKind::Opaque,
            "sensitive" => TokenKind::Sensitive,
            "secret" => TokenKind::Secret,
            "redact" => TokenKind::Redact,
            "immutable" => TokenKind::Immutable,
            "audited" => TokenKind::Audited,
            "public" => TokenKind::Public,
            "internal" => TokenKind::Internal,
            "confidential" => TokenKind::Confidential,
            "restricted" => TokenKind::Restricted,

            // Lock mode keywords
            "exclusive" => TokenKind::Exclusive,
            "shared" => TokenKind::Shared,

            // Provider keywords
            "api_key" => TokenKind::ApiKey,
            "model" => TokenKind::Model,
            "openai" => TokenKind::Openai,
            "anthropic" => TokenKind::Anthropic,

            // Utility keywords
            "env" => TokenKind::Env,
            "description" => TokenKind::Description,
            "agent_type" => TokenKind::AgentType,
            "token_budget" => TokenKind::TokenBudget,
            "memory_refs" => TokenKind::MemoryRefs,

            // Index types
            "btree" => TokenKind::Btree,
            "hash" => TokenKind::Hash,
            "gin" => TokenKind::Gin,
            "hnsw" => TokenKind::Hnsw,
            "ivfflat" => TokenKind::Ivfflat,

            // Operators
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "in" => TokenKind::In,
            "contains" => TokenKind::Contains,

            _ => TokenKind::Identifier(ident.to_string()),
        }
    }

    /// Scan a string literal with escape sequences.
    fn scan_string(&mut self) -> TokenKind {
        self.advance(); // consume opening quote
        let mut value = String::new();

        loop {
            match self.peek_char() {
                None => return TokenKind::Error("Unterminated string".to_string()),
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.peek_char() {
                        Some('n') => {
                            self.advance();
                            value.push('\n');
                        }
                        Some('t') => {
                            self.advance();
                            value.push('\t');
                        }
                        Some('\\') => {
                            self.advance();
                            value.push('\\');
                        }
                        Some('"') => {
                            self.advance();
                            value.push('"');
                        }
                        Some('r') => {
                            self.advance();
                            value.push('\r');
                        }
                        _ => value.push('\\'),
                    }
                }
                Some(c) => {
                    self.advance();
                    value.push(c);
                }
            }
        }

        TokenKind::String(value)
    }

    /// Scan a number or duration literal.
    fn scan_number_or_duration(&mut self) -> TokenKind {
        let start = self.pos;

        // Scan digits and optional decimal point
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '.' {
                self.advance();
            } else {
                break;
            }
        }

        // Check for duration suffix
        if let Some(c) = self.peek_char() {
            if matches!(c, 's' | 'm' | 'h' | 'd' | 'w') {
                self.advance();
                let text = &self.source[start..self.pos];
                return TokenKind::Duration(text.to_string());
            }
        }

        let text = &self.source[start..self.pos];
        match text.parse::<f64>() {
            Ok(n) => TokenKind::Number(n),
            Err(_) => TokenKind::Error(format!("Invalid number: {}", text)),
        }
    }

    /// Scan a number starting from a given position (for negative numbers).
    fn scan_number_from_pos(&mut self, start: usize) -> TokenKind {
        // Scan digits and optional decimal point
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '.' {
                self.advance();
            } else {
                break;
            }
        }

        // Check for duration suffix
        if let Some(c) = self.peek_char() {
            if matches!(c, 's' | 'm' | 'h' | 'd' | 'w') {
                self.advance();
                let text = &self.source[start..self.pos];
                return TokenKind::Duration(text.to_string());
            }
        }

        let text = &self.source[start..self.pos];
        match text.parse::<f64>() {
            Ok(n) => TokenKind::Number(n),
            Err(_) => TokenKind::Error(format!("Invalid number: {}", text)),
        }
    }

    /// Skip whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_char() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('\n') => {
                    self.advance();
                    self.line += 1;
                    self.column = 1;
                }
                Some('/') => {
                    let next = self.peek_next_char();
                    if next == Some('/') {
                        // Line comment
                        while let Some(c) = self.peek_char() {
                            if c == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    } else if next == Some('*') {
                        // Block comment
                        self.advance(); // /
                        self.advance(); // *
                        loop {
                            match self.peek_char() {
                                None => break,
                                Some('*') if self.peek_next_char() == Some('/') => {
                                    self.advance();
                                    self.advance();
                                    break;
                                }
                                Some('\n') => {
                                    self.advance();
                                    self.line += 1;
                                    self.column = 1;
                                }
                                _ => {
                                    self.advance();
                                }
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut iter = self.source[self.pos..].char_indices();
        iter.next();
        iter.next().map(|(_, c)| c)
    }

    fn advance(&mut self) -> Option<char> {
        if let Some((i, c)) = self.chars.next() {
            self.pos = i + c.len_utf8();
            self.column += 1;
            Some(c)
        } else {
            None
        }
    }
}

