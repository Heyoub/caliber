//! OpenAI summarization provider implementation

use super::client::OpenAIClient;
use super::types::{CompletionRequest, CompletionResponse, Message};
use crate::providers::invalid_response;
use crate::{ExtractedArtifact, SummarizationProvider, SummarizeConfig, SummarizeStyle};
use async_trait::async_trait;
use caliber_core::{ArtifactType, CaliberResult};

/// OpenAI summarization provider using GPT models.
pub struct OpenAISummarizationProvider {
    client: OpenAIClient,
    model: String,
}

impl OpenAISummarizationProvider {
    /// Create a new OpenAI summarization provider.
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key
    /// * `model` - Model name (e.g., "gpt-4o-mini", "gpt-4o")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: OpenAIClient::new(api_key, 60),
            model: model.into(),
        }
    }

    /// Create provider with default gpt-4o-mini model.
    pub fn with_default_model(api_key: impl Into<String>) -> Self {
        Self::new(api_key, "gpt-4o-mini")
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
}

#[async_trait]
impl SummarizationProvider for OpenAISummarizationProvider {
    async fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String> {
        let request = CompletionRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: self.build_system_prompt(config.style),
                },
                Message {
                    role: "user".to_string(),
                    content: format!("Summarize the following content:\n\n{}", content),
                },
            ],
            max_tokens: Some(config.max_tokens),
            temperature: Some(0.3), // Lower temperature for more focused summaries
        };

        let response: CompletionResponse = self.client.request("chat/completions", request).await?;

        let summary = response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| invalid_response("openai", "No completion in response"))?;

        Ok(summary)
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

        let request = CompletionRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: format!(
                        "You are an artifact extraction assistant. \
                         Extract the following artifact types from the content: {}. \
                         For each artifact found, provide: type, content, and confidence (0.0-1.0). \
                         Format as JSON array: [{{\"type\": \"...\", \"content\": \"...\", \"confidence\": 0.8}}]",
                        types_list
                    ),
                },
                Message {
                    role: "user".to_string(),
                    content: format!("Extract artifacts from:\n\n{}", content),
                },
            ],
            max_tokens: Some(1000),
            temperature: Some(0.2),
        };

        let response: CompletionResponse = self.client.request("chat/completions", request).await?;

        let response_text = response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| invalid_response("openai", "No completion in response"))?;

        // Try to parse JSON response
        #[derive(serde::Deserialize)]
        struct ArtifactJson {
            r#type: String,
            content: String,
            confidence: f32,
        }

        let artifacts_json: Vec<ArtifactJson> = serde_json::from_str(&response_text)
            .unwrap_or_else(|_| Vec::new());

        let artifacts = artifacts_json
            .into_iter()
            .filter_map(|a| {
                // Map string type to ArtifactType enum
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
        let request = CompletionRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a contradiction detection assistant. \
                              Analyze two statements and determine if they contradict each other. \
                              Respond with ONLY 'yes' or 'no'."
                        .to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: format!("Statement A: {}\n\nStatement B: {}\n\nDo these contradict?", a, b),
                },
            ],
            max_tokens: Some(10),
            temperature: Some(0.0), // Deterministic
        };

        let response: CompletionResponse = self.client.request("chat/completions", request).await?;

        let answer = response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content.to_lowercase())
            .ok_or_else(|| invalid_response("openai", "No completion in response"))?;

        Ok(answer.contains("yes"))
    }
}

impl std::fmt::Debug for OpenAISummarizationProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAISummarizationProvider")
            .field("model", &self.model)
            .finish()
    }
}
