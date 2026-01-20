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
    auth::{generate_jwt_token, AuthConfig, JwtSecret},
    error::ApiError,
    middleware::{auth_middleware, extract_auth_context, AuthMiddlewareState},
    types::{CreateTrajectoryRequest, ListTrajectoriesResponse, TrajectoryResponse},
};
use chrono::Utc;
use caliber_core::{EntityId, TrajectoryStatus};
use proptest::prelude::*;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use tokio::runtime::Runtime;
use uuid::Uuid;

#[path = "support/auth_with_tenant.rs"]
mod test_auth_with_tenant_support;
use test_auth_with_tenant_support::test_auth_context_with_tenant;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

fn test_runtime() -> Result<Runtime, TestCaseError> {
    Runtime::new().map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))
}

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
        let now = Utc::now();

        let trajectory = TrajectoryResponse {
            trajectory_id,
            tenant_id: Some(tenant_id),
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
            .unwrap_or_else(|err| err.into_inner())
            .push((tenant_id, trajectory.clone()));

        trajectory
    }

    /// List trajectories for a specific tenant
    fn list_trajectories(&self, tenant_id: EntityId) -> Vec<TrajectoryResponse> {
        self.trajectories
            .lock()
            .unwrap_or_else(|err| err.into_inner())
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
            .unwrap_or_else(|err| err.into_inner())
            .iter()
            .find(|(tid, traj)| *tid == tenant_id && traj.trajectory_id == trajectory_id)
            .map(|(_, traj)| traj.clone())
    }

    /// Count total trajectories across all tenants
    fn count_all(&self) -> usize {
        self.trajectories
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .len()
    }

    /// Count trajectories for a specific tenant
    fn count_for_tenant(&self, tenant_id: EntityId) -> usize {
        self.trajectories
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .iter()
            .filter(|(tid, _)| *tid == tenant_id)
            .count()
    }
}

