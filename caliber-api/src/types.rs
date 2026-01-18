//! API Request and Response Types
//!
//! This module defines all request and response types for the CALIBER API.
//! These types are used by both REST and gRPC endpoints.

use caliber_core::{
    AbstractionLevel, ArtifactType, EdgeType, EntityId, EntityType, ExtractionMethod, NoteType,
    OutcomeStatus, SummarizationTrigger, Timestamp, TrajectoryStatus, TurnRole, TTL,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ============================================================================
// TRAJECTORY TYPES
// ============================================================================

/// Request to create a new trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateTrajectoryRequest {
    /// Name of the trajectory
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Parent trajectory ID (for sub-tasks)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_trajectory_id: Option<EntityId>,
    /// Agent assigned to this trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<EntityId>,
    /// Additional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to update an existing trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateTrajectoryRequest {
    /// New name (if changing)
    pub name: Option<String>,
    /// New description (if changing)
    pub description: Option<String>,
    /// New status (if changing)
    pub status: Option<TrajectoryStatus>,
    /// New metadata (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to list trajectories with filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListTrajectoriesRequest {
    /// Filter by status
    pub status: Option<TrajectoryStatus>,
    /// Filter by agent
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<EntityId>,
    /// Filter by parent trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_id: Option<EntityId>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

/// Response containing a list of trajectories.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListTrajectoriesResponse {
    /// List of trajectories
    pub trajectories: Vec<TrajectoryResponse>,
    /// Total count (before pagination)
    pub total: i32,
}

/// Trajectory response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrajectoryResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Tenant this trajectory belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
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
    pub outcome: Option<TrajectoryOutcomeResponse>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Trajectory outcome details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrajectoryOutcomeResponse {
    pub status: OutcomeStatus,
    pub summary: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_artifacts: Vec<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub produced_notes: Vec<EntityId>,
    pub error: Option<String>,
}

// ============================================================================
// SCOPE TYPES
// ============================================================================

/// Request to create a new scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateScopeRequest {
    /// Trajectory this scope belongs to
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Parent scope (for nested scopes)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub parent_scope_id: Option<EntityId>,
    /// Name of the scope
    pub name: String,
    /// Purpose/description
    pub purpose: Option<String>,
    /// Token budget for this scope
    pub token_budget: i32,
    /// Additional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to update an existing scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateScopeRequest {
    /// New name (if changing)
    pub name: Option<String>,
    /// New purpose (if changing)
    pub purpose: Option<String>,
    /// New token budget (if changing)
    pub token_budget: Option<i32>,
    /// New metadata (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to create a checkpoint for a scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateCheckpointRequest {
    /// Serialized context state
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_state: Vec<u8>,
    /// Whether this checkpoint is recoverable
    pub recoverable: bool,
}

/// Scope response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ScopeResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    /// Tenant this scope belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
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
    pub checkpoint: Option<CheckpointResponse>,
    pub token_budget: i32,
    pub tokens_used: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Checkpoint response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CheckpointResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_state: Vec<u8>,
    pub recoverable: bool,
}

// ============================================================================
// ARTIFACT TYPES
// ============================================================================

/// Request to create a new artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateArtifactRequest {
    /// Trajectory this artifact belongs to
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Scope this artifact was created in
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    /// Type of artifact
    pub artifact_type: ArtifactType,
    /// Name of the artifact
    pub name: String,
    /// Content of the artifact
    pub content: String,
    /// Source turn number
    pub source_turn: i32,
    /// Extraction method used
    pub extraction_method: ExtractionMethod,
    /// Confidence score (0.0-1.0)
    pub confidence: Option<f32>,
    /// Time-to-live configuration
    pub ttl: TTL,
    /// Additional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to update an existing artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateArtifactRequest {
    /// New name (if changing)
    pub name: Option<String>,
    /// New content (if changing)
    pub content: Option<String>,
    /// New artifact type (if changing)
    pub artifact_type: Option<ArtifactType>,
    /// New TTL (if changing)
    pub ttl: Option<TTL>,
    /// New metadata (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to list artifacts with filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListArtifactsRequest {
    /// Filter by artifact type
    pub artifact_type: Option<ArtifactType>,
    /// Filter by trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Filter by scope
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<EntityId>,
    /// Filter by date range (from)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub created_after: Option<Timestamp>,
    /// Filter by date range (to)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub created_before: Option<Timestamp>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

