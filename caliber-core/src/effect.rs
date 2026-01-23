//! Effect system for error-as-effects pattern.
//!
//! This module implements the "errors as effects" pattern where domain errors
//! are first-class events that can be persisted, replayed, and affect downstream
//! handlers.
//!
//! # Usage Guidelines
//!
//! ## Effect at Boundaries, Result Internally
//!
//! Internal code should use `Result<T, E>` for normal error handling.
//! `Effect<T>` should only be used at system boundaries:
//! - Event handler outputs
//! - Persistence layer responses
//! - API response wrappers
//!
//! ```rust,ignore
//! // Internal: use Result
//! async fn fetch_notes(&self, id: EntityId) -> Result<Vec<Note>, StorageError> {
//!     let rows = self.db.query(...).await?;
//!     Ok(rows)
//! }
//!
//! // Boundary: wrap in Effect
//! pub async fn handle_request(&self, req: Request) -> Effect<Response> {
//!     match self.do_work(req).await {
//!         Ok(resp) => Effect::Ok(resp),
//!         Err(e) if e.is_transient() => Effect::Retry { ... },
//!         Err(e) => Effect::Err(ErrorEffect::from(e)),
//!     }
//! }
//! ```
//!
//! # Key Distinction
//!
//! - **Domain errors**: Persist, replay, affect downstream handlers
//! - **Operational errors**: Telemetry only, can sample/discard
//!
//! Domain errors are part of the business logic and must be tracked.
//! Operational errors are infrastructure concerns and can be handled separately.

use crate::{EntityId, EventId, DagPosition};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

// ============================================================================
// EFFECT TYPE
// ============================================================================

/// An effect represents the outcome of an operation.
///
/// Effects are more expressive than simple `Result<T, E>` because they can
/// represent retry conditions, compensation actions, and pending states.
///
/// # Variants
///
/// - `Ok(T)`: Successful result
/// - `Err(ErrorEffect)`: Domain-level error (replayable, affects downstream)
/// - `Retry`: Operation should be retried
/// - `Compensate`: Compensation action is needed
/// - `Pending`: Operation is waiting for something
/// - `Batch`: Multiple effects (for fan-out scenarios)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Effect<T> {
    /// Successful result
    Ok(T),
    /// Domain-level error (must be persisted and handled)
    Err(ErrorEffect),
    /// Operation should be retried
    Retry {
        /// Duration to wait before retrying
        #[serde(with = "duration_millis")]
        after: Duration,
        /// Current attempt number (1-indexed)
        attempt: u32,
        /// Maximum number of attempts
        max_attempts: u32,
        /// Reason for retry
        reason: String,
    },
    /// Compensation action is needed
    Compensate {
        /// The action to take
        action: CompensationAction,
        /// The error that caused compensation
        cause: Box<ErrorEffect>,
    },
    /// Operation is waiting for something
    Pending {
        /// What the operation is waiting for
        waiting_for: WaitCondition,
        /// Token to resume when condition is met
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        resume_token: EventId,
    },
    /// Multiple effects (for fan-out scenarios)
    Batch(Vec<Effect<T>>),
}

impl<T> Effect<T> {
    /// Create a successful effect.
    pub fn ok(value: T) -> Self {
        Effect::Ok(value)
    }

    /// Create an error effect.
    pub fn err(error: ErrorEffect) -> Self {
        Effect::Err(error)
    }

    /// Create a domain error effect.
    pub fn domain_error(error: DomainError, source_event: EventId, position: DagPosition) -> Self {
        Effect::Err(ErrorEffect::Domain(DomainErrorContext {
            error,
            source_event,
            position,
            correlation_id: source_event, // Default to same as source
        }))
    }

    /// Create a retry effect.
    pub fn retry(after: Duration, attempt: u32, max_attempts: u32, reason: impl Into<String>) -> Self {
        Effect::Retry {
            after,
            attempt,
            max_attempts,
            reason: reason.into(),
        }
    }

