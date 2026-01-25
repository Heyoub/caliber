//! Agent types for multi-agent coordination.
//!
//! This module contains agent identity, memory access control, and message types
//! that were consolidated from caliber-agents into caliber-core.

use crate::{
    identity::EntityIdType, AbstractionLevel, ActionId, AgentId, BeliefId, GoalId,
    LearningId, ObservationId, PlanId, StepId, Timestamp, TrajectoryId,
};
use crate::event::EvidenceRef;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

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
// CONFLICT RESOLUTION
// ============================================================================

/// Strategy for resolving conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ConflictResolution {
    /// Last write wins
    #[default]
    LastWriteWins,
    /// Highest confidence wins
    HighestConfidence,
    /// Escalate to user/admin
    Escalate,
}

// ============================================================================
// MEMORY REGION CONFIG
// ============================================================================

/// Configuration for a memory region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryRegionConfig {
    /// Unique identifier for this region
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub region_id: Uuid,
    /// Type of region
    pub region_type: MemoryRegion,
    /// Agent that owns this region
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub owner_agent_id: Uuid,
    /// Team this region belongs to (if applicable)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub team_id: Option<Uuid>,

    /// Agents with read access
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub readers: Vec<Uuid>,
    /// Agents with write access
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub writers: Vec<Uuid>,

    /// Whether writes require a lock
    pub require_lock: bool,
    /// How to resolve conflicts
    pub conflict_resolution: ConflictResolution,
    /// Whether to track versions
    pub version_tracking: bool,
}

impl MemoryRegionConfig {
    /// Create a new private region.
    pub fn private(owner_agent_id: Uuid) -> Self {
        Self {
            region_id: Uuid::now_v7(),
            region_type: MemoryRegion::Private,
            owner_agent_id,
            team_id: None,
            readers: vec![owner_agent_id],
            writers: vec![owner_agent_id],
            require_lock: false,
            conflict_resolution: ConflictResolution::LastWriteWins,
            version_tracking: false,
        }
    }

    /// Create a new team region.
    pub fn team(owner_agent_id: Uuid, team_id: Uuid) -> Self {
        Self {
            region_id: Uuid::now_v7(),
            region_type: MemoryRegion::Team,
            owner_agent_id,
            team_id: Some(team_id),
            readers: Vec::new(),
            writers: Vec::new(),
            require_lock: false,
            conflict_resolution: ConflictResolution::LastWriteWins,
            version_tracking: true,
        }
    }

    /// Create a new public region.
    pub fn public(owner_agent_id: Uuid) -> Self {
        Self {
            region_id: Uuid::now_v7(),
            region_type: MemoryRegion::Public,
            owner_agent_id,
            team_id: None,
            readers: Vec::new(),
            writers: vec![owner_agent_id],
            require_lock: false,
            conflict_resolution: ConflictResolution::LastWriteWins,
            version_tracking: false,
        }
    }

    /// Create a new collaborative region.
    pub fn collaborative(owner_agent_id: Uuid) -> Self {
        Self {
            region_id: Uuid::now_v7(),
            region_type: MemoryRegion::Collaborative,
            owner_agent_id,
            team_id: None,
            readers: Vec::new(),
            writers: Vec::new(),
            require_lock: true,
            conflict_resolution: ConflictResolution::Escalate,
            version_tracking: true,
        }
    }

    /// Add a reader to the region.
    pub fn add_reader(&mut self, agent_id: Uuid) {
        if !self.readers.contains(&agent_id) {
            self.readers.push(agent_id);
        }
    }

    /// Add a writer to the region.
    pub fn add_writer(&mut self, agent_id: Uuid) {
        if !self.writers.contains(&agent_id) {
            self.writers.push(agent_id);
        }
    }

    /// Check if an agent can read from this region.
    pub fn can_read(&self, agent_id: Uuid) -> bool {
        match self.region_type {
            MemoryRegion::Private => agent_id == self.owner_agent_id,
            MemoryRegion::Team => {
                agent_id == self.owner_agent_id || self.readers.contains(&agent_id)
            }
            MemoryRegion::Public | MemoryRegion::Collaborative => true,
        }
    }

