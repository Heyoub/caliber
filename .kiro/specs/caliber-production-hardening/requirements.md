# Requirements Document

## Introduction

This specification addresses critical architectural issues identified in the CALIBER codebase review. The current implementation uses in-memory storage that is session-local and lost on backend exit, has mismatched advisory lock semantics, silent defaults that violate the "no hard-coded values" philosophy, and missing access control enforcement. This spec covers all fixes needed to make CALIBER production-ready.

## Glossary

- **CALIBER_PG**: The pgrx-based PostgreSQL extension that provides the runtime
- **SPI**: Server Programming Interface - Postgres API for executing SQL from extensions
- **Advisory_Lock**: Postgres cooperative locking mechanism for application-level coordination
- **Session_Lock**: Advisory lock that persists until explicitly released or session ends
- **Transaction_Lock**: Advisory lock that auto-releases at transaction commit/rollback
- **Backend**: A Postgres backend process handling a single client connection
- **Shared_Memory**: Memory visible to all Postgres backends (requires extension setup)
- **PCP_Runtime**: The Persistent Context Protocol validation and checkpoint engine
- **DSL_Parser**: The CALIBER domain-specific language parser

## Requirements

### Requirement 1: SQL-Based Persistent Storage

**User Story:** As a developer, I want all CALIBER data to persist across backend restarts and be visible to all connections, so that the memory framework functions correctly in a multi-connection environment.

#### Acceptance Criteria

1. WHEN caliber_init() is called, THE CALIBER_PG SHALL execute the bootstrap SQL schema creating all required tables, indexes, and functions
2. WHEN any entity (trajectory, scope, artifact, note, turn) is created, THE CALIBER_PG SHALL insert it into the corresponding SQL table via SPI
3. WHEN any entity is queried, THE CALIBER_PG SHALL retrieve it from the SQL table via SPI
4. WHEN any entity is updated, THE CALIBER_PG SHALL update the corresponding SQL row via SPI
5. WHEN a Postgres backend exits, THE CALIBER_PG SHALL preserve all data in SQL tables
6. WHEN multiple backends query the same entity, THE CALIBER_PG SHALL return consistent data from the shared SQL tables

### Requirement 2: Correct Advisory Lock Semantics

**User Story:** As a developer, I want advisory locks to work correctly across transactions and sessions, so that distributed coordination is reliable.

#### Acceptance Criteria

1. WHEN caliber_lock_acquire is called with session mode, THE CALIBER_PG SHALL use pg_try_advisory_lock (session-level)
2. WHEN caliber_lock_release is called for a session lock, THE CALIBER_PG SHALL use pg_advisory_unlock (session-level)
3. WHEN caliber_lock_acquire is called with transaction mode, THE CALIBER_PG SHALL use pg_try_advisory_xact_lock (transaction-level)
4. WHEN a transaction-level lock is acquired, THE CALIBER_PG SHALL NOT provide an explicit release function (auto-releases at transaction end)
5. WHEN computing lock keys, THE CALIBER_PG SHALL use a deterministic hash algorithm (FNV-1a or SipHash with fixed seed) that is stable across Rust versions
6. WHEN a lock is acquired, THE CALIBER_PG SHALL store the lock record in the SQL locks table for cross-session visibility
7. WHEN timeout_ms is specified, THE CALIBER_PG SHALL attempt lock acquisition with the specified timeout using pg_advisory_lock with statement_timeout

### Requirement 3: Message Persistence and Cross-Session Delivery

**User Story:** As a developer, I want messages to persist and be retrievable by any session, so that multi-agent communication works across connections.

#### Acceptance Criteria

