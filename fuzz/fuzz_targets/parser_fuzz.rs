//! Fuzz test for the CALIBER DSL Parser
//!
//! This fuzz target tests the parser with arbitrary byte sequences to find:
//! - Panics or crashes
//! - Infinite loops
//! - Memory safety issues
//!
//! Run with: cargo +nightly fuzz run parser_fuzz -- -max_total_time=60
//!
//! Feature: caliber-core-implementation
//! Task: 4.11 Write fuzz tests for parser
//! Validates: Requirements 5.7 (ParseError with line/column info)

#![no_main]

use caliber_dsl::{parse, Lexer, Parser};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try to interpret the bytes as UTF-8
    // The parser should handle any valid UTF-8 string without panicking
    if let Ok(input) = std::str::from_utf8(data) {
        // Test the full parse pipeline
        let result = parse(input);
        
        // The parser should never panic - it should return Ok or Err
        match result {
            Ok(ast) => {
                // If parsing succeeded, verify basic AST invariants
                // Version should not be empty
                assert!(!ast.version.is_empty() || ast.definitions.is_empty(),
                    "Parsed AST should have a version or be minimal");
            }
            Err(err) => {
                // If parsing failed, verify error has valid location info
                assert!(err.line >= 1, "Error line should be >= 1");
                assert!(err.column >= 1, "Error column should be >= 1");
                assert!(!err.message.is_empty(), "Error message should not be empty");
            }
        }
        
        // Also test lexer -> parser pipeline separately
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        
        // Parser should handle any token stream without panicking
        let mut parser = Parser::new(tokens);
        let _ = parser.parse(); // Result doesn't matter, just shouldn't panic
    }
});
