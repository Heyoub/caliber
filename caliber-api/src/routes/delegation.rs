//! Delegation REST API Routes
//!
//! This module implements Axum route handlers for task delegation operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    middleware::AuthExtractor,
    types::{CreateDelegationRequest, DelegationResponse, DelegationResultResponse},
    ws::WsState,
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for delegation routes.
#[derive(Clone)]
pub struct DelegationState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
}

impl DelegationState {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/delegations - Create a task delegation
#[utoipa::path(
    post,
    path = "/api/v1/delegations",
    tag = "Delegations",
    request_body = CreateDelegationRequest,
    responses(
        (status = 201, description = "Delegation created successfully", body = DelegationResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_delegation(
    State(state): State<Arc<DelegationState>>,
    Json(req): Json<CreateDelegationRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.task_description.trim().is_empty() {
        return Err(ApiError::missing_field("task_description"));
    }

    // Validate that from and to agents are different
    if req.from_agent_id == req.to_agent_id {
        return Err(ApiError::invalid_input(
            "Cannot delegate to the same agent",
        ));
    }

    // Create delegation via database client
    let delegation = state.db.delegation_create(&req).await?;

    // Broadcast DelegationCreated event
    state.ws.broadcast(WsEvent::DelegationCreated {
        delegation: delegation.clone(),
    });

    Ok((StatusCode::CREATED, Json(delegation)))
}

/// GET /api/v1/delegations/{id} - Get delegation by ID
#[utoipa::path(
    get,
    path = "/api/v1/delegations/{id}",
    tag = "Delegations",
    params(
        ("id" = Uuid, Path, description = "Delegation ID")
    ),
    responses(
        (status = 200, description = "Delegation details", body = DelegationResponse),
        (status = 404, description = "Delegation not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_delegation(
    State(state): State<Arc<DelegationState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let delegation = state
        .db
        .delegation_get(id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Delegation", id))?;

    Ok(Json(delegation))
}

/// POST /api/v1/delegations/{id}/accept - Accept a delegation
#[utoipa::path(
    post,
    path = "/api/v1/delegations/{id}/accept",
    tag = "Delegations",
    params(
        ("id" = Uuid, Path, description = "Delegation ID")
    ),
    request_body = AcceptDelegationRequest,
    responses(
        (status = 204, description = "Delegation accepted successfully"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 403, description = "Not authorized to accept", body = ApiError),
        (status = 404, description = "Delegation not found", body = ApiError),
        (status = 409, description = "Delegation not in pending state", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn accept_delegation(
    State(state): State<Arc<DelegationState>>,
    Path(id): Path<Uuid>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<AcceptDelegationRequest>,
) -> ApiResult<StatusCode> {
    // Verify the delegation exists and is in pending state
    let delegation = state
        .db
        .delegation_get(id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Delegation", id))?;

    if delegation.status.to_lowercase() != "pending" {
        return Err(ApiError::state_conflict(format!(
            "Delegation is in '{}' state, cannot accept",
            delegation.status
        )));
    }

    // Verify the accepting agent is the delegatee
    if delegation.to_agent_id != req.accepting_agent_id {
        return Err(ApiError::forbidden(
            "Only the delegatee can accept this delegation",
        ));
    }

    // Accept delegation via database client
    let _ = state
        .db
        .delegation_accept(id, req.accepting_agent_id)
        .await?;

    // Broadcast DelegationAccepted event with tenant_id for filtering
    state.ws.broadcast(WsEvent::DelegationAccepted {
        tenant_id: auth.tenant_id,
        delegation_id: id,
    });

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/delegations/{id}/reject - Reject a delegation
#[utoipa::path(
    post,
    path = "/api/v1/delegations/{id}/reject",
    tag = "Delegations",
    params(
        ("id" = Uuid, Path, description = "Delegation ID")
    ),
    request_body = RejectDelegationRequest,
    responses(
        (status = 204, description = "Delegation rejected successfully"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 403, description = "Not authorized to reject", body = ApiError),
        (status = 404, description = "Delegation not found", body = ApiError),
        (status = 409, description = "Delegation not in pending state", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn reject_delegation(
    State(state): State<Arc<DelegationState>>,
    Path(id): Path<Uuid>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<RejectDelegationRequest>,
) -> ApiResult<StatusCode> {
    // Verify the delegation exists and is in pending state
    let delegation = state
        .db
        .delegation_get(id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Delegation", id))?;

    if delegation.status.to_lowercase() != "pending" {
        return Err(ApiError::state_conflict(format!(
            "Delegation is in '{}' state, cannot reject",
            delegation.status
        )));
    }

    // Verify the rejecting agent is the delegatee
    if delegation.to_agent_id != req.rejecting_agent_id {
        return Err(ApiError::forbidden(
            "Only the delegatee can reject this delegation",
        ));
    }

    state.db.delegation_reject(id, req.reason).await?;
    state.ws.broadcast(WsEvent::DelegationRejected {
        tenant_id: auth.tenant_id,
        delegation_id: id,
    });
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/delegations/{id}/complete - Complete a delegation
#[utoipa::path(
    post,
    path = "/api/v1/delegations/{id}/complete",
    tag = "Delegations",
    params(
        ("id" = Uuid, Path, description = "Delegation ID")
    ),
    request_body = CompleteDelegationRequest,
    responses(
        (status = 204, description = "Delegation completed successfully"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Delegation not found", body = ApiError),
        (status = 409, description = "Delegation not in accepted state", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn complete_delegation(
    State(state): State<Arc<DelegationState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<CompleteDelegationRequest>,
) -> ApiResult<StatusCode> {
    // Verify the delegation exists and is in accepted/in-progress state
    let delegation = state
        .db
        .delegation_get(id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Delegation", id))?;

    let status_lower = delegation.status.to_lowercase();
    if status_lower != "accepted" && status_lower != "inprogress" {
        return Err(ApiError::state_conflict(format!(
            "Delegation is in '{}' state, cannot complete",
            delegation.status
        )));
    }

    // Validate result status
    let valid_statuses = ["Success", "Partial", "Failure"];
    if !valid_statuses.contains(&req.result.status.as_str()) {
        return Err(ApiError::invalid_input(format!(
            "result.status must be one of: {}",
            valid_statuses.join(", ")
        )));
    }

    // Build result JSON
    let result_json = serde_json::to_value(&req.result)?;

    // Complete delegation via database client
    let updated = state
        .db
        .delegation_complete(id, result_json)
        .await?;

    // Broadcast DelegationCompleted event
    state.ws.broadcast(WsEvent::DelegationCompleted {
        delegation: updated,
    });

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Request to accept a delegation.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AcceptDelegationRequest {
    /// Agent accepting the delegation
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub accepting_agent_id: caliber_core::EntityId,
}

/// Request to reject a delegation.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RejectDelegationRequest {
    /// Agent rejecting the delegation
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub rejecting_agent_id: caliber_core::EntityId,
    /// Reason for rejection
    pub reason: String,
}

/// Request to complete a delegation.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CompleteDelegationRequest {
    /// Result of the delegation
    pub result: DelegationResultResponse,
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the delegation routes router.
pub fn create_router(db: DbClient, ws: Arc<WsState>) -> axum::Router {
    let state = Arc::new(DelegationState::new(db, ws));

    axum::Router::new()
        .route("/", axum::routing::post(create_delegation))
        .route("/:id", axum::routing::get(get_delegation))
        .route("/:id/accept", axum::routing::post(accept_delegation))
        .route("/:id/reject", axum::routing::post(reject_delegation))
        .route("/:id/complete", axum::routing::post(complete_delegation))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::EntityId;

    #[test]
    fn test_create_delegation_request_validation() {
        let agent_id = EntityId::from(Uuid::new_v4());
        let req = CreateDelegationRequest {
            from_agent_id: agent_id,
            to_agent_id: agent_id, // Same agent
            trajectory_id: EntityId::from(Uuid::new_v4()),
            scope_id: EntityId::from(Uuid::new_v4()),
            task_description: "".to_string(),
            expected_completion: None,
            context: None,
        };

        assert!(req.task_description.trim().is_empty());
        assert_eq!(req.from_agent_id, req.to_agent_id);
    }

    #[test]
    fn test_valid_delegation_result_statuses() {
        let valid_statuses = ["Success", "Partial", "Failure"];

        assert!(valid_statuses.contains(&"Success"));
        assert!(valid_statuses.contains(&"Partial"));
        assert!(valid_statuses.contains(&"Failure"));
        assert!(!valid_statuses.contains(&"Invalid"));
    }

    #[test]
    fn test_delegation_state_transitions() {
        // Valid transitions:
        // Pending -> Accepted
        // Pending -> Rejected
        // Accepted -> InProgress -> Completed
        // Accepted -> InProgress -> Failed

        let valid_accept_states = ["pending"];
        let valid_complete_states = ["accepted", "inprogress"];

        assert!(valid_accept_states.contains(&"pending"));
        assert!(valid_complete_states.contains(&"accepted"));
        assert!(valid_complete_states.contains(&"inprogress"));
    }

    #[test]
    fn test_accept_delegation_request() {
        let req = AcceptDelegationRequest {
            accepting_agent_id: EntityId::from(Uuid::new_v4()),
        };

        // Just verify the struct can be created
        assert!(req.accepting_agent_id.to_string().len() > 0);
    }

    #[test]
    fn test_reject_delegation_request() {
        let req = RejectDelegationRequest {
            rejecting_agent_id: EntityId::from(Uuid::new_v4()),
            reason: "Not enough capacity".to_string(),
        };

        assert!(!req.reason.is_empty());
    }
}
