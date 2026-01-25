//! Direct heap operations for Trajectory entities.
//!
//! This module provides hot-path operations for trajectories that bypass SQL
//! parsing entirely by using direct heap access via pgrx.
//!
//! # Operations
//!
//! - `trajectory_create_heap` - Insert a new trajectory
//! - `trajectory_get_heap` - Get a trajectory by ID
//! - `trajectory_update_heap` - Update trajectory fields
//! - `trajectory_set_status_heap` - Update trajectory status
//! - `trajectory_list_by_status_heap` - List trajectories by status

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    AgentId, CaliberError, CaliberResult, EntityIdType, EntityType, StorageError,
    TenantId, Trajectory, TrajectoryId, TrajectoryOutcome, TrajectoryStatus,
};

use crate::column_maps::trajectory;
use crate::heap_ops::{
    current_timestamp, form_tuple, insert_tuple, open_relation,
    update_tuple, PgLockMode as LockMode, HeapRelation, get_active_snapshot,
    timestamp_to_pgrx,
};
use crate::index_ops::{
    init_scan_key, open_index, update_indexes_for_insert,
    BTreeStrategy, IndexScanner, operator_oids,
};
use crate::tuple_extract::{
    extract_uuid, extract_text, extract_timestamp, extract_jsonb,
    extract_values_and_nulls, uuid_to_datum, string_to_datum,
    option_string_to_datum, option_uuid_to_datum, json_to_datum,
    option_json_to_datum, timestamp_to_chrono,
};

/// Trajectory row with tenant ownership metadata.
pub struct TrajectoryRow {
    pub trajectory: Trajectory,
    pub tenant_id: Option<TenantId>,
}

impl From<TrajectoryRow> for Trajectory {
    fn from(row: TrajectoryRow) -> Self {
        row.trajectory
    }
}

/// Create a new trajectory using direct heap operations.
///
/// This bypasses SQL parsing entirely for hot-path performance.
///
/// # Arguments
/// * `trajectory_id` - The pre-generated UUIDv7 for this trajectory
/// * `name` - The trajectory name (required)
/// * `description` - Optional description
/// * `agent_id` - Optional agent ID that owns this trajectory
///
/// # Returns
/// * `Ok(TrajectoryId)` - The trajectory ID on success
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 1.1: Uses heap_form_tuple and simple_heap_insert instead of SPI
/// - 1.6: Updates all relevant indexes via CatalogIndexInsert
pub fn trajectory_create_heap(
    trajectory_id: TrajectoryId,
    name: &str,
    description: Option<&str>,
    agent_id: Option<AgentId>,
    tenant_id: TenantId,
) -> CaliberResult<TrajectoryId> {
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(trajectory::TABLE_NAME, LockMode::RowExclusive)?;

    // Validate relation schema matches expectations
    validate_trajectory_relation(&rel)?;

    // Get current transaction timestamp for created_at/updated_at
    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now)?.into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Trajectory,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    // Build datum array - must match column order in caliber_trajectory table
    let mut values: [pg_sys::Datum; trajectory::NUM_COLS] = [pg_sys::Datum::from(0); trajectory::NUM_COLS];
    let mut nulls: [bool; trajectory::NUM_COLS] = [false; trajectory::NUM_COLS];

    // Use helper for optional fields (description, agent_id, outcome, metadata)
    let (desc_datum, agent_datum, outcome_datum, metadata_datum,
         desc_null, agent_null, outcome_null, metadata_null) =
        build_optional_datums(description, agent_id, None, None);

    // Column 1: trajectory_id (UUID, NOT NULL)
    values[trajectory::TRAJECTORY_ID as usize - 1] = uuid_to_datum(trajectory_id.as_uuid());

    // Column 2: name (TEXT, NOT NULL)
    values[trajectory::NAME as usize - 1] = string_to_datum(name);

    // Column 3: description (TEXT, nullable)
    values[trajectory::DESCRIPTION as usize - 1] = desc_datum;
    nulls[trajectory::DESCRIPTION as usize - 1] = desc_null;

    // Column 4: status (TEXT, NOT NULL) - default to "active"
    values[trajectory::STATUS as usize - 1] = string_to_datum("active");

    // Column 5: parent_trajectory_id (UUID, nullable)
    nulls[trajectory::PARENT_TRAJECTORY_ID as usize - 1] = true;

    // Column 6: root_trajectory_id (UUID, nullable)
    nulls[trajectory::ROOT_TRAJECTORY_ID as usize - 1] = true;

    // Column 7: agent_id (UUID, nullable)
    values[trajectory::AGENT_ID as usize - 1] = agent_datum;
    nulls[trajectory::AGENT_ID as usize - 1] = agent_null;

    // Column 8: created_at (TIMESTAMPTZ, NOT NULL)
    values[trajectory::CREATED_AT as usize - 1] = now_datum;

    // Column 9: updated_at (TIMESTAMPTZ, NOT NULL)
    values[trajectory::UPDATED_AT as usize - 1] = now_datum;

    // Column 10: completed_at (TIMESTAMPTZ, nullable)
    nulls[trajectory::COMPLETED_AT as usize - 1] = true;

    // Column 11: outcome (JSONB, nullable)
    values[trajectory::OUTCOME as usize - 1] = outcome_datum;
    nulls[trajectory::OUTCOME as usize - 1] = outcome_null;

    // Column 12: metadata (JSONB, nullable)
    values[trajectory::METADATA as usize - 1] = metadata_datum;
    nulls[trajectory::METADATA as usize - 1] = metadata_null;

    // Column 13: tenant_id (UUID, NOT NULL)
    values[trajectory::TENANT_ID as usize - 1] = uuid_to_datum(tenant_id.as_uuid());
    
    // Form the heap tuple
    let tuple = form_tuple(&rel, &values, &nulls)?;
    
    // Insert into heap
    let _tid = unsafe { insert_tuple(&rel, tuple)? };
    
    // Update all indexes via CatalogIndexInsert
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };
    
    Ok(trajectory_id)
}


