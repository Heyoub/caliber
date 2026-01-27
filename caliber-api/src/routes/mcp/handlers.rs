//! MCP handler functions

use super::{types::*, tools::*};
use crate::components::TrajectoryListFilter;
use crate::middleware::AuthExtractor;
use crate::types::{AgentResponse, ArtifactResponse, PackSource, TrajectoryResponse};
use crate::*;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use caliber_core::{AgentId, EntityIdType, ScopeId, TenantId, TrajectoryId};
use caliber_dsl::compiler::{CompiledConfig as DslCompiledConfig, CompiledToolKind};
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
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
    AuthExtractor(auth): AuthExtractor,
    headers: HeaderMap,
) -> impl IntoResponse {
    tracing::debug!(db_pool_size = state.db.pool_size(), "MCP list_tools");

    let agent_name = resolve_agent_name_from_headers(&state.db, auth.tenant_id, &headers).await;

    // Start with core tools (always available)
    let mut tools = get_available_tools();

    // Add pack tools if a compiled config is active
    match state.db.dsl_compiled_get_active(auth.tenant_id, "default").await {
        Ok(Some(compiled)) if !compiled.tools.is_empty() => {
            tools.extend(tools_from_compiled(&compiled, agent_name.as_deref()));
        }
        Ok(_) => {
            tracing::debug!("No deployed pack tools found; returning core tools only");
        }
        Err(err) => {
            tracing::warn!(error = %err, "Failed to load active compiled config; returning core tools only");
        }
    };

    Json(ListToolsResponse { tools })
}

fn tools_from_compiled(compiled: &DslCompiledConfig, agent_name: Option<&str>) -> Vec<Tool> {
    let allowed = agent_name
        .map(|name| allowed_tools_for_agent(compiled, name))
        .unwrap_or_else(|| all_tool_ids(compiled));

    compiled
        .tools
        .iter()
        .filter(|tool| allowed.contains(&tool.id))
        .map(|tool| {
            let description = match tool.kind {
                CompiledToolKind::Exec => format!("Pack exec tool: {}", tool.id),
                CompiledToolKind::Prompt => format!("Pack prompt tool: {}", tool.id),
            };

            Tool {
                name: tool.id.clone(),
                description,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": {
                            "type": "string",
                            "description": "Optional input for the tool"
                        },
                        "agent_name": {
                            "type": "string",
                            "description": "Optional agent name for toolset scoping"
                        }
                    }
                }),
            }
        })
        .collect()
}

fn all_tool_ids(compiled: &DslCompiledConfig) -> HashSet<String> {
    compiled.tools.iter().map(|t| t.id.clone()).collect()
}

fn allowed_tools_for_agent(compiled: &DslCompiledConfig, agent_name: &str) -> HashSet<String> {
    let Some(agent) = compiled.pack_agents.iter().find(|a| a.name == agent_name) else {
        return HashSet::new();
    };

    let toolset_names: HashSet<&str> = agent.toolsets.iter().map(|s| s.as_str()).collect();
    compiled
        .toolsets
        .iter()
        .filter(|set| toolset_names.contains(set.name.as_str()))
        .flat_map(|set| set.tools.iter().cloned())
        .collect()
}

