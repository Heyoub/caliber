//! Search-related API types

use caliber_core::{EntityId, EntityType};
use serde::{Deserialize, Serialize};

// Re-export FilterExpr from core for unified filtering
pub use caliber_core::filter::FilterExpr;

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
