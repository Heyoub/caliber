//! Event types for the Event DAG system.
//!
//! The Event DAG provides unidirectional event flow with upstream signaling
//! (the "tram car tracks" pattern).
//!
//! # Event Header
//!
//! The `EventHeader` is a 64-byte cache-aligned structure for optimal memory access.
//! It contains all metadata needed to process an event without accessing the payload.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ============================================================================
// EVENT IDENTIFICATION
// ============================================================================

/// Unique identifier for an event.
pub type EventId = Uuid;

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
        Self {
            depth,
            lane,
            sequence,
        }
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
    // Generic data event (neutral/unspecified kind)
    pub const DATA: Self = Self(0x0000);

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

    // Cache invalidation events (0xExxx)
    pub const CACHE_INVALIDATE_TRAJECTORY: Self = Self(0xE001);
    pub const CACHE_INVALIDATE_SCOPE: Self = Self(0xE002);
    pub const CACHE_INVALIDATE_ARTIFACT: Self = Self(0xE003);
    pub const CACHE_INVALIDATE_NOTE: Self = Self(0xE004);

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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

// Manual serde implementation for EventFlags (bitflags 2.x + serde)
impl Serialize for EventFlags {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EventFlags {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u8::deserialize(deserializer)?;
        Self::from_bits(bits).ok_or_else(|| {
            serde::de::Error::custom(format!("invalid EventFlags bits: {:#04x}", bits))
        })
    }
}

#[cfg(feature = "openapi")]
impl utoipa::ToSchema for EventFlags {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("EventFlags")
    }
}

#[cfg(feature = "openapi")]
impl utoipa::PartialSchema for EventFlags {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::SchemaType::Type(
                utoipa::openapi::schema::Type::Integer,
            ))
            .description(Some("Event processing flags as a u8 bitfield (0-255)"))
            .minimum(Some(0.0))
            .maximum(Some(255.0))
            .into()
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
/// Layout (64 bytes total, ordered by alignment to minimize padding):
/// - event_id: 16 bytes (UUIDv7)
/// - correlation_id: 16 bytes (UUIDv7)
/// - timestamp: 8 bytes (microseconds since epoch)
/// - position: 12 bytes (DagPosition)
/// - payload_size: 4 bytes (u32)
/// - event_kind: 2 bytes (EventKind)
/// - flags: 1 byte (EventFlags)
/// - _reserved: 5 bytes (padding to 64)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[repr(C, align(64))]
pub struct EventHeader {
    /// Unique event identifier (UUIDv7 for timestamp-sortable IDs)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub event_id: EventId,
    /// Correlation ID for tracing related events
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub correlation_id: EventId,
    /// Timestamp in microseconds since Unix epoch
    pub timestamp: i64,
    /// Position in the event DAG
    pub position: DagPosition,
    /// Size of the payload in bytes
    pub payload_size: u32,
    /// Event kind (category + type)
    pub event_kind: EventKind,
    /// Processing flags
    pub flags: EventFlags,
    /// Reserved for future use (padding to 64 bytes)
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
        correlation_id: EventId,
        timestamp: i64,
        position: DagPosition,
        payload_size: u32,
        event_kind: EventKind,
        flags: EventFlags,
    ) -> Self {
        Self {
            event_id,
            correlation_id,
            timestamp,
            position,
            payload_size,
            event_kind,
            flags,
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
    /// Hash chain for tamper-evident audit trail (Blake3 hash of parent + self).
    /// None for genesis events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_chain: Option<HashChain>,
}

impl<P> Event<P> {
    /// Create a new event with the given header and payload (genesis event, no hash chain).
    pub fn new(header: EventHeader, payload: P) -> Self {
        Self {
            header,
            payload,
            hash_chain: None,
        }
    }

    /// Create a new event with the given header, payload, and hash chain.
    pub fn with_hash_chain(header: EventHeader, payload: P, hash_chain: HashChain) -> Self {
        Self {
            header,
            payload,
            hash_chain: Some(hash_chain),
        }
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
            hash_chain: self.hash_chain,
        }
    }
}

impl<P: Serialize> Event<P> {
    /// Compute Blake3 hash of this event (canonical JSON serialization).
    pub fn compute_hash(&self) -> [u8; 32] {
        let canonical = serde_json::to_vec(self).unwrap_or_default();
        blake3::hash(&canonical).into()
    }

    /// Verify this event against a parent hash using Blake3.
    /// Returns true for genesis events (no hash chain).
    pub fn verify(&self, parent_hash: &[u8; 32]) -> bool {
        Blake3Verifier.verify_chain(self, parent_hash)
    }

