//! DSL REST API Routes
//!
//! This module implements Axum route handlers for DSL validation, parsing,
//! compilation, and deployment operations.

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    middleware::AuthExtractor,
    state::AppState,
    types::{
        CompileDslRequest, CompileDslResponse, CompileErrorResponse, ComposePackMultipart,
        ComposePackResponse, DeployDslRequest, DeployDslResponse, DslConfigStatus, PackDiagnostic,
        ParseErrorResponse, ValidateDslRequest, ValidateDslResponse,
    },
};
use caliber_dsl::pack::{compose_pack as compose_pack_internal, PackInput, PackMarkdownFile};
use caliber_dsl::pretty_printer::pretty_print;
use std::path::PathBuf;

// Use shared parse_markdown_source from utils
use crate::utils::parse_markdown_source;

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// Validate a DSL Markdown source and return either its parsed AST or parse diagnostics.
///
/// Validates that the provided request source is not empty; if empty, the request is rejected with a missing-field error. On successful parsing the response contains `valid: true` and `ast` set to the serialized AST. On parse failure the response contains `valid: false` and a single `ParseErrorResponse` with `line: 0`, `column: 0`, and `message` set to the parser error string.
///
/// # Examples
///
/// ```no_run
/// use axum::Json;
/// use crate::api::ValidateDslRequest;
///
/// // Construct a request with DSL source and send it to the handler.
/// let req = ValidateDslRequest { source: "# Example DSL".into() };
/// // Calling the handler requires a `State<DbClient>`; in integration tests provide a test DbClient.
/// // let response = tokio::runtime::Runtime::new().unwrap().block_on(validate_dsl(state, Json(req)));
/// ```
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

    match parse_markdown_source(&req.source) {
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
        Err(err_msg) => {
            let response = ValidateDslResponse {
                valid: false,
                errors: vec![ParseErrorResponse {
                    line: 0,
                    column: 0,
                    message: err_msg,
                }],
                ast: None,
            };

            Ok(Json(response))
        }
    }
}

/// Parses DSL Markdown source and returns a validation response containing the AST or parse errors.
///
/// On success the response's `ast` field contains the parsed Abstract Syntax Tree (AST) serialized as JSON.
/// On failure the response includes one or more `ParseErrorResponse` entries with error messages (line and column are set to 0 for parse errors produced by the internal pack parser).
///
/// # Examples
///
/// ```
/// // Prefer using the HTTP endpoint in integration tests; for local parsing the helper can be used:
/// let source = "# My DSL\n...";
/// let ast = parse_markdown_source(source).expect("should parse");
/// let ast_json = serde_json::to_value(&ast).unwrap();
/// assert!(ast_json.is_object());
/// ```
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

    match parse_markdown_source(&req.source) {
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
        Err(err_msg) => {
            let response = ValidateDslResponse {
                valid: false,
                errors: vec![ParseErrorResponse {
                    line: 0,
                    column: 0,
                    message: err_msg,
                }],
                ast: None,
            };

            Ok(Json(response))
        }
    }
}

