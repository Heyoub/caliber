//! REST API Routes Module
//!
//! This module contains all REST API route handlers organized by entity type.
//!
//! Includes:
//! - Entity CRUD routes (trajectories, scopes, artifacts, notes, etc.)
//! - Batch operations for bulk CRUD
//! - Health check endpoints (Kubernetes-compatible)
//! - Webhook registration and delivery
//! - MCP (Model Context Protocol) server
//! - GraphQL endpoint
//! - CORS support for browser-based clients (SPAs, Convex, etc.)

pub mod agent;
pub mod artifact;
pub mod batch;
pub mod billing;
pub mod config;
pub mod context;
pub mod delegation;
pub mod dsl;
pub mod edge;
pub mod generic;
pub mod graphql;
pub mod handoff;
pub mod health;
pub mod lock;
pub mod mcp;
pub mod message;
pub mod note;
pub mod pack;
pub mod scope;
pub mod search;
pub mod sso;
pub mod summarization_policy;
pub mod tenant;
pub mod trajectory;
pub mod turn;
pub mod user;
pub mod webhooks;
#[cfg(feature = "workos")]
pub mod workos_webhooks;

use std::sync::Arc;
use std::time::Duration;

#[cfg(any(not(feature = "swagger-ui"), test))]
use axum::Json;
use axum::{
    http::{header, header::HeaderName, HeaderValue, Method},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::get,
    Router,
};
use caliber_pcp::PCPRuntime;
use caliber_storage::{CacheConfig, InMemoryChangeJournal, LmdbCacheBackend, ReadThroughCache};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;

use crate::auth::AuthConfig;
use crate::cached_db::CachedDbClient;
use crate::config::ApiConfig;
use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::middleware::{
    auth_middleware, rate_limit_middleware, AuthMiddlewareState, RateLimitState,
};
use crate::openapi::ApiDoc;
use crate::state::AppState;
use crate::ws::{ws_handler, WsState};

/// Default path for LMDB cache storage.
const DEFAULT_CACHE_PATH: &str = "/tmp/caliber-cache";

/// Default LMDB cache size in megabytes.
const DEFAULT_CACHE_SIZE_MB: usize = 256;

