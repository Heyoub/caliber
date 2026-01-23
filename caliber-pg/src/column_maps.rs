//! Column mapping constants for all CALIBER entity tables.
//!
//! When using direct heap operations, we need to know the exact column positions
//! for each table. These constants define the 1-based attribute numbers for
//! each column in each entity table.
//!
//! # Important
//!
//! These mappings MUST match the column order in `sql/caliber_init.sql`.
//! If the schema changes, these constants must be updated accordingly.
//!
//! # Usage
//!
//! ```ignore
//! use crate::column_maps::trajectory;
//!
//! // Extract trajectory_id from column 1
//! let id = extract_uuid(tuple, desc, trajectory::TRAJECTORY_ID)?;
//!
//! // Extract name from column 2
//! let name = extract_text(tuple, desc, trajectory::NAME)?;
//! ```

// ============================================================================
// TRAJECTORY TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_trajectory` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_trajectory (
///     trajectory_id UUID PRIMARY KEY,           -- 1
///     name TEXT NOT NULL,                       -- 2
///     description TEXT,                         -- 3
///     status TEXT NOT NULL,                     -- 4
///     parent_trajectory_id UUID,                -- 5
///     root_trajectory_id UUID,                  -- 6
///     agent_id UUID,                            -- 7
///     created_at TIMESTAMPTZ NOT NULL,          -- 8
///     updated_at TIMESTAMPTZ NOT NULL,          -- 9
///     completed_at TIMESTAMPTZ,                 -- 10
///     outcome JSONB,                            -- 11
///     metadata JSONB,                           -- 12
///     tenant_id UUID                            -- 13
/// );
/// ```
pub mod trajectory {
    /// trajectory_id UUID PRIMARY KEY
    pub const TRAJECTORY_ID: i16 = 1;
    /// name TEXT NOT NULL
    pub const NAME: i16 = 2;
    /// description TEXT
    pub const DESCRIPTION: i16 = 3;
    /// status TEXT NOT NULL ('active', 'completed', 'failed', 'suspended')
    pub const STATUS: i16 = 4;
    /// parent_trajectory_id UUID (FK)
    pub const PARENT_TRAJECTORY_ID: i16 = 5;
    /// root_trajectory_id UUID (FK)
    pub const ROOT_TRAJECTORY_ID: i16 = 6;
    /// agent_id UUID
    pub const AGENT_ID: i16 = 7;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 8;
    /// updated_at TIMESTAMPTZ NOT NULL
    pub const UPDATED_AT: i16 = 9;
    /// completed_at TIMESTAMPTZ
    pub const COMPLETED_AT: i16 = 10;
    /// outcome JSONB
    pub const OUTCOME: i16 = 11;
    /// metadata JSONB
    pub const METADATA: i16 = 12;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 13;

    /// Total number of columns in the trajectory table
    pub const NUM_COLS: usize = 13;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_trajectory";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_trajectory_pkey";
    /// Status index name
    pub const STATUS_INDEX: &str = "idx_trajectory_status";
}


// ============================================================================
// SCOPE TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_scope` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_scope (
///     scope_id UUID PRIMARY KEY,                -- 1
///     trajectory_id UUID NOT NULL,              -- 2
///     parent_scope_id UUID,                     -- 3
///     name TEXT NOT NULL,                       -- 4
///     purpose TEXT,                             -- 5
///     is_active BOOLEAN NOT NULL,               -- 6
///     created_at TIMESTAMPTZ NOT NULL,          -- 7
///     closed_at TIMESTAMPTZ,                    -- 8
///     checkpoint JSONB,                         -- 9
///     token_budget INTEGER NOT NULL,            -- 10
///     tokens_used INTEGER NOT NULL,             -- 11
///     metadata JSONB,                           -- 12
///     tenant_id UUID                            -- 13
/// );
/// ```
pub mod scope {
    /// scope_id UUID PRIMARY KEY
    pub const SCOPE_ID: i16 = 1;
    /// trajectory_id UUID NOT NULL (FK)
    pub const TRAJECTORY_ID: i16 = 2;
    /// parent_scope_id UUID (FK)
    pub const PARENT_SCOPE_ID: i16 = 3;
    /// name TEXT NOT NULL
    pub const NAME: i16 = 4;
    /// purpose TEXT
    pub const PURPOSE: i16 = 5;
    /// is_active BOOLEAN NOT NULL
    pub const IS_ACTIVE: i16 = 6;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 7;
    /// closed_at TIMESTAMPTZ
    pub const CLOSED_AT: i16 = 8;
    /// checkpoint JSONB
    pub const CHECKPOINT: i16 = 9;
    /// token_budget INTEGER NOT NULL
    pub const TOKEN_BUDGET: i16 = 10;
    /// tokens_used INTEGER NOT NULL
    pub const TOKENS_USED: i16 = 11;
    /// metadata JSONB
    pub const METADATA: i16 = 12;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 13;

    /// Total number of columns in the scope table
    pub const NUM_COLS: usize = 13;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_scope";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_scope_pkey";
    /// Trajectory index name
    pub const TRAJECTORY_INDEX: &str = "idx_scope_trajectory";
}

