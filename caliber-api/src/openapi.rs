//! OpenAPI Specification for CALIBER API
//!
//! This module defines the OpenAPI document for the CALIBER REST API.
//! It uses utoipa to generate the OpenAPI specification from Rust types
//! and route annotations.

use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::error::{ApiError, ErrorCode};
use crate::types::*;

// Import route modules for path references
use crate::routes::{
    agent, artifact, config, delegation, dsl, handoff, lock, message, note, scope, tenant,
    trajectory, turn,
};

// Import domain types from caliber-core
use caliber_core::{
    Artifact, ArtifactType, CaliberConfig, Checkpoint, ContextPersistence, EmbeddingVector,
    EntityRef, EntityType, ExtractionMethod, MemoryCategory, Note, NoteType, OutcomeStatus,
    ProviderConfig, Provenance, RetryConfig, Scope, SectionPriorities, Trajectory,
    TrajectoryOutcome, TrajectoryStatus, Turn, TurnRole, ValidationMode, TTL,
};

// Import multi-agent types from caliber-agents
use caliber_agents::{
    Agent, AgentHandoff, AgentMessage, AgentStatus, Conflict, ConflictResolutionRecord,
    ConflictStatus, ConflictType, DelegatedTask, DelegationResult, DelegationResultStatus,
    DelegationStatus, DistributedLock, HandoffReason, HandoffStatus, LockMode, MemoryAccess,
    MemoryPermission, MemoryRegion, MemoryRegionConfig, MessagePriority, MessageType,
    PermissionScope, ResolutionStrategy,
};

// Import ConflictResolution from pcp
use caliber_pcp::ConflictResolution;

