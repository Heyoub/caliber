//! Artifact REST API Routes
//!
//! This module implements Axum route handlers for artifact operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use caliber_core::ArtifactId;
use std::sync::Arc;

use crate::{
    auth::validate_tenant_ownership,
    components::ArtifactListFilter,
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    extractors::PathId,
    middleware::AuthExtractor,
    state::AppState,
    types::{
        ArtifactResponse, CreateArtifactRequest, ListArtifactsRequest, ListArtifactsResponse,
        SearchRequest, SearchResponse, UpdateArtifactRequest,
    },
    ws::WsState,
};

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/artifacts - Create a new artifact
#[utoipa::path(
    post,
    path = "/api/v1/artifacts",
    tag = "Artifacts",
    request_body = CreateArtifactRequest,
    responses(
        (status = 201, description = "Artifact created successfully", body = ArtifactResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_artifact(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<CreateArtifactRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.name.trim().is_empty() {
        return Err(ApiError::missing_field("name"));
    }

    if req.content.trim().is_empty() {
        return Err(ApiError::missing_field("content"));
    }

    if req.source_turn < 0 {
        return Err(ApiError::invalid_range("source_turn", 0, i32::MAX));
    }

    // Validate confidence if provided
    if let Some(confidence) = req.confidence {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(ApiError::invalid_range("confidence", 0.0, 1.0));
        }
    }

    // Create artifact via database client with tenant_id for isolation
    let artifact = db.create::<ArtifactResponse>(&req, auth.tenant_id).await?;

    // Broadcast ArtifactCreated event
    ws.broadcast(WsEvent::ArtifactCreated {
        artifact: artifact.clone(),
    });

    Ok((StatusCode::CREATED, Json(artifact.linked())))
}

/// GET /api/v1/artifacts - List artifacts with filters
#[utoipa::path(
    get,
    path = "/api/v1/artifacts",
    tag = "Artifacts",
    params(
        ("artifact_type" = Option<String>, Query, description = "Filter by artifact type"),
        ("trajectory_id" = Option<String>, Query, description = "Filter by trajectory ID"),
        ("scope_id" = Option<String>, Query, description = "Filter by scope ID"),
        ("created_after" = Option<String>, Query, description = "Filter by creation date (after)"),
        ("created_before" = Option<String>, Query, description = "Filter by creation date (before)"),
        ("limit" = Option<i32>, Query, description = "Maximum number of results"),
        ("offset" = Option<i32>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "List of artifacts", body = ListArtifactsResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_artifacts(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Query(params): Query<ListArtifactsRequest>,
) -> ApiResult<impl IntoResponse> {
    // Build filter from query params - require scope_id or trajectory_id
    if params.scope_id.is_none() && params.trajectory_id.is_none() {
        return Err(ApiError::invalid_input(
            "Either scope_id or trajectory_id filter is required",
        ));
    }

    // Use generic list with ArtifactListFilter
    let filter = ArtifactListFilter {
        trajectory_id: params.trajectory_id,
        scope_id: params.scope_id,
        artifact_type: params.artifact_type,
        limit: params.limit,
        offset: params.offset,
    };

    let mut artifacts = db.list::<ArtifactResponse>(&filter, auth.tenant_id).await?;

    // Apply date filters in Rust (not supported in filter)
    if let Some(created_after) = params.created_after {
        artifacts.retain(|a| a.created_at >= created_after);
    }
    if let Some(created_before) = params.created_before {
        artifacts.retain(|a| a.created_at <= created_before);
    }

    let total = artifacts.len() as i32;
    let response = ListArtifactsResponse {
        artifacts: artifacts.into_iter().map(|a| a.linked()).collect(),
        total,
    };

    Ok(Json(response))
}

/// GET /api/v1/artifacts/{id} - Get artifact by ID
#[utoipa::path(
    get,
    path = "/api/v1/artifacts/{id}",
    tag = "Artifacts",
    params(
        ("id" = Uuid, Path, description = "Artifact ID")
    ),
    responses(
        (status = 200, description = "Artifact details", body = ArtifactResponse),
        (status = 404, description = "Artifact not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_artifact(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<ArtifactId>,
) -> ApiResult<impl IntoResponse> {
    let artifact = db
        .get::<ArtifactResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::artifact_not_found(id))?;

    // Validate tenant ownership before returning
    validate_tenant_ownership(&auth, Some(artifact.tenant_id))?;

    Ok(Json(artifact.linked()))
}

/// PATCH /api/v1/artifacts/{id} - Update artifact
#[utoipa::path(
    patch,
    path = "/api/v1/artifacts/{id}",
    tag = "Artifacts",
    params(
        ("id" = Uuid, Path, description = "Artifact ID")
    ),
    request_body = UpdateArtifactRequest,
    responses(
        (status = 200, description = "Artifact updated successfully", body = ArtifactResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Artifact not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn update_artifact(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<ArtifactId>,
    Json(req): Json<UpdateArtifactRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that at least one field is being updated
    if req.name.is_none()
        && req.content.is_none()
        && req.artifact_type.is_none()
        && req.ttl.is_none()
        && req.metadata.is_none()
    {
        return Err(ApiError::invalid_input(
            "At least one field must be provided for update",
        ));
    }

    // Validate name if provided
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(ApiError::invalid_input("name cannot be empty"));
        }
    }

    // Validate content if provided
    if let Some(ref content) = req.content {
        if content.trim().is_empty() {
            return Err(ApiError::invalid_input("content cannot be empty"));
        }
    }

    // First verify the artifact exists and belongs to this tenant
    let existing = db
        .get::<ArtifactResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::artifact_not_found(id))?;
    validate_tenant_ownership(&auth, Some(existing.tenant_id))?;

    let artifact = db
        .update::<ArtifactResponse>(id, &req, auth.tenant_id)
        .await?;
    ws.broadcast(WsEvent::ArtifactUpdated {
        artifact: artifact.clone(),
    });
    Ok(Json(artifact.linked()))
}

/// DELETE /api/v1/artifacts/{id} - Delete artifact
#[utoipa::path(
    delete,
    path = "/api/v1/artifacts/{id}",
    tag = "Artifacts",
    params(
        ("id" = Uuid, Path, description = "Artifact ID")
    ),
    responses(
        (status = 204, description = "Artifact deleted successfully"),
        (status = 404, description = "Artifact not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn delete_artifact(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<ArtifactId>,
) -> ApiResult<StatusCode> {
    // First verify the artifact exists and belongs to this tenant
    let artifact = db
        .get::<ArtifactResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::artifact_not_found(id))?;
    validate_tenant_ownership(&auth, Some(artifact.tenant_id))?;

    db.delete::<ArtifactResponse>(id, auth.tenant_id).await?;
    ws.broadcast(WsEvent::ArtifactDeleted {
        tenant_id: auth.tenant_id,
        id,
    });
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/artifacts/search - Search artifacts by similarity
#[utoipa::path(
    post,
    path = "/api/v1/artifacts/search",
    tag = "Artifacts",
    request_body = SearchRequest,
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn search_artifacts(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<SearchRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate search query
    if req.query.trim().is_empty() {
        return Err(ApiError::missing_field("query"));
    }

    // Validate entity types include Artifact
    if !req
        .entity_types
        .contains(&caliber_core::EntityType::Artifact)
    {
        return Err(ApiError::invalid_input(
            "entity_types must include Artifact for artifact search",
        ));
    }

    // Perform tenant-isolated search
    let response = db.search(&req, auth.tenant_id).await?;

    Ok(Json(response))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the artifact routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(create_artifact))
        .route("/", axum::routing::get(list_artifacts))
        .route("/:id", axum::routing::get(get_artifact))
        .route("/:id", axum::routing::patch(update_artifact))
        .route("/:id", axum::routing::delete(delete_artifact))
        .route("/search", axum::routing::post(search_artifacts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthContext, AuthMethod};
    use crate::db::{DbClient, DbConfig};
    use crate::routes::scope::create_scope;
    use crate::routes::trajectory::create_trajectory;
    use crate::state::ApiEventDag;
    use crate::types::{
        CreateScopeRequest, CreateTrajectoryRequest, ScopeResponse, TrajectoryResponse,
    };
    use crate::ws::WsState;
    use axum::{body::to_bytes, extract::Query, http::StatusCode, response::IntoResponse};
    use caliber_core::{ArtifactType, EntityIdType, ExtractionMethod, ScopeId, TrajectoryId, TTL};
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

        let tenant_id = db.tenant_create("test-artifact", None, None).await.ok()?;
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
    fn test_create_artifact_request_validation() {
        let req = CreateArtifactRequest {
            trajectory_id: TrajectoryId::nil(),
            scope_id: ScopeId::nil(),
            artifact_type: ArtifactType::Fact,
            name: "".to_string(),
            content: "".to_string(),
            source_turn: -1,
            extraction_method: ExtractionMethod::Explicit,
            confidence: Some(1.5),
            ttl: TTL::Persistent,
            metadata: None,
        };

        assert!(req.name.trim().is_empty());
        assert!(req.content.trim().is_empty());
        assert!(req.source_turn < 0);
        assert!(req.confidence.unwrap_or(0.0) > 1.0);
    }

    #[test]
    fn test_update_artifact_request_validation() {
        let req = UpdateArtifactRequest {
            name: None,
            content: None,
            artifact_type: None,
            ttl: None,
            metadata: None,
        };

        let has_updates = req.name.is_some()
            || req.content.is_some()
            || req.artifact_type.is_some()
            || req.ttl.is_some()
            || req.metadata.is_some();

        assert!(!has_updates);
    }

    #[test]
    fn test_list_artifacts_pagination() {
        let params = ListArtifactsRequest {
            artifact_type: Some(ArtifactType::Fact),
            trajectory_id: None,
            scope_id: Some(ScopeId::nil()),
            created_after: None,
            created_before: None,
            limit: Some(10),
            offset: Some(0),
        };

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(0));
    }

    #[test]
    fn test_search_request_validation() {
        let req = SearchRequest {
            query: "".to_string(),
            entity_types: vec![caliber_core::EntityType::Artifact],
            filters: vec![],
            limit: Some(10),
        };

        assert!(req.query.trim().is_empty());
        assert!(req
            .entity_types
            .contains(&caliber_core::EntityType::Artifact));
    }

    #[test]
    fn test_confidence_range_validation() {
        let valid_confidence = 0.85;
        let invalid_low = -0.1;
        let invalid_high = 1.5;

        assert!((0.0..=1.0).contains(&valid_confidence));
        assert!(!(0.0..=1.0).contains(&invalid_low));
        assert!(!(0.0..=1.0).contains(&invalid_high));
    }

    #[tokio::test]
    async fn test_create_and_list_artifacts_db_backed() {
        let Some(ctx) = db_test_context().await else {
            return;
        };

        let trajectory_req = CreateTrajectoryRequest {
            name: format!("artifact-traj-{}", Uuid::now_v7()),
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
            name: "artifact-scope".to_string(),
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

        let artifact_req = CreateArtifactRequest {
            trajectory_id: trajectory.trajectory_id,
            scope_id: scope.scope_id,
            artifact_type: ArtifactType::Fact,
            name: "artifact".to_string(),
            content: "content".to_string(),
            source_turn: 1,
            extraction_method: ExtractionMethod::Explicit,
            confidence: Some(0.9),
            ttl: TTL::Persistent,
            metadata: None,
        };

        let artifact_response = create_artifact(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            AuthExtractor(ctx.auth.clone()),
            Json(artifact_req),
        )
        .await
        .expect("create_artifact should succeed")
        .into_response();
        assert_eq!(artifact_response.status(), StatusCode::CREATED);
        let artifact: ArtifactResponse = response_json(artifact_response).await;

        let list_response = list_artifacts(
            State(ctx.db.clone()),
            AuthExtractor(ctx.auth.clone()),
            Query(ListArtifactsRequest {
                artifact_type: Some(ArtifactType::Fact),
                trajectory_id: Some(trajectory.trajectory_id),
                scope_id: Some(scope.scope_id),
                created_after: None,
                created_before: None,
                limit: None,
                offset: None,
            }),
        )
        .await
        .expect("list_artifacts should succeed")
        .into_response();
        assert_eq!(list_response.status(), StatusCode::OK);
        let list: ListArtifactsResponse = response_json(list_response).await;
        assert!(list
            .artifacts
            .iter()
            .any(|a| a.artifact_id == artifact.artifact_id));

        ctx.db
            .delete::<ArtifactResponse>(artifact.artifact_id, ctx.auth.tenant_id)
            .await
            .expect("delete artifact should succeed");
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
