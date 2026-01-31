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

use crate::auth::AuthContext;
use crate::components::{ArtifactListFilter, NoteListFilter, ScopeListFilter, TurnListFilter};
use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::providers::{
    EmbedRequest, EmbedResponse, PingResponse, ProviderAdapter, ProviderRegistry, SummarizeRequest,
    SummarizeResponse,
};
use crate::types::{
    ArtifactResponse, NoteResponse, ScopeResponse, SearchRequest, TrajectoryResponse, TurnResponse,
};
use async_trait::async_trait;
use axum::{extract::State, Extension, Json};
use caliber_core::{
    AgentId, ArtifactId, CaliberConfig, CaliberError, ContextAssembler, ContextPackage,
    ContextWindow, EntityIdType, EntityType, HealthStatus, KernelConfig, LlmError, NoteId,
    ProviderCapability, RoutingStrategy, ScopeId, SessionMarkers, TrajectoryId,
};
use caliber_dsl::compiler::{
    CompiledConfig as DslCompiledConfig, CompiledInjectionMode, InjectionConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

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

    // Query text for semantic retrieval (prefer explicit relevance_query).
    let query_text = req
        .relevance_query
        .clone()
        .or_else(|| req.user_input.clone());

    // Active compiled config (used for providers + injections).
    let compiled_config = db
        .dsl_compiled_get_active(tenant_id, "default")
        .await
        .ok()
        .flatten();

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
        let note_injection = compiled_config
            .as_ref()
            .and_then(|c| select_injection_mode(c, EntityType::Note));

        let notes: Vec<NoteResponse> =
            if let (Some(query), Some(mode)) = (query_text.as_deref(), note_injection.as_ref()) {
                semantic_notes(&db, tenant_id, query, mode, max_notes).await?
            } else {
                let filter = NoteListFilter {
                    source_trajectory_id: Some(req.trajectory_id),
                    ..Default::default()
                };
                db.list::<NoteResponse>(&filter, tenant_id)
                    .await?
                    .into_iter()
                    .take(max_notes)
                    .collect()
            };

        notes_count = notes.len() as i32;
        let core_notes: Vec<caliber_core::Note> =
            notes.into_iter().map(note_response_to_core).collect();
        pkg = pkg.with_notes(core_notes);
    }

    // Fetch and add recent artifacts
    let mut artifacts_count = 0;
    if req.include_artifacts {
        let max_artifacts = req.max_artifacts.unwrap_or(5) as usize;
        let artifact_injection = compiled_config
            .as_ref()
            .and_then(|c| select_injection_mode(c, EntityType::Artifact));

        let artifacts: Vec<ArtifactResponse> = if let (Some(query), Some(mode)) =
            (query_text.as_deref(), artifact_injection.as_ref())
        {
            semantic_artifacts(&db, tenant_id, scope_id, query, mode, max_artifacts).await?
        } else {
            let filter = ArtifactListFilter {
                scope_id: Some(scope_id),
                ..Default::default()
            };
            db.list::<ArtifactResponse>(&filter, tenant_id)
                .await?
                .into_iter()
                .take(max_artifacts)
                .collect()
        };

        artifacts_count = artifacts.len() as i32;
        let core_artifacts: Vec<caliber_core::Artifact> = artifacts
            .into_iter()
            .map(artifact_response_to_core)
            .collect();
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

    // Build assembler config from core defaults, then layer in runtime config.
    let token_budget = req.token_budget.unwrap_or(8000);
    let mut config = CaliberConfig::default_context(token_budget);

    if let Some(compiled) = compiled_config.as_ref() {
        if let Some(provider) =
            select_provider_for_capability(compiled, ProviderCapability::Embedding).await
        {
            config.embedding_provider = Some(provider);
        }
        if let Some(provider) =
            select_provider_for_capability(compiled, ProviderCapability::Summarization).await
        {
            config.summarization_provider = Some(provider);
        }
    }

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

fn select_injection_mode(
    compiled: &DslCompiledConfig,
    entity_type: EntityType,
) -> Option<CompiledInjectionMode> {
    if !compiled.pack_injections.is_empty() {
        let mut best: Option<&caliber_dsl::compiler::CompiledPackInjectionConfig> = None;
        for injection in &compiled.pack_injections {
            if injection
                .entity_type
                .as_deref()
                .and_then(entity_type_from_str)
                != Some(entity_type)
            {
                continue;
            }
            match best {
                Some(current) if current.priority >= injection.priority => {}
                _ => best = Some(injection),
            }
        }
        if best.is_some() {
            return best.map(|i| i.mode.clone());
        }
    }

    let mut best: Option<&InjectionConfig> = None;
    for injection in &compiled.injections {
        if injection_entity_type(injection) != Some(entity_type) {
            continue;
        }
        match best {
            Some(current) if current.priority >= injection.priority => {}
            _ => best = Some(injection),
        }
    }
    best.map(|i| i.mode.clone())
}

fn entity_type_from_str(value: &str) -> Option<EntityType> {
    match value.to_lowercase().as_str() {
        "note" | "notes" => Some(EntityType::Note),
        "artifact" | "artifacts" => Some(EntityType::Artifact),
        _ => None,
    }
}

fn injection_entity_type(injection: &InjectionConfig) -> Option<EntityType> {
    let source = injection.source.to_lowercase();
    if source.contains("note") {
        Some(EntityType::Note)
    } else if source.contains("artifact") {
        Some(EntityType::Artifact)
    } else {
        None
    }
}

fn semantic_params(mode: &CompiledInjectionMode, max_items: usize) -> (i32, Option<f32>) {
    let base_limit = i32::try_from(max_items).unwrap_or(50).max(1);
    match mode {
        CompiledInjectionMode::TopK { k } => (std::cmp::min(*k, base_limit).max(1), None),
        CompiledInjectionMode::Relevant { threshold } => {
            let overfetch = (base_limit * 3).max(10);
            (overfetch, Some(*threshold))
        }
        CompiledInjectionMode::Full | CompiledInjectionMode::Summary => (base_limit, None),
    }
}

async fn semantic_notes(
    db: &DbClient,
    tenant_id: caliber_core::TenantId,
    query: &str,
    mode: &CompiledInjectionMode,
    max_notes: usize,
) -> ApiResult<Vec<NoteResponse>> {
    let (limit, threshold) = semantic_params(mode, max_notes);
    let search = SearchRequest {
        query: query.to_string(),
        entity_types: vec![EntityType::Note],
        filters: vec![],
        limit: Some(limit),
    };
    let response = db.search(&search, tenant_id).await?;

    let mut notes = Vec::new();
    for result in response.results {
        if let Some(threshold) = threshold {
            if result.score < threshold {
                continue;
            }
        }
        let id = NoteId::new(result.id);
        if let Some(note) = db.get::<NoteResponse>(id, tenant_id).await? {
            notes.push(note);
        }
        if notes.len() >= max_notes {
            break;
        }
    }
    Ok(notes)
}

async fn semantic_artifacts(
    db: &DbClient,
    tenant_id: caliber_core::TenantId,
    scope_id: ScopeId,
    query: &str,
    mode: &CompiledInjectionMode,
    max_artifacts: usize,
) -> ApiResult<Vec<ArtifactResponse>> {
    let (limit, threshold) = semantic_params(mode, max_artifacts);
    let filters = vec![caliber_core::FilterExpr::eq(
        "scope_id",
        json!(scope_id.as_uuid()),
    )];
    let search = SearchRequest {
        query: query.to_string(),
        entity_types: vec![EntityType::Artifact],
        filters,
        limit: Some(limit),
    };
    let response = db.search(&search, tenant_id).await?;

    let mut artifacts = Vec::new();
    for result in response.results {
        if let Some(threshold) = threshold {
            if result.score < threshold {
                continue;
            }
        }
        let id = ArtifactId::new(result.id);
        if let Some(artifact) = db.get::<ArtifactResponse>(id, tenant_id).await? {
            artifacts.push(artifact);
        }
        if artifacts.len() >= max_artifacts {
            break;
        }
    }
    Ok(artifacts)
}

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

async fn select_provider_for_capability(
    compiled: &DslCompiledConfig,
    capability: ProviderCapability,
) -> Option<caliber_core::ProviderConfig> {
    if compiled.providers.is_empty() {
        return None;
    }

    let providers_by_name: HashMap<&str, &caliber_dsl::compiler::CompiledProviderConfig> = compiled
        .providers
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    // Respect explicit routing hints first.
    let preferred = compiled
        .pack_routing
        .as_ref()
        .and_then(|routing| match capability {
            ProviderCapability::Embedding => routing.embedding_provider.as_deref(),
            ProviderCapability::Summarization => routing.summarization_provider.as_deref(),
            _ => None,
        });

    if let Some(name) = preferred {
        if let Some(provider) = providers_by_name.get(name) {
            return provider_from_compiled(provider);
        }
    }

    // Fall back to registry-based selection using the routing strategy hint.
    let strategy = compiled
        .pack_routing
        .as_ref()
        .and_then(|r| r.strategy.as_deref())
        .and_then(routing_strategy_from_hint)
        .unwrap_or(RoutingStrategy::First);

    let registry = ProviderRegistry::new(strategy);
    for provider in &compiled.providers {
        let adapter: Arc<dyn ProviderAdapter> = Arc::new(PackProviderAdapter::new(&provider.name));
        registry.register(adapter).await;
    }

    let selected = registry.select_provider(capability).await.ok()?;
    providers_by_name
        .get(selected.provider_id())
        .and_then(|p| provider_from_compiled(p))
}

fn routing_strategy_from_hint(hint: &str) -> Option<RoutingStrategy> {
    match hint.to_lowercase().as_str() {
        "first" => Some(RoutingStrategy::First),
        "round_robin" | "roundrobin" => Some(RoutingStrategy::RoundRobin),
        "random" => Some(RoutingStrategy::Random),
        "least_latency" | "leastlatency" => Some(RoutingStrategy::LeastLatency),
        _ => None,
    }
}

struct PackProviderAdapter {
    id: String,
    capabilities: Vec<ProviderCapability>,
}

impl PackProviderAdapter {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            capabilities: vec![
                ProviderCapability::Embedding,
                ProviderCapability::Summarization,
            ],
        }
    }
}

