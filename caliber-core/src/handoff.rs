//! Handoff typestate for compile-time safety of handoff lifecycle.
//!
//! Uses the typestate pattern to make invalid state transitions uncompilable.
//!
//! # State Transition Diagram
//!
//! ```text
//! create() → Initiated ──┬── accept() ──→ Accepted ── complete() → Completed
//!                        └── reject() ──→ Rejected (terminal)
//! ```

use crate::{AgentId, HandoffId, ScopeId, TenantId, Timestamp, TrajectoryId};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

// ============================================================================
// HANDOFF STATUS ENUM (replaces String)
// ============================================================================

/// Status of a handoff operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum HandoffStatus {
    /// Handoff has been initiated, waiting for acceptance
    Initiated,
    /// Handoff was accepted by the receiving agent
    Accepted,
    /// Handoff was rejected by the receiving agent
    Rejected,
    /// Handoff has been completed successfully
    Completed,
}

impl HandoffStatus {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            HandoffStatus::Initiated => "Initiated",
            HandoffStatus::Accepted => "Accepted",
            HandoffStatus::Rejected => "Rejected",
            HandoffStatus::Completed => "Completed",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, HandoffStatusParseError> {
        match s.to_lowercase().as_str() {
            "initiated" | "pending" => Ok(HandoffStatus::Initiated),
            "accepted" => Ok(HandoffStatus::Accepted),
            "rejected" => Ok(HandoffStatus::Rejected),
            "completed" | "complete" => Ok(HandoffStatus::Completed),
            _ => Err(HandoffStatusParseError(s.to_string())),
        }
    }

    /// Check if this is a terminal state (no further transitions possible).
    pub fn is_terminal(&self) -> bool {
        matches!(self, HandoffStatus::Rejected | HandoffStatus::Completed)
    }
}

impl fmt::Display for HandoffStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for HandoffStatus {
    type Err = HandoffStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid handoff status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffStatusParseError(pub String);

impl fmt::Display for HandoffStatusParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid handoff status: {}", self.0)
    }
}

impl std::error::Error for HandoffStatusParseError {}

// ============================================================================
// HANDOFF DATA (internal storage, state-independent)
// ============================================================================

/// Internal data storage for a handoff, independent of typestate.
/// This is what gets persisted to the database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HandoffData {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub handoff_id: HandoffId,
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
    pub reason: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_snapshot: Vec<u8>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    /// Rejection reason (only set if rejected)
    pub rejection_reason: Option<String>,
}

// ============================================================================
// TYPESTATE MARKERS
// ============================================================================

/// Marker trait for handoff states.
pub trait HandoffState: private::Sealed + Send + Sync {}

/// Handoff has been initiated, waiting for acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Initiated;
impl HandoffState for Initiated {}

/// Handoff was accepted by the receiving agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandoffAccepted;
impl HandoffState for HandoffAccepted {}

/// Handoff was rejected by the receiving agent (terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejected;
impl HandoffState for Rejected {}

/// Handoff has been completed (terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandoffCompleted;
impl HandoffState for HandoffCompleted {}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Initiated {}
    impl Sealed for super::HandoffAccepted {}
    impl Sealed for super::Rejected {}
    impl Sealed for super::HandoffCompleted {}
}

// ============================================================================
// HANDOFF TYPESTATE WRAPPER
// ============================================================================

/// A handoff with compile-time state tracking.
///
/// The type parameter `S` indicates the current state of the handoff.
/// Methods are only available in appropriate states:
/// - `Handoff<Initiated>`: Can be accepted or rejected
/// - `Handoff<HandoffAccepted>`: Can be completed
/// - `Handoff<Rejected>`: Terminal, no further transitions
/// - `Handoff<HandoffCompleted>`: Terminal, no further transitions
#[derive(Debug, Clone)]
pub struct Handoff<S: HandoffState> {
    data: HandoffData,
    _state: PhantomData<S>,
}

impl<S: HandoffState> Handoff<S> {
    /// Access the underlying handoff data (read-only).
    pub fn data(&self) -> &HandoffData {
        &self.data
    }

    /// Get the handoff ID.
    pub fn handoff_id(&self) -> HandoffId {
        self.data.handoff_id
    }

