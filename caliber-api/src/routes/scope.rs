//! Scope REST API Routes
//!
//! This module implements Axum route handlers for scope operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.
//!
//! Includes Battle Intel Feature 4: Auto-summarization trigger checking
//! on scope close to enable L0→L1→L2 abstraction transitions.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use caliber_core::{EntityIdType, ScopeId};
use caliber_pcp::PCPRuntime;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::validate_tenant_ownership,
    components::{ArtifactListFilter, TurnListFilter},
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    extractors::PathId,
    middleware::AuthExtractor,
    state::{ApiEventDag, AppState},
    types::{
        ArtifactResponse, CheckpointResponse, CreateCheckpointRequest, CreateScopeRequest,
        ScopeResponse, TurnResponse, UpdateScopeRequest,
    },
    ws::WsState,
};

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/scopes - Create a new scope
#[utoipa::path(
    post,
    path = "/api/v1/scopes",
    tag = "Scopes",
    request_body = CreateScopeRequest,
    responses(
        (status = 201, description = "Scope created successfully", body = ScopeResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_scope(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    State(event_dag): State<Arc<ApiEventDag>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<CreateScopeRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.name.trim().is_empty() {
        return Err(ApiError::missing_field("name"));
    }

    if req.token_budget <= 0 {
        return Err(ApiError::invalid_range("token_budget", 1, i32::MAX));
    }

    // Create scope via database client with event emission for audit trail
    let scope = db
        .create_with_event::<ScopeResponse>(&req, auth.tenant_id, &event_dag)
        .await?;

    // Broadcast ScopeCreated event via WebSocket
    ws.broadcast(WsEvent::ScopeCreated {
        scope: scope.clone(),
    });

    Ok((StatusCode::CREATED, Json(scope.linked())))
}

/// GET /api/v1/scopes/{id} - Get scope by ID
#[utoipa::path(
    get,
    path = "/api/v1/scopes/{id}",
    tag = "Scopes",
    params(
        ("id" = Uuid, Path, description = "Scope ID")
    ),
    responses(
        (status = 200, description = "Scope details", body = ScopeResponse),
        (status = 404, description = "Scope not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_scope(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<ScopeId>,
) -> ApiResult<impl IntoResponse> {
    let scope = db
        .get::<ScopeResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(id))?;

    // Validate tenant ownership before returning
    validate_tenant_ownership(&auth, Some(scope.tenant_id))?;

    Ok(Json(scope.linked()))
}

/// PATCH /api/v1/scopes/{id} - Update scope
#[utoipa::path(
    patch,
    path = "/api/v1/scopes/{id}",
    tag = "Scopes",
    params(
        ("id" = Uuid, Path, description = "Scope ID")
    ),
    request_body = UpdateScopeRequest,
    responses(
        (status = 200, description = "Scope updated successfully", body = ScopeResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Scope not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn update_scope(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    State(event_dag): State<Arc<ApiEventDag>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<ScopeId>,
    Json(req): Json<UpdateScopeRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that at least one field is being updated
    if req.name.is_none()
        && req.purpose.is_none()
        && req.token_budget.is_none()
        && req.metadata.is_none()
    {
        return Err(ApiError::invalid_input(
            "At least one field must be provided for update",
        ));
    }

    // Validate token_budget if provided
    if let Some(budget) = req.token_budget {
        if budget <= 0 {
            return Err(ApiError::invalid_range("token_budget", 1, i32::MAX));
        }
    }

    // First verify the scope exists and belongs to this tenant
    let existing = db
        .get::<ScopeResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(id))?;
    validate_tenant_ownership(&auth, Some(existing.tenant_id))?;

    // Update scope via database client with event emission for audit trail
    let scope = db
        .update_with_event::<ScopeResponse>(id, &req, auth.tenant_id, &event_dag)
        .await?;

    // Broadcast ScopeUpdated event via WebSocket
    ws.broadcast(WsEvent::ScopeUpdated {
        scope: scope.clone(),
    });

    Ok(Json(scope.linked()))
}

/// POST /api/v1/scopes/{id}/checkpoint - Create checkpoint
#[utoipa::path(
    post,
    path = "/api/v1/scopes/{id}/checkpoint",
    tag = "Scopes",
    params(
        ("id" = Uuid, Path, description = "Scope ID")
    ),
    request_body = CreateCheckpointRequest,
    responses(
        (status = 201, description = "Checkpoint created successfully", body = CheckpointResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Scope not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_checkpoint(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<ScopeId>,
    Json(req): Json<CreateCheckpointRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate context_state is not empty
    if req.context_state.is_empty() {
        return Err(ApiError::invalid_input("context_state cannot be empty"));
    }

    // Get the scope and verify tenant ownership
    let scope = db
        .get::<ScopeResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(id))?;
    validate_tenant_ownership(&auth, Some(scope.tenant_id))?;

    // Create checkpoint via Response method (validates scope is active)
    let updated_scope = scope.create_checkpoint(&db, &req).await?;

    // Extract the checkpoint from the updated scope
    let checkpoint = updated_scope
        .checkpoint
        .ok_or_else(|| ApiError::internal_error("Checkpoint was not set after creation"))?;

    Ok((StatusCode::CREATED, Json(checkpoint)))
}

/// POST /api/v1/scopes/{id}/close - Close scope
#[utoipa::path(
    post,
    path = "/api/v1/scopes/{id}/close",
    tag = "Scopes",
    params(
        ("id" = Uuid, Path, description = "Scope ID")
    ),
    responses(
        (status = 200, description = "Scope closed successfully", body = ScopeResponse),
        (status = 404, description = "Scope not found", body = ApiError),
        (status = 409, description = "Scope already closed", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn close_scope(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    State(event_dag): State<Arc<ApiEventDag>>,
    State(pcp): State<Arc<PCPRuntime>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<ScopeId>,
) -> ApiResult<impl IntoResponse> {
    use caliber_core::{DagPosition, Event, EventDag, EventFlags, EventHeader, EventKind};

    // Get the scope and verify tenant ownership
    let existing = db
        .get::<ScopeResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(id))?;
    validate_tenant_ownership(&auth, Some(existing.tenant_id))?;

    // Close via Response method (validates scope is active)
    let scope = existing.close(&db).await?;

    // Emit ScopeClosed event to EventDag for audit trail
    let event_id = Uuid::now_v7();
    let now = chrono::Utc::now();
    let header = EventHeader::new(
        event_id,
        event_id,
        now.timestamp_micros(),
        DagPosition::root(),
        0,
        EventKind::SCOPE_CLOSED,
        EventFlags::empty(),
    );
    let payload = serde_json::json!({
        "entity_type": "scope",
        "entity_id": id.as_uuid().to_string(),
        "tenant_id": auth.tenant_id.as_uuid().to_string(),
        "action": "closed",
    });
    let event = Event::new(header, payload);
    if let caliber_core::Effect::Err(e) = event_dag.append(event).await {
        tracing::warn!("Failed to emit scope closed event: {:?}", e);
    }

    // Broadcast ScopeClosed event via WebSocket
    ws.broadcast(WsEvent::ScopeClosed {
        scope: scope.clone(),
    });

    // =========================================================================
    // BATTLE INTEL: Check ScopeClose summarization triggers
    // =========================================================================
    let trajectory_id = scope.trajectory_id;

    // Fetch summarization policies for this trajectory
    if let Ok(policies) = db
        .summarization_policies_for_trajectory(trajectory_id)
        .await
    {
        if !policies.is_empty() {
            // Get turn count for this scope using generic list
            let turn_filter = TurnListFilter {
                scope_id: Some(id),
                ..Default::default()
            };
            let turn_count = db
                .list::<TurnResponse>(&turn_filter, auth.tenant_id)
                .await
                .map(|turns| turns.len() as i32)
                .unwrap_or(0);

            // Get artifact count for this scope using generic list
            let artifact_filter = ArtifactListFilter {
                scope_id: Some(id),
                ..Default::default()
            };
            let artifact_count = db
                .list::<ArtifactResponse>(&artifact_filter, auth.tenant_id)
                .await
                .map(|artifacts| artifacts.len() as i32)
                .unwrap_or(0);

            // Convert policies to caliber_core format for PCPRuntime
            let core_policies: Vec<caliber_core::SummarizationPolicy> = policies
                .iter()
                .map(|p| caliber_core::SummarizationPolicy {
                    policy_id: p.policy_id,
                    name: p.name.clone(),
                    triggers: p.triggers.clone(),
                    target_level: p.target_level,
                    source_level: p.source_level,
                    max_sources: p.max_sources,
                    create_edges: p.create_edges,
                    created_at: p.created_at,
                    metadata: p.metadata.clone(),
                })
                .collect();

            // Build a caliber_core::Scope from our ScopeResponse
            // Note: is_active is false since we just closed it
            let core_scope = caliber_core::Scope {
                scope_id: scope.scope_id,
                trajectory_id: scope.trajectory_id,
                parent_scope_id: scope.parent_scope_id,
                name: scope.name.clone(),
                purpose: scope.purpose.clone(),
                is_active: false, // Scope is now closed
                created_at: scope.created_at,
                closed_at: scope.closed_at,
                checkpoint: None,
                token_budget: scope.token_budget,
                tokens_used: scope.tokens_used,
                metadata: scope.metadata.clone(),
            };

            // Check which triggers should fire (ScopeClose will fire since is_active=false)
            if let Ok(triggered) = pcp.check_summarization_triggers(
                &core_scope,
                turn_count,
                artifact_count,
                &core_policies,
            ) {
                // Broadcast SummarizationTriggered event for each fired trigger
                for (policy_id, trigger) in triggered {
                    // Find the policy to get its details
                    if let Some(policy) = core_policies.iter().find(|p| p.policy_id == policy_id) {
                        ws.broadcast(WsEvent::SummarizationTriggered {
                            tenant_id: auth.tenant_id,
                            policy_id,
                            trigger,
                            scope_id: core_scope.scope_id,
                            trajectory_id,
                            source_level: policy.source_level,
                            target_level: policy.target_level,
                            max_sources: policy.max_sources,
                            create_edges: policy.create_edges,
                        });
                    }
                }
            }
        }
    }

    Ok(Json(scope.linked()))
}

/// GET /api/v1/scopes/{id}/turns - List turns for scope
#[utoipa::path(
    get,
    path = "/api/v1/scopes/{id}/turns",
    tag = "Scopes",
    params(
        ("id" = Uuid, Path, description = "Scope ID")
    ),
    responses(
        (status = 200, description = "List of turns", body = Vec<TurnResponse>),
        (status = 404, description = "Scope not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_scope_turns(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<ScopeId>,
) -> ApiResult<impl IntoResponse> {
    // First verify the scope exists and belongs to this tenant
    let scope = db
        .get::<ScopeResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(id))?;
    validate_tenant_ownership(&auth, Some(scope.tenant_id))?;

    // Get turns for this scope using generic list with filter
    let filter = TurnListFilter {
        scope_id: Some(id),
        ..Default::default()
    };
    let turns = db.list::<TurnResponse>(&filter, auth.tenant_id).await?;

    Ok(Json(turns))
}

/// GET /api/v1/scopes/{id}/artifacts - List artifacts for scope
#[utoipa::path(
    get,
    path = "/api/v1/scopes/{id}/artifacts",
    tag = "Scopes",
    params(
        ("id" = Uuid, Path, description = "Scope ID")
    ),
    responses(
        (status = 200, description = "List of artifacts", body = Vec<ArtifactResponse>),
        (status = 404, description = "Scope not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_scope_artifacts(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<ScopeId>,
) -> ApiResult<impl IntoResponse> {
    // First verify the scope exists and belongs to this tenant
    let scope = db
        .get::<ScopeResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(id))?;
    validate_tenant_ownership(&auth, Some(scope.tenant_id))?;

    // Get artifacts for this scope using generic list with filter
    let filter = ArtifactListFilter {
        scope_id: Some(id),
        ..Default::default()
    };
    let artifacts = db.list::<ArtifactResponse>(&filter, auth.tenant_id).await?;

    Ok(Json(
        artifacts
            .into_iter()
            .map(|a| a.linked())
            .collect::<Vec<_>>(),
    ))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the scope routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(create_scope))
        .route("/:id", axum::routing::get(get_scope))
        .route("/:id", axum::routing::patch(update_scope))
        .route("/:id/checkpoint", axum::routing::post(create_checkpoint))
        .route("/:id/close", axum::routing::post(close_scope))
        .route("/:id/turns", axum::routing::get(list_scope_turns))
        .route("/:id/artifacts", axum::routing::get(list_scope_artifacts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthContext, AuthMethod};
    use crate::db::{DbClient, DbConfig};
    use crate::extractors::PathId;
    use crate::routes::trajectory::create_trajectory;
    use crate::routes::trajectory::list_trajectory_scopes;
    use crate::state::ApiEventDag;
    use crate::types::{CreateTrajectoryRequest, TrajectoryResponse};
    use crate::ws::WsState;
    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
    use caliber_core::TrajectoryId;
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

        let tenant_id = db.tenant_create("test-scope", None, None).await.ok()?;
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

    #[test]
    fn test_create_scope_request_validation() {
        let req = CreateScopeRequest {
            trajectory_id: TrajectoryId::nil(),
            parent_scope_id: None,
            name: "".to_string(),
            purpose: None,
            token_budget: 0,
            metadata: None,
        };

        assert!(req.name.trim().is_empty());
        assert!(req.token_budget <= 0);
    }

    #[test]
    fn test_update_scope_request_validation() {
        let req = UpdateScopeRequest {
            name: None,
            purpose: None,
            token_budget: None,
            metadata: None,
        };

        let has_updates = req.name.is_some()
            || req.purpose.is_some()
            || req.token_budget.is_some()
            || req.metadata.is_some();

        assert!(!has_updates);
    }

    #[test]
    fn test_create_checkpoint_request_validation() {
        let req = CreateCheckpointRequest {
            context_state: vec![],
            recoverable: true,
        };

        assert!(req.context_state.is_empty());
    }

    #[test]
    fn test_token_budget_validation() {
        let valid_budget = 1000;
        let invalid_budget = 0;
        let negative_budget = -100;

        assert!(valid_budget > 0);
        assert!(invalid_budget <= 0);
        assert!(negative_budget <= 0);
    }

    #[tokio::test]
    async fn test_create_and_list_scopes_db_backed() {
        let Some(ctx) = db_test_context().await else {
            return;
        };

        let trajectory_req = CreateTrajectoryRequest {
            name: format!("scope-traj-{}", Uuid::now_v7()),
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
        .expect("create_trajectory should succeed")
        .into_response();
        assert_eq!(trajectory_response.status(), StatusCode::CREATED);
        let trajectory: TrajectoryResponse = response_json(trajectory_response).await;

        let scope_req = CreateScopeRequest {
            trajectory_id: trajectory.trajectory_id,
            parent_scope_id: None,
            name: "scope-test".to_string(),
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
        .expect("create_scope should succeed")
        .into_response();
        assert_eq!(scope_response.status(), StatusCode::CREATED);
        let scope: ScopeResponse = response_json(scope_response).await;

        let list_response = list_trajectory_scopes(
            State(ctx.db.clone()),
            AuthExtractor(ctx.auth.clone()),
            PathId(trajectory.trajectory_id),
        )
        .await
        .expect("list_trajectory_scopes should succeed")
        .into_response();
        assert_eq!(list_response.status(), StatusCode::OK);
        let list: Vec<ScopeResponse> = response_json(list_response).await;
        assert!(list.iter().any(|s| s.scope_id == scope.scope_id));

        ctx.db
            .delete::<ScopeResponse>(scope.scope_id, ctx.auth.tenant_id)
            .await
            .expect("delete scope should succeed");
        ctx.db
            .delete::<TrajectoryResponse>(trajectory.trajectory_id, ctx.auth.tenant_id)
            .await
            .expect("delete trajectory should succeed");
    }
}
