//! Property-Based Tests for Mutation Broadcast Events
//!
//! **Property 3: Mutation Broadcast**
//!
//! For any entity mutation (create, update, delete) performed via the API,
//! a corresponding WebSocket event SHALL be broadcast to all subscribed
//! clients within 100ms.
//!
//! **Validates: Requirements 1.4**

use axum::extract::{Path, State};
use axum::Json;
use caliber_api::{
    db::DbClient,
    events::WsEvent,
    middleware::AuthExtractor,
    routes::{
        agent, artifact, delegation, handoff, lock, message, note, scope, trajectory, turn,
    },
    types::{
        AcquireLockRequest, CreateArtifactRequest, CreateDelegationRequest, CreateHandoffRequest,
        CreateNoteRequest, CreateScopeRequest, CreateTrajectoryRequest, CreateTurnRequest,
        MemoryAccessRequest, MemoryPermissionRequest, RegisterAgentRequest, SendMessageRequest,
        UpdateAgentRequest, UpdateScopeRequest, UpdateTrajectoryRequest,
    },
};
use caliber_core::{ArtifactType, EntityId, ExtractionMethod, NoteType, TTL, TrajectoryStatus, TurnRole};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use uuid::Uuid;
#[path = "support/auth.rs"]
mod test_auth_support;
#[path = "support/db.rs"]
mod test_db_support;
#[path = "support/ws.rs"]
mod test_ws_support;
#[path = "support/pcp.rs"]
mod test_pcp_support;
use test_auth_support::test_auth_context;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

/// Create a WebSocket state for testing broadcasts.
fn test_ws_state() -> Arc<caliber_api::ws::WsState> {
    test_ws_support::test_ws_state(128)
}
// ============================================================================
// MUTATION CASES
// ============================================================================

#[derive(Debug, Clone)]
enum MutationCase {
    TrajectoryCreate,
    TrajectoryUpdate,
    TrajectoryDelete,
    ScopeCreate,
    ScopeUpdate,
    ScopeClose,
    ArtifactCreate,
    NoteCreate,
    TurnCreate,
    AgentRegister,
    AgentUpdate,
    AgentUnregister,
    LockAcquire,
    LockRelease,
    MessageSend,
    MessageAcknowledge,
    DelegationCreate,
    DelegationAccept,
    DelegationComplete,
    HandoffCreate,
    HandoffAccept,
    HandoffComplete,
}

fn mutation_case_strategy() -> impl Strategy<Value = MutationCase> {
    prop_oneof![
        Just(MutationCase::TrajectoryCreate),
        Just(MutationCase::TrajectoryUpdate),
        Just(MutationCase::TrajectoryDelete),
        Just(MutationCase::ScopeCreate),
        Just(MutationCase::ScopeUpdate),
        Just(MutationCase::ScopeClose),
        Just(MutationCase::ArtifactCreate),
        Just(MutationCase::NoteCreate),
        Just(MutationCase::TurnCreate),
        Just(MutationCase::AgentRegister),
        Just(MutationCase::AgentUpdate),
        Just(MutationCase::AgentUnregister),
        Just(MutationCase::LockAcquire),
        Just(MutationCase::LockRelease),
        Just(MutationCase::MessageSend),
        Just(MutationCase::MessageAcknowledge),
        Just(MutationCase::DelegationCreate),
        Just(MutationCase::DelegationAccept),
        Just(MutationCase::DelegationComplete),
        Just(MutationCase::HandoffCreate),
        Just(MutationCase::HandoffAccept),
        Just(MutationCase::HandoffComplete),
    ]
}

// ============================================================================
// SEED HELPERS
// ============================================================================

fn default_memory_access() -> MemoryAccessRequest {
    MemoryAccessRequest {
        read: vec![MemoryPermissionRequest {
            memory_type: "trajectory".to_string(),
            scope: "own".to_string(),
            filter: None,
        }],
        write: vec![MemoryPermissionRequest {
            memory_type: "artifact".to_string(),
            scope: "own".to_string(),
            filter: None,
        }],
    }
}

