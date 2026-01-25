//! Direct heap operations for Message entities.
//!
//! This module provides hot-path operations for inter-agent messages that bypass SQL
//! parsing entirely by using direct heap access via pgrx.

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    AgentId, AgentMessage, ArtifactId, CaliberError, CaliberResult, EntityIdType, EntityType,
    MessageId, MessagePriority, MessageType, ScopeId, StorageError, TenantId, TrajectoryId,
};

use crate::column_maps::message;
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
    extract_uuid, extract_text, extract_timestamp, extract_uuid_array,
    extract_values_and_nulls, uuid_to_datum, string_to_datum,
    timestamp_to_chrono, uuid_array_to_datum, option_uuid_to_datum,
    option_string_to_datum, option_datetime_to_datum,
};

/// Message row with tenant ownership metadata.
pub struct MessageRow {
    pub message: AgentMessage,
    pub tenant_id: Option<TenantId>,
}

impl From<MessageRow> for AgentMessage {
    fn from(row: MessageRow) -> Self {
        row.message
    }
}

type OptionalMessageDatums = (
    pg_sys::Datum,
    pg_sys::Datum,
    pg_sys::Datum,
    pg_sys::Datum,
    pg_sys::Datum,
    bool,
    bool,
    bool,
    bool,
    bool,
);

/// Send a message by inserting a message record using direct heap operations.
pub struct MessageSendParams<'a> {
    pub message_id: MessageId,
    pub from_agent_id: AgentId,
    pub to_agent_id: Option<AgentId>,
    pub to_agent_type: Option<&'a str>,
    pub message_type: MessageType,
    pub payload: &'a str,
    pub trajectory_id: Option<TrajectoryId>,
    pub scope_id: Option<ScopeId>,
    pub artifact_ids: &'a [ArtifactId],
    pub priority: MessagePriority,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tenant_id: TenantId,
}

pub fn message_send_heap(params: MessageSendParams<'_>) -> CaliberResult<MessageId> {
    let MessageSendParams {
        message_id,
        from_agent_id,
        to_agent_id,
        to_agent_type,
        message_type,
        payload,
        trajectory_id,
        scope_id,
        artifact_ids,
        priority,
        expires_at,
        tenant_id,
    } = params;
    let rel = open_relation(message::TABLE_NAME, HeapLockMode::RowExclusive)?;
    validate_message_relation(&rel)?;

    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now)?.into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Message,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    let mut values: [pg_sys::Datum; message::NUM_COLS] = [pg_sys::Datum::from(0); message::NUM_COLS];
    let mut nulls: [bool; message::NUM_COLS] = [false; message::NUM_COLS];

    // Use helper for optional fields
    let (to_agent_datum, to_type_datum, traj_datum, scope_datum, expires_datum,
         to_agent_null, to_type_null, traj_null, scope_null, expires_null) =
        build_optional_message_datums(to_agent_id, to_agent_type, trajectory_id, scope_id, expires_at)?;

    // Set required fields
    values[message::MESSAGE_ID as usize - 1] = uuid_to_datum(message_id.as_uuid());
    values[message::FROM_AGENT_ID as usize - 1] = uuid_to_datum(from_agent_id.as_uuid());

    // Set optional to_agent_id
    values[message::TO_AGENT_ID as usize - 1] = to_agent_datum;
    nulls[message::TO_AGENT_ID as usize - 1] = to_agent_null;

    // Set optional to_agent_type
    values[message::TO_AGENT_TYPE as usize - 1] = to_type_datum;
    nulls[message::TO_AGENT_TYPE as usize - 1] = to_type_null;

    // Set message_type
    let message_type_str = match message_type {
        MessageType::TaskDelegation => "task_delegation",
        MessageType::TaskResult => "task_result",
        MessageType::ContextRequest => "context_request",
        MessageType::ContextShare => "context_share",
        MessageType::CoordinationSignal => "coordination_signal",
        MessageType::Handoff => "handoff",
        MessageType::Interrupt => "interrupt",
        MessageType::Heartbeat => "heartbeat",
    };
    values[message::MESSAGE_TYPE as usize - 1] = string_to_datum(message_type_str);

    // Set payload
    values[message::PAYLOAD as usize - 1] = string_to_datum(payload);

    // Set optional trajectory_id
    values[message::TRAJECTORY_ID as usize - 1] = traj_datum;
    nulls[message::TRAJECTORY_ID as usize - 1] = traj_null;

    // Set optional scope_id
    values[message::SCOPE_ID as usize - 1] = scope_datum;
    nulls[message::SCOPE_ID as usize - 1] = scope_null;

    // Set artifact_ids array
    if artifact_ids.is_empty() {
        nulls[message::ARTIFACT_IDS as usize - 1] = true;
    } else {
        let artifact_uuids: Vec<uuid::Uuid> = artifact_ids.iter().map(|id| id.as_uuid()).collect();
        values[message::ARTIFACT_IDS as usize - 1] = uuid_array_to_datum(&artifact_uuids);
    }

    // Set timestamps
    values[message::CREATED_AT as usize - 1] = now_datum;
    nulls[message::DELIVERED_AT as usize - 1] = true;
    nulls[message::ACKNOWLEDGED_AT as usize - 1] = true;

    // Set priority
    let priority_str = match priority {
        MessagePriority::Low => "low",
        MessagePriority::Normal => "normal",
        MessagePriority::High => "high",
        MessagePriority::Critical => "critical",
    };
    values[message::PRIORITY as usize - 1] = string_to_datum(priority_str);

    // Set optional expires_at
    values[message::EXPIRES_AT as usize - 1] = expires_datum;
    nulls[message::EXPIRES_AT as usize - 1] = expires_null;

    // Set tenant_id
    values[message::TENANT_ID as usize - 1] = uuid_to_datum(tenant_id.as_uuid());
    
    let tuple = form_tuple(&rel, &values, &nulls)?;
    let _tid = unsafe { insert_tuple(&rel, tuple)? };
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };
    
    Ok(message_id)
}

