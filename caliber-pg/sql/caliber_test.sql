-- CALIBER Regression Tests
-- Tests core CRUD operations, constraints, triggers, and indexes
-- Run with: cargo pgrx regress pg18

-- ============================================================================
-- SETUP: Load extension and schema
-- ============================================================================
CREATE EXTENSION IF NOT EXISTS caliber_pg CASCADE;

-- Verify extension loaded
SELECT extname, extversion FROM pg_extension WHERE extname = 'caliber_pg';

-- ============================================================================
-- PROPERTY 1: Trajectory CRUD (Insert-Get Round Trip)
-- Validates: Requirements 1.1, 1.2
-- ============================================================================

-- Test: Create trajectory
INSERT INTO caliber_trajectory (trajectory_id, name, description, status)
VALUES ('11111111-1111-1111-1111-111111111111', 'Test Trajectory', 'A test description', 'active')
RETURNING trajectory_id, name, status;

-- Test: Get trajectory
SELECT trajectory_id, name, description, status
FROM caliber_trajectory
WHERE trajectory_id = '11111111-1111-1111-1111-111111111111';

-- Test: Status constraint (should succeed)
UPDATE caliber_trajectory
SET status = 'completed'
WHERE trajectory_id = '11111111-1111-1111-1111-111111111111'
RETURNING trajectory_id, status;

-- Test: Invalid status (should fail)
\set ON_ERROR_STOP off
INSERT INTO caliber_trajectory (trajectory_id, name, status)
VALUES ('11111111-1111-1111-1111-111111111112', 'Bad Status', 'invalid_status');
\set ON_ERROR_STOP on

-- Test: Parent-child relationship
INSERT INTO caliber_trajectory (trajectory_id, name, status, parent_trajectory_id)
VALUES ('22222222-2222-2222-2222-222222222222', 'Child Trajectory', 'active', '11111111-1111-1111-1111-111111111111')
RETURNING trajectory_id, parent_trajectory_id;

-- ============================================================================
-- PROPERTY 1: Scope CRUD
-- Validates: Requirements 2.1, 2.2
-- ============================================================================

-- Test: Create scope
INSERT INTO caliber_scope (scope_id, trajectory_id, name, purpose, token_budget)
VALUES ('33333333-3333-3333-3333-333333333333', '11111111-1111-1111-1111-111111111111', 'Test Scope', 'Testing purposes', 4000)
RETURNING scope_id, trajectory_id, name, is_active, token_budget, tokens_used;

-- Test: Get scope
SELECT scope_id, name, is_active, token_budget, tokens_used
FROM caliber_scope
WHERE scope_id = '33333333-3333-3333-3333-333333333333';

-- Test: Close scope (update)
UPDATE caliber_scope
SET is_active = FALSE, closed_at = NOW()
WHERE scope_id = '33333333-3333-3333-3333-333333333333'
RETURNING scope_id, is_active, closed_at IS NOT NULL AS has_closed_at;

-- Test: Nested scope
INSERT INTO caliber_scope (scope_id, trajectory_id, parent_scope_id, name, token_budget)
VALUES ('44444444-4444-4444-4444-444444444444', '11111111-1111-1111-1111-111111111111', '33333333-3333-3333-3333-333333333333', 'Child Scope', 2000)
RETURNING scope_id, parent_scope_id;

-- ============================================================================
-- PROPERTY 1: Turn CRUD
-- Validates: Requirements 5.1, 5.2
-- ============================================================================

-- Reactivate scope for turn tests
UPDATE caliber_scope SET is_active = TRUE WHERE scope_id = '33333333-3333-3333-3333-333333333333';

-- Test: Create turns in sequence
INSERT INTO caliber_turn (turn_id, scope_id, sequence, role, content, token_count)
VALUES
    ('55555555-5555-5555-5555-555555555551', '33333333-3333-3333-3333-333333333333', 1, 'user', 'Hello, assistant.', 10),
    ('55555555-5555-5555-5555-555555555552', '33333333-3333-3333-3333-333333333333', 2, 'assistant', 'Hello! How can I help?', 15),
    ('55555555-5555-5555-5555-555555555553', '33333333-3333-3333-3333-333333333333', 3, 'user', 'Run a tool please.', 12)
