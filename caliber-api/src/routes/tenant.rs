//! Tenant REST API Routes
//!
//! This module implements Axum route handlers for tenant operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    state::AppState,
    types::{ListTenantsResponse, TenantInfo},
};

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
    State(db): State<DbClient>,
) -> ApiResult<impl IntoResponse> {
    let tenants = db.tenant_list().await?;
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
    State(db): State<DbClient>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let tenant = db
        .tenant_get(id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found("Tenant", id))?;
    Ok(Json(tenant))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the tenant routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list_tenants))
        .route("/:id", axum::routing::get(get_tenant))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TenantStatus;

    #[test]
    fn test_tenant_info_structure() {
        let tenant = TenantInfo {
            tenant_id: Uuid::new_v4(),
            name: "Test Tenant".to_string(),
            domain: None,
            workos_organization_id: None,
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
                    tenant_id: Uuid::new_v4(),
                    name: "Tenant 1".to_string(),
                    domain: None,
                    workos_organization_id: None,
                    status: TenantStatus::Active,
                    created_at: chrono::Utc::now(),
                },
                TenantInfo {
                    tenant_id: Uuid::new_v4(),
                    name: "Tenant 2".to_string(),
                    domain: None,
                    workos_organization_id: None,
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
    fn test_tenant_status_serialization() -> Result<(), serde_json::Error> {
        // Test that TenantStatus can be serialized/deserialized
        let status = TenantStatus::Active;
        let json = serde_json::to_string(&status)?;
        let deserialized: TenantStatus = serde_json::from_str(&json)?;

        assert_eq!(status, deserialized);
        Ok(())
    }
}
