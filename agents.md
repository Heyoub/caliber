# CALIBER Agent Survival Guide

> **Quick Start**: Read this, then `docs/DEPENDENCY_GRAPH.md` for types.

## What Is This

Postgres-native memory framework for AI agents. Multi-agent coordination with direct heap access (no SQL in hot path).

```
caliber-core    → Pure data types (EntityId, TTL, Trajectory, Scope, Artifact, Note)
caliber-storage → Storage trait (NOT pgrx impl - that's in caliber-pg)
caliber-context → Context assembly, token budgets
caliber-pcp     → Validation, checkpoints, contradiction detection
caliber-llm     → VAL (Vector Abstraction Layer), embeddings
caliber-agents  → Locks, messages, delegation, handoffs
caliber-dsl     → Custom DSL parser → CaliberConfig
caliber-pg      → pgrx extension - wires ALL components
```

## DO NOT

| Anti-Pattern | Why | Instead |
|--------------|-----|---------|
| Use `#[allow(unused_*)]` | Hides broken code | Wire up or remove |
| Create stub files | Context loss, forgotten | Wait until full impl ready |
| Use `todo!()` | Panic in prod | Implement or don't create |
| Hard-code values | Framework, not product | User configures ALL |
| Call `CaliberConfig::default()` | Doesn't exist | Build config explicitly |
| Use `RawContent(x)` | It's `Vec<u8>` alias | Just use `x: RawContent` |
| SQL in hot path | Performance | Direct heap ops |
| Run cargo incrementally | 47 errors syndrome | Generate complete, build once |

## Type Traps

```rust
// ExtractionMethod - ONLY these variants:
Explicit | Inferred | UserProvided

// TTL - these exist:
Persistent | Session | Scope | Duration(ms) | Ephemeral | ShortTerm | MediumTerm | LongTerm | Permanent

// Checkpoint - NOT { data }, it's:
Checkpoint { context_state: RawContent, recoverable: bool }

// MemoryAccess - STRUCT not enum:
MemoryAccess { read: Vec<MemoryPermission>, write: Vec<MemoryPermission> }

// MemoryRegionConfig - use constructors:
MemoryRegionConfig::private(owner_id)
MemoryRegionConfig::public(owner_id)
MemoryRegionConfig::collaborative(owner_id)
```

## File Locations

| Need | File |
|------|------|
| All types | `docs/DEPENDENCY_GRAPH.md` |
| Core spec | `docs/CALIBER_PCP_SPEC.md` |
| DSL grammar | `docs/DSL_PARSER.md` |
| Multi-agent | `docs/MULTI_AGENT_COORDINATION.md` |
| Dev philosophy | `.kiro/steering/dev-philosophy.md` |
| Tech standards | `.kiro/steering/tech.md` |
| Custom prompts | `.kiro/prompts/*.md` |

## caliber-pg Heap Pattern

```rust
// Direct heap access - NO SQL parsing overhead
use crate::heap_ops::{open_relation, insert_tuple, ...};
use crate::tuple_extract::{extract_uuid, extract_text, ...};

// Open relation with lock
let rel = open_relation("caliber_artifact", AccessShareLock)?;

// Build datum arrays for insert
let values: [Datum; N] = [...];
let nulls: [bool; N] = [...];

// Insert via heap
let tid = insert_tuple(&rel, &values, &nulls)?;
```

## Error Pattern

```rust
// Use CaliberResult<T> everywhere
pub fn my_fn() -> CaliberResult<T> {
    // ? propagates to Postgres via ereport
    let x = fallible_op()?;
    Ok(x)
}

// Never .unwrap() in prod
```

## Build Commands

```bash
cargo build --workspace           # Full build
cargo build -p caliber-pg         # Just extension
cargo pgrx test -p caliber-pg     # pgrx tests
```

## Deep Dives

- **caliber-core**: `caliber-core/src/lib.rs` - ALL type defs
- **caliber-agents**: `caliber-agents/src/lib.rs` - Lock/Message/Delegation
- **caliber-pg heaps**: `caliber-pg/src/*_heap.rs` - Direct storage ops

---

*Don't chase the compiler. Understand the types first.*
