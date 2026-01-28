//! Agent-related API types

use caliber_core::{AgentId, AgentStatus, ScopeId, TenantId, Timestamp, TrajectoryId};
use serde::{Deserialize, Serialize};

use crate::db::DbClient;
use crate::error::ApiResult;

/// Request to register a new agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RegisterAgentRequest {
    /// Type of agent
    pub agent_type: String,
    /// Capabilities this agent has
    pub capabilities: Vec<String>,
    /// Memory access permissions
    pub memory_access: MemoryAccessRequest,
    /// Agent types this agent can delegate to
    pub can_delegate_to: Vec<String>,
    /// Supervisor agent (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub reports_to: Option<AgentId>,
}

/// Memory access configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryAccessRequest {
    /// Read permissions
    pub read: Vec<MemoryPermissionRequest>,
    /// Write permissions
    pub write: Vec<MemoryPermissionRequest>,
}

/// A single memory permission entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryPermissionRequest {
    /// Type of memory
    pub memory_type: String,
    /// Scope of the permission
    pub scope: String,
    /// Optional filter expression
    pub filter: Option<String>,
}

/// Request to update an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateAgentRequest {
    /// New status (if changing)
    pub status: Option<AgentStatus>,
    /// New current trajectory (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_trajectory_id: Option<TrajectoryId>,
    /// New current scope (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_scope_id: Option<ScopeId>,
    /// New capabilities (if changing)
    pub capabilities: Option<Vec<String>>,
    /// New memory access (if changing)
    pub memory_access: Option<MemoryAccessRequest>,
}

/// Agent response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub agent_id: AgentId,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub memory_access: MemoryAccessResponse,
    pub status: AgentStatus,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_trajectory_id: Option<TrajectoryId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_scope_id: Option<ScopeId>,
    pub can_delegate_to: Vec<String>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub reports_to: Option<AgentId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub last_heartbeat: Timestamp,
}

/// Memory access response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryAccessResponse {
    pub read: Vec<MemoryPermissionResponse>,
    pub write: Vec<MemoryPermissionResponse>,
}

/// Memory permission response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryPermissionResponse {
    pub memory_type: String,
    pub scope: String,
    pub filter: Option<String>,
}

/// Request to list agents with filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListAgentsRequest {
    /// Filter by agent type
    pub agent_type: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by current trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<TrajectoryId>,
    /// Only return active agents
    pub active_only: Option<bool>,
}

/// Response containing a list of agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListAgentsResponse {
    /// List of agents
    pub agents: Vec<AgentResponse>,
    /// Total count
    pub total: i32,
}

// ============================================================================
// STATE TRANSITION METHODS
// ============================================================================

impl AgentResponse {
    /// Send a heartbeat to update last_heartbeat timestamp.
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    ///
    /// # Returns
    /// Updated agent with new last_heartbeat timestamp.
    pub async fn heartbeat(&self, db: &DbClient) -> ApiResult<Self> {
        let updates = serde_json::json!({
            "last_heartbeat": chrono::Utc::now().to_rfc3339()
        });

        db.update_raw::<Self>(self.agent_id, updates, self.tenant_id)
            .await
    }

    /// Unregister this agent (set status to Offline).
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    ///
    /// # Returns
    /// Updated agent with Offline status.
    pub async fn unregister(&self, db: &DbClient) -> ApiResult<Self> {
        let updates = serde_json::json!({
            "status": "Offline"
        });

        db.update_raw::<Self>(self.agent_id, updates, self.tenant_id)
            .await
    }
}
