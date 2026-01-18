//! MCP (Model Context Protocol) Server Routes
//!
//! This module implements an MCP server as an integrated API endpoint.
//! It allows LLMs like Claude to interact with CALIBER's memory system
//! using the standard MCP protocol.
//!
//! Endpoints:
//! - POST /mcp/initialize - Initialize MCP session
//! - GET /mcp/tools/list - List available tools
//! - POST /mcp/tools/call - Execute a tool
//! - GET /mcp/resources/list - List available resources
//! - POST /mcp/resources/read - Read a resource

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

use caliber_core::EntityId;
use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    middleware::AuthExtractor,
    types::*,
    ws::WsState,
};

// ============================================================================
// MCP PROTOCOL TYPES
// ============================================================================

/// MCP Protocol version we support.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// MCP Initialize request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct InitializeRequest {
    /// Protocol version requested by client
    pub protocol_version: String,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
    /// Client information
    pub client_info: ClientInfo,
}

/// Client capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClientCapabilities {
    /// Roots capability (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    /// Sampling capability (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
}

/// Roots capability details.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RootsCapability {
    /// Whether list changed notifications are supported
    #[serde(default)]
    pub list_changed: bool,
}

/// Sampling capability details.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SamplingCapability {}

/// Client information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClientInfo {
    /// Client name
    pub name: String,
    /// Client version
    pub version: String,
}

/// MCP Initialize response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct InitializeResponse {
    /// Protocol version we're using
    pub protocol_version: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Server information
    pub server_info: ServerInfo,
}

/// Server capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ServerCapabilities {
    /// Tools capability
    pub tools: ToolsCapability,
    /// Resources capability
    pub resources: ResourcesCapability,
    /// Prompts capability (not implemented yet)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
}

/// Tools capability details.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ToolsCapability {
    /// Whether list changed notifications are supported
    #[serde(default)]
    pub list_changed: bool,
}

/// Resources capability details.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ResourcesCapability {
    /// Whether subscriptions are supported
    #[serde(default)]
    pub subscribe: bool,
    /// Whether list changed notifications are supported
    #[serde(default)]
    pub list_changed: bool,
}

/// Prompts capability details.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PromptsCapability {
    /// Whether list changed notifications are supported
    #[serde(default)]
    pub list_changed: bool,
}

/// Server information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
}

/// MCP Tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for input parameters
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub input_schema: JsonValue,
}

/// List tools response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListToolsResponse {
    /// Available tools
    pub tools: Vec<Tool>,
}

/// Tool call request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CallToolRequest {
    /// Tool name
    pub name: String,
    /// Tool arguments
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub arguments: JsonValue,
}

/// Tool call response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CallToolResponse {
    /// Content blocks
    pub content: Vec<ContentBlock>,
    /// Whether this is an error response
    #[serde(default)]
    pub is_error: bool,
}

/// Content block in tool response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Text content
    #[serde(rename = "text")]
    Text { text: String },
    /// Image content
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    /// Resource reference
    #[serde(rename = "resource")]
    Resource { resource: ResourceReference },
}

/// Resource reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ResourceReference {
    /// Resource URI
    pub uri: String,
    /// Resource text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP Resource definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Resource {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// List resources response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListResourcesResponse {
    /// Available resources
    pub resources: Vec<Resource>,
}

/// Read resource request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ReadResourceRequest {
    /// Resource URI
    pub uri: String,
}

/// Read resource response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ReadResourceResponse {
    /// Resource contents
    pub contents: Vec<ResourceContent>,
}

