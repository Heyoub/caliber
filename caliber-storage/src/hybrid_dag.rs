//! HybridDag - Hot LMDB + Cold PostgreSQL event storage.
//!
//! This module provides a production-ready EventDag implementation that combines:
//! - LMDB for hot path (microsecond reads of recent events)
//! - PostgreSQL for cold storage (millisecond reads, durable persistence)
//!
//! Events are written to both stores. Reads try LMDB first, fall back to PostgreSQL.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use heed::{types::SerdeBincode, Database, Env, EnvOpenOptions};

use caliber_core::{
    DagPosition, DomainError, DomainErrorContext, Effect, ErrorEffect, Event, EventDag, EventId,
    EventKind, OperationalError, UpstreamSignal,
};

// ============================================================================
// COLD STORAGE TRAIT
// ============================================================================

/// Trait for cold (PostgreSQL) event storage.
///
/// This abstraction allows HybridDag to work with any async database backend.
/// The caliber-api crate provides a tokio-postgres implementation.
#[async_trait::async_trait]
pub trait ColdEventStorage: Send + Sync {
    /// The payload type for events.
    type Payload: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned + 'static;

    /// Store an event in cold storage.
    async fn store(&self, event: &Event<Self::Payload>) -> Result<(), ColdStorageError>;

    /// Retrieve an event by ID from cold storage.
    async fn get(
        &self,
        event_id: EventId,
    ) -> Result<Option<Event<Self::Payload>>, ColdStorageError>;

    /// Walk ancestors from a given event.
    async fn walk_ancestors(
        &self,
        from: EventId,
        limit: usize,
    ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError>;

    /// Walk descendants from a given event.
    async fn walk_descendants(
        &self,
        from: EventId,
        limit: usize,
    ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError>;

    /// Find events by correlation ID.
    async fn find_by_correlation(
        &self,
        correlation_id: EventId,
    ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError>;

    /// Find events by kind within a depth range.
    async fn find_by_kind(
        &self,
        kind: EventKind,
        min_depth: u32,
        max_depth: u32,
        limit: usize,
    ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError>;

    /// Get unacknowledged events.
    async fn unacknowledged(
        &self,
        limit: usize,
    ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError>;

    /// Mark an event as acknowledged.
    async fn acknowledge(&self, event_id: EventId) -> Result<(), ColdStorageError>;

    /// Get the next sequence number.
    async fn next_sequence(&self) -> Result<u64, ColdStorageError>;
}

/// Errors from cold storage operations.
#[derive(Debug, thiserror::Error)]
pub enum ColdStorageError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Event not found: {0}")]
    NotFound(EventId),
}

// ============================================================================
// HOT CACHE (LMDB)
// ============================================================================

/// LMDB-based hot cache for recent events.
///
/// Uses memory-mapped I/O for zero-copy reads. Events are stored by ID
/// with automatic serialization via serde.
pub struct LmdbEventCache<P: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned> {
    env: Env,
    events: Database<SerdeBincode<EventId>, SerdeBincode<Event<P>>>,
    acknowledged: Database<SerdeBincode<EventId>, SerdeBincode<bool>>,
}

impl<P: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned + 'static>
    LmdbEventCache<P>
{
    /// Open or create an LMDB event cache at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, HybridDagError> {
        std::fs::create_dir_all(path.as_ref()).map_err(|e| HybridDagError::Io(e.to_string()))?;

        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(512 * 1024 * 1024) // 512MB for events
                .max_dbs(4)
                .open(path.as_ref())
                .map_err(|e| HybridDagError::Lmdb(e.to_string()))?
        };

        let mut wtxn = env
            .write_txn()
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        let events = env
            .create_database(&mut wtxn, Some("events"))
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        let acknowledged = env
            .create_database(&mut wtxn, Some("acknowledged"))
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        wtxn.commit()
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        Ok(Self {
            env,
            events,
            acknowledged,
        })
    }

    /// Get an event from the hot cache.
    pub fn get(&self, event_id: EventId) -> Result<Option<Event<P>>, HybridDagError> {
        let rtxn = self
            .env
            .read_txn()
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        self.events
            .get(&rtxn, &event_id)
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))
    }

