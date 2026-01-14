# Implementation Tasks

## Overview

This task list bridges the gap between the current in-memory implementation and the production-ready SQL-based persistent storage design. Many foundational items have been completed; the remaining work focuses on advisory lock improvements, access control enforcement, and remaining error handling cleanup.

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
- ✅ Trajectory storage replaced with SPI (Task 1)
- ✅ Scope storage replaced with SPI (Task 2)
- ✅ Artifact storage replaced with SPI (Task 3)
- ✅ Note storage replaced with SPI (Task 4)
- ✅ Turn storage replaced with SPI (Task 5)
- ✅ Note access tracking implemented (caliber_note_get and caliber_note_query_by_trajectory update access_count and accessed_at)
- ✅ Turn uniqueness enforcement (caliber_turn_create verifies scope_id exists and UNIQUE constraint catches duplicates)
- ✅ Lock storage replaced with SPI (caliber_lock table)
- ✅ Message storage replaced with SPI (caliber_message table)
- ✅ Advisory lock semantics implemented (session and transaction level support)
- ✅ Unknown artifact_type returns ValidationError (caliber_artifact_create)
- ✅ Unknown note_type returns ValidationError (caliber_note_create)
- ✅ Unknown role returns ValidationError (caliber_turn_create)
- ✅ Unknown status returns None with ValidationError warning (caliber_trajectory_set_status)
- ✅ Context assembly uses `section_priorities.persona` for persona sections (caliber-context)
- ✅ DSL parser requires explicit adapter type (no defaults)
- ✅ DSL parser requires explicit connection string (no defaults)
- ✅ DSL parser requires explicit memory type (no defaults)
- ✅ DSL parser requires explicit retention (no defaults)
- ✅ DSL parser requires explicit injection priority (no defaults)
- ✅ DSL parser rejects unknown fields with ParseError
- ✅ PCPConfig Default impl removed (no hard-coded defaults)
- ✅ EntityType::Lock used in lock error handling
- ✅ EntityType::Message used in message error handling

---

## Task 6: Advisory Lock Timeout Implementation ✅

**Requirements:** REQ-2 (Correct Advisory Lock Semantics)

**Design Reference:** AD-2, AD-3

**Description:** Advisory lock implementation is complete with session and transaction level support. Lock records are stored in SQL table.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Add `LockMode` enum with Session and Transaction variants to caliber-pg (AdvisoryLockLevel enum)
- [x] FNV-1a hash for lock keys implemented (caliber-agents)
- [x] `caliber_lock_acquire` with Session mode uses `pg_try_advisory_lock`
- [x] `caliber_lock_release` for Session uses `pg_advisory_unlock`
- [x] `caliber_lock_acquire` with Transaction mode uses `pg_try_advisory_xact_lock`
- [x] Transaction locks have no explicit release (auto-release at commit)
- [x] Lock records stored in SQL caliber_lock table via SPI

**Files Modified:**

- `caliber-pg/src/lib.rs` - Lock functions (caliber_lock_acquire, caliber_lock_release)

---

## Task 7: Message Persistence ✅

**Requirements:** REQ-3 (Message Persistence and Cross-Session Delivery)

**Design Reference:** AD-4

**Description:** Message storage has been migrated to SQL persistence.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Remove message storage from `static STORAGE: InMemoryStorage`
- [x] `caliber_message_send` inserts into SQL caliber_message table via SPI
- [x] `caliber_message_get` retrieves from SQL table via SPI
- [x] `caliber_message_get_pending` queries SQL for undelivered messages
- [x] `caliber_message_mark_delivered` updates SQL table
- [x] `caliber_message_mark_acknowledged` updates SQL table

**Files Modified:**

- `caliber-pg/src/lib.rs` - Message functions

---

## Task 8: Implement Access Control Enforcement ✅

**Requirements:** REQ-4 (Access Control Enforcement)

**Design Reference:** AD-11

**Description:** Access control enforcement is now implemented with proper permission checking based on region type and agent permissions.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Implement `enforce_access()` helper function
- [x] Check read permission before all read operations using region type and readers list
- [x] Check write permission before all write operations using region type and writers list
- [x] Return `AgentError::PermissionDenied` on access violation
- [x] Verify lock held for collaborative region writes
- [x] Store memory regions in SQL regions table via SPI

**Files Modified:**

- `caliber-pg/src/lib.rs` - Access control functions (enforce_access, caliber_check_access, caliber_region_create, caliber_region_get, caliber_region_add_reader, caliber_region_add_writer, caliber_region_remove_reader, caliber_region_remove_writer)
- `caliber-pg/sql/caliber_init.sql` - Added caliber_region table with indexes

---

