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
