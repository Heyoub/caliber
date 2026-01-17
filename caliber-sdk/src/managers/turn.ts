/**
 * Turn Manager
 *
 * Manages turns (ephemeral conversation messages) in CALIBER.
 */

import { BaseManager } from './base';
import type { HttpClient } from '../http';
import type { Turn, CreateTurnParams } from '../types';

/**
 * Manager for turn operations
 *
 * Note: Turns are ephemeral and are deleted when their scope closes.
 * Extract important information to artifacts before closing a scope.
 */
export class TurnManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/turns');
  }

  /**
   * Create a new turn
   */
  async create(params: CreateTurnParams): Promise<Turn> {
    return this.http.post<Turn>(this.basePath, params);
  }

  /**
   * Get a turn by ID
   */
  async get(turnId: string): Promise<Turn> {
    return this.http.get<Turn>(this.pathWithId(turnId));
  }
}
