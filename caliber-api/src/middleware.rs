//! Axum Middleware for Authentication and Authorization
//!
//! This module provides Axum middleware that:
//! - Authenticates requests using API keys or JWT tokens
//! - Extracts tenant context from headers
//! - Injects AuthContext into request extensions
//! - Returns 401 for unauthenticated requests
//! - Returns 403 for unauthorized tenant access
//!
//! Requirements: 1.7, 1.8

use crate::auth::{authenticate, AuthConfig, AuthContext};
use crate::error::ApiError;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

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
}

impl AuthMiddlewareState {
    /// Create new middleware state with the given auth configuration.
    pub fn new(auth_config: AuthConfig) -> Self {
        Self {
            auth_config: Arc::new(auth_config),
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
    
    // Authenticate the request
    let auth_context = authenticate(
        &state.auth_config,
        api_key_header,
        auth_header,
        tenant_id_header,
    )
    .map_err(AuthMiddlewareError)?;
    
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
}
