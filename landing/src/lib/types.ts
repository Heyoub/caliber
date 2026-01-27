/**
 * CALIBER API Types
 * Shared TypeScript types for API responses
 */

// HATEOAS Link types
export interface Link {
  /** The URL for this link (absolute or relative path). */
  href: string;
  /** HTTP method to use. Defaults to GET if not specified. */
  method?: string;
  /** Human-readable title for this action. */
  title?: string;
}

/** A collection of named links, keyed by relation name. */
export type Links = Record<string, Link>;

/** A link with its relation name included. */
export interface LinkWithRel extends Link {
  rel: string;
}

// Entity types
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
  _links?: Links;
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
  _links?: Links;
}

export interface Agent {
  id: string;
  tenant_id: string;
  agent_type: string;
  status: 'idle' | 'busy' | 'offline';
  created_at: string;
  updated_at: string;
  _links?: Links;
}

export interface Artifact {
  id: string;
  tenant_id: string;
  name: string;
  artifact_type: string;
  created_at: string;
  updated_at: string;
  _links?: Links;
}

export interface Note {
  id: string;
  tenant_id: string;
  title: string;
  content: string;
  note_type: string;
  created_at: string;
  updated_at: string;
  _links?: Links;
}

export interface Message {
  id: string;
  tenant_id: string;
  from_agent_id: string;
  to_agent_id: string;
  message_type: string;
  content: string;
  created_at: string;
  _links?: Links;
}

export interface Lock {
  id: string;
  tenant_id: string;
  resource_type: string;
  resource_id: string;
  holder_agent_id: string;
  acquired_at: string;
  expires_at?: string;
  _links?: Links;
}

// API Response types
export interface ApiResponse<T> {
  data: T;
  meta?: {
    total?: number;
    page?: number;
    limit?: number;
  };
  _links?: Links;
}

export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

// User types
export interface UserProfile {
  id: string;
  email: string;
  first_name?: string;
  last_name?: string;
  tenant_id?: string;
  api_key?: string;
  created_at: string;
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
