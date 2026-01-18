//! CALIBER Agents - Multi-Agent Coordination
//!
//! Provides coordination primitives for multi-agent systems:
//! - Agent identity and registration
//! - Memory regions and access control
//! - Distributed locks
//! - Message passing
//! - Task delegation
//! - Agent handoffs
//! - Conflict detection and resolution

use caliber_core::{EntityId, Timestamp};
#[cfg(test)]
use caliber_core::{AgentError, CaliberError, CaliberResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export ConflictResolution from caliber-pcp for consistency
pub use caliber_pcp::ConflictResolution;

// ============================================================================
// AGENT IDENTITY (Task 10.1)
// ============================================================================

/// Agent status in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum AgentStatus {
    /// Agent is idle and available for work
    Idle,
    /// Agent is actively working on a task
    Active,
    /// Agent is blocked waiting for a resource
    Blocked,
    /// Agent has failed and needs recovery
    Failed,
}

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

/// An agent in the multi-agent system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Agent {
    /// Unique identifier for this agent
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub agent_id: EntityId,
    /// Type of agent (e.g., "coder", "reviewer", "planner")
    pub agent_type: String,
    /// Capabilities this agent has
    pub capabilities: Vec<String>,
    /// Memory access permissions
    pub memory_access: MemoryAccess,

    /// Current status
    pub status: AgentStatus,
    /// Current trajectory being worked on
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_trajectory_id: Option<EntityId>,
    /// Current scope being worked on
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_scope_id: Option<EntityId>,

    /// Agent types this agent can delegate to
    pub can_delegate_to: Vec<String>,
    /// Supervisor agent (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub reports_to: Option<EntityId>,

    /// When this agent was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// Last heartbeat timestamp
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub last_heartbeat: Timestamp,
}

impl Agent {
    /// Create a new agent.
    pub fn new(agent_type: &str, capabilities: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            agent_id: Uuid::now_v7(),
            agent_type: agent_type.to_string(),
            capabilities,
            memory_access: MemoryAccess::default(),
            status: AgentStatus::Idle,
            current_trajectory_id: None,
            current_scope_id: None,
            can_delegate_to: Vec::new(),
            reports_to: None,
            created_at: now,
            last_heartbeat: now,
        }
    }

    /// Set memory access permissions.
    pub fn with_memory_access(mut self, access: MemoryAccess) -> Self {
        self.memory_access = access;
        self
    }

    /// Set delegation targets.
    pub fn with_delegation_targets(mut self, targets: Vec<String>) -> Self {
        self.can_delegate_to = targets;
        self
    }

    /// Set supervisor.
    pub fn with_supervisor(mut self, supervisor_id: EntityId) -> Self {
        self.reports_to = Some(supervisor_id);
        self
    }

    /// Update heartbeat timestamp.
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
    }

    /// Check if agent has a specific capability.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// Check if agent can delegate to a specific agent type.
    pub fn can_delegate_to_type(&self, agent_type: &str) -> bool {
        self.can_delegate_to.iter().any(|t| t == agent_type)
    }
}


// ============================================================================
// MEMORY REGIONS (Task 10.2)
// ============================================================================

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

/// Configuration for a memory region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryRegionConfig {
    /// Unique identifier for this region
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub region_id: EntityId,
    /// Type of region
    pub region_type: MemoryRegion,
    /// Agent that owns this region
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub owner_agent_id: EntityId,
    /// Team this region belongs to (if applicable)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub team_id: Option<EntityId>,

    /// Agents with read access
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub readers: Vec<EntityId>,
    /// Agents with write access
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub writers: Vec<EntityId>,

    /// Whether writes require a lock
    pub require_lock: bool,
    /// How to resolve conflicts
    pub conflict_resolution: ConflictResolution,
    /// Whether to track versions
    pub version_tracking: bool,
}

