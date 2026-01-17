//! Direct heap operations for Handoff entities.
//!
//! This module provides hot-path operations for agent handoffs that bypass SQL
//! parsing entirely by using direct heap access via pgrx.

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    CaliberError, CaliberResult, EntityId, EntityType, StorageError,
};
use caliber_agents::{AgentHandoff, HandoffStatus, HandoffReason};

use crate::column_maps::handoff;
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
    extract_uuid, extract_text, extract_timestamp, extract_text_array,
    extract_values_and_nulls, uuid_to_datum, string_to_datum,
    timestamp_to_chrono, text_array_to_datum,
};

/// Create a new handoff by inserting a handoff record using direct heap operations.
pub struct HandoffCreateParams<'a> {
    pub handoff_id: EntityId,
    pub from_agent_id: EntityId,
    pub to_agent_id: Option<EntityId>,
    pub to_agent_type: Option<&'a str>,
    pub trajectory_id: EntityId,
    pub scope_id: EntityId,
    pub context_snapshot_id: EntityId,
    pub handoff_notes: &'a str,
    pub next_steps: &'a [String],
    pub blockers: &'a [String],
    pub open_questions: &'a [String],
    pub reason: HandoffReason,
}

pub fn handoff_create_heap(params: HandoffCreateParams<'_>) -> CaliberResult<EntityId> {
    let HandoffCreateParams {
        handoff_id,
        from_agent_id,
        to_agent_id,
        to_agent_type,
        trajectory_id,
        scope_id,
        context_snapshot_id,
        handoff_notes,
        next_steps,
        blockers,
        open_questions,
        reason,
    } = params;
    let rel = open_relation(handoff::TABLE_NAME, HeapLockMode::RowExclusive)?;
    validate_handoff_relation(&rel)?;

    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now).into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Handoff,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    let mut values: [pg_sys::Datum; handoff::NUM_COLS] = [pg_sys::Datum::from(0); handoff::NUM_COLS];
    let mut nulls: [bool; handoff::NUM_COLS] = [false; handoff::NUM_COLS];
    
    // Set required fields
    values[handoff::HANDOFF_ID as usize - 1] = uuid_to_datum(handoff_id);
    values[handoff::FROM_AGENT_ID as usize - 1] = uuid_to_datum(from_agent_id);
    
    // Set optional to_agent_id
    if let Some(to_id) = to_agent_id {
        values[handoff::TO_AGENT_ID as usize - 1] = uuid_to_datum(to_id);
    } else {
        nulls[handoff::TO_AGENT_ID as usize - 1] = true;
    }
    
    // Set optional to_agent_type
    if let Some(to_type) = to_agent_type {
        values[handoff::TO_AGENT_TYPE as usize - 1] = string_to_datum(to_type);
    } else {
        nulls[handoff::TO_AGENT_TYPE as usize - 1] = true;
    }
    
    // Set trajectory_id and scope_id
    values[handoff::TRAJECTORY_ID as usize - 1] = uuid_to_datum(trajectory_id);
    values[handoff::SCOPE_ID as usize - 1] = uuid_to_datum(scope_id);
    
    // Set context_snapshot_id
    values[handoff::CONTEXT_SNAPSHOT_ID as usize - 1] = uuid_to_datum(context_snapshot_id);
    
    // Set handoff_notes
    values[handoff::HANDOFF_NOTES as usize - 1] = string_to_datum(handoff_notes);
    
    // Set next_steps array
    if next_steps.is_empty() {
        nulls[handoff::NEXT_STEPS as usize - 1] = true;
    } else {
        values[handoff::NEXT_STEPS as usize - 1] = text_array_to_datum(next_steps);
    }
    
    // Set blockers array
    if blockers.is_empty() {
        nulls[handoff::BLOCKERS as usize - 1] = true;
    } else {
        values[handoff::BLOCKERS as usize - 1] = text_array_to_datum(blockers);
    }
    
    // Set open_questions array
    if open_questions.is_empty() {
        nulls[handoff::OPEN_QUESTIONS as usize - 1] = true;
    } else {
        values[handoff::OPEN_QUESTIONS as usize - 1] = text_array_to_datum(open_questions);
    }
    
    // Set status - default to "initiated"
    values[handoff::STATUS as usize - 1] = string_to_datum("initiated");
    
    // Set timestamps
    values[handoff::INITIATED_AT as usize - 1] = now_datum;
    nulls[handoff::ACCEPTED_AT as usize - 1] = true;
    nulls[handoff::COMPLETED_AT as usize - 1] = true;
    
    // Set reason
    let reason_str = match reason {
        HandoffReason::CapabilityMismatch => "capability_mismatch",
        HandoffReason::LoadBalancing => "load_balancing",
        HandoffReason::Specialization => "specialization",
        HandoffReason::Escalation => "escalation",
        HandoffReason::Timeout => "timeout",
        HandoffReason::Failure => "failure",
        HandoffReason::Scheduled => "scheduled",
    };
    values[handoff::REASON as usize - 1] = string_to_datum(reason_str);
    
    let tuple = form_tuple(&rel, &values, &nulls)?;
    let _tid = unsafe { insert_tuple(&rel, tuple)? };
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };
    
    Ok(handoff_id)
}

