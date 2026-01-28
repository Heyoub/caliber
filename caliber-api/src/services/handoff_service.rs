//! Handoff Service
//!
//! Business logic for handoff operations, extracted from HandoffResponse.

use caliber_core::{AgentId, HandoffStatus};

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::types::HandoffResponse;

/// Accept a handoff (Initiated -> Accepted transition).
///
/// # Arguments
/// - `db`: Database client for persisting the update
/// - `handoff`: The handoff to accept
/// - `accepting_agent_id`: ID of the agent accepting the handoff
///
/// # Errors
/// Returns error if handoff is not in Initiated state or agent is not the recipient.
pub async fn accept_handoff(
    db: &DbClient,
    handoff: &HandoffResponse,
    accepting_agent_id: AgentId,
) -> ApiResult<HandoffResponse> {
    if handoff.status != HandoffStatus::Initiated {
        return Err(ApiError::state_conflict(format!(
            "Handoff is in '{:?}' state, cannot accept (expected Initiated)",
            handoff.status
        )));
    }

    // Verify the accepting agent is the recipient
    if handoff.to_agent_id != accepting_agent_id {
        return Err(ApiError::forbidden(
            "Only the recipient agent can accept this handoff",
        ));
    }

    let updates = serde_json::json!({
        "status": "Accepted",
        "accepted_at": chrono::Utc::now().to_rfc3339()
    });

    db.update_raw::<HandoffResponse>(handoff.handoff_id, updates, handoff.tenant_id).await
}

/// Complete a handoff (Accepted -> Completed transition).
///
/// # Arguments
/// - `db`: Database client for persisting the update
/// - `handoff`: The handoff to complete
///
/// # Errors
/// Returns error if handoff is not in Accepted state.
pub async fn complete_handoff(
    db: &DbClient,
    handoff: &HandoffResponse,
) -> ApiResult<HandoffResponse> {
    if handoff.status != HandoffStatus::Accepted {
        return Err(ApiError::state_conflict(format!(
            "Handoff is in '{:?}' state, cannot complete (expected Accepted)",
            handoff.status
        )));
    }

    let updates = serde_json::json!({
        "status": "Completed",
        "completed_at": chrono::Utc::now().to_rfc3339()
    });

    db.update_raw::<HandoffResponse>(handoff.handoff_id, updates, handoff.tenant_id).await
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
        AgentResponse, CreateHandoffRequest, CreateScopeRequest, CreateTrajectoryRequest,
        MemoryAccessRequest, MemoryPermissionRequest, RegisterAgentRequest, ScopeResponse,
        TrajectoryResponse,
    };
    use caliber_core::{AgentId, HandoffId, HandoffStatus, ScopeId, TenantId, TrajectoryId};
    use chrono::Utc;

    fn dummy_db() -> DbClient {
        DbClient::from_config(&DbConfig::default()).expect("db client")
    }

    fn sample_handoff(status: HandoffStatus, to_agent: AgentId) -> HandoffResponse {
        HandoffResponse {
            tenant_id: TenantId::now_v7(),
            handoff_id: HandoffId::now_v7(),
            from_agent_id: AgentId::now_v7(),
            to_agent_id: to_agent,
            trajectory_id: TrajectoryId::now_v7(),
            scope_id: Some(ScopeId::now_v7()),
            reason: "handoff".to_string(),
            status,
            created_at: Utc::now(),
            accepted_at: None,
            completed_at: None,
            context_snapshot: vec![1, 2, 3],
        }
    }

    #[tokio::test]
    async fn test_accept_handoff_rejects_non_initiated() {
        let db = dummy_db();
        let to_agent = AgentId::now_v7();
        let handoff = sample_handoff(HandoffStatus::Accepted, to_agent);
        let err = accept_handoff(&db, &handoff, to_agent).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::StateConflict);
    }

    #[tokio::test]
    async fn test_accept_handoff_rejects_wrong_agent() {
        let db = dummy_db();
        let to_agent = AgentId::now_v7();
        let handoff = sample_handoff(HandoffStatus::Initiated, to_agent);
        let err = accept_handoff(&db, &handoff, AgentId::now_v7())
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }

    #[tokio::test]
    async fn test_complete_handoff_rejects_non_accepted() {
        let db = dummy_db();
        let to_agent = AgentId::now_v7();
        let handoff = sample_handoff(HandoffStatus::Initiated, to_agent);
        let err = complete_handoff(&db, &handoff).await.unwrap_err();
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

        let tenant_id = db.tenant_create("test-handoff-service", None, None).await.ok()?;
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
            name: format!("handoff-svc-{}", uuid::Uuid::now_v7()),
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
            name: "handoff-scope".to_string(),
            purpose: None,
            token_budget: 1000,
            metadata: None,
        };

        db.create::<ScopeResponse>(&req, tenant_id)
            .await
            .expect("create scope")
    }

    async fn create_handoff(
        db: &DbClient,
        tenant_id: TenantId,
        from_agent: AgentId,
        to_agent: AgentId,
        trajectory_id: TrajectoryId,
        scope_id: ScopeId,
    ) -> HandoffResponse {
        let req = CreateHandoffRequest {
            from_agent_id: from_agent,
            to_agent_id: to_agent,
            trajectory_id,
            scope_id,
            reason: "handoff".to_string(),
            context_snapshot: vec![1, 2, 3],
        };

        db.create::<HandoffResponse>(&req, tenant_id)
            .await
            .expect("create handoff")
    }

    #[tokio::test]
    async fn test_accept_complete_handoff_db_backed() {
        let Some(ctx) = db_test_context().await else { return; };

        let trajectory = create_trajectory(&ctx.db, ctx.tenant_id).await;
        let scope = create_scope(&ctx.db, ctx.tenant_id, trajectory.trajectory_id).await;
        let from_agent = register_agent(&ctx.db, ctx.tenant_id, "handoff-from").await;
        let to_agent = register_agent(&ctx.db, ctx.tenant_id, "handoff-to").await;

        let handoff = create_handoff(
            &ctx.db,
            ctx.tenant_id,
            from_agent.agent_id,
            to_agent.agent_id,
            trajectory.trajectory_id,
            scope.scope_id,
        )
        .await;
        assert_eq!(handoff.status, HandoffStatus::Initiated);

        let accepted = accept_handoff(&ctx.db, &handoff, to_agent.agent_id)
            .await
            .expect("accept handoff");
        assert_eq!(accepted.status, HandoffStatus::Accepted);

        let completed = complete_handoff(&ctx.db, &accepted)
            .await
            .expect("complete handoff");
        assert_eq!(completed.status, HandoffStatus::Completed);

        ctx.db
            .delete::<HandoffResponse>(completed.handoff_id, ctx.tenant_id)
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
            .delete::<AgentResponse>(from_agent.agent_id, ctx.tenant_id)
            .await
            .ok();
        ctx.db
            .delete::<AgentResponse>(to_agent.agent_id, ctx.tenant_id)
            .await
            .ok();
    }
}
