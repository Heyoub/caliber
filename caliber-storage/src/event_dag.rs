//! In-memory EventDag implementation for testing.
//!
//! This module provides a simple in-memory implementation of the EventDag trait
//! suitable for unit tests and development scenarios.

use caliber_core::{
    CaliberError, CaliberResult, DagPosition, DomainError, DomainErrorContext, Effect, ErrorEffect,
    Event, EventDag, EventFlags, EventId, EventKind, OperationalError, StorageError,
    UpstreamSignal,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// In-memory EventDag implementation for testing.
///
/// This implementation stores events in memory using a HashMap protected by
/// an RwLock for thread safety. It is not intended for production use but
/// is suitable for unit tests and development.
///
/// # Example
///
/// ```rust,ignore
/// use caliber_storage::InMemoryEventDag;
/// use caliber_core::{EventDag, EventDagExt};
///
/// let dag: InMemoryEventDag<String> = InMemoryEventDag::new();
/// let event_id = dag.append_root("initial event".to_string()).await;
/// ```
pub struct InMemoryEventDag<P: Clone + Send + Sync> {
    events: Arc<RwLock<HashMap<EventId, Event<P>>>>,
    acknowledged: Arc<RwLock<std::collections::HashSet<EventId>>>,
    next_sequence: Arc<RwLock<u64>>,
}

impl<P: Clone + Send + Sync> InMemoryEventDag<P> {
    /// Create a new empty in-memory EventDag.
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
            acknowledged: Arc::new(RwLock::new(std::collections::HashSet::new())),
            next_sequence: Arc::new(RwLock::new(0)),
        }
    }

    /// Get the number of events in the DAG.
    pub fn len(&self) -> CaliberResult<usize> {
        let events = self
            .events
            .read()
            .map_err(|_| CaliberError::Storage(StorageError::LockPoisoned))?;
        Ok(events.len())
    }

    /// Check if the DAG is empty.
    pub fn is_empty(&self) -> CaliberResult<bool> {
        let events = self
            .events
            .read()
            .map_err(|_| CaliberError::Storage(StorageError::LockPoisoned))?;
        Ok(events.is_empty())
    }

    /// Clear all events from the DAG.
    pub fn clear(&self) -> CaliberResult<()> {
        self.events
            .write()
            .map_err(|_| CaliberError::Storage(StorageError::LockPoisoned))?
            .clear();
        self.acknowledged
            .write()
            .map_err(|_| CaliberError::Storage(StorageError::LockPoisoned))?
            .clear();
        *self
            .next_sequence
            .write()
            .map_err(|_| CaliberError::Storage(StorageError::LockPoisoned))? = 0;
        Ok(())
    }
}

impl<P: Clone + Send + Sync> Default for InMemoryEventDag<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Clone + Send + Sync> Clone for InMemoryEventDag<P> {
    fn clone(&self) -> Self {
        Self {
            events: Arc::clone(&self.events),
            acknowledged: Arc::clone(&self.acknowledged),
            next_sequence: Arc::clone(&self.next_sequence),
        }
    }
}

