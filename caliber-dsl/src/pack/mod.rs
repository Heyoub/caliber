//! Pack compiler: TOML + Markdown -> Pack IR -> CaliberAst -> CompiledConfig
//!
//! This module is a façade over the existing DSL IR. It does NOT emit DSL text.

mod ast;
mod ir;
mod markdown;
mod schema;

use crate::{compiler::CompiledConfig, parser::CaliberAst, DslCompiler};
use std::path::PathBuf;

pub use ast::build_ast;
pub use ir::*;
pub use markdown::*;
pub use schema::*;

#[derive(Debug, Clone)]
pub struct PackInput {
    pub root: PathBuf,
    pub manifest: String,
    pub markdowns: Vec<PackMarkdownFile>,
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
    compiled.tools = compile_tools(&manifest);
    compiled.toolsets = compile_toolsets(&manifest);
    compiled.pack_agents = compile_pack_agents(&manifest);
    compiled.pack_injections = compile_pack_injections(&manifest)?;
    compiled.pack_routing = compile_pack_routing(&manifest);

    Ok(PackOutput { ast, compiled })
}
