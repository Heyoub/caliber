//! CALIBER Core - Entity Types
//!
//! Pure data structures with no behavior. All other crates depend on this.
//! This crate contains ONLY data types - no business logic.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// IDENTITY TYPES (Task 2.1)
// ============================================================================

/// Entity identifier using UUIDv7 for timestamp-sortable IDs.
/// UUIDv7 embeds a Unix timestamp, making IDs naturally sortable by creation time.
pub type EntityId = Uuid;

/// Timestamp type using UTC timezone.
pub type Timestamp = DateTime<Utc>;

/// Duration in milliseconds for TTL and timeout values.
pub type DurationMs = i64;

/// SHA-256 content hash for deduplication and integrity verification.
pub type ContentHash = [u8; 32];

/// Raw binary content for BYTEA storage.
pub type RawContent = Vec<u8>;

/// Generate a new UUIDv7 EntityId (timestamp-sortable).
pub fn new_entity_id() -> EntityId {
    Uuid::now_v7()
}

/// Compute SHA-256 hash of content.
pub fn compute_content_hash(content: &[u8]) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

// ============================================================================
// ENUMS (Task 2.2)
// ============================================================================

/// Time-to-live configuration for memory entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TTL {
    /// Never expires
    Persistent,
    /// Expires when session ends
    Session,
    /// Expires when scope closes
    Scope,
    /// Expires after specified duration in milliseconds
    Duration(DurationMs),
    // Semantic aliases for common TTL patterns
    /// Ephemeral - expires when scope closes (alias for Scope)
    Ephemeral,
    /// Short-term retention (~1 hour)
    ShortTerm,
    /// Medium-term retention (~24 hours)
    MediumTerm,
    /// Long-term retention (~7 days)
    LongTerm,
    /// Permanent - never expires (alias for Persistent)
    Permanent,
}


/// Entity type discriminator for polymorphic references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Trajectory,
    Scope,
    Artifact,
    Note,
    Turn,
    Lock,
    Message,
    Agent,
    Delegation,
    Handoff,
    Conflict,
}

/// Memory category for hierarchical memory organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryCategory {
    /// Session/scope-bound, dies with scope
    Ephemeral,
    /// Bounded retention, active scope
    Working,
    /// Configurable retention, artifacts
    Episodic,
    /// Long-lived, notes
    Semantic,
    /// Persistent, procedures
    Procedural,
    /// Persistent, trajectories
    Meta,
}

/// Status of a trajectory (task container).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrajectoryStatus {
    Active,
    Completed,
    Failed,
    Suspended,
}

/// Outcome status for completed trajectories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutcomeStatus {
    Success,
    Partial,
    Failure,
}

/// Role of a turn in conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TurnRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Type of artifact produced during a trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactType {
    // Core artifact types
    ErrorLog,
    CodePatch,
    DesignDecision,
    UserPreference,
    Fact,
    Constraint,
    ToolResult,
    IntermediateOutput,
    Custom,
    // Extended artifact types for full-featured system
    /// Source code artifacts
    Code,
    /// Documentation, specifications
    Document,
    /// Structured data outputs
    Data,
    /// Configuration files
    Config,
    /// General logs (non-error)
    Log,
    /// Summaries, abstracts
    Summary,
    /// Decision records
    Decision,
    /// Plans, roadmaps
    Plan,
}

/// Method used to extract an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtractionMethod {
    Explicit,
    Inferred,
    UserProvided,
}

/// Type of note (cross-trajectory knowledge).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoteType {
    // Core note types
    Convention,
    Strategy,
    Gotcha,
    Fact,
    Preference,
    Relationship,
    Procedure,
    Meta,
    // Extended note types for full-featured system
    /// Discovered insights
    Insight,
    /// Corrections to previous knowledge
    Correction,
    /// Summary notes
    Summary,
}

// ============================================================================
// EMBEDDING VECTOR (Task 2.3)
// ============================================================================

