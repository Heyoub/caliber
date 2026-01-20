//! Pretty printer for CALIBER DSL AST

use crate::parser::*;
use std::fmt::{self, Write};

        if let Definition::Injection(injection) = &ast.definitions[0] {
            assert!(injection.filter.is_some());
            // The filter should be an Or expression
            if let Some(FilterExpr::Or(_)) = &injection.filter {
                // OK
            } else {
                return Err(test_parse_error("Expected Or filter expression"));
            }
        } else {
            return Err(test_parse_error("Expected injection definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_schedule_trigger() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                policy scheduled_cleanup {
                    on schedule("0 0 * * *"): [
                        prune(old_data, age > 30d)
                    ]
                }
            }
        "#;
        let ast = parse(source)?;

        if let Definition::Policy(policy) = &ast.definitions[0] {
            assert_eq!(policy.rules[0].trigger, Trigger::Schedule("0 0 * * *".to_string()));
        } else {
            return Err(test_parse_error("Expected policy definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_prune_action() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                policy cleanup {
                    on task_end: [
                        prune(artifacts, age > 7d)
                    ]
                }
            }
        "#;
        let ast = parse(source)?;

        if let Definition::Policy(policy) = &ast.definitions[0] {
            if let Action::Prune { target, criteria } = &policy.rules[0].actions[0] {
                assert_eq!(target, "artifacts");
                if let FilterExpr::Comparison { field, op, .. } = criteria {
                    assert_eq!(field, "age");
                    assert_eq!(*op, CompareOp::Gt);
                } else {
                    return Err(test_parse_error("Expected comparison filter"));
                }
            } else {
                return Err(test_parse_error("Expected prune action"));
            }
        } else {
            return Err(test_parse_error("Expected policy definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_error_line_column() -> Result<(), ParseError> {
        let source = "caliber: \"1.0\" { invalid_keyword }";
        let result = parse(source);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.line >= 1);
        assert!(err.column >= 1);
        Ok(())
    }

    // ========================================================================
    // Pretty Printer Tests
    // ========================================================================

    #[test]
    fn test_pretty_print_minimal() {
        let ast = CaliberAst {
            version: "1.0".to_string(),
            definitions: vec![],
        };
        let output = pretty_print(&ast);
        assert!(output.contains("caliber: \"1.0\""));
    }

    #[test]
    fn test_pretty_print_adapter() {
        let ast = CaliberAst {
            version: "1.0".to_string(),
            definitions: vec![Definition::Adapter(AdapterDef {
                name: "main_db".to_string(),
                adapter_type: AdapterType::Postgres,
                connection: "postgresql://localhost/caliber".to_string(),
                options: vec![],
            })],
        };
        let output = pretty_print(&ast);
        assert!(output.contains("adapter main_db"));
        assert!(output.contains("type: postgres"));
        assert!(output.contains("connection: \"postgresql://localhost/caliber\""));
    }

    #[test]
    fn test_pretty_print_memory() {
        let ast = CaliberAst {
            version: "1.0".to_string(),
            definitions: vec![Definition::Memory(MemoryDef {
                name: "turns".to_string(),
                memory_type: MemoryType::Ephemeral,
                schema: vec![
                    FieldDef {
                        name: "id".to_string(),
                        field_type: FieldType::Uuid,
                        nullable: false,
                        default: None,
                    },
                ],
                retention: Retention::Scope,
                lifecycle: Lifecycle::Explicit,
                parent: None,
                indexes: vec![],
                inject_on: vec![],
                artifacts: vec![],
            })],
        };
        let output = pretty_print(&ast);
        assert!(output.contains("memory turns"));
        assert!(output.contains("type: ephemeral"));
        assert!(output.contains("retention: scope"));
    }

    // ========================================================================
    // Round-Trip Tests
    // ========================================================================

    #[test]
    fn test_round_trip_minimal() -> Result<(), ParseError> {
        let source = r#"caliber: "1.0" {}"#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1.version, ast2.version);
        assert_eq!(ast1.definitions.len(), ast2.definitions.len());
        Ok(())
    }

    #[test]
    fn test_round_trip_adapter() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                adapter main_db {
                    type: postgres
                    connection: "postgresql://localhost/caliber"
                }
            }
        "#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1, ast2);
        Ok(())
    }

    #[test]
    fn test_round_trip_memory() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                memory turns {
                    type: ephemeral
                    schema: {
                        id: uuid
                        content: text
                    }
                    retention: scope
                    lifecycle: explicit
                }
            }
        "#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1, ast2);
        Ok(())
    }

    #[test]
    fn test_parse_defaults_and_index_options() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                memory notes {
                    type: semantic
                    schema: {
                        id: uuid
                        title: text optional = "untitled"
                        score: float = 0.75
                        active: bool = true
                    }
                    retention: persistent
                    lifecycle: explicit
                    index: {
                        embedding: hnsw options: {
                            "m": 16,
                            "ef_construction": 64
                        }
                    }
                }
            }
        "#;

        let ast = parse(source)?;
        let memory = match &ast.definitions[0] {
            Definition::Memory(def) => def,
            _ => return Err(test_parse_error("Expected memory definition")),
        };

        let title = &memory.schema[1];
        assert!(title.nullable);
        assert_eq!(title.default.as_deref(), Some("\"untitled\""));

        let score = &memory.schema[2];
        assert_eq!(score.default.as_deref(), Some("0.75"));

        let active = &memory.schema[3];
        assert_eq!(active.default.as_deref(), Some("true"));

        let index = &memory.indexes[0];
        assert_eq!(index.options.len(), 2);
        assert!(index.options.iter().any(|(k, v)| k == "m" && v == "16"));
        assert!(index.options.iter().any(|(k, v)| k == "ef_construction" && v == "64"));

        let printed = pretty_print(&ast);
        assert!(printed.contains("optional"));
        assert!(printed.contains("= \"untitled\""));
        assert!(printed.contains("options: {"));
        Ok(())
    }

    #[test]
    fn test_round_trip_policy() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                policy cleanup {
                    on scope_close: [
                        summarize(turns)
                        checkpoint(scope)
                    ]
                }
            }
        "#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1, ast2);
        Ok(())
    }

    #[test]
    fn test_round_trip_injection() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                inject notes into context {
                    mode: full
                    priority: 50
                }
            }
        "#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1, ast2);
        Ok(())
    }
}


