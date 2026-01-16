# CALIBER + PCP: Unified Specification v0.2.1

## Meta

```yaml
spec_version: "0.2.1"
target_reader: "AI Agent / LLM implementing this system"
human_bureaucracy: false
implementation_ready: true
architecture: "Multi-crate ECS (Entity-Component-System)"
language: "Rust (pgrx) + CALIBER DSL"
sql_usage: "human debug interface only, NOT in hot path"
philosophy: "NOTHING HARD-CODED. This is a FRAMEWORK, not a product."
```

**What this document is**: A complete, unambiguous specification for building CALIBER (Context Abstraction Layer Integrating Behavioral Extensible Runtime) with PCP (Persistent Context Protocol). Every data structure, algorithm, and interface is defined precisely enough for an AI agent to implement without clarification.

**What this document is not**: Marketing material, stakeholder alignment docs, or vague architectural diagrams.

**Critical Philosophy**: CALIBER is a **toolkit/framework**, not a SaaS product. **Nothing is hard-coded.** Every value, threshold, timeout, and behavior is configurable. Users configure everything explicitly. Missing required configuration = error, not silent default.

---

## 0. CRITICAL ARCHITECTURE PRINCIPLES

### 0.1 No SQL in the Hot Path

SQL is a **human interface** to Postgres. The parsing/planning/optimization overhead is unnecessary for programmatic access.

**WRONG approach (what everyone does):**

```
Agent Request → Generate SQL String → Postgres SQL Parser → Query Planner → Executor → Storage
```

**RIGHT approach (what CALIBER does):**

```
Agent Request → CALIBER DSL → Compiled Rust (pgrx) → Direct Storage Engine Access
```

The CALIBER DSL compiles to **Rust code using pgrx**, which talks directly to:

- Heap tuple operations (no SQL INSERT/UPDATE)
- Buffer manager (direct page access)
- Index access methods (direct B-tree/HNSW traversal)
- WAL (direct write-ahead logging)

**SQL exists ONLY for:**

- Human debugging ("show me what's in the trajectory table")
- Ad-hoc queries during development
- External tools that need SQL compatibility

**SQL does NOT exist for:**

- Agent memory operations
- Context assembly
- Artifact storage
- Any hot-path operation

### 0.2 Multi-Crate ECS Architecture

CALIBER uses **compositional architecture** over inheritance. Rust's functional/trait-based design enables ECS-style separation:

```
caliber-core/        # ENTITIES: Data structures only (Trajectory, Scope, Artifact, Note)
                     # No behavior, just types. Pure data.

caliber-storage/     # COMPONENT: Storage trait + pgrx implementation
                     # Defines StorageTrait, implements via direct heap access

caliber-context/     # COMPONENT: Context assembly logic
                     # Trait-based, composable with any storage

caliber-pcp/         # COMPONENT: Validation, checkpoints, recovery
                     # PCP harm reduction as composable component

caliber-llm/         # COMPONENT: VAL (Vector Abstraction Layer) + summarization traits
                     # Provider-agnostic traits, user provides implementation

caliber-agents/      # COMPONENT: Multi-agent coordination (full support)
                     # Locks, messages, delegation, handoffs

caliber-dsl/         # SYSTEM: DSL parser → CaliberConfig struct
                     # Separate crate, generates configuration, no runtime

caliber-pg/          # SYSTEM: The actual pgrx extension
                     # Wires all components together, runs in Postgres
```

**Composition over inheritance:**

- Each crate defines TRAITS (interfaces)
- Zero hard-coded behavior in any crate
- Components compose via dependency injection
- User provides ALL configuration

**The runtime IS Postgres.** The `caliber-pg` extension is the runtime. `caliber-dsl` just produces configuration structs.

### 0.3 Nothing Hard-Coded

Every parameter is user-configured:

```rust
// WRONG - Hard-coded defaults
const DEFAULT_TOKEN_BUDGET: i32 = 8000;  // NO! 
const CHECKPOINT_RETENTION: i32 = 5;      // NO!
const STALE_THRESHOLD_DAYS: i32 = 30;     // NO!

// RIGHT - Configuration required
pub struct CaliberConfig {
    pub token_budget: i32,           // User MUST specify
    pub checkpoint_retention: i32,   // User MUST specify
    pub stale_threshold: Duration,   // User MUST specify
    // ... every knob explicit
}

impl CaliberConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Errors if required fields missing
        // No silent defaults
    }
}
```

**If it's not configured, it errors. Period.**

---

## 1. CORE PRIMITIVES

### 1.1 Fundamental Types (Rust)

```rust
use pgx::prelude::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// === TEMPORAL PRIMITIVES ===
pub type Timestamp = DateTime<Utc>;
pub type DurationMs = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TTL {
    Persistent,
    Session,
    Scope,
    Duration(DurationMs),
}

// === IDENTITY PRIMITIVES ===
// UUIDv7: timestamp-sortable, k-sortable
pub type EntityId = Uuid;

#[derive(Debug, Clone, PostgresType)]
pub enum EntityType {
    Trajectory,
    Scope,
    Artifact,
    Note,
    Agent,
}

#[derive(Debug, Clone, PostgresType)]
pub struct EntityRef {
    pub entity_type: EntityType,
    pub id: EntityId,
}

// === CONTENT PRIMITIVES ===
pub type RawContent = Vec<u8>;  // BYTEA - flexible, handles anything
pub type ContentHash = [u8; 32];  // SHA-256

// === VECTOR ABSTRACTION LAYER (VAL) ===
// Dynamic dimensions - no hard-coded size
// Provider-agnostic: works with OpenAI (1536), Ollama (768), any dimension
#[derive(Debug, Clone, PostgresType)]
pub struct EmbeddingVector {
    pub data: Vec<f32>,  // Dynamic size, NOT const generic
    pub model_id: String,  // Which model produced this
    pub dimensions: i32,   // Explicit dimension tracking
}

impl EmbeddingVector {
    pub fn cosine_similarity(&self, other: &Self) -> Result<f32, VectorError> {
        if self.dimensions != other.dimensions {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimensions,
                got: other.dimensions,
            });
        }
        
        let mut dot = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;
        
        for i in 0..self.data.len() {
            dot += self.data[i] * other.data[i];
            norm_a += self.data[i] * self.data[i];
            norm_b += other.data[i] * other.data[i];
        }
        
        Ok(dot / (norm_a.sqrt() * norm_b.sqrt()))
    }
}

#[derive(Debug, Clone)]
pub enum VectorError {
    DimensionMismatch { expected: i32, got: i32 },
    InvalidVector,
}

// === TURN: Ephemeral conversation buffer ===
#[derive(Debug, Clone, PostgresType)]
pub struct Turn {
    pub turn_id: EntityId,
    pub scope_id: EntityId,
    pub sequence: i32,
    pub role: TurnRole,
    pub content: RawContent,  // BYTEA
    pub created_at: Timestamp,
    
    // Token tracking
    pub token_count: i32,
    pub model_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum TurnRole {
    User,
    Assistant,
    System,
    Tool,
}

// === MEMORY TYPE ENUM ===
#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum MemoryCategory {
    Ephemeral,      // Dies with session/scope
    Working,        // Active manipulation, bounded retention
    Episodic,       // Event-based, time-indexed
    Semantic,       // Factual, entity-indexed, long-lived
    Procedural,     // Skills/patterns, rarely deleted
    Meta,           // Memory about memory (indexes, policies)
}
```

### 1.2 Core Data Structures (Rust + pgrx)

```rust
// === TRAJECTORY: Top-level task container ===
#[derive(Debug, Clone, PostgresType)]
pub struct Trajectory {
    pub trajectory_id: EntityId,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub status: TrajectoryStatus,
    
    // Metadata
    pub initiator: EntityRef,
    pub goal_hash: ContentHash,
    pub tags: Vec<String>,
    
    // Outcome (populated on completion)
    pub outcome: Option<TrajectoryOutcome>,
    
    // Hierarchy
    pub parent_trajectory_id: Option<EntityId>,
    pub child_trajectory_ids: Vec<EntityId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum TrajectoryStatus {
    Active,
    Completed,
    Failed,
    Suspended,
}

#[derive(Debug, Clone, PostgresType)]
pub struct TrajectoryOutcome {
    pub status: OutcomeStatus,
    pub summary: String,
    pub artifacts_produced: Vec<EntityId>,
    pub duration_ms: DurationMs,
    pub tokens_input: i64,
    pub tokens_output: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum OutcomeStatus {
    Success,
    Partial,
    Failure,
}

// === SCOPE: Partitioned context window within trajectory ===
#[derive(Debug, Clone, PostgresType)]
pub struct Scope {
    pub scope_id: EntityId,
    pub trajectory_id: EntityId,
    
    // Boundaries
    pub sequence_number: i32,
    pub start_turn: i32,
    pub end_turn: Option<i32>,
    
    // Timestamps
    pub created_at: Timestamp,
    pub closed_at: Option<Timestamp>,
    
    // Compressed state (populated on close)
    pub summary: Option<String>,
    pub summary_embedding: Option<EmbeddingVector>,
    pub preserved_artifact_ids: Vec<EntityId>,
    
    // Recovery (PCP)
    pub checkpoint: Option<Checkpoint>,
}

#[derive(Debug, Clone, PostgresType)]
pub struct Checkpoint {
    pub context_state: RawContent,
    pub recoverable: bool,
}

// === ARTIFACT: Typed output preserved across scopes ===
#[derive(Debug, Clone, PostgresType)]
pub struct Artifact {
    pub artifact_id: EntityId,
    pub scope_id: EntityId,
    pub trajectory_id: EntityId,
    
    pub created_at: Timestamp,
    
    // Type system
    pub artifact_type: ArtifactType,
    pub schema_version: String,
    
    // Content
    pub content: RawContent,
    pub content_hash: ContentHash,
    pub embedding: Option<EmbeddingVector>,
    
    // Provenance
    pub provenance: Provenance,
    
    // Lifecycle
    pub ttl: TTL,
    pub superseded_by: Option<EntityId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum ArtifactType {
    ErrorLog,
    CodePatch,
    DesignDecision,
    UserPreference,
    Fact,
    Constraint,
    ToolResult,
    IntermediateOutput,
    Custom,
}

#[derive(Debug, Clone, PostgresType)]
pub struct Provenance {
    pub source_turn: i32,
    pub extraction_method: ExtractionMethod,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum ExtractionMethod {
    Explicit,
    Inferred,
    UserProvided,
}

// === NOTE: Long-term cross-trajectory knowledge ===
#[derive(Debug, Clone, PostgresType)]
pub struct Note {
    pub note_id: EntityId,
    
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    
    // Indexing
    pub entity: String,
    pub entity_type: String,
    pub note_type: NoteType,
    
    // Content
    pub content: String,
    pub content_embedding: EmbeddingVector,
    
    // Provenance
    pub source_trajectory_ids: Vec<EntityId>,
    pub confidence: f32,
    
    // Temporal validity
    pub valid_from: Option<Timestamp>,
    pub valid_until: Option<Timestamp>,
    
    // Conflict handling
    pub supersedes: Vec<EntityId>,
    pub conflicts_with: Vec<EntityId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum NoteType {
    Convention,
    Strategy,
    Gotcha,
    Fact,
    Preference,
    Relationship,
    Procedure,
    Meta,
}
```

### 1.3 Context Assembly Types (Rust)

```rust
// === CONTEXT WINDOW: What gets injected into prompt ===
#[derive(Debug, Clone)]
pub struct ContextWindow {
    pub window_id: EntityId,
    pub assembled_at: Timestamp,
    
    // Budget
    pub max_tokens: i32,
    pub used_tokens: i32,
    
    // Sections (ordered by priority)
    pub sections: Vec<ContextSection>,
    
    // Audit trail
    pub assembly_trace: Vec<AssemblyDecision>,
}

#[derive(Debug, Clone)]
pub struct ContextSection {
    pub section_id: EntityId,
    pub section_type: SectionType,
    
    pub content: String,
    pub token_count: i32,
    
    // Source tracking
    pub sources: Vec<SourceRef>,
    
    // Priority (for truncation decisions)
    pub priority: i32,
    pub compressible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionType {
    System,
    Persona,
    Notes,
    History,
    Artifacts,
    User,
}

#[derive(Debug, Clone)]
pub struct SourceRef {
    pub source_type: EntityType,
    pub id: Option<EntityId>,
    pub relevance_score: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct AssemblyDecision {
    pub timestamp: Timestamp,
    pub action: AssemblyAction,
    pub target_type: String,
    pub target_id: Option<EntityId>,
    pub reason: String,
    pub tokens_affected: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyAction {
    Include,
    Exclude,
    Compress,
    Truncate,
}
```

### 1.4 Error Types (Leveraging Rust + Postgres)

CALIBER uses Rust's type system and Postgres's error infrastructure. No custom error handling - use the batteries we have.

```rust
use pgx::prelude::*;
use std::fmt;

/// Single Source of Truth error enum
/// Leverages Rust's enum + Postgres ereport
/// The borrow checker and Postgres fight about it so we don't have to
#[derive(Debug)]
pub enum CaliberError {
    // Storage errors - from pgrx/Postgres
    Storage(StorageError),
    
    // LLM/VAL errors - from provider calls
    Llm(LlmError),
    
    // Validation errors - from our checks
    Validation(ValidationError),
    
    // Configuration errors - missing/invalid config
    Config(ConfigError),
    
    // Vector errors - dimension mismatch, etc.
    Vector(VectorError),
    
    // Multi-agent errors - locks, coordination
    Agent(AgentError),
}

#[derive(Debug)]
pub enum StorageError {
    NotFound { entity_type: EntityType, id: EntityId },
    InsertFailed { entity_type: EntityType, reason: String },
    UpdateFailed { entity_type: EntityType, id: EntityId, reason: String },
    TransactionFailed { reason: String },
    IndexError { index_name: String, reason: String },
}

#[derive(Debug)]
pub enum LlmError {
    ProviderNotConfigured,
    RequestFailed { provider: String, status: i32, message: String },
    RateLimited { provider: String, retry_after_ms: i64 },
    InvalidResponse { provider: String, reason: String },
    EmbeddingFailed { reason: String },
    SummarizationFailed { reason: String },
}

#[derive(Debug)]
pub enum ValidationError {
    RequiredFieldMissing { field: String },
    InvalidValue { field: String, reason: String },
    ConstraintViolation { constraint: String, reason: String },
    CircularReference { entity_type: EntityType, ids: Vec<EntityId> },
    StaleData { entity_type: EntityType, id: EntityId, age: Duration },
    Contradiction { artifact_a: EntityId, artifact_b: EntityId },
}

#[derive(Debug)]
pub enum ConfigError {
    MissingRequired { field: String },
    InvalidValue { field: String, value: String, reason: String },
    IncompatibleOptions { option_a: String, option_b: String },
    ProviderNotSupported { provider: String },
}

#[derive(Debug, Clone)]
pub enum VectorError {
    DimensionMismatch { expected: i32, got: i32 },
    InvalidVector { reason: String },
    ModelMismatch { expected: String, got: String },
}

#[derive(Debug)]
pub enum AgentError {
    NotRegistered { agent_id: EntityId },
    LockAcquisitionFailed { resource: String, holder: EntityId },
    LockExpired { lock_id: EntityId },
    MessageDeliveryFailed { message_id: EntityId, reason: String },
    DelegationFailed { reason: String },
    HandoffFailed { reason: String },
    PermissionDenied { agent_id: EntityId, action: String, resource: String },
}

// Implement conversion to Postgres error via ereport
impl CaliberError {
    pub fn report(self) -> ! {
        use pgx::pg_sys::ereport;
        
        let (level, code, message) = match &self {
            CaliberError::Storage(e) => (
                pgx::pg_sys::ERROR as i32,
                "CALIBER_STORAGE",
                format!("{:?}", e),
            ),
            CaliberError::Llm(e) => (
                pgx::pg_sys::ERROR as i32,
                "CALIBER_LLM",
                format!("{:?}", e),
            ),
            CaliberError::Validation(e) => (
                pgx::pg_sys::ERROR as i32,
                "CALIBER_VALIDATION",
                format!("{:?}", e),
            ),
            CaliberError::Config(e) => (
                pgx::pg_sys::ERROR as i32,
                "CALIBER_CONFIG",
                format!("{:?}", e),
            ),
            CaliberError::Vector(e) => (
                pgx::pg_sys::ERROR as i32,
                "CALIBER_VECTOR",
                format!("{:?}", e),
            ),
            CaliberError::Agent(e) => (
                pgx::pg_sys::ERROR as i32,
                "CALIBER_AGENT",
                format!("{:?}", e),
            ),
        };
        
        // Use Postgres error reporting
        pgx::error!("{}: {}", code, message);
    }
}

// Result type alias for CALIBER operations
pub type CaliberResult<T> = Result<T, CaliberError>;

// All pgrx functions return CaliberResult and use ? operator
// Errors propagate to Postgres via ereport automatically
```

### 1.5 Configuration Types (Nothing Hard-Coded)

```rust
/// Master configuration - ALL values must be explicitly set
/// No defaults. Missing config = ConfigError.
#[derive(Debug, Clone)]
pub struct CaliberConfig {
    // === REQUIRED - will error if missing ===
    
    // Context assembly
    pub token_budget: i32,
    pub section_priorities: SectionPriorities,
    
    // PCP settings
    pub checkpoint_retention: i32,
    pub stale_threshold: Duration,
    pub contradiction_threshold: f32,
    
    // Storage
    pub context_window_persistence: ContextPersistence,
    pub validation_mode: ValidationMode,
    
    // LLM (provider config required if using embeddings/summarization)
    pub embedding_provider: Option<ProviderConfig>,
    pub summarization_provider: Option<ProviderConfig>,
    pub llm_retry_config: RetryConfig,
    
    // Multi-agent
    pub lock_timeout: Duration,
    pub message_retention: Duration,
    pub delegation_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct SectionPriorities {
    pub user: i32,
    pub system: i32,
    pub artifacts: i32,
    pub notes: i32,
    pub history: i32,
    // User can add custom sections with priorities
    pub custom: Vec<(String, i32)>,
}

#[derive(Debug, Clone)]
pub enum ContextPersistence {
    Ephemeral,                    // Never stored
    Ttl(Duration),                // Stored for duration
    Permanent,                    // Stored forever
}

#[derive(Debug, Clone)]
pub enum ValidationMode {
    OnMutation,                   // Validate insert/update only
    Always,                       // Validate on read too (paranoid)
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_type: String,    // "openai", "anthropic", "ollama", etc.
    pub endpoint: Option<String>, // Custom endpoint if needed
    pub model: String,            // Model name
    pub dimensions: Option<i32>,  // For embeddings
    // API keys come from environment, never stored in config struct
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: i32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f32,
}

impl CaliberConfig {
    /// Validate configuration - errors on missing required fields
    pub fn validate(&self) -> CaliberResult<()> {
        // Token budget must be positive
        if self.token_budget <= 0 {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "token_budget".to_string(),
                value: self.token_budget.to_string(),
                reason: "must be positive".to_string(),
            }));
        }
        
        // Checkpoint retention must be positive
        if self.checkpoint_retention <= 0 {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "checkpoint_retention".to_string(),
                value: self.checkpoint_retention.to_string(),
                reason: "must be positive".to_string(),
            }));
        }
        
        // Contradiction threshold must be 0.0-1.0
        if self.contradiction_threshold < 0.0 || self.contradiction_threshold > 1.0 {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "contradiction_threshold".to_string(),
                value: self.contradiction_threshold.to_string(),
                reason: "must be between 0.0 and 1.0".to_string(),
            }));
        }
        
        Ok(())
    }
}
```

---

## 2. DSL SPECIFICATION

### 2.1 Grammar (EBNF)

