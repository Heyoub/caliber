//! Enum types for CALIBER entities

use crate::DurationMs;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ============================================================================
// CORE ENUMS
// ============================================================================

/// Time-to-live and retention configuration for memory entries.
/// Supports both time-based expiration and count-based limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TTL {
    /// Never expires
    Persistent,
    /// Expires when session ends
    Session,
    /// Expires when scope closes
    Scope,
    /// Expires after specified duration in milliseconds
    Duration(DurationMs),
    /// Ephemeral - expires when scope closes (alias for Scope)
    Ephemeral,
    /// Short-term retention (~1 hour)
    ShortTerm,
    /// Medium-term retention (~24 hours)
    MediumTerm,
    /// Long-term retention (~7 days)
    LongTerm,
    /// Permanent - never expires (alias for Persistent)
    Permanent,
    /// Keep at most N entries (count-based retention from DSL)
    Max(usize),
}

/// Entity type discriminator for polymorphic references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum EntityType {
    Trajectory,
    Scope,
    Artifact,
    Note,
    Turn,
    Lock,
    Message,
    Agent,
    Delegation,
    Handoff,
    Conflict,
    Edge,
    EvolutionSnapshot,
    SummarizationPolicy,
}

/// Memory category for hierarchical memory organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum MemoryCategory {
    Ephemeral,
    Working,
    Episodic,
    Semantic,
    Procedural,
    Meta,
}

/// Field types for schema definitions.
/// Used by DSL compiler and runtime validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum FieldType {
    /// UUID identifier
    Uuid,
    /// Text/string
    Text,
    /// Integer
    Int,
    /// Floating point
    Float,
    /// Boolean
    Bool,
    /// Timestamp
    Timestamp,
    /// JSON blob
    Json,
    /// Vector embedding with optional dimension hint
    Embedding { dimensions: Option<usize> },
    /// Enumeration with named variants
    Enum { variants: Vec<String> },
    /// Array of another field type
    Array(Box<FieldType>),
}

/// Status of a trajectory (task container).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TrajectoryStatus {
    Active,
    Completed,
    Failed,
    Suspended,
}

/// Outcome status for completed trajectories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum OutcomeStatus {
    Success,
    Partial,
    Failure,
}

/// Status of an agent in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum AgentStatus {
    /// Agent is registered but not actively working
    #[default]
    Idle,
    /// Agent is actively processing a task
    Active,
    /// Agent is blocked waiting on something (lock, delegation, etc.)
    Blocked,
    /// Agent has failed and requires attention
    Failed,
    /// Agent has been unregistered
    Offline,
}

impl AgentStatus {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            AgentStatus::Idle => "Idle",
            AgentStatus::Active => "Active",
            AgentStatus::Blocked => "Blocked",
            AgentStatus::Failed => "Failed",
            AgentStatus::Offline => "Offline",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, AgentStatusParseError> {
        match s.to_lowercase().as_str() {
            "idle" => Ok(AgentStatus::Idle),
            "active" => Ok(AgentStatus::Active),
            "blocked" => Ok(AgentStatus::Blocked),
            "failed" => Ok(AgentStatus::Failed),
            "offline" => Ok(AgentStatus::Offline),
            _ => Err(AgentStatusParseError(s.to_string())),
        }
    }

    /// Check if the agent can accept new work.
    pub fn can_accept_work(&self) -> bool {
        matches!(self, AgentStatus::Idle)
    }
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for AgentStatus {
    type Err = AgentStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid agent status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStatusParseError(pub String);

impl fmt::Display for AgentStatusParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid agent status: {}", self.0)
    }
}

impl std::error::Error for AgentStatusParseError {}

/// Role of a turn in conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TurnRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Type of artifact produced during a trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ArtifactType {
    ErrorLog,
    CodePatch,
    DesignDecision,
    UserPreference,
    Fact,
    Constraint,
    ToolResult,
    IntermediateOutput,
    Custom,
    Code,
    Document,
    Data,
    Model,
    Config,
    Log,
    Summary,
    Decision,
    Plan,
}

