//! Core entity structures

use crate::{
    // ID types
    TrajectoryId, ScopeId, ArtifactId, NoteId, TurnId, AgentId,
    MessageId, DelegationId, HandoffId, ConflictId,
    // Other types
    EntityType, TrajectoryStatus, ArtifactType, NoteType, TurnRole,
    TTL, AbstractionLevel, ExtractionMethod, OutcomeStatus,
    EmbeddingVector, ContentHash, RawContent, Timestamp,
    // Agent-related types
    AgentStatus, MemoryAccess,
    MessageType, MessagePriority,
    DelegationStatus, DelegationResult, DelegationResultStatus,
    HandoffStatus, HandoffReason,
    ConflictType, ConflictStatus, ResolutionStrategy,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Reference to an entity by type and ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EntityRef {
    pub entity_type: EntityType,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub id: Uuid,  // Keep as Uuid - this is intentional, represents ANY entity
}

/// Trajectory - top-level task container.
/// A trajectory represents a complete task or goal being pursued.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Trajectory {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    pub name: String,
    pub description: Option<String>,
    pub status: TrajectoryStatus,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_trajectory_id: Option<TrajectoryId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub root_trajectory_id: Option<TrajectoryId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<AgentId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub updated_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    pub outcome: Option<TrajectoryOutcome>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Outcome of a completed trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrajectoryOutcome {
    pub status: OutcomeStatus,
    pub summary: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_artifacts: Vec<ArtifactId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_notes: Vec<NoteId>,
    pub error: Option<String>,
}

/// Scope - partitioned context window within a trajectory.
/// Scopes provide isolation and checkpointing boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Scope {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_scope_id: Option<ScopeId>,
    pub name: String,
    pub purpose: Option<String>,
    pub is_active: bool,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub closed_at: Option<Timestamp>,
    pub checkpoint: Option<Checkpoint>,
    pub token_budget: i32,
    pub tokens_used: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Checkpoint for scope recovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Checkpoint {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_state: RawContent,
    pub recoverable: bool,
}

/// Artifact - typed output preserved across scopes.
/// Artifacts survive scope closure and can be referenced later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Artifact {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub artifact_id: ArtifactId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
    pub artifact_type: ArtifactType,
    pub name: String,
    pub content: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub content_hash: ContentHash,
    pub embedding: Option<EmbeddingVector>,
    pub provenance: Provenance,
    pub ttl: TTL,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub updated_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub superseded_by: Option<ArtifactId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Provenance information for an artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Provenance {
    pub source_turn: i32,
    pub extraction_method: ExtractionMethod,
    pub confidence: Option<f32>,
}

/// Note - long-term cross-trajectory knowledge.
/// Notes persist beyond individual trajectories.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Note {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub note_id: NoteId,
    pub note_type: NoteType,
    pub title: String,
    pub content: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub content_hash: ContentHash,
    pub embedding: Option<EmbeddingVector>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_trajectory_ids: Vec<TrajectoryId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_artifact_ids: Vec<ArtifactId>,
    pub ttl: TTL,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub updated_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub accessed_at: Timestamp,
    pub access_count: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub superseded_by: Option<NoteId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
    // ══════════════════════════════════════════════════════════════════════════
    // Battle Intel Feature 2: Abstraction levels (EVOLVE-MEM L0/L1/L2 hierarchy)
    // ══════════════════════════════════════════════════════════════════════════
    /// Semantic abstraction tier (Raw=L0, Summary=L1, Principle=L2)
    pub abstraction_level: AbstractionLevel,
    /// Notes this was derived from (for L1/L2 derivation chains)
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_note_ids: Vec<NoteId>,
}

/// Turn - ephemeral conversation buffer entry.
/// Turns die with their scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Turn {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub turn_id: TurnId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
    pub sequence: i32,
    pub role: TurnRole,
    pub content: String,
    pub token_count: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_calls: Option<serde_json::Value>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_results: Option<serde_json::Value>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// AGENT ENTITIES (from caliber-agents)
// ============================================================================

/// An agent in the multi-agent system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Agent {
    /// Unique identifier for this agent
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub agent_id: AgentId,
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
    pub current_trajectory_id: Option<TrajectoryId>,
    /// Current scope being worked on
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_scope_id: Option<ScopeId>,

    /// Agent types this agent can delegate to
    pub can_delegate_to: Vec<String>,
    /// Supervisor agent (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub reports_to: Option<AgentId>,

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
            agent_id: AgentId::new(Uuid::now_v7()),
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
    pub fn with_supervisor(mut self, supervisor_id: AgentId) -> Self {
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

/// A message between agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentMessage {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub message_id: MessageId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<AgentId>,
    pub to_agent_type: Option<String>,
    pub message_type: MessageType,
    pub payload: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<TrajectoryId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<ScopeId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifact_ids: Vec<ArtifactId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub delivered_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub acknowledged_at: Option<Timestamp>,
    pub priority: MessagePriority,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expires_at: Option<Timestamp>,
}

