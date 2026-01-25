//! Turn-related API types

use caliber_core::{ScopeId, TenantId, Timestamp, TurnId, TurnRole};
use serde::{Deserialize, Serialize};

/// Request to create a new turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateTurnRequest {
    /// Scope this turn belongs to
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
    /// Sequence number within the scope
    pub sequence: i32,
    /// Role of the turn
    pub role: TurnRole,
    /// Content of the turn
    pub content: String,
    /// Token count
    pub token_count: i32,
    /// Tool calls (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_calls: Option<serde_json::Value>,
    /// Tool results (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_results: Option<serde_json::Value>,
    /// Additional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Turn response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TurnResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub turn_id: TurnId,
    /// Tenant this turn belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
    pub sequence: i32,
    pub role: TurnRole,
    pub content: String,
    pub token_count: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_calls: Option<serde_json::Value>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_results: Option<serde_json::Value>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}
