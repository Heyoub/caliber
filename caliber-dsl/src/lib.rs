//! CALIBER DSL - Domain Specific Language Parser
//!
//! This crate provides a lexer, parser, and pretty-printer for the CALIBER DSL.
//! The DSL is used to define memory types, policies, adapters, and injection rules.
//!
//! Architecture:
//! ```text
//! DSL Source (.caliber file)
//!     ↓
//! Lexer (tokenize)
//!     ↓
//! Parser (build AST)
//!     ↓
//! Pretty-Printer (for round-trip testing)
//! ```

pub mod lexer;
pub mod parser;
pub mod pretty_printer;

// Re-export key types for convenience
pub use lexer::*;
pub use parser::*;
