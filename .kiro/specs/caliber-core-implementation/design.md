# Design Document: CALIBER Core Implementation

## Overview

CALIBER is a Postgres-native memory framework for AI agents, built as a multi-crate Rust workspace using pgrx. The architecture follows ECS (Entity-Component-System) principles: entities are pure data in caliber-core, components provide behavior via traits, and systems wire everything together in caliber-pg.

The key innovation is bypassing SQL entirely in the hot path — agent memory operations go directly to Postgres storage via pgrx, avoiding the parsing/planning overhead that makes traditional approaches slow.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      CALIBER + PCP                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              CaliberConfig (user-provided)               │   │
│  │  • Every value explicit    • No defaults anywhere       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                PCP Protocol Layer                        │   │
│  │  • Context validation    • Checkpoint/recovery          │   │
│  │  • Dosage control        • Contradiction detection      │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              CALIBER Components (ECS)                    │   │
│  │  caliber-core │ caliber-storage │ caliber-context       │   │
│  │  caliber-pcp  │ caliber-llm     │ caliber-agents        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                pgrx Direct Storage                       │   │
│  │  • Heap tuple ops    • Index access    • WAL writes     │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              PostgreSQL Storage Engine                   │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Crate Dependency Graph

```
caliber-core (entities, no deps)
     ↑
     ├── caliber-storage (storage traits)
     ├── caliber-context (context assembly)
     ├── caliber-pcp (validation)
     ├── caliber-llm (VAL traits)
     ├── caliber-agents (coordination)
     └── caliber-dsl (parser)
              ↑
         caliber-pg (pgrx extension, wires everything)
```

## Components and Interfaces

### caliber-core: Entity Types

Pure data structures with no behavior. All other crates depend on this.

```rust
// Identity
pub type EntityId = Uuid;  // UUIDv7 for timestamp-sortability
pub type Timestamp = DateTime<Utc>;
pub type ContentHash = [u8; 32];
pub type RawContent = Vec<u8>;

// Core entities
pub struct Trajectory { ... }
pub struct Scope { ... }
pub struct Artifact { ... }
pub struct Note { ... }
pub struct Turn { ... }

// Embeddings (dynamic dimension)
pub struct EmbeddingVector {
    pub data: Vec<f32>,
    pub model_id: String,
    pub dimensions: i32,
}

// Errors (single source of truth)
pub enum CaliberError {
    Storage(StorageError),
    Llm(LlmError),
    Validation(ValidationError),
    Config(ConfigError),
    Vector(VectorError),
    Agent(AgentError),
}

pub type CaliberResult<T> = Result<T, CaliberError>;
```

### caliber-storage: Storage Traits

Defines storage interface, implemented via pgrx in caliber-pg.

```rust
pub trait StorageTrait {
    fn trajectory_insert(&self, t: &Trajectory) -> CaliberResult<()>;
    fn trajectory_get(&self, id: EntityId) -> CaliberResult<Option<Trajectory>>;
    fn scope_insert(&self, s: &Scope) -> CaliberResult<()>;
    fn artifact_insert(&self, a: &Artifact) -> CaliberResult<()>;
    fn note_insert(&self, n: &Note) -> CaliberResult<()>;
    fn vector_search(&self, query: &EmbeddingVector, limit: i32) -> CaliberResult<Vec<(EntityId, f32)>>;
}
```

### caliber-llm: VAL (Vector Abstraction Layer)

Provider-agnostic traits for embeddings and summarization.

```rust
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector>;
    fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>>;
    fn dimensions(&self) -> i32;
    fn model_id(&self) -> &str;
}

pub trait SummarizationProvider: Send + Sync {
    fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String>;
    fn extract_artifacts(&self, content: &str, types: &[ArtifactType]) -> CaliberResult<Vec<ExtractedArtifact>>;
    fn detect_contradiction(&self, a: &str, b: &str) -> CaliberResult<bool>;
}

pub struct ProviderRegistry {
    embedding: Option<Box<dyn EmbeddingProvider>>,
    summarization: Option<Box<dyn SummarizationProvider>>,
}
```

### caliber-dsl: DSL Parser

Lexer → Parser → Validator → CaliberConfig

