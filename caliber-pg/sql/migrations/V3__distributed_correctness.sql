-- ============================================================================
-- CALIBER DISTRIBUTED CORRECTNESS MIGRATION
-- Version: 3
-- Description: Adds version columns for CAS, timeout tracking, unique
--              constraints for saga patterns, and idempotency key table
-- ============================================================================

BEGIN;

-- ============================================================================
-- PHASE 1: VERSION COLUMNS FOR COMPARE-AND-SWAP (CAS)
-- ============================================================================

-- Lock table: Add version for optimistic concurrency control
ALTER TABLE caliber_lock ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;

-- Delegation table: Add version for CAS operations
ALTER TABLE caliber_delegation ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;

-- Handoff table: Add version for CAS operations
ALTER TABLE caliber_handoff ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;

-- ============================================================================
-- PHASE 2: TIMEOUT TRACKING FOR SAGA PATTERNS
-- ============================================================================

-- Delegation: Timeout and progress tracking for saga state machines
ALTER TABLE caliber_delegation ADD COLUMN IF NOT EXISTS timeout_at TIMESTAMPTZ;
ALTER TABLE caliber_delegation ADD COLUMN IF NOT EXISTS last_progress_at TIMESTAMPTZ DEFAULT NOW();

-- Handoff: Timeout and progress tracking for saga state machines
ALTER TABLE caliber_handoff ADD COLUMN IF NOT EXISTS timeout_at TIMESTAMPTZ;
ALTER TABLE caliber_handoff ADD COLUMN IF NOT EXISTS last_progress_at TIMESTAMPTZ DEFAULT NOW();

-- ============================================================================
-- PHASE 3: INDEXES FOR TIMEOUT QUERIES
-- ============================================================================

-- Index for finding delegations that have timed out (active states only)
CREATE INDEX IF NOT EXISTS idx_delegation_timeout ON caliber_delegation (timeout_at)
WHERE status IN ('pending', 'accepted', 'in_progress') AND timeout_at IS NOT NULL;

-- Index for finding handoffs that have timed out (active states only)
CREATE INDEX IF NOT EXISTS idx_handoff_timeout ON caliber_handoff (timeout_at)
WHERE status IN ('initiated', 'accepted') AND timeout_at IS NOT NULL;

-- Index for finding delegations by last progress time (for stale detection)
CREATE INDEX IF NOT EXISTS idx_delegation_progress ON caliber_delegation (last_progress_at)
WHERE status IN ('pending', 'accepted', 'in_progress');

-- Index for finding handoffs by last progress time (for stale detection)
CREATE INDEX IF NOT EXISTS idx_handoff_progress ON caliber_handoff (last_progress_at)
WHERE status IN ('initiated', 'accepted');

-- ============================================================================
-- PHASE 4: UNIQUE CONSTRAINTS TO PREVENT DUPLICATE SAGAS
-- ============================================================================

-- Prevent duplicate pending delegations for the same task
-- Uses MD5 hash of task_description to handle TEXT comparison in unique index
-- COALESCE handles NULL delegatee_agent_id for unassigned delegations
CREATE UNIQUE INDEX IF NOT EXISTS idx_delegation_unique_pending ON caliber_delegation (
    delegator_agent_id,
    COALESCE(delegatee_agent_id, '00000000-0000-0000-0000-000000000000'::uuid),
    parent_trajectory_id,
    md5(task_description)
) WHERE status = 'pending';

-- Prevent duplicate initiated handoffs for the same context
CREATE UNIQUE INDEX IF NOT EXISTS idx_handoff_unique_initiated ON caliber_handoff (
    from_agent_id,
    trajectory_id,
    scope_id
) WHERE status = 'initiated';

-- ============================================================================
-- PHASE 5: IDEMPOTENCY KEY TABLE
-- ============================================================================

