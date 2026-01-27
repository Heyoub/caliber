-- CALIBER Bootstrap SQL Schema
-- This SQL runs ONCE at extension install, NOT in hot path.
-- All hot-path operations use direct pgrx heap operations.

-- ============================================================================
-- SCHEMA VERSION TRACKING (for built-in migrations)
-- ============================================================================

CREATE TABLE IF NOT EXISTS caliber_schema_version (
    version INTEGER PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    checksum TEXT NOT NULL,
    execution_time_ms INTEGER
);

INSERT INTO caliber_schema_version (version, description, checksum)
VALUES (1, 'Initial schema - CALIBER 0.4.4', 'base')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION caliber_schema_version()
RETURNS INTEGER AS $$
    SELECT COALESCE(MAX(version), 0) FROM caliber_schema_version;
$$ LANGUAGE SQL STABLE;

-- ============================================================================
-- CONFIGURATION TABLE (Required by caliber-api)
-- ============================================================================

-- Single-row config table storing PCPConfig as JSONB
CREATE TABLE IF NOT EXISTS caliber_config (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),  -- Enforce single row
    config JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default configuration if not exists
-- This matches the PCPConfig structure expected by caliber-api
INSERT INTO caliber_config (id, config) VALUES (1, '{
    "context_dag": {
        "max_depth": 10,
        "prune_strategy": "OldestFirst"
    },
    "recovery": {
        "enabled": true,
        "frequency": "OnScopeClose",
        "max_checkpoints": 5
    },
    "dosage": {
        "max_tokens_per_scope": 8000,
        "max_artifacts_per_scope": 100,
        "max_notes_per_trajectory": 500
    },
    "anti_sprawl": {
        "max_trajectory_depth": 5,
        "max_concurrent_scopes": 10
    },
    "grounding": {
        "require_artifact_backing": true,
        "contradiction_threshold": 0.8,
        "conflict_resolution": "HighestConfidence"
    },
    "linting": {
        "max_artifact_size": 1048576,
        "min_confidence_threshold": 0.3
    },
    "staleness": {
        "stale_hours": 720
    }
}'::jsonb) ON CONFLICT DO NOTHING;

-- Get current configuration (called by caliber-api at startup)
CREATE OR REPLACE FUNCTION caliber_config_get()
RETURNS JSONB AS $$
    SELECT config FROM caliber_config WHERE id = 1;
$$ LANGUAGE SQL STABLE;

-- Update configuration (called by caliber-api for config changes)
CREATE OR REPLACE FUNCTION caliber_config_update(new_config JSONB)
RETURNS BOOLEAN AS $$
    UPDATE caliber_config
    SET config = new_config, updated_at = NOW()
    WHERE id = 1;
    SELECT TRUE;
$$ LANGUAGE SQL;

-- ============================================================================
-- TENANT TABLES (Multi-tenancy support)
-- ============================================================================

-- Tenant: Organization-level container
CREATE TABLE IF NOT EXISTS caliber_tenant (
    tenant_id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    domain TEXT UNIQUE,                    -- For auto-association (acme.com → "acme")
    workos_organization_id TEXT UNIQUE,    -- WorkOS mapping
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'archived')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

-- Tenant Member: User membership in a tenant
CREATE TABLE IF NOT EXISTS caliber_tenant_member (
    member_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES caliber_tenant(tenant_id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,                 -- WorkOS user ID
    email TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('admin', 'member', 'readonly')),
    first_name TEXT,
    last_name TEXT,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, user_id)
);