impl MemoryRegionConfig {
    /// Create a new private region.
    pub fn private(owner_agent_id: EntityId) -> Self {
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
    pub fn team(owner_agent_id: EntityId, team_id: EntityId) -> Self {
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
    pub fn public(owner_agent_id: EntityId) -> Self {
        Self {
            region_id: Uuid::now_v7(),
            region_type: MemoryRegion::Public,
            owner_agent_id,
            team_id: None,
            readers: Vec::new(), // Empty means all can read
            writers: vec![owner_agent_id],
            require_lock: false,
            conflict_resolution: ConflictResolution::LastWriteWins,
            version_tracking: false,
        }
    }

    /// Create a new collaborative region.
    pub fn collaborative(owner_agent_id: EntityId) -> Self {
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
    pub fn add_reader(&mut self, agent_id: EntityId) {
        if !self.readers.contains(&agent_id) {
            self.readers.push(agent_id);
        }
    }

    /// Add a writer to the region.
    pub fn add_writer(&mut self, agent_id: EntityId) {
        if !self.writers.contains(&agent_id) {
            self.writers.push(agent_id);
        }
    }

    /// Check if an agent can read from this region.
    pub fn can_read(&self, agent_id: EntityId) -> bool {
        match self.region_type {
            MemoryRegion::Private => agent_id == self.owner_agent_id,
            MemoryRegion::Team => {
                agent_id == self.owner_agent_id || self.readers.contains(&agent_id)
            }
            MemoryRegion::Public | MemoryRegion::Collaborative => true,
        }
    }

    /// Check if an agent can write to this region.
    pub fn can_write(&self, agent_id: EntityId) -> bool {
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
// DISTRIBUTED LOCKS (Task 10.3)
// ============================================================================

/// Lock mode for distributed locks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LockMode {
    /// Exclusive lock - only one holder
    Exclusive,
    /// Shared lock - multiple readers allowed
    Shared,
}

/// A distributed lock on a resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DistributedLock {
    /// Unique identifier for this lock
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub lock_id: EntityId,
    /// Type of resource being locked
    pub resource_type: String,
    /// ID of the resource being locked
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub resource_id: EntityId,
    /// Agent holding the lock
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub holder_agent_id: EntityId,
    /// When the lock was acquired
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub acquired_at: Timestamp,
    /// When the lock expires
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub expires_at: Timestamp,
    /// Lock mode
    pub mode: LockMode,
}

impl DistributedLock {
    /// Create a new lock.
    pub fn new(
        resource_type: &str,
        resource_id: EntityId,
        holder_agent_id: EntityId,
        timeout_ms: i64,
        mode: LockMode,
    ) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::milliseconds(timeout_ms);

        Self {
            lock_id: Uuid::now_v7(),
            resource_type: resource_type.to_string(),
            resource_id,
            holder_agent_id,
            acquired_at: now,
            expires_at,
            mode,
        }
    }

    /// Check if the lock has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Extend the lock expiration.
    pub fn extend(&mut self, additional_ms: i64) {
        self.expires_at += chrono::Duration::milliseconds(additional_ms);
    }

    /// Compute a stable hash key for this lock (for advisory locks).
    pub fn compute_key(&self) -> i64 {
        compute_lock_key(&self.resource_type, self.resource_id)
    }
}

/// Compute a stable i64 key for advisory locks using FNV-1a hash.
/// FNV-1a is deterministic across Rust versions and compilations.
pub fn compute_lock_key(resource_type: &str, resource_id: EntityId) -> i64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    
    let mut hash = FNV_OFFSET_BASIS;
    
    // Hash resource type
    for byte in resource_type.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    
    // Hash resource ID bytes
    for byte in resource_id.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    
    hash as i64
}


// ============================================================================
// MESSAGE PASSING (Task 10.4, 10.5)
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

/// Priority level for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum MessagePriority {
    /// Low priority - can be delayed
    Low,
    /// Normal priority
    Normal,
    /// High priority - should be processed soon
    High,
    /// Critical - must be processed immediately
    Critical,
}

/// A message between agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentMessage {
    /// Unique identifier for this message
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub message_id: EntityId,

    /// Agent sending the message
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    /// Specific agent to receive (if targeted)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<EntityId>,
    /// Agent type to receive (for broadcast)
    pub to_agent_type: Option<String>,

    /// Type of message
    pub message_type: MessageType,
    /// Message payload (JSON serialized)
    pub payload: String,

    /// Related trajectory (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Related scope (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<EntityId>,
    /// Related artifacts (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifact_ids: Vec<EntityId>,

    /// When the message was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// When the message was delivered
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub delivered_at: Option<Timestamp>,
    /// When the message was acknowledged
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub acknowledged_at: Option<Timestamp>,

    /// Message priority
    pub priority: MessagePriority,
    /// When the message expires
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expires_at: Option<Timestamp>,
}

impl AgentMessage {
    /// Create a new message to a specific agent.
    pub fn to_agent(
        from_agent_id: EntityId,
        to_agent_id: EntityId,
        message_type: MessageType,
        payload: &str,
    ) -> Self {
        Self {
            message_id: Uuid::now_v7(),
            from_agent_id,
            to_agent_id: Some(to_agent_id),
            to_agent_type: None,
            message_type,
            payload: payload.to_string(),
            trajectory_id: None,
            scope_id: None,
            artifact_ids: Vec::new(),
            created_at: Utc::now(),
            delivered_at: None,
            acknowledged_at: None,
            priority: MessagePriority::Normal,
            expires_at: None,
        }
    }

    /// Create a new broadcast message to an agent type.
    pub fn to_type(
        from_agent_id: EntityId,
        to_agent_type: &str,
        message_type: MessageType,
        payload: &str,
    ) -> Self {
        Self {
            message_id: Uuid::now_v7(),
            from_agent_id,
            to_agent_id: None,
            to_agent_type: Some(to_agent_type.to_string()),
            message_type,
            payload: payload.to_string(),
            trajectory_id: None,
            scope_id: None,
            artifact_ids: Vec::new(),
            created_at: Utc::now(),
            delivered_at: None,
            acknowledged_at: None,
            priority: MessagePriority::Normal,
            expires_at: None,
        }
    }

    /// Set trajectory context.
    pub fn with_trajectory(mut self, trajectory_id: EntityId) -> Self {
        self.trajectory_id = Some(trajectory_id);
        self
    }

    /// Set scope context.
    pub fn with_scope(mut self, scope_id: EntityId) -> Self {
        self.scope_id = Some(scope_id);
        self
    }

