//! LLM provider implementations
//!
//! This module contains concrete implementations of the EmbeddingProvider
//! and SummarizationProvider traits for various LLM services.

pub mod anthropic;
pub mod ollama;
pub mod openai;

pub use anthropic::{AnthropicClient, AnthropicSummarizationProvider};
pub use ollama::OllamaEmbeddingProvider;
pub use openai::{OpenAIClient, OpenAIEmbeddingProvider, OpenAISummarizationProvider};

use caliber_core::{CaliberError, LlmError};

pub(crate) fn request_failed(provider: &str, status: i32, message: impl Into<String>) -> CaliberError {
    CaliberError::Llm(LlmError::RequestFailed {
        provider: provider.to_string(),
        status,
        message: message.into(),
    })
}

pub(crate) fn invalid_response(provider: &str, reason: impl Into<String>) -> CaliberError {
    CaliberError::Llm(LlmError::InvalidResponse {
        provider: provider.to_string(),
        reason: reason.into(),
    })
}

pub(crate) fn rate_limited(provider: &str, retry_after_ms: i64) -> CaliberError {
    CaliberError::Llm(LlmError::RateLimited {
        provider: provider.to_string(),
        retry_after_ms,
    })
}
