//! Middleware modules for CALIBER API
//!
//! This module contains all Axum middleware implementations:
//!
//! - `auth`: Authentication and authorization middleware
//! - `rate_limit`: Rate limiting middleware (in auth module)
//! - `idempotency`: Idempotency key handling for safe retries
//!
//! # Middleware Order
//!
//! When applying middleware, order matters. The recommended order is:
//!
//! ```ignore
//! Router::new()
//!     .route("/api/v1/resource", post(handler))
//!     // Innermost (runs last on request, first on response)
//!     .layer(middleware::from_fn_with_state(idempotency_state, idempotency_middleware))
//!     // Auth must run before idempotency (provides tenant context)
//!     .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
//!     // Rate limiting runs first (before auth)
//!     .layer(middleware::from_fn_with_state(rate_limit_state, rate_limit_middleware))
//!     // Outermost
//! ```

mod auth;
pub mod idempotency;

// Re-export auth middleware types (backwards compatibility)
pub use auth::{
    auth_middleware, extract_auth_context, extract_auth_context_owned, rate_limit_middleware,
    tenant_access_middleware, AuthExtractor, AuthMiddlewareError, AuthMiddlewareState,
    RateLimitError, RateLimitKey, RateLimitState,
};

// Re-export idempotency types
pub use idempotency::{
    idempotency_middleware, IdempotencyConfig, IdempotencyError, IdempotencyState,
    IDEMPOTENCY_KEY_HEADER,
};
