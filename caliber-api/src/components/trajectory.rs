//! Trajectory component implementation.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{
    CreateTrajectoryRequest, ListTrajectoriesRequest, TrajectoryResponse, UpdateTrajectoryRequest,
};
use caliber_core::{AgentId, TenantId, TrajectoryId, TrajectoryStatus};
use serde_json::Value as JsonValue;

// Implement Component trait for TrajectoryResponse
impl_component! {
    TrajectoryResponse {
        entity_name: "trajectory",
        pk_field: "trajectory_id",
        id_type: TrajectoryId,
        requires_tenant: true,
        create_type: CreateTrajectoryRequest,
        update_type: UpdateTrajectoryRequest,
        filter_type: TrajectoryListFilter,
        entity_id: |self| self.trajectory_id,
        create_params: |req, tenant_id| vec![
            SqlParam::String(req.name.clone()),
            SqlParam::OptString(req.description.clone()),
            SqlParam::OptUuid(req.agent_id.map(|id| id.as_uuid())),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 4,
        build_updates: |req| {
            let mut updates = serde_json::Map::new();
            if let Some(name) = &req.name {
                updates.insert("name".to_string(), JsonValue::String(name.clone()));
            }
            if let Some(description) = &req.description {
                updates.insert("description".to_string(), JsonValue::String(description.clone()));
            }
            if let Some(status) = &req.status {
                let status_str = match status {
                    TrajectoryStatus::Active => "active",
                    TrajectoryStatus::Completed => "completed",
                    TrajectoryStatus::Failed => "failed",
                    TrajectoryStatus::Suspended => "suspended",
                };
                updates.insert("status".to_string(), JsonValue::String(status_str.to_string()));
            }
            if let Some(metadata) = &req.metadata {
                updates.insert("metadata".to_string(), metadata.clone());
            }
            JsonValue::Object(updates)
        },
        not_found_error: |id| ApiError::trajectory_not_found(id.as_uuid()),
    }
}

impl TenantScoped for TrajectoryResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}
impl Listable for TrajectoryResponse {}

/// Filter for listing trajectories.
#[derive(Debug, Clone, Default)]
pub struct TrajectoryListFilter {
    pub status: Option<TrajectoryStatus>,
    pub agent_id: Option<AgentId>,
    pub parent_id: Option<TrajectoryId>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<ListTrajectoriesRequest> for TrajectoryListFilter {
    fn from(req: ListTrajectoriesRequest) -> Self {
        Self {
            status: req.status,
            agent_id: req.agent_id,
            parent_id: req.parent_id,
            limit: req.limit,
            offset: req.offset,
        }
    }
}

impl ListFilter for TrajectoryListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(status) = &self.status {
            let status_str = match status {
                TrajectoryStatus::Active => "active",
                TrajectoryStatus::Completed => "completed",
                TrajectoryStatus::Failed => "failed",
                TrajectoryStatus::Suspended => "suspended",
            };
            conditions.push(format!("status = ${}", param_idx));
            params.push(SqlParam::String(status_str.to_string()));
            param_idx += 1;
        }

        if let Some(agent_id) = self.agent_id {
            conditions.push(format!("agent_id = ${}", param_idx));
            params.push(SqlParam::Uuid(agent_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(parent_id) = self.parent_id {
            conditions.push(format!("parent_trajectory_id = ${}", param_idx));
            params.push(SqlParam::Uuid(parent_id.as_uuid()));
            // param_idx += 1; // unused after this
        }

        // Always have at least tenant_id condition, so never empty
        let where_clause = conditions.join(" AND ");
        (Some(where_clause), params)
    }

    fn limit(&self) -> i32 {
        self.limit.unwrap_or(100)
    }

    fn offset(&self) -> i32 {
        self.offset.unwrap_or(0)
    }
}
