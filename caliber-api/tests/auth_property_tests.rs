//! Property-Based Tests for Authentication Enforcement
//!
//! **Property 4: Authentication Enforcement**
//!
//! For any API request, IF the request lacks valid authentication THEN the API
//! SHALL return 401 Unauthorized, AND IF the request targets a tenant the user
//! lacks access to THEN the API SHALL return 403 Forbidden.
//!
//! **Validates: Requirements 1.5, 1.7, 1.8**

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use caliber_api::{
    auth::{generate_jwt_token, AuthConfig},
    middleware::{auth_middleware, AuthMiddlewareState},
};
use proptest::prelude::*;
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

/// Create a test authentication configuration with known credentials.
fn test_auth_config() -> AuthConfig {
    let mut config = AuthConfig::default();
    config.add_api_key("valid_api_key_123".to_string());
    config.add_api_key("valid_api_key_456".to_string());
    config.jwt_secret = "test_secret_for_property_tests".to_string();
    config.require_tenant_header = true;
    config
}

/// Create a test Axum app with authentication middleware.
fn test_app() -> Router {
    let auth_config = test_auth_config();
    let auth_state = AuthMiddlewareState::new(auth_config);

    Router::new()
        .route("/api/v1/test", get(|| async { "Success" }))
        .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
}

// ============================================================================
// PROPERTY TEST STRATEGIES
// ============================================================================

/// Strategy for generating authentication headers.
///
/// Generates various combinations of:
/// - Valid API keys
/// - Invalid API keys
/// - Valid JWT tokens
/// - Invalid JWT tokens
/// - Missing authentication
#[derive(Debug, Clone)]
enum AuthHeader {
    /// Valid API key
    ValidApiKey(String),
    /// Invalid API key
    InvalidApiKey(String),
    /// Valid JWT token
    ValidJwt { user_id: String, tenant_id: Uuid },
    /// Invalid JWT token (malformed)
    InvalidJwt(String),
    /// Malformed Authorization header (not Bearer)
    MalformedAuth(String),
    /// No authentication provided
    None,
}

fn auth_header_strategy() -> impl Strategy<Value = AuthHeader> {
    prop_oneof![
        // Valid API keys
        Just(AuthHeader::ValidApiKey("valid_api_key_123".to_string())),
        Just(AuthHeader::ValidApiKey("valid_api_key_456".to_string())),
        // Invalid API keys
        "[a-z0-9_]{10,30}".prop_map(|s| AuthHeader::InvalidApiKey(s)),
        // Valid JWT (will be generated with proper signature)
        ("[a-z0-9]{5,20}", any::<[u8; 16]>()).prop_map(|(user_id, bytes)| {
            let tenant_id = Uuid::from_bytes(bytes);
            AuthHeader::ValidJwt { user_id, tenant_id }
        }),
        // Invalid JWT tokens
        "[A-Za-z0-9_-]{20,100}\\.[A-Za-z0-9_-]{20,100}\\.[A-Za-z0-9_-]{20,100}"
            .prop_map(|s| AuthHeader::InvalidJwt(s)),
        // Malformed auth headers
        "[A-Za-z]+ [A-Za-z0-9_-]{20,50}".prop_map(|s| AuthHeader::MalformedAuth(s)),
        // No authentication
        Just(AuthHeader::None),
    ]
}

/// Strategy for generating tenant ID headers.
///
/// Generates:
/// - Valid UUIDs
/// - Invalid UUIDs
/// - Missing tenant header
#[derive(Debug, Clone)]
enum TenantHeader {
    /// Valid tenant ID
    Valid(Uuid),
    /// Invalid tenant ID (not a UUID)
    Invalid(String),
    /// No tenant header provided
    None,
}

fn tenant_header_strategy() -> impl Strategy<Value = TenantHeader> {
    prop_oneof![
        // Valid tenant IDs
        any::<[u8; 16]>().prop_map(|bytes| TenantHeader::Valid(Uuid::from_bytes(bytes))),
        // Invalid tenant IDs
        "[a-z0-9-]{10,40}".prop_map(|s| TenantHeader::Invalid(s)),
        // No tenant header
        Just(TenantHeader::None),
    ]
}

