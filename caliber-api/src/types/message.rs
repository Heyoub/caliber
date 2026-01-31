//! Message-related API types

use caliber_core::{
    AgentId, ArtifactId, MessageId, MessagePriority, MessageType, ScopeId, TenantId, Timestamp,
    TrajectoryId,
};
use serde::{Deserialize, Serialize};

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};

/// Request to send a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SendMessageRequest {
    /// Agent sending the message
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: AgentId,
    /// Specific agent to receive (if targeted)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<AgentId>,
    /// Agent type to receive (for broadcast)
    pub to_agent_type: Option<String>,
    /// Type of message
    pub message_type: MessageType,
    /// Message payload (JSON serialized)
    pub payload: String,
    /// Related trajectory (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<TrajectoryId>,
    /// Related scope (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<ScopeId>,
    /// Related artifacts (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifact_ids: Vec<ArtifactId>,
    /// Message priority
    pub priority: MessagePriority,
    /// When the message expires (optional)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expires_at: Option<Timestamp>,
}

/// Message response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MessageResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub message_id: MessageId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub sender_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub recipient_id: Option<AgentId>,
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

/// Request to list messages with filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListMessagesRequest {
    /// Filter by message type
    pub message_type: Option<MessageType>,
    /// Filter by sender agent
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub from_agent_id: Option<AgentId>,
    /// Filter by recipient agent
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<AgentId>,
    /// Filter by recipient agent type
    pub to_agent_type: Option<String>,
    /// Filter by trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<TrajectoryId>,
    /// Filter by priority
    pub priority: Option<MessagePriority>,
    /// Only return undelivered messages
    pub undelivered_only: Option<bool>,
    /// Only return unacknowledged messages
    pub unacknowledged_only: Option<bool>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

/// Response containing a list of messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListMessagesResponse {
    /// List of messages
    pub messages: Vec<MessageResponse>,
    /// Total count
    pub total: i32,
}

// ============================================================================
// STATE TRANSITION METHODS
// ============================================================================

impl MessageResponse {
    /// Mark this message as delivered.
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    ///
    /// # Errors
    /// Returns error if the message has already been delivered.
    pub async fn deliver(&self, db: &DbClient) -> ApiResult<Self> {
        if self.delivered_at.is_some() {
            return Err(ApiError::state_conflict(
                "Message has already been delivered",
            ));
        }

        let updates = serde_json::json!({
            "delivered_at": chrono::Utc::now().to_rfc3339()
        });

        db.update_raw::<Self>(self.message_id, updates, self.tenant_id)
            .await
    }

    /// Mark this message as acknowledged.
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    ///
    /// # Errors
    /// Returns error if the message has already been acknowledged.
    pub async fn acknowledge(&self, db: &DbClient) -> ApiResult<Self> {
        if self.acknowledged_at.is_some() {
            return Err(ApiError::state_conflict(
                "Message has already been acknowledged",
            ));
        }

        let updates = serde_json::json!({
            "acknowledged_at": chrono::Utc::now().to_rfc3339()
        });

        db.update_raw::<Self>(self.message_id, updates, self.tenant_id)
            .await
    }
}
