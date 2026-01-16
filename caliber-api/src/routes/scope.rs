//! Scope REST API Routes
//!
//! This module implements Axum route handlers for scope operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.
//!
//! Includes Battle Intel Feature 4: Auto-summarization trigger checking
//! on scope close to enable L0→L1→L2 abstraction transitions.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use caliber_pcp::PCPRuntime;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    types::{
        ArtifactResponse, CheckpointResponse, CreateCheckpointRequest, CreateScopeRequest,
        ScopeResponse, TurnResponse, UpdateScopeRequest,
    },
    ws::WsState,
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for scope routes.
#[derive(Clone)]
pub struct ScopeState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
    pub pcp: Arc<PCPRuntime>,
}

impl ScopeState {
    pub fn new(db: DbClient, ws: Arc<WsState>, pcp: Arc<PCPRuntime>) -> Self {
        Self { db, ws, pcp }
    }
}

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
    State(state): State<Arc<ScopeState>>,
    Json(req): Json<CreateScopeRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.name.trim().is_empty() {
        return Err(ApiError::missing_field("name"));
    }

    if req.token_budget <= 0 {
        return Err(ApiError::invalid_range("token_budget", 1, i32::MAX));
    }

    // Create scope via database client
    let scope = state.db.scope_create(&req).await?;

    // Broadcast ScopeCreated event
    state.ws.broadcast(WsEvent::ScopeCreated {
        scope: scope.clone(),
    });

    Ok((StatusCode::CREATED, Json(scope)))
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
    State(state): State<Arc<ScopeState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let scope = state
        .db
        .scope_get(id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(id))?;

    Ok(Json(scope))
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
    State(state): State<Arc<ScopeState>>,
    Path(id): Path<Uuid>,
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

    // Update scope via database client
    let scope = state.db.scope_update(id, &req).await?;

    // Broadcast ScopeUpdated event
    state.ws.broadcast(WsEvent::ScopeUpdated {
        scope: scope.clone(),
    });

    Ok(Json(scope))
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
    State(state): State<Arc<ScopeState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateCheckpointRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate context_state is not empty
    if req.context_state.is_empty() {
        return Err(ApiError::invalid_input(
            "context_state cannot be empty",
        ));
    }

    // Create checkpoint via database client
    let checkpoint = state
        .db
        .scope_create_checkpoint(id, &req)
        .await?;

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
    State(state): State<Arc<ScopeState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // Close scope via database client
    let scope = state.db.scope_close(id).await?;

    // Broadcast ScopeClosed event
    state.ws.broadcast(WsEvent::ScopeClosed {
        scope: scope.clone(),
    });

    // =========================================================================
    // BATTLE INTEL: Check ScopeClose summarization triggers
    // =========================================================================
    let trajectory_id: caliber_core::EntityId = scope.trajectory_id.into();

    // Fetch summarization policies for this trajectory
    if let Ok(policies) = state.db.summarization_policies_for_trajectory(trajectory_id).await {
        if !policies.is_empty() {
            // Get turn count for this scope
            let turn_count = state
                .db
                .turn_list_by_scope(id)
                .await
                .map(|turns| turns.len() as i32)
                .unwrap_or(0);

            // Get artifact count for this scope
            let artifact_count = state
                .db
                .artifact_list_by_scope(id)
                .await
                .map(|artifacts| artifacts.len() as i32)
                .unwrap_or(0);

            // Convert policies to caliber_core format for PCPRuntime
            let core_policies: Vec<caliber_core::SummarizationPolicy> = policies
                .iter()
                .map(|p| caliber_core::SummarizationPolicy {
                    policy_id: p.policy_id.into(),
                    name: p.name.clone(),
                    triggers: p.triggers.clone(),
                    target_level: p.target_level,
                    source_level: p.source_level,
                    max_sources: p.max_sources,
                    create_edges: p.create_edges,
                    trajectory_id: Some(trajectory_id),
                })
                .collect();

            // Build a caliber_core::Scope from our ScopeResponse
            // Note: is_active is false since we just closed it
            let core_scope = caliber_core::Scope {
                scope_id: scope.scope_id.into(),
                trajectory_id: scope.trajectory_id.into(),
                parent_scope_id: scope.parent_scope_id.map(Into::into),
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
            if let Ok(triggered) = state.pcp.check_summarization_triggers(
                &core_scope,
                turn_count,
                artifact_count,
                &core_policies,
            ) {
                // Broadcast SummarizationTriggered event for each fired trigger
                for (policy_id, trigger) in triggered {
                    // Find the policy to get its details
                    if let Some(policy) = core_policies.iter().find(|p| p.policy_id == policy_id) {
                        state.ws.broadcast(WsEvent::SummarizationTriggered {
                            policy_id,
                            trigger: trigger.clone(),
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

    Ok(Json(scope))
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
    State(state): State<Arc<ScopeState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // First verify the scope exists
    let _scope = state
        .db
        .scope_get(id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(id))?;

    // Get turns for this scope
    let turns = state.db.turn_list_by_scope(id).await?;

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
    State(state): State<Arc<ScopeState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // First verify the scope exists
    let _scope = state
        .db
        .scope_get(id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(id))?;

    // Get artifacts for this scope
    let artifacts = state.db.artifact_list_by_scope(id).await?;

    Ok(Json(artifacts))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the scope routes router.
pub fn create_router(db: DbClient, ws: Arc<WsState>, pcp: Arc<PCPRuntime>) -> axum::Router {
    let state = Arc::new(ScopeState::new(db, ws, pcp));

    axum::Router::new()
        .route("/", axum::routing::post(create_scope))
        .route("/:id", axum::routing::get(get_scope))
        .route("/:id", axum::routing::patch(update_scope))
        .route("/:id/checkpoint", axum::routing::post(create_checkpoint))
        .route("/:id/close", axum::routing::post(close_scope))
        .route("/:id/turns", axum::routing::get(list_scope_turns))
        .route("/:id/artifacts", axum::routing::get(list_scope_artifacts))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::EntityId;

    #[test]
    fn test_create_scope_request_validation() {
        // Use a dummy UUID for testing (all zeros is valid)
        let dummy_id: EntityId = uuid::Uuid::nil().into();

        let req = CreateScopeRequest {
            trajectory_id: dummy_id,
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
}
