//! Edge REST API Routes (Battle Intel Feature 1)
//!
//! This module implements Axum route handlers for edge operations.
//! Edges represent graph relationships between entities (binary or hyperedge).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use caliber_core::{EdgeId, EntityIdType};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::validate_tenant_ownership,
    components::EdgeListFilter,
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    extractors::PathId,
    middleware::AuthExtractor,
    state::AppState,
    types::{CreateEdgeRequest, EdgeResponse, ListEdgesResponse},
    ws::WsState,
};

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/edges - Create a new edge
#[utoipa::path(
    post,
    path = "/api/v1/edges",
    tag = "Edges",
    request_body = CreateEdgeRequest,
    responses(
        (status = 201, description = "Edge created successfully", body = EdgeResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_edge(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<CreateEdgeRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate participants (must have at least 2)
    if req.participants.len() < 2 {
        return Err(ApiError::invalid_input(
            "Edge must have at least 2 participants",
        ));
    }

    // Validate weight if provided
    if let Some(weight) = req.weight {
        if !(0.0..=1.0).contains(&weight) {
            return Err(ApiError::invalid_input("Weight must be between 0.0 and 1.0"));
        }
    }

    // Create edge via database client with tenant_id for isolation
    let edge = db.create::<EdgeResponse>(&req, auth.tenant_id).await?;

    // Broadcast EdgeCreated event
    ws.broadcast(WsEvent::EdgeCreated {
        tenant_id: auth.tenant_id,
        edge_id: edge.edge_id,
        edge_type: edge.edge_type,
    });

    Ok((StatusCode::CREATED, Json(edge)))
}

/// POST /api/v1/edges/batch - Create multiple edges
#[utoipa::path(
    post,
    path = "/api/v1/edges/batch",
    tag = "Edges",
    request_body = Vec<CreateEdgeRequest>,
    responses(
        (status = 201, description = "Edges created successfully", body = ListEdgesResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_edges_batch(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(requests): Json<Vec<CreateEdgeRequest>>,
) -> ApiResult<impl IntoResponse> {
    let mut edges = Vec::new();

    for req in requests {
        // Skip invalid edges
        if req.participants.len() < 2 {
            continue;
        }

        match db.create::<EdgeResponse>(&req, auth.tenant_id).await {
            Ok(edge) => edges.push(edge),
            Err(_) => continue, // Skip failed creations
        }
    }

    // Broadcast batch event
    ws.broadcast(WsEvent::EdgesBatchCreated { tenant_id: auth.tenant_id, count: edges.len() });

    Ok((StatusCode::CREATED, Json(ListEdgesResponse { edges })))
}

/// GET /api/v1/edges/{id} - Get an edge by ID
#[utoipa::path(
    get,
    path = "/api/v1/edges/{id}",
    tag = "Edges",
    params(
        ("id" = String, Path, description = "Edge ID")
    ),
    responses(
        (status = 200, description = "Edge found", body = EdgeResponse),
        (status = 404, description = "Edge not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_edge(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<EdgeId>,
) -> ApiResult<impl IntoResponse> {
    // Edge doesn't enforce tenant filtering, so validate ownership after get
    let edge = db
        .get::<EdgeResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Edge", id))?;

    // Validate tenant ownership before returning
    validate_tenant_ownership(&auth, edge.tenant_id)?;

    Ok(Json(edge))
}

/// GET /api/v1/edges/by-participant/{entity_id} - Get edges by participant entity
#[utoipa::path(
    get,
    path = "/api/v1/edges/by-participant/{entity_id}",
    tag = "Edges",
    params(
        ("entity_id" = String, Path, description = "Entity ID to find edges for")
    ),
    responses(
        (status = 200, description = "Edges found", body = ListEdgesResponse),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_edges_by_participant(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Path(entity_id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // List edges filtered by participant and tenant for isolation
    let filter = EdgeListFilter {
        participant_id: Some(entity_id),
        tenant_id: Some(auth.tenant_id),
        ..Default::default()
    };
    let edges = db.list::<EdgeResponse>(&filter, auth.tenant_id).await?;

    Ok(Json(ListEdgesResponse { edges }))
}

// ============================================================================
// ROUTER FACTORY
// ============================================================================

/// Create the edge router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(create_edge))
        .route("/batch", axum::routing::post(create_edges_batch))
        .route("/{id}", axum::routing::get(get_edge))
        .route(
            "/by-participant/{entity_id}",
            axum::routing::get(list_edges_by_participant),
        )
}