RETURNING turn_id, sequence, role, token_count;

-- Test: Get turns in order
SELECT turn_id, sequence, role, content
FROM caliber_turn
WHERE scope_id = '33333333-3333-3333-3333-333333333333'
ORDER BY sequence;

-- Test: Role constraint (should fail)
\set ON_ERROR_STOP off
INSERT INTO caliber_turn (turn_id, scope_id, sequence, role, content, token_count)
VALUES ('55555555-5555-5555-5555-555555555554', '33333333-3333-3333-3333-333333333333', 4, 'invalid_role', 'Bad role', 5);
\set ON_ERROR_STOP on

-- Test: Sequence uniqueness (should fail)
\set ON_ERROR_STOP off
INSERT INTO caliber_turn (turn_id, scope_id, sequence, role, content, token_count)
VALUES ('55555555-5555-5555-5555-555555555555', '33333333-3333-3333-3333-333333333333', 1, 'user', 'Duplicate sequence', 5);
\set ON_ERROR_STOP on

-- ============================================================================
-- PROPERTY 1: Artifact CRUD
-- Validates: Requirements 3.1, 3.2
-- ============================================================================

-- Test: Create artifact
INSERT INTO caliber_artifact (artifact_id, trajectory_id, scope_id, artifact_type, name, content, content_hash, provenance)
VALUES (
    '66666666-6666-6666-6666-666666666666',
    '11111111-1111-1111-1111-111111111111',
    '33333333-3333-3333-3333-333333333333',
    'fact',
    'Test Artifact',
    'This is important content.',
    E'\\xDEADBEEF',
    '{"source_turn": 1, "extraction_method": "explicit", "confidence": 0.95}'
)
RETURNING artifact_id, artifact_type, name, ttl;

-- Test: Get artifact
SELECT artifact_id, artifact_type, name, content, ttl
FROM caliber_artifact
WHERE artifact_id = '66666666-6666-6666-6666-666666666666';

-- ============================================================================
-- PROPERTY 1: Note CRUD
-- Validates: Requirements 4.1, 4.2
-- ============================================================================

-- Test: Create note
INSERT INTO caliber_note (note_id, note_type, title, content, content_hash, abstraction_level)
VALUES (
    '77777777-7777-7777-7777-777777777777',
    'convention',
    'Code Style',
    'Use 4-space indentation.',
    E'\\xCAFEBABE',
    'raw'
)
RETURNING note_id, note_type, title, abstraction_level, access_count;

-- Test: Get note
SELECT note_id, note_type, title, content, access_count
FROM caliber_note
WHERE note_id = '77777777-7777-7777-7777-777777777777';

-- Test: Update access count
UPDATE caliber_note
SET access_count = access_count + 1, accessed_at = NOW()
WHERE note_id = '77777777-7777-7777-7777-777777777777'
RETURNING note_id, access_count;

-- Test: Abstraction level constraint (should fail)
\set ON_ERROR_STOP off
INSERT INTO caliber_note (note_id, note_type, title, content, content_hash, abstraction_level)
VALUES ('77777777-7777-7777-7777-777777777778', 'fact', 'Bad Level', 'Content', E'\\x00', 'invalid_level');
\set ON_ERROR_STOP on

-- ============================================================================
-- PROPERTY 1: Agent CRUD
-- Validates: Requirements 6.1, 6.2, 6.3
-- ============================================================================

-- Test: Register agent
INSERT INTO caliber_agent (agent_id, agent_type, capabilities, status)
VALUES (
    '88888888-8888-8888-8888-888888888888',
    'coordinator',
    ARRAY['planning', 'delegation'],
    'idle'
)
RETURNING agent_id, agent_type, status;

-- Test: Get agent
SELECT agent_id, agent_type, capabilities, status
FROM caliber_agent
WHERE agent_id = '88888888-8888-8888-8888-888888888888';

-- Test: Update agent status
UPDATE caliber_agent
SET status = 'active',
    current_trajectory_id = '11111111-1111-1111-1111-111111111111',
    current_scope_id = '33333333-3333-3333-3333-333333333333'
WHERE agent_id = '88888888-8888-8888-8888-888888888888'
RETURNING agent_id, status, current_trajectory_id IS NOT NULL AS has_trajectory;

