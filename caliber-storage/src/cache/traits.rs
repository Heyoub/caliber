//! Cache backend traits and cacheable entity marker.
//!
//! This module defines the traits that must be implemented by cache backends
//! and entities that can be cached.

use async_trait::async_trait;
use caliber_core::{CaliberResult, EntityType};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

/// Marker trait for types that can be cached.
///
/// Types implementing this trait must provide information about their
/// entity type and identity, which the cache uses for key generation
/// and invalidation.
///
/// # Implementation Requirements
///
/// - `entity_type()` must return a consistent value for all instances
/// - `entity_id()` must return the unique identifier for this instance
/// - `tenant_id()` must return the tenant that owns this entity
/// - Implementations must be `Clone`, `Serialize`, and `DeserializeOwned` for cache storage
/// - Implementations must be `Send + Sync + 'static` for async compatibility
pub trait CacheableEntity: Clone + Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Get the entity type for this cacheable.
    fn entity_type() -> EntityType;

    /// Get the unique identifier for this entity.
    fn entity_id(&self) -> Uuid;

    /// Get the tenant ID that owns this entity.
    fn tenant_id(&self) -> Uuid;
}

/// Cache backend trait for pluggable cache implementations.
///
/// This trait abstracts over different cache backends (e.g., LMDB, Redis,
/// in-memory). Implementations should be thread-safe and support concurrent
/// access.
///
/// # Key Format
///
/// Keys are constructed from entity type, tenant ID, and entity ID. The
/// exact format is implementation-defined but must be consistent.
///
/// # Serialization
///
/// Implementations are responsible for serializing/deserializing values.
/// Consider using a compact binary format for performance.
#[async_trait]
pub trait CacheBackend: Send + Sync {
    /// Get a value from the cache.
    ///
    /// Returns the cached value and when it was cached, or None if not found.
    async fn get<T: CacheableEntity>(
        &self,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<(T, DateTime<Utc>)>>;

    /// Put a value into the cache.
    ///
    /// The `cached_at` timestamp is stored alongside the value to support
    /// staleness calculations.
    async fn put<T: CacheableEntity>(
        &self,
        entity: &T,
        cached_at: DateTime<Utc>,
    ) -> CaliberResult<()>;

    /// Delete a value from the cache.
    ///
    /// This is typically called when an entity is deleted from storage.
    async fn delete<T: CacheableEntity>(
        &self,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<()>;

    /// Delete a value by key components without needing the entity type parameter.
    ///
    /// This is useful when you know the key but don't have the full type information.
    async fn delete_by_key(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<()>;

    /// Invalidate all cached entries for a tenant.
    ///
    /// This is a bulk invalidation operation, typically used when:
    /// - A tenant's data is being reset
    /// - Cache corruption is suspected
    /// - During tenant migration
    async fn invalidate_tenant(&self, tenant_id: Uuid) -> CaliberResult<u64>;

    /// Invalidate all cached entries of a specific entity type for a tenant.
    ///
    /// More targeted than `invalidate_tenant`, useful when a bulk mutation
    /// affects all entities of a type.
    async fn invalidate_entity_type(
        &self,
        tenant_id: Uuid,
        entity_type: EntityType,
    ) -> CaliberResult<u64>;

    /// Get cache statistics.
    async fn stats(&self) -> CaliberResult<CacheStats>;
}

/// Statistics about cache usage.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of entries currently in cache.
    pub entry_count: u64,
    /// Approximate memory usage in bytes.
    pub memory_bytes: u64,
    /// Number of evictions due to capacity.
    pub evictions: u64,
}

impl CacheStats {
    /// Calculate the hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

// ============================================================================
// IMPLEMENTATIONS FOR CALIBER ENTITIES
// ============================================================================

use caliber_core::{Artifact, EntityIdType, Note, Scope, Trajectory, Turn};

impl CacheableEntity for Trajectory {
    fn entity_type() -> EntityType {
        EntityType::Trajectory
    }

    fn entity_id(&self) -> Uuid {
        self.trajectory_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        // Trajectories use their own ID as tenant_id for now
        // This will be updated when multi-tenancy is fully implemented
        self.trajectory_id.as_uuid()
    }
}

impl CacheableEntity for Scope {
    fn entity_type() -> EntityType {
        EntityType::Scope
    }

    fn entity_id(&self) -> Uuid {
        self.scope_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        // Scopes belong to trajectories
        self.trajectory_id.as_uuid()
    }
}

impl CacheableEntity for Artifact {
    fn entity_type() -> EntityType {
        EntityType::Artifact
    }

    fn entity_id(&self) -> Uuid {
        self.artifact_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        self.trajectory_id.as_uuid()
    }
}

impl CacheableEntity for Note {
    fn entity_type() -> EntityType {
        EntityType::Note
    }

    fn entity_id(&self) -> Uuid {
        self.note_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        // Notes can span multiple trajectories, use first source trajectory
        self.source_trajectory_ids
            .first()
            .map(|id| id.as_uuid())
            .unwrap_or_else(|| self.note_id.as_uuid())
    }
}

impl CacheableEntity for Turn {
    fn entity_type() -> EntityType {
        EntityType::Turn
    }

    fn entity_id(&self) -> Uuid {
        self.turn_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        // Turns belong to scopes, use scope_id as proxy
        self.scope_id.as_uuid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            ..Default::default()
        };
        assert!((stats.hit_rate() - 0.8).abs() < 0.001);

        let empty_stats = CacheStats::default();
        assert!((empty_stats.hit_rate() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cacheable_entity_types() {
        assert_eq!(Trajectory::entity_type(), EntityType::Trajectory);
        assert_eq!(Scope::entity_type(), EntityType::Scope);
        assert_eq!(Artifact::entity_type(), EntityType::Artifact);
        assert_eq!(Note::entity_type(), EntityType::Note);
        assert_eq!(Turn::entity_type(), EntityType::Turn);
    }
}
