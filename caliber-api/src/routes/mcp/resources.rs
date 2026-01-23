//! MCP resource handlers

use super::tools::get_available_resources;
use super::{types::*, McpState};
use crate::components::{NoteListFilter, TrajectoryListFilter};
use crate::types::{ArtifactResponse, NoteResponse, TrajectoryResponse};
use crate::*;
use axum::{extract::State, response::IntoResponse, Json};
use caliber_core::EntityId;
use std::sync::Arc;
use uuid::Uuid;

pub async fn list_resources(
    State(state): State<Arc<McpState>>,
) -> impl IntoResponse {
    tracing::debug!(db_pool_size = state.db.pool_size(), "MCP list_resources");
    Json(ListResourcesResponse {
        resources: get_available_resources(),
    })
}

/// POST /mcp/resources/read - Read a resource
#[utoipa::path(
    post,
    path = "/mcp/resources/read",
    tag = "MCP",
    request_body = ReadResourceRequest,
    responses(
        (status = 200, description = "Resource contents", body = ReadResourceResponse),
        (status = 404, description = "Resource not found", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn read_resource(
    State(state): State<Arc<McpState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<ReadResourceRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::debug!(uri = %req.uri, "MCP resource read");

    let content = read_resource_content(&state, &req.uri, auth.tenant_id).await?;

    Ok(Json(ReadResourceResponse {
        contents: vec![content],
    }))
}

async fn read_resource_content(
    state: &McpState,
    uri: &str,
    tenant_id: EntityId,
) -> ApiResult<ResourceContent> {
    match uri {
        "caliber://trajectories" => {
            let filter = TrajectoryListFilter {
                status: Some(caliber_core::TrajectoryStatus::Active),
                ..Default::default()
            };
            let trajectories = state.db.list::<TrajectoryResponse>(&filter, tenant_id).await?;

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&trajectories).unwrap_or_default()),
                blob: None,
            })
        }

        "caliber://agents" => {
            let agents = state.db.agent_list_active().await?;

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&agents).unwrap_or_default()),
                blob: None,
            })
        }

        "caliber://notes" => {
            // Fetch recent notes for this tenant
            let filter = NoteListFilter {
                limit: Some(100),
                offset: Some(0),
                ..Default::default()
            };
            let notes = state.db.list::<NoteResponse>(&filter, tenant_id).await?;

            #[derive(serde::Serialize)]
            struct NoteResourceView {
                note_id: EntityId,
                note_type: String,
                title: String,
                created_at: caliber_core::Timestamp,
                uri: String,
            }

            let resource_notes: Vec<NoteResourceView> = notes
                .into_iter()
                .map(|n| NoteResourceView {
                    note_id: n.note_id,
                    note_type: format!("{:?}", n.note_type),
                    title: n.title.clone(),
                    created_at: n.created_at,
                    uri: format!("caliber://note/{}", n.note_id),
                })
                .collect();

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&resource_notes).unwrap_or_default()),
                blob: None,
            })
        }

        "caliber://artifacts" => {
            // Fetch recent artifacts across all trajectories for this tenant
            let artifacts = state.db.artifact_list_recent(tenant_id, 100).await?;

            #[derive(serde::Serialize)]
            struct ArtifactResourceView {
                artifact_id: EntityId,
                trajectory_id: EntityId,
                artifact_type: String,
                name: String,
                uri: String,
            }

            let resource_artifacts: Vec<ArtifactResourceView> = artifacts
                .into_iter()
                .map(|a| ArtifactResourceView {
                    artifact_id: a.artifact_id,
                    trajectory_id: a.trajectory_id,
                    artifact_type: format!("{:?}", a.artifact_type),
                    name: a.name.clone(),
                    uri: format!("caliber://artifact/{}", a.artifact_id),
                })
                .collect();

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&resource_artifacts).unwrap_or_default()),
                blob: None,
            })
        }

        uri if uri.starts_with("caliber://trajectory/") => {
            let id_str = uri.trim_start_matches("caliber://trajectory/");
            let id = Uuid::parse_str(id_str)
                .map_err(|_| ApiError::invalid_input("Invalid trajectory ID in URI"))?;

            let trajectory = state
                .db
                .get::<TrajectoryResponse>(id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::trajectory_not_found(id))?;

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&trajectory).unwrap_or_default()),
                blob: None,
            })
        }

        uri if uri.starts_with("caliber://note/") => {
            let id_str = uri.trim_start_matches("caliber://note/");
            let id = Uuid::parse_str(id_str)
                .map_err(|_| ApiError::invalid_input("Invalid note ID in URI"))?;

            let note = state
                .db
                .get::<NoteResponse>(id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::note_not_found(id))?;

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&note).unwrap_or_default()),
                blob: None,
            })
        }

        uri if uri.starts_with("caliber://artifact/") => {
            let id_str = uri.trim_start_matches("caliber://artifact/");
            let id = Uuid::parse_str(id_str)
                .map_err(|_| ApiError::invalid_input("Invalid artifact ID in URI"))?;

            let artifact = state
                .db
                .get::<ArtifactResponse>(id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::artifact_not_found(id))?;

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&artifact).unwrap_or_default()),
                blob: None,
            })
        }

        _ => Err(ApiError::entity_not_found("Resource", Uuid::nil())),
    }
}

