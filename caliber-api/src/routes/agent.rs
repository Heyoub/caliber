//! Agent REST API Routes
//!
//! This module implements Axum route handlers for agent operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    types::{AgentResponse, RegisterAgentRequest, UpdateAgentRequest},
    ws::WsState,
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for agent routes.
#[derive(Clone)]
pub struct AgentState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
}

impl AgentState {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

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
    State(state): State<Arc<AgentState>>,
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

    // Register agent via database client
    let agent = state.db.agent_register(&req).await?;

    // Broadcast AgentRegistered event
    state.ws.broadcast(WsEvent::AgentRegistered {
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
    State(state): State<Arc<AgentState>>,
    Query(params): Query<ListAgentsRequest>,
) -> ApiResult<impl IntoResponse> {
    let agents = if let Some(agent_type) = params.agent_type {
        // Filter by agent type
        state.db.agent_list_by_type(&agent_type).await?
    } else if params.active_only.unwrap_or(false) {
        // Filter by active status
        state.db.agent_list_active().await?
    } else {
        // List all agents
        state.db.agent_list_all().await?
    };

    // Apply additional filters if needed
    let mut filtered = agents;

    if let Some(status) = params.status {
        filtered.retain(|a| a.status == status);
    }

    if let Some(trajectory_id) = params.trajectory_id {
        filtered.retain(|a| a.current_trajectory_id == Some(trajectory_id));
    }

    let total = filtered.len() as i32;

    let response = ListAgentsResponse {
        agents: filtered,
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
    State(state): State<Arc<AgentState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let agent = state
        .db
        .agent_get(id)
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
    State(state): State<Arc<AgentState>>,
    Path(id): Path<Uuid>,
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

    // Update agent via database client
    let agent = state.db.agent_update(id, &req).await?;

    // Broadcast AgentStatusChanged when status updates are included
    if let Some(status) = &req.status {
        state.ws.broadcast(WsEvent::AgentStatusChanged {
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
    State(state): State<Arc<AgentState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // First verify the agent exists
    let agent = state
        .db
        .agent_get(id)
        .await?
        .ok_or_else(|| ApiError::agent_not_found(id))?;

    // Check if agent is currently active
    if agent.status.to_lowercase() == "active" {
        return Err(ApiError::invalid_input(
            "Cannot unregister an active agent. Set status to idle first.",
        ));
    }

    // Unregister agent via database client
    state.db.agent_unregister(id).await?;

    // Broadcast AgentUnregistered event
    state.ws.broadcast(WsEvent::AgentUnregistered { id });

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
    State(state): State<Arc<AgentState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // Update heartbeat via database client
    state.db.agent_heartbeat(id).await?;

    // Broadcast AgentHeartbeat event
    state.ws.broadcast(WsEvent::AgentHeartbeat {
        agent_id: id,
        timestamp: Utc::now(),
    });

    // Return the updated agent
    let agent = state
        .db
        .agent_get(id)
        .await?
        .ok_or_else(|| ApiError::agent_not_found(id))?;

    Ok(Json(agent))
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Request to list agents with filters.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListAgentsRequest {
    /// Filter by agent type
    pub agent_type: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by current trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<caliber_core::EntityId>,
    /// Only return active agents
    pub active_only: Option<bool>,
}

/// Response containing a list of agents.
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListAgentsResponse {
    /// List of agents
    pub agents: Vec<AgentResponse>,
    /// Total count
    pub total: i32,
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the agent routes router.
pub fn create_router(db: DbClient, ws: Arc<WsState>) -> axum::Router {
    let state = Arc::new(AgentState::new(db, ws));

    axum::Router::new()
        .route("/", axum::routing::post(register_agent))
        .route("/", axum::routing::get(list_agents))
        .route("/:id", axum::routing::get(get_agent))
        .route("/:id", axum::routing::patch(update_agent))
        .route("/:id", axum::routing::delete(unregister_agent))
        .route("/:id/heartbeat", axum::routing::post(agent_heartbeat))
        .with_state(state)
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
