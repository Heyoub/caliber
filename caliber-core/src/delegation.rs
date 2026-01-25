//! Delegation typestate for compile-time safety of delegation lifecycle.
//!
//! Uses the typestate pattern to make invalid state transitions uncompilable.
//!
//! # State Transition Diagram
//!
//! ```text
//! create() → Pending ──┬── accept() ──→ Accepted ── start() → InProgress ──┬── complete() → Completed
//!                      ├── reject() ──→ Rejected (terminal)                └── fail() → Failed (terminal)
//!                      └── timeout() ─→ Failed (terminal)
//! ```

use crate::{AgentId, ArtifactId, DelegationId, ScopeId, TenantId, Timestamp, TrajectoryId};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

// ============================================================================
// DELEGATION STATUS ENUM (replaces String)
// ============================================================================

/// Status of a delegation operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum DelegationStatus {
    /// Delegation is pending acceptance
    Pending,
    /// Delegation was accepted but not yet started
    Accepted,
    /// Delegation work is in progress
    InProgress,
    /// Delegation was completed successfully
    Completed,
    /// Delegation was rejected
    Rejected,
    /// Delegation failed (timeout, error, etc.)
    Failed,
}

impl DelegationStatus {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            DelegationStatus::Pending => "Pending",
            DelegationStatus::Accepted => "Accepted",
            DelegationStatus::InProgress => "InProgress",
            DelegationStatus::Completed => "Completed",
            DelegationStatus::Rejected => "Rejected",
            DelegationStatus::Failed => "Failed",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, DelegationStatusParseError> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(DelegationStatus::Pending),
            "accepted" => Ok(DelegationStatus::Accepted),
            "inprogress" | "in_progress" | "in-progress" => Ok(DelegationStatus::InProgress),
            "completed" | "complete" => Ok(DelegationStatus::Completed),
            "rejected" => Ok(DelegationStatus::Rejected),
            "failed" | "failure" => Ok(DelegationStatus::Failed),
            _ => Err(DelegationStatusParseError(s.to_string())),
        }
    }

    /// Check if this is a terminal state (no further transitions possible).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            DelegationStatus::Completed | DelegationStatus::Rejected | DelegationStatus::Failed
        )
    }
}

impl fmt::Display for DelegationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for DelegationStatus {
    type Err = DelegationStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid delegation status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationStatusParseError(pub String);

impl fmt::Display for DelegationStatusParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid delegation status: {}", self.0)
    }
}

impl std::error::Error for DelegationStatusParseError {}

// ============================================================================
// DELEGATION RESULT STATUS (replaces String in result)
// ============================================================================

/// Status of a delegation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum DelegationResultStatus {
    /// Task completed successfully
    Success,
    /// Task partially completed
    Partial,
    /// Task failed
    Failure,
}

impl DelegationResultStatus {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            DelegationResultStatus::Success => "Success",
            DelegationResultStatus::Partial => "Partial",
            DelegationResultStatus::Failure => "Failure",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, DelegationResultStatusParseError> {
        match s.to_lowercase().as_str() {
            "success" => Ok(DelegationResultStatus::Success),
            "partial" => Ok(DelegationResultStatus::Partial),
            "failure" | "failed" => Ok(DelegationResultStatus::Failure),
            _ => Err(DelegationResultStatusParseError(s.to_string())),
        }
    }
}

impl fmt::Display for DelegationResultStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for DelegationResultStatus {
    type Err = DelegationResultStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid delegation result status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationResultStatusParseError(pub String);

impl fmt::Display for DelegationResultStatusParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid delegation result status: {}", self.0)
    }
}

impl std::error::Error for DelegationResultStatusParseError {}

// ============================================================================
// DELEGATION RESULT
// ============================================================================

/// Result of a completed delegation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationResult {
    pub status: DelegationResultStatus,
    pub output: Option<String>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifacts: Vec<ArtifactId>,
    pub error: Option<String>,
}

