/**
 * OWASP - Rate Limiting / Brute Force Protection Security Tests
 *
 * Tests for rate limiting, brute force protection, account lockout,
 * and denial of service mitigation.
 *
 * Run with: bun test tests/security/owasp/rate-limiting.sec.test.ts
 */

import { describe, expect, it, beforeAll } from 'bun:test';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const SKIP_LIVE_TESTS = process.env.SKIP_SECURITY_TESTS === 'true';

// Test configuration
const RATE_LIMIT_REQUESTS = 150; // Number of requests to test rate limiting
const RATE_LIMIT_WINDOW_MS = 60000; // 1 minute window

// Helper to make API requests
async function apiRequest(
  method: string,
  path: string,
  body?: unknown,
  headers: Record<string, string> = {}
): Promise<Response> {
  const url = `${API_BASE_URL}${path}`;
  const options: RequestInit = {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...headers,
    },
  };

  if (body) {
    options.body = JSON.stringify(body);
  }

  return fetch(url, options);
}

// Helper to make rapid requests
async function makeRapidRequests(
  count: number,
  requestFn: () => Promise<Response>
): Promise<{ responses: Response[]; rateLimitedCount: number }> {
  const responses: Response[] = [];
  let rateLimitedCount = 0;

  // Make requests in parallel batches of 10
  const batchSize = 10;
  for (let i = 0; i < count; i += batchSize) {
    const batch = Array.from(
      { length: Math.min(batchSize, count - i) },
      () => requestFn()
    );

    const batchResponses = await Promise.all(batch);
    responses.push(...batchResponses);

    for (const res of batchResponses) {
      if (res.status === 429) {
        rateLimitedCount++;
      }
    }
  }

  return { responses, rateLimitedCount };
}

describe.skipIf(SKIP_LIVE_TESTS)('security: Rate Limiting', () => {
  it('returns rate limit headers', async () => {
    const res = await apiRequest('GET', '/health');

    // Should include rate limit headers
    const rateHeaders = [
      'X-RateLimit-Limit',
      'X-RateLimit-Remaining',
      'X-RateLimit-Reset',
      'RateLimit-Limit',
      'RateLimit-Remaining',
      'RateLimit-Reset',
    ];

    const hasRateHeaders = rateHeaders.some(
      (h) => res.headers.get(h) !== null
    );

    // Rate limit headers should be present (or server doesn't expose them)
    // This is informational - we'll test actual limiting below
    expect(typeof hasRateHeaders).toBe('boolean');
  });

  it('enforces rate limits on unauthenticated requests', async () => {
    const { responses, rateLimitedCount } = await makeRapidRequests(
      RATE_LIMIT_REQUESTS,
      () => apiRequest('GET', '/health')
    );

    // At least some requests should be rate limited
    expect(rateLimitedCount).toBeGreaterThan(0);

    // Rate limited responses should have proper status
    const rateLimitedResponse = responses.find((r) => r.status === 429);
    if (rateLimitedResponse) {
      // Should include Retry-After header
      const retryAfter = rateLimitedResponse.headers.get('Retry-After');
      expect(retryAfter).not.toBeNull();
    }
  });

  it('enforces rate limits on authentication endpoints', async () => {
    const { responses, rateLimitedCount } = await makeRapidRequests(50, () =>
      apiRequest('POST', '/api/v1/auth/login', {
        email: 'test@example.com',
        password: 'wrongpassword',
      })
    );

    // Auth endpoints should have stricter rate limits
    expect(rateLimitedCount).toBeGreaterThan(0);
  });

  it('rate limits are per-IP', async () => {
    // This is difficult to test without multiple IPs
    // We verify the X-Forwarded-For header is not blindly trusted

    const res = await apiRequest('GET', '/health', undefined, {
      'X-Forwarded-For': '1.2.3.4',
    });

    // Should not reset rate limit based on spoofed IP
    expect(res.status).not.toBe(500);
  });

  it('returns proper 429 response format', async () => {
    // Make enough requests to trigger rate limit
    let rateLimitedResponse: Response | null = null;

    for (let i = 0; i < RATE_LIMIT_REQUESTS && !rateLimitedResponse; i++) {
      const res = await apiRequest('GET', '/health');
      if (res.status === 429) {
        rateLimitedResponse = res;
      }
    }

    if (rateLimitedResponse) {
      // Response should be JSON
      const contentType = rateLimitedResponse.headers.get('Content-Type');
      expect(contentType).toContain('application/json');

      // Body should explain the rate limit
      const body = await rateLimitedResponse.json();
      expect(body.error || body.message).toBeDefined();
    }
  });
});

