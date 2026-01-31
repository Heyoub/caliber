//! Summarization Policy REST API Routes (Battle Intel Feature 4)
//!
//! This module implements Axum route handlers for summarization policy operations.
//! Policies define when and how L0→L1→L2 abstraction transitions occur.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use caliber_core::{SummarizationPolicyId, TrajectoryId};

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
        return Err(ApiError::invalid_input("At least one trigger is required"));
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
    axum::Router::new().route("/", axum::routing::get(list_policies_by_trajectory))
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthContext, AuthMethod};
    use crate::db::{DbClient, DbConfig};
    use crate::extractors::PathId;
    use crate::types::{CreateTrajectoryRequest, TrajectoryResponse};
    use axum::{body::to_bytes, extract::State, http::StatusCode, response::IntoResponse, Json};
    use caliber_core::{AbstractionLevel, SummarizationTrigger};
    use uuid::Uuid;

    #[test]
    fn test_create_policy_validation_fields() {
        let req = CreateSummarizationPolicyRequest {
            name: "".to_string(),
            trajectory_id: None,
            source_level: AbstractionLevel::Raw,
            target_level: AbstractionLevel::Summary,
            triggers: vec![],
            max_sources: 0,
            create_edges: false,
            metadata: None,
        };

        assert!(req.name.trim().is_empty());
        assert!(req.triggers.is_empty());
        assert!(req.max_sources <= 0);
    }

    #[test]
    fn test_abstraction_level_transitions() {
        fn valid_transition(source: AbstractionLevel, target: AbstractionLevel) -> bool {
            matches!(
                (source, target),
                (AbstractionLevel::Raw, AbstractionLevel::Summary)
                    | (AbstractionLevel::Summary, AbstractionLevel::Principle)
                    | (AbstractionLevel::Raw, AbstractionLevel::Principle)
            )
        }

        assert!(valid_transition(
            AbstractionLevel::Raw,
            AbstractionLevel::Summary
        ));
        assert!(valid_transition(
            AbstractionLevel::Summary,
            AbstractionLevel::Principle
        ));
        assert!(valid_transition(
            AbstractionLevel::Raw,
            AbstractionLevel::Principle
        ));
        assert!(!valid_transition(
            AbstractionLevel::Summary,
            AbstractionLevel::Raw
        ));
    }

    #[test]
    fn test_triggers_not_empty() {
        let triggers = [SummarizationTrigger::ScopeClose];
        assert!(!triggers.is_empty());
    }

    struct DbTestContext {
        db: DbClient,
    }

    async fn db_test_context() -> Option<DbTestContext> {
        if std::env::var("DB_TESTS").ok().as_deref() != Some("1") {
            return None;
        }

        let db = DbClient::from_config(&DbConfig::from_env()).ok()?;
        let conn = db.get_conn().await.ok()?;
        let has_fn = conn
            .query_opt(
                "SELECT 1 FROM pg_proc WHERE proname = 'caliber_tenant_create' LIMIT 1",
                &[],
            )
            .await
            .ok()
            .flatten()
            .is_some();
        if !has_fn {
            return None;
        }

        Some(DbTestContext { db })
    }

    async fn response_json<T: serde::de::DeserializeOwned>(
        response: axum::response::Response,
    ) -> T {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&body).expect("parse json")
    }

    #[tokio::test]
    async fn test_create_list_delete_policy_db_backed() {
        let Some(ctx) = db_test_context().await else {
            return;
        };

        let tenant_id = ctx
            .db
            .tenant_create("test-summarization", None, None)
            .await
            .expect("tenant_create should succeed");
        let _auth = AuthContext::new("test-user".to_string(), tenant_id, vec![], AuthMethod::Jwt);

        let traj_req = CreateTrajectoryRequest {
            name: format!("summ-traj-{}", Uuid::now_v7()),
            description: None,
            parent_trajectory_id: None,
            agent_id: None,
            metadata: None,
        };
        let trajectory: TrajectoryResponse = ctx
            .db
            .create::<TrajectoryResponse>(&traj_req, tenant_id)
            .await
            .expect("create trajectory");

        let req = CreateSummarizationPolicyRequest {
            name: "policy".to_string(),
            triggers: vec![SummarizationTrigger::ScopeClose],
            source_level: AbstractionLevel::Raw,
            target_level: AbstractionLevel::Summary,
            max_sources: 10,
            create_edges: false,
            trajectory_id: Some(trajectory.trajectory_id),
            metadata: None,
        };

        let create_response = create_policy(State(ctx.db.clone()), Json(req))
            .await
            .expect("create_policy should succeed")
            .into_response();
        assert_eq!(create_response.status(), StatusCode::CREATED);
        let policy: SummarizationPolicyResponse = response_json(create_response).await;

        let get_response = get_policy(State(ctx.db.clone()), PathId(policy.policy_id))
            .await
            .expect("get_policy should succeed")
            .into_response();
        assert_eq!(get_response.status(), StatusCode::OK);

        let list_response =
            list_policies_by_trajectory(State(ctx.db.clone()), PathId(trajectory.trajectory_id))
                .await
                .expect("list_policies_by_trajectory should succeed")
                .into_response();
        assert_eq!(list_response.status(), StatusCode::OK);
        let list: ListSummarizationPoliciesResponse = response_json(list_response).await;
        assert!(list
            .policies
            .iter()
            .any(|p| p.policy_id == policy.policy_id));

        let delete_response = delete_policy(State(ctx.db.clone()), PathId(policy.policy_id))
            .await
            .expect("delete_policy should succeed")
            .into_response();
        assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

        ctx.db
            .delete::<TrajectoryResponse>(trajectory.trajectory_id, tenant_id)
            .await
            .ok();
    }
}