/// Get a handoff by ID using direct heap operations.
pub fn handoff_get_heap(handoff_id: EntityId) -> CaliberResult<Option<AgentHandoff>> {
    let rel = open_relation(handoff::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(handoff::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(handoff_id),
    );
    
    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };
    
    if let Some(tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let handoff = unsafe { tuple_to_handoff(tuple, tuple_desc) }?;
        Ok(Some(handoff))
    } else {
        Ok(None)
    }
}

/// Accept a handoff by updating status, accepted_at, and to_agent_id.
///
/// This function updates the handoff to record:
/// - The agent that accepted the handoff (accepting_agent_id -> stored in to_agent_id)
/// - The acceptance timestamp
/// - Status change to "accepted"
pub fn handoff_accept_heap(
    handoff_id: EntityId,
    accepting_agent_id: EntityId,
) -> CaliberResult<bool> {
    let rel = open_relation(handoff::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(handoff::PK_INDEX)?;
    let snapshot = get_active_snapshot();

    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(handoff_id),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    if let Some(old_tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;

        // Update to_agent_id - the agent accepting the handoff
        values[handoff::TO_AGENT_ID as usize - 1] = uuid_to_datum(accepting_agent_id);
        nulls[handoff::TO_AGENT_ID as usize - 1] = false;

        // Update status to "accepted"
        values[handoff::STATUS as usize - 1] = string_to_datum("accepted");

        // Update accepted_at to current timestamp
        let now = current_timestamp();
        let now_datum = timestamp_to_pgrx(now).into_datum()
            .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Handoff,
                id: handoff_id,
                reason: "Failed to convert timestamp to datum".to_string(),
            }))?;

        values[handoff::ACCEPTED_AT as usize - 1] = now_datum;
        nulls[handoff::ACCEPTED_AT as usize - 1] = false;

        let new_tuple = form_tuple(&rel, &values, &nulls)?;
        let old_tid = scanner.current_tid()
            .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of handoff tuple".to_string(),
            }))?;

        unsafe { update_tuple(&rel, &old_tid, new_tuple)? };
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Complete a handoff by updating status and completed_at using direct heap operations.
pub fn handoff_complete_heap(handoff_id: EntityId) -> CaliberResult<bool> {
    let rel = open_relation(handoff::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(handoff::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(handoff_id),
    );
    
    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };
    
    if let Some(old_tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;
        
        // Update status to "completed"
        values[handoff::STATUS as usize - 1] = string_to_datum("completed");
        
        // Update completed_at to current timestamp
        let now = current_timestamp();
        let now_datum = timestamp_to_pgrx(now).into_datum()
            .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Handoff,
                id: handoff_id,
                reason: "Failed to convert timestamp to datum".to_string(),
            }))?;
        
        values[handoff::COMPLETED_AT as usize - 1] = now_datum;
        nulls[handoff::COMPLETED_AT as usize - 1] = false;
        
        let new_tuple = form_tuple(&rel, &values, &nulls)?;
        let old_tid = scanner.current_tid()
            .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of handoff tuple".to_string(),
            }))?;
        
        unsafe { update_tuple(&rel, &old_tid, new_tuple)? };
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Validate that a HeapRelation is suitable for handoff operations.
fn validate_handoff_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != handoff::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Handoff relation has {} columns, expected {}",
                natts,
                handoff::NUM_COLS
            ),
        }));
    }
    Ok(())
}

