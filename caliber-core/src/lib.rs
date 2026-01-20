//! CALIBER Core - Entity Types
//!
//! Pure data structures with no behavior. All other crates depend on this.
//! This crate contains ONLY data types - no business logic.

// Import external dependencies
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

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
