//! Ollama API request and response types

use serde::{Deserialize, Serialize};

// ============================================================================
// EMBEDDING TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
}

// ============================================================================
// MODEL TYPES
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub modified_at: String,
    pub size: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListModelsResponse {
    pub models: Vec<ModelInfo>,
}