// ============================================================================
// ARTIFACT TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_artifact` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_artifact (
///     artifact_id UUID PRIMARY KEY,             -- 1
///     trajectory_id UUID NOT NULL,              -- 2
///     scope_id UUID NOT NULL,                   -- 3
///     artifact_type TEXT NOT NULL,              -- 4
///     name TEXT NOT NULL,                       -- 5
///     content TEXT NOT NULL,                    -- 6
///     content_hash BYTEA NOT NULL,              -- 7
///     embedding VECTOR,                         -- 8
///     provenance JSONB NOT NULL,                -- 9
///     ttl TEXT NOT NULL,                        -- 10
///     created_at TIMESTAMPTZ NOT NULL,          -- 11
///     updated_at TIMESTAMPTZ NOT NULL,          -- 12
///     superseded_by UUID,                       -- 13
///     metadata JSONB,                           -- 14
///     tenant_id UUID                            -- 15
/// );
/// ```
pub mod artifact {
    /// artifact_id UUID PRIMARY KEY
    pub const ARTIFACT_ID: i16 = 1;
    /// trajectory_id UUID NOT NULL (FK)
    pub const TRAJECTORY_ID: i16 = 2;
    /// scope_id UUID NOT NULL (FK)
    pub const SCOPE_ID: i16 = 3;
    /// artifact_type TEXT NOT NULL
    pub const ARTIFACT_TYPE: i16 = 4;
    /// name TEXT NOT NULL
    pub const NAME: i16 = 5;
    /// content TEXT NOT NULL
    pub const CONTENT: i16 = 6;
    /// content_hash BYTEA NOT NULL
    pub const CONTENT_HASH: i16 = 7;
    /// embedding VECTOR (pgvector)
    pub const EMBEDDING: i16 = 8;
    /// provenance JSONB NOT NULL
    pub const PROVENANCE: i16 = 9;
    /// ttl TEXT NOT NULL
    pub const TTL: i16 = 10;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 11;
    /// updated_at TIMESTAMPTZ NOT NULL
    pub const UPDATED_AT: i16 = 12;
    /// superseded_by UUID (FK)
    pub const SUPERSEDED_BY: i16 = 13;
    /// metadata JSONB
    pub const METADATA: i16 = 14;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 15;

    /// Total number of columns in the artifact table
    pub const NUM_COLS: usize = 15;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_artifact";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_artifact_pkey";
    /// Trajectory index name
    pub const TRAJECTORY_INDEX: &str = "idx_artifact_trajectory";
    /// Scope index name
    pub const SCOPE_INDEX: &str = "idx_artifact_scope";
    /// Type index name
    pub const TYPE_INDEX: &str = "idx_artifact_type";
}


// ============================================================================
// NOTE TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_note` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_note (
///     note_id UUID PRIMARY KEY,                 -- 1
///     note_type TEXT NOT NULL,                  -- 2
///     title TEXT NOT NULL,                      -- 3
///     content TEXT NOT NULL,                    -- 4
///     content_hash BYTEA NOT NULL,              -- 5
///     embedding VECTOR,                         -- 6
///     source_trajectory_ids UUID[],             -- 7
///     source_artifact_ids UUID[],               -- 8
///     ttl TEXT NOT NULL,                        -- 9
///     created_at TIMESTAMPTZ NOT NULL,          -- 10
///     updated_at TIMESTAMPTZ NOT NULL,          -- 11
///     accessed_at TIMESTAMPTZ NOT NULL,         -- 12
///     access_count INTEGER NOT NULL,            -- 13
///     superseded_by UUID,                       -- 14
///     metadata JSONB,                           -- 15
///     abstraction_level TEXT NOT NULL,          -- 16 (Battle Intel Feature 2)
///     source_note_ids UUID[],                   -- 17 (Battle Intel Feature 2)
///     tenant_id UUID                            -- 18
/// );
/// ```
pub mod note {
    /// note_id UUID PRIMARY KEY
    pub const NOTE_ID: i16 = 1;
    /// note_type TEXT NOT NULL
    pub const NOTE_TYPE: i16 = 2;
    /// title TEXT NOT NULL
    pub const TITLE: i16 = 3;
    /// content TEXT NOT NULL
    pub const CONTENT: i16 = 4;
    /// content_hash BYTEA NOT NULL
    pub const CONTENT_HASH: i16 = 5;
    /// embedding VECTOR (pgvector)
    pub const EMBEDDING: i16 = 6;
    /// source_trajectory_ids UUID[]
    pub const SOURCE_TRAJECTORY_IDS: i16 = 7;
    /// source_artifact_ids UUID[]
    pub const SOURCE_ARTIFACT_IDS: i16 = 8;
    /// ttl TEXT NOT NULL
    pub const TTL: i16 = 9;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 10;
    /// updated_at TIMESTAMPTZ NOT NULL
    pub const UPDATED_AT: i16 = 11;
    /// accessed_at TIMESTAMPTZ NOT NULL
    pub const ACCESSED_AT: i16 = 12;
    /// access_count INTEGER NOT NULL
    pub const ACCESS_COUNT: i16 = 13;
    /// superseded_by UUID (FK)
    pub const SUPERSEDED_BY: i16 = 14;
    /// metadata JSONB
    pub const METADATA: i16 = 15;
    /// abstraction_level TEXT NOT NULL (Battle Intel Feature 2: L0/L1/L2)
    pub const ABSTRACTION_LEVEL: i16 = 16;
    /// source_note_ids UUID[] (Battle Intel Feature 2: derivation chain)
    pub const SOURCE_NOTE_IDS: i16 = 17;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 18;

    /// Total number of columns in the note table
    pub const NUM_COLS: usize = 18;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_note";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_note_pkey";
    /// Type index name
    pub const TYPE_INDEX: &str = "idx_note_type";
    /// Source trajectories index name
    pub const SOURCE_TRAJECTORIES_INDEX: &str = "idx_note_source_trajectories";
    /// Abstraction level index name (Battle Intel Feature 2)
    pub const ABSTRACTION_INDEX: &str = "idx_note_abstraction";
    /// Source notes index name (Battle Intel Feature 2)
    pub const SOURCE_NOTES_INDEX: &str = "idx_note_source_notes";
}

