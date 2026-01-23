//! Error Types for CALIBER API
//!
//! This module defines error handling for the API layer, including:
//! - ApiError struct for structured error responses
//! - ErrorCode enum for categorizing errors
//! - IntoResponse implementation for Axum HTTP responses
//!
//! All errors are serialized as JSON with appropriate HTTP status codes.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// ERROR CODE ENUM
// ============================================================================

/// Error codes for API responses.
///
/// Each error code maps to a specific HTTP status code and represents
/// a category of error that can occur during API operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // ========================================================================
    // Authentication Errors (401, 403)
    // ========================================================================
    /// Request lacks valid authentication credentials
    Unauthorized,
    
    /// Request is authenticated but lacks permission for the resource
    Forbidden,
    
    /// Authentication token is invalid or malformed
    InvalidToken,
    
    /// Authentication token has expired
    TokenExpired,
    
    // ========================================================================
    // Validation Errors (400)
    // ========================================================================
    /// Request validation failed
    ValidationFailed,
    
    /// Request contains invalid input data
    InvalidInput,
    
    /// Required field is missing from request
    MissingField,
    
    /// Field value is out of valid range
    InvalidRange,
    
    /// Field format is incorrect
    InvalidFormat,
    
    // ========================================================================
    // Not Found Errors (404)
    // ========================================================================
    /// Requested entity does not exist
    EntityNotFound,
    
    /// Requested tenant does not exist
    TenantNotFound,
    
    /// Requested trajectory does not exist
    TrajectoryNotFound,
    
    /// Requested scope does not exist
    ScopeNotFound,
    
    /// Requested artifact does not exist
    ArtifactNotFound,
    
    /// Requested note does not exist
    NoteNotFound,
    
    /// Requested agent does not exist
    AgentNotFound,
    
    /// Requested lock does not exist
    LockNotFound,
    
    /// Requested message does not exist
    MessageNotFound,

    /// Requested API key does not exist
    ApiKeyNotFound,

    /// Requested webhook does not exist
    WebhookNotFound,
    
    // ========================================================================
    // Conflict Errors (409)
    // ========================================================================
    /// Entity with the same identifier already exists
    EntityAlreadyExists,
    
    /// Concurrent modification detected (optimistic locking failure)
    ConcurrentModification,
    
    /// Lock conflict - resource is already locked
    LockConflict,
    
    /// Lock has expired
    LockExpired,
    
    /// Operation conflicts with current state
    StateConflict,
    
    // ========================================================================
    // Server Errors (500, 503)
    // ========================================================================
    /// Internal server error
    InternalError,
    
    /// Database operation failed
    DatabaseError,
    
    /// Service is temporarily unavailable
    ServiceUnavailable,
    
    /// Database connection pool exhausted
    ConnectionPoolExhausted,

    /// Operation timed out
    Timeout,

    /// Request rate limit exceeded
    TooManyRequests,
}

impl ErrorCode {
    /// Get the HTTP status code for this error code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            // Authentication errors
            ErrorCode::Unauthorized
            | ErrorCode::InvalidToken
            | ErrorCode::TokenExpired => StatusCode::UNAUTHORIZED,
            
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            
            // Validation errors
            ErrorCode::ValidationFailed
            | ErrorCode::InvalidInput
            | ErrorCode::MissingField
            | ErrorCode::InvalidRange
            | ErrorCode::InvalidFormat => StatusCode::BAD_REQUEST,
            
            // Not found errors
            ErrorCode::EntityNotFound
            | ErrorCode::TenantNotFound
            | ErrorCode::TrajectoryNotFound
            | ErrorCode::ScopeNotFound
            | ErrorCode::ArtifactNotFound
            | ErrorCode::NoteNotFound
            | ErrorCode::AgentNotFound
            | ErrorCode::LockNotFound
            | ErrorCode::MessageNotFound
            | ErrorCode::ApiKeyNotFound
            | ErrorCode::WebhookNotFound => StatusCode::NOT_FOUND,
            
            // Conflict errors
            ErrorCode::EntityAlreadyExists
            | ErrorCode::ConcurrentModification
            | ErrorCode::LockConflict
            | ErrorCode::LockExpired
            | ErrorCode::StateConflict => StatusCode::CONFLICT,
            
