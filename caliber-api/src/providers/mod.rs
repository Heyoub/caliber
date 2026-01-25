//! LLM Provider Orchestration
//!
//! Runtime orchestration for LLM providers including:
//! - `ProviderRegistry` - manages multiple providers with routing strategies
//! - `CircuitBreaker` - failure detection and recovery
//! - `CostTracker` - token usage tracking
//! - `ListenerChain` - event dispatch for observability
//!
//! The pure traits (`EmbeddingProvider`, `SummarizationProvider`) live in `caliber_core::llm`.
//! Real provider implementations (OpenAI, Anthropic, Ollama) are in submodules.

use async_trait::async_trait;
use caliber_core::{
    CaliberError, CaliberResult, EmbeddingVector, HealthStatus, LlmError,
    ProviderCapability, CircuitState, RoutingStrategy, SummarizeConfig,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock as TokioRwLock;
use uuid::Uuid;

// ============================================================================
// ECHO/PING DISCOVERY
// ============================================================================

/// Echo request for provider discovery.
#[derive(Debug, Clone)]
pub struct EchoRequest {
    pub capabilities: Vec<ProviderCapability>,
    pub request_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

impl EchoRequest {
    pub fn new(capabilities: Vec<ProviderCapability>) -> Self {
        Self {
            capabilities,
            request_id: Uuid::now_v7(),
            timestamp: Utc::now(),
        }
    }
}

/// Ping response from a provider.
#[derive(Debug, Clone)]
pub struct PingResponse {
    pub provider_id: String,
    pub capabilities: Vec<ProviderCapability>,
    pub latency_ms: u64,
    pub health: HealthStatus,
    pub metadata: HashMap<String, String>,
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Request for embedding operation.
#[derive(Debug, Clone)]
pub struct EmbedRequest {
    pub text: String,
    pub request_id: Uuid,
}

/// Response from embedding operation.
#[derive(Debug, Clone)]
pub struct EmbedResponse {
    pub embedding: EmbeddingVector,
    pub request_id: Uuid,
    pub latency_ms: u64,
}

/// Request for summarization operation.
#[derive(Debug, Clone)]
pub struct SummarizeRequest {
    pub content: String,
    pub config: SummarizeConfig,
    pub request_id: Uuid,
}

/// Response from summarization operation.
#[derive(Debug, Clone)]
pub struct SummarizeResponse {
    pub summary: String,
    pub request_id: Uuid,
    pub latency_ms: u64,
}

// ============================================================================
// PROVIDER ADAPTER TRAIT
// ============================================================================

/// Adapter trait for providers with Echo/Ping support.
///
/// This extends the pure traits from caliber-core with network discovery capabilities.
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// Get the unique identifier for this provider.
    fn provider_id(&self) -> &str;

    /// Get the capabilities this provider offers.
    fn capabilities(&self) -> &[ProviderCapability];

    /// Ping the provider to check health and measure latency.
    async fn ping(&self) -> CaliberResult<PingResponse>;

    /// Perform embedding operation.
    async fn embed(&self, request: EmbedRequest) -> CaliberResult<EmbedResponse>;

    /// Perform summarization operation.
    async fn summarize(&self, request: SummarizeRequest) -> CaliberResult<SummarizeResponse>;
}

// ============================================================================
// EVENT LISTENER
// ============================================================================

/// Event emitted when a request is made.
#[derive(Debug, Clone)]
pub struct RequestEvent {
    pub request_id: Uuid,
    pub provider_id: String,
    pub operation: String,
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a response is received.
#[derive(Debug, Clone)]
pub struct ResponseEvent {
    pub request_id: Uuid,
    pub provider_id: String,
    pub operation: String,
    pub latency_ms: u64,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when an error occurs.
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    pub request_id: Uuid,
    pub provider_id: String,
    pub operation: String,
    pub error_message: String,
    pub timestamp: DateTime<Utc>,
}

/// Async trait for event listeners.
#[async_trait]
pub trait EventListener: Send + Sync {
    async fn on_request(&self, event: RequestEvent) -> CaliberResult<()>;
    async fn on_response(&self, event: ResponseEvent) -> CaliberResult<()>;
    async fn on_error(&self, event: ErrorEvent) -> CaliberResult<()>;
}

/// Chain of event listeners.
pub struct ListenerChain {
    listeners: Vec<Arc<dyn EventListener>>,
}

impl ListenerChain {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    pub fn add(&mut self, listener: Arc<dyn EventListener>) {
        self.listeners.push(listener);
    }

    pub async fn emit_request(&self, event: RequestEvent) {
        for listener in &self.listeners {
            let _ = listener.on_request(event.clone()).await;
        }
    }

    pub async fn emit_response(&self, event: ResponseEvent) {
        for listener in &self.listeners {
            let _ = listener.on_response(event.clone()).await;
        }
    }

    pub async fn emit_error(&self, event: ErrorEvent) {
        for listener in &self.listeners {
            let _ = listener.on_error(event.clone()).await;
        }
    }
}

impl Default for ListenerChain {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CIRCUIT BREAKER
// ============================================================================

/// Configuration for circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(30),
        }
    }
}

/// Circuit breaker for provider health management.
pub struct CircuitBreaker {
    state: AtomicU8,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure: RwLock<Option<Instant>>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure: RwLock::new(None),
            config,
        }
    }

    pub fn state(&self) -> CircuitState {
        CircuitState::from(self.state.load(Ordering::SeqCst))
    }

    pub fn is_allowed(&self) -> bool {
        match self.state() {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has passed
                if let Ok(guard) = self.last_failure.read() {
                    if let Some(last) = *guard {
                        if last.elapsed() > self.config.timeout {
                            // Transition to half-open
                            self.state
                                .store(CircuitState::HalfOpen as u8, Ordering::SeqCst);
                            return true;
                        }
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::SeqCst);

        if self.state() == CircuitState::HalfOpen {
            let count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
            if count >= self.config.success_threshold {
                self.state.store(CircuitState::Closed as u8, Ordering::SeqCst);
                self.success_count.store(0, Ordering::SeqCst);
            }
        }
    }

    pub fn record_failure(&self) {
        self.success_count.store(0, Ordering::SeqCst);

        if let Ok(mut guard) = self.last_failure.write() {
            *guard = Some(Instant::now());
        }

        let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        if count >= self.config.failure_threshold {
            self.state.store(CircuitState::Open as u8, Ordering::SeqCst);
        }
    }

    pub fn reset(&self) {
        self.state.store(CircuitState::Closed as u8, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        if let Ok(mut guard) = self.last_failure.write() {
            *guard = None;
        }
    }
}

impl std::fmt::Debug for CircuitBreaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreaker")
            .field("state", &self.state())
            .field("failure_count", &self.failure_count.load(Ordering::Relaxed))
            .field("success_count", &self.success_count.load(Ordering::Relaxed))
            .finish()
    }
}

