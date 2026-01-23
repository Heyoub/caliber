//! Event types for the Event DAG system.
//!
//! The Event DAG provides unidirectional event flow with upstream signaling
//! (the "tram car tracks" pattern).
//!
//! # Event Header
//!
//! The `EventHeader` is a 64-byte cache-aligned structure for optimal memory access.
//! It contains all metadata needed to process an event without accessing the payload.

use crate::EntityId;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// EVENT IDENTIFICATION
// ============================================================================

/// Unique identifier for an event (same as EntityId but semantically distinct).
pub type EventId = EntityId;

// ============================================================================
// DAG POSITION
// ============================================================================

/// Position of an event in the DAG.
///
/// The DAG position uniquely identifies where an event sits in the event graph.
/// - `depth`: Distance from the root event (0 = root)
/// - `lane`: Parallel track for fan-out scenarios
/// - `sequence`: Monotonic counter within a lane
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[repr(C)]
pub struct DagPosition {
    /// Distance from the root event (0 = root event)
    pub depth: u32,
    /// Parallel track number for fan-out (0 = main track)
    pub lane: u32,
    /// Monotonic sequence number within the lane
    pub sequence: u32,
}

impl DagPosition {
    /// Create a new DAG position.
    pub const fn new(depth: u32, lane: u32, sequence: u32) -> Self {
        Self { depth, lane, sequence }
    }

    /// Create the root position.
    pub const fn root() -> Self {
        Self::new(0, 0, 0)
    }

    /// Create a child position on the same lane.
    pub const fn child(&self, sequence: u32) -> Self {
        Self::new(self.depth + 1, self.lane, sequence)
    }

    /// Create a position on a new lane (fork).
    pub const fn fork(&self, new_lane: u32, sequence: u32) -> Self {
        Self::new(self.depth + 1, new_lane, sequence)
    }

    /// Check if this position is an ancestor of another.
    pub const fn is_ancestor_of(&self, other: &Self) -> bool {
        self.depth < other.depth && self.lane == other.lane
    }

    /// Check if this is the root position.
    pub const fn is_root(&self) -> bool {
        self.depth == 0
    }
}

impl Default for DagPosition {
    fn default() -> Self {
        Self::root()
    }
}

impl fmt::Display for DagPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.depth, self.lane, self.sequence)
    }
}

// ============================================================================
// EVENT KIND
// ============================================================================

/// Event kind encoded as a 16-bit value.
///
/// The upper 4 bits encode the category, the lower 12 bits encode the specific type.
/// This allows 16 categories with 4096 event types each (65536 total).
///
/// Category allocation:
/// - 0x0xxx: System events
/// - 0x1xxx: Trajectory events
/// - 0x2xxx: Scope events
/// - 0x3xxx: Artifact events
/// - 0x4xxx: Note events
/// - 0x5xxx: Turn events
/// - 0x6xxx: Agent events
/// - 0x7xxx: Lock events
/// - 0x8xxx: Message events
/// - 0x9xxx: Delegation events
/// - 0xAxxx: Handoff events
/// - 0xBxxx: Edge events
/// - 0xCxxx: Evolution events
/// - 0xDxxx: Effect events (errors, compensations)
/// - 0xExxx: Reserved
/// - 0xFxxx: Custom/extension events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[repr(transparent)]
pub struct EventKind(pub u16);

impl EventKind {
    // System events (0x0xxx)
    pub const SYSTEM_INIT: Self = Self(0x0001);
    pub const SYSTEM_SHUTDOWN: Self = Self(0x0002);
    pub const SYSTEM_HEARTBEAT: Self = Self(0x0003);
    pub const SYSTEM_CONFIG_CHANGE: Self = Self(0x0004);

    // Trajectory events (0x1xxx)
    pub const TRAJECTORY_CREATED: Self = Self(0x1001);
    pub const TRAJECTORY_UPDATED: Self = Self(0x1002);
    pub const TRAJECTORY_COMPLETED: Self = Self(0x1003);
    pub const TRAJECTORY_FAILED: Self = Self(0x1004);
    pub const TRAJECTORY_SUSPENDED: Self = Self(0x1005);
    pub const TRAJECTORY_RESUMED: Self = Self(0x1006);
    pub const TRAJECTORY_DELETED: Self = Self(0x1007);