    /// Create a pending effect.
    pub fn pending(waiting_for: WaitCondition, resume_token: EventId) -> Self {
        Effect::Pending {
            waiting_for,
            resume_token,
        }
    }

    /// Check if this is a successful effect.
    pub fn is_ok(&self) -> bool {
        matches!(self, Effect::Ok(_))
    }

    /// Check if this is an error effect.
    pub fn is_err(&self) -> bool {
        matches!(self, Effect::Err(_))
    }

    /// Check if this effect requires retry.
    pub fn needs_retry(&self) -> bool {
        matches!(self, Effect::Retry { .. })
    }

    /// Check if this effect is pending.
    pub fn is_pending(&self) -> bool {
        matches!(self, Effect::Pending { .. })
    }

    /// Convert to a Result, losing retry/compensation information.
    pub fn into_result(self) -> Result<T, ErrorEffect> {
        match self {
            Effect::Ok(v) => Ok(v),
            Effect::Err(e) => Err(e),
            Effect::Retry { reason, .. } => Err(ErrorEffect::Operational(OperationalError::RetryExhausted {
                reason,
            })),
            Effect::Compensate { cause, .. } => Err(*cause),
            Effect::Pending { waiting_for, .. } => Err(ErrorEffect::Operational(OperationalError::Timeout {
                operation: format!("Pending: {:?}", waiting_for),
            })),
            Effect::Batch(effects) => {
                // Return first error or last success
                for effect in effects {
                    effect.into_result()?;
                }
                Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Empty batch".to_string(),
                }))
            }
        }
    }

    /// Map the success value.
    pub fn map<U, F: FnOnce(T) -> U + Clone>(self, f: F) -> Effect<U> {
        match self {
            Effect::Ok(v) => Effect::Ok(f(v)),
            Effect::Err(e) => Effect::Err(e),
            Effect::Retry { after, attempt, max_attempts, reason } => {
                Effect::Retry { after, attempt, max_attempts, reason }
            }
            Effect::Compensate { action, cause } => Effect::Compensate { action, cause },
            Effect::Pending { waiting_for, resume_token } => {
                Effect::Pending { waiting_for, resume_token }
            }
            Effect::Batch(effects) => Effect::Batch(effects.into_iter().map(|e| e.map(f.clone())).collect()),
        }
    }

    /// Extract the success value, panicking if not Ok.
    pub fn unwrap(self) -> T {
        match self {
            Effect::Ok(v) => v,
            _ => panic!("Called unwrap on non-Ok effect: {:?}", std::any::type_name::<Self>()),
        }
    }

    /// Chain a function that returns an Effect on the success value.
    ///
    /// If `self` is `Ok(t)`, returns `f(t)`. Otherwise, returns the original
    /// non-Ok effect unchanged.
    pub fn and_then<U, F: FnOnce(T) -> Effect<U>>(self, f: F) -> Effect<U> {
        match self {
            Effect::Ok(v) => f(v),
            Effect::Err(e) => Effect::Err(e),
            Effect::Retry { after, attempt, max_attempts, reason } => {
                Effect::Retry { after, attempt, max_attempts, reason }
            }
            Effect::Compensate { action, cause } => Effect::Compensate { action, cause },
            Effect::Pending { waiting_for, resume_token } => {
                Effect::Pending { waiting_for, resume_token }
            }
            Effect::Batch(_) => Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                message: "Cannot and_then on Batch effect".to_string(),
            })),
        }
    }

    /// Map the error effect using a transformation function.
    ///
    /// If `self` is `Err(e)`, returns `Err(f(e))`. Otherwise, returns `self` unchanged.
    pub fn map_err<F: FnOnce(ErrorEffect) -> ErrorEffect>(self, f: F) -> Self {
        match self {
            Effect::Err(e) => Effect::Err(f(e)),
            other => other,
        }
    }

    /// Handle an error by applying a recovery function.
    ///
    /// If `self` is `Err(e)`, returns `f(e)`. Otherwise, returns `self` unchanged.
    pub fn or_else<F: FnOnce(ErrorEffect) -> Effect<T>>(self, f: F) -> Self {
        match self {
            Effect::Err(e) => f(e),
            other => other,
        }
    }

    /// Extract the success value or return a default.
    ///
    /// If `self` is `Ok(t)`, returns `t`. Otherwise, returns `default`.
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Effect::Ok(v) => v,
            _ => default,
        }
    }

    /// Extract the success value or compute a default from the error.
    ///
    /// If `self` is `Ok(t)`, returns `t`. Otherwise, returns `f(e)` where
    /// `e` is the error effect (or a synthesized one for non-error variants).
    pub fn unwrap_or_else<F: FnOnce(ErrorEffect) -> T>(self, f: F) -> T {
        match self {
            Effect::Ok(v) => v,
            Effect::Err(e) => f(e),
            Effect::Retry { reason, .. } => f(ErrorEffect::Operational(OperationalError::RetryExhausted {
                reason,
            })),
            Effect::Compensate { cause, .. } => f(*cause),
            Effect::Pending { waiting_for, .. } => f(ErrorEffect::Operational(OperationalError::Timeout {
                operation: format!("Pending: {:?}", waiting_for),
            })),
            Effect::Batch(_) => f(ErrorEffect::Operational(OperationalError::Internal {
                message: "Cannot unwrap Batch effect".to_string(),
            })),
        }
    }
}