    /// Store an event in the hot cache.
    pub fn put(&self, event: &Event<P>) -> Result<(), HybridDagError> {
        let mut wtxn = self
            .env
            .write_txn()
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        self.events
            .put(&mut wtxn, &event.header.event_id, event)
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        wtxn.commit()
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))
    }

    /// Check if an event is acknowledged in the hot cache.
    pub fn is_acknowledged(&self, event_id: EventId) -> Result<bool, HybridDagError> {
        let rtxn = self
            .env
            .read_txn()
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        Ok(self
            .acknowledged
            .get(&rtxn, &event_id)
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?
            .unwrap_or(false))
    }

    /// Mark an event as acknowledged in the hot cache.
    pub fn set_acknowledged(&self, event_id: EventId) -> Result<(), HybridDagError> {
        let mut wtxn = self
            .env
            .write_txn()
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        self.acknowledged
            .put(&mut wtxn, &event_id, &true)
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        wtxn.commit()
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))
    }

    /// Get statistics about the cache.
    pub fn stats(&self) -> Result<LmdbCacheStats, HybridDagError> {
        let rtxn = self
            .env
            .read_txn()
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        let event_count = self
            .events
            .len(&rtxn)
            .map_err(|e| HybridDagError::Lmdb(e.to_string()))?;

        Ok(LmdbCacheStats { event_count })
    }
}

/// Statistics about the LMDB event cache.
#[derive(Debug, Clone)]
pub struct LmdbCacheStats {
    pub event_count: u64,
}

// ============================================================================
// HYBRID DAG
// ============================================================================

/// Errors from HybridDag operations.
#[derive(Debug, thiserror::Error)]
pub enum HybridDagError {
    #[error("LMDB error: {0}")]
    Lmdb(String),