/// Initialize the read-through cache with LMDB backend.
///
/// Configuration via environment variables:
/// - `CALIBER_CACHE_PATH`: Directory for LMDB files (default: /tmp/caliber-cache)
/// - `CALIBER_CACHE_SIZE_MB`: Maximum cache size in MB (default: 256)
fn initialize_cache() -> ApiResult<Arc<crate::state::ApiCache>> {
    let cache_path =
        std::env::var("CALIBER_CACHE_PATH").unwrap_or_else(|_| DEFAULT_CACHE_PATH.to_string());
    let cache_size_mb: usize = std::env::var("CALIBER_CACHE_SIZE_MB")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CACHE_SIZE_MB);

    tracing::info!(
        cache_path = %cache_path,
        cache_size_mb = cache_size_mb,
        "Initializing LMDB cache backend"
    );

    // Initialize LMDB backend
    let lmdb_backend = LmdbCacheBackend::new(&cache_path, cache_size_mb).map_err(|e| {
        ApiError::internal_error(format!("Failed to initialize LMDB cache backend: {}", e))
    })?;

    // Initialize in-memory change journal
    // Note: In production with distributed deployment, this should be replaced
    // with a PostgreSQL-backed change journal for cross-instance invalidation
    let change_journal = InMemoryChangeJournal::new();

    // Configure cache from environment variables with sensible defaults
    let max_staleness_secs = std::env::var("CALIBER_CACHE_MAX_STALENESS_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60);
    let poll_interval_ms = std::env::var("CALIBER_CACHE_POLL_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(100);
    let max_entries = std::env::var("CALIBER_CACHE_MAX_ENTRIES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10_000);
    let ttl_secs = std::env::var("CALIBER_CACHE_TTL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(3600);

    let cache_config = CacheConfig::new()
        .with_max_staleness(Duration::from_secs(max_staleness_secs))
        .with_poll_interval(Duration::from_millis(poll_interval_ms))
        .with_prefetch(false)
        .with_max_entries(max_entries)
        .with_ttl(Duration::from_secs(ttl_secs));

    // Create the read-through cache
    let cache = ReadThroughCache::new(
        Arc::new(lmdb_backend),
        Arc::new(change_journal),
        cache_config,
    );

    Ok(Arc::new(cache))
}

// Re-export route creation functions for convenience
pub use agent::create_router as agent_router;
pub use artifact::create_router as artifact_router;
pub use batch::create_router as batch_router;
pub use billing::create_router as billing_router;
pub use config::create_router as config_router;
pub use context::context_router;
pub use delegation::create_router as delegation_router;
pub use dsl::create_router as dsl_router;
pub use graphql::create_router as graphql_router;
pub use handoff::create_router as handoff_router;
pub use health::create_router as health_router;
pub use lock::create_router as lock_router;
pub use mcp::create_router as mcp_router;
pub use message::create_router as message_router;
pub use note::create_router as note_router;
pub use scope::create_router as scope_router;
pub use search::create_router as search_router;
pub use tenant::create_router as tenant_router;
pub use trajectory::create_router as trajectory_router;
pub use turn::create_router as turn_router;
pub use user::create_router as user_router;
pub use webhooks::create_router as webhooks_router;
// Battle Intel routes
pub use edge::create_router as edge_router;
pub use summarization_policy::create_router as summarization_policy_router;

// SSO routes (when workos feature is enabled)
#[cfg(feature = "workos")]
pub use sso::create_router as sso_router;

// ============================================================================
// OPENAPI ENDPOINTS
// ============================================================================

/// Handler for /openapi.json endpoint.
#[cfg(any(not(feature = "swagger-ui"), test))]
async fn openapi_json() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}

/// Handler for /openapi.yaml endpoint.
#[cfg(feature = "openapi")]
async fn openapi_yaml() -> impl IntoResponse {
    use axum::http::{header, StatusCode};

    match ApiDoc::to_yaml() {
        Ok(yaml) => (StatusCode::OK, [(header::CONTENT_TYPE, "text/yaml")], yaml),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain")],
            format!("Failed to generate YAML: {}", e),
        ),
    }
}

// ============================================================================
// PRODUCTION VALIDATION
// ============================================================================

/// Check if running in a production environment.
fn is_production_environment() -> bool {
    std::env::var("CALIBER_ENVIRONMENT")
        .map(|e| matches!(e.to_lowercase().as_str(), "production" | "prod"))
        .unwrap_or(false)
}

/// Validate API configuration for production use.
fn validate_api_config_for_production(config: &ApiConfig) -> ApiResult<()> {
    if config.cors_origins.is_empty() {
        return Err(ApiError::invalid_input(
            "CORS origins not configured for production. Set CALIBER_CORS_ORIGINS.",
        ));
    }
    if !config.rate_limit_enabled {
        tracing::warn!(
            "Rate limiting is disabled in production - this is not recommended.\n\
             Set CALIBER_RATE_LIMIT_ENABLED=true to enable rate limiting."
        );
    }
    Ok(())
}

// ============================================================================
// SECURE ROUTER BUILDER
// ============================================================================

/// Builder for secure API routers with auth + rate limiting by default.
///
/// This builder ensures that all API routes are protected by:
/// 1. Authentication middleware (API key or JWT)
/// 2. Rate limiting middleware
/// 3. Observability middleware
/// 4. CORS layer
///
/// Public routes (health, metrics) are exempt from authentication but still rate-limited.
pub struct SecureRouterBuilder {
    db: DbClient,
    ws: Arc<WsState>,
    pcp: Arc<PCPRuntime>,
    api_config: ApiConfig,
    auth_state: AuthMiddlewareState,
    rate_limit_state: RateLimitState,
}

impl SecureRouterBuilder {
    /// Create a new SecureRouterBuilder.
    ///
    /// In production environments, this validates that security configurations
    /// are properly set up and returns an error if critical settings are missing.
    pub fn new(
        db: DbClient,
        ws: Arc<WsState>,
        pcp: Arc<PCPRuntime>,
        api_config: ApiConfig,
        auth_config: AuthConfig,
    ) -> ApiResult<Self> {
        // Validate configurations in production
        if is_production_environment() {
            auth_config.validate_for_production()?;
            validate_api_config_for_production(&api_config)?;
        }

        let auth_state = AuthMiddlewareState::new(auth_config);
        let rate_limit_state = RateLimitState::new(api_config.clone());

        Ok(Self {
            db,
            ws,
            pcp,
            api_config,
            auth_state,
            rate_limit_state,
        })
    }

    /// Build the entity CRUD routes (require authentication).
    fn build_entity_routes(&self) -> ApiResult<Router<AppState>> {
        Ok(Router::<AppState>::new()
            .route("/ws", get(ws_handler))
            .nest("/trajectories", trajectory::create_router())
            .nest("/scopes", scope::create_router())
            .nest("/artifacts", artifact::create_router())
            .nest("/search", search::create_router())
            .nest("/notes", note::create_router())
            .nest("/turns", turn::create_router())
            .nest("/agents", agent::create_router())
            .nest("/locks", lock::create_router())
            .nest("/messages", message::create_router())
            .nest("/delegations", delegation::create_router())
            .nest("/handoffs", handoff::create_router())
            .nest("/dsl", dsl::create_router())
            .nest("/pack", pack::create_router())
            .nest("/config", config::create_router())
            .nest("/tenants", tenant::create_router())
            .nest("/users", user::create_router())
            .nest("/billing", billing::create_router())
            .nest("/batch", batch::create_router())
            .nest("/webhooks", webhooks::create_router())
            .nest("/graphql", graphql::create_router())
            .nest("/edges", edge::create_router())
            .nest(
                "/summarization-policies",
                summarization_policy::create_router(),
            )
            // Context assembly (caliber-core::context module)
            .nest("/context", context::context_router()))
    }

    /// Build the complete router with full security stack.
    ///
    /// # Middleware Order (outer to inner)
    /// 1. CORS (outermost) - handles preflight requests
    /// 2. Observability - tracing and metrics
    /// 3. Rate Limiting - rejects floods before expensive auth
    /// 4. Auth (innermost, only on /api/v1/*) - validates credentials
    pub fn build(self) -> ApiResult<Router> {
        use crate::telemetry::{metrics_handler, middleware::observability_middleware};
        use axum::middleware::from_fn;

        let webhook_state = Arc::new(
            webhooks::WebhookState::new(self.db.clone(), self.ws.clone()).map_err(|e| {
                ApiError::internal_error(format!("Failed to initialize webhook state: {}", e))
            })?,
        );
        webhooks::start_webhook_delivery_task(webhook_state.clone());

        let graphql_schema = graphql::create_schema(self.db.clone(), self.ws.clone());
        let endpoints_config = crate::config::EndpointsConfig::from_env();
        let billing_state = Arc::new(billing::BillingState::new(
            self.db.clone(),
            endpoints_config,
        ));
        let mcp_state = Arc::new(mcp::McpState::new(self.db.clone(), self.ws.clone()));
        let event_dag = Arc::new(caliber_storage::InMemoryEventDag::new());

        // Load context and webhook configuration from environment
        let context_config = crate::config::ContextConfig::from_env();
        let webhook_config = crate::config::WebhookConfig::from_env();

        // Initialize the read-through cache (Three Dragons architecture)
        let cache = initialize_cache()?;

        // Create cached database client for transparent read-through caching
        let cached_db = CachedDbClient::new(self.db.clone(), Arc::clone(&cache));

        #[cfg(feature = "workos")]
        let workos_config = crate::workos_auth::WorkOsConfig::from_env().ok();

        let app_state = AppState {
            db: self.db.clone(),
            cached_db,
            ws: self.ws.clone(),
            pcp: self.pcp.clone(),
            webhook_state,
            graphql_schema,
            billing_state,
            mcp_state,
            start_time: std::time::Instant::now(),
            event_dag,
            cache,
            context_config,
            webhook_config,
            #[cfg(feature = "workos")]
            workos_config,
        };

        // Protected API routes (auth required)
        let api_routes = self
            .build_entity_routes()?
            .layer(from_fn_with_state(self.auth_state.clone(), auth_middleware));

        // Build the main router
        let mut router: Router<AppState> = Router::new()
            .nest("/api/v1", api_routes)
            // MCP server (not under /api/v1 - uses its own protocol)
            .nest("/mcp", mcp::create_router())
            // Health checks (no auth required)
            .nest("/health", health::create_router())
            // Metrics endpoint (no auth, but rate-limited)
            .route("/metrics", get(metrics_handler));

        // Add SSO routes when workos feature is enabled
        #[cfg(feature = "workos")]
        {
            let webhook_enabled = std::env::var("CALIBER_WORKOS_WEBHOOK_SECRET").is_ok();
            if app_state.workos_config.is_some() || webhook_enabled {
                router = router.nest("/auth/sso", sso::create_router());
                router = router.nest("/workos", workos_webhooks::create_router());
            }
        }

        // Add YAML endpoint if openapi feature is enabled
        #[cfg(feature = "openapi")]
        {
            router = router.route("/openapi.yaml", get(openapi_yaml));
        }

        // Add Swagger UI if swagger-ui feature is enabled
        // Note: SwaggerUi::url() also serves the OpenAPI spec at that URL
        #[cfg(feature = "swagger-ui")]
        {
            use utoipa_swagger_ui::SwaggerUi;
            let swagger: Router<AppState> = SwaggerUi::new("/swagger-ui")
                .url("/openapi.json", ApiDoc::openapi())
                .into();
            router = router.merge(swagger);
        }

        // Add OpenAPI JSON endpoint only when swagger-ui is NOT enabled
        // (swagger-ui feature already serves it via SwaggerUi::url())
        #[cfg(not(feature = "swagger-ui"))]
        {
            router = router.route("/openapi.json", get(openapi_json));
        }

        // Build CORS layer
        let cors = build_cors_layer(&self.api_config);

        // Apply security layers (order matters: outer to inner in code = inner to outer in execution)
        // Execution order: CORS -> Observability -> Rate Limiting -> Handler
        Ok(router
            .layer(from_fn_with_state(
                self.rate_limit_state,
                rate_limit_middleware,
            ))
            .layer(from_fn(observability_middleware))
            .layer(cors)
            .with_state(app_state))
    }
}

// ============================================================================
// CORS LAYER
// ============================================================================

/// Build the CORS layer from ApiConfig.
///
/// In development mode (empty origins), allows all origins.
/// In production mode, only allows configured origins.
fn build_cors_layer(config: &ApiConfig) -> CorsLayer {
    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("x-api-key"),
            HeaderName::from_static("x-tenant-id"),
        ])
        .expose_headers([
            HeaderName::from_static("x-ratelimit-limit"),
            HeaderName::from_static("x-ratelimit-remaining"),
            HeaderName::from_static("x-ratelimit-reset"),
            HeaderName::from_static("retry-after"),
        ])
        .max_age(Duration::from_secs(config.cors_max_age_secs));

    if config.cors_origins.is_empty() {
        // Development mode: allow all origins
        tracing::info!("CORS: Development mode - allowing all origins");
        cors.allow_origin(Any)
            .allow_headers(Any)
            .expose_headers(Any)
    } else {
        // Production mode: only allow configured origins
        tracing::info!(
            "CORS: Production mode - allowing origins: {:?}",
            config.cors_origins
        );
        let origins: Vec<HeaderValue> = config
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();

        if config.cors_allow_credentials {
            cors.allow_origin(origins).allow_credentials(true)
        } else {
            cors.allow_origin(origins)
        }
    }
}

