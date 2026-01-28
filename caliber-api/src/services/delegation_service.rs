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
    use crate::db::DbConfig;
    use crate::error::ErrorCode;
    use caliber_core::{AgentId, DelegationId, DelegationResultStatus, ScopeId, TenantId, TrajectoryId};
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
}
