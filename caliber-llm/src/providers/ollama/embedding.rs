//! Ollama embedding provider implementation (local models)

use super::types::{EmbeddingRequest, EmbeddingResponse};
use crate::providers::{invalid_response, request_failed};
use crate::EmbeddingProvider;
use async_trait::async_trait;
use caliber_core::{CaliberResult, EmbeddingVector};
use reqwest::Client;

/// Ollama embedding provider for local LLM models.
pub struct OllamaEmbeddingProvider {
    client: Client,
    base_url: String,
    model: String,
    dimensions: i32,
}

impl OllamaEmbeddingProvider {
    /// Create a new Ollama embedding provider.
    ///
    /// # Arguments
    /// * `base_url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model` - Model name (e.g., "llama2", "mistral", "nomic-embed-text")
    /// * `dimensions` - Embedding dimensions for the model
    pub fn new(base_url: impl Into<String>, model: impl Into<String>, dimensions: i32) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            model: model.into(),
            dimensions,
        }
    }

    /// Create provider with default nomic-embed-text model.
    pub fn with_default_model(base_url: impl Into<String>) -> Self {
        Self::new(base_url, "nomic-embed-text", 768)
    }

    /// Check if the model is available locally.
    pub async fn check_model_available(&self) -> CaliberResult<bool> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await.map_err(|e| {
            request_failed("ollama", 0, format!("Failed to connect to Ollama: {}", e))
        })?;

        if !response.status().is_success() {
            return Ok(false);
        }

        #[derive(serde::Deserialize)]
        struct ListResponse {
            models: Vec<ModelEntry>,
        }
        #[derive(serde::Deserialize)]
        struct ModelEntry {
            name: String,
        }

        let list: ListResponse = response.json().await.map_err(|e| {
            invalid_response("ollama", format!("Failed to parse models list: {}", e))
        })?;

        Ok(list.models.iter().any(|m| m.name.contains(&self.model)))
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector> {
        let request = EmbeddingRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let url = format!("{}/api/embeddings", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| request_failed("ollama", 0, format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(request_failed(
                "ollama",
                status.as_u16() as i32,
                error_text,
            ));
        }

        let embedding_response: EmbeddingResponse = response.json().await.map_err(|e| {
            invalid_response("ollama", format!("Failed to parse response: {}", e))
        })?;

        Ok(EmbeddingVector::new(
            embedding_response.embedding,
            self.model.clone(),
        ))
    }

    async fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>> {
        // Ollama doesn't have batch API, so we do sequential requests
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
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

impl std::fmt::Debug for OllamaEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OllamaEmbeddingProvider")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("dimensions", &self.dimensions)
            .finish()
    }
}
