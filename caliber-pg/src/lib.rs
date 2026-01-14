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
    // NOTE: All entity storage has been migrated to SPI-based SQL:
    // - trajectories: Task 1
    // - scopes: Task 2
    // - artifacts: Task 3
    // - notes: Task 4
    // - turns: Task 5
    // - locks: Task 6
    // - messages: Task 7
    // - agents: Task 8
    // - delegations: Task 9
    // - handoffs: Task 10
    // - conflicts: Task 11
    //
    // This struct is kept for backwards compatibility but is now empty.
    // All operations use caliber_* SQL tables via pgrx SPI.
}

// ============================================================================
// SAFE STORAGE ACCESS HELPERS
// ============================================================================

/// Safely acquire a read lock on storage, handling poisoning gracefully.
/// Returns the guard or panics with a clear error message for PostgreSQL.
fn storage_read() -> std::sync::RwLockReadGuard<'static, InMemoryStorage> {
    match STORAGE.read() {
        Ok(guard) => guard,
        Err(poisoned) => {
            pgrx::warning!("CALIBER: Storage lock was poisoned, recovering...");
            poisoned.into_inner()
        }
    }
}

/// Safely acquire a write lock on storage, handling poisoning gracefully.
/// Returns the guard or panics with a clear error message for PostgreSQL.
fn storage_write() -> std::sync::RwLockWriteGuard<'static, InMemoryStorage> {
    match STORAGE.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            pgrx::warning!("CALIBER: Storage lock was poisoned, recovering...");
            poisoned.into_inner()
        }
    }
}

/// Safely serialize a value to JSON, returning null on failure.
fn safe_to_json<T: serde::Serialize>(value: &T) -> serde_json::Value {
    match serde_json::to_value(value) {
        Ok(v) => v,
        Err(e) => {
            pgrx::warning!("CALIBER: JSON serialization failed: {}", e);
            serde_json::Value::Null
        }
    }
}

/// Safely serialize a collection to JSON array, returning empty array on failure.
fn safe_to_json_array<T: serde::Serialize>(values: &[T]) -> serde_json::Value {
    match serde_json::to_value(values) {
        Ok(v) => v,
        Err(e) => {
            pgrx::warning!("CALIBER: JSON array serialization failed: {}", e);
            serde_json::json!([])
        }
    }
}


// ============================================================================
// BOOTSTRAP SQL SCHEMA (Task 12.2)
// ============================================================================

/// Bootstrap SQL schema embedded at compile time.
/// This is the complete schema from caliber_init.sql.
const BOOTSTRAP_SQL: &str = include_str!("../sql/caliber_init.sql");

/// Initialize the CALIBER schema.
/// This creates all tables, indexes, and functions needed by the extension.
/// This SQL runs ONCE at extension install, NOT in hot path.
/// 
/// The schema is idempotent - all CREATE statements use IF NOT EXISTS.
/// 
/// # Returns
/// - Success message with table count on success
/// - Error message on failure
#[pg_extern]
fn caliber_init() -> String {
    pgrx::log!("CALIBER: Initializing schema...");
    
    // Execute the bootstrap SQL via SPI
    match Spi::connect(|client| {
        // Split the SQL into individual statements and execute each
        // This handles the multi-statement SQL file properly
        client.update(BOOTSTRAP_SQL, None, None)?;
        Ok(())
    }) {
        Ok(()) => {
            pgrx::log!("CALIBER: Schema initialization complete");
            "CALIBER schema initialized successfully".to_string()
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Schema initialization failed: {}", e);
            format!("CALIBER schema initialization failed: {}", e)
        }
    }
}

/// Check if the CALIBER schema is initialized.
/// Returns true if the core tables exist.
#[pg_extern]
fn caliber_schema_exists() -> bool {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_name = 'caliber_trajectory'
            )",
            None,
            None,
        );
        
        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    row.get::<bool>(1).unwrap_or(Some(false)).unwrap_or(false)
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    })
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

    // Insert into SQL table via SPI
    let result = Spi::connect(|client| {
        let agent_id_str = agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string());
        
        client.update(
            "INSERT INTO caliber_trajectory (trajectory_id, name, description, status, agent_id, created_at, updated_at) 
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), trajectory_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), name.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), description.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), "active".into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), agent_id.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
            ]),
        )
    });

    if let Err(e) = result {
        pgrx::warning!("CALIBER: Failed to insert trajectory: {}", e);
    }

    pgrx::Uuid::from_bytes(*trajectory_id.as_bytes())
}

/// Get a trajectory by ID.
#[pg_extern]
fn caliber_trajectory_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT trajectory_id, name, description, status, parent_trajectory_id, 
                    root_trajectory_id, agent_id, created_at, updated_at, completed_at, 
                    outcome, metadata 
             FROM caliber_trajectory WHERE trajectory_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let trajectory_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let name: Option<String> = row.get(2).ok().flatten();
                    let description: Option<String> = row.get(3).ok().flatten();
                    let status: Option<String> = row.get(4).ok().flatten();
                    let parent_trajectory_id: Option<pgrx::Uuid> = row.get(5).ok().flatten();
                    let root_trajectory_id: Option<pgrx::Uuid> = row.get(6).ok().flatten();
                    let agent_id: Option<pgrx::Uuid> = row.get(7).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(8).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(9).ok().flatten();
                    let completed_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let outcome: Option<pgrx::JsonB> = row.get(11).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                    Some(pgrx::JsonB(serde_json::json!({
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "name": name,
                        "description": description,
                        "status": status,
                        "parent_trajectory_id": parent_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "root_trajectory_id": root_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_id": agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                        "completed_at": completed_at.map(|t| format!("{:?}", t)),
                        "outcome": outcome.map(|j| j.0),
                        "metadata": metadata.map(|j| j.0),
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get trajectory: {}", e);
                None
            }
        }
    })
}

/// Update trajectory status.
/// Returns None if status is invalid.
#[pg_extern]
fn caliber_trajectory_set_status(id: pgrx::Uuid, status: &str) -> Option<bool> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    
    // Validate status - reject unknown values instead of returning false silently (REQ-12)
    let valid_status = match status {
        "active" | "completed" | "failed" | "suspended" => status,
        _ => {
            let validation_err = ValidationError::InvalidValue {
                field: "status".to_string(),
                reason: format!("unknown value '{}'. Valid values: active, completed, failed, suspended", status),
            };
            pgrx::warning!("CALIBER: {:?}", validation_err);
            return None;
        }
    };

    let result = Spi::connect(|client| {
        let now = Utc::now();
        client.update(
            "UPDATE caliber_trajectory SET status = $1, updated_at = $2 WHERE trajectory_id = $3",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::TEXTOID.oid(), valid_status.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()),
            ]),
        )
    });

    match result {
        Ok(update_result) => Some(update_result.rows_affected() > 0),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to update trajectory status: {}", e);
            Some(false)
        }
    }
}

/// Update a trajectory with the provided fields.
/// Accepts a JSON object with optional fields: name, description, status, 
/// parent_trajectory_id, root_trajectory_id, agent_id, completed_at, outcome, metadata.
/// Only provided fields are updated; null/missing fields are left unchanged.
/// Returns true if the trajectory was found and updated, false otherwise.
#[pg_extern]
fn caliber_trajectory_update(id: pgrx::Uuid, updates: pgrx::JsonB) -> bool {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let update_obj = &updates.0;

    // Build dynamic UPDATE query based on provided fields
    let mut set_clauses: Vec<String> = Vec::new();
    let mut params: Vec<(pgrx::pg_sys::Oid, Option<pgrx::pg_sys::Datum>)> = Vec::new();
    let mut param_idx = 1;

    // Handle name update
    if let Some(name) = update_obj.get("name").and_then(|v| v.as_str()) {
        set_clauses.push(format!("name = ${}", param_idx));
        params.push((pgrx::PgBuiltInOids::TEXTOID.oid(), name.into_datum()));
        param_idx += 1;
    }

    // Handle description update
    if let Some(description) = update_obj.get("description") {
        if description.is_null() {
            set_clauses.push(format!("description = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::TEXTOID.oid(), None::<&str>.into_datum()));
            param_idx += 1;
        } else if let Some(desc_str) = description.as_str() {
            set_clauses.push(format!("description = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::TEXTOID.oid(), desc_str.into_datum()));
            param_idx += 1;
        }
    }

    // Handle status update with validation
    if let Some(status) = update_obj.get("status").and_then(|v| v.as_str()) {
        let valid_status = match status {
            "active" | "completed" | "failed" | "suspended" => status,
            _ => {
                pgrx::warning!("CALIBER: Invalid trajectory status: {}", status);
                return false;
            }
        };
        set_clauses.push(format!("status = ${}", param_idx));
        params.push((pgrx::PgBuiltInOids::TEXTOID.oid(), valid_status.into_datum()));
        param_idx += 1;

        // If status is completed or failed, set completed_at if not already provided
        if (status == "completed" || status == "failed") && !update_obj.contains_key("completed_at") {
            set_clauses.push(format!("completed_at = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), Utc::now().into_datum()));
            param_idx += 1;
        }
    }

    // Handle parent_trajectory_id update
    if let Some(parent_id) = update_obj.get("parent_trajectory_id") {
        if parent_id.is_null() {
            set_clauses.push(format!("parent_trajectory_id = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), None::<pgrx::Uuid>.into_datum()));
            param_idx += 1;
        } else if let Some(parent_str) = parent_id.as_str() {
            if let Ok(parent_uuid) = Uuid::parse_str(parent_str) {
                set_clauses.push(format!("parent_trajectory_id = ${}", param_idx));
                params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), pgrx::Uuid::from_bytes(*parent_uuid.as_bytes()).into_datum()));
                param_idx += 1;
            }
        }
    }

    // Handle root_trajectory_id update
    if let Some(root_id) = update_obj.get("root_trajectory_id") {
        if root_id.is_null() {
            set_clauses.push(format!("root_trajectory_id = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), None::<pgrx::Uuid>.into_datum()));
            param_idx += 1;
        } else if let Some(root_str) = root_id.as_str() {
            if let Ok(root_uuid) = Uuid::parse_str(root_str) {
                set_clauses.push(format!("root_trajectory_id = ${}", param_idx));
                params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), pgrx::Uuid::from_bytes(*root_uuid.as_bytes()).into_datum()));
                param_idx += 1;
            }
        }
    }

    // Handle agent_id update
    if let Some(agent_id) = update_obj.get("agent_id") {
        if agent_id.is_null() {
            set_clauses.push(format!("agent_id = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), None::<pgrx::Uuid>.into_datum()));
            param_idx += 1;
        } else if let Some(agent_str) = agent_id.as_str() {
            if let Ok(agent_uuid) = Uuid::parse_str(agent_str) {
                set_clauses.push(format!("agent_id = ${}", param_idx));
                params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), pgrx::Uuid::from_bytes(*agent_uuid.as_bytes()).into_datum()));
                param_idx += 1;
            }
        }
    }

    // Handle completed_at update (explicit)
    if let Some(completed_at) = update_obj.get("completed_at") {
        if completed_at.is_null() {
            set_clauses.push(format!("completed_at = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), None::<pgrx::TimestampWithTimeZone>.into_datum()));
            param_idx += 1;
        } else if let Some(completed_str) = completed_at.as_str() {
            if let Ok(completed_dt) = chrono::DateTime::parse_from_rfc3339(completed_str) {
                set_clauses.push(format!("completed_at = ${}", param_idx));
                params.push((pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), completed_dt.with_timezone(&Utc).into_datum()));
                param_idx += 1;
            }
        }
    }

    // Handle outcome update (JSONB)
    if let Some(outcome) = update_obj.get("outcome") {
        if outcome.is_null() {
            set_clauses.push(format!("outcome = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::JSONBOID.oid(), None::<pgrx::JsonB>.into_datum()));
            param_idx += 1;
        } else {
            set_clauses.push(format!("outcome = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::JSONBOID.oid(), pgrx::JsonB(outcome.clone()).into_datum()));
            param_idx += 1;
        }
    }

    // Handle metadata update (JSONB)
    if let Some(metadata) = update_obj.get("metadata") {
        if metadata.is_null() {
            set_clauses.push(format!("metadata = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::JSONBOID.oid(), None::<pgrx::JsonB>.into_datum()));
            param_idx += 1;
        } else {
            set_clauses.push(format!("metadata = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::JSONBOID.oid(), pgrx::JsonB(metadata.clone()).into_datum()));
            param_idx += 1;
        }
    }

    // If no fields to update, return false
    if set_clauses.is_empty() {
        pgrx::warning!("CALIBER: No valid fields to update in trajectory");
        return false;
    }

    // Always update updated_at
    set_clauses.push(format!("updated_at = ${}", param_idx));
    params.push((pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), Utc::now().into_datum()));
    param_idx += 1;

    // Add the WHERE clause parameter
    params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()));

    let query = format!(
        "UPDATE caliber_trajectory SET {} WHERE trajectory_id = ${}",
        set_clauses.join(", "),
        param_idx
    );

    let result = Spi::connect(|client| {
        client.update(&query, None, Some(params))
    });

    match result {
        Ok(update_result) => update_result.rows_affected() > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to update trajectory: {}", e);
            false
        }
    }
}

/// List trajectories by status.
#[pg_extern]
fn caliber_trajectory_list_by_status(status: &str) -> pgrx::JsonB {
    // Validate status
    let valid_status = match status {
        "active" | "completed" | "failed" | "suspended" => status,
        _ => return pgrx::JsonB(serde_json::json!([])),
    };

    Spi::connect(|client| {
        let result = client.select(
            "SELECT trajectory_id, name, description, status, parent_trajectory_id, 
                    root_trajectory_id, agent_id, created_at, updated_at, completed_at, 
                    outcome, metadata 
             FROM caliber_trajectory WHERE status = $1 ORDER BY created_at DESC",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::TEXTOID.oid(), valid_status.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                let mut trajectories = Vec::new();
                for row in table {
                    let trajectory_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let name: Option<String> = row.get(2).ok().flatten();
                    let description: Option<String> = row.get(3).ok().flatten();
                    let status: Option<String> = row.get(4).ok().flatten();
                    let parent_trajectory_id: Option<pgrx::Uuid> = row.get(5).ok().flatten();
                    let root_trajectory_id: Option<pgrx::Uuid> = row.get(6).ok().flatten();
                    let agent_id: Option<pgrx::Uuid> = row.get(7).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(8).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(9).ok().flatten();
                    let completed_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let outcome: Option<pgrx::JsonB> = row.get(11).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                    trajectories.push(serde_json::json!({
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "name": name,
                        "description": description,
                        "status": status,
                        "parent_trajectory_id": parent_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "root_trajectory_id": root_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_id": agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                        "completed_at": completed_at.map(|t| format!("{:?}", t)),
                        "outcome": outcome.map(|j| j.0),
                        "metadata": metadata.map(|j| j.0),
                    }));
                }
                pgrx::JsonB(serde_json::json!(trajectories))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to list trajectories: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
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

    // Insert into SQL table via SPI
    let result = Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber_scope (scope_id, trajectory_id, name, purpose, is_active, created_at, token_budget, tokens_used) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), scope_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), traj_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), name.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), purpose.into_datum()),
                (pgrx::PgBuiltInOids::BOOLOID.oid(), true.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::INT4OID.oid(), token_budget.into_datum()),
                (pgrx::PgBuiltInOids::INT4OID.oid(), 0i32.into_datum()),
            ]),
        )
    });

    if let Err(e) = result {
        pgrx::warning!("CALIBER: Failed to insert scope: {}", e);
    }

    pgrx::Uuid::from_bytes(*scope_id.as_bytes())
}

/// Get a scope by ID.
#[pg_extern]
fn caliber_scope_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT scope_id, trajectory_id, parent_scope_id, name, purpose, is_active, 
                    created_at, closed_at, checkpoint, token_budget, tokens_used, metadata 
             FROM caliber_scope WHERE scope_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let scope_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let trajectory_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let parent_scope_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let name: Option<String> = row.get(4).ok().flatten();
                    let purpose: Option<String> = row.get(5).ok().flatten();
                    let is_active: Option<bool> = row.get(6).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(7).ok().flatten();
                    let closed_at: Option<pgrx::TimestampWithTimeZone> = row.get(8).ok().flatten();
                    let checkpoint: Option<pgrx::JsonB> = row.get(9).ok().flatten();
                    let token_budget: Option<i32> = row.get(10).ok().flatten();
                    let tokens_used: Option<i32> = row.get(11).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                    Some(pgrx::JsonB(serde_json::json!({
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "parent_scope_id": parent_scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "name": name,
                        "purpose": purpose,
                        "is_active": is_active,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "closed_at": closed_at.map(|t| format!("{:?}", t)),
                        "checkpoint": checkpoint.map(|j| j.0),
                        "token_budget": token_budget,
                        "tokens_used": tokens_used,
                        "metadata": metadata.map(|j| j.0),
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get scope: {}", e);
                None
            }
        }
    })
}

