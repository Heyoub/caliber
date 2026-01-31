//! Property-Based Tests for Markdown Config Round-Trip
//!
//! Property: For any valid Markdown config, parsing → AST → Canonical Markdown → parsing
//! SHALL produce an equivalent AST.
//!
//! This validates:
//! - Canonical printer is deterministic
//! - Parser preserves all semantic information
//! - Round-trip is lossless (at AST level, not byte-level)

use caliber_dsl::config::ast_to_markdown;
use caliber_dsl::pack::{compose_pack, PackInput, PackMarkdownFile};
use caliber_dsl::parser::ast::*;
use proptest::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// ARBITRATORS (Generate Random AST Nodes)
// ============================================================================

/// Generate arbitrary adapter type
fn arb_adapter_type() -> impl Strategy<Value = AdapterType> {
    prop_oneof![
        Just(AdapterType::Postgres),
        Just(AdapterType::Redis),
        Just(AdapterType::Memory),
    ]
}

/// Generate arbitrary provider type
fn arb_provider_type() -> impl Strategy<Value = ProviderType> {
    prop_oneof![
        Just(ProviderType::OpenAI),
        Just(ProviderType::Anthropic),
        Just(ProviderType::Custom),
    ]
}

/// Generate arbitrary trigger
fn arb_trigger() -> impl Strategy<Value = Trigger> {
    prop_oneof![
        Just(Trigger::TaskStart),
        Just(Trigger::TaskEnd),
        Just(Trigger::ScopeClose),
        Just(Trigger::TurnEnd),
        Just(Trigger::Manual),
        "[a-z0-9_]+".prop_map(Trigger::Schedule),
    ]
}

/// Generate arbitrary injection mode
fn arb_injection_mode() -> impl Strategy<Value = InjectionMode> {
    prop_oneof![
        Just(InjectionMode::Full),
        Just(InjectionMode::Summary),
        (1..100usize).prop_map(InjectionMode::TopK),
        (0.0..1.0f32).prop_map(InjectionMode::Relevant),
    ]
}

/// Generate arbitrary action
fn arb_action() -> impl Strategy<Value = Action> {
    prop_oneof![
        "[a-z_]+".prop_map(Action::Summarize),
        "[a-z_]+".prop_map(Action::Checkpoint),
        "[a-z_]+".prop_map(Action::ExtractArtifacts),
        "[a-z_]+".prop_map(Action::Notify),
    ]
}

/// Generate arbitrary adapter definition
fn arb_adapter_def() -> impl Strategy<Value = AdapterDef> {
    (
        "[a-zA-Z][a-zA-Z0-9_]*",
        arb_adapter_type(),
        "[a-z]+://[a-z]+/[a-z]+",
    )
        .prop_map(|(name, adapter_type, connection)| AdapterDef {
            name,
            adapter_type,
            connection,
            options: vec![],
        })
}

/// Generate arbitrary provider definition
fn arb_provider_def() -> impl Strategy<Value = ProviderDef> {
    (
        "[a-zA-Z][a-zA-Z0-9_]*",
        arb_provider_type(),
        "[a-z]+",
        "[a-zA-Z0-9-]+",
    )
        .prop_map(|(name, provider_type, key, model)| ProviderDef {
            name,
            provider_type,
            api_key: EnvValue::Literal(key),
            model,
            options: vec![],
        })
}

/// Generate arbitrary policy definition
fn arb_policy_def() -> impl Strategy<Value = PolicyDef> {
    (
        "[a-zA-Z][a-zA-Z0-9_]*",
        arb_trigger(),
        prop::collection::vec(arb_action(), 1..3),
    )
        .prop_map(|(name, trigger, actions)| PolicyDef {
            name,
            rules: vec![PolicyRule { trigger, actions }],
        })
}

/// Generate arbitrary memory definition
#[allow(dead_code)]
fn arb_memory_def() -> impl Strategy<Value = MemoryDef> {
    "[a-zA-Z][a-zA-Z0-9_]*".prop_map(|name| MemoryDef {
        name,
        memory_type: MemoryType::Working,
        retention: Retention::Session,
        lifecycle: Lifecycle::Explicit,
        parent: None,
        schema: vec![],
        indexes: vec![],
        inject_on: vec![],
        artifacts: vec![],
        modifiers: vec![],
    })
}

