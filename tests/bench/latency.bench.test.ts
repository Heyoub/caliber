/**
 * Latency Benchmark Tests
 *
 * Measures p50, p95, p99 latency for key API endpoints.
 * Run with: bun test tests/bench/latency.bench.ts
 *
 * Environment variables:
 *   CALIBER_API_URL - API base URL (default: http://localhost:3000)
 *   CALIBER_TEST_TOKEN - Auth token for authenticated endpoints
 *   BENCH_ITERATIONS - Number of iterations per endpoint (default: 100)
 */

import { describe, expect, it, beforeAll, beforeEach } from 'bun:test';
import { clearRateLimits } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const TEST_TOKEN = process.env.CALIBER_TEST_TOKEN ?? '';
const ITERATIONS = parseInt(process.env.BENCH_ITERATIONS ?? '100', 10);
const SKIP_BENCH = process.env.SKIP_BENCH_TESTS === 'true';

// Latency thresholds (ms) - these are our SLOs
const LATENCY_THRESHOLDS = {
  health: { p50: 10, p95: 25, p99: 50 },
  ready: { p50: 20, p95: 50, p99: 100 },
  trajectories_list: { p50: 50, p95: 100, p99: 200 },
  trajectory_get: { p50: 30, p95: 75, p99: 150 },
  auth_check: { p50: 20, p95: 50, p99: 100 },
};

// Statistics helpers
function percentile(arr: number[], p: number): number {
  const sorted = [...arr].sort((a, b) => a - b);
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, idx)];
}

function mean(arr: number[]): number {
  return arr.reduce((a, b) => a + b, 0) / arr.length;
}

function stdDev(arr: number[]): number {
  const avg = mean(arr);
  const squareDiffs = arr.map((v) => (v - avg) ** 2);
  return Math.sqrt(mean(squareDiffs));
}

// Helper to measure request latency
async function measureLatency(
  method: string,
  path: string,
  headers: Record<string, string> = {}
): Promise<number> {
  const url = `${API_BASE_URL}${path}`;
  const start = performance.now();

  await fetch(url, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...headers,
    },
  });

  return performance.now() - start;
}

// Run benchmark and collect stats
async function runBenchmark(
  name: string,
  method: string,
  path: string,
  headers: Record<string, string> = {},
  iterations: number = ITERATIONS
): Promise<{
  name: string;
  iterations: number;
  min: number;
  max: number;
  mean: number;
  stdDev: number;
  p50: number;
  p95: number;
  p99: number;
}> {
  const latencies: number[] = [];

  // Warm-up (10% of iterations)
  const warmupCount = Math.max(1, Math.floor(iterations * 0.1));
  for (let i = 0; i < warmupCount; i++) {
    await measureLatency(method, path, headers);
  }

  // Actual benchmark
  for (let i = 0; i < iterations; i++) {
    const latency = await measureLatency(method, path, headers);
    latencies.push(latency);
  }

  return {
    name,
    iterations,
    min: Math.min(...latencies),
    max: Math.max(...latencies),
    mean: mean(latencies),
    stdDev: stdDev(latencies),
    p50: percentile(latencies, 50),
    p95: percentile(latencies, 95),
    p99: percentile(latencies, 99),
  };
}

// Format results as table
function formatResults(
  results: Array<{
    name: string;
    iterations: number;
    min: number;
    max: number;
    mean: number;
    stdDev: number;
    p50: number;
    p95: number;
    p99: number;
  }>
): string {
  const header = '| Endpoint | Iterations | Min | Mean | p50 | p95 | p99 | Max | StdDev |';
  const separator = '|----------|------------|-----|------|-----|-----|-----|-----|--------|';

  const rows = results.map((r) =>
    `| ${r.name.padEnd(20)} | ${r.iterations.toString().padStart(10)} | ${r.min.toFixed(1).padStart(5)}ms | ${r.mean.toFixed(1).padStart(6)}ms | ${r.p50.toFixed(1).padStart(5)}ms | ${r.p95.toFixed(1).padStart(5)}ms | ${r.p99.toFixed(1).padStart(5)}ms | ${r.max.toFixed(1).padStart(5)}ms | ${r.stdDev.toFixed(1).padStart(6)}ms |`
  );

  return [header, separator, ...rows].join('\n');
}

