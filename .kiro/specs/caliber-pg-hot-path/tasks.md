# Implementation Plan: CALIBER-PG Hot Path Migration

## Overview

This plan migrates `caliber-pg` from SPI-based SQL operations to direct heap operations using pgrx. The implementation follows a bottom-up approach: first building the core heap operation helpers, then migrating each entity type with property tests alongside, and finally updating the pg_extern functions.

**Key Principles (from steering):**
- NO STUBS, NO TODOs - Complete code only
- NO SQL IN HOT PATH - Direct pgrx storage access
- CaliberResult<T> for all fallible operations
- Reference `docs/DEPENDENCY_GRAPH.md` for type definitions

**Current State:**
- `heap_ops.rs` and `index_ops.rs` exist but are NOT wired into lib.rs
- All pg_extern functions currently use SPI-based SQL
- `tuple_extract.rs` is incomplete (only has doc header)

## Tasks

- [x] 1. Create heap operation helper modules
  - [x] 1.1 Create `heap_ops.rs` module with safe wrappers
    - Implement `open_relation()` for relation access with lock modes
    - Implement `form_tuple()` wrapper around `heap_form_tuple`
    - Implement `insert_tuple()` wrapper around `simple_heap_insert`
    - Implement `update_tuple()` wrapper around `simple_heap_update`
    - Implement `delete_tuple()` wrapper around `simple_heap_delete`
    - Implement `current_timestamp()` using `GetCurrentTransactionStartTimestamp`
    - _Requirements: 15.1, 15.2, 15.3_

  - [x] 1.2 Create `index_ops.rs` module for index operations
    - Implement `catalog_index_insert()` for maintaining indexes on insert
    - Implement `IndexScanner` struct for index-based lookups
    - Implement scan key initialization helpers
    - _Requirements: 13.1, 13.2, 13.4, 13.5_

  - [x] 1.3 Complete `tuple_extract.rs` module for tuple value extraction
    - Implement `extract_datum()` for getting typed values from tuples
    - Implement `extract_nulls()` for getting null flags
    - Implement type-specific extractors (uuid, text, i32, timestamp, jsonb)
    - _Requirements: 1.2, 2.2, 3.2_

  - [x] 1.4 Wire helper modules into lib.rs
    - Add `mod heap_ops;` declaration
    - Add `mod index_ops;` declaration
    - Add `mod tuple_extract;` declaration
    - _Requirements: All entity requirements_

  - [x] 1.5 Create column mapping constants for all entity tables
    - Define `trajectory_cols` module with column positions
    - Define `scope_cols` module with column positions
    - Define `artifact_cols` module with column positions
    - Define `note_cols` module with column positions
    - Define `turn_cols` module with column positions
    - Define `lock_cols` module with column positions
    - Define `message_cols` module with column positions
    - Define `agent_cols` module with column positions
    - Define `delegation_cols` module with column positions
    - Define `handoff_cols` module with column positions
    - Define `conflict_cols` module with column positions
    - _Requirements: All entity requirements_

- [x] 2. Migrate Trajectory operations to direct heap
  - [x] 2.1 Implement `trajectory_create_heap()`
    - Build datum array from parameters
    - Form tuple and insert into heap
    - Update all indexes via CatalogIndexInsert
    - _Requirements: 1.1, 1.6_

  - [x] 2.2 Implement `trajectory_get_heap()`
    - Open primary key index
    - Build scan key for trajectory_id
    - Execute index scan and extract tuple
    - Convert tuple to Trajectory struct
    - Return None if not found
    - _Requirements: 1.2, 1.7_

  - [x] 2.3 Implement `trajectory_update_heap()`
    - Find existing tuple via index scan
    - Extract current values
    - Apply updates to datum array
    - Form new tuple and call simple_heap_update
    - Update indexes if indexed columns changed
    - _Requirements: 1.3_

  - [x] 2.4 Implement `trajectory_set_status_heap()`
    - Find existing tuple via index scan
    - Update status and updated_at fields
    - Form new tuple and call simple_heap_update
    - Update status index
    - _Requirements: 1.4_

  - [x] 2.5 Implement `trajectory_list_by_status_heap()`
    - Open status index
    - Build scan key for status value
    - Iterate index scan collecting matching tuples
    - Convert each tuple to Trajectory
    - _Requirements: 1.5_

  - [x] 2.6 Write property test for Trajectory round-trip
    - **Property 1: Insert-Get Round Trip (Trajectory)**
    - Generate random Trajectory data using proptest
    - Insert via heap, get via heap, compare
    - **Validates: Requirements 1.1, 1.2, 1.7**

  - [x] 2.7 Write property test for Trajectory update persistence
    - **Property 2: Update Persistence (Trajectory)**
    - Generate random Trajectory and updates
    - Apply update, get, verify updated values
    - **Validates: Requirements 1.3, 1.4**

