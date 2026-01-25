//! CALIBER Storage - Storage Trait and Mock Implementation
//!
//! Defines the storage abstraction layer for CALIBER entities.
//! The actual pgrx implementation lives in caliber-pg.

pub mod async_trait;
pub mod cache;
pub mod event_dag;
pub mod hybrid_dag;

pub use async_trait::{AsyncStorageTrait, StorageStatistics};
pub use event_dag::InMemoryEventDag;

// Re-export HybridDag types for production event storage
pub use hybrid_dag::{
    ColdEventStorage, ColdStorageError, HybridDag, HybridDagError, LmdbCacheStats, LmdbEventCache,
};

// Re-export cache types for API integration
pub use cache::{
    CacheBackend, CacheConfig, CacheRead, CacheStats, CacheableEntity, ChangeJournal, Freshness,
    InMemoryChangeJournal, LmdbCacheBackend, LmdbCacheError, ReadThroughCache, StorageFetcher,
    TenantScopedKey, Watermark,
};

use caliber_core::{
    AbstractionLevel, Artifact, ArtifactType, CaliberError, CaliberResult, Checkpoint, Edge,
    EdgeType, EmbeddingVector, UuidType, EntityType, Note, Scope, StorageError, Trajectory,
    TrajectoryId, ScopeId, ArtifactId, NoteId, TurnId, EdgeId,
    TrajectoryStatus, Turn,
};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// UPDATE TYPES
// ============================================================================

/// Update payload for trajectories.
#[derive(Debug, Clone, Default)]
pub struct TrajectoryUpdate {
    /// New status
    pub status: Option<TrajectoryStatus>,
    /// Updated metadata
    pub metadata: Option<serde_json::Value>,
}

/// Update payload for scopes.
#[derive(Debug, Clone, Default)]
pub struct ScopeUpdate {
    /// Whether scope is active
    pub is_active: Option<bool>,
    /// Closed timestamp
    pub closed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Updated tokens used
    pub tokens_used: Option<i32>,
    /// Updated checkpoint
    pub checkpoint: Option<Checkpoint>,
}

/// Update payload for artifacts.
#[derive(Debug, Clone, Default)]
pub struct ArtifactUpdate {
    /// Updated content
    pub content: Option<String>,
    /// Updated embedding
    pub embedding: Option<EmbeddingVector>,
    /// Superseded by another artifact
    pub superseded_by: Option<Uuid>,
}

/// Update payload for notes.
#[derive(Debug, Clone, Default)]
pub struct NoteUpdate {
    /// Updated content
    pub content: Option<String>,
    /// Updated embedding
    pub embedding: Option<EmbeddingVector>,
    /// Superseded by another note
    pub superseded_by: Option<Uuid>,
}


// ============================================================================
// STORAGE TRAIT (Task 11.1, 11.2)
// ============================================================================

/// Storage trait for CALIBER entities.
/// Implementations provide persistence for trajectories, scopes, artifacts, notes, and turns.
pub trait StorageTrait: Send + Sync {
    // === Trajectory Operations ===

    /// Insert a new trajectory.
    fn trajectory_insert(&self, t: &Trajectory) -> CaliberResult<()>;

    /// Get a trajectory by ID.
    fn trajectory_get(&self, id: Uuid) -> CaliberResult<Option<Trajectory>>;

    /// Update a trajectory.
    fn trajectory_update(&self, id: Uuid, update: TrajectoryUpdate) -> CaliberResult<()>;

    /// List trajectories by status.
    fn trajectory_list_by_status(&self, status: TrajectoryStatus) -> CaliberResult<Vec<Trajectory>>;

    // === Scope Operations ===

    /// Insert a new scope.
    fn scope_insert(&self, s: &Scope) -> CaliberResult<()>;

    /// Get a scope by ID.
    fn scope_get(&self, id: Uuid) -> CaliberResult<Option<Scope>>;

    /// Get the current active scope for a trajectory.
    fn scope_get_current(&self, trajectory_id: Uuid) -> CaliberResult<Option<Scope>>;

    /// Update a scope.
    fn scope_update(&self, id: Uuid, update: ScopeUpdate) -> CaliberResult<()>;

