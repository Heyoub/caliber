//! Message component implementation.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{ListMessagesRequest, MessageResponse, SendMessageRequest};
use caliber_core::{AgentId, EntityIdType, MessageId, TenantId, TrajectoryId};
use serde_json::Value as JsonValue;

// Implement Component trait for MessageResponse
// Note: Messages are immutable, so update_type is () - use deliver/acknowledge operations instead
impl_component! {
    MessageResponse {
        entity_name: "message",
        pk_field: "message_id",
        id_type: MessageId,
        requires_tenant: true,
        create_type: SendMessageRequest,
        update_type: (),
        filter_type: MessageListFilter,
        entity_id: |self| self.message_id,
        create_params: |req, tenant_id| vec![
            SqlParam::Uuid(req.from_agent_id.as_uuid()),
            SqlParam::OptUuid(req.to_agent_id.map(|id| id.as_uuid())),
            SqlParam::OptString(req.to_agent_type.clone()),
            SqlParam::String(req.message_type.clone()),
            SqlParam::String(req.payload.clone()),
            SqlParam::OptUuid(req.trajectory_id.map(|id| id.as_uuid())),
            SqlParam::OptUuid(req.scope_id.map(|id| id.as_uuid())),
            SqlParam::Json(serde_json::to_value(&req.artifact_ids).unwrap_or(JsonValue::Array(vec![]))),
            SqlParam::String(req.priority.clone()),
            SqlParam::OptTimestamp(req.expires_at),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 11,
        build_updates: |_req| {
            // Messages are immutable - no updates allowed
            // Use deliver/acknowledge operations instead
            JsonValue::Object(serde_json::Map::new())
        },
        not_found_error: |id| ApiError::message_not_found(id.as_uuid()),
    }
}

impl TenantScoped for MessageResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}

impl Listable for MessageResponse {}

/// Filter for listing messages.
#[derive(Debug, Clone, Default)]
pub struct MessageListFilter {
    /// Filter by message type
    pub message_type: Option<String>,
    /// Filter by sender agent
    pub from_agent_id: Option<AgentId>,
    /// Filter by recipient agent
    pub to_agent_id: Option<AgentId>,
    /// Filter by recipient agent type
    pub to_agent_type: Option<String>,
    /// Filter by trajectory
    pub trajectory_id: Option<TrajectoryId>,
    /// Filter by priority
    pub priority: Option<String>,
    /// Only return undelivered messages
    pub undelivered_only: Option<bool>,
    /// Only return unacknowledged messages
    pub unacknowledged_only: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<ListMessagesRequest> for MessageListFilter {
    fn from(req: ListMessagesRequest) -> Self {
        Self {
            message_type: req.message_type.map(|t| t.to_string()),
            from_agent_id: req.from_agent_id,
            to_agent_id: req.to_agent_id,
            to_agent_type: req.to_agent_type,
            trajectory_id: req.trajectory_id,
            priority: req.priority.map(|p| p.to_string()),
            undelivered_only: req.undelivered_only,
            unacknowledged_only: req.unacknowledged_only,
            limit: req.limit,
            offset: req.offset,
        }
    }
}

impl ListFilter for MessageListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(message_type) = &self.message_type {
            conditions.push(format!("message_type = ${}", param_idx));
            params.push(SqlParam::String(message_type.clone()));
            param_idx += 1;
        }

        if let Some(from_agent_id) = self.from_agent_id {
            conditions.push(format!("from_agent_id = ${}", param_idx));
            params.push(SqlParam::Uuid(from_agent_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(to_agent_id) = self.to_agent_id {
            conditions.push(format!("to_agent_id = ${}", param_idx));
            params.push(SqlParam::Uuid(to_agent_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(to_agent_type) = &self.to_agent_type {
            conditions.push(format!("to_agent_type = ${}", param_idx));
            params.push(SqlParam::String(to_agent_type.clone()));
            param_idx += 1;
        }

        if let Some(trajectory_id) = self.trajectory_id {
            conditions.push(format!("trajectory_id = ${}", param_idx));
            params.push(SqlParam::Uuid(trajectory_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(priority) = &self.priority {
            conditions.push(format!("priority = ${}", param_idx));
            params.push(SqlParam::String(priority.clone()));
            param_idx += 1;
        }

        if let Some(true) = self.undelivered_only {
            conditions.push("delivered_at IS NULL".to_string());
        }

        if let Some(true) = self.unacknowledged_only {
            conditions.push("acknowledged_at IS NULL".to_string());
        }

        // Note: param_idx may be unused after the last condition

        if conditions.is_empty() {
            (None, params)
        } else {
            (Some(conditions.join(" AND ")), params)
        }
    }

    fn limit(&self) -> i32 {
        self.limit.unwrap_or(100)
    }

    fn offset(&self) -> i32 {
        self.offset.unwrap_or(0)
    }
}
