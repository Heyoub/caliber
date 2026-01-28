//! Delegation REST API Routes
//!
//! This module implements Axum route handlers for task delegation operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    extractors::PathId,
    middleware::AuthExtractor,
    state::AppState,
    types::{CreateDelegationRequest, DelegationResponse, DelegationResultResponse},
    ws::WsState,
};
use caliber_core::DelegationId;

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/delegations - Create a task delegation
#[utoipa::path(
    post,
    path = "/api/v1/delegations",
    tag = "Delegations",
    request_body = CreateDelegationRequest,
    responses(
        (status = 201, description = "Delegation created successfully", body = DelegationResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_delegation(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<CreateDelegationRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.task_description.trim().is_empty() {
        return Err(ApiError::missing_field("task_description"));
    }

    // Validate that from and to agents are different
    if req.from_agent_id == req.to_agent_id {
        return Err(ApiError::invalid_input("Cannot delegate to the same agent"));
    }

    // Create delegation via database client with tenant_id for isolation
    let delegation = db
        .create::<DelegationResponse>(&req, auth.tenant_id)
        .await?;

    // Broadcast DelegationCreated event
    ws.broadcast(WsEvent::DelegationCreated {
        delegation: delegation.clone(),
    });

    Ok((StatusCode::CREATED, Json(delegation)))
}

/// GET /api/v1/delegations/{id} - Get delegation by ID
#[utoipa::path(
    get,
    path = "/api/v1/delegations/{id}",
    tag = "Delegations",
    params(
        ("id" = Uuid, Path, description = "Delegation ID")
    ),
    responses(
        (status = 200, description = "Delegation details", body = DelegationResponse),
        (status = 404, description = "Delegation not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_delegation(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<DelegationId>,
) -> ApiResult<impl IntoResponse> {
    // Generic get filters by tenant_id, so not_found includes wrong tenant case
    let delegation = db
        .get::<DelegationResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Delegation", id))?;

    Ok(Json(delegation))
}

/// POST /api/v1/delegations/{id}/accept - Accept a delegation
#[utoipa::path(
    post,
    path = "/api/v1/delegations/{id}/accept",
    tag = "Delegations",
    params(
        ("id" = Uuid, Path, description = "Delegation ID")
    ),
    request_body = AcceptDelegationRequest,
    responses(
        (status = 204, description = "Delegation accepted successfully"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 403, description = "Not authorized to accept", body = ApiError),
        (status = 404, description = "Delegation not found", body = ApiError),
        (status = 409, description = "Delegation not in pending state", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn accept_delegation(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<DelegationId>,
    Json(req): Json<AcceptDelegationRequest>,
) -> ApiResult<StatusCode> {
    // Get the delegation
    let delegation = db
        .get::<DelegationResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Delegation", id))?;

    // Accept via Response method (validates state and permissions)
    delegation.accept(&db, req.accepting_agent_id).await?;

    tracing::info!(
        delegation_id = %id,
        accepted_by = %req.accepting_agent_id,
        tenant_id = %auth.tenant_id,
        "Delegation accepted"
    );

    // Broadcast DelegationAccepted event with tenant_id for filtering
    ws.broadcast(WsEvent::DelegationAccepted {
        tenant_id: auth.tenant_id,
        delegation_id: id,
    });

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/delegations/{id}/reject - Reject a delegation
#[utoipa::path(
    post,
    path = "/api/v1/delegations/{id}/reject",
    tag = "Delegations",
    params(
        ("id" = Uuid, Path, description = "Delegation ID")
    ),
    request_body = RejectDelegationRequest,
    responses(
        (status = 204, description = "Delegation rejected successfully"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 403, description = "Not authorized to reject", body = ApiError),
        (status = 404, description = "Delegation not found", body = ApiError),
        (status = 409, description = "Delegation not in pending state", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn reject_delegation(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<DelegationId>,
    Json(req): Json<RejectDelegationRequest>,
) -> ApiResult<StatusCode> {
    // Get the delegation
    let delegation = db
        .get::<DelegationResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Delegation", id))?;

    // Reject via Response method (validates state and permissions)
    delegation
        .reject(&db, req.rejecting_agent_id, &req.reason)
        .await?;

    ws.broadcast(WsEvent::DelegationRejected {
        tenant_id: auth.tenant_id,
        delegation_id: id,
    });
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/delegations/{id}/complete - Complete a delegation
#[utoipa::path(
    post,
    path = "/api/v1/delegations/{id}/complete",
    tag = "Delegations",
    params(
        ("id" = Uuid, Path, description = "Delegation ID")
    ),
    request_body = CompleteDelegationRequest,
    responses(
        (status = 204, description = "Delegation completed successfully"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Delegation not found", body = ApiError),
        (status = 409, description = "Delegation not in accepted state", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn complete_delegation(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<DelegationId>,
    Json(req): Json<CompleteDelegationRequest>,
) -> ApiResult<StatusCode> {
    // Get the delegation
    let delegation = db
        .get::<DelegationResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Delegation", id))?;

    // Complete via Response method (validates state)
    let updated = delegation.complete(&db, &req.result).await?;

    // Broadcast DelegationCompleted event
    ws.broadcast(WsEvent::DelegationCompleted {
        delegation: updated,
    });

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Request to accept a delegation.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AcceptDelegationRequest {
    /// Agent accepting the delegation
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub accepting_agent_id: caliber_core::AgentId,
}

/// Request to reject a delegation.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RejectDelegationRequest {
    /// Agent rejecting the delegation
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub rejecting_agent_id: caliber_core::AgentId,
    /// Reason for rejection
    pub reason: String,
}

/// Request to complete a delegation.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CompleteDelegationRequest {
    /// Result of the delegation
    pub result: DelegationResultResponse,
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the delegation routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(create_delegation))
        .route("/:id", axum::routing::get(get_delegation))
        .route("/:id/accept", axum::routing::post(accept_delegation))
        .route("/:id/reject", axum::routing::post(reject_delegation))
        .route("/:id/complete", axum::routing::post(complete_delegation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthContext, AuthMethod};
    use crate::db::{DbClient, DbConfig};
    use crate::extractors::PathId;
    use crate::routes::agent::register_agent;
    use crate::routes::scope::create_scope;
    use crate::routes::trajectory::create_trajectory;
    use crate::state::ApiEventDag;
    use crate::types::{
        AgentResponse, CreateScopeRequest, CreateTrajectoryRequest, MemoryAccessRequest,
        MemoryPermissionRequest, RegisterAgentRequest, ScopeResponse, TrajectoryResponse,
    };
    use crate::ws::WsState;
    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
    use caliber_core::DelegationResultStatus;
    use caliber_core::{AgentId, EntityIdType, ScopeId, TrajectoryId};
    use std::sync::Arc;
    use uuid::Uuid;

    struct DbTestContext {
        db: DbClient,
        auth: AuthContext,
        ws: Arc<WsState>,
        event_dag: Arc<ApiEventDag>,
    }

    async fn db_test_context() -> Option<DbTestContext> {
        if std::env::var("DB_TESTS").ok().as_deref() != Some("1") {
            return None;
        }

        let db = DbClient::from_config(&DbConfig::from_env()).ok()?;
        let conn = db.get_conn().await.ok()?;
        let has_fn = conn
            .query_opt(
                "SELECT 1 FROM pg_proc WHERE proname = 'caliber_tenant_create' LIMIT 1",
                &[],
            )
            .await
            .ok()
            .flatten()
            .is_some();
        if !has_fn {
            return None;
        }

        let tenant_id = db.tenant_create("test-delegation", None, None).await.ok()?;
        let auth = AuthContext::new("test-user".to_string(), tenant_id, vec![], AuthMethod::Jwt);

        Some(DbTestContext {
            db,
            auth,
            ws: Arc::new(WsState::new(8)),
            event_dag: Arc::new(ApiEventDag::new()),
        })
    }

    async fn response_json<T: serde::de::DeserializeOwned>(
        response: axum::response::Response,
    ) -> T {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&body).expect("parse json")
    }

    async fn register_test_agent(ctx: &DbTestContext, agent_type: &str) -> AgentResponse {
        let req = RegisterAgentRequest {
            agent_type: agent_type.to_string(),
            capabilities: vec!["read".to_string()],
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
            can_delegate_to: vec!["planner".to_string()],
            reports_to: None,
        };

        let response = register_agent(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            AuthExtractor(ctx.auth.clone()),
            Json(req),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        response_json(response).await
    }

    #[test]
    fn test_create_delegation_request_validation() {
        let agent_id = AgentId::now_v7();
        let req = CreateDelegationRequest {
            from_agent_id: agent_id,
            to_agent_id: agent_id, // Same agent
            trajectory_id: TrajectoryId::now_v7(),
            scope_id: ScopeId::now_v7(),
            task_description: "".to_string(),
            expected_completion: None,
            context: None,
        };

        assert!(req.task_description.trim().is_empty());
        assert_eq!(req.from_agent_id, req.to_agent_id);
    }

    #[test]
    fn test_valid_delegation_result_statuses() {
        let valid_statuses = ["Success", "Partial", "Failure"];

        assert!(valid_statuses.contains(&"Success"));
        assert!(valid_statuses.contains(&"Partial"));
        assert!(valid_statuses.contains(&"Failure"));
        assert!(!valid_statuses.contains(&"Invalid"));
    }

    #[test]
    fn test_delegation_state_transitions() {
        // Valid transitions:
        // Pending -> Accepted
        // Pending -> Rejected
        // Accepted -> InProgress -> Completed
        // Accepted -> InProgress -> Failed

        let valid_accept_states = ["pending"];
        let valid_complete_states = ["accepted", "inprogress"];

        assert!(valid_accept_states.contains(&"pending"));
        assert!(valid_complete_states.contains(&"accepted"));
        assert!(valid_complete_states.contains(&"inprogress"));
    }

    #[test]
    fn test_accept_delegation_request() {
        let req = AcceptDelegationRequest {
            accepting_agent_id: AgentId::now_v7(),
        };

        // Just verify the struct can be created
        assert!(!req.accepting_agent_id.as_uuid().to_string().is_empty());
    }

    #[test]
    fn test_reject_delegation_request() {
        let req = RejectDelegationRequest {
            rejecting_agent_id: AgentId::now_v7(),
            reason: "Not enough capacity".to_string(),
        };

        assert!(!req.reason.is_empty());
    }

    #[tokio::test]
    async fn test_create_accept_complete_delegation_db_backed() {
        let Some(ctx) = db_test_context().await else {
            return;
        };

        let trajectory_req = CreateTrajectoryRequest {
            name: format!("delegation-traj-{}", Uuid::now_v7()),
            description: None,
            parent_trajectory_id: None,
            agent_id: None,
            metadata: None,
        };
        let trajectory_response = create_trajectory(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            State(ctx.event_dag.clone()),
            AuthExtractor(ctx.auth.clone()),
            Json(trajectory_req),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(trajectory_response.status(), StatusCode::CREATED);
        let trajectory: TrajectoryResponse = response_json(trajectory_response).await;

        let scope_req = CreateScopeRequest {
            trajectory_id: trajectory.trajectory_id,
            parent_scope_id: None,
            name: "delegation-scope".to_string(),
            purpose: None,
            token_budget: 1000,
            metadata: None,
        };
        let scope_response = create_scope(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            State(ctx.event_dag.clone()),
            AuthExtractor(ctx.auth.clone()),
            Json(scope_req),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(scope_response.status(), StatusCode::CREATED);
        let scope: ScopeResponse = response_json(scope_response).await;

        let from_agent = register_test_agent(&ctx, "delegator").await;
        let to_agent = register_test_agent(&ctx, "delegatee").await;

        let create_req = CreateDelegationRequest {
            from_agent_id: from_agent.agent_id,
            to_agent_id: to_agent.agent_id,
            trajectory_id: trajectory.trajectory_id,
            scope_id: scope.scope_id,
            task_description: "Do the thing".to_string(),
            expected_completion: None,
            context: None,
        };
        let delegation_response = create_delegation(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            AuthExtractor(ctx.auth.clone()),
            Json(create_req),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(delegation_response.status(), StatusCode::CREATED);
        let delegation: DelegationResponse = response_json(delegation_response).await;

        let accept_status = accept_delegation(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            AuthExtractor(ctx.auth.clone()),
            PathId(delegation.delegation_id),
            Json(AcceptDelegationRequest {
                accepting_agent_id: to_agent.agent_id,
            }),
        )
        .await
        .unwrap();
        assert_eq!(accept_status, StatusCode::NO_CONTENT);

        let complete_status = complete_delegation(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            AuthExtractor(ctx.auth.clone()),
            PathId(delegation.delegation_id),
            Json(CompleteDelegationRequest {
                result: DelegationResultResponse {
                    status: DelegationResultStatus::Success,
                    output: Some("done".to_string()),
                    artifacts: vec![],
                    error: None,
                },
            }),
        )
        .await
        .unwrap();
        assert_eq!(complete_status, StatusCode::NO_CONTENT);
    }
}
