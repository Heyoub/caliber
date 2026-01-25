//! Lock component implementation.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{AcquireLockRequest, ExtendLockRequest, LockResponse};
use caliber_core::{AgentId, LockId, TenantId};
use serde_json::Value as JsonValue;
use uuid::Uuid;

// Implement Component trait for LockResponse
// Note: Lock creation is "acquire" and update is "extend"
impl_component! {
    LockResponse {
        entity_name: "lock",
        pk_field: "lock_id",
        id_type: LockId,
        requires_tenant: true,
        create_type: AcquireLockRequest,
        update_type: ExtendLockRequest,
        filter_type: LockListFilter,
        entity_id: |self| self.lock_id,
        create_params: |req, tenant_id| vec![
            SqlParam::String(req.resource_type.clone()),
            SqlParam::Uuid(req.resource_id),
            SqlParam::Uuid(req.holder_agent_id.as_uuid()),
            SqlParam::Long(req.timeout_ms),
            SqlParam::String(req.mode.clone()),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 6,
        build_updates: |req| {
            let mut updates = serde_json::Map::new();
            // Extend operation - add additional time to the lock
            updates.insert("additional_ms".to_string(), JsonValue::Number(req.additional_ms.into()));
            JsonValue::Object(updates)
        },
        not_found_error: |id| ApiError::lock_not_found(id.as_uuid()),
    }
}

impl TenantScoped for LockResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}

impl Listable for LockResponse {}

/// Filter for listing locks.
#[derive(Debug, Clone, Default)]
pub struct LockListFilter {
    /// Filter by holder agent
    pub holder_agent_id: Option<AgentId>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by resource ID (can lock any entity type)
    pub resource_id: Option<Uuid>,
    /// Filter by lock mode (Exclusive or Shared)
    pub mode: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl ListFilter for LockListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(holder_agent_id) = self.holder_agent_id {
            conditions.push(format!("holder_agent_id = ${}", param_idx));
            params.push(SqlParam::Uuid(holder_agent_id.as_uuid()));
            param_idx += 1;
        }

        if let Some(resource_type) = &self.resource_type {
            conditions.push(format!("resource_type = ${}", param_idx));
            params.push(SqlParam::String(resource_type.clone()));
            param_idx += 1;
        }

        if let Some(resource_id) = self.resource_id {
            conditions.push(format!("resource_id = ${}", param_idx));
            params.push(SqlParam::Uuid(resource_id));
            param_idx += 1;
        }

        if let Some(mode) = &self.mode {
            conditions.push(format!("mode = ${}", param_idx));
            params.push(SqlParam::String(mode.clone()));
            // param_idx += 1; // unused after this
        }

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
