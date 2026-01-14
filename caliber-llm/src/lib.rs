//! CALIBER LLM - Vector Abstraction Layer (VAL)
//!
//! Provider-agnostic traits for embeddings and summarization.
//! This crate defines the interfaces that LLM providers must implement.
//! Actual provider implementations are user-supplied.

use caliber_core::{
    ArtifactType, CaliberError, CaliberResult, EmbeddingVector, LlmError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// EMBEDDING PROVIDER TRAIT (Task 6.1)
// ============================================================================

/// Trait for embedding providers.
/// Implementations must be thread-safe (Send + Sync).
///
/// # Example
/// ```ignore
/// struct OpenAIEmbedding { /* ... */ }
///
/// impl EmbeddingProvider for OpenAIEmbedding {
///     fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector> {
///         // Call OpenAI API
///     }
///     // ...
/// }
/// ```
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding for a single text.
    ///
    /// # Arguments
    /// * `text` - The text to embed
    ///
    /// # Returns
    /// * `Ok(EmbeddingVector)` - The embedding vector
    /// * `Err(CaliberError::Llm)` - If embedding fails
    fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector>;

    /// Generate embeddings for multiple texts in a batch.
    /// More efficient than calling embed() multiple times.
    ///
    /// # Arguments
    /// * `texts` - Slice of texts to embed
    ///
    /// # Returns
    /// * `Ok(Vec<EmbeddingVector>)` - Embedding vectors in same order as input
    /// * `Err(CaliberError::Llm)` - If embedding fails
    fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>>;

    /// Get the number of dimensions this provider produces.
    ///
    /// # Returns
    /// The dimension count (e.g., 384, 768, 1536, 3072)
    fn dimensions(&self) -> i32;

    /// Get the model identifier for this provider.
    ///
    /// # Returns
    /// A string identifying the model (e.g., "text-embedding-3-small")
    fn model_id(&self) -> &str;
}


// ============================================================================
// SUMMARIZATION PROVIDER TRAIT (Task 6.2)
// ============================================================================

/// Style of summarization output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SummarizeStyle {
    /// Short, concise summary
    Brief,
    /// Longer, more detailed summary
    Detailed,
    /// Structured summary with sections
    Structured,
}

/// Configuration for summarization requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummarizeConfig {
    /// Maximum tokens in the summary output
    pub max_tokens: i32,
    /// Style of summarization
    pub style: SummarizeStyle,
}

/// An artifact extracted from content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractedArtifact {
    /// Type of the extracted artifact
    pub artifact_type: ArtifactType,
    /// The extracted content
    pub content: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
}

/// Trait for summarization providers.
/// Implementations must be thread-safe (Send + Sync).
///
/// # Example
/// ```ignore
/// struct ClaudeSummarizer { /* ... */ }
///
/// impl SummarizationProvider for ClaudeSummarizer {
///     fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String> {
///         // Call Claude API
///     }
///     // ...
/// }
/// ```
pub trait SummarizationProvider: Send + Sync {
    /// Summarize content according to the provided configuration.
    ///
    /// # Arguments
    /// * `content` - The content to summarize
    /// * `config` - Summarization configuration
    ///
    /// # Returns
    /// * `Ok(String)` - The summary
    /// * `Err(CaliberError::Llm)` - If summarization fails
    fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String>;

    /// Extract artifacts of specified types from content.
    ///
    /// # Arguments
    /// * `content` - The content to extract from
    /// * `types` - Types of artifacts to look for
    ///
    /// # Returns
    /// * `Ok(Vec<ExtractedArtifact>)` - Extracted artifacts with confidence scores
    /// * `Err(CaliberError::Llm)` - If extraction fails
    fn extract_artifacts(
        &self,
        content: &str,
        types: &[ArtifactType],
    ) -> CaliberResult<Vec<ExtractedArtifact>>;

    /// Detect if two pieces of content contradict each other.
    ///
    /// # Arguments
    /// * `a` - First content
    /// * `b` - Second content
    ///
    /// # Returns
    /// * `Ok(true)` - If contradiction detected
    /// * `Ok(false)` - If no contradiction
    /// * `Err(CaliberError::Llm)` - If detection fails
    fn detect_contradiction(&self, a: &str, b: &str) -> CaliberResult<bool>;
}


