//! Agent types for multi-agent coordination.
//!
//! This module contains agent identity, memory access control, and message types
//! that were consolidated from caliber-agents into caliber-core.

use crate::{ArtifactId, LockMode, NoteId, Timestamp};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ============================================================================
// MESSAGE TYPES (from caliber-agents)
// ============================================================================

/// Type of agent message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum MessageType {
    /// Task delegation request
    TaskDelegation,
    /// Result of a delegated task
    TaskResult,
    /// Request for context from another agent
    ContextRequest,
    /// Sharing context with another agent
    ContextShare,
    /// Coordination signal (e.g., ready, waiting)
    CoordinationSignal,
    /// Handoff request
    Handoff,
    /// Interrupt signal
    Interrupt,
    /// Heartbeat/keepalive
    Heartbeat,
}

impl MessageType {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            MessageType::TaskDelegation => "TaskDelegation",
            MessageType::TaskResult => "TaskResult",
            MessageType::ContextRequest => "ContextRequest",
            MessageType::ContextShare => "ContextShare",
            MessageType::CoordinationSignal => "CoordinationSignal",
            MessageType::Handoff => "Handoff",
            MessageType::Interrupt => "Interrupt",
            MessageType::Heartbeat => "Heartbeat",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, MessageTypeParseError> {
        match s.to_lowercase().replace('_', "").as_str() {
            "taskdelegation" => Ok(MessageType::TaskDelegation),
            "taskresult" => Ok(MessageType::TaskResult),
            "contextrequest" => Ok(MessageType::ContextRequest),
            "contextshare" => Ok(MessageType::ContextShare),
            "coordinationsignal" => Ok(MessageType::CoordinationSignal),
            "handoff" => Ok(MessageType::Handoff),
            "interrupt" => Ok(MessageType::Interrupt),
            "heartbeat" => Ok(MessageType::Heartbeat),
            _ => Err(MessageTypeParseError(s.to_string())),
        }
    }
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for MessageType {
    type Err = MessageTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid message type string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageTypeParseError(pub String);

impl fmt::Display for MessageTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid message type: {}", self.0)
    }
}

impl std::error::Error for MessageTypeParseError {}

/// Priority level for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum MessagePriority {
    /// Low priority - can be delayed
    Low,
    /// Normal priority
    #[default]
    Normal,
    /// High priority - should be processed soon
    High,
    /// Critical - must be processed immediately
    Critical,
}

impl MessagePriority {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            MessagePriority::Low => "Low",
            MessagePriority::Normal => "Normal",
            MessagePriority::High => "High",
            MessagePriority::Critical => "Critical",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, MessagePriorityParseError> {
        match s.to_lowercase().as_str() {
            "low" => Ok(MessagePriority::Low),
            "normal" => Ok(MessagePriority::Normal),
            "high" => Ok(MessagePriority::High),
            "critical" => Ok(MessagePriority::Critical),
            _ => Err(MessagePriorityParseError(s.to_string())),
        }
    }
}

impl fmt::Display for MessagePriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for MessagePriority {
    type Err = MessagePriorityParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid message priority string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagePriorityParseError(pub String);

impl fmt::Display for MessagePriorityParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid message priority: {}", self.0)
    }
}

impl std::error::Error for MessagePriorityParseError {}

// ============================================================================
// MEMORY REGIONS AND ACCESS CONTROL
// ============================================================================

/// Permission scope for memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum PermissionScope {
    /// Only own resources
    Own,
    /// Resources belonging to same team
    Team,
    /// All resources (global access)
    Global,
}

/// Type of memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum MemoryRegion {
    /// Only owning agent can access
    Private,
    /// Agents in same team can access
    Team,
    /// Any agent can read, owner can write
    Public,
    /// Any agent can read/write with coordination
    Collaborative,
}

/// A single memory permission entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryPermission {
    /// Type of memory (e.g., "artifact", "note", "trajectory")
    pub memory_type: String,
    /// Scope of the permission
    pub scope: PermissionScope,
    /// Optional filter expression (serialized)
    pub filter: Option<String>,
}

/// Memory access configuration for an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryAccess {
    /// Read permissions
    pub read: Vec<MemoryPermission>,
    /// Write permissions
    pub write: Vec<MemoryPermission>,
}

impl Default for MemoryAccess {
    fn default() -> Self {
        Self {
            read: vec![MemoryPermission {
                memory_type: "*".to_string(),
                scope: PermissionScope::Own,
                filter: None,
            }],
            write: vec![MemoryPermission {
                memory_type: "*".to_string(),
                scope: PermissionScope::Own,
                filter: None,
            }],
        }
    }
}

// ============================================================================
// HANDOFF REASON
// ============================================================================