            // Server errors
            ErrorCode::ServiceUnavailable
            | ErrorCode::ConnectionPoolExhausted => StatusCode::SERVICE_UNAVAILABLE,
            
            ErrorCode::Timeout => StatusCode::GATEWAY_TIMEOUT,

            ErrorCode::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,

            ErrorCode::InternalError
            | ErrorCode::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    
    /// Get a default message for this error code.
    pub fn default_message(&self) -> &'static str {
        match self {
            // Authentication
            ErrorCode::Unauthorized => "Authentication required",
            ErrorCode::Forbidden => "Access forbidden",
            ErrorCode::InvalidToken => "Invalid authentication token",
            ErrorCode::TokenExpired => "Authentication token has expired",
            
            // Validation
            ErrorCode::ValidationFailed => "Request validation failed",
            ErrorCode::InvalidInput => "Invalid input data",
            ErrorCode::MissingField => "Required field is missing",
            ErrorCode::InvalidRange => "Value is out of valid range",
            ErrorCode::InvalidFormat => "Invalid format",
            
            // Not Found
            ErrorCode::EntityNotFound => "Entity not found",
            ErrorCode::TenantNotFound => "Tenant not found",
            ErrorCode::TrajectoryNotFound => "Trajectory not found",
            ErrorCode::ScopeNotFound => "Scope not found",
            ErrorCode::ArtifactNotFound => "Artifact not found",
            ErrorCode::NoteNotFound => "Note not found",
            ErrorCode::AgentNotFound => "Agent not found",
            ErrorCode::LockNotFound => "Lock not found",
            ErrorCode::MessageNotFound => "Message not found",
            ErrorCode::ApiKeyNotFound => "API key not found",
            ErrorCode::WebhookNotFound => "Webhook not found",
            
            // Conflict
            ErrorCode::EntityAlreadyExists => "Entity already exists",
            ErrorCode::ConcurrentModification => "Concurrent modification detected",
            ErrorCode::LockConflict => "Resource is locked by another agent",
            ErrorCode::LockExpired => "Lock has expired",
            ErrorCode::StateConflict => "Operation conflicts with current state",
            
            // Server
            ErrorCode::InternalError => "Internal server error",
            ErrorCode::DatabaseError => "Database operation failed",
            ErrorCode::ServiceUnavailable => "Service temporarily unavailable",
            ErrorCode::ConnectionPoolExhausted => "Connection pool exhausted",
            ErrorCode::Timeout => "Operation timed out",
            ErrorCode::TooManyRequests => "Rate limit exceeded",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ============================================================================
// API ERROR STRUCT
// ============================================================================

/// Structured error response for API operations.
///
/// This type is returned by all API endpoints when an error occurs.
/// It provides a consistent error format across REST, gRPC, and WebSocket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ApiError {
    /// Error code categorizing the error
    pub code: ErrorCode,

    /// Human-readable error message
    pub message: String,

    /// Optional additional details (field errors, stack traces, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    /// Create a new API error with the given code and message.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }
    
    /// Create a new API error with the given code, using the default message.
    pub fn from_code(code: ErrorCode) -> Self {
        Self {
            code,
            message: code.default_message().to_string(),
            details: None,
        }
    }
    
    /// Add additional details to the error.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
    
    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        self.code.status_code()
    }
    
    // ========================================================================
    // Convenience constructors for common errors
    // ========================================================================
    
    /// Create an Unauthorized error.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }
    
    /// Create a Forbidden error.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }
    
