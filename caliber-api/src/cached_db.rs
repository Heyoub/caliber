//! Cached Database Client
//!
//! This module provides a `CachedDbClient` wrapper that transparently integrates
//! caching with database operations. Routes call `cached_db.get()` unchanged,
//! and the cache is used transparently.
//!
//! Part of the Three Dragons architecture:
//! - Dragon 1: LMDB cache for sub-millisecond hot reads
//! - Dragon 2: PostgreSQL for persistent storage
//! - Dragon 3: Change journal for cache invalidation

use async_trait::async_trait;
use caliber_core::{
    CaliberResult, EntityIdType, EntityType, StorageError, TenantId, TrajectoryId, ScopeId,
    ArtifactId, NoteId, AgentId,
};
use uuid::Uuid;
use caliber_storage::{
    CacheableEntity, ChangeJournal, Freshness, InMemoryChangeJournal, LmdbCacheBackend,
    ReadThroughCache, StorageFetcher,
};
use std::sync::Arc;

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::types::*;

/// Type alias for the cache implementation used by CachedDbClient.
///
/// This matches the `ApiCache` type alias in state.rs but is defined here
/// to avoid circular imports. Both must be kept in sync.
pub type CacheImpl = ReadThroughCache<LmdbCacheBackend, InMemoryChangeJournal>;

// ============================================================================
// CACHEABLE ENTITY IMPLEMENTATIONS FOR RESPONSE TYPES
// ============================================================================

impl CacheableEntity for TrajectoryResponse {
    fn entity_type() -> EntityType {
        EntityType::Trajectory
    }

    fn entity_id(&self) -> Uuid {
        self.trajectory_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        self.tenant_id.as_uuid()
    }
}

impl CacheableEntity for ScopeResponse {
    fn entity_type() -> EntityType {
        EntityType::Scope
    }

    fn entity_id(&self) -> Uuid {
        self.scope_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        self.tenant_id.as_uuid()
    }
}

impl CacheableEntity for ArtifactResponse {
    fn entity_type() -> EntityType {
        EntityType::Artifact
    }

    fn entity_id(&self) -> Uuid {
        self.artifact_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        self.tenant_id.as_uuid()
    }
}

impl CacheableEntity for NoteResponse {
    fn entity_type() -> EntityType {
        EntityType::Note
    }

    fn entity_id(&self) -> Uuid {
        self.note_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        self.tenant_id.as_uuid()
    }
}

impl CacheableEntity for AgentResponse {
    fn entity_type() -> EntityType {
        EntityType::Agent
    }

    fn entity_id(&self) -> Uuid {
        self.agent_id.as_uuid()
    }

    fn tenant_id(&self) -> Uuid {
        self.tenant_id.as_uuid()
    }
}

// ============================================================================
// CACHED DATABASE CLIENT
// ============================================================================

/// Cached database client that transparently integrates caching.
///
/// This wrapper provides the same interface as `DbClient` but checks the
/// cache first for read operations, using `Freshness::default()` (Consistent).
/// Write operations pass through to the database and record changes in the
/// change journal for cache invalidation.
///
/// # Usage
///
/// ```ignore
/// // Routes call get() unchanged - cache is transparent
/// let trajectory = cached_db.trajectory_get(id, tenant_id).await?;
///
/// // Write operations automatically invalidate cache
/// let trajectory = cached_db.trajectory_create(&req, tenant_id).await?;
/// ```
#[derive(Clone)]
pub struct CachedDbClient {
    /// The underlying database client.
    db: DbClient,
    /// The read-through cache.
    cache: Arc<CacheImpl>,
}

impl CachedDbClient {
    /// Create a new cached database client.
    pub fn new(db: DbClient, cache: Arc<CacheImpl>) -> Self {
        Self { db, cache }
    }

    /// Get a reference to the underlying database client.
    pub fn db(&self) -> &DbClient {
        &self.db
    }

