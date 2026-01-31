//! LMDB-backed cache implementation with tenant isolation.
//!
//! Uses the heed crate (Rust bindings for LMDB) to provide a high-performance,
//! memory-mapped key-value store for caching CALIBER entities.
//!
//! # Tenant Isolation
//!
//! All cache operations use `TenantScopedKey`, ensuring that:
//! - Data for different tenants is stored with different key prefixes
//! - Tenant invalidation only affects that tenant's data
//! - Cross-tenant access is prevented at compile time
//!
//! # Thread Safety
//!
//! LMDB provides ACID transactions. The backend uses:
//! - Read transactions for `get` operations
//! - Write transactions for `put`, `delete`, and `invalidate_tenant`
//! - Statistics are tracked with atomic counters

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use caliber_core::{CaliberResult, EntityType};
use chrono::{DateTime, Utc};
use heed::types::Bytes;
use heed::{Database, Env, EnvOpenOptions};
use uuid::Uuid;

use super::tenant_key::TenantScopedKey;
use super::traits::{CacheBackend, CacheStats, CacheableEntity};

/// Error type for LMDB cache operations.
#[derive(Debug, thiserror::Error)]
pub enum LmdbCacheError {
    /// Failed to open or create the LMDB environment.
    #[error("Failed to open LMDB environment: {0}")]
    EnvOpen(String),

    /// Failed to open the database within the environment.
    #[error("Failed to open database: {0}")]
    DbOpen(String),

    /// Transaction error.
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error.
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convert LmdbCacheError to CaliberError.
impl From<LmdbCacheError> for caliber_core::CaliberError {
    fn from(e: LmdbCacheError) -> Self {
        caliber_core::CaliberError::Storage(caliber_core::StorageError::TransactionFailed {
            reason: e.to_string(),
        })
    }
}

/// Per-tenant statistics tracking.
#[derive(Debug, Default)]
struct TenantStatsInner {
    hits: u64,
    misses: u64,
    entries: u64,
    size_bytes: u64,
}

/// LMDB-backed cache with tenant isolation.
///
/// # Example
///
/// ```ignore
/// use caliber_storage::cache::{LmdbCacheBackend, TenantScopedKey};
/// use caliber_core::EntityType;
/// use uuid::Uuid;
///
/// let backend = LmdbCacheBackend::new("/tmp/cache", 100)?;
///
/// let tenant_id = Uuid::now_v7();
/// let entity_id = Uuid::now_v7();
///
/// // Store and retrieve using the CacheBackend trait
/// backend.put(&my_artifact, Utc::now()).await?;
/// let cached = backend.get::<Artifact>(entity_id, tenant_id).await?;
/// ```
pub struct LmdbCacheBackend {
    /// The LMDB environment.
    env: Env,
    /// The main database (single unnamed database).
    db: Database<Bytes, Bytes>,
    /// Per-tenant statistics.
    tenant_stats: Arc<RwLock<HashMap<Uuid, TenantStatsInner>>>,
    /// Global statistics.
    global_stats: Arc<RwLock<CacheStats>>,
}

impl LmdbCacheBackend {
    /// Create a new LMDB cache backend.
    ///
    /// # Arguments
    ///
    /// * `path` - Directory where LMDB files will be stored
    /// * `max_size_mb` - Maximum size of the database in megabytes
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The directory cannot be created
    /// - LMDB environment cannot be opened
    /// - Database cannot be created
    pub fn new<P: AsRef<Path>>(path: P, max_size_mb: usize) -> Result<Self, LmdbCacheError> {
        // Ensure directory exists
        std::fs::create_dir_all(&path)?;

        // Open LMDB environment
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(max_size_mb * 1024 * 1024)
                .max_dbs(1)
                .open(path.as_ref())
        }
        .map_err(|e| LmdbCacheError::EnvOpen(e.to_string()))?;

        // Open the default database
        let mut wtxn = env
            .write_txn()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        let db: Database<Bytes, Bytes> = env
            .create_database(&mut wtxn, None)
            .map_err(|e| LmdbCacheError::DbOpen(e.to_string()))?;

        wtxn.commit()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        Ok(Self {
            env,
            db,
            tenant_stats: Arc::new(RwLock::new(HashMap::new())),
            global_stats: Arc::new(RwLock::new(CacheStats::default())),
        })
    }

    /// Record a cache hit for a tenant.
    fn record_hit(&self, tenant_id: Uuid) {
        if let Ok(mut stats) = self.tenant_stats.write() {
            stats.entry(tenant_id).or_default().hits += 1;
        }
        if let Ok(mut stats) = self.global_stats.write() {
            stats.hits += 1;
        }
    }

