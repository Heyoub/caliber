//! Property-Based Tests for REST ↔ gRPC Parity
//!
//! **Property 2: REST-gRPC Parity**
//!
//! For any REST endpoint, there SHALL exist an equivalent gRPC method that
//! accepts equivalent input and returns equivalent output.
//!
//! **Validates: Requirements 1.1, 1.2**

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use caliber_api::{
    db::{DbClient, DbConfig},
    grpc::{self, proto},
    routes::{scope, trajectory},
    types::{CreateScopeRequest, CreateTrajectoryRequest, ScopeResponse, TrajectoryResponse},
    ws::WsState,
};
use caliber_api::proto::scope_service_server::ScopeService;
use caliber_api::proto::trajectory_service_server::TrajectoryService;
use caliber_pcp::{
    AntiSprawlConfig, ConflictResolution, ContextDagConfig, DosageConfig, GroundingConfig,
    LintingConfig, PCPConfig, PCPRuntime, PruneStrategy, RecoveryConfig, RecoveryFrequency,
    StalenessConfig,
};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tonic::Request;
use uuid::Uuid;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

/// Create a test database client.
fn test_db_client() -> DbClient {
    let config = DbConfig::from_env();
    DbClient::from_config(&config).expect("Failed to create database client")
}

fn make_test_pcp_config() -> PCPConfig {
    PCPConfig {
        context_dag: ContextDagConfig {
            max_depth: 10,
            prune_strategy: PruneStrategy::OldestFirst,
        },
        recovery: RecoveryConfig {
            enabled: true,
            frequency: RecoveryFrequency::OnScopeClose,
            max_checkpoints: 5,
        },
        dosage: DosageConfig {
            max_tokens_per_scope: 8000,
            max_artifacts_per_scope: 100,
            max_notes_per_trajectory: 500,
        },
        anti_sprawl: AntiSprawlConfig {
            max_trajectory_depth: 5,
            max_concurrent_scopes: 10,
        },
        grounding: GroundingConfig {
            require_artifact_backing: false,
            contradiction_threshold: 0.85,
            conflict_resolution: ConflictResolution::LastWriteWins,
        },
        linting: LintingConfig {
            max_artifact_size: 1024 * 1024,
            min_confidence_threshold: 0.3,
        },
        staleness: StalenessConfig { stale_hours: 24 * 30 },
    }
}

fn test_pcp_runtime() -> Arc<PCPRuntime> {
    Arc::new(PCPRuntime::new(make_test_pcp_config()).expect("Failed to create PCP runtime"))
}

/// Create a WebSocket state for gRPC services (broadcasts are ignored here).
fn test_ws_state() -> Arc<WsState> {
    Arc::new(WsState::new(64))
}

async fn extract_json<T: DeserializeOwned>(response: impl IntoResponse) -> T {
    let response = response.into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&body).expect("Failed to parse JSON response")
}

fn assert_trajectory_parity(
    rest: &TrajectoryResponse,
    grpc: &proto::TrajectoryResponse,
) -> Result<(), TestCaseError> {
    let rest_trajectory_id = rest.trajectory_id.to_string();
    let rest_status = rest.status.to_string();
    let rest_parent = rest.parent_trajectory_id.as_ref().map(|id| id.to_string());
    let rest_root = rest.root_trajectory_id.as_ref().map(|id| id.to_string());
    let rest_agent = rest.agent_id.as_ref().map(|id| id.to_string());

    prop_assert_eq!(grpc.trajectory_id.as_str(), rest_trajectory_id.as_str());
    prop_assert_eq!(grpc.name.as_str(), rest.name.as_str());
    prop_assert_eq!(grpc.description.as_deref(), rest.description.as_deref());
    prop_assert_eq!(grpc.status.as_str(), rest_status.as_str());
    prop_assert_eq!(grpc.parent_trajectory_id.as_deref(), rest_parent.as_deref());
    prop_assert_eq!(grpc.root_trajectory_id.as_deref(), rest_root.as_deref());
    prop_assert_eq!(grpc.agent_id.as_deref(), rest_agent.as_deref());
    prop_assert_eq!(grpc.created_at, rest.created_at.timestamp_millis());
    prop_assert_eq!(grpc.updated_at, rest.updated_at.timestamp_millis());
    prop_assert_eq!(grpc.completed_at, rest.completed_at.map(|t| t.timestamp_millis()));
    Ok(())
}

