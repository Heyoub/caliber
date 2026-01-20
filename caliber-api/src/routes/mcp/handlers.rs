//! MCP handler functions

use super::{types::*, tools::*};
use crate::*;
use axum::{extract::State, response::IntoResponse, Json};
use caliber_core::EntityId;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

pub async fn initialize(
    State(state): State<Arc<McpState>>,
    Json(req): Json<InitializeRequest>,
) -> impl IntoResponse {
    tracing::debug!(db_pool_size = state.db.pool_size(), "MCP initialize");
    tracing::info!(
        client_name = %req.client_info.name,
        client_version = %req.client_info.version,
        protocol_version = %req.protocol_version,
        "MCP session initialized"
    );

    let response = InitializeResponse {
        protocol_version: MCP_PROTOCOL_VERSION.to_string(),
        capabilities: ServerCapabilities {
            tools: ToolsCapability { list_changed: false },
            resources: ResourcesCapability {
                subscribe: false,
                list_changed: false,
            },
            prompts: None,
        },
        server_info: ServerInfo {
            name: "CALIBER MCP Server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    Json(response)
}

/// GET /mcp/tools/list - List available tools
#[utoipa::path(
    get,
    path = "/mcp/tools/list",
    tag = "MCP",
    responses(
        (status = 200, description = "List of available tools", body = ListToolsResponse),
    ),
)]
pub async fn list_tools(
    State(state): State<Arc<McpState>>,
) -> impl IntoResponse {
    tracing::debug!(db_pool_size = state.db.pool_size(), "MCP list_tools");
    Json(ListToolsResponse {
        tools: get_available_tools(),
    })
}