    /// List scopes for a trajectory.
    fn scope_list_by_trajectory(&self, trajectory_id: Uuid) -> CaliberResult<Vec<Scope>>;

    // === Artifact Operations ===

    /// Insert a new artifact.
    fn artifact_insert(&self, a: &Artifact) -> CaliberResult<()>;

    /// Get an artifact by ID.
    fn artifact_get(&self, id: Uuid) -> CaliberResult<Option<Artifact>>;

    /// Query artifacts by type within a trajectory.
    fn artifact_query_by_type(
        &self,
        trajectory_id: Uuid,
        artifact_type: ArtifactType,
    ) -> CaliberResult<Vec<Artifact>>;

    /// Query artifacts by scope.
    fn artifact_query_by_scope(&self, scope_id: Uuid) -> CaliberResult<Vec<Artifact>>;

    /// Update an artifact.
    fn artifact_update(&self, id: Uuid, update: ArtifactUpdate) -> CaliberResult<()>;

    // === Note Operations ===

    /// Insert a new note.
    fn note_insert(&self, n: &Note) -> CaliberResult<()>;

    /// Get a note by ID.
    fn note_get(&self, id: Uuid) -> CaliberResult<Option<Note>>;

    /// Query notes by trajectory.
    fn note_query_by_trajectory(&self, trajectory_id: Uuid) -> CaliberResult<Vec<Note>>;

    /// Update a note.
    fn note_update(&self, id: Uuid, update: NoteUpdate) -> CaliberResult<()>;

    // === Turn Operations ===

    /// Insert a new turn.
    fn turn_insert(&self, t: &Turn) -> CaliberResult<()>;

    /// Get turns by scope.
    fn turn_get_by_scope(&self, scope_id: Uuid) -> CaliberResult<Vec<Turn>>;

    // === Vector Search (Task 11.2) ===

    /// Search for similar vectors.
    /// Returns (entity_id, similarity_score) pairs.
    fn vector_search(
        &self,
        query: &EmbeddingVector,
        limit: i32,
    ) -> CaliberResult<Vec<(Uuid, f32)>>;

    // === Edge Operations (Battle Intel Feature 1) ===

    /// Insert a new graph edge (binary or hyperedge).
    fn edge_insert(&self, e: &Edge) -> CaliberResult<()>;

    /// Get an edge by ID.
    fn edge_get(&self, id: Uuid) -> CaliberResult<Option<Edge>>;

    /// Query edges by type.
    fn edge_query_by_type(&self, edge_type: EdgeType) -> CaliberResult<Vec<Edge>>;

    /// Query edges by trajectory.
    fn edge_query_by_trajectory(&self, trajectory_id: Uuid) -> CaliberResult<Vec<Edge>>;

    /// Query edges involving a specific entity (as any participant).
    fn edge_query_by_participant(&self, entity_id: Uuid) -> CaliberResult<Vec<Edge>>;

    // === Note Abstraction Level Queries (Battle Intel Feature 2) ===

    /// Query notes by abstraction level.
    fn note_query_by_abstraction_level(
        &self,
        level: AbstractionLevel,
    ) -> CaliberResult<Vec<Note>>;

    /// Query notes derived from a source note (derivation chain).
    fn note_query_by_source_note(&self, source_note_id: Uuid) -> CaliberResult<Vec<Note>>;
}


// ============================================================================
// MOCK STORAGE (Task 11.4)
// ============================================================================

/// In-memory mock storage for testing.
#[derive(Debug, Default)]
pub struct MockStorage {
    trajectories: Arc<RwLock<HashMap<Uuid, Trajectory>>>,
    scopes: Arc<RwLock<HashMap<Uuid, Scope>>>,
    artifacts: Arc<RwLock<HashMap<Uuid, Artifact>>>,
    notes: Arc<RwLock<HashMap<Uuid, Note>>>,
    turns: Arc<RwLock<HashMap<Uuid, Turn>>>,
    // Battle Intel Feature 1: Graph edges
    edges: Arc<RwLock<HashMap<Uuid, Edge>>>,
}

impl MockStorage {
    /// Create a new mock storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all stored data.
    pub fn clear(&self) {
        self.trajectories.write().unwrap().clear();
        self.scopes.write().unwrap().clear();
        self.artifacts.write().unwrap().clear();
        self.notes.write().unwrap().clear();
        self.turns.write().unwrap().clear();
        self.edges.write().unwrap().clear();
    }

