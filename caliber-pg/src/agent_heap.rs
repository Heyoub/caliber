//! Direct heap operations for Agent entities.
//!
//! This module provides hot-path operations for agent registration and coordination
//! that bypass SQL parsing entirely by using direct heap access via pgrx.

use pgrx::pg_sys;
use pgrx::prelude::*;

use caliber_core::{
    Agent, AgentId, AgentStatus, CaliberError, CaliberResult, EntityIdType, EntityType,
    MemoryAccess, ScopeId, StorageError, TenantId, TrajectoryId,
};

use crate::column_maps::agent;
use crate::heap_ops::{
    current_timestamp, form_tuple, get_active_snapshot, insert_tuple, open_relation,
    timestamp_to_pgrx, update_tuple, HeapRelation, PgLockMode as HeapLockMode,
};
use crate::index_ops::{
    init_scan_key, open_index, operator_oids, update_indexes_for_insert, BTreeStrategy,
    IndexScanner,
};
use crate::tuple_extract::{
    extract_jsonb, extract_text, extract_text_array, extract_timestamp, extract_uuid,
    extract_values_and_nulls, json_to_datum, option_uuid_to_datum, string_to_datum,
    text_array_to_datum, timestamp_to_chrono, uuid_to_datum,
};

/// Agent row with tenant ownership metadata.
pub struct AgentRow {
    pub agent: Agent,
    pub tenant_id: Option<TenantId>,
}

impl From<AgentRow> for Agent {
    fn from(row: AgentRow) -> Self {
        row.agent
    }
}

/// Register a new agent by inserting an agent record using direct heap operations.
pub fn agent_register_heap(
    agent_id: AgentId,
    agent_type: &str,
    capabilities: &[String],
    memory_access: &MemoryAccess,
    can_delegate_to: &[String],
    reports_to: Option<AgentId>,
    tenant_id: TenantId,
) -> CaliberResult<AgentId> {
    let rel = open_relation(agent::TABLE_NAME, HeapLockMode::RowExclusive)?;
    validate_agent_relation(&rel)?;

    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now)?.into_datum().ok_or_else(|| {
        CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Agent,
            reason: "Failed to convert timestamp to datum".to_string(),
        })
    })?;

    // Serialize memory_access to JSON
    let memory_access_json = serde_json::to_value(memory_access).map_err(|e| {
        CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Agent,
            reason: format!("Failed to serialize memory_access: {}", e),
        })
    })?;

    let mut values: [pg_sys::Datum; agent::NUM_COLS] = [pg_sys::Datum::from(0); agent::NUM_COLS];
    let mut nulls: [bool; agent::NUM_COLS] = [false; agent::NUM_COLS];

    // Set required fields
    values[agent::AGENT_ID as usize - 1] = uuid_to_datum(agent_id.as_uuid());
    values[agent::AGENT_TYPE as usize - 1] = string_to_datum(agent_type);

    // Set capabilities array
    if capabilities.is_empty() {
        nulls[agent::CAPABILITIES as usize - 1] = true;
    } else {
        values[agent::CAPABILITIES as usize - 1] = text_array_to_datum(capabilities);
    }

    // Set memory_access JSONB
    values[agent::MEMORY_ACCESS as usize - 1] = json_to_datum(&memory_access_json);

    // Set status - default to "idle"
    values[agent::STATUS as usize - 1] = string_to_datum("idle");

    // Set optional current_trajectory_id
    nulls[agent::CURRENT_TRAJECTORY_ID as usize - 1] = true;

    // Set optional current_scope_id
    nulls[agent::CURRENT_SCOPE_ID as usize - 1] = true;

    // Set can_delegate_to array
    if can_delegate_to.is_empty() {
        nulls[agent::CAN_DELEGATE_TO as usize - 1] = true;
    } else {
        values[agent::CAN_DELEGATE_TO as usize - 1] = text_array_to_datum(can_delegate_to);
    }

    // Set optional reports_to using helper
    let (reports_to_datum, reports_to_null) = build_optional_agent_uuid(reports_to);
    values[agent::REPORTS_TO as usize - 1] = reports_to_datum;
    nulls[agent::REPORTS_TO as usize - 1] = reports_to_null;

    // Set timestamps
    values[agent::CREATED_AT as usize - 1] = now_datum;
    values[agent::LAST_HEARTBEAT as usize - 1] = now_datum;

    // Set tenant_id
    values[agent::TENANT_ID as usize - 1] = uuid_to_datum(tenant_id.as_uuid());

    let tuple = form_tuple(&rel, &values, &nulls)?;
    let _tid = unsafe { insert_tuple(&rel, tuple)? };
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };

    Ok(agent_id)
}