    /// Check if an agent can write to this region.
    pub fn can_write(&self, agent_id: Uuid) -> bool {
        match self.region_type {
            MemoryRegion::Private => agent_id == self.owner_agent_id,
            MemoryRegion::Team => {
                agent_id == self.owner_agent_id || self.writers.contains(&agent_id)
            }
            MemoryRegion::Public => agent_id == self.owner_agent_id,
            MemoryRegion::Collaborative => true,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================


// ============================================================================
// BDI PRIMITIVES: GOAL SYSTEM (Phase 2.2)
// ============================================================================

/// Status of an agent goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum GoalStatus {
    /// Awaiting activation
    #[default]
    Pending,
    /// Currently being pursued
    Active,
    /// Successfully completed
    Achieved,
    /// Permanently failed
    Failed,
    /// Intentionally dropped
    Abandoned,
    /// Temporarily paused
    Suspended,
}

/// Type of goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum GoalType {
    /// End goal - ultimate objective
    #[default]
    Terminal,
    /// Decomposed from parent goal
    Subgoal,
    /// Progress checkpoint
    Milestone,
    /// Constraint that must always hold
    Invariant,
}

/// A measurable criterion for goal success.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SuccessCriterion {
    /// Description of the criterion
    pub description: String,
    /// Whether this criterion is measurable
    pub measurable: bool,
    /// Target value (if measurable)
    pub target_value: Option<String>,
    /// Current value (if measured)
    pub current_value: Option<String>,
    /// Whether this criterion is satisfied
    pub satisfied: bool,
}

impl SuccessCriterion {
    /// Create a new success criterion.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            measurable: false,
            target_value: None,
            current_value: None,
            satisfied: false,
        }
    }

    /// Make this criterion measurable.
    pub fn measurable(mut self, target: impl Into<String>) -> Self {
        self.measurable = true;
        self.target_value = Some(target.into());
        self
    }

    /// Update the current value.
    pub fn update(&mut self, value: impl Into<String>) {
        self.current_value = Some(value.into());
    }

    /// Mark as satisfied.
    pub fn satisfy(&mut self) {
        self.satisfied = true;
    }
}

/// A goal that an agent is pursuing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentGoal {
    /// Unique identifier for this goal
    pub goal_id: GoalId,
    /// Agent pursuing this goal
    pub agent_id: AgentId,
    /// Trajectory this goal belongs to (if any)
    pub trajectory_id: Option<TrajectoryId>,
    /// Description of the goal
    pub description: String,
    /// Type of goal
    pub goal_type: GoalType,
    /// Current status
    pub status: GoalStatus,
    /// Criteria for success
    pub success_criteria: Vec<SuccessCriterion>,
    /// Priority (higher = more important)
    pub priority: i32,
    /// Deadline (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub deadline: Option<Timestamp>,
    /// Parent goal (for subgoals)
    pub parent_goal_id: Option<GoalId>,
    /// Child goals (subgoals)
    pub child_goal_ids: Vec<GoalId>,
    /// When this goal was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// When work started
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub started_at: Option<Timestamp>,
    /// When completed
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    /// Reason for failure (if failed)
    pub failure_reason: Option<String>,
}

impl AgentGoal {
    /// Create a new goal.
    pub fn new(
        agent_id: AgentId,
        description: impl Into<String>,
        goal_type: GoalType,
    ) -> Self {
        Self {
            goal_id: GoalId::now_v7(),
            agent_id,
            trajectory_id: None,
            description: description.into(),
            goal_type,
            status: GoalStatus::Pending,
            success_criteria: Vec::new(),
            priority: 0,
            deadline: None,
            parent_goal_id: None,
            child_goal_ids: Vec::new(),
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            failure_reason: None,
        }
    }

    /// Set trajectory.
    pub fn with_trajectory(mut self, trajectory_id: TrajectoryId) -> Self {
        self.trajectory_id = Some(trajectory_id);
        self
    }

    /// Set parent goal.
    pub fn with_parent(mut self, parent_id: GoalId) -> Self {
        self.parent_goal_id = Some(parent_id);
        self
    }

