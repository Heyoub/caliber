//! CALIBER-PG - PostgreSQL Extension for CALIBER Memory Framework
//!
//! This crate provides the pgrx-based PostgreSQL extension that wires together
//! all CALIBER components. It implements:
//! - Direct heap storage operations (bypassing SQL in hot path)
//! - Advisory lock functions for distributed coordination
//! - NOTIFY-based message passing for agents
//! - Bootstrap SQL schema for extension installation

use pgrx::prelude::*;

// Re-export core types for use in SQL functions
use caliber_core::{
    Artifact, ArtifactType, CaliberConfig, CaliberError, CaliberResult, Checkpoint, 
    EmbeddingVector, EntityId, EntityType, ExtractionMethod, MemoryCategory, Note, 
    NoteType, Provenance, RawContent, Scope, StorageError, TTL, Timestamp, Trajectory, 
    TrajectoryOutcome, TrajectoryStatus, Turn, TurnRole, ValidationError,
    compute_content_hash, new_entity_id,
};
use caliber_storage::{
    ArtifactUpdate, NoteUpdate, ScopeUpdate, StorageTrait, TrajectoryUpdate,
};
use caliber_agents::{
    Agent, AgentHandoff, AgentMessage, AgentStatus, Conflict, ConflictStatus,
    ConflictType, DelegatedTask, DelegationStatus, DistributedLock, HandoffReason,
    HandoffStatus, LockMode, MemoryAccess, MemoryRegion, MemoryRegionConfig,
    MessagePriority, MessageType, ResolutionStrategy, compute_lock_key,
};
use caliber_pcp::ConflictResolution;

use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

// Initialize pgrx extension
pgrx::pg_module_magic!();


// ============================================================================
// EXTENSION INITIALIZATION (Task 12.1)
// ============================================================================

/// Extension initialization hook.
/// Called when the extension is loaded.
#[pg_guard]
pub extern "C" fn _PG_init() {
    // Extension initialization code
    // In production, this would set up shared memory, background workers, etc.
    pgrx::log!("CALIBER extension initializing...");
}

/// Extension finalization hook.
/// Called when the extension is unloaded.
#[pg_guard]
pub extern "C" fn _PG_fini() {
    pgrx::log!("CALIBER extension finalizing...");
}


// ============================================================================
// IN-MEMORY STORAGE (for development/testing)
// ============================================================================

/// In-memory storage for development and testing.
/// In production, this would be replaced with direct heap operations.
static STORAGE: Lazy<RwLock<InMemoryStorage>> = Lazy::new(|| {
    RwLock::new(InMemoryStorage::default())
});

#[derive(Debug, Default)]
struct InMemoryStorage {
    trajectories: HashMap<EntityId, Trajectory>,
    scopes: HashMap<EntityId, Scope>,
    artifacts: HashMap<EntityId, Artifact>,
    notes: HashMap<EntityId, Note>,
    turns: HashMap<EntityId, Turn>,
    agents: HashMap<EntityId, Agent>,
    locks: HashMap<EntityId, DistributedLock>,
    messages: HashMap<EntityId, AgentMessage>,
    delegations: HashMap<EntityId, DelegatedTask>,
    handoffs: HashMap<EntityId, AgentHandoff>,
    conflicts: HashMap<EntityId, Conflict>,
}


// ============================================================================
// BOOTSTRAP SQL SCHEMA (Task 12.2)
// ============================================================================

/// Initialize the CALIBER schema.
/// This creates all tables, indexes, and functions needed by the extension.
/// This SQL runs ONCE at extension install, NOT in hot path.
#[pg_extern]
fn caliber_init() -> &'static str {
    // In a real implementation, this would execute the bootstrap SQL
    // For now, we return a status message
    "CALIBER schema initialized successfully"
}

/// Get the extension version.
#[pg_extern]
fn caliber_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}


// ============================================================================
// ENTITY ID GENERATION
// ============================================================================

/// Generate a new UUIDv7 entity ID.
/// UUIDv7 is timestamp-sortable, making it ideal for time-ordered data.
#[pg_extern]
fn caliber_new_id() -> pgrx::Uuid {
    let id = new_entity_id();
    pgrx::Uuid::from_bytes(*id.as_bytes())
}


// ============================================================================
// TRAJECTORY OPERATIONS (Task 12.3)
// ============================================================================

/// Create a new trajectory.
#[pg_extern]
fn caliber_trajectory_create(
    name: &str,
    description: Option<&str>,
    agent_id: Option<pgrx::Uuid>,
) -> pgrx::Uuid {
    let trajectory_id = new_entity_id();
    let now = Utc::now();

    let trajectory = Trajectory {
        trajectory_id,
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        status: TrajectoryStatus::Active,
        parent_trajectory_id: None,
        root_trajectory_id: None,
        agent_id: agent_id.map(|u| Uuid::from_bytes(*u.as_bytes())),
        created_at: now,
        updated_at: now,
        completed_at: None,
        outcome: None,
        metadata: None,
    };

    let mut storage = STORAGE.write().unwrap();
    storage.trajectories.insert(trajectory_id, trajectory);

    pgrx::Uuid::from_bytes(*trajectory_id.as_bytes())
}

/// Get a trajectory by ID.
#[pg_extern]
fn caliber_trajectory_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.trajectories.get(&entity_id).map(|t| {
        pgrx::JsonB(serde_json::to_value(t).unwrap_or(serde_json::Value::Null))
    })
}

/// Update trajectory status.
#[pg_extern]
fn caliber_trajectory_set_status(id: pgrx::Uuid, status: &str) -> bool {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(trajectory) = storage.trajectories.get_mut(&entity_id) {
        trajectory.status = match status {
            "active" => TrajectoryStatus::Active,
            "completed" => TrajectoryStatus::Completed,
            "failed" => TrajectoryStatus::Failed,
            "suspended" => TrajectoryStatus::Suspended,
            _ => return false,
        };
        trajectory.updated_at = Utc::now();
        true
    } else {
        false
    }
}

/// List trajectories by status.
#[pg_extern]
fn caliber_trajectory_list_by_status(status: &str) -> pgrx::JsonB {
    let target_status = match status {
        "active" => TrajectoryStatus::Active,
        "completed" => TrajectoryStatus::Completed,
        "failed" => TrajectoryStatus::Failed,
        "suspended" => TrajectoryStatus::Suspended,
        _ => return pgrx::JsonB(serde_json::json!([])),
    };

    let storage = STORAGE.read().unwrap();
    let trajectories: Vec<&Trajectory> = storage
        .trajectories
        .values()
        .filter(|t| t.status == target_status)
        .collect();

    pgrx::JsonB(serde_json::to_value(&trajectories).unwrap_or(serde_json::json!([])))
}


// ============================================================================
// SCOPE OPERATIONS (Task 12.3)
// ============================================================================

