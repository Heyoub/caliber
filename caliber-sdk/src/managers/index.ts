/**
 * Manager Exports
 *
 * This module exports all resource managers and re-exports types from ../types
 */

// Base Manager (SDK-specific, not generated)
export { BaseManager } from './base';
export type { PaginationParams, PaginatedResponse } from './base';

// Managers (hand-written ergonomic wrappers)
export { TrajectoryManager } from './trajectory';
export { ScopeManager } from './scope';
export { ArtifactManager } from './artifact';
export { NoteManager } from './note';
export { TurnManager } from './turn';
export { AgentManager } from './agent';
export { LockManager } from './lock';
export { MessageManager } from './message';
export { DelegationManager } from './delegation';
export { HandoffManager } from './handoff';
export { SearchManager } from './search';
export { DslManager } from './dsl';
export { BatchManager } from './batch';
export type {
  BatchOperation,
  BatchItemResult,
  TrajectoryBatchItem,
  ArtifactBatchItem,
  NoteBatchItem,
  BatchRequestParams,
  BatchResponse,
} from './batch';

// Re-export all types from ../types (which imports from generated code)
// This ensures all types come from the single source of truth
export type {
  // Trajectory types
  Trajectory,
  CreateTrajectoryParams,
  UpdateTrajectoryParams,
  ListTrajectoriesParams,
  ListTrajectoriesResponse,
  TrajectoryOutcome,
  TrajectoryStatus,
  OutcomeStatus,

  // Scope types
  Scope,
  CreateScopeParams,
  UpdateScopeParams,
  CreateCheckpointParams,
  Checkpoint,

  // Artifact types
  Artifact,
  CreateArtifactParams,
  UpdateArtifactParams,
  ListArtifactsParams,
  ListArtifactsResponse,
  SearchArtifactsParams,
  SearchResponse,
  ArtifactType,
  ExtractionMethod,
  TTL,
  Provenance,
  Embedding,

  // Note types
  Note,
  CreateNoteParams,
  UpdateNoteParams,
  ListNotesParams,
  ListNotesResponse,
  SearchNotesParams,
  NoteType,

  // Turn types
  Turn,
  CreateTurnParams,
  TurnRole,

  // Agent types
  Agent,
  RegisterAgentParams,
  UpdateAgentParams,
  ListAgentsParams,
  ListAgentsResponse,
  HeartbeatResponse,
  MemoryAccess,
  MemoryPermission,

  // Lock types
  Lock,
  AcquireLockParams,
  ExtendLockParams,
  ListLocksParams,
  ListLocksResponse,
  LockMode,

  // Message types
  Message,
  SendMessageParams,
  ListMessagesParams,
  ListMessagesResponse,
  MessagePriority,

  // Delegation types
  Delegation,
  CreateDelegationParams,
  CompleteDelegationParams,
  DelegationResult,
  DelegationStatus,
  DelegationResultStatus,

  // Handoff types
  Handoff,
  CreateHandoffParams,
  HandoffStatus,

  // Search types
  SearchParams,
  SearchResult,
  EntityType,
  FilterExpr,

  // DSL types
  ValidateDslResponse,
  ParseDslResponse,
  ParseError,
} from '../types';
