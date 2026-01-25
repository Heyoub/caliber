//! Common traits for API types

use caliber_core::{TenantId, Timestamp};
use uuid::Uuid;

/// A type that has an entity ID.
///
/// This trait can be implemented for all response types that represent
/// stored entities with a unique identifier.
pub trait Entity {
    /// Get the entity's unique identifier as a raw UUID.
    fn entity_id(&self) -> Uuid;

    /// Get the tenant ID if this is a multi-tenant entity.
    fn tenant_id(&self) -> Option<TenantId> {
        None
    }
}

/// A type with creation and update timestamps.
///
/// This trait can be implemented for response types that track
/// when they were created and last modified.
pub trait HasTimestamps {
    /// Get when this entity was created.
    fn created_at(&self) -> Timestamp;

    /// Get when this entity was last updated (if tracked).
    fn updated_at(&self) -> Option<Timestamp> {
        None
    }
}

/// Normalize tenant IDs to an optional value.
///
/// This allows entity responses to use either `TenantId` or `Option<TenantId>`
/// while still implementing `Entity::tenant_id`.
pub trait IntoTenantId {
    fn into_option(self) -> Option<TenantId>;
}

impl IntoTenantId for TenantId {
    fn into_option(self) -> Option<TenantId> {
        Some(self)
    }
}

impl IntoTenantId for Option<TenantId> {
    fn into_option(self) -> Option<TenantId> {
        self
    }
}

/// Convert a tenant ID field into `Option<TenantId>`.
pub fn normalize_tenant_id<T: IntoTenantId>(value: T) -> Option<TenantId> {
    value.into_option()
}