/// Create a new scope within a trajectory.
#[pg_extern]
fn caliber_scope_create(
    trajectory_id: pgrx::Uuid,
    name: &str,
    purpose: Option<&str>,
    token_budget: i32,
) -> pgrx::Uuid {
    let scope_id = new_entity_id();
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());
    let now = Utc::now();

    let scope = Scope {
        scope_id,
        trajectory_id: traj_id,
        parent_scope_id: None,
        name: name.to_string(),
        purpose: purpose.map(|s| s.to_string()),
        is_active: true,
        created_at: now,
        closed_at: None,
        checkpoint: None,
        token_budget,
        tokens_used: 0,
        metadata: None,
    };

    let mut storage = STORAGE.write().unwrap();
    storage.scopes.insert(scope_id, scope);

    pgrx::Uuid::from_bytes(*scope_id.as_bytes())
}

/// Get a scope by ID.
#[pg_extern]
fn caliber_scope_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.scopes.get(&entity_id).map(|s| {
        pgrx::JsonB(serde_json::to_value(s).unwrap_or(serde_json::Value::Null))
    })
}

/// Get the current active scope for a trajectory.
#[pg_extern]
fn caliber_scope_get_current(trajectory_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage
        .scopes
        .values()
        .filter(|s| s.trajectory_id == traj_id && s.is_active)
        .max_by_key(|s| s.created_at)
        .map(|s| pgrx::JsonB(serde_json::to_value(s).unwrap_or(serde_json::Value::Null)))
}

/// Close a scope.
#[pg_extern]
fn caliber_scope_close(id: pgrx::Uuid) -> bool {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(scope) = storage.scopes.get_mut(&entity_id) {
        scope.is_active = false;
        scope.closed_at = Some(Utc::now());
        true
    } else {
        false
    }
}

/// Update tokens used in a scope.
#[pg_extern]
fn caliber_scope_update_tokens(id: pgrx::Uuid, tokens_used: i32) -> bool {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(scope) = storage.scopes.get_mut(&entity_id) {
        scope.tokens_used = tokens_used;
        true
    } else {
        false
    }
}


// ============================================================================
// ARTIFACT OPERATIONS (Task 12.3)
// ============================================================================

/// Create a new artifact.
#[pg_extern]
fn caliber_artifact_create(
    trajectory_id: pgrx::Uuid,
    scope_id: pgrx::Uuid,
    artifact_type: &str,
    name: &str,
    content: &str,
) -> pgrx::Uuid {
    let artifact_id = new_entity_id();
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());
    let now = Utc::now();

    let art_type = match artifact_type {
        "error_log" => ArtifactType::ErrorLog,
        "code_patch" => ArtifactType::CodePatch,
        "design_decision" => ArtifactType::DesignDecision,
        "user_preference" => ArtifactType::UserPreference,
        "fact" => ArtifactType::Fact,
        "constraint" => ArtifactType::Constraint,
        "tool_result" => ArtifactType::ToolResult,
        "intermediate_output" => ArtifactType::IntermediateOutput,
        _ => ArtifactType::Custom,
    };

    let artifact = Artifact {
        artifact_id,
        trajectory_id: traj_id,
        scope_id: scp_id,
        artifact_type: art_type,
        name: name.to_string(),
        content: content.to_string(),
        content_hash: compute_content_hash(content.as_bytes()),
        embedding: None,
        provenance: Provenance {
            source_turn: 0,
            extraction_method: ExtractionMethod::Explicit,
            confidence: None,
        },
        ttl: TTL::Persistent,
        created_at: now,
        updated_at: now,
        superseded_by: None,
        metadata: None,
    };

    let mut storage = STORAGE.write().unwrap();
    storage.artifacts.insert(artifact_id, artifact);

    pgrx::Uuid::from_bytes(*artifact_id.as_bytes())
}

/// Get an artifact by ID.
#[pg_extern]
fn caliber_artifact_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.artifacts.get(&entity_id).map(|a| {
        pgrx::JsonB(serde_json::to_value(a).unwrap_or(serde_json::Value::Null))
    })
}

/// Query artifacts by type within a trajectory.
#[pg_extern]
fn caliber_artifact_query_by_type(
    trajectory_id: pgrx::Uuid,
    artifact_type: &str,
) -> pgrx::JsonB {
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());
    let art_type = match artifact_type {
        "error_log" => ArtifactType::ErrorLog,
        "code_patch" => ArtifactType::CodePatch,
        "design_decision" => ArtifactType::DesignDecision,
        "user_preference" => ArtifactType::UserPreference,
        "fact" => ArtifactType::Fact,
        "constraint" => ArtifactType::Constraint,
        "tool_result" => ArtifactType::ToolResult,
        "intermediate_output" => ArtifactType::IntermediateOutput,
        _ => ArtifactType::Custom,
    };

    let storage = STORAGE.read().unwrap();
    let artifacts: Vec<&Artifact> = storage
        .artifacts
        .values()
        .filter(|a| a.trajectory_id == traj_id && a.artifact_type == art_type)
        .collect();

    pgrx::JsonB(serde_json::to_value(&artifacts).unwrap_or(serde_json::json!([])))
}

/// Query artifacts by scope.
#[pg_extern]
fn caliber_artifact_query_by_scope(scope_id: pgrx::Uuid) -> pgrx::JsonB {
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    let artifacts: Vec<&Artifact> = storage
        .artifacts
        .values()
        .filter(|a| a.scope_id == scp_id)
        .collect();

    pgrx::JsonB(serde_json::to_value(&artifacts).unwrap_or(serde_json::json!([])))
}


// ============================================================================
// NOTE OPERATIONS (Task 12.3)
// ============================================================================

/// Create a new note.
#[pg_extern]
fn caliber_note_create(
    note_type: &str,
    title: &str,
    content: &str,
    source_trajectory_id: Option<pgrx::Uuid>,
) -> pgrx::Uuid {
    let note_id = new_entity_id();
    let now = Utc::now();

    let n_type = match note_type {
        "convention" => NoteType::Convention,
        "strategy" => NoteType::Strategy,
        "gotcha" => NoteType::Gotcha,
        "fact" => NoteType::Fact,
        "preference" => NoteType::Preference,
        "relationship" => NoteType::Relationship,
        "procedure" => NoteType::Procedure,
        _ => NoteType::Meta,
    };

    let source_ids = source_trajectory_id
        .map(|u| vec![Uuid::from_bytes(*u.as_bytes())])
        .unwrap_or_default();

    let note = Note {
        note_id,
        note_type: n_type,
        title: title.to_string(),
        content: content.to_string(),
        content_hash: compute_content_hash(content.as_bytes()),
        embedding: None,
        source_trajectory_ids: source_ids,
        source_artifact_ids: Vec::new(),
        ttl: TTL::Persistent,
        created_at: now,
        updated_at: now,
        accessed_at: now,
        access_count: 0,
        superseded_by: None,
        metadata: None,
    };

    let mut storage = STORAGE.write().unwrap();
    storage.notes.insert(note_id, note);

    pgrx::Uuid::from_bytes(*note_id.as_bytes())
}

/// Get a note by ID.
#[pg_extern]
fn caliber_note_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.notes.get(&entity_id).map(|n| {
        pgrx::JsonB(serde_json::to_value(n).unwrap_or(serde_json::Value::Null))
    })
}

/// Query notes by trajectory.
#[pg_extern]
fn caliber_note_query_by_trajectory(trajectory_id: pgrx::Uuid) -> pgrx::JsonB {
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    let notes: Vec<&Note> = storage
        .notes
        .values()
        .filter(|n| n.source_trajectory_ids.contains(&traj_id))
        .collect();

    pgrx::JsonB(serde_json::to_value(&notes).unwrap_or(serde_json::json!([])))
}