    /// Set related artifacts.
    pub fn with_artifacts(mut self, artifact_ids: Vec<EntityId>) -> Self {
        self.artifact_ids = artifact_ids;
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set expiration.
    pub fn with_expiration(mut self, expires_at: Timestamp) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Mark as delivered.
    pub fn mark_delivered(&mut self) {
        self.delivered_at = Some(Utc::now());
    }

    /// Mark as acknowledged.
    pub fn mark_acknowledged(&mut self) {
        self.acknowledged_at = Some(Utc::now());
    }

    /// Check if message has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| Utc::now() > exp)
    }

    /// Check if message is for a specific agent.
    pub fn is_for_agent(&self, agent_id: EntityId, agent_type: &str) -> bool {
        // Check direct targeting
        if let Some(to_id) = self.to_agent_id {
            return to_id == agent_id;
        }

        // Check type targeting
        if let Some(ref to_type) = self.to_agent_type {
            return to_type == agent_type;
        }

        false
    }
}


// ============================================================================
// TASK DELEGATION (Task 10.5)
// ============================================================================

/// Status of a delegated task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum DelegationStatus {
    /// Task is pending acceptance
    Pending,
    /// Task has been accepted
    Accepted,
    /// Task was rejected
    Rejected,
    /// Task is in progress
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
}

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
    pub produced_artifacts: Vec<EntityId>,
    /// Notes produced by the task
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_notes: Vec<EntityId>,
    /// Summary of what was accomplished
    pub summary: String,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl DelegationResult {
    /// Create a successful result.
    pub fn success(summary: &str, artifacts: Vec<EntityId>) -> Self {
        Self {
            status: DelegationResultStatus::Success,
            produced_artifacts: artifacts,
            produced_notes: Vec::new(),
            summary: summary.to_string(),
            error: None,
        }
    }

    /// Create a partial result.
    pub fn partial(summary: &str, artifacts: Vec<EntityId>) -> Self {
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

/// A delegated task between agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegatedTask {
    /// Unique identifier for this delegation
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub delegation_id: EntityId,

    /// Agent delegating the task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub delegator_agent_id: EntityId,
    /// Specific agent to delegate to (if targeted)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub delegatee_agent_id: Option<EntityId>,
    /// Agent type to delegate to (for broadcast)
    pub delegatee_agent_type: Option<String>,

    /// Description of the task
    pub task_description: String,
    /// Parent trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub parent_trajectory_id: EntityId,
    /// Child trajectory created for this task
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub child_trajectory_id: Option<EntityId>,

    /// Artifacts shared with the delegatee
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub shared_artifacts: Vec<EntityId>,
    /// Notes shared with the delegatee
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub shared_notes: Vec<EntityId>,
    /// Additional context (JSON)
    pub additional_context: Option<String>,

    /// Constraints for the task (JSON)
    pub constraints: String,
    /// Deadline for completion
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub deadline: Option<Timestamp>,

    /// Current status
    pub status: DelegationStatus,
    /// Result (when completed)
    pub result: Option<DelegationResult>,

    /// When the delegation was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// When the delegation was accepted
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    /// When the delegation was completed
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
}

impl DelegatedTask {
    /// Create a new delegation to a specific agent.
    pub fn to_agent(
        delegator_agent_id: EntityId,
        delegatee_agent_id: EntityId,
        task_description: &str,
        parent_trajectory_id: EntityId,
    ) -> Self {
        Self {
            delegation_id: Uuid::now_v7(),
            delegator_agent_id,
            delegatee_agent_id: Some(delegatee_agent_id),
            delegatee_agent_type: None,
            task_description: task_description.to_string(),
            parent_trajectory_id,
            child_trajectory_id: None,
            shared_artifacts: Vec::new(),
            shared_notes: Vec::new(),
            additional_context: None,
            constraints: "{}".to_string(),
            deadline: None,
            status: DelegationStatus::Pending,
            result: None,
            created_at: Utc::now(),
            accepted_at: None,
            completed_at: None,
        }
    }

    /// Create a new delegation to an agent type.
    pub fn to_type(
        delegator_agent_id: EntityId,
        delegatee_agent_type: &str,
        task_description: &str,
        parent_trajectory_id: EntityId,
    ) -> Self {
        Self {
            delegation_id: Uuid::now_v7(),
            delegator_agent_id,
            delegatee_agent_id: None,
            delegatee_agent_type: Some(delegatee_agent_type.to_string()),
            task_description: task_description.to_string(),
            parent_trajectory_id,
            child_trajectory_id: None,
            shared_artifacts: Vec::new(),
            shared_notes: Vec::new(),
            additional_context: None,
            constraints: "{}".to_string(),
            deadline: None,
            status: DelegationStatus::Pending,
            result: None,
            created_at: Utc::now(),
            accepted_at: None,
            completed_at: None,
        }
    }

    /// Set shared artifacts.
    pub fn with_shared_artifacts(mut self, artifacts: Vec<EntityId>) -> Self {
        self.shared_artifacts = artifacts;
        self
    }

    /// Set shared notes.
    pub fn with_shared_notes(mut self, notes: Vec<EntityId>) -> Self {
        self.shared_notes = notes;
        self
    }

    /// Set deadline.
    pub fn with_deadline(mut self, deadline: Timestamp) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Set constraints.
    pub fn with_constraints(mut self, constraints: &str) -> Self {
        self.constraints = constraints.to_string();
        self
    }

    /// Accept the delegation.
    pub fn accept(&mut self, delegatee_agent_id: EntityId, child_trajectory_id: EntityId) {
        self.delegatee_agent_id = Some(delegatee_agent_id);
        self.child_trajectory_id = Some(child_trajectory_id);
        self.status = DelegationStatus::Accepted;
        self.accepted_at = Some(Utc::now());
    }

    /// Reject the delegation.
    pub fn reject(&mut self) {
        self.status = DelegationStatus::Rejected;
    }

    /// Start working on the delegation.
    pub fn start(&mut self) {
        self.status = DelegationStatus::InProgress;
    }

    /// Complete the delegation.
    pub fn complete(&mut self, result: DelegationResult) {
        self.status = if result.status == DelegationResultStatus::Failure {
            DelegationStatus::Failed
        } else {
            DelegationStatus::Completed
        };
        self.result = Some(result);
        self.completed_at = Some(Utc::now());
    }

    /// Check if deadline has passed.
    pub fn is_overdue(&self) -> bool {
        self.deadline.is_some_and(|d| Utc::now() > d)
    }
}


// ============================================================================
// AGENT HANDOFF (Task 10.6)
// ============================================================================

/// Status of a handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum HandoffStatus {
    /// Handoff has been initiated
    Initiated,
    /// Handoff has been accepted
    Accepted,
    /// Handoff has been completed
    Completed,
    /// Handoff was rejected
    Rejected,
}

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

