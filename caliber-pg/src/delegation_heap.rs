//! Direct heap operations for Delegation entities.
//!
//! This module provides hot-path operations for task delegation between agents
//! that bypass SQL parsing entirely by using direct heap access via pgrx.

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    CaliberError, CaliberResult, EntityId, EntityType, StorageError,
};
use caliber_agents::{DelegatedTask, DelegationStatus, DelegationResult};

use crate::column_maps::delegation;
use crate::heap_ops::{
    current_timestamp, form_tuple, insert_tuple, open_relation, update_tuple,
    PgLockMode as HeapLockMode, HeapRelation, get_active_snapshot,
    timestamp_to_pgrx,
};
use crate::index_ops::{
    init_scan_key, open_index, update_indexes_for_insert,
    BTreeStrategy, IndexScanner, operator_oids,
};
use crate::tuple_extract::{
    extract_uuid, extract_text, extract_timestamp, extract_jsonb,
    extract_uuid_array, extract_values_and_nulls, uuid_to_datum,
    string_to_datum, timestamp_to_chrono, uuid_array_to_datum,
    json_to_datum, option_datetime_to_datum,
};

/// Delegation row with tenant ownership metadata.
pub struct DelegationRow {
    pub delegation: DelegatedTask,
    pub tenant_id: Option<EntityId>,
}

impl From<DelegationRow> for DelegatedTask {
    fn from(row: DelegationRow) -> Self {
        row.delegation
    }
}

/// Create a new delegation by inserting a delegation record using direct heap operations.
pub struct DelegationCreateParams<'a> {
    pub delegation_id: EntityId,
    pub delegator_agent_id: EntityId,
    pub delegatee_agent_id: Option<EntityId>,
    pub delegatee_agent_type: Option<&'a str>,
    pub task_description: &'a str,
    pub parent_trajectory_id: EntityId,
    pub child_trajectory_id: Option<EntityId>,
    pub shared_artifacts: &'a [EntityId],
    pub shared_notes: &'a [EntityId],
    pub additional_context: Option<&'a str>,
    pub constraints: &'a str,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub tenant_id: EntityId,
}