1. WHEN caliber_message_send is called, THE CALIBER_PG SHALL insert the message into the SQL messages table
2. WHEN caliber_message_get is called, THE CALIBER_PG SHALL retrieve the message from the SQL messages table
3. WHEN caliber_message_get_pending is called, THE CALIBER_PG SHALL query the SQL messages table for undelivered messages matching the agent
4. WHEN pg_notify is called, THE CALIBER_PG SHALL include the full message_id in the payload for cross-session lookup
5. IF pg_notify fails, THEN THE CALIBER_PG SHALL return an error rather than silently ignoring the failure
6. WHEN a message expires, THE CALIBER_PG SHALL mark it as expired in the SQL table (not delete immediately for audit)

### Requirement 4: Access Control Enforcement

**User Story:** As a developer, I want memory region access controls to be enforced, so that agents cannot access data they shouldn't.

#### Acceptance Criteria

1. WHEN an agent attempts to read from a memory region, THE CALIBER_PG SHALL verify the agent has read permission via MemoryRegionConfig::can_read
2. WHEN an agent attempts to write to a memory region, THE CALIBER_PG SHALL verify the agent has write permission via MemoryRegionConfig::can_write
3. IF an agent lacks required permission, THEN THE CALIBER_PG SHALL return AgentError::PermissionDenied
4. WHEN a collaborative region requires a lock, THE CALIBER_PG SHALL verify the agent holds the lock before allowing writes
5. WHEN memory regions are created, THE CALIBER_PG SHALL store them in the SQL regions table for persistence

### Requirement 5: Strict DSL Parsing (No Silent Defaults)

**User Story:** As a developer, I want the DSL parser to reject incomplete configurations rather than supplying defaults, so that misconfigurations are caught early.

#### Acceptance Criteria

1. WHEN an adapter definition is missing the 'type' field, THE DSL_Parser SHALL return ParseError with "missing required field: type"
2. WHEN an adapter definition has an empty connection string, THE DSL_Parser SHALL return ParseError with "connection string cannot be empty"
3. WHEN a memory definition is missing the 'type' field, THE DSL_Parser SHALL return ParseError with "missing required field: type"
4. WHEN a memory definition is missing the 'retention' field, THE DSL_Parser SHALL return ParseError with "missing required field: retention"
5. WHEN an injection definition is missing the 'priority' field, THE DSL_Parser SHALL return ParseError with "missing required field: priority"
6. WHEN an unknown field is encountered, THE DSL_Parser SHALL return ParseError with "unknown field: {name}"
7. THE DSL_Parser SHALL NOT supply default values for any required field

### Requirement 6: PCP Configuration Without Defaults

**User Story:** As a developer, I want PCP to require explicit configuration for all thresholds and limits, so that behavior is predictable and intentional.

#### Acceptance Criteria

1. WHEN PCPRuntime is created without a config, THE PCP_Runtime SHALL return ConfigError::MissingRequired
2. WHEN PCPConfig is missing max_tokens_per_scope, THE PCP_Runtime SHALL return ConfigError::MissingRequired
3. WHEN PCPConfig is missing contradiction_threshold, THE PCP_Runtime SHALL return ConfigError::MissingRequired
4. WHEN PCPConfig is missing max_checkpoints, THE PCP_Runtime SHALL return ConfigError::MissingRequired
5. THE PCPConfig SHALL NOT implement Default trait
6. WHEN checkpoints are created, THE PCP_Runtime SHALL persist them to the SQL checkpoints table
7. WHEN recovery is requested, THE PCP_Runtime SHALL load checkpoints from the SQL checkpoints table

### Requirement 7: EntityType Completeness

**User Story:** As a developer, I want all entity types to be represented in the EntityType enum, so that error messages are accurate.

#### Acceptance Criteria

1. THE EntityType enum SHALL include a Turn variant
2. WHEN a turn operation fails, THE Storage layer SHALL use EntityType::Turn in the error
3. WHEN a lock operation fails, THE Storage layer SHALL use EntityType::Lock in the error (add Lock variant)
4. WHEN a message operation fails, THE Storage layer SHALL use EntityType::Message in the error (add Message variant)