-- Test: Agent heartbeat
UPDATE caliber_agent
SET last_heartbeat = NOW()
WHERE agent_id = '88888888-8888-8888-8888-888888888888'
RETURNING agent_id, last_heartbeat > NOW() - INTERVAL '1 second' AS heartbeat_recent;

-- Test: Status constraint (should fail)
\set ON_ERROR_STOP off
INSERT INTO caliber_agent (agent_id, agent_type, status)
VALUES ('88888888-8888-8888-8888-888888888889', 'worker', 'invalid_status');
\set ON_ERROR_STOP on

-- ============================================================================
-- PROPERTY 1: Lock CRUD
-- Validates: Requirements 7.1, 7.2
-- ============================================================================

-- Test: Acquire lock
INSERT INTO caliber_lock (lock_id, resource_type, resource_id, holder_agent_id, expires_at, mode)
VALUES (
    '99999999-9999-9999-9999-999999999999',
    'trajectory',
    '11111111-1111-1111-1111-111111111111',
    '88888888-8888-8888-8888-888888888888',
    NOW() + INTERVAL '5 minutes',
    'exclusive'
)
RETURNING lock_id, resource_type, resource_id, mode;

-- Test: Get lock
SELECT lock_id, resource_type, resource_id, holder_agent_id, mode,
       expires_at > NOW() AS is_valid
FROM caliber_lock
WHERE lock_id = '99999999-9999-9999-9999-999999999999';

-- Test: Lock mode constraint (should fail)
\set ON_ERROR_STOP off
INSERT INTO caliber_lock (lock_id, resource_type, resource_id, holder_agent_id, expires_at, mode)
VALUES ('99999999-9999-9999-9999-999999999990', 'scope', '33333333-3333-3333-3333-333333333333', '88888888-8888-8888-8888-888888888888', NOW() + INTERVAL '1 minute', 'invalid_mode');
\set ON_ERROR_STOP on

-- ============================================================================
-- PROPERTY 1: Message CRUD
-- Validates: Requirements 8.1, 8.2
-- ============================================================================

-- Create second agent for messaging
INSERT INTO caliber_agent (agent_id, agent_type, status)
VALUES ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'worker', 'idle');

-- Test: Send message
INSERT INTO caliber_message (message_id, from_agent_id, to_agent_id, message_type, payload, priority)
VALUES (
    'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
    '88888888-8888-8888-8888-888888888888',
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    'task_request',
    '{"action": "process_data"}',
    'high'
)
RETURNING message_id, message_type, priority, delivered_at IS NULL AS is_pending;

-- Test: Get pending messages
SELECT message_id, from_agent_id, message_type, priority
FROM caliber_message
WHERE to_agent_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
  AND delivered_at IS NULL;

-- Test: Deliver message
UPDATE caliber_message
SET delivered_at = NOW()
WHERE message_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
RETURNING message_id, delivered_at IS NOT NULL AS is_delivered;

-- Test: Acknowledge message
UPDATE caliber_message
SET acknowledged_at = NOW()
WHERE message_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
RETURNING message_id, acknowledged_at IS NOT NULL AS is_acknowledged;

-- ============================================================================
-- PROPERTY 1: Delegation CRUD
-- Validates: Requirements 9.1, 9.2
-- ============================================================================

-- Test: Create delegation
INSERT INTO caliber_delegation (delegation_id, delegator_agent_id, delegatee_agent_id, task_description, parent_trajectory_id, status)
VALUES (
    'cccccccc-cccc-cccc-cccc-cccccccccccc',
    '88888888-8888-8888-8888-888888888888',
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    'Process the data files',
    '11111111-1111-1111-1111-111111111111',
    'pending'
)
RETURNING delegation_id, status;

-- Test: Accept delegation
UPDATE caliber_delegation
SET status = 'accepted', accepted_at = NOW()
WHERE delegation_id = 'cccccccc-cccc-cccc-cccc-cccccccccccc'
RETURNING delegation_id, status, accepted_at IS NOT NULL AS was_accepted;