pub fn delegation_create_heap(params: DelegationCreateParams<'_>) -> CaliberResult<EntityId> {
    let DelegationCreateParams {
        delegation_id,
        delegator_agent_id,
        delegatee_agent_id,
        delegatee_agent_type,
        task_description,
        parent_trajectory_id,
        child_trajectory_id,
        shared_artifacts,
        shared_notes,
        additional_context,
        constraints,
        deadline,
        tenant_id,
    } = params;
    let rel = open_relation(delegation::TABLE_NAME, HeapLockMode::RowExclusive)?;
    validate_delegation_relation(&rel)?;

    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now)?.into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Delegation,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    let mut values: [pg_sys::Datum; delegation::NUM_COLS] = [pg_sys::Datum::from(0); delegation::NUM_COLS];
    let mut nulls: [bool; delegation::NUM_COLS] = [false; delegation::NUM_COLS];

    // Use helper for optional fields (delegatee_agent_id, child_trajectory_id, deadline)
    let ((delegatee_datum, delegatee_null), (child_datum, child_null), (deadline_datum, deadline_null)) =
        build_optional_delegation_datums(delegatee_agent_id, child_trajectory_id, deadline)?;

    // Set required fields
    values[delegation::DELEGATION_ID as usize - 1] = uuid_to_datum(delegation_id);
    values[delegation::DELEGATOR_AGENT_ID as usize - 1] = uuid_to_datum(delegator_agent_id);

    // Set optional delegatee_agent_id
    values[delegation::DELEGATEE_AGENT_ID as usize - 1] = delegatee_datum;
    nulls[delegation::DELEGATEE_AGENT_ID as usize - 1] = delegatee_null;

    // Set optional delegatee_agent_type
    if let Some(agent_type) = delegatee_agent_type {
        values[delegation::DELEGATEE_AGENT_TYPE as usize - 1] = string_to_datum(agent_type);
    } else {
        nulls[delegation::DELEGATEE_AGENT_TYPE as usize - 1] = true;
    }

    // Set task_description
    values[delegation::TASK_DESCRIPTION as usize - 1] = string_to_datum(task_description);

    // Set parent_trajectory_id
    values[delegation::PARENT_TRAJECTORY_ID as usize - 1] = uuid_to_datum(parent_trajectory_id);

    // Set optional child_trajectory_id
    values[delegation::CHILD_TRAJECTORY_ID as usize - 1] = child_datum;
    nulls[delegation::CHILD_TRAJECTORY_ID as usize - 1] = child_null;

    // Set shared_artifacts array
    if shared_artifacts.is_empty() {
        nulls[delegation::SHARED_ARTIFACTS as usize - 1] = true;
    } else {
        values[delegation::SHARED_ARTIFACTS as usize - 1] = uuid_array_to_datum(shared_artifacts);
    }

    // Set shared_notes array
    if shared_notes.is_empty() {
        nulls[delegation::SHARED_NOTES as usize - 1] = true;
    } else {
        values[delegation::SHARED_NOTES as usize - 1] = uuid_array_to_datum(shared_notes);
    }

    // Set optional additional_context
    if let Some(context) = additional_context {
        values[delegation::ADDITIONAL_CONTEXT as usize - 1] = string_to_datum(context);
    } else {
        nulls[delegation::ADDITIONAL_CONTEXT as usize - 1] = true;
    }

    // Set constraints
    values[delegation::CONSTRAINTS as usize - 1] = string_to_datum(constraints);

    // Set optional deadline
    values[delegation::DEADLINE as usize - 1] = deadline_datum;
    nulls[delegation::DEADLINE as usize - 1] = deadline_null;

    // Set tenant_id
    values[delegation::TENANT_ID as usize - 1] = uuid_to_datum(tenant_id);
    
    // Set status - default to "pending"
    values[delegation::STATUS as usize - 1] = string_to_datum("pending");
    
    // Set result to NULL initially
    nulls[delegation::RESULT as usize - 1] = true;
    
    // Set timestamps
    values[delegation::CREATED_AT as usize - 1] = now_datum;
    nulls[delegation::ACCEPTED_AT as usize - 1] = true;
    nulls[delegation::COMPLETED_AT as usize - 1] = true;
    
    let tuple = form_tuple(&rel, &values, &nulls)?;
    let _tid = unsafe { insert_tuple(&rel, tuple)? };
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };
    
    Ok(delegation_id)
}

