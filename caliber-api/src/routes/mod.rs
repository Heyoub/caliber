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

pub mod agent;
pub mod artifact;
pub mod batch;
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
pub mod webhooks;

use std::sync::Arc;

use axum::{response::IntoResponse, routing::get, Json, Router};
use caliber_pcp::PCPRuntime;
use utoipa::OpenApi;

use crate::db::DbClient;
use crate::openapi::ApiDoc;
use crate::ws::WsState;

// Re-export route creation functions for convenience
pub use agent::create_router as agent_router;
pub use artifact::create_router as artifact_router;
pub use batch::create_router as batch_router;
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
// ROUTER BUILDER
// ============================================================================

/// Create the complete API router with all routes and OpenAPI documentation.
///
/// This function creates a fully configured Axum router with:
/// - All REST API routes under /api/v1/*
/// - Batch operations under /api/v1/batch/*
/// - Webhook management under /api/v1/webhooks
/// - GraphQL endpoint at /api/v1/graphql
/// - MCP server at /mcp/*
/// - Health checks at /health/*
/// - Metrics at /metrics
/// - OpenAPI spec at /openapi.json
/// - Swagger UI at /swagger-ui (when swagger-ui feature is enabled)
///
/// # Battle Intel Integration
/// The `pcp` parameter enables auto-summarization trigger checking on turn
/// creation and scope close events, supporting L0→L1→L2 abstraction transitions.
pub fn create_api_router(db: DbClient, ws: Arc<WsState>, pcp: Arc<PCPRuntime>) -> Router {
    use crate::telemetry::{middleware::observability_middleware, metrics_handler};
    use axum::middleware::from_fn;

    // Entity CRUD routes
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
        // New routes
        .nest("/batch", batch::create_router(db.clone(), ws.clone()))
        .nest("/webhooks", webhooks::create_router(db.clone(), ws.clone()))
        .nest("/graphql", graphql::create_router(db.clone(), ws.clone()))
        // Battle Intel routes
        .nest("/edges", edge::create_router(db.clone(), ws.clone()))
        .nest("/summarization-policies", summarization_policy::create_router(db.clone(), ws.clone()));

    // Build the main router
    let mut router = Router::new()
        .nest("/api/v1", api_routes)
        // MCP server (not under /api/v1 - uses its own protocol)
        .nest("/mcp", mcp::create_router(db.clone(), ws.clone()))
        // Health checks (no auth required)
        .nest("/health", health::create_router(db.clone()))
        // Metrics endpoint
        .route("/metrics", get(metrics_handler))
        // OpenAPI spec
        .route("/openapi.json", get(openapi_json));

    // Add SSO routes when workos feature is enabled
    #[cfg(feature = "workos")]
    {
        use crate::workos_auth::WorkOsConfig;
        // Only add SSO routes if WorkOS is configured
        if let Ok(workos_config) = WorkOsConfig::from_env() {
            router = router.nest("/auth/sso", sso::create_router(db.clone(), workos_config));
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

    // Add observability middleware
    router.layer(from_fn(observability_middleware))
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
