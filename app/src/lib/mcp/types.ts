/**
 * MCP Protocol Types
 * Full Model Context Protocol types based on spec 2025-11-25
 * https://modelcontextprotocol.io/specification/2025-11-25
 */

// ═══════════════════════════════════════════════════════════════════════════
// JSON-RPC BASE TYPES
// ═══════════════════════════════════════════════════════════════════════════

export type JsonRpcId = string | number;

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: JsonRpcId;
  method: string;
  params?: Record<string, unknown>;
}

export interface JsonRpcResponse {
  jsonrpc: '2.0';
  id: JsonRpcId;
  result?: unknown;
  error?: JsonRpcError;
}

export interface JsonRpcNotification {
  jsonrpc: '2.0';
  method: string;
  params?: Record<string, unknown>;
}

export interface JsonRpcError {
  code: number;
  message: string;
  data?: unknown;
}

// Standard JSON-RPC error codes
export const JsonRpcErrorCode = {
  ParseError: -32700,
  InvalidRequest: -32600,
  MethodNotFound: -32601,
  InvalidParams: -32602,
  InternalError: -32603,
} as const;

// ═══════════════════════════════════════════════════════════════════════════
// MCP PROTOCOL TYPES
// ═══════════════════════════════════════════════════════════════════════════

export interface McpCapabilities {
  tools?: McpToolsCapability;
  resources?: McpResourcesCapability;
  prompts?: McpPromptsCapability;
  logging?: Record<string, never>;
  experimental?: Record<string, unknown>;
}

export interface McpToolsCapability {
  listChanged?: boolean;
}

export interface McpResourcesCapability {
  subscribe?: boolean;
  listChanged?: boolean;
}

export interface McpPromptsCapability {
  listChanged?: boolean;
}

// ═══════════════════════════════════════════════════════════════════════════
// INITIALIZATION
// ═══════════════════════════════════════════════════════════════════════════

export interface InitializeParams {
  protocolVersion: string;
  capabilities: McpCapabilities;
  clientInfo: Implementation;
}

export interface InitializeResult {
  protocolVersion: string;
  capabilities: McpCapabilities;
  serverInfo: Implementation;
  instructions?: string;
}

export interface Implementation {
  name: string;
  version: string;
}

// ═══════════════════════════════════════════════════════════════════════════
// TOOLS
// ═══════════════════════════════════════════════════════════════════════════

export interface Tool {
  name: string;
  description?: string;
  inputSchema: JsonSchema;
}

export interface JsonSchema {
  type: string;
  properties?: Record<string, JsonSchema>;
  required?: string[];
  items?: JsonSchema;
  description?: string;
  enum?: unknown[];
  default?: unknown;
  additionalProperties?: boolean | JsonSchema;
  [key: string]: unknown;
}

export interface ListToolsResult {
  tools: Tool[];
  nextCursor?: string;
}

export interface CallToolParams {
  name: string;
  arguments?: Record<string, unknown>;
}

export interface CallToolResult {
  content: ToolContent[];
  isError?: boolean;
}

export type ToolContent = TextContent | ImageContent | EmbeddedResource;

export interface TextContent {
  type: 'text';
  text: string;
}

export interface ImageContent {
  type: 'image';
  data: string;
  mimeType: string;
}

export interface EmbeddedResource {
  type: 'resource';
  resource: ResourceContents;
}

// ═══════════════════════════════════════════════════════════════════════════
// RESOURCES
// ═══════════════════════════════════════════════════════════════════════════

export interface Resource {
  uri: string;
  name: string;
  description?: string;
  mimeType?: string;
  annotations?: ResourceAnnotations;
}

export interface ResourceAnnotations {
  audience?: ('user' | 'assistant')[];
  priority?: number;
}

export interface ResourceTemplate {
  uriTemplate: string;
  name: string;
  description?: string;
  mimeType?: string;
}

export interface ListResourcesResult {
  resources: Resource[];
  nextCursor?: string;
}

export interface ListResourceTemplatesResult {
  resourceTemplates: ResourceTemplate[];
  nextCursor?: string;
}

export interface ReadResourceParams {
  uri: string;
}

export interface ReadResourceResult {
  contents: ResourceContents[];
}

export interface ResourceContents {
  uri: string;
  mimeType?: string;
  text?: string;
  blob?: string;
}

export interface SubscribeParams {
  uri: string;
}

export interface UnsubscribeParams {
  uri: string;
}