async fn seed_trajectory(
    db: &DbClient,
    name: &str,
    tenant_id: EntityId,
) -> Result<caliber_api::types::TrajectoryResponse, TestCaseError> {
    let req = CreateTrajectoryRequest {
        name: name.to_string(),
        description: Some("Seed trajectory".to_string()),
        parent_trajectory_id: None,
        agent_id: None,
        metadata: None,
    };
    db.trajectory_create(&req, tenant_id)
        .await
        .map_err(|e| TestCaseError::fail(format!("Failed to seed trajectory: {}", e.message)))
}

async fn seed_scope(
    db: &DbClient,
    trajectory_id: EntityId,
    tenant_id: EntityId,
) -> Result<caliber_api::types::ScopeResponse, TestCaseError> {
    let req = CreateScopeRequest {
        trajectory_id,
        parent_scope_id: None,
        name: "Seed scope".to_string(),
        purpose: Some("Seed scope for broadcast tests".to_string()),
        token_budget: 1000,
        metadata: None,
    };
    db.scope_create(&req, tenant_id)
        .await
        .map_err(|e| TestCaseError::fail(format!("Failed to seed scope: {}", e.message)))
}

async fn seed_agent(
    db: &DbClient,
    agent_type: &str,
    tenant_id: EntityId,
) -> Result<caliber_api::types::AgentResponse, TestCaseError> {
    let req = RegisterAgentRequest {
        agent_type: agent_type.to_string(),
        capabilities: vec!["coordination".to_string()],
        memory_access: default_memory_access(),
        can_delegate_to: vec![],
        reports_to: None,
    };
    db.agent_register(&req, tenant_id)
        .await
        .map_err(|e| TestCaseError::fail(format!("Failed to seed agent: {}", e.message)))
}

async fn seed_lock(
    db: &DbClient,
    holder_agent_id: EntityId,
    resource_id: EntityId,
    tenant_id: EntityId,
) -> Result<caliber_api::types::LockResponse, TestCaseError> {
    let req = AcquireLockRequest {
        resource_type: "trajectory".to_string(),
        resource_id,
        holder_agent_id,
        timeout_ms: 30_000,
        mode: "exclusive".to_string(),
    };
    db.lock_acquire(&req, tenant_id)
        .await
        .map_err(|e| TestCaseError::fail(format!("Failed to seed lock: {}", e.message)))
}

async fn seed_message(
    db: &DbClient,
    from_agent_id: EntityId,
    to_agent_id: EntityId,
    trajectory_id: Option<EntityId>,
    scope_id: Option<EntityId>,
    tenant_id: EntityId,
) -> Result<caliber_api::types::MessageResponse, TestCaseError> {
    let req = SendMessageRequest {
        from_agent_id,
        to_agent_id: Some(to_agent_id),
        to_agent_type: None,
        message_type: "TaskDelegation".to_string(),
        payload: r#"{"hello":"world"}"#.to_string(),
        trajectory_id,
        scope_id,
        artifact_ids: vec![],
        priority: "Normal".to_string(),
        expires_at: None,
    };
    db.message_send(&req, tenant_id)
        .await
        .map_err(|e| TestCaseError::fail(format!("Failed to seed message: {}", e.message)))
}

async fn seed_delegation(
    db: &DbClient,
    from_agent_id: EntityId,
    to_agent_id: EntityId,
    trajectory_id: EntityId,
    scope_id: EntityId,
    tenant_id: EntityId,
) -> Result<caliber_api::types::DelegationResponse, TestCaseError> {
    let req = CreateDelegationRequest {
        from_agent_id,
        to_agent_id,
        trajectory_id,
        scope_id,
        task_description: "Seed delegation".to_string(),
        expected_completion: None,
        context: None,
    };
    db.delegation_create(&req, tenant_id)
        .await
        .map_err(|e| TestCaseError::fail(format!("Failed to seed delegation: {}", e.message)))
}

async fn seed_handoff(
    db: &DbClient,
    from_agent_id: EntityId,
    to_agent_id: EntityId,
    trajectory_id: EntityId,
    scope_id: EntityId,
    tenant_id: EntityId,
) -> Result<caliber_api::types::HandoffResponse, TestCaseError> {
    let req = CreateHandoffRequest {
        from_agent_id,
        to_agent_id,
        trajectory_id,
        scope_id,
        reason: "CapabilityMismatch".to_string(),
        context_snapshot: vec![1, 2, 3],
    };
    db.handoff_create(&req, tenant_id)
        .await
        .map_err(|e| TestCaseError::fail(format!("Failed to seed handoff: {}", e.message)))
}

