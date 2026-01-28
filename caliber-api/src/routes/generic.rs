//! Generic CRUD route handlers for Component types.
//!
//! This module provides generic route handlers that work with any type
//! implementing the Component trait. These handlers reduce boilerplate
//! across entity route files.
//!
//! # Usage
//!
//! For entities that don't need custom OpenAPI documentation:
//! ```ignore
//! use super::generic::crud_routes;
//!
//! pub fn create_router() -> Router<AppState> {
//!     crud_routes::<MyResponse>()
//!         .route("/:id/custom", post(custom_handler))
//! }
//! ```
//!
//! For entities with OpenAPI docs, use the handler helpers directly:
//! ```ignore
//! pub async fn create_my_entity(...) -> ApiResult<impl IntoResponse> {
//!     generic::create_handler::<MyResponse>(db, auth, req).await
//! }
//! ```

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::de::DeserializeOwned;

use crate::{
    component::{Component, Listable, TenantScoped},
    db::DbClient,
    error::ApiResult,
    extractors::PathId,
    middleware::AuthExtractor,
    state::AppState,
};

// ============================================================================
// GENERIC HANDLER HELPERS
// ============================================================================

/// Generic create handler - creates an entity and returns it.
///
/// Use this in your route handlers to reduce boilerplate:
/// ```ignore
/// pub async fn create_trajectory(...) -> ApiResult<impl IntoResponse> {
///     // Custom validation here
///     generic::create_handler::<TrajectoryResponse>(db, auth, req).await
/// }
/// ```
pub async fn create_handler<C>(
    db: DbClient,
    auth: crate::auth::AuthContext,
    req: C::Create,
) -> ApiResult<(StatusCode, Json<C>)>
where
    C: Component + TenantScoped,
    C::Create: Send,
{
    let entity = db.create::<C>(&req, auth.tenant_id).await?;
    Ok((StatusCode::CREATED, Json(entity)))
}

/// Generic get handler - retrieves an entity by ID.
pub async fn get_handler<C>(
    db: DbClient,
    auth: crate::auth::AuthContext,
    id: C::Id,
) -> ApiResult<Json<C>>
where
    C: Component + TenantScoped,
{
    let entity = db
        .get::<C>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| C::not_found_error(id))?;
    Ok(Json(entity))
}

/// Generic update handler - updates an entity and returns it.
pub async fn update_handler<C>(
    db: DbClient,
    auth: crate::auth::AuthContext,
    id: C::Id,
    req: C::Update,
) -> ApiResult<Json<C>>
where
    C: Component + TenantScoped,
    C::Update: Send,
{
    let entity = db.update::<C>(id, &req, auth.tenant_id).await?;
    Ok(Json(entity))
}

