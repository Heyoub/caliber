-- ============================================================================
-- CALIBER SHARED LOCK FIX
-- Version: 7
-- Description: Fix shared lock semantics using PostgreSQL advisory locks
-- ============================================================================

-- ============================================================================
-- PROBLEM STATEMENT
-- ============================================================================
--
-- The current unique constraint on caliber_lock:
--     UNIQUE (resource_type, resource_id, mode)
--
-- This is BROKEN because it only allows ONE shared lock holder.
-- Shared locks should allow MULTIPLE holders.
--
-- SOLUTION:
-- 1. Remove the broken constraint
-- 2. Add constraint per holder (unique per agent + resource)
-- 3. Use PostgreSQL advisory locks for actual enforcement
-- 4. Keep the table for audit trail only
--
-- ============================================================================

-- ============================================================================
-- PHASE 1: DROP BROKEN CONSTRAINT
-- ============================================================================

-- The constraint name may vary; try both common patterns
DO $$
BEGIN
    -- Try the auto-generated name pattern
    ALTER TABLE caliber_lock DROP CONSTRAINT IF EXISTS caliber_lock_resource_type_resource_id_mode_key;
EXCEPTION
    WHEN undefined_object THEN NULL;
END;
$$;

DO $$
BEGIN
    -- Try another common pattern
    ALTER TABLE caliber_lock DROP CONSTRAINT IF EXISTS caliber_lock_resource_mode_unique;
EXCEPTION
    WHEN undefined_object THEN NULL;
END;
$$;

-- ============================================================================
-- PHASE 2: ADD CORRECT CONSTRAINT
-- ============================================================================

-- One lock record per holder per resource (allows multiple shared holders)
ALTER TABLE caliber_lock ADD CONSTRAINT caliber_lock_holder_resource_unique
    UNIQUE (resource_type, resource_id, holder_agent_id);

-- Add comment explaining the design
COMMENT ON TABLE caliber_lock IS 
    'Audit log for distributed locks. Actual enforcement uses pg_advisory_xact_lock(). '
    'Multiple agents can hold shared locks on the same resource.';

-- ============================================================================
-- PHASE 3: ADD ADVISORY LOCK FUNCTIONS
-- ============================================================================

-- Compute lock key using FNV-1a hash (matches Rust compute_lock_key)
CREATE OR REPLACE FUNCTION caliber_compute_lock_key(
    p_resource_type TEXT,
    p_resource_id UUID
) RETURNS BIGINT AS $$
DECLARE
    fnv_offset_basis BIGINT := x'cbf29ce484222325'::bigint;
    fnv_prime BIGINT := x'100000001b3'::bigint;
    hash BIGINT := fnv_offset_basis;
    byte_val INT;
    resource_bytes BYTEA;
BEGIN
    -- Hash resource type bytes
    FOR byte_val IN SELECT get_byte(p_resource_type::bytea, i) 
                    FROM generate_series(0, octet_length(p_resource_type) - 1) AS i
    LOOP
        hash := hash # byte_val;  -- XOR
        hash := hash * fnv_prime;
    END LOOP;

    -- Hash resource ID bytes
    resource_bytes := decode(replace(p_resource_id::text, '-', ''), 'hex');
    FOR byte_val IN SELECT get_byte(resource_bytes, i) 
                    FROM generate_series(0, octet_length(resource_bytes) - 1) AS i
    LOOP
        hash := hash # byte_val;  -- XOR
        hash := hash * fnv_prime;
    END LOOP;

    RETURN hash;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Try to acquire an exclusive lock (non-blocking)
CREATE OR REPLACE FUNCTION caliber_try_lock_exclusive(
    p_tenant_id UUID,
    p_resource_type TEXT,
    p_resource_id UUID,
    p_holder_agent_id UUID,
    p_timeout_ms INT DEFAULT 5000
) RETURNS TABLE (
    acquired BOOLEAN,
    lock_id UUID,
    message TEXT
) AS $$
DECLARE
    lock_key BIGINT;
    v_lock_id UUID;
    v_acquired BOOLEAN;
    v_expires_at TIMESTAMPTZ;
BEGIN
    lock_key := caliber_compute_lock_key(p_resource_type, p_resource_id);
    
    -- Try to acquire advisory lock (transaction-scoped)
    v_acquired := pg_try_advisory_xact_lock(lock_key);
    
    IF NOT v_acquired THEN
        RETURN QUERY SELECT false, NULL::UUID, 'Lock contention - resource is locked by another holder'::TEXT;
        RETURN;
    END IF;
    
    -- Calculate expiry
    v_expires_at := NOW() + (p_timeout_ms || ' milliseconds')::INTERVAL;
    
    -- Record in audit table
    INSERT INTO caliber_lock (
        tenant_id, resource_type, resource_id, holder_agent_id, mode, acquired_at, expires_at
    ) VALUES (
        p_tenant_id, p_resource_type, p_resource_id, p_holder_agent_id, 'Exclusive', NOW(), v_expires_at
    )
    ON CONFLICT (resource_type, resource_id, holder_agent_id) 
    DO UPDATE SET acquired_at = NOW(), expires_at = EXCLUDED.expires_at, mode = 'Exclusive'
    RETURNING caliber_lock.lock_id INTO v_lock_id;
    
    RETURN QUERY SELECT true, v_lock_id, 'Exclusive lock acquired'::TEXT;