    /// Add a success criterion.
    pub fn with_criterion(mut self, criterion: SuccessCriterion) -> Self {
        self.success_criteria.push(criterion);
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set deadline.
    pub fn with_deadline(mut self, deadline: Timestamp) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Start pursuing this goal.
    pub fn start(&mut self) {
        self.status = GoalStatus::Active;
        self.started_at = Some(chrono::Utc::now());
    }

    /// Mark as achieved.
    pub fn achieve(&mut self) {
        self.status = GoalStatus::Achieved;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark as failed.
    pub fn fail(&mut self, reason: impl Into<String>) {
        self.status = GoalStatus::Failed;
        self.failure_reason = Some(reason.into());
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Check if all criteria are satisfied.
    pub fn all_criteria_satisfied(&self) -> bool {
        self.success_criteria.iter().all(|c| c.satisfied)
    }
}

// ============================================================================
// BDI PRIMITIVES: PLAN SYSTEM (Phase 2.3)
// ============================================================================

/// Status of a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum PlanStatus {
    /// Being formulated
    #[default]
    Draft,
    /// Approved, awaiting execution
    Ready,
    /// Actively executing
    InProgress,
    /// All steps done
    Completed,
    /// Execution failed
    Failed,
    /// Cancelled
    Abandoned,
}

/// Status of a plan step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum StepStatus {
    /// Not yet started
    #[default]
    Pending,
    /// Preconditions met
    Ready,
    /// Currently executing
    InProgress,
    /// Successfully finished
    Completed,
    /// Execution failed
    Failed,
    /// Intentionally skipped
    Skipped,
    /// Waiting on dependency
    Blocked,
}

/// A single step in a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PlanStep {
    /// Unique identifier for this step
    pub step_id: StepId,
    /// Order in the plan
    pub index: i32,
    /// Description of what this step does
    pub description: String,
    /// Type of action required
    pub action_type: ActionType,
    /// What must be true before this step
    pub preconditions: Vec<String>,
    /// What will be true after this step
    pub postconditions: Vec<String>,
    /// Steps this depends on
    pub depends_on: Vec<StepId>,
    /// Estimated token cost
    pub estimated_tokens: Option<i32>,
    /// Current status
    pub status: StepStatus,
}

impl PlanStep {
    /// Create a new plan step.
    pub fn new(index: i32, description: impl Into<String>, action_type: ActionType) -> Self {
        Self {
            step_id: StepId::now_v7(),
            index,
            description: description.into(),
            action_type,
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            depends_on: Vec::new(),
            estimated_tokens: None,
            status: StepStatus::Pending,
        }
    }

    /// Add a precondition.
    pub fn with_precondition(mut self, condition: impl Into<String>) -> Self {
        self.preconditions.push(condition.into());
        self
    }

    /// Add a postcondition.
    pub fn with_postcondition(mut self, condition: impl Into<String>) -> Self {
        self.postconditions.push(condition.into());
        self
    }

    /// Add a dependency.
    pub fn depends_on(mut self, step_id: StepId) -> Self {
        self.depends_on.push(step_id);
        self
    }
}

/// Cost estimate for a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PlanCost {
    /// Estimated token usage
    pub estimated_tokens: i32,
    /// Estimated duration in milliseconds
    pub estimated_duration_ms: i64,
    /// Monetary cost in USD (if applicable)
    pub monetary_cost_usd: Option<f64>,
}

impl PlanCost {
    /// Create a new cost estimate.
    pub fn new(tokens: i32, duration_ms: i64) -> Self {
        Self {
            estimated_tokens: tokens,
            estimated_duration_ms: duration_ms,
            monetary_cost_usd: None,
        }
    }

    /// Add monetary cost.
    pub fn with_monetary_cost(mut self, cost_usd: f64) -> Self {
        self.monetary_cost_usd = Some(cost_usd);
        self
    }
}

/// A plan to achieve a goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentPlan {
    /// Unique identifier for this plan
    pub plan_id: PlanId,
    /// Agent that created this plan
    pub agent_id: AgentId,
    /// Goal this plan is for
    pub goal_id: GoalId,
    /// Description of the plan
    pub description: String,
    /// Current status
    pub status: PlanStatus,
    /// Steps in the plan
    pub steps: Vec<PlanStep>,
    /// Estimated cost
    pub estimated_cost: Option<PlanCost>,
    /// Actual cost (after execution)
    pub actual_cost: Option<PlanCost>,
    /// When this plan was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// When execution started
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub started_at: Option<Timestamp>,
    /// When completed
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
}

