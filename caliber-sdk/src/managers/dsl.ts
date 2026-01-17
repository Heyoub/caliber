/**
 * DSL Manager
 *
 * Validates and parses CALIBER DSL configurations.
 */

import { BaseManager } from './base';
import type { HttpClient } from '../http';
import type { ValidateDslResponse } from '../types';

/**
 * Response from parse endpoint (same as validate, but includes AST)
 * Both endpoints use ValidateDslResponse - parse includes the `ast` field.
 */
export type ParseDslResponse = ValidateDslResponse;

/**
 * Manager for DSL operations
 */
export class DslManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/dsl');
  }

  /**
   * Validate DSL source code (syntax check only)
   */
  async validate(source: string): Promise<ValidateDslResponse> {
    return this.http.post<ValidateDslResponse>(`${this.basePath}/validate`, { source });
  }

  /**
   * Parse DSL source code to AST
   * Returns the same structure as validate, but with the `ast` field populated.
   */
  async parse(source: string): Promise<ParseDslResponse> {
    return this.http.post<ParseDslResponse>(`${this.basePath}/parse`, { source });
  }
}
