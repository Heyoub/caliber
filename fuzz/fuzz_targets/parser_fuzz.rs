//! Fuzz test for the CALIBER Markdown Parser
//!
//! This fuzz target tests the pack markdown parser with arbitrary byte sequences to find:
//! - Panics or crashes
//! - Infinite loops
//! - Memory safety issues
//!
//! Run with: cargo +nightly fuzz run parser_fuzz -- -max_total_time=60

#![no_main]

use caliber_dsl::pack::{compose_pack, PackInput, PackMarkdownFile};
use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

fuzz_target!(|data: &[u8]| {
    // Try to interpret the bytes as UTF-8
    // The parser should handle any valid UTF-8 string without panicking
    if let Ok(input) = std::str::from_utf8(data) {
        // Create minimal manifest
        let manifest_toml = r#"
[meta]
name = "fuzz"
version = "1.0"
[tools]
bin = {}
prompts = {}
[profiles]
[agents]
[toolsets]
[adapters]
[providers]
[policies]
[injections]
"#;

        let pack_input = PackInput {
            root: PathBuf::from("."),
            manifest: manifest_toml.to_string(),
            markdowns: vec![PackMarkdownFile {
                path: PathBuf::from("fuzz.md"),
                content: input.to_string(),
            }],
            contracts: std::collections::HashMap::new(),
        };

        // Test the full parse pipeline
        let result = compose_pack(pack_input);

        // The parser should never panic - it should return Ok or Err
        match result {
            Ok(output) => {
                // If parsing succeeded, verify basic AST invariants
                // Version should not be empty
                assert!(
                    !output.ast.version.is_empty() || output.ast.definitions.is_empty(),
                    "Parsed AST should have a version or be minimal"
                );
            }
            Err(_err) => {
                // If parsing failed, that's fine - just shouldn't panic
            }
        }
    }
});