```rust
// Lexer
pub struct Lexer<'a> { ... }
pub enum TokenKind { ... }
pub struct Token { kind: TokenKind, span: Span }

// AST
pub struct CaliberAst {
    pub version: String,
    pub definitions: Vec<Definition>,
}

pub enum Definition {
    Adapter(AdapterDef),
    Memory(MemoryDef),
    Policy(PolicyDef),
    Injection(InjectionDef),
}

// Parser
pub struct Parser { ... }
impl Parser {
    pub fn parse(&mut self) -> Result<CaliberAst, ParseError>;
}

// Code generator
pub fn generate_config(ast: &CaliberAst) -> CaliberResult<CaliberConfig>;
```

### caliber-agents: Multi-Agent Coordination

```rust
pub struct Agent {
    pub agent_id: EntityId,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
    pub memory_access: MemoryAccess,
}

pub struct DistributedLock {
    pub lock_id: EntityId,
    pub resource_type: String,
    pub resource_id: EntityId,
    pub holder_agent_id: EntityId,
    pub expires_at: Timestamp,
    pub mode: LockMode,
}

pub struct AgentMessage {
    pub message_id: EntityId,
    pub from_agent_id: EntityId,
    pub to_agent_id: Option<EntityId>,
    pub message_type: MessageType,
    pub payload: String,
    pub priority: MessagePriority,
}

pub struct DelegatedTask { ... }
pub struct AgentHandoff { ... }
pub struct Conflict { ... }
```

### caliber-context: Context Assembly

Inspired by the Context Conveyor pattern — combines all inputs into a single coherent prompt with token budget management.

```rust
/// Context package - all inputs for assembly
/// Similar to ContextPackage in the TypeScript CRM
pub struct ContextPackage {
    pub trajectory_id: EntityId,
    pub scope_id: EntityId,
    
    /// Current user query/input
    pub user_input: Option<String>,
    
    /// Relevant notes (semantic memory)
    pub relevant_notes: Vec<Note>,
    
    /// Recent artifacts from current trajectory
    pub recent_artifacts: Vec<Artifact>,
    
    /// Scope summaries (compressed history)
    pub scope_summaries: Vec<ScopeSummary>,
    
    /// Session markers (active context)
    pub session_markers: SessionMarkers,
    
    /// Kernel/persona configuration
    pub kernel_config: Option<KernelConfig>,
}

pub struct SessionMarkers {
    pub active_trajectory_id: Option<EntityId>,
    pub active_scope_id: Option<EntityId>,
    pub recent_artifact_ids: Vec<EntityId>,
    pub agent_id: Option<EntityId>,
}

pub struct KernelConfig {
    pub persona: Option<String>,
    pub tone: Option<String>,
    pub reasoning_style: Option<String>,
    pub domain_focus: Option<String>,
}

pub struct ContextWindow {
    pub window_id: EntityId,
    pub assembled_at: Timestamp,
    
    /// Token budget management
    pub max_tokens: i32,
    pub used_tokens: i32,
    
    /// Sections ordered by priority
    pub sections: Vec<ContextSection>,
    
    /// Whether any section was truncated
    pub truncated: bool,
    
    /// Which sections were included
    pub included_sections: Vec<String>,
    
    /// Full audit trail of assembly decisions
    pub assembly_trace: Vec<AssemblyDecision>,
}

pub struct ContextSection {
    pub section_id: EntityId,
    pub section_type: SectionType,
    pub content: String,
    pub token_count: i32,
    pub priority: i32,
    pub compressible: bool,
    pub sources: Vec<SourceRef>,
}

pub struct ContextAssembler {
    config: CaliberConfig,
}

impl ContextAssembler {
    /// Assemble context from package with token budget management
    /// Sections are added in priority order until budget exhausted
    pub fn assemble(&self, pkg: ContextPackage) -> CaliberResult<ContextWindow>;
    
    /// Estimate tokens for text (rough: ~0.75 tokens per char for English)
    pub fn estimate_tokens(text: &str) -> i32;
    
    /// Truncate text to fit token budget, preferring sentence boundaries
    pub fn truncate_to_budget(text: &str, budget: i32) -> String;
}
```

### Token Utilities