impl<T, E: Into<ErrorEffect>> From<Result<T, E>> for Effect<T> {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(v) => Effect::Ok(v),
            Err(e) => Effect::Err(e.into()),
        }
    }
}

// ============================================================================
// ERROR EFFECT
// ============================================================================

/// An error effect that can be persisted and replayed.
///
/// This distinguishes between:
/// - Domain errors: Business logic errors that must be tracked
/// - Operational errors: Infrastructure errors that can be sampled/discarded
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ErrorEffect {
    /// Domain-level error (must persist, replay, affect downstream)
    Domain(DomainErrorContext),
    /// Operational error (telemetry only, can sample/discard)
    Operational(OperationalError),
}

impl ErrorEffect {
    /// Check if this is a domain error.
    pub fn is_domain(&self) -> bool {
        matches!(self, ErrorEffect::Domain(_))
    }

    /// Check if this is an operational error.
    pub fn is_operational(&self) -> bool {
        matches!(self, ErrorEffect::Operational(_))
    }

    /// Get the error kind for categorization.
    pub fn kind(&self) -> ErrorKind {
        match self {
            ErrorEffect::Domain(ctx) => ctx.error.kind(),
            ErrorEffect::Operational(op) => op.kind(),
        }
    }
}

impl fmt::Display for ErrorEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorEffect::Domain(ctx) => write!(f, "Domain error: {}", ctx.error),
            ErrorEffect::Operational(op) => write!(f, "Operational error: {}", op),
        }
    }
}

impl std::error::Error for ErrorEffect {}

// ============================================================================
// DOMAIN ERROR CONTEXT
// ============================================================================

/// Domain error with event context for correlation and replay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DomainErrorContext {
    /// The domain error
    pub error: DomainError,
    /// Event that caused this error
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub source_event: EventId,
    /// Position in the DAG where error occurred
    pub position: DagPosition,
    /// Correlation ID for tracing
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub correlation_id: EventId,
}

// ============================================================================
// DOMAIN ERRORS
// ============================================================================