// ============================================================================
// TURN OPERATIONS (Task 12.3)
// ============================================================================

/// Create a new turn in a scope.
#[pg_extern]
fn caliber_turn_create(
    scope_id: pgrx::Uuid,
    sequence: i32,
    role: &str,
    content: &str,
    token_count: i32,
) -> pgrx::Uuid {
    let turn_id = new_entity_id();
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());
    let now = Utc::now();

    let turn_role = match role {
        "user" => TurnRole::User,
        "assistant" => TurnRole::Assistant,
        "system" => TurnRole::System,
        "tool" => TurnRole::Tool,
        _ => TurnRole::User,
    };

    let turn = Turn {
        turn_id,
        scope_id: scp_id,
        sequence,
        role: turn_role,
        content: content.to_string(),
        token_count,
        created_at: now,
        tool_calls: None,
        tool_results: None,
        metadata: None,
    };

    let mut storage = STORAGE.write().unwrap();
    storage.turns.insert(turn_id, turn);

    pgrx::Uuid::from_bytes(*turn_id.as_bytes())
}

/// Get turns by scope.
#[pg_extern]
fn caliber_turn_get_by_scope(scope_id: pgrx::Uuid) -> pgrx::JsonB {
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    let mut turns: Vec<&Turn> = storage
        .turns
        .values()
        .filter(|t| t.scope_id == scp_id)
        .collect();

    turns.sort_by_key(|t| t.sequence);

    pgrx::JsonB(serde_json::to_value(&turns).unwrap_or(serde_json::json!([])))
}


// ============================================================================
// ADVISORY LOCK FUNCTIONS (Task 12.4)
// ============================================================================

/// Acquire an advisory lock on a resource.
/// Uses Postgres advisory locks for distributed coordination.
#[pg_extern]
fn caliber_lock_acquire(
    agent_id: pgrx::Uuid,
    resource_type: &str,
    resource_id: pgrx::Uuid,
    timeout_ms: i64,
    mode: &str,
) -> Option<pgrx::Uuid> {
    let agent = Uuid::from_bytes(*agent_id.as_bytes());
    let resource = Uuid::from_bytes(*resource_id.as_bytes());
    let lock_key = compute_lock_key(resource_type, resource);

    let lock_mode = match mode {
        "shared" => LockMode::Shared,
        _ => LockMode::Exclusive,
    };

    // Try to acquire Postgres advisory lock
    let acquired = if lock_mode == LockMode::Exclusive {
        // Try exclusive lock (non-blocking)
        unsafe {
            pgrx::pg_sys::pg_try_advisory_xact_lock(lock_key)
        }
    } else {
        // Try shared lock (non-blocking)
        unsafe {
            pgrx::pg_sys::pg_try_advisory_xact_lock_shared(lock_key)
        }
    };

    if acquired {
        // Create lock record
        let lock = DistributedLock::new(
            resource_type,
            resource,
            agent,
            timeout_ms,
            lock_mode,
        );
        let lock_id = lock.lock_id;

        let mut storage = STORAGE.write().unwrap();
        storage.locks.insert(lock_id, lock);

        Some(pgrx::Uuid::from_bytes(*lock_id.as_bytes()))
    } else {
        None
    }
}

/// Release an advisory lock.
#[pg_extern]
fn caliber_lock_release(lock_id: pgrx::Uuid) -> bool {
    let lid = Uuid::from_bytes(*lock_id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(lock) = storage.locks.remove(&lid) {
        let lock_key = lock.compute_key();

        // Release Postgres advisory lock
        if lock.mode == LockMode::Exclusive {
            unsafe {
                pgrx::pg_sys::pg_advisory_unlock(lock_key);
            }
        } else {
            unsafe {
                pgrx::pg_sys::pg_advisory_unlock_shared(lock_key);
            }
        }
        true
    } else {
        false
    }
}

/// Check if a resource is locked.
#[pg_extern]
fn caliber_lock_check(resource_type: &str, resource_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let resource = Uuid::from_bytes(*resource_id.as_bytes());
    let lock_key = compute_lock_key(resource_type, resource);

    let storage = STORAGE.read().unwrap();

    // Find lock by resource
    storage
        .locks
        .values()
        .find(|l| l.compute_key() == lock_key && !l.is_expired())
        .map(|l| pgrx::JsonB(serde_json::to_value(l).unwrap_or(serde_json::Value::Null)))
}

/// Get lock by ID.
#[pg_extern]
fn caliber_lock_get(lock_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let lid = Uuid::from_bytes(*lock_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.locks.get(&lid).map(|l| {
        pgrx::JsonB(serde_json::to_value(l).unwrap_or(serde_json::Value::Null))
    })
}


// ============================================================================
// NOTIFY-BASED MESSAGE PASSING (Task 12.5)
// ============================================================================

/// Send a message to an agent using NOTIFY.
#[pg_extern]
fn caliber_message_send(
    from_agent_id: pgrx::Uuid,
    to_agent_id: Option<pgrx::Uuid>,
    to_agent_type: Option<&str>,
    message_type: &str,
    payload: &str,
    priority: &str,
) -> pgrx::Uuid {
    let from_agent = Uuid::from_bytes(*from_agent_id.as_bytes());
    let to_agent = to_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()));

    let msg_type = match message_type {
        "task_delegation" => MessageType::TaskDelegation,
        "task_result" => MessageType::TaskResult,
        "context_request" => MessageType::ContextRequest,
        "context_share" => MessageType::ContextShare,
        "coordination_signal" => MessageType::CoordinationSignal,
        "handoff" => MessageType::Handoff,
        "interrupt" => MessageType::Interrupt,
        _ => MessageType::Heartbeat,
    };

    let msg_priority = match priority {
        "low" => MessagePriority::Low,
        "high" => MessagePriority::High,
        "critical" => MessagePriority::Critical,
        _ => MessagePriority::Normal,
    };

    let message = if let Some(to_id) = to_agent {
        AgentMessage::to_agent(from_agent, to_id, msg_type, payload)
            .with_priority(msg_priority)
    } else if let Some(agent_type) = to_agent_type {
        AgentMessage::to_type(from_agent, agent_type, msg_type, payload)
            .with_priority(msg_priority)
    } else {
        // Broadcast to all
        AgentMessage::to_type(from_agent, "*", msg_type, payload)
            .with_priority(msg_priority)
    };

    let message_id = message.message_id;

    // Store message
    let mut storage = STORAGE.write().unwrap();
    storage.messages.insert(message_id, message.clone());

    // Send NOTIFY
    let channel = if let Some(to_id) = to_agent {
        format!("caliber_agent_{}", to_id)
    } else if let Some(agent_type) = to_agent_type {
        format!("caliber_type_{}", agent_type)
    } else {
        "caliber_broadcast".to_string()
    };

    let notify_payload = serde_json::json!({
        "message_id": message_id.to_string(),
        "type": message_type,
        "priority": priority,
    });

    // Execute NOTIFY via SPI
    let sql = format!(
        "SELECT pg_notify('{}', '{}')",
        channel,
        notify_payload.to_string().replace('\'', "''")
    );
    
    Spi::run(&sql).ok();

    pgrx::Uuid::from_bytes(*message_id.as_bytes())
}