impl AgentMessage {
    /// Create a message to a specific agent.
    pub fn to_agent(from: AgentId, to: AgentId, msg_type: MessageType, payload: &str) -> Self {
        Self {
            message_id: MessageId::new(Uuid::now_v7()),
            from_agent_id: from,
            to_agent_id: Some(to),
            to_agent_type: None,
            message_type: msg_type,
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

    /// Create a message to an agent type.
    pub fn to_type(from: AgentId, agent_type: &str, msg_type: MessageType, payload: &str) -> Self {
        Self {
            message_id: MessageId::new(Uuid::now_v7()),
            from_agent_id: from,
            to_agent_id: None,
            to_agent_type: Some(agent_type.to_string()),
            message_type: msg_type,
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

    /// Associate with trajectory.
    pub fn with_trajectory(mut self, trajectory_id: TrajectoryId) -> Self {
        self.trajectory_id = Some(trajectory_id);
        self
    }

    /// Associate with scope.
    pub fn with_scope(mut self, scope_id: ScopeId) -> Self {
        self.scope_id = Some(scope_id);
        self
    }

    /// Add artifacts.
    pub fn with_artifacts(mut self, artifacts: Vec<ArtifactId>) -> Self {
        self.artifact_ids = artifacts;
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
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

    /// Check if message is for a specific agent.
    pub fn is_for_agent(&self, agent_id: AgentId) -> bool {
        self.to_agent_id == Some(agent_id)
    }
}

/// A delegated task from one agent to another.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegatedTask {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub delegation_id: DelegationId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub delegator_agent_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub delegatee_agent_id: Option<AgentId>,
    pub delegatee_agent_type: Option<String>,
    pub task_description: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub parent_trajectory_id: TrajectoryId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub child_trajectory_id: Option<TrajectoryId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub shared_artifacts: Vec<ArtifactId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub shared_notes: Vec<NoteId>,
    pub additional_context: Option<String>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub constraints: Option<serde_json::Value>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub deadline: Option<Timestamp>,
    pub status: DelegationStatus,
    pub result: Option<DelegationResult>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
}

impl DelegatedTask {
    /// Create a delegation to a specific agent.
    pub fn to_agent(from: AgentId, to: AgentId, trajectory: TrajectoryId, description: &str) -> Self {
        Self {
            delegation_id: DelegationId::new(Uuid::now_v7()),
            delegator_agent_id: from,
            delegatee_agent_id: Some(to),
            delegatee_agent_type: None,
            task_description: description.to_string(),
            parent_trajectory_id: trajectory,
            child_trajectory_id: None,
            shared_artifacts: Vec::new(),
            shared_notes: Vec::new(),
            additional_context: None,
            constraints: None,
            deadline: None,
            status: DelegationStatus::Pending,
            result: None,
            created_at: Utc::now(),
            accepted_at: None,
            completed_at: None,
        }
    }

    /// Create a delegation to an agent type.
    pub fn to_type(from: AgentId, agent_type: &str, trajectory: TrajectoryId, description: &str) -> Self {
        Self {
            delegation_id: DelegationId::new(Uuid::now_v7()),
            delegator_agent_id: from,
            delegatee_agent_id: None,
            delegatee_agent_type: Some(agent_type.to_string()),
            task_description: description.to_string(),
            parent_trajectory_id: trajectory,
            child_trajectory_id: None,
            shared_artifacts: Vec::new(),
            shared_notes: Vec::new(),
            additional_context: None,
            constraints: None,
            deadline: None,
            status: DelegationStatus::Pending,
            result: None,
            created_at: Utc::now(),
            accepted_at: None,
            completed_at: None,
        }
    }

    /// Add shared artifacts.
    pub fn with_shared_artifacts(mut self, artifacts: Vec<ArtifactId>) -> Self {
        self.shared_artifacts = artifacts;
        self
    }

    /// Add shared notes.
    pub fn with_shared_notes(mut self, notes: Vec<NoteId>) -> Self {
        self.shared_notes = notes;
        self
    }

    /// Set deadline.
    pub fn with_deadline(mut self, deadline: Timestamp) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Accept the delegation.
    pub fn accept(&mut self) {
        self.status = DelegationStatus::Accepted;
        self.accepted_at = Some(Utc::now());
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
}

/// A handoff from one agent to another.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentHandoff {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub handoff_id: HandoffId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<AgentId>,
    pub to_agent_type: Option<String>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub context_snapshot_id: Option<ArtifactId>,
    pub handoff_notes: Option<String>,
    pub next_steps: Vec<String>,
    pub blockers: Vec<String>,
    pub open_questions: Vec<String>,
    pub status: HandoffStatus,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub initiated_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    pub reason: HandoffReason,
}

impl AgentHandoff {
    /// Create a handoff to a specific agent.
    pub fn to_agent(from: AgentId, to: AgentId, trajectory: TrajectoryId, scope: ScopeId, reason: HandoffReason) -> Self {
        Self {
            handoff_id: HandoffId::new(Uuid::now_v7()),
            from_agent_id: from,
            to_agent_id: Some(to),
            to_agent_type: None,
            trajectory_id: trajectory,
            scope_id: scope,
            context_snapshot_id: None,
            handoff_notes: None,
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

    /// Create a handoff to an agent type.
    pub fn to_type(from: AgentId, agent_type: &str, trajectory: TrajectoryId, scope: ScopeId, reason: HandoffReason) -> Self {
        Self {
            handoff_id: HandoffId::new(Uuid::now_v7()),
            from_agent_id: from,
            to_agent_id: None,
            to_agent_type: Some(agent_type.to_string()),
            trajectory_id: trajectory,
            scope_id: scope,
            context_snapshot_id: None,
            handoff_notes: None,
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

    /// Add handoff notes.
    pub fn with_notes(mut self, notes: &str) -> Self {
        self.handoff_notes = Some(notes.to_string());
        self
    }

    /// Add next steps.
    pub fn with_next_steps(mut self, steps: Vec<String>) -> Self {
        self.next_steps = steps;
        self
    }

    /// Accept the handoff.
    pub fn accept(&mut self) {
        self.status = HandoffStatus::Accepted;
        self.accepted_at = Some(Utc::now());
    }

    /// Complete the handoff.
    pub fn complete(&mut self) {
        self.status = HandoffStatus::Completed;
        self.completed_at = Some(Utc::now());
    }
}

/// A conflict between memory items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Conflict {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub conflict_id: ConflictId,
    pub conflict_type: ConflictType,
    pub item_a_type: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub item_a_id: Uuid,
    pub item_b_type: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub item_b_id: Uuid,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_a_id: Option<AgentId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_b_id: Option<AgentId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<TrajectoryId>,
    pub status: ConflictStatus,
    pub resolution: Option<ConflictResolutionRecord>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub detected_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub resolved_at: Option<Timestamp>,
}

impl Conflict {
    /// Create a new conflict.
    pub fn new(
        conflict_type: ConflictType,
        item_a_type: &str,
        item_a_id: Uuid,
        item_b_type: &str,
        item_b_id: Uuid,
    ) -> Self {
        Self {
            conflict_id: ConflictId::new(Uuid::now_v7()),
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

    /// Associate with agents.
    pub fn with_agents(mut self, agent_a: AgentId, agent_b: AgentId) -> Self {
        self.agent_a_id = Some(agent_a);
        self.agent_b_id = Some(agent_b);
        self
    }

    /// Associate with trajectory.
    pub fn with_trajectory(mut self, trajectory_id: TrajectoryId) -> Self {
        self.trajectory_id = Some(trajectory_id);
        self
    }

    /// Resolve the conflict.
    pub fn resolve(&mut self, resolution: ConflictResolutionRecord) {
        self.status = ConflictStatus::Resolved;
        self.resolution = Some(resolution);
        self.resolved_at = Some(Utc::now());
    }
}

/// Record of how a conflict was resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ConflictResolutionRecord {
    pub strategy: ResolutionStrategy,
    pub winner: Option<String>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub merged_result_id: Option<Uuid>,
    pub reason: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub resolved_by: Option<AgentId>,
}

impl ConflictResolutionRecord {
    /// Create an automatic resolution.
    pub fn automatic(strategy: ResolutionStrategy, reason: &str) -> Self {
        Self {
            strategy,
            winner: None,
            merged_result_id: None,
            reason: reason.to_string(),
            resolved_by: None,
        }
    }

    /// Create a manual resolution.
    pub fn manual(strategy: ResolutionStrategy, reason: &str, resolved_by: AgentId) -> Self {
        Self {
            strategy,
            winner: None,
            merged_result_id: None,
            reason: reason.to_string(),
            resolved_by: Some(resolved_by),
        }
    }
}