/// Method used to extract an artifact or evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ExtractionMethod {
    /// Explicitly provided by user
    Explicit,
    /// Inferred from context
    Inferred,
    /// User provided directly
    UserProvided,
    /// Extracted by LLM
    LlmExtraction,
    /// Extracted by tool/function
    ToolExtraction,
    /// From memory recall
    MemoryRecall,
    /// From external API
    ExternalApi,
    /// Unknown or unspecified
    #[default]
    Unknown,
}

/// Type of note (cross-trajectory knowledge).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum NoteType {
    Convention,
    Strategy,
    Gotcha,
    Fact,
    Preference,
    Relationship,
    Procedure,
    Meta,
    Insight,
    Correction,
    Summary,
}

// ============================================================================
// BATTLE INTEL ENUMS
// ============================================================================

/// Type of edge relationship between entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum EdgeType {
    Supports,
    Contradicts,
    Supersedes,
    DerivedFrom,
    RelatesTo,
    Temporal,
    Causal,
    SynthesizedFrom,
    Grouped,
    Compared,
}

/// Semantic abstraction level for notes (L0 → L1 → L2 hierarchy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum AbstractionLevel {
    #[default]
    Raw,
    Summary,
    Principle,
}

/// Phase of DSL config evolution cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum EvolutionPhase {
    #[default]
    Online,
    Frozen,
    Evolving,
}

/// Trigger condition for auto-summarization policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum SummarizationTrigger {
    DosageThreshold { percent: u8 },
    ScopeClose,
    TurnCount { count: i32 },
    ArtifactCount { count: i32 },
    Manual,
}

// ============================================================================
// STRING CONVERSIONS
// ============================================================================

fn normalize_token(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            EntityType::Trajectory => "Trajectory",
            EntityType::Scope => "Scope",
            EntityType::Artifact => "Artifact",
            EntityType::Note => "Note",
            EntityType::Turn => "Turn",
            EntityType::Lock => "Lock",
            EntityType::Message => "Message",
            EntityType::Agent => "Agent",
            EntityType::Delegation => "Delegation",
            EntityType::Handoff => "Handoff",
            EntityType::Conflict => "Conflict",
            EntityType::Edge => "Edge",
            EntityType::EvolutionSnapshot => "EvolutionSnapshot",
            EntityType::SummarizationPolicy => "SummarizationPolicy",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for EntityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = normalize_token(s);
        match normalized.as_str() {
            "trajectory" => Ok(EntityType::Trajectory),
            "scope" => Ok(EntityType::Scope),
            "artifact" => Ok(EntityType::Artifact),
            "note" => Ok(EntityType::Note),
            "turn" => Ok(EntityType::Turn),
            "lock" => Ok(EntityType::Lock),
            "message" => Ok(EntityType::Message),
            "agent" => Ok(EntityType::Agent),
            "delegation" => Ok(EntityType::Delegation),
            "handoff" => Ok(EntityType::Handoff),
            "conflict" => Ok(EntityType::Conflict),
            "edge" => Ok(EntityType::Edge),
            "evolutionsnapshot" => Ok(EntityType::EvolutionSnapshot),
            "summarizationpolicy" => Ok(EntityType::SummarizationPolicy),
            _ => Err(format!("Invalid EntityType: {}", s)),
        }
    }
}

impl fmt::Display for TrajectoryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            TrajectoryStatus::Active => "Active",
            TrajectoryStatus::Completed => "Completed",
            TrajectoryStatus::Failed => "Failed",
            TrajectoryStatus::Suspended => "Suspended",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for TrajectoryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_token(s).as_str() {
            "active" => Ok(TrajectoryStatus::Active),
            "completed" | "complete" => Ok(TrajectoryStatus::Completed),
            "failed" | "failure" => Ok(TrajectoryStatus::Failed),
            "suspended" => Ok(TrajectoryStatus::Suspended),
            _ => Err(format!("Invalid TrajectoryStatus: {}", s)),
        }
    }
}

impl fmt::Display for OutcomeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            OutcomeStatus::Success => "Success",
            OutcomeStatus::Partial => "Partial",
            OutcomeStatus::Failure => "Failure",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for OutcomeStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_token(s).as_str() {
            "success" => Ok(OutcomeStatus::Success),
            "partial" => Ok(OutcomeStatus::Partial),
            "failure" | "failed" => Ok(OutcomeStatus::Failure),
            _ => Err(format!("Invalid OutcomeStatus: {}", s)),
        }
    }
}

