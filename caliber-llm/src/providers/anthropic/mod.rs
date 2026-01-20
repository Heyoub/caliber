//! Anthropic (Claude) provider implementation
//!
//! This module provides Claude-based summarization capabilities.

pub mod client;
pub mod summarization;
pub mod types;

pub use client::AnthropicClient;
pub use summarization::AnthropicSummarizationProvider;
