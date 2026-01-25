//! Artifact component implementation.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{
    ArtifactResponse, CreateArtifactRequest, ListArtifactsRequest, UpdateArtifactRequest,
};
use caliber_core::{ArtifactId, ArtifactType, EntityIdType, ScopeId, TenantId, TrajectoryId};
use serde_json::Value as JsonValue;

// Implement Component trait for ArtifactResponse
impl_component! {
    ArtifactResponse {
        entity_name: "artifact",
        pk_field: "artifact_id",
        id_type: ArtifactId,
        requires_tenant: true,
        create_type: CreateArtifactRequest,
        update_type: UpdateArtifactRequest,
        filter_type: ArtifactListFilter,
        entity_id: |self| self.artifact_id,
        create_params: |req, tenant_id| vec![
            SqlParam::Uuid(req.trajectory_id.as_uuid()),
            SqlParam::Uuid(req.scope_id.as_uuid()),
            SqlParam::String(format!("{:?}", req.artifact_type)),
            SqlParam::String(req.name.clone()),
            SqlParam::String(req.content.clone()),
            SqlParam::Int(req.source_turn),
            SqlParam::String(format!("{:?}", req.extraction_method)),
            SqlParam::OptFloat(req.confidence),
            SqlParam::Json(serde_json::to_value(&req.ttl).unwrap_or(JsonValue::Null)),
            SqlParam::OptJson(req.metadata.clone()),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 11,
        build_updates: |req| {
            let mut updates = serde_json::Map::new();
            if let Some(name) = &req.name {
                updates.insert("name".to_string(), JsonValue::String(name.clone()));
            }
            if let Some(content) = &req.content {
                updates.insert("content".to_string(), JsonValue::String(content.clone()));
            }
            if let Some(artifact_type) = &req.artifact_type {
                updates.insert("artifact_type".to_string(), JsonValue::String(format!("{:?}", artifact_type)));
            }
            if let Some(ttl) = &req.ttl {
                if let Ok(ttl_value) = serde_json::to_value(ttl) {
                    updates.insert("ttl".to_string(), ttl_value);
                }
            }
            if let Some(metadata) = &req.metadata {
                updates.insert("metadata".to_string(), metadata.clone());
            }
            JsonValue::Object(updates)
        },
        not_found_error: |id| ApiError::artifact_not_found(id.as_uuid()),
    }
}

impl TenantScoped for ArtifactResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}
impl Listable for ArtifactResponse {}

/// Filter for listing artifacts.
#[derive(Debug, Clone, Default)]
pub struct ArtifactListFilter {
    pub trajectory_id: Option<TrajectoryId>,
    pub scope_id: Option<ScopeId>,
    pub artifact_type: Option<ArtifactType>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<ListArtifactsRequest> for ArtifactListFilter {
    fn from(req: ListArtifactsRequest) -> Self {
        Self {
            trajectory_id: req.trajectory_id,
            scope_id: req.scope_id,
            artifact_type: req.artifact_type,
            limit: req.limit,
            offset: req.offset,
        }
    }
}

impl ListFilter for ArtifactListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(trajectory_id) = self.trajectory_id {
            conditions.push(format!("trajectory_id = ${}", param_idx));
            params.push(SqlParam::Uuid(trajectory_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(scope_id) = self.scope_id {
            conditions.push(format!("scope_id = ${}", param_idx));
            params.push(SqlParam::Uuid(scope_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(artifact_type) = &self.artifact_type {
            conditions.push(format!("artifact_type = ${}", param_idx));
            params.push(SqlParam::String(format!("{:?}", artifact_type)));
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