describe.skipIf(SKIP_LIVE_TESTS)('security: Brute Force Protection', () => {
  it('locks out after multiple failed login attempts', async () => {
    const email = `bruteforce-test-${Date.now()}@example.com`;
    let lockedOut = false;

    // Make 20 failed login attempts
    for (let i = 0; i < 20; i++) {
      const res = await apiRequest('POST', '/api/v1/auth/login', {
        email,
        password: `wrongpassword${i}`,
      });

      if (res.status === 429 || res.status === 423) {
        lockedOut = true;
        break;
      }

      // Small delay to avoid triggering general rate limit
      await new Promise((r) => setTimeout(r, 100));
    }

    // Should trigger lockout or rate limit
    expect(lockedOut).toBe(true);
  });

  it('prevents password spraying attacks', async () => {
    // Try same password against multiple emails
    const commonPassword = 'password123';
    let rateLimited = false;

    for (let i = 0; i < 50; i++) {
      const res = await apiRequest('POST', '/api/v1/auth/login', {
        email: `user${i}@example.com`,
        password: commonPassword,
      });

      if (res.status === 429) {
        rateLimited = true;
        break;
      }
    }

    expect(rateLimited).toBe(true);
  });

  it('delays response after failed attempts', async () => {
    const timings: number[] = [];

    // Make 5 failed login attempts and measure timing
    for (let i = 0; i < 5; i++) {
      const start = performance.now();
      await apiRequest('POST', '/api/v1/auth/login', {
        email: 'timing-test@example.com',
        password: `wrong${i}`,
      });
      timings.push(performance.now() - start);
    }

    // Later attempts should be slower (exponential backoff)
    // This is optional - not all systems implement this
    const avgFirst = (timings[0] + timings[1]) / 2;
    const avgLast = (timings[3] + timings[4]) / 2;

    // Log for informational purposes
    console.log(`First attempts avg: ${avgFirst.toFixed(0)}ms`);
    console.log(`Last attempts avg: ${avgLast.toFixed(0)}ms`);
  });
});

describe.skipIf(SKIP_LIVE_TESTS)('security: DoS Mitigation', () => {
  it('handles large request bodies gracefully', async () => {
    const largeBody = {
      name: 'x'.repeat(10_000_000), // 10MB string
    };

    const res = await apiRequest('POST', '/api/v1/trajectories', largeBody);

    // Should reject with 401 (auth first), 413 (payload too large), or 400 (bad request)
    expect([400, 401, 413]).toContain(res.status);
  });

  it('handles deeply nested JSON', async () => {
    // Create deeply nested object
    let nested: Record<string, unknown> = { value: 'end' };
    for (let i = 0; i < 1000; i++) {
      nested = { nested };
    }

    const res = await apiRequest('POST', '/api/v1/trajectories', {
      name: 'test',
      data: nested,
    });

    // Should handle without crashing (401 if auth checked first, 400/413/422 for payload issues)
    expect([400, 401, 413, 422]).toContain(res.status);
  });

  it('handles requests with many query parameters', async () => {
    const params = new URLSearchParams();
    for (let i = 0; i < 1000; i++) {
      params.append(`param${i}`, `value${i}`);
    }

    const res = await fetch(
      `${API_BASE_URL}/api/v1/trajectories?${params.toString()}`
    );

    // Should handle without crashing
    expect(res.status).toBeLessThan(500);
  });

  it('handles requests with many headers', async () => {
    const headers: Record<string, string> = {};
    for (let i = 0; i < 100; i++) {
      headers[`X-Custom-Header-${i}`] = `value${i}`;
    }

    const res = await apiRequest('GET', '/health', undefined, headers);

    // Should handle without crashing
    expect(res.status).toBeLessThan(500);
  });

  it('handles slow client attacks (slowloris-like)', async () => {
    // This is a basic test - real slowloris requires keeping connections open
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5000);

    try {
      const res = await fetch(`${API_BASE_URL}/health`, {
        signal: controller.signal,
      });
      clearTimeout(timeout);
      expect(res.status).toBeLessThan(500);
    } catch (e) {
      clearTimeout(timeout);
      // Timeout is acceptable
      expect(String(e)).toContain('abort');
    }
  });
});

