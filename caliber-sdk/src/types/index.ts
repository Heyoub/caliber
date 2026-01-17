/**
 * Type Exports
 *
 * This module imports types from the generated OpenAPI client and re-exports
 * them with ergonomic names. This is the SINGLE source of truth for types.
 *
 * When the Rust API changes:
 * 1. Run `scripts/generate-sdk.sh` to regenerate OpenAPI types
 * 2. Types here automatically update (they're just re-exports)
 * 3. No manual type maintenance needed
 *
 * IMPORTANT: Type names here must match the generated code exactly.
 * The generated names come from Rust struct names via utoipa.
 */

// =============================================================================
// Generated Type Imports
//
// These are imported from the OpenAPI-generated code in ../generated/models
// The names match the Rust struct names from caliber-api
// =============================================================================

export type {
  // Trajectory types (from types.rs)
  TrajectoryResponse as Trajectory,
  CreateTrajectoryRequest as CreateTrajectoryParams,
  UpdateTrajectoryRequest as UpdateTrajectoryParams,
  ListTrajectoriesRequest as ListTrajectoriesParams,
  ListTrajectoriesResponse,
  TrajectoryOutcomeResponse as TrajectoryOutcome,
  TrajectoryStatus,
  OutcomeStatus,

  // Scope types (from types.rs)
  ScopeResponse as Scope,
  CreateScopeRequest as CreateScopeParams,
  UpdateScopeRequest as UpdateScopeParams,
  CreateCheckpointRequest as CreateCheckpointParams,
  CheckpointResponse as Checkpoint,

  // Artifact types (from types.rs)
  ArtifactResponse as Artifact,
  CreateArtifactRequest as CreateArtifactParams,
  UpdateArtifactRequest as UpdateArtifactParams,
  ListArtifactsRequest as ListArtifactsParams,
  ListArtifactsResponse,
  SearchResponse,
  ArtifactType,
  ExtractionMethod,
  TTL,
  ProvenanceResponse as Provenance,
  EmbeddingResponse as Embedding,

  // Note types (from types.rs)
  NoteResponse as Note,
  CreateNoteRequest as CreateNoteParams,
  UpdateNoteRequest as UpdateNoteParams,
  ListNotesRequest as ListNotesParams,
  ListNotesResponse,
  NoteType,

  // Turn types (from types.rs)
  TurnResponse as Turn,
  CreateTurnRequest as CreateTurnParams,
  TurnRole,

  // Agent types (from types.rs and routes/agent.rs)
  AgentResponse as Agent,
  RegisterAgentRequest as RegisterAgentParams,
  UpdateAgentRequest as UpdateAgentParams,
  ListAgentsRequest as ListAgentsParams,
  ListAgentsResponse,
  MemoryAccessResponse as MemoryAccess,
  MemoryPermissionResponse as MemoryPermission,

  // Lock types (from types.rs and routes/lock.rs)
  LockResponse as Lock,
  AcquireLockRequest as AcquireLockParams,
  ExtendLockRequest as ExtendLockParams,
  ListLocksResponse,
  LockMode,

  // Message types (from types.rs and routes/message.rs)
  MessageResponse as Message,
  SendMessageRequest as SendMessageParams,
  ListMessagesRequest as ListMessagesParams,
  ListMessagesResponse,
  MessagePriority,

  // Delegation types (from types.rs)
  DelegationResponse as Delegation,
  CreateDelegationRequest as CreateDelegationParams,
  DelegationResultRequest as CompleteDelegationParams,
  DelegationResultResponse as DelegationResult,
  DelegationStatus,
  DelegationResultStatus,

  // Handoff types (from types.rs)
  HandoffResponse as Handoff,
  CreateHandoffRequest as CreateHandoffParams,
  HandoffStatus,

  // Search types (from types.rs)
  SearchRequest as SearchParams,
  SearchResult,
  EntityType,
  FilterExpr,

  // DSL types (from types.rs)
  ValidateDslResponse,
  ParseDslResponse,
  ParseErrorResponse as ParseError,
} from '../generated/models';

// =============================================================================
// Type Aliases for Convenience
// =============================================================================

// ListLocksParams doesn't exist in REST API (no query params)
// Export an empty interface for consistency
export interface ListLocksParams {}

// SearchArtifactsParams and SearchNotesParams use the generic SearchRequest
export type { SearchRequest as SearchArtifactsParams } from '../generated/models';
export type { SearchRequest as SearchNotesParams } from '../generated/models';

// HeartbeatResponse - REST API returns AgentResponse, not a dedicated type
// Export an alias for backwards compatibility
export type { AgentResponse as HeartbeatResponse } from '../generated/models';