/// A handoff between agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentHandoff {
    /// Unique identifier for this handoff
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub handoff_id: EntityId,

    /// Agent initiating the handoff
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    /// Specific agent to hand off to (if targeted)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<EntityId>,
    /// Agent type to hand off to (for broadcast)
    pub to_agent_type: Option<String>,

    /// Trajectory being handed off
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Scope being handed off
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,

    /// Context snapshot ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub context_snapshot_id: EntityId,
    /// Notes for the receiving agent
    pub handoff_notes: String,

    /// Suggested next steps
    pub next_steps: Vec<String>,
    /// Known blockers
    pub blockers: Vec<String>,
    /// Open questions
    pub open_questions: Vec<String>,

    /// Current status
    pub status: HandoffStatus,

    /// When the handoff was initiated
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub initiated_at: Timestamp,
    /// When the handoff was accepted
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    /// When the handoff was completed
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,

    /// Reason for the handoff
    pub reason: HandoffReason,
}

impl AgentHandoff {
    /// Create a new handoff to a specific agent.
    pub fn to_agent(
        from_agent_id: EntityId,
        to_agent_id: EntityId,
        trajectory_id: EntityId,
        scope_id: EntityId,
        context_snapshot_id: EntityId,
        reason: HandoffReason,
    ) -> Self {
        Self {
            handoff_id: Uuid::now_v7(),
            from_agent_id,
            to_agent_id: Some(to_agent_id),
            to_agent_type: None,
            trajectory_id,
            scope_id,
            context_snapshot_id,
            handoff_notes: String::new(),
            next_steps: Vec::new(),
            blockers: Vec::new(),
            open_questions: Vec::new(),
            status: HandoffStatus::Initiated,
            initiated_at: Utc::now(),
            accepted_at: None,
            completed_at: None,
            reason,
        }
    }

    /// Create a new handoff to an agent type.
    pub fn to_type(
        from_agent_id: EntityId,
        to_agent_type: &str,
        trajectory_id: EntityId,
        scope_id: EntityId,
        context_snapshot_id: EntityId,
        reason: HandoffReason,
    ) -> Self {
        Self {
            handoff_id: Uuid::now_v7(),
            from_agent_id,
            to_agent_id: None,
            to_agent_type: Some(to_agent_type.to_string()),
            trajectory_id,
            scope_id,
            context_snapshot_id,
            handoff_notes: String::new(),
            next_steps: Vec::new(),
            blockers: Vec::new(),
            open_questions: Vec::new(),
            status: HandoffStatus::Initiated,
            initiated_at: Utc::now(),
            accepted_at: None,
            completed_at: None,
            reason,
        }
    }

    /// Set handoff notes.
    pub fn with_notes(mut self, notes: &str) -> Self {
        self.handoff_notes = notes.to_string();
        self
    }

    /// Set next steps.
    pub fn with_next_steps(mut self, steps: Vec<String>) -> Self {
        self.next_steps = steps;
        self
    }

    /// Set blockers.
    pub fn with_blockers(mut self, blockers: Vec<String>) -> Self {
        self.blockers = blockers;
        self
    }

    /// Set open questions.
    pub fn with_open_questions(mut self, questions: Vec<String>) -> Self {
        self.open_questions = questions;
        self
    }

    /// Accept the handoff.
    pub fn accept(&mut self, accepting_agent_id: EntityId) {
        self.to_agent_id = Some(accepting_agent_id);
        self.status = HandoffStatus::Accepted;
        self.accepted_at = Some(Utc::now());
    }

    /// Reject the handoff.
    pub fn reject(&mut self) {
        self.status = HandoffStatus::Rejected;
    }

    /// Complete the handoff.
    pub fn complete(&mut self) {
        self.status = HandoffStatus::Completed;
        self.completed_at = Some(Utc::now());
    }
}


// ============================================================================
// CONFLICT TYPES (Task 10.7)
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

