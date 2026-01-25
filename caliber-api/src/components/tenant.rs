//! Tenant component implementation.
//!
//! Tenants are the root entities for multi-tenant isolation.
//! They are NOT tenant-scoped (they ARE the tenant).

use crate::component::{impl_component, ListFilter, Listable, SqlParam};
use crate::error::ApiError;
use crate::types::{CreateTenantRequest, TenantInfo, TenantStatus, UpdateTenantRequest};
use caliber_core::{EntityIdType, TenantId};
use serde_json::Value as JsonValue;

// Implement Component trait for TenantInfo
// Note: Tenant does NOT implement TenantScoped - it IS the tenant
impl_component! {
    TenantInfo {
        entity_name: "tenant",
        pk_field: "tenant_id",
        id_type: TenantId,
        requires_tenant: false,  // Tenant is the root entity, not tenant-scoped
        create_type: CreateTenantRequest,
        update_type: UpdateTenantRequest,
        filter_type: TenantListFilter,
        entity_id: |self| self.tenant_id,
        create_params: |req, _tenant_id| vec![
            SqlParam::String(req.name.clone()),
            SqlParam::OptString(req.domain.clone()),
            SqlParam::OptJson(req.settings.clone()),
        ],
        create_param_count: 3,
        build_updates: |req| {
            let mut updates = serde_json::Map::new();
            if let Some(name) = &req.name {
                updates.insert("name".to_string(), JsonValue::String(name.clone()));
            }
            if let Some(domain) = &req.domain {
                updates.insert("domain".to_string(), JsonValue::String(domain.clone()));
            }
            if let Some(status) = &req.status {
                let status_str = match status {
                    TenantStatus::Active => "active",
                    TenantStatus::Suspended => "suspended",
                    TenantStatus::Archived => "archived",
                };
                updates.insert("status".to_string(), JsonValue::String(status_str.to_string()));
            }
            if let Some(settings) = &req.settings {
                updates.insert("settings".to_string(), settings.clone());
            }
            JsonValue::Object(updates)
        },
        not_found_error: |id| ApiError::tenant_not_found(id.as_uuid()),
    }
}

// Note: TenantInfo does NOT implement TenantScoped - it IS the tenant
impl Listable for TenantInfo {}

/// Filter for listing tenants.
#[derive(Debug, Clone, Default)]
pub struct TenantListFilter {
    /// Filter by tenant status
    pub status: Option<TenantStatus>,
    /// Filter by domain (exact match)
    pub domain: Option<String>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

impl ListFilter for TenantListFilter {
    fn build_where(&self, _tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
        // Tenant is not tenant-scoped, so we don't filter by tenant_id
        let mut conditions = Vec::new();
        let mut params = Vec::new();
        let mut param_idx = 1;

        if let Some(status) = &self.status {
            let status_str = match status {
                TenantStatus::Active => "active",
                TenantStatus::Suspended => "suspended",
                TenantStatus::Archived => "archived",
            };
            conditions.push(format!("status = ${}", param_idx));
            params.push(SqlParam::String(status_str.to_string()));
            param_idx += 1;
        }

        if let Some(domain) = &self.domain {
            conditions.push(format!("domain = ${}", param_idx));
            params.push(SqlParam::String(domain.clone()));
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