fn assert_scope_parity(
    rest: &ScopeResponse,
    grpc: &proto::ScopeResponse,
) -> Result<(), TestCaseError> {
    let rest_scope_id = rest.scope_id.to_string();
    let rest_trajectory_id = rest.trajectory_id.to_string();
    let rest_parent = rest.parent_scope_id.as_ref().map(|id| id.to_string());

    prop_assert_eq!(grpc.scope_id.as_str(), rest_scope_id.as_str());
    prop_assert_eq!(grpc.trajectory_id.as_str(), rest_trajectory_id.as_str());
    prop_assert_eq!(grpc.parent_scope_id.as_deref(), rest_parent.as_deref());
    prop_assert_eq!(grpc.name.as_str(), rest.name.as_str());
    prop_assert_eq!(grpc.purpose.as_deref(), rest.purpose.as_deref());
    prop_assert_eq!(grpc.is_active, rest.is_active);
    prop_assert_eq!(grpc.created_at, rest.created_at.timestamp_millis());
    prop_assert_eq!(grpc.closed_at, rest.closed_at.map(|t| t.timestamp_millis()));
    prop_assert_eq!(grpc.token_budget, rest.token_budget);
    prop_assert_eq!(grpc.tokens_used, rest.tokens_used);
    Ok(())
}

// ============================================================================
// STRATEGIES
// ============================================================================

fn name_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 _-]{3,30}"
        .prop_map(|s| s.trim().to_string())
        .prop_filter("name cannot be empty after trim", |s| !s.is_empty())
}

fn description_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        "[A-Za-z0-9 _-]{5,50}".prop_map(Some),
    ]
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// REST create ↔ gRPC get parity for trajectories, and vice-versa.
    #[test]
    fn prop_trajectory_rest_grpc_parity(
        name in name_strategy(),
        description in description_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = test_db_client();
            let ws = test_ws_state();
            let rest_state = Arc::new(trajectory::TrajectoryState::new(db.clone(), ws.clone()));
            let grpc_service = grpc::TrajectoryServiceImpl::new(db.clone(), ws.clone());

            // REST create → gRPC get
            let rest_req = CreateTrajectoryRequest {
                name: name.clone(),
                description: description.clone(),
                parent_trajectory_id: None,
                agent_id: None,
                metadata: None,
            };
            let rest_created: TrajectoryResponse = extract_json(
                trajectory::create_trajectory(State(rest_state.clone()), Json(rest_req)).await?
            ).await;

            let grpc_get = grpc_service
                .get_trajectory(Request::new(proto::GetTrajectoryRequest {
                    trajectory_id: rest_created.trajectory_id.to_string(),
                }))
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC get failed: {}", e)))?
                .into_inner();

            assert_trajectory_parity(&rest_created, &grpc_get)?;

            // gRPC create → REST get
            let grpc_created = grpc_service
                .create_trajectory(Request::new(proto::CreateTrajectoryRequest {
                    name: format!("grpc-{}", name),
                    description,
                    parent_trajectory_id: None,
                    agent_id: None,
                    metadata: None,
                }))
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC create failed: {}", e)))?
                .into_inner();

            let rest_get: TrajectoryResponse = extract_json(
                trajectory::get_trajectory(
                    State(rest_state),
                    Path(Uuid::parse_str(&grpc_created.trajectory_id).expect("invalid uuid")),
                )
                .await?
            )
            .await;

            assert_trajectory_parity(&rest_get, &grpc_created)?;

            Ok(())
        })?;
    }

    /// REST create ↔ gRPC get parity for scopes, and vice-versa.
    #[test]
    fn prop_scope_rest_grpc_parity(
        name in name_strategy(),
        purpose in description_strategy(),
        token_budget in 100..5000i32,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = test_db_client();
            let ws = test_ws_state();
            let trajectory_state = Arc::new(trajectory::TrajectoryState::new(db.clone(), ws.clone()));
            let pcp = test_pcp_runtime();
            let scope_state = Arc::new(scope::ScopeState::new(db.clone(), ws.clone(), pcp));
            let grpc_service = grpc::ScopeServiceImpl::new(db.clone(), ws.clone());

            // Create a trajectory to attach scopes to
            let trajectory_req = CreateTrajectoryRequest {
                name: format!("scope-parent-{}", name),
                description: Some("Scope parent".to_string()),
                parent_trajectory_id: None,
                agent_id: None,
                metadata: None,
            };
            let trajectory: TrajectoryResponse = extract_json(
                trajectory::create_trajectory(State(trajectory_state), Json(trajectory_req)).await?
            ).await;

            // REST create → gRPC get
            let rest_scope_req = CreateScopeRequest {
                trajectory_id: trajectory.trajectory_id,
                parent_scope_id: None,
                name: name.clone(),
                purpose: purpose.clone(),
                token_budget,
                metadata: None,
            };
            let rest_created: ScopeResponse = extract_json(
                scope::create_scope(State(scope_state.clone()), Json(rest_scope_req)).await?
            ).await;

            let grpc_get = grpc_service
                .get_scope(Request::new(proto::GetScopeRequest {
                    scope_id: rest_created.scope_id.to_string(),
                }))
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC get failed: {}", e)))?
                .into_inner();

            assert_scope_parity(&rest_created, &grpc_get)?;

            // gRPC create → REST get
            let grpc_created = grpc_service
                .create_scope(Request::new(proto::CreateScopeRequest {
                    trajectory_id: trajectory.trajectory_id.to_string(),
                    parent_scope_id: None,
                    name: format!("grpc-{}", name),
                    purpose,
                    token_budget,
                    metadata: None,
                }))
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC create failed: {}", e)))?
                .into_inner();

            let rest_get: ScopeResponse = extract_json(
                scope::get_scope(
                    State(scope_state),
                    Path(Uuid::parse_str(&grpc_created.scope_id).expect("invalid uuid")),
                )
                .await?
            )
            .await;

            assert_scope_parity(&rest_get, &grpc_created)?;

            Ok(())
        })?;
    }
}

