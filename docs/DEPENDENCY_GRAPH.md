# CALIBER Dependency Graph & Type System

**Generated for Step 0: Complete Semantic Planning**

---

## 1. Crate Dependency Graph (DAG)

### Visual Representation

```
                    ┌─────────────────┐
                    │  caliber-core   │  (Foundation - NO dependencies)
                    │  Data + context │
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┬───────────────────┐
         │                   │                   │                   │
         ▼                   ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│ caliber-storage │ │   caliber-pcp   │ │   caliber-llm   │ │   caliber-dsl   │
│  Storage trait  │ │   Validation    │ │      VAL        │ │   (core only)   │
└────────┬────────┘ └────────┬────────┘ └────────┬────────┘ └────────┬────────┘
         │                   │                   │                   │
         └───────────────────┼───────────────────┼───────────────────┘
                             │                   │
                             ▼                   │
                    ┌─────────────────┐          │
                    │ caliber-agents  │          │
                    │  Coordination   │          │
                    │ (core, storage) │          │
                    └────────┬────────┘          │
                             │                   │
                             └───────────────────┘
                                     │
                                     ▼
                            ┌─────────────────┐
                            │   caliber-pg    │  (pgrx extension - wires ALL)
                            │  Runtime system │
                            └─────────────────┘
```

### Dependency Matrix

| Crate           | Depends On                                      | Depended By                                    |
|-----------------|------------------------------------------------|------------------------------------------------|
| caliber-core    | (none - foundation)                            | ALL other crates                               |
| caliber-storage | caliber-core                                   | caliber-agents, caliber-pg                     |
| caliber-pcp     | caliber-core                                   | caliber-pg                                     |
| caliber-llm     | caliber-core                                   | caliber-pg                                     |
| caliber-agents  | caliber-core, caliber-storage                  | caliber-pg                                     |
| caliber-dsl     | caliber-core                                   | caliber-pg                                     |
| caliber-pg      | ALL crates + pgrx                              | (none - top-level)                             |

### DAG Verification

✅ **No cycles detected** - The dependency graph is a valid DAG:
- `caliber-core` has no dependencies (leaf node)
- All component crates depend only on `caliber-core` (except `caliber-agents`)
- `caliber-agents` depends on `caliber-core` + `caliber-storage` (valid - no cycle)
- `caliber-pg` is the root node, depending on all others

---

## 2. Type Flow Between Crates

### Types Exported from caliber-core (consumed by ALL)

```rust
// === Identity Types ===
pub type EntityId = Uuid;           // UUIDv7 - timestamp-sortable
pub type Timestamp = DateTime<Utc>;
pub type DurationMs = i64;
pub type ContentHash = [u8; 32];    // SHA-256
pub type RawContent = Vec<u8>;      // BYTEA

// === Enums ===
pub enum TTL { Persistent, Session, Scope, Duration(DurationMs), Ephemeral, ShortTerm, MediumTerm, LongTerm, Permanent }
pub enum EntityType { Trajectory, Scope, Artifact, Note, Turn, Lock, Message, Agent, Delegation, Handoff, Conflict }
pub enum MemoryCategory { Ephemeral, Working, Episodic, Semantic, Procedural, Meta }
pub enum TrajectoryStatus { Active, Completed, Failed, Suspended }
pub enum OutcomeStatus { Success, Partial, Failure }
pub enum TurnRole { User, Assistant, System, Tool }
pub enum ArtifactType { ErrorLog, CodePatch, DesignDecision, UserPreference, Fact, Constraint, ToolResult, IntermediateOutput, Custom, Code, Document, Data, Model, Config, Log, Summary, Decision, Plan }
pub enum ExtractionMethod { Explicit, Inferred, UserProvided }
pub enum NoteType { Convention, Strategy, Gotcha, Fact, Preference, Relationship, Procedure, Meta, Insight, Correction, Summary }

// === Core Structs ===
pub struct EntityRef { entity_type: EntityType, id: EntityId }
pub struct EmbeddingVector { data: Vec<f32>, model_id: String, dimensions: i32 }
pub struct Trajectory { ... }
pub struct TrajectoryOutcome { ... }
pub struct Scope { ... }
pub struct Checkpoint { context_state: RawContent, recoverable: bool }
pub struct Artifact { ... }
pub struct Provenance { source_turn: i32, extraction_method: ExtractionMethod, confidence: Option<f32> }
pub struct Note { ... }
pub struct Turn { ... }

// === Error Types ===
pub enum CaliberError { Storage(StorageError), Llm(LlmError), Validation(ValidationError), Config(ConfigError), Vector(VectorError), Agent(AgentError) }
pub enum StorageError { NotFound { entity_type, id }, InsertFailed { entity_type, reason }, UpdateFailed { entity_type, id, reason }, TransactionFailed { reason }, IndexError { index_name, reason } }
pub enum LlmError { ProviderNotConfigured, RequestFailed { provider, status, message }, RateLimited { provider, retry_after_ms }, InvalidResponse { provider, reason }, EmbeddingFailed { reason }, SummarizationFailed { reason } }
pub enum ValidationError { RequiredFieldMissing { field }, InvalidValue { field, reason }, ConstraintViolation { constraint, reason }, CircularReference { entity_type, ids }, StaleData { entity_type, id, age }, Contradiction { artifact_a, artifact_b } }
pub enum ConfigError { MissingRequired { field }, InvalidValue { field, value, reason }, IncompatibleOptions { option_a, option_b }, ProviderNotSupported { provider } }
pub enum VectorError { DimensionMismatch { expected, got }, InvalidVector { reason }, ModelMismatch { expected, got } }
pub enum AgentError { NotRegistered { agent_id }, LockAcquisitionFailed { resource, holder }, LockExpired { lock_id }, MessageDeliveryFailed { message_id, reason }, DelegationFailed { reason }, HandoffFailed { reason }, PermissionDenied { agent_id, action, resource } }
pub type CaliberResult<T> = Result<T, CaliberError>;

// === Config Types ===
pub struct CaliberConfig { ... }
pub struct SectionPriorities { user: i32, system: i32, artifacts: i32, notes: i32, history: i32, custom: Vec<(String, i32)> }
pub enum ContextPersistence { Ephemeral, Ttl(Duration), Permanent }
pub enum ValidationMode { OnMutation, Always }
pub struct ProviderConfig { provider_type: String, endpoint: Option<String>, model: String, dimensions: Option<i32> }
pub struct RetryConfig { max_retries: i32, initial_backoff: Duration, max_backoff: Duration, backoff_multiplier: f32 }
```

