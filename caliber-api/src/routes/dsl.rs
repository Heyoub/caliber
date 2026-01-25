//! DSL REST API Routes
//!
//! This module implements Axum route handlers for DSL validation, parsing,
//! compilation, and deployment operations.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    middleware::AuthExtractor,
    db::DbClient,
    error::{ApiError, ApiResult},
    state::AppState,
    types::{
        CompileDslRequest, CompileDslResponse, CompileErrorResponse,
        DeployDslRequest, DeployDslResponse, DslConfigStatus,
        ParseErrorResponse, ValidateDslRequest, ValidateDslResponse,
    },
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

/// POST /api/v1/dsl/compile - Compile DSL source to runtime configuration
#[utoipa::path(
    post,
    path = "/api/v1/dsl/compile",
    tag = "DSL",
    request_body = CompileDslRequest,
    responses(
        (status = 200, description = "Compilation result", body = CompileDslResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn compile_dsl(
    State(_db): State<DbClient>,
    Json(req): Json<CompileDslRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::debug!("DSL compile request");

    // Validate that source is not empty
    if req.source.trim().is_empty() {
        return Err(ApiError::missing_field("source"));
    }

    // Step 1: Parse the DSL
    let ast = match caliber_dsl::parse(&req.source) {
        Ok(ast) => ast,
        Err(err) => {
            let response = CompileDslResponse {
                success: false,
                errors: vec![CompileErrorResponse {
                    error_type: "ParseError".to_string(),
                    message: format!("Line {}, Column {}: {}", err.line, err.column, err.message),
                }],
                compiled: None,
            };
            return Ok(Json(response));
        }
    };

    // Step 2: Compile the AST
    match caliber_dsl::DslCompiler::compile(&ast) {
        Ok(config) => {
            let compiled_json = serde_json::to_value(&config)
                .map_err(|e| ApiError::internal_error(format!("Failed to serialize compiled config: {}", e)))?;

            let response = CompileDslResponse {
                success: true,
                errors: Vec::new(),
                compiled: Some(compiled_json),
            };

            Ok(Json(response))
        }
        Err(err) => {
            let response = CompileDslResponse {
                success: false,
                errors: vec![CompileErrorResponse {
                    error_type: compile_error_type(&err),
                    message: err.to_string(),
                }],
                compiled: None,
            };

            Ok(Json(response))
        }
    }
}

/// Extract the error type name from a CompileError
fn compile_error_type(err: &caliber_dsl::CompileError) -> String {
    match err {
        caliber_dsl::CompileError::UndefinedReference { .. } => "UndefinedReference".to_string(),
        caliber_dsl::CompileError::DuplicateDefinition { .. } => "DuplicateDefinition".to_string(),
        caliber_dsl::CompileError::InvalidValue { .. } => "InvalidValue".to_string(),
        caliber_dsl::CompileError::MissingField { .. } => "MissingField".to_string(),
        caliber_dsl::CompileError::CircularDependency { .. } => "CircularDependency".to_string(),
        caliber_dsl::CompileError::TypeMismatch { .. } => "TypeMismatch".to_string(),
        caliber_dsl::CompileError::InvalidDuration { .. } => "InvalidDuration".to_string(),
        caliber_dsl::CompileError::SemanticError { .. } => "SemanticError".to_string(),
    }
}

/// POST /api/v1/dsl/deploy - Deploy a DSL configuration
///
/// This endpoint parses, compiles, saves, and optionally activates a DSL configuration.
#[utoipa::path(
    post,
    path = "/api/v1/dsl/deploy",
    tag = "DSL",
    request_body = DeployDslRequest,
    responses(
        (status = 201, description = "Configuration deployed", body = DeployDslResponse),
        (status = 400, description = "Invalid DSL or compilation error", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn deploy_dsl(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<DeployDslRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::info!(
        tenant_id = %auth.tenant_id,
        config_name = %req.name,
        activate = req.activate,
        "DSL deploy request"
    );

    // Validate that source is not empty
    if req.source.trim().is_empty() {
        return Err(ApiError::missing_field("source"));
    }

    // Step 1: Parse the DSL
    let ast = caliber_dsl::parse(&req.source)
        .map_err(|err| ApiError::invalid_input(format!(
            "Parse error at line {}, column {}: {}", 
            err.line, err.column, err.message
        )))?;

    // Step 2: Compile the AST
    let compiled = caliber_dsl::DslCompiler::compile(&ast)
        .map_err(|err| ApiError::invalid_input(format!("Compilation error: {}", err)))?;

    // Step 3: Serialize for storage
    let ast_json = serde_json::to_value(&ast)
        .map_err(|e| ApiError::internal_error(format!("Failed to serialize AST: {}", e)))?;
    let compiled_json = serde_json::to_value(&compiled)
        .map_err(|e| ApiError::internal_error(format!("Failed to serialize compiled config: {}", e)))?;

    // Step 4: Get next version number for this config name
    let version = db.dsl_config_next_version(auth.tenant_id, &req.name).await?;

    // Step 5: Insert the config
    let config_id = db.dsl_config_create(
        auth.tenant_id,
        &req.name,
        version,
        &req.source,
        ast_json,
        compiled_json,
    ).await?;

    // Step 6: If activate is true, deploy it
    // Note: We pass None for agent_id since deployment is user-initiated
    let status = if req.activate {
        db.dsl_config_deploy(config_id, None, req.notes.as_deref()).await?;
        DslConfigStatus::Deployed
    } else {
        DslConfigStatus::Draft
    };

    let message = if req.activate {
        format!("Configuration '{}' v{} deployed successfully", req.name, version)
    } else {
        format!("Configuration '{}' v{} saved as draft", req.name, version)
    };

    let response = DeployDslResponse {
        config_id,
        name: req.name,
        version,
        status,
        message,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the DSL routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/validate", axum::routing::post(validate_dsl))
        .route("/parse", axum::routing::post(parse_dsl))
        .route("/compile", axum::routing::post(compile_dsl))
        .route("/deploy", axum::routing::post(deploy_dsl))
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