/// Record of how a conflict was resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ConflictResolutionRecord {
    /// Strategy used
    pub strategy: ResolutionStrategy,
    /// Winner (if applicable): "a", "b", or "merged"
    pub winner: Option<String>,
    /// ID of merged result (if applicable)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub merged_result_id: Option<EntityId>,
    /// Reason for the resolution
    pub reason: String,
    /// Who resolved it: "automatic" or agent UUID
    pub resolved_by: String,
}

impl ConflictResolutionRecord {
    /// Create an automatic resolution record.
    pub fn automatic(strategy: ResolutionStrategy, winner: Option<&str>, reason: &str) -> Self {
        Self {
            strategy,
            winner: winner.map(|s| s.to_string()),
            merged_result_id: None,
            reason: reason.to_string(),
            resolved_by: "automatic".to_string(),
        }
    }

    /// Create a manual resolution record.
    pub fn manual(
        strategy: ResolutionStrategy,
        winner: Option<&str>,
        reason: &str,
        resolved_by: EntityId,
    ) -> Self {
        Self {
            strategy,
            winner: winner.map(|s| s.to_string()),
            merged_result_id: None,
            reason: reason.to_string(),
            resolved_by: resolved_by.to_string(),
        }
    }

    /// Set merged result ID.
    pub fn with_merged_result(mut self, result_id: EntityId) -> Self {
        self.merged_result_id = Some(result_id);
        self
    }
}

/// A detected conflict between items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Conflict {
    /// Unique identifier for this conflict
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub conflict_id: EntityId,
    /// Type of conflict
    pub conflict_type: ConflictType,

    /// Type of first item
    pub item_a_type: String,
    /// ID of first item
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub item_a_id: EntityId,
    /// Type of second item
    pub item_b_type: String,
    /// ID of second item
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub item_b_id: EntityId,

    /// Agent that created first item (if known)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_a_id: Option<EntityId>,
    /// Agent that created second item (if known)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_b_id: Option<EntityId>,

    /// Related trajectory (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,

    /// Current status
    pub status: ConflictStatus,
    /// Resolution record (when resolved)
    pub resolution: Option<ConflictResolutionRecord>,

    /// When the conflict was detected
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub detected_at: Timestamp,
    /// When the conflict was resolved
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub resolved_at: Option<Timestamp>,
}

impl Conflict {
    /// Create a new conflict.
    pub fn new(
        conflict_type: ConflictType,
        item_a_type: &str,
        item_a_id: EntityId,
        item_b_type: &str,
        item_b_id: EntityId,
    ) -> Self {
        Self {
            conflict_id: Uuid::now_v7(),
            conflict_type,
            item_a_type: item_a_type.to_string(),
            item_a_id,
            item_b_type: item_b_type.to_string(),
            item_b_id,
            agent_a_id: None,
            agent_b_id: None,
            trajectory_id: None,
            status: ConflictStatus::Detected,
            resolution: None,
            detected_at: Utc::now(),
            resolved_at: None,
        }
    }

    /// Set agents involved.
    pub fn with_agents(mut self, agent_a: Option<EntityId>, agent_b: Option<EntityId>) -> Self {
        self.agent_a_id = agent_a;
        self.agent_b_id = agent_b;
        self
    }

    /// Set trajectory context.
    pub fn with_trajectory(mut self, trajectory_id: EntityId) -> Self {
        self.trajectory_id = Some(trajectory_id);
        self
    }

    /// Start resolving the conflict.
    pub fn start_resolving(&mut self) {
        self.status = ConflictStatus::Resolving;
    }

    /// Resolve the conflict.
    pub fn resolve(&mut self, resolution: ConflictResolutionRecord) {
        self.status = if resolution.strategy == ResolutionStrategy::Escalate {
            ConflictStatus::Escalated
        } else {
            ConflictStatus::Resolved
        };
        self.resolution = Some(resolution);
        self.resolved_at = Some(Utc::now());
    }

    /// Escalate the conflict.
    pub fn escalate(&mut self, reason: &str) {
        self.status = ConflictStatus::Escalated;
        self.resolution = Some(ConflictResolutionRecord::automatic(
            ResolutionStrategy::Escalate,
            None,
            reason,
        ));
    }
}


// ============================================================================
// LOCK MANAGER (In-Memory for Testing)
// ============================================================================

/// In-memory lock manager for testing and non-Postgres environments.
/// NOTE: In production, use Postgres advisory locks via caliber-pg.
#[cfg(test)]
#[derive(Debug, Default)]
struct LockManager {
    locks: std::collections::HashMap<i64, DistributedLock>,
}

#[cfg(test)]
impl LockManager {
    /// Create a new lock manager.
    fn new() -> Self {
        Self {
            locks: std::collections::HashMap::new(),
        }
    }

    /// Try to acquire a lock.
    fn acquire(
        &mut self,
        agent_id: EntityId,
        resource_type: &str,
        resource_id: EntityId,
        mode: LockMode,
        timeout_ms: i64,
    ) -> CaliberResult<DistributedLock> {
        let key = compute_lock_key(resource_type, resource_id);

        // Check if lock exists and is not expired
        if let Some(existing) = self.locks.get(&key) {
            if !existing.is_expired() {
                // Lock is held
                if existing.mode == LockMode::Exclusive || mode == LockMode::Exclusive {
                    return Err(CaliberError::Agent(AgentError::LockAcquisitionFailed {
                        resource: format!("{}:{}", resource_type, resource_id),
                        holder: existing.holder_agent_id,
                    }));
                }
            }
        }

        // Create new lock
        let lock = DistributedLock::new(resource_type, resource_id, agent_id, timeout_ms, mode);
        self.locks.insert(key, lock.clone());

        Ok(lock)
    }

