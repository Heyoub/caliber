//! LLM-related primitive types.
//!
//! Pure data types for LLM operations. Traits and orchestration live in caliber-llm.

use crate::ArtifactType;
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
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

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
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
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_style_roundtrip() {
        for style in [SummarizeStyle::Brief, SummarizeStyle::Detailed, SummarizeStyle::Structured] {
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
        for state in [CircuitState::Closed, CircuitState::Open, CircuitState::HalfOpen] {
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
}
