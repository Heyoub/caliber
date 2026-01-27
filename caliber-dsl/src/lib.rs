//! CALIBER DSL - Domain Specific Language Parser & Compiler
//!
//! This crate provides a lexer, parser, compiler, and pretty-printer for the CALIBER DSL.
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
//! Compiler (validate + transform)
//!     ↓
//! CompiledConfig (runtime-ready)
//!     ↓
//! Pretty-Printer (for round-trip testing)
//! ```

pub mod compiler;
pub mod lexer;
pub mod pack;
pub mod parser;
pub mod pretty_printer;

// Re-export key types for convenience
pub use compiler::*;
pub use lexer::*;
pub use pack::{compose_pack, PackInput, PackMarkdownFile, PackOutput, PackError};
pub use parser::*;