    /// Get a reference to the cache.
    pub fn cache(&self) -> &CacheImpl {
        &self.cache
    }

    // ========================================================================
    // CACHED TRAJECTORY OPERATIONS
    // ========================================================================

    /// Get a trajectory by ID, checking cache first.
    ///
    /// Uses `Freshness::default()` (Consistent) to ensure read-after-write
    /// consistency by checking the change journal.
    pub async fn trajectory_get(
        &self,
        id: TrajectoryId,
        tenant_id: TenantId,
    ) -> ApiResult<Option<TrajectoryResponse>> {
        // Use cache with consistent freshness
        let fetcher = TrajectoryFetcher { db: &self.db };
        let result = self
            .cache
            .get::<TrajectoryResponse, _>(id.as_uuid(), tenant_id.as_uuid(), Freshness::default(), &fetcher)
            .await
            .map_err(|e| ApiError::internal_error(format!("Cache error: {}", e)))?;

        Ok(result.map(|cache_read| cache_read.into_value()))
    }

    /// Create a trajectory, recording the change in the journal.
    pub async fn trajectory_create(
        &self,
        req: &CreateTrajectoryRequest,
        tenant_id: TenantId,
    ) -> ApiResult<TrajectoryResponse> {
        // Create in database
        let response = self.db.trajectory_create(req, tenant_id).await?;

        // Record change in journal for cache invalidation
        self.cache
            .journal()
            .record_change(tenant_id, EntityType::Trajectory, response.trajectory_id.as_uuid())
            .await
            .map_err(|e| ApiError::internal_error(format!("Journal error: {}", e)))?;

        // Warm the cache with the new entity
        self.cache
            .put(&response)
            .await
            .map_err(|e| ApiError::internal_error(format!("Cache put error: {}", e)))?;

        Ok(response)
    }

    /// Update a trajectory, recording the change in the journal.
    pub async fn trajectory_update(
        &self,
        id: TrajectoryId,
        req: &UpdateTrajectoryRequest,
        tenant_id: TenantId,
    ) -> ApiResult<TrajectoryResponse> {
        // Update in database
        let response = self.db.trajectory_update(id, req, tenant_id).await?;

        // Record change in journal for cache invalidation
        self.cache
            .journal()
            .record_change(tenant_id, EntityType::Trajectory, id.as_uuid())
            .await
            .map_err(|e| ApiError::internal_error(format!("Journal error: {}", e)))?;

        // Update cache with new value
        self.cache
            .put(&response)
            .await
            .map_err(|e| ApiError::internal_error(format!("Cache put error: {}", e)))?;

        Ok(response)
    }

    /// List trajectories by status (not cached, as list operations are complex).
    pub async fn trajectory_list_by_status(
        &self,
        status: caliber_core::TrajectoryStatus,
        tenant_id: TenantId,
    ) -> ApiResult<Vec<TrajectoryResponse>> {
        self.db.trajectory_list_by_status(status, tenant_id).await
    }

    // ========================================================================
    // CACHED SCOPE OPERATIONS
    // ========================================================================

    /// Get a scope by ID, checking cache first.
    pub async fn scope_get(
        &self,
        id: ScopeId,
        tenant_id: TenantId,
    ) -> ApiResult<Option<ScopeResponse>> {
        let fetcher = ScopeFetcher { db: &self.db };
        let result = self
            .cache
            .get::<ScopeResponse, _>(id.as_uuid(), tenant_id.as_uuid(), Freshness::default(), &fetcher)
            .await
            .map_err(|e| ApiError::internal_error(format!("Cache error: {}", e)))?;

        Ok(result.map(|cache_read| cache_read.into_value()))
    }

    // ========================================================================
    // CACHED ARTIFACT OPERATIONS
    // ========================================================================