    // Scope events (0x2xxx)
    pub const SCOPE_CREATED: Self = Self(0x2001);
    pub const SCOPE_UPDATED: Self = Self(0x2002);
    pub const SCOPE_CLOSED: Self = Self(0x2003);
    pub const SCOPE_CHECKPOINTED: Self = Self(0x2004);

    // Artifact events (0x3xxx)
    pub const ARTIFACT_CREATED: Self = Self(0x3001);
    pub const ARTIFACT_UPDATED: Self = Self(0x3002);
    pub const ARTIFACT_SUPERSEDED: Self = Self(0x3003);
    pub const ARTIFACT_DELETED: Self = Self(0x3004);

    // Note events (0x4xxx)
    pub const NOTE_CREATED: Self = Self(0x4001);
    pub const NOTE_UPDATED: Self = Self(0x4002);
    pub const NOTE_SUPERSEDED: Self = Self(0x4003);
    pub const NOTE_DELETED: Self = Self(0x4004);
    pub const NOTE_ACCESSED: Self = Self(0x4005);

    // Turn events (0x5xxx)
    pub const TURN_CREATED: Self = Self(0x5001);

    // Agent events (0x6xxx)
    pub const AGENT_REGISTERED: Self = Self(0x6001);
    pub const AGENT_UPDATED: Self = Self(0x6002);
    pub const AGENT_UNREGISTERED: Self = Self(0x6003);
    pub const AGENT_STATUS_CHANGED: Self = Self(0x6004);

    // Lock events (0x7xxx)
    pub const LOCK_ACQUIRED: Self = Self(0x7001);
    pub const LOCK_EXTENDED: Self = Self(0x7002);
    pub const LOCK_RELEASED: Self = Self(0x7003);
    pub const LOCK_EXPIRED: Self = Self(0x7004);
    pub const LOCK_CONTENTION: Self = Self(0x7005);

    // Message events (0x8xxx)
    pub const MESSAGE_SENT: Self = Self(0x8001);
    pub const MESSAGE_DELIVERED: Self = Self(0x8002);
    pub const MESSAGE_ACKNOWLEDGED: Self = Self(0x8003);
    pub const MESSAGE_EXPIRED: Self = Self(0x8004);

    // Delegation events (0x9xxx)
    pub const DELEGATION_CREATED: Self = Self(0x9001);
    pub const DELEGATION_ACCEPTED: Self = Self(0x9002);
    pub const DELEGATION_REJECTED: Self = Self(0x9003);
    pub const DELEGATION_STARTED: Self = Self(0x9004);
    pub const DELEGATION_COMPLETED: Self = Self(0x9005);
    pub const DELEGATION_FAILED: Self = Self(0x9006);

    // Handoff events (0xAxxx)
    pub const HANDOFF_CREATED: Self = Self(0xA001);
    pub const HANDOFF_ACCEPTED: Self = Self(0xA002);
    pub const HANDOFF_REJECTED: Self = Self(0xA003);
    pub const HANDOFF_COMPLETED: Self = Self(0xA004);

    // Edge events (0xBxxx)
    pub const EDGE_CREATED: Self = Self(0xB001);
    pub const EDGE_UPDATED: Self = Self(0xB002);
    pub const EDGE_DELETED: Self = Self(0xB003);

    // Evolution events (0xCxxx)
    pub const EVOLUTION_SNAPSHOT_CREATED: Self = Self(0xC001);
    pub const EVOLUTION_PHASE_CHANGED: Self = Self(0xC002);

    // Effect events (0xDxxx)
    pub const EFFECT_ERROR: Self = Self(0xD001);
    pub const EFFECT_RETRY: Self = Self(0xD002);
    pub const EFFECT_COMPENSATE: Self = Self(0xD003);
    pub const EFFECT_ACK: Self = Self(0xD004);
    pub const EFFECT_BACKPRESSURE: Self = Self(0xD005);
    pub const EFFECT_CANCEL: Self = Self(0xD006);

    /// Get the category (upper 4 bits).
    pub const fn category(&self) -> u8 {
        (self.0 >> 12) as u8
    }

    /// Get the type within the category (lower 12 bits).
    pub const fn type_id(&self) -> u16 {
        self.0 & 0x0FFF
    }