    /// Release a lock.
    fn release(&mut self, lock_id: EntityId) -> CaliberResult<bool> {
        let key = self
            .locks
            .iter()
            .find(|(_, l)| l.lock_id == lock_id)
            .map(|(k, _)| *k);

        if let Some(key) = key {
            self.locks.remove(&key);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get a lock by ID.
    fn get(&self, lock_id: EntityId) -> Option<&DistributedLock> {
        self.locks.values().find(|l| l.lock_id == lock_id)
    }

    /// Clean up expired locks.
    fn cleanup_expired(&mut self) -> usize {
        let before = self.locks.len();
        self.locks.retain(|_, l| !l.is_expired());
        before - self.locks.len()
    }
}


// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Agent Tests
    // ========================================================================

    #[test]
    fn test_agent_new() {
        let agent = Agent::new("coder", vec!["rust".to_string(), "python".to_string()]);

        assert_eq!(agent.agent_type, "coder");
        assert_eq!(agent.capabilities.len(), 2);
        assert_eq!(agent.status, AgentStatus::Idle);
        assert!(agent.current_trajectory_id.is_none());
    }

    #[test]
    fn test_agent_has_capability() {
        let agent = Agent::new("coder", vec!["rust".to_string()]);

        assert!(agent.has_capability("rust"));
        assert!(!agent.has_capability("python"));
    }

    #[test]
    fn test_agent_can_delegate() {
        let agent = Agent::new("planner", vec![])
            .with_delegation_targets(vec!["coder".to_string(), "reviewer".to_string()]);

        assert!(agent.can_delegate_to_type("coder"));
        assert!(agent.can_delegate_to_type("reviewer"));
        assert!(!agent.can_delegate_to_type("planner"));
    }

    // ========================================================================
    // Memory Region Tests
    // ========================================================================

    #[test]
    fn test_memory_region_private() {
        let owner_id = Uuid::now_v7();
        let other_id = Uuid::now_v7();
        let region = MemoryRegionConfig::private(owner_id);

        assert!(region.can_read(owner_id));
        assert!(region.can_write(owner_id));
        assert!(!region.can_read(other_id));
        assert!(!region.can_write(other_id));
    }

    #[test]
    fn test_memory_region_public() {
        let owner_id = Uuid::now_v7();
        let other_id = Uuid::now_v7();
        let region = MemoryRegionConfig::public(owner_id);

        assert!(region.can_read(owner_id));
        assert!(region.can_write(owner_id));
        assert!(region.can_read(other_id));
        assert!(!region.can_write(other_id));
    }

    #[test]
    fn test_memory_region_collaborative() {
        let owner_id = Uuid::now_v7();
        let other_id = Uuid::now_v7();
        let region = MemoryRegionConfig::collaborative(owner_id);

        assert!(region.can_read(owner_id));
        assert!(region.can_write(owner_id));
        assert!(region.can_read(other_id));
        assert!(region.can_write(other_id));
        assert!(region.require_lock);
    }

    // ========================================================================
    // Lock Tests
    // ========================================================================

    #[test]
    fn test_distributed_lock_new() {
        let agent_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();
        let lock = DistributedLock::new("artifact", resource_id, agent_id, 30000, LockMode::Exclusive);

        assert_eq!(lock.resource_type, "artifact");
        assert_eq!(lock.resource_id, resource_id);
        assert_eq!(lock.holder_agent_id, agent_id);
        assert_eq!(lock.mode, LockMode::Exclusive);
        assert!(!lock.is_expired());
    }

    #[test]
    fn test_lock_manager_acquire_release() -> CaliberResult<()> {
        let mut manager = LockManager::new();
        let agent_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();

        let lock = manager
            .acquire(agent_id, "artifact", resource_id, LockMode::Exclusive, 30000)
            ?;

        assert!(manager.get(lock.lock_id).is_some());

        let released = manager.release(lock.lock_id)?;
        assert!(released);
        assert!(manager.get(lock.lock_id).is_none());
        Ok(())
    }

    #[test]
    fn test_lock_manager_conflict() -> CaliberResult<()> {
        let mut manager = LockManager::new();
        let agent1 = Uuid::now_v7();
        let agent2 = Uuid::now_v7();
        let resource_id = Uuid::now_v7();

        // First agent acquires lock
        let _lock1 = manager
            .acquire(agent1, "artifact", resource_id, LockMode::Exclusive, 30000)
            ?;

        // Second agent tries to acquire - should fail
        let result = manager.acquire(agent2, "artifact", resource_id, LockMode::Exclusive, 30000);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_lock_manager_cleanup_expired() -> CaliberResult<()> {
        let mut manager = LockManager::new();
        let agent_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();

        // Acquire a lock with very short timeout (already expired by negative value trick)
        // Actually, we can't easily test expiration without waiting.
        // Instead, test that cleanup returns 0 when no locks are expired.
        let _lock = manager
            .acquire(agent_id, "artifact", resource_id, LockMode::Exclusive, 30000)
            ?;

        // Fresh lock should not be expired
        let cleaned = manager.cleanup_expired();
        assert_eq!(cleaned, 0);
        Ok(())
    }

    // ========================================================================
    // Message Tests
    // ========================================================================

    #[test]
    fn test_agent_message_to_agent() {
        let from_id = Uuid::now_v7();
        let to_id = Uuid::now_v7();
        let msg = AgentMessage::to_agent(from_id, to_id, MessageType::TaskDelegation, "{}");

        assert_eq!(msg.from_agent_id, from_id);
        assert_eq!(msg.to_agent_id, Some(to_id));
        assert!(msg.to_agent_type.is_none());
        assert_eq!(msg.message_type, MessageType::TaskDelegation);
    }

    #[test]
    fn test_agent_message_to_type() {
        let from_id = Uuid::now_v7();
        let msg = AgentMessage::to_type(from_id, "coder", MessageType::TaskDelegation, "{}");

        assert_eq!(msg.from_agent_id, from_id);
        assert!(msg.to_agent_id.is_none());
        assert_eq!(msg.to_agent_type, Some("coder".to_string()));
    }

    #[test]
    fn test_agent_message_is_for_agent() {
        let from_id = Uuid::now_v7();
        let to_id = Uuid::now_v7();
        let other_id = Uuid::now_v7();

        let msg = AgentMessage::to_agent(from_id, to_id, MessageType::Heartbeat, "{}");

        assert!(msg.is_for_agent(to_id, "any"));
        assert!(!msg.is_for_agent(other_id, "any"));
    }

    // ========================================================================
    // Delegation Tests
    // ========================================================================

    #[test]
    fn test_delegated_task_lifecycle() {
        let delegator = Uuid::now_v7();
        let delegatee = Uuid::now_v7();
        let trajectory = Uuid::now_v7();
        let child_trajectory = Uuid::now_v7();

        let mut task = DelegatedTask::to_type(delegator, "coder", "Implement feature X", trajectory);

        assert_eq!(task.status, DelegationStatus::Pending);

        task.accept(delegatee, child_trajectory);
        assert_eq!(task.status, DelegationStatus::Accepted);
        assert_eq!(task.delegatee_agent_id, Some(delegatee));

        task.start();
        assert_eq!(task.status, DelegationStatus::InProgress);

        task.complete(DelegationResult::success("Feature implemented", vec![]));
        assert_eq!(task.status, DelegationStatus::Completed);
        assert!(task.result.is_some());
    }

    // ========================================================================
    // Handoff Tests
    // ========================================================================

    #[test]
    fn test_agent_handoff_lifecycle() {
        let from_agent = Uuid::now_v7();
        let to_agent = Uuid::now_v7();
        let trajectory = Uuid::now_v7();
        let scope = Uuid::now_v7();
        let snapshot = Uuid::now_v7();

        let mut handoff = AgentHandoff::to_type(
            from_agent,
            "specialist",
            trajectory,
            scope,
            snapshot,
            HandoffReason::Specialization,
        )
        .with_notes("Need specialized knowledge")
        .with_next_steps(vec!["Review context".to_string()]);

        assert_eq!(handoff.status, HandoffStatus::Initiated);

        handoff.accept(to_agent);
        assert_eq!(handoff.status, HandoffStatus::Accepted);
        assert_eq!(handoff.to_agent_id, Some(to_agent));

        handoff.complete();
        assert_eq!(handoff.status, HandoffStatus::Completed);
    }

    // ========================================================================
    // Conflict Tests
    // ========================================================================

    #[test]
    fn test_conflict_lifecycle() {
        let artifact_a = Uuid::now_v7();
        let artifact_b = Uuid::now_v7();

        let mut conflict = Conflict::new(
            ConflictType::ContradictingFact,
            "artifact",
            artifact_a,
            "artifact",
            artifact_b,
        );

        assert_eq!(conflict.status, ConflictStatus::Detected);

        conflict.start_resolving();
        assert_eq!(conflict.status, ConflictStatus::Resolving);

        conflict.resolve(ConflictResolutionRecord::automatic(
            ResolutionStrategy::HighestConfidence,
            Some("a"),
            "Artifact A has higher confidence",
        ));
        assert_eq!(conflict.status, ConflictStatus::Resolved);
        assert!(conflict.resolution.is_some());
    }

    #[test]
    fn test_conflict_escalation() {
        let artifact_a = Uuid::now_v7();
        let artifact_b = Uuid::now_v7();

        let mut conflict = Conflict::new(
            ConflictType::GoalConflict,
            "artifact",
            artifact_a,
            "artifact",
            artifact_b,
        );

        conflict.escalate("Cannot automatically resolve goal conflict");
        assert_eq!(conflict.status, ConflictStatus::Escalated);
    }
}


// ============================================================================
// PROPERTY-BASED TESTS (Task 10.8)
// ============================================================================

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating agent types
    fn arb_agent_type() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("coder".to_string()),
            Just("reviewer".to_string()),
            Just("planner".to_string()),
            Just("specialist".to_string()),
        ]
    }

    // Strategy for generating capabilities
    fn arb_capabilities() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec("[a-z]{3,10}".prop_map(|s| s), 0..5)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 9: Lock acquisition records holder
        /// When a lock is acquired, the holder_agent_id SHALL be set to the acquiring agent
        #[test]
        fn prop_lock_acquisition_records_holder(
            agent_type in arb_agent_type()
        ) {
            let mut manager = LockManager::new();
            let agent_id = Uuid::now_v7();
            let resource_id = Uuid::now_v7();

            let lock = match manager
                .acquire(agent_id, &agent_type, resource_id, LockMode::Exclusive, 30000)
            {
                Ok(lock) => lock,
                Err(err) => {
                    prop_assert!(false, "Expected lock acquisition to succeed: {:?}", err);
                    return Ok(());
                }
            };

            prop_assert_eq!(lock.holder_agent_id, agent_id, "Lock holder should be the acquiring agent");
            prop_assert_eq!(lock.resource_id, resource_id, "Lock resource should match");
        }

        /// Property: Agent creation preserves type and capabilities
        #[test]
        fn prop_agent_preserves_type_and_capabilities(
            agent_type in arb_agent_type(),
            capabilities in arb_capabilities()
        ) {
            let agent = Agent::new(&agent_type, capabilities.clone());

            prop_assert_eq!(&agent.agent_type, &agent_type);
            prop_assert_eq!(&agent.capabilities, &capabilities);
            prop_assert_eq!(agent.status, AgentStatus::Idle);
        }

        /// Property: Lock key is deterministic
        #[test]
        fn prop_lock_key_deterministic(
            resource_type in "[a-z]{3,10}",
        ) {
            let resource_id = Uuid::now_v7();

            let key1 = compute_lock_key(&resource_type, resource_id);
            let key2 = compute_lock_key(&resource_type, resource_id);

            prop_assert_eq!(key1, key2, "Lock key should be deterministic");
        }

        /// Property: Different resources produce different keys (with high probability)
        #[test]
        fn prop_different_resources_different_keys(
            resource_type1 in "[a-z]{3,10}",
            resource_type2 in "[a-z]{3,10}",
        ) {
            let resource_id1 = Uuid::now_v7();
            let resource_id2 = Uuid::now_v7();

            // Same type, different IDs
            let key1 = compute_lock_key(&resource_type1, resource_id1);
            let key2 = compute_lock_key(&resource_type1, resource_id2);

            // Different type, same ID
            let key3 = compute_lock_key(&resource_type1, resource_id1);
            let key4 = compute_lock_key(&resource_type2, resource_id1);

            // Keys should differ (with very high probability due to UUID uniqueness)
            if resource_id1 != resource_id2 {
                prop_assert_ne!(key1, key2, "Different resource IDs should produce different keys");
            }
            if resource_type1 != resource_type2 {
                prop_assert_ne!(key3, key4, "Different resource types should produce different keys");
            }
        }

        /// Property: Memory region access control is consistent
        #[test]
        fn prop_memory_region_access_consistent(
            region_type in prop_oneof![
                Just(MemoryRegion::Private),
                Just(MemoryRegion::Public),
                Just(MemoryRegion::Collaborative),
            ]
        ) {
            let owner_id = Uuid::now_v7();
            let other_id = Uuid::now_v7();

            let region = match region_type {
                MemoryRegion::Private => MemoryRegionConfig::private(owner_id),
                MemoryRegion::Public => MemoryRegionConfig::public(owner_id),
                MemoryRegion::Collaborative => MemoryRegionConfig::collaborative(owner_id),
                MemoryRegion::Team => MemoryRegionConfig::team(owner_id, Uuid::now_v7()),
            };

            // Owner should always have read access
            prop_assert!(region.can_read(owner_id), "Owner should always have read access");

            // Owner should always have write access
            prop_assert!(region.can_write(owner_id), "Owner should always have write access");

            // Check non-owner access based on region type
            match region_type {
                MemoryRegion::Private => {
                    prop_assert!(!region.can_read(other_id), "Private: others cannot read");
                    prop_assert!(!region.can_write(other_id), "Private: others cannot write");
                }
                MemoryRegion::Public => {
                    prop_assert!(region.can_read(other_id), "Public: others can read");
                    prop_assert!(!region.can_write(other_id), "Public: others cannot write");
                }
                MemoryRegion::Collaborative => {
                    prop_assert!(region.can_read(other_id), "Collaborative: others can read");
                    prop_assert!(region.can_write(other_id), "Collaborative: others can write");
                }
                _ => {}
            }
        }

        /// Property: Message targeting is mutually exclusive
        #[test]
        fn prop_message_targeting_exclusive(
            to_specific in any::<bool>()
        ) {
            let from_id = Uuid::now_v7();
            let to_id = Uuid::now_v7();

            let msg = if to_specific {
                AgentMessage::to_agent(from_id, to_id, MessageType::Heartbeat, "{}")
            } else {
                AgentMessage::to_type(from_id, "coder", MessageType::Heartbeat, "{}")
            };

            // Either to_agent_id or to_agent_type should be set, not both
            let has_agent_id = msg.to_agent_id.is_some();
            let has_agent_type = msg.to_agent_type.is_some();

            prop_assert!(
                has_agent_id != has_agent_type,
                "Message should target either specific agent or agent type, not both"
            );
        }
    }
}
