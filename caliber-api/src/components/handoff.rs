//! Handoff component implementation.
//!
//! Handoffs represent trajectory transfers from one agent to another.
//! They have state machine operations: accept, complete.
//! Updates are handled via state transitions, not direct field updates.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{CreateHandoffRequest, HandoffResponse};
use caliber_core::{AgentId, EntityIdType, HandoffId, TenantId, TrajectoryId};
use serde_json::Value as JsonValue;

// Implement Component trait for HandoffResponse
impl_component! {
    HandoffResponse {
        entity_name: "handoff",
        pk_field: "handoff_id",
        id_type: HandoffId,
        requires_tenant: true,
        create_type: CreateHandoffRequest,
        update_type: (),  // State transitions via accept/complete, not direct updates
        filter_type: HandoffListFilter,
        entity_id: |self| self.handoff_id,
        create_params: |req, tenant_id| vec![
            SqlParam::Uuid(req.from_agent_id.as_uuid()),
            SqlParam::Uuid(req.to_agent_id.as_uuid()),
            SqlParam::Uuid(req.trajectory_id.as_uuid()),
            SqlParam::Uuid(req.scope_id.as_uuid()),
            SqlParam::String(req.reason.clone()),
            SqlParam::Bytes(req.context_snapshot.clone()),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 7,
        build_updates: |_req| {
            // Handoffs use state machine transitions (accept/complete)
            // rather than direct field updates
            JsonValue::Object(serde_json::Map::new())
        },
        not_found_error: |id| ApiError::entity_not_found("Handoff", id.as_uuid()),
    }
}

impl TenantScoped for HandoffResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}

impl Listable for HandoffResponse {}

/// Filter for listing handoffs.
#[derive(Debug, Clone, Default)]
pub struct HandoffListFilter {
    /// Filter by source agent
    pub from_agent_id: Option<AgentId>,
    /// Filter by destination agent
    pub to_agent_id: Option<AgentId>,
    /// Filter by trajectory
    pub trajectory_id: Option<TrajectoryId>,
    /// Filter by status (pending, accepted, completed)
    pub status: Option<String>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

impl ListFilter for HandoffListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(from_agent) = self.from_agent_id {
            conditions.push(format!("from_agent_id = ${}", param_idx));
            params.push(SqlParam::Uuid(from_agent.as_uuid()));
            param_idx += 1;
        }

        if let Some(to_agent) = self.to_agent_id {
            conditions.push(format!("to_agent_id = ${}", param_idx));
            params.push(SqlParam::Uuid(to_agent.as_uuid()));
            param_idx += 1;
        }

        if let Some(trajectory_id) = self.trajectory_id {
            conditions.push(format!("trajectory_id = ${}", param_idx));
            params.push(SqlParam::Uuid(trajectory_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(status) = &self.status {
            conditions.push(format!("status = ${}", param_idx));
            params.push(SqlParam::String(status.clone()));
            // param_idx += 1; // unused after this
        }

        if conditions.is_empty() {
            (None, params)
        } else {
            (Some(conditions.join(" AND ")), params)
        }
    }

    fn limit(&self) -> i32 {
        self.limit.unwrap_or(100)
    }

    fn offset(&self) -> i32 {
        self.offset.unwrap_or(0)
    }
}
