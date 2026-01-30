/**
 * CALIBER WebSocket Client
 *
 * Provides real-time event streaming from the CALIBER API.
 * Handles automatic reconnection, authentication, and event filtering.
 */

import { CaliberError, AuthenticationError } from './errors';

/**
 * WebSocket event types (matches Rust WsEvent enum)
 */
export type WsEventType =
  // Trajectory events
  | 'TrajectoryCreated'
  | 'TrajectoryUpdated'
  | 'TrajectoryDeleted'
  // Scope events
  | 'ScopeCreated'
  | 'ScopeUpdated'
  | 'ScopeClosed'
  // Artifact events
  | 'ArtifactCreated'
  | 'ArtifactUpdated'
  | 'ArtifactDeleted'
  // Note events
  | 'NoteCreated'
  | 'NoteUpdated'
  | 'NoteDeleted'
  // Turn events
  | 'TurnCreated'
  // Agent events
  | 'AgentRegistered'
  | 'AgentStatusChanged'
  | 'AgentHeartbeat'
  | 'AgentUnregistered'
  // Lock events
  | 'LockAcquired'
  | 'LockReleased'
  | 'LockExpired'
  // Message events
  | 'MessageSent'
  | 'MessageDelivered'
  | 'MessageAcknowledged'
  // Delegation events
  | 'DelegationCreated'
  | 'DelegationAccepted'
  | 'DelegationRejected'
  | 'DelegationCompleted'
  // Handoff events
  | 'HandoffCreated'
  | 'HandoffAccepted'
  | 'HandoffCompleted'
  // Config events
  | 'ConfigUpdated'
  // Connection events
  | 'Connected'
  | 'Disconnected'
  | 'Error'
  // Battle Intel events
  | 'SummarizationTriggered'
  | 'EdgeCreated'
  | 'EdgesBatchCreated';

/**
 * Base interface for all WebSocket events
 */
export interface WsEvent {
  type: WsEventType;
  [key: string]: unknown;
}

/**
 * Connection state
 */
export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

/**
 * Configuration for the CALIBER WebSocket client
 */
export interface CaliberWebSocketConfig {
  /** Base URL for the CALIBER API (will be converted to ws:// or wss://) */
  baseUrl: string;
  /** API key for authentication */
  apiKey: string;
  /** Tenant ID for multi-tenant isolation */
  tenantId: string;
  /** Enable automatic reconnection (default: true) */
  autoReconnect?: boolean;
  /** Maximum reconnection attempts (default: 10) */
  maxReconnectAttempts?: number;
  /** Initial reconnection delay in ms (default: 1000) */
  reconnectDelay?: number;
  /** Maximum reconnection delay in ms (default: 30000) */
  maxReconnectDelay?: number;
  /** Heartbeat interval in ms (default: 30000) */
  heartbeatInterval?: number;
}

/**
 * Event handler callback type
 */
export type EventHandler<T extends WsEvent = WsEvent> = (event: T) => void;

/**
 * CALIBER WebSocket Client
 *
 * Provides real-time event streaming with automatic reconnection.
 *
 * @example
 * ```typescript
 * import { CaliberWebSocket } from '@caliber-run/sdk';
 *
 * const ws = new CaliberWebSocket({
 *   baseUrl: 'https://api.caliber.run',
 *   apiKey: process.env.CALIBER_API_KEY!,
 *   tenantId: 'your-tenant-id',
 * });
 *
 * ws.onEvent((event) => {
 *   console.log('Received event:', event.type, event);
 * });
 *
 * ws.on('TrajectoryCreated', (event) => {
 *   console.log('New trajectory:', event.trajectory);
 * });
 *
 * await ws.connect();
 *
 * // Later, when done:
 * ws.disconnect();
 * ```
 */
export class CaliberWebSocket {
  private readonly config: Required<CaliberWebSocketConfig>;
  private socket: WebSocket | null = null;
  private state: ConnectionState = 'disconnected';
  private reconnectAttempts = 0;
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  private heartbeatTimeout: ReturnType<typeof setInterval> | null = null;

  // Event handlers
  private eventHandlers: Map<WsEventType | '*', Set<EventHandler>> = new Map();
  private stateChangeHandlers: Set<(state: ConnectionState) => void> = new Set();

  constructor(config: CaliberWebSocketConfig) {
    this.config = {
      baseUrl: config.baseUrl,
      apiKey: config.apiKey,
      tenantId: config.tenantId,
      autoReconnect: config.autoReconnect ?? true,
      maxReconnectAttempts: config.maxReconnectAttempts ?? 10,
      reconnectDelay: config.reconnectDelay ?? 1000,
      maxReconnectDelay: config.maxReconnectDelay ?? 30000,
      heartbeatInterval: config.heartbeatInterval ?? 30000,
    };
  }

  /**
   * Get current connection state
   */
  getState(): ConnectionState {
    return this.state;
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.state === 'connected';
  }

  /**
   * Connect to the CALIBER WebSocket server
   */
  async connect(): Promise<void> {
    if (this.state === 'connected' || this.state === 'connecting') {
      return;
    }

    this.setState('connecting');
    return this.createConnection();
  }

  /**
   * Disconnect from the CALIBER WebSocket server
   */
  disconnect(): void {
    this.cleanup();
    this.setState('disconnected');
  }

  /**
   * Subscribe to all events
   */
  onEvent(handler: EventHandler): () => void {
    return this.on('*' as WsEventType, handler);
  }