### Types Exported from caliber-dsl (consumed by caliber-pg)

```rust
// === Lexer Types ===
pub enum TokenKind { /* 50+ variants */ }
pub struct Token { kind: TokenKind, span: Span }
pub struct Span { start: usize, end: usize, line: usize, column: usize }
pub struct Lexer<'a> { ... }

// === AST Types ===
pub struct CaliberAst { version: String, definitions: Vec<Definition> }
pub enum Definition { Adapter(AdapterDef), Memory(MemoryDef), Policy(PolicyDef), Injection(InjectionDef) }
pub struct AdapterDef { name: String, adapter_type: AdapterType, connection: String, options: Vec<(String, String)> }
pub enum AdapterType { Postgres, Redis, Memory }
pub struct MemoryDef { name: String, memory_type: MemoryType, schema: Vec<FieldDef>, retention: Retention, lifecycle: Lifecycle, parent: Option<String>, indexes: Vec<IndexDef>, inject_on: Vec<Trigger>, artifacts: Vec<String> }
pub enum MemoryType { Ephemeral, Working, Episodic, Semantic, Procedural, Meta }
pub struct FieldDef { name: String, field_type: FieldType, nullable: bool, default: Option<String> }
pub enum FieldType { Uuid, Text, Int, Float, Bool, Timestamp, Json, Embedding(Option<usize>), Enum(Vec<String>), Array(Box<FieldType>) }
pub enum Retention { Persistent, Session, Scope, Duration(String), Max(usize) }
pub enum Lifecycle { Explicit, AutoClose(Trigger) }
pub enum Trigger { TaskStart, TaskEnd, ScopeClose, TurnEnd, Manual, Schedule(String) }
pub struct IndexDef { field: String, index_type: IndexType, options: Vec<(String, String)> }
pub enum IndexType { Btree, Hash, Gin, Hnsw, Ivfflat }
pub struct PolicyDef { name: String, rules: Vec<PolicyRule> }
pub struct PolicyRule { trigger: Trigger, actions: Vec<Action> }
pub enum Action { Summarize(String), ExtractArtifacts(String), Checkpoint(String), Prune { target: String, criteria: FilterExpr }, Notify(String), Inject { target: String, mode: InjectionMode } }
pub struct InjectionDef { source: String, target: String, mode: InjectionMode, priority: i32, max_tokens: Option<i32>, filter: Option<FilterExpr> }
pub enum InjectionMode { Full, Summary, TopK(usize), Relevant(f32) }
pub enum FilterExpr { Comparison { field, op, value }, And(Vec<FilterExpr>), Or(Vec<FilterExpr>), Not(Box<FilterExpr>) }
pub enum CompareOp { Eq, Ne, Gt, Lt, Ge, Le, Contains, Regex, In }
pub enum FilterValue { String(String), Number(f64), Bool(bool), Null, CurrentTrajectory, CurrentScope, Now, Array(Vec<FilterValue>) }

// === Parser Types ===
pub struct Parser { tokens: Vec<Token>, pos: usize }
pub struct ParseError { message: String, line: usize, column: usize }
```

