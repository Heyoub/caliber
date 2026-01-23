//! Read-through cache with correctness contracts.
//!
//! This module implements the core caching logic, routing reads based on
//! freshness requirements and using the change journal for invalidation.

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use caliber_core::{CaliberResult, EntityId, EntityType};
use chrono::Utc;

use super::freshness::{CacheRead, Freshness};
use super::traits::{CacheBackend, CacheableEntity};
use super::watermark::{ChangeJournal, Watermark};

/// Configuration for the read-through cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum staleness for best-effort reads when not specified.
    pub default_max_staleness: Duration,
    /// How often to poll the change journal for updates.
    pub journal_poll_interval: Duration,
    /// Whether to prefetch related entities on cache miss.
    pub prefetch_enabled: bool,
    /// Maximum number of entries to cache per tenant.
    pub max_entries_per_tenant: usize,
    /// TTL for cached entries (even if not stale by watermark).
    pub entry_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_max_staleness: Duration::from_secs(60),
            journal_poll_interval: Duration::from_millis(100),
            prefetch_enabled: false,
            max_entries_per_tenant: 10_000,
            entry_ttl: Duration::from_secs(3600), // 1 hour
        }
    }
}

impl CacheConfig {
    /// Create a new cache config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default max staleness.
    pub fn with_max_staleness(mut self, duration: Duration) -> Self {
        self.default_max_staleness = duration;
        self
    }

    /// Set the journal poll interval.
    pub fn with_poll_interval(mut self, duration: Duration) -> Self {
        self.journal_poll_interval = duration;
        self
    }

    /// Enable or disable prefetching.
    pub fn with_prefetch(mut self, enabled: bool) -> Self {
        self.prefetch_enabled = enabled;
        self
    }

    /// Set the max entries per tenant.
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries_per_tenant = max;
        self
    }

    /// Set the entry TTL.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.entry_ttl = ttl;
        self
    }
}

/// Storage fetcher trait for retrieving entities from the underlying storage.
///
/// This trait abstracts over the actual storage implementation, allowing
/// the cache to work with any storage backend.
#[async_trait]
pub trait StorageFetcher<T: CacheableEntity>: Send + Sync {
    /// Fetch an entity from storage by ID.
    async fn fetch(&self, entity_id: EntityId, tenant_id: EntityId) -> CaliberResult<Option<T>>;
}

/// Read-through cache with correctness contracts.
///
/// This cache ensures callers explicitly specify their freshness requirements
/// and provides staleness metadata with all reads.
///
/// # Type Parameters
///
/// - `S`: The storage fetcher for retrieving entities on cache miss
/// - `C`: The cache backend for storing cached entities
/// - `J`: The change journal for invalidation
///
/// # Example
///
/// ```ignore
/// let cache = ReadThroughCache::new(storage, backend, journal, config);
///
/// // Best-effort read (may be stale)
/// let read = cache.get::<Artifact>(
///     artifact_id,
///     tenant_id,
///     Freshness::BestEffort { max_staleness: Duration::from_secs(60) },
/// ).await?;
///
/// // Consistent read (checks journal)
/// let read = cache.get::<Artifact>(
///     artifact_id,
///     tenant_id,
///     Freshness::Consistent,
/// ).await?;
/// ```
pub struct ReadThroughCache<C, J>
where
    C: CacheBackend,
    J: ChangeJournal,
{
    /// The cache backend.
    cache: Arc<C>,
    /// The change journal for invalidation.
    journal: Arc<J>,
    /// Cache configuration.
    config: CacheConfig,
}

