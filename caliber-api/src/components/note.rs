//! Note component implementation.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{CreateNoteRequest, ListNotesRequest, NoteResponse, UpdateNoteRequest};
use caliber_core::{ArtifactId, NoteId, NoteType, TenantId, TrajectoryId};
use serde_json::Value as JsonValue;

// Implement Component trait for NoteResponse
impl_component! {
    NoteResponse {
        entity_name: "note",
        pk_field: "note_id",
        id_type: NoteId,
        requires_tenant: true,
        create_type: CreateNoteRequest,
        update_type: UpdateNoteRequest,
        filter_type: NoteListFilter,
        entity_id: |self| self.note_id,
        create_params: |req, tenant_id| vec![
            SqlParam::String(format!("{:?}", req.note_type)),
            SqlParam::String(req.title.clone()),
            SqlParam::String(req.content.clone()),
            SqlParam::Json(serde_json::to_value(&req.source_trajectory_ids.iter().map(|id| id.as_uuid()).collect::<Vec<_>>()).unwrap_or(JsonValue::Array(vec![]))),
            SqlParam::Json(serde_json::to_value(&req.source_artifact_ids.iter().map(|id| id.as_uuid()).collect::<Vec<_>>()).unwrap_or(JsonValue::Array(vec![]))),
            SqlParam::Json(serde_json::to_value(&req.ttl).unwrap_or(JsonValue::Null)),
            SqlParam::OptJson(req.metadata.clone()),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 8,
        build_updates: |req| {
            let mut updates = serde_json::Map::new();
            if let Some(title) = &req.title {
                updates.insert("title".to_string(), JsonValue::String(title.clone()));
            }
            if let Some(content) = &req.content {
                updates.insert("content".to_string(), JsonValue::String(content.clone()));
            }
            if let Some(note_type) = &req.note_type {
                updates.insert("note_type".to_string(), JsonValue::String(format!("{:?}", note_type)));
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
        not_found_error: |id| ApiError::note_not_found(id.as_uuid()),
    }
}

impl TenantScoped for NoteResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}
impl Listable for NoteResponse {}

/// Filter for listing notes.
#[derive(Debug, Clone, Default)]
pub struct NoteListFilter {
    pub note_type: Option<NoteType>,
    pub source_trajectory_id: Option<TrajectoryId>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<ListNotesRequest> for NoteListFilter {
    fn from(req: ListNotesRequest) -> Self {
        Self {
            note_type: req.note_type,
            source_trajectory_id: req.source_trajectory_id,
            limit: req.limit,
            offset: req.offset,
        }
    }
}

impl ListFilter for NoteListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(note_type) = &self.note_type {
            conditions.push(format!("note_type = ${}", param_idx));
            params.push(SqlParam::String(format!("{:?}", note_type)));
            param_idx += 1;
        }

        if let Some(trajectory_id) = self.source_trajectory_id {
            // Note: source_trajectory_ids is stored as a JSONB array, so we use the contains operator
            conditions.push(format!("source_trajectory_ids @> ${}::jsonb", param_idx));
            params.push(SqlParam::Json(serde_json::json!([trajectory_id.as_uuid().to_string()])));
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