// ============================================================================
// PROPERTY-BASED TESTS (Task 4.10)
// ============================================================================

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Property 3: DSL round-trip parsing preserves semantics
    // Feature: caliber-core-implementation, Property 3: DSL round-trip parsing preserves semantics
    // Validates: Requirements 5.8
    // ========================================================================

    // Generators for AST types
    fn arb_adapter_type() -> impl Strategy<Value = AdapterType> {
        prop_oneof![
            Just(AdapterType::Postgres),
            Just(AdapterType::Redis),
            Just(AdapterType::Memory),
        ]
    }

    fn arb_memory_type() -> impl Strategy<Value = MemoryType> {
        prop_oneof![
            Just(MemoryType::Ephemeral),
            Just(MemoryType::Working),
            Just(MemoryType::Episodic),
            Just(MemoryType::Semantic),
            Just(MemoryType::Procedural),
            Just(MemoryType::Meta),
        ]
    }

    fn arb_field_type() -> impl Strategy<Value = FieldType> {
        prop_oneof![
            Just(FieldType::Uuid),
            Just(FieldType::Text),
            Just(FieldType::Int),
            Just(FieldType::Float),
            Just(FieldType::Bool),
            Just(FieldType::Timestamp),
            Just(FieldType::Json),
            (0usize..4096).prop_map(|d| FieldType::Embedding(Some(d))),
            Just(FieldType::Embedding(None)),
        ]
    }

    fn arb_retention() -> impl Strategy<Value = Retention> {
        prop_oneof![
            Just(Retention::Persistent),
            Just(Retention::Session),
            Just(Retention::Scope),
            "[0-9]+[smhdw]".prop_map(Retention::Duration),
            (1usize..1000).prop_map(Retention::Max),
        ]
    }

    fn arb_index_type() -> impl Strategy<Value = IndexType> {
        prop_oneof![
            Just(IndexType::Btree),
            Just(IndexType::Hash),
            Just(IndexType::Gin),
            Just(IndexType::Hnsw),
            Just(IndexType::Ivfflat),
        ]
    }

    fn arb_trigger() -> impl Strategy<Value = Trigger> {
        prop_oneof![
            Just(Trigger::TaskStart),
            Just(Trigger::TaskEnd),
            Just(Trigger::ScopeClose),
            Just(Trigger::TurnEnd),
            Just(Trigger::Manual),
            // Simple cron-like patterns for schedule
            "[0-9]+ [0-9]+ \\* \\* \\*".prop_map(Trigger::Schedule),
        ]
    }

    fn arb_injection_mode() -> impl Strategy<Value = InjectionMode> {
        prop_oneof![
            Just(InjectionMode::Full),
            Just(InjectionMode::Summary),
            (1usize..100).prop_map(InjectionMode::TopK),
            (0.0f32..1.0f32).prop_map(InjectionMode::Relevant),
        ]
    }

    fn arb_compare_op() -> impl Strategy<Value = CompareOp> {
        prop_oneof![
            Just(CompareOp::Eq),
            Just(CompareOp::Ne),
            Just(CompareOp::Gt),
            Just(CompareOp::Lt),
            Just(CompareOp::Ge),
            Just(CompareOp::Le),
            Just(CompareOp::Contains),
            Just(CompareOp::In),
        ]
    }

    fn arb_filter_value() -> impl Strategy<Value = FilterValue> {
        prop_oneof![
            "[a-zA-Z0-9_]+".prop_map(FilterValue::String),
            (-1000.0f64..1000.0f64).prop_map(FilterValue::Number),
            any::<bool>().prop_map(FilterValue::Bool),
            Just(FilterValue::Null),
            Just(FilterValue::CurrentTrajectory),
            Just(FilterValue::CurrentScope),
            Just(FilterValue::Now),
        ]
    }

    fn arb_simple_filter_expr() -> impl Strategy<Value = FilterExpr> {
        ("[a-z_]+", arb_compare_op(), arb_filter_value()).prop_map(|(field, op, value)| {
            FilterExpr::Comparison { field, op, value }
        })
    }

    fn arb_identifier() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_string())
    }

    fn arb_safe_string() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_/:.-]{1,50}".prop_map(|s| s.to_string())
    }

    fn arb_field_def() -> impl Strategy<Value = FieldDef> {
        (arb_identifier(), arb_field_type()).prop_map(|(name, field_type)| FieldDef {
            name,
            field_type,
            nullable: false,
            default: None,
        })
    }

    fn arb_index_def() -> impl Strategy<Value = IndexDef> {
        (arb_identifier(), arb_index_type()).prop_map(|(field, index_type)| IndexDef {
            field,
            index_type,
            options: vec![],
        })
    }

    fn arb_adapter_def() -> impl Strategy<Value = AdapterDef> {
        (arb_identifier(), arb_adapter_type(), arb_safe_string()).prop_map(
            |(name, adapter_type, connection)| AdapterDef {
                name,
                adapter_type,
                connection,
                options: vec![],
            },
        )
    }

    fn arb_memory_def() -> impl Strategy<Value = MemoryDef> {
        (
            arb_identifier(),
            arb_memory_type(),
            prop::collection::vec(arb_field_def(), 0..3),
            arb_retention(),
            prop::collection::vec(arb_index_def(), 0..2),
        )
            .prop_map(|(name, memory_type, schema, retention, indexes)| MemoryDef {
                name,
                memory_type,
                schema,
                retention,
                lifecycle: Lifecycle::Explicit,
                parent: None,
                indexes,
                inject_on: vec![],
                artifacts: vec![],
            })
    }

    fn arb_simple_action() -> impl Strategy<Value = Action> {
        prop_oneof![
            arb_identifier().prop_map(Action::Summarize),
            arb_identifier().prop_map(Action::ExtractArtifacts),
            arb_identifier().prop_map(Action::Checkpoint),
            arb_safe_string().prop_map(Action::Notify),
        ]
    }

    fn arb_policy_rule() -> impl Strategy<Value = PolicyRule> {
        (arb_trigger(), prop::collection::vec(arb_simple_action(), 1..3)).prop_map(
            |(trigger, actions)| PolicyRule { trigger, actions },
        )
    }

    fn arb_policy_def() -> impl Strategy<Value = PolicyDef> {
        (arb_identifier(), prop::collection::vec(arb_policy_rule(), 1..3))
            .prop_map(|(name, rules)| PolicyDef { name, rules })
    }

    fn arb_injection_def() -> impl Strategy<Value = InjectionDef> {
        (
            arb_identifier(),
            arb_identifier(),
            arb_injection_mode(),
            1i32..100i32,
            prop::option::of(arb_simple_filter_expr()),
        )
            .prop_map(|(source, target, mode, priority, filter)| InjectionDef {
                source,
                target,
                mode,
                priority,
                max_tokens: None,
                filter,
            })
    }

    fn arb_definition() -> impl Strategy<Value = Definition> {
        prop_oneof![
            arb_adapter_def().prop_map(Definition::Adapter),
            arb_memory_def().prop_map(Definition::Memory),
            arb_policy_def().prop_map(Definition::Policy),
            arb_injection_def().prop_map(Definition::Injection),
        ]
    }

    fn arb_caliber_ast() -> impl Strategy<Value = CaliberAst> {
        (
            "[0-9]+\\.[0-9]+".prop_map(|s| s.to_string()),
            prop::collection::vec(arb_definition(), 0..5),
        )
            .prop_map(|(version, definitions)| CaliberAst {
                version,
                definitions,
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 3: For any valid CaliberAst, pretty-printing then parsing
        /// SHALL produce an equivalent AST (round-trip property)
        #[test]
        fn prop_round_trip_preserves_semantics(ast in arb_caliber_ast()) {
            let printed = pretty_print(&ast);
            let parsed = parse(&printed);

            match parsed {
                Ok(parsed_ast) => {
                    prop_assert_eq!(ast, parsed_ast, "Round-trip did not preserve AST semantics");
                }
                Err(err) => {
                    prop_assert!(false, "Failed to parse pretty-printed AST: {:?}\nPrinted:\n{}", err, printed);
                }
            }
        }

        /// Property 4: For any input containing invalid characters,
        /// the Lexer SHALL produce at least one TokenKind::Error
        #[test]
        fn prop_lexer_error_on_invalid_chars(
            prefix in "[a-z]+",
            invalid in "[^a-zA-Z0-9_{}()\\[\\]:,.<>=!~\"\\s/-]+",
            suffix in "[a-z]*"
        ) {
            // Skip if invalid is empty
            prop_assume!(!invalid.is_empty());

            let source = format!("{} {} {}", prefix, invalid, suffix);
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();

            let has_error = tokens.iter().any(|t| matches!(t.kind, TokenKind::Error(_)));
            prop_assert!(has_error, "Expected error token for invalid input: {}", source);
        }

        /// Property: Lexer always produces Eof as last token
        #[test]
        fn prop_lexer_always_ends_with_eof(source in ".*") {
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();

            prop_assert!(!tokens.is_empty(), "Token list should not be empty");
            prop_assert!(
                matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::Eof)),
                "Last token should be Eof"
            );
        }

        /// Property: Span positions are valid
        #[test]
        fn prop_span_positions_valid(source in "[a-z ]+") {
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();

            for token in &tokens {
                prop_assert!(token.span.start <= token.span.end, "Span start should be <= end");
                prop_assert!(token.span.line >= 1, "Line should be >= 1");
                prop_assert!(token.span.column >= 1, "Column should be >= 1");
            }
        }
    }
}
