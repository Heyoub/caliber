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
# Standard build workflow
cargo build --workspace
cargo clippy --workspace -- -D warnings  # REQUIRED before marking complete
cargo test --workspace

# Package-specific builds
cargo build -p caliber-pg --release
cargo pgrx package -p caliber-pg

# Property test verification
cargo test --workspace -- --test-threads=1  # For property tests
```

## Code Quality Standards

### Verification Gates

**CRITICAL:** All code must pass these gates before marking complete:

1. **Build Gate:** `cargo build --workspace` succeeds
2. **Clippy Gate:** `cargo clippy --workspace -- -D warnings` succeeds (ZERO warnings)
3. **Test Gate:** `cargo test --workspace` succeeds (ALL tests pass)
4. **Property Test Gate:** All property tests run 100+ iterations

### Error Handling Standards

```rust
// ❌ WRONG - Panics in production
let value = env::var("CONFIG").expect("CONFIG must be set");
let result = operation().unwrap();

// ✅ RIGHT - Returns CaliberResult
let value = env::var("CONFIG")
    .map_err(|_| CaliberError::Config(ConfigError::MissingEnvVar("CONFIG")))?;
let result = operation()?;
```

### Import Standards

```rust
// ❌ WRONG - Assumes re-exports
use axum::async_trait;

// ✅ RIGHT - Direct imports
use async_trait::async_trait;
```

### Framework Integration Standards

When integrating with frameworks:

1. **Verify version in Cargo.toml**
2. **Check current version API docs** (don't rely on AI training data)
3. **Use debug attributes** where available:
   ```rust
   #[axum::debug_handler]  // Axum will validate handler signature
   async fn my_handler(...) -> Result<...> { ... }
   ```
4. **Test with framework examples** from current version

### Security Standards

For security-sensitive code:

1. **Grep for all affected locations** before implementing fix
2. **Update ALL locations atomically** (no partial fixes)
3. **Add property tests** for security properties
4. **Document security implications** in code comments
5. **Verify with security-focused code review**

### Completeness Standards

Before marking code complete:

- [ ] Zero clippy warnings
- [ ] Zero unused imports
- [ ] Zero unused variables
- [ ] All extracted values used
- [ ] All functions wired up
- [ ] No TODO/FIXME in production code
- [ ] All tests passing
- [ ] Property tests run 100+ iterations

### Multi-Agent Deployment Standards

For complex issues, deploy specialized strike teams:

| Team | Size | Mission |
|------|------|---------|
| Alpha | 2-3 Opus | Critical blocking issues |
| Bravo | 2-3 Opus | Research + complex fixes |
| Charlie | 1-2 Sonnet | Cleanup + minor issues |
| QA | 1 Opus | Final verification |

**When to deploy:**
- Blocking build errors → Alpha
- Framework integration → Bravo
- Security fixes → Alpha + QA
- Code quality → Charlie
- Comprehensive audit → All teams
