-- ============================================================================
-- CALIBER ROW-LEVEL SECURITY (RLS) POLICIES
-- Version: 5
-- Description: Add RLS policies for tenant isolation enforcement at database level
-- ============================================================================

-- ============================================================================
-- PHASE 1: ENABLE RLS ON ALL TENANT-SCOPED TABLES
-- ============================================================================

-- Core entity tables
ALTER TABLE caliber_trajectory ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_scope ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_artifact ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_note ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_turn ENABLE ROW LEVEL SECURITY;

-- Agent-related tables
ALTER TABLE caliber_agent ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_lock ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_message ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_delegation ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_handoff ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_conflict ENABLE ROW LEVEL SECURITY;

-- Battle Intel tables
ALTER TABLE caliber_edge ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_summarization_policy ENABLE ROW LEVEL SECURITY;

-- DSL config tables
ALTER TABLE caliber_dsl_config ENABLE ROW LEVEL SECURITY;
ALTER TABLE caliber_dsl_deployment ENABLE ROW LEVEL SECURITY;

-- ============================================================================
-- PHASE 2: CREATE TENANT ISOLATION POLICIES
-- Uses session variable 'app.tenant_id' set by application
-- ============================================================================

-- Policy helper: Check if tenant context is set
CREATE OR REPLACE FUNCTION caliber_current_tenant_id()
RETURNS UUID AS $$
BEGIN
    RETURN current_setting('app.tenant_id', true)::uuid;
EXCEPTION
    WHEN OTHERS THEN
        RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Core entity policies
CREATE POLICY tenant_isolation_trajectory ON caliber_trajectory
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_scope ON caliber_scope
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_artifact ON caliber_artifact
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_note ON caliber_note
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_turn ON caliber_turn
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

-- Agent-related policies
CREATE POLICY tenant_isolation_agent ON caliber_agent
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_lock ON caliber_lock
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_message ON caliber_message
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_delegation ON caliber_delegation
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_handoff ON caliber_handoff
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_conflict ON caliber_conflict
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

-- Battle Intel policies
CREATE POLICY tenant_isolation_edge ON caliber_edge
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_summarization_policy ON caliber_summarization_policy
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

-- DSL config policies
CREATE POLICY tenant_isolation_dsl_config ON caliber_dsl_config
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation_dsl_deployment ON caliber_dsl_deployment
    FOR ALL
    USING (tenant_id = caliber_current_tenant_id() OR caliber_current_tenant_id() IS NULL);

-- ============================================================================
-- PHASE 3: ADMIN BYPASS ROLE (FOR MAINTENANCE)
-- ============================================================================

-- Create admin role if it doesn't exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'caliber_admin') THEN
        CREATE ROLE caliber_admin;
    END IF;
END
$$;

-- Admin bypass policies (allows full access for admin role)
CREATE POLICY admin_bypass_trajectory ON caliber_trajectory
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_scope ON caliber_scope
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_artifact ON caliber_artifact
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_note ON caliber_note
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_turn ON caliber_turn
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_agent ON caliber_agent
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_lock ON caliber_lock
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_message ON caliber_message
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_delegation ON caliber_delegation
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_handoff ON caliber_handoff
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_conflict ON caliber_conflict
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_edge ON caliber_edge
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_summarization_policy ON caliber_summarization_policy
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_dsl_config ON caliber_dsl_config
    FOR ALL TO caliber_admin
    USING (true);

CREATE POLICY admin_bypass_dsl_deployment ON caliber_dsl_deployment
    FOR ALL TO caliber_admin
    USING (true);

-- ============================================================================
-- PHASE 4: HELPER FUNCTION FOR SETTING TENANT CONTEXT
-- ============================================================================

-- Function to set the tenant context for RLS
-- Call this at the beginning of each request with the authenticated tenant_id
CREATE OR REPLACE FUNCTION caliber_set_tenant_context(p_tenant_id UUID)
RETURNS VOID AS $$
BEGIN
    -- Set the session variable for RLS policies
    -- 'true' makes it local to the current transaction
    PERFORM set_config('app.tenant_id', p_tenant_id::text, true);
END;
$$ LANGUAGE plpgsql;

-- Function to clear the tenant context (for connection pooling safety)
CREATE OR REPLACE FUNCTION caliber_clear_tenant_context()
RETURNS VOID AS $$
BEGIN
    PERFORM set_config('app.tenant_id', '', true);
END;
$$ LANGUAGE plpgsql;

-- Function to get the current tenant context
CREATE OR REPLACE FUNCTION caliber_get_tenant_context()
RETURNS UUID AS $$
BEGIN
    RETURN caliber_current_tenant_id();
END;
$$ LANGUAGE plpgsql STABLE;

-- ============================================================================
-- PHASE 5: UPDATE SCHEMA VERSION
-- ============================================================================

INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (5, 'Row-Level Security policies for tenant isolation', 'rls-policies-v5')
ON CONFLICT (version) DO UPDATE SET
    applied_at = NOW(),
    description = EXCLUDED.description,
    checksum = EXCLUDED.checksum;

-- ============================================================================
-- MIGRATION NOTES
-- ============================================================================

-- This migration enables Row-Level Security (RLS) for multi-tenant isolation.
--
-- HOW TO USE:
--
-- 1. At the start of each request, call:
--    SELECT caliber_set_tenant_context('<tenant-uuid>');
--
-- 2. All subsequent queries will automatically filter by tenant_id
--
-- 3. For admin operations (bypassing RLS), use the caliber_admin role:
--    SET ROLE caliber_admin;
--    -- perform admin operations
--    RESET ROLE;
--
-- FALLBACK BEHAVIOR:
-- - If app.tenant_id is not set or NULL, policies allow all rows
-- - This ensures backwards compatibility during migration
-- - In production, always ensure tenant context is set
--
-- IMPORTANT:
-- - RLS only works when connecting as a non-superuser
-- - Superusers bypass RLS by default
-- - For production, create a non-superuser role for the application