    /// Create an InvalidToken error.
    pub fn invalid_token(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidToken, message)
    }
    
    /// Create a TokenExpired error.
    pub fn token_expired() -> Self {
        Self::from_code(ErrorCode::TokenExpired)
    }
    
    /// Create a ValidationFailed error.
    pub fn validation_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationFailed, message)
    }
    
    /// Create an InvalidInput error.
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }
    
    /// Create a MissingField error.
    pub fn missing_field(field: &str) -> Self {
        Self::new(
            ErrorCode::MissingField,
            format!("Required field '{}' is missing", field),
        )
    }
    
    /// Create an InvalidRange error.
    pub fn invalid_range(field: &str, min: impl fmt::Display, max: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::InvalidRange,
            format!("Field '{}' must be between {} and {}", field, min, max),
        )
    }
    
    /// Create an InvalidFormat error.
    pub fn invalid_format(field: &str, expected: &str) -> Self {
        Self::new(
            ErrorCode::InvalidFormat,
            format!("Field '{}' has invalid format, expected {}", field, expected),
        )
    }
    
    /// Create an EntityNotFound error.
    pub fn entity_not_found(entity_type: &str, id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::EntityNotFound,
            format!("{} with id {} not found", entity_type, id),
        )
    }

    /// Create a generic not found error with custom message.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::EntityNotFound, message)
    }
    
    /// Create a TenantNotFound error.
    pub fn tenant_not_found(tenant_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::TenantNotFound,
            format!("Tenant {} not found", tenant_id),
        )
    }
    
    /// Create a TrajectoryNotFound error.
    pub fn trajectory_not_found(trajectory_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::TrajectoryNotFound,
            format!("Trajectory {} not found", trajectory_id),
        )
    }
    
    /// Create a ScopeNotFound error.
    pub fn scope_not_found(scope_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::ScopeNotFound,
            format!("Scope {} not found", scope_id),
        )
    }
    
    /// Create an ArtifactNotFound error.
    pub fn artifact_not_found(artifact_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::ArtifactNotFound,
            format!("Artifact {} not found", artifact_id),
        )
    }
    
    /// Create a NoteNotFound error.
    pub fn note_not_found(note_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::NoteNotFound,
            format!("Note {} not found", note_id),
        )
    }
    
    /// Create an AgentNotFound error.
    pub fn agent_not_found(agent_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::AgentNotFound,
            format!("Agent {} not found", agent_id),
        )
    }
    
    /// Create a LockNotFound error.
    pub fn lock_not_found(lock_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::LockNotFound,
            format!("Lock {} not found", lock_id),
        )
    }
    
    /// Create a MessageNotFound error.
    pub fn message_not_found(message_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::MessageNotFound,
            format!("Message {} not found", message_id),
        )
    }

    /// Create an ApiKeyNotFound error.
    pub fn api_key_not_found(api_key_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::ApiKeyNotFound,
            format!("API key {} not found", api_key_id),
        )
    }

    /// Create a WebhookNotFound error.
    pub fn webhook_not_found(webhook_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::WebhookNotFound,
            format!("Webhook {} not found", webhook_id),
        )
    }
    
    /// Create an EntityAlreadyExists error.
    pub fn entity_already_exists(entity_type: &str, id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::EntityAlreadyExists,
            format!("{} with id {} already exists", entity_type, id),
        )
    }
    
    /// Create a ConcurrentModification error.
    pub fn concurrent_modification(entity_type: &str, id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::ConcurrentModification,
            format!("{} {} was modified by another request", entity_type, id),
        )
    }
    
    /// Create a LockConflict error.
    pub fn lock_conflict(resource_type: &str, resource_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::LockConflict,
            format!("{} {} is locked by another agent", resource_type, resource_id),
        )
    }
    
    /// Create a LockExpired error.
    pub fn lock_expired(lock_id: impl fmt::Display) -> Self {
        Self::new(
            ErrorCode::LockExpired,
            format!("Lock {} has expired", lock_id),
        )
    }
    
    /// Create a StateConflict error.
    pub fn state_conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::StateConflict, message)
    }
    
    /// Create an InternalError.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }
    
    /// Create a DatabaseError.
    pub fn database_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DatabaseError, message)
    }
    
    /// Create a ServiceUnavailable error.
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ServiceUnavailable, message)
    }
    
    /// Create a ConnectionPoolExhausted error.
    pub fn connection_pool_exhausted() -> Self {
        Self::from_code(ErrorCode::ConnectionPoolExhausted)
    }
    
    /// Create a Timeout error.
    pub fn timeout(operation: &str) -> Self {
        Self::new(
            ErrorCode::Timeout,
            format!("Operation '{}' timed out", operation),
        )
    }

    /// Create a TooManyRequests error.
    pub fn too_many_requests(retry_after_secs: Option<u64>) -> Self {
        let message = match retry_after_secs {
            Some(secs) => format!("Rate limit exceeded. Retry after {} seconds", secs),
            None => "Rate limit exceeded".to_string(),
        };
        Self::new(ErrorCode::TooManyRequests, message)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

// ============================================================================
// AXUM INTEGRATION
// ============================================================================

/// Implement IntoResponse for ApiError to enable automatic error handling in Axum.
///
/// This allows ApiError to be returned directly from Axum handlers:
/// ```ignore
/// async fn handler() -> Result<Json<Response>, ApiError> {
///     Err(ApiError::unauthorized("Invalid credentials"))
/// }
/// ```
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(self);
        (status, body).into_response()
    }
}

