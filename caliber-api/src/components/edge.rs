//! Edge component implementation.
//!
//! Edges represent relationships between entities (artifacts, notes, etc.).
//! Edges are immutable and can be cross-tenant (requires_tenant: false).

use crate::component::{impl_component, ListFilter, Listable, SqlParam};
use crate::error::ApiError;
use crate::types::{CreateEdgeRequest, EdgeResponse};
use caliber_core::{EdgeType, EntityId};
use serde_json::Value as JsonValue;

// Implement Component trait for EdgeResponse
// Note: Update type is () because edges are immutable
// Note: requires_tenant is false because edges can be cross-tenant
impl_component! {
    EdgeResponse {
        entity_name: "edge",
        pk_field: "edge_id",
        requires_tenant: false,
        create_type: CreateEdgeRequest,
        update_type: (),
        filter_type: EdgeListFilter,
        entity_id: |self| self.edge_id,
        create_params: |req, _tenant_id| vec![
            SqlParam::String(format!("{:?}", req.edge_type)),
            SqlParam::Json(serde_json::to_value(&req.participants).unwrap_or(JsonValue::Array(vec![]))),
            SqlParam::OptFloat(req.weight),
            SqlParam::OptUuid(req.trajectory_id),
            SqlParam::Int(req.provenance.source_turn),
            SqlParam::String(format!("{:?}", req.provenance.extraction_method)),
            SqlParam::OptFloat(req.provenance.confidence),
            SqlParam::OptJson(req.metadata.clone()),
        ],
        create_param_count: 8,
        build_updates: |_req| {
            // Edges are immutable - no updates allowed
            JsonValue::Object(serde_json::Map::new())
        },
        not_found_error: |id| ApiError::entity_not_found("Edge", id),
    }
}

// Note: EdgeResponse does NOT implement TenantScoped because edges can be cross-tenant
impl Listable for EdgeResponse {}

/// Filter for listing edges.
#[derive(Debug, Clone, Default)]
pub struct EdgeListFilter {
    /// Filter by source entity ID (any participant with "source" role)
    pub source_id: Option<EntityId>,
    /// Filter by target entity ID (any participant with "target" role)
    pub target_id: Option<EntityId>,
    /// Filter by any participant entity ID (regardless of role)
    pub participant_id: Option<EntityId>,
    /// Filter by tenant ID for cross-tenant edges
    pub tenant_id: Option<EntityId>,
    /// Filter by edge type
    pub edge_type: Option<EdgeType>,
    /// Filter by trajectory context
    pub trajectory_id: Option<EntityId>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl ListFilter for EdgeListFilter {
    fn build_where(&self, _tenant_id: EntityId) -> (Option<String>, Vec<SqlParam>) {
        // Edges don't require tenant filtering, but we still build conditions
        let mut conditions = Vec::new();
        let mut params = Vec::new();
        let mut param_idx = 1;

        // Filter by tenant_id (for cross-tenant edge filtering)
        if let Some(tenant_id) = self.tenant_id {
            conditions.push(format!("tenant_id = ${}", param_idx));
            params.push(SqlParam::Uuid(tenant_id));
            param_idx += 1;
        }

        if let Some(edge_type) = &self.edge_type {
            conditions.push(format!("edge_type = ${}", param_idx));
            params.push(SqlParam::String(format!("{:?}", edge_type)));
            param_idx += 1;
        }

        if let Some(trajectory_id) = self.trajectory_id {
            conditions.push(format!("trajectory_id = ${}", param_idx));
            params.push(SqlParam::Uuid(trajectory_id));
            param_idx += 1;
        }

        // Filter by any participant (regardless of role)
        if let Some(participant_id) = self.participant_id {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM jsonb_array_elements(participants) p WHERE p->>'entity_id' = ${})",
                param_idx
            ));
            params.push(SqlParam::String(participant_id.to_string()));
            param_idx += 1;
        }

        // For source_id and target_id, we need to query the JSONB participants array
        if let Some(source_id) = self.source_id {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM jsonb_array_elements(participants) p WHERE p->>'entity_id' = ${} AND (p->>'role' = 'source' OR p->>'role' IS NULL))",
                param_idx
            ));
            params.push(SqlParam::String(source_id.to_string()));
            param_idx += 1;
        }

        if let Some(target_id) = self.target_id {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM jsonb_array_elements(participants) p WHERE p->>'entity_id' = ${} AND p->>'role' = 'target')",
                param_idx
            ));
            params.push(SqlParam::String(target_id.to_string()));
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
