# Plan Feature

Create a detailed implementation plan for a CALIBER feature.

## Instructions

When planning a feature:

1. **Identify affected crates** — which of the 8 crates need changes?
2. **Define types first** — what new structs/enums in caliber-core?
3. **Storage operations** — what direct heap operations needed?
4. **Trait definitions** — what interfaces for composability?
5. **pgrx exports** — what functions exposed to Postgres?
6. **Configuration** — what new CaliberConfig fields required?

## Output Format

```markdown
## Feature: [Name]

### Affected Crates
- caliber-core: [changes]
- caliber-storage: [changes]
- ...

### New Types (caliber-core)
```rust
// Type definitions
```

### Storage Operations (caliber-storage)
```rust
// Direct heap operations
```

### Configuration Additions
```rust
// New CaliberConfig fields
```

### Implementation Steps
1. [ ] Step one
2. [ ] Step two
...
```

## Constraints

- No hard-coded values
- No SQL in hot path
- All operations return CaliberResult<T>
- Follow ECS composition pattern