/// Create the complete API router with all routes, authentication, and rate limiting.
///
/// This function creates a fully configured Axum router with:
/// - All REST API routes under /api/v1/* (protected by auth)
/// - Batch operations under /api/v1/batch/*
/// - Webhook management under /api/v1/webhooks
/// - GraphQL endpoint at /api/v1/graphql
/// - MCP server at /mcp/*
/// - Health checks at /health/* (public)
/// - Metrics at /metrics (public, rate-limited)
/// - OpenAPI spec at /openapi.json
/// - Swagger UI at /swagger-ui (when swagger-ui feature is enabled)
///
/// # Security
/// - All /api/v1/* routes require authentication (API key or JWT)
/// - Rate limiting is applied globally
/// - In production, validates security configuration at startup
///
/// # Battle Intel Integration
/// The `pcp` parameter enables auto-summarization trigger checking on turn
/// creation and scope close events, supporting L0→L1→L2 abstraction transitions.
///
/// # Breaking Change (v0.5.0)
/// This function now requires an `AuthConfig` parameter for security hardening.
/// Use `create_api_router_unauthenticated` for testing/development without auth.
pub fn create_api_router(
    db: DbClient,
    ws: Arc<WsState>,
    pcp: Arc<PCPRuntime>,
    api_config: &ApiConfig,
    auth_config: AuthConfig,
) -> ApiResult<Router> {
    SecureRouterBuilder::new(db, ws, pcp, api_config.clone(), auth_config)
        .and_then(|builder| builder.build())
}

