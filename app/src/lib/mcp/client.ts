/**
 * MCP JSON-RPC Client
 * Client for communicating with MCP servers
 */
import type {
  JsonRpcId,
  JsonRpcRequest,
  JsonRpcResponse,
  JsonRpcNotification,
  JsonRpcError,
  McpCapabilities,
  InitializeParams,
  InitializeResult,
  Tool,
  ListToolsResult,
  CallToolParams,
  CallToolResult,
  Resource,
  ListResourcesResult,
  ReadResourceParams,
  ReadResourceResult,
  Prompt,
  ListPromptsResult,
  GetPromptParams,
  GetPromptResult,
  McpMethodName,
} from './types';
import { McpMethod, JsonRpcErrorCode } from './types';

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

export interface McpClientOptions {
  /** WebSocket URL or HTTP endpoint */
  endpoint: string;
  /** Client name for initialization */
  clientName: string;
  /** Client version */
  clientVersion: string;
  /** Request timeout in ms */
  timeout?: number;
  /** Auto-reconnect on disconnect */
  autoReconnect?: boolean;
  /** Reconnect delay in ms */
  reconnectDelay?: number;
}

export interface McpClientState {
  isConnected: boolean;
  isInitialized: boolean;
  serverInfo: { name: string; version: string } | null;
  serverCapabilities: McpCapabilities | null;
  error: string | null;
}

type NotificationHandler = (method: string, params?: Record<string, unknown>) => void;
type ResponseHandler = (response: JsonRpcResponse) => void;

// ═══════════════════════════════════════════════════════════════════════════
// MCP CLIENT
// ═══════════════════════════════════════════════════════════════════════════

export class McpClient {
  private options: Required<McpClientOptions>;
  private ws: WebSocket | null = null;
  private requestId = 0;
  private pendingRequests = new Map<JsonRpcId, ResponseHandler>();
  private notificationHandlers = new Set<NotificationHandler>();

  state: McpClientState = {
    isConnected: false,
    isInitialized: false,
    serverInfo: null,
    serverCapabilities: null,
    error: null,
  };

