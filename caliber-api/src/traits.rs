//! Common traits for API types

use caliber_core::{EntityId, Timestamp};

/// A type that has an entity ID.
///
/// This trait can be implemented for all response types that represent
/// stored entities with a unique identifier.
pub trait Entity {
    /// Get the entity's unique identifier.
    fn entity_id(&self) -> EntityId;

    /// Get the tenant ID if this is a multi-tenant entity.
    fn tenant_id(&self) -> Option<EntityId> {
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
