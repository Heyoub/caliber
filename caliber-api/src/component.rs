//! ECS Component trait for generic CRUD operations.
//!
//! This module implements the Entity-Component-System pattern for CALIBER,
//! allowing all 14 entity types to share 5 generic CRUD functions instead
//! of 96 copy-paste implementations.
//!
//! # Pattern
//!
//! Each entity implements the `Component` trait, which provides:
//! - Entity name and primary key information
//! - Create/Update request types
//! - Parameter extraction for stored procedures
//! - JSON building for updates
//! - Error factory for not-found errors
//!
//! The `DbClient` then uses these trait methods to implement generic
//! create, get, update, delete, and list operations.

use crate::error::{ApiError, ApiResult};
use caliber_core::EntityId;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;

// ============================================================================
// COMPONENT TRAIT
// ============================================================================

/// Trait for ECS components that can be persisted via generic CRUD operations.
///
/// Implementations provide metadata and behavior needed for the 5 generic
/// database functions:
/// - `create<C>`: Insert a new entity
/// - `get<C>`: Retrieve by ID
/// - `update<C>`: Modify an existing entity
/// - `delete<C>`: Remove an entity
/// - `list<C>`: Query with filters
///
/// # Type Parameters
///
/// The associated types define the request/response shapes:
/// - `Create`: Request type for creating new entities
/// - `Update`: Request type for updating entities
/// - `ListFilter`: Filter type for list queries
pub trait Component: Sized + Send + Sync + Clone + DeserializeOwned + Serialize {
    /// Request type for creating new entities.
    type Create: Serialize + Send + Sync;

    /// Request type for updating entities.
    type Update: Serialize + Send + Sync;

    /// Filter type for list queries.
    type ListFilter: ListFilter + Default + Send + Sync;

    /// Entity name used in stored procedure names (e.g., "trajectory").
    const ENTITY_NAME: &'static str;

    /// Primary key column name (e.g., "trajectory_id").
    const PK_FIELD: &'static str;

    /// Whether this entity requires tenant isolation.
    const REQUIRES_TENANT: bool = true;

    /// Get the entity ID from this component instance.
    fn entity_id(&self) -> EntityId;

    /// Build the SQL function name for a given operation.
    ///
    /// Default implementation: `caliber_{entity_name}_{operation}`
    fn sql_function(operation: &str) -> String {
        format!("caliber_{}_{}", Self::ENTITY_NAME, operation)
    }

    /// Build parameters for the create stored procedure.
    ///
    /// Returns a vector of parameter values in the order expected by
    /// the stored procedure.
    fn create_params(
        req: &Self::Create,
        tenant_id: EntityId,
    ) -> Vec<SqlParam>;

    /// Get the number of parameters for create (excluding tenant_id).
    fn create_param_count() -> usize;

    /// Build the update JSON from an update request.
    ///
    /// Returns a JSON object with the fields to update.
    fn build_updates(req: &Self::Update) -> JsonValue;

    /// Create a not-found error for this entity type.
    fn not_found_error(id: EntityId) -> ApiError;

    /// Parse a JSON value into this component type.
    ///
    /// Default implementation uses serde_json::from_value.
    fn from_json(json: &JsonValue) -> ApiResult<Self> {
        serde_json::from_value(json.clone())
            .map_err(|e| ApiError::internal_error(format!("Failed to parse {}: {}", Self::ENTITY_NAME, e)))
    }
}

// ============================================================================
// SQL PARAMETER TYPE
// ============================================================================

/// Type-erased SQL parameter for generic CRUD operations.
///
/// This allows building parameter lists without knowing the concrete types
/// at compile time.
#[derive(Debug, Clone)]
pub enum SqlParam {
    /// UUID value
    Uuid(uuid::Uuid),
    /// String value
    String(String),
    /// Optional string value
    OptString(Option<String>),
    /// Integer value
    Int(i32),
    /// Optional integer value
    OptInt(Option<i32>),
    /// Long integer value
    Long(i64),
    /// Optional long value
    OptLong(Option<i64>),
    /// Boolean value
    Bool(bool),
    /// Optional boolean value
    OptBool(Option<bool>),
    /// JSON value
    Json(JsonValue),
    /// Optional JSON value
    OptJson(Option<JsonValue>),
    /// Timestamp (stored as string for PostgreSQL)
    Timestamp(chrono::DateTime<chrono::Utc>),
    /// Optional timestamp
    OptTimestamp(Option<chrono::DateTime<chrono::Utc>>),
    /// Bytes (for BYTEA)
    Bytes(Vec<u8>),
    /// Optional bytes
    OptBytes(Option<Vec<u8>>),
    /// Optional UUID
    OptUuid(Option<uuid::Uuid>),
    /// Float value
    Float(f32),
    /// Optional float value
    OptFloat(Option<f32>),
}