/// Embedding vector with dynamic dimensions.
/// Supports any embedding model dimension (e.g., 384, 768, 1536, 3072).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingVector {
    /// The embedding data as a vector of f32 values.
    pub data: Vec<f32>,
    /// Identifier of the model that produced this embedding.
    pub model_id: String,
    /// Number of dimensions (must match data.len()).
    pub dimensions: i32,
}

impl EmbeddingVector {
    /// Create a new embedding vector.
    ///
    /// # Arguments
    /// * `data` - The embedding values
    /// * `model_id` - Identifier of the model that produced this embedding
    ///
    /// # Returns
    /// A new EmbeddingVector with dimensions set from data length.
    pub fn new(data: Vec<f32>, model_id: String) -> Self {
        let dimensions = data.len() as i32;
        Self {
            data,
            model_id,
            dimensions,
        }
    }

    /// Compute cosine similarity between two embedding vectors.
    ///
    /// # Arguments
    /// * `other` - The other embedding vector to compare with
    ///
    /// # Returns
    /// * `Ok(f32)` - Cosine similarity in range [-1.0, 1.0]
    /// * `Err(CaliberError)` - If dimensions don't match
    pub fn cosine_similarity(&self, other: &EmbeddingVector) -> CaliberResult<f32> {
        if self.dimensions != other.dimensions {
            return Err(CaliberError::Vector(VectorError::DimensionMismatch {
                expected: self.dimensions,
                got: other.dimensions,
            }));
        }

        let mut dot_product = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;

        for (a, b) in self.data.iter().zip(other.data.iter()) {
            dot_product += a * b;
            norm_a += a * a;
            norm_b += b * b;
        }

        let norm_a = norm_a.sqrt();
        let norm_b = norm_b.sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        Ok(dot_product / (norm_a * norm_b))
    }

    /// Check if this vector has valid dimensions.
    pub fn is_valid(&self) -> bool {
        self.dimensions > 0 && self.data.len() == self.dimensions as usize
    }
}


// ============================================================================
// CORE ENTITY STRUCTS (Task 2.4)
// ============================================================================

/// Reference to an entity by type and ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityRef {
    pub entity_type: EntityType,
    pub id: EntityId,
}

/// Trajectory - top-level task container.
/// A trajectory represents a complete task or goal being pursued.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trajectory {
    pub trajectory_id: EntityId,
    pub name: String,
    pub description: Option<String>,
    pub status: TrajectoryStatus,
    pub parent_trajectory_id: Option<EntityId>,
    pub root_trajectory_id: Option<EntityId>,
    pub agent_id: Option<EntityId>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub completed_at: Option<Timestamp>,
    pub outcome: Option<TrajectoryOutcome>,
    pub metadata: Option<serde_json::Value>,
}

/// Outcome of a completed trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryOutcome {
    pub status: OutcomeStatus,
    pub summary: String,
    pub produced_artifacts: Vec<EntityId>,
    pub produced_notes: Vec<EntityId>,
    pub error: Option<String>,
}

/// Scope - partitioned context window within a trajectory.
/// Scopes provide isolation and checkpointing boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    pub scope_id: EntityId,
    pub trajectory_id: EntityId,
    pub parent_scope_id: Option<EntityId>,
    pub name: String,
    pub purpose: Option<String>,
    pub is_active: bool,
    pub created_at: Timestamp,
    pub closed_at: Option<Timestamp>,
    pub checkpoint: Option<Checkpoint>,
    pub token_budget: i32,
    pub tokens_used: i32,
    pub metadata: Option<serde_json::Value>,
}

/// Checkpoint for scope recovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub context_state: RawContent,
    pub recoverable: bool,
}

/// Artifact - typed output preserved across scopes.
/// Artifacts survive scope closure and can be referenced later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: EntityId,
    pub trajectory_id: EntityId,
    pub scope_id: EntityId,
    pub artifact_type: ArtifactType,
    pub name: String,
    pub content: String,
    pub content_hash: ContentHash,
    pub embedding: Option<EmbeddingVector>,
    pub provenance: Provenance,
    pub ttl: TTL,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub superseded_by: Option<EntityId>,
    pub metadata: Option<serde_json::Value>,
}