END;
$$ LANGUAGE plpgsql;

-- Try to acquire a shared lock (non-blocking)
CREATE OR REPLACE FUNCTION caliber_try_lock_shared(
    p_tenant_id UUID,
    p_resource_type TEXT,
    p_resource_id UUID,
    p_holder_agent_id UUID,
    p_timeout_ms INT DEFAULT 5000
) RETURNS TABLE (
    acquired BOOLEAN,
    lock_id UUID,
    message TEXT
) AS $$
DECLARE
    lock_key BIGINT;
    v_lock_id UUID;
    v_acquired BOOLEAN;
    v_expires_at TIMESTAMPTZ;
BEGIN
    lock_key := caliber_compute_lock_key(p_resource_type, p_resource_id);
    
    -- Try to acquire shared advisory lock (transaction-scoped)
    -- Multiple holders can acquire shared locks simultaneously
    v_acquired := pg_try_advisory_xact_lock_shared(lock_key);
    
    IF NOT v_acquired THEN
        -- Shared lock failed - there's an exclusive lock
        RETURN QUERY SELECT false, NULL::UUID, 'Lock contention - resource has exclusive lock'::TEXT;
        RETURN;
    END IF;
    
    -- Calculate expiry
    v_expires_at := NOW() + (p_timeout_ms || ' milliseconds')::INTERVAL;
    
    -- Record in audit table
    INSERT INTO caliber_lock (
        tenant_id, resource_type, resource_id, holder_agent_id, mode, acquired_at, expires_at
    ) VALUES (
        p_tenant_id, p_resource_type, p_resource_id, p_holder_agent_id, 'Shared', NOW(), v_expires_at
    )
    ON CONFLICT (resource_type, resource_id, holder_agent_id) 
    DO UPDATE SET acquired_at = NOW(), expires_at = EXCLUDED.expires_at, mode = 'Shared'
    RETURNING caliber_lock.lock_id INTO v_lock_id;
    
    RETURN QUERY SELECT true, v_lock_id, 'Shared lock acquired'::TEXT;
END;
$$ LANGUAGE plpgsql;

-- Release a lock (marks as released in audit table)
-- Note: Advisory lock is automatically released at end of transaction
CREATE OR REPLACE FUNCTION caliber_release_lock(
    p_lock_id UUID
) RETURNS BOOLEAN AS $$
BEGIN
    -- Mark as released in audit table
    DELETE FROM caliber_lock WHERE lock_id = p_lock_id;
    RETURN FOUND;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PHASE 4: ADD LOCK STATUS VIEW
-- ============================================================================

CREATE OR REPLACE VIEW caliber_active_locks AS
SELECT 
    l.lock_id,
    l.tenant_id,
    l.resource_type,
    l.resource_id,
    l.holder_agent_id,
    l.mode,
    l.acquired_at,
    l.expires_at,
    CASE 
        WHEN l.expires_at < NOW() THEN 'expired'
        ELSE 'active'
    END AS status,
    (SELECT COUNT(*) FROM caliber_lock l2 
     WHERE l2.resource_type = l.resource_type 
       AND l2.resource_id = l.resource_id 
       AND l2.mode = 'Shared'
       AND l2.expires_at >= NOW()) AS shared_holder_count
FROM caliber_lock l
WHERE l.expires_at >= NOW();

-- ============================================================================
-- PHASE 5: UPDATE SCHEMA VERSION
-- ============================================================================

INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (7, 'Fix shared lock semantics with advisory locks', 'shared-locks-v7')
ON CONFLICT (version) DO UPDATE SET
    applied_at = NOW(),
    description = EXCLUDED.description,
    checksum = EXCLUDED.checksum;

-- ============================================================================
-- MIGRATION NOTES
-- ============================================================================

-- This migration fixes the broken shared lock semantics.
--
-- BEFORE: UNIQUE (resource_type, resource_id, mode) - only 1 shared holder!
-- AFTER:  UNIQUE (resource_type, resource_id, holder_agent_id) - N shared holders
--
-- Lock enforcement now uses PostgreSQL advisory locks:
-- - pg_try_advisory_xact_lock(key) for exclusive
-- - pg_try_advisory_xact_lock_shared(key) for shared
--
-- Advisory locks are transaction-scoped, so they automatically release
-- when the transaction commits/rolls back.
--
-- The caliber_lock table is now an AUDIT LOG only. The actual lock
-- enforcement is done by PostgreSQL's built-in advisory locking system.
--
-- Usage:
-- SELECT * FROM caliber_try_lock_exclusive(tenant_id, 'trajectory', resource_id, agent_id);
-- SELECT * FROM caliber_try_lock_shared(tenant_id, 'trajectory', resource_id, agent_id);
-- SELECT caliber_release_lock(lock_id);
