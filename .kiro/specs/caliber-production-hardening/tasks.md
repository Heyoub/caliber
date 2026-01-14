# Implementation Tasks

## Overview

This task list bridges the gap between the current in-memory implementation and the production-ready SQL-based persistent storage design. Many foundational items have been completed; the remaining work focuses on replacing in-memory storage with SPI-based SQL operations and adding strict validation.

## Completed Tasks Summary

The following items have been verified as complete in the current codebase:

- ✅ EntityType enum includes Turn, Lock, Message variants (caliber-core)
- ✅ FNV-1a hash implemented for lock keys (caliber-agents)
- ✅ Unicode-safe `extract_first_sentence` using `char_indices()` (caliber-pcp)
- ✅ StorageError::LockPoisoned and StorageError::SpiError variants added (caliber-core)
- ✅ Debug feature flag added to Cargo.toml (caliber-pg)
- ✅ Debug functions gated with `#[cfg(any(feature = "debug", feature = "pg_test"))]` (caliber-pg)
- ✅ Debug functions log warnings when called (caliber-pg)
- ✅ SectionPriorities includes `persona: i32` field (caliber-core)
- ✅ SQL schema exists with all tables, indexes, and constraints (caliber_init.sql)
- ✅ UNIQUE constraint on (scope_id, sequence) in caliber_turn table
- ✅ `caliber_init()` reads and executes `caliber_init.sql` via SPI
- ✅ Schema creation is idempotent (IF NOT EXISTS)
- ✅ RwLock poisoning handled gracefully via `storage_read()` and `storage_write()` helpers
- ✅ pg_notify errors logged with warning (partial fix - still returns message_id)
- ✅ Trajectory storage replaced with SPI (Task 1)
- ✅ Scope storage replaced with SPI (Task 2)
- ✅ Artifact storage replaced with SPI (Task 3)
- ✅ Note storage replaced with SPI (Task 4)
- ✅ Turn storage replaced with SPI (Task 5)
- ✅ Note access tracking implemented (caliber_note_get and caliber_note_query_by_trajectory update access_count and accessed_at)
- ✅ Turn uniqueness enforcement (caliber_turn_create verifies scope_id exists and UNIQUE constraint catches duplicates)

---

## Task 1: Replace In-Memory Trajectory Storage with SPI ✅

**Requirements:** REQ-1 (SQL-Based Persistent Storage)

**Design Reference:** AD-1

**Description:** Replace the `TRAJECTORIES` static HashMap with SPI-based SQL operations for trajectory CRUD.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Remove trajectory storage from `static STORAGE: InMemoryStorage`
- [x] Implement `trajectory_insert()` via SPI INSERT to caliber_trajectory table
- [x] Implement `trajectory_get()` via SPI SELECT from caliber_trajectory table
- [x] Implement `trajectory_update()` via SPI UPDATE on caliber_trajectory table
- [x] Update all `caliber_trajectory_*` pg_extern functions to use SPI

**Files Modified:**

- `caliber-pg/src/lib.rs` - Trajectory functions (caliber_trajectory_create, caliber_trajectory_get, caliber_trajectory_set_status, caliber_trajectory_update, caliber_trajectory_list_by_status)

---

## Task 2: Replace In-Memory Scope Storage with SPI ✅

**Requirements:** REQ-1 (SQL-Based Persistent Storage)

**Design Reference:** AD-1

**Description:** Replace scope storage in `InMemoryStorage` with SPI-based SQL operations.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Remove scope storage from `static STORAGE: InMemoryStorage`
- [x] Implement scope CRUD via SPI to caliber_scope table
- [x] Update all `caliber_scope_*` pg_extern functions to use SPI

**Files Modified:**

- `caliber-pg/src/lib.rs` - Scope functions (caliber_scope_create, caliber_scope_get, caliber_scope_get_current, caliber_scope_close, caliber_scope_update_tokens)

---

## Task 3: Replace In-Memory Artifact Storage with SPI ✅

**Requirements:** REQ-1 (SQL-Based Persistent Storage)

**Design Reference:** AD-1

**Description:** Replace artifact storage in `InMemoryStorage` with SPI-based SQL operations.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Remove artifact storage from `static STORAGE: InMemoryStorage`
- [x] Implement artifact CRUD via SPI to caliber_artifact table
- [x] Update all `caliber_artifact_*` pg_extern functions to use SPI

**Files Modified:**

- `caliber-pg/src/lib.rs` - Artifact functions (caliber_artifact_create, caliber_artifact_get, caliber_artifact_query_by_type, caliber_artifact_query_by_scope)

---

## Task 4: Replace In-Memory Note Storage with SPI ✅

**Requirements:** REQ-1, REQ-8 (SQL-Based Storage, Note Access Tracking)

