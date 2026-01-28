//! Trajectory-related API types

use caliber_core::{
    AgentId, ArtifactId, NoteId, OutcomeStatus, TenantId, Timestamp, TrajectoryId, TrajectoryStatus,
};
use serde::{Deserialize, Serialize};

use super::{Linkable, Links, LINK_REGISTRY};

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
    pub parent_trajectory_id: Option<TrajectoryId>,
    /// Agent assigned to this trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<AgentId>,
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
    pub agent_id: Option<AgentId>,
    /// Filter by parent trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_id: Option<TrajectoryId>,
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
    pub trajectory_id: TrajectoryId,
    /// Tenant this trajectory belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
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
    pub outcome: Option<TrajectoryOutcomeResponse>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,

    /// HATEOAS links for available actions.
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub links: Option<Links>,
}

impl Linkable for TrajectoryResponse {
    const ENTITY_TYPE: &'static str = "trajectory";

    fn link_id(&self) -> String {
        self.trajectory_id.to_string()
    }

    fn check_condition(&self, condition: &str) -> bool {
        match condition {
            "mutable" => matches!(
                self.status,
                TrajectoryStatus::Active | TrajectoryStatus::Suspended
            ),
            "has_parent" => self.parent_trajectory_id.is_some(),
            _ => true,
        }
    }

    fn relation_id(&self, relation: &str) -> Option<String> {
        match relation {
            "parent_id" => self.parent_trajectory_id.map(|id| id.to_string()),
            _ => None,
        }
    }
}

impl TrajectoryResponse {
    /// Add HATEOAS links from the registry.
    pub fn linked(mut self) -> Self {
        self.links = Some(LINK_REGISTRY.generate(&self));
        self
    }
}

/// Trajectory outcome details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrajectoryOutcomeResponse {
    pub status: OutcomeStatus,
    pub summary: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_artifacts: Vec<ArtifactId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_notes: Vec<NoteId>,
    pub error: Option<String>,
}