/// Get a message by ID using direct heap operations.
pub fn message_get_heap(message_id: MessageId, tenant_id: TenantId) -> CaliberResult<Option<MessageRow>> {
    let rel = open_relation(message::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(message::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(message_id.as_uuid()),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    if let Some(tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let row = unsafe { tuple_to_message(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid()) {
            Ok(Some(row))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// List messages for a specific agent using direct heap operations.
pub fn message_list_for_agent_heap(
    to_agent_id: AgentId,
    tenant_id: TenantId,
) -> CaliberResult<Vec<MessageRow>> {
    let rel = open_relation(message::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(message::TO_AGENT_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(to_agent_id.as_uuid()),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();

    for tuple in &mut scanner {
        let row = unsafe { tuple_to_message(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid()) {
            results.push(row);
        }
    }
    
    Ok(results)
}

/// Acknowledge a message by updating its acknowledged_at field using direct heap operations.
pub fn message_acknowledge_heap(message_id: MessageId, tenant_id: TenantId) -> CaliberResult<bool> {
    let rel = open_relation(message::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(message::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(message_id.as_uuid()),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    if let Some(old_tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, message::TENANT_ID)? };
        if existing_tenant != Some(tenant_id.as_uuid()) {
            return Ok(false);
        }
        let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;
        
        // Update acknowledged_at to current timestamp
        let now = current_timestamp();
        let now_datum = timestamp_to_pgrx(now)?.into_datum()
            .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Message,
                id: message_id.as_uuid(),
                reason: "Failed to convert timestamp to datum".to_string(),
            }))?;
        
        values[message::ACKNOWLEDGED_AT as usize - 1] = now_datum;
        nulls[message::ACKNOWLEDGED_AT as usize - 1] = false;
        
        let new_tuple = form_tuple(&rel, &values, &nulls)?;
        let old_tid = scanner.current_tid()
            .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of message tuple".to_string(),
            }))?;
        
        unsafe { update_tuple(&rel, &old_tid, new_tuple)? };
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Validate that a HeapRelation is suitable for message operations.
fn validate_message_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != message::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Message relation has {} columns, expected {}",
                natts,
                message::NUM_COLS
            ),
        }));
    }
    Ok(())
}