async fn resolve_agent_name_from_headers(
    db: &crate::db::DbClient,
    tenant_id: TenantId,
    headers: &HeaderMap,
) -> Option<String> {
    if let Some(name) = headers
        .get("x-agent-name")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        return Some(name);
    }

    let agent_id = headers
        .get("x-agent-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .map(AgentId::new)?;

    db.get::<AgentResponse>(agent_id, tenant_id)
        .await
        .ok()
        .flatten()
        .map(|agent| agent.agent_type)
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
    tenant_id: TenantId,
) -> ApiResult<Vec<ContentBlock>> {
    if let Some(result) = execute_pack_tool(state, name, &args, tenant_id).await? {
        return Ok(result);
    }

    // Strict pack-only mode: do not fall back to hardcoded MCP tools.
    let strict_pack_only = std::env::var("CALIBER_MCP_STRICT_PACK")
        .map(|v| v != "0")
        .unwrap_or(true);
    if strict_pack_only {
        return Err(ApiError::entity_not_found("Tool", Uuid::nil()));
    }

    match name {
        "trajectory_create" => {
            let name = args["name"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("name"))?;
            let description = args["description"].as_str().map(|s| s.to_string());
            let agent_id = args["agent_id"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok())
                .map(AgentId::new);

            let req = CreateTrajectoryRequest {
                name: name.to_string(),
                description,
                parent_trajectory_id: None,
                agent_id,
                metadata: None,
            };

            let trajectory = state.db.create::<crate::types::TrajectoryResponse>(&req, tenant_id).await?;
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
                .map(TrajectoryId::new)
                .map_err(|_| ApiError::invalid_input("Invalid UUID"))?;

            let trajectory = state
                .db
                .get::<TrajectoryResponse>(id, tenant_id)
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

            let filter = TrajectoryListFilter {
                status: Some(status),
                ..Default::default()
            };
            let trajectories = state.db.list::<TrajectoryResponse>(&filter, tenant_id).await?;

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
                .filter_map(|s| Uuid::parse_str(s).ok().map(TrajectoryId::new))
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

            let note = state.db.create::<crate::types::NoteResponse>(&req, tenant_id).await?;
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
            .map(TrajectoryId::new)
            .map_err(|_| ApiError::invalid_input("Invalid trajectory_id"))?;

            let scope_id = Uuid::parse_str(
                args["scope_id"]
                    .as_str()
                    .ok_or_else(|| ApiError::missing_field("scope_id"))?,
            )
            .map(ScopeId::new)
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

            let artifact = state.db.create::<crate::types::ArtifactResponse>(&req, tenant_id).await?;
            state.ws.broadcast(WsEvent::ArtifactCreated {
                artifact: artifact.clone(),
            });

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&artifact)
                    .unwrap_or_else(|_| "Created artifact".to_string()),
            }])
        }

        "artifact_get" => {
            use caliber_core::ArtifactId;
            let id_str = args["artifact_id"]
                .as_str()
                .ok_or_else(|| ApiError::missing_field("artifact_id"))?;
            let id = Uuid::parse_str(id_str)
                .map(ArtifactId::new)
                .map_err(|_| ApiError::invalid_input("Invalid UUID"))?;

            let artifact = state
                .db
                .get::<ArtifactResponse>(id, tenant_id)
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
            .map(TrajectoryId::new)
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

            let scope = state.db.create::<crate::types::ScopeResponse>(&req, tenant_id).await?;
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
            let agents: Vec<_> = agents
                .into_iter()
                .filter(|agent| agent.tenant_id == tenant_id)
                .collect();

            Ok(vec![ContentBlock::Text {
                text: serde_json::to_string_pretty(&agents)
                    .unwrap_or_else(|_| "Agent list".to_string()),
            }])
        }

        _ => Err(ApiError::entity_not_found("Tool", Uuid::nil())),
    }
}

