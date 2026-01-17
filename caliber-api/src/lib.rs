//! CALIBER API - REST/gRPC/WebSocket API Layer
//!
//! This crate provides a unified API layer for the CALIBER memory framework.
//! It exposes REST endpoints (Axum), gRPC services (Tonic), and WebSocket
//! connections for real-time event streaming.
//!
//! The API layer calls PostgreSQL functions from the caliber-pg extension,
//! which internally use direct heap operations for maximum performance.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod events;
pub mod grpc;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod telemetry;
pub mod types;
pub mod ws;

/// WorkOS SSO authentication module.
///
/// This module provides enterprise-grade authentication via WorkOS,
/// supporting SSO/SAML, SCIM, MFA, and multi-tenancy.
///
/// Enable with the `workos` feature flag.
#[cfg(feature = "workos")]
pub mod workos_auth;

// Re-export commonly used types
pub use auth::{
    authenticate, authenticate_api_key, authenticate_jwt, check_tenant_access, extract_tenant_id,
    generate_jwt_token, validate_api_key, validate_jwt_token, AuthConfig, AuthContext,
    AuthMethod, AuthProvider, Claims,
};

// Re-export WorkOS types when feature is enabled
#[cfg(feature = "workos")]
pub use workos_auth::{
    create_session_token, exchange_code_for_profile, generate_authorization_url,
    validate_workos_token, SsoAuthorizationParams, SsoCallbackParams, SsoCallbackResponse,
    WorkOsClaims, WorkOsConfig,
};
pub use config::ApiConfig;
pub use db::{DbClient, DbConfig, TenantInfo};
pub use error::{ApiError, ApiResult, ErrorCode};
pub use events::WsEvent;
pub use grpc::{create_services, proto};
pub use middleware::{
    auth_middleware, extract_auth_context, extract_auth_context_owned, rate_limit_middleware,
    tenant_access_middleware, AuthExtractor, AuthMiddlewareState, RateLimitKey, RateLimitState,
};
pub use openapi::ApiDoc;
pub use routes::create_api_router;
pub use telemetry::{init_tracer, metrics_handler, shutdown_tracer, CaliberMetrics, TelemetryConfig, METRICS};
pub use types::*;
pub use ws::{should_deliver_event, tenant_id_from_event, WsState};
