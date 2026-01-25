//! Webhook Request and Response Types
//!
//! Types for managing webhook configurations for event notifications.

use caliber_core::{WebhookId, TenantId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Webhook configuration for event notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct WebhookResponse {
    pub webhook_id: WebhookId,
    pub tenant_id: TenantId,
    pub name: String,
    pub url: String,
    pub events: Vec<String>, // e.g., ["trajectory.created", "artifact.updated"]
    pub is_active: bool,
    pub secret_hash: Option<String>, // Don't return actual secret
    pub retry_policy: WebhookRetryPolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct WebhookRetryPolicy {
    pub max_retries: i32,
    pub retry_interval_seconds: i32,
    pub exponential_backoff: bool,
}

impl Default for WebhookRetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_interval_seconds: 60,
            exponential_backoff: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
    pub retry_policy: Option<WebhookRetryPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateWebhookRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub secret: Option<String>,
    pub is_active: Option<bool>,
    pub retry_policy: Option<WebhookRetryPolicy>,
}

/// Request for listing webhooks with optional filters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListWebhooksRequest {
    pub is_active: Option<bool>,
    pub event: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}
