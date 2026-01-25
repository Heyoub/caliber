//! Scope component implementation.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::scope::{CreateScopeRequest, ScopeResponse, UpdateScopeRequest};
use caliber_core::{ScopeId, TenantId, TrajectoryId};
use serde_json::Value as JsonValue;

// Implement Component trait for ScopeResponse
impl_component! {
    ScopeResponse {
        entity_name: "scope",
        pk_field: "scope_id",
        id_type: ScopeId,
        requires_tenant: true,
        create_type: CreateScopeRequest,
        update_type: UpdateScopeRequest,
        filter_type: ScopeListFilter,
        entity_id: |self| self.scope_id,
        create_params: |req, tenant_id| vec![
            SqlParam::Uuid(req.trajectory_id.as_uuid()),
            SqlParam::String(req.name.clone()),
            SqlParam::OptString(req.purpose.clone()),
            SqlParam::Int(req.token_budget),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 5,
        build_updates: |req| {
            let mut updates = serde_json::Map::new();
            if let Some(name) = &req.name {
                updates.insert("name".to_string(), JsonValue::String(name.clone()));
            }
            if let Some(purpose) = &req.purpose {
                updates.insert("purpose".to_string(), JsonValue::String(purpose.clone()));
            }
            if let Some(token_budget) = req.token_budget {
                updates.insert("token_budget".to_string(), JsonValue::Number(token_budget.into()));
            }
            if let Some(metadata) = &req.metadata {
                updates.insert("metadata".to_string(), metadata.clone());
            }
            JsonValue::Object(updates)
        },
        not_found_error: |id| ApiError::scope_not_found(id.as_uuid()),
    }
}

impl TenantScoped for ScopeResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}
impl Listable for ScopeResponse {}

/// Filter for listing scopes.
#[derive(Debug, Clone, Default)]
pub struct ScopeListFilter {
    pub trajectory_id: Option<TrajectoryId>,
    pub is_active: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl ListFilter for ScopeListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(trajectory_id) = self.trajectory_id {
            conditions.push(format!("trajectory_id = ${}", param_idx));
            params.push(SqlParam::Uuid(trajectory_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(is_active) = self.is_active {
            conditions.push(format!("is_active = ${}", param_idx));
            params.push(SqlParam::Bool(is_active));
            // param_idx += 1;
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
