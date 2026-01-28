//! Delegation Service
//!
//! Business logic for delegation operations, extracted from DelegationResponse.

use caliber_core::{AgentId, DelegationStatus};

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::types::{DelegationResponse, DelegationResultResponse};

/// Accept a delegation (Pending -> Accepted transition).
///
/// # Arguments
/// - `db`: Database client for persisting the update
/// - `delegation`: The delegation to accept
/// - `accepting_agent_id`: ID of the agent accepting the delegation
///
/// # Errors
/// Returns error if delegation is not in Pending state.
pub async fn accept_delegation(
    db: &DbClient,
    delegation: &DelegationResponse,
    accepting_agent_id: AgentId,
) -> ApiResult<DelegationResponse> {
    if delegation.status != DelegationStatus::Pending {
        return Err(ApiError::state_conflict(format!(
            "Delegation is in '{:?}' state, cannot accept (expected Pending)",
            delegation.status
        )));
    }

    // Verify the accepting agent is the delegatee
    if delegation.delegatee_id != accepting_agent_id {
        return Err(ApiError::forbidden(
            "Only the delegatee can accept this delegation",
        ));
    }

    let updates = serde_json::json!({
        "status": "Accepted",
        "accepted_at": chrono::Utc::now().to_rfc3339()
    });

    db.update_raw::<DelegationResponse>(delegation.delegation_id, updates, delegation.tenant_id).await
}

/// Reject a delegation (Pending -> Rejected transition).
///
/// # Arguments
/// - `db`: Database client for persisting the update
/// - `delegation`: The delegation to reject
/// - `rejecting_agent_id`: ID of the agent rejecting the delegation
/// - `reason`: Reason for rejection
///
/// # Errors
/// Returns error if delegation is not in Pending state.
pub async fn reject_delegation(
    db: &DbClient,
    delegation: &DelegationResponse,
    rejecting_agent_id: AgentId,
    reason: &str,
) -> ApiResult<DelegationResponse> {
    if delegation.status != DelegationStatus::Pending {
        return Err(ApiError::state_conflict(format!(
            "Delegation is in '{:?}' state, cannot reject (expected Pending)",
            delegation.status
        )));
    }

    // Verify the rejecting agent is the delegatee
    if delegation.delegatee_id != rejecting_agent_id {
        return Err(ApiError::forbidden(
            "Only the delegatee can reject this delegation",
        ));
    }

    let updates = serde_json::json!({
        "status": "Rejected",
        "rejection_reason": reason
    });

    db.update_raw::<DelegationResponse>(delegation.delegation_id, updates, delegation.tenant_id).await
}

