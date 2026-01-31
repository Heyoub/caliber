/**
 * CALIBER API Client
 * Production-ready HTTP/WebSocket client with typed errors, retry logic, and streaming support.
 */
import { getToken } from '$lib/stores/auth';
import type {
  DashboardStats,
  Trajectory,
  Scope,
  Event,
  AssistantResponse,
  ApiResponse,
  ApiError,
  PaginatedResponse,
  CreateTrajectoryRequest,
  UpdateTrajectoryRequest,
  CreateScopeRequest,
  SendMessageRequest,
  StreamEvent,
  StreamEventType,
} from './types';

// Re-export types for convenience
export type { DashboardStats, Trajectory, Scope, Event, AssistantResponse };

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════

/** API base URL - defaults to relative path for same-origin */
const API_BASE = import.meta.env.VITE_API_URL || '/api';

/** WebSocket base URL */
const WS_BASE = import.meta.env.VITE_WS_URL ||
  (typeof window !== 'undefined'
    ? `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`
    : 'ws://localhost:3000/ws');

/** Default request timeout in milliseconds */
const DEFAULT_TIMEOUT = 30000;

/** Maximum retry attempts */
const MAX_RETRIES = 3;

/** Base delay for exponential backoff in ms */
const RETRY_BASE_DELAY = 1000;

// ═══════════════════════════════════════════════════════════════════════════
// ERROR TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * API error codes for typed error handling.
 */
export const ApiErrorCode = {
  NetworkError: 'NETWORK_ERROR',
  Timeout: 'TIMEOUT',
  Unauthorized: 'UNAUTHORIZED',
  Forbidden: 'FORBIDDEN',
  NotFound: 'NOT_FOUND',
  ValidationError: 'VALIDATION_ERROR',
  RateLimit: 'RATE_LIMIT',
  ServerError: 'SERVER_ERROR',
  Unknown: 'UNKNOWN',
} as const;

export type ApiErrorCodeType = (typeof ApiErrorCode)[keyof typeof ApiErrorCode];

/**
 * Typed API error with code, message, and optional details.
 */
export class CaliberApiError extends Error {
  readonly code: ApiErrorCodeType;
  readonly status: number;
  readonly details?: Record<string, unknown>;
  readonly retryable: boolean;

  constructor(options: {
    code: ApiErrorCodeType;
    message: string;
    status: number;
    details?: Record<string, unknown>;
    retryable?: boolean;
  }) {
    super(options.message);
    this.name = 'CaliberApiError';
    this.code = options.code;
    this.status = options.status;
    this.details = options.details;
    this.retryable = options.retryable ?? false;
  }

  static fromHttpStatus(status: number, message?: string, details?: Record<string, unknown>): CaliberApiError {
    const errorMap: Record<number, { code: ApiErrorCodeType; retryable: boolean; defaultMessage: string }> = {
      400: { code: ApiErrorCode.ValidationError, retryable: false, defaultMessage: 'Invalid request' },
      401: { code: ApiErrorCode.Unauthorized, retryable: false, defaultMessage: 'Authentication required' },
      403: { code: ApiErrorCode.Forbidden, retryable: false, defaultMessage: 'Access denied' },
      404: { code: ApiErrorCode.NotFound, retryable: false, defaultMessage: 'Resource not found' },
      429: { code: ApiErrorCode.RateLimit, retryable: true, defaultMessage: 'Rate limit exceeded' },
      500: { code: ApiErrorCode.ServerError, retryable: true, defaultMessage: 'Server error' },
      502: { code: ApiErrorCode.ServerError, retryable: true, defaultMessage: 'Bad gateway' },
      503: { code: ApiErrorCode.ServerError, retryable: true, defaultMessage: 'Service unavailable' },
      504: { code: ApiErrorCode.ServerError, retryable: true, defaultMessage: 'Gateway timeout' },
    };

    const errorInfo = errorMap[status] || {
      code: ApiErrorCode.Unknown,
      retryable: status >= 500,
      defaultMessage: `HTTP Error ${status}`,
    };

    return new CaliberApiError({
      code: errorInfo.code,
      message: message || errorInfo.defaultMessage,
      status,
      details,
      retryable: errorInfo.retryable,
    });
  }

