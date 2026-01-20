//! Ollama provider implementation (local models)
//!
//! This module provides Ollama-based embedding capabilities for local LLMs.

pub mod embedding;
pub mod types;

pub use embedding::OllamaEmbeddingProvider;
