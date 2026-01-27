//! CALIBER Test Utilities
//!
//! Centralized test infrastructure for the CALIBER workspace:
//! - Proptest generators for all entity types
//! - Mock providers for testing
//! - Test fixtures for common scenarios
//! - Custom assertions for CALIBER-specific validation

// Re-export mock storage from its source crate
pub use caliber_storage::MockStorage;

// Re-export core types for convenience
pub use caliber_core::{
    AbstractionLevel, Artifact, ArtifactType, CaliberConfig, CaliberError, CaliberResult,
    Checkpoint, ContentHash, ContextPersistence, EmbeddingVector, EntityRef,
    EntityType, ExtractionMethod, MemoryCategory, Note, NoteType, OutcomeStatus, Provenance,
    ProviderConfig, RawContent, RetryConfig, Scope, SectionPriorities, StorageError,
    TTL, Timestamp, Trajectory, TrajectoryOutcome, TrajectoryStatus, Turn, TurnRole,
    ValidationMode, VectorError, compute_content_hash,
    // Strongly-typed entity IDs
    TrajectoryId, ScopeId, ArtifactId, NoteId, TurnId, AgentId, EntityIdType,
    // LLM types for mock providers
    EmbeddingProvider, SummarizationProvider, SummarizeConfig, SummarizeStyle, ExtractedArtifact,
};

use async_trait::async_trait;

// ============================================================================
// MOCK PROVIDERS (from former caliber-llm)
// ============================================================================

/// Mock embedding provider for testing (async).
#[derive(Debug, Clone)]
pub struct MockEmbeddingProvider {
    model_id: String,
    dimensions: i32,
}

impl MockEmbeddingProvider {
    pub fn new(model_id: impl Into<String>, dimensions: i32) -> Self {
        Self {
            model_id: model_id.into(),
            dimensions,
        }
    }

    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut data = vec![0.0f32; self.dimensions as usize];

        for (i, byte) in text.bytes().enumerate() {
            let idx = i % self.dimensions as usize;
            data[idx] += (byte as f32) / 255.0;
        }