impl SqlParam {
    /// Convert this SqlParam to a reference that can be used with tokio_postgres.
    ///
    /// Returns a trait object reference that implements `ToSql + Sync`.
    pub fn as_to_sql(&self) -> &(dyn tokio_postgres::types::ToSql + Sync) {
        match self {
            SqlParam::Uuid(v) => v,
            SqlParam::String(v) => v,
            SqlParam::OptString(v) => v,
            SqlParam::Int(v) => v,
            SqlParam::OptInt(v) => v,
            SqlParam::Long(v) => v,
            SqlParam::OptLong(v) => v,
            SqlParam::Bool(v) => v,
            SqlParam::OptBool(v) => v,
            SqlParam::Json(v) => v,
            SqlParam::OptJson(v) => v,
            SqlParam::Timestamp(v) => v,
            SqlParam::OptTimestamp(v) => v,
            SqlParam::Bytes(v) => v,
            SqlParam::OptBytes(v) => v,
            SqlParam::OptUuid(v) => v,
            SqlParam::Float(v) => v,
            SqlParam::OptFloat(v) => v,
        }
    }
}

// ============================================================================
// LIST FILTER TRAIT
// ============================================================================

/// Trait for list query filters.
///
/// Implementations provide SQL WHERE clause generation for filtered queries.
pub trait ListFilter {
    /// Build the WHERE clause and parameters for this filter.
    ///
    /// Returns a tuple of (where_clause, parameters).
    /// The where_clause should NOT include the "WHERE" keyword - just the conditions.
    /// Returns None if no filtering is needed.
    fn build_where(&self, tenant_id: EntityId) -> (Option<String>, Vec<SqlParam>);

    /// Get the limit for this query (default: 100).
    fn limit(&self) -> i32 {
        100
    }

    /// Get the offset for this query (default: 0).
    fn offset(&self) -> i32 {
        0
    }
}

/// Empty filter that returns all entities (with tenant isolation).
#[derive(Debug, Clone, Default)]
pub struct NoFilter {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl ListFilter for NoFilter {
    fn build_where(&self, _tenant_id: EntityId) -> (Option<String>, Vec<SqlParam>) {
        (None, vec![])
    }

    fn limit(&self) -> i32 {
        self.limit.unwrap_or(100)
    }

    fn offset(&self) -> i32 {
        self.offset.unwrap_or(0)
    }
}

// ============================================================================
// TENANT SCOPED MARKER TRAIT
// ============================================================================

/// Marker trait for components that require tenant isolation.
///
/// Most components are tenant-scoped. This trait ensures that
/// tenant_id is always passed to the database operations.
pub trait TenantScoped: Component {}

// ============================================================================
// LISTABLE MARKER TRAIT
// ============================================================================

/// Marker trait for components that support list operations.
///
/// Some components may not support listing (e.g., singleton entities).
pub trait Listable: Component {}

// ============================================================================
// HELPER MACROS
// ============================================================================

/// Macro to implement the Component trait for an entity type.
///
/// # Example
///
/// ```ignore
/// impl_component! {
///     TrajectoryResponse {
///         entity_name: "trajectory",
///         pk_field: "trajectory_id",
///         requires_tenant: true,
///         create_type: CreateTrajectoryRequest,
///         update_type: UpdateTrajectoryRequest,
///         filter_type: TrajectoryListFilter,
///         entity_id: |self| self.trajectory_id,
///         create_params: |req, tenant_id| vec![
///             SqlParam::String(req.name.clone()),
///             SqlParam::OptString(req.description.clone()),
///             SqlParam::OptUuid(req.agent_id),
///         ],
///         create_param_count: 3,
///         build_updates: |req| {
///             let mut updates = serde_json::Map::new();
///             if let Some(name) = &req.name {
///                 updates.insert("name".to_string(), JsonValue::String(name.clone()));
///             }
///             JsonValue::Object(updates)
///         },
///         not_found_error: |id| ApiError::trajectory_not_found(id),
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_component {
    (
        $response_type:ty {
            entity_name: $entity_name:literal,
            pk_field: $pk_field:literal,
            requires_tenant: $requires_tenant:expr,
            create_type: $create_type:ty,
            update_type: $update_type:ty,
            filter_type: $filter_type:ty,
            entity_id: |$self_id:ident| $entity_id_expr:expr,
            create_params: |$req:ident, $tenant_id:ident| $create_params_expr:expr,
            create_param_count: $create_param_count:expr,
            build_updates: |$update_req:ident| $build_updates_expr:expr,
            not_found_error: |$err_id:ident| $not_found_expr:expr,
        }
    ) => {
        impl $crate::component::Component for $response_type {
            type Create = $create_type;
            type Update = $update_type;
            type ListFilter = $filter_type;

            const ENTITY_NAME: &'static str = $entity_name;
            const PK_FIELD: &'static str = $pk_field;
            const REQUIRES_TENANT: bool = $requires_tenant;

            fn entity_id(&$self_id) -> caliber_core::EntityId {
                $entity_id_expr
            }

            fn create_params(
                $req: &Self::Create,
                $tenant_id: caliber_core::EntityId,
            ) -> Vec<$crate::component::SqlParam> {
                $create_params_expr
            }

            fn create_param_count() -> usize {
                $create_param_count
            }

            fn build_updates($update_req: &Self::Update) -> serde_json::Value {
                $build_updates_expr
            }

            fn not_found_error($err_id: caliber_core::EntityId) -> $crate::error::ApiError {
                $not_found_expr
            }
        }
    };
}

// Re-export the macro at module level
pub use impl_component;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_filter_defaults() {
        let filter = NoFilter::default();
        assert_eq!(filter.limit(), 100);
        assert_eq!(filter.offset(), 0);
    }

    #[test]
    fn test_sql_param_debug() {
        let param = SqlParam::String("test".to_string());
        assert!(format!("{:?}", param).contains("test"));
    }
}