async fn execute_pack_tool(
    state: &McpState,
    name: &str,
    args: &JsonValue,
    tenant_id: TenantId,
) -> ApiResult<Option<Vec<ContentBlock>>> {
    let Some(compiled) = state
        .db
        .dsl_compiled_get_active(tenant_id, "default")
        .await?
    else {
        return Ok(None);
    };

    let agent_name = resolve_agent_name_from_args(&state.db, tenant_id, args).await;
    if let Some(agent_name) = agent_name.as_deref() {
        let allowed = allowed_tools_for_agent(&compiled, agent_name);
        if allowed.is_empty() {
            return Err(ApiError::forbidden(format!(
                "Agent '{}' has no toolsets configured in the active pack",
                agent_name
            )));
        }
        if !allowed.contains(name) {
            return Err(ApiError::forbidden(format!(
                "Tool '{}' is not allowed for agent '{}'",
                name, agent_name
            )));
        }
    }

    // Check scope token budget if scope_id is provided
    if let Some(scope_id_str) = args.get("scope_id").and_then(|v| v.as_str()) {
        if let Ok(uuid) = uuid::Uuid::parse_str(scope_id_str) {
            let scope_id = ScopeId::new(uuid);
            if let Ok(Some(scope)) = state.db.get::<crate::types::ScopeResponse>(scope_id, tenant_id).await {
                if scope.tokens_used >= scope.token_budget {
                    return Err(ApiError::forbidden(format!(
                        "Scope '{}' has exceeded its token budget ({}/{} tokens used)",
                        scope_id, scope.tokens_used, scope.token_budget
                    )));
                }
            }
        }
    }

    let Some(tool) = compiled.tools.iter().find(|t| t.id == name) else {
        return Ok(None);
    };

    // Validate input against compiled JSON Schema if present
    if let Some(schema) = &tool.compiled_schema {
        if let Err(e) = validate_tool_input(args, schema) {
            return Err(ApiError::bad_request(format!(
                "Tool '{}' input validation failed: {}",
                name, e
            )));
        }
    }

    // Resolve agent_id for audit trail (from args if provided)
    let agent_id = args
        .get("agent_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .map(caliber_core::AgentId::new);

    match tool.kind {
        CompiledToolKind::Exec => {
            // Check subprocess permission - Exec tools spawn subprocesses
            if !tool.allow_subprocess.unwrap_or(false) {
                return Err(ApiError::forbidden(format!(
                    "Tool '{}' is not allowed to spawn subprocesses (allow_subprocess=false)",
                    name
                )));
            }

            let cmd = tool
                .cmd
                .as_ref()
                .ok_or_else(|| ApiError::internal_error("Exec tool missing cmd"))?;
            let input = args.get("input").and_then(|v| v.as_str()).map(str::to_string);

            // Execute directly - cmd is validated during pack compilation to be an executable path
            let mut command = Command::new(cmd);
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
            if input.is_some() {
                command.stdin(Stdio::piped());
            }

            // Get timeout from config (default 30s, max 5min)
            let timeout_ms = tool.timeout_ms.unwrap_or(30_000).clamp(100, 300_000) as u64;
            let timeout = Duration::from_millis(timeout_ms);

            // Start timing for audit event
            let start = Instant::now();

            let execution = async {
                let mut child = command
                    .spawn()
                    .map_err(|e| ApiError::internal_error(format!("Failed to spawn tool: {}", e)))?;

                if let Some(input_text) = input {
                    if let Some(stdin) = child.stdin.as_mut() {
                        stdin
                            .write_all(input_text.as_bytes())
                            .await
                            .map_err(|e| ApiError::internal_error(format!("Failed to write tool stdin: {}", e)))?;
                    }
                }

                child
                    .wait_with_output()
                    .await
                    .map_err(|e| ApiError::internal_error(format!("Failed to run tool: {}", e)))
            };

            let result = tokio::time::timeout(timeout, execution).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            let output = match result {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    // Emit failure event
                    state.ws.broadcast(crate::events::WsEvent::ToolExecuted {
                        tenant_id,
                        agent_id,
                        tool_name: name.to_string(),
                        success: false,
                        duration_ms,
                        error: Some(e.to_string()),
                    });
                    return Err(e);
                }
                Err(_) => {
                    // Emit timeout event
                    state.ws.broadcast(crate::events::WsEvent::ToolExecuted {
                        tenant_id,
                        agent_id,
                        tool_name: name.to_string(),
                        success: false,
                        duration_ms,
                        error: Some(format!("Timed out after {}ms", timeout_ms)),
                    });
                    return Err(ApiError::internal_error(format!(
                        "Tool '{}' execution timed out after {}ms",
                        name, timeout_ms
                    )));
                }
            };

            let status = output.status.code().unwrap_or_default();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let success = output.status.success();

            // Emit success/failure event
            state.ws.broadcast(crate::events::WsEvent::ToolExecuted {
                tenant_id,
                agent_id,
                tool_name: name.to_string(),
                success,
                duration_ms,
                error: if success { None } else { Some(format!("Exit code: {}", status)) },
            });

            let mut text = format!("Tool '{}' exited with status {}", name, status);
            if !stdout.is_empty() {
                text.push_str("\n\nstdout:\n");
                text.push_str(&stdout);
            }
            if !stderr.is_empty() {
                text.push_str("\n\nstderr:\n");
                text.push_str(&stderr);
            }

            tracing::info!(
                tool_name = %name,
                exit_status = status,
                duration_ms = duration_ms,
                "Tool execution completed"
            );

            Ok(Some(vec![ContentBlock::Text { text }]))
        }
        CompiledToolKind::Prompt => {
            let prompt_md = tool
                .prompt_md
                .as_ref()
                .ok_or_else(|| ApiError::internal_error("Prompt tool missing prompt_md"))?;

            let pack_source = state.db.dsl_pack_get_active(tenant_id, "default").await?;
            let prompt_text = pack_source
                .and_then(|value| serde_json::from_value::<PackSource>(value).ok())
                .and_then(|pack| find_prompt_content(&pack, prompt_md));

            let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");

            let text = match prompt_text {
                Some(prompt) if input.is_empty() => prompt,
                Some(prompt) => format!("{}\n\n---\n\ninput:\n{}", prompt, input),
                None => format!(
                    "Prompt tool '{}' is configured but prompt content was not found in the active pack source.",
                    name
                ),
            };

            Ok(Some(vec![ContentBlock::Text { text }]))
        }
    }
}

fn find_prompt_content(pack: &PackSource, prompt_md: &str) -> Option<String> {
    pack.markdowns
        .iter()
        .find(|m| m.path == prompt_md || m.path.ends_with(prompt_md))
        .map(|m| m.content.clone())
}

/// Validate tool input against a JSON Schema.
fn validate_tool_input(input: &JsonValue, schema: &JsonValue) -> Result<(), String> {
    let compiled = jsonschema::draft202012::new(schema)
        .map_err(|e| format!("Invalid schema: {}", e))?;

    compiled
        .validate(input)
        .map_err(|errors| {
            errors
                .map(|e| format!("{}: {}", e.instance_path, e))
                .collect::<Vec<_>>()
                .join("; ")
        })
}

async fn resolve_agent_name_from_args(
    db: &crate::db::DbClient,
    tenant_id: TenantId,
    args: &JsonValue,
) -> Option<String> {
    if let Some(name) = args.get("agent_name").and_then(|v| v.as_str()) {
        return Some(name.to_string());
    }

    let agent_id = args
        .get("agent_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .map(AgentId::new)?;

    db.get::<AgentResponse>(agent_id, tenant_id)
        .await
        .ok()
        .flatten()
        .map(|agent| agent.agent_type)
}