// ============================================================================
// DELEGATION DATA (internal storage, state-independent)
// ============================================================================

/// Internal data storage for a delegation, independent of typestate.
/// This is what gets persisted to the database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationData {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub delegation_id: DelegationId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
    pub task_description: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub started_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expected_completion: Option<Timestamp>,
    pub result: Option<DelegationResult>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub context: Option<serde_json::Value>,
    /// Rejection reason (only set if rejected)
    pub rejection_reason: Option<String>,
    /// Failure reason (only set if failed)
    pub failure_reason: Option<String>,
}

// ============================================================================
// TYPESTATE MARKERS
// ============================================================================

/// Marker trait for delegation states.
pub trait DelegationState: private::Sealed + Send + Sync {}

/// Delegation is pending acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pending;
impl DelegationState for Pending {}

/// Delegation was accepted but not yet started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelegationAccepted;
impl DelegationState for DelegationAccepted {}

/// Delegation work is in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InProgress;
impl DelegationState for InProgress {}

/// Delegation was completed successfully (terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelegationCompleted;
impl DelegationState for DelegationCompleted {}

/// Delegation was rejected (terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelegationRejected;
impl DelegationState for DelegationRejected {}

/// Delegation failed (terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelegationFailed;
impl DelegationState for DelegationFailed {}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Pending {}
    impl Sealed for super::DelegationAccepted {}
    impl Sealed for super::InProgress {}
    impl Sealed for super::DelegationCompleted {}
    impl Sealed for super::DelegationRejected {}
    impl Sealed for super::DelegationFailed {}
}

// ============================================================================
// DELEGATION TYPESTATE WRAPPER
// ============================================================================

/// A delegation with compile-time state tracking.
///
/// The type parameter `S` indicates the current state of the delegation.
/// Methods are only available in appropriate states:
/// - `Delegation<Pending>`: Can be accepted, rejected, or timeout
/// - `Delegation<DelegationAccepted>`: Can be started or fail
/// - `Delegation<InProgress>`: Can be completed or fail
/// - `Delegation<DelegationCompleted>`: Terminal, has result
/// - `Delegation<DelegationRejected>`: Terminal
/// - `Delegation<DelegationFailed>`: Terminal, has failure reason
#[derive(Debug, Clone)]
pub struct Delegation<S: DelegationState> {
    data: DelegationData,
    _state: PhantomData<S>,
}

impl<S: DelegationState> Delegation<S> {
    /// Access the underlying delegation data (read-only).
    pub fn data(&self) -> &DelegationData {
        &self.data
    }

    /// Get the delegation ID.
    pub fn delegation_id(&self) -> DelegationId {
        self.data.delegation_id
    }

    /// Get the tenant ID.
    pub fn tenant_id(&self) -> TenantId {
        self.data.tenant_id
    }

    /// Get the delegating agent ID.
    pub fn from_agent_id(&self) -> AgentId {
        self.data.from_agent_id
    }

    /// Get the delegate agent ID.
    pub fn to_agent_id(&self) -> AgentId {
        self.data.to_agent_id
    }

    /// Get the trajectory ID.
    pub fn trajectory_id(&self) -> TrajectoryId {
        self.data.trajectory_id
    }

    /// Get the scope ID.
    pub fn scope_id(&self) -> ScopeId {
        self.data.scope_id
    }

    /// Get the task description.
    pub fn task_description(&self) -> &str {
        &self.data.task_description
    }

    /// Get when the delegation was created.
    pub fn created_at(&self) -> Timestamp {
        self.data.created_at
    }

    /// Get the expected completion time.
    pub fn expected_completion(&self) -> Option<Timestamp> {
        self.data.expected_completion
    }

    /// Get the context.
    pub fn context(&self) -> Option<&serde_json::Value> {
        self.data.context.as_ref()
    }