/// Response containing a list of artifacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListArtifactsResponse {
    /// List of artifacts
    pub artifacts: Vec<ArtifactResponse>,
    /// Total count (before pagination)
    pub total: i32,
}

/// Artifact response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ArtifactResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub artifact_id: EntityId,
    /// Tenant this artifact belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    pub artifact_type: ArtifactType,
    pub name: String,
    pub content: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub content_hash: [u8; 32],
    pub embedding: Option<EmbeddingResponse>,
    pub provenance: ProvenanceResponse,
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
pub struct ProvenanceResponse {
    pub source_turn: i32,
    pub extraction_method: ExtractionMethod,
    pub confidence: Option<f32>,
}

/// Embedding vector response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EmbeddingResponse {
    pub data: Vec<f32>,
    pub model_id: String,
    pub dimensions: i32,
}

// ============================================================================
// NOTE TYPES
// ============================================================================

/// Request to create a new note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateNoteRequest {
    /// Type of note
    pub note_type: NoteType,
    /// Title of the note
    pub title: String,
    /// Content of the note
    pub content: String,
    /// Source trajectories
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_trajectory_ids: Vec<EntityId>,
    /// Source artifacts
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_artifact_ids: Vec<EntityId>,
    /// Time-to-live configuration
    pub ttl: TTL,
    /// Additional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to update an existing note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateNoteRequest {
    /// New title (if changing)
    pub title: Option<String>,
    /// New content (if changing)
    pub content: Option<String>,
    /// New note type (if changing)
    pub note_type: Option<NoteType>,
    /// New TTL (if changing)
    pub ttl: Option<TTL>,
    /// New metadata (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to list notes with filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListNotesRequest {
    /// Filter by note type
    pub note_type: Option<NoteType>,
    /// Filter by source trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub source_trajectory_id: Option<EntityId>,
    /// Filter by date range (from)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub created_after: Option<Timestamp>,
    /// Filter by date range (to)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub created_before: Option<Timestamp>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

/// Response containing a list of notes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListNotesResponse {
    /// List of notes
    pub notes: Vec<NoteResponse>,
    /// Total count (before pagination)
    pub total: i32,
}

/// Note response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NoteResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub note_id: EntityId,
    /// Tenant this note belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
    pub note_type: NoteType,
    pub title: String,
    pub content: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub content_hash: [u8; 32],
    pub embedding: Option<EmbeddingResponse>,
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
}

// ============================================================================
// TURN TYPES
// ============================================================================

/// Request to create a new turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateTurnRequest {
    /// Scope this turn belongs to
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    /// Sequence number within the scope
    pub sequence: i32,
    /// Role of the turn
    pub role: TurnRole,
    /// Content of the turn
    pub content: String,
    /// Token count
    pub token_count: i32,
    /// Tool calls (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_calls: Option<serde_json::Value>,
    /// Tool results (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub tool_results: Option<serde_json::Value>,
    /// Additional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Turn response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TurnResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub turn_id: EntityId,
    /// Tenant this turn belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
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

// ============================================================================
// AGENT TYPES
// ============================================================================

/// Request to register a new agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RegisterAgentRequest {
    /// Type of agent
    pub agent_type: String,
    /// Capabilities this agent has
    pub capabilities: Vec<String>,
    /// Memory access permissions
    pub memory_access: MemoryAccessRequest,
    /// Agent types this agent can delegate to
    pub can_delegate_to: Vec<String>,
    /// Supervisor agent (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub reports_to: Option<EntityId>,
}