/// OpenAPI document for CALIBER API.
///
/// This struct generates the complete OpenAPI specification for the API,
/// including all schemas, paths, and security definitions.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "CALIBER API",
        version = "0.1.0",
        description = "Cognitive Agent Long-term Intelligence, Behavioral Episodic Recall - Production Memory Framework for AI Agents",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(name = "CALIBER", url = "https://caliber.run")
    ),
    servers(
        (url = "https://api.caliber.run", description = "Production"),
        (url = "http://localhost:3000", description = "Local Development")
    ),
    tags(
        (name = "Trajectories", description = "Task container management - top-level goals and sub-tasks"),
        (name = "Scopes", description = "Context window partitioning with checkpointing"),
        (name = "Artifacts", description = "Typed outputs preserved across scopes"),
        (name = "Notes", description = "Long-term cross-trajectory knowledge"),
        (name = "Turns", description = "Ephemeral conversation buffer entries"),
        (name = "Agents", description = "Multi-agent registration and management"),
        (name = "Locks", description = "Distributed locking for resource coordination"),
        (name = "Messages", description = "Inter-agent communication"),
        (name = "Delegations", description = "Task delegation between agents"),
        (name = "Handoffs", description = "Context handoff between agents"),
        (name = "DSL", description = "Domain-specific language validation and parsing"),
        (name = "Config", description = "System configuration"),
        (name = "Tenants", description = "Multi-tenant management")
    ),
    paths(
        // === Trajectory Routes ===
        trajectory::create_trajectory,
        trajectory::list_trajectories,
        trajectory::get_trajectory,
        trajectory::update_trajectory,
        trajectory::delete_trajectory,
        trajectory::list_trajectory_scopes,
        trajectory::list_trajectory_children,

        // === Scope Routes ===
        scope::create_scope,
        scope::get_scope,
        scope::update_scope,
        scope::create_checkpoint,
        scope::close_scope,
        scope::list_scope_turns,
        scope::list_scope_artifacts,

        // === Artifact Routes ===
        artifact::create_artifact,
        artifact::list_artifacts,
        artifact::get_artifact,
        artifact::update_artifact,
        artifact::delete_artifact,
        artifact::search_artifacts,

        // === Note Routes ===
        note::create_note,
        note::list_notes,
        note::get_note,
        note::update_note,
        note::delete_note,
        note::search_notes,

        // === Turn Routes ===
        turn::create_turn,
        turn::get_turn,

        // === Agent Routes ===
        agent::register_agent,
        agent::list_agents,
        agent::get_agent,
        agent::update_agent,
        agent::unregister_agent,
        agent::agent_heartbeat,

        // === Lock Routes ===
        lock::acquire_lock,
        lock::release_lock,
        lock::extend_lock,
        lock::list_locks,
        lock::get_lock,

        // === Message Routes ===
        message::send_message,
        message::list_messages,
        message::get_message,
        message::acknowledge_message,

        // === Delegation Routes ===
        delegation::create_delegation,
        delegation::get_delegation,
        delegation::accept_delegation,
        delegation::reject_delegation,
        delegation::complete_delegation,

        // === Handoff Routes ===
        handoff::create_handoff,
        handoff::get_handoff,
        handoff::accept_handoff,
        handoff::complete_handoff,

        // === DSL Routes ===
        dsl::validate_dsl,
        dsl::parse_dsl,

        // === Config Routes ===
        config::get_config,
        config::update_config,
        config::validate_config,

        // === Tenant Routes ===
        tenant::list_tenants,
        tenant::get_tenant,
    ),
    components(
        schemas(
            // === Error Types ===
            ApiError, ErrorCode,

            // === Trajectory Types ===
            CreateTrajectoryRequest, UpdateTrajectoryRequest, ListTrajectoriesRequest,
            ListTrajectoriesResponse, TrajectoryResponse, TrajectoryOutcomeResponse,

            // === Scope Types ===
            CreateScopeRequest, UpdateScopeRequest, CreateCheckpointRequest,
            ScopeResponse, CheckpointResponse,

            // === Artifact Types ===
            CreateArtifactRequest, UpdateArtifactRequest, ListArtifactsRequest,
            ListArtifactsResponse, ArtifactResponse, ProvenanceResponse, EmbeddingResponse,

            // === Note Types ===
            CreateNoteRequest, UpdateNoteRequest, ListNotesRequest,
            ListNotesResponse, NoteResponse,

            // === Turn Types ===
            CreateTurnRequest, TurnResponse,

            // === Agent Types ===
            RegisterAgentRequest, UpdateAgentRequest, AgentResponse,
            MemoryAccessRequest, MemoryAccessResponse,
            MemoryPermissionRequest, MemoryPermissionResponse,

            // === Lock Types ===
            AcquireLockRequest, ExtendLockRequest, LockResponse,

            // === Message Types ===
            SendMessageRequest, MessageResponse,

            // === Delegation Types ===
            CreateDelegationRequest, DelegationResponse, DelegationResultResponse,

            // === Handoff Types ===
            CreateHandoffRequest, HandoffResponse,

            // === Search Types ===
            SearchRequest, SearchResponse, SearchResult, FilterExpr,

            // === DSL Types ===
            ValidateDslRequest, ValidateDslResponse, ParseErrorResponse,

            // === Config Types ===
            UpdateConfigRequest, ConfigResponse,

            // === Tenant Types ===
            TenantInfo, TenantStatus, ListTenantsResponse,

            // === List Types (from types.rs, not route modules) ===
            ListAgentsRequest, ListAgentsResponse,
            ListLocksResponse,
            ListMessagesRequest, ListMessagesResponse,
            delegation::AcceptDelegationRequest, delegation::RejectDelegationRequest,
            delegation::CompleteDelegationRequest,
            handoff::AcceptHandoffRequest,

            // === Core Domain Types (from caliber-core) ===
            TTL, EntityType, MemoryCategory, TrajectoryStatus, OutcomeStatus,
            TurnRole, ArtifactType, ExtractionMethod, NoteType,
            EmbeddingVector, EntityRef, Trajectory, TrajectoryOutcome,
            Scope, Checkpoint, Artifact, Provenance, Note, Turn,
            SectionPriorities, ContextPersistence, ValidationMode,
            ProviderConfig, RetryConfig, CaliberConfig,

            // === Agent Domain Types (from caliber-agents) ===
            AgentStatus, PermissionScope, MemoryRegion, LockMode,
            MessageType, MessagePriority, DelegationStatus, DelegationResultStatus,
            HandoffStatus, HandoffReason, ConflictType, ConflictStatus, ResolutionStrategy,
            MemoryPermission, MemoryAccess, Agent, MemoryRegionConfig,
            DistributedLock, AgentMessage, DelegationResult, DelegatedTask,
            AgentHandoff, ConflictResolutionRecord, Conflict,

            // === PCP Types ===
            ConflictResolution
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security scheme modifier for OpenAPI document.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // API Key authentication (header)
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
            );

            // Bearer token authentication (JWT)
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT Bearer token"))
                        .build(),
                ),
            );
        }
    }
}

