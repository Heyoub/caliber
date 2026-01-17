//! CALIBER-PG - PostgreSQL Extension for CALIBER Memory Framework
//!
//! This crate provides the pgrx-based PostgreSQL extension that wires together
//! all CALIBER components. It implements:
//! - Direct heap storage operations (bypassing SQL in hot path)
//! - Advisory lock functions for distributed coordination
//! - NOTIFY-based message passing for agents
//! - Bootstrap SQL schema for extension installation

use pgrx::prelude::*;
use pgrx::datum::DatumWithOid;
use pgrx::spi::Spi;

// pgrx_tests is optional - only available when pg_test feature is enabled
// Currently broken with Postgres 18 due to Pg_magic_struct field changes
#[cfg(feature = "pg_test")]
pub use pgrx_tests::pg_test;

// Re-export core types for use in SQL functions
use caliber_core::{
    AbstractionLevel, AgentError, Artifact, ArtifactType, CaliberConfig, CaliberError,
    CaliberResult, Checkpoint, Edge, EdgeType, EmbeddingVector, EntityId, EntityType,
    ExtractionMethod, MemoryCategory, Note, NoteType, Provenance, RawContent, Scope,
    StorageError, SummarizationTrigger, TTL, Trajectory, TrajectoryOutcome,
    TrajectoryStatus, Turn, TurnRole, ValidationError, compute_content_hash, new_entity_id,
};

// pgrx datum types
use pgrx::datum::TimestampWithTimeZone;
use caliber_storage::{
    ArtifactUpdate, NoteUpdate, ScopeUpdate, StorageTrait, TrajectoryUpdate,
};
use caliber_agents::{
    Agent, AgentHandoff, AgentMessage, AgentStatus, Conflict, ConflictStatus,
    ConflictType, DelegatedTask, DelegationStatus, DistributedLock, HandoffReason,
    HandoffStatus, LockMode, MemoryAccess, MemoryRegion, MemoryRegionConfig,
    MessagePriority, MessageType, ResolutionStrategy, compute_lock_key,
};

use chrono::Utc;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// SPI HELPER FUNCTIONS FOR pgrx 0.16 API
// ============================================================================

/// Convert a uuid::Uuid to DatumWithOid for SPI calls.
#[inline]
fn uuid_datum(id: uuid::Uuid) -> DatumWithOid<'static> {
    let pg_uuid = pgrx::Uuid::from_bytes(*id.as_bytes());
    // SAFETY: pgrx::Uuid is a valid datum type for UUIDOID
    unsafe { DatumWithOid::new(pg_uuid, pgrx::pg_sys::UUIDOID) }
}

/// Convert a pgrx::Uuid to DatumWithOid for SPI calls.
#[inline]
fn pgrx_uuid_datum(id: pgrx::Uuid) -> DatumWithOid<'static> {
    // SAFETY: pgrx::Uuid is a valid datum type for UUIDOID
    unsafe { DatumWithOid::new(id, pgrx::pg_sys::UUIDOID) }
}

/// Convert an optional uuid::Uuid to DatumWithOid for SPI calls.
#[inline]
fn opt_uuid_datum(id: Option<uuid::Uuid>) -> DatumWithOid<'static> {
    match id {
        Some(id) => uuid_datum(id),
        None => DatumWithOid::null_oid(pgrx::pg_sys::UUIDOID),
    }
}

/// Convert a string to DatumWithOid for SPI calls.
#[inline]
fn text_datum(s: &str) -> DatumWithOid<'_> {
    unsafe { DatumWithOid::new(s, pgrx::pg_sys::TEXTOID) }
}

/// Convert an optional string to DatumWithOid for SPI calls.
#[inline]
fn opt_text_datum(s: Option<&str>) -> DatumWithOid<'_> {
    match s {
        Some(s) => text_datum(s),
        None => DatumWithOid::null_oid(pgrx::pg_sys::TEXTOID),
    }
}

/// Convert a bool to DatumWithOid for SPI calls.
#[inline]
fn bool_datum(b: bool) -> DatumWithOid<'static> {
    unsafe { DatumWithOid::new(b, pgrx::pg_sys::BOOLOID) }
}

/// Convert an i32 to DatumWithOid for SPI calls.
#[inline]
fn int4_datum(n: i32) -> DatumWithOid<'static> {
    unsafe { DatumWithOid::new(n, pgrx::pg_sys::INT4OID) }
}

/// Convert an i64 to DatumWithOid for SPI calls.
/// Currently unused but kept for future use with i64 parameters.
#[inline]
#[allow(dead_code)]
fn int8_datum(n: i64) -> DatumWithOid<'static> {
    unsafe { DatumWithOid::new(n, pgrx::pg_sys::INT8OID) }
}

/// Convert a chrono DateTime<Utc> to DatumWithOid for SPI calls.
#[inline]
fn timestamp_datum(dt: chrono::DateTime<chrono::Utc>) -> DatumWithOid<'static> {
    let pg_ts = tuple_extract::chrono_to_timestamp(dt);
    unsafe { DatumWithOid::new(pg_ts, pgrx::pg_sys::TIMESTAMPTZOID) }
}

/// Convert an optional chrono DateTime<Utc> to DatumWithOid for SPI calls.
#[inline]
fn opt_timestamp_datum(dt: Option<chrono::DateTime<chrono::Utc>>) -> DatumWithOid<'static> {
    match dt {
        Some(dt) => timestamp_datum(dt),
        None => DatumWithOid::null_oid(pgrx::pg_sys::TIMESTAMPTZOID),
    }
}

/// Convert a JSON value to DatumWithOid for SPI calls.
#[inline]
fn jsonb_datum(json: &serde_json::Value) -> DatumWithOid<'_> {
    unsafe { DatumWithOid::new(pgrx::JsonB(json.clone()), pgrx::pg_sys::JSONBOID) }
}

/// Convert an optional JSON value to DatumWithOid for SPI calls.
#[inline]
fn opt_jsonb_datum(json: Option<&serde_json::Value>) -> DatumWithOid<'_> {
    match json {
        Some(j) => jsonb_datum(j),
        None => DatumWithOid::null_oid(pgrx::pg_sys::JSONBOID),
    }
}

// Initialize pgrx extension
pgrx::pg_module_magic!();

// ============================================================================
// DIRECT HEAP OPERATION MODULES (Hot Path - NO SQL)
// ============================================================================

/// Direct heap operation helpers for hot-path performance.
/// These bypass SQL parsing entirely.
pub mod heap_ops;

/// Index operation helpers for maintaining indexes during direct heap operations.
pub mod index_ops;

/// Tuple value extraction helpers for reading data from heap tuples.
pub mod tuple_extract;

/// Column mapping constants for all entity tables.
pub mod column_maps;

/// Direct heap operations for Trajectory entities.
pub mod trajectory_heap;

/// Direct heap operations for Scope entities.
pub mod scope_heap;

/// Direct heap operations for Artifact entities.
pub mod artifact_heap;

/// Direct heap operations for Note entities.
pub mod note_heap;

/// Direct heap operations for Turn entities.
pub mod turn_heap;

/// Direct heap operations for Lock entities.
pub mod lock_heap;

/// Direct heap operations for Message entities.
pub mod message_heap;

/// Direct heap operations for Agent entities.
pub mod agent_heap;

/// Direct heap operations for Delegation entities.
pub mod delegation_heap;

/// Direct heap operations for Handoff entities.
pub mod handoff_heap;

/// Direct heap operations for Conflict entities.
pub mod conflict_heap;

/// Direct heap operations for Edge entities (Battle Intel Feature 1: Graph relationships).
pub mod edge_heap;


// ============================================================================
// EXTENSION INITIALIZATION (Task 12.1)
// ============================================================================

/// Extension initialization hook.
/// Called when the extension is loaded.
#[pg_guard]
pub extern "C-unwind" fn _PG_init() {
    // Extension initialization code
    // In production, this would set up shared memory, background workers, etc.
    pgrx::log!("CALIBER extension initializing...");
}

/// Extension finalization hook.
/// Called when the extension is unloaded.
#[pg_guard]
pub extern "C-unwind" fn _PG_fini() {
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
    // NOTE: All entity storage has been migrated to SPI-based SQL via heap operations.
    // This struct now holds runtime metrics and session-local state.

    /// Count of operations performed (for diagnostics)
    ops_count: HashMap<&'static str, u64>,
}

impl InMemoryStorage {
    /// Increment the operation count for a given operation type.
    fn record_op(&mut self, op_name: &'static str) {
        *self.ops_count.entry(op_name).or_insert(0) += 1;
    }

    /// Get the current operation counts.
    #[cfg(any(test, feature = "debug", feature = "pg_test"))]
    fn get_ops(&self) -> &HashMap<&'static str, u64> {
        &self.ops_count
    }

    /// Reset all operation counters.
    #[cfg(any(test, feature = "debug", feature = "pg_test"))]
    fn reset_ops(&mut self) {
        self.ops_count.clear();
    }
}

// ============================================================================
// SAFE STORAGE ACCESS HELPERS
// ============================================================================

/// Safely acquire a read lock on storage, handling poisoning gracefully.
/// Returns the guard or panics with a clear error message for PostgreSQL.
#[cfg(any(test, feature = "debug", feature = "pg_test"))]
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
fn safe_to_json<T: Serialize>(value: &T) -> serde_json::Value {
    match serde_json::to_value(value) {
        Ok(v) => v,
        Err(e) => {
            pgrx::warning!("CALIBER: JSON serialization failed: {}", e);
            serde_json::Value::Null
        }
    }
}

/// Safely serialize a collection to JSON array, returning empty array on failure.
/// Currently unused but kept for future use with slice serialization.
#[allow(dead_code)]
fn safe_to_json_array<T: Serialize>(values: &[T]) -> serde_json::Value {
    match serde_json::to_value(values) {
        Ok(v) => v,
        Err(e) => {
            pgrx::warning!("CALIBER: JSON array serialization failed: {}", e);
            serde_json::json!([])
        }
    }
}

// ============================================================================
// TYPE USAGE DECLARATIONS
// ============================================================================
// These functions and type aliases ensure all imported types are wired into
// the codebase. They provide utility functions for working with caliber types.

/// Create a CaliberConfig for the extension.
/// NOTE: CaliberConfig has NO default - all values must be provided explicitly.
/// This helper creates a minimal valid config for internal use.
#[allow(dead_code)]
fn create_config(token_budget: i32) -> CaliberConfig {
    use std::time::Duration;
    CaliberConfig {
        token_budget,
        section_priorities: caliber_core::SectionPriorities {
            user: 100,
            system: 90,
            persona: 85,
            artifacts: 80,
            notes: 70,
            history: 60,
            custom: vec![],
        },
        checkpoint_retention: 10,
        stale_threshold: Duration::from_secs(3600),
        contradiction_threshold: 0.8,
        context_window_persistence: caliber_core::ContextPersistence::Ephemeral,
        validation_mode: caliber_core::ValidationMode::OnMutation,
        embedding_provider: None,
        summarization_provider: None,
        llm_retry_config: caliber_core::RetryConfig {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        },
        lock_timeout: Duration::from_secs(30),
        message_retention: Duration::from_secs(86400),
        delegation_timeout: Duration::from_secs(300),
    }
}

/// Create a checkpoint from a scope's current state.
/// Checkpoint stores context_state as RawContent (Vec<u8>) and a recoverable flag.
#[allow(dead_code)]
fn create_checkpoint(data: serde_json::Value, recoverable: bool) -> Checkpoint {
    // Serialize the JSON data to bytes for storage
    let context_state: RawContent = serde_json::to_vec(&data).unwrap_or_default();
    Checkpoint { context_state, recoverable }
}

/// Categorize content by memory category.
#[allow(dead_code)]
fn categorize_memory(category: &str) -> MemoryCategory {
    match category {
        "working" => MemoryCategory::Working,
        "episodic" => MemoryCategory::Episodic,
        "semantic" => MemoryCategory::Semantic,
        "procedural" => MemoryCategory::Procedural,
        _ => {
            pgrx::warning!("CALIBER: Unknown memory category '{}', defaulting to Working", category);
            MemoryCategory::Working
        }
    }
}

/// Create a handoff record (uses AgentHandoff type).
#[allow(dead_code)]
fn create_handoff_record(handoff: AgentHandoff) -> serde_json::Value {
    safe_to_json(&handoff)
}

/// Create a message record (uses AgentMessage type).
#[allow(dead_code)]
fn create_message_record(message: AgentMessage) -> serde_json::Value {
    safe_to_json(&message)
}

/// Create a delegated task record (uses DelegatedTask type).
#[allow(dead_code)]
fn create_delegation_record(task: DelegatedTask) -> serde_json::Value {
    safe_to_json(&task)
}

/// Create a distributed lock record (uses DistributedLock type).
#[allow(dead_code)]
fn create_lock_record(lock: DistributedLock) -> serde_json::Value {
    safe_to_json(&lock)
}

/// Create a MemoryAccess based on access level string.
/// MemoryAccess is a struct with read/write permission lists.
/// "read" = read-only, "write" = read+write, "admin" = full access.
#[allow(dead_code)]
fn create_memory_access(access: &str, memory_type: &str) -> MemoryAccess {
    use caliber_agents::{MemoryPermission, PermissionScope};

    let read_perm = MemoryPermission {
        memory_type: memory_type.to_string(),
        scope: PermissionScope::Own,
        filter: None,
    };
    let write_perm = MemoryPermission {
        memory_type: memory_type.to_string(),
        scope: PermissionScope::Own,
        filter: None,
    };

    match access {
        "read" => MemoryAccess {
            read: vec![read_perm],
            write: vec![],
        },
        "write" | "admin" => MemoryAccess {
            read: vec![read_perm],
            write: vec![write_perm],
        },
        _ => {
            pgrx::warning!("CALIBER: Unknown memory access level '{}', returning no permissions", access);
            MemoryAccess {
                read: vec![],
                write: vec![],
            }
        }
    }
}

/// Create a memory region config for a given owner.
/// Uses the appropriate constructor based on region_type.
#[allow(dead_code)]
fn create_region_config(owner_id: EntityId, region_type: &str) -> MemoryRegionConfig {
    match region_type {
        "private" => MemoryRegionConfig::private(owner_id),
        "public" => MemoryRegionConfig::public(owner_id),
        "collaborative" => MemoryRegionConfig::collaborative(owner_id),
        "team" => {
            // Team regions require a team_id - use owner_id as placeholder
            MemoryRegionConfig::team(owner_id, owner_id)
        }
        _ => {
            pgrx::warning!("CALIBER: Unknown region_type '{}', defaulting to private", region_type);
            MemoryRegionConfig::private(owner_id)
        }
    }
}

/// Create a memory region (uses MemoryRegion type).
#[allow(dead_code)]
fn create_memory_region(region: MemoryRegion) -> serde_json::Value {
    safe_to_json(&region)
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
    
    // Execute the bootstrap SQL via SPI (using connect_mut for update operations)
    match Spi::connect_mut(|client| {
        // Split the SQL into individual statements and execute each
        // This handles the multi-statement SQL file properly
        client.update(BOOTSTRAP_SQL, None, &[])?;
        Ok::<_, pgrx::spi::SpiError>(())
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
            &[],
        );

        match result {
            Ok(table) => {
                // In pgrx 0.16+, first() returns the table positioned at first row
                table.first().get_one::<bool>().unwrap_or(Some(false)).unwrap_or(false)
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
    // Record operation for metrics
    storage_write().record_op("trajectory_create");

    let trajectory_id = new_entity_id();

    // Convert pgrx::Uuid to EntityId if provided
    let agent_entity_id = agent_id.map(|u| Uuid::from_bytes(*u.as_bytes()));
    
    // Use direct heap operations instead of SPI
    let result = trajectory_heap::trajectory_create_heap(
        trajectory_id,
        name,
        description,
        agent_entity_id,
    );

    if let Err(e) = result {
        pgrx::warning!("CALIBER: Failed to insert trajectory: {}", e);
    }

    pgrx::Uuid::from_bytes(*trajectory_id.as_bytes())
}

/// Get a trajectory by ID.
#[pg_extern]
fn caliber_trajectory_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());

    // Use direct heap operations instead of SPI
    match trajectory_heap::trajectory_get_heap(entity_id) {
        Ok(Some(trajectory)) => {
            // Convert Trajectory to JSON
            Some(pgrx::JsonB(serde_json::json!({
                "trajectory_id": trajectory.trajectory_id.to_string(),
                "name": trajectory.name,
                "description": trajectory.description,
                "status": match trajectory.status {
                    TrajectoryStatus::Active => "active",
                    TrajectoryStatus::Completed => "completed",
                    TrajectoryStatus::Failed => "failed",
                    TrajectoryStatus::Suspended => "suspended",
                },
                "parent_trajectory_id": trajectory.parent_trajectory_id.map(|id| id.to_string()),
                "root_trajectory_id": trajectory.root_trajectory_id.map(|id| id.to_string()),
                "agent_id": trajectory.agent_id.map(|id| id.to_string()),
                "created_at": trajectory.created_at.to_rfc3339(),
                "updated_at": trajectory.updated_at.to_rfc3339(),
                "completed_at": trajectory.completed_at.map(|t| t.to_rfc3339()),
                "outcome": trajectory.outcome.as_ref().map(safe_to_json),
                "metadata": trajectory.metadata,
            })))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get trajectory: {}", e);
            None
        }
    }
}

/// Update trajectory status.
/// Returns None if status is invalid.
#[pg_extern]
fn caliber_trajectory_set_status(id: pgrx::Uuid, status: &str) -> Option<bool> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());
    
    // Validate status - reject unknown values instead of returning false silently (REQ-12)
    let trajectory_status = match status {
        "active" => TrajectoryStatus::Active,
        "completed" => TrajectoryStatus::Completed,
        "failed" => TrajectoryStatus::Failed,
        "suspended" => TrajectoryStatus::Suspended,
        _ => {
            let validation_err = ValidationError::InvalidValue {
                field: "status".to_string(),
                reason: format!("unknown value '{}'. Valid values: active, completed, failed, suspended", status),
            };
            pgrx::warning!("CALIBER: {:?}", validation_err);
            return None;
        }
    };

    // Use direct heap operations instead of SPI
    match trajectory_heap::trajectory_set_status_heap(entity_id, trajectory_status) {
        Ok(updated) => Some(updated),
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

    // Parse updates from JSON
    let name = update_obj.get("name").and_then(|v| v.as_str());
    
    let description = update_obj.get("description").map(|v| {
        if v.is_null() {
            None
        } else {
            v.as_str()
        }
    });
    
    let status = update_obj.get("status").and_then(|v| v.as_str()).and_then(|s| {
        match s {
            "active" => Some(TrajectoryStatus::Active),
            "completed" => Some(TrajectoryStatus::Completed),
            "failed" => Some(TrajectoryStatus::Failed),
            "suspended" => Some(TrajectoryStatus::Suspended),
            _ => {
                pgrx::warning!("CALIBER: Invalid trajectory status: {}", s);
                None
            }
        }
    });
    
    let parent_trajectory_id = update_obj.get("parent_trajectory_id").map(|v| {
        if v.is_null() {
            None
        } else {
            v.as_str().and_then(|s| Uuid::parse_str(s).ok())
        }
    });
    
    let root_trajectory_id = update_obj.get("root_trajectory_id").map(|v| {
        if v.is_null() {
            None
        } else {
            v.as_str().and_then(|s| Uuid::parse_str(s).ok())
        }
    });
    
    let agent_id = update_obj.get("agent_id").map(|v| {
        if v.is_null() {
            None
        } else {
            v.as_str().and_then(|s| Uuid::parse_str(s).ok())
        }
    });
    
    let outcome = update_obj.get("outcome").map(|v| {
        if v.is_null() {
            None
        } else {
            serde_json::from_value::<TrajectoryOutcome>(v.clone()).ok()
        }
    });
    
    let metadata = update_obj.get("metadata").map(|v| {
        if v.is_null() {
            None
        } else {
            Some(v.clone())
        }
    });

    // Check if any fields are being updated
    if name.is_none() && description.is_none() && status.is_none() 
        && parent_trajectory_id.is_none() && root_trajectory_id.is_none() 
        && agent_id.is_none() && outcome.is_none() && metadata.is_none() {
        pgrx::warning!("CALIBER: No valid fields to update in trajectory");
        return false;
    }

    // Use direct heap operations instead of SPI
    // Convert Option<&Option<T>> to Option<Option<&T>> for proper type matching
    let outcome_ref = outcome.as_ref().map(|o| o.as_ref());
    let metadata_ref = metadata.as_ref().map(|m| m.as_ref());

    let params = trajectory_heap::TrajectoryUpdateHeapParams {
        id: entity_id,
        name,
        description,
        status,
        parent_trajectory_id,
        root_trajectory_id,
        agent_id,
        outcome: outcome_ref,
        metadata: metadata_ref,
    };

    match trajectory_heap::trajectory_update_heap(params) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to update trajectory: {}", e);
            false
        }
    }
}