// ============================================================================
// CONVERSIONS FROM STANDARD ERRORS
// ============================================================================

/// Convert from tokio_postgres::Error to ApiError.
impl From<tokio_postgres::Error> for ApiError {
    fn from(err: tokio_postgres::Error) -> Self {
        // Log the full error for debugging
        tracing::error!("Database error: {:?}", err);
        
        // Return a generic database error to avoid leaking internal details
        ApiError::database_error("Database operation failed")
    }
}

/// Convert from deadpool_postgres::PoolError to ApiError.
impl From<deadpool_postgres::PoolError> for ApiError {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        tracing::error!("Connection pool error: {:?}", err);
        
        match err {
            deadpool_postgres::PoolError::Timeout(_) => {
                ApiError::connection_pool_exhausted()
            }
            deadpool_postgres::PoolError::Closed => {
                ApiError::service_unavailable("Database connection pool is closed")
            }
            _ => ApiError::database_error("Failed to acquire database connection"),
        }
    }
}

/// Convert from serde_json::Error to ApiError.
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!("JSON serialization error: {:?}", err);
        ApiError::invalid_input(format!("Invalid JSON: {}", err))
    }
}

/// Convert from uuid::Error to ApiError.
impl From<uuid::Error> for ApiError {
    fn from(err: uuid::Error) -> Self {
        ApiError::invalid_format("id", &format!("valid UUID: {}", err))
    }
}

// ============================================================================
// RESULT TYPE ALIAS
// ============================================================================

/// Result type alias for API operations.
///
/// This is the standard result type used throughout the API layer.
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_code_status_mapping() {
        assert_eq!(ErrorCode::Unauthorized.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(ErrorCode::Forbidden.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(ErrorCode::ValidationFailed.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(ErrorCode::EntityNotFound.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(ErrorCode::LockConflict.status_code(), StatusCode::CONFLICT);
        assert_eq!(ErrorCode::InternalError.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(ErrorCode::ServiceUnavailable.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(ErrorCode::Timeout.status_code(), StatusCode::GATEWAY_TIMEOUT);
    }
    
    #[test]
    fn test_api_error_constructors() {
        let err = ApiError::unauthorized("Invalid credentials");
        assert_eq!(err.code, ErrorCode::Unauthorized);
        assert_eq!(err.message, "Invalid credentials");
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
        
        let err = ApiError::entity_not_found("Trajectory", "123");
        assert_eq!(err.code, ErrorCode::EntityNotFound);
        assert!(err.message.contains("Trajectory"));
        assert!(err.message.contains("123"));
        
        let err = ApiError::missing_field("name");
        assert_eq!(err.code, ErrorCode::MissingField);
        assert!(err.message.contains("name"));
    }
    
    #[test]
    fn test_api_error_with_details() {
        let details = serde_json::json!({
            "field": "email",
            "constraint": "must be valid email address"
        });
        
        let err = ApiError::validation_failed("Invalid email")
            .with_details(details.clone());
        
        assert_eq!(err.code, ErrorCode::ValidationFailed);
        assert_eq!(err.details, Some(details));
    }
    
    #[test]
    fn test_error_serialization() -> Result<(), serde_json::Error> {
        let err = ApiError::unauthorized("Invalid token");
        let json = serde_json::to_string(&err)?;
        
        assert!(json.contains("UNAUTHORIZED"));
        assert!(json.contains("Invalid token"));
        
        let deserialized: ApiError = serde_json::from_str(&json)?;
        assert_eq!(deserialized, err);
        Ok(())
    }
    
    #[test]
    fn test_error_display() {
        let err = ApiError::database_error("Connection failed");
        let display = format!("{}", err);
        
        assert!(display.contains("DatabaseError"));
        assert!(display.contains("Connection failed"));
    }
}
