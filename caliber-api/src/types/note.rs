//! Note-related API types

use caliber_core::{ArtifactId, NoteId, NoteType, TenantId, Timestamp, TrajectoryId, TTL};
use serde::{Deserialize, Serialize};

use super::{EmbeddingResponse, Linkable, Links, LINK_REGISTRY};

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
    pub source_trajectory_ids: Vec<TrajectoryId>,
    /// Source artifacts
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_artifact_ids: Vec<ArtifactId>,
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
    pub source_trajectory_id: Option<TrajectoryId>,
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
    pub note_id: NoteId,
    /// Tenant this note belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    pub note_type: NoteType,
    pub title: String,
    pub content: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub content_hash: [u8; 32],
    pub embedding: Option<EmbeddingResponse>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_trajectory_ids: Vec<TrajectoryId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub source_artifact_ids: Vec<ArtifactId>,
    pub ttl: TTL,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub updated_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub accessed_at: Timestamp,
    pub access_count: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub superseded_by: Option<NoteId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,

    /// HATEOAS links for available actions.
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub links: Option<Links>,
}

impl Linkable for NoteResponse {
    const ENTITY_TYPE: &'static str = "note";

    fn entity_id(&self) -> String {
        self.note_id.to_string()
    }

    fn check_condition(&self, condition: &str) -> bool {
        match condition {
            "has_superseded" => self.superseded_by.is_some(),
            _ => true,
        }
    }

    fn relation_id(&self, relation: &str) -> Option<String> {
        match relation {
            "superseded_by" => self.superseded_by.map(|id| id.to_string()),
            _ => None,
        }
    }
}

impl NoteResponse {
    /// Add HATEOAS links from the registry.
    pub fn linked(mut self) -> Self {
        self.links = Some(LINK_REGISTRY.generate(&self));
        self
    }
}
