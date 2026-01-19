//! Config REST API Routes
//!
//! This module implements Axum route handlers for configuration operations.
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
    events::WsEvent,
    state::AppState,
    types::{ConfigResponse, UpdateConfigRequest, ValidateConfigRequest},
    ws::WsState,
};

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// GET /api/v1/config - Get current configuration
#[utoipa::path(
    get,
    path = "/api/v1/config",
    tag = "Config",
    responses(
        (status = 200, description = "Current configuration", body = ConfigResponse),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_config(
    State(db): State<DbClient>,
) -> ApiResult<impl IntoResponse> {
    let response = db.config_get().await?;
    Ok(Json(response))
}

/// PATCH /api/v1/config - Update configuration
#[utoipa::path(
    patch,
    path = "/api/v1/config",
    tag = "Config",
    request_body = UpdateConfigRequest,
    responses(
        (status = 200, description = "Configuration updated", body = ConfigResponse),
        (status = 400, description = "Invalid configuration", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn update_config(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    Json(req): Json<UpdateConfigRequest>,
) -> ApiResult<Json<ConfigResponse>> {
    // Validate that config is not empty
    if req.config.is_null() {
        return Err(ApiError::missing_field("config"));
    }

    // Validate that config is an object
    if !req.config.is_object() {
        return Err(ApiError::invalid_input("config must be a JSON object"));
    }

    let response = db.config_update(&req).await?;
    ws.broadcast(WsEvent::ConfigUpdated {
        config: response.clone(),
    });
    Ok(Json(response))
}

/// POST /api/v1/config/validate - Validate configuration
#[utoipa::path(
    post,
    path = "/api/v1/config/validate",
    tag = "Config",
    request_body = ValidateConfigRequest,
    responses(
        (status = 200, description = "Validation result", body = ConfigResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn validate_config(
    State(db): State<DbClient>,
    Json(req): Json<ValidateConfigRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that config is not empty
    if req.config.is_null() {
        return Err(ApiError::missing_field("config"));
    }

    // Validate that config is an object
    if !req.config.is_object() {
        return Err(ApiError::invalid_input("config must be a JSON object"));
    }

    let response = db.config_validate(&req).await?;
    Ok(Json(response))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the config routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(get_config))
        .route("/", axum::routing::patch(update_config))
        .route("/validate", axum::routing::post(validate_config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_config_request_validation() {
        let req = UpdateConfigRequest {
            config: serde_json::json!(null),
        };

        assert!(req.config.is_null());
    }

    #[test]
    fn test_update_config_request_object_validation() {
        let req = UpdateConfigRequest {
            config: serde_json::json!("not an object"),
        };

        assert!(!req.config.is_object());
    }

    #[test]
    fn test_config_response_structure() {
        let response = ConfigResponse {
            config: serde_json::json!({
                "token_budget": 8000
            }),
            valid: true,
            errors: vec![],
        };

        assert!(response.valid);
        assert!(response.errors.is_empty());
        assert!(response.config.is_object());
    }

    #[test]
    fn test_config_response_with_errors() {
        let response = ConfigResponse {
            config: serde_json::json!({
                "token_budget": -1
            }),
            valid: false,
            errors: vec![
                "token_budget must be positive".to_string(),
                "missing required field: embedding_provider".to_string(),
            ],
        };

        assert!(!response.valid);
        assert_eq!(response.errors.len(), 2);
    }

    #[test]
    fn test_valid_config_structure() {
        let config = serde_json::json!({
            "context_assembly": {
                "token_budget": 8000,
                "relevance_threshold": 0.7
            },
            "pcp_settings": {
                "checkpoint_interval": 100,
                "contradiction_detection": true
            },
            "storage": {
                "adapter": "postgres",
                "connection": "postgresql://localhost/caliber"
            },
            "llm": {
                "embedding_provider": "openai",
                "embedding_model": "text-embedding-3-small"
            },
            "multi_agent": {
                "lock_timeout_ms": 30000
            }
        });

        assert!(config.is_object());
        assert!(config.get("context_assembly").is_some());
        assert!(config.get("pcp_settings").is_some());
        assert!(config.get("storage").is_some());
        assert!(config.get("llm").is_some());
        assert!(config.get("multi_agent").is_some());
    }
}