-- Public Email Domains: gmail, outlook, etc. - users get personal tenants
CREATE TABLE IF NOT EXISTS caliber_public_email_domain (
    domain TEXT PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed common public domains
INSERT INTO caliber_public_email_domain (domain) VALUES
    ('gmail.com'), ('googlemail.com'), ('outlook.com'), ('hotmail.com'),
    ('live.com'), ('yahoo.com'), ('icloud.com'), ('protonmail.com'),
    ('proton.me'), ('aol.com'), ('zoho.com')
ON CONFLICT DO NOTHING;

-- Tenant indexes
CREATE INDEX IF NOT EXISTS idx_tenant_domain ON caliber_tenant(domain) WHERE domain IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tenant_workos_org ON caliber_tenant(workos_organization_id) WHERE workos_organization_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tenant_status ON caliber_tenant(status);
CREATE INDEX IF NOT EXISTS idx_tenant_member_tenant ON caliber_tenant_member(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_member_user ON caliber_tenant_member(user_id);
CREATE INDEX IF NOT EXISTS idx_tenant_member_email ON caliber_tenant_member(email);

-- ============================================================================
-- CORE ENTITY TABLES
-- ============================================================================

-- Trajectory: Top-level task container
CREATE TABLE IF NOT EXISTS caliber_trajectory (
    trajectory_id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'failed', 'suspended')),
    parent_trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    root_trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    agent_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    outcome JSONB,
    metadata JSONB
);

-- Scope: Partitioned context window within a trajectory
CREATE TABLE IF NOT EXISTS caliber_scope (
    scope_id UUID PRIMARY KEY,
    trajectory_id UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id),
    parent_scope_id UUID REFERENCES caliber_scope(scope_id),
    name TEXT NOT NULL,
    purpose TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at TIMESTAMPTZ,
    checkpoint JSONB,
    token_budget INTEGER NOT NULL,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    metadata JSONB
);

-- Artifact: Typed output preserved across scopes
CREATE TABLE IF NOT EXISTS caliber_artifact (
    artifact_id UUID PRIMARY KEY,
    trajectory_id UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id),
    scope_id UUID NOT NULL REFERENCES caliber_scope(scope_id),
    artifact_type TEXT NOT NULL,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    content_hash BYTEA NOT NULL,
    embedding VECTOR,  -- pgvector type for similarity search
    provenance JSONB NOT NULL,
    ttl TEXT NOT NULL DEFAULT 'persistent',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    superseded_by UUID REFERENCES caliber_artifact(artifact_id),
    metadata JSONB
);

-- Note: Long-term cross-trajectory knowledge
CREATE TABLE IF NOT EXISTS caliber_note (
    note_id UUID PRIMARY KEY,
    note_type TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    content_hash BYTEA NOT NULL,
    embedding VECTOR,  -- pgvector type for similarity search
    source_trajectory_ids UUID[] NOT NULL DEFAULT '{}',
    source_artifact_ids UUID[] NOT NULL DEFAULT '{}',
    ttl TEXT NOT NULL DEFAULT 'persistent',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    access_count INTEGER NOT NULL DEFAULT 0,
    superseded_by UUID REFERENCES caliber_note(note_id),
    metadata JSONB,
    -- Battle Intel Feature 2: Abstraction levels (EVOLVE-MEM L0/L1/L2)
    abstraction_level TEXT NOT NULL DEFAULT 'raw' CHECK (abstraction_level IN ('raw', 'summary', 'principle')),
    source_note_ids UUID[] NOT NULL DEFAULT '{}'  -- Derivation chain for L1/L2
);

-- Turn: Ephemeral conversation buffer entry
CREATE TABLE IF NOT EXISTS caliber_turn (
    turn_id UUID PRIMARY KEY,
    scope_id UUID NOT NULL REFERENCES caliber_scope(scope_id),
    sequence INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    token_count INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tool_calls JSONB,
    tool_results JSONB,
    metadata JSONB,
    UNIQUE (scope_id, sequence)
);


-- ============================================================================
-- AGENT TABLES
-- ============================================================================