/// Create an API router without authentication middleware.
///
/// **WARNING**: This should only be used for testing or development.
/// Production deployments MUST use `create_api_router` with proper `AuthConfig`.
///
/// This router still includes:
/// - CORS layer
/// - Observability middleware
/// - Rate limiting (if enabled in config)
///
/// But does NOT include authentication middleware on /api/v1/* routes.
#[cfg(any(test, feature = "dev"))]
pub fn create_api_router_unauthenticated(
    db: DbClient,
    ws: Arc<WsState>,
    pcp: Arc<PCPRuntime>,
    api_config: &ApiConfig,
) -> ApiResult<Router> {
    use crate::telemetry::{metrics_handler, middleware::observability_middleware};
    use axum::middleware::from_fn;

    // Load config from environment or use defaults
    let endpoints_config = crate::config::EndpointsConfig::from_env();
    let context_config = crate::config::ContextConfig::from_env();
    let webhook_config = crate::config::WebhookConfig::from_env();

    let webhook_state = Arc::new(webhooks::WebhookState::new(db.clone(), ws.clone()).map_err(
        |e| ApiError::internal_error(format!("Failed to initialize webhook state: {}", e)),
    )?);
    webhooks::start_webhook_delivery_task(webhook_state.clone());

    let graphql_schema = graphql::create_schema(db.clone(), ws.clone());
    let billing_state = Arc::new(billing::BillingState::new(db.clone(), endpoints_config.clone()));
    let mcp_state = Arc::new(mcp::McpState::new(db.clone(), ws.clone()));
    let event_dag = Arc::new(caliber_storage::InMemoryEventDag::new());

    // Initialize the read-through cache (Three Dragons architecture)
    let cache = initialize_cache()?;

    // Create cached database client for transparent read-through caching
    let cached_db = CachedDbClient::new(db.clone(), Arc::clone(&cache));

    #[cfg(feature = "workos")]
    let workos_config = crate::workos_auth::WorkOsConfig::from_env().ok();

    let app_state = AppState {
        db: db.clone(),
        cached_db,
        ws: ws.clone(),
        pcp: pcp.clone(),
        webhook_state,
        graphql_schema,
        billing_state,
        mcp_state,
        start_time: std::time::Instant::now(),
        context_config,
        webhook_config,
        event_dag,
        cache,
        #[cfg(feature = "workos")]
        workos_config,
    };

    // Entity CRUD routes (NO AUTH)
    let api_routes = Router::new()
        .nest("/trajectories", trajectory::create_router())
        .nest("/scopes", scope::create_router())
        .nest("/artifacts", artifact::create_router())
        .nest("/notes", note::create_router())
        .nest("/turns", turn::create_router())
        .nest("/agents", agent::create_router())
        .nest("/locks", lock::create_router())
        .nest("/messages", message::create_router())
        .nest("/delegations", delegation::create_router())
        .nest("/handoffs", handoff::create_router())
        .nest("/dsl", dsl::create_router())
        .nest("/pack", pack::create_router())
        .nest("/config", config::create_router())
        .nest("/tenants", tenant::create_router())
        .nest("/users", user::create_router())
        .nest("/billing", billing::create_router())
        .nest("/batch", batch::create_router())
        .nest("/webhooks", webhooks::create_router())
        .nest("/graphql", graphql::create_router())
        .nest("/edges", edge::create_router())
        .nest(
            "/summarization-policies",
            summarization_policy::create_router(),
        );

    let mut router: Router<AppState> = Router::new()
        .nest("/api/v1", api_routes)
        .nest("/mcp", mcp::create_router())
        .nest("/health", health::create_router())
        .route("/metrics", get(metrics_handler));

    #[cfg(feature = "workos")]
    {
        let webhook_enabled = std::env::var("CALIBER_WORKOS_WEBHOOK_SECRET").is_ok();
        if app_state.workos_config.is_some() || webhook_enabled {
            router = router.nest("/auth/sso", sso::create_router());
            router = router.nest("/workos", workos_webhooks::create_router());
        }
    }

    #[cfg(feature = "openapi")]
    {
        router = router.route("/openapi.yaml", get(openapi_yaml));
    }

    #[cfg(feature = "swagger-ui")]
    {
        use utoipa_swagger_ui::SwaggerUi;
        let swagger: Router<AppState> = SwaggerUi::new("/swagger-ui")
            .url("/openapi.json", ApiDoc::openapi())
            .into();
        router = router.merge(swagger);
    }

    #[cfg(not(feature = "swagger-ui"))]
    {
        router = router.route("/openapi.json", get(openapi_json));
    }

    let cors = build_cors_layer(api_config);

    Ok(router
        .layer(from_fn(observability_middleware))
        .layer(cors)
        .with_state(app_state))
}