impl fmt::Display for TurnRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            TurnRole::User => "User",
            TurnRole::Assistant => "Assistant",
            TurnRole::System => "System",
            TurnRole::Tool => "Tool",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for TurnRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_token(s).as_str() {
            "user" => Ok(TurnRole::User),
            "assistant" => Ok(TurnRole::Assistant),
            "system" => Ok(TurnRole::System),
            "tool" => Ok(TurnRole::Tool),
            _ => Err(format!("Invalid TurnRole: {}", s)),
        }
    }
}

impl fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            ArtifactType::ErrorLog => "ErrorLog",
            ArtifactType::CodePatch => "CodePatch",
            ArtifactType::DesignDecision => "DesignDecision",
            ArtifactType::UserPreference => "UserPreference",
            ArtifactType::Fact => "Fact",
            ArtifactType::Constraint => "Constraint",
            ArtifactType::ToolResult => "ToolResult",
            ArtifactType::IntermediateOutput => "IntermediateOutput",
            ArtifactType::Custom => "Custom",
            ArtifactType::Code => "Code",
            ArtifactType::Document => "Document",
            ArtifactType::Data => "Data",
            ArtifactType::Model => "Model",
            ArtifactType::Config => "Config",
            ArtifactType::Log => "Log",
            ArtifactType::Summary => "Summary",
            ArtifactType::Decision => "Decision",
            ArtifactType::Plan => "Plan",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for ArtifactType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_token(s).as_str() {
            "errorlog" => Ok(ArtifactType::ErrorLog),
            "codepatch" => Ok(ArtifactType::CodePatch),
            "designdecision" => Ok(ArtifactType::DesignDecision),
            "userpreference" => Ok(ArtifactType::UserPreference),
            "fact" => Ok(ArtifactType::Fact),
            "constraint" => Ok(ArtifactType::Constraint),
            "toolresult" => Ok(ArtifactType::ToolResult),
            "intermediateoutput" => Ok(ArtifactType::IntermediateOutput),
            "custom" => Ok(ArtifactType::Custom),
            "code" => Ok(ArtifactType::Code),
            "document" => Ok(ArtifactType::Document),
            "data" => Ok(ArtifactType::Data),
            "model" => Ok(ArtifactType::Model),
            "config" => Ok(ArtifactType::Config),
            "log" => Ok(ArtifactType::Log),
            "summary" => Ok(ArtifactType::Summary),
            "decision" => Ok(ArtifactType::Decision),
            "plan" => Ok(ArtifactType::Plan),
            _ => Err(format!("Invalid ArtifactType: {}", s)),
        }
    }
}

impl fmt::Display for ExtractionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            ExtractionMethod::Explicit => "Explicit",
            ExtractionMethod::Inferred => "Inferred",
            ExtractionMethod::UserProvided => "UserProvided",
            ExtractionMethod::LlmExtraction => "LlmExtraction",
            ExtractionMethod::ToolExtraction => "ToolExtraction",
            ExtractionMethod::MemoryRecall => "MemoryRecall",
            ExtractionMethod::ExternalApi => "ExternalApi",
            ExtractionMethod::Unknown => "Unknown",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for ExtractionMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_token(s).as_str() {
            "explicit" => Ok(ExtractionMethod::Explicit),
            "inferred" => Ok(ExtractionMethod::Inferred),
            "userprovided" => Ok(ExtractionMethod::UserProvided),
            "llmextraction" => Ok(ExtractionMethod::LlmExtraction),
            "toolextraction" => Ok(ExtractionMethod::ToolExtraction),
            "memoryrecall" => Ok(ExtractionMethod::MemoryRecall),
            "externalapi" => Ok(ExtractionMethod::ExternalApi),
            "unknown" => Ok(ExtractionMethod::Unknown),
            _ => Err(format!("Invalid ExtractionMethod: {}", s)),
        }
    }
}

