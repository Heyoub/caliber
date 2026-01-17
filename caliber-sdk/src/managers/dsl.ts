/**
 * DSL Manager
 *
 * Validates and parses CALIBER DSL configurations.
 */

import { BaseManager } from './base';
import { HttpClient } from '../http';
import type { ValidateDslResponse, ParseDslResponse } from '../types';

/**
 * Manager for DSL operations
 */
export class DslManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/dsl');
  }

  /**
   * Validate DSL source code
   */
  async validate(source: string): Promise<ValidateDslResponse> {
    return this.http.post<ValidateDslResponse>(`${this.basePath}/validate`, { source });
  }

  /**
   * Parse DSL source code to AST
   */
  async parse(source: string): Promise<ParseDslResponse> {
    return this.http.post<ParseDslResponse>(`${this.basePath}/parse`, { source });
  }
}