```rust
/// Token estimation and truncation utilities
/// Ported from TypeScript CRM patterns

/// Estimate token count for text
/// Rough estimate: ~0.75 tokens per character (English)
/// More accurate than 1:4 ratio, less accurate than tiktoken
pub fn estimate_tokens(text: &str) -> i32 {
    (text.len() as f32 * 0.75).ceil() as i32
}

/// Truncate text to fit within token budget
/// Prefers sentence boundaries, falls back to word boundaries
pub fn truncate_to_token_budget(text: &str, budget: i32) -> String {
    let max_chars = (budget as f32 / 0.75).floor() as usize;
    if text.len() <= max_chars {
        return text.to_string();
    }
    
    let truncated = &text[..max_chars];
    
    // Try sentence boundary
    let last_period = truncated.rfind('.');
    let last_question = truncated.rfind('?');
    let last_exclaim = truncated.rfind('!');
    let last_sentence = [last_period, last_question, last_exclaim]
        .into_iter()
        .flatten()
        .max();
    
    if let Some(pos) = last_sentence {
        if pos > max_chars / 2 {
            return truncated[..=pos].to_string();
        }
    }
    
    // Fall back to word boundary
    if let Some(pos) = truncated.rfind(' ') {
        if pos > max_chars * 4 / 5 {
            return truncated[..pos].to_string();
        }
    }
    
    truncated.to_string()
}
```

### caliber-pcp: Validation & Checkpoints

PCP (Persistent Context Protocol) provides harm reduction through validation, checkpointing, and contradiction detection.

```rust
pub struct PCPValidator {
    config: CaliberConfig,
}

impl PCPValidator {
    pub fn validate_context(&self, ctx: &ContextWindow) -> CaliberResult<ValidationResult>;
    pub fn detect_contradictions(&self, artifacts: &[Artifact]) -> CaliberResult<Vec<Contradiction>>;
    pub fn create_checkpoint(&self, scope: &Scope) -> CaliberResult<Checkpoint>;
    pub fn recover_from_checkpoint(&self, checkpoint: &Checkpoint) -> CaliberResult<Scope>;
}
```

### Memory Commit Layer (Git-Style Versioning)

Inspired by the Memory Commit pattern from the TypeScript CRM — every interaction creates a versioned commit for recall, rollback, audit, and rehydration.

```rust
/// Memory commit - versioned record of an interaction
/// Enables: "Last time we decided X because Y"
pub struct MemoryCommit {
    pub commit_id: EntityId,
    pub trajectory_id: EntityId,
    pub scope_id: EntityId,
    pub agent_id: Option<EntityId>,
    
    /// Input/output
    pub query: String,
    pub response: String,
    
    /// Metadata
    pub mode: String,  // "standard", "deep_work", "super_think"
    pub reasoning_trace: Option<serde_json::Value>,
    
    /// Context contribution
    pub rag_contributed: bool,
    pub artifacts_referenced: Vec<EntityId>,
    pub notes_referenced: Vec<EntityId>,
    
    /// Tool execution
    pub tools_invoked: Vec<String>,
    
    /// Cost tracking
    pub tokens_input: i64,
    pub tokens_output: i64,
    pub estimated_cost: Option<f64>,
    
    /// Timestamps
    pub created_at: Timestamp,
}

/// Recall service - query past interactions
pub struct RecallService {
    config: CaliberConfig,
}

impl RecallService {
    /// Recall previous interactions for context
    /// "Last time we decided X because Y"
    pub fn recall_previous(
        &self,
        trajectory_id: Option<EntityId>,
        scope_id: Option<EntityId>,
        limit: i32,
    ) -> CaliberResult<Vec<MemoryCommit>>;
    
    /// Search interactions by content
    pub fn search_interactions(
        &self,
        search_text: &str,
        limit: i32,
    ) -> CaliberResult<Vec<MemoryCommit>>;
    
    /// Recall decisions made in past interactions
    /// Filters for decision-support interactions
    pub fn recall_decisions(
        &self,
        topic: Option<&str>,
        limit: i32,
    ) -> CaliberResult<Vec<DecisionRecall>>;
    
    /// Get session/scope history
    pub fn get_scope_history(
        &self,
        scope_id: EntityId,
    ) -> CaliberResult<ScopeHistory>;
    
    /// Get memory stats for analytics
    pub fn get_memory_stats(
        &self,
        trajectory_id: Option<EntityId>,
    ) -> CaliberResult<MemoryStats>;
}

pub struct DecisionRecall {
    pub commit_id: EntityId,
    pub query: String,
    pub decision_summary: String,  // Extracted recommendation
    pub mode: String,
    pub created_at: Timestamp,
}

pub struct ScopeHistory {
    pub scope_id: EntityId,
    pub interaction_count: i32,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub commits: Vec<MemoryCommit>,
}

pub struct MemoryStats {
    pub total_interactions: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub unique_scopes: i64,
    pub by_mode: HashMap<String, i64>,
    pub avg_tokens_per_interaction: i64,
}

/// Extract decision summary from response
/// Looks for recommendation patterns
fn extract_decision(response: &str) -> String {
    let patterns = [
        r"recommend[^\n.]*[.]",
        r"should[^\n.]*[.]",
        r"decision[^\n.]*[.]",
        r"conclude[^\n.]*[.]",
    ];
    
    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(m) = re.find(response) {
                return m.as_str().to_string();
            }
        }
    }
    
    // Fall back to first sentence
    response.split(&['.', '!', '?'][..])
        .next()
        .map(|s| format!("{}.", s))
        .unwrap_or_else(|| response.chars().take(200).collect())
}
```

