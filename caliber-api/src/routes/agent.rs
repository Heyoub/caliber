//! Agent REST API Routes
//!
//! This module implements Axum route handlers for agent operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use std::sync::Arc;

use caliber_core::AgentId;
use crate::{
    auth::validate_tenant_ownership,
    components::AgentListFilter,
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    extractors::PathId,
    middleware::AuthExtractor,
    state::AppState,
    types::{AgentResponse, ListAgentsRequest, ListAgentsResponse, RegisterAgentRequest, UpdateAgentRequest},
    ws::WsState,
};

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/agents - Register a new agent
#[utoipa::path(
    post,
    path = "/api/v1/agents",
    tag = "Agents",
    request_body = RegisterAgentRequest,
    responses(
        (status = 201, description = "Agent registered successfully", body = AgentResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn register_agent(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<RegisterAgentRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.agent_type.trim().is_empty() {
        return Err(ApiError::missing_field("agent_type"));
    }

    if req.capabilities.is_empty() {
        return Err(ApiError::invalid_input(
            "At least one capability must be specified",
        ));
    }

    // Validate memory access permissions
    if req.memory_access.read.is_empty() && req.memory_access.write.is_empty() {
        return Err(ApiError::invalid_input(
            "At least one memory permission (read or write) must be specified",
        ));
    }

    // Register agent via database client with tenant_id for isolation
    let agent = db.agent_register(&req, auth.tenant_id).await?;

    // Broadcast AgentRegistered event
    ws.broadcast(WsEvent::AgentRegistered {
        agent: agent.clone(),
    });

    Ok((StatusCode::CREATED, Json(agent)))
}

/// GET /api/v1/agents - List agents with filters
#[utoipa::path(
    get,
    path = "/api/v1/agents",
    tag = "Agents",
    params(
        ("agent_type" = Option<String>, Query, description = "Filter by agent type"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("trajectory_id" = Option<String>, Query, description = "Filter by current trajectory"),
        ("active_only" = Option<bool>, Query, description = "Only return active agents"),
    ),
    responses(
        (status = 200, description = "List of agents", body = ListAgentsResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_agents(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Query(params): Query<ListAgentsRequest>,
) -> ApiResult<impl IntoResponse> {
    // Build filter from query params - all filtering handled by generic list
    let filter = AgentListFilter {
        agent_type: params.agent_type,
        status: params.status,
        trajectory_id: params.trajectory_id,
        active_only: params.active_only,
        limit: None,
        offset: None,
    };

    let agents = db.list::<AgentResponse>(&filter, auth.tenant_id).await?;
    let total = agents.len() as i32;

    let response = ListAgentsResponse {
        agents,
        total,
    };

    Ok(Json(response))
}

/// GET /api/v1/agents/{id} - Get agent by ID
#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}",
    tag = "Agents",
    params(
        ("id" = Uuid, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Agent details", body = AgentResponse),
        (status = 404, description = "Agent not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_agent(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<AgentId>,
) -> ApiResult<impl IntoResponse> {
    // Generic get filters by tenant_id, so not_found includes wrong tenant case
    let agent = db
        .get::<AgentResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::agent_not_found(id))?;

    Ok(Json(agent))
}

/// PATCH /api/v1/agents/{id} - Update agent
#[utoipa::path(
    patch,
    path = "/api/v1/agents/{id}",
    tag = "Agents",
    params(
        ("id" = Uuid, Path, description = "Agent ID")
    ),
    request_body = UpdateAgentRequest,
    responses(
        (status = 200, description = "Agent updated successfully", body = AgentResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Agent not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn update_agent(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<AgentId>,
    Json(req): Json<UpdateAgentRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that at least one field is being updated
    if req.status.is_none()
        && req.current_trajectory_id.is_none()
        && req.current_scope_id.is_none()
        && req.capabilities.is_none()
        && req.memory_access.is_none()
    {
        return Err(ApiError::invalid_input(
            "At least one field must be provided for update",
        ));
    }

    // Validate status if provided
    if let Some(ref status) = req.status {
        let valid_statuses = ["idle", "active", "blocked", "failed"];
        if !valid_statuses.contains(&status.to_lowercase().as_str()) {
            return Err(ApiError::invalid_input(
                "status must be one of: idle, active, blocked, failed",
            ));
        }
    }

    // Validate capabilities if provided
    if let Some(ref capabilities) = req.capabilities {
        if capabilities.is_empty() {
            return Err(ApiError::invalid_input(
                "capabilities cannot be empty if provided",
            ));
        }
    }

    // First verify the agent exists and belongs to this tenant
    let existing = db
        .get::<AgentResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::agent_not_found(id))?;

    // Update agent via database client
    let agent = db.update::<AgentResponse>(id, &req, auth.tenant_id).await?;

    // Broadcast AgentStatusChanged when status updates are included
    if let Some(status) = &req.status {
        ws.broadcast(WsEvent::AgentStatusChanged {
            tenant_id: auth.tenant_id,
            agent_id: agent.agent_id,
            status: status.clone(),
        });
    }

    Ok(Json(agent))
}

/// DELETE /api/v1/agents/{id} - Unregister agent
#[utoipa::path(
    delete,
    path = "/api/v1/agents/{id}",
    tag = "Agents",
    params(
        ("id" = Uuid, Path, description = "Agent ID")
    ),
    responses(
        (status = 204, description = "Agent unregistered successfully"),
        (status = 400, description = "Cannot unregister active agent", body = ApiError),
        (status = 404, description = "Agent not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn unregister_agent(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<AgentId>,
) -> ApiResult<StatusCode> {
    // Get the agent
    let agent = db
        .get::<AgentResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::agent_not_found(id))?;

    // Check if agent is currently active
    if agent.status == caliber_core::AgentStatus::Active {
        return Err(ApiError::invalid_input(
            "Cannot unregister an active agent. Set status to idle first.",
        ));
    }

    // Unregister via Response method (sets status to Offline)
    agent.unregister(&db).await?;

    // Broadcast AgentUnregistered event with tenant_id for filtering
    ws.broadcast(WsEvent::AgentUnregistered {
        tenant_id: auth.tenant_id,
        id,
    });

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/agents/{id}/heartbeat - Send agent heartbeat
#[utoipa::path(
    post,
    path = "/api/v1/agents/{id}/heartbeat",
    tag = "Agents",
    params(
        ("id" = Uuid, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Heartbeat recorded", body = AgentResponse),
        (status = 404, description = "Agent not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn agent_heartbeat(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<AgentId>,
) -> ApiResult<impl IntoResponse> {
    // Get the agent
    let agent = db
        .get::<AgentResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::agent_not_found(id))?;

    // Update heartbeat via Response method
    let updated_agent = agent.heartbeat(&db).await?;

    // Broadcast AgentHeartbeat event with tenant_id for filtering
    ws.broadcast(WsEvent::AgentHeartbeat {
        tenant_id: auth.tenant_id,
        agent_id: id,
        timestamp: Utc::now(),
    });

    Ok(Json(updated_agent))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the agent routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(register_agent))
        .route("/", axum::routing::get(list_agents))
        .route("/:id", axum::routing::get(get_agent))
        .route("/:id", axum::routing::patch(update_agent))
        .route("/:id", axum::routing::delete(unregister_agent))
        .route("/:id/heartbeat", axum::routing::post(agent_heartbeat))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryAccessRequest, MemoryPermissionRequest};

    #[test]
    fn test_register_agent_request_validation() {
        let req = RegisterAgentRequest {
            agent_type: "".to_string(),
            capabilities: vec![],
            memory_access: MemoryAccessRequest {
                read: vec![],
                write: vec![],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        assert!(req.agent_type.trim().is_empty());
        assert!(req.capabilities.is_empty());
        assert!(req.memory_access.read.is_empty());
        assert!(req.memory_access.write.is_empty());
    }

    #[test]
    fn test_update_agent_request_validation() {
        let req = UpdateAgentRequest {
            status: None,
            current_trajectory_id: None,
            current_scope_id: None,
            capabilities: None,
            memory_access: None,
        };

        let has_updates = req.status.is_some()
            || req.current_trajectory_id.is_some()
            || req.current_scope_id.is_some()
            || req.capabilities.is_some()
            || req.memory_access.is_some();

        assert!(!has_updates);
    }

    #[test]
    fn test_valid_agent_statuses() {
        let valid_statuses = ["idle", "active", "blocked", "failed"];

        assert!(valid_statuses.contains(&"idle"));
        assert!(valid_statuses.contains(&"active"));
        assert!(valid_statuses.contains(&"blocked"));
        assert!(valid_statuses.contains(&"failed"));
        assert!(!valid_statuses.contains(&"invalid"));
    }

    #[test]
    fn test_list_agents_request_filters() {
        let req = ListAgentsRequest {
            agent_type: Some("coder".to_string()),
            status: Some("active".to_string()),
            trajectory_id: None,
            active_only: Some(true),
        };

        assert_eq!(req.agent_type, Some("coder".to_string()));
        assert_eq!(req.status, Some("active".to_string()));
        assert_eq!(req.active_only, Some(true));
    }

    #[test]
    fn test_memory_permission_validation() {
        let perm = MemoryPermissionRequest {
            memory_type: "artifact".to_string(),
            scope: "own".to_string(),
            filter: None,
        };

        assert_eq!(perm.memory_type, "artifact");
        assert_eq!(perm.scope, "own");
        assert!(perm.filter.is_none());
    }
}