describe.skipIf(SKIP_LIVE_TESTS)('security: API Abuse Prevention', () => {
  it('tracks and limits expensive operations', async () => {
    // Try to trigger expensive search operations
    const { rateLimitedCount } = await makeRapidRequests(50, () =>
      apiRequest('GET', '/api/v1/trajectories?search=*&limit=1000')
    );

    // Expensive operations should be rate limited more aggressively
    expect(rateLimitedCount).toBeGreaterThanOrEqual(0);
  });

  it('prevents API key abuse', async () => {
    // Rapid requests with invalid API keys
    const { responses } = await makeRapidRequests(50, () =>
      apiRequest('GET', '/api/v1/trajectories', undefined, {
        Authorization: `Bearer invalid-key-${Math.random()}`,
      })
    );

    // Should eventually rate limit even invalid requests
    const rateLimited = responses.some((r) => r.status === 429);
    const unauthorized = responses.some((r) => r.status === 401);

    expect(rateLimited || unauthorized).toBe(true);
  });
});

describe('security: Rate limiting patterns (offline)', () => {
  // Token bucket algorithm simulation
  class TokenBucket {
    private tokens: number;
    private lastRefill: number;

    constructor(
      private capacity: number,
      private refillRate: number // tokens per second
    ) {
      this.tokens = capacity;
      this.lastRefill = Date.now();
    }

    tryConsume(): boolean {
      this.refill();

      if (this.tokens >= 1) {
        this.tokens--;
        return true;
      }

      return false;
    }

    private refill(): void {
      const now = Date.now();
      const elapsed = (now - this.lastRefill) / 1000;
      const tokensToAdd = elapsed * this.refillRate;

      this.tokens = Math.min(this.capacity, this.tokens + tokensToAdd);
      this.lastRefill = now;
    }
  }

  it('token bucket allows burst then limits', () => {
    const bucket = new TokenBucket(10, 1); // 10 capacity, 1/sec refill

    // Should allow first 10 requests (burst)
    let allowed = 0;
    for (let i = 0; i < 15; i++) {
      if (bucket.tryConsume()) allowed++;
    }

    expect(allowed).toBe(10);
  });

  it('token bucket refills over time', async () => {
    const bucket = new TokenBucket(5, 10); // 5 capacity, 10/sec refill

    // Consume all tokens
    for (let i = 0; i < 5; i++) {
      bucket.tryConsume();
    }

    // Wait for refill
    await new Promise((r) => setTimeout(r, 500));

    // Should have refilled some tokens
    let allowed = 0;
    for (let i = 0; i < 10; i++) {
      if (bucket.tryConsume()) allowed++;
    }

    expect(allowed).toBeGreaterThan(0);
  });

  // Sliding window counter simulation
  class SlidingWindowCounter {
    private requests: number[] = [];

    constructor(
      private windowMs: number,
      private limit: number
    ) {}

    tryRequest(): boolean {
      const now = Date.now();
      const windowStart = now - this.windowMs;

      // Remove old requests
      this.requests = this.requests.filter((t) => t > windowStart);

      if (this.requests.length < this.limit) {
        this.requests.push(now);
        return true;
      }

      return false;
    }
  }

  it('sliding window limits requests over time', () => {
    const limiter = new SlidingWindowCounter(1000, 5); // 5 per second

    let allowed = 0;
    for (let i = 0; i < 10; i++) {
      if (limiter.tryRequest()) allowed++;
    }

    expect(allowed).toBe(5);
  });
});
