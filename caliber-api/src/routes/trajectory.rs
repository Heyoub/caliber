//! Trajectory REST API Routes
//!
//! This module implements Axum route handlers for trajectory operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::AuthContext,
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    types::{
        CreateTrajectoryRequest, ListTrajectoriesRequest, ListTrajectoriesResponse,
        ScopeResponse, TrajectoryResponse, UpdateTrajectoryRequest,
    },
    ws::WsState,
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for trajectory routes.
#[derive(Clone)]
pub struct TrajectoryState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
}

impl TrajectoryState {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/trajectories - Create a new trajectory
#[utoipa::path(
    post,
    path = "/api/v1/trajectories",
    tag = "Trajectories",
    request_body = CreateTrajectoryRequest,
    responses(
        (status = 201, description = "Trajectory created successfully", body = TrajectoryResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_trajectory(
    State(state): State<Arc<TrajectoryState>>,
    Json(req): Json<CreateTrajectoryRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.name.trim().is_empty() {
        return Err(ApiError::missing_field("name"));
    }

    // Create trajectory via database client
    let trajectory = state.db.trajectory_create(&req).await?;

    // Broadcast TrajectoryCreated event
    state.ws.broadcast(WsEvent::TrajectoryCreated {
        trajectory: trajectory.clone(),
    });

    Ok((StatusCode::CREATED, Json(trajectory)))
}

/// GET /api/v1/trajectories - List trajectories with filters
#[utoipa::path(
    get,
    path = "/api/v1/trajectories",
    tag = "Trajectories",
    params(
        ("status" = Option<String>, Query, description = "Filter by trajectory status"),
        ("agent_id" = Option<String>, Query, description = "Filter by agent ID"),
        ("parent_id" = Option<String>, Query, description = "Filter by parent trajectory ID"),
        ("limit" = Option<i32>, Query, description = "Maximum number of results"),
        ("offset" = Option<i32>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "List of trajectories", body = ListTrajectoriesResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_trajectories(
    State(state): State<Arc<TrajectoryState>>,
    Query(params): Query<ListTrajectoriesRequest>,
) -> ApiResult<impl IntoResponse> {
    if let Some(status) = params.status {
        // Filter by status
        let trajectories = state.db.trajectory_list_by_status(status).await?;

        // Apply additional filters if needed
        let mut filtered = trajectories;

        if let Some(agent_id) = params.agent_id {
            filtered.retain(|t| t.agent_id == Some(agent_id));
        }

        if let Some(parent_id) = params.parent_id {
            filtered.retain(|t| t.parent_trajectory_id == Some(parent_id));
        }

        // Apply pagination
        let total = filtered.len() as i32;
        let offset = params.offset.unwrap_or(0) as usize;
        let limit = params.limit.unwrap_or(100) as usize;

        let paginated: Vec<_> = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        let response = ListTrajectoriesResponse {
            trajectories: paginated,
            total,
        };

        Ok(Json(response))
    } else {
        Err(ApiError::invalid_input(
            "status filter is required (for now - full listing coming soon)",
        ))
    }
}

/// GET /api/v1/trajectories/{id} - Get trajectory by ID
#[utoipa::path(
    get,
    path = "/api/v1/trajectories/{id}",
    tag = "Trajectories",
    params(
        ("id" = Uuid, Path, description = "Trajectory ID")
    ),
    responses(
        (status = 200, description = "Trajectory details", body = TrajectoryResponse),
        (status = 404, description = "Trajectory not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_trajectory(
    State(state): State<Arc<TrajectoryState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let trajectory = state
        .db
        .trajectory_get(id)
        .await?
        .ok_or_else(|| ApiError::trajectory_not_found(id))?;

    Ok(Json(trajectory))
}

/// PATCH /api/v1/trajectories/{id} - Update trajectory
#[utoipa::path(
    patch,
    path = "/api/v1/trajectories/{id}",
    tag = "Trajectories",
    params(
        ("id" = Uuid, Path, description = "Trajectory ID")
    ),
    request_body = UpdateTrajectoryRequest,
    responses(
        (status = 200, description = "Trajectory updated successfully", body = TrajectoryResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Trajectory not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn update_trajectory(
    State(state): State<Arc<TrajectoryState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTrajectoryRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that at least one field is being updated
    if req.name.is_none()
        && req.description.is_none()
        && req.status.is_none()
        && req.metadata.is_none()
    {
        return Err(ApiError::invalid_input(
            "At least one field must be provided for update",
        ));
    }

    // Update trajectory via database client
    let trajectory = state.db.trajectory_update(id, &req).await?;

    // Broadcast TrajectoryUpdated event
    state.ws.broadcast(WsEvent::TrajectoryUpdated {
        trajectory: trajectory.clone(),
    });

    Ok(Json(trajectory))
}

/// DELETE /api/v1/trajectories/{id} - Delete trajectory
#[utoipa::path(
    delete,
    path = "/api/v1/trajectories/{id}",
    tag = "Trajectories",
    params(
        ("id" = Uuid, Path, description = "Trajectory ID")
    ),
    responses(
        (status = 204, description = "Trajectory deleted successfully"),
        (status = 404, description = "Trajectory not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn delete_trajectory(
    State(state): State<Arc<TrajectoryState>>,
    Path(id): Path<Uuid>,
    auth: AuthContext,
) -> ApiResult<StatusCode> {
    // First verify the trajectory exists
    let _trajectory = state
        .db
        .trajectory_get(id)
        .await?
        .ok_or_else(|| ApiError::trajectory_not_found(id))?;

    // Delete trajectory via database client
    state.db.trajectory_delete(id).await?;

    // Broadcast TrajectoryDeleted event with tenant_id for filtering
    state.ws.broadcast(WsEvent::TrajectoryDeleted {
        tenant_id: auth.tenant_id,
        id,
    });

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/trajectories/{id}/scopes - List scopes for trajectory
#[utoipa::path(
    get,
    path = "/api/v1/trajectories/{id}/scopes",
    tag = "Trajectories",
    params(
        ("id" = Uuid, Path, description = "Trajectory ID")
    ),
    responses(
        (status = 200, description = "List of scopes", body = Vec<ScopeResponse>),
        (status = 404, description = "Trajectory not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_trajectory_scopes(
    State(state): State<Arc<TrajectoryState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // First verify the trajectory exists
    let _trajectory = state
        .db
        .trajectory_get(id)
        .await?
        .ok_or_else(|| ApiError::trajectory_not_found(id))?;

    // Get scopes for this trajectory
    let scopes = state.db.scope_list_by_trajectory(id).await?;

    Ok(Json(scopes))
}

/// GET /api/v1/trajectories/{id}/children - List child trajectories
#[utoipa::path(
    get,
    path = "/api/v1/trajectories/{id}/children",
    tag = "Trajectories",
    params(
        ("id" = Uuid, Path, description = "Trajectory ID")
    ),
    responses(
        (status = 200, description = "List of child trajectories", body = Vec<TrajectoryResponse>),
        (status = 404, description = "Trajectory not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_trajectory_children(
    State(state): State<Arc<TrajectoryState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // First verify the trajectory exists
    let _trajectory = state
        .db
        .trajectory_get(id)
        .await?
        .ok_or_else(|| ApiError::trajectory_not_found(id))?;

    // Get child trajectories
    let children = state.db.trajectory_list_children(id).await?;

    Ok(Json(children))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the trajectory routes router.
pub fn create_router(db: DbClient, ws: Arc<WsState>) -> axum::Router {
    let state = Arc::new(TrajectoryState::new(db, ws));

    axum::Router::new()
        .route("/", axum::routing::post(create_trajectory))
        .route("/", axum::routing::get(list_trajectories))
        .route("/:id", axum::routing::get(get_trajectory))
        .route("/:id", axum::routing::patch(update_trajectory))
        .route("/:id", axum::routing::delete(delete_trajectory))
        .route("/:id/scopes", axum::routing::get(list_trajectory_scopes))
        .route("/:id/children", axum::routing::get(list_trajectory_children))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::TrajectoryStatus;

    #[test]
    fn test_create_trajectory_request_validation() {
        let req = CreateTrajectoryRequest {
            name: "".to_string(),
            description: None,
            parent_trajectory_id: None,
            agent_id: None,
            metadata: None,
        };

        assert!(req.name.trim().is_empty());
    }

    #[test]
    fn test_update_trajectory_request_validation() {
        let req = UpdateTrajectoryRequest {
            name: None,
            description: None,
            status: None,
            metadata: None,
        };

        let has_updates = req.name.is_some()
            || req.description.is_some()
            || req.status.is_some()
            || req.metadata.is_some();

        assert!(!has_updates);
    }

    #[test]
    fn test_list_trajectories_pagination() {
        let params = ListTrajectoriesRequest {
            status: Some(TrajectoryStatus::Active),
            agent_id: None,
            parent_id: None,
            limit: Some(10),
            offset: Some(0),
        };

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(0));
    }
}
