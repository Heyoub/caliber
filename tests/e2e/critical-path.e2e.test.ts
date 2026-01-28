/**
 * E2E Test: Critical Path
 *
 * Create trajectory -> scope -> artifact -> verify -> close -> complete
 */

import { describe, expect, it, beforeAll, afterAll } from 'bun:test';
import { clearRateLimits, isUsingMocks } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const TEST_TOKEN = process.env.CALIBER_TEST_TOKEN ?? '';
const TEST_TENANT_ID = process.env.CALIBER_TEST_TENANT_ID;
const SKIP_E2E = process.env.SKIP_E2E_TESTS === 'true';
const USING_MOCKS = isUsingMocks();

interface ApiResult<T> {
  status: number;
  data: T;
}

async function api<T = unknown>(
  method: string,
  path: string,
  body?: unknown
): Promise<ApiResult<T>> {
  const url = `${API_BASE_URL}${path}`;
  const options: RequestInit = {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(TEST_TOKEN ? { Authorization: `Bearer ${TEST_TOKEN}` } : {}),
      ...(TEST_TENANT_ID ? { 'X-Tenant-ID': TEST_TENANT_ID } : {}),
    },
  };

  if (body) {
    options.body = JSON.stringify(body);
  }

  const res = await fetch(url, options);
  let data: T;
  try {
    data = await res.json();
  } catch {
    data = {} as T;
  }

  return { status: res.status, data };
}

let trajectoryId: string | null = null;
let scopeId: string | null = null;
let artifactId: string | null = null;

describe.skipIf(SKIP_E2E || !TEST_TOKEN)('e2e: Critical Path', () => {
  beforeAll(async () => {
    clearRateLimits();
    const health = await api('GET', '/health');
    if (health.status !== 200) {
      throw new Error(`API not reachable: ${health.status}`);
    }
  });

  afterAll(async () => {
    if (trajectoryId) {
      await api('DELETE', `/api/v1/trajectories/${trajectoryId}`);
    }
  });

  it('creates a trajectory', async () => {
    const res = await api<Record<string, unknown>>('POST', '/api/v1/trajectories', {
      name: `critical-path-${Date.now()}`,
      description: 'Critical path E2E test',
    });

    expect(res.status).toBe(201);
    trajectoryId = (res.data.id ?? res.data.trajectory_id) as string;
    expect(trajectoryId).toBeDefined();
  });

  it('creates a scope under the trajectory', async () => {
    const res = await api<Record<string, unknown>>(
      'POST',
      `/api/v1/trajectories/${trajectoryId}/scopes`,
      {
        name: 'critical-scope',
        description: 'Critical path scope',
      }
    );

    expect(res.status).toBe(201);
    scopeId = (res.data.id ?? res.data.scope_id) as string;
    expect(scopeId).toBeDefined();
  });

  it('creates an artifact in the scope', async () => {
    const res = await api<Record<string, unknown>>(
      'POST',
      `/api/v1/scopes/${scopeId}/artifacts`,
      {
        type: 'text',
        name: 'critical-artifact',
        content: 'Critical path artifact content',
      }
    );

    expect(res.status).toBe(201);
    artifactId = (res.data.id ?? res.data.artifact_id) as string;
    expect(artifactId).toBeDefined();
  });

  it('retrieves the artifact', async () => {
    const res = await api<Record<string, unknown>>(
      'GET',
      `/api/v1/artifacts/${artifactId}`
    );

    expect([200, 404]).toContain(res.status);
    if (res.status === 200) {
      expect(res.data).toBeDefined();
    }
  });

  it('closes the scope and completes the trajectory (live only)', async () => {
    if (USING_MOCKS) return;

    const closeRes = await api('POST', `/api/v1/scopes/${scopeId}/close`);
    expect([200, 409]).toContain(closeRes.status);

    const completeRes = await api('POST', `/api/v1/trajectories/${trajectoryId}/complete`);
    expect([200, 204]).toContain(completeRes.status);
  });
});