impl ApiDoc {
    /// Generate OpenAPI spec as JSON string.
    pub fn to_json() -> Result<String, serde_json::Error> {
        let openapi = Self::openapi();
        serde_json::to_string_pretty(&openapi)
    }

    /// Generate OpenAPI spec as YAML string.
    #[cfg(feature = "openapi")]
    pub fn to_yaml() -> Result<String, String> {
        let openapi = Self::openapi();
        serde_yaml::to_string(&openapi).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::OpenApi;

    #[test]
    fn test_openapi_generation() -> Result<(), String> {
        let openapi = ApiDoc::openapi();

        // Verify basic structure
        assert_eq!(openapi.info.title, "CALIBER API");
        assert_eq!(openapi.info.version, "0.1.0");

        // Verify servers
        let servers = openapi
            .servers
            .as_ref()
            .ok_or_else(|| "OpenAPI servers missing".to_string())?;
        assert_eq!(servers.len(), 2);

        // Verify tags exist
        let tags = openapi
            .tags
            .as_ref()
            .ok_or_else(|| "OpenAPI tags missing".to_string())?;
        assert!(tags.len() >= 10);

        // Verify security schemes
        let components = openapi
            .components
            .as_ref()
            .ok_or_else(|| "OpenAPI components missing".to_string())?;
        assert!(components.security_schemes.contains_key("api_key"));
        assert!(components.security_schemes.contains_key("bearer_auth"));
        Ok(())
    }

    #[test]
    fn test_openapi_json_serialization() -> Result<(), String> {
        let json = ApiDoc::to_json().map_err(|e| format!("Failed to serialize OpenAPI: {}", e))?;

        // Verify it's valid JSON by parsing it back
        serde_json::from_str::<serde_json::Value>(&json)
            .map_err(|e| format!("Generated JSON invalid: {}", e))?;

        // Verify key fields are present (allow for spacing variations)
        assert!(json.contains("CALIBER API"));
        assert!(json.contains("\"api_key\""));
        assert!(json.contains("\"bearer_auth\""));
        Ok(())
    }

    #[test]
    fn test_openapi_paths_exist() {
        let openapi = ApiDoc::openapi();

        // Verify paths are populated
        assert!(!openapi.paths.paths.is_empty());

        // Verify key paths exist
        assert!(openapi.paths.paths.contains_key("/api/v1/trajectories"));
        assert!(openapi.paths.paths.contains_key("/api/v1/scopes"));
        assert!(openapi.paths.paths.contains_key("/api/v1/artifacts"));
        assert!(openapi.paths.paths.contains_key("/api/v1/notes"));
        assert!(openapi.paths.paths.contains_key("/api/v1/turns"));
        assert!(openapi.paths.paths.contains_key("/api/v1/agents"));
        assert!(openapi.paths.paths.contains_key("/api/v1/locks/acquire"));
        assert!(openapi.paths.paths.contains_key("/api/v1/messages"));
        assert!(openapi.paths.paths.contains_key("/api/v1/delegations"));
        assert!(openapi.paths.paths.contains_key("/api/v1/handoffs"));
        assert!(openapi.paths.paths.contains_key("/api/v1/dsl/validate"));
        assert!(openapi.paths.paths.contains_key("/api/v1/config"));
        assert!(openapi.paths.paths.contains_key("/api/v1/tenants"));
    }
}
