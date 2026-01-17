# Code Review

Review CALIBER code for quality and adherence to project standards.

## Checklist

### Architecture

- [ ] Correct crate placement (types in core, storage in storage, etc.)
- [ ] ECS composition pattern followed
- [ ] No circular dependencies between crates
- [ ] **No stub files or TODO placeholders** — every file has real code

### Multi-Phase Verification (CRITICAL)

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` succeeds (ZERO warnings)
- [ ] `cargo test --workspace` succeeds (ALL tests pass)
- [ ] Property tests run 100+ iterations
- [ ] **DO NOT mark complete until all phases pass**

### Configuration

- [ ] No hard-coded values
- [ ] All new config fields added to CaliberConfig
- [ ] Missing config produces error, not silent default

### Error Handling

- [ ] All functions return CaliberResult<T>
- [ ] No unwrap() or expect() in production code
- [ ] Errors use appropriate CaliberError variant
- [ ] No panic! in production code paths

### Framework Integration

- [ ] Framework version verified in Cargo.toml
- [ ] API signatures match current framework version (not AI training data)
- [ ] Debug attributes used where available (`#[axum::debug_handler]`, etc.)
- [ ] Imports verified (no assumed re-exports)

### Security

- [ ] Security fixes are complete across ALL locations (grep verification)
- [ ] No partial implementations of security features
- [ ] Tenant isolation enforced everywhere
- [ ] No insecure defaults or fallbacks

### Storage

- [ ] No SQL in hot path
- [ ] Direct heap operations via pgrx
- [ ] Proper index usage for queries

### Types

- [ ] EntityId (UUIDv7) for all IDs
- [ ] EmbeddingVector for embeddings (dynamic dimension)
- [ ] RawContent (Vec<u8>) for flexible content
- [ ] PostgresType/PostgresEnum derives where needed

### Rust Quality

- [ ] No clippy warnings (run with `-D warnings`)
- [ ] No unused imports
- [ ] No unused variables
- [ ] All extracted values are used
- [ ] Proper documentation comments
- [ ] Tests for public functions

### AI Code Smell Detection

Watch for these patterns in AI-generated code:

- [ ] **Partial Feature Implementation** - Feature started but not completed across all locations
- [ ] **Framework Version Mismatch** - Code uses API from older framework version
- [ ] **Import Path Confusion** - Assumes re-exports that don't exist
- [ ] **Unused Variables** - Variables extracted but never used
- [ ] **Panic-Prone Error Handling** - `.expect()` or `.unwrap()` in production code

## Output

Provide specific feedback with:

- File and line references
- Severity (critical/warning/suggestion)
- Recommended fix
