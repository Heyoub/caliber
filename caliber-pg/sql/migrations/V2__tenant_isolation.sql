-- ============================================================================
-- CALIBER SECURITY HARDENING: TENANT ISOLATION MIGRATION
-- Version: 2
-- Description: Add tenant_id columns to all core entity tables for multi-tenant isolation
-- ============================================================================

-- ============================================================================
-- PHASE 1: ADD TENANT_ID COLUMNS (NULLABLE INITIALLY)
-- ============================================================================

-- Core entity tables
ALTER TABLE caliber_trajectory ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_scope ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_artifact ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_note ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_turn ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);

-- Agent-related tables
ALTER TABLE caliber_agent ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_lock ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_message ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_delegation ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_handoff ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_conflict ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_region ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);

-- Battle Intel tables
ALTER TABLE caliber_edge ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);
ALTER TABLE caliber_summarization_policy ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES caliber_tenant(tenant_id);

-- ============================================================================
-- PHASE 2: CREATE INDEXES FOR TENANT FILTERING
-- ============================================================================

-- These indexes are critical for performance when filtering by tenant
CREATE INDEX IF NOT EXISTS idx_trajectory_tenant ON caliber_trajectory(tenant_id);
CREATE INDEX IF NOT EXISTS idx_scope_tenant ON caliber_scope(tenant_id);
CREATE INDEX IF NOT EXISTS idx_artifact_tenant ON caliber_artifact(tenant_id);
CREATE INDEX IF NOT EXISTS idx_note_tenant ON caliber_note(tenant_id);
CREATE INDEX IF NOT EXISTS idx_turn_tenant ON caliber_turn(tenant_id);
CREATE INDEX IF NOT EXISTS idx_agent_tenant ON caliber_agent(tenant_id);
CREATE INDEX IF NOT EXISTS idx_lock_tenant ON caliber_lock(tenant_id);
CREATE INDEX IF NOT EXISTS idx_message_tenant ON caliber_message(tenant_id);
CREATE INDEX IF NOT EXISTS idx_delegation_tenant ON caliber_delegation(tenant_id);
CREATE INDEX IF NOT EXISTS idx_handoff_tenant ON caliber_handoff(tenant_id);
CREATE INDEX IF NOT EXISTS idx_conflict_tenant ON caliber_conflict(tenant_id);
CREATE INDEX IF NOT EXISTS idx_region_tenant ON caliber_region(tenant_id);
CREATE INDEX IF NOT EXISTS idx_edge_tenant ON caliber_edge(tenant_id);
CREATE INDEX IF NOT EXISTS idx_summarization_policy_tenant ON caliber_summarization_policy(tenant_id);

-- Composite indexes for common query patterns (tenant + other common filters)
CREATE INDEX IF NOT EXISTS idx_trajectory_tenant_status ON caliber_trajectory(tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_scope_tenant_active ON caliber_scope(tenant_id, is_active) WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_artifact_tenant_type ON caliber_artifact(tenant_id, artifact_type);
CREATE INDEX IF NOT EXISTS idx_note_tenant_type ON caliber_note(tenant_id, note_type);
CREATE INDEX IF NOT EXISTS idx_agent_tenant_status ON caliber_agent(tenant_id, status);

-- ============================================================================
-- PHASE 3: HELPER FUNCTION FOR TENANT VALIDATION
-- ============================================================================

-- Function to validate tenant access to a resource
-- Returns true if the resource belongs to the specified tenant, false otherwise
-- This can be used in RLS policies or direct queries
CREATE OR REPLACE FUNCTION caliber_validate_tenant_access(
    p_resource_tenant_id UUID,
    p_request_tenant_id UUID
) RETURNS BOOLEAN AS $$
BEGIN
    -- NULL tenant_id means legacy data (pre-migration) - deny by default in strict mode
    IF p_resource_tenant_id IS NULL THEN
        RETURN FALSE;
    END IF;

    RETURN p_resource_tenant_id = p_request_tenant_id;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- ============================================================================
-- PHASE 4: UPDATE SCHEMA VERSION
-- ============================================================================

-- Record this migration in the schema version table
INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (2, 'Tenant isolation columns added to all core entity tables', 'tenant-isolation-v2')
ON CONFLICT (version) DO UPDATE SET
    applied_at = NOW(),
    description = EXCLUDED.description,
    checksum = EXCLUDED.checksum;

-- ============================================================================
-- MIGRATION NOTES
-- ============================================================================

-- After running this migration:
-- 1. Existing data will have NULL tenant_id values
-- 2. Run a data migration script to assign tenant_id to existing records
-- 3. Update API handlers to pass tenant_id on create operations
-- 4. Update API handlers to validate tenant_id on read/update/delete
-- 5. Once all data has tenant_id assigned, consider making columns NOT NULL

-- Example data migration (run separately):
-- UPDATE caliber_trajectory SET tenant_id = '<default-tenant-uuid>' WHERE tenant_id IS NULL;
-- UPDATE caliber_scope SET tenant_id = '<default-tenant-uuid>' WHERE tenant_id IS NULL;
-- ... etc.