impl AgentPlan {
    /// Create a new plan.
    pub fn new(agent_id: AgentId, goal_id: GoalId, description: impl Into<String>) -> Self {
        Self {
            plan_id: PlanId::now_v7(),
            agent_id,
            goal_id,
            description: description.into(),
            status: PlanStatus::Draft,
            steps: Vec::new(),
            estimated_cost: None,
            actual_cost: None,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Add a step to the plan.
    pub fn add_step(&mut self, step: PlanStep) {
        self.steps.push(step);
    }

    /// Set estimated cost.
    pub fn with_estimated_cost(mut self, cost: PlanCost) -> Self {
        self.estimated_cost = Some(cost);
        self
    }

    /// Mark as ready for execution.
    pub fn ready(&mut self) {
        self.status = PlanStatus::Ready;
    }

    /// Start execution.
    pub fn start(&mut self) {
        self.status = PlanStatus::InProgress;
        self.started_at = Some(chrono::Utc::now());
    }

    /// Mark as completed.
    pub fn complete(&mut self, actual_cost: Option<PlanCost>) {
        self.status = PlanStatus::Completed;
        self.actual_cost = actual_cost;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Get next pending step.
    pub fn next_step(&self) -> Option<&PlanStep> {
        self.steps.iter().find(|s| s.status == StepStatus::Pending || s.status == StepStatus::Ready)
    }
}

// ============================================================================
// BDI PRIMITIVES: ACTION SYSTEM (Phase 2.4)
// ============================================================================

/// Type of action an agent can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ActionType {
    /// Direct system operation
    #[default]
    Operation,
    /// External tool invocation
    ToolCall,
    /// LLM inference
    ModelQuery,
    /// Deliberation/choice
    Decision,
    /// Message another agent
    Communication,
    /// Sense/perceive environment
    Observation,
    /// Read/write memory
    MemoryAccess,
}

/// Status of an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ActionStatus {
    /// Not yet started
    #[default]
    Pending,
    /// Currently executing
    InProgress,
    /// Successfully finished
    Completed,
    /// Execution failed
    Failed,
    /// Being retried
    Retrying,
    /// Cancelled
    Cancelled,
}

/// Backoff strategy for retries.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum BackoffStrategy {
    /// No backoff
    #[default]
    None,
    /// Fixed delay
    Fixed {
        delay_ms: i64,
    },
    /// Linear backoff
    Linear {
        base_ms: i64,
        increment_ms: i64,
    },
    /// Exponential backoff
    Exponential {
        base_ms: i64,
        multiplier: f64,
        max_ms: i64,
    },
}

impl BackoffStrategy {
    /// Calculate delay for a given attempt number.
    pub fn delay_for_attempt(&self, attempt: i32) -> i64 {
        match self {
            BackoffStrategy::None => 0,
            BackoffStrategy::Fixed { delay_ms } => *delay_ms,
            BackoffStrategy::Linear { base_ms, increment_ms } => {
                base_ms + (attempt as i64 * increment_ms)
            }
            BackoffStrategy::Exponential { base_ms, multiplier, max_ms } => {
                let delay = (*base_ms as f64) * multiplier.powi(attempt);
                (delay as i64).min(*max_ms)
            }
        }
    }
}

/// Policy for retrying failed actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RetryPolicy {
    /// Maximum number of attempts
    pub max_attempts: i32,
    /// Backoff strategy
    pub backoff: BackoffStrategy,
    /// Timeout per attempt in milliseconds
    pub timeout_per_attempt_ms: i64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                base_ms: 100,
                multiplier: 2.0,
                max_ms: 10_000,
            },
            timeout_per_attempt_ms: 30_000,
        }
    }
}

/// An action taken by an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentAction {
    /// Unique identifier for this action
    pub action_id: ActionId,
    /// Agent performing this action
    pub agent_id: AgentId,
    /// Plan step this action is part of (if any)
    pub step_id: Option<StepId>,
    /// Type of action
    pub action_type: ActionType,
    /// Description of the action
    pub description: String,
    /// Action parameters (JSON)
    pub parameters: Option<serde_json::Value>,
    /// Retry policy
    pub retry_policy: Option<RetryPolicy>,
    /// Timeout in milliseconds
    pub timeout_ms: Option<i64>,
    /// Current status
    pub status: ActionStatus,
    /// Number of attempts made
    pub attempt_count: i32,
    /// When this action was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// When execution started
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub started_at: Option<Timestamp>,
    /// When completed
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
}