impl fmt::Display for NoteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            NoteType::Convention => "Convention",
            NoteType::Strategy => "Strategy",
            NoteType::Gotcha => "Gotcha",
            NoteType::Fact => "Fact",
            NoteType::Preference => "Preference",
            NoteType::Relationship => "Relationship",
            NoteType::Procedure => "Procedure",
            NoteType::Meta => "Meta",
            NoteType::Insight => "Insight",
            NoteType::Correction => "Correction",
            NoteType::Summary => "Summary",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for NoteType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_token(s).as_str() {
            "convention" => Ok(NoteType::Convention),
            "strategy" => Ok(NoteType::Strategy),
            "gotcha" => Ok(NoteType::Gotcha),
            "fact" => Ok(NoteType::Fact),
            "preference" => Ok(NoteType::Preference),
            "relationship" => Ok(NoteType::Relationship),
            "procedure" => Ok(NoteType::Procedure),
            "meta" => Ok(NoteType::Meta),
            "insight" => Ok(NoteType::Insight),
            "correction" => Ok(NoteType::Correction),
            "summary" => Ok(NoteType::Summary),
            _ => Err(format!("Invalid NoteType: {}", s)),
        }
    }
}

impl fmt::Display for EdgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            EdgeType::Supports => "Supports",
            EdgeType::Contradicts => "Contradicts",
            EdgeType::Supersedes => "Supersedes",
            EdgeType::DerivedFrom => "DerivedFrom",
            EdgeType::RelatesTo => "RelatesTo",
            EdgeType::Temporal => "Temporal",
            EdgeType::Causal => "Causal",
            EdgeType::SynthesizedFrom => "SynthesizedFrom",
            EdgeType::Grouped => "Grouped",
            EdgeType::Compared => "Compared",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for EdgeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_token(s).as_str() {
            "supports" => Ok(EdgeType::Supports),
            "contradicts" => Ok(EdgeType::Contradicts),
            "supersedes" => Ok(EdgeType::Supersedes),
            "derivedfrom" => Ok(EdgeType::DerivedFrom),
            "relatesto" => Ok(EdgeType::RelatesTo),
            "temporal" => Ok(EdgeType::Temporal),
            "causal" => Ok(EdgeType::Causal),
            "synthesizedfrom" => Ok(EdgeType::SynthesizedFrom),
            "grouped" => Ok(EdgeType::Grouped),
            "compared" => Ok(EdgeType::Compared),
            _ => Err(format!("Invalid EdgeType: {}", s)),
        }
    }
}

impl fmt::Display for AbstractionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            AbstractionLevel::Raw => "Raw",
            AbstractionLevel::Summary => "Summary",
            AbstractionLevel::Principle => "Principle",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for AbstractionLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_token(s).as_str() {
            "raw" | "l0" => Ok(AbstractionLevel::Raw),
            "summary" | "l1" => Ok(AbstractionLevel::Summary),
            "principle" | "l2" => Ok(AbstractionLevel::Principle),
            _ => Err(format!("Invalid AbstractionLevel: {}", s)),
        }
    }
}

impl fmt::Display for EvolutionPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            EvolutionPhase::Online => "Online",
            EvolutionPhase::Frozen => "Frozen",
            EvolutionPhase::Evolving => "Evolving",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for EvolutionPhase {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_token(s).as_str() {
            "online" => Ok(EvolutionPhase::Online),
            "frozen" | "freeze" => Ok(EvolutionPhase::Frozen),
            "evolving" | "evolve" => Ok(EvolutionPhase::Evolving),
            _ => Err(format!("Invalid EvolutionPhase: {}", s)),
        }
    }
}

