/**
 * Scope Manager
 *
 * Manages scopes (context windows) in CALIBER.
 */

import { BaseManager } from './base';
import type { HttpClient } from '../http';
import type {
  Scope,
  CreateScopeParams,
  UpdateScopeParams,
  CreateCheckpointParams,
  Checkpoint,
} from '../types';

/**
 * Manager for scope operations
 */
export class ScopeManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/scopes');
  }

  /**
   * Create a new scope
   */
  async create(params: CreateScopeParams): Promise<Scope> {
    return this.http.post<Scope>(this.basePath, params);
  }

  /**
   * Get a scope by ID
   */
  async get(scopeId: string): Promise<Scope> {
    return this.http.get<Scope>(this.pathWithId(scopeId));
  }

  /**
   * Update a scope
   */
  async update(scopeId: string, params: UpdateScopeParams): Promise<Scope> {
    return this.http.patch<Scope>(this.pathWithId(scopeId), params);
  }

  /**
   * Create a checkpoint for a scope
   */
  async checkpoint(scopeId: string, params: CreateCheckpointParams): Promise<Checkpoint> {
    return this.http.post<Checkpoint>(`${this.pathWithId(scopeId)}/checkpoint`, params);
  }

  /**
   * Close a scope
   *
   * WARNING: Closing a scope deletes all turns within it.
   * Extract important information to artifacts before closing.
   */
  async close(scopeId: string): Promise<Scope> {
    return this.http.post<Scope>(`${this.pathWithId(scopeId)}/close`);
  }

  /**
   * List turns in a scope
   */
  async listTurns(scopeId: string): Promise<{ turns: unknown[]; total: number }> {
    return this.http.get(`${this.pathWithId(scopeId)}/turns`);
  }

  /**
   * List artifacts in a scope
   */
  async listArtifacts(scopeId: string): Promise<{ artifacts: unknown[]; total: number }> {
    return this.http.get(`${this.pathWithId(scopeId)}/artifacts`);
  }
}
