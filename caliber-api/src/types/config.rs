//! Configuration-related API types

use serde::{Deserialize, Serialize};

/// Request to update configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateConfigRequest {
    /// Configuration as JSON
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub config: serde_json::Value,
}

/// Request to validate configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ValidateConfigRequest {
    /// Configuration as JSON to validate
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub config: serde_json::Value,
}

/// Configuration response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ConfigResponse {
    /// Configuration as JSON
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub config: serde_json::Value,
    /// Validation status
    pub valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<String>,
}