## Data Models

### Memory Hierarchy

```
Trajectory (task container)
├── Scope (context partition)
│   ├── Turn (ephemeral) - dies with scope
│   └── Artifact (preserved) - survives scope close
└── Note (cross-trajectory) - long-term knowledge
```

### Entity Relationships

```
Trajectory 1──* Scope
Scope 1──* Turn
Scope 1──* Artifact
Trajectory *──* Note (via source_trajectory_ids)
Agent 1──* Trajectory (via current_trajectory_id)
Agent *──* Agent (via delegation/handoff)
```

### Configuration Schema

```rust
pub struct CaliberConfig {
    // Context assembly (REQUIRED)
    pub token_budget: i32,
    pub section_priorities: SectionPriorities,
    
    // PCP settings (REQUIRED)
    pub checkpoint_retention: i32,
    pub stale_threshold: Duration,
    pub contradiction_threshold: f32,
    
    // Storage (REQUIRED)
    pub context_window_persistence: ContextPersistence,
    pub validation_mode: ValidationMode,
    
    // LLM (optional, but required if using embeddings)
    pub embedding_provider: Option<ProviderConfig>,
    pub summarization_provider: Option<ProviderConfig>,
    pub llm_retry_config: RetryConfig,
    
    // Multi-agent (REQUIRED)
    pub lock_timeout: Duration,
    pub message_retention: Duration,
    pub delegation_timeout: Duration,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Configuration validation rejects invalid values
*For any* CaliberConfig with token_budget <= 0 OR contradiction_threshold outside [0.0, 1.0], validate() SHALL return Err(ConfigError::InvalidValue)
**Validates: Requirements 3.4, 3.5**

### Property 2: Configuration validation rejects missing required fields
*For any* CaliberConfig construction attempt missing a required field, the system SHALL return ConfigError::MissingRequired
**Validates: Requirements 3.2**

### Property 3: DSL round-trip parsing preserves semantics
*For any* valid CaliberAst, pretty-printing then parsing SHALL produce an equivalent AST
**Validates: Requirements 5.8**

### Property 4: Lexer produces Error token for invalid characters
*For any* input string containing characters outside the valid token set, the Lexer SHALL produce at least one TokenKind::Error
**Validates: Requirements 4.8**

### Property 5: EmbeddingVector dimension mismatch detection
*For any* two EmbeddingVectors with different dimensions, cosine_similarity() SHALL return Err(VectorError::DimensionMismatch)
**Validates: Requirements 6.6**

### Property 6: Provider registry returns error when not configured
*For any* ProviderRegistry with no embedding provider registered, calling embedding() SHALL return Err(LlmError::ProviderNotConfigured)
**Validates: Requirements 6.4**

### Property 7: EntityId uses UUIDv7 (timestamp-sortable)
*For any* two EntityIds generated in sequence, the first SHALL sort before the second lexicographically
**Validates: Requirements 2.3**

### Property 8: Context assembly respects token budget
*For any* ContextWindow assembled with max_tokens = N, used_tokens SHALL be <= N
**Validates: Requirements 9.3**

### Property 9: Lock acquisition records holder
*For any* successful lock acquisition by agent A on resource R, the DistributedLock SHALL have holder_agent_id = A
**Validates: Requirements 7.3**

### Property 10: Storage not-found returns correct error
*For any* storage query for a non-existent EntityId, the system SHALL return StorageError::NotFound with the correct entity_type
**Validates: Requirements 8.4**

### Property 11: Context sections ordered by priority
*For any* assembled ContextWindow, sections SHALL be ordered by descending priority
**Validates: Requirements 9.2**

### Property 12: Token estimation consistency
*For any* text T, estimate_tokens(T) SHALL be >= 0 AND approximately proportional to T.len()
**Validates: Context assembly token management**

### Property 13: Truncation respects budget
*For any* text T and budget B, estimate_tokens(truncate_to_token_budget(T, B)) SHALL be <= B
**Validates: Context assembly truncation**

### Property 14: Memory commit preserves query/response
*For any* MemoryCommit created with query Q and response R, recall SHALL return the same Q and R
**Validates: Memory commit layer integrity**

### Property 15: Recall decisions filters correctly
*For any* set of MemoryCommits, recall_decisions() SHALL only return commits where mode is "deep_work" or "super_think" OR response contains decision keywords
**Validates: Decision recall filtering**

## Error Handling

All errors flow through CaliberError enum and propagate to Postgres via ereport:

```rust
impl CaliberError {
    pub fn report(self) -> ! {
        let (code, message) = match &self {
            CaliberError::Storage(e) => ("CALIBER_STORAGE", format!("{:?}", e)),
            CaliberError::Llm(e) => ("CALIBER_LLM", format!("{:?}", e)),
            CaliberError::Validation(e) => ("CALIBER_VALIDATION", format!("{:?}", e)),
            CaliberError::Config(e) => ("CALIBER_CONFIG", format!("{:?}", e)),
            CaliberError::Vector(e) => ("CALIBER_VECTOR", format!("{:?}", e)),
            CaliberError::Agent(e) => ("CALIBER_AGENT", format!("{:?}", e)),
        };
        pgx::error!("{}: {}", code, message);
    }
}
```

## Testing Strategy

CALIBER requires comprehensive testing at multiple levels. Testing is not an afterthought — it's core to the framework's reliability.

### Testing Pyramid

```
                    ┌─────────────┐
                    │   E2E/Smoke │  (Few, slow, high confidence)
                   ┌┴─────────────┴┐
                   │  Integration   │
                  ┌┴───────────────┴┐
                  │   Component     │
                 ┌┴─────────────────┴┐
                 │  Property-Based   │
                ┌┴───────────────────┴┐
                │      Unit Tests     │  (Many, fast, focused)
                └─────────────────────┘