impl AgentAction {
    /// Create a new action.
    pub fn new(
        agent_id: AgentId,
        action_type: ActionType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            action_id: ActionId::now_v7(),
            agent_id,
            step_id: None,
            action_type,
            description: description.into(),
            parameters: None,
            retry_policy: None,
            timeout_ms: None,
            status: ActionStatus::Pending,
            attempt_count: 0,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Set plan step.
    pub fn with_step(mut self, step_id: StepId) -> Self {
        self.step_id = Some(step_id);
        self
    }

    /// Set parameters.
    pub fn with_parameters(mut self, params: serde_json::Value) -> Self {
        self.parameters = Some(params);
        self
    }

    /// Set retry policy.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout_ms: i64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Start execution.
    pub fn start(&mut self) {
        self.status = ActionStatus::InProgress;
        self.attempt_count += 1;
        if self.started_at.is_none() {
            self.started_at = Some(chrono::Utc::now());
        }
    }

    /// Mark as completed.
    pub fn complete(&mut self) {
        self.status = ActionStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark as failed.
    pub fn fail(&mut self) {
        self.status = ActionStatus::Failed;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Check if retry is allowed.
    pub fn can_retry(&self) -> bool {
        if let Some(policy) = &self.retry_policy {
            self.attempt_count < policy.max_attempts
        } else {
            false
        }
    }
}

// ============================================================================
// BDI PRIMITIVES: BELIEF SYSTEM (Phase 2.5)
// ============================================================================

/// Source of a belief.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum BeliefSource {
    /// Direct sensing/perception
    #[default]
    Observation,
    /// Logical deduction
    Inference,
    /// From another agent
    Communication,
    /// From persisted knowledge
    MemoryRecall,
    /// Assumed without proof
    Assumption,
    /// Explicitly provided by user
    UserProvided,
}

/// Type of belief.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum BeliefType {
    /// Known to be true
    #[default]
    Fact,
    /// Suspected to be true
    Hypothesis,
    /// Unknown, needs resolution
    Uncertainty,
}

/// A belief held by an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Belief {
    /// Unique identifier for this belief
    pub belief_id: BeliefId,
    /// Agent holding this belief
    pub agent_id: AgentId,
    /// Type of belief
    pub belief_type: BeliefType,
    /// Content of the belief
    pub content: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// Source of the belief
    pub source: BeliefSource,
    /// Evidence supporting this belief
    pub evidence_refs: Vec<EvidenceRef>,
    /// When this belief was formed
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// When last updated
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub updated_at: Timestamp,
    /// Belief that supersedes this one (if any)
    pub superseded_by: Option<BeliefId>,
}

impl Belief {
    /// Create a new belief.
    pub fn new(
        agent_id: AgentId,
        content: impl Into<String>,
        belief_type: BeliefType,
        source: BeliefSource,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            belief_id: BeliefId::now_v7(),
            agent_id,
            belief_type,
            content: content.into(),
            confidence: 1.0,
            source,
            evidence_refs: Vec::new(),
            created_at: now,
            updated_at: now,
            superseded_by: None,
        }
    }

    /// Set confidence level.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add evidence reference.
    pub fn with_evidence(mut self, evidence: EvidenceRef) -> Self {
        self.evidence_refs.push(evidence);
        self
    }

    /// Update confidence.
    pub fn update_confidence(&mut self, confidence: f32) {
        self.confidence = confidence.clamp(0.0, 1.0);
        self.updated_at = chrono::Utc::now();
    }

    /// Mark as superseded.
    pub fn supersede(&mut self, new_belief_id: BeliefId) {
        self.superseded_by = Some(new_belief_id);
        self.updated_at = chrono::Utc::now();
    }

    /// Check if this belief is still active.
    pub fn is_active(&self) -> bool {
        self.superseded_by.is_none()
    }
}

/// Collection of an agent's beliefs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentBeliefs {
    /// Agent these beliefs belong to
    pub agent_id: AgentId,
    /// Beliefs classified as facts
    pub facts: Vec<Belief>,
    /// Beliefs classified as hypotheses
    pub hypotheses: Vec<Belief>,
    /// Beliefs classified as uncertainties
    pub uncertainties: Vec<Belief>,
    /// When last updated
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub last_updated: Timestamp,
}

