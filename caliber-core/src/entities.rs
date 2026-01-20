//! Core entity structures

use crate::*;
use serde::{Deserialize, Serialize};

/// Reference to an entity by type and ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EntityRef {
    pub entity_type: EntityType,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub id: EntityId,
}

/// Trajectory - top-level task container.
/// A trajectory represents a complete task or goal being pursued.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Trajectory {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    pub name: String,
    pub description: Option<String>,
    pub status: TrajectoryStatus,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_trajectory_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub root_trajectory_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub updated_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    pub outcome: Option<TrajectoryOutcome>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Outcome of a completed trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrajectoryOutcome {
    pub status: OutcomeStatus,
    pub summary: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_artifacts: Vec<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_notes: Vec<EntityId>,
    pub error: Option<String>,
}

/// Scope - partitioned context window within a trajectory.
/// Scopes provide isolation and checkpointing boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Scope {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_scope_id: Option<EntityId>,
    pub name: String,
    pub purpose: Option<String>,
    pub is_active: bool,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub closed_at: Option<Timestamp>,
    pub checkpoint: Option<Checkpoint>,
    pub token_budget: i32,
    pub tokens_used: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Checkpoint for scope recovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Checkpoint {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_state: RawContent,
    pub recoverable: bool,
}

/// Artifact - typed output preserved across scopes.
/// Artifacts survive scope closure and can be referenced later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Artifact {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub artifact_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    pub artifact_type: ArtifactType,
    pub name: String,
    pub content: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub content_hash: ContentHash,
    pub embedding: Option<EmbeddingVector>,
    pub provenance: Provenance,
    pub ttl: TTL,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub updated_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub superseded_by: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Provenance information for an artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Provenance {
    pub source_turn: i32,
    pub extraction_method: ExtractionMethod,
    pub confidence: Option<f32>,
}

/// Note - long-term cross-trajectory knowledge.
/// Notes persist beyond individual trajectories.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Note {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub note_id: EntityId,
    pub note_type: NoteType,
    pub title: String,
    pub content: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub content_hash: ContentHash,
    pub embedding: Option<EmbeddingVector>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_trajectory_ids: Vec<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_artifact_ids: Vec<EntityId>,
    pub ttl: TTL,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub updated_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub accessed_at: Timestamp,
    pub access_count: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub superseded_by: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
    // ══════════════════════════════════════════════════════════════════════════
    // Battle Intel Feature 2: Abstraction levels (EVOLVE-MEM L0/L1/L2 hierarchy)
    // ══════════════════════════════════════════════════════════════════════════
    /// Semantic abstraction tier (Raw=L0, Summary=L1, Principle=L2)
    pub abstraction_level: AbstractionLevel,
    /// Notes this was derived from (for L1/L2 derivation chains)
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_note_ids: Vec<EntityId>,
}

/// Turn - ephemeral conversation buffer entry.
/// Turns die with their scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Turn {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub turn_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    pub sequence: i32,
    pub role: TurnRole,
    pub content: String,
    pub token_count: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_calls: Option<serde_json::Value>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_results: Option<serde_json::Value>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

