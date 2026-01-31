# Requirements Document

## Introduction

This spec covers migrating `caliber-pg` from SPI-based SQL operations to direct heap operations using pgrx. The CALIBER spec mandates NO SQL IN HOT PATH - all entity CRUD operations must bypass SQL parsing entirely for maximum performance. This is a prerequisite for the caliber-api spec.

## Glossary

- **SPI**: Server Programming Interface - PostgreSQL's internal API for executing SQL from within extensions. Still involves SQL parsing.
- **Direct_Heap**: pgrx functions that manipulate PostgreSQL heap tuples directly without SQL parsing (heap_form_tuple, simple_heap_insert, etc.)
- **Hot_Path**: Frequently executed code paths that must be optimized for performance (entity CRUD operations)
- **Cold_Path**: Infrequently executed code paths where SQL overhead is acceptable (schema init, migrations)
- **pg_extern**: pgrx macro that exposes Rust functions as PostgreSQL functions

## Requirements

### Requirement 1: Trajectory Direct Heap Operations

**User Story:** As a system, I want trajectory operations to bypass SQL parsing, so that hot-path performance is maximized.

#### Acceptance Criteria

1. THE caliber_trajectory_create function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_trajectory_get function SHALL use index_beginscan for O(log n) lookup instead of SPI SELECT
3. THE caliber_trajectory_update function SHALL use simple_heap_update instead of SPI UPDATE
4. THE caliber_trajectory_set_status function SHALL use simple_heap_update instead of SPI UPDATE
5. THE caliber_trajectory_list_by_status function SHALL use heap_beginscan with index for filtering instead of SPI SELECT
6. WHEN a trajectory is inserted, THE System SHALL update all relevant indexes via CatalogIndexInsert
7. IF a trajectory is not found, THEN THE System SHALL return None without SQL error overhead

### Requirement 2: Scope Direct Heap Operations

**User Story:** As a system, I want scope operations to bypass SQL parsing, so that context partitioning is fast.

#### Acceptance Criteria

1. THE caliber_scope_create function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_scope_get function SHALL use index_beginscan instead of SPI SELECT
3. THE caliber_scope_close function SHALL use simple_heap_update instead of SPI UPDATE
4. THE caliber_scope_list_by_trajectory function SHALL use index scan on trajectory_id instead of SPI SELECT
5. THE caliber_scope_update_tokens function SHALL use simple_heap_update instead of SPI UPDATE
6. WHEN a scope is inserted, THE System SHALL update all relevant indexes

### Requirement 3: Artifact Direct Heap Operations

**User Story:** As a system, I want artifact operations to bypass SQL parsing, so that knowledge extraction is fast.

#### Acceptance Criteria

1. THE caliber_artifact_create function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_artifact_get function SHALL use index_beginscan instead of SPI SELECT
3. THE caliber_artifact_query_by_type function SHALL use index scan instead of SPI SELECT
4. THE caliber_artifact_query_by_scope function SHALL use index scan instead of SPI SELECT
5. THE caliber_artifact_update function SHALL use simple_heap_update instead of SPI UPDATE
6. WHEN an artifact is inserted, THE System SHALL update btree and hnsw indexes

### Requirement 4: Note Direct Heap Operations

**User Story:** As a system, I want note operations to bypass SQL parsing, so that cross-trajectory knowledge is fast.

#### Acceptance Criteria

1. THE caliber_note_create function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_note_get function SHALL use index_beginscan instead of SPI SELECT
3. THE caliber_note_query_by_trajectory function SHALL use index scan instead of SPI SELECT
4. THE caliber_note_update function SHALL use simple_heap_update instead of SPI UPDATE
5. WHEN a note is inserted, THE System SHALL update btree and hnsw indexes

### Requirement 5: Turn Direct Heap Operations

**User Story:** As a system, I want turn operations to bypass SQL parsing, so that conversation buffering is fast.

#### Acceptance Criteria

1. THE caliber_turn_create function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_turn_get_by_scope function SHALL use index scan on scope_id instead of SPI SELECT
3. WHEN a turn is inserted, THE System SHALL update the scope_id index

### Requirement 6: Lock Direct Heap Operations

**User Story:** As a system, I want lock operations to bypass SQL parsing, so that distributed coordination is fast.

#### Acceptance Criteria

1. THE caliber_lock_acquire function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_lock_release function SHALL use simple_heap_delete instead of SPI DELETE
3. THE caliber_lock_get function SHALL use index_beginscan instead of SPI SELECT
4. THE caliber_lock_list_by_resource function SHALL use index scan instead of SPI SELECT
5. THE System SHALL continue to use pg_advisory_lock for actual locking semantics

### Requirement 7: Message Direct Heap Operations

**User Story:** As a system, I want message operations to bypass SQL parsing, so that inter-agent communication is fast.

