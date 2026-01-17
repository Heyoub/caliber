//! Direct heap operations for Conflict entities.
//!
//! This module provides hot-path operations for conflict detection and resolution
//! that bypass SQL parsing entirely by using direct heap access via pgrx.

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    CaliberError, CaliberResult, EntityId, EntityType, StorageError,
};
use caliber_agents::{Conflict, ConflictStatus, ConflictType, ConflictResolutionRecord};

use crate::column_maps::conflict;
use crate::heap_ops::{
    current_timestamp, form_tuple, insert_tuple, open_relation, update_tuple,
    LockMode as HeapLockMode, HeapRelation, get_active_snapshot,
    timestamp_to_pgrx,
};
use crate::index_ops::{
    init_scan_key, open_index, update_indexes_for_insert,
    BTreeStrategy, IndexScanner, operator_oids,
};
use crate::tuple_extract::{
    extract_uuid, extract_text, extract_timestamp, extract_jsonb,
    extract_values_and_nulls, uuid_to_datum, string_to_datum,
    timestamp_to_chrono, json_to_datum, option_uuid_to_datum,
};

/// Create a new conflict by inserting a conflict record using direct heap operations.
pub struct ConflictCreateParams<'a> {
    pub conflict_id: EntityId,
    pub conflict_type: ConflictType,
    pub item_a_type: &'a str,
    pub item_a_id: EntityId,
    pub item_b_type: &'a str,
    pub item_b_id: EntityId,
    pub agent_a_id: Option<EntityId>,
    pub agent_b_id: Option<EntityId>,
    pub trajectory_id: Option<EntityId>,
}

pub fn conflict_create_heap(params: ConflictCreateParams<'_>) -> CaliberResult<EntityId> {
    let ConflictCreateParams {
        conflict_id,
        conflict_type,
        item_a_type,
        item_a_id,
        item_b_type,
        item_b_id,
        agent_a_id,
        agent_b_id,
        trajectory_id,
    } = params;
    let rel = open_relation(conflict::TABLE_NAME, HeapLockMode::RowExclusive)?;
    validate_conflict_relation(&rel)?;

    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now).into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Conflict,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    let mut values: [pg_sys::Datum; conflict::NUM_COLS] = [pg_sys::Datum::from(0); conflict::NUM_COLS];
    let mut nulls: [bool; conflict::NUM_COLS] = [false; conflict::NUM_COLS];
    
    // Set required fields
    values[conflict::CONFLICT_ID as usize - 1] = uuid_to_datum(conflict_id);
    
    // Set conflict_type
    let conflict_type_str = match conflict_type {
        ConflictType::ConcurrentWrite => "concurrent_write",
        ConflictType::ContradictingFact => "contradicting_fact",
        ConflictType::IncompatibleDecision => "incompatible_decision",
        ConflictType::ResourceContention => "resource_contention",
        ConflictType::GoalConflict => "goal_conflict",
    };
    values[conflict::CONFLICT_TYPE as usize - 1] = string_to_datum(conflict_type_str);
    
    // Set item A
    values[conflict::ITEM_A_TYPE as usize - 1] = string_to_datum(item_a_type);
    values[conflict::ITEM_A_ID as usize - 1] = uuid_to_datum(item_a_id);

    // Set item B
    values[conflict::ITEM_B_TYPE as usize - 1] = string_to_datum(item_b_type);
    values[conflict::ITEM_B_ID as usize - 1] = uuid_to_datum(item_b_id);

    // Use helper for optional agent and trajectory fields
    let ((agent_a_datum, agent_a_null), (agent_b_datum, agent_b_null), (traj_datum, traj_null)) =
        build_optional_conflict_agents(agent_a_id, agent_b_id, trajectory_id);

    // Set optional agent_a_id
    values[conflict::AGENT_A_ID as usize - 1] = agent_a_datum;
    nulls[conflict::AGENT_A_ID as usize - 1] = agent_a_null;

    // Set optional agent_b_id
    values[conflict::AGENT_B_ID as usize - 1] = agent_b_datum;
    nulls[conflict::AGENT_B_ID as usize - 1] = agent_b_null;

    // Set optional trajectory_id
    values[conflict::TRAJECTORY_ID as usize - 1] = traj_datum;
    nulls[conflict::TRAJECTORY_ID as usize - 1] = traj_null;
    
    // Set status - default to "detected"
    values[conflict::STATUS as usize - 1] = string_to_datum("detected");
    
    // Set resolution to NULL initially
    nulls[conflict::RESOLUTION as usize - 1] = true;
    
    // Set timestamps
    values[conflict::DETECTED_AT as usize - 1] = now_datum;
    nulls[conflict::RESOLVED_AT as usize - 1] = true;
    
    let tuple = form_tuple(&rel, &values, &nulls)?;
    let _tid = unsafe { insert_tuple(&rel, tuple)? };
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };
    
    Ok(conflict_id)
}

