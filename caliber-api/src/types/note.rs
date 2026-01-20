//! Note-related API types

use caliber_core::{EntityId, NoteType, Timestamp, TTL};
use serde::{Deserialize, Serialize};

use super::EmbeddingResponse;

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