// ============================================================================
// PROVIDER REGISTRY
// ============================================================================

/// Enhanced registry for LLM providers with routing support.
pub struct ProviderRegistry {
    adapters: TokioRwLock<HashMap<String, Arc<dyn ProviderAdapter>>>,
    routing_strategy: RoutingStrategy,
    health_cache: TokioRwLock<HashMap<String, (PingResponse, Instant)>>,
    health_cache_ttl: Duration,
    round_robin_index: AtomicU64,
    listeners: TokioRwLock<ListenerChain>,
    circuit_breakers: TokioRwLock<HashMap<String, Arc<CircuitBreaker>>>,
}

impl ProviderRegistry {
    /// Create a new provider registry with the specified routing strategy.
    pub fn new(routing_strategy: RoutingStrategy) -> Self {
        Self {
            adapters: TokioRwLock::new(HashMap::new()),
            routing_strategy,
            health_cache: TokioRwLock::new(HashMap::new()),
            health_cache_ttl: Duration::from_secs(60),
            round_robin_index: AtomicU64::new(0),
            listeners: TokioRwLock::new(ListenerChain::new()),
            circuit_breakers: TokioRwLock::new(HashMap::new()),
        }
    }

    /// Create a registry with default round-robin strategy.
    pub fn with_round_robin() -> Self {
        Self::new(RoutingStrategy::RoundRobin)
    }

    /// Register a provider adapter.
    pub async fn register(&self, adapter: Arc<dyn ProviderAdapter>) {
        let id = adapter.provider_id().to_string();
        let mut adapters = self.adapters.write().await;
        adapters.insert(id.clone(), adapter);

        // Create circuit breaker for this provider
        let mut breakers = self.circuit_breakers.write().await;
        breakers.insert(
            id,
            Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
        );
    }