/// Get a conflict by ID using direct heap operations.
pub fn conflict_get_heap(conflict_id: EntityId) -> CaliberResult<Option<Conflict>> {
    let rel = open_relation(conflict::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(conflict::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(conflict_id),
    );
    
    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };
    
    if let Some(tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let conflict = unsafe { tuple_to_conflict(tuple, tuple_desc) }?;
        Ok(Some(conflict))
    } else {
        Ok(None)
    }
}

/// Resolve a conflict by updating status, resolution, and resolved_at using direct heap operations.
pub fn conflict_resolve_heap(
    conflict_id: EntityId,
    resolution: &ConflictResolutionRecord,
) -> CaliberResult<bool> {
    let rel = open_relation(conflict::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(conflict::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(conflict_id),
    );
    
    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };
    
    if let Some(old_tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;
        
        // Update status to "resolved"
        values[conflict::STATUS as usize - 1] = string_to_datum("resolved");
        
        // Serialize resolution to JSON
        let resolution_json = serde_json::to_value(resolution)
            .map_err(|e| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Conflict,
                id: conflict_id,
                reason: format!("Failed to serialize resolution: {}", e),
            }))?;
        
        values[conflict::RESOLUTION as usize - 1] = json_to_datum(&resolution_json);
        nulls[conflict::RESOLUTION as usize - 1] = false;
        
        // Update resolved_at to current timestamp
        let now = current_timestamp();
        let now_datum = timestamp_to_pgrx(now).into_datum()
            .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Conflict,
                id: conflict_id,
                reason: "Failed to convert timestamp to datum".to_string(),
            }))?;
        
        values[conflict::RESOLVED_AT as usize - 1] = now_datum;
        nulls[conflict::RESOLVED_AT as usize - 1] = false;
        
        let new_tuple = form_tuple(&rel, &values, &nulls)?;
        let old_tid = scanner.current_tid()
            .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of conflict tuple".to_string(),
            }))?;
        
        unsafe { update_tuple(&rel, &old_tid, new_tuple)? };
        Ok(true)
    } else {
        Ok(false)
    }
}

/// List pending conflicts (detected or resolving) using direct heap operations.
pub fn conflict_list_pending_heap() -> CaliberResult<Vec<Conflict>> {
    let rel = open_relation(conflict::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(conflict::STATUS_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();
    
    // Scan for "detected" status
    let mut scan_key_detected = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key_detected,
        1,
        BTreeStrategy::Equal,
        operator_oids::TEXT_EQ,
        string_to_datum("detected"),
    );
    
    let mut scanner_detected = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key_detected) };
    
    for tuple in &mut scanner_detected {
        let conflict = unsafe { tuple_to_conflict(tuple, tuple_desc) }?;
        results.push(conflict);
    }
    
    // Scan for "resolving" status
    let mut scan_key_resolving = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key_resolving,
        1,
        BTreeStrategy::Equal,
        operator_oids::TEXT_EQ,
        string_to_datum("resolving"),
    );
    
    let mut scanner_resolving = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key_resolving) };
    
    for tuple in &mut scanner_resolving {
        let conflict = unsafe { tuple_to_conflict(tuple, tuple_desc) }?;
        results.push(conflict);
    }
    
    Ok(results)
}

/// Validate that a HeapRelation is suitable for conflict operations.
fn validate_conflict_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != conflict::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Conflict relation has {} columns, expected {}",
                natts,
                conflict::NUM_COLS
            ),
        }));
    }
    Ok(())
}