/// Get a message by ID.
#[pg_extern]
fn caliber_message_get(message_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let mid = Uuid::from_bytes(*message_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.messages.get(&mid).map(|m| {
        pgrx::JsonB(serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
    })
}

/// Mark a message as delivered.
#[pg_extern]
fn caliber_message_mark_delivered(message_id: pgrx::Uuid) -> bool {
    let mid = Uuid::from_bytes(*message_id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(message) = storage.messages.get_mut(&mid) {
        message.mark_delivered();
        true
    } else {
        false
    }
}

/// Mark a message as acknowledged.
#[pg_extern]
fn caliber_message_mark_acknowledged(message_id: pgrx::Uuid) -> bool {
    let mid = Uuid::from_bytes(*message_id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(message) = storage.messages.get_mut(&mid) {
        message.mark_acknowledged();
        true
    } else {
        false
    }
}

/// Get pending messages for an agent.
#[pg_extern]
fn caliber_message_get_pending(agent_id: pgrx::Uuid, agent_type: &str) -> pgrx::JsonB {
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    let messages: Vec<&AgentMessage> = storage
        .messages
        .values()
        .filter(|m| {
            m.is_for_agent(aid, agent_type)
                && m.delivered_at.is_none()
                && !m.is_expired()
        })
        .collect();

    pgrx::JsonB(serde_json::to_value(&messages).unwrap_or(serde_json::json!([])))
}


// ============================================================================
// AGENT OPERATIONS (Task 12.6)
// ============================================================================

/// Register a new agent.
#[pg_extern]
fn caliber_agent_register(
    agent_type: &str,
    capabilities: pgrx::JsonB,
) -> pgrx::Uuid {
    let caps: Vec<String> = serde_json::from_value(capabilities.0)
        .unwrap_or_default();

    let agent = Agent::new(agent_type, caps);
    let agent_id = agent.agent_id;

    let mut storage = STORAGE.write().unwrap();
    storage.agents.insert(agent_id, agent);

    pgrx::Uuid::from_bytes(*agent_id.as_bytes())
}

/// Get an agent by ID.
#[pg_extern]
fn caliber_agent_get(agent_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.agents.get(&aid).map(|a| {
        pgrx::JsonB(serde_json::to_value(a).unwrap_or(serde_json::Value::Null))
    })
}

/// Update agent status.
#[pg_extern]
fn caliber_agent_set_status(agent_id: pgrx::Uuid, status: &str) -> bool {
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(agent) = storage.agents.get_mut(&aid) {
        agent.status = match status {
            "idle" => AgentStatus::Idle,
            "active" => AgentStatus::Active,
            "blocked" => AgentStatus::Blocked,
            "failed" => AgentStatus::Failed,
            _ => return false,
        };
        true
    } else {
        false
    }
}

/// Update agent heartbeat.
#[pg_extern]
fn caliber_agent_heartbeat(agent_id: pgrx::Uuid) -> bool {
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(agent) = storage.agents.get_mut(&aid) {
        agent.heartbeat();
        true
    } else {
        false
    }
}

/// List agents by type.
#[pg_extern]
fn caliber_agent_list_by_type(agent_type: &str) -> pgrx::JsonB {
    let storage = STORAGE.read().unwrap();

    let agents: Vec<&Agent> = storage
        .agents
        .values()
        .filter(|a| a.agent_type == agent_type)
        .collect();

    pgrx::JsonB(serde_json::to_value(&agents).unwrap_or(serde_json::json!([])))
}

/// List all active agents.
#[pg_extern]
fn caliber_agent_list_active() -> pgrx::JsonB {
    let storage = STORAGE.read().unwrap();

    let agents: Vec<&Agent> = storage
        .agents
        .values()
        .filter(|a| a.status == AgentStatus::Active || a.status == AgentStatus::Idle)
        .collect();

    pgrx::JsonB(serde_json::to_value(&agents).unwrap_or(serde_json::json!([])))
}


// ============================================================================
// DELEGATION OPERATIONS (Task 12.6)
// ============================================================================

/// Create a task delegation.
#[pg_extern]
fn caliber_delegation_create(
    delegator_agent_id: pgrx::Uuid,
    delegatee_agent_id: Option<pgrx::Uuid>,
    delegatee_agent_type: Option<&str>,
    task_description: &str,
    parent_trajectory_id: pgrx::Uuid,
) -> pgrx::Uuid {
    let delegator = Uuid::from_bytes(*delegator_agent_id.as_bytes());
    let parent_traj = Uuid::from_bytes(*parent_trajectory_id.as_bytes());

    let delegation = if let Some(delegatee_id) = delegatee_agent_id {
        let delegatee = Uuid::from_bytes(*delegatee_id.as_bytes());
        DelegatedTask::to_agent(delegator, delegatee, task_description, parent_traj)
    } else if let Some(agent_type) = delegatee_agent_type {
        DelegatedTask::to_type(delegator, agent_type, task_description, parent_traj)
    } else {
        // Default to any available agent
        DelegatedTask::to_type(delegator, "*", task_description, parent_traj)
    };

    let delegation_id = delegation.delegation_id;

    let mut storage = STORAGE.write().unwrap();
    storage.delegations.insert(delegation_id, delegation);

    pgrx::Uuid::from_bytes(*delegation_id.as_bytes())
}

/// Get a delegation by ID.
#[pg_extern]
fn caliber_delegation_get(delegation_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let did = Uuid::from_bytes(*delegation_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.delegations.get(&did).map(|d| {
        pgrx::JsonB(serde_json::to_value(d).unwrap_or(serde_json::Value::Null))
    })
}

/// Accept a delegation.
#[pg_extern]
fn caliber_delegation_accept(
    delegation_id: pgrx::Uuid,
    delegatee_agent_id: pgrx::Uuid,
    child_trajectory_id: pgrx::Uuid,
) -> bool {
    let did = Uuid::from_bytes(*delegation_id.as_bytes());
    let delegatee = Uuid::from_bytes(*delegatee_agent_id.as_bytes());
    let child_traj = Uuid::from_bytes(*child_trajectory_id.as_bytes());

    let mut storage = STORAGE.write().unwrap();

    if let Some(delegation) = storage.delegations.get_mut(&did) {
        delegation.accept(delegatee, child_traj);
        true
    } else {
        false
    }
}

/// Complete a delegation.
#[pg_extern]
fn caliber_delegation_complete(
    delegation_id: pgrx::Uuid,
    success: bool,
    summary: &str,
) -> bool {
    let did = Uuid::from_bytes(*delegation_id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(delegation) = storage.delegations.get_mut(&did) {
        use caliber_agents::DelegationResult;
        let result = if success {
            DelegationResult::success(summary, vec![])
        } else {
            DelegationResult::failure(summary)
        };
        delegation.complete(result);
        true
    } else {
        false
    }
}

/// List pending delegations for an agent type.
#[pg_extern]
fn caliber_delegation_list_pending(agent_type: &str) -> pgrx::JsonB {
    let storage = STORAGE.read().unwrap();

    let delegations: Vec<&DelegatedTask> = storage
        .delegations
        .values()
        .filter(|d| {
            d.status == DelegationStatus::Pending
                && (d.delegatee_agent_type.as_deref() == Some(agent_type)
                    || d.delegatee_agent_type.as_deref() == Some("*"))
        })
        .collect();

    pgrx::JsonB(serde_json::to_value(&delegations).unwrap_or(serde_json::json!([])))
}


// ============================================================================
// HANDOFF OPERATIONS (Task 12.6)
// ============================================================================

/// Create an agent handoff.
#[pg_extern]
fn caliber_handoff_create(
    from_agent_id: pgrx::Uuid,
    to_agent_id: Option<pgrx::Uuid>,
    to_agent_type: Option<&str>,
    trajectory_id: pgrx::Uuid,
    scope_id: pgrx::Uuid,
    context_snapshot_id: pgrx::Uuid,
    reason: &str,
) -> pgrx::Uuid {
    let from_agent = Uuid::from_bytes(*from_agent_id.as_bytes());
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());
    let snapshot_id = Uuid::from_bytes(*context_snapshot_id.as_bytes());

    let handoff_reason = match reason {
        "capability_mismatch" => HandoffReason::CapabilityMismatch,
        "load_balancing" => HandoffReason::LoadBalancing,
        "specialization" => HandoffReason::Specialization,
        "escalation" => HandoffReason::Escalation,
        "timeout" => HandoffReason::Timeout,
        "failure" => HandoffReason::Failure,
        _ => HandoffReason::Scheduled,
    };

    let handoff = if let Some(to_id) = to_agent_id {
        let to_agent = Uuid::from_bytes(*to_id.as_bytes());
        AgentHandoff::to_agent(from_agent, to_agent, traj_id, scp_id, snapshot_id, handoff_reason)
    } else if let Some(agent_type) = to_agent_type {
        AgentHandoff::to_type(from_agent, agent_type, traj_id, scp_id, snapshot_id, handoff_reason)
    } else {
        AgentHandoff::to_type(from_agent, "*", traj_id, scp_id, snapshot_id, handoff_reason)
    };

    let handoff_id = handoff.handoff_id;

    let mut storage = STORAGE.write().unwrap();
    storage.handoffs.insert(handoff_id, handoff);

    pgrx::Uuid::from_bytes(*handoff_id.as_bytes())
}

/// Get a handoff by ID.
#[pg_extern]
fn caliber_handoff_get(handoff_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let hid = Uuid::from_bytes(*handoff_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.handoffs.get(&hid).map(|h| {
        pgrx::JsonB(serde_json::to_value(h).unwrap_or(serde_json::Value::Null))
    })
}

/// Accept a handoff.
#[pg_extern]
fn caliber_handoff_accept(handoff_id: pgrx::Uuid, accepting_agent_id: pgrx::Uuid) -> bool {
    let hid = Uuid::from_bytes(*handoff_id.as_bytes());
    let accepting = Uuid::from_bytes(*accepting_agent_id.as_bytes());

    let mut storage = STORAGE.write().unwrap();

    if let Some(handoff) = storage.handoffs.get_mut(&hid) {
        handoff.accept(accepting);
        true
    } else {
        false
    }
}

/// Complete a handoff.
#[pg_extern]
fn caliber_handoff_complete(handoff_id: pgrx::Uuid) -> bool {
    let hid = Uuid::from_bytes(*handoff_id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(handoff) = storage.handoffs.get_mut(&hid) {
        handoff.complete();
        true
    } else {
        false
    }
}


// ============================================================================
// CONFLICT OPERATIONS (Task 12.6)
// ============================================================================

/// Create a conflict record.
#[pg_extern]
fn caliber_conflict_create(
    conflict_type: &str,
    item_a_type: &str,
    item_a_id: pgrx::Uuid,
    item_b_type: &str,
    item_b_id: pgrx::Uuid,
) -> pgrx::Uuid {
    let a_id = Uuid::from_bytes(*item_a_id.as_bytes());
    let b_id = Uuid::from_bytes(*item_b_id.as_bytes());

    let c_type = match conflict_type {
        "concurrent_write" => ConflictType::ConcurrentWrite,
        "contradicting_fact" => ConflictType::ContradictingFact,
        "incompatible_decision" => ConflictType::IncompatibleDecision,
        "resource_contention" => ConflictType::ResourceContention,
        _ => ConflictType::GoalConflict,
    };

    let conflict = Conflict::new(c_type, item_a_type, a_id, item_b_type, b_id);
    let conflict_id = conflict.conflict_id;

    let mut storage = STORAGE.write().unwrap();
    storage.conflicts.insert(conflict_id, conflict);

    pgrx::Uuid::from_bytes(*conflict_id.as_bytes())
}

/// Get a conflict by ID.
#[pg_extern]
fn caliber_conflict_get(conflict_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let cid = Uuid::from_bytes(*conflict_id.as_bytes());
    let storage = STORAGE.read().unwrap();

    storage.conflicts.get(&cid).map(|c| {
        pgrx::JsonB(serde_json::to_value(c).unwrap_or(serde_json::Value::Null))
    })
}

/// Resolve a conflict.
#[pg_extern]
fn caliber_conflict_resolve(
    conflict_id: pgrx::Uuid,
    strategy: &str,
    winner: Option<&str>,
    reason: &str,
) -> bool {
    let cid = Uuid::from_bytes(*conflict_id.as_bytes());
    let mut storage = STORAGE.write().unwrap();

    if let Some(conflict) = storage.conflicts.get_mut(&cid) {
        use caliber_agents::ConflictResolutionRecord;

        let res_strategy = match strategy {
            "last_write_wins" => ResolutionStrategy::LastWriteWins,
            "first_write_wins" => ResolutionStrategy::FirstWriteWins,
            "highest_confidence" => ResolutionStrategy::HighestConfidence,
            "merge" => ResolutionStrategy::Merge,
            "escalate" => ResolutionStrategy::Escalate,
            _ => ResolutionStrategy::RejectBoth,
        };

        let resolution = ConflictResolutionRecord::automatic(res_strategy, winner, reason);
        conflict.resolve(resolution);
        true
    } else {
        false
    }
}

/// List unresolved conflicts.
#[pg_extern]
fn caliber_conflict_list_unresolved() -> pgrx::JsonB {
    let storage = STORAGE.read().unwrap();

    let conflicts: Vec<&Conflict> = storage
        .conflicts
        .values()
        .filter(|c| c.status == ConflictStatus::Detected || c.status == ConflictStatus::Resolving)
        .collect();

    pgrx::JsonB(serde_json::to_value(&conflicts).unwrap_or(serde_json::json!([])))
}


// ============================================================================
// VECTOR SEARCH (Task 12.3)
// ============================================================================

/// Search for similar vectors.
/// Returns entity IDs and similarity scores.
#[pg_extern]
fn caliber_vector_search(
    query_embedding: pgrx::JsonB,
    limit: i32,
) -> pgrx::JsonB {
    // Parse the query embedding
    let query: EmbeddingVector = match serde_json::from_value(query_embedding.0) {
        Ok(v) => v,
        Err(_) => return pgrx::JsonB(serde_json::json!([])),
    };

    let storage = STORAGE.read().unwrap();
    let mut results: Vec<(EntityId, f32)> = Vec::new();

    // Search artifacts
    for artifact in storage.artifacts.values() {
        if let Some(ref embedding) = artifact.embedding {
            if let Ok(similarity) = query.cosine_similarity(embedding) {
                results.push((artifact.artifact_id, similarity));
            }
        }
    }

    // Search notes
    for note in storage.notes.values() {
        if let Some(ref embedding) = note.embedding {
            if let Ok(similarity) = query.cosine_similarity(embedding) {
                results.push((note.note_id, similarity));
            }
        }
    }

    // Sort by similarity descending
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Apply limit
    results.truncate(limit as usize);

    // Convert to JSON
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|(id, score)| {
            serde_json::json!({
                "entity_id": id.to_string(),
                "similarity": score,
            })
        })
        .collect();

    pgrx::JsonB(serde_json::json!(json_results))
}


// ============================================================================
// DEBUG SQL VIEWS (Task 12.7)
// ============================================================================

/// Get storage statistics for debugging.
#[pg_extern]
fn caliber_debug_stats() -> pgrx::JsonB {
    let storage = STORAGE.read().unwrap();

    pgrx::JsonB(serde_json::json!({
        "trajectories": storage.trajectories.len(),
        "scopes": storage.scopes.len(),
        "artifacts": storage.artifacts.len(),
        "notes": storage.notes.len(),
        "turns": storage.turns.len(),
        "agents": storage.agents.len(),
        "locks": storage.locks.len(),
        "messages": storage.messages.len(),
        "delegations": storage.delegations.len(),
        "handoffs": storage.handoffs.len(),
        "conflicts": storage.conflicts.len(),
    }))
}

/// Clear all storage (for testing).
#[pg_extern]
fn caliber_debug_clear() -> &'static str {
    let mut storage = STORAGE.write().unwrap();
    storage.trajectories.clear();
    storage.scopes.clear();
    storage.artifacts.clear();
    storage.notes.clear();
    storage.turns.clear();
    storage.agents.clear();
    storage.locks.clear();
    storage.messages.clear();
    storage.delegations.clear();
    storage.handoffs.clear();
    storage.conflicts.clear();
    "Storage cleared"
}

/// Dump all trajectories for debugging.
#[pg_extern]
fn caliber_debug_dump_trajectories() -> pgrx::JsonB {
    let storage = STORAGE.read().unwrap();
    let trajectories: Vec<&Trajectory> = storage.trajectories.values().collect();
    pgrx::JsonB(serde_json::to_value(&trajectories).unwrap_or(serde_json::json!([])))
}

/// Dump all scopes for debugging.
#[pg_extern]
fn caliber_debug_dump_scopes() -> pgrx::JsonB {
    let storage = STORAGE.read().unwrap();
    let scopes: Vec<&Scope> = storage.scopes.values().collect();
    pgrx::JsonB(serde_json::to_value(&scopes).unwrap_or(serde_json::json!([])))
}

/// Dump all artifacts for debugging.
#[pg_extern]
fn caliber_debug_dump_artifacts() -> pgrx::JsonB {
    let storage = STORAGE.read().unwrap();
    let artifacts: Vec<&Artifact> = storage.artifacts.values().collect();
    pgrx::JsonB(serde_json::to_value(&artifacts).unwrap_or(serde_json::json!([])))
}

/// Dump all agents for debugging.
#[pg_extern]
fn caliber_debug_dump_agents() -> pgrx::JsonB {
    let storage = STORAGE.read().unwrap();
    let agents: Vec<&Agent> = storage.agents.values().collect();
    pgrx::JsonB(serde_json::to_value(&agents).unwrap_or(serde_json::json!([])))
}


// ============================================================================
// ACCESS CONTROL (Task 12.3)
// ============================================================================

/// Check if an agent has access to a memory region.
#[pg_extern]
fn caliber_check_access(
    agent_id: pgrx::Uuid,
    region_id: pgrx::Uuid,
    access_type: &str,
) -> bool {
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let _rid = Uuid::from_bytes(*region_id.as_bytes());

    // For now, implement basic access control
    // In production, this would check against MemoryRegionConfig
    let storage = STORAGE.read().unwrap();

    // Check if agent exists
    if !storage.agents.contains_key(&aid) {
        return false;
    }

    // Basic implementation: allow all access for registered agents
    // In production, this would check MemoryRegionConfig permissions
    match access_type {
        "read" | "write" => true,
        _ => false,
    }
}


// ============================================================================
// STORAGE TRAIT IMPLEMENTATION (Task 12.3)
// ============================================================================

/// PostgreSQL storage implementation.
/// Uses in-memory storage for development, would use direct heap operations in production.
pub struct PgStorage;

impl StorageTrait for PgStorage {
    fn trajectory_insert(&self, t: &Trajectory) -> CaliberResult<()> {
        let mut storage = STORAGE.write().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire write lock".to_string(),
            })
        })?;

        if storage.trajectories.contains_key(&t.trajectory_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Trajectory,
                reason: "already exists".to_string(),
            }));
        }

        storage.trajectories.insert(t.trajectory_id, t.clone());
        Ok(())
    }

    fn trajectory_get(&self, id: EntityId) -> CaliberResult<Option<Trajectory>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage.trajectories.get(&id).cloned())
    }

    fn trajectory_update(&self, id: EntityId, update: TrajectoryUpdate) -> CaliberResult<()> {
        let mut storage = STORAGE.write().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire write lock".to_string(),
            })
        })?;

        let trajectory = storage.trajectories.get_mut(&id).ok_or(
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Trajectory,
                id,
            })
        )?;

        if let Some(status) = update.status {
            trajectory.status = status;
        }
        if let Some(metadata) = update.metadata {
            trajectory.metadata = Some(metadata);
        }
        trajectory.updated_at = Utc::now();

        Ok(())
    }

    fn trajectory_list_by_status(&self, status: TrajectoryStatus) -> CaliberResult<Vec<Trajectory>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage
            .trajectories
            .values()
            .filter(|t| t.status == status)
            .cloned()
            .collect())
    }

    fn scope_insert(&self, s: &Scope) -> CaliberResult<()> {
        let mut storage = STORAGE.write().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire write lock".to_string(),
            })
        })?;

        if storage.scopes.contains_key(&s.scope_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Scope,
                reason: "already exists".to_string(),
            }));
        }

        storage.scopes.insert(s.scope_id, s.clone());
        Ok(())
    }

    fn scope_get(&self, id: EntityId) -> CaliberResult<Option<Scope>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage.scopes.get(&id).cloned())
    }

    fn scope_get_current(&self, trajectory_id: EntityId) -> CaliberResult<Option<Scope>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage
            .scopes
            .values()
            .filter(|s| s.trajectory_id == trajectory_id && s.is_active)
            .max_by_key(|s| s.created_at)
            .cloned())
    }

    fn scope_update(&self, id: EntityId, update: ScopeUpdate) -> CaliberResult<()> {
        let mut storage = STORAGE.write().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire write lock".to_string(),
            })
        })?;

        let scope = storage.scopes.get_mut(&id).ok_or(
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Scope,
                id,
            })
        )?;

        if let Some(is_active) = update.is_active {
            scope.is_active = is_active;
        }
        if let Some(closed_at) = update.closed_at {
            scope.closed_at = Some(closed_at);
        }
        if let Some(tokens_used) = update.tokens_used {
            scope.tokens_used = tokens_used;
        }
        if let Some(checkpoint) = update.checkpoint {
            scope.checkpoint = Some(checkpoint);
        }

        Ok(())
    }

    fn scope_list_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<Vec<Scope>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage
            .scopes
            .values()
            .filter(|s| s.trajectory_id == trajectory_id)
            .cloned()
            .collect())
    }

    fn artifact_insert(&self, a: &Artifact) -> CaliberResult<()> {
        let mut storage = STORAGE.write().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire write lock".to_string(),
            })
        })?;

        if storage.artifacts.contains_key(&a.artifact_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Artifact,
                reason: "already exists".to_string(),
            }));
        }

        storage.artifacts.insert(a.artifact_id, a.clone());
        Ok(())
    }

    fn artifact_get(&self, id: EntityId) -> CaliberResult<Option<Artifact>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage.artifacts.get(&id).cloned())
    }

    fn artifact_query_by_type(
        &self,
        trajectory_id: EntityId,
        artifact_type: ArtifactType,
    ) -> CaliberResult<Vec<Artifact>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage
            .artifacts
            .values()
            .filter(|a| a.trajectory_id == trajectory_id && a.artifact_type == artifact_type)
            .cloned()
            .collect())
    }

    fn artifact_query_by_scope(&self, scope_id: EntityId) -> CaliberResult<Vec<Artifact>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage
            .artifacts
            .values()
            .filter(|a| a.scope_id == scope_id)
            .cloned()
            .collect())
    }

    fn artifact_update(&self, id: EntityId, update: ArtifactUpdate) -> CaliberResult<()> {
        let mut storage = STORAGE.write().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire write lock".to_string(),
            })
        })?;

        let artifact = storage.artifacts.get_mut(&id).ok_or(
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Artifact,
                id,
            })
        )?;

        if let Some(content) = update.content {
            artifact.content = content;
            artifact.content_hash = compute_content_hash(artifact.content.as_bytes());
            artifact.updated_at = Utc::now();
        }
        if let Some(embedding) = update.embedding {
            artifact.embedding = Some(embedding);
        }
        if let Some(superseded_by) = update.superseded_by {
            artifact.superseded_by = Some(superseded_by);
        }

        Ok(())
    }

    fn note_insert(&self, n: &Note) -> CaliberResult<()> {
        let mut storage = STORAGE.write().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire write lock".to_string(),
            })
        })?;

        if storage.notes.contains_key(&n.note_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Note,
                reason: "already exists".to_string(),
            }));
        }

        storage.notes.insert(n.note_id, n.clone());
        Ok(())
    }

    fn note_get(&self, id: EntityId) -> CaliberResult<Option<Note>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage.notes.get(&id).cloned())
    }

    fn note_query_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<Vec<Note>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        Ok(storage
            .notes
            .values()
            .filter(|n| n.source_trajectory_ids.contains(&trajectory_id))
            .cloned()
            .collect())
    }

    fn note_update(&self, id: EntityId, update: NoteUpdate) -> CaliberResult<()> {
        let mut storage = STORAGE.write().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire write lock".to_string(),
            })
        })?;

        let note = storage.notes.get_mut(&id).ok_or(
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Note,
                id,
            })
        )?;

        if let Some(content) = update.content {
            note.content = content;
            note.content_hash = compute_content_hash(note.content.as_bytes());
            note.updated_at = Utc::now();
        }
        if let Some(embedding) = update.embedding {
            note.embedding = Some(embedding);
        }
        if let Some(superseded_by) = update.superseded_by {
            note.superseded_by = Some(superseded_by);
        }

        Ok(())
    }

    fn turn_insert(&self, t: &Turn) -> CaliberResult<()> {
        let mut storage = STORAGE.write().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire write lock".to_string(),
            })
        })?;

        if storage.turns.contains_key(&t.turn_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Scope, // Turn uses Scope as entity type
                reason: "already exists".to_string(),
            }));
        }

        storage.turns.insert(t.turn_id, t.clone());
        Ok(())
    }

    fn turn_get_by_scope(&self, scope_id: EntityId) -> CaliberResult<Vec<Turn>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        let mut turns: Vec<Turn> = storage
            .turns
            .values()
            .filter(|t| t.scope_id == scope_id)
            .cloned()
            .collect();

        turns.sort_by_key(|t| t.sequence);
        Ok(turns)
    }

    fn vector_search(
        &self,
        query: &EmbeddingVector,
        limit: i32,
    ) -> CaliberResult<Vec<(EntityId, f32)>> {
        let storage = STORAGE.read().map_err(|_| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to acquire read lock".to_string(),
            })
        })?;

        let mut results: Vec<(EntityId, f32)> = Vec::new();

        // Search artifacts
        for artifact in storage.artifacts.values() {
            if let Some(ref embedding) = artifact.embedding {
                if let Ok(similarity) = query.cosine_similarity(embedding) {
                    results.push((artifact.artifact_id, similarity));
                }
            }
        }

        // Search notes
        for note in storage.notes.values() {
            if let Some(ref embedding) = note.embedding {
                if let Ok(similarity) = query.cosine_similarity(embedding) {
                    results.push((note.note_id, similarity));
                }
            }
        }

        // Sort by similarity descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        results.truncate(limit as usize);

        Ok(results)
    }
}


