/**
 * Delegation Manager
 *
 * Manages task delegations between agents in CALIBER.
 */

import { BaseManager } from './base';
import { HttpClient } from '../http';
import type {
  Delegation,
  CreateDelegationParams,
  CompleteDelegationParams,
} from '../types';

/**
 * Manager for delegation operations
 */
export class DelegationManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/delegations');
  }

  /**
   * Create a new delegation
   */
  async create(params: CreateDelegationParams): Promise<Delegation> {
    return this.http.post<Delegation>(this.basePath, params);
  }

  /**
   * Get a delegation by ID
   */
  async get(delegationId: string): Promise<Delegation> {
    return this.http.get<Delegation>(this.pathWithId(delegationId));
  }

  /**
   * Accept a delegation
   */
  async accept(delegationId: string): Promise<Delegation> {
    return this.http.post<Delegation>(`${this.pathWithId(delegationId)}/accept`);
  }

  /**
   * Reject a delegation
   */
  async reject(delegationId: string, reason: string): Promise<Delegation> {
    return this.http.post<Delegation>(`${this.pathWithId(delegationId)}/reject`, { reason });
  }

  /**
   * Complete a delegation with results
   */
  async complete(delegationId: string, params: CompleteDelegationParams): Promise<Delegation> {
    return this.http.post<Delegation>(`${this.pathWithId(delegationId)}/complete`, params);
  }
}
