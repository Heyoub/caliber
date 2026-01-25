//! Handoff-related API types

use caliber_core::{AgentId, HandoffId, HandoffStatus, ScopeId, TenantId, Timestamp, TrajectoryId};
use serde::{Deserialize, Serialize};

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};

/// Request to create a handoff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateHandoffRequest {
    /// Agent initiating the handoff
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: AgentId,
    /// Agent receiving the handoff
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: AgentId,
    /// Trajectory being handed off
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    /// Current scope
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
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
    pub tenant_id: TenantId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub handoff_id: HandoffId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<ScopeId>,
    pub reason: String,
    pub status: HandoffStatus,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_snapshot: Vec<u8>,
}

// ============================================================================
// STATE TRANSITION METHODS
// ============================================================================

impl HandoffResponse {
    /// Accept this handoff (Initiated -> Accepted transition).
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    /// - `accepting_agent_id`: ID of the agent accepting the handoff
    ///
    /// # Errors
    /// Returns error if handoff is not in Initiated state or agent is not the recipient.
    pub async fn accept(&self, db: &DbClient, accepting_agent_id: AgentId) -> ApiResult<Self> {
        if self.status != HandoffStatus::Initiated {
            return Err(ApiError::state_conflict(format!(
                "Handoff is in '{:?}' state, cannot accept (expected Initiated)",
                self.status
            )));
        }

        // Verify the accepting agent is the recipient
        if self.to_agent_id != accepting_agent_id {
            return Err(ApiError::forbidden(
                "Only the recipient agent can accept this handoff",
            ));
        }

        let updates = serde_json::json!({
            "status": "Accepted",
            "accepted_at": chrono::Utc::now().to_rfc3339()
        });

        db.update_raw::<Self>(self.handoff_id, updates, self.tenant_id).await
    }

    /// Complete this handoff (Accepted -> Completed transition).
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    ///
    /// # Errors
    /// Returns error if handoff is not in Accepted state.
    pub async fn complete(&self, db: &DbClient) -> ApiResult<Self> {
        if self.status != HandoffStatus::Accepted {
            return Err(ApiError::state_conflict(format!(
                "Handoff is in '{:?}' state, cannot complete (expected Accepted)",
                self.status
            )));
        }

        let updates = serde_json::json!({
            "status": "Completed",
            "completed_at": chrono::Utc::now().to_rfc3339()
        });

        db.update_raw::<Self>(self.handoff_id, updates, self.tenant_id).await
    }
}
