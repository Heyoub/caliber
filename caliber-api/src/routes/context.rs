//! Context assembly routes.
//!
//! This module provides the `/context/assemble` endpoint that wires the
//! context assembly module (`caliber_core::context`) into the API layer.
//!
//! # Purpose
//!
//! The context assembly endpoint replaces 5+ HTTP calls that SDKs were making
//! with a single call that:
//! 1. Fetches relevant notes based on the user query
//! 2. Fetches recent artifacts from the trajectory
//! 3. Fetches scope summaries for compressed history
//! 4. Assembles everything into a token-budgeted context window
//!
//! # Example
//!
//! ```ignore
//! POST /api/v1/context/assemble
//! {
//!   "trajectory_id": "...",
//!   "scope_id": "...",
//!   "user_input": "What files did we modify yesterday?",
//!   "token_budget": 8000,
//!   "include_notes": true,
//!   "include_artifacts": true,
//!   "max_notes": 10,
//!   "max_artifacts": 5
//! }
//! ```

use crate::components::{ArtifactListFilter, NoteListFilter, ScopeListFilter, TurnListFilter};
use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::auth::AuthContext;
use crate::types::{ArtifactResponse, NoteResponse, ScopeResponse, TrajectoryResponse, TurnResponse};
use axum::{extract::State, Extension, Json};
use caliber_core::{
    AgentId, CaliberConfig, ContextAssembler, ContextPackage, ContextWindow, KernelConfig,
    ScopeId, SessionMarkers, TrajectoryId,
};
use caliber_core::{ContextPersistence, RetryConfig, SectionPriorities, ValidationMode};
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Request to assemble context for an LLM prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AssembleContextRequest {
    /// Trajectory to assemble context for
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub trajectory_id: TrajectoryId,

    /// Scope to assemble context for (optional - auto-selects most recent active scope if not provided)
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub scope_id: Option<ScopeId>,

    /// Current user input/query (used for relevance ranking)
    pub user_input: Option<String>,

    /// Maximum token budget for the assembled context
    pub token_budget: Option<i32>,

    /// Whether to include relevant notes (semantic memory)
    #[serde(default = "default_true")]
    pub include_notes: bool,

    /// Whether to include recent artifacts
    #[serde(default = "default_true")]
    pub include_artifacts: bool,

    /// Whether to include scope summaries (compressed history)
    #[serde(default = "default_true")]
    pub include_history: bool,

    /// Whether to include conversation turns from the scope
    #[serde(default = "default_true")]
    pub include_turns: bool,

    /// Whether to include parent trajectory hierarchy
    #[serde(default)]
    pub include_hierarchy: bool,

    /// Maximum number of notes to include
    pub max_notes: Option<i32>,

    /// Maximum number of artifacts to include
    pub max_artifacts: Option<i32>,

    /// Maximum number of scope summaries to include
    pub max_summaries: Option<i32>,

    /// Maximum number of turns to include
    pub max_turns: Option<i32>,

    /// Optional kernel/persona configuration
    pub kernel_config: Option<KernelConfigRequest>,

    /// Agent ID for multi-agent scenarios
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<AgentId>,

    /// Semantic search query for filtering notes/artifacts by relevance
    pub relevance_query: Option<String>,

    /// Minimum relevance score (0.0-1.0) for semantic filtering
    pub min_relevance: Option<f32>,

    /// Output format for the assembled context
    #[serde(default)]
    pub format: ContextFormat,
}

/// Output format for assembled context.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum ContextFormat {
    /// Markdown format (default)
    #[default]
    Markdown,
    /// XML format
    Xml,
    /// JSON format (returns structured data)
    Json,
}

fn default_true() -> bool {
    true
}

/// Kernel configuration for persona and behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct KernelConfigRequest {
    /// Persona description
    pub persona: Option<String>,
    /// Tone of responses
    pub tone: Option<String>,
    /// Reasoning style preference
    pub reasoning_style: Option<String>,
    /// Domain focus area
    pub domain_focus: Option<String>,
}

impl From<KernelConfigRequest> for KernelConfig {
    fn from(req: KernelConfigRequest) -> Self {
        KernelConfig {
            persona: req.persona,
            tone: req.tone,
            reasoning_style: req.reasoning_style,
            domain_focus: req.domain_focus,
        }
    }
}

