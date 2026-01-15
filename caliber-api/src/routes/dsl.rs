//! DSL REST API Routes
//!
//! This module implements Axum route handlers for DSL validation and parsing operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    types::{ParseErrorResponse, ValidateDslRequest, ValidateDslResponse},
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for DSL routes.
#[derive(Clone)]
pub struct DslState {
    pub db: DbClient,
}

impl DslState {
    pub fn new(db: DbClient) -> Self {
        Self { db }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/dsl/validate - Validate DSL source
#[utoipa::path(
    post,
    path = "/api/v1/dsl/validate",
    tag = "DSL",
    request_body = ValidateDslRequest,
    responses(
        (status = 200, description = "Validation result", body = ValidateDslResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn validate_dsl(
    State(state): State<Arc<DslState>>,
    Json(req): Json<ValidateDslRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that source is not empty
    if req.source.trim().is_empty() {
        return Err(ApiError::missing_field("source"));
    }

    // TODO: Implement caliber_dsl_validate in caliber-pg
    // This will:
    // 1. Tokenize the DSL source using the lexer
    // 2. Parse tokens into an AST
    // 3. Validate semantic correctness
    // 4. Return errors with line/column information

    // For now, return a basic validation response
    // In a real implementation, this would call the DSL parser
    let response = ValidateDslResponse {
        valid: false,
        errors: vec![ParseErrorResponse {
            line: 1,
            column: 1,
            message: "DSL validation not yet implemented in caliber-pg".to_string(),
        }],
        ast: None,
    };

    Ok(Json(response))
}

/// POST /api/v1/dsl/parse - Parse DSL source
#[utoipa::path(
    post,
    path = "/api/v1/dsl/parse",
    tag = "DSL",
    request_body = ValidateDslRequest,
    responses(
        (status = 200, description = "Parse result with AST", body = ValidateDslResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn parse_dsl(
    State(state): State<Arc<DslState>>,
    Json(req): Json<ValidateDslRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that source is not empty
    if req.source.trim().is_empty() {
        return Err(ApiError::missing_field("source"));
    }

    // TODO: Implement caliber_dsl_parse in caliber-pg
    // This will:
    // 1. Tokenize the DSL source
    // 2. Parse tokens into an AST
    // 3. Return the AST as JSON (even if there are warnings)
    // 4. Return errors separately

    // For now, return a basic parse response
    let response = ValidateDslResponse {
        valid: false,
        errors: vec![ParseErrorResponse {
            line: 1,
            column: 1,
            message: "DSL parsing not yet implemented in caliber-pg".to_string(),
        }],
        ast: None,
    };

    Ok(Json(response))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the DSL routes router.
pub fn create_router(db: DbClient) -> axum::Router {
    let state = Arc::new(DslState::new(db));

    axum::Router::new()
        .route("/validate", axum::routing::post(validate_dsl))
        .route("/parse", axum::routing::post(parse_dsl))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_dsl_request_validation() {
        let req = ValidateDslRequest {
            source: "".to_string(),
        };

        assert!(req.source.trim().is_empty());
    }

    #[test]
    fn test_parse_error_response_structure() {
        let error = ParseErrorResponse {
            line: 42,
            column: 10,
            message: "Unexpected token".to_string(),
        };

        assert_eq!(error.line, 42);
        assert_eq!(error.column, 10);
        assert_eq!(error.message, "Unexpected token");
    }

    #[test]
    fn test_validate_dsl_response_structure() {
        let response = ValidateDslResponse {
            valid: true,
            errors: vec![],
            ast: Some(serde_json::json!({
                "version": "1.0",
                "definitions": []
            })),
        };

        assert!(response.valid);
        assert!(response.errors.is_empty());
        assert!(response.ast.is_some());
    }

    #[test]
    fn test_validate_dsl_response_with_errors() {
        let response = ValidateDslResponse {
            valid: false,
            errors: vec![
                ParseErrorResponse {
                    line: 1,
                    column: 1,
                    message: "Expected 'caliber' keyword".to_string(),
                },
                ParseErrorResponse {
                    line: 5,
                    column: 10,
                    message: "Unterminated string".to_string(),
                },
            ],
            ast: None,
        };

        assert!(!response.valid);
        assert_eq!(response.errors.len(), 2);
        assert!(response.ast.is_none());
    }
}