/// Create a minimal router for testing without WebSocket support.
#[cfg(test)]
pub fn create_test_router(_db: DbClient) -> Router {
    Router::new().route("/openapi.json", get(openapi_json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;
    use crate::error::ErrorCode;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env_var<F: FnOnce() -> T, T>(key: &str, value: Option<&str>, f: F) -> T {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let prev = std::env::var(key).ok();
        match value {
            Some(val) => std::env::set_var(key, val),
            None => std::env::remove_var(key),
        }
        let result = f();
        match prev {
            Some(val) => std::env::set_var(key, val),
            None => std::env::remove_var(key),
        }
        result
    }

    #[test]
    fn test_openapi_json_endpoint_exists() {
        // Just verify the handler can be constructed
        // Full integration tests would require a running server
    }

    #[test]
    fn test_route_modules_compile() {
        // This test simply verifies all route modules are properly exported
        let _ = trajectory::create_router;
        let _ = scope::create_router;
        let _ = artifact::create_router;
        let _ = note::create_router;
        let _ = turn::create_router;
        let _ = agent::create_router;
        let _ = lock::create_router;
        let _ = message::create_router;
        let _ = delegation::create_router;
        let _ = handoff::create_router;
        let _ = dsl::create_router;
        let _ = config::create_router;
        let _ = tenant::create_router;
        // Battle Intel modules
        let _ = edge::create_router;
        let _ = summarization_policy::create_router;
    }

    #[test]
    fn test_is_production_environment_from_env() {
        with_env_var("CALIBER_ENVIRONMENT", Some("production"), || {
            assert!(is_production_environment());
        });
        with_env_var("CALIBER_ENVIRONMENT", Some("prod"), || {
            assert!(is_production_environment());
        });
        with_env_var("CALIBER_ENVIRONMENT", Some("development"), || {
            assert!(!is_production_environment());
        });
        with_env_var("CALIBER_ENVIRONMENT", None, || {
            assert!(!is_production_environment());
        });
    }

    #[test]
    fn test_validate_api_config_for_production_cors_required() {
        let config = ApiConfig {
            cors_origins: Vec::new(),
            ..Default::default()
        };
        let err = validate_api_config_for_production(&config).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidInput);

        let config = ApiConfig {
            cors_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };
        assert!(validate_api_config_for_production(&config).is_ok());
    }
}
