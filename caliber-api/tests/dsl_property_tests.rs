#![cfg(feature = "db-tests")]
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
    routes::dsl,
    types::{ValidateDslRequest, ValidateDslResponse},
};
use caliber_dsl::{pretty_print, CaliberAst};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use serde::de::DeserializeOwned;
#[path = "support/db.rs"]
mod test_db_support;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

async fn extract_json<T: DeserializeOwned>(response: impl IntoResponse) -> Result<T, String> {
    let response = response.into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| format!("Failed to read response body: {:?}", e))?;
    serde_json::from_slice(&body).map_err(|e| format!("Failed to parse JSON response: {}", e))
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
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(async {
            let db = test_db_support::test_db_client();
            // Parse original source through API
            let parsed: ValidateDslResponse = extract_json(
                dsl::parse_dsl(
                    State(db.clone()),
                    Json(ValidateDslRequest { source: source.clone() })
                )
                .await?
            )
            .await
            .map_err(TestCaseError::fail)?;

            prop_assert!(parsed.valid, "DSL source should be valid");
            let ast_value = parsed.ast.ok_or_else(|| TestCaseError::fail("AST missing for valid DSL"))?;
            let ast: CaliberAst = serde_json::from_value(ast_value)
                .map_err(|e| TestCaseError::fail(format!("Failed to deserialize AST: {}", e)))?;

            // Pretty-print and parse again via API
            let pretty = pretty_print(&ast);
            let reparsed: ValidateDslResponse = extract_json(
                dsl::parse_dsl(
                    State(db.clone()),
                    Json(ValidateDslRequest { source: pretty })
                )
                .await?
            )
            .await
            .map_err(TestCaseError::fail)?;

            prop_assert!(reparsed.valid, "Pretty-printed DSL should be valid");
            let ast_value = reparsed.ast.ok_or_else(|| TestCaseError::fail("AST missing after round-trip"))?;
            let ast_roundtrip: CaliberAst = serde_json::from_value(ast_value)
                .map_err(|e| TestCaseError::fail(format!("Failed to deserialize AST: {}", e)))?;

            prop_assert_eq!(ast, ast_roundtrip);

            Ok::<(), TestCaseError>(())
        })?;
    }
}
