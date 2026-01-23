//! MCP prompt handlers

use super::{types::*, McpState};
use crate::types::TrajectoryResponse;
use crate::*;
use axum::{extract::State, response::IntoResponse, Json};
use caliber_core::EntityId;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// POST /mcp/prompts/list - List available prompts
pub async fn list_prompts(
    State(_state): State<Arc<McpState>>,
) -> impl IntoResponse {
    Json(ListPromptsResponse {
        prompts: get_available_prompts(),
        next_cursor: None,
    })
}

/// POST /mcp/prompts/get - Get a prompt template
pub async fn get_prompt(
    State(state): State<Arc<McpState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<GetPromptRequest>,
) -> ApiResult<impl IntoResponse> {
    let response = execute_prompt(&state, &req.name, req.arguments, auth.tenant_id).await?;
    Ok(Json(response))
}

/// Get list of available prompts.
fn get_available_prompts() -> Vec<Prompt> {
    vec![
        Prompt {
            name: "summarize_trajectory".to_string(),
            description: Some("Summarize the progress and key points of a trajectory".to_string()),
            arguments: Some(vec![PromptArgument {
                name: "trajectory_id".to_string(),
                description: Some("UUID of the trajectory to summarize".to_string()),
                required: true,
            }]),
        },
        Prompt {
            name: "find_relevant_notes".to_string(),
            description: Some("Search for notes relevant to a query".to_string()),
            arguments: Some(vec![PromptArgument {
                name: "query".to_string(),
                description: Some("Search query string".to_string()),
                required: true,
            }]),
        },
        Prompt {
            name: "analyze_contradictions".to_string(),
            description: Some("Find contradictions between notes in a trajectory".to_string()),
            arguments: Some(vec![PromptArgument {
                name: "trajectory_id".to_string(),
                description: Some("UUID of the trajectory to analyze".to_string()),
                required: true,
            }]),
        },
        Prompt {
            name: "create_artifact_summary".to_string(),
            description: Some("Generate a summary of artifacts in a trajectory".to_string()),
            arguments: Some(vec![PromptArgument {
                name: "trajectory_id".to_string(),
                description: Some("UUID of the trajectory".to_string()),
                required: true,
            }]),
        },
    ]
}

/// Execute a prompt and return the result.
async fn execute_prompt(
    state: &McpState,
    name: &str,
    args: HashMap<String, String>,
    tenant_id: EntityId,
) -> ApiResult<GetPromptResponse> {
    match name {
        "summarize_trajectory" => {
            let trajectory_id = parse_uuid(&args, "trajectory_id")?;
            let trajectory = state
                .db
                .get::<TrajectoryResponse>(trajectory_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::trajectory_not_found(trajectory_id))?;

            let prompt_text = format!(
                "Summarize this trajectory:\n\
                Name: {}\n\
                Description: {}\n\
                Status: {:?}\n\
                Created: {}\n\n\
                Please provide a concise summary of the trajectory's purpose and current state.",
                trajectory.name,
                trajectory.description.as_deref().unwrap_or("No description"),
                trajectory.status,
                trajectory.created_at
            );

            Ok(GetPromptResponse {
                description: Some(format!("Summarize trajectory '{}'", trajectory.name)),
                messages: vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptContent::Text { text: prompt_text },
                }],
            })
        }

        "find_relevant_notes" => {
            let query = args
                .get("query")
                .ok_or_else(|| ApiError::missing_field("query"))?;

            let prompt_text = format!(
                "Find and list notes relevant to the following query:\n\n{}\n\n\
                Use the caliber://notes resource to access all available notes.",
                query
            );

            Ok(GetPromptResponse {
                description: Some(format!("Search notes for: {}", query)),
                messages: vec![
                    PromptMessage {
                        role: "user".to_string(),
                        content: PromptContent::Text { text: prompt_text },
                    },
                    PromptMessage {
                        role: "assistant".to_string(),
                        content: PromptContent::Resource {
                            uri: "caliber://notes".to_string(),
                            mime_type: Some("application/json".to_string()),
                        },
                    },
                ],
            })
        }

        "analyze_contradictions" => {
            let trajectory_id = parse_uuid(&args, "trajectory_id")?;
            let trajectory = state
                .db
                .get::<TrajectoryResponse>(trajectory_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::trajectory_not_found(trajectory_id))?;

            let prompt_text = format!(
                "Analyze the notes in trajectory '{}' for contradictions.\n\n\
                Look for:\n\
                - Conflicting information\n\
                - Inconsistent statements\n\
                - Changed decisions or assumptions\n\n\
                Use the caliber://trajectory/{} resource to access trajectory data.",
                trajectory.name, trajectory_id
            );

            Ok(GetPromptResponse {
                description: Some(format!(
                    "Analyze contradictions in '{}'",
                    trajectory.name
                )),
                messages: vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptContent::Text { text: prompt_text },
                }],
            })
        }

        "create_artifact_summary" => {
            let trajectory_id = parse_uuid(&args, "trajectory_id")?;
            let trajectory = state
                .db
                .get::<TrajectoryResponse>(trajectory_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::trajectory_not_found(trajectory_id))?;

            let prompt_text = format!(
                "Create a comprehensive summary of all artifacts in trajectory '{}'.\n\n\
                For each artifact type (Code, Document, Data, Model, Config), list:\n\
                - Count of artifacts\n\
                - Key artifacts and their purpose\n\
                - Overall completeness\n\n\
                Use caliber://trajectory/{} to access the data.",
                trajectory.name, trajectory_id
            );

            Ok(GetPromptResponse {
                description: Some(format!("Summarize artifacts in '{}'", trajectory.name)),
                messages: vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptContent::Text { text: prompt_text },
                }],
            })
        }

        _ => Err(ApiError::entity_not_found("Prompt", Uuid::nil())),
    }
}

/// Helper to parse UUID from args.
fn parse_uuid(args: &HashMap<String, String>, key: &str) -> ApiResult<EntityId> {
    let value = args
        .get(key)
        .ok_or_else(|| ApiError::missing_field(key))?;
    Uuid::parse_str(value).map_err(|_| ApiError::invalid_input(format!("Invalid UUID for {}", key)))
}
