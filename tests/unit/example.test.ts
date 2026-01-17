/**
 * Unit Test Example
 *
 * Unit tests verify individual functions/modules in isolation.
 * Run with: bun test tests/unit/
 */

import { describe, expect, it, beforeEach, afterEach, mock } from 'bun:test';

// Example: Testing a utility function
function parseTrajectoryId(id: string): { tenant: string; trajectory: string } | null {
  const parts = id.split(':');
  if (parts.length !== 2) return null;
  const [tenant, trajectory] = parts;
  if (!tenant || !trajectory) return null;
  return { tenant, trajectory };
}

describe('parseTrajectoryId', () => {
  it('parses valid trajectory ID', () => {
    const result = parseTrajectoryId('tenant-abc:traj-123');
    expect(result).toEqual({
      tenant: 'tenant-abc',
      trajectory: 'traj-123',
    });
  });

  it('returns null for invalid format', () => {
    expect(parseTrajectoryId('invalid')).toBeNull();
    expect(parseTrajectoryId('')).toBeNull();
    expect(parseTrajectoryId('a:b:c')).toBeNull();
  });

  it('returns null for empty parts', () => {
    expect(parseTrajectoryId(':trajectory')).toBeNull();
    expect(parseTrajectoryId('tenant:')).toBeNull();
  });
});

// Example: Testing with mocks
describe('API client mocking', () => {
  const mockFetch = mock(() =>
    Promise.resolve({
      ok: true,
      json: () => Promise.resolve({ data: 'test' }),
    })
  );

  beforeEach(() => {
    globalThis.fetch = mockFetch as unknown as typeof fetch;
  });

  afterEach(() => {
    mockFetch.mockClear();
  });

  it('calls fetch with correct URL', async () => {
    await fetch('/api/trajectories');
    expect(mockFetch).toHaveBeenCalledWith('/api/trajectories');
  });
});

// Example: Testing async functions
describe('async operations', () => {
  async function fetchWithRetry(url: string, maxRetries = 3): Promise<Response> {
    let lastError: Error | null = null;
    for (let i = 0; i < maxRetries; i++) {
      try {
        return await fetch(url);
      } catch (e) {
        lastError = e as Error;
      }
    }
    throw lastError;
  }

  it('retries on failure', async () => {
    let attempts = 0;
    const mockFetch = mock(() => {
      attempts++;
      if (attempts < 3) {
        return Promise.reject(new Error('Network error'));
      }
      return Promise.resolve({ ok: true } as Response);
    });

    globalThis.fetch = mockFetch as unknown as typeof fetch;

    const result = await fetchWithRetry('/api/test');
    expect(result.ok).toBe(true);
    expect(attempts).toBe(3);
  });
});