    /// Create a custom event kind.
    pub const fn custom(category: u8, type_id: u16) -> Self {
        Self(((category as u16) << 12) | (type_id & 0x0FFF))
    }

    /// Check if this is a system event.
    pub const fn is_system(&self) -> bool {
        self.category() == 0
    }

    /// Check if this is an effect event.
    pub const fn is_effect(&self) -> bool {
        self.category() == 0xD
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#06X}", self.0)
    }
}

// ============================================================================
// EVENT FLAGS
// ============================================================================

bitflags! {
    /// Flags for event processing hints.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct EventFlags: u8 {
        /// Event requires acknowledgment
        const REQUIRES_ACK = 0b0000_0001;
        /// Event is part of a transaction
        const TRANSACTIONAL = 0b0000_0010;
        /// Event payload is compressed
        const COMPRESSED = 0b0000_0100;
        /// Event is a replay (not original)
        const REPLAY = 0b0000_1000;
        /// Event has been acknowledged
        const ACKNOWLEDGED = 0b0001_0000;
        /// Event triggered compensation
        const COMPENSATED = 0b0010_0000;
        /// Event is critical (must be processed)
        const CRITICAL = 0b0100_0000;
        /// Reserved for future use
        const RESERVED = 0b1000_0000;
    }
}

impl Default for EventFlags {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// EVENT HEADER (64-byte cache-aligned)
// ============================================================================

/// Event header with all metadata needed for processing.
///
/// This structure is 64 bytes and cache-line aligned for optimal performance.
/// The payload is stored separately and referenced by the header.
///
/// Layout (64 bytes total):
/// - event_id: 16 bytes (UUIDv7)
/// - position: 12 bytes (DagPosition)
/// - timestamp: 8 bytes (microseconds since epoch)
/// - event_kind: 2 bytes (EventKind)
/// - correlation_id: 16 bytes (UUIDv7)
/// - flags: 1 byte (EventFlags)
/// - payload_size: 4 bytes (u32)
/// - _reserved: 5 bytes (padding for alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[repr(C, align(64))]
pub struct EventHeader {
    /// Unique event identifier (UUIDv7 for timestamp-sortable IDs)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub event_id: EventId,
    /// Position in the event DAG
    pub position: DagPosition,
    /// Timestamp in microseconds since Unix epoch
    pub timestamp: i64,
    /// Event kind (category + type)
    pub event_kind: EventKind,
    /// Correlation ID for tracing related events
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub correlation_id: EventId,
    /// Processing flags
    pub flags: EventFlags,
    /// Size of the payload in bytes
    pub payload_size: u32,
    /// Reserved for future use (padding)
    #[serde(skip)]
    _reserved: [u8; 5],
}

// Compile-time check that EventHeader is exactly 64 bytes
const _: () = assert!(std::mem::size_of::<EventHeader>() == 64);
const _: () = assert!(std::mem::align_of::<EventHeader>() == 64);

impl EventHeader {
    /// Create a new event header.
    pub fn new(
        event_id: EventId,
        position: DagPosition,
        timestamp: i64,
        event_kind: EventKind,
        correlation_id: EventId,
        flags: EventFlags,
        payload_size: u32,
    ) -> Self {
        Self {
            event_id,
            position,
            timestamp,
            event_kind,
            correlation_id,
            flags,
            payload_size,
            _reserved: [0; 5],
        }
    }

    /// Check if this event requires acknowledgment.
    pub fn requires_ack(&self) -> bool {
        self.flags.contains(EventFlags::REQUIRES_ACK)
    }

    /// Check if this event is part of a transaction.
    pub fn is_transactional(&self) -> bool {
        self.flags.contains(EventFlags::TRANSACTIONAL)
    }

    /// Check if this event is a replay.
    pub fn is_replay(&self) -> bool {
        self.flags.contains(EventFlags::REPLAY)
    }

    /// Check if this event has been acknowledged.
    pub fn is_acknowledged(&self) -> bool {
        self.flags.contains(EventFlags::ACKNOWLEDGED)
    }

    /// Check if this event is critical.
    pub fn is_critical(&self) -> bool {
        self.flags.contains(EventFlags::CRITICAL)
    }

    /// Mark this event as acknowledged.
    pub fn acknowledge(&mut self) {
        self.flags |= EventFlags::ACKNOWLEDGED;
    }