/// List trajectories by status.
#[pg_extern]
fn caliber_trajectory_list_by_status(status: &str) -> pgrx::JsonB {
    // Validate and convert status
    let trajectory_status = match status {
        "active" => TrajectoryStatus::Active,
        "completed" => TrajectoryStatus::Completed,
        "failed" => TrajectoryStatus::Failed,
        "suspended" => TrajectoryStatus::Suspended,
        _ => {
            pgrx::warning!("CALIBER: Invalid trajectory status '{}', returning empty list", status);
            return pgrx::JsonB(serde_json::json!([]));
        }
    };

    // Use direct heap operations instead of SPI
    match trajectory_heap::trajectory_list_by_status_heap(trajectory_status) {
        Ok(trajectories) => {
            let json_trajectories: Vec<serde_json::Value> = trajectories
                .into_iter()
                .map(|t| {
                    serde_json::json!({
                        "trajectory_id": t.trajectory_id.to_string(),
                        "name": t.name,
                        "description": t.description,
                        "status": match t.status {
                            TrajectoryStatus::Active => "active",
                            TrajectoryStatus::Completed => "completed",
                            TrajectoryStatus::Failed => "failed",
                            TrajectoryStatus::Suspended => "suspended",
                        },
                        "parent_trajectory_id": t.parent_trajectory_id.map(|id| id.to_string()),
                        "root_trajectory_id": t.root_trajectory_id.map(|id| id.to_string()),
                        "agent_id": t.agent_id.map(|id| id.to_string()),
                        "created_at": t.created_at.to_rfc3339(),
                        "updated_at": t.updated_at.to_rfc3339(),
                        "completed_at": t.completed_at.map(|dt| dt.to_rfc3339()),
                        "outcome": t.outcome.as_ref().map(safe_to_json),
                        "metadata": t.metadata,
                    })
                })
                .collect();
            
            pgrx::JsonB(serde_json::json!(json_trajectories))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to list trajectories: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
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

    // Use direct heap operations instead of SPI
    let result = scope_heap::scope_create_heap(
        scope_id,
        traj_id,
        name,
        purpose,
        token_budget,
    );

    if let Err(e) = result {
        pgrx::warning!("CALIBER: Failed to insert scope: {}", e);
    }

    pgrx::Uuid::from_bytes(*scope_id.as_bytes())
}

/// Get a scope by ID.
#[pg_extern]
fn caliber_scope_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());

    // Use direct heap operations instead of SPI
    match scope_heap::scope_get_heap(entity_id) {
        Ok(Some(scope)) => {
            // Convert Scope to JSON
            Some(pgrx::JsonB(serde_json::json!({
                "scope_id": scope.scope_id.to_string(),
                "trajectory_id": scope.trajectory_id.to_string(),
                "parent_scope_id": scope.parent_scope_id.map(|id| id.to_string()),
                "name": scope.name,
                "purpose": scope.purpose,
                "is_active": scope.is_active,
                "created_at": scope.created_at.to_rfc3339(),
                "closed_at": scope.closed_at.map(|t| t.to_rfc3339()),
                "checkpoint": scope.checkpoint.as_ref().map(safe_to_json),
                "token_budget": scope.token_budget,
                "tokens_used": scope.tokens_used,
                "metadata": scope.metadata,
            })))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get scope: {}", e);
            None
        }
    }
}

