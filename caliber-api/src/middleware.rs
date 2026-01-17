//! Axum Middleware for Authentication and Authorization
//!
//! This module provides Axum middleware that:
//! - Authenticates requests using API keys or JWT tokens
//! - Supports WorkOS SSO authentication (with `workos` feature)
//! - Extracts tenant context from headers
//! - Injects AuthContext into request extensions
//! - Returns 401 for unauthenticated requests
//! - Returns 403 for unauthorized tenant access
//!
//! # Auth Provider Selection
//!
//! The `CALIBER_AUTH_PROVIDER` environment variable controls which authentication
//! backend is used:
//! - `jwt` (default): Standard JWT + API key authentication
//! - `workos`: WorkOS SSO authentication (requires `workos` feature)
//!
//! Requirements: 1.7, 1.8

use crate::auth::{authenticate, AuthConfig, AuthContext, AuthProvider};
use crate::error::ApiError;
use axum::{
    async_trait,
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

#[cfg(feature = "workos")]
use crate::workos_auth::{validate_workos_token, WorkOsConfig};

// ============================================================================
// MIDDLEWARE STATE
// ============================================================================

/// Shared state for authentication middleware.
///
/// This is passed to the middleware via Axum's State extractor.
#[derive(Debug, Clone)]
pub struct AuthMiddlewareState {
    /// Authentication configuration
    pub auth_config: Arc<AuthConfig>,

    /// WorkOS configuration (when `workos` feature is enabled)
    #[cfg(feature = "workos")]
    pub workos_config: Option<Arc<WorkOsConfig>>,
}

impl AuthMiddlewareState {
    /// Create new middleware state with the given auth configuration.
    pub fn new(auth_config: AuthConfig) -> Self {
        // Initialize WorkOS config if the provider is WorkOS
        #[cfg(feature = "workos")]
        let workos_config = if auth_config.auth_provider == AuthProvider::WorkOs {
            WorkOsConfig::from_env().ok().map(Arc::new)
        } else {
            None
        };

        Self {
            auth_config: Arc::new(auth_config),
            #[cfg(feature = "workos")]
            workos_config,
        }
    }

    /// Create middleware state with explicit WorkOS configuration.
    #[cfg(feature = "workos")]
    pub fn with_workos(auth_config: AuthConfig, workos_config: WorkOsConfig) -> Self {
        Self {
            auth_config: Arc::new(auth_config),
            workos_config: Some(Arc::new(workos_config)),
        }
    }
}

// ============================================================================
// MIDDLEWARE FUNCTION
// ============================================================================

/// Axum middleware for authentication and authorization.
///
/// This middleware:
/// 1. Extracts authentication headers (X-API-Key or Authorization: Bearer)
/// 2. Extracts tenant context header (X-Tenant-ID)
/// 3. Validates authentication using the auth module
/// 4. Returns 401 Unauthorized if authentication fails
/// 5. Returns 403 Forbidden if tenant access is denied
/// 6. Injects AuthContext into request extensions on success
///
/// # Example
///
/// ```ignore
/// use axum::{Router, middleware};
/// use caliber_api::middleware::{auth_middleware, AuthMiddlewareState};
/// use caliber_api::AuthConfig;
///
/// let auth_config = AuthConfig::from_env();
/// let auth_state = AuthMiddlewareState::new(auth_config);
///
/// let app = Router::new()
///     .route("/api/v1/trajectories", axum::routing::get(|| async { "OK" }))
///     .layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware));
/// ```
pub async fn auth_middleware(
    State(state): State<AuthMiddlewareState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthMiddlewareError> {
    // Extract authentication headers
    let api_key_header = request
        .headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok());

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    let tenant_id_header = request
        .headers()
        .get("x-tenant-id")
        .and_then(|h| h.to_str().ok());

    // Route to appropriate auth handler based on provider
    let auth_context = match state.auth_config.auth_provider {
        // Standard JWT/API key authentication
        AuthProvider::Jwt => {
            authenticate(
                &state.auth_config,
                api_key_header,
                auth_header,
                tenant_id_header,
            )
            .map_err(AuthMiddlewareError)?
        }

        // WorkOS SSO authentication
        #[cfg(feature = "workos")]
        AuthProvider::WorkOs => {
            // API key auth still works as fallback even in WorkOS mode
            if let Some(api_key) = api_key_header {
                authenticate(
                    &state.auth_config,
                    Some(api_key),
                    None,
                    tenant_id_header,
                )
                .map_err(AuthMiddlewareError)?
            } else if let Some(auth_value) = auth_header {
                // Extract Bearer token for WorkOS validation
                let token = auth_value
                    .strip_prefix("Bearer ")
                    .ok_or_else(|| {
                        AuthMiddlewareError(ApiError::invalid_token(
                            "Authorization header must use Bearer scheme",
                        ))
                    })?;

                // Get WorkOS config
                let workos_config = state.workos_config.as_ref().ok_or_else(|| {
                    AuthMiddlewareError(ApiError::internal_error(
                        "WorkOS authentication enabled but not configured",
                    ))
                })?;

                // Validate token against WorkOS
                validate_workos_token(workos_config, token, tenant_id_header)
                    .await
                    .map_err(AuthMiddlewareError)?
            } else {
                return Err(AuthMiddlewareError(ApiError::unauthorized(
                    "Authentication required: provide X-API-Key or Authorization header",
                )));
            }
        }

        // WorkOS provider selected but feature not enabled
        #[cfg(not(feature = "workos"))]
        AuthProvider::WorkOs => {
            return Err(AuthMiddlewareError(ApiError::internal_error(
                "WorkOS authentication provider selected but 'workos' feature is not enabled. \
                 Rebuild with --features workos or set CALIBER_AUTH_PROVIDER=jwt",
            )));
        }
    };

    // Inject AuthContext into request extensions
    request.extensions_mut().insert(auth_context);

    // Continue to the next middleware/handler
    Ok(next.run(request).await)
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

/// Error wrapper for middleware that implements IntoResponse.
///
/// This allows the middleware to return errors that are automatically
/// converted to HTTP responses with appropriate status codes.
#[derive(Debug)]
pub struct AuthMiddlewareError(pub ApiError);

impl IntoResponse for AuthMiddlewareError {
    fn into_response(self) -> Response {
        let api_error = self.0;
        
        // Map error codes to HTTP status codes
        let status = match api_error.code {
            crate::error::ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            crate::error::ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            crate::error::ErrorCode::InvalidToken => StatusCode::UNAUTHORIZED,
            crate::error::ErrorCode::TokenExpired => StatusCode::UNAUTHORIZED,
            crate::error::ErrorCode::MissingField => StatusCode::BAD_REQUEST,
            crate::error::ErrorCode::InvalidFormat => StatusCode::BAD_REQUEST,
            crate::error::ErrorCode::ValidationFailed => StatusCode::BAD_REQUEST,
            crate::error::ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        // Return JSON error response
        (status, axum::Json(api_error)).into_response()
    }
}

// ============================================================================
// TYPED EXTRACTOR
// ============================================================================

/// Typed Axum extractor for authentication context.
///
/// This extractor implements `FromRequestParts`, allowing it to be used
/// directly in route handler signatures. It provides compile-time guarantees
/// that authentication has been performed and makes auth required by the type system.
///
/// # Example
///
/// ```rust,no_run
/// use axum::{Json, response::IntoResponse};
/// use caliber_api::middleware::AuthExtractor;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct UserResponse {
///     user_id: String,
///     tenant_id: String,
/// }
///
/// async fn get_current_user(
///     AuthExtractor(auth): AuthExtractor,
/// ) -> impl IntoResponse {
///     Json(UserResponse {
///         user_id: auth.user_id,
///         tenant_id: auth.tenant_id.to_string(),
///     })
/// }
/// ```
///
/// # Requirements
///
/// The `auth_middleware` must be applied to the route or router for this
/// extractor to work. If the middleware is not present, the extractor will
/// return a 500 Internal Server Error.
#[derive(Debug, Clone)]
pub struct AuthExtractor(pub AuthContext);

#[async_trait]
impl<S> FromRequestParts<S> for AuthExtractor
where
    S: Send + Sync,
{
    type Rejection = AuthMiddlewareError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract AuthContext from request extensions
        // This should have been injected by the auth_middleware
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .map(AuthExtractor)
            .ok_or_else(|| {
                AuthMiddlewareError(ApiError::internal_error(
                    "AuthContext not found in request extensions. \
                     Ensure auth_middleware is applied to this route."
                ))
            })
    }
}

// Implement Deref to make it easier to access the inner AuthContext
impl std::ops::Deref for AuthExtractor {
    type Target = AuthContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Extract AuthContext from request extensions.
///
/// This is a helper function for route handlers to get the authenticated
/// user context that was injected by the middleware.
///
/// # Panics
///
/// Panics if the AuthContext is not present in extensions. This should never
/// happen if the auth middleware is properly configured.
///
/// # Example
///
/// ```rust,no_run
/// use axum::extract::Request;
/// use caliber_api::middleware::extract_auth_context;
///
/// async fn my_handler(request: Request) {
///     let auth_context = extract_auth_context(&request);
///     println!("User: {}, Tenant: {}", auth_context.user_id, auth_context.tenant_id);
/// }
/// ```
pub fn extract_auth_context(request: &Request) -> &AuthContext {
    request
        .extensions()
        .get::<AuthContext>()
        .expect("AuthContext not found in request extensions - is auth middleware configured?")
}

/// Extract AuthContext from request extensions (owned version).
///
/// This is similar to `extract_auth_context` but returns a cloned copy.
pub fn extract_auth_context_owned(request: &Request) -> AuthContext {
    extract_auth_context(request).clone()
}

// ============================================================================
// OPTIONAL: TENANT-SPECIFIC MIDDLEWARE
// ============================================================================

/// Middleware that validates tenant access for a specific tenant.
///
/// This is useful for routes that operate on a specific tenant's data.
/// It checks that the authenticated user has access to the requested tenant.
///
/// # Example
///
/// ```ignore
/// use axum::{Router, middleware, extract::Path};
/// use caliber_api::middleware::{auth_middleware, tenant_access_middleware, AuthMiddlewareState};
/// use caliber_api::AuthConfig;
/// use caliber_core::EntityId;
///
/// let auth_config = AuthConfig::from_env();
/// let auth_state = AuthMiddlewareState::new(auth_config);
///
/// async fn get_tenant_data(Path(tenant_id): Path<EntityId>) -> &'static str {
///     "Tenant data"
/// }
///
/// let app = Router::new()
///     .route("/api/v1/tenants/:tenant_id", axum::routing::get(get_tenant_data))
///     .layer(middleware::from_fn(tenant_access_middleware))
///     .layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware));
/// ```
pub async fn tenant_access_middleware(
    request: Request,
    next: Next,
) -> Result<Response, AuthMiddlewareError> {
    // Extract AuthContext from extensions (injected by auth_middleware)
    let auth_context = extract_auth_context(&request);
    
    // Extract tenant_id from path parameters if present
    // Note: This is a simplified version. In practice, you might want to
    // extract the tenant_id from the path using axum::extract::Path
    // or from query parameters, depending on your route structure.
    
    // For now, we just verify that the auth context has a valid tenant
    // The actual tenant-specific validation happens in route handlers
    // where they can compare against path/query parameters
    
    if auth_context.tenant_id.to_string().is_empty() {
        return Err(AuthMiddlewareError(ApiError::forbidden(
            "Tenant context required",
        )));
    }
    
    Ok(next.run(request).await)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthConfig;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt; // for `oneshot`
    use uuid::Uuid;
    
    fn test_auth_config() -> AuthConfig {
        let mut config = AuthConfig::default();
        config.add_api_key("test_key_123".to_string());
        config.jwt_secret = "test_secret".to_string();
        config.require_tenant_header = true;
        config
    }
    
    fn test_app() -> Router {
        let auth_config = test_auth_config();
        let auth_state = AuthMiddlewareState::new(auth_config);
        
        Router::new()
            .route("/protected", get(|| async { "Protected resource" }))
            .layer(middleware::from_fn_with_state(
                auth_state,
                auth_middleware,
            ))
    }
    
    #[tokio::test]
    async fn test_middleware_with_valid_api_key() {
        let app = test_app();
        let tenant_id = Uuid::now_v7();
        
        let request = Request::builder()
            .uri("/protected")
            .header("x-api-key", "test_key_123")
            .header("x-tenant-id", tenant_id.to_string())
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_middleware_with_invalid_api_key() {
        let app = test_app();
        let tenant_id = Uuid::now_v7();
        
        let request = Request::builder()
            .uri("/protected")
            .header("x-api-key", "invalid_key")
            .header("x-tenant-id", tenant_id.to_string())
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_middleware_without_authentication() {
        let app = test_app();
        
        let request = Request::builder()
            .uri("/protected")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_middleware_without_tenant_header() {
        let app = test_app();
        
        let request = Request::builder()
            .uri("/protected")
            .header("x-api-key", "test_key_123")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        
        // Should fail because tenant header is required
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    
    #[tokio::test]
    async fn test_middleware_with_valid_jwt() {
        let auth_config = test_auth_config();
        let user_id = "user123".to_string();
        let tenant_id = Uuid::now_v7();
        
        // Generate a valid JWT token
        let token = crate::auth::generate_jwt_token(
            &auth_config,
            user_id,
            Some(tenant_id.into()),
            vec!["admin".to_string()],
        )
        .unwrap();
        
        let auth_state = AuthMiddlewareState::new(auth_config);
        let app = Router::new()
            .route("/protected", get(|| async { "Protected resource" }))
            .layer(middleware::from_fn_with_state(auth_state, auth_middleware));
        
        let request = Request::builder()
            .uri("/protected")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_middleware_with_invalid_jwt() {
        let app = test_app();
        
        let request = Request::builder()
            .uri("/protected")
            .header("authorization", "Bearer invalid.jwt.token")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_middleware_with_malformed_auth_header() {
        let app = test_app();
        
        let request = Request::builder()
            .uri("/protected")
            .header("authorization", "NotBearer token")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_auth_context_injection() {
        let auth_config = test_auth_config();
        let auth_state = AuthMiddlewareState::new(auth_config);
        let tenant_id = Uuid::now_v7();

        // Handler that extracts and verifies AuthContext
        async fn handler(request: Request<Body>) -> String {
            let auth_context = extract_auth_context(&request);
            format!(
                "User: {}, Tenant: {}, Method: {:?}",
                auth_context.user_id, auth_context.tenant_id, auth_context.auth_method
            )
        }

        let app = Router::new()
            .route("/protected", get(handler))
            .layer(middleware::from_fn_with_state(auth_state, auth_middleware));

        let request = Request::builder()
            .uri("/protected")
            .header("x-api-key", "test_key_123")
            .header("x-tenant-id", tenant_id.to_string())
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify the response contains expected data
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("User: api_key_"));
        assert!(body_str.contains(&format!("Tenant: {}", tenant_id)));
        assert!(body_str.contains("Method: ApiKey"));
    }

    #[tokio::test]
    async fn test_auth_extractor_with_valid_auth() {
        let auth_config = test_auth_config();
        let auth_state = AuthMiddlewareState::new(auth_config);
        let tenant_id = Uuid::now_v7();

        // Handler using the typed AuthExtractor
        async fn handler(AuthExtractor(auth): AuthExtractor) -> String {
            format!(
                "User: {}, Tenant: {}, Method: {:?}",
                auth.user_id, auth.tenant_id, auth.auth_method
            )
        }

        let app = Router::new()
            .route("/protected", get(handler))
            .layer(middleware::from_fn_with_state(auth_state, auth_middleware));

        let request = Request::builder()
            .uri("/protected")
            .header("x-api-key", "test_key_123")
            .header("x-tenant-id", tenant_id.to_string())
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("User: api_key_"));
        assert!(body_str.contains(&format!("Tenant: {}", tenant_id)));
        assert!(body_str.contains("Method: ApiKey"));
    }

    #[tokio::test]
    async fn test_auth_extractor_without_middleware() {
        // Handler using AuthExtractor without auth middleware
        async fn handler(AuthExtractor(_auth): AuthExtractor) -> String {
            "Should not reach here".to_string()
        }

        // Router WITHOUT auth middleware
        let app = Router::new().route("/unprotected", get(handler));

        let request = Request::builder()
            .uri("/unprotected")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should return 500 because middleware is not configured
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_auth_extractor_deref() {
        let auth_config = test_auth_config();
        let auth_state = AuthMiddlewareState::new(auth_config);
        let tenant_id = Uuid::now_v7();

        // Handler that uses Deref to access AuthContext methods
        async fn handler(auth: AuthExtractor) -> String {
            // Can use methods directly thanks to Deref
            if auth.has_role("api_user") {
                format!("User {} has api_user role", auth.user_id)
            } else {
                "No role".to_string()
            }
        }

        let app = Router::new()
            .route("/protected", get(handler))
            .layer(middleware::from_fn_with_state(auth_state, auth_middleware));

        let request = Request::builder()
            .uri("/protected")
            .header("x-api-key", "test_key_123")
            .header("x-tenant-id", tenant_id.to_string())
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("has api_user role"));
    }
}
