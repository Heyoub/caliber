//! Database Helper Functions
//!
//! Common database operations that combine multiple steps,
//! reducing boilerplate in route handlers.

use caliber_core::TenantId;

use crate::auth::{validate_tenant_ownership, AuthContext};
use crate::component::{Component, TenantScoped};
use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};

/// Get a resource by ID with tenant ownership validation.
///
/// This helper combines the common pattern of:
/// 1. Fetching a resource by ID
/// 2. Checking if it exists (returning not found error if not)
/// 3. Validating tenant ownership
///
/// # Arguments
/// - `db`: Database client
/// - `id`: ID of the resource to fetch
/// - `auth`: Authentication context for tenant validation
///
/// # Type Parameters
/// - `T`: The response type, must implement `Component` and `TenantScoped`
///
/// # Example
/// ```ignore
/// use caliber_api::db_helpers::get_owned;
///
/// let note = get_owned::<NoteResponse>(&db, note_id, &auth).await?;
/// // note is guaranteed to exist and belong to auth.tenant_id
/// ```
pub async fn get_owned<T>(
    db: &DbClient,
    id: T::Id,
    auth: &AuthContext,
) -> ApiResult<T>
where
    T: Component + TenantScoped,
    T::Id: Copy + std::fmt::Display,
{
    let resource = db
        .get::<T>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found(T::ENTITY_NAME, id))?;

    validate_tenant_ownership(auth, Some(resource.tenant_id()))?;
    Ok(resource)
}

/// Get a resource by ID without tenant ownership validation.
///
/// Use this when you need to fetch a resource but handle ownership
/// validation separately (e.g., for cross-tenant admin operations).
///
/// # Arguments
/// - `db`: Database client
/// - `id`: ID of the resource to fetch
/// - `tenant_id`: Tenant ID for the query
///
/// # Example
/// ```ignore
/// let note = get_or_not_found::<NoteResponse>(&db, note_id, tenant_id).await?;
/// ```
pub async fn get_or_not_found<T>(
    db: &DbClient,
    id: T::Id,
    tenant_id: TenantId,
) -> ApiResult<T>
where
    T: Component + TenantScoped,
    T::Id: Copy + std::fmt::Display,
{
    db.get::<T>(id, tenant_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found(T::ENTITY_NAME, id))
}
