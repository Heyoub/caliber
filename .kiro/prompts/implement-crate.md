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