// ============================================================================
// EXPECTED EVENT ASSERTIONS
// ============================================================================

#[derive(Debug)]
enum ExpectedEvent {
    TrajectoryCreated,
    TrajectoryUpdated(EntityId),
    TrajectoryDeleted(EntityId),
    ScopeCreated,
    ScopeUpdated(EntityId),
    ScopeClosed(EntityId),
    ArtifactCreated,
    NoteCreated,
    TurnCreated,
    AgentRegistered,
    AgentStatusChanged(EntityId, String),
    AgentUnregistered(EntityId),
    LockAcquired,
    LockReleased(EntityId),
    MessageSent,
    MessageAcknowledged(EntityId),
    DelegationCreated,
    DelegationAccepted(EntityId),
    DelegationCompleted(EntityId),
    HandoffCreated,
    HandoffAccepted(EntityId),
    HandoffCompleted(EntityId),
}

fn assert_event(expected: ExpectedEvent, actual: WsEvent) -> Result<(), TestCaseError> {
    let nil_id: EntityId = Uuid::nil().into();

    match (expected, actual) {
        (ExpectedEvent::TrajectoryCreated, WsEvent::TrajectoryCreated { trajectory }) => {
            prop_assert_ne!(trajectory.trajectory_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::TrajectoryUpdated(id), WsEvent::TrajectoryUpdated { trajectory }) => {
            prop_assert_eq!(trajectory.trajectory_id, id);
            Ok(())
        }
        (ExpectedEvent::TrajectoryDeleted(id), WsEvent::TrajectoryDeleted { tenant_id: _, id: event_id }) => {
            prop_assert_eq!(event_id, id);
            Ok(())
        }
        (ExpectedEvent::ScopeCreated, WsEvent::ScopeCreated { scope }) => {
            prop_assert_ne!(scope.scope_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::ScopeUpdated(id), WsEvent::ScopeUpdated { scope }) => {
            prop_assert_eq!(scope.scope_id, id);
            Ok(())
        }
        (ExpectedEvent::ScopeClosed(id), WsEvent::ScopeClosed { scope }) => {
            prop_assert_eq!(scope.scope_id, id);
            Ok(())
        }
        (ExpectedEvent::ArtifactCreated, WsEvent::ArtifactCreated { artifact }) => {
            prop_assert_ne!(artifact.artifact_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::NoteCreated, WsEvent::NoteCreated { note }) => {
            prop_assert_ne!(note.note_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::TurnCreated, WsEvent::TurnCreated { turn }) => {
            prop_assert_ne!(turn.turn_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::AgentRegistered, WsEvent::AgentRegistered { agent }) => {
            prop_assert_ne!(agent.agent_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::AgentStatusChanged(id, status), WsEvent::AgentStatusChanged { tenant_id: _, agent_id, status: actual_status }) => {
            prop_assert_eq!(agent_id, id);
            prop_assert_eq!(actual_status.to_lowercase(), status.to_lowercase());
            Ok(())
        }
        (ExpectedEvent::AgentUnregistered(id), WsEvent::AgentUnregistered { tenant_id: _, id: event_id }) => {
            prop_assert_eq!(event_id, id);
            Ok(())
        }
        (ExpectedEvent::LockAcquired, WsEvent::LockAcquired { lock }) => {
            prop_assert_ne!(lock.lock_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::LockReleased(id), WsEvent::LockReleased { tenant_id: _, lock_id }) => {
            prop_assert_eq!(lock_id, id);
            Ok(())
        }
        (ExpectedEvent::MessageSent, WsEvent::MessageSent { message }) => {
            prop_assert_ne!(message.message_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::MessageAcknowledged(id), WsEvent::MessageAcknowledged { tenant_id: _, message_id }) => {
            prop_assert_eq!(message_id, id);
            Ok(())
        }
        (ExpectedEvent::DelegationCreated, WsEvent::DelegationCreated { delegation }) => {
            prop_assert_ne!(delegation.delegation_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::DelegationAccepted(id), WsEvent::DelegationAccepted { tenant_id: _, delegation_id }) => {
            prop_assert_eq!(delegation_id, id);
            Ok(())
        }
        (ExpectedEvent::DelegationCompleted(id), WsEvent::DelegationCompleted { delegation }) => {
            prop_assert_eq!(delegation.delegation_id, id);
            Ok(())
        }
        (ExpectedEvent::HandoffCreated, WsEvent::HandoffCreated { handoff }) => {
            prop_assert_ne!(handoff.handoff_id, nil_id);
            Ok(())
        }
        (ExpectedEvent::HandoffAccepted(id), WsEvent::HandoffAccepted { tenant_id: _, handoff_id }) => {
            prop_assert_eq!(handoff_id, id);
            Ok(())
        }
        (ExpectedEvent::HandoffCompleted(id), WsEvent::HandoffCompleted { handoff }) => {
            prop_assert_eq!(handoff.handoff_id, id);
            Ok(())
        }
        (expected, actual) => Err(TestCaseError::fail(format!(
            "Unexpected event. Expected {:?}, got {:?}",
            expected, actual
        ))),
    }
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: Mutation Broadcast**
    ///
    /// For any supported mutation, a corresponding WebSocket event SHALL be
    /// broadcast within 100ms.
    #[test]
    fn prop_mutation_broadcast(case in mutation_case_strategy()) {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(async {
            let db = test_db_support::test_db_client();
            let auth = test_auth_context();
            let ws = test_ws_state();
            let pcp = test_pcp_support::test_pcp_runtime();
            let mut rx = ws.subscribe();

            let trajectory_state = Arc::new(trajectory::TrajectoryState::new(db.clone(), ws.clone()));
            let scope_state = Arc::new(scope::ScopeState::new(db.clone(), ws.clone(), pcp.clone()));
            let artifact_state = Arc::new(artifact::ArtifactState::new(db.clone(), ws.clone()));
            let note_state = Arc::new(note::NoteState::new(db.clone(), ws.clone()));
            let turn_state = Arc::new(turn::TurnState::new(db.clone(), ws.clone(), pcp));
            let agent_state = Arc::new(agent::AgentState::new(db.clone(), ws.clone()));
            let lock_state = Arc::new(lock::LockState::new(db.clone(), ws.clone()));
            let message_state = Arc::new(message::MessageState::new(db.clone(), ws.clone()));
            let delegation_state = Arc::new(delegation::DelegationState::new(db.clone(), ws.clone()));
            let handoff_state = Arc::new(handoff::HandoffState::new(db.clone(), ws.clone()));

            let expected = match case {
                MutationCase::TrajectoryCreate => {
                    let req = CreateTrajectoryRequest {
                        name: "Broadcast Trajectory".to_string(),
                        description: Some("Broadcast test".to_string()),
                        parent_trajectory_id: None,
                        agent_id: None,
                        metadata: None,
                    };
                    trajectory::create_trajectory(State(trajectory_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::TrajectoryCreated
                }
                MutationCase::TrajectoryUpdate => {
                    let trajectory = seed_trajectory(&db, "Seed Trajectory Update", auth.tenant_id).await?;
                    let req = UpdateTrajectoryRequest {
                        name: Some("Updated".to_string()),
                        description: None,
                        status: Some(TrajectoryStatus::Completed),
                        metadata: None,
                    };
                    trajectory::update_trajectory(
                        State(trajectory_state),
                        AuthExtractor(auth.clone()),
                        Path(trajectory.trajectory_id),
                        Json(req),
                    )
                    .await?;
                    ExpectedEvent::TrajectoryUpdated(trajectory.trajectory_id)
                }
                MutationCase::TrajectoryDelete => {
                    let trajectory = seed_trajectory(&db, "Seed Trajectory Delete", auth.tenant_id).await?;
                    trajectory::delete_trajectory(
                        State(trajectory_state),
                        AuthExtractor(auth.clone()),
                        Path(trajectory.trajectory_id),
                    )
                    .await?;
                    ExpectedEvent::TrajectoryDeleted(trajectory.trajectory_id)
                }
                MutationCase::ScopeCreate => {
                    let trajectory = seed_trajectory(&db, "Seed Trajectory Scope Create", auth.tenant_id).await?;
                    let req = CreateScopeRequest {
                        trajectory_id: trajectory.trajectory_id,
                        parent_scope_id: None,
                        name: "Broadcast Scope".to_string(),
                        purpose: Some("Scope create broadcast".to_string()),
                        token_budget: 800,
                        metadata: None,
                    };
                    scope::create_scope(State(scope_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::ScopeCreated
                }
                MutationCase::ScopeUpdate => {
                    let trajectory = seed_trajectory(&db, "Seed Trajectory Scope Update", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    let req = UpdateScopeRequest {
                        name: Some("Scope Updated".to_string()),
                        purpose: Some("Updated purpose".to_string()),
                        token_budget: Some(1200),
                        metadata: None,
                    };
                    scope::update_scope(
                        State(scope_state),
                        AuthExtractor(auth.clone()),
                        Path(scope.scope_id),
                        Json(req),
                    )
                    .await?;
                    ExpectedEvent::ScopeUpdated(scope.scope_id)
                }
                MutationCase::ScopeClose => {
                    let trajectory = seed_trajectory(&db, "Seed Trajectory Scope Close", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    scope::close_scope(State(scope_state), AuthExtractor(auth.clone()), Path(scope.scope_id)).await?;
                    ExpectedEvent::ScopeClosed(scope.scope_id)
                }
                MutationCase::ArtifactCreate => {
                    let trajectory = seed_trajectory(&db, "Seed Trajectory Artifact", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    let req = CreateArtifactRequest {
                        trajectory_id: trajectory.trajectory_id,
                        scope_id: scope.scope_id,
                        artifact_type: ArtifactType::Fact,
                        name: "Broadcast Artifact".to_string(),
                        content: "Artifact content".to_string(),
                        source_turn: 0,
                        extraction_method: ExtractionMethod::Explicit,
                        confidence: Some(0.9),
                        ttl: TTL::Persistent,
                        metadata: None,
                    };
                    artifact::create_artifact(State(artifact_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::ArtifactCreated
                }
                MutationCase::NoteCreate => {
                    let trajectory = seed_trajectory(&db, "Seed Trajectory Note", auth.tenant_id).await?;
                    let req = CreateNoteRequest {
                        note_type: NoteType::Fact,
                        title: "Broadcast Note".to_string(),
                        content: "Note content".to_string(),
                        source_trajectory_ids: vec![trajectory.trajectory_id],
                        source_artifact_ids: vec![],
                        ttl: TTL::LongTerm,
                        metadata: None,
                    };
                    note::create_note(State(note_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::NoteCreated
                }
                MutationCase::TurnCreate => {
                    let trajectory = seed_trajectory(&db, "Seed Trajectory Turn", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    let req = CreateTurnRequest {
                        scope_id: scope.scope_id,
                        sequence: 1,
                        role: TurnRole::User,
                        content: "Hello".to_string(),
                        token_count: 5,
                        tool_calls: None,
                        tool_results: None,
                        metadata: None,
                    };
                    turn::create_turn(State(turn_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::TurnCreated
                }
                MutationCase::AgentRegister => {
                    let req = RegisterAgentRequest {
                        agent_type: "planner".to_string(),
                        capabilities: vec!["planning".to_string()],
                        memory_access: default_memory_access(),
                        can_delegate_to: vec![],
                        reports_to: None,
                    };
                    agent::register_agent(State(agent_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::AgentRegistered
                }
                MutationCase::AgentUpdate => {
                    let agent = seed_agent(&db, "seed-updater", auth.tenant_id).await?;
                    let req = UpdateAgentRequest {
                        status: Some("active".to_string()),
                        current_trajectory_id: None,
                        current_scope_id: None,
                        capabilities: None,
                        memory_access: None,
                    };
                    agent::update_agent(
                        State(agent_state),
                        AuthExtractor(auth.clone()),
                        Path(agent.agent_id),
                        Json(req),
                    )
                    .await?;
                    ExpectedEvent::AgentStatusChanged(agent.agent_id, "active".to_string())
                }
                MutationCase::AgentUnregister => {
                    let agent = seed_agent(&db, "seed-unregister", auth.tenant_id).await?;
                    agent::unregister_agent(
                        State(agent_state),
                        AuthExtractor(auth.clone()),
                        Path(agent.agent_id),
                    )
                    .await?;
                    ExpectedEvent::AgentUnregistered(agent.agent_id)
                }
                MutationCase::LockAcquire => {
                    let agent = seed_agent(&db, "lock-holder", auth.tenant_id).await?;
                    let resource_id: EntityId = Uuid::now_v7().into();
                    let req = AcquireLockRequest {
                        resource_type: "trajectory".to_string(),
                        resource_id,
                        holder_agent_id: agent.agent_id,
                        timeout_ms: 30_000,
                        mode: "exclusive".to_string(),
                    };
                    lock::acquire_lock(State(lock_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::LockAcquired
                }
                MutationCase::LockRelease => {
                    let agent = seed_agent(&db, "lock-releaser", auth.tenant_id).await?;
                    let resource_id: EntityId = Uuid::now_v7().into();
                    let lock = seed_lock(&db, agent.agent_id, resource_id, auth.tenant_id).await?;
                    lock::release_lock(
                        State(lock_state),
                        AuthExtractor(auth.clone()),
                        Path(lock.lock_id),
                    )
                    .await?;
                    ExpectedEvent::LockReleased(lock.lock_id)
                }
                MutationCase::MessageSend => {
                    let sender = seed_agent(&db, "sender", auth.tenant_id).await?;
                    let receiver = seed_agent(&db, "receiver", auth.tenant_id).await?;
                    let req = SendMessageRequest {
                        from_agent_id: sender.agent_id,
                        to_agent_id: Some(receiver.agent_id),
                        to_agent_type: None,
                        message_type: "TaskDelegation".to_string(),
                        payload: r#"{"task":"ping"}"#.to_string(),
                        trajectory_id: None,
                        scope_id: None,
                        artifact_ids: vec![],
                        priority: "Normal".to_string(),
                        expires_at: None,
                    };
                    message::send_message(State(message_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::MessageSent
                }
                MutationCase::MessageAcknowledge => {
                    let sender = seed_agent(&db, "ack-sender", auth.tenant_id).await?;
                    let receiver = seed_agent(&db, "ack-receiver", auth.tenant_id).await?;
                    let message = seed_message(&db, sender.agent_id, receiver.agent_id, None, None, auth.tenant_id).await?;
                    message::acknowledge_message(
                        State(message_state),
                        AuthExtractor(auth.clone()),
                        Path(message.message_id),
                    )
                    .await?;
                    ExpectedEvent::MessageAcknowledged(message.message_id)
                }
                MutationCase::DelegationCreate => {
                    let from_agent = seed_agent(&db, "delegator", auth.tenant_id).await?;
                    let to_agent = seed_agent(&db, "delegatee", auth.tenant_id).await?;
                    let trajectory = seed_trajectory(&db, "Delegation Trajectory", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    let req = CreateDelegationRequest {
                        from_agent_id: from_agent.agent_id,
                        to_agent_id: to_agent.agent_id,
                        trajectory_id: trajectory.trajectory_id,
                        scope_id: scope.scope_id,
                        task_description: "Delegate task".to_string(),
                        expected_completion: None,
                        context: None,
                    };
                    delegation::create_delegation(State(delegation_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::DelegationCreated
                }
                MutationCase::DelegationAccept => {
                    let from_agent = seed_agent(&db, "delegator-accept", auth.tenant_id).await?;
                    let to_agent = seed_agent(&db, "delegatee-accept", auth.tenant_id).await?;
                    let trajectory = seed_trajectory(&db, "Delegation Accept Trajectory", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    let delegation = seed_delegation(&db, from_agent.agent_id, to_agent.agent_id, trajectory.trajectory_id, scope.scope_id, auth.tenant_id).await?;
                    let req = delegation::AcceptDelegationRequest {
                        accepting_agent_id: to_agent.agent_id,
                    };
                    delegation::accept_delegation(
                        State(delegation_state),
                        AuthExtractor(auth.clone()),
                        Path(delegation.delegation_id),
                        Json(req),
                    )
                    .await?;
                    ExpectedEvent::DelegationAccepted(delegation.delegation_id)
                }
                MutationCase::DelegationComplete => {
                    let from_agent = seed_agent(&db, "delegator-complete", auth.tenant_id).await?;
                    let to_agent = seed_agent(&db, "delegatee-complete", auth.tenant_id).await?;
                    let trajectory = seed_trajectory(&db, "Delegation Complete Trajectory", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    let delegation = seed_delegation(&db, from_agent.agent_id, to_agent.agent_id, trajectory.trajectory_id, scope.scope_id, auth.tenant_id).await?;
                    db.delegation_accept(delegation.delegation_id, to_agent.agent_id).await?;
                    let req = delegation::CompleteDelegationRequest {
                        result: caliber_api::types::DelegationResultResponse {
                            status: "Success".to_string(),
                            output: Some("Done".to_string()),
                            artifacts: vec![],
                            error: None,
                        },
                    };
                    delegation::complete_delegation(
                        State(delegation_state),
                        AuthExtractor(auth.clone()),
                        Path(delegation.delegation_id),
                        Json(req),
                    )
                    .await?;
                    ExpectedEvent::DelegationCompleted(delegation.delegation_id)
                }
                MutationCase::HandoffCreate => {
                    let from_agent = seed_agent(&db, "handoff-from", auth.tenant_id).await?;
                    let to_agent = seed_agent(&db, "handoff-to", auth.tenant_id).await?;
                    let trajectory = seed_trajectory(&db, "Handoff Trajectory", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    let req = CreateHandoffRequest {
                        from_agent_id: from_agent.agent_id,
                        to_agent_id: to_agent.agent_id,
                        trajectory_id: trajectory.trajectory_id,
                        scope_id: scope.scope_id,
                        reason: "CapabilityMismatch".to_string(),
                        context_snapshot: vec![1, 2, 3],
                    };
                    handoff::create_handoff(State(handoff_state), AuthExtractor(auth.clone()), Json(req)).await?;
                    ExpectedEvent::HandoffCreated
                }
                MutationCase::HandoffAccept => {
                    let from_agent = seed_agent(&db, "handoff-accept-from", auth.tenant_id).await?;
                    let to_agent = seed_agent(&db, "handoff-accept-to", auth.tenant_id).await?;
                    let trajectory = seed_trajectory(&db, "Handoff Accept Trajectory", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    let handoff = seed_handoff(&db, from_agent.agent_id, to_agent.agent_id, trajectory.trajectory_id, scope.scope_id, auth.tenant_id).await?;
                    let req = handoff::AcceptHandoffRequest {
                        accepting_agent_id: to_agent.agent_id,
                    };
                    handoff::accept_handoff(
                        State(handoff_state),
                        AuthExtractor(auth.clone()),
                        Path(handoff.handoff_id),
                        Json(req),
                    )
                    .await?;
                    ExpectedEvent::HandoffAccepted(handoff.handoff_id)
                }
                MutationCase::HandoffComplete => {
                    let from_agent = seed_agent(&db, "handoff-complete-from", auth.tenant_id).await?;
                    let to_agent = seed_agent(&db, "handoff-complete-to", auth.tenant_id).await?;
                    let trajectory = seed_trajectory(&db, "Handoff Complete Trajectory", auth.tenant_id).await?;
                    let scope = seed_scope(&db, trajectory.trajectory_id, auth.tenant_id).await?;
                    let handoff = seed_handoff(&db, from_agent.agent_id, to_agent.agent_id, trajectory.trajectory_id, scope.scope_id, auth.tenant_id).await?;
                    db.handoff_accept(handoff.handoff_id, to_agent.agent_id).await?;
                    handoff::complete_handoff(State(handoff_state), AuthExtractor(auth.clone()), Path(handoff.handoff_id)).await?;
                    ExpectedEvent::HandoffCompleted(handoff.handoff_id)
                }
            };

            let event = timeout(Duration::from_millis(100), rx.recv()).await
                .map_err(|_| TestCaseError::fail("Timed out waiting for broadcast event"))?;
            let event = event
                .map_err(|err| TestCaseError::fail(format!("Broadcast channel error: {:?}", err)))?;

            assert_event(expected, event)?;

            Ok::<(), TestCaseError>(())
        })?;
    }
}