    /// Get the tenant ID.
    pub fn tenant_id(&self) -> TenantId {
        self.data.tenant_id
    }

    /// Get the source agent ID.
    pub fn from_agent_id(&self) -> AgentId {
        self.data.from_agent_id
    }

    /// Get the target agent ID.
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

    /// Get the handoff reason.
    pub fn reason(&self) -> &str {
        &self.data.reason
    }

    /// Get the context snapshot.
    pub fn context_snapshot(&self) -> &[u8] {
        &self.data.context_snapshot
    }

    /// Get when the handoff was created.
    pub fn created_at(&self) -> Timestamp {
        self.data.created_at
    }

    /// Consume and return the underlying data (for serialization).
    pub fn into_data(self) -> HandoffData {
        self.data
    }
}

impl Handoff<Initiated> {
    /// Create a new initiated handoff.
    pub fn new(data: HandoffData) -> Self {
        Handoff {
            data,
            _state: PhantomData,
        }
    }

    /// Accept the handoff.
    ///
    /// Transitions to `Handoff<HandoffAccepted>`.
    /// Consumes the current handoff.
    pub fn accept(mut self, accepted_at: Timestamp) -> Handoff<HandoffAccepted> {
        self.data.accepted_at = Some(accepted_at);
        Handoff {
            data: self.data,
            _state: PhantomData,
        }
    }

    /// Reject the handoff.
    ///
    /// Transitions to `Handoff<Rejected>` (terminal state).
    /// Consumes the current handoff.
    pub fn reject(mut self, reason: String) -> Handoff<Rejected> {
        self.data.rejection_reason = Some(reason);
        Handoff {
            data: self.data,
            _state: PhantomData,
        }
    }
}

impl Handoff<HandoffAccepted> {
    /// Get when the handoff was accepted.
    pub fn accepted_at(&self) -> Timestamp {
        self.data.accepted_at.expect("Accepted handoff must have accepted_at")
    }

    /// Complete the handoff.
    ///
    /// Transitions to `Handoff<HandoffCompleted>` (terminal state).
    /// Consumes the current handoff.
    pub fn complete(mut self, completed_at: Timestamp) -> Handoff<HandoffCompleted> {
        self.data.completed_at = Some(completed_at);
        Handoff {
            data: self.data,
            _state: PhantomData,
        }
    }
}

impl Handoff<Rejected> {
    /// Get the rejection reason.
    pub fn rejection_reason(&self) -> &str {
        self.data.rejection_reason.as_deref().unwrap_or("No reason provided")
    }
}

impl Handoff<HandoffCompleted> {
    /// Get when the handoff was accepted.
    pub fn accepted_at(&self) -> Timestamp {
        self.data.accepted_at.expect("Completed handoff must have accepted_at")
    }

    /// Get when the handoff was completed.
    pub fn completed_at(&self) -> Timestamp {
        self.data.completed_at.expect("Completed handoff must have completed_at")
    }
}

// ============================================================================
// DATABASE BOUNDARY: STORED HANDOFF
// ============================================================================

/// A handoff as stored in the database (status-agnostic).
///
/// When loading from the database, we don't know the state at compile time.
/// Use the `into_*` methods to validate and convert to a typed handoff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredHandoff {
    pub data: HandoffData,
    pub status: HandoffStatus,
}

/// Enum representing all possible runtime states of a handoff.
/// Use this when you need to handle handoffs loaded from the database.
#[derive(Debug, Clone)]
pub enum LoadedHandoff {
    Initiated(Handoff<Initiated>),
    Accepted(Handoff<HandoffAccepted>),
    Rejected(Handoff<Rejected>),
    Completed(Handoff<HandoffCompleted>),
}

impl StoredHandoff {
    /// Convert to a typed handoff based on the stored status.
    pub fn into_typed(self) -> LoadedHandoff {
        match self.status {
            HandoffStatus::Initiated => LoadedHandoff::Initiated(Handoff {
                data: self.data,
                _state: PhantomData,
            }),
            HandoffStatus::Accepted => LoadedHandoff::Accepted(Handoff {
                data: self.data,
                _state: PhantomData,
            }),
            HandoffStatus::Rejected => LoadedHandoff::Rejected(Handoff {
                data: self.data,
                _state: PhantomData,
            }),
            HandoffStatus::Completed => LoadedHandoff::Completed(Handoff {
                data: self.data,
                _state: PhantomData,
            }),
        }
    }