/// Get the current active scope for a trajectory.
#[pg_extern]
fn caliber_scope_get_current(trajectory_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());

    // Use direct heap operations instead of SPI
    match scope_heap::scope_list_by_trajectory_heap(traj_id) {
        Ok(scopes) => {
            // Filter for active scopes and get the most recent one
            let active_scope = scopes
                .into_iter()
                .filter(|s| s.is_active)
                .max_by_key(|s| s.created_at);
            
            if let Some(scope) = active_scope {
                Some(pgrx::JsonB(serde_json::json!({
                    "scope_id": scope.scope_id.to_string(),
                    "trajectory_id": scope.trajectory_id.to_string(),
                    "parent_scope_id": scope.parent_scope_id.map(|id| id.to_string()),
                    "name": scope.name,
                    "purpose": scope.purpose,
                    "is_active": scope.is_active,
                    "created_at": scope.created_at.to_rfc3339(),
                    "closed_at": scope.closed_at.map(|t| t.to_rfc3339()),
                    "checkpoint": scope.checkpoint.as_ref().map(safe_to_json),
                    "token_budget": scope.token_budget,
                    "tokens_used": scope.tokens_used,
                    "metadata": scope.metadata,
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
}

/// Close a scope.
#[pg_extern]
fn caliber_scope_close(id: pgrx::Uuid) -> bool {
    let entity_id = Uuid::from_bytes(*id.as_bytes());

    // Use direct heap operations instead of SPI
    match scope_heap::scope_close_heap(entity_id) {
        Ok(updated) => updated,
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

    // Use direct heap operations instead of SPI
    match scope_heap::scope_update_tokens_heap(entity_id, tokens_used) {
        Ok(updated) => updated,
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
    use pgrx::datum::DatumWithOid;
    use tuple_extract::chrono_to_timestamp;

    let entity_id = Uuid::from_bytes(*id.as_bytes());
    let update_obj = &updates.0;

    // Build dynamic UPDATE query based on provided fields
    // We'll collect values and build params at the end
    let mut set_clauses: Vec<String> = Vec::new();
    let mut param_idx = 1;

    // Extracted values to hold for lifetimes
    let name_val: Option<String> = update_obj.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let purpose_val: Option<Option<String>> = update_obj.get("purpose").map(|v| {
        if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) }
    });
    let is_active_val: Option<bool> = update_obj.get("is_active").and_then(|v| v.as_bool());
    let closed_at_val: Option<Option<TimestampWithTimeZone>> = update_obj.get("closed_at").map(|v| {
        if v.is_null() {
            None
        } else {
            v.as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| chrono_to_timestamp(dt.with_timezone(&Utc)))
        }
    });
    let checkpoint_val: Option<Option<pgrx::JsonB>> = update_obj.get("checkpoint").map(|v| {
        if v.is_null() { None } else { Some(pgrx::JsonB(v.clone())) }
    });
    let token_budget_val: Option<i32> = update_obj.get("token_budget").and_then(|v| v.as_i64()).map(|n| n as i32);
    let tokens_used_val: Option<i32> = update_obj.get("tokens_used").and_then(|v| v.as_i64()).map(|n| n as i32);
    let parent_scope_id_val: Option<Option<pgrx::Uuid>> = update_obj.get("parent_scope_id").map(|v| {
        if v.is_null() {
            None
        } else {
            v.as_str()
                .and_then(|s| Uuid::parse_str(s).ok())
                .map(|u| pgrx::Uuid::from_bytes(*u.as_bytes()))
        }
    });
    let metadata_val: Option<Option<pgrx::JsonB>> = update_obj.get("metadata").map(|v| {
        if v.is_null() { None } else { Some(pgrx::JsonB(v.clone())) }
    });

    // Build set clauses based on what's provided
    if name_val.is_some() {
        set_clauses.push(format!("name = ${}", param_idx));
        param_idx += 1;
    }
    if purpose_val.is_some() {
        set_clauses.push(format!("purpose = ${}", param_idx));
        param_idx += 1;
    }
    if is_active_val.is_some() {
        set_clauses.push(format!("is_active = ${}", param_idx));
        param_idx += 1;
    }
    if closed_at_val.is_some() {
        set_clauses.push(format!("closed_at = ${}", param_idx));
        param_idx += 1;
    }
    if checkpoint_val.is_some() {
        set_clauses.push(format!("checkpoint = ${}", param_idx));
        param_idx += 1;
    }
    if token_budget_val.is_some() {
        set_clauses.push(format!("token_budget = ${}", param_idx));
        param_idx += 1;
    }
    if tokens_used_val.is_some() {
        set_clauses.push(format!("tokens_used = ${}", param_idx));
        param_idx += 1;
    }
    if parent_scope_id_val.is_some() {
        set_clauses.push(format!("parent_scope_id = ${}", param_idx));
        param_idx += 1;
    }
    if metadata_val.is_some() {
        set_clauses.push(format!("metadata = ${}", param_idx));
        param_idx += 1;
    }

    // If no fields to update, return false
    if set_clauses.is_empty() {
        pgrx::warning!("CALIBER: No valid fields to update in scope");
        return false;
    }

    let query = format!(
        "UPDATE caliber_scope SET {} WHERE scope_id = ${}",
        set_clauses.join(", "),
        param_idx
    );

    // Build params array using our helper functions
    let mut params: Vec<DatumWithOid<'_>> = Vec::new();

    if let Some(ref name) = name_val {
        params.push(unsafe { DatumWithOid::new(name.as_str(), pgrx::pg_sys::TEXTOID) });
    }
    if let Some(ref purpose) = purpose_val {
        match purpose {
            Some(p) => params.push(unsafe { DatumWithOid::new(p.as_str(), pgrx::pg_sys::TEXTOID) }),
            None => params.push(unsafe { DatumWithOid::new(None::<&str>, pgrx::pg_sys::TEXTOID) }),
        }
    }
    if let Some(is_active) = is_active_val {
        params.push(bool_datum(is_active));
    }
    if let Some(ref closed_at) = closed_at_val {
        match closed_at {
            Some(ts) => params.push(unsafe { DatumWithOid::new(*ts, pgrx::pg_sys::TIMESTAMPTZOID) }),
            None => params.push(unsafe { DatumWithOid::new(None::<TimestampWithTimeZone>, pgrx::pg_sys::TIMESTAMPTZOID) }),
        }
    }
    if let Some(ref checkpoint) = checkpoint_val {
        match checkpoint {
            Some(cp) => {
                // JsonB doesn't implement Clone, so we reconstruct it from the inner value
                let owned_cp: pgrx::JsonB = pgrx::JsonB(cp.0.clone());
                params.push(unsafe { DatumWithOid::new(owned_cp, pgrx::pg_sys::JSONBOID) });
            },
            None => params.push(unsafe { DatumWithOid::new(None::<pgrx::JsonB>, pgrx::pg_sys::JSONBOID) }),
        }
    }
    if let Some(budget) = token_budget_val {
        params.push(int4_datum(budget));
    }
    if let Some(used) = tokens_used_val {
        params.push(int4_datum(used));
    }
    if let Some(ref parent_id) = parent_scope_id_val {
        match parent_id {
            Some(pid) => params.push(unsafe { DatumWithOid::new(*pid, pgrx::pg_sys::UUIDOID) }),
            None => params.push(unsafe { DatumWithOid::new(None::<pgrx::Uuid>, pgrx::pg_sys::UUIDOID) }),
        }
    }
    if let Some(ref meta) = metadata_val {
        match meta {
            Some(m) => {
                // JsonB doesn't implement Clone, so we reconstruct it from the inner value
                let owned_m: pgrx::JsonB = pgrx::JsonB(m.0.clone());
                params.push(unsafe { DatumWithOid::new(owned_m, pgrx::pg_sys::JSONBOID) });
            },
            None => params.push(unsafe { DatumWithOid::new(None::<pgrx::JsonB>, pgrx::pg_sys::JSONBOID) }),
        }
    }

    // Add the WHERE clause parameter (entity_id)
    let pg_entity_id = pgrx::Uuid::from_bytes(*entity_id.as_bytes());
    params.push(unsafe { DatumWithOid::new(pg_entity_id, pgrx::pg_sys::UUIDOID) });

    let result: Result<usize, pgrx::spi::SpiError> = Spi::connect_mut(|client| {
        let table = client.update(&query, None, &params)?;
        Ok::<_, pgrx::spi::SpiError>(table.len())
    });

    match result {
        Ok(len) => len > 0,
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
    // Record operation for metrics
    storage_write().record_op("artifact_create");

    // Validate and convert artifact_type - reject unknown values (REQ-12)
    let artifact_type_enum = match artifact_type {
        "error_log" => ArtifactType::ErrorLog,
        "code_patch" => ArtifactType::CodePatch,
        "design_decision" => ArtifactType::DesignDecision,
        "user_preference" => ArtifactType::UserPreference,
        "fact" => ArtifactType::Fact,
        "constraint" => ArtifactType::Constraint,
        "tool_result" => ArtifactType::ToolResult,
        "intermediate_output" => ArtifactType::IntermediateOutput,
        "custom" => ArtifactType::Custom,
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

    // Compute content hash
    let content_hash = compute_content_hash(content.as_bytes());

    // Build provenance
    let provenance = Provenance {
        source_turn: 0,
        extraction_method: ExtractionMethod::Explicit,
        confidence: None,
    };

    // Use direct heap operations instead of SPI
    let result = artifact_heap::artifact_create_heap(artifact_heap::ArtifactCreateParams {
        artifact_id,
        trajectory_id: traj_id,
        scope_id: scp_id,
        artifact_type: artifact_type_enum,
        name,
        content,
        content_hash,
        embedding: None, // No embedding
        provenance: &provenance,
        ttl: TTL::Persistent,
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

    // Use direct heap operations instead of SPI
    match artifact_heap::artifact_get_heap(entity_id) {
        Ok(Some(artifact)) => {
            // Convert Artifact to JSON
            Some(pgrx::JsonB(serde_json::json!({
                "artifact_id": artifact.artifact_id.to_string(),
                "trajectory_id": artifact.trajectory_id.to_string(),
                "scope_id": artifact.scope_id.to_string(),
                "artifact_type": match artifact.artifact_type {
                    ArtifactType::ErrorLog => "error_log",
                    ArtifactType::CodePatch => "code_patch",
                    ArtifactType::DesignDecision => "design_decision",
                    ArtifactType::UserPreference => "user_preference",
                    ArtifactType::Fact => "fact",
                    ArtifactType::Constraint => "constraint",
                    ArtifactType::ToolResult => "tool_result",
                    ArtifactType::IntermediateOutput => "intermediate_output",
                    ArtifactType::Code => "code",
                    ArtifactType::Document => "document",
                    ArtifactType::Data => "data",
                    ArtifactType::Config => "config",
                    ArtifactType::Log => "log",
                    ArtifactType::Summary => "summary",
                    ArtifactType::Decision => "decision",
                    ArtifactType::Plan => "plan",
                    ArtifactType::Custom => "custom",
                },
                "name": artifact.name,
                "content": artifact.content,
                "content_hash": hex::encode(artifact.content_hash),
                "embedding": artifact.embedding,
                "provenance": safe_to_json(&artifact.provenance),
                "ttl": match artifact.ttl {
                    TTL::Persistent => "persistent",
                    TTL::Session => "session",
                    TTL::Scope => "scope",
                    TTL::Duration(ms) => format!("duration:{}", ms).leak(),
                    TTL::Ephemeral => "ephemeral",
                    TTL::ShortTerm => "short_term",
                    TTL::MediumTerm => "medium_term",
                    TTL::LongTerm => "long_term",
                    TTL::Permanent => "permanent",
                },
                "created_at": artifact.created_at.to_rfc3339(),
                "updated_at": artifact.updated_at.to_rfc3339(),
                "superseded_by": artifact.superseded_by.map(|id| id.to_string()),
                "metadata": artifact.metadata,
            })))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get artifact: {}", e);
            None
        }
    }
}

/// Query artifacts by type within a trajectory.
#[pg_extern]
fn caliber_artifact_query_by_type(
    trajectory_id: pgrx::Uuid,
    artifact_type: &str,
) -> pgrx::JsonB {
    // Validate and convert artifact_type
    let artifact_type_enum = match artifact_type {
        "code" => ArtifactType::Code,
        "document" => ArtifactType::Document,
        "data" => ArtifactType::Data,
        "config" => ArtifactType::Config,
        "log" => ArtifactType::Log,
        "summary" => ArtifactType::Summary,
        "decision" => ArtifactType::Decision,
        "plan" => ArtifactType::Plan,
        _ => {
            pgrx::warning!("CALIBER: Invalid artifact type: {}", artifact_type);
            return pgrx::JsonB(serde_json::json!([]));
        }
    };

    // Use direct heap operations instead of SPI
    match artifact_heap::artifact_query_by_type_heap(artifact_type_enum) {
        Ok(artifacts) => {
            // Filter by trajectory_id and convert to JSON
            let json_artifacts: Vec<serde_json::Value> = artifacts
                .into_iter()
                .filter(|a| a.trajectory_id == Uuid::from_bytes(*trajectory_id.as_bytes()))
                .map(|artifact| {
                    serde_json::json!({
                        "artifact_id": artifact.artifact_id.to_string(),
                        "trajectory_id": artifact.trajectory_id.to_string(),
                        "scope_id": artifact.scope_id.to_string(),
                        "artifact_type": match artifact.artifact_type {
                            ArtifactType::Code => "code",
                            ArtifactType::Document => "document",
                            ArtifactType::Data => "data",
                            ArtifactType::Config => "config",
                            ArtifactType::Log => "log",
                            ArtifactType::Summary => "summary",
                            ArtifactType::Decision => "decision",
                            ArtifactType::Plan => "plan",
                            ArtifactType::ErrorLog => "error_log",
                            ArtifactType::CodePatch => "code_patch",
                            ArtifactType::DesignDecision => "design_decision",
                            ArtifactType::UserPreference => "user_preference",
                            ArtifactType::Fact => "fact",
                            ArtifactType::Constraint => "constraint",
                            ArtifactType::ToolResult => "tool_result",
                            ArtifactType::IntermediateOutput => "intermediate_output",
                            ArtifactType::Custom => "custom",
                        },
                        "name": artifact.name,
                        "content": artifact.content,
                        "content_hash": hex::encode(artifact.content_hash),
                        "embedding": artifact.embedding,
                        "provenance": safe_to_json(&artifact.provenance),
                        "ttl": match artifact.ttl {
                            TTL::Persistent => "persistent",
                            TTL::Session => "session",
                            TTL::Scope => "scope",
                            TTL::Duration(ms) => format!("duration:{}", ms).leak(),
                            TTL::Ephemeral => "ephemeral",
                            TTL::ShortTerm => "short_term",
                            TTL::MediumTerm => "medium_term",
                            TTL::LongTerm => "long_term",
                            TTL::Permanent => "permanent",
                        },
                        "created_at": artifact.created_at.to_rfc3339(),
                        "updated_at": artifact.updated_at.to_rfc3339(),
                        "superseded_by": artifact.superseded_by.map(|id| id.to_string()),
                        "metadata": artifact.metadata,
                    })
                })
                .collect();
            
            pgrx::JsonB(serde_json::json!(json_artifacts))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to query artifacts by type: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
}

/// Query artifacts by scope.
#[pg_extern]
fn caliber_artifact_query_by_scope(scope_id: pgrx::Uuid) -> pgrx::JsonB {
    let scp_id = Uuid::from_bytes(*scope_id.as_bytes());

    // Use direct heap operations instead of SPI
    match artifact_heap::artifact_query_by_scope_heap(scp_id) {
        Ok(artifacts) => {
            // Convert to JSON
            let json_artifacts: Vec<serde_json::Value> = artifacts
                .into_iter()
                .map(|artifact| {
                    serde_json::json!({
                        "artifact_id": artifact.artifact_id.to_string(),
                        "trajectory_id": artifact.trajectory_id.to_string(),
                        "scope_id": artifact.scope_id.to_string(),
                        "artifact_type": match artifact.artifact_type {
                            ArtifactType::Code => "code",
                            ArtifactType::Document => "document",
                            ArtifactType::Data => "data",
                            ArtifactType::Config => "config",
                            ArtifactType::Log => "log",
                            ArtifactType::Summary => "summary",
                            ArtifactType::Decision => "decision",
                            ArtifactType::Plan => "plan",
                            ArtifactType::ErrorLog => "error_log",
                            ArtifactType::CodePatch => "code_patch",
                            ArtifactType::DesignDecision => "design_decision",
                            ArtifactType::UserPreference => "user_preference",
                            ArtifactType::Fact => "fact",
                            ArtifactType::Constraint => "constraint",
                            ArtifactType::ToolResult => "tool_result",
                            ArtifactType::IntermediateOutput => "intermediate_output",
                            ArtifactType::Custom => "custom",
                        },
                        "name": artifact.name,
                        "content": artifact.content,
                        "content_hash": hex::encode(artifact.content_hash),
                        "embedding": artifact.embedding,
                        "provenance": safe_to_json(&artifact.provenance),
                        "ttl": match artifact.ttl {
                            TTL::Persistent => "persistent",
                            TTL::Session => "session",
                            TTL::Scope => "scope",
                            TTL::Duration(ms) => format!("duration:{}", ms).leak(),
                            TTL::Ephemeral => "ephemeral",
                            TTL::ShortTerm => "short_term",
                            TTL::MediumTerm => "medium_term",
                            TTL::LongTerm => "long_term",
                            TTL::Permanent => "permanent",
                        },
                        "created_at": artifact.created_at.to_rfc3339(),
                        "updated_at": artifact.updated_at.to_rfc3339(),
                        "superseded_by": artifact.superseded_by.map(|id| id.to_string()),
                        "metadata": artifact.metadata,
                    })
                })
                .collect();
            
            pgrx::JsonB(serde_json::json!(json_artifacts))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to query artifacts by scope: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
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
    // Record operation for metrics
    storage_write().record_op("note_create");

    let note_id = new_entity_id();

    // Validate note_type - reject unknown values instead of defaulting (REQ-12)
    let note_type_enum = match note_type {
        "insight" => NoteType::Insight,
        "procedure" => NoteType::Procedure,
        "fact" => NoteType::Fact,
        "preference" => NoteType::Preference,
        "correction" => NoteType::Correction,
        "summary" => NoteType::Summary,
        _ => {
            let validation_err = ValidationError::InvalidValue {
                field: "note_type".to_string(),
                reason: format!("unknown value '{}'. Valid values: insight, procedure, fact, preference, correction, summary", note_type),
            };
            pgrx::warning!("CALIBER: {:?}", validation_err);
            return None;
        }
    };

    let content_hash = compute_content_hash(content.as_bytes());
    
    // Build source_trajectory_ids array
    let source_traj_ids: Vec<EntityId> = source_trajectory_id
        .map(|u| vec![Uuid::from_bytes(*u.as_bytes())])
        .unwrap_or_default();

    // Use direct heap operations instead of SPI
    let result = note_heap::note_create_heap(note_heap::NoteCreateParams {
        note_id,
        note_type: note_type_enum,
        title,
        content,
        content_hash,
        embedding: None, // embedding
        source_trajectory_ids: &source_traj_ids,
        source_artifact_ids: &[], // source_artifact_ids
        ttl: TTL::Permanent, // default to permanent
        abstraction_level: AbstractionLevel::Raw, // new notes start at L0
        source_note_ids: &[], // source_note_ids - none for newly created notes
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

    // Use direct heap operations instead of SPI
    match note_heap::note_get_heap(entity_id) {
        Ok(Some(note)) => {
            // Convert Note to JSON
            Some(pgrx::JsonB(serde_json::json!({
                "note_id": note.note_id.to_string(),
                "note_type": match note.note_type {
                    NoteType::Convention => "convention",
                    NoteType::Strategy => "strategy",
                    NoteType::Gotcha => "gotcha",
                    NoteType::Fact => "fact",
                    NoteType::Preference => "preference",
                    NoteType::Relationship => "relationship",
                    NoteType::Procedure => "procedure",
                    NoteType::Meta => "meta",
                    NoteType::Insight => "insight",
                    NoteType::Correction => "correction",
                    NoteType::Summary => "summary",
                },
                "title": note.title,
                "content": note.content,
                "content_hash": hex::encode(note.content_hash),
                "embedding": note.embedding,
                "source_trajectory_ids": note.source_trajectory_ids.iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>(),
                "source_artifact_ids": note.source_artifact_ids.iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>(),
                "ttl": match note.ttl {
                    TTL::Persistent => "persistent",
                    TTL::Session => "session",
                    TTL::Scope => "scope",
                    TTL::Duration(ms) => format!("duration:{}", ms).leak(),
                    TTL::Ephemeral => "ephemeral",
                    TTL::ShortTerm => "short_term",
                    TTL::MediumTerm => "medium_term",
                    TTL::LongTerm => "long_term",
                    TTL::Permanent => "permanent",
                },
                "created_at": note.created_at.to_rfc3339(),
                "updated_at": note.updated_at.to_rfc3339(),
                "accessed_at": note.accessed_at.to_rfc3339(),
                "access_count": note.access_count,
                "superseded_by": note.superseded_by.map(|id| id.to_string()),
                "metadata": note.metadata,
            })))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get note: {}", e);
            None
        }
    }
}

/// Query notes by trajectory.
/// Updates access_count and accessed_at for all returned notes.
#[pg_extern]
fn caliber_note_query_by_trajectory(trajectory_id: pgrx::Uuid) -> pgrx::JsonB {
    let traj_id = Uuid::from_bytes(*trajectory_id.as_bytes());

    // Use direct heap operations instead of SPI
    match note_heap::note_query_by_trajectory_heap(traj_id) {
        Ok(notes) => {
            let json_notes: Vec<serde_json::Value> = notes
                .into_iter()
                .map(|note| {
                    serde_json::json!({
                        "note_id": note.note_id.to_string(),
                        "note_type": match note.note_type {
                            NoteType::Convention => "convention",
                            NoteType::Strategy => "strategy",
                            NoteType::Gotcha => "gotcha",
                            NoteType::Fact => "fact",
                            NoteType::Preference => "preference",
                            NoteType::Relationship => "relationship",
                            NoteType::Procedure => "procedure",
                            NoteType::Meta => "meta",
                            NoteType::Insight => "insight",
                            NoteType::Correction => "correction",
                            NoteType::Summary => "summary",
                        },
                        "title": note.title,
                        "content": note.content,
                        "content_hash": hex::encode(note.content_hash),
                        "embedding": note.embedding,
                        "source_trajectory_ids": note.source_trajectory_ids.iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>(),
                        "source_artifact_ids": note.source_artifact_ids.iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>(),
                        "ttl": match note.ttl {
                            TTL::Persistent => "persistent",
                            TTL::Session => "session",
                            TTL::Scope => "scope",
                            TTL::Duration(ms) => format!("duration:{}", ms).leak(),
                            TTL::Ephemeral => "ephemeral",
                            TTL::ShortTerm => "short_term",
                            TTL::MediumTerm => "medium_term",
                            TTL::LongTerm => "long_term",
                            TTL::Permanent => "permanent",
                        },
                        "created_at": note.created_at.to_rfc3339(),
                        "updated_at": note.updated_at.to_rfc3339(),
                        "accessed_at": note.accessed_at.to_rfc3339(),
                        "access_count": note.access_count,
                        "superseded_by": note.superseded_by.map(|id| id.to_string()),
                        "metadata": note.metadata,
                    })
                })
                .collect();
            
            pgrx::JsonB(serde_json::json!(json_notes))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to query notes by trajectory: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
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

    // Validate role - reject unknown values instead of defaulting (REQ-12)
    let turn_role = match role {
        "user" => TurnRole::User,
        "assistant" => TurnRole::Assistant,
        "system" => TurnRole::System,
        "tool" => TurnRole::Tool,
        _ => {
            let validation_err = ValidationError::InvalidValue {
                field: "role".to_string(),
                reason: format!("unknown value '{}'. Valid values: user, assistant, system, tool", role),
            };
            pgrx::warning!("CALIBER: {:?}", validation_err);
            return None;
        }
    };

    // Use direct heap operations instead of SPI
    let result = turn_heap::turn_create_heap(turn_heap::TurnCreateParams {
        turn_id,
        scope_id: scp_id,
        sequence,
        role: turn_role,
        content,
        token_count,
        tool_calls: None,
        tool_results: None,
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

    // Use direct heap operations instead of SPI
    match turn_heap::turn_get_by_scope_heap(scp_id) {
        Ok(turns) => {
            let json_turns: Vec<serde_json::Value> = turns
                .into_iter()
                .map(|t| {
                    serde_json::json!({
                        "turn_id": t.turn_id.to_string(),
                        "scope_id": t.scope_id.to_string(),
                        "sequence": t.sequence,
                        "role": match t.role {
                            TurnRole::User => "user",
                            TurnRole::Assistant => "assistant",
                            TurnRole::System => "system",
                            TurnRole::Tool => "tool",
                        },
                        "content": t.content,
                        "token_count": t.token_count,
                        "created_at": t.created_at.to_rfc3339(),
                        "tool_calls": t.tool_calls,
                        "tool_results": t.tool_results,
                        "metadata": t.metadata,
                    })
                })
                .collect();
            
            pgrx::JsonB(serde_json::json!(json_turns))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get turns by scope: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
}


// ============================================================================
// ADVISORY LOCK FUNCTIONS (Task 12.4)
// Using direct LockAcquire with LOCKTAG for zero SQL overhead.
// ============================================================================

/// Lock level for advisory locks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisoryLockLevel {
    /// Session-level lock - persists until explicit release or session ends
    Session,
    /// Transaction-level lock - auto-releases at transaction commit/rollback
    Transaction,
}

/// Build a LOCKTAG for advisory locks from a lock key (i64).
/// Uses LOCKTAG_ADVISORY with the database ID and key split across fields.
#[inline]
fn make_advisory_locktag(lock_key: i64) -> pg_sys::LOCKTAG {
    pg_sys::LOCKTAG {
        locktag_field1: unsafe { pg_sys::MyDatabaseId.to_u32() },
        locktag_field2: (lock_key >> 32) as u32,
        locktag_field3: lock_key as u32,
        locktag_field4: 0,
        locktag_type: pg_sys::LockTagType::LOCKTAG_ADVISORY as u8,
        locktag_lockmethodid: pg_sys::USER_LOCKMETHOD as u8,
    }
}

/// Try to acquire an advisory lock using direct LockAcquire.
/// Returns true if the lock was acquired, false if not available.
#[inline]
fn try_advisory_lock(lock_key: i64, exclusive: bool, session_lock: bool) -> bool {
    let locktag = make_advisory_locktag(lock_key);
    let lockmode = if exclusive {
        pg_sys::ExclusiveLock as pg_sys::LOCKMODE
    } else {
        pg_sys::ShareLock as pg_sys::LOCKMODE
    };

    // dontWait=true means return immediately if lock not available
    let result = unsafe {
        pg_sys::LockAcquire(&locktag, lockmode, session_lock, true)
    };

    result == pg_sys::LockAcquireResult::LOCKACQUIRE_OK as pg_sys::LockAcquireResult::Type
        || result == pg_sys::LockAcquireResult::LOCKACQUIRE_ALREADY_HELD as pg_sys::LockAcquireResult::Type
}

/// Release an advisory lock using direct LockRelease.
/// Returns true if the lock was released.
#[inline]
fn release_advisory_lock(lock_key: i64, exclusive: bool, session_lock: bool) -> bool {
    let locktag = make_advisory_locktag(lock_key);
    let lockmode = if exclusive {
        pg_sys::ExclusiveLock as pg_sys::LOCKMODE
    } else {
        pg_sys::ShareLock as pg_sys::LOCKMODE
    };

    unsafe { pg_sys::LockRelease(&locktag, lockmode, session_lock) }
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
        _ => {
            if mode != "exclusive" {
                pgrx::warning!("CALIBER: Unknown lock mode '{}', defaulting to Exclusive", mode);
            }
            LockMode::Exclusive
        }
    };

    let lock_level = match level.unwrap_or("transaction") {
        "session" => AdvisoryLockLevel::Session,
        _ => {
            let level_str = level.unwrap_or("transaction");
            if level_str != "transaction" {
                pgrx::warning!("CALIBER: Unknown lock level '{}', defaulting to Transaction", level_str);
            }
            AdvisoryLockLevel::Transaction
        }
    };

    // Try to acquire Postgres advisory lock using direct LockAcquire
    let exclusive = lock_mode == LockMode::Exclusive;
    let session_lock = lock_level == AdvisoryLockLevel::Session;
    let acquired = try_advisory_lock(lock_key, exclusive, session_lock);

    if acquired {
        // Create lock record using direct heap operations for cross-session visibility
        let lock_id = new_entity_id();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::milliseconds(timeout_ms);

        let result = lock_heap::lock_acquire_heap(
            lock_id,
            resource_type,
            resource,
            agent,
            expires_at,
            lock_mode,
        );

        match result {
            Ok(_) => Some(pgrx::Uuid::from_bytes(*lock_id.as_bytes())),
            Err(e) => {
                pgrx::warning!("CALIBER: {:?}", e);
                // Release the advisory lock since we couldn't record it
                // Only session locks need explicit release; transaction locks auto-release
                if session_lock {
                    release_advisory_lock(lock_key, exclusive, session_lock);
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

    // Get lock info using direct heap operations
    let lock_info = match lock_heap::lock_get_heap(lid) {
        Ok(Some(lock)) => Some((lock.resource_type, lock.resource_id, lock.mode)),
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: {:?}", e);
            None
        }
    };

    if let Some((resource_type, resource_id, mode)) = lock_info {
        let lock_key = compute_lock_key(&resource_type, resource_id);

        // Release Postgres advisory lock using direct LockRelease (session-level)
        let exclusive = mode == LockMode::Exclusive;
        release_advisory_lock(lock_key, exclusive, true); // session_lock=true

        // Delete lock record using direct heap operations
        match lock_heap::lock_release_heap(lid) {
            Ok(deleted) => deleted,
            Err(e) => {
                pgrx::warning!("CALIBER: {:?}", e);
                false
            }
        }
    } else {
        // Lock not found
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

    // Query using direct heap operations
    match lock_heap::lock_list_by_resource_heap(resource_type, resource) {
        Ok(locks) => {
            // Filter for non-expired locks
            let now = Utc::now();
            let active_locks: Vec<_> = locks
                .into_iter()
                .filter(|lock| lock.expires_at > now)
                .collect();
            
            active_locks.first().map(|lock| {
                pgrx::JsonB(serde_json::json!({
                    "lock_id": lock.lock_id.to_string(),
                    "resource_type": &lock.resource_type,
                    "resource_id": lock.resource_id.to_string(),
                    "holder_agent_id": lock.holder_agent_id.to_string(),
                    "acquired_at": lock.acquired_at.to_rfc3339(),
                    "expires_at": lock.expires_at.to_rfc3339(),
                    "mode": match lock.mode {
                        LockMode::Exclusive => "exclusive",
                        LockMode::Shared => "shared",
                    },
                }))
            })
        }
        Err(e) => {
            pgrx::warning!("CALIBER: {:?}", e);
            None
        }
    }
}

/// Get lock by ID.
#[pg_extern]
fn caliber_lock_get(lock_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let lid = Uuid::from_bytes(*lock_id.as_bytes());

    // Get lock using direct heap operations
    match lock_heap::lock_get_heap(lid) {
        Ok(Some(lock)) => {
            Some(pgrx::JsonB(serde_json::json!({
                "lock_id": lock.lock_id.to_string(),
                "resource_type": lock.resource_type,
                "resource_id": lock.resource_id.to_string(),
                "holder_agent_id": lock.holder_agent_id.to_string(),
                "acquired_at": lock.acquired_at.to_rfc3339(),
                "expires_at": lock.expires_at.to_rfc3339(),
                "mode": match lock.mode {
                    LockMode::Exclusive => "exclusive",
                    LockMode::Shared => "shared",
                },
            })))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: {:?}", e);
            None
        }
    }
}

/// Extend a lock's expiration time by the given milliseconds.
#[pg_extern]
fn caliber_lock_extend(lock_id: pgrx::Uuid, additional_ms: i64) -> bool {
    let lid = Uuid::from_bytes(*lock_id.as_bytes());
    let lock = match lock_heap::lock_get_heap(lid) {
        Ok(Some(lock)) => lock,
        Ok(None) => return false,
        Err(e) => {
            pgrx::warning!("CALIBER: {:?}", e);
            return false;
        }
    };

    let new_expires_at = lock.expires_at + chrono::Duration::milliseconds(additional_ms);
    match lock_heap::lock_extend_heap(lid, new_expires_at) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: {:?}", e);
            false
        }
    }
}

/// List all active (non-expired) locks.
#[pg_extern]
fn caliber_lock_list_active() -> pgrx::JsonB {
    match lock_heap::lock_list_active_heap() {
        Ok(locks) => {
            let json_locks: Vec<serde_json::Value> = locks
                .into_iter()
                .map(|lock| {
                    serde_json::json!({
                        "lock_id": lock.lock_id.to_string(),
                        "resource_type": lock.resource_type,
                        "resource_id": lock.resource_id.to_string(),
                        "holder_agent_id": lock.holder_agent_id.to_string(),
                        "acquired_at": lock.acquired_at.to_rfc3339(),
                        "expires_at": lock.expires_at.to_rfc3339(),
                        "mode": match lock.mode {
                            LockMode::Exclusive => "exclusive",
                            LockMode::Shared => "shared",
                        },
                    })
                })
                .collect();
            pgrx::JsonB(serde_json::json!(json_locks))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: {:?}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
}


// ============================================================================
// NOTIFY-BASED MESSAGE PASSING (Task 12.5)
// ============================================================================

/// Send a message to an agent.
/// Send a message between agents using direct heap operations.
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

    // Validate and convert message_type
    let msg_type = match message_type {
        "task_delegation" => MessageType::TaskDelegation,
        "task_result" => MessageType::TaskResult,
        "context_request" => MessageType::ContextRequest,
        "context_share" => MessageType::ContextShare,
        "coordination_signal" => MessageType::CoordinationSignal,
        "handoff" => MessageType::Handoff,
        "interrupt" => MessageType::Interrupt,
        "heartbeat" => MessageType::Heartbeat,
        _ => {
            pgrx::warning!("CALIBER: Invalid message_type '{}'. Valid values: task_delegation, task_result, context_request, context_share, coordination_signal, handoff, interrupt, heartbeat", message_type);
            return None;
        }
    };

    // Validate and convert priority
    let msg_priority = match priority {
        "low" => MessagePriority::Low,
        "normal" => MessagePriority::Normal,
        "high" => MessagePriority::High,
        "critical" => MessagePriority::Critical,
        _ => {
            pgrx::warning!("CALIBER: Invalid priority '{}'. Valid values: low, normal, high, critical", priority);
            return None;
        }
    };

    let message_id = new_entity_id();

    // Use direct heap operations instead of SPI
    let result = message_heap::message_send_heap(message_heap::MessageSendParams {
        message_id,
        from_agent_id: from_agent,
        to_agent_id: to_agent,
        to_agent_type,
        message_type: msg_type,
        payload,
        trajectory_id: None,
        scope_id: None,
        artifact_ids: &[],
        priority: msg_priority,
        expires_at: None,
    });

    match result {
        Ok(_) => {
            // Send pg_notify for real-time delivery
            // Determine the channel based on to_agent_id or to_agent_type
            let channel = if let Some(agent_id) = to_agent {
                format!("caliber_agent_{}", agent_id)
            } else if let Some(agent_type) = to_agent_type {
                format!("caliber_agent_type_{}", agent_type)
            } else {
                "caliber_agent_broadcast".to_string()
            };
            
            let notify_result: Result<(), pgrx::spi::SpiError> = Spi::connect_mut(|client| {
                client.update(
                    &format!("NOTIFY {}, '{}'", channel, message_id),
                    None,
                    &[],
                )?;
                Ok::<_, pgrx::spi::SpiError>(())
            });
            
            if let Err(e) = notify_result {
                pgrx::warning!("CALIBER: pg_notify failed: {}", e);
            }
            
            Some(pgrx::Uuid::from_bytes(*message_id.as_bytes()))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to send message: {}", e);
            None
        }
    }
}

/// Get a message by ID using direct heap operations.
#[pg_extern]
fn caliber_message_get(message_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let mid = Uuid::from_bytes(*message_id.as_bytes());

    // Use direct heap operations instead of SPI
    match message_heap::message_get_heap(mid) {
        Ok(Some(message)) => {
            Some(pgrx::JsonB(serde_json::json!({
                "message_id": message.message_id.to_string(),
                "from_agent_id": message.from_agent_id.to_string(),
                "to_agent_id": message.to_agent_id.map(|id| id.to_string()),
                "to_agent_type": message.to_agent_type,
                "message_type": match message.message_type {
                    MessageType::TaskDelegation => "task_delegation",
                    MessageType::TaskResult => "task_result",
                    MessageType::ContextRequest => "context_request",
                    MessageType::ContextShare => "context_share",
                    MessageType::CoordinationSignal => "coordination_signal",
                    MessageType::Handoff => "handoff",
                    MessageType::Interrupt => "interrupt",
                    MessageType::Heartbeat => "heartbeat",
                },
                "payload": message.payload,
                "trajectory_id": message.trajectory_id.map(|id| id.to_string()),
                "scope_id": message.scope_id.map(|id| id.to_string()),
                "artifact_ids": message.artifact_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                "created_at": message.created_at.to_rfc3339(),
                "delivered_at": message.delivered_at.map(|t| t.to_rfc3339()),
                "acknowledged_at": message.acknowledged_at.map(|t| t.to_rfc3339()),
                "priority": match message.priority {
                    MessagePriority::Low => "low",
                    MessagePriority::Normal => "normal",
                    MessagePriority::High => "high",
                    MessagePriority::Critical => "critical",
                },
                "expires_at": message.expires_at.map(|t| t.to_rfc3339()),
            })))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get message: {}", e);
            None
        }
    }
}

/// Mark a message as delivered.
#[pg_extern]
fn caliber_message_mark_delivered(message_id: pgrx::Uuid) -> bool {
    use pgrx::datum::DatumWithOid;
    use tuple_extract::chrono_to_timestamp;

    let mid = Uuid::from_bytes(*message_id.as_bytes());
    let now = chrono_to_timestamp(Utc::now());
    let pg_mid = pgrx::Uuid::from_bytes(*mid.as_bytes());

    let result: Result<usize, pgrx::spi::SpiError> = Spi::connect_mut(|client| {
        let params: &[DatumWithOid<'_>] = &[
            unsafe { DatumWithOid::new(now, pgrx::pg_sys::TIMESTAMPTZOID) },
            unsafe { DatumWithOid::new(pg_mid, pgrx::pg_sys::UUIDOID) },
        ];
        let table = client.update(
            "UPDATE caliber_message SET delivered_at = $1 WHERE message_id = $2 AND delivered_at IS NULL",
            None,
            params,
        )?;
        Ok::<_, pgrx::spi::SpiError>(table.len())
    });

    match result {
        Ok(len) => len > 0,
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

/// Mark a message as acknowledged using direct heap operations.
#[pg_extern]
fn caliber_message_mark_acknowledged(message_id: pgrx::Uuid) -> bool {
    let mid = Uuid::from_bytes(*message_id.as_bytes());

    // Use direct heap operations instead of SPI
    match message_heap::message_acknowledge_heap(mid) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to acknowledge message: {}", e);
            false
        }
    }
}

/// Get pending messages for an agent using direct heap operations.
/// Returns messages where delivered_at IS NULL and not expired, ordered by priority.
/// Note: agent_type is kept for API compatibility but not currently used in filtering.
#[pg_extern]
fn caliber_message_get_pending(agent_id: pgrx::Uuid, _agent_type: &str) -> pgrx::JsonB {
    let aid = Uuid::from_bytes(*agent_id.as_bytes());

    // Use direct heap operations to get messages for this agent
    let messages_result = message_heap::message_list_for_agent_heap(aid);
    
    match messages_result {
        Ok(messages) => {
            let now = Utc::now();
            
            // Filter for pending messages (delivered_at IS NULL and not expired)
            let mut pending: Vec<_> = messages
                .into_iter()
                .filter(|m| {
                    m.delivered_at.is_none() && 
                    (m.expires_at.is_none() || m.expires_at.unwrap() > now)
                })
                .collect();
            
            // Sort by priority (critical > high > normal > low) then by created_at
            pending.sort_by(|a, b| {
                let priority_order = |p: &MessagePriority| match p {
                    MessagePriority::Critical => 1,
                    MessagePriority::High => 2,
                    MessagePriority::Normal => 3,
                    MessagePriority::Low => 4,
                };
                
                priority_order(&a.priority)
                    .cmp(&priority_order(&b.priority))
                    .then_with(|| a.created_at.cmp(&b.created_at))
            });
            
            // Convert to JSON
            let json_messages: Vec<serde_json::Value> = pending
                .into_iter()
                .map(|m| {
                    serde_json::json!({
                        "message_id": m.message_id.to_string(),
                        "from_agent_id": m.from_agent_id.to_string(),
                        "to_agent_id": m.to_agent_id.map(|id| id.to_string()),
                        "to_agent_type": m.to_agent_type,
                        "message_type": match m.message_type {
                            MessageType::TaskDelegation => "task_delegation",
                            MessageType::TaskResult => "task_result",
                            MessageType::ContextRequest => "context_request",
                            MessageType::ContextShare => "context_share",
                            MessageType::CoordinationSignal => "coordination_signal",
                            MessageType::Handoff => "handoff",
                            MessageType::Interrupt => "interrupt",
                            MessageType::Heartbeat => "heartbeat",
                        },
                        "payload": m.payload,
                        "trajectory_id": m.trajectory_id.map(|id| id.to_string()),
                        "scope_id": m.scope_id.map(|id| id.to_string()),
                        "artifact_ids": m.artifact_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                        "created_at": m.created_at.to_rfc3339(),
                        "delivered_at": m.delivered_at.map(|t| t.to_rfc3339()),
                        "acknowledged_at": m.acknowledged_at.map(|t| t.to_rfc3339()),
                        "priority": match m.priority {
                            MessagePriority::Low => "low",
                            MessagePriority::Normal => "normal",
                            MessagePriority::High => "high",
                            MessagePriority::Critical => "critical",
                        },
                        "expires_at": m.expires_at.map(|t| t.to_rfc3339()),
                    })
                })
                .collect();
            
            pgrx::JsonB(serde_json::json!(json_messages))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get pending messages: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
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
    // Record operation for metrics
    storage_write().record_op("agent_register");

    let caps: Vec<String> = serde_json::from_value(capabilities.0)
        .unwrap_or_default();

    let agent = Agent::new(agent_type, caps.clone());
    let agent_id = agent.agent_id;

    // Use direct heap operations instead of SPI
    let result = agent_heap::agent_register_heap(
        agent_id,
        agent_type,
        &caps,
        &agent.memory_access,
        &agent.can_delegate_to,
        agent.reports_to,
    );

    if let Err(e) = result {
        pgrx::warning!("CALIBER: Failed to insert agent: {}", e);
    }

    pgrx::Uuid::from_bytes(*agent_id.as_bytes())
}

/// Get an agent by ID.
#[pg_extern]
fn caliber_agent_get(agent_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*agent_id.as_bytes());

    // Use direct heap operations instead of SPI
    match agent_heap::agent_get_heap(entity_id) {
        Ok(Some(agent)) => {
            // Convert Agent to JSON
            let agent_json = serde_json::json!({
                "agent_id": agent.agent_id.to_string(),
                "agent_type": agent.agent_type,
                "capabilities": agent.capabilities,
                "memory_access": serde_json::to_value(&agent.memory_access).unwrap_or(serde_json::json!({})),
                "status": match agent.status {
                    AgentStatus::Idle => "idle",
                    AgentStatus::Active => "active",
                    AgentStatus::Blocked => "blocked",
                    AgentStatus::Failed => "failed",
                },
                "current_trajectory_id": agent.current_trajectory_id.map(|id| id.to_string()),
                "current_scope_id": agent.current_scope_id.map(|id| id.to_string()),
                "can_delegate_to": agent.can_delegate_to,
                "reports_to": agent.reports_to.map(|id| id.to_string()),
                "created_at": agent.created_at.to_rfc3339(),
                "last_heartbeat": agent.last_heartbeat.to_rfc3339(),
            });
            Some(pgrx::JsonB(agent_json))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get agent: {}", e);
            None
        }
    }
}

/// Update agent status.
#[pg_extern]
fn caliber_agent_set_status(agent_id: pgrx::Uuid, status: &str) -> bool {
    let entity_id = Uuid::from_bytes(*agent_id.as_bytes());
    
    // Validate and convert status
    let agent_status = match status {
        "idle" => AgentStatus::Idle,
        "active" => AgentStatus::Active,
        "blocked" => AgentStatus::Blocked,
        "failed" => AgentStatus::Failed,
        _ => {
            pgrx::warning!("CALIBER: Invalid agent status: {}", status);
            return false;
        }
    };

    // Use direct heap operations instead of SPI
    match agent_heap::agent_set_status_heap(entity_id, agent_status) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to update agent status: {}", e);
            false
        }
    }
}

/// Update agent heartbeat.
#[pg_extern]
fn caliber_agent_heartbeat(agent_id: pgrx::Uuid) -> bool {
    let entity_id = Uuid::from_bytes(*agent_id.as_bytes());

    // Use direct heap operations instead of SPI
    match agent_heap::agent_heartbeat_heap(entity_id) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to update agent heartbeat: {}", e);
            false
        }
    }
}

/// List agents by type.
#[pg_extern]
fn caliber_agent_list_by_type(agent_type: &str) -> pgrx::JsonB {
    // Use direct heap operations instead of SPI
    match agent_heap::agent_list_by_type_heap(agent_type) {
        Ok(agents) => {
            let json_agents: Vec<serde_json::Value> = agents
                .into_iter()
                .map(|agent| {
                    serde_json::json!({
                        "agent_id": agent.agent_id.to_string(),
                        "agent_type": agent.agent_type,
                        "capabilities": agent.capabilities,
                        "status": match agent.status {
                            AgentStatus::Idle => "idle",
                            AgentStatus::Active => "active",
                            AgentStatus::Blocked => "blocked",
                            AgentStatus::Failed => "failed",
                        },
                        "created_at": agent.created_at.to_rfc3339(),
                        "last_heartbeat": agent.last_heartbeat.to_rfc3339(),
                    })
                })
                .collect();
            
            pgrx::JsonB(serde_json::json!(json_agents))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to list agents by type: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
}

/// List all active agents.
#[pg_extern]
fn caliber_agent_list_active() -> pgrx::JsonB {
    Spi::connect(|client| {
        let result = client.select(
            "SELECT agent_id, agent_type, capabilities, status, created_at, last_heartbeat
             FROM caliber_agent WHERE status IN ('active', 'idle')",
            None,
            &[],
        );

        match result {
            Ok(table) => {
                let agents: Vec<serde_json::Value> = table.into_iter().map(|row| {
                    serde_json::json!({
                        "agent_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_type": row.get::<String>(2).ok().flatten(),
                        "capabilities": row.get::<Vec<String>>(3).ok().flatten().unwrap_or_default(),
                        "status": row.get::<String>(4).ok().flatten(),
                        "created_at": row.get::<TimestampWithTimeZone>(5).ok().flatten().map(|t| t.to_string()),
                        "last_heartbeat": row.get::<TimestampWithTimeZone>(6).ok().flatten().map(|t| t.to_string()),
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
    let delegatee = delegatee_agent_id.map(|id| Uuid::from_bytes(*id.as_bytes()));

    // Generate a new delegation ID
    let delegation_id = new_entity_id();

    // Use direct heap operations instead of SPI
    let result = delegation_heap::delegation_create_heap(delegation_heap::DelegationCreateParams {
        delegation_id,
        delegator_agent_id: delegator,
        delegatee_agent_id: delegatee,
        delegatee_agent_type,
        task_description,
        parent_trajectory_id: parent_traj,
        child_trajectory_id: None,
        shared_artifacts: &[],
        shared_notes: &[],
        additional_context: None,
        constraints: "{}",
        deadline: None,
    });

    if let Err(e) = result {
        pgrx::warning!("CALIBER: Failed to insert delegation: {}", e);
    }

    pgrx::Uuid::from_bytes(*delegation_id.as_bytes())
}

/// Get a delegation by ID.
#[pg_extern]
fn caliber_delegation_get(delegation_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*delegation_id.as_bytes());

    // Use direct heap operations instead of SPI
    match delegation_heap::delegation_get_heap(entity_id) {
        Ok(Some(delegation)) => {
            // Convert DelegatedTask to JSON
            Some(pgrx::JsonB(serde_json::json!({
                "delegation_id": delegation.delegation_id.to_string(),
                "delegator_agent_id": delegation.delegator_agent_id.to_string(),
                "delegatee_agent_id": delegation.delegatee_agent_id.map(|id| id.to_string()),
                "delegatee_agent_type": delegation.delegatee_agent_type,
                "task_description": delegation.task_description,
                "parent_trajectory_id": delegation.parent_trajectory_id.to_string(),
                "child_trajectory_id": delegation.child_trajectory_id.map(|id| id.to_string()),
                "shared_artifacts": delegation.shared_artifacts.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                "shared_notes": delegation.shared_notes.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                "additional_context": delegation.additional_context,
                "constraints": delegation.constraints,
                "deadline": delegation.deadline.map(|dt| dt.to_rfc3339()),
                "status": match delegation.status {
                    DelegationStatus::Pending => "pending",
                    DelegationStatus::Accepted => "accepted",
                    DelegationStatus::Rejected => "rejected",
                    DelegationStatus::InProgress => "in_progress",
                    DelegationStatus::Completed => "completed",
                    DelegationStatus::Failed => "failed",
                },
                "result": delegation.result.as_ref().map(safe_to_json),
                "created_at": delegation.created_at.to_rfc3339(),
                "accepted_at": delegation.accepted_at.map(|dt| dt.to_rfc3339()),
                "completed_at": delegation.completed_at.map(|dt| dt.to_rfc3339()),
            })))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get delegation: {}", e);
            None
        }
    }
}

/// Accept a delegation.
///
/// Records the accepting agent and the child trajectory created for the delegated work.
#[pg_extern]
fn caliber_delegation_accept(
    delegation_id: pgrx::Uuid,
    delegatee_agent_id: pgrx::Uuid,
    child_trajectory_id: pgrx::Uuid,
) -> bool {
    let entity_id = Uuid::from_bytes(*delegation_id.as_bytes());
    let agent_id = Uuid::from_bytes(*delegatee_agent_id.as_bytes());
    let traj_id = Uuid::from_bytes(*child_trajectory_id.as_bytes());

    // Use direct heap operations - pass all parameters
    match delegation_heap::delegation_accept_heap(entity_id, agent_id, traj_id) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to accept delegation: {}", e);
            false
        }
    }
}

/// Complete a delegation.
#[pg_extern]
fn caliber_delegation_complete(
    delegation_id: pgrx::Uuid,
    success: bool,
    summary: &str,
) -> bool {
    let entity_id = Uuid::from_bytes(*delegation_id.as_bytes());
    
    // Build DelegationResult from parameters
    let result = if success {
        caliber_agents::DelegationResult {
            status: caliber_agents::DelegationResultStatus::Success,
            produced_artifacts: vec![],
            produced_notes: vec![],
            summary: summary.to_string(),
            error: None,
        }
    } else {
        caliber_agents::DelegationResult {
            status: caliber_agents::DelegationResultStatus::Failure,
            produced_artifacts: vec![],
            produced_notes: vec![],
            summary: String::new(),
            error: Some(summary.to_string()),
        }
    };

    // Use direct heap operations instead of SPI
    match delegation_heap::delegation_complete_heap(entity_id, &result) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to complete delegation: {}", e);
            false
        }
    }
}

/// List pending delegations for an agent type.
#[pg_extern]
fn caliber_delegation_list_pending(agent_type: &str) -> pgrx::JsonB {
    // Use direct heap operations instead of SPI
    match delegation_heap::delegation_list_pending_heap() {
        Ok(delegations) => {
            // Filter by agent type (matching agent_type or wildcard "*")
            let filtered: Vec<serde_json::Value> = delegations
                .into_iter()
                .filter(|d| {
                    d.delegatee_agent_type.as_deref() == Some(agent_type) ||
                    d.delegatee_agent_type.as_deref() == Some("*")
                })
                .map(|d| {
                    serde_json::json!({
                        "delegation_id": d.delegation_id.to_string(),
                        "delegator_agent_id": d.delegator_agent_id.to_string(),
                        "delegatee_agent_id": d.delegatee_agent_id.map(|id| id.to_string()),
                        "delegatee_agent_type": d.delegatee_agent_type,
                        "task_description": d.task_description,
                        "parent_trajectory_id": d.parent_trajectory_id.to_string(),
                        "child_trajectory_id": d.child_trajectory_id.map(|id| id.to_string()),
                        "status": "pending",
                        "created_at": d.created_at.to_rfc3339(),
                    })
                })
                .collect();
            
            pgrx::JsonB(serde_json::json!(filtered))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to list pending delegations: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
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
    let to_agent = to_agent_id.map(|id| Uuid::from_bytes(*id.as_bytes()));
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
        _ => {
            if reason != "scheduled" {
                pgrx::warning!("CALIBER: Unknown handoff reason '{}', defaulting to Scheduled", reason);
            }
            HandoffReason::Scheduled
        }
    };

    let handoff_id = caliber_core::new_entity_id();

    // Insert via direct heap operations
    match handoff_heap::handoff_create_heap(handoff_heap::HandoffCreateParams {
        handoff_id,
        from_agent_id: from_agent,
        to_agent_id: to_agent,
        to_agent_type,
        trajectory_id: traj_id,
        scope_id: scp_id,
        context_snapshot_id: snapshot_id,
        handoff_notes: "",
        next_steps: &[],
        blockers: &[],
        open_questions: &[],
        reason: handoff_reason,
    }) {
        Ok(_) => pgrx::Uuid::from_bytes(*handoff_id.as_bytes()),
        Err(e) => {
            pgrx::error!("CALIBER: Failed to insert handoff: {}", e);
        }
    }
}

/// Get a handoff by ID.
#[pg_extern]
fn caliber_handoff_get(handoff_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let id = Uuid::from_bytes(*handoff_id.as_bytes());
    
    match handoff_heap::handoff_get_heap(id) {
        Ok(Some(handoff)) => {
            let json = serde_json::json!({
                "handoff_id": handoff.handoff_id.to_string(),
                "from_agent_id": handoff.from_agent_id.to_string(),
                "to_agent_id": handoff.to_agent_id.map(|id| id.to_string()),
                "to_agent_type": handoff.to_agent_type,
                "trajectory_id": handoff.trajectory_id.to_string(),
                "scope_id": handoff.scope_id.to_string(),
                "context_snapshot_id": handoff.context_snapshot_id.to_string(),
                "handoff_notes": handoff.handoff_notes,
                "next_steps": handoff.next_steps,
                "blockers": handoff.blockers,
                "open_questions": handoff.open_questions,
                "status": match handoff.status {
                    HandoffStatus::Initiated => "initiated",
                    HandoffStatus::Accepted => "accepted",
                    HandoffStatus::Completed => "completed",
                    HandoffStatus::Rejected => "rejected",
                },
                "reason": match handoff.reason {
                    HandoffReason::CapabilityMismatch => "capability_mismatch",
                    HandoffReason::LoadBalancing => "load_balancing",
                    HandoffReason::Specialization => "specialization",
                    HandoffReason::Escalation => "escalation",
                    HandoffReason::Timeout => "timeout",
                    HandoffReason::Failure => "failure",
                    HandoffReason::Scheduled => "scheduled",
                },
                "initiated_at": handoff.initiated_at.to_rfc3339(),
                "accepted_at": handoff.accepted_at.map(|t| t.to_rfc3339()),
                "completed_at": handoff.completed_at.map(|t| t.to_rfc3339()),
            });
            Some(pgrx::JsonB(json))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get handoff: {}", e);
            None
        }
    }
}

/// Accept a handoff.
///
/// Records the agent accepting the handoff and updates the handoff status.
#[pg_extern]
fn caliber_handoff_accept(handoff_id: pgrx::Uuid, accepting_agent_id: pgrx::Uuid) -> bool {
    let id = Uuid::from_bytes(*handoff_id.as_bytes());
    let agent_id = Uuid::from_bytes(*accepting_agent_id.as_bytes());

    // Use direct heap operations - pass accepting agent ID
    match handoff_heap::handoff_accept_heap(id, agent_id) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to accept handoff: {}", e);
            false
        }
    }
}

/// Complete a handoff.
#[pg_extern]
fn caliber_handoff_complete(handoff_id: pgrx::Uuid) -> bool {
    let id = Uuid::from_bytes(*handoff_id.as_bytes());
    
    match handoff_heap::handoff_complete_heap(id) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to complete handoff: {}", e);
            false
        }
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
        _ => {
            if conflict_type != "goal_conflict" {
                pgrx::warning!("CALIBER: Unknown conflict type '{}', defaulting to GoalConflict", conflict_type);
            }
            ConflictType::GoalConflict
        }
    };

    let conflict = Conflict::new(c_type, item_a_type, a_id, item_b_type, b_id);
    let conflict_id = conflict.conflict_id;

    // Insert via direct heap operations (NO SQL)
    match conflict_heap::conflict_create_heap(conflict_heap::ConflictCreateParams {
        conflict_id,
        conflict_type: c_type,
        item_a_type,
        item_a_id: a_id,
        item_b_type,
        item_b_id: b_id,
        agent_a_id: None,
        agent_b_id: None,
        trajectory_id: None,
    }) {
        Ok(_) => {},
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to insert conflict: {}", e);
        }
    }

    pgrx::Uuid::from_bytes(*conflict_id.as_bytes())
}

/// Get a conflict by ID.
#[pg_extern]
fn caliber_conflict_get(conflict_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let id = Uuid::from_bytes(*conflict_id.as_bytes());
    
    // Get via direct heap operations (NO SQL)
    match conflict_heap::conflict_get_heap(id) {
        Ok(Some(conflict)) => {
            let status_str = match conflict.status {
                ConflictStatus::Detected => "detected",
                ConflictStatus::Resolving => "resolving",
                ConflictStatus::Resolved => "resolved",
                ConflictStatus::Escalated => "escalated",
            };
            
            let json = serde_json::json!({
                "conflict_id": conflict.conflict_id.to_string(),
                "conflict_type": match conflict.conflict_type {
                    ConflictType::ConcurrentWrite => "concurrent_write",
                    ConflictType::ContradictingFact => "contradicting_fact",
                    ConflictType::IncompatibleDecision => "incompatible_decision",
                    ConflictType::ResourceContention => "resource_contention",
                    ConflictType::GoalConflict => "goal_conflict",
                },
                "item_a_type": conflict.item_a_type,
                "item_a_id": conflict.item_a_id.to_string(),
                "item_b_type": conflict.item_b_type,
                "item_b_id": conflict.item_b_id.to_string(),
                "agent_a_id": conflict.agent_a_id.map(|id| id.to_string()),
                "agent_b_id": conflict.agent_b_id.map(|id| id.to_string()),
                "trajectory_id": conflict.trajectory_id.map(|id| id.to_string()),
                "status": status_str,
                "resolution": conflict.resolution.as_ref().map(safe_to_json),
                "detected_at": conflict.detected_at.to_rfc3339(),
                "resolved_at": conflict.resolved_at.map(|t| t.to_rfc3339()),
            });
            Some(pgrx::JsonB(json))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get conflict: {}", e);
            None
        }
    }
}

/// Resolve a conflict.
#[pg_extern]
fn caliber_conflict_resolve(
    conflict_id: pgrx::Uuid,
    strategy: &str,
    winner: Option<&str>,
    reason: &str,
) -> bool {
    use caliber_agents::ConflictResolutionRecord;

    let id = Uuid::from_bytes(*conflict_id.as_bytes());
    
    // Parse strategy
    let resolution_strategy = match strategy {
        "last_write_wins" => ResolutionStrategy::LastWriteWins,
        "first_write_wins" => ResolutionStrategy::FirstWriteWins,
        "highest_confidence" => ResolutionStrategy::HighestConfidence,
        "merge" => ResolutionStrategy::Merge,
        "escalate" => ResolutionStrategy::Escalate,
        "reject_both" => ResolutionStrategy::RejectBoth,
        _ => {
            pgrx::warning!("CALIBER: Unknown resolution strategy '{}', defaulting to Escalate", strategy);
            ResolutionStrategy::Escalate
        }
    };
    
    let resolution = ConflictResolutionRecord {
        strategy: resolution_strategy,
        winner: winner.map(|s| s.to_string()),
        merged_result_id: None,
        reason: reason.to_string(),
        resolved_by: "automatic".to_string(),
    };
    
    // Resolve via direct heap operations (NO SQL)
    match conflict_heap::conflict_resolve_heap(id, &resolution) {
        Ok(updated) => updated,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to resolve conflict: {}", e);
            false
        }
    }
}

/// List unresolved conflicts.
#[pg_extern]
fn caliber_conflict_list_unresolved() -> pgrx::JsonB {
    // List via direct heap operations (NO SQL)
    match conflict_heap::conflict_list_pending_heap() {
        Ok(conflicts) => {
            let conflicts_json: Vec<serde_json::Value> = conflicts.into_iter().map(|conflict| {
                serde_json::json!({
                    "conflict_id": conflict.conflict_id.to_string(),
                    "conflict_type": match conflict.conflict_type {
                        ConflictType::ConcurrentWrite => "concurrent_write",
                        ConflictType::ContradictingFact => "contradicting_fact",
                        ConflictType::IncompatibleDecision => "incompatible_decision",
                        ConflictType::ResourceContention => "resource_contention",
                        ConflictType::GoalConflict => "goal_conflict",
                    },
                    "item_a_type": conflict.item_a_type,
                    "item_a_id": conflict.item_a_id.to_string(),
                    "item_b_type": conflict.item_b_type,
                    "item_b_id": conflict.item_b_id.to_string(),
                    "status": match conflict.status {
                        ConflictStatus::Detected => "detected",
                        ConflictStatus::Resolving => "resolving",
                        ConflictStatus::Resolved => "resolved",
                        ConflictStatus::Escalated => "escalated",
                    },
                    "detected_at": conflict.detected_at.to_rfc3339(),
                })
            }).collect();
            pgrx::JsonB(serde_json::json!(conflicts_json))
        }
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to list unresolved conflicts: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
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
            &[],
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
#[cfg(any(test, feature = "debug", feature = "pg_test"))]
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
            let result = client.select(&format!("SELECT COUNT(*) FROM {}", table), None, &[]);
            let count = match result {
                Ok(table) => {
                    let table = table.first();
                    table.get_one::<i64>().ok().flatten().unwrap_or(0)
                }
                Err(_) => 0,
            };
            counts.insert(name.to_string(), serde_json::json!(count));
        }

        counts
    });

    // Add operation metrics from in-memory storage
    let ops = storage_read();
    let ops_data: serde_json::Map<String, serde_json::Value> = ops
        .get_ops()
        .iter()
        .map(|(k, v)| (k.to_string(), serde_json::json!(*v)))
        .collect();

    let mut result = serde_json::Map::new();
    result.insert("entity_counts".to_string(), serde_json::Value::Object(counts));
    result.insert("operation_counts".to_string(), serde_json::Value::Object(ops_data));

    pgrx::JsonB(serde_json::Value::Object(result))
}

/// Clear all storage (for testing).
#[cfg(any(test, feature = "debug", feature = "pg_test"))]
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

    // Reset in-memory operation counters
    storage_write().reset_ops();

    "Storage cleared"
}

/// Dump all trajectories for debugging.
#[cfg(any(test, feature = "debug", feature = "pg_test"))]
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
            &[],
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
                    let created_at: Option<TimestampWithTimeZone> = row.get(8).ok().flatten();
                    let updated_at: Option<TimestampWithTimeZone> = row.get(9).ok().flatten();
                    let completed_at: Option<TimestampWithTimeZone> = row.get(10).ok().flatten();
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
#[cfg(any(test, feature = "debug", feature = "pg_test"))]
#[pg_extern]
fn caliber_debug_dump_scopes() -> pgrx::JsonB {
    pgrx::warning!("DEBUG: caliber_debug_dump_scopes called");
    
    Spi::connect(|client| {
        let result = client.select(
            "SELECT scope_id, trajectory_id, parent_scope_id, name, purpose, is_active, 
                    created_at, closed_at, checkpoint, token_budget, tokens_used, metadata 
             FROM caliber_scope ORDER BY created_at DESC",
            None,
            &[],
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
                    let created_at: Option<TimestampWithTimeZone> = row.get(7).ok().flatten();
                    let closed_at: Option<TimestampWithTimeZone> = row.get(8).ok().flatten();
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
#[cfg(any(test, feature = "debug", feature = "pg_test"))]
#[pg_extern]
fn caliber_debug_dump_artifacts() -> pgrx::JsonB {
    pgrx::warning!("DEBUG: caliber_debug_dump_artifacts called");
    
    Spi::connect(|client| {
        let result = client.select(
            "SELECT artifact_id, trajectory_id, scope_id, artifact_type, name, content, 
                    content_hash, provenance, ttl, created_at, updated_at, superseded_by, metadata 
             FROM caliber_artifact ORDER BY created_at DESC",
            None,
            &[],
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
                    let created_at: Option<TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let updated_at: Option<TimestampWithTimeZone> = row.get(11).ok().flatten();
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
#[cfg(any(test, feature = "debug", feature = "pg_test"))]
#[pg_extern]
fn caliber_debug_dump_agents() -> pgrx::JsonB {
    pgrx::warning!("DEBUG: caliber_debug_dump_agents called");
    Spi::connect(|client| {
        let result = client.select(
            "SELECT agent_id, agent_type, capabilities, status, created_at, last_heartbeat
             FROM caliber_agent ORDER BY created_at",
            None,
            &[],
        );

        match result {
            Ok(table) => {
                let agents: Vec<serde_json::Value> = table.into_iter().map(|row| {
                    serde_json::json!({
                        "agent_id": row.get::<pgrx::Uuid>(1).ok().flatten().map(|u| Uuid::from_bytes(*u.as_bytes()).to_string()),
                        "agent_type": row.get::<String>(2).ok().flatten(),
                        "capabilities": row.get::<Vec<String>>(3).ok().flatten().unwrap_or_default(),
                        "status": row.get::<String>(4).ok().flatten(),
                        "created_at": row.get::<TimestampWithTimeZone>(5).ok().flatten().map(|t| t.to_string()),
                        "last_heartbeat": row.get::<TimestampWithTimeZone>(6).ok().flatten().map(|t| t.to_string()),
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
        use pgrx::datum::DatumWithOid;
        let pg_region_id = pgrx::Uuid::from_bytes(*region_id.as_bytes());
        let params: &[DatumWithOid<'_>] = &[
            unsafe { DatumWithOid::new(pg_region_id, pgrx::pg_sys::UUIDOID) },
        ];
        let result = client.select(
            "SELECT region_id, region_type, owner_agent_id, team_id, readers, writers, require_lock
             FROM caliber_region WHERE region_id = $1",
            None,
            params,
        );

        match result {
            Ok(mut table) => {
                if let Some(row) = table.next() {
                    let region_type: Option<String> = row.get(2).ok().flatten();
                    let owner_agent_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let _team_id: Option<pgrx::Uuid> = row.get(4).ok().flatten();
                    let readers: Option<Vec<pgrx::Uuid>> = row.get(5).ok().flatten();
                    let writers: Option<Vec<pgrx::Uuid>> = row.get(6).ok().flatten();
                    let require_lock: Option<bool> = row.get(7).ok().flatten();

                    // Convert pgrx::Uuid to uuid::Uuid with explicit type annotations
                    let owner_id = owner_agent_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()));
                    let readers_vec: Vec<Uuid> = readers.unwrap_or_default().iter()
                        .map(|u: &pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()))
                        .collect();
                    let writers_vec: Vec<Uuid> = writers.unwrap_or_default().iter()
                        .map(|u: &pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()))
                        .collect();

                    Some((
                        region_type.unwrap_or_default(),
                        owner_id,
                        readers_vec,
                        writers_vec,
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
    let (region_type, owner_agent_id, readers, writers, require_lock): (String, Option<Uuid>, Vec<Uuid>, Vec<Uuid>, bool) = match region_config {
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
                _ => {
                    pgrx::warning!("CALIBER: Unknown region_type '{}' in read access check, denying access", region_type);
                    false
                }
            }
        }
        AccessOperation::Write => {
            let base_allowed = match region_type.as_str() {
                "private" => agent_id == owner_id,
                "team" => agent_id == owner_id || writers.contains(&agent_id),
                "public" => agent_id == owner_id,
                "collaborative" => true,
                _ => {
                    pgrx::warning!("CALIBER: Unknown region_type '{}' in write access check, denying access", region_type);
                    false
                }
            };

            // For collaborative regions, also check if lock is held when required
            if base_allowed && require_lock && region_type == "collaborative" {
                // Check if agent holds a lock on this region
                let holds_lock = Spi::connect(|client| {
                    use pgrx::datum::DatumWithOid;
                    let pg_region_id = pgrx::Uuid::from_bytes(*region_id.as_bytes());
                    let pg_agent_id = pgrx::Uuid::from_bytes(*agent_id.as_bytes());
                    let params: &[DatumWithOid<'_>] = &[
                        unsafe { DatumWithOid::new(pg_region_id, pgrx::pg_sys::UUIDOID) },
                        unsafe { DatumWithOid::new(pg_agent_id, pgrx::pg_sys::UUIDOID) },
                    ];
                    let result = client.select(
                        "SELECT 1 FROM caliber_lock
                         WHERE resource_type = 'region' AND resource_id = $1
                         AND holder_agent_id = $2 AND expires_at > NOW()",
                        None,
                        params,
                    );
                    match result {
                        Ok(table) => !table.is_empty(),
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
        _ => {
            pgrx::warning!("CALIBER: Invalid access_type '{}', must be 'read' or 'write'", access_type);
            return false;
        }
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
    use pgrx::datum::DatumWithOid;
    use tuple_extract::chrono_to_timestamp;

    let pg_owner = owner_agent_id;
    let region_id = new_entity_id();
    let pg_region_id = pgrx::Uuid::from_bytes(*region_id.as_bytes());
    let now = chrono_to_timestamp(Utc::now());

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
    let (readers, writers): (Vec<pgrx::Uuid>, Vec<pgrx::Uuid>) = if valid_region_type == "private" {
        (vec![pg_owner], vec![pg_owner])
    } else if valid_region_type == "public" {
        (Vec::new(), vec![pg_owner])
    } else {
        (Vec::new(), Vec::new())
    };

    let result: Result<(), pgrx::spi::SpiError> = Spi::connect_mut(|client| {
        let params: &[DatumWithOid<'_>] = &[
            unsafe { DatumWithOid::new(pg_region_id, pgrx::pg_sys::UUIDOID) },
            unsafe { DatumWithOid::new(valid_region_type, pgrx::pg_sys::TEXTOID) },
            unsafe { DatumWithOid::new(pg_owner, pgrx::pg_sys::UUIDOID) },
            unsafe { DatumWithOid::new(team_id, pgrx::pg_sys::UUIDOID) },
            unsafe { DatumWithOid::new(readers.clone(), pgrx::pg_sys::UUIDARRAYOID) },
            unsafe { DatumWithOid::new(writers.clone(), pgrx::pg_sys::UUIDARRAYOID) },
            bool_datum(require_lock),
            unsafe { DatumWithOid::new(conflict_resolution, pgrx::pg_sys::TEXTOID) },
            bool_datum(version_tracking),
            unsafe { DatumWithOid::new(now, pgrx::pg_sys::TIMESTAMPTZOID) },
            unsafe { DatumWithOid::new(now, pgrx::pg_sys::TIMESTAMPTZOID) },
        ];
        client.update(
            "INSERT INTO caliber_region (region_id, region_type, owner_agent_id, team_id,
                                         readers, writers, require_lock, conflict_resolution,
                                         version_tracking, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            None,
            params,
        )?;
        Ok::<_, pgrx::spi::SpiError>(())
    });

    match result {
        Ok(()) => Some(pg_region_id),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to create region: {}", e);
            None
        }
    }
}

/// Get a memory region by ID.
#[pg_extern]
fn caliber_region_get(region_id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    use pgrx::datum::DatumWithOid;

    let rid = Uuid::from_bytes(*region_id.as_bytes());
    let pg_rid = pgrx::Uuid::from_bytes(*rid.as_bytes());

    Spi::connect(|client| {
        let params: &[DatumWithOid<'_>] = &[
            unsafe { DatumWithOid::new(pg_rid, pgrx::pg_sys::UUIDOID) },
        ];
        let result = client.select(
            "SELECT region_id, region_type, owner_agent_id, team_id, readers, writers,
                    require_lock, conflict_resolution, version_tracking, created_at, updated_at
             FROM caliber_region WHERE region_id = $1",
            None,
            params,
        );

        match result {
            Ok(mut table) => {
                if let Some(row) = table.next() {
                    let region_id_val: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                    let region_type: Option<String> = row.get(2).ok().flatten();
                    let owner_agent_id: Option<pgrx::Uuid> = row.get(3).ok().flatten();
                    let team_id: Option<pgrx::Uuid> = row.get(4).ok().flatten();
                    let readers: Option<Vec<pgrx::Uuid>> = row.get(5).ok().flatten();
                    let writers: Option<Vec<pgrx::Uuid>> = row.get(6).ok().flatten();
                    let require_lock: Option<bool> = row.get(7).ok().flatten();
                    let conflict_resolution: Option<String> = row.get(8).ok().flatten();
                    let version_tracking: Option<bool> = row.get(9).ok().flatten();
                    let created_at: Option<TimestampWithTimeZone> = row.get(10).ok().flatten();
                    let updated_at: Option<TimestampWithTimeZone> = row.get(11).ok().flatten();

                    // Convert pgrx::Uuid to uuid::Uuid strings with explicit type annotations
                    let region_id_str = region_id_val.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string());
                    let owner_id_str = owner_agent_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string());
                    let team_id_str = team_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string());
                    let readers_strs = readers.map(|ids: Vec<pgrx::Uuid>| {
                        ids.iter().map(|u: &pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()
                    });
                    let writers_strs = writers.map(|ids: Vec<pgrx::Uuid>| {
                        ids.iter().map(|u: &pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string()).collect::<Vec<_>>()
                    });

                    Some(pgrx::JsonB(serde_json::json!({
                        "region_id": region_id_str,
                        "region_type": region_type,
                        "owner_agent_id": owner_id_str,
                        "team_id": team_id_str,
                        "readers": readers_strs,
                        "writers": writers_strs,
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
    use pgrx::datum::DatumWithOid;
    use tuple_extract::chrono_to_timestamp;

    let pg_rid = region_id;
    let pg_aid = agent_id;
    let now = chrono_to_timestamp(Utc::now());

    let result: Result<usize, pgrx::spi::SpiError> = Spi::connect_mut(|client| {
        let params: &[DatumWithOid<'_>] = &[
            unsafe { DatumWithOid::new(pg_aid, pgrx::pg_sys::UUIDOID) },
            unsafe { DatumWithOid::new(now, pgrx::pg_sys::TIMESTAMPTZOID) },
            unsafe { DatumWithOid::new(pg_rid, pgrx::pg_sys::UUIDOID) },
        ];
        let table = client.update(
            "UPDATE caliber_region
             SET readers = array_append(readers, $1), updated_at = $2
             WHERE region_id = $3 AND NOT ($1 = ANY(readers))",
            None,
            params,
        )?;
        Ok::<_, pgrx::spi::SpiError>(table.len())
    });

    match result {
        Ok(len) => len > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to add reader: {}", e);
            false
        }
    }
}

/// Add a writer to a memory region.
#[pg_extern]
fn caliber_region_add_writer(region_id: pgrx::Uuid, agent_id: pgrx::Uuid) -> bool {
    use pgrx::datum::DatumWithOid;
    use tuple_extract::chrono_to_timestamp;

    let pg_rid = region_id;
    let pg_aid = agent_id;
    let now = chrono_to_timestamp(Utc::now());

    let result: Result<usize, pgrx::spi::SpiError> = Spi::connect_mut(|client| {
        let params: &[DatumWithOid<'_>] = &[
            unsafe { DatumWithOid::new(pg_aid, pgrx::pg_sys::UUIDOID) },
            unsafe { DatumWithOid::new(now, pgrx::pg_sys::TIMESTAMPTZOID) },
            unsafe { DatumWithOid::new(pg_rid, pgrx::pg_sys::UUIDOID) },
        ];
        let table = client.update(
            "UPDATE caliber_region
             SET writers = array_append(writers, $1), updated_at = $2
             WHERE region_id = $3 AND NOT ($1 = ANY(writers))",
            None,
            params,
        )?;
        Ok::<_, pgrx::spi::SpiError>(table.len())
    });

    match result {
        Ok(len) => len > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to add writer: {}", e);
            false
        }
    }
}

/// Remove a reader from a memory region.
#[pg_extern]
fn caliber_region_remove_reader(region_id: pgrx::Uuid, agent_id: pgrx::Uuid) -> bool {
    use pgrx::datum::DatumWithOid;
    use tuple_extract::chrono_to_timestamp;

    let pg_rid = region_id;
    let pg_aid = agent_id;
    let now = chrono_to_timestamp(Utc::now());

    let result: Result<usize, pgrx::spi::SpiError> = Spi::connect_mut(|client| {
        let params: &[DatumWithOid<'_>] = &[
            unsafe { DatumWithOid::new(pg_aid, pgrx::pg_sys::UUIDOID) },
            unsafe { DatumWithOid::new(now, pgrx::pg_sys::TIMESTAMPTZOID) },
            unsafe { DatumWithOid::new(pg_rid, pgrx::pg_sys::UUIDOID) },
        ];
        let table = client.update(
            "UPDATE caliber_region
             SET readers = array_remove(readers, $1), updated_at = $2
             WHERE region_id = $3",
            None,
            params,
        )?;
        Ok::<_, pgrx::spi::SpiError>(table.len())
    });

    match result {
        Ok(len) => len > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to remove reader: {}", e);
            false
        }
    }
}

/// Remove a writer from a memory region.
#[pg_extern]
fn caliber_region_remove_writer(region_id: pgrx::Uuid, agent_id: pgrx::Uuid) -> bool {
    use pgrx::datum::DatumWithOid;
    use tuple_extract::chrono_to_timestamp;

    let pg_rid = region_id;
    let pg_aid = agent_id;
    let now = chrono_to_timestamp(Utc::now());

    let result: Result<usize, pgrx::spi::SpiError> = Spi::connect_mut(|client| {
        let params: &[DatumWithOid<'_>] = &[
            unsafe { DatumWithOid::new(pg_aid, pgrx::pg_sys::UUIDOID) },
            unsafe { DatumWithOid::new(now, pgrx::pg_sys::TIMESTAMPTZOID) },
            unsafe { DatumWithOid::new(pg_rid, pgrx::pg_sys::UUIDOID) },
        ];
        let table = client.update(
            "UPDATE caliber_region
             SET writers = array_remove(writers, $1), updated_at = $2
             WHERE region_id = $3",
            None,
            params,
        )?;
        Ok::<_, pgrx::spi::SpiError>(table.len())
    });

    match result {
        Ok(len) => len > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to remove writer: {}", e);
            false
        }
    }
}


// ============================================================================
// EDGE OPERATIONS (Battle Intel Feature 1)
// ============================================================================

/// Create a new edge (graph relationship).
///
/// Edges can be binary (2 participants) or hyperedges (N participants).
/// Inspired by Mem0's graph-based memory for +2% retrieval improvement.
///
/// # Arguments
/// * `edge_type` - Type of relationship: supports, contradicts, supersedes,
///   derivedfrom, relatesto, temporal, causal, synthesizedfrom, grouped, compared
/// * `participants` - JSON array of participants with entity_type, id, and role
/// * `weight` - Optional relationship strength 0.0-1.0
/// * `trajectory_id` - Optional trajectory context
/// * `source_turn` - Turn where this edge was extracted
/// * `extraction_method` - How edge was created: explicit, inferred, userprovided
/// * `confidence` - Optional confidence score 0.0-1.0
#[pg_extern]
fn caliber_edge_create(
    edge_type: &str,
    participants: pgrx::JsonB,
    weight: Option<f32>,
    trajectory_id: Option<pgrx::Uuid>,
    source_turn: i32,
    extraction_method: &str,
    confidence: Option<f32>,
) -> Option<pgrx::Uuid> {
    // Record operation for metrics
    storage_write().record_op("edge_create");

    let edge_id = new_entity_id();

    // Validate edge_type - reject unknown values (REQ-12)
    let edge_type_enum = match edge_type {
        "supports" => EdgeType::Supports,
        "contradicts" => EdgeType::Contradicts,
        "supersedes" => EdgeType::Supersedes,
        "derivedfrom" => EdgeType::DerivedFrom,
        "relatesto" => EdgeType::RelatesTo,
        "temporal" => EdgeType::Temporal,
        "causal" => EdgeType::Causal,
        "synthesizedfrom" => EdgeType::SynthesizedFrom,
        "grouped" => EdgeType::Grouped,
        "compared" => EdgeType::Compared,
        _ => {
            pgrx::warning!("CALIBER: Unknown edge_type '{}'. Valid values: supports, contradicts, supersedes, derivedfrom, relatesto, temporal, causal, synthesizedfrom, grouped, compared", edge_type);
            return None;
        }
    };

    // Validate extraction_method
    let extraction_method_enum = match extraction_method {
        "explicit" => ExtractionMethod::Explicit,
        "inferred" => ExtractionMethod::Inferred,
        "userprovided" => ExtractionMethod::UserProvided,
        _ => {
            pgrx::warning!("CALIBER: Unknown extraction_method '{}'. Valid values: explicit, inferred, userprovided", extraction_method);
            return None;
        }
    };

    // Parse participants from JSON
    let participants_vec: Vec<caliber_core::EdgeParticipant> =
        match serde_json::from_value(participants.0) {
            Ok(p) => p,
            Err(e) => {
                pgrx::warning!("CALIBER: Failed to parse participants JSON: {}", e);
                return None;
            }
        };

    // Validate at least 2 participants
    if participants_vec.len() < 2 {
        pgrx::warning!("CALIBER: Edge must have at least 2 participants");
        return None;
    }

    // Build Edge struct
    let edge = caliber_core::Edge {
        edge_id,
        edge_type: edge_type_enum,
        participants: participants_vec,
        weight,
        trajectory_id: trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes())),
        provenance: Provenance {
            source_turn,
            extraction_method: extraction_method_enum,
            confidence,
        },
        created_at: Utc::now(),
        metadata: None,
    };

    // Insert via direct heap operations (NO SQL)
    match edge_heap::edge_create_heap(&edge) {
        Ok(_) => Some(pgrx::Uuid::from_bytes(*edge_id.as_bytes())),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to insert edge: {}", e);
            None
        }
    }
}

/// Get an edge by ID.
#[pg_extern]
fn caliber_edge_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let entity_id = Uuid::from_bytes(*id.as_bytes());

    match edge_heap::edge_get_heap(entity_id) {
        Ok(Some(edge)) => {
            let json = serde_json::json!({
                "edge_id": edge.edge_id.to_string(),
                "edge_type": match edge.edge_type {
                    EdgeType::Supports => "supports",
                    EdgeType::Contradicts => "contradicts",
                    EdgeType::Supersedes => "supersedes",
                    EdgeType::DerivedFrom => "derivedfrom",
                    EdgeType::RelatesTo => "relatesto",
                    EdgeType::Temporal => "temporal",
                    EdgeType::Causal => "causal",
                    EdgeType::SynthesizedFrom => "synthesizedfrom",
                    EdgeType::Grouped => "grouped",
                    EdgeType::Compared => "compared",
                },
                "participants": edge.participants,
                "weight": edge.weight,
                "trajectory_id": edge.trajectory_id.map(|id| id.to_string()),
                "provenance": {
                    "source_turn": edge.provenance.source_turn,
                    "extraction_method": match edge.provenance.extraction_method {
                        ExtractionMethod::Explicit => "explicit",
                        ExtractionMethod::Inferred => "inferred",
                        ExtractionMethod::UserProvided => "userprovided",
                    },
                    "confidence": edge.provenance.confidence,
                },
                "created_at": edge.created_at.to_rfc3339(),
                "metadata": edge.metadata,
            });
            Some(pgrx::JsonB(json))
        }
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get edge: {}", e);
            None
        }
    }
}

/// List edges by participant entity.
/// Returns all edges that include the given entity as a participant.
///
/// NOTE: This uses a sequential scan with JSONB containment check.
/// For high-frequency queries, consider adding a GIN index on participants.
/// This is NOT hot path - edge queries are analytical, not per-turn.
#[pg_extern]
fn caliber_edges_by_participant(entity_id: pgrx::Uuid) -> pgrx::JsonB {
    let id = Uuid::from_bytes(*entity_id.as_bytes());

    // Use SPI for JSONB containment query - this is analytical, not hot path
    let result: Result<Vec<serde_json::Value>, pgrx::spi::SpiError> = Spi::connect(|client| {
        let search_json = serde_json::json!([{"entity_ref": {"id": id.to_string()}}]);

        let table = client.select(
            "SELECT edge_id, edge_type, participants, weight, trajectory_id,
                    source_turn, extraction_method, confidence, created_at, metadata
             FROM caliber_edge
             WHERE participants @> $1::jsonb",
            None,
            &[jsonb_datum(&search_json)],
        )?;

        let mut edges = Vec::new();
        for row in table {
            let edge_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
            let edge_type: Option<String> = row.get(2).ok().flatten();
            let participants: Option<pgrx::JsonB> = row.get(3).ok().flatten();
            let weight: Option<f32> = row.get(4).ok().flatten();
            let trajectory_id: Option<pgrx::Uuid> = row.get(5).ok().flatten();
            let source_turn: Option<i32> = row.get(6).ok().flatten();
            let extraction_method: Option<String> = row.get(7).ok().flatten();
            let confidence: Option<f32> = row.get(8).ok().flatten();
            let created_at: Option<TimestampWithTimeZone> = row.get(9).ok().flatten();
            let metadata: Option<pgrx::JsonB> = row.get(10).ok().flatten();

            let edge_json = serde_json::json!({
                "edge_id": edge_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string()),
                "edge_type": edge_type,
                "participants": participants.map(|j| j.0),
                "weight": weight,
                "trajectory_id": trajectory_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string()),
                "provenance": {
                    "source_turn": source_turn,
                    "extraction_method": extraction_method,
                    "confidence": confidence,
                },
                "created_at": created_at.map(|t: TimestampWithTimeZone| t.to_string()),
                "metadata": metadata.map(|j| j.0),
            });
            edges.push(edge_json);
        }
        Ok(edges)
    });

    match result {
        Ok(edges) => pgrx::JsonB(serde_json::json!(edges)),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to list edges by participant: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
}


// ============================================================================
// SUMMARIZATION POLICY OPERATIONS (Battle Intel Feature 4)
// ============================================================================

/// Create a summarization policy.
///
/// Policies define when and how L0→L1→L2 abstraction transitions occur.
/// Inspired by EVOLVE-MEM's self-improvement engine.
///
/// # Arguments
/// * `name` - Policy name
/// * `triggers` - JSON array of trigger definitions
/// * `source_level` - Source abstraction level: raw, summary, principle
/// * `target_level` - Target abstraction level: raw, summary, principle
/// * `max_sources` - Maximum items to summarize at once
/// * `create_edges` - Whether to auto-create SynthesizedFrom edges
/// * `trajectory_id` - Optional trajectory this policy applies to
#[pg_extern]
fn caliber_summarization_policy_create(
    name: &str,
    triggers: pgrx::JsonB,
    source_level: &str,
    target_level: &str,
    max_sources: i32,
    create_edges: bool,
    trajectory_id: Option<pgrx::Uuid>,
) -> Option<pgrx::Uuid> {
    // Record operation for metrics
    storage_write().record_op("summarization_policy_create");

    let policy_id = new_entity_id();

    // Validate abstraction levels
    let source_level_enum = match source_level {
        "raw" => AbstractionLevel::Raw,
        "summary" => AbstractionLevel::Summary,
        "principle" => AbstractionLevel::Principle,
        _ => {
            pgrx::warning!("CALIBER: Unknown source_level '{}'. Valid values: raw, summary, principle", source_level);
            return None;
        }
    };

    let target_level_enum = match target_level {
        "raw" => AbstractionLevel::Raw,
        "summary" => AbstractionLevel::Summary,
        "principle" => AbstractionLevel::Principle,
        _ => {
            pgrx::warning!("CALIBER: Unknown target_level '{}'. Valid values: raw, summary, principle", target_level);
            return None;
        }
    };

    // Validate level transition (must go upward)
    let valid_transition = matches!(
        (source_level_enum, target_level_enum),
        (AbstractionLevel::Raw, AbstractionLevel::Summary)
            | (AbstractionLevel::Raw, AbstractionLevel::Principle)
            | (AbstractionLevel::Summary, AbstractionLevel::Principle)
    );

    if !valid_transition {
        pgrx::warning!("CALIBER: Invalid abstraction level transition. Must go from lower to higher level.");
        return None;
    }

    // Parse triggers from JSON
    let triggers_vec: Vec<SummarizationTrigger> = match serde_json::from_value(triggers.0) {
        Ok(t) => t,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to parse triggers JSON: {}", e);
            return None;
        }
    };

    if triggers_vec.is_empty() {
        pgrx::warning!("CALIBER: At least one trigger is required");
        return None;
    }

    if max_sources <= 0 {
        pgrx::warning!("CALIBER: max_sources must be positive");
        return None;
    }

    // Insert via SPI (no heap operations for policies yet)
    let traj_id = trajectory_id.map(|u| Uuid::from_bytes(*u.as_bytes()));
    let triggers_json = serde_json::to_value(&triggers_vec).unwrap_or(serde_json::Value::Null);

    let result: Result<(), pgrx::spi::SpiError> = Spi::connect_mut(|client| {
        use pgrx::datum::DatumWithOid;
        use tuple_extract::chrono_to_timestamp;

        let now = chrono_to_timestamp(Utc::now());
        let pg_id = pgrx::Uuid::from_bytes(*policy_id.as_bytes());
        let pg_traj_id = traj_id.map(|id| pgrx::Uuid::from_bytes(*id.as_bytes()));

        // Build params with proper OIDs
        let params: Vec<DatumWithOid<'_>> = vec![
            unsafe { DatumWithOid::new(pg_id, pgrx::pg_sys::UUIDOID) },
            unsafe { DatumWithOid::new(name, pgrx::pg_sys::TEXTOID) },
            unsafe { DatumWithOid::new(pgrx::JsonB(triggers_json.clone()), pgrx::pg_sys::JSONBOID) },
            unsafe { DatumWithOid::new(source_level, pgrx::pg_sys::TEXTOID) },
            unsafe { DatumWithOid::new(target_level, pgrx::pg_sys::TEXTOID) },
            unsafe { DatumWithOid::new(max_sources, pgrx::pg_sys::INT4OID) },
            unsafe { DatumWithOid::new(create_edges, pgrx::pg_sys::BOOLOID) },
            match pg_traj_id {
                Some(id) => unsafe { DatumWithOid::new(id, pgrx::pg_sys::UUIDOID) },
                None => unsafe { DatumWithOid::new((), pgrx::pg_sys::UUIDOID) },
            },
            unsafe { DatumWithOid::new(now, pgrx::pg_sys::TIMESTAMPTZOID) },
        ];

        client.update(
            "INSERT INTO caliber_summarization_policy
             (policy_id, name, triggers, source_level, target_level, max_sources, create_edges, trajectory_id, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            None,
            &params,
        )?;

        Ok(())
    });

    match result {
        Ok(()) => Some(pgrx::Uuid::from_bytes(*policy_id.as_bytes())),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to insert summarization policy: {}", e);
            None
        }
    }
}

/// Get a summarization policy by ID.
/// NOTE: Policy reads are config/admin operations, not hot path.
#[pg_extern]
fn caliber_summarization_policy_get(id: pgrx::Uuid) -> Option<pgrx::JsonB> {
    let result: Result<Option<serde_json::Value>, pgrx::spi::SpiError> = Spi::connect(|client| {
        let mut table = client.select(
            "SELECT policy_id, name, triggers, source_level, target_level, max_sources, create_edges, trajectory_id, created_at
             FROM caliber_summarization_policy
             WHERE policy_id = $1",
            None,
            &[pgrx_uuid_datum(id)],
        )?;

        // Iterate and take first row
        if let Some(row) = table.next() {
            let policy_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
            let name: Option<String> = row.get(2).ok().flatten();
            let triggers: Option<pgrx::JsonB> = row.get(3).ok().flatten();
            let source_level: Option<String> = row.get(4).ok().flatten();
            let target_level: Option<String> = row.get(5).ok().flatten();
            let max_sources: Option<i32> = row.get(6).ok().flatten();
            let create_edges: Option<bool> = row.get(7).ok().flatten();
            let trajectory_id: Option<pgrx::Uuid> = row.get(8).ok().flatten();
            let created_at: Option<TimestampWithTimeZone> = row.get(9).ok().flatten();

            let json = serde_json::json!({
                "policy_id": policy_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string()),
                "name": name,
                "triggers": triggers.map(|j| j.0),
                "source_level": source_level,
                "target_level": target_level,
                "max_sources": max_sources,
                "create_edges": create_edges,
                "trajectory_id": trajectory_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string()),
                "created_at": created_at.map(|t: TimestampWithTimeZone| t.to_string()),
            });
            return Ok(Some(json));
        }
        Ok(None)
    });

    match result {
        Ok(Some(json)) => Some(pgrx::JsonB(json)),
        Ok(None) => None,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to get summarization policy: {}", e);
            None
        }
    }
}

/// List summarization policies for a trajectory.
/// NOTE: Policy listing is config/admin operation, not hot path.
#[pg_extern]
fn caliber_summarization_policies_by_trajectory(trajectory_id: pgrx::Uuid) -> pgrx::JsonB {
    let result: Result<Vec<serde_json::Value>, pgrx::spi::SpiError> = Spi::connect(|client| {
        let table = client.select(
            "SELECT policy_id, name, triggers, source_level, target_level, max_sources, create_edges, trajectory_id, created_at
             FROM caliber_summarization_policy
             WHERE trajectory_id = $1
             ORDER BY created_at DESC",
            None,
            &[pgrx_uuid_datum(trajectory_id)],
        )?;

        let mut policies = Vec::new();
        for row in table {
            let policy_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
            let name: Option<String> = row.get(2).ok().flatten();
            let triggers: Option<pgrx::JsonB> = row.get(3).ok().flatten();
            let source_level: Option<String> = row.get(4).ok().flatten();
            let target_level: Option<String> = row.get(5).ok().flatten();
            let max_sources: Option<i32> = row.get(6).ok().flatten();
            let create_edges: Option<bool> = row.get(7).ok().flatten();
            let traj_id: Option<pgrx::Uuid> = row.get(8).ok().flatten();
            let created_at: Option<TimestampWithTimeZone> = row.get(9).ok().flatten();

            let json = serde_json::json!({
                "policy_id": policy_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string()),
                "name": name,
                "triggers": triggers.map(|j| j.0),
                "source_level": source_level,
                "target_level": target_level,
                "max_sources": max_sources,
                "create_edges": create_edges,
                "trajectory_id": traj_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()).to_string()),
                "created_at": created_at.map(|t: TimestampWithTimeZone| t.to_string()),
            });
            policies.push(json);
        }
        Ok(policies)
    });

    match result {
        Ok(policies) => pgrx::JsonB(serde_json::json!(policies)),
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to list summarization policies: {}", e);
            pgrx::JsonB(serde_json::json!([]))
        }
    }
}

/// Delete a summarization policy.
/// NOTE: Policy deletion is config/admin operation, not hot path.
#[pg_extern]
fn caliber_summarization_policy_delete(id: pgrx::Uuid) -> bool {
    let result: Result<usize, pgrx::spi::SpiError> = Spi::connect_mut(|client| {
        let table = client.update(
            "DELETE FROM caliber_summarization_policy WHERE policy_id = $1",
            None,
            &[pgrx_uuid_datum(id)],
        )?;
        Ok(table.len())
    });

    match result {
        Ok(count) => count > 0,
        Err(e) => {
            pgrx::warning!("CALIBER: Failed to delete summarization policy: {}", e);
            false
        }
    }
}


// ============================================================================
// STORAGE TRAIT IMPLEMENTATION (Task 12.3)
// ============================================================================
// Provides unified storage interface. Trajectory methods use SPI,
// other entity types delegate to direct heap operations for performance.

/// PostgreSQL storage implementation.
/// Provides StorageTrait interface - trajectory via SPI, others via heap operations.
pub struct PgStorage;

impl StorageTrait for PgStorage {
    fn trajectory_insert(&self, t: &Trajectory) -> CaliberResult<()> {
        // Insert into SQL table via SPI
        Spi::connect_mut(|client| {
            // Check if trajectory already exists
            let exists_result = client.select(
                "SELECT 1 FROM caliber_trajectory WHERE trajectory_id = $1",
                None,
                &[uuid_datum(t.trajectory_id)],
            );

            if let Ok(table) = exists_result {
                if !table.is_empty() {
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

            let outcome_json = t.outcome.as_ref()
                .map(|o| serde_json::to_value(o).unwrap_or(serde_json::Value::Null));
            let metadata_json = t.metadata.as_ref()
                .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null));

            client.update(
                "INSERT INTO caliber_trajectory (trajectory_id, name, description, status,
                 parent_trajectory_id, root_trajectory_id, agent_id, created_at, updated_at,
                 completed_at, outcome, metadata)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                None,
                &[
                    uuid_datum(t.trajectory_id),
                    text_datum(&t.name),
                    opt_text_datum(t.description.as_deref()),
                    text_datum(status_str),
                    opt_uuid_datum(t.parent_trajectory_id),
                    opt_uuid_datum(t.root_trajectory_id),
                    opt_uuid_datum(t.agent_id),
                    timestamp_datum(t.created_at),
                    timestamp_datum(t.updated_at),
                    opt_timestamp_datum(t.completed_at),
                    opt_jsonb_datum(outcome_json.as_ref()),
                    opt_jsonb_datum(metadata_json.as_ref()),
                ],
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;

            Ok(())
        })
    }

    fn trajectory_get(&self, id: EntityId) -> CaliberResult<Option<Trajectory>> {
        Spi::connect(|client| {
            let mut result = client.select(
                "SELECT trajectory_id, name, description, status, parent_trajectory_id,
                        root_trajectory_id, agent_id, created_at, updated_at, completed_at,
                        outcome, metadata
                 FROM caliber_trajectory WHERE trajectory_id = $1",
                None,
                &[uuid_datum(id)],
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;

            // Iterate over result rows (SpiTupleTable is iterable)
            if let Some(row) = result.next() {
                let trajectory_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                let name: Option<String> = row.get(2).ok().flatten();
                let description: Option<String> = row.get(3).ok().flatten();
                let status_str: Option<String> = row.get(4).ok().flatten();
                let parent_trajectory_id: Option<pgrx::Uuid> = row.get(5).ok().flatten();
                let root_trajectory_id: Option<pgrx::Uuid> = row.get(6).ok().flatten();
                let agent_id_val: Option<pgrx::Uuid> = row.get(7).ok().flatten();
                let created_at: Option<TimestampWithTimeZone> = row.get(8).ok().flatten();
                let updated_at: Option<TimestampWithTimeZone> = row.get(9).ok().flatten();
                let completed_at: Option<TimestampWithTimeZone> = row.get(10).ok().flatten();
                let outcome: Option<pgrx::JsonB> = row.get(11).ok().flatten();
                let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                let status = match status_str.as_deref() {
                    Some("active") => TrajectoryStatus::Active,
                    Some("completed") => TrajectoryStatus::Completed,
                    Some("failed") => TrajectoryStatus::Failed,
                    Some("suspended") => TrajectoryStatus::Suspended,
                    _ => {
                        pgrx::warning!("CALIBER: Unknown trajectory status '{:?}', defaulting to Active", status_str);
                        TrajectoryStatus::Active
                    }
                };

                // Convert pgrx::Uuid to uuid::Uuid
                let traj_id = trajectory_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes())).unwrap_or(id);
                let parent_id = parent_trajectory_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()));
                let root_id = root_trajectory_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()));
                let agent_id = agent_id_val.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()));

                return Ok(Some(Trajectory {
                    trajectory_id: traj_id,
                    name: name.unwrap_or_default(),
                    description,
                    status,
                    parent_trajectory_id: parent_id,
                    root_trajectory_id: root_id,
                    agent_id,
                    created_at: created_at.map(tuple_extract::timestamp_to_chrono).unwrap_or_else(Utc::now),
                    updated_at: updated_at.map(tuple_extract::timestamp_to_chrono).unwrap_or_else(Utc::now),
                    completed_at: completed_at.map(tuple_extract::timestamp_to_chrono),
                    outcome: outcome.and_then(|j| serde_json::from_value(j.0).ok()),
                    metadata: metadata.and_then(|j| serde_json::from_value(j.0).ok()),
                }));
            }
            Ok(None)
        })
    }

    fn trajectory_update(&self, id: EntityId, update: TrajectoryUpdate) -> CaliberResult<()> {
        Spi::connect_mut(|client| {
            // First check if trajectory exists
            let exists_result = client.select(
                "SELECT 1 FROM caliber_trajectory WHERE trajectory_id = $1",
                None,
                &[uuid_datum(id)],
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;

            if exists_result.is_empty() {
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
                    &[
                        text_datum(status_str),
                        timestamp_datum(now),
                        uuid_datum(id),
                    ],
                ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;
            }

            if let Some(metadata) = update.metadata {
                let metadata_json = serde_json::to_value(&metadata).unwrap_or(serde_json::Value::Null);
                client.update(
                    "UPDATE caliber_trajectory SET metadata = $1, updated_at = $2 WHERE trajectory_id = $3",
                    None,
                    &[
                        jsonb_datum(&metadata_json),
                        timestamp_datum(now),
                        uuid_datum(id),
                    ],
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
                &[text_datum(status_str)],
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;

            let mut trajectories = Vec::new();
            for row in result {
                let trajectory_id: Option<pgrx::Uuid> = row.get(1).ok().flatten();
                let name: Option<String> = row.get(2).ok().flatten();
                let description: Option<String> = row.get(3).ok().flatten();
                let status_str_val: Option<String> = row.get(4).ok().flatten();
                let parent_trajectory_id: Option<pgrx::Uuid> = row.get(5).ok().flatten();
                let root_trajectory_id: Option<pgrx::Uuid> = row.get(6).ok().flatten();
                let agent_id_val: Option<pgrx::Uuid> = row.get(7).ok().flatten();
                let created_at: Option<TimestampWithTimeZone> = row.get(8).ok().flatten();
                let updated_at: Option<TimestampWithTimeZone> = row.get(9).ok().flatten();
                let completed_at: Option<TimestampWithTimeZone> = row.get(10).ok().flatten();
                let outcome: Option<pgrx::JsonB> = row.get(11).ok().flatten();
                let metadata: Option<pgrx::JsonB> = row.get(12).ok().flatten();

                let traj_status = match status_str_val.as_deref() {
                    Some("active") => TrajectoryStatus::Active,
                    Some("completed") => TrajectoryStatus::Completed,
                    Some("failed") => TrajectoryStatus::Failed,
                    Some("suspended") => TrajectoryStatus::Suspended,
                    _ => {
                        pgrx::warning!("CALIBER: Unknown trajectory status '{:?}', defaulting to Active", status_str_val);
                        TrajectoryStatus::Active
                    }
                };

                if let Some(tid) = trajectory_id {
                    // Convert pgrx::Uuid to uuid::Uuid with explicit type annotations
                    let parent_id = parent_trajectory_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()));
                    let root_id = root_trajectory_id.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()));
                    let agent_id = agent_id_val.map(|u: pgrx::Uuid| Uuid::from_bytes(*u.as_bytes()));

                    trajectories.push(Trajectory {
                        trajectory_id: Uuid::from_bytes(*tid.as_bytes()),
                        name: name.unwrap_or_default(),
                        description,
                        status: traj_status,
                        parent_trajectory_id: parent_id,
                        root_trajectory_id: root_id,
                        agent_id,
                        created_at: created_at.map(tuple_extract::timestamp_to_chrono).unwrap_or_else(Utc::now),
                        updated_at: updated_at.map(tuple_extract::timestamp_to_chrono).unwrap_or_else(Utc::now),
                        completed_at: completed_at.map(tuple_extract::timestamp_to_chrono),
                        outcome: outcome.and_then(|j| serde_json::from_value(j.0).ok()),
                        metadata: metadata.and_then(|j| serde_json::from_value(j.0).ok()),
                    });
                }
            }
            Ok(trajectories)
        })
    }

    fn scope_insert(&self, s: &Scope) -> CaliberResult<()> {
        // Check if already exists using heap module
        if scope_heap::scope_get_heap(s.scope_id)?.is_some() {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Scope,
                reason: "already exists".to_string(),
            }));
        }
        scope_heap::scope_create_heap(
            s.scope_id,
            s.trajectory_id,
            &s.name,
            s.purpose.as_deref(),
            s.token_budget,
        )?;
        Ok(())
    }

    fn scope_get(&self, id: EntityId) -> CaliberResult<Option<Scope>> {
        scope_heap::scope_get_heap(id)
    }

    fn scope_get_current(&self, trajectory_id: EntityId) -> CaliberResult<Option<Scope>> {
        // Get all scopes for trajectory and find the active one with latest created_at
        let scopes = scope_heap::scope_list_by_trajectory_heap(trajectory_id)?;
        Ok(scopes
            .into_iter()
            .filter(|s| s.is_active)
            .max_by_key(|s| s.created_at))
    }

    fn scope_update(&self, id: EntityId, update: ScopeUpdate) -> CaliberResult<()> {
        // Verify scope exists
        let existing = scope_heap::scope_get_heap(id)?;
        if existing.is_none() {
            return Err(CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Scope,
                id,
            }));
        }

        // For now, handle close specifically (most common update)
        if update.is_active == Some(false) {
            scope_heap::scope_close_heap(id)?;
        }
        // Token updates via dedicated function
        if let Some(tokens) = update.tokens_used {
            scope_heap::scope_update_tokens_heap(id, tokens)?;
        }
        Ok(())
    }

    fn scope_list_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<Vec<Scope>> {
        scope_heap::scope_list_by_trajectory_heap(trajectory_id)
    }

    fn artifact_insert(&self, a: &Artifact) -> CaliberResult<()> {
        // Check if already exists using heap module
        if artifact_heap::artifact_get_heap(a.artifact_id)?.is_some() {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Artifact,
                reason: "already exists".to_string(),
            }));
        }
        artifact_heap::artifact_create_heap(artifact_heap::ArtifactCreateParams {
            artifact_id: a.artifact_id,
            trajectory_id: a.trajectory_id,
            scope_id: a.scope_id,
            artifact_type: a.artifact_type,
            name: &a.name,
            content: &a.content,
            content_hash: a.content_hash,
            embedding: a.embedding.as_ref(),
            provenance: &a.provenance,
            ttl: a.ttl.clone(),
        })?;
        Ok(())
    }

    fn artifact_get(&self, id: EntityId) -> CaliberResult<Option<Artifact>> {
        artifact_heap::artifact_get_heap(id)
    }

    fn artifact_query_by_type(
        &self,
        trajectory_id: EntityId,
        artifact_type: ArtifactType,
    ) -> CaliberResult<Vec<Artifact>> {
        // Get all artifacts of type and filter by trajectory
        let artifacts = artifact_heap::artifact_query_by_type_heap(artifact_type)?;
        Ok(artifacts
            .into_iter()
            .filter(|a| a.trajectory_id == trajectory_id)
            .collect())
    }

    fn artifact_query_by_scope(&self, scope_id: EntityId) -> CaliberResult<Vec<Artifact>> {
        artifact_heap::artifact_query_by_scope_heap(scope_id)
    }

    fn artifact_update(&self, id: EntityId, update: ArtifactUpdate) -> CaliberResult<()> {
        // Compute content hash if content is being updated
        let content_hash = update.content.as_ref().map(|c| compute_content_hash(c.as_bytes()));
        // Convert embedding to the nested Option format (Some(Some(x)) = update, Some(None) = set to null, None = don't update)
        let embedding_opt = update.embedding.as_ref().map(Some);
        // Same for superseded_by
        let superseded_opt = update.superseded_by.map(Some);

        artifact_heap::artifact_update_heap(
            id,
            update.content.as_deref(),
            content_hash,
            embedding_opt,
            superseded_opt,
            None, // metadata not in ArtifactUpdate struct
        )?;
        Ok(())
    }

    fn note_insert(&self, n: &Note) -> CaliberResult<()> {
        // Check if already exists using heap module
        if note_heap::note_get_heap(n.note_id)?.is_some() {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Note,
                reason: "already exists".to_string(),
            }));
        }
        note_heap::note_create_heap(note_heap::NoteCreateParams {
            note_id: n.note_id,
            note_type: n.note_type,
            title: &n.title,
            content: &n.content,
            content_hash: n.content_hash,
            embedding: n.embedding.as_ref(),
            source_trajectory_ids: &n.source_trajectory_ids,
            source_artifact_ids: &n.source_artifact_ids,
            ttl: n.ttl.clone(),
            abstraction_level: n.abstraction_level,
            source_note_ids: &n.source_note_ids,
        })?;
        Ok(())
    }

    fn note_get(&self, id: EntityId) -> CaliberResult<Option<Note>> {
        note_heap::note_get_heap(id)
    }

    fn note_query_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<Vec<Note>> {
        note_heap::note_query_by_trajectory_heap(trajectory_id)
    }

    fn note_update(&self, id: EntityId, update: NoteUpdate) -> CaliberResult<()> {
        // Compute content hash if content is being updated
        let content_hash = update.content.as_ref().map(|c| compute_content_hash(c.as_bytes()));
        // Convert embedding to the nested Option format (Some(Some(x)) = update, Some(None) = set to null, None = don't update)
        let embedding_opt = update.embedding.as_ref().map(Some);
        // Same for superseded_by
        let superseded_opt = update.superseded_by.map(Some);

        note_heap::note_update_heap(
            id,
            update.content.as_deref(),
            content_hash,
            embedding_opt,
            superseded_opt,
            None, // metadata not in NoteUpdate struct
        )?;
        Ok(())
    }

    fn turn_insert(&self, t: &Turn) -> CaliberResult<()> {
        turn_heap::turn_create_heap(turn_heap::TurnCreateParams {
            turn_id: t.turn_id,
            scope_id: t.scope_id,
            sequence: t.sequence,
            role: t.role,
            content: &t.content,
            token_count: t.token_count,
            tool_calls: t.tool_calls.as_ref(),
            tool_results: t.tool_results.as_ref(),
        })?;
        Ok(())
    }

    fn turn_get_by_scope(&self, scope_id: EntityId) -> CaliberResult<Vec<Turn>> {
        turn_heap::turn_get_by_scope_heap(scope_id)
    }

    fn vector_search(
        &self,
        query: &EmbeddingVector,
        limit: i32,
    ) -> CaliberResult<Vec<(EntityId, f32)>> {
        // Vector search requires scanning all artifacts and notes with embeddings.
        // In production, this would use pgvector's index-based search via SPI.
        // For now, use a hybrid approach - query via SPI with vector operators.

        Spi::connect(|client| {
            let mut results: Vec<(EntityId, f32)> = Vec::new();

            // Use pgvector's cosine similarity operator if available
            // Query format: embedding <=> $1 returns cosine distance
            let query_vec: Vec<f32> = query.data.clone();
            let query_str = format!("[{}]", query_vec.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(","));

            // Search artifacts with embeddings
            let artifact_query = format!(
                "SELECT artifact_id, 1 - (embedding <=> '{}') as similarity \
                 FROM caliber_artifact \
                 WHERE embedding IS NOT NULL \
                 ORDER BY embedding <=> '{}' \
                 LIMIT {}",
                query_str, query_str, limit
            );

            if let Ok(artifact_results) = client.select(&artifact_query, None, &[]) {
                for row in artifact_results {
                    if let (Ok(Some(id)), Ok(Some(sim))) = (row.get::<pgrx::Uuid>(1), row.get::<f32>(2)) {
                        results.push((Uuid::from_bytes(*id.as_bytes()), sim));
                    }
                }
            }

            // Search notes with embeddings
            let note_query = format!(
                "SELECT note_id, 1 - (embedding <=> '{}') as similarity \
                 FROM caliber_note \
                 WHERE embedding IS NOT NULL \
                 ORDER BY embedding <=> '{}' \
                 LIMIT {}",
                query_str, query_str, limit
            );

            if let Ok(note_results) = client.select(&note_query, None, &[]) {
                for row in note_results {
                    if let (Ok(Some(id)), Ok(Some(sim))) = (row.get::<pgrx::Uuid>(1), row.get::<f32>(2)) {
                        results.push((Uuid::from_bytes(*id.as_bytes()), sim));
                    }
                }
            }

            // Sort combined results by similarity
            results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(limit as usize);

            Ok(results)
        })
    }

    // === Edge Operations (Battle Intel Feature 1) ===

    fn edge_insert(&self, e: &Edge) -> CaliberResult<()> {
        edge_heap::edge_create_heap(e)?;
        Ok(())
    }

    fn edge_get(&self, id: EntityId) -> CaliberResult<Option<Edge>> {
        edge_heap::edge_get_heap(id)
    }

    fn edge_query_by_type(&self, edge_type: EdgeType) -> CaliberResult<Vec<Edge>> {
        edge_heap::edge_query_by_type_heap(edge_type)
    }

    fn edge_query_by_trajectory(&self, trajectory_id: EntityId) -> CaliberResult<Vec<Edge>> {
        edge_heap::edge_query_by_trajectory_heap(trajectory_id)
    }

    fn edge_query_by_participant(&self, entity_id: EntityId) -> CaliberResult<Vec<Edge>> {
        // Query edges and filter by participant - uses SPI since we need JSON filtering
        Spi::connect(|client| {
            let result = client.select(
                "SELECT edge_id, edge_type, participants, weight, trajectory_id,
                        source_turn, extraction_method, confidence, created_at, metadata
                 FROM caliber_edge
                 WHERE participants @> $1::jsonb",
                None,
                &[text_datum(&format!(r#"[{{"id":"{}"}}]"#, entity_id))],
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;

            let mut edges = Vec::new();
            for row in result {
                // Parse each row into Edge struct
                if let Ok(Some(edge_id_pg)) = row.get::<pgrx::Uuid>(1) {
                    let edge_id = Uuid::from_bytes(*edge_id_pg.as_bytes());
                    if let Some(edge) = edge_heap::edge_get_heap(edge_id)? {
                        edges.push(edge);
                    }
                }
            }
            Ok(edges)
        })
    }

    // === Note Abstraction Level Queries (Battle Intel Feature 2) ===

    fn note_query_by_abstraction_level(
        &self,
        level: AbstractionLevel,
    ) -> CaliberResult<Vec<Note>> {
        let level_str = match level {
            AbstractionLevel::Raw => "raw",
            AbstractionLevel::Summary => "summary",
            AbstractionLevel::Principle => "principle",
        };

        Spi::connect(|client| {
            let result = client.select(
                "SELECT note_id FROM caliber_note WHERE abstraction_level = $1",
                None,
                &[text_datum(level_str)],
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;

            let mut notes = Vec::new();
            for row in result {
                if let Ok(Some(note_id_pg)) = row.get::<pgrx::Uuid>(1) {
                    let note_id = Uuid::from_bytes(*note_id_pg.as_bytes());
                    if let Some(note) = note_heap::note_get_heap(note_id)? {
                        notes.push(note);
                    }
                }
            }
            Ok(notes)
        })
    }

    fn note_query_by_source_note(&self, source_note_id: EntityId) -> CaliberResult<Vec<Note>> {
        Spi::connect(|client| {
            let result = client.select(
                "SELECT note_id FROM caliber_note WHERE $1 = ANY(source_note_ids)",
                None,
                &[uuid_datum(source_note_id)],
            ).map_err(|e| CaliberError::Storage(StorageError::SpiError { reason: e.to_string() }))?;

            let mut notes = Vec::new();
            for row in result {
                if let Ok(Some(note_id_pg)) = row.get::<pgrx::Uuid>(1) {
                    let note_id = Uuid::from_bytes(*note_id_pg.as_bytes());
                    if let Some(note) = note_heap::note_get_heap(note_id)? {
                        notes.push(note);
                    }
                }
            }
            Ok(notes)
        })
    }
}


// ============================================================================
// PGRX INTEGRATION TESTS (Task 12.8)
// ============================================================================

#[cfg(feature = "pg_test")]
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
        assert_eq!(updated, Some(true));

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
        )
        .expect("artifact should be created");

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
        )
        .expect("note should be created");

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

        let caps_value = serde_json::json!([]);
        let agent1 = crate::caliber_agent_register("sender", pgrx::JsonB(caps_value.clone()));
        let agent2 = crate::caliber_agent_register("receiver", pgrx::JsonB(caps_value));

        // Send message
        let msg_id = crate::caliber_message_send(
            agent1,
            Some(agent2),
            None,
            "heartbeat",
            "{}",
            "normal",
        )
        .expect("message should be sent");

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

        let caps_value = serde_json::json!([]);
        let delegator = crate::caliber_agent_register("planner", pgrx::JsonB(caps_value.clone()));
        let delegatee = crate::caliber_agent_register("coder", pgrx::JsonB(caps_value));
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

        let caps_value = serde_json::json!([]);
        let agent1 = crate::caliber_agent_register("generalist", pgrx::JsonB(caps_value.clone()));
        let agent2 = crate::caliber_agent_register("specialist", pgrx::JsonB(caps_value));
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
        
        assert_eq!(obj["entity_counts"]["trajectories"], 1);
        assert_eq!(obj["entity_counts"]["agents"], 1);
    }
}