#[async_trait]
impl ProviderAdapter for PackProviderAdapter {
    fn provider_id(&self) -> &str {
        &self.id
    }

    fn capabilities(&self) -> &[ProviderCapability] {
        &self.capabilities
    }

    async fn ping(&self) -> caliber_core::CaliberResult<PingResponse> {
        Ok(PingResponse {
            provider_id: self.id.clone(),
            capabilities: self.capabilities.clone(),
            latency_ms: 1,
            health: HealthStatus::Healthy,
            metadata: HashMap::new(),
        })
    }

    async fn embed(&self, _request: EmbedRequest) -> caliber_core::CaliberResult<EmbedResponse> {
        Err(CaliberError::Llm(LlmError::ProviderNotConfigured))
    }

    async fn summarize(
        &self,
        _request: SummarizeRequest,
    ) -> caliber_core::CaliberResult<SummarizeResponse> {
        Err(CaliberError::Llm(LlmError::ProviderNotConfigured))
    }
}

fn provider_from_compiled(
    p: &caliber_dsl::compiler::CompiledProviderConfig,
) -> Option<caliber_core::ProviderConfig> {
    let provider_type = match p.provider_type {
        caliber_dsl::compiler::CompiledProviderType::OpenAI => "openai",
        caliber_dsl::compiler::CompiledProviderType::Anthropic => "anthropic",
        caliber_dsl::compiler::CompiledProviderType::Custom => "custom",
    }
    .to_string();

    let endpoint = p.options.get("endpoint").cloned();
    let dimensions = p
        .options
        .get("dimensions")
        .and_then(|v| v.parse::<i32>().ok());

    Some(caliber_core::ProviderConfig {
        provider_type,
        endpoint,
        model: p.model.clone(),
        dimensions,
    })
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthContext, AuthMethod};
    use crate::db::{DbClient, DbConfig};
    use crate::types::{
        CreateScopeRequest, CreateTrajectoryRequest, ScopeResponse, TrajectoryResponse,
    };
    use axum::{extract::State, Extension, Json};
    use caliber_dsl::compiler::{CompiledInjectionMode, InjectionConfig};
    use uuid::Uuid;

    #[test]
    fn test_default_true_is_true() {
        assert!(default_true());
    }

    #[test]
    fn test_context_format_serde() {
        let json =
            serde_json::to_string(&ContextFormat::Markdown).expect("serialization should succeed");
        assert_eq!(json, "\"markdown\"");

        let parsed: ContextFormat =
            serde_json::from_str("\"xml\"").expect("deserialization should succeed");
        assert_eq!(parsed, ContextFormat::Xml);
    }

    #[test]
    fn test_kernel_config_request_conversion() {
        let req = KernelConfigRequest {
            persona: Some("analyst".to_string()),
            tone: Some("concise".to_string()),
            reasoning_style: Some("stepwise".to_string()),
            domain_focus: Some("security".to_string()),
        };
        let cfg: KernelConfig = req.clone().into();
        assert_eq!(cfg.persona, req.persona);
        assert_eq!(cfg.tone, req.tone);
        assert_eq!(cfg.reasoning_style, req.reasoning_style);
        assert_eq!(cfg.domain_focus, req.domain_focus);
    }

    #[test]
    fn test_entity_type_from_str() {
        assert_eq!(entity_type_from_str("note"), Some(EntityType::Note));
        assert_eq!(entity_type_from_str("notes"), Some(EntityType::Note));
        assert_eq!(entity_type_from_str("artifact"), Some(EntityType::Artifact));
        assert_eq!(
            entity_type_from_str("ARTIFACTS"),
            Some(EntityType::Artifact)
        );
        assert_eq!(entity_type_from_str("unknown"), None);
    }

    #[test]
    fn test_injection_entity_type() {
        let note_injection = InjectionConfig {
            source: "notes.semantic".to_string(),
            target: "context".to_string(),
            mode: CompiledInjectionMode::Summary,
            priority: 1,
            max_tokens: None,
            filter: None,
        };
        assert_eq!(
            injection_entity_type(&note_injection),
            Some(EntityType::Note)
        );

        let artifact_injection = InjectionConfig {
            source: "artifacts.recent".to_string(),
            target: "context".to_string(),
            mode: CompiledInjectionMode::Full,
            priority: 1,
            max_tokens: None,
            filter: None,
        };
        assert_eq!(
            injection_entity_type(&artifact_injection),
            Some(EntityType::Artifact)
        );

        let unknown = InjectionConfig {
            source: "memory".to_string(),
            target: "context".to_string(),
            mode: CompiledInjectionMode::Full,
            priority: 1,
            max_tokens: None,
            filter: None,
        };
        assert_eq!(injection_entity_type(&unknown), None);
    }

    #[test]
    fn test_semantic_params() {
        let (limit, threshold) = semantic_params(&CompiledInjectionMode::TopK { k: 3 }, 10);
        assert_eq!(limit, 3);
        assert!(threshold.is_none());

        let (limit, threshold) =
            semantic_params(&CompiledInjectionMode::Relevant { threshold: 0.42 }, 5);
        assert_eq!(limit, 15);
        assert_eq!(threshold, Some(0.42));

        let (limit, threshold) = semantic_params(&CompiledInjectionMode::Full, 0);
        assert_eq!(limit, 1);
        assert!(threshold.is_none());
    }

    #[test]
    fn test_routing_strategy_from_hint() {
        assert_eq!(
            routing_strategy_from_hint("round_robin"),
            Some(RoutingStrategy::RoundRobin)
        );
        assert_eq!(
            routing_strategy_from_hint("leastlatency"),
            Some(RoutingStrategy::LeastLatency)
        );
        assert_eq!(routing_strategy_from_hint("unknown"), None);
    }

    #[tokio::test]
    async fn test_pack_provider_adapter_ping() {
        let adapter = PackProviderAdapter::new("test");
        assert_eq!(adapter.provider_id(), "test");
        assert!(adapter
            .capabilities()
            .contains(&ProviderCapability::Embedding));

        let response = adapter.ping().await.expect("ping should succeed");
        assert_eq!(response.provider_id, "test");
        assert_eq!(response.health, HealthStatus::Healthy);
    }

    struct DbTestContext {
        db: DbClient,
        auth: AuthContext,
    }

    async fn db_test_context() -> Option<DbTestContext> {
        if std::env::var("DB_TESTS").ok().as_deref() != Some("1") {
            return None;
        }

        let db = DbClient::from_config(&DbConfig::from_env()).ok()?;
        let conn = db.get_conn().await.ok()?;
        let has_fn = conn
            .query_opt(
                "SELECT 1 FROM pg_proc WHERE proname = 'caliber_tenant_create' LIMIT 1",
                &[],
            )
            .await
            .ok()
            .flatten()
            .is_some();
        if !has_fn {
            return None;
        }

        let tenant_id = db.tenant_create("test-context", None, None).await.ok()?;
        let auth = AuthContext::new("test-user".to_string(), tenant_id, vec![], AuthMethod::Jwt);

        Some(DbTestContext { db, auth })
    }

    #[tokio::test]
    async fn test_assemble_context_db_backed_minimal() {
        let Some(ctx) = db_test_context().await else {
            return;
        };

        let traj_req = CreateTrajectoryRequest {
            name: format!("context-traj-{}", Uuid::now_v7()),
            description: None,
            parent_trajectory_id: None,
            agent_id: None,
            metadata: None,
        };
        let trajectory: TrajectoryResponse = ctx
            .db
            .create::<TrajectoryResponse>(&traj_req, ctx.auth.tenant_id)
            .await
            .expect("create trajectory");

        let scope_req = CreateScopeRequest {
            trajectory_id: trajectory.trajectory_id,
            parent_scope_id: None,
            name: "context-scope".to_string(),
            purpose: None,
            token_budget: 1000,
            metadata: None,
        };
        let scope: ScopeResponse = ctx
            .db
            .create::<ScopeResponse>(&scope_req, ctx.auth.tenant_id)
            .await
            .expect("create scope");

        let req = AssembleContextRequest {
            trajectory_id: trajectory.trajectory_id,
            scope_id: Some(scope.scope_id),
            user_input: None,
            token_budget: Some(1000),
            include_notes: false,
            include_artifacts: false,
            include_history: false,
            include_turns: false,
            include_hierarchy: false,
            max_notes: None,
            max_artifacts: None,
            max_summaries: None,
            max_turns: None,
            kernel_config: None,
            agent_id: None,
            relevance_query: None,
            min_relevance: None,
            format: ContextFormat::Markdown,
        };

        let Json(resp) = assemble_context(
            State(ctx.db.clone()),
            Extension(ctx.auth.clone()),
            Json(req),
        )
        .await
        .expect("assemble context");

        assert_eq!(resp.notes_count, 0);
        assert_eq!(resp.artifacts_count, 0);
        assert_eq!(resp.turns_count, 0);
        assert_eq!(resp.summaries_count, 0);

        ctx.db
            .delete::<ScopeResponse>(scope.scope_id, ctx.auth.tenant_id)
            .await
            .ok();
        ctx.db
            .delete::<TrajectoryResponse>(trajectory.trajectory_id, ctx.auth.tenant_id)
            .await
            .ok();
    }
}
