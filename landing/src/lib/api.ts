/**
 * CALIBER API Client
 * Authenticated HTTP client for CALIBER Cloud API
 */

import { getToken, logout } from '../stores/auth';

// API base URL from environment or default
const API_URL = import.meta.env.PUBLIC_API_URL || 'https://api.caliber.run';

// Types
export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

export interface ApiResponse<T> {
  data: T;
  meta?: {
    total?: number;
    page?: number;
    limit?: number;
  };
}

export interface Trajectory {
  id: string;
  tenant_id: string;
  name: string;
  description?: string;
  created_at: string;
  updated_at: string;
  scope_count?: number;
  turn_count?: number;
  status?: 'active' | 'completed' | 'archived';
}

export interface Scope {
  id: string;
  trajectory_id: string;
  parent_id?: string;
  name: string;
  kind: string;
  status: 'open' | 'closed' | 'suspended';
  created_at: string;
  updated_at: string;
}

export interface BillingStatus {
  tenant_id: string;
  plan: 'trial' | 'pro' | 'enterprise';
  trial_ends_at?: string;
  storage_used_bytes: number;
  storage_limit_bytes: number;
  hot_cache_used_bytes: number;
  hot_cache_limit_bytes: number;
}

export interface UserProfile {
  id: string;
  email: string;
  first_name?: string;
  last_name?: string;
  tenant_id?: string;
  api_key?: string;
  created_at: string;
}

// HTTP client class
class CaliberApi {
  private baseUrl: string;

  constructor(baseUrl: string = API_URL) {
    this.baseUrl = baseUrl;
  }

  private async request<T>(
    method: string,
    path: string,
    options: {
      body?: unknown;
      params?: Record<string, string>;
      headers?: Record<string, string>;
    } = {}
  ): Promise<T> {
    const token = getToken();
    const url = new URL(path, this.baseUrl);

    // Add query params
    if (options.params) {
      Object.entries(options.params).forEach(([key, value]) => {
        url.searchParams.set(key, value);
      });
    }

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...options.headers,
    };

    // Add auth token if available
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    const response = await fetch(url.toString(), {
      method,
      headers,
      body: options.body ? JSON.stringify(options.body) : undefined,
    });

    // Handle auth errors
    if (response.status === 401) {
      logout();
      throw new Error('Session expired. Please log in again.');
    }

    // Parse response
    const data = await response.json();

    if (!response.ok) {
      const error = data as ApiError;
      throw new Error(error.message || `API error: ${response.status}`);
    }

    return data as T;
  }

  // Auth endpoints
  async getAuthUrl(params: { organization?: string; loginHint?: string } = {}): Promise<string> {
    const url = new URL('/auth/sso/authorize', this.baseUrl);
    if (params.organization) {
      url.searchParams.set('organization', params.organization);
    }
    if (params.loginHint) {
      url.searchParams.set('login_hint', params.loginHint);
    }
    // Set redirect_uri for web clients
    url.searchParams.set('redirect_uri', `${window.location.origin}/auth/callback`);
    return url.toString();
  }

  // User endpoints
  async getCurrentUser(): Promise<UserProfile> {
    return this.request<UserProfile>('GET', '/api/v1/users/me');
  }

  async regenerateApiKey(): Promise<{ api_key: string }> {
    return this.request<{ api_key: string }>('POST', '/api/v1/users/me/api-key');
  }

  // Trajectory endpoints
  async listTrajectories(params: {
    page?: number;
    limit?: number;
    status?: string;
  } = {}): Promise<ApiResponse<Trajectory[]>> {
    const queryParams: Record<string, string> = {};
    if (params.page) queryParams.page = String(params.page);
    if (params.limit) queryParams.limit = String(params.limit);
    if (params.status) queryParams.status = params.status;

    return this.request<ApiResponse<Trajectory[]>>('GET', '/api/v1/trajectories', {
      params: queryParams,
    });
  }

  async getTrajectory(id: string): Promise<Trajectory> {
    return this.request<Trajectory>('GET', `/api/v1/trajectories/${id}`);
  }

  // Scope endpoints
  async listScopes(trajectoryId: string): Promise<ApiResponse<Scope[]>> {
    return this.request<ApiResponse<Scope[]>>('GET', `/api/v1/trajectories/${trajectoryId}/scopes`);
  }

  // Billing endpoints
  async getBillingStatus(): Promise<BillingStatus> {
    return this.request<BillingStatus>('GET', '/api/v1/billing/status');
  }

  async createCheckoutSession(params: {
    plan: string;
    variantId?: string;
    successUrl?: string;
    cancelUrl?: string;
  }): Promise<{ checkout_url: string }> {
    return this.request<{ checkout_url: string }>('POST', '/api/v1/billing/checkout', {
      body: params,
    });
  }

  async getPortalUrl(): Promise<{ portal_url: string }> {
    return this.request<{ portal_url: string }>('GET', '/api/v1/billing/portal');
  }
}

// Export singleton instance
export const api = new CaliberApi();

// Export class for testing/custom instances
export { CaliberApi };