/// Get the current active scope for a trajectory.
#[pg_extern]
fn caliber_scope_get_current(trajectory_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT scope_id, trajectory_id, parent_scope_id, name, purpose, is_active, 
                    created_at, closed_at, checkpoint, token_budget, tokens_used, metadata 
             FROM caliber_scope 
             WHERE trajectory_id = $1 AND is_active = TRUE 
             ORDER BY created_at DESC 
             LIMIT 1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), traj_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let scope_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let trajectory_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let parent_scope_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let name: Option<String> = row.get(4).ok().flatten();
                    let purpose: Option<String> = row.get(5).ok().flatten();
                    let is_active: Option<bool> = row.get(6).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(7).ok().flatten();
                    let closed_at: Option<pgrx::TimestampWithTimeZone> = row.get(8).ok().flatten();
                    let checkpoint: Option<pgrx::JsonB> = row.get(9).ok().flatten();
                    let token_budget: Option<i32> = row.get(10).ok().flatten();
                    let tokens_used: Option<i32> = row.get(11).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                    Some(pgrx::JsonB(serde_json::json!({
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "parent_scope_id": parent_scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "name": name,
                        "purpose": purpose,
                        "is_active": is_active,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "closed_at": closed_at.map(|t| format!("{:?}", t)),
                        "checkpoint": checkpoint.map(|j| j.0),
                        "token_budget": token_budget,
                        "tokens_used": tokens_used,
                        "metadata": metadata.map(|j| j.0),
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get current scope: {}", e);
                None
            }
        }
    })
}

/// Close a scope.
#[pg_extern]
fn caliber_scope_close(id: pgrx::Uuid) -> bool {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let now = Utc::now();

    let result = Spi::connect(|client| {
        client.update(
            "UPDATE caliber_scope SET is_active = $1, closed_at = $2 WHERE scope_id = $3",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::BOOLOID.oid(), false.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()),
            ]),
        )
    });

    match result {
        Ok(update_result) => update_result.rows_affected() > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to close scope: {}", e);
            false
        }
    }
}

/// Update tokens used in a scope.
#[pg_extern]
fn caliber_scope_update_tokens(id: pgrx::Uuid, tokens_used: i32) -> bool {
    let entity_id = Uuid::from_bytes(*id.as_bytes());

    let result = Spi::connect(|client| {
        client.update(
            "UPDATE caliber_scope SET tokens_used = $1 WHERE scope_id = $2",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::INT4OID.oid(), tokens_used.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()),
            ]),
        )
    });

    match result {
        Ok(update_result) => update_result.rows_affected() > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to update scope tokens: {}", e);
            false
        }
    }
}

/// Update a scope with the provided fields.
/// Accepts a JSON object with optional fields: name, purpose, is_active, closed_at, 
/// checkpoint, token_budget, tokens_used, parent_scope_id, metadata.
/// Only provided fields are updated; null/missing fields are left unchanged.
/// Returns true if the scope was found and updated, false otherwise.
#[pg_extern]
fn caliber_scope_update(id: pgrx::Uuid, updates: pgrx::JsonB) -> bool {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let update_obj = &updates.0;

    // Build dynamic UPDATE query based on provided fields
    let mut set_clauses: Vec<String> = Vec::new();
    let mut params: Vec<(pgrx::pg_sys::Oid, Option<pgrx::pg_sys::Datum>)> = Vec::new();
    let mut param_idx = 1;

    // Handle name update
    if let Some(name) = update_obj.get("name").and_then(|v| v.as_str()) {
        set_clauses.push(format!("name = ${}", param_idx));
        params.push((pgrx::PgBuiltInOids::TEXTOID.oid(), name.into_datum()));
        param_idx += 1;
    }

    // Handle purpose update
    if let Some(purpose) = update_obj.get("purpose") {
        if purpose.is_null() {
            set_clauses.push(format!("purpose = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::TEXTOID.oid(), None::<&str>.into_datum()));
            param_idx += 1;
        } else if let Some(purpose_str) = purpose.as_str() {
            set_clauses.push(format!("purpose = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::TEXTOID.oid(), purpose_str.into_datum()));
            param_idx += 1;
        }
    }

    // Handle is_active update
    if let Some(is_active) = update_obj.get("is_active").and_then(|v| v.as_bool()) {
        set_clauses.push(format!("is_active = ${}", param_idx));
        params.push((pgrx::PgBuiltInOids::BOOLOID.oid(), is_active.into_datum()));
        param_idx += 1;
    }

    // Handle closed_at update
    if let Some(closed_at) = update_obj.get("closed_at") {
        if closed_at.is_null() {
            set_clauses.push(format!("closed_at = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), None::<pgrx::TimestampWithTimeZone>.into_datum()));
            param_idx += 1;
        } else if let Some(closed_str) = closed_at.as_str() {
            if let Ok(closed_dt) = chrono::DateTime::parse_from_rfc3339(closed_str) {
                set_clauses.push(format!("closed_at = ${}", param_idx));
                params.push((pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), closed_dt.with_timezone(&Utc).into_datum()));
                param_idx += 1;
            }
        }
    }

    // Handle checkpoint update (JSONB)
    if let Some(checkpoint) = update_obj.get("checkpoint") {
        if checkpoint.is_null() {
            set_clauses.push(format!("checkpoint = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::JSONBOID.oid(), None::<pgrx::JsonB>.into_datum()));
            param_idx += 1;
        } else {
            set_clauses.push(format!("checkpoint = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::JSONBOID.oid(), pgrx::JsonB(checkpoint.clone()).into_datum()));
            param_idx += 1;
        }
    }

    // Handle token_budget update
    if let Some(token_budget) = update_obj.get("token_budget").and_then(|v| v.as_i64()) {
        set_clauses.push(format!("token_budget = ${}", param_idx));
        params.push((pgrx::PgBuiltInOids::INT4OID.oid(), (token_budget as i32).into_datum()));
        param_idx += 1;
    }

    // Handle tokens_used update
    if let Some(tokens_used) = update_obj.get("tokens_used").and_then(|v| v.as_i64()) {
        set_clauses.push(format!("tokens_used = ${}", param_idx));
        params.push((pgrx::PgBuiltInOids::INT4OID.oid(), (tokens_used as i32).into_datum()));
        param_idx += 1;
    }

    // Handle parent_scope_id update
    if let Some(parent_id) = update_obj.get("parent_scope_id") {
        if parent_id.is_null() {
            set_clauses.push(format!("parent_scope_id = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), None::<pgrx::Uuid>.into_datum()));
            param_idx += 1;
        } else if let Some(parent_str) = parent_id.as_str() {
            if let Ok(parent_uuid) = Uuid::parse_str(parent_str) {
                set_clauses.push(format!("parent_scope_id = ${}", param_idx));
                params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), pgrx::Uuid::from_bytes(*parent_uuid.as_bytes()).into_datum()));
                param_idx += 1;
            }
        }
    }

    // Handle metadata update (JSONB)
    if let Some(metadata) = update_obj.get("metadata") {
        if metadata.is_null() {
            set_clauses.push(format!("metadata = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::JSONBOID.oid(), None::<pgrx::JsonB>.into_datum()));
            param_idx += 1;
        } else {
            set_clauses.push(format!("metadata = ${}", param_idx));
            params.push((pgrx::PgBuiltInOids::JSONBOID.oid(), pgrx::JsonB(metadata.clone()).into_datum()));
            param_idx += 1;
        }
    }

    // If no fields to update, return false
    if set_clauses.is_empty() {
        pgrx::warning!("CALIBER: No valid fields to update in scope");
        return false;
    }

    // Add the WHERE clause parameter
    params.push((pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()));

    let query = format!(
        "UPDATE caliber_scope SET {} WHERE scope_id = ${}",
        set_clauses.join(", "),
        param_idx
    );

    let result = Spi::connect(|client| {
        client.update(&query, None, Some(params))
    });

    match result {
        Ok(update_result) => update_result.rows_affected() > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to update scope: {}", e);
            false
        }
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
) -> Option<pgrx::Uuid> {
    // Validate artifact_type - reject unknown values (REQ-12)
    let valid_artifact_type = match artifact_type {
        "error_log" | "code_patch" | "design_decision" | "user_preference" 
        | "fact" | "constraint" | "tool_result" | "intermediate_output" | "custom" => artifact_type,
        _ => {
            let validation_err = ValidationError::InvalidValue {
                field: "artifact_type".to_string(),
                reason: format!("unknown value '{}'. Valid values: error_log, code_patch, design_decision, user_preference, fact, constraint, tool_result, intermediate_output, custom", artifact_type),
            };
            pgrx::warning!("CALIBER: {:?}", validation_err);
            return None;
        }
    };

    let artifact_id = new_entity_id();
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());
    let now = Utc::now();

    // Compute content hash
    let content_hash = compute_content_hash(content.as_bytes());

    // Build provenance JSON
    let provenance = serde_json::json!({
        "source_turn": 0,
        "extraction_method": "explicit",
        "confidence": null
    });

    // Insert into SQL table via SPI
    let result = Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber_artifact (
                artifact_id, trajectory_id, scope_id, artifact_type, name, 
                content, content_hash, embedding, provenance, ttl, 
                created_at, updated_at, superseded_by, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), artifact_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), traj_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), scp_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), valid_artifact_type.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), name.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), content.into_datum()),
                (pgrx::PgBuiltInOids::BYTEAOID.oid(), content_hash.as_slice().into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), None::<&str>.into_datum()), // embedding (NULL for now)
                (pgrx::PgBuiltInOids::JSONBOID.oid(), pgrx::JsonB(provenance).into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), "persistent".into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), None::<pgrx::Uuid>.into_datum()),
                (pgrx::PgBuiltInOids::JSONBOID.oid(), None::<pgrx::JsonB>.into_datum()),
            ]),
        )
    });

    match result {
        Ok(_) => Some(pgrx::Uuid::from_bytes(*artifact_id.as_bytes())),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to insert artifact: {}", e);
            None
        }
    }
}

/// Get an artifact by ID.
#[pg_extern]
fn caliber_artifact_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT artifact_id, trajectory_id, scope_id, artifact_type, name, 
                    content, content_hash, embedding, provenance, ttl, 
                    created_at, updated_at, superseded_by, metadata 
             FROM caliber_artifact WHERE artifact_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let artifact_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let trajectory_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let scope_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let artifact_type: Option<String> = row.get(4).ok().flatten();
                    let name: Option<String> = row.get(5).ok().flatten();
                    let content: Option<String> = row.get(6).ok().flatten();
                    let content_hash: Option<Vec<u8>> = row.get(7).ok().flatten();
                    let embedding: Option<String> = row.get(8).ok().flatten();
                    let provenance: Option<pgrx::JsonB> = row.get(9).ok().flatten();
                    let ttl: Option<String> = row.get(10).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(11).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(12).ok().flatten();
                    let superseded_by: Option<pgrx::Uuid> = row.get(13).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(14).ok().flatten();

                    Some(pgrx::JsonB(serde_json::json!({
                        "artifact_id": artifact_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "artifact_type": artifact_type,
                        "name": name,
                        "content": content,
                        "content_hash": content_hash.map(|h| hex::encode(h)),
                        "embedding": embedding,
                        "provenance": provenance.map(|j| j.0),
                        "ttl": ttl,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                        "superseded_by": superseded_by.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "metadata": metadata.map(|j| j.0),
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get artifact: {}", e);
                None
            }
        }
    })
}