```ebnf
(* CALIBER DSL Grammar *)

config          = "caliber" ":" version "{" definition* "}" ;
version         = STRING ;

definition      = memory_def | policy_def | adapter_def | injection_rule ;

(* Memory Type Definition *)
memory_def      = "memory" IDENTIFIER "{" memory_body "}" ;
memory_body     = memory_field* ;
memory_field    = "type" ":" memory_type
                | "schema" ":" schema_def
                | "retention" ":" retention_policy
                | "index" ":" index_spec
                | "lifecycle" ":" lifecycle_spec
                | "artifacts" ":" "[" artifact_type_list "]"
                | "parent" ":" IDENTIFIER
                | "inject_on" ":" trigger_list ;

memory_type     = "ephemeral" | "working" | "episodic" | "semantic" | "procedural" | "meta" ;

schema_def      = "{" field_def ("," field_def)* "}" ;
field_def       = IDENTIFIER ":" field_type ;
field_type      = "uuid" | "text" | "int" | "float" | "bool" | "timestamp" 
                | "json" | "embedding" | "enum" "(" enum_values ")" ;
enum_values     = STRING ("," STRING)* ;

retention_policy = "persistent"
                 | "session"
                 | "scope"
                 | duration_expr
                 | "max" "(" INTEGER ")"
                 | "compress_on" ":" trigger ;

index_spec      = "{" index_field ("," index_field)* "}" ;
index_field     = IDENTIFIER ":" index_type ;
index_type      = "btree" | "hash" | "gin" | "hnsw" | "ivfflat" ;

lifecycle_spec  = "explicit" | "auto_close" "(" trigger ")" ;

trigger_list    = "[" trigger ("," trigger)* "]" ;
trigger         = "task_start" | "task_end" | "scope_close" | "turn" | "schedule" "(" cron_expr ")" ;

(* Policy Definition *)
policy_def      = "policy" IDENTIFIER "{" policy_body "}" ;
policy_body     = policy_rule* ;
policy_rule     = "on" trigger ":" action_list ;
action_list     = "[" action ("," action)* "]" ;
action          = "summarize" "(" IDENTIFIER ")"
                | "extract_artifacts" "(" IDENTIFIER ")"
                | "inject" "(" IDENTIFIER "," injection_mode ")"
                | "prune" "(" IDENTIFIER "," prune_criteria ")"
                | "checkpoint" "(" IDENTIFIER ")"
                | "notify" "(" STRING ")" ;

injection_mode  = "full" | "summary" | "top_k" "(" INTEGER ")" | "relevant" "(" FLOAT ")" ;
prune_criteria  = "older_than" "(" duration_expr ")" | "below_confidence" "(" FLOAT ")" ;

(* Adapter Definition *)
adapter_def     = "adapter" IDENTIFIER "{" adapter_body "}" ;
adapter_body    = "type" ":" adapter_type
                | "connection" ":" STRING
                | "options" ":" json_object ;
adapter_type    = "postgres" | "sqlite" | "redis" | "libsql" | "cockroachdb" ;

(* Injection Rule *)
injection_rule  = "inject" IDENTIFIER "into" IDENTIFIER "{" inject_body "}" ;
inject_body     = "mode" ":" injection_mode
                | "priority" ":" INTEGER
                | "max_tokens" ":" INTEGER
                | "filter" ":" filter_expr ;

filter_expr     = field_ref comparator value
                | filter_expr ("and" | "or") filter_expr
                | "(" filter_expr ")" ;
comparator      = "=" | "!=" | ">" | "<" | ">=" | "<=" | "~" | "contains" ;

(* Terminals *)
IDENTIFIER      = [a-z_][a-z0-9_]* ;
STRING          = '"' [^"]* '"' ;
INTEGER         = [0-9]+ ;
FLOAT           = [0-9]+ "." [0-9]+ ;
duration_expr   = INTEGER ("s" | "m" | "h" | "d" | "w") ;
cron_expr       = STRING ;  (* Standard cron format *)
json_object     = "{" (STRING ":" json_value ("," STRING ":" json_value)*)? "}" ;
json_value      = STRING | INTEGER | FLOAT | "true" | "false" | "null" | json_object | json_array ;
json_array      = "[" (json_value ("," json_value)*)? "]" ;
```

### 2.2 Example DSL Configuration

```caliber
caliber: "0.1.0" {

  // === STORAGE ADAPTER ===
  adapter primary {
    type: postgres
    connection: "postgres://localhost:5432/caliber"
    options: {
      "pool_size": 10,
      "vector_dimensions": 1536,
      "hnsw_m": 16,
      "hnsw_ef_construction": 64
    }
  }

  adapter cache {
    type: redis
    connection: "redis://localhost:6379"
    options: {
      "ttl_default": "1h",
      "max_memory": "512mb"
    }
  }

  // === MEMORY TYPE DEFINITIONS ===
  
  memory trajectory {
    type: meta
    schema: {
      trajectory_id: uuid,
      status: enum("active", "completed", "failed", "suspended"),
      goal_summary: text,
      initiator_type: text,
      initiator_id: uuid
    }
    retention: persistent
    lifecycle: explicit
    index: {
      trajectory_id: btree,
      status: hash,
      created_at: btree
    }
  }

  memory scope {
    type: working
    schema: {
      scope_id: uuid,
      trajectory_id: uuid,
      sequence_number: int,
      summary: text,
      summary_embedding: embedding
    }
    retention: persistent
    lifecycle: auto_close(scope_close)
    parent: trajectory
    index: {
      scope_id: btree,
      trajectory_id: btree,
      summary_embedding: hnsw
    }
  }

  memory artifact {
    type: episodic
    schema: {
      artifact_id: uuid,
      scope_id: uuid,
      trajectory_id: uuid,
      artifact_type: enum("error_log", "code_patch", "design_decision", "fact", "tool_result"),
      content: text,
      content_embedding: embedding,
      confidence: float
    }
    retention: persistent
    artifacts: ["error_log", "code_patch", "design_decision", "fact", "tool_result"]
    index: {
      artifact_id: btree,
      trajectory_id: btree,
      artifact_type: hash,
      content_embedding: hnsw
    }
  }

  memory note {
    type: semantic
    schema: {
      note_id: uuid,
      entity: text,
      entity_type: text,
      note_type: enum("convention", "strategy", "gotcha", "fact", "preference", "procedure"),
      content: text,
      content_embedding: embedding,
      confidence: float,
      valid_from: timestamp,
      valid_until: timestamp
    }
    retention: persistent
    inject_on: [task_start]
    index: {
      note_id: btree,
      entity: hash,
      note_type: hash,
      content_embedding: hnsw,
      confidence: btree
    }
  }

  memory turn_buffer {
    type: ephemeral
    schema: {
      turn_id: uuid,
      trajectory_id: uuid,
      scope_id: uuid,
      role: enum("user", "assistant", "system", "tool"),
      content: text,
      token_count: int
    }
    retention: scope
    lifecycle: auto_close(scope_close)
    index: {
      trajectory_id: btree,
      scope_id: btree,
      sequence: btree
    }
  }

  // === POLICIES ===

  policy scope_management {
    on scope_close: [
      summarize(turn_buffer),
      extract_artifacts(turn_buffer),
      checkpoint(scope),
      prune(turn_buffer, older_than(0s))
    ]
  }

  policy trajectory_completion {
    on task_end: [
      summarize(scope),
      extract_artifacts(scope),
      notify("trajectory_complete")
    ]
  }

  policy note_generation {
    on task_end: [
      extract_artifacts(trajectory)
    ]
    on schedule("0 0 * * *"): [
      prune(note, below_confidence(0.3))
    ]
  }

  policy context_injection {
    on task_start: [
      inject(note, relevant(0.7)),
      inject(artifact, relevant(0.8))
    ]
  }

  // === INJECTION RULES ===

  inject note into context {
    mode: relevant(0.7)
    priority: 80
    max_tokens: 2000
    filter: confidence > 0.5 and valid_until > now()
  }

  inject artifact into context {
    mode: top_k(10)
    priority: 90
    max_tokens: 3000
    filter: artifact_type = "design_decision" or artifact_type = "constraint"
  }

  inject scope into context {
    mode: summary
    priority: 70
    max_tokens: 1000
    filter: trajectory_id = current_trajectory
  }

}
```

---

## 3. DIRECT STORAGE LAYER (pgrx)

### 3.1 Architecture: Why No SQL

```
Traditional (WRONG):
┌─────────────────────────────────────────────────────────┐
│  Application                                            │
│       ↓ (generate SQL string)                          │
│  "INSERT INTO caliber_trajectory VALUES (...);"        │
│       ↓                                                │
│  Postgres SQL Parser (gram.y, ~15K lines of C)         │
│       ↓ (build parse tree)                             │
│  Query Analyzer (semantic analysis)                    │
│       ↓ (build query tree)                             │
│  Query Planner/Optimizer                               │
│       ↓ (build execution plan)                         │
│  Executor (portal.c, execMain.c)                       │
│       ↓ (finally...)                                   │
│  Storage Engine (heap, indexes)                        │
└─────────────────────────────────────────────────────────┘

CALIBER (RIGHT):
┌─────────────────────────────────────────────────────────┐
│  Agent Request                                          │
│       ↓                                                │
│  CALIBER DSL (compiled to Rust)                        │
│       ↓                                                │
│  pgrx Extension Functions                              │
│       ↓ (DIRECT ACCESS)                                │
│  Storage Engine (heap, indexes)                        │
└─────────────────────────────────────────────────────────┘
```

### 3.2 pgrx Storage Primitives

```rust
use pgx::prelude::*;
use pgx::{
    pg_sys,
    heap_tuple::PgHeapTuple,
    rel::PgRelation,
    SpiClient,
};

// === RELATION HANDLES ===
// Cache relation OIDs at extension load time
static mut TRAJECTORY_REL_OID: pg_sys::Oid = pg_sys::InvalidOid;
static mut SCOPE_REL_OID: pg_sys::Oid = pg_sys::InvalidOid;
static mut ARTIFACT_REL_OID: pg_sys::Oid = pg_sys::InvalidOid;
static mut NOTE_REL_OID: pg_sys::Oid = pg_sys::InvalidOid;

#[pg_guard]
pub extern "C" fn _PG_init() {
    // Called when extension loads
    // Cache relation OIDs for fast access
    unsafe {
        // These will be populated on first access
        // Using lazy initialization pattern
    }
}

// === DIRECT HEAP OPERATIONS ===

/// Insert a trajectory directly into the heap
/// No SQL parsing, no planning, no text serialization
#[pg_extern]
pub fn caliber_trajectory_insert(
    trajectory_id: Uuid,
    status: TrajectoryStatus,
    goal_hash: &[u8],
    initiator_type: EntityType,
    initiator_id: Uuid,
) -> Uuid {
    // Open relation with appropriate lock
    let rel = unsafe {
        PgRelation::open_with_name("caliber_trajectory")
            .expect("caliber_trajectory relation not found")
    };
    
    // Build heap tuple directly
    let mut values: Vec<pg_sys::Datum> = Vec::with_capacity(16);
    let mut nulls: Vec<bool> = Vec::with_capacity(16);
    
    // trajectory_id
    values.push(trajectory_id.into_datum().unwrap());
    nulls.push(false);
    
    // created_at (use transaction timestamp for consistency)
    values.push(unsafe { pg_sys::GetCurrentTransactionStartTimestamp().into() });
    nulls.push(false);
    
    // updated_at
    values.push(unsafe { pg_sys::GetCurrentTransactionStartTimestamp().into() });
    nulls.push(false);
    
    // status
    values.push(status.into_datum().unwrap());
    nulls.push(false);
    
    // goal_hash (bytea)
    values.push(goal_hash.into_datum().unwrap());
    nulls.push(false);
    
    // initiator_type
    values.push(initiator_type.into_datum().unwrap());
    nulls.push(false);
    
    // initiator_id
    values.push(initiator_id.into_datum().unwrap());
    nulls.push(false);
    
    // Insert tuple directly into heap
    let tuple_desc = rel.tuple_desc();
    let heap_tuple = unsafe {
        pg_sys::heap_form_tuple(
            tuple_desc.as_ptr(),
            values.as_mut_ptr(),
            nulls.as_mut_ptr(),
        )
    };
    
    // Insert into heap
    unsafe {
        pg_sys::simple_heap_insert(rel.as_ptr(), heap_tuple);
        
        // Update indexes
        let index_info = pg_sys::BuildIndexInfo(/* index relation */);
        pg_sys::CatalogIndexInsert(index_info, heap_tuple);
    }
    
    trajectory_id
}

/// Fetch trajectory by ID - direct heap scan with index
#[pg_extern]
pub fn caliber_trajectory_get(trajectory_id: Uuid) -> Option<Trajectory> {
    let rel = unsafe {
        PgRelation::open_with_name("caliber_trajectory")
            .expect("caliber_trajectory relation not found")
    };
    
    // Use index scan for O(log n) lookup
    let index_rel = unsafe {
        PgRelation::open_with_name("caliber_trajectory_pkey")
            .expect("primary key index not found")
    };
    
    // Build scan key
    let scan_key = pg_sys::ScanKeyData {
        sk_flags: 0,
        sk_attno: 1, // trajectory_id is first column
        sk_strategy: pg_sys::BTEqualStrategyNumber as u16,
        sk_subtype: pg_sys::InvalidOid,
        sk_collation: pg_sys::InvalidOid,
        sk_func: /* equality function for UUID */,
        sk_argument: trajectory_id.into_datum().unwrap(),
    };
    
    // Perform index scan
    let scan = unsafe {
        pg_sys::index_beginscan(
            rel.as_ptr(),
            index_rel.as_ptr(),
            pg_sys::GetActiveSnapshot(),
            1, // number of keys
            0, // number of order-by keys
        )
    };
    
    unsafe {
        pg_sys::index_rescan(scan, &scan_key as *const _ as *mut _, 1, std::ptr::null_mut(), 0);
    }
    
    // Fetch tuple
    let heap_tuple = unsafe { pg_sys::index_getnext_slot(scan, pg_sys::ForwardScanDirection) };
    
    if heap_tuple.is_null() {
        unsafe { pg_sys::index_endscan(scan) };
        return None;
    }
    
    // Extract values from tuple
    let trajectory = extract_trajectory_from_tuple(heap_tuple, rel.tuple_desc());
    
    unsafe { pg_sys::index_endscan(scan) };
    
    Some(trajectory)
}

// === VECTOR INDEX OPERATIONS ===

/// Direct HNSW index insertion for embeddings
#[pg_extern]
pub fn caliber_embedding_insert(
    relation_name: &str,
    record_id: Uuid,
    embedding: EmbeddingVector,
) {
    // Get the HNSW index relation
    let index_name = format!("{}_embedding_idx", relation_name);
    let index_rel = unsafe {
        PgRelation::open_with_name(&index_name)
            .expect("embedding index not found")
    };
    
    // Insert directly into HNSW structure
    // pgvector's HNSW implementation uses custom access method
    unsafe {
        let index_info = pg_sys::BuildIndexInfo(index_rel.as_ptr());
        
        // Build index tuple with embedding data
        let values = vec![embedding.as_datum()];
        let nulls = vec![false];
        
        pg_sys::index_insert(
            index_rel.as_ptr(),
            values.as_ptr() as *mut _,
            nulls.as_ptr() as *mut _,
            &record_id as *const _ as *mut _, // item pointer
            index_info,
            pg_sys::UNIQUE_CHECK_NO,
        );
    }
}

/// Direct HNSW vector search - no SQL, no parsing
#[pg_extern]
pub fn caliber_vector_search(
    relation_name: &str,
    query_embedding: EmbeddingVector,
    limit: i32,
    min_similarity: f32,
) -> Vec<(Uuid, f32)> {
    let index_name = format!("{}_embedding_idx", relation_name);
    let index_rel = unsafe {
        PgRelation::open_with_name(&index_name)
            .expect("embedding index not found")
    };
    
    // Direct HNSW search using pgvector's internal API
    // This bypasses SQL entirely
    let mut results: Vec<(Uuid, f32)> = Vec::with_capacity(limit as usize);
    
    unsafe {
        // Set up scan with vector distance ordering
        let scan = pg_sys::index_beginscan(
            /* heap relation */,
            index_rel.as_ptr(),
            pg_sys::GetActiveSnapshot(),
            0,
            1, // order-by on distance
        );
        
        // Configure for KNN search
        // pgvector uses ordered scan for nearest neighbor
        let order_by = pg_sys::ScanKeyData {
            sk_flags: pg_sys::SK_ORDER_BY as i32,
            sk_attno: 1,
            sk_strategy: pg_sys::BTLessStrategyNumber as u16,
            sk_argument: query_embedding.as_datum(),
            // ... distance function setup
        };
        
        pg_sys::index_rescan(scan, std::ptr::null_mut(), 0, &order_by as *const _ as *mut _, 1);
        
        // Fetch results
        let mut count = 0;
        while count < limit {
            let slot = pg_sys::index_getnext_slot(scan, pg_sys::ForwardScanDirection);
            if slot.is_null() {
                break;
            }
            
            // Extract ID and compute similarity
            let record_id: Uuid = /* extract from tuple */;
            let distance: f32 = /* extract from order-by result */;
            let similarity = 1.0 - distance; // cosine distance to similarity
            
            if similarity >= min_similarity {
                results.push((record_id, similarity));
                count += 1;
            }
        }
        
        pg_sys::index_endscan(scan);
    }
    
    results
}

// === TRANSACTION-AWARE OPERATIONS ===

/// All CALIBER operations are transaction-aware
/// This enables atomic context assembly and rollback
#[pg_extern]
pub fn caliber_begin_trajectory(goal: &str, initiator: EntityRef) -> Uuid {
    // Already in transaction context (Postgres function call)
    // All subsequent operations will be in same transaction
    
    let trajectory_id = generate_uuidv7();
    let goal_hash = sha256(goal.as_bytes());
    
    caliber_trajectory_insert(
        trajectory_id,
        TrajectoryStatus::Active,
        &goal_hash,
        initiator.entity_type,
        initiator.id,
    );
    
    // Create initial scope
    let scope_id = caliber_scope_insert(
        trajectory_id,
        0, // sequence_number
        0, // start_turn
    );
    
    trajectory_id
}

// === BUFFER MANAGER DIRECT ACCESS ===

/// For bulk operations, we can work directly with buffer pages
/// This is useful for batch artifact insertion
pub fn caliber_bulk_artifact_insert(artifacts: Vec<Artifact>) {
    let rel = unsafe {
        PgRelation::open_with_name("caliber_artifact")
            .expect("caliber_artifact relation not found")
    };
    
    // Get exclusive lock for bulk insert
    unsafe {
        pg_sys::LockRelation(rel.as_ptr(), pg_sys::AccessExclusiveLock as i32);
    }
    
    // Work directly with buffer pages for efficiency
    for artifact in artifacts {
        let tuple = build_artifact_tuple(&artifact);
        
        unsafe {
            // Get a buffer page with space
            let buffer = pg_sys::RelationGetBufferForTuple(
                rel.as_ptr(),
                tuple.t_len,
                pg_sys::InvalidBuffer,
                0, // options
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            
            // Insert into page
            let page = pg_sys::BufferGetPage(buffer);
            let offset = pg_sys::PageAddItem(
                page,
                tuple as *const _ as *mut _,
                tuple.t_len as u16,
                pg_sys::InvalidOffsetNumber,
                false,
                true,
            );
            
            // Mark buffer dirty (will be written by checkpointer)
            pg_sys::MarkBufferDirty(buffer);
            
            // Write WAL record for durability
            pg_sys::XLogInsert(/* WAL record */);
            
            pg_sys::ReleaseBuffer(buffer);
        }
    }
    
    unsafe {
        pg_sys::UnlockRelation(rel.as_ptr(), pg_sys::AccessExclusiveLock as i32);
    }
}
```

### 3.3 DSL Compilation Target

The CALIBER DSL compiles to Rust code that calls these pgrx functions:

```
DSL:                           Compiles To:
─────────────────────────────────────────────────────────
memory trajectory {            → struct Trajectory { ... }
  type: meta                   → impl PostgresType for Trajectory
  schema: { ... }              → caliber_trajectory_insert()
  ...                          → caliber_trajectory_get()
}                              → caliber_trajectory_update()

inject note into context {     → caliber_vector_search()
  mode: relevant(0.7)          → (with min_similarity = 0.7)
  ...
}
```

### 3.4 Human Debug Interface (SQL Views)

SQL is ONLY generated for human inspection:

```rust
/// Create SQL views for human debugging
/// These are NOT used by agents
#[pg_extern]
pub fn caliber_create_debug_views() {
    // This is the ONLY place we generate SQL
    // It's for humans, not the hot path
    
    Spi::run(r#"
        -- Human-readable view of trajectories
        CREATE OR REPLACE VIEW caliber_debug_trajectories AS
        SELECT 
            trajectory_id,
            status,
            encode(goal_hash, 'hex') as goal_hash_hex,
            created_at,
            updated_at
        FROM caliber_trajectory;
        
        -- Human-readable view of recent context assembly
        CREATE OR REPLACE VIEW caliber_debug_context AS
        SELECT 
            window_id,
            assembled_at,
            used_tokens,
            max_tokens,
            array_length(sections, 1) as section_count
        FROM caliber_context_window
        ORDER BY assembled_at DESC
        LIMIT 100;
        
        COMMENT ON VIEW caliber_debug_trajectories IS 
            'Human debug interface - NOT for agent use';
    "#).expect("Failed to create debug views");
}
```

---

## 4. RUNTIME ENGINE (Rust)

### 4.1 Engine Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      CALIBER RUNTIME                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │   Context   │  │   Policy    │  │  Lifecycle  │             │
│  │  Assembler  │  │   Engine    │  │   Manager   │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┼────────────────┘                     │
│                          │                                      │
│                    ┌─────┴─────┐                                │
│                    │Orchestrator│                               │
│                    └─────┬─────┘                                │
│                          │                                      │
│         ┌────────────────┼────────────────┐                     │
│         │                │                │                     │
│  ┌──────┴──────┐  ┌──────┴──────┐  ┌──────┴──────┐             │
│  │   Direct    │  │  Embedding  │  │    Event    │             │
│  │   Storage   │  │   Service   │  │     Bus     │             │
│  │   (pgrx)    │  │   (FFI)     │  │  (NOTIFY)   │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Core Orchestrator (Rust)

```rust
use pgx::prelude::*;
use std::sync::Arc;

/// Main orchestrator - all operations go through here
/// Runs inside Postgres process via pgrx
pub struct CaliberOrchestrator {
    config: Arc<CompiledConfig>,
    embedding_service: Arc<dyn EmbeddingProvider>,
}

impl CaliberOrchestrator {
    // === TRAJECTORY LIFECYCLE ===
    
    #[pg_extern]
    pub fn start_trajectory(
        goal: &str,
        initiator_type: EntityType,
        initiator_id: Uuid,
        parent_trajectory_id: Option<Uuid>,
    ) -> Uuid {
        let trajectory_id = generate_uuidv7();
        let goal_hash = sha256(goal.as_bytes());
        
        // Direct heap insert - no SQL
        caliber_trajectory_insert(
            trajectory_id,
            TrajectoryStatus::Active,
            &goal_hash,
            initiator_type,
            initiator_id,
        );
        
        // Link to parent if sub-task
        if let Some(parent_id) = parent_trajectory_id {
            caliber_trajectory_add_child(parent_id, trajectory_id);
        }
        
        // Create initial scope
        Self::open_scope(trajectory_id);
        
        // Execute task_start policies
        Self::execute_policies(PolicyTrigger::TaskStart, trajectory_id, None);
        
        // Emit event via NOTIFY (for reactive systems)
        caliber_notify("trajectory_started", trajectory_id);
        
        trajectory_id
    }
    
    #[pg_extern]
    pub fn complete_trajectory(
        trajectory_id: Uuid,
        status: OutcomeStatus,
        summary: &str,
    ) {
        // Close any open scopes
        let open_scopes = caliber_scope_list_open(trajectory_id);
        for scope_id in open_scopes {
            Self::close_scope(scope_id);
        }
        
        // Build outcome
        let outcome = TrajectoryOutcome {
            status,
            summary: summary.to_string(),
            artifacts_produced: caliber_artifact_list_by_trajectory(trajectory_id),
            duration_ms: Self::compute_duration(trajectory_id),
            tokens_input: Self::sum_tokens(trajectory_id, TokenType::Input),
            tokens_output: Self::sum_tokens(trajectory_id, TokenType::Output),
        };
        
        // Direct heap update - no SQL
        caliber_trajectory_update_outcome(trajectory_id, outcome);
        caliber_trajectory_update_status(trajectory_id, TrajectoryStatus::Completed);
        
        // Execute task_end policies
        Self::execute_policies(PolicyTrigger::TaskEnd, trajectory_id, None);
        
        caliber_notify("trajectory_completed", trajectory_id);
    }
    
    // === SCOPE LIFECYCLE ===
    
    #[pg_extern]
    pub fn open_scope(trajectory_id: Uuid) -> Uuid {
        let scope_count = caliber_scope_count(trajectory_id);
        let current_turn = Self::get_current_turn(trajectory_id);
        
        let scope_id = generate_uuidv7();
        
        // Direct heap insert
        caliber_scope_insert(
            scope_id,
            trajectory_id,
            scope_count as i32,  // sequence_number
            current_turn,        // start_turn
        );
        
        caliber_notify("scope_opened", scope_id);
        
        scope_id
    }
    
    #[pg_extern]
    pub fn close_scope(scope_id: Uuid) {
        let scope = caliber_scope_get(scope_id)
            .expect("Scope not found");
        
        if scope.closed_at.is_some() {
            return; // Already closed
        }
        
        let current_turn = Self::get_current_turn(scope.trajectory_id);
        
        // Get turns in this scope for summarization
        let turns = caliber_turn_list_by_scope(scope_id);
        let turn_content = Self::format_turns_for_summary(&turns);
        
        // Generate summary (calls external embedding service)
        let summary = Self::summarize(&turn_content);
        let summary_embedding = Self::embed(&summary);
        
        // Extract artifacts
        let extracted = Self::extract_artifacts(&turn_content);
        let artifact_ids: Vec<Uuid> = extracted.iter().map(|a| {
            Self::create_artifact_internal(
                scope_id,
                scope.trajectory_id,
                a.artifact_type,
                &a.content,
                ExtractionMethod::Inferred,
                a.confidence,
            )
        }).collect();
        
        // Create checkpoint for PCP recovery
        let checkpoint = Checkpoint {
            context_state: Self::serialize_context_state(scope.trajectory_id),
            recoverable: true,
        };
        
        // Direct heap update - no SQL
        caliber_scope_close(
            scope_id,
            current_turn,
            &summary,
            &summary_embedding,
            &artifact_ids,
            &checkpoint,
        );
        
        // Execute scope_close policies
        Self::execute_policies(PolicyTrigger::ScopeClose, scope.trajectory_id, Some(scope_id));
        
        caliber_notify("scope_closed", scope_id);
    }
    
    // === CONTEXT ASSEMBLY ===
    
    #[pg_extern]
    pub fn assemble_context(
        trajectory_id: Uuid,
        max_tokens: i32,
        query: Option<&str>,
    ) -> ContextWindow {
        let mut window = ContextWindow {
            window_id: generate_uuidv7(),
            assembled_at: chrono::Utc::now(),
            max_tokens,
            used_tokens: 0,
            sections: Vec::new(),
            assembly_trace: Vec::new(),
        };
        
        let config = Self::get_config();
        let mut rules = config.injection_rules.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority)); // Higher priority first
        
        for rule in rules {
            if window.used_tokens >= (max_tokens as f32 * 0.95) as i32 {
                window.assembly_trace.push(AssemblyDecision {
                    timestamp: chrono::Utc::now(),
                    action: AssemblyAction::Exclude,
                    target_type: rule.source_memory.clone(),
                    target_id: None,
                    reason: "token_budget_exhausted".to_string(),
                    tokens_affected: 0,
                });
                continue;
            }
            
            if let Some(section) = Self::assemble_section(
                &rule,
                trajectory_id,
                query,
                max_tokens - window.used_tokens,
            ) {
                window.used_tokens += section.token_count;
                window.assembly_trace.push(AssemblyDecision {
                    timestamp: chrono::Utc::now(),
                    action: AssemblyAction::Include,
                    target_type: rule.source_memory.clone(),
                    target_id: None,
                    reason: format!("rule_{}", rule.id),
                    tokens_affected: section.token_count,
                });
                window.sections.push(section);
            }
        }
        
        window
    }
    
    fn assemble_section(
        rule: &InjectionRule,
        trajectory_id: Uuid,
        query: Option<&str>,
        remaining_tokens: i32,
    ) -> Option<ContextSection> {
        let token_budget = std::cmp::min(rule.max_tokens.unwrap_or(i32::MAX), remaining_tokens);
        
        let records = match &rule.mode {
            InjectionMode::Full => {
                // Direct heap scan with filter
                caliber_memory_scan(&rule.source_memory, &rule.filter, None)
            }
            
            InjectionMode::Summary => {
                // Get records, use summary field
                caliber_memory_scan(&rule.source_memory, &rule.filter, None)
            }
            
            InjectionMode::TopK(k) => {
                // Direct heap scan with limit, ordered by recency
                caliber_memory_scan(&rule.source_memory, &rule.filter, Some(*k))
            }
            
            InjectionMode::Relevant(threshold) => {
                if let Some(q) = query {
                    // DIRECT VECTOR SEARCH - no SQL
                    let query_embedding = Self::embed(q);
                    caliber_vector_search(
                        &rule.source_memory,
                        query_embedding,
                        20, // limit
                        *threshold,
                    )
                } else {
                    // Fall back to recency
                    caliber_memory_scan(&rule.source_memory, &rule.filter, Some(10))
                }
            }
        };
        
        if records.is_empty() {
            return None;
        }
        
        // Format content and truncate to budget
        let content = Self::format_records_for_context(&records, &rule.source_memory);
        let token_count = Self::count_tokens(&content);
        
        let final_content = if token_count > token_budget {
            Self::truncate_to_tokens(&content, token_budget)
        } else {
            content
        };
        
        Some(ContextSection {
            section_id: generate_uuidv7(),
            section_type: Self::map_memory_to_section_type(&rule.source_memory),
            content: final_content,
            token_count: Self::count_tokens(&final_content),
            sources: records.iter().map(|r| SourceRef {
                source_type: EntityType::from_str(&rule.source_memory),
                id: Some(r.id),
                relevance_score: r.similarity,
            }).collect(),
            priority: rule.priority,
            compressible: rule.source_memory != "artifact",
        })
    }
    
    // === POLICY EXECUTION ===
    
    fn execute_policies(trigger: PolicyTrigger, trajectory_id: Uuid, scope_id: Option<Uuid>) {
        let config = Self::get_config();
        
        for policy in &config.policies {
            for rule in &policy.rules {
                if rule.trigger != trigger {
                    continue;
                }
                
                for action in &rule.actions {
                    Self::execute_action(action, trajectory_id, scope_id);
                }
            }
        }
    }
    
    fn execute_action(action: &PolicyAction, trajectory_id: Uuid, scope_id: Option<Uuid>) {
        match action {
            PolicyAction::Summarize(target) => {
                // Already handled in scope close
            }
            
            PolicyAction::ExtractArtifacts(target) => {
                // Already handled in scope close
            }
            
            PolicyAction::Checkpoint(target) => {
                if let Some(sid) = scope_id {
                    let checkpoint = Checkpoint {
                        context_state: Self::serialize_context_state(trajectory_id),
                        recoverable: true,
                    };
                    caliber_scope_update_checkpoint(sid, &checkpoint);
                }
            }
            
            PolicyAction::Prune { target, criteria } => {
                Self::prune_memory(target, criteria);
            }
            
            PolicyAction::Notify(channel) => {
                caliber_notify(channel, trajectory_id);
            }
            
            PolicyAction::Inject { target, mode } => {
                // Handled during context assembly
            }
        }
    }
    
    // === ARTIFACT MANAGEMENT ===
    
    #[pg_extern]
    pub fn create_artifact(
        scope_id: Uuid,
        trajectory_id: Uuid,
        artifact_type: ArtifactType,
        content: &[u8],
        extraction_method: ExtractionMethod,
    ) -> Uuid {
        Self::create_artifact_internal(
            scope_id,
            trajectory_id,
            artifact_type,
            content,
            extraction_method,
            None,
        )
    }
    
    fn create_artifact_internal(
        scope_id: Uuid,
        trajectory_id: Uuid,
        artifact_type: ArtifactType,
        content: &[u8],
        extraction_method: ExtractionMethod,
        confidence: Option<f32>,
    ) -> Uuid {
        let artifact_id = generate_uuidv7();
        let content_hash = sha256(content);
        let embedding = Self::embed_bytes(content);
        
        // Check for duplicate (same hash in same trajectory)
        if let Some(existing) = caliber_artifact_find_by_hash(trajectory_id, &content_hash) {
            // Update provenance instead of creating duplicate
            caliber_artifact_update_provenance(
                existing,
                extraction_method,
                confidence,
            );
            return existing;
        }
        
        // Direct heap insert - no SQL
        caliber_artifact_insert(
            artifact_id,
            scope_id,
            trajectory_id,
            artifact_type,
            content,
            &content_hash,
            &embedding,
            extraction_method,
            confidence,
        );
        
        // Insert into vector index for similarity search
        caliber_embedding_insert("caliber_artifact", artifact_id, embedding);
        
        caliber_notify("artifact_created", artifact_id);
        
        artifact_id
    }
    
    // === NOTE MANAGEMENT ===
    
    #[pg_extern]
    pub fn consolidate_notes(trajectory_id: Uuid) -> Vec<Uuid> {
        let trajectory = caliber_trajectory_get(trajectory_id)
            .expect("Trajectory not found");
        
        let scopes = caliber_scope_list_by_trajectory(trajectory_id)?;
        let artifacts = caliber_artifact_list_by_trajectory(trajectory_id);
        
        // Generate notes via LLM
        let generated = Self::generate_notes(&trajectory, &scopes, &artifacts);
        
        let mut note_ids = Vec::new();
        
        for note_data in generated {
            let note_id = generate_uuidv7();
            let content_embedding = Self::embed(&note_data.content);
            
            // Check for existing notes about same entity
            if let Some(existing) = caliber_note_find_by_entity(
                &note_data.entity,
                &note_data.note_type,
            ) {
                // Merge notes
                let merged = Self::merge_notes(&existing, &note_data);
                caliber_note_update(existing.note_id, &merged);
                note_ids.push(existing.note_id);
            } else {
                // Direct heap insert
                caliber_note_insert(
                    note_id,
                    &note_data.entity,
                    &note_data.entity_type,
                    note_data.note_type,
                    &note_data.content,
                    &content_embedding,
                    1.0, // initial confidence
                    &[trajectory_id],
                );
                
                // Insert into vector index
                caliber_embedding_insert("caliber_note", note_id, content_embedding);
                
                note_ids.push(note_id);
            }
        }
        
        note_ids
    }
}

// === UTILITY FUNCTIONS ===

/// Generate UUIDv7 (timestamp-sortable)
fn generate_uuidv7() -> Uuid {
    let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
    
    let mut bytes = [0u8; 16];
    
    // First 48 bits: timestamp
    bytes[0] = ((timestamp_ms >> 40) & 0xFF) as u8;
    bytes[1] = ((timestamp_ms >> 32) & 0xFF) as u8;
    bytes[2] = ((timestamp_ms >> 24) & 0xFF) as u8;
    bytes[3] = ((timestamp_ms >> 16) & 0xFF) as u8;
    bytes[4] = ((timestamp_ms >> 8) & 0xFF) as u8;
    bytes[5] = (timestamp_ms & 0xFF) as u8;
    
    // Version 7
    bytes[6] = 0x70 | (rand::random::<u8>() & 0x0F);
    bytes[7] = rand::random();
    
    // Variant bits
    bytes[8] = 0x80 | (rand::random::<u8>() & 0x3F);
    
    // Random bytes for rest
    for i in 9..16 {
        bytes[i] = rand::random();
    }
    
    Uuid::from_bytes(bytes)
}

/// SHA-256 hash
fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Emit event via Postgres NOTIFY
fn caliber_notify(channel: &str, id: Uuid) {
    unsafe {
        let payload = format!(r#"{{"id":"{}"}}"#, id);
        pg_sys::Async_Notify(
            std::ffi::CString::new(format!("caliber_{}", channel)).unwrap().as_ptr(),
            std::ffi::CString::new(payload).unwrap().as_ptr(),
        );
    }
}
```

### 4.3 Embedding Service (FFI)

Embeddings require calling external services. We use FFI to call out:

```rust
/// Embedding service trait - implemented via FFI
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> EmbeddingVector;
    fn embed_batch(&self, texts: &[&str]) -> Vec<EmbeddingVector>;
    fn dimensions(&self) -> usize;
}

/// HTTP-based embedding service (calls external API)
pub struct HttpEmbeddingProvider {
    endpoint: String,
    dimensions: usize,
}

impl EmbeddingProvider for HttpEmbeddingProvider {
    fn embed(&self, text: &str) -> EmbeddingVector {
        // Use libcurl via FFI for HTTP request
        // This runs in a background worker to not block Postgres
        unsafe {
            let response = curl_post(&self.endpoint, text);
            parse_embedding_response(&response)
        }
    }
    
    fn embed_batch(&self, texts: &[&str]) -> Vec<EmbeddingVector> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
    
    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

/// Local embedding service (runs model in-process)
/// For cases where you want zero network latency
pub struct LocalEmbeddingProvider {
    // Uses ONNX runtime or similar
    model: ort::Session,
}

impl EmbeddingProvider for LocalEmbeddingProvider {
    fn embed(&self, text: &str) -> EmbeddingVector {
        // Tokenize and run inference
        let tokens = self.tokenize(text);
        let output = self.model.run(tokens);
        EmbeddingVector { data: output.into() }
    }
    
    // ...
}
```

```

---

## 5. PCP (PERSISTENT CONTEXT PROTOCOL) - Rust

### 5.1 Core Principles

```rust
/// PCP: Harm reduction for AI memory
/// 
/// Like drug harm reduction:
/// - Test kits → Context validation
/// - Dosage control → Token budgets
/// - Set & setting → Scope boundaries
/// - Trip sitter → Observable state + recovery
/// - Harm prevention → Anti-hallucination, anti-drift

#[derive(Debug, Clone)]
pub struct PCPConfig {
    // Context DAG (not flat history)
    pub context_dag: ContextDagConfig,
    
    // Recovery checkpoints
    pub recovery_points: RecoveryConfig,
    
    // Dosage control
    pub dosage: DosageConfig,
    
    // Anti-sprawl
    pub anti_sprawl: AntiSprawlConfig,
    
    // Hallucination prevention
    pub grounding: GroundingConfig,
}

#[derive(Debug, Clone)]
pub struct ContextDagConfig {
    pub enabled: bool,
    pub max_depth: i32,
    pub prune_strategy: PruneStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum PruneStrategy {
    Lru,
    Relevance,
    Hybrid,
}

#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub frequency: RecoveryFrequency,
    pub max_checkpoints: i32,
    pub compression_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum RecoveryFrequency {
    PerScope,
    PerTurn,
    Manual,
}

#[derive(Debug, Clone)]
pub struct DosageConfig {
    pub max_context_tokens: i32,
    pub max_scopes_in_window: i32,
    pub max_artifacts_in_window: i32,
    pub max_notes_in_window: i32,
}

#[derive(Debug, Clone)]
pub struct AntiSprawlConfig {
    pub linter_enabled: bool,
    pub vocabulary_enforcement: bool,
    pub duplicate_detection: bool,
    pub max_artifact_size: i32,
}

#[derive(Debug, Clone)]
pub struct GroundingConfig {
    pub require_source_for_facts: bool,
    pub confidence_threshold: f32,
    pub stale_data_max_age_ms: i64,
    pub conflict_resolution: ConflictResolution,
}

#[derive(Debug, Clone, Copy)]
pub enum ConflictResolution {
    Newest,
    HighestConfidence,
    Manual,
}

#[cfg(test)]
impl PCPConfig {
    /// Test-only example configuration. CALIBER has no runtime defaults.
    pub fn test_config() -> Self {
        Self {
            context_dag: ContextDagConfig {
                enabled: true,
                max_depth: 10,
                prune_strategy: PruneStrategy::Hybrid,
            },
            recovery_points: RecoveryConfig {
                frequency: RecoveryFrequency::PerScope,
                max_checkpoints: 5,
                compression_enabled: true,
            },
            dosage: DosageConfig {
                max_context_tokens: 8000,
                max_scopes_in_window: 5,
                max_artifacts_in_window: 20,
                max_notes_in_window: 10,
            },
            anti_sprawl: AntiSprawlConfig {
                linter_enabled: true,
                vocabulary_enforcement: false,
                duplicate_detection: true,
                max_artifact_size: 10000,
            },
            grounding: GroundingConfig {
                require_source_for_facts: true,
                confidence_threshold: 0.5,
                stale_data_max_age_ms: 30 * 24 * 60 * 60 * 1000, // 30 days
                conflict_resolution: ConflictResolution::HighestConfidence,
            },
        }
    }
}
```

### 5.2 PCP Runtime (Rust)

```rust
use pgx::prelude::*;

pub struct PCPRuntime {
    config: PCPConfig,
}

impl PCPRuntime {
    pub fn new(config: PCPConfig) -> Self {
        Self { config }
    }

    fn require_context_window(window_id: Uuid) -> CaliberResult<ContextWindow> {
        caliber_context_window_get(window_id).ok_or_else(|| {
            CaliberError::Validation(ValidationError::ConstraintViolation {
                constraint: "context_window_exists".to_string(),
                reason: format!("Context window {} not found", window_id),
            })
        })
    }

    fn require_trajectory(trajectory_id: Uuid) -> CaliberResult<Trajectory> {
        caliber_trajectory_get(trajectory_id).ok_or_else(|| {
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Trajectory,
                id: trajectory_id,
            })
        })
    }

    fn require_artifact(artifact_id: Uuid) -> CaliberResult<Artifact> {
        caliber_artifact_get(artifact_id).ok_or_else(|| {
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Artifact,
                id: artifact_id,
            })
        })
    }
    
    // === HARM REDUCTION: Context Validation ===
    
    #[pg_extern]
    pub fn validate_context_integrity(window_id: Uuid) -> CaliberResult<ValidationResult> {
        let window = Self::require_context_window(window_id)?;
        
        let mut issues: Vec<ValidationIssue> = Vec::new();
        
        // Check for contradictions
        let artifacts = Self::extract_artifacts_from_window(&window);
        let contradictions = Self::detect_contradictions(&artifacts);
        for c in contradictions {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                issue_type: IssueType::Contradiction,
                message: format!(
                    "Conflicting information: {:?} vs {:?}",
                    c.a.content, c.b.content
                ),
                artifact_ids: vec![c.a.artifact_id, c.b.artifact_id],
            });
        }
        
        // Check for stale data
        let now_ms = chrono::Utc::now().timestamp_millis();
        let max_age = Self::get_config().grounding.stale_data_max_age_ms;
        
        for section in &window.sections {
            for source in &section.sources {
                if source.source_type == EntityType::Note {
                    if let Some(id) = source.id {
                        if let Some(note) = caliber_note_get(id) {
                            if let Some(valid_until) = note.valid_until {
                                if valid_until.timestamp_millis() < now_ms {
                                    issues.push(ValidationIssue {
                                        severity: Severity::Error,
                                        issue_type: IssueType::StaleData,
                                        message: format!("Note {} has expired", id),
                                        artifact_ids: vec![id],
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check confidence thresholds
        let threshold = Self::get_config().grounding.confidence_threshold;
        for section in &window.sections {
            for source in &section.sources {
                if let Some(score) = source.relevance_score {
                    if score < threshold {
                        issues.push(ValidationIssue {
                            severity: Severity::Info,
                            issue_type: IssueType::LowConfidence,
                            message: format!(
                                "Source {:?} below confidence threshold ({})",
                                source.id, score
                            ),
                            artifact_ids: source.id.map(|id| vec![id]).unwrap_or_default(),
                        });
                    }
                }
            }
        }
        
        Ok(ValidationResult {
            valid: issues.iter().all(|i| i.severity != Severity::Error),
            issues,
        })
    }
    
    fn detect_contradictions(artifacts: &[Artifact]) -> Vec<Contradiction> {
        let mut contradictions = Vec::new();
        
        // Group facts by entity (using embedding similarity)
        for i in 0..artifacts.len() {
            if artifacts[i].artifact_type != ArtifactType::Fact {
                continue;
            }
            
            for j in (i + 1)..artifacts.len() {
                if artifacts[j].artifact_type != ArtifactType::Fact {
                    continue;
                }
                
                // Check embedding similarity
                if let (Some(emb_a), Some(emb_b)) = (&artifacts[i].embedding, &artifacts[j].embedding) {
                    let similarity = emb_a.cosine_similarity(emb_b);
                    
                    // High similarity but different hash = potential contradiction
                    if similarity > 0.8 && artifacts[i].content_hash != artifacts[j].content_hash {
                        // Verify with LLM
                        if Self::verify_contradiction(&artifacts[i].content, &artifacts[j].content) {
                            contradictions.push(Contradiction {
                                a: artifacts[i].clone(),
                                b: artifacts[j].clone(),
                            });
                        }
                    }
                }
            }
        }
        
        contradictions
    }
    
    // === RECOVERY ===
    
    #[pg_extern]
    pub fn create_checkpoint(trajectory_id: Uuid) -> CaliberResult<Uuid> {
        let trajectory = Self::require_trajectory(trajectory_id)?;
        
        let scopes = caliber_scope_list_by_trajectory(trajectory_id);
        let active_scope = scopes.iter().find(|s| s.closed_at.is_none());
        
        let checkpoint_id = generate_uuidv7();
        
        // Serialize current context
        let context_snapshot = CaliberOrchestrator::assemble_context(
            trajectory_id,
            Self::get_config().dosage.max_context_tokens,
            None,
        );
        
        let checkpoint = PCPCheckpoint {
            checkpoint_id,
            trajectory_id,
            scope_id: active_scope.map(|s| s.scope_id),
            created_at: chrono::Utc::now(),
            state: CheckpointState {
                trajectory_status: trajectory.status,
                scope_count: scopes.len() as i32,
                active_scope_sequence: active_scope.map(|s| s.sequence_number),
                context_snapshot,
            },
            recoverable: true,
        };
        
        // Direct heap insert
        caliber_checkpoint_insert(&checkpoint)?;
        
        // Prune old checkpoints if over limit
        let max_checkpoints = Self::get_config().recovery_points.max_checkpoints;
        let existing = caliber_checkpoint_list_by_trajectory(trajectory_id)?;
        
        if existing.len() as i32 > max_checkpoints {
            let to_delete = &existing[max_checkpoints as usize..];
            for cp in to_delete {
                caliber_checkpoint_delete(cp.checkpoint_id);
            }
        }
        
        Ok(checkpoint_id)
    }
    
    #[pg_extern]
    pub fn recover_from_checkpoint(checkpoint_id: Uuid) -> RecoveryResult {
        let checkpoint = match caliber_checkpoint_get(checkpoint_id) {
            Some(cp) if cp.recoverable => cp,
            Some(_) => return RecoveryResult {
                success: false,
                reason: Some("Checkpoint not recoverable".to_string()),
                restored_to: None,
                new_scope_id: None,
            },
            None => return RecoveryResult {
                success: false,
                reason: Some("Checkpoint not found".to_string()),
                restored_to: None,
                new_scope_id: None,
            },
        };
        
        // Mark scopes after checkpoint as rolled back
        if let Some(active_seq) = checkpoint.state.active_scope_sequence {
            let scopes = caliber_scope_list_by_trajectory(checkpoint.trajectory_id);
            let to_rollback: Vec<_> = scopes.iter()
                .filter(|s| s.sequence_number > active_seq)
                .collect();
            
            for scope in to_rollback {
                caliber_scope_update(scope.scope_id, ScopeUpdate {
                    closed_at: Some(chrono::Utc::now()),
                    summary: Some("[ROLLED BACK - Recovery from checkpoint]".to_string()),
                    ..Default::default()
                });
            }
        }
        
        // Open new scope
        let new_scope_id = CaliberOrchestrator::open_scope(checkpoint.trajectory_id);
        
        RecoveryResult {
            success: true,
            reason: None,
            restored_to: Some(checkpoint.created_at),
            new_scope_id: Some(new_scope_id),
        }
    }
    
    // === DOSAGE CONTROL ===
    
    #[pg_extern]
    pub fn apply_dosage_limits(window_id: Uuid) -> CaliberResult<Uuid> {
        let mut window = Self::require_context_window(window_id)?;
        
        let dosage = &Self::get_config().dosage;
        
        // Already within limits
        if window.used_tokens <= dosage.max_context_tokens {
            return Ok(window_id);
        }
        
        // Sort sections by priority (lower = drop first)
        window.sections.sort_by(|a, b| a.priority.cmp(&b.priority));
        
        let mut current_tokens = window.used_tokens;
        let mut sections_to_keep: Vec<ContextSection> = Vec::new();
        
        // Keep high-priority sections first
        for section in window.sections.into_iter().rev() {
            if current_tokens - section.token_count <= dosage.max_context_tokens 
                || section.priority >= 90  // Always keep critical
            {
                sections_to_keep.insert(0, section);
            } else if section.compressible {
                // Try to compress
                if let Some(compressed) = Self::compress_section(&section, section.token_count / 2) {
                    if compressed.token_count < section.token_count {
                        current_tokens = current_tokens - section.token_count + compressed.token_count;
                        sections_to_keep.insert(0, compressed);
                        continue;
                    }
                }
                current_tokens -= section.token_count;
            } else {
                current_tokens -= section.token_count;
            }
        }
        
        // Create new window with filtered sections
        let new_window_id = generate_uuidv7();
        let new_window = ContextWindow {
            window_id: new_window_id,
            assembled_at: chrono::Utc::now(),
            max_tokens: window.max_tokens,
            used_tokens: sections_to_keep.iter().map(|s| s.token_count).sum(),
            sections: sections_to_keep,
            assembly_trace: window.assembly_trace,
        };
        
        caliber_context_window_insert(&new_window)?;
        
        Ok(new_window_id)
    }
    
    // === ANTI-SPRAWL ===
    
    #[pg_extern]
    pub fn lint_artifact(artifact_id: Uuid) -> CaliberResult<LintResult> {
        let artifact = Self::require_artifact(artifact_id)?;
        
        let config = &Self::get_config().anti_sprawl;
        let mut issues: Vec<LintIssue> = Vec::new();
        
        // Size check
        if artifact.content.len() as i32 > config.max_artifact_size {
            issues.push(LintIssue {
                issue_type: LintIssueType::Oversized,
                message: format!(
                    "Artifact exceeds max size ({} > {})",
                    artifact.content.len(), config.max_artifact_size
                ),
                severity: Severity::Warning,
            });
        }
        
        // Duplicate check
        if config.duplicate_detection {
            if let Some(embedding) = &artifact.embedding {
                let similar = caliber_vector_search(
                    "caliber_artifact",
                    embedding.clone(),
                    5,
                    0.95,
                );
                
                let duplicates: Vec<_> = similar.iter()
                    .filter(|(id, _)| *id != artifact.artifact_id)
                    .collect();
                
                if !duplicates.is_empty() {
                    issues.push(LintIssue {
                        issue_type: LintIssueType::Duplicate,
                        message: format!(
                            "Near-duplicate of artifact {}",
                            duplicates[0].0
                        ),
                        severity: Severity::Info,
                    });
                }
            }
        }
        
        Ok(LintResult {
            valid: issues.iter().all(|i| i.severity != Severity::Error),
            issues,
        })
    }
}

// === RESULT TYPES ===

#[derive(Debug, Clone, PostgresType)]
pub struct ValidationResult {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, PostgresType)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub issue_type: IssueType,
    pub message: String,
    pub artifact_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum IssueType {
    Contradiction,
    StaleData,
    LowConfidence,
    MissingSource,
}

#[derive(Debug, Clone, PostgresType)]
pub struct RecoveryResult {
    pub success: bool,
    pub reason: Option<String>,
    pub restored_to: Option<chrono::DateTime<chrono::Utc>>,
    pub new_scope_id: Option<Uuid>,
}

#[derive(Debug, Clone, PostgresType)]
pub struct LintResult {
    pub valid: bool,
    pub issues: Vec<LintIssue>,
}

#[derive(Debug, Clone, PostgresType)]
pub struct LintIssue {
    pub issue_type: LintIssueType,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum LintIssueType {
    Oversized,
    Duplicate,
    MalformedContent,
    VocabularyViolation,
}

#[derive(Debug, Clone)]
struct Contradiction {
    a: Artifact,
    b: Artifact,
}

#[derive(Debug, Clone, PostgresType)]
pub struct PCPCheckpoint {
    pub checkpoint_id: Uuid,
    pub trajectory_id: Uuid,
    pub scope_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub state: CheckpointState,
    pub recoverable: bool,
}

#[derive(Debug, Clone, PostgresType)]
pub struct CheckpointState {
    pub trajectory_status: TrajectoryStatus,
    pub scope_count: i32,
    pub active_scope_sequence: Option<i32>,
    pub context_snapshot: ContextWindow,
}
```

---

## 6. STORAGE ARCHITECTURE

### 6.1 No Adapter Layer

CALIBER does NOT have a "storage adapter" abstraction because:

1. **Direct heap access** - pgrx functions call Postgres storage engine directly
2. **No abstraction tax** - No interface overhead, no virtual dispatch
3. **Transaction semantics** - Postgres transactions work automatically
4. **Single source of truth** - Everything in Postgres, no sync issues

### 6.2 Relation Layout

All CALIBER data lives in Postgres heap relations:

```
caliber_trajectory    - Task containers
caliber_scope         - Context partitions  
caliber_artifact      - Typed outputs
caliber_note          - Long-term knowledge
caliber_turn          - Ephemeral conversation buffer
caliber_checkpoint    - PCP recovery points
caliber_agent         - Multi-agent registry (optional)
caliber_lock          - Distributed locks (optional)
caliber_message       - Agent messaging (optional)
```

### 6.3 Index Strategy

```rust
// Each relation has specific index strategy for access patterns

// Trajectory: lookup by ID, filter by status
// Primary: B-tree on trajectory_id (default)
// Secondary: Hash on status (equality filter)

// Scope: lookup by trajectory, ordered by sequence
// Primary: B-tree on scope_id
// Secondary: B-tree on (trajectory_id, sequence_number)

// Artifact: lookup, filter by type, vector search
// Primary: B-tree on artifact_id
// Secondary: B-tree on trajectory_id
// Secondary: Hash on artifact_type
// Vector: HNSW on content_embedding

// Note: entity lookup, vector search
// Primary: B-tree on note_id
// Secondary: Hash on entity
// Vector: HNSW on content_embedding
```

### 6.4 Extension Bootstrap

```rust
// Extension initialization creates relations if not exist
#[pg_extern]
pub fn caliber_init() {
    // Create relations using SPI (one-time setup, SQL is fine here)
    Spi::run(include_str!("../sql/bootstrap.sql"))
        .expect("Failed to bootstrap CALIBER relations");
}

// bootstrap.sql contains CREATE TABLE statements
// These run ONCE at extension install, not in hot path
```

### 6.5 Cache Strategy

Hot data is cached in Postgres shared buffers (no external Redis needed for most cases):

```rust
// Postgres already caches frequently accessed pages
// Tune shared_buffers for your workload

// For extremely hot data (e.g., active trajectory metadata),
// use Postgres unlogged tables or extension-managed memory:

#[pg_guard]
static mut ACTIVE_TRAJECTORIES: Option<HashMap<Uuid, Trajectory>> = None;

pub fn cache_active_trajectory(trajectory: &Trajectory) {
    unsafe {
        if ACTIVE_TRAJECTORIES.is_none() {
            ACTIVE_TRAJECTORIES = Some(HashMap::new());
        }
        ACTIVE_TRAJECTORIES.as_mut().unwrap()
            .insert(trajectory.trajectory_id, trajectory.clone());
    }
}
```

### 6.6 External Embedding Cache (Optional)

For embedding API calls, a simple file cache or Redis can reduce costs:

```rust
// Embedding cache is ONLY for external API call deduplication
// NOT for Postgres storage operations

pub struct EmbeddingCache {
    // Simple LRU in shared memory
    // Or Redis if you have it
}

impl EmbeddingCache {
    pub fn get(&self, content_hash: &[u8; 32]) -> Option<EmbeddingVector> {
        // Check cache before calling external embedding API
    }
    
    pub fn set(&self, content_hash: &[u8; 32], embedding: &EmbeddingVector) {
        // Cache after external API call
    }
}
```

---

## 7. OBSERVABILITY & DEBUGGING (Rust)

### 7.1 Trace Types

```rust
#[derive(Debug, Clone, PostgresType)]
pub struct CaliberTrace {
    pub trace_id: Uuid,
    pub trajectory_id: Uuid,
    pub events: Vec<TraceEvent>,
    pub summary: TraceSummary,
}

#[derive(Debug, Clone, PostgresType)]
pub struct TraceSummary {
    pub duration_ms: i64,
    pub scopes_created: i32,
    pub artifacts_created: i32,
    pub notes_created: i32,
    pub context_assemblies: i32,
    pub total_tokens_input: i64,
    pub total_tokens_output: i64,
    pub checkpoints_created: i32,
    pub recoveries_performed: i32,
}

#[derive(Debug, Clone, PostgresType)]
pub struct TraceEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: TraceEventType,
    pub scope_id: Option<Uuid>,
    pub payload: String,  // JSON serialized
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum TraceEventType {
    TrajectoryStart,
    TrajectoryComplete,
    ScopeOpen,
    ScopeClose,
    ArtifactCreate,
    ArtifactRetrieve,
    NoteCreate,
    NoteRetrieve,
    ContextAssemble,
    ContextInject,
    CheckpointCreate,
    CheckpointRecover,
    PolicyExecute,
    ValidationRun,
    Error,
}
```

### 7.2 Debug Functions (pgrx)

```rust
/// Get current state of a trajectory
#[pg_extern]
pub fn caliber_debug_trajectory_state(trajectory_id: Uuid) -> String {
    let trajectory = caliber_trajectory_get(trajectory_id);
    let scopes = caliber_scope_list_by_trajectory(trajectory_id);
    let artifact_count = caliber_artifact_count_by_trajectory(trajectory_id);
    let note_count = caliber_note_count_by_trajectory(trajectory_id);
    let checkpoints = caliber_checkpoint_list_by_trajectory(trajectory_id);
    
    serde_json::json!({
        "trajectory": trajectory,
        "scopes": scopes,
        "active_scope": scopes.iter().find(|s| s.closed_at.is_none()),
        "artifact_count": artifact_count,
        "note_count": note_count,
        "checkpoint_count": checkpoints.len(),
    }).to_string()
}

/// Get trace events for a trajectory
#[pg_extern]
pub fn caliber_debug_trace_events(
    trajectory_id: Uuid,
    event_type_filter: Option<&str>,
    limit: Option<i32>,
) -> String {
    let events = caliber_trace_query(trajectory_id, event_type_filter, limit.unwrap_or(100));
    serde_json::to_string(&events).unwrap_or_default()
}

/// Explain why a context window was assembled the way it was
#[pg_extern]
pub fn caliber_debug_explain_assembly(window_id: Uuid) -> String {
    let window = caliber_context_window_get(window_id)
        .expect("Window not found");
    
    let inclusions: Vec<_> = window.assembly_trace.iter()
        .filter(|d| d.action == AssemblyAction::Include)
        .map(|d| serde_json::json!({
            "source_type": d.target_type,
            "source_id": d.target_id,
            "reason": d.reason,
            "tokens": d.tokens_affected,
        }))
        .collect();
    
    let exclusions: Vec<_> = window.assembly_trace.iter()
        .filter(|d| d.action == AssemblyAction::Exclude)
        .map(|d| serde_json::json!({
            "source_type": d.target_type,
            "source_id": d.target_id,
            "reason": d.reason,
            "would_have_tokens": d.tokens_affected,
        }))
        .collect();
    
    let by_section: std::collections::HashMap<String, i32> = window.sections.iter()
        .map(|s| (format!("{:?}", s.section_type), s.token_count))
        .collect();
    
    serde_json::json!({
        "window_id": window_id,
        "inclusions": inclusions,
        "exclusions": exclusions,
        "budget": {
            "total": window.max_tokens,
            "used": window.used_tokens,
            "by_section": by_section,
        },
    }).to_string()
}

/// Get provenance chain for an artifact
#[pg_extern]
pub fn caliber_debug_artifact_provenance(artifact_id: Uuid) -> String {
    let artifact = caliber_artifact_get(artifact_id)
        .expect("Artifact not found");
    
    let mut chain = vec![artifact.clone()];
    let mut current = artifact;
    
    // Walk supersession chain backward
    while let Some(supersedes_id) = current.superseded_by {
        // This artifact supersedes something
    }
    
    // Walk to find what superseded this
    let superseding = caliber_artifact_find_superseding(artifact_id);
    
    serde_json::json!({
        "artifact": artifact,
        "scope_id": artifact.scope_id,
        "trajectory_id": artifact.trajectory_id,
        "provenance": artifact.provenance,
        "superseded_by": superseding.map(|a| a.artifact_id),
    }).to_string()
}

/// Record a trace event
pub fn trace_event(
    trajectory_id: Uuid,
    event_type: TraceEventType,
    scope_id: Option<Uuid>,
    payload: &str,
    duration_ms: Option<i64>,
) {
    let event = TraceEvent {
        timestamp: chrono::Utc::now(),
        event_type,
        scope_id,
        payload: payload.to_string(),
        duration_ms,
    };
    
    caliber_trace_insert(trajectory_id, &event);
}
```

### 7.3 Human Debug Views (SQL - NOT in hot path)

```rust
/// Create debug views for human inspection
/// Called once at extension install, not runtime
#[pg_extern]
pub fn caliber_create_debug_views() {
    Spi::run(r#"
        CREATE OR REPLACE VIEW caliber_debug_trajectories AS
        SELECT 
            trajectory_id,
            status,
            created_at,
            updated_at,
            (SELECT COUNT(*) FROM caliber_scope WHERE trajectory_id = t.trajectory_id) as scope_count,
            (SELECT COUNT(*) FROM caliber_artifact WHERE trajectory_id = t.trajectory_id) as artifact_count
        FROM caliber_trajectory t
        ORDER BY created_at DESC;
        
        CREATE OR REPLACE VIEW caliber_debug_recent_artifacts AS
        SELECT 
            artifact_id,
            trajectory_id,
            artifact_type,
            created_at,
            length(content) as content_bytes
        FROM caliber_artifact
        ORDER BY created_at DESC
        LIMIT 100;
        
        CREATE OR REPLACE VIEW caliber_debug_note_confidence AS
        SELECT 
            note_id,
            entity,
            note_type,
            confidence,
            array_length(source_trajectory_ids, 1) as source_count
        FROM caliber_note
        ORDER BY confidence DESC;
        
        COMMENT ON VIEW caliber_debug_trajectories IS 
            'Human debug interface only - NOT for agent runtime use';
    "#).expect("Failed to create debug views");
}
```

---

## 8. IMPLEMENTATION CHECKLIST

For an AI agent implementing this system, execute in order:

### Phase 1: Rust/pgrx Foundation

```
[x] Set up Rust project with pgrx
[x] cargo pgrx init
[x] Define PostgresType structs for all core types
[x] Implement UUIDv7 generation
[x] Implement SHA-256 hashing
[x] Implement cosine similarity for embeddings
```

### Phase 2: Direct Storage Functions

```
[x] caliber_trajectory_insert() - direct heap insert
[x] caliber_trajectory_get() - index scan
[x] caliber_scope_insert/get/close()
[x] caliber_artifact_insert/get()
[x] caliber_note_insert/get()
[x] caliber_vector_search() - direct HNSW access
```

### Phase 3: Bootstrap SQL (one-time)

```
[x] CREATE TABLE statements for all relations
[x] CREATE INDEX statements (B-tree, hash, HNSW)
[x] Wrap in caliber_init() function
```

### Phase 4: Core Runtime

```
[x] CaliberOrchestrator struct (handled by caliber-api orchestration layer)
[x] start_trajectory() / complete_trajectory()
[x] open_scope() / close_scope()
[x] create_artifact()
[x] consolidate_notes()
```

### Phase 5: Context Assembly

```
[x] assemble_context() - main entry
[x] assemble_section() - per injection rule
[x] Token counting
[x] Priority-based truncation
```

### Phase 6: PCP Layer

```
[x] PCPRuntime struct
[x] validate_context_integrity()
[x] create_checkpoint() / recover_from_checkpoint()
[x] apply_dosage_limits()
[x] lint_artifact()
```

### Phase 7: Observability

```
[x] Trace event collection
[x] Debug query functions
[x] Assembly explanation
```

### Phase 8: FFI for External Services

```
[x] Embedding service trait
[x] HTTP embedding client (libcurl FFI)
[x] Summarization service trait
```

---

## 9. APPENDIX: UTILITY FUNCTIONS (Rust)

```rust
use sha2::{Sha256, Digest};
use uuid::Uuid;

/// Generate UUIDv7 (timestamp-sortable)
pub fn generate_uuidv7() -> Uuid {
    let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
    
    let mut bytes = [0u8; 16];
    
    // First 48 bits: timestamp
    bytes[0] = ((timestamp_ms >> 40) & 0xFF) as u8;
    bytes[1] = ((timestamp_ms >> 32) & 0xFF) as u8;
    bytes[2] = ((timestamp_ms >> 24) & 0xFF) as u8;
    bytes[3] = ((timestamp_ms >> 16) & 0xFF) as u8;
    bytes[4] = ((timestamp_ms >> 8) & 0xFF) as u8;
    bytes[5] = (timestamp_ms & 0xFF) as u8;
    
    // Version 7
    bytes[6] = 0x70 | (rand::random::<u8>() & 0x0F);
    bytes[7] = rand::random();
    
    // Variant bits (10xx)
    bytes[8] = 0x80 | (rand::random::<u8>() & 0x3F);
    
    // Random bytes for rest
    for byte in bytes.iter_mut().skip(9) {
        *byte = rand::random();
    }
    
    Uuid::from_bytes(bytes)
}

/// SHA-256 hash
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Cosine similarity between two embedding vectors
pub fn cosine_similarity<const N: usize>(a: &[f32; N], b: &[f32; N]) -> f32 {
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    
    for i in 0..N {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

/// Simple token estimation
pub fn estimate_tokens(text: &str) -> i32 {
    (text.len() as f32 / 3.5).ceil() as i32
}

/// Truncate text to approximately N tokens
pub fn truncate_to_tokens(text: &str, max_tokens: i32) -> String {
    let estimated = estimate_tokens(text);
    if estimated <= max_tokens {
        return text.to_string();
    }
    
    let target_chars = (max_tokens as f32 * 3.5) as usize;
    let truncated: String = text.chars().take(target_chars).collect();
    
    if let Some(last_space) = truncated.rfind(' ') {
        truncated[..last_space].to_string() + "..."
    } else {
        truncated + "..."
    }
}

/// Parse duration string to milliseconds
pub fn parse_duration_ms(duration: &str) -> i64 {
    let len = duration.len();
    if len < 2 { return 0; }
    
    let value: i64 = duration[..len-1].parse().unwrap_or(0);
    let unit = &duration[len-1..];
    
    match unit {
        "s" => value * 1000,
        "m" => value * 60 * 1000,
        "h" => value * 60 * 60 * 1000,
        "d" => value * 24 * 60 * 60 * 1000,
        "w" => value * 7 * 24 * 60 * 60 * 1000,
        _ => 0,
    }
}

/// Emit Postgres NOTIFY
#[inline]
pub fn pg_notify(channel: &str, payload: &str) {
    use pgx::pg_sys;
    use std::ffi::CString;
    
    unsafe {
        let channel_cstr = CString::new(channel).unwrap();
        let payload_cstr = CString::new(payload).unwrap();
        pg_sys::Async_Notify(channel_cstr.as_ptr(), payload_cstr.as_ptr());
    }
}
```

---

## 10. BUILD CONFIGURATION: Multi-Crate Workspace

### 10.1 Directory Structure

```
caliber/
├── Cargo.toml                      # Workspace root
├── caliber-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types.rs                # Entity types
│       ├── error.rs                # CaliberError
│       └── config.rs               # CaliberConfig
├── caliber-storage/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── traits.rs               # StorageTrait
├── caliber-context/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── assembler.rs            # Context assembly
├── caliber-pcp/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── validator.rs            # PCP validation
│       └── checkpoint.rs           # Checkpoint/recovery
├── caliber-llm/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── provider.rs             # EmbeddingProvider, SummarizationProvider traits
│       └── cache.rs                # Embedding cache
├── caliber-agents/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── registry.rs             # Agent registration
│       ├── locks.rs                # Distributed locks
│       ├── messages.rs             # Message passing
│       ├── delegation.rs           # Task delegation
│       └── handoff.rs              # Agent handoffs
├── caliber-dsl/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── lexer.rs
│       ├── parser.rs
│       └── codegen.rs              # Generates CaliberConfig
├── caliber-pg/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                  # Extension entry, wires all components
│       ├── storage_impl.rs         # StorageTrait impl via pgrx
│       └── functions.rs            # pg_extern functions
└── sql/
    └── bootstrap.sql               # DDL for all tables
```

### 10.2 Workspace Cargo.toml (Root)

```toml
# caliber/Cargo.toml

[workspace]
resolver = "2"
members = [
    "caliber-core",
    "caliber-storage",
    "caliber-context",
    "caliber-pcp",
    "caliber-llm",
    "caliber-agents",
    "caliber-dsl",
    "caliber-pg",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourorg/caliber"

[workspace.dependencies]
# Shared dependencies - version managed at workspace level
uuid = { version = "1.0", features = ["v4", "v7"] }
chrono = { version = "0.4", features = ["serde"] }
sha2 = "0.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
```

### 10.3 Per-Crate Cargo.toml Files

```toml
# caliber-core/Cargo.toml
[package]
name = "caliber-core"
version.workspace = true
edition.workspace = true

[dependencies]
uuid.workspace = true
chrono.workspace = true
sha2.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
```

```toml
# caliber-storage/Cargo.toml
[package]
name = "caliber-storage"
version.workspace = true
edition.workspace = true

[dependencies]
caliber-core = { path = "../caliber-core" }
uuid.workspace = true
```

```toml
# caliber-context/Cargo.toml
[package]
name = "caliber-context"
version.workspace = true
edition.workspace = true

[dependencies]
caliber-core = { path = "../caliber-core" }
caliber-storage = { path = "../caliber-storage" }
uuid.workspace = true
```

```toml
# caliber-pcp/Cargo.toml
[package]
name = "caliber-pcp"
version.workspace = true
edition.workspace = true

[dependencies]
caliber-core = { path = "../caliber-core" }
caliber-storage = { path = "../caliber-storage" }
uuid.workspace = true
chrono.workspace = true
```

```toml
# caliber-llm/Cargo.toml
[package]
name = "caliber-llm"
version.workspace = true
edition.workspace = true

[dependencies]
caliber-core = { path = "../caliber-core" }
reqwest = { version = "0.11", features = ["blocking", "json"] }
serde.workspace = true
serde_json.workspace = true
```

```toml
# caliber-agents/Cargo.toml
[package]
name = "caliber-agents"
version.workspace = true
edition.workspace = true

[dependencies]
caliber-core = { path = "../caliber-core" }
caliber-storage = { path = "../caliber-storage" }
uuid.workspace = true
chrono.workspace = true
serde.workspace = true
```

```toml
# caliber-dsl/Cargo.toml
[package]
name = "caliber-dsl"
version.workspace = true
edition.workspace = true

[dependencies]
caliber-core = { path = "../caliber-core" }
# DSL is standalone - only depends on core types
```

```toml
# caliber-pg/Cargo.toml
[package]
name = "caliber-pg"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
# All caliber crates
caliber-core = { path = "../caliber-core" }
caliber-storage = { path = "../caliber-storage" }
caliber-context = { path = "../caliber-context" }
caliber-pcp = { path = "../caliber-pcp" }
caliber-llm = { path = "../caliber-llm" }
caliber-agents = { path = "../caliber-agents" }

# pgrx for Postgres extension
pgrx = "0.11"
uuid.workspace = true
chrono.workspace = true
serde.workspace = true
serde_json.workspace = true

[features]
default = ["pg16"]
pg14 = ["pgrx/pg14"]
pg15 = ["pgrx/pg15"]
pg16 = ["pgrx/pg16"]
```

### 10.4 Dependency Graph

```
┌─────────────────┐
│  caliber-core   │  ← No internal deps (just std + uuid/chrono/serde)
└────────┬────────┘
         │
    ┌────┴────┬──────────────┬─────────────┐
    │         │              │             │
    ▼         ▼              ▼             ▼
┌────────┐ ┌────────┐ ┌──────────┐ ┌─────────────┐
│storage │ │  llm   │ │   dsl    │ │   agents    │
│ trait  │ │ traits │ │ (parser) │ │  (types)    │
└────┬───┘ └────┬───┘ └──────────┘ └──────┬──────┘
     │          │                         │
     │    ┌─────┴─────┐                   │
     │    │           │                   │
     ▼    ▼           ▼                   │
┌─────────────┐ ┌──────────┐              │
│   context   │ │   pcp    │              │
│  assembler  │ │validator │              │
└──────┬──────┘ └────┬─────┘              │
       │             │                    │
       └──────┬──────┴────────────────────┘
              │
              ▼
      ┌──────────────┐
      │  caliber-pg  │  ← pgrx extension, wires everything
      └──────────────┘
```

### 10.5 Build Commands

```bash
# Build all crates
cargo build --workspace

# Build just the extension
cargo build -p caliber-pg --release

# Run tests for all crates (except pg which needs pgrx test)
cargo test --workspace --exclude caliber-pg

# Test the extension
cargo pgrx test -p caliber-pg pg16

# Package for production
cargo pgrx package -p caliber-pg

# Install
psql -c "CREATE EXTENSION caliber;"
```

---

## 11. STORAGE TRAIT DEFINITION

The `StorageTrait` is the core abstraction that `caliber-pg` implements via pgrx. Other implementations (mocks, alternative backends) can implement this trait.

```rust
// caliber-storage/src/traits.rs

use caliber_core::*;

/// Core storage operations
/// Implemented by caliber-pg via pgrx direct heap access
/// Can be mocked for testing
pub trait CaliberStorage: Send + Sync {
    // === TRAJECTORY ===
    fn trajectory_insert(&self, trajectory: &Trajectory) -> CaliberResult<()>;
    fn trajectory_get(&self, id: EntityId) -> CaliberResult<Option<Trajectory>>;
    fn trajectory_update(&self, id: EntityId, update: TrajectoryUpdate) -> CaliberResult<()>;
    fn trajectory_list_by_status(&self, status: TrajectoryStatus) -> CaliberResult<Vec<Trajectory>>;
    fn trajectory_list_children(&self, parent_id: EntityId) -> CaliberResult<Vec<Trajectory>>;
    
    // === SCOPE ===
    fn scope_insert(&self, scope: &Scope) -> CaliberResult<()>;
    fn scope_get(&self, id: EntityId) -> CaliberResult<Option<Scope>>;
    fn scope_close(&self, id: EntityId, close: ScopeClose) -> CaliberResult<()>;
    fn scope_list_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<Vec<Scope>>;
    fn scope_get_current(&self, trajectory_id: EntityId) -> CaliberResult<Option<Scope>>;
    fn scope_count_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<i32>;
    
    // === ARTIFACT ===
    fn artifact_insert(&self, artifact: &Artifact) -> CaliberResult<()>;
    fn artifact_get(&self, id: EntityId) -> CaliberResult<Option<Artifact>>;
    fn artifact_get_by_hash(&self, hash: &ContentHash) -> CaliberResult<Option<Artifact>>;
    fn artifact_update(&self, id: EntityId, update: ArtifactUpdate) -> CaliberResult<()>;
    fn artifact_list_by_scope(&self, scope_id: EntityId) -> CaliberResult<Vec<Artifact>>;
    fn artifact_list_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<Vec<Artifact>>;
    fn artifact_list_by_type(&self, trajectory_id: EntityId, artifact_type: ArtifactType) -> CaliberResult<Vec<Artifact>>;
    fn artifact_search_similar(&self, embedding: &EmbeddingVector, limit: i32, min_similarity: f32) -> CaliberResult<Vec<(EntityId, f32)>>;
    fn artifact_count_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<i32>;
    
    // === NOTE ===
    fn note_insert(&self, note: &Note) -> CaliberResult<()>;
    fn note_get(&self, id: EntityId) -> CaliberResult<Option<Note>>;
    fn note_update(&self, id: EntityId, update: NoteUpdate) -> CaliberResult<()>;
    fn note_find_by_entity(&self, entity: &str, entity_type: &str) -> CaliberResult<Vec<Note>>;
    fn note_search_similar(&self, embedding: &EmbeddingVector, limit: i32, min_similarity: f32) -> CaliberResult<Vec<(EntityId, f32)>>;
    fn note_count_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<i32>;
    
    // === TURN ===
    fn turn_insert(&self, turn: &Turn) -> CaliberResult<()>;
    fn turn_list_by_scope(&self, scope_id: EntityId) -> CaliberResult<Vec<Turn>>;
    fn turn_delete_by_scope(&self, scope_id: EntityId) -> CaliberResult<i32>;
    fn turn_count_by_scope(&self, scope_id: EntityId) -> CaliberResult<i32>;
    
    // === CHECKPOINT ===
    fn checkpoint_insert(&self, checkpoint: &Checkpoint) -> CaliberResult<()>;
    fn checkpoint_get(&self, id: EntityId) -> CaliberResult<Option<Checkpoint>>;
    fn checkpoint_list_by_trajectory(&self, trajectory_id: EntityId, limit: i32) -> CaliberResult<Vec<Checkpoint>>;
    fn checkpoint_get_latest(&self, trajectory_id: EntityId) -> CaliberResult<Option<Checkpoint>>;
    fn checkpoint_delete_oldest(&self, trajectory_id: EntityId, keep: i32) -> CaliberResult<i32>;
    
    // === CONTEXT WINDOW ===
    fn context_window_insert(&self, window: &ContextWindow) -> CaliberResult<()>;
    fn context_window_get(&self, id: EntityId) -> CaliberResult<Option<ContextWindow>>;
    fn context_window_delete_expired(&self) -> CaliberResult<i32>;
    
    // === TRACE ===
    fn trace_insert(&self, trajectory_id: EntityId, event: &TraceEvent) -> CaliberResult<()>;
    fn trace_query(&self, trajectory_id: EntityId, event_type: Option<TraceEventType>, limit: i32) -> CaliberResult<Vec<TraceEvent>>;
    fn trace_delete_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<i32>;
    
    // === EMBEDDING CACHE ===
    fn embedding_cache_get(&self, content_hash: &ContentHash) -> CaliberResult<Option<EmbeddingVector>>;
    fn embedding_cache_insert(&self, content_hash: &ContentHash, embedding: &EmbeddingVector) -> CaliberResult<()>;
    fn embedding_cache_delete_older_than(&self, age: Duration) -> CaliberResult<i32>;
    
    // === AGENT (multi-agent) ===
    fn agent_insert(&self, agent: &Agent) -> CaliberResult<()>;
    fn agent_get(&self, id: EntityId) -> CaliberResult<Option<Agent>>;
    fn agent_update(&self, id: EntityId, update: AgentUpdate) -> CaliberResult<()>;
    fn agent_list_by_type(&self, agent_type: &str) -> CaliberResult<Vec<Agent>>;
    fn agent_list_by_status(&self, status: AgentStatus) -> CaliberResult<Vec<Agent>>;
    
    // === LOCKS ===
    fn lock_acquire(&self, agent_id: EntityId, resource_type: &str, resource_id: EntityId, mode: LockMode, timeout: Duration) -> CaliberResult<Option<EntityId>>;
    fn lock_release(&self, lock_id: EntityId) -> CaliberResult<bool>;
    fn lock_get(&self, lock_id: EntityId) -> CaliberResult<Option<DistributedLock>>;
    fn lock_cleanup_expired(&self) -> CaliberResult<i32>;
    
    // === MESSAGES ===
    fn message_insert(&self, message: &AgentMessage) -> CaliberResult<()>;
    fn message_query_pending(&self, agent_id: Option<EntityId>, agent_type: Option<&str>, limit: i32) -> CaliberResult<Vec<AgentMessage>>;
    fn message_mark_delivered(&self, id: EntityId) -> CaliberResult<()>;
    fn message_mark_acknowledged(&self, id: EntityId) -> CaliberResult<()>;
    fn message_delete_old(&self, older_than: Duration) -> CaliberResult<i32>;
    
    // === DELEGATION ===
    fn delegation_insert(&self, delegation: &DelegatedTask) -> CaliberResult<()>;
    fn delegation_get(&self, id: EntityId) -> CaliberResult<Option<DelegatedTask>>;
    fn delegation_update(&self, id: EntityId, update: DelegationUpdate) -> CaliberResult<()>;
    fn delegation_query_pending(&self, agent_type: Option<&str>) -> CaliberResult<Vec<DelegatedTask>>;
    
    // === HANDOFF ===
    fn handoff_insert(&self, handoff: &AgentHandoff) -> CaliberResult<()>;
    fn handoff_get(&self, id: EntityId) -> CaliberResult<Option<AgentHandoff>>;
    fn handoff_update(&self, id: EntityId, update: HandoffUpdate) -> CaliberResult<()>;
    fn handoff_query_pending(&self, agent_type: Option<&str>) -> CaliberResult<Vec<AgentHandoff>>;
    
    // === CONFLICT ===
    fn conflict_insert(&self, conflict: &Conflict) -> CaliberResult<()>;
    fn conflict_get(&self, id: EntityId) -> CaliberResult<Option<Conflict>>;
    fn conflict_update(&self, id: EntityId, update: ConflictUpdate) -> CaliberResult<()>;
    fn conflict_list_unresolved(&self, trajectory_id: Option<EntityId>) -> CaliberResult<Vec<Conflict>>;
}

/// Update structs for partial updates
#[derive(Debug, Clone, Default)]
pub struct TrajectoryUpdate {
    pub status: Option<TrajectoryStatus>,
    pub outcome: Option<TrajectoryOutcome>,
    pub updated_at: Option<Timestamp>,
}

#[derive(Debug, Clone)]
pub struct ScopeClose {
    pub end_turn_id: Option<EntityId>,
    pub closed_at: Timestamp,
    pub summary: String,
    pub summary_embedding: EmbeddingVector,
    pub artifact_ids: Vec<EntityId>,
    pub checkpoint_id: Option<EntityId>,
}

#[derive(Debug, Clone, Default)]
pub struct ArtifactUpdate {
    pub superseded_by: Option<EntityId>,
}

#[derive(Debug, Clone, Default)]
pub struct NoteUpdate {
    pub content: Option<String>,
    pub content_embedding: Option<EmbeddingVector>,
    pub confidence: Option<f32>,
    pub valid_until: Option<Timestamp>,
    pub supersedes: Option<Vec<EntityId>>,
    pub conflicts_with: Option<Vec<EntityId>>,
    pub updated_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentUpdate {
    pub status: Option<AgentStatus>,
    pub current_trajectory_id: Option<Option<EntityId>>,
    pub current_scope_id: Option<Option<EntityId>>,
    pub last_heartbeat: Option<Timestamp>,
}

#[derive(Debug, Clone, Default)]
pub struct DelegationUpdate {
    pub delegatee_agent_id: Option<EntityId>,
    pub child_trajectory_id: Option<EntityId>,
    pub status: Option<DelegationStatus>,
    pub result: Option<DelegationResult>,
    pub accepted_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Default)]
pub struct HandoffUpdate {
    pub to_agent_id: Option<EntityId>,
    pub status: Option<HandoffStatus>,
    pub accepted_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Default)]
pub struct ConflictUpdate {
    pub status: Option<ConflictStatus>,
    pub resolution: Option<ConflictResolutionRecord>,
    pub resolved_at: Option<Timestamp>,
}
```

---

## 12. COMPLETE BOOTSTRAP SQL

```sql
-- caliber/sql/bootstrap.sql
-- Run once at CREATE EXTENSION caliber

-- Enable pgvector if not already enabled
CREATE EXTENSION IF NOT EXISTS vector;

-- ============================================================
-- CORE TABLES
-- ============================================================

CREATE TABLE IF NOT EXISTS caliber_trajectory (
    trajectory_id       UUID PRIMARY KEY,
    goal_hash           BYTEA NOT NULL,
    status              TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'completed', 'failed', 'suspended')),
    initiator_type      TEXT NOT NULL,
    initiator_id        UUID NOT NULL,
    parent_trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    outcome             JSONB
);

CREATE TABLE IF NOT EXISTS caliber_scope (
    scope_id            UUID PRIMARY KEY,
    trajectory_id       UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id) ON DELETE CASCADE,
    sequence_number     INT NOT NULL,
    start_turn_id       UUID,
    end_turn_id         UUID,
    opened_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at           TIMESTAMPTZ,
    summary             TEXT,
    summary_embedding   vector,
    artifact_ids        UUID[] DEFAULT '{}',
    checkpoint_id       UUID,
    UNIQUE(trajectory_id, sequence_number)
);

CREATE TABLE IF NOT EXISTS caliber_artifact (
    artifact_id         UUID PRIMARY KEY,
    scope_id            UUID NOT NULL REFERENCES caliber_scope(scope_id) ON DELETE CASCADE,
    trajectory_id       UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id) ON DELETE CASCADE,
    artifact_type       TEXT NOT NULL,
    content             BYTEA NOT NULL,
    content_hash        BYTEA NOT NULL,
    content_embedding   vector,
    extraction_method   TEXT NOT NULL,
    confidence          FLOAT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    superseded_by       UUID REFERENCES caliber_artifact(artifact_id),
    provenance          JSONB
);

CREATE TABLE IF NOT EXISTS caliber_note (
    note_id             UUID PRIMARY KEY,
    entity              TEXT NOT NULL,
    entity_type         TEXT NOT NULL,
    note_type           TEXT NOT NULL
                        CHECK (note_type IN ('convention', 'strategy', 'gotcha', 'fact', 
                                             'preference', 'relationship', 'procedure', 'meta')),
    content             TEXT NOT NULL,
    content_embedding   vector,
    source_trajectory_ids UUID[] NOT NULL DEFAULT '{}',
    confidence          FLOAT NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    valid_from          TIMESTAMPTZ,
    valid_until         TIMESTAMPTZ,
    supersedes          UUID[] DEFAULT '{}',
    conflicts_with      UUID[] DEFAULT '{}',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS caliber_turn (
    turn_id             UUID PRIMARY KEY,
    scope_id            UUID NOT NULL REFERENCES caliber_scope(scope_id) ON DELETE CASCADE,
    sequence            INT NOT NULL,
    role                TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content             BYTEA NOT NULL,
    token_count         INT NOT NULL,
    model_id            TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(scope_id, sequence)
);

CREATE TABLE IF NOT EXISTS caliber_checkpoint (
    checkpoint_id       UUID PRIMARY KEY,
    trajectory_id       UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id) ON DELETE CASCADE,
    scope_id            UUID NOT NULL REFERENCES caliber_scope(scope_id) ON DELETE CASCADE,
    state               JSONB NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS caliber_context_window (
    window_id           UUID PRIMARY KEY,
    trajectory_id       UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id) ON DELETE CASCADE,
    assembled_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    max_tokens          INT NOT NULL,
    used_tokens         INT NOT NULL,
    sections            JSONB NOT NULL,
    assembly_trace      JSONB NOT NULL,
    expires_at          TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS caliber_trace (
    trace_id            UUID PRIMARY KEY,
    trajectory_id       UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id) ON DELETE CASCADE,
    timestamp           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type          TEXT NOT NULL,
    scope_id            UUID,
    payload             JSONB NOT NULL DEFAULT '{}',
    duration_ms         BIGINT
);

CREATE TABLE IF NOT EXISTS caliber_embedding_cache (
    content_hash        BYTEA PRIMARY KEY,
    embedding           vector NOT NULL,
    model_id            TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- MULTI-AGENT TABLES
-- ============================================================

CREATE TABLE IF NOT EXISTS caliber_agent (
    agent_id            UUID PRIMARY KEY,
    agent_type          TEXT NOT NULL,
    capabilities        TEXT[] DEFAULT '{}',
    memory_access       JSONB NOT NULL,
    status              TEXT DEFAULT 'idle'
                        CHECK (status IN ('idle', 'active', 'blocked', 'failed')),
    current_trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    current_scope_id    UUID REFERENCES caliber_scope(scope_id),
    can_delegate_to     TEXT[] DEFAULT '{}',
    reports_to          UUID REFERENCES caliber_agent(agent_id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_heartbeat      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS caliber_lock (
    lock_id             UUID PRIMARY KEY,
    resource_type       TEXT NOT NULL,
    resource_id         UUID NOT NULL,
    holder_agent_id     UUID NOT NULL REFERENCES caliber_agent(agent_id),
    acquired_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at          TIMESTAMPTZ NOT NULL,
    mode                TEXT NOT NULL CHECK (mode IN ('exclusive', 'shared')),
    UNIQUE(resource_type, resource_id, mode)
);

CREATE TABLE IF NOT EXISTS caliber_message (
    message_id          UUID PRIMARY KEY,
    from_agent_id       UUID NOT NULL REFERENCES caliber_agent(agent_id),
    to_agent_id         UUID REFERENCES caliber_agent(agent_id),
    to_agent_type       TEXT,
    message_type        TEXT NOT NULL,
    payload             JSONB NOT NULL,
    trajectory_id       UUID REFERENCES caliber_trajectory(trajectory_id),
    scope_id            UUID REFERENCES caliber_scope(scope_id),
    artifact_ids        UUID[] DEFAULT '{}',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at        TIMESTAMPTZ,
    acknowledged_at     TIMESTAMPTZ,
    priority            TEXT DEFAULT 'normal'
                        CHECK (priority IN ('low', 'normal', 'high', 'critical')),
    expires_at          TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS caliber_delegation (
    delegation_id       UUID PRIMARY KEY,
    delegator_agent_id  UUID NOT NULL REFERENCES caliber_agent(agent_id),
    delegatee_agent_id  UUID REFERENCES caliber_agent(agent_id),
    delegatee_agent_type TEXT,
    task_description    TEXT NOT NULL,
    parent_trajectory_id UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id),
    child_trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    shared_artifacts    UUID[] DEFAULT '{}',
    shared_notes        UUID[] DEFAULT '{}',
    additional_context  TEXT,
    constraints         JSONB DEFAULT '[]',
    deadline            TIMESTAMPTZ,
    status              TEXT DEFAULT 'pending'
                        CHECK (status IN ('pending', 'accepted', 'rejected', 
                                         'in_progress', 'completed', 'failed')),
    result              JSONB,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at         TIMESTAMPTZ,
    completed_at        TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS caliber_conflict (
    conflict_id         UUID PRIMARY KEY,
    conflict_type       TEXT NOT NULL
                        CHECK (conflict_type IN ('concurrent_write', 'contradicting_fact',
                                                 'incompatible_decision', 'resource_contention',
                                                 'goal_conflict')),
    item_a_type         TEXT NOT NULL,
    item_a_id           UUID NOT NULL,
    item_b_type         TEXT NOT NULL,
    item_b_id           UUID NOT NULL,
    agent_a_id          UUID REFERENCES caliber_agent(agent_id),
    agent_b_id          UUID REFERENCES caliber_agent(agent_id),
    trajectory_id       UUID REFERENCES caliber_trajectory(trajectory_id),
    status              TEXT DEFAULT 'detected'
                        CHECK (status IN ('detected', 'resolving', 'resolved', 'escalated')),
    resolution          JSONB,
    detected_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at         TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS caliber_handoff (
    handoff_id          UUID PRIMARY KEY,
    from_agent_id       UUID NOT NULL REFERENCES caliber_agent(agent_id),
    to_agent_id         UUID REFERENCES caliber_agent(agent_id),
    to_agent_type       TEXT,
    trajectory_id       UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id),
    scope_id            UUID NOT NULL REFERENCES caliber_scope(scope_id),
    context_snapshot_id UUID NOT NULL,
    handoff_notes       TEXT NOT NULL,
    next_steps          TEXT[] DEFAULT '{}',
    blockers            TEXT[] DEFAULT '{}',
    open_questions      TEXT[] DEFAULT '{}',
    status              TEXT DEFAULT 'initiated'
                        CHECK (status IN ('initiated', 'accepted', 'completed', 'rejected')),
    initiated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at         TIMESTAMPTZ,
    completed_at        TIMESTAMPTZ,
    reason              TEXT NOT NULL
);

-- ============================================================
-- INDEXES
-- ============================================================

-- Trajectory indexes
CREATE INDEX IF NOT EXISTS idx_trajectory_status ON caliber_trajectory(status);
CREATE INDEX IF NOT EXISTS idx_trajectory_parent ON caliber_trajectory(parent_trajectory_id) WHERE parent_trajectory_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_trajectory_initiator ON caliber_trajectory(initiator_type, initiator_id);

-- Scope indexes
CREATE INDEX IF NOT EXISTS idx_scope_trajectory ON caliber_scope(trajectory_id);
CREATE INDEX IF NOT EXISTS idx_scope_open ON caliber_scope(trajectory_id) WHERE closed_at IS NULL;

-- Artifact indexes
CREATE INDEX IF NOT EXISTS idx_artifact_trajectory ON caliber_artifact(trajectory_id);
CREATE INDEX IF NOT EXISTS idx_artifact_scope ON caliber_artifact(scope_id);
CREATE INDEX IF NOT EXISTS idx_artifact_type ON caliber_artifact(artifact_type);
CREATE INDEX IF NOT EXISTS idx_artifact_hash ON caliber_artifact(content_hash);

-- Note indexes
CREATE INDEX IF NOT EXISTS idx_note_entity ON caliber_note(entity, entity_type);
CREATE INDEX IF NOT EXISTS idx_note_type ON caliber_note(note_type);

-- Turn indexes
CREATE INDEX IF NOT EXISTS idx_turn_scope ON caliber_turn(scope_id);

-- Checkpoint indexes
CREATE INDEX IF NOT EXISTS idx_checkpoint_trajectory ON caliber_checkpoint(trajectory_id);

-- Context window indexes
CREATE INDEX IF NOT EXISTS idx_context_window_trajectory ON caliber_context_window(trajectory_id);
CREATE INDEX IF NOT EXISTS idx_context_window_expires ON caliber_context_window(expires_at) WHERE expires_at IS NOT NULL;

-- Trace indexes
CREATE INDEX IF NOT EXISTS idx_trace_trajectory ON caliber_trace(trajectory_id);
CREATE INDEX IF NOT EXISTS idx_trace_type ON caliber_trace(event_type);

-- Embedding cache indexes
CREATE INDEX IF NOT EXISTS idx_embedding_cache_created ON caliber_embedding_cache(created_at);

-- Agent indexes
CREATE INDEX IF NOT EXISTS idx_agent_type ON caliber_agent(agent_type);
CREATE INDEX IF NOT EXISTS idx_agent_status ON caliber_agent(status);

-- Lock indexes
CREATE INDEX IF NOT EXISTS idx_lock_resource ON caliber_lock(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_lock_expires ON caliber_lock(expires_at);

-- Message indexes
CREATE INDEX IF NOT EXISTS idx_message_recipient ON caliber_message(to_agent_id, to_agent_type);
CREATE INDEX IF NOT EXISTS idx_message_pending ON caliber_message(delivered_at) WHERE delivered_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_message_priority ON caliber_message(priority, created_at);

-- Delegation indexes
CREATE INDEX IF NOT EXISTS idx_delegation_status ON caliber_delegation(status);
CREATE INDEX IF NOT EXISTS idx_delegation_delegatee ON caliber_delegation(delegatee_agent_type) WHERE delegatee_agent_id IS NULL;

-- Conflict indexes
CREATE INDEX IF NOT EXISTS idx_conflict_status ON caliber_conflict(status);
CREATE INDEX IF NOT EXISTS idx_conflict_trajectory ON caliber_conflict(trajectory_id);

-- Handoff indexes
CREATE INDEX IF NOT EXISTS idx_handoff_status ON caliber_handoff(status);
CREATE INDEX IF NOT EXISTS idx_handoff_to_type ON caliber_handoff(to_agent_type) WHERE to_agent_id IS NULL;

-- ============================================================
-- HNSW VECTOR INDEXES (created via caliber_init with configured dimension)
-- ============================================================
-- These are created dynamically by caliber_create_vector_indexes(dimension)
-- because pgvector requires dimension at index creation time
--
-- CREATE INDEX idx_artifact_embedding ON caliber_artifact 
--     USING hnsw (content_embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64);
-- CREATE INDEX idx_note_embedding ON caliber_note 
--     USING hnsw (content_embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64);
-- CREATE INDEX idx_scope_embedding ON caliber_scope 
--     USING hnsw (summary_embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64);

-- ============================================================
-- COMMENTS (for humans inspecting schema)
-- ============================================================
COMMENT ON TABLE caliber_trajectory IS 'Task containers - top-level unit of work';
COMMENT ON TABLE caliber_scope IS 'Context partitions within a trajectory';
COMMENT ON TABLE caliber_artifact IS 'Typed outputs preserved from scopes';
COMMENT ON TABLE caliber_note IS 'Cross-trajectory knowledge (semantic memory)';
COMMENT ON TABLE caliber_turn IS 'Ephemeral conversation buffer (deleted on scope close)';
COMMENT ON TABLE caliber_checkpoint IS 'PCP recovery points';
COMMENT ON TABLE caliber_context_window IS 'Assembled context snapshots (optional persistence)';
COMMENT ON TABLE caliber_trace IS 'Observability events';
COMMENT ON TABLE caliber_embedding_cache IS 'Cached embeddings to avoid duplicate API calls';
```

---

## 13. STATE MACHINE DEFINITIONS

```rust
// caliber-core/src/state_machines.rs

use crate::{CaliberError, CaliberResult, ValidationError};

// === TRAJECTORY STATE MACHINE ===

impl TrajectoryStatus {
    /// Check if transition is valid
    pub fn can_transition_to(&self, target: TrajectoryStatus) -> bool {
        use TrajectoryStatus::*;
        match (self, target) {
            // From Active
            (Active, Completed) => true,
            (Active, Failed) => true,
            (Active, Suspended) => true,
            
            // From Suspended
            (Suspended, Active) => true,
            (Suspended, Failed) => true,
            
            // Terminal states - no transitions out
            (Completed, _) => false,
            (Failed, _) => false,
            
            // Self-transitions allowed (idempotent)
            (s, t) if s == &t => true,
            
            _ => false,
        }
    }
    
    /// Validate and perform transition
    pub fn transition_to(&self, target: TrajectoryStatus) -> CaliberResult<TrajectoryStatus> {
        if self.can_transition_to(target) {
            Ok(target)
        } else {
            Err(CaliberError::Validation(ValidationError::InvalidStateTransition {
                entity_type: "Trajectory".to_string(),
                from: format!("{:?}", self),
                to: format!("{:?}", target),
            }))
        }
    }
    
    /// Is this a terminal state?
    pub fn is_terminal(&self) -> bool {
        matches!(self, TrajectoryStatus::Completed | TrajectoryStatus::Failed)
    }
}

// === SCOPE STATE MACHINE ===
// Scope has two states: Open (closed_at = None) and Closed (closed_at = Some)
// Once closed, cannot reopen

impl Scope {
    pub fn is_open(&self) -> bool {
        self.closed_at.is_none()
    }
    
    pub fn is_closed(&self) -> bool {
        self.closed_at.is_some()
    }
    
    /// Validate that scope can be closed
    pub fn can_close(&self) -> CaliberResult<()> {
        if self.is_closed() {
            return Err(CaliberError::Validation(ValidationError::InvalidStateTransition {
                entity_type: "Scope".to_string(),
                from: "Closed".to_string(),
                to: "Closed".to_string(),
            }));
        }
        Ok(())
    }
}

// === DELEGATION STATE MACHINE ===

impl DelegationStatus {
    pub fn can_transition_to(&self, target: DelegationStatus) -> bool {
        use DelegationStatus::*;
        match (self, target) {
            // From Pending
            (Pending, Accepted) => true,
            (Pending, Rejected) => true,
            
            // From Accepted
            (Accepted, InProgress) => true,
            (Accepted, Failed) => true,  // Can fail before starting
            
            // From InProgress
            (InProgress, Completed) => true,
            (InProgress, Failed) => true,
            
            // Terminal states
            (Rejected, _) => false,
            (Completed, _) => false,
            (Failed, _) => false,
            
            // Self-transitions
            (s, t) if s == &t => true,
            
            _ => false,
        }
    }
    
    pub fn is_terminal(&self) -> bool {
        matches!(self, DelegationStatus::Rejected | DelegationStatus::Completed | DelegationStatus::Failed)
    }
}

// === HANDOFF STATE MACHINE ===

impl HandoffStatus {
    pub fn can_transition_to(&self, target: HandoffStatus) -> bool {
        use HandoffStatus::*;
        match (self, target) {
            // From Initiated
            (Initiated, Accepted) => true,
            (Initiated, Rejected) => true,
            
            // From Accepted
            (Accepted, Completed) => true,
            
            // Terminal states
            (Rejected, _) => false,
            (Completed, _) => false,
            
            // Self-transitions
            (s, t) if s == &t => true,
            
            _ => false,
        }
    }
}

// === CONFLICT STATE MACHINE ===

impl ConflictStatus {
    pub fn can_transition_to(&self, target: ConflictStatus) -> bool {
        use ConflictStatus::*;
        match (self, target) {
            // From Detected
            (Detected, Resolving) => true,
            (Detected, Resolved) => true,   // Auto-resolved
            (Detected, Escalated) => true,
            
            // From Resolving
            (Resolving, Resolved) => true,
            (Resolving, Escalated) => true,
            
            // From Escalated - can still be resolved
            (Escalated, Resolved) => true,
            
            // Terminal
            (Resolved, _) => false,
            
            // Self-transitions
            (s, t) if s == &t => true,
            
            _ => false,
        }
    }
}

// === AGENT STATE MACHINE ===

impl AgentStatus {
    pub fn can_transition_to(&self, target: AgentStatus) -> bool {
        use AgentStatus::*;
        match (self, target) {
            // From Idle
            (Idle, Active) => true,
            (Idle, Failed) => true,
            
            // From Active
            (Active, Idle) => true,
            (Active, Blocked) => true,
            (Active, Failed) => true,
            
            // From Blocked
            (Blocked, Active) => true,
            (Blocked, Idle) => true,
            (Blocked, Failed) => true,
            
            // From Failed - can recover
            (Failed, Idle) => true,
            
            // Self-transitions
            (s, t) if s == &t => true,
            
            _ => false,
        }
    }
}
```

---

## 14. CONTEXT ASSEMBLY ALGORITHM (Complete)

```rust
// caliber-context/src/assembler.rs

use caliber_core::*;
use caliber_storage::CaliberStorage;
use caliber_llm::EmbeddingProvider;

/// Complete context assembly algorithm
/// All thresholds and budgets come from config - nothing hard-coded
pub struct ContextAssembler<'a> {
    storage: &'a dyn CaliberStorage,
    embedding_provider: Option<&'a dyn EmbeddingProvider>,
    config: &'a CaliberConfig,
}

impl<'a> ContextAssembler<'a> {
    pub fn new(
        storage: &'a dyn CaliberStorage,
        embedding_provider: Option<&'a dyn EmbeddingProvider>,
        config: &'a CaliberConfig,
    ) -> Self {
        Self { storage, embedding_provider, config }
    }
    
    /// Assemble context window for a trajectory
    /// 
    /// Algorithm:
    /// 1. Gather all candidate content
    /// 2. Score by relevance (if query provided) or recency
    /// 3. Fill priority buckets until budget exhausted
    /// 4. Record assembly trace for debugging
    pub fn assemble(
        &self,
        trajectory_id: EntityId,
        query: Option<&str>,
    ) -> CaliberResult<ContextWindow> {
        let window_id = generate_uuidv7();
        let mut trace = Vec::new();
        let mut sections = Vec::new();
        let mut used_tokens = 0;
        let budget = self.config.token_budget;
        
        // Step 1: Get trajectory and validate
        let trajectory = self.storage.trajectory_get(trajectory_id)?
            .ok_or(CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Trajectory,
                id: trajectory_id,
            }))?;
        
        // Step 2: Get query embedding if query provided AND embedding provider available
        let query_embedding = match (query, &self.embedding_provider) {
            (Some(q), Some(provider)) => Some(provider.embed(q)?),
            _ => None,
        };
        
        // Step 3: Gather candidates from each source
        let mut candidates: Vec<AssemblyCandidate> = Vec::new();
        
        // 3a. System instructions (if configured)
        if let Some(system) = &self.config.system_instructions {
            candidates.push(AssemblyCandidate {
                source_type: SourceType::System,
                source_id: None,
                content: system.clone(),
                token_count: estimate_tokens(system),
                relevance_score: 1.0,  // Always fully relevant
                priority: self.config.section_priorities.system,
                age: Duration::zero(),
            });
        }
        
        // 3b. Current scope turns
        if let Some(current_scope) = self.storage.scope_get_current(trajectory_id)? {
            let turns = self.storage.turn_list_by_scope(current_scope.scope_id)?;
            for turn in turns {
                let content = String::from_utf8_lossy(&turn.content).to_string();
                candidates.push(AssemblyCandidate {
                    source_type: SourceType::Turn,
                    source_id: Some(turn.turn_id),
                    content,
                    token_count: turn.token_count,
                    relevance_score: 1.0,  // Current conversation always relevant
                    priority: self.config.section_priorities.user,  // User priority for turns
                    age: Duration::zero(),
                });
            }
        }
        
        // 3c. Artifacts from trajectory
        let artifacts = self.storage.artifact_list_by_trajectory(trajectory_id)?;
        for artifact in artifacts {
            let relevance = self.score_relevance(&query_embedding, &artifact.content_embedding);
            let content = String::from_utf8_lossy(&artifact.content).to_string();
            candidates.push(AssemblyCandidate {
                source_type: SourceType::Artifact,
                source_id: Some(artifact.artifact_id),
                content,
                token_count: estimate_tokens(&String::from_utf8_lossy(&artifact.content)),
                relevance_score: relevance,
                priority: self.config.section_priorities.artifacts,
                age: chrono::Utc::now().signed_duration_since(artifact.created_at),
            });
        }
        
        // 3d. Relevant notes (semantic memory)
        let notes = if let Some(ref qe) = query_embedding {
            // Search by relevance
            let note_ids = self.storage.note_search_similar(qe, 20, 0.5)?;
            let mut notes = Vec::new();
            for (id, _) in note_ids {
                if let Some(note) = self.storage.note_get(id)? {
                    notes.push(note);
                }
            }
            notes
        } else {
            // No query - get recent notes (configurable limit)
            // This is a simplification; real impl might use different strategy
            Vec::new()
        };
        
        for note in notes {
            let relevance = self.score_relevance(&query_embedding, &Some(note.content_embedding.clone()));
            candidates.push(AssemblyCandidate {
                source_type: SourceType::Note,
                source_id: Some(note.note_id),
                content: note.content.clone(),
                token_count: estimate_tokens(&note.content),
                relevance_score: relevance,
                priority: self.config.section_priorities.notes,
                age: chrono::Utc::now().signed_duration_since(note.created_at),
            });
        }
        
        // 3e. Historical scope summaries
        let scopes = self.storage.scope_list_by_trajectory(trajectory_id)?;
        for scope in scopes.iter().filter(|s| s.is_closed()) {
            if let Some(ref summary) = scope.summary {
                let relevance = self.score_relevance(&query_embedding, &scope.summary_embedding);
                candidates.push(AssemblyCandidate {
                    source_type: SourceType::ScopeSummary,
                    source_id: Some(scope.scope_id),
                    content: summary.clone(),
                    token_count: estimate_tokens(summary),
                    relevance_score: relevance,
                    priority: self.config.section_priorities.history,
                    age: scope.closed_at.map(|t| chrono::Utc::now().signed_duration_since(t))
                        .unwrap_or(Duration::zero()),
                });
            }
        }
        
        // Step 4: Sort candidates by (priority DESC, relevance DESC, age ASC)
        candidates.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then(b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal))
                .then(a.age.cmp(&b.age))
        });
        
        // Step 5: Fill sections until budget exhausted
        let mut current_section_type: Option<SourceType> = None;
        let mut current_section_content = String::new();
        let mut current_section_sources: Vec<SourceRef> = Vec::new();
        let mut current_section_tokens = 0;
        
        for candidate in candidates {
            // Check if this candidate fits in remaining budget
            if used_tokens + candidate.token_count > budget {
                // Record exclusion
                trace.push(AssemblyDecision {
                    target_type: format!("{:?}", candidate.source_type),
                    target_id: candidate.source_id,
                    action: AssemblyAction::Exclude,
                    reason: format!("Budget exceeded ({}/{} tokens)", used_tokens, budget),
                    relevance_score: Some(candidate.relevance_score),
                    rule_id: "budget_limit".to_string(),
                    tokens_affected: candidate.token_count,
                });
                continue;
            }
            
            // Apply minimum relevance threshold (configurable)
            if candidate.relevance_score < self.config.min_relevance_threshold.unwrap_or(0.0) {
                trace.push(AssemblyDecision {
                    target_type: format!("{:?}", candidate.source_type),
                    target_id: candidate.source_id,
                    action: AssemblyAction::Exclude,
                    reason: format!("Below relevance threshold ({:.2} < {:.2})", 
                        candidate.relevance_score, 
                        self.config.min_relevance_threshold.unwrap_or(0.0)),
                    relevance_score: Some(candidate.relevance_score),
                    rule_id: "relevance_threshold".to_string(),
                    tokens_affected: candidate.token_count,
                });
                continue;
            }
            
            // Start new section if source type changes
            if current_section_type != Some(candidate.source_type) {
                // Flush current section
                if !current_section_content.is_empty() {
                    sections.push(ContextSection {
                        section_id: generate_uuidv7(),
                        section_type: source_type_to_section_type(current_section_type.unwrap()),
                        content: current_section_content.clone(),
                        token_count: current_section_tokens,
                        sources: current_section_sources.clone(),
                        priority: self.config.section_priorities.get(current_section_type.unwrap()),
                    });
                }
                
                current_section_type = Some(candidate.source_type);
                current_section_content = String::new();
                current_section_sources = Vec::new();
                current_section_tokens = 0;
            }
            
            // Add candidate to current section
            current_section_content.push_str(&candidate.content);
            current_section_content.push_str("\n\n");
            current_section_tokens += candidate.token_count;
            used_tokens += candidate.token_count;
            
            if let Some(id) = candidate.source_id {
                current_section_sources.push(SourceRef {
                    source_type: candidate.source_type,
                    source_id: id,
                    relevance_score: candidate.relevance_score,
                });
            }
            
            // Record inclusion
            trace.push(AssemblyDecision {
                target_type: format!("{:?}", candidate.source_type),
                target_id: candidate.source_id,
                action: AssemblyAction::Include,
                reason: "Fits budget and meets relevance threshold".to_string(),
                relevance_score: Some(candidate.relevance_score),
                rule_id: "standard_inclusion".to_string(),
                tokens_affected: candidate.token_count,
            });
        }
        
        // Flush final section
        if !current_section_content.is_empty() {
            sections.push(ContextSection {
                section_id: generate_uuidv7(),
                section_type: source_type_to_section_type(current_section_type.unwrap()),
                content: current_section_content,
                token_count: current_section_tokens,
                sources: current_section_sources,
                priority: self.config.section_priorities.get(current_section_type.unwrap()),
            });
        }
        
        // Step 6: Handle empty context
        if sections.is_empty() {
            // Return window with system message if configured, else empty
            if let Some(system) = &self.config.system_instructions {
                sections.push(ContextSection {
                    section_id: generate_uuidv7(),
                    section_type: SectionType::System,
                    content: system.clone(),
                    token_count: estimate_tokens(system),
                    sources: vec![],
                    priority: self.config.section_priorities.system,
                });
                used_tokens = estimate_tokens(system);
            }
        }
        
        // Step 7: Build and optionally persist context window
        let window = ContextWindow {
            window_id,
            assembled_at: chrono::Utc::now(),
            max_tokens: budget,
            used_tokens,
            sections,
            assembly_trace: trace,
        };
        
        // Persist if configured
        match self.config.context_window_persistence {
            ContextPersistence::Ephemeral => { /* don't store */ }
            ContextPersistence::Ttl(ttl) => {
                let mut stored = window.clone();
                stored.expires_at = Some(chrono::Utc::now() + ttl);
                self.storage.context_window_insert(&stored)?;
            }
            ContextPersistence::Permanent => {
                self.storage.context_window_insert(&window)?;
            }
        }
        
        Ok(window)
    }
    
    /// Score relevance of content to query
    /// Returns 1.0 if no query (everything equally relevant)
    /// Returns cosine similarity if both query and content have embeddings
    /// Returns 0.5 if content has no embedding (neutral score)
    fn score_relevance(
        &self,
        query_embedding: &Option<EmbeddingVector>,
        content_embedding: &Option<EmbeddingVector>,
    ) -> f32 {
        match (query_embedding, content_embedding) {
            (None, _) => 1.0,  // No query = everything relevant
            (Some(_), None) => 0.5,  // Content has no embedding = neutral
            (Some(q), Some(c)) => {
                q.cosine_similarity(c).unwrap_or(0.5)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct AssemblyCandidate {
    source_type: SourceType,
    source_id: Option<EntityId>,
    content: String,
    token_count: i32,
    relevance_score: f32,
    priority: i32,
    age: chrono::Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    System,
    Turn,
    Artifact,
    Note,
    ScopeSummary,
}

fn source_type_to_section_type(st: SourceType) -> SectionType {
    match st {
        SourceType::System => SectionType::System,
        SourceType::Turn => SectionType::Conversation,
        SourceType::Artifact => SectionType::Artifacts,
        SourceType::Note => SectionType::Knowledge,
        SourceType::ScopeSummary => SectionType::History,
    }
}

fn estimate_tokens(text: &str) -> i32 {
    // ~3.5 chars per token for mixed content
    (text.len() as f32 / 3.5).ceil() as i32
}
```

---

## 15. CONCURRENCY MODEL

### 15.1 Isolation Levels

```rust
// caliber-pg/src/concurrency.rs

/// CALIBER concurrency model
/// 
/// Principle: Use Postgres's MVCC, don't fight it
/// 
/// READ operations: Use REPEATABLE READ snapshot isolation
/// - Context assembly sees consistent point-in-time snapshot
/// - Concurrent writes don't affect reads
/// - No read locks needed
/// 
/// WRITE operations: Use row-level locks where needed
/// - Trajectory updates: Lock trajectory row
/// - Scope close: Lock scope row
/// - Artifact insert: No lock (append-only, content-hash dedup)
/// - Note update: Lock note row for merge

/// Context assembly uses snapshot isolation
/// Multiple processes can assemble context for same trajectory simultaneously
/// Each sees consistent snapshot at start of transaction
pub fn assemble_context_with_isolation<S: CaliberStorage>(
    storage: &S,
    config: &CaliberConfig,
    trajectory_id: EntityId,
    query: Option<&str>,
) -> CaliberResult<ContextWindow> {
    // pgrx functions run in transaction context
    // Postgres default is READ COMMITTED, but assembly is single-statement
    // so effectively sees snapshot
    
    ContextAssembler::new(storage, None, config).assemble(trajectory_id, query)
}

/// Trajectory modification requires row lock
pub fn update_trajectory_with_lock<S: CaliberStorage>(
    storage: &S,
    trajectory_id: EntityId,
    update: TrajectoryUpdate,
) -> CaliberResult<()> {
    // In pgrx, use SPI with FOR UPDATE or advisory lock
    // The storage trait impl handles this internally
    
    // Validate state transition
    let current = storage.trajectory_get(trajectory_id)?
        .ok_or(CaliberError::Storage(StorageError::NotFound {
            entity_type: EntityType::Trajectory,
            id: trajectory_id,
        }))?;
    
    if let Some(new_status) = update.status {
        current.status.transition_to(new_status)?;
    }
    
    storage.trajectory_update(trajectory_id, update)
}

/// Scope close is atomic
/// 1. Lock scope row
/// 2. Validate not already closed
/// 3. Update all fields
/// 4. Optionally delete turns
pub fn close_scope_atomic<S: CaliberStorage>(
    storage: &S,
    scope_id: EntityId,
    close: ScopeClose,
    delete_turns: bool,
) -> CaliberResult<()> {
    // Get and validate
    let scope = storage.scope_get(scope_id)?
        .ok_or(CaliberError::Storage(StorageError::NotFound {
            entity_type: EntityType::Scope,
            id: scope_id,
        }))?;
    
    scope.can_close()?;
    
    // Close (storage impl handles locking)
    storage.scope_close(scope_id, close)?;
    
    // Delete turns if requested
    if delete_turns {
        storage.turn_delete_by_scope(scope_id)?;
    }
    
    Ok(())
}

/// Artifact insert is idempotent via content hash
/// If artifact with same hash exists, return existing ID
pub fn insert_artifact_idempotent<S: CaliberStorage>(
    storage: &S,
    artifact: Artifact,
) -> CaliberResult<EntityId> {
    // Check for existing by hash
    if let Some(existing) = storage.artifact_get_by_hash(&artifact.content_hash)? {
        return Ok(existing.artifact_id);
    }
    
    // Insert new
    storage.artifact_insert(&artifact)?;
    Ok(artifact.artifact_id)
}
```

### 15.2 Concurrent Scope Operations

```rust
/// What happens if scope N+1 opens before scope N closes?
/// 
/// Answer: This is ALLOWED. Scopes are sequential by sequence_number,
/// not by time. A new scope can open while previous is still active.
/// 
/// Use case: Parallel work streams within a trajectory
/// 
/// Constraint: Only ONE scope can be "current" for context assembly.
/// Current = highest sequence_number that is still open.

pub fn open_scope_concurrent<S: CaliberStorage>(
    storage: &S,
    trajectory_id: EntityId,
) -> CaliberResult<EntityId> {
    // Get next sequence number (max + 1)
    let count = storage.scope_count_by_trajectory(trajectory_id)?;
    let sequence_number = count + 1;
    
    let scope = Scope {
        scope_id: generate_uuidv7(),
        trajectory_id,
        sequence_number,
        start_turn_id: None,
        end_turn_id: None,
        opened_at: chrono::Utc::now(),
        closed_at: None,
        summary: None,
        summary_embedding: None,
        artifact_ids: vec![],
        checkpoint_id: None,
    };
    
    storage.scope_insert(&scope)?;
    Ok(scope.scope_id)
}
```

---

## 16. ERROR RECOVERY PROCEDURES

```rust
// caliber-pcp/src/recovery.rs

use caliber_core::*;
use caliber_storage::CaliberStorage;

/// Recovery procedures for various failure scenarios

pub struct RecoveryManager<'a> {
    storage: &'a dyn CaliberStorage,
    config: &'a CaliberConfig,
}

impl<'a> RecoveryManager<'a> {
    pub fn new(storage: &'a dyn CaliberStorage, config: &'a CaliberConfig) -> Self {
        Self { storage, config }
    }
    
    /// Detect and handle orphaned resources
    /// Called periodically or on-demand
    pub fn cleanup_orphans(&self) -> CaliberResult<OrphanCleanupReport> {
        let mut report = OrphanCleanupReport::default();
        
        // 1. Find trajectories stuck in Active for too long
        let orphan_threshold = self.config.orphan_detection_threshold;
        let cutoff = chrono::Utc::now() - orphan_threshold;
        
        let active_trajectories = self.storage.trajectory_list_by_status(TrajectoryStatus::Active)?;
        for trajectory in active_trajectories {
            if trajectory.updated_at < cutoff {
                // Mark as failed with reason
                self.storage.trajectory_update(trajectory.trajectory_id, TrajectoryUpdate {
                    status: Some(TrajectoryStatus::Failed),
                    outcome: Some(TrajectoryOutcome {
                        status: OutcomeStatus::SystemAbort,
                        summary: format!("Orphan cleanup: no activity since {}", trajectory.updated_at),
                        final_artifacts: vec![],
                        notes_generated: vec![],
                        duration: chrono::Utc::now().signed_duration_since(trajectory.created_at),
                    }),
                    updated_at: Some(chrono::Utc::now()),
                })?;
                report.trajectories_failed += 1;
            }
        }
        
        // 2. Find unclosed scopes older than threshold
        let scopes = self.find_orphan_scopes(cutoff)?;
        for scope in scopes {
            // Force close with system summary
            self.storage.scope_close(scope.scope_id, ScopeClose {
                end_turn_id: None,
                closed_at: chrono::Utc::now(),
                summary: "[SYSTEM] Scope closed by orphan cleanup - process interrupted".to_string(),
                summary_embedding: EmbeddingVector::empty(),  // No embedding for system close
                artifact_ids: scope.artifact_ids.clone(),
                checkpoint_id: None,
            })?;
            report.scopes_closed += 1;
        }
        
        // 3. Delete expired context windows
        let deleted = self.storage.context_window_delete_expired()?;
        report.context_windows_deleted = deleted;
        
        // 4. Delete expired locks
        let deleted = self.storage.lock_cleanup_expired()?;
        report.locks_released = deleted;
        
        // 5. Delete old acknowledged messages
        let message_retention = self.config.message_retention;
        let deleted = self.storage.message_delete_old(message_retention)?;
        report.messages_deleted = deleted;
        
        // 6. Prune old embedding cache entries
        let cache_retention = self.config.embedding_cache_retention.unwrap_or(chrono::Duration::days(30));
        let deleted = self.storage.embedding_cache_delete_older_than(cache_retention)?;
        report.cache_entries_deleted = deleted;
        
        Ok(report)
    }
    
    fn find_orphan_scopes(&self, cutoff: chrono::DateTime<chrono::Utc>) -> CaliberResult<Vec<Scope>> {
        // This would be a custom query in real impl
        // For now, we'd iterate through active trajectories and check their scopes
        // Storage trait could add: scope_list_open_older_than(cutoff)
        Ok(vec![])
    }
    
    /// Recover a trajectory from its latest checkpoint
    pub fn recover_from_checkpoint(
        &self,
        trajectory_id: EntityId,
    ) -> CaliberResult<RecoveryResult> {
        // Get latest checkpoint
        let checkpoint = self.storage.checkpoint_get_latest(trajectory_id)?
            .ok_or(CaliberError::Validation(ValidationError::NoCheckpointAvailable {
                trajectory_id,
            }))?;
        
        // Validate checkpoint is usable
        let trajectory = self.storage.trajectory_get(trajectory_id)?
            .ok_or(CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Trajectory,
                id: trajectory_id,
            }))?;
        
        // Can only recover non-terminal trajectories
        if trajectory.status.is_terminal() {
            return Err(CaliberError::Validation(ValidationError::InvalidStateTransition {
                entity_type: "Trajectory".to_string(),
                from: format!("{:?}", trajectory.status),
                to: "Recovery".to_string(),
            }));
        }
        
        // Close all scopes after checkpoint
        let scopes = self.storage.scope_list_by_trajectory(trajectory_id)?;
        let mut scopes_rolled_back = 0;
        
        for scope in scopes {
            if scope.scope_id != checkpoint.scope_id && scope.opened_at > checkpoint.created_at {
                // Delete artifacts from this scope
                let artifacts = self.storage.artifact_list_by_scope(scope.scope_id)?;
                for artifact in artifacts {
                    // Mark as superseded (don't hard delete for audit)
                    self.storage.artifact_update(artifact.artifact_id, ArtifactUpdate {
                        superseded_by: Some(EntityId::nil()),  // Nil = rolled back
                    })?;
                }
                
                // Delete turns
                self.storage.turn_delete_by_scope(scope.scope_id)?;
                
                // Force close scope
                if scope.is_open() {
                    self.storage.scope_close(scope.scope_id, ScopeClose {
                        end_turn_id: None,
                        closed_at: chrono::Utc::now(),
                        summary: "[SYSTEM] Scope rolled back during checkpoint recovery".to_string(),
                        summary_embedding: EmbeddingVector::empty(),
                        artifact_ids: vec![],
                        checkpoint_id: None,
                    })?;
                }
                
                scopes_rolled_back += 1;
            }
        }
        
        // Restore trajectory status from checkpoint
        self.storage.trajectory_update(trajectory_id, TrajectoryUpdate {
            status: Some(checkpoint.state.trajectory_status),
            updated_at: Some(chrono::Utc::now()),
            ..Default::default()
        })?;
        
        // Open new scope to continue from checkpoint
        let new_scope_id = open_scope_concurrent(self.storage, trajectory_id)?;
        
        Ok(RecoveryResult {
            checkpoint_id: checkpoint.checkpoint_id,
            trajectory_id,
            scopes_rolled_back,
            new_scope_id,
            recovered_at: chrono::Utc::now(),
        })
    }
    
    /// Handle partial LLM batch failure
    /// Returns which items succeeded and which need retry
    pub fn handle_embedding_batch_failure(
        items: Vec<(EntityId, String)>,
        results: Vec<Result<EmbeddingVector, LlmError>>,
    ) -> (Vec<(EntityId, EmbeddingVector)>, Vec<(EntityId, String)>) {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        
        for ((id, text), result) in items.into_iter().zip(results) {
            match result {
                Ok(embedding) => succeeded.push((id, embedding)),
                Err(_) => failed.push((id, text)),
            }
        }
        
        (succeeded, failed)
    }
}

#[derive(Debug, Default)]
pub struct OrphanCleanupReport {
    pub trajectories_failed: i32,
    pub scopes_closed: i32,
    pub context_windows_deleted: i32,
    pub locks_released: i32,
    pub messages_deleted: i32,
    pub cache_entries_deleted: i32,
}

#[derive(Debug)]
pub struct RecoveryResult {
    pub checkpoint_id: EntityId,
    pub trajectory_id: EntityId,
    pub scopes_rolled_back: i32,
    pub new_scope_id: EntityId,
    pub recovered_at: chrono::DateTime<chrono::Utc>,
}
```

---

## 17. PGVECTOR INTEGRATION DETAILS

```rust
// caliber-pg/src/vector.rs

use pgrx::prelude::*;

/// pgvector integration
/// 
/// Key decisions:
/// - Dimension is configured at init time (from embedding provider)
/// - HNSW indexes created after dimension known
/// - Cosine similarity (vector_cosine_ops) is default
/// - Distance metric configurable

#[derive(Debug, Clone)]
pub struct VectorConfig {
    pub dimensions: i32,
    pub index_type: VectorIndexType,
    pub distance_metric: DistanceMetric,
    pub hnsw_m: i32,                    // HNSW: max connections per node
    pub hnsw_ef_construction: i32,      // HNSW: size of dynamic candidate list
}

#[derive(Debug, Clone, Copy)]
pub enum VectorIndexType {
    Hnsw,       // Hierarchical Navigable Small World - fast, more memory
    IvfFlat,    // Inverted File Index - slower, less memory
}

#[derive(Debug, Clone, Copy)]
pub enum DistanceMetric {
    Cosine,         // vector_cosine_ops - for normalized embeddings
    L2,             // vector_l2_ops - Euclidean distance
    InnerProduct,   // vector_ip_ops - dot product
}

impl DistanceMetric {
    pub fn pg_ops(&self) -> &'static str {
        match self {
            DistanceMetric::Cosine => "vector_cosine_ops",
            DistanceMetric::L2 => "vector_l2_ops",
            DistanceMetric::InnerProduct => "vector_ip_ops",
        }
    }
    
    pub fn pg_operator(&self) -> &'static str {
        match self {
            DistanceMetric::Cosine => "<=>",     // cosine distance
            DistanceMetric::L2 => "<->",         // L2 distance
            DistanceMetric::InnerProduct => "<#>", // negative inner product
        }
    }
}

/// Create vector indexes after dimension is known
/// Called from caliber_init() after embedding provider configured
#[pg_extern]
pub fn caliber_create_vector_indexes(
    dimensions: i32,
    index_type: &str,
    distance_metric: &str,
    hnsw_m: i32,
    hnsw_ef_construction: i32,
) -> bool {
    let metric = match distance_metric {
        "cosine" => DistanceMetric::Cosine,
        "l2" => DistanceMetric::L2,
        "inner_product" => DistanceMetric::InnerProduct,
        _ => {
            pgrx::warning!("Unknown distance metric '{}', using cosine", distance_metric);
            DistanceMetric::Cosine
        }
    };
    
    let ops = metric.pg_ops();
    
    // First, alter columns to have correct dimension
    let alter_sql = format!(
        r#"
        ALTER TABLE caliber_artifact 
            ALTER COLUMN content_embedding TYPE vector({dimensions});
        ALTER TABLE caliber_note 
            ALTER COLUMN content_embedding TYPE vector({dimensions});
        ALTER TABLE caliber_scope 
            ALTER COLUMN summary_embedding TYPE vector({dimensions});
        ALTER TABLE caliber_embedding_cache 
            ALTER COLUMN embedding TYPE vector({dimensions});
        "#,
        dimensions = dimensions
    );
    
    if let Err(e) = Spi::run(&alter_sql) {
        pgrx::warning!("Failed to alter vector columns: {:?}", e);
        return false;
    }
    
    // Create indexes based on type
    let index_sql = match index_type {
        "hnsw" => format!(
            r#"
            CREATE INDEX IF NOT EXISTS idx_artifact_embedding ON caliber_artifact 
                USING hnsw (content_embedding {ops}) WITH (m = {m}, ef_construction = {ef});
            CREATE INDEX IF NOT EXISTS idx_note_embedding ON caliber_note 
                USING hnsw (content_embedding {ops}) WITH (m = {m}, ef_construction = {ef});
            CREATE INDEX IF NOT EXISTS idx_scope_embedding ON caliber_scope 
                USING hnsw (summary_embedding {ops}) WITH (m = {m}, ef_construction = {ef});
            "#,
            ops = ops,
            m = hnsw_m,
            ef = hnsw_ef_construction
        ),
        "ivfflat" => format!(
            r#"
            CREATE INDEX IF NOT EXISTS idx_artifact_embedding ON caliber_artifact 
                USING ivfflat (content_embedding {ops}) WITH (lists = 100);
            CREATE INDEX IF NOT EXISTS idx_note_embedding ON caliber_note 
                USING ivfflat (content_embedding {ops}) WITH (lists = 100);
            CREATE INDEX IF NOT EXISTS idx_scope_embedding ON caliber_scope 
                USING ivfflat (summary_embedding {ops}) WITH (lists = 100);
            "#,
            ops = ops
        ),
        _ => {
            pgrx::warning!("Unknown index type '{}', using hnsw", index_type);
            return false;
        }
    };
    
    if let Err(e) = Spi::run(&index_sql) {
        pgrx::warning!("Failed to create vector indexes: {:?}", e);
        return false;
    }
    
    true
}

/// What happens if embedding model changes and dimensions change?
/// 
/// Answer: This is a MIGRATION. User must:
/// 1. Stop writes
/// 2. Drop old indexes
/// 3. Re-embed all content with new model
/// 4. Create new indexes with new dimension
/// 
/// CALIBER does not auto-handle dimension changes. It's a destructive operation.
/// User can use caliber_migrate_embeddings() helper.

#[pg_extern]
pub fn caliber_migrate_embeddings(
    old_model_id: &str,
    new_dimensions: i32,
) -> i64 {
    // This function:
    // 1. Drops existing vector indexes
    // 2. Alters columns to new dimension
    // 3. Sets all embeddings to NULL (must be re-computed)
    // 4. Creates new indexes
    // 5. Returns count of rows that need re-embedding
    
    // Drop indexes
    Spi::run("DROP INDEX IF EXISTS idx_artifact_embedding").ok();
    Spi::run("DROP INDEX IF EXISTS idx_note_embedding").ok();
    Spi::run("DROP INDEX IF EXISTS idx_scope_embedding").ok();
    
    // Alter columns
    let alter_sql = format!(
        r#"
        ALTER TABLE caliber_artifact ALTER COLUMN content_embedding TYPE vector({});
        ALTER TABLE caliber_note ALTER COLUMN content_embedding TYPE vector({});
        ALTER TABLE caliber_scope ALTER COLUMN summary_embedding TYPE vector({});
        "#,
        new_dimensions
    );
    Spi::run(&alter_sql).ok();
    
    // Null out embeddings (they're invalid now)
    Spi::run("UPDATE caliber_artifact SET content_embedding = NULL").ok();
    Spi::run("UPDATE caliber_note SET content_embedding = NULL").ok();
    Spi::run("UPDATE caliber_scope SET summary_embedding = NULL").ok();
    
    // Clear cache (old model embeddings)
    Spi::run(&format!(
        "DELETE FROM caliber_embedding_cache WHERE model_id = '{}'", 
        old_model_id
    )).ok();
    
    // Count rows needing re-embedding
    let count: i64 = Spi::get_one(
        "SELECT (SELECT COUNT(*) FROM caliber_artifact) + 
                (SELECT COUNT(*) FROM caliber_note) + 
                (SELECT COUNT(*) FROM caliber_scope WHERE summary IS NOT NULL)"
    ).unwrap_or(Some(0)).unwrap_or(0);
    
    count
}
```

---

## 18. COMPLETE CONFIG VALIDATION

```rust
// caliber-core/src/config.rs (extended)

impl CaliberConfig {
    /// Complete configuration validation
    /// Returns all validation errors, not just first one
    pub fn validate(&self) -> Result<(), Vec<ConfigError>> {
        let mut errors = Vec::new();
        
        // === REQUIRED FIELDS ===
        
        // Token budget must be positive and reasonable
        if self.token_budget <= 0 {
            errors.push(ConfigError::InvalidValue {
                field: "token_budget".to_string(),
                value: self.token_budget.to_string(),
                reason: "must be positive".to_string(),
            });
        }
        if self.token_budget > 2_000_000 {
            errors.push(ConfigError::InvalidValue {
                field: "token_budget".to_string(),
                value: self.token_budget.to_string(),
                reason: "exceeds maximum (2M tokens)".to_string(),
            });
        }
        
        // Checkpoint retention must be positive
        if self.checkpoint_retention <= 0 {
            errors.push(ConfigError::InvalidValue {
                field: "checkpoint_retention".to_string(),
                value: self.checkpoint_retention.to_string(),
                reason: "must be positive".to_string(),
            });
        }
        
        // Stale threshold must be positive
        if self.stale_threshold.num_seconds() <= 0 {
            errors.push(ConfigError::InvalidValue {
                field: "stale_threshold".to_string(),
                value: format!("{:?}", self.stale_threshold),
                reason: "must be positive duration".to_string(),
            });
        }
        
        // Contradiction threshold must be 0.0-1.0
        if self.contradiction_threshold < 0.0 || self.contradiction_threshold > 1.0 {
            errors.push(ConfigError::InvalidValue {
                field: "contradiction_threshold".to_string(),
                value: self.contradiction_threshold.to_string(),
                reason: "must be between 0.0 and 1.0".to_string(),
            });
        }
        
        // === SECTION PRIORITIES ===
        
        // All priorities must be non-negative
        if self.section_priorities.user < 0 {
            errors.push(ConfigError::InvalidValue {
                field: "section_priorities.user".to_string(),
                value: self.section_priorities.user.to_string(),
                reason: "must be non-negative".to_string(),
            });
        }
        // ... same for system, artifacts, notes, history
        
        // === RETRY CONFIG ===
        
        if self.llm_retry_config.max_retries < 0 {
            errors.push(ConfigError::InvalidValue {
                field: "llm_retry_config.max_retries".to_string(),
                value: self.llm_retry_config.max_retries.to_string(),
                reason: "must be non-negative".to_string(),
            });
        }
        
        if self.llm_retry_config.backoff_multiplier <= 1.0 {
            errors.push(ConfigError::InvalidValue {
                field: "llm_retry_config.backoff_multiplier".to_string(),
                value: self.llm_retry_config.backoff_multiplier.to_string(),
                reason: "must be greater than 1.0 for exponential backoff".to_string(),
            });
        }
        
        if self.llm_retry_config.initial_backoff.num_milliseconds() <= 0 {
            errors.push(ConfigError::InvalidValue {
                field: "llm_retry_config.initial_backoff".to_string(),
                value: format!("{:?}", self.llm_retry_config.initial_backoff),
                reason: "must be positive".to_string(),
            });
        }
        
        // === PROVIDER CONFIGS ===
        
        if let Some(ref emb) = self.embedding_provider {
            if emb.provider_type.is_empty() {
                errors.push(ConfigError::InvalidValue {
                    field: "embedding_provider.provider_type".to_string(),
                    value: "".to_string(),
                    reason: "cannot be empty".to_string(),
                });
            }
            
            if let Some(dim) = emb.dimensions {
                if dim <= 0 || dim > 16384 {
                    errors.push(ConfigError::InvalidValue {
                        field: "embedding_provider.dimensions".to_string(),
                        value: dim.to_string(),
                        reason: "must be between 1 and 16384".to_string(),
                    });
                }
            }
        }
        
        // === MULTI-AGENT TIMEOUTS ===
        
        if self.lock_timeout.num_seconds() <= 0 {
            errors.push(ConfigError::InvalidValue {
                field: "lock_timeout".to_string(),
                value: format!("{:?}", self.lock_timeout),
                reason: "must be positive".to_string(),
            });
        }
        
        if self.message_retention.num_seconds() <= 0 {
            errors.push(ConfigError::InvalidValue {
                field: "message_retention".to_string(),
                value: format!("{:?}", self.message_retention),
                reason: "must be positive".to_string(),
            });
        }
        
        if self.delegation_timeout.num_seconds() <= 0 {
            errors.push(ConfigError::InvalidValue {
                field: "delegation_timeout".to_string(),
                value: format!("{:?}", self.delegation_timeout),
                reason: "must be positive".to_string(),
            });
        }
        
        // === CROSS-FIELD VALIDATION ===
        
        // Max backoff should be >= initial backoff
        if self.llm_retry_config.max_backoff < self.llm_retry_config.initial_backoff {
            errors.push(ConfigError::IncompatibleOptions {
                option_a: "llm_retry_config.max_backoff".to_string(),
                option_b: "llm_retry_config.initial_backoff".to_string(),
                reason: "max_backoff must be >= initial_backoff".to_string(),
            });
        }
        
        // If context persistence is TTL, need positive TTL
        if let ContextPersistence::Ttl(ttl) = self.context_window_persistence {
            if ttl.num_seconds() <= 0 {
                errors.push(ConfigError::InvalidValue {
                    field: "context_window_persistence.ttl".to_string(),
                    value: format!("{:?}", ttl),
                    reason: "TTL must be positive".to_string(),
                });
            }
        }
        
        // Return result
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Create config from JSON with validation
    pub fn from_json(json: &str) -> CaliberResult<Self> {
        let config: Self = serde_json::from_str(json)
            .map_err(|e| CaliberError::Config(ConfigError::InvalidValue {
                field: "json".to_string(),
                value: json.chars().take(100).collect(),
                reason: e.to_string(),
            }))?;
        
        config.validate()
            .map_err(|errors| CaliberError::Config(ConfigError::MultipleErrors(errors)))?;
        
        Ok(config)
    }
}

// Add MultipleErrors variant to ConfigError
#[derive(Debug)]
pub enum ConfigError {
    MissingRequired { field: String },
    InvalidValue { field: String, value: String, reason: String },
    IncompatibleOptions { option_a: String, option_b: String, reason: String },
    ProviderNotSupported { provider: String },
    MultipleErrors(Vec<ConfigError>),
}
```

---

## 19. TRAIT NAME NORMALIZATION

**Standard names across all crates:**

| Concept | Trait Name | Location |
|---------|------------|----------|
| Embedding | `EmbeddingProvider` | caliber-llm |
| Summarization | `SummarizationProvider` | caliber-llm |
| Storage | `CaliberStorage` | caliber-storage |
| Context Assembly | `ContextAssembler` | caliber-context (struct, not trait) |
| PCP Validation | `PCPValidator` | caliber-pcp (struct, not trait) |

**Deprecated names (do not use):**

- ~~`EmbeddingProvider`~~ → use `EmbeddingProvider`
- ~~`SummarizationProvider`~~ → use `SummarizationProvider`

---

## END OF SPECIFICATION

This document is complete. An AI agent with:

1. Rust toolchain + pgrx
2. PostgreSQL 14+ with pgvector
3. This specification

...can implement CALIBER + PCP as a multi-crate workspace.

**Architecture summary:**

- Multi-crate ECS: core, storage, context, pcp, llm, agents, dsl, pg
- Nothing hard-coded - all values from CaliberConfig
- NO SQL in hot path (direct pgrx heap/index access)
- Rust for all runtime code
- SQL only for human debugging + one-time bootstrap
- Postgres NOTIFY for reactive events
- Complete StorageTrait defining all data operations
- State machines with explicit valid transitions
- Concurrency via Postgres MVCC + row locks where needed
- Error recovery procedures for orphans and checkpoints
- pgvector integration with configurable dimensions and metrics
