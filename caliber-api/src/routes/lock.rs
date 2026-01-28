//! Lock REST API Routes
//!
//! This module implements Axum route handlers for distributed lock operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;

use crate::{
    auth::validate_tenant_ownership,
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    extractors::PathId,
    middleware::AuthExtractor,
    state::AppState,
    types::{
        AcquireLockRequest, ExtendLockRequest, ListLocksResponse, LockResponse, ReleaseLockRequest,
    },
    ws::WsState,
};
use caliber_core::LockId;

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
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
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

    // Acquire lock via database client with tenant_id for isolation
    let lock = db.lock_acquire(&req, auth.tenant_id).await?;

    // Broadcast LockAcquired event
    ws.broadcast(WsEvent::LockAcquired { lock: lock.clone() });

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
    request_body = ReleaseLockRequest,
    responses(
        (status = 204, description = "Lock released successfully"),
        (status = 403, description = "Not the lock holder", body = ApiError),
        (status = 404, description = "Lock not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn release_lock(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<LockId>,
    Json(req): Json<ReleaseLockRequest>,
) -> ApiResult<StatusCode> {
    // Get the lock and verify tenant ownership
    let lock = db
        .lock_get(id)
        .await?
        .ok_or_else(|| ApiError::lock_not_found(id))?;
    validate_tenant_ownership(&auth, Some(lock.tenant_id))?;

    // Release via Response method (validates ownership)
    lock.release(&db, req.releasing_agent_id).await?;

    // Broadcast LockReleased event with tenant_id for filtering
    ws.broadcast(WsEvent::LockReleased {
        tenant_id: auth.tenant_id,
        lock_id: id,
    });

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
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<LockId>,
    Json(req): Json<ExtendLockRequest>,
) -> ApiResult<Json<LockResponse>> {
    // Validate additional time
    if req.additional_ms <= 0 {
        return Err(ApiError::invalid_range("additional_ms", 1, i64::MAX));
    }

    // Get the lock and verify tenant ownership
    let existing = db
        .lock_get(id)
        .await?
        .ok_or_else(|| ApiError::lock_not_found(id))?;
    validate_tenant_ownership(&auth, Some(existing.tenant_id))?;

    // Extend via Response method (validates lock is held)
    let duration = std::time::Duration::from_millis(req.additional_ms as u64);
    let lock = existing.extend(&db, duration).await?;
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
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
) -> ApiResult<impl IntoResponse> {
    // List active locks filtered by tenant for isolation
    let locks = db.lock_list_active_by_tenant(auth.tenant_id).await?;
    let total = locks.len() as i32;

    Ok(Json(ListLocksResponse { locks, total }))
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
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<LockId>,
) -> ApiResult<impl IntoResponse> {
    let lock = db
        .lock_get(id)
        .await?
        .ok_or_else(|| ApiError::lock_not_found(id))?;

    // Validate tenant ownership before returning
    validate_tenant_ownership(&auth, Some(lock.tenant_id))?;

    Ok(Json(lock))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the lock routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/acquire", axum::routing::post(acquire_lock))
        .route("/:id/release", axum::routing::post(release_lock))
        .route("/:id/extend", axum::routing::post(extend_lock))
        .route("/", axum::routing::get(list_locks))
        .route("/:id", axum::routing::get(get_lock))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthContext, AuthMethod};
    use crate::db::{DbClient, DbConfig};
    use crate::extractors::PathId;
    use crate::routes::agent::register_agent;
    use crate::types::{
        AgentResponse, MemoryAccessRequest, MemoryPermissionRequest, RegisterAgentRequest,
    };
    use crate::ws::WsState;
    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
    use caliber_core::{AgentId, EntityIdType};
    use std::sync::Arc;
    use uuid::Uuid;

    struct DbTestContext {
        db: DbClient,
        auth: AuthContext,
        ws: Arc<WsState>,
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

        let tenant_id = db.tenant_create("test-lock", None, None).await.ok()?;
        let auth = AuthContext::new("test-user".to_string(), tenant_id, vec![], AuthMethod::Jwt);

        Some(DbTestContext {
            db,
            auth,
            ws: Arc::new(WsState::new(8)),
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
    fn test_acquire_lock_request_validation() {
        let req = AcquireLockRequest {
            resource_type: "".to_string(),
            resource_id: Uuid::now_v7(),
            holder_agent_id: AgentId::now_v7(),
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

    #[tokio::test]
    async fn test_acquire_list_extend_release_lock_db_backed() {
        let Some(ctx) = db_test_context().await else {
            return;
        };

        let agent = register_test_agent(&ctx, "locker").await;
        let resource_id = Uuid::now_v7();

        let acquire_req = AcquireLockRequest {
            resource_type: "test-resource".to_string(),
            resource_id,
            holder_agent_id: agent.agent_id,
            timeout_ms: 5_000,
            mode: "exclusive".to_string(),
        };

        let acquire_response = acquire_lock(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            AuthExtractor(ctx.auth.clone()),
            Json(acquire_req),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(acquire_response.status(), StatusCode::CREATED);
        let lock: LockResponse = response_json(acquire_response).await;
        assert_eq!(lock.resource_id, resource_id);

        let list_response = list_locks(State(ctx.db.clone()), AuthExtractor(ctx.auth.clone()))
            .await
            .unwrap()
            .into_response();
        assert_eq!(list_response.status(), StatusCode::OK);
        let list: ListLocksResponse = response_json(list_response).await;
        assert!(list.locks.iter().any(|l| l.lock_id == lock.lock_id));

        let extend_response = extend_lock(
            State(ctx.db.clone()),
            AuthExtractor(ctx.auth.clone()),
            PathId(lock.lock_id),
            Json(ExtendLockRequest {
                additional_ms: 1_000,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(extend_response.status(), StatusCode::OK);
        let extended: LockResponse = response_json(extend_response).await;
        assert_eq!(extended.lock_id, lock.lock_id);

        let release_status = release_lock(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            AuthExtractor(ctx.auth.clone()),
            PathId(lock.lock_id),
            Json(ReleaseLockRequest {
                releasing_agent_id: agent.agent_id,
            }),
        )
        .await
        .unwrap();
        assert_eq!(release_status, StatusCode::NO_CONTENT);
    }
}
