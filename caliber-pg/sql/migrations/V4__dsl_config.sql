-- ============================================================================
-- CALIBER DSL CONFIGURATION STORAGE MIGRATION
-- Version: 4
-- Description: Add tables for DSL config storage and deployment
-- ============================================================================

-- ============================================================================
-- PHASE 1: DSL CONFIG TABLE
-- ============================================================================

-- Table to store DSL configurations for each tenant
-- Supports versioning and deployment status tracking
CREATE TABLE IF NOT EXISTS caliber_dsl_config (
    config_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES caliber_tenant(tenant_id),
    name TEXT NOT NULL,
    version INT NOT NULL DEFAULT 1,
    -- The raw DSL source code
    dsl_source TEXT NOT NULL,
    -- Parsed AST (JSONB for queryability)
    ast JSONB NOT NULL,
    -- Compiled configuration (JSONB, populated after successful compile)
    compiled JSONB,
    -- Status: draft, deployed, archived
    status TEXT NOT NULL DEFAULT 'draft',
    -- When this config was deployed (NULL if never deployed)
    deployed_at TIMESTAMPTZ,
    -- Deployment metadata (e.g., who deployed, what was replaced)
    deployed_by UUID REFERENCES caliber_agent(agent_id),
    -- Creation timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Last update timestamp
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Unique constraint: one name/version combo per tenant
    CONSTRAINT caliber_dsl_config_tenant_name_version_unique 
        UNIQUE (tenant_id, name, version)
);

-- ============================================================================
-- PHASE 2: INDEXES FOR COMMON QUERIES
-- ============================================================================

-- Find configs by tenant and status (most common query)
CREATE INDEX IF NOT EXISTS idx_dsl_config_tenant_status 
    ON caliber_dsl_config(tenant_id, status);

-- Find the latest version of a config
CREATE INDEX IF NOT EXISTS idx_dsl_config_tenant_name_version 
    ON caliber_dsl_config(tenant_id, name, version DESC);

-- Find deployed configs for hot-reload
CREATE INDEX IF NOT EXISTS idx_dsl_config_deployed 
    ON caliber_dsl_config(tenant_id, status, deployed_at DESC) 
    WHERE status = 'deployed';

-- ============================================================================
-- PHASE 3: DSL DEPLOYMENT HISTORY TABLE
-- ============================================================================

-- Audit table for tracking deployment history
CREATE TABLE IF NOT EXISTS caliber_dsl_deployment (
    deployment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES caliber_tenant(tenant_id),
    config_id UUID NOT NULL REFERENCES caliber_dsl_config(config_id),
    -- Previous config that was replaced (NULL for first deployment)
    previous_config_id UUID REFERENCES caliber_dsl_config(config_id),
    -- Deployment action: deploy, rollback, archive
    action TEXT NOT NULL DEFAULT 'deploy',
    -- Who initiated the deployment
    deployed_by UUID REFERENCES caliber_agent(agent_id),
    -- Deployment timestamp
    deployed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Optional notes about the deployment
    notes TEXT,
    -- Metadata (e.g., diff summary, validation warnings)
    metadata JSONB
);

-- Index for finding deployment history by tenant and config
CREATE INDEX IF NOT EXISTS idx_dsl_deployment_tenant 
    ON caliber_dsl_deployment(tenant_id, deployed_at DESC);

CREATE INDEX IF NOT EXISTS idx_dsl_deployment_config 
    ON caliber_dsl_deployment(config_id, deployed_at DESC);

-- ============================================================================
-- PHASE 4: ACTIVE CONFIG VIEW
-- ============================================================================

-- View to quickly find the currently active config for each tenant/name
CREATE OR REPLACE VIEW caliber_dsl_active_config AS
SELECT DISTINCT ON (tenant_id, name)
    config_id,
    tenant_id,
    name,
    version,
    dsl_source,
    ast,
    compiled,
    deployed_at,
    deployed_by,
    created_at,
    updated_at
