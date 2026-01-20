//! Error types for CALIBER operations

use crate::*;
use std::time::Duration;
use thiserror::Error;

/// Storage layer errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("Entity not found: {entity_type:?} with id {id}")]
    NotFound { entity_type: EntityType, id: EntityId },

    #[error("Insert failed for {entity_type:?}: {reason}")]
    InsertFailed { entity_type: EntityType, reason: String },

    #[error("Update failed for {entity_type:?} with id {id}: {reason}")]
    UpdateFailed {
        entity_type: EntityType,
        id: EntityId,
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
        ids: Vec<EntityId>,
    },

    #[error("Stale data for {entity_type:?} with id {id}, age: {age:?}")]
    StaleData {
        entity_type: EntityType,
        id: EntityId,
        age: Duration,
    },

    #[error("Contradiction detected between artifacts {artifact_a} and {artifact_b}")]
    Contradiction {
        artifact_a: EntityId,
        artifact_b: EntityId,
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
    NotRegistered { agent_id: EntityId },

    #[error("Lock acquisition failed for {resource}: held by {holder}")]
    LockAcquisitionFailed { resource: String, holder: EntityId },

    #[error("Lock expired: {lock_id}")]
    LockExpired { lock_id: EntityId },

    #[error("Message delivery failed for {message_id}: {reason}")]
    MessageDeliveryFailed { message_id: EntityId, reason: String },

    #[error("Delegation failed: {reason}")]
    DelegationFailed { reason: String },

    #[error("Handoff failed: {reason}")]
    HandoffFailed { reason: String },

    #[error("Permission denied for agent {agent_id}: {action} on {resource}")]
    PermissionDenied {
        agent_id: EntityId,
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

