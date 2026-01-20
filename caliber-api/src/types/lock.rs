//! Lock-related API types

use caliber_core::{EntityId, Timestamp};
use serde::{Deserialize, Serialize};

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