  /**
   * Subscribe to a specific event type
   */
  on<T extends WsEvent = WsEvent>(eventType: WsEventType, handler: EventHandler<T>): () => void {
    let handlers = this.eventHandlers.get(eventType);
    if (!handlers) {
      handlers = new Set();
      this.eventHandlers.set(eventType, handlers);
    }
    handlers.add(handler as EventHandler);

    // Return unsubscribe function
    return () => {
      this.eventHandlers.get(eventType)?.delete(handler as EventHandler);
    };
  }

  /**
   * Subscribe to connection state changes
   */
  onStateChange(handler: (state: ConnectionState) => void): () => void {
    this.stateChangeHandlers.add(handler);
    return () => {
      this.stateChangeHandlers.delete(handler);
    };
  }

  /**
   * Unsubscribe from a specific event type
   */
  off(eventType: WsEventType, handler?: EventHandler): void {
    if (handler) {
      this.eventHandlers.get(eventType)?.delete(handler);
    } else {
      this.eventHandlers.delete(eventType);
    }
  }

  /**
   * Unsubscribe from all events
   */
  offAll(): void {
    this.eventHandlers.clear();
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private setState(state: ConnectionState): void {
    if (this.state !== state) {
      this.state = state;
      this.stateChangeHandlers.forEach((handler) => handler(state));
    }
  }

  private createConnection(): Promise<void> {
    return new Promise((resolve, reject) => {
      const wsUrl = this.buildWebSocketUrl();

      try {
        // In Node.js environments, we need to pass headers differently
        // The WebSocket API in browsers doesn't support custom headers
        // So we pass auth via query params (API must support this)
        this.socket = new WebSocket(wsUrl);
      } catch {
        this.setState('disconnected');
        reject(new CaliberError('Failed to create WebSocket connection', 'CONNECTION_ERROR'));
        return;
      }

      this.socket.onopen = () => {
        this.setState('connected');
        this.reconnectAttempts = 0;
        this.startHeartbeat();
        resolve();
      };

      this.socket.onclose = (event) => {
        this.cleanup();

        if (event.code === 1008 || event.code === 4001) {
          // Authentication failure
          this.setState('disconnected');
          reject(new AuthenticationError('WebSocket authentication failed'));
          return;
        }

        if (
          this.config.autoReconnect &&
          this.reconnectAttempts < this.config.maxReconnectAttempts
        ) {
          this.scheduleReconnect();
        } else {
          this.setState('disconnected');
        }
      };

      this.socket.onerror = (error) => {
        console.error('WebSocket error:', error);
        // onclose will be called after onerror
      };

      this.socket.onmessage = (event) => {
        this.handleMessage(event.data);
      };
    });
  }

  private buildWebSocketUrl(): string {
    // Convert http(s) to ws(s)
    let wsUrl = this.config.baseUrl.replace(/^https:\/\//, 'wss://').replace(/^http:\/\//, 'ws://');

    // Remove trailing slash
    wsUrl = wsUrl.replace(/\/$/, '');

    // Add WebSocket path and auth params
    // Note: Browsers don't allow custom headers on WebSocket connections,
    // so we pass authentication via query parameters
    const params = new URLSearchParams({
      token: this.config.apiKey,
      tenant_id: this.config.tenantId,
    });

    return `${wsUrl}/api/v1/ws?${params.toString()}`;
  }

  private handleMessage(data: string): void {
    try {
      const event = JSON.parse(data) as WsEvent;

      // Emit to specific type handlers
      const typeHandlers = this.eventHandlers.get(event.type);
      if (typeHandlers) {
        typeHandlers.forEach((handler) => {
          try {
            handler(event);
          } catch (error) {
            console.error('Error in event handler', {
              type: event.type,
              error,
            });
          }
        });
      }

      // Emit to wildcard handlers
      const wildcardHandlers = this.eventHandlers.get('*' as WsEventType);
      if (wildcardHandlers) {
        wildcardHandlers.forEach((handler) => {
          try {
            handler(event);
          } catch (error) {
            console.error('Error in wildcard event handler', { error });
          }
        });
      }
    } catch (error) {
      console.error('Failed to parse WebSocket message:', error);
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();

    this.heartbeatTimeout = setInterval(() => {
      if (this.socket?.readyState === WebSocket.OPEN) {
        // Send ping frame (text message - server will respond or ignore)
        try {
          this.socket.send(JSON.stringify({ type: 'ping' }));
        } catch {
          // Ignore send errors - onclose will handle disconnection
        }
      }
    }, this.config.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimeout) {
      clearInterval(this.heartbeatTimeout);
      this.heartbeatTimeout = null;
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimeout) {
      return; // Already scheduled
    }

    this.setState('reconnecting');
    this.reconnectAttempts++;

    // Exponential backoff with jitter
    const delay = Math.min(
      this.config.reconnectDelay * 2 ** (this.reconnectAttempts - 1) + Math.random() * 1000,
      this.config.maxReconnectDelay
    );

    this.reconnectTimeout = setTimeout(async () => {
      this.reconnectTimeout = null;
      try {
        await this.createConnection();
      } catch {
        // createConnection handles its own state transitions
      }
    }, delay);
  }

  private cleanup(): void {
    this.stopHeartbeat();

    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    if (this.socket) {
      this.socket.onopen = null;
      this.socket.onclose = null;
      this.socket.onerror = null;
      this.socket.onmessage = null;

      if (this.socket.readyState === WebSocket.OPEN) {
        this.socket.close(1000, 'Client disconnect');
      }
      this.socket = null;
    }
  }
}
