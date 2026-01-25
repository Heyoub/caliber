//! Delegation component implementation.
//!
//! Delegations represent task assignments from one agent to another.
//! They have state machine operations: accept, reject, complete.
//! Updates are handled via state transitions, not direct field updates.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{CreateDelegationRequest, DelegationResponse};
use caliber_core::{AgentId, DelegationId, EntityIdType, TenantId, TrajectoryId};
use serde_json::Value as JsonValue;

// Implement Component trait for DelegationResponse
impl_component! {
    DelegationResponse {
        entity_name: "delegation",
        pk_field: "delegation_id",
        id_type: DelegationId,
        requires_tenant: true,
        create_type: CreateDelegationRequest,
        update_type: (),  // State transitions via accept/reject/complete, not direct updates
        filter_type: DelegationListFilter,
        entity_id: |self| self.delegation_id,
        create_params: |req, tenant_id| vec![
            SqlParam::Uuid(req.from_agent_id.as_uuid()),
            SqlParam::Uuid(req.to_agent_id.as_uuid()),
            SqlParam::Uuid(req.trajectory_id.as_uuid()),
            SqlParam::Uuid(req.scope_id.as_uuid()),
            SqlParam::String(req.task_description.clone()),
            SqlParam::OptTimestamp(req.expected_completion),
            SqlParam::OptJson(req.context.clone()),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 8,
        build_updates: |_req| {
            // Delegations use state machine transitions (accept/reject/complete)
            // rather than direct field updates
            JsonValue::Object(serde_json::Map::new())
        },
        not_found_error: |id| ApiError::entity_not_found("Delegation", id.as_uuid()),
    }
}

impl TenantScoped for DelegationResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}

impl Listable for DelegationResponse {}

/// Filter for listing delegations.
#[derive(Debug, Clone, Default)]
pub struct DelegationListFilter {
    /// Filter by delegating agent
    pub delegator_agent_id: Option<AgentId>,
    /// Filter by receiving agent
    pub delegatee_agent_id: Option<AgentId>,
    /// Filter by status (pending, accepted, completed, rejected)
    pub status: Option<String>,
    /// Filter by trajectory
    pub trajectory_id: Option<TrajectoryId>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

impl ListFilter for DelegationListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(delegator_id) = self.delegator_agent_id {
            conditions.push(format!("from_agent_id = ${}", param_idx));
            params.push(SqlParam::Uuid(delegator_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(delegatee_id) = self.delegatee_agent_id {
            conditions.push(format!("to_agent_id = ${}", param_idx));
            params.push(SqlParam::Uuid(delegatee_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(status) = &self.status {
            conditions.push(format!("status = ${}", param_idx));
            params.push(SqlParam::String(status.clone()));
            param_idx += 1;
        }

        if let Some(trajectory_id) = self.trajectory_id {
            conditions.push(format!("trajectory_id = ${}", param_idx));
            params.push(SqlParam::Uuid(trajectory_id.as_uuid()));
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
