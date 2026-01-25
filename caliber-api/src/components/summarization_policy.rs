//! SummarizationPolicy component implementation.
//!
//! Summarization policies define when and how note/artifact summarization occurs.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{CreateSummarizationPolicyRequest, SummarizationPolicyResponse};
use caliber_core::{SummarizationPolicyId, TenantId, TrajectoryId};
use serde_json::Value as JsonValue;

// Implement Component trait for SummarizationPolicyResponse
impl_component! {
    SummarizationPolicyResponse {
        entity_name: "summarization_policy",
        pk_field: "policy_id",
        id_type: SummarizationPolicyId,
        requires_tenant: true,
        create_type: CreateSummarizationPolicyRequest,
        update_type: (),  // Policies are immutable; create new ones to change behavior
        filter_type: SummarizationPolicyListFilter,
        entity_id: |self| self.policy_id,
        create_params: |req, tenant_id| vec![
            SqlParam::Uuid(tenant_id.as_uuid()),
            SqlParam::String(req.name.clone()),
            SqlParam::Json(serde_json::to_value(&req.triggers).unwrap_or(JsonValue::Array(vec![]))),
            SqlParam::String(format!("{:?}", req.source_level)),
            SqlParam::String(format!("{:?}", req.target_level)),
            SqlParam::Int(req.max_sources),
            SqlParam::Bool(req.create_edges),
            SqlParam::OptUuid(req.trajectory_id.map(|id| id.as_uuid())),
            SqlParam::OptJson(req.metadata.clone()),
        ],
        create_param_count: 9,
        build_updates: |_req| {
            // Policies are immutable - no updates allowed
            JsonValue::Object(serde_json::Map::new())
        },
        not_found_error: |id| ApiError::entity_not_found("SummarizationPolicy", id),
    }
}

impl TenantScoped for SummarizationPolicyResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}
impl Listable for SummarizationPolicyResponse {}

/// Filter for listing summarization policies.
#[derive(Debug, Clone, Default)]
pub struct SummarizationPolicyListFilter {
    /// Filter by trajectory ID
    pub trajectory_id: Option<TrajectoryId>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

impl ListFilter for SummarizationPolicyListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(trajectory_id) = self.trajectory_id {
            conditions.push(format!("trajectory_id = ${}", param_idx));
            params.push(SqlParam::Uuid(trajectory_id.as_uuid()));
            // param_idx += 1; // unused after this
        }

        (Some(conditions.join(" AND ")), params)
    }

    fn limit(&self) -> i32 {
        self.limit.unwrap_or(100)
    }

    fn offset(&self) -> i32 {
        self.offset.unwrap_or(0)
    }
}