### Types Exported from caliber-llm (consumed by caliber-pg)

```rust
// === Traits ===
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector>;
    async fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>>;
    fn dimensions(&self) -> i32;
    fn model_id(&self) -> &str;
}

#[async_trait]
pub trait SummarizationProvider: Send + Sync {
    async fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String>;
    async fn extract_artifacts(&self, content: &str, types: &[ArtifactType]) -> CaliberResult<Vec<ExtractedArtifact>>;
    async fn detect_contradiction(&self, a: &str, b: &str) -> CaliberResult<bool>;
}

pub enum ProviderCapability { Embedding, Summarization, ArtifactExtraction, ContradictionDetection }
pub enum HealthStatus { Healthy, Degraded, Unhealthy, Unknown }

// === Structs ===
pub struct SummarizeConfig { max_tokens: i32, style: SummarizeStyle }
pub enum SummarizeStyle { Brief, Detailed, Structured }
pub struct ExtractedArtifact { artifact_type: ArtifactType, content: String, confidence: f32 }
pub struct EchoRequest { capabilities: Vec<ProviderCapability>, request_id: Uuid, timestamp: Timestamp }
pub struct PingResponse { provider_id: String, capabilities: Vec<ProviderCapability>, latency_ms: u64, health: HealthStatus, metadata: HashMap<String, String> }
pub struct EmbedRequest { text: String, request_id: Uuid }
pub struct EmbedResponse { embedding: EmbeddingVector, request_id: Uuid, latency_ms: u64 }
pub struct SummarizeRequest { content: String, config: SummarizeConfig, request_id: Uuid }
pub struct SummarizeResponse { summary: String, request_id: Uuid, latency_ms: u64 }
pub struct RequestEvent { request_id: Uuid, provider_id: String, operation: String, timestamp: Timestamp }
pub struct ResponseEvent { request_id: Uuid, provider_id: String, operation: String, latency_ms: u64, success: bool, timestamp: Timestamp }
pub struct ErrorEvent { request_id: Uuid, provider_id: String, operation: String, error_message: String, timestamp: Timestamp }

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn provider_id(&self) -> &str;
    fn capabilities(&self) -> &[ProviderCapability];
    async fn ping(&self) -> CaliberResult<PingResponse>;
    async fn embed(&self, request: EmbedRequest) -> CaliberResult<EmbedResponse>;
    async fn summarize(&self, request: SummarizeRequest) -> CaliberResult<SummarizeResponse>;
}

pub enum CircuitState { Closed, Open, HalfOpen }
pub struct CircuitBreakerConfig { failure_threshold: u32, success_threshold: u32, timeout: Duration }
pub struct CircuitBreaker { state: AtomicU8, failure_count: AtomicU32, success_count: AtomicU32, last_failure: RwLock<Option<Instant>>, config: CircuitBreakerConfig }

pub enum RoutingStrategy { RoundRobin, LeastLatency, Random, Capability(ProviderCapability), First }

pub struct ListenerChain { listeners: Vec<Arc<dyn EventListener>> }

#[async_trait]
pub trait EventListener: Send + Sync {
    async fn on_request(&self, event: RequestEvent) -> CaliberResult<()>;
    async fn on_response(&self, event: ResponseEvent) -> CaliberResult<()>;
    async fn on_error(&self, event: ErrorEvent) -> CaliberResult<()>;
}

pub struct ProviderRegistry { adapters: TokioRwLock<HashMap<String, Arc<dyn ProviderAdapter>>>, routing_strategy: RoutingStrategy, health_cache: TokioRwLock<HashMap<String, (PingResponse, Instant)>>, health_cache_ttl: Duration, round_robin_index: AtomicU64, listeners: TokioRwLock<ListenerChain>, circuit_breakers: TokioRwLock<HashMap<String, Arc<CircuitBreaker>>> }

pub struct EmbeddingCache { cache: RwLock<HashMap<[u8; 32], EmbeddingVector>>, max_size: usize }
pub struct CostTracker { embedding_tokens: AtomicI64, completion_input: AtomicI64, completion_output: AtomicI64 }
```