    /// Record a cache miss for a tenant.
    fn record_miss(&self, tenant_id: Uuid) {
        if let Ok(mut stats) = self.tenant_stats.write() {
            stats.entry(tenant_id).or_default().misses += 1;
        }
        if let Ok(mut stats) = self.global_stats.write() {
            stats.misses += 1;
        }
    }

    /// Update entry statistics after a successful put.
    fn update_entry_stats(&self, tenant_id: Uuid, size_bytes: usize, is_new: bool) {
        if let Ok(mut stats) = self.tenant_stats.write() {
            let tenant_stats = stats.entry(tenant_id).or_default();
            if is_new {
                tenant_stats.entries += 1;
            }
            tenant_stats.size_bytes += size_bytes as u64;
        }
        if is_new {
            if let Ok(mut stats) = self.global_stats.write() {
                stats.entry_count += 1;
            }
        }
    }

    /// Get statistics for a specific tenant.
    pub fn tenant_stats(&self, tenant_id: Uuid) -> CacheStats {
        if let Ok(stats) = self.tenant_stats.read() {
            if let Some(tenant_stats) = stats.get(&tenant_id) {
                return CacheStats {
                    hits: tenant_stats.hits,
                    misses: tenant_stats.misses,
                    entry_count: tenant_stats.entries,
                    memory_bytes: tenant_stats.size_bytes,
                    evictions: 0, // LMDB doesn't track evictions directly
                };
            }
        }
        CacheStats::default()
    }