    /// Consume and return the underlying data (for serialization).
    pub fn into_data(self) -> DelegationData {
        self.data
    }
}

impl Delegation<Pending> {
    /// Create a new pending delegation.
    pub fn new(data: DelegationData) -> Self {
        Delegation {
            data,
            _state: PhantomData,
        }
    }

    /// Accept the delegation.
    ///
    /// Transitions to `Delegation<DelegationAccepted>`.
    /// Consumes the current delegation.
    pub fn accept(mut self, accepted_at: Timestamp) -> Delegation<DelegationAccepted> {
        self.data.accepted_at = Some(accepted_at);
        Delegation {
            data: self.data,
            _state: PhantomData,
        }
    }

    /// Reject the delegation.
    ///
    /// Transitions to `Delegation<DelegationRejected>` (terminal state).
    /// Consumes the current delegation.
    pub fn reject(mut self, reason: String) -> Delegation<DelegationRejected> {
        self.data.rejection_reason = Some(reason);
        Delegation {
            data: self.data,
            _state: PhantomData,
        }
    }

    /// Mark the delegation as failed (e.g., timeout).
    ///
    /// Transitions to `Delegation<DelegationFailed>` (terminal state).
    /// Consumes the current delegation.
    pub fn fail(mut self, reason: String) -> Delegation<DelegationFailed> {
        self.data.failure_reason = Some(reason);
        Delegation {
            data: self.data,
            _state: PhantomData,
        }
    }
}

impl Delegation<DelegationAccepted> {
    /// Get when the delegation was accepted.
    pub fn accepted_at(&self) -> Timestamp {
        self.data.accepted_at.expect("Accepted delegation must have accepted_at")
    }

    /// Start working on the delegation.
    ///
    /// Transitions to `Delegation<InProgress>`.
    /// Consumes the current delegation.
    pub fn start(mut self, started_at: Timestamp) -> Delegation<InProgress> {
        self.data.started_at = Some(started_at);
        Delegation {
            data: self.data,
            _state: PhantomData,
        }
    }

    /// Mark the delegation as failed before starting.
    ///
    /// Transitions to `Delegation<DelegationFailed>` (terminal state).
    /// Consumes the current delegation.
    pub fn fail(mut self, reason: String) -> Delegation<DelegationFailed> {
        self.data.failure_reason = Some(reason);
        Delegation {
            data: self.data,
            _state: PhantomData,
        }
    }
}

impl Delegation<InProgress> {
    /// Get when the delegation was accepted.
    pub fn accepted_at(&self) -> Timestamp {
        self.data.accepted_at.expect("In-progress delegation must have accepted_at")
    }

    /// Get when work started.
    pub fn started_at(&self) -> Timestamp {
        self.data.started_at.expect("In-progress delegation must have started_at")
    }

    /// Complete the delegation with a result.
    ///
    /// Transitions to `Delegation<DelegationCompleted>` (terminal state).
    /// Consumes the current delegation.
    pub fn complete(
        mut self,
        completed_at: Timestamp,
        result: DelegationResult,
    ) -> Delegation<DelegationCompleted> {
        self.data.completed_at = Some(completed_at);
        self.data.result = Some(result);
        Delegation {
            data: self.data,
            _state: PhantomData,
        }
    }

    /// Mark the delegation as failed during execution.
    ///
    /// Transitions to `Delegation<DelegationFailed>` (terminal state).
    /// Consumes the current delegation.
    pub fn fail(mut self, reason: String) -> Delegation<DelegationFailed> {
        self.data.failure_reason = Some(reason);
        Delegation {
            data: self.data,
            _state: PhantomData,
        }
    }
}

impl Delegation<DelegationCompleted> {
    /// Get when the delegation was accepted.
    pub fn accepted_at(&self) -> Timestamp {
        self.data.accepted_at.expect("Completed delegation must have accepted_at")
    }

    /// Get when work started.
    pub fn started_at(&self) -> Timestamp {
        self.data.started_at.expect("Completed delegation must have started_at")
    }