/// Memory access configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryAccessRequest {
    /// Read permissions
    pub read: Vec<MemoryPermissionRequest>,
    /// Write permissions
    pub write: Vec<MemoryPermissionRequest>,
}

/// A single memory permission entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryPermissionRequest {
    /// Type of memory
    pub memory_type: String,
    /// Scope of the permission
    pub scope: String,
    /// Optional filter expression
    pub filter: Option<String>,
}

/// Request to update an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateAgentRequest {
    /// New status (if changing)
    pub status: Option<String>,
    /// New current trajectory (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_trajectory_id: Option<EntityId>,
    /// New current scope (if changing)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_scope_id: Option<EntityId>,
    /// New capabilities (if changing)
    pub capabilities: Option<Vec<String>>,
    /// New memory access (if changing)
    pub memory_access: Option<MemoryAccessRequest>,
}

/// Agent response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub agent_id: EntityId,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub memory_access: MemoryAccessResponse,
    pub status: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_trajectory_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub current_scope_id: Option<EntityId>,
    pub can_delegate_to: Vec<String>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub reports_to: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub last_heartbeat: Timestamp,
}

/// Memory access response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryAccessResponse {
    pub read: Vec<MemoryPermissionResponse>,
    pub write: Vec<MemoryPermissionResponse>,
}

/// Memory permission response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemoryPermissionResponse {
    pub memory_type: String,
    pub scope: String,
    pub filter: Option<String>,
}

/// Request to list agents with filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListAgentsRequest {
    /// Filter by agent type
    pub agent_type: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by current trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Only return active agents
    pub active_only: Option<bool>,
}

/// Response containing a list of agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListAgentsResponse {
    /// List of agents
    pub agents: Vec<AgentResponse>,
    /// Total count
    pub total: i32,
}

// ============================================================================
// LOCK TYPES
// ============================================================================

/// Request to acquire a lock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AcquireLockRequest {
    /// Type of resource to lock
    pub resource_type: String,
    /// ID of the resource to lock
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub resource_id: EntityId,
    /// Agent requesting the lock
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub holder_agent_id: EntityId,
    /// Lock timeout in milliseconds
    pub timeout_ms: i64,
    /// Lock mode (Exclusive or Shared)
    pub mode: String,
}

/// Request to extend a lock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExtendLockRequest {
    /// Additional time in milliseconds
    pub additional_ms: i64,
}

/// Lock response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LockResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub lock_id: EntityId,
    pub resource_type: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub resource_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub holder_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub acquired_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub expires_at: Timestamp,
    pub mode: String,
}

/// Response containing a list of locks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListLocksResponse {
    /// List of locks
    pub locks: Vec<LockResponse>,
    /// Total count
    pub total: i32,
}

// ============================================================================
// MESSAGE TYPES
// ============================================================================

/// Request to send a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SendMessageRequest {
    /// Agent sending the message
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    /// Specific agent to receive (if targeted)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<EntityId>,
    /// Agent type to receive (for broadcast)
    pub to_agent_type: Option<String>,
    /// Type of message
    pub message_type: String,
    /// Message payload (JSON serialized)
    pub payload: String,
    /// Related trajectory (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Related scope (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<EntityId>,
    /// Related artifacts (if any)
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifact_ids: Vec<EntityId>,
    /// Message priority
    pub priority: String,
    /// When the message expires (optional)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expires_at: Option<Timestamp>,
}

/// Message response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MessageResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub message_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<EntityId>,
    pub to_agent_type: Option<String>,
    pub message_type: String,
    pub payload: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifact_ids: Vec<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub delivered_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub acknowledged_at: Option<Timestamp>,
    pub priority: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expires_at: Option<Timestamp>,
}