    /// Get the event age in microseconds from a reference time.
    pub fn age_micros(&self, now_micros: i64) -> i64 {
        now_micros - self.timestamp
    }
}

// ============================================================================
// FULL EVENT (Header + Payload)
// ============================================================================

/// A complete event with header and payload.
///
/// The payload is generic to allow different event types to have different
/// payload structures while sharing the same header format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Event<P> {
    pub header: EventHeader,
    pub payload: P,
}

impl<P> Event<P> {
    /// Create a new event with the given header and payload.
    pub fn new(header: EventHeader, payload: P) -> Self {
        Self { header, payload }
    }

    /// Get the event ID.
    pub fn event_id(&self) -> EventId {
        self.header.event_id
    }

    /// Get the event kind.
    pub fn event_kind(&self) -> EventKind {
        self.header.event_kind
    }

    /// Get the correlation ID.
    pub fn correlation_id(&self) -> EventId {
        self.header.correlation_id
    }

    /// Get the DAG position.
    pub fn position(&self) -> DagPosition {
        self.header.position
    }

    /// Map the payload to a different type.
    pub fn map_payload<Q, F: FnOnce(P) -> Q>(self, f: F) -> Event<Q> {
        Event {
            header: self.header,
            payload: f(self.payload),
        }
    }
}

// ============================================================================
// UPSTREAM SIGNALS
// ============================================================================

/// Signals that can be sent upstream in the DAG ("tram car tracks").
///
/// These signals flow in the opposite direction to events, allowing
/// downstream processors to communicate back to upstream producers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum UpstreamSignal {
    /// Acknowledge receipt/processing of an event
    Ack {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        event_id: EventId,
    },
    /// Request backpressure (slow down event production)
    Backpressure {
        /// Resume timestamp in microseconds
        until: i64,
    },
    /// Cancel a correlation chain
    Cancel {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        correlation_id: EventId,
        reason: String,
    },
    /// Signal that compensation is complete
    CompensationComplete {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        event_id: EventId,
    },
    /// Propagate an error upstream
    ErrorPropagation {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        source_event_id: EventId,
        error_code: String,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_event_header_size() {
        assert_eq!(std::mem::size_of::<EventHeader>(), 64);
        assert_eq!(std::mem::align_of::<EventHeader>(), 64);
    }

    #[test]
    fn test_dag_position() {
        let root = DagPosition::root();
        assert!(root.is_root());
        assert_eq!(root.depth, 0);

        let child = root.child(1);
        assert!(!child.is_root());
        assert_eq!(child.depth, 1);
        assert!(root.is_ancestor_of(&child));

        let fork = root.fork(1, 0);
        assert_eq!(fork.lane, 1);
        assert!(!root.is_ancestor_of(&fork)); // Different lane
    }

    #[test]
    fn test_event_kind_categories() {
        assert!(EventKind::SYSTEM_INIT.is_system());
        assert!(!EventKind::TRAJECTORY_CREATED.is_system());
        assert!(EventKind::EFFECT_ERROR.is_effect());

        let custom = EventKind::custom(0xF, 0x123);
        assert_eq!(custom.category(), 0xF);
        assert_eq!(custom.type_id(), 0x123);
    }

    #[test]
    fn test_event_flags() {
        let mut flags = EventFlags::REQUIRES_ACK | EventFlags::CRITICAL;
        assert!(flags.contains(EventFlags::REQUIRES_ACK));
        assert!(flags.contains(EventFlags::CRITICAL));
        assert!(!flags.contains(EventFlags::REPLAY));

        flags |= EventFlags::ACKNOWLEDGED;
        assert!(flags.contains(EventFlags::ACKNOWLEDGED));
    }

    #[test]
    fn test_event_header_creation() {
        let event_id = Uuid::now_v7();
        let correlation_id = Uuid::now_v7();
        let header = EventHeader::new(
            event_id,
            DagPosition::root(),
            1234567890,
            EventKind::TRAJECTORY_CREATED,
            correlation_id,
            EventFlags::REQUIRES_ACK,
            100,
        );

        assert_eq!(header.event_id, event_id);
        assert!(header.requires_ack());
        assert!(!header.is_acknowledged());
    }
}
