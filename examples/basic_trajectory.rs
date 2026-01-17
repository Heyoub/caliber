//! Basic Trajectory Example
//!
//! Demonstrates the fundamental CALIBER workflow:
//! 1. Create a trajectory (task container)
//! 2. Create a scope (context partition)
//! 3. Add artifacts (preserved outputs)
//! 4. Query and retrieve data
//!
//! This example uses the in-memory storage for simplicity.
//! For production, use caliber-pg with PostgreSQL.

use caliber_core::{
    Artifact, ArtifactType, CaliberConfig, CaliberResult, ContextPersistence, EntityId,
    ExtractionMethod, MemoryCategory, Note, NoteType, Provenance, RawContent, RetryConfig,
    Scope, SectionPriorities, Trajectory, TrajectoryStatus, TTL, Turn, TurnRole,
    ValidationMode, compute_content_hash, new_entity_id,
};
use caliber_storage::{MockStorage, StorageTrait};
use chrono::Utc;
use std::time::Duration;

fn main() -> CaliberResult<()> {
    println!("=== CALIBER Basic Trajectory Example ===\n");

    // Step 1: Create configuration (NO DEFAULTS - all explicit)
    let config = create_config();
    println!("✓ Configuration created");
    println!("  Token budget: {}", config.token_budget);
    println!("  Checkpoint retention: {}", config.checkpoint_retention);

    // Step 2: Initialize storage
    let storage = MockStorage::new();
    println!("\n✓ Storage initialized (in-memory mock)");

    // Step 3: Create a trajectory
    let trajectory = create_trajectory(&storage)?;
    println!("\n✓ Trajectory created");
    println!("  ID: {}", trajectory.trajectory_id);
    println!("  Name: {}", trajectory.name);
    println!("  Status: {:?}", trajectory.status);

    // Step 4: Create a scope within the trajectory
    let scope = create_scope(&storage, trajectory.trajectory_id)?;
    println!("\n✓ Scope created");
    println!("  ID: {}", scope.scope_id);
    println!("  Name: {}", scope.name);
    println!("  Trajectory: {}", scope.trajectory_id);
    println!("  Token budget: {}", scope.token_budget);

    // Step 5: Add artifacts to the scope
    let artifacts = create_artifacts(&storage, scope.scope_id)?;
    println!("\n✓ Artifacts created: {}", artifacts.len());
    for (i, artifact) in artifacts.iter().enumerate() {
        println!("  {}. {} - {} bytes", i + 1, artifact.artifact_type, artifact.content.len());
    }

    // Step 6: Add a turn (conversation entry)
    let turn = create_turn(&storage, scope.scope_id)?;
    println!("\n✓ Turn created");
    println!("  Role: {:?}", turn.role);
    println!("  Sequence: {}", turn.sequence);
    println!("  Content: {} bytes", turn.content.len());

    // Step 7: Add a note (cross-trajectory knowledge)
    let note = create_note(&storage, trajectory.trajectory_id)?;
    println!("\n✓ Note created");
    println!("  Type: {:?}", note.note_type);
    println!("  Content: {} bytes", note.content.len());

    // Step 8: Query data back
    println!("\n=== Querying Data ===");
    
    // Get trajectory
    let retrieved_trajectory = storage.get_trajectory(trajectory.trajectory_id)?
        .expect("Trajectory should exist");
    println!("✓ Retrieved trajectory: {}", retrieved_trajectory.name);

    // Get scope
    let retrieved_scope = storage.get_scope(scope.scope_id)?
        .expect("Scope should exist");
    println!("✓ Retrieved scope: {}", retrieved_scope.name);

    // List artifacts in scope
    let scope_artifacts = storage.list_artifacts_by_scope(scope.scope_id)?;
    println!("✓ Found {} artifacts in scope", scope_artifacts.len());

    // Get turns in scope
    let scope_turns = storage.get_turns_by_scope(scope.scope_id)?;
    println!("✓ Found {} turns in scope", scope_turns.len());

    println!("\n=== Example Complete ===");
    println!("This demonstrates the basic CALIBER workflow:");
    println!("  Trajectory → Scope → Artifacts/Turns → Notes");
    println!("\nNext steps:");
    println!("  - Run 'context_assembly' example for context assembly");
    println!("  - Run 'multi_agent_coordination' for agent coordination");
    println!("  - Run 'vector_search' for semantic search");

    Ok(())
}

/// Create CALIBER configuration with all required fields.
/// NO DEFAULTS - every value must be explicitly provided.
fn create_config() -> CaliberConfig {
    CaliberConfig {
        token_budget: 8000,
        checkpoint_retention: 5,
        stale_threshold: Duration::from_secs(86400 * 30), // 30 days
        contradiction_threshold: 0.85,
        context_window_persistence: ContextPersistence::Ttl(Duration::from_secs(86400)),
        validation_mode: ValidationMode::OnMutation,
        section_priorities: SectionPriorities {
            user: 100,
            system: 90,
            persona: 85,
            artifacts: 80,
            notes: 70,
            history: 60,
            custom: vec![],
        },
        embedding_provider: None,
        summarization_provider: None,
        llm_retry_config: RetryConfig {
            max_retries: 3,
            initial_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(2),
            backoff_multiplier: 2.0,
        },
        lock_timeout: Duration::from_secs(30),
        message_retention: Duration::from_secs(86400),
        delegation_timeout: Duration::from_secs(3600),
    }
}