/// Complete a delegation (Accepted/InProgress -> Completed transition).
///
/// # Arguments
/// - `db`: Database client for persisting the update
/// - `delegation`: The delegation to complete
/// - `result`: The result of the delegation
///
/// # Errors
/// Returns error if delegation is not in Accepted or InProgress state.
pub async fn complete_delegation(
    db: &DbClient,
    delegation: &DelegationResponse,
    result: &DelegationResultResponse,
) -> ApiResult<DelegationResponse> {
    let can_complete = matches!(
        delegation.status,
        DelegationStatus::Accepted | DelegationStatus::InProgress
    );

    if !can_complete {
        return Err(ApiError::state_conflict(format!(
            "Delegation is in '{:?}' state, cannot complete (expected Accepted or InProgress)",
            delegation.status
        )));
    }

    let result_json = serde_json::to_value(result)?;

    let updates = serde_json::json!({
        "status": "Completed",
        "completed_at": chrono::Utc::now().to_rfc3339(),
        "result": result_json
    });

    db.update_raw::<DelegationResponse>(delegation.delegation_id, updates, delegation.tenant_id).await
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DbClient, DbConfig};
    use crate::error::ErrorCode;
    use crate::types::{
        AgentResponse, CreateDelegationRequest, CreateScopeRequest, CreateTrajectoryRequest,
        MemoryAccessRequest, MemoryPermissionRequest, RegisterAgentRequest, ScopeResponse,
        TrajectoryResponse,
    };
    use caliber_core::{
        AgentId, DelegationId, DelegationResultStatus, DelegationStatus, ScopeId, TenantId,
        TrajectoryId,
    };
    use chrono::Utc;

    fn dummy_db() -> DbClient {
        DbClient::from_config(&DbConfig::default()).expect("db client")
    }

    fn sample_delegation(status: DelegationStatus, delegatee: AgentId) -> DelegationResponse {
        DelegationResponse {
            delegation_id: DelegationId::now_v7(),
            tenant_id: TenantId::now_v7(),
            delegator_id: AgentId::now_v7(),
            delegatee_id: delegatee,
            trajectory_id: TrajectoryId::now_v7(),
            scope_id: Some(ScopeId::now_v7()),
            task_description: "task".to_string(),
            status,
            created_at: Utc::now(),
            accepted_at: None,
            completed_at: None,
            expected_completion: None,
            result: None,
            context: None,
        }
    }

    #[tokio::test]
    async fn test_accept_delegation_rejects_non_pending() {
        let db = dummy_db();
        let delegatee = AgentId::now_v7();
        let delegation = sample_delegation(DelegationStatus::Accepted, delegatee);
        let err = accept_delegation(&db, &delegation, delegatee).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::StateConflict);
    }

    #[tokio::test]
    async fn test_accept_delegation_rejects_wrong_agent() {
        let db = dummy_db();
        let delegatee = AgentId::now_v7();
        let delegation = sample_delegation(DelegationStatus::Pending, delegatee);
        let err = accept_delegation(&db, &delegation, AgentId::now_v7())
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }

    #[tokio::test]
    async fn test_reject_delegation_rejects_non_pending() {
        let db = dummy_db();
        let delegatee = AgentId::now_v7();
        let delegation = sample_delegation(DelegationStatus::Completed, delegatee);
        let err = reject_delegation(&db, &delegation, delegatee, "no")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::StateConflict);
    }

    #[tokio::test]
    async fn test_reject_delegation_rejects_wrong_agent() {
        let db = dummy_db();
        let delegatee = AgentId::now_v7();
        let delegation = sample_delegation(DelegationStatus::Pending, delegatee);
        let err = reject_delegation(&db, &delegation, AgentId::now_v7(), "no")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }

    #[tokio::test]
    async fn test_complete_delegation_rejects_invalid_state() {
        let db = dummy_db();
        let delegatee = AgentId::now_v7();
        let delegation = sample_delegation(DelegationStatus::Rejected, delegatee);
        let result = DelegationResultResponse {
            status: DelegationResultStatus::Success,
            output: Some("ok".to_string()),
            artifacts: vec![],
            error: None,
        };
        let err = complete_delegation(&db, &delegation, &result)
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::StateConflict);
    }

    struct DbTestContext {
        db: DbClient,
        tenant_id: TenantId,
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

        let tenant_id = db.tenant_create("test-delegation-service", None, None).await.ok()?;
        Some(DbTestContext { db, tenant_id })
    }

    async fn register_agent(db: &DbClient, tenant_id: TenantId, agent_type: &str) -> AgentResponse {
        let req = RegisterAgentRequest {
            agent_type: agent_type.to_string(),
            capabilities: vec!["read".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
                write: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
            },
            can_delegate_to: vec!["planner".to_string()],
            reports_to: None,
        };

        db.agent_register(&req, tenant_id)
            .await
            .expect("register agent")
    }

    async fn create_trajectory(db: &DbClient, tenant_id: TenantId) -> TrajectoryResponse {
        let req = CreateTrajectoryRequest {
            name: format!("delegation-svc-{}", uuid::Uuid::now_v7()),
            description: None,
            parent_trajectory_id: None,
            agent_id: None,
            metadata: None,
        };

        db.create::<TrajectoryResponse>(&req, tenant_id)
            .await
            .expect("create trajectory")
    }

    async fn create_scope(
        db: &DbClient,
        tenant_id: TenantId,
        trajectory_id: TrajectoryId,
    ) -> ScopeResponse {
        let req = CreateScopeRequest {
            trajectory_id,
            parent_scope_id: None,
            name: "delegation-scope".to_string(),
            purpose: None,
            token_budget: 1000,
            metadata: None,
        };

        db.create::<ScopeResponse>(&req, tenant_id)
            .await
            .expect("create scope")
    }

    async fn create_delegation(
        db: &DbClient,
        tenant_id: TenantId,
        from_agent: AgentId,
        to_agent: AgentId,
        trajectory_id: TrajectoryId,
        scope_id: ScopeId,
    ) -> DelegationResponse {
        let req = CreateDelegationRequest {
            from_agent_id: from_agent,
            to_agent_id: to_agent,
            trajectory_id,
            scope_id,
            task_description: "do the thing".to_string(),
            expected_completion: None,
            context: None,
        };

        db.create::<DelegationResponse>(&req, tenant_id)
            .await
            .expect("create delegation")
    }

    #[tokio::test]
    async fn test_accept_complete_delegation_db_backed() {
        let Some(ctx) = db_test_context().await else { return; };

        let trajectory = create_trajectory(&ctx.db, ctx.tenant_id).await;
        let scope = create_scope(&ctx.db, ctx.tenant_id, trajectory.trajectory_id).await;
        let delegator = register_agent(&ctx.db, ctx.tenant_id, "delegator").await;
        let delegatee = register_agent(&ctx.db, ctx.tenant_id, "delegatee").await;

        let delegation = create_delegation(
            &ctx.db,
            ctx.tenant_id,
            delegator.agent_id,
            delegatee.agent_id,
            trajectory.trajectory_id,
            scope.scope_id,
        )
        .await;
        assert_eq!(delegation.status, DelegationStatus::Pending);

        let accepted = accept_delegation(&ctx.db, &delegation, delegatee.agent_id)
            .await
            .expect("accept delegation");
        assert_eq!(accepted.status, DelegationStatus::Accepted);

        let result = DelegationResultResponse {
            status: DelegationResultStatus::Success,
            output: Some("ok".to_string()),
            artifacts: vec![],
            error: None,
        };
        let completed = complete_delegation(&ctx.db, &accepted, &result)
            .await
            .expect("complete delegation");
        assert_eq!(completed.status, DelegationStatus::Completed);
        assert!(completed.result.is_some());

        ctx.db
            .delete::<DelegationResponse>(completed.delegation_id, ctx.tenant_id)
            .await
            .ok();
        ctx.db
            .delete::<ScopeResponse>(scope.scope_id, ctx.tenant_id)
            .await
            .ok();
        ctx.db
            .delete::<TrajectoryResponse>(trajectory.trajectory_id, ctx.tenant_id)
            .await
            .ok();
        ctx.db
            .delete::<AgentResponse>(delegator.agent_id, ctx.tenant_id)
            .await
            .ok();
        ctx.db
            .delete::<AgentResponse>(delegatee.agent_id, ctx.tenant_id)
            .await
            .ok();
    }
}
