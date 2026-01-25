//! Declarative macros for generating pg_extern CRUD functions.
//!
//! These macros reduce boilerplate by generating the thin pg_extern wrappers
//! that call into the heap operation modules. The heap modules remain hand-written
//! as they contain entity-specific low-level logic.
//!
//! # Design Philosophy
//!
//! The macros handle:
//! - UUID conversion (pgrx::Uuid <-> uuid::Uuid)
//! - Error handling with pgrx::warning!
//! - Consistent function signatures
//!
//! Entity-specific JSON building remains inline because each entity has unique
//! enum variants and field structures that don't generalize well.
//!
//! # Usage
//!
//! ```ignore
//! caliber_pg_get!(trajectory, trajectory_heap, |row| {
//!     let t = row.trajectory;
//!     serde_json::json!({
//!         "trajectory_id": t.trajectory_id.to_string(),
//!         // ... entity-specific fields
//!     })
//! });
//! ```
//!
//! # Savings
//!
//! Each macro invocation saves ~8 lines of boilerplate:
//! - 2 lines: UUID conversion
//! - 4 lines: match arms structure
//! - 2 lines: error handling

/// Generate a `caliber_{entity}_get` function that wraps heap operations.
///
/// Takes entity name, heap module, entity ID type, and a closure that builds JSON from the row.
/// The closure receives the full row (with tenant_id) and returns serde_json::Value.
#[macro_export]
macro_rules! caliber_pg_get {
    ($entity:ident, $heap_mod:ident, $id_ty:ty, |$row:ident| $json_expr:expr) => {
        paste::paste! {
            #[pg_extern]
            fn [<caliber_ $entity _get>](
                id: pgrx::Uuid,
                tenant_id: pgrx::Uuid,
            ) -> Option<pgrx::JsonB> {
                let entity_id: $id_ty = crate::id_from_pgrx(id);
                let tenant_uuid = crate::id_from_pgrx::<caliber_core::TenantId>(tenant_id);

                match $heap_mod::[<$entity _get_heap>](entity_id, tenant_uuid) {
                    Ok(Some($row)) => {
                        Some(pgrx::JsonB($json_expr))
                    }
                    Ok(None) => None,
                    Err(e) => {
                        pgrx::warning!("CALIBER: {} get failed: {}", stringify!($entity), e);
                        None
                    }
                }
            }
        }
    };
}

/// Generate a `caliber_{entity}_list_active` function for entities with active/inactive state.
///
/// Takes entity name, heap module, and a closure that builds JSON from each row.
#[macro_export]
macro_rules! caliber_pg_list_active {
    ($entity:ident, $heap_mod:ident, |$row:ident| $json_expr:expr) => {
        paste::paste! {
            #[pg_extern]
            fn [<caliber_ $entity _list_active>](tenant_id: pgrx::Uuid) -> pgrx::JsonB {
                let tenant_uuid = crate::id_from_pgrx::<caliber_core::TenantId>(tenant_id);

                match $heap_mod::[<$entity _list_active_heap>](tenant_uuid) {
                    Ok(rows) => {
                        let items: Vec<serde_json::Value> = rows
                            .into_iter()
                            .map(|$row| $json_expr)
                            .collect();
                        pgrx::JsonB(serde_json::json!(items))
                    }
                    Err(e) => {
                        pgrx::warning!("CALIBER: {} list_active failed: {}", stringify!($entity), e);
                        pgrx::JsonB(serde_json::json!([]))
                    }
                }
            }
        }
    };
}

/// Generate a `caliber_{entity}_delete` function.
#[macro_export]
macro_rules! caliber_pg_delete {
    ($entity:ident, $heap_mod:ident, $id_ty:ty) => {
        paste::paste! {
            #[pg_extern]
            fn [<caliber_ $entity _delete>](
                id: pgrx::Uuid,
                tenant_id: pgrx::Uuid,
            ) -> bool {
                let entity_id: $id_ty = crate::id_from_pgrx(id);
                let tenant_uuid = crate::id_from_pgrx::<caliber_core::TenantId>(tenant_id);

                match $heap_mod::[<$entity _delete_heap>](entity_id, tenant_uuid) {
                    Ok(deleted) => deleted,
                    Err(e) => {
                        pgrx::warning!("CALIBER: {} delete failed: {}", stringify!($entity), e);
                        false
                    }
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    // Macro expansion tests would go here
    // Can't easily test pgrx macros without full extension context
}
