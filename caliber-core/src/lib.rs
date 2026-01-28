//! CALIBER Core - Entity Types
//!
//! Pure data structures with no behavior. All other crates depend on this.
//! This crate contains ONLY data types - no business logic.
//!
//! # Type Dictionary
//!
//! This crate serves as the "type dictionary" for CALIBER. All types visible
//! here form the vocabulary of the system:
//!
//! - **Identity**: Type-safe ID newtypes (`TenantId`, `TrajectoryId`, etc.), `Timestamp`, `ContentHash`
//! - **Enums**: Status types, entity types, categories
//! - **Entities**: Core domain entities (Trajectory, Scope, Artifact, Note, Turn)
//! - **Typestate**: Compile-time safe Lock, Handoff, Delegation lifecycles
//! - **Events**: Event DAG types (EventHeader, DagPosition, EventKind)
//! - **Effects**: Error-as-effects pattern (Effect<T>, ErrorEffect)

// Core modules
mod agent;
mod battle_intel;
mod config;
mod embedding;
mod entities;
mod enums;
mod error;
mod filter;
mod health;
mod identity;
mod llm;

// Typestate modules (compile-time safety for critical paths)
mod delegation;
mod handoff;
mod lock;

// Event DAG modules
mod effect;
mod event;

// Context assembly module
mod context;

// Re-export identity types
pub use identity::*;

// Re-export all enums
pub use enums::*;

// Re-export embedding types
pub use embedding::*;

// Re-export entity structs
pub use entities::*;

// Re-export Battle Intel types
pub use battle_intel::*;

// Re-export error types
pub use error::*;

// Re-export config types
pub use config::*;

// Re-export filter types
pub use filter::*;

// Re-export health types
pub use health::*;

// Re-export typestate types (Lock, Handoff, Delegation)
pub use delegation::*;
pub use handoff::*;
pub use lock::*;

// Re-export event DAG types
pub use event::*;

// Re-export effect types
pub use effect::*;

// Re-export context assembly types
pub use context::*;

// Re-export agent types (consolidated from caliber-agents)
pub use agent::*;

// Re-export LLM primitive types (consolidated from caliber-llm)
pub use llm::*;