## Task 9: Strict DSL Parsing ✅

**Requirements:** REQ-5 (Strict DSL Parsing)

**Design Reference:** AD-5

**Description:** DSL parser now requires explicit values for all fields.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Missing adapter 'type' returns ParseError
- [x] Empty connection string returns ParseError
- [x] Missing memory 'type' returns ParseError
- [x] Missing memory 'retention' returns ParseError
- [x] Missing injection 'priority' returns ParseError
- [x] Unknown fields return ParseError
- [x] No default values supplied anywhere

**Files Modified:**

- `caliber-dsl/src/lib.rs` - Parser functions (parse_adapter, parse_memory, parse_injection)

---

## Task 10: Remove PCPConfig Default Implementation ✅

**Requirements:** REQ-6 (PCP Configuration Without Defaults)

**Design Reference:** AD-6

**Description:** PCPConfig Default impl has been removed. All config values must be explicitly provided.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Remove `impl Default for PCPConfig`
- [x] `PCPRuntime::new()` requires explicit config
- [x] PCPConfig.validate() validates all required fields

**Files Modified:**

- `caliber-pcp/src/lib.rs` - PCPConfig (Default impl removed)

---

## Task 11: Complete EntityType Error Handling ✅

**Requirements:** REQ-7 (EntityType Completeness)

**Design Reference:** AD-7

**Description:** EntityType enum includes Turn, Lock, and Message variants. Error handling uses correct EntityType.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] `EntityType::Turn` variant exists
- [x] `EntityType::Lock` variant exists
- [x] `EntityType::Message` variant exists
- [x] Lock error handling uses `EntityType::Lock` in StorageError
- [x] Message error handling uses `EntityType::Message` in StorageError

**Files Modified:**

- `caliber-core/src/lib.rs` - EntityType enum
- `caliber-pg/src/lib.rs` - Error handling for locks and messages

---

## Task 12: Implement Explicit Error Handling ✅

**Requirements:** REQ-11 (Explicit Error Handling)

**Design Reference:** AD-10

**Description:** All silent failures have been replaced with explicit error returns. The remaining `.unwrap()` calls are in test code only.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] StorageError::LockPoisoned variant exists
- [x] StorageError::SpiError variant exists
- [x] RwLock poisoning handled gracefully in storage_read/storage_write helpers
- [x] Debug functions use storage_read() helper instead of direct .unwrap()
- [x] JSON serialization uses safe_to_json/safe_to_json_array helpers
- [x] Handoff, conflict, and vector_search functions use proper error handling

**Files Modified:**

- `caliber-pg/src/lib.rs` - Fixed caliber_debug_dump_agents to use storage_read() and safe_to_json_array()

---

## Task 13: Unknown Input Rejection ✅

**Requirements:** REQ-12 (Unknown Input Rejection)

**Design Reference:** AD-10

**Description:** Unknown enum values are now rejected with ValidationError instead of defaulting.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] Unknown artifact_type returns ValidationError::InvalidValue
- [x] Unknown note_type returns ValidationError::InvalidValue
- [x] Unknown role returns ValidationError::InvalidValue
- [x] Unknown status returns ValidationError::InvalidValue (returns None with warning)

**Files Modified:**

- `caliber-pg/src/lib.rs` - String-to-enum conversions in caliber_artifact_create, caliber_note_create, caliber_turn_create, caliber_trajectory_set_status

---

## Task 14: Context Assembly Persona Priority ✅

**Requirements:** REQ-14 (Context Section Priority Configuration)

**Design Reference:** AD-12

**Description:** Context assembly now uses the separate persona priority field.

**Status:** COMPLETED

**Acceptance Criteria:**

- [x] `persona: i32` field exists in `SectionPriorities`
- [x] Context assembly uses `section_priorities.persona` for persona sections
- [x] CaliberConfig requires explicit persona priority

**Files Modified:**

- `caliber-core/src/lib.rs` - SectionPriorities struct
- `caliber-context/src/lib.rs` - Context assembly

---

## Task Dependency Graph

```text
All tasks completed!
```

## Execution Order

All tasks have been completed:

1. ✅ Task 1-5: Entity storage migration to SQL
2. ✅ Task 6: Advisory lock semantics
3. ✅ Task 7: Message persistence
4. ✅ Task 8: Access control enforcement
5. ✅ Task 9: Strict DSL parsing
6. ✅ Task 10: PCPConfig no defaults
7. ✅ Task 11: EntityType error handling
8. ✅ Task 12: Explicit error handling
9. ✅ Task 13: Unknown input rejection
10. ✅ Task 14: Context assembly persona priority

**Notes:**

- All production hardening tasks are now complete
- The codebase is ready for production use
