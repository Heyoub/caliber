//! Webhook component implementation.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::webhook::{
    CreateWebhookRequest, ListWebhooksRequest, UpdateWebhookRequest, WebhookResponse,
    WebhookRetryPolicy,
};
use caliber_core::EntityId;
use serde_json::Value as JsonValue;

// Implement Component trait for WebhookResponse
impl_component! {
    WebhookResponse {
        entity_name: "webhook",
        pk_field: "webhook_id",
        requires_tenant: true,
        create_type: CreateWebhookRequest,
        update_type: UpdateWebhookRequest,
        filter_type: WebhookListFilter,
        entity_id: |self| self.webhook_id,
        create_params: |req, tenant_id| vec![
            SqlParam::String(req.name.clone()),
            SqlParam::String(req.url.clone()),
            SqlParam::Json(serde_json::to_value(&req.events).unwrap_or(JsonValue::Array(vec![]))),
            SqlParam::OptString(req.secret.clone()),
            SqlParam::Json(serde_json::to_value(&req.retry_policy.clone().unwrap_or_default()).unwrap_or(JsonValue::Object(serde_json::Map::new()))),
            SqlParam::Uuid(tenant_id),
        ],
        create_param_count: 6,
        build_updates: |req| {
            let mut updates = serde_json::Map::new();
            if let Some(name) = &req.name {
                updates.insert("name".to_string(), JsonValue::String(name.clone()));
            }
            if let Some(url) = &req.url {
                updates.insert("url".to_string(), JsonValue::String(url.clone()));
            }
            if let Some(events) = &req.events {
                updates.insert("events".to_string(), serde_json::to_value(events).unwrap_or(JsonValue::Array(vec![])));
            }
            if let Some(secret) = &req.secret {
                // Note: In real implementation, this would be hashed before storage
                updates.insert("secret_hash".to_string(), JsonValue::String(secret.clone()));
            }
            if let Some(is_active) = req.is_active {
                updates.insert("is_active".to_string(), JsonValue::Bool(is_active));
            }
            if let Some(retry_policy) = &req.retry_policy {
                updates.insert("retry_policy".to_string(), serde_json::to_value(retry_policy).unwrap_or(JsonValue::Object(serde_json::Map::new())));
            }
            JsonValue::Object(updates)
        },
        not_found_error: |id| ApiError::webhook_not_found(id),
    }
}

impl TenantScoped for WebhookResponse {}
impl Listable for WebhookResponse {}

/// Filter for listing webhooks.
#[derive(Debug, Clone, Default)]
pub struct WebhookListFilter {
    pub is_active: Option<bool>,
    pub event: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<ListWebhooksRequest> for WebhookListFilter {
    fn from(req: ListWebhooksRequest) -> Self {
        Self {
            is_active: req.is_active,
            event: req.event,
            limit: req.limit,
            offset: req.offset,
        }
    }
}

impl ListFilter for WebhookListFilter {
    fn build_where(&self, tenant_id: EntityId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id)];
        let mut param_idx = 2;

        if let Some(is_active) = self.is_active {
            conditions.push(format!("is_active = ${}", param_idx));
            params.push(SqlParam::Bool(is_active));
            param_idx += 1;
        }

        if let Some(event) = &self.event {
            // Filter webhooks that are subscribed to a specific event
            // Using array containment: events @> ARRAY[$N]::text[]
            conditions.push(format!("events @> ARRAY[${param_idx}]::text[]"));
            params.push(SqlParam::String(event.clone()));
            // param_idx += 1;
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