    /// Get count of stored trajectories.
    pub fn trajectory_count(&self) -> usize {
        self.trajectories.read().unwrap().len()
    }

    /// Get count of stored scopes.
    pub fn scope_count(&self) -> usize {
        self.scopes.read().unwrap().len()
    }

    /// Get count of stored artifacts.
    pub fn artifact_count(&self) -> usize {
        self.artifacts.read().unwrap().len()
    }

    /// Get count of stored notes.
    pub fn note_count(&self) -> usize {
        self.notes.read().unwrap().len()
    }

    /// Get count of stored edges (Battle Intel Feature 1).
    pub fn edge_count(&self) -> usize {
        self.edges.read().unwrap().len()
    }
}

impl StorageTrait for MockStorage {
    // === Trajectory Operations ===

    fn trajectory_insert(&self, t: &Trajectory) -> CaliberResult<()> {
        let mut trajectories = self.trajectories.write().unwrap();
        if trajectories.contains_key(&t.trajectory_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Trajectory,
                reason: "already exists".to_string(),
            }));
        }
        trajectories.insert(t.trajectory_id, t.clone());
        Ok(())
    }

    fn trajectory_get(&self, id: Uuid) -> CaliberResult<Option<Trajectory>> {
        let trajectories = self.trajectories.read().unwrap();
        Ok(trajectories.get(&id).cloned())
    }

    fn trajectory_update(&self, id: Uuid, update: TrajectoryUpdate) -> CaliberResult<()> {
        let mut trajectories = self.trajectories.write().unwrap();
        let trajectory = trajectories.get_mut(&id).ok_or(
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Trajectory,
                id,
            })
        )?;

        if let Some(status) = update.status {
            trajectory.status = status;
        }
        if let Some(metadata) = update.metadata {
            trajectory.metadata = Some(metadata);
        }
        trajectory.updated_at = chrono::Utc::now();

        Ok(())
    }

    fn trajectory_list_by_status(&self, status: TrajectoryStatus) -> CaliberResult<Vec<Trajectory>> {
        let trajectories = self.trajectories.read().unwrap();
        Ok(trajectories
            .values()
            .filter(|t| t.status == status)
            .cloned()
            .collect())
    }

    // === Scope Operations ===

    fn scope_insert(&self, s: &Scope) -> CaliberResult<()> {
        let mut scopes = self.scopes.write().unwrap();
        if scopes.contains_key(&s.scope_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Scope,
                reason: "already exists".to_string(),
            }));
        }
        scopes.insert(s.scope_id, s.clone());
        Ok(())
    }

    fn scope_get(&self, id: Uuid) -> CaliberResult<Option<Scope>> {
        let scopes = self.scopes.read().unwrap();
        Ok(scopes.get(&id).cloned())
    }

    fn scope_get_current(&self, trajectory_id: Uuid) -> CaliberResult<Option<Scope>> {
        let scopes = self.scopes.read().unwrap();
        Ok(scopes
            .values()
            .filter(|s| s.trajectory_id == trajectory_id && s.is_active)
            .max_by_key(|s| s.created_at)
            .cloned())
    }

    fn scope_update(&self, id: Uuid, update: ScopeUpdate) -> CaliberResult<()> {
        let mut scopes = self.scopes.write().unwrap();
        let scope = scopes.get_mut(&id).ok_or(
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Scope,
                id,
            })
        )?;

        if let Some(is_active) = update.is_active {
            scope.is_active = is_active;
        }
        if let Some(closed_at) = update.closed_at {
            scope.closed_at = Some(closed_at);
        }
        if let Some(tokens_used) = update.tokens_used {
            scope.tokens_used = tokens_used;
        }
        if let Some(checkpoint) = update.checkpoint {
            scope.checkpoint = Some(checkpoint);
        }

        Ok(())
    }

    fn scope_list_by_trajectory(&self, trajectory_id: Uuid) -> CaliberResult<Vec<Scope>> {
        let scopes = self.scopes.read().unwrap();
        Ok(scopes
            .values()
            .filter(|s| s.trajectory_id == trajectory_id)
            .cloned()
            .collect())
    }


    // === Artifact Operations ===

    fn artifact_insert(&self, a: &Artifact) -> CaliberResult<()> {
        let mut artifacts = self.artifacts.write().unwrap();
        if artifacts.contains_key(&a.artifact_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Artifact,
                reason: "already exists".to_string(),
            }));
        }
        artifacts.insert(a.artifact_id, a.clone());
        Ok(())
    }

    fn artifact_get(&self, id: Uuid) -> CaliberResult<Option<Artifact>> {
        let artifacts = self.artifacts.read().unwrap();
        Ok(artifacts.get(&id).cloned())
    }

    fn artifact_query_by_type(
        &self,
        trajectory_id: Uuid,
        artifact_type: ArtifactType,
    ) -> CaliberResult<Vec<Artifact>> {
        let artifacts = self.artifacts.read().unwrap();
        Ok(artifacts
            .values()
            .filter(|a| a.trajectory_id == trajectory_id && a.artifact_type == artifact_type)
            .cloned()
            .collect())
    }

    fn artifact_query_by_scope(&self, scope_id: Uuid) -> CaliberResult<Vec<Artifact>> {
        let artifacts = self.artifacts.read().unwrap();
        Ok(artifacts
            .values()
            .filter(|a| a.scope_id == scope_id)
            .cloned()
            .collect())
    }

    fn artifact_update(&self, id: Uuid, update: ArtifactUpdate) -> CaliberResult<()> {
        let mut artifacts = self.artifacts.write().unwrap();
        let artifact = artifacts.get_mut(&id).ok_or(
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Artifact,
                id,
            })
        )?;

        if let Some(content) = update.content {
            artifact.content = content;
            artifact.content_hash = caliber_core::compute_content_hash(artifact.content.as_bytes());
            artifact.updated_at = chrono::Utc::now();
        }
        if let Some(embedding) = update.embedding {
            artifact.embedding = Some(embedding);
        }
        if let Some(superseded_by) = update.superseded_by {
            artifact.superseded_by = Some(superseded_by);
        }

        Ok(())
    }

    // === Note Operations ===

    fn note_insert(&self, n: &Note) -> CaliberResult<()> {
        let mut notes = self.notes.write().unwrap();
        if notes.contains_key(&n.note_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Note,
                reason: "already exists".to_string(),
            }));
        }
        notes.insert(n.note_id, n.clone());
        Ok(())
    }

    fn note_get(&self, id: Uuid) -> CaliberResult<Option<Note>> {
        let notes = self.notes.read().unwrap();
        Ok(notes.get(&id).cloned())
    }

    fn note_query_by_trajectory(&self, trajectory_id: Uuid) -> CaliberResult<Vec<Note>> {
        let notes = self.notes.read().unwrap();
        Ok(notes
            .values()
            .filter(|n| n.source_trajectory_ids.contains(&trajectory_id))
            .cloned()
            .collect())
    }

    fn note_update(&self, id: Uuid, update: NoteUpdate) -> CaliberResult<()> {
        let mut notes = self.notes.write().unwrap();
        let note = notes.get_mut(&id).ok_or(
            CaliberError::Storage(StorageError::NotFound {
                entity_type: EntityType::Note,
                id,
            })
        )?;

        if let Some(content) = update.content {
            note.content = content;
            note.content_hash = caliber_core::compute_content_hash(note.content.as_bytes());
            note.updated_at = chrono::Utc::now();
        }
        if let Some(embedding) = update.embedding {
            note.embedding = Some(embedding);
        }
        if let Some(superseded_by) = update.superseded_by {
            note.superseded_by = Some(superseded_by);
        }

        Ok(())
    }

    // === Turn Operations ===

    fn turn_insert(&self, t: &Turn) -> CaliberResult<()> {
        let mut turns = self.turns.write().unwrap();
        if turns.contains_key(&t.turn_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Turn,
                reason: "already exists".to_string(),
            }));
        }
        turns.insert(t.turn_id, t.clone());
        Ok(())
    }

    fn turn_get_by_scope(&self, scope_id: Uuid) -> CaliberResult<Vec<Turn>> {
        let turns = self.turns.read().unwrap();
        let mut result: Vec<Turn> = turns
            .values()
            .filter(|t| t.scope_id == scope_id)
            .cloned()
            .collect();
        result.sort_by_key(|t| t.sequence);
        Ok(result)
    }

    // === Vector Search ===

    fn vector_search(
        &self,
        query: &EmbeddingVector,
        limit: i32,
    ) -> CaliberResult<Vec<(Uuid, f32)>> {
        let mut results: Vec<(Uuid, f32)> = Vec::new();

        // Search artifacts
        let artifacts = self.artifacts.read().unwrap();
        for artifact in artifacts.values() {
            if let Some(ref embedding) = artifact.embedding {
                if let Ok(similarity) = query.cosine_similarity(embedding) {
                    results.push((artifact.artifact_id, similarity));
                }
            }
        }

        // Search notes
        let notes = self.notes.read().unwrap();
        for note in notes.values() {
            if let Some(ref embedding) = note.embedding {
                if let Ok(similarity) = query.cosine_similarity(embedding) {
                    results.push((note.note_id, similarity));
                }
            }
        }

        // Sort by similarity descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        results.truncate(limit as usize);

        Ok(results)
    }

    // === Edge Operations (Battle Intel Feature 1) ===

    fn edge_insert(&self, e: &Edge) -> CaliberResult<()> {
        let mut edges = self.edges.write().unwrap();
        if edges.contains_key(&e.edge_id) {
            return Err(CaliberError::Storage(StorageError::InsertFailed {
                entity_type: EntityType::Edge,
                reason: format!("Duplicate edge_id: {}", e.edge_id),
            }));
        }
        edges.insert(e.edge_id, e.clone());
        Ok(())
    }

    fn edge_get(&self, id: Uuid) -> CaliberResult<Option<Edge>> {
        let edges = self.edges.read().unwrap();
        Ok(edges.get(&id).cloned())
    }

    fn edge_query_by_type(&self, edge_type: EdgeType) -> CaliberResult<Vec<Edge>> {
        let edges = self.edges.read().unwrap();
        Ok(edges
            .values()
            .filter(|e| e.edge_type == edge_type)
            .cloned()
            .collect())
    }

    fn edge_query_by_trajectory(&self, trajectory_id: Uuid) -> CaliberResult<Vec<Edge>> {
        let edges = self.edges.read().unwrap();
        Ok(edges
            .values()
            .filter(|e| e.trajectory_id == Some(trajectory_id))
            .cloned()
            .collect())
    }

    fn edge_query_by_participant(&self, entity_id: Uuid) -> CaliberResult<Vec<Edge>> {
        let edges = self.edges.read().unwrap();
        Ok(edges
            .values()
            .filter(|e| {
                e.participants
                    .iter()
                    .any(|p| p.entity_ref.id == entity_id)
            })
            .cloned()
            .collect())
    }

    // === Note Abstraction Level Queries (Battle Intel Feature 2) ===

    fn note_query_by_abstraction_level(
        &self,
        level: AbstractionLevel,
    ) -> CaliberResult<Vec<Note>> {
        let notes = self.notes.read().unwrap();
        Ok(notes
            .values()
            .filter(|n| n.abstraction_level == level)
            .cloned()
            .collect())
    }

    fn note_query_by_source_note(&self, source_note_id: Uuid) -> CaliberResult<Vec<Note>> {
        let notes = self.notes.read().unwrap();
        Ok(notes
            .values()
            .filter(|n| n.source_note_ids.contains(&source_note_id))
            .cloned()
            .collect())
    }
}


// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{
        ArtifactType, ExtractionMethod, NoteType, Provenance, TTL, TurnRole,
    };
    use chrono::Utc;
    use uuid::Uuid;

    fn make_test_trajectory() -> Trajectory {
        Trajectory {
            trajectory_id: Uuid::now_v7(),
            name: "Test task".to_string(),
            description: Some("Test description".to_string()),
            status: TrajectoryStatus::Active,
            parent_trajectory_id: None,
            root_trajectory_id: None,
            agent_id: Some(Uuid::now_v7()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            outcome: None,
            metadata: None,
        }
    }

    fn make_test_scope(trajectory_id: Uuid) -> Scope {
        Scope {
            scope_id: Uuid::now_v7(),
            trajectory_id,
            parent_scope_id: None,
            name: "Test Scope".to_string(),
            purpose: Some("Testing".to_string()),
            is_active: true,
            created_at: Utc::now(),
            closed_at: None,
            checkpoint: None,
            token_budget: 8000,
            tokens_used: 0,
            metadata: None,
        }
    }

    fn make_test_artifact(trajectory_id: Uuid, scope_id: Uuid) -> Artifact {
        let content = "Test artifact content";
        Artifact {
            artifact_id: Uuid::now_v7(),
            trajectory_id,
            scope_id,
            artifact_type: ArtifactType::Fact,
            name: "Test Artifact".to_string(),
            content: content.to_string(),
            content_hash: caliber_core::compute_content_hash(content.as_bytes()),
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

    fn make_test_note(trajectory_id: Uuid) -> Note {
        let content = "Test note content";
        Note {
            note_id: Uuid::now_v7(),
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content: content.to_string(),
            content_hash: caliber_core::compute_content_hash(content.as_bytes()),
            embedding: None,
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: Vec::new(),
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

    fn make_test_turn(scope_id: Uuid) -> Turn {
        Turn {
            turn_id: Uuid::now_v7(),
            scope_id,
            sequence: 1,
            role: TurnRole::User,
            content: "Test turn content".to_string(),
            token_count: 10,
            created_at: Utc::now(),
            tool_calls: None,
            tool_results: None,
            metadata: None,
        }
    }

    // ========================================================================
    // Trajectory Tests
    // ========================================================================

    #[test]
    fn test_trajectory_insert_get() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();

        storage.trajectory_insert(&trajectory).unwrap();
        let retrieved = storage.trajectory_get(trajectory.trajectory_id).unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().trajectory_id, trajectory.trajectory_id);
    }

    #[test]
    fn test_trajectory_insert_duplicate() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();

        storage.trajectory_insert(&trajectory).unwrap();
        let result = storage.trajectory_insert(&trajectory);

        assert!(result.is_err());
    }

    #[test]
    fn test_trajectory_update() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();

        storage.trajectory_insert(&trajectory).unwrap();
        storage
            .trajectory_update(
                trajectory.trajectory_id,
                TrajectoryUpdate {
                    status: Some(TrajectoryStatus::Completed),
                    ..Default::default()
                },
            )
            .unwrap();

        let retrieved = storage.trajectory_get(trajectory.trajectory_id).unwrap().unwrap();
        assert_eq!(retrieved.status, TrajectoryStatus::Completed);
    }

    #[test]
    fn test_trajectory_list_by_status() {
        let storage = MockStorage::new();

        let mut t1 = make_test_trajectory();
        t1.status = TrajectoryStatus::Active;
        let mut t2 = make_test_trajectory();
        t2.status = TrajectoryStatus::Completed;
        let mut t3 = make_test_trajectory();
        t3.status = TrajectoryStatus::Active;

        storage.trajectory_insert(&t1).unwrap();
        storage.trajectory_insert(&t2).unwrap();
        storage.trajectory_insert(&t3).unwrap();

        let active = storage.trajectory_list_by_status(TrajectoryStatus::Active).unwrap();
        assert_eq!(active.len(), 2);

        let completed = storage.trajectory_list_by_status(TrajectoryStatus::Completed).unwrap();
        assert_eq!(completed.len(), 1);
    }


    // ========================================================================
    // Scope Tests
    // ========================================================================

    #[test]
    fn test_scope_insert_get() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();
        let scope = make_test_scope(trajectory.trajectory_id);

        storage.trajectory_insert(&trajectory).unwrap();
        storage.scope_insert(&scope).unwrap();

        let retrieved = storage.scope_get(scope.scope_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().scope_id, scope.scope_id);
    }

    #[test]
    fn test_scope_get_current() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();

        let mut scope1 = make_test_scope(trajectory.trajectory_id);
        scope1.is_active = false;
        let scope2 = make_test_scope(trajectory.trajectory_id);

        storage.trajectory_insert(&trajectory).unwrap();
        storage.scope_insert(&scope1).unwrap();
        storage.scope_insert(&scope2).unwrap();

        let current = storage.scope_get_current(trajectory.trajectory_id).unwrap();
        assert!(current.is_some());
        assert!(current.unwrap().is_active);
    }

    // ========================================================================
    // Artifact Tests
    // ========================================================================

    #[test]
    fn test_artifact_insert_get() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();
        let scope = make_test_scope(trajectory.trajectory_id);
        let artifact = make_test_artifact(trajectory.trajectory_id, scope.scope_id);

        storage.trajectory_insert(&trajectory).unwrap();
        storage.scope_insert(&scope).unwrap();
        storage.artifact_insert(&artifact).unwrap();

        let retrieved = storage.artifact_get(artifact.artifact_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().artifact_id, artifact.artifact_id);
    }

    #[test]
    fn test_artifact_query_by_type() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();
        let scope = make_test_scope(trajectory.trajectory_id);

        let mut a1 = make_test_artifact(trajectory.trajectory_id, scope.scope_id);
        a1.artifact_type = ArtifactType::Fact;
        let mut a2 = make_test_artifact(trajectory.trajectory_id, scope.scope_id);
        a2.artifact_type = ArtifactType::CodePatch;
        let mut a3 = make_test_artifact(trajectory.trajectory_id, scope.scope_id);
        a3.artifact_type = ArtifactType::Fact;

        storage.trajectory_insert(&trajectory).unwrap();
        storage.scope_insert(&scope).unwrap();
        storage.artifact_insert(&a1).unwrap();
        storage.artifact_insert(&a2).unwrap();
        storage.artifact_insert(&a3).unwrap();

        let facts = storage
            .artifact_query_by_type(trajectory.trajectory_id, ArtifactType::Fact)
            .unwrap();
        assert_eq!(facts.len(), 2);
    }

    // ========================================================================
    // Note Tests
    // ========================================================================

    #[test]
    fn test_note_insert_get() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();
        let note = make_test_note(trajectory.trajectory_id);

        storage.trajectory_insert(&trajectory).unwrap();
        storage.note_insert(&note).unwrap();

        let retrieved = storage.note_get(note.note_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().note_id, note.note_id);
    }

    #[test]
    fn test_note_query_by_trajectory() {
        let storage = MockStorage::new();
        let t1 = make_test_trajectory();
        let t2 = make_test_trajectory();

        let n1 = make_test_note(t1.trajectory_id);
        let n2 = make_test_note(t2.trajectory_id);

        storage.trajectory_insert(&t1).unwrap();
        storage.trajectory_insert(&t2).unwrap();
        storage.note_insert(&n1).unwrap();
        storage.note_insert(&n2).unwrap();

        let notes = storage.note_query_by_trajectory(t1.trajectory_id).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].note_id, n1.note_id);
    }

    // ========================================================================
    // Turn Tests
    // ========================================================================

    #[test]
    fn test_turn_insert_get_by_scope() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();
        let scope = make_test_scope(trajectory.trajectory_id);

        let mut t1 = make_test_turn(scope.scope_id);
        t1.sequence = 1;
        let mut t2 = make_test_turn(scope.scope_id);
        t2.sequence = 2;

        storage.trajectory_insert(&trajectory).unwrap();
        storage.scope_insert(&scope).unwrap();
        storage.turn_insert(&t1).unwrap();
        storage.turn_insert(&t2).unwrap();

        let turns = storage.turn_get_by_scope(scope.scope_id).unwrap();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].sequence, 1);
        assert_eq!(turns[1].sequence, 2);
    }

    // ========================================================================
    // Vector Search Tests
    // ========================================================================

    #[test]
    fn test_vector_search() {
        let storage = MockStorage::new();
        let trajectory = make_test_trajectory();
        let scope = make_test_scope(trajectory.trajectory_id);

        let mut a1 = make_test_artifact(trajectory.trajectory_id, scope.scope_id);
        a1.embedding = Some(EmbeddingVector::new(vec![1.0, 0.0, 0.0], "test".to_string()));
        let mut a2 = make_test_artifact(trajectory.trajectory_id, scope.scope_id);
        a2.embedding = Some(EmbeddingVector::new(vec![0.9, 0.1, 0.0], "test".to_string()));
        let mut a3 = make_test_artifact(trajectory.trajectory_id, scope.scope_id);
        a3.embedding = Some(EmbeddingVector::new(vec![0.0, 1.0, 0.0], "test".to_string()));

        storage.trajectory_insert(&trajectory).unwrap();
        storage.scope_insert(&scope).unwrap();
        storage.artifact_insert(&a1).unwrap();
        storage.artifact_insert(&a2).unwrap();
        storage.artifact_insert(&a3).unwrap();

        let query = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "test".to_string());
        let results = storage.vector_search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        // First result should be the most similar (a1 with exact match)
        assert_eq!(results[0].0, a1.artifact_id);
        assert!((results[0].1 - 1.0).abs() < 0.001);
    }
}