/// Get a delegation by ID using direct heap operations.
pub fn delegation_get_heap(
    delegation_id: EntityId,
    tenant_id: EntityId,
) -> CaliberResult<Option<DelegationRow>> {
    let rel = open_relation(delegation::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(delegation::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(delegation_id),
    );
    
    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };
    
    if let Some(tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let row = unsafe { tuple_to_delegation(tuple, tuple_desc) }?;
        if row.tenant_id == Some(tenant_id) {
            Ok(Some(row))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Accept a delegation by updating status, accepted_at, delegatee_agent_id, and child_trajectory_id.
///
/// This function updates the delegation to record:
/// - The agent that accepted the delegation (delegatee_agent_id)
/// - The child trajectory created for the delegated work (child_trajectory_id)
/// - The acceptance timestamp
/// - Status change to "accepted"
pub fn delegation_accept_heap(
    delegation_id: EntityId,
    delegatee_agent_id: EntityId,
    child_trajectory_id: EntityId,
    tenant_id: EntityId,
) -> CaliberResult<bool> {
    let rel = open_relation(delegation::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(delegation::PK_INDEX)?;
    let snapshot = get_active_snapshot();

    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(delegation_id),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    if let Some(old_tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, delegation::TENANT_ID)? };
        if existing_tenant != Some(tenant_id) {
            return Ok(false);
        }
        let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;

        // Update delegatee_agent_id - the agent accepting the delegation
        values[delegation::DELEGATEE_AGENT_ID as usize - 1] = uuid_to_datum(delegatee_agent_id);
        nulls[delegation::DELEGATEE_AGENT_ID as usize - 1] = false;

        // Update child_trajectory_id - the trajectory created for the delegated work
        values[delegation::CHILD_TRAJECTORY_ID as usize - 1] = uuid_to_datum(child_trajectory_id);
        nulls[delegation::CHILD_TRAJECTORY_ID as usize - 1] = false;

        // Update status to "accepted"
        values[delegation::STATUS as usize - 1] = string_to_datum("accepted");

        // Update accepted_at to current timestamp
        let now = current_timestamp();
        let now_datum = timestamp_to_pgrx(now)?.into_datum()
            .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Delegation,
                id: delegation_id,
                reason: "Failed to convert timestamp to datum".to_string(),
            }))?;

        values[delegation::ACCEPTED_AT as usize - 1] = now_datum;
        nulls[delegation::ACCEPTED_AT as usize - 1] = false;

        let new_tuple = form_tuple(&rel, &values, &nulls)?;
        let old_tid = scanner.current_tid()
            .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of delegation tuple".to_string(),
            }))?;

        unsafe { update_tuple(&rel, &old_tid, new_tuple)? };
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Complete a delegation by updating status, result, and completed_at using direct heap operations.
pub fn delegation_complete_heap(
    delegation_id: EntityId,
    result: &DelegationResult,
    tenant_id: EntityId,
) -> CaliberResult<bool> {
    let rel = open_relation(delegation::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(delegation::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(delegation_id),
    );
    
    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };
    
    if let Some(old_tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, delegation::TENANT_ID)? };
        if existing_tenant != Some(tenant_id) {
            return Ok(false);
        }
        let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;
        
        // Update status to "completed"
        values[delegation::STATUS as usize - 1] = string_to_datum("completed");
        
        // Serialize result to JSON
        let result_json = serde_json::to_value(result)
            .map_err(|e| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Delegation,
                id: delegation_id,
                reason: format!("Failed to serialize result: {}", e),
            }))?;
        
        values[delegation::RESULT as usize - 1] = json_to_datum(&result_json);
        nulls[delegation::RESULT as usize - 1] = false;
        
        // Update completed_at to current timestamp
        let now = current_timestamp();
        let now_datum = timestamp_to_pgrx(now)?.into_datum()
            .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Delegation,
                id: delegation_id,
                reason: "Failed to convert timestamp to datum".to_string(),
            }))?;
        
        values[delegation::COMPLETED_AT as usize - 1] = now_datum;
        nulls[delegation::COMPLETED_AT as usize - 1] = false;
        
        let new_tuple = form_tuple(&rel, &values, &nulls)?;
        let old_tid = scanner.current_tid()
            .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of delegation tuple".to_string(),
            }))?;
        
        unsafe { update_tuple(&rel, &old_tid, new_tuple)? };
        Ok(true)
    } else {
        Ok(false)
    }
}

/// List pending delegations using direct heap operations.
pub fn delegation_list_pending_heap(tenant_id: EntityId) -> CaliberResult<Vec<DelegationRow>> {
    let rel = open_relation(delegation::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(delegation::STATUS_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::TEXT_EQ,
        string_to_datum("pending"),
    );
    
    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };
    
    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();
    
    for tuple in &mut scanner {
        let row = unsafe { tuple_to_delegation(tuple, tuple_desc) }?;
        if row.tenant_id == Some(tenant_id) {
            results.push(row);
        }
    }
    
    Ok(results)
}

/// Validate that a HeapRelation is suitable for delegation operations.
fn validate_delegation_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != delegation::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Delegation relation has {} columns, expected {}",
                natts,
                delegation::NUM_COLS
            ),
        }));
    }
    Ok(())
}

/// Build optional delegation datums using proper helpers.
fn build_optional_delegation_datums(
    delegatee_agent_id: Option<EntityId>,
    child_trajectory_id: Option<EntityId>,
    deadline: Option<chrono::DateTime<chrono::Utc>>,
) -> CaliberResult<((pg_sys::Datum, bool), (pg_sys::Datum, bool), (pg_sys::Datum, bool))> {
    let delegatee = match delegatee_agent_id {
        Some(id) => (uuid_to_datum(id), false),
        None => (pg_sys::Datum::from(0), true),
    };
    let child = match child_trajectory_id {
        Some(id) => (uuid_to_datum(id), false),
        None => (pg_sys::Datum::from(0), true),
    };
    let dl = match deadline {
        Some(dt) => (option_datetime_to_datum(Some(dt))?, false),
        None => (pg_sys::Datum::from(0), true),
    };
    Ok((delegatee, child, dl))
}

