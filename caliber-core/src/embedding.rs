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

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CaliberError, VectorError};

    #[test]
    fn test_new_sets_dimensions() {
        let data = vec![0.0, 1.0, 0.5];
        let vec = EmbeddingVector::new(data.clone(), "model".to_string());
        assert_eq!(vec.dimensions, data.len() as i32);
        assert_eq!(vec.data, data);
        assert_eq!(vec.model_id, "model");
    }

    #[test]
    fn test_is_valid_checks_dimensions_and_length() {
        let valid = EmbeddingVector {
            data: vec![0.0, 1.0],
            model_id: "m".to_string(),
            dimensions: 2,
        };
        assert!(valid.is_valid());

        let invalid_len = EmbeddingVector {
            data: vec![0.0, 1.0],
            model_id: "m".to_string(),
            dimensions: 3,
        };
        assert!(!invalid_len.is_valid());

        let invalid_dim = EmbeddingVector {
            data: vec![0.0, 1.0],
            model_id: "m".to_string(),
            dimensions: 0,
        };
        assert!(!invalid_dim.is_valid());
    }

    #[test]
    fn test_empty_vector_is_invalid() {
        let vec = EmbeddingVector::new(vec![], "model".to_string());
        assert_eq!(vec.dimensions, 0);
        assert!(!vec.is_valid());
    }

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let a = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "model".to_string());
        let b = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "model".to_string());
        let sim = a.cosine_similarity(&b).unwrap();
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let a = EmbeddingVector::new(vec![1.0, 0.0], "model".to_string());
        let b = EmbeddingVector::new(vec![0.0, 1.0], "model".to_string());
        let sim = a.cosine_similarity(&b).unwrap();
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_zero_vector_returns_zero() {
        let a = EmbeddingVector::new(vec![0.0, 0.0], "model".to_string());
        let b = EmbeddingVector::new(vec![1.0, 0.0], "model".to_string());
        let sim = a.cosine_similarity(&b).unwrap();
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_dimension_mismatch() {
        let a = EmbeddingVector::new(vec![1.0, 0.0], "model".to_string());
        let b = EmbeddingVector::new(vec![1.0, 0.0, 0.0], "model".to_string());
        let err = a.cosine_similarity(&b).unwrap_err();
        assert!(matches!(
            err,
            CaliberError::Vector(VectorError::DimensionMismatch { expected: 2, got: 3 })
        ));
    }

    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let a = EmbeddingVector::new(vec![1.0, 0.0], "model".to_string());
        let b = EmbeddingVector::new(vec![-1.0, 0.0], "model".to_string());
        let sim = a.cosine_similarity(&b).unwrap();
        assert!((sim + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_scaled_vectors() {
        let a = EmbeddingVector::new(vec![1.0, 2.0, 3.0], "model".to_string());
        let b = EmbeddingVector::new(vec![2.0, 4.0, 6.0], "model".to_string());
        let sim = a.cosine_similarity(&b).unwrap();
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_is_symmetric() {
        let a = EmbeddingVector::new(vec![1.0, 2.0], "model".to_string());
        let b = EmbeddingVector::new(vec![3.0, 4.0], "model".to_string());
        let ab = a.cosine_similarity(&b).unwrap();
        let ba = b.cosine_similarity(&a).unwrap();
        assert!((ab - ba).abs() < 1e-6);
    }

    #[test]
    fn test_is_valid_negative_dimensions() {
        let invalid = EmbeddingVector {
            data: vec![0.0, 1.0],
            model_id: "m".to_string(),
            dimensions: -1,
        };
        assert!(!invalid.is_valid());
    }
}
