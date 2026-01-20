//! Message-related API types

use caliber_core::{EntityId, Timestamp};
use serde::{Deserialize, Serialize};

/// Request to send a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SendMessageRequest {
    /// Agent sending the message
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    /// Specific agent to receive (if targeted)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<EntityId>,
    /// Agent type to receive (for broadcast)
    pub to_agent_type: Option<String>,
    /// Type of message
    pub message_type: String,
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
    /// Message priority
    pub priority: String,
    /// When the message expires (optional)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expires_at: Option<Timestamp>,
}

/// Message response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MessageResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub message_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<EntityId>,
    pub to_agent_type: Option<String>,
    pub message_type: String,
    pub payload: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifact_ids: Vec<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub delivered_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub acknowledged_at: Option<Timestamp>,
    pub priority: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expires_at: Option<Timestamp>,
}

/// Request to list messages with filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListMessagesRequest {
    /// Filter by message type
    pub message_type: Option<String>,
    /// Filter by sender agent
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub from_agent_id: Option<EntityId>,
    /// Filter by recipient agent
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<EntityId>,
    /// Filter by recipient agent type
    pub to_agent_type: Option<String>,
    /// Filter by trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Filter by priority
    pub priority: Option<String>,
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
