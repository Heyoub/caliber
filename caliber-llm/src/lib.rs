//! CALIBER LLM - Vector Abstraction Layer (VAL)
//!
//! Provider-agnostic async traits for embeddings and summarization.
//! Features:
//! - Async traits with tokio support
//! - ProviderAdapter with Echo/Ping discovery
//! - EventListener pattern for request/response hooks
//! - Circuit breaker for health management
//! - Routing strategies (RoundRobin, LeastLatency, etc.)

use async_trait::async_trait;
use caliber_core::{ArtifactType, CaliberError, CaliberResult, EmbeddingVector, LlmError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock as TokioRwLock;
use uuid::Uuid;

// ============================================================================
// ASYNC EMBEDDING PROVIDER TRAIT
// ============================================================================

/// Async trait for embedding providers.
/// Implementations must be thread-safe (Send + Sync).
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding for a single text.
    async fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector>;

    /// Generate embeddings for multiple texts in a batch.
    async fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>>;

    /// Get the number of dimensions this provider produces.
    fn dimensions(&self) -> i32;

    /// Get the model identifier for this provider.
    fn model_id(&self) -> &str;
}

// ============================================================================
// ASYNC SUMMARIZATION PROVIDER TRAIT
// ============================================================================

/// Style of summarization output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SummarizeStyle {
    Brief,
    Detailed,
    Structured,
}

/// Configuration for summarization requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummarizeConfig {
    pub max_tokens: i32,
    pub style: SummarizeStyle,
}

/// An artifact extracted from content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractedArtifact {
    pub artifact_type: ArtifactType,
    pub content: String,
    pub confidence: f32,
}

/// Async trait for summarization providers.
#[async_trait]
pub trait SummarizationProvider: Send + Sync {
    /// Summarize content according to the provided configuration.
    async fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String>;

    /// Extract artifacts of specified types from content.
    async fn extract_artifacts(
        &self,
        content: &str,
        types: &[ArtifactType],
    ) -> CaliberResult<Vec<ExtractedArtifact>>;

    /// Detect if two pieces of content contradict each other.
    async fn detect_contradiction(&self, a: &str, b: &str) -> CaliberResult<bool>;
}

// ============================================================================
// PROVIDER CAPABILITIES & HEALTH
// ============================================================================

/// Capabilities a provider can offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderCapability {
    Embedding,
    Summarization,
    ArtifactExtraction,
    ContradictionDetection,
}

/// Health status of a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

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
// PROVIDER ADAPTER TRAIT
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

/// Adapter trait for providers with Echo/Ping support.
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

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed = 0,
    Open = 1,
    HalfOpen = 2,
}

impl From<u8> for CircuitState {
    fn from(v: u8) -> Self {
        match v {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            _ => CircuitState::HalfOpen,
        }
    }
}

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
// ROUTING STRATEGIES
// ============================================================================

/// Strategy for routing requests to providers.
#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    /// Round-robin between providers
    RoundRobin,
    /// Route to provider with lowest latency
    LeastLatency,
    /// Random selection
    Random,
    /// Route based on capability
    Capability(ProviderCapability),
    /// Always use first available provider
    First,
}