// ============================================================================
// TURN TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_turn` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_turn (
///     turn_id UUID PRIMARY KEY,                 -- 1
///     scope_id UUID NOT NULL,                   -- 2
///     sequence INTEGER NOT NULL,                -- 3
///     role TEXT NOT NULL,                       -- 4
///     content TEXT NOT NULL,                    -- 5
///     token_count INTEGER NOT NULL,             -- 6
///     created_at TIMESTAMPTZ NOT NULL,          -- 7
///     tool_calls JSONB,                         -- 8
///     tool_results JSONB,                       -- 9
///     metadata JSONB,                           -- 10
///     tenant_id UUID                            -- 11
/// );
/// ```
pub mod turn {
    /// turn_id UUID PRIMARY KEY
    pub const TURN_ID: i16 = 1;
    /// scope_id UUID NOT NULL (FK)
    pub const SCOPE_ID: i16 = 2;
    /// sequence INTEGER NOT NULL
    pub const SEQUENCE: i16 = 3;
    /// role TEXT NOT NULL ('user', 'assistant', 'system', 'tool')
    pub const ROLE: i16 = 4;
    /// content TEXT NOT NULL
    pub const CONTENT: i16 = 5;
    /// token_count INTEGER NOT NULL
    pub const TOKEN_COUNT: i16 = 6;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 7;
    /// tool_calls JSONB
    pub const TOOL_CALLS: i16 = 8;
    /// tool_results JSONB
    pub const TOOL_RESULTS: i16 = 9;
    /// metadata JSONB
    pub const METADATA: i16 = 10;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 11;

    /// Total number of columns in the turn table
    pub const NUM_COLS: usize = 11;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_turn";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_turn_pkey";
    /// Scope index name
    pub const SCOPE_INDEX: &str = "idx_turn_scope";
    /// Scope+sequence index name
    pub const SCOPE_SEQ_INDEX: &str = "idx_turn_scope_seq";
}


// ============================================================================
// AGENT TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_agent` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_agent (
///     agent_id UUID PRIMARY KEY,                -- 1
///     agent_type TEXT NOT NULL,                 -- 2
///     capabilities TEXT[],                      -- 3
///     memory_access JSONB NOT NULL,             -- 4
///     status TEXT NOT NULL,                     -- 5
///     current_trajectory_id UUID,               -- 6
///     current_scope_id UUID,                    -- 7
///     can_delegate_to TEXT[],                   -- 8
///     reports_to UUID,                          -- 9
///     created_at TIMESTAMPTZ NOT NULL,          -- 10
///     last_heartbeat TIMESTAMPTZ NOT NULL,      -- 11
///     tenant_id UUID                            -- 12
/// );
/// ```
pub mod agent {
    /// agent_id UUID PRIMARY KEY
    pub const AGENT_ID: i16 = 1;
    /// agent_type TEXT NOT NULL
    pub const AGENT_TYPE: i16 = 2;
    /// capabilities TEXT[]
    pub const CAPABILITIES: i16 = 3;
    /// memory_access JSONB NOT NULL
    pub const MEMORY_ACCESS: i16 = 4;
    /// status TEXT NOT NULL ('idle', 'active', 'blocked', 'failed')
    pub const STATUS: i16 = 5;
    /// current_trajectory_id UUID (FK)
    pub const CURRENT_TRAJECTORY_ID: i16 = 6;
    /// current_scope_id UUID (FK)
    pub const CURRENT_SCOPE_ID: i16 = 7;
    /// can_delegate_to TEXT[]
    pub const CAN_DELEGATE_TO: i16 = 8;
    /// reports_to UUID (FK)
    pub const REPORTS_TO: i16 = 9;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 10;
    /// last_heartbeat TIMESTAMPTZ NOT NULL
    pub const LAST_HEARTBEAT: i16 = 11;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 12;

    /// Total number of columns in the agent table
    pub const NUM_COLS: usize = 12;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_agent";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_agent_pkey";
    /// Type index name
    pub const TYPE_INDEX: &str = "idx_agent_type";
    /// Status index name
    pub const STATUS_INDEX: &str = "idx_agent_status";
}

// ============================================================================
// LOCK TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_lock` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_lock (
///     lock_id UUID PRIMARY KEY,                 -- 1
///     resource_type TEXT NOT NULL,              -- 2
///     resource_id UUID NOT NULL,                -- 3
///     holder_agent_id UUID NOT NULL,            -- 4
///     acquired_at TIMESTAMPTZ NOT NULL,         -- 5
///     expires_at TIMESTAMPTZ NOT NULL,          -- 6
///     mode TEXT NOT NULL,                       -- 7
///     tenant_id UUID,                           -- 8
///     version BIGINT NOT NULL                   -- 9 (V3: CAS)
/// );
/// ```
pub mod lock {
    /// lock_id UUID PRIMARY KEY
    pub const LOCK_ID: i16 = 1;
    /// resource_type TEXT NOT NULL
    pub const RESOURCE_TYPE: i16 = 2;
    /// resource_id UUID NOT NULL
    pub const RESOURCE_ID: i16 = 3;
    /// holder_agent_id UUID NOT NULL (FK)
    pub const HOLDER_AGENT_ID: i16 = 4;
    /// acquired_at TIMESTAMPTZ NOT NULL
    pub const ACQUIRED_AT: i16 = 5;
    /// expires_at TIMESTAMPTZ NOT NULL
    pub const EXPIRES_AT: i16 = 6;
    /// mode TEXT NOT NULL ('exclusive', 'shared')
    pub const MODE: i16 = 7;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 8;
    /// version BIGINT NOT NULL (V3: Compare-And-Swap version for optimistic locking)
    pub const VERSION: i16 = 9;

    /// Total number of columns in the lock table
    pub const NUM_COLS: usize = 9;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_lock";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_lock_pkey";
    /// Resource index name
    pub const RESOURCE_INDEX: &str = "idx_lock_resource";
    /// Holder index name
    pub const HOLDER_INDEX: &str = "idx_lock_holder";
}