### Context assembly types (caliber-core::context, consumed by caliber-pg)

```rust
pub struct ContextPackage { trajectory_id: EntityId, scope_id: EntityId, user_input: Option<String>, relevant_notes: Vec<Note>, recent_artifacts: Vec<Artifact>, scope_summaries: Vec<ScopeSummary>, session_markers: SessionMarkers, kernel_config: Option<KernelConfig> }
pub struct SessionMarkers { active_trajectory_id: Option<EntityId>, active_scope_id: Option<EntityId>, recent_artifact_ids: Vec<EntityId>, agent_id: Option<EntityId> }
pub struct KernelConfig { persona: Option<String>, tone: Option<String>, reasoning_style: Option<String>, domain_focus: Option<String> }
pub struct ContextWindow { window_id: EntityId, assembled_at: Timestamp, max_tokens: i32, used_tokens: i32, sections: Vec<ContextSection>, truncated: bool, included_sections: Vec<String>, assembly_trace: Vec<AssemblyDecision> }
pub struct ContextSection { section_id: EntityId, section_type: SectionType, content: String, token_count: i32, priority: i32, compressible: bool, sources: Vec<SourceRef> }
pub enum SectionType { System, Persona, Notes, History, Artifacts, User }
pub struct SourceRef { source_type: EntityType, id: Option<EntityId>, relevance_score: Option<f32> }
pub struct AssemblyDecision { timestamp: Timestamp, action: AssemblyAction, target_type: String, target_id: Option<EntityId>, reason: String, tokens_affected: i32 }
pub enum AssemblyAction { Include, Exclude, Compress, Truncate }
pub struct ContextAssembler { config: CaliberConfig }
pub struct ScopeSummary { scope_id: EntityId, summary: String, token_count: i32 }
```

### Types Exported from caliber-pcp (consumed by caliber-pg)

```rust
// === Config Types ===
pub struct PCPConfig { context_dag: ContextDagConfig, recovery: RecoveryConfig, dosage: DosageConfig, anti_sprawl: AntiSprawlConfig, grounding: GroundingConfig }
pub struct ContextDagConfig { max_depth: i32, prune_strategy: PruneStrategy }
pub enum PruneStrategy { OldestFirst, LowestRelevance, Hybrid }
pub struct RecoveryConfig { enabled: bool, frequency: RecoveryFrequency, max_checkpoints: i32 }
pub enum RecoveryFrequency { OnScopeClose, OnMutation, Manual }
pub struct DosageConfig { max_tokens_per_scope: i32, max_artifacts_per_scope: i32, max_notes_per_trajectory: i32 }
pub struct AntiSprawlConfig { max_trajectory_depth: i32, max_concurrent_scopes: i32 }
pub struct GroundingConfig { require_artifact_backing: bool, contradiction_threshold: f32, conflict_resolution: ConflictResolution }
pub enum ConflictResolution { LastWriteWins, HighestConfidence, Escalate }

// === Runtime Types ===
pub struct PCPRuntime { config: PCPConfig }
pub struct PCPCheckpoint { checkpoint_id: EntityId, scope_id: EntityId, state: CheckpointState, created_at: Timestamp }
pub struct CheckpointState { context_snapshot: RawContent, artifact_ids: Vec<EntityId>, note_ids: Vec<EntityId> }

// === Result Types ===
pub struct ValidationResult { valid: bool, issues: Vec<ValidationIssue> }
pub struct ValidationIssue { severity: Severity, issue_type: IssueType, message: String, entity_id: Option<EntityId> }
pub enum Severity { Warning, Error, Critical }
pub enum IssueType { StaleData, Contradiction, MissingReference, DosageExceeded, CircularDependency }
pub struct RecoveryResult { success: bool, recovered_scope: Option<Scope>, errors: Vec<String> }
pub struct LintResult { passed: bool, issues: Vec<LintIssue> }
pub struct LintIssue { issue_type: LintIssueType, message: String, artifact_id: EntityId }
pub enum LintIssueType { TooLarge, Duplicate, MissingEmbedding, LowConfidence }

// === Memory Commit Types ===
pub struct MemoryCommit { commit_id: EntityId, trajectory_id: EntityId, scope_id: EntityId, agent_id: Option<EntityId>, query: String, response: String, mode: String, reasoning_trace: Option<serde_json::Value>, rag_contributed: bool, artifacts_referenced: Vec<EntityId>, notes_referenced: Vec<EntityId>, tools_invoked: Vec<String>, tokens_input: i64, tokens_output: i64, estimated_cost: Option<f64>, created_at: Timestamp }
pub struct RecallService { config: CaliberConfig }
pub struct DecisionRecall { commit_id: EntityId, query: String, decision_summary: String, mode: String, created_at: Timestamp }
pub struct ScopeHistory { scope_id: EntityId, interaction_count: i32, total_tokens: i64, total_cost: f64, commits: Vec<MemoryCommit> }
pub struct MemoryStats { total_interactions: i64, total_tokens: i64, total_cost: f64, unique_scopes: i64, by_mode: HashMap<String, i64>, avg_tokens_per_interaction: i64 }
```

