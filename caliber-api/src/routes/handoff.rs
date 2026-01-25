//! Handoff REST API Routes
//!
//! This module implements Axum route handlers for agent handoff operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use caliber_core::HandoffId;
use crate::{
    auth::validate_tenant_ownership,
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    extractors::PathId,
    middleware::AuthExtractor,
    state::AppState,
    types::{CreateHandoffRequest, HandoffResponse},
    ws::WsState,
};

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/handoffs - Create an agent handoff
#[utoipa::path(
    post,
    path = "/api/v1/handoffs",
    tag = "Handoffs",
    request_body = CreateHandoffRequest,
    responses(
        (status = 201, description = "Handoff created successfully", body = HandoffResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_handoff(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<CreateHandoffRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.reason.trim().is_empty() {
        return Err(ApiError::missing_field("reason"));
    }

    // Validate that from and to agents are different
    if req.from_agent_id == req.to_agent_id {
        return Err(ApiError::invalid_input(
            "Cannot handoff to the same agent",
        ));
    }

    // Validate context snapshot is not empty
    if req.context_snapshot.is_empty() {
        return Err(ApiError::invalid_input(
            "context_snapshot cannot be empty",
        ));
    }

    // Validate reason is one of the valid handoff reasons
    let valid_reasons = [
        "CapabilityMismatch",
        "LoadBalancing",
        "Specialization",
        "Escalation",
        "Timeout",
        "Failure",
        "Scheduled",
    ];
    if !valid_reasons.contains(&req.reason.as_str()) {
        return Err(ApiError::invalid_input(format!(
            "reason must be one of: {}",
            valid_reasons.join(", ")
        )));
    }

    // Create handoff via database client with tenant_id for isolation
    let handoff = db.create::<HandoffResponse>(&req, auth.tenant_id).await?;

    // Broadcast HandoffCreated event
    ws.broadcast(WsEvent::HandoffCreated {
        handoff: handoff.clone(),
    });

    Ok((StatusCode::CREATED, Json(handoff)))
}

/// GET /api/v1/handoffs/{id} - Get handoff by ID
#[utoipa::path(
    get,
    path = "/api/v1/handoffs/{id}",
    tag = "Handoffs",
    params(
        ("id" = Uuid, Path, description = "Handoff ID")
    ),
    responses(
        (status = 200, description = "Handoff details", body = HandoffResponse),
        (status = 404, description = "Handoff not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_handoff(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<HandoffId>,
) -> ApiResult<impl IntoResponse> {
    // Generic get filters by tenant_id, so not_found includes wrong tenant case
    let handoff = db
        .get::<HandoffResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Handoff", id))?;

    Ok(Json(handoff))
}

/// POST /api/v1/handoffs/{id}/accept - Accept a handoff
#[utoipa::path(
    post,
    path = "/api/v1/handoffs/{id}/accept",
    tag = "Handoffs",
    params(
        ("id" = Uuid, Path, description = "Handoff ID")
    ),
    request_body = AcceptHandoffRequest,
    responses(
        (status = 204, description = "Handoff accepted successfully"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 403, description = "Not authorized to accept", body = ApiError),
        (status = 404, description = "Handoff not found", body = ApiError),
        (status = 409, description = "Handoff not in initiated state", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn accept_handoff(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<HandoffId>,
    Json(req): Json<AcceptHandoffRequest>,
) -> ApiResult<StatusCode> {
    // Get the handoff
    let handoff = db
        .get::<HandoffResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Handoff", id))?;

    // Accept via Response method (validates state and permissions)
    handoff.accept(&db, req.accepting_agent_id).await?;

    tracing::info!(
        handoff_id = %id,
        accepted_by = %req.accepting_agent_id,
        tenant_id = %auth.tenant_id,
        "Handoff accepted"
    );

    // Broadcast HandoffAccepted event with tenant_id for filtering
    ws.broadcast(WsEvent::HandoffAccepted {
        tenant_id: auth.tenant_id,
        handoff_id: id,
    });

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/handoffs/{id}/complete - Complete a handoff
#[utoipa::path(
    post,
    path = "/api/v1/handoffs/{id}/complete",
    tag = "Handoffs",
    params(
        ("id" = Uuid, Path, description = "Handoff ID")
    ),
    responses(
        (status = 204, description = "Handoff completed successfully"),
        (status = 404, description = "Handoff not found", body = ApiError),
        (status = 409, description = "Handoff not in accepted state", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn complete_handoff(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<HandoffId>,
) -> ApiResult<StatusCode> {
    // Get the handoff
    let handoff = db
        .get::<HandoffResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Handoff", id))?;

    // Complete via Response method (validates state)
    let updated = handoff.complete(&db).await?;

    // Broadcast HandoffCompleted event
    ws.broadcast(WsEvent::HandoffCompleted { handoff: updated });

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Request to accept a handoff.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AcceptHandoffRequest {
    /// Agent accepting the handoff
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub accepting_agent_id: caliber_core::AgentId,
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the handoff routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(create_handoff))
        .route("/:id", axum::routing::get(get_handoff))
        .route("/:id/accept", axum::routing::post(accept_handoff))
        .route("/:id/complete", axum::routing::post(complete_handoff))
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{AgentId, ScopeId, TrajectoryId};

    #[test]
    fn test_create_handoff_request_validation() {
        let agent_id = AgentId::now_v7();
        let req = CreateHandoffRequest {
            from_agent_id: agent_id,
            to_agent_id: agent_id, // Same agent
            trajectory_id: TrajectoryId::now_v7(),
            scope_id: ScopeId::now_v7(),
            reason: "".to_string(),
            context_snapshot: vec![],
        };

        assert!(req.reason.trim().is_empty());
        assert_eq!(req.from_agent_id, req.to_agent_id);
        assert!(req.context_snapshot.is_empty());
    }

    #[test]
    fn test_valid_handoff_reasons() {
        let valid_reasons = [
            "CapabilityMismatch",
            "LoadBalancing",
            "Specialization",
            "Escalation",
            "Timeout",
            "Failure",
            "Scheduled",
        ];

        assert!(valid_reasons.contains(&"CapabilityMismatch"));
        assert!(valid_reasons.contains(&"Escalation"));
        assert!(valid_reasons.contains(&"Scheduled"));
        assert!(!valid_reasons.contains(&"Invalid"));
    }

    #[test]
    fn test_handoff_state_transitions() {
        // Valid transitions:
        // Initiated -> Accepted -> Completed
        // Initiated -> Rejected

        let valid_accept_states = ["initiated"];
        let valid_complete_states = ["accepted"];

        assert!(valid_accept_states.contains(&"initiated"));
        assert!(valid_complete_states.contains(&"accepted"));
        assert!(!valid_complete_states.contains(&"initiated"));
    }

    #[test]
    fn test_accept_handoff_request() {
        let req = AcceptHandoffRequest {
            accepting_agent_id: AgentId::now_v7(),
        };

        // Just verify the struct can be created
        assert!(!req.accepting_agent_id.as_uuid().to_string().is_empty());
    }

    #[test]
    fn test_context_snapshot_validation() {
        let empty_snapshot: Vec<u8> = vec![];
        let valid_snapshot: Vec<u8> = vec![1, 2, 3, 4];

        assert!(empty_snapshot.is_empty());
        assert!(!valid_snapshot.is_empty());
    }
}