// ============================================================================
// MESSAGE TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_message` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_message (
///     message_id UUID PRIMARY KEY,              -- 1
///     from_agent_id UUID NOT NULL,              -- 2
///     to_agent_id UUID,                         -- 3
///     to_agent_type TEXT,                       -- 4
///     message_type TEXT NOT NULL,               -- 5
///     payload TEXT NOT NULL,                    -- 6
///     trajectory_id UUID,                       -- 7
///     scope_id UUID,                            -- 8
///     artifact_ids UUID[],                      -- 9
///     created_at TIMESTAMPTZ NOT NULL,          -- 10
///     delivered_at TIMESTAMPTZ,                 -- 11
///     acknowledged_at TIMESTAMPTZ,              -- 12
///     priority TEXT NOT NULL,                   -- 13
///     expires_at TIMESTAMPTZ,                   -- 14
///     tenant_id UUID                            -- 15
/// );
/// ```
pub mod message {
    /// message_id UUID PRIMARY KEY
    pub const MESSAGE_ID: i16 = 1;
    /// from_agent_id UUID NOT NULL (FK)
    pub const FROM_AGENT_ID: i16 = 2;
    /// to_agent_id UUID (FK)
    pub const TO_AGENT_ID: i16 = 3;
    /// to_agent_type TEXT
    pub const TO_AGENT_TYPE: i16 = 4;
    /// message_type TEXT NOT NULL
    pub const MESSAGE_TYPE: i16 = 5;
    /// payload TEXT NOT NULL
    pub const PAYLOAD: i16 = 6;
    /// trajectory_id UUID (FK)
    pub const TRAJECTORY_ID: i16 = 7;
    /// scope_id UUID (FK)
    pub const SCOPE_ID: i16 = 8;
    /// artifact_ids UUID[]
    pub const ARTIFACT_IDS: i16 = 9;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 10;
    /// delivered_at TIMESTAMPTZ
    pub const DELIVERED_AT: i16 = 11;
    /// acknowledged_at TIMESTAMPTZ
    pub const ACKNOWLEDGED_AT: i16 = 12;
    /// priority TEXT NOT NULL ('low', 'normal', 'high', 'critical')
    pub const PRIORITY: i16 = 13;
    /// expires_at TIMESTAMPTZ
    pub const EXPIRES_AT: i16 = 14;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 15;

    /// Total number of columns in the message table
    pub const NUM_COLS: usize = 15;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_message";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_message_pkey";
    /// To agent index name
    pub const TO_AGENT_INDEX: &str = "idx_message_to_agent";
    /// Pending messages index name
    pub const PENDING_INDEX: &str = "idx_message_pending";
}

// ============================================================================
// DELEGATION TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_delegation` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_delegation (
///     delegation_id UUID PRIMARY KEY,           -- 1
///     delegator_agent_id UUID NOT NULL,         -- 2
///     delegatee_agent_id UUID,                  -- 3
///     delegatee_agent_type TEXT,                -- 4
///     task_description TEXT NOT NULL,           -- 5
///     parent_trajectory_id UUID NOT NULL,       -- 6
///     child_trajectory_id UUID,                 -- 7
///     shared_artifacts UUID[],                  -- 8
///     shared_notes UUID[],                      -- 9
///     additional_context TEXT,                  -- 10
///     constraints TEXT NOT NULL,                -- 11
///     deadline TIMESTAMPTZ,                     -- 12
///     status TEXT NOT NULL,                     -- 13
///     result JSONB,                             -- 14
///     created_at TIMESTAMPTZ NOT NULL,          -- 15
///     accepted_at TIMESTAMPTZ,                  -- 16
///     completed_at TIMESTAMPTZ,                 -- 17
///     tenant_id UUID,                           -- 18
///     version BIGINT NOT NULL,                  -- 19 (V3: CAS)
///     timeout_at TIMESTAMPTZ,                   -- 20 (V3: saga timeout)
///     last_progress_at TIMESTAMPTZ              -- 21 (V3: saga heartbeat)
/// );
/// ```
pub mod delegation {
    /// delegation_id UUID PRIMARY KEY
    pub const DELEGATION_ID: i16 = 1;
    /// delegator_agent_id UUID NOT NULL (FK)
    pub const DELEGATOR_AGENT_ID: i16 = 2;
    /// delegatee_agent_id UUID (FK)
    pub const DELEGATEE_AGENT_ID: i16 = 3;
    /// delegatee_agent_type TEXT
    pub const DELEGATEE_AGENT_TYPE: i16 = 4;
    /// task_description TEXT NOT NULL
    pub const TASK_DESCRIPTION: i16 = 5;
    /// parent_trajectory_id UUID NOT NULL (FK)
    pub const PARENT_TRAJECTORY_ID: i16 = 6;
    /// child_trajectory_id UUID (FK)
    pub const CHILD_TRAJECTORY_ID: i16 = 7;
    /// shared_artifacts UUID[]
    pub const SHARED_ARTIFACTS: i16 = 8;
    /// shared_notes UUID[]
    pub const SHARED_NOTES: i16 = 9;
    /// additional_context TEXT
    pub const ADDITIONAL_CONTEXT: i16 = 10;
    /// constraints TEXT NOT NULL
    pub const CONSTRAINTS: i16 = 11;
    /// deadline TIMESTAMPTZ
    pub const DEADLINE: i16 = 12;
    /// status TEXT NOT NULL
    pub const STATUS: i16 = 13;
    /// result JSONB
    pub const RESULT: i16 = 14;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 15;
    /// accepted_at TIMESTAMPTZ
    pub const ACCEPTED_AT: i16 = 16;
    /// completed_at TIMESTAMPTZ
    pub const COMPLETED_AT: i16 = 17;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 18;
    /// version BIGINT NOT NULL (V3: Compare-And-Swap version for optimistic locking)
    pub const VERSION: i16 = 19;
    /// timeout_at TIMESTAMPTZ (V3: explicit saga timeout deadline)
    pub const TIMEOUT_AT: i16 = 20;
    /// last_progress_at TIMESTAMPTZ (V3: saga heartbeat timestamp)
    pub const LAST_PROGRESS_AT: i16 = 21;

