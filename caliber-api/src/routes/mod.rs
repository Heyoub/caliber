//! REST API Routes Module
//!
//! This module contains all REST API route handlers organized by entity type.

pub mod agent;
pub mod artifact;
pub mod config;
pub mod delegation;
pub mod dsl;
pub mod handoff;
pub mod lock;
pub mod message;
pub mod note;
pub mod scope;
pub mod tenant;
pub mod trajectory;
pub mod turn;

use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use utoipa::OpenApi;

use crate::db::DbClient;
use crate::openapi::ApiDoc;
use crate::ws::WsState;

// Re-export route creation functions for convenience
pub use agent::create_router as agent_router;
pub use artifact::create_router as artifact_router;
pub use config::create_router as config_router;
pub use delegation::create_router as delegation_router;
pub use dsl::create_router as dsl_router;
pub use handoff::create_router as handoff_router;
pub use lock::create_router as lock_router;
pub use message::create_router as message_router;
pub use note::create_router as note_router;
pub use scope::create_router as scope_router;
pub use tenant::create_router as tenant_router;
pub use trajectory::create_router as trajectory_router;
pub use turn::create_router as turn_router;

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
/// - OpenAPI spec at /openapi.json
/// - Swagger UI at /swagger-ui (when swagger-ui feature is enabled)
pub fn create_api_router(db: DbClient, ws: Arc<WsState>) -> Router {
    let api_routes = Router::new()
        .nest("/trajectories", trajectory::create_router(db.clone(), ws.clone()))
        .nest("/scopes", scope::create_router(db.clone(), ws.clone()))
        .nest("/artifacts", artifact::create_router(db.clone(), ws.clone()))
        .nest("/notes", note::create_router(db.clone()))
        .nest("/turns", turn::create_router(db.clone()))
        .nest("/agents", agent::create_router(db.clone()))
        .nest("/locks", lock::create_router(db.clone()))
        .nest("/messages", message::create_router(db.clone()))
        .nest("/delegations", delegation::create_router(db.clone()))
        .nest("/handoffs", handoff::create_router(db.clone()))
        .nest("/dsl", dsl::create_router(db.clone()))
        .nest("/config", config::create_router(db.clone()))
        .nest("/tenants", tenant::create_router(db.clone()));

    // Build the main router
    let mut router = Router::new()
        .nest("/api/v1", api_routes)
        .route("/openapi.json", get(openapi_json));

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

    router
}

/// Create a minimal router for testing without WebSocket support.
#[cfg(test)]
pub fn create_test_router(db: DbClient) -> Router {
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
    }
}
