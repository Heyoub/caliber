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
pub use config::{
    ast_to_markdown, parse_adapter_block, parse_agent_block, parse_cache_block,
    parse_injection_block, parse_memory_block, parse_policy_block, parse_provider_block,
    parse_trajectory_block, ConfigError,
};
pub use pack::{compose_pack, PackError, PackInput, PackMarkdownFile, PackOutput};
pub use parser::*;

/// DEPRECATED: Custom DSL parser has been removed in favor of Markdown-based configuration.
///
/// Use `compose_pack()` with Markdown fence blocks instead.
///
/// Migration guide:
/// ```text
/// OLD (removed):
///   let ast = caliber_dsl::parse(dsl_source)?;
///
/// NEW:
///   use caliber_dsl::{PackInput, PackMarkdownFile, compose_pack};
///
///   let input = PackInput {
///       root: PathBuf::from("."),
///       manifest: manifest_toml,
///       markdowns: vec![PackMarkdownFile { path, content }],
///       contracts: HashMap::new(),
///   };
///   let output = compose_pack(input)?;
///   let ast = output.ast;
/// ```
#[deprecated(
    since = "0.4.4",
    note = "Use `compose_pack()` with Markdown configurations instead of DSL strings"
)]
pub fn parse(_source: &str) -> Result<CaliberAst, String> {
    Err("DSL parser removed. Use compose_pack() with Markdown fence blocks instead. See docs/MARKDOWN_CONFIG.md".to_string())
}