/// Provenance information for an artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub source_turn: i32,
    pub extraction_method: ExtractionMethod,
    pub confidence: Option<f32>,
}

/// Note - long-term cross-trajectory knowledge.
/// Notes persist beyond individual trajectories.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub note_id: EntityId,
    pub note_type: NoteType,
    pub title: String,
    pub content: String,
    pub content_hash: ContentHash,
    pub embedding: Option<EmbeddingVector>,
    pub source_trajectory_ids: Vec<EntityId>,
    pub source_artifact_ids: Vec<EntityId>,
    pub ttl: TTL,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub accessed_at: Timestamp,
    pub access_count: i32,
    pub superseded_by: Option<EntityId>,
    pub metadata: Option<serde_json::Value>,
}

/// Turn - ephemeral conversation buffer entry.
/// Turns die with their scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    pub turn_id: EntityId,
    pub scope_id: EntityId,
    pub sequence: i32,
    pub role: TurnRole,
    pub content: String,
    pub token_count: i32,
    pub created_at: Timestamp,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_results: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}


// ============================================================================
// ERROR TYPES (Task 2.5)
// ============================================================================

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


// ============================================================================
// CONFIGURATION (Task 2.6)
// ============================================================================

/// Section priorities for context assembly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionPriorities {
    pub user: i32,
    pub system: i32,
    pub persona: i32,
    pub artifacts: i32,
    pub notes: i32,
    pub history: i32,
    pub custom: Vec<(String, i32)>,
}

/// Context persistence mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContextPersistence {
    /// Context is not persisted
    Ephemeral,
    /// Context persists for specified duration
    Ttl(Duration),
    /// Context persists permanently
    Permanent,
}

/// Validation mode for PCP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationMode {
    /// Validate only on mutations
    OnMutation,
    /// Always validate
    Always,
}

/// LLM provider configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub endpoint: Option<String>,
    pub model: String,
    pub dimensions: Option<i32>,
}

/// Retry configuration for LLM operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: i32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f32,
}