  static networkError(message: string = 'Network error'): CaliberApiError {
    return new CaliberApiError({
      code: ApiErrorCode.NetworkError,
      message,
      status: 0,
      retryable: true,
    });
  }

  static timeout(message: string = 'Request timed out'): CaliberApiError {
    return new CaliberApiError({
      code: ApiErrorCode.Timeout,
      message,
      status: 0,
      retryable: true,
    });
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// REQUEST/RESPONSE INTERCEPTORS
// ═══════════════════════════════════════════════════════════════════════════

export type RequestInterceptor = (config: RequestConfig) => RequestConfig | Promise<RequestConfig>;
export type ResponseInterceptor = <T>(response: T) => T | Promise<T>;
export type ErrorInterceptor = (error: CaliberApiError) => CaliberApiError | Promise<CaliberApiError>;

interface RequestConfig {
  url: string;
  method: string;
  headers: HeadersInit;
  body?: string;
  timeout?: number;
}

const requestInterceptors: RequestInterceptor[] = [];
const responseInterceptors: ResponseInterceptor[] = [];
const errorInterceptors: ErrorInterceptor[] = [];

/**
 * Add a request interceptor.
 */
export function addRequestInterceptor(interceptor: RequestInterceptor): () => void {
  requestInterceptors.push(interceptor);
  return () => {
    const index = requestInterceptors.indexOf(interceptor);
    if (index > -1) requestInterceptors.splice(index, 1);
  };
}

/**
 * Add a response interceptor.
 */
export function addResponseInterceptor(interceptor: ResponseInterceptor): () => void {
  responseInterceptors.push(interceptor);
  return () => {
    const index = responseInterceptors.indexOf(interceptor);
    if (index > -1) responseInterceptors.splice(index, 1);
  };
}

/**
 * Add an error interceptor.
 */
export function addErrorInterceptor(interceptor: ErrorInterceptor): () => void {
  errorInterceptors.push(interceptor);
  return () => {
    const index = errorInterceptors.indexOf(interceptor);
    if (index > -1) errorInterceptors.splice(index, 1);
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// FETCH WITH RETRY AND TIMEOUT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate exponential backoff delay.
 */
function getRetryDelay(attempt: number): number {
  // Exponential backoff with jitter: base * 2^attempt + random(0-1000)ms
  return RETRY_BASE_DELAY * Math.pow(2, attempt) + Math.random() * 1000;
}

/**
 * Fetch with timeout support.
 */
async function fetchWithTimeout(
  url: string,
  options: RequestInit,
  timeout: number
): Promise<Response> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal,
    });
    return response;
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw CaliberApiError.timeout();
    }
    throw CaliberApiError.networkError(error instanceof Error ? error.message : 'Network error');
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Core API fetch wrapper with auth, error handling, retry, and interceptors.
 */
async function apiFetch<T>(
  endpoint: string,
  options: RequestInit = {},
  config: { timeout?: number; retries?: number; retryOn?: ApiErrorCodeType[] } = {}
): Promise<T> {
  const { timeout = DEFAULT_TIMEOUT, retries = 0, retryOn = [] } = config;
  const token = getToken();

  let requestConfig: RequestConfig = {
    url: `${API_BASE}${endpoint}`,
    method: options.method || 'GET',
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...options.headers,
    },
    body: options.body as string | undefined,
    timeout,
  };

  // Apply request interceptors
  for (const interceptor of requestInterceptors) {
    requestConfig = await interceptor(requestConfig);
  }