/// Generate arbitrary injection definition with a matching memory
fn arb_injection_with_memory() -> impl Strategy<Value = (MemoryDef, InjectionDef)> {
    (
        "[a-z][a-z0-9_]*",
        "[a-z_]+",
        arb_injection_mode(),
        0..899i32,  // Pack injections max priority is 899
    )
        .prop_map(|(source, target, mode, priority)| {
            let memory = MemoryDef {
                name: source.clone(),
                memory_type: MemoryType::Working,
                retention: Retention::Session,
                lifecycle: Lifecycle::Explicit,
                parent: None,
                schema: vec![],
                indexes: vec![],
                inject_on: vec![],
                artifacts: vec![],
                modifiers: vec![],
            };
            let injection = InjectionDef {
                source,
                target,
                mode,
                priority,
                max_tokens: None,
                filter: None,
            };
            (memory, injection)
        })
}

/// Generate arbitrary CaliberAst with 1-3 definitions
/// Note: Injections are paired with memories to satisfy validation
fn arb_caliber_ast() -> impl Strategy<Value = CaliberAst> {
    // Generate non-injection definitions
    let non_injection_defs = prop::collection::vec(
        prop_oneof![
            arb_adapter_def().prop_map(Definition::Adapter),
            arb_provider_def().prop_map(Definition::Provider),
            arb_policy_def().prop_map(Definition::Policy),
        ],
        0..3,
    );

    // Optionally include injection with its required memory
    let maybe_injection = prop::option::of(arb_injection_with_memory());

    (non_injection_defs, maybe_injection).prop_map(|(mut defs, injection_opt)| {
        if let Some((memory, injection)) = injection_opt {
            defs.push(Definition::Memory(memory));
            defs.push(Definition::Injection(injection));
        }
        // Ensure we have at least one definition
        if defs.is_empty() {
            defs.push(Definition::Adapter(AdapterDef {
                name: "default".to_string(),
                adapter_type: AdapterType::Memory,
                connection: "mem://default".to_string(),
                options: vec![],
            }));
        }
        CaliberAst {
            version: "1.0".to_string(),
            definitions: defs,
        }
    })
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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

const MARKDOWN_TEMPLATE: &str = r#"
# System

Generated test

## PCP

Generated test

### User

{content}
"#;

fn parse_markdown_to_ast(markdown: &str) -> Result<CaliberAst, String> {
    let input = PackInput {
        root: PathBuf::from("."),
        manifest: MINIMAL_MANIFEST.to_string(),
        markdowns: vec![PackMarkdownFile {
            path: PathBuf::from("test.md"),
            content: markdown.to_string(),
        }],
        contracts: HashMap::new(),
    };

    compose_pack(input)
        .map(|output| output.ast)
        .map_err(|e| e.to_string())
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: AST → Markdown → AST preserves semantic equality
    ///
    /// This is the CORE property that validates the refactor:
    /// - Old DSL parser: FAILED this test (case loss)
    /// - New Markdown parser: PASSES this test (case preserved)
    #[test]
    fn prop_round_trip_ast_semantic_equality(ast in arb_caliber_ast()) {
        // Step 1: AST → Canonical Markdown
        let canonical_markdown = ast_to_markdown(&ast);

        // Step 2: Wrap in markdown template
        let full_markdown = MARKDOWN_TEMPLATE.replace("{content}", &canonical_markdown);

        // Step 3: Canonical Markdown → AST'
        let ast_prime = parse_markdown_to_ast(&full_markdown)
            .map_err(|e| TestCaseError::fail(format!("Failed to parse canonical markdown: {}", e)))?;

        // Step 4: Assert semantic equality
        prop_assert_eq!(ast.version, ast_prime.version, "Version should be preserved");
        prop_assert_eq!(ast.definitions.len(), ast_prime.definitions.len(), "Number of definitions should match");

        // Compare definitions by type and name (order may differ due to sorting)
        for original in ast.definitions.iter() {
            match original {
                Definition::Adapter(a1) => {
                    let found = ast_prime.definitions.iter().find_map(|d| {
                        if let Definition::Adapter(a) = d { if a.name == a1.name { Some(a) } else { None } } else { None }
                    });
                    let a2 = found.ok_or_else(|| TestCaseError::fail(format!("Adapter '{}' not found after round-trip", a1.name)))?;
                    prop_assert_eq!(a1.adapter_type, a2.adapter_type, "Adapter type should be preserved");
                    prop_assert_eq!(&a1.connection, &a2.connection, "Adapter connection should be preserved");
                }
                Definition::Provider(p1) => {
                    let found = ast_prime.definitions.iter().find_map(|d| {
                        if let Definition::Provider(p) = d { if p.name == p1.name { Some(p) } else { None } } else { None }
                    });
                    let p2 = found.ok_or_else(|| TestCaseError::fail(format!("Provider '{}' not found after round-trip", p1.name)))?;
                    prop_assert_eq!(p1.provider_type, p2.provider_type, "Provider type should be preserved");
                    prop_assert_eq!(&p1.model, &p2.model, "Provider model should be preserved");
                }
                Definition::Policy(pol1) => {
                    let found = ast_prime.definitions.iter().find_map(|d| {
                        if let Definition::Policy(p) = d { if p.name == pol1.name { Some(p) } else { None } } else { None }
                    });
                    let pol2 = found.ok_or_else(|| TestCaseError::fail(format!("Policy '{}' not found after round-trip", pol1.name)))?;
                    prop_assert_eq!(pol1.rules.len(), pol2.rules.len(), "Policy rules count should match");
                }
                Definition::Injection(i1) => {
                    let found = ast_prime.definitions.iter().find_map(|d| {
                        if let Definition::Injection(i) = d { if i.source == i1.source && i.target == i1.target { Some(i) } else { None } } else { None }
                    });
                    let i2 = found.ok_or_else(|| TestCaseError::fail(format!("Injection source='{}' target='{}' not found after round-trip", i1.source, i1.target)))?;
                    prop_assert_eq!(&i1.mode, &i2.mode, "Injection mode should be preserved");
                    prop_assert_eq!(i1.priority, i2.priority, "Injection priority should be preserved");
                }
                Definition::Memory(m1) => {
                    let found = ast_prime.definitions.iter().find_map(|d| {
                        if let Definition::Memory(m) = d { if m.name == m1.name { Some(m) } else { None } } else { None }
                    });
                    let m2 = found.ok_or_else(|| TestCaseError::fail(format!("Memory '{}' not found after round-trip", m1.name)))?;
                    prop_assert_eq!(&m1.memory_type, &m2.memory_type, "Memory type should be preserved");
                    prop_assert_eq!(&m1.retention, &m2.retention, "Memory retention should be preserved");
                    prop_assert_eq!(&m1.lifecycle, &m2.lifecycle, "Memory lifecycle should be preserved");
                }
                _ => {
                    // Skip other definition types for now
                }
            }
        }
    }

    /// Property: Canonical Markdown → AST → Canonical Markdown is stable
    ///
    /// This validates that the canonical printer is deterministic:
    /// - Same AST always produces same Markdown
    /// - Field ordering is stable
    /// - Alphabetical sorting is applied
    #[test]
    fn prop_canonical_markdown_stability(ast in arb_caliber_ast()) {
        // Generate canonical markdown twice
        let canonical1 = ast_to_markdown(&ast);
        let canonical2 = ast_to_markdown(&ast);

        // Should be byte-identical
        prop_assert_eq!(&canonical1, &canonical2, "Canonical printer should be deterministic");

        // Parse → regenerate → should still be identical
        let full_markdown = MARKDOWN_TEMPLATE.replace("{content}", &canonical1);
        let ast_prime = parse_markdown_to_ast(&full_markdown)
            .map_err(|e| TestCaseError::fail(format!("Failed to parse: {}", e)))?;

        let canonical3 = ast_to_markdown(&ast_prime);

        prop_assert_eq!(&canonical1, &canonical3, "Round-trip should preserve canonical form");
    }

    /// Property: Case preservation in names
    ///
    /// This is the REGRESSION TEST for the original bug:
    /// - Old parser: "MyAdapter" → "myadapter" (FAIL)
    /// - New parser: "MyAdapter" → "MyAdapter" (PASS)
    #[test]
    fn prop_case_preserved_in_names(
        name in "[a-zA-Z][a-zA-Z0-9_]*",
        adapter_type in arb_adapter_type(),
    ) {
        let ast = CaliberAst {
            version: "1.0".to_string(),
            definitions: vec![Definition::Adapter(AdapterDef {
                name: name.clone(),
                adapter_type,
                connection: "test://conn".to_string(),
                options: vec![],
            })],
        };

        let markdown = ast_to_markdown(&ast);
        let full_markdown = MARKDOWN_TEMPLATE.replace("{content}", &markdown);

        let ast_prime = parse_markdown_to_ast(&full_markdown)
            .map_err(|e| TestCaseError::fail(format!("Parse failed: {}", e)))?;

        let adapter = ast_prime.definitions.iter()
            .find_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
            .ok_or_else(|| TestCaseError::fail("No adapter found"))?;

        prop_assert_eq!(&adapter.name, &name, "Case should be preserved exactly");
    }

    /// Property: Unknown fields are rejected
    ///
    /// Validates that serde's deny_unknown_fields catches typos
    #[test]
    fn prop_unknown_fields_rejected(
        field_name in "[a-z_]{5,15}",
        field_value in ".*",
    ) {
        // Inject unknown field into valid YAML
        let markdown = format!(r#"
# System
Test
## PCP
Test
### User
```adapter test
adapter_type: postgres
connection: "conn"
{}: "{}"
```
"#, field_name, field_value.replace("\"", "\\\""));

        let result = parse_markdown_to_ast(&markdown);

        // Should fail if field is truly unknown
        // (Will pass if field happens to be valid - that's OK)
        if !["adapter_type", "connection", "options", "name"].contains(&field_name.as_str()) {
            prop_assert!(result.is_err(), "Unknown field '{}' should be rejected", field_name);
        }
    }
}

