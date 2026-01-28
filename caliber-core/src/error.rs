//! Error types for CALIBER operations

use crate::EntityType;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

/// Storage layer errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("Entity not found: {entity_type:?} with id {id}")]
    NotFound { entity_type: EntityType, id: Uuid },

    #[error("Insert failed for {entity_type:?}: {reason}")]
    InsertFailed { entity_type: EntityType, reason: String },

    #[error("Update failed for {entity_type:?} with id {id}: {reason}")]
    UpdateFailed {
        entity_type: EntityType,
        id: Uuid,
        reason: String,
    },

    #[error("Transaction failed: {reason}")]
    TransactionFailed { reason: String },

    #[error("Index error on {index_name}: {reason}")]
    IndexError { index_name: String, reason: String },

    #[error("Storage lock poisoned")]
    LockPoisoned,

    #[error("SPI error: {reason}")]
    SpiError { reason: String },
}

/// LLM provider errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LlmError {
    #[error("No LLM provider configured")]
    ProviderNotConfigured,

    #[error("Request to {provider} failed with status {status}: {message}")]
    RequestFailed {
        provider: String,
        status: i32,
        message: String,
    },

    #[error("Rate limited by {provider}, retry after {retry_after_ms}ms")]
    RateLimited {
        provider: String,
        retry_after_ms: i64,
    },

    #[error("Invalid response from {provider}: {reason}")]
    InvalidResponse { provider: String, reason: String },

    #[error("Embedding failed: {reason}")]
    EmbeddingFailed { reason: String },

    #[error("Summarization failed: {reason}")]
    SummarizationFailed { reason: String },
}

/// Validation errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Required field missing: {field}")]
    RequiredFieldMissing { field: String },

    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("Constraint violation on {constraint}: {reason}")]
    ConstraintViolation { constraint: String, reason: String },

    #[error("Circular reference detected in {entity_type:?}: {ids:?}")]
    CircularReference {
        entity_type: EntityType,
        ids: Vec<Uuid>,
    },

    #[error("Stale data for {entity_type:?} with id {id}, age: {age:?}")]
    StaleData {
        entity_type: EntityType,
        id: Uuid,
        age: Duration,
    },

    #[error("Contradiction detected between artifacts {artifact_a} and {artifact_b}")]
    Contradiction {
        artifact_a: Uuid,
        artifact_b: Uuid,
    },
}

/// Configuration errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("Missing required configuration field: {field}")]
    MissingRequired { field: String },

    #[error("Invalid value for {field}: {value} - {reason}")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },

    #[error("Incompatible options: {option_a} and {option_b}")]
    IncompatibleOptions { option_a: String, option_b: String },

    #[error("Provider not supported: {provider}")]
    ProviderNotSupported { provider: String },
}

/// Vector operation errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum VectorError {
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: i32, got: i32 },

    #[error("Invalid vector: {reason}")]
    InvalidVector { reason: String },

    #[error("Model mismatch: expected {expected}, got {got}")]
    ModelMismatch { expected: String, got: String },
}

/// Agent coordination errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AgentError {
    #[error("Agent not registered: {agent_id}")]
    NotRegistered { agent_id: Uuid },

    #[error("Lock acquisition failed for {resource}: held by {holder}")]
    LockAcquisitionFailed { resource: String, holder: Uuid },

    #[error("Lock expired: {lock_id}")]
    LockExpired { lock_id: Uuid },

    #[error("Message delivery failed for {message_id}: {reason}")]
    MessageDeliveryFailed { message_id: Uuid, reason: String },

    #[error("Delegation failed: {reason}")]
    DelegationFailed { reason: String },

    #[error("Handoff failed: {reason}")]
    HandoffFailed { reason: String },

    #[error("Permission denied for agent {agent_id}: {action} on {resource}")]
    PermissionDenied {
        agent_id: Uuid,
        action: String,
        resource: String,
    },
}

