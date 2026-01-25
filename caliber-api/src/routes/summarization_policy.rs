//! Summarization Policy REST API Routes (Battle Intel Feature 4)
//!
//! This module implements Axum route handlers for summarization policy operations.
//! Policies define when and how L0→L1→L2 abstraction transitions occur.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use caliber_core::{SummarizationPolicyId, TrajectoryId};
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    extractors::PathId,
    state::AppState,
    types::{
        CreateSummarizationPolicyRequest, ListSummarizationPoliciesResponse,
        SummarizationPolicyResponse,
    },
};

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/summarization-policies - Create a new summarization policy
#[utoipa::path(
    post,
    path = "/api/v1/summarization-policies",
    tag = "SummarizationPolicies",
    request_body = CreateSummarizationPolicyRequest,
    responses(
        (status = 201, description = "Policy created successfully", body = SummarizationPolicyResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_policy(
    State(db): State<DbClient>,
    Json(req): Json<CreateSummarizationPolicyRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.name.trim().is_empty() {
        return Err(ApiError::missing_field("name"));
    }

    if req.triggers.is_empty() {
        return Err(ApiError::invalid_input(
            "At least one trigger is required",
        ));
    }

    if req.max_sources <= 0 {
        return Err(ApiError::invalid_input("max_sources must be positive"));
    }

    // Validate abstraction level progression
    // Raw -> Summary -> Principle
    use caliber_core::AbstractionLevel;
    let valid_transition = match (&req.source_level, &req.target_level) {
        (AbstractionLevel::Raw, AbstractionLevel::Summary) => true,
        (AbstractionLevel::Summary, AbstractionLevel::Principle) => true,
        (AbstractionLevel::Raw, AbstractionLevel::Principle) => true, // Skip L1
        _ => false,
    };

    if !valid_transition {
        return Err(ApiError::invalid_input(
            "Invalid abstraction level transition (must go from lower to higher level)",
        ));
    }

    // Create policy via database client
    let policy = db.summarization_policy_create(&req).await?;

    Ok((StatusCode::CREATED, Json(policy)))
}

/// GET /api/v1/summarization-policies/{id} - Get a policy by ID
#[utoipa::path(
    get,
    path = "/api/v1/summarization-policies/{id}",
    tag = "SummarizationPolicies",
    params(
        ("id" = String, Path, description = "Policy ID")
    ),
    responses(
        (status = 200, description = "Policy found", body = SummarizationPolicyResponse),
        (status = 404, description = "Policy not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_policy(
    State(db): State<DbClient>,
    PathId(id): PathId<SummarizationPolicyId>,
) -> ApiResult<impl IntoResponse> {
    let policy = db.summarization_policy_get(id).await?;

    match policy {
        Some(p) => Ok(Json(p)),
        None => Err(ApiError::entity_not_found("SummarizationPolicy", id)),
    }
}

/// GET /api/v1/trajectories/{id}/summarization-policies - List policies for a trajectory
#[utoipa::path(
    get,
    path = "/api/v1/trajectories/{id}/summarization-policies",
    tag = "SummarizationPolicies",
    params(
        ("id" = String, Path, description = "Trajectory ID")
    ),
    responses(
        (status = 200, description = "Policies found", body = ListSummarizationPoliciesResponse),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_policies_by_trajectory(
    State(db): State<DbClient>,
    PathId(trajectory_id): PathId<TrajectoryId>,
) -> ApiResult<impl IntoResponse> {
    let policies = db
        .summarization_policies_for_trajectory(trajectory_id)
        .await?;

    Ok(Json(ListSummarizationPoliciesResponse { policies }))
}

/// DELETE /api/v1/summarization-policies/{id} - Delete a policy
#[utoipa::path(
    delete,
    path = "/api/v1/summarization-policies/{id}",
    tag = "SummarizationPolicies",
    params(
        ("id" = String, Path, description = "Policy ID")
    ),
    responses(
        (status = 204, description = "Policy deleted successfully"),
        (status = 404, description = "Policy not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn delete_policy(
    State(db): State<DbClient>,
    PathId(id): PathId<SummarizationPolicyId>,
) -> ApiResult<impl IntoResponse> {
    db.summarization_policy_delete(id).await?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// ROUTER FACTORY
// ============================================================================

/// Create the summarization policy router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(create_policy))
        .route("/{id}", axum::routing::get(get_policy))
        .route("/{id}", axum::routing::delete(delete_policy))
}

/// Create the trajectory-scoped summarization policy router.
/// This is nested under /api/v1/trajectories/{id}/summarization-policies
pub fn create_trajectory_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list_policies_by_trajectory))
}