unsafe fn tuple_to_delegation(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<DelegationRow> {
    let delegation_id = extract_uuid(tuple, tuple_desc, delegation::DELEGATION_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "delegation_id is NULL".to_string(),
        }))?;
    
    let delegator_agent_id = extract_uuid(tuple, tuple_desc, delegation::DELEGATOR_AGENT_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "delegator_agent_id is NULL".to_string(),
        }))?;
    
    let delegatee_agent_id = extract_uuid(tuple, tuple_desc, delegation::DELEGATEE_AGENT_ID)?;
    let delegatee_agent_type = extract_text(tuple, tuple_desc, delegation::DELEGATEE_AGENT_TYPE)?;
    
    let task_description = extract_text(tuple, tuple_desc, delegation::TASK_DESCRIPTION)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "task_description is NULL".to_string(),
        }))?;
    
    let parent_trajectory_id = extract_uuid(tuple, tuple_desc, delegation::PARENT_TRAJECTORY_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "parent_trajectory_id is NULL".to_string(),
        }))?;
    
    let child_trajectory_id = extract_uuid(tuple, tuple_desc, delegation::CHILD_TRAJECTORY_ID)?;
    
    let shared_artifacts = extract_uuid_array(tuple, tuple_desc, delegation::SHARED_ARTIFACTS)?
        .unwrap_or_default();
    
    let shared_notes = extract_uuid_array(tuple, tuple_desc, delegation::SHARED_NOTES)?
        .unwrap_or_default();
    
    let additional_context = extract_text(tuple, tuple_desc, delegation::ADDITIONAL_CONTEXT)?;
    
    let constraints = extract_text(tuple, tuple_desc, delegation::CONSTRAINTS)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "constraints is NULL".to_string(),
        }))?;
    
    let deadline = extract_timestamp(tuple, tuple_desc, delegation::DEADLINE)?
        .map(timestamp_to_chrono);
    
    let status_str = extract_text(tuple, tuple_desc, delegation::STATUS)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "status is NULL".to_string(),
        }))?;
    let status = match status_str.as_str() {
        "pending" => DelegationStatus::Pending,
        "accepted" => DelegationStatus::Accepted,
        "rejected" => DelegationStatus::Rejected,
        "in_progress" => DelegationStatus::InProgress,
        "completed" => DelegationStatus::Completed,
        "failed" => DelegationStatus::Failed,
        _ => {
            pgrx::warning!("CALIBER: Unknown delegation status '{}', defaulting to Pending", status_str);
            DelegationStatus::Pending
        }
    };
    
    let result = extract_jsonb(tuple, tuple_desc, delegation::RESULT)?
        .and_then(|json| serde_json::from_value::<DelegationResult>(json).ok());
    
    let created_at_ts = extract_timestamp(tuple, tuple_desc, delegation::CREATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "created_at is NULL".to_string(),
        }))?;
    let created_at = timestamp_to_chrono(created_at_ts);
    
    let accepted_at = extract_timestamp(tuple, tuple_desc, delegation::ACCEPTED_AT)?
        .map(timestamp_to_chrono);
    
    let completed_at = extract_timestamp(tuple, tuple_desc, delegation::COMPLETED_AT)?
        .map(timestamp_to_chrono);

    let tenant_id = extract_uuid(tuple, tuple_desc, delegation::TENANT_ID)?;
    
    Ok(DelegationRow {
        delegation: DelegatedTask {
            delegation_id,
            delegator_agent_id,
            delegatee_agent_id,
            delegatee_agent_type,
            task_description,
            parent_trajectory_id,
            child_trajectory_id,
            shared_artifacts,
            shared_notes,
            additional_context,
            constraints,
            deadline,
            status,
            result,
            created_at,
            accepted_at,
            completed_at,
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
    use chrono::Duration;

    // ========================================================================
    // Test Helpers - Generators for Delegation data
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

    /// Generate an optional agent type string
    fn arb_optional_agent_type() -> impl Strategy<Value = Option<String>> {
        prop_oneof![
            1 => Just(None),
            3 => prop_oneof![
                Just(Some("coordinator".to_string())),
                Just(Some("worker".to_string())),
                Just(Some("specialist".to_string())),
            ],
        ]
    }

    /// Generate a task description
    fn arb_task_description() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Process data and generate report".to_string()),
            Just("Analyze logs for errors".to_string()),
            Just("Review code changes".to_string()),
            Just("Execute test suite".to_string()),
        ]
    }

    /// Generate a constraints string (JSON)
    fn arb_constraints() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("{}".to_string()),
            Just(r#"{"timeout":3600}"#.to_string()),
            Just(r#"{"max_tokens":1000}"#.to_string()),
        ]
    }

    /// Generate an optional future deadline
    fn arb_optional_deadline() -> impl Strategy<Value = Option<chrono::DateTime<chrono::Utc>>> {
        prop_oneof![
            2 => Just(None),
            1 => (60i64..3600i64).prop_map(|seconds| {
                Some(chrono::Utc::now() + Duration::seconds(seconds))
            }),
        ]
    }

    /// Generate a vector of artifact IDs (0-5 items)
    fn arb_artifact_ids() -> impl Strategy<Value = Vec<EntityId>> {
        prop::collection::vec(arb_entity_id(), 0..=5)
    }

    /// Generate an optional additional context string
    fn arb_optional_context() -> impl Strategy<Value = Option<String>> {
        prop_oneof![
            2 => Just(None),
            1 => prop_oneof![
                Just(Some("Additional context here".to_string())),
                Just(Some("See previous discussion".to_string())),
            ],
        ]
    }

    /// Generate a delegation result
    fn arb_delegation_result() -> impl Strategy<Value = DelegationResult> {
        (arb_artifact_ids(), arb_artifact_ids()).prop_map(|(artifacts, notes)| {
            DelegationResult {
                status: caliber_agents::DelegationResultStatus::Success,
                produced_artifacts: artifacts,
                produced_notes: notes,
                summary: "Task completed successfully".to_string(),
                error: None,
            }
        })
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Delegation)
    // Feature: caliber-pg-hot-path, Property 1: Insert-Get Round Trip
    // Validates: Requirements 9.1, 9.2
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use crate::pg_test;

        /// Property 1: Insert-Get Round Trip (Delegation)
        /// 
        /// *For any* valid delegation data, inserting via direct heap then getting
        /// via direct heap SHALL return an equivalent delegation.
        ///
        /// **Validates: Requirements 9.1, 9.2**
        #[pg_test]
        fn prop_delegation_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_agent_type(),
                arb_task_description(),
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_artifact_ids(),
                arb_artifact_ids(),
                arb_optional_context(),
                arb_constraints(),
                arb_optional_deadline(),
            );

            runner.run(&strategy, |(
                delegator_agent_id,
                delegatee_agent_id,
                delegatee_agent_type,
                task_description,
                parent_trajectory_id,
                child_trajectory_id,
                shared_artifacts,
                shared_notes,
                additional_context,
                constraints,
                deadline,
            )| {
                // Generate a new delegation ID
                let delegation_id = caliber_core::new_entity_id();
                let tenant_id = caliber_core::new_entity_id();

                // Insert via heap
                let result = delegation_create_heap(DelegationCreateParams {
                    delegation_id,
                    delegator_agent_id,
                    delegatee_agent_id,
                    delegatee_agent_type: delegatee_agent_type.as_deref(),
                    task_description: &task_description,
                    parent_trajectory_id,
                    child_trajectory_id,
                    shared_artifacts: &shared_artifacts,
                    shared_notes: &shared_notes,
                    additional_context: additional_context.as_deref(),
                    constraints: &constraints,
                    deadline,
                    tenant_id,
                });
                prop_assert!(result.is_ok(), "Insert should succeed: {:?}", result.err());
                prop_assert_eq!(result.unwrap(), delegation_id);

                // Get via heap
                let get_result = delegation_get_heap(delegation_id, tenant_id);
                prop_assert!(get_result.is_ok(), "Get should succeed: {:?}", get_result.err());
                
                let delegation = get_result.unwrap();
                prop_assert!(delegation.is_some(), "Delegation should be found");
                
                let row = delegation.unwrap();
                let d = row.delegation;
                
                // Verify round-trip preserves data
                prop_assert_eq!(d.delegation_id, delegation_id);
                prop_assert_eq!(d.delegator_agent_id, delegator_agent_id);
                prop_assert_eq!(d.delegatee_agent_id, delegatee_agent_id);
                prop_assert_eq!(d.delegatee_agent_type, delegatee_agent_type);
                prop_assert_eq!(d.task_description, task_description);
                prop_assert_eq!(d.parent_trajectory_id, parent_trajectory_id);
                prop_assert_eq!(d.child_trajectory_id, child_trajectory_id);
                prop_assert_eq!(d.shared_artifacts, shared_artifacts);
                prop_assert_eq!(d.shared_notes, shared_notes);
                prop_assert_eq!(d.additional_context, additional_context);
                prop_assert_eq!(d.constraints, constraints);
                prop_assert_eq!(d.status, DelegationStatus::Pending);
                
                // Timestamps should be set
                prop_assert!(d.created_at <= chrono::Utc::now());
                prop_assert!(d.accepted_at.is_none(), "accepted_at should be None initially");
                prop_assert!(d.completed_at.is_none(), "completed_at should be None initially");
                prop_assert!(d.result.is_none(), "result should be None initially");
                prop_assert_eq!(row.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Property 1 (edge case): Get non-existent delegation returns None
        ///
        /// *For any* random UUID that was never inserted, getting it SHALL
        /// return Ok(None), not an error.
        ///
        /// **Validates: Requirements 9.2**
        #[pg_test]
        fn prop_delegation_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let tenant_id = caliber_core::new_entity_id();
                let result = delegation_get_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Get should not error: {:?}", result.err());
                prop_assert!(result.unwrap().is_none(), "Non-existent delegation should return None");

                Ok(())
            }).unwrap();
        }

        /// Property 2: Update Persistence (Delegation - accept)
        ///
        /// *For any* delegation that has been inserted, accepting it SHALL
        /// update the status and accepted_at fields and persist the change.
        ///
        /// **Validates: Requirements 9.3**
        #[pg_test]
        fn prop_delegation_accept_persistence() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_agent_type(),
                arb_task_description(),
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_artifact_ids(),
                arb_artifact_ids(),
                arb_optional_context(),
                arb_constraints(),
                arb_optional_deadline(),
            );

            runner.run(&strategy, |(
                delegator_agent_id,
                delegatee_agent_id,
                delegatee_agent_type,
                task_description,
                parent_trajectory_id,
                child_trajectory_id,
                shared_artifacts,
                shared_notes,
                additional_context,
                constraints,
                deadline,
            )| {
                // Generate a new delegation ID
                let delegation_id = caliber_core::new_entity_id();
                let tenant_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = delegation_create_heap(DelegationCreateParams {
                    delegation_id,
                    delegator_agent_id,
                    delegatee_agent_id,
                    delegatee_agent_type: delegatee_agent_type.as_deref(),
                    task_description: &task_description,
                    parent_trajectory_id,
                    child_trajectory_id,
                    shared_artifacts: &shared_artifacts,
                    shared_notes: &shared_notes,
                    additional_context: additional_context.as_deref(),
                    constraints: &constraints,
                    deadline,
                    tenant_id,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Verify initial status is Pending
                let get_before = delegation_get_heap(delegation_id, tenant_id);
                prop_assert!(get_before.is_ok(), "Get before accept should succeed");
                let delegation_before = get_before.unwrap().unwrap();
                prop_assert_eq!(delegation_before.delegation.status, DelegationStatus::Pending);
                prop_assert!(delegation_before.delegation.accepted_at.is_none(), "accepted_at should be None before accept");
                prop_assert_eq!(delegation_before.tenant_id, Some(tenant_id));

                // Generate IDs for acceptance
                let accepting_agent_id = caliber_core::new_entity_id();
                let new_child_trajectory_id = caliber_core::new_entity_id();

                // Accept the delegation with accepting agent and child trajectory
                let accept_result = delegation_accept_heap(
                    delegation_id,
                    accepting_agent_id,
                    new_child_trajectory_id,
                    tenant_id,
                );
                prop_assert!(accept_result.is_ok(), "Accept should succeed: {:?}", accept_result.err());
                prop_assert!(accept_result.unwrap(), "Accept should return true for existing delegation");

                // Verify status, accepted_at, delegatee_agent_id, and child_trajectory_id were updated
                let get_after = delegation_get_heap(delegation_id, tenant_id);
                prop_assert!(get_after.is_ok(), "Get after accept should succeed");
                let delegation_after = get_after.unwrap().unwrap();
                prop_assert_eq!(delegation_after.delegation.status, DelegationStatus::Accepted, "Status should be Accepted");
                prop_assert!(delegation_after.delegation.accepted_at.is_some(), "accepted_at should be set after accept");
                prop_assert!(delegation_after.delegation.accepted_at.unwrap() <= chrono::Utc::now(), "accepted_at should be <= now");
                prop_assert_eq!(delegation_after.delegation.delegatee_agent_id, Some(accepting_agent_id), "delegatee_agent_id should be set");
                prop_assert_eq!(delegation_after.delegation.child_trajectory_id, Some(new_child_trajectory_id), "child_trajectory_id should be set");
                prop_assert_eq!(delegation_after.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Property 2: Update Persistence (Delegation - complete)
        ///
        /// *For any* delegation that has been inserted, completing it SHALL
        /// update the status, result, and completed_at fields and persist the change.
        ///
        /// **Validates: Requirements 9.4**
        #[pg_test]
        fn prop_delegation_complete_persistence() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_agent_type(),
                arb_task_description(),
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_artifact_ids(),
                arb_artifact_ids(),
                arb_optional_context(),
                arb_constraints(),
                arb_optional_deadline(),
                arb_delegation_result(),
            );

            runner.run(&strategy, |(
                delegator_agent_id,
                delegatee_agent_id,
                delegatee_agent_type,
                task_description,
                parent_trajectory_id,
                child_trajectory_id,
                shared_artifacts,
                shared_notes,
                additional_context,
                constraints,
                deadline,
                result,
            )| {
                // Generate a new delegation ID
                let delegation_id = caliber_core::new_entity_id();
                let tenant_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = delegation_create_heap(DelegationCreateParams {
                    delegation_id,
                    delegator_agent_id,
                    delegatee_agent_id,
                    delegatee_agent_type: delegatee_agent_type.as_deref(),
                    task_description: &task_description,
                    parent_trajectory_id,
                    child_trajectory_id,
                    shared_artifacts: &shared_artifacts,
                    shared_notes: &shared_notes,
                    additional_context: additional_context.as_deref(),
                    constraints: &constraints,
                    deadline,
                    tenant_id,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Verify initial state
                let get_before = delegation_get_heap(delegation_id, tenant_id);
                prop_assert!(get_before.is_ok(), "Get before complete should succeed");
                let delegation_before = get_before.unwrap().unwrap();
                prop_assert_eq!(delegation_before.delegation.status, DelegationStatus::Pending);
                prop_assert!(delegation_before.delegation.result.is_none(), "result should be None before complete");
                prop_assert!(delegation_before.delegation.completed_at.is_none(), "completed_at should be None before complete");
                prop_assert_eq!(delegation_before.tenant_id, Some(tenant_id));

                // Complete the delegation
                let complete_result = delegation_complete_heap(delegation_id, &result, tenant_id);
                prop_assert!(complete_result.is_ok(), "Complete should succeed: {:?}", complete_result.err());
                prop_assert!(complete_result.unwrap(), "Complete should return true for existing delegation");

                // Verify status, result, and completed_at were updated
                let get_after = delegation_get_heap(delegation_id, tenant_id);
                prop_assert!(get_after.is_ok(), "Get after complete should succeed");
                let delegation_after = get_after.unwrap().unwrap();
                prop_assert_eq!(delegation_after.delegation.status, DelegationStatus::Completed, "Status should be Completed");
                prop_assert!(delegation_after.delegation.result.is_some(), "result should be set after complete");
                prop_assert!(delegation_after.delegation.completed_at.is_some(), "completed_at should be set after complete");
                prop_assert!(delegation_after.delegation.completed_at.unwrap() <= chrono::Utc::now(), "completed_at should be <= now");
                prop_assert_eq!(delegation_after.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Property 2 (edge case): Update non-existent delegation returns false
        ///
        /// *For any* random UUID that was never inserted, updating it SHALL
        /// return Ok(false), not an error.
        ///
        /// **Validates: Requirements 9.3, 9.4**
        #[pg_test]
        fn prop_delegation_update_nonexistent_returns_false() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (any::<[u8; 16]>(), any::<[u8; 16]>(), any::<[u8; 16]>(), arb_delegation_result());

            runner.run(&strategy, |(bytes, agent_bytes, traj_bytes, result)| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                let random_agent_id = uuid::Uuid::from_bytes(agent_bytes);
                let random_traj_id = uuid::Uuid::from_bytes(traj_bytes);

                // Try accept with random IDs
                let tenant_id = caliber_core::new_entity_id();
                let accept_result = delegation_accept_heap(
                    random_id,
                    random_agent_id,
                    random_traj_id,
                    tenant_id,
                );
                prop_assert!(accept_result.is_ok(), "Accept should not error");
                prop_assert!(!accept_result.unwrap(), "Accept of non-existent delegation should return false");

                // Try complete
                let tenant_id = caliber_core::new_entity_id();
                let complete_result = delegation_complete_heap(random_id, &result, tenant_id);
                prop_assert!(complete_result.is_ok(), "Complete should not error");
                prop_assert!(!complete_result.unwrap(), "Complete of non-existent delegation should return false");

                Ok(())
            }).unwrap();
        }

        /// Property 3: Index Consistency - Status Index
        ///
        /// *For any* delegation inserted via direct heap with status "pending",
        /// querying via the status index SHALL return that delegation.
        ///
        /// **Validates: Requirements 9.5, 13.1, 13.2, 13.4, 13.5**
        #[pg_test]
        fn prop_delegation_status_index_consistency() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_agent_type(),
                arb_task_description(),
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_artifact_ids(),
                arb_artifact_ids(),
                arb_optional_context(),
                arb_constraints(),
                arb_optional_deadline(),
            );

            runner.run(&strategy, |(
                delegator_agent_id,
                delegatee_agent_id,
                delegatee_agent_type,
                task_description,
                parent_trajectory_id,
                child_trajectory_id,
                shared_artifacts,
                shared_notes,
                additional_context,
                constraints,
                deadline,
            )| {
                // Generate a new delegation ID
                let delegation_id = caliber_core::new_entity_id();
                let tenant_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = delegation_create_heap(DelegationCreateParams {
                    delegation_id,
                    delegator_agent_id,
                    delegatee_agent_id,
                    delegatee_agent_type: delegatee_agent_type.as_deref(),
                    task_description: &task_description,
                    parent_trajectory_id,
                    child_trajectory_id,
                    shared_artifacts: &shared_artifacts,
                    shared_notes: &shared_notes,
                    additional_context: additional_context.as_deref(),
                    constraints: &constraints,
                    deadline,
                    tenant_id,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Query via status index (should be "pending")
                let list_result = delegation_list_pending_heap(tenant_id);
                prop_assert!(list_result.is_ok(), "List pending should succeed: {:?}", list_result.err());
                
                let delegations = list_result.unwrap();
                prop_assert!(
                    delegations.iter().any(|d| d.delegation.delegation_id == delegation_id),
                    "Inserted delegation should be found via status index"
                );

                // Verify the found delegation has correct data
                let found_delegation = delegations
                    .iter()
                    .find(|d| d.delegation.delegation_id == delegation_id)
                    .unwrap();
                prop_assert_eq!(found_delegation.delegation.delegator_agent_id, delegator_agent_id);
                prop_assert_eq!(&found_delegation.delegation.task_description, &task_description);
                prop_assert_eq!(found_delegation.delegation.status, DelegationStatus::Pending);

                Ok(())
            }).unwrap();
        }
    }
}