### Types Exported from caliber-storage (consumed by caliber-agents, caliber-pg)

```rust
pub trait StorageTrait {
    fn trajectory_insert(&self, t: &Trajectory) -> CaliberResult<()>;
    fn trajectory_get(&self, id: EntityId) -> CaliberResult<Option<Trajectory>>;
    fn trajectory_update(&self, id: EntityId, update: TrajectoryUpdate) -> CaliberResult<()>;
    fn scope_insert(&self, s: &Scope) -> CaliberResult<()>;
    fn scope_get(&self, id: EntityId) -> CaliberResult<Option<Scope>>;
    fn scope_get_current(&self, trajectory_id: EntityId) -> CaliberResult<Option<Scope>>;
    fn artifact_insert(&self, a: &Artifact) -> CaliberResult<()>;
    fn artifact_get(&self, id: EntityId) -> CaliberResult<Option<Artifact>>;
    fn artifact_query_by_type(&self, trajectory_id: EntityId, artifact_type: ArtifactType) -> CaliberResult<Vec<Artifact>>;
    fn note_insert(&self, n: &Note) -> CaliberResult<()>;
    fn note_get(&self, id: EntityId) -> CaliberResult<Option<Note>>;
    fn turn_insert(&self, t: &Turn) -> CaliberResult<()>;
    fn turn_get_by_scope(&self, scope_id: EntityId) -> CaliberResult<Vec<Turn>>;
    fn vector_search(&self, query: &EmbeddingVector, limit: i32) -> CaliberResult<Vec<(EntityId, f32)>>;
}
```

### Types Exported from caliber-agents (consumed by caliber-pg)

