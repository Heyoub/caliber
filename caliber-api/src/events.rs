//! WebSocket Event Types
//!
//! This module defines all event types that are broadcast via WebSocket
//! to connected clients for real-time updates.

use crate::types::*;
use caliber_core::{
    TenantId, TrajectoryId, ScopeId, ArtifactId, NoteId, AgentId,
    LockId, MessageId, DelegationId, HandoffId, EdgeId, SummarizationPolicyId,
};
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
        tenant_id: TenantId,
        /// ID of the deleted trajectory
        id: TrajectoryId,
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
        tenant_id: TenantId,
        /// ID of the deleted artifact
        id: ArtifactId,
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
        tenant_id: TenantId,
        /// ID of the deleted note
        id: NoteId,
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
        tenant_id: TenantId,
        /// ID of the agent
        agent_id: AgentId,
        /// New status
        status: String,
    },

    /// An agent sent a heartbeat.
    AgentHeartbeat {
        /// Tenant ID for filtering
        tenant_id: TenantId,
        /// ID of the agent
        agent_id: AgentId,
        /// Timestamp of the heartbeat
        timestamp: caliber_core::Timestamp,
    },

    /// An agent was unregistered.
    AgentUnregistered {
        /// Tenant ID for filtering
        tenant_id: TenantId,
        /// ID of the unregistered agent
        id: AgentId,
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
        tenant_id: TenantId,
        /// ID of the released lock
        lock_id: LockId,
    },

    /// A lock expired.
    LockExpired {
        /// Tenant ID for filtering
        tenant_id: TenantId,
        /// ID of the expired lock
        lock_id: LockId,
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
        tenant_id: TenantId,
        /// ID of the delivered message
        message_id: MessageId,
    },

    /// A message was acknowledged.
    MessageAcknowledged {
        /// Tenant ID for filtering
        tenant_id: TenantId,
        /// ID of the acknowledged message
        message_id: MessageId,
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
        tenant_id: TenantId,
        /// ID of the accepted delegation
        delegation_id: DelegationId,
    },

    /// A delegation was rejected.
    DelegationRejected {
        /// Tenant ID for filtering
        tenant_id: TenantId,
        /// ID of the rejected delegation
        delegation_id: DelegationId,
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
        tenant_id: TenantId,
        /// ID of the accepted handoff
        handoff_id: HandoffId,
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
        tenant_id: TenantId,
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
        tenant_id: TenantId,
        /// ID of the policy that triggered
        policy_id: SummarizationPolicyId,
        /// The trigger that fired
        trigger: caliber_core::SummarizationTrigger,
        /// Scope ID where trigger fired
        scope_id: ScopeId,
        /// Trajectory ID for context
        trajectory_id: TrajectoryId,
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
        tenant_id: TenantId,
        /// ID of the created edge
        edge_id: EdgeId,
        /// Type of the edge
        edge_type: caliber_core::EdgeType,
    },

    /// Multiple edges were created in a batch.
    EdgesBatchCreated {
        /// Tenant ID for filtering
        tenant_id: TenantId,
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
    use caliber_core::EntityIdType;

    #[test]
    fn test_event_type_names() {
        let event = WsEvent::TrajectoryCreated {
            trajectory: TrajectoryResponse {
                trajectory_id: TrajectoryId::now_v7(),
                tenant_id: TenantId::now_v7(),
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
                trajectory_id: TrajectoryId::now_v7(),
                tenant_id: TenantId::now_v7(),
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
            tenant_id: TenantId::now_v7(),
        };
        assert!(!connected_event.is_tenant_specific());
    }

    #[test]
    fn test_event_serialization() -> Result<(), serde_json::Error> {
        let event = WsEvent::AgentStatusChanged {
            tenant_id: TenantId::now_v7(),
            agent_id: AgentId::now_v7(),
            status: "Active".to_string(),
        };

        let json = serde_json::to_string(&event)?;
        let deserialized: WsEvent = serde_json::from_str(&json)?;

        assert_eq!(event, deserialized);
        Ok(())
    }
}
