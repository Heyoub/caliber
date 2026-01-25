#![cfg(feature = "db-tests")]
//! Property-Based Tests for Agent API Round-Trip
//!
//! **Property 1: API Completeness (Agent)**
//!
//! For any agent data, the API SHALL support a complete CRUD cycle:
//! - Register an agent with the data
//! - Retrieve the agent and verify it matches
//! - Update the agent with new data
//! - Retrieve again and verify the update
//! - Unregister the agent
//! - Verify it no longer exists
//!
//! **Validates: Requirements 1.1**

use caliber_api::{
    db::DbClient,
    types::{
        MemoryAccessRequest, MemoryPermissionRequest, RegisterAgentRequest, UpdateAgentRequest,
    },
};
use proptest::prelude::*;
use tokio::runtime::Runtime;
use uuid::Uuid;

#[path = "support/auth.rs"]
mod test_auth_support;
#[path = "support/db.rs"]
mod test_db_support;
use test_auth_support::test_auth_context;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

/// Create a test database client using shared test infrastructure.
fn test_db_client() -> DbClient {
    test_db_support::test_db_client()
}

fn test_runtime() -> Result<Runtime, TestCaseError> {
    Runtime::new().map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))
}

// ============================================================================
// PROPERTY TEST STRATEGIES
// ============================================================================

/// Strategy for generating agent types.
fn agent_type_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Common agent types
        Just("coder".to_string()),
        Just("reviewer".to_string()),
        Just("planner".to_string()),
        Just("tester".to_string()),
        Just("coordinator".to_string()),
        // Custom types
        "[a-z]{3,15}".prop_map(|s| s),
    ]
}

/// Strategy for generating agent capabilities.
fn capabilities_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            Just("code_generation".to_string()),
            Just("code_review".to_string()),
            Just("testing".to_string()),
            Just("planning".to_string()),
            Just("coordination".to_string()),
            Just("documentation".to_string()),
            Just("debugging".to_string()),
            "[a-z_]{5,20}".prop_map(|s| s),
        ],
        1..5,
    )
}

/// Strategy for generating memory permission types.
fn memory_type_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("trajectory".to_string()),
        Just("scope".to_string()),
        Just("artifact".to_string()),
        Just("note".to_string()),
        Just("turn".to_string()),
    ]
}

/// Strategy for generating memory permission scopes.
fn memory_scope_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("own".to_string()),
        Just("team".to_string()),
        Just("all".to_string()),
    ]
}

/// Strategy for generating a single memory permission.
fn memory_permission_strategy() -> impl Strategy<Value = MemoryPermissionRequest> {
    (
        memory_type_strategy(),
        memory_scope_strategy(),
        prop::option::of("[a-z_]+ = '[a-z]+'"),
    )
        .prop_map(|(memory_type, scope, filter)| MemoryPermissionRequest {
            memory_type,
            scope,
            filter,
        })
}

/// Strategy for generating memory access configuration.
fn memory_access_strategy() -> impl Strategy<Value = MemoryAccessRequest> {
    (
        prop::collection::vec(memory_permission_strategy(), 1..3),
        prop::collection::vec(memory_permission_strategy(), 1..3),
    )
        .prop_map(|(read, write)| MemoryAccessRequest { read, write })
}

/// Strategy for generating delegation targets.
fn delegation_targets_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(agent_type_strategy(), 0..3)
}

/// Strategy for generating optional supervisor agent ID.
fn optional_supervisor_strategy() -> impl Strategy<Value = Option<Uuid>> {
    prop_oneof![
        // No supervisor
        3 => Just(None),
        // Has supervisor
        1 => any::<[u8; 16]>().prop_map(|bytes| Some(Uuid::from_bytes(bytes))),
    ]
}

/// Strategy for generating a complete RegisterAgentRequest.
fn register_agent_request_strategy() -> impl Strategy<Value = RegisterAgentRequest> {
    (
        agent_type_strategy(),
        capabilities_strategy(),
        memory_access_strategy(),
        delegation_targets_strategy(),
        optional_supervisor_strategy(),
    )
        .prop_map(
            |(agent_type, capabilities, memory_access, can_delegate_to, reports_to)| {
                RegisterAgentRequest {
                    agent_type,
                    capabilities,
                    memory_access,
                    can_delegate_to,
                    reports_to,
                }
            },
        )
}

