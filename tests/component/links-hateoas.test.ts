/**
 * Component Tests: HATEOAS link generation
 */

import { describe, expect, it, beforeAll, afterAll } from 'bun:test';
import { getTestToken, isUsingMocks, setupMocking } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const USING_MOCKS = isUsingMocks();
const TEST_TOKEN = USING_MOCKS
  ? process.env.CALIBER_TEST_TOKEN ?? getTestToken()
  : process.env.CALIBER_TEST_TOKEN ?? '';
const TEST_TENANT_ID = process.env.CALIBER_TEST_TENANT_ID;
const SKIP_COMPONENT = process.env.SKIP_COMPONENT_TESTS === 'true';

async function api<T = unknown>(
  method: string,
  path: string,
  body?: unknown
): Promise<{ status: number; data: T }> {
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

beforeAll(async () => {
  await setupMocking(API_BASE_URL);

  if (USING_MOCKS) return;

  const createRes = await api<Record<string, unknown>>('POST', '/api/v1/trajectories', {
    name: `links-trajectory-${Date.now()}`,
  });
  expect(createRes.status).toBe(201);
  trajectoryId = (createRes.data.id ?? createRes.data.trajectory_id) as string;
});

afterAll(async () => {
  if (USING_MOCKS) return;
  if (trajectoryId) {
    await api('DELETE', `/api/v1/trajectories/${trajectoryId}`);
  }
});

describe.skipIf(SKIP_COMPONENT || USING_MOCKS || !TEST_TOKEN)(
  'component: HATEOAS links',
  () => {
    it('trajectory responses include _links', async () => {
      const { status, data } = await api<Record<string, unknown>>(
        'GET',
        `/api/v1/trajectories/${trajectoryId}`
      );
      expect(status).toBe(200);

      const links = (data._links ?? data.links) as Record<string, unknown> | undefined;
      expect(links).toBeDefined();
      if (links) {
        expect(links.self).toBeDefined();
      }
    });
  }
);