#[async_trait::async_trait]
impl<P: Clone + Send + Sync + 'static> EventDag for InMemoryEventDag<P> {
    type Payload = P;

    async fn append(&self, mut event: Event<Self::Payload>) -> Effect<EventId> {
        let mut events = match self.events.write() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };
        let mut seq = match self.next_sequence.write() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: next_sequence".to_string(),
                }))
            }
        };

        // Assign a new ID if not already set (ID is zero)
        if event.header.event_id == Uuid::nil() {
            event.header.event_id = Uuid::now_v7();
        }

        // Set sequence number
        event.header.position.sequence = *seq as u32;
        *seq += 1;

        let id = event.header.event_id;
        events.insert(id, event);
        Effect::Ok(id)
    }

    async fn read(&self, event_id: EventId) -> Effect<Event<Self::Payload>> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };
        match events.get(&event_id) {
            Some(event) => Effect::Ok(event.clone()),
            None => Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                error: DomainError::EntityNotFound {
                    entity_type: "Event".to_string(),
                    id: event_id,
                },
                source_event: event_id,
                position: DagPosition::root(),
                correlation_id: event_id,
            }))),
        }
    }

    async fn walk_ancestors(
        &self,
        from: EventId,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };

        // First verify the starting event exists
        if !events.contains_key(&from) {
            return Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                error: DomainError::EntityNotFound {
                    entity_type: "Event".to_string(),
                    id: from,
                },
                source_event: from,
                position: DagPosition::root(),
                correlation_id: from,
            })));
        }

        let mut result = Vec::new();
        let mut current_id = Some(from);

        while let Some(id) = current_id {
            if result.len() >= limit {
                break;
            }

            if let Some(event) = events.get(&id) {
                result.push(event.clone());
                // In this simple implementation, we don't track parent relationships
                // This would need to be enhanced for real ancestor walking
                current_id = None;
            } else {
                break;
            }
        }

        Effect::Ok(result)
    }

    async fn walk_descendants(
        &self,
        from: EventId,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };

        // First verify the starting event exists
        if !events.contains_key(&from) {
            return Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                error: DomainError::EntityNotFound {
                    entity_type: "Event".to_string(),
                    id: from,
                },
                source_event: from,
                position: DagPosition::root(),
                correlation_id: from,
            })));
        }

        // In this simple implementation, we return events with higher depth
        let from_event = match events.get(&from) {
            Some(e) => e,
            None => {
                return Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                    error: DomainError::EntityNotFound {
                        entity_type: "Event".to_string(),
                        id: from,
                    },
                    source_event: from,
                    position: DagPosition::root(),
                    correlation_id: from,
                })));
            }
        };
        let from_depth = from_event.header.position.depth;

        let mut result: Vec<Event<Self::Payload>> = events
            .values()
            .filter(|e| e.header.position.depth > from_depth)
            .take(limit)
            .cloned()
            .collect();

        result.sort_by_key(|e| e.header.position.sequence);
        Effect::Ok(result)
    }

    async fn signal_upstream(&self, from: EventId, _signal: UpstreamSignal) -> Effect<()> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };

        if !events.contains_key(&from) {
            return Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                error: DomainError::EntityNotFound {
                    entity_type: "Event".to_string(),
                    id: from,
                },
                source_event: from,
                position: DagPosition::root(),
                correlation_id: from,
            })));
        }

        // In-memory implementation: signals are no-ops (no persistence)
        Effect::Ok(())
    }

    async fn find_correlation_chain(
        &self,
        correlation_id: EventId,
    ) -> Effect<Vec<Event<Self::Payload>>> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };

        let mut result: Vec<Event<Self::Payload>> = events
            .values()
            .filter(|e| e.header.correlation_id == correlation_id)
            .cloned()
            .collect();

        result.sort_by_key(|e| e.header.position.sequence);
        Effect::Ok(result)
    }

    async fn next_position(&self, parent: Option<EventId>, lane: u32) -> Effect<DagPosition> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };
        let seq = match self.next_sequence.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: next_sequence".to_string(),
                }))
            }
        };

        let depth = match parent {
            Some(parent_id) => match events.get(&parent_id) {
                Some(parent_event) => parent_event.header.position.depth + 1,
                None => {
                    return Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                        error: DomainError::EntityNotFound {
                            entity_type: "Event".to_string(),
                            id: parent_id,
                        },
                        source_event: parent_id,
                        position: DagPosition::root(),
                        correlation_id: parent_id,
                    })));
                }
            },
            None => 0,
        };

        Effect::Ok(DagPosition {
            depth,
            lane,
            sequence: *seq as u32,
        })
    }

    async fn find_by_kind(
        &self,
        kind: EventKind,
        min_depth: u32,
        max_depth: u32,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };

        let mut result: Vec<Event<Self::Payload>> = events
            .values()
            .filter(|e| {
                e.header.event_kind == kind
                    && e.header.position.depth >= min_depth
                    && e.header.position.depth <= max_depth
            })
            .take(limit)
            .cloned()
            .collect();

        result.sort_by_key(|e| e.header.position.sequence);
        Effect::Ok(result)
    }

    async fn acknowledge(&self, event_id: EventId, _send_upstream: bool) -> Effect<()> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };

        if !events.contains_key(&event_id) {
            return Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                error: DomainError::EntityNotFound {
                    entity_type: "Event".to_string(),
                    id: event_id,
                },
                source_event: event_id,
                position: DagPosition::root(),
                correlation_id: event_id,
            })));
        }

        match self.acknowledged.write() {
            Ok(mut guard) => {
                guard.insert(event_id);
            }
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: acknowledged".to_string(),
                }))
            }
        };
        Effect::Ok(())
    }

    async fn unacknowledged(&self, limit: usize) -> Effect<Vec<Event<Self::Payload>>> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };
        let acknowledged = match self.acknowledged.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: acknowledged".to_string(),
                }))
            }
        };

        let mut result: Vec<Event<Self::Payload>> = events
            .values()
            .filter(|e| {
                e.header.flags.contains(EventFlags::REQUIRES_ACK)
                    && !acknowledged.contains(&e.header.event_id)
            })
            .take(limit)
            .cloned()
            .collect();

        result.sort_by_key(|e| e.header.position.sequence);
        Effect::Ok(result)
    }
}