**Design Reference:** AD-1

**Description:** Replace note storage in `InMemoryStorage` with SPI-based SQL operations, including access tracking updates.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Remove note storage from `static STORAGE: InMemoryStorage`
- [x] Implement note CRUD via SPI to caliber_note table
- [x] `caliber_note_get` increments `access_count` and updates `accessed_at` via SPI UPDATE
- [x] `caliber_note_query_by_trajectory` updates access metrics for returned notes
- [x] Update all `caliber_note_*` pg_extern functions to use SPI

**Files Modified:**

- `caliber-pg/src/lib.rs` - Note functions (caliber_note_create, caliber_note_get, caliber_note_query_by_trajectory)

---

## Task 5: Replace In-Memory Turn Storage with SPI ✅

**Requirements:** REQ-1, REQ-9 (SQL-Based Storage, Turn Uniqueness)

**Design Reference:** AD-1

**Description:** Replace turn storage in `InMemoryStorage` with SPI-based SQL operations, enforcing uniqueness.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Remove turn storage from `static STORAGE: InMemoryStorage`
- [x] Implement turn CRUD via SPI to caliber_turn table
- [x] `caliber_turn_create` verifies scope_id exists via SPI SELECT before insert
- [x] `caliber_turn_create` returns error on duplicate (scope_id, sequence) - enforced by UNIQUE constraint
- [x] Update all `caliber_turn_*` pg_extern functions to use SPI

**Files Modified:**

- `caliber-pg/src/lib.rs` - Turn functions (caliber_turn_create, caliber_turn_get_by_scope)

---

## Task 6: Implement Correct Advisory Lock Semantics

**Requirements:** REQ-2 (Correct Advisory Lock Semantics)

**Design Reference:** AD-2, AD-3

**Description:** Fix advisory lock acquire/release to use matching session or transaction level, with deterministic key hashing. Currently uses transaction-level locks (`pg_try_advisory_xact_lock`) but needs session-level support and proper lock persistence in SQL.

**Acceptance Criteria:**

- [ ] Add `LockMode` enum with Session and Transaction variants to caliber-pg (separate from caliber-agents LockMode which is Exclusive/Shared)
- [x] FNV-1a hash for lock keys implemented (caliber-agents)
- [ ] `caliber_lock_acquire` with Session mode uses `pg_try_advisory_lock`
- [ ] `caliber_lock_release` for Session uses `pg_advisory_unlock`
- [ ] `caliber_lock_acquire` with Transaction mode uses `pg_try_advisory_xact_lock`
- [ ] Transaction locks have no explicit release (auto-release at commit)
- [ ] Lock records stored in SQL caliber_lock table via SPI (currently only in-memory)
- [ ] timeout_ms implemented via statement_timeout for blocking acquire

**Files to Modify:**

- `caliber-pg/src/lib.rs` - Lock functions (caliber_lock_acquire, caliber_lock_release)

**Notes:**
- Current implementation uses transaction-level locks but stores lock records in in-memory HashMap
- Need to persist lock records to SQL table for cross-session visibility
- Need to add session-level lock support as an alternative to transaction-level

---

## Task 7: Implement Message Persistence

**Requirements:** REQ-3 (Message Persistence and Cross-Session Delivery)

**Design Reference:** AD-4

**Description:** Replace in-memory message storage with SQL persistence, using pg_notify for wake-up only. Currently messages are stored in-memory HashMap and pg_notify failures are logged but don't return errors.

**Acceptance Criteria:**

- [ ] Remove message storage from `static STORAGE: InMemoryStorage`
- [ ] `caliber_message_send` inserts into SQL caliber_message table via SPI
- [ ] `caliber_message_send` calls pg_notify with message_id only (currently done)
- [ ] `caliber_message_get` retrieves from SQL table via SPI
- [ ] `caliber_message_get_pending` queries SQL for undelivered messages
- [ ] pg_notify failures return error (currently logs warning but continues)
- [ ] Expired messages marked as expired (not deleted)

**Files to Modify:**

- `caliber-pg/src/lib.rs` - Message functions (caliber_message_send, caliber_message_get, caliber_message_get_pending, caliber_message_mark_delivered, caliber_message_mark_acknowledged)

**Notes:**
- Current implementation stores messages in in-memory HashMap
- pg_notify failures are logged with warning but message_id is still returned
- Need to persist messages to SQL table for cross-session delivery

---

## Task 8: Implement Access Control Enforcement

**Requirements:** REQ-4 (Access Control Enforcement)

**Design Reference:** AD-11

**Description:** Enforce memory region permissions on all read/write operations. Currently `caliber_check_access` always returns true for registered agents. Need to implement actual permission checking.

**Acceptance Criteria:**

