//! WebSocket Event Types
//!
//! This module defines all event types that are broadcast via WebSocket
//! to connected clients for real-time updates.

use crate::types::*;
use caliber_core::EntityId;
use serde::{Deserialize, Serialize};

/// WebSocket event types for real-time updates.
///
/// All mutation operations (create, update, delete) on CALIBER entities
/// trigger corresponding events that are broadcast to subscribed clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    // ========================================================================
    // TRAJECTORY EVENTS
    // ========================================================================
    /// A new trajectory was created.
    TrajectoryCreated {
        /// The created trajectory
        trajectory: TrajectoryResponse,
    },

    /// An existing trajectory was updated.
    TrajectoryUpdated {
        /// The updated trajectory
        trajectory: TrajectoryResponse,
    },

    /// A trajectory was deleted.
    TrajectoryDeleted {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the deleted trajectory
        id: EntityId,
    },

    // ========================================================================
    // SCOPE EVENTS
    // ========================================================================
    /// A new scope was created.
    ScopeCreated {
        /// The created scope
        scope: ScopeResponse,
    },

    /// An existing scope was updated.
    ScopeUpdated {
        /// The updated scope
        scope: ScopeResponse,
    },

    /// A scope was closed.
    ScopeClosed {
        /// The closed scope
        scope: ScopeResponse,
    },

    // ========================================================================
    // ARTIFACT EVENTS
    // ========================================================================
    /// A new artifact was created.
    ArtifactCreated {
        /// The created artifact
        artifact: ArtifactResponse,
    },

    /// An existing artifact was updated.
    ArtifactUpdated {
        /// The updated artifact
        artifact: ArtifactResponse,
    },

    /// An artifact was deleted.
    ArtifactDeleted {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the deleted artifact
        id: EntityId,
    },

    // ========================================================================
    // NOTE EVENTS
    // ========================================================================
    /// A new note was created.
    NoteCreated {
        /// The created note
        note: NoteResponse,
    },

    /// An existing note was updated.
    NoteUpdated {
        /// The updated note
        note: NoteResponse,
    },

    /// A note was deleted.
    NoteDeleted {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the deleted note
        id: EntityId,
    },

    // ========================================================================
    // TURN EVENTS
    // ========================================================================
    /// A new turn was created.
    TurnCreated {
        /// The created turn
        turn: TurnResponse,
    },

    // ========================================================================
    // AGENT EVENTS
    // ========================================================================
    /// A new agent was registered.
    AgentRegistered {
        /// The registered agent
        agent: AgentResponse,
    },

    /// An agent's status changed.
    AgentStatusChanged {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the agent
        agent_id: EntityId,
        /// New status
        status: String,
    },

    /// An agent sent a heartbeat.
    AgentHeartbeat {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the agent
        agent_id: EntityId,
        /// Timestamp of the heartbeat
        timestamp: caliber_core::Timestamp,
    },

    /// An agent was unregistered.
    AgentUnregistered {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the unregistered agent
        id: EntityId,
    },

    // ========================================================================
    // LOCK EVENTS
    // ========================================================================
    /// A lock was acquired.
    LockAcquired {
        /// The acquired lock
        lock: LockResponse,
    },

    /// A lock was released.
    LockReleased {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the released lock
        lock_id: EntityId,
    },

    /// A lock expired.
    LockExpired {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the expired lock
        lock_id: EntityId,
    },

    // ========================================================================
    // MESSAGE EVENTS
    // ========================================================================
    /// A message was sent.
    MessageSent {
        /// The sent message
        message: MessageResponse,
    },

    /// A message was delivered.
    MessageDelivered {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the delivered message
        message_id: EntityId,
    },

    /// A message was acknowledged.
    MessageAcknowledged {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the acknowledged message
        message_id: EntityId,
    },

    // ========================================================================
    // DELEGATION EVENTS
    // ========================================================================
    /// A delegation was created.
    DelegationCreated {
        /// The created delegation
        delegation: DelegationResponse,
    },

    /// A delegation was accepted.
    DelegationAccepted {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the accepted delegation
        delegation_id: EntityId,
    },

    /// A delegation was rejected.
    DelegationRejected {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the rejected delegation
        delegation_id: EntityId,
    },

    /// A delegation was completed.
    DelegationCompleted {
        /// The completed delegation
        delegation: DelegationResponse,
    },

    // ========================================================================
    // HANDOFF EVENTS
    // ========================================================================
    /// A handoff was created.
    HandoffCreated {
        /// The created handoff
        handoff: HandoffResponse,
    },

    /// A handoff was accepted.
    HandoffAccepted {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the accepted handoff
        handoff_id: EntityId,
    },

    /// A handoff was completed.
    HandoffCompleted {
        /// The completed handoff
        handoff: HandoffResponse,
    },

    // ========================================================================
    // CONFIG EVENTS
    // ========================================================================
    /// Configuration was updated.
    ConfigUpdated {
        /// The updated configuration
        config: ConfigResponse,
    },

    // ========================================================================
    // CONNECTION EVENTS
    // ========================================================================
    /// Client successfully connected.
    Connected {
        /// Tenant ID the client is connected to
        tenant_id: EntityId,
    },

    /// Client disconnected.
    Disconnected {
        /// Reason for disconnection
        reason: String,
    },

    /// An error occurred.
    Error {
        /// Error message
        message: String,
    },

    // ========================================================================
    // BATTLE INTEL EVENTS
    // ========================================================================
    /// A summarization policy trigger was fired.
    SummarizationTriggered {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the policy that triggered
        policy_id: EntityId,
        /// The trigger that fired
        trigger: caliber_core::SummarizationTrigger,
        /// Scope ID where trigger fired
        scope_id: EntityId,
        /// Trajectory ID for context
        trajectory_id: EntityId,
        /// Source abstraction level for summarization
        source_level: caliber_core::AbstractionLevel,
        /// Target abstraction level for summarization
        target_level: caliber_core::AbstractionLevel,
        /// Maximum sources to summarize
        max_sources: i32,
        /// Whether to create SynthesizedFrom edges
        create_edges: bool,
    },

    /// An edge was created.
    EdgeCreated {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// ID of the created edge
        edge_id: EntityId,
        /// Type of the edge
        edge_type: caliber_core::EdgeType,
    },

    /// Multiple edges were created in a batch.
    EdgesBatchCreated {
        /// Tenant ID for filtering
        tenant_id: EntityId,
        /// Number of edges created
        count: usize,
    },
}

