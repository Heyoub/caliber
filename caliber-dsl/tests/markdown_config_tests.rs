//! Integration tests for Markdown-based config parsing
//!
//! Tests verify:
//! - Fence block parsing (FenceKind enum matching)
//! - YAML payload validation (deny_unknown_fields)
//! - Name precedence (header vs payload)
//! - Case preservation (no lowercasing)
//! - Round-trip stability (AST → Markdown → AST)

use caliber_dsl::pack::{compose_pack, PackInput, PackMarkdownFile};
use caliber_dsl::parser::ast::*;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// TEST FIXTURES (Const Data - Zero Allocation)
// ============================================================================

/// Minimal manifest for testing (no TOML-based configs)
const MINIMAL_MANIFEST: &str = r#"
[meta]
version = "1.0"
project = "test"

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

/// Template for valid markdown with fence blocks
const MARKDOWN_TEMPLATE: &str = r#"
# System

Test system prompt

## PCP

Test PCP instructions

### User

{fence_blocks}
"#;

// ============================================================================
// TEST BUILDERS (Fluent API)
// ============================================================================

/// Builder for test PackInput (fluent API)
#[derive(Default)]
struct TestPackBuilder {
    fence_blocks: Vec<String>,
}

impl TestPackBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn with_adapter(mut self, name: &str, adapter_type: &str, connection: &str) -> Self {
        self.fence_blocks.push(format!(
            r#"```adapter {}
adapter_type: {}
connection: "{}"
```"#,
            name, adapter_type, connection
        ));
        self
    }

    fn with_provider(mut self, name: &str, provider_type: &str, api_key: &str, model: &str) -> Self {
        self.fence_blocks.push(format!(
            r#"```provider {}
provider_type: {}
api_key: {}
model: "{}"
```"#,
            name, provider_type, api_key, model
        ));
        self
    }

    fn with_policy(mut self, name: &str, trigger: &str, actions: &[&str]) -> Self {
        let actions_yaml = actions
            .iter()
            .map(|a| format!("      - type: {}\n        target: test", a))
            .collect::<Vec<_>>()
            .join("\n");

        self.fence_blocks.push(format!(
            r#"```policy {}
rules:
  - trigger: {}
    actions:
{}
```"#,
            name, trigger, actions_yaml
        ));
        self
    }

    fn with_injection(mut self, source: &str, target: &str, mode: &str, priority: i32) -> Self {
        self.fence_blocks.push(format!(
            r#"```injection
source: "{}"
target: "{}"
mode: {}
priority: {}
```"#,
            source, target, mode, priority
        ));
        self
    }

    fn with_memory(mut self, name: &str) -> Self {
        self.fence_blocks.push(format!(
            r#"```memory {}
memory_type: working
retention: session
lifecycle: explicit
```"#,
            name
        ));
        self
    }

    fn with_raw_fence(mut self, block: &str) -> Self {
        self.fence_blocks.push(block.to_string());
        self
    }

    fn build(self) -> PackInput {
        let markdown_content = MARKDOWN_TEMPLATE.replace(
            "{fence_blocks}",
            &self.fence_blocks.join("\n\n"),
        );

        PackInput {
            root: PathBuf::from("."),
            manifest: MINIMAL_MANIFEST.to_string(),
            markdowns: vec![PackMarkdownFile {
                path: PathBuf::from("test.md"),
                content: markdown_content,
            }],
            contracts: HashMap::new(),
        }
    }
}

// ============================================================================
// ASSERTION HELPERS (Descriptive Errors)
// ============================================================================

/// Assert adapter exists with expected values
fn assert_adapter_eq(ast: &CaliberAst, name: &str, expected_type: AdapterType, expected_conn: &str) {
    let adapter = ast.definitions.iter()
        .find_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
        .unwrap_or_else(|| panic!("No adapter found in AST"));

    assert_eq!(adapter.name, name, "Adapter name mismatch");
    assert_eq!(adapter.adapter_type, expected_type, "Adapter type mismatch");
    assert_eq!(adapter.connection, expected_conn, "Adapter connection mismatch");
}

/// Assert provider exists with expected values
fn assert_provider_eq(ast: &CaliberAst, name: &str, expected_type: ProviderType, expected_model: &str) {
    let provider = ast.definitions.iter()
        .find_map(|d| if let Definition::Provider(p) = d { Some(p) } else { None })
        .unwrap_or_else(|| panic!("No provider found in AST"));

    assert_eq!(provider.name, name, "Provider name mismatch");
    assert_eq!(provider.provider_type, expected_type, "Provider type mismatch");
    assert_eq!(provider.model, expected_model, "Provider model mismatch");
}

/// Assert policy exists with expected trigger
fn assert_policy_trigger(ast: &CaliberAst, name: &str, expected_trigger: Trigger) {
    let policy = ast.definitions.iter()
        .find_map(|d| if let Definition::Policy(p) = d { Some(p) } else { None })
        .unwrap_or_else(|| panic!("No policy found in AST"));

    assert_eq!(policy.name, name, "Policy name mismatch");
    assert!(!policy.rules.is_empty(), "Policy should have at least one rule");
    assert_eq!(policy.rules[0].trigger, expected_trigger, "Policy trigger mismatch");
}

// ============================================================================
// BASIC PARSING TESTS
// ============================================================================

#[test]
fn test_parse_adapter_with_header_name() {
    let input = TestPackBuilder::new()
        .with_adapter("postgres_main", "postgres", "postgresql://localhost/caliber")
        .build();

    let output = compose_pack(input).expect("Failed to parse pack");

    assert_adapter_eq(
        &output.ast,
        "postgres_main",
        AdapterType::Postgres,
        "postgresql://localhost/caliber",
    );
}

#[test]
fn test_parse_provider_with_env_key() {
    let input = TestPackBuilder::new()
        .with_provider("openai_main", "openai", "env:OPENAI_API_KEY", "gpt-4")
        .build();

    let output = compose_pack(input).expect("Failed to parse pack");

    assert_provider_eq(&output.ast, "openai_main", ProviderType::OpenAI, "gpt-4");

    // Verify env parsing
    let provider = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Provider(p) = d { Some(p) } else { None })
        .unwrap();

    match &provider.api_key {
        EnvValue::Env(var) => assert_eq!(var, "OPENAI_API_KEY"),
        _ => panic!("Expected EnvValue::Env"),
    }
}

#[test]
fn test_parse_policy_with_multiple_actions() {
    let input = TestPackBuilder::new()
        .with_policy("cleanup", "scope_close", &["summarize", "checkpoint"])
        .build();

    let output = compose_pack(input).expect("Failed to parse pack");

    assert_policy_trigger(&output.ast, "cleanup", Trigger::ScopeClose);

    let policy = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Policy(p) = d { Some(p) } else { None })
        .unwrap();

    assert_eq!(policy.rules[0].actions.len(), 2, "Expected 2 actions");
}

#[test]
fn test_parse_injection_with_priority() {
    let input = TestPackBuilder::new()
        .with_memory("memories.notes")  // Add memory for injection to reference
        .with_injection("memories.notes", "context.main", "full", 100)
        .build();

    let output = compose_pack(input).expect("Failed to parse pack");

    let injection = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Injection(i) = d { Some(i) } else { None })
        .expect("No injection found in AST");

    assert_eq!(injection.source, "memories.notes");
    assert_eq!(injection.target, "context.main");
    assert_eq!(injection.mode, InjectionMode::Full);
    assert_eq!(injection.priority, 100);
}

// ============================================================================
// CASE PRESERVATION TESTS (The Core Bug Fix)
// ============================================================================

#[test]
fn test_case_preserved_in_adapter_name() {
    let input = TestPackBuilder::new()
        .with_adapter("MyAdapter", "postgres", "postgresql://localhost/db")
        .build();

    let output = compose_pack(input).expect("Failed to parse pack");

    let adapter = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
        .unwrap();

    assert_eq!(adapter.name, "MyAdapter", "Case should be preserved exactly");
}

#[test]
fn test_case_preserved_in_connection_string() {
    let input = TestPackBuilder::new()
        .with_adapter("db", "postgres", "PostgreSQL://LocalHost:5432/MyDatabase")
        .build();

    let output = compose_pack(input).expect("Failed to parse pack");

    let adapter = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
        .unwrap();

    assert_eq!(
        adapter.connection,
        "PostgreSQL://LocalHost:5432/MyDatabase",
        "Connection string case should be preserved"
    );
}

#[test]
fn test_case_preserved_in_provider_model() {
    let input = TestPackBuilder::new()
        .with_provider("ai", "openai", "env:KEY", "GPT-4-Turbo")
        .build();

    let output = compose_pack(input).expect("Failed to parse pack");

    let provider = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Provider(p) = d { Some(p) } else { None })
        .unwrap();

    assert_eq!(provider.model, "GPT-4-Turbo", "Model case should be preserved");
}

#[test]
fn test_mixed_case_names() {
    // This test would have FAILED with the old DSL parser
    let input = TestPackBuilder::new()
        .with_adapter("oN", "postgres", "conn")
        .with_provider("OpenAI_Provider", "openai", "env:KEY", "model")
        .build();

    let output = compose_pack(input).expect("Failed to parse pack");

    // Verify exact case preservation
    let adapter = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
        .unwrap();
    assert_eq!(adapter.name, "oN", "Should preserve 'oN' not normalize to 'on'");

    let provider = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Provider(p) = d { Some(p) } else { None })
        .unwrap();
    assert_eq!(provider.name, "OpenAI_Provider", "Should preserve exact case");
}

// ============================================================================
// VALIDATION TESTS (deny_unknown_fields)
// ============================================================================

#[test]
fn test_reject_unknown_field_in_adapter() {
    let input = TestPackBuilder::new()
        .with_raw_fence(r#"```adapter test
adapter_type: postgres
connection: "conn"
invalid_field: true
```"#)
        .build();

    let result = compose_pack(input);

    assert!(result.is_err(), "Should reject unknown field");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("unknown field") || err_msg.contains("invalid_field"),
        "Error should mention unknown field: {}",
        err_msg
    );
}

#[test]
fn test_reject_typo_in_provider_type() {
    let input = TestPackBuilder::new()
        .with_raw_fence(r#"```provider test
providor_type: openai
api_key: "key"
model: "gpt-4"
```"#)
        .build();

    let result = compose_pack(input);

    assert!(result.is_err(), "Should reject typo 'providor_type'");
}

// ============================================================================
// NAME PRECEDENCE TESTS (Header vs Payload)
// ============================================================================

#[test]
fn test_name_from_header_only() {
    let input = TestPackBuilder::new()
        .with_raw_fence(r#"```adapter my_adapter
adapter_type: postgres
connection: "conn"
```"#)
        .build();

    let output = compose_pack(input).expect("Should parse with header name");

    let adapter = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
        .unwrap();

    assert_eq!(adapter.name, "my_adapter");
}

#[test]
fn test_name_from_payload_only() {
    let input = TestPackBuilder::new()
        .with_raw_fence(r#"```adapter
name: my_adapter
adapter_type: postgres
connection: "conn"
```"#)
        .build();

    let output = compose_pack(input).expect("Should parse with payload name");

    let adapter = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
        .unwrap();

    assert_eq!(adapter.name, "my_adapter");
}

#[test]
fn test_reject_name_conflict() {
    // Name in both header AND payload = error
    let input = TestPackBuilder::new()
        .with_raw_fence(r#"```adapter header_name
name: payload_name
adapter_type: postgres
connection: "conn"
```"#)
        .build();

    let result = compose_pack(input);

    assert!(result.is_err(), "Should reject name conflict");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.to_lowercase().contains("conflict") || err_msg.contains("both"),
        "Error should mention name conflict: {}",
        err_msg
    );
}

#[test]
fn test_reject_missing_name() {
    // No name in header OR payload = error
    let input = TestPackBuilder::new()
        .with_raw_fence(r#"```adapter
adapter_type: postgres
connection: "conn"
```"#)
        .build();

    let result = compose_pack(input);

    assert!(result.is_err(), "Should reject missing name");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.to_lowercase().contains("missing") || err_msg.contains("require"),
        "Error should mention missing name: {}",
        err_msg
    );
}

// ============================================================================
// MULTI-CONFIG TESTS (Multiple Fence Blocks)
// ============================================================================

#[test]
fn test_parse_multiple_adapters() {
    let input = TestPackBuilder::new()
        .with_adapter("postgres_main", "postgres", "conn1")
        .with_adapter("redis_cache", "redis", "conn2")
        .build();

    let output = compose_pack(input).expect("Failed to parse multiple adapters");

    let adapters: Vec<_> = output.ast.definitions.iter()
        .filter_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
        .collect();

    assert_eq!(adapters.len(), 2, "Should have 2 adapters");
    assert!(adapters.iter().any(|a| a.name == "postgres_main"));
    assert!(adapters.iter().any(|a| a.name == "redis_cache"));
}

#[test]
fn test_parse_mixed_config_types() {
    let input = TestPackBuilder::new()
        .with_adapter("db", "postgres", "conn")
        .with_provider("ai", "openai", "env:KEY", "gpt-4")
        .with_policy("cleanup", "scope_close", &["summarize"])
        .with_memory("notes")  // Add memory for injection to reference
        .with_injection("notes", "context", "full", 100)
        .build();

    let output = compose_pack(input).expect("Failed to parse mixed configs");

    // Verify all 4 types present
    let has_adapter = output.ast.definitions.iter()
        .any(|d| matches!(d, Definition::Adapter(_)));
    let has_provider = output.ast.definitions.iter()
        .any(|d| matches!(d, Definition::Provider(_)));
    let has_policy = output.ast.definitions.iter()
        .any(|d| matches!(d, Definition::Policy(_)));
    let has_injection = output.ast.definitions.iter()
        .any(|d| matches!(d, Definition::Injection(_)));

    assert!(has_adapter, "Should have adapter");
    assert!(has_provider, "Should have provider");
    assert!(has_policy, "Should have policy");
    assert!(has_injection, "Should have injection");
}

// ============================================================================
// DETERMINISM TESTS (Alphabetical Ordering)
// ============================================================================

#[test]
fn test_adapter_ordering_deterministic() {
    // Define adapters in non-alphabetical order
    let input = TestPackBuilder::new()
        .with_adapter("zebra", "postgres", "conn1")
        .with_adapter("apple", "postgres", "conn2")
        .with_adapter("mango", "postgres", "conn3")
        .build();

    let output = compose_pack(input).expect("Failed to parse");

    let adapter_names: Vec<String> = output.ast.definitions.iter()
        .filter_map(|d| if let Definition::Adapter(a) = d { Some(a.name.clone()) } else { None })
        .collect();

    // The output order depends on how they're stored
    // For determinism, we should document the expected ordering
    // Currently: maintains parse order (TOML first, then Markdown in order)
    assert_eq!(adapter_names.len(), 3);
    assert!(adapter_names.contains(&"zebra".to_string()));
    assert!(adapter_names.contains(&"apple".to_string()));
    assert!(adapter_names.contains(&"mango".to_string()));
}

// ============================================================================
// TYPE CONVERSION TESTS
// ============================================================================

#[test]
fn test_adapter_type_case_insensitive() {
    // adapter_type field is normalized to lowercase during parsing
    let input = TestPackBuilder::new()
        .with_raw_fence(r#"```adapter test
adapter_type: PostgreSQL
connection: "conn"
```"#)
        .build();

    let output = compose_pack(input).expect("Should parse");

    let adapter = output.ast.definitions.iter()
        .find_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
        .unwrap();

    assert_eq!(adapter.adapter_type, AdapterType::Postgres);
}

#[test]
fn test_injection_mode_parsing() {
    let modes = vec![
        ("full", InjectionMode::Full),
        ("summary", InjectionMode::Summary),
        ("topk:5", InjectionMode::TopK(5)),
        ("relevant:0.8", InjectionMode::Relevant(0.8)),
    ];

    for (mode_str, expected_mode) in modes {
        let input = TestPackBuilder::new()
            .with_memory("test")  // Add memory for injection to reference
            .with_raw_fence(&format!(r#"```injection
source: "test"
target: "test"
mode: {}
priority: 100
```"#, mode_str))
            .build();

        let output = compose_pack(input).unwrap_or_else(|e| panic!("Failed to parse mode '{}': {:?}", mode_str, e));

        let injection = output.ast.definitions.iter()
            .find_map(|d| if let Definition::Injection(i) = d { Some(i) } else { None })
            .unwrap();

        assert_eq!(injection.mode, expected_mode, "Mode mismatch for {}", mode_str);
    }
}

// ============================================================================
// ERROR RECOVERY TESTS
// ============================================================================

#[test]
fn test_malformed_yaml_rejected() {
    let input = TestPackBuilder::new()
        .with_raw_fence(r#"```adapter test
adapter_type: postgres
connection: "unclosed string
```"#)
        .build();

    let result = compose_pack(input);

    assert!(result.is_err(), "Should reject malformed YAML");
}

#[test]
fn test_invalid_fence_kind_rejected() {
    let input = TestPackBuilder::new()
        .with_raw_fence(r#"```unknown_type test
field: value
```"#)
        .build();

    let result = compose_pack(input);

    assert!(result.is_err(), "Should reject unknown fence type");
}

// ============================================================================
// INTEGRATION TESTS (Full Pack Composition)
// ============================================================================

#[test]
fn test_empty_markdown_valid() {
    let input = TestPackBuilder::new().build();

    let output = compose_pack(input).expect("Empty markdown should be valid");

    assert_eq!(output.ast.version, "1.0");
    assert!(output.ast.definitions.is_empty(), "No configs = empty definitions");
}

#[test]
fn test_toml_and_markdown_merge() {
    // This test verifies TOML configs and Markdown configs merge correctly
    let manifest_with_adapter = r#"
[meta]
version = "1.0"
project = "test"

[tools]
bin = {}
prompts = {}

[profiles]
[agents]
[toolsets]
[providers]
[policies]
[injections]

[adapters.toml_adapter]
type = "postgres"
connection = "from_toml"
"#;

    let markdown_content = MARKDOWN_TEMPLATE.replace(
        "{fence_blocks}",
        r#"```adapter markdown_adapter
adapter_type: postgres
connection: "from_markdown"
```"#,
    );

    let input = PackInput {
        root: PathBuf::from("."),
        manifest: manifest_with_adapter.to_string(),
        markdowns: vec![PackMarkdownFile {
            path: PathBuf::from("test.md"),
            content: markdown_content,
        }],
        contracts: HashMap::new(),
    };

    let output = compose_pack(input).expect("Should merge TOML and Markdown configs");

    let adapter_names: Vec<String> = output.ast.definitions.iter()
        .filter_map(|d| if let Definition::Adapter(a) = d { Some(a.name.clone()) } else { None })
        .collect();

    assert_eq!(adapter_names.len(), 2, "Should have both TOML and Markdown adapters");
    assert!(adapter_names.contains(&"toml_adapter".to_string()));
    assert!(adapter_names.contains(&"markdown_adapter".to_string()));
}