    /// Unregister a provider by ID.
    pub async fn unregister(&self, provider_id: &str) {
        let mut adapters = self.adapters.write().await;
        adapters.remove(provider_id);

        let mut breakers = self.circuit_breakers.write().await;
        breakers.remove(provider_id);
    }

    /// Add an event listener.
    pub async fn add_listener(&self, listener: Arc<dyn EventListener>) {
        let mut listeners = self.listeners.write().await;
        listeners.add(listener);
    }

    /// Get all registered provider IDs.
    pub async fn provider_ids(&self) -> Vec<String> {
        let adapters = self.adapters.read().await;
        adapters.keys().cloned().collect()
    }

    /// Check if any providers are registered.
    pub async fn has_providers(&self) -> bool {
        let adapters = self.adapters.read().await;
        !adapters.is_empty()
    }

    /// Echo to discover providers with specific capabilities.
    pub async fn echo(&self, request: EchoRequest) -> Vec<PingResponse> {
        let adapters = self.adapters.read().await;
        let mut responses = Vec::new();

        for (id, adapter) in adapters.iter() {
            // Check if adapter has any requested capability
            let has_capability = request.capabilities.is_empty()
                || request
                    .capabilities
                    .iter()
                    .any(|c| adapter.capabilities().contains(c));

            if has_capability {
                if let Ok(response) = adapter.ping().await {
                    // Cache the health response for routing decisions
                    {
                        let mut cache = self.health_cache.write().await;
                        cache.insert(id.clone(), (response.clone(), Instant::now()));
                    }
                    responses.push(response);
                }
            }
        }

        responses
    }

    /// Select a provider based on routing strategy.
    pub async fn select_provider(
        &self,
        capability: ProviderCapability,
    ) -> CaliberResult<Arc<dyn ProviderAdapter>> {
        let adapters = self.adapters.read().await;
        let breakers = self.circuit_breakers.read().await;

        // Filter by capability and circuit breaker state
        let available: Vec<_> = adapters
            .iter()
            .filter(|(id, adapter)| {
                adapter.capabilities().contains(&capability)
                    && breakers
                        .get(*id)
                        .map(|cb| cb.is_allowed())
                        .unwrap_or(true)
            })
            .collect();

        if available.is_empty() {
            return Err(CaliberError::Llm(LlmError::ProviderNotConfigured));
        }

        let selected = match &self.routing_strategy {
            RoutingStrategy::First => available.first().map(|(_, a)| Arc::clone(a)),
            RoutingStrategy::RoundRobin => {
                let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) as usize;
                available.get(idx % available.len()).map(|(_, a)| Arc::clone(a))
            }
            RoutingStrategy::Random => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                Instant::now().hash(&mut hasher);
                let idx = hasher.finish() as usize % available.len();
                available.get(idx).map(|(_, a)| Arc::clone(a))
            }
            RoutingStrategy::LeastLatency => {
                let cache = self.health_cache.read().await;
                available
                    .iter()
                    .min_by_key(|(id, _)| {
                        cache.get(*id).map(|(r, _)| r.latency_ms).unwrap_or(u64::MAX)
                    })
                    .map(|(_, a)| Arc::clone(a))
            }
            RoutingStrategy::Capability(_) => available.first().map(|(_, a)| Arc::clone(a)),
        };

        selected.ok_or_else(|| CaliberError::Llm(LlmError::ProviderNotConfigured))
    }

    /// Perform an embedding operation using the registry's routing.
    pub async fn embed(&self, request: EmbedRequest) -> CaliberResult<EmbedResponse> {
        let provider = self.select_provider(ProviderCapability::Embedding).await?;
        let breakers = self.circuit_breakers.read().await;
        let breaker = breakers.get(provider.provider_id());

        // Emit request event
        {
            let listeners = self.listeners.read().await;
            listeners
                .emit_request(RequestEvent {
                    request_id: request.request_id,
                    provider_id: provider.provider_id().to_string(),
                    operation: "embed".to_string(),
                    timestamp: Utc::now(),
                })
                .await;
        }

        match provider.embed(request.clone()).await {
            Ok(response) => {
                if let Some(cb) = breaker {
                    cb.record_success();
                }
                // Emit response event
                {
                    let listeners = self.listeners.read().await;
                    listeners
                        .emit_response(ResponseEvent {
                            request_id: request.request_id,
                            provider_id: provider.provider_id().to_string(),
                            operation: "embed".to_string(),
                            latency_ms: response.latency_ms,
                            success: true,
                            timestamp: Utc::now(),
                        })
                        .await;
                }
                Ok(response)
            }
            Err(e) => {
                if let Some(cb) = breaker {
                    cb.record_failure();
                }
                // Emit error event
                {
                    let listeners = self.listeners.read().await;
                    listeners
                        .emit_error(ErrorEvent {
                            request_id: request.request_id,
                            provider_id: provider.provider_id().to_string(),
                            operation: "embed".to_string(),
                            error_message: e.to_string(),
                            timestamp: Utc::now(),
                        })
                        .await;
                }
                Err(e)
            }
        }
    }

    /// Perform a summarization operation using the registry's routing.
    pub async fn summarize(&self, request: SummarizeRequest) -> CaliberResult<SummarizeResponse> {
        let provider = self
            .select_provider(ProviderCapability::Summarization)
            .await?;
        let breakers = self.circuit_breakers.read().await;
        let breaker = breakers.get(provider.provider_id());

        // Emit request event
        {
            let listeners = self.listeners.read().await;
            listeners
                .emit_request(RequestEvent {
                    request_id: request.request_id,
                    provider_id: provider.provider_id().to_string(),
                    operation: "summarize".to_string(),
                    timestamp: Utc::now(),
                })
                .await;
        }

        match provider.summarize(request.clone()).await {
            Ok(response) => {
                if let Some(cb) = breaker {
                    cb.record_success();
                }
                {
                    let listeners = self.listeners.read().await;
                    listeners
                        .emit_response(ResponseEvent {
                            request_id: request.request_id,
                            provider_id: provider.provider_id().to_string(),
                            operation: "summarize".to_string(),
                            latency_ms: response.latency_ms,
                            success: true,
                            timestamp: Utc::now(),
                        })
                        .await;
                }
                Ok(response)
            }
            Err(e) => {
                if let Some(cb) = breaker {
                    cb.record_failure();
                }
                {
                    let listeners = self.listeners.read().await;
                    listeners
                        .emit_error(ErrorEvent {
                            request_id: request.request_id,
                            provider_id: provider.provider_id().to_string(),
                            operation: "summarize".to_string(),
                            error_message: e.to_string(),
                            timestamp: Utc::now(),
                        })
                        .await;
                }
                Err(e)
            }
        }
    }
}