impl<C, J> ReadThroughCache<C, J>
where
    C: CacheBackend,
    J: ChangeJournal,
{
    /// Create a new read-through cache.
    pub fn new(cache: Arc<C>, journal: Arc<J>, config: CacheConfig) -> Self {
        Self {
            cache,
            journal,
            config,
        }
    }

    /// Create a new read-through cache with default configuration.
    pub fn with_defaults(cache: Arc<C>, journal: Arc<J>) -> Self {
        Self::new(cache, journal, CacheConfig::default())
    }

    /// Get the cache configuration.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Get a reference to the cache backend.
    pub fn backend(&self) -> &C {
        &self.cache
    }

    /// Get a reference to the change journal.
    pub fn journal(&self) -> &J {
        &self.journal
    }

    /// Get an entity from the cache, falling back to storage on miss.
    ///
    /// The freshness parameter determines how the cache handles potentially
    /// stale data:
    ///
    /// - `BestEffort`: Returns cached data if not older than max_staleness,
    ///   otherwise fetches from storage.
    /// - `Consistent`: Checks the change journal to see if any mutations have
    ///   occurred since caching, fetching from storage if so.
    ///
    /// # Returns
    ///
    /// Returns a `CacheRead<T>` wrapper that carries staleness metadata,
    /// or `Ok(None)` if the entity doesn't exist in storage.
    pub async fn get<T, S>(
        &self,
        entity_id: EntityId,
        tenant_id: EntityId,
        freshness: Freshness,
        storage: &S,
    ) -> CaliberResult<Option<CacheRead<T>>>
    where
        T: CacheableEntity,
        S: StorageFetcher<T>,
    {
        match freshness {
            Freshness::BestEffort { max_staleness } => {
                self.get_best_effort(entity_id, tenant_id, max_staleness, storage)
                    .await
            }
            Freshness::Consistent => {
                self.get_consistent(entity_id, tenant_id, storage).await
            }
        }
    }

    /// Best-effort read: cache first, refresh if too stale.
    async fn get_best_effort<T, S>(
        &self,
        entity_id: EntityId,
        tenant_id: EntityId,
        max_staleness: Duration,
        storage: &S,
    ) -> CaliberResult<Option<CacheRead<T>>>
    where
        T: CacheableEntity,
        S: StorageFetcher<T>,
    {
        // Try cache first
        if let Some((entity, cached_at)) = self.cache.get::<T>(entity_id, tenant_id).await? {
            let staleness = Utc::now()
                .signed_duration_since(cached_at)
                .to_std()
                .unwrap_or(Duration::ZERO);

            if staleness <= max_staleness {
                // Cache hit and fresh enough
                return Ok(Some(CacheRead::from_cache(entity, cached_at, None)));
            }
            // Cache hit but too stale, fall through to storage
        }

        // Cache miss or too stale, fetch from storage
        self.fetch_and_cache(entity_id, tenant_id, storage).await
    }

    /// Consistent read: check watermark, fallback to storage if stale.
    async fn get_consistent<T, S>(
        &self,
        entity_id: EntityId,
        tenant_id: EntityId,
        storage: &S,
    ) -> CaliberResult<Option<CacheRead<T>>>
    where
        T: CacheableEntity,
        S: StorageFetcher<T>,
    {
        // Get current watermark
        let current_watermark = self.journal.current_watermark(tenant_id).await?;

        // Try cache
        if let Some((entity, cached_at)) = self.cache.get::<T>(entity_id, tenant_id).await? {
            // Get watermark at cache time
            if let Some(cache_watermark) = self.journal.watermark_at(tenant_id, cached_at).await? {
                // Check if any changes have occurred since caching
                let has_changes = self
                    .journal
                    .changes_since(tenant_id, &cache_watermark, &[T::entity_type()])
                    .await?;

                if !has_changes {
                    // No changes, cache is valid
                    return Ok(Some(CacheRead::from_cache(
                        entity,
                        cached_at,
                        Some(cache_watermark),
                    )));
                }
            }
            // Changes detected or watermark unavailable, fall through to storage
        }

        // Cache miss or stale, fetch from storage
        self.fetch_and_cache_with_watermark(entity_id, tenant_id, storage, current_watermark)
            .await
    }

    /// Fetch from storage and update cache.
    async fn fetch_and_cache<T, S>(
        &self,
        entity_id: EntityId,
        tenant_id: EntityId,
        storage: &S,
    ) -> CaliberResult<Option<CacheRead<T>>>
    where
        T: CacheableEntity,
        S: StorageFetcher<T>,
    {
        let watermark = self.journal.current_watermark(tenant_id).await?;
        self.fetch_and_cache_with_watermark(entity_id, tenant_id, storage, watermark)
            .await
    }

    /// Fetch from storage and update cache with known watermark.
    async fn fetch_and_cache_with_watermark<T, S>(
        &self,
        entity_id: EntityId,
        tenant_id: EntityId,
        storage: &S,
        watermark: Watermark,
    ) -> CaliberResult<Option<CacheRead<T>>>
    where
        T: CacheableEntity,
        S: StorageFetcher<T>,
    {
        if let Some(entity) = storage.fetch(entity_id, tenant_id).await? {
            let cached_at = Utc::now();
            self.cache.put(&entity, cached_at).await?;
            Ok(Some(CacheRead::from_storage(entity, Some(watermark))))
        } else {
            Ok(None)
        }
    }

    /// Put an entity into the cache.
    ///
    /// This is typically called after a write operation to keep the cache
    /// warm with the latest data.
    pub async fn put<T: CacheableEntity>(&self, entity: &T) -> CaliberResult<()> {
        self.cache.put(entity, Utc::now()).await
    }

    /// Invalidate a single entity.
    pub async fn invalidate<T: CacheableEntity>(
        &self,
        entity_id: EntityId,
        tenant_id: EntityId,
    ) -> CaliberResult<()> {
        self.cache.delete::<T>(entity_id, tenant_id).await
    }

    /// Invalidate all cached entries for a tenant.
    pub async fn invalidate_tenant(&self, tenant_id: EntityId) -> CaliberResult<u64> {
        self.cache.invalidate_tenant(tenant_id).await
    }

    /// Invalidate all cached entries of a specific entity type for a tenant.
    pub async fn invalidate_entity_type(
        &self,
        tenant_id: EntityId,
        entity_type: EntityType,
    ) -> CaliberResult<u64> {
        self.cache.invalidate_entity_type(tenant_id, entity_type).await
    }
}