// ============================================================================
// PROVIDER REGISTRY (Task 6.3)
// ============================================================================

/// Registry for LLM providers.
/// Providers must be explicitly registered - no auto-discovery.
///
/// # Example
/// ```ignore
/// let mut registry = ProviderRegistry::new();
/// registry.register_embedding(Box::new(my_embedding_provider));
/// registry.register_summarization(Box::new(my_summarization_provider));
///
/// // Later, use the providers
/// let embedding = registry.embedding()?.embed("hello")?;
/// ```
pub struct ProviderRegistry {
    /// Registered embedding provider (optional)
    embedding: Option<Arc<dyn EmbeddingProvider>>,
    /// Registered summarization provider (optional)
    summarization: Option<Arc<dyn SummarizationProvider>>,
}

impl ProviderRegistry {
    /// Create a new empty provider registry.
    /// No providers are registered by default.
    pub fn new() -> Self {
        Self {
            embedding: None,
            summarization: None,
        }
    }

    /// Register an embedding provider.
    /// Replaces any previously registered embedding provider.
    ///
    /// # Arguments
    /// * `provider` - The embedding provider to register
    pub fn register_embedding(&mut self, provider: Box<dyn EmbeddingProvider>) {
        self.embedding = Some(Arc::from(provider));
    }

    /// Register a summarization provider.
    /// Replaces any previously registered summarization provider.
    ///
    /// # Arguments
    /// * `provider` - The summarization provider to register
    pub fn register_summarization(&mut self, provider: Box<dyn SummarizationProvider>) {
        self.summarization = Some(Arc::from(provider));
    }

    /// Get the registered embedding provider.
    ///
    /// # Returns
    /// * `Ok(&dyn EmbeddingProvider)` - Reference to the provider
    /// * `Err(CaliberError::Llm(LlmError::ProviderNotConfigured))` - If no provider registered
    pub fn embedding(&self) -> CaliberResult<Arc<dyn EmbeddingProvider>> {
        self.embedding
            .clone()
            .ok_or(CaliberError::Llm(LlmError::ProviderNotConfigured))
    }

    /// Get the registered summarization provider.
    ///
    /// # Returns
    /// * `Ok(&dyn SummarizationProvider)` - Reference to the provider
    /// * `Err(CaliberError::Llm(LlmError::ProviderNotConfigured))` - If no provider registered
    pub fn summarization(&self) -> CaliberResult<Arc<dyn SummarizationProvider>> {
        self.summarization
            .clone()
            .ok_or(CaliberError::Llm(LlmError::ProviderNotConfigured))
    }

    /// Check if an embedding provider is registered.
    pub fn has_embedding(&self) -> bool {
        self.embedding.is_some()
    }

    /// Check if a summarization provider is registered.
    pub fn has_summarization(&self) -> bool {
        self.summarization.is_some()
    }

    /// Clear the embedding provider registration.
    pub fn clear_embedding(&mut self) {
        self.embedding = None;
    }

    /// Clear the summarization provider registration.
    pub fn clear_summarization(&mut self) {
        self.summarization = None;
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRegistry")
            .field("embedding", &self.embedding.is_some())
            .field("summarization", &self.summarization.is_some())
            .finish()
    }
}


// ============================================================================
// EMBEDDING CACHE (Optional utility)
// ============================================================================

/// Cache for embedding vectors to avoid redundant API calls.
/// Thread-safe via RwLock.
pub struct EmbeddingCache {
    /// Cache storage: content hash -> embedding
    cache: RwLock<HashMap<[u8; 32], EmbeddingVector>>,
    /// Maximum number of entries
    max_size: usize,
}

impl EmbeddingCache {
    /// Create a new embedding cache with specified maximum size.
    ///
    /// # Arguments
    /// * `max_size` - Maximum number of entries to cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    /// Get a cached embedding by content hash.
    ///
    /// # Arguments
    /// * `hash` - SHA-256 hash of the content
    ///
    /// # Returns
    /// * `Some(EmbeddingVector)` - If found in cache
    /// * `None` - If not cached
    pub fn get(&self, hash: &[u8; 32]) -> Option<EmbeddingVector> {
        self.cache.read().ok()?.get(hash).cloned()
    }