/// Query artifacts by type within a trajectory.
#[pg_extern]
fn caliber_artifact_query_by_type(
    trajectory_id: pgrx::Uuid,
    artifact_type: &str,
) -> pgrx::JsonB {
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT artifact_id, trajectory_id, scope_id, artifact_type, name, 
                    content, content_hash, embedding, provenance, ttl, 
                    created_at, updated_at, superseded_by, metadata 
             FROM caliber_artifact 
             WHERE trajectory_id = $1 AND artifact_type = $2 
             ORDER BY created_at DESC",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), traj_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), artifact_type.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                let mut artifacts = Vec::new();
                for row in table {
                    let artifact_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let trajectory_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let scope_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let artifact_type: Option<String> = row.get(4).ok().flatten();
                    let name: Option<String> = row.get(5).ok().flatten();
                    let content: Option<String> = row.get(6).ok().flatten();
                    let content_hash: Option<Vec<u8>> = row.get(7).ok().flatten();
                    let embedding: Option<String> = row.get(8).ok().flatten();
                    let provenance: Option<pgrx::JsonB> = row.get(9).ok().flatten();
                    let ttl: Option<String> = row.get(10).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(11).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(12).ok().flatten();
                    let superseded_by: Option<pgrx::Uuid> = row.get(13).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(14).ok().flatten();

                    artifacts.push(serde_json::json!({
                        "artifact_id": artifact_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "artifact_type": artifact_type,
                        "name": name,
                        "content": content,
                        "content_hash": content_hash.map(|h| hex::encode(h)),
                        "embedding": embedding,
                        "provenance": provenance.map(|j| j.0),
                        "ttl": ttl,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                        "superseded_by": superseded_by.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "metadata": metadata.map(|j| j.0),
                    }));
                }
                pgrx::JsonB(serde_json::json!(artifacts))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to query artifacts by type: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
}

/// Query artifacts by scope.
#[pg_extern]
fn caliber_artifact_query_by_scope(scope_id: pgrx::Uuid) -> pgrx::JsonB {
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT artifact_id, trajectory_id, scope_id, artifact_type, name, 
                    content, content_hash, embedding, provenance, ttl, 
                    created_at, updated_at, superseded_by, metadata 
             FROM caliber_artifact 
             WHERE scope_id = $1 
             ORDER BY created_at DESC",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), scp_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                let mut artifacts = Vec::new();
                for row in table {
                    let artifact_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let trajectory_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let scope_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let artifact_type: Option<String> = row.get(4).ok().flatten();
                    let name: Option<String> = row.get(5).ok().flatten();
                    let content: Option<String> = row.get(6).ok().flatten();
                    let content_hash: Option<Vec<u8>> = row.get(7).ok().flatten();
                    let embedding: Option<String> = row.get(8).ok().flatten();
                    let provenance: Option<pgrx::JsonB> = row.get(9).ok().flatten();
                    let ttl: Option<String> = row.get(10).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(11).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(12).ok().flatten();
                    let superseded_by: Option<pgrx::Uuid> = row.get(13).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(14).ok().flatten();

                    artifacts.push(serde_json::json!({
                        "artifact_id": artifact_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "artifact_type": artifact_type,
                        "name": name,
                        "content": content,
                        "content_hash": content_hash.map(|h| hex::encode(h)),
                        "embedding": embedding,
                        "provenance": provenance.map(|j| j.0),
                        "ttl": ttl,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                        "superseded_by": superseded_by.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "metadata": metadata.map(|j| j.0),
                    }));
                }
                pgrx::JsonB(serde_json::json!(artifacts))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to query artifacts by scope: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
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
) -> Option<pgrx::Uuid> {
    let note_id = new_entity_id();
    let now = Utc::now();

    // Validate note_type - reject unknown values instead of defaulting (REQ-12)
    let valid_note_type = match note_type {
        "convention" | "strategy" | "gotcha" | "fact" | "preference" | "relationship" | "procedure" | "meta" => note_type,
        _ => {
            let validation_err = ValidationError::InvalidValue {
                field: "note_type".to_string(),
                reason: format!("unknown value '{}'. Valid values: convention, strategy, gotcha, fact, preference, relationship, procedure, meta", note_type),
            };
            pgrx::warning!("CALIBER: {:?}", validation_err);
            return None;
        }
    };

    let content_hash = compute_content_hash(content.as_bytes());
    
    // Build source_trajectory_ids array
    let source_traj_ids: Vec<pgrx::Uuid> = source_trajectory_id
        .map(|u| vec![u])
        .unwrap_or_default();

    // Insert into SQL table via SPI
    let result = Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber_note (note_id, note_type, title, content, content_hash, 
                                       source_trajectory_ids, source_artifact_ids, ttl, 
                                       created_at, updated_at, accessed_at, access_count) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), note_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), valid_note_type.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), title.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), content.into_datum()),
                (pgrx::PgBuiltInOids::BYTEAOID.oid(), content_hash.as_slice().into_datum()),
                (pgrx::PgBuiltInOids::UUIDARRAYOID.oid(), source_traj_ids.into_datum()),
                (pgrx::PgBuiltInOids::UUIDARRAYOID.oid(), Vec::<pgrx::Uuid>::new().into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), "persistent".into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::INT4OID.oid(), 0i32.into_datum()),
            ]),
        )
    });

    match result {
        Ok(_) => Some(pgrx::Uuid::from_bytes(*note_id.as_bytes())),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to insert note: {}", e);
            None
        }
    }
}

/// Get a note by ID.
/// Updates access_count and accessed_at timestamp on each read.
#[pg_extern]
fn caliber_note_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());

    Spi::connect(|client| {
        // First, get the note
        let result = client.select(
            "SELECT note_id, note_type, title, content, content_hash, embedding, 
                    source_trajectory_ids, source_artifact_ids, ttl, created_at, 
                    updated_at, accessed_at, access_count, superseded_by, metadata 
             FROM caliber_note WHERE note_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let note_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let note_type: Option<String> = row.get(2).ok().flatten();
                    let title: Option<String> = row.get(3).ok().flatten();
                    let content: Option<String> = row.get(4).ok().flatten();
                    let content_hash: Option<Vec<u8>> = row.get(5).ok().flatten();
                    // embedding is VECTOR type - skip for now
                    let source_trajectory_ids: Option<Vec<pgrx::Uuid>> = row.get(7).ok().flatten();
                    let source_artifact_ids: Option<Vec<pgrx::Uuid>> = row.get(8).ok().flatten();
                    let ttl: Option<String> = row.get(9).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(11).ok().flatten();
                    let accessed_at: Option<pgrx::TimestampWithTimeZone> = row.get(12).ok().flatten();
                    let access_count: Option<i32> = row.get(13).ok().flatten();
                    let superseded_by: Option<pgrx::Uuid> = row.get(14).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(15).ok().flatten();

                    // Update access tracking (increment access_count, update accessed_at)
                    let now = Utc::now();
                    let update_result = client.update(
                        "UPDATE caliber_note 
                         SET access_count = access_count + 1, accessed_at = $1 
                         WHERE note_id = $2",
                        None,
                        Some(vec![
                            (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                            (pgrx::PgBuiltInOids::UUIDOID.oid(), entity_id.into_datum()),
                        ]),
                    );

                    if let Err(e) = update_result {
                        pgrx::warning!("CALIBER: Failed to update note access tracking: {}", e);
                    }

                    Some(pgrx::JsonB(serde_json::json!({
                        "note_id": note_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "note_type": note_type,
                        "title": title,
                        "content": content,
                        "content_hash": content_hash.map(|h| hex::encode(h)),
                        "source_trajectory_ids": source_trajectory_ids.map(|ids| 
                            ids.iter().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()
                        ),
                        "source_artifact_ids": source_artifact_ids.map(|ids| 
                            ids.iter().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()
                        ),
                        "ttl": ttl,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                        "accessed_at": accessed_at.map(|t| format!("{:?}", t)),
                        "access_count": access_count,
                        "superseded_by": superseded_by.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "metadata": metadata.map(|j| j.0),
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get note: {}", e);
                None
            }
        }
    })
}

/// Query notes by trajectory.
/// Updates access_count and accessed_at for all returned notes.
#[pg_extern]
fn caliber_note_query_by_trajectory(trajectory_id: pgrx::Uuid) -> pgrx::JsonB {
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT note_id, note_type, title, content, content_hash, embedding, 
                    source_trajectory_ids, source_artifact_ids, ttl, created_at, 
                    updated_at, accessed_at, access_count, superseded_by, metadata 
             FROM caliber_note WHERE $1 = ANY(source_trajectory_ids) 
             ORDER BY created_at DESC",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), traj_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                let mut notes = Vec::new();
                let mut note_ids_to_update = Vec::new();
                
                for row in table {
                    let note_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let note_type: Option<String> = row.get(2).ok().flatten();
                    let title: Option<String> = row.get(3).ok().flatten();
                    let content: Option<String> = row.get(4).ok().flatten();
                    let content_hash: Option<Vec<u8>> = row.get(5).ok().flatten();
                    // embedding is VECTOR type - skip for now
                    let source_trajectory_ids: Option<Vec<pgrx::Uuid>> = row.get(7).ok().flatten();
                    let source_artifact_ids: Option<Vec<pgrx::Uuid>> = row.get(8).ok().flatten();
                    let ttl: Option<String> = row.get(9).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(11).ok().flatten();
                    let accessed_at: Option<pgrx::TimestampWithTimeZone> = row.get(12).ok().flatten();
                    let access_count: Option<i32> = row.get(13).ok().flatten();
                    let superseded_by: Option<pgrx::Uuid> = row.get(14).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(15).ok().flatten();

                    if let Some(nid) = note_id {
                        note_ids_to_update.push(Uuid::from_bytes(*nid.as_bytes()));
                    }

                    notes.push(serde_json::json!({
                        "note_id": note_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "note_type": note_type,
                        "title": title,
                        "content": content,
                        "content_hash": content_hash.map(|h| hex::encode(h)),
                        "source_trajectory_ids": source_trajectory_ids.map(|ids| 
                            ids.iter().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()
                        ),
                        "source_artifact_ids": source_artifact_ids.map(|ids| 
                            ids.iter().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()
                        ),
                        "ttl": ttl,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                        "accessed_at": accessed_at.map(|t| format!("{:?}", t)),
                        "access_count": access_count,
                        "superseded_by": superseded_by.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "metadata": metadata.map(|j| j.0),
                    }));
                }

                // Update access tracking for all returned notes
                if !note_ids_to_update.is_empty() {
                    let now = Utc::now();
                    for note_id in note_ids_to_update {
                        let update_result = client.update(
                            "UPDATE caliber_note 
                             SET access_count = access_count + 1, accessed_at = $1 
                             WHERE note_id = $2",
                            None,
                            Some(vec![
                                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                                (pgrx::PgBuiltInOids::UUIDOID.oid(), note_id.into_datum()),
                            ]),
                        );

                        if let Err(e) = update_result {
                            pgrx::warning!("CALIBER: Failed to update note access tracking for {}: {}", note_id, e);
                        }
                    }
                }

                pgrx::JsonB(serde_json::json!(notes))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to query notes by trajectory: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
}


// ============================================================================
// TURN OPERATIONS (Task 12.3)
// ============================================================================

/// Create a new turn in a scope.
/// Verifies scope_id exists before insert.
/// Returns error on duplicate (scope_id, sequence) via UNIQUE constraint.
/// Returns None if role is invalid.
#[pg_extern]
fn caliber_turn_create(
    scope_id: pgrx::Uuid,
    sequence: i32,
    role: &str,
    content: &str,
    token_count: i32,
) -> Option<pgrx::Uuid> {
    let turn_id = new_entity_id();
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());
    let now = Utc::now();

    // Validate role - reject unknown values instead of defaulting (REQ-12)
    let role_str = match role {
        "user" | "assistant" | "system" | "tool" => role,
        _ => {
            let validation_err = ValidationError::InvalidValue {
                field: "role".to_string(),
                reason: format!("unknown value '{}'. Valid values: user, assistant, system, tool", role),
            };
            pgrx::warning!("CALIBER: {:?}", validation_err);
            return None;
        }
    };

    // Insert into SQL table via SPI
    let result = Spi::connect(|client| {
        // First, verify scope exists
        let scope_check = client.select(
            "SELECT scope_id FROM caliber_scope WHERE scope_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), scp_id.into_datum()),
            ]),
        );

        match scope_check {
            Ok(table) => {
                if table.first().is_none() {
                    pgrx::warning!("CALIBER: Scope {} does not exist", scp_id);
                    return Err(spi::Error::InvalidPosition);
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to verify scope existence: {}", e);
                return Err(e);
            }
        }

        // Insert turn (UNIQUE constraint on (scope_id, sequence) will catch duplicates)
        client.update(
            "INSERT INTO caliber_turn (turn_id, scope_id, sequence, role, content, token_count, created_at) 
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), turn_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), scp_id.into_datum()),
                (pgrx::PgBuiltInOids::INT4OID.oid(), sequence.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), role_str.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), content.into_datum()),
                (pgrx::PgBuiltInOids::INT4OID.oid(), token_count.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
            ]),
        )
    });

    match result {
        Ok(_) => Some(pgrx::Uuid::from_bytes(*turn_id.as_bytes())),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to insert turn: {}", e);
            None
        }
    }
}

/// Get turns by scope.
#[pg_extern]
fn caliber_turn_get_by_scope(scope_id: pgrx::Uuid) -> pgrx::JsonB {
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT turn_id, scope_id, sequence, role, content, token_count, created_at, 
                    tool_calls, tool_results, metadata 
             FROM caliber_turn 
             WHERE scope_id = $1 
             ORDER BY sequence ASC",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), scp_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                let mut turns = Vec::new();
                for row in table {
                    let turn_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let scope_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let sequence: Option<i32> = row.get(3).ok().flatten();
                    let role: Option<String> = row.get(4).ok().flatten();
                    let content: Option<String> = row.get(5).ok().flatten();
                    let token_count: Option<i32> = row.get(6).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(7).ok().flatten();
                    let tool_calls: Option<pgrx::JsonB> = row.get(8).ok().flatten();
                    let tool_results: Option<pgrx::JsonB> = row.get(9).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(10).ok().flatten();

                    turns.push(serde_json::json!({
                        "turn_id": turn_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "sequence": sequence,
                        "role": role,
                        "content": content,
                        "token_count": token_count,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "tool_calls": tool_calls.map(|j| j.0),
                        "tool_results": tool_results.map(|j| j.0),
                        "metadata": metadata.map(|j| j.0),
                    }));
                }
                pgrx::JsonB(serde_json::json!(turns))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get turns by scope: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
}