/// Get an agent by ID using direct heap operations.
pub fn agent_get_heap(agent_id: AgentId, tenant_id: TenantId) -> CaliberResult<Option<AgentRow>> {
    let rel = open_relation(agent::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(agent::PK_INDEX)?;
    let snapshot = get_active_snapshot();

    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(agent_id.as_uuid()),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    if let Some(tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let row = unsafe { tuple_to_agent(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid()) {
            Ok(Some(row))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Update agent heartbeat timestamp using direct heap operations.
pub fn agent_heartbeat_heap(agent_id: AgentId, tenant_id: TenantId) -> CaliberResult<bool> {
    let rel = open_relation(agent::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(agent::PK_INDEX)?;
    let snapshot = get_active_snapshot();

    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(agent_id.as_uuid()),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    if let Some(old_tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, agent::TENANT_ID)? };
        if existing_tenant != Some(tenant_id.as_uuid()) {
            return Ok(false);
        }
        let (mut values, nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;

        // Update last_heartbeat to current timestamp
        let now = current_timestamp();
        let now_datum = timestamp_to_pgrx(now)?.into_datum().ok_or_else(|| {
            CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Agent,
                id: agent_id.as_uuid(),
                reason: "Failed to convert timestamp to datum".to_string(),
            })
        })?;

        values[agent::LAST_HEARTBEAT as usize - 1] = now_datum;

        let new_tuple = form_tuple(&rel, &values, &nulls)?;
        let old_tid = scanner.current_tid().ok_or_else(|| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of agent tuple".to_string(),
            })
        })?;

        unsafe { update_tuple(&rel, &old_tid, new_tuple)? };
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Update agent status using direct heap operations.
pub fn agent_set_status_heap(
    agent_id: AgentId,
    status: AgentStatus,
    tenant_id: TenantId,
) -> CaliberResult<bool> {
    let rel = open_relation(agent::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(agent::PK_INDEX)?;
    let snapshot = get_active_snapshot();

    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(agent_id.as_uuid()),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    if let Some(old_tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, agent::TENANT_ID)? };
        if existing_tenant != Some(tenant_id.as_uuid()) {
            return Ok(false);
        }
        let (mut values, nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;

        // Update status field
        let status_str = match status {
            AgentStatus::Idle => "idle",
            AgentStatus::Active => "active",
            AgentStatus::Blocked => "blocked",
            AgentStatus::Failed => "failed",
            AgentStatus::Offline => "offline",
        };
        values[agent::STATUS as usize - 1] = string_to_datum(status_str);

        let new_tuple = form_tuple(&rel, &values, &nulls)?;
        let old_tid = scanner.current_tid().ok_or_else(|| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of agent tuple".to_string(),
            })
        })?;

        unsafe { update_tuple(&rel, &old_tid, new_tuple)? };
        Ok(true)
    } else {
        Ok(false)
    }
}

/// List agents by type using direct heap operations.
pub fn agent_list_by_type_heap(
    agent_type: &str,
    tenant_id: TenantId,
) -> CaliberResult<Vec<AgentRow>> {
    let rel = open_relation(agent::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(agent::TYPE_INDEX)?;
    let snapshot = get_active_snapshot();

    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::TEXT_EQ,
        string_to_datum(agent_type),
    );

    let mut scanner = unsafe { IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key) };

    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();

    for tuple in &mut scanner {
        let row = unsafe { tuple_to_agent(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid()) {
            results.push(row);
        }
    }

    Ok(results)
}

/// Validate that a HeapRelation is suitable for agent operations.
fn validate_agent_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != agent::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Agent relation has {} columns, expected {}",
                natts,
                agent::NUM_COLS
            ),
        }));
    }
    Ok(())
}

/// Build optional UUID datum using proper helper.
fn build_optional_agent_uuid(reports_to: Option<AgentId>) -> (pg_sys::Datum, bool) {
    match reports_to {
        Some(id) => (option_uuid_to_datum(Some(id.as_uuid())), false),
        None => (pg_sys::Datum::from(0), true),
    }
}

