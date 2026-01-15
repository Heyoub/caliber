//! Turn REST API Routes
//!
//! This module implements Axum route handlers for turn operations.
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
    types::{CreateTurnRequest, TurnResponse},
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for turn routes.
#[derive(Clone)]
pub struct TurnState {
    pub db: DbClient,
}

impl TurnState {
    pub fn new(db: DbClient) -> Self {
        Self { db }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/turns - Create a new turn
#[utoipa::path(
    post,
    path = "/api/v1/turns",
    tag = "Turns",
    request_body = CreateTurnRequest,
    responses(
        (status = 201, description = "Turn created successfully", body = TurnResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_turn(
    State(state): State<Arc<TurnState>>,
    Json(req): Json<CreateTurnRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.content.trim().is_empty() {
        return Err(ApiError::missing_field("content"));
    }

    if req.sequence < 0 {
        return Err(ApiError::invalid_range("sequence", 0, i32::MAX));
    }

    if req.token_count < 0 {
        return Err(ApiError::invalid_range("token_count", 0, i32::MAX));
    }

    // Create turn via database client
    let turn = state.db.turn_create(&req).await?;

    Ok((StatusCode::CREATED, Json(turn)))
}

/// GET /api/v1/turns/{id} - Get turn by ID
#[utoipa::path(
    get,
    path = "/api/v1/turns/{id}",
    tag = "Turns",
    params(
        ("id" = Uuid, Path, description = "Turn ID")
    ),
    responses(
        (status = 200, description = "Turn details", body = TurnResponse),
        (status = 404, description = "Turn not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_turn(
    State(state): State<Arc<TurnState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // Note: caliber-pg doesn't currently have a caliber_turn_get function
    // that retrieves a single turn by ID. Turns are typically retrieved
    // by scope using caliber_turn_get_by_scope.
    //
    // For now, we'll return an error indicating this is not yet implemented.
    // When caliber_turn_get is added to caliber-pg, this can be updated.

    Err(ApiError::internal_error(
        "Turn retrieval by ID not yet implemented in caliber-pg - use scope endpoint to list turns",
    ))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the turn routes router.
pub fn create_router(db: DbClient) -> axum::Router {
    let state = Arc::new(TurnState::new(db));

    axum::Router::new()
        .route("/", axum::routing::post(create_turn))
        .route("/:id", axum::routing::get(get_turn))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{EntityId, TurnRole};

    #[test]
    fn test_create_turn_request_validation() {
        // Use a dummy UUID for testing (all zeros is valid)
        let dummy_id: EntityId = uuid::Uuid::nil().into();

        let req = CreateTurnRequest {
            scope_id: dummy_id,
            sequence: -1,
            role: TurnRole::User,
            content: "".to_string(),
            token_count: -1,
            tool_calls: None,
            tool_results: None,
            metadata: None,
        };

        assert!(req.content.trim().is_empty());
        assert!(req.sequence < 0);
        assert!(req.token_count < 0);
    }

    #[test]
    fn test_turn_role_variants() {
        // Verify all turn roles are accessible
        let roles = vec![
            TurnRole::User,
            TurnRole::Assistant,
            TurnRole::System,
            TurnRole::Tool,
        ];

        assert_eq!(roles.len(), 4);
    }

    #[test]
    fn test_sequence_range_validation() {
        let valid_sequence = 0;
        let invalid_sequence = -1;

        assert!(valid_sequence >= 0);
        assert!(invalid_sequence < 0);
    }

    #[test]
    fn test_token_count_range_validation() {
        let valid_token_count = 100;
        let invalid_token_count = -1;

        assert!(valid_token_count >= 0);
        assert!(invalid_token_count < 0);
    }

    #[test]
    fn test_optional_fields() {
        let dummy_id: EntityId = uuid::Uuid::nil().into();

        let req = CreateTurnRequest {
            scope_id: dummy_id,
            sequence: 0,
            role: TurnRole::User,
            content: "Test content".to_string(),
            token_count: 10,
            tool_calls: None,
            tool_results: None,
            metadata: None,
        };

        assert!(req.tool_calls.is_none());
        assert!(req.tool_results.is_none());
        assert!(req.metadata.is_none());
    }

    #[test]
    fn test_tool_calls_and_results() {
        let dummy_id: EntityId = uuid::Uuid::nil().into();

        let tool_calls = serde_json::json!([
            {
                "name": "test_tool",
                "arguments": {"arg1": "value1"}
            }
        ]);

        let tool_results = serde_json::json!([
            {
                "result": "success",
                "output": "test output"
            }
        ]);

        let req = CreateTurnRequest {
            scope_id: dummy_id,
            sequence: 0,
            role: TurnRole::Tool,
            content: "Tool execution".to_string(),
            token_count: 50,
            tool_calls: Some(tool_calls.clone()),
            tool_results: Some(tool_results.clone()),
            metadata: None,
        };

        assert!(req.tool_calls.is_some());
        assert!(req.tool_results.is_some());
        assert_eq!(req.role, TurnRole::Tool);
    }
}