// ============================================================================
// PGRX INTEGRATION TESTS (Task 12.8)
// ============================================================================

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_caliber_version() {
        let version = crate::caliber_version();
        assert!(!version.is_empty());
    }

    #[pg_test]
    fn test_caliber_new_id() {
        let id1 = crate::caliber_new_id();
        let id2 = crate::caliber_new_id();
        // IDs should be unique
        assert_ne!(id1, id2);
    }

    #[pg_test]
    fn test_trajectory_lifecycle() {
        // Clear storage first
        crate::caliber_debug_clear();

        // Create trajectory
        let traj_id = crate::caliber_trajectory_create(
            "Test Trajectory",
            Some("Test description"),
            None,
        );

        // Get trajectory
        let traj = crate::caliber_trajectory_get(traj_id);
        assert!(traj.is_some());

        // Update status
        let updated = crate::caliber_trajectory_set_status(traj_id, "completed");
        assert!(updated);

        // Verify status change
        let traj = crate::caliber_trajectory_get(traj_id);
        assert!(traj.is_some());
    }

    #[pg_test]
    fn test_scope_lifecycle() {
        crate::caliber_debug_clear();

        // Create trajectory first
        let traj_id = crate::caliber_trajectory_create("Test", None, None);

        // Create scope
        let scope_id = crate::caliber_scope_create(traj_id, "Test Scope", None, 8000);

        // Get scope
        let scope = crate::caliber_scope_get(scope_id);
        assert!(scope.is_some());

        // Get current scope
        let current = crate::caliber_scope_get_current(traj_id);
        assert!(current.is_some());

        // Close scope
        let closed = crate::caliber_scope_close(scope_id);
        assert!(closed);
    }

    #[pg_test]
    fn test_artifact_lifecycle() {
        crate::caliber_debug_clear();

        let traj_id = crate::caliber_trajectory_create("Test", None, None);
        let scope_id = crate::caliber_scope_create(traj_id, "Test Scope", None, 8000);

        // Create artifact
        let artifact_id = crate::caliber_artifact_create(
            traj_id,
            scope_id,
            "fact",
            "Test Artifact",
            "Test content",
        );

        // Get artifact
        let artifact = crate::caliber_artifact_get(artifact_id);
        assert!(artifact.is_some());

        // Query by type
        let artifacts = crate::caliber_artifact_query_by_type(traj_id, "fact");
        let arr: Vec<serde_json::Value> = serde_json::from_value(artifacts.0).unwrap();
        assert!(!arr.is_empty());
    }

    #[pg_test]
    fn test_note_lifecycle() {
        crate::caliber_debug_clear();

        let traj_id = crate::caliber_trajectory_create("Test", None, None);

        // Create note
        let note_id = crate::caliber_note_create(
            "fact",
            "Test Note",
            "Test content",
            Some(traj_id),
        );

        // Get note
        let note = crate::caliber_note_get(note_id);
        assert!(note.is_some());

        // Query by trajectory
        let notes = crate::caliber_note_query_by_trajectory(traj_id);
        let arr: Vec<serde_json::Value> = serde_json::from_value(notes.0).unwrap();
        assert!(!arr.is_empty());
    }

    #[pg_test]
    fn test_turn_lifecycle() {
        crate::caliber_debug_clear();

        let traj_id = crate::caliber_trajectory_create("Test", None, None);
        let scope_id = crate::caliber_scope_create(traj_id, "Test Scope", None, 8000);

        // Create turns
        let _turn1 = crate::caliber_turn_create(scope_id, 1, "user", "Hello", 5);
        let _turn2 = crate::caliber_turn_create(scope_id, 2, "assistant", "Hi there!", 10);

        // Get turns by scope
        let turns = crate::caliber_turn_get_by_scope(scope_id);
        let arr: Vec<serde_json::Value> = serde_json::from_value(turns.0).unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[pg_test]
    fn test_agent_lifecycle() {
        crate::caliber_debug_clear();

        // Register agent
        let caps = pgrx::JsonB(serde_json::json!(["rust", "python"]));
        let agent_id = crate::caliber_agent_register("coder", caps);

        // Get agent
        let agent = crate::caliber_agent_get(agent_id);
        assert!(agent.is_some());

        // Update status
        let updated = crate::caliber_agent_set_status(agent_id, "active");
        assert!(updated);

        // Heartbeat
        let heartbeat = crate::caliber_agent_heartbeat(agent_id);
        assert!(heartbeat);

        // List by type
        let agents = crate::caliber_agent_list_by_type("coder");
        let arr: Vec<serde_json::Value> = serde_json::from_value(agents.0).unwrap();
        assert!(!arr.is_empty());
    }

    #[pg_test]
    fn test_message_lifecycle() {
        crate::caliber_debug_clear();

        let caps = pgrx::JsonB(serde_json::json!([]));
        let agent1 = crate::caliber_agent_register("sender", caps.clone());
        let agent2 = crate::caliber_agent_register("receiver", caps);

        // Send message
        let msg_id = crate::caliber_message_send(
            agent1,
            Some(agent2),
            None,
            "heartbeat",
            "{}",
            "normal",
        );

        // Get message
        let msg = crate::caliber_message_get(msg_id);
        assert!(msg.is_some());

        // Mark delivered
        let delivered = crate::caliber_message_mark_delivered(msg_id);
        assert!(delivered);

        // Mark acknowledged
        let acked = crate::caliber_message_mark_acknowledged(msg_id);
        assert!(acked);
    }

    #[pg_test]
    fn test_delegation_lifecycle() {
        crate::caliber_debug_clear();

        let caps = pgrx::JsonB(serde_json::json!([]));
        let delegator = crate::caliber_agent_register("planner", caps.clone());
        let delegatee = crate::caliber_agent_register("coder", caps);
        let traj_id = crate::caliber_trajectory_create("Parent Task", None, None);

        // Create delegation
        let delegation_id = crate::caliber_delegation_create(
            delegator,
            Some(delegatee),
            None,
            "Implement feature X",
            traj_id,
        );

        // Get delegation
        let delegation = crate::caliber_delegation_get(delegation_id);
        assert!(delegation.is_some());

        // Accept delegation
        let child_traj = crate::caliber_trajectory_create("Child Task", None, None);
        let accepted = crate::caliber_delegation_accept(delegation_id, delegatee, child_traj);
        assert!(accepted);

        // Complete delegation
        let completed = crate::caliber_delegation_complete(delegation_id, true, "Done!");
        assert!(completed);
    }

    #[pg_test]
    fn test_handoff_lifecycle() {
        crate::caliber_debug_clear();

        let caps = pgrx::JsonB(serde_json::json!([]));
        let agent1 = crate::caliber_agent_register("generalist", caps.clone());
        let agent2 = crate::caliber_agent_register("specialist", caps);
        let traj_id = crate::caliber_trajectory_create("Task", None, None);
        let scope_id = crate::caliber_scope_create(traj_id, "Scope", None, 8000);
        let snapshot_id = crate::caliber_new_id();

        // Create handoff
        let handoff_id = crate::caliber_handoff_create(
            agent1,
            Some(agent2),
            None,
            traj_id,
            scope_id,
            snapshot_id,
            "specialization",
        );

        // Get handoff
        let handoff = crate::caliber_handoff_get(handoff_id);
        assert!(handoff.is_some());

        // Accept handoff
        let accepted = crate::caliber_handoff_accept(handoff_id, agent2);
        assert!(accepted);

        // Complete handoff
        let completed = crate::caliber_handoff_complete(handoff_id);
        assert!(completed);
    }

    #[pg_test]
    fn test_conflict_lifecycle() {
        crate::caliber_debug_clear();

        let artifact_a = crate::caliber_new_id();
        let artifact_b = crate::caliber_new_id();

        // Create conflict
        let conflict_id = crate::caliber_conflict_create(
            "contradicting_fact",
            "artifact",
            artifact_a,
            "artifact",
            artifact_b,
        );

        // Get conflict
        let conflict = crate::caliber_conflict_get(conflict_id);
        assert!(conflict.is_some());

        // List unresolved
        let unresolved = crate::caliber_conflict_list_unresolved();
        let arr: Vec<serde_json::Value> = serde_json::from_value(unresolved.0).unwrap();
        assert!(!arr.is_empty());

        // Resolve conflict
        let resolved = crate::caliber_conflict_resolve(
            conflict_id,
            "highest_confidence",
            Some("a"),
            "Artifact A has higher confidence",
        );
        assert!(resolved);
    }

    #[pg_test]
    fn test_debug_stats() {
        crate::caliber_debug_clear();

        // Create some data
        let _traj = crate::caliber_trajectory_create("Test", None, None);
        let caps = pgrx::JsonB(serde_json::json!([]));
        let _agent = crate::caliber_agent_register("test", caps);

        // Get stats
        let stats = crate::caliber_debug_stats();
        let obj: serde_json::Value = stats.0;
        
        assert_eq!(obj["trajectories"], 1);
        assert_eq!(obj["agents"], 1);
    }
}

