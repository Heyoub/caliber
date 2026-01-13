//! Fuzz test for the CALIBER DSL Lexer
//!
//! This fuzz target tests the lexer with arbitrary byte sequences to find:
//! - Panics or crashes
//! - Infinite loops
//! - Memory safety issues
//!
//! Run with: cargo +nightly fuzz run lexer_fuzz -- -max_total_time=60
//!
//! Feature: caliber-core-implementation
//! Task: 3.6 Write fuzz tests for lexer
//! Validates: Requirements 4.8 (error handling for invalid characters)

#![no_main]

use caliber_dsl::Lexer;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try to interpret the bytes as UTF-8
    // The lexer should handle any valid UTF-8 string without panicking
    if let Ok(input) = std::str::from_utf8(data) {
        let mut lexer = Lexer::new(input);
        
        // Tokenize should never panic, even with malformed input
        let tokens = lexer.tokenize();
        
        // Basic invariants that should always hold:
        // 1. We should always get at least one token (Eof)
        assert!(!tokens.is_empty(), "Tokenization should produce at least Eof");
        
        // 2. The last token should always be Eof
        assert_eq!(
            tokens.last().unwrap().kind,
            caliber_dsl::TokenKind::Eof,
            "Last token should always be Eof"
        );
        
        // 3. Span positions should be valid
        for token in &tokens {
            assert!(token.span.start <= token.span.end, "Span start should be <= end");
            assert!(token.span.line >= 1, "Line numbers should be >= 1");
            assert!(token.span.column >= 1, "Column numbers should be >= 1");
        }
    }
});
