//! Turn component implementation.
//!
//! Turns are immutable - once created, they cannot be updated.
//! This ensures conversation history integrity.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{CreateTurnRequest, TurnResponse};
use caliber_core::{ScopeId, TenantId, TurnId};
use serde_json::Value as JsonValue;

// Implement Component trait for TurnResponse
// Note: Update type is () because turns are immutable
impl_component! {
    TurnResponse {
        entity_name: "turn",
        pk_field: "turn_id",
        id_type: TurnId,
        requires_tenant: true,
        create_type: CreateTurnRequest,
        update_type: (),
        filter_type: TurnListFilter,
        entity_id: |self| self.turn_id,
        create_params: |req, tenant_id| vec![
            SqlParam::Uuid(req.scope_id.as_uuid()),
            SqlParam::Int(req.sequence),
            SqlParam::String(format!("{:?}", req.role)),
            SqlParam::String(req.content.clone()),
            SqlParam::Int(req.token_count),
            SqlParam::OptJson(req.tool_calls.clone()),
            SqlParam::OptJson(req.tool_results.clone()),
            SqlParam::OptJson(req.metadata.clone()),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 9,
        build_updates: |_req| {
            // Turns are immutable - no updates allowed
            JsonValue::Object(serde_json::Map::new())
        },
        not_found_error: |id| ApiError::entity_not_found("Turn", id.as_uuid()),
    }
}

impl TenantScoped for TurnResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}
impl Listable for TurnResponse {}

/// Filter for listing turns.
#[derive(Debug, Clone, Default)]
pub struct TurnListFilter {
    pub scope_id: Option<ScopeId>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl ListFilter for TurnListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(scope_id) = self.scope_id {
            conditions.push(format!("scope_id = ${}", param_idx));
            params.push(SqlParam::Uuid(scope_id.as_uuid()));
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
