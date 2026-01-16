# CALIBER DSL Parser & Code Generator

**Crate:** `caliber-dsl/` (standalone, depends only on `caliber-core`)

## Architecture

```
DSL Source (.caliber file)
    ↓
Lexer (tokenize)
    ↓
Parser (build AST)
    ↓
Validator (check semantics)
    ↓
Code Generator (emit CaliberConfig)
    ↓
CaliberConfig struct (passed to caliber-pg at runtime)
```

**Critical**: The DSL does NOT generate SQL. It generates a `CaliberConfig` struct that is consumed by the runtime.

---

## 1. LEXER (Rust)

```rust
use std::str::CharIndices;
use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Caliber, Memory, Policy, Adapter, Inject, Into, On, Context,
    Type, Schema, Retention, Index, Lifecycle, Artifacts, Parent,
    InjectOn, Connection, Options, Mode, Priority, MaxTokens, Filter,
    
    // Memory types
    Ephemeral, Working, Episodic, Semantic, Procedural, Meta,
    
    // Field types
    Uuid, Text, Int, Float, Bool, Timestamp, Json, Embedding, Enum,
    
    // Retention types
    Persistent, Session, Scope,
    
    // Injection modes
    Full, Summary, TopK, Relevant,
    
    // Lifecycle triggers
    TaskStart, TaskEnd, ScopeClose, TurnEnd, Manual, Explicit,
    
    // Policy actions
    Summarize, ExtractArtifacts, Checkpoint, Prune, Notify,
    
    // Index types
    Btree, Hash, Gin, Hnsw, Ivfflat,
    
    // Operators
    Eq, Ne, Gt, Lt, Ge, Le, Contains, Regex, And, Or, Not, In,
    
    // Delimiters
    LBrace, RBrace, LParen, RParen, LBracket, RBracket,
    Colon, Comma, Dot, Arrow,
    
    // Literals
    String(String),
    Number(f64),
    Duration(String),
    Identifier(String),
    
    // Special
    Eof,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    line: usize,
    column: usize,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
            pos: 0,
        }
    }
    
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof { break; }
        }
        
        tokens
    }
    
    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();
        
        let start_pos = self.pos;
        let start_line = self.line;
        let start_col = self.column;
        
        let kind = match self.peek_char() {
            None => TokenKind::Eof,
            Some(c) => match c {
                '{' => { self.advance(); TokenKind::LBrace }
                '}' => { self.advance(); TokenKind::RBrace }
                '(' => { self.advance(); TokenKind::LParen }
                ')' => { self.advance(); TokenKind::RParen }
                '[' => { self.advance(); TokenKind::LBracket }
                ']' => { self.advance(); TokenKind::RBracket }
                ':' => { self.advance(); TokenKind::Colon }
                ',' => { self.advance(); TokenKind::Comma }
                '.' => { self.advance(); TokenKind::Dot }
                
                '=' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        TokenKind::Eq
                    } else {
                        TokenKind::Eq
                    }
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
                
                '~' => { self.advance(); TokenKind::Regex }
                
                '-' => {
                    self.advance();
                    if self.peek_char() == Some('>') {
                        self.advance();
                        TokenKind::Arrow
                    } else {
                        self.scan_number_or_duration('-')
                    }
                }
                
                '"' => self.scan_string(),
                
                c if c.is_ascii_digit() => self.scan_number_or_duration(c),
                
                c if c.is_alphabetic() || c == '_' => self.scan_identifier(),
                
                c => {
                    self.advance();
                    TokenKind::Error(format!("Unexpected character: {}", c))
                }
            }
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
    
    fn scan_identifier(&mut self) -> TokenKind {
        let start = self.pos;
        
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        
        let ident = &self.source[start..self.pos];
        
        // Check keywords
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
    
    fn scan_string(&mut self) -> TokenKind {
        self.advance(); // consume opening quote
        let start = self.pos;
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
                        Some('n') => { self.advance(); value.push('\n'); }
                        Some('t') => { self.advance(); value.push('\t'); }
                        Some('\\') => { self.advance(); value.push('\\'); }
                        Some('"') => { self.advance(); value.push('"'); }
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
    
    fn scan_number_or_duration(&mut self, first: char) -> TokenKind {
        let start = self.pos;
        if first != '-' {
            // Don't re-advance if we already have the first char
        }
        
        // Scan digits
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
                    // Check for comment
                    let next = self.peek_next_char();
                    if next == Some('/') {
                        // Line comment
                        while let Some(c) = self.peek_char() {
                            if c == '\n' { break; }
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
                                _ => { self.advance(); }
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
```

