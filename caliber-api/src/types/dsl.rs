//! DSL-related API types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

// ============================================================================
// COMPILE REQUEST/RESPONSE
// ============================================================================

/// Request to compile DSL source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CompileDslRequest {
    /// DSL source code to compile
    pub source: String,
}

/// Response from DSL compilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CompileDslResponse {
    /// Whether the compilation succeeded
    pub success: bool,
    /// Compilation errors (if any)
    pub errors: Vec<CompileErrorResponse>,
    /// Compiled configuration (if successful)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub compiled: Option<serde_json::Value>,
}

/// Compilation error details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CompileErrorResponse {
    /// Error type (e.g., "UndefinedReference", "DuplicateDefinition")
    pub error_type: String,
    /// Error message
    pub message: String,
}

// ============================================================================
// DEPLOY REQUEST/RESPONSE
// ============================================================================

/// Request to deploy a DSL configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeployDslRequest {
    /// DSL source code to deploy
    pub source: String,
    /// Configuration name (defaults to "default")
    #[serde(default = "default_config_name")]
    pub name: String,
    /// Whether to activate the config immediately after saving
    #[serde(default)]
    pub activate: bool,
    /// Optional deployment notes
    pub notes: Option<String>,
}

fn default_config_name() -> String {
    "default".to_string()
}

/// Response from DSL deployment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeployDslResponse {
    /// Configuration ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub config_id: Uuid,
    /// Configuration name
    pub name: String,
    /// Configuration version
    pub version: i32,
    /// Deployment status
    pub status: DslConfigStatus,
    /// Message
    pub message: String,
}

/// DSL configuration status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum DslConfigStatus {
    /// Draft - saved but not deployed
    Draft,
    /// Deployed - active configuration
    Deployed,
    /// Archived - previously deployed, now superseded
    Archived,
}

impl DslConfigStatus {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            DslConfigStatus::Draft => "draft",
            DslConfigStatus::Deployed => "deployed",
            DslConfigStatus::Archived => "archived",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "draft" => Some(DslConfigStatus::Draft),
            "deployed" => Some(DslConfigStatus::Deployed),
            "archived" => Some(DslConfigStatus::Archived),
            _ => None,
        }
    }
}

// ============================================================================
// CONFIG GET/LIST
// ============================================================================

/// Response for a stored DSL configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DslConfigResponse {
    /// Configuration ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub config_id: Uuid,
    /// Tenant ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: Uuid,
    /// Configuration name
    pub name: String,
    /// Configuration version
    pub version: i32,
    /// DSL source code
    pub dsl_source: String,
    /// Parsed AST
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub ast: serde_json::Value,
    /// Compiled configuration (if available)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub compiled: Option<serde_json::Value>,
    /// Configuration status
    pub status: DslConfigStatus,
    /// When deployed (if deployed)
    pub deployed_at: Option<String>,
    /// When created
    pub created_at: String,
    /// When last updated
    pub updated_at: String,
}

/// Request to list DSL configurations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListDslConfigsRequest {
    /// Filter by name
    pub name: Option<String>,
    /// Filter by status
    pub status: Option<DslConfigStatus>,
    /// Maximum number of results
    #[serde(default = "default_limit")]
    pub limit: i32,
    /// Offset for pagination
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    50
}

/// Response for listing DSL configurations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListDslConfigsResponse {
    /// List of configurations
    pub configs: Vec<DslConfigSummary>,
    /// Total count (for pagination)
    pub total: i32,
}

/// Summary of a DSL configuration (for lists).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DslConfigSummary {
    /// Configuration ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub config_id: Uuid,
    /// Configuration name
    pub name: String,
    /// Configuration version
    pub version: i32,
    /// Configuration status
    pub status: DslConfigStatus,
    /// When deployed (if deployed)
    pub deployed_at: Option<String>,
    /// When created
    pub created_at: String,
}
