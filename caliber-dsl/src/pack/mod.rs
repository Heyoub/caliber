//! Pack compiler: TOML + Markdown -> Pack IR -> CaliberAst -> CompiledConfig
//!
//! This module is a façade over the existing DSL IR. It does NOT emit DSL text.

mod ast;
mod ir;
mod markdown;
mod mcp;
mod schema;

use crate::{compiler::CompiledConfig, parser::CaliberAst, DslCompiler};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::path::PathBuf;

pub use ast::build_ast;
pub use ir::*;
pub use markdown::*;
pub use mcp::*;
pub use schema::*;

#[derive(Debug, Clone)]
pub struct PackInput {
    pub root: PathBuf,
    pub manifest: String,
    pub markdowns: Vec<PackMarkdownFile>,
    /// Contract JSON Schema files: maps relative path to content.
    #[allow(dead_code)]
    pub contracts: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PackMarkdownFile {
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct PackOutput {
    pub ast: CaliberAst,
    pub compiled: CompiledConfig,
}

pub fn compose_pack(input: PackInput) -> Result<PackOutput, PackError> {
    let manifest = parse_manifest(&input.manifest)?;
    let md_docs = parse_markdown_files(&manifest, &input.markdowns)?;
    let ir = PackIr::new(manifest.clone(), md_docs)?;
    let ast = build_ast(&ir)?;
    let mut compiled = DslCompiler::compile(&ast)?;

    // Inject pack tool registry + toolsets into the runtime config.
    // Pass contract files for schema compilation.
    compiled.tools = compile_tools(&manifest, &input.contracts)?;
    compiled.toolsets = compile_toolsets(&manifest);
    compiled.pack_agents = compile_pack_agents(&manifest, &ir.markdown);
    compiled.pack_injections = compile_pack_injections(&manifest)?;
    compiled.pack_routing = compile_pack_routing(&manifest);

    // Compute file hashes for artifact determinism (lockfile support).
    compiled.file_hashes = compute_file_hashes(&input);

    Ok(PackOutput { ast, compiled })
}

/// Compute SHA-256 hashes for all pack source files.
/// These hashes enable runtime drift detection without recompilation.
fn compute_file_hashes(input: &PackInput) -> HashMap<String, String> {
    let mut hashes = HashMap::new();

    // Hash the manifest (cal.toml)
    let manifest_hash = sha256_hex(&input.manifest);
    hashes.insert("cal.toml".to_string(), manifest_hash);

    // Hash all markdown files
    for md in &input.markdowns {
        let rel_path = md.path
            .strip_prefix(&input.root)
            .unwrap_or(&md.path)
            .to_string_lossy()
            .to_string();
        let hash = sha256_hex(&md.content);
        hashes.insert(rel_path, hash);
    }

    // Hash all contract files
    for (path, content) in &input.contracts {
        let hash = sha256_hex(content);
        hashes.insert(path.clone(), hash);
    }

    hashes
}

/// Compute SHA-256 hash of content, returning hex-encoded string.
fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}
