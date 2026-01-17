/**
 * Agent Manager
 *
 * Manages agent registration and lifecycle in CALIBER.
 */

import { BaseManager } from './base';
import { HttpClient } from '../http';
import type {
  Agent,
  RegisterAgentParams,
  UpdateAgentParams,
  ListAgentsParams,
  ListAgentsResponse,
  HeartbeatResponse,
} from '../types';

/**
 * Manager for agent operations
 */
export class AgentManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/agents');
  }

  /**
   * Register a new agent
   */
  async register(params: RegisterAgentParams): Promise<Agent> {
    return this.http.post<Agent>(`${this.basePath}/register`, params);
  }

  /**
   * Get an agent by ID
   */
  async get(agentId: string): Promise<Agent> {
    return this.http.get<Agent>(this.pathWithId(agentId));
  }

  /**
   * List agents with optional filters
   */
  async list(params: ListAgentsParams = {}): Promise<ListAgentsResponse> {
    return this.http.get<ListAgentsResponse>(this.basePath, {
      params: this.buildParams(params),
    });
  }

  /**
   * Update an agent
   */
  async update(agentId: string, params: UpdateAgentParams): Promise<Agent> {
    return this.http.patch<Agent>(this.pathWithId(agentId), params);
  }

  /**
   * Unregister an agent
   */
  async unregister(agentId: string): Promise<void> {
    await this.http.delete(this.pathWithId(agentId));
  }

  /**
   * Send heartbeat for an agent
   */
  async heartbeat(agentId: string): Promise<HeartbeatResponse> {
    return this.http.post<HeartbeatResponse>(`${this.pathWithId(agentId)}/heartbeat`);
  }
}
