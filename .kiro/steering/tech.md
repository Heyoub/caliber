# CALIBER Technical Standards

## Language & Runtime

- **Primary Language:** Rust
- **Database:** PostgreSQL via pgrx extension
- **DSL:** Custom CALIBER DSL (see docs/DSL_PARSER.md)

## Development Philosophy

**NO STUBS. NO TODOs. COMPLETE CODE ONLY.**

See `.kiro/steering/dev-philosophy.md` for full rationale.

- Every source file has real, working code from creation
- Reference `docs/DEPENDENCY_GRAPH.md` for all type definitions
- Create Cargo.toml files first, lib.rs only when implementing
- Run cargo check ONCE after all code is written, not incrementally

## Multi-Crate Architecture

```text
caliber-core/        # ENTITIES: Data structures only
caliber-storage/     # COMPONENT: Storage trait + pgrx
caliber-context/     # COMPONENT: Context assembly logic
caliber-pcp/         # COMPONENT: Validation, checkpoints, recovery
caliber-llm/         # COMPONENT: VAL (Vector Abstraction Layer)
caliber-agents/      # COMPONENT: Multi-agent coordination
caliber-dsl/         # SYSTEM: DSL parser → CaliberConfig
caliber-pg/          # SYSTEM: pgrx extension (runtime)
```

## Code Standards

### Rust Conventions

- Use `CaliberResult<T>` for all fallible operations
- Errors propagate to Postgres via ereport
- No unwrap() in production code — use `?` operator
- All config values explicit — no defaults

### Error Handling

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

### Configuration Philosophy

```rust
// WRONG - Hard-coded defaults
const DEFAULT_TOKEN_BUDGET: i32 = 8000;

// RIGHT - User MUST configure
pub struct CaliberConfig {
    pub token_budget: i32,  // Required, no default
}
```

## Key Types

- `EntityId` = UUIDv7 (timestamp-sortable)
- `EmbeddingVector` = Dynamic `Vec<f32>` (any dimension)
- `RawContent` = `Vec<u8>` (BYTEA)
- `ContentHash` = `[u8; 32]` (SHA-256)

## Build Commands

```bash
cargo build --workspace
cargo build -p caliber-pg --release
cargo pgrx package -p caliber-pg
```