impl fmt::Display for SummarizationTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SummarizationTrigger::DosageThreshold { percent } => {
                write!(f, "DosageThreshold({}%)", percent)
            }
            SummarizationTrigger::ScopeClose => write!(f, "ScopeClose"),
            SummarizationTrigger::TurnCount { count } => write!(f, "TurnCount({})", count),
            SummarizationTrigger::ArtifactCount { count } => write!(f, "ArtifactCount({})", count),
            SummarizationTrigger::Manual => write!(f, "Manual"),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Serde Roundtrip Tests - TTL
    // ========================================================================

    #[test]
    fn test_ttl_persistent_serde_roundtrip() {
        let original = TTL::Persistent;
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_ttl_session_serde_roundtrip() {
        let original = TTL::Session;
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_ttl_scope_serde_roundtrip() {
        let original = TTL::Scope;
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_ttl_duration_serde_roundtrip() {
        let original = TTL::Duration(3600000);
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_ttl_ephemeral_serde_roundtrip() {
        let original = TTL::Ephemeral;
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_ttl_max_serde_roundtrip() {
        let original = TTL::Max(1000);
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_ttl_short_term_serde_roundtrip() {
        let original = TTL::ShortTerm;
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_ttl_medium_term_serde_roundtrip() {
        let original = TTL::MediumTerm;
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_ttl_long_term_serde_roundtrip() {
        let original = TTL::LongTerm;
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_ttl_permanent_serde_roundtrip() {
        let original = TTL::Permanent;
        let json = serde_json::to_string(&original).unwrap();
        let restored: TTL = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // ========================================================================
    // Serde Roundtrip Tests - EntityType
    // ========================================================================

    #[test]
    fn test_entity_type_all_variants_serde_roundtrip() {
        let variants = [
            EntityType::Trajectory,
            EntityType::Scope,
            EntityType::Artifact,
            EntityType::Note,
            EntityType::Turn,
            EntityType::Lock,
            EntityType::Message,
            EntityType::Agent,
            EntityType::Delegation,
            EntityType::Handoff,
            EntityType::Conflict,
            EntityType::Edge,
            EntityType::EvolutionSnapshot,
            EntityType::SummarizationPolicy,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: EntityType = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored, "Failed for {:?}", original);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - TrajectoryStatus
    // ========================================================================

    #[test]
    fn test_trajectory_status_all_variants_serde_roundtrip() {
        let variants = [
            TrajectoryStatus::Active,
            TrajectoryStatus::Completed,
            TrajectoryStatus::Failed,
            TrajectoryStatus::Suspended,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: TrajectoryStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - AgentStatus
    // ========================================================================

    #[test]
    fn test_agent_status_all_variants_serde_roundtrip() {
        let variants = [
            AgentStatus::Idle,
            AgentStatus::Active,
            AgentStatus::Blocked,
            AgentStatus::Failed,
            AgentStatus::Offline,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: AgentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - ArtifactType
    // ========================================================================

    #[test]
    fn test_artifact_type_all_variants_serde_roundtrip() {
        let variants = [
            ArtifactType::ErrorLog,
            ArtifactType::CodePatch,
            ArtifactType::DesignDecision,
            ArtifactType::UserPreference,
            ArtifactType::Fact,
            ArtifactType::Constraint,
            ArtifactType::ToolResult,
            ArtifactType::IntermediateOutput,
            ArtifactType::Custom,
            ArtifactType::Code,
            ArtifactType::Document,
            ArtifactType::Data,
            ArtifactType::Model,
            ArtifactType::Config,
            ArtifactType::Log,
            ArtifactType::Summary,
            ArtifactType::Decision,
            ArtifactType::Plan,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: ArtifactType = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - ExtractionMethod
    // ========================================================================

    #[test]
    fn test_extraction_method_all_variants_serde_roundtrip() {
        let variants = [
            ExtractionMethod::Explicit,
            ExtractionMethod::Inferred,
            ExtractionMethod::UserProvided,
            ExtractionMethod::LlmExtraction,
            ExtractionMethod::ToolExtraction,
            ExtractionMethod::MemoryRecall,
            ExtractionMethod::ExternalApi,
            ExtractionMethod::Unknown,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: ExtractionMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - NoteType
    // ========================================================================

    #[test]
    fn test_note_type_all_variants_serde_roundtrip() {
        let variants = [
            NoteType::Convention,
            NoteType::Strategy,
            NoteType::Gotcha,
            NoteType::Fact,
            NoteType::Preference,
            NoteType::Relationship,
            NoteType::Procedure,
            NoteType::Meta,
            NoteType::Insight,
            NoteType::Correction,
            NoteType::Summary,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: NoteType = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - EdgeType
    // ========================================================================

    #[test]
    fn test_edge_type_all_variants_serde_roundtrip() {
        let variants = [
            EdgeType::Supports,
            EdgeType::Contradicts,
            EdgeType::Supersedes,
            EdgeType::DerivedFrom,
            EdgeType::RelatesTo,
            EdgeType::Temporal,
            EdgeType::Causal,
            EdgeType::SynthesizedFrom,
            EdgeType::Grouped,
            EdgeType::Compared,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: EdgeType = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - AbstractionLevel
    // ========================================================================

    #[test]
    fn test_abstraction_level_all_variants_serde_roundtrip() {
        let variants = [
            AbstractionLevel::Raw,
            AbstractionLevel::Summary,
            AbstractionLevel::Principle,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: AbstractionLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - EvolutionPhase
    // ========================================================================

    #[test]
    fn test_evolution_phase_all_variants_serde_roundtrip() {
        let variants = [
            EvolutionPhase::Online,
            EvolutionPhase::Frozen,
            EvolutionPhase::Evolving,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: EvolutionPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - SummarizationTrigger
    // ========================================================================

    #[test]
    fn test_summarization_trigger_all_variants_serde_roundtrip() {
        let variants = [
            SummarizationTrigger::DosageThreshold { percent: 80 },
            SummarizationTrigger::ScopeClose,
            SummarizationTrigger::TurnCount { count: 10 },
            SummarizationTrigger::ArtifactCount { count: 5 },
            SummarizationTrigger::Manual,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: SummarizationTrigger = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - MemoryCategory
    // ========================================================================

    #[test]
    fn test_memory_category_all_variants_serde_roundtrip() {
        let variants = [
            MemoryCategory::Ephemeral,
            MemoryCategory::Working,
            MemoryCategory::Episodic,
            MemoryCategory::Semantic,
            MemoryCategory::Procedural,
            MemoryCategory::Meta,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: MemoryCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - TurnRole
    // ========================================================================

    #[test]
    fn test_turn_role_all_variants_serde_roundtrip() {
        let variants = [
            TurnRole::User,
            TurnRole::Assistant,
            TurnRole::System,
            TurnRole::Tool,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: TurnRole = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - OutcomeStatus
    // ========================================================================

    #[test]
    fn test_outcome_status_all_variants_serde_roundtrip() {
        let variants = [
            OutcomeStatus::Success,
            OutcomeStatus::Partial,
            OutcomeStatus::Failure,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: OutcomeStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // Serde Roundtrip Tests - FieldType
    // ========================================================================

    #[test]
    fn test_field_type_simple_variants_serde_roundtrip() {
        let variants = [
            FieldType::Uuid,
            FieldType::Text,
            FieldType::Int,
            FieldType::Float,
            FieldType::Bool,
            FieldType::Timestamp,
            FieldType::Json,
        ];

        for original in variants {
            let json = serde_json::to_string(&original).unwrap();
            let restored: FieldType = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_field_type_embedding_serde_roundtrip() {
        let original = FieldType::Embedding { dimensions: Some(1536) };
        let json = serde_json::to_string(&original).unwrap();
        let restored: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_field_type_enum_serde_roundtrip() {
        let original = FieldType::Enum {
            variants: vec!["A".to_string(), "B".to_string(), "C".to_string()],
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_field_type_array_serde_roundtrip() {
        let original = FieldType::Array(Box::new(FieldType::Text));
        let json = serde_json::to_string(&original).unwrap();
        let restored: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // ========================================================================
    // Display/FromStr Roundtrip Tests
    // ========================================================================

    #[test]
    fn test_entity_type_display_fromstr_roundtrip() {
        let variants = [
            EntityType::Trajectory,
            EntityType::Scope,
            EntityType::Artifact,
            EntityType::Note,
            EntityType::Turn,
            EntityType::Lock,
            EntityType::Message,
            EntityType::Agent,
            EntityType::Delegation,
            EntityType::Handoff,
            EntityType::Conflict,
            EntityType::Edge,
            EntityType::EvolutionSnapshot,
            EntityType::SummarizationPolicy,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: EntityType = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_trajectory_status_display_fromstr_roundtrip() {
        let variants = [
            TrajectoryStatus::Active,
            TrajectoryStatus::Completed,
            TrajectoryStatus::Failed,
            TrajectoryStatus::Suspended,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: TrajectoryStatus = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_agent_status_display_fromstr_roundtrip() {
        let variants = [
            AgentStatus::Idle,
            AgentStatus::Active,
            AgentStatus::Blocked,
            AgentStatus::Failed,
            AgentStatus::Offline,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: AgentStatus = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_artifact_type_display_fromstr_roundtrip() {
        let variants = [
            ArtifactType::ErrorLog,
            ArtifactType::CodePatch,
            ArtifactType::DesignDecision,
            ArtifactType::UserPreference,
            ArtifactType::Fact,
            ArtifactType::Constraint,
            ArtifactType::ToolResult,
            ArtifactType::IntermediateOutput,
            ArtifactType::Custom,
            ArtifactType::Code,
            ArtifactType::Document,
            ArtifactType::Data,
            ArtifactType::Model,
            ArtifactType::Config,
            ArtifactType::Log,
            ArtifactType::Summary,
            ArtifactType::Decision,
            ArtifactType::Plan,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: ArtifactType = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_extraction_method_display_fromstr_roundtrip() {
        let variants = [
            ExtractionMethod::Explicit,
            ExtractionMethod::Inferred,
            ExtractionMethod::UserProvided,
            ExtractionMethod::LlmExtraction,
            ExtractionMethod::ToolExtraction,
            ExtractionMethod::MemoryRecall,
            ExtractionMethod::ExternalApi,
            ExtractionMethod::Unknown,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: ExtractionMethod = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_note_type_display_fromstr_roundtrip() {
        let variants = [
            NoteType::Convention,
            NoteType::Strategy,
            NoteType::Gotcha,
            NoteType::Fact,
            NoteType::Preference,
            NoteType::Relationship,
            NoteType::Procedure,
            NoteType::Meta,
            NoteType::Insight,
            NoteType::Correction,
            NoteType::Summary,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: NoteType = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_edge_type_display_fromstr_roundtrip() {
        let variants = [
            EdgeType::Supports,
            EdgeType::Contradicts,
            EdgeType::Supersedes,
            EdgeType::DerivedFrom,
            EdgeType::RelatesTo,
            EdgeType::Temporal,
            EdgeType::Causal,
            EdgeType::SynthesizedFrom,
            EdgeType::Grouped,
            EdgeType::Compared,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: EdgeType = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_abstraction_level_display_fromstr_roundtrip() {
        let variants = [
            AbstractionLevel::Raw,
            AbstractionLevel::Summary,
            AbstractionLevel::Principle,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: AbstractionLevel = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_evolution_phase_display_fromstr_roundtrip() {
        let variants = [
            EvolutionPhase::Online,
            EvolutionPhase::Frozen,
            EvolutionPhase::Evolving,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: EvolutionPhase = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_turn_role_display_fromstr_roundtrip() {
        let variants = [
            TurnRole::User,
            TurnRole::Assistant,
            TurnRole::System,
            TurnRole::Tool,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: TurnRole = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn test_outcome_status_display_fromstr_roundtrip() {
        let variants = [
            OutcomeStatus::Success,
            OutcomeStatus::Partial,
            OutcomeStatus::Failure,
        ];

        for original in variants {
            let string = original.to_string();
            let restored: OutcomeStatus = string.parse().unwrap();
            assert_eq!(original, restored);
        }
    }

    // ========================================================================
    // FromStr with Aliases Tests
    // ========================================================================

    #[test]
    fn test_abstraction_level_aliases() {
        // "raw" and "l0" should both parse to Raw
        assert_eq!("raw".parse::<AbstractionLevel>().unwrap(), AbstractionLevel::Raw);
        assert_eq!("l0".parse::<AbstractionLevel>().unwrap(), AbstractionLevel::Raw);

        // "summary" and "l1" should both parse to Summary
        assert_eq!("summary".parse::<AbstractionLevel>().unwrap(), AbstractionLevel::Summary);
        assert_eq!("l1".parse::<AbstractionLevel>().unwrap(), AbstractionLevel::Summary);

        // "principle" and "l2" should both parse to Principle
        assert_eq!("principle".parse::<AbstractionLevel>().unwrap(), AbstractionLevel::Principle);
        assert_eq!("l2".parse::<AbstractionLevel>().unwrap(), AbstractionLevel::Principle);
    }

    #[test]
    fn test_trajectory_status_aliases() {
        assert_eq!("completed".parse::<TrajectoryStatus>().unwrap(), TrajectoryStatus::Completed);
        assert_eq!("complete".parse::<TrajectoryStatus>().unwrap(), TrajectoryStatus::Completed);
        assert_eq!("failed".parse::<TrajectoryStatus>().unwrap(), TrajectoryStatus::Failed);
        assert_eq!("failure".parse::<TrajectoryStatus>().unwrap(), TrajectoryStatus::Failed);
    }

    #[test]
    fn test_evolution_phase_aliases() {
        assert_eq!("frozen".parse::<EvolutionPhase>().unwrap(), EvolutionPhase::Frozen);
        assert_eq!("freeze".parse::<EvolutionPhase>().unwrap(), EvolutionPhase::Frozen);
        assert_eq!("evolving".parse::<EvolutionPhase>().unwrap(), EvolutionPhase::Evolving);
        assert_eq!("evolve".parse::<EvolutionPhase>().unwrap(), EvolutionPhase::Evolving);
    }

    // ========================================================================
    // Default Implementation Tests
    // ========================================================================

    #[test]
    fn test_agent_status_default() {
        assert_eq!(AgentStatus::default(), AgentStatus::Idle);
    }

    #[test]
    fn test_extraction_method_default() {
        assert_eq!(ExtractionMethod::default(), ExtractionMethod::Unknown);
    }

    #[test]
    fn test_abstraction_level_default() {
        assert_eq!(AbstractionLevel::default(), AbstractionLevel::Raw);
    }

    #[test]
    fn test_evolution_phase_default() {
        assert_eq!(EvolutionPhase::default(), EvolutionPhase::Online);
    }

    // ========================================================================
    // AgentStatus Business Logic Tests
    // ========================================================================

    #[test]
    fn test_agent_status_can_accept_work() {
        assert!(AgentStatus::Idle.can_accept_work());
        assert!(!AgentStatus::Active.can_accept_work());
        assert!(!AgentStatus::Blocked.can_accept_work());
        assert!(!AgentStatus::Failed.can_accept_work());
        assert!(!AgentStatus::Offline.can_accept_work());
    }

    #[test]
    fn test_agent_status_as_db_str() {
        assert_eq!(AgentStatus::Idle.as_db_str(), "Idle");
        assert_eq!(AgentStatus::Active.as_db_str(), "Active");
        assert_eq!(AgentStatus::Blocked.as_db_str(), "Blocked");
        assert_eq!(AgentStatus::Failed.as_db_str(), "Failed");
        assert_eq!(AgentStatus::Offline.as_db_str(), "Offline");
    }

    #[test]
    fn test_agent_status_from_db_str() {
        assert_eq!(AgentStatus::from_db_str("idle").unwrap(), AgentStatus::Idle);
        assert_eq!(AgentStatus::from_db_str("ACTIVE").unwrap(), AgentStatus::Active);
        assert_eq!(AgentStatus::from_db_str("Blocked").unwrap(), AgentStatus::Blocked);
        assert!(AgentStatus::from_db_str("invalid").is_err());
    }

    // ========================================================================
    // Invalid Parse Tests
    // ========================================================================

    #[test]
    fn test_entity_type_invalid_parse() {
        assert!("invalid".parse::<EntityType>().is_err());
        assert!("".parse::<EntityType>().is_err());
    }

    #[test]
    fn test_trajectory_status_invalid_parse() {
        assert!("invalid".parse::<TrajectoryStatus>().is_err());
    }

    #[test]
    fn test_artifact_type_invalid_parse() {
        assert!("invalid".parse::<ArtifactType>().is_err());
    }

    #[test]
    fn test_note_type_invalid_parse() {
        assert!("invalid".parse::<NoteType>().is_err());
    }

    #[test]
    fn test_edge_type_invalid_parse() {
        assert!("invalid".parse::<EdgeType>().is_err());
    }

    // ========================================================================
    // Case Insensitivity Tests
    // ========================================================================

    #[test]
    fn test_parse_case_insensitive() {
        assert_eq!("ACTIVE".parse::<TrajectoryStatus>().unwrap(), TrajectoryStatus::Active);
        assert_eq!("active".parse::<TrajectoryStatus>().unwrap(), TrajectoryStatus::Active);
        assert_eq!("Active".parse::<TrajectoryStatus>().unwrap(), TrajectoryStatus::Active);
        assert_eq!("TRAJECTORY".parse::<EntityType>().unwrap(), EntityType::Trajectory);
        assert_eq!("trajectory".parse::<EntityType>().unwrap(), EntityType::Trajectory);
    }

    // ========================================================================
    // Error Type Tests
    // ========================================================================

    #[test]
    fn test_agent_status_parse_error_display() {
        let err = AgentStatusParseError("invalid_status".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("invalid_status"));
        assert!(msg.contains("Invalid agent status"));
    }
}