/// Create a test authentication configuration
fn test_auth_config() -> AuthConfig {
    let mut config = AuthConfig::default();
    config.add_api_key("test_api_key".to_string());
    config.jwt_secret = JwtSecret::new("test_secret_for_tenant_isolation".to_string())
        .expect("test secret should be valid");
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
        let auth_context = match extract_auth_context(&request) {
            Ok(ctx) => ctx,
            Err(err) => return (StatusCode::UNAUTHORIZED, Json(err)).into_response(),
        };
        let tenant_id = auth_context.tenant_id;

        // Parse request body
        let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
            .await
            .map_err(|e| ApiError::invalid_input(format!("Failed to read request body: {:?}", e)))
            .and_then(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|e| ApiError::invalid_input(format!("Invalid JSON: {}", e)))
            });
        let req = match body_bytes {
            Ok(req) => req,
            Err(err) => return (StatusCode::BAD_REQUEST, Json(err)).into_response(),
        };

        // Create trajectory for this tenant
        let trajectory = storage.create_trajectory(tenant_id, &req);

        (StatusCode::CREATED, Json(trajectory)).into_response()
    }

    // Handler for listing trajectories
    async fn list_trajectories_handler(
        State(storage): State<TestStorage>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        let auth_context = match extract_auth_context(&request) {
            Ok(ctx) => ctx,
            Err(err) => return (StatusCode::UNAUTHORIZED, Json(err)).into_response(),
        };
        let tenant_id = auth_context.tenant_id;

        // List only trajectories for this tenant
        let trajectories = storage.list_trajectories(tenant_id);

        let response = ListTrajectoriesResponse {
            trajectories,
            total: storage.count_for_tenant(tenant_id) as i32,
        };

        Json(response).into_response()
    }

    // Handler for getting a specific trajectory
    async fn get_trajectory_handler(
        State(storage): State<TestStorage>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        let auth_context = match extract_auth_context(&request) {
            Ok(ctx) => ctx,
            Err(err) => return (StatusCode::UNAUTHORIZED, Json(err)).into_response(),
        };
        let tenant_id = auth_context.tenant_id;

        // Extract trajectory ID from path (simplified - in real app would use Path extractor)
        let uri = request.uri().path();
        let trajectory_id_str = match uri.split('/').next_back() {
            Some(id) => id,
            None => return StatusCode::BAD_REQUEST.into_response(),
        };
        let trajectory_id = match Uuid::parse_str(trajectory_id_str) {
            Ok(id) => EntityId::from(id),
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };

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
    any::<[u8; 16]>().prop_map(Uuid::from_bytes)
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
        tenant_id in tenant_id_strategy(),
        trajectory_req in create_trajectory_request_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
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
            .map_err(|e| TestCaseError::fail(format!("Failed to generate JWT: {}", e.message)))?;

            // Create trajectory
            let request = Request::builder()
                .uri("/api/v1/trajectories")
                .method("POST")
                .header("authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&trajectory_req)
                        .map_err(|e| TestCaseError::fail(format!("Failed to serialize request: {}", e)))?,
                ))
                .map_err(|e| TestCaseError::fail(format!("Failed to build request: {}", e)))?;

            let response = app
                .oneshot(request)
                .await
                .map_err(|e| TestCaseError::fail(format!("Request failed: {:?}", e)))?;

            // Verify successful creation
            prop_assert_eq!(response.status(), StatusCode::CREATED);

            // Verify the trajectory is associated with the correct tenant
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .map_err(|e| TestCaseError::fail(format!("Failed to read body: {:?}", e)))?;
            let created_trajectory: TrajectoryResponse =
                serde_json::from_slice(&body)
                    .map_err(|e| TestCaseError::fail(format!("Failed to parse response: {}", e)))?;

            // Verify we can retrieve it with the same tenant
            let trajectory_count = storage.count_for_tenant(tenant_id);
            prop_assert_eq!(trajectory_count, 1);

            // Verify the trajectory matches what we created
            let retrieved = storage.get_trajectory(tenant_id, created_trajectory.trajectory_id);
            prop_assert!(retrieved.is_some());
            prop_assert_eq!(retrieved.map(|t| t.name), Some(trajectory_req.name));

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
        tenant_a in tenant_id_strategy(),
        tenant_b in tenant_id_strategy(),
        trajectory_req in create_trajectory_request_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            // Ensure we have two different tenants
            prop_assume!(tenant_a != tenant_b);
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
            .map_err(|e| TestCaseError::fail(format!("Failed to generate JWT: {}", e.message)))?;

            // List trajectories as tenant A
            let request = Request::builder()
                .uri("/api/v1/trajectories")
                .method("GET")
                .header("authorization", format!("Bearer {}", token_a))
                .body(Body::empty())
                .map_err(|e| TestCaseError::fail(format!("Failed to build request: {}", e)))?;

            let response = app
                .clone()
                .oneshot(request)
                .await
                .map_err(|e| TestCaseError::fail(format!("Request failed: {:?}", e)))?;
            prop_assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .map_err(|e| TestCaseError::fail(format!("Failed to read body: {:?}", e)))?;
            let list_response: ListTrajectoriesResponse =
                serde_json::from_slice(&body)
                    .map_err(|e| TestCaseError::fail(format!("Failed to parse response: {}", e)))?;

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
            .map_err(|e| TestCaseError::fail(format!("Failed to generate JWT: {}", e.message)))?;

            // List trajectories as tenant B
            let request = Request::builder()
                .uri("/api/v1/trajectories")
                .method("GET")
                .header("authorization", format!("Bearer {}", token_b))
                .body(Body::empty())
                .map_err(|e| TestCaseError::fail(format!("Failed to build request: {}", e)))?;

            let response = app
                .oneshot(request)
                .await
                .map_err(|e| TestCaseError::fail(format!("Request failed: {:?}", e)))?;
            prop_assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .map_err(|e| TestCaseError::fail(format!("Failed to read body: {:?}", e)))?;
            let list_response: ListTrajectoriesResponse =
                serde_json::from_slice(&body)
                    .map_err(|e| TestCaseError::fail(format!("Failed to parse response: {}", e)))?;

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
        tenant_a in tenant_id_strategy(),
        tenant_b in tenant_id_strategy(),
        trajectory_req in create_trajectory_request_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            // Ensure we have two different tenants
            prop_assume!(tenant_a != tenant_b);
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
            .map_err(|e| TestCaseError::fail(format!("Failed to generate JWT: {}", e.message)))?;

            // Tenant A should be able to get their own trajectory
            let request = Request::builder()
                .uri(format!("/api/v1/trajectories/{}", trajectory_a.trajectory_id))
                .method("GET")
                .header("authorization", format!("Bearer {}", token_a))
                .body(Body::empty())
                .map_err(|e| TestCaseError::fail(format!("Failed to build request: {}", e)))?;

            let response = app
                .clone()
                .oneshot(request)
                .await
                .map_err(|e| TestCaseError::fail(format!("Request failed: {:?}", e)))?;
            prop_assert_eq!(response.status(), StatusCode::OK);

            // Generate JWT token for tenant B
            let token_b = generate_jwt_token(
                &auth_config,
                "user_b".to_string(),
                Some(tenant_b),
                vec![],
            )
            .map_err(|e| TestCaseError::fail(format!("Failed to generate JWT: {}", e.message)))?;

            // Tenant B should NOT be able to get tenant A's trajectory
            let request = Request::builder()
                .uri(format!("/api/v1/trajectories/{}", trajectory_a.trajectory_id))
                .method("GET")
                .header("authorization", format!("Bearer {}", token_b))
                .body(Body::empty())
                .map_err(|e| TestCaseError::fail(format!("Failed to build request: {}", e)))?;

            let response = app
                .oneshot(request)
                .await
                .map_err(|e| TestCaseError::fail(format!("Request failed: {:?}", e)))?;

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
        let rt = test_runtime()?;
        rt.block_on(async {
            let storage = TestStorage::new();
            let auth_config = test_auth_config();

            // Convert to EntityIds and ensure uniqueness
            let mut tenant_ids: Vec<EntityId> = tenant_ids
                .into_iter()
                .map(Uuid::from_bytes)
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
                .map_err(|e| TestCaseError::fail(format!("Failed to generate JWT: {}", e.message)))?;

                let request = Request::builder()
                    .uri("/api/v1/trajectories")
                    .method("GET")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .map_err(|e| TestCaseError::fail(format!("Failed to build request: {}", e)))?;

                let response = app
                    .clone()
                    .oneshot(request)
                    .await
                    .map_err(|e| TestCaseError::fail(format!("Request failed: {:?}", e)))?;
                prop_assert_eq!(response.status(), StatusCode::OK);

                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .map_err(|e| TestCaseError::fail(format!("Failed to read body: {:?}", e)))?;
                let list_response: ListTrajectoriesResponse =
                    serde_json::from_slice(&body)
                        .map_err(|e| TestCaseError::fail(format!("Failed to parse response: {}", e)))?;

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
// WEBSOCKET TENANT ISOLATION PROPERTY TESTS
// ============================================================================

/// Module testing WebSocket event filtering by tenant.
/// These tests verify that events are only delivered to clients with matching tenant_id.
mod ws_tenant_isolation {
    use caliber_api::{
        events::WsEvent,
        should_deliver_event, tenant_id_from_event,
    };
    use caliber_core::EntityId;
    use proptest::prelude::*;
    use uuid::Uuid;

    /// Strategy for generating EntityIds
    fn entity_id_strategy() -> impl Strategy<Value = EntityId> {
        any::<[u8; 16]>().prop_map(Uuid::from_bytes)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Property 5.5: WebSocket Tenant Isolation - Events with tenant_id**
        ///
        /// Events that include a tenant_id field SHALL only be delivered to
        /// WebSocket clients authenticated for that tenant.
        ///
        /// **Validates: Requirements 1.6, 2.5**
        #[test]
        fn prop_ws_tenant_filtered_events_match_tenant(
            tenant_a in entity_id_strategy(),
            tenant_b in entity_id_strategy(),
            event_id in entity_id_strategy(),
        ) {
            // Ensure different tenants
            prop_assume!(tenant_a != tenant_b);

            // Test TrajectoryDeleted event (has tenant_id field)
            let event = WsEvent::TrajectoryDeleted {
                tenant_id: tenant_a,
                id: event_id,
            };

            // Extract tenant_id from event
            let extracted_tenant_id = tenant_id_from_event(&event);
            prop_assert_eq!(extracted_tenant_id, Some(tenant_a));

            // Event should be delivered to tenant A
            let should_deliver_a = should_deliver_event(&event, tenant_a);
            prop_assert!(should_deliver_a, "Event should be delivered to matching tenant");

            // Event should NOT be delivered to tenant B
            let should_deliver_b = should_deliver_event(&event, tenant_b);
            prop_assert!(!should_deliver_b, "Event should NOT be delivered to non-matching tenant");
        }

        /// **Property 5.6: WebSocket Tenant Isolation - Delete Events**
        ///
        /// All delete events SHALL include tenant_id and only be delivered to
        /// the owning tenant.
        ///
        /// **Validates: Requirements 1.6, 2.5**
        #[test]
        fn prop_ws_delete_events_include_tenant_id(
            tenant_id in entity_id_strategy(),
            entity_id in entity_id_strategy(),
        ) {
            // Test all delete event types
            let delete_events = vec![
                WsEvent::TrajectoryDeleted { tenant_id, id: entity_id },
                WsEvent::ArtifactDeleted { tenant_id, id: entity_id },
                WsEvent::NoteDeleted { tenant_id, id: entity_id },
            ];

            for event in delete_events {
                // All delete events must have extractable tenant_id
                let extracted = tenant_id_from_event(&event);
                prop_assert!(
                    extracted.is_some(),
                    "Delete event {:?} must have extractable tenant_id",
                    event.event_type()
                );
                prop_assert_eq!(
                    extracted.ok_or_else(|| {
                        TestCaseError::fail("Delete event missing tenant_id".to_string())
                    })?,
                    tenant_id,
                    "Extracted tenant_id must match"
                );
            }
        }

        /// **Property 5.7: WebSocket Tenant Isolation - Status Events**
        ///
        /// Status change events SHALL include tenant_id and only be delivered
        /// to the owning tenant.
        ///
        /// **Validates: Requirements 1.6, 2.5**
        #[test]
        fn prop_ws_status_events_include_tenant_id(
            tenant_id_bytes in any::<[u8; 16]>(),
            entity_id_bytes in any::<[u8; 16]>(),
        ) {
            let tenant_id: EntityId = Uuid::from_bytes(tenant_id_bytes);
            let entity_id: EntityId = Uuid::from_bytes(entity_id_bytes);

            let status_events = vec![
                WsEvent::AgentStatusChanged {
                    tenant_id,
                    agent_id: entity_id,
                    status: "active".to_string(),
                },
                WsEvent::AgentHeartbeat {
                    tenant_id,
                    agent_id: entity_id,
                    timestamp: chrono::Utc::now(),
                },
                WsEvent::AgentUnregistered { tenant_id, id: entity_id },
                WsEvent::LockReleased { tenant_id, lock_id: entity_id },
                WsEvent::LockExpired { tenant_id, lock_id: entity_id },
                WsEvent::MessageDelivered { tenant_id, message_id: entity_id },
                WsEvent::MessageAcknowledged { tenant_id, message_id: entity_id },
                WsEvent::DelegationAccepted { tenant_id, delegation_id: entity_id },
                WsEvent::DelegationRejected { tenant_id, delegation_id: entity_id },
                WsEvent::HandoffAccepted { tenant_id, handoff_id: entity_id },
            ];

            for event in status_events {
                let extracted = tenant_id_from_event(&event);
                prop_assert!(
                    extracted.is_some(),
                    "Status event {:?} must have extractable tenant_id",
                    event.event_type()
                );
                prop_assert_eq!(
                    extracted.ok_or_else(|| {
                        TestCaseError::fail("Status event missing tenant_id".to_string())
                    })?,
                    tenant_id,
                    "Extracted tenant_id must match for {:?}",
                    event.event_type()
                );
            }
        }

        /// **Property 5.8: WebSocket Tenant Isolation - Deny by Default**
        ///
        /// If tenant_id cannot be determined from an event, it SHALL NOT be
        /// delivered (deny by default for security).
        ///
        /// **Validates: Requirements 1.6, 2.5**
        #[test]
        fn prop_ws_unknown_tenant_events_denied(
            client_tenant_id_bytes in any::<[u8; 16]>(),
        ) {
            let client_tenant_id: EntityId = Uuid::from_bytes(client_tenant_id_bytes);

            // Events that currently return None for tenant_id should be denied
            // (Note: This tests the DENY by default behavior)
            let event_without_tenant = WsEvent::Disconnected {
                reason: "test".to_string(),
            };

            let extracted = tenant_id_from_event(&event_without_tenant);
            prop_assert!(
                extracted.is_none(),
                "Disconnected event should not have tenant_id"
            );

            let should_deliver = should_deliver_event(&event_without_tenant, client_tenant_id);
            prop_assert_eq!(
                should_deliver,
                !event_without_tenant.is_tenant_specific(),
                "Non-tenant-specific events should deliver; tenant-specific without tenant should not"
            );
        }

        /// **Property 5.9: WebSocket Cross-Tenant Isolation**
        ///
        /// For any event with a tenant_id, it SHALL NEVER be delivered to a
        /// client authenticated for a different tenant.
        ///
        /// **Validates: Requirements 1.6, 2.5**
        #[test]
        fn prop_ws_cross_tenant_never_delivered(
            tenant_a_bytes in any::<[u8; 16]>(),
            tenant_b_bytes in any::<[u8; 16]>(),
            entity_id_bytes in any::<[u8; 16]>(),
        ) {
            // Must be different tenants
            prop_assume!(tenant_a_bytes != tenant_b_bytes);

            let tenant_a: EntityId = Uuid::from_bytes(tenant_a_bytes);
            let tenant_b: EntityId = Uuid::from_bytes(tenant_b_bytes);
            let entity_id: EntityId = Uuid::from_bytes(entity_id_bytes);

            // All tenant-specific events with tenant_a should not be delivered to tenant_b
            let tenant_a_events = vec![
                WsEvent::TrajectoryDeleted { tenant_id: tenant_a, id: entity_id },
                WsEvent::ArtifactDeleted { tenant_id: tenant_a, id: entity_id },
                WsEvent::NoteDeleted { tenant_id: tenant_a, id: entity_id },
                WsEvent::AgentUnregistered { tenant_id: tenant_a, id: entity_id },
                WsEvent::AgentStatusChanged {
                    tenant_id: tenant_a,
                    agent_id: entity_id,
                    status: "idle".to_string(),
                },
                WsEvent::LockReleased { tenant_id: tenant_a, lock_id: entity_id },
                WsEvent::MessageAcknowledged { tenant_id: tenant_a, message_id: entity_id },
                WsEvent::DelegationAccepted { tenant_id: tenant_a, delegation_id: entity_id },
                WsEvent::HandoffAccepted { tenant_id: tenant_a, handoff_id: entity_id },
            ];

            for event in tenant_a_events {
                let should_deliver = should_deliver_event(&event, tenant_b);
                prop_assert!(
                    !should_deliver,
                    "Event {:?} from tenant_a MUST NOT be delivered to tenant_b",
                    event.event_type()
                );
            }
        }
    }
}

// ============================================================================
// UNIT TESTS FOR EDGE CASES
// ============================================================================

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_tenant_isolation_empty_database() -> Result<(), String> {
        let storage = TestStorage::new();
        let app = test_app(storage.clone());
        let auth_config = test_auth_config();
        let tenant_id: EntityId = Uuid::now_v7();

        let auth_context = test_auth_context_with_tenant(tenant_id);
        let token = generate_jwt_token(
            &auth_config,
            auth_context.user_id.clone(),
            Some(auth_context.tenant_id),
            auth_context.roles.clone(),
        )
        .map_err(|e| e.message)?;

        // List trajectories for a tenant with no data
        let request = Request::builder()
            .uri("/api/v1/trajectories")
            .method("GET")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .map_err(|e| e.to_string())?;

        let response = app
            .oneshot(request)
            .await
            .map_err(|e| format!("Request failed: {:?}", e))?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .map_err(|e| format!("Failed to read body: {:?}", e))?;
        let list_response: ListTrajectoriesResponse =
            serde_json::from_slice(&body).map_err(|e| e.to_string())?;

        assert_eq!(list_response.trajectories.len(), 0);
        assert_eq!(list_response.total, 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_tenant_isolation_same_trajectory_name() {
        let storage = TestStorage::new();
        let _auth_config = test_auth_config();

        let tenant_a: EntityId = Uuid::now_v7();
        let tenant_b: EntityId = Uuid::now_v7();

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
    async fn test_tenant_isolation_nonexistent_trajectory() -> Result<(), String> {
        let storage = TestStorage::new();
        let app = test_app(storage.clone());
        let auth_config = test_auth_config();
        let tenant_id: EntityId = Uuid::now_v7();
        let nonexistent_id: EntityId = Uuid::now_v7();

        let token = generate_jwt_token(
            &auth_config,
            "user123".to_string(),
            Some(tenant_id),
            vec![],
        )
        .map_err(|e| e.message)?;

        // Try to get a trajectory that doesn't exist
        let request = Request::builder()
            .uri(format!("/api/v1/trajectories/{}", nonexistent_id))
            .method("GET")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .map_err(|e| e.to_string())?;

        let response = app
            .oneshot(request)
            .await
            .map_err(|e| format!("Request failed: {:?}", e))?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }
}