    /// Get when the delegation was completed.
    pub fn completed_at(&self) -> Timestamp {
        self.data.completed_at.expect("Completed delegation must have completed_at")
    }

    /// Get the delegation result.
    pub fn result(&self) -> &DelegationResult {
        self.data.result.as_ref().expect("Completed delegation must have result")
    }
}

impl Delegation<DelegationRejected> {
    /// Get the rejection reason.
    pub fn rejection_reason(&self) -> &str {
        self.data.rejection_reason.as_deref().unwrap_or("No reason provided")
    }
}

impl Delegation<DelegationFailed> {
    /// Get the failure reason.
    pub fn failure_reason(&self) -> &str {
        self.data.failure_reason.as_deref().unwrap_or("No reason provided")
    }
}

// ============================================================================
// DATABASE BOUNDARY: STORED DELEGATION
// ============================================================================

/// A delegation as stored in the database (status-agnostic).
///
/// When loading from the database, we don't know the state at compile time.
/// Use the `into_*` methods to validate and convert to a typed delegation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredDelegation {
    pub data: DelegationData,
    pub status: DelegationStatus,
}

/// Enum representing all possible runtime states of a delegation.
/// Use this when you need to handle delegations loaded from the database.
#[derive(Debug, Clone)]
pub enum LoadedDelegation {
    Pending(Delegation<Pending>),
    Accepted(Delegation<DelegationAccepted>),
    InProgress(Delegation<InProgress>),
    Completed(Delegation<DelegationCompleted>),
    Rejected(Delegation<DelegationRejected>),
    Failed(Delegation<DelegationFailed>),
}

impl StoredDelegation {
    /// Convert to a typed delegation based on the stored status.
    pub fn into_typed(self) -> LoadedDelegation {
        match self.status {
            DelegationStatus::Pending => LoadedDelegation::Pending(Delegation {
                data: self.data,
                _state: PhantomData,
            }),
            DelegationStatus::Accepted => LoadedDelegation::Accepted(Delegation {
                data: self.data,
                _state: PhantomData,
            }),
            DelegationStatus::InProgress => LoadedDelegation::InProgress(Delegation {
                data: self.data,
                _state: PhantomData,
            }),
            DelegationStatus::Completed => LoadedDelegation::Completed(Delegation {
                data: self.data,
                _state: PhantomData,
            }),
            DelegationStatus::Rejected => LoadedDelegation::Rejected(Delegation {
                data: self.data,
                _state: PhantomData,
            }),
            DelegationStatus::Failed => LoadedDelegation::Failed(Delegation {
                data: self.data,
                _state: PhantomData,
            }),
        }
    }

    /// Try to convert to a pending delegation.
    pub fn into_pending(self) -> Result<Delegation<Pending>, DelegationStateError> {
        if self.status != DelegationStatus::Pending {
            return Err(DelegationStateError::WrongState {
                delegation_id: self.data.delegation_id,
                expected: DelegationStatus::Pending,
                actual: self.status,
            });
        }
        Ok(Delegation {
            data: self.data,
            _state: PhantomData,
        })
    }

    /// Try to convert to an accepted delegation.
    pub fn into_accepted(self) -> Result<Delegation<DelegationAccepted>, DelegationStateError> {
        if self.status != DelegationStatus::Accepted {
            return Err(DelegationStateError::WrongState {
                delegation_id: self.data.delegation_id,
                expected: DelegationStatus::Accepted,
                actual: self.status,
            });
        }
        Ok(Delegation {
            data: self.data,
            _state: PhantomData,
        })
    }

    /// Try to convert to an in-progress delegation.
    pub fn into_in_progress(self) -> Result<Delegation<InProgress>, DelegationStateError> {
        if self.status != DelegationStatus::InProgress {
            return Err(DelegationStateError::WrongState {
                delegation_id: self.data.delegation_id,
                expected: DelegationStatus::InProgress,
                actual: self.status,
            });
        }
        Ok(Delegation {
            data: self.data,
            _state: PhantomData,
        })
    }