  let lastError: CaliberApiError | null = null;
  const maxAttempts = retries + 1;

  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    try {
      if (attempt > 0) {
        // Wait before retry with exponential backoff
        await new Promise((resolve) => setTimeout(resolve, getRetryDelay(attempt - 1)));
      }

      const response = await fetchWithTimeout(
        requestConfig.url,
        {
          method: requestConfig.method,
          headers: requestConfig.headers,
          body: requestConfig.body,
        },
        requestConfig.timeout || DEFAULT_TIMEOUT
      );

      if (!response.ok) {
        let errorMessage = `API Error: ${response.status}`;
        let errorDetails: Record<string, unknown> | undefined;

        try {
          const errorData: ApiError = await response.json();
          errorMessage = errorData.error || errorMessage;
          errorDetails = errorData.details;
        } catch {
          // Use default error message
        }

        throw CaliberApiError.fromHttpStatus(response.status, errorMessage, errorDetails);
      }

      // Handle empty responses
      if (response.status === 204) {
        return undefined as T;
      }

      let result = await response.json() as T;

      // Apply response interceptors
      for (const interceptor of responseInterceptors) {
        result = await interceptor(result);
      }

      return result;
    } catch (error) {
      let apiError: CaliberApiError;

      if (error instanceof CaliberApiError) {
        apiError = error;
      } else {
        apiError = CaliberApiError.networkError(error instanceof Error ? error.message : 'Unknown error');
      }

      // Apply error interceptors
      for (const interceptor of errorInterceptors) {
        apiError = await interceptor(apiError);
      }

      lastError = apiError;

      // Check if we should retry
      const shouldRetry =
        attempt < maxAttempts - 1 &&
        apiError.retryable &&
        (retryOn.length === 0 || retryOn.includes(apiError.code));

      if (!shouldRetry) {
        throw apiError;
      }
    }
  }

  throw lastError || CaliberApiError.networkError('Request failed after all retries');
}

// ═══════════════════════════════════════════════════════════════════════════
// WEBSOCKET STREAMING
// ═══════════════════════════════════════════════════════════════════════════

export interface StreamConnection {
  send: (data: unknown) => void;
  close: () => void;
  isConnected: () => boolean;
}

export type StreamHandler = (event: StreamEvent) => void;
export type StreamErrorHandler = (error: CaliberApiError) => void;
export type StreamCloseHandler = () => void;

/**
 * Create a WebSocket connection for streaming.
 */
export function createStreamConnection(
  path: string,
  handlers: {
    onMessage: StreamHandler;
    onError?: StreamErrorHandler;
    onClose?: StreamCloseHandler;
    onOpen?: () => void;
  }
): StreamConnection {
  const token = getToken();
  const url = `${WS_BASE}${path}${token ? `?token=${encodeURIComponent(token)}` : ''}`;

  let ws: WebSocket | null = null;
  let isConnected = false;

  const connect = () => {
    ws = new WebSocket(url);

    ws.onopen = () => {
      isConnected = true;
      handlers.onOpen?.();
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as StreamEvent;
        handlers.onMessage(data);
      } catch (error) {
        console.error('Failed to parse stream message:', error);
      }
    };

    ws.onerror = () => {
      handlers.onError?.(CaliberApiError.networkError('WebSocket error'));
    };

    ws.onclose = () => {
      isConnected = false;
      handlers.onClose?.();
    };
  };

  connect();

  return {
    send: (data: unknown) => {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(data));
      }
    },
    close: () => {
      if (ws) {
        ws.close();
        ws = null;
      }
    },
    isConnected: () => isConnected,
  };
}

/**
 * Stream a chat response from the assistant.
 */
export function streamChatResponse(
  content: string,
  options: {
    trajectoryId?: string;
    scopeId?: string;
    onChunk: (chunk: string) => void;
    onToolCall?: (toolCall: {
      id: string;
      name: string;
      arguments: Record<string, unknown>;
    }) => void;
    onComplete: (response: AssistantResponse) => void;
    onError: (error: CaliberApiError) => void;
  }
): () => void {
  const connection = createStreamConnection('/chat/stream', {
    onMessage: (event) => {
      switch (event.type) {
        case 'chunk':
          options.onChunk(event.data as string);
          break;
        case 'tool_call':
          options.onToolCall?.(event.data as {
            id: string;
            name: string;
            arguments: Record<string, unknown>;
          });
          break;
        case 'complete':
          options.onComplete(event.data as AssistantResponse);
          connection.close();
          break;
        case 'error':
          options.onError(
            new CaliberApiError({
              code: ApiErrorCode.ServerError,
              message: (event.data as { message: string }).message,
              status: 500,
            })
          );
          connection.close();
          break;
      }
    },
    onError: options.onError,
    onOpen: () => {
      connection.send({
        type: 'message',
        content,
        trajectoryId: options.trajectoryId,
        scopeId: options.scopeId,
      });
    },
  });

  return () => connection.close();
}