    /// Check if this is a genesis event (no parent).
    pub fn is_genesis(&self) -> bool {
        self.hash_chain.is_none()
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

// ============================================================================
// HASH CHAINING FOR AUDIT INTEGRITY (Phase 1.1)
// ============================================================================

/// Hash algorithm for event integrity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum HashAlgorithm {
    /// SHA-256 hash algorithm
    Sha256,
    /// Blake3 hash algorithm (faster, recommended)
    #[default]
    Blake3,
}

/// Hash chain for tamper-evident event log.
///
/// Each event in the chain contains a hash of the previous event,
/// creating an immutable, verifiable audit trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HashChain {
    /// Hash of previous event (creates chain). Genesis event has zero prev_hash.
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub prev_hash: [u8; 32],
    /// Hash of this event (canonical serialization)
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub event_hash: [u8; 32],
    /// Algorithm used for hashing
    pub algorithm: HashAlgorithm,
}

impl Default for HashChain {
    fn default() -> Self {
        Self {
            prev_hash: [0u8; 32], // Genesis event has zero prev_hash
            event_hash: [0u8; 32],
            algorithm: HashAlgorithm::Blake3,
        }
    }
}

// ============================================================================
// CAUSALITY TRACKING (Phase 1.2)
// ============================================================================

/// W3C Trace Context compatible distributed tracing.
///
/// Provides full causality tracking for events across distributed systems.
/// Compatible with OpenTelemetry and other distributed tracing systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Causality {
    /// Stable trace ID across request lifecycle (W3C compatible)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trace_id: Uuid,
    /// Current operation span ID
    pub span_id: u64,
    /// Parent span (if not root)
    pub parent_span_id: Option<u64>,
    /// Parent event IDs for causality fan-in (multiple parents)
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub parent_event_ids: Vec<EventId>,
    /// Root event of this trace tree
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub root_event_id: EventId,
}

impl Default for Causality {
    fn default() -> Self {
        let root_id = Uuid::now_v7();
        Self {
            trace_id: Uuid::now_v7(),
            span_id: 0,
            parent_span_id: None,
            parent_event_ids: Vec::new(),
            root_event_id: root_id,
        }
    }
}

impl Causality {
    /// Create a new causality context with a fresh trace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a child span from this causality context.
    pub fn child(&self, new_span_id: u64) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: new_span_id,
            parent_span_id: Some(self.span_id),
            parent_event_ids: vec![self.root_event_id],
            root_event_id: self.root_event_id,
        }
    }

    /// Create a causality context with multiple parents (fan-in).
    pub fn merge(parents: &[&Causality], new_span_id: u64) -> Option<Self> {
        let first = parents.first()?;
        Some(Self {
            trace_id: first.trace_id,
            span_id: new_span_id,
            parent_span_id: Some(first.span_id),
            parent_event_ids: parents.iter().map(|p| p.root_event_id).collect(),
            root_event_id: first.root_event_id,
        })
    }
}

// ============================================================================
// RICH EVIDENCE REFERENCES (Phase 1.3)
// ============================================================================

use crate::{AgentId, ArtifactId, ExtractionMethod, NoteId, Timestamp};

/// Evidence reference types for provenance tracking.
///
/// Rich references to external sources that support claims or data in an event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum EvidenceRef {
    /// Reference to a document chunk
    DocChunk {
        doc_id: String,
        chunk_id: String,
        offset: u32,
        length: u32,
    },
    /// Reference to another event
    Event {
        #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
        event_id: EventId,
        timestamp: i64,
    },
    /// Reference to a tool call result
    ToolResult {
        call_id: String,
        tool_name: String,
        result_index: u32,
    },
    /// Reference to external URL
    Url {
        url: String,
        accessed_at: i64,
        #[cfg_attr(feature = "openapi", schema(value_type = Option<String>))]
        hash: Option<[u8; 32]>,
    },
    /// Reference to knowledge pack section
    KnowledgePack { pack_id: String, section: String },
    /// Reference to artifact
    Artifact { artifact_id: ArtifactId },
    /// Reference to note
    Note { note_id: NoteId },
    /// Manual user-provided evidence
    Manual {
        user_id: String,
        timestamp: i64,
        description: String,
    },
}

/// Verification status for evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum VerificationStatus {
    /// Evidence has not been verified
    #[default]
    Unverified,
    /// Evidence has been verified
    Verified,
    /// Evidence has been partially verified
    PartiallyVerified,
    /// Evidence verification failed
    Invalid,
    /// Evidence has expired
    Expired,
}