    /// Total number of columns in the delegation table
    pub const NUM_COLS: usize = 21;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_delegation";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_delegation_pkey";
    /// Delegator index name
    pub const DELEGATOR_INDEX: &str = "idx_delegation_delegator";
    /// Status index name
    pub const STATUS_INDEX: &str = "idx_delegation_status";
    /// Pending index name
    pub const PENDING_INDEX: &str = "idx_delegation_pending";
    /// Timeout index name (V3)
    pub const TIMEOUT_INDEX: &str = "idx_delegation_timeout";
    /// Progress index name (V3)
    pub const PROGRESS_INDEX: &str = "idx_delegation_progress";
}


// ============================================================================
// HANDOFF TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_handoff` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_handoff (
///     handoff_id UUID PRIMARY KEY,              -- 1
///     from_agent_id UUID NOT NULL,              -- 2
///     to_agent_id UUID,                         -- 3
///     to_agent_type TEXT,                       -- 4
///     trajectory_id UUID NOT NULL,              -- 5
///     scope_id UUID NOT NULL,                   -- 6
///     context_snapshot_id UUID NOT NULL,        -- 7
///     handoff_notes TEXT NOT NULL,              -- 8
///     next_steps TEXT[],                        -- 9
///     blockers TEXT[],                          -- 10
///     open_questions TEXT[],                    -- 11
///     status TEXT NOT NULL,                     -- 12
///     initiated_at TIMESTAMPTZ NOT NULL,        -- 13
///     accepted_at TIMESTAMPTZ,                  -- 14
///     completed_at TIMESTAMPTZ,                 -- 15
///     reason TEXT NOT NULL,                     -- 16
///     tenant_id UUID,                           -- 17
///     version BIGINT NOT NULL,                  -- 18 (V3: CAS)
///     timeout_at TIMESTAMPTZ,                   -- 19 (V3: saga timeout)
///     last_progress_at TIMESTAMPTZ              -- 20 (V3: saga heartbeat)
/// );
/// ```
pub mod handoff {
    /// handoff_id UUID PRIMARY KEY
    pub const HANDOFF_ID: i16 = 1;
    /// from_agent_id UUID NOT NULL (FK)
    pub const FROM_AGENT_ID: i16 = 2;
    /// to_agent_id UUID (FK)
    pub const TO_AGENT_ID: i16 = 3;
    /// to_agent_type TEXT
    pub const TO_AGENT_TYPE: i16 = 4;
    /// trajectory_id UUID NOT NULL (FK)
    pub const TRAJECTORY_ID: i16 = 5;
    /// scope_id UUID NOT NULL (FK)
    pub const SCOPE_ID: i16 = 6;
    /// context_snapshot_id UUID NOT NULL
    pub const CONTEXT_SNAPSHOT_ID: i16 = 7;
    /// handoff_notes TEXT NOT NULL
    pub const HANDOFF_NOTES: i16 = 8;
    /// next_steps TEXT[]
    pub const NEXT_STEPS: i16 = 9;
    /// blockers TEXT[]
    pub const BLOCKERS: i16 = 10;
    /// open_questions TEXT[]
    pub const OPEN_QUESTIONS: i16 = 11;
    /// status TEXT NOT NULL ('initiated', 'accepted', 'completed', 'rejected')
    pub const STATUS: i16 = 12;
    /// initiated_at TIMESTAMPTZ NOT NULL
    pub const INITIATED_AT: i16 = 13;
    /// accepted_at TIMESTAMPTZ
    pub const ACCEPTED_AT: i16 = 14;
    /// completed_at TIMESTAMPTZ
    pub const COMPLETED_AT: i16 = 15;
    /// reason TEXT NOT NULL
    pub const REASON: i16 = 16;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 17;
    /// version BIGINT NOT NULL (V3: Compare-And-Swap version for optimistic locking)
    pub const VERSION: i16 = 18;
    /// timeout_at TIMESTAMPTZ (V3: explicit saga timeout deadline)
    pub const TIMEOUT_AT: i16 = 19;
    /// last_progress_at TIMESTAMPTZ (V3: saga heartbeat timestamp)
    pub const LAST_PROGRESS_AT: i16 = 20;

    /// Total number of columns in the handoff table
    pub const NUM_COLS: usize = 20;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_handoff";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_handoff_pkey";
    /// From agent index name
    pub const FROM_AGENT_INDEX: &str = "idx_handoff_from";
    /// To agent index name
    pub const TO_AGENT_INDEX: &str = "idx_handoff_to";
    /// Status index name
    pub const STATUS_INDEX: &str = "idx_handoff_status";
    /// Timeout index name (V3)
    pub const TIMEOUT_INDEX: &str = "idx_handoff_timeout";
    /// Progress index name (V3)
    pub const PROGRESS_INDEX: &str = "idx_handoff_progress";
}

// ============================================================================
// CONFLICT TABLE COLUMNS
// ============================================================================

