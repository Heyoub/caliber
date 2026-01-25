//! API Key component implementation.

use crate::component::{impl_component, ListFilter, Listable, SqlParam, TenantScoped};
use crate::error::ApiError;
use crate::types::{
    ApiKeyResponse, CreateApiKeyRequest, ListApiKeysRequest, UpdateApiKeyRequest,
};
use caliber_core::{ApiKeyId, TenantId};
use serde_json::Value as JsonValue;

// Implement Component trait for ApiKeyResponse
impl_component! {
    ApiKeyResponse {
        entity_name: "api_key",
        pk_field: "api_key_id",
        id_type: ApiKeyId,
        requires_tenant: true,
        create_type: CreateApiKeyRequest,
        update_type: UpdateApiKeyRequest,
        filter_type: ApiKeyListFilter,
        entity_id: |self| self.api_key_id,
        create_params: |req, tenant_id| vec![
            SqlParam::String(req.name.clone()),
            SqlParam::Json(serde_json::to_value(&req.scopes).unwrap_or(JsonValue::Array(vec![]))),
            SqlParam::OptTimestamp(req.expires_at),
            SqlParam::Uuid(tenant_id.as_uuid()),
        ],
        create_param_count: 4,
        build_updates: |req| {
            let mut updates = serde_json::Map::new();
            if let Some(name) = &req.name {
                updates.insert("name".to_string(), JsonValue::String(name.clone()));
            }
            if let Some(scopes) = &req.scopes {
                updates.insert("scopes".to_string(), serde_json::to_value(scopes).unwrap_or(JsonValue::Array(vec![])));
            }
            if let Some(expires_at) = &req.expires_at {
                updates.insert("expires_at".to_string(), JsonValue::String(expires_at.to_rfc3339()));
            }
            if let Some(is_active) = req.is_active {
                updates.insert("is_active".to_string(), JsonValue::Bool(is_active));
            }
            JsonValue::Object(updates)
        },
        not_found_error: |id| ApiError::api_key_not_found(id),
    }
}

impl TenantScoped for ApiKeyResponse {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}
impl Listable for ApiKeyResponse {}

/// Filter for listing API keys.
#[derive(Debug, Clone, Default)]
pub struct ApiKeyListFilter {
    pub is_active: Option<bool>,
    pub name: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<ListApiKeysRequest> for ApiKeyListFilter {
    fn from(req: ListApiKeysRequest) -> Self {
        Self {
            is_active: req.is_active,
            name: req.name,
            limit: req.limit,
            offset: req.offset,
        }
    }
}

impl ListFilter for ApiKeyListFilter {
    fn build_where(&self, tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut params = vec![SqlParam::Uuid(tenant_id.as_uuid())];
        let mut param_idx = 2;

        if let Some(is_active) = self.is_active {
            conditions.push(format!("is_active = ${}", param_idx));
            params.push(SqlParam::Bool(is_active));
            param_idx += 1;
        }

        if let Some(name) = &self.name {
            conditions.push(format!("name ILIKE ${}", param_idx));
            params.push(SqlParam::String(format!("%{}%", name)));
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