/// Reason for a handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum HandoffReason {
    /// Current agent lacks required capability
    CapabilityMismatch,
    /// Load balancing across agents
    LoadBalancing,
    /// Task requires specialized agent
    Specialization,
    /// Escalation to supervisor
    Escalation,
    /// Agent timed out
    Timeout,
    /// Agent failed
    Failure,
    /// Scheduled handoff
    Scheduled,
}

impl HandoffReason {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            HandoffReason::CapabilityMismatch => "CapabilityMismatch",
            HandoffReason::LoadBalancing => "LoadBalancing",
            HandoffReason::Specialization => "Specialization",
            HandoffReason::Escalation => "Escalation",
            HandoffReason::Timeout => "Timeout",
            HandoffReason::Failure => "Failure",
            HandoffReason::Scheduled => "Scheduled",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, HandoffReasonParseError> {
        match s.to_lowercase().replace('_', "").as_str() {
            "capabilitymismatch" => Ok(HandoffReason::CapabilityMismatch),
            "loadbalancing" => Ok(HandoffReason::LoadBalancing),
            "specialization" => Ok(HandoffReason::Specialization),
            "escalation" => Ok(HandoffReason::Escalation),
            "timeout" => Ok(HandoffReason::Timeout),
            "failure" | "failed" => Ok(HandoffReason::Failure),
            "scheduled" => Ok(HandoffReason::Scheduled),
            _ => Err(HandoffReasonParseError(s.to_string())),
        }
    }
}

impl fmt::Display for HandoffReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for HandoffReason {
    type Err = HandoffReasonParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid handoff reason string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffReasonParseError(pub String);

impl fmt::Display for HandoffReasonParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid handoff reason: {}", self.0)
    }
}

impl std::error::Error for HandoffReasonParseError {}

// ============================================================================
// CONFLICT TYPES
// ============================================================================

/// Type of conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ConflictType {
    /// Two agents wrote to the same resource concurrently
    ConcurrentWrite,
    /// Two facts contradict each other
    ContradictingFact,
    /// Two decisions are incompatible
    IncompatibleDecision,
    /// Two agents are contending for the same resource
    ResourceContention,
    /// Two agents have conflicting goals
    GoalConflict,
}

impl ConflictType {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            ConflictType::ConcurrentWrite => "ConcurrentWrite",
            ConflictType::ContradictingFact => "ContradictingFact",
            ConflictType::IncompatibleDecision => "IncompatibleDecision",
            ConflictType::ResourceContention => "ResourceContention",
            ConflictType::GoalConflict => "GoalConflict",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, ConflictTypeParseError> {
        match s.to_lowercase().replace('_', "").as_str() {
            "concurrentwrite" => Ok(ConflictType::ConcurrentWrite),
            "contradictingfact" => Ok(ConflictType::ContradictingFact),
            "incompatibledecision" => Ok(ConflictType::IncompatibleDecision),
            "resourcecontention" => Ok(ConflictType::ResourceContention),
            "goalconflict" => Ok(ConflictType::GoalConflict),
            _ => Err(ConflictTypeParseError(s.to_string())),
        }
    }
}

impl fmt::Display for ConflictType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for ConflictType {
    type Err = ConflictTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid conflict type string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictTypeParseError(pub String);

impl fmt::Display for ConflictTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid conflict type: {}", self.0)
    }
}

impl std::error::Error for ConflictTypeParseError {}

/// Status of a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ConflictStatus {
    /// Conflict has been detected
    Detected,
    /// Conflict is being resolved
    Resolving,
    /// Conflict has been resolved
    Resolved,
    /// Conflict has been escalated
    Escalated,
}

impl ConflictStatus {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            ConflictStatus::Detected => "Detected",
            ConflictStatus::Resolving => "Resolving",
            ConflictStatus::Resolved => "Resolved",
            ConflictStatus::Escalated => "Escalated",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, ConflictStatusParseError> {
        match s.to_lowercase().as_str() {
            "detected" => Ok(ConflictStatus::Detected),
            "resolving" => Ok(ConflictStatus::Resolving),
            "resolved" => Ok(ConflictStatus::Resolved),
            "escalated" => Ok(ConflictStatus::Escalated),
            _ => Err(ConflictStatusParseError(s.to_string())),
        }
    }

    /// Check if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, ConflictStatus::Resolved | ConflictStatus::Escalated)
    }
}

impl fmt::Display for ConflictStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for ConflictStatus {
    type Err = ConflictStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid conflict status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictStatusParseError(pub String);

impl fmt::Display for ConflictStatusParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid conflict status: {}", self.0)
    }
}

impl std::error::Error for ConflictStatusParseError {}

