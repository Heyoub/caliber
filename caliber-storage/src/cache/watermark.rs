//! Watermark and change journal for cache invalidation.
//!
//! The change journal tracks mutations in storage, allowing the cache to
//! determine if cached data might be stale. Watermarks represent a point
//! in the mutation history.

use async_trait::async_trait;
use caliber_core::{CaliberResult, EntityId, EntityType};
use chrono::{DateTime, Utc};

/// A watermark representing a point in the change journal.
///
/// Watermarks are monotonically increasing and can be compared to determine
/// if mutations have occurred between two points in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Watermark {
    /// Monotonically increasing sequence number.
    /// Each mutation increments this value.
    pub sequence: i64,
    /// When this watermark was observed.
    pub observed_at: DateTime<Utc>,
}

impl Watermark {
    /// Create a new watermark with the given sequence number.
    pub fn new(sequence: i64) -> Self {
        Self {
            sequence,
            observed_at: Utc::now(),
        }
    }

    /// Create a new watermark with explicit observed_at timestamp.
    pub fn with_timestamp(sequence: i64, observed_at: DateTime<Utc>) -> Self {
        Self {
            sequence,
            observed_at,
        }
    }

    /// Create a zero watermark (beginning of time).
    pub fn zero() -> Self {
        Self {
            sequence: 0,
            observed_at: DateTime::UNIX_EPOCH.into(),
        }
    }

    /// Check if this watermark is newer than another.
    pub fn is_newer_than(&self, other: &Watermark) -> bool {
        self.sequence > other.sequence
    }

    /// Check if this watermark is at least as fresh as another.
    pub fn is_at_least(&self, other: &Watermark) -> bool {
        self.sequence >= other.sequence
    }

    /// Calculate the sequence gap between two watermarks.
    pub fn gap(&self, other: &Watermark) -> i64 {
        (self.sequence - other.sequence).abs()
    }
}

impl Default for Watermark {
    fn default() -> Self {
        Self::zero()
    }
}

/// Change journal for tracking mutations and cache invalidation.
///
/// The change journal maintains a log of all mutations, allowing the cache
/// to determine if data has changed since it was cached. Implementations
/// should be efficient for the common case where no changes have occurred.
///
/// # Implementation Notes
///
/// Implementations should:
/// - Be efficient for the `changes_since` query (this is called frequently)
/// - Support tenant isolation (each tenant has independent watermarks)
/// - Consider using database-level change tracking if available
#[async_trait]
pub trait ChangeJournal: Send + Sync {
    /// Get the current watermark for a tenant.
    ///
    /// This returns the latest sequence number for all mutations
    /// affecting the tenant's data.
    async fn current_watermark(&self, tenant_id: EntityId) -> CaliberResult<Watermark>;

    /// Get the watermark at a specific point in time.
    ///
    /// This is useful for determining what the watermark was when
    /// data was cached. Returns None if the timestamp is too old
    /// and the journal has been pruned.
    async fn watermark_at(
        &self,
        tenant_id: EntityId,
        at: DateTime<Utc>,
    ) -> CaliberResult<Option<Watermark>>;

    /// Check if any changes have occurred since the given watermark.
    ///
    /// This is the primary method used by the cache to determine if
    /// cached data might be stale. It returns true if any mutations
    /// of the specified entity types have occurred since the watermark.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant to check changes for
    /// * `watermark` - The watermark to check against
    /// * `entity_types` - Only check for changes to these entity types.
    ///   If empty, checks all entity types.
    async fn changes_since(
        &self,
        tenant_id: EntityId,
        watermark: &Watermark,
        entity_types: &[EntityType],
    ) -> CaliberResult<bool>;

    /// Record a mutation in the journal.
    ///
    /// This should be called whenever an entity is created, updated, or deleted.
    /// It increments the watermark and records the affected entity.
    async fn record_change(
        &self,
        tenant_id: EntityId,
        entity_type: EntityType,
        entity_id: EntityId,
    ) -> CaliberResult<Watermark>;

    /// Prune old entries from the journal.
    ///
    /// Implementations should periodically prune old entries to prevent
    /// unbounded growth. The `before` timestamp indicates that all entries
    /// older than this can be safely removed.
    async fn prune(&self, tenant_id: EntityId, before: DateTime<Utc>) -> CaliberResult<u64>;
}

/// In-memory change journal for testing.
#[derive(Debug, Default)]
pub struct InMemoryChangeJournal {
    /// Changes indexed by tenant_id.
    changes: std::sync::RwLock<std::collections::HashMap<EntityId, TenantChanges>>,
}

#[derive(Debug, Default)]
struct TenantChanges {
    /// Current sequence number.
    sequence: i64,
    /// Log of changes (sequence, timestamp, entity_type, entity_id).
    log: Vec<ChangeEntry>,
}

#[derive(Debug, Clone)]
struct ChangeEntry {
    sequence: i64,
    timestamp: DateTime<Utc>,
    entity_type: EntityType,
    entity_id: EntityId,
}

impl InMemoryChangeJournal {
    /// Create a new in-memory change journal.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ChangeJournal for InMemoryChangeJournal {
    async fn current_watermark(&self, tenant_id: EntityId) -> CaliberResult<Watermark> {
        let changes = self.changes.read().unwrap();
        let sequence = changes
            .get(&tenant_id)
            .map(|tc| tc.sequence)
            .unwrap_or(0);
        Ok(Watermark::new(sequence))
    }