/// Create a trajectory (task container).
fn create_trajectory(storage: &MockStorage) -> CaliberResult<Trajectory> {
    let trajectory = Trajectory {
        trajectory_id: new_entity_id(),
        name: "example-task".to_string(),
        description: Some("Demonstrates basic CALIBER workflow".to_string()),
        status: TrajectoryStatus::Active,
        parent_trajectory_id: None,
        root_trajectory_id: None,
        agent_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        completed_at: None,
        outcome: None,
        metadata: serde_json::json!({
            "example": true,
            "category": "tutorial"
        }),
    };

    storage.create_trajectory(trajectory.clone())?;
    Ok(trajectory)
}

/// Create a scope (context partition) within a trajectory.
fn create_scope(storage: &MockStorage, trajectory_id: EntityId) -> CaliberResult<Scope> {
    let scope = Scope {
        scope_id: new_entity_id(),
        trajectory_id,
        name: "initial-scope".to_string(),
        description: Some("First scope in the trajectory".to_string()),
        is_active: true,
        token_budget: 8000,
        tokens_used: 0,
        created_at: Utc::now(),
        closed_at: None,
        metadata: serde_json::json!({
            "phase": "initialization"
        }),
    };

    storage.create_scope(scope.clone())?;
    Ok(scope)
}

/// Create sample artifacts in a scope.
fn create_artifacts(storage: &MockStorage, scope_id: EntityId) -> CaliberResult<Vec<Artifact>> {
    let mut artifacts = Vec::new();

    // Code artifact
    let code_content = b"fn hello_world() { println!(\"Hello, CALIBER!\"); }";
    let code_artifact = Artifact {
        artifact_id: new_entity_id(),
        trajectory_id: storage.get_scope(scope_id)?.unwrap().trajectory_id,
        scope_id,
        artifact_type: ArtifactType::Code,
        content: code_content.to_vec(),
        content_hash: compute_content_hash(code_content),
        extraction_method: ExtractionMethod::Explicit,
        memory_category: MemoryCategory::Episodic,
        ttl: TTL::Persistent,
        embedding: None,
        provenance: Provenance {
            source: "user".to_string(),
            created_by: None,
            created_at: Utc::now(),
            version: 1,
        },
        metadata: serde_json::json!({
            "language": "rust",
            "file": "example.rs"
        }),
        created_at: Utc::now(),
    };
    storage.create_artifact(code_artifact.clone())?;
    artifacts.push(code_artifact);

    // Documentation artifact
    let doc_content = b"# CALIBER Example\n\nThis demonstrates basic usage.";
    let doc_artifact = Artifact {
        artifact_id: new_entity_id(),
        trajectory_id: storage.get_scope(scope_id)?.unwrap().trajectory_id,
        scope_id,
        artifact_type: ArtifactType::Documentation,
        content: doc_content.to_vec(),
        content_hash: compute_content_hash(doc_content),
        extraction_method: ExtractionMethod::Explicit,
        memory_category: MemoryCategory::Episodic,
        ttl: TTL::Persistent,
        embedding: None,
        provenance: Provenance {
            source: "user".to_string(),
            created_by: None,
            created_at: Utc::now(),
            version: 1,
        },
        metadata: serde_json::json!({
            "format": "markdown"
        }),
        created_at: Utc::now(),
    };
    storage.create_artifact(doc_artifact.clone())?;
    artifacts.push(doc_artifact);

    Ok(artifacts)
}

/// Create a turn (conversation entry) in a scope.
fn create_turn(storage: &MockStorage, scope_id: EntityId) -> CaliberResult<Turn> {
    let turn = Turn {
        turn_id: new_entity_id(),
        scope_id,
        sequence: 1,
        role: TurnRole::User,
        content: b"Create a hello world function in Rust".to_vec(),
        token_count: 8,
        created_at: Utc::now(),
        metadata: serde_json::json!({}),
    };

    storage.create_turn(turn.clone())?;
    Ok(turn)
}

/// Create a note (cross-trajectory knowledge).
fn create_note(storage: &MockStorage, trajectory_id: EntityId) -> CaliberResult<Note> {
    let note_content = b"Rust functions use the 'fn' keyword";
    let note = Note {
        note_id: new_entity_id(),
        note_type: NoteType::Insight,
        content: note_content.to_vec(),
        content_hash: compute_content_hash(note_content),
        source_trajectory_ids: vec![trajectory_id],
        embedding: None,
        access_count: 0,
        created_at: Utc::now(),
        accessed_at: Utc::now(),
        metadata: serde_json::json!({
            "topic": "rust-syntax"
        }),
    };

    storage.create_note(note.clone())?;
    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_workflow() {
        let result = main();
        assert!(result.is_ok(), "Basic workflow should complete successfully");
    }

    #[test]
    fn test_config_validation() {
        let config = create_config();
        let result = config.validate();
        assert!(result.is_ok(), "Config should be valid");
    }
}