/// Strategy for resolving a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ResolutionStrategy {
    /// Last write wins
    LastWriteWins,
    /// First write wins
    FirstWriteWins,
    /// Highest confidence wins
    HighestConfidence,
    /// Merge the conflicting items
    Merge,
    /// Escalate to human or supervisor
    Escalate,
    /// Reject both items
    RejectBoth,
}

impl ResolutionStrategy {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            ResolutionStrategy::LastWriteWins => "LastWriteWins",
            ResolutionStrategy::FirstWriteWins => "FirstWriteWins",
            ResolutionStrategy::HighestConfidence => "HighestConfidence",
            ResolutionStrategy::Merge => "Merge",
            ResolutionStrategy::Escalate => "Escalate",
            ResolutionStrategy::RejectBoth => "RejectBoth",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, ResolutionStrategyParseError> {
        match s.to_lowercase().replace('_', "").as_str() {
            "lastwritewins" => Ok(ResolutionStrategy::LastWriteWins),
            "firstwritewins" => Ok(ResolutionStrategy::FirstWriteWins),
            "highestconfidence" => Ok(ResolutionStrategy::HighestConfidence),
            "merge" => Ok(ResolutionStrategy::Merge),
            "escalate" => Ok(ResolutionStrategy::Escalate),
            "rejectboth" => Ok(ResolutionStrategy::RejectBoth),
            _ => Err(ResolutionStrategyParseError(s.to_string())),
        }
    }
}

impl fmt::Display for ResolutionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for ResolutionStrategy {
    type Err = ResolutionStrategyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid resolution strategy string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionStrategyParseError(pub String);

impl fmt::Display for ResolutionStrategyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid resolution strategy: {}", self.0)
    }
}

impl std::error::Error for ResolutionStrategyParseError {}

// ============================================================================
// DELEGATION RESULT TYPES (from caliber-agents)
// ============================================================================

/// Result status of a delegation.
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

/// Result of a delegated task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationResult {
    /// Status of the result
    pub status: DelegationResultStatus,
    /// Artifacts produced by the task
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_artifacts: Vec<ArtifactId>,
    /// Notes produced by the task
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_notes: Vec<NoteId>,
    /// Summary of what was accomplished
    pub summary: String,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl DelegationResult {
    /// Create a successful result.
    pub fn success(summary: &str, artifacts: Vec<ArtifactId>) -> Self {
        Self {
            status: DelegationResultStatus::Success,
            produced_artifacts: artifacts,
            produced_notes: Vec::new(),
            summary: summary.to_string(),
            error: None,
        }
    }

    /// Create a partial result.
    pub fn partial(summary: &str, artifacts: Vec<ArtifactId>) -> Self {
        Self {
            status: DelegationResultStatus::Partial,
            produced_artifacts: artifacts,
            produced_notes: Vec::new(),
            summary: summary.to_string(),
            error: None,
        }
    }

    /// Create a failure result.
    pub fn failure(error: &str) -> Self {
        Self {
            status: DelegationResultStatus::Failure,
            produced_artifacts: Vec::new(),
            produced_notes: Vec::new(),
            summary: String::new(),
            error: Some(error.to_string()),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_roundtrip() {
        for mt in [
            MessageType::TaskDelegation,
            MessageType::TaskResult,
            MessageType::ContextRequest,
            MessageType::ContextShare,
            MessageType::CoordinationSignal,
            MessageType::Handoff,
            MessageType::Interrupt,
            MessageType::Heartbeat,
        ] {
            let s = mt.as_db_str();
            let parsed = MessageType::from_db_str(s).unwrap();
            assert_eq!(mt, parsed);
        }
    }

    #[test]
    fn test_message_priority_roundtrip() {
        for mp in [
            MessagePriority::Low,
            MessagePriority::Normal,
            MessagePriority::High,
            MessagePriority::Critical,
        ] {
            let s = mp.as_db_str();
            let parsed = MessagePriority::from_db_str(s).unwrap();
            assert_eq!(mp, parsed);
        }
    }

    #[test]
    fn test_conflict_status_terminal() {
        assert!(!ConflictStatus::Detected.is_terminal());
        assert!(!ConflictStatus::Resolving.is_terminal());
        assert!(ConflictStatus::Resolved.is_terminal());
        assert!(ConflictStatus::Escalated.is_terminal());
    }

    #[test]
    fn test_delegation_result_constructors() {
        let success = DelegationResult::success("Done", vec![]);
        assert_eq!(success.status, DelegationResultStatus::Success);
        assert!(success.error.is_none());

        let failure = DelegationResult::failure("Oops");
        assert_eq!(failure.status, DelegationResultStatus::Failure);
        assert_eq!(failure.error, Some("Oops".to_string()));
    }
}
