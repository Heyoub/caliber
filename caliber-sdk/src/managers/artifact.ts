/**
 * Artifact Manager
 *
 * Manages artifacts (extracted valuable outputs) in CALIBER.
 */

import { BaseManager } from './base';
import type { HttpClient } from '../http';
import type {
  Artifact,
  CreateArtifactParams,
  UpdateArtifactParams,
  ListArtifactsParams,
  ListArtifactsResponse,
  SearchArtifactsParams,
  SearchResponse,
} from '../types';

/**
 * Manager for artifact operations
 */
export class ArtifactManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/artifacts');
  }

  /**
   * Create a new artifact
   */
  async create(params: CreateArtifactParams): Promise<Artifact> {
    return this.http.post<Artifact>(this.basePath, params);
  }

  /**
   * Get an artifact by ID
   */
  async get(artifactId: string): Promise<Artifact> {
    return this.http.get<Artifact>(this.pathWithId(artifactId));
  }

  /**
   * List artifacts with optional filters
   */
  async list(params: ListArtifactsParams = {}): Promise<ListArtifactsResponse> {
    return this.http.get<ListArtifactsResponse>(this.basePath, {
      params: this.buildParams(params),
    });
  }

  /**
   * Update an artifact
   */
  async update(artifactId: string, params: UpdateArtifactParams): Promise<Artifact> {
    return this.http.patch<Artifact>(this.pathWithId(artifactId), params);
  }

  /**
   * Delete an artifact
   */
  async delete(artifactId: string): Promise<void> {
    await this.http.delete(this.pathWithId(artifactId));
  }

  /**
   * Search artifacts by content
   */
  async search(params: SearchArtifactsParams): Promise<SearchResponse> {
    return this.http.post<SearchResponse>(`${this.basePath}/search`, params);
  }
}
