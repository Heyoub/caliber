//! Artifact-related API types

use caliber_core::{ArtifactId, ArtifactType, ExtractionMethod, ScopeId, TenantId, Timestamp, TrajectoryId, TTL};
use serde::{Deserialize, Serialize};

use super::{Linkable, Links, LINK_REGISTRY};

/// Request to create a new artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateArtifactRequest {
    /// Trajectory this artifact belongs to
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    /// Scope this artifact was created in
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
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
    pub trajectory_id: Option<TrajectoryId>,
    /// Filter by scope
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<ScopeId>,
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
    pub artifact_id: ArtifactId,
    /// Tenant this artifact belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: ScopeId,
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
    pub superseded_by: Option<ArtifactId>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,

    /// HATEOAS links for available actions.
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub links: Option<Links>,
}

impl Linkable for ArtifactResponse {
    const ENTITY_TYPE: &'static str = "artifact";

    fn link_id(&self) -> String {
        self.artifact_id.to_string()
    }

    fn check_condition(&self, condition: &str) -> bool {
        match condition {
            "has_superseded" => self.superseded_by.is_some(),
            _ => true,
        }
    }

    fn relation_id(&self, relation: &str) -> Option<String> {
        match relation {
            "trajectory_id" => Some(self.trajectory_id.to_string()),
            "scope_id" => Some(self.scope_id.to_string()),
            "superseded_by" => self.superseded_by.map(|id| id.to_string()),
            _ => None,
        }
    }
}

impl ArtifactResponse {
    /// Add HATEOAS links from the registry.
    pub fn linked(mut self) -> Self {
        self.links = Some(LINK_REGISTRY.generate(&self));
        self
    }
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
