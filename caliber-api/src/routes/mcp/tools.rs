//! MCP tool definitions and execution

use super::types::*;
use crate::*;
use axum::*;

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