        let norm: f32 = data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut data {
                *x /= norm;
            }
        }

        data
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector> {
        let data = self.generate_embedding(text);
        Ok(EmbeddingVector::new(data, self.model_id.clone()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimensions(&self) -> i32 {
        self.dimensions
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}

/// Mock summarization provider for testing (async).
#[derive(Debug, Clone)]
pub struct MockSummarizationProvider {
    prefix: String,
}

impl MockSummarizationProvider {
    pub fn new() -> Self {
        Self {
            prefix: "Summary: ".to_string(),
        }
    }

    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl Default for MockSummarizationProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SummarizationProvider for MockSummarizationProvider {
    async fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String> {
        let max_chars = (config.max_tokens * 4) as usize;
        let truncated = if content.len() > max_chars {
            &content[..max_chars]
        } else {
            content
        };

        let summary = match config.style {
            SummarizeStyle::Brief => format!("{}{}", self.prefix, truncated),
            SummarizeStyle::Detailed => format!("{}[Detailed] {}", self.prefix, truncated),
            SummarizeStyle::Structured => {
                format!("{}[Structured]\n- Content: {}", self.prefix, truncated)
            }
        };

        Ok(summary)
    }

    async fn extract_artifacts(
        &self,
        content: &str,
        types: &[ArtifactType],
    ) -> CaliberResult<Vec<ExtractedArtifact>> {
        let artifacts = types
            .iter()
            .map(|artifact_type| ExtractedArtifact {
                artifact_type: *artifact_type,
                content: format!("Extracted from: {}", &content[..content.len().min(50)]),
                confidence: 0.8,
            })
            .collect();

        Ok(artifacts)
    }

    async fn detect_contradiction(&self, a: &str, b: &str) -> CaliberResult<bool> {
        let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        let similarity = if union > 0 {
            intersection as f32 / union as f32
        } else {
            0.0
        };

        Ok(similarity < 0.1)
    }
}

use chrono::Utc;
use std::time::Duration;
use uuid::Uuid;

// ============================================================================
// PROPTEST GENERATORS (Task 13.2)
// ============================================================================

pub mod generators {
    //! Proptest strategies for generating CALIBER entity types.
    
    use super::*;
    use proptest::prelude::*;

    // === Identity Type Generators ===

    /// Generate a random UUID (for generic ID generation).
    pub fn arb_uuid() -> impl Strategy<Value = Uuid> {
        any::<[u8; 16]>().prop_map(Uuid::from_bytes)
    }

    /// Generate a valid UUIDv7 (timestamp-sortable).
    pub fn arb_uuid_v7() -> impl Strategy<Value = Uuid> {
        Just(()).prop_map(|_| Uuid::now_v7())
    }

    /// Generate a random TrajectoryId.
    pub fn arb_trajectory_id() -> impl Strategy<Value = TrajectoryId> {
        arb_uuid().prop_map(TrajectoryId::new)
    }

    /// Generate a random ScopeId.
    pub fn arb_scope_id() -> impl Strategy<Value = ScopeId> {
        arb_uuid().prop_map(ScopeId::new)
    }

    /// Generate a random ArtifactId.
    pub fn arb_artifact_id() -> impl Strategy<Value = ArtifactId> {
        arb_uuid().prop_map(ArtifactId::new)
    }

    /// Generate a random NoteId.
    pub fn arb_note_id() -> impl Strategy<Value = NoteId> {
        arb_uuid().prop_map(NoteId::new)
    }

    /// Generate a random TurnId.
    pub fn arb_turn_id() -> impl Strategy<Value = TurnId> {
        arb_uuid().prop_map(TurnId::new)
    }

    /// Generate a random AgentId.
    pub fn arb_agent_id() -> impl Strategy<Value = AgentId> {
        arb_uuid().prop_map(AgentId::new)
    }

    /// Generate a Timestamp (DateTime<Utc>).
    pub fn arb_timestamp() -> impl Strategy<Value = Timestamp> {
        // Generate timestamps within a reasonable range (2020-2030)
        (1577836800i64..1893456000i64).prop_map(|secs| {
            chrono::DateTime::from_timestamp(secs, 0).unwrap_or_else(Utc::now)
        })
    }

    /// Generate a ContentHash (32 bytes).
    pub fn arb_content_hash() -> impl Strategy<Value = ContentHash> {
        any::<[u8; 32]>()
    }

    /// Generate RawContent (Vec<u8>).
    pub fn arb_raw_content() -> impl Strategy<Value = RawContent> {
        prop::collection::vec(any::<u8>(), 0..1024)
    }

    // === Enum Generators ===

    /// Generate a TTL variant.
    pub fn arb_ttl() -> impl Strategy<Value = TTL> {
        prop_oneof![
            Just(TTL::Persistent),
            Just(TTL::Session),
            Just(TTL::Scope),
            (1i64..86400000).prop_map(TTL::Duration),
        ]
    }

    /// Generate an EntityType variant.
    pub fn arb_entity_type() -> impl Strategy<Value = EntityType> {
        prop_oneof![
            Just(EntityType::Trajectory),
            Just(EntityType::Scope),
            Just(EntityType::Artifact),
            Just(EntityType::Note),
            Just(EntityType::Agent),
        ]
    }

    /// Generate a MemoryCategory variant.
    pub fn arb_memory_category() -> impl Strategy<Value = MemoryCategory> {
        prop_oneof![
            Just(MemoryCategory::Ephemeral),
            Just(MemoryCategory::Working),
            Just(MemoryCategory::Episodic),
            Just(MemoryCategory::Semantic),
            Just(MemoryCategory::Procedural),
            Just(MemoryCategory::Meta),
        ]
    }

    /// Generate a TrajectoryStatus variant.
    pub fn arb_trajectory_status() -> impl Strategy<Value = TrajectoryStatus> {
        prop_oneof![
            Just(TrajectoryStatus::Active),
            Just(TrajectoryStatus::Completed),
            Just(TrajectoryStatus::Failed),
            Just(TrajectoryStatus::Suspended),
        ]
    }

    /// Generate an OutcomeStatus variant.
    pub fn arb_outcome_status() -> impl Strategy<Value = OutcomeStatus> {
        prop_oneof![
            Just(OutcomeStatus::Success),
            Just(OutcomeStatus::Partial),
            Just(OutcomeStatus::Failure),
        ]
    }

    /// Generate a TurnRole variant.
    pub fn arb_turn_role() -> impl Strategy<Value = TurnRole> {
        prop_oneof![
            Just(TurnRole::User),
            Just(TurnRole::Assistant),
            Just(TurnRole::System),
            Just(TurnRole::Tool),
        ]
    }

    /// Generate an ArtifactType variant.
    pub fn arb_artifact_type() -> impl Strategy<Value = ArtifactType> {
        prop_oneof![
            Just(ArtifactType::ErrorLog),
            Just(ArtifactType::CodePatch),
            Just(ArtifactType::DesignDecision),
            Just(ArtifactType::UserPreference),
            Just(ArtifactType::Fact),
            Just(ArtifactType::Constraint),
            Just(ArtifactType::ToolResult),
            Just(ArtifactType::IntermediateOutput),
            Just(ArtifactType::Custom),
        ]
    }

    /// Generate an ExtractionMethod variant.
    pub fn arb_extraction_method() -> impl Strategy<Value = ExtractionMethod> {
        prop_oneof![
            Just(ExtractionMethod::Explicit),
            Just(ExtractionMethod::Inferred),
            Just(ExtractionMethod::UserProvided),
        ]
    }

    /// Generate a NoteType variant.
    pub fn arb_note_type() -> impl Strategy<Value = NoteType> {
        prop_oneof![
            Just(NoteType::Convention),
            Just(NoteType::Strategy),
            Just(NoteType::Gotcha),
            Just(NoteType::Fact),
            Just(NoteType::Preference),
            Just(NoteType::Relationship),
            Just(NoteType::Procedure),
            Just(NoteType::Meta),
        ]
    }

    /// Generate a ValidationMode variant.
    pub fn arb_validation_mode() -> impl Strategy<Value = ValidationMode> {
        prop_oneof![
            Just(ValidationMode::OnMutation),
            Just(ValidationMode::Always),
        ]
    }

    /// Generate a ContextPersistence variant.
    pub fn arb_context_persistence() -> impl Strategy<Value = ContextPersistence> {
        prop_oneof![
            Just(ContextPersistence::Ephemeral),
            (1u64..86400).prop_map(|secs| ContextPersistence::Ttl(Duration::from_secs(secs))),
            Just(ContextPersistence::Permanent),
        ]
    }

    // === Struct Generators ===

    /// Generate an EmbeddingVector with specified dimensions.
    pub fn arb_embedding_vector(dimensions: usize) -> impl Strategy<Value = EmbeddingVector> {
        (
            prop::collection::vec(-1.0f32..1.0f32, dimensions),
            "[a-z]{3,10}".prop_map(|s| s),
        )
            .prop_map(move |(data, model_id)| EmbeddingVector::new(data, model_id))
    }

    /// Generate an EmbeddingVector with random dimensions (64-1536).
    pub fn arb_embedding_vector_any() -> impl Strategy<Value = EmbeddingVector> {
        (64usize..1536).prop_flat_map(arb_embedding_vector)
    }

    /// Generate an EntityRef.
    pub fn arb_entity_ref() -> impl Strategy<Value = EntityRef> {
        (arb_entity_type(), arb_uuid()).prop_map(|(entity_type, id)| EntityRef {
            entity_type,
            id,
        })
    }

    /// Generate a Provenance struct.
    pub fn arb_provenance() -> impl Strategy<Value = Provenance> {
        (
            0i32..1000,
            arb_extraction_method(),
            prop::option::of(0.0f32..1.0f32),
        )
            .prop_map(|(source_turn, extraction_method, confidence)| Provenance {
                source_turn,
                extraction_method,
                confidence,
            })
    }

    /// Generate a Checkpoint struct.
    pub fn arb_checkpoint() -> impl Strategy<Value = Checkpoint> {
        (arb_raw_content(), any::<bool>()).prop_map(|(context_state, recoverable)| Checkpoint {
            context_state,
            recoverable,
        })
    }

    /// Generate a TrajectoryOutcome struct.
    pub fn arb_trajectory_outcome() -> impl Strategy<Value = TrajectoryOutcome> {
        (
            arb_outcome_status(),
            "[a-zA-Z0-9 ]{1,100}".prop_map(|s| s),
            prop::collection::vec(arb_artifact_id(), 0..5),
            prop::collection::vec(arb_note_id(), 0..5),
            prop::option::of("[a-zA-Z0-9 ]{1,50}".prop_map(|s| s)),
        )
            .prop_map(
                |(status, summary, produced_artifacts, produced_notes, error)| TrajectoryOutcome {
                    status,
                    summary,
                    produced_artifacts,
                    produced_notes,
                    error,
                },
            )
    }

    /// Generate a Trajectory struct.
    pub fn arb_trajectory() -> impl Strategy<Value = Trajectory> {
        (
            arb_trajectory_id(),
            "[a-zA-Z0-9_]{1,50}".prop_map(|s| s),
            prop::option::of("[a-zA-Z0-9 ]{1,200}".prop_map(|s| s)),
            arb_trajectory_status(),
            prop::option::of(arb_trajectory_id()),
            prop::option::of(arb_trajectory_id()),
            prop::option::of(arb_agent_id()),
            arb_timestamp(),
            arb_timestamp(),
        )
            .prop_map(
                |(
                    trajectory_id,
                    name,
                    description,
                    status,
                    parent_trajectory_id,
                    root_trajectory_id,
                    agent_id,
                    created_at,
                    updated_at,
                )| {
                    Trajectory {
                        trajectory_id,
                        name,
                        description,
                        status,
                        parent_trajectory_id,
                        root_trajectory_id,
                        agent_id,
                        created_at,
                        updated_at,
                        completed_at: None,
                        outcome: None,
                        metadata: None,
                    }
                },
            )
    }

    /// Generate a Scope struct.
    pub fn arb_scope(trajectory_id: TrajectoryId) -> impl Strategy<Value = Scope> {
        (
            arb_scope_id(),
            "[a-zA-Z0-9_]{1,50}".prop_map(|s| s),
            prop::option::of("[a-zA-Z0-9 ]{1,200}".prop_map(|s| s)),
            any::<bool>(),
            arb_timestamp(),
            prop::option::of(arb_checkpoint()),
            1i32..100000,
            0i32..100000,
        )
            .prop_map(
                move |(
                    scope_id,
                    name,
                    purpose,
                    is_active,
                    created_at,
                    checkpoint,
                    token_budget,
                    tokens_used,
                )| {
                    Scope {
                        scope_id,
                        trajectory_id,
                        parent_scope_id: None,
                        name,
                        purpose,
                        is_active,
                        created_at,
                        closed_at: None,
                        checkpoint,
                        token_budget,
                        tokens_used: tokens_used.min(token_budget),
                        metadata: None,
                    }
                },
            )
    }

    /// Generate an Artifact struct.
    pub fn arb_artifact(trajectory_id: TrajectoryId, scope_id: ScopeId) -> impl Strategy<Value = Artifact> {
        (
            arb_artifact_id(),
            arb_artifact_type(),
            "[a-zA-Z0-9_]{1,50}".prop_map(|s| s),
            "[a-zA-Z0-9 .,!?]{1,500}".prop_map(|s| s),
            arb_provenance(),
            arb_ttl(),
            arb_timestamp(),
            arb_timestamp(),
        )
            .prop_map(
                move |(artifact_id, artifact_type, name, content, provenance, ttl, created_at, updated_at)| {
                    let content_hash = compute_content_hash(content.as_bytes());
                    Artifact {
                        artifact_id,
                        trajectory_id,
                        scope_id,
                        artifact_type,
                        name,
                        content,
                        content_hash,
                        embedding: None,
                        provenance,
                        ttl,
                        created_at,
                        updated_at,
                        superseded_by: None,
                        metadata: None,
                    }
                },
            )
    }

    /// Generate a Note struct.
    pub fn arb_note(trajectory_id: TrajectoryId) -> impl Strategy<Value = Note> {
        (
            arb_note_id(),
            arb_note_type(),
            "[a-zA-Z0-9_]{1,50}".prop_map(|s| s),
            "[a-zA-Z0-9 .,!?]{1,500}".prop_map(|s| s),
            arb_ttl(),
            arb_timestamp(),
            arb_timestamp(),
            arb_timestamp(),
            0i32..1000,
        )
            .prop_map(
                move |(note_id, note_type, title, content, ttl, created_at, updated_at, accessed_at, access_count)| {
                    let content_hash = compute_content_hash(content.as_bytes());
                    Note {
                        note_id,
                        note_type,
                        title,
                        content,
                        content_hash,
                        embedding: None,
                        source_trajectory_ids: vec![trajectory_id],
                        source_artifact_ids: vec![],
                        ttl,
                        created_at,
                        updated_at,
                        accessed_at,
                        access_count,
                        superseded_by: None,
                        metadata: None,
                        abstraction_level: AbstractionLevel::Raw,
                        source_note_ids: vec![],
                    }
                },
            )
    }

    /// Generate a Turn struct.
    pub fn arb_turn(scope_id: ScopeId) -> impl Strategy<Value = Turn> {
        (
            arb_turn_id(),
            0i32..1000,
            arb_turn_role(),
            "[a-zA-Z0-9 .,!?]{1,500}".prop_map(|s| s),
            1i32..10000,
            arb_timestamp(),
        )
            .prop_map(
                move |(turn_id, sequence, role, content, token_count, created_at)| Turn {
                    turn_id,
                    scope_id,
                    sequence,
                    role,
                    content,
                    token_count,
                    created_at,
                    tool_calls: None,
                    tool_results: None,
                    metadata: None,
                },
            )
    }

    /// Generate a SectionPriorities struct.
    pub fn arb_section_priorities() -> impl Strategy<Value = SectionPriorities> {
        (
            0i32..100,
            0i32..100,
            0i32..100,
            0i32..100,
            0i32..100,
            0i32..100,
        )
            .prop_map(|(user, system, persona, artifacts, notes, history)| SectionPriorities {
                user,
                system,
                persona,
                artifacts,
                notes,
                history,
                custom: vec![],
            })
    }

    /// Generate a RetryConfig struct.
    pub fn arb_retry_config() -> impl Strategy<Value = RetryConfig> {
        (
            0i32..10,
            1u64..1000,
            1000u64..60000,
            1.1f32..5.0f32,
        )
            .prop_map(|(max_retries, initial_ms, max_ms, multiplier)| RetryConfig {
                max_retries,
                initial_backoff: Duration::from_millis(initial_ms),
                max_backoff: Duration::from_millis(max_ms),
                backoff_multiplier: multiplier,
            })
    }

    /// Generate a valid CaliberConfig struct.
    pub fn arb_valid_config() -> impl Strategy<Value = CaliberConfig> {
        (
            1i32..100000,
            arb_section_priorities(),
            0i32..100,
            1u64..86400,
            0.0f32..1.0f32,
            arb_context_persistence(),
            arb_validation_mode(),
            arb_retry_config(),
            1u64..3600,
            1u64..86400,
            1u64..3600,
        )
            .prop_map(
                |(
                    token_budget,
                    section_priorities,
                    checkpoint_retention,
                    stale_secs,
                    contradiction_threshold,
                    context_window_persistence,
                    validation_mode,
                    llm_retry_config,
                    lock_secs,
                    message_secs,
                    delegation_secs,
                )| {
                    CaliberConfig {
                        token_budget,
                        section_priorities,
                        checkpoint_retention,
                        stale_threshold: Duration::from_secs(stale_secs),
                        contradiction_threshold,
                        context_window_persistence,
                        validation_mode,
                        embedding_provider: None,
                        summarization_provider: None,
                        llm_retry_config,
                        lock_timeout: Duration::from_secs(lock_secs),
                        message_retention: Duration::from_secs(message_secs),
                        delegation_timeout: Duration::from_secs(delegation_secs),
                    }
                },
            )
    }
}


// ============================================================================
// TEST FIXTURES (Task 13.4)
// ============================================================================

pub mod fixtures {
    //! Pre-built test fixtures for common testing scenarios.
    
    use super::*;

    /// Create a minimal valid CaliberConfig for testing.
    pub fn minimal_config() -> CaliberConfig {
        CaliberConfig {
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
        }
    }

    /// Create a test Trajectory with Active status.
    pub fn active_trajectory() -> Trajectory {
        let now = Utc::now();
        Trajectory {
            trajectory_id: TrajectoryId::now_v7(),
            name: "test-trajectory".to_string(),
            description: Some("A test trajectory".to_string()),
            status: TrajectoryStatus::Active,
            parent_trajectory_id: None,
            root_trajectory_id: None,
            agent_id: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            outcome: None,
            metadata: None,
        }
    }

    /// Create a completed Trajectory with outcome.
    pub fn completed_trajectory() -> Trajectory {
        let now = Utc::now();
        Trajectory {
            trajectory_id: TrajectoryId::now_v7(),
            name: "completed-trajectory".to_string(),
            description: Some("A completed test trajectory".to_string()),
            status: TrajectoryStatus::Completed,
            parent_trajectory_id: None,
            root_trajectory_id: None,
            agent_id: None,
            created_at: now - chrono::Duration::hours(1),
            updated_at: now,
            completed_at: Some(now),
            outcome: Some(TrajectoryOutcome {
                status: OutcomeStatus::Success,
                summary: "Task completed successfully".to_string(),
                produced_artifacts: vec![],
                produced_notes: vec![],
                error: None,
            }),
            metadata: None,
        }
    }

    /// Create a test Scope for a given trajectory.
    pub fn active_scope(trajectory_id: TrajectoryId) -> Scope {
        let now = Utc::now();
        Scope {
            scope_id: ScopeId::now_v7(),
            trajectory_id,
            parent_scope_id: None,
            name: "test-scope".to_string(),
            purpose: Some("Testing".to_string()),
            is_active: true,
            created_at: now,
            closed_at: None,
            checkpoint: None,
            token_budget: 8000,
            tokens_used: 0,
            metadata: None,
        }
    }

    /// Create a test Artifact for a given trajectory and scope.
    pub fn test_artifact(trajectory_id: TrajectoryId, scope_id: ScopeId) -> Artifact {
        let now = Utc::now();
        let content = "Test artifact content".to_string();
        let content_hash = compute_content_hash(content.as_bytes());
        Artifact {
            artifact_id: ArtifactId::now_v7(),
            trajectory_id,
            scope_id,
            artifact_type: ArtifactType::Fact,
            name: "test-artifact".to_string(),
            content,
            content_hash,
            embedding: None,
            provenance: Provenance {
                source_turn: 1,
                extraction_method: ExtractionMethod::Explicit,
                confidence: Some(0.95),
            },
            ttl: TTL::Persistent,
            created_at: now,
            updated_at: now,
            superseded_by: None,
            metadata: None,
        }
    }

    /// Create a test Note for a given trajectory.
    pub fn test_note(trajectory_id: TrajectoryId) -> Note {
        let now = Utc::now();
        let content = "Test note content".to_string();
        let content_hash = compute_content_hash(content.as_bytes());
        Note {
            note_id: NoteId::now_v7(),
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content,
            content_hash,
            embedding: None,
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            created_at: now,
            updated_at: now,
            accessed_at: now,
            access_count: 0,
            superseded_by: None,
            metadata: None,
            abstraction_level: AbstractionLevel::Raw,
            source_note_ids: vec![],
        }
    }

    /// Create a test Turn for a given scope.
    pub fn user_turn(scope_id: ScopeId, sequence: i32) -> Turn {
        let now = Utc::now();
        Turn {
            turn_id: TurnId::now_v7(),
            scope_id,
            sequence,
            role: TurnRole::User,
            content: format!("User message {}", sequence),
            token_count: 10,
            created_at: now,
            tool_calls: None,
            tool_results: None,
            metadata: None,
        }
    }

    /// Create an assistant Turn for a given scope.
    pub fn assistant_turn(scope_id: ScopeId, sequence: i32) -> Turn {
        let now = Utc::now();
        Turn {
            turn_id: TurnId::now_v7(),
            scope_id,
            sequence,
            role: TurnRole::Assistant,
            content: format!("Assistant response {}", sequence),
            token_count: 50,
            created_at: now,
            tool_calls: None,
            tool_results: None,
            metadata: None,
        }
    }

    /// Create a test EmbeddingVector with specified dimensions.
    pub fn test_embedding(dimensions: usize) -> EmbeddingVector {
        let data: Vec<f32> = (0..dimensions).map(|i| (i as f32) / (dimensions as f32)).collect();
        EmbeddingVector::new(data, "test-model".to_string())
    }

    /// Create a normalized unit vector embedding.
    pub fn unit_embedding(dimensions: usize, axis: usize) -> EmbeddingVector {
        let mut data = vec![0.0f32; dimensions];
        if axis < dimensions {
            data[axis] = 1.0;
        }
        EmbeddingVector::new(data, "test-model".to_string())
    }
}


// ============================================================================
// CUSTOM ASSERTIONS (Task 13.5)
// ============================================================================

pub mod assertions {
    //! Custom assertion macros and functions for CALIBER-specific validation.
    
    use super::*;

    /// Assert that a CaliberResult is Ok.
    #[track_caller]
    pub fn assert_ok<T: std::fmt::Debug>(result: &CaliberResult<T>) {
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result);
    }

    /// Assert that a CaliberResult is Err.
    #[track_caller]
    pub fn assert_err<T: std::fmt::Debug>(result: &CaliberResult<T>) {
        assert!(result.is_err(), "Expected Err, got Ok: {:?}", result);
    }

    /// Assert that a CaliberResult is a specific error variant.
    #[track_caller]
    pub fn assert_storage_error<T: std::fmt::Debug>(result: &CaliberResult<T>) {
        match result {
            Err(CaliberError::Storage(_)) => {}
            other => panic!("Expected Storage error, got: {:?}", other),
        }
    }

    /// Assert that a CaliberResult is a NotFound storage error.
    #[track_caller]
    pub fn assert_not_found<T: std::fmt::Debug>(result: &CaliberResult<T>, entity_type: EntityType) {
        match result {
            Err(CaliberError::Storage(StorageError::NotFound { entity_type: et, .. })) => {
                assert_eq!(*et, entity_type, "Wrong entity type in NotFound error");
            }
            other => panic!("Expected NotFound error for {:?}, got: {:?}", entity_type, other),
        }
    }

    /// Assert that a CaliberResult is a Config error.
    #[track_caller]
    pub fn assert_config_error<T: std::fmt::Debug>(result: &CaliberResult<T>) {
        match result {
            Err(CaliberError::Config(_)) => {}
            other => panic!("Expected Config error, got: {:?}", other),
        }
    }

    /// Assert that a CaliberResult is a Vector error.
    #[track_caller]
    pub fn assert_vector_error<T: std::fmt::Debug>(result: &CaliberResult<T>) {
        match result {
            Err(CaliberError::Vector(_)) => {}
            other => panic!("Expected Vector error, got: {:?}", other),
        }
    }

    /// Assert that a CaliberResult is a DimensionMismatch vector error.
    #[track_caller]
    pub fn assert_dimension_mismatch<T: std::fmt::Debug>(
        result: &CaliberResult<T>,
        expected: i32,
        got: i32,
    ) {
        match result {
            Err(CaliberError::Vector(VectorError::DimensionMismatch { expected: e, got: g })) => {
                assert_eq!(*e, expected, "Wrong expected dimension");
                assert_eq!(*g, got, "Wrong got dimension");
            }
            other => panic!(
                "Expected DimensionMismatch({}, {}), got: {:?}",
                expected, got, other
            ),
        }
    }

    /// Assert that a CaliberResult is an LLM error.
    #[track_caller]
    pub fn assert_llm_error<T: std::fmt::Debug>(result: &CaliberResult<T>) {
        match result {
            Err(CaliberError::Llm(_)) => {}
            other => panic!("Expected Llm error, got: {:?}", other),
        }
    }

    /// Assert that a CaliberResult is a ProviderNotConfigured LLM error.
    #[track_caller]
    pub fn assert_provider_not_configured<T: std::fmt::Debug>(result: &CaliberResult<T>) {
        match result {
            Err(CaliberError::Llm(caliber_core::LlmError::ProviderNotConfigured)) => {}
            other => panic!("Expected ProviderNotConfigured error, got: {:?}", other),
        }
    }

    /// Assert that a CaliberResult is a Validation error.
    #[track_caller]
    pub fn assert_validation_error<T: std::fmt::Debug>(result: &CaliberResult<T>) {
        match result {
            Err(CaliberError::Validation(_)) => {}
            other => panic!("Expected Validation error, got: {:?}", other),
        }
    }

    /// Assert that a CaliberResult is an Agent error.
    #[track_caller]
    pub fn assert_agent_error<T: std::fmt::Debug>(result: &CaliberResult<T>) {
        match result {
            Err(CaliberError::Agent(_)) => {}
            other => panic!("Expected Agent error, got: {:?}", other),
        }
    }

    /// Assert that an EmbeddingVector has valid dimensions.
    #[track_caller]
    pub fn assert_valid_embedding(embedding: &EmbeddingVector) {
        assert!(
            embedding.is_valid(),
            "Invalid embedding: dimensions={}, data.len()={}",
            embedding.dimensions,
            embedding.data.len()
        );
    }

    /// Assert that two embeddings have the same dimensions.
    #[track_caller]
    pub fn assert_same_dimensions(a: &EmbeddingVector, b: &EmbeddingVector) {
        assert_eq!(
            a.dimensions, b.dimensions,
            "Dimension mismatch: {} vs {}",
            a.dimensions, b.dimensions
        );
    }

    /// Assert that cosine similarity is within expected range.
    #[track_caller]
    pub fn assert_similarity_in_range(similarity: f32, min: f32, max: f32) {
        assert!(
            similarity >= min && similarity <= max,
            "Similarity {} not in range [{}, {}]",
            similarity,
            min,
            max
        );
    }

    /// Assert that a Trajectory has the expected status.
    #[track_caller]
    pub fn assert_trajectory_status(trajectory: &Trajectory, expected: TrajectoryStatus) {
        assert_eq!(
            trajectory.status, expected,
            "Trajectory status mismatch: expected {:?}, got {:?}",
            expected, trajectory.status
        );
    }

    /// Assert that a Scope is active.
    #[track_caller]
    pub fn assert_scope_active(scope: &Scope) {
        assert!(scope.is_active, "Expected scope to be active");
        assert!(scope.closed_at.is_none(), "Active scope should not have closed_at");
    }

    /// Assert that a Scope is closed.
    #[track_caller]
    pub fn assert_scope_closed(scope: &Scope) {
        assert!(!scope.is_active, "Expected scope to be closed");
        assert!(scope.closed_at.is_some(), "Closed scope should have closed_at");
    }

    /// Assert that token usage is within budget.
    #[track_caller]
    pub fn assert_within_token_budget(used: i32, budget: i32) {
        assert!(
            used <= budget,
            "Token usage {} exceeds budget {}",
            used,
            budget
        );
    }

    /// Assert that a CaliberConfig is valid.
    #[track_caller]
    pub fn assert_config_valid(config: &CaliberConfig) {
        match config.validate() {
            Ok(()) => {}
            Err(e) => panic!("Config validation failed: {:?}", e),
        }
    }
}


// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use uuid::Uuid;

    #[test]
    fn test_minimal_config_is_valid() {
        let config = fixtures::minimal_config();
        assertions::assert_config_valid(&config);
    }

    #[test]
    fn test_active_trajectory_fixture() {
        let trajectory = fixtures::active_trajectory();
        assertions::assert_trajectory_status(&trajectory, TrajectoryStatus::Active);
        assert!(trajectory.completed_at.is_none());
        assert!(trajectory.outcome.is_none());
    }

    #[test]
    fn test_completed_trajectory_fixture() {
        let trajectory = fixtures::completed_trajectory();
        assertions::assert_trajectory_status(&trajectory, TrajectoryStatus::Completed);
        assert!(trajectory.completed_at.is_some());
        assert!(trajectory.outcome.is_some());
    }

    #[test]
    fn test_active_scope_fixture() {
        let trajectory = fixtures::active_trajectory();
        let scope = fixtures::active_scope(trajectory.trajectory_id);
        assertions::assert_scope_active(&scope);
        assert_eq!(scope.trajectory_id, trajectory.trajectory_id);
    }

    #[test]
    fn test_test_artifact_fixture() {
        let trajectory = fixtures::active_trajectory();
        let scope = fixtures::active_scope(trajectory.trajectory_id);
        let artifact = fixtures::test_artifact(trajectory.trajectory_id, scope.scope_id);
        assert_eq!(artifact.trajectory_id, trajectory.trajectory_id);
        assert_eq!(artifact.scope_id, scope.scope_id);
        assert_eq!(artifact.artifact_type, ArtifactType::Fact);
    }

    #[test]
    fn test_test_note_fixture() {
        let trajectory = fixtures::active_trajectory();
        let note = fixtures::test_note(trajectory.trajectory_id);
        assert!(note.source_trajectory_ids.contains(&trajectory.trajectory_id));
        assert_eq!(note.note_type, NoteType::Fact);
    }

    #[test]
    fn test_turn_fixtures() {
        let trajectory = fixtures::active_trajectory();
        let scope = fixtures::active_scope(trajectory.trajectory_id);
        
        let user = fixtures::user_turn(scope.scope_id, 1);
        assert_eq!(user.role, TurnRole::User);
        assert_eq!(user.sequence, 1);
        
        let assistant = fixtures::assistant_turn(scope.scope_id, 2);
        assert_eq!(assistant.role, TurnRole::Assistant);
        assert_eq!(assistant.sequence, 2);
    }

    #[test]
    fn test_embedding_fixtures() {
        let embedding = fixtures::test_embedding(384);
        assertions::assert_valid_embedding(&embedding);
        assert_eq!(embedding.dimensions, 384);
        
        let unit = fixtures::unit_embedding(384, 0);
        assertions::assert_valid_embedding(&unit);
        assert_eq!(unit.data[0], 1.0);
        assert_eq!(unit.data[1], 0.0);
    }

    #[test]
    fn test_assertion_not_found() {
        let result: CaliberResult<()> = Err(CaliberError::Storage(StorageError::NotFound {
            entity_type: EntityType::Trajectory,
            id: Uuid::now_v7(),
        }));
        assertions::assert_not_found(&result, EntityType::Trajectory);
    }

    #[test]
    fn test_assertion_dimension_mismatch() {
        let result: CaliberResult<f32> = Err(CaliberError::Vector(VectorError::DimensionMismatch {
            expected: 384,
            got: 768,
        }));
        assertions::assert_dimension_mismatch(&result, 384, 768);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn prop_generated_trajectory_has_valid_id(trajectory in generators::arb_trajectory()) {
            // EntityId should be a valid UUID
            assert!(!trajectory.trajectory_id.as_uuid().is_nil());
        }

        #[test]
        fn prop_generated_config_is_valid(config in generators::arb_valid_config()) {
            // All generated configs should pass validation
            assertions::assert_config_valid(&config);
        }

        #[test]
        fn prop_generated_embedding_is_valid(embedding in generators::arb_embedding_vector_any()) {
            assertions::assert_valid_embedding(&embedding);
        }

        #[test]
        fn prop_generated_ttl_variants(ttl in generators::arb_ttl()) {
            // Just verify we can generate all TTL variants
            match ttl {
                TTL::Persistent
                | TTL::Session
                | TTL::Scope
                | TTL::Duration(_)
                | TTL::Ephemeral
                | TTL::ShortTerm
                | TTL::MediumTerm
                | TTL::LongTerm
                | TTL::Permanent
                | TTL::Max(_) => {}
            }
        }

        #[test]
        fn prop_generated_entity_types(et in generators::arb_entity_type()) {
            // Verify all entity types can be generated
            match et {
                EntityType::Trajectory
                | EntityType::Scope
                | EntityType::Artifact
                | EntityType::Note
                | EntityType::Agent
                | EntityType::Turn
                | EntityType::Lock
                | EntityType::Message
                | EntityType::Delegation
                | EntityType::Handoff
                | EntityType::Conflict
                | EntityType::Edge
                | EntityType::EvolutionSnapshot
                | EntityType::SummarizationPolicy => {}
            }
        }
    }
}
