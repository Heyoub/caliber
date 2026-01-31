//! Shared Markdown parsing utilities.
//!
//! This module provides a shared `parse_markdown_source` function used by
//! both the DSL routes and database validation to parse standalone Markdown
//! source into a CaliberAst.

use caliber_dsl::pack::{compose_pack, PackInput, PackMarkdownFile};
use caliber_dsl::CaliberAst;
use std::collections::HashMap;
use std::path::PathBuf;

/// Parse standalone Markdown source to AST using compose_pack.
///
/// This wraps the source in a minimal manifest for standalone parsing.
/// Used by DSL routes and database validation.
///
/// # Arguments
///
/// * `source` - The Markdown source string to parse
///
/// # Returns
///
/// * `Ok(CaliberAst)` - The parsed AST on success
/// * `Err(String)` - Error message on parse failure
///
/// # Example
///
/// ```ignore
/// use caliber_api::utils::parse_markdown_source;
///
/// let source = r#"
/// ```adapter main_db
/// adapter_type: postgres
/// connection: "postgresql://localhost/caliber"
/// ```
/// "#;
///
/// let ast = parse_markdown_source(source)?;
/// ```
pub fn parse_markdown_source(source: &str) -> Result<CaliberAst, String> {
    // Minimal manifest for standalone DSL parsing
    let manifest = r#"
[meta]
version = "1.0"
project = "standalone"

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

    let input = PackInput {
        root: PathBuf::from("."),
        manifest: manifest.to_string(),
        markdowns: vec![PackMarkdownFile {
            path: PathBuf::from("input.md"),
            content: source.to_string(),
        }],
        contracts: HashMap::new(),
    };

    compose_pack(input)
        .map(|output| output.ast)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_source_requires_sections() {
        // Empty source should fail because compose_pack requires proper markdown structure
        let result = parse_markdown_source("");
        assert!(
            result.is_err(),
            "Empty source should fail - requires agent prompt structure"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("missing required sections"),
            "Error should mention missing required sections"
        );
    }

    #[test]
    fn test_parse_adapter_block_requires_sections() {
        // Standalone adapter blocks should fail because compose_pack requires full structure
        let source = r#"
```adapter test_db
adapter_type: postgres
connection: "postgresql://localhost/test"
```
"#;
        let result = parse_markdown_source(source);
        assert!(
            result.is_err(),
            "Standalone adapter block should fail - requires agent prompt structure"
        );
    }

    #[test]
    fn test_parse_full_agent_prompt() {
        // Full agent prompt with required sections should succeed
        let source = r#"# System

You are a helpful assistant.

## PCP

- Follow user instructions carefully.

### User

The user wants help with tasks.
"#;
        let result = parse_markdown_source(source);
        assert!(
            result.is_ok(),
            "Full agent prompt with required sections should parse: {:?}",
            result.err()
        );
    }
}
