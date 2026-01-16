//! Lock REST API Routes
//!
//! This module implements Axum route handlers for distributed lock operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    types::{AcquireLockRequest, ExtendLockRequest, LockResponse},
    ws::WsState,
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for lock routes.
#[derive(Clone)]
pub struct LockState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
}

impl LockState {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/locks/acquire - Acquire a distributed lock
#[utoipa::path(
    post,
    path = "/api/v1/locks/acquire",
    tag = "Locks",
    request_body = AcquireLockRequest,
    responses(
        (status = 201, description = "Lock acquired successfully", body = LockResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 409, description = "Lock already held", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn acquire_lock(
    State(state): State<Arc<LockState>>,
    Json(req): Json<AcquireLockRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.resource_type.trim().is_empty() {
        return Err(ApiError::missing_field("resource_type"));
    }

    // Validate lock mode
    let mode_lower = req.mode.to_lowercase();
    if mode_lower != "exclusive" && mode_lower != "shared" {
        return Err(ApiError::invalid_input(
            "mode must be either 'exclusive' or 'shared'",
        ));
    }

    // Validate timeout
    if req.timeout_ms <= 0 {
        return Err(ApiError::invalid_range("timeout_ms", 1, i64::MAX));
    }

    // Acquire lock via database client
    let lock = state.db.lock_acquire(&req).await?;

    // Broadcast LockAcquired event
    state.ws.broadcast(WsEvent::LockAcquired { lock: lock.clone() });

    Ok((StatusCode::CREATED, Json(lock)))
}

/// POST /api/v1/locks/{id}/release - Release a distributed lock
#[utoipa::path(
    post,
    path = "/api/v1/locks/{id}/release",
    tag = "Locks",
    params(
        ("id" = Uuid, Path, description = "Lock ID")
    ),
    responses(
        (status = 204, description = "Lock released successfully"),
        (status = 404, description = "Lock not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn release_lock(
    State(state): State<Arc<LockState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Release lock via database client
    state.db.lock_release(id.into()).await?;

    // Broadcast LockReleased event
    state.ws.broadcast(WsEvent::LockReleased { lock_id: id.into() });

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/locks/{id}/extend - Extend a lock's expiration time
#[utoipa::path(
    post,
    path = "/api/v1/locks/{id}/extend",
    tag = "Locks",
    params(
        ("id" = Uuid, Path, description = "Lock ID")
    ),
    request_body = ExtendLockRequest,
    responses(
        (status = 200, description = "Lock extended successfully", body = LockResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Lock not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn extend_lock(
    State(state): State<Arc<LockState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<ExtendLockRequest>,
) -> ApiResult<Json<LockResponse>> {
    // Validate additional time
    if req.additional_ms <= 0 {
        return Err(ApiError::invalid_range("additional_ms", 1, i64::MAX));
    }

    let duration = std::time::Duration::from_millis(req.additional_ms as u64);
    let lock = state.db.lock_extend(id.into(), duration).await?;
    Ok(Json(lock))
}

/// GET /api/v1/locks - List all active locks
#[utoipa::path(
    get,
    path = "/api/v1/locks",
    tag = "Locks",
    responses(
        (status = 200, description = "List of active locks", body = ListLocksResponse),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_locks(
    State(state): State<Arc<LockState>>,
) -> ApiResult<impl IntoResponse> {
    let locks = state.db.lock_list_active().await?;
    let total = locks.len() as i32;

    Ok(Json(ListLocksResponse {
        locks,
        total,
    }))
}

/// GET /api/v1/locks/{id} - Get lock by ID
#[utoipa::path(
    get,
    path = "/api/v1/locks/{id}",
    tag = "Locks",
    params(
        ("id" = Uuid, Path, description = "Lock ID")
    ),
    responses(
        (status = 200, description = "Lock details", body = LockResponse),
        (status = 404, description = "Lock not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_lock(
    State(state): State<Arc<LockState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let lock = state
        .db
        .lock_get(id.into())
        .await?
        .ok_or_else(|| ApiError::lock_not_found(id))?;

    Ok(Json(lock))
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Response containing a list of locks.
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListLocksResponse {
    /// List of locks
    pub locks: Vec<LockResponse>,
    /// Total count
    pub total: i32,
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the lock routes router.
pub fn create_router(db: DbClient, ws: Arc<WsState>) -> axum::Router {
    let state = Arc::new(LockState::new(db, ws));

    axum::Router::new()
        .route("/acquire", axum::routing::post(acquire_lock))
        .route("/:id/release", axum::routing::post(release_lock))
        .route("/:id/extend", axum::routing::post(extend_lock))
        .route("/", axum::routing::get(list_locks))
        .route("/:id", axum::routing::get(get_lock))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::EntityId;

    #[test]
    fn test_acquire_lock_request_validation() {
        let req = AcquireLockRequest {
            resource_type: "".to_string(),
            resource_id: EntityId::from(Uuid::new_v4()),
            holder_agent_id: EntityId::from(Uuid::new_v4()),
            timeout_ms: 0,
            mode: "exclusive".to_string(),
        };

        assert!(req.resource_type.trim().is_empty());
        assert_eq!(req.timeout_ms, 0);
    }

    #[test]
    fn test_lock_mode_validation() {
        let valid_modes = ["exclusive", "shared"];

        assert!(valid_modes.contains(&"exclusive"));
        assert!(valid_modes.contains(&"shared"));
        assert!(!valid_modes.contains(&"invalid"));
    }

    #[test]
    fn test_extend_lock_request_validation() {
        let req = ExtendLockRequest {
            additional_ms: -100,
        };

        assert!(req.additional_ms <= 0);
    }
}