// ============================================================================
// ADVISORY LOCK FUNCTIONS (Task 12.4)
// ============================================================================

/// Lock level for advisory locks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisoryLockLevel {
    /// Session-level lock - persists until explicit release or session ends
    Session,
    /// Transaction-level lock - auto-releases at transaction commit/rollback
    Transaction,
}

/// Acquire an advisory lock on a resource.
/// Uses Postgres advisory locks for distributed coordination.
/// Stores lock record in SQL table for cross-session visibility.
/// 
/// Parameters:
/// - agent_id: The agent acquiring the lock
/// - resource_type: Type of resource being locked
/// - resource_id: ID of the resource being locked
/// - timeout_ms: Lock expiration timeout in milliseconds
/// - mode: "exclusive" or "shared"
/// - level: "session" or "transaction" (defaults to "transaction")
#[pg_extern]
fn caliber_lock_acquire(
    agent_id: pgrx::Uuid,
    resource_type: &str,
    resource_id: pgrx::Uuid,
    timeout_ms: i64,
    mode: &str,
    level: Option<&str>,
) -> Option<pgrx::Uuid> {
    let agent = Uuid::from_bytes(*agent_id.as_bytes());
    let resource = Uuid::from_bytes(*resource_id.as_bytes());
    let lock_key = compute_lock_key(resource_type, resource);

    let lock_mode = match mode {
        "shared" => LockMode::Shared,
        _ => LockMode::Exclusive,
    };

    let lock_level = match level.unwrap_or("transaction") {
        "session" => AdvisoryLockLevel::Session,
        _ => AdvisoryLockLevel::Transaction,
    };

    // Try to acquire Postgres advisory lock based on level and mode
    let acquired = match (lock_level, lock_mode) {
        (AdvisoryLockLevel::Session, LockMode::Exclusive) => unsafe {
            pgrx::pg_sys::pg_try_advisory_lock(lock_key)
        },
        (AdvisoryLockLevel::Session, LockMode::Shared) => unsafe {
            pgrx::pg_sys::pg_try_advisory_lock_shared(lock_key)
        },
        (AdvisoryLockLevel::Transaction, LockMode::Exclusive) => unsafe {
            pgrx::pg_sys::pg_try_advisory_xact_lock(lock_key)
        },
        (AdvisoryLockLevel::Transaction, LockMode::Shared) => unsafe {
            pgrx::pg_sys::pg_try_advisory_xact_lock_shared(lock_key)
        },
    };

    if acquired {
        // Create lock record in SQL table for cross-session visibility
        let lock_id = new_entity_id();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::milliseconds(timeout_ms);
        let mode_str = if lock_mode == LockMode::Exclusive { "exclusive" } else { "shared" };

        let result = Spi::connect(|client| {
            client.update(
                "INSERT INTO caliber_lock (lock_id, resource_type, resource_id, holder_agent_id, acquired_at, expires_at, mode)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                None,
                Some(vec![
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), lock_id.into_datum()),
                    (pgrx::PgBuiltInOids::TEXTOID.oid(), resource_type.into_datum()),
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), resource.into_datum()),
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), agent.into_datum()),
                    (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                    (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), expires_at.into_datum()),
                    (pgrx::PgBuiltInOids::TEXTOID.oid(), mode_str.into_datum()),
                ]),
            )
        });

        match result {
            Ok(_) => Some(pgrx::Uuid::from_bytes(*lock_id.as_bytes())),
            Err(e) => {
                // Use EntityType::Lock for proper error categorization
                let storage_err = StorageError::InsertFailed {
                    entity_type: EntityType::Lock,
                    reason: e.to_string(),
                };
                pgrx::warning!("CALIBER: {:?}", storage_err);
                // Release the advisory lock since we couldn't record it
                match (lock_level, lock_mode) {
                    (AdvisoryLockLevel::Session, LockMode::Exclusive) => unsafe {
                        pgrx::pg_sys::pg_advisory_unlock(lock_key);
                    },
                    (AdvisoryLockLevel::Session, LockMode::Shared) => unsafe {
                        pgrx::pg_sys::pg_advisory_unlock_shared(lock_key);
                    },
                    // Transaction locks auto-release, no explicit unlock needed
                    _ => {}
                }
                None
            }
        }
    } else {
        None
    }
}

/// Release an advisory lock.
/// Only works for session-level locks. Transaction locks auto-release.
#[pg_extern]
fn caliber_lock_release(lock_id: pgrx::Uuid) -> bool {
    let lid = Uuid::from_bytes(*lock_id.as_bytes());

    // Get lock info from SQL table
    let lock_info = Spi::connect(|client| {
        let result = client.select(
            "SELECT resource_type, resource_id, mode FROM caliber_lock WHERE lock_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), lid.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let resource_type: Option<String> = row.get(1).ok().flatten();
                    let resource_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let mode: Option<String> = row.get(3).ok().flatten();
                    
                    if let (Some(rt), Some(rid), Some(m)) = (resource_type, resource_id, mode) {
                        Some((rt, Uuid::from_bytes(*rid.as_bytes()), m))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    });

    if let Some((resource_type, resource_id, mode)) = lock_info {
        let lock_key = compute_lock_key(&resource_type, resource_id);

        // Release Postgres advisory lock (session-level)
        if mode == "exclusive" {
            unsafe {
                pgrx::pg_sys::pg_advisory_unlock(lock_key);
            }
        } else {
            unsafe {
                pgrx::pg_sys::pg_advisory_unlock_shared(lock_key);
            }
        }

        // Delete lock record from SQL table
        let delete_result = Spi::connect(|client| {
            client.update(
                "DELETE FROM caliber_lock WHERE lock_id = $1",
                None,
                Some(vec![
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), lid.into_datum()),
                ]),
            )
        });

        match delete_result {
            Ok(r) => r.rows_affected() > 0,
            Err(e) => {
                // Use EntityType::Lock for proper error categorization
                let storage_err = StorageError::UpdateFailed {
                    entity_type: EntityType::Lock,
                    id: lid,
                    reason: format!("delete failed: {}", e),
                };
                pgrx::warning!("CALIBER: {:?}", storage_err);
                false
            }
        }
    } else {
        // Lock not found in SQL table
        let storage_err = StorageError::NotFound {
            entity_type: EntityType::Lock,
            id: lid,
        };
        pgrx::warning!("CALIBER: {:?}", storage_err);
        false
    }
}

/// Check if a resource is locked.
#[pg_extern]
fn caliber_lock_check(resource_type: &str, resource_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let resource = Uuid::from_bytes(*resource_id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT lock_id, resource_type, resource_id, holder_agent_id, acquired_at, expires_at, mode
             FROM caliber_lock 
             WHERE resource_type = $1 AND resource_id = $2 AND expires_at > NOW()",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::TEXTOID.oid(), resource_type.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), resource.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let lock_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let resource_type: Option<String> = row.get(2).ok().flatten();
                    let resource_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let holder_agent_id: Option<pgrx::Uuid> = row.get(4).ok().flatten();
                    let acquired_at: Option<pgrx::TimestampWithTimeZone> = row.get(5).ok().flatten();
                    let expires_at: Option<pgrx::TimestampWithTimeZone> = row.get(6).ok().flatten();
                    let mode: Option<String> = row.get(7).ok().flatten();

                    Some(pgrx::JsonB(serde_json::json!({
                        "lock_id": lock_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "resource_type": resource_type,
                        "resource_id": resource_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "holder_agent_id": holder_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "acquired_at": acquired_at.map(|t| format!("{:?}", t)),
                        "expires_at": expires_at.map(|t| format!("{:?}", t)),
                        "mode": mode,
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                // Use EntityType::Lock for proper error categorization
                let storage_err = StorageError::SpiError { reason: format!("lock check failed: {}", e) };
                pgrx::warning!("CALIBER: EntityType::Lock - {:?}", storage_err);
                None
            }
        }
    })
}

/// Get lock by ID.
#[pg_extern]
fn caliber_lock_get(lock_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let lid = Uuid::from_bytes(*lock_id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT lock_id, resource_type, resource_id, holder_agent_id, acquired_at, expires_at, mode
             FROM caliber_lock WHERE lock_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), lid.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let lock_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let resource_type: Option<String> = row.get(2).ok().flatten();
                    let resource_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let holder_agent_id: Option<pgrx::Uuid> = row.get(4).ok().flatten();
                    let acquired_at: Option<pgrx::TimestampWithTimeZone> = row.get(5).ok().flatten();
                    let expires_at: Option<pgrx::TimestampWithTimeZone> = row.get(6).ok().flatten();
                    let mode: Option<String> = row.get(7).ok().flatten();

                    Some(pgrx::JsonB(serde_json::json!({
                        "lock_id": lock_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "resource_type": resource_type,
                        "resource_id": resource_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "holder_agent_id": holder_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "acquired_at": acquired_at.map(|t| format!("{:?}", t)),
                        "expires_at": expires_at.map(|t| format!("{:?}", t)),
                        "mode": mode,
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                // Use EntityType::Lock for proper error categorization
                let storage_err = StorageError::SpiError { reason: format!("lock get failed: {}", e) };
                pgrx::warning!("CALIBER: EntityType::Lock - {:?}", storage_err);
                None
            }
        }
    })
}


// ============================================================================
// NOTIFY-BASED MESSAGE PASSING (Task 12.5)
// ============================================================================

/// Send a message to an agent.
/// Stores message in SQL table and triggers pg_notify via database trigger.
/// Returns None if message_type or priority is invalid.
#[pg_extern]
fn caliber_message_send(
    from_agent_id: pgrx::Uuid,
    to_agent_id: Option<pgrx::Uuid>,
    to_agent_type: Option<&str>,
    message_type: &str,
    payload: &str,
    priority: &str,
) -> Option<pgrx::Uuid> {
    let from_agent = Uuid::from_bytes(*from_agent_id.as_bytes());
    let to_agent = to_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()));

    // Validate message_type - reject unknown values
    let valid_message_type = match message_type {
        "task_delegation" | "task_result" | "context_request" | "context_share" 
        | "coordination_signal" | "handoff" | "interrupt" | "heartbeat" => message_type,
        _ => {
            pgrx::warning!("CALIBER: Invalid message_type '{}'. Valid values: task_delegation, task_result, context_request, context_share, coordination_signal, handoff, interrupt, heartbeat", message_type);
            return None;
        }
    };

    // Validate priority - reject unknown values
    let valid_priority = match priority {
        "low" | "normal" | "high" | "critical" => priority,
        _ => {
            pgrx::warning!("CALIBER: Invalid priority '{}'. Valid values: low, normal, high, critical", priority);
            return None;
        }
    };

    let message_id = new_entity_id();
    let now = Utc::now();

    // Insert into SQL table via SPI
    // The database trigger will handle pg_notify automatically
    let result = Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber_message (message_id, from_agent_id, to_agent_id, to_agent_type, 
                                          message_type, payload, created_at, priority) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), message_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), from_agent.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), to_agent.map(|u| pgrx::Uuid::from_bytes(*u.as_bytes())).into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), to_agent_type.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), valid_message_type.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), payload.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), valid_priority.into_datum()),
            ]),
        )
    });

    match result {
        Ok(_) => Some(pgrx::Uuid::from_bytes(*message_id.as_bytes())),
        Err(e) => {
            // Use EntityType::Message for proper error categorization
            let storage_err = StorageError::InsertFailed {
                entity_type: EntityType::Message,
                reason: e.to_string(),
            };
            pgrx::warning!("CALIBER: {:?}", storage_err);
            None
        }
    }
}

/// Get a message by ID.
#[pg_extern]
fn caliber_message_get(message_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let mid = Uuid::from_bytes(*message_id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT message_id, from_agent_id, to_agent_id, to_agent_type, message_type, 
                    payload, trajectory_id, scope_id, artifact_ids, created_at, 
                    delivered_at, acknowledged_at, priority, expires_at
             FROM caliber_message WHERE message_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), mid.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let message_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let from_agent_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let to_agent_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let to_agent_type: Option<String> = row.get(4).ok().flatten();
                    let message_type: Option<String> = row.get(5).ok().flatten();
                    let payload: Option<String> = row.get(6).ok().flatten();
                    let trajectory_id: Option<pgrx::Uuid> = row.get(7).ok().flatten();
                    let scope_id: Option<pgrx::Uuid> = row.get(8).ok().flatten();
                    let artifact_ids: Option<Vec<pgrx::Uuid>> = row.get(9).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let delivered_at: Option<pgrx::TimestampWithTimeZone> = row.get(11).ok().flatten();
                    let acknowledged_at: Option<pgrx::TimestampWithTimeZone> = row.get(12).ok().flatten();
                    let priority: Option<String> = row.get(13).ok().flatten();
                    let expires_at: Option<pgrx::TimestampWithTimeZone> = row.get(14).ok().flatten();

                    Some(pgrx::JsonB(serde_json::json!({
                        "message_id": message_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "from_agent_id": from_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "to_agent_id": to_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "to_agent_type": to_agent_type,
                        "message_type": message_type,
                        "payload": payload,
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "artifact_ids": artifact_ids.map(|ids| ids.iter().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()),
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "delivered_at": delivered_at.map(|t| format!("{:?}", t)),
                        "acknowledged_at": acknowledged_at.map(|t| format!("{:?}", t)),
                        "priority": priority,
                        "expires_at": expires_at.map(|t| format!("{:?}", t)),
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                // Use EntityType::Message for proper error categorization
                let storage_err = StorageError::SpiError { reason: format!("message get failed: {}", e) };
                pgrx::warning!("CALIBER: EntityType::Message - {:?}", storage_err);
                None
            }
        }
    })
}

