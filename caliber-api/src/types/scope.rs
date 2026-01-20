//! Scope-related API types

use caliber_core::{EntityId, Timestamp};
use serde::{Deserialize, Serialize};

/// Request to create a new scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateScopeRequest {
    /// Trajectory this scope belongs to
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Parent scope (for nested scopes)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_scope_id: Option<EntityId>,
    /// Name of the scope
    pub name: String,
    /// Purpose/description
    pub purpose: Option<String>,
    /// Token budget for this scope
    pub token_budget: i32,
    /// Additional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to update an existing scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateScopeRequest {
    /// New name (if changing)
    pub name: Option<String>,
    /// New purpose (if changing)
    pub purpose: Option<String>,
    /// New token budget (if changing)
    pub token_budget: Option<i32>,
    /// New metadata (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to create a checkpoint for a scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateCheckpointRequest {
    /// Serialized context state
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_state: Vec<u8>,
    /// Whether this checkpoint is recoverable
    pub recoverable: bool,
}

/// Scope response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ScopeResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    /// Tenant this scope belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_scope_id: Option<EntityId>,
    pub name: String,
    pub purpose: Option<String>,
    pub is_active: bool,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub closed_at: Option<Timestamp>,
    pub checkpoint: Option<CheckpointResponse>,
    pub token_budget: i32,
    pub tokens_used: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Checkpoint response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CheckpointResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_state: Vec<u8>,
    pub recoverable: bool,
}