unsafe fn tuple_to_agent(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<AgentRow> {
    let agent_id = extract_uuid(tuple, tuple_desc, agent::AGENT_ID)?.ok_or_else(|| {
        CaliberError::Storage(StorageError::TransactionFailed {
            reason: "agent_id is NULL".to_string(),
        })
    })?;
    let agent_id = AgentId::new(agent_id);

    let agent_type = extract_text(tuple, tuple_desc, agent::AGENT_TYPE)?.ok_or_else(|| {
        CaliberError::Storage(StorageError::TransactionFailed {
            reason: "agent_type is NULL".to_string(),
        })
    })?;

    let capabilities =
        extract_text_array(tuple, tuple_desc, agent::CAPABILITIES)?.unwrap_or_default();

    let memory_access_json =
        extract_jsonb(tuple, tuple_desc, agent::MEMORY_ACCESS)?.ok_or_else(|| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "memory_access is NULL".to_string(),
            })
        })?;
    let memory_access: MemoryAccess = serde_json::from_value(memory_access_json).map_err(|e| {
        CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to deserialize memory_access: {}", e),
        })
    })?;

    let status_str = extract_text(tuple, tuple_desc, agent::STATUS)?.ok_or_else(|| {
        CaliberError::Storage(StorageError::TransactionFailed {
            reason: "status is NULL".to_string(),
        })
    })?;
    let status = match status_str.as_str() {
        "idle" => AgentStatus::Idle,
        "active" => AgentStatus::Active,
        "blocked" => AgentStatus::Blocked,
        "failed" => AgentStatus::Failed,
        "offline" => AgentStatus::Offline,
        _ => {
            pgrx::warning!(
                "CALIBER: Unknown agent status '{}', defaulting to Idle",
                status_str
            );
            AgentStatus::Idle
        }
    };

    let current_trajectory_id =
        extract_uuid(tuple, tuple_desc, agent::CURRENT_TRAJECTORY_ID)?.map(TrajectoryId::new);
    let current_scope_id =
        extract_uuid(tuple, tuple_desc, agent::CURRENT_SCOPE_ID)?.map(ScopeId::new);

    let can_delegate_to =
        extract_text_array(tuple, tuple_desc, agent::CAN_DELEGATE_TO)?.unwrap_or_default();

    let reports_to = extract_uuid(tuple, tuple_desc, agent::REPORTS_TO)?.map(AgentId::new);

    let created_at_ts =
        extract_timestamp(tuple, tuple_desc, agent::CREATED_AT)?.ok_or_else(|| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "created_at is NULL".to_string(),
            })
        })?;
    let created_at = timestamp_to_chrono(created_at_ts);

    let last_heartbeat_ts = extract_timestamp(tuple, tuple_desc, agent::LAST_HEARTBEAT)?
        .ok_or_else(|| {
            CaliberError::Storage(StorageError::TransactionFailed {
                reason: "last_heartbeat is NULL".to_string(),
            })
        })?;
    let last_heartbeat = timestamp_to_chrono(last_heartbeat_ts);

    let tenant_id = extract_uuid(tuple, tuple_desc, agent::TENANT_ID)?.map(TenantId::new);

    Ok(AgentRow {
        agent: Agent {
            agent_id,
            agent_type,
            capabilities,
            memory_access,
            status,
            current_trajectory_id,
            current_scope_id,
            can_delegate_to,
            reports_to,
            created_at,
            last_heartbeat,
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
    use caliber_core::{MemoryPermission, PermissionScope};
    use proptest::prelude::*;

    // ========================================================================
    // Test Helpers - Generators for Agent data
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

    /// Generate a valid agent type
    fn arb_agent_type() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("coordinator".to_string()),
            Just("worker".to_string()),
            Just("specialist".to_string()),
            Just("reviewer".to_string()),
            Just("planner".to_string()),
        ]
    }

    /// Generate a vector of capabilities (0-5 items)
    fn arb_capabilities() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop_oneof![
                Just("read".to_string()),
                Just("write".to_string()),
                Just("execute".to_string()),
                Just("analyze".to_string()),
                Just("coordinate".to_string()),
            ],
            0..=5,
        )
    }

    /// Generate a MemoryAccess configuration
    fn arb_memory_access() -> impl Strategy<Value = MemoryAccess> {
        Just(MemoryAccess {
            read: vec![MemoryPermission {
                memory_type: "*".to_string(),
                scope: PermissionScope::Own,
                filter: None,
            }],
            write: vec![MemoryPermission {
                memory_type: "*".to_string(),
                scope: PermissionScope::Own,
                filter: None,
            }],
        })
    }

    /// Generate a vector of agent types for delegation (0-3 items)
    fn arb_can_delegate_to() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop_oneof![
                Just("worker".to_string()),
                Just("specialist".to_string()),
                Just("reviewer".to_string()),
            ],
            0..=3,
        )
    }

    /// Generate an agent status
    fn arb_agent_status() -> impl Strategy<Value = AgentStatus> {
        prop_oneof![
            Just(AgentStatus::Idle),
            Just(AgentStatus::Active),
            Just(AgentStatus::Blocked),
            Just(AgentStatus::Failed),
        ]
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Agent)
    // Feature: caliber-pg-hot-path, Property 1: Insert-Get Round Trip
    // Validates: Requirements 8.1, 8.2
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use crate::pg_test;

        /// Property 1: Insert-Get Round Trip (Agent)
        ///
        /// *For any* valid agent data, inserting via direct heap then getting
        /// via direct heap SHALL return an equivalent agent.
        ///
        /// **Validates: Requirements 8.1, 8.2**
        #[pg_test]
        fn prop_agent_insert_get_roundtrip() {
            use proptest::test_runner::{Config, TestRunner};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_agent_type(),
                arb_capabilities(),
                arb_memory_access(),
                arb_can_delegate_to(),
                arb_optional_agent_id(),
            );

            runner
                .run(
                    &strategy,
                    |(agent_type, capabilities, memory_access, can_delegate_to, reports_to)| {
                        // Generate a new agent ID
                        let agent_id = AgentId::now_v7();
                        let tenant_id = TenantId::now_v7();

                        // Insert via heap
                        let result = agent_register_heap(
                            agent_id,
                            &agent_type,
                            &capabilities,
                            &memory_access,
                            &can_delegate_to,
                            reports_to,
                            tenant_id,
                        );
                        prop_assert!(result.is_ok(), "Insert should succeed: {:?}", result.err());
                        prop_assert_eq!(result.unwrap(), agent_id);

                        // Get via heap
                        let get_result = agent_get_heap(agent_id, tenant_id);
                        prop_assert!(
                            get_result.is_ok(),
                            "Get should succeed: {:?}",
                            get_result.err()
                        );

                        let agent = get_result.unwrap();
                        prop_assert!(agent.is_some(), "Agent should be found");

                        let row = agent.unwrap();
                        let a = row.agent;

                        // Verify round-trip preserves data
                        prop_assert_eq!(a.agent_id, agent_id);
                        prop_assert_eq!(a.agent_type, agent_type);
                        prop_assert_eq!(a.capabilities, capabilities);
                        prop_assert_eq!(a.status, AgentStatus::Idle); // Default status
                        prop_assert_eq!(a.can_delegate_to, can_delegate_to);
                        prop_assert_eq!(a.reports_to, reports_to);

                        // Timestamps should be set
                        prop_assert!(a.created_at <= chrono::Utc::now());
                        prop_assert!(a.last_heartbeat <= chrono::Utc::now());
                        prop_assert!(
                            a.current_trajectory_id.is_none(),
                            "current_trajectory_id should be None initially"
                        );
                        prop_assert!(
                            a.current_scope_id.is_none(),
                            "current_scope_id should be None initially"
                        );
                        prop_assert_eq!(row.tenant_id, Some(tenant_id));

                        Ok(())
                    },
                )
                .unwrap();
        }

        /// Property 1 (edge case): Get non-existent agent returns None
        ///
        /// *For any* random UUID that was never inserted, getting it SHALL
        /// return Ok(None), not an error.
        ///
        /// **Validates: Requirements 8.2**
        #[pg_test]
        fn prop_agent_get_nonexistent_returns_none() {
            use proptest::test_runner::{Config, TestRunner};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner
                .run(&any::<[u8; 16]>(), |bytes| {
                    let random_id = AgentId::new(uuid::Uuid::from_bytes(bytes));

                    let tenant_id = TenantId::now_v7();
                    let result = agent_get_heap(random_id, tenant_id);
                    prop_assert!(result.is_ok(), "Get should not error: {:?}", result.err());
                    prop_assert!(
                        result.unwrap().is_none(),
                        "Non-existent agent should return None"
                    );

                    Ok(())
                })
                .unwrap();
        }

        /// Property 2: Update Persistence (Agent - heartbeat)
        ///
        /// *For any* agent that has been inserted, updating its heartbeat SHALL
        /// update the last_heartbeat field and persist the change.
        ///
        /// **Validates: Requirements 8.3**
        #[pg_test]
        fn prop_agent_heartbeat_persistence() {
            use proptest::test_runner::{Config, TestRunner};
            use std::thread;
            use std::time::Duration;

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_agent_type(),
                arb_capabilities(),
                arb_memory_access(),
                arb_can_delegate_to(),
                arb_optional_agent_id(),
            );

            runner
                .run(
                    &strategy,
                    |(agent_type, capabilities, memory_access, can_delegate_to, reports_to)| {
                        // Generate a new agent ID
                        let agent_id = AgentId::now_v7();
                        let tenant_id = TenantId::now_v7();

                        // Insert via heap
                        let insert_result = agent_register_heap(
                            agent_id,
                            &agent_type,
                            &capabilities,
                            &memory_access,
                            &can_delegate_to,
                            reports_to,
                            tenant_id,
                        );
                        prop_assert!(insert_result.is_ok(), "Insert should succeed");

                        // Get initial heartbeat
                        let get_before = agent_get_heap(agent_id, tenant_id);
                        prop_assert!(get_before.is_ok(), "Get before heartbeat should succeed");
                        let agent_before = get_before.unwrap().unwrap();
                        let heartbeat_before = agent_before.agent.last_heartbeat;
                        prop_assert_eq!(agent_before.tenant_id, Some(tenant_id));

                        // Small delay to ensure timestamp difference
                        thread::sleep(Duration::from_millis(10));

                        // Update heartbeat
                        let heartbeat_result = agent_heartbeat_heap(agent_id, tenant_id);
                        prop_assert!(
                            heartbeat_result.is_ok(),
                            "Heartbeat should succeed: {:?}",
                            heartbeat_result.err()
                        );
                        prop_assert!(
                            heartbeat_result.unwrap(),
                            "Heartbeat should return true for existing agent"
                        );

                        // Verify heartbeat was updated
                        let get_after = agent_get_heap(agent_id, tenant_id);
                        prop_assert!(get_after.is_ok(), "Get after heartbeat should succeed");
                        let agent_after = get_after.unwrap().unwrap();
                        prop_assert!(
                            agent_after.agent.last_heartbeat >= heartbeat_before,
                            "last_heartbeat should be updated (before: {:?}, after: {:?})",
                            heartbeat_before,
                            agent_after.agent.last_heartbeat
                        );
                        prop_assert_eq!(agent_after.tenant_id, Some(tenant_id));

                        Ok(())
                    },
                )
                .unwrap();
        }

        /// Property 2: Update Persistence (Agent - status)
        ///
        /// *For any* agent that has been inserted, updating its status SHALL
        /// update the status field and persist the change.
        ///
        /// **Validates: Requirements 8.4**
        #[pg_test]
        fn prop_agent_set_status_persistence() {
            use proptest::test_runner::{Config, TestRunner};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_agent_type(),
                arb_capabilities(),
                arb_memory_access(),
                arb_can_delegate_to(),
                arb_optional_agent_id(),
                arb_agent_status(),
            );

            runner
                .run(
                    &strategy,
                    |(
                        agent_type,
                        capabilities,
                        memory_access,
                        can_delegate_to,
                        reports_to,
                        new_status,
                    )| {
                        // Generate a new agent ID
                        let agent_id = AgentId::now_v7();
                        let tenant_id = TenantId::now_v7();

                        // Insert via heap
                        let insert_result = agent_register_heap(
                            agent_id,
                            &agent_type,
                            &capabilities,
                            &memory_access,
                            &can_delegate_to,
                            reports_to,
                            tenant_id,
                        );
                        prop_assert!(insert_result.is_ok(), "Insert should succeed");

                        // Verify initial status is Idle
                        let get_before = agent_get_heap(agent_id, tenant_id);
                        prop_assert!(
                            get_before.is_ok(),
                            "Get before status update should succeed"
                        );
                        let agent_before = get_before.unwrap().unwrap();
                        prop_assert_eq!(
                            agent_before.agent.status,
                            AgentStatus::Idle,
                            "Initial status should be Idle"
                        );
                        prop_assert_eq!(agent_before.tenant_id, Some(tenant_id));

                        // Update status
                        let status_result = agent_set_status_heap(agent_id, new_status, tenant_id);
                        prop_assert!(
                            status_result.is_ok(),
                            "Set status should succeed: {:?}",
                            status_result.err()
                        );
                        prop_assert!(
                            status_result.unwrap(),
                            "Set status should return true for existing agent"
                        );

                        // Verify status was updated
                        let get_after = agent_get_heap(agent_id, tenant_id);
                        prop_assert!(get_after.is_ok(), "Get after status update should succeed");
                        let agent_after = get_after.unwrap().unwrap();
                        prop_assert_eq!(
                            agent_after.agent.status,
                            new_status,
                            "Status should be updated"
                        );
                        prop_assert_eq!(agent_after.tenant_id, Some(tenant_id));

                        Ok(())
                    },
                )
                .unwrap();
        }

        /// Property 2 (edge case): Update non-existent agent returns false
        ///
        /// *For any* random UUID that was never inserted, updating it SHALL
        /// return Ok(false), not an error.
        ///
        /// **Validates: Requirements 8.3, 8.4**
        #[pg_test]
        fn prop_agent_update_nonexistent_returns_false() {
            use proptest::test_runner::{Config, TestRunner};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (any::<[u8; 16]>(), arb_agent_status());

            runner
                .run(&strategy, |(bytes, status)| {
                    let random_id = AgentId::new(uuid::Uuid::from_bytes(bytes));

                    // Try heartbeat
                    let tenant_id = TenantId::now_v7();
                    let heartbeat_result = agent_heartbeat_heap(random_id, tenant_id);
                    prop_assert!(heartbeat_result.is_ok(), "Heartbeat should not error");
                    prop_assert!(
                        !heartbeat_result.unwrap(),
                        "Heartbeat of non-existent agent should return false"
                    );

                    // Try set status
                    let tenant_id = TenantId::now_v7();
                    let status_result = agent_set_status_heap(random_id, status, tenant_id);
                    prop_assert!(status_result.is_ok(), "Set status should not error");
                    prop_assert!(
                        !status_result.unwrap(),
                        "Set status of non-existent agent should return false"
                    );

                    Ok(())
                })
                .unwrap();
        }

        /// Property 3: Index Consistency - Type Index
        ///
        /// *For any* agent inserted via direct heap, querying via the agent_type
        /// index SHALL return that agent.
        ///
        /// **Validates: Requirements 8.5, 13.1, 13.2, 13.4, 13.5**
        #[pg_test]
        fn prop_agent_type_index_consistency() {
            use proptest::test_runner::{Config, TestRunner};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_agent_type(),
                arb_capabilities(),
                arb_memory_access(),
                arb_can_delegate_to(),
                arb_optional_agent_id(),
            );

            runner
                .run(
                    &strategy,
                    |(agent_type, capabilities, memory_access, can_delegate_to, reports_to)| {
                        // Generate a new agent ID
                        let agent_id = AgentId::now_v7();
                        let tenant_id = TenantId::now_v7();

                        // Insert via heap
                        let insert_result = agent_register_heap(
                            agent_id,
                            &agent_type,
                            &capabilities,
                            &memory_access,
                            &can_delegate_to,
                            reports_to,
                            tenant_id,
                        );
                        prop_assert!(insert_result.is_ok(), "Insert should succeed");

                        // Query via type index
                        let list_result = agent_list_by_type_heap(&agent_type, tenant_id);
                        prop_assert!(
                            list_result.is_ok(),
                            "List by type should succeed: {:?}",
                            list_result.err()
                        );

                        let agents = list_result.unwrap();
                        prop_assert!(
                            agents.iter().any(|a| a.agent.agent_id == agent_id),
                            "Inserted agent should be found via type index"
                        );

                        // Verify the found agent has correct data
                        let found_agent = agents
                            .iter()
                            .find(|a| a.agent.agent_id == agent_id)
                            .unwrap();
                        prop_assert_eq!(&found_agent.agent.agent_type, &agent_type);
                        prop_assert_eq!(&found_agent.agent.capabilities, &capabilities);
                        prop_assert_eq!(&found_agent.agent.can_delegate_to, &can_delegate_to);
                        prop_assert_eq!(found_agent.agent.reports_to, reports_to);
                        prop_assert_eq!(found_agent.tenant_id, Some(tenant_id));

                        Ok(())
                    },
                )
                .unwrap();
        }
    }
}