    /// Insert an embedding into the cache.
    /// If cache is full, this is a no-op (simple eviction strategy).
    ///
    /// # Arguments
    /// * `hash` - SHA-256 hash of the content
    /// * `embedding` - The embedding to cache
    pub fn insert(&self, hash: [u8; 32], embedding: EmbeddingVector) {
        if let Ok(mut cache) = self.cache.write() {
            if cache.len() < self.max_size {
                cache.insert(hash, embedding);
            }
        }
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Get the current number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }

    /// Check if the cache is empty.
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
// COST TRACKER (Optional utility)
// ============================================================================

/// Tracks token usage and estimated costs for LLM operations.
/// Thread-safe via atomic operations.
pub struct CostTracker {
    /// Total embedding tokens processed
    embedding_tokens: std::sync::atomic::AtomicI64,
    /// Total completion input tokens
    completion_input: std::sync::atomic::AtomicI64,
    /// Total completion output tokens
    completion_output: std::sync::atomic::AtomicI64,
}

impl CostTracker {
    /// Create a new cost tracker with zero counts.
    pub fn new() -> Self {
        Self {
            embedding_tokens: std::sync::atomic::AtomicI64::new(0),
            completion_input: std::sync::atomic::AtomicI64::new(0),
            completion_output: std::sync::atomic::AtomicI64::new(0),
        }
    }

