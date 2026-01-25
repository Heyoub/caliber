//! Direct heap operations for Scope entities.
//!
//! This module provides hot-path operations for scopes that bypass SQL
//! parsing entirely by using direct heap access via pgrx.
//!
//! # Operations
//!
//! - `scope_create_heap` - Insert a new scope
//! - `scope_get_heap` - Get a scope by ID
//! - `scope_close_heap` - Close a scope (set is_active=false)
//! - `scope_list_by_trajectory_heap` - List scopes by trajectory
//! - `scope_update_tokens_heap` - Update tokens_used field

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    CaliberError, CaliberResult, Checkpoint, EntityIdType, EntityType,
    Scope, ScopeId, StorageError, TenantId, TrajectoryId,
};

use crate::column_maps::scope;
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
    extract_bool, extract_i32, extract_values_and_nulls, 
    uuid_to_datum, string_to_datum, bool_to_datum, i32_to_datum,
    json_to_datum, timestamp_to_chrono,
};

/// Scope row with tenant ownership metadata.
pub struct ScopeRow {
    pub scope: Scope,
    pub tenant_id: Option<TenantId>,
}

impl From<ScopeRow> for Scope {
    fn from(row: ScopeRow) -> Self {
        row.scope
    }
}

/// Create a new scope using direct heap operations.
///
/// This bypasses SQL parsing entirely for hot-path performance.
///
/// # Arguments
/// * `scope_id` - The pre-generated UUIDv7 for this scope
/// * `trajectory_id` - The parent trajectory ID
/// * `name` - The scope name (required)
/// * `purpose` - Optional purpose description
/// * `token_budget` - Token budget for this scope
///
/// # Returns
/// * `Ok(ScopeId)` - The scope ID on success
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 2.1: Uses heap_form_tuple and simple_heap_insert instead of SPI
/// - 2.6: Updates all relevant indexes via CatalogIndexInsert
pub fn scope_create_heap(
    scope_id: ScopeId,
    trajectory_id: TrajectoryId,
    name: &str,
    purpose: Option<&str>,
    token_budget: i32,
    tenant_id: TenantId,
) -> CaliberResult<ScopeId> {
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(scope::TABLE_NAME, LockMode::RowExclusive)?;

    // Validate relation schema matches expectations
    validate_scope_relation(&rel)?;

    // Get current transaction timestamp for created_at
    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now)?.into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Scope,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    // Build datum array - must match column order in caliber_scope table
    let mut values: [pg_sys::Datum; scope::NUM_COLS] = [pg_sys::Datum::from(0); scope::NUM_COLS];
    let mut nulls: [bool; scope::NUM_COLS] = [false; scope::NUM_COLS];
    
    // Column 1: scope_id (UUID, NOT NULL)
    values[scope::SCOPE_ID as usize - 1] = uuid_to_datum(scope_id.as_uuid());

    // Column 2: trajectory_id (UUID, NOT NULL)
    values[scope::TRAJECTORY_ID as usize - 1] = uuid_to_datum(trajectory_id.as_uuid());
    
    // Column 3: parent_scope_id (UUID, nullable)
    nulls[scope::PARENT_SCOPE_ID as usize - 1] = true;
    
    // Column 4: name (TEXT, NOT NULL)
    values[scope::NAME as usize - 1] = string_to_datum(name);
    
    // Column 5: purpose (TEXT, nullable)
    if let Some(p) = purpose {
        values[scope::PURPOSE as usize - 1] = string_to_datum(p);
    } else {
        nulls[scope::PURPOSE as usize - 1] = true;
    }
    
    // Column 6: is_active (BOOLEAN, NOT NULL) - default to true
    values[scope::IS_ACTIVE as usize - 1] = bool_to_datum(true);
    
    // Column 7: created_at (TIMESTAMPTZ, NOT NULL)
    values[scope::CREATED_AT as usize - 1] = now_datum;
    
    // Column 8: closed_at (TIMESTAMPTZ, nullable)
    nulls[scope::CLOSED_AT as usize - 1] = true;
    
    // Column 9: checkpoint (JSONB, nullable)
    nulls[scope::CHECKPOINT as usize - 1] = true;
    
    // Column 10: token_budget (INTEGER, NOT NULL)
    values[scope::TOKEN_BUDGET as usize - 1] = i32_to_datum(token_budget);
    
    // Column 11: tokens_used (INTEGER, NOT NULL) - default to 0
    values[scope::TOKENS_USED as usize - 1] = i32_to_datum(0);
    
    // Column 12: metadata (JSONB, nullable)
    nulls[scope::METADATA as usize - 1] = true;

    // Column 13: tenant_id (UUID, NOT NULL)
    values[scope::TENANT_ID as usize - 1] = uuid_to_datum(tenant_id.as_uuid());
    
    // Form the heap tuple
    let tuple = form_tuple(&rel, &values, &nulls)?;
    
    // Insert into heap
    let _tid = unsafe { insert_tuple(&rel, tuple)? };
    
    // Update all indexes via CatalogIndexInsert
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };
    
    Ok(scope_id)
}


