//! Property-Based Tests for REST ↔ gRPC Parity (WS broadcast variant)
//!
//! **Property 2: REST-gRPC Parity**
//!
//! For any REST endpoint, there SHALL exist an equivalent gRPC method that
//! accepts equivalent input and returns equivalent output.
//!
//! **Validates: Requirements 1.1, 1.2**

use axum::extract::State;
use axum::Json;
use caliber_api::grpc::{proto, TrajectoryServiceImpl};
use caliber_api::routes::trajectory;
use caliber_api::types::CreateTrajectoryRequest;
use caliber_api::ws::WsState;
use caliber_api::proto::trajectory_service_server::TrajectoryService;
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use std::sync::Arc;
use tonic::Request;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

#[path = "support/db.rs"]
mod test_db_support;
#[path = "support/ws.rs"]
mod test_ws_support;

async fn recv_trajectory_event(
    rx: &mut broadcast::Receiver<caliber_api::events::WsEvent>,
) -> Result<caliber_api::types::TrajectoryResponse, String> {
    match timeout(Duration::from_millis(200), rx.recv()).await {
        Ok(Ok(caliber_api::events::WsEvent::TrajectoryCreated { trajectory })) => Ok(trajectory),
        Ok(Ok(other)) => Err(format!("Expected TrajectoryCreated, got {:?}", other)),
        Ok(Err(err)) => Err(format!("Broadcast recv error: {:?}", err)),
        Err(_) => Err("Timed out waiting for TrajectoryCreated event".to_string()),
    }
}

fn trajectory_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 _-]{3,30}".prop_map(|s| s.trim().to_string())
}

fn description_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        "[a-zA-Z0-9 ,.]{10,60}".prop_map(|s| Some(s.trim().to_string())),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Property 2: REST-gRPC Parity (Trajectory Create)**
    #[test]
    fn prop_rest_grpc_parity_trajectory_create(
        name in trajectory_name_strategy(),
        description in description_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(async {
            let db = test_db_support::test_db_client();

            // REST handler setup
            let ws_rest = test_ws_support::test_ws_state(50);
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
            let rest_trajectory = recv_trajectory_event(&mut rx_rest)
                .await
                .map_err(TestCaseError::fail)?;

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
                .map_err(|e| TestCaseError::fail(format!("gRPC create failed: {}", e)))?
                .into_inner();

            // Parity checks: core fields align across REST and gRPC
            prop_assert_eq!(grpc_resp.name.as_str(), rest_trajectory.name.as_str());
            prop_assert_eq!(grpc_resp.description.as_deref(), rest_trajectory.description.as_deref());
            prop_assert!(!grpc_resp.status.is_empty(), "gRPC status should be set");
            prop_assert!(!grpc_resp.trajectory_id.is_empty(), "gRPC id should be set");

            Ok::<(), TestCaseError>(())
        })?;
    }
}
