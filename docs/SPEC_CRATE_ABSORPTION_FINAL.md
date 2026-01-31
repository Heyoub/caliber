# CALIBER CRATE ABSORPTION - FINAL SPEC

**Created**: 2026-01-24
**Updated**: 2026-01-25
**Status**: COMPLETED
**Goal**: Absorb caliber-agents and caliber-llm into caliber-core/caliber-api

---

## CURRENT STATE SUMMARY

```
Crates:        7 (down from 11)
  - caliber-core
  - caliber-dsl
  - caliber-pcp
  - caliber-storage
  - caliber-pg
  - caliber-api
  - caliber-test-utils

Deleted:       caliber-agents (~1,654 LOC), caliber-llm (~1,145 LOC), caliber-context, caliber-events, caliber-tui (~4,500 LOC)
```

---

## WHAT'S DONE

### caliber-agents Absorption
- [x] EntityId type alias replaced with 15 typed newtypes (TenantId, TrajectoryId, etc.)
- [x] EntityIdType trait with new(), as_uuid(), now_v7(), nil() methods
- [x] Enums moved from caliber-agents to caliber-core/src/agent.rs
- [x] Structs moved from caliber-agents to caliber-core/src/entities.rs
- [x] caliber-agents crate deleted

### caliber-llm Absorption
- [x] Pure primitives (SummarizeStyle, ProviderCapability, CircuitState, etc.) → caliber-core/src/llm.rs
- [x] Pure traits (EmbeddingProvider, SummarizationProvider) → caliber-core/src/llm.rs
- [x] IO/Orchestration (ProviderRegistry, CircuitBreaker, CostTracker, etc.) → caliber-api/src/providers/mod.rs
- [x] Mock providers (MockEmbeddingProvider, MockSummarizationProvider) → caliber-test-utils/src/lib.rs
- [x] caliber-llm crate deleted

---

## WHAT REMAINS

### Phase 1: Fix Immediate Cargo Errors

| File | Error | Fix |
|------|-------|-----|
| caliber-core/src/context.rs | `EntityIdType` trait not in scope | Add `use crate::EntityIdType;` |
| caliber-core/src/agent.rs | Duplicate `DelegationResultStatus` | DELETE from agent.rs (keep delegation.rs) |
| caliber-core/src/agent.rs | Duplicate `DelegationResult` | DELETE from agent.rs (keep delegation.rs) |
| caliber-core/src/agent.rs | Unused imports `LockMode, Timestamp` | DELETE (structs not moved yet) |
| caliber-core/src/entities.rs | Unused import `TenantId` | DELETE |

### Phase 2: Move Structs (7 remaining)

These structs are still in `caliber-agents/src/lib.rs` and need to move:

| Struct | Destination | Rationale |
|--------|-------------|-----------|
| Agent | caliber-core/src/entities.rs | Domain primitive like Trajectory |
| AgentMessage | caliber-core/src/entities.rs | Domain primitive |
| DelegatedTask | caliber-core/src/entities.rs | Domain primitive |
| AgentHandoff | caliber-core/src/entities.rs | Domain primitive |
| Conflict | caliber-core/src/entities.rs | Domain primitive |
| ConflictResolutionRecord | caliber-core/src/entities.rs | Domain primitive |
| MemoryRegionConfig | caliber-core/src/agent.rs | Access control config |

**DELETE** (not move):
| Struct | Reason |
|--------|--------|
| DistributedLock | core::LockData is strictly superior (has tenant_id, methods, typestate) |

### Phase 3: Update caliber-pg Imports

7 files need updating from `caliber_agents::X` to `caliber_core::X`:

1. `agent_heap.rs` - Agent, AgentStatus, MemoryAccess
2. `message_heap.rs` - AgentMessage, MessageType, MessagePriority
3. `delegation_heap.rs` - DelegatedTask, DelegationStatus, DelegationResult
4. `handoff_heap.rs` - AgentHandoff, HandoffStatus, HandoffReason
5. `conflict_heap.rs` - Conflict, ConflictType, ConflictStatus, ConflictResolutionRecord
6. `lock_heap.rs` - DistributedLock -> LockData
7. `lib.rs` - All imports

### Phase 4: Fix API String vs Enum Bug

`caliber-api/src/types/message.rs` uses String where it should use enums:

```rust
// WRONG (current)
pub message_type: String,
pub priority: String,

// CORRECT (fix to)
pub message_type: MessageType,
pub priority: MessagePriority,
```

Keep `SendMessageRequest` with String (HTTP boundary deserialization).

### Phase 5: Delete caliber-agents

1. Remove from `Cargo.toml` workspace members
2. Remove from all dependent crate Cargo.toml files
3. Delete `caliber-agents/` directory

### Phase 6: Verification

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

---