  constructor(options: McpClientOptions) {
    this.options = {
      timeout: 30000,
      autoReconnect: true,
      reconnectDelay: 1000,
      ...options,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════
  // CONNECTION
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Connect to the MCP server
   */
  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.options.endpoint);

        this.ws.onopen = () => {
          this.state.isConnected = true;
          this.state.error = null;
          resolve();
        };

        this.ws.onclose = (event) => {
          this.state.isConnected = false;
          this.state.isInitialized = false;

          if (this.options.autoReconnect && !event.wasClean) {
            setTimeout(() => this.connect(), this.options.reconnectDelay);
          }
        };

        this.ws.onerror = (event) => {
          this.state.error = 'WebSocket error';
          reject(new Error('WebSocket connection failed'));
        };

        this.ws.onmessage = (event) => {
          this.handleMessage(event.data);
        };
      } catch (error) {
        reject(error);
      }
    });
  }

  /**
   * Disconnect from the MCP server
   */
  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.state.isConnected = false;
    this.state.isInitialized = false;
  }

  /**
   * Initialize the MCP connection
   */
  async initialize(): Promise<InitializeResult> {
    const params: InitializeParams = {
      protocolVersion: '2025-11-25',
      capabilities: {
        tools: {},
        resources: { subscribe: true },
        prompts: {},
      },
      clientInfo: {
        name: this.options.clientName,
        version: this.options.clientVersion,
      },
    };

    const result = await this.request<InitializeResult>(McpMethod.Initialize, params);

    this.state.serverInfo = result.serverInfo;
    this.state.serverCapabilities = result.capabilities;
    this.state.isInitialized = true;

    // Send initialized notification
    this.notify(McpMethod.Initialized);

    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════
  // TOOLS
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * List available tools
   */
  async listTools(cursor?: string): Promise<ListToolsResult> {
    return this.request<ListToolsResult>(McpMethod.ListTools, cursor ? { cursor } : undefined);
  }

  /**
   * Call a tool
   */
  async callTool(name: string, args?: Record<string, unknown>): Promise<CallToolResult> {
    const params: CallToolParams = { name, arguments: args };
    return this.request<CallToolResult>(McpMethod.CallTool, params);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // RESOURCES
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * List available resources
   */
  async listResources(cursor?: string): Promise<ListResourcesResult> {
    return this.request<ListResourcesResult>(
      McpMethod.ListResources,
      cursor ? { cursor } : undefined
    );
  }

  /**
   * Read a resource
   */
  async readResource(uri: string): Promise<ReadResourceResult> {
    const params: ReadResourceParams = { uri };
    return this.request<ReadResourceResult>(McpMethod.ReadResource, params);
  }

  /**
   * Subscribe to resource updates
   */
  async subscribeResource(uri: string): Promise<void> {
    await this.request(McpMethod.Subscribe, { uri });
  }

  /**
   * Unsubscribe from resource updates
   */
  async unsubscribeResource(uri: string): Promise<void> {
    await this.request(McpMethod.Unsubscribe, { uri });
  }

  // ═══════════════════════════════════════════════════════════════════════
  // PROMPTS
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * List available prompts
   */
  async listPrompts(cursor?: string): Promise<ListPromptsResult> {
    return this.request<ListPromptsResult>(McpMethod.ListPrompts, cursor ? { cursor } : undefined);
  }

  /**
   * Get a prompt
   */
  async getPrompt(name: string, args?: Record<string, string>): Promise<GetPromptResult> {
    const params: GetPromptParams = { name, arguments: args };
    return this.request<GetPromptResult>(McpMethod.GetPrompt, params);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // NOTIFICATIONS
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Register a notification handler
   */
  onNotification(handler: NotificationHandler): () => void {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // INTERNAL
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Send a JSON-RPC request and wait for response
   */
  private async request<T>(method: McpMethodName | string, params?: object): Promise<T> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('Not connected to MCP server');
    }

    const id = ++this.requestId;

    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      id,
      method,
      params: params as Record<string, unknown> | undefined,
    };

    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request timeout: ${method}`));
      }, this.options.timeout);

      this.pendingRequests.set(id, (response) => {
        clearTimeout(timeoutId);
        this.pendingRequests.delete(id);

        if (response.error) {
          reject(new McpError(response.error));
        } else {
          resolve(response.result as T);
        }
      });

      this.ws!.send(JSON.stringify(request));
    });
  }

  /**
   * Send a JSON-RPC notification (no response expected)
   */
  private notify(method: McpMethodName | string, params?: Record<string, unknown>): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return;
    }

    const notification: JsonRpcNotification = {
      jsonrpc: '2.0',
      method,
      params,
    };

    this.ws.send(JSON.stringify(notification));
  }

  /**
   * Handle incoming WebSocket message
   */
  private handleMessage(data: string): void {
    try {
      const message = JSON.parse(data);

      // Check if it's a response (has id)
      if ('id' in message) {
        const handler = this.pendingRequests.get(message.id);
        if (handler) {
          handler(message as JsonRpcResponse);
        }
        return;
      }

      // It's a notification
      if ('method' in message) {
        const notification = message as JsonRpcNotification;
        for (const handler of this.notificationHandlers) {
          try {
            handler(notification.method, notification.params);
          } catch (error) {
            console.error('Notification handler error:', error);
          }
        }
      }
    } catch (error) {
      console.error('Failed to parse MCP message:', error);
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// MCP ERROR
// ═══════════════════════════════════════════════════════════════════════════

export class McpError extends Error {
  code: number;
  data?: unknown;

  constructor(error: JsonRpcError) {
    super(error.message);
    this.name = 'McpError';
    this.code = error.code;
    this.data = error.data;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create and initialize an MCP client
 */
export async function createMcpClient(
  endpoint: string,
  options?: Partial<Omit<McpClientOptions, 'endpoint'>>
): Promise<McpClient> {
  const client = new McpClient({
    endpoint,
    clientName: options?.clientName || 'caliber-app',
    clientVersion: options?.clientVersion || '0.1.0',
    ...options,
  });

  await client.connect();
  await client.initialize();

  return client;
}
