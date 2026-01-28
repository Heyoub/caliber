//! Battle Intel: Summarization policy types

use caliber_core::{
    AbstractionLevel, SummarizationPolicyId, SummarizationTrigger, TenantId, Timestamp,
    TrajectoryId,
};
use serde::{Deserialize, Serialize};

/// Request to create a summarization policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateSummarizationPolicyRequest {
    /// Policy name
    pub name: String,
    /// Triggers that fire this policy
    pub triggers: Vec<SummarizationTrigger>,
    /// Source abstraction level (e.g., Raw/L0)
    pub source_level: AbstractionLevel,
    /// Target abstraction level (e.g., Summary/L1)
    pub target_level: AbstractionLevel,
    /// Maximum sources to summarize at once
    pub max_sources: i32,
    /// Whether to create SynthesizedFrom edges
    pub create_edges: bool,
    /// Optional trajectory ID to scope this policy
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<TrajectoryId>,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Response for a summarization policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SummarizationPolicyResponse {
    /// Policy ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub policy_id: SummarizationPolicyId,
    /// Tenant this policy belongs to (for multi-tenant isolation)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub tenant_id: Option<TenantId>,
    /// Policy name
    pub name: String,
    /// Triggers that fire this policy
    pub triggers: Vec<SummarizationTrigger>,
    /// Source abstraction level
    pub source_level: AbstractionLevel,
    /// Target abstraction level
    pub target_level: AbstractionLevel,
    /// Maximum sources to summarize at once
    pub max_sources: i32,
    /// Whether to create SynthesizedFrom edges
    pub create_edges: bool,
    /// Trajectory ID if scoped
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub trajectory_id: Option<TrajectoryId>,
    /// When the policy was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
    /// Optional metadata
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub metadata: Option<serde_json::Value>,
}

/// Response containing a list of summarization policies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListSummarizationPoliciesResponse {
    /// List of policies
    pub policies: Vec<SummarizationPolicyResponse>,
}
