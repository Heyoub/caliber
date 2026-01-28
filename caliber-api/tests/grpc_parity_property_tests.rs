#![cfg(feature = "db-tests")]
//! Property-Based Tests for REST ↔ gRPC Parity
//!
//! **Property 2: REST-gRPC Parity**
//!
//! For any REST endpoint, there SHALL exist an equivalent gRPC method that
//! accepts equivalent input and returns equivalent output.
//!
//! **Validates: Requirements 1.1, 1.2**

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use caliber_api::proto::scope_service_server::ScopeService;
use caliber_api::proto::trajectory_service_server::TrajectoryService;
use caliber_api::{
    extractors::PathId,
    grpc::{self, proto},
    middleware::AuthExtractor,
    routes::{scope, trajectory},
    types::{CreateScopeRequest, CreateTrajectoryRequest, ScopeResponse, TrajectoryResponse},
};
use caliber_core::{EntityIdType, ScopeId, TenantId, TrajectoryId};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use serde::de::DeserializeOwned;
use tokio::runtime::Runtime;
use tonic::metadata::MetadataValue;
use tonic::Request;
use uuid::Uuid;
#[path = "support/auth.rs"]
mod test_auth_support;
#[path = "support/db.rs"]
mod test_db_support;
#[path = "support/event_dag.rs"]
mod test_event_dag_support;
#[path = "support/ws.rs"]
mod test_ws_support;
use test_auth_support::test_auth_context;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

async fn extract_json<T: DeserializeOwned>(
    response: impl IntoResponse,
) -> Result<T, TestCaseError> {
    let response = response.into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| TestCaseError::fail(format!("Failed to read response body: {:?}", e)))?;
    serde_json::from_slice(&body)
        .map_err(|e| TestCaseError::fail(format!("Failed to parse JSON response: {}", e)))
}

fn request_with_tenant<T>(payload: T, tenant_id: TenantId) -> Result<Request<T>, TestCaseError> {
    let mut request = Request::new(payload);
    let tenant_header = MetadataValue::try_from(tenant_id.to_string()).map_err(|e| {
        TestCaseError::fail(format!("Failed to build tenant metadata header: {}", e))
    })?;
    request.metadata_mut().insert("x-tenant-id", tenant_header);
    Ok(request)
}

fn test_runtime() -> Result<Runtime, TestCaseError> {
    Runtime::new().map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))
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
    prop_assert_eq!(
        grpc.completed_at,
        rest.completed_at.map(|t| t.timestamp_millis())
    );
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
    prop_oneof![Just(None), "[A-Za-z0-9 _-]{5,50}".prop_map(Some),]
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
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_support::test_db_client();
            let auth = test_auth_context();
            let ws = test_ws_support::test_ws_state(64);
            let event_dag = test_event_dag_support::test_event_dag();
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
                trajectory::create_trajectory(
                    State(db.clone()),
                    State(ws.clone()),
                    State(event_dag.clone()),
                    AuthExtractor(auth.clone()),
                    Json(rest_req),
                )
                .await?,
            )
            .await?;

            let grpc_get = grpc_service
                .get_trajectory(request_with_tenant(
                    proto::GetTrajectoryRequest {
                    trajectory_id: rest_created.trajectory_id.to_string(),
                },
                    auth.tenant_id,
                )?)
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
                )?)
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC create failed: {}", e)))?
                .into_inner();

            let trajectory_id = Uuid::parse_str(&grpc_created.trajectory_id)
                .map_err(|e| TestCaseError::fail(format!("invalid uuid: {}", e)))?;
            let trajectory_id = TrajectoryId::new(trajectory_id);
            let rest_get: TrajectoryResponse = extract_json(
                trajectory::get_trajectory(
                    State(db.clone()),
                    AuthExtractor(auth.clone()),
                    PathId(trajectory_id),
                )
                .await?,
            )
            .await?;

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
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_support::test_db_client();
            let auth = test_auth_context();
            let ws = test_ws_support::test_ws_state(64);
            let event_dag = test_event_dag_support::test_event_dag();
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
                trajectory::create_trajectory(
                    State(db.clone()),
                    State(ws.clone()),
                    State(event_dag.clone()),
                    AuthExtractor(auth.clone()),
                    Json(trajectory_req),
                )
                .await?,
            )
            .await?;

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
                scope::create_scope(
                    State(db.clone()),
                    State(ws.clone()),
                    State(event_dag.clone()),
                    AuthExtractor(auth.clone()),
                    Json(rest_scope_req),
                )
                .await?,
            )
            .await?;

            let grpc_get = grpc_service
                .get_scope(request_with_tenant(
                    proto::GetScopeRequest {
                    scope_id: rest_created.scope_id.to_string(),
                },
                    auth.tenant_id,
                )?)
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
                )?)
                .await
                .map_err(|e| TestCaseError::fail(format!("gRPC create failed: {}", e)))?
                .into_inner();

            let scope_id = Uuid::parse_str(&grpc_created.scope_id)
                .map_err(|e| TestCaseError::fail(format!("invalid uuid: {}", e)))?;
            let scope_id = ScopeId::new(scope_id);
            let rest_get: ScopeResponse = extract_json(
                scope::get_scope(
                    State(db.clone()),
                    AuthExtractor(auth.clone()),
                    PathId(scope_id),
                )
                .await?,
            )
            .await?;

            assert_scope_parity(&rest_get, &grpc_created)?;

            Ok::<(), TestCaseError>(())
        })?;
    }
}
