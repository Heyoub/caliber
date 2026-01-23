//! Delegation-related API types

use caliber_core::{DelegationResultStatus, DelegationStatus, EntityId, Timestamp};
use serde::{Deserialize, Serialize};

/// Request to create a delegation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateDelegationRequest {
    /// Agent delegating the task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    /// Agent receiving the delegation
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: EntityId,
    /// Trajectory for the delegated task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Scope for the delegated task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    /// Task description
    pub task_description: String,
    /// Expected completion time
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expected_completion: Option<Timestamp>,
    /// Additional context
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub context: Option<serde_json::Value>,
}

/// Delegation response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub delegation_id: EntityId,
    /// Tenant this delegation belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    pub task_description: String,
    pub status: DelegationStatus,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expected_completion: Option<Timestamp>,
    pub result: Option<DelegationResultResponse>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub context: Option<serde_json::Value>,
}

/// Delegation result response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationResultResponse {
    pub status: DelegationResultStatus,
    pub output: Option<String>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifacts: Vec<EntityId>,
    pub error: Option<String>,
}

/// Request to complete a delegation with results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationResultRequest {
    /// Result status
    pub status: DelegationResultStatus,
    /// Output from the delegated task
    pub output: Option<String>,
    /// Artifacts produced during delegation
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifacts: Vec<EntityId>,
    /// Error message if failed
    pub error: Option<String>,
}