/// Request to list messages with filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListMessagesRequest {
    /// Filter by message type
    pub message_type: Option<String>,
    /// Filter by sender agent
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub from_agent_id: Option<EntityId>,
    /// Filter by recipient agent
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub to_agent_id: Option<EntityId>,
    /// Filter by recipient agent type
    pub to_agent_type: Option<String>,
    /// Filter by trajectory
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Filter by priority
    pub priority: Option<String>,
    /// Only return undelivered messages
    pub undelivered_only: Option<bool>,
    /// Only return unacknowledged messages
    pub unacknowledged_only: Option<bool>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

/// Response containing a list of messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListMessagesResponse {
    /// List of messages
    pub messages: Vec<MessageResponse>,
    /// Total count
    pub total: i32,
}

// ============================================================================
// DELEGATION TYPES
// ============================================================================

/// Request to create a delegation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateDelegationRequest {
    /// Agent delegating the task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    /// Agent receiving the delegation
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: EntityId,
    /// Trajectory for the delegated task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Scope for the delegated task
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    /// Task description
    pub task_description: String,
    /// Expected completion time
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expected_completion: Option<Timestamp>,
    /// Additional context
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub context: Option<serde_json::Value>,
}

/// Delegation response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub delegation_id: EntityId,
    /// Tenant this delegation belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    pub task_description: String,
    pub status: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub expected_completion: Option<Timestamp>,
    pub result: Option<DelegationResultResponse>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub context: Option<serde_json::Value>,
}

/// Delegation result response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationResultResponse {
    pub status: String,
    pub output: Option<String>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifacts: Vec<EntityId>,
    pub error: Option<String>,
}

/// Request to complete a delegation with results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DelegationResultRequest {
    /// Result status
    pub status: String,
    /// Output from the delegated task
    pub output: Option<String>,
    /// Artifacts produced during delegation
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub artifacts: Vec<EntityId>,
    /// Error message if failed
    pub error: Option<String>,
}

// ============================================================================
// HANDOFF TYPES
// ============================================================================

/// Request to create a handoff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateHandoffRequest {
    /// Agent initiating the handoff
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    /// Agent receiving the handoff
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: EntityId,
    /// Trajectory being handed off
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    /// Current scope
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    /// Reason for handoff
    pub reason: String,
    /// Context to transfer
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_snapshot: Vec<u8>,
}

/// Handoff response with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HandoffResponse {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub handoff_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub from_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub to_agent_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: EntityId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,
    pub reason: String,
    pub status: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub accepted_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub completed_at: Option<Timestamp>,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub context_snapshot: Vec<u8>,
}

// ============================================================================
// SEARCH TYPES
// ============================================================================

/// Request to search entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SearchRequest {
    /// Search query text
    pub query: String,
    /// Entity types to search
    pub entity_types: Vec<EntityType>,
    /// Additional filters
    pub filters: Vec<FilterExpr>,
    /// Maximum number of results
    pub limit: Option<i32>,
}

/// Filter expression for search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FilterExpr {
    /// Field to filter on
    pub field: String,
    /// Operator (eq, ne, gt, lt, contains, etc.)
    pub operator: String,
    /// Value to compare against
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub value: serde_json::Value,
}

/// Search result entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SearchResult {
    /// Type of entity found
    pub entity_type: EntityType,
    /// ID of the entity
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub id: EntityId,
    /// Name or title of the entity
    pub name: String,
    /// Snippet of matching content
    pub snippet: String,
    /// Relevance score
    pub score: f32,
}

/// Response containing search results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SearchResponse {
    /// List of search results
    pub results: Vec<SearchResult>,
    /// Total count of matches
    pub total: i32,
}

// ============================================================================
// DSL TYPES
// ============================================================================

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
// BATCH TYPES
// ============================================================================

/// Operation type for batch requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum BatchOperation {
    /// Create a new entity
    Create,
    /// Update an existing entity
    Update,
    /// Delete an entity
    Delete,
}