```rust
// === Agent Types ===
pub struct Agent { agent_id: EntityId, agent_type: String, capabilities: Vec<String>, memory_access: MemoryAccess, status: AgentStatus, current_trajectory_id: Option<EntityId>, current_scope_id: Option<EntityId>, can_delegate_to: Vec<String>, reports_to: Option<EntityId>, created_at: Timestamp, last_heartbeat: Timestamp }
pub enum AgentStatus { Idle, Active, Blocked, Failed }
pub struct MemoryAccess { read: Vec<MemoryPermission>, write: Vec<MemoryPermission> }
pub struct MemoryPermission { memory_type: String, scope: PermissionScope, filter: Option<String> }
pub enum PermissionScope { Own, Team, Global }

// === Region Types ===
pub enum MemoryRegion { Private, Team, Public, Collaborative }
pub struct MemoryRegionConfig { region_id: EntityId, region_type: MemoryRegion, owner_agent_id: EntityId, team_id: Option<EntityId>, readers: Vec<EntityId>, writers: Vec<EntityId>, require_lock: bool, conflict_resolution: ConflictResolution, version_tracking: bool }

// === Lock Types ===
pub struct DistributedLock { lock_id: EntityId, resource_type: String, resource_id: EntityId, holder_agent_id: EntityId, acquired_at: Timestamp, expires_at: Timestamp, mode: LockMode }
pub enum LockMode { Exclusive, Shared }

// === Message Types ===
pub struct AgentMessage { message_id: EntityId, from_agent_id: EntityId, to_agent_id: Option<EntityId>, to_agent_type: Option<String>, message_type: MessageType, payload: String, trajectory_id: Option<EntityId>, scope_id: Option<EntityId>, artifact_ids: Vec<EntityId>, created_at: Timestamp, delivered_at: Option<Timestamp>, acknowledged_at: Option<Timestamp>, priority: MessagePriority, expires_at: Option<Timestamp> }
pub enum MessageType { TaskDelegation, TaskResult, ContextRequest, ContextShare, CoordinationSignal, Handoff, Interrupt, Heartbeat }
pub enum MessagePriority { Low, Normal, High, Critical }

// === Delegation Types ===
pub struct DelegatedTask { delegation_id: EntityId, delegator_agent_id: EntityId, delegatee_agent_id: Option<EntityId>, delegatee_agent_type: Option<String>, task_description: String, parent_trajectory_id: EntityId, child_trajectory_id: Option<EntityId>, shared_artifacts: Vec<EntityId>, shared_notes: Vec<EntityId>, additional_context: Option<String>, constraints: String, deadline: Option<Timestamp>, status: DelegationStatus, result: Option<DelegationResult>, created_at: Timestamp, accepted_at: Option<Timestamp>, completed_at: Option<Timestamp> }
pub enum DelegationStatus { Pending, Accepted, Rejected, InProgress, Completed, Failed }
pub struct DelegationResult { status: DelegationResultStatus, produced_artifacts: Vec<EntityId>, produced_notes: Vec<EntityId>, summary: String, error: Option<String> }
pub enum DelegationResultStatus { Success, Partial, Failure }

// === Handoff Types ===
pub struct AgentHandoff { handoff_id: EntityId, from_agent_id: EntityId, to_agent_id: Option<EntityId>, to_agent_type: Option<String>, trajectory_id: EntityId, scope_id: EntityId, context_snapshot_id: EntityId, handoff_notes: String, next_steps: Vec<String>, blockers: Vec<String>, open_questions: Vec<String>, status: HandoffStatus, initiated_at: Timestamp, accepted_at: Option<Timestamp>, completed_at: Option<Timestamp>, reason: HandoffReason }
pub enum HandoffStatus { Initiated, Accepted, Completed, Rejected }
pub enum HandoffReason { CapabilityMismatch, LoadBalancing, Specialization, Escalation, Timeout, Failure, Scheduled }

// === Conflict Types ===
pub struct Conflict { conflict_id: EntityId, conflict_type: ConflictType, item_a_type: String, item_a_id: EntityId, item_b_type: String, item_b_id: EntityId, agent_a_id: Option<EntityId>, agent_b_id: Option<EntityId>, trajectory_id: Option<EntityId>, status: ConflictStatus, resolution: Option<ConflictResolutionRecord>, detected_at: Timestamp, resolved_at: Option<Timestamp> }
pub enum ConflictType { ConcurrentWrite, ContradictingFact, IncompatibleDecision, ResourceContention, GoalConflict }
pub enum ConflictStatus { Detected, Resolving, Resolved, Escalated }
pub struct ConflictResolutionRecord { strategy: ResolutionStrategy, winner: Option<String>, merged_result_id: Option<EntityId>, reason: String, resolved_by: String }
pub enum ResolutionStrategy { LastWriteWins, FirstWriteWins, HighestConfidence, Merge, Escalate, RejectBoth }
```

### Types in caliber-pg (runtime orchestration)

```rust
pub struct CaliberOrchestrator { config: CaliberConfig, storage: Box<dyn StorageTrait>, providers: ProviderRegistry, pcp: PCPRuntime, assembler: ContextAssembler }

// === Tracing Types ===
pub struct CaliberTrace { trace_id: EntityId, trajectory_id: EntityId, events: Vec<TraceEvent>, started_at: Timestamp, completed_at: Option<Timestamp> }
pub struct TraceSummary { total_events: i32, total_tokens: i64, total_cost: f64, duration_ms: i64 }
pub struct TraceEvent { event_id: EntityId, event_type: TraceEventType, timestamp: Timestamp, details: serde_json::Value }
pub enum TraceEventType { ScopeOpen, ScopeClose, ArtifactCreate, NoteCreate, ContextAssemble, Checkpoint, LockAcquire, LockRelease, MessageSend, MessageReceive, Delegation, Handoff, Conflict }
```

---

## 3. Verification Checklist

### DAG Properties
- [x] No cycles in dependency graph
- [x] Single root node (caliber-pg)
- [x] Single leaf node (caliber-core)
- [x] All paths from root reach leaf

### Type Flow Properties
- [x] caliber-core exports only data types (no behavior)
- [x] All error types defined in caliber-core
- [x] CaliberResult<T> used consistently across crates
- [x] EmbeddingVector is dynamic (Vec<f32>, not const generic)
- [x] EntityId is UUIDv7 (timestamp-sortable)

### Architecture Properties
- [x] Only caliber-pg has pgrx runtime dependency
- [x] caliber-dsl produces CaliberConfig, not SQL
- [x] Storage trait defined in caliber-storage, implemented in caliber-pg
- [x] LLM traits defined in caliber-llm, user provides implementations

---

## 4. External Dependencies (Locked Versions)

### Core Dependencies

