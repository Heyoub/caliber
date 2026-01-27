//! Delegation-related API types

use caliber_core::{AgentId, ArtifactId, DelegationId, DelegationResultStatus, DelegationStatus, ScopeId, TenantId, Timestamp, TrajectoryId};
use serde::{Deserialize, Serialize};

use crate::db::DbClient;
use crate::error::ApiResult;

/// Request to create a delegation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateDelegationRequest {
    /// Agent delegating the task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: AgentId,
    /// Agent receiving the delegation
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: AgentId,
    /// Trajectory for the delegated task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    /// Scope for the delegated task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
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
    pub delegation_id: DelegationId,
    /// Tenant this delegation belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub delegator_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub delegatee_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<ScopeId>,
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

// ============================================================================
// STATE TRANSITION METHODS (delegating to service layer)
// ============================================================================

impl DelegationResponse {
    /// Accept this delegation (Pending -> Accepted transition).
    ///
    /// Delegates to `services::accept_delegation()`.
    pub async fn accept(&self, db: &DbClient, accepting_agent_id: AgentId) -> ApiResult<Self> {
        crate::services::accept_delegation(db, self, accepting_agent_id).await
    }

    /// Reject this delegation (Pending -> Rejected transition).
    ///
    /// Delegates to `services::reject_delegation()`.
    pub async fn reject(&self, db: &DbClient, rejecting_agent_id: AgentId, reason: &str) -> ApiResult<Self> {
        crate::services::reject_delegation(db, self, rejecting_agent_id, reason).await
    }

    /// Complete this delegation (Accepted/InProgress -> Completed transition).
    ///
    /// Delegates to `services::complete_delegation()`.
    pub async fn complete(&self, db: &DbClient, result: &DelegationResultResponse) -> ApiResult<Self> {
        crate::services::complete_delegation(db, self, result).await
    }
}

/// Delegation result response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationResultResponse {
    pub status: DelegationResultStatus,
    pub output: Option<String>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifacts: Vec<ArtifactId>,
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
    pub artifacts: Vec<ArtifactId>,
    /// Error message if failed
    pub error: Option<String>,
}
