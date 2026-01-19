//! DSL REST API Routes
//!
//! This module implements Axum route handlers for DSL validation and parsing operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    state::AppState,
    types::{ParseErrorResponse, ValidateDslRequest, ValidateDslResponse},
};

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
    State(db): State<DbClient>,
    Json(req): Json<ValidateDslRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::debug!(db_pool_size = db.pool_size(), "DSL validation request");

    // Validate that source is not empty
    if req.source.trim().is_empty() {
        return Err(ApiError::missing_field("source"));
    }

    match caliber_dsl::parse(&req.source) {
        Ok(ast) => {
            let ast_json = serde_json::to_value(&ast)
                .map_err(|e| ApiError::internal_error(format!("Failed to serialize AST: {}", e)))?;

            let response = ValidateDslResponse {
                valid: true,
                errors: Vec::new(),
                ast: Some(ast_json),
            };

            Ok(Json(response))
        }
        Err(err) => {
            let response = ValidateDslResponse {
                valid: false,
                errors: vec![ParseErrorResponse {
                    line: err.line,
                    column: err.column,
                    message: err.message,
                }],
                ast: None,
            };

            Ok(Json(response))
        }
    }
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
    State(db): State<DbClient>,
    Json(req): Json<ValidateDslRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::debug!(db_pool_size = db.pool_size(), "DSL parse request");

    // Validate that source is not empty
    if req.source.trim().is_empty() {
        return Err(ApiError::missing_field("source"));
    }

    match caliber_dsl::parse(&req.source) {
        Ok(ast) => {
            let ast_json = serde_json::to_value(&ast)
                .map_err(|e| ApiError::internal_error(format!("Failed to serialize AST: {}", e)))?;

            let response = ValidateDslResponse {
                valid: true,
                errors: Vec::new(),
                ast: Some(ast_json),
            };

            Ok(Json(response))
        }
        Err(err) => {
            let response = ValidateDslResponse {
                valid: false,
                errors: vec![ParseErrorResponse {
                    line: err.line,
                    column: err.column,
                    message: err.message,
                }],
                ast: None,
            };

            Ok(Json(response))
        }
    }
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the DSL routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/validate", axum::routing::post(validate_dsl))
        .route("/parse", axum::routing::post(parse_dsl))
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