    /// Record embedding token usage.
    ///
    /// # Arguments
    /// * `tokens` - Number of tokens processed
    pub fn record_embedding(&self, tokens: i64) {
        self.embedding_tokens
            .fetch_add(tokens, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record completion token usage.
    ///
    /// # Arguments
    /// * `input_tokens` - Number of input tokens
    /// * `output_tokens` - Number of output tokens
    pub fn record_completion(&self, input_tokens: i64, output_tokens: i64) {
        self.completion_input
            .fetch_add(input_tokens, std::sync::atomic::Ordering::Relaxed);
        self.completion_output
            .fetch_add(output_tokens, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get total embedding tokens.
    pub fn embedding_tokens(&self) -> i64 {
        self.embedding_tokens
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get total completion input tokens.
    pub fn completion_input(&self) -> i64 {
        self.completion_input
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get total completion output tokens.
    pub fn completion_output(&self) -> i64 {
        self.completion_output
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Reset all counters to zero.
    pub fn reset(&self) {
        self.embedding_tokens
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.completion_input
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.completion_output
            .store(0, std::sync::atomic::Ordering::Relaxed);
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
// MOCK PROVIDERS FOR TESTING (Task 6.4)
// ============================================================================

/// Mock embedding provider for testing.
/// Generates deterministic embeddings based on text content.
#[derive(Debug, Clone)]
pub struct MockEmbeddingProvider {
    /// Model identifier
    model_id: String,
    /// Number of dimensions to generate
    dimensions: i32,
}

impl MockEmbeddingProvider {
    /// Create a new mock embedding provider.
    ///
    /// # Arguments
    /// * `model_id` - Model identifier to report
    /// * `dimensions` - Number of dimensions to generate
    pub fn new(model_id: impl Into<String>, dimensions: i32) -> Self {
        Self {
            model_id: model_id.into(),
            dimensions,
        }
    }

    /// Generate a deterministic embedding from text.
    /// Uses a simple hash-based approach for reproducibility.
    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut data = vec![0.0f32; self.dimensions as usize];

        // Simple deterministic embedding based on text bytes
        for (i, byte) in text.bytes().enumerate() {
            let idx = i % self.dimensions as usize;
            data[idx] += (byte as f32) / 255.0;
        }

        // Normalize to unit vector
        let norm: f32 = data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut data {
                *x /= norm;
            }
        }

        data
    }
}

impl EmbeddingProvider for MockEmbeddingProvider {
    fn embed(&self, text: &str) -> CaliberResult<EmbeddingVector> {
        let data = self.generate_embedding(text);
        Ok(EmbeddingVector::new(data, self.model_id.clone()))
    }

    fn embed_batch(&self, texts: &[&str]) -> CaliberResult<Vec<EmbeddingVector>> {
        texts.iter().map(|text| self.embed(text)).collect()
    }

    fn dimensions(&self) -> i32 {
        self.dimensions
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}

/// Mock summarization provider for testing.
/// Generates simple summaries by truncating content.
#[derive(Debug, Clone)]
pub struct MockSummarizationProvider {
    /// Prefix to add to summaries
    prefix: String,
}

impl MockSummarizationProvider {
    /// Create a new mock summarization provider.
    pub fn new() -> Self {
        Self {
            prefix: "Summary: ".to_string(),
        }
    }

    /// Create a mock provider with a custom prefix.
    ///
    /// # Arguments
    /// * `prefix` - Prefix to add to summaries
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

impl SummarizationProvider for MockSummarizationProvider {
    fn summarize(&self, content: &str, config: &SummarizeConfig) -> CaliberResult<String> {
        // Simple mock: truncate content based on max_tokens
        // Rough estimate: 4 chars per token
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

    fn extract_artifacts(
        &self,
        content: &str,
        types: &[ArtifactType],
    ) -> CaliberResult<Vec<ExtractedArtifact>> {
        // Simple mock: return one artifact per requested type
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

    fn detect_contradiction(&self, a: &str, b: &str) -> CaliberResult<bool> {
        // Simple mock: detect contradiction if content is very different
        // (just check if they share any words)
        let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        // If Jaccard similarity is very low, consider it a contradiction
        let similarity = if union > 0 {
            intersection as f32 / union as f32
        } else {
            0.0
        };

        Ok(similarity < 0.1)
    }
}


// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry_new_is_empty() {
        let registry = ProviderRegistry::new();
        assert!(!registry.has_embedding());
        assert!(!registry.has_summarization());
    }

    #[test]
    fn test_provider_registry_register_embedding() {
        let mut registry = ProviderRegistry::new();
        let provider = MockEmbeddingProvider::new("test-model", 384);
        registry.register_embedding(Box::new(provider));
        assert!(registry.has_embedding());
        assert!(!registry.has_summarization());
    }

    #[test]
    fn test_provider_registry_register_summarization() {
        let mut registry = ProviderRegistry::new();
        let provider = MockSummarizationProvider::new();
        registry.register_summarization(Box::new(provider));
        assert!(!registry.has_embedding());
        assert!(registry.has_summarization());
    }

    #[test]
    fn test_provider_registry_clear() {
        let mut registry = ProviderRegistry::new();
        registry.register_embedding(Box::new(MockEmbeddingProvider::new("test", 384)));
        registry.register_summarization(Box::new(MockSummarizationProvider::new()));

        registry.clear_embedding();
        assert!(!registry.has_embedding());
        assert!(registry.has_summarization());

        registry.clear_summarization();
        assert!(!registry.has_embedding());
        assert!(!registry.has_summarization());
    }

    #[test]
    fn test_mock_embedding_provider_dimensions() {
        let provider = MockEmbeddingProvider::new("test-model", 768);
        assert_eq!(provider.dimensions(), 768);
        assert_eq!(provider.model_id(), "test-model");
    }

    #[test]
    fn test_mock_embedding_provider_embed() {
        let provider = MockEmbeddingProvider::new("test-model", 384);
        let embedding = provider.embed("hello world").unwrap();
        assert_eq!(embedding.dimensions, 384);
        assert_eq!(embedding.data.len(), 384);
        assert_eq!(embedding.model_id, "test-model");
    }

    #[test]
    fn test_mock_embedding_provider_deterministic() {
        let provider = MockEmbeddingProvider::new("test-model", 384);
        let e1 = provider.embed("hello world").unwrap();
        let e2 = provider.embed("hello world").unwrap();
        assert_eq!(e1.data, e2.data);
    }

    #[test]
    fn test_mock_embedding_provider_batch() {
        let provider = MockEmbeddingProvider::new("test-model", 384);
        let texts = vec!["hello", "world", "test"];
        let embeddings = provider.embed_batch(&texts).unwrap();
        assert_eq!(embeddings.len(), 3);
        for e in &embeddings {
            assert_eq!(e.dimensions, 384);
        }
    }

    #[test]
    fn test_mock_summarization_provider_summarize() {
        let provider = MockSummarizationProvider::new();
        let config = SummarizeConfig {
            max_tokens: 100,
            style: SummarizeStyle::Brief,
        };
        let summary = provider.summarize("This is a test content", &config).unwrap();
        assert!(summary.starts_with("Summary: "));
    }

    #[test]
    fn test_mock_summarization_provider_extract_artifacts() {
        let provider = MockSummarizationProvider::new();
        let types = vec![ArtifactType::Fact, ArtifactType::DesignDecision];
        let artifacts = provider
            .extract_artifacts("Some content here", &types)
            .unwrap();
        assert_eq!(artifacts.len(), 2);
        assert_eq!(artifacts[0].artifact_type, ArtifactType::Fact);
        assert_eq!(artifacts[1].artifact_type, ArtifactType::DesignDecision);
    }

    #[test]
    fn test_mock_summarization_provider_detect_contradiction() {
        let provider = MockSummarizationProvider::new();

        // Similar content - no contradiction
        let result = provider
            .detect_contradiction("the cat sat on the mat", "the cat sat on the floor")
            .unwrap();
        assert!(!result);

        // Very different content - contradiction
        let result = provider
            .detect_contradiction("xyz abc 123", "completely different words here")
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_embedding_cache_basic() {
        let cache = EmbeddingCache::new(100);
        assert!(cache.is_empty());

        let hash = [0u8; 32];
        let embedding = EmbeddingVector::new(vec![1.0, 2.0, 3.0], "test".to_string());

        cache.insert(hash, embedding.clone());
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(retrieved.data, embedding.data);
    }

    #[test]
    fn test_embedding_cache_clear() {
        let cache = EmbeddingCache::new(100);
        let hash = [0u8; 32];
        let embedding = EmbeddingVector::new(vec![1.0, 2.0, 3.0], "test".to_string());

        cache.insert(hash, embedding);
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cost_tracker_basic() {
        let tracker = CostTracker::new();
        assert_eq!(tracker.embedding_tokens(), 0);
        assert_eq!(tracker.completion_input(), 0);
        assert_eq!(tracker.completion_output(), 0);

        tracker.record_embedding(100);
        assert_eq!(tracker.embedding_tokens(), 100);

        tracker.record_completion(50, 25);
        assert_eq!(tracker.completion_input(), 50);
        assert_eq!(tracker.completion_output(), 25);
    }

    #[test]
    fn test_cost_tracker_reset() {
        let tracker = CostTracker::new();
        tracker.record_embedding(100);
        tracker.record_completion(50, 25);

        tracker.reset();
        assert_eq!(tracker.embedding_tokens(), 0);
        assert_eq!(tracker.completion_input(), 0);
        assert_eq!(tracker.completion_output(), 0);
    }

    #[test]
    fn test_summarize_style_variants() {
        let provider = MockSummarizationProvider::new();
        let content = "Test content";

        let brief = provider
            .summarize(
                content,
                &SummarizeConfig {
                    max_tokens: 100,
                    style: SummarizeStyle::Brief,
                },
            )
            .unwrap();
        assert!(brief.contains("Summary: "));

        let detailed = provider
            .summarize(
                content,
                &SummarizeConfig {
                    max_tokens: 100,
                    style: SummarizeStyle::Detailed,
                },
            )
            .unwrap();
        assert!(detailed.contains("[Detailed]"));

        let structured = provider
            .summarize(
                content,
                &SummarizeConfig {
                    max_tokens: 100,
                    style: SummarizeStyle::Structured,
                },
            )
            .unwrap();
        assert!(structured.contains("[Structured]"));
    }
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Property 6: Provider registry returns error when not configured
    // Feature: caliber-core-implementation, Property 6: Provider registry returns error when not configured
    // Validates: Requirements 6.4
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 6: For any ProviderRegistry with no embedding provider registered,
        /// calling embedding() SHALL return Err(LlmError::ProviderNotConfigured)
        #[test]
        fn prop_registry_returns_error_when_embedding_not_configured(
            // Generate random state to ensure we test various scenarios
            _seed in 0u64..1000u64
        ) {
            let registry = ProviderRegistry::new();

            // Verify embedding() returns ProviderNotConfigured error
            let result = registry.embedding();
            prop_assert!(result.is_err());

            match result {
                Err(CaliberError::Llm(LlmError::ProviderNotConfigured)) => {
                    // Expected error
                }
                Err(other) => {
                    prop_assert!(false, "Expected ProviderNotConfigured, got {:?}", other);
                }
                Ok(_) => {
                    prop_assert!(false, "Expected error, got Ok");
                }
            }
        }

        /// Property 6 (extended): For any ProviderRegistry with no summarization provider registered,
        /// calling summarization() SHALL return Err(LlmError::ProviderNotConfigured)
        #[test]
        fn prop_registry_returns_error_when_summarization_not_configured(
            _seed in 0u64..1000u64
        ) {
            let registry = ProviderRegistry::new();

            // Verify summarization() returns ProviderNotConfigured error
            let result = registry.summarization();
            prop_assert!(result.is_err());

            match result {
                Err(CaliberError::Llm(LlmError::ProviderNotConfigured)) => {
                    // Expected error
                }
                Err(other) => {
                    prop_assert!(false, "Expected ProviderNotConfigured, got {:?}", other);
                }
                Ok(_) => {
                    prop_assert!(false, "Expected error, got Ok");
                }
            }
        }

        /// Property: After registering an embedding provider, embedding() SHALL return Ok
        #[test]
        fn prop_registry_returns_ok_when_embedding_configured(
            dimensions in 1i32..4096i32,
            model_id in "[a-z]{1,20}"
        ) {
            let mut registry = ProviderRegistry::new();
            let provider = MockEmbeddingProvider::new(model_id.clone(), dimensions);
            registry.register_embedding(Box::new(provider));

            let result = registry.embedding();
            prop_assert!(result.is_ok());

            let provider = result.unwrap();
            prop_assert_eq!(provider.dimensions(), dimensions);
            prop_assert_eq!(provider.model_id(), model_id);
        }

        /// Property: After registering a summarization provider, summarization() SHALL return Ok
        #[test]
        fn prop_registry_returns_ok_when_summarization_configured(
            _seed in 0u64..1000u64
        ) {
            let mut registry = ProviderRegistry::new();
            let provider = MockSummarizationProvider::new();
            registry.register_summarization(Box::new(provider));

            let result = registry.summarization();
            prop_assert!(result.is_ok());
        }

        /// Property: Mock embedding provider produces vectors with correct dimensions
        #[test]
        fn prop_mock_embedding_correct_dimensions(
            dimensions in 1i32..2048i32,
            text in ".{1,100}"
        ) {
            let provider = MockEmbeddingProvider::new("test", dimensions);
            let embedding = provider.embed(&text).unwrap();

            prop_assert_eq!(embedding.dimensions, dimensions);
            prop_assert_eq!(embedding.data.len(), dimensions as usize);
        }

        /// Property: Mock embedding provider is deterministic
        #[test]
        fn prop_mock_embedding_deterministic(
            dimensions in 1i32..1024i32,
            text in ".{1,100}"
        ) {
            let provider = MockEmbeddingProvider::new("test", dimensions);
            let e1 = provider.embed(&text).unwrap();
            let e2 = provider.embed(&text).unwrap();

            prop_assert_eq!(e1.data, e2.data);
            prop_assert_eq!(e1.model_id, e2.model_id);
            prop_assert_eq!(e1.dimensions, e2.dimensions);
        }

        /// Property: Mock embedding batch produces correct number of embeddings
        #[test]
        fn prop_mock_embedding_batch_count(
            dimensions in 1i32..512i32,
            texts in prop::collection::vec(".{1,50}", 1..10)
        ) {
            let provider = MockEmbeddingProvider::new("test", dimensions);
            let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
            let embeddings = provider.embed_batch(&text_refs).unwrap();

            prop_assert_eq!(embeddings.len(), texts.len());
            for e in &embeddings {
                prop_assert_eq!(e.dimensions, dimensions);
            }
        }
    }
}
