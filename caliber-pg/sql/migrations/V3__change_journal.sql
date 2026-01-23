-- ============================================================================
-- CALIBER CHANGE JOURNAL: CACHE INVALIDATION INFRASTRUCTURE
-- Version: 3
-- Description: Create change journal table and triggers for cache invalidation
-- ============================================================================

BEGIN;

-- ============================================================================
-- PHASE 1: CHANGE JOURNAL TABLE
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
-- PHASE 2: INDEXES FOR EFFICIENT POLLING
-- ============================================================================

-- Primary index for watermark-based polling: get changes since watermark for tenant
CREATE INDEX IF NOT EXISTS idx_changes_tenant_seq ON caliber_changes(tenant_id, change_id DESC);

-- Index for entity-specific invalidation: check if specific entity has changed
CREATE INDEX IF NOT EXISTS idx_changes_entity ON caliber_changes(tenant_id, entity_type, entity_id, change_id DESC);

-- Index for time-based cleanup and debugging
CREATE INDEX IF NOT EXISTS idx_changes_time ON caliber_changes(tenant_id, changed_at DESC);

-- ============================================================================
-- PHASE 3: GENERIC TRIGGER FUNCTION
-- ============================================================================

-- This trigger function extracts the entity type from the table name and the
-- entity ID from the appropriate column based on table structure.

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

    -- Return appropriate value for trigger
    IF TG_OP = 'DELETE' THEN RETURN OLD; ELSE RETURN NEW; END IF;
END; $$ LANGUAGE plpgsql;

-- ============================================================================
-- PHASE 4: CREATE TRIGGERS FOR ALL ENTITY TABLES
-- ============================================================================

-- Trajectory changes
DROP TRIGGER IF EXISTS trajectory_change_journal ON caliber_trajectory;
CREATE TRIGGER trajectory_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_trajectory FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Scope changes
DROP TRIGGER IF EXISTS scope_change_journal ON caliber_scope;
CREATE TRIGGER scope_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_scope FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Artifact changes
DROP TRIGGER IF EXISTS artifact_change_journal ON caliber_artifact;
CREATE TRIGGER artifact_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_artifact FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Note changes
DROP TRIGGER IF EXISTS note_change_journal ON caliber_note;
CREATE TRIGGER note_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_note FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Turn changes
DROP TRIGGER IF EXISTS turn_change_journal ON caliber_turn;
CREATE TRIGGER turn_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_turn FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Agent changes
DROP TRIGGER IF EXISTS agent_change_journal ON caliber_agent;
CREATE TRIGGER agent_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_agent FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Edge changes
DROP TRIGGER IF EXISTS edge_change_journal ON caliber_edge;
CREATE TRIGGER edge_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_edge FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Message changes
DROP TRIGGER IF EXISTS message_change_journal ON caliber_message;
CREATE TRIGGER message_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_message FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Delegation changes
DROP TRIGGER IF EXISTS delegation_change_journal ON caliber_delegation;
CREATE TRIGGER delegation_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_delegation FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Handoff changes
DROP TRIGGER IF EXISTS handoff_change_journal ON caliber_handoff;
CREATE TRIGGER handoff_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_handoff FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- Lock changes
DROP TRIGGER IF EXISTS lock_change_journal ON caliber_lock;
CREATE TRIGGER lock_change_journal AFTER INSERT OR UPDATE OR DELETE ON caliber_lock FOR EACH ROW EXECUTE FUNCTION caliber_record_change();

-- ============================================================================
-- PHASE 5: WATERMARK QUERY FUNCTIONS
-- ============================================================================

-- Get the current maximum change_id for a tenant (used as watermark)
CREATE OR REPLACE FUNCTION caliber_current_watermark(p_tenant_id UUID)
RETURNS BIGINT AS $$
    SELECT COALESCE(MAX(change_id), 0) FROM caliber_changes WHERE tenant_id = p_tenant_id;
$$ LANGUAGE SQL STABLE;

-- Check if there are any changes since the given watermark
-- Optionally filter by entity types for selective cache invalidation
CREATE OR REPLACE FUNCTION caliber_has_changes_since(p_tenant_id UUID, p_watermark BIGINT, p_entity_types TEXT[] DEFAULT NULL)
RETURNS BOOLEAN AS $$
    SELECT EXISTS (
        SELECT 1 FROM caliber_changes
        WHERE tenant_id = p_tenant_id AND change_id > p_watermark
          AND (p_entity_types IS NULL OR entity_type = ANY(p_entity_types))
    );
$$ LANGUAGE SQL STABLE;

-- Get all changes since the given watermark with optional limit
-- Returns changes in ascending order for sequential processing
CREATE OR REPLACE FUNCTION caliber_changes_since(p_tenant_id UUID, p_watermark BIGINT, p_limit INT DEFAULT 1000)
RETURNS TABLE (change_id BIGINT, entity_type TEXT, entity_id UUID, operation TEXT, changed_at TIMESTAMPTZ) AS $$
    SELECT change_id, entity_type, entity_id, operation, changed_at
    FROM caliber_changes WHERE tenant_id = p_tenant_id AND change_id > p_watermark
    ORDER BY change_id ASC LIMIT p_limit;
$$ LANGUAGE SQL STABLE;

-- ============================================================================
-- PHASE 6: CLEANUP FUNCTION
-- ============================================================================

-- Delete old change journal entries to prevent unbounded growth
-- Default retention is 7 days, but can be configured per-call
CREATE OR REPLACE FUNCTION caliber_cleanup_change_journal(p_retention_days INT DEFAULT 7)
RETURNS BIGINT AS $$
DECLARE v_deleted BIGINT;
BEGIN
    DELETE FROM caliber_changes WHERE changed_at < NOW() - (p_retention_days || ' days')::INTERVAL;
    GET DIAGNOSTICS v_deleted = ROW_COUNT;
    RETURN v_deleted;
END; $$ LANGUAGE plpgsql;

-- ============================================================================
-- PHASE 7: RECORD MIGRATION VERSION
-- ============================================================================

INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (3, 'Change journal for cache invalidation', 'change-journal-v3')
ON CONFLICT (version) DO UPDATE SET
    applied_at = NOW(),
    description = EXCLUDED.description,
    checksum = EXCLUDED.checksum;

COMMIT;

-- ============================================================================
-- MIGRATION NOTES
-- ============================================================================

-- This migration creates the infrastructure for cache invalidation:
--
-- 1. caliber_changes table: Records all entity changes with monotonic change_id
-- 2. Triggers on all entity tables: Automatically record changes
-- 3. Query functions:
--    - caliber_current_watermark(tenant_id): Get latest change_id
--    - caliber_has_changes_since(tenant_id, watermark, entity_types[]): Check for changes
--    - caliber_changes_since(tenant_id, watermark, limit): Get change details
-- 4. Cleanup function: caliber_cleanup_change_journal(retention_days)
--
-- Usage pattern:
-- 1. On startup, get current watermark: SELECT caliber_current_watermark($tenant_id)
-- 2. Periodically poll: SELECT caliber_has_changes_since($tenant_id, $watermark)
-- 3. If changes exist, invalidate cache and get new watermark
-- 4. Run cleanup periodically: SELECT caliber_cleanup_change_journal(7)
