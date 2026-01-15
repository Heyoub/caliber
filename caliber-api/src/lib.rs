//! CALIBER API - REST/gRPC/WebSocket API Layer
//!
//! This crate provides a unified API layer for the CALIBER memory framework.
//! It exposes REST endpoints (Axum), gRPC services (Tonic), and WebSocket
//! connections for real-time event streaming.
//!
//! The API layer calls PostgreSQL functions from the caliber-pg extension,
//! which internally use direct heap operations for maximum performance.

pub mod auth;
pub mod db;
pub mod error;
pub mod events;
pub mod grpc;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod types;
pub mod ws;

// Re-export commonly used types
pub use auth::{
    authenticate, authenticate_api_key, authenticate_jwt, check_tenant_access, extract_tenant_id,
    generate_jwt_token, validate_api_key, validate_jwt_token, AuthConfig, AuthContext,
    AuthMethod, Claims,
};
pub use db::{DbClient, DbConfig};
pub use error::{ApiError, ApiResult, ErrorCode};
pub use events::WsEvent;
pub use grpc::{create_services, proto};
pub use middleware::{
    auth_middleware, extract_auth_context, extract_auth_context_owned, tenant_access_middleware,
    AuthMiddlewareState,
};
pub use openapi::ApiDoc;
pub use types::*;
pub use ws::WsState;
