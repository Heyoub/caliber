//! Configuration types

use crate::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Section priorities for context assembly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SectionPriorities {
    pub user: i32,
    pub system: i32,
    pub persona: i32,
    pub artifacts: i32,
    pub notes: i32,
    pub history: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<Vec<Object>>))]
    pub custom: Vec<(String, i32)>,
}

/// Context persistence mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ContextPersistence {
    /// Context is not persisted
    Ephemeral,
    /// Context persists for specified duration (in nanoseconds)
    #[cfg_attr(feature = "openapi", schema(value_type = u64))]
    Ttl(Duration),
    /// Context persists permanently
    Permanent,
}

/// Validation mode for PCP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ValidationMode {
    /// Validate only on mutations
    OnMutation,
    /// Always validate
    Always,
}

/// LLM provider configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ProviderConfig {
    pub provider_type: String,
    pub endpoint: Option<String>,
    pub model: String,
    pub dimensions: Option<i32>,
}

/// Retry configuration for LLM operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RetryConfig {
    pub max_retries: i32,
    /// Initial backoff duration in nanoseconds
    #[cfg_attr(feature = "openapi", schema(value_type = u64))]
    pub initial_backoff: Duration,
    /// Maximum backoff duration in nanoseconds
    #[cfg_attr(feature = "openapi", schema(value_type = u64))]
    pub max_backoff: Duration,
    pub backoff_multiplier: f32,
}

/// Master configuration struct.
/// ALL values are required - no defaults anywhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CaliberConfig {
    // Context assembly (REQUIRED)
    pub token_budget: i32,
    pub section_priorities: SectionPriorities,

    // PCP settings (REQUIRED)
    pub checkpoint_retention: i32,
    /// Stale threshold duration in nanoseconds
    #[cfg_attr(feature = "openapi", schema(value_type = u64))]
    pub stale_threshold: Duration,
    pub contradiction_threshold: f32,

    // Storage (REQUIRED)
    pub context_window_persistence: ContextPersistence,
    pub validation_mode: ValidationMode,

    // LLM (optional, but required if using embeddings)
    pub embedding_provider: Option<ProviderConfig>,
    pub summarization_provider: Option<ProviderConfig>,
    pub llm_retry_config: RetryConfig,

    // Multi-agent (REQUIRED)
    /// Lock timeout duration in nanoseconds
    #[cfg_attr(feature = "openapi", schema(value_type = u64))]
    pub lock_timeout: Duration,
    /// Message retention duration in nanoseconds
    #[cfg_attr(feature = "openapi", schema(value_type = u64))]
    pub message_retention: Duration,
    /// Delegation timeout duration in nanoseconds
    #[cfg_attr(feature = "openapi", schema(value_type = u64))]
    pub delegation_timeout: Duration,
}

impl CaliberConfig {
    /// Validate the configuration.
    /// Returns Ok(()) if valid, Err(CaliberError::Config) if invalid.
    ///
    /// Validates:
    /// - token_budget > 0
    /// - contradiction_threshold in [0.0, 1.0]
    /// - checkpoint_retention >= 0
    /// - All duration values are positive
    pub fn validate(&self) -> CaliberResult<()> {
        // Validate token_budget
        if self.token_budget <= 0 {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "token_budget".to_string(),
                value: self.token_budget.to_string(),
                reason: "token_budget must be greater than 0".to_string(),
            }));
        }

        // Validate contradiction_threshold
        if self.contradiction_threshold < 0.0 || self.contradiction_threshold > 1.0 {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "contradiction_threshold".to_string(),
                value: self.contradiction_threshold.to_string(),
                reason: "contradiction_threshold must be between 0.0 and 1.0".to_string(),
            }));
        }

        // Validate checkpoint_retention
        if self.checkpoint_retention < 0 {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "checkpoint_retention".to_string(),
                value: self.checkpoint_retention.to_string(),
                reason: "checkpoint_retention must be non-negative".to_string(),
            }));
        }

        // Validate stale_threshold is positive
        if self.stale_threshold.is_zero() {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "stale_threshold".to_string(),
                value: format!("{:?}", self.stale_threshold),
                reason: "stale_threshold must be positive".to_string(),
            }));
        }

        // Validate lock_timeout is positive
        if self.lock_timeout.is_zero() {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "lock_timeout".to_string(),
                value: format!("{:?}", self.lock_timeout),
                reason: "lock_timeout must be positive".to_string(),
            }));
        }

        // Validate message_retention is positive
        if self.message_retention.is_zero() {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "message_retention".to_string(),
                value: format!("{:?}", self.message_retention),
                reason: "message_retention must be positive".to_string(),
            }));
        }

        // Validate delegation_timeout is positive
        if self.delegation_timeout.is_zero() {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "delegation_timeout".to_string(),
                value: format!("{:?}", self.delegation_timeout),
                reason: "delegation_timeout must be positive".to_string(),
            }));
        }

        // Validate retry config
        if self.llm_retry_config.max_retries < 0 {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "llm_retry_config.max_retries".to_string(),
                value: self.llm_retry_config.max_retries.to_string(),
                reason: "max_retries must be non-negative".to_string(),
            }));
        }

        if self.llm_retry_config.backoff_multiplier <= 0.0 {
            return Err(CaliberError::Config(ConfigError::InvalidValue {
                field: "llm_retry_config.backoff_multiplier".to_string(),
                value: self.llm_retry_config.backoff_multiplier.to_string(),
                reason: "backoff_multiplier must be positive".to_string(),
            }));
        }

        Ok(())
    }
}

// ============================================================================
