//! Property-Based Tests for DSL Round-Trip via API
//!
//! **Property 12: DSL Validation Round-Trip**
//!
//! For any valid CALIBER DSL source, parsing then pretty-printing then parsing
//! again SHALL produce an equivalent AST.
//!
//! **Validates: Requirements 11.6, 11.7**

use axum::{body::to_bytes, extract::State, response::IntoResponse, Json};
use caliber_api::routes::dsl;
use caliber_api::types::{ValidateDslRequest, ValidateDslResponse};
use caliber_dsl::{parse, pretty_print, CaliberAst};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use std::sync::Arc;

#[path = "support/db.rs"]
mod test_db_support;

async fn call_parse_endpoint(
    state: Arc<dsl::DslState>,
    source: String,
) -> ValidateDslResponse {
    let response = dsl::parse_dsl(State(state), Json(ValidateDslRequest { source }))
        .await
        .expect("Parse endpoint should return ApiResult");

    let response = response.into_response();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    serde_json::from_slice::<ValidateDslResponse>(&body)
        .expect("Failed to deserialize ValidateDslResponse")
}

fn dsl_source_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(r#"caliber: "1.0" {}"#.to_string()),
        Just(r#"
            caliber: "1.0" {
                adapter main_db {
                    type: postgres
                    connection: "postgresql://localhost/caliber"
                }
            }
        "#.to_string()),
        Just(r#"
            caliber: "1.0" {
                memory turns {
                    type: ephemeral
                    schema: {
                        id: uuid
                        content: text
                        embedding: embedding(1536)
                    }
                    retention: scope
                    lifecycle: explicit
                }
            }
        "#.to_string()),
        Just(r#"
            caliber: "1.0" {
                policy cleanup {
                    on scope_close: [
                        summarize(turns)
                        checkpoint(scope)
                    ]
                }
            }
        "#.to_string()),
        Just(r#"
            caliber: "1.0" {
                inject notes into context {
                    mode: relevant(0.8)
                    priority: 80
                    max_tokens: 2000
                    filter: category = "important"
                }
            }
        "#.to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Property 12: DSL Validation Round-Trip**
    #[test]
    fn prop_dsl_round_trip_via_api(source in dsl_source_strategy()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = test_db_support::test_db_client();
            let state = Arc::new(dsl::DslState::new(db));

            let response = call_parse_endpoint(state, source.clone()).await;
            prop_assert!(response.valid, "Expected valid DSL response");
            prop_assert!(response.errors.is_empty(), "Expected no parse errors");

            let ast_json = response.ast.clone().expect("AST should be present");
            let api_ast: CaliberAst = serde_json::from_value(ast_json)
                .expect("AST JSON should deserialize into CaliberAst");

            let pretty = pretty_print(&api_ast);
            let reparsed = parse(&pretty).expect("Pretty-printed DSL should parse");
            let original = parse(&source).expect("Original DSL should parse");

            prop_assert_eq!(reparsed, original);
            Ok::<(), TestCaseError>(())
        })?;
    }
}
