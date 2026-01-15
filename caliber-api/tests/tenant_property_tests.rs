//! Property-Based Tests for Tenant Isolation
//!
//! **Property 5: Tenant Isolation**
//!
//! For any authenticated request with a tenant context header, the API SHALL
//! return ONLY data belonging to that tenant, AND mutations SHALL only affect
//! that tenant's data.
//!
//! **Validates: Requirements 1.6, 2.5**

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use caliber_api::{
    auth::{generate_jwt_token, AuthConfig},
    middleware::{auth_middleware, extract_auth_context, AuthMiddlewareState},
    types::{CreateTrajectoryRequest, ListTrajectoriesResponse, TrajectoryResponse},
};
use caliber_core::{EntityId, TrajectoryStatus};
use proptest::prelude::*;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

/// In-memory storage for testing tenant isolation.
///
/// This simulates a database where each trajectory is associated with a tenant.
#[derive(Debug, Clone, Default)]
struct TestStorage {
    /// Map of trajectory_id -> (tenant_id, trajectory)
    trajectories: Arc<Mutex<Vec<(EntityId, TrajectoryResponse)>>>,
}

impl TestStorage {
    fn new() -> Self {
        Self {
            trajectories: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a trajectory for a specific tenant
    fn create_trajectory(
        &self,
        tenant_id: EntityId,
        req: &CreateTrajectoryRequest,
    ) -> TrajectoryResponse {
        let trajectory_id = EntityId::from(Uuid::now_v7());
        let now = caliber_core::Timestamp::now();

        let trajectory = TrajectoryResponse {
            trajectory_id,
            name: req.name.clone(),
            description: req.description.clone(),
            status: TrajectoryStatus::Active,
            parent_trajectory_id: req.parent_trajectory_id,
            root_trajectory_id: None,
            agent_id: req.agent_id,
            created_at: now,
            updated_at: now,
            completed_at: None,
            outcome: None,
            metadata: req.metadata.clone(),
        };

        self.trajectories
            .lock()
            .unwrap()
            .push((tenant_id, trajectory.clone()));

        trajectory
    }

    /// List trajectories for a specific tenant
    fn list_trajectories(&self, tenant_id: EntityId) -> Vec<TrajectoryResponse> {
        self.trajectories
            .lock()
            .unwrap()
            .iter()
            .filter(|(tid, _)| *tid == tenant_id)
            .map(|(_, traj)| traj.clone())
            .collect()
    }

    /// Get a trajectory by ID, only if it belongs to the tenant
    fn get_trajectory(
        &self,
        tenant_id: EntityId,
        trajectory_id: EntityId,
    ) -> Option<TrajectoryResponse> {
        self.trajectories
            .lock()
            .unwrap()
            .iter()
            .find(|(tid, traj)| *tid == tenant_id && traj.trajectory_id == trajectory_id)
            .map(|(_, traj)| traj.clone())
    }

    /// Count total trajectories across all tenants
    fn count_all(&self) -> usize {
        self.trajectories.lock().unwrap().len()
    }

    /// Count trajectories for a specific tenant
    fn count_for_tenant(&self, tenant_id: EntityId) -> usize {
        self.trajectories
            .lock()
            .unwrap()
            .iter()
            .filter(|(tid, _)| *tid == tenant_id)
            .count()
    }
}

/// Create a test authentication configuration
fn test_auth_config() -> AuthConfig {
    let mut config = AuthConfig::default();
    config.add_api_key("test_api_key".to_string());
    config.jwt_secret = "test_secret_for_tenant_isolation".to_string();
    config.require_tenant_header = true;
    config
}

/// Create a test app with tenant-isolated routes
fn test_app(storage: TestStorage) -> Router {
    let auth_config = test_auth_config();
    let auth_state = AuthMiddlewareState::new(auth_config);

    // Handler for creating trajectories
    async fn create_trajectory_handler(
        State(storage): State<TestStorage>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        let auth_context = extract_auth_context(&request);
        let tenant_id = auth_context.tenant_id;

        // Parse request body
        let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
            .await
            .unwrap();
        let req: CreateTrajectoryRequest = serde_json::from_slice(&body_bytes).unwrap();

        // Create trajectory for this tenant
        let trajectory = storage.create_trajectory(tenant_id, &req);

        (StatusCode::CREATED, Json(trajectory))
    }

    // Handler for listing trajectories
    async fn list_trajectories_handler(
        State(storage): State<TestStorage>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        let auth_context = extract_auth_context(&request);
        let tenant_id = auth_context.tenant_id;

        // List only trajectories for this tenant
        let trajectories = storage.list_trajectories(tenant_id);

        let response = ListTrajectoriesResponse {
            trajectories,
            total: storage.count_for_tenant(tenant_id) as i32,
        };

        Json(response)
    }

    // Handler for getting a specific trajectory
    async fn get_trajectory_handler(
        State(storage): State<TestStorage>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        let auth_context = extract_auth_context(&request);
        let tenant_id = auth_context.tenant_id;

        // Extract trajectory ID from path (simplified - in real app would use Path extractor)
        let uri = request.uri().path();
        let trajectory_id_str = uri.split('/').last().unwrap();
        let trajectory_id = Uuid::parse_str(trajectory_id_str).unwrap().into();

        // Get trajectory only if it belongs to this tenant
        match storage.get_trajectory(tenant_id, trajectory_id) {
            Some(trajectory) => (StatusCode::OK, Json(Some(trajectory))).into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }

    Router::new()
        .route("/api/v1/trajectories", post(create_trajectory_handler))
        .route("/api/v1/trajectories", get(list_trajectories_handler))
        .route(
            "/api/v1/trajectories/:id",
            get(get_trajectory_handler),
        )
        .with_state(storage)
        .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
}

// ============================================================================
// PROPERTY TEST STRATEGIES
// ============================================================================

/// Strategy for generating tenant IDs
fn tenant_id_strategy() -> impl Strategy<Value = EntityId> {
    any::<[u8; 16]>().prop_map(|bytes| Uuid::from_bytes(bytes).into())
}

/// Strategy for generating trajectory names
fn trajectory_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{5,50}".prop_map(|s| s.trim().to_string())
}

/// Strategy for generating CreateTrajectoryRequest
fn create_trajectory_request_strategy() -> impl Strategy<Value = CreateTrajectoryRequest> {
    trajectory_name_strategy().prop_map(|name| CreateTrajectoryRequest {
        name,
        description: Some("Test trajectory".to_string()),
        parent_trajectory_id: None,
        agent_id: None,
        metadata: None,
    })
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 5.1: Tenant Isolation - Create**
    ///
    /// For any authenticated request with a tenant context, creating a
    /// trajectory SHALL associate it with that tenant only.
    ///
    /// **Validates: Requirements 1.6, 2.5**
    #[test]
    fn prop_tenant_isolation_create(
        tenant_id_bytes in any::<[u8; 16]>(),
        trajectory_req in create_trajectory_request_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tenant_id = Uuid::from_bytes(tenant_id_bytes).into();
            let storage = TestStorage::new();
            let app = test_app(storage.clone());
            let auth_config = test_auth_config();

            // Generate JWT token for this tenant
            let token = generate_jwt_token(
                &auth_config,
                "user123".to_string(),
                Some(tenant_id),
                vec![],
            )
            .unwrap();

            // Create trajectory
            let request = Request::builder()
                .uri("/api/v1/trajectories")
                .method("POST")
                .header("authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&trajectory_req).unwrap()))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            // Verify successful creation
            prop_assert_eq!(response.status(), StatusCode::CREATED);

            // Verify the trajectory is associated with the correct tenant
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let created_trajectory: TrajectoryResponse =
                serde_json::from_slice(&body).unwrap();

            // Verify we can retrieve it with the same tenant
            let trajectory_count = storage.count_for_tenant(tenant_id);
            prop_assert_eq!(trajectory_count, 1);

            // Verify the trajectory matches what we created
            let retrieved = storage.get_trajectory(tenant_id, created_trajectory.trajectory_id);
            prop_assert!(retrieved.is_some());
            prop_assert_eq!(retrieved.unwrap().name, trajectory_req.name);

            Ok(())
        })?;
    }

    /// **Property 5.2: Tenant Isolation - List**
    ///
    /// For any authenticated request with a tenant context, listing
    /// trajectories SHALL return ONLY trajectories belonging to that tenant.
    ///
    /// **Validates: Requirements 1.6, 2.5**
    #[test]
    fn prop_tenant_isolation_list(
        tenant_a_bytes in any::<[u8; 16]>(),
        tenant_b_bytes in any::<[u8; 16]>(),
        trajectory_req in create_trajectory_request_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Ensure we have two different tenants
            prop_assume!(tenant_a_bytes != tenant_b_bytes);

            let tenant_a: EntityId = Uuid::from_bytes(tenant_a_bytes).into();
            let tenant_b: EntityId = Uuid::from_bytes(tenant_b_bytes).into();
            let storage = TestStorage::new();
            let auth_config = test_auth_config();

            // Create trajectories for both tenants
            storage.create_trajectory(tenant_a, &trajectory_req);
            storage.create_trajectory(tenant_b, &trajectory_req);

            // Verify total count
            prop_assert_eq!(storage.count_all(), 2);

            // Create app
            let app = test_app(storage.clone());

            // Generate JWT token for tenant A
            let token_a = generate_jwt_token(
                &auth_config,
                "user_a".to_string(),
                Some(tenant_a),
                vec![],
            )
            .unwrap();

            // List trajectories as tenant A
            let request = Request::builder()
                .uri("/api/v1/trajectories")
                .method("GET")
                .header("authorization", format!("Bearer {}", token_a))
                .body(Body::empty())
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            prop_assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let list_response: ListTrajectoriesResponse =
                serde_json::from_slice(&body).unwrap();

            // Property: Tenant A should only see their own trajectory
            prop_assert_eq!(list_response.trajectories.len(), 1);
            prop_assert_eq!(list_response.total, 1);

            // Generate JWT token for tenant B
            let token_b = generate_jwt_token(
                &auth_config,
                "user_b".to_string(),
                Some(tenant_b),
                vec![],
            )
            .unwrap();

            // List trajectories as tenant B
            let request = Request::builder()
                .uri("/api/v1/trajectories")
                .method("GET")
                .header("authorization", format!("Bearer {}", token_b))
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();
            prop_assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let list_response: ListTrajectoriesResponse =
                serde_json::from_slice(&body).unwrap();

            // Property: Tenant B should only see their own trajectory
            prop_assert_eq!(list_response.trajectories.len(), 1);
            prop_assert_eq!(list_response.total, 1);

            Ok(())
        })?;
    }

    /// **Property 5.3: Tenant Isolation - Get**
    ///
    /// For any authenticated request with a tenant context, getting a
    /// trajectory by ID SHALL return 404 if the trajectory belongs to a
    /// different tenant.
    ///
    /// **Validates: Requirements 1.6, 2.5**
    #[test]
    fn prop_tenant_isolation_get(
        tenant_a_bytes in any::<[u8; 16]>(),
        tenant_b_bytes in any::<[u8; 16]>(),
        trajectory_req in create_trajectory_request_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Ensure we have two different tenants
            prop_assume!(tenant_a_bytes != tenant_b_bytes);

            let tenant_a: EntityId = Uuid::from_bytes(tenant_a_bytes).into();
            let tenant_b: EntityId = Uuid::from_bytes(tenant_b_bytes).into();
            let storage = TestStorage::new();
            let auth_config = test_auth_config();

            // Create trajectory for tenant A
            let trajectory_a = storage.create_trajectory(tenant_a, &trajectory_req);

            // Create app
            let app = test_app(storage.clone());

            // Generate JWT token for tenant A
            let token_a = generate_jwt_token(
                &auth_config,
                "user_a".to_string(),
                Some(tenant_a),
                vec![],
            )
            .unwrap();

            // Tenant A should be able to get their own trajectory
            let request = Request::builder()
                .uri(format!("/api/v1/trajectories/{}", trajectory_a.trajectory_id))
                .method("GET")
                .header("authorization", format!("Bearer {}", token_a))
                .body(Body::empty())
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            prop_assert_eq!(response.status(), StatusCode::OK);

            // Generate JWT token for tenant B
            let token_b = generate_jwt_token(
                &auth_config,
                "user_b".to_string(),
                Some(tenant_b),
                vec![],
            )
            .unwrap();

            // Tenant B should NOT be able to get tenant A's trajectory
            let request = Request::builder()
                .uri(format!("/api/v1/trajectories/{}", trajectory_a.trajectory_id))
                .method("GET")
                .header("authorization", format!("Bearer {}", token_b))
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            // Property: Cross-tenant access should return 404
            prop_assert_eq!(response.status(), StatusCode::NOT_FOUND);

            Ok(())
        })?;
    }

    /// **Property 5.4: Tenant Isolation - Multiple Tenants**
    ///
    /// For any set of tenants, each tenant SHALL only see their own data,
    /// regardless of how many other tenants exist.
    ///
    /// **Validates: Requirements 1.6, 2.5**
    #[test]
    fn prop_tenant_isolation_multiple_tenants(
        tenant_ids in prop::collection::vec(any::<[u8; 16]>(), 2..5),
        trajectory_req in create_trajectory_request_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = TestStorage::new();
            let auth_config = test_auth_config();

            // Convert to EntityIds and ensure uniqueness
            let mut tenant_ids: Vec<EntityId> = tenant_ids
                .into_iter()
                .map(|bytes| Uuid::from_bytes(bytes).into())
                .collect();
            tenant_ids.sort();
            tenant_ids.dedup();

            // Need at least 2 unique tenants
            prop_assume!(tenant_ids.len() >= 2);

            // Create one trajectory for each tenant
            for tenant_id in &tenant_ids {
                storage.create_trajectory(*tenant_id, &trajectory_req);
            }

            // Verify total count
            prop_assert_eq!(storage.count_all(), tenant_ids.len());

            // Create app
            let app = test_app(storage.clone());

            // For each tenant, verify they only see their own trajectory
            for tenant_id in &tenant_ids {
                let token = generate_jwt_token(
                    &auth_config,
                    format!("user_{}", tenant_id),
                    Some(*tenant_id),
                    vec![],
                )
                .unwrap();

                let request = Request::builder()
                    .uri("/api/v1/trajectories")
                    .method("GET")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap();

                let response = app.clone().oneshot(request).await.unwrap();
                prop_assert_eq!(response.status(), StatusCode::OK);

                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let list_response: ListTrajectoriesResponse =
                    serde_json::from_slice(&body).unwrap();

                // Property: Each tenant should see exactly 1 trajectory (their own)
                prop_assert_eq!(
                    list_response.trajectories.len(),
                    1,
                    "Tenant {} should see exactly 1 trajectory, but saw {}",
                    tenant_id,
                    list_response.trajectories.len()
                );
                prop_assert_eq!(list_response.total, 1);
            }

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
    async fn test_tenant_isolation_empty_database() {
        let storage = TestStorage::new();
        let app = test_app(storage.clone());
        let auth_config = test_auth_config();
        let tenant_id = Uuid::now_v7().into();

        let token = generate_jwt_token(
            &auth_config,
            "user123".to_string(),
            Some(tenant_id),
            vec![],
        )
        .unwrap();

        // List trajectories for a tenant with no data
        let request = Request::builder()
            .uri("/api/v1/trajectories")
            .method("GET")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_response: ListTrajectoriesResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(list_response.trajectories.len(), 0);
        assert_eq!(list_response.total, 0);
    }

    #[tokio::test]
    async fn test_tenant_isolation_same_trajectory_name() {
        let storage = TestStorage::new();
        let auth_config = test_auth_config();

        let tenant_a = Uuid::now_v7().into();
        let tenant_b = Uuid::now_v7().into();

        // Create trajectories with the same name for different tenants
        let req = CreateTrajectoryRequest {
            name: "Duplicate Name".to_string(),
            description: None,
            parent_trajectory_id: None,
            agent_id: None,
            metadata: None,
        };

        storage.create_trajectory(tenant_a, &req);
        storage.create_trajectory(tenant_b, &req);

        // Verify each tenant only sees their own trajectory
        assert_eq!(storage.count_for_tenant(tenant_a), 1);
        assert_eq!(storage.count_for_tenant(tenant_b), 1);
        assert_eq!(storage.count_all(), 2);
    }

    #[tokio::test]
    async fn test_tenant_isolation_nonexistent_trajectory() {
        let storage = TestStorage::new();
        let app = test_app(storage.clone());
        let auth_config = test_auth_config();
        let tenant_id = Uuid::now_v7().into();
        let nonexistent_id = Uuid::now_v7().into();

        let token = generate_jwt_token(
            &auth_config,
            "user123".to_string(),
            Some(tenant_id),
            vec![],
        )
        .unwrap();

        // Try to get a trajectory that doesn't exist
        let request = Request::builder()
            .uri(format!("/api/v1/trajectories/{}", nonexistent_id))
            .method("GET")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}