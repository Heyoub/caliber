//! Unified health check types
//!
//! This module provides unified health check types that can be used across
//! different crates (API, LLM, etc.) for consistent health reporting.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health status for a service or component.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Component is fully operational
    Healthy,
    /// Component is operational but degraded
    Degraded,
    /// Component is not operational
    Unhealthy,
    /// Health status is unknown (e.g., not yet checked)
    Unknown,
}

/// Detailed health check result for a component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthCheck {
    /// Overall health status
    pub status: HealthStatus,
    /// Component name
    pub component: String,
    /// Detailed status message
    pub message: Option<String>,
    /// Response time in milliseconds (if applicable)
    pub response_time_ms: Option<i64>,
    /// Additional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl HealthCheck {
    /// Create a healthy check result.
    pub fn healthy(component: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Healthy,
            component: component.into(),
            message: None,
            response_time_ms: None,
            metadata: None,
        }
    }

    /// Create a degraded check result.
    pub fn degraded(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            component: component.into(),
            message: Some(message.into()),
            response_time_ms: None,
            metadata: None,
        }
    }

    /// Create an unhealthy check result.
    pub fn unhealthy(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            component: component.into(),
            message: Some(message.into()),
            response_time_ms: None,
            metadata: None,
        }
    }

    /// Set the response time.
    pub fn with_response_time(mut self, ms: i64) -> Self {
        self.response_time_ms = Some(ms);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value);
        self
    }
}