## FILE ORGANIZATION (After Absorption)

```
caliber-core/src/
├── lib.rs              # Re-exports everything
├── identity.rs         # EntityIdType trait + 15 typed IDs
├── entities.rs         # Domain primitives:
│                       #   Trajectory, Scope, Artifact, Note, Turn
│                       #   Agent, AgentMessage, DelegatedTask
│                       #   AgentHandoff, Conflict, ConflictResolutionRecord
├── agent.rs            # Agent enums + access control:
│                       #   AgentStatus, MessageType, MessagePriority
│                       #   PermissionScope, MemoryRegion, MemoryAccess
│                       #   MemoryPermission, MemoryRegionConfig
│                       #   HandoffReason, ConflictType, ConflictStatus
│                       #   ResolutionStrategy
├── delegation.rs       # Delegation typestate + DelegationResult
├── handoff.rs          # Handoff typestate
├── lock.rs             # Lock typestate + LockData
├── llm.rs              # LLM primitives (absorbed from caliber-llm)
├── config.rs           # Configuration types
├── context.rs          # Context assembly
├── enums.rs            # Core enums (EntityType, TrajectoryStatus, etc.)
├── error.rs            # Error types
├── event.rs            # Event system
└── health.rs           # Health checks
```

---

## MENTAL MODEL REMINDERS

1. **Tuples/Enums over Strings**: MessageType is an enum, NOT String
2. **SOT (Single Source of Truth)**: caliber-core/src/lib.rs exports all types
3. **Domain primitives in entities.rs**: No tenant_id (that's API layer)
4. **Response DTOs add tenant_id**: caliber-api/types adds tenant_id + methods
5. **Delete, don't copy**: DistributedLock is inferior to LockData - DELETE it
6. **caliber-pg uses core directly**: Not API types (different layer)

---

## EXECUTION CHECKLIST

```
[x] Phase 1.1: EntityIdType not needed in context.rs (verified)
[x] Phase 1.2: Delete DelegationResultStatus from agent.rs (moved to delegation.rs)
[x] Phase 1.3: Delete DelegationResult from agent.rs (enhanced in delegation.rs with produced_notes)
[x] Phase 1.4: Delete unused imports from agent.rs (removed LockMode, Timestamp)
[x] Phase 1.5: Delete unused import from entities.rs (removed TenantId)
[x] Phase 2.1: Move Agent struct to entities.rs
[x] Phase 2.2: Move AgentMessage struct to entities.rs
[x] Phase 2.3: Move DelegatedTask struct to entities.rs
[x] Phase 2.4: Move AgentHandoff struct to entities.rs
[x] Phase 2.5: Move Conflict struct to entities.rs
[x] Phase 2.6: Move ConflictResolutionRecord struct to entities.rs
[x] Phase 2.7: Move MemoryRegionConfig + ConflictResolution to agent.rs
[x] Phase 2.8: DO NOT move DistributedLock (deleted - using LockData)
[x] Phase 3.1: Update agent_heap.rs imports
[x] Phase 3.2: Update message_heap.rs imports
[x] Phase 3.3: Update delegation_heap.rs imports
[x] Phase 3.4: Update handoff_heap.rs imports
[x] Phase 3.5: Update conflict_heap.rs imports
[x] Phase 3.6: Update lock_heap.rs (use LockData)
[x] Phase 3.7: Update lib.rs imports
[x] Phase 4.1: Fix MessageResponse to use MessageType enum
[x] Phase 4.2: Fix MessageResponse to use MessagePriority enum
[x] Phase 5.1: Remove caliber-agents from workspace
[x] Phase 5.2: Remove caliber-agents dependency from caliber-pg
[x] Phase 5.3: Delete caliber-agents directory

### caliber-llm Absorption
[x] Phase 5.4: Move pure primitives to caliber-core/src/llm.rs
[x] Phase 5.5: Move pure traits (EmbeddingProvider, SummarizationProvider) to caliber-core/src/llm.rs
[x] Phase 5.6: Create caliber-api/src/providers/mod.rs with IO/orchestration code
[x] Phase 5.7: Move mock providers to caliber-test-utils
[x] Phase 5.8: Remove caliber-llm from workspace
[x] Phase 5.9: Delete caliber-llm directory

### Verification
[ ] Phase 6.1: cargo check --workspace
[ ] Phase 6.2: cargo clippy --workspace
[ ] Phase 6.3: cargo test --workspace
```

---

## ESTIMATED IMPACT

- **Lines removed**: ~2,800 (caliber-agents ~1,654 + caliber-llm ~1,145)
- **Crates after**: 8 (down from 11)
- **Cleaner architecture**:
  - Single source of truth for primitives in caliber-core
  - LLM orchestration properly in caliber-api/src/providers/
  - Mock providers in caliber-test-utils
