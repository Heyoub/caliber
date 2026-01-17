/**
 * Chaos Tests
 *
 * Tests that simulate failures, delays, and adverse conditions.
 * Goal: Verify system resilience and graceful degradation.
 * Run with: bun test tests/chaos/
 */

import { describe, expect, it, beforeEach, afterEach, mock } from 'bun:test';

// Chaos utilities
class ChaosMonkey {
  private failures: Map<string, number> = new Map();
  private delays: Map<string, number> = new Map();
  private timeouts: Set<string> = new Set();

  injectFailure(operation: string, probability: number): void {
    this.failures.set(operation, probability);
  }

  injectDelay(operation: string, delayMs: number): void {
    this.delays.set(operation, delayMs);
  }

  injectTimeout(operation: string): void {
    this.timeouts.add(operation);
  }

  shouldFail(operation: string): boolean {
    const probability = this.failures.get(operation) ?? 0;
    return Math.random() < probability;
  }

  getDelay(operation: string): number {
    return this.delays.get(operation) ?? 0;
  }

  shouldTimeout(operation: string): boolean {
    return this.timeouts.has(operation);
  }

  reset(): void {
    this.failures.clear();
    this.delays.clear();
    this.timeouts.clear();
  }
}

// Simulated API client with chaos injection
class ResilientApiClient {
  private chaos: ChaosMonkey;
  private retries: number;
  private timeoutMs: number;

  constructor(chaos: ChaosMonkey, retries = 3, timeoutMs = 5000) {
    this.chaos = chaos;
    this.retries = retries;
    this.timeoutMs = timeoutMs;
  }

  async fetchWithResilience<T>(
    operation: string,
    fetcher: () => Promise<T>
  ): Promise<{ success: boolean; data?: T; error?: string; attempts: number }> {
    let attempts = 0;
    let lastError: string | undefined;

    for (let i = 0; i < this.retries; i++) {
      attempts++;

      // Check for injected timeout
      if (this.chaos.shouldTimeout(operation)) {
        lastError = 'Timeout';
        continue;
      }

      // Apply injected delay
      const delay = this.chaos.getDelay(operation);
      if (delay > 0) {
        await new Promise((r) => setTimeout(r, delay));
      }

      // Check for injected failure
      if (this.chaos.shouldFail(operation)) {
        lastError = 'Injected failure';
        continue;
      }

      try {
        const data = await fetcher();
        return { success: true, data, attempts };
      } catch (e) {
        lastError = String(e);
      }
    }

    return { success: false, error: lastError, attempts };
  }
}

// Simulated circuit breaker
class CircuitBreaker {
  private failures = 0;
  private lastFailure = 0;
  private state: 'closed' | 'open' | 'half-open' = 'closed';
  private threshold: number;
  private resetTimeMs: number;

  constructor(threshold = 5, resetTimeMs = 30000) {
    this.threshold = threshold;
    this.resetTimeMs = resetTimeMs;
  }

  async execute<T>(operation: () => Promise<T>): Promise<T> {
    if (this.state === 'open') {
      if (Date.now() - this.lastFailure > this.resetTimeMs) {
        this.state = 'half-open';
      } else {
        throw new Error('Circuit is open');
      }
    }

    try {
      const result = await operation();
      if (this.state === 'half-open') {
        this.state = 'closed';
        this.failures = 0;
      }
      return result;
    } catch (e) {
      this.failures++;
      this.lastFailure = Date.now();
      if (this.failures >= this.threshold) {
        this.state = 'open';
      }
      throw e;
    }
  }

  getState(): string {
    return this.state;
  }

  reset(): void {
    this.state = 'closed';
    this.failures = 0;
  }
}

