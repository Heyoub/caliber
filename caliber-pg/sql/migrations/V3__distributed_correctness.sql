-- ============================================================================
-- CALIBER V3: DISTRIBUTED CORRECTNESS & CACHE INVALIDATION
-- Version: 3
-- Description: Complete distributed correctness infrastructure including:
--              - CAS version columns for optimistic concurrency
--              - Timeout tracking and saga pattern support
--              - Unique constraints preventing duplicate sagas
--              - Change journal for cache invalidation
--              - Idempotency key management
-- ============================================================================

BEGIN;

-- ============================================================================
-- PART A: COMPARE-AND-SWAP (CAS) VERSION COLUMNS
-- ============================================================================

-- Lock table: Add version for optimistic concurrency control
ALTER TABLE caliber_lock ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;

-- Delegation table: Add version for CAS operations
ALTER TABLE caliber_delegation ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;

-- Handoff table: Add version for CAS operations
ALTER TABLE caliber_handoff ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;

-- ============================================================================
-- PART B: TIMEOUT TRACKING FOR SAGA PATTERNS
-- ============================================================================

-- Delegation: Timeout and progress tracking for saga state machines
ALTER TABLE caliber_delegation ADD COLUMN IF NOT EXISTS timeout_at TIMESTAMPTZ;
ALTER TABLE caliber_delegation ADD COLUMN IF NOT EXISTS last_progress_at TIMESTAMPTZ DEFAULT NOW();

-- Handoff: Timeout and progress tracking for saga state machines
ALTER TABLE caliber_handoff ADD COLUMN IF NOT EXISTS timeout_at TIMESTAMPTZ;
ALTER TABLE caliber_handoff ADD COLUMN IF NOT EXISTS last_progress_at TIMESTAMPTZ DEFAULT NOW();

-- ============================================================================
-- PART C: CHANGE JOURNAL TABLE
-- ============================================================================

-- The change journal records all INSERT, UPDATE, DELETE operations on entity
-- tables. Each record has a monotonically increasing change_id that serves as
-- a watermark for cache invalidation polling.