/// Get a trajectory by ID using direct heap operations.
///
/// This bypasses SQL parsing entirely for hot-path performance.
///
/// # Arguments
/// * `id` - The trajectory ID to look up
///
/// # Returns
/// * `Ok(Some(Trajectory))` - The trajectory if found
/// * `Ok(None)` - If no trajectory with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 1.2: Uses index_beginscan for O(log n) lookup instead of SPI SELECT
/// - 1.7: Returns None without SQL error overhead if not found
pub fn trajectory_get_heap(id: TrajectoryId, tenant_id: TenantId) -> CaliberResult<Option<TrajectoryRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(trajectory::TABLE_NAME, LockMode::AccessShare)?;
    
    // Open the primary key index
    let index_rel = open_index(trajectory::PK_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for primary key lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (trajectory_id)
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(id.as_uuid()),
    );
    
    // Create index scanner
    let mut scanner = unsafe { IndexScanner::new(
        &rel,
        &index_rel,
        snapshot,
        1,
        &mut scan_key,
    ) };
    
    // Get the first (and should be only) matching tuple
    if let Some(tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let row = unsafe { tuple_to_trajectory(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid()) {
            Ok(Some(row))
        } else {
            Ok(None)
        }
    } else {
        // Not found - return None per requirement 1.7
        Ok(None)
    }
}

/// Update a trajectory with the provided fields using direct heap operations.
///
/// # Arguments
/// * `id` - The trajectory ID to update
/// * `name` - Optional new name
/// * `description` - Optional new description (Some(None) to clear)
/// * `status` - Optional new status
/// * `parent_trajectory_id` - Optional new parent (Some(None) to clear)
/// * `root_trajectory_id` - Optional new root (Some(None) to clear)
/// * `agent_id` - Optional new agent (Some(None) to clear)
/// * `outcome` - Optional outcome (Some(None) to clear)
/// * `metadata` - Optional metadata (Some(None) to clear)
///
/// # Returns
/// * `Ok(true)` - If the trajectory was found and updated
/// * `Ok(false)` - If no trajectory with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 1.3: Uses simple_heap_update instead of SPI UPDATE
pub struct TrajectoryUpdateHeapParams<'a> {
    pub id: TrajectoryId,
    pub tenant_id: TenantId,
    pub name: Option<&'a str>,
    pub description: Option<Option<&'a str>>,
    pub status: Option<TrajectoryStatus>,
    pub parent_trajectory_id: Option<Option<TrajectoryId>>,
    pub root_trajectory_id: Option<Option<TrajectoryId>>,
    pub agent_id: Option<Option<AgentId>>,
    pub outcome: Option<Option<&'a TrajectoryOutcome>>,
    pub metadata: Option<Option<&'a serde_json::Value>>,
}