### Requirement 8: Note Access Tracking

**User Story:** As a developer, I want note access metrics to be updated on reads, so that access patterns can be analyzed.

#### Acceptance Criteria

1. WHEN caliber_note_get is called, THE CALIBER_PG SHALL increment access_count by 1
2. WHEN caliber_note_get is called, THE CALIBER_PG SHALL update accessed_at to current timestamp
3. WHEN caliber_note_query_by_trajectory is called, THE CALIBER_PG SHALL update access metrics for all returned notes

### Requirement 9: Turn Uniqueness Enforcement

**User Story:** As a developer, I want turn sequence uniqueness to be enforced, so that conversation ordering is consistent.

#### Acceptance Criteria

1. WHEN caliber_turn_create is called, THE CALIBER_PG SHALL verify the scope_id exists
2. WHEN caliber_turn_create is called with a duplicate (scope_id, sequence) pair, THE CALIBER_PG SHALL return StorageError::InsertFailed with "duplicate sequence"
3. THE SQL turns table SHALL have a UNIQUE constraint on (scope_id, sequence)

### Requirement 10: Safe String Handling

**User Story:** As a developer, I want string operations to handle Unicode correctly, so that non-ASCII text doesn't cause panics.

#### Acceptance Criteria

1. WHEN extract_first_sentence truncates text, THE PCP_Runtime SHALL use char boundaries, not byte indices
2. WHEN truncating to 200 characters, THE PCP_Runtime SHALL count characters, not bytes
3. FOR ALL valid UTF-8 strings, truncation operations SHALL NOT panic

### Requirement 11: Explicit Error Handling (No Silent Failures)

**User Story:** As a developer, I want all operations to report errors explicitly, so that failures are visible.

#### Acceptance Criteria

1. WHEN pg_notify fails, THE CALIBER_PG SHALL return CaliberError::Agent with the failure reason
2. WHEN SPI operations fail, THE CALIBER_PG SHALL return CaliberError::Storage with the failure reason
3. WHEN RwLock is poisoned, THE CALIBER_PG SHALL return CaliberError::Storage with "lock poisoned" rather than panicking
4. WHEN JSON serialization fails, THE CALIBER_PG SHALL return CaliberError::Validation rather than returning null

### Requirement 12: Unknown Input Rejection

**User Story:** As a developer, I want unknown enum values to be rejected rather than silently defaulting, so that typos are caught.

#### Acceptance Criteria

1. WHEN caliber_artifact_create receives an unknown artifact_type string, THE CALIBER_PG SHALL return ValidationError::InvalidValue
2. WHEN caliber_note_create receives an unknown note_type string, THE CALIBER_PG SHALL return ValidationError::InvalidValue
3. WHEN caliber_turn_create receives an unknown role string, THE CALIBER_PG SHALL return ValidationError::InvalidValue
4. WHEN caliber_trajectory_set_status receives an unknown status string, THE CALIBER_PG SHALL return ValidationError::InvalidValue

### Requirement 13: Debug Endpoint Protection

**User Story:** As a developer, I want debug endpoints to be protected, so that production data cannot be accidentally wiped.

#### Acceptance Criteria

1. THE caliber_debug_clear and caliber_debug_dump functions SHALL only be available when compiled with the "debug" feature flag
2. WHEN the "debug" feature is not enabled, THE CALIBER_PG SHALL NOT expose debug functions
3. WHEN debug functions are called, THE CALIBER_PG SHALL log a warning to the Postgres log

### Requirement 14: Context Section Priority Configuration

**User Story:** As a developer, I want persona sections to have their own priority setting, so that context assembly is fully configurable.

#### Acceptance Criteria

1. THE SectionPriorities struct SHALL include a 'persona' field separate from 'system'
2. WHEN assembling context with a kernel config, THE Context_Assembler SHALL use the persona priority for persona sections
3. THE CaliberConfig SHALL require an explicit persona priority value