/// Build optional conflict datums using proper helper.
fn build_optional_conflict_agents(
    agent_a_id: Option<EntityId>,
    agent_b_id: Option<EntityId>,
    trajectory_id: Option<EntityId>,
) -> ((pg_sys::Datum, bool), (pg_sys::Datum, bool), (pg_sys::Datum, bool)) {
    let a = match agent_a_id {
        Some(id) => (option_uuid_to_datum(Some(id)), false),
        None => (pg_sys::Datum::from(0), true),
    };
    let b = match agent_b_id {
        Some(id) => (option_uuid_to_datum(Some(id)), false),
        None => (pg_sys::Datum::from(0), true),
    };
    let t = match trajectory_id {
        Some(id) => (option_uuid_to_datum(Some(id)), false),
        None => (pg_sys::Datum::from(0), true),
    };
    (a, b, t)
}

unsafe fn tuple_to_conflict(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<Conflict> {
    let conflict_id = extract_uuid(tuple, tuple_desc, conflict::CONFLICT_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "conflict_id is NULL".to_string(),
        }))?;
    
    let conflict_type_str = extract_text(tuple, tuple_desc, conflict::CONFLICT_TYPE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "conflict_type is NULL".to_string(),
        }))?;
    let conflict_type = match conflict_type_str.as_str() {
        "concurrent_write" => ConflictType::ConcurrentWrite,
        "contradicting_fact" => ConflictType::ContradictingFact,
        "incompatible_decision" => ConflictType::IncompatibleDecision,
        "resource_contention" => ConflictType::ResourceContention,
        "goal_conflict" => ConflictType::GoalConflict,
        _ => {
            pgrx::warning!("CALIBER: Unknown conflict type '{}', defaulting to ConcurrentWrite", conflict_type_str);
            ConflictType::ConcurrentWrite
        }
    };
    
    let item_a_type = extract_text(tuple, tuple_desc, conflict::ITEM_A_TYPE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "item_a_type is NULL".to_string(),
        }))?;
    
    let item_a_id = extract_uuid(tuple, tuple_desc, conflict::ITEM_A_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "item_a_id is NULL".to_string(),
        }))?;
    
    let item_b_type = extract_text(tuple, tuple_desc, conflict::ITEM_B_TYPE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "item_b_type is NULL".to_string(),
        }))?;
    
    let item_b_id = extract_uuid(tuple, tuple_desc, conflict::ITEM_B_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "item_b_id is NULL".to_string(),
        }))?;
    
    let agent_a_id = extract_uuid(tuple, tuple_desc, conflict::AGENT_A_ID)?;
    let agent_b_id = extract_uuid(tuple, tuple_desc, conflict::AGENT_B_ID)?;
    let trajectory_id = extract_uuid(tuple, tuple_desc, conflict::TRAJECTORY_ID)?;
    
    let status_str = extract_text(tuple, tuple_desc, conflict::STATUS)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "status is NULL".to_string(),
        }))?;
    let status = match status_str.as_str() {
        "detected" => ConflictStatus::Detected,
        "resolving" => ConflictStatus::Resolving,
        "resolved" => ConflictStatus::Resolved,
        "escalated" => ConflictStatus::Escalated,
        _ => {
            pgrx::warning!("CALIBER: Unknown conflict status '{}', defaulting to Detected", status_str);
            ConflictStatus::Detected
        }
    };
    
    let resolution = extract_jsonb(tuple, tuple_desc, conflict::RESOLUTION)?
        .and_then(|json| serde_json::from_value::<ConflictResolutionRecord>(json).ok());
    
    let detected_at_ts = extract_timestamp(tuple, tuple_desc, conflict::DETECTED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "detected_at is NULL".to_string(),
        }))?;
    let detected_at = timestamp_to_chrono(detected_at_ts);
    
    let resolved_at = extract_timestamp(tuple, tuple_desc, conflict::RESOLVED_AT)?
        .map(timestamp_to_chrono);
    
    Ok(Conflict {
        conflict_id,
        conflict_type,
        item_a_type,
        item_a_id,
        item_b_type,
        item_b_id,
        agent_a_id,
        agent_b_id,
        trajectory_id,
        status,
        resolution,
        detected_at,
        resolved_at,
    })
}


// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use caliber_agents::ResolutionStrategy;

    // ========================================================================
    // Test Helpers - Generators for Conflict data
    // ========================================================================

    /// Generate a random EntityId
    fn arb_entity_id() -> impl Strategy<Value = EntityId> {
        any::<[u8; 16]>().prop_map(|bytes| uuid::Uuid::from_bytes(bytes))
    }

    /// Generate an optional EntityId
    fn arb_optional_entity_id() -> impl Strategy<Value = Option<EntityId>> {
        prop_oneof![
            1 => Just(None),
            3 => arb_entity_id().prop_map(Some),
        ]
    }

    /// Generate a conflict type
    fn arb_conflict_type() -> impl Strategy<Value = ConflictType> {
        prop_oneof![
            Just(ConflictType::ConcurrentWrite),
            Just(ConflictType::ContradictingFact),
            Just(ConflictType::IncompatibleDecision),
            Just(ConflictType::ResourceContention),
            Just(ConflictType::GoalConflict),
        ]
    }

    /// Generate an item type string
    fn arb_item_type() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("artifact".to_string()),
            Just("note".to_string()),
            Just("decision".to_string()),
            Just("resource".to_string()),
        ]
    }

    /// Generate a conflict resolution record
    fn arb_resolution() -> impl Strategy<Value = ConflictResolutionRecord> {
        (
            prop_oneof![
                Just(ResolutionStrategy::LastWriteWins),
                Just(ResolutionStrategy::FirstWriteWins),
                Just(ResolutionStrategy::HighestConfidence),
                Just(ResolutionStrategy::Merge),
                Just(ResolutionStrategy::Escalate),
                Just(ResolutionStrategy::RejectBoth),
            ],
            prop_oneof![
                Just(None),
                Just(Some("a".to_string())),
                Just(Some("b".to_string())),
                Just(Some("merged".to_string())),
            ],
            prop_oneof![
                Just("Automatic resolution".to_string()),
                Just("Manual intervention required".to_string()),
                Just("Merged conflicting items".to_string()),
            ],
        ).prop_map(|(strategy, winner, reason)| {
            ConflictResolutionRecord {
                strategy,
                winner,
                merged_result_id: None,
                reason,
                resolved_by: "automatic".to_string(),
            }
        })
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Conflict)
    // Feature: caliber-pg-hot-path, Property 1: Insert-Get Round Trip
    // Validates: Requirements 11.1, 11.2
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use pgrx_tests::pg_test;

        /// Property 1: Insert-Get Round Trip (Conflict)
        /// 
        /// *For any* valid conflict data, inserting via direct heap then getting
        /// via direct heap SHALL return an equivalent conflict.
        ///
        /// **Validates: Requirements 11.1, 11.2**
        #[pg_test]
        fn prop_conflict_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_conflict_type(),
                arb_item_type(),
                arb_entity_id(),
                arb_item_type(),
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_entity_id(),
                arb_optional_entity_id(),
            );

            runner.run(&strategy, |(
                conflict_type,
                item_a_type,
                item_a_id,
                item_b_type,
                item_b_id,
                agent_a_id,
                agent_b_id,
                trajectory_id,
            )| {
                // Generate a new conflict ID
                let conflict_id = caliber_core::new_entity_id();

                // Insert via heap
                let result = conflict_create_heap(ConflictCreateParams {
                    conflict_id,
                    conflict_type,
                    item_a_type: &item_a_type,
                    item_a_id,
                    item_b_type: &item_b_type,
                    item_b_id,
                    agent_a_id,
                    agent_b_id,
                    trajectory_id,
                });
                prop_assert!(result.is_ok(), "Insert should succeed: {:?}", result.err());
                prop_assert_eq!(result.unwrap(), conflict_id);

                // Get via heap
                let get_result = conflict_get_heap(conflict_id);
                prop_assert!(get_result.is_ok(), "Get should succeed: {:?}", get_result.err());
                
                let conflict = get_result.unwrap();
                prop_assert!(conflict.is_some(), "Conflict should be found");
                
                let c = conflict.unwrap();
                
                // Verify round-trip preserves data
                prop_assert_eq!(c.conflict_id, conflict_id);
                prop_assert_eq!(c.conflict_type, conflict_type);
                prop_assert_eq!(c.item_a_type, item_a_type);
                prop_assert_eq!(c.item_a_id, item_a_id);
                prop_assert_eq!(c.item_b_type, item_b_type);
                prop_assert_eq!(c.item_b_id, item_b_id);
                prop_assert_eq!(c.agent_a_id, agent_a_id);
                prop_assert_eq!(c.agent_b_id, agent_b_id);
                prop_assert_eq!(c.trajectory_id, trajectory_id);
                prop_assert_eq!(c.status, ConflictStatus::Detected);
                
                // Timestamps should be set
                prop_assert!(c.detected_at <= chrono::Utc::now());
                prop_assert!(c.resolved_at.is_none(), "resolved_at should be None initially");
                prop_assert!(c.resolution.is_none(), "resolution should be None initially");

                Ok(())
            }).unwrap();
        }

        /// Property 1 (edge case): Get non-existent conflict returns None
        ///
        /// *For any* random UUID that was never inserted, getting it SHALL
        /// return Ok(None), not an error.
        ///
        /// **Validates: Requirements 11.2**
        #[pg_test]
        fn prop_conflict_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let result = conflict_get_heap(random_id);
                prop_assert!(result.is_ok(), "Get should not error: {:?}", result.err());
                prop_assert!(result.unwrap().is_none(), "Non-existent conflict should return None");

                Ok(())
            }).unwrap();
        }

        /// Property 2: Update Persistence (Conflict - resolve)
        ///
        /// *For any* conflict that has been inserted, resolving it SHALL
        /// update the status, resolution, and resolved_at fields and persist the change.
        ///
        /// **Validates: Requirements 11.3**
        #[pg_test]
        fn prop_conflict_resolve_persistence() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_conflict_type(),
                arb_item_type(),
                arb_entity_id(),
                arb_item_type(),
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_entity_id(),
                arb_optional_entity_id(),
                arb_resolution(),
            );

            runner.run(&strategy, |(
                conflict_type,
                item_a_type,
                item_a_id,
                item_b_type,
                item_b_id,
                agent_a_id,
                agent_b_id,
                trajectory_id,
                resolution,
            )| {
                // Generate a new conflict ID
                let conflict_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = conflict_create_heap(ConflictCreateParams {
                    conflict_id,
                    conflict_type,
                    item_a_type: &item_a_type,
                    item_a_id,
                    item_b_type: &item_b_type,
                    item_b_id,
                    agent_a_id,
                    agent_b_id,
                    trajectory_id,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Verify initial status is Detected
                let get_before = conflict_get_heap(conflict_id);
                prop_assert!(get_before.is_ok(), "Get before resolve should succeed");
                let conflict_before = get_before.unwrap().unwrap();
                prop_assert_eq!(conflict_before.status, ConflictStatus::Detected);
                prop_assert!(conflict_before.resolved_at.is_none(), "resolved_at should be None before resolve");
                prop_assert!(conflict_before.resolution.is_none(), "resolution should be None before resolve");

                // Resolve the conflict
                let resolve_result = conflict_resolve_heap(conflict_id, &resolution);
                prop_assert!(resolve_result.is_ok(), "Resolve should succeed: {:?}", resolve_result.err());
                prop_assert!(resolve_result.unwrap(), "Resolve should return true for existing conflict");

                // Verify status, resolution, and resolved_at were updated
                let get_after = conflict_get_heap(conflict_id);
                prop_assert!(get_after.is_ok(), "Get after resolve should succeed");
                let conflict_after = get_after.unwrap().unwrap();
                prop_assert_eq!(conflict_after.status, ConflictStatus::Resolved, "Status should be Resolved");
                prop_assert!(conflict_after.resolved_at.is_some(), "resolved_at should be set after resolve");
                prop_assert!(conflict_after.resolved_at.unwrap() <= chrono::Utc::now(), "resolved_at should be <= now");
                prop_assert!(conflict_after.resolution.is_some(), "resolution should be set after resolve");
                
                // Verify resolution data
                let res = conflict_after.resolution.unwrap();
                prop_assert_eq!(res.strategy, resolution.strategy);
                prop_assert_eq!(res.winner, resolution.winner);
                prop_assert_eq!(res.reason, resolution.reason);

                Ok(())
            }).unwrap();
        }


        /// Property 2 (edge case): Resolve non-existent conflict returns false
        ///
        /// *For any* random UUID that was never inserted, resolving it SHALL
        /// return Ok(false), not an error.
        ///
        /// **Validates: Requirements 11.3**
        #[pg_test]
        fn prop_conflict_resolve_nonexistent_returns_false() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (any::<[u8; 16]>(), arb_resolution());

            runner.run(&strategy, |(bytes, resolution)| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let result = conflict_resolve_heap(random_id, &resolution);
                prop_assert!(result.is_ok(), "Resolve should not error: {:?}", result.err());
                prop_assert!(!result.unwrap(), "Resolve of non-existent conflict should return false");

                Ok(())
            }).unwrap();
        }

        /// Property 3: Index Consistency - List Pending
        ///
        /// *For any* conflict inserted with "detected" status, querying via
        /// conflict_list_pending_heap SHALL return that conflict.
        ///
        /// **Validates: Requirements 11.4, 13.1, 13.2, 13.4, 13.5**
        #[pg_test]
        fn prop_conflict_list_pending_consistency() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_conflict_type(),
                arb_item_type(),
                arb_entity_id(),
                arb_item_type(),
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_entity_id(),
                arb_optional_entity_id(),
            );

            runner.run(&strategy, |(
                conflict_type,
                item_a_type,
                item_a_id,
                item_b_type,
                item_b_id,
                agent_a_id,
                agent_b_id,
                trajectory_id,
            )| {
                // Generate a new conflict ID
                let conflict_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = conflict_create_heap(ConflictCreateParams {
                    conflict_id,
                    conflict_type,
                    item_a_type: &item_a_type,
                    item_a_id,
                    item_b_type: &item_b_type,
                    item_b_id,
                    agent_a_id,
                    agent_b_id,
                    trajectory_id,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Query via list_pending
                let list_result = conflict_list_pending_heap();
                prop_assert!(list_result.is_ok(), "List pending should succeed: {:?}", list_result.err());
                
                let conflicts = list_result.unwrap();
                prop_assert!(
                    conflicts.iter().any(|c| c.conflict_id == conflict_id),
                    "Inserted conflict should be found via list_pending"
                );

                // Verify the found conflict has correct data
                let found_conflict = conflicts.iter().find(|c| c.conflict_id == conflict_id).unwrap();
                prop_assert_eq!(found_conflict.conflict_type, conflict_type);
                prop_assert_eq!(found_conflict.item_a_type, item_a_type);
                prop_assert_eq!(found_conflict.item_a_id, item_a_id);
                prop_assert_eq!(found_conflict.item_b_type, item_b_type);
                prop_assert_eq!(found_conflict.item_b_id, item_b_id);
                prop_assert_eq!(found_conflict.status, ConflictStatus::Detected);

                Ok(())
            }).unwrap();
        }

        /// Property 3 (edge case): Resolved conflicts not in pending list
        ///
        /// *For any* conflict that has been resolved, it SHALL NOT appear
        /// in the list_pending results.
        ///
        /// **Validates: Requirements 11.4**
        #[pg_test]
        fn prop_conflict_resolved_not_in_pending() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_conflict_type(),
                arb_item_type(),
                arb_entity_id(),
                arb_item_type(),
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_entity_id(),
                arb_optional_entity_id(),
                arb_resolution(),
            );

            runner.run(&strategy, |(
                conflict_type,
                item_a_type,
                item_a_id,
                item_b_type,
                item_b_id,
                agent_a_id,
                agent_b_id,
                trajectory_id,
                resolution,
            )| {
                // Generate a new conflict ID
                let conflict_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = conflict_create_heap(ConflictCreateParams {
                    conflict_id,
                    conflict_type,
                    item_a_type: &item_a_type,
                    item_a_id,
                    item_b_type: &item_b_type,
                    item_b_id,
                    agent_a_id,
                    agent_b_id,
                    trajectory_id,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Verify it appears in pending list
                let list_before = conflict_list_pending_heap();
                prop_assert!(list_before.is_ok(), "List pending before resolve should succeed");
                prop_assert!(
                    list_before.unwrap().iter().any(|c| c.conflict_id == conflict_id),
                    "Conflict should be in pending list before resolve"
                );

                // Resolve the conflict
                let resolve_result = conflict_resolve_heap(conflict_id, &resolution);
                prop_assert!(resolve_result.is_ok(), "Resolve should succeed");

                // Verify it no longer appears in pending list
                let list_after = conflict_list_pending_heap();
                prop_assert!(list_after.is_ok(), "List pending after resolve should succeed");
                prop_assert!(
                    !list_after.unwrap().iter().any(|c| c.conflict_id == conflict_id),
                    "Resolved conflict should NOT be in pending list"
                );

                Ok(())
            }).unwrap();
        }
    }
}
