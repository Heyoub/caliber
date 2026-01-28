/**
 * Throughput Benchmark Tests
 *
 * Measures maximum RPS (requests per second) before degradation.
 * Run with: bun test tests/bench/throughput.bench.ts
 *
 * Environment variables:
 *   CALIBER_API_URL - API base URL (default: http://localhost:3000)
 *   BENCH_DURATION_SECS - Duration of each throughput test (default: 10)
 *   BENCH_CONCURRENCY - Number of concurrent connections (default: 10)
 */

import { describe, expect, it, beforeAll, beforeEach } from 'bun:test';
import { clearRateLimits } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const DURATION_SECS = parseInt(process.env.BENCH_DURATION_SECS ?? '10', 10);
const CONCURRENCY = parseInt(process.env.BENCH_CONCURRENCY ?? '10', 10);
const SKIP_BENCH = process.env.SKIP_BENCH_TESTS === 'true';

// Throughput thresholds (RPS)
const THROUGHPUT_THRESHOLDS = {
  health: 1000,
  ready: 500,
  trajectories_unauth: 500,
};

interface ThroughputResult {
  name: string;
  durationSecs: number;
  concurrency: number;
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  rps: number;
  avgLatencyMs: number;
  p50LatencyMs: number;
  p95LatencyMs: number;
  p99LatencyMs: number;
  errorRate: number;
}

// Statistics helper
function percentile(arr: number[], p: number): number {
  if (arr.length === 0) return 0;
  const sorted = [...arr].sort((a, b) => a - b);
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, idx)];
}

// Run throughput benchmark
async function runThroughputBenchmark(
  name: string,
  method: string,
  path: string,
  headers: Record<string, string> = {},
  durationSecs: number = DURATION_SECS,
  concurrency: number = CONCURRENCY
): Promise<ThroughputResult> {
  const url = `${API_BASE_URL}${path}`;
  const durationMs = durationSecs * 1000;

  let totalRequests = 0;
  let successfulRequests = 0;
  let failedRequests = 0;
  const latencies: number[] = [];

  const startTime = Date.now();

  // Worker function that makes requests as fast as possible
  async function worker(): Promise<void> {
    while (Date.now() - startTime < durationMs) {
      const reqStart = performance.now();

      try {
        const res = await fetch(url, {
          method,
          headers: {
            'Content-Type': 'application/json',
            ...headers,
          },
        });

        const latency = performance.now() - reqStart;
        latencies.push(latency);
        totalRequests++;

        if (res.ok || res.status === 401 || res.status === 429) {
          // 401 is expected for unauthenticated requests
          // 429 is expected when rate limiting kicks in
          successfulRequests++;
        } else {
          failedRequests++;
        }
      } catch {
        const latency = performance.now() - reqStart;
        latencies.push(latency);
        totalRequests++;
        failedRequests++;
      }
    }
  }

  // Start concurrent workers
  const workers = Array.from({ length: concurrency }, () => worker());
  await Promise.all(workers);

  const actualDuration = (Date.now() - startTime) / 1000;

  return {
    name,
    durationSecs: actualDuration,
    concurrency,
    totalRequests,
    successfulRequests,
    failedRequests,
    rps: totalRequests / actualDuration,
    avgLatencyMs: latencies.reduce((a, b) => a + b, 0) / latencies.length || 0,
    p50LatencyMs: percentile(latencies, 50),
    p95LatencyMs: percentile(latencies, 95),
    p99LatencyMs: percentile(latencies, 99),
    errorRate: failedRequests / totalRequests || 0,
  };
}