/// Master configuration struct.
/// ALL values are required - no defaults anywhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaliberConfig {
    // Context assembly (REQUIRED)
    pub token_budget: i32,
    pub section_priorities: SectionPriorities,

    // PCP settings (REQUIRED)
    pub checkpoint_retention: i32,
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
    pub lock_timeout: Duration,
    pub message_retention: Duration,
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
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_entity_id_is_v7() {
        let id = new_entity_id();
        assert_eq!(id.get_version_num(), 7);
    }

    #[test]
    fn test_entity_ids_are_sortable() {
        let id1 = new_entity_id();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = new_entity_id();
        // UUIDv7 should be lexicographically sortable by time
        assert!(id1.to_string() < id2.to_string());
    }

    #[test]
    fn test_content_hash() {
        let content = b"hello world";
        let hash = compute_content_hash(content);
        assert_eq!(hash.len(), 32);
        // Same content should produce same hash
        let hash2 = compute_content_hash(content);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_embedding_vector_cosine_similarity() {
        let v1 = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "test".to_string());
        let v2 = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "test".to_string());
        let similarity = v1.cosine_similarity(&v2).unwrap();
        assert!((similarity - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_embedding_vector_orthogonal() {
        let v1 = EmbeddingVector::new(vec![1.0, 0.0], "test".to_string());
        let v2 = EmbeddingVector::new(vec![0.0, 1.0], "test".to_string());
        let similarity = v1.cosine_similarity(&v2).unwrap();
        assert!((similarity - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_embedding_vector_dimension_mismatch() {
        let v1 = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "test".to_string());
        let v2 = EmbeddingVector::new(vec![1.0, 0.0], "test".to_string());
        let result = v1.cosine_similarity(&v2);
        assert!(matches!(
            result,
            Err(CaliberError::Vector(VectorError::DimensionMismatch { .. }))
        ));
    }

    #[test]
    fn test_config_validation_valid() {
        let config = CaliberConfig {
            token_budget: 8000,
            section_priorities: SectionPriorities {
                user: 100,
                system: 90,
                persona: 85,
                artifacts: 80,
                notes: 70,
                history: 60,
                custom: vec![],
            },
            checkpoint_retention: 10,
            stale_threshold: Duration::from_secs(3600),
            contradiction_threshold: 0.8,
            context_window_persistence: ContextPersistence::Ephemeral,
            validation_mode: ValidationMode::OnMutation,
            embedding_provider: None,
            summarization_provider: None,
            llm_retry_config: RetryConfig {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(10),
                backoff_multiplier: 2.0,
            },
            lock_timeout: Duration::from_secs(30),
            message_retention: Duration::from_secs(86400),
            delegation_timeout: Duration::from_secs(300),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_token_budget() {
        let config = CaliberConfig {
            token_budget: 0,
            section_priorities: SectionPriorities {
                user: 100,
                system: 90,
                persona: 85,
                artifacts: 80,
                notes: 70,
                history: 60,
                custom: vec![],
            },
            checkpoint_retention: 10,
            stale_threshold: Duration::from_secs(3600),
            contradiction_threshold: 0.8,
            context_window_persistence: ContextPersistence::Ephemeral,
            validation_mode: ValidationMode::OnMutation,
            embedding_provider: None,
            summarization_provider: None,
            llm_retry_config: RetryConfig {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(10),
                backoff_multiplier: 2.0,
            },
            lock_timeout: Duration::from_secs(30),
            message_retention: Duration::from_secs(86400),
            delegation_timeout: Duration::from_secs(300),
        };
        let result = config.validate();
        assert!(matches!(
            result,
            Err(CaliberError::Config(ConfigError::InvalidValue { field, .. })) if field == "token_budget"
        ));
    }

    #[test]
    fn test_config_validation_invalid_contradiction_threshold() {
        let config = CaliberConfig {
            token_budget: 8000,
            section_priorities: SectionPriorities {
                user: 100,
                system: 90,
                persona: 85,
                artifacts: 80,
                notes: 70,
                history: 60,
                custom: vec![],
            },
            checkpoint_retention: 10,
            stale_threshold: Duration::from_secs(3600),
            contradiction_threshold: 1.5, // Invalid: > 1.0
            context_window_persistence: ContextPersistence::Ephemeral,
            validation_mode: ValidationMode::OnMutation,
            embedding_provider: None,
            summarization_provider: None,
            llm_retry_config: RetryConfig {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(10),
                backoff_multiplier: 2.0,
            },
            lock_timeout: Duration::from_secs(30),
            message_retention: Duration::from_secs(86400),
            delegation_timeout: Duration::from_secs(300),
        };
        let result = config.validate();
        assert!(matches!(
            result,
            Err(CaliberError::Config(ConfigError::InvalidValue { field, .. })) if field == "contradiction_threshold"
        ));
    }
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Property 1: Config validation rejects invalid values
    // Feature: caliber-core-implementation, Property 1: Config validation rejects invalid values
    // Validates: Requirements 3.4, 3.5
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 1: For any token_budget <= 0, validate() SHALL return ConfigError::InvalidValue
        #[test]
        fn prop_config_rejects_invalid_token_budget(token_budget in i32::MIN..=0) {
            let config = CaliberConfig {
                token_budget,
                section_priorities: SectionPriorities {
                    user: 100,
                    system: 90,
                    persona: 85,
                    artifacts: 80,
                    notes: 70,
                    history: 60,
                    custom: vec![],
                },
                checkpoint_retention: 10,
                stale_threshold: Duration::from_secs(3600),
                contradiction_threshold: 0.5,
                context_window_persistence: ContextPersistence::Ephemeral,
                validation_mode: ValidationMode::OnMutation,
                embedding_provider: None,
                summarization_provider: None,
                llm_retry_config: RetryConfig {
                    max_retries: 3,
                    initial_backoff: Duration::from_millis(100),
                    max_backoff: Duration::from_secs(10),
                    backoff_multiplier: 2.0,
                },
                lock_timeout: Duration::from_secs(30),
                message_retention: Duration::from_secs(86400),
                delegation_timeout: Duration::from_secs(300),
            };

            let result = config.validate();
            prop_assert!(result.is_err());
            if let Err(CaliberError::Config(ConfigError::InvalidValue { field, .. })) = result {
                prop_assert_eq!(field, "token_budget");
            } else {
                prop_assert!(false, "Expected ConfigError::InvalidValue");
            }
        }

        /// Property 1: For any contradiction_threshold outside [0.0, 1.0], validate() SHALL return ConfigError::InvalidValue
        #[test]
        fn prop_config_rejects_invalid_contradiction_threshold_high(threshold in 1.001f32..100.0f32) {
            let config = CaliberConfig {
                token_budget: 8000,
                section_priorities: SectionPriorities {
                    user: 100,
                    system: 90,
                    persona: 85,
                    artifacts: 80,
                    notes: 70,
                    history: 60,
                    custom: vec![],
                },
                checkpoint_retention: 10,
                stale_threshold: Duration::from_secs(3600),
                contradiction_threshold: threshold,
                context_window_persistence: ContextPersistence::Ephemeral,
                validation_mode: ValidationMode::OnMutation,
                embedding_provider: None,
                summarization_provider: None,
                llm_retry_config: RetryConfig {
                    max_retries: 3,
                    initial_backoff: Duration::from_millis(100),
                    max_backoff: Duration::from_secs(10),
                    backoff_multiplier: 2.0,
                },
                lock_timeout: Duration::from_secs(30),
                message_retention: Duration::from_secs(86400),
                delegation_timeout: Duration::from_secs(300),
            };

            let result = config.validate();
            prop_assert!(result.is_err());
            if let Err(CaliberError::Config(ConfigError::InvalidValue { field, .. })) = result {
                prop_assert_eq!(field, "contradiction_threshold");
            } else {
                prop_assert!(false, "Expected ConfigError::InvalidValue for high threshold");
            }
        }

        /// Property 1: For any contradiction_threshold < 0.0, validate() SHALL return ConfigError::InvalidValue
        #[test]
        fn prop_config_rejects_invalid_contradiction_threshold_low(threshold in -100.0f32..-0.001f32) {
            let config = CaliberConfig {
                token_budget: 8000,
                section_priorities: SectionPriorities {
                    user: 100,
                    system: 90,
                    persona: 85,
                    artifacts: 80,
                    notes: 70,
                    history: 60,
                    custom: vec![],
                },
                checkpoint_retention: 10,
                stale_threshold: Duration::from_secs(3600),
                contradiction_threshold: threshold,
                context_window_persistence: ContextPersistence::Ephemeral,
                validation_mode: ValidationMode::OnMutation,
                embedding_provider: None,
                summarization_provider: None,
                llm_retry_config: RetryConfig {
                    max_retries: 3,
                    initial_backoff: Duration::from_millis(100),
                    max_backoff: Duration::from_secs(10),
                    backoff_multiplier: 2.0,
                },
                lock_timeout: Duration::from_secs(30),
                message_retention: Duration::from_secs(86400),
                delegation_timeout: Duration::from_secs(300),
            };

            let result = config.validate();
            prop_assert!(result.is_err());
            if let Err(CaliberError::Config(ConfigError::InvalidValue { field, .. })) = result {
                prop_assert_eq!(field, "contradiction_threshold");
            } else {
                prop_assert!(false, "Expected ConfigError::InvalidValue for low threshold");
            }
        }

        /// Property 1: For any valid config values, validate() SHALL return Ok(())
        #[test]
        fn prop_config_accepts_valid_values(
            token_budget in 1..100000i32,
            contradiction_threshold in 0.0f32..=1.0f32,
            checkpoint_retention in 0..1000i32,
        ) {
            let config = CaliberConfig {
                token_budget,
                section_priorities: SectionPriorities {
                    user: 100,
                    system: 90,
                    persona: 85,
                    artifacts: 80,
                    notes: 70,
                    history: 60,
                    custom: vec![],
                },
                checkpoint_retention,
                stale_threshold: Duration::from_secs(3600),
                contradiction_threshold,
                context_window_persistence: ContextPersistence::Ephemeral,
                validation_mode: ValidationMode::OnMutation,
                embedding_provider: None,
                summarization_provider: None,
                llm_retry_config: RetryConfig {
                    max_retries: 3,
                    initial_backoff: Duration::from_millis(100),
                    max_backoff: Duration::from_secs(10),
                    backoff_multiplier: 2.0,
                },
                lock_timeout: Duration::from_secs(30),
                message_retention: Duration::from_secs(86400),
                delegation_timeout: Duration::from_secs(300),
            };

            prop_assert!(config.validate().is_ok());
        }
    }

    // ========================================================================
    // Property 5: EmbeddingVector dimension mismatch detection
    // Feature: caliber-core-implementation, Property 5: EmbeddingVector dimension mismatch detection
    // Validates: Requirements 6.6
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 5: For any two EmbeddingVectors with different dimensions,
        /// cosine_similarity() SHALL return Err(VectorError::DimensionMismatch)
        #[test]
        fn prop_embedding_dimension_mismatch_detected(
            dim1 in 1usize..100,
            dim2 in 1usize..100,
        ) {
            // Only test when dimensions are actually different
            prop_assume!(dim1 != dim2);

            let v1 = EmbeddingVector::new(vec![1.0; dim1], "model_a".to_string());
            let v2 = EmbeddingVector::new(vec![1.0; dim2], "model_b".to_string());

            let result = v1.cosine_similarity(&v2);

            prop_assert!(result.is_err());
            if let Err(CaliberError::Vector(VectorError::DimensionMismatch { expected, got })) = result {
                prop_assert_eq!(expected, dim1 as i32);
                prop_assert_eq!(got, dim2 as i32);
            } else {
                prop_assert!(false, "Expected VectorError::DimensionMismatch");
            }
        }

        /// Property 5: For any two EmbeddingVectors with same dimensions,
        /// cosine_similarity() SHALL return Ok(f32)
        #[test]
        fn prop_embedding_same_dimension_succeeds(
            dim in 1usize..100,
            values1 in prop::collection::vec(-1.0f32..1.0f32, 1..100),
            values2 in prop::collection::vec(-1.0f32..1.0f32, 1..100),
        ) {
            // Ensure both vectors have the same dimension
            let v1_data: Vec<f32> = values1.into_iter().take(dim).chain(std::iter::repeat(0.0)).take(dim).collect();
            let v2_data: Vec<f32> = values2.into_iter().take(dim).chain(std::iter::repeat(0.0)).take(dim).collect();

            let v1 = EmbeddingVector::new(v1_data, "model".to_string());
            let v2 = EmbeddingVector::new(v2_data, "model".to_string());

            let result = v1.cosine_similarity(&v2);
            prop_assert!(result.is_ok());

            // Cosine similarity should be in [-1, 1]
            if let Ok(sim) = result {
                prop_assert!(sim >= -1.0 && sim <= 1.0);
            }
        }
    }

    // ========================================================================
    // Property 7: EntityId uses UUIDv7 (timestamp-sortable)
    // Feature: caliber-core-implementation, Property 7: EntityId uses UUIDv7 (timestamp-sortable)
    // Validates: Requirements 2.3
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 7: For any two EntityIds generated in sequence,
        /// the first SHALL sort before the second lexicographically
        #[test]
        fn prop_entity_ids_are_timestamp_sortable(_iteration in 0..100u32) {
            let id1 = new_entity_id();
            // Small delay to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_millis(1));
            let id2 = new_entity_id();

            // UUIDv7 should be lexicographically sortable by time
            prop_assert!(id1.to_string() < id2.to_string(),
                "id1 ({}) should sort before id2 ({})", id1, id2);
        }

        /// Property 7: All generated EntityIds SHALL be UUIDv7
        #[test]
        fn prop_entity_ids_are_v7(_iteration in 0..100u32) {
            let id = new_entity_id();
            prop_assert_eq!(id.get_version_num(), 7,
                "EntityId {} should be version 7, got version {}", id, id.get_version_num());
        }
    }
}