// ============================================================================
// SPECIFIC CASE TESTS (Known Problematic Cases)
// ============================================================================

#[test]
fn test_case_bug_regression() {
    // This is the EXACT case that failed with the old parser
    let ast = CaliberAst {
        version: "1.0".to_string(),
        definitions: vec![Definition::Adapter(AdapterDef {
            name: "oN".to_string(), // Mixed case
            adapter_type: AdapterType::Postgres,
            connection: "test://conn".to_string(),
            options: vec![],
        })],
    };

    let markdown = ast_to_markdown(&ast);
    let full_markdown = MARKDOWN_TEMPLATE.replace("{content}", &markdown);

    let ast_prime = parse_markdown_to_ast(&full_markdown).expect("Parse should succeed");

    let adapter = ast_prime.definitions.iter()
        .find_map(|d| if let Definition::Adapter(a) = d { Some(a) } else { None })
        .expect("Adapter should exist");

    // OLD PARSER: Would fail here (adapter.name == "on")
    // NEW PARSER: Passes (adapter.name == "oN")
    assert_eq!(adapter.name, "oN", "Case regression test: 'oN' should not become 'on'");
}

#[test]
fn test_all_caps_name() {
    let ast = CaliberAst {
        version: "1.0".to_string(),
        definitions: vec![Definition::Adapter(AdapterDef {
            name: "MAIN_DB".to_string(),
            adapter_type: AdapterType::Postgres,
            connection: "conn".to_string(),
            options: vec![],
        })],
    };

    let markdown = ast_to_markdown(&ast);
    let full_markdown = MARKDOWN_TEMPLATE.replace("{content}", &markdown);
    let ast_prime = parse_markdown_to_ast(&full_markdown).unwrap();

    let adapter = ast_prime.definitions[0].as_adapter().unwrap();
    assert_eq!(adapter.name, "MAIN_DB");
}