/// POST /mcp/tools/call - Execute a tool
#[utoipa::path(
    post,
    path = "/mcp/tools/call",
    tag = "MCP",
    request_body = CallToolRequest,
    responses(
        (status = 200, description = "Tool execution result", body = CallToolResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Tool not found", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn call_tool(
    State(state): State<Arc<McpState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<CallToolRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::debug!(tool = %req.name, "MCP tool call");

    // Record metric
    #[cfg(feature = "openapi")]
    {
        use crate::telemetry::METRICS;
        if let Ok(metrics) = METRICS.as_ref() {
            metrics.record_mcp_tool_call(&req.name, true);
        } else {
            tracing::error!("Metrics registry unavailable; skipping MCP metrics");
        }
    }

    let result = execute_tool(&state, &req.name, req.arguments, auth.tenant_id).await;

    match result {
        Ok(content) => Ok(Json(CallToolResponse {
            content,
            is_error: false,
        })),
        Err(e) => Ok(Json(CallToolResponse {
            content: vec![ContentBlock::Text {
                text: format!("Error: {}", e.message),
            }],
            is_error: true,
        })),
    }
}

async fn execute_tool(
    state: &McpState,
    name: &str,
    args: JsonValue,
    tenant_id: EntityId,
) -> ApiResult<Vec<ContentBlock>> {
    match name {
        "trajectory_create" => {
            let name = args["name"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("name"))?;
            let description = args["description"].as_str().map(|s| s.to_string());
            let agent_id = args["agent_id"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok());

            let req = CreateTrajectoryRequest {
                name: name.to_string(),
                description,
                parent_trajectory_id: None,
                agent_id,
                metadata: None,
            };

            let trajectory = state.db.trajectory_create(&req, tenant_id).await?;
            state.ws.broadcast(WsEvent::TrajectoryCreated {
                trajectory: trajectory.clone(),
            });

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&trajectory)
                    .unwrap_or_else(|_| "Created trajectory".to_string()),
            }])
        }

        "trajectory_get" => {
            let id_str = args["trajectory_id"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("trajectory_id"))?;
            let id = Uuid::parse_str(id_str)
                .map_err(|_| ApiError::invalid_input("Invalid UUID"))?;

            let trajectory = state
                .db
                .trajectory_get(id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::trajectory_not_found(id))?;

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&trajectory)
                    .unwrap_or_else(|_| "Trajectory data".to_string()),
            }])
        }

        "trajectory_list" => {
            let status_str = args["status"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("status"))?;

            let status = match status_str {
                "active" => caliber_core::TrajectoryStatus::Active,
                "completed" => caliber_core::TrajectoryStatus::Completed,
                "failed" => caliber_core::TrajectoryStatus::Failed,
                "suspended" => caliber_core::TrajectoryStatus::Suspended,
                _ => return Err(ApiError::invalid_input("Invalid status")),
            };

            let trajectories = state.db.trajectory_list_by_status(status, tenant_id).await?;

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&trajectories)
                    .unwrap_or_else(|_| "Trajectory list".to_string()),
            }])
        }

        "note_create" => {
            let note_type_str = args["note_type"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("note_type"))?;
            let title = args["title"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("title"))?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("content"))?;
            let source_ids: Vec<_> = args["source_trajectory_ids"]
                .as_array()
                .ok_or_else(|| ApiError::missing_field("source_trajectory_ids"))?
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| Uuid::parse_str(s).ok())
                .collect();

            let note_type = match note_type_str {
                "Convention" => caliber_core::NoteType::Convention,
                "Strategy" => caliber_core::NoteType::Strategy,
                "Gotcha" => caliber_core::NoteType::Gotcha,
                "Fact" => caliber_core::NoteType::Fact,
                "Preference" => caliber_core::NoteType::Preference,
                "Relationship" => caliber_core::NoteType::Relationship,
                "Procedure" => caliber_core::NoteType::Procedure,
                "Meta" => caliber_core::NoteType::Meta,
                _ => return Err(ApiError::invalid_input("Invalid note_type")),
            };

            let req = CreateNoteRequest {
                note_type,
                title: title.to_string(),
                content: content.to_string(),
                source_trajectory_ids: source_ids,
                source_artifact_ids: vec![],
                ttl: caliber_core::TTL::Persistent,
                metadata: None,
            };

            let note = state.db.note_create(&req, tenant_id).await?;
            state.ws.broadcast(WsEvent::NoteCreated { note: note.clone() });

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&note)
                    .unwrap_or_else(|_| "Created note".to_string()),
            }])
        }

        "note_search" => {
            let query = args["query"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("query"))?;
            let limit = args["limit"].as_i64().unwrap_or(10) as i32;

            // Search notes using the database search function
            let notes = state.db.note_search(query, limit).await?;

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&notes)
                    .unwrap_or_else(|_| "Note search results".to_string()),
            }])
        }

        "artifact_create" => {
            let trajectory_id = Uuid::parse_str(
                args["trajectory_id"]
                    .as_str()
                    .ok_or_else(|| ApiError::missing_field("trajectory_id"))?,
            )
            .map_err(|_| ApiError::invalid_input("Invalid trajectory_id"))?;

            let scope_id = Uuid::parse_str(
                args["scope_id"]
                    .as_str()
                    .ok_or_else(|| ApiError::missing_field("scope_id"))?,
            )
            .map_err(|_| ApiError::invalid_input("Invalid scope_id"))?;

            let artifact_type_str = args["artifact_type"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("artifact_type"))?;
            let name = args["name"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("name"))?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("content"))?;
            let source_turn = args["source_turn"]
                .as_i64()
                .ok_or_else(|| ApiError::missing_field("source_turn"))? as i32;

            let artifact_type = match artifact_type_str {
                "Fact" => caliber_core::ArtifactType::Fact,
                "Code" => caliber_core::ArtifactType::Code,
                "Document" => caliber_core::ArtifactType::Document,
                "Data" => caliber_core::ArtifactType::Data,
                "Config" => caliber_core::ArtifactType::Config,
                "Log" => caliber_core::ArtifactType::Log,
                "Summary" => caliber_core::ArtifactType::Summary,
                "Decision" => caliber_core::ArtifactType::Decision,
                "Plan" => caliber_core::ArtifactType::Plan,
                _ => return Err(ApiError::invalid_input("Invalid artifact_type")),
            };

            let req = CreateArtifactRequest {
                trajectory_id,
                scope_id,
                artifact_type,
                name: name.to_string(),
                content: content.to_string(),
                source_turn,
                extraction_method: caliber_core::ExtractionMethod::Explicit,
                confidence: Some(1.0),
                ttl: caliber_core::TTL::Persistent,
                metadata: None,
            };

            let artifact = state.db.artifact_create(&req, tenant_id).await?;
            state.ws.broadcast(WsEvent::ArtifactCreated {
                artifact: artifact.clone(),
            });

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&artifact)
                    .unwrap_or_else(|_| "Created artifact".to_string()),
            }])
        }

        "artifact_get" => {
            let id_str = args["artifact_id"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("artifact_id"))?;
            let id = Uuid::parse_str(id_str)
                .map_err(|_| ApiError::invalid_input("Invalid UUID"))?;

            let artifact = state
                .db
                .artifact_get(id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::artifact_not_found(id))?;

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&artifact)
                    .unwrap_or_else(|_| "Artifact data".to_string()),
            }])
        }

        "scope_create" => {
            let trajectory_id = Uuid::parse_str(
                args["trajectory_id"]
                    .as_str()
                    .ok_or_else(|| ApiError::missing_field("trajectory_id"))?,
            )
            .map_err(|_| ApiError::invalid_input("Invalid trajectory_id"))?;

            let name = args["name"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("name"))?;
            let purpose = args["purpose"].as_str().map(|s| s.to_string());
            let token_budget = args["token_budget"].as_i64().unwrap_or(4096) as i32;

            let req = CreateScopeRequest {
                trajectory_id,
                parent_scope_id: None,
                name: name.to_string(),
                purpose,
                token_budget,
                metadata: None,
            };

            let scope = state.db.scope_create(&req, tenant_id).await?;
            state.ws.broadcast(WsEvent::ScopeCreated { scope: scope.clone() });

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&scope)
                    .unwrap_or_else(|_| "Created scope".to_string()),
            }])
        }

        "agent_register" => {
            let agent_type = args["agent_type"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("agent_type"))?;
            let capabilities: Vec<String> = args["capabilities"]
                .as_array()
                .ok_or_else(|| ApiError::missing_field("capabilities"))?
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();

            let req = RegisterAgentRequest {
                agent_type: agent_type.to_string(),
                capabilities,
                memory_access: MemoryAccessRequest {
                    read: vec![],
                    write: vec![],
                },
                can_delegate_to: vec![],
                reports_to: None,
            };

            let agent = state.db.agent_register(&req, tenant_id).await?;
            state.ws.broadcast(WsEvent::AgentRegistered { agent: agent.clone() });

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&agent)
                    .unwrap_or_else(|_| "Registered agent".to_string()),
            }])
        }

        "agent_list" => {
            let agents = state.db.agent_list_active().await?;

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&agents)
                    .unwrap_or_else(|_| "Agent list".to_string()),
            }])
        }

        _ => Err(ApiError::entity_not_found("Tool", Uuid::nil())),
    }
}

