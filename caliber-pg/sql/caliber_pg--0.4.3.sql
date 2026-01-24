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
VALUES (1, 'Initial schema - CALIBER 0.4.3', 'base')
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
BEGIN
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
BEGIN
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
BEGIN
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
BEGIN
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
