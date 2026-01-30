/**
 * CALIBER SDK HTTP Client
 *
 * Wraps axios with CALIBER-specific configuration, authentication,
 * and error handling.
 */

import axios, { type AxiosInstance, type AxiosRequestConfig } from 'axios';
import { CaliberError, parseApiError, AuthenticationError } from './errors';

export interface HttpClientConfig {
  /** Base URL for the CALIBER API */
  baseUrl: string;
  /** API key for authentication */
  apiKey: string;
  /** Tenant ID for multi-tenant isolation */
  tenantId: string;
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;
  /** Custom headers to include in all requests */
  headers?: Record<string, string>;
}

export interface RequestOptions {
  /** Query parameters */
  params?: Record<string, unknown>;
  /** Request headers */
  headers?: Record<string, string>;
  /** Request timeout override */
  timeout?: number;
}

/**
 * HTTP client for CALIBER API requests
 */
export class HttpClient {
  private readonly client: AxiosInstance;
  private readonly tenantId: string;

  constructor(config: HttpClientConfig) {
    if (!config.apiKey) {
      throw new AuthenticationError('API key is required');
    }
    if (!config.tenantId) {
      throw new CaliberError('Tenant ID is required', 'CONFIGURATION_ERROR');
    }

    this.tenantId = config.tenantId;

    this.client = axios.create({
      baseURL: config.baseUrl.replace(/\/$/, ''),
      timeout: config.timeout ?? 30000,
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${config.apiKey}`,
        'X-Tenant-ID': config.tenantId,
        ...config.headers,
      },
    });

    // Response interceptor for error handling
    this.client.interceptors.response.use(
      (response) => response,
      (error) => {
        if (axios.isAxiosError(error)) {
          const statusCode = error.response?.status ?? 500;
          const body = error.response?.data;
          throw parseApiError(statusCode, body);
        }
        throw new CaliberError(error.message ?? 'Network error', 'NETWORK_ERROR');
      }
    );
  }

  /**
   * Get the tenant ID for this client
   */
  getTenantId(): string {
    return this.tenantId;
  }

  /**
   * Perform a GET request
   */
  async get<T>(path: string, options?: RequestOptions): Promise<T> {
    const response = await this.client.get<T>(path, this.buildConfig(options));
    return response.data;
  }

  /**
   * Perform a POST request
   */
  async post<T>(path: string, data?: unknown, options?: RequestOptions): Promise<T> {
    const response = await this.client.post<T>(path, data, this.buildConfig(options));
    return response.data;
  }

  /**
   * Perform a PATCH request
   */
  async patch<T>(path: string, data?: unknown, options?: RequestOptions): Promise<T> {
    const response = await this.client.patch<T>(path, data, this.buildConfig(options));
    return response.data;
  }

  /**
   * Perform a PUT request
   */
  async put<T>(path: string, data?: unknown, options?: RequestOptions): Promise<T> {
    const response = await this.client.put<T>(path, data, this.buildConfig(options));
    return response.data;
  }

  /**
   * Perform a DELETE request
   */
  async delete<T = void>(path: string, options?: RequestOptions): Promise<T> {
    const response = await this.client.delete<T>(path, this.buildConfig(options));
    return response.data;
  }

  /**
   * Build axios config from request options
   */
  private buildConfig(options?: RequestOptions): AxiosRequestConfig {
    if (!options) return {};

    return {
      params: options.params,
      headers: options.headers,
      timeout: options.timeout,
    };
  }
}
