//! Agent component implementation.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::agent::{
    AgentResponse, ListAgentsRequest, RegisterAgentRequest, UpdateAgentRequest,
};
use caliber_core::EntityId;
use serde_json::Value as JsonValue;

// Implement Component trait for AgentResponse
impl_component! {
    AgentResponse {
        entity_name: "agent",
        pk_field: "agent_id",
        requires_tenant: true,
        create_type: RegisterAgentRequest,
        update_type: UpdateAgentRequest,
        filter_type: AgentListFilter,
        entity_id: |self| self.agent_id,
        create_params: |req, tenant_id| vec![
            SqlParam::String(req.agent_type.clone()),
            SqlParam::Json(serde_json::to_value(&req.capabilities).unwrap_or(JsonValue::Array(vec![]))),
            SqlParam::Json(serde_json::to_value(&req.memory_access).unwrap_or(JsonValue::Null)),
            SqlParam::Json(serde_json::to_value(&req.can_delegate_to).unwrap_or(JsonValue::Array(vec![]))),
            SqlParam::OptUuid(req.reports_to),
            SqlParam::Uuid(tenant_id),
        ],
        create_param_count: 6,
        build_updates: |req| {
            let mut updates = serde_json::Map::new();
            if let Some(status) = &req.status {
                updates.insert("status".to_string(), JsonValue::String(status.clone()));
            }
            if let Some(current_trajectory_id) = req.current_trajectory_id {
                updates.insert("current_trajectory_id".to_string(), JsonValue::String(current_trajectory_id.to_string()));
            }
            if let Some(current_scope_id) = req.current_scope_id {
                updates.insert("current_scope_id".to_string(), JsonValue::String(current_scope_id.to_string()));
            }
            if let Some(capabilities) = &req.capabilities {
                updates.insert("capabilities".to_string(), serde_json::to_value(capabilities).unwrap_or(JsonValue::Array(vec![])));
            }
            if let Some(memory_access) = &req.memory_access {
                updates.insert("memory_access".to_string(), serde_json::to_value(memory_access).unwrap_or(JsonValue::Null));
            }
            JsonValue::Object(updates)
        },
        not_found_error: |id| ApiError::agent_not_found(id),
    }
}

impl TenantScoped for AgentResponse {}
impl Listable for AgentResponse {}

/// Filter for listing agents.
#[derive(Debug, Clone, Default)]
pub struct AgentListFilter {
    /// Filter by agent type
    pub agent_type: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by current trajectory
    pub trajectory_id: Option<EntityId>,
    /// Only return active agents
    pub active_only: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<ListAgentsRequest> for AgentListFilter {
    fn from(req: ListAgentsRequest) -> Self {
        Self {
            agent_type: req.agent_type,
            status: req.status,
            trajectory_id: req.trajectory_id,
            active_only: req.active_only,
            limit: None,
            offset: None,
        }
    }
}

impl ListFilter for AgentListFilter {
    fn build_where(&self, tenant_id: EntityId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id)];
        let mut param_idx = 2;

        if let Some(agent_type) = &self.agent_type {
            conditions.push(format!("agent_type = ${}", param_idx));
            params.push(SqlParam::String(agent_type.clone()));
            param_idx += 1;
        }

        if let Some(status) = &self.status {
            conditions.push(format!("status = ${}", param_idx));
            params.push(SqlParam::String(status.clone()));
            param_idx += 1;
        }

        if let Some(trajectory_id) = self.trajectory_id {
            conditions.push(format!("current_trajectory_id = ${}", param_idx));
            params.push(SqlParam::Uuid(trajectory_id));
            param_idx += 1;
        }

        if let Some(true) = self.active_only {
            conditions.push(format!("status = ${}", param_idx));
            params.push(SqlParam::String("active".to_string()));
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