/// A single trajectory batch operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TrajectoryBatchItem {
    /// Operation to perform
    pub operation: BatchOperation,
    /// Trajectory ID (required for update/delete)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Create request data (required for create)
    pub create: Option<CreateTrajectoryRequest>,
    /// Update request data (required for update)
    pub update: Option<UpdateTrajectoryRequest>,
}

/// Request to perform batch trajectory operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BatchTrajectoryRequest {
    /// List of operations to perform
    pub items: Vec<TrajectoryBatchItem>,
    /// Whether to stop on first error (default: false = continue on error)
    #[serde(default)]
    pub stop_on_error: bool,
}

/// Result of a single batch operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "status")]
pub enum BatchItemResult<T> {
    /// Operation succeeded
    #[serde(rename = "success")]
    Success {
        /// The resulting entity
        data: T,
    },
    /// Operation failed
    #[serde(rename = "error")]
    Error {
        /// Error message
        message: String,
        /// Error code
        code: String,
    },
}

/// Response from batch trajectory operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BatchTrajectoryResponse {
    /// Results for each operation in order
    pub results: Vec<BatchItemResult<TrajectoryResponse>>,
    /// Number of successful operations
    pub succeeded: i32,
    /// Number of failed operations
    pub failed: i32,
}

/// A single artifact batch operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ArtifactBatchItem {
    /// Operation to perform
    pub operation: BatchOperation,
    /// Artifact ID (required for update/delete)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub artifact_id: Option<EntityId>,
    /// Create request data (required for create)
    pub create: Option<CreateArtifactRequest>,
    /// Update request data (required for update)
    pub update: Option<UpdateArtifactRequest>,
}

/// Request to perform batch artifact operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BatchArtifactRequest {
    /// List of operations to perform
    pub items: Vec<ArtifactBatchItem>,
    /// Whether to stop on first error (default: false = continue on error)
    #[serde(default)]
    pub stop_on_error: bool,
}

/// Response from batch artifact operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BatchArtifactResponse {
    /// Results for each operation in order
    pub results: Vec<BatchItemResult<ArtifactResponse>>,
    /// Number of successful operations
    pub succeeded: i32,
    /// Number of failed operations
    pub failed: i32,
}

/// A single note batch operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NoteBatchItem {
    /// Operation to perform
    pub operation: BatchOperation,
    /// Note ID (required for update/delete)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub note_id: Option<EntityId>,
    /// Create request data (required for create)
    pub create: Option<CreateNoteRequest>,
    /// Update request data (required for update)
    pub update: Option<UpdateNoteRequest>,
}

/// Request to perform batch note operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BatchNoteRequest {
    /// List of operations to perform
    pub items: Vec<NoteBatchItem>,
    /// Whether to stop on first error (default: false = continue on error)
    #[serde(default)]
    pub stop_on_error: bool,
}

/// Response from batch note operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BatchNoteResponse {
    /// Results for each operation in order
    pub results: Vec<BatchItemResult<NoteResponse>>,
    /// Number of successful operations
    pub succeeded: i32,
    /// Number of failed operations
    pub failed: i32,
}

/// Deleted entity response (for batch deletes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeletedResponse {
    /// ID of the deleted entity
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub id: EntityId,
    /// Whether the entity was deleted
    pub deleted: bool,
}

// ============================================================================
// CONFIG TYPES
// ============================================================================

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

// ============================================================================
// TENANT TYPES
// ============================================================================

/// Tenant information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TenantInfo {
    /// Tenant ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: EntityId,
    /// Tenant name
    pub name: String,
    /// Email domain for auto-association (e.g., "acme.com")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// WorkOS organization ID for enterprise SSO
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workos_organization_id: Option<String>,
    /// Tenant status
    pub status: TenantStatus,
    /// When the tenant was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
}

/// Tenant status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TenantStatus {
    /// Tenant is active and operational
    Active,
    /// Tenant is suspended
    Suspended,
    /// Tenant is archived
    Archived,
}

