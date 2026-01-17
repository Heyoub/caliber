/**
 * Base Manager Class
 *
 * Provides common functionality for all resource managers.
 */

import { HttpClient, RequestOptions } from '../http';

/**
 * Pagination parameters
 */
export interface PaginationParams {
  limit?: number;
  offset?: number;
}

/**
 * Paginated response wrapper
 */
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
}

/**
 * Base class for all resource managers
 */
export abstract class BaseManager {
  protected readonly http: HttpClient;
  protected readonly basePath: string;

  constructor(http: HttpClient, basePath: string) {
    this.http = http;
    this.basePath = basePath;
  }

  /**
   * Build query parameters, filtering out undefined values
   */
  protected buildParams(params: Record<string, unknown>): Record<string, unknown> {
    const result: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined && value !== null) {
        result[key] = value;
      }
    }
    return result;
  }

  /**
   * Build full path with ID
   */
  protected pathWithId(id: string): string {
    return `${this.basePath}/${id}`;
  }
}
