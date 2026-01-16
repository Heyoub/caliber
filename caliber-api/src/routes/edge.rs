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
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    types::{CreateEdgeRequest, EdgeResponse, ListEdgesResponse},
    ws::WsState,
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for edge routes.
#[derive(Clone)]
pub struct EdgeState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
}

impl EdgeState {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

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
    State(state): State<Arc<EdgeState>>,
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

    // Create edge via database client
    let edge = state.db.edge_create(&req).await?;

    // Broadcast EdgeCreated event
    state.ws.broadcast(WsEvent::EdgeCreated {
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
    State(state): State<Arc<EdgeState>>,
    Json(requests): Json<Vec<CreateEdgeRequest>>,
) -> ApiResult<impl IntoResponse> {
    let mut edges = Vec::new();

    for req in requests {
        // Skip invalid edges
        if req.participants.len() < 2 {
            continue;
        }

        match state.db.edge_create(&req).await {
            Ok(edge) => edges.push(edge),
            Err(_) => continue, // Skip failed creations
        }
    }

    // Broadcast batch event
    state.ws.broadcast(WsEvent::EdgesBatchCreated { count: edges.len() });

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
    State(state): State<Arc<EdgeState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let edge = state.db.edge_get(id.into()).await?;

    match edge {
        Some(e) => Ok(Json(e)),
        None => Err(ApiError::not_found("Edge", id)),
    }
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
    State(state): State<Arc<EdgeState>>,
    Path(entity_id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let edges = state.db.edge_list_by_participant(entity_id.into()).await?;

    Ok(Json(ListEdgesResponse { edges }))
}

// ============================================================================
// ROUTER FACTORY
// ============================================================================

/// Create the edge router.
pub fn create_router(db: DbClient, ws: Arc<WsState>) -> axum::Router {
    let state = Arc::new(EdgeState::new(db, ws));

    axum::Router::new()
        .route("/", axum::routing::post(create_edge))
        .route("/batch", axum::routing::post(create_edges_batch))
        .route("/{id}", axum::routing::get(get_edge))
        .route(
            "/by-participant/{entity_id}",
            axum::routing::get(list_edges_by_participant),
        )
        .with_state(state)
}