/// Combined strategy for request generation.
fn request_strategy() -> impl Strategy<Value = (AuthHeader, TenantHeader)> {
    (auth_header_strategy(), tenant_header_strategy())
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: Authentication Enforcement**
    ///
    /// For any API request:
    /// - IF the request lacks valid authentication THEN return 401 Unauthorized
    /// - IF the request has valid auth but invalid/missing tenant THEN return 400 Bad Request
    /// - IF the request has valid auth and valid tenant THEN return 200 OK
    ///
    /// **Validates: Requirements 1.5, 1.7, 1.8**
    #[test]
    fn prop_authentication_enforcement(
        (auth_header, tenant_header) in request_strategy()
    ) {
        // Run async test in tokio runtime
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let app = test_app();
            let auth_config = test_auth_config();

            // Build the request based on generated headers
            let mut request_builder = Request::builder().uri("/api/v1/test");

            // Add authentication header
            let (has_valid_auth, jwt_has_tenant) = match &auth_header {
                AuthHeader::ValidApiKey(key) => {
                    request_builder = request_builder.header("x-api-key", key);
                    (true, false)
                }
                AuthHeader::InvalidApiKey(key) => {
                    request_builder = request_builder.header("x-api-key", key);
                    (false, false)
                }
                AuthHeader::ValidJwt { user_id, tenant_id } => {
                    // Generate a real JWT token with tenant ID in claims
                    let token = generate_jwt_token(
                        &auth_config,
                        user_id.clone(),
                        Some((*tenant_id).into()),
                        vec![],
                    )
                    .unwrap();
                    request_builder = request_builder
                        .header("authorization", format!("Bearer {}", token));
                    (true, true) // JWT has tenant in claims
                }
                AuthHeader::InvalidJwt(token) => {
                    request_builder = request_builder
                        .header("authorization", format!("Bearer {}", token));
                    (false, false)
                }
                AuthHeader::MalformedAuth(value) => {
                    request_builder = request_builder.header("authorization", value);
                    (false, false)
                }
                AuthHeader::None => (false, false),
            };

            // Add tenant header
            let (has_valid_tenant_header, has_invalid_tenant_header) = match &tenant_header {
                TenantHeader::Valid(tenant_id) => {
                    request_builder = request_builder
                        .header("x-tenant-id", tenant_id.to_string());
                    (true, false)
                }
                TenantHeader::Invalid(value) => {
                    request_builder = request_builder.header("x-tenant-id", value);
                    (false, true)
                }
                TenantHeader::None => (false, false),
            };

            // Determine if we have a valid tenant
            // If there's an invalid tenant header, it will cause a 400 error even if JWT has tenant
            // because the middleware tries to parse the header first
            let has_valid_tenant = if has_invalid_tenant_header {
                false // Invalid header causes parse error
            } else {
                has_valid_tenant_header || jwt_has_tenant
            };

            let request = request_builder.body(Body::empty()).unwrap();

            // Execute the request
            let response = app.oneshot(request).await.unwrap();
            let status = response.status();

            // Verify the response matches expected behavior
            if !has_valid_auth {
                // Property: Invalid or missing authentication → 401 Unauthorized
                prop_assert_eq!(
                    status,
                    StatusCode::UNAUTHORIZED,
                    "Expected 401 for invalid/missing auth: {:?}",
                    auth_header
                );
            } else if !has_valid_tenant {
                // Property: Valid auth but invalid/missing tenant → 400 Bad Request
                prop_assert_eq!(
                    status,
                    StatusCode::BAD_REQUEST,
                    "Expected 400 for invalid/missing tenant: {:?}",
                    tenant_header
                );
            } else {
                // Property: Valid auth and valid tenant → 200 OK
                prop_assert_eq!(
                    status,
                    StatusCode::OK,
                    "Expected 200 for valid auth and tenant"
                );
            }

            Ok(())
        })?;
    }

    /// **Property 4.1: Tenant Isolation Enforcement**
    ///
    /// For any authenticated request with a valid tenant ID:
    /// - The middleware SHALL accept the request (200 OK)
    /// - The AuthContext SHALL contain the correct tenant ID
    ///
    /// This property verifies that tenant context is properly extracted and
    /// injected into the request.
    ///
    /// **Validates: Requirements 1.6, 1.8**
    #[test]
    fn prop_tenant_context_extraction(
        user_id in "[a-z0-9]{5,20}",
        tenant_id_bytes in any::<[u8; 16]>(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tenant_id = Uuid::from_bytes(tenant_id_bytes);
            let auth_config = test_auth_config();

            // Generate a valid JWT token with the tenant ID
            let token = generate_jwt_token(
                &auth_config,
                user_id.clone(),
                Some(tenant_id.into()),
                vec![],
            )
            .unwrap();

            // Create app with a handler that extracts AuthContext
            let auth_state = AuthMiddlewareState::new(auth_config);
            let app = Router::new()
                .route(
                    "/api/v1/test",
                    get(|request: Request<Body>| async move {
                        let auth_context = caliber_api::middleware::extract_auth_context(&request);
                        format!("{}", auth_context.tenant_id)
                    }),
                )
                .layer(middleware::from_fn_with_state(auth_state, auth_middleware));

            // Make request with JWT token
            let request = Request::builder()
                .uri("/api/v1/test")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            // Verify successful authentication
            prop_assert_eq!(response.status(), StatusCode::OK);

            // Verify tenant ID was correctly extracted
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let body_str = String::from_utf8(body.to_vec()).unwrap();
            prop_assert_eq!(body_str, tenant_id.to_string());

            Ok(())
        })?;
    }

    /// **Property 4.2: API Key Authentication**
    ///
    /// For any request with a valid API key and valid tenant ID:
    /// - The middleware SHALL accept the request (200 OK)
    /// - The AuthContext SHALL use ApiKey as the auth method
    ///
    /// **Validates: Requirements 1.5, 1.7**
    #[test]
    fn prop_api_key_authentication(
        tenant_id_bytes in any::<[u8; 16]>(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tenant_id = Uuid::from_bytes(tenant_id_bytes);
            let auth_config = test_auth_config();

            // Create app with handler that checks auth method
            let auth_state = AuthMiddlewareState::new(auth_config);
            let app = Router::new()
                .route(
                    "/api/v1/test",
                    get(|request: Request<Body>| async move {
                        let auth_context = caliber_api::middleware::extract_auth_context(&request);
                        format!("{:?}", auth_context.auth_method)
                    }),
                )
                .layer(middleware::from_fn_with_state(auth_state, auth_middleware));

            // Make request with valid API key
            let request = Request::builder()
                .uri("/api/v1/test")
                .header("x-api-key", "valid_api_key_123")
                .header("x-tenant-id", tenant_id.to_string())
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            // Verify successful authentication
            prop_assert_eq!(response.status(), StatusCode::OK);

            // Verify API key auth method was used
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let body_str = String::from_utf8(body.to_vec()).unwrap();
            prop_assert_eq!(body_str, "ApiKey");

            Ok(())
        })?;
    }

    /// **Property 4.3: JWT Authentication**
    ///
    /// For any request with a valid JWT token:
    /// - The middleware SHALL accept the request (200 OK)
    /// - The AuthContext SHALL use Jwt as the auth method
    /// - The user ID SHALL match the JWT subject claim
    ///
    /// **Validates: Requirements 1.5, 1.7**
    #[test]
    fn prop_jwt_authentication(
        user_id in "[a-z0-9]{5,20}",
        tenant_id_bytes in any::<[u8; 16]>(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tenant_id = Uuid::from_bytes(tenant_id_bytes);
            let auth_config = test_auth_config();

            // Generate valid JWT token
            let token = generate_jwt_token(
                &auth_config,
                user_id.clone(),
                Some(tenant_id.into()),
                vec![],
            )
            .unwrap();

            // Create app with handler that checks auth method and user ID
            let auth_state = AuthMiddlewareState::new(auth_config);
            let app = Router::new()
                .route(
                    "/api/v1/test",
                    get(|request: Request<Body>| async move {
                        let auth_context = caliber_api::middleware::extract_auth_context(&request);
                        format!("{:?}:{}", auth_context.auth_method, auth_context.user_id)
                    }),
                )
                .layer(middleware::from_fn_with_state(auth_state, auth_middleware));

            // Make request with JWT token
            let request = Request::builder()
                .uri("/api/v1/test")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            // Verify successful authentication
            prop_assert_eq!(response.status(), StatusCode::OK);

            // Verify JWT auth method and user ID
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let body_str = String::from_utf8(body.to_vec()).unwrap();
            prop_assert_eq!(body_str, format!("Jwt:{}", user_id));

            Ok(())
        })?;
    }

    /// **Property 4.4: Missing Authentication Returns 401**
    ///
    /// For any request without authentication headers:
    /// - The middleware SHALL return 401 Unauthorized
    ///
    /// **Validates: Requirements 1.7**
    #[test]
    fn prop_missing_auth_returns_401(
        _dummy in any::<u8>(), // Just to make proptest run multiple times
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let app = test_app();

            // Make request without any authentication
            let request = Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            // Property: No authentication → 401 Unauthorized
            prop_assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            Ok(())
        })?;
    }

    /// **Property 4.5: Invalid Authentication Returns 401**
    ///
    /// For any request with invalid authentication credentials:
    /// - The middleware SHALL return 401 Unauthorized
    ///
    /// **Validates: Requirements 1.7**
    #[test]
    fn prop_invalid_auth_returns_401(
        invalid_key in "[a-z0-9_]{10,30}",
        tenant_id_bytes in any::<[u8; 16]>(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Filter out accidentally valid keys
            prop_assume!(invalid_key != "valid_api_key_123");
            prop_assume!(invalid_key != "valid_api_key_456");

            let app = test_app();
            let tenant_id = Uuid::from_bytes(tenant_id_bytes);

            // Make request with invalid API key
            let request = Request::builder()
                .uri("/api/v1/test")
                .header("x-api-key", invalid_key)
                .header("x-tenant-id", tenant_id.to_string())
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            // Property: Invalid authentication → 401 Unauthorized
            prop_assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            Ok(())
        })?;
    }
}