impl AgentBeliefs {
    /// Create a new empty belief set.
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            facts: Vec::new(),
            hypotheses: Vec::new(),
            uncertainties: Vec::new(),
            last_updated: chrono::Utc::now(),
        }
    }

    /// Add a belief.
    pub fn add(&mut self, belief: Belief) {
        match belief.belief_type {
            BeliefType::Fact => self.facts.push(belief),
            BeliefType::Hypothesis => self.hypotheses.push(belief),
            BeliefType::Uncertainty => self.uncertainties.push(belief),
        }
        self.last_updated = chrono::Utc::now();
    }

    /// Get all active beliefs.
    pub fn active_beliefs(&self) -> impl Iterator<Item = &Belief> {
        self.facts
            .iter()
            .chain(self.hypotheses.iter())
            .chain(self.uncertainties.iter())
            .filter(|b| b.is_active())
    }
}

// ============================================================================
// BDI PRIMITIVES: OBSERVATION & LEARNING (Phase 2.6)
// ============================================================================

/// An observation made by an agent after an action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentObservation {
    /// Unique identifier for this observation
    pub observation_id: ObservationId,
    /// Agent that made the observation
    pub agent_id: AgentId,
    /// Action that led to this observation
    pub action_id: ActionId,
    /// Whether the action succeeded
    pub success: bool,
    /// Result data (JSON)
    pub result: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: i64,
    /// Tokens used (if applicable)
    pub tokens_used: Option<i32>,
    /// Cost in USD (if applicable)
    pub cost_usd: Option<f64>,
    /// When this observation was made
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub timestamp: Timestamp,
    /// Beliefs created/updated from this observation
    pub belief_updates: Vec<BeliefId>,
    /// Learnings extracted
    pub learnings: Vec<Learning>,
}

impl AgentObservation {
    /// Create a new observation.
    pub fn new(agent_id: AgentId, action_id: ActionId, success: bool, duration_ms: i64) -> Self {
        Self {
            observation_id: ObservationId::now_v7(),
            agent_id,
            action_id,
            success,
            result: None,
            error: None,
            duration_ms,
            tokens_used: None,
            cost_usd: None,
            timestamp: chrono::Utc::now(),
            belief_updates: Vec::new(),
            learnings: Vec::new(),
        }
    }

    /// Set result.
    pub fn with_result(mut self, result: serde_json::Value) -> Self {
        self.result = Some(result);
        self
    }

    /// Set error.
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Set token usage.
    pub fn with_tokens(mut self, tokens: i32) -> Self {
        self.tokens_used = Some(tokens);
        self
    }

    /// Set cost.
    pub fn with_cost(mut self, cost_usd: f64) -> Self {
        self.cost_usd = Some(cost_usd);
        self
    }

    /// Add belief update.
    pub fn add_belief_update(&mut self, belief_id: BeliefId) {
        self.belief_updates.push(belief_id);
    }

    /// Add learning.
    pub fn add_learning(&mut self, learning: Learning) {
        self.learnings.push(learning);
    }
}

/// Type of learning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LearningType {
    /// New fact discovered
    #[default]
    FactualUpdate,
    /// Pattern observed
    PatternRecognition,
    /// Improvement to approach
    StrategyRefinement,
    /// Correction of wrong belief
    ErrorCorrection,
    /// New capability identified
    CapabilityUpdate,
}

/// A learning extracted from an observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Learning {
    /// Unique identifier for this learning
    pub learning_id: LearningId,
    /// Observation this came from
    pub observation_id: ObservationId,
    /// Type of learning
    pub learning_type: LearningType,
    /// Content of the learning
    pub content: String,
    /// Level of abstraction
    pub abstraction_level: AbstractionLevel,
    /// Where this learning applies
    pub applicability: Option<String>,
    /// Confidence in this learning
    pub confidence: f32,
    /// When this was learned
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
}

impl Learning {
    /// Create a new learning.
    pub fn new(
        observation_id: ObservationId,
        learning_type: LearningType,
        content: impl Into<String>,
    ) -> Self {
        Self {
            learning_id: LearningId::now_v7(),
            observation_id,
            learning_type,
            content: content.into(),
            abstraction_level: AbstractionLevel::Raw,
            applicability: None,
            confidence: 1.0,
            created_at: chrono::Utc::now(),
        }
    }

    /// Set abstraction level.
    pub fn with_abstraction(mut self, level: AbstractionLevel) -> Self {
        self.abstraction_level = level;
        self
    }

    /// Set applicability.
    pub fn with_applicability(mut self, applicability: impl Into<String>) -> Self {
        self.applicability = Some(applicability.into());
        self
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

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

}
