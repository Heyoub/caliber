//! Battle Intel features: Edges, Evolution, Summarization

use crate::*;
use serde::{Deserialize, Serialize};

/// Participant in an edge with optional role.
/// Enables both binary edges (2 participants) and hyperedges (N participants).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EdgeParticipant {
    /// Reference to the entity participating in this edge
    pub entity_ref: EntityRef,
    /// Optional role label (e.g., "source", "target", "input", "output")
    pub role: Option<String>,
}

/// Edge - graph relationship between entities.
/// Supports both binary edges (A→B) and hyperedges (N-ary relationships).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Edge {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub edge_id: EntityId,
    pub edge_type: EdgeType,
    /// Participants in this edge (len=2 for binary, len>2 for hyperedge)
    pub participants: Vec<EdgeParticipant>,
    /// Optional relationship strength [0.0, 1.0]
    pub weight: Option<f32>,
    /// Optional trajectory context
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<EntityId>,
    /// How this edge was created
    pub provenance: Provenance,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

impl Edge {
    /// Check if this is a binary edge (exactly 2 participants)
    pub fn is_binary(&self) -> bool {
        self.participants.len() == 2
    }

    /// Check if this is a hyperedge (more than 2 participants)
    pub fn is_hyperedge(&self) -> bool {
        self.participants.len() > 2
    }

    /// Get participants with a specific role
    pub fn participants_with_role(&self, role: &str) -> Vec<&EdgeParticipant> {
        self.participants
            .iter()
            .filter(|p| p.role.as_deref() == Some(role))
            .collect()
    }
}

// ============================================================================
// EVOLUTION ENTITIES (Battle Intel Feature 3)
// ============================================================================

/// Benchmark metrics from an evolution run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EvolutionMetrics {
    /// How relevant were the retrievals? [0.0, 1.0]
    pub retrieval_accuracy: f32,
    /// Tokens used vs budget ratio [0.0, 1.0]
    pub token_efficiency: f32,
    /// 50th percentile latency in milliseconds
    pub latency_p50_ms: i64,
    /// 99th percentile latency in milliseconds
    pub latency_p99_ms: i64,
    /// Estimated cost for the benchmark run
    pub cost_estimate: f32,
    /// Number of queries used in benchmark
    pub benchmark_queries: i32,
}

/// Evolution snapshot for DSL config benchmarking.
/// Captures a frozen state of configuration for A/B testing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EvolutionSnapshot {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub snapshot_id: EntityId,
    /// Human-readable snapshot name
    pub name: String,
    /// SHA-256 hash of the DSL config source
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "byte"))]
    pub config_hash: ContentHash,
    /// The actual DSL configuration text
    pub config_source: String,
    /// Current phase of this snapshot
    pub phase: EvolutionPhase,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// Metrics populated after benchmark completes
    pub metrics: Option<EvolutionMetrics>,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// SUMMARIZATION POLICY (Battle Intel Feature 4)
// ============================================================================

/// Policy for automatic summarization/abstraction.
/// Defines when and how to generate L1/L2 notes from lower levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SummarizationPolicy {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub policy_id: EntityId,
    /// Human-readable policy name
    pub name: String,
    /// Conditions that trigger summarization
    pub triggers: Vec<SummarizationTrigger>,
    /// Target abstraction level to generate (L1 or L2)
    pub target_level: AbstractionLevel,
    /// Source abstraction level to summarize FROM
    pub source_level: AbstractionLevel,
    /// Maximum number of source items to summarize at once
    pub max_sources: i32,
    /// Whether to auto-create SynthesizedFrom edges
    pub create_edges: bool,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