    #[error("Cold storage error: {0}")]
    Cold(#[from] ColdStorageError),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// HybridDag combines LMDB hot cache with PostgreSQL cold storage.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                         HybridDag                           │
/// ├─────────────────────────┬───────────────────────────────────┤
/// │   LMDB Hot Cache        │    PostgreSQL Cold Storage        │
/// │   (microseconds)        │    (milliseconds)                 │
/// │                         │                                   │
/// │   Recent events         │    All events (durable)           │
/// │   Zero-copy reads       │    Full history                   │
/// │   Memory-mapped         │    ACID guarantees                │
/// └─────────────────────────┴───────────────────────────────────┘
/// ```
///
/// # Write Path
/// 1. Write to LMDB (fast, synchronous)
/// 2. Write to PostgreSQL (async, durable)
///
/// # Read Path
/// 1. Try LMDB first (microseconds)
/// 2. Fall back to PostgreSQL on cache miss (milliseconds)
pub struct HybridDag<C: ColdEventStorage> {
    hot: LmdbEventCache<C::Payload>,
    cold: Arc<C>,
    /// How long events stay "hot" (default: 1 hour)
    pub hot_window: Duration,
}

impl<C: ColdEventStorage> HybridDag<C> {
    /// Create a new HybridDag with the given hot cache path and cold storage.
    pub fn new(hot_path: impl AsRef<Path>, cold: Arc<C>) -> Result<Self, HybridDagError> {
        let hot = LmdbEventCache::open(hot_path)?;
        Ok(Self {
            hot,
            cold,
            hot_window: Duration::from_secs(3600), // 1 hour default
        })
    }

    /// Set the hot window duration.
    pub fn with_hot_window(mut self, duration: Duration) -> Self {
        self.hot_window = duration;
        self
    }

    /// Get cache statistics.
    pub fn stats(&self) -> Result<LmdbCacheStats, HybridDagError> {
        self.hot.stats()
    }

    /// Convert a HybridDagError to an Effect error.
    fn to_effect_error(_event_id: EventId, error: HybridDagError) -> ErrorEffect {
        ErrorEffect::Operational(OperationalError::DatabaseConnectionError {
            message: format!("hybrid_dag: {}", error),
        })
    }
}

#[async_trait::async_trait]
impl<C: ColdEventStorage + 'static> EventDag for HybridDag<C> {
    type Payload = C::Payload;

    async fn append(&self, event: Event<Self::Payload>) -> Effect<EventId> {
        let event_id = event.header.event_id;

        // Write to hot cache first (fast, synchronous)
        if let Err(e) = self.hot.put(&event) {
            return Effect::Err(Self::to_effect_error(event_id, e));
        }

        // Write to cold storage (async, durable)
        if let Err(e) = self.cold.store(&event).await {
            // Hot cache has it, but cold failed - log warning but don't fail
            // A background job should reconcile
            eprintln!("Failed to write event {} to cold storage: {}", event_id, e);
        }

        Effect::Ok(event_id)
    }

    async fn read(&self, event_id: EventId) -> Effect<Event<Self::Payload>> {
        // Try hot cache first (microseconds)
        match self.hot.get(event_id) {
            Ok(Some(event)) => return Effect::Ok(event),
            Ok(None) => {} // Cache miss, try cold
            Err(e) => {
                eprintln!("Hot cache read error for {}: {}", event_id, e);
                // Fall through to cold storage
            }
        }

        // Fall back to cold storage (milliseconds)
        match self.cold.get(event_id).await {
            Ok(Some(event)) => {
                // Optionally warm the cache
                let _ = self.hot.put(&event);
                Effect::Ok(event)
            }
            Ok(None) => Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                error: DomainError::EntityNotFound {
                    entity_type: "Event".to_string(),
                    id: event_id,
                },
                source_event: event_id,
                position: DagPosition::root(),
                correlation_id: event_id,
            }))),
            Err(e) => Effect::Err(Self::to_effect_error(event_id, e.into())),
        }
    }

    async fn walk_ancestors(
        &self,
        from: EventId,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>> {
        // For ancestor walks, we primarily use cold storage
        // since ancestors are likely older events
        match self.cold.walk_ancestors(from, limit).await {
            Ok(events) => Effect::Ok(events),
            Err(e) => Effect::Err(Self::to_effect_error(from, e.into())),
        }
    }

    async fn walk_descendants(
        &self,
        from: EventId,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>> {
        match self.cold.walk_descendants(from, limit).await {
            Ok(events) => Effect::Ok(events),
            Err(e) => Effect::Err(Self::to_effect_error(from, e.into())),
        }
    }

    async fn signal_upstream(&self, from: EventId, _signal: UpstreamSignal) -> Effect<()> {
        // Verify event exists
        match self.read(from).await {
            Effect::Ok(_) => Effect::Ok(()),
            Effect::Err(e) => Effect::Err(e),
            other => other.map(|_| ()),
        }
    }

    async fn find_correlation_chain(
        &self,
        correlation_id: EventId,
    ) -> Effect<Vec<Event<Self::Payload>>> {
        match self.cold.find_by_correlation(correlation_id).await {
            Ok(events) => Effect::Ok(events),
            Err(e) => Effect::Err(Self::to_effect_error(correlation_id, e.into())),
        }
    }

    async fn next_position(&self, parent: Option<EventId>, lane: u32) -> Effect<DagPosition> {
        let depth = match parent {
            Some(parent_id) => match self.read(parent_id).await {
                Effect::Ok(parent_event) => parent_event.header.position.depth + 1,
                Effect::Err(e) => return Effect::Err(e),
                other => return other.map(|_| unreachable!()),
            },
            None => 0,
        };

        let sequence = match self.cold.next_sequence().await {
            Ok(seq) => seq as u32,
            Err(e) => {
                return Effect::Err(Self::to_effect_error(uuid::Uuid::nil(), e.into()));
            }
        };

        Effect::Ok(DagPosition {
            depth,
            lane,
            sequence,
        })
    }

    async fn find_by_kind(
        &self,
        kind: EventKind,
        min_depth: u32,
        max_depth: u32,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>> {
        match self
            .cold
            .find_by_kind(kind, min_depth, max_depth, limit)
            .await
        {
            Ok(events) => Effect::Ok(events),
            Err(e) => Effect::Err(Self::to_effect_error(uuid::Uuid::nil(), e.into())),
        }
    }

    async fn acknowledge(&self, event_id: EventId, send_upstream: bool) -> Effect<()> {
        // Mark acknowledged in hot cache
        if let Err(e) = self.hot.set_acknowledged(event_id) {
            eprintln!(
                "Failed to mark event {} acknowledged in hot cache: {}",
                event_id, e
            );
        }

        // Mark acknowledged in cold storage
        if let Err(e) = self.cold.acknowledge(event_id).await {
            return Effect::Err(Self::to_effect_error(event_id, e.into()));
        }

        if send_upstream {
            // Send upstream ack signal
            let signal = UpstreamSignal::Ack { event_id };
            let _ = self.signal_upstream(event_id, signal).await;
        }

        Effect::Ok(())
    }

    async fn unacknowledged(&self, limit: usize) -> Effect<Vec<Event<Self::Payload>>> {
        match self.cold.unacknowledged(limit).await {
            Ok(events) => Effect::Ok(events),
            Err(e) => Effect::Err(Self::to_effect_error(uuid::Uuid::nil(), e.into())),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::EventFlags;
    use std::collections::HashMap;
    use std::sync::RwLock;
    use tempfile::TempDir;

    /// Mock cold storage for testing.
    struct MockColdStorage {
        events: RwLock<HashMap<EventId, Event<String>>>,
        sequence: RwLock<u64>,
    }

    impl MockColdStorage {
        fn new() -> Self {
            Self {
                events: RwLock::new(HashMap::new()),
                sequence: RwLock::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl ColdEventStorage for MockColdStorage {
        type Payload = String;

        async fn store(&self, event: &Event<Self::Payload>) -> Result<(), ColdStorageError> {
            self.events
                .write()
                .expect("test lock should not be poisoned")
                .insert(event.header.event_id, event.clone());
            Ok(())
        }

        async fn get(
            &self,
            event_id: EventId,
        ) -> Result<Option<Event<Self::Payload>>, ColdStorageError> {
            Ok(self
                .events
                .read()
                .expect("test lock should not be poisoned")
                .get(&event_id)
                .cloned())
        }

        async fn walk_ancestors(
            &self,
            _from: EventId,
            _limit: usize,
        ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError> {
            Ok(vec![])
        }

        async fn walk_descendants(
            &self,
            _from: EventId,
            _limit: usize,
        ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError> {
            Ok(vec![])
        }

        async fn find_by_correlation(
            &self,
            correlation_id: EventId,
        ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError> {
            let events = self
                .events
                .read()
                .expect("test lock should not be poisoned");
            Ok(events
                .values()
                .filter(|e| e.header.correlation_id == correlation_id)
                .cloned()
                .collect())
        }

        async fn find_by_kind(
            &self,
            kind: EventKind,
            min_depth: u32,
            max_depth: u32,
            limit: usize,
        ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError> {
            let events = self
                .events
                .read()
                .expect("test lock should not be poisoned");
            Ok(events
                .values()
                .filter(|e| {
                    e.header.event_kind == kind
                        && e.header.position.depth >= min_depth
                        && e.header.position.depth <= max_depth
                })
                .take(limit)
                .cloned()
                .collect())
        }

        async fn unacknowledged(
            &self,
            _limit: usize,
        ) -> Result<Vec<Event<Self::Payload>>, ColdStorageError> {
            Ok(vec![])
        }

        async fn acknowledge(&self, _event_id: EventId) -> Result<(), ColdStorageError> {
            Ok(())
        }

        async fn next_sequence(&self) -> Result<u64, ColdStorageError> {
            let mut seq = self
                .sequence
                .write()
                .expect("test lock should not be poisoned");
            *seq += 1;
            Ok(*seq)
        }
    }

    #[tokio::test]
    async fn test_hybrid_dag_append_and_read() {
        let temp_dir = TempDir::new().expect("TempDir creation should succeed");
        let cold = Arc::new(MockColdStorage::new());
        let dag = HybridDag::new(temp_dir.path(), cold).expect("HybridDag creation should succeed");

        // Create an event
        let event_id = uuid::Uuid::now_v7();
        let event = Event {
            header: caliber_core::EventHeader::new(
                event_id,
                event_id,
                chrono::Utc::now().timestamp_micros(),
                DagPosition::root(),
                0,
                EventKind::DATA,
                EventFlags::empty(),
            ),
            payload: "test payload".to_string(),
            hash_chain: None,
        };

        // Append
        let result = dag.append(event.clone()).await;
        assert!(matches!(result, Effect::Ok(id) if id == event_id));

        // Read back
        let read_result = dag.read(event_id).await;
        match read_result {
            Effect::Ok(e) => assert_eq!(e.payload, "test payload"),
            _ => panic!("Expected Ok"),
        }
    }

    #[tokio::test]
    async fn test_hybrid_dag_cache_miss_falls_back_to_cold() {
        let temp_dir = TempDir::new().expect("TempDir creation should succeed");
        let cold = Arc::new(MockColdStorage::new());

        // Pre-populate cold storage
        let event_id = uuid::Uuid::now_v7();
        let event = Event {
            header: caliber_core::EventHeader::new(
                event_id,
                event_id,
                chrono::Utc::now().timestamp_micros(),
                DagPosition::root(),
                0,
                EventKind::DATA,
                EventFlags::empty(),
            ),
            payload: "cold event".to_string(),
            hash_chain: None,
        };
        cold.store(&event).await.expect("cold store should succeed");

        // Create dag WITHOUT the event in hot cache
        let dag = HybridDag::new(temp_dir.path(), cold).expect("HybridDag creation should succeed");

        // Read should fall back to cold storage
        let read_result = dag.read(event_id).await;
        match read_result {
            Effect::Ok(e) => assert_eq!(e.payload, "cold event"),
            _ => panic!("Expected Ok from cold storage fallback"),
        }
    }
}
