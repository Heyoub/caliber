//! Lock Service
//!
//! Business logic for lock operations, extracted from LockResponse.

use caliber_core::AgentId;
use std::time::Duration;

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::types::LockResponse;

/// Check if a lock is currently held (not expired).
pub fn is_lock_held(lock: &LockResponse) -> bool {
    chrono::Utc::now() < lock.expires_at
}

/// Release a lock.
///
/// # Arguments
/// - `db`: Database client for persisting the update
/// - `lock`: The lock to release
/// - `releasing_agent_id`: ID of the agent releasing the lock
///
/// # Errors
/// Returns error if the agent is not the lock holder or lock is expired.
pub async fn release_lock(
    db: &DbClient,
    lock: &LockResponse,
    releasing_agent_id: AgentId,
) -> ApiResult<()> {
    // Verify the releasing agent is the lock holder
    if lock.holder_agent_id != releasing_agent_id {
        return Err(ApiError::forbidden(
            "Only the lock holder can release this lock",
        ));
    }

    if !is_lock_held(lock) {
        return Err(ApiError::state_conflict("Lock has already expired"));
    }

    // Delete the lock record (release = delete for locks)
    db.delete::<LockResponse>(lock.lock_id, lock.tenant_id).await?;
    Ok(())
}

/// Extend a lock's expiration time.
///
/// # Arguments
/// - `db`: Database client for persisting the update
/// - `lock`: The lock to extend
/// - `additional`: Additional duration to add to expiration
///
/// # Errors
/// Returns error if the lock has expired.
pub async fn extend_lock(
    db: &DbClient,
    lock: &LockResponse,
    additional: Duration,
) -> ApiResult<LockResponse> {
    if !is_lock_held(lock) {
        return Err(ApiError::state_conflict("Cannot extend an expired lock"));
    }

    let new_expires = lock.expires_at
        + chrono::Duration::from_std(additional)
            .map_err(|e| ApiError::invalid_input(format!("Invalid duration: {}", e)))?;

    let updates = serde_json::json!({
        "expires_at": new_expires.to_rfc3339()
    });

    db.update_raw::<LockResponse>(lock.lock_id, updates, lock.tenant_id).await
}