impl fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            TenantStatus::Active => "Active",
            TenantStatus::Suspended => "Suspended",
            TenantStatus::Archived => "Archived",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for TenantStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "active" => Ok(TenantStatus::Active),
            "suspended" => Ok(TenantStatus::Suspended),
            "archived" => Ok(TenantStatus::Archived),
            _ => Err(format!("Invalid TenantStatus: {}", s)),
        }
    }
}

/// Response containing a list of tenants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListTenantsResponse {
    /// List of tenants
    pub tenants: Vec<TenantInfo>,
}

// ============================================================================
// BATTLE INTEL: EDGE TYPES
// ============================================================================

/// Request to create a new edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateEdgeRequest {
    /// Type of edge
    pub edge_type: EdgeType,
    /// Edge participants (entities involved)
    pub participants: Vec<EdgeParticipantRequest>,
    /// Optional weight/strength of relationship [0.0, 1.0]
    pub weight: Option<f32>,
    /// Optional trajectory ID for context
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Provenance information
    pub provenance: ProvenanceRequest,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Edge participant in a request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EdgeParticipantRequest {
    /// Type of the entity
    pub entity_type: EntityType,
    /// ID of the entity
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub entity_id: EntityId,
    /// Role in the relationship (e.g., "source", "target", "input", "output")
    pub role: Option<String>,
}

/// Provenance information for a request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ProvenanceRequest {
    /// Source turn sequence number
    pub source_turn: i32,
    /// How this was extracted
    pub extraction_method: ExtractionMethod,
    /// Confidence score [0.0, 1.0]
    pub confidence: Option<f32>,
}

/// Response for an edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EdgeResponse {
    /// Edge ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub edge_id: EntityId,
    /// Tenant this edge belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
    /// Type of edge
    pub edge_type: EdgeType,
    /// Edge participants
    pub participants: Vec<EdgeParticipantResponse>,
    /// Weight/strength of relationship
    pub weight: Option<f32>,
    /// Trajectory ID for context
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Provenance information
    pub provenance: ProvenanceResponse,
    /// When the edge was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// Optional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Edge participant in a response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EdgeParticipantResponse {
    /// Type of the entity
    pub entity_type: EntityType,
    /// ID of the entity
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub entity_id: EntityId,
    /// Role in the relationship
    pub role: Option<String>,
}

/// Response containing a list of edges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListEdgesResponse {
    /// List of edges
    pub edges: Vec<EdgeResponse>,
}

// ============================================================================
// BATTLE INTEL: SUMMARIZATION POLICY TYPES
// ============================================================================

/// Request to create a summarization policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateSummarizationPolicyRequest {
    /// Policy name
    pub name: String,
    /// Triggers that fire this policy
    pub triggers: Vec<SummarizationTrigger>,
    /// Source abstraction level (e.g., Raw/L0)
    pub source_level: AbstractionLevel,
    /// Target abstraction level (e.g., Summary/L1)
    pub target_level: AbstractionLevel,
    /// Maximum sources to summarize at once
    pub max_sources: i32,
    /// Whether to create SynthesizedFrom edges
    pub create_edges: bool,
    /// Optional trajectory ID to scope this policy
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Response for a summarization policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SummarizationPolicyResponse {
    /// Policy ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub policy_id: EntityId,
    /// Tenant this policy belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<EntityId>,
    /// Policy name
    pub name: String,
    /// Triggers that fire this policy
    pub triggers: Vec<SummarizationTrigger>,
    /// Source abstraction level
    pub source_level: AbstractionLevel,
    /// Target abstraction level
    pub target_level: AbstractionLevel,
    /// Maximum sources to summarize at once
    pub max_sources: i32,
    /// Whether to create SynthesizedFrom edges
    pub create_edges: bool,
    /// Trajectory ID if scoped
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// When the policy was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// Optional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Response containing a list of summarization policies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListSummarizationPoliciesResponse {
    /// List of policies
    pub policies: Vec<SummarizationPolicyResponse>,
}
