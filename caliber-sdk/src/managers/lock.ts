/**
 * Lock Manager
 *
 * Manages distributed locks for resource coordination in CALIBER.
 */

import { BaseManager } from './base';
import type { HttpClient } from '../http';
import type {
  Lock,
  AcquireLockParams,
  ExtendLockParams,
  ListLocksParams,
  ListLocksResponse,
} from '../types';

/**
 * Manager for lock operations
 */
export class LockManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/locks');
  }

  /**
   * Acquire a lock on a resource
   */
  async acquire(params: AcquireLockParams): Promise<Lock> {
    return this.http.post<Lock>(`${this.basePath}/acquire`, params);
  }

  /**
   * Get a lock by ID
   */
  async get(lockId: string): Promise<Lock> {
    return this.http.get<Lock>(this.pathWithId(lockId));
  }

  /**
   * List locks with optional filters
   */
  async list(params: ListLocksParams = {}): Promise<ListLocksResponse> {
    return this.http.get<ListLocksResponse>(this.basePath, {
      params: this.buildParams(params),
    });
  }

  /**
   * Release a lock
   */
  async release(lockId: string): Promise<void> {
    await this.http.delete(`${this.pathWithId(lockId)}/release`);
  }

  /**
   * Extend a lock's expiration
   */
  async extend(lockId: string, params: ExtendLockParams): Promise<Lock> {
    return this.http.patch<Lock>(`${this.pathWithId(lockId)}/extend`, params);
  }
}