#### Acceptance Criteria

1. THE caliber_message_send function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_message_get function SHALL use index_beginscan instead of SPI SELECT
3. THE caliber_message_list_for_agent function SHALL use index scan instead of SPI SELECT
4. THE caliber_message_acknowledge function SHALL use simple_heap_update instead of SPI UPDATE
5. THE System SHALL continue to use pg_notify for real-time message delivery

### Requirement 8: Agent Direct Heap Operations

**User Story:** As a system, I want agent operations to bypass SQL parsing, so that agent coordination is fast.

#### Acceptance Criteria

1. THE caliber_agent_register function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_agent_get function SHALL use index_beginscan instead of SPI SELECT
3. THE caliber_agent_heartbeat function SHALL use simple_heap_update instead of SPI UPDATE
4. THE caliber_agent_set_status function SHALL use simple_heap_update instead of SPI UPDATE
5. THE caliber_agent_list_by_type function SHALL use index scan instead of SPI SELECT

### Requirement 9: Delegation Direct Heap Operations

**User Story:** As a system, I want delegation operations to bypass SQL parsing, so that task delegation is fast.

#### Acceptance Criteria

1. THE caliber_delegation_create function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_delegation_get function SHALL use index_beginscan instead of SPI SELECT
3. THE caliber_delegation_accept function SHALL use simple_heap_update instead of SPI UPDATE
4. THE caliber_delegation_complete function SHALL use simple_heap_update instead of SPI UPDATE
5. THE caliber_delegation_list_pending function SHALL use index scan instead of SPI SELECT

### Requirement 10: Handoff Direct Heap Operations

**User Story:** As a system, I want handoff operations to bypass SQL parsing, so that agent handoffs are fast.

#### Acceptance Criteria

1. THE caliber_handoff_create function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_handoff_get function SHALL use index_beginscan instead of SPI SELECT
3. THE caliber_handoff_accept function SHALL use simple_heap_update instead of SPI UPDATE
4. THE caliber_handoff_complete function SHALL use simple_heap_update instead of SPI UPDATE

### Requirement 11: Conflict Direct Heap Operations

**User Story:** As a system, I want conflict operations to bypass SQL parsing, so that conflict resolution is fast.

#### Acceptance Criteria

1. THE caliber_conflict_create function SHALL use heap_form_tuple and simple_heap_insert instead of SPI
2. THE caliber_conflict_get function SHALL use index_beginscan instead of SPI SELECT
3. THE caliber_conflict_resolve function SHALL use simple_heap_update instead of SPI UPDATE
4. THE caliber_conflict_list_pending function SHALL use index scan instead of SPI SELECT

### Requirement 12: Cold Path Operations (SPI Acceptable)

**User Story:** As a system, I want schema initialization to remain SPI-based, so that complexity is contained to hot paths.

#### Acceptance Criteria

1. THE caliber_init function MAY continue to use SPI for schema creation (cold path)
2. THE caliber_schema_exists function MAY continue to use SPI for schema checks (cold path)
3. THE caliber_version function SHALL NOT use SPI (returns static string)

### Requirement 13: Index Management

**User Story:** As a system, I want indexes to be properly maintained during direct heap operations, so that queries remain fast.

#### Acceptance Criteria

1. WHEN an entity is inserted via direct heap, THE System SHALL call CatalogIndexInsert for all indexes
2. WHEN an entity is updated via direct heap, THE System SHALL call CatalogIndexInsert if indexed columns changed
3. WHEN an entity is deleted via direct heap, THE System SHALL call CatalogIndexDelete
4. THE System SHALL use index_beginscan for all lookup operations
5. THE System SHALL use heap_beginscan with index for all list/filter operations

### Requirement 14: Error Handling

**User Story:** As a system, I want direct heap operations to return proper CaliberError types, so that error handling is consistent.

#### Acceptance Criteria

1. IF a relation cannot be opened, THEN THE System SHALL return StorageError::SpiError with descriptive message
2. IF an index scan finds no results, THEN THE System SHALL return None (not an error)
3. IF a heap insert fails, THEN THE System SHALL return StorageError::InsertFailed
4. IF a heap update fails, THEN THE System SHALL return StorageError::UpdateFailed
5. THE System SHALL NOT panic on storage errors - all errors must be CaliberResult

### Requirement 15: Transaction Safety

**User Story:** As a system, I want direct heap operations to respect PostgreSQL transaction semantics, so that ACID properties are maintained.

#### Acceptance Criteria

1. THE System SHALL use GetCurrentTransactionStartTimestamp for created_at/updated_at fields
2. THE System SHALL NOT commit or rollback transactions - let PostgreSQL handle this
3. THE System SHALL use appropriate lock modes when opening relations
4. IF an operation is within a transaction, THEN changes SHALL be visible to subsequent operations in the same transaction

