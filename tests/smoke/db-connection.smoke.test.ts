/**
 * Smoke Test: DB Connection
 *
 * Verifies DB-backed endpoints respond without 5xx.
 */

import { describe, expect, it, beforeAll } from 'bun:test';
import { getTestToken, isUsingMocks, setupMocking } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const TEST_TOKEN = process.env.CALIBER_TEST_TOKEN ?? getTestToken();

beforeAll(async () => {
  await setupMocking(API_BASE_URL);
});

describe('smoke: db connection', () => {
  it('list endpoint does not 500', async () => {
    const res = await fetch(`${API_BASE_URL}/api/v1/trajectories`, {
      headers: {
        ...(TEST_TOKEN ? { Authorization: `Bearer ${TEST_TOKEN}` } : {}),
      },
    });

    expect(res.status).not.toBe(500);

    if (res.status === 200) {
      const data = await res.json();
      const items = data.trajectories ?? data.items ?? [];
      expect(Array.isArray(items)).toBe(true);
    }
  });

  it('health ready responds', async () => {
    const res = await fetch(`${API_BASE_URL}/health/ready`);
    expect(res.status).not.toBe(500);
    if (!isUsingMocks()) {
      expect([200, 503]).toContain(res.status);
    }
  });
});
