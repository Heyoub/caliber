//! Search REST API Routes
//!
//! This module implements Axum route handlers for global search.

use axum::{extract::State, response::IntoResponse, Json};
use std::sync::Arc;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    middleware::AuthExtractor,
    types::{SearchRequest, SearchResponse},
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for search routes.
#[derive(Clone)]
pub struct SearchState {
    pub db: DbClient,
}

impl SearchState {
    pub fn new(db: DbClient) -> Self {
        Self { db }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/search - Global search across entities
#[utoipa::path(
    post,
    path = "/api/v1/search",
    tag = "Search",
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
pub async fn search(
    State(state): State<Arc<SearchState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<SearchRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.query.trim().is_empty() {
        return Err(ApiError::missing_field("query"));
    }

    if req.entity_types.is_empty() {
        return Err(ApiError::missing_field("entity_types"));
    }

    let response = state.db.search(&req, auth.tenant_id).await?;
    Ok(Json(response))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the search routes router.
pub fn create_router(db: DbClient) -> axum::Router {
    let state = Arc::new(SearchState::new(db));
    axum::Router::new()
        .route("/", axum::routing::post(search))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_request_validation() {
        let empty_query = SearchRequest {
            query: "".to_string(),
            entity_types: vec![caliber_core::EntityType::Artifact],
            filters: vec![],
            limit: Some(10),
        };

        assert!(empty_query.query.trim().is_empty());
        assert!(!empty_query.entity_types.is_empty());

        let empty_types = SearchRequest {
            query: "query".to_string(),
            entity_types: vec![],
            filters: vec![],
            limit: Some(10),
        };

        assert!(!empty_types.query.trim().is_empty());
        assert!(empty_types.entity_types.is_empty());
    }
}