-- Table for storing idempotency keys to prevent duplicate API operations
-- Keys expire after a configurable TTL (typically 24h-7d)
CREATE TABLE IF NOT EXISTS caliber_idempotency_key (
    -- The idempotency key provided by the client
    idempotency_key TEXT PRIMARY KEY,

    -- Tenant isolation
    tenant_id UUID NOT NULL,

    -- Operation identifier (e.g., "create_delegation", "initiate_handoff")
    operation TEXT NOT NULL,

    -- Hash of the original request (method + path + body) for conflict detection
    request_hash BYTEA NOT NULL,

    -- Cached response to return for duplicate requests
    response_status INTEGER NOT NULL,
    response_body JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

-- Index for cleaning up expired keys
CREATE INDEX IF NOT EXISTS idx_idempotency_expires ON caliber_idempotency_key (expires_at);

-- Index for tenant-scoped lookups
CREATE INDEX IF NOT EXISTS idx_idempotency_tenant ON caliber_idempotency_key (tenant_id, operation);

-- ============================================================================
-- PHASE 6: STUCK SAGA DETECTION FUNCTIONS
-- ============================================================================

-- Function to find delegations that are stuck (timed out or stale)
-- Returns delegations that:
-- 1. Have explicit timeout_at that has passed, OR
-- 2. Have been in active state longer than the threshold with no timeout set
CREATE OR REPLACE FUNCTION caliber_find_stuck_delegations(
    p_timeout_threshold INTERVAL DEFAULT '1 hour'
)
RETURNS TABLE (
    delegation_id UUID,
    status TEXT,
    created_at TIMESTAMPTZ,
    stuck_duration INTERVAL
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        d.delegation_id,
        d.status,
        d.created_at,
        NOW() - COALESCE(d.last_progress_at, d.created_at) AS stuck_duration
    FROM caliber_delegation d
    WHERE d.status IN ('pending', 'accepted', 'in_progress')
      AND (
          -- Explicit timeout has passed
          (d.timeout_at IS NOT NULL AND d.timeout_at < NOW())
          OR
          -- No explicit timeout but exceeded threshold since last progress
          (d.timeout_at IS NULL AND d.last_progress_at + p_timeout_threshold < NOW())
          OR
          -- No progress timestamp (legacy), use created_at
          (d.timeout_at IS NULL AND d.last_progress_at IS NULL
           AND d.created_at + p_timeout_threshold < NOW())
      );
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to find handoffs that are stuck (timed out or stale)
CREATE OR REPLACE FUNCTION caliber_find_stuck_handoffs(
    p_timeout_threshold INTERVAL DEFAULT '30 minutes'
)
RETURNS TABLE (
    handoff_id UUID,
    status TEXT,
    initiated_at TIMESTAMPTZ,
    stuck_duration INTERVAL
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        h.handoff_id,
        h.status,
        h.initiated_at,
        NOW() - COALESCE(h.last_progress_at, h.initiated_at) AS stuck_duration
    FROM caliber_handoff h
    WHERE h.status IN ('initiated', 'accepted')
      AND (
          -- Explicit timeout has passed
          (h.timeout_at IS NOT NULL AND h.timeout_at < NOW())
          OR
          -- No explicit timeout but exceeded threshold since last progress
          (h.timeout_at IS NULL AND h.last_progress_at + p_timeout_threshold < NOW())
          OR
          -- No progress timestamp (legacy), use initiated_at
          (h.timeout_at IS NULL AND h.last_progress_at IS NULL
           AND h.initiated_at + p_timeout_threshold < NOW())
      );
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to timeout a stuck delegation with CAS protection
-- Returns TRUE if the delegation was successfully timed out
CREATE OR REPLACE FUNCTION caliber_timeout_delegation(
    p_delegation_id UUID,
    p_reason TEXT DEFAULT 'Timeout',
    p_expected_version BIGINT DEFAULT NULL
)
RETURNS BOOLEAN AS $$
DECLARE
    v_updated BOOLEAN;
BEGIN
    UPDATE caliber_delegation
    SET
        status = 'failed',
        result = jsonb_build_object(
            'status', 'Failure',
            'error', p_reason,
            'timed_out_at', NOW()
        ),
        completed_at = NOW(),
        version = version + 1
    WHERE delegation_id = p_delegation_id
      AND status IN ('pending', 'accepted', 'in_progress')
      AND (p_expected_version IS NULL OR version = p_expected_version);

    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- Function to timeout a stuck handoff with CAS protection
-- Returns TRUE if the handoff was successfully timed out
CREATE OR REPLACE FUNCTION caliber_timeout_handoff(
    p_handoff_id UUID,
    p_reason TEXT DEFAULT 'Timeout',
    p_expected_version BIGINT DEFAULT NULL
)
RETURNS BOOLEAN AS $$
DECLARE
    v_updated BOOLEAN;
BEGIN
    UPDATE caliber_handoff
    SET
        status = 'rejected',
        completed_at = NOW(),
        version = version + 1
    WHERE handoff_id = p_handoff_id
      AND status IN ('initiated', 'accepted')
      AND (p_expected_version IS NULL OR version = p_expected_version);

    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- Function to update delegation progress (heartbeat)
-- Resets the last_progress_at timestamp to prevent timeout
CREATE OR REPLACE FUNCTION caliber_delegation_heartbeat(
    p_delegation_id UUID,
    p_expected_version BIGINT DEFAULT NULL
)
RETURNS BOOLEAN AS $$
DECLARE
    v_updated BOOLEAN;
BEGIN
    UPDATE caliber_delegation
    SET
        last_progress_at = NOW(),
        version = version + 1
    WHERE delegation_id = p_delegation_id
      AND status IN ('pending', 'accepted', 'in_progress')
      AND (p_expected_version IS NULL OR version = p_expected_version);

    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- Function to update handoff progress (heartbeat)
CREATE OR REPLACE FUNCTION caliber_handoff_heartbeat(
    p_handoff_id UUID,
    p_expected_version BIGINT DEFAULT NULL
)
RETURNS BOOLEAN AS $$
DECLARE
    v_updated BOOLEAN;
BEGIN
    UPDATE caliber_handoff
    SET
        last_progress_at = NOW(),
        version = version + 1
    WHERE handoff_id = p_handoff_id
      AND status IN ('initiated', 'accepted')
      AND (p_expected_version IS NULL OR version = p_expected_version);

    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PHASE 7: IDEMPOTENCY KEY MANAGEMENT FUNCTIONS
-- ============================================================================

-- Function to check/insert idempotency key atomically
-- Returns the cached response if key exists with matching hash
-- Returns NULL if key is new (caller should proceed with operation)
-- Raises exception if key exists with different hash (409 Conflict)
CREATE OR REPLACE FUNCTION caliber_check_idempotency_key(
    p_idempotency_key TEXT,
    p_tenant_id UUID,
    p_operation TEXT,
    p_request_hash BYTEA,
    p_ttl_interval INTERVAL DEFAULT '24 hours'
)
RETURNS TABLE (
    key_exists BOOLEAN,
    cached_status INTEGER,
    cached_body JSONB
) AS $$
DECLARE
    v_existing RECORD;
BEGIN
    -- Try to find existing key
    SELECT ik.request_hash, ik.response_status, ik.response_body
    INTO v_existing
    FROM caliber_idempotency_key ik
    WHERE ik.idempotency_key = p_idempotency_key
      AND ik.tenant_id = p_tenant_id;

    IF FOUND THEN
        -- Key exists - check if request hash matches
        IF v_existing.request_hash != p_request_hash THEN
            -- Different request with same key - conflict
            RAISE EXCEPTION 'Idempotency key conflict: key "%" was used with a different request'
                USING ERRCODE = '23505'; -- unique_violation
        END IF;

        -- Same request - return cached response
        RETURN QUERY SELECT TRUE, v_existing.response_status, v_existing.response_body;
        RETURN;
    END IF;

    -- Key doesn't exist - insert placeholder (will be updated with response)
    INSERT INTO caliber_idempotency_key (
        idempotency_key,
        tenant_id,
        operation,
        request_hash,
        response_status,
        response_body,
        expires_at
    ) VALUES (
        p_idempotency_key,
        p_tenant_id,
        p_operation,
        p_request_hash,
        0,  -- Placeholder status
        NULL,
        NOW() + p_ttl_interval
    );

    -- Return NULL to indicate caller should proceed
    RETURN QUERY SELECT FALSE, NULL::INTEGER, NULL::JSONB;
END;
$$ LANGUAGE plpgsql;

-- Function to store the response for an idempotency key
CREATE OR REPLACE FUNCTION caliber_store_idempotency_response(
    p_idempotency_key TEXT,
    p_tenant_id UUID,
    p_response_status INTEGER,
    p_response_body JSONB
)
RETURNS VOID AS $$
BEGIN
    UPDATE caliber_idempotency_key
    SET
        response_status = p_response_status,
        response_body = p_response_body
    WHERE idempotency_key = p_idempotency_key
      AND tenant_id = p_tenant_id;
END;
$$ LANGUAGE plpgsql;

-- Function to clean up expired idempotency keys
-- Returns the number of keys deleted
CREATE OR REPLACE FUNCTION caliber_cleanup_idempotency_keys()
RETURNS INTEGER AS $$
DECLARE
    v_deleted INTEGER;
BEGIN
    DELETE FROM caliber_idempotency_key
    WHERE expires_at < NOW();

    GET DIAGNOSTICS v_deleted = ROW_COUNT;
    RETURN v_deleted;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PHASE 8: CAS UPDATE HELPERS
-- ============================================================================

-- Generic CAS update for delegation status transitions
-- Returns the new version if successful, NULL if version mismatch
CREATE OR REPLACE FUNCTION caliber_delegation_cas_update(
    p_delegation_id UUID,
    p_expected_version BIGINT,
    p_new_status TEXT,
    p_result JSONB DEFAULT NULL,
    p_set_completed BOOLEAN DEFAULT FALSE
)
RETURNS BIGINT AS $$
DECLARE
    v_new_version BIGINT;
BEGIN
    UPDATE caliber_delegation
    SET
        status = p_new_status,
        result = COALESCE(p_result, result),
        completed_at = CASE WHEN p_set_completed THEN NOW() ELSE completed_at END,
        last_progress_at = NOW(),
        version = version + 1
    WHERE delegation_id = p_delegation_id
      AND version = p_expected_version
    RETURNING version INTO v_new_version;

    RETURN v_new_version;
END;
$$ LANGUAGE plpgsql;

-- Generic CAS update for handoff status transitions
CREATE OR REPLACE FUNCTION caliber_handoff_cas_update(
    p_handoff_id UUID,
    p_expected_version BIGINT,
    p_new_status TEXT,
    p_set_accepted BOOLEAN DEFAULT FALSE,
    p_set_completed BOOLEAN DEFAULT FALSE
)
RETURNS BIGINT AS $$
DECLARE
    v_new_version BIGINT;
BEGIN
    UPDATE caliber_handoff
    SET
        status = p_new_status,
        accepted_at = CASE WHEN p_set_accepted THEN NOW() ELSE accepted_at END,
        completed_at = CASE WHEN p_set_completed THEN NOW() ELSE completed_at END,
        last_progress_at = NOW(),
        version = version + 1
    WHERE handoff_id = p_handoff_id
      AND version = p_expected_version
    RETURNING version INTO v_new_version;

    RETURN v_new_version;
END;
$$ LANGUAGE plpgsql;

-- Generic CAS update for lock renewal
CREATE OR REPLACE FUNCTION caliber_lock_cas_renew(
    p_lock_id UUID,
    p_expected_version BIGINT,
    p_new_expires_at TIMESTAMPTZ
)
RETURNS BIGINT AS $$
DECLARE
    v_new_version BIGINT;
BEGIN
    UPDATE caliber_lock
    SET
        expires_at = p_new_expires_at,
        version = version + 1
    WHERE lock_id = p_lock_id
      AND version = p_expected_version
      AND expires_at > NOW()  -- Can only renew non-expired locks
    RETURNING version INTO v_new_version;

    RETURN v_new_version;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PHASE 9: UPDATE SCHEMA VERSION
-- ============================================================================

INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (
    3,
    'Distributed correctness: CAS versioning, timeout tracking, idempotency keys',
    'distributed-correctness-v3'
)
ON CONFLICT (version) DO UPDATE SET
    applied_at = NOW(),
    description = EXCLUDED.description,
    checksum = EXCLUDED.checksum;

COMMIT;

-- ============================================================================
-- MIGRATION NOTES
-- ============================================================================

-- After running this migration:
--
-- 1. Version columns enable Compare-And-Swap (CAS) operations:
--    - Always read version when fetching entities
--    - Pass expected_version to update functions
--    - Handle NULL return (version mismatch) with retry logic
--
-- 2. Timeout tracking requires periodic cleanup:
--    - Call caliber_find_stuck_delegations() periodically (e.g., every minute)
--    - Call caliber_timeout_delegation() for each stuck delegation
--    - Consider alerting on stuck sagas
--
-- 3. Idempotency keys require periodic cleanup:
--    - Call caliber_cleanup_idempotency_keys() periodically (e.g., hourly)
--    - Keys typically expire after 24 hours
--
-- 4. Unique constraints prevent duplicate sagas:
--    - Clients get 409 Conflict if attempting duplicate pending delegation
--    - Use ON CONFLICT handling in application code
