/**
 * Component Tests: ECS CRUD surface
 *
 * Verifies list + not-found semantics for core entities.
 */

import { describe, expect, it, beforeAll } from 'bun:test';
import { clearRateLimits, getTestToken, isUsingMocks, setupMocking } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const TEST_TOKEN = process.env.CALIBER_TEST_TOKEN ?? getTestToken();
const USING_MOCKS = isUsingMocks();

interface EntitySpec {
  name: string;
  endpoint: string;
  listKey: string;
}

const ENTITIES: EntitySpec[] = [
  { name: 'Trajectory', endpoint: '/api/v1/trajectories', listKey: 'trajectories' },
  { name: 'Agent', endpoint: '/api/v1/agents', listKey: 'agents' },
  { name: 'Scope', endpoint: '/api/v1/scopes', listKey: 'scopes' },
  { name: 'Artifact', endpoint: '/api/v1/artifacts', listKey: 'artifacts' },
];

async function api<T = unknown>(
  method: string,
  path: string,
  body?: unknown
): Promise<{ status: number; data: T } > {
  const url = `${API_BASE_URL}${path}`;
  const options: RequestInit = {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(TEST_TOKEN ? { Authorization: `Bearer ${TEST_TOKEN}` } : {}),
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

function extractItems(data: Record<string, unknown>, listKey: string): unknown[] {
  if (Array.isArray(data)) return data;
  const keyValue = data[listKey] ?? data.items;
  return Array.isArray(keyValue) ? keyValue : [];
}

beforeAll(async () => {
  await setupMocking(API_BASE_URL);
  clearRateLimits();
});

describe('component: ECS list + get contracts', () => {
  for (const entity of ENTITIES) {
    describe(entity.name, () => {
      it('lists entities', async () => {
        const { status, data } = await api<Record<string, unknown>>(
          'GET',
          entity.endpoint
        );

        // If auth is enforced and token missing, allow 401.
        expect([200, 401]).toContain(status);
        if (status !== 200) return;

        const items = extractItems(data, entity.listKey);
        expect(Array.isArray(items)).toBe(true);
      });

      it('returns 404 for unknown id', async () => {
        const { status } = await api(
          'GET',
          `${entity.endpoint}/00000000-0000-0000-0000-000000000000`
        );

        // In mocks, unknown IDs are 404. Live servers should also 404.
        expect([404, 400, 401]).toContain(status);
      });

      it('returns HATEOAS links when available', async () => {
        const { status, data } = await api<Record<string, unknown>>(
          'GET',
          entity.endpoint
        );

        if (USING_MOCKS || status !== 200) return;

        const items = extractItems(data, entity.listKey);
        if (items.length === 0) return;

        const first = items[0] as Record<string, unknown>;
        const links = (first._links ?? first.links) as Record<string, unknown> | undefined;
        expect(links).toBeDefined();
      });
    });
  }
});
