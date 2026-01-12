# Code Review

Review CALIBER code for quality and adherence to project standards.

## Checklist

### Architecture
- [ ] Correct crate placement (types in core, storage in storage, etc.)
- [ ] ECS composition pattern followed
- [ ] No circular dependencies between crates

### Configuration
- [ ] No hard-coded values
- [ ] All new config fields added to CaliberConfig
- [ ] Missing config produces error, not silent default

### Error Handling
- [ ] All functions return CaliberResult<T>
- [ ] No unwrap() or expect() in production code
- [ ] Errors use appropriate CaliberError variant

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
- [ ] No clippy warnings
- [ ] Proper documentation comments
- [ ] Tests for public functions

## Output

Provide specific feedback with:
- File and line references
- Severity (critical/warning/suggestion)
- Recommended fix