| Crate | Version | Features | Used By | Notes |
|-------|---------|----------|---------|-------|
| `pgrx` | `0.16` | default | caliber-pg | Pinned workspace version; check pgrx release notes for Postgres support |
| `uuid` | `1.11` | `v7`, `serde` | ALL | UUIDv7 for timestamp-sortable IDs |
| `chrono` | `0.4.39` | `serde` | ALL | DateTime handling |
| `serde` | `1.0` | `derive` | ALL | Serialization framework |
| `serde_json` | `1.0` | default | ALL | JSON serialization |
| `thiserror` | `2.0` | default | ALL | Error derive macros |
| `sha2` | `0.10` | default | caliber-core | Content hashing (SHA-256) |
| `regex` | `1.11` | default | caliber-pcp | Pattern matching for decision extraction |
| `once_cell` | `1.20` | default | caliber-pg | Lazy static initialization |

### Development Dependencies

| Crate | Version | Features | Used By | Notes |
|-------|---------|----------|---------|-------|
| `proptest` | `1.5` | default | ALL (dev) | Property-based testing |

### pgrx Compatibility Notes

- pgrx version is pinned in workspace `Cargo.toml` (currently `0.16`)
- Supported Postgres versions follow pgrx release notes; `caliber-pg/Cargo.toml` exposes `pg13`–`pg18`
- Keep pgvector version aligned with the pinned pgrx version

### Version Constraints

```toml
# Workspace Cargo.toml [workspace.dependencies]
pgrx = "0.16"
uuid = { version = "1.11", features = ["v7", "serde"] }
chrono = { version = "0.4.39", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
sha2 = "0.10"
regex = "1.11"
once_cell = "1.20"

[workspace.dev-dependencies]
proptest = "1.5"
```

### Known Constraints

1. **pgrx + pgvector**: Pin pgvector to a version compatible with the pinned pgrx release
2. **chrono + serde**: Always enable `serde` feature for DateTime serialization
3. **uuid v7**: Requires `v7` feature flag explicitly
4. **thiserror 2.0**: Breaking change from 1.x - uses `#[error(...)]` syntax

---

## 5. Free Batteries Inventory

### From pgrx (Free Derives)

| Feature | Usage | Notes |
|---------|-------|-------|
| `#[derive(PostgresType)]` | Custom Postgres types | Auto-generates type I/O functions |
| `#[derive(PostgresEnum)]` | Enum to Postgres enum | Maps Rust enum to Postgres enum |
| `#[pg_extern]` | Expose Rust fn to SQL | Creates SQL function wrapper |
| `pg_sys::pg_advisory_xact_lock` | Advisory locks | Transaction-scoped distributed locking |
| `Spi::run()` | One-time SQL execution | For bootstrap schema only |
| `pg_sys::Async_Notify` | NOTIFY | Real-time message passing |

### From Postgres (Free Infrastructure)

| Feature | Usage | Notes |
|---------|-------|-------|
| Advisory locks | Distributed locking | `pg_advisory_xact_lock(key)` |
| NOTIFY/LISTEN | Real-time messaging | Agent message delivery |
| HNSW indexes | Vector similarity | Via pgvector extension |
| WAL | Durability | Automatic write-ahead logging |
| MVCC | Transactions | Automatic isolation |
| Buffer manager | Caching | Automatic page caching |

### From Rust std

| Feature | Usage | Notes |
|---------|-------|-------|
| `Result<T, E>` | Error handling | CaliberResult<T> |
| `Option<T>` | Nullable values | Optional fields |
| `Vec<T>` | Dynamic arrays | EmbeddingVector.data |
| `HashMap<K, V>` | Key-value storage | MemoryStats.by_mode |
| `std::hash::Hash` | Hashing | Lock key computation |

### From External Crates

| Crate | Feature | Usage |
|-------|---------|-------|
| `uuid::Uuid` | UUIDv7 | Timestamp-sortable EntityId |
| `chrono::DateTime<Utc>` | Timestamps | All temporal fields |
| `sha2::Sha256` | Content hashing | ContentHash = [u8; 32] |
| `serde` | Serialization | JSON payloads, config |
| `thiserror` | Error derives | CaliberError variants |
| `proptest` | Property testing | Correctness properties |

---

## 6. Cargo.toml Template

### Workspace Root (Cargo.toml)

