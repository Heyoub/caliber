//! LLM-related primitive types and traits.
//!
//! Pure data types and interface definitions for LLM operations.
//! Runtime orchestration (ProviderRegistry, CircuitBreaker) lives in caliber-api/src/providers/.

use crate::{ArtifactType, CaliberResult, EmbeddingVector};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ============================================================================
// SUMMARIZATION TYPES
// ============================================================================

/// Style of summarization output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum SummarizeStyle {
    /// Brief, high-level summary
    Brief,
    /// Detailed, comprehensive summary
    Detailed,
    /// Structured summary with sections
    Structured,
}

impl SummarizeStyle {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Brief => "Brief",
            Self::Detailed => "Detailed",
            Self::Structured => "Structured",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, SummarizeStyleParseError> {
        match s {
            "Brief" => Ok(Self::Brief),
            "Detailed" => Ok(Self::Detailed),
            "Structured" => Ok(Self::Structured),
            _ => Err(SummarizeStyleParseError(s.to_string())),
        }
    }
}

/// Error parsing SummarizeStyle from string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummarizeStyleParseError(pub String);

impl std::fmt::Display for SummarizeStyleParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid summarize style: {}", self.0)
    }
}

impl std::error::Error for SummarizeStyleParseError {}

/// Configuration for summarization requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SummarizeConfig {
    /// Maximum tokens in the summary
    pub max_tokens: i32,
    /// Style of summary to generate
    pub style: SummarizeStyle,
}

impl Default for SummarizeConfig {
    fn default() -> Self {
        Self {
            max_tokens: 256,
            style: SummarizeStyle::Brief,
        }
    }
}

// ============================================================================
// PROVIDER CAPABILITY
// ============================================================================

/// Capabilities a provider can offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ProviderCapability {
    /// Generate embeddings
    Embedding,
    /// Generate summaries
    Summarization,
    /// Extract artifacts from content
    ArtifactExtraction,
    /// Detect contradictions between content
    ContradictionDetection,
}

impl ProviderCapability {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Embedding => "Embedding",
            Self::Summarization => "Summarization",
            Self::ArtifactExtraction => "ArtifactExtraction",
            Self::ContradictionDetection => "ContradictionDetection",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, ProviderCapabilityParseError> {
        match s {
            "Embedding" => Ok(Self::Embedding),
            "Summarization" => Ok(Self::Summarization),
            "ArtifactExtraction" => Ok(Self::ArtifactExtraction),
            "ContradictionDetection" => Ok(Self::ContradictionDetection),
            _ => Err(ProviderCapabilityParseError(s.to_string())),
        }
    }
}

/// Error parsing ProviderCapability from string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCapabilityParseError(pub String);

impl std::fmt::Display for ProviderCapabilityParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid provider capability: {}", self.0)
    }
}

impl std::error::Error for ProviderCapabilityParseError {}

// ============================================================================
// CIRCUIT STATE
// ============================================================================

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed = 0,
    /// Circuit is open, requests are rejected
    Open = 1,
    /// Circuit is half-open, testing if service recovered
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

impl CircuitState {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Closed => "Closed",
            Self::Open => "Open",
            Self::HalfOpen => "HalfOpen",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, CircuitStateParseError> {
        match s {
            "Closed" => Ok(Self::Closed),
            "Open" => Ok(Self::Open),
            "HalfOpen" => Ok(Self::HalfOpen),
            _ => Err(CircuitStateParseError(s.to_string())),
        }
    }
}

/// Error parsing CircuitState from string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitStateParseError(pub String);

impl std::fmt::Display for CircuitStateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid circuit state: {}", self.0)
    }
}

impl std::error::Error for CircuitStateParseError {}

// ============================================================================
// ROUTING STRATEGY
// ============================================================================

/// Strategy for routing requests to providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum RoutingStrategy {
    /// Round-robin between providers
    #[default]
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