/// Response from context assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AssembleContextResponse {
    /// The assembled context as a single string
    pub context: String,

    /// Token count of the assembled context
    pub token_count: i32,

    /// Maximum token budget that was used
    pub max_tokens: i32,

    /// Whether any content was truncated to fit the budget
    pub truncated: bool,

    /// Names of sections included in the context
    pub included_sections: Vec<String>,

    /// Number of notes included in the context
    pub notes_count: i32,

    /// Number of artifacts included in the context
    pub artifacts_count: i32,

    /// Number of turns included in the context
    pub turns_count: i32,

    /// Number of summaries included in the context
    pub summaries_count: i32,

    /// Parent trajectory hierarchy (nearest parent first)
    pub hierarchy: Vec<TrajectoryResponse>,

    /// Detailed breakdown of the context window (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_details: Option<ContextWindow>,
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// Assemble context for an LLM prompt.
///
/// This endpoint replaces multiple SDK calls with a single server-side
/// assembly that respects token budgets and includes relevant context.
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/api/v1/context/assemble",
    request_body = AssembleContextRequest,
    responses(
        (status = 200, description = "Context assembled successfully", body = AssembleContextResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Trajectory or scope not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Context"
))]
pub async fn assemble_context(
    State(db): State<DbClient>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<AssembleContextRequest>,
) -> ApiResult<Json<AssembleContextResponse>> {
    let tenant_id = auth.tenant_id;

    // Validate trajectory exists and belongs to tenant
    let trajectory = db
        .get::<TrajectoryResponse>(req.trajectory_id, tenant_id)
        .await?
        .ok_or_else(|| ApiError::trajectory_not_found(req.trajectory_id))?;

    // Resolve scope_id (auto-select most recent active scope if not provided)
    let scope_id = match req.scope_id {
        Some(id) => {
            // Validate provided scope exists and belongs to tenant
            let _scope = db
                .get::<ScopeResponse>(id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::scope_not_found(id))?;
            id
        }
        None => {
            // Auto-select most recent active scope for this trajectory
            let filter = ScopeListFilter {
                trajectory_id: Some(req.trajectory_id),
                is_active: Some(true),
                limit: Some(1),
                ..Default::default()
            };
            let scopes = db.list::<ScopeResponse>(&filter, tenant_id).await?;
            scopes
                .first()
                .map(|s| s.scope_id)
                .ok_or_else(|| ApiError::invalid_input("No active scope found for trajectory"))?
        }
    };

    // Build the context package
    let mut pkg = ContextPackage::new(req.trajectory_id, scope_id);

    // Add user input if provided
    if let Some(user_input) = req.user_input {
        pkg = pkg.with_user_input(user_input);
    }

    // Add session markers
    let session_markers = SessionMarkers {
        active_trajectory_id: Some(req.trajectory_id),
        active_scope_id: Some(scope_id),
        recent_artifact_ids: vec![],
        agent_id: req.agent_id,
    };
    pkg = pkg.with_session_markers(session_markers);

    // Add kernel config if provided
    if let Some(kernel_config) = req.kernel_config {
        pkg = pkg.with_kernel_config(kernel_config.into());
    }

    // Fetch and add relevant notes
    let mut notes_count = 0;
    if req.include_notes {
        let max_notes = req.max_notes.unwrap_or(10) as usize;
        let filter = NoteListFilter {
            source_trajectory_id: Some(req.trajectory_id),
            ..Default::default()
        };
        let notes = db.list::<NoteResponse>(&filter, tenant_id).await?;

        // Apply limit
        let limited_notes: Vec<_> = notes.into_iter().take(max_notes).collect();
        notes_count = limited_notes.len() as i32;

        // Convert API notes to core notes
        let core_notes: Vec<caliber_core::Note> =
            limited_notes.into_iter().map(note_response_to_core).collect();
        pkg = pkg.with_notes(core_notes);
    }

    // Fetch and add recent artifacts
    let mut artifacts_count = 0;
    if req.include_artifacts {
        let max_artifacts = req.max_artifacts.unwrap_or(5) as usize;
        let filter = ArtifactListFilter {
            scope_id: Some(scope_id),
            ..Default::default()
        };
        let artifacts = db.list::<ArtifactResponse>(&filter, tenant_id).await?;

        // Apply limit
        let limited_artifacts: Vec<_> = artifacts.into_iter().take(max_artifacts).collect();
        artifacts_count = limited_artifacts.len() as i32;

        // Convert API artifacts to core artifacts
        let core_artifacts: Vec<caliber_core::Artifact> =
            limited_artifacts.into_iter().map(artifact_response_to_core).collect();
        pkg = pkg.with_artifacts(core_artifacts);
    }

    // Fetch and count conversation turns
    // Note: ContextPackage doesn't support turns yet, but counts are returned in the response
    let mut turns_count = 0;
    if req.include_turns {
        let max_turns = req.max_turns.unwrap_or(20) as usize;
        let filter = TurnListFilter {
            scope_id: Some(scope_id),
            ..Default::default()
        };
        let turns = db.list::<TurnResponse>(&filter, tenant_id).await?;

        // Apply limit and count
        let limited_turns: Vec<_> = turns.into_iter().take(max_turns).collect();
        turns_count = limited_turns.len() as i32;

        // TODO: Add turns to package when caliber_context supports it
        // For now, turns are counted but not included in context assembly
    }

    // Fetch parent trajectory hierarchy if requested (returned in response only)
    let mut hierarchy: Vec<TrajectoryResponse> = Vec::new();
    if req.include_hierarchy {
        let mut current_id = trajectory.parent_trajectory_id;
        while let Some(parent_id) = current_id {
            if let Some(parent) = db.get::<TrajectoryResponse>(parent_id, tenant_id).await? {
                current_id = parent.parent_trajectory_id;
                hierarchy.push(parent);
            } else {
                break;
            }
        }
        // TODO: Add hierarchy to package when caliber_context supports it
    }

    // Fetch and add scope summaries (if we had a summarization system)
    // For now, we'll use empty summaries - this can be enhanced later
    let summaries_count = 0;
    if req.include_history {
        // TODO: Fetch scope summaries from database when summarization is implemented
        // For now, use empty vec
        let _max_summaries = req.max_summaries.unwrap_or(5);
    }

    // Build assembler config
    let token_budget = req.token_budget.unwrap_or(8000);
    let config = CaliberConfig {
        token_budget,
        section_priorities: SectionPriorities {
            user: 100,
            system: 90,
            persona: 85,
            artifacts: 80,
            notes: 70,
            history: 60,
            custom: vec![],
        },
        checkpoint_retention: 10,
        stale_threshold: Duration::from_secs(3600),
        contradiction_threshold: 0.8,
        context_window_persistence: ContextPersistence::Ephemeral,
        validation_mode: ValidationMode::OnMutation,
        embedding_provider: None,
        summarization_provider: None,
        llm_retry_config: RetryConfig {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        },
        lock_timeout: Duration::from_secs(30),
        message_retention: Duration::from_secs(86400),
        delegation_timeout: Duration::from_secs(300),
    };

    // Assemble the context
    let assembler = ContextAssembler::new(config)
        .map_err(|e| ApiError::internal_error(format!("Failed to create assembler: {}", e)))?;

    let window = assembler
        .assemble(pkg)
        .map_err(|e| ApiError::internal_error(format!("Failed to assemble context: {}", e)))?;

    // Build the response
    let response = AssembleContextResponse {
        context: window.as_text(),
        token_count: window.used_tokens,
        max_tokens: window.max_tokens,
        truncated: window.truncated,
        included_sections: window.included_sections.clone(),
        notes_count,
        artifacts_count,
        turns_count,
        summaries_count,
        hierarchy,
        window_details: Some(window),
    };

    Ok(Json(response))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert an API NoteResponse to a core Note.
fn note_response_to_core(note: crate::types::NoteResponse) -> caliber_core::Note {
    caliber_core::Note {
        note_id: note.note_id,
        note_type: note.note_type,
        title: note.title,
        content: note.content,
        content_hash: note.content_hash,
        embedding: None, // Embeddings are not returned in API response
        source_trajectory_ids: note.source_trajectory_ids,
        source_artifact_ids: note.source_artifact_ids,
        ttl: note.ttl,
        created_at: note.created_at,
        updated_at: note.updated_at,
        accessed_at: note.accessed_at,
        access_count: note.access_count,
        superseded_by: note.superseded_by,
        metadata: note.metadata,
        abstraction_level: caliber_core::AbstractionLevel::default(),
        source_note_ids: vec![],
    }
}

/// Convert an API ArtifactResponse to a core Artifact.
fn artifact_response_to_core(artifact: crate::types::ArtifactResponse) -> caliber_core::Artifact {
    caliber_core::Artifact {
        artifact_id: artifact.artifact_id,
        trajectory_id: artifact.trajectory_id,
        scope_id: artifact.scope_id,
        artifact_type: artifact.artifact_type,
        name: artifact.name,
        content: artifact.content,
        content_hash: artifact.content_hash,
        embedding: None, // Embeddings are not returned in API response
        provenance: caliber_core::Provenance {
            source_turn: artifact.provenance.source_turn,
            extraction_method: artifact.provenance.extraction_method,
            confidence: artifact.provenance.confidence,
        },
        ttl: artifact.ttl,
        created_at: artifact.created_at,
        updated_at: artifact.updated_at,
        superseded_by: artifact.superseded_by,
        metadata: artifact.metadata,
    }
}

// ============================================================================
// ROUTER
// ============================================================================

use axum::{routing::post, Router};

/// Create the context router.
pub fn context_router() -> Router<crate::state::AppState> {
    Router::new().route("/assemble", post(assemble_context))
}