---

## 2. AST (Rust)

```rust
#[derive(Debug, Clone)]
pub struct CaliberConfig {
    pub version: String,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, Clone)]
pub enum Definition {
    Adapter(AdapterDef),
    Memory(MemoryDef),
    Policy(PolicyDef),
    Injection(InjectionDef),
}

#[derive(Debug, Clone)]
pub struct AdapterDef {
    pub name: String,
    pub adapter_type: AdapterType,
    pub connection: String,
    pub options: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy)]
pub enum AdapterType {
    Postgres,
    Redis,
    Memory,
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryType {
    Ephemeral,
    Working,
    Episodic,
    Semantic,
    Procedural,
    Meta,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub nullable: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    Uuid,
    Text,
    Int,
    Float,
    Bool,
    Timestamp,
    Json,
    Embedding(Option<usize>),  // Optional dimension
    Enum(Vec<String>),
    Array(Box<FieldType>),
}

#[derive(Debug, Clone)]
pub enum Retention {
    Persistent,
    Session,
    Scope,
    Duration(String),
    Max(usize),
}

#[derive(Debug, Clone)]
pub enum Lifecycle {
    Explicit,
    AutoClose(Trigger),
}

#[derive(Debug, Clone, Copy)]
pub enum Trigger {
    TaskStart,
    TaskEnd,
    ScopeClose,
    TurnEnd,
    Manual,
}

#[derive(Debug, Clone)]
pub struct IndexDef {
    pub field: String,
    pub index_type: IndexType,
    pub options: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy)]
pub enum IndexType {
    Btree,
    Hash,
    Gin,
    Hnsw,
    Ivfflat,
}

#[derive(Debug, Clone)]
pub struct PolicyDef {
    pub name: String,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone)]
pub struct PolicyRule {
    pub trigger: Trigger,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone)]
pub enum Action {
    Summarize(String),
    ExtractArtifacts(String),
    Checkpoint(String),
    Prune { target: String, criteria: FilterExpr },
    Notify(String),
    Inject { target: String, mode: InjectionMode },
}

#[derive(Debug, Clone)]
pub struct InjectionDef {
    pub source: String,
    pub target: String,
    pub mode: InjectionMode,
    pub priority: i32,
    pub max_tokens: Option<i32>,
    pub filter: Option<FilterExpr>,
}

#[derive(Debug, Clone)]
pub enum InjectionMode {
    Full,
    Summary,
    TopK(usize),
    Relevant(f32),
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy)]
pub enum CompareOp {
    Eq, Ne, Gt, Lt, Ge, Le, Contains, Regex, In,
}

#[derive(Debug, Clone)]
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
```

---

## 3. PARSER (Rust)