impl RoutingStrategy {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::RoundRobin => "RoundRobin",
            Self::LeastLatency => "LeastLatency",
            Self::Random => "Random",
            Self::Capability(_) => "Capability",
            Self::First => "First",
        }
    }
}

// ============================================================================
// EXTRACTED ARTIFACT
// ============================================================================

/// An artifact extracted from content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExtractedArtifact {
    /// Type of artifact extracted
    pub artifact_type: ArtifactType,
    /// The extracted content
    pub content: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

// ============================================================================
// PROVIDER TRAITS
// ============================================================================

/// Async trait for embedding providers.
///
/// Implementations must be thread-safe (Send + Sync).
/// This is the interface definition only - implementations live in caliber-api/src/providers/.
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

/// Async trait for summarization providers.
///
/// This is the interface definition only - implementations live in caliber-api/src/providers/.
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
// TOKENIZER TRAIT
// ============================================================================

/// Trait for counting tokens in text.
///
/// Used for token budget management in context assembly.
/// Implementations can provide exact counts (using actual tokenizer)
/// or heuristic estimates based on character ratios.
pub trait Tokenizer: Send + Sync {
    /// Count tokens in the given text.
    fn count(&self, text: &str) -> i32;

    /// Get the model family this tokenizer is for (e.g., "gpt-4", "claude").
    fn model_family(&self) -> &str;

    /// Encode text to token IDs (for advanced use cases).
    /// Returns empty vec if not supported.
    fn encode(&self, text: &str) -> Vec<u32>;

    /// Decode token IDs back to text.
    /// Returns empty string if not supported.
    fn decode(&self, tokens: &[u32]) -> String;
}

/// Heuristic tokenizer using character-to-token ratios.
///
/// This provides fast, approximate token counts without requiring
/// an actual tokenizer model. Good for quick estimates.
#[derive(Debug, Clone)]
pub struct HeuristicTokenizer {
    /// Tokens per character ratio (model-specific)
    ratio: f32,
    /// Model family identifier
    model_family: String,
}

impl HeuristicTokenizer {
    /// Create a new heuristic tokenizer for a specific model.
    ///
    /// Uses empirically-derived ratios based on model family.
    pub fn for_model(model: &str) -> Self {
        let (ratio, family) = if model.contains("gpt-4") || model.contains("gpt-3.5") {
            // GPT models: ~4 characters per token on average
            (0.25, "gpt")
        } else if model.contains("claude") {
            // Claude models: slightly higher token density
            (0.28, "claude")
        } else if model.contains("text-embedding") {
            // OpenAI embedding models
            (0.25, "openai-embedding")
        } else if model.contains("llama") || model.contains("mistral") {
            // Open source models vary more
            (0.27, "open-source")
        } else {
            // Conservative default
            (0.30, "unknown")
        };

        Self {
            ratio,
            model_family: family.to_string(),
        }
    }

    /// Create with a custom ratio.
    pub fn with_ratio(ratio: f32, model_family: impl Into<String>) -> Self {
        Self {
            ratio,
            model_family: model_family.into(),
        }
    }

    /// Get the current ratio.
    pub fn ratio(&self) -> f32 {
        self.ratio
    }
}

impl Default for HeuristicTokenizer {
    fn default() -> Self {
        Self::for_model("gpt-4")
    }
}

impl Tokenizer for HeuristicTokenizer {
    fn count(&self, text: &str) -> i32 {
        // Multiply character count by ratio
        (text.len() as f32 * self.ratio).ceil() as i32
    }

    fn model_family(&self) -> &str {
        &self.model_family
    }

    fn encode(&self, _text: &str) -> Vec<u32> {
        // Heuristic tokenizer doesn't support encoding
        Vec::new()
    }

    fn decode(&self, _tokens: &[u32]) -> String {
        // Heuristic tokenizer doesn't support decoding
        String::new()
    }
}