// ═══════════════════════════════════════════════════════════════════════════
// PROMPTS
// ═══════════════════════════════════════════════════════════════════════════

export interface Prompt {
  name: string;
  description?: string;
  arguments?: PromptArgument[];
}

export interface PromptArgument {
  name: string;
  description?: string;
  required?: boolean;
}

export interface ListPromptsResult {
  prompts: Prompt[];
  nextCursor?: string;
}

export interface GetPromptParams {
  name: string;
  arguments?: Record<string, string>;
}

export interface GetPromptResult {
  description?: string;
  messages: PromptMessage[];
}

export interface PromptMessage {
  role: 'user' | 'assistant';
  content: PromptContent;
}

export type PromptContent = TextContent | ImageContent | EmbeddedResource;

// ═══════════════════════════════════════════════════════════════════════════
// SAMPLING (Client -> Server)
// ═══════════════════════════════════════════════════════════════════════════

export interface CreateMessageParams {
  messages: SamplingMessage[];
  modelPreferences?: ModelPreferences;
  systemPrompt?: string;
  includeContext?: 'none' | 'thisServer' | 'allServers';
  temperature?: number;
  maxTokens: number;
  stopSequences?: string[];
  metadata?: Record<string, unknown>;
}

export interface SamplingMessage {
  role: 'user' | 'assistant';
  content: TextContent | ImageContent;
}

export interface ModelPreferences {
  hints?: ModelHint[];
  costPriority?: number;
  speedPriority?: number;
  intelligencePriority?: number;
}

export interface ModelHint {
  name?: string;
}

export interface CreateMessageResult {
  role: 'assistant';
  content: TextContent | ImageContent;
  model: string;
  stopReason?: 'endTurn' | 'stopSequence' | 'maxTokens';
}

// ═══════════════════════════════════════════════════════════════════════════
// ROOTS
// ═══════════════════════════════════════════════════════════════════════════

export interface Root {
  uri: string;
  name?: string;
}

export interface ListRootsResult {
  roots: Root[];
}

// ═══════════════════════════════════════════════════════════════════════════
// LOGGING
// ═══════════════════════════════════════════════════════════════════════════

export type LogLevel = 'debug' | 'info' | 'notice' | 'warning' | 'error' | 'critical' | 'alert' | 'emergency';

export interface SetLevelParams {
  level: LogLevel;
}

export interface LoggingMessageParams {
  level: LogLevel;
  logger?: string;
  data: unknown;
}

// ═══════════════════════════════════════════════════════════════════════════
// NOTIFICATIONS
// ═══════════════════════════════════════════════════════════════════════════

export interface ProgressParams {
  progressToken: string | number;
  progress: number;
  total?: number;
}

export interface CancelledParams {
  requestId: JsonRpcId;
  reason?: string;
}

export interface ResourceUpdatedParams {
  uri: string;
}

export interface ResourceListChangedParams {
  // Empty
}

export interface ToolListChangedParams {
  // Empty
}

export interface PromptListChangedParams {
  // Empty
}

export interface RootsListChangedParams {
  // Empty
}

// ═══════════════════════════════════════════════════════════════════════════
// METHOD NAMES
// ═══════════════════════════════════════════════════════════════════════════

export const McpMethod = {
  // Lifecycle
  Initialize: 'initialize',
  Initialized: 'notifications/initialized',
  Ping: 'ping',

  // Tools
  ListTools: 'tools/list',
  CallTool: 'tools/call',
  ToolListChanged: 'notifications/tools/list_changed',

  // Resources
  ListResources: 'resources/list',
  ListResourceTemplates: 'resources/templates/list',
  ReadResource: 'resources/read',
  Subscribe: 'resources/subscribe',
  Unsubscribe: 'resources/unsubscribe',
  ResourceUpdated: 'notifications/resources/updated',
  ResourceListChanged: 'notifications/resources/list_changed',

  // Prompts
  ListPrompts: 'prompts/list',
  GetPrompt: 'prompts/get',
  PromptListChanged: 'notifications/prompts/list_changed',

  // Sampling
  CreateMessage: 'sampling/createMessage',

  // Roots
  ListRoots: 'roots/list',
  RootsListChanged: 'notifications/roots/list_changed',

  // Logging
  SetLevel: 'logging/setLevel',
  LogMessage: 'notifications/message',

  // Progress
  Progress: 'notifications/progress',
  Cancelled: 'notifications/cancelled',
} as const;

export type McpMethodName = (typeof McpMethod)[keyof typeof McpMethod];
