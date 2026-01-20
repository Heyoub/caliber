//! OpenAI provider implementation
//!
//! This module provides OpenAI-based embedding and summarization capabilities.

pub mod client;
pub mod embedding;
pub mod summarization;
pub mod types;

pub use client::OpenAIClient;
pub use embedding::OpenAIEmbeddingProvider;
pub use summarization::OpenAISummarizationProvider;