- [x] 3. Migrate Scope operations to direct heap
  - [x] 3.1 Implement `scope_create_heap()`
    - Build datum array from parameters
    - Form tuple and insert into heap
    - Update trajectory_id index
    - _Requirements: 2.1, 2.6_

  - [x] 3.2 Implement `scope_get_heap()`
    - Index scan on primary key
    - Convert tuple to Scope struct
    - _Requirements: 2.2_

  - [x] 3.3 Implement `scope_close_heap()`
    - Find tuple, update is_active and closed_at
    - _Requirements: 2.3_

  - [x] 3.4 Implement `scope_list_by_trajectory_heap()`
    - Index scan on trajectory_id
    - Collect and return matching Scopes
    - _Requirements: 2.4_

  - [x] 3.5 Implement `scope_update_tokens_heap()`
    - Find tuple, update tokens_used
    - _Requirements: 2.5_

  - [x] 3.6 Write property test for Scope round-trip
    - **Property 1: Insert-Get Round Trip (Scope)**
    - Generate random Scope data, insert, get, compare
    - **Validates: Requirements 2.1, 2.2**

- [x] 4. Migrate Artifact operations to direct heap
  - [x] 4.1 Implement `artifact_create_heap()`
    - Handle embedding vector storage
    - Update btree and hnsw indexes
    - _Requirements: 3.1, 3.6_

  - [x] 4.2 Implement `artifact_get_heap()`
    - Index scan on primary key
    - _Requirements: 3.2_

  - [x] 4.3 Implement `artifact_query_by_type_heap()`
    - Index scan on artifact_type
    - _Requirements: 3.3_

  - [x] 4.4 Implement `artifact_query_by_scope_heap()`
    - Index scan on scope_id
    - _Requirements: 3.4_

  - [x] 4.5 Implement `artifact_update_heap()`
    - Update content and embedding if changed
    - _Requirements: 3.5_

  - [x] 4.6 Write property test for Artifact round-trip
    - **Property 1: Insert-Get Round Trip (Artifact)**
    - Generate random Artifact data, insert, get, compare
    - **Validates: Requirements 3.1, 3.2**

- [x] 5. Migrate Note operations to direct heap
  - [x] 5.1 Implement `note_create_heap()`
    - Handle embedding vector storage
    - Update btree and hnsw indexes
    - _Requirements: 4.1, 4.5_

  - [x] 5.2 Implement `note_get_heap()`
    - Index scan on primary key
    - _Requirements: 4.2_

  - [x] 5.3 Implement `note_query_by_trajectory_heap()`
    - Index scan on source_trajectory_ids
    - _Requirements: 4.3_

  - [x] 5.4 Implement `note_update_heap()`
    - Update content and embedding if changed
    - _Requirements: 4.4_

  - [x] 5.5 Write property test for Note round-trip
    - **Property 1: Insert-Get Round Trip (Note)**
    - Generate random Note data, insert, get, compare
    - **Validates: Requirements 4.1, 4.2**

- [x] 6. Migrate Turn operations to direct heap
  - [x] 6.1 Implement `turn_create_heap()`
    - Insert turn with scope_id index update
    - _Requirements: 5.1, 5.3_

  - [x] 6.2 Implement `turn_get_by_scope_heap()`
    - Index scan on scope_id
    - Return ordered by sequence
    - _Requirements: 5.2_

  - [x] 6.3 Write property test for Turn round-trip
    - **Property 1: Insert-Get Round Trip (Turn)**
    - Generate random Turn data, insert, get by scope, verify
    - **Validates: Requirements 5.1, 5.2**

