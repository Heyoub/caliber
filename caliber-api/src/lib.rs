//! CALIBER API - REST/gRPC/WebSocket API Layer
//!
//! This crate provides a unified API layer for the CALIBER memory framework.
//! It exposes REST endpoints (Axum), gRPC services (Tonic), and WebSocket
//! connections for real-time event streaming.
//!
//! The API layer calls PostgreSQL functions from the caliber-pg extension,
//! which internally use direct heap operations for maximum performance.

pub mod auth;
pub mod cached_db;
pub mod component;
pub mod components;
pub mod config;
pub mod constants;
pub mod db;
pub mod db_helpers;
pub mod error;
pub mod events;
pub mod extractors;
pub mod grpc;
pub mod jobs;
pub mod macros;
pub mod middleware;
pub mod openapi;
pub mod providers;
pub mod routes;
pub mod services;
pub mod state;
pub mod telemetry;
pub mod traits;
pub mod types;
pub mod validation;
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
    generate_jwt_token, validate_api_key, validate_jwt_token, AuthConfig, AuthContext, AuthMethod,
    AuthProvider, Claims,
};

// Re-export WorkOS types when feature is enabled
pub use cached_db::{CacheImpl, CachedDbClient};
pub use config::ApiConfig;
pub use db::{DbClient, DbConfig};
pub use error::{ApiError, ApiResult, ErrorCode};
pub use events::WsEvent;
pub use grpc::{create_services, proto};
pub use middleware::{
    auth_middleware,
    extract_auth_context,
    extract_auth_context_owned,
    // Idempotency middleware (V3: Distributed Correctness)
    idempotency_middleware,
    rate_limit_middleware,
    tenant_access_middleware,
    AuthExtractor,
    AuthMiddlewareState,
    IdempotencyConfig,
    IdempotencyState,
    RateLimitKey,
    RateLimitState,
    IDEMPOTENCY_KEY_HEADER,
};
#[cfg(feature = "workos")]
pub use workos_auth::{
    create_session_token, exchange_code_for_profile, generate_authorization_url,
    validate_workos_token, SsoAuthorizationParams, SsoCallbackParams, SsoCallbackResponse,
    WorkOsClaims, WorkOsConfig,
};

// Background jobs (V3: Distributed Correctness)
pub use jobs::{saga_cleanup_task, SagaCleanupConfig, SagaCleanupMetrics};
pub use openapi::ApiDoc;
pub use routes::create_api_router;
pub use state::{ApiCache, ApiEventDag, AppState};
pub use telemetry::{
    init_tracer, metrics_handler, shutdown_tracer, CaliberMetrics, TelemetryConfig, METRICS,
};
pub use types::*;
pub use ws::{should_deliver_event, tenant_id_from_event, WsState};

// Re-export ECS component types
pub use component::{Component, ListFilter, Listable, NoFilter, SqlParam, TenantScoped};
pub use components::{ScopeListFilter, TrajectoryListFilter};

// Re-export custom extractors
pub use extractors::{PathId, PathIdError, PathIds};