    /// Iterate over keys matching a prefix and collect them.
    fn collect_keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>, LmdbCacheError> {
        let rtxn = self
            .env
            .read_txn()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        let mut keys = Vec::new();
        let iter = self
            .db
            .iter(&rtxn)
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        for result in iter {
            match result {
                Ok((key, _)) => {
                    if key.len() >= prefix.len() && &key[0..prefix.len()] == prefix {
                        keys.push(key.to_vec());
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(keys)
    }
}

#[async_trait]
impl CacheBackend for LmdbCacheBackend {
    async fn get<T: CacheableEntity>(
        &self,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<(T, DateTime<Utc>)>> {
        let key = TenantScopedKey::new(tenant_id, T::entity_type(), entity_id);
        let encoded_key = key.encode();

        let rtxn = self
            .env
            .read_txn()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        match self.db.get(&rtxn, &encoded_key) {
            Ok(Some(bytes)) => {
                // Update hit statistics
                self.record_hit(tenant_id);

                // Format: [timestamp: 8 bytes][json value]
                if bytes.len() < 8 {
                    return Ok(None);
                }

                // Parse timestamp
                let timestamp_bytes: [u8; 8] = bytes[0..8]
                    .try_into()
                    .map_err(|_| LmdbCacheError::Deserialization("Invalid timestamp".into()))?;
                let timestamp_millis = i64::from_le_bytes(timestamp_bytes);
                let cached_at =
                    DateTime::from_timestamp_millis(timestamp_millis).unwrap_or(Utc::now());

                // Parse value
                let value: T = serde_json::from_slice(&bytes[8..])
                    .map_err(|e| LmdbCacheError::Deserialization(e.to_string()))?;

                Ok(Some((value, cached_at)))
            }
            Ok(None) => {
                self.record_miss(tenant_id);
                Ok(None)
            }
            Err(e) => {
                self.record_miss(tenant_id);
                Err(LmdbCacheError::Transaction(e.to_string()).into())
            }
        }
    }

    async fn put<T: CacheableEntity>(
        &self,
        entity: &T,
        cached_at: DateTime<Utc>,
    ) -> CaliberResult<()> {
        let key = TenantScopedKey::new(entity.tenant_id(), T::entity_type(), entity.entity_id());
        let encoded_key = key.encode();

        // Serialize timestamp + value
        let timestamp_bytes = cached_at.timestamp_millis().to_le_bytes();
        let value_bytes =
            serde_json::to_vec(entity).map_err(|e| LmdbCacheError::Serialization(e.to_string()))?;

        let mut full_bytes = Vec::with_capacity(8 + value_bytes.len());
        full_bytes.extend_from_slice(&timestamp_bytes);
        full_bytes.extend_from_slice(&value_bytes);

        // Check if key already exists (for statistics)
        let is_new = {
            let rtxn = self
                .env
                .read_txn()
                .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;
            self.db.get(&rtxn, &encoded_key).ok().flatten().is_none()
        };

        // Write to LMDB
        let mut wtxn = self
            .env
            .write_txn()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        self.db
            .put(&mut wtxn, &encoded_key, &full_bytes)
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        wtxn.commit()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        // Update statistics
        self.update_entry_stats(entity.tenant_id(), full_bytes.len(), is_new);

        Ok(())
    }

    async fn delete<T: CacheableEntity>(
        &self,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<()> {
        self.delete_by_key(T::entity_type(), entity_id, tenant_id)
            .await
    }

    async fn delete_by_key(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<()> {
        let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
        let encoded_key = key.encode();

        let mut wtxn = self
            .env
            .write_txn()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        let deleted = self
            .db
            .delete(&mut wtxn, &encoded_key)
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        wtxn.commit()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        if deleted {
            if let Ok(mut stats) = self.tenant_stats.write() {
                if let Some(tenant_stats) = stats.get_mut(&tenant_id) {
                    tenant_stats.entries = tenant_stats.entries.saturating_sub(1);
                }
            }
            if let Ok(mut stats) = self.global_stats.write() {
                stats.entry_count = stats.entry_count.saturating_sub(1);
            }
        }

        Ok(())
    }

    async fn invalidate_tenant(&self, tenant_id: Uuid) -> CaliberResult<u64> {
        let prefix = TenantScopedKey::tenant_prefix(tenant_id);
        let keys_to_delete = self.collect_keys_with_prefix(&prefix)?;

        let mut wtxn = self
            .env
            .write_txn()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        let mut deleted = 0u64;
        for key in &keys_to_delete {
            if self.db.delete(&mut wtxn, key).unwrap_or(false) {
                deleted += 1;
            }
        }

        wtxn.commit()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        // Reset tenant statistics
        if let Ok(mut stats) = self.tenant_stats.write() {
            stats.remove(&tenant_id);
        }

        // Update global entry count
        if let Ok(mut stats) = self.global_stats.write() {
            stats.entry_count = stats.entry_count.saturating_sub(deleted);
        }

        Ok(deleted)
    }

    async fn invalidate_entity_type(
        &self,
        tenant_id: Uuid,
        entity_type: EntityType,
    ) -> CaliberResult<u64> {
        let prefix = TenantScopedKey::tenant_type_prefix(tenant_id, entity_type);
        let keys_to_delete = self.collect_keys_with_prefix(&prefix)?;

        let mut wtxn = self
            .env
            .write_txn()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        let mut deleted = 0u64;
        for key in &keys_to_delete {
            if self.db.delete(&mut wtxn, key).unwrap_or(false) {
                deleted += 1;
            }
        }

        wtxn.commit()
            .map_err(|e| LmdbCacheError::Transaction(e.to_string()))?;

        // Update tenant statistics
        if let Ok(mut stats) = self.tenant_stats.write() {
            if let Some(tenant_stats) = stats.get_mut(&tenant_id) {
                tenant_stats.entries = tenant_stats.entries.saturating_sub(deleted);
            }
        }

        // Update global entry count
        if let Ok(mut stats) = self.global_stats.write() {
            stats.entry_count = stats.entry_count.saturating_sub(deleted);
        }

        Ok(deleted)
    }

    async fn stats(&self) -> CaliberResult<CacheStats> {
        Ok(self
            .global_stats
            .read()
            .map(|s| s.clone())
            .unwrap_or_default())
    }
}

// Need to implement serde for CacheableEntity types to work with LMDB
// This requires the entities to derive Serialize/Deserialize, which they do

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{
        AbstractionLevel, ArtifactType, ExtractionMethod, NoteType, Provenance, TrajectoryStatus,
        TTL,
    };
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_backend() -> (LmdbCacheBackend, TempDir) {
        let temp_dir = TempDir::new().expect("TempDir creation should succeed");
        let backend =
            LmdbCacheBackend::new(temp_dir.path(), 10).expect("backend creation should succeed");
        (backend, temp_dir)
    }

    fn make_test_trajectory(tenant_id: Uuid) -> caliber_core::Trajectory {
        use caliber_core::{EntityIdType, TrajectoryId};
        caliber_core::Trajectory {
            trajectory_id: TrajectoryId::new(tenant_id), // Using trajectory_id as tenant_id for this entity type
            name: "Test Trajectory".to_string(),
            description: Some("A test trajectory".to_string()),
            status: TrajectoryStatus::Active,
            parent_trajectory_id: None,
            root_trajectory_id: None,
            agent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            outcome: None,
            metadata: None,
        }
    }

    fn make_test_note(trajectory_id: Uuid) -> caliber_core::Note {
        use caliber_core::{EntityIdType, NoteId, TrajectoryId};
        caliber_core::Note {
            note_id: NoteId::now_v7(),
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content: "This is test content".to_string(),
            content_hash: [0u8; 32],
            embedding: None,
            source_trajectory_ids: vec![TrajectoryId::new(trajectory_id)],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
            superseded_by: None,
            metadata: None,
            abstraction_level: AbstractionLevel::Raw,
            source_note_ids: vec![],
        }
    }

    fn make_test_artifact(trajectory_id: Uuid, scope_id: Uuid) -> caliber_core::Artifact {
        use caliber_core::{ArtifactId, EntityIdType, ScopeId, TrajectoryId};
        caliber_core::Artifact {
            artifact_id: ArtifactId::now_v7(),
            trajectory_id: TrajectoryId::new(trajectory_id),
            scope_id: ScopeId::new(scope_id),
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
    async fn test_new_backend() {
        let (backend, _temp_dir) = create_test_backend();
        // Just verify it was created successfully
        drop(backend);
    }

    #[tokio::test]
    async fn test_put_and_get() {
        use caliber_core::EntityIdType;
        let (backend, _temp_dir) = create_test_backend();

        let trajectory_id = Uuid::now_v7();
        let trajectory = make_test_trajectory(trajectory_id);
        let cached_at = Utc::now();

        backend
            .put(&trajectory, cached_at)
            .await
            .expect("put should succeed");

        let cached = backend
            .get::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), trajectory_id)
            .await
            .expect("get should succeed");
        assert!(cached.is_some());

        let (value, retrieved_cached_at) = cached.expect("cached should be Some");
        assert_eq!(value.trajectory_id, trajectory.trajectory_id);
        assert_eq!(value.name, trajectory.name);
        // Timestamps should be close (within 1 second due to millisecond precision)
        assert!((cached_at - retrieved_cached_at).num_seconds().abs() < 1);
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let (backend, _temp_dir) = create_test_backend();

        let tenant_id = Uuid::now_v7();
        let entity_id = Uuid::now_v7();

        let cached = backend
            .get::<caliber_core::Trajectory>(entity_id, tenant_id)
            .await
            .expect("get should succeed");
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        use caliber_core::EntityIdType;
        let (backend, _temp_dir) = create_test_backend();

        let trajectory_id = Uuid::now_v7();
        let trajectory = make_test_trajectory(trajectory_id);

        backend
            .put(&trajectory, Utc::now())
            .await
            .expect("put should succeed");
        assert!(backend
            .get::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), trajectory_id)
            .await
            .expect("get should succeed")
            .is_some());

        backend
            .delete::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), trajectory_id)
            .await
            .expect("delete should succeed");
        assert!(backend
            .get::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), trajectory_id)
            .await
            .expect("get should succeed")
            .is_none());
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        use caliber_core::EntityIdType;
        let (backend, _temp_dir) = create_test_backend();

        let tenant1 = Uuid::now_v7();
        let tenant2 = Uuid::now_v7();

        // Create trajectory for tenant1
        let trajectory = make_test_trajectory(tenant1);
        backend
            .put(&trajectory, Utc::now())
            .await
            .expect("put should succeed");

        // Try to retrieve under tenant2 (same entity_id, different tenant)
        let cached = backend
            .get::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), tenant2)
            .await
            .expect("get should succeed");
        assert!(cached.is_none(), "Tenant2 should not see tenant1's data");

        // Verify tenant1 can still retrieve
        let cached = backend
            .get::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), tenant1)
            .await
            .expect("get should succeed");
        assert!(cached.is_some(), "Tenant1 should still see their data");
    }

    #[tokio::test]
    async fn test_invalidate_tenant() {
        use caliber_core::EntityIdType;
        let (backend, _temp_dir) = create_test_backend();

        let tenant1 = Uuid::now_v7();
        let tenant2 = Uuid::now_v7();
        let scope_id = Uuid::now_v7();

        // Store multiple items under tenant1
        for _ in 0..5 {
            let artifact = make_test_artifact(tenant1, scope_id);
            backend
                .put(&artifact, Utc::now())
                .await
                .expect("put should succeed");
        }

        // Store item under tenant2
        let t2_trajectory = make_test_trajectory(tenant2);
        backend
            .put(&t2_trajectory, Utc::now())
            .await
            .expect("put should succeed");

        // Invalidate tenant1
        let deleted = backend
            .invalidate_tenant(tenant1)
            .await
            .expect("invalidate_tenant should succeed");
        assert_eq!(deleted, 5);

        // Tenant2's data should still exist
        let cached = backend
            .get::<caliber_core::Trajectory>(t2_trajectory.trajectory_id.as_uuid(), tenant2)
            .await
            .expect("get should succeed");
        assert!(cached.is_some(), "Tenant2's data should not be affected");
    }

    #[tokio::test]
    async fn test_invalidate_entity_type() {
        use caliber_core::EntityIdType;
        let (backend, _temp_dir) = create_test_backend();

        let tenant_id = Uuid::now_v7();
        let scope_id = Uuid::now_v7();

        // Store multiple artifacts
        let mut artifact_ids = Vec::new();
        for _ in 0..3 {
            let artifact = make_test_artifact(tenant_id, scope_id);
            artifact_ids.push(artifact.artifact_id.as_uuid());
            backend
                .put(&artifact, Utc::now())
                .await
                .expect("put should succeed");
        }

        // Store a note
        let note = make_test_note(tenant_id);
        backend
            .put(&note, Utc::now())
            .await
            .expect("put should succeed");

        // Invalidate only artifacts
        let deleted = backend
            .invalidate_entity_type(tenant_id, EntityType::Artifact)
            .await
            .expect("invalidate_entity_type should succeed");
        assert_eq!(deleted, 3);

        // Note should still exist
        let cached = backend
            .get::<caliber_core::Note>(note.note_id.as_uuid(), tenant_id)
            .await
            .expect("get should succeed");
        assert!(cached.is_some(), "Note should not be affected");

        // Artifacts should be gone
        for artifact_id in artifact_ids {
            let cached = backend
                .get::<caliber_core::Artifact>(artifact_id, tenant_id)
                .await
                .expect("get should succeed");
            assert!(cached.is_none(), "Artifact should be deleted");
        }
    }

    #[tokio::test]
    async fn test_stats() {
        use caliber_core::EntityIdType;
        let (backend, _temp_dir) = create_test_backend();

        let tenant_id = Uuid::now_v7();
        let trajectory = make_test_trajectory(tenant_id);

        // Miss
        let _ = backend
            .get::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), tenant_id)
            .await;

        // Put
        backend
            .put(&trajectory, Utc::now())
            .await
            .expect("put should succeed");

        // Hit
        let _ = backend
            .get::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), tenant_id)
            .await;
        let _ = backend
            .get::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), tenant_id)
            .await;

        let stats = backend.stats().await.expect("stats should succeed");
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.entry_count, 1);
    }

    #[tokio::test]
    async fn test_tenant_stats() {
        use caliber_core::EntityIdType;
        let (backend, _temp_dir) = create_test_backend();

        let tenant1 = Uuid::now_v7();
        let tenant2 = Uuid::now_v7();

        // Generate hits for tenant1
        let t1 = make_test_trajectory(tenant1);
        backend
            .put(&t1, Utc::now())
            .await
            .expect("put should succeed");
        let _ = backend
            .get::<caliber_core::Trajectory>(t1.trajectory_id.as_uuid(), tenant1)
            .await;
        let _ = backend
            .get::<caliber_core::Trajectory>(t1.trajectory_id.as_uuid(), tenant1)
            .await;
        let _ = backend
            .get::<caliber_core::Trajectory>(t1.trajectory_id.as_uuid(), tenant1)
            .await;

        // Generate misses for tenant2
        let _ = backend
            .get::<caliber_core::Trajectory>(Uuid::now_v7(), tenant2)
            .await;
        let _ = backend
            .get::<caliber_core::Trajectory>(Uuid::now_v7(), tenant2)
            .await;

        // Verify isolation
        let stats1 = backend.tenant_stats(tenant1);
        let stats2 = backend.tenant_stats(tenant2);

        assert_eq!(stats1.hits, 3);
        assert_eq!(stats1.misses, 0);
        assert_eq!(stats2.hits, 0);
        assert_eq!(stats2.misses, 2);
    }

    #[tokio::test]
    async fn test_overwrite() {
        use caliber_core::EntityIdType;
        let (backend, _temp_dir) = create_test_backend();

        let tenant_id = Uuid::now_v7();
        let mut trajectory = make_test_trajectory(tenant_id);

        // Store initial version
        backend
            .put(&trajectory, Utc::now())
            .await
            .expect("put should succeed");

        // Modify and store again
        trajectory.name = "Updated Name".to_string();
        backend
            .put(&trajectory, Utc::now())
            .await
            .expect("put should succeed");

        // Verify the updated version is returned
        let cached = backend
            .get::<caliber_core::Trajectory>(trajectory.trajectory_id.as_uuid(), tenant_id)
            .await
            .expect("get should succeed");
        assert!(cached.is_some());
        assert_eq!(
            cached.expect("cached should be Some").0.name,
            "Updated Name"
        );
    }
}