```rust
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }
    
    pub fn parse(&mut self) -> Result<CaliberConfig, ParseError> {
        // Expect: caliber: "version" { definitions... }
        self.expect(TokenKind::Caliber)?;
        self.expect(TokenKind::Colon)?;
        
        let version = match self.current().kind {
            TokenKind::String(ref s) => s.clone(),
            _ => return Err(self.error("Expected version string")),
        };
        self.advance();
        
        self.expect(TokenKind::LBrace)?;
        
        let mut definitions = Vec::new();
        
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            definitions.push(self.parse_definition()?);
        }
        
        self.expect(TokenKind::RBrace)?;
        
        Ok(CaliberConfig { version, definitions })
    }
    
    fn parse_definition(&mut self) -> Result<Definition, ParseError> {
        match self.current().kind {
            TokenKind::Adapter => self.parse_adapter().map(Definition::Adapter),
            TokenKind::Memory => self.parse_memory().map(Definition::Memory),
            TokenKind::Policy => self.parse_policy().map(Definition::Policy),
            TokenKind::Inject => self.parse_injection().map(Definition::Injection),
            _ => Err(self.error("Expected definition (adapter, memory, policy, inject)")),
        }
    }
    
    fn parse_adapter(&mut self) -> Result<AdapterDef, ParseError> {
        self.expect(TokenKind::Adapter)?;
        
        let name = self.expect_identifier()?;
        
        self.expect(TokenKind::LBrace)?;
        
        let mut adapter_type = AdapterType::Postgres;
        let mut connection = String::new();
        let mut options = Vec::new();
        
        while !self.check(TokenKind::RBrace) {
            let field = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            
            match field.as_str() {
                "type" => {
                    adapter_type = match self.current().kind {
                        TokenKind::Identifier(ref s) if s == "postgres" => AdapterType::Postgres,
                        TokenKind::Identifier(ref s) if s == "redis" => AdapterType::Redis,
                        TokenKind::Identifier(ref s) if s == "memory" => AdapterType::Memory,
                        _ => return Err(self.error("Expected adapter type")),
                    };
                    self.advance();
                }
                "connection" => {
                    connection = self.expect_string()?;
                }
                "options" => {
                    self.expect(TokenKind::LBrace)?;
                    while !self.check(TokenKind::RBrace) {
                        let key = self.expect_string()?;
                        self.expect(TokenKind::Colon)?;
                        let value = self.expect_string_or_number()?;
                        options.push((key, value));
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                _ => return Err(self.error(&format!("Unknown adapter field: {}", field))),
            }
        }
        
        self.expect(TokenKind::RBrace)?;
        
        Ok(AdapterDef { name, adapter_type, connection, options })
    }
    
    fn parse_memory(&mut self) -> Result<MemoryDef, ParseError> {
        self.expect(TokenKind::Memory)?;
        
        let name = self.expect_identifier()?;
        
        self.expect(TokenKind::LBrace)?;
        
        let mut memory_type = MemoryType::Working;
        let mut schema = Vec::new();
        let mut retention = Retention::Persistent;
        let mut lifecycle = Lifecycle::Explicit;
        let mut parent = None;
        let mut indexes = Vec::new();
        let mut inject_on = Vec::new();
        let mut artifacts = Vec::new();
        
        while !self.check(TokenKind::RBrace) {
            let field = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            
            match field.as_str() {
                "type" => {
                    memory_type = self.parse_memory_type()?;
                }
                "schema" => {
                    self.expect(TokenKind::LBrace)?;
                    while !self.check(TokenKind::RBrace) {
                        schema.push(self.parse_field_def()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                "retention" => {
                    retention = self.parse_retention()?;
                }
                "lifecycle" => {
                    lifecycle = self.parse_lifecycle()?;
                }
                "parent" => {
                    parent = Some(self.expect_identifier()?);
                }
                "index" => {
                    self.expect(TokenKind::LBrace)?;
                    while !self.check(TokenKind::RBrace) {
                        indexes.push(self.parse_index_def()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                "inject_on" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(TokenKind::RBracket) {
                        inject_on.push(self.parse_trigger()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                "artifacts" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(TokenKind::RBracket) {
                        artifacts.push(self.expect_string()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                _ => return Err(self.error(&format!("Unknown memory field: {}", field))),
            }
        }
        
        self.expect(TokenKind::RBrace)?;
        
        Ok(MemoryDef {
            name, memory_type, schema, retention, lifecycle,
            parent, indexes, inject_on, artifacts,
        })
    }
    
    fn parse_field_def(&mut self) -> Result<FieldDef, ParseError> {
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Colon)?;
        let field_type = self.parse_field_type()?;
        
        // Optional nullable marker + default literal (after '=')
        let nullable = self.parse_optional_flag()?;
        let default_value = self.parse_default_literal_if_present()?;
        
        Ok(FieldDef {
            name,
            field_type,
            nullable,              // parsed from optional keyword
            default: default_value // parsed after '=' literal
        })
    }
    
    fn parse_field_type(&mut self) -> Result<FieldType, ParseError> {
        match self.current().kind {
            TokenKind::Uuid => { self.advance(); Ok(FieldType::Uuid) }
            TokenKind::Text => { self.advance(); Ok(FieldType::Text) }
            TokenKind::Int => { self.advance(); Ok(FieldType::Int) }
            TokenKind::Float => { self.advance(); Ok(FieldType::Float) }
            TokenKind::Bool => { self.advance(); Ok(FieldType::Bool) }
            TokenKind::Timestamp => { self.advance(); Ok(FieldType::Timestamp) }
            TokenKind::Json => { self.advance(); Ok(FieldType::Json) }
            TokenKind::Embedding => {
                self.advance();
                // Check for optional dimension
                let dim = if self.check(TokenKind::LParen) {
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
                while !self.check(TokenKind::RParen) {
                    variants.push(self.expect_string()?);
                    self.optional_comma();
                }
                self.expect(TokenKind::RParen)?;
                Ok(FieldType::Enum(variants))
            }
            _ => Err(self.error("Expected field type")),
        }
    }
    
    fn parse_memory_type(&mut self) -> Result<MemoryType, ParseError> {
        match self.current().kind {
            TokenKind::Ephemeral => { self.advance(); Ok(MemoryType::Ephemeral) }
            TokenKind::Working => { self.advance(); Ok(MemoryType::Working) }
            TokenKind::Episodic => { self.advance(); Ok(MemoryType::Episodic) }
            TokenKind::Semantic => { self.advance(); Ok(MemoryType::Semantic) }
            TokenKind::Procedural => { self.advance(); Ok(MemoryType::Procedural) }
            TokenKind::Meta => { self.advance(); Ok(MemoryType::Meta) }
            _ => Err(self.error("Expected memory type")),
        }
    }
    
    fn parse_retention(&mut self) -> Result<Retention, ParseError> {
        match self.current().kind {
            TokenKind::Persistent => { self.advance(); Ok(Retention::Persistent) }
            TokenKind::Session => { self.advance(); Ok(Retention::Session) }
            TokenKind::Scope => { self.advance(); Ok(Retention::Scope) }
            TokenKind::Duration(ref d) => {
                let d = d.clone();
                self.advance();
                Ok(Retention::Duration(d))
            }
            TokenKind::Number(n) => {
                let n = n as usize;
                self.advance();
                Ok(Retention::Max(n))
            }
            _ => Err(self.error("Expected retention type")),
        }
    }
    
    fn parse_lifecycle(&mut self) -> Result<Lifecycle, ParseError> {
        match self.current().kind {
            TokenKind::Explicit => { self.advance(); Ok(Lifecycle::Explicit) }
            TokenKind::Identifier(ref s) if s == "auto_close" => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let trigger = self.parse_trigger()?;
                self.expect(TokenKind::RParen)?;
                Ok(Lifecycle::AutoClose(trigger))
            }
            _ => Err(self.error("Expected lifecycle")),
        }
    }
    
    fn parse_trigger(&mut self) -> Result<Trigger, ParseError> {
        match self.current().kind {
            TokenKind::TaskStart => { self.advance(); Ok(Trigger::TaskStart) }
            TokenKind::TaskEnd => { self.advance(); Ok(Trigger::TaskEnd) }
            TokenKind::ScopeClose => { self.advance(); Ok(Trigger::ScopeClose) }
            TokenKind::TurnEnd => { self.advance(); Ok(Trigger::TurnEnd) }
            TokenKind::Manual => { self.advance(); Ok(Trigger::Manual) }
            _ => Err(self.error("Expected trigger")),
        }
    }
    
    fn parse_index_def(&mut self) -> Result<IndexDef, ParseError> {
        let field = self.expect_identifier()?;
        self.expect(TokenKind::Colon)?;
        let index_type = self.parse_index_type()?;
        
        Ok(IndexDef {
            field,
            index_type,
            options, // parsed from options: { ... }
        })
    }
    
    fn parse_index_type(&mut self) -> Result<IndexType, ParseError> {
        match self.current().kind {
            TokenKind::Btree => { self.advance(); Ok(IndexType::Btree) }
            TokenKind::Hash => { self.advance(); Ok(IndexType::Hash) }
            TokenKind::Gin => { self.advance(); Ok(IndexType::Gin) }
            TokenKind::Hnsw => { self.advance(); Ok(IndexType::Hnsw) }
            TokenKind::Ivfflat => { self.advance(); Ok(IndexType::Ivfflat) }
            _ => Err(self.error("Expected index type")),
        }
    }
    
    fn parse_policy(&mut self) -> Result<PolicyDef, ParseError> {
        self.expect(TokenKind::Policy)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LBrace)?;
        
        let mut rules = Vec::new();
        
        while !self.check(TokenKind::RBrace) {
            if self.check(TokenKind::On) {
                self.advance();
                let trigger = self.parse_trigger()?;
                self.expect(TokenKind::Colon)?;
                self.expect(TokenKind::LBracket)?;
                
                let mut actions = Vec::new();
                while !self.check(TokenKind::RBracket) {
                    actions.push(self.parse_action()?);
                    self.optional_comma();
                }
                self.expect(TokenKind::RBracket)?;
                
                rules.push(PolicyRule { trigger, actions });
            } else {
                return Err(self.error("Expected 'on' trigger"));
            }
        }
        
        self.expect(TokenKind::RBrace)?;
        
        Ok(PolicyDef { name, rules })
    }
    
    fn parse_action(&mut self) -> Result<Action, ParseError> {
        match self.current().kind {
            TokenKind::Summarize => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let target = self.expect_identifier()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::Summarize(target))
            }
            TokenKind::ExtractArtifacts => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let target = self.expect_identifier()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::ExtractArtifacts(target))
            }
            TokenKind::Checkpoint => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let target = self.expect_identifier()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::Checkpoint(target))
            }
            TokenKind::Notify => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let channel = self.expect_string()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::Notify(channel))
            }
            _ => Err(self.error("Expected action")),
        }
    }
    
    fn parse_injection(&mut self) -> Result<InjectionDef, ParseError> {
        self.expect(TokenKind::Inject)?;
        let source = self.expect_identifier()?;
        self.expect(TokenKind::Into)?;
        let target = self.expect_identifier()?;
        self.expect(TokenKind::LBrace)?;
        
        let mut mode = InjectionMode::Full;
        let mut priority = 50;
        let mut max_tokens = None;
        let mut filter = None;
        
        while !self.check(TokenKind::RBrace) {
            let field = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            
            match field.as_str() {
                "mode" => mode = self.parse_injection_mode()?,
                "priority" => priority = self.expect_number()? as i32,
                "max_tokens" => max_tokens = Some(self.expect_number()? as i32),
                "filter" => filter = Some(self.parse_filter_expr()?),
                _ => return Err(self.error(&format!("Unknown injection field: {}", field))),
            }
        }
        
        self.expect(TokenKind::RBrace)?;
        
        Ok(InjectionDef { source, target, mode, priority, max_tokens, filter })
    }
    
    fn parse_injection_mode(&mut self) -> Result<InjectionMode, ParseError> {
        match self.current().kind {
            TokenKind::Full => { self.advance(); Ok(InjectionMode::Full) }
            TokenKind::Summary => { self.advance(); Ok(InjectionMode::Summary) }
            TokenKind::TopK => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let k = self.expect_number()? as usize;
                self.expect(TokenKind::RParen)?;
                Ok(InjectionMode::TopK(k))
            }
            TokenKind::Relevant => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let threshold = self.expect_number()? as f32;
                self.expect(TokenKind::RParen)?;
                Ok(InjectionMode::Relevant(threshold))
            }
            _ => Err(self.error("Expected injection mode")),
        }
    }
    
    fn parse_filter_expr(&mut self) -> Result<FilterExpr, ParseError> {
        self.parse_or_expr()
    }
    
    fn parse_or_expr(&mut self) -> Result<FilterExpr, ParseError> {
        let mut left = self.parse_and_expr()?;
        
        while self.check(TokenKind::Or) {
            self.advance();
            let right = self.parse_and_expr()?;
            left = FilterExpr::Or(vec![left, right]);
        }
        
        Ok(left)
    }
    
    fn parse_and_expr(&mut self) -> Result<FilterExpr, ParseError> {
        let mut left = self.parse_comparison()?;
        
        while self.check(TokenKind::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = FilterExpr::And(vec![left, right]);
        }
        
        Ok(left)
    }
    
    fn parse_comparison(&mut self) -> Result<FilterExpr, ParseError> {
        if self.check(TokenKind::Not) {
            self.advance();
            let expr = self.parse_comparison()?;
            return Ok(FilterExpr::Not(Box::new(expr)));
        }
        
        if self.check(TokenKind::LParen) {
            self.advance();
            let expr = self.parse_filter_expr()?;
            self.expect(TokenKind::RParen)?;
            return Ok(expr);
        }
        
        let field = self.expect_identifier()?;
        let op = self.parse_compare_op()?;
        let value = self.parse_filter_value()?;
        
        Ok(FilterExpr::Comparison { field, op, value })
    }
    
    fn parse_compare_op(&mut self) -> Result<CompareOp, ParseError> {
        match self.current().kind {
            TokenKind::Eq => { self.advance(); Ok(CompareOp::Eq) }
            TokenKind::Ne => { self.advance(); Ok(CompareOp::Ne) }
            TokenKind::Gt => { self.advance(); Ok(CompareOp::Gt) }
            TokenKind::Lt => { self.advance(); Ok(CompareOp::Lt) }
            TokenKind::Ge => { self.advance(); Ok(CompareOp::Ge) }
            TokenKind::Le => { self.advance(); Ok(CompareOp::Le) }
            TokenKind::Contains => { self.advance(); Ok(CompareOp::Contains) }
            TokenKind::Regex => { self.advance(); Ok(CompareOp::Regex) }
            TokenKind::In => { self.advance(); Ok(CompareOp::In) }
            _ => Err(self.error("Expected comparison operator")),
        }
    }
    
    fn parse_filter_value(&mut self) -> Result<FilterValue, ParseError> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(FilterValue::String(s))
            }
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                Ok(FilterValue::Number(n))
            }
            TokenKind::Identifier(s) if s == "true" => {
                self.advance();
                Ok(FilterValue::Bool(true))
            }
            TokenKind::Identifier(s) if s == "false" => {
                self.advance();
                Ok(FilterValue::Bool(false))
            }
            TokenKind::Identifier(s) if s == "null" => {
                self.advance();
                Ok(FilterValue::Null)
            }
            TokenKind::Identifier(s) if s == "current_trajectory" => {
                self.advance();
                Ok(FilterValue::CurrentTrajectory)
            }
            TokenKind::Identifier(s) if s == "current_scope" => {
                self.advance();
                Ok(FilterValue::CurrentScope)
            }
            TokenKind::Identifier(s) if s == "now" => {
                self.advance();
                Ok(FilterValue::Now)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut values = Vec::new();
                while !self.check(TokenKind::RBracket) {
                    values.push(self.parse_filter_value()?);
                    self.optional_comma();
                }
                self.expect(TokenKind::RBracket)?;
                Ok(FilterValue::Array(values))
            }
            _ => Err(self.error("Expected filter value")),
        }
    }
    
    // Helper methods
    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }
    
    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }
    
    fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }
    
    fn check(&self, kind: TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(&kind)
    }
    
    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("Expected {:?}", kind)))
        }
    }
    
    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::Identifier(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(self.error("Expected identifier")),
        }
    }
    
    fn expect_string(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(self.error("Expected string")),
        }
    }
    
    fn expect_number(&mut self) -> Result<f64, ParseError> {
        match self.current().kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(n)
            }
            _ => Err(self.error("Expected number")),
        }
    }
    
    fn expect_string_or_number(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            TokenKind::Number(n) => {
                let s = n.to_string();
                self.advance();
                Ok(s)
            }
            _ => Err(self.error("Expected string or number")),
        }
    }
    
    fn optional_comma(&mut self) {
        if self.check(TokenKind::Comma) {
            self.advance();
        }
    }
    
    fn error(&self, msg: &str) -> ParseError {
        ParseError {
            message: msg.to_string(),
            span: self.current().span,
        }
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}
```

