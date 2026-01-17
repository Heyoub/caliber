/**
 * Search Manager
 *
 * Provides global search across CALIBER entities.
 */

import { BaseManager } from './base';
import type { HttpClient } from '../http';
import type { SearchParams, SearchResponse } from '../types';

/**
 * Manager for global search operations
 */
export class SearchManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/search');
  }

  /**
   * Search across entities
   */
  async search(params: SearchParams): Promise<SearchResponse> {
    return this.http.post<SearchResponse>(this.basePath, params);
  }
}