```toml
[workspace]
resolver = "2"
members = [
    "caliber-core",
    "caliber-storage",
    "caliber-pcp",
    "caliber-llm",
    "caliber-agents",
    "caliber-dsl",
    "caliber-pg",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = "https://github.com/caliber-run/caliber"

[workspace.dependencies]
# Core dependencies
uuid = { version = "1.11", features = ["v7", "serde"] }
chrono = { version = "0.4.39", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
sha2 = "0.10"
regex = "1.11"
once_cell = "1.20"

# pgrx (only for caliber-pg)
pgrx = "0.16"

# Internal crates
caliber-core = { path = "caliber-core" }
caliber-storage = { path = "caliber-storage" }
caliber-pcp = { path = "caliber-pcp" }
caliber-llm = { path = "caliber-llm" }
caliber-agents = { path = "caliber-agents" }
caliber-dsl = { path = "caliber-dsl" }

[workspace.dev-dependencies]
proptest = "1.5"

# === PROFILE OPTIMIZATIONS ===

[profile.dev]
opt-level = 0
debug = true
incremental = true

[profile.dev.package."*"]
opt-level = 2  # Optimize deps even in dev

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 16

[profile.test]
opt-level = 1  # Faster test compilation
```

### caliber-core/Cargo.toml

```toml
[package]
name = "caliber-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
uuid = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
sha2 = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

### caliber-storage/Cargo.toml

```toml
[package]
name = "caliber-storage"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
caliber-core = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

### Context module (caliber-core::context)

Context assembly lives inside `caliber-core` as a module; there is no standalone
context crate in the current workspace.

### caliber-pcp/Cargo.toml

```toml
[package]
name = "caliber-pcp"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
caliber-core = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
regex = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

### caliber-llm/Cargo.toml

```toml
[package]
name = "caliber-llm"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
caliber-core = { workspace = true }
uuid = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

### caliber-agents/Cargo.toml

```toml
[package]
name = "caliber-agents"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
caliber-core = { workspace = true }
caliber-storage = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

### caliber-dsl/Cargo.toml

```toml
[package]
name = "caliber-dsl"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
caliber-core = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

### caliber-pg/Cargo.toml

```toml
[package]
name = "caliber-pg"
version.workspace = true
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["cdylib"]

[features]
default = ["pg18"]
pg13 = ["pgrx/pg13"]
pg14 = ["pgrx/pg14"]
pg15 = ["pgrx/pg15"]
pg16 = ["pgrx/pg16"]
pg17 = ["pgrx/pg17"]
pg18 = ["pgrx/pg18"]
pg_test = []

[dependencies]
pgrx = { workspace = true }
caliber-core = { workspace = true }
caliber-storage = { workspace = true }
caliber-pcp = { workspace = true }
caliber-llm = { workspace = true }
caliber-agents = { workspace = true }
caliber-dsl = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
once_cell = { workspace = true }

[dev-dependencies]
pgrx-tests = "0.16"
proptest = { workspace = true }
```

---

## 7. Known Gotchas & Constraints

### pgrx Constraints

1. **Rust Edition**: pgrx 0.16 targets Rust 2021 edition
2. **Postgres Versions**: Feature flags select version (pg13–pg18)
3. **cdylib**: caliber-pg must be `crate-type = ["cdylib"]`
4. **pg_test feature**: Required for pgrx integration tests

### Type System Constraints

1. **EmbeddingVector**: Use `Vec<f32>`, NOT const generics (dynamic dimensions)
2. **EntityId**: Must be UUIDv7 for timestamp-sortability
3. **ContentHash**: Fixed `[u8; 32]` for SHA-256
4. **RawContent**: Use `Vec<u8>` for BYTEA compatibility

### Error Handling Constraints

1. **CaliberError**: Single source of truth for all errors
2. **CaliberResult<T>**: Use everywhere, propagate with `?`
3. **No unwrap()**: Use `?` operator in production code
4. **ereport**: Errors propagate to Postgres via pgrx error macros

### Configuration Constraints

1. **No defaults**: Every config value must be explicit
2. **validate()**: CaliberConfig must validate all constraints
3. **Missing = Error**: ConfigError::MissingRequired for missing fields

---

**Step 0 Status:**
- [x] 0.1 Build Full Crate Dependency Graph
- [x] 0.2 Lock External Dependencies with Exact Versions
- [x] 0.3 Design Complete Type System (ALL types, ALL crates)
- [x] 0.4 Identify Free Batteries (Don't Reinvent)
- [x] 0.5 Create Optimized Cargo.toml Template
- [x] 0.6 Output: Create `docs/DEPENDENCY_GRAPH.md`

**Note:** Sections 0.3-0.5 are now complete within this document. The type system is documented in Section 2, free batteries in Section 5, and Cargo.toml template in Section 6.
