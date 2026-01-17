/**
 * Handoff Manager
 *
 * Manages context handoffs between agents in CALIBER.
 */

import { BaseManager } from './base';
import { HttpClient } from '../http';
import type { Handoff, CreateHandoffParams } from '../types';

/**
 * Manager for handoff operations
 */
export class HandoffManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/handoffs');
  }

  /**
   * Create a new handoff
   */
  async create(params: CreateHandoffParams): Promise<Handoff> {
    return this.http.post<Handoff>(this.basePath, params);
  }

  /**
   * Get a handoff by ID
   */
  async get(handoffId: string): Promise<Handoff> {
    return this.http.get<Handoff>(this.pathWithId(handoffId));
  }

  /**
   * Accept a handoff
   */
  async accept(handoffId: string): Promise<Handoff> {
    return this.http.post<Handoff>(`${this.pathWithId(handoffId)}/accept`);
  }

  /**
   * Complete a handoff
   */
  async complete(handoffId: string): Promise<Handoff> {
    return this.http.post<Handoff>(`${this.pathWithId(handoffId)}/complete`);
  }
}
