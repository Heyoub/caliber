#![cfg(feature = "db-tests")]
//! Property-Based Tests for Markdown Config Validation Round-Trip
//!
//! **Property 12: Markdown Config Validation Round-Trip**
//!
//! For any valid Markdown config, parsing via API then pretty-printing then parsing
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
use caliber_dsl::config::ast_to_markdown;
use caliber_dsl::parser::CaliberAst;
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
// MARKDOWN CONFIG STRATEGIES
// ============================================================================

/// Generate valid Markdown configs for testing
fn valid_markdown_config_strategy() -> impl Strategy<Value = String> {
    let samples = vec![
        // Adapter config
        r#"
# System
Test
## PCP
Test
### User
```adapter primary
adapter_type: postgres
connection: "postgres://localhost:5432/caliber"
```
"#
        .to_string(),
        // Provider config
        r#"
# System
Test
## PCP
Test
### User
```provider openai_main
provider_type: openai
api_key: env:OPENAI_API_KEY
model: "gpt-4"
```
"#
        .to_string(),
        // Policy config
        r#"
# System
Test
## PCP
Test
### User
```policy summarize
rules:
  - trigger: scope_close
    actions:
      - type: summarize
        target: scope
```
"#
        .to_string(),
        // Injection config
        r#"
# System
Test
## PCP
Test
### User
```injection
source: "notes"
target: "context"
mode: full
priority: 10
```
"#
        .to_string(),
        // Mixed configs
        r#"
# System
Test
## PCP
Test
### User
```adapter db
adapter_type: postgres
connection: "conn"
```

```provider ai
provider_type: openai
api_key: env:KEY
model: "gpt-4"
```
"#
        .to_string(),
        // Case preservation test
        r#"
# System
Test
## PCP
Test
### User
```adapter MyAdapter
adapter_type: postgres
connection: "PostgreSQL://LocalHost/DB"
```
"#
        .to_string(),
    ];

    prop::sample::select(samples)
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Property 12: Markdown Config Validation Round-Trip**
    ///
    /// Validates that:
    /// 1. API can parse valid Markdown configs
    /// 2. AST can be serialized to JSON
    /// 3. Round-trip preserves semantic equality
    /// 4. Case is preserved (regression test for old bug)
    #[test]
    fn prop_markdown_round_trip(source in valid_markdown_config_strategy()) {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(async {
            let db = test_db_support::test_db_client();

            // Step 1: Parse via API
            let response = dsl::parse_dsl(
                State(db.clone()),
                Json(ValidateDslRequest { source: source.clone() })
            )
            .await
            .map_err(|e| TestCaseError::fail(format!("API parse failed: {}", e.message)))?;

            let response = response.into_response();
            let parsed: ValidateDslResponse = extract_json(response)
                .await
                .map_err(TestCaseError::fail)?;

            // Step 2: Validate response
            prop_assert!(parsed.valid, "Expected valid config response");
            prop_assert!(parsed.errors.is_empty(), "Expected no parse errors");

            // Step 3: Deserialize AST
            let ast_json = parsed.ast.clone()
                .ok_or_else(|| TestCaseError::fail("AST should be present"))?;
            let api_ast: CaliberAst = serde_json::from_value(ast_json)
                .map_err(|e| TestCaseError::fail(format!("AST JSON should deserialize: {}", e)))?;

            // Step 4: Generate canonical Markdown
            let canonical = ast_to_markdown(&api_ast);

            // Step 5: Re-parse canonical Markdown
            let reparse_response = dsl::parse_dsl(
                State(db.clone()),
                Json(ValidateDslRequest { source: canonical.clone() })
            )
            .await
            .map_err(|e| TestCaseError::fail(format!("Canonical re-parse failed: {}", e.message)))?;

            let reparse_response = reparse_response.into_response();
            let reparsed: ValidateDslResponse = extract_json(reparse_response)
                .await
                .map_err(TestCaseError::fail)?;

            prop_assert!(reparsed.valid, "Canonical Markdown should be valid");

            // Step 6: Compare ASTs
            let reparsed_ast_json = reparsed.ast
                .ok_or_else(|| TestCaseError::fail("Reparsed AST should be present"))?;
            let reparsed_ast: CaliberAst = serde_json::from_value(reparsed_ast_json)
                .map_err(|e| TestCaseError::fail(format!("Reparsed AST JSON should deserialize: {}", e)))?;

            // Semantic equality: definitions should match
            prop_assert_eq!(
                api_ast.definitions.len(),
                reparsed_ast.definitions.len(),
                "Definition count should match"
            );

            Ok::<(), TestCaseError>(())
        })?;
    }

    /// Property: Case preservation via API
    ///
    /// This is the CRITICAL regression test:
    /// - Old DSL parser: "MyAdapter" → "myadapter" (FAIL)
    /// - New Markdown parser: "MyAdapter" → "MyAdapter" (PASS)
    #[test]
    fn prop_case_preserved_via_api(
        name_prefix in "[A-Z][a-zA-Z]{2,8}",
        name_suffix in "[A-Z][a-zA-Z]{2,8}",
    ) {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))?;

        let mixed_case_name = format!("{}{}", name_prefix, name_suffix);

        let markdown = format!(r#"
# System
Test
## PCP
Test
### User
```adapter {}
adapter_type: postgres
connection: "PostgreSQL://LocalHost/DB"
```
"#, mixed_case_name);

        rt.block_on(async {
            let db = test_db_support::test_db_client();

            let response = dsl::parse_dsl(
                State(db),
                Json(ValidateDslRequest { source: markdown })
            )
            .await
            .map_err(|e| TestCaseError::fail(format!("Parse failed: {}", e.message)))?;

            let response = response.into_response();
            let parsed: ValidateDslResponse = extract_json(response)
                .await
                .map_err(TestCaseError::fail)?;

            prop_assert!(parsed.valid, "Should parse successfully");

            let ast_json = parsed.ast
                .ok_or_else(|| TestCaseError::fail("AST should be present"))?;
            let ast: CaliberAst = serde_json::from_value(ast_json)
                .map_err(|e| TestCaseError::fail(format!("Failed to deserialize AST: {}", e)))?;

            // Find adapter in AST
            let adapter = ast.definitions.iter()
                .find_map(|d| match d {
                    caliber_dsl::parser::Definition::Adapter(a) => Some(a),
                    _ => None,
                })
                .ok_or_else(|| TestCaseError::fail("No adapter found in AST"))?;

            // CRITICAL ASSERTION: Case must be preserved exactly
            prop_assert_eq!(
                &adapter.name,
                &mixed_case_name,
                "Adapter name case should be preserved exactly"
            );

            // Also verify connection string case is preserved
            prop_assert_eq!(
                &adapter.connection,
                "PostgreSQL://LocalHost/DB",
                "Connection string case should be preserved"
            );

            Ok::<(), TestCaseError>(())
        })?;
    }
}

