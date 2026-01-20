//! OpenAI HTTP client with rate limiting

use super::types::ApiError;
use crate::providers::{invalid_response, rate_limited, request_failed};
use caliber_core::CaliberResult;
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
    start_time: Instant,
}

impl OpenAIClient {
    /// Create a new OpenAI client.
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key
    /// * `requests_per_minute` - Maximum requests per minute (default: 60)
    pub fn new(api_key: impl Into<String>, requests_per_minute: u32) -> Self {
        let rpm = requests_per_minute.max(1);
        let permits = rpm as usize;
        let min_interval_ms = (60_000 / rpm as u64).max(10);

        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            rate_limiter: Arc::new(Semaphore::new(permits)),
            last_request: Arc::new(AtomicU64::new(0)),
            min_request_interval_ms: min_interval_ms,
            start_time: Instant::now(),
        }
    }

    /// Make an API request with automatic rate limiting.
    pub async fn request<Req: Serialize, Res: DeserializeOwned>(
        &self,
        endpoint: &str,
        body: Req,
    ) -> CaliberResult<Res> {
        // Rate limiting: acquire permit
        let _permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| request_failed("openai", 0, format!("Rate limiter error: {}", e)))?;

        // Enforce minimum interval between requests
        let now_ms = self.start_time.elapsed().as_millis() as u64;
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
            .map_err(|e| request_failed("openai", 0, format!("HTTP request failed: {}", e)))?;

        // Handle response
        let status = response.status();
        let retry_after_ms = parse_retry_after_ms(response.headers()).unwrap_or(0);

        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| invalid_response("openai", format!("Failed to parse response: {}", e)))
        } else {
            // Parse error response
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

            let error_msg = if let Ok(api_error) = serde_json::from_str::<ApiError>(&error_text) {
                api_error.error.message
            } else {
                error_text
            };

            Err(match status {
                StatusCode::TOO_MANY_REQUESTS => rate_limited("openai", retry_after_ms),
                _ => request_failed("openai", status.as_u16() as i32, error_msg),
            })
        }
    }
}

fn parse_retry_after_ms(headers: &reqwest::header::HeaderMap) -> Option<i64> {
    headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<f64>().ok())
        .map(|seconds| (seconds * 1000.0) as i64)
}

impl std::fmt::Debug for OpenAIClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIClient")
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}
