//! OpenAI HTTP client with rate limiting

use super::types::ApiError;
use caliber_core::{CaliberError, CaliberResult, LlmError};
use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// OpenAI API client with rate limiting.
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    base_url: String,
    rate_limiter: Arc<Semaphore>,
    last_request: Arc<AtomicU64>,
    min_request_interval_ms: u64,
}

impl OpenAIClient {
    /// Create a new OpenAI client.
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key
    /// * `requests_per_minute` - Maximum requests per minute (default: 60)
    pub fn new(api_key: impl Into<String>, requests_per_minute: u32) -> Self {
        let permits = (requests_per_minute as usize).max(1);
        let min_interval_ms = (60_000 / requests_per_minute as u64).max(10);

        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            rate_limiter: Arc::new(Semaphore::new(permits)),
            last_request: Arc::new(AtomicU64::new(0)),
            min_request_interval_ms: min_interval_ms,
        }
    }

    /// Make an API request with automatic rate limiting.
    pub async fn request<Req: Serialize, Res: DeserializeOwned>(
        &self,
        endpoint: &str,
        body: Req,
    ) -> CaliberResult<Res> {
        // Rate limiting: acquire permit
        let _permit = self.rate_limiter.acquire().await.map_err(|e| {
            CaliberError::Llm(LlmError::ProviderError {
                provider: "openai".to_string(),
                message: format!("Rate limiter error: {}", e),
            })
        })?;

        // Enforce minimum interval between requests
        let now_ms = Instant::now().elapsed().as_millis() as u64;
        let last_ms = self.last_request.load(Ordering::Relaxed);
        let elapsed = now_ms.saturating_sub(last_ms);

        if elapsed < self.min_request_interval_ms {
            let wait_ms = self.min_request_interval_ms - elapsed;
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        }

        self.last_request.store(now_ms, Ordering::Relaxed);

        // Make HTTP request
        let url = format!("{}/{}", self.base_url, endpoint);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                CaliberError::Llm(LlmError::ProviderError {
                    provider: "openai".to_string(),
                    message: format!("HTTP request failed: {}", e),
                })
            })?;

        // Handle response
        let status = response.status();

        if status.is_success() {
            response.json().await.map_err(|e| {
                CaliberError::Llm(LlmError::ProviderError {
                    provider: "openai".to_string(),
                    message: format!("Failed to parse response: {}", e),
                })
            })
        } else {
            // Parse error response
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

            let error_msg = if let Ok(api_error) = serde_json::from_str::<ApiError>(&error_text) {
                api_error.error.message
            } else {
                error_text
            };

            Err(match status {
                StatusCode::TOO_MANY_REQUESTS => CaliberError::Llm(LlmError::RateLimited),
                StatusCode::UNAUTHORIZED => CaliberError::Llm(LlmError::InvalidApiKey {
                    provider: "openai".to_string(),
                }),
                _ => CaliberError::Llm(LlmError::ProviderError {
                    provider: "openai".to_string(),
                    message: error_msg,
                }),
            })
        }
    }
}

impl std::fmt::Debug for OpenAIClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIClient")
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}