// Find maximum RPS before degradation
async function findMaxRps(
  name: string,
  method: string,
  path: string,
  headers: Record<string, string> = {},
  targetLatencyP95Ms: number = 100
): Promise<{
  maxRps: number;
  optimalConcurrency: number;
  degradationPoint: number | null;
}> {
  const results: Array<{ concurrency: number; rps: number; p95: number }> = [];

  // Test with increasing concurrency
  for (const concurrency of [1, 2, 5, 10, 20, 50]) {
    const result = await runThroughputBenchmark(
      name,
      method,
      path,
      headers,
      5, // Shorter duration for quick scan
      concurrency
    );

    results.push({
      concurrency,
      rps: result.rps,
      p95: result.p95LatencyMs,
    });

    // If latency exceeds target, we've found degradation point
    if (result.p95LatencyMs > targetLatencyP95Ms) {
      break;
    }
  }

  // Find optimal point (highest RPS before degradation)
  let maxRps = 0;
  let optimalConcurrency = 1;
  let degradationPoint: number | null = null;

  for (let i = 0; i < results.length; i++) {
    const r = results[i];
    if (r.rps > maxRps && r.p95 <= targetLatencyP95Ms) {
      maxRps = r.rps;
      optimalConcurrency = r.concurrency;
    }
    if (r.p95 > targetLatencyP95Ms && degradationPoint === null) {
      degradationPoint = r.concurrency;
    }
  }

  return { maxRps, optimalConcurrency, degradationPoint };
}

// Format results
function formatThroughputResult(result: ThroughputResult): string {
  return `
Throughput Benchmark: ${result.name}
${'='.repeat(50)}
Duration:      ${result.durationSecs.toFixed(1)}s
Concurrency:   ${result.concurrency}
Total Reqs:    ${result.totalRequests.toLocaleString()}
Successful:    ${result.successfulRequests.toLocaleString()}
Failed:        ${result.failedRequests.toLocaleString()}

Performance:
  RPS:         ${result.rps.toFixed(1)} req/s
  Avg Latency: ${result.avgLatencyMs.toFixed(2)}ms
  p50 Latency: ${result.p50LatencyMs.toFixed(2)}ms
  p95 Latency: ${result.p95LatencyMs.toFixed(2)}ms
  p99 Latency: ${result.p99LatencyMs.toFixed(2)}ms
  Error Rate:  ${(result.errorRate * 100).toFixed(2)}%
`;
}

describe.skipIf(SKIP_BENCH)('bench: Throughput', () => {
  beforeAll(async () => {
    // Clear rate limits from previous test files
    clearRateLimits();

    // Verify API is reachable
    const res = await fetch(`${API_BASE_URL}/health`);
    if (!res.ok) {
      throw new Error(`API not reachable: ${res.status}`);
    }
    console.log(`\nThroughput benchmark against ${API_BASE_URL}`);
    console.log(`Duration: ${DURATION_SECS}s, Concurrency: ${CONCURRENCY}\n`);
  });

  // Clear rate limits before each benchmark
  beforeEach(() => {
    clearRateLimits();
  });

  describe('Health Endpoints', () => {
    it('measures health endpoint throughput', { timeout: 15000 }, async () => {
      const result = await runThroughputBenchmark(
        'GET /health',
        'GET',
        '/health'
      );

      console.log(formatThroughputResult(result));

      // Should handle at least threshold RPS
      expect(result.rps).toBeGreaterThan(THROUGHPUT_THRESHOLDS.health);
      // Error rate should be negligible
      expect(result.errorRate).toBeLessThan(0.01);
    });

    it('measures ready endpoint throughput', { timeout: 15000 }, async () => {
      const result = await runThroughputBenchmark(
        'GET /health/ready',
        'GET',
        '/health/ready'
      );

      console.log(formatThroughputResult(result));

      expect(result.rps).toBeGreaterThan(THROUGHPUT_THRESHOLDS.ready);
      expect(result.errorRate).toBeLessThan(0.01);
    });
  });

  describe('API Endpoints', () => {
    it('measures unauthenticated API throughput', { timeout: 15000 }, async () => {
      const result = await runThroughputBenchmark(
        'GET /api/v1/trajectories (unauth)',
        'GET',
        '/api/v1/trajectories'
      );

      console.log(formatThroughputResult(result));

      // Even 401 responses should be fast
      expect(result.rps).toBeGreaterThan(THROUGHPUT_THRESHOLDS.trajectories_unauth);
    });
  });

  describe('Concurrency Scaling', () => {
    it('measures throughput at different concurrency levels', { timeout: 30000 }, async () => {
      console.log('\nConcurrency scaling test:');
      console.log('-'.repeat(50));

      const concurrencyLevels = [1, 5, 10, 20];
      const results: Array<{ concurrency: number; rps: number; p95: number }> = [];

      for (const c of concurrencyLevels) {
        const result = await runThroughputBenchmark(
          `health (c=${c})`,
          'GET',
          '/health',
          {},
          5, // 5 second test per level
          c
        );

        results.push({ concurrency: c, rps: result.rps, p95: result.p95LatencyMs });

        console.log(
          `  Concurrency ${c.toString().padStart(2)}: ` +
            `${result.rps.toFixed(0).padStart(6)} RPS, ` +
            `p95: ${result.p95LatencyMs.toFixed(1)}ms`
        );
      }

      // RPS should generally increase with concurrency (up to a point)
      const minRps = Math.min(...results.map((r) => r.rps));
      const maxRps = Math.max(...results.map((r) => r.rps));

      console.log(`\nScaling: ${minRps.toFixed(0)} -> ${maxRps.toFixed(0)} RPS`);
      console.log(`Improvement: ${((maxRps / minRps - 1) * 100).toFixed(0)}%`);

      // Should see some improvement with concurrency
      expect(maxRps).toBeGreaterThan(minRps);
    });
  });

  describe('Sustained Load', () => {
    it('maintains performance under sustained load', { timeout: 45000 }, async () => {
      // Run longer test to check for degradation
      const result = await runThroughputBenchmark(
        'Sustained load',
        'GET',
        '/health',
        {},
        30, // 30 seconds
        10
      );

      console.log('\nSustained load test (30s):');
      console.log(`  RPS: ${result.rps.toFixed(1)}`);
      console.log(`  p95 Latency: ${result.p95LatencyMs.toFixed(2)}ms`);
      console.log(`  Error Rate: ${(result.errorRate * 100).toFixed(2)}%`);

      // Should maintain performance over time
      expect(result.errorRate).toBeLessThan(0.01);
      expect(result.p95LatencyMs).toBeLessThan(100);
    });
  });
});

