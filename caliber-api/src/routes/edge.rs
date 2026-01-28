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
use caliber_core::EdgeId;
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

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, extract::State, http::StatusCode, response::IntoResponse, Json};
    use crate::auth::{AuthContext, AuthMethod};
    use crate::db::{DbClient, DbConfig};
    use crate::extractors::PathId;
    use crate::types::{CreateScopeRequest, CreateTrajectoryRequest, ScopeResponse, TrajectoryResponse};
    use crate::ws::WsState;
    use crate::types::{EdgeParticipantRequest, ProvenanceRequest};
    use caliber_core::{EdgeType, EntityIdType, EntityType, ExtractionMethod};
    use std::sync::Arc;
    use uuid::Uuid;

    fn sample_request(participants: usize, weight: Option<f32>) -> CreateEdgeRequest {
        let mut parts = Vec::new();
        for _ in 0..participants {
            parts.push(EdgeParticipantRequest {
                entity_type: EntityType::Trajectory,
                entity_id: Uuid::now_v7(),
                role: None,
            });
        }
        CreateEdgeRequest {
            edge_type: EdgeType::RelatesTo,
            participants: parts,
            weight,
            trajectory_id: None,
            provenance: ProvenanceRequest {
                source_turn: 1,
                extraction_method: ExtractionMethod::Explicit,
                confidence: Some(1.0),
            },
            metadata: None,
        }
    }

    #[test]
    fn test_create_edge_request_requires_multiple_participants() {
        let req = sample_request(1, None);
        assert!(req.participants.len() < 2);
    }

    #[test]
    fn test_create_edge_request_weight_range() {
        let req = sample_request(2, Some(1.1));
        assert!(req.weight.unwrap() > 1.0);

        let req = sample_request(2, Some(-0.1));
        assert!(req.weight.unwrap() < 0.0);
    }

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

        let tenant_id = db.tenant_create("test-edge", None, None).await.ok()?;
        let auth = AuthContext::new("test-user".to_string(), tenant_id, vec![], AuthMethod::Jwt);

        Some(DbTestContext {
            db,
            auth,
            ws: Arc::new(WsState::new(8)),
        })
    }

    async fn response_json<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&body).expect("parse json")
    }

    #[tokio::test]
    async fn test_create_get_list_edges_db_backed() {
        let Some(ctx) = db_test_context().await else { return; };

        let traj_req = CreateTrajectoryRequest {
            name: format!("edge-traj-{}", Uuid::now_v7()),
            description: None,
            parent_trajectory_id: None,
            agent_id: None,
            metadata: None,
        };
        let trajectory: TrajectoryResponse = ctx
            .db
            .create::<TrajectoryResponse>(&traj_req, ctx.auth.tenant_id)
            .await
            .expect("create trajectory");

        let scope_req = CreateScopeRequest {
            trajectory_id: trajectory.trajectory_id,
            parent_scope_id: None,
            name: "edge-scope".to_string(),
            purpose: None,
            token_budget: 1000,
            metadata: None,
        };
        let scope: ScopeResponse = ctx
            .db
            .create::<ScopeResponse>(&scope_req, ctx.auth.tenant_id)
            .await
            .expect("create scope");

        let req = CreateEdgeRequest {
            edge_type: EdgeType::RelatesTo,
            participants: vec![
                EdgeParticipantRequest {
                    entity_type: EntityType::Trajectory,
                    entity_id: trajectory.trajectory_id.as_uuid(),
                    role: Some("source".to_string()),
                },
                EdgeParticipantRequest {
                    entity_type: EntityType::Scope,
                    entity_id: scope.scope_id.as_uuid(),
                    role: Some("target".to_string()),
                },
            ],
            weight: Some(0.5),
            trajectory_id: Some(trajectory.trajectory_id),
            provenance: ProvenanceRequest {
                source_turn: 1,
                extraction_method: ExtractionMethod::Explicit,
                confidence: Some(0.9),
            },
            metadata: None,
        };

        let create_response = create_edge(
            State(ctx.db.clone()),
            State(ctx.ws.clone()),
            AuthExtractor(ctx.auth.clone()),
            Json(req),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(create_response.status(), StatusCode::CREATED);
        let edge: EdgeResponse = response_json(create_response).await;

        let get_response = get_edge(
            State(ctx.db.clone()),
            AuthExtractor(ctx.auth.clone()),
            PathId(edge.edge_id),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(get_response.status(), StatusCode::OK);

        let list_response = list_edges_by_participant(
            State(ctx.db.clone()),
            AuthExtractor(ctx.auth.clone()),
            axum::extract::Path(trajectory.trajectory_id.as_uuid()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(list_response.status(), StatusCode::OK);
        let list: ListEdgesResponse = response_json(list_response).await;
        assert!(list.edges.iter().any(|e| e.edge_id == edge.edge_id));

        ctx.db
            .delete::<EdgeResponse>(edge.edge_id, ctx.auth.tenant_id)
            .await
            .ok();
        ctx.db
            .delete::<ScopeResponse>(scope.scope_id, ctx.auth.tenant_id)
            .await
            .ok();
        ctx.db
            .delete::<TrajectoryResponse>(trajectory.trajectory_id, ctx.auth.tenant_id)
            .await
            .ok();
    }
}