/// Compile a Markdown DSL source into a runtime configuration.
///
/// Parses the provided Markdown DSL into an AST and attempts to compile that AST into a runtime
/// configuration. On success the handler returns a JSON representation of the compiled config;
/// on failure it returns structured compilation or parse errors.
///
/// # Examples
///
/// ```
/// // Demonstrates the core operations performed by the handler:
/// // parse the Markdown DSL into an AST and compile it into a runtime config.
/// let source = "# config\n\n; a minimal DSL sample";
/// let ast = parse_markdown_source(source).expect("failed to parse DSL");
/// let config = caliber_dsl::DslCompiler::compile(&ast).expect("failed to compile DSL");
/// let compiled_json = serde_json::to_value(&config).expect("failed to serialize compiled config");
/// assert!(compiled_json.is_object());
/// ```
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

    // Step 1: Parse the Markdown DSL
    let ast = match parse_markdown_source(&req.source) {
        Ok(ast) => ast,
        Err(err_msg) => {
            let response = CompileDslResponse {
                success: false,
                errors: vec![CompileErrorResponse {
                    error_type: "ParseError".to_string(),
                    message: err_msg,
                }],
                compiled: None,
            };
            return Ok(Json(response));
        }
    };

    // Step 2: Compile the AST
    match caliber_dsl::DslCompiler::compile(&ast) {
        Ok(config) => {
            let compiled_json = serde_json::to_value(&config).map_err(|e| {
                ApiError::internal_error(format!("Failed to serialize compiled config: {}", e))
            })?;

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

/// POST /api/v1/dsl/compose - Compose pack (TOML + Markdown) to AST + compiled config
#[utoipa::path(
    post,
    path = "/api/v1/dsl/compose",
    tag = "DSL",
    request_body(content = ComposePackMultipart, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Composition result", body = ComposePackResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn compose_pack(
    State(_db): State<DbClient>,
    mut multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let mut manifest: Option<String> = None;
    let mut markdowns: Vec<PackMarkdownFile> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::invalid_input(format!("Invalid multipart field: {}", e)))?
    {
        let name = field.name().map(|s| s.to_string()).unwrap_or_default();
        let file_name = field.file_name().map(|s| s.to_string());
        let text = field
            .text()
            .await
            .map_err(|e| ApiError::invalid_input(format!("Invalid multipart field text: {}", e)))?;

        match name.as_str() {
            "cal_toml" | "manifest" => manifest = Some(text),
            "markdown" => {
                let path = file_name.unwrap_or_else(|| "unknown.md".to_string());
                markdowns.push(PackMarkdownFile {
                    path: PathBuf::from(path),
                    content: text,
                });
            }
            _ => {}
        }
    }

    let manifest = manifest.ok_or_else(|| ApiError::missing_field("cal_toml"))?;

    let input = PackInput {
        root: PathBuf::from("."),
        manifest,
        markdowns,
        contracts: std::collections::HashMap::new(),
    };

    match compose_pack_internal(input) {
        Ok(result) => {
            let ast_json = serde_json::to_value(&result.ast)
                .map_err(|e| ApiError::internal_error(format!("Failed to serialize AST: {}", e)))?;
            let compiled_json = serde_json::to_value(&result.compiled).map_err(|e| {
                ApiError::internal_error(format!("Failed to serialize compiled config: {}", e))
            })?;
            let dsl_source = pretty_print(&result.ast);
            Ok(Json(ComposePackResponse {
                success: true,
                ast: Some(ast_json),
                compiled: Some(compiled_json),
                dsl_source: Some(dsl_source),
                errors: Vec::new(),
            }))
        }
        Err(err) => {
            let diag = match err {
                caliber_dsl::pack::PackError::Markdown(m) => PackDiagnostic {
                    file: m.file,
                    line: m.line,
                    column: m.column,
                    message: m.message,
                },
                caliber_dsl::pack::PackError::Validation(msg)
                    if msg.starts_with("injections.") || msg.starts_with("routing.") =>
                {
                    let (file, message) = msg
                        .split_once(':')
                        .map(|(f, rest)| (f.trim().to_string(), rest.trim().to_string()))
                        .unwrap_or_else(|| ("manifest".to_string(), msg));

                    PackDiagnostic {
                        file,
                        line: 0,
                        column: 0,
                        message,
                    }
                }
                other => PackDiagnostic {
                    file: "manifest".to_string(),
                    line: 0,
                    column: 0,
                    message: other.to_string(),
                },
            };
            Ok(Json(ComposePackResponse {
                success: false,
                ast: None,
                compiled: None,
                dsl_source: None,
                errors: vec![diag],
            }))
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

/// Save a DSL configuration for the authenticated tenant, optionally activating it.
///
/// Parses and compiles either a raw DSL source or a submitted pack, persists the resulting
/// configuration with the next version number, stores the pack (if provided), and—when
/// requested—deploys the configuration. Returns metadata about the created configuration
/// (id, name, version, status, and a human-readable message).
///
/// # Examples
///
/// ```no_run
/// use crate::api::dsl::DeployDslRequest;
///
/// let req = DeployDslRequest {
///     name: "example-config".into(),
///     source: "service example { }".into(),
///     pack: None,
///     activate: false,
///     notes: None,
/// };
///
/// // Invoking the handler requires application State and authenticated context;
/// // the above illustrates constructing the request payload.
/// ```
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

    // Validate that either DSL source or pack source is provided
    if req.pack.is_none() && req.source.trim().is_empty() {
        return Err(ApiError::missing_field("source"));
    }

    // Step 1-2: Parse/compile either DSL source or pack source
    let (dsl_source, ast, compiled) = if let Some(pack) = &req.pack {
        let markdowns = pack
            .markdowns
            .iter()
            .map(|m| PackMarkdownFile {
                path: PathBuf::from(&m.path),
                content: m.content.clone(),
            })
            .collect();

        let input = PackInput {
            root: PathBuf::from("."),
            manifest: pack.manifest.clone(),
            markdowns,
            contracts: std::collections::HashMap::new(),
        };

        let output = compose_pack_internal(input)
            .map_err(|err| ApiError::invalid_input(format!("Pack composition error: {}", err)))?;

        // Store canonical DSL source for audit/debug.
        let dsl_source = pretty_print(&output.ast);
        (dsl_source, output.ast, output.compiled)
    } else {
        let ast = parse_markdown_source(&req.source)
            .map_err(|err_msg| ApiError::invalid_input(format!("Parse error: {}", err_msg)))?;

        let compiled = caliber_dsl::DslCompiler::compile(&ast)
            .map_err(|err| ApiError::invalid_input(format!("Compilation error: {}", err)))?;

        (req.source.clone(), ast, compiled)
    };

    // Step 3: Serialize for storage
    let ast_json = serde_json::to_value(&ast)
        .map_err(|e| ApiError::internal_error(format!("Failed to serialize AST: {}", e)))?;
    let compiled_json = serde_json::to_value(&compiled).map_err(|e| {
        ApiError::internal_error(format!("Failed to serialize compiled config: {}", e))
    })?;

    // Step 4: Get next version number for this config name
    let version = db
        .dsl_config_next_version(auth.tenant_id, &req.name)
        .await?;

    // Step 5: Insert the config
    let config_id = db
        .dsl_config_create(
            auth.tenant_id,
            &req.name,
            version,
            &dsl_source,
            ast_json,
            compiled_json,
        )
        .await?;

    // Step 5b: Store pack source if provided
    if let Some(pack) = &req.pack {
        let pack_json = serde_json::to_value(pack)
            .map_err(|e| ApiError::internal_error(format!("Failed to serialize pack: {}", e)))?;
        db.dsl_pack_create(config_id, auth.tenant_id, pack_json)
            .await?;
    }

    // Step 6: If activate is true, deploy it
    // Note: We pass None for agent_id since deployment is user-initiated
    let status = if req.activate {
        db.dsl_config_deploy(config_id, None, req.notes.as_deref())
            .await?;
        DslConfigStatus::Deployed
    } else {
        DslConfigStatus::Draft
    };

    let message = if req.activate {
        format!(
            "Configuration '{}' v{} deployed successfully",
            req.name, version
        )
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
        .route("/compose", axum::routing::post(compose_pack))
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