// ============================================================================
// PROVIDER REGISTRY (Enhanced)
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

    /// Echo to discover providers with specific capabilities.
    /// Results are cached for health-aware routing (LeastLatency strategy).
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
                let idx =
                    self.round_robin_index.fetch_add(1, Ordering::Relaxed) as usize % available.len();
                available.get(idx).map(|(_, a)| Arc::clone(a))
            }
            RoutingStrategy::Random => {
                use std::time::SystemTime;
                let seed = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as usize;
                let idx = seed % available.len();
                available.get(idx).map(|(_, a)| Arc::clone(a))
            }
            RoutingStrategy::LeastLatency => {
                let health_cache = self.health_cache.read().await;
                let ttl = self.health_cache_ttl;
                let mut best: Option<(&str, u64)> = None;

                for (id, _) in &available {
                    if let Some((ping, cached_at)) = health_cache.get(*id) {
                        // Only use cached health data if not stale
                        if cached_at.elapsed() < ttl {
                            match best {
                                None => best = Some((id.as_str(), ping.latency_ms)),
                                Some((_, lat)) if ping.latency_ms < lat => {
                                    best = Some((id.as_str(), ping.latency_ms))
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if let Some((id, _)) = best {
                    adapters.get(id).cloned()
                } else {
                    available.first().map(|(_, a)| Arc::clone(a))
                }
            }
            RoutingStrategy::Capability(_) => available.first().map(|(_, a)| Arc::clone(a)),
        };

        selected.ok_or(CaliberError::Llm(LlmError::ProviderNotConfigured))
    }

    /// Perform embedding using selected provider.
    pub async fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector> {
        let provider = self.select_provider(ProviderCapability::Embedding).await?;
        let provider_id = provider.provider_id().to_string();
        let request_id = Uuid::now_v7();

        // Emit request event
        {
            let listeners = self.listeners.read().await;
            listeners
                .emit_request(RequestEvent {
                    request_id,
                    provider_id: provider_id.clone(),
                    operation: "embed".to_string(),
                    timestamp: Utc::now(),
                })
                .await;
        }

        let start = Instant::now();
        let result = provider
            .embed(EmbedRequest {
                text: text.to_string(),
                request_id,
            })
            .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        // Update circuit breaker and emit events
        {
            let breakers = self.circuit_breakers.read().await;
            if let Some(cb) = breakers.get(&provider_id) {
                match &result {
                    Ok(_) => cb.record_success(),
                    Err(_) => cb.record_failure(),
                }
            }
        }

        {
            let listeners = self.listeners.read().await;
            match &result {
                Ok(_) => {
                    listeners
                        .emit_response(ResponseEvent {
                            request_id,
                            provider_id,
                            operation: "embed".to_string(),
                            latency_ms,
                            success: true,
                            timestamp: Utc::now(),
                        })
                        .await;
                }
                Err(e) => {
                    listeners
                        .emit_error(ErrorEvent {
                            request_id,
                            provider_id,
                            operation: "embed".to_string(),
                            error_message: e.to_string(),
                            timestamp: Utc::now(),
                        })
                        .await;
                }
            }
        }

        result.map(|r| r.embedding)
    }

    /// Perform summarization using selected provider.
    pub async fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String> {
        let provider = self
            .select_provider(ProviderCapability::Summarization)
            .await?;
        let provider_id = provider.provider_id().to_string();
        let request_id = Uuid::now_v7();

        // Emit request event
        {
            let listeners = self.listeners.read().await;
            listeners
                .emit_request(RequestEvent {
                    request_id,
                    provider_id: provider_id.clone(),
                    operation: "summarize".to_string(),
                    timestamp: Utc::now(),
                })
                .await;
        }

        let start = Instant::now();
        let result = provider
            .summarize(SummarizeRequest {
                content: content.to_string(),
                config: config.clone(),
                request_id,
            })
            .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        // Update circuit breaker
        {
            let breakers = self.circuit_breakers.read().await;
            if let Some(cb) = breakers.get(&provider_id) {
                match &result {
                    Ok(_) => cb.record_success(),
                    Err(_) => cb.record_failure(),
                }
            }
        }

        {
            let listeners = self.listeners.read().await;
            match &result {
                Ok(_) => {
                    listeners
                        .emit_response(ResponseEvent {
                            request_id,
                            provider_id,
                            operation: "summarize".to_string(),
                            latency_ms,
                            success: true,
                            timestamp: Utc::now(),
                        })
                        .await;
                }
                Err(e) => {
                    listeners
                        .emit_error(ErrorEvent {
                            request_id,
                            provider_id,
                            operation: "summarize".to_string(),
                            error_message: e.to_string(),
                            timestamp: Utc::now(),
                        })
                        .await;
                }
            }
        }

        result.map(|r| r.summary)
    }

    /// Check if any provider is registered.
    pub async fn has_providers(&self) -> bool {
        !self.adapters.read().await.is_empty()
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
// EMBEDDING CACHE
// ============================================================================

/// Cache for embedding vectors to avoid redundant API calls.
pub struct EmbeddingCache {
    cache: RwLock<HashMap<[u8; 32], EmbeddingVector>>,
    max_size: usize,
}

impl EmbeddingCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    pub fn get(&self, hash: &[u8; 32]) -> Option<EmbeddingVector> {
        self.cache.read().ok()?.get(hash).cloned()
    }

    pub fn insert(&self, hash: [u8; 32], embedding: EmbeddingVector) {
        if let Ok(mut cache) = self.cache.write() {
            if cache.len() < self.max_size {
                cache.insert(hash, embedding);
            }
        }
    }

    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    pub fn len(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Debug for EmbeddingCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingCache")
            .field("max_size", &self.max_size)
            .field("current_size", &self.len())
            .finish()
    }
}

// ============================================================================
// COST TRACKER
// ============================================================================

/// Tracks token usage and estimated costs for LLM operations.
pub struct CostTracker {
    embedding_tokens: std::sync::atomic::AtomicI64,
    completion_input: std::sync::atomic::AtomicI64,
    completion_output: std::sync::atomic::AtomicI64,
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            embedding_tokens: std::sync::atomic::AtomicI64::new(0),
            completion_input: std::sync::atomic::AtomicI64::new(0),
            completion_output: std::sync::atomic::AtomicI64::new(0),
        }
    }

    pub fn record_embedding(&self, tokens: i64) {
        self.embedding_tokens
            .fetch_add(tokens, Ordering::Relaxed);
    }

    pub fn record_completion(&self, input_tokens: i64, output_tokens: i64) {
        self.completion_input.fetch_add(input_tokens, Ordering::Relaxed);
        self.completion_output.fetch_add(output_tokens, Ordering::Relaxed);
    }

    pub fn embedding_tokens(&self) -> i64 {
        self.embedding_tokens.load(Ordering::Relaxed)
    }

    pub fn completion_input(&self) -> i64 {
        self.completion_input.load(Ordering::Relaxed)
    }

    pub fn completion_output(&self) -> i64 {
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
// MOCK PROVIDERS (Async)
// ============================================================================

/// Mock embedding provider for testing (async).
#[derive(Debug, Clone)]
pub struct MockEmbeddingProvider {
    model_id: String,
    dimensions: i32,
}

impl MockEmbeddingProvider {
    pub fn new(model_id: impl Into<String>, dimensions: i32) -> Self {
        Self {
            model_id: model_id.into(),
            dimensions,
        }
    }

    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut data = vec![0.0f32; self.dimensions as usize];

        for (i, byte) in text.bytes().enumerate() {
            let idx = i % self.dimensions as usize;
            data[idx] += (byte as f32) / 255.0;
        }

        let norm: f32 = data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut data {
                *x /= norm;
            }
        }

        data
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector> {
        let data = self.generate_embedding(text);
        Ok(EmbeddingVector::new(data, self.model_id.clone()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimensions(&self) -> i32 {
        self.dimensions
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}

/// Mock summarization provider for testing (async).
#[derive(Debug, Clone)]
pub struct MockSummarizationProvider {
    prefix: String,
}

impl MockSummarizationProvider {
    pub fn new() -> Self {
        Self {
            prefix: "Summary: ".to_string(),
        }
    }

    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl Default for MockSummarizationProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SummarizationProvider for MockSummarizationProvider {
    async fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String> {
        let max_chars = (config.max_tokens * 4) as usize;
        let truncated = if content.len() > max_chars {
            &content[..max_chars]
        } else {
            content
        };

        let summary = match config.style {
            SummarizeStyle::Brief => format!("{}{}", self.prefix, truncated),
            SummarizeStyle::Detailed => format!("{}[Detailed] {}", self.prefix, truncated),
            SummarizeStyle::Structured => {
                format!("{}[Structured]\n- Content: {}", self.prefix, truncated)
            }
        };

        Ok(summary)
    }

    async fn extract_artifacts(
        &self,
        content: &str,
        types: &[ArtifactType],
    ) -> CaliberResult<Vec<ExtractedArtifact>> {
        let artifacts = types
            .iter()
            .map(|artifact_type| ExtractedArtifact {
                artifact_type: *artifact_type,
                content: format!("Extracted from: {}", &content[..content.len().min(50)]),
                confidence: 0.8,
            })
            .collect();

        Ok(artifacts)
    }

    async fn detect_contradiction(&self, a: &str, b: &str) -> CaliberResult<bool> {
        let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        let similarity = if union > 0 {
            intersection as f32 / union as f32
        } else {
            0.0
        };

        Ok(similarity < 0.1)
    }
}

/// Mock provider adapter that wraps embedding and summarization providers.
pub struct MockProviderAdapter {
    provider_id: String,
    embedding: MockEmbeddingProvider,
    summarization: MockSummarizationProvider,
    capabilities: Vec<ProviderCapability>,
}

impl MockProviderAdapter {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            embedding: MockEmbeddingProvider::new("mock-embed", 384),
            summarization: MockSummarizationProvider::new(),
            capabilities: vec![
                ProviderCapability::Embedding,
                ProviderCapability::Summarization,
                ProviderCapability::ArtifactExtraction,
                ProviderCapability::ContradictionDetection,
            ],
        }
    }
}

#[async_trait]
impl ProviderAdapter for MockProviderAdapter {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn capabilities(&self) -> &[ProviderCapability] {
        &self.capabilities
    }

    async fn ping(&self) -> CaliberResult<PingResponse> {
        Ok(PingResponse {
            provider_id: self.provider_id.clone(),
            capabilities: self.capabilities.clone(),
            latency_ms: 1,
            health: HealthStatus::Healthy,
            metadata: HashMap::new(),
        })
    }

    async fn embed(&self, request: EmbedRequest) -> CaliberResult<EmbedResponse> {
        let start = Instant::now();
        let embedding = self.embedding.embed(&request.text).await?;
        Ok(EmbedResponse {
            embedding,
            request_id: request.request_id,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn summarize(&self, request: SummarizeRequest) -> CaliberResult<SummarizeResponse> {
        let start = Instant::now();
        let summary = self.summarization.summarize(&request.content, &request.config).await?;
        Ok(SummarizeResponse {
            summary,
            request_id: request.request_id,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_provider() {
        let provider = MockEmbeddingProvider::new("test-model", 384);
        let embedding = provider.embed("hello world").await.unwrap();
        assert_eq!(embedding.dimensions, 384);
        assert_eq!(embedding.data.len(), 384);
    }

    #[tokio::test]
    async fn test_mock_embedding_deterministic() {
        let provider = MockEmbeddingProvider::new("test-model", 384);
        let e1 = provider.embed("hello world").await.unwrap();
        let e2 = provider.embed("hello world").await.unwrap();
        assert_eq!(e1.data, e2.data);
    }

    #[tokio::test]
    async fn test_mock_summarization_provider() {
        let provider = MockSummarizationProvider::new();
        let config = SummarizeConfig {
            max_tokens: 100,
            style: SummarizeStyle::Brief,
        };
        let summary = provider.summarize("Test content", &config).await.unwrap();
        assert!(summary.starts_with("Summary: "));
    }

    #[tokio::test]
    async fn test_provider_registry_empty() {
        let registry = ProviderRegistry::with_round_robin();
        assert!(!registry.has_providers().await);
    }

    #[tokio::test]
    async fn test_provider_registry_register() {
        let registry = ProviderRegistry::with_round_robin();
        let adapter = Arc::new(MockProviderAdapter::new("test"));
        registry.register(adapter).await;
        assert!(registry.has_providers().await);
    }

    #[tokio::test]
    async fn test_provider_registry_embed() {
        let registry = ProviderRegistry::with_round_robin();
        let adapter = Arc::new(MockProviderAdapter::new("test"));
        registry.register(adapter).await;

        let embedding = registry.embed("hello").await.unwrap();
        assert_eq!(embedding.dimensions, 384);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_allowed());
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed());
    }

    #[test]
    fn test_embedding_cache() {
        let cache = EmbeddingCache::new(100);
        let hash = [0u8; 32];
        let embedding = EmbeddingVector::new(vec![1.0, 2.0, 3.0], "test".to_string());

        cache.insert(hash, embedding.clone());
        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(retrieved.data, embedding.data);
    }

    #[test]
    fn test_cost_tracker() {
        let tracker = CostTracker::new();
        tracker.record_embedding(100);
        assert_eq!(tracker.embedding_tokens(), 100);

        tracker.record_completion(50, 25);
        assert_eq!(tracker.completion_input(), 50);
        assert_eq!(tracker.completion_output(), 25);
    }
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn prop_mock_embedding_correct_dimensions(
            dimensions in 1i32..1024i32,
            text in ".{1,100}"
        ) {
            let provider = MockEmbeddingProvider::new("test", dimensions);
            let rt = tokio::runtime::Runtime::new().unwrap();
            let embedding = rt.block_on(provider.embed(&text)).unwrap();

            prop_assert_eq!(embedding.dimensions, dimensions);
            prop_assert_eq!(embedding.data.len(), dimensions as usize);
        }

        #[test]
        fn prop_mock_embedding_deterministic(
            dimensions in 1i32..512i32,
            text in ".{1,100}"
        ) {
            let provider = MockEmbeddingProvider::new("test", dimensions);
            let rt = tokio::runtime::Runtime::new().unwrap();
            let e1 = rt.block_on(provider.embed(&text)).unwrap();
            let e2 = rt.block_on(provider.embed(&text)).unwrap();

            prop_assert_eq!(e1.data, e2.data);
        }

        #[test]
        fn prop_circuit_breaker_opens_after_threshold(
            threshold in 1u32..10u32
        ) {
            let config = CircuitBreakerConfig {
                failure_threshold: threshold,
                success_threshold: 3,
                timeout: Duration::from_secs(30),
            };
            let cb = CircuitBreaker::new(config);

            for _ in 0..threshold {
                cb.record_failure();
            }

            prop_assert_eq!(cb.state(), CircuitState::Open);
        }
    }
}
