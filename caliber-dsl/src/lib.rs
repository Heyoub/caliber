//! CALIBER DSL - Domain Specific Language Parser & Compiler
//!
//! This crate provides a Markdown-based configuration parser, compiler, and pretty-printer.
//! Configurations are defined in Markdown fence blocks with YAML payloads.
//!
//! Architecture:
//! ```text
//! Markdown Source (.md files)
//!     ↓
//! Pack Markdown Parser (fence blocks)
//!     ↓
//! Config Parser (YAML → AST)
//!     ↓
//! Compiler (validate + transform)
//!     ↓
//! CompiledConfig (runtime-ready)
//!     ↓
//! Markdown Printer (for round-trip testing)
//! ```

pub mod compiler;
pub mod config;
pub mod pack;
pub mod parser;
pub mod pretty_printer;

// Re-export key types for convenience
pub use compiler::*;
pub use config::*;
pub use pack::{compose_pack, PackError, PackInput, PackMarkdownFile, PackOutput};
pub use parser::*;