impl<P: Clone + Send + Sync + Serialize> InMemoryEventDag<P> {
    /// Find events by kind after a given timestamp.
    ///
    /// # Arguments
    /// * `kind` - Event kind to filter by
    /// * `after_timestamp` - Only return events after this timestamp (microseconds)
    /// * `limit` - Maximum number of events to return
    pub async fn find_by_kind_after(
        &self,
        kind: EventKind,
        after_timestamp: i64,
        limit: usize,
    ) -> Effect<Vec<Event<P>>> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };

        let mut matching: Vec<Event<P>> = events
            .values()
            .filter(|e| e.header.event_kind == kind && e.header.timestamp > after_timestamp)
            .cloned()
            .collect();

        matching.sort_by_key(|e| e.header.timestamp);
        matching.truncate(limit);

        Effect::Ok(matching)
    }

    /// Verify hash chain integrity for a sequence of events.
    ///
    /// Checks that each event's hash chain correctly references the previous event's hash.
    /// Returns true if the chain is valid, false if any link is broken.
    ///
    /// # Arguments
    /// * `event_ids` - Ordered list of event IDs to verify (parent to child order)
    ///
    /// # Returns
    /// * `Effect::Ok(true)` - Chain is valid
    /// * `Effect::Ok(false)` - Chain has broken links
    /// * `Effect::Err` - Event not found
    pub async fn verify_chain_integrity(&self, event_ids: &[EventId]) -> Effect<bool> {
        let events = match self.events.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Effect::Err(ErrorEffect::Operational(OperationalError::Internal {
                    message: "Lock poisoned: events".to_string(),
                }))
            }
        };

        for window in event_ids.windows(2) {
            let parent_id = window[0];
            let child_id = window[1];

            let parent = match events.get(&parent_id) {
                Some(e) => e,
                None => {
                    return Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                        error: DomainError::EntityNotFound {
                            entity_type: "Event".to_string(),
                            id: parent_id,
                        },
                        source_event: parent_id,
                        position: DagPosition::root(),
                        correlation_id: parent_id,
                    })));
                }
            };

            let child = match events.get(&child_id) {
                Some(e) => e,
                None => {
                    return Effect::Err(ErrorEffect::Domain(Box::new(DomainErrorContext {
                        error: DomainError::EntityNotFound {
                            entity_type: "Event".to_string(),
                            id: child_id,
                        },
                        source_event: child_id,
                        position: DagPosition::root(),
                        correlation_id: child_id,
                    })));
                }
            };

            let parent_hash = parent.compute_hash();
            if !child.verify(&parent_hash) {
                return Effect::Ok(false);
            }
        }

        Effect::Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{EventDagExt, EventHeader};

    /// Helper to unwrap Effect in tests
    fn unwrap_effect<T: std::fmt::Debug>(effect: Effect<T>, msg: &str) -> T {
        match effect {
            Effect::Ok(value) => value,
            Effect::Err(e) => panic!("{}: {:?}", msg, e),
            _ => panic!("{}: unexpected Effect variant (not Ok or Err)", msg),
        }
    }

    #[tokio::test]
    async fn test_append_and_read() {
        let dag: InMemoryEventDag<String> = InMemoryEventDag::new();

        let event_id = dag.append_root("test payload".to_string()).await;
        assert!(event_id.is_ok());

        let id = unwrap_effect(event_id, "append_root should succeed");
        let read_result = dag.read(id).await;
        assert!(read_result.is_ok());

        let event = unwrap_effect(read_result, "read should succeed");
        assert_eq!(event.payload, "test payload");
    }

    #[tokio::test]
    async fn test_read_nonexistent() {
        let dag: InMemoryEventDag<String> = InMemoryEventDag::new();
        let fake_id = Uuid::now_v7();

        let result = dag.read(fake_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_append_child() {
        let dag: InMemoryEventDag<String> = InMemoryEventDag::new();

        let root_id = unwrap_effect(
            dag.append_root("root".to_string()).await,
            "append_root should succeed",
        );
        let child_id = dag.append_child(root_id, "child".to_string()).await;

        assert!(child_id.is_ok());
        let child = unwrap_effect(
            dag.read(unwrap_effect(child_id, "append_child should succeed"))
                .await,
            "read should succeed",
        );
        assert_eq!(child.header.position.depth, 1);
    }

    #[tokio::test]
    async fn test_acknowledge() {
        let dag: InMemoryEventDag<String> = InMemoryEventDag::new();

        // Create event requiring ack
        let position = unwrap_effect(
            dag.next_position(None, 0).await,
            "next_position should succeed",
        );
        let event_id_val = Uuid::now_v7();
        let header = EventHeader::new(
            event_id_val,
            event_id_val,
            chrono::Utc::now().timestamp_micros(),
            position,
            0,
            EventKind::DATA,
            EventFlags::REQUIRES_ACK,
        );
        let event = Event {
            header,
            payload: "needs ack".to_string(),
            hash_chain: None,
        };
        let event_id = unwrap_effect(dag.append(event).await, "append should succeed");

        // Should appear in unacknowledged
        let unacked = unwrap_effect(
            dag.unacknowledged(10).await,
            "unacknowledged should succeed",
        );
        assert_eq!(unacked.len(), 1);

        // Acknowledge it
        unwrap_effect(
            dag.acknowledge(event_id, false).await,
            "acknowledge should succeed",
        );

        // Should no longer appear
        let unacked = unwrap_effect(
            dag.unacknowledged(10).await,
            "unacknowledged should succeed",
        );
        assert_eq!(unacked.len(), 0);
    }

    #[tokio::test]
    async fn test_find_by_kind() {
        let dag: InMemoryEventDag<String> = InMemoryEventDag::new();

        // Create a few events
        unwrap_effect(
            dag.append_root("event1".to_string()).await,
            "append_root should succeed",
        );
        unwrap_effect(
            dag.append_root("event2".to_string()).await,
            "append_root should succeed",
        );

        let found = unwrap_effect(
            dag.find_by_kind(EventKind::DATA, 0, 10, 100).await,
            "find_by_kind should succeed",
        );
        assert_eq!(found.len(), 2);
    }
}
