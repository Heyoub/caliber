//! Scope-related API types

use caliber_core::{ScopeId, TenantId, Timestamp, TrajectoryId};
use serde::{Deserialize, Serialize};

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};

use super::{Linkable, Links, LINK_REGISTRY};

/// Request to create a new scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateScopeRequest {
    /// Trajectory this scope belongs to
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    /// Parent scope (for nested scopes)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_scope_id: Option<ScopeId>,
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
    pub scope_id: ScopeId,
    /// Tenant this scope belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_scope_id: Option<ScopeId>,
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

    /// HATEOAS links for available actions.
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub links: Option<Links>,
}

impl Linkable for ScopeResponse {
    const ENTITY_TYPE: &'static str = "scope";

    fn link_id(&self) -> String {
        self.scope_id.to_string()
    }

    fn check_condition(&self, condition: &str) -> bool {
        match condition {
            "active" => self.is_active,
            "has_parent" => self.parent_scope_id.is_some(),
            _ => true,
        }
    }

    fn relation_id(&self, relation: &str) -> Option<String> {
        match relation {
            "trajectory_id" => Some(self.trajectory_id.to_string()),
            "parent_id" => self.parent_scope_id.map(|id| id.to_string()),
            _ => None,
        }
    }
}

impl ScopeResponse {
    /// Add HATEOAS links from the registry.
    pub fn linked(mut self) -> Self {
        self.links = Some(LINK_REGISTRY.generate(&self));
        self
    }
}

/// Checkpoint response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CheckpointResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_state: Vec<u8>,
    pub recoverable: bool,
}

// ============================================================================
// STATE TRANSITION METHODS
// ============================================================================

impl ScopeResponse {
    /// Close this scope (set is_active to false).
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    ///
    /// # Errors
    /// Returns error if the scope is already closed.
    pub async fn close(&self, db: &DbClient) -> ApiResult<Self> {
        if !self.is_active {
            return Err(ApiError::state_conflict("Scope is already closed"));
        }

        let tenant_id = self.tenant_id;

        let updates = serde_json::json!({
            "is_active": false,
            "closed_at": chrono::Utc::now().to_rfc3339()
        });

        db.update_raw::<Self>(self.scope_id, updates, tenant_id)
            .await
    }

    /// Create a checkpoint for this scope.
    ///
    /// # Arguments
    /// - `db`: Database client for persisting the update
    /// - `req`: Checkpoint creation request
    ///
    /// # Errors
    /// Returns error if the scope is inactive.
    pub async fn create_checkpoint(
        &self,
        db: &DbClient,
        req: &CreateCheckpointRequest,
    ) -> ApiResult<Self> {
        if !self.is_active {
            return Err(ApiError::state_conflict(
                "Cannot create checkpoint for inactive scope",
            ));
        }

        let tenant_id = self.tenant_id;

        let checkpoint = CheckpointResponse {
            context_state: req.context_state.clone(),
            recoverable: req.recoverable,
        };

        let checkpoint_json = serde_json::to_value(&checkpoint)?;

        let updates = serde_json::json!({
            "checkpoint": checkpoint_json
        });

        db.update_raw::<Self>(self.scope_id, updates, tenant_id)
            .await
    }
}