/// Build optional datum values for message fields using proper option_* helpers.
fn build_optional_message_datums(
    to_agent_id: Option<AgentId>,
    to_agent_type: Option<&str>,
    trajectory_id: Option<TrajectoryId>,
    scope_id: Option<ScopeId>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> CaliberResult<OptionalMessageDatums> {
    let (to_agent_datum, to_agent_null) = match to_agent_id {
        Some(id) => (option_uuid_to_datum(Some(id.as_uuid())), false),
        None => (pg_sys::Datum::from(0), true),
    };

    let (to_type_datum, to_type_null) = match to_agent_type {
        Some(t) => (option_string_to_datum(Some(t)), false),
        None => (pg_sys::Datum::from(0), true),
    };

    let (traj_datum, traj_null) = match trajectory_id {
        Some(id) => (option_uuid_to_datum(Some(id.as_uuid())), false),
        None => (pg_sys::Datum::from(0), true),
    };

    let (scope_datum, scope_null) = match scope_id {
        Some(id) => (option_uuid_to_datum(Some(id.as_uuid())), false),
        None => (pg_sys::Datum::from(0), true),
    };

    let (expires_datum, expires_null) = match expires_at {
        Some(dt) => (option_datetime_to_datum(Some(dt))?, false),
        None => (pg_sys::Datum::from(0), true),
    };

    Ok((
        to_agent_datum,
        to_type_datum,
        traj_datum,
        scope_datum,
        expires_datum,
        to_agent_null,
        to_type_null,
        traj_null,
        scope_null,
        expires_null,
    ))
}

unsafe fn tuple_to_message(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<MessageRow> {
    let message_id = extract_uuid(tuple, tuple_desc, message::MESSAGE_ID)?
        .map(MessageId::new)
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "message_id is NULL".to_string(),
        }))?;

    let from_agent_id = extract_uuid(tuple, tuple_desc, message::FROM_AGENT_ID)?
        .map(AgentId::new)
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "from_agent_id is NULL".to_string(),
        }))?;

    let to_agent_id = extract_uuid(tuple, tuple_desc, message::TO_AGENT_ID)?.map(AgentId::new);
    let to_agent_type = extract_text(tuple, tuple_desc, message::TO_AGENT_TYPE)?;
    
    let message_type_str = extract_text(tuple, tuple_desc, message::MESSAGE_TYPE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "message_type is NULL".to_string(),
        }))?;
    let message_type = match message_type_str.as_str() {
        "task_delegation" => MessageType::TaskDelegation,
        "task_result" => MessageType::TaskResult,
        "context_request" => MessageType::ContextRequest,
        "context_share" => MessageType::ContextShare,
        "coordination_signal" => MessageType::CoordinationSignal,
        "handoff" => MessageType::Handoff,
        "interrupt" => MessageType::Interrupt,
        "heartbeat" => MessageType::Heartbeat,
        _ => {
            pgrx::warning!("CALIBER: Unknown message type '{}', defaulting to CoordinationSignal", message_type_str);
            MessageType::CoordinationSignal
        }
    };
    
    let payload = extract_text(tuple, tuple_desc, message::PAYLOAD)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "payload is NULL".to_string(),
        }))?;
    
    let trajectory_id = extract_uuid(tuple, tuple_desc, message::TRAJECTORY_ID)?.map(TrajectoryId::new);
    let scope_id = extract_uuid(tuple, tuple_desc, message::SCOPE_ID)?.map(ScopeId::new);

    let artifact_ids = extract_uuid_array(tuple, tuple_desc, message::ARTIFACT_IDS)?
        .map(|uuids| uuids.into_iter().map(ArtifactId::new).collect())
        .unwrap_or_default();
    
    let created_at_ts = extract_timestamp(tuple, tuple_desc, message::CREATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "created_at is NULL".to_string(),
        }))?;
    let created_at = timestamp_to_chrono(created_at_ts);
    
    let delivered_at = extract_timestamp(tuple, tuple_desc, message::DELIVERED_AT)?
        .map(timestamp_to_chrono);
    
    let acknowledged_at = extract_timestamp(tuple, tuple_desc, message::ACKNOWLEDGED_AT)?
        .map(timestamp_to_chrono);
    
    let priority_str = extract_text(tuple, tuple_desc, message::PRIORITY)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "priority is NULL".to_string(),
        }))?;
    let priority = match priority_str.as_str() {
        "low" => MessagePriority::Low,
        "normal" => MessagePriority::Normal,
        "high" => MessagePriority::High,
        "critical" => MessagePriority::Critical,
        _ => {
            pgrx::warning!("CALIBER: Unknown message priority '{}', defaulting to Normal", priority_str);
            MessagePriority::Normal
        }
    };
    
    let expires_at = extract_timestamp(tuple, tuple_desc, message::EXPIRES_AT)?
        .map(timestamp_to_chrono);

    let tenant_id = extract_uuid(tuple, tuple_desc, message::TENANT_ID)?.map(TenantId::new);
    
    Ok(MessageRow {
        message: AgentMessage {
            message_id,
            from_agent_id,
            to_agent_id,
            to_agent_type,
            message_type,
            payload,
            trajectory_id,
            scope_id,
            artifact_ids,
            created_at,
            delivered_at,
            acknowledged_at,
            priority,
            expires_at,
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
    // Test Helpers - Generators for Message data
    // ========================================================================

    /// Generate a random AgentId
    fn arb_agent_id() -> impl Strategy<Value = AgentId> {
        any::<[u8; 16]>().prop_map(|bytes| AgentId::new(uuid::Uuid::from_bytes(bytes)))
    }

    /// Generate an optional AgentId
    fn arb_optional_agent_id() -> impl Strategy<Value = Option<AgentId>> {
        prop_oneof![
            1 => Just(None),
            3 => arb_agent_id().prop_map(Some),
        ]
    }

    /// Generate an optional TrajectoryId
    fn arb_optional_trajectory_id() -> impl Strategy<Value = Option<TrajectoryId>> {
        prop_oneof![
            1 => Just(None),
            3 => any::<[u8; 16]>().prop_map(|bytes| Some(TrajectoryId::new(uuid::Uuid::from_bytes(bytes)))),
        ]
    }

    /// Generate an optional ScopeId
    fn arb_optional_scope_id() -> impl Strategy<Value = Option<ScopeId>> {
        prop_oneof![
            1 => Just(None),
            3 => any::<[u8; 16]>().prop_map(|bytes| Some(ScopeId::new(uuid::Uuid::from_bytes(bytes)))),
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

    /// Generate a message type
    fn arb_message_type() -> impl Strategy<Value = MessageType> {
        prop_oneof![
            Just(MessageType::TaskDelegation),
            Just(MessageType::TaskResult),
            Just(MessageType::ContextRequest),
            Just(MessageType::ContextShare),
            Just(MessageType::CoordinationSignal),
            Just(MessageType::Handoff),
            Just(MessageType::Interrupt),
            Just(MessageType::Heartbeat),
        ]
    }

    /// Generate a payload string
    fn arb_payload() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("{}".to_string()),
            Just(r#"{"task":"process_data"}"#.to_string()),
            Just(r#"{"status":"completed","result":"success"}"#.to_string()),
            Just(r#"{"request":"context","scope_id":"abc123"}"#.to_string()),
        ]
    }

    /// Generate a message priority
    fn arb_priority() -> impl Strategy<Value = MessagePriority> {
        prop_oneof![
            Just(MessagePriority::Low),
            Just(MessagePriority::Normal),
            Just(MessagePriority::High),
            Just(MessagePriority::Critical),
        ]
    }

    /// Generate an optional future expiration time
    fn arb_optional_expires_at() -> impl Strategy<Value = Option<chrono::DateTime<chrono::Utc>>> {
        prop_oneof![
            2 => Just(None),
            1 => (60i64..3600i64).prop_map(|seconds| {
                Some(chrono::Utc::now() + Duration::seconds(seconds))
            }),
        ]
    }

    /// Generate a vector of artifact IDs (0-5 items)
    fn arb_artifact_ids() -> impl Strategy<Value = Vec<ArtifactId>> {
        prop::collection::vec(
            any::<[u8; 16]>().prop_map(|bytes| ArtifactId::new(uuid::Uuid::from_bytes(bytes))),
            0..=5
        )
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Message)
    // Feature: caliber-pg-hot-path, Property 1: Insert-Get Round Trip
    // Validates: Requirements 7.1, 7.2
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use crate::pg_test;

        /// Property 1: Insert-Get Round Trip (Message)
        /// 
        /// *For any* valid message data, inserting via direct heap then getting
        /// via direct heap SHALL return an equivalent message.
        ///
        /// **Validates: Requirements 7.1, 7.2**
        #[pg_test]
        fn prop_message_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_agent_id(),
                arb_agent_id(),
                arb_optional_agent_id(),
                arb_optional_agent_type(),
                arb_message_type(),
                arb_payload(),
                arb_optional_trajectory_id(),
                arb_optional_scope_id(),
                arb_artifact_ids(),
                arb_priority(),
                arb_optional_expires_at(),
            );

            runner.run(&strategy, |(
                from_agent_id,
                _to_agent_id_val,
                to_agent_id,
                to_agent_type,
                message_type,
                payload,
                trajectory_id,
                scope_id,
                artifact_ids,
                priority,
                expires_at,
            )| {
                // Generate a new message ID
                let message_id = MessageId::now_v7();
                let tenant_id = TenantId::now_v7();

                // Insert via heap
                let result = message_send_heap(MessageSendParams {
                    message_id,
                    from_agent_id,
                    to_agent_id,
                    to_agent_type: to_agent_type.as_deref(),
                    message_type,
                    payload: &payload,
                    trajectory_id,
                    scope_id,
                    artifact_ids: &artifact_ids,
                    priority,
                    expires_at,
                    tenant_id,
                });
                prop_assert!(result.is_ok(), "Insert should succeed: {:?}", result.err());
                prop_assert_eq!(result.unwrap(), message_id);

                // Get via heap
                let get_result = message_get_heap(message_id, tenant_id);
                prop_assert!(get_result.is_ok(), "Get should succeed: {:?}", get_result.err());
                
                let message = get_result.unwrap();
                prop_assert!(message.is_some(), "Message should be found");
                
                let row = message.unwrap();
                let m = row.message;
                
                // Verify round-trip preserves data
                prop_assert_eq!(m.message_id, message_id);
                prop_assert_eq!(m.from_agent_id, from_agent_id);
                prop_assert_eq!(m.to_agent_id, to_agent_id);
                prop_assert_eq!(m.to_agent_type, to_agent_type);
                prop_assert_eq!(m.message_type, message_type);
                prop_assert_eq!(m.payload, payload);
                prop_assert_eq!(m.trajectory_id, trajectory_id);
                prop_assert_eq!(m.scope_id, scope_id);
                prop_assert_eq!(m.artifact_ids, artifact_ids);
                prop_assert_eq!(m.priority, priority);
                
                // Timestamps should be set
                prop_assert!(m.created_at <= chrono::Utc::now());
                prop_assert!(m.delivered_at.is_none(), "delivered_at should be None initially");
                prop_assert!(m.acknowledged_at.is_none(), "acknowledged_at should be None initially");
                prop_assert_eq!(row.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Property 1 (edge case): Get non-existent message returns None
        ///
        /// *For any* random UUID that was never inserted, getting it SHALL
        /// return Ok(None), not an error.
        ///
        /// **Validates: Requirements 7.2**
        #[pg_test]
        fn prop_message_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = MessageId::new(uuid::Uuid::from_bytes(bytes));

                let tenant_id = TenantId::now_v7();
                let result = message_get_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Get should not error: {:?}", result.err());
                prop_assert!(result.unwrap().is_none(), "Non-existent message should return None");

                Ok(())
            }).unwrap();
        }

        /// Property 2: Update Persistence (Message - acknowledge)
        ///
        /// *For any* message that has been inserted, acknowledging it SHALL
        /// update the acknowledged_at field and persist the change.
        ///
        /// **Validates: Requirements 7.4**
        #[pg_test]
        fn prop_message_acknowledge_persistence() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_agent_id(),
                arb_agent_id(),
                arb_optional_agent_id(),
                arb_optional_agent_type(),
                arb_message_type(),
                arb_payload(),
                arb_optional_trajectory_id(),
                arb_optional_scope_id(),
                arb_artifact_ids(),
                arb_priority(),
                arb_optional_expires_at(),
            );

            runner.run(&strategy, |(
                from_agent_id,
                _to_agent_id_val,
                to_agent_id,
                to_agent_type,
                message_type,
                payload,
                trajectory_id,
                scope_id,
                artifact_ids,
                priority,
                expires_at,
            )| {
                // Generate a new message ID
                let message_id = MessageId::now_v7();
                let tenant_id = TenantId::now_v7();

                // Insert via heap
                let insert_result = message_send_heap(MessageSendParams {
                    message_id,
                    from_agent_id,
                    to_agent_id,
                    to_agent_type: to_agent_type.as_deref(),
                    message_type,
                    payload: &payload,
                    trajectory_id,
                    scope_id,
                    artifact_ids: &artifact_ids,
                    priority,
                    expires_at,
                    tenant_id,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Verify acknowledged_at is None initially
                let get_before = message_get_heap(message_id, tenant_id);
                prop_assert!(get_before.is_ok(), "Get before acknowledge should succeed");
                let msg_before = get_before.unwrap().unwrap();
                prop_assert!(msg_before.message.acknowledged_at.is_none(), "acknowledged_at should be None before acknowledge");
                prop_assert_eq!(msg_before.tenant_id, Some(tenant_id));

                // Acknowledge the message
                let ack_result = message_acknowledge_heap(message_id, tenant_id);
                prop_assert!(ack_result.is_ok(), "Acknowledge should succeed: {:?}", ack_result.err());
                prop_assert!(ack_result.unwrap(), "Acknowledge should return true for existing message");

                // Verify acknowledged_at is now set
                let get_after = message_get_heap(message_id, tenant_id);
                prop_assert!(get_after.is_ok(), "Get after acknowledge should succeed");
                let msg_after = get_after.unwrap().unwrap();
                prop_assert!(msg_after.message.acknowledged_at.is_some(), "acknowledged_at should be set after acknowledge");
                prop_assert!(msg_after.message.acknowledged_at.unwrap() <= chrono::Utc::now(), "acknowledged_at should be <= now");
                prop_assert_eq!(msg_after.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Property 3: Index Consistency - To Agent Index
        ///
        /// *For any* message inserted via direct heap with a to_agent_id,
        /// querying via the to_agent index SHALL return that message.
        ///
        /// **Validates: Requirements 7.3, 13.1, 13.2, 13.4, 13.5**
        #[pg_test]
        fn prop_message_to_agent_index_consistency() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_agent_id(),
                arb_agent_id(),
                arb_agent_id(), // Always provide to_agent_id for this test
                arb_optional_agent_type(),
                arb_message_type(),
                arb_payload(),
                arb_optional_trajectory_id(),
                arb_optional_scope_id(),
                arb_artifact_ids(),
                arb_priority(),
                arb_optional_expires_at(),
            );

            runner.run(&strategy, |(
                from_agent_id,
                _to_agent_id_val,
                to_agent_id,
                to_agent_type,
                message_type,
                payload,
                trajectory_id,
                scope_id,
                artifact_ids,
                priority,
                expires_at,
            )| {
                // Generate a new message ID
                let message_id = MessageId::now_v7();
                let tenant_id = TenantId::now_v7();

                // Insert via heap with to_agent_id
                let insert_result = message_send_heap(MessageSendParams {
                    message_id,
                    from_agent_id,
                    to_agent_id: Some(to_agent_id),
                    to_agent_type: to_agent_type.as_deref(),
                    message_type,
                    payload: &payload,
                    trajectory_id,
                    scope_id,
                    artifact_ids: &artifact_ids,
                    priority,
                    expires_at,
                    tenant_id,
                });
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Query via to_agent index
                let list_result = message_list_for_agent_heap(to_agent_id, tenant_id);
                prop_assert!(list_result.is_ok(), "List for agent should succeed: {:?}", list_result.err());
                
                let messages = list_result.unwrap();
                prop_assert!(
                    messages.iter().any(|m| m.message.message_id == message_id),
                    "Inserted message should be found via to_agent index"
                );

                // Verify the found message has correct data
                let found_message = messages.iter().find(|m| m.message.message_id == message_id).unwrap();
                prop_assert_eq!(found_message.message.from_agent_id, from_agent_id);
                prop_assert_eq!(found_message.message.to_agent_id, Some(to_agent_id));
                prop_assert_eq!(found_message.message.message_type, message_type);
                prop_assert_eq!(found_message.message.priority, priority);
                prop_assert_eq!(found_message.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }
    }
}
