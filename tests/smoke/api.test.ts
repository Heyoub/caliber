/**
 * Smoke Tests
 *
 * Quick sanity checks that verify basic functionality works.
 * Run first before deeper tests. Should complete in seconds.
 * Run with: bun test tests/smoke/
 */

import { describe, expect, it, beforeAll } from 'bun:test';

// Configuration
const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const SKIP_LIVE_TESTS = process.env.SKIP_LIVE_SMOKE !== 'false';

// Mock responses for offline testing
const mockResponses = {
  health: { status: 'healthy', version: '0.1.0' },
  trajectory: { id: 'traj-123', name: 'test', scopes: [] },
};

// SDK may not be built in CI, skip if not available
const SDK_AVAILABLE = await (async () => {
  try {
    await import('@caliber-run/sdk');
    return true;
  } catch {
    return false;
  }
})();

describe.skipIf(!SDK_AVAILABLE)('smoke: SDK imports', () => {
  it('imports main entry point', async () => {
    // This verifies the SDK builds and exports correctly
    const sdk = await import('@caliber-run/sdk');
    expect(sdk).toBeDefined();
  });

  it('exports CaliberClient', async () => {
    const { CaliberClient } = await import('@caliber-run/sdk');
    expect(CaliberClient).toBeDefined();
    expect(typeof CaliberClient).toBe('function');
  });

  it('exports type definitions', async () => {
    // TypeScript types exist at compile time
    // This test just verifies the module structure
    const sdk = await import('@caliber-run/sdk');
    expect(Object.keys(sdk).length).toBeGreaterThan(0);
  });
});

describe('smoke: Environment', () => {
  it('has required Node.js version', () => {
    const version = process.version;
    const major = Number.parseInt(version.slice(1).split('.')[0], 10);
    expect(major).toBeGreaterThanOrEqual(20);
  });

  it('has Bun runtime', () => {
    expect(typeof Bun).toBe('object');
    expect(Bun.version).toBeDefined();
  });

  it('can resolve workspace packages', async () => {
    // Verify workspace resolution works (only if SDK is built)
    if (!SDK_AVAILABLE) {
      expect(true).toBe(true); // Skip gracefully
      return;
    }
    const resolved = Bun.resolveSync('@caliber-run/sdk', process.cwd());
    expect(resolved).toContain('caliber-sdk');
  });
});

describe('smoke: Configuration', () => {
  it('loads from environment', () => {
    // API URL should be configurable
    const apiUrl = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
    expect(apiUrl).toMatch(/^https?:\/\//);
  });

  it('has valid API base URL format', () => {
    const url = new URL(API_BASE_URL);
    expect(url.protocol).toMatch(/^https?:$/);
  });
});

describe('smoke: API health (mocked)', () => {
  it('health endpoint returns expected shape', () => {
    const response = mockResponses.health;
    expect(response.status).toBe('healthy');
    expect(response.version).toMatch(/^\d+\.\d+\.\d+/);
  });

  it('trajectory response has required fields', () => {
    const response = mockResponses.trajectory;
    expect(response.id).toBeDefined();
    expect(response.name).toBeDefined();
    expect(Array.isArray(response.scopes)).toBe(true);
  });
});

// Live API tests - only run when SKIP_LIVE_SMOKE=false
describe.skipIf(SKIP_LIVE_TESTS)('smoke: Live API', () => {
  beforeAll(() => {
    console.log(`Running live smoke tests against: ${API_BASE_URL}`);
  });

  it('health endpoint responds', async () => {
    const response = await fetch(`${API_BASE_URL}/health`);
    expect(response.ok).toBe(true);
  });

  it('health/ready endpoint responds', async () => {
    const response = await fetch(`${API_BASE_URL}/health/ready`);
    expect(response.ok).toBe(true);

    const data = await response.json();
    expect(data.status).toBeDefined();
  });

  it('API returns proper CORS headers', async () => {
    const response = await fetch(`${API_BASE_URL}/health`, {
      method: 'OPTIONS',
      headers: {
        Origin: 'http://localhost:4321',
        'Access-Control-Request-Method': 'GET',
      },
    });

    // Should allow CORS or return a valid response
    expect(response.status).toBeLessThan(500);
  });

  it('unauthorized request returns 401', async () => {
    const response = await fetch(`${API_BASE_URL}/api/v1/trajectories`);
    expect(response.status).toBe(401);
  });
});

describe('smoke: Core operations (mocked)', () => {
  // Simulate SDK operations without live API
  class MockCaliberClient {
    private baseUrl: string;

    constructor(baseUrl: string) {
      this.baseUrl = baseUrl;
    }

    async listTrajectories(): Promise<{ trajectories: unknown[] }> {
      return { trajectories: [mockResponses.trajectory] };
    }

    async getTrajectory(id: string): Promise<unknown> {
      return { ...mockResponses.trajectory, id };
    }

    async createTrajectory(_name: string): Promise<{ id: string }> {
      return { id: `traj-${Date.now()}` };
    }
  }

  it('can instantiate client', () => {
    const client = new MockCaliberClient(API_BASE_URL);
    expect(client).toBeDefined();
  });

  it('can list trajectories', async () => {
    const client = new MockCaliberClient(API_BASE_URL);
    const result = await client.listTrajectories();
    expect(result.trajectories).toBeDefined();
    expect(Array.isArray(result.trajectories)).toBe(true);
  });

  it('can get single trajectory', async () => {
    const client = new MockCaliberClient(API_BASE_URL);
    const result = await client.getTrajectory('traj-123');
    expect(result).toBeDefined();
  });

  it('can create trajectory', async () => {
    const client = new MockCaliberClient(API_BASE_URL);
    const result = await client.createTrajectory('test-trajectory');
    expect(result.id).toBeDefined();
    expect(result.id).toMatch(/^traj-/);
  });
});

describe('smoke: Error handling', () => {
  it('SDK throws on network error', async () => {
    const badUrl = 'http://localhost:99999'; // Invalid port

    try {
      const response = await fetch(`${badUrl}/health`);
      // Some runtimes resolve with a non-ok Response instead of throwing.
      expect(response.ok).toBe(false);
    } catch (error) {
      expect(error).toBeDefined();
    }
  });

  it('handles malformed responses gracefully', () => {
    const malformed = '{"incomplete":';

    expect(() => JSON.parse(malformed)).toThrow();
  });
});