/// Estimate tokens using the default heuristic.
///
/// This is the legacy function for backward compatibility.
/// New code should use `HeuristicTokenizer` directly for model-specific estimates.
pub fn estimate_tokens(text: &str) -> i32 {
    HeuristicTokenizer::default().count(text)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_style_roundtrip() {
        for style in [
            SummarizeStyle::Brief,
            SummarizeStyle::Detailed,
            SummarizeStyle::Structured,
        ] {
            let s = style.as_db_str();
            let parsed = SummarizeStyle::from_db_str(s).unwrap();
            assert_eq!(style, parsed);
        }
    }

    #[test]
    fn test_provider_capability_roundtrip() {
        for cap in [
            ProviderCapability::Embedding,
            ProviderCapability::Summarization,
            ProviderCapability::ArtifactExtraction,
            ProviderCapability::ContradictionDetection,
        ] {
            let s = cap.as_db_str();
            let parsed = ProviderCapability::from_db_str(s).unwrap();
            assert_eq!(cap, parsed);
        }
    }

    #[test]
    fn test_circuit_state_from_u8() {
        assert_eq!(CircuitState::from(0), CircuitState::Closed);
        assert_eq!(CircuitState::from(1), CircuitState::Open);
        assert_eq!(CircuitState::from(2), CircuitState::HalfOpen);
        assert_eq!(CircuitState::from(255), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_state_roundtrip() {
        for state in [
            CircuitState::Closed,
            CircuitState::Open,
            CircuitState::HalfOpen,
        ] {
            let s = state.as_db_str();
            let parsed = CircuitState::from_db_str(s).unwrap();
            assert_eq!(state, parsed);
        }
    }

    #[test]
    fn test_summarize_config_default() {
        let config = SummarizeConfig::default();
        assert_eq!(config.max_tokens, 256);
        assert_eq!(config.style, SummarizeStyle::Brief);
    }

    #[test]
    fn test_routing_strategy_default() {
        assert_eq!(RoutingStrategy::default(), RoutingStrategy::RoundRobin);
    }

    #[test]
    fn test_heuristic_tokenizer_gpt4() {
        let tokenizer = HeuristicTokenizer::for_model("gpt-4");
        assert_eq!(tokenizer.model_family(), "gpt");
        assert_eq!(tokenizer.ratio(), 0.25);

        // 100 chars * 0.25 = 25 tokens
        let text = "a".repeat(100);
        assert_eq!(tokenizer.count(&text), 25);
    }

    #[test]
    fn test_heuristic_tokenizer_claude() {
        let tokenizer = HeuristicTokenizer::for_model("claude-3-opus");
        assert_eq!(tokenizer.model_family(), "claude");
        assert_eq!(tokenizer.ratio(), 0.28);

        // 100 chars * 0.28 = 28 tokens
        let text = "a".repeat(100);
        assert_eq!(tokenizer.count(&text), 28);
    }

    #[test]
    fn test_heuristic_tokenizer_unknown() {
        let tokenizer = HeuristicTokenizer::for_model("some-random-model");
        assert_eq!(tokenizer.model_family(), "unknown");
        assert_eq!(tokenizer.ratio(), 0.30);
    }

    #[test]
    fn test_heuristic_tokenizer_custom() {
        let tokenizer = HeuristicTokenizer::with_ratio(0.5, "custom");
        assert_eq!(tokenizer.model_family(), "custom");
        assert_eq!(tokenizer.ratio(), 0.5);

        // 100 chars * 0.5 = 50 tokens
        let text = "a".repeat(100);
        assert_eq!(tokenizer.count(&text), 50);
    }

    #[test]
    fn test_estimate_tokens_legacy() {
        // Legacy function uses default (GPT-4) ratio
        let text = "a".repeat(100);
        assert_eq!(estimate_tokens(&text), 25);
    }

    #[test]
    fn test_tokenizer_trait_object() {
        // Verify it can be used as a trait object
        let tokenizer: Box<dyn Tokenizer> = Box::new(HeuristicTokenizer::default());
        assert!(!tokenizer.model_family().is_empty());
    }
}