    /// Get an artifact by ID, checking cache first.
    pub async fn artifact_get(
        &self,
        id: ArtifactId,
        tenant_id: TenantId,
    ) -> ApiResult<Option<ArtifactResponse>> {
        let fetcher = ArtifactFetcher { db: &self.db };
        let result = self
            .cache
            .get::<ArtifactResponse, _>(id.as_uuid(), tenant_id.as_uuid(), Freshness::default(), &fetcher)
            .await
            .map_err(|e| ApiError::internal_error(format!("Cache error: {}", e)))?;

        Ok(result.map(|cache_read| cache_read.into_value()))
    }

    /// List artifacts by trajectory (not cached).
    pub async fn artifact_list_by_trajectory_and_tenant(
        &self,
        trajectory_id: TrajectoryId,
        tenant_id: TenantId,
    ) -> ApiResult<Vec<ArtifactResponse>> {
        self.db
            .artifact_list_by_trajectory_and_tenant(trajectory_id, tenant_id)
            .await
    }

    /// List recent artifacts (not cached).
    pub async fn artifact_list_recent(
        &self,
        tenant_id: TenantId,
        limit: usize,
    ) -> ApiResult<Vec<ArtifactResponse>> {
        self.db.artifact_list_recent(tenant_id, limit).await
    }

    // ========================================================================
    // CACHED NOTE OPERATIONS
    // ========================================================================

    /// Get a note by ID, checking cache first.
    pub async fn note_get(
        &self,
        id: NoteId,
        tenant_id: TenantId,
    ) -> ApiResult<Option<NoteResponse>> {
        let fetcher = NoteFetcher { db: &self.db };
        let result = self
            .cache
            .get::<NoteResponse, _>(id.as_uuid(), tenant_id.as_uuid(), Freshness::default(), &fetcher)
            .await
            .map_err(|e| ApiError::internal_error(format!("Cache error: {}", e)))?;

        Ok(result.map(|cache_read| cache_read.into_value()))
    }

    /// Search notes by content (not cached).
    pub async fn note_search(&self, query: &str, limit: i32) -> ApiResult<Vec<NoteResponse>> {
        self.db.note_search(query, limit).await
    }

    // ========================================================================
    // PASSTHROUGH AGENT OPERATIONS (agents use different caching strategy)
    // ========================================================================

    /// Get an agent by ID (passthrough for now).
    pub async fn agent_get(&self, id: AgentId) -> ApiResult<Option<AgentResponse>> {
        self.db.agent_get(id).await
    }

    /// Register a new agent.
    pub async fn agent_register(
        &self,
        req: &RegisterAgentRequest,
        tenant_id: TenantId,
    ) -> ApiResult<AgentResponse> {
        self.db.agent_register(req, tenant_id).await
    }

    /// Update an agent.
    pub async fn agent_update(
        &self,
        id: AgentId,
        req: &UpdateAgentRequest,
    ) -> ApiResult<AgentResponse> {
        self.db.agent_update(id, req).await
    }

    /// List agents by type.
    pub async fn agent_list_by_type(&self, agent_type: &str) -> ApiResult<Vec<AgentResponse>> {
        self.db.agent_list_by_type(agent_type).await
    }

    /// List all active agents.
    pub async fn agent_list_active(&self) -> ApiResult<Vec<AgentResponse>> {
        self.db.agent_list_active().await
    }

    /// List all agents.
    pub async fn agent_list_all(&self) -> ApiResult<Vec<AgentResponse>> {
        self.db.agent_list_all().await
    }

    // ========================================================================
    // PASSTHROUGH OPERATIONS (operations that don't benefit from caching)
    // ========================================================================

    /// Get the pool size (passthrough).
    pub fn pool_size(&self) -> usize {
        self.db.pool_size()
    }
}

// ============================================================================
// STORAGE FETCHERS FOR EACH ENTITY TYPE
// ============================================================================

/// Storage fetcher for trajectories.
struct TrajectoryFetcher<'a> {
    db: &'a DbClient,
}

