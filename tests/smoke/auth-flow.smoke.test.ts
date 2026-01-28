/**
 * Smoke Test: Auth Flow
 */

import { describe, expect, it, beforeAll } from 'bun:test';
import { isUsingMocks, setupMocking } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';

beforeAll(async () => {
  await setupMocking(API_BASE_URL);
});

describe('smoke: auth flow', () => {
  it('rejects missing credentials', async () => {
    const res = await fetch(`${API_BASE_URL}/api/v1/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });

    expect([400, 401]).toContain(res.status);
  });

  it('accepts valid credentials in mock mode', async () => {
    if (!isUsingMocks()) return;

    const res = await fetch(`${API_BASE_URL}/api/v1/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        email: 'test@caliber.run',
        password: 'testpassword',
      }),
    });

    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.token).toBeDefined();
  });
});