mod grpc_ws_parity_tests {
//! Property-Based Tests for REST ↔ gRPC Parity
//!
//! **Property 2: REST-gRPC Parity**
//!
//! For any REST endpoint, there SHALL exist an equivalent gRPC method that
//! accepts equivalent input and returns equivalent output.
//!
//! **Validates: Requirements 1.1, 1.2**

use axum::extract::State;
use axum::Json;
use caliber_api::{
    db::{DbClient, DbConfig},
    grpc::{proto, TrajectoryServiceImpl},
    routes::trajectory,
    types::CreateTrajectoryRequest,
    WsState,
};
use caliber_api::proto::trajectory_service_server::TrajectoryService;
use proptest::prelude::*;
use std::sync::Arc;
use tonic::Request;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};
async fn recv_trajectory_event(
    rx: &mut broadcast::Receiver<caliber_api::events::WsEvent>,
) -> caliber_api::types::TrajectoryResponse {
    match timeout(Duration::from_millis(200), rx.recv()).await {
        Ok(Ok(caliber_api::events::WsEvent::TrajectoryCreated { trajectory })) => trajectory,
        Ok(Ok(other)) => panic!("Expected TrajectoryCreated, got {:?}", other),
        Ok(Err(err)) => panic!("Broadcast recv error: {:?}", err),
        Err(_) => panic!("Timed out waiting for TrajectoryCreated event"),
    }
}
}

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

fn test_db_client() -> DbClient {
    let config = DbConfig::from_env();
    DbClient::from_config(&config).expect("Failed to create database client")
}

// ============================================================================
// PROPERTY TEST STRATEGIES
// ============================================================================

fn trajectory_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 _-]{3,30}".prop_map(|s| s.trim().to_string())
}

fn description_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        "[a-zA-Z0-9 ,.]{10,60}".prop_map(|s| Some(s.trim().to_string())),
    ]
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Property 2: REST-gRPC Parity (Trajectory Create)**
    #[test]
    fn prop_rest_grpc_parity_trajectory_create(
        name in trajectory_name_strategy(),
        description in description_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = test_db_client();

            // REST handler setup
            let ws_rest = Arc::new(WsState::new(50));
            let mut rx_rest = ws_rest.subscribe();
            let rest_state = Arc::new(trajectory::TrajectoryState::new(db.clone(), ws_rest.clone()));

            // gRPC service setup
            let ws_grpc = Arc::new(WsState::new(50));
            let grpc_service = TrajectoryServiceImpl::new(db.clone(), ws_grpc);

            // REST create
            let rest_req = CreateTrajectoryRequest {
                name: name.clone(),
                description: description.clone(),
                parent_trajectory_id: None,
                agent_id: None,
                metadata: None,
            };
            let _ = trajectory::create_trajectory(State(rest_state), Json(rest_req)).await?;
            let rest_trajectory = recv_trajectory_event(&mut rx_rest).await;

            // gRPC create
            let grpc_req = proto::CreateTrajectoryRequest {
                name: name.clone(),
                description: description.clone(),
                parent_trajectory_id: None,
                agent_id: None,
                metadata: None,
            };
            let grpc_resp = grpc_service
                .create_trajectory(Request::new(grpc_req))
                .await
                .expect("gRPC create should succeed")
                .into_inner();

            // Parity checks: core fields align across REST and gRPC
            prop_assert_eq!(grpc_resp.name.as_str(), rest_trajectory.name.as_str());
            prop_assert_eq!(grpc_resp.description.as_deref(), rest_trajectory.description.as_deref());
            prop_assert!(!grpc_resp.status.is_empty(), "gRPC status should be set");
            prop_assert!(!grpc_resp.trajectory_id.is_empty(), "gRPC id should be set");

            Ok(())
        })?;
    }
}