CREATE TABLE IF NOT EXISTS caliber_changes (
    change_id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    operation TEXT NOT NULL CHECK (operation IN ('INSERT', 'UPDATE', 'DELETE')),
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- PART D: INDEXES
-- ============================================================================

-- Timeout query indexes
CREATE INDEX IF NOT EXISTS idx_delegation_timeout ON caliber_delegation (timeout_at)
WHERE status IN ('pending', 'accepted', 'in_progress') AND timeout_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_handoff_timeout ON caliber_handoff (timeout_at)
WHERE status IN ('initiated', 'accepted') AND timeout_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_delegation_progress ON caliber_delegation (last_progress_at)
WHERE status IN ('pending', 'accepted', 'in_progress');

CREATE INDEX IF NOT EXISTS idx_handoff_progress ON caliber_handoff (last_progress_at)
WHERE status IN ('initiated', 'accepted');

-- Change journal indexes
CREATE INDEX IF NOT EXISTS idx_changes_tenant_seq ON caliber_changes(tenant_id, change_id DESC);
CREATE INDEX IF NOT EXISTS idx_changes_entity ON caliber_changes(tenant_id, entity_type, entity_id, change_id DESC);
CREATE INDEX IF NOT EXISTS idx_changes_time ON caliber_changes(tenant_id, changed_at DESC);

-- ============================================================================
-- PART E: UNIQUE CONSTRAINTS TO PREVENT DUPLICATE SAGAS
-- ============================================================================

-- Prevent duplicate pending delegations for the same task
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
-- PART F: IDEMPOTENCY KEY TABLE
-- ============================================================================

CREATE TABLE IF NOT EXISTS caliber_idempotency_key (
    idempotency_key TEXT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    operation TEXT NOT NULL,
    request_hash BYTEA NOT NULL,
    response_status INTEGER NOT NULL,
    response_body JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_idempotency_expires ON caliber_idempotency_key (expires_at);
CREATE INDEX IF NOT EXISTS idx_idempotency_tenant ON caliber_idempotency_key (tenant_id, operation);

-- ============================================================================
-- PART G: CHANGE JOURNAL TRIGGER FUNCTION
-- ============================================================================

CREATE OR REPLACE FUNCTION caliber_record_change() RETURNS TRIGGER AS $$
DECLARE
    v_tenant_id UUID;
    v_entity_id UUID;
    v_entity_type TEXT;
BEGIN
    -- Extract entity type from table name (strip 'caliber_' prefix)
    v_entity_type := TG_TABLE_NAME;
    IF v_entity_type LIKE 'caliber_%' THEN
        v_entity_type := SUBSTRING(v_entity_type FROM 9);
    END IF;

    -- Extract tenant_id and entity_id based on operation type
    IF TG_OP = 'DELETE' THEN
        v_tenant_id := OLD.tenant_id;
        v_entity_id := CASE v_entity_type
            WHEN 'trajectory' THEN OLD.trajectory_id
            WHEN 'scope' THEN OLD.scope_id
            WHEN 'artifact' THEN OLD.artifact_id
            WHEN 'note' THEN OLD.note_id
            WHEN 'turn' THEN OLD.turn_id
            WHEN 'agent' THEN OLD.agent_id
            WHEN 'edge' THEN OLD.edge_id
            WHEN 'message' THEN OLD.message_id
            WHEN 'delegation' THEN OLD.delegation_id
            WHEN 'handoff' THEN OLD.handoff_id
            WHEN 'lock' THEN OLD.lock_id
            ELSE NULL END;
    ELSE
        v_tenant_id := NEW.tenant_id;
        v_entity_id := CASE v_entity_type
            WHEN 'trajectory' THEN NEW.trajectory_id
            WHEN 'scope' THEN NEW.scope_id
            WHEN 'artifact' THEN NEW.artifact_id
            WHEN 'note' THEN NEW.note_id
            WHEN 'turn' THEN NEW.turn_id
            WHEN 'agent' THEN NEW.agent_id
            WHEN 'edge' THEN NEW.edge_id
            WHEN 'message' THEN NEW.message_id
            WHEN 'delegation' THEN NEW.delegation_id
            WHEN 'handoff' THEN NEW.handoff_id
            WHEN 'lock' THEN NEW.lock_id
            ELSE NULL END;
    END IF;

    -- Only record if we have a valid tenant_id (skip legacy data without tenant)
    IF v_tenant_id IS NOT NULL THEN
        INSERT INTO caliber_changes (tenant_id, entity_type, entity_id, operation)
        VALUES (v_tenant_id, v_entity_type, v_entity_id, TG_OP);
    END IF;

    IF TG_OP = 'DELETE' THEN RETURN OLD; ELSE RETURN NEW; END IF;
END; $$ LANGUAGE plpgsql;

-- ============================================================================
-- PART H: CHANGE JOURNAL TRIGGERS
-- ============================================================================

DROP TRIGGER IF EXISTS trajectory_change_journal ON caliber_trajectory;
CREATE TRIGGER trajectory_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_trajectory FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS scope_change_journal ON caliber_scope;
CREATE TRIGGER scope_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_scope FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS artifact_change_journal ON caliber_artifact;
CREATE TRIGGER artifact_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_artifact FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS note_change_journal ON caliber_note;
CREATE TRIGGER note_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_note FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS turn_change_journal ON caliber_turn;
CREATE TRIGGER turn_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_turn FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS agent_change_journal ON caliber_agent;
CREATE TRIGGER agent_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_agent FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS edge_change_journal ON caliber_edge;
CREATE TRIGGER edge_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_edge FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS message_change_journal ON caliber_message;
CREATE TRIGGER message_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_message FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS delegation_change_journal ON caliber_delegation;
CREATE TRIGGER delegation_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_delegation FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS handoff_change_journal ON caliber_handoff;
CREATE TRIGGER handoff_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_handoff FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

DROP TRIGGER IF EXISTS lock_change_journal ON caliber_lock;
CREATE TRIGGER lock_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_lock FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- ============================================================================
-- PART I: STUCK SAGA DETECTION FUNCTIONS
-- ============================================================================

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
    SELECT d.delegation_id, d.status, d.created_at,
           NOW() - COALESCE(d.last_progress_at, d.created_at) AS stuck_duration
    FROM caliber_delegation d
    WHERE d.status IN ('pending', 'accepted', 'in_progress')
      AND (
          (d.timeout_at IS NOT NULL AND d.timeout_at < NOW())
          OR (d.timeout_at IS NULL AND d.last_progress_at + p_timeout_threshold < NOW())
          OR (d.timeout_at IS NULL AND d.last_progress_at IS NULL
              AND d.created_at + p_timeout_threshold < NOW())
      );
END;
$$ LANGUAGE plpgsql STABLE;

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
    SELECT h.handoff_id, h.status, h.initiated_at,
           NOW() - COALESCE(h.last_progress_at, h.initiated_at) AS stuck_duration
    FROM caliber_handoff h
    WHERE h.status IN ('initiated', 'accepted')
      AND (
          (h.timeout_at IS NOT NULL AND h.timeout_at < NOW())
          OR (h.timeout_at IS NULL AND h.last_progress_at + p_timeout_threshold < NOW())
          OR (h.timeout_at IS NULL AND h.last_progress_at IS NULL
              AND h.initiated_at + p_timeout_threshold < NOW())
      );
END;
$$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION caliber_timeout_delegation(
    p_delegation_id UUID,
    p_reason TEXT DEFAULT 'Timeout',
    p_expected_version BIGINT DEFAULT NULL
)
RETURNS BOOLEAN AS $$
DECLARE v_updated BOOLEAN;
BEGIN
    UPDATE caliber_delegation
    SET status = 'failed',
        result = jsonb_build_object('status', 'Failure', 'error', p_reason, 'timed_out_at', NOW()),
        completed_at = NOW(),
        version = version + 1
    WHERE delegation_id = p_delegation_id
      AND status IN ('pending', 'accepted', 'in_progress')
      AND (p_expected_version IS NULL OR version = p_expected_version);
    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION caliber_timeout_handoff(
    p_handoff_id UUID,
    p_reason TEXT DEFAULT 'Timeout',
    p_expected_version BIGINT DEFAULT NULL
)
RETURNS BOOLEAN AS $$
DECLARE v_updated BOOLEAN;
BEGIN
    UPDATE caliber_handoff
    SET status = 'rejected', completed_at = NOW(), version = version + 1
    WHERE handoff_id = p_handoff_id
      AND status IN ('initiated', 'accepted')
      AND (p_expected_version IS NULL OR version = p_expected_version);
    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION caliber_delegation_heartbeat(
    p_delegation_id UUID,
    p_expected_version BIGINT DEFAULT NULL
)
RETURNS BOOLEAN AS $$
DECLARE v_updated BOOLEAN;
BEGIN
    UPDATE caliber_delegation SET last_progress_at = NOW(), version = version + 1
    WHERE delegation_id = p_delegation_id
      AND status IN ('pending', 'accepted', 'in_progress')
      AND (p_expected_version IS NULL OR version = p_expected_version);
    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION caliber_handoff_heartbeat(
    p_handoff_id UUID,
    p_expected_version BIGINT DEFAULT NULL
)
RETURNS BOOLEAN AS $$
DECLARE v_updated BOOLEAN;
BEGIN
    UPDATE caliber_handoff SET last_progress_at = NOW(), version = version + 1
    WHERE handoff_id = p_handoff_id
      AND status IN ('initiated', 'accepted')
      AND (p_expected_version IS NULL OR version = p_expected_version);
    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART J: WATERMARK QUERY FUNCTIONS
-- ============================================================================

CREATE OR REPLACE FUNCTION caliber_current_watermark(p_tenant_id UUID)
RETURNS BIGINT AS $$
    SELECT COALESCE(MAX(change_id), 0) FROM caliber_changes WHERE tenant_id = p_tenant_id;
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION caliber_has_changes_since(p_tenant_id UUID, p_watermark BIGINT, p_entity_types TEXT[] DEFAULT NULL)
RETURNS BOOLEAN AS $$
    SELECT EXISTS (
        SELECT 1 FROM caliber_changes
        WHERE tenant_id = p_tenant_id AND change_id > p_watermark
          AND (p_entity_types IS NULL OR entity_type = ANY(p_entity_types))
    );
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION caliber_changes_since(p_tenant_id UUID, p_watermark BIGINT, p_limit INT DEFAULT 1000)
RETURNS TABLE (change_id BIGINT, entity_type TEXT, entity_id UUID, operation TEXT, changed_at TIMESTAMPTZ) AS $$
    SELECT change_id, entity_type, entity_id, operation, changed_at
    FROM caliber_changes WHERE tenant_id = p_tenant_id AND change_id > p_watermark
    ORDER BY change_id ASC LIMIT p_limit;
$$ LANGUAGE SQL STABLE;

-- ============================================================================
-- PART K: CLEANUP FUNCTIONS
-- ============================================================================

CREATE OR REPLACE FUNCTION caliber_cleanup_change_journal(p_retention_days INT DEFAULT 7)
RETURNS BIGINT AS $$
DECLARE v_deleted BIGINT;
BEGIN
    DELETE FROM caliber_changes WHERE changed_at < NOW() - (p_retention_days || ' days')::INTERVAL;
    GET DIAGNOSTICS v_deleted = ROW_COUNT;
    RETURN v_deleted;
END; $$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION caliber_cleanup_idempotency_keys()
RETURNS INTEGER AS $$
DECLARE v_deleted INTEGER;
BEGIN
    DELETE FROM caliber_idempotency_key WHERE expires_at < NOW();
    GET DIAGNOSTICS v_deleted = ROW_COUNT;
    RETURN v_deleted;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART L: IDEMPOTENCY KEY FUNCTIONS
-- ============================================================================

CREATE OR REPLACE FUNCTION caliber_check_idempotency_key(
    p_idempotency_key TEXT,
    p_tenant_id UUID,
    p_operation TEXT,
    p_request_hash BYTEA,
    p_ttl_interval INTERVAL DEFAULT '24 hours'
)
RETURNS TABLE (key_exists BOOLEAN, cached_status INTEGER, cached_body JSONB) AS $$
DECLARE v_existing RECORD;
BEGIN
    SELECT ik.request_hash, ik.response_status, ik.response_body
    INTO v_existing
    FROM caliber_idempotency_key ik
    WHERE ik.idempotency_key = p_idempotency_key AND ik.tenant_id = p_tenant_id;

    IF FOUND THEN
        IF v_existing.request_hash != p_request_hash THEN
            RAISE EXCEPTION 'Idempotency key conflict: key "%" was used with a different request'
                USING ERRCODE = '23505';
        END IF;
        RETURN QUERY SELECT TRUE, v_existing.response_status, v_existing.response_body;
        RETURN;
    END IF;

    INSERT INTO caliber_idempotency_key (
        idempotency_key, tenant_id, operation, request_hash,
        response_status, response_body, expires_at
    ) VALUES (
        p_idempotency_key, p_tenant_id, p_operation, p_request_hash,
        0, NULL, NOW() + p_ttl_interval
    );

    RETURN QUERY SELECT FALSE, NULL::INTEGER, NULL::JSONB;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION caliber_store_idempotency_response(
    p_idempotency_key TEXT,
    p_tenant_id UUID,
    p_response_status INTEGER,
    p_response_body JSONB
)
RETURNS VOID AS $$
BEGIN
    UPDATE caliber_idempotency_key
    SET response_status = p_response_status, response_body = p_response_body
    WHERE idempotency_key = p_idempotency_key AND tenant_id = p_tenant_id;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART M: CAS UPDATE HELPERS
-- ============================================================================

CREATE OR REPLACE FUNCTION caliber_delegation_cas_update(
    p_delegation_id UUID,
    p_expected_version BIGINT,
    p_new_status TEXT,
    p_result JSONB DEFAULT NULL,
    p_set_completed BOOLEAN DEFAULT FALSE
)
RETURNS BIGINT AS $$
DECLARE v_new_version BIGINT;
BEGIN
    UPDATE caliber_delegation
    SET status = p_new_status,
        result = COALESCE(p_result, result),
        completed_at = CASE WHEN p_set_completed THEN NOW() ELSE completed_at END,
        last_progress_at = NOW(),
        version = version + 1
    WHERE delegation_id = p_delegation_id AND version = p_expected_version
    RETURNING version INTO v_new_version;
    RETURN v_new_version;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION caliber_handoff_cas_update(
    p_handoff_id UUID,
    p_expected_version BIGINT,
    p_new_status TEXT,
    p_set_accepted BOOLEAN DEFAULT FALSE,
    p_set_completed BOOLEAN DEFAULT FALSE
)
RETURNS BIGINT AS $$
DECLARE v_new_version BIGINT;
BEGIN
    UPDATE caliber_handoff
    SET status = p_new_status,
        accepted_at = CASE WHEN p_set_accepted THEN NOW() ELSE accepted_at END,
        completed_at = CASE WHEN p_set_completed THEN NOW() ELSE completed_at END,
        last_progress_at = NOW(),
        version = version + 1
    WHERE handoff_id = p_handoff_id AND version = p_expected_version
    RETURNING version INTO v_new_version;
    RETURN v_new_version;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION caliber_lock_cas_renew(
    p_lock_id UUID,
    p_expected_version BIGINT,
    p_new_expires_at TIMESTAMPTZ
)
RETURNS BIGINT AS $$
DECLARE v_new_version BIGINT;
BEGIN
    UPDATE caliber_lock
    SET expires_at = p_new_expires_at, version = version + 1
    WHERE lock_id = p_lock_id
      AND version = p_expected_version
      AND expires_at > NOW()
    RETURNING version INTO v_new_version;
    RETURN v_new_version;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART N: RECORD MIGRATION VERSION
-- ============================================================================

INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (3, 'Distributed correctness: CAS, change journal, idempotency', 'v3-distributed-correctness')
ON CONFLICT (version) DO UPDATE SET
    applied_at = NOW(),
    description = EXCLUDED.description,
    checksum = EXCLUDED.checksum;

COMMIT;

-- ============================================================================
-- MIGRATION NOTES
-- ============================================================================
--
-- This migration provides complete distributed correctness infrastructure:
--
-- A. CAS Version Columns:
--    - Always read version when fetching Lock/Delegation/Handoff
--    - Pass expected_version to update functions
--    - Handle NULL return (version mismatch) with retry logic
--
-- B. Saga Timeout Tracking:
--    - caliber_find_stuck_delegations() finds stuck sagas
--    - caliber_timeout_delegation() safely times out with CAS
--    - caliber_delegation_heartbeat() resets progress timer
--    - Run cleanup job periodically (e.g., every minute)
--
-- C. Change Journal:
--    - caliber_current_watermark(tenant_id): Get latest change_id
--    - caliber_has_changes_since(tenant_id, watermark): Check for changes
--    - caliber_changes_since(tenant_id, watermark, limit): Get change details
--    - caliber_cleanup_change_journal(retention_days): Prune old entries
--
-- D. Idempotency:
--    - caliber_check_idempotency_key(): Check/insert key atomically
--    - caliber_store_idempotency_response(): Cache response
--    - caliber_cleanup_idempotency_keys(): Prune expired keys