/// Mark a message as delivered.
#[pg_extern]
fn caliber_message_mark_delivered(message_id: pgrx::Uuid) -> bool {
    let mid = Uuid::from_bytes(*message_id.as_bytes());
    let now = Utc::now();

    let result = Spi::connect(|client| {
        client.update(
            "UPDATE caliber_message SET delivered_at = $1 WHERE message_id = $2 AND delivered_at IS NULL",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), mid.into_datum()),
            ]),
        )
    });

    match result {
        Ok(update_result) => update_result.rows_affected() > 0,
        Err(e) => {
            // Use EntityType::Message for proper error categorization
            let storage_err = StorageError::UpdateFailed {
                entity_type: EntityType::Message,
                id: mid,
                reason: format!("mark delivered failed: {}", e),
            };
            pgrx::warning!("CALIBER: {:?}", storage_err);
            false
        }
    }
}

/// Mark a message as acknowledged.
#[pg_extern]
fn caliber_message_mark_acknowledged(message_id: pgrx::Uuid) -> bool {
    let mid = Uuid::from_bytes(*message_id.as_bytes());
    let now = Utc::now();

    let result = Spi::connect(|client| {
        client.update(
            "UPDATE caliber_message SET acknowledged_at = $1 WHERE message_id = $2 AND acknowledged_at IS NULL",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), mid.into_datum()),
            ]),
        )
    });

    match result {
        Ok(update_result) => update_result.rows_affected() > 0,
        Err(e) => {
            // Use EntityType::Message for proper error categorization
            let storage_err = StorageError::UpdateFailed {
                entity_type: EntityType::Message,
                id: mid,
                reason: format!("mark acknowledged failed: {}", e),
            };
            pgrx::warning!("CALIBER: {:?}", storage_err);
            false
        }
    }
}

/// Get pending messages for an agent.
/// Returns messages where delivered_at IS NULL and not expired.
#[pg_extern]
fn caliber_message_get_pending(agent_id: pgrx::Uuid, agent_type: &str) -> pgrx::JsonB {
    let aid = Uuid::from_bytes(*agent_id.as_bytes());

    Spi::connect(|client| {
        // Query for messages to this specific agent OR to this agent type OR broadcast
        let result = client.select(
            "SELECT message_id, from_agent_id, to_agent_id, to_agent_type, message_type, 
                    payload, trajectory_id, scope_id, artifact_ids, created_at, 
                    delivered_at, acknowledged_at, priority, expires_at
             FROM caliber_message 
             WHERE delivered_at IS NULL 
               AND (expires_at IS NULL OR expires_at > NOW())
               AND (to_agent_id = $1 OR to_agent_type = $2 OR to_agent_type = '*')
             ORDER BY 
               CASE priority 
                 WHEN 'critical' THEN 1 
                 WHEN 'high' THEN 2 
                 WHEN 'normal' THEN 3 
                 WHEN 'low' THEN 4 
               END,
               created_at ASC",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), aid.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), agent_type.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                let mut messages = Vec::new();
                for row in table {
                    let message_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let from_agent_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let to_agent_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let to_agent_type: Option<String> = row.get(4).ok().flatten();
                    let message_type: Option<String> = row.get(5).ok().flatten();
                    let payload: Option<String> = row.get(6).ok().flatten();
                    let trajectory_id: Option<pgrx::Uuid> = row.get(7).ok().flatten();
                    let scope_id: Option<pgrx::Uuid> = row.get(8).ok().flatten();
                    let artifact_ids: Option<Vec<pgrx::Uuid>> = row.get(9).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let delivered_at: Option<pgrx::TimestampWithTimeZone> = row.get(11).ok().flatten();
                    let acknowledged_at: Option<pgrx::TimestampWithTimeZone> = row.get(12).ok().flatten();
                    let priority: Option<String> = row.get(13).ok().flatten();
                    let expires_at: Option<pgrx::TimestampWithTimeZone> = row.get(14).ok().flatten();

                    messages.push(serde_json::json!({
                        "message_id": message_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "from_agent_id": from_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "to_agent_id": to_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "to_agent_type": to_agent_type,
                        "message_type": message_type,
                        "payload": payload,
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "artifact_ids": artifact_ids.map(|ids| ids.iter().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()),
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "delivered_at": delivered_at.map(|t| format!("{:?}", t)),
                        "acknowledged_at": acknowledged_at.map(|t| format!("{:?}", t)),
                        "priority": priority,
                        "expires_at": expires_at.map(|t| format!("{:?}", t)),
                    }));
                }
                pgrx::JsonB(serde_json::json!(messages))
            }
            Err(e) => {
                // Use EntityType::Message for proper error categorization
                let storage_err = StorageError::SpiError { reason: format!("get pending messages failed: {}", e) };
                pgrx::warning!("CALIBER: EntityType::Message - {:?}", storage_err);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
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

    // Insert via SPI
    Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber_agent (agent_id, agent_type, capabilities, memory_access, status, can_delegate_to, created_at, last_heartbeat)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), pgrx::Uuid::from_bytes(*agent_id.as_bytes()).into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), agent_type.into_datum()),
                (pgrx::PgBuiltInOids::TEXTARRAYOID.oid(), caps.into_datum()),
                (pgrx::PgBuiltInOids::JSONBOID.oid(), pgrx::JsonB(safe_to_json(&agent.memory_access)).into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), "idle".into_datum()),
                (pgrx::PgBuiltInOids::TEXTARRAYOID.oid(), agent.can_delegate_to.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), agent.created_at.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), agent.last_heartbeat.into_datum()),
            ]),
        )
    }).unwrap_or_else(|e| {
        pgrx::warning!("CALIBER: Failed to insert agent: {}", e);
    });

    pgrx::Uuid::from_bytes(*agent_id.as_bytes())
}

/// Get an agent by ID.
#[pg_extern]
fn caliber_agent_get(agent_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT agent_id, agent_type, capabilities, memory_access, status,
                    current_trajectory_id, current_scope_id, can_delegate_to,
                    reports_to, created_at, last_heartbeat
             FROM caliber_agent WHERE agent_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), agent_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let agent_json = serde_json::json!({
                        "agent_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_type": row.get::<String>(2).ok().flatten(),
                        "capabilities": row.get::<Vec<String>>(3).ok().flatten().unwrap_or_default(),
                        "memory_access": row.get::<pgrx::JsonB>(4).ok().flatten().map(|j| j.0).unwrap_or(serde_json::json!({})),
                        "status": row.get::<String>(5).ok().flatten(),
                        "current_trajectory_id": row.get::<pgrx::Uuid>(6).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "current_scope_id": row.get::<pgrx::Uuid>(7).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "can_delegate_to": row.get::<Vec<String>>(8).ok().flatten().unwrap_or_default(),
                        "reports_to": row.get::<pgrx::Uuid>(9).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "created_at": row.get::<pgrx::TimestampWithTimeZone>(10).ok().flatten().map(|t| t.to_string()),
                        "last_heartbeat": row.get::<pgrx::TimestampWithTimeZone>(11).ok().flatten().map(|t| t.to_string()),
                    });
                    Some(pgrx::JsonB(agent_json))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get agent: {}", e);
                None
            }
        }
    })
}

/// Update agent status.
#[pg_extern]
fn caliber_agent_set_status(agent_id: pgrx::Uuid, status: &str) -> bool {
    // Validate status value
    if !["idle", "active", "blocked", "failed"].contains(&status) {
        return false;
    }

    Spi::connect(|client| {
        match client.update(
            "UPDATE caliber_agent SET status = $2 WHERE agent_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), agent_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), status.into_datum()),
            ]),
        ) {
            Ok(count) => count > 0,
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to update agent status: {}", e);
                false
            }
        }
    })
}

/// Update agent heartbeat.
#[pg_extern]
fn caliber_agent_heartbeat(agent_id: pgrx::Uuid) -> bool {
    Spi::connect(|client| {
        match client.update(
            "UPDATE caliber_agent SET last_heartbeat = NOW() WHERE agent_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), agent_id.into_datum()),
            ]),
        ) {
            Ok(count) => count > 0,
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to update agent heartbeat: {}", e);
                false
            }
        }
    })
}

/// List agents by type.
#[pg_extern]
fn caliber_agent_list_by_type(agent_type: &str) -> pgrx::JsonB {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT agent_id, agent_type, capabilities, status, created_at, last_heartbeat
             FROM caliber_agent WHERE agent_type = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::TEXTOID.oid(), agent_type.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                let agents: Vec<serde_json::Value> = table.into_iter().map(|row| {
                    serde_json::json!({
                        "agent_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_type": row.get::<String>(2).ok().flatten(),
                        "capabilities": row.get::<Vec<String>>(3).ok().flatten().unwrap_or_default(),
                        "status": row.get::<String>(4).ok().flatten(),
                        "created_at": row.get::<pgrx::TimestampWithTimeZone>(5).ok().flatten().map(|t| t.to_string()),
                        "last_heartbeat": row.get::<pgrx::TimestampWithTimeZone>(6).ok().flatten().map(|t| t.to_string()),
                    })
                }).collect();
                pgrx::JsonB(serde_json::json!(agents))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to list agents by type: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
}

/// List all active agents.
#[pg_extern]
fn caliber_agent_list_active() -> pgrx::JsonB {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT agent_id, agent_type, capabilities, status, created_at, last_heartbeat
             FROM caliber_agent WHERE status IN ('active', 'idle')",
            None,
            None,
        );

        match result {
            Ok(table) => {
                let agents: Vec<serde_json::Value> = table.into_iter().map(|row| {
                    serde_json::json!({
                        "agent_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_type": row.get::<String>(2).ok().flatten(),
                        "capabilities": row.get::<Vec<String>>(3).ok().flatten().unwrap_or_default(),
                        "status": row.get::<String>(4).ok().flatten(),
                        "created_at": row.get::<pgrx::TimestampWithTimeZone>(5).ok().flatten().map(|t| t.to_string()),
                        "last_heartbeat": row.get::<pgrx::TimestampWithTimeZone>(6).ok().flatten().map(|t| t.to_string()),
                    })
                }).collect();
                pgrx::JsonB(serde_json::json!(agents))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to list active agents: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
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

    // Insert via SPI
    Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber_delegation (delegation_id, delegator_agent_id, delegatee_agent_id, delegatee_agent_type, task_description, parent_trajectory_id, status, constraints, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), pgrx::Uuid::from_bytes(*delegation_id.as_bytes()).into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), delegator_agent_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), delegatee_agent_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), delegatee_agent_type.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), task_description.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), parent_trajectory_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), "pending".into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), "{}".into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), delegation.created_at.into_datum()),
            ]),
        )
    }).unwrap_or_else(|e| {
        pgrx::warning!("CALIBER: Failed to insert delegation: {}", e);
    });

    pgrx::Uuid::from_bytes(*delegation_id.as_bytes())
}

/// Get a delegation by ID.
#[pg_extern]
fn caliber_delegation_get(delegation_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT delegation_id, delegator_agent_id, delegatee_agent_id, delegatee_agent_type,
                    task_description, parent_trajectory_id, child_trajectory_id, status, result,
                    created_at, accepted_at, completed_at
             FROM caliber_delegation WHERE delegation_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), delegation_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let json = serde_json::json!({
                        "delegation_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "delegator_agent_id": row.get::<pgrx::Uuid>(2).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "delegatee_agent_id": row.get::<pgrx::Uuid>(3).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "delegatee_agent_type": row.get::<String>(4).ok().flatten(),
                        "task_description": row.get::<String>(5).ok().flatten(),
                        "parent_trajectory_id": row.get::<pgrx::Uuid>(6).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "child_trajectory_id": row.get::<pgrx::Uuid>(7).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "status": row.get::<String>(8).ok().flatten(),
                        "result": row.get::<pgrx::JsonB>(9).ok().flatten().map(|j| j.0),
                        "created_at": row.get::<pgrx::TimestampWithTimeZone>(10).ok().flatten().map(|t| t.to_string()),
                        "accepted_at": row.get::<pgrx::TimestampWithTimeZone>(11).ok().flatten().map(|t| t.to_string()),
                        "completed_at": row.get::<pgrx::TimestampWithTimeZone>(12).ok().flatten().map(|t| t.to_string()),
                    });
                    Some(pgrx::JsonB(json))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get delegation: {}", e);
                None
            }
        }
    })
}

/// Accept a delegation.
#[pg_extern]
fn caliber_delegation_accept(
    delegation_id: pgrx::Uuid,
    delegatee_agent_id: pgrx::Uuid,
    child_trajectory_id: pgrx::Uuid,
) -> bool {
    Spi::connect(|client| {
        match client.update(
            "UPDATE caliber_delegation SET delegatee_agent_id = $2, child_trajectory_id = $3, status = 'accepted', accepted_at = NOW()
             WHERE delegation_id = $1 AND status = 'pending'",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), delegation_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), delegatee_agent_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), child_trajectory_id.into_datum()),
            ]),
        ) {
            Ok(count) => count > 0,
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to accept delegation: {}", e);
                false
            }
        }
    })
}

/// Complete a delegation.
#[pg_extern]
fn caliber_delegation_complete(
    delegation_id: pgrx::Uuid,
    success: bool,
    summary: &str,
) -> bool {
    let status = if success { "completed" } else { "failed" };
    let result_json = if success {
        serde_json::json!({
            "status": "Success",
            "produced_artifacts": [],
            "produced_notes": [],
            "summary": summary,
            "error": null
        })
    } else {
        serde_json::json!({
            "status": "Failure",
            "produced_artifacts": [],
            "produced_notes": [],
            "summary": "",
            "error": summary
        })
    };

    Spi::connect(|client| {
        match client.update(
            "UPDATE caliber_delegation SET status = $2, result = $3, completed_at = NOW()
             WHERE delegation_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), delegation_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), status.into_datum()),
                (pgrx::PgBuiltInOids::JSONBOID.oid(), pgrx::JsonB(result_json).into_datum()),
            ]),
        ) {
            Ok(count) => count > 0,
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to complete delegation: {}", e);
                false
            }
        }
    })
}

