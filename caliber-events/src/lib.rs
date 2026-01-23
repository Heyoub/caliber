//! CALIBER Events - Event DAG Trait and Operations
//!
//! This crate defines the EventDag trait for persistent event storage and traversal.
//! It provides the contract for event DAG implementations without providing
//! the actual implementation.
//!
//! # Architecture
//!
//! The Event DAG follows the "tram car tracks" pattern:
//! - Events flow forward (downstream) through the DAG
//! - Signals can flow backward (upstream) for coordination
//!
//! ```text
//! Events:   Root → Event1 → Event2 → Event3
//!                     ↑         ↑        ↑
//! Signals:  ← Ack ← Ack ← Backpressure
//! ```
//!
//! # Key Types
//!
//! Core types are re-exported from `caliber-core`:
//! - `Event<P>`: An event with header and payload
//! - `EventHeader`: 64-byte cache-aligned event metadata
//! - `EventKind`: Event type discriminator
//! - `DagPosition`: Position in the event graph
//! - `Effect<T>`: Result type with retry/compensation support
//! - `UpstreamSignal`: Signals sent back through the DAG
//!
//! # Traits
//!
//! - `EventDag`: Core trait for DAG operations (append, read, walk, signal)
//! - `EventDagExt`: Convenience extension methods

mod dag;
mod in_memory;

// Re-export the EventDag trait and related types
pub use dag::{EventBuilder, EventDag, EventDagExt};
pub use in_memory::InMemoryEventDag;

// Re-export core types for convenience
pub use caliber_core::{
    DagPosition, Effect, ErrorEffect, Event, EventFlags, EventHeader, EventId, EventKind,
    UpstreamSignal, WaitCondition,
};
