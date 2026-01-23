//! API Key Request and Response Types
//!
//! Types for managing API keys for programmatic access to the CALIBER API.

use caliber_core::EntityId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// API key for programmatic access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ApiKeyResponse {
    pub api_key_id: EntityId,
    pub tenant_id: EntityId,
    pub name: String,
    pub key_prefix: String, // First 8 chars for identification
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateApiKeyResponse {
    pub api_key: ApiKeyResponse,
    pub key: String, // Full key, only returned once
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateApiKeyRequest {
    pub name: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
}

/// Request for listing API keys with optional filters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListApiKeysRequest {
    pub is_active: Option<bool>,
    pub name: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}
