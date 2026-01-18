//! User REST API Routes
//!
//! This module implements user profile and API key management endpoints.
//! These routes require authentication and return information about the
//! currently authenticated user.

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    middleware::AuthExtractor,
};

// ============================================================================
// TYPES
// ============================================================================

/// User profile response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UserProfile {
    /// User ID (from auth provider)
    pub id: String,
    /// User's email address
    pub email: String,
    /// User's first name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// User's last name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// Tenant ID (organization)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<Uuid>,
    /// User's API key (may be none if not generated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Account creation timestamp
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Response for API key regeneration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ApiKeyResponse {
    /// The new API key
    pub api_key: String,
}

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for user routes.
#[derive(Clone)]
pub struct UserState {
    pub db: DbClient,
}

impl UserState {
    pub fn new(db: DbClient) -> Self {
        Self { db }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// GET /api/v1/users/me - Get current user's profile
#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "Users",
    responses(
        (status = 200, description = "User profile", body = UserProfile),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_current_user(
    State(state): State<Arc<UserState>>,
    AuthExtractor(auth): AuthExtractor,
) -> ApiResult<impl IntoResponse> {
    // Get user profile from database or auth context
    // For now, we derive from the auth context and check for API key in DB

    // Try to get API key from database
    let api_key = state.db.user_get_api_key(&auth.user_id).await.ok().flatten();

    let profile = UserProfile {
        id: auth.user_id.clone(),
        email: auth.email.clone().unwrap_or_default(),
        first_name: auth.first_name.clone(),
        last_name: auth.last_name.clone(),
        tenant_id: Some(auth.tenant_id),
        api_key,
        created_at: chrono::Utc::now(), // In production, fetch from DB
    };

    Ok(Json(profile))
}

/// POST /api/v1/users/me/api-key - Regenerate API key
#[utoipa::path(
    post,
    path = "/api/v1/users/me/api-key",
    tag = "Users",
    responses(
        (status = 200, description = "New API key", body = ApiKeyResponse),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn regenerate_api_key(
    State(state): State<Arc<UserState>>,
    AuthExtractor(auth): AuthExtractor,
) -> ApiResult<impl IntoResponse> {
    // Generate a new API key
    let new_key = generate_api_key();

    // Store in database
    state.db.user_set_api_key(&auth.user_id, &new_key).await?;

    tracing::info!(
        user_id = %auth.user_id,
        "API key regenerated"
    );

    Ok(Json(ApiKeyResponse { api_key: new_key }))
}

// ============================================================================
// HELPERS
// ============================================================================

/// Generate a secure API key.
fn generate_api_key() -> String {
    use rand::Rng;

    const PREFIX: &str = "cal_";
    const KEY_LENGTH: usize = 32;

    let mut rng = rand::thread_rng();
    const CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let key: String = (0..KEY_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    format!("{}{}", PREFIX, key)
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the user routes router.
pub fn create_router(db: DbClient) -> Router {
    let state = Arc::new(UserState::new(db));

    Router::new()
        .route("/me", get(get_current_user))
        .route("/me/api-key", post(regenerate_api_key))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key();

        // Should start with prefix
        assert!(key.starts_with("cal_"));

        // Should be correct length (prefix + 32 chars)
        assert_eq!(key.len(), 4 + 32);

        // Should only contain alphanumeric characters after prefix
        assert!(key[4..].chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_api_keys_are_unique() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_user_profile_serialization() -> Result<(), serde_json::Error> {
        let profile = UserProfile {
            id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            tenant_id: Some(Uuid::new_v4()),
            api_key: Some("cal_abcdef123456".to_string()),
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&profile)?;

        assert!(json.contains("user_123"));
        assert!(json.contains("test@example.com"));
        assert!(json.contains("cal_abcdef123456"));
        Ok(())
    }

    #[test]
    fn test_user_profile_optional_fields() -> Result<(), serde_json::Error> {
        let profile = UserProfile {
            id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            first_name: None,
            last_name: None,
            tenant_id: None,
            api_key: None,
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&profile)?;

        // Optional fields should be skipped
        assert!(!json.contains("first_name"));
        assert!(!json.contains("last_name"));
        assert!(!json.contains("api_key"));
        Ok(())
    }
}
