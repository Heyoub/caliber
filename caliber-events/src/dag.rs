//! Event DAG trait for persistent event storage and traversal.
//!
//! The Event DAG provides unidirectional event flow with upstream signaling
//! (the "tram car tracks" pattern).
//!
//! # Architecture
//!
//! Events flow forward (downstream) through the DAG, while signals can flow
//! backward (upstream) to communicate acknowledgments, backpressure, and errors.
//!
//! ```text
//! Events:   Root → Event1 → Event2 → Event3
//!                     ↑         ↑        ↑
//! Signals:  ← Ack ← Ack ← Backpressure
//! ```

use caliber_core::{
    DagPosition, Effect, EntityId, Event, EventHeader, EventId, EventKind, UpstreamSignal,
};

/// Trait for Event DAG operations.
///
/// Implementations of this trait provide persistent storage and traversal
/// of the event graph. The DAG supports:
///
/// - Appending new events with automatic position calculation
/// - Reading events by ID
/// - Walking ancestor chains for context reconstruction
/// - Sending upstream signals for coordination
/// - Correlation chain traversal for related event discovery
///
/// # Payload Type
///
/// The `Payload` associated type determines what data is stored with each event.
/// This is typically a serializable enum of all possible event payloads.
pub trait EventDag {
    /// The payload type stored with events.
    type Payload: Clone + Send + Sync;

    /// Append a new event to the DAG.
    ///
    /// The event's position should be set by the caller based on the parent event.
    /// Returns the assigned event ID on success.
    ///
    /// # Errors
    ///
    /// Returns an error effect if:
    /// - The position is invalid (e.g., sequence collision)
    /// - Storage fails
    fn append(&self, event: Event<Self::Payload>) -> Effect<EventId>;

    /// Read an event by its ID.
    ///
    /// # Errors
    ///
    /// Returns an error effect if:
    /// - The event doesn't exist
    /// - Storage read fails
    fn read(&self, event_id: EventId) -> Effect<Event<Self::Payload>>;