impl WsEvent {
    /// Get the event type as a string for logging/debugging.
    pub fn event_type(&self) -> &'static str {
        match self {
            WsEvent::TrajectoryCreated { .. } => "TrajectoryCreated",
            WsEvent::TrajectoryUpdated { .. } => "TrajectoryUpdated",
            WsEvent::TrajectoryDeleted { .. } => "TrajectoryDeleted",
            WsEvent::ScopeCreated { .. } => "ScopeCreated",
            WsEvent::ScopeUpdated { .. } => "ScopeUpdated",
            WsEvent::ScopeClosed { .. } => "ScopeClosed",
            WsEvent::ArtifactCreated { .. } => "ArtifactCreated",
            WsEvent::ArtifactUpdated { .. } => "ArtifactUpdated",
            WsEvent::ArtifactDeleted { .. } => "ArtifactDeleted",
            WsEvent::NoteCreated { .. } => "NoteCreated",
            WsEvent::NoteUpdated { .. } => "NoteUpdated",
            WsEvent::NoteDeleted { .. } => "NoteDeleted",
            WsEvent::TurnCreated { .. } => "TurnCreated",
            WsEvent::AgentRegistered { .. } => "AgentRegistered",
            WsEvent::AgentStatusChanged { .. } => "AgentStatusChanged",
            WsEvent::AgentHeartbeat { .. } => "AgentHeartbeat",
            WsEvent::AgentUnregistered { .. } => "AgentUnregistered",
            WsEvent::LockAcquired { .. } => "LockAcquired",
            WsEvent::LockReleased { .. } => "LockReleased",
            WsEvent::LockExpired { .. } => "LockExpired",
            WsEvent::MessageSent { .. } => "MessageSent",
            WsEvent::MessageDelivered { .. } => "MessageDelivered",
            WsEvent::MessageAcknowledged { .. } => "MessageAcknowledged",
            WsEvent::DelegationCreated { .. } => "DelegationCreated",
            WsEvent::DelegationAccepted { .. } => "DelegationAccepted",
            WsEvent::DelegationRejected { .. } => "DelegationRejected",
            WsEvent::DelegationCompleted { .. } => "DelegationCompleted",
            WsEvent::HandoffCreated { .. } => "HandoffCreated",
            WsEvent::HandoffAccepted { .. } => "HandoffAccepted",
            WsEvent::HandoffCompleted { .. } => "HandoffCompleted",
            WsEvent::ConfigUpdated { .. } => "ConfigUpdated",
            WsEvent::Connected { .. } => "Connected",
            WsEvent::Disconnected { .. } => "Disconnected",
            WsEvent::Error { .. } => "Error",
            // Battle Intel events
            WsEvent::SummarizationTriggered { .. } => "SummarizationTriggered",
            WsEvent::EdgeCreated { .. } => "EdgeCreated",
            WsEvent::EdgesBatchCreated { .. } => "EdgesBatchCreated",
        }
    }

    /// Check if this event is tenant-specific (most events are).
    pub fn is_tenant_specific(&self) -> bool {
        !matches!(
            self,
            WsEvent::Connected { .. }
                | WsEvent::Disconnected { .. }
                | WsEvent::Error { .. }
                | WsEvent::ConfigUpdated { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_names() {
        let event = WsEvent::TrajectoryCreated {
            trajectory: TrajectoryResponse {
                trajectory_id: caliber_core::new_entity_id(),
                name: "test".to_string(),
                description: None,
                status: caliber_core::TrajectoryStatus::Active,
                parent_trajectory_id: None,
                root_trajectory_id: None,
                agent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                completed_at: None,
                outcome: None,
                metadata: None,
            },
        };
        assert_eq!(event.event_type(), "TrajectoryCreated");
    }

    #[test]
    fn test_tenant_specific_events() {
        let trajectory_event = WsEvent::TrajectoryCreated {
            trajectory: TrajectoryResponse {
                trajectory_id: caliber_core::new_entity_id(),
                name: "test".to_string(),
                description: None,
                status: caliber_core::TrajectoryStatus::Active,
                parent_trajectory_id: None,
                root_trajectory_id: None,
                agent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                completed_at: None,
                outcome: None,
                metadata: None,
            },
        };
        assert!(trajectory_event.is_tenant_specific());

        let connected_event = WsEvent::Connected {
            tenant_id: caliber_core::new_entity_id(),
        };
        assert!(!connected_event.is_tenant_specific());
    }

    #[test]
    fn test_event_serialization() {
        let event = WsEvent::AgentStatusChanged {
            agent_id: caliber_core::new_entity_id(),
            status: "Active".to_string(),
        };

        let json = serde_json::to_string(&event).expect("Failed to serialize");
        let deserialized: WsEvent =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(event, deserialized);
    }
}