/// Get a scope by ID using direct heap operations.
///
/// This bypasses SQL parsing entirely for hot-path performance.
///
/// # Arguments
/// * `id` - The scope ID to look up
///
/// # Returns
/// * `Ok(Some(Scope))` - The scope if found
/// * `Ok(None)` - If no scope with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 2.2: Uses index_beginscan for O(log n) lookup instead of SPI SELECT
pub fn scope_get_heap(id: ScopeId, tenant_id: TenantId) -> CaliberResult<Option<ScopeRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(scope::TABLE_NAME, LockMode::AccessShare)?;
    
    // Open the primary key index
    let index_rel = open_index(scope::PK_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for primary key lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (scope_id)
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
        let row = unsafe { tuple_to_scope(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid()) {
            Ok(Some(row))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Close a scope using direct heap operations.
///
/// Sets is_active to false and closed_at to current timestamp.
///
/// # Arguments
/// * `id` - The scope ID to close
///
/// # Returns
/// * `Ok(true)` - If the scope was found and closed
/// * `Ok(false)` - If no scope with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 2.3: Uses simple_heap_update instead of SPI UPDATE
pub fn scope_close_heap(id: ScopeId, tenant_id: TenantId) -> CaliberResult<bool> {
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(scope::TABLE_NAME, LockMode::RowExclusive)?;

    // Open the primary key index
    let index_rel = open_index(scope::PK_INDEX)?;

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
            entity_type: EntityType::Scope,
            id: id.as_uuid(),
            reason: "Failed to get TID of existing tuple".to_string(),
        }))?;

    let tuple_desc = rel.tuple_desc();
    let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, scope::TENANT_ID)? };
    if existing_tenant != Some(tenant_id.as_uuid()) {
        return Ok(false);
    }

    // Extract current values and nulls
    let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;

    // Update is_active to false
    values[scope::IS_ACTIVE as usize - 1] = bool_to_datum(false);

    // Set closed_at to current timestamp
    let now = current_timestamp();
    values[scope::CLOSED_AT as usize - 1] = timestamp_to_pgrx(now)?
        .into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
            entity_type: EntityType::Scope,
            id: id.as_uuid(),
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    nulls[scope::CLOSED_AT as usize - 1] = false;
    
    // Form new tuple
    let new_tuple = form_tuple(&rel, &values, &nulls)?;
    
    // Update in place
    unsafe { update_tuple(&rel, &tid, new_tuple)? };
    
    // Update indexes
    unsafe { update_indexes_for_insert(&rel, new_tuple, &values, &nulls)? };
    
    Ok(true)
}


/// List scopes by trajectory ID using direct heap operations.
///
/// # Arguments
/// * `trajectory_id` - The trajectory ID to filter by
///
/// # Returns
/// * `Ok(Vec<Scope>)` - List of matching scopes
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 2.4: Uses index scan on trajectory_id instead of SPI SELECT
pub fn scope_list_by_trajectory_heap(
    trajectory_id: TrajectoryId,
    tenant_id: TenantId,
) -> CaliberResult<Vec<ScopeRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(scope::TABLE_NAME, LockMode::AccessShare)?;
    
    // Open the trajectory index
    let index_rel = open_index(scope::TRAJECTORY_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for trajectory_id lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (trajectory_id)
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(trajectory_id.as_uuid()),
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
        let row = unsafe { tuple_to_scope(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid()) {
            results.push(row);
        }
    }
    
    Ok(results)
}