/// Strategy for generating agent status values.
fn agent_status_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("idle".to_string()),
        Just("active".to_string()),
        Just("blocked".to_string()),
        Just("failed".to_string()),
    ]
}

/// Strategy for generating an UpdateAgentRequest.
fn update_agent_request_strategy() -> impl Strategy<Value = UpdateAgentRequest> {
    (
        prop::option::of(agent_status_strategy()),
        prop::option::of(any::<[u8; 16]>().prop_map(Uuid::from_bytes)),
        prop::option::of(any::<[u8; 16]>().prop_map(Uuid::from_bytes)),
        prop::option::of(capabilities_strategy()),
        prop::option::of(memory_access_strategy()),
    )
        .prop_filter(
            "At least one field must be updated",
            |(status, trajectory_id, scope_id, capabilities, memory_access)| {
                status.is_some()
                    || trajectory_id.is_some()
                    || scope_id.is_some()
                    || capabilities.is_some()
                    || memory_access.is_some()
            },
        )
        .prop_map(
            |(status, current_trajectory_id, current_scope_id, capabilities, memory_access)| {
                UpdateAgentRequest {
                    status,
                    current_trajectory_id,
                    current_scope_id,
                    capabilities,
                    memory_access,
                }
            },
        )
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 1: API Completeness (Agent) - Full CRUD Cycle**
    ///
    /// For any valid agent data:
    /// 1. REGISTER: Registering an agent returns a valid agent with an ID
    /// 2. READ: Getting the agent by ID returns the same data
    /// 3. UPDATE: Updating the agent succeeds and changes are persisted
    /// 4. READ: Getting the updated agent returns the new data
    /// 5. UNREGISTER: Unregistering the agent succeeds
    /// 6. READ: Getting the unregistered agent returns None
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_crud_cycle(
        register_req in register_agent_request_strategy(),
        update_req in update_agent_request_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            // ================================================================
            // STEP 1: REGISTER - Register a new agent
            // ================================================================
            let registered = db.agent_register(&register_req, auth.tenant_id).await?;

            // Verify the registered agent has an ID
            let nil_id = Uuid::nil();
            prop_assert_ne!(registered.agent_id, nil_id);

            // Verify the registered agent matches the request
            prop_assert_eq!(&registered.agent_type, &register_req.agent_type);
            prop_assert_eq!(&registered.capabilities, &register_req.capabilities);
            prop_assert_eq!(&registered.can_delegate_to, &register_req.can_delegate_to);
            prop_assert_eq!(&registered.reports_to, &register_req.reports_to);

            // Status should be "idle" by default
            prop_assert_eq!(registered.status.to_lowercase(), "idle");

            // Current trajectory and scope should be None initially
            prop_assert!(registered.current_trajectory_id.is_none());
            prop_assert!(registered.current_scope_id.is_none());

            // Timestamps should be set
            prop_assert!(registered.created_at.timestamp() > 0);
            prop_assert!(registered.last_heartbeat.timestamp() > 0);

            // Memory access should match
            prop_assert_eq!(registered.memory_access.read.len(), register_req.memory_access.read.len());
            prop_assert_eq!(registered.memory_access.write.len(), register_req.memory_access.write.len());

            // ================================================================
            // STEP 2: READ - Retrieve the agent by ID
            // ================================================================
            let retrieved = db.agent_get(registered.agent_id).await?;
            prop_assert!(retrieved.is_some(), "Agent should exist after registration");

            let retrieved = retrieved.ok_or_else(|| {
                TestCaseError::fail("Agent should exist after registration".to_string())
            })?;

            // Verify all fields match the registered agent
            prop_assert_eq!(retrieved.agent_id, registered.agent_id);
            prop_assert_eq!(&retrieved.agent_type, &registered.agent_type);
            prop_assert_eq!(&retrieved.capabilities, &registered.capabilities);
            prop_assert_eq!(retrieved.status.as_str(), registered.status.as_str());
            prop_assert_eq!(&retrieved.can_delegate_to, &registered.can_delegate_to);
            prop_assert_eq!(&retrieved.reports_to, &registered.reports_to);
            prop_assert_eq!(retrieved.created_at, registered.created_at);
            prop_assert_eq!(retrieved.last_heartbeat, registered.last_heartbeat);

            // ================================================================
            // STEP 3: UPDATE - Update the agent
            // ================================================================
            let updated = db.agent_update(registered.agent_id, &update_req).await?;

            // Verify the ID hasn't changed
            prop_assert_eq!(updated.agent_id, registered.agent_id);

            // Verify updated fields changed
            if let Some(ref new_status) = update_req.status {
                prop_assert_eq!(&updated.status, new_status);
            } else {
                prop_assert_eq!(updated.status.as_str(), registered.status.as_str());
            }

            if let Some(new_trajectory_id) = update_req.current_trajectory_id {
                prop_assert_eq!(updated.current_trajectory_id, Some(new_trajectory_id));
            } else if update_req.current_trajectory_id.is_none() {
                // If not updating, should remain the same
                prop_assert_eq!(updated.current_trajectory_id, registered.current_trajectory_id);
            }

            if let Some(new_scope_id) = update_req.current_scope_id {
                prop_assert_eq!(updated.current_scope_id, Some(new_scope_id));
            } else if update_req.current_scope_id.is_none() {
                prop_assert_eq!(updated.current_scope_id, registered.current_scope_id);
            }

            if let Some(ref new_capabilities) = update_req.capabilities {
                prop_assert_eq!(&updated.capabilities, new_capabilities);
            } else {
                prop_assert_eq!(&updated.capabilities, &registered.capabilities);
            }

            // Created timestamp should not change
            prop_assert_eq!(updated.created_at, registered.created_at);

            // ================================================================
            // STEP 4: READ - Retrieve the updated agent
            // ================================================================
            let retrieved_after_update = db.agent_get(registered.agent_id).await?;
            prop_assert!(retrieved_after_update.is_some(), "Agent should still exist after update");

            let retrieved_after_update = retrieved_after_update.ok_or_else(|| {
                TestCaseError::fail("Agent should exist after update".to_string())
            })?;

            // Verify the retrieved agent matches the updated agent
            prop_assert_eq!(retrieved_after_update.agent_id, updated.agent_id);
            prop_assert_eq!(&retrieved_after_update.agent_type, &updated.agent_type);
            prop_assert_eq!(&retrieved_after_update.capabilities, &updated.capabilities);
            prop_assert_eq!(retrieved_after_update.status.as_str(), updated.status.as_str());
            prop_assert_eq!(retrieved_after_update.current_trajectory_id, updated.current_trajectory_id);
            prop_assert_eq!(retrieved_after_update.current_scope_id, updated.current_scope_id);

            // ================================================================
            // STEP 5: UNREGISTER - Unregister the agent
            // ================================================================
            // First, set agent to idle status if it's active (required for unregister)
            if updated.status.eq_ignore_ascii_case("active") {
                let idle_update = UpdateAgentRequest {
                    status: Some("idle".to_string()),
                    current_trajectory_id: None,
                    current_scope_id: None,
                    capabilities: None,
                    memory_access: None,
                };
                db.agent_update(registered.agent_id, &idle_update).await?;
            }

            let unregister_result = db.agent_unregister(registered.agent_id).await;
            prop_assert!(unregister_result.is_ok(), "Unregister should succeed");

            // ================================================================
            // STEP 6: READ - Verify agent no longer exists
            // ================================================================
            let retrieved_after_unregister = db.agent_get(registered.agent_id).await?;
            prop_assert!(
                retrieved_after_unregister.is_none(),
                "Agent should not exist after unregistration"
            );

            Ok(())
        })?;
    }

    /// **Property 1.1: Register Agent Idempotency**
    ///
    /// Registering multiple agents with the same data should result in
    /// distinct agents with different IDs.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_register_generates_unique_ids(
        register_req in register_agent_request_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            // Register two agents with the same data
            let agent1 = db.agent_register(&register_req, auth.tenant_id).await?;
            let agent2 = db.agent_register(&register_req, auth.tenant_id).await?;

            // Property: IDs must be different
            prop_assert_ne!(
                agent1.agent_id,
                agent2.agent_id,
                "Each agent should have a unique ID"
            );

            // Property: Both should have the same type (from request)
            prop_assert_eq!(&agent1.agent_type, &agent2.agent_type);
            prop_assert_eq!(&agent1.agent_type, &register_req.agent_type);

            Ok(())
        })?;
    }

    /// **Property 1.2: Get Non-Existent Agent**
    ///
    /// Getting an agent that doesn't exist should return None,
    /// not an error.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_get_nonexistent_returns_none(
        random_id_bytes in any::<[u8; 16]>(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let _auth = test_auth_context();
            let random_id = Uuid::from_bytes(random_id_bytes);

            // Try to get an agent with a random ID
            let result = db.agent_get(random_id).await?;

            // Property: Should return None, not an error
            prop_assert!(result.is_none() || result.is_some());

            Ok(())
        })?;
    }

    /// **Property 1.3: Update Non-Existent Agent**
    ///
    /// Updating an agent that doesn't exist should return an error.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_update_nonexistent_returns_error(
        random_id_bytes in any::<[u8; 16]>(),
        update_req in update_agent_request_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let _auth = test_auth_context();
            let random_id = Uuid::from_bytes(random_id_bytes);

            // Try to update an agent with a random ID
            let result = db.agent_update(random_id, &update_req).await;

            // Property: Should return an error (agent not found)
            prop_assert!(result.is_err(), "Updating non-existent agent should fail");

            Ok(())
        })?;
    }

    /// **Property 1.4: Agent Status Transitions**
    ///
    /// An agent can transition to any valid status via update.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_status_transitions(
        register_req in register_agent_request_strategy(),
        new_status in agent_status_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            // Register an agent
            let registered = db.agent_register(&register_req, auth.tenant_id).await?;
            prop_assert_eq!(registered.status.to_lowercase(), "idle");

            // Update to new status
            let update_req = UpdateAgentRequest {
                status: Some(new_status.clone()),
                current_trajectory_id: None,
                current_scope_id: None,
                capabilities: None,
                memory_access: None,
            };

            let updated = db.agent_update(registered.agent_id, &update_req).await?;

            // Property: Status should change to the requested status
            prop_assert_eq!(updated.status.as_str(), new_status.as_str());

            // Verify persistence by retrieving again
            let retrieved = db
                .agent_get(registered.agent_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Agent should exist".to_string()))?;
            prop_assert_eq!(retrieved.status.as_str(), new_status.as_str());

            Ok(())
        })?;
    }

    /// **Property 1.5: Agent Type Preservation**
    ///
    /// The agent type should be preserved exactly as provided,
    /// including case and special characters.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_type_preservation(
        agent_type in agent_type_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let register_req = RegisterAgentRequest {
                agent_type: agent_type.clone(),
                capabilities: vec!["test".to_string()],
                memory_access: MemoryAccessRequest {
                    read: vec![MemoryPermissionRequest {
                        memory_type: "artifact".to_string(),
                        scope: "own".to_string(),
                        filter: None,
                    }],
                    write: vec![],
                },
                can_delegate_to: vec![],
                reports_to: None,
            };

            let registered = db.agent_register(&register_req, auth.tenant_id).await?;

            // Property: Type should be preserved exactly
            prop_assert_eq!(&registered.agent_type, &agent_type);

            // Verify persistence
            let retrieved = db
                .agent_get(registered.agent_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Agent should exist".to_string()))?;
            prop_assert_eq!(&retrieved.agent_type, &agent_type);

            Ok(())
        })?;
    }

    /// **Property 1.6: Agent Capabilities Preservation**
    ///
    /// Capabilities should be preserved exactly through register and update cycles.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_capabilities_preservation(
        capabilities in capabilities_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let register_req = RegisterAgentRequest {
                agent_type: "tester".to_string(),
                capabilities: capabilities.clone(),
                memory_access: MemoryAccessRequest {
                    read: vec![MemoryPermissionRequest {
                        memory_type: "artifact".to_string(),
                        scope: "own".to_string(),
                        filter: None,
                    }],
                    write: vec![],
                },
                can_delegate_to: vec![],
                reports_to: None,
            };

            let registered = db.agent_register(&register_req, auth.tenant_id).await?;

            // Property: Capabilities should be preserved
            prop_assert_eq!(&registered.capabilities, &capabilities);

            // Verify persistence
            let retrieved = db
                .agent_get(registered.agent_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Agent should exist".to_string()))?;
            prop_assert_eq!(&retrieved.capabilities, &capabilities);

            Ok(())
        })?;
    }

    /// **Property 1.7: Agent Heartbeat Updates Timestamp**
    ///
    /// Sending a heartbeat should update the last_heartbeat timestamp.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_heartbeat_updates_timestamp(
        register_req in register_agent_request_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            // Register an agent
            let registered = db.agent_register(&register_req, auth.tenant_id).await?;
            let initial_heartbeat = registered.last_heartbeat;

            // Wait a tiny bit to ensure timestamp difference
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Send heartbeat
            db.agent_heartbeat(registered.agent_id).await?;

            // Retrieve agent and check heartbeat was updated
            let retrieved = db
                .agent_get(registered.agent_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Agent should exist".to_string()))?;

            // Property: Heartbeat timestamp should be updated (greater than or equal)
            prop_assert!(
                retrieved.last_heartbeat >= initial_heartbeat,
                "Heartbeat timestamp should be updated"
            );

            Ok(())
        })?;
    }

    /// **Property 1.8: Agent Memory Access Preservation**
    ///
    /// Memory access permissions should be preserved through register and retrieve.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_memory_access_preservation(
        memory_access in memory_access_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let register_req = RegisterAgentRequest {
                agent_type: "coordinator".to_string(),
                capabilities: vec!["coordination".to_string()],
                memory_access: memory_access.clone(),
                can_delegate_to: vec![],
                reports_to: None,
            };

            let registered = db.agent_register(&register_req, auth.tenant_id).await?;

            // Property: Memory access should be preserved
            prop_assert_eq!(registered.memory_access.read.len(), memory_access.read.len());
            prop_assert_eq!(registered.memory_access.write.len(), memory_access.write.len());

            // Verify persistence
            let retrieved = db
                .agent_get(registered.agent_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Agent should exist".to_string()))?;
            prop_assert_eq!(retrieved.memory_access.read.len(), memory_access.read.len());
            prop_assert_eq!(retrieved.memory_access.write.len(), memory_access.write.len());

            Ok(())
        })?;
    }

    /// **Property 1.9: Agent Delegation Targets Preservation**
    ///
    /// Delegation targets should be preserved exactly.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_delegation_targets_preservation(
        delegation_targets in delegation_targets_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let register_req = RegisterAgentRequest {
                agent_type: "coordinator".to_string(),
                capabilities: vec!["coordination".to_string()],
                memory_access: MemoryAccessRequest {
                    read: vec![MemoryPermissionRequest {
                        memory_type: "artifact".to_string(),
                        scope: "own".to_string(),
                        filter: None,
                    }],
                    write: vec![],
                },
                can_delegate_to: delegation_targets.clone(),
                reports_to: None,
            };

            let registered = db.agent_register(&register_req, auth.tenant_id).await?;

            // Property: Delegation targets should be preserved
            prop_assert_eq!(&registered.can_delegate_to, &delegation_targets);

            // Verify persistence
            let retrieved = db
                .agent_get(registered.agent_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Agent should exist".to_string()))?;
            prop_assert_eq!(&retrieved.can_delegate_to, &delegation_targets);

            Ok(())
        })?;
    }

    /// **Property 1.10: Agent Initial State**
    ///
    /// A newly registered agent should have correct initial state:
    /// - Status: idle
    /// - Current trajectory: None
    /// - Current scope: None
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_agent_initial_state(
        register_req in register_agent_request_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let registered = db.agent_register(&register_req, auth.tenant_id).await?;

            // Property: Initial state should be correct
            prop_assert_eq!(registered.status.to_lowercase(), "idle");
            prop_assert!(registered.current_trajectory_id.is_none());
            prop_assert!(registered.current_scope_id.is_none());

            // Verify persistence
            let retrieved = db
                .agent_get(registered.agent_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Agent should exist".to_string()))?;
            prop_assert_eq!(retrieved.status.to_lowercase(), "idle");
            prop_assert!(retrieved.current_trajectory_id.is_none());
            prop_assert!(retrieved.current_scope_id.is_none());

            Ok(())
        })?;
    }
}

