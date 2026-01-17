//! SSO Routes Module
//!
//! This module provides SSO (Single Sign-On) endpoints for WorkOS authentication.
//! These routes handle the OIDC authorization code flow:
//!
//! 1. `/auth/sso/authorize` - Initiates SSO login, redirects to WorkOS
//! 2. `/auth/sso/callback` - Handles WorkOS callback, exchanges code for tokens
//!
//! # Example Flow
//!
//! ```text
//! 1. User visits /auth/sso/authorize?organization=org_xyz
//! 2. Server redirects to WorkOS authorization URL
//! 3. User authenticates with their identity provider
//! 4. WorkOS redirects back to /auth/sso/callback?code=xxx
//! 5. Server exchanges code for profile and token
//! 6. Server returns session token to client
//! ```
//!
//! Enable with the `workos` feature flag.

#[cfg(feature = "workos")]
use crate::workos_auth::{
    create_session_token, exchange_code_for_profile, generate_authorization_url,
    SsoAuthorizationParams, SsoCallbackParams, SsoCallbackResponse, WorkOsConfig,
};

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use axum::Router;
use serde::{Deserialize, Serialize};

#[cfg(feature = "workos")]
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json,
};

#[cfg(feature = "workos")]
use std::sync::Arc;

#[cfg(not(feature = "workos"))]
use axum::routing::get;

// ============================================================================
// STATE
// ============================================================================

/// State for SSO routes.
#[derive(Clone)]
pub struct SsoState {
    /// Database client (for future session storage)
    pub db: DbClient,

    /// WorkOS configuration
    #[cfg(feature = "workos")]
    pub workos_config: Arc<WorkOsConfig>,
}

impl SsoState {
    /// Create new SSO state.
    #[cfg(feature = "workos")]
    pub fn new(db: DbClient, workos_config: WorkOsConfig) -> Self {
        Self {
            db,
            workos_config: Arc::new(workos_config),
        }
    }

    /// Create SSO state from environment.
    #[cfg(feature = "workos")]
    pub fn from_env(db: DbClient) -> ApiResult<Self> {
        let workos_config = WorkOsConfig::from_env()?;
        Ok(Self::new(db, workos_config))
    }
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Query parameters for SSO authorization request.
#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    /// WorkOS connection ID for direct connection (e.g., SAML)
    pub connection: Option<String>,

    /// WorkOS organization ID for organization-level SSO
    pub organization: Option<String>,

    /// Login hint (email) to pre-fill
    pub login_hint: Option<String>,

    /// State parameter for CSRF protection (optional, will be generated if not provided)
    pub state: Option<String>,

    /// Redirect URI after successful auth (for web clients)
    /// When provided, callback will redirect to this URI with token in query string
    /// instead of returning JSON. This enables browser-based auth flows.
    pub redirect_uri: Option<String>,
}

/// Encoded state for maintaining redirect_uri across the OAuth flow
#[derive(Debug, Serialize, Deserialize)]
struct AuthState {
    /// CSRF token
    csrf: String,
    /// Client redirect URI (if web flow)
    redirect_uri: Option<String>,
}

/// Response from token endpoint.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// Access token for API calls
    pub access_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// Token expiration in seconds
    pub expires_in: i64,

    /// Tenant ID from WorkOS organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// User ID from WorkOS
    pub user_id: String,

    /// User email
    pub email: String,
}

/// Response type for callback that can be either JSON or a redirect.
/// This enables both API client and web client flows.
#[cfg(feature = "workos")]
pub enum CallbackResponse {
    /// JSON response for API clients
    Json(Json<TokenResponse>),
    /// Redirect for web clients
    Redirect(Redirect),
}

#[cfg(feature = "workos")]
impl IntoResponse for CallbackResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            CallbackResponse::Json(json) => json.into_response(),
            CallbackResponse::Redirect(redirect) => redirect.into_response(),
        }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// GET /auth/sso/authorize
///
/// Initiates SSO login by redirecting to WorkOS authorization URL.
///
/// Query parameters:
/// - `connection`: WorkOS connection ID (for direct SAML/OIDC)
/// - `organization`: WorkOS organization ID (for org-level SSO)
/// - `login_hint`: Email to pre-fill in login form
/// - `state`: CSRF protection state (generated if not provided)
/// - `redirect_uri`: Redirect URI for web clients (token returned via redirect)
#[cfg(feature = "workos")]
async fn authorize(
    State(state): State<SsoState>,
    Query(params): Query<AuthorizeParams>,
) -> impl IntoResponse {
    // Generate CSRF token
    let csrf = params.state.unwrap_or_else(|| {
        uuid::Uuid::new_v4().to_string()
    });

    // Encode redirect_uri in state if provided (for web client flow)
    let state_value = if params.redirect_uri.is_some() {
        let auth_state = AuthState {
            csrf,
            redirect_uri: params.redirect_uri,
        };
        // Base64 encode the state to pass through OAuth flow
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let state_json = serde_json::to_string(&auth_state)
            .map_err(|e| ApiError::internal_error(format!("Failed to serialize state: {}", e)))?;
        URL_SAFE_NO_PAD.encode(state_json.as_bytes())
    } else {
        csrf
    };

    let auth_params = SsoAuthorizationParams {
        connection: params.connection,
        organization: params.organization,
        login_hint: params.login_hint,
        state: Some(state_value),
    };

    let auth_url = generate_authorization_url(&state.workos_config, &auth_params);

    if auth_url.is_empty() {
        return Err(ApiError::internal_error("Failed to generate authorization URL"));
    }

    Ok(Redirect::temporary(&auth_url))
}