impl std::fmt::Debug for ProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRegistry")
            .field("routing_strategy", &self.routing_strategy)
            .finish()
    }
}

// ============================================================================
// COST TRACKER
// ============================================================================

/// Tracks token usage costs across providers.
pub struct CostTracker {
    embedding_tokens: AtomicU64,
    completion_input: AtomicU64,
    completion_output: AtomicU64,
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            embedding_tokens: AtomicU64::new(0),
            completion_input: AtomicU64::new(0),
            completion_output: AtomicU64::new(0),
        }
    }

    pub fn record_embedding(&self, tokens: u64) {
        self.embedding_tokens.fetch_add(tokens, Ordering::Relaxed);
    }

    pub fn record_completion(&self, input_tokens: u64, output_tokens: u64) {
        self.completion_input.fetch_add(input_tokens, Ordering::Relaxed);
        self.completion_output.fetch_add(output_tokens, Ordering::Relaxed);
    }

    pub fn embedding_tokens(&self) -> u64 {
        self.embedding_tokens.load(Ordering::Relaxed)
    }

    pub fn completion_input(&self) -> u64 {
        self.completion_input.load(Ordering::Relaxed)
    }

    pub fn completion_output(&self) -> u64 {
        self.completion_output.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.embedding_tokens.store(0, Ordering::Relaxed);
        self.completion_input.store(0, Ordering::Relaxed);
        self.completion_output.store(0, Ordering::Relaxed);
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CostTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CostTracker")
            .field("embedding_tokens", &self.embedding_tokens())
            .field("completion_input", &self.completion_input())
            .field("completion_output", &self.completion_output())
            .finish()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
        };
        let cb = CircuitBreaker::new(config);

        // Record failures up to threshold
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed());
    }

    #[test]
    fn test_cost_tracker() {
        let tracker = CostTracker::new();

        tracker.record_embedding(100);
        tracker.record_embedding(50);
        assert_eq!(tracker.embedding_tokens(), 150);

        tracker.record_completion(200, 100);
        assert_eq!(tracker.completion_input(), 200);
        assert_eq!(tracker.completion_output(), 100);

        tracker.reset();
        assert_eq!(tracker.embedding_tokens(), 0);
    }
}