- [ ] Implement `enforce_access()` helper function
- [ ] Check read permission before all read operations using `MemoryRegionConfig::can_read`
- [ ] Check write permission before all write operations using `MemoryRegionConfig::can_write`
- [ ] Return `AgentError::PermissionDenied` on access violation
- [ ] Verify lock held for collaborative region writes
- [ ] Store memory regions in SQL regions table via SPI (need to add table to schema)

**Files to Modify:**

- `caliber-pg/src/lib.rs` - Access control functions (add enforce_access helper, update all read/write operations)
- `caliber-pg/sql/caliber_init.sql` - Add regions table if missing

**Notes:**
- Current implementation has placeholder access control that always returns true
- Need to add caliber_region table to SQL schema
- Need to integrate permission checks into all entity read/write operations

---

## Task 9: Implement Strict DSL Parsing

**Requirements:** REQ-5 (Strict DSL Parsing)

**Design Reference:** AD-5

**Description:** Remove all silent defaults from DSL parser, require explicit values for all fields. Currently the parser supplies defaults for adapter type, memory type, retention, priority, etc.

**Acceptance Criteria:**

- [ ] Missing adapter 'type' returns ParseError (currently defaults to Postgres)
- [ ] Empty connection string returns ParseError
- [ ] Missing memory 'type' returns ParseError (currently defaults to Working)
- [ ] Missing memory 'retention' returns ParseError (currently defaults to Persistent)
- [ ] Missing injection 'priority' returns ParseError (currently defaults to 50)
- [ ] Unknown fields return ParseError (currently implemented for some fields)
- [ ] No default values supplied anywhere

**Files to Modify:**

- `caliber-dsl/src/lib.rs` - Parser functions (parse_adapter, parse_memory, parse_injection)

**Notes:**
- Need to audit all parser functions for `.unwrap_or()` and `.unwrap_or_default()` calls
- Replace silent defaults with explicit ParseError returns
- Ensure all required fields are validated

---

## Task 10: Remove PCPConfig Default Implementation

**Requirements:** REQ-6 (PCP Configuration Without Defaults)

**Design Reference:** AD-6

**Description:** Remove `Default` impl from `PCPConfig`, require explicit configuration. Currently `PCPConfig` implements `Default` with hard-coded values at line 701 in caliber-pcp/src/lib.rs.

**Acceptance Criteria:**

- [ ] Remove `impl Default for PCPConfig` (line 701 in caliber-pcp/src/lib.rs)
- [ ] `PCPRuntime::new()` requires explicit config
- [ ] Missing `max_tokens_per_scope` returns ConfigError
- [ ] Missing `contradiction_threshold` returns ConfigError
- [ ] Missing `max_checkpoints` returns ConfigError
- [ ] Checkpoints persisted to SQL checkpoints table (need to add table)
- [ ] Recovery loads checkpoints from SQL table

**Files to Modify:**

- `caliber-pcp/src/lib.rs` - PCPConfig and PCPRuntime (remove Default impl at line 701)
- `caliber-pg/sql/caliber_init.sql` - Add checkpoints table if missing

**Notes:**
- Default impl exists at line 701 with hard-coded values for all config fields
- This violates the "no hard-coded values" philosophy
- Need to add caliber_checkpoint table to SQL schema for persistence

---

## Task 11: Complete EntityType Error Handling

**Requirements:** REQ-7 (EntityType Completeness)

**Design Reference:** AD-7

**Description:** Update error handling to use the new EntityType variants (Turn, Lock, Message are already added to the enum). Need to update actual error handling code to use these variants.

**Acceptance Criteria:**

- [x] `EntityType::Turn` variant exists
- [x] `EntityType::Lock` variant exists
- [x] `EntityType::Message` variant exists
- [x] Update lock error handling to use `EntityType::Lock` in StorageError
- [x] Update message error handling to use `EntityType::Message` in StorageError

**Files to Modify:**

- `caliber-pg/src/lib.rs` - Error handling for locks and messages (caliber_lock_acquire, caliber_lock_release, caliber_message_send, caliber_message_get)

**Notes:**
- EntityType enum already has Turn, Lock, and Message variants
- Need to update error construction in lock and message functions to use correct EntityType

---

## Task 12: Implement Explicit Error Handling

**Requirements:** REQ-11 (Explicit Error Handling)

**Design Reference:** AD-10

**Description:** Replace all silent failures with explicit error returns. Currently uses `.ok()` on some operations and has various `.unwrap()` and `.unwrap_or()` calls that hide errors.

**Acceptance Criteria:**

