# CALIBER Dependency Graph & Type System

Updated for v0.4.7 - typed ID refactor and current workspace layout.

> **Note (v0.4.6):** The following crates were deleted and absorbed:
> - `caliber-tui` (~4,500 LOC) → TypeScript SDK replaces
> - `caliber-agents` (~1,654 LOC) → `caliber-core/src/agent.rs`, `entities.rs`
> - `caliber-llm` (~1,145 LOC) → `caliber-core/src/llm.rs`, `caliber-api/src/providers/`
> - `caliber-context`, `caliber-events` → absorbed into core/storage
>
> See `docs/SPEC_CRATE_ABSORPTION_FINAL.md` for details.

---

## 1. Crate Dependency Graph (DAG) - 7 Crates

```
                    ┌─────────────────┐
                    │  caliber-core   │  (foundation)
                    └────────┬────────┘
                             │
         ┌───────────────────┼─────────────────────┬─────────────────────┐
         │                   │                     │                     │
         ▼                   ▼                     ▼                     ▼
┌─────────────────┐ ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│ caliber-storage │ │   caliber-pcp   │   │   caliber-dsl   │   │ caliber-test-   │
│  storage traits │ │  validation     │   │  DSL compiler   │   │ utils (fixtures)│
└────────┬────────┘ └────────┬────────┘   └────────┬────────┘   └─────────────────┘
         │                   │                     │
         ├──────────────┬────┴──────────────┬──────┘
         ▼              ▼                   ▼
┌─────────────────┐ ┌─────────────────┐
│   caliber-api   │ │   caliber-pg    │
│ REST/WS/gRPC    │ │ pgrx extension  │
└─────────────────┘ └─────────────────┘
```

Notes:
- `caliber-sdk/` and `landing/` are repo apps (not Rust workspace members).
- `caliber-pg` uses pgrx and must be tested via `cargo pgrx test`.

---

## 2. Core Type System (caliber-core)

### Typed IDs
All entity IDs are distinct newtypes (no aliasing):

- `TenantId`, `TrajectoryId`, `ScopeId`, `ArtifactId`, `NoteId`, `TurnId`
- `AgentId`, `EdgeId`, `LockId`, `MessageId`, `DelegationId`, `HandoffId`
- `ApiKeyId`, `WebhookId`, `SummarizationPolicyId`

The `EntityIdType` trait provides:
- `new(uuid)`
- `as_uuid()`
- `now_v7()`
- `nil()`

### Shared Entities
Core entities include `Trajectory`, `Scope`, `Artifact`, `Note`, `Turn`, and related enums
(`TTL`, `TrajectoryStatus`, `ArtifactType`, `NoteType`, `AgentStatus`, etc.).

### Embeddings
`EmbeddingVector` lives in `caliber-core` and is used by storage, API, and pgrx.

---

## 3. Storage Trait (caliber-storage)

`caliber-storage` defines typed-ID CRUD contracts and cache interfaces.
Implementations include the in-memory DAG and the pgrx-backed `caliber-pg`.

---

## 4. Test & Build Matrix

- Non‑pgrx: `TMPDIR=$PWD/target/tmp cargo test --workspace --exclude caliber-pg`
- pgrx: `cargo pgrx test pg18 --package caliber-pg`
- One‑shot: `./scripts/test.sh`

---

## 5. pgrx Notes

- `caliber-pg` depends on pgrx 0.16 (workspace patch on `develop`).
- Vector search relies on pgvector and is exercised in pg_test lanes.
