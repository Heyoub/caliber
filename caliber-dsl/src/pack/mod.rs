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
    let ir = PackIr::new(manifest, md_docs)?;
    let ast = build_ast(&ir)?;
    let compiled = DslCompiler::compile(&ast)?;
    Ok(PackOutput { ast, compiled })
}