/// Column mappings for `caliber_conflict` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_conflict (
///     conflict_id UUID PRIMARY KEY,             -- 1
///     conflict_type TEXT NOT NULL,              -- 2
///     item_a_type TEXT NOT NULL,                -- 3
///     item_a_id UUID NOT NULL,                  -- 4
///     item_b_type TEXT NOT NULL,                -- 5
///     item_b_id UUID NOT NULL,                  -- 6
///     agent_a_id UUID,                          -- 7
///     agent_b_id UUID,                          -- 8
///     trajectory_id UUID,                       -- 9
///     status TEXT NOT NULL,                     -- 10
///     resolution JSONB,                         -- 11
///     detected_at TIMESTAMPTZ NOT NULL,         -- 12
///     resolved_at TIMESTAMPTZ,                  -- 13
///     tenant_id UUID                            -- 14
/// );
/// ```
pub mod conflict {
    /// conflict_id UUID PRIMARY KEY
    pub const CONFLICT_ID: i16 = 1;
    /// conflict_type TEXT NOT NULL
    pub const CONFLICT_TYPE: i16 = 2;
    /// item_a_type TEXT NOT NULL
    pub const ITEM_A_TYPE: i16 = 3;
    /// item_a_id UUID NOT NULL
    pub const ITEM_A_ID: i16 = 4;
    /// item_b_type TEXT NOT NULL
    pub const ITEM_B_TYPE: i16 = 5;
    /// item_b_id UUID NOT NULL
    pub const ITEM_B_ID: i16 = 6;
    /// agent_a_id UUID (FK)
    pub const AGENT_A_ID: i16 = 7;
    /// agent_b_id UUID (FK)
    pub const AGENT_B_ID: i16 = 8;
    /// trajectory_id UUID (FK)
    pub const TRAJECTORY_ID: i16 = 9;
    /// status TEXT NOT NULL ('detected', 'resolving', 'resolved', 'escalated')
    pub const STATUS: i16 = 10;
    /// resolution JSONB
    pub const RESOLUTION: i16 = 11;
    /// detected_at TIMESTAMPTZ NOT NULL
    pub const DETECTED_AT: i16 = 12;
    /// resolved_at TIMESTAMPTZ
    pub const RESOLVED_AT: i16 = 13;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 14;

    /// Total number of columns in the conflict table
    pub const NUM_COLS: usize = 14;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_conflict";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_conflict_pkey";
    /// Status index name
    pub const STATUS_INDEX: &str = "idx_conflict_status";
    /// Items index name
    pub const ITEMS_INDEX: &str = "idx_conflict_items";
}

// ============================================================================
// REGION TABLE COLUMNS (for completeness)
// ============================================================================

/// Column mappings for `caliber_region` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_region (
///     region_id UUID PRIMARY KEY,               -- 1
///     region_type TEXT NOT NULL,                -- 2
///     owner_agent_id UUID NOT NULL,             -- 3
///     team_id UUID,                             -- 4
///     readers UUID[],                           -- 5
///     writers UUID[],                           -- 6
///     require_lock BOOLEAN NOT NULL,            -- 7
///     conflict_resolution TEXT NOT NULL,        -- 8
///     version_tracking BOOLEAN NOT NULL,        -- 9
///     created_at TIMESTAMPTZ NOT NULL,          -- 10
///     updated_at TIMESTAMPTZ NOT NULL,          -- 11
///     tenant_id UUID                            -- 12
/// );
/// ```
pub mod region {
    /// region_id UUID PRIMARY KEY
    pub const REGION_ID: i16 = 1;
    /// region_type TEXT NOT NULL ('private', 'team', 'public', 'collaborative')
    pub const REGION_TYPE: i16 = 2;
    /// owner_agent_id UUID NOT NULL (FK)
    pub const OWNER_AGENT_ID: i16 = 3;
    /// team_id UUID
    pub const TEAM_ID: i16 = 4;
    /// readers UUID[]
    pub const READERS: i16 = 5;
    /// writers UUID[]
    pub const WRITERS: i16 = 6;
    /// require_lock BOOLEAN NOT NULL
    pub const REQUIRE_LOCK: i16 = 7;
    /// conflict_resolution TEXT NOT NULL
    pub const CONFLICT_RESOLUTION: i16 = 8;
    /// version_tracking BOOLEAN NOT NULL
    pub const VERSION_TRACKING: i16 = 9;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 10;
    /// updated_at TIMESTAMPTZ NOT NULL
    pub const UPDATED_AT: i16 = 11;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 12;

    /// Total number of columns in the region table
    pub const NUM_COLS: usize = 12;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_region";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_region_pkey";
    /// Owner index name
    pub const OWNER_INDEX: &str = "idx_region_owner";
    /// Type index name
    pub const TYPE_INDEX: &str = "idx_region_type";
}

// ============================================================================
// EDGE TABLE COLUMNS (Battle Intel Feature 1)
// ============================================================================

/// Column mappings for `caliber_edge` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_edge (
///     edge_id UUID PRIMARY KEY,                 -- 1
///     edge_type TEXT NOT NULL,                  -- 2
///     participants JSONB NOT NULL,              -- 3
///     weight REAL,                              -- 4
///     trajectory_id UUID,                       -- 5
///     source_turn INTEGER NOT NULL,             -- 6
///     extraction_method TEXT NOT NULL,          -- 7
///     confidence REAL,                          -- 8
///     created_at TIMESTAMPTZ NOT NULL,          -- 9
///     metadata JSONB,                           -- 10
///     tenant_id UUID                            -- 11
/// );
/// ```
pub mod edge {
    /// edge_id UUID PRIMARY KEY
    pub const EDGE_ID: i16 = 1;
    /// edge_type TEXT NOT NULL
    pub const EDGE_TYPE: i16 = 2;
    /// participants JSONB NOT NULL (array of {entity_type, id, role})
    pub const PARTICIPANTS: i16 = 3;
    /// weight REAL (relationship strength [0.0, 1.0])
    pub const WEIGHT: i16 = 4;
    /// trajectory_id UUID (FK, optional)
    pub const TRAJECTORY_ID: i16 = 5;
    /// source_turn INTEGER NOT NULL (provenance)
    pub const SOURCE_TURN: i16 = 6;
    /// extraction_method TEXT NOT NULL (provenance)
    pub const EXTRACTION_METHOD: i16 = 7;
    /// confidence REAL (provenance)
    pub const CONFIDENCE: i16 = 8;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 9;
    /// metadata JSONB
    pub const METADATA: i16 = 10;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 11;