describe.skipIf(SKIP_BENCH)('bench: Latency', () => {
  beforeAll(async () => {
    // Clear rate limits from previous test files
    clearRateLimits();

    // Verify API is reachable
    const res = await fetch(`${API_BASE_URL}/health`);
    if (!res.ok) {
      throw new Error(`API not reachable: ${res.status}`);
    }
    console.log(`\nLatency benchmark against ${API_BASE_URL}`);
    console.log(`Iterations per endpoint: ${ITERATIONS}\n`);
  });

  // Clear rate limits before each benchmark
  beforeEach(() => {
    clearRateLimits();
  });

  describe('Health Endpoints', () => {
    it('health endpoint latency', async () => {
      const result = await runBenchmark('GET /health', 'GET', '/health');

      console.log(`\n/health latency:`);
      console.log(`  p50: ${result.p50.toFixed(2)}ms`);
      console.log(`  p95: ${result.p95.toFixed(2)}ms`);
      console.log(`  p99: ${result.p99.toFixed(2)}ms`);

      // Verify against thresholds
      expect(result.p50).toBeLessThan(LATENCY_THRESHOLDS.health.p50);
      expect(result.p95).toBeLessThan(LATENCY_THRESHOLDS.health.p95);
      expect(result.p99).toBeLessThan(LATENCY_THRESHOLDS.health.p99);
    });

    it('ready endpoint latency', async () => {
      const result = await runBenchmark('GET /health/ready', 'GET', '/health/ready');

      console.log(`\n/health/ready latency:`);
      console.log(`  p50: ${result.p50.toFixed(2)}ms`);
      console.log(`  p95: ${result.p95.toFixed(2)}ms`);
      console.log(`  p99: ${result.p99.toFixed(2)}ms`);

      expect(result.p50).toBeLessThan(LATENCY_THRESHOLDS.ready.p50);
      expect(result.p95).toBeLessThan(LATENCY_THRESHOLDS.ready.p95);
    });
  });

  describe('Auth Boundary', () => {
    it('unauthorized request latency', async () => {
      const result = await runBenchmark(
        'GET /api/v1/trajectories (401)',
        'GET',
        '/api/v1/trajectories'
      );

      console.log(`\nUnauthorized request latency:`);
      console.log(`  p50: ${result.p50.toFixed(2)}ms`);
      console.log(`  p95: ${result.p95.toFixed(2)}ms`);

      // Auth rejection should be fast
      expect(result.p50).toBeLessThan(LATENCY_THRESHOLDS.auth_check.p50);
      expect(result.p95).toBeLessThan(LATENCY_THRESHOLDS.auth_check.p95);
    });
  });

  describe.skipIf(!TEST_TOKEN)('Authenticated Endpoints', () => {
    const authHeaders = { Authorization: `Bearer ${TEST_TOKEN}` };

    it('trajectories list latency', async () => {
      const result = await runBenchmark(
        'GET /api/v1/trajectories',
        'GET',
        '/api/v1/trajectories',
        authHeaders
      );

      console.log(`\n/api/v1/trajectories latency:`);
      console.log(`  p50: ${result.p50.toFixed(2)}ms`);
      console.log(`  p95: ${result.p95.toFixed(2)}ms`);
      console.log(`  p99: ${result.p99.toFixed(2)}ms`);

      expect(result.p95).toBeLessThan(LATENCY_THRESHOLDS.trajectories_list.p95);
    });

    it('paginated list latency', async () => {
      const result = await runBenchmark(
        'GET /api/v1/trajectories?limit=10',
        'GET',
        '/api/v1/trajectories?limit=10',
        authHeaders
      );

      console.log(`\nPaginated list latency:`);
      console.log(`  p50: ${result.p50.toFixed(2)}ms`);
      console.log(`  p95: ${result.p95.toFixed(2)}ms`);

      // Paginated should be similar or faster
      expect(result.p95).toBeLessThan(LATENCY_THRESHOLDS.trajectories_list.p95);
    });
  });

  describe('Latency Distribution', () => {
    it('checks for latency outliers', async () => {
      const result = await runBenchmark(
        'Outlier check',
        'GET',
        '/health',
        {},
        50
      );

      // p99/p50 ratio indicates tail latency
      const tailRatio = result.p99 / result.p50;

      console.log(`\nTail latency analysis:`);
      console.log(`  p50: ${result.p50.toFixed(2)}ms`);
      console.log(`  p99: ${result.p99.toFixed(2)}ms`);
      console.log(`  p99/p50 ratio: ${tailRatio.toFixed(2)}x`);
      console.log(`  StdDev: ${result.stdDev.toFixed(2)}ms`);

      // Tail should not be more than 5x the median
      expect(tailRatio).toBeLessThan(5);
    });

    it('checks latency consistency', async () => {
      const result = await runBenchmark(
        'Consistency check',
        'GET',
        '/health',
        {},
        100
      );

      // Coefficient of variation (stdDev/mean) should be low
      const cv = result.stdDev / result.mean;

      console.log(`\nLatency consistency:`);
      console.log(`  Mean: ${result.mean.toFixed(2)}ms`);
      console.log(`  StdDev: ${result.stdDev.toFixed(2)}ms`);
      console.log(`  CV: ${(cv * 100).toFixed(1)}%`);

      // CV should be under 50% for consistent latency
      expect(cv).toBeLessThan(0.5);
    });
  });
});

describe.skipIf(SKIP_BENCH)('bench: Latency Comparison', () => {
  beforeAll(() => {
    clearRateLimits();
  });

  it('compares all endpoint latencies', async () => {
    clearRateLimits();
    const results = [];

    // Health endpoints
    results.push(await runBenchmark('health', 'GET', '/health', {}, 50));
    results.push(await runBenchmark('health/ready', 'GET', '/health/ready', {}, 50));

    // Auth boundary
    results.push(
      await runBenchmark('trajectories (unauth)', 'GET', '/api/v1/trajectories', {}, 50)
    );

    // Print comparison table
    console.log('\n' + '='.repeat(100));
    console.log('LATENCY BENCHMARK RESULTS');
    console.log('='.repeat(100));
    console.log(formatResults(results));
    console.log('='.repeat(100));

    // All endpoints should have reasonable latency
    for (const r of results) {
      expect(r.p95).toBeLessThan(500);
    }
  });
});