/// List pending delegations for an agent type.
#[pg_extern]
fn caliber_delegation_list_pending(agent_type: &str) -> pgrx::JsonB {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT delegation_id, delegator_agent_id, delegatee_agent_type, task_description, created_at
             FROM caliber_delegation
             WHERE status = 'pending' AND (delegatee_agent_type = $1 OR delegatee_agent_type = '*')",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::TEXTOID.oid(), agent_type.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                let delegations: Vec<serde_json::Value> = table.into_iter().map(|row| {
                    serde_json::json!({
                        "delegation_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "delegator_agent_id": row.get::<pgrx::Uuid>(2).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "delegatee_agent_type": row.get::<String>(3).ok().flatten(),
                        "task_description": row.get::<String>(4).ok().flatten(),
                        "created_at": row.get::<pgrx::TimestampWithTimeZone>(5).ok().flatten().map(|t| t.to_string()),
                    })
                }).collect();
                pgrx::JsonB(serde_json::json!(delegations))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to list pending delegations: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
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

    // Insert via SPI
    Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber_handoff (handoff_id, from_agent_id, to_agent_id, to_agent_type, trajectory_id, scope_id, context_snapshot_id, reason, status, initiated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'initiated', $9)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), pgrx::Uuid::from_bytes(*handoff_id.as_bytes()).into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), from_agent_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), to_agent_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), to_agent_type.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), trajectory_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), scope_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), context_snapshot_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), reason.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), handoff.initiated_at.into_datum()),
            ]),
        )
    }).unwrap_or_else(|e| {
        pgrx::warning!("CALIBER: Failed to insert handoff: {}", e);
    });

    pgrx::Uuid::from_bytes(*handoff_id.as_bytes())
}

/// Get a handoff by ID.
#[pg_extern]
fn caliber_handoff_get(handoff_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT handoff_id, from_agent_id, to_agent_id, to_agent_type, trajectory_id, scope_id,
                    context_snapshot_id, handoff_notes, status, reason, initiated_at, accepted_at, completed_at
             FROM caliber_handoff WHERE handoff_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), handoff_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let json = serde_json::json!({
                        "handoff_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "from_agent_id": row.get::<pgrx::Uuid>(2).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "to_agent_id": row.get::<pgrx::Uuid>(3).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "to_agent_type": row.get::<String>(4).ok().flatten(),
                        "trajectory_id": row.get::<pgrx::Uuid>(5).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "scope_id": row.get::<pgrx::Uuid>(6).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "context_snapshot_id": row.get::<pgrx::Uuid>(7).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "handoff_notes": row.get::<String>(8).ok().flatten(),
                        "status": row.get::<String>(9).ok().flatten(),
                        "reason": row.get::<String>(10).ok().flatten(),
                        "initiated_at": row.get::<pgrx::TimestampWithTimeZone>(11).ok().flatten().map(|t| t.to_string()),
                        "accepted_at": row.get::<pgrx::TimestampWithTimeZone>(12).ok().flatten().map(|t| t.to_string()),
                        "completed_at": row.get::<pgrx::TimestampWithTimeZone>(13).ok().flatten().map(|t| t.to_string()),
                    });
                    Some(pgrx::JsonB(json))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get handoff: {}", e);
                None
            }
        }
    })
}

/// Accept a handoff.
#[pg_extern]
fn caliber_handoff_accept(handoff_id: pgrx::Uuid, accepting_agent_id: pgrx::Uuid) -> bool {
    Spi::connect(|client| {
        match client.update(
            "UPDATE caliber_handoff SET to_agent_id = $2, status = 'accepted', accepted_at = NOW()
             WHERE handoff_id = $1 AND status = 'initiated'",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), handoff_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), accepting_agent_id.into_datum()),
            ]),
        ) {
            Ok(count) => count > 0,
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to accept handoff: {}", e);
                false
            }
        }
    })
}

/// Complete a handoff.
#[pg_extern]
fn caliber_handoff_complete(handoff_id: pgrx::Uuid) -> bool {
    Spi::connect(|client| {
        match client.update(
            "UPDATE caliber_handoff SET status = 'completed', completed_at = NOW()
             WHERE handoff_id = $1 AND status = 'accepted'",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), handoff_id.into_datum()),
            ]),
        ) {
            Ok(count) => count > 0,
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to complete handoff: {}", e);
                false
            }
        }
    })
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

    // Insert via SPI
    Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber_conflict (conflict_id, conflict_type, item_a_type, item_a_id, item_b_type, item_b_id, status, detected_at)
             VALUES ($1, $2, $3, $4, $5, $6, 'detected', $7)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), pgrx::Uuid::from_bytes(*conflict_id.as_bytes()).into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), conflict_type.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), item_a_type.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), item_a_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), item_b_type.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), item_b_id.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), conflict.detected_at.into_datum()),
            ]),
        )
    }).unwrap_or_else(|e| {
        pgrx::warning!("CALIBER: Failed to insert conflict: {}", e);
    });

    pgrx::Uuid::from_bytes(*conflict_id.as_bytes())
}

/// Get a conflict by ID.
#[pg_extern]
fn caliber_conflict_get(conflict_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT conflict_id, conflict_type, item_a_type, item_a_id, item_b_type, item_b_id,
                    agent_a_id, agent_b_id, trajectory_id, status, resolution, detected_at, resolved_at
             FROM caliber_conflict WHERE conflict_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), conflict_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let json = serde_json::json!({
                        "conflict_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "conflict_type": row.get::<String>(2).ok().flatten(),
                        "item_a_type": row.get::<String>(3).ok().flatten(),
                        "item_a_id": row.get::<pgrx::Uuid>(4).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "item_b_type": row.get::<String>(5).ok().flatten(),
                        "item_b_id": row.get::<pgrx::Uuid>(6).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_a_id": row.get::<pgrx::Uuid>(7).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_b_id": row.get::<pgrx::Uuid>(8).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "trajectory_id": row.get::<pgrx::Uuid>(9).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "status": row.get::<String>(10).ok().flatten(),
                        "resolution": row.get::<pgrx::JsonB>(11).ok().flatten().map(|j| j.0),
                        "detected_at": row.get::<pgrx::TimestampWithTimeZone>(12).ok().flatten().map(|t| t.to_string()),
                        "resolved_at": row.get::<pgrx::TimestampWithTimeZone>(13).ok().flatten().map(|t| t.to_string()),
                    });
                    Some(pgrx::JsonB(json))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get conflict: {}", e);
                None
            }
        }
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
    let status = if strategy == "escalate" { "escalated" } else { "resolved" };
    let resolution_json = serde_json::json!({
        "strategy": strategy,
        "winner": winner,
        "merged_result_id": null,
        "reason": reason,
        "resolved_by": "automatic"
    });

    Spi::connect(|client| {
        match client.update(
            "UPDATE caliber_conflict SET status = $2, resolution = $3, resolved_at = NOW()
             WHERE conflict_id = $1 AND status IN ('detected', 'resolving')",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), conflict_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), status.into_datum()),
                (pgrx::PgBuiltInOids::JSONBOID.oid(), pgrx::JsonB(resolution_json).into_datum()),
            ]),
        ) {
            Ok(count) => count > 0,
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to resolve conflict: {}", e);
                false
            }
        }
    })
}

/// List unresolved conflicts.
#[pg_extern]
fn caliber_conflict_list_unresolved() -> pgrx::JsonB {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT conflict_id, conflict_type, item_a_type, item_a_id, item_b_type, item_b_id, status, detected_at
             FROM caliber_conflict WHERE status IN ('detected', 'resolving')
             ORDER BY detected_at",
            None,
            None,
        );

        match result {
            Ok(table) => {
                let conflicts: Vec<serde_json::Value> = table.into_iter().map(|row| {
                    serde_json::json!({
                        "conflict_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "conflict_type": row.get::<String>(2).ok().flatten(),
                        "item_a_type": row.get::<String>(3).ok().flatten(),
                        "item_a_id": row.get::<pgrx::Uuid>(4).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "item_b_type": row.get::<String>(5).ok().flatten(),
                        "item_b_id": row.get::<pgrx::Uuid>(6).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "status": row.get::<String>(7).ok().flatten(),
                        "detected_at": row.get::<pgrx::TimestampWithTimeZone>(8).ok().flatten().map(|t| t.to_string()),
                    })
                }).collect();
                pgrx::JsonB(serde_json::json!(conflicts))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to list unresolved conflicts: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
}


// ============================================================================
// VECTOR SEARCH (Task 12.3)
// ============================================================================

/// Search for similar vectors using pgvector.
/// Returns entity IDs and similarity scores.
/// Note: This requires pgvector extension and HNSW indexes to be created.
#[pg_extern]
fn caliber_vector_search(
    query_embedding: pgrx::JsonB,
    limit: i32,
) -> pgrx::JsonB {
    // Parse the query embedding
    let query: Vec<f32> = match serde_json::from_value(query_embedding.0) {
        Ok(v) => v,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to parse query embedding: {}", e);
            return pgrx::JsonB(serde_json::json!([]));
        }
    };

    // Convert to pgvector format string: '[1.0, 2.0, 3.0]'
    let vector_str = format!("[{}]", query.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","));

    // Search artifacts and notes using pgvector cosine similarity
    // Using <=> operator for cosine distance (1 - similarity)
    let results = Spi::connect(|client| {
        let result = client.select(
            &format!(
                "SELECT entity_id, entity_type, 1 - (embedding <=> '{}'::vector) as similarity
                 FROM (
                     SELECT artifact_id as entity_id, 'artifact' as entity_type, embedding 
                     FROM caliber_artifact WHERE embedding IS NOT NULL
                     UNION ALL
                     SELECT note_id as entity_id, 'note' as entity_type, embedding 
                     FROM caliber_note WHERE embedding IS NOT NULL
                 ) combined
                 ORDER BY embedding <=> '{}'::vector
                 LIMIT {}",
                vector_str, vector_str, limit
            ),
            None,
            None,
        );

        match result {
            Ok(table) => {
                let mut results = Vec::new();
                for row in table {
                    let entity_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let entity_type: Option<String> = row.get(2).ok().flatten();
                    let similarity: Option<f64> = row.get(3).ok().flatten();

                    if let (Some(eid), Some(etype), Some(sim)) = (entity_id, entity_type, similarity) {
                        results.push(serde_json::json!({
                            "entity_id": Uuid::from_bytes(*eid.as_bytes()).to_string(),
                            "entity_type": etype,
                            "similarity": sim,
                        }));
                    }
                }
                results
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Vector search failed: {}", e);
                Vec::new()
            }
        }
    });

    pgrx::JsonB(serde_json::json!(results))
}


// ============================================================================
// DEBUG SQL VIEWS (Task 12.7)
// Gated behind "debug" or "pg_test" feature flag for safety
// ============================================================================

/// Get storage statistics for debugging.
#[cfg(any(feature = "debug", feature = "pg_test"))]
#[pg_extern]
fn caliber_debug_stats() -> pgrx::JsonB {
    pgrx::warning!("DEBUG: caliber_debug_stats called");

    // Get counts from SQL tables (all entities now in SQL)
    let counts = Spi::connect(|client| {
        let mut counts = serde_json::Map::new();

        let tables = [
            ("trajectories", "caliber_trajectory"),
            ("scopes", "caliber_scope"),
            ("artifacts", "caliber_artifact"),
            ("notes", "caliber_note"),
            ("turns", "caliber_turn"),
            ("locks", "caliber_lock"),
            ("messages", "caliber_message"),
            ("agents", "caliber_agent"),
            ("delegations", "caliber_delegation"),
            ("handoffs", "caliber_handoff"),
            ("conflicts", "caliber_conflict"),
        ];

        for (name, table) in tables {
            let result = client.select(&format!("SELECT COUNT(*) FROM {}", table), None, None);
            let count = match result {
                Ok(table) => {
                    if let Some(row) = table.first() {
                        row.get::<i64>(1).ok().flatten().unwrap_or(0)
                    } else {
                        0
                    }
                }
                Err(_) => 0,
            };
            counts.insert(name.to_string(), serde_json::json!(count));
        }

        counts
    });

    pgrx::JsonB(serde_json::Value::Object(counts))
}

/// Clear all storage (for testing).
#[cfg(any(feature = "debug", feature = "pg_test"))]
#[pg_extern]
fn caliber_debug_clear() -> &'static str {
    pgrx::warning!("DEBUG: caliber_debug_clear called - clearing all storage!");

    // Clear SQL tables in correct order (respecting foreign keys)
    let _ = Spi::run("DELETE FROM caliber_turn");
    let _ = Spi::run("DELETE FROM caliber_artifact");
    let _ = Spi::run("DELETE FROM caliber_scope");
    let _ = Spi::run("DELETE FROM caliber_note");
    let _ = Spi::run("DELETE FROM caliber_message");
    let _ = Spi::run("DELETE FROM caliber_lock");
    let _ = Spi::run("DELETE FROM caliber_conflict");
    let _ = Spi::run("DELETE FROM caliber_handoff");
    let _ = Spi::run("DELETE FROM caliber_delegation");
    let _ = Spi::run("DELETE FROM caliber_region");
    let _ = Spi::run("DELETE FROM caliber_agent");
    let _ = Spi::run("DELETE FROM caliber_trajectory");

    "Storage cleared"
}