    async fn watermark_at(
        &self,
        tenant_id: EntityId,
        at: DateTime<Utc>,
    ) -> CaliberResult<Option<Watermark>> {
        let changes = self.changes.read().unwrap();
        if let Some(tenant_changes) = changes.get(&tenant_id) {
            // Find the latest entry at or before the given timestamp
            let sequence = tenant_changes
                .log
                .iter()
                .rev()
                .find(|e| e.timestamp <= at)
                .map(|e| e.sequence)
                .unwrap_or(0);
            Ok(Some(Watermark::with_timestamp(sequence, at)))
        } else {
            Ok(Some(Watermark::zero()))
        }
    }

    async fn changes_since(
        &self,
        tenant_id: EntityId,
        watermark: &Watermark,
        entity_types: &[EntityType],
    ) -> CaliberResult<bool> {
        let changes = self.changes.read().unwrap();
        if let Some(tenant_changes) = changes.get(&tenant_id) {
            // Check if any changes exist after the watermark
            let has_changes = tenant_changes.log.iter().any(|e| {
                e.sequence > watermark.sequence
                    && (entity_types.is_empty() || entity_types.contains(&e.entity_type))
            });
            Ok(has_changes)
        } else {
            Ok(false)
        }
    }

    async fn record_change(
        &self,
        tenant_id: EntityId,
        entity_type: EntityType,
        entity_id: EntityId,
    ) -> CaliberResult<Watermark> {
        let mut changes = self.changes.write().unwrap();
        let tenant_changes = changes.entry(tenant_id).or_default();

        tenant_changes.sequence += 1;
        let entry = ChangeEntry {
            sequence: tenant_changes.sequence,
            timestamp: Utc::now(),
            entity_type,
            entity_id,
        };
        tenant_changes.log.push(entry);

        Ok(Watermark::new(tenant_changes.sequence))
    }

    async fn prune(&self, tenant_id: EntityId, before: DateTime<Utc>) -> CaliberResult<u64> {
        let mut changes = self.changes.write().unwrap();
        if let Some(tenant_changes) = changes.get_mut(&tenant_id) {
            let before_len = tenant_changes.log.len();
            tenant_changes.log.retain(|e| e.timestamp >= before);
            let after_len = tenant_changes.log.len();
            Ok((before_len - after_len) as u64)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_watermark_ordering() {
        let w1 = Watermark::new(1);
        let w2 = Watermark::new(2);
        let w3 = Watermark::new(2);

        assert!(w2.is_newer_than(&w1));
        assert!(!w1.is_newer_than(&w2));
        assert!(!w2.is_newer_than(&w3));

        assert!(w2.is_at_least(&w1));
        assert!(w2.is_at_least(&w3));
        assert!(!w1.is_at_least(&w2));
    }

    #[test]
    fn test_watermark_gap() {
        let w1 = Watermark::new(10);
        let w2 = Watermark::new(15);

        assert_eq!(w1.gap(&w2), 5);
        assert_eq!(w2.gap(&w1), 5);
    }

    #[test]
    fn test_watermark_zero() {
        let zero = Watermark::zero();
        assert_eq!(zero.sequence, 0);
    }

    #[tokio::test]
    async fn test_in_memory_journal() {
        let journal = InMemoryChangeJournal::new();
        let tenant_id = Uuid::now_v7();
        let entity_id = Uuid::now_v7();

        // Initial watermark should be 0
        let w0 = journal.current_watermark(tenant_id).await.unwrap();
        assert_eq!(w0.sequence, 0);

        // Record a change
        let w1 = journal
            .record_change(tenant_id, EntityType::Artifact, entity_id)
            .await
            .unwrap();
        assert_eq!(w1.sequence, 1);

        // Changes since w0 should be true
        let has_changes = journal
            .changes_since(tenant_id, &w0, &[])
            .await
            .unwrap();
        assert!(has_changes);

        // Changes since w1 should be false
        let has_changes = journal
            .changes_since(tenant_id, &w1, &[])
            .await
            .unwrap();
        assert!(!has_changes);
    }

    #[tokio::test]
    async fn test_journal_entity_type_filter() {
        let journal = InMemoryChangeJournal::new();
        let tenant_id = Uuid::now_v7();
        let entity_id = Uuid::now_v7();

        let w0 = journal.current_watermark(tenant_id).await.unwrap();

        // Record an Artifact change
        journal
            .record_change(tenant_id, EntityType::Artifact, entity_id)
            .await
            .unwrap();

        // Changes for Artifact type should be true
        let has_artifact_changes = journal
            .changes_since(tenant_id, &w0, &[EntityType::Artifact])
            .await
            .unwrap();
        assert!(has_artifact_changes);

        // Changes for Note type should be false
        let has_note_changes = journal
            .changes_since(tenant_id, &w0, &[EntityType::Note])
            .await
            .unwrap();
        assert!(!has_note_changes);
    }

    #[tokio::test]
    async fn test_journal_tenant_isolation() {
        let journal = InMemoryChangeJournal::new();
        let tenant_a = Uuid::now_v7();
        let tenant_b = Uuid::now_v7();
        let entity_id = Uuid::now_v7();

        let w0_a = journal.current_watermark(tenant_a).await.unwrap();
        let w0_b = journal.current_watermark(tenant_b).await.unwrap();

        // Record change for tenant A
        journal
            .record_change(tenant_a, EntityType::Artifact, entity_id)
            .await
            .unwrap();

        // Tenant A should see changes
        let has_changes_a = journal
            .changes_since(tenant_a, &w0_a, &[])
            .await
            .unwrap();
        assert!(has_changes_a);

        // Tenant B should not see changes
        let has_changes_b = journal
            .changes_since(tenant_b, &w0_b, &[])
            .await
            .unwrap();
        assert!(!has_changes_b);
    }
}
