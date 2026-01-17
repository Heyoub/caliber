/**
 * CALIBER SDK
 *
 * TypeScript SDK for CALIBER - Cognitive Agent Long-term Intelligence,
 * Behavioral Episodic Recall.
 *
 * @packageDocumentation
 */

// Main client
export { CalibrClient } from './client';
export type { CalibrClientConfig } from './client';

// HTTP layer
export { HttpClient } from './http';
export type { HttpClientConfig, RequestOptions } from './http';

// Errors
export {
  CaliberError,
  NotFoundError,
  AuthenticationError,
  AuthorizationError,
  ValidationError,
  ConflictError,
  RateLimitError,
  parseApiError,
} from './errors';
export type { ApiErrorDetails } from './errors';

// WebSocket client
export { CaliberWebSocket } from './websocket';
export type {
  WsEventType,
  WsEvent,
  ConnectionState,
  CaliberWebSocketConfig,
  EventHandler,
} from './websocket';

// Context assembly helper
export { ContextHelper } from './context';
export type {
  AssembleContextOptions,
  ContextPackage,
  FormatContextOptions,
} from './context';

// Managers
export {
  TrajectoryManager,
  ScopeManager,
  ArtifactManager,
  NoteManager,
  TurnManager,
  AgentManager,
  LockManager,
  MessageManager,
  DelegationManager,
  HandoffManager,
  SearchManager,
  DslManager,
  BatchManager,
} from './managers';

// Batch operation types
export type {
  BatchOperation,
  BatchItemResult,
  TrajectoryBatchItem,
  ArtifactBatchItem,
  NoteBatchItem,
  BatchRequestParams,
  BatchResponse,
} from './managers';

// Types - Trajectory
export type {
  Trajectory,
  TrajectoryStatus,
  TrajectoryOutcome,
  OutcomeStatus,
  CreateTrajectoryParams,
  UpdateTrajectoryParams,
  ListTrajectoriesParams,
  ListTrajectoriesResponse,
} from './managers';

// Types - Scope
export type {
  Scope,
  Checkpoint,
  CreateScopeParams,
  UpdateScopeParams,
  CreateCheckpointParams,
} from './managers';

// Types - Artifact
export type {
  Artifact,
  ArtifactType,
  ExtractionMethod,
  TTL,
  Provenance,
  Embedding,
  CreateArtifactParams,
  UpdateArtifactParams,
  ListArtifactsParams,
  ListArtifactsResponse,
  SearchArtifactsParams,
  SearchResult,
  SearchResponse,
} from './managers';

// Types - Note
export type {
  Note,
  NoteType,
  CreateNoteParams,
  UpdateNoteParams,
  ListNotesParams,
  ListNotesResponse,
  SearchNotesParams,
} from './managers';

// Types - Turn
export type {
  Turn,
  TurnRole,
  CreateTurnParams,
} from './managers';

// Types - Agent
export type {
  Agent,
  MemoryAccess,
  MemoryPermission,
  RegisterAgentParams,
  UpdateAgentParams,
  ListAgentsParams,
  ListAgentsResponse,
  HeartbeatResponse,
} from './managers';

// Types - Lock
export type {
  Lock,
  LockMode,
  AcquireLockParams,
  ExtendLockParams,
  ListLocksParams,
  ListLocksResponse,
} from './managers';

// Types - Message
export type {
  Message,
  MessagePriority,
  SendMessageParams,
  ListMessagesParams,
  ListMessagesResponse,
} from './managers';

// Types - Delegation
export type {
  Delegation,
  DelegationStatus,
  DelegationResult,
  DelegationResultStatus,
  CreateDelegationParams,
  CompleteDelegationParams,
} from './managers';

// Types - Handoff
export type {
  Handoff,
  HandoffStatus,
  CreateHandoffParams,
} from './managers';

// Types - Search
export type {
  EntityType,
  FilterExpr,
  SearchParams,
} from './managers';

// Types - DSL
export type {
  ParseError,
  ValidateDslResponse,
  ParseDslResponse,
} from './managers';

// Common types
export type { PaginationParams, PaginatedResponse } from './managers';