/// GET /auth/sso/callback
///
/// Handles the OAuth callback from WorkOS after user authentication.
///
/// Query parameters:
/// - `code`: Authorization code from WorkOS
/// - `state`: State parameter for CSRF validation
///
/// Returns:
/// - For API clients: JSON response with session token and user information
/// - For web clients (redirect_uri in state): Redirects to the provided URI with token
#[cfg(feature = "workos")]
async fn callback(
    State(state): State<SsoState>,
    Query(params): Query<SsoCallbackParams>,
) -> ApiResult<CallbackResponse> {
    // Try to decode state to check for redirect_uri
    let auth_state: Option<AuthState> = params.state.as_ref().and_then(|s| {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        URL_SAFE_NO_PAD.decode(s)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|json| serde_json::from_str(&json).ok())
    });

    // Exchange authorization code for profile and token
    let (_access_token, claims) = exchange_code_for_profile(&state.workos_config, &params.code).await?;

    // Create a session token for subsequent API calls
    let expiration_secs = std::env::var("CALIBER_JWT_EXPIRATION_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600i64);

    let session_token = create_session_token(&claims, expiration_secs)?;

    // If we have a redirect_uri from web client, redirect with token
    if let Some(ref redirect_uri) = auth_state.as_ref().and_then(|s| s.redirect_uri.clone()) {
        // Build redirect URL with token
        let mut redirect_url = redirect_uri.clone();
        let separator = if redirect_url.contains('?') { "&" } else { "?" };
        redirect_url.push_str(&format!("{}token={}", separator, session_token));

        return Ok(CallbackResponse::Redirect(Redirect::temporary(&redirect_url)));
    }

    // Return JSON response for API clients
    Ok(CallbackResponse::Json(Json(TokenResponse {
        access_token: session_token,
        token_type: "Bearer".to_string(),
        expires_in: expiration_secs,
        tenant_id: claims.organization_id.clone(),
        user_id: claims.user_id,
        email: claims.email,
    })))
}

/// POST /auth/sso/callback
///
/// Alternative POST handler for the callback (some IdPs use POST).
#[cfg(feature = "workos")]
async fn callback_post(
    State(state): State<SsoState>,
    Query(params): Query<SsoCallbackParams>,
) -> ApiResult<CallbackResponse> {
    callback(State(state), Query(params)).await
}

// ============================================================================
// NON-WORKOS FALLBACKS
// ============================================================================

/// Fallback handler when workos feature is not enabled.
#[cfg(not(feature = "workos"))]
async fn authorize() -> ApiResult<()> {
    Err(ApiError::internal_error(
        "SSO is not available. Enable the 'workos' feature to use SSO authentication.",
    ))
}

/// Fallback handler when workos feature is not enabled.
#[cfg(not(feature = "workos"))]
async fn callback() -> ApiResult<()> {
    Err(ApiError::internal_error(
        "SSO is not available. Enable the 'workos' feature to use SSO authentication.",
    ))
}

/// Fallback handler when workos feature is not enabled.
#[cfg(not(feature = "workos"))]
async fn callback_post() -> ApiResult<()> {
    Err(ApiError::internal_error(
        "SSO is not available. Enable the 'workos' feature to use SSO authentication.",
    ))
}

// ============================================================================
// ROUTER
// ============================================================================

/// Create the SSO router.
///
/// Routes:
/// - GET /authorize - Initiate SSO login
/// - GET /callback - Handle OAuth callback
/// - POST /callback - Handle OAuth callback (alternative)
#[cfg(feature = "workos")]
pub fn create_router(db: DbClient, workos_config: WorkOsConfig) -> Router {
    let state = SsoState::new(db, workos_config);

    Router::new()
        .route("/authorize", get(authorize))
        .route("/callback", get(callback).post(callback_post))
        .with_state(state)
}

/// Create a stub router when workos feature is not enabled.
///
/// All routes return an error indicating SSO is not available.
#[cfg(not(feature = "workos"))]
pub fn create_router(_db: DbClient) -> Router {
    Router::new()
        .route("/authorize", get(authorize))
        .route("/callback", get(callback).post(callback_post))
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn test_sso_module_compiles() {
        // Basic compilation test
    }
}
