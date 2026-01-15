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
    types::{ConfigResponse, UpdateConfigRequest},
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for config routes.
#[derive(Clone)]
pub struct ConfigState {
    pub db: DbClient,
}

impl ConfigState {
    pub fn new(db: DbClient) -> Self {
        Self { db }
    }
}

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
    State(state): State<Arc<ConfigState>>,
) -> ApiResult<impl IntoResponse> {
    // TODO: Implement caliber_config_get in caliber-pg
    // This will:
    // 1. Retrieve the current CaliberConfig from the database
    // 2. Serialize it to JSON
    // 3. Validate all sections
    // 4. Return with validation status

    // For now, return a placeholder config
    let response = ConfigResponse {
        config: serde_json::json!({
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
                "embedding_model": "text-embedding-3-small",
                "summarization_provider": "openai",
                "summarization_model": "gpt-4"
            },
            "multi_agent": {
                "lock_timeout_ms": 30000,
                "message_ttl_ms": 3600000
            }
        }),
        valid: false,
        errors: vec!["Configuration retrieval not yet implemented in caliber-pg".to_string()],
    };

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
    State(state): State<Arc<ConfigState>>,
    Json(req): Json<UpdateConfigRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that config is not empty
    if req.config.is_null() {
        return Err(ApiError::missing_field("config"));
    }

    // Validate that config is an object
    if !req.config.is_object() {
        return Err(ApiError::invalid_input("config must be a JSON object"));
    }

    // TODO: Implement caliber_config_update in caliber-pg
    // This will:
    // 1. Validate the new configuration
    // 2. Check for required fields
    // 3. Validate value ranges and types
    // 4. Apply the configuration if valid
    // 5. Return the updated config with validation status

    // For now, return an error indicating this is not yet implemented
    Err(ApiError::internal_error(
        "Configuration update not yet implemented in caliber-pg",
    ))
}

/// POST /api/v1/config/validate - Validate configuration
#[utoipa::path(
    post,
    path = "/api/v1/config/validate",
    tag = "Config",
    request_body = UpdateConfigRequest,
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
    State(state): State<Arc<ConfigState>>,
    Json(req): Json<UpdateConfigRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that config is not empty
    if req.config.is_null() {
        return Err(ApiError::missing_field("config"));
    }

    // Validate that config is an object
    if !req.config.is_object() {
        return Err(ApiError::invalid_input("config must be a JSON object"));
    }

    // TODO: Implement caliber_config_validate in caliber-pg
    // This will:
    // 1. Parse the configuration
    // 2. Validate all sections
    // 3. Check for required fields
    // 4. Validate value ranges and types
    // 5. Return validation results without applying

    // For now, return a basic validation response
    let response = ConfigResponse {
        config: req.config,
        valid: false,
        errors: vec!["Configuration validation not yet implemented in caliber-pg".to_string()],
    };

    Ok(Json(response))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the config routes router.
pub fn create_router(db: DbClient) -> axum::Router {
    let state = Arc::new(ConfigState::new(db));

    axum::Router::new()
        .route("/", axum::routing::get(get_config))
        .route("/", axum::routing::patch(update_config))
        .route("/validate", axum::routing::post(validate_config))
        .with_state(state)
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
