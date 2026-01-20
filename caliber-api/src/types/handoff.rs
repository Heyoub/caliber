//! Handoff-related API types

use caliber_core::{EntityId, Timestamp};
use serde::{Deserialize, Serialize};

/// Request to create a handoff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateHandoffRequest {
    /// Agent initiating the handoff
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    /// Agent receiving the handoff
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: EntityId,
    /// Trajectory being handed off
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Current scope
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    /// Reason for handoff
    pub reason: String,
    /// Context to transfer
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_snapshot: Vec<u8>,
}

/// Handoff response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HandoffResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub handoff_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    pub reason: String,
    pub status: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_snapshot: Vec<u8>,
}