-- Agent: Multi-agent coordination entity
CREATE TABLE IF NOT EXISTS caliber_agent (
    agent_id UUID PRIMARY KEY,
    agent_type TEXT NOT NULL,
    capabilities TEXT[] NOT NULL DEFAULT '{}',
    memory_access JSONB NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'active', 'blocked', 'failed')),
    current_trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    current_scope_id UUID REFERENCES caliber_scope(scope_id),
    can_delegate_to TEXT[] NOT NULL DEFAULT '{}',
    reports_to UUID REFERENCES caliber_agent(agent_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_heartbeat TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Lock: Distributed lock for resource coordination
CREATE TABLE IF NOT EXISTS caliber_lock (
    lock_id UUID PRIMARY KEY,
    resource_type TEXT NOT NULL,
    resource_id UUID NOT NULL,
    holder_agent_id UUID NOT NULL REFERENCES caliber_agent(agent_id),
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    mode TEXT NOT NULL DEFAULT 'exclusive' CHECK (mode IN ('exclusive', 'shared')),
    UNIQUE (resource_type, resource_id, mode)
);

-- Message: Inter-agent communication
CREATE TABLE IF NOT EXISTS caliber_message (
    message_id UUID PRIMARY KEY,
    from_agent_id UUID NOT NULL REFERENCES caliber_agent(agent_id),
    to_agent_id UUID REFERENCES caliber_agent(agent_id),
    to_agent_type TEXT,
    message_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    scope_id UUID REFERENCES caliber_scope(scope_id),
    artifact_ids UUID[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at TIMESTAMPTZ,
    acknowledged_at TIMESTAMPTZ,
    priority TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('low', 'normal', 'high', 'critical')),
    expires_at TIMESTAMPTZ
);

-- Delegation: Task delegation between agents
CREATE TABLE IF NOT EXISTS caliber_delegation (
    delegation_id UUID PRIMARY KEY,
    delegator_agent_id UUID NOT NULL REFERENCES caliber_agent(agent_id),
    delegatee_agent_id UUID REFERENCES caliber_agent(agent_id),
    delegatee_agent_type TEXT,
    task_description TEXT NOT NULL,
    parent_trajectory_id UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id),
    child_trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    shared_artifacts UUID[] NOT NULL DEFAULT '{}',
    shared_notes UUID[] NOT NULL DEFAULT '{}',
    additional_context TEXT,
    constraints TEXT NOT NULL DEFAULT '{}',
    deadline TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'rejected', 'in_progress', 'completed', 'failed')),
    result JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