    /// Try to convert to an initiated handoff.
    pub fn into_initiated(self) -> Result<Handoff<Initiated>, HandoffStateError> {
        if self.status != HandoffStatus::Initiated {
            return Err(HandoffStateError::WrongState {
                handoff_id: self.data.handoff_id,
                expected: HandoffStatus::Initiated,
                actual: self.status,
            });
        }
        Ok(Handoff {
            data: self.data,
            _state: PhantomData,
        })
    }

    /// Try to convert to an accepted handoff.
    pub fn into_accepted(self) -> Result<Handoff<HandoffAccepted>, HandoffStateError> {
        if self.status != HandoffStatus::Accepted {
            return Err(HandoffStateError::WrongState {
                handoff_id: self.data.handoff_id,
                expected: HandoffStatus::Accepted,
                actual: self.status,
            });
        }
        Ok(Handoff {
            data: self.data,
            _state: PhantomData,
        })
    }

    /// Get the underlying data without state validation.
    pub fn data(&self) -> &HandoffData {
        &self.data
    }

    /// Get the current status.
    pub fn status(&self) -> HandoffStatus {
        self.status
    }
}

/// Errors when transitioning handoff states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffStateError {
    /// Handoff is not in the expected state.
    WrongState {
        handoff_id: HandoffId,
        expected: HandoffStatus,
        actual: HandoffStatus,
    },
}

impl fmt::Display for HandoffStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandoffStateError::WrongState {
                handoff_id,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Handoff {} is in state {} but expected {}",
                    handoff_id, actual, expected
                )
            }
        }
    }
}

impl std::error::Error for HandoffStateError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntityIdType;
    use chrono::Utc;

    fn make_handoff_data() -> HandoffData {
        let now = Utc::now();
        HandoffData {
            handoff_id: HandoffId::now_v7(),
            tenant_id: TenantId::now_v7(),
            from_agent_id: AgentId::now_v7(),
            to_agent_id: AgentId::now_v7(),
            trajectory_id: TrajectoryId::now_v7(),
            scope_id: ScopeId::now_v7(),
            reason: "Need specialist".to_string(),
            context_snapshot: vec![1, 2, 3],
            created_at: now,
            accepted_at: None,
            completed_at: None,
            rejection_reason: None,
        }
    }

    #[test]
    fn test_handoff_status_roundtrip() {
        for status in [
            HandoffStatus::Initiated,
            HandoffStatus::Accepted,
            HandoffStatus::Rejected,
            HandoffStatus::Completed,
        ] {
            let db_str = status.as_db_str();
            let parsed = HandoffStatus::from_db_str(db_str).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_handoff_accept_complete() {
        let now = Utc::now();
        let data = make_handoff_data();
        let handoff = Handoff::<Initiated>::new(data);

        let accepted = handoff.accept(now);
        assert_eq!(accepted.accepted_at(), now);

        let completed = accepted.complete(now);
        assert_eq!(completed.completed_at(), now);
    }

    #[test]
    fn test_handoff_reject() {
        let data = make_handoff_data();
        let handoff = Handoff::<Initiated>::new(data);

        let rejected = handoff.reject("Not available".to_string());
        assert_eq!(rejected.rejection_reason(), "Not available");
    }

    #[test]
    fn test_stored_handoff_conversion() {
        let data = make_handoff_data();
        let stored = StoredHandoff {
            data: data.clone(),
            status: HandoffStatus::Initiated,
        };

        let initiated = stored.into_initiated().unwrap();
        assert_eq!(initiated.handoff_id(), data.handoff_id);
    }

    #[test]
    fn test_stored_handoff_wrong_state() {
        let data = make_handoff_data();
        let stored = StoredHandoff {
            data,
            status: HandoffStatus::Accepted,
        };

        assert!(matches!(
            stored.into_initiated(),
            Err(HandoffStateError::WrongState { .. })
        ));
    }
}
