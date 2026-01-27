//! Lock-related API types

use caliber_core::{AgentId, LockId, TenantId, Timestamp};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::db::DbClient;
use crate::error::ApiResult;

/// Request to acquire a lock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AcquireLockRequest {
    /// Type of resource to lock
    pub resource_type: String,
    /// ID of the resource to lock
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub resource_id: Uuid,
    /// Agent requesting the lock
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub holder_agent_id: AgentId,
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

/// Request to release a lock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ReleaseLockRequest {
    /// Agent releasing the lock (must be the holder)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub releasing_agent_id: AgentId,
}

/// Lock response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LockResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub lock_id: LockId,
    pub resource_type: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub resource_id: Uuid,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub holder_agent_id: AgentId,
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
// STATE TRANSITION METHODS (delegating to service layer)
// ============================================================================

impl LockResponse {
    /// Check if this lock is currently held (not expired).
    #[inline]
    pub fn is_held(&self) -> bool {
        crate::services::is_lock_held(self)
    }

    /// Release this lock.
    ///
    /// Delegates to `services::release_lock()`.
    pub async fn release(&self, db: &DbClient, releasing_agent_id: AgentId) -> ApiResult<()> {
        crate::services::release_lock(db, self, releasing_agent_id).await
    }

    /// Extend this lock's expiration time.
    ///
    /// Delegates to `services::extend_lock()`.
    pub async fn extend(&self, db: &DbClient, additional: Duration) -> ApiResult<Self> {
        crate::services::extend_lock(db, self, additional).await
    }
}
