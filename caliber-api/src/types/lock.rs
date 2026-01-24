//! Lock-related API types

use caliber_core::{EntityId, Timestamp};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};

/// Request to acquire a lock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AcquireLockRequest {
    /// Type of resource to lock
    pub resource_type: String,
    /// ID of the resource to lock
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub resource_id: EntityId,
    /// Agent requesting the lock
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub holder_agent_id: EntityId,
    /// Lock timeout in milliseconds
    pub timeout_ms: i64,
    /// Lock mode (Exclusive or Shared)
    pub mode: String,
}

/// Request to extend a lock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExtendLockRequest {
    /// Additional time in milliseconds
    pub additional_ms: i64,
}

/// Lock response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LockResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub lock_id: EntityId,
    pub resource_type: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub resource_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub holder_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub acquired_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub expires_at: Timestamp,
    pub mode: String,
}

/// Response containing a list of locks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListLocksResponse {
    /// List of locks
    pub locks: Vec<LockResponse>,
    /// Total count
    pub total: i32,
}

// ============================================================================
// STATE TRANSITION METHODS
// ============================================================================

impl LockResponse {
    /// Check if this lock is currently held (not expired).
    pub fn is_held(&self) -> bool {
        chrono::Utc::now() < self.expires_at
    }

    /// Release this lock.
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    /// - `releasing_agent_id`: ID of the agent releasing the lock
    ///
    /// # Errors
    /// Returns error if the agent is not the lock holder or lock is expired.
    pub async fn release(&self, db: &DbClient, releasing_agent_id: EntityId) -> ApiResult<()> {
        // Verify the releasing agent is the lock holder
        if self.holder_agent_id != releasing_agent_id {
            return Err(ApiError::forbidden(
                "Only the lock holder can release this lock",
            ));
        }

        if !self.is_held() {
            return Err(ApiError::state_conflict(
                "Lock has already expired",
            ));
        }

        // Delete the lock record (release = delete for locks)
        db.delete::<Self>(self.lock_id, self.tenant_id).await?;
        Ok(())
    }

    /// Extend this lock's expiration time.
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    /// - `additional`: Additional duration to add to expiration
    ///
    /// # Errors
    /// Returns error if the lock has expired.
    pub async fn extend(&self, db: &DbClient, additional: Duration) -> ApiResult<Self> {
        if !self.is_held() {
            return Err(ApiError::state_conflict(
                "Cannot extend an expired lock",
            ));
        }

        let new_expires = self.expires_at + chrono::Duration::from_std(additional)
            .map_err(|e| ApiError::invalid_input(format!("Invalid duration: {}", e)))?;

        let updates = serde_json::json!({
            "expires_at": new_expires.to_rfc3339()
        });

        db.update_raw::<Self>(self.lock_id, updates, self.tenant_id).await
    }
}