/// Master error type for all CALIBER errors.
#[derive(Debug, Clone, Error)]
pub enum CaliberError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Vector error: {0}")]
    Vector(#[from] VectorError),

    #[error("Agent error: {0}")]
    Agent(#[from] AgentError),
}

/// Result type alias for CALIBER operations.
pub type CaliberResult<T> = Result<T, CaliberError>;

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_error_display_not_found() {
        let err = StorageError::NotFound {
            entity_type: EntityType::Trajectory,
            id: Uuid::nil(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Entity not found"));
        assert!(msg.contains("Trajectory"));
        assert!(msg.contains("00000000-0000-0000-0000-000000000000"));
    }

    #[test]
    fn test_validation_error_display_stale_data() {
        let err = ValidationError::StaleData {
            entity_type: EntityType::Artifact,
            id: Uuid::nil(),
            age: Duration::from_secs(42),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Stale data"));
        assert!(msg.contains("Artifact"));
    }

    #[test]
    fn test_llm_error_display_rate_limited() {
        let err = LlmError::RateLimited {
            provider: "openai".to_string(),
            retry_after_ms: 1500,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Rate limited"));
        assert!(msg.contains("openai"));
        assert!(msg.contains("1500"));
    }

    #[test]
    fn test_config_error_display_invalid_value() {
        let err = ConfigError::InvalidValue {
            field: "api_base_url".to_string(),
            value: "bad".to_string(),
            reason: "must be url".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("api_base_url"));
        assert!(msg.contains("bad"));
        assert!(msg.contains("must be url"));
    }

    #[test]
    fn test_vector_error_display_dimension_mismatch() {
        let err = VectorError::DimensionMismatch {
            expected: 1536,
            got: 768,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Dimension mismatch"));
        assert!(msg.contains("1536"));
        assert!(msg.contains("768"));
    }

    #[test]
    fn test_agent_error_display_permission_denied() {
        let err = AgentError::PermissionDenied {
            agent_id: Uuid::nil(),
            action: "read".to_string(),
            resource: "artifact".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Permission denied"));
        assert!(msg.contains("read"));
        assert!(msg.contains("artifact"));
    }

    #[test]
    fn test_caliber_error_from_variants() {
        let storage = CaliberError::from(StorageError::LockPoisoned);
        assert!(matches!(storage, CaliberError::Storage(_)));

        let llm = CaliberError::from(LlmError::ProviderNotConfigured);
        assert!(matches!(llm, CaliberError::Llm(_)));

        let validation = CaliberError::from(ValidationError::RequiredFieldMissing {
            field: "name".to_string(),
        });
        assert!(matches!(validation, CaliberError::Validation(_)));

        let config = CaliberError::from(ConfigError::ProviderNotSupported {
            provider: "test".to_string(),
        });
        assert!(matches!(config, CaliberError::Config(_)));

        let vector = CaliberError::from(VectorError::InvalidVector {
            reason: "empty".to_string(),
        });
        assert!(matches!(vector, CaliberError::Vector(_)));

        let agent = CaliberError::from(AgentError::DelegationFailed {
            reason: "timeout".to_string(),
        });
        assert!(matches!(agent, CaliberError::Agent(_)));
    }

    #[test]
    fn test_storage_error_display_lock_poisoned() {
        let err = StorageError::LockPoisoned;
        let msg = format!("{}", err);
        assert!(msg.contains("Lock poisoned"));
    }

    #[test]
    fn test_validation_error_display_invalid_field_value() {
        let err = ValidationError::InvalidFieldValue {
            field: "priority".to_string(),
            value: "high".to_string(),
            reason: "must be numeric".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("priority"));
        assert!(msg.contains("high"));
        assert!(msg.contains("must be numeric"));
    }

    #[test]
    fn test_agent_error_display_delegation_failed() {
        let err = AgentError::DelegationFailed {
            reason: "timeout".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Delegation failed"));
        assert!(msg.contains("timeout"));
    }
}