FROM caliber_dsl_config
WHERE status = 'deployed'
ORDER BY tenant_id, name, version DESC;

-- ============================================================================
-- PHASE 5: HELPER FUNCTIONS
-- ============================================================================

-- Function to get the active config for a tenant by name
CREATE OR REPLACE FUNCTION caliber_get_active_dsl_config(
    p_tenant_id UUID,
    p_name TEXT DEFAULT 'default'
) RETURNS TABLE (
    config_id UUID,
    name TEXT,
    version INT,
    dsl_source TEXT,
    ast JSONB,
    compiled JSONB,
    deployed_at TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        c.config_id,
        c.name,
        c.version,
        c.dsl_source,
        c.ast,
        c.compiled,
        c.deployed_at
    FROM caliber_dsl_config c
    WHERE c.tenant_id = p_tenant_id
      AND c.name = p_name
      AND c.status = 'deployed'
    ORDER BY c.version DESC
    LIMIT 1;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to deploy a new config version
CREATE OR REPLACE FUNCTION caliber_deploy_dsl_config(
    p_config_id UUID,
    p_deployed_by UUID DEFAULT NULL,
    p_notes TEXT DEFAULT NULL
) RETURNS BOOLEAN AS $$
DECLARE
    v_tenant_id UUID;
    v_name TEXT;
    v_previous_config_id UUID;
BEGIN
    -- Get config details
    SELECT tenant_id, name INTO v_tenant_id, v_name
    FROM caliber_dsl_config
    WHERE config_id = p_config_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Config not found: %', p_config_id;
    END IF;

    -- Find currently deployed config (if any)
    SELECT config_id INTO v_previous_config_id
    FROM caliber_dsl_config
    WHERE tenant_id = v_tenant_id
      AND name = v_name
      AND status = 'deployed'
    ORDER BY version DESC
    LIMIT 1;

    -- Archive the previous config
    IF v_previous_config_id IS NOT NULL AND v_previous_config_id != p_config_id THEN
        UPDATE caliber_dsl_config
        SET status = 'archived',
            updated_at = NOW()
        WHERE config_id = v_previous_config_id;
    END IF;

    -- Deploy the new config
    UPDATE caliber_dsl_config
    SET status = 'deployed',
        deployed_at = NOW(),
        deployed_by = p_deployed_by,
        updated_at = NOW()
    WHERE config_id = p_config_id;

    -- Record in deployment history
    INSERT INTO caliber_dsl_deployment (
        tenant_id, config_id, previous_config_id, action, deployed_by, notes
    ) VALUES (
        v_tenant_id, p_config_id, v_previous_config_id, 'deploy', p_deployed_by, p_notes
    );

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PHASE 6: UPDATE SCHEMA VERSION
-- ============================================================================

-- Record this migration in the schema version table
INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (4, 'DSL configuration storage and deployment', 'dsl-config-v4')
ON CONFLICT (version) DO UPDATE SET
    applied_at = NOW(),
    description = EXCLUDED.description,
    checksum = EXCLUDED.checksum;

-- ============================================================================
-- MIGRATION NOTES
-- ============================================================================

-- This migration adds:
-- 1. caliber_dsl_config: Main table for storing DSL configs
-- 2. caliber_dsl_deployment: Audit table for deployment history
-- 3. caliber_dsl_active_config: View for quick active config lookup
-- 4. caliber_get_active_dsl_config(): Function to get active config
-- 5. caliber_deploy_dsl_config(): Function to deploy a config
--
-- Usage:
-- 1. Insert new config: INSERT INTO caliber_dsl_config (tenant_id, name, dsl_source, ast, compiled)
-- 2. Deploy: SELECT caliber_deploy_dsl_config(config_id, agent_id, 'deployment notes');
-- 3. Get active: SELECT * FROM caliber_get_active_dsl_config(tenant_id, 'default');
