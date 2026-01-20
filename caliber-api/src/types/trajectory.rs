//! Trajectory-related API types

use caliber_core::{EntityId, OutcomeStatus, Timestamp, TrajectoryStatus};
use serde::{Deserialize, Serialize};

/// Request to create a new trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateTrajectoryRequest {
    /// Name of the trajectory
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Parent trajectory ID (for sub-tasks)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_trajectory_id: Option<EntityId>,
    /// Agent assigned to this trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<EntityId>,
    /// Additional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to update an existing trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateTrajectoryRequest {
    /// New name (if changing)
    pub name: Option<String>,
    /// New description (if changing)
    pub description: Option<String>,
    /// New status (if changing)
    pub status: Option<TrajectoryStatus>,
    /// New metadata (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to list trajectories with filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListTrajectoriesRequest {
    /// Filter by status
    pub status: Option<TrajectoryStatus>,
    /// Filter by agent
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<EntityId>,
    /// Filter by parent trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_id: Option<EntityId>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

/// Response containing a list of trajectories.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListTrajectoriesResponse {
    /// List of trajectories
    pub trajectories: Vec<TrajectoryResponse>,
    /// Total count (before pagination)
    pub total: i32,
}

/// Trajectory response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrajectoryResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Tenant this trajectory belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
    pub name: String,
    pub description: Option<String>,
    pub status: TrajectoryStatus,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_trajectory_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub root_trajectory_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub updated_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    pub outcome: Option<TrajectoryOutcomeResponse>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Trajectory outcome details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrajectoryOutcomeResponse {
    pub status: OutcomeStatus,
    pub summary: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_artifacts: Vec<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_notes: Vec<EntityId>,
    pub error: Option<String>,
}