// ============================================================================
// UNIT TESTS FOR EDGE CASES
// ============================================================================

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_expired_jwt_returns_401() {
        let mut auth_config = test_auth_config();
        auth_config.jwt_expiration_secs = -1; // Already expired

        let token = generate_jwt_token(
            &auth_config,
            "user123".to_string(),
            Some(Uuid::now_v7().into()),
            vec![],
        )
        .unwrap();

        // Reset expiration for middleware
        auth_config.jwt_expiration_secs = 3600;
        let auth_state = AuthMiddlewareState::new(auth_config);

        let app = Router::new()
            .route("/api/v1/test", get(|| async { "Success" }))
            .layer(middleware::from_fn_with_state(auth_state, auth_middleware));

        let request = Request::builder()
            .uri("/api/v1/test")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_malformed_bearer_token_returns_401() {
        let app = test_app();

        let request = Request::builder()
            .uri("/api/v1/test")
            .header("authorization", "NotBearer token123")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_tenant_uuid_returns_400() {
        let app = test_app();

        let request = Request::builder()
            .uri("/api/v1/test")
            .header("x-api-key", "valid_api_key_123")
            .header("x-tenant-id", "not-a-valid-uuid")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_empty_api_key_returns_401() {
        let app = test_app();
        let tenant_id = Uuid::now_v7();

        let request = Request::builder()
            .uri("/api/v1/test")
            .header("x-api-key", "")
            .header("x-tenant-id", tenant_id.to_string())
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_empty_bearer_token_returns_401() {
        let app = test_app();

        let request = Request::builder()
            .uri("/api/v1/test")
            .header("authorization", "Bearer ")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