-- Conflict: Detected conflicts between items
CREATE TABLE IF NOT EXISTS caliber_conflict (
    conflict_id UUID PRIMARY KEY,
    conflict_type TEXT NOT NULL,
    item_a_type TEXT NOT NULL,
    item_a_id UUID NOT NULL,
    item_b_type TEXT NOT NULL,
    item_b_id UUID NOT NULL,
    agent_a_id UUID REFERENCES caliber_agent(agent_id),
    agent_b_id UUID REFERENCES caliber_agent(agent_id),
    trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    status TEXT NOT NULL DEFAULT 'detected' CHECK (status IN ('detected', 'resolving', 'resolved', 'escalated')),
    resolution JSONB,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

-- Handoff: Agent handoff records
CREATE TABLE IF NOT EXISTS caliber_handoff (
    handoff_id UUID PRIMARY KEY,
    from_agent_id UUID NOT NULL REFERENCES caliber_agent(agent_id),
    to_agent_id UUID REFERENCES caliber_agent(agent_id),
    to_agent_type TEXT,
    trajectory_id UUID NOT NULL REFERENCES caliber_trajectory(trajectory_id),
    scope_id UUID NOT NULL REFERENCES caliber_scope(scope_id),
    context_snapshot_id UUID NOT NULL,
    handoff_notes TEXT NOT NULL DEFAULT '',
    next_steps TEXT[] NOT NULL DEFAULT '{}',
    blockers TEXT[] NOT NULL DEFAULT '{}',
    open_questions TEXT[] NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'initiated' CHECK (status IN ('initiated', 'accepted', 'completed', 'rejected')),
    initiated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    reason TEXT NOT NULL
);

-- Region: Memory region access control
CREATE TABLE IF NOT EXISTS caliber_region (
    region_id UUID PRIMARY KEY,
    region_type TEXT NOT NULL CHECK (region_type IN ('private', 'team', 'public', 'collaborative')),
    owner_agent_id UUID NOT NULL REFERENCES caliber_agent(agent_id),
    team_id UUID,
    readers UUID[] NOT NULL DEFAULT '{}',
    writers UUID[] NOT NULL DEFAULT '{}',
    require_lock BOOLEAN NOT NULL DEFAULT FALSE,
    conflict_resolution TEXT NOT NULL DEFAULT 'last_write_wins' CHECK (conflict_resolution IN ('last_write_wins', 'highest_confidence', 'escalate')),
    version_tracking BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- ============================================================================
-- BATTLE INTEL FEATURE TABLES
-- ============================================================================

-- Edge: Graph relationships between entities (Battle Intel Feature 1)
-- Supports both binary edges (2 participants) and hyperedges (N participants)
CREATE TABLE IF NOT EXISTS caliber_edge (
    edge_id UUID PRIMARY KEY,
    edge_type TEXT NOT NULL CHECK (edge_type IN (
        'supports', 'contradicts', 'supersedes', 'derivedfrom', 'relatesto',
        'temporal', 'causal', 'synthesizedfrom', 'grouped', 'compared'
    )),
    participants JSONB NOT NULL,  -- Array of {entity_type, id, role}
    weight REAL,  -- Relationship strength [0.0, 1.0]
    trajectory_id UUID REFERENCES caliber_trajectory(trajectory_id),
    -- Provenance (reusing pattern from artifacts)
    source_turn INTEGER NOT NULL DEFAULT 0,
    extraction_method TEXT NOT NULL DEFAULT 'explicit' CHECK (extraction_method IN ('explicit', 'inferred', 'userprovided')),
    confidence REAL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

-- Evolution Snapshot: DSL config benchmarking (Battle Intel Feature 3)
CREATE TABLE IF NOT EXISTS caliber_evolution_snapshot (
    snapshot_id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    config_hash BYTEA NOT NULL,
    config_source TEXT NOT NULL,
    phase TEXT NOT NULL DEFAULT 'online' CHECK (phase IN ('online', 'frozen', 'evolving')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Metrics (populated after benchmark)
    retrieval_accuracy REAL,
    token_efficiency REAL,
    latency_p50_ms BIGINT,
    latency_p99_ms BIGINT,
    cost_estimate REAL,
    benchmark_queries INTEGER,
    metadata JSONB
);

-- Summarization Policy: Auto-abstraction rules (Battle Intel Feature 4)
CREATE TABLE IF NOT EXISTS caliber_summarization_policy (
    policy_id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    triggers JSONB NOT NULL,  -- Array of trigger definitions
    target_level TEXT NOT NULL CHECK (target_level IN ('raw', 'summary', 'principle')),
    source_level TEXT NOT NULL CHECK (source_level IN ('raw', 'summary', 'principle')),
    max_sources INTEGER NOT NULL DEFAULT 10,
    create_edges BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);


-- ============================================================================
-- INDEXES
-- ============================================================================

-- Trajectory indexes
CREATE INDEX IF NOT EXISTS idx_trajectory_status ON caliber_trajectory(status);
CREATE INDEX IF NOT EXISTS idx_trajectory_agent ON caliber_trajectory(agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_trajectory_parent ON caliber_trajectory(parent_trajectory_id) WHERE parent_trajectory_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_trajectory_created ON caliber_trajectory(created_at);

-- Scope indexes
CREATE INDEX IF NOT EXISTS idx_scope_trajectory ON caliber_scope(trajectory_id);
CREATE INDEX IF NOT EXISTS idx_scope_active ON caliber_scope(trajectory_id, is_active) WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_scope_created ON caliber_scope(created_at);

-- Artifact indexes
CREATE INDEX IF NOT EXISTS idx_artifact_trajectory ON caliber_artifact(trajectory_id);
CREATE INDEX IF NOT EXISTS idx_artifact_scope ON caliber_artifact(scope_id);
CREATE INDEX IF NOT EXISTS idx_artifact_type ON caliber_artifact(trajectory_id, artifact_type);
CREATE INDEX IF NOT EXISTS idx_artifact_hash ON caliber_artifact USING hash(content_hash);
CREATE INDEX IF NOT EXISTS idx_artifact_created ON caliber_artifact(created_at);
-- HNSW index for vector similarity search (requires pgvector)
-- CREATE INDEX IF NOT EXISTS idx_artifact_embedding ON caliber_artifact USING hnsw(embedding vector_cosine_ops);

-- Note indexes
CREATE INDEX IF NOT EXISTS idx_note_type ON caliber_note(note_type);
CREATE INDEX IF NOT EXISTS idx_note_hash ON caliber_note USING hash(content_hash);
CREATE INDEX IF NOT EXISTS idx_note_accessed ON caliber_note(accessed_at);
CREATE INDEX IF NOT EXISTS idx_note_source_trajectories ON caliber_note USING gin(source_trajectory_ids);
-- Battle Intel Feature 2: Abstraction level indexes
CREATE INDEX IF NOT EXISTS idx_note_abstraction ON caliber_note(abstraction_level);
CREATE INDEX IF NOT EXISTS idx_note_source_notes ON caliber_note USING gin(source_note_ids);
-- HNSW index for vector similarity search (requires pgvector)
-- CREATE INDEX IF NOT EXISTS idx_note_embedding ON caliber_note USING hnsw(embedding vector_cosine_ops);

-- Turn indexes
CREATE INDEX IF NOT EXISTS idx_turn_scope ON caliber_turn(scope_id);
CREATE INDEX IF NOT EXISTS idx_turn_scope_seq ON caliber_turn(scope_id, sequence);

-- Agent indexes
CREATE INDEX IF NOT EXISTS idx_agent_type ON caliber_agent(agent_type);
CREATE INDEX IF NOT EXISTS idx_agent_status ON caliber_agent(status);
CREATE INDEX IF NOT EXISTS idx_agent_heartbeat ON caliber_agent(last_heartbeat);

-- Lock indexes
CREATE INDEX IF NOT EXISTS idx_lock_resource ON caliber_lock(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_lock_holder ON caliber_lock(holder_agent_id);
CREATE INDEX IF NOT EXISTS idx_lock_expires ON caliber_lock(expires_at);

-- Message indexes
CREATE INDEX IF NOT EXISTS idx_message_to_agent ON caliber_message(to_agent_id) WHERE to_agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_message_to_type ON caliber_message(to_agent_type) WHERE to_agent_type IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_message_pending ON caliber_message(to_agent_id, delivered_at) WHERE delivered_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_message_created ON caliber_message(created_at);

-- Delegation indexes
CREATE INDEX IF NOT EXISTS idx_delegation_delegator ON caliber_delegation(delegator_agent_id);
CREATE INDEX IF NOT EXISTS idx_delegation_delegatee ON caliber_delegation(delegatee_agent_id) WHERE delegatee_agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_delegation_status ON caliber_delegation(status);
CREATE INDEX IF NOT EXISTS idx_delegation_pending ON caliber_delegation(delegatee_agent_type, status) WHERE status = 'pending';

-- Conflict indexes
CREATE INDEX IF NOT EXISTS idx_conflict_status ON caliber_conflict(status);
CREATE INDEX IF NOT EXISTS idx_conflict_items ON caliber_conflict(item_a_id, item_b_id);

-- Handoff indexes
CREATE INDEX IF NOT EXISTS idx_handoff_from ON caliber_handoff(from_agent_id);
CREATE INDEX IF NOT EXISTS idx_handoff_to ON caliber_handoff(to_agent_id) WHERE to_agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_handoff_status ON caliber_handoff(status);

-- Region indexes
CREATE INDEX IF NOT EXISTS idx_region_owner ON caliber_region(owner_agent_id);
CREATE INDEX IF NOT EXISTS idx_region_type ON caliber_region(region_type);
CREATE INDEX IF NOT EXISTS idx_region_team ON caliber_region(team_id) WHERE team_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_region_readers ON caliber_region USING gin(readers);
CREATE INDEX IF NOT EXISTS idx_region_writers ON caliber_region USING gin(writers);

-- Edge indexes (Battle Intel Feature 1)
CREATE INDEX IF NOT EXISTS idx_edge_type ON caliber_edge(edge_type);
CREATE INDEX IF NOT EXISTS idx_edge_participants ON caliber_edge USING gin(participants);
CREATE INDEX IF NOT EXISTS idx_edge_trajectory ON caliber_edge(trajectory_id) WHERE trajectory_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_edge_created ON caliber_edge(created_at);

-- Evolution Snapshot indexes (Battle Intel Feature 3)
CREATE INDEX IF NOT EXISTS idx_evolution_phase ON caliber_evolution_snapshot(phase);
CREATE INDEX IF NOT EXISTS idx_evolution_created ON caliber_evolution_snapshot(created_at);

-- Summarization Policy indexes (Battle Intel Feature 4)
CREATE INDEX IF NOT EXISTS idx_summarization_target ON caliber_summarization_policy(target_level);
CREATE INDEX IF NOT EXISTS idx_summarization_source ON caliber_summarization_policy(source_level);


-- ============================================================================
-- TRIGGERS FOR UPDATED_AT
-- ============================================================================

CREATE OR REPLACE FUNCTION caliber_update_timestamp()
RETURNS TRIGGER AS $$
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trajectory_updated_at
    BEFORE UPDATE ON caliber_trajectory
    FOR EACH ROW EXECUTE FUNCTION caliber_update_timestamp();

CREATE TRIGGER artifact_updated_at
    BEFORE UPDATE ON caliber_artifact
    FOR EACH ROW EXECUTE FUNCTION caliber_update_timestamp();

CREATE TRIGGER note_updated_at
    BEFORE UPDATE ON caliber_note
    FOR EACH ROW EXECUTE FUNCTION caliber_update_timestamp();

CREATE TRIGGER tenant_updated_at
    BEFORE UPDATE ON caliber_tenant
    FOR EACH ROW EXECUTE FUNCTION caliber_update_timestamp();


-- ============================================================================
-- NOTIFY CHANNELS FOR MESSAGE PASSING
-- ============================================================================

-- Function to notify agents of new messages
CREATE OR REPLACE FUNCTION caliber_notify_message()
RETURNS TRIGGER AS $$
DECLARE
    channel TEXT;
    payload JSONB;
    -- Determine channel
    IF NEW.to_agent_id IS NOT NULL THEN
        channel := 'caliber_agent_' || NEW.to_agent_id::TEXT;
    ELSIF NEW.to_agent_type IS NOT NULL THEN
        channel := 'caliber_type_' || NEW.to_agent_type;
    ELSE
        channel := 'caliber_broadcast';
    END IF;
    
    -- Build payload
    payload := jsonb_build_object(
        'message_id', NEW.message_id,
        'type', NEW.message_type,
        'priority', NEW.priority,
        'from_agent_id', NEW.from_agent_id
    );
    
    -- Send notification
    PERFORM pg_notify(channel, payload::TEXT);
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER message_notify
    AFTER INSERT ON caliber_message
    FOR EACH ROW EXECUTE FUNCTION caliber_notify_message();


-- ============================================================================
-- CLEANUP FUNCTIONS
-- ============================================================================

-- Clean up expired locks
CREATE OR REPLACE FUNCTION caliber_cleanup_expired_locks()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
    DELETE FROM caliber_lock WHERE expires_at < NOW();
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Clean up expired messages
CREATE OR REPLACE FUNCTION caliber_cleanup_expired_messages()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
    DELETE FROM caliber_message WHERE expires_at IS NOT NULL AND expires_at < NOW();
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;


-- ============================================================================
-- DEBUG VIEWS (Human interface only, not for hot path)
-- ============================================================================

-- View: Active trajectories with scope counts
CREATE OR REPLACE VIEW caliber_v_active_trajectories AS
SELECT 
    t.trajectory_id,
    t.name,
    t.status,
    t.agent_id,
    t.created_at,
    COUNT(s.scope_id) AS scope_count,
    SUM(s.tokens_used) AS total_tokens_used
FROM caliber_trajectory t
LEFT JOIN caliber_scope s ON t.trajectory_id = s.trajectory_id
WHERE t.status = 'active'
GROUP BY t.trajectory_id;

-- View: Agent status overview
CREATE OR REPLACE VIEW caliber_v_agent_status AS
SELECT 
    a.agent_id,
    a.agent_type,
    a.status,
    a.last_heartbeat,
    NOW() - a.last_heartbeat AS time_since_heartbeat,
    COUNT(DISTINCT l.lock_id) AS held_locks,
    COUNT(DISTINCT m.message_id) AS pending_messages
FROM caliber_agent a
LEFT JOIN caliber_lock l ON a.agent_id = l.holder_agent_id AND l.expires_at > NOW()
LEFT JOIN caliber_message m ON a.agent_id = m.to_agent_id AND m.delivered_at IS NULL
GROUP BY a.agent_id;

-- View: Unresolved conflicts
CREATE OR REPLACE VIEW caliber_v_unresolved_conflicts AS
SELECT 
    c.conflict_id,
    c.conflict_type,
    c.item_a_type,
    c.item_a_id,
    c.item_b_type,
    c.item_b_id,
    c.status,
    c.detected_at,
    NOW() - c.detected_at AS age
FROM caliber_conflict c
WHERE c.status IN ('detected', 'resolving')
ORDER BY c.detected_at;

-- View: Pending delegations
CREATE OR REPLACE VIEW caliber_v_pending_delegations AS
SELECT 
    d.delegation_id,
    d.delegator_agent_id,
    d.delegatee_agent_type,
    d.task_description,
    d.deadline,
    d.created_at,
    CASE 
        WHEN d.deadline IS NOT NULL AND d.deadline < NOW() THEN TRUE 
        ELSE FALSE 
    END AS is_overdue
FROM caliber_delegation d
WHERE d.status = 'pending'
ORDER BY d.deadline NULLS LAST, d.created_at;
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
    DELETE FROM caliber_changes WHERE changed_at < NOW() - (p_retention_days || ' days')::INTERVAL;
    GET DIAGNOSTICS v_deleted = ROW_COUNT;
    RETURN v_deleted;
END; $$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION caliber_cleanup_idempotency_keys()
RETURNS INTEGER AS $$
DECLARE v_deleted INTEGER;
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
    -- Set the session variable for RLS policies
    -- 'true' makes it local to the current transaction
    PERFORM set_config('app.tenant_id', p_tenant_id::text, true);
END;
$$ LANGUAGE plpgsql;

-- Function to clear the tenant context (for connection pooling safety)
CREATE OR REPLACE FUNCTION caliber_clear_tenant_context()
RETURNS VOID AS $$
    PERFORM set_config('app.tenant_id', '', true);
END;
$$ LANGUAGE plpgsql;

-- Function to get the current tenant context
CREATE OR REPLACE FUNCTION caliber_get_tenant_context()
RETURNS UUID AS $$
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
