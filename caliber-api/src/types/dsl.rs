//! DSL-related API types

use serde::{Deserialize, Serialize};

/// Request to validate DSL source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ValidateDslRequest {
    /// DSL source code
    pub source: String,
}

/// Request to parse DSL source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ParseDslRequest {
    /// DSL source code to parse
    pub source: String,
}

/// Response from DSL validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ValidateDslResponse {
    /// Whether the DSL is valid
    pub valid: bool,
    /// Parse errors (if any)
    pub errors: Vec<ParseErrorResponse>,
    /// Parsed AST (if valid)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub ast: Option<serde_json::Value>,
}

/// Parse error details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ParseErrorResponse {
    /// Line number where error occurred
    pub line: usize,
    /// Column number where error occurred
    pub column: usize,
    /// Error message
    pub message: String,
}