/// Generic delete handler - deletes an entity.
pub async fn delete_handler<C>(
    db: DbClient,
    auth: crate::auth::AuthContext,
    id: C::Id,
) -> ApiResult<StatusCode>
where
    C: Component + TenantScoped,
{
    db.delete::<C>(id, auth.tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Generic list handler - lists entities with a filter.
///
/// Returns a tuple of (entities, total) for flexibility in response formatting.
pub async fn list_handler<C>(
    db: DbClient,
    auth: crate::auth::AuthContext,
    filter: C::ListFilter,
) -> ApiResult<(Vec<C>, i32)>
where
    C: Component + TenantScoped + Listable,
{
    let entities = db.list::<C>(&filter, auth.tenant_id).await?;
    let total = entities.len() as i32;
    Ok((entities, total))
}

// ============================================================================
// GENERIC ROUTE FACTORY
// ============================================================================

/// Create a router with standard CRUD routes for a Component type.
///
/// This generates routes without OpenAPI documentation. Use for internal
/// or simple entities. For entities requiring OpenAPI docs, create
/// explicit handlers with utoipa annotations.
///
/// # Routes Created
///
/// - `POST /` - Create entity
/// - `GET /` - List entities
/// - `GET /:id` - Get entity by ID
/// - `PATCH /:id` - Update entity
/// - `DELETE /:id` - Delete entity
///
/// # Example
///
/// ```ignore
/// pub fn create_router() -> Router<AppState> {
///     crud_routes::<MyResponse>()
///         .route("/:id/activate", post(activate_handler))
/// }
/// ```
pub fn crud_routes<C>() -> Router<AppState>
where
    C: Component + TenantScoped + Listable + 'static,
    C::Create: DeserializeOwned + Send + 'static,
    C::Update: DeserializeOwned + Send + 'static,
    C::ListFilter: DeserializeOwned + Default + Send + 'static,
{
    Router::new()
        .route("/", post(create_route::<C>).get(list_route::<C>))
        .route(
            "/:id",
            get(get_route::<C>)
                .patch(update_route::<C>)
                .delete(delete_route::<C>),
        )
}

// Internal route handlers for crud_routes factory
async fn create_route<C>(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<C::Create>,
) -> ApiResult<impl IntoResponse>
where
    C: Component + TenantScoped,
    C::Create: Send,
{
    create_handler::<C>(db, auth, req).await
}

async fn get_route<C>(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<C::Id>,
) -> ApiResult<impl IntoResponse>
where
    C: Component + TenantScoped,
{
    get_handler::<C>(db, auth, id).await
}

async fn update_route<C>(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<C::Id>,
    Json(req): Json<C::Update>,
) -> ApiResult<impl IntoResponse>
where
    C: Component + TenantScoped,
    C::Update: Send,
{
    update_handler::<C>(db, auth, id, req).await
}

async fn delete_route<C>(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<C::Id>,
) -> ApiResult<StatusCode>
where
    C: Component + TenantScoped,
{
    delete_handler::<C>(db, auth, id).await
}

async fn list_route<C>(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Query(filter): Query<C::ListFilter>,
) -> ApiResult<impl IntoResponse>
where
    C: Component + TenantScoped + Listable,
{
    let (entities, _total) = list_handler::<C>(db, auth, filter).await?;
    Ok(Json(entities))
}

// ============================================================================
// RESPONSE WRAPPER HELPERS
// ============================================================================

/// Standard list response wrapper.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: i32,
}

impl<T> ListResponse<T> {
    pub fn new(items: Vec<T>) -> Self {
        let total = items.len() as i32;
        Self { items, total }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{ListFilter, SqlParam};
    use crate::error::ApiError;
    use caliber_core::{TenantId, TrajectoryId};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct DummyEntity {
        id: TrajectoryId,
        tenant_id: TenantId,
        name: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct DummyCreate {
        name: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct DummyUpdate {
        name: Option<String>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct DummyFilter {
        limit: Option<i32>,
        offset: Option<i32>,
    }

    impl ListFilter for DummyFilter {
        fn build_where(&self, _tenant_id: TenantId) -> (Option<String>, Vec<SqlParam>) {
            (None, vec![])
        }

        fn limit(&self) -> i32 {
            self.limit.unwrap_or(100)
        }

        fn offset(&self) -> i32 {
            self.offset.unwrap_or(0)
        }
    }

    impl Component for DummyEntity {
        type Id = TrajectoryId;
        type Create = DummyCreate;
        type Update = DummyUpdate;
        type ListFilter = DummyFilter;

        const ENTITY_NAME: &'static str = "dummy";
        const PK_FIELD: &'static str = "dummy_id";

        fn entity_id(&self) -> Self::Id {
            self.id
        }

        fn create_params(req: &Self::Create, _tenant_id: TenantId) -> Vec<SqlParam> {
            vec![SqlParam::String(req.name.clone())]
        }

        fn create_param_count() -> usize {
            1
        }

        fn build_updates(req: &Self::Update) -> serde_json::Value {
            let mut value = serde_json::Map::new();
            if let Some(name) = &req.name {
                value.insert("name".to_string(), serde_json::Value::String(name.clone()));
            }
            serde_json::Value::Object(value)
        }

        fn not_found_error(id: Self::Id) -> ApiError {
            ApiError::entity_not_found("Dummy", id)
        }
    }

    impl TenantScoped for DummyEntity {
        fn tenant_id(&self) -> TenantId {
            self.tenant_id
        }
    }

    impl Listable for DummyEntity {}

    #[test]
    fn test_crud_routes_builds() {
        let _router = crud_routes::<DummyEntity>();
    }

    #[test]
    fn test_dummy_filter_defaults() {
        let filter = DummyFilter::default();
        assert_eq!(filter.limit(), 100);
        assert_eq!(filter.offset(), 0);
    }
}
