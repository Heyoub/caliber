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