- [x] 7. Migrate Lock operations to direct heap
  - [x] 7.1 Create `lock_heap.rs` module
    - Implement `lock_acquire_heap()` - Insert lock record
    - Implement `lock_release_heap()` - Delete lock record via simple_heap_delete
    - Implement `lock_get_heap()` - Index scan on primary key
    - Implement `lock_list_by_resource_heap()` - Index scan on resource_type + resource_id
    - Continue using pg_advisory_lock for actual locking
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

  - [x] 7.2 Write property test for Lock round-trip and delete
    - **Property 1: Insert-Get Round Trip (Lock)**
    - **Property 4: Delete Removes from Index**
    - Generate random Lock, insert, get, delete, verify not found
    - **Validates: Requirements 6.1, 6.2, 6.3**

- [x] 8. Migrate Message operations to direct heap
  - [x] 8.1 Create `message_heap.rs` module
    - Implement `message_send_heap()` - Insert message record
    - Implement `message_get_heap()` - Index scan on primary key
    - Implement `message_list_for_agent_heap()` - Index scan on to_agent_id
    - Implement `message_acknowledge_heap()` - Update acknowledged_at field
    - Continue using pg_notify for delivery
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

  - [x] 8.2 Write property test for Message round-trip
    - **Property 1: Insert-Get Round Trip (Message)**
    - Generate random Message, insert, get, compare
    - **Validates: Requirements 7.1, 7.2**

- [x] 9. Migrate Agent operations to direct heap
  - [x] 9.1 Create `agent_heap.rs` module
    - Implement `agent_register_heap()` - Insert agent record
    - Implement `agent_get_heap()` - Index scan on primary key
    - Implement `agent_heartbeat_heap()` - Update last_heartbeat field
    - Implement `agent_set_status_heap()` - Update status field
    - Implement `agent_list_by_type_heap()` - Index scan on agent_type
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

  - [x] 9.2 Write property test for Agent round-trip
    - **Property 1: Insert-Get Round Trip (Agent)**
    - Generate random Agent, insert, get, compare
    - **Validates: Requirements 8.1, 8.2**

- [x] 10. Migrate Delegation operations to direct heap
  - [x] 10.1 Create `delegation_heap.rs` module
    - Implement `delegation_create_heap()` - Insert delegation record
    - Implement `delegation_get_heap()` - Index scan on primary key
    - Implement `delegation_accept_heap()` - Update status and accepted_at
    - Implement `delegation_complete_heap()` - Update status, result, completed_at
    - Implement `delegation_list_pending_heap()` - Index scan on status = 'pending'
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

  - [x] 10.2 Write property test for Delegation round-trip
    - **Property 1: Insert-Get Round Trip (Delegation)**
    - Generate random Delegation, insert, get, compare
    - **Validates: Requirements 9.1, 9.2**

- [x] 11. Migrate Handoff operations to direct heap
  - [x] 11.1 Create `handoff_heap.rs` module
    - Implement `handoff_create_heap()` - Insert handoff record
    - Implement `handoff_get_heap()` - Index scan on primary key
    - Implement `handoff_accept_heap()` - Update status and accepted_at
    - Implement `handoff_complete_heap()` - Update status and completed_at
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

  - [x] 11.2 Write property test for Handoff round-trip
    - **Property 1: Insert-Get Round Trip (Handoff)**
    - Generate random Handoff, insert, get, compare
    - **Validates: Requirements 10.1, 10.2**

- [x] 12. Migrate Conflict operations to direct heap
  - [x] 12.1 Create `conflict_heap.rs` module
    - Implement `conflict_create_heap()` - Insert conflict record
    - Implement `conflict_get_heap()` - Index scan on primary key
    - Implement `conflict_resolve_heap()` - Update status, resolution, resolved_at
    - Implement `conflict_list_pending_heap()` - Index scan on status = 'detected' or 'resolving'
    - _Requirements: 11.1, 11.2, 11.3, 11.4_

  - [x] 12.2 Write property test for Conflict round-trip
    - **Property 1: Insert-Get Round Trip (Conflict)**
    - Generate random Conflict, insert, get, compare
    - **Validates: Requirements 11.1, 11.2**

