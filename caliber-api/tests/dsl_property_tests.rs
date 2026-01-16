//! Property-Based Tests for DSL Validation Round-Trip
//!
//! **Property 12: DSL Validation Round-Trip**
//!
//! For any valid CALIBER DSL source, parsing then pretty-printing then parsing
//! again SHALL produce an equivalent AST.
//!
//! **Validates: Requirements 11.6, 11.7**

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use caliber_api::{
    db::{DbClient, DbConfig},
    routes::dsl,
    types::{ValidateDslRequest, ValidateDslResponse},
};
use caliber_dsl::{pretty_print, CaliberAst};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use serde::de::DeserializeOwned;
use std::sync::Arc;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

/// Create a test database client (unused by DSL routes but required for state).
fn test_db_client() -> DbClient {
    let config = DbConfig::from_env();
    DbClient::from_config(&config).expect("Failed to create database client")
}

async fn extract_json<T: DeserializeOwned>(response: impl IntoResponse) -> T {
    let response = response.into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&body).expect("Failed to parse JSON response")
}

// ============================================================================
// DSL STRATEGIES
// ============================================================================

fn valid_dsl_strategy() -> impl Strategy<Value = String> {
    let samples = vec![
        r#"caliber: "0.1.0" {
  adapter primary {
    type: postgres
    connection: "postgres://localhost:5432/caliber"
  }
}"#
        .to_string(),
        r#"caliber: "0.1.0" {
  memory notes {
    type: semantic
    retention: persistent
  }
}"#
        .to_string(),
        r#"caliber: "0.1.0" {
  memory scope {
    type: working
    schema: {
      scope_id: uuid,
      summary: text
    }
    retention: session
    lifecycle: auto_close(scope_close)
  }
}"#
        .to_string(),
        r#"caliber: "0.1.0" {
  policy summarize {
    on scope_close: [summarize(scope)]
  }
}"#
        .to_string(),
        r#"caliber: "0.1.0" {
  inject notes into context {
    mode: full
    priority: 10
  }
}"#
        .to_string(),
    ];

    prop::sample::select(samples)
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 12: DSL Validation Round-Trip**
    #[test]
    fn prop_dsl_round_trip(source in valid_dsl_strategy()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = test_db_client();
            let state = Arc::new(dsl::DslState::new(db));

            // Parse original source through API
            let parsed: ValidateDslResponse = extract_json(
                dsl::parse_dsl(
                    State(state.clone()),
                    Json(ValidateDslRequest { source: source.clone() })
                )
                .await?
            )
            .await;

            prop_assert!(parsed.valid, "DSL source should be valid");
            let ast_value = parsed.ast.ok_or_else(|| TestCaseError::fail("AST missing for valid DSL"))?;
            let ast: CaliberAst = serde_json::from_value(ast_value)
                .map_err(|e| TestCaseError::fail(format!("Failed to deserialize AST: {}", e)))?;

            // Pretty-print and parse again via API
            let pretty = pretty_print(&ast);
            let reparsed: ValidateDslResponse = extract_json(
                dsl::parse_dsl(
                    State(state),
                    Json(ValidateDslRequest { source: pretty })
                )
                .await?
            )
            .await;

            prop_assert!(reparsed.valid, "Pretty-printed DSL should be valid");
            let ast_value = reparsed.ast.ok_or_else(|| TestCaseError::fail("AST missing after round-trip"))?;
            let ast_roundtrip: CaliberAst = serde_json::from_value(ast_value)
                .map_err(|e| TestCaseError::fail(format!("Failed to deserialize AST: {}", e)))?;

            prop_assert_eq!(ast, ast_roundtrip);

            Ok(())
        })?;
    }
}

mod dsl_round_trip_api {
//! Property-Based Tests for DSL Round-Trip via API
//!
//! **Property 12: DSL Validation Round-Trip**
//!
//! For any valid CALIBER DSL source, parsing then pretty-printing then parsing
//! again SHALL produce an equivalent AST.
//!
//! **Validates: Requirements 11.6, 11.7**

use axum::{body::to_bytes, extract::State, response::IntoResponse, Json};
use caliber_api::{
    db::{DbClient, DbConfig},
    routes::dsl,
    types::{ValidateDslRequest, ValidateDslResponse},
};
use caliber_dsl::{parse, pretty_print, CaliberAst};
use proptest::prelude::*;
use std::sync::Arc;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

fn test_db_client() -> DbClient {
    let config = DbConfig::from_env();
    DbClient::from_config(&config).expect("Failed to create database client")
}
}

async fn call_parse_endpoint(
    state: Arc<dsl::DslState>,
    source: String,
) -> ValidateDslResponse {
    let response = dsl::parse_dsl(
        State(state),
        Json(ValidateDslRequest { source }),
    )
    .await
    .expect("Parse endpoint should return ApiResult");

    let response = response.into_response();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    serde_json::from_slice::<ValidateDslResponse>(&body)
        .expect("Failed to deserialize ValidateDslResponse")
}

// ============================================================================
// PROPERTY TEST STRATEGIES
// ============================================================================

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

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Property 12: DSL Validation Round-Trip**
    #[test]
    fn prop_dsl_round_trip_via_api(source in dsl_source_strategy()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = test_db_client();
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
            Ok(())
        })?;
    }
}
