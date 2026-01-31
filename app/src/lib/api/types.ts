/**
 * API Response Types
 * Types for CALIBER API responses
 */

// ═══════════════════════════════════════════════════════════════════════════
// CORE TYPES
// ═══════════════════════════════════════════════════════════════════════════

export interface User {
  id: string;
  email: string;
  firstName?: string;
  lastName?: string;
  tenantId?: string;
  createdAt: string;
}

export interface Trajectory {
  id: string;
  name: string;
  description?: string;
  scopes: Scope[];
  eventCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface Scope {
  id: string;
  name: string;
  trajectoryId: string;
  parentId?: string;
  eventCount: number;
  memoryLimit?: number;
  tags: string[];
  createdAt: string;
  updatedAt: string;
}

export interface Event {
  id: string;
  scopeId: string;
  type: EventType;
  role: EventRole;
  content: string;
  metadata?: Record<string, unknown>;
  toolCalls?: ToolCall[];
  timestamp: string;
}

export type EventType =
  | 'message'
  | 'tool_call'
  | 'tool_result'
  | 'system'
  | 'observation'
  | 'decision'
  | 'error';

export type EventRole = 'user' | 'assistant' | 'system' | 'tool';

// ═══════════════════════════════════════════════════════════════════════════
// TOOL CALLS
// ═══════════════════════════════════════════════════════════════════════════

export interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
  status: ToolCallStatus;
  result?: ToolResult;
  duration?: number;
  timestamp: string;
}

export type ToolCallStatus = 'pending' | 'approved' | 'running' | 'success' | 'error' | 'rejected';

export interface ToolResult {
  content: string | Record<string, unknown>;
  isError: boolean;
  errorType?: string;
}

// ═══════════════════════════════════════════════════════════════════════════
// MESSAGES
// ═══════════════════════════════════════════════════════════════════════════

export interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: string;
  toolCalls?: ToolCall[];
  metadata?: Record<string, unknown>;
}

export interface AssistantResponse {
  id: string;
  content: string;
  timestamp: string;
  toolCalls?: ToolCall[];
  finishReason: 'stop' | 'tool_calls' | 'length' | 'error';
}

// ═══════════════════════════════════════════════════════════════════════════
// DASHBOARD
// ═══════════════════════════════════════════════════════════════════════════

export interface DashboardStats {
  trajectoryCount: number;
  scopeCount: number;
  eventCount: number;
  storageUsedBytes: number;
  recentActivity: ActivityItem[];
}

export interface ActivityItem {
  id: string;
  type: 'trajectory' | 'scope' | 'event';
  name: string;
  action: 'created' | 'updated' | 'deleted';
  timestamp: string;
}

// ═══════════════════════════════════════════════════════════════════════════
// API RESPONSES
// ═══════════════════════════════════════════════════════════════════════════

export interface ApiResponse<T> {
  data: T;
  meta?: {
    page?: number;
    perPage?: number;
    total?: number;
  };
}

export interface ApiError {
  error: string;
  code: string;
  details?: Record<string, unknown>;
}

export interface PaginatedResponse<T> {
  data: T[];
  meta: {
    page: number;
    perPage: number;
    total: number;
    totalPages: number;
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// REQUEST TYPES
// ═══════════════════════════════════════════════════════════════════════════

export interface CreateTrajectoryRequest {
  name: string;
  description?: string;
}

export interface UpdateTrajectoryRequest {
  name?: string;
  description?: string;
}

export interface CreateScopeRequest {
  name: string;
  trajectoryId: string;
  parentId?: string;
  memoryLimit?: number;
  tags?: string[];
}

export interface SendMessageRequest {
  content: string;
  trajectoryId?: string;
  scopeId?: string;
}

// ═══════════════════════════════════════════════════════════════════════════
// STREAMING TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Stream event types for WebSocket communication.
 */
export type StreamEventType =
  | 'chunk'
  | 'tool_call'
  | 'tool_result'
  | 'complete'
  | 'error'
  | 'heartbeat';

/**
 * Generic stream event wrapper.
 */
export interface StreamEvent {
  type: StreamEventType;
  data: unknown;
  timestamp?: string;
}

/**
 * Text chunk stream event.
 */
export interface ChunkEvent {
  type: 'chunk';
  data: string;
}

/**
 * Tool call stream event.
 */
export interface ToolCallStreamEvent {
  type: 'tool_call';
  data: {
    id: string;
    name: string;
    arguments: Record<string, unknown>;
  };
}

/**
 * Tool result stream event.
 */
export interface ToolResultStreamEvent {
  type: 'tool_result';
  data: {
    toolCallId: string;
    content: string | Record<string, unknown>;
    isError: boolean;
  };
}

/**
 * Stream complete event.
 */
export interface CompleteEvent {
  type: 'complete';
  data: AssistantResponse;
}

/**
 * Stream error event.
 */
export interface StreamErrorEvent {
  type: 'error';
  data: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TREE NODE TYPES (for FileTree component)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Tree node type for memory browser.
 */
export type TreeNodeType = 'trajectory' | 'scope' | 'event' | 'file' | 'folder';

/**
 * Tree node for FileTree/memory browser.
 */
export interface TreeNode {
  id: string;
  label: string;
  type: TreeNodeType;
  expanded?: boolean;
  selected?: boolean;
  icon?: string;
  children?: TreeNode[];
  metadata?: {
    path?: string;
    mimeType?: string;
    eventCount?: number;
    timestamp?: string;
  };
}
