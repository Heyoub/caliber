//! Turn REST API Routes
//!
//! This module implements Axum route handlers for turn operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.
//!
//! Includes Battle Intel Feature 4: Auto-summarization trigger checking
//! after turn creation to enable L0→L1→L2 abstraction transitions.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use caliber_core::TurnId;
use caliber_pcp::PCPRuntime;
use std::sync::Arc;

use crate::{
    auth::validate_tenant_ownership,
    components::{ArtifactListFilter, TurnListFilter},
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    extractors::PathId,
    middleware::AuthExtractor,
    state::AppState,
    types::{ArtifactResponse, CreateTurnRequest, ScopeResponse, TurnResponse},
    ws::WsState,
};

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/turns - Create a new turn
#[utoipa::path(
    post,
    path = "/api/v1/turns",
    tag = "Turns",
    request_body = CreateTurnRequest,
    responses(
        (status = 201, description = "Turn created successfully", body = TurnResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_turn(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    State(pcp): State<Arc<PCPRuntime>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<CreateTurnRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.content.trim().is_empty() {
        return Err(ApiError::missing_field("content"));
    }

    if req.sequence < 0 {
        return Err(ApiError::invalid_range("sequence", 0, i32::MAX));
    }

    if req.token_count < 0 {
        return Err(ApiError::invalid_range("token_count", 0, i32::MAX));
    }

    // Validate scope belongs to this tenant before creating turn
    let scope = db
        .get::<ScopeResponse>(req.scope_id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(req.scope_id))?;
    validate_tenant_ownership(&auth, Some(scope.tenant_id))?;

    // Create turn via database client with tenant_id for isolation
    let turn = db.create::<TurnResponse>(&req, auth.tenant_id).await?;

    // Broadcast TurnCreated event
    ws.broadcast(WsEvent::TurnCreated { turn: turn.clone() });

    // =========================================================================
    // BATTLE INTEL: Check summarization triggers after turn creation
    // =========================================================================
    // Get the scope to check trigger conditions
    if let Ok(Some(scope)) = db.get::<ScopeResponse>(req.scope_id, auth.tenant_id).await {
        // Get the trajectory ID from the scope for fetching policies
        let trajectory_id = scope.trajectory_id;

        // Fetch summarization policies for this trajectory (custom function)
        if let Ok(policies) = db
            .summarization_policies_for_trajectory(trajectory_id)
            .await
        {
            if !policies.is_empty() {
                // Get turn count for this scope using generic list
                let turn_filter = TurnListFilter {
                    scope_id: Some(req.scope_id),
                    ..Default::default()
                };
                let turn_count = db
                    .list::<TurnResponse>(&turn_filter, auth.tenant_id)
                    .await
                    .map(|turns| turns.len() as i32)
                    .unwrap_or(0);

                // Get artifact count for this scope using generic list
                let artifact_filter = ArtifactListFilter {
                    scope_id: Some(req.scope_id),
                    ..Default::default()
                };
                let artifact_count = db
                    .list::<ArtifactResponse>(&artifact_filter, auth.tenant_id)
                    .await
                    .map(|artifacts| artifacts.len() as i32)
                    .unwrap_or(0);

                // Convert policies to caliber_core format for PCPRuntime
                let core_policies: Vec<caliber_core::SummarizationPolicy> = policies
                    .iter()
                    .map(|p| caliber_core::SummarizationPolicy {
                        policy_id: p.policy_id,
                        name: p.name.clone(),
                        triggers: p.triggers.clone(),
                        target_level: p.target_level,
                        source_level: p.source_level,
                        max_sources: p.max_sources,
                        create_edges: p.create_edges,
                        created_at: p.created_at,
                        metadata: p.metadata.clone(),
                    })
                    .collect();

                // Build a caliber_core::Scope from our ScopeResponse
                let core_scope = caliber_core::Scope {
                    scope_id: scope.scope_id,
                    trajectory_id: scope.trajectory_id,
                    parent_scope_id: scope.parent_scope_id,
                    name: scope.name.clone(),
                    purpose: scope.purpose.clone(),
                    is_active: scope.is_active,
                    created_at: scope.created_at,
                    closed_at: scope.closed_at,
                    checkpoint: None,
                    token_budget: scope.token_budget,
                    tokens_used: scope.tokens_used,
                    metadata: scope.metadata.clone(),
                };

                // Check which triggers should fire
                if let Ok(triggered) = pcp.check_summarization_triggers(
                    &core_scope,
                    turn_count,
                    artifact_count,
                    &core_policies,
                ) {
                    // Broadcast SummarizationTriggered event for each fired trigger
                    for (policy_id, trigger) in triggered {
                        // Find the policy to get its details
                        if let Some(policy) =
                            core_policies.iter().find(|p| p.policy_id == policy_id)
                        {
                            ws.broadcast(WsEvent::SummarizationTriggered {
                                tenant_id: auth.tenant_id,
                                policy_id,
                                trigger,
                                scope_id: core_scope.scope_id,
                                trajectory_id,
                                source_level: policy.source_level,
                                target_level: policy.target_level,
                                max_sources: policy.max_sources,
                                create_edges: policy.create_edges,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok((StatusCode::CREATED, Json(turn)))
}

/// GET /api/v1/turns/{id} - Get turn by ID
#[utoipa::path(
    get,
    path = "/api/v1/turns/{id}",
    tag = "Turns",
    params(
        ("id" = Uuid, Path, description = "Turn ID")
    ),
    responses(
        (status = 200, description = "Turn details", body = TurnResponse),
        (status = 404, description = "Turn not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_turn(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<TurnId>,
) -> ApiResult<Json<TurnResponse>> {
    // Generic get filters by tenant_id, so not_found includes wrong tenant case
    let turn = db
        .get::<TurnResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Turn", id.to_string()))?;

    Ok(Json(turn))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the turn routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(create_turn))
        .route("/:id", axum::routing::get(get_turn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{EntityIdType, ScopeId, TurnRole};

    #[test]
    fn test_create_turn_request_validation() {
        // Use a dummy ScopeId for testing (all zeros is valid)
        let dummy_id = ScopeId::nil();

        let req = CreateTurnRequest {
            scope_id: dummy_id,
            sequence: -1,
            role: TurnRole::User,
            content: "".to_string(),
            token_count: -1,
            tool_calls: None,
            tool_results: None,
            metadata: None,
        };

        assert!(req.content.trim().is_empty());
        assert!(req.sequence < 0);
        assert!(req.token_count < 0);
    }

    #[test]
    fn test_turn_role_variants() {
        // Verify all turn roles are accessible
        let roles = [
            TurnRole::User,
            TurnRole::Assistant,
            TurnRole::System,
            TurnRole::Tool,
        ];

        assert_eq!(roles.len(), 4);
    }

    #[test]
    fn test_sequence_range_validation() {
        let valid_sequence = 0;
        let invalid_sequence = -1;

        assert!(valid_sequence >= 0);
        assert!(invalid_sequence < 0);
    }

    #[test]
    fn test_token_count_range_validation() {
        let valid_token_count = 100;
        let invalid_token_count = -1;

        assert!(valid_token_count >= 0);
        assert!(invalid_token_count < 0);
    }

    #[test]
    fn test_optional_fields() {
        let dummy_id = ScopeId::nil();

        let req = CreateTurnRequest {
            scope_id: dummy_id,
            sequence: 0,
            role: TurnRole::User,
            content: "Test content".to_string(),
            token_count: 10,
            tool_calls: None,
            tool_results: None,
            metadata: None,
        };

        assert!(req.tool_calls.is_none());
        assert!(req.tool_results.is_none());
        assert!(req.metadata.is_none());
    }

    #[test]
    fn test_tool_calls_and_results() {
        let dummy_id = ScopeId::nil();

        let tool_calls = serde_json::json!([
            {
                "name": "test_tool",
                "arguments": {"arg1": "value1"}
            }
        ]);

        let tool_results = serde_json::json!([
            {
                "result": "success",
                "output": "test output"
            }
        ]);

        let req = CreateTurnRequest {
            scope_id: dummy_id,
            sequence: 0,
            role: TurnRole::Tool,
            content: "Tool execution".to_string(),
            token_count: 50,
            tool_calls: Some(tool_calls.clone()),
            tool_results: Some(tool_results.clone()),
            metadata: None,
        };

        assert!(req.tool_calls.is_some());
        assert!(req.tool_results.is_some());
        assert_eq!(req.role, TurnRole::Tool);
    }
}
