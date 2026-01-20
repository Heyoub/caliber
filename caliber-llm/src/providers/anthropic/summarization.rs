//! Anthropic (Claude) summarization provider implementation

use super::client::AnthropicClient;
use super::types::{ContentBlock, Message, MessageRequest, MessageResponse};
use crate::{ExtractedArtifact, SummarizationProvider, SummarizeConfig, SummarizeStyle};
use async_trait::async_trait;
use caliber_core::{ArtifactType, CaliberResult};

/// Anthropic summarization provider using Claude models.
pub struct AnthropicSummarizationProvider {
    client: AnthropicClient,
    model: String,
}

impl AnthropicSummarizationProvider {
    /// Create a new Anthropic summarization provider.
    ///
    /// # Arguments
    /// * `api_key` - Anthropic API key
    /// * `model` - Model name (e.g., "claude-3-5-sonnet-20241022", "claude-3-haiku-20240307")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: AnthropicClient::new(api_key, 50),
            model: model.into(),
        }
    }

    /// Create provider with default Claude 3.5 Sonnet model.
    pub fn with_default_model(api_key: impl Into<String>) -> Self {
        Self::new(api_key, "claude-3-5-sonnet-20241022")
    }

    /// Build system prompt based on summarization style.
    fn build_system_prompt(&self, style: SummarizeStyle) -> String {
        match style {
            SummarizeStyle::Brief => {
                "You are a helpful assistant that creates concise summaries. \
                 Focus on the key points and main ideas. \
                 Keep your response short and to the point."
                    .to_string()
            }
            SummarizeStyle::Detailed => {
                "You are a helpful assistant that creates detailed summaries. \
                 Include important context, key points, and supporting details. \
                 Organize the information clearly."
                    .to_string()
            }
            SummarizeStyle::Structured => {
                "You are a helpful assistant that creates structured summaries. \
                 Use bullet points or numbered lists to organize information. \
                 Include sections like: Overview, Key Points, Details, Conclusion."
                    .to_string()
            }
        }
    }

    /// Extract text from content blocks.
    fn extract_text(content: Vec<ContentBlock>) -> String {
        content
            .into_iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[async_trait]
impl SummarizationProvider for AnthropicSummarizationProvider {
    async fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String> {
        let request = MessageRequest {
            model: self.model.clone(),
            system: Some(self.build_system_prompt(config.style)),
            messages: vec![Message {
                role: "user".to_string(),
                content: format!("Summarize the following content:\n\n{}", content),
            }],
            max_tokens: config.max_tokens,
            temperature: Some(0.3), // Lower temperature for focused summaries
        };

        let response: MessageResponse = self.client.request("messages", request).await?;

        Ok(Self::extract_text(response.content))
    }

    async fn extract_artifacts(
        &self,
        content: &str,
        types: &[ArtifactType],
    ) -> CaliberResult<Vec<ExtractedArtifact>> {
        let types_list = types
            .iter()
            .map(|t| format!("{:?}", t))
            .collect::<Vec<_>>()
            .join(", ");

        let request = MessageRequest {
            model: self.model.clone(),
            system: Some(format!(
                "You are an artifact extraction assistant. \
                 Extract the following artifact types from the content: {}. \
                 For each artifact found, provide: type, content, and confidence (0.0-1.0). \
                 Format as JSON array: [{{\"type\": \"...\", \"content\": \"...\", \"confidence\": 0.8}}]",
                types_list
            )),
            messages: vec![Message {
                role: "user".to_string(),
                content: format!("Extract artifacts from:\n\n{}", content),
            }],
            max_tokens: 1000,
            temperature: Some(0.2),
        };

        let response: MessageResponse = self.client.request("messages", request).await?;
        let response_text = Self::extract_text(response.content);

        // Try to parse JSON response
        #[derive(serde::Deserialize)]
        struct ArtifactJson {
            r#type: String,
            content: String,
            confidence: f32,
        }

        let artifacts_json: Vec<ArtifactJson> =
            serde_json::from_str(&response_text).unwrap_or_else(|_| Vec::new());

        let artifacts = artifacts_json
            .into_iter()
            .filter_map(|a| {
                let artifact_type = match a.r#type.to_lowercase().as_str() {
                    "code" => ArtifactType::Code,
                    "document" => ArtifactType::Document,
                    "data" => ArtifactType::Data,
                    "model" => ArtifactType::Model,
                    "config" => ArtifactType::Config,
                    _ => return None,
                };

                Some(ExtractedArtifact {
                    artifact_type,
                    content: a.content,
                    confidence: a.confidence.clamp(0.0, 1.0),
                })
            })
            .collect();

        Ok(artifacts)
    }

    async fn detect_contradiction(&self, a: &str, b: &str) -> CaliberResult<bool> {
        let request = MessageRequest {
            model: self.model.clone(),
            system: Some(
                "You are a contradiction detection assistant. \
                 Analyze two statements and determine if they contradict each other. \
                 Respond with ONLY 'yes' or 'no'."
                    .to_string(),
            ),
            messages: vec![Message {
                role: "user".to_string(),
                content: format!(
                    "Statement A: {}\n\nStatement B: {}\n\nDo these contradict?",
                    a, b
                ),
            }],
            max_tokens: 10,
            temperature: Some(0.0), // Deterministic
        };

        let response: MessageResponse = self.client.request("messages", request).await?;
        let answer = Self::extract_text(response.content).to_lowercase();

        Ok(answer.contains("yes"))
    }
}

impl std::fmt::Debug for AnthropicSummarizationProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicSummarizationProvider")
            .field("model", &self.model)
            .finish()
    }
}