// ═══════════════════════════════════════════════════════════════════════════
// API CLIENT
// ═══════════════════════════════════════════════════════════════════════════

export const apiClient = {
  // ═══════════════════════════════════════════════════════════════════════
  // DASHBOARD
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get dashboard statistics.
   */
  getDashboardStats: async (): Promise<DashboardStats> => {
    const response = await apiFetch<ApiResponse<DashboardStats>>('/dashboard/stats', {}, {
      retries: 2,
      retryOn: [ApiErrorCode.ServerError, ApiErrorCode.NetworkError],
    });
    return response.data;
  },

  // ═══════════════════════════════════════════════════════════════════════
  // TRAJECTORIES
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get all trajectories.
   */
  getTrajectories: async (): Promise<Trajectory[]> => {
    const response = await apiFetch<ApiResponse<Trajectory[]>>('/trajectories', {}, {
      retries: 2,
    });
    return response.data;
  },

  /**
   * Get trajectories with pagination.
   */
  getTrajectoriesPaginated: async (
    page = 1,
    perPage = 20
  ): Promise<PaginatedResponse<Trajectory>> => {
    return apiFetch<PaginatedResponse<Trajectory>>(
      `/trajectories?page=${page}&per_page=${perPage}`,
      {},
      { retries: 2 }
    );
  },

  /**
   * Get a single trajectory.
   */
  getTrajectory: async (id: string): Promise<Trajectory> => {
    const response = await apiFetch<ApiResponse<Trajectory>>(`/trajectories/${id}`, {}, {
      retries: 2,
    });
    return response.data;
  },

  /**
   * Get the active trajectory.
   */
  getActiveTrajectory: async (): Promise<Trajectory | null> => {
    try {
      const response = await apiFetch<ApiResponse<Trajectory>>('/trajectories/active');
      return response.data;
    } catch (error) {
      if (error instanceof CaliberApiError && error.code === ApiErrorCode.NotFound) {
        return null;
      }
      throw error;
    }
  },

  /**
   * Create a new trajectory.
   */
  createTrajectory: async (name: string, description?: string): Promise<Trajectory> => {
    const body: CreateTrajectoryRequest = { name, description };
    const response = await apiFetch<ApiResponse<Trajectory>>('/trajectories', {
      method: 'POST',
      body: JSON.stringify(body),
    });
    return response.data;
  },

  /**
   * Update a trajectory.
   */
  updateTrajectory: async (id: string, updates: UpdateTrajectoryRequest): Promise<Trajectory> => {
    const response = await apiFetch<ApiResponse<Trajectory>>(`/trajectories/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(updates),
    });
    return response.data;
  },

  /**
   * Delete a trajectory.
   */
  deleteTrajectory: async (id: string): Promise<void> => {
    await apiFetch(`/trajectories/${id}`, { method: 'DELETE' });
  },

  // ═══════════════════════════════════════════════════════════════════════
  // SCOPES
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get scopes for a trajectory.
   */
  getScopes: async (trajectoryId: string): Promise<Scope[]> => {
    const response = await apiFetch<ApiResponse<Scope[]>>(
      `/trajectories/${trajectoryId}/scopes`,
      {},
      { retries: 2 }
    );
    return response.data;
  },

  /**
   * Get a single scope.
   */
  getScope: async (id: string): Promise<Scope> => {
    const response = await apiFetch<ApiResponse<Scope>>(`/scopes/${id}`, {}, { retries: 2 });
    return response.data;
  },

  /**
   * Create a new scope.
   */
  createScope: async (request: CreateScopeRequest): Promise<Scope> => {
    const response = await apiFetch<ApiResponse<Scope>>('/scopes', {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return response.data;
  },

  /**
   * Fork an existing scope.
   */
  forkScope: async (
    scopeId: string,
    options: { name: string; fromEventId?: string }
  ): Promise<Scope> => {
    const response = await apiFetch<ApiResponse<Scope>>(`/scopes/${scopeId}/fork`, {
      method: 'POST',
      body: JSON.stringify(options),
    });
    return response.data;
  },

  /**
   * Update scope metadata.
   */
  updateScope: async (
    id: string,
    updates: { name?: string; memoryLimit?: number; tags?: string[] }
  ): Promise<Scope> => {
    const response = await apiFetch<ApiResponse<Scope>>(`/scopes/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(updates),
    });
    return response.data;
  },

  /**
   * Delete a scope.
   */
  deleteScope: async (id: string): Promise<void> => {
    await apiFetch(`/scopes/${id}`, { method: 'DELETE' });
  },

  // ═══════════════════════════════════════════════════════════════════════
  // EVENTS (TURNS)
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get events for a scope.
   */
  getScopeEvents: async (scopeId: string): Promise<Event[]> => {
    const response = await apiFetch<ApiResponse<Event[]>>(
      `/scopes/${scopeId}/events`,
      {},
      { retries: 2 }
    );
    return response.data;
  },

  /**
   * Get a single event.
   */
  getEvent: async (id: string): Promise<Event> => {
    const response = await apiFetch<ApiResponse<Event>>(`/events/${id}`, {}, { retries: 2 });
    return response.data;
  },

  /**
   * Update an event's content.
   */
  updateEvent: async (id: string, content: string): Promise<Event> => {
    const response = await apiFetch<ApiResponse<Event>>(`/events/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ content }),
    });
    return response.data;
  },

  /**
   * Delete an event.
   */
  deleteEvent: async (id: string): Promise<void> => {
    await apiFetch(`/events/${id}`, { method: 'DELETE' });
  },

  /**
   * Reorder events within a scope.
   */
  reorderEvents: async (scopeId: string, eventIds: string[]): Promise<Event[]> => {
    const response = await apiFetch<ApiResponse<Event[]>>(`/scopes/${scopeId}/events/reorder`, {
      method: 'POST',
      body: JSON.stringify({ eventIds }),
    });
    return response.data;
  },

  // ═══════════════════════════════════════════════════════════════════════
  // ASSISTANT
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Send a message to the assistant (non-streaming).
   */
  sendMessage: async (
    content: string,
    trajectoryId?: string,
    scopeId?: string
  ): Promise<AssistantResponse> => {
    const body: SendMessageRequest = { content, trajectoryId, scopeId };
    const response = await apiFetch<ApiResponse<AssistantResponse>>(
      '/assistant/message',
      {
        method: 'POST',
        body: JSON.stringify(body),
      },
      { timeout: 120000 } // 2 minute timeout for AI responses
    );
    return response.data;
  },

  /**
   * Stream a message to the assistant.
   */
  streamMessage: streamChatResponse,

  /**
   * Approve a tool call.
   */
  approveToolCall: async (toolCallId: string): Promise<void> => {
    await apiFetch(`/assistant/tool-calls/${toolCallId}/approve`, { method: 'POST' });
  },

  /**
   * Reject a tool call.
   */
  rejectToolCall: async (toolCallId: string, reason?: string): Promise<void> => {
    await apiFetch(`/assistant/tool-calls/${toolCallId}/reject`, {
      method: 'POST',
      body: JSON.stringify({ reason }),
    });
  },

  // ═══════════════════════════════════════════════════════════════════════
  // RESOURCES
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get a resource by URI.
   */
  getResource: async (uri: string): Promise<{ content: string; mimeType: string }> => {
    const response = await apiFetch<ApiResponse<{ content: string; mimeType: string }>>(
      `/resources?uri=${encodeURIComponent(uri)}`,
      {},
      { retries: 2 }
    );
    return response.data;
  },

  /**
   * Update a resource.
   */
  updateResource: async (uri: string, content: string): Promise<void> => {
    await apiFetch('/resources', {
      method: 'PUT',
      body: JSON.stringify({ uri, content }),
    });
  },

  /**
   * List resources matching a pattern.
   */
  listResources: async (pattern?: string): Promise<Array<{ uri: string; name: string; mimeType: string }>> => {
    const query = pattern ? `?pattern=${encodeURIComponent(pattern)}` : '';
    const response = await apiFetch<ApiResponse<Array<{ uri: string; name: string; mimeType: string }>>>(
      `/resources/list${query}`,
      {},
      { retries: 2 }
    );
    return response.data;
  },
};
