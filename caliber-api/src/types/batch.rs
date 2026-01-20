//! Batch operation types

use caliber_core::EntityId;
use serde::{Deserialize, Serialize};

use super::{
    ArtifactResponse, CreateArtifactRequest, CreateNoteRequest, CreateTrajectoryRequest,
    NoteResponse, TrajectoryResponse, UpdateArtifactRequest, UpdateNoteRequest,
    UpdateTrajectoryRequest,
};

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
