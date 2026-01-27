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
