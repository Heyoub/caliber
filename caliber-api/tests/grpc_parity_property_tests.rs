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
    grpc::{self, proto},
    middleware::AuthExtractor,
    routes::{scope, trajectory},
    types::{CreateScopeRequest, CreateTrajectoryRequest, ScopeResponse, TrajectoryResponse},
};
use caliber_api::proto::scope_service_server::ScopeService;
use caliber_api::proto::trajectory_service_server::TrajectoryService;
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tonic::metadata::MetadataValue;
use tonic::Request;
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

async fn extract_json<T: DeserializeOwned>(response: impl IntoResponse) -> T {
    let response = response.into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&body).expect("Failed to parse JSON response")
}

fn request_with_tenant<T>(payload: T, tenant_id: caliber_core::EntityId) -> Request<T> {
    let mut request = Request::new(payload);
    let tenant_header = MetadataValue::try_from(tenant_id.to_string())
        .expect("Failed to build tenant metadata header");
    request.metadata_mut().insert("x-tenant-id", tenant_header);
    request
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
            let db = test_db_support::test_db_client();
            let auth = test_auth_context();
            let ws = test_ws_support::test_ws_state(64);
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
                trajectory::create_trajectory(State(rest_state.clone()), AuthExtractor(auth.clone()), Json(rest_req)).await?
            ).await;

            let grpc_get = grpc_service
                .get_trajectory(request_with_tenant(
                    proto::GetTrajectoryRequest {
                    trajectory_id: rest_created.trajectory_id.to_string(),
                },
                    auth.tenant_id,
                ))
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC get failed: {}", e)))?
                .into_inner();

            assert_trajectory_parity(&rest_created, &grpc_get)?;

            // gRPC create → REST get
            let grpc_created = grpc_service
                .create_trajectory(request_with_tenant(
                    proto::CreateTrajectoryRequest {
                    name: format!("grpc-{}", name),
                    description,
                    parent_trajectory_id: None,
                    agent_id: None,
                    metadata: None,
                },
                    auth.tenant_id,
                ))
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC create failed: {}", e)))?
                .into_inner();

            let rest_get: TrajectoryResponse = extract_json(
                trajectory::get_trajectory(
                    State(rest_state),
                    AuthExtractor(auth.clone()),
                    Path(Uuid::parse_str(&grpc_created.trajectory_id).expect("invalid uuid")),
                )
                .await?
            )
            .await;

            assert_trajectory_parity(&rest_get, &grpc_created)?;

            Ok::<(), TestCaseError>(())
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
            let db = test_db_support::test_db_client();
            let auth = test_auth_context();
            let ws = test_ws_support::test_ws_state(64);
            let pcp = test_pcp_support::test_pcp_runtime();
            let trajectory_state = Arc::new(trajectory::TrajectoryState::new(db.clone(), ws.clone()));
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
                trajectory::create_trajectory(State(trajectory_state), AuthExtractor(auth.clone()), Json(trajectory_req)).await?
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
                scope::create_scope(State(scope_state.clone()), AuthExtractor(auth.clone()), Json(rest_scope_req)).await?
            ).await;

            let grpc_get = grpc_service
                .get_scope(request_with_tenant(
                    proto::GetScopeRequest {
                    scope_id: rest_created.scope_id.to_string(),
                },
                    auth.tenant_id,
                ))
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC get failed: {}", e)))?
                .into_inner();

            assert_scope_parity(&rest_created, &grpc_get)?;

            // gRPC create → REST get
            let grpc_created = grpc_service
                .create_scope(request_with_tenant(
                    proto::CreateScopeRequest {
                    trajectory_id: trajectory.trajectory_id.to_string(),
                    parent_scope_id: None,
                    name: format!("grpc-{}", name),
                    purpose,
                    token_budget,
                    metadata: None,
                },
                    auth.tenant_id,
                ))
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC create failed: {}", e)))?
                .into_inner();

            let rest_get: ScopeResponse = extract_json(
                scope::get_scope(
                    State(scope_state),
                    AuthExtractor(auth.clone()),
                    Path(Uuid::parse_str(&grpc_created.scope_id).expect("invalid uuid")),
                )
                .await?
            )
            .await;

            assert_scope_parity(&rest_get, &grpc_created)?;

            Ok::<(), TestCaseError>(())
        })?;
    }
}