unsafe fn tuple_to_handoff(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<AgentHandoff> {
    let handoff_id = extract_uuid(tuple, tuple_desc, handoff::HANDOFF_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "handoff_id is NULL".to_string(),
        }))?;
    
    let from_agent_id = extract_uuid(tuple, tuple_desc, handoff::FROM_AGENT_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "from_agent_id is NULL".to_string(),
        }))?;
    
    let to_agent_id = extract_uuid(tuple, tuple_desc, handoff::TO_AGENT_ID)?;
    let to_agent_type = extract_text(tuple, tuple_desc, handoff::TO_AGENT_TYPE)?;
    
    let trajectory_id = extract_uuid(tuple, tuple_desc, handoff::TRAJECTORY_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "trajectory_id is NULL".to_string(),
        }))?;
    
    let scope_id = extract_uuid(tuple, tuple_desc, handoff::SCOPE_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "scope_id is NULL".to_string(),
        }))?;
    
    let context_snapshot_id = extract_uuid(tuple, tuple_desc, handoff::CONTEXT_SNAPSHOT_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "context_snapshot_id is NULL".to_string(),
        }))?;
    
    let handoff_notes = extract_text(tuple, tuple_desc, handoff::HANDOFF_NOTES)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "handoff_notes is NULL".to_string(),
        }))?;
    
    let next_steps = extract_text_array(tuple, tuple_desc, handoff::NEXT_STEPS)?
        .unwrap_or_default();
    
    let blockers = extract_text_array(tuple, tuple_desc, handoff::BLOCKERS)?
        .unwrap_or_default();
    
    let open_questions = extract_text_array(tuple, tuple_desc, handoff::OPEN_QUESTIONS)?
        .unwrap_or_default();
    
    let status_str = extract_text(tuple, tuple_desc, handoff::STATUS)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "status is NULL".to_string(),
        }))?;
    let status = match status_str.as_str() {
        "initiated" => HandoffStatus::Initiated,
        "accepted" => HandoffStatus::Accepted,
        "completed" => HandoffStatus::Completed,
        "rejected" => HandoffStatus::Rejected,
        _ => {
            pgrx::warning!("CALIBER: Unknown handoff status '{}', defaulting to Initiated", status_str);
            HandoffStatus::Initiated
        }
    };
    
    let initiated_at_ts = extract_timestamp(tuple, tuple_desc, handoff::INITIATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "initiated_at is NULL".to_string(),
        }))?;
    let initiated_at = timestamp_to_chrono(initiated_at_ts);
    
    let accepted_at = extract_timestamp(tuple, tuple_desc, handoff::ACCEPTED_AT)?
        .map(timestamp_to_chrono);
    
    let completed_at = extract_timestamp(tuple, tuple_desc, handoff::COMPLETED_AT)?
        .map(timestamp_to_chrono);
    
    let reason_str = extract_text(tuple, tuple_desc, handoff::REASON)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "reason is NULL".to_string(),
        }))?;
    let reason = match reason_str.as_str() {
        "capability_mismatch" => HandoffReason::CapabilityMismatch,
        "load_balancing" => HandoffReason::LoadBalancing,
        "specialization" => HandoffReason::Specialization,
        "escalation" => HandoffReason::Escalation,
        "timeout" => HandoffReason::Timeout,
        "failure" => HandoffReason::Failure,
        "scheduled" => HandoffReason::Scheduled,
        _ => {
            pgrx::warning!("CALIBER: Unknown handoff reason '{}', defaulting to Scheduled", reason_str);
            HandoffReason::Scheduled
        }
    };
    
    Ok(AgentHandoff {
        handoff_id,
        from_agent_id,
        to_agent_id,
        to_agent_type,
        trajectory_id,
        scope_id,
        context_snapshot_id,
        handoff_notes,
        next_steps,
        blockers,
        open_questions,
        status,
        initiated_at,
        accepted_at,
        completed_at,
        reason,
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
    // Test Helpers - Generators for Handoff data
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
                Just(Some("reviewer".to_string())),
            ],
        ]
    }

    /// Generate handoff notes
    fn arb_handoff_notes() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Handing off due to capability mismatch".to_string()),
            Just("Load balancing - agent overloaded".to_string()),
            Just("Requires specialized knowledge".to_string()),
            Just("Escalating to supervisor".to_string()),
            Just("Task timeout - reassigning".to_string()),
        ]
    }

    /// Generate a vector of next steps (0-5 items)
    fn arb_next_steps() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop_oneof![
                Just("Review context snapshot".to_string()),
                Just("Continue from checkpoint".to_string()),
                Just("Validate assumptions".to_string()),
                Just("Execute remaining tasks".to_string()),
                Just("Report back to delegator".to_string()),
            ],
            0..=5
        )
    }

    /// Generate a vector of blockers (0-3 items)
    fn arb_blockers() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop_oneof![
                Just("Missing required data".to_string()),
                Just("Dependency not resolved".to_string()),
                Just("Resource unavailable".to_string()),
            ],
            0..=3
        )
    }

    /// Generate a vector of open questions (0-4 items)
    fn arb_open_questions() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop_oneof![
                Just("What is the expected output format?".to_string()),
                Just("Should we prioritize speed or accuracy?".to_string()),
                Just("Are there any constraints?".to_string()),
                Just("Who should review the results?".to_string()),
            ],
            0..=4
        )
    }

    /// Generate a handoff reason
    fn arb_handoff_reason() -> impl Strategy<Value = HandoffReason> {
        prop_oneof![
            Just(HandoffReason::CapabilityMismatch),
            Just(HandoffReason::LoadBalancing),
            Just(HandoffReason::Specialization),
            Just(HandoffReason::Escalation),
            Just(HandoffReason::Timeout),
            Just(HandoffReason::Failure),
            Just(HandoffReason::Scheduled),
        ]
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Handoff)
    // Feature: caliber-pg-hot-path, Property 1: Insert-Get Round Trip
    // Validates: Requirements 10.1, 10.2
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use pgrx_tests::pg_test;

        /// Property 1: Insert-Get Round Trip (Handoff)
        /// 
        /// *For any* valid handoff data, inserting via direct heap then getting
        /// via direct heap SHALL return an equivalent handoff.
        ///
        /// **Validates: Requirements 10.1, 10.2**
        #[pg_test]
        fn prop_handoff_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_agent_type(),
                arb_entity_id(),
                arb_entity_id(),
                arb_entity_id(),
                arb_handoff_notes(),
                arb_next_steps(),
                arb_blockers(),
                arb_open_questions(),
                arb_handoff_reason(),
            );

            runner.run(&strategy, |(
                from_agent_id,
                to_agent_id,
                to_agent_type,
                trajectory_id,
                scope_id,
                context_snapshot_id,
                handoff_notes,
                next_steps,
                blockers,
                open_questions,
                reason,
            )| {
                // Generate a new handoff ID
                let handoff_id = caliber_core::new_entity_id();

                // Insert via heap
                let result = handoff_create_heap(HandoffCreateParams {
                    handoff_id,
                    from_agent_id,
                    to_agent_id,
                    to_agent_type: to_agent_type.as_deref(),
                    trajectory_id,
                    scope_id,
                    context_snapshot_id,
                    handoff_notes: &handoff_notes,
                    next_steps: &next_steps,
                    blockers: &blockers,
                    open_questions: &open_questions,
                    reason,
                });
                prop_assert!(result.is_ok(), "Insert should succeed: {:?}", result.err());
                prop_assert_eq!(result.unwrap(), handoff_id);

                // Get via heap
                let get_result = handoff_get_heap(handoff_id);
                prop_assert!(get_result.is_ok(), "Get should succeed: {:?}", get_result.err());
                
                let handoff = get_result.unwrap();
                prop_assert!(handoff.is_some(), "Handoff should be found");
                
                let h = handoff.unwrap();
                
                // Verify round-trip preserves data
                prop_assert_eq!(h.handoff_id, handoff_id);
                prop_assert_eq!(h.from_agent_id, from_agent_id);
                prop_assert_eq!(h.to_agent_id, to_agent_id);
                prop_assert_eq!(h.to_agent_type, to_agent_type);
                prop_assert_eq!(h.trajectory_id, trajectory_id);
                prop_assert_eq!(h.scope_id, scope_id);
                prop_assert_eq!(h.context_snapshot_id, context_snapshot_id);
                prop_assert_eq!(h.handoff_notes, handoff_notes);
                prop_assert_eq!(h.next_steps, next_steps);
                prop_assert_eq!(h.blockers, blockers);
                prop_assert_eq!(h.open_questions, open_questions);
                prop_assert_eq!(h.status, HandoffStatus::Initiated);
                prop_assert_eq!(h.reason, reason);
                
                // Timestamps should be set
                prop_assert!(h.initiated_at <= chrono::Utc::now());
                prop_assert!(h.accepted_at.is_none(), "accepted_at should be None initially");
                prop_assert!(h.completed_at.is_none(), "completed_at should be None initially");

                Ok(())
            }).unwrap();
        }

        /// Property 1 (edge case): Get non-existent handoff returns None
        ///
        /// *For any* random UUID that was never inserted, getting it SHALL
        /// return Ok(None), not an error.
        ///
        /// **Validates: Requirements 10.2**
        #[pg_test]
        fn prop_handoff_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let result = handoff_get_heap(random_id);
                prop_assert!(result.is_ok(), "Get should not error: {:?}", result.err());
                prop_assert!(result.unwrap().is_none(), "Non-existent handoff should return None");

                Ok(())
            }).unwrap();
        }

        /// Property 2: Update Persistence (Handoff - accept)
        ///
        /// *For any* handoff that has been inserted, accepting it SHALL
        /// update the status and accepted_at fields and persist the change.
        ///
        /// **Validates: Requirements 10.3**
        #[pg_test]
        fn prop_handoff_accept_persistence() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_agent_type(),
                arb_entity_id(),
                arb_entity_id(),
                arb_entity_id(),
                arb_handoff_notes(),
                arb_next_steps(),
                arb_blockers(),
                arb_open_questions(),
                arb_handoff_reason(),
            );

            runner.run(&strategy, |(
                from_agent_id,
                to_agent_id,
                to_agent_type,
                trajectory_id,
                scope_id,
                context_snapshot_id,
                handoff_notes,
                next_steps,
                blockers,
                open_questions,
                reason,
            )| {
                // Generate a new handoff ID
                let handoff_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = handoff_create_heap(HandoffCreateParams {
                    handoff_id,
                    from_agent_id,
                    to_agent_id,
                    to_agent_type: to_agent_type.as_deref(),
                    trajectory_id,
                    scope_id,
                    context_snapshot_id,
                    handoff_notes: &handoff_notes,
                    next_steps: &next_steps,
                    blockers: &blockers,
                    open_questions: &open_questions,
                    reason,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Verify initial status is Initiated
                let get_before = handoff_get_heap(handoff_id);
                prop_assert!(get_before.is_ok(), "Get before accept should succeed");
                let handoff_before = get_before.unwrap().unwrap();
                prop_assert_eq!(handoff_before.status, HandoffStatus::Initiated);
                prop_assert!(handoff_before.accepted_at.is_none(), "accepted_at should be None before accept");

                // Generate accepting agent ID
                let accepting_agent = caliber_core::new_entity_id();

                // Accept the handoff with accepting agent
                let accept_result = handoff_accept_heap(handoff_id, accepting_agent);
                prop_assert!(accept_result.is_ok(), "Accept should succeed: {:?}", accept_result.err());
                prop_assert!(accept_result.unwrap(), "Accept should return true for existing handoff");

                // Verify status, accepted_at, and to_agent_id were updated
                let get_after = handoff_get_heap(handoff_id);
                prop_assert!(get_after.is_ok(), "Get after accept should succeed");
                let handoff_after = get_after.unwrap().unwrap();
                prop_assert_eq!(handoff_after.status, HandoffStatus::Accepted, "Status should be Accepted");
                prop_assert!(handoff_after.accepted_at.is_some(), "accepted_at should be set after accept");
                prop_assert!(handoff_after.accepted_at.unwrap() <= chrono::Utc::now(), "accepted_at should be <= now");
                prop_assert_eq!(handoff_after.to_agent_id, Some(accepting_agent), "to_agent_id should be set to accepting agent");

                Ok(())
            }).unwrap();
        }

        /// Property 2: Update Persistence (Handoff - complete)
        ///
        /// *For any* handoff that has been inserted, completing it SHALL
        /// update the status and completed_at fields and persist the change.
        ///
        /// **Validates: Requirements 10.4**
        #[pg_test]
        fn prop_handoff_complete_persistence() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_entity_id(),
                arb_optional_entity_id(),
                arb_optional_agent_type(),
                arb_entity_id(),
                arb_entity_id(),
                arb_entity_id(),
                arb_handoff_notes(),
                arb_next_steps(),
                arb_blockers(),
                arb_open_questions(),
                arb_handoff_reason(),
            );

            runner.run(&strategy, |(
                from_agent_id,
                to_agent_id,
                to_agent_type,
                trajectory_id,
                scope_id,
                context_snapshot_id,
                handoff_notes,
                next_steps,
                blockers,
                open_questions,
                reason,
            )| {
                // Generate a new handoff ID
                let handoff_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = handoff_create_heap(HandoffCreateParams {
                    handoff_id,
                    from_agent_id,
                    to_agent_id,
                    to_agent_type: to_agent_type.as_deref(),
                    trajectory_id,
                    scope_id,
                    context_snapshot_id,
                    handoff_notes: &handoff_notes,
                    next_steps: &next_steps,
                    blockers: &blockers,
                    open_questions: &open_questions,
                    reason,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Verify initial state
                let get_before = handoff_get_heap(handoff_id);
                prop_assert!(get_before.is_ok(), "Get before complete should succeed");
                let handoff_before = get_before.unwrap().unwrap();
                prop_assert_eq!(handoff_before.status, HandoffStatus::Initiated);
                prop_assert!(handoff_before.completed_at.is_none(), "completed_at should be None before complete");

                // Complete the handoff
                let complete_result = handoff_complete_heap(handoff_id);
                prop_assert!(complete_result.is_ok(), "Complete should succeed: {:?}", complete_result.err());
                prop_assert!(complete_result.unwrap(), "Complete should return true for existing handoff");

                // Verify status and completed_at were updated
                let get_after = handoff_get_heap(handoff_id);
                prop_assert!(get_after.is_ok(), "Get after complete should succeed");
                let handoff_after = get_after.unwrap().unwrap();
                prop_assert_eq!(handoff_after.status, HandoffStatus::Completed, "Status should be Completed");
                prop_assert!(handoff_after.completed_at.is_some(), "completed_at should be set after complete");
                prop_assert!(handoff_after.completed_at.unwrap() <= chrono::Utc::now(), "completed_at should be <= now");

                Ok(())
            }).unwrap();
        }

        /// Property 2 (edge case): Update non-existent handoff returns false
        ///
        /// *For any* random UUID that was never inserted, updating it SHALL
        /// return Ok(false), not an error.
        ///
        /// **Validates: Requirements 10.3, 10.4**
        #[pg_test]
        fn prop_handoff_update_nonexistent_returns_false() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&(any::<[u8; 16]>(), any::<[u8; 16]>()), |(bytes, agent_bytes)| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                let random_agent_id = uuid::Uuid::from_bytes(agent_bytes);

                // Try accept with random accepting agent
                let accept_result = handoff_accept_heap(random_id, random_agent_id);
                prop_assert!(accept_result.is_ok(), "Accept should not error");
                prop_assert!(!accept_result.unwrap(), "Accept of non-existent handoff should return false");

                // Try complete
                let complete_result = handoff_complete_heap(random_id);
                prop_assert!(complete_result.is_ok(), "Complete should not error");
                prop_assert!(!complete_result.unwrap(), "Complete of non-existent handoff should return false");

                Ok(())
            }).unwrap();
        }

        /// Property: All handoff fields properly preserved in round-trip
        ///
        /// *For any* handoff with all optional fields populated, the round-trip
        /// SHALL preserve all field values exactly.
        ///
        /// **Validates: Requirements 10.1, 10.2**
        #[pg_test]
        fn prop_handoff_all_fields_preserved() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_entity_id(),
                arb_entity_id(), // to_agent_id (always Some)
                arb_optional_agent_type(),
                arb_entity_id(),
                arb_entity_id(),
                arb_entity_id(),
                arb_handoff_notes(),
                arb_next_steps(),
                arb_blockers(),
                arb_open_questions(),
                arb_handoff_reason(),
            );

            runner.run(&strategy, |(
                from_agent_id,
                to_agent_id,
                to_agent_type,
                trajectory_id,
                scope_id,
                context_snapshot_id,
                handoff_notes,
                next_steps,
                blockers,
                open_questions,
                reason,
            )| {
                // Generate a new handoff ID
                let handoff_id = caliber_core::new_entity_id();

                // Insert with all fields populated
                let result = handoff_create_heap(HandoffCreateParams {
                    handoff_id,
                    from_agent_id,
                    to_agent_id: Some(to_agent_id),
                    to_agent_type: to_agent_type.as_deref(),
                    trajectory_id,
                    scope_id,
                    context_snapshot_id,
                    handoff_notes: &handoff_notes,
                    next_steps: &next_steps,
                    blockers: &blockers,
                    open_questions: &open_questions,
                    reason,
                });
                prop_assert!(result.is_ok(), "Insert should succeed");

                // Get and verify all fields
                let get_result = handoff_get_heap(handoff_id);
                prop_assert!(get_result.is_ok(), "Get should succeed");
                
                let h = get_result.unwrap().unwrap();
                
                // Verify all fields are preserved
                prop_assert_eq!(h.handoff_id, handoff_id);
                prop_assert_eq!(h.from_agent_id, from_agent_id);
                prop_assert_eq!(h.to_agent_id, Some(to_agent_id));
                prop_assert_eq!(h.to_agent_type, to_agent_type);
                prop_assert_eq!(h.trajectory_id, trajectory_id);
                prop_assert_eq!(h.scope_id, scope_id);
                prop_assert_eq!(h.context_snapshot_id, context_snapshot_id);
                prop_assert_eq!(h.handoff_notes, handoff_notes);
                prop_assert_eq!(h.next_steps, next_steps);
                prop_assert_eq!(h.blockers, blockers);
                prop_assert_eq!(h.open_questions, open_questions);
                prop_assert_eq!(h.reason, reason);

                Ok(())
            }).unwrap();
        }
    }
}
