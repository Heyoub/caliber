//! Battle Intel: Edge-related API types

use caliber_core::{EdgeId, EdgeType, EntityType, ExtractionMethod, TenantId, Timestamp, TrajectoryId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ProvenanceResponse;

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
    pub trajectory_id: Option<TrajectoryId>,
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
    /// ID of the entity (can reference ANY entity type)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub entity_id: Uuid,
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
    pub edge_id: EdgeId,
    /// Tenant this edge belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<TenantId>,
    /// Type of edge
    pub edge_type: EdgeType,
    /// Edge participants
    pub participants: Vec<EdgeParticipantResponse>,
    /// Weight/strength of relationship
    pub weight: Option<f32>,
    /// Trajectory ID for context
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<TrajectoryId>,
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
    /// ID of the entity (can reference ANY entity type)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub entity_id: Uuid,
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