/// Enhanced provenance with evidence chains.
///
/// Extends basic provenance with rich evidence references, chain of custody,
/// and verification status for robust audit trails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EnhancedProvenance {
    /// Source turn number
    pub source_turn: i32,
    /// How this data was extracted
    pub extraction_method: ExtractionMethod,
    /// Confidence score (0.0 to 1.0)
    pub confidence: Option<f32>,
    /// Rich evidence references
    pub evidence_refs: Vec<EvidenceRef>,
    /// Chain of custody (agent trail) - tuples of (AgentId, Timestamp)
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<serde_json::Value>))]
    pub chain_of_custody: Vec<(AgentId, Timestamp)>,
    /// Verification status
    pub verification_status: VerificationStatus,
}

impl Default for EnhancedProvenance {
    fn default() -> Self {
        Self {
            source_turn: 0,
            extraction_method: ExtractionMethod::Unknown,
            confidence: None,
            evidence_refs: Vec::new(),
            chain_of_custody: Vec::new(),
            verification_status: VerificationStatus::Unverified,
        }
    }
}

impl EnhancedProvenance {
    /// Create a new enhanced provenance.
    pub fn new(source_turn: i32, extraction_method: ExtractionMethod) -> Self {
        Self {
            source_turn,
            extraction_method,
            ..Default::default()
        }
    }

    /// Add an evidence reference.
    pub fn with_evidence(mut self, evidence: EvidenceRef) -> Self {
        self.evidence_refs.push(evidence);
        self
    }

    /// Add a custody entry.
    pub fn with_custody(mut self, agent_id: AgentId, timestamp: Timestamp) -> Self {
        self.chain_of_custody.push((agent_id, timestamp));
        self
    }

    /// Set confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }

    /// Set verification status.
    pub fn with_verification(mut self, status: VerificationStatus) -> Self {
        self.verification_status = status;
        self
    }
}

// ============================================================================
// EVENT VERIFIER TRAIT (Phase 1.4)
// ============================================================================

/// Trait for cryptographic event verification.
///
/// Implementations of this trait provide hash computation and verification
/// for events, enabling tamper-evident audit logs.
pub trait EventVerifier: Send + Sync {
    /// Compute hash for an event.
    fn compute_hash<P: Serialize>(&self, event: &Event<P>) -> [u8; 32];

    /// Verify event integrity using its hash.
    fn verify_hash<P: Serialize>(&self, event: &Event<P>, expected: &[u8; 32]) -> bool {
        &self.compute_hash(event) == expected
    }

    /// Verify hash chain (current depends on previous).
    fn verify_chain<P: Serialize>(&self, current: &Event<P>, previous_hash: &[u8; 32]) -> bool;

    /// Get the algorithm used by this verifier.
    fn algorithm(&self) -> HashAlgorithm;
}

/// Blake3-based event verifier (default, recommended for performance).
#[derive(Debug, Clone, Copy, Default)]
pub struct Blake3Verifier;

impl EventVerifier for Blake3Verifier {
    fn compute_hash<P: Serialize>(&self, event: &Event<P>) -> [u8; 32] {
        let canonical = serde_json::to_vec(event).unwrap_or_default();
        blake3::hash(&canonical).into()
    }

    fn verify_chain<P: Serialize>(&self, current: &Event<P>, previous_hash: &[u8; 32]) -> bool {
        let Some(hash_chain) = &current.hash_chain else {
            return true; // Genesis events are always valid
        };

        if hash_chain.prev_hash != *previous_hash {
            return false;
        }

        let canonical = serde_json::to_vec(&current).unwrap_or_default();
        let computed_hash = blake3::hash(&canonical);
        computed_hash.as_bytes() == &hash_chain.event_hash
    }

    fn algorithm(&self) -> HashAlgorithm {
        HashAlgorithm::Blake3
    }
}

/// SHA-256-based event verifier (for compatibility).
#[derive(Debug, Clone, Copy, Default)]
pub struct Sha256Verifier;

impl EventVerifier for Sha256Verifier {
    fn compute_hash<P: Serialize>(&self, event: &Event<P>) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let canonical = serde_json::to_vec(event).unwrap_or_default();
        let result = Sha256::digest(&canonical);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    fn verify_chain<P: Serialize>(&self, current: &Event<P>, previous_hash: &[u8; 32]) -> bool {
        let Some(hash_chain) = &current.hash_chain else {
            return true; // Genesis events are always valid
        };

        if hash_chain.prev_hash != *previous_hash {
            return false;
        }

        use sha2::{Digest, Sha256};
        let canonical = serde_json::to_vec(&current).unwrap_or_default();
        let computed = Sha256::digest(&canonical);
        computed[..] == hash_chain.event_hash
    }