    /// Total number of columns in the edge table
    pub const NUM_COLS: usize = 11;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_edge";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_edge_pkey";
    /// Type index name
    pub const TYPE_INDEX: &str = "idx_edge_type";
    /// Participants index name (GIN)
    pub const PARTICIPANTS_INDEX: &str = "idx_edge_participants";
    /// Trajectory index name
    pub const TRAJECTORY_INDEX: &str = "idx_edge_trajectory";
}

// ============================================================================
// EVOLUTION SNAPSHOT TABLE COLUMNS (Battle Intel Feature 3)
// ============================================================================

/// Column mappings for `caliber_evolution_snapshot` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_evolution_snapshot (
///     snapshot_id UUID PRIMARY KEY,             -- 1
///     name TEXT NOT NULL UNIQUE,                -- 2
///     config_hash BYTEA NOT NULL,               -- 3
///     config_source TEXT NOT NULL,              -- 4
///     phase TEXT NOT NULL,                      -- 5
///     created_at TIMESTAMPTZ NOT NULL,          -- 6
///     retrieval_accuracy REAL,                  -- 7
///     token_efficiency REAL,                    -- 8
///     latency_p50_ms BIGINT,                    -- 9
///     latency_p99_ms BIGINT,                    -- 10
///     cost_estimate REAL,                       -- 11
///     benchmark_queries INTEGER,                -- 12
///     metadata JSONB                            -- 13
/// );
/// ```
pub mod evolution_snapshot {
    /// snapshot_id UUID PRIMARY KEY
    pub const SNAPSHOT_ID: i16 = 1;
    /// name TEXT NOT NULL UNIQUE
    pub const NAME: i16 = 2;
    /// config_hash BYTEA NOT NULL
    pub const CONFIG_HASH: i16 = 3;
    /// config_source TEXT NOT NULL
    pub const CONFIG_SOURCE: i16 = 4;
    /// phase TEXT NOT NULL ('online', 'frozen', 'evolving')
    pub const PHASE: i16 = 5;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 6;
    /// retrieval_accuracy REAL (metrics)
    pub const RETRIEVAL_ACCURACY: i16 = 7;
    /// token_efficiency REAL (metrics)
    pub const TOKEN_EFFICIENCY: i16 = 8;
    /// latency_p50_ms BIGINT (metrics)
    pub const LATENCY_P50_MS: i16 = 9;
    /// latency_p99_ms BIGINT (metrics)
    pub const LATENCY_P99_MS: i16 = 10;
    /// cost_estimate REAL (metrics)
    pub const COST_ESTIMATE: i16 = 11;
    /// benchmark_queries INTEGER (metrics)
    pub const BENCHMARK_QUERIES: i16 = 12;
    /// metadata JSONB
    pub const METADATA: i16 = 13;

    /// Total number of columns in the evolution_snapshot table
    pub const NUM_COLS: usize = 13;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_evolution_snapshot";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_evolution_snapshot_pkey";
    /// Phase index name
    pub const PHASE_INDEX: &str = "idx_evolution_phase";
}

// ============================================================================
// SUMMARIZATION POLICY TABLE COLUMNS (Battle Intel Feature 4)
// ============================================================================

/// Column mappings for `caliber_summarization_policy` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_summarization_policy (
///     policy_id UUID PRIMARY KEY,               -- 1
///     name TEXT NOT NULL UNIQUE,                -- 2
///     triggers JSONB NOT NULL,                  -- 3
///     target_level TEXT NOT NULL,               -- 4
///     source_level TEXT NOT NULL,               -- 5
///     max_sources INTEGER NOT NULL,             -- 6
///     create_edges BOOLEAN NOT NULL,            -- 7
///     created_at TIMESTAMPTZ NOT NULL,          -- 8
///     metadata JSONB,                           -- 9
///     tenant_id UUID                            -- 10
/// );
/// ```
pub mod summarization_policy {
    /// policy_id UUID PRIMARY KEY
    pub const POLICY_ID: i16 = 1;
    /// name TEXT NOT NULL UNIQUE
    pub const NAME: i16 = 2;
    /// triggers JSONB NOT NULL (array of trigger definitions)
    pub const TRIGGERS: i16 = 3;
    /// target_level TEXT NOT NULL ('raw', 'summary', 'principle')
    pub const TARGET_LEVEL: i16 = 4;
    /// source_level TEXT NOT NULL ('raw', 'summary', 'principle')
    pub const SOURCE_LEVEL: i16 = 5;
    /// max_sources INTEGER NOT NULL
    pub const MAX_SOURCES: i16 = 6;
    /// create_edges BOOLEAN NOT NULL
    pub const CREATE_EDGES: i16 = 7;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 8;
    /// metadata JSONB
    pub const METADATA: i16 = 9;
    /// tenant_id UUID (FK)
    pub const TENANT_ID: i16 = 10;

    /// Total number of columns in the summarization_policy table
    pub const NUM_COLS: usize = 10;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_summarization_policy";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_summarization_policy_pkey";
    /// Target level index name
    pub const TARGET_INDEX: &str = "idx_summarization_target";
    /// Source level index name
    pub const SOURCE_INDEX: &str = "idx_summarization_source";
}

// ============================================================================
// CHANGE JOURNAL TABLE COLUMNS (Cache Invalidation)
// ============================================================================

