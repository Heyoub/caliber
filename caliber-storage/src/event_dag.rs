//! In-memory EventDag implementation for testing.
//!
//! This module provides a simple in-memory implementation of the EventDag trait
//! suitable for unit tests and development scenarios.

use caliber_core::{
    DagPosition, DomainError, DomainErrorContext, Effect, ErrorEffect, Event, EventDag, EventFlags,
    EventId, EventKind, UpstreamSignal,
};
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
    pub fn len(&self) -> usize {
        self.events.read().unwrap().len()
    }

    /// Check if the DAG is empty.
    pub fn is_empty(&self) -> bool {
        self.events.read().unwrap().is_empty()
    }

    /// Clear all events from the DAG.
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
        self.acknowledged.write().unwrap().clear();
        *self.next_sequence.write().unwrap() = 0;
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
        let mut events = self.events.write().unwrap();
        let mut seq = self.next_sequence.write().unwrap();

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
        let events = self.events.read().unwrap();
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
        let events = self.events.read().unwrap();

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
        let events = self.events.read().unwrap();

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
        let from_event = events.get(&from).unwrap();
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
        let events = self.events.read().unwrap();

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
        let events = self.events.read().unwrap();

        let mut result: Vec<Event<Self::Payload>> = events
            .values()
            .filter(|e| e.header.correlation_id == correlation_id)
            .cloned()
            .collect();

        result.sort_by_key(|e| e.header.position.sequence);
        Effect::Ok(result)
    }

    async fn next_position(&self, parent: Option<EventId>, lane: u32) -> Effect<DagPosition> {
        let events = self.events.read().unwrap();
        let seq = self.next_sequence.read().unwrap();

        let depth = match parent {
            Some(parent_id) => {
                match events.get(&parent_id) {
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
                }
            }
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
        let events = self.events.read().unwrap();

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
        let events = self.events.read().unwrap();

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

        self.acknowledged.write().unwrap().insert(event_id);
        Effect::Ok(())
    }

    async fn unacknowledged(&self, limit: usize) -> Effect<Vec<Event<Self::Payload>>> {
        let events = self.events.read().unwrap();
        let acknowledged = self.acknowledged.read().unwrap();

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

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{EventDagExt, EventHeader};

    #[tokio::test]
    async fn test_append_and_read() {
        let dag: InMemoryEventDag<String> = InMemoryEventDag::new();

        let event_id = dag.append_root("test payload".to_string()).await;
        assert!(event_id.is_ok());

        let id = event_id.unwrap();
        let read_result = dag.read(id).await;
        assert!(read_result.is_ok());

        let event = read_result.unwrap();
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

        let root_id = dag.append_root("root".to_string()).await.unwrap();
        let child_id = dag.append_child(root_id, "child".to_string()).await;

        assert!(child_id.is_ok());
        let child = dag.read(child_id.unwrap()).await.unwrap();
        assert_eq!(child.header.position.depth, 1);
    }

    #[tokio::test]
    async fn test_acknowledge() {
        let dag: InMemoryEventDag<String> = InMemoryEventDag::new();

        // Create event requiring ack
        let position = dag.next_position(None, 0).await.unwrap();
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
        };
        let event_id = dag.append(event).await.unwrap();

        // Should appear in unacknowledged
        let unacked = dag.unacknowledged(10).await.unwrap();
        assert_eq!(unacked.len(), 1);

        // Acknowledge it
        dag.acknowledge(event_id, false).await.unwrap();

        // Should no longer appear
        let unacked = dag.unacknowledged(10).await.unwrap();
        assert_eq!(unacked.len(), 0);
    }

    #[tokio::test]
    async fn test_find_by_kind() {
        let dag: InMemoryEventDag<String> = InMemoryEventDag::new();

        // Create a few events
        dag.append_root("event1".to_string()).await.unwrap();
        dag.append_root("event2".to_string()).await.unwrap();

        let found = dag.find_by_kind(EventKind::DATA, 0, 10, 100).await.unwrap();
        assert_eq!(found.len(), 2);
    }
}