#[async_trait]
impl StorageFetcher<TrajectoryResponse> for TrajectoryFetcher<'_> {
    async fn fetch(
        &self,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<TrajectoryResponse>> {
        self.db
            .trajectory_get(TrajectoryId::new(entity_id), TenantId::new(tenant_id))
            .await
            .map_err(|e| caliber_core::CaliberError::Storage(StorageError::TransactionFailed { reason: e.to_string() }))
    }
}

/// Storage fetcher for scopes.
struct ScopeFetcher<'a> {
    db: &'a DbClient,
}

#[async_trait]
impl StorageFetcher<ScopeResponse> for ScopeFetcher<'_> {
    async fn fetch(
        &self,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<ScopeResponse>> {
        self.db
            .scope_get(ScopeId::new(entity_id), TenantId::new(tenant_id))
            .await
            .map_err(|e| caliber_core::CaliberError::Storage(StorageError::TransactionFailed { reason: e.to_string() }))
    }
}

/// Storage fetcher for artifacts.
struct ArtifactFetcher<'a> {
    db: &'a DbClient,
}

#[async_trait]
impl StorageFetcher<ArtifactResponse> for ArtifactFetcher<'_> {
    async fn fetch(
        &self,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<ArtifactResponse>> {
        self.db
            .artifact_get(ArtifactId::new(entity_id), TenantId::new(tenant_id))
            .await
            .map_err(|e| caliber_core::CaliberError::Storage(StorageError::TransactionFailed { reason: e.to_string() }))
    }
}

/// Storage fetcher for notes.
struct NoteFetcher<'a> {
    db: &'a DbClient,
}

#[async_trait]
impl StorageFetcher<NoteResponse> for NoteFetcher<'_> {
    async fn fetch(
        &self,
        entity_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<NoteResponse>> {
        self.db
            .note_get(NoteId::new(entity_id), TenantId::new(tenant_id))
            .await
            .map_err(|e| caliber_core::CaliberError::Storage(StorageError::TransactionFailed { reason: e.to_string() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trajectory_response_cacheable_entity() {
        let trajectory_id = TrajectoryId::new(Uuid::now_v7());
        let tenant_id = TenantId::new(Uuid::now_v7());

        let trajectory = TrajectoryResponse {
            trajectory_id,
            tenant_id,
            name: "Test".to_string(),
            description: None,
            status: caliber_core::TrajectoryStatus::Active,
            parent_trajectory_id: None,
            root_trajectory_id: None,
            agent_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            completed_at: None,
            outcome: None,
            metadata: None,
            links: None,
        };

        assert_eq!(TrajectoryResponse::entity_type(), EntityType::Trajectory);
        assert_eq!(trajectory.entity_id(), trajectory.trajectory_id.as_uuid());
        assert_eq!(trajectory.tenant_id(), trajectory.tenant_id.as_uuid());
    }

    #[test]
    fn test_artifact_response_cacheable_entity() {
        let artifact_id = ArtifactId::new(Uuid::now_v7());
        let tenant_id = TenantId::new(Uuid::now_v7());
        let trajectory_id = TrajectoryId::new(Uuid::now_v7());
        let scope_id = ScopeId::new(Uuid::now_v7());

        let artifact = ArtifactResponse {
            artifact_id,
            tenant_id,
            trajectory_id,
            scope_id,
            artifact_type: caliber_core::ArtifactType::Fact,
            name: "Test".to_string(),
            content: "content".to_string(),
            content_hash: [0u8; 32],
            embedding: None,
            provenance: ProvenanceResponse {
                source_turn: 1,
                extraction_method: caliber_core::ExtractionMethod::Explicit,
                confidence: None,
            },
            ttl: caliber_core::TTL::Persistent,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            superseded_by: None,
            metadata: None,
            links: None,
        };

        assert_eq!(ArtifactResponse::entity_type(), EntityType::Artifact);
        assert_eq!(artifact.entity_id(), artifact.artifact_id.as_uuid());
        assert_eq!(artifact.tenant_id(), artifact.tenant_id.as_uuid());
    }
}
