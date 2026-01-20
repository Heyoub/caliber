//! OpenAI embedding provider implementation

use super::client::OpenAIClient;
use super::types::{EmbeddingRequest, EmbeddingResponse};
use crate::{EmbeddingProvider, ProviderAdapter, ProviderCapability};
use async_trait::async_trait;
use caliber_core::{CaliberResult, EmbeddingVector};

/// OpenAI embedding provider using text-embedding-3-small or custom model.
pub struct OpenAIEmbeddingProvider {
    client: OpenAIClient,
    model: String,
    dimensions: i32,
}

impl OpenAIEmbeddingProvider {
    /// Create a new OpenAI embedding provider.
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key
    /// * `model` - Model name (e.g., "text-embedding-3-small", "text-embedding-ada-002")
    /// * `dimensions` - Embedding dimensions (1536 for ada-002, 512/1536 for 3-small)
    pub fn new(api_key: impl Into<String>, model: impl Into<String>, dimensions: i32) -> Self {
        Self {
            client: OpenAIClient::new(api_key, 60), // 60 requests per minute
            model: model.into(),
            dimensions,
        }
    }

    /// Create provider with default text-embedding-3-small model.
    pub fn with_default_model(api_key: impl Into<String>) -> Self {
        Self::new(api_key, "text-embedding-3-small", 1536)
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector> {
        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: vec![text.to_string()],
            dimensions: Some(self.dimensions),
        };

        let response: EmbeddingResponse = self.client.request("embeddings", request).await?;

        let embedding_data = response.data.into_iter().next().ok_or_else(|| {
            caliber_core::CaliberError::Llm(caliber_core::LlmError::ProviderError {
                provider: "openai".to_string(),
                message: "No embedding data in response".to_string(),
            })
        })?;

        Ok(EmbeddingVector::new(embedding_data.embedding, self.model.clone()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>> {
        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: texts.iter().map(|s| s.to_string()).collect(),
            dimensions: Some(self.dimensions),
        };

        let response: EmbeddingResponse = self.client.request("embeddings", request).await?;

        let mut embeddings: Vec<_> = response
            .data
            .into_iter()
            .map(|data| EmbeddingVector::new(data.embedding, self.model.clone()))
            .collect();

        // Ensure ordering matches input
        if embeddings.len() != texts.len() {
            return Err(caliber_core::CaliberError::Llm(caliber_core::LlmError::ProviderError {
                provider: "openai".to_string(),
                message: format!(
                    "Expected {} embeddings but got {}",
                    texts.len(),
                    embeddings.len()
                ),
            }));
        }

        Ok(embeddings)
    }

    fn dimensions(&self) -> i32 {
        self.dimensions
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

impl std::fmt::Debug for OpenAIEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIEmbeddingProvider")
            .field("model", &self.model)
            .field("dimensions", &self.dimensions)
            .finish()
    }
}