// ============================================================================
// PROPERTY-BASED TESTS (Task 11.5)
// ============================================================================

#[cfg(test)]
mod prop_tests {
    use super::*;
    use caliber_core::TrajectoryStatus;
    use proptest::prelude::*;
    use uuid::Uuid;

    fn make_trajectory_with_status(status: TrajectoryStatus) -> Trajectory {
        Trajectory {
            trajectory_id: Uuid::now_v7(),
            name: "Test".to_string(),
            description: None,
            status,
            parent_trajectory_id: None,
            root_trajectory_id: None,
            agent_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            completed_at: None,
            outcome: None,
            metadata: None,
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 10: Storage not-found returns correct error
        /// When getting a non-existent entity, storage SHALL return Ok(None)
        /// Feature: caliber-core-implementation, Property 10: Storage not-found returns correct error
        /// Validates: Requirements 8.4
        #[test]
        fn prop_storage_not_found_returns_none(
            _dummy in any::<u8>()
        ) {
            let storage = MockStorage::new();
            let non_existent_id = Uuid::now_v7();

            // Trajectory not found
            let result = storage.trajectory_get(non_existent_id).unwrap();
            prop_assert!(result.is_none(), "Non-existent trajectory should return None");

            // Scope not found
            let result = storage.scope_get(non_existent_id).unwrap();
            prop_assert!(result.is_none(), "Non-existent scope should return None");

            // Artifact not found
            let result = storage.artifact_get(non_existent_id).unwrap();
            prop_assert!(result.is_none(), "Non-existent artifact should return None");

            // Note not found
            let result = storage.note_get(non_existent_id).unwrap();
            prop_assert!(result.is_none(), "Non-existent note should return None");
        }

        /// Property: Insert then get returns same entity
        #[test]
        fn prop_insert_get_roundtrip(
            _dummy in any::<u8>()
        ) {
            let storage = MockStorage::new();
            let trajectory = make_trajectory_with_status(TrajectoryStatus::Active);
            let original_id = trajectory.trajectory_id;

            storage.trajectory_insert(&trajectory).unwrap();
            let retrieved = storage.trajectory_get(original_id).unwrap();

            prop_assert!(retrieved.is_some());
            prop_assert_eq!(retrieved.unwrap().trajectory_id, original_id);
        }

        /// Property: Update not-found returns error
        #[test]
        fn prop_update_not_found_returns_error(
            _dummy in any::<u8>()
        ) {
            let storage = MockStorage::new();
            let non_existent_id = Uuid::now_v7();

            let result = storage.trajectory_update(
                non_existent_id,
                TrajectoryUpdate {
                    status: Some(TrajectoryStatus::Completed),
                    ..Default::default()
                },
            );

            prop_assert!(result.is_err(), "Update on non-existent entity should fail");
        }

        /// Property: Duplicate insert returns error
        #[test]
        fn prop_duplicate_insert_returns_error(
            _dummy in any::<u8>()
        ) {
            let storage = MockStorage::new();
            let trajectory = make_trajectory_with_status(TrajectoryStatus::Active);

            storage.trajectory_insert(&trajectory).unwrap();
            let result = storage.trajectory_insert(&trajectory);

            prop_assert!(result.is_err(), "Duplicate insert should fail");
        }

        /// Property: Vector search returns at most limit results
        #[test]
        fn prop_vector_search_respects_limit(
            limit in 1i32..10
        ) {
            let storage = MockStorage::new();
            let query = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "test".to_string());

            let results = storage.vector_search(&query, limit).unwrap();

            prop_assert!(
                results.len() <= limit as usize,
                "Vector search should return at most {} results, got {}",
                limit,
                results.len()
            );
        }
    }
}
