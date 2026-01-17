# Implement Crate

Implement a specific CALIBER crate with COMPLETE code.

## ⚠️ NO STUBS RULE

**NEVER create empty files or TODO placeholders.**

- Every lib.rs must have REAL, WORKING code
- If you don't know what goes in a file, don't create it yet
- Copy types from `docs/DEPENDENCY_GRAPH.md` — they're already defined
- One crate at a time, fully implemented before moving on

## Crate Responsibilities

### caliber-core

- Entity types only (Trajectory, Scope, Artifact, Note, Turn)
- No behavior, just data structures
- PostgresType/PostgresEnum derives

### caliber-storage

- StorageTrait definition
- pgrx implementation with direct heap access
- Index operations (btree, hnsw, etc.)

### caliber-context

- ContextWindow assembly
- Section prioritization
- Token budget management

### caliber-pcp

- Validation logic
- Checkpoint creation/recovery
- Contradiction detection

### caliber-llm

- EmbeddingProvider trait
- SummarizationProvider trait
- Provider implementations (OpenAI, Ollama, etc.)
- Caching layer

### caliber-agents

- Agent registration
- Distributed locks (advisory locks)
- Message passing (NOTIFY)
- Task delegation
- Handoff protocol

### caliber-dsl

- Lexer (tokenization)
- Parser (AST building)
- Validator (semantic checks)
- Code generator (CaliberConfig output)

### caliber-pg

- pgrx extension entry point
- Wire all components together
- Export pg_extern functions

## Template

When implementing a crate:

1. **Read** `docs/DEPENDENCY_GRAPH.md` for the exact types
2. **Create** lib.rs with FULL implementation (not stubs)
3. **Include** all types, traits, and impls for that crate
4. **Test** only after all code is written

```rust
//! caliber-{name}
//! 
//! {description}

use caliber_core::*;

// FULL implementation here — no TODOs, no placeholders
// Copy types from docs/DEPENDENCY_GRAPH.md
```

## Anti-Patterns to Avoid

```rust
// ❌ WRONG - Empty stub
pub struct SomeType;  // TODO: add fields

// ❌ WRONG - Placeholder
pub fn some_function() {
    todo!()
}

// ✅ RIGHT - Complete implementation
pub struct SomeType {
    pub id: EntityId,
    pub name: String,
    pub created_at: Timestamp,
}

pub fn some_function(input: &str) -> CaliberResult<SomeType> {
    // Actual implementation
}
```

## Verification Workflow

**CRITICAL:** Follow this workflow for EVERY crate implementation:

### Phase 1: Generate Complete Code

1. Read `docs/DEPENDENCY_GRAPH.md` for exact types
2. Create lib.rs with FULL implementation
3. Include all types, traits, and impls
4. NO stubs, NO TODOs, NO placeholders

### Phase 2: Build Verification

```bash
cargo build -p caliber-{name}
```

- Fix compilation errors
- Verify all dependencies resolve
- Ensure types match docs

### Phase 3: Clippy Verification (REQUIRED)

```bash
cargo clippy -p caliber-{name} -- -D warnings
```

- **ZERO warnings allowed**
- Fix unused imports
- Fix unused variables
- Verify all code paths reachable

**DO NOT mark complete until clippy is clean.**

### Phase 4: Test Verification

```bash
cargo test -p caliber-{name}
```

- All unit tests pass
- All property tests pass (100+ iterations)
- All integration tests pass

### Phase 5: Mark Complete

Only after ALL phases pass:

- [ ] Build succeeds
- [ ] Clippy clean (zero warnings)
- [ ] All tests pass
- [ ] No stubs or TODOs
- [ ] All types match docs

## Framework Integration Standards

When integrating with frameworks (Axum, Tokio, pgrx):

1. **Verify version in Cargo.toml**
   ```toml
   axum = "0.8.0"  # Check current version
   ```

2. **Check current version API docs** (don't rely on AI training data)

3. **Use debug attributes** where available:
   ```rust
   #[axum::debug_handler]  // Framework validates signature
   async fn my_handler(...) -> Result<...> { ... }
   ```

4. **Verify imports compile**:
   ```rust
   // ❌ WRONG - Assumes re-export
   use axum::async_trait;
   
   // ✅ RIGHT - Direct import
   use async_trait::async_trait;
   ```

## Security Implementation Standards

For security-sensitive code:

1. **Grep for all affected locations** before implementing
2. **Update ALL locations atomically** (no partial fixes)
3. **Add property tests** for security properties
4. **Document security implications** in code comments

Example:
```bash
# Before implementing tenant isolation fix
rg "WsEvent::" --type rust  # Find all usage locations
# Update ALL locations, not just some
```

## Error Handling Standards

```rust
// ❌ WRONG - Panics in production
let value = env::var("CONFIG").expect("CONFIG must be set");
let result = operation().unwrap();

// ✅ RIGHT - Returns CaliberResult
let value = env::var("CONFIG")
    .map_err(|_| CaliberError::Config(ConfigError::MissingEnvVar("CONFIG")))?;
let result = operation()?;
```

## Completeness Checklist

Before marking crate complete:

- [ ] All types from docs/DEPENDENCY_GRAPH.md implemented
- [ ] Zero clippy warnings
- [ ] Zero unused imports/variables
- [ ] All extracted values used
- [ ] All functions wired up
- [ ] No TODO/FIXME in production code
- [ ] All tests passing
- [ ] Property tests run 100+ iterations
- [ ] Documentation comments on public items
- [ ] Error handling uses CaliberResult<T>
- [ ] No unwrap()/expect() in production code