    /// Walk the ancestor chain from a given event.
    ///
    /// Returns events from `from` toward the root, limited by `limit`.
    /// The events are returned in order from most recent to oldest.
    ///
    /// # Arguments
    ///
    /// * `from` - Starting event ID
    /// * `limit` - Maximum number of ancestors to return
    ///
    /// # Errors
    ///
    /// Returns an error effect if the starting event doesn't exist.
    fn walk_ancestors(
        &self,
        from: EventId,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>>;

    /// Walk descendants from a given event.
    ///
    /// Returns events that descend from `from`, limited by `limit`.
    ///
    /// # Arguments
    ///
    /// * `from` - Starting event ID
    /// * `limit` - Maximum number of descendants to return
    fn walk_descendants(
        &self,
        from: EventId,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>>;

    /// Send an upstream signal from a downstream event.
    ///
    /// Signals propagate backward through the DAG to notify upstream
    /// producers of acknowledgments, backpressure, or errors.
    ///
    /// # Arguments
    ///
    /// * `from` - The event ID sending the signal
    /// * `signal` - The signal to send
    ///
    /// # Errors
    ///
    /// Returns an error effect if the source event doesn't exist.
    fn signal_upstream(&self, from: EventId, signal: UpstreamSignal) -> Effect<()>;

    /// Find all events in a correlation chain.
    ///
    /// Returns all events that share the same correlation ID,
    /// ordered by their DAG position.
    ///
    /// # Arguments
    ///
    /// * `correlation_id` - The correlation ID to search for
    ///
    /// # Errors
    ///
    /// Returns an error effect if storage read fails.
    fn find_correlation_chain(
        &self,
        correlation_id: EventId,
    ) -> Effect<Vec<Event<Self::Payload>>>;

    /// Get the current position for appending a new event.
    ///
    /// This is useful for calculating the next position in a lane
    /// without actually appending an event.
    ///
    /// # Arguments
    ///
    /// * `parent` - Optional parent event ID (None for root events)
    /// * `lane` - The lane to append to
    fn next_position(&self, parent: Option<EventId>, lane: u32) -> Effect<DagPosition>;

    /// Get events by kind within a position range.
    ///
    /// Useful for finding all events of a specific type within a time window.
    ///
    /// # Arguments
    ///
    /// * `kind` - The event kind to filter by
    /// * `min_depth` - Minimum depth in the DAG
    /// * `max_depth` - Maximum depth in the DAG
    /// * `limit` - Maximum number of events to return
    fn find_by_kind(
        &self,
        kind: EventKind,
        min_depth: u32,
        max_depth: u32,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>>;

    /// Acknowledge an event.
    ///
    /// Marks the event as acknowledged and optionally sends an upstream
    /// acknowledgment signal.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The event to acknowledge
    /// * `send_upstream` - Whether to propagate the ack upstream
    fn acknowledge(&self, event_id: EventId, send_upstream: bool) -> Effect<()>;

    /// Get unacknowledged events that require acknowledgment.
    ///
    /// Returns events that have the REQUIRES_ACK flag set but haven't
    /// been acknowledged yet.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of events to return
    fn unacknowledged(&self, limit: usize) -> Effect<Vec<Event<Self::Payload>>>;
}

/// Extension trait for EventDag with convenience methods.
///
/// This trait provides default implementations that delegate to the base
/// `EventDag` methods. Implementors get these methods for free by implementing
/// `EventDag`.
pub trait EventDagExt: EventDag {
    /// Append a new root event.
    ///
    /// Creates an event at the root position (depth 0, lane 0).
    fn append_root(&self, payload: Self::Payload) -> Effect<EventId> {
        let position = match self.next_position(None, 0) {
            Effect::Ok(pos) => pos,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        let event = Event {
            header: EventHeader::new(EventKind::Data, position),
            payload,
        };
        self.append(event)
    }

    /// Append a child event to an existing event.
    ///
    /// The child is placed at depth + 1 in the same lane as the parent.
    fn append_child(&self, parent: EventId, payload: Self::Payload) -> Effect<EventId> {
        let parent_event = match self.read(parent) {
            Effect::Ok(e) => e,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        let position = match self.next_position(Some(parent), parent_event.header.position.lane) {
            Effect::Ok(pos) => pos,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        let event = Event {
            header: EventHeader::new(EventKind::Data, position),
            payload,
        };
        self.append(event)
    }

    /// Fork a new lane from an existing event.
    ///
    /// Creates a new event in a new lane at depth + 1.
    fn fork(&self, parent: EventId, new_lane: u32, payload: Self::Payload) -> Effect<EventId> {
        let position = match self.next_position(Some(parent), new_lane) {
            Effect::Ok(pos) => pos,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        let event = Event {
            header: EventHeader::new(EventKind::Data, position),
            payload,
        };
        self.append(event)
    }

    /// Get the depth of an event in the DAG.
    fn depth(&self, event_id: EventId) -> Effect<u32> {
        match self.read(event_id) {
            Effect::Ok(event) => Effect::Ok(event.header.position.depth),
            Effect::Err(e) => Effect::Err(e),
            other => other.map(|_| unreachable!()),
        }
    }

    /// Check if one event is an ancestor of another.
    fn is_ancestor(&self, ancestor: EventId, descendant: EventId) -> Effect<bool> {
        // Walk ancestors from descendant looking for ancestor
        let ancestors = match self.walk_ancestors(descendant, 1000) {
            Effect::Ok(events) => events,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        for event in ancestors {
            if event.header.id == ancestor {
                return Effect::Ok(true);
            }
        }
        Effect::Ok(false)
    }
}

// Blanket implementation: any type implementing EventDag automatically gets EventDagExt
impl<T: EventDag> EventDagExt for T {}

/// Async version of the EventDag trait.
///
/// For async contexts, use this trait instead of EventDag.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncEventDag {
    type Payload: Clone + Send + Sync;

    async fn append(&self, event: Event<Self::Payload>) -> Effect<EventId>;
    async fn read(&self, event_id: EventId) -> Effect<Event<Self::Payload>>;
    async fn walk_ancestors(&self, from: EventId, limit: usize) -> Effect<Vec<Event<Self::Payload>>>;
    async fn walk_descendants(&self, from: EventId, limit: usize) -> Effect<Vec<Event<Self::Payload>>>;
    async fn signal_upstream(&self, from: EventId, signal: UpstreamSignal) -> Effect<()>;
    async fn find_correlation_chain(&self, correlation_id: EventId) -> Effect<Vec<Event<Self::Payload>>>;
    async fn next_position(&self, parent: Option<EventId>, lane: u32) -> Effect<DagPosition>;
    async fn find_by_kind(&self, kind: EventKind, min_depth: u32, max_depth: u32, limit: usize) -> Effect<Vec<Event<Self::Payload>>>;
    async fn acknowledge(&self, event_id: EventId, send_upstream: bool) -> Effect<()>;
    async fn unacknowledged(&self, limit: usize) -> Effect<Vec<Event<Self::Payload>>>;
}

/// Builder for creating events with proper positioning.
#[derive(Debug, Clone)]
pub struct EventBuilder<P> {
    parent: Option<EventId>,
    lane: u32,
    correlation_id: Option<EventId>,
    payload: P,
    flags: caliber_core::EventFlags,
}

impl<P> EventBuilder<P> {
    /// Create a new event builder with the given payload.
    pub fn new(payload: P) -> Self {
        Self {
            parent: None,
            lane: 0,
            correlation_id: None,
            payload,
            flags: caliber_core::EventFlags::empty(),
        }
    }

    /// Set the parent event.
    pub fn parent(mut self, parent: EventId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set the lane.
    pub fn lane(mut self, lane: u32) -> Self {
        self.lane = lane;
        self
    }

    /// Set the correlation ID.
    pub fn correlation(mut self, correlation_id: EventId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Mark this event as requiring acknowledgment.
    pub fn requires_ack(mut self) -> Self {
        self.flags |= caliber_core::EventFlags::REQUIRES_ACK;
        self
    }

    /// Mark this event as critical.
    pub fn critical(mut self) -> Self {
        self.flags |= caliber_core::EventFlags::CRITICAL;
        self
    }

    /// Mark this event as transactional.
    pub fn transactional(mut self) -> Self {
        self.flags |= caliber_core::EventFlags::TRANSACTIONAL;
        self
    }

    /// Get the configured parent.
    pub fn get_parent(&self) -> Option<EventId> {
        self.parent
    }

    /// Get the configured lane.
    pub fn get_lane(&self) -> u32 {
        self.lane
    }

    /// Get the configured flags.
    pub fn get_flags(&self) -> caliber_core::EventFlags {
        self.flags
    }

    /// Get the correlation ID (or None).
    pub fn get_correlation_id(&self) -> Option<EventId> {
        self.correlation_id
    }

    /// Consume the builder and return the payload.
    pub fn into_payload(self) -> P {
        self.payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder() {
        let builder = EventBuilder::new("test payload")
            .lane(1)
            .requires_ack()
            .critical();

        assert_eq!(builder.get_lane(), 1);
        assert!(builder.get_flags().contains(caliber_core::EventFlags::REQUIRES_ACK));
        assert!(builder.get_flags().contains(caliber_core::EventFlags::CRITICAL));
    }
}
