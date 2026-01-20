#![cfg(feature = "db-tests")]
//! Property-Based Tests for DSL Round-Trip via API
//!
//! **Property 12: DSL Validation Round-Trip**
//!
//! For any valid CALIBER DSL source, parsing then pretty-printing then parsing
//! again SHALL produce an equivalent AST.
//!
//! **Validates: Requirements 11.6, 11.7**

use axum::{body::to_bytes, extract::State, response::IntoResponse, Json};
use caliber_api::db::DbClient;
use caliber_api::routes::dsl;
use caliber_api::types::{ValidateDslRequest, ValidateDslResponse};
use caliber_dsl::{parse, pretty_print, CaliberAst};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

#[path = "support/db.rs"]
mod test_db_support;

async fn call_parse_endpoint(db: DbClient, source: String) -> Result<ValidateDslResponse, String> {
    let response = dsl::parse_dsl(State(db), Json(ValidateDslRequest { source }))
        .await
        .map_err(|e| format!("Parse endpoint failed: {}", e.message))?;

    let response = response.into_response();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| format!("Failed to read response body: {:?}", e))?;

    serde_json::from_slice::<ValidateDslResponse>(&body)
        .map_err(|e| format!("Failed to deserialize ValidateDslResponse: {}", e))
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
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(async {
            let db = test_db_support::test_db_client();

            let response = call_parse_endpoint(db.clone(), source.clone())
                .await
                .map_err(TestCaseError::fail)?;
            prop_assert!(response.valid, "Expected valid DSL response");
            prop_assert!(response.errors.is_empty(), "Expected no parse errors");

            let ast_json = response.ast.clone()
                .ok_or_else(|| TestCaseError::fail("AST should be present"))?;
            let api_ast: CaliberAst = serde_json::from_value(ast_json)
                .map_err(|e| TestCaseError::fail(format!("AST JSON should deserialize into CaliberAst: {}", e)))?;

            let pretty = pretty_print(&api_ast);
            let reparsed = parse(&pretty)
                .map_err(|e| TestCaseError::fail(format!("Pretty-printed DSL should parse: {}", e)))?;
            let original = parse(&source)
                .map_err(|e| TestCaseError::fail(format!("Original DSL should parse: {}", e)))?;

            prop_assert_eq!(reparsed, original);
            Ok::<(), TestCaseError>(())
        })?;
    }
}
