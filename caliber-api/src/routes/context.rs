//! Context assembly routes.
//!
//! This module provides the `/context/assemble` endpoint that wires the
//! previously-unused `caliber-context` crate into the API layer.
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

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use axum::{extract::State, Extension, Json};
use caliber_context::{
    ContextAssembler, ContextPackage, ContextWindow, KernelConfig, ScopeSummary, SessionMarkers,
};
use caliber_core::{CaliberConfig, EntityId};
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
    pub trajectory_id: EntityId,

    /// Scope to assemble context for
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub scope_id: EntityId,

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

    /// Maximum number of notes to include
    pub max_notes: Option<i32>,

    /// Maximum number of artifacts to include
    pub max_artifacts: Option<i32>,

    /// Maximum number of scope summaries to include
    pub max_summaries: Option<i32>,

    /// Optional kernel/persona configuration
    pub kernel_config: Option<KernelConfigRequest>,

    /// Agent ID for multi-agent scenarios
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "uuid"))]
    pub agent_id: Option<EntityId>,
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

    /// Detailed breakdown of the context window (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_details: Option<ContextWindowDetails>,
}

/// Detailed breakdown of the assembled context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ContextWindowDetails {
    /// Window ID for tracing
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub window_id: EntityId,

    /// Number of notes included
    pub notes_included: i32,

    /// Number of artifacts included
    pub artifacts_included: i32,

    /// Number of scope summaries included
    pub summaries_included: i32,

    /// Assembly trace for debugging
    pub assembly_trace: Vec<AssemblyTraceEntry>,
}

/// Entry in the assembly trace for debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AssemblyTraceEntry {
    /// Action taken (Include, Exclude, Truncate, Compress)
    pub action: String,
    /// Type of content (Notes, Artifacts, History, etc.)
    pub content_type: String,
    /// Reason for the action
    pub reason: String,
    /// Tokens affected by this action
    pub tokens_affected: i32,
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
    let _trajectory = db
        .trajectory_get(req.trajectory_id, tenant_id)
        .await?
        .ok_or_else(|| ApiError::trajectory_not_found(req.trajectory_id))?;

    // Validate scope exists and belongs to tenant
    let _scope = db
        .scope_get(req.scope_id, tenant_id)
        .await?
        .ok_or_else(|| ApiError::scope_not_found(req.scope_id))?;

    // Build the context package
    let mut pkg = ContextPackage::new(req.trajectory_id, req.scope_id);

    // Add user input if provided
    if let Some(user_input) = req.user_input {
        pkg = pkg.with_user_input(user_input);
    }

    // Add session markers
    let session_markers = SessionMarkers {
        active_trajectory_id: Some(req.trajectory_id),
        active_scope_id: Some(req.scope_id),
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
        let notes = db
            .note_list_by_trajectory(req.trajectory_id, tenant_id)
            .await?;

        // Apply limit
        let limited_notes: Vec<_> = notes.into_iter().take(max_notes).collect();
        notes_count = limited_notes.len() as i32;

        // Convert API notes to core notes
        let core_notes: Vec<caliber_core::Note> = limited_notes
            .into_iter()
            .map(|n| note_response_to_core(n))
            .collect();
        pkg = pkg.with_notes(core_notes);
    }

    // Fetch and add recent artifacts
    let mut artifacts_count = 0;
    if req.include_artifacts {
        let max_artifacts = req.max_artifacts.unwrap_or(5) as usize;
        let artifacts = db
            .artifact_list_by_scope(req.scope_id, tenant_id)
            .await?;

        // Apply limit
        let limited_artifacts: Vec<_> = artifacts.into_iter().take(max_artifacts).collect();
        artifacts_count = limited_artifacts.len() as i32;

        // Convert API artifacts to core artifacts
        let core_artifacts: Vec<caliber_core::Artifact> = limited_artifacts
            .into_iter()
            .map(|a| artifact_response_to_core(a))
            .collect();
        pkg = pkg.with_artifacts(core_artifacts);
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
        ..Default::default()
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
        window_details: Some(ContextWindowDetails {
            window_id: window.window_id,
            notes_included: notes_count,
            artifacts_included: artifacts_count,
            summaries_included: summaries_count,
            assembly_trace: window
                .assembly_trace
                .iter()
                .map(|d| AssemblyTraceEntry {
                    action: format!("{:?}", d.action),
                    content_type: d.target_type.clone(),
                    reason: d.reason.clone(),
                    tokens_affected: d.tokens_affected,
                })
                .collect(),
        }),
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
        content_hash: note.content_hash.unwrap_or([0u8; 32]),
        embedding: None, // Embeddings are not returned in API response
        source_trajectory_ids: note.source_trajectory_ids,
        source_artifact_ids: note.source_artifact_ids.unwrap_or_default(),
        ttl: note.ttl,
        created_at: note.created_at,
        updated_at: note.updated_at,
        accessed_at: note.accessed_at.unwrap_or(note.updated_at),
        access_count: note.access_count.unwrap_or(0),
        superseded_by: note.superseded_by,
        metadata: note.metadata,
        abstraction_level: note.abstraction_level.unwrap_or_default(),
        source_note_ids: note.source_note_ids.unwrap_or_default(),
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
        content_hash: artifact.content_hash.unwrap_or([0u8; 32]),
        embedding: None, // Embeddings are not returned in API response
        provenance: caliber_core::Provenance {
            source_turn: artifact.provenance.as_ref().map(|p| p.source_turn).unwrap_or(0),
            extraction_method: artifact
                .provenance
                .as_ref()
                .map(|p| p.extraction_method)
                .unwrap_or(caliber_core::ExtractionMethod::Explicit),
            confidence: artifact.provenance.as_ref().and_then(|p| p.confidence),
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