-- Test: Complete delegation
UPDATE caliber_delegation
SET status = 'completed', completed_at = NOW(), result = '{"success": true}'
WHERE delegation_id = 'cccccccc-cccc-cccc-cccc-cccccccccccc'
RETURNING delegation_id, status, result;

-- ============================================================================
-- PROPERTY 1: Conflict CRUD
-- Validates: Requirements 11.1, 11.2
-- ============================================================================

-- Create second artifact for conflict
INSERT INTO caliber_artifact (artifact_id, trajectory_id, scope_id, artifact_type, name, content, content_hash, provenance)
VALUES (
    '66666666-6666-6666-6666-666666666667',
    '11111111-1111-1111-1111-111111111111',
    '33333333-3333-3333-3333-333333333333',
    'fact',
    'Conflicting Artifact',
    'Different content.',
    E'\\xFEEDFACE',
    '{"source_turn": 2, "extraction_method": "inferred", "confidence": 0.8}'
);

-- Test: Detect conflict
INSERT INTO caliber_conflict (conflict_id, conflict_type, item_a_type, item_a_id, item_b_type, item_b_id, trajectory_id, status)
VALUES (
    'dddddddd-dddd-dddd-dddd-dddddddddddd',
    'contradiction',
    'artifact',
    '66666666-6666-6666-6666-666666666666',
    'artifact',
    '66666666-6666-6666-6666-666666666667',
    '11111111-1111-1111-1111-111111111111',
    'detected'
)
RETURNING conflict_id, conflict_type, status;

-- Test: Resolve conflict
UPDATE caliber_conflict
SET status = 'resolved', resolved_at = NOW(), resolution = '{"winner": "item_a", "reason": "higher confidence"}'
WHERE conflict_id = 'dddddddd-dddd-dddd-dddd-dddddddddddd'
RETURNING conflict_id, status, resolution;

-- ============================================================================
-- PROPERTY 1: Handoff CRUD
-- Validates: Requirements 10.1, 10.2
-- ============================================================================

-- Test: Create handoff
INSERT INTO caliber_handoff (handoff_id, from_agent_id, to_agent_id, trajectory_id, scope_id, context_snapshot_id, reason, status)
VALUES (
    'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee',
    '88888888-8888-8888-8888-888888888888',
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    '11111111-1111-1111-1111-111111111111',
    '33333333-3333-3333-3333-333333333333',
    'ffffffff-ffff-ffff-ffff-ffffffffffff',
    'shift_change',
    'initiated'
)
RETURNING handoff_id, reason, status;

-- Test: Accept handoff
UPDATE caliber_handoff
SET status = 'accepted', accepted_at = NOW()
WHERE handoff_id = 'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee'
RETURNING handoff_id, status;

-- Test: Complete handoff
UPDATE caliber_handoff
SET status = 'completed', completed_at = NOW()
WHERE handoff_id = 'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee'
RETURNING handoff_id, status;

-- ============================================================================
-- PROPERTY 1: Edge CRUD (Battle Intel Feature 1)
-- Validates: Graph relationships
-- ============================================================================

-- Test: Create edge between artifacts
INSERT INTO caliber_edge (edge_id, edge_type, participants, weight, trajectory_id, source_turn, extraction_method, confidence)
VALUES (
    '12121212-1212-1212-1212-121212121212',
    'contradicts',
    '[{"entity_type": "artifact", "id": "66666666-6666-6666-6666-666666666666", "role": "source"}, {"entity_type": "artifact", "id": "66666666-6666-6666-6666-666666666667", "role": "target"}]'::jsonb,
    0.85,
    '11111111-1111-1111-1111-111111111111',
    2,
    'inferred',
    0.9
)
RETURNING edge_id, edge_type, weight;

-- Test: Query edges by type
SELECT edge_id, edge_type, weight, participants
FROM caliber_edge
WHERE edge_type = 'contradicts';

-- ============================================================================
-- PROPERTY 2: Update Persistence (Trigger Tests)
-- Validates: Requirements 15.1
-- ============================================================================

-- Test: Trajectory updated_at trigger
SELECT updated_at AS before_update FROM caliber_trajectory WHERE trajectory_id = '11111111-1111-1111-1111-111111111111';

-- Small delay to ensure timestamp difference
SELECT pg_sleep(0.01);

