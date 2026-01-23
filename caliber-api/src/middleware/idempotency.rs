//! Idempotency Middleware for CALIBER API
//!
//! This module provides idempotency key support for POST/PUT/PATCH/DELETE requests.
//! It ensures that retried requests with the same idempotency key return the same
//! response, preventing duplicate operations.
//!
//! # Usage
//!
//! Clients should include an `Idempotency-Key` header with a unique value (typically
//! a UUID) on mutating requests. The server will:
//!
//! 1. Check if the key exists with matching request hash
//! 2. If exists and matches: return cached response
//! 3. If exists but hash differs: return 409 Conflict
//! 4. If new: execute request and cache response
//!
//! # Example
//!
//! ```ignore
//! use axum::{Router, middleware};
//! use caliber_api::middleware::idempotency::{idempotency_middleware, IdempotencyState};
//!
//! let state = IdempotencyState::new(db_client);
//!
//! let app = Router::new()
//!     .route("/api/v1/delegations", axum::routing::post(create_delegation))
//!     .layer(middleware::from_fn_with_state(state.clone(), idempotency_middleware));
//! ```

use crate::auth::AuthContext;
use crate::db::DbClient;
use crate::error::{ApiError, ErrorCode};
use axum::{
    body::{Body, Bytes},
    extract::{Request, State},
    http::{header::HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio_postgres::types::ToSql;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Header name for idempotency key
pub const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

/// Default TTL for idempotency keys (24 hours)
pub const DEFAULT_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Maximum size of request body to hash (1MB)
/// Larger bodies are truncated for hashing purposes
pub const MAX_BODY_HASH_SIZE: usize = 1024 * 1024;

// ============================================================================
// STATE
// ============================================================================

/// Configuration for idempotency middleware.
#[derive(Debug, Clone)]
pub struct IdempotencyConfig {
    /// Time-to-live for idempotency keys
    pub ttl: Duration,

    /// Maximum body size to consider for hashing
    pub max_body_size: usize,

    /// Whether to require idempotency keys on mutating requests
    pub require_key: bool,
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self {
            ttl: DEFAULT_TTL,
            max_body_size: MAX_BODY_HASH_SIZE,
            require_key: false, // Optional by default
        }
    }
}

/// Shared state for idempotency middleware.
#[derive(Clone)]
pub struct IdempotencyState {
    /// Database client for storing/retrieving idempotency keys
    pub db: Arc<DbClient>,

    /// Configuration
    pub config: IdempotencyConfig,
}

impl IdempotencyState {
    /// Create new idempotency state with the given database client.
    pub fn new(db: Arc<DbClient>) -> Self {
        Self {
            db,
            config: IdempotencyConfig::default(),
        }
    }

    /// Create idempotency state with custom configuration.
    pub fn with_config(db: Arc<DbClient>, config: IdempotencyConfig) -> Self {
        Self { db, config }
    }
}

// ============================================================================
// MIDDLEWARE
// ============================================================================

/// Axum middleware for idempotency key handling.
///
/// This middleware intercepts mutating requests (POST, PUT, PATCH, DELETE) that
/// include an `Idempotency-Key` header and ensures idempotent behavior:
///
/// - If the key exists with matching request hash, return cached response
/// - If the key exists with different hash, return 409 Conflict
/// - If the key is new, execute request and cache response
///
/// GET and HEAD requests are passed through unchanged.
pub async fn idempotency_middleware(
    State(state): State<IdempotencyState>,
    request: Request,
    next: Next,
) -> Result<Response, IdempotencyError> {
    // Only process mutating methods
    let method = request.method().clone();
    if !is_mutating_method(&method) {
        return Ok(next.run(request).await);
    }

    // Extract idempotency key from header
    let idempotency_key = request
        .headers()
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // If no key provided
    let idempotency_key = match idempotency_key {
        Some(key) => {
            // Validate key format (should be non-empty, reasonable length)
            if key.is_empty() || key.len() > 256 {
                return Err(IdempotencyError::InvalidKey(
                    "Idempotency key must be 1-256 characters".to_string(),
                ));
            }
            key
        }
        None => {
            if state.config.require_key {
                return Err(IdempotencyError::MissingKey);
            }
            // No key provided and not required - pass through
            return Ok(next.run(request).await);
        }
    };

    // Extract tenant ID from auth context
    let tenant_id = request
        .extensions()
        .get::<AuthContext>()
        .map(|ctx| ctx.tenant_id)
        .ok_or_else(|| {
            IdempotencyError::Internal("Auth context missing, ensure auth middleware runs first".to_string())
        })?;

    // Extract operation name from path
    let operation = extract_operation_name(request.uri().path(), &method);

    // Buffer the request body for hashing
    let (parts, body) = request.into_parts();
    let body_bytes = axum::body::to_bytes(body, state.config.max_body_size)
        .await
        .map_err(|e| IdempotencyError::Internal(format!("Failed to read request body: {}", e)))?;

    // Compute request hash (method + path + body)
    let request_hash = compute_request_hash(&method, parts.uri.path(), &body_bytes);

    // Check idempotency key in database
    let ttl_interval = format!("{} seconds", state.config.ttl.as_secs());

    let check_result = check_idempotency_key(
        &state.db,
        &idempotency_key,
        tenant_id,
        &operation,
        &request_hash,
        &ttl_interval,
    )
    .await?;

    match check_result {
        IdempotencyCheckResult::Cached { status, body } => {
            // Return cached response
            tracing::debug!(
                idempotency_key = %idempotency_key,
                "Returning cached response for idempotency key"
            );

            let status_code = StatusCode::from_u16(status as u16)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

            let response = if let Some(body_json) = body {
                let body_str = serde_json::to_string(&body_json)
                    .map_err(|e| IdempotencyError::Internal(format!("Failed to serialize cached response: {}", e)))?;

                Response::builder()
                    .status(status_code)
                    .header("content-type", "application/json")
                    .header("x-idempotency-replay", "true")
                    .body(Body::from(body_str))
                    .map_err(|e| IdempotencyError::Internal(format!("Failed to build response: {}", e)))?
            } else {
                Response::builder()
                    .status(status_code)
                    .header("x-idempotency-replay", "true")
                    .body(Body::empty())
                    .map_err(|e| IdempotencyError::Internal(format!("Failed to build response: {}", e)))?
            };

            Ok(response)
        }
        IdempotencyCheckResult::New => {
            // Execute the request
            let request = Request::from_parts(parts, Body::from(body_bytes));
            let response = next.run(request).await;

            // Extract and cache the response
            let (resp_parts, resp_body) = response.into_parts();
            let resp_bytes = axum::body::to_bytes(resp_body, state.config.max_body_size)
                .await
                .unwrap_or_default();

            let status = resp_parts.status.as_u16() as i32;
            let body_json: Option<serde_json::Value> = serde_json::from_slice(&resp_bytes).ok();

            // Store the response (best effort - don't fail the request if storage fails)
            if let Err(e) = store_idempotency_response(
                &state.db,
                &idempotency_key,
                tenant_id,
                status,
                body_json.as_ref(),
            )
            .await
            {
                tracing::warn!(
                    error = %e,
                    idempotency_key = %idempotency_key,
                    "Failed to store idempotency response"
                );
            }

            // Reconstruct and return the response
            let response = Response::from_parts(resp_parts, Body::from(resp_bytes));
            Ok(response)
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Check if the HTTP method is a mutating operation.
fn is_mutating_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// Extract a meaningful operation name from the request path and method.
fn extract_operation_name(path: &str, method: &Method) -> String {
    // Extract the last path segment as the resource name
    let resource = path
        .trim_end_matches('/')
        .rsplit('/')
        .find(|s| !s.is_empty() && !uuid::Uuid::parse_str(s).is_ok())
        .unwrap_or("unknown");

    format!("{}_{}", method.as_str().to_lowercase(), resource)
}

/// Compute SHA-256 hash of method + path + body.
fn compute_request_hash(method: &Method, path: &str, body: &Bytes) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(method.as_str().as_bytes());
    hasher.update(b"|");
    hasher.update(path.as_bytes());
    hasher.update(b"|");
    hasher.update(body);
    hasher.finalize().to_vec()
}

/// Result of checking an idempotency key.
enum IdempotencyCheckResult {
    /// Key exists with cached response
    Cached {
        status: i32,
        body: Option<serde_json::Value>,
    },
    /// Key is new, proceed with request
    New,
}

/// Check idempotency key in database.
async fn check_idempotency_key(
    db: &DbClient,
    idempotency_key: &str,
    tenant_id: uuid::Uuid,
    operation: &str,
    request_hash: &[u8],
    ttl_interval: &str,
) -> Result<IdempotencyCheckResult, IdempotencyError> {
    let client = db.pool.get().await.map_err(|e| {
        IdempotencyError::Internal(format!("Failed to get database connection: {}", e))
    })?;

    // Note: Using raw SQL here because the function has special error handling
    // The function raises an exception on hash mismatch (conflict)
    let result = client
        .query(
            "SELECT key_exists, cached_status, cached_body FROM caliber_check_idempotency_key($1, $2, $3, $4, $5::interval)",
            &[
                &idempotency_key,
                &tenant_id,
                &operation,
                &request_hash,
                &ttl_interval,
            ],
        )
        .await;

    match result {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                let key_exists: bool = row.get(0);
                if key_exists {
                    let cached_status: i32 = row.get(1);
                    let cached_body: Option<serde_json::Value> = row.get(2);
                    Ok(IdempotencyCheckResult::Cached {
                        status: cached_status,
                        body: cached_body,
                    })
                } else {
                    Ok(IdempotencyCheckResult::New)
                }
            } else {
                Ok(IdempotencyCheckResult::New)
            }
        }
        Err(e) => {
            // Check for unique_violation (hash mismatch / conflict)
            if let Some(db_error) = e.as_db_error() {
                if db_error.code() == &tokio_postgres::error::SqlState::UNIQUE_VIOLATION {
                    return Err(IdempotencyError::Conflict(idempotency_key.to_string()));
                }
            }
            Err(IdempotencyError::Internal(format!(
                "Failed to check idempotency key: {}",
                e
            )))
        }
    }
}

/// Store the response for an idempotency key.
async fn store_idempotency_response(
    db: &DbClient,
    idempotency_key: &str,
    tenant_id: uuid::Uuid,
    status: i32,
    body: Option<&serde_json::Value>,
) -> Result<(), IdempotencyError> {
    let client = db.pool.get().await.map_err(|e| {
        IdempotencyError::Internal(format!("Failed to get database connection: {}", e))
    })?;

    client
        .execute(
            "SELECT caliber_store_idempotency_response($1, $2, $3, $4)",
            &[&idempotency_key, &tenant_id, &status, &body],
        )
        .await
        .map_err(|e| {
            IdempotencyError::Internal(format!("Failed to store idempotency response: {}", e))
        })?;

    Ok(())
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

/// Errors that can occur in idempotency middleware.
#[derive(Debug)]
pub enum IdempotencyError {
    /// Idempotency key is required but not provided
    MissingKey,

    /// Idempotency key format is invalid
    InvalidKey(String),

    /// Key exists but request hash doesn't match (different request)
    Conflict(String),

    /// Internal error (database, serialization, etc.)
    Internal(String),
}

impl IntoResponse for IdempotencyError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            IdempotencyError::MissingKey => (
                StatusCode::BAD_REQUEST,
                ApiError::new(
                    ErrorCode::MissingField,
                    format!("Header '{}' is required for this operation", IDEMPOTENCY_KEY_HEADER),
                ),
            ),
            IdempotencyError::InvalidKey(msg) => (
                StatusCode::BAD_REQUEST,
                ApiError::new(ErrorCode::InvalidFormat, msg),
            ),
            IdempotencyError::Conflict(key) => (
                StatusCode::CONFLICT,
                ApiError::new(
                    ErrorCode::StateConflict,
                    format!(
                        "Idempotency key '{}' was already used with a different request",
                        key
                    ),
                ),
            ),
            IdempotencyError::Internal(msg) => {
                tracing::error!(error = %msg, "Idempotency middleware internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiError::new(ErrorCode::InternalError, "Internal server error"),
                )
            }
        };

        (status, axum::Json(error)).into_response()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_mutating_method() {
        assert!(is_mutating_method(&Method::POST));
        assert!(is_mutating_method(&Method::PUT));
        assert!(is_mutating_method(&Method::PATCH));
        assert!(is_mutating_method(&Method::DELETE));
        assert!(!is_mutating_method(&Method::GET));
        assert!(!is_mutating_method(&Method::HEAD));
        assert!(!is_mutating_method(&Method::OPTIONS));
    }

    #[test]
    fn test_extract_operation_name() {
        assert_eq!(
            extract_operation_name("/api/v1/delegations", &Method::POST),
            "post_delegations"
        );
        assert_eq!(
            extract_operation_name("/api/v1/delegations/", &Method::POST),
            "post_delegations"
        );
        assert_eq!(
            extract_operation_name(
                "/api/v1/delegations/123e4567-e89b-12d3-a456-426614174000",
                &Method::PUT
            ),
            "put_delegations"
        );
        assert_eq!(
            extract_operation_name("/api/v1/unknown", &Method::DELETE),
            "delete_unknown"
        );
    }

    #[test]
    fn test_compute_request_hash_deterministic() {
        let body = Bytes::from(r#"{"name": "test"}"#);
        let hash1 = compute_request_hash(&Method::POST, "/api/v1/test", &body);
        let hash2 = compute_request_hash(&Method::POST, "/api/v1/test", &body);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_request_hash_different_for_different_inputs() {
        let body = Bytes::from(r#"{"name": "test"}"#);

        let hash1 = compute_request_hash(&Method::POST, "/api/v1/test", &body);
        let hash2 = compute_request_hash(&Method::PUT, "/api/v1/test", &body);
        assert_ne!(hash1, hash2);

        let hash3 = compute_request_hash(&Method::POST, "/api/v1/other", &body);
        assert_ne!(hash1, hash3);

        let other_body = Bytes::from(r#"{"name": "other"}"#);
        let hash4 = compute_request_hash(&Method::POST, "/api/v1/test", &other_body);
        assert_ne!(hash1, hash4);
    }

    #[test]
    fn test_idempotency_config_default() {
        let config = IdempotencyConfig::default();
        assert_eq!(config.ttl, DEFAULT_TTL);
        assert_eq!(config.max_body_size, MAX_BODY_HASH_SIZE);
        assert!(!config.require_key);
    }
}