pub fn trajectory_update_heap(params: TrajectoryUpdateHeapParams<'_>) -> CaliberResult<bool> {
    let TrajectoryUpdateHeapParams {
        id,
        tenant_id,
        name,
        description,
        status,
        parent_trajectory_id,
        root_trajectory_id,
        agent_id,
        outcome,
        metadata,
    } = params;
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(trajectory::TABLE_NAME, LockMode::RowExclusive)?;
    
    // Open the primary key index
    let index_rel = open_index(trajectory::PK_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for primary key lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(id.as_uuid()),
    );
    
    // Create index scanner
    let mut scanner = unsafe { IndexScanner::new(
        &rel,
        &index_rel,
        snapshot,
        1,
        &mut scan_key,
    ) };
    
    // Find the existing tuple
    let old_tuple = match scanner.next() {
        Some(t) => t,
        None => return Ok(false), // Not found
    };
    
    let tid = scanner.current_tid()
        .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
            entity_type: EntityType::Trajectory,
            id,
            reason: "Failed to get TID of existing tuple".to_string(),
        }))?;
    
    let tuple_desc = rel.tuple_desc();
    let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, trajectory::TENANT_ID)? };
    if existing_tenant != Some(tenant_id.as_uuid()) {
        return Ok(false);
    }
    
    // Extract current values and nulls
    let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;
    
    // Apply updates
    if let Some(new_name) = name {
        values[trajectory::NAME as usize - 1] = string_to_datum(new_name);
    }
    
    if let Some(new_desc) = description {
        match new_desc {
            Some(d) => {
                values[trajectory::DESCRIPTION as usize - 1] = string_to_datum(d);
                nulls[trajectory::DESCRIPTION as usize - 1] = false;
            }
            None => {
                nulls[trajectory::DESCRIPTION as usize - 1] = true;
            }
        }
    }
    
    if let Some(new_status) = status {
        values[trajectory::STATUS as usize - 1] = string_to_datum(status_to_str(new_status));
        
        // If status is completed or failed, set completed_at
        if new_status == TrajectoryStatus::Completed || new_status == TrajectoryStatus::Failed {
            let now = current_timestamp();
            values[trajectory::COMPLETED_AT as usize - 1] = timestamp_to_pgrx(now)?
                .into_datum()
                .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
                    entity_type: EntityType::Trajectory,
                    id,
                    reason: "Failed to convert timestamp to datum".to_string(),
                }))?;
            nulls[trajectory::COMPLETED_AT as usize - 1] = false;
        }
    }
    
    if let Some(new_parent) = parent_trajectory_id {
        match new_parent {
            Some(p) => {
                values[trajectory::PARENT_TRAJECTORY_ID as usize - 1] = uuid_to_datum(p);
                nulls[trajectory::PARENT_TRAJECTORY_ID as usize - 1] = false;
            }
            None => {
                nulls[trajectory::PARENT_TRAJECTORY_ID as usize - 1] = true;
            }
        }
    }
    
    if let Some(new_root) = root_trajectory_id {
        match new_root {
            Some(r) => {
                values[trajectory::ROOT_TRAJECTORY_ID as usize - 1] = uuid_to_datum(r);
                nulls[trajectory::ROOT_TRAJECTORY_ID as usize - 1] = false;
            }
            None => {
                nulls[trajectory::ROOT_TRAJECTORY_ID as usize - 1] = true;
            }
        }
    }
    
    if let Some(new_agent) = agent_id {
        match new_agent {
            Some(a) => {
                values[trajectory::AGENT_ID as usize - 1] = uuid_to_datum(a);
                nulls[trajectory::AGENT_ID as usize - 1] = false;
            }
            None => {
                nulls[trajectory::AGENT_ID as usize - 1] = true;
            }
        }
    }
    
    if let Some(new_outcome) = outcome {
        match new_outcome {
            Some(o) => {
                let outcome_json = serde_json::to_value(o)
                    .map_err(|e| CaliberError::Storage(StorageError::UpdateFailed {
                        entity_type: EntityType::Trajectory,
                        id,
                        reason: format!("Failed to serialize outcome: {}", e),
                    }))?;
                values[trajectory::OUTCOME as usize - 1] = json_to_datum(&outcome_json);
                nulls[trajectory::OUTCOME as usize - 1] = false;
            }
            None => {
                nulls[trajectory::OUTCOME as usize - 1] = true;
            }
        }
    }
    
    if let Some(new_metadata) = metadata {
        match new_metadata {
            Some(m) => {
                values[trajectory::METADATA as usize - 1] = json_to_datum(m);
                nulls[trajectory::METADATA as usize - 1] = false;
            }
            None => {
                nulls[trajectory::METADATA as usize - 1] = true;
            }
        }
    }
    
    // Always update updated_at
    let now = current_timestamp();
    values[trajectory::UPDATED_AT as usize - 1] = timestamp_to_pgrx(now)?
        .into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
            entity_type: EntityType::Trajectory,
            id,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    // Form new tuple
    let new_tuple = form_tuple(&rel, &values, &nulls)?;
    
    // Update in place
    unsafe { update_tuple(&rel, &tid, new_tuple)? };
    
    // Update indexes (status index may have changed)
    unsafe { update_indexes_for_insert(&rel, new_tuple, &values, &nulls)? };
    
    Ok(true)
}