/// Dump all trajectories for debugging.
#[cfg(any(feature = "debug", feature = "pg_test"))]
#[pg_extern]
fn caliber_debug_dump_trajectories() -> pgrx::JsonB {
    pgrx::warning!("DEBUG: caliber_debug_dump_trajectories called");
    
    Spi::connect(|client| {
        let result = client.select(
            "SELECT trajectory_id, name, description, status, parent_trajectory_id, 
                    root_trajectory_id, agent_id, created_at, updated_at, completed_at, 
                    outcome, metadata 
             FROM caliber_trajectory ORDER BY created_at DESC",
            None,
            None,
        );

        match result {
            Ok(table) => {
                let mut trajectories = Vec::new();
                for row in table {
                    let trajectory_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let name: Option<String> = row.get(2).ok().flatten();
                    let description: Option<String> = row.get(3).ok().flatten();
                    let status: Option<String> = row.get(4).ok().flatten();
                    let parent_trajectory_id: Option<pgrx::Uuid> = row.get(5).ok().flatten();
                    let root_trajectory_id: Option<pgrx::Uuid> = row.get(6).ok().flatten();
                    let agent_id: Option<pgrx::Uuid> = row.get(7).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(8).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(9).ok().flatten();
                    let completed_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let outcome: Option<pgrx::JsonB> = row.get(11).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                    trajectories.push(serde_json::json!({
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "name": name,
                        "description": description,
                        "status": status,
                        "parent_trajectory_id": parent_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "root_trajectory_id": root_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_id": agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                        "completed_at": completed_at.map(|t| format!("{:?}", t)),
                        "outcome": outcome.map(|j| j.0),
                        "metadata": metadata.map(|j| j.0),
                    }));
                }
                pgrx::JsonB(serde_json::json!(trajectories))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to dump trajectories: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
}

/// Dump all scopes for debugging.
#[cfg(any(feature = "debug", feature = "pg_test"))]
#[pg_extern]
fn caliber_debug_dump_scopes() -> pgrx::JsonB {
    pgrx::warning!("DEBUG: caliber_debug_dump_scopes called");
    
    Spi::connect(|client| {
        let result = client.select(
            "SELECT scope_id, trajectory_id, parent_scope_id, name, purpose, is_active, 
                    created_at, closed_at, checkpoint, token_budget, tokens_used, metadata 
             FROM caliber_scope ORDER BY created_at DESC",
            None,
            None,
        );

        match result {
            Ok(table) => {
                let mut scopes = Vec::new();
                for row in table {
                    let scope_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let trajectory_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let parent_scope_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let name: Option<String> = row.get(4).ok().flatten();
                    let purpose: Option<String> = row.get(5).ok().flatten();
                    let is_active: Option<bool> = row.get(6).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(7).ok().flatten();
                    let closed_at: Option<pgrx::TimestampWithTimeZone> = row.get(8).ok().flatten();
                    let checkpoint: Option<pgrx::JsonB> = row.get(9).ok().flatten();
                    let token_budget: Option<i32> = row.get(10).ok().flatten();
                    let tokens_used: Option<i32> = row.get(11).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                    scopes.push(serde_json::json!({
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "parent_scope_id": parent_scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "name": name,
                        "purpose": purpose,
                        "is_active": is_active,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "closed_at": closed_at.map(|t| format!("{:?}", t)),
                        "checkpoint": checkpoint.map(|j| j.0),
                        "token_budget": token_budget,
                        "tokens_used": tokens_used,
                        "metadata": metadata.map(|j| j.0),
                    }));
                }
                pgrx::JsonB(serde_json::json!(scopes))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to dump scopes: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
}

/// Dump all artifacts for debugging.
#[cfg(any(feature = "debug", feature = "pg_test"))]
#[pg_extern]
fn caliber_debug_dump_artifacts() -> pgrx::JsonB {
    pgrx::warning!("DEBUG: caliber_debug_dump_artifacts called");
    
    Spi::connect(|client| {
        let result = client.select(
            "SELECT artifact_id, trajectory_id, scope_id, artifact_type, name, content, 
                    content_hash, provenance, ttl, created_at, updated_at, superseded_by, metadata 
             FROM caliber_artifact ORDER BY created_at DESC",
            None,
            None,
        );

        match result {
            Ok(table) => {
                let mut artifacts = Vec::new();
                for row in table {
                    let artifact_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let trajectory_id: Option<pgrx::Uuid> = row.get(2).ok().flatten();
                    let scope_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let artifact_type: Option<String> = row.get(4).ok().flatten();
                    let name: Option<String> = row.get(5).ok().flatten();
                    let content: Option<String> = row.get(6).ok().flatten();
                    let content_hash: Option<Vec<u8>> = row.get(7).ok().flatten();
                    let provenance: Option<pgrx::JsonB> = row.get(8).ok().flatten();
                    let ttl: Option<String> = row.get(9).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(11).ok().flatten();
                    let superseded_by: Option<pgrx::Uuid> = row.get(12).ok().flatten();
                    let metadata: Option<pgrx::JsonB> = row.get(13).ok().flatten();

                    artifacts.push(serde_json::json!({
                        "artifact_id": artifact_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "trajectory_id": trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "scope_id": scope_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "artifact_type": artifact_type,
                        "name": name,
                        "content": content,
                        "content_hash": content_hash.map(|h| hex::encode(&h)),
                        "provenance": provenance.map(|j| j.0),
                        "ttl": ttl,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                        "superseded_by": superseded_by.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "metadata": metadata.map(|j| j.0),
                    }));
                }
                pgrx::JsonB(serde_json::json!(artifacts))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to dump artifacts: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
}

/// Dump all agents for debugging.
#[cfg(any(feature = "debug", feature = "pg_test"))]
#[pg_extern]
fn caliber_debug_dump_agents() -> pgrx::JsonB {
    pgrx::warning!("DEBUG: caliber_debug_dump_agents called");
    Spi::connect(|client| {
        let result = client.select(
            "SELECT agent_id, agent_type, capabilities, status, created_at, last_heartbeat
             FROM caliber_agent ORDER BY created_at",
            None,
            None,
        );

        match result {
            Ok(table) => {
                let agents: Vec<serde_json::Value> = table.into_iter().map(|row| {
                    serde_json::json!({
                        "agent_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_type": row.get::<String>(2).ok().flatten(),
                        "capabilities": row.get::<Vec<String>>(3).ok().flatten().unwrap_or_default(),
                        "status": row.get::<String>(4).ok().flatten(),
                        "created_at": row.get::<pgrx::TimestampWithTimeZone>(5).ok().flatten().map(|t| t.to_string()),
                        "last_heartbeat": row.get::<pgrx::TimestampWithTimeZone>(6).ok().flatten().map(|t| t.to_string()),
                    })
                }).collect();
                pgrx::JsonB(serde_json::json!(agents))
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to dump agents: {}", e);
                pgrx::JsonB(serde_json::json!([]))
            }
        }
    })
}


// ============================================================================
// ACCESS CONTROL (Task 12.3)
// ============================================================================

/// Access operation type for permission checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessOperation {
    Read,
    Write,
}

/// Enforce access control for a memory region.
/// Returns Ok(()) if access is allowed, Err with PermissionDenied otherwise.
fn enforce_access(
    agent_id: Uuid,
    region_id: Uuid,
    operation: AccessOperation,
) -> CaliberResult<()> {
    // Get region config from SQL table
    let region_config = Spi::connect(|client| {
        let result = client.select(
            "SELECT region_id, region_type, owner_agent_id, team_id, readers, writers, require_lock
             FROM caliber_region WHERE region_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), region_id.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let region_type: Option<String> = row.get(2).ok().flatten();
                    let owner_agent_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let _team_id: Option<pgrx::Uuid> = row.get(4).ok().flatten();
                    let readers: Option<Vec<pgrx::Uuid>> = row.get(5).ok().flatten();
                    let writers: Option<Vec<pgrx::Uuid>> = row.get(6).ok().flatten();
                    let require_lock: Option<bool> = row.get(7).ok().flatten();

                    Some((
                        region_type.unwrap_or_default(),
                        owner_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes())),
                        readers.unwrap_or_default().iter().map(|u| Uuid::from_bytes(*u.as_bytes())).collect::<Vec<_>>(),
                        writers.unwrap_or_default().iter().map(|u| Uuid::from_bytes(*u.as_bytes())).collect::<Vec<_>>(),
                        require_lock.unwrap_or(false),
                    ))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    });

    // If region not found, deny access
    let (region_type, owner_agent_id, readers, writers, require_lock) = match region_config {
        Some(config) => config,
        None => {
            return Err(CaliberError::Agent(AgentError::PermissionDenied {
                agent_id,
                action: format!("{:?}", operation).to_lowercase(),
                resource: format!("region:{}", region_id),
            }));
        }
    };

    let owner_id = owner_agent_id.unwrap_or(Uuid::nil());

    // Check permission based on region type and operation
    let allowed = match operation {
        AccessOperation::Read => {
            match region_type.as_str() {
                "private" => agent_id == owner_id,
                "team" => agent_id == owner_id || readers.contains(&agent_id),
                "public" | "collaborative" => true,
                _ => false,
            }
        }
        AccessOperation::Write => {
            let base_allowed = match region_type.as_str() {
                "private" => agent_id == owner_id,
                "team" => agent_id == owner_id || writers.contains(&agent_id),
                "public" => agent_id == owner_id,
                "collaborative" => true,
                _ => false,
            };

            // For collaborative regions, also check if lock is held when required
            if base_allowed && require_lock && region_type == "collaborative" {
                // Check if agent holds a lock on this region
                let holds_lock = Spi::connect(|client| {
                    let result = client.select(
                        "SELECT 1 FROM caliber_lock 
                         WHERE resource_type = 'region' AND resource_id = $1 
                         AND holder_agent_id = $2 AND expires_at > NOW()",
                        None,
                        Some(vec![
                            (pgrx::PgBuiltInOids::UUIDOID.oid(), region_id.into_datum()),
                            (pgrx::PgBuiltInOids::UUIDOID.oid(), agent_id.into_datum()),
                        ]),
                    );
                    match result {
                        Ok(table) => table.first().is_some(),
                        Err(_) => false,
                    }
                });

                if !holds_lock {
                    return Err(CaliberError::Agent(AgentError::LockAcquisitionFailed {
                        resource: format!("region:{}", region_id),
                        holder: Uuid::nil(), // Unknown holder
                    }));
                }
                true
            } else {
                base_allowed
            }
        }
    };

    if allowed {
        Ok(())
    } else {
        Err(CaliberError::Agent(AgentError::PermissionDenied {
            agent_id,
            action: format!("{:?}", operation).to_lowercase(),
            resource: format!("region:{}", region_id),
        }))
    }
}

/// Check if an agent has access to a memory region.
/// Returns true if access is allowed, false otherwise.
#[pg_extern]
fn caliber_check_access(
    agent_id: pgrx::Uuid,
    region_id: pgrx::Uuid,
    access_type: &str,
) -> bool {
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let rid = Uuid::from_bytes(*region_id.as_bytes());

    let operation = match access_type {
        "read" => AccessOperation::Read,
        "write" => AccessOperation::Write,
        _ => return false,
    };

    match enforce_access(aid, rid, operation) {
        Ok(()) => true,
        Err(e) => {
            pgrx::warning!("CALIBER: Access denied - {:?}", e);
            false
        }
    }
}

/// Create a new memory region.
#[pg_extern]
fn caliber_region_create(
    owner_agent_id: pgrx::Uuid,
    region_type: &str,
    team_id: Option<pgrx::Uuid>,
    require_lock: bool,
) -> Option<pgrx::Uuid> {
    let owner = Uuid::from_bytes(*owner_agent_id.as_bytes());
    let region_id = new_entity_id();
    let now = Utc::now();

    // Validate region_type
    let valid_region_type = match region_type {
        "private" | "team" | "public" | "collaborative" => region_type,
        _ => {
            pgrx::warning!("CALIBER: Invalid region_type '{}'. Valid values: private, team, public, collaborative", region_type);
            return None;
        }
    };

    // Determine default conflict resolution based on region type
    let conflict_resolution = match valid_region_type {
        "collaborative" => "escalate",
        _ => "last_write_wins",
    };

    // Determine version tracking based on region type
    let version_tracking = matches!(valid_region_type, "team" | "collaborative");

    // For private regions, owner is the only reader/writer
    let (readers, writers) = if valid_region_type == "private" {
        (vec![pgrx::Uuid::from_bytes(*owner.as_bytes())], vec![pgrx::Uuid::from_bytes(*owner.as_bytes())])
    } else if valid_region_type == "public" {
        (Vec::new(), vec![pgrx::Uuid::from_bytes(*owner.as_bytes())])
    } else {
        (Vec::new(), Vec::new())
    };

    let result = Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber_region (region_id, region_type, owner_agent_id, team_id, 
                                         readers, writers, require_lock, conflict_resolution, 
                                         version_tracking, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), region_id.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), valid_region_type.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), owner.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), team_id.into_datum()),
                (pgrx::PgBuiltInOids::UUIDARRAYOID.oid(), readers.into_datum()),
                (pgrx::PgBuiltInOids::UUIDARRAYOID.oid(), writers.into_datum()),
                (pgrx::PgBuiltInOids::BOOLOID.oid(), require_lock.into_datum()),
                (pgrx::PgBuiltInOids::TEXTOID.oid(), conflict_resolution.into_datum()),
                (pgrx::PgBuiltInOids::BOOLOID.oid(), version_tracking.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
            ]),
        )
    });

    match result {
        Ok(_) => Some(pgrx::Uuid::from_bytes(*region_id.as_bytes())),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to create region: {}", e);
            None
        }
    }
}

/// Get a memory region by ID.
#[pg_extern]
fn caliber_region_get(region_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let rid = Uuid::from_bytes(*region_id.as_bytes());

    Spi::connect(|client| {
        let result = client.select(
            "SELECT region_id, region_type, owner_agent_id, team_id, readers, writers, 
                    require_lock, conflict_resolution, version_tracking, created_at, updated_at
             FROM caliber_region WHERE region_id = $1",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), rid.into_datum()),
            ]),
        );

        match result {
            Ok(table) => {
                if let Some(row) = table.first() {
                    let region_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let region_type: Option<String> = row.get(2).ok().flatten();
                    let owner_agent_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let team_id: Option<pgrx::Uuid> = row.get(4).ok().flatten();
                    let readers: Option<Vec<pgrx::Uuid>> = row.get(5).ok().flatten();
                    let writers: Option<Vec<pgrx::Uuid>> = row.get(6).ok().flatten();
                    let require_lock: Option<bool> = row.get(7).ok().flatten();
                    let conflict_resolution: Option<String> = row.get(8).ok().flatten();
                    let version_tracking: Option<bool> = row.get(9).ok().flatten();
                    let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(11).ok().flatten();

                    Some(pgrx::JsonB(serde_json::json!({
                        "region_id": region_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "region_type": region_type,
                        "owner_agent_id": owner_agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "team_id": team_id.map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "readers": readers.map(|ids| ids.iter().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()),
                        "writers": writers.map(|ids| ids.iter().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()),
                        "require_lock": require_lock,
                        "conflict_resolution": conflict_resolution,
                        "version_tracking": version_tracking,
                        "created_at": created_at.map(|t| format!("{:?}", t)),
                        "updated_at": updated_at.map(|t| format!("{:?}", t)),
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to get region: {}", e);
                None
            }
        }
    })
}

/// Add a reader to a memory region.
#[pg_extern]
fn caliber_region_add_reader(region_id: pgrx::Uuid, agent_id: pgrx::Uuid) -> bool {
    let rid = Uuid::from_bytes(*region_id.as_bytes());
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let now = Utc::now();

    let result = Spi::connect(|client| {
        client.update(
            "UPDATE caliber_region 
             SET readers = array_append(readers, $1), updated_at = $2 
             WHERE region_id = $3 AND NOT ($1 = ANY(readers))",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), aid.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), rid.into_datum()),
            ]),
        )
    });

    match result {
        Ok(r) => r.rows_affected() > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to add reader: {}", e);
            false
        }
    }
}

