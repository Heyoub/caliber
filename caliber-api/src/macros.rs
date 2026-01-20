//! Utility macros for reducing boilerplate

/// Macro to implement `FromRef<AppState>` for state extractors.
///
/// This macro reduces boilerplate for types that need to be extracted
/// from AppState in Axum handlers.
///
/// # Example
/// ```ignore
/// impl_from_ref!(DbClient, db);
/// // Expands to:
/// impl axum::extract::FromRef<AppState> for DbClient {
///     fn from_ref(state: &AppState) -> Self {
///         state.db.clone()
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_from_ref {
    ($type:ty, $field:ident) => {
        impl axum::extract::FromRef<$crate::state::AppState> for $type {
            fn from_ref(state: &$crate::state::AppState) -> Self {
                state.$field.clone()
            }
        }
    };
}

/// Macro to implement the `Entity` trait for response types.
///
/// This macro reduces boilerplate for response types that have an entity_id field.
///
/// # Example
/// ```ignore
/// impl_entity!(TrajectoryResponse, trajectory_id);
/// // Expands to:
/// impl crate::traits::Entity for TrajectoryResponse {
///     fn entity_id(&self) -> caliber_core::EntityId {
///         self.trajectory_id
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_entity {
    ($type:ty, $id_field:ident) => {
        impl $crate::traits::Entity for $type {
            fn entity_id(&self) -> caliber_core::EntityId {
                self.$id_field
            }
        }
    };
    ($type:ty, $id_field:ident, $tenant_field:ident) => {
        impl $crate::traits::Entity for $type {
            fn entity_id(&self) -> caliber_core::EntityId {
                self.$id_field
            }

            fn tenant_id(&self) -> Option<caliber_core::EntityId> {
                $crate::traits::normalize_tenant_id(self.$tenant_field)
            }
        }
    };
}
