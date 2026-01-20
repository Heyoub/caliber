//! CALIBER Core - Entity Types
//!
//! Pure data structures with no behavior. All other crates depend on this.
//! This crate contains ONLY data types - no business logic.

// Core modules
mod identity;
mod enums;
mod embedding;
mod entities;
mod battle_intel;
mod error;
mod config;
mod filter;
mod health;

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
