//! Async storage trait for asynchronous database operations.
//!
//! This trait provides an async alternative to the synchronous `StorageTrait`.
//! It's designed for use with async runtimes like tokio and enables better
//! integration with async web frameworks.

use ::async_trait::async_trait;
use caliber_core::{
    Artifact, CaliberResult, EntityType, Note, NoteType, Scope, Trajectory,
    TrajectoryStatus, Turn,
};
use uuid::Uuid;

/// Async storage trait for database operations.
///
/// This trait defines async methods for all CRUD operations on CALIBER entities.
/// Implementations should ensure proper error handling and transaction support.
#[async_trait]
pub trait AsyncStorageTrait: Send + Sync {
    // ========================================================================
    // TRAJECTORY OPERATIONS
    // ========================================================================

    /// Insert a new trajectory.
    async fn trajectory_insert(&self, t: &Trajectory) -> CaliberResult<()>;

    /// Get a trajectory by ID.
    async fn trajectory_get(
        &self,
        id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<Trajectory>>;

    /// Update a trajectory.
    async fn trajectory_update(&self, t: &Trajectory) -> CaliberResult<()>;

    /// List trajectories by status.
    async fn trajectory_list_by_status(
        &self,
        status: TrajectoryStatus,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<Trajectory>>;

    /// Delete a trajectory.
    async fn trajectory_delete(&self, id: Uuid, tenant_id: Uuid) -> CaliberResult<()>;

    // ========================================================================
    // SCOPE OPERATIONS
    // ========================================================================

    /// Insert a new scope.
    async fn scope_insert(&self, s: &Scope) -> CaliberResult<()>;

    /// Get a scope by ID.
    async fn scope_get(&self, id: Uuid, tenant_id: Uuid) -> CaliberResult<Option<Scope>>;

    /// Get the current (active) scope for a trajectory.
    async fn scope_get_current(
        &self,
        trajectory_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<Scope>>;

    /// List all scopes in a trajectory.
    async fn scope_list_by_trajectory(
        &self,
        trajectory_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<Scope>>;

    /// Update a scope.
    async fn scope_update(&self, s: &Scope) -> CaliberResult<()>;

    // ========================================================================
    // ARTIFACT OPERATIONS
    // ========================================================================

    /// Insert a new artifact.
    async fn artifact_insert(&self, a: &Artifact) -> CaliberResult<()>;

    /// Get an artifact by ID.
    async fn artifact_get(
        &self,
        id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<Artifact>>;

    /// List artifacts by trajectory.
    async fn artifact_list_by_trajectory(
        &self,
        trajectory_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<Artifact>>;

    /// List artifacts by scope.
    async fn artifact_list_by_scope(
        &self,
        scope_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<Artifact>>;

    /// List recent artifacts across all trajectories for a tenant.
    async fn artifact_list_recent(
        &self,
        tenant_id: Uuid,
        limit: i32,
    ) -> CaliberResult<Vec<Artifact>>;

    /// Update an artifact.
    async fn artifact_update(&self, a: &Artifact) -> CaliberResult<()>;

    /// Delete an artifact.
    async fn artifact_delete(&self, id: Uuid, tenant_id: Uuid) -> CaliberResult<()>;

    // ========================================================================
    // NOTE OPERATIONS
    // ========================================================================

    /// Insert a new note.
    async fn note_insert(&self, n: &Note) -> CaliberResult<()>;

    /// Get a note by ID.
    async fn note_get(&self, id: Uuid, tenant_id: Uuid) -> CaliberResult<Option<Note>>;

    /// List notes by trajectory.
    async fn note_list_by_trajectory(
        &self,
        trajectory_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<Note>>;

    /// List notes by type.
    async fn note_list_by_type(
        &self,
        note_type: NoteType,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<Note>>;

    /// List all notes for a tenant with pagination.
    async fn note_list_all_by_tenant(
        &self,
        limit: i32,
        offset: i32,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<Note>>;

    /// Search notes by content (full-text search).
    async fn note_search(
        &self,
        query: &str,
        limit: i32,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<Note>>;

    /// Update a note.
    async fn note_update(&self, n: &Note) -> CaliberResult<()>;

    /// Delete a note.
    async fn note_delete(&self, id: Uuid, tenant_id: Uuid) -> CaliberResult<()>;

    // ========================================================================
    // TURN OPERATIONS
    // ========================================================================

    /// Insert a new turn.
    async fn turn_insert(&self, t: &Turn) -> CaliberResult<()>;

    /// Get a turn by ID.
    async fn turn_get(&self, id: Uuid, tenant_id: Uuid) -> CaliberResult<Option<Turn>>;

    /// List turns by scope.
    async fn turn_list_by_scope(
        &self,
        scope_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<Turn>>;

    /// Get the latest turn in a scope.
    async fn turn_get_latest(
        &self,
        scope_id: Uuid,
        tenant_id: Uuid,
    ) -> CaliberResult<Option<Turn>>;

    // ========================================================================
    // VECTOR SEARCH OPERATIONS
    // ========================================================================

    /// Search for similar vectors (semantic search).
    ///
    /// Returns a list of (entity_id, similarity_score) tuples ordered by similarity.
    async fn vector_search(
        &self,
        query_vector: &[f32],
        entity_type: EntityType,
        limit: i32,
        tenant_id: Uuid,
    ) -> CaliberResult<Vec<(Uuid, f32)>>;

    /// Store an embedding vector for an entity.
    async fn vector_store(
        &self,
        entity_id: Uuid,
        entity_type: EntityType,
        vector: &[f32],
        tenant_id: Uuid,
    ) -> CaliberResult<()>;

    // ========================================================================
    // HEALTH & DIAGNOSTICS
    // ========================================================================

    /// Check if the storage backend is healthy.
    async fn health_check(&self) -> CaliberResult<bool>;

    /// Get storage statistics (counts, sizes, etc.).
    async fn get_statistics(&self, tenant_id: Uuid) -> CaliberResult<StorageStatistics>;
}

/// Storage statistics for diagnostics.
#[derive(Debug, Clone)]
pub struct StorageStatistics {
    pub trajectory_count: i64,
    pub scope_count: i64,
    pub artifact_count: i64,
    pub note_count: i64,
    pub turn_count: i64,
    pub total_size_bytes: Option<i64>,
}
