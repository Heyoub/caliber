//! Delegation-related API types

use caliber_core::{AgentId, ArtifactId, DelegationId, DelegationResultStatus, DelegationStatus, ScopeId, TenantId, Timestamp, TrajectoryId};
use serde::{Deserialize, Serialize};

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};

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
// STATE TRANSITION METHODS
// ============================================================================

impl DelegationResponse {
    /// Accept this delegation (Pending -> Accepted transition).
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    /// - `accepting_agent_id`: ID of the agent accepting the delegation
    ///
    /// # Errors
    /// Returns error if delegation is not in Pending state.
    pub async fn accept(&self, db: &DbClient, accepting_agent_id: AgentId) -> ApiResult<Self> {
        if self.status != DelegationStatus::Pending {
            return Err(ApiError::state_conflict(format!(
                "Delegation is in '{:?}' state, cannot accept (expected Pending)",
                self.status
            )));
        }

        // Verify the accepting agent is the delegatee
        if self.delegatee_id != accepting_agent_id {
            return Err(ApiError::forbidden(
                "Only the delegatee can accept this delegation",
            ));
        }

        let updates = serde_json::json!({
            "status": "Accepted",
            "accepted_at": chrono::Utc::now().to_rfc3339()
        });

        db.update_raw::<Self>(self.delegation_id, updates, self.tenant_id).await
    }

    /// Reject this delegation (Pending -> Rejected transition).
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    /// - `rejecting_agent_id`: ID of the agent rejecting the delegation
    /// - `reason`: Reason for rejection
    ///
    /// # Errors
    /// Returns error if delegation is not in Pending state.
    pub async fn reject(&self, db: &DbClient, rejecting_agent_id: AgentId, reason: &str) -> ApiResult<Self> {
        if self.status != DelegationStatus::Pending {
            return Err(ApiError::state_conflict(format!(
                "Delegation is in '{:?}' state, cannot reject (expected Pending)",
                self.status
            )));
        }

        // Verify the rejecting agent is the delegatee
        if self.delegatee_id != rejecting_agent_id {
            return Err(ApiError::forbidden(
                "Only the delegatee can reject this delegation",
            ));
        }

        let updates = serde_json::json!({
            "status": "Rejected",
            "rejection_reason": reason
        });

        db.update_raw::<Self>(self.delegation_id, updates, self.tenant_id).await
    }

    /// Complete this delegation (Accepted/InProgress -> Completed transition).
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    /// - `result`: The result of the delegation
    ///
    /// # Errors
    /// Returns error if delegation is not in Accepted or InProgress state.
    pub async fn complete(&self, db: &DbClient, result: &DelegationResultResponse) -> ApiResult<Self> {
        let can_complete = matches!(
            self.status,
            DelegationStatus::Accepted | DelegationStatus::InProgress
        );

        if !can_complete {
            return Err(ApiError::state_conflict(format!(
                "Delegation is in '{:?}' state, cannot complete (expected Accepted or InProgress)",
                self.status
            )));
        }

        let result_json = serde_json::to_value(result)?;

        let updates = serde_json::json!({
            "status": "Completed",
            "completed_at": chrono::Utc::now().to_rfc3339(),
            "result": result_json
        });

        db.update_raw::<Self>(self.delegation_id, updates, self.tenant_id).await
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
