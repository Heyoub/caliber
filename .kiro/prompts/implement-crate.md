# Implement Crate

Scaffold or implement a specific CALIBER crate.

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

```rust
//! caliber-{name}
//! 
//! {description}

use caliber_core::*;

// ... implementation
```
