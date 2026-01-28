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
    use crate::db::DbConfig;
    use crate::error::ErrorCode;
    use caliber_core::{AgentId, HandoffId, TenantId, TrajectoryId, ScopeId};
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
}
