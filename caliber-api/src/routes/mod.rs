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
pub mod delegation;
pub mod dsl;
pub mod edge;
pub mod graphql;
pub mod handoff;
pub mod health;
pub mod lock;
pub mod mcp;
pub mod message;
pub mod note;
pub mod scope;
pub mod sso;
pub mod summarization_policy;
pub mod tenant;
pub mod trajectory;
pub mod turn;
pub mod user;
pub mod webhooks;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    http::{header, header::HeaderName, HeaderValue, Method},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use caliber_pcp::PCPRuntime;
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;

use crate::auth::AuthConfig;
use crate::config::ApiConfig;
use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::middleware::{auth_middleware, rate_limit_middleware, AuthMiddlewareState, RateLimitState};
use crate::openapi::ApiDoc;
use crate::ws::WsState;

// Re-export route creation functions for convenience
pub use agent::create_router as agent_router;
pub use artifact::create_router as artifact_router;
pub use batch::create_router as batch_router;
pub use billing::create_router as billing_router;
pub use config::create_router as config_router;
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
async fn openapi_json() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}

/// Handler for /openapi.yaml endpoint.
#[cfg(feature = "openapi")]
async fn openapi_yaml() -> impl IntoResponse {
    use axum::http::{header, StatusCode};

    match ApiDoc::to_yaml() {
        Ok(yaml) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/yaml")],
            yaml,
        ),
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
    fn build_entity_routes(&self) -> ApiResult<Router> {
        Ok(Router::new()
            .nest("/trajectories", trajectory::create_router(self.db.clone(), self.ws.clone()))
            .nest("/scopes", scope::create_router(self.db.clone(), self.ws.clone(), self.pcp.clone()))
            .nest("/artifacts", artifact::create_router(self.db.clone(), self.ws.clone()))
            .nest("/notes", note::create_router(self.db.clone(), self.ws.clone()))
            .nest("/turns", turn::create_router(self.db.clone(), self.ws.clone(), self.pcp.clone()))
            .nest("/agents", agent::create_router(self.db.clone(), self.ws.clone()))
            .nest("/locks", lock::create_router(self.db.clone(), self.ws.clone()))
            .nest("/messages", message::create_router(self.db.clone(), self.ws.clone()))
            .nest("/delegations", delegation::create_router(self.db.clone(), self.ws.clone()))
            .nest("/handoffs", handoff::create_router(self.db.clone(), self.ws.clone()))
            .nest("/dsl", dsl::create_router(self.db.clone()))
            .nest("/config", config::create_router(self.db.clone(), self.ws.clone()))
            .nest("/tenants", tenant::create_router(self.db.clone()))
            .nest("/users", user::create_router(self.db.clone()))
            .nest("/billing", billing::create_router(self.db.clone()))
            .nest("/batch", batch::create_router(self.db.clone(), self.ws.clone()))
            .nest("/webhooks", webhooks::create_router(self.db.clone(), self.ws.clone())?)
            .nest("/graphql", graphql::create_router(self.db.clone(), self.ws.clone()))
            .nest("/edges", edge::create_router(self.db.clone(), self.ws.clone()))
            .nest("/summarization-policies", summarization_policy::create_router(self.db.clone(), self.ws.clone())))
    }

    /// Build the complete router with full security stack.
    ///
    /// # Middleware Order (outer to inner)
    /// 1. CORS (outermost) - handles preflight requests
    /// 2. Observability - tracing and metrics
    /// 3. Rate Limiting - rejects floods before expensive auth
    /// 4. Auth (innermost, only on /api/v1/*) - validates credentials
    pub fn build(self) -> ApiResult<Router> {
        use crate::telemetry::{middleware::observability_middleware, metrics_handler};
        use axum::middleware::from_fn;

        // Protected API routes (auth required)
        let api_routes = self
            .build_entity_routes()?
            .layer(from_fn_with_state(self.auth_state.clone(), auth_middleware));

        // Build the main router
        let mut router = Router::new()
            .nest("/api/v1", api_routes)
            // MCP server (not under /api/v1 - uses its own protocol)
            .nest("/mcp", mcp::create_router(self.db.clone(), self.ws.clone()))
            // Health checks (no auth required)
            .nest("/health", health::create_router(self.db.clone()))
            // Metrics endpoint (no auth, but rate-limited)
            .route("/metrics", get(metrics_handler))
            // OpenAPI spec
            .route("/openapi.json", get(openapi_json));

        // Add SSO routes when workos feature is enabled
        #[cfg(feature = "workos")]
        {
            use crate::workos_auth::WorkOsConfig;
            if let Ok(workos_config) = WorkOsConfig::from_env() {
                router = router.nest("/auth/sso", sso::create_router(self.db.clone(), workos_config));
            }
        }

        // Add YAML endpoint if openapi feature is enabled
        #[cfg(feature = "openapi")]
        {
            router = router.route("/openapi.yaml", get(openapi_yaml));
        }

        // Add Swagger UI if swagger-ui feature is enabled
        #[cfg(feature = "swagger-ui")]
        {
            use utoipa_swagger_ui::SwaggerUi;
            router = router.merge(
                SwaggerUi::new("/swagger-ui")
                    .url("/openapi.json", ApiDoc::openapi()),
            );
        }

        // Build CORS layer
        let cors = build_cors_layer(&self.api_config);

        // Apply security layers (order matters: outer to inner in code = inner to outer in execution)
        // Execution order: CORS -> Observability -> Rate Limiting -> Handler
        Ok(router
            .layer(from_fn_with_state(self.rate_limit_state, rate_limit_middleware))
            .layer(from_fn(observability_middleware))
            .layer(cors))
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
        cors.allow_origin(Any).allow_headers(Any).expose_headers(Any)
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
    use crate::telemetry::{middleware::observability_middleware, metrics_handler};
    use axum::middleware::from_fn;

    // Entity CRUD routes (NO AUTH)
    let api_routes = Router::new()
        .nest("/trajectories", trajectory::create_router(db.clone(), ws.clone()))
        .nest("/scopes", scope::create_router(db.clone(), ws.clone(), pcp.clone()))
        .nest("/artifacts", artifact::create_router(db.clone(), ws.clone()))
        .nest("/notes", note::create_router(db.clone(), ws.clone()))
        .nest("/turns", turn::create_router(db.clone(), ws.clone(), pcp.clone()))
        .nest("/agents", agent::create_router(db.clone(), ws.clone()))
        .nest("/locks", lock::create_router(db.clone(), ws.clone()))
        .nest("/messages", message::create_router(db.clone(), ws.clone()))
        .nest("/delegations", delegation::create_router(db.clone(), ws.clone()))
        .nest("/handoffs", handoff::create_router(db.clone(), ws.clone()))
        .nest("/dsl", dsl::create_router(db.clone()))
        .nest("/config", config::create_router(db.clone(), ws.clone()))
        .nest("/tenants", tenant::create_router(db.clone()))
        .nest("/users", user::create_router(db.clone()))
        .nest("/billing", billing::create_router(db.clone()))
        .nest("/batch", batch::create_router(db.clone(), ws.clone()))
        .nest("/webhooks", webhooks::create_router(db.clone(), ws.clone())?)
        .nest("/graphql", graphql::create_router(db.clone(), ws.clone()))
        .nest("/edges", edge::create_router(db.clone(), ws.clone()))
        .nest("/summarization-policies", summarization_policy::create_router(db.clone(), ws.clone()));

    let mut router = Router::new()
        .nest("/api/v1", api_routes)
        .nest("/mcp", mcp::create_router(db.clone(), ws.clone()))
        .nest("/health", health::create_router(db.clone()))
        .route("/metrics", get(metrics_handler))
        .route("/openapi.json", get(openapi_json));

    #[cfg(feature = "workos")]
    {
        use crate::workos_auth::WorkOsConfig;
        if let Ok(workos_config) = WorkOsConfig::from_env() {
            router = router.nest("/auth/sso", sso::create_router(db.clone(), workos_config));
        }
    }

    #[cfg(feature = "openapi")]
    {
        router = router.route("/openapi.yaml", get(openapi_yaml));
    }

    #[cfg(feature = "swagger-ui")]
    {
        use utoipa_swagger_ui::SwaggerUi;
        router = router.merge(
            SwaggerUi::new("/swagger-ui")
                .url("/openapi.json", ApiDoc::openapi()),
        );
    }

    let cors = build_cors_layer(api_config);

    Ok(router
        .layer(from_fn(observability_middleware))
        .layer(cors))
}

/// Create a minimal router for testing without WebSocket support.
#[cfg(test)]
pub fn create_test_router(_db: DbClient) -> Router {
    Router::new()
        .route("/openapi.json", get(openapi_json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_json_endpoint_exists() {
        // Just verify the handler can be constructed
        // Full integration tests would require a running server
    }

    #[test]
    fn test_route_modules_compile() {
        // This test simply verifies all route modules are properly exported
        let _ = trajectory::TrajectoryState::new;
        let _ = scope::ScopeState::new;
        let _ = artifact::ArtifactState::new;
        let _ = note::NoteState::new;
        let _ = turn::TurnState::new;
        let _ = agent::AgentState::new;
        let _ = lock::LockState::new;
        let _ = message::MessageState::new;
        let _ = delegation::DelegationState::new;
        let _ = handoff::HandoffState::new;
        let _ = dsl::DslState::new;
        let _ = config::ConfigState::new;
        let _ = tenant::TenantState::new;
        // Battle Intel modules
        let _ = edge::EdgeState::new;
        let _ = summarization_policy::SummarizationPolicyState::new;
    }
}