---

## 4. CODE GENERATOR (Rust → Rust)

The code generator emits Rust source code with pgrx:

```rust
pub struct CodeGenerator {
    output: String,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self { output: String::new() }
    }
    
    pub fn generate(&mut self, config: &CaliberConfig) -> String {
        self.emit_header();
        
        for def in &config.definitions {
            match def {
                Definition::Memory(m) => self.emit_memory(m),
                Definition::Policy(p) => self.emit_policy(p),
                Definition::Injection(i) => self.emit_injection(i),
                Definition::Adapter(_) => {} // Adapter is runtime config
            }
        }
        
        self.emit_footer();
        
        self.output.clone()
    }
    
    fn emit_header(&mut self) {
        self.output.push_str(r#"
// AUTO-GENERATED BY CALIBER DSL COMPILER
// DO NOT EDIT MANUALLY

use pgx::prelude::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};

"#);
    }
    
    fn emit_memory(&mut self, m: &MemoryDef) {
        // Emit struct
        self.output.push_str(&format!(r#"
#[derive(Debug, Clone, PostgresType)]
pub struct {} {{
"#, pascal_case(&m.name)));
        
        for field in &m.schema {
            let rust_type = self.field_type_to_rust(&field.field_type);
            self.output.push_str(&format!("    pub {}: {},\n", field.name, rust_type));
        }
        
        self.output.push_str("}\n\n");
        
        // Emit insert function
        self.output.push_str(&format!(r#"
#[pg_extern]
pub fn caliber_{}_insert(
"#, m.name));
        
        for (i, field) in m.schema.iter().enumerate() {
            let rust_type = self.field_type_to_rust(&field.field_type);
            if i > 0 { self.output.push_str(",\n"); }
            self.output.push_str(&format!("    {}: {}", field.name, rust_type));
        }
        
        self.output.push_str(&format!(r#"
) -> Uuid {{
    // Direct heap insert via pgrx
    let rel = unsafe {{
        PgRelation::open_with_name("caliber_{}")
            .expect("relation not found")
    }};
    
    // Build tuple and insert
    // ... (pgrx heap tuple code)
    
    {} // Return primary key
}}
"#, m.name, m.schema[0].name)); // Assumes first field is PK
        
        // Emit get function
        self.output.push_str(&format!(r#"
#[pg_extern]
pub fn caliber_{}_get(id: Uuid) -> Option<{}> {{
    // Direct index scan via pgrx
    // ... (pgrx index scan code)
    None
}}
"#, m.name, pascal_case(&m.name)));
    }
    
    fn emit_policy(&mut self, p: &PolicyDef) {
        self.output.push_str(&format!(r#"
/// Policy: {}
pub fn execute_policy_{}(trigger: Trigger, context: &PolicyContext) {{
    match trigger {{
"#, p.name, p.name));
        
        for rule in &p.rules {
            self.output.push_str(&format!(r#"
        Trigger::{:?} => {{
"#, rule.trigger));
            
            for action in &rule.actions {
                match action {
                    Action::Summarize(target) => {
                        self.output.push_str(&format!(
                            "            summarize_{}(context);\n", target
                        ));
                    }
                    Action::ExtractArtifacts(target) => {
                        self.output.push_str(&format!(
                            "            extract_artifacts_{}(context);\n", target
                        ));
                    }
                    Action::Checkpoint(target) => {
                        self.output.push_str(&format!(
                            "            checkpoint_{}(context);\n", target
                        ));
                    }
                    Action::Notify(channel) => {
                        self.output.push_str(&format!(
                            "            pg_notify(\"{}\", &context.id.to_string());\n", channel
                        ));
                    }
                    _ => {}
                }
            }
            
            self.output.push_str("        }\n");
        }
        
        self.output.push_str(r#"
        _ => {}
    }
}
"#);
    }
    
    fn emit_injection(&mut self, i: &InjectionDef) {
        self.output.push_str(&format!(r#"
/// Injection rule: {} -> {}
pub const INJECTION_{}: InjectionRule = InjectionRule {{
    source: "{}",
    target: "{}",
    mode: {:?},
    priority: {},
    max_tokens: {:?},
}};
"#, 
            i.source, i.target,
            i.source.to_uppercase(),
            i.source, i.target,
            i.mode,
            i.priority,
            i.max_tokens,
        ));
    }
    
    fn emit_footer(&mut self) {
        self.output.push_str(r#"
// Bootstrap function (one-time setup)
#[pg_extern]
pub fn caliber_init() {
    // Create tables using SPI (SQL is OK for one-time setup)
    Spi::run(include_str!("../sql/bootstrap.sql"))
        .expect("Failed to bootstrap");
}
"#);
    }
    
    fn field_type_to_rust(&self, ft: &FieldType) -> String {
        match ft {
            FieldType::Uuid => "Uuid".to_string(),
            FieldType::Text => "String".to_string(),
            FieldType::Int => "i64".to_string(),
            FieldType::Float => "f64".to_string(),
            FieldType::Bool => "bool".to_string(),
            FieldType::Timestamp => "DateTime<Utc>".to_string(),
            FieldType::Json => "serde_json::Value".to_string(),
            FieldType::Embedding(dim) => {
                format!("EmbeddingVector<{}>", dim.unwrap_or(1536))
            }
            FieldType::Enum(variants) => {
                // Would need to emit enum definition separately
                "String".to_string()
            }
            FieldType::Array(inner) => {
                format!("Vec<{}>", self.field_type_to_rust(inner))
            }
        }
    }
}

fn pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
```

---

## 5. USAGE

```rust
fn main() {
    let source = include_str!("config.caliber");
    
    // Lex
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    
    // Parse
    let mut parser = Parser::new(tokens);
    let config = parser.parse().expect("Parse error");
    
    // Generate Rust
    let mut codegen = CodeGenerator::new();
    let rust_code = codegen.generate(&config);
    
    // Write to file
    std::fs::write("src/generated.rs", rust_code).expect("Write failed");
    
    // Compile with cargo
    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .expect("Cargo failed");
}
```

---

## 6. KEY INSIGHT

The DSL compiler is a **build-time tool**, not a runtime dependency:

```
Development:
  config.caliber → caliber-compiler → generated.rs → cargo build → caliber.so

Runtime:
  Agent → caliber.so (pure Rust+pgrx) → Postgres Storage
```

There is **zero DSL parsing at runtime**. The DSL is compiled away into native Rust code.
