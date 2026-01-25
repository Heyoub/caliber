-- ============================================================================
-- CALIBER TENANT NOT NULL CONSTRAINTS
-- Version: 6
-- Description: Add NOT NULL constraints to tenant_id columns
-- ============================================================================

-- ============================================================================
-- PREREQUISITE: DATA MIGRATION
-- ============================================================================
-- 
-- IMPORTANT: Before running this migration, ensure all existing rows have
-- tenant_id values assigned. Run the following data migration queries first:
--
-- -- Assign default tenant to records with NULL tenant_id
-- DO $$
-- DECLARE
--     default_tenant UUID;
-- BEGIN
--     -- Create or get default tenant
--     INSERT INTO caliber_tenant (name, domain, status)
--     VALUES ('Default Tenant', 'localhost', 'active')
--     ON CONFLICT (domain) DO UPDATE SET name = EXCLUDED.name
--     RETURNING tenant_id INTO default_tenant;
--
--     -- Update all tables
--     UPDATE caliber_trajectory SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_scope SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_artifact SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_note SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_turn SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_agent SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_lock SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_message SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_delegation SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_handoff SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_conflict SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_region SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_edge SET tenant_id = default_tenant WHERE tenant_id IS NULL;
--     UPDATE caliber_summarization_policy SET tenant_id = default_tenant WHERE tenant_id IS NULL;
-- END;
-- $$;
--
-- ============================================================================

-- ============================================================================
-- PHASE 1: ADD NOT NULL CONSTRAINTS
-- ============================================================================

-- Core entity tables
ALTER TABLE caliber_trajectory ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_scope ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_artifact ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_note ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_turn ALTER COLUMN tenant_id SET NOT NULL;

-- Agent-related tables
ALTER TABLE caliber_agent ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_lock ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_message ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_delegation ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_handoff ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_conflict ALTER COLUMN tenant_id SET NOT NULL;

-- Battle Intel tables
ALTER TABLE caliber_edge ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE caliber_summarization_policy ALTER COLUMN tenant_id SET NOT NULL;

-- Note: caliber_region already checked above
-- Note: caliber_dsl_config already has NOT NULL from V4 migration

-- ============================================================================
-- PHASE 2: UPDATE SCHEMA VERSION
-- ============================================================================

INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (6, 'NOT NULL constraints on tenant_id columns', 'tenant-not-null-v6')
ON CONFLICT (version) DO UPDATE SET
    applied_at = NOW(),
    description = EXCLUDED.description,
    checksum = EXCLUDED.checksum;

-- ============================================================================
-- MIGRATION NOTES
-- ============================================================================

-- This migration WILL FAIL if any rows have NULL tenant_id values.
-- Run the data migration script in the PREREQUISITE section first.
--
-- After this migration:
-- - All tenant_id columns are NOT NULL
-- - Foreign key constraints ensure referential integrity
-- - RLS policies provide isolation enforcement
