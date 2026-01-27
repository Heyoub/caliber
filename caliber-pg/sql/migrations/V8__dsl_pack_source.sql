-- ============================================================================
-- CALIBER DSL PACK SOURCE STORAGE
-- Version: 8
-- Description: Store pack source (cal.toml + markdown) separately from compiled DSL
-- ============================================================================

CREATE TABLE IF NOT EXISTS caliber_dsl_pack (
    pack_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    config_id UUID NOT NULL REFERENCES caliber_dsl_config(config_id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES caliber_tenant(tenant_id),
    -- Source payload (TOML + markdown + metadata)
    pack_source JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- One pack per config version
    CONSTRAINT caliber_dsl_pack_config_unique UNIQUE (config_id)
);

CREATE INDEX IF NOT EXISTS idx_dsl_pack_tenant
    ON caliber_dsl_pack(tenant_id, created_at DESC);

INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (8, 'DSL pack source storage', 'dsl-pack-source-v8')
ON CONFLICT (version) DO UPDATE SET
    applied_at = NOW(),
    description = EXCLUDED.description,
    checksum = EXCLUDED.checksum;