/// Domain-level errors that affect business logic.
///
/// These errors must be persisted and can affect downstream processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum DomainError {
    // Entity errors
    EntityNotFound {
        entity_type: String,
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        id: EntityId,
    },
    EntityAlreadyExists {
        entity_type: String,
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        id: EntityId,
    },
    EntityConflict {
        entity_type: String,
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        id: EntityId,
        reason: String,
    },

    // State errors
    InvalidStateTransition {
        entity_type: String,
        from_state: String,
        to_state: String,
        reason: String,
    },
    StaleData {
        entity_type: String,
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        id: EntityId,
        expected_version: u64,
        actual_version: u64,
    },

    // Validation errors
    ValidationFailed {
        field: String,
        reason: String,
    },
    ConstraintViolation {
        constraint: String,
        reason: String,
    },
    CircularReference {
        entity_type: String,
        #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
        ids: Vec<EntityId>,
    },

    // Business logic errors
    Contradiction {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        artifact_a: EntityId,
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        artifact_b: EntityId,
        description: String,
    },
    QuotaExceeded {
        resource: String,
        limit: u64,
        requested: u64,
    },
    PermissionDenied {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        agent_id: EntityId,
        action: String,
        resource: String,
    },

    // Agent coordination errors
    LockAcquisitionFailed {
        resource: String,
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        holder: EntityId,
    },
    LockExpired {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        lock_id: EntityId,
    },
    DelegationFailed {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        delegation_id: EntityId,
        reason: String,
    },
    HandoffFailed {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        handoff_id: EntityId,
        reason: String,
    },
}

impl DomainError {
    /// Get the error kind for categorization.
    pub fn kind(&self) -> ErrorKind {
        match self {
            DomainError::EntityNotFound { .. } => ErrorKind::NotFound,
            DomainError::EntityAlreadyExists { .. } => ErrorKind::AlreadyExists,
            DomainError::EntityConflict { .. } => ErrorKind::Conflict,
            DomainError::InvalidStateTransition { .. } => ErrorKind::InvalidState,
            DomainError::StaleData { .. } => ErrorKind::Conflict,
            DomainError::ValidationFailed { .. } => ErrorKind::Validation,
            DomainError::ConstraintViolation { .. } => ErrorKind::Validation,
            DomainError::CircularReference { .. } => ErrorKind::Validation,
            DomainError::Contradiction { .. } => ErrorKind::BusinessLogic,
            DomainError::QuotaExceeded { .. } => ErrorKind::QuotaExceeded,
            DomainError::PermissionDenied { .. } => ErrorKind::PermissionDenied,
            DomainError::LockAcquisitionFailed { .. } => ErrorKind::LockFailed,
            DomainError::LockExpired { .. } => ErrorKind::LockFailed,
            DomainError::DelegationFailed { .. } => ErrorKind::CoordinationFailed,
            DomainError::HandoffFailed { .. } => ErrorKind::CoordinationFailed,
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::EntityNotFound { entity_type, id } => {
                write!(f, "{} not found: {}", entity_type, id)
            }
            DomainError::EntityAlreadyExists { entity_type, id } => {
                write!(f, "{} already exists: {}", entity_type, id)
            }
            DomainError::EntityConflict { entity_type, id, reason } => {
                write!(f, "Conflict on {} {}: {}", entity_type, id, reason)
            }
            DomainError::InvalidStateTransition { entity_type, from_state, to_state, reason } => {
                write!(f, "Invalid {} transition {} -> {}: {}", entity_type, from_state, to_state, reason)
            }
            DomainError::StaleData { entity_type, id, expected_version, actual_version } => {
                write!(f, "Stale {} {}: expected version {}, got {}", entity_type, id, expected_version, actual_version)
            }
            DomainError::ValidationFailed { field, reason } => {
                write!(f, "Validation failed for {}: {}", field, reason)
            }
            DomainError::ConstraintViolation { constraint, reason } => {
                write!(f, "Constraint {} violated: {}", constraint, reason)
            }
            DomainError::CircularReference { entity_type, ids } => {
                write!(f, "Circular reference in {}: {:?}", entity_type, ids)
            }
            DomainError::Contradiction { artifact_a, artifact_b, description } => {
                write!(f, "Contradiction between {} and {}: {}", artifact_a, artifact_b, description)
            }
            DomainError::QuotaExceeded { resource, limit, requested } => {
                write!(f, "Quota exceeded for {}: limit {}, requested {}", resource, limit, requested)
            }
            DomainError::PermissionDenied { agent_id, action, resource } => {
                write!(f, "Permission denied: agent {} cannot {} on {}", agent_id, action, resource)
            }
            DomainError::LockAcquisitionFailed { resource, holder } => {
                write!(f, "Lock acquisition failed for {}: held by {}", resource, holder)
            }
            DomainError::LockExpired { lock_id } => {
                write!(f, "Lock expired: {}", lock_id)
            }
            DomainError::DelegationFailed { delegation_id, reason } => {
                write!(f, "Delegation {} failed: {}", delegation_id, reason)
            }
            DomainError::HandoffFailed { handoff_id, reason } => {
                write!(f, "Handoff {} failed: {}", handoff_id, reason)
            }
        }
    }
}