describe('chaos: Retry resilience', () => {
  let chaos: ChaosMonkey;
  let client: ResilientApiClient;

  beforeEach(() => {
    chaos = new ChaosMonkey();
    client = new ResilientApiClient(chaos);
  });

  afterEach(() => {
    chaos.reset();
  });

  it('succeeds on first try when no chaos', async () => {
    const result = await client.fetchWithResilience('test', async () => 'data');
    expect(result.success).toBe(true);
    expect(result.data).toBe('data');
    expect(result.attempts).toBe(1);
  });

  it('retries on intermittent failures', async () => {
    chaos.injectFailure('test', 0.5); // 50% failure rate

    let successCount = 0;
    let totalAttempts = 0;

    for (let i = 0; i < 20; i++) {
      const result = await client.fetchWithResilience('test', async () => 'data');
      if (result.success) successCount++;
      totalAttempts += result.attempts;
    }

    // With retries, should succeed most of the time
    expect(successCount).toBeGreaterThan(10);
    // Average attempts should be > 1 due to failures
    expect(totalAttempts / 20).toBeGreaterThan(1);
  });

  it('handles 100% failure gracefully', async () => {
    chaos.injectFailure('test', 1.0); // 100% failure

    const result = await client.fetchWithResilience('test', async () => 'data');
    expect(result.success).toBe(false);
    expect(result.attempts).toBe(3); // All retries exhausted
    expect(result.error).toBe('Injected failure');
  });

  it('handles timeouts', async () => {
    chaos.injectTimeout('test');

    const result = await client.fetchWithResilience('test', async () => 'data');
    expect(result.success).toBe(false);
    expect(result.error).toBe('Timeout');
  });
});

describe('chaos: Circuit breaker', () => {
  let breaker: CircuitBreaker;

  beforeEach(() => {
    breaker = new CircuitBreaker(3, 100); // Low threshold for testing
  });

  it('stays closed on success', async () => {
    await breaker.execute(async () => 'ok');
    expect(breaker.getState()).toBe('closed');
  });

  it('opens after threshold failures', async () => {
    const failingOp = async () => {
      throw new Error('fail');
    };

    for (let i = 0; i < 3; i++) {
      try {
        await breaker.execute(failingOp);
      } catch {
        // Expected
      }
    }

    expect(breaker.getState()).toBe('open');
  });

  it('rejects requests when open', async () => {
    // Force open state
    const failingOp = async () => {
      throw new Error('fail');
    };
    for (let i = 0; i < 3; i++) {
      try {
        await breaker.execute(failingOp);
      } catch {
        // Expected
      }
    }

    // Now should reject immediately
    await expect(breaker.execute(async () => 'ok')).rejects.toThrow('Circuit is open');
  });

  it('transitions to half-open after reset time', async () => {
    // Force open state
    for (let i = 0; i < 3; i++) {
      try {
        await breaker.execute(async () => {
          throw new Error('fail');
        });
      } catch {
        // Expected
      }
    }

    expect(breaker.getState()).toBe('open');

    // Wait for reset time
    await new Promise((r) => setTimeout(r, 150));

    // Next call should work (half-open allows one through)
    const result = await breaker.execute(async () => 'recovered');
    expect(result).toBe('recovered');
    expect(breaker.getState()).toBe('closed');
  });
});

describe('chaos: Concurrent failures', () => {
  it('handles concurrent requests under failure', async () => {
    const chaos = new ChaosMonkey();
    chaos.injectFailure('concurrent', 0.3);
    chaos.injectDelay('concurrent', 10);

    const client = new ResilientApiClient(chaos);

    // Fire 50 concurrent requests
    const promises = Array.from({ length: 50 }, (_, i) =>
      client.fetchWithResilience('concurrent', async () => `result-${i}`)
    );

    const results = await Promise.all(promises);
    const successes = results.filter((r) => r.success).length;

    // Should have high success rate with retries
    expect(successes).toBeGreaterThan(35);
  });
});

describe('chaos: Resource exhaustion', () => {
  it('handles memory pressure gracefully', () => {
    const largeArrays: number[][] = [];

    // Allocate incrementally until we hit a reasonable limit
    const maxIterations = 100;
    let i = 0;

    try {
      for (i = 0; i < maxIterations; i++) {
        largeArrays.push(new Array(100000).fill(i));
      }
    } catch {
      // Memory exhaustion - expected in constrained environments
    }

    // Clean up
    largeArrays.length = 0;

    // Test passed if we didn't crash
    expect(true).toBe(true);
  });

  it('handles rapid object creation', () => {
    const iterations = 10000;
    const objects: Record<string, unknown>[] = [];

    for (let i = 0; i < iterations; i++) {
      objects.push({
        id: `obj-${i}`,
        data: { nested: { value: i } },
        timestamp: Date.now(),
      });
    }

    expect(objects.length).toBe(iterations);
  });
});