#[test]
fn test_camel_case_name() {
    let ast = CaliberAst {
        version: "1.0".to_string(),
        definitions: vec![Definition::Provider(ProviderDef {
            name: "MyOpenAiProvider".to_string(),
            provider_type: ProviderType::OpenAI,
            api_key: EnvValue::Literal("key".to_string()),
            model: "gpt-4".to_string(),
            options: vec![],
        })],
    };

    let markdown = ast_to_markdown(&ast);
    let full_markdown = MARKDOWN_TEMPLATE.replace("{content}", &markdown);
    let ast_prime = parse_markdown_to_ast(&full_markdown).unwrap();

    let provider = ast_prime.definitions[0].as_provider().unwrap();
    assert_eq!(provider.name, "MyOpenAiProvider");
}

// ============================================================================
// HELPER TRAIT (Ergonomic Definition Unwrapping)
// ============================================================================

trait DefinitionExt {
    fn as_adapter(&self) -> Option<&AdapterDef>;
    fn as_provider(&self) -> Option<&ProviderDef>;
}

impl DefinitionExt for Definition {
    fn as_adapter(&self) -> Option<&AdapterDef> {
        if let Definition::Adapter(a) = self {
            Some(a)
        } else {
            None
        }
    }

    fn as_provider(&self) -> Option<&ProviderDef> {
        if let Definition::Provider(p) = self {
            Some(p)
        } else {
            None
        }
    }
}