/// Resource content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ResourceContent {
    /// Resource URI
    pub uri: String,
    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Text content (for text resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Binary content as base64 (for binary resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for MCP routes.
#[derive(Clone)]
pub struct McpState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
}

impl McpState {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

// ============================================================================
// TOOL DEFINITIONS
// ============================================================================

fn get_available_tools() -> Vec<Tool> {
    vec![
        // Trajectory tools
        Tool {
            name: "trajectory_create".to_string(),
            description: "Create a new trajectory (work unit or task)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the trajectory"
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional description"
                    },
                    "agent_id": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Optional agent ID to assign"
                    }
                },
                "required": ["name"]
            }),
        },
        Tool {
            name: "trajectory_get".to_string(),
            description: "Get a trajectory by ID".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "trajectory_id": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Trajectory ID"
                    }
                },
                "required": ["trajectory_id"]
            }),
        },
        Tool {
            name: "trajectory_list".to_string(),
            description: "List trajectories by status".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["active", "completed", "failed", "suspended"],
                        "description": "Filter by status"
                    }
                },
                "required": ["status"]
            }),
        },
        // Note tools
        Tool {
            name: "note_create".to_string(),
            description: "Create a new note (cross-trajectory memory)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "note_type": {
                        "type": "string",
                        "enum": ["Convention", "Strategy", "Gotcha", "Fact", "Preference", "Relationship", "Procedure", "Meta"],
                        "description": "Type of note"
                    },
                    "title": {
                        "type": "string",
                        "description": "Note title"
                    },
                    "content": {
                        "type": "string",
                        "description": "Note content"
                    },
                    "source_trajectory_ids": {
                        "type": "array",
                        "items": {"type": "string", "format": "uuid"},
                        "description": "Source trajectory IDs"
                    }
                },
                "required": ["note_type", "title", "content", "source_trajectory_ids"]
            }),
        },
        Tool {
            name: "note_search".to_string(),
            description: "Search notes by content similarity".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results to return",
                        "default": 10
                    }
                },
                "required": ["query"]
            }),
        },
        // Artifact tools
        Tool {
            name: "artifact_create".to_string(),
            description: "Create a new artifact (trajectory-scoped memory)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "trajectory_id": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Trajectory this artifact belongs to"
                    },
                    "scope_id": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Scope this artifact was created in"
                    },
                    "artifact_type": {
                        "type": "string",
                        "enum": ["Fact", "Code", "Document", "Data", "Config", "Log", "Summary", "Decision", "Plan"],
                        "description": "Type of artifact"
                    },
                    "name": {
                        "type": "string",
                        "description": "Artifact name"
                    },
                    "content": {
                        "type": "string",
                        "description": "Artifact content"
                    },
                    "source_turn": {
                        "type": "integer",
                        "description": "Source turn number"
                    }
                },
                "required": ["trajectory_id", "scope_id", "artifact_type", "name", "content", "source_turn"]
            }),
        },
        Tool {
            name: "artifact_get".to_string(),
            description: "Get an artifact by ID".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "artifact_id": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Artifact ID"
                    }
                },
                "required": ["artifact_id"]
            }),
        },
        // Scope tools
        Tool {
            name: "scope_create".to_string(),
            description: "Create a new scope (context window segment)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "trajectory_id": {
                        "type": "string",
                        "format": "uuid",
                        "description": "Trajectory this scope belongs to"
                    },
                    "name": {
                        "type": "string",
                        "description": "Scope name"
                    },
                    "purpose": {
                        "type": "string",
                        "description": "Scope purpose/description"
                    },
                    "token_budget": {
                        "type": "integer",
                        "description": "Token budget for this scope",
                        "default": 4096
                    }
                },
                "required": ["trajectory_id", "name"]
            }),
        },
        // Agent tools
        Tool {
            name: "agent_register".to_string(),
            description: "Register a new agent".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_type": {
                        "type": "string",
                        "description": "Type of agent (e.g., 'orchestrator', 'worker', 'specialist')"
                    },
                    "capabilities": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of capabilities"
                    }
                },
                "required": ["agent_type", "capabilities"]
            }),
        },
        Tool {
            name: "agent_list".to_string(),
            description: "List active agents".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

// ============================================================================
// RESOURCE DEFINITIONS
// ============================================================================

fn get_available_resources() -> Vec<Resource> {
    vec![
        Resource {
            uri: "caliber://trajectories".to_string(),
            name: "Active Trajectories".to_string(),
            description: Some("List of all active trajectories".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        Resource {
            uri: "caliber://notes".to_string(),
            name: "All Notes".to_string(),
            description: Some("List of all notes in the memory system".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        Resource {
            uri: "caliber://artifacts".to_string(),
            name: "Recent Artifacts".to_string(),
            description: Some("List of recently created artifacts".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        Resource {
            uri: "caliber://agents".to_string(),
            name: "Active Agents".to_string(),
            description: Some("List of all registered agents".to_string()),
            mime_type: Some("application/json".to_string()),
        },
    ]
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /mcp/initialize - Initialize MCP session
#[utoipa::path(
    post,
    path = "/mcp/initialize",
    tag = "MCP",
    request_body = InitializeRequest,
    responses(
        (status = 200, description = "Session initialized", body = InitializeResponse),
        (status = 400, description = "Invalid request", body = ApiError),
    ),
)]
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
        METRICS.record_mcp_tool_call(&req.name, true);
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
                .trajectory_get(id)
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

            let trajectories = state.db.trajectory_list_by_status(status).await?;

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
                .artifact_get(id)
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

/// GET /mcp/resources/list - List available resources
#[utoipa::path(
    get,
    path = "/mcp/resources/list",
    tag = "MCP",
    responses(
        (status = 200, description = "List of available resources", body = ListResourcesResponse),
    ),
)]
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
    Json(req): Json<ReadResourceRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::debug!(uri = %req.uri, "MCP resource read");

    let content = read_resource_content(&state, &req.uri).await?;

    Ok(Json(ReadResourceResponse {
        contents: vec![content],
    }))
}

async fn read_resource_content(state: &McpState, uri: &str) -> ApiResult<ResourceContent> {
    match uri {
        "caliber://trajectories" => {
            let trajectories = state
                .db
                .trajectory_list_by_status(caliber_core::TrajectoryStatus::Active)
                .await?;

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

        "caliber://notes" | "caliber://artifacts" => {
            // These require more context to list properly
            // Return placeholder for now
            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some("[]".to_string()),
                blob: None,
            })
        }

        uri if uri.starts_with("caliber://trajectory/") => {
            let id_str = uri.trim_start_matches("caliber://trajectory/");
            let id = Uuid::parse_str(id_str)
                .map_err(|_| ApiError::invalid_input("Invalid trajectory ID in URI"))?;

            let trajectory = state
                .db
                .trajectory_get(id)
                .await?
                .ok_or_else(|| ApiError::trajectory_not_found(id))?;

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&trajectory).unwrap_or_default()),
                blob: None,
            })
        }

        _ => Err(ApiError::entity_not_found("Resource", Uuid::nil())),
    }
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the MCP routes router.
pub fn create_router(db: DbClient, ws: Arc<WsState>) -> Router {
    let state = Arc::new(McpState::new(db, ws));

    Router::new()
        .route("/initialize", post(initialize))
        .route("/tools/list", get(list_tools))
        .route("/tools/call", post(call_tool))
        .route("/resources/list", get(list_resources))
        .route("/resources/read", post(read_resource))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_response() {
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
                version: "0.1.0".to_string(),
            },
        };

        let json = serde_json::to_string(&response).expect("Failed to serialize");
        assert!(json.contains("CALIBER MCP Server"));
        assert!(json.contains(MCP_PROTOCOL_VERSION));
    }

    #[test]
    fn test_tool_definitions() {
        let tools = get_available_tools();
        assert!(!tools.is_empty());

        // Check required tools exist
        let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"trajectory_create"));
        assert!(tool_names.contains(&"note_create"));
        assert!(tool_names.contains(&"artifact_create"));
        assert!(tool_names.contains(&"agent_register"));
    }

    #[test]
    fn test_resource_definitions() {
        let resources = get_available_resources();
        assert!(!resources.is_empty());

        // Check required resources exist
        let uris: Vec<_> = resources.iter().map(|r| r.uri.as_str()).collect();
        assert!(uris.contains(&"caliber://trajectories"));
        assert!(uris.contains(&"caliber://notes"));
        assert!(uris.contains(&"caliber://agents"));
    }

    #[test]
    fn test_content_block_serialization() {
        let text_block = ContentBlock::Text {
            text: "Hello, world!".to_string(),
        };

        let json = serde_json::to_string(&text_block).expect("Failed to serialize");
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("Hello, world!"));
    }

    #[test]
    fn test_call_tool_request_parsing() {
        let json = r#"{
            "name": "trajectory_create",
            "arguments": {
                "name": "Test Trajectory",
                "description": "A test"
            }
        }"#;

        let req: CallToolRequest = serde_json::from_str(json).expect("Failed to parse");
        assert_eq!(req.name, "trajectory_create");
        assert_eq!(req.arguments["name"], "Test Trajectory");
    }
}
