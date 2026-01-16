//! Tenant REST API Routes
//!
//! This module implements Axum route handlers for tenant operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    types::{ListTenantsResponse, TenantInfo, TenantStatus},
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for tenant routes.
#[derive(Clone)]
pub struct TenantState {
    pub db: DbClient,
}

impl TenantState {
    pub fn new(db: DbClient) -> Self {
        Self { db }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// GET /api/v1/tenants - List all tenants
#[utoipa::path(
    get,
    path = "/api/v1/tenants",
    tag = "Tenants",
    responses(
        (status = 200, description = "List of tenants", body = ListTenantsResponse),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_tenants(
    State(state): State<Arc<TenantState>>,
) -> ApiResult<impl IntoResponse> {
    // TODO: Implement caliber_tenant_list in caliber-pg
    // This will:
    // 1. Query all tenants the authenticated user has access to
    // 2. Return tenant ID, name, status, and created_at
    // 3. Filter based on user permissions

    // For now, return a placeholder list
    // In a real implementation, this would query the database
    let tenants = vec![
        TenantInfo {
            tenant_id: Uuid::new_v4().into(),
            name: "Default Tenant".to_string(),
            status: TenantStatus::Active,
            created_at: chrono::Utc::now(),
        },
    ];

    let response = ListTenantsResponse { tenants };

    Ok(Json(response))
}

/// GET /api/v1/tenants/{id} - Get tenant by ID
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{id}",
    tag = "Tenants",
    params(
        ("id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 200, description = "Tenant details", body = TenantInfo),
        (status = 404, description = "Tenant not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_tenant(
    State(state): State<Arc<TenantState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // TODO: Implement caliber_tenant_get in caliber-pg
    // This will:
    // 1. Query the tenant by ID
    // 2. Verify the user has access to this tenant
    // 3. Return tenant details

    // For now, return a placeholder tenant
    // In a real implementation, this would query the database
    let tenant = TenantInfo {
        tenant_id: id.into(),
        name: "Default Tenant".to_string(),
        status: TenantStatus::Active,
        created_at: chrono::Utc::now(),
    };

    Ok(Json(tenant))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the tenant routes router.
pub fn create_router(db: DbClient) -> axum::Router {
    let state = Arc::new(TenantState::new(db));

    axum::Router::new()
        .route("/", axum::routing::get(list_tenants))
        .route("/:id", axum::routing::get(get_tenant))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::EntityId;

    #[test]
    fn test_tenant_info_structure() {
        let tenant = TenantInfo {
            tenant_id: Uuid::new_v4().into(),
            name: "Test Tenant".to_string(),
            status: TenantStatus::Active,
            created_at: chrono::Utc::now(),
        };

        assert_eq!(tenant.name, "Test Tenant");
        assert_eq!(tenant.status, TenantStatus::Active);
    }

    #[test]
    fn test_tenant_status_variants() {
        let active = TenantStatus::Active;
        let suspended = TenantStatus::Suspended;
        let archived = TenantStatus::Archived;

        assert_ne!(active, suspended);
        assert_ne!(active, archived);
        assert_ne!(suspended, archived);
    }

    #[test]
    fn test_list_tenants_response_structure() {
        let response = ListTenantsResponse {
            tenants: vec![
                TenantInfo {
                    tenant_id: Uuid::new_v4().into(),
                    name: "Tenant 1".to_string(),
                    status: TenantStatus::Active,
                    created_at: chrono::Utc::now(),
                },
                TenantInfo {
                    tenant_id: Uuid::new_v4().into(),
                    name: "Tenant 2".to_string(),
                    status: TenantStatus::Suspended,
                    created_at: chrono::Utc::now(),
                },
            ],
        };

        assert_eq!(response.tenants.len(), 2);
        assert_eq!(response.tenants[0].name, "Tenant 1");
        assert_eq!(response.tenants[1].name, "Tenant 2");
    }

    #[test]
    fn test_empty_tenant_list() {
        let response = ListTenantsResponse {
            tenants: vec![],
        };

        assert!(response.tenants.is_empty());
    }

    #[test]
    fn test_tenant_status_serialization() {
        // Test that TenantStatus can be serialized/deserialized
        let status = TenantStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: TenantStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status, deserialized);
    }
}