    /// Get the underlying data without state validation.
    pub fn data(&self) -> &DelegationData {
        &self.data
    }

    /// Get the current status.
    pub fn status(&self) -> DelegationStatus {
        self.status
    }
}

/// Errors when transitioning delegation states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelegationStateError {
    /// Delegation is not in the expected state.
    WrongState {
        delegation_id: DelegationId,
        expected: DelegationStatus,
        actual: DelegationStatus,
    },
}

impl fmt::Display for DelegationStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DelegationStateError::WrongState {
                delegation_id,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Delegation {} is in state {} but expected {}",
                    delegation_id, actual, expected
                )
            }
        }
    }
}

impl std::error::Error for DelegationStateError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntityIdType;
    use chrono::Utc;

    fn make_delegation_data() -> DelegationData {
        let now = Utc::now();
        DelegationData {
            delegation_id: DelegationId::now_v7(),
            tenant_id: TenantId::now_v7(),
            from_agent_id: AgentId::now_v7(),
            to_agent_id: AgentId::now_v7(),
            trajectory_id: TrajectoryId::now_v7(),
            scope_id: ScopeId::now_v7(),
            task_description: "Analyze codebase".to_string(),
            created_at: now,
            accepted_at: None,
            started_at: None,
            completed_at: None,
            expected_completion: Some(now + chrono::Duration::hours(1)),
            result: None,
            context: None,
            rejection_reason: None,
            failure_reason: None,
        }
    }

    #[test]
    fn test_delegation_status_roundtrip() {
        for status in [
            DelegationStatus::Pending,
            DelegationStatus::Accepted,
            DelegationStatus::InProgress,
            DelegationStatus::Completed,
            DelegationStatus::Rejected,
            DelegationStatus::Failed,
        ] {
            let db_str = status.as_db_str();
            let parsed = DelegationStatus::from_db_str(db_str).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_delegation_happy_path() {
        let now = Utc::now();
        let data = make_delegation_data();
        let delegation = Delegation::<Pending>::new(data);

        let accepted = delegation.accept(now);
        assert_eq!(accepted.accepted_at(), now);

        let in_progress = accepted.start(now);
        assert_eq!(in_progress.started_at(), now);

        let result = DelegationResult {
            status: DelegationResultStatus::Success,
            output: Some("Done".to_string()),
            artifacts: vec![],
            error: None,
        };
        let completed = in_progress.complete(now, result);
        assert_eq!(completed.result().status, DelegationResultStatus::Success);
    }

    #[test]
    fn test_delegation_reject() {
        let data = make_delegation_data();
        let delegation = Delegation::<Pending>::new(data);

        let rejected = delegation.reject("Not available".to_string());
        assert_eq!(rejected.rejection_reason(), "Not available");
    }

    #[test]
    fn test_delegation_fail() {
        let now = Utc::now();
        let data = make_delegation_data();
        let delegation = Delegation::<Pending>::new(data);

        let accepted = delegation.accept(now);
        let in_progress = accepted.start(now);
        let failed = in_progress.fail("Timeout".to_string());
        assert_eq!(failed.failure_reason(), "Timeout");
    }

    #[test]
    fn test_stored_delegation_conversion() {
        let data = make_delegation_data();
        let stored = StoredDelegation {
            data: data.clone(),
            status: DelegationStatus::Pending,
        };

        let pending = stored.into_pending().unwrap();
        assert_eq!(pending.delegation_id(), data.delegation_id);
    }

    #[test]
    fn test_stored_delegation_wrong_state() {
        let data = make_delegation_data();
        let stored = StoredDelegation {
            data,
            status: DelegationStatus::Accepted,
        };

        assert!(matches!(
            stored.into_pending(),
            Err(DelegationStateError::WrongState { .. })
        ));
    }
}