impl<C, J> Clone for ReadThroughCache<C, J>
where
    C: CacheBackend,
    J: ChangeJournal,
{
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
            journal: Arc::clone(&self.journal),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::watermark::InMemoryChangeJournal;
    use caliber_core::{Artifact, ArtifactType, ExtractionMethod, Provenance, TTL};
    use std::collections::HashMap;
    use std::sync::RwLock;
    use uuid::Uuid;

    // Mock cache backend for testing
    #[derive(Default)]
    struct MockCacheBackend {
        storage: RwLock<HashMap<String, (Vec<u8>, chrono::DateTime<Utc>)>>,
        stats: RwLock<super::super::traits::CacheStats>,
    }

    impl MockCacheBackend {
        fn key(entity_type: EntityType, entity_id: EntityId, tenant_id: EntityId) -> String {
            format!("{:?}:{}:{}", entity_type, tenant_id, entity_id)
        }
    }

    #[async_trait]
    impl CacheBackend for MockCacheBackend {
        async fn get<T: CacheableEntity>(
            &self,
            entity_id: EntityId,
            tenant_id: EntityId,
        ) -> CaliberResult<Option<(T, chrono::DateTime<Utc>)>> {
            // For testing, always return None (cache miss)
            let mut stats = self.stats.write().unwrap();
            stats.misses += 1;
            Ok(None)
        }

        async fn put<T: CacheableEntity>(
            &self,
            _entity: &T,
            _cached_at: chrono::DateTime<Utc>,
        ) -> CaliberResult<()> {
            Ok(())
        }

        async fn delete<T: CacheableEntity>(
            &self,
            _entity_id: EntityId,
            _tenant_id: EntityId,
        ) -> CaliberResult<()> {
            Ok(())
        }

        async fn delete_by_key(
            &self,
            _entity_type: EntityType,
            _entity_id: EntityId,
            _tenant_id: EntityId,
        ) -> CaliberResult<()> {
            Ok(())
        }

        async fn invalidate_tenant(&self, _tenant_id: EntityId) -> CaliberResult<u64> {
            Ok(0)
        }

        async fn invalidate_entity_type(
            &self,
            _tenant_id: EntityId,
            _entity_type: EntityType,
        ) -> CaliberResult<u64> {
            Ok(0)
        }

        async fn stats(&self) -> CaliberResult<super::super::traits::CacheStats> {
            Ok(self.stats.read().unwrap().clone())
        }
    }

    // Mock storage fetcher for testing
    struct MockStorageFetcher {
        artifacts: RwLock<HashMap<EntityId, Artifact>>,
    }

    impl MockStorageFetcher {
        fn new() -> Self {
            Self {
                artifacts: RwLock::new(HashMap::new()),
            }
        }

        fn insert(&self, artifact: Artifact) {
            self.artifacts
                .write()
                .unwrap()
                .insert(artifact.artifact_id, artifact);
        }
    }

    #[async_trait]
    impl StorageFetcher<Artifact> for MockStorageFetcher {
        async fn fetch(
            &self,
            entity_id: EntityId,
            _tenant_id: EntityId,
        ) -> CaliberResult<Option<Artifact>> {
            Ok(self.artifacts.read().unwrap().get(&entity_id).cloned())
        }
    }

    fn make_test_artifact(trajectory_id: EntityId, scope_id: EntityId) -> Artifact {
        Artifact {
            artifact_id: Uuid::now_v7(),
            trajectory_id,
            scope_id,
            artifact_type: ArtifactType::Fact,
            name: "Test Artifact".to_string(),
            content: "test content".to_string(),
            content_hash: [0u8; 32],
            embedding: None,
            provenance: Provenance {
                source_turn: 1,
                extraction_method: ExtractionMethod::Explicit,
                confidence: Some(0.9),
            },
            ttl: TTL::Persistent,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            superseded_by: None,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn test_cache_miss_fetches_from_storage() {
        let cache = Arc::new(MockCacheBackend::default());
        let journal = Arc::new(InMemoryChangeJournal::new());
        let config = CacheConfig::default();

        let read_through = ReadThroughCache::new(cache, journal, config);

        let trajectory_id = Uuid::now_v7();
        let scope_id = Uuid::now_v7();
        let artifact = make_test_artifact(trajectory_id, scope_id);

        let storage = MockStorageFetcher::new();
        storage.insert(artifact.clone());

        let result = read_through
            .get::<Artifact, _>(
                artifact.artifact_id,
                trajectory_id,
                Freshness::BestEffort {
                    max_staleness: Duration::from_secs(60),
                },
                &storage,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let cache_read = result.unwrap();
        assert!(cache_read.was_cache_miss());
        assert_eq!(cache_read.into_value().artifact_id, artifact.artifact_id);
    }

    #[tokio::test]
    async fn test_consistent_read_checks_journal() {
        let cache = Arc::new(MockCacheBackend::default());
        let journal = Arc::new(InMemoryChangeJournal::new());
        let config = CacheConfig::default();

        let read_through = ReadThroughCache::new(cache, journal, config);

        let trajectory_id = Uuid::now_v7();
        let scope_id = Uuid::now_v7();
        let artifact = make_test_artifact(trajectory_id, scope_id);

        let storage = MockStorageFetcher::new();
        storage.insert(artifact.clone());

        let result = read_through
            .get::<Artifact, _>(
                artifact.artifact_id,
                trajectory_id,
                Freshness::Consistent,
                &storage,
            )
            .await
            .unwrap();

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_not_found_returns_none() {
        let cache = Arc::new(MockCacheBackend::default());
        let journal = Arc::new(InMemoryChangeJournal::new());
        let config = CacheConfig::default();

        let read_through = ReadThroughCache::new(cache, journal, config);

        let storage = MockStorageFetcher::new();
        let non_existent_id = Uuid::now_v7();
        let tenant_id = Uuid::now_v7();

        let result = read_through
            .get::<Artifact, _>(
                non_existent_id,
                tenant_id,
                Freshness::Consistent,
                &storage,
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::new()
            .with_max_staleness(Duration::from_secs(120))
            .with_poll_interval(Duration::from_millis(50))
            .with_prefetch(true)
            .with_max_entries(5000)
            .with_ttl(Duration::from_secs(1800));

        assert_eq!(config.default_max_staleness, Duration::from_secs(120));
        assert_eq!(config.journal_poll_interval, Duration::from_millis(50));
        assert!(config.prefetch_enabled);
        assert_eq!(config.max_entries_per_tenant, 5000);
        assert_eq!(config.entry_ttl, Duration::from_secs(1800));
    }
}