```

### 1. Unit Tests
- Test each entity type's construction and field access
- Test CaliberConfig validation with valid/invalid inputs
- Test Lexer tokenization of all token types
- Test Parser with valid/invalid DSL snippets
- Test EmbeddingVector operations (cosine similarity, dimension checks)
- Test token estimation and truncation utilities
- Test error type construction and conversion

### 2. Property-Based Tests (proptest)
- Use `proptest` crate for Rust property-based testing
- Minimum 100 iterations per property test
- Each test tagged with: **Feature: caliber-core-implementation, Property N: [description]**
- Key properties:
  - DSL round-trip (parse → print → parse)
  - Config validation invariants
  - Vector dimension consistency
  - Token budget constraints

### 3. Component Tests
- Test each crate in isolation with mocked dependencies
- Test trait implementations against their contracts
- Test error propagation through component boundaries
- Test configuration validation at component level

### 4. Integration Tests
- Test full DSL → CaliberConfig pipeline
- Test storage operations via pgrx test framework
- Test multi-agent coordination scenarios
- Test context assembly with real storage
- Test PCP validation with real artifacts

### 5. End-to-End / Smoke Tests
- Test complete agent workflow: trajectory → scope → artifact → note
- Test multi-agent delegation and handoff
- Test checkpoint creation and recovery
- Test context assembly under token pressure
- Verify Postgres extension loads and initializes

### 6. Regression Tests
- Capture bugs as test cases before fixing
- Test edge cases discovered in production
- Test backward compatibility of DSL syntax
- Test config migration between versions

### 7. Chaos Testing
- Test behavior under storage failures
- Test lock timeout and expiration
- Test message delivery failures
- Test provider unavailability (LLM down)
- Test concurrent agent operations
- Test transaction rollback scenarios

### 8. Mutation Testing (cargo-mutants)
- Verify test suite catches code mutations
- Target >80% mutation score for core logic
- Focus on:
  - Validation logic
  - Error handling paths
  - Boundary conditions

### 9. Fuzz Testing (cargo-fuzz)
- Fuzz the DSL lexer with arbitrary byte sequences
- Fuzz the DSL parser with random token streams
- Fuzz config validation with random field values
- Fuzz vector operations with edge-case floats (NaN, Inf)

### 10. Security / Penetration Testing
- Test SQL injection resistance (even though we don't use SQL in hot path)
- Test advisory lock exhaustion
- Test message payload size limits
- Test config value bounds (prevent DoS via huge token budgets)
- Test tenant isolation in multi-tenant scenarios

### Test Infrastructure

```rust
// Test utilities crate: caliber-test-utils
pub mod generators {
    // Proptest generators for all entity types
    pub fn arb_trajectory() -> impl Strategy<Value = Trajectory>;
    pub fn arb_scope() -> impl Strategy<Value = Scope>;
    pub fn arb_artifact() -> impl Strategy<Value = Artifact>;
    pub fn arb_embedding_vector(dims: i32) -> impl Strategy<Value = EmbeddingVector>;
    pub fn arb_dsl_source() -> impl Strategy<Value = String>;
}

