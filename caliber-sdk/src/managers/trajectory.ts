/**
 * Trajectory Manager
 *
 * Manages trajectories (task containers) in CALIBER.
 */

import { BaseManager } from './base';
import type { HttpClient } from '../http';
import type {
  Trajectory,
  CreateTrajectoryParams,
  UpdateTrajectoryParams,
  ListTrajectoriesParams,
  ListTrajectoriesResponse,
} from '../types';

/**
 * Manager for trajectory operations
 */
export class TrajectoryManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/trajectories');
  }

  /**
   * Create a new trajectory
   */
  async create(params: CreateTrajectoryParams): Promise<Trajectory> {
    return this.http.post<Trajectory>(this.basePath, params);
  }

  /**
   * Get a trajectory by ID
   */
  async get(trajectoryId: string): Promise<Trajectory> {
    return this.http.get<Trajectory>(this.pathWithId(trajectoryId));
  }

  /**
   * List trajectories with optional filters
   */
  async list(params: ListTrajectoriesParams = {}): Promise<ListTrajectoriesResponse> {
    return this.http.get<ListTrajectoriesResponse>(this.basePath, {
      params: this.buildParams(params),
    });
  }

  /**
   * Update a trajectory
   */
  async update(trajectoryId: string, params: UpdateTrajectoryParams): Promise<Trajectory> {
    return this.http.patch<Trajectory>(this.pathWithId(trajectoryId), params);
  }

  /**
   * Delete a trajectory
   */
  async delete(trajectoryId: string): Promise<void> {
    await this.http.delete(this.pathWithId(trajectoryId));
  }

  /**
   * List scopes for a trajectory
   */
  async listScopes(trajectoryId: string): Promise<{ scopes: unknown[]; total: number }> {
    return this.http.get(`${this.pathWithId(trajectoryId)}/scopes`);
  }

  /**
   * List child trajectories
   */
  async listChildren(trajectoryId: string): Promise<ListTrajectoriesResponse> {
    return this.http.get(`${this.pathWithId(trajectoryId)}/children`);
  }
}