impl std::error::Error for DomainError {}

// ============================================================================
// OPERATIONAL ERRORS
// ============================================================================

/// Operational errors that don't affect business logic.
///
/// These errors are for infrastructure concerns and can be sampled/discarded
/// for telemetry purposes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum OperationalError {
    /// Network or connection error
    NetworkError { message: String },
    /// Database connection error
    DatabaseConnectionError { message: String },
    /// Operation timed out
    Timeout { operation: String },
    /// Rate limited by external service
    RateLimited { service: String, retry_after_ms: i64 },
    /// Internal error (unexpected)
    Internal { message: String },
    /// Resource temporarily unavailable
    Unavailable { resource: String },
    /// Retries exhausted
    RetryExhausted { reason: String },
    /// Serialization/deserialization error
    SerializationError { message: String },
}

impl OperationalError {
    /// Get the error kind for categorization.
    pub fn kind(&self) -> ErrorKind {
        match self {
            OperationalError::NetworkError { .. } => ErrorKind::Network,
            OperationalError::DatabaseConnectionError { .. } => ErrorKind::Database,
            OperationalError::Timeout { .. } => ErrorKind::Timeout,
            OperationalError::RateLimited { .. } => ErrorKind::RateLimited,
            OperationalError::Internal { .. } => ErrorKind::Internal,
            OperationalError::Unavailable { .. } => ErrorKind::Unavailable,
            OperationalError::RetryExhausted { .. } => ErrorKind::RetryExhausted,
            OperationalError::SerializationError { .. } => ErrorKind::Serialization,
        }
    }
}

impl fmt::Display for OperationalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationalError::NetworkError { message } => write!(f, "Network error: {}", message),
            OperationalError::DatabaseConnectionError { message } => write!(f, "Database error: {}", message),
            OperationalError::Timeout { operation } => write!(f, "Timeout: {}", operation),
            OperationalError::RateLimited { service, retry_after_ms } => {
                write!(f, "Rate limited by {}, retry after {}ms", service, retry_after_ms)
            }
            OperationalError::Internal { message } => write!(f, "Internal error: {}", message),
            OperationalError::Unavailable { resource } => write!(f, "Unavailable: {}", resource),
            OperationalError::RetryExhausted { reason } => write!(f, "Retries exhausted: {}", reason),
            OperationalError::SerializationError { message } => write!(f, "Serialization error: {}", message),
        }
    }
}

impl std::error::Error for OperationalError {}

// ============================================================================
// ERROR KIND
// ============================================================================

/// High-level error categorization for metrics and routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ErrorKind {
    // Domain error kinds
    NotFound,
    AlreadyExists,
    Conflict,
    InvalidState,
    Validation,
    BusinessLogic,
    QuotaExceeded,
    PermissionDenied,
    LockFailed,
    CoordinationFailed,

    // Operational error kinds
    Network,
    Database,
    Timeout,
    RateLimited,
    Internal,
    Unavailable,
    RetryExhausted,
    Serialization,
}