/// Update trajectory status using direct heap operations.
///
/// This is a specialized update for just the status field.
///
/// # Arguments
/// * `id` - The trajectory ID to update
/// * `status` - The new status
///
/// # Returns
/// * `Ok(true)` - If the trajectory was found and updated
/// * `Ok(false)` - If no trajectory with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 1.4: Uses simple_heap_update instead of SPI UPDATE
pub fn trajectory_set_status_heap(
    id: TrajectoryId,
    status: TrajectoryStatus,
    tenant_id: TenantId,
) -> CaliberResult<bool> {
    trajectory_update_heap(TrajectoryUpdateHeapParams {
        id,
        tenant_id,
        name: None,
        description: None,
        status: Some(status),
        parent_trajectory_id: None,
        root_trajectory_id: None,
        agent_id: None,
        outcome: None,
        metadata: None,
    })
}

/// List trajectories by status using direct heap operations.
///
/// # Arguments
/// * `status` - The status to filter by
///
/// # Returns
/// * `Ok(Vec<Trajectory>)` - List of matching trajectories
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 1.5: Uses heap_beginscan with index for filtering instead of SPI SELECT
pub fn trajectory_list_by_status_heap(
    status: TrajectoryStatus,
    tenant_id: TenantId,
) -> CaliberResult<Vec<TrajectoryRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(trajectory::TABLE_NAME, LockMode::AccessShare)?;
    
    // Open the status index
    let index_rel = open_index(trajectory::STATUS_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for status lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (status)
        BTreeStrategy::Equal,
        operator_oids::TEXT_EQ,
        string_to_datum(status_to_str(status)),
    );
    
    // Create index scanner
    let mut scanner = unsafe { IndexScanner::new(
        &rel,
        &index_rel,
        snapshot,
        1,
        &mut scan_key,
    ) };
    
    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();
    
    // Collect all matching tuples
    for tuple in &mut scanner {
        let row = unsafe { tuple_to_trajectory(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid()) {
            results.push(row);
        }
    }
    
    Ok(results)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Validate that a HeapRelation is suitable for trajectory operations.
/// This ensures the relation schema matches what we expect before operations.
fn validate_trajectory_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != trajectory::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Trajectory relation has {} columns, expected {}",
                natts,
                trajectory::NUM_COLS
            ),
        }));
    }
    Ok(())
}

