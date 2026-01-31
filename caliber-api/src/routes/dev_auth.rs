//! Development Authentication Endpoints
//!
//! Provides development-only authentication endpoints for testing and local development.
//! These endpoints are ONLY available when:
//! - CALIBER_ENVIRONMENT is NOT set to "production" or "prod"
//!
//! NEVER enable these in production - they bypass real authentication!
//!
//! Endpoints:
//! - POST /auth/dev/token - Generate a dev token for local testing

use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use caliber_core::{EntityIdType, TenantId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{generate_jwt_token, AuthConfig},
    error::{ApiError, ApiResult},
    state::AppState,
};

// ============================================================================
// TYPES
// ============================================================================

/// Request body for dev token generation.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DevTokenRequest {
    /// User email (optional, defaults to dev@caliber.run)
    #[serde(default = "default_email")]
    pub email: String,

    /// User name (optional, defaults to "Dev User")
    #[serde(default = "default_name")]
    pub name: String,

    /// Tenant ID (optional, creates a dev tenant if not specified)
    pub tenant_id: Option<Uuid>,
}

fn default_email() -> String {
    "dev@caliber.run".to_string()
}

fn default_name() -> String {
    "Dev User".to_string()
}

/// Response for dev token generation.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DevTokenResponse {
    /// The access token for API calls
    pub access_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// Token expiration in seconds
    pub expires_in: i64,

    /// Tenant ID
    pub tenant_id: String,

    /// User ID
    pub user_id: String,

    /// User email
    pub email: String,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// POST /auth/dev/token - Generate a development token
///
/// This endpoint is ONLY available in development mode.
/// It generates a valid JWT token for local testing without requiring
/// real authentication through WorkOS.
///
/// # Security Warning
/// This endpoint bypasses all authentication. It should NEVER be enabled
/// in production environments.
pub async fn generate_dev_token(
    Json(request): Json<DevTokenRequest>,
) -> ApiResult<impl IntoResponse> {
    // Double-check we're in development mode
    let environment = std::env::var("CALIBER_ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase();

    if environment == "production" || environment == "prod" {
        return Err(ApiError::forbidden(
            "Dev authentication is disabled in production",
        ));
    }

    // Load auth config from environment (same as the API uses)
    let auth_config = AuthConfig::from_env();

    // Use provided tenant ID or generate a dev tenant
    let tenant_id = request.tenant_id.map(TenantId::new).unwrap_or_else(|| {
        TenantId::new(
            Uuid::parse_str("00000000-0000-0000-0000-000000000001")
                .expect("valid dev tenant UUID"),
        )
    });

    // Generate user ID from email
    let user_id = format!(
        "dev-{}",
        request.email.replace('@', "-").replace('.', "-")
    );

    // Generate JWT token
    let expiration_secs = auth_config.jwt_expiration_secs;
    let token = generate_jwt_token(
        &auth_config,
        user_id.clone(),
        Some(tenant_id),
        vec!["admin".to_string(), "dev".to_string()],
    )?;

    tracing::info!(
        user_id = %user_id,
        email = %request.email,
        tenant_id = %tenant_id,
        "Generated dev token"
    );

    Ok((
        StatusCode::OK,
        Json(DevTokenResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: expiration_secs,
            tenant_id: tenant_id.to_string(),
            user_id,
            email: request.email,
        }),
    ))
}

// ============================================================================
// ROUTER
// ============================================================================

/// Check if dev auth should be enabled.
///
/// Returns true if:
/// - CALIBER_ENVIRONMENT is not set to "production" or "prod"
pub fn is_dev_auth_enabled() -> bool {
    let environment = std::env::var("CALIBER_ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase();

    environment != "production" && environment != "prod"
}

/// Create dev auth router.
///
/// This router is conditionally included based on environment.
/// In production, it returns an empty router.
pub fn create_router() -> Router<AppState> {
    if !is_dev_auth_enabled() {
        tracing::info!("Dev auth disabled (production environment)");
        return Router::new();
    }

    tracing::warn!(
        "Dev auth enabled - DO NOT USE IN PRODUCTION! \
         Set CALIBER_ENVIRONMENT=production to disable."
    );

    Router::new().route("/token", post(generate_dev_token))
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_email() {
        assert_eq!(default_email(), "dev@caliber.run");
    }

    #[test]
    fn test_default_name() {
        assert_eq!(default_name(), "Dev User");
    }
}