pub mod mocks {
    // Mock implementations for testing
    pub struct MockStorageTrait { ... }
    pub struct MockEmbeddingProvider { ... }
    pub struct MockSummarizationProvider { ... }
}

pub mod fixtures {
    // Pre-built test data
    pub fn sample_trajectory() -> Trajectory;
    pub fn sample_config() -> CaliberConfig;
    pub fn sample_dsl() -> &'static str;
}

pub mod assertions {
    // Custom assertions for CALIBER types
    pub fn assert_valid_config(config: &CaliberConfig);
    pub fn assert_tokens_within_budget(ctx: &ContextWindow);
    pub fn assert_sections_ordered_by_priority(ctx: &ContextWindow);
}
```

### CI/CD Test Pipeline

```yaml
# .github/workflows/test.yml
stages:
  - lint: cargo clippy --workspace -- -D warnings
  - unit: cargo test --workspace --lib
  - property: cargo test --workspace --test '*_prop' -- --test-threads=1
  - integration: cargo pgrx test
  - fuzz: cargo +nightly fuzz run lexer_fuzz -- -max_total_time=60
  - mutation: cargo mutants --workspace --timeout=300
  - coverage: cargo tarpaulin --workspace --out Html
```

### Coverage Targets
- Line coverage: >80%
- Branch coverage: >70%
- Mutation score: >80% for core logic

---

## 🎯 HACKATHON CHECKPOINTS

**Remember to do these as you implement!**

### After Workspace Setup (Req 1)
- [ ] Update DEVLOG.md with workspace structure decisions
- [ ] Commit with meaningful message

### After caliber-core (Req 2)
- [ ] Run @code-review on entity types
- [ ] Update DEVLOG.md with type design decisions

### After caliber-dsl Lexer (Req 4)
- [ ] Write property tests for lexer
- [ ] Update DEVLOG.md with parsing approach

### After caliber-dsl Parser (Req 5)
- [ ] Write round-trip property test
- [ ] Update DEVLOG.md with AST design

### After caliber-llm (Req 6)
- [ ] Document VAL design decisions
- [ ] Note any provider-specific challenges

### After caliber-agents (Req 7)
- [ ] Document coordination protocol decisions
- [ ] Note Postgres advisory lock usage

### Before Final Submission
- [ ] Run @code-review-hackathon
- [ ] Ensure README has clear setup instructions
- [ ] Record 2-5 minute demo video
- [ ] Verify DEVLOG.md is complete