    fn algorithm(&self) -> HashAlgorithm {
        HashAlgorithm::Sha256
    }
}

// ============================================================================
// EVENT DAG TRAIT (Async for PG async I/O + LMDB hot path)
// ============================================================================

use crate::Effect;

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
/// # Async Design
///
/// All methods are async to support:
/// - LMDB hot cache (sync but fast - microseconds, safe in async context)
/// - PostgreSQL fallback (truly async - milliseconds, non-blocking)
///
/// # Payload Type
///
/// The `Payload` associated type determines what data is stored with each event.
/// This is typically a serializable enum of all possible event payloads.
#[async_trait::async_trait]
pub trait EventDag: Send + Sync {
    /// The payload type stored with events.
    type Payload: Clone + Send + Sync + 'static;

    /// Append a new event to the DAG.
    ///
    /// The event's position should be set by the caller based on the parent event.
    /// Returns the assigned event ID on success.
    async fn append(&self, event: Event<Self::Payload>) -> Effect<EventId>;

    /// Read an event by its ID.
    async fn read(&self, event_id: EventId) -> Effect<Event<Self::Payload>>;

    /// Walk the ancestor chain from a given event.
    ///
    /// Returns events from `from` toward the root, limited by `limit`.
    /// The events are returned in order from most recent to oldest.
    async fn walk_ancestors(
        &self,
        from: EventId,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>>;

    /// Walk descendants from a given event.
    async fn walk_descendants(
        &self,
        from: EventId,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>>;

    /// Send an upstream signal from a downstream event.
    ///
    /// Signals propagate backward through the DAG to notify upstream
    /// producers of acknowledgments, backpressure, or errors.
    async fn signal_upstream(&self, from: EventId, signal: UpstreamSignal) -> Effect<()>;

    /// Find all events in a correlation chain.
    async fn find_correlation_chain(
        &self,
        correlation_id: EventId,
    ) -> Effect<Vec<Event<Self::Payload>>>;

    /// Get the current position for appending a new event.
    async fn next_position(&self, parent: Option<EventId>, lane: u32) -> Effect<DagPosition>;

    /// Get events by kind within a position range.
    async fn find_by_kind(
        &self,
        kind: EventKind,
        min_depth: u32,
        max_depth: u32,
        limit: usize,
    ) -> Effect<Vec<Event<Self::Payload>>>;

    /// Acknowledge an event.
    async fn acknowledge(&self, event_id: EventId, send_upstream: bool) -> Effect<()>;

    /// Get unacknowledged events that require acknowledgment.
    async fn unacknowledged(&self, limit: usize) -> Effect<Vec<Event<Self::Payload>>>;
}

/// Extension trait for EventDag with convenience methods.
#[async_trait::async_trait]
pub trait EventDagExt: EventDag {
    /// Append a new root event.
    async fn append_root(&self, payload: Self::Payload) -> Effect<EventId> {
        let position = match self.next_position(None, 0).await {
            Effect::Ok(pos) => pos,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        let event_id = uuid::Uuid::now_v7();
        let event = Event {
            header: EventHeader::new(
                event_id,
                event_id,
                chrono::Utc::now().timestamp_micros(),
                position,
                0,
                EventKind::DATA,
                EventFlags::empty(),
            ),
            payload,
            hash_chain: None,
        };
        self.append(event).await
    }

    /// Append a child event to an existing event.
    async fn append_child(&self, parent: EventId, payload: Self::Payload) -> Effect<EventId> {
        let parent_event = match self.read(parent).await {
            Effect::Ok(e) => e,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        let position = match self
            .next_position(Some(parent), parent_event.header.position.lane)
            .await
        {
            Effect::Ok(pos) => pos,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        let event_id = uuid::Uuid::now_v7();
        let event = Event {
            header: EventHeader::new(
                event_id,
                parent_event.header.correlation_id,
                chrono::Utc::now().timestamp_micros(),
                position,
                0,
                EventKind::DATA,
                EventFlags::empty(),
            ),
            payload,
            hash_chain: None,
        };
        self.append(event).await
    }

    /// Fork a new lane from an existing event.
    async fn fork(
        &self,
        parent: EventId,
        new_lane: u32,
        payload: Self::Payload,
    ) -> Effect<EventId> {
        let parent_event = match self.read(parent).await {
            Effect::Ok(e) => e,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        let position = match self.next_position(Some(parent), new_lane).await {
            Effect::Ok(pos) => pos,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        let event_id = uuid::Uuid::now_v7();
        let event = Event {
            header: EventHeader::new(
                event_id,
                parent_event.header.correlation_id,
                chrono::Utc::now().timestamp_micros(),
                position,
                0,
                EventKind::DATA,
                EventFlags::empty(),
            ),
            payload,
            hash_chain: None,
        };
        self.append(event).await
    }

    /// Get the depth of an event in the DAG.
    async fn depth(&self, event_id: EventId) -> Effect<u32> {
        match self.read(event_id).await {
            Effect::Ok(event) => Effect::Ok(event.header.position.depth),
            Effect::Err(e) => Effect::Err(e),
            other => other.map(|_| unreachable!()),
        }
    }

    /// Check if one event is an ancestor of another.
    async fn is_ancestor(&self, ancestor: EventId, descendant: EventId) -> Effect<bool> {
        let ancestors = match self.walk_ancestors(descendant, 1000).await {
            Effect::Ok(events) => events,
            Effect::Err(e) => return Effect::Err(e),
            other => return other.map(|_| unreachable!()),
        };

        for event in ancestors {
            if event.header.event_id == ancestor {
                return Effect::Ok(true);
            }
        }
        Effect::Ok(false)
    }
}

// Blanket implementation: any type implementing EventDag automatically gets EventDagExt
impl<T: EventDag> EventDagExt for T {}

/// Builder for creating events with proper positioning.
#[derive(Debug, Clone)]
pub struct EventBuilder<P> {
    parent: Option<EventId>,
    lane: u32,
    correlation_id: Option<EventId>,
    payload: P,
    flags: EventFlags,
}

impl<P> EventBuilder<P> {
    /// Create a new event builder with the given payload.
    pub fn new(payload: P) -> Self {
        Self {
            parent: None,
            lane: 0,
            correlation_id: None,
            payload,
            flags: EventFlags::empty(),
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
        self.flags |= EventFlags::REQUIRES_ACK;
        self
    }

    /// Mark this event as critical.
    pub fn critical(mut self) -> Self {
        self.flags |= EventFlags::CRITICAL;
        self
    }

    /// Mark this event as transactional.
    pub fn transactional(mut self) -> Self {
        self.flags |= EventFlags::TRANSACTIONAL;
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
    pub fn get_flags(&self) -> EventFlags {
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
            correlation_id,
            1234567890,
            DagPosition::root(),
            100,
            EventKind::TRAJECTORY_CREATED,
            EventFlags::REQUIRES_ACK,
        );

        assert_eq!(header.event_id, event_id);
        assert!(header.requires_ack());
        assert!(!header.is_acknowledged());
    }

    #[test]
    fn test_hash_chain_genesis_event() {
        use serde_json::json;

        let event = Event::new(
            EventHeader::new(
                Uuid::now_v7(),
                Uuid::now_v7(),
                chrono::Utc::now().timestamp_micros(),
                DagPosition::root(),
                0,
                EventKind::DATA,
                EventFlags::empty(),
            ),
            json!({"type": "genesis"}),
        );

        assert!(event.is_genesis());
        assert!(event.verify(&[0u8; 32]));
    }

    #[test]
    fn test_hash_chain_tamper_detection() {
        use serde_json::json;

        let parent = Event::new(
            EventHeader::new(
                Uuid::now_v7(),
                Uuid::now_v7(),
                chrono::Utc::now().timestamp_micros(),
                DagPosition::root(),
                0,
                EventKind::DATA,
                EventFlags::empty(),
            ),
            json!({"data": "original"}),
        );

        let parent_hash = parent.compute_hash();
        let wrong_hash = [0xFF; 32];

        let child_hash = {
            let temp = Event::new(
                EventHeader::new(
                    Uuid::now_v7(),
                    Uuid::now_v7(),
                    chrono::Utc::now().timestamp_micros(),
                    DagPosition::new(1, 0, 0),
                    0,
                    EventKind::DATA,
                    EventFlags::empty(),
                ),
                json!({"data": "child"}),
            );
            temp.compute_hash()
        };

        let hash_chain = HashChain {
            prev_hash: wrong_hash, // Wrong!
            event_hash: child_hash,
            algorithm: HashAlgorithm::Blake3,
        };

        let tampered = Event::with_hash_chain(
            EventHeader::new(
                Uuid::now_v7(),
                Uuid::now_v7(),
                chrono::Utc::now().timestamp_micros(),
                DagPosition::new(1, 0, 0),
                0,
                EventKind::DATA,
                EventFlags::empty(),
            ),
            json!({"data": "child"}),
            hash_chain,
        );

        assert!(!tampered.verify(&parent_hash)); // Should fail
    }
}
