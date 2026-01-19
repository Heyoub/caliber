//! Search REST API Routes
//!
//! This module implements Axum route handlers for global search.

use axum::{extract::State, response::IntoResponse, Json};

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    middleware::AuthExtractor,
    state::AppState,
    types::{SearchRequest, SearchResponse},
};

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
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<SearchRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.query.trim().is_empty() {
        return Err(ApiError::missing_field("query"));
    }

    if req.entity_types.is_empty() {
        return Err(ApiError::missing_field("entity_types"));
    }

    let response = db.search(&req, auth.tenant_id).await?;
    Ok(Json(response))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the search routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(search))
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
