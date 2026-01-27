//! Pack IR -> CaliberAst

use super::ir::{ast_from_ir, PackError, PackIr};
use crate::parser::CaliberAst;

pub fn build_ast(ir: &PackIr) -> Result<CaliberAst, PackError> {
    Ok(ast_from_ir(ir))
}