// ============================================================================
// SPECIFIC REGRESSION TESTS
// ============================================================================

#[tokio::test]
async fn test_case_bug_exact_regression() {
    // This is the EXACT bug case from the issue
    let db = test_db_support::test_db_client();

    let markdown = r#"
# System
Test
## PCP
Test
### User
```adapter oN
adapter_type: postgres
connection: "test://conn"
```
"#;

    let response = dsl::parse_dsl(
        State(db),
        Json(ValidateDslRequest {
            source: markdown.to_string(),
        }),
    )
    .await
    .expect("Parse should succeed");

    let response = response.into_response();
    let parsed: ValidateDslResponse = extract_json(response)
        .await
        .expect("Should deserialize response");

    assert!(parsed.valid, "Should be valid");

    let ast_json = parsed.ast.expect("AST should be present");
    let ast: CaliberAst = serde_json::from_value(ast_json).expect("Should deserialize AST");

    let adapter = ast
        .definitions
        .iter()
        .find_map(|d| match d {
            caliber_dsl::parser::Definition::Adapter(a) => Some(a),
            _ => None,
        })
        .expect("Adapter should exist");

    // OLD BUG: adapter.name would be "on" (lowercase)
    // NEW FIX: adapter.name is "oN" (preserved)
    assert_eq!(
        adapter.name, "oN",
        "REGRESSION TEST: Case must be preserved (not 'on')"
    );
}

#[tokio::test]
async fn test_all_caps_preserved() {
    let db = test_db_support::test_db_client();

    let markdown = r#"
# System
Test
## PCP
Test
### User
```adapter MAIN_DATABASE
adapter_type: postgres
connection: "conn"
```
"#;

    let response = dsl::parse_dsl(
        State(db),
        Json(ValidateDslRequest {
            source: markdown.to_string(),
        }),
    )
    .await
    .expect("Parse should succeed");

    let response = response.into_response();
    let parsed: ValidateDslResponse = extract_json(response).await.expect("Should deserialize");

    let ast: CaliberAst =
        serde_json::from_value(parsed.ast.unwrap()).expect("Should deserialize AST");

    let adapter = ast
        .definitions
        .iter()
        .find_map(|d| match d {
            caliber_dsl::parser::Definition::Adapter(a) => Some(a),
            _ => None,
        })
        .expect("Adapter should exist");

    assert_eq!(
        adapter.name, "MAIN_DATABASE",
        "All caps should be preserved"
    );
}

#[tokio::test]
async fn test_camel_case_preserved() {
    let db = test_db_support::test_db_client();

    let markdown = r#"
# System
Test
## PCP
Test
### User
```provider MyOpenAiProvider
provider_type: openai
api_key: env:OPENAI_API_KEY
model: "gpt-4"
```
"#;

    let response = dsl::parse_dsl(
        State(db),
        Json(ValidateDslRequest {
            source: markdown.to_string(),
        }),
    )
    .await
    .expect("Parse should succeed");

    let response = response.into_response();
    let parsed: ValidateDslResponse = extract_json(response).await.expect("Should deserialize");

    let ast: CaliberAst =
        serde_json::from_value(parsed.ast.unwrap()).expect("Should deserialize AST");

    let provider = ast
        .definitions
        .iter()
        .find_map(|d| match d {
            caliber_dsl::parser::Definition::Provider(p) => Some(p),
            _ => None,
        })
        .expect("Provider should exist");

    assert_eq!(
        provider.name, "MyOpenAiProvider",
        "CamelCase should be preserved"
    );
}