describe.skipIf(SKIP_BENCH)('bench: Max RPS Discovery', () => {
  beforeAll(() => {
    clearRateLimits();
  });

  it('finds maximum sustainable RPS', { timeout: 45000 }, async () => {
    console.log('\nFinding maximum sustainable RPS...');

    const { maxRps, optimalConcurrency, degradationPoint } = await findMaxRps(
      'health',
      'GET',
      '/health',
      {},
      100 // Target p95 < 100ms
    );

    console.log(`\nMax RPS Analysis:`);
    console.log(`  Max sustainable RPS: ${maxRps.toFixed(0)}`);
    console.log(`  Optimal concurrency: ${optimalConcurrency}`);
    if (degradationPoint) {
      console.log(`  Degradation starts at: ${degradationPoint} concurrent`);
    }

    expect(maxRps).toBeGreaterThan(100);
  });
});

describe.skipIf(SKIP_BENCH)('bench: Throughput Summary', () => {
  // This test runs 3 endpoints for 5s each = 15s minimum
  it('generates throughput summary report', { timeout: 30000 }, async () => {
    console.log('\n' + '='.repeat(70));
    console.log('THROUGHPUT BENCHMARK SUMMARY');
    console.log('='.repeat(70));

    const endpoints = [
      { name: 'health', path: '/health' },
      { name: 'ready', path: '/health/ready' },
      { name: 'api (unauth)', path: '/api/v1/trajectories' },
    ];

    const results: ThroughputResult[] = [];

    for (const ep of endpoints) {
      const result = await runThroughputBenchmark(
        `GET ${ep.path}`,
        'GET',
        ep.path,
        {},
        5,
        10
      );
      results.push(result);
    }

    // Summary table
    console.log('\n| Endpoint                    |     RPS |  p95 (ms) | Errors |');
    console.log('|-----------------------------|---------|-----------|--------|');

    for (const r of results) {
      console.log(
        `| ${r.name.padEnd(27)} | ${r.rps.toFixed(0).padStart(7)} | ${r.p95LatencyMs.toFixed(1).padStart(9)} | ${(r.errorRate * 100).toFixed(1).padStart(5)}% |`
      );
    }

    console.log('='.repeat(70));

    // All should pass minimum thresholds
    for (const r of results) {
      expect(r.rps).toBeGreaterThan(100);
      expect(r.errorRate).toBeLessThan(0.05);
    }
  });
});