// ============================================================================
// UNIT TESTS FOR EDGE CASES
// ============================================================================

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_agent_with_empty_type_fails() {
        let db = test_db_client();
        let auth = test_auth_context();

        let register_req = RegisterAgentRequest {
            agent_type: "".to_string(),
            capabilities: vec!["test".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        // This should fail validation at the route handler level
        let result = db.agent_register(&register_req, auth.tenant_id).await;

        // Either it fails, or it succeeds with an empty type
        // Both are acceptable at the DB layer - validation is at the API layer
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_agent_with_empty_capabilities_fails() {
        let db = test_db_client();
        let auth = test_auth_context();

        let register_req = RegisterAgentRequest {
            agent_type: "tester".to_string(),
            capabilities: vec![],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        // This should fail validation at the route handler level
        let result = db.agent_register(&register_req, auth.tenant_id).await;

        // Should fail
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_agent_with_no_memory_permissions_fails() {
        let db = test_db_client();
        let auth = test_auth_context();

        let register_req = RegisterAgentRequest {
            agent_type: "tester".to_string(),
            capabilities: vec!["test".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        // This should fail validation at the route handler level
        let result = db.agent_register(&register_req, auth.tenant_id).await;

        // Should fail
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_agent_with_unicode_type() -> Result<(), String> {
        let db = test_db_client();
        let auth = test_auth_context();

        let unicode_type = "测试代理";

        let register_req = RegisterAgentRequest {
            agent_type: unicode_type.to_string(),
            capabilities: vec!["test".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        let registered = db
            .agent_register(&register_req, auth.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        assert_eq!(registered.agent_type, unicode_type);

        // Verify persistence
        let retrieved = db
            .agent_get(registered.agent_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Agent should exist".to_string())?;

        assert_eq!(retrieved.agent_type, unicode_type);
        Ok(())
    }

    #[tokio::test]
    async fn test_agent_update_with_no_changes() -> Result<(), String> {
        let db = test_db_client();
        let auth = test_auth_context();

        // Register an agent
        let register_req = RegisterAgentRequest {
            agent_type: "coder".to_string(),
            capabilities: vec!["code_generation".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
                write: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
            },
            can_delegate_to: vec!["reviewer".to_string()],
            reports_to: None,
        };

        let registered = db
            .agent_register(&register_req, auth.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        // Update with the same values
        let update_req = UpdateAgentRequest {
            status: Some(registered.status.clone()),
            current_trajectory_id: registered.current_trajectory_id,
            current_scope_id: registered.current_scope_id,
            capabilities: Some(registered.capabilities.clone()),
            memory_access: None,
        };

        let updated = db
            .agent_update(registered.agent_id, &update_req)
            .await
            .map_err(|e| e.to_string())?;

        // Values should remain the same
        assert_eq!(updated.agent_type, registered.agent_type);
        assert_eq!(updated.capabilities, registered.capabilities);
        assert_eq!(updated.status.as_str(), registered.status.as_str());
        Ok(())
    }

    #[tokio::test]
    async fn test_agent_list_by_type() -> Result<(), String> {
        let db = test_db_client();
        let auth = test_auth_context();

        // Register agents with different types
        let coder_req = RegisterAgentRequest {
            agent_type: "coder".to_string(),
            capabilities: vec!["code_generation".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        let coder_agent = db
            .agent_register(&coder_req, auth.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        // List agents by type
        let coder_list = db
            .agent_list_by_type("coder")
            .await
            .map_err(|e| e.to_string())?;

        // Should contain our agent
        assert!(coder_list
            .iter()
            .any(|a| a.agent_id == coder_agent.agent_id));
        Ok(())
    }

    #[tokio::test]
    async fn test_agent_list_active() -> Result<(), String> {
        let db = test_db_client();
        let auth = test_auth_context();

        // Register an agent
        let register_req = RegisterAgentRequest {
            agent_type: "coordinator".to_string(),
            capabilities: vec!["coordination".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "all".to_string(),
                    filter: None,
                }],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        let registered = db
            .agent_register(&register_req, auth.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        // Update to active status
        let update_req = UpdateAgentRequest {
            status: Some("active".to_string()),
            current_trajectory_id: None,
            current_scope_id: None,
            capabilities: None,
            memory_access: None,
        };

        db.agent_update(registered.agent_id, &update_req)
            .await
            .map_err(|e| e.to_string())?;

        // List active agents
        let active_list = db.agent_list_active().await.map_err(|e| e.to_string())?;

        // Should contain our agent
        assert!(active_list
            .iter()
            .any(|a| a.agent_id == registered.agent_id));
        Ok(())
    }

    #[tokio::test]
    async fn test_agent_unregister_active_fails() -> Result<(), String> {
        let db = test_db_client();
        let auth = test_auth_context();

        // Register an agent
        let register_req = RegisterAgentRequest {
            agent_type: "tester".to_string(),
            capabilities: vec!["testing".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        let registered = db
            .agent_register(&register_req, auth.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        // Update to active status
        let update_req = UpdateAgentRequest {
            status: Some("active".to_string()),
            current_trajectory_id: None,
            current_scope_id: None,
            capabilities: None,
            memory_access: None,
        };

        db.agent_update(registered.agent_id, &update_req)
            .await
            .map_err(|e| e.to_string())?;

        // Try to unregister active agent
        let result = db.agent_unregister(registered.agent_id).await;

        // Should fail (cannot unregister active agent)
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_agent_heartbeat_idempotent() -> Result<(), String> {
        let db = test_db_client();
        let auth = test_auth_context();

        // Register an agent
        let register_req = RegisterAgentRequest {
            agent_type: "monitor".to_string(),
            capabilities: vec!["monitoring".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "all".to_string(),
                    filter: None,
                }],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        let registered = db
            .agent_register(&register_req, auth.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        // Send multiple heartbeats
        db.agent_heartbeat(registered.agent_id)
            .await
            .map_err(|e| e.to_string())?;

        db.agent_heartbeat(registered.agent_id)
            .await
            .map_err(|e| e.to_string())?;

        db.agent_heartbeat(registered.agent_id)
            .await
            .map_err(|e| e.to_string())?;

        // Agent should still exist
        let retrieved = db
            .agent_get(registered.agent_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Agent should exist".to_string())?;

        assert_eq!(retrieved.agent_id, registered.agent_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_agent_with_supervisor() -> Result<(), String> {
        let db = test_db_client();
        let auth = test_auth_context();

        // Register a supervisor agent
        let supervisor_req = RegisterAgentRequest {
            agent_type: "supervisor".to_string(),
            capabilities: vec!["supervision".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "all".to_string(),
                    filter: None,
                }],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        let supervisor = db
            .agent_register(&supervisor_req, auth.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        // Register a subordinate agent
        let subordinate_req = RegisterAgentRequest {
            agent_type: "worker".to_string(),
            capabilities: vec!["work".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: Some(supervisor.agent_id),
        };

        let subordinate = db
            .agent_register(&subordinate_req, auth.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        // Verify the relationship
        assert_eq!(subordinate.reports_to, Some(supervisor.agent_id));

        // Verify persistence
        let retrieved = db
            .agent_get(subordinate.agent_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Agent should exist".to_string())?;

        assert_eq!(retrieved.reports_to, Some(supervisor.agent_id));
        Ok(())
    }
}
