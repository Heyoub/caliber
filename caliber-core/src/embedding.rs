//! Embedding vector operations

use crate::{CaliberError, CaliberResult, VectorError};
use serde::{Deserialize, Serialize};

/// Embedding vector with dynamic dimensions.
/// Supports any embedding model dimension (e.g., 384, 768, 1536, 3072).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EmbeddingVector {
    /// The embedding data as a vector of f32 values.
    pub data: Vec<f32>,
    /// Identifier of the model that produced this embedding.
    pub model_id: String,
    /// Number of dimensions (must match data.len()).
    pub dimensions: i32,
}

impl EmbeddingVector {
    /// Create a new embedding vector.
    pub fn new(data: Vec<f32>, model_id: String) -> Self {
        let dimensions = data.len() as i32;
        Self {
            data,
            model_id,
            dimensions,
        }
    }

    /// Compute cosine similarity between two embedding vectors.
    pub fn cosine_similarity(&self, other: &EmbeddingVector) -> CaliberResult<f32> {
        if self.dimensions != other.dimensions {
            return Err(CaliberError::Vector(VectorError::DimensionMismatch {
                expected: self.dimensions,
                got: other.dimensions,
            }));
        }

        let mut dot_product = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;

        for (a, b) in self.data.iter().zip(other.data.iter()) {
            dot_product += a * b;
            norm_a += a * a;
            norm_b += b * b;
        }

        let norm_a = norm_a.sqrt();
        let norm_b = norm_b.sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        Ok(dot_product / (norm_a * norm_b))
    }

    /// Check if this vector has valid dimensions.
    pub fn is_valid(&self) -> bool {
        self.dimensions > 0 && self.data.len() == self.dimensions as usize
    }
}
