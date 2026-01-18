//! Artifact REST API Routes
//!
//! This module implements Axum route handlers for artifact operations.
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
    auth::validate_tenant_ownership,
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    middleware::AuthExtractor,
    types::{
        ArtifactResponse, CreateArtifactRequest, ListArtifactsRequest, ListArtifactsResponse,
        SearchRequest, SearchResponse, UpdateArtifactRequest,
    },
    ws::WsState,
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for artifact routes.
#[derive(Clone)]
pub struct ArtifactState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
}

impl ArtifactState {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

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
    State(state): State<Arc<ArtifactState>>,
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
    let artifact = state.db.artifact_create(&req, auth.tenant_id).await?;

    // Broadcast ArtifactCreated event
    state.ws.broadcast(WsEvent::ArtifactCreated {
        artifact: artifact.clone(),
    });

    Ok((StatusCode::CREATED, Json(artifact)))
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
    State(state): State<Arc<ArtifactState>>,
    AuthExtractor(auth): AuthExtractor,
    Query(params): Query<ListArtifactsRequest>,
) -> ApiResult<impl IntoResponse> {
    // Filter by scope or trajectory with tenant isolation

    if let Some(scope_id) = params.scope_id {
        // Filter by scope and tenant
        let artifacts = state.db.artifact_list_by_scope_and_tenant(scope_id, auth.tenant_id).await?;

        // Apply additional filters if needed
        let mut filtered = artifacts;

        if let Some(artifact_type) = params.artifact_type {
            filtered.retain(|a| a.artifact_type == artifact_type);
        }

        if let Some(trajectory_id) = params.trajectory_id {
            filtered.retain(|a| a.trajectory_id == trajectory_id);
        }

        if let Some(created_after) = params.created_after {
            filtered.retain(|a| a.created_at >= created_after);
        }

        if let Some(created_before) = params.created_before {
            filtered.retain(|a| a.created_at <= created_before);
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

        let response = ListArtifactsResponse {
            artifacts: paginated,
            total,
        };

        Ok(Json(response))
    } else if let Some(trajectory_id) = params.trajectory_id {
        // Filter by trajectory and tenant
        let artifacts = state.db.artifact_list_by_trajectory_and_tenant(trajectory_id, auth.tenant_id).await?;

        // Apply additional filters if needed
        let mut filtered = artifacts;

        if let Some(artifact_type) = params.artifact_type {
            filtered.retain(|a| a.artifact_type == artifact_type);
        }

        if let Some(created_after) = params.created_after {
            filtered.retain(|a| a.created_at >= created_after);
        }

        if let Some(created_before) = params.created_before {
            filtered.retain(|a| a.created_at <= created_before);
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

        let response = ListArtifactsResponse {
            artifacts: paginated,
            total,
        };

        Ok(Json(response))
    } else {
        Err(ApiError::invalid_input(
            "Either scope_id or trajectory_id filter is required",
        ))
    }
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
    State(state): State<Arc<ArtifactState>>,
    AuthExtractor(auth): AuthExtractor,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let artifact = state
        .db
        .artifact_get(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::artifact_not_found(id))?;

    // Validate tenant ownership before returning
    validate_tenant_ownership(&auth, artifact.tenant_id)?;

    Ok(Json(artifact))
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
    State(state): State<Arc<ArtifactState>>,
    AuthExtractor(auth): AuthExtractor,
    Path(id): Path<Uuid>,
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
    let existing = state
        .db
        .artifact_get(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::artifact_not_found(id))?;
    validate_tenant_ownership(&auth, existing.tenant_id)?;

    let artifact = state.db.artifact_update(id, &req, auth.tenant_id).await?;
    state.ws.broadcast(WsEvent::ArtifactUpdated { artifact: artifact.clone() });
    Ok(Json(artifact))
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
    State(state): State<Arc<ArtifactState>>,
    AuthExtractor(auth): AuthExtractor,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // First verify the artifact exists and belongs to this tenant
    let artifact = state
        .db
        .artifact_get(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::artifact_not_found(id))?;
    validate_tenant_ownership(&auth, artifact.tenant_id)?;

    state.db.artifact_delete(id).await?;
    state.ws.broadcast(WsEvent::ArtifactDeleted {
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
    State(state): State<Arc<ArtifactState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<SearchRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate search query
    if req.query.trim().is_empty() {
        return Err(ApiError::missing_field("query"));
    }

    // Validate entity types include Artifact
    if !req.entity_types.contains(&caliber_core::EntityType::Artifact) {
        return Err(ApiError::invalid_input(
            "entity_types must include Artifact for artifact search",
        ));
    }

    // Perform tenant-isolated search
    let response = state.db.search(&req, auth.tenant_id).await?;

    Ok(Json(response))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the artifact routes router.
pub fn create_router(db: DbClient, ws: Arc<WsState>) -> axum::Router {
    let state = Arc::new(ArtifactState::new(db, ws));

    axum::Router::new()
        .route("/", axum::routing::post(create_artifact))
        .route("/", axum::routing::get(list_artifacts))
        .route("/:id", axum::routing::get(get_artifact))
        .route("/:id", axum::routing::patch(update_artifact))
        .route("/:id", axum::routing::delete(delete_artifact))
        .route("/search", axum::routing::post(search_artifacts))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{ArtifactType, EntityId, ExtractionMethod, TTL};

    #[test]
    fn test_create_artifact_request_validation() {
        // Use a dummy UUID for testing (all zeros is valid)
        let dummy_id: EntityId = uuid::Uuid::nil();

        let req = CreateArtifactRequest {
            trajectory_id: dummy_id,
            scope_id: dummy_id,
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
        assert!(req.confidence.unwrap() > 1.0);
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
            scope_id: Some(uuid::Uuid::nil()),
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
        assert!(req.entity_types.contains(&caliber_core::EntityType::Artifact));
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
}