impl ErrorKind {
    /// Check if this is a retriable error kind.
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            ErrorKind::Network
                | ErrorKind::Database
                | ErrorKind::Timeout
                | ErrorKind::RateLimited
                | ErrorKind::Unavailable
        )
    }

    /// Check if this is a domain error kind.
    pub fn is_domain(&self) -> bool {
        matches!(
            self,
            ErrorKind::NotFound
                | ErrorKind::AlreadyExists
                | ErrorKind::Conflict
                | ErrorKind::InvalidState
                | ErrorKind::Validation
                | ErrorKind::BusinessLogic
                | ErrorKind::QuotaExceeded
                | ErrorKind::PermissionDenied
                | ErrorKind::LockFailed
                | ErrorKind::CoordinationFailed
        )
    }
}

// ============================================================================
// COMPENSATION ACTION
// ============================================================================

/// Action to take for compensating a failed operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum CompensationAction {
    /// Rollback changes
    Rollback {
        /// Events to undo
        #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
        events: Vec<EventId>,
    },
    /// Notify an agent about the failure
    NotifyAgent {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        agent_id: EntityId,
        message: String,
    },
    /// Release held resources
    ReleaseResources {
        #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
        resource_ids: Vec<EntityId>,
    },
    /// Custom compensation
    Custom {
        action_type: String,
        #[cfg_attr(feature = "openapi", schema(value_type = Object))]
        payload: serde_json::Value,
    },
}

// ============================================================================
// WAIT CONDITION
// ============================================================================

/// Condition that a pending effect is waiting for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum WaitCondition {
    /// Waiting for an event
    Event {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        event_id: EventId,
    },
    /// Waiting for a lock to be released
    Lock {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        lock_id: EntityId,
    },
    /// Waiting for a delegation to complete
    Delegation {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        delegation_id: EntityId,
    },
    /// Waiting for a timeout
    Timeout {
        /// Timestamp in microseconds when to resume
        resume_at: i64,
    },
    /// Waiting for external input
    ExternalInput {
        source: String,
    },
}

// ============================================================================
// SERDE HELPERS
// ============================================================================

mod duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_effect_ok() {
        let effect: Effect<i32> = Effect::ok(42);
        assert!(effect.is_ok());
        assert_eq!(effect.unwrap(), 42);
    }

    #[test]
    fn test_effect_err() {
        let effect: Effect<i32> = Effect::domain_error(
            DomainError::EntityNotFound {
                entity_type: "Trajectory".to_string(),
                id: Uuid::now_v7(),
            },
            Uuid::now_v7(),
            DagPosition::root(),
        );
        assert!(effect.is_err());
    }

    #[test]
    fn test_effect_retry() {
        let effect: Effect<i32> = Effect::retry(
            Duration::from_secs(1),
            1,
            3,
            "Temporary failure",
        );
        assert!(effect.needs_retry());
    }

    #[test]
    fn test_effect_map() {
        let effect: Effect<i32> = Effect::ok(42);
        let mapped = effect.map(|n| n * 2);
        assert_eq!(mapped.unwrap(), 84);
    }

    #[test]
    fn test_error_kind_retriable() {
        assert!(ErrorKind::Network.is_retriable());
        assert!(ErrorKind::Timeout.is_retriable());
        assert!(!ErrorKind::NotFound.is_retriable());
        assert!(!ErrorKind::PermissionDenied.is_retriable());
    }

    #[test]
    fn test_domain_vs_operational() {
        let domain = ErrorEffect::Domain(DomainErrorContext {
            error: DomainError::EntityNotFound {
                entity_type: "Test".to_string(),
                id: Uuid::now_v7(),
            },
            source_event: Uuid::now_v7(),
            position: DagPosition::root(),
            correlation_id: Uuid::now_v7(),
        });
        assert!(domain.is_domain());

        let operational = ErrorEffect::Operational(OperationalError::Timeout {
            operation: "test".to_string(),
        });
        assert!(operational.is_operational());
    }
}