- [x] 13. Update pg_extern functions to use heap implementations
  - [x] 13.1 Update `caliber_trajectory_*` functions
    - Replace SPI calls with heap function calls
    - Maintain same function signatures
    - _Requirements: 1.1-1.7_

  - [x] 13.2 Update `caliber_scope_*` functions
    - Replace SPI calls with heap function calls
    - _Requirements: 2.1-2.6_

  - [x] 13.3 Update `caliber_artifact_*` functions
    - Replace SPI calls with heap function calls
    - _Requirements: 3.1-3.6_

  - [x] 13.4 Update `caliber_note_*` functions
    - Replace SPI calls with heap function calls
    - _Requirements: 4.1-4.5_

  - [x] 13.5 Update `caliber_turn_*` functions
    - Replace SPI calls with heap function calls
    - _Requirements: 5.1-5.3_

  - [x] 13.6 Update `caliber_lock_*` functions
    - Replace SPI calls with heap function calls
    - Keep pg_advisory_lock usage
    - _Requirements: 6.1-6.5_

  - [x] 13.7 Update `caliber_message_*` functions
    - Replace SPI calls with heap function calls
    - Keep pg_notify usage
    - _Requirements: 7.1-7.5_

  - [x] 13.8 Update `caliber_agent_*` functions
    - Replace SPI calls with heap function calls
    - _Requirements: 8.1-8.5_

  - [x] 13.9 Update `caliber_delegation_*` functions
    - Replace SPI calls with heap function calls
    - _Requirements: 9.1-9.5_

  - [x] 13.10 Update `caliber_handoff_*` functions
    - Replace SPI calls with heap function calls
    - _Requirements: 10.1-10.4_

  - [x] 13.11 Update `caliber_conflict_*` functions
    - Replace SPI calls with heap function calls
    - _Requirements: 11.1-11.4_

- [x] 14. Checkpoint - Verify all entity operations work
  - Ensure all tests pass, ask the user if questions arise.
  - Run `cargo pgrx test` to verify pg_extern functions
  - Verify no SPI usage in hot-path functions

- [x] 15. Write cross-cutting property tests
  - [x] 15.1 Write property test for index consistency
    - **Property 3: Index Consistency**
    - For each entity type, insert then query via each index
    - Verify all indexes return correct results
    - **Validates: Requirements 13.1, 13.2, 13.4, 13.5**

  - [x] 15.2 Write property test for error propagation
    - **Property 7: Error Propagation**
    - Test invalid inputs return CaliberResult::Err
    - Verify no panics on storage errors
    - **Validates: Requirements 14.1, 14.3, 14.4, 14.5**

  - [x] 15.3 Write property test for timestamp consistency
    - **Property 6: Timestamp Consistency**
    - Verify created_at/updated_at use transaction timestamp
    - Multiple inserts in same transaction have same timestamp
    - **Validates: Requirements 15.1**

  - [x] 15.4 Write property test for transaction visibility
    - **Property 5: Transaction Visibility**
    - Sequence of ops in transaction see prior results
    - Insert then get in same transaction returns entity
    - **Validates: Requirements 15.4**

- [x] 16. Final checkpoint - All tests pass
  - Ensure all tests pass, ask the user if questions arise.
  - Run full test suite including property tests
  - Verify no regressions in existing functionality
  - Run `cargo clippy --workspace` - no warnings

## Notes

- All tasks are required - no optional markers
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties using proptest
- Cold-path operations (caliber_init, caliber_schema_exists) remain SPI-based per Requirement 12
- Follow steering: NO STUBS, complete code only, reference docs/DEPENDENCY_GRAPH.md
- Helper modules (heap_ops.rs, index_ops.rs) exist but need to be wired into lib.rs