/// Update tokens_used for a scope using direct heap operations.
///
/// # Arguments
/// * `id` - The scope ID to update
/// * `tokens_used` - The new tokens_used value
///
/// # Returns
/// * `Ok(true)` - If the scope was found and updated
/// * `Ok(false)` - If no scope with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 2.5: Uses simple_heap_update instead of SPI UPDATE
pub fn scope_update_tokens_heap(
    id: ScopeId,
    tokens_used: i32,
    tenant_id: TenantId,
) -> CaliberResult<bool> {
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(scope::TABLE_NAME, LockMode::RowExclusive)?;
    
    // Open the primary key index
    let index_rel = open_index(scope::PK_INDEX)?;
    
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
            entity_type: EntityType::Scope,
            id: id.as_uuid(),
            reason: "Failed to get TID of existing tuple".to_string(),
        }))?;

    let tuple_desc = rel.tuple_desc();
    let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, scope::TENANT_ID)? };
    if existing_tenant != Some(tenant_id.as_uuid()) {
        return Ok(false);
    }

    // Extract current values and nulls
    let (mut values, nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;

    // Update tokens_used
    values[scope::TOKENS_USED as usize - 1] = i32_to_datum(tokens_used);
    
    // Form new tuple
    let new_tuple = form_tuple(&rel, &values, &nulls)?;
    
    // Update in place
    unsafe { update_tuple(&rel, &tid, new_tuple)? };
    
    // Update indexes (tokens_used is not indexed, but call anyway for consistency)
    unsafe { update_indexes_for_insert(&rel, new_tuple, &values, &nulls)? };
    
    Ok(true)
}


// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Validate that a HeapRelation is suitable for scope operations.
/// This ensures the relation schema matches what we expect before operations.
fn validate_scope_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != scope::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Scope relation has {} columns, expected {}",
                natts,
                scope::NUM_COLS
            ),
        }));
    }
    Ok(())
}

/// Update the checkpoint for a scope using direct heap operations.
/// This uses json_to_datum for proper JSONB serialization.
pub fn scope_update_checkpoint_heap(
    id: ScopeId,
    checkpoint: Option<&Checkpoint>,
    tenant_id: TenantId,
) -> CaliberResult<bool> {
    let rel = open_relation(scope::TABLE_NAME, LockMode::RowExclusive)?;
    validate_scope_relation(&rel)?;

    let index_rel = open_index(scope::PK_INDEX)?;
    let snapshot = get_active_snapshot();

    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(id.as_uuid()),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    let old_tuple = match scanner.next() {
        Some(t) => t,
        None => return Ok(false),
    };

    let tid = scanner.current_tid()
        .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
            entity_type: EntityType::Scope,
            id: id.as_uuid(),
            reason: "Failed to get TID".to_string(),
        }))?;

    let tuple_desc = rel.tuple_desc();
    let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, scope::TENANT_ID)? };
    if existing_tenant != Some(tenant_id.as_uuid()) {
        return Ok(false);
    }
    let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;

    // Update checkpoint using json_to_datum
    if let Some(cp) = checkpoint {
        let checkpoint_json = serde_json::to_value(cp)
            .map_err(|e| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Scope,
                id: id.as_uuid(),
                reason: format!("Failed to serialize checkpoint: {}", e),
            }))?;
        values[scope::CHECKPOINT as usize - 1] = json_to_datum(&checkpoint_json);
        nulls[scope::CHECKPOINT as usize - 1] = false;
    } else {
        nulls[scope::CHECKPOINT as usize - 1] = true;
    }

    let new_tuple = form_tuple(&rel, &values, &nulls)?;
    unsafe { update_tuple(&rel, &tid, new_tuple)? };
    unsafe { update_indexes_for_insert(&rel, new_tuple, &values, &nulls)? };
    unsafe { update_tuple(&rel, &tid, new_tuple)? };
    unsafe { update_indexes_for_insert(&rel, new_tuple, &values, &nulls)? };
    unsafe { update_tuple(&rel, &tid, new_tuple)? };
    unsafe { update_indexes_for_insert(&rel, new_tuple, &values, &nulls)? };

    Ok(true)
}