/// Add a writer to a memory region.
#[pg_extern]
fn caliber_region_add_writer(region_id: pgrx::Uuid, agent_id: pgrx::Uuid) -> bool {
    let rid = Uuid::from_bytes(*region_id.as_bytes());
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let now = Utc::now();

    let result = Spi::connect(|client| {
        client.update(
            "UPDATE caliber_region 
             SET writers = array_append(writers, $1), updated_at = $2 
             WHERE region_id = $3 AND NOT ($1 = ANY(writers))",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), aid.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), rid.into_datum()),
            ]),
        )
    });

    match result {
        Ok(r) => r.rows_affected() > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to add writer: {}", e);
            false
        }
    }
}

/// Remove a reader from a memory region.
#[pg_extern]
fn caliber_region_remove_reader(region_id: pgrx::Uuid, agent_id: pgrx::Uuid) -> bool {
    let rid = Uuid::from_bytes(*region_id.as_bytes());
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let now = Utc::now();

    let result = Spi::connect(|client| {
        client.update(
            "UPDATE caliber_region 
             SET readers = array_remove(readers, $1), updated_at = $2 
             WHERE region_id = $3",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), aid.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), rid.into_datum()),
            ]),
        )
    });

    match result {
        Ok(r) => r.rows_affected() > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to remove reader: {}", e);
            false
        }
    }
}

/// Remove a writer from a memory region.
#[pg_extern]
fn caliber_region_remove_writer(region_id: pgrx::Uuid, agent_id: pgrx::Uuid) -> bool {
    let rid = Uuid::from_bytes(*region_id.as_bytes());
    let aid = Uuid::from_bytes(*agent_id.as_bytes());
    let now = Utc::now();

    let result = Spi::connect(|client| {
        client.update(
            "UPDATE caliber_region 
             SET writers = array_remove(writers, $1), updated_at = $2 
             WHERE region_id = $3",
            None,
            Some(vec![
                (pgrx::PgBuiltInOids::UUIDOID.oid(), aid.into_datum()),
                (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                (pgrx::PgBuiltInOids::UUIDOID.oid(), rid.into_datum()),
            ]),
        )
    });

    match result {
        Ok(r) => r.rows_affected() > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to remove writer: {}", e);
            false
        }
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
        // Insert into SQL table via SPI
        Spi::connect(|client| {
            // Check if trajectory already exists
            let exists_result = client.select(
                "SELECT 1 FROM caliber_trajectory WHERE trajectory_id = $1",
                None,
                Some(vec![
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), t.trajectory_id.into_datum()),
                ]),
            );
            
            if let Ok(table) = exists_result {
                if table.first().is_some() {
                    return Err(CaliberError::Storage(StorageError::InsertFailed {
                        entity_type: EntityType::Trajectory,
                        reason: "already exists".to_string(),
                    }));
                }
            }

            let status_str = match t.status {
                TrajectoryStatus::Active => "active",
                TrajectoryStatus::Completed => "completed",
                TrajectoryStatus::Failed => "failed",
                TrajectoryStatus::Suspended => "suspended",
            };

            let outcome_json = t.outcome.as_ref().map(|o| {
                pgrx::JsonB(serde_json::to_value(o).unwrap_or(serde_json::Value::Null))
            });
            let metadata_json = t.metadata.as_ref().map(|m| {
                pgrx::JsonB(serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
            });

            client.update(
                "INSERT INTO caliber_trajectory (trajectory_id, name, description, status, 
                 parent_trajectory_id, root_trajectory_id, agent_id, created_at, updated_at, 
                 completed_at, outcome, metadata) 
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                None,
                Some(vec![
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), t.trajectory_id.into_datum()),
                    (pgrx::PgBuiltInOids::TEXTOID.oid(), t.name.clone().into_datum()),
                    (pgrx::PgBuiltInOids::TEXTOID.oid(), t.description.clone().into_datum()),
                    (pgrx::PgBuiltInOids::TEXTOID.oid(), status_str.into_datum()),
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), t.parent_trajectory_id.into_datum()),
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), t.root_trajectory_id.into_datum()),
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), t.agent_id.into_datum()),
                    (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), t.created_at.into_datum()),
                    (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), t.updated_at.into_datum()),
                    (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), t.completed_at.into_datum()),
                    (pgrx::PgBuiltInOids::JSONBOID.oid(), outcome_json.into_datum()),
                    (pgrx::PgBuiltInOids::JSONBOID.oid(), metadata_json.into_datum()),
                ]),
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;
            
            Ok(())
        })
    }

    fn trajectory_get(&self, id: EntityId) -> CaliberResult<Option<Trajectory>> {
        Spi::connect(|client| {
            let result = client.select(
                "SELECT trajectory_id, name, description, status, parent_trajectory_id, 
                        root_trajectory_id, agent_id, created_at, updated_at, completed_at, 
                        outcome, metadata 
                 FROM caliber_trajectory WHERE trajectory_id = $1",
                None,
                Some(vec![
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), id.into_datum()),
                ]),
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;

            if let Some(row) = result.first() {
                let trajectory_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                let name: Option<String> = row.get(2).ok().flatten();
                let description: Option<String> = row.get(3).ok().flatten();
                let status_str: Option<String> = row.get(4).ok().flatten();
                let parent_trajectory_id: Option<pgrx::Uuid> = row.get(5).ok().flatten();
                let root_trajectory_id: Option<pgrx::Uuid> = row.get(6).ok().flatten();
                let agent_id: Option<pgrx::Uuid> = row.get(7).ok().flatten();
                let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(8).ok().flatten();
                let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(9).ok().flatten();
                let completed_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                let outcome: Option<pgrx::JsonB> = row.get(11).ok().flatten();
                let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                let status = match status_str.as_deref() {
                    Some("active") => TrajectoryStatus::Active,
                    Some("completed") => TrajectoryStatus::Completed,
                    Some("failed") => TrajectoryStatus::Failed,
                    Some("suspended") => TrajectoryStatus::Suspended,
                    _ => TrajectoryStatus::Active,
                };

                Ok(Some(Trajectory {
                    trajectory_id: trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes())).unwrap_or(id),
                    name: name.unwrap_or_default(),
                    description,
                    status,
                    parent_trajectory_id: parent_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes())),
                    root_trajectory_id: root_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes())),
                    agent_id: agent_id.map(|u| Uuid::from_bytes(*u.as_bytes())),
                    created_at: created_at.map(|t| chrono::DateTime::<Utc>::from(t)).unwrap_or_else(Utc::now),
                    updated_at: updated_at.map(|t| chrono::DateTime::<Utc>::from(t)).unwrap_or_else(Utc::now),
                    completed_at: completed_at.map(|t| chrono::DateTime::<Utc>::from(t)),
                    outcome: outcome.and_then(|j| serde_json::from_value(j.0).ok()),
                    metadata: metadata.and_then(|j| serde_json::from_value(j.0).ok()),
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn trajectory_update(&self, id: EntityId, update: TrajectoryUpdate) -> CaliberResult<()> {
        Spi::connect(|client| {
            // First check if trajectory exists
            let exists_result = client.select(
                "SELECT 1 FROM caliber_trajectory WHERE trajectory_id = $1",
                None,
                Some(vec![
                    (pgrx::PgBuiltInOids::UUIDOID.oid(), id.into_datum()),
                ]),
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;
            
            if exists_result.first().is_none() {
                return Err(CaliberError::Storage(StorageError::NotFound {
                    entity_type: EntityType::Trajectory,
                    id,
                }));
            }

            let now = Utc::now();

            // Build dynamic update query based on what fields are provided
            if let Some(status) = update.status {
                let status_str = match status {
                    TrajectoryStatus::Active => "active",
                    TrajectoryStatus::Completed => "completed",
                    TrajectoryStatus::Failed => "failed",
                    TrajectoryStatus::Suspended => "suspended",
                };
                client.update(
                    "UPDATE caliber_trajectory SET status = $1, updated_at = $2 WHERE trajectory_id = $3",
                    None,
                    Some(vec![
                        (pgrx::PgBuiltInOids::TEXTOID.oid(), status_str.into_datum()),
                        (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                        (pgrx::PgBuiltInOids::UUIDOID.oid(), id.into_datum()),
                    ]),
                ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;
            }

            if let Some(metadata) = update.metadata {
                let metadata_json = pgrx::JsonB(serde_json::to_value(&metadata).unwrap_or(serde_json::Value::Null));
                client.update(
                    "UPDATE caliber_trajectory SET metadata = $1, updated_at = $2 WHERE trajectory_id = $3",
                    None,
                    Some(vec![
                        (pgrx::PgBuiltInOids::JSONBOID.oid(), metadata_json.into_datum()),
                        (pgrx::PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
                        (pgrx::PgBuiltInOids::UUIDOID.oid(), id.into_datum()),
                    ]),
                ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;
            }

            Ok(())
        })
    }

    fn trajectory_list_by_status(&self, status: TrajectoryStatus) -> CaliberResult<Vec<Trajectory>> {
        let status_str = match status {
            TrajectoryStatus::Active => "active",
            TrajectoryStatus::Completed => "completed",
            TrajectoryStatus::Failed => "failed",
            TrajectoryStatus::Suspended => "suspended",
        };

        Spi::connect(|client| {
            let result = client.select(
                "SELECT trajectory_id, name, description, status, parent_trajectory_id, 
                        root_trajectory_id, agent_id, created_at, updated_at, completed_at, 
                        outcome, metadata 
                 FROM caliber_trajectory WHERE status = $1 ORDER BY created_at DESC",
                None,
                Some(vec![
                    (pgrx::PgBuiltInOids::TEXTOID.oid(), status_str.into_datum()),
                ]),
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;

            let mut trajectories = Vec::new();
            for row in result {
                let trajectory_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                let name: Option<String> = row.get(2).ok().flatten();
                let description: Option<String> = row.get(3).ok().flatten();
                let status_str: Option<String> = row.get(4).ok().flatten();
                let parent_trajectory_id: Option<pgrx::Uuid> = row.get(5).ok().flatten();
                let root_trajectory_id: Option<pgrx::Uuid> = row.get(6).ok().flatten();
                let agent_id: Option<pgrx::Uuid> = row.get(7).ok().flatten();
                let created_at: Option<pgrx::TimestampWithTimeZone> = row.get(8).ok().flatten();
                let updated_at: Option<pgrx::TimestampWithTimeZone> = row.get(9).ok().flatten();
                let completed_at: Option<pgrx::TimestampWithTimeZone> = row.get(10).ok().flatten();
                let outcome: Option<pgrx::JsonB> = row.get(11).ok().flatten();
                let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                let traj_status = match status_str.as_deref() {
                    Some("active") => TrajectoryStatus::Active,
                    Some("completed") => TrajectoryStatus::Completed,
                    Some("failed") => TrajectoryStatus::Failed,
                    Some("suspended") => TrajectoryStatus::Suspended,
                    _ => TrajectoryStatus::Active,
                };

                if let Some(tid) = trajectory_id {
                    trajectories.push(Trajectory {
                        trajectory_id: Uuid::from_bytes(*tid.as_bytes()),
                        name: name.unwrap_or_default(),
                        description,
                        status: traj_status,
                        parent_trajectory_id: parent_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes())),
                        root_trajectory_id: root_trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes())),
                        agent_id: agent_id.map(|u| Uuid::from_bytes(*u.as_bytes())),
                        created_at: created_at.map(|t| chrono::DateTime::<Utc>::from(t)).unwrap_or_else(Utc::now),
                        updated_at: updated_at.map(|t| chrono::DateTime::<Utc>::from(t)).unwrap_or_else(Utc::now),
                        completed_at: completed_at.map(|t| chrono::DateTime::<Utc>::from(t)),
                        outcome: outcome.and_then(|j| serde_json::from_value(j.0).ok()),
                        metadata: metadata.and_then(|j| serde_json::from_value(j.0).ok()),
                    });
                }
            }
            Ok(trajectories)
        })
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
                entity_type: EntityType::Turn,
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
    fn test_scope_update() {
        crate::caliber_debug_clear();

        // Create trajectory first
        let traj_id = crate::caliber_trajectory_create("Test", None, None);

        // Create scope
        let scope_id = crate::caliber_scope_create(traj_id, "Test Scope", Some("Initial purpose"), 8000);

        // Update scope with various fields
        let updates = pgrx::JsonB(serde_json::json!({
            "name": "Updated Scope",
            "purpose": "Updated purpose",
            "tokens_used": 100,
            "metadata": {"key": "value"}
        }));
        let updated = crate::caliber_scope_update(scope_id, updates);
        assert!(updated);

        // Get scope and verify updates
        let scope = crate::caliber_scope_get(scope_id);
        assert!(scope.is_some());
        let scope_data = scope.unwrap().0;
        assert_eq!(scope_data["name"].as_str(), Some("Updated Scope"));
        assert_eq!(scope_data["purpose"].as_str(), Some("Updated purpose"));
        assert_eq!(scope_data["tokens_used"].as_i64(), Some(100));
        assert!(scope_data["metadata"].is_object());

        // Update with null values
        let null_updates = pgrx::JsonB(serde_json::json!({
            "purpose": null,
            "metadata": null
        }));
        let updated_null = crate::caliber_scope_update(scope_id, null_updates);
        assert!(updated_null);

        // Verify null updates
        let scope_after_null = crate::caliber_scope_get(scope_id);
        assert!(scope_after_null.is_some());
        let scope_null_data = scope_after_null.unwrap().0;
        assert!(scope_null_data["purpose"].is_null());
        assert!(scope_null_data["metadata"].is_null());
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

