/**
 * Pack API client
 *
 * NOTE: Some endpoints are pending backend implementation:
 * - GET /api/v1/pack/history (new)
 * - GET /api/v1/pack/version/:id (new)
 * - GET /api/v1/pack/diff (new)
 * - POST /api/v1/pack/revert (new)
 *
 * Existing endpoints:
 * - GET /api/v1/pack/inspect (exists)
 * - POST /api/v1/dsl/compose (exists)
 * - POST /api/v1/dsl/deploy (exists)
 */

import { apiRequest, apiMultipart } from './client';
import type {
  PackHistoryResponse,
  PackVersionResponse,
  PackDiffResponse,
  PackInspectResponse,
  ComposePackResponse,
  DeployDslRequest,
  DeployDslResponse,
  PackRevertRequest,
  PackSourceFile,
} from '../../types';

export const packApi = {
  /**
   * Get version history for a pack
   * NOTE: Endpoint pending backend implementation
   */
  async getHistory(name: string, limit = 50): Promise<PackHistoryResponse> {
    return apiRequest(
      `/api/v1/pack/history?name=${encodeURIComponent(name)}&limit=${limit}`
    );
  },

  /**
   * Get a specific pack version
   * NOTE: Endpoint pending backend implementation
   */
  async getVersion(configId: string): Promise<PackVersionResponse> {
    return apiRequest(`/api/v1/pack/version/${configId}`);
  },

  /**
   * Compare two pack versions
   * NOTE: Endpoint pending backend implementation
   */
  async diff(fromConfigId: string, toConfigId: string): Promise<PackDiffResponse> {
    return apiRequest(
      `/api/v1/pack/diff?from=${fromConfigId}&to=${toConfigId}`
    );
  },

  /**
   * Revert to a previous version (creates new version)
   * NOTE: Endpoint pending backend implementation
   */
  async revert(request: PackRevertRequest): Promise<DeployDslResponse> {
    return apiRequest('/api/v1/pack/revert', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },

  /**
   * Inspect the currently active pack configuration
   * (Uses existing endpoint)
   */
  async inspect(): Promise<PackInspectResponse> {
    return apiRequest('/api/v1/pack/inspect');
  },

  /**
   * Compose a pack (validate without saving)
   * (Uses existing endpoint)
   */
  async compose(
    manifest: string,
    files: PackSourceFile[]
  ): Promise<ComposePackResponse> {
    const formData = new FormData();
    formData.append('cal_toml', manifest);
    files.forEach((f) => {
      formData.append('markdown', new Blob([f.content]), f.path);
    });

    return apiMultipart('/api/v1/dsl/compose', formData);
  },

  /**
   * Deploy a pack (save and optionally activate)
   * (Uses existing endpoint)
   */
  async deploy(request: DeployDslRequest): Promise<DeployDslResponse> {
    return apiRequest('/api/v1/dsl/deploy', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },
};

export default packApi;