/// Convert a heap tuple to a Scope struct.
unsafe fn tuple_to_scope(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<ScopeRow> {
    // Extract all fields from the tuple
    let scope_id = extract_uuid(tuple, tuple_desc, scope::SCOPE_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "scope_id is NULL".to_string(),
        }))?;
    let scope_id = ScopeId::new(scope_id);
    
    let trajectory_id = extract_uuid(tuple, tuple_desc, scope::TRAJECTORY_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "trajectory_id is NULL".to_string(),
        }))?;
    let trajectory_id = TrajectoryId::new(trajectory_id);
    
    let parent_scope_id = extract_uuid(tuple, tuple_desc, scope::PARENT_SCOPE_ID)?.map(ScopeId::new);
    
    let name = extract_text(tuple, tuple_desc, scope::NAME)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "name is NULL".to_string(),
        }))?;
    
    let purpose = extract_text(tuple, tuple_desc, scope::PURPOSE)?;
    
    let is_active = extract_bool(tuple, tuple_desc, scope::IS_ACTIVE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "is_active is NULL".to_string(),
        }))?;
    
    let created_at_ts = extract_timestamp(tuple, tuple_desc, scope::CREATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "created_at is NULL".to_string(),
        }))?;
    let created_at = timestamp_to_chrono(created_at_ts);
    
    let closed_at = extract_timestamp(tuple, tuple_desc, scope::CLOSED_AT)?
        .map(timestamp_to_chrono);
    
    let checkpoint_json = extract_jsonb(tuple, tuple_desc, scope::CHECKPOINT)?;
    let checkpoint: Option<Checkpoint> = checkpoint_json
        .and_then(|j| serde_json::from_value(j).ok());
    
    let token_budget = extract_i32(tuple, tuple_desc, scope::TOKEN_BUDGET)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "token_budget is NULL".to_string(),
        }))?;
    
    let tokens_used = extract_i32(tuple, tuple_desc, scope::TOKENS_USED)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "tokens_used is NULL".to_string(),
        }))?;
    
    let metadata = extract_jsonb(tuple, tuple_desc, scope::METADATA)?;
    let tenant_id = extract_uuid(tuple, tuple_desc, scope::TENANT_ID)?.map(TenantId::new);

    Ok(ScopeRow {
        scope: Scope {
            scope_id,
            trajectory_id,
            parent_scope_id,
            name,
            purpose,
            is_active,
            created_at,
            closed_at,
            checkpoint,
            token_budget,
            tokens_used,
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
    use proptest::prelude::*;

    // ========================================================================
    // Test Helpers - Generators for Scope data
    // ========================================================================

    /// Generate a valid scope name (non-empty, reasonable length)
    fn arb_scope_name() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9_ -]{0,63}".prop_map(|s| s.trim().to_string())
            .prop_filter("name must not be empty", |s| !s.is_empty())
    }

    /// Generate an optional purpose
    fn arb_purpose() -> impl Strategy<Value = Option<String>> {
        prop_oneof![
            Just(None),
            "[a-zA-Z0-9 .,!?-]{0,255}".prop_map(|s| Some(s)),
        ]
    }

    /// Generate a valid token budget (positive)
    fn arb_token_budget() -> impl Strategy<Value = i32> {
        1000i32..100000i32
    }

    /// Generate a valid tokens_used value
    fn arb_tokens_used() -> impl Strategy<Value = i32> {
        0i32..50000i32
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Scope)
    // Feature: caliber-pg-hot-path, Property 1: Insert-Get Round Trip
    // Validates: Requirements 2.1, 2.2
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use crate::pg_test;
        use crate::scope_heap::{scope_create_heap, scope_get_heap};

        /// Property 1: Insert-Get Round Trip (Scope)
        /// 
        /// *For any* valid scope data (trajectory_id, name, purpose, token_budget),
        /// inserting via direct heap then getting via direct heap SHALL
        /// return an equivalent scope.
        ///
        /// **Validates: Requirements 2.1, 2.2**
        #[pg_test]
        fn prop_scope_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_scope_name(),
                arb_purpose(),
                arb_token_budget(),
            );

            runner.run(&strategy, |(name, purpose, token_budget)| {
                // First create a trajectory to be the parent
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let traj_result = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                    tenant_id,
                );
                prop_assert!(traj_result.is_ok(), "Trajectory creation should succeed");

                // Generate a new scope ID
                let scope_id = ScopeId::now_v7();

                // Insert via heap
                let result = scope_create_heap(
                    scope_id,
                    trajectory_id,
                    &name,
                    purpose.as_deref(),
                    token_budget,
                    tenant_id,
                );
                prop_assert!(result.is_ok(), "Insert should succeed");
                prop_assert_eq!(result.unwrap(), scope_id);

                // Get via heap
                let get_result = scope_get_heap(scope_id, tenant_id);
                prop_assert!(get_result.is_ok(), "Get should succeed");
                
                let scope = get_result.unwrap();
                prop_assert!(scope.is_some(), "Scope should be found");
                
                let row = scope.unwrap();
                let s = row.scope;
                
                // Verify round-trip preserves data
                prop_assert_eq!(s.scope_id, scope_id);
                prop_assert_eq!(s.trajectory_id, trajectory_id);
                prop_assert_eq!(s.name, name);
                prop_assert_eq!(s.purpose, purpose);
                prop_assert_eq!(s.token_budget, token_budget);
                prop_assert_eq!(s.tokens_used, 0); // Default
                prop_assert!(s.is_active); // Default true
                prop_assert!(s.parent_scope_id.is_none());
                prop_assert!(s.closed_at.is_none());
                prop_assert!(s.checkpoint.is_none());
                prop_assert!(s.metadata.is_none());
                prop_assert_eq!(row.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Property 1 (edge case): Get non-existent scope returns None
        ///
        /// *For any* random UUID that was never inserted, getting it SHALL
        /// return Ok(None), not an error.
        #[pg_test]
        fn prop_scope_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = ScopeId::new(uuid::Uuid::from_bytes(bytes));

                let tenant_id = TenantId::now_v7();
                let result = scope_get_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Get should not error");
                prop_assert!(result.unwrap().is_none(), "Non-existent scope should return None");

                Ok(())
            }).unwrap();
        }
    }


    // ========================================================================
    // Property 2: Close and Update Persistence (Scope)
    // Feature: caliber-pg-hot-path, Property 2: Update Persistence
    // Validates: Requirements 2.3, 2.5
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod update_tests {
        use super::*;
        use crate::pg_test;
        use crate::scope_heap::{
            scope_close_heap, scope_create_heap, scope_get_heap, scope_update_tokens_heap,
        };

        /// Property 2: Close scope persists is_active=false and closed_at
        ///
        /// *For any* scope, closing it via direct heap then getting it SHALL
        /// return is_active=false and a non-null closed_at.
        ///
        /// **Validates: Requirements 2.3**
        #[pg_test]
        fn prop_scope_close_persists() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_scope_name(),
                arb_token_budget(),
            );

            runner.run(&strategy, |(name, token_budget)| {
                // Create trajectory first
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                    tenant_id,
                );

                // Create scope
                let scope_id = ScopeId::now_v7();
                let _ = scope_create_heap(
                    scope_id,
                    trajectory_id,
                    &name,
                    None,
                    token_budget,
                    tenant_id,
                );

                // Verify initially active
                let before = scope_get_heap(scope_id, tenant_id).unwrap().unwrap();
                prop_assert!(before.scope.is_active, "Scope should be active initially");
                prop_assert!(before.scope.closed_at.is_none(), "closed_at should be None initially");
                prop_assert_eq!(before.tenant_id, Some(tenant_id));

                // Close the scope
                let close_result = scope_close_heap(scope_id, tenant_id);
                prop_assert!(close_result.is_ok());
                prop_assert!(close_result.unwrap(), "Close should find the scope");

                // Verify closed
                let after = scope_get_heap(scope_id, tenant_id).unwrap().unwrap();
                prop_assert!(!after.scope.is_active, "Scope should be inactive after close");
                prop_assert!(after.scope.closed_at.is_some(), "closed_at should be set after close");
                prop_assert_eq!(after.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Property 2: Update tokens_used persists
        ///
        /// *For any* scope and valid tokens_used value, updating via direct heap
        /// then getting SHALL return the updated tokens_used.
        ///
        /// **Validates: Requirements 2.5**
        #[pg_test]
        fn prop_scope_update_tokens_persists() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_scope_name(),
                arb_token_budget(),
                arb_tokens_used(),
            );

            runner.run(&strategy, |(name, token_budget, new_tokens_used)| {
                // Create trajectory first
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                    tenant_id,
                );

                // Create scope
                let scope_id = ScopeId::now_v7();
                let _ = scope_create_heap(
                    scope_id,
                    trajectory_id,
                    &name,
                    None,
                    token_budget,
                    tenant_id,
                );

                // Update tokens_used
                let update_result = scope_update_tokens_heap(scope_id, new_tokens_used, tenant_id);
                prop_assert!(update_result.is_ok());
                prop_assert!(update_result.unwrap(), "Update should find the scope");

                // Verify updated
                let after = scope_get_heap(scope_id, tenant_id).unwrap().unwrap();
                prop_assert_eq!(after.scope.tokens_used, new_tokens_used, "tokens_used should be updated");
                prop_assert_eq!(after.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Property 2: Close non-existent scope returns false
        #[pg_test]
        fn prop_scope_close_nonexistent_returns_false() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = ScopeId::new(uuid::Uuid::from_bytes(bytes));

                let tenant_id = TenantId::now_v7();
                let result = scope_close_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Close should not error");
                prop_assert!(!result.unwrap(), "Close of non-existent scope should return false");

                Ok(())
            }).unwrap();
        }

        /// Property 2: Update tokens on non-existent scope returns false
        #[pg_test]
        fn prop_scope_update_tokens_nonexistent_returns_false() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (any::<[u8; 16]>(), arb_tokens_used());

            runner.run(&strategy, |(bytes, tokens)| {
                let random_id = ScopeId::new(uuid::Uuid::from_bytes(bytes));

                let tenant_id = TenantId::now_v7();
                let result = scope_update_tokens_heap(random_id, tokens, tenant_id);
                prop_assert!(result.is_ok(), "Update should not error");
                prop_assert!(!result.unwrap(), "Update of non-existent scope should return false");

                Ok(())
            }).unwrap();
        }
    }

    // ========================================================================
    // Property 3: List by Trajectory (Scope)
    // Feature: caliber-pg-hot-path, Property 3: Index Consistency
    // Validates: Requirements 2.4
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod list_tests {
        use super::*;
        use crate::pg_test;
        use crate::scope_heap::{scope_create_heap, scope_list_by_trajectory_heap};

        /// Property 3: List by trajectory returns all scopes for that trajectory
        ///
        /// *For any* trajectory with N scopes, listing by trajectory_id SHALL
        /// return exactly N scopes, all with matching trajectory_id.
        ///
        /// **Validates: Requirements 2.4**
        #[pg_test]
        fn prop_scope_list_by_trajectory() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(50);
            let mut runner = TestRunner::new(config);

            let strategy = (1usize..5usize, arb_token_budget());

            runner.run(&strategy, |(num_scopes, token_budget)| {
                // Create trajectory
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                    tenant_id,
                );

                // Create multiple scopes
                let mut scope_ids = Vec::new();
                for i in 0..num_scopes {
                    let scope_id = ScopeId::now_v7();
                    let _ = scope_create_heap(
                        scope_id,
                        trajectory_id,
                        &format!("scope_{}", i),
                        None,
                        token_budget,
                        tenant_id,
                    );
                    scope_ids.push(scope_id);
                }

                // List by trajectory
                let list_result = scope_list_by_trajectory_heap(trajectory_id, tenant_id);
                prop_assert!(list_result.is_ok(), "List should succeed");
                
                let scopes = list_result.unwrap();
                prop_assert_eq!(scopes.len(), num_scopes, "Should return all scopes");

                // Verify all scopes have correct trajectory_id
                for row in &scopes {
                    prop_assert_eq!(row.scope.trajectory_id, trajectory_id);
                    prop_assert_eq!(row.tenant_id, Some(tenant_id));
                }

                // Verify all created scope_ids are in the result
                for scope_id in &scope_ids {
                    prop_assert!(
                        scopes.iter().any(|s| s.scope.scope_id == *scope_id),
                        "All created scopes should be in result"
                    );
                }

                Ok(())
            }).unwrap();
        }

        /// Property 3: List by non-existent trajectory returns empty
        #[pg_test]
        fn prop_scope_list_by_nonexistent_trajectory_returns_empty() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = TrajectoryId::new(uuid::Uuid::from_bytes(bytes));

                let tenant_id = TenantId::now_v7();
                let result = scope_list_by_trajectory_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "List should not error");
                prop_assert!(result.unwrap().is_empty(), "List for non-existent trajectory should be empty");

                Ok(())
            }).unwrap();
        }
    }
}