UPDATE caliber_trajectory
SET description = 'Updated description'
WHERE trajectory_id = '11111111-1111-1111-1111-111111111111';

SELECT updated_at AS after_update,
       updated_at > created_at AS updated_after_created
FROM caliber_trajectory
WHERE trajectory_id = '11111111-1111-1111-1111-111111111111';

-- ============================================================================
-- PROPERTY 3: Index Consistency
-- Validates: Requirements 13.1, 13.2
-- ============================================================================

-- Test: Index on trajectory status
EXPLAIN (COSTS OFF) SELECT * FROM caliber_trajectory WHERE status = 'active';

-- Test: Index on scope trajectory_id
EXPLAIN (COSTS OFF) SELECT * FROM caliber_scope WHERE trajectory_id = '11111111-1111-1111-1111-111111111111';

-- Test: Index on turn scope_id + sequence
EXPLAIN (COSTS OFF) SELECT * FROM caliber_turn WHERE scope_id = '33333333-3333-3333-3333-333333333333' ORDER BY sequence;

-- Test: Index on agent status
EXPLAIN (COSTS OFF) SELECT * FROM caliber_agent WHERE status = 'active';

-- Test: Index on message pending
EXPLAIN (COSTS OFF) SELECT * FROM caliber_message WHERE to_agent_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa' AND delivered_at IS NULL;

-- ============================================================================
-- PROPERTY 8: Not Found Returns Empty (Not Error)
-- Validates: Requirement 14.2
-- ============================================================================

-- Test: Get non-existent trajectory (should return 0 rows, not error)
SELECT COUNT(*) AS found_count
FROM caliber_trajectory
WHERE trajectory_id = 'ffffffff-ffff-ffff-ffff-000000000000';

-- Test: Get non-existent agent
SELECT COUNT(*) AS found_count
FROM caliber_agent
WHERE agent_id = 'ffffffff-ffff-ffff-ffff-000000000001';

-- ============================================================================
-- CLEANUP FUNCTION TESTS
-- Validates: Lock and message expiry
-- ============================================================================

-- Create an expired lock for testing
INSERT INTO caliber_lock (lock_id, resource_type, resource_id, holder_agent_id, expires_at, mode)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'artifact',
    '66666666-6666-6666-6666-666666666666',
    '88888888-8888-8888-8888-888888888888',
    NOW() - INTERVAL '1 hour',  -- Already expired
    'shared'
);

-- Test: Cleanup expired locks
SELECT caliber_cleanup_expired_locks() AS locks_deleted;

-- Verify lock was deleted
SELECT COUNT(*) AS remaining_expired_locks
FROM caliber_lock
WHERE lock_id = '00000000-0000-0000-0000-000000000001';

-- ============================================================================
-- VIEW TESTS
-- Validates: Debug views work correctly
-- ============================================================================

-- Test: Active trajectories view
SELECT trajectory_id, name, status, scope_count
FROM caliber_v_active_trajectories
LIMIT 5;

-- Test: Agent status view
SELECT agent_id, agent_type, status, held_locks, pending_messages
FROM caliber_v_agent_status
LIMIT 5;

-- ============================================================================
-- CLEANUP
-- ============================================================================

-- Delete in reverse dependency order
DELETE FROM caliber_edge;
DELETE FROM caliber_handoff;
DELETE FROM caliber_conflict;
DELETE FROM caliber_delegation;
DELETE FROM caliber_message;
DELETE FROM caliber_lock;
DELETE FROM caliber_turn;
DELETE FROM caliber_artifact;
DELETE FROM caliber_note;
DELETE FROM caliber_scope;
DELETE FROM caliber_trajectory WHERE parent_trajectory_id IS NOT NULL;
DELETE FROM caliber_trajectory;
DELETE FROM caliber_agent;

-- Verify cleanup
SELECT
    (SELECT COUNT(*) FROM caliber_trajectory) AS trajectories,
    (SELECT COUNT(*) FROM caliber_scope) AS scopes,
    (SELECT COUNT(*) FROM caliber_artifact) AS artifacts,
    (SELECT COUNT(*) FROM caliber_note) AS notes,
    (SELECT COUNT(*) FROM caliber_agent) AS agents;
