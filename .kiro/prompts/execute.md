# Execute Plan

Implement a planned feature systematically.

## ⚠️ NO STUBS RULE

**NEVER create empty files or TODO placeholders.**

Before creating ANY source file:
1. Do I know EXACTLY what goes in it? → Create with full content
2. Am I just making a placeholder? → DON'T CREATE IT

Reference `docs/DEPENDENCY_GRAPH.md` for all type definitions.

## Instructions

Given a feature plan, implement it following this order:

1. **caliber-core** — Add entity types first (data only, no behavior)
2. **caliber-storage** — Add storage trait methods + pgrx implementation
3. **Component crates** — Implement business logic (context, pcp, llm, agents)
4. **caliber-dsl** — Add DSL grammar if needed
5. **caliber-pg** — Wire up pgrx exports

## Implementation Rules

### Types (caliber-core)

```rust
#[derive(Debug, Clone, PostgresType)]
pub struct NewEntity {
    pub id: EntityId,
    // ... fields
}
```

### Storage (caliber-storage)

```rust
// Direct heap operations - NO SQL
pub fn caliber_entity_insert(entity: &NewEntity) -> CaliberResult<()> {
    // Direct heap tuple insert
}
```

### Configuration

```rust
// Add to CaliberConfig - user MUST provide
pub new_field: SomeType,  // Required, no default
```

### Error Handling

```rust
// Use CaliberResult everywhere
pub fn operation() -> CaliberResult<T> {
    // Use ? operator, errors propagate to Postgres
}
```

## Validation

After implementation:

1. `cargo build --workspace` — must compile
2. `cargo clippy --workspace` — no warnings
3. `cargo test --workspace` — tests pass