/// Build datum values for optional trajectory fields.
/// Uses option_string_to_datum, option_uuid_to_datum, option_json_to_datum
/// for proper NULL handling in PostgreSQL.
fn build_optional_datums(
    description: Option<&str>,
    agent_id: Option<AgentId>,
    outcome: Option<&serde_json::Value>,
    metadata: Option<&serde_json::Value>,
) -> (pg_sys::Datum, pg_sys::Datum, pg_sys::Datum, pg_sys::Datum, bool, bool, bool, bool) {
    let (desc_datum, desc_null) = if let Some(d) = description {
        (option_string_to_datum(Some(d)), false)
    } else {
        (pg_sys::Datum::from(0), true)
    };

    let (agent_datum, agent_null) = if let Some(a) = agent_id {
        (option_uuid_to_datum(Some(a.as_uuid())), false)
    } else {
        (pg_sys::Datum::from(0), true)
    };

    let (outcome_datum, outcome_null) = if let Some(o) = outcome {
        (option_json_to_datum(Some(o)), false)
    } else {
        (pg_sys::Datum::from(0), true)
    };

    let (metadata_datum, metadata_null) = if let Some(m) = metadata {
        (option_json_to_datum(Some(m)), false)
    } else {
        (pg_sys::Datum::from(0), true)
    };

    (desc_datum, agent_datum, outcome_datum, metadata_datum,
     desc_null, agent_null, outcome_null, metadata_null)
}

/// Convert a TrajectoryStatus enum to its string representation.
fn status_to_str(status: TrajectoryStatus) -> &'static str {
    match status {
        TrajectoryStatus::Active => "active",
        TrajectoryStatus::Completed => "completed",
        TrajectoryStatus::Failed => "failed",
        TrajectoryStatus::Suspended => "suspended",
    }
}

/// Parse a status string to TrajectoryStatus enum.
fn str_to_status(s: &str) -> TrajectoryStatus {
    match s {
        "active" => TrajectoryStatus::Active,
        "completed" => TrajectoryStatus::Completed,
        "failed" => TrajectoryStatus::Failed,
        "suspended" => TrajectoryStatus::Suspended,
        _ => {
            pgrx::warning!("CALIBER: Unknown trajectory status '{}', defaulting to Active", s);
            TrajectoryStatus::Active
        }
    }
}

/// Convert a heap tuple to a Trajectory struct.
unsafe fn tuple_to_trajectory(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<TrajectoryRow> {
    // Extract all fields from the tuple
    let trajectory_id = extract_uuid(tuple, tuple_desc, trajectory::TRAJECTORY_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "trajectory_id is NULL".to_string(),
        }))?;
    
    let name = extract_text(tuple, tuple_desc, trajectory::NAME)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "name is NULL".to_string(),
        }))?;
    
    let description = extract_text(tuple, tuple_desc, trajectory::DESCRIPTION)?;
    
    let status_str = extract_text(tuple, tuple_desc, trajectory::STATUS)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "status is NULL".to_string(),
        }))?;
    let status = str_to_status(&status_str);
    
    let parent_trajectory_id = extract_uuid(tuple, tuple_desc, trajectory::PARENT_TRAJECTORY_ID)?;
    let root_trajectory_id = extract_uuid(tuple, tuple_desc, trajectory::ROOT_TRAJECTORY_ID)?;
    let agent_id = extract_uuid(tuple, tuple_desc, trajectory::AGENT_ID)?;
    
    let created_at_ts = extract_timestamp(tuple, tuple_desc, trajectory::CREATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "created_at is NULL".to_string(),
        }))?;
    let created_at = timestamp_to_chrono(created_at_ts);
    
    let updated_at_ts = extract_timestamp(tuple, tuple_desc, trajectory::UPDATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "updated_at is NULL".to_string(),
        }))?;
    let updated_at = timestamp_to_chrono(updated_at_ts);
    
    let completed_at = extract_timestamp(tuple, tuple_desc, trajectory::COMPLETED_AT)?
        .map(timestamp_to_chrono);
    
    let outcome_json = extract_jsonb(tuple, tuple_desc, trajectory::OUTCOME)?;
    let outcome: Option<TrajectoryOutcome> = outcome_json
        .and_then(|j| serde_json::from_value(j).ok());
    
    let metadata = extract_jsonb(tuple, tuple_desc, trajectory::METADATA)?;
    let tenant_id = extract_uuid(tuple, tuple_desc, trajectory::TENANT_ID)?.map(TenantId::new);

    Ok(TrajectoryRow {
        trajectory: Trajectory {
            trajectory_id,
            name,
            description,
            status,
            parent_trajectory_id,
            root_trajectory_id,
            agent_id,
            created_at,
            updated_at,
            completed_at,
            outcome,
            metadata,
        },
        tenant_id,
    })
}


// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Test Helpers - Generators for Trajectory data
    // ========================================================================

    /// Generate a valid trajectory name (non-empty, reasonable length)
    fn arb_trajectory_name() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9_ -]{0,63}".prop_map(|s| s.trim().to_string())
            .prop_filter("name must not be empty", |s| !s.is_empty())
    }

    /// Generate an optional description
    fn arb_description() -> impl Strategy<Value = Option<String>> {
        prop_oneof![
            Just(None),
            "[a-zA-Z0-9 .,!?-]{0,255}".prop_map(|s| Some(s)),
        ]
    }

    /// Generate an optional agent ID
    fn arb_agent_id() -> impl Strategy<Value = Option<AgentId>> {
        prop_oneof![
            Just(None),
            any::<[u8; 16]>().prop_map(|bytes| Some(AgentId::new(uuid::Uuid::from_bytes(bytes)))),
        ]
    }

    /// Generate a trajectory status
    fn arb_status() -> impl Strategy<Value = TrajectoryStatus> {
        prop_oneof![
            Just(TrajectoryStatus::Active),
            Just(TrajectoryStatus::Completed),
            Just(TrajectoryStatus::Failed),
            Just(TrajectoryStatus::Suspended),
        ]
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Trajectory)
    // Feature: caliber-pg-hot-path, Property 1: Insert-Get Round Trip
    // Validates: Requirements 1.1, 1.2, 1.7
    // ========================================================================

    // NOTE: These property tests require a running PostgreSQL instance with
    // the CALIBER extension installed. They are designed to be run via
    // `cargo pgrx test` which sets up the test database environment.
    //
    // The tests use proptest to generate random trajectory data, insert it
    // via direct heap operations, retrieve it, and verify the round-trip
    // preserves all data.

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use crate::pg_test;

        /// Property 1: Insert-Get Round Trip (Trajectory)
        /// 
        /// *For any* valid trajectory data (name, description, agent_id),
        /// inserting via direct heap then getting via direct heap SHALL
        /// return an equivalent trajectory.
        ///
        /// **Validates: Requirements 1.1, 1.2, 1.7**
        #[pg_test]
        fn prop_trajectory_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_trajectory_name(),
                arb_description(),
                arb_agent_id(),
            );

            runner.run(&strategy, |(name, description, agent_id)| {
                // Generate a new trajectory ID
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();

                // Insert via heap
                let result = trajectory_create_heap(
                    trajectory_id,
                    &name,
                    description.as_deref(),
                    agent_id,
                    tenant_id,
                );
                prop_assert!(result.is_ok(), "Insert should succeed");
                prop_assert_eq!(result.unwrap(), trajectory_id);

                // Get via heap
                let get_result = trajectory_get_heap(trajectory_id, tenant_id);
                prop_assert!(get_result.is_ok(), "Get should succeed");
                
                let trajectory = get_result.unwrap();
                prop_assert!(trajectory.is_some(), "Trajectory should be found");
                
                let row = trajectory.unwrap();
                let t = row.trajectory;
                
                // Verify round-trip preserves data
                prop_assert_eq!(t.trajectory_id, trajectory_id);
                prop_assert_eq!(t.name, name);
                prop_assert_eq!(t.description, description);
                prop_assert_eq!(t.status, TrajectoryStatus::Active); // Default status
                prop_assert_eq!(t.agent_id, agent_id);
                prop_assert!(t.parent_trajectory_id.is_none());
                prop_assert!(t.root_trajectory_id.is_none());
                prop_assert!(t.completed_at.is_none());
                prop_assert!(t.outcome.is_none());
                prop_assert!(t.metadata.is_none());
                prop_assert_eq!(row.tenant_id.map(|t| t.as_uuid()), Some(tenant_id.as_uuid()));

                Ok(())
            }).unwrap();
        }

        /// Property 1 (edge case): Get non-existent trajectory returns None
        ///
        /// *For any* random UUID that was never inserted, getting it SHALL
        /// return Ok(None), not an error.
        ///
        /// **Validates: Requirements 1.7**
        #[pg_test]
        fn prop_trajectory_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let tenant_id = TenantId::now_v7();
                let result = trajectory_get_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Get should not error");
                prop_assert!(result.unwrap().is_none(), "Non-existent trajectory should return None");

                Ok(())
            }).unwrap();
        }
    }

    // ========================================================================
    // Property 2: Update Persistence (Trajectory)
    // Feature: caliber-pg-hot-path, Property 2: Update Persistence
    // Validates: Requirements 1.3, 1.4
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod update_tests {
        use super::*;
        use crate::pg_test;

        /// Property 2: Update Persistence (Trajectory)
        ///
        /// *For any* trajectory and valid status update, applying the update
        /// via direct heap then getting the trajectory SHALL return the
        /// updated status value.
        ///
        /// **Validates: Requirements 1.3, 1.4**
        #[pg_test]
        fn prop_trajectory_update_status_persists() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_trajectory_name(),
                arb_status(),
            );

            runner.run(&strategy, |(name, new_status)| {
                // Create a trajectory first
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let create_result = trajectory_create_heap(
                    trajectory_id,
                    &name,
                    None,
                    None,
                    tenant_id,
                );
                prop_assert!(create_result.is_ok());

                // Update status
                let update_result = trajectory_set_status_heap(trajectory_id, new_status, tenant_id);
                prop_assert!(update_result.is_ok());
                prop_assert!(update_result.unwrap(), "Update should find the trajectory");

                // Get and verify
                let get_result = trajectory_get_heap(trajectory_id, tenant_id);
                prop_assert!(get_result.is_ok());
                
                let trajectory = get_result.unwrap();
                prop_assert!(trajectory.is_some());
                
                let row = trajectory.unwrap();
                let t = row.trajectory;
                prop_assert_eq!(t.status, new_status, "Status should be updated");
                prop_assert_eq!(row.tenant_id.map(|t| t.as_uuid()), Some(tenant_id.as_uuid()));

                // If status is completed or failed, completed_at should be set
                if new_status == TrajectoryStatus::Completed || new_status == TrajectoryStatus::Failed {
                    prop_assert!(t.completed_at.is_some(), "completed_at should be set for terminal status");
                }

                Ok(())
            }).unwrap();
        }

        /// Property 2: Update non-existent trajectory returns false
        ///
        /// *For any* random UUID that was never inserted, updating it SHALL
        /// return Ok(false), not an error.
        ///
        /// **Validates: Requirements 1.3**
        #[pg_test]
        fn prop_trajectory_update_nonexistent_returns_false() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (any::<[u8; 16]>(), arb_status());

            runner.run(&strategy, |(bytes, status)| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let tenant_id = TenantId::now_v7();
                let result = trajectory_set_status_heap(random_id, status, tenant_id);
                prop_assert!(result.is_ok(), "Update should not error");
                prop_assert!(!result.unwrap(), "Update of non-existent trajectory should return false");

                Ok(())
            }).unwrap();
        }

        /// Property 2: Full update preserves all fields
        ///
        /// *For any* trajectory and valid updates to multiple fields,
        /// applying the update then getting SHALL return all updated values.
        ///
        /// **Validates: Requirements 1.3**
        #[pg_test]
        fn prop_trajectory_full_update_persists() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_trajectory_name(),
                arb_trajectory_name(),
                arb_description(),
                arb_status(),
            );

            runner.run(&strategy, |(original_name, new_name, new_desc, new_status)| {
                // Create a trajectory first
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let create_result = trajectory_create_heap(
                    trajectory_id,
                    &original_name,
                    None,
                    None,
                    tenant_id,
                );
                prop_assert!(create_result.is_ok());

                // Update multiple fields
                let update_result = trajectory_update_heap(TrajectoryUpdateHeapParams {
                    id: trajectory_id,
                    tenant_id,
                    name: Some(&new_name),
                    description: Some(new_desc.as_deref()),
                    status: Some(new_status),
                    parent_trajectory_id: None,
                    root_trajectory_id: None,
                    agent_id: None,
                    outcome: None,
                    metadata: None,
                });
                prop_assert!(update_result.is_ok());
                prop_assert!(update_result.unwrap());

                // Get and verify
                let get_result = trajectory_get_heap(trajectory_id, tenant_id);
                prop_assert!(get_result.is_ok());
                
                let row = get_result.unwrap().unwrap();
                let t = row.trajectory;
                prop_assert_eq!(t.name, new_name);
                prop_assert_eq!(t.description, new_desc);
                prop_assert_eq!(t.status, new_status);
                prop_assert_eq!(row.tenant_id.map(|t| t.as_uuid()), Some(tenant_id.as_uuid()));

                Ok(())
            }).unwrap();
        }
    }
}
