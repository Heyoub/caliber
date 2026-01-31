/**
 * API Client
 * Thin REST wrapper for CALIBER API
 */
import { getToken } from '$stores/auth';
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
} from './types';

// Re-export types for convenience
export type { DashboardStats, Trajectory, Scope, Event, AssistantResponse };

// API base URL - defaults to relative path for same-origin
const API_BASE = import.meta.env.VITE_API_URL || '/api';

/**
 * API fetch wrapper with auth and error handling
 */
async function apiFetch<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const token = getToken();

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...options.headers,
  };

  if (token) {
    (headers as Record<string, string>)['Authorization'] = `Bearer ${token}`;
  }

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    let errorMessage = `API Error: ${response.status}`;

    try {
      const errorData: ApiError = await response.json();
      errorMessage = errorData.error || errorMessage;
    } catch {
      // Use default error message
    }

    throw new Error(errorMessage);
  }

  // Handle empty responses
  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

/**
 * API Client
 */
export const apiClient = {
  // ═══════════════════════════════════════════════════════════════════════
  // DASHBOARD
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get dashboard statistics
   */
  getDashboardStats: async (): Promise<DashboardStats> => {
    const response = await apiFetch<ApiResponse<DashboardStats>>('/dashboard/stats');
    return response.data;
  },

  // ═══════════════════════════════════════════════════════════════════════
  // TRAJECTORIES
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get all trajectories
   */
  getTrajectories: async (): Promise<Trajectory[]> => {
    const response = await apiFetch<ApiResponse<Trajectory[]>>('/trajectories');
    return response.data;
  },

  /**
   * Get trajectories with pagination
   */
  gettrajectoriesPaginated: async (
    page = 1,
    perPage = 20
  ): Promise<PaginatedResponse<Trajectory>> => {
    return apiFetch<PaginatedResponse<Trajectory>>(
      `/trajectories?page=${page}&per_page=${perPage}`
    );
  },

  /**
   * Get a single trajectory
   */
  getTrajectory: async (id: string): Promise<Trajectory> => {
    const response = await apiFetch<ApiResponse<Trajectory>>(`/trajectories/${id}`);
    return response.data;
  },

  /**
   * Get the active trajectory
   */
  getActiveTrajectory: async (): Promise<Trajectory | null> => {
    try {
      const response = await apiFetch<ApiResponse<Trajectory>>('/trajectories/active');
      return response.data;
    } catch {
      return null;
    }
  },

  /**
   * Create a new trajectory
   */
  createTrajectory: async (
    name: string,
    description?: string
  ): Promise<Trajectory> => {
    const body: CreateTrajectoryRequest = { name, description };
    const response = await apiFetch<ApiResponse<Trajectory>>('/trajectories', {
      method: 'POST',
      body: JSON.stringify(body),
    });
    return response.data;
  },

  /**
   * Update a trajectory
   */
  updateTrajectory: async (
    id: string,
    updates: UpdateTrajectoryRequest
  ): Promise<Trajectory> => {
    const response = await apiFetch<ApiResponse<Trajectory>>(`/trajectories/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(updates),
    });
    return response.data;
  },

  /**
   * Delete a trajectory
   */
  deleteTrajectory: async (id: string): Promise<void> => {
    await apiFetch(`/trajectories/${id}`, {
      method: 'DELETE',
    });
  },

  // ═══════════════════════════════════════════════════════════════════════
  // SCOPES
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get scopes for a trajectory
   */
  getScopes: async (trajectoryId: string): Promise<Scope[]> => {
    const response = await apiFetch<ApiResponse<Scope[]>>(
      `/trajectories/${trajectoryId}/scopes`
    );
    return response.data;
  },

  /**
   * Get a single scope
   */
  getScope: async (id: string): Promise<Scope> => {
    const response = await apiFetch<ApiResponse<Scope>>(`/scopes/${id}`);
    return response.data;
  },

  /**
   * Create a new scope
   */
  createScope: async (request: CreateScopeRequest): Promise<Scope> => {
    const response = await apiFetch<ApiResponse<Scope>>('/scopes', {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return response.data;
  },

  /**
   * Delete a scope
   */
  deleteScope: async (id: string): Promise<void> => {
    await apiFetch(`/scopes/${id}`, {
      method: 'DELETE',
    });
  },

  // ═══════════════════════════════════════════════════════════════════════
  // EVENTS
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get events for a scope
   */
  getScopeEvents: async (scopeId: string): Promise<Event[]> => {
    const response = await apiFetch<ApiResponse<Event[]>>(`/scopes/${scopeId}/events`);
    return response.data;
  },

  /**
   * Get a single event
   */
  getEvent: async (id: string): Promise<Event> => {
    const response = await apiFetch<ApiResponse<Event>>(`/events/${id}`);
    return response.data;
  },

  // ═══════════════════════════════════════════════════════════════════════
  // ASSISTANT
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Send a message to the assistant
   */
  sendMessage: async (
    content: string,
    trajectoryId?: string,
    scopeId?: string
  ): Promise<AssistantResponse> => {
    const body: SendMessageRequest = { content, trajectoryId, scopeId };
    const response = await apiFetch<ApiResponse<AssistantResponse>>('/assistant/message', {
      method: 'POST',
      body: JSON.stringify(body),
    });
    return response.data;
  },

  /**
   * Approve a tool call
   */
  approveToolCall: async (toolCallId: string): Promise<void> => {
    await apiFetch(`/assistant/tool-calls/${toolCallId}/approve`, {
      method: 'POST',
    });
  },

  /**
   * Reject a tool call
   */
  rejectToolCall: async (toolCallId: string): Promise<void> => {
    await apiFetch(`/assistant/tool-calls/${toolCallId}/reject`, {
      method: 'POST',
    });
  },

  // ═══════════════════════════════════════════════════════════════════════
  // RESOURCES
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get a resource by URI
   */
  getResource: async (uri: string): Promise<{ content: string; mimeType: string }> => {
    const response = await apiFetch<ApiResponse<{ content: string; mimeType: string }>>(
      `/resources?uri=${encodeURIComponent(uri)}`
    );
    return response.data;
  },

  /**
   * Update a resource
   */
  updateResource: async (uri: string, content: string): Promise<void> => {
    await apiFetch('/resources', {
      method: 'PUT',
      body: JSON.stringify({ uri, content }),
    });
  },
};