- [ ] pg_notify failures return CaliberError::Agent (currently logs warning but returns message_id)
- [ ] SPI failures return CaliberError::Storage
- [x] StorageError::LockPoisoned variant exists
- [x] StorageError::SpiError variant exists
- [x] RwLock poisoning handled gracefully in storage_read/storage_write helpers
- [ ] Replace remaining `.unwrap()` calls in handoff functions (lines 1161, 1171, 1184, 1198)
- [ ] Replace remaining `.unwrap()` calls in conflict functions (lines 1236, 1246, 1262, 1287)
- [ ] Replace remaining `.unwrap()` calls in vector_search (line 1316)
- [ ] JSON serialization failures return CaliberError::Validation (currently uses safe_to_json helper)

**Files to Modify:**

- `caliber-pg/src/lib.rs` - Error handling throughout (handoff, conflict, vector_search functions, caliber_message_send)

**Notes:**
- RwLock poisoning already handled via storage_read/storage_write helpers
- Need to audit for remaining .unwrap() calls in delegation/handoff/conflict functions
- pg_notify failures currently logged but don't prevent function from returning success

---

## Task 13: Implement Unknown Input Rejection

**Requirements:** REQ-12 (Unknown Input Rejection)

**Design Reference:** AD-10

**Description:** Replace silent defaults for unknown enum strings with validation errors. Currently unknown values default to a fallback (e.g., `ArtifactType::Custom`, `NoteType::Meta`, `TurnRole::User`).

**Acceptance Criteria:**

- [x] Unknown artifact_type returns ValidationError::InvalidValue (currently defaults to `ArtifactType::Custom`)
- [x] Unknown note_type returns ValidationError::InvalidValue (currently defaults to `NoteType::Meta` at line 1227)
- [x] Unknown role returns ValidationError::InvalidValue (currently defaults to `TurnRole::User` at line 1502)
- [x] Unknown status returns ValidationError::InvalidValue (currently returns false at line 456)

**Files to Modify:**

- `caliber-pg/src/lib.rs` - String-to-enum conversions in caliber_artifact_create, caliber_note_create (line 1227), caliber_turn_create (line 1502), caliber_trajectory_set_status (line 456)

**Notes:**
- caliber_note_create defaults unknown note_type to "meta" at line 1227
- caliber_turn_create defaults unknown role to "user" at line 1502
- caliber_trajectory_set_status returns false for unknown status at line 456
- Need to return proper ValidationError instead of silent defaults

---

## Task 14: Update Context Assembly for Persona Priority

**Requirements:** REQ-14 (Context Section Priority Configuration)

**Design Reference:** AD-12

**Description:** Update context assembly to use the separate persona priority field (field already added to SectionPriorities). Currently uses `section_priorities.system` for persona sections at line 546.

**Acceptance Criteria:**

- [x] `persona: i32` field exists in `SectionPriorities`
- [x] Update context assembly to use `section_priorities.persona` for persona sections (currently uses `section_priorities.system` at line 546)
- [x] Update CaliberConfig to require explicit persona priority
- [ ] Update DSL parser to parse persona priority (NOTE: DSL doesn't currently support section priorities syntax - this is a larger feature)

**Files to Modify:**

- `caliber-context/src/lib.rs` - Context assembly (line 546, change `.system` to `.persona`)
- `caliber-dsl/src/lib.rs` - Parser (add persona priority parsing)

**Notes:**
- SectionPriorities already has persona field in caliber-core
- Context assembly at line 546 uses `.system` instead of `.persona`
- Need to update DSL parser to parse persona priority from config

---

## Task Dependency Graph

```text
Tasks 1-5 (Entity Storage) ← can be parallelized
    ↓
Task 6 (Advisory Locks)
    ↓
Task 7 (Messages)
    ↓
Task 8 (Access Control)

Task 9 (Strict DSL) ← independent
Task 10 (PCPConfig) ← independent
Task 11 (EntityType errors) ← independent
Task 12 (Error Handling) ← independent
Task 13 (Input Rejection) ← independent
Task 14 (Persona Priority) ← independent
```

## Execution Order

**Phase 1 - Quick Wins (Independent, Low Risk):**

1. Task 11: EntityType error handling (partial - complete remaining items)
2. Task 13: Unknown input rejection
3. Task 14: Persona priority in context assembly

**Phase 2 - Configuration Hardening:**

4. Task 9: Strict DSL parsing
5. Task 10: PCPConfig no defaults
6. Task 12: Explicit error handling patterns

**Phase 3 - Storage Migration (Remaining Items):**

7. Task 6: Advisory locks (add session-level support and SQL persistence)
8. Task 7: Message persistence (move from in-memory to SQL)
9. Task 8: Access control (implement actual permission checking)

**Notes:**
- Tasks 1-5 are complete (entity storage migration to SQL)
- Focus on configuration hardening and remaining storage items
- Access control is the most complex remaining task