/// Column mappings for `caliber_changes` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_changes (
///     change_id BIGSERIAL PRIMARY KEY,          -- 1
///     tenant_id UUID NOT NULL,                  -- 2
///     entity_type TEXT NOT NULL,                -- 3
///     entity_id UUID NOT NULL,                  -- 4
///     operation TEXT NOT NULL,                  -- 5
///     changed_at TIMESTAMPTZ NOT NULL           -- 6
/// );
/// ```
pub mod changes {
    /// change_id BIGSERIAL PRIMARY KEY
    pub const CHANGE_ID: i16 = 1;
    /// tenant_id UUID NOT NULL
    pub const TENANT_ID: i16 = 2;
    /// entity_type TEXT NOT NULL
    pub const ENTITY_TYPE: i16 = 3;
    /// entity_id UUID NOT NULL
    pub const ENTITY_ID: i16 = 4;
    /// operation TEXT NOT NULL ('INSERT', 'UPDATE', 'DELETE')
    pub const OPERATION: i16 = 5;
    /// changed_at TIMESTAMPTZ NOT NULL
    pub const CHANGED_AT: i16 = 6;

    /// Total number of columns in the changes table
    pub const NUM_COLS: usize = 6;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_changes";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_changes_pkey";
    /// Tenant+sequence index name
    pub const TENANT_SEQ_INDEX: &str = "idx_changes_tenant_seq";
    /// Entity index name
    pub const ENTITY_INDEX: &str = "idx_changes_entity";
    /// Time index name
    pub const TIME_INDEX: &str = "idx_changes_time";
}

// ============================================================================
// IDEMPOTENCY KEY TABLE COLUMNS (V3: Distributed Correctness)
// ============================================================================

/// Column mappings for `caliber_idempotency_key` table.
///
/// Schema:
/// ```sql
/// CREATE TABLE caliber_idempotency_key (
///     idempotency_key TEXT PRIMARY KEY,          -- 1
///     tenant_id UUID NOT NULL,                   -- 2
///     operation TEXT NOT NULL,                   -- 3
///     request_hash BYTEA NOT NULL,               -- 4
///     response_status INTEGER NOT NULL,          -- 5
///     response_body JSONB,                       -- 6
///     created_at TIMESTAMPTZ NOT NULL,           -- 7
///     expires_at TIMESTAMPTZ NOT NULL            -- 8
/// );
/// ```
pub mod idempotency_key {
    /// idempotency_key TEXT PRIMARY KEY
    pub const IDEMPOTENCY_KEY: i16 = 1;
    /// tenant_id UUID NOT NULL
    pub const TENANT_ID: i16 = 2;
    /// operation TEXT NOT NULL
    pub const OPERATION: i16 = 3;
    /// request_hash BYTEA NOT NULL
    pub const REQUEST_HASH: i16 = 4;
    /// response_status INTEGER NOT NULL
    pub const RESPONSE_STATUS: i16 = 5;
    /// response_body JSONB
    pub const RESPONSE_BODY: i16 = 6;
    /// created_at TIMESTAMPTZ NOT NULL
    pub const CREATED_AT: i16 = 7;
    /// expires_at TIMESTAMPTZ NOT NULL
    pub const EXPIRES_AT: i16 = 8;

    /// Total number of columns in the idempotency_key table
    pub const NUM_COLS: usize = 8;

    /// Table name
    pub const TABLE_NAME: &str = "caliber_idempotency_key";
    /// Primary key index name
    pub const PK_INDEX: &str = "caliber_idempotency_key_pkey";
    /// Expires index name
    pub const EXPIRES_INDEX: &str = "idx_idempotency_expires";
    /// Tenant+operation index name
    pub const TENANT_INDEX: &str = "idx_idempotency_tenant";
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trajectory_column_count() {
        assert_eq!(trajectory::NUM_COLS, 13);
    }

    #[test]
    fn test_scope_column_count() {
        assert_eq!(scope::NUM_COLS, 13);
    }

    #[test]
    fn test_artifact_column_count() {
        assert_eq!(artifact::NUM_COLS, 15);
    }

    #[test]
    fn test_note_column_count() {
        assert_eq!(note::NUM_COLS, 18); // Updated for Battle Intel Feature 2
    }

    #[test]
    fn test_turn_column_count() {
        assert_eq!(turn::NUM_COLS, 11);
    }

    #[test]
    fn test_agent_column_count() {
        assert_eq!(agent::NUM_COLS, 12);
    }

    #[test]
    fn test_lock_column_count() {
        assert_eq!(lock::NUM_COLS, 9); // Updated for V3: +version
    }

    #[test]
    fn test_message_column_count() {
        assert_eq!(message::NUM_COLS, 15);
    }

    #[test]
    fn test_delegation_column_count() {
        assert_eq!(delegation::NUM_COLS, 21); // Updated for V3: +version, timeout_at, last_progress_at
    }

    #[test]
    fn test_handoff_column_count() {
        assert_eq!(handoff::NUM_COLS, 20); // Updated for V3: +version, timeout_at, last_progress_at
    }

    #[test]
    fn test_conflict_column_count() {
        assert_eq!(conflict::NUM_COLS, 14);
    }

    #[test]
    fn test_region_column_count() {
        assert_eq!(region::NUM_COLS, 12);
    }

    // Battle Intel Feature 1: Graph Edges
    #[test]
    fn test_edge_column_count() {
        assert_eq!(edge::NUM_COLS, 11);
    }

    // Battle Intel Feature 3: Evolution Snapshots
    #[test]
    fn test_evolution_snapshot_column_count() {
        assert_eq!(evolution_snapshot::NUM_COLS, 13);
    }

    // Battle Intel Feature 4: Summarization Policies
    #[test]
    fn test_summarization_policy_column_count() {
        assert_eq!(summarization_policy::NUM_COLS, 10);
    }

    // Cache Invalidation: Change Journal
    #[test]
    fn test_changes_column_count() {
        assert_eq!(changes::NUM_COLS, 6);
    }

    // V3 Distributed Correctness: Idempotency Keys
    #[test]
    fn test_idempotency_key_column_count() {
        assert_eq!(idempotency_key::NUM_COLS, 8);
    }
}