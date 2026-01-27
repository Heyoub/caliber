# CALIBER Quick Reference

**Cross-crate reference sheet for implementers.**

## ⚠️ Critical: Preset-First, No Implicit Defaults

CALIBER is a framework. Every value is configured by the user.
Presets are explicit and validated; overrides are opt-in.

```rust
// Config is REQUIRED - no implicit defaults
let config = CaliberConfig { /* user provides all values */ };
```

Keep presets explicit and tracked alongside config changes.

---

## Multi-Crate Architecture

```
caliber-core/        # Entities (data only)
caliber-storage/     # Storage trait + pgrx
caliber-pcp/         # Validation, checkpoints
caliber-dsl/         # DSL → config (separate)
caliber-pg/          # pgrx extension (runtime)
```

---

## Memory Hierarchy

```
Trajectory (task container)
├── Scope (context partition)
│   ├── Turn (ephemeral) {turn_id, scope_id, role, content, token_count, model_id}
│   └── Artifact (preserved output)
└── Note (cross-trajectory knowledge)
```

## Memory Types

| Type | Retention | Use Case |
|------|-----------|----------|
| `Ephemeral` | Session/scope | Turn buffer |
| `Working` | Bounded | Active scope |
| `Episodic` | Configurable | Artifacts |
| `Semantic` | Long-lived | Notes |
| `Procedural` | Persistent | Procedures |
| `Meta` | Persistent | Trajectories |

---

## Error Types (Single Source of Truth)

```rust
pub enum CaliberError {
    Storage(StorageError),
    Llm(LlmError),
    Validation(ValidationError),
    Config(ConfigError),
    Vector(VectorError),
    Agent(AgentError),
}
```

All functions return `CaliberResult<T>`. Errors propagate to Postgres via ereport.

---

## VAL: Vector Abstraction Layer

Dynamic dimensions, provider-agnostic:

```rust
pub struct EmbeddingVector {
    pub data: Vec<f32>,      // Any dimension
    pub model_id: String,
    pub dimensions: i32,
}

pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector>;
    fn dimensions(&self) -> i32;
}
```

---

## Configuration (ALL Required)

```rust
pub struct CaliberConfig {
    // Context
    pub token_budget: i32,
    pub section_priorities: SectionPriorities,
    
    // PCP
    pub checkpoint_retention: i32,
    pub stale_threshold: Duration,
    pub contradiction_threshold: f32,
    
    // Storage
    pub context_window_persistence: ContextPersistence,
    pub validation_mode: ValidationMode,
    
    // LLM (optional but required if using embeddings)
    pub embedding_provider: Option<ProviderConfig>,
    pub summarization_provider: Option<ProviderConfig>,
    pub llm_retry_config: RetryConfig,
    
    // Multi-agent
    pub lock_timeout: Duration,
    pub message_retention: Duration,
    pub delegation_timeout: Duration,
}
```

---

## Core Functions

```rust
// All take &CaliberConfig - no globals, no defaults

caliber_trajectory_insert(&config, ...) -> CaliberResult<Uuid>
caliber_scope_insert(&config, ...) -> CaliberResult<Uuid>
caliber_artifact_insert(&config, ...) -> CaliberResult<Uuid>
caliber_note_insert(&config, ...) -> CaliberResult<Uuid>

caliber_vector_search(&config, ...) -> CaliberResult<Vec<(Uuid, f32)>>

CaliberOrchestrator::assemble_context(&config, ...) -> CaliberResult<ContextWindow>
PCPRuntime::validate_context_integrity(&config, ...) -> CaliberResult<ValidationResult>
```

---

## Build (Multi-Crate Workspace)

```bash
# Build all crates
cargo build --workspace

# Build just the extension
cargo build -p caliber-pg --release

# Package for Postgres
cargo pgrx package -p caliber-pg

# Install into Postgres (requires permissions to write extension files)
cargo pgrx install --package caliber-pg --pg-config "/usr/lib/postgresql/18/bin/pg_config"

# Enable extensions
psql -c "CREATE EXTENSION vector;"
psql -c "CREATE EXTENSION caliber_pg;"

# Run tests (non-pgrx)
TMPDIR=$PWD/target/tmp cargo test --workspace --exclude caliber-pg

# Run extension tests
cargo pgrx test pg18 --package caliber-pg
```

---

## Initialize / Configure

Initialization happens at extension install:

```sql
CREATE EXTENSION caliber_pg;
```

Configuration is applied via the DSL pipeline (REST/gRPC), not `caliber_init()`.
