/**
 * k6 Load Test - API Baseline
 *
 * Establishes baseline performance metrics (p50, p95, p99 latency).
 * Run with: k6 run tests/load/k6/api-baseline.js
 *
 * Environment variables:
 *   CALIBER_API_URL - API base URL (default: http://localhost:3000)
 *   CALIBER_API_KEY - API key for authenticated tests
 */

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';
import { API_BASE_URL, thresholds, baseOptions, getHeaders, checks } from './config.js';

// Custom metrics
const healthLatency = new Trend('health_latency', true);
const trajectoriesLatency = new Trend('trajectories_latency', true);
const errorRate = new Rate('errors');

// Test configuration
export const options = {
  ...baseOptions,

  // Baseline: Moderate constant load
  scenarios: {
    baseline: {
      executor: 'constant-vus',
      vus: 10,
      duration: '2m',
    },
  },

  thresholds: {
    ...thresholds,
    // Custom metric thresholds
    health_latency: ['p(95)<50'],
    trajectories_latency: ['p(95)<100'],
    errors: ['rate<0.001'],
  },
};

// Setup - runs once before all VUs
export function setup() {
  // Verify API is reachable
  const res = http.get(`${API_BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`API not reachable: ${res.status}`);
  }

  console.log(`Baseline test starting against ${API_BASE_URL}`);

  return {
    startTime: Date.now(),
  };
}

// Main test function - runs for each VU iteration
export default function (_data) {
  // Health check endpoint (should be fastest)
  group('Health Endpoints', () => {
    const healthRes = http.get(`${API_BASE_URL}/health`, {
      headers: getHeaders(),
    });

    healthLatency.add(healthRes.timings.duration);

    const healthChecks = check(healthRes, {
      'health status is 200': checks.is200,
      'health has body': checks.hasBody,
      'health latency < 50ms': checks.durationBelow(50),
    });

    if (!healthChecks) {
      errorRate.add(1);
    }

    // Readiness check
    const readyRes = http.get(`${API_BASE_URL}/health/ready`, {
      headers: getHeaders(),
    });

    check(readyRes, {
      'ready status is 200': checks.is200,
    });
  });

  sleep(0.5);

  // Authenticated endpoints (expect 401 without auth)
  group('Auth Boundaries', () => {
    const trajRes = http.get(`${API_BASE_URL}/api/v1/trajectories`, {
      headers: getHeaders(false), // No auth
    });

    trajectoriesLatency.add(trajRes.timings.duration);

    check(trajRes, {
      'unauthorized returns 401': checks.is401,
      'unauthorized latency < 100ms': checks.durationBelow(100),
    });
  });

  sleep(0.5);

  // API info endpoints (public)
  group('API Info', () => {
    const docsRes = http.get(`${API_BASE_URL}/api/docs`, {
      headers: getHeaders(),
    });

    check(docsRes, {
      'docs accessible': (r) => r.status < 500,
    });
  });

  sleep(1);
}

// Teardown - runs once after all VUs complete
export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`Baseline test completed in ${duration.toFixed(1)}s`);
}

// Handle summary - generate JSON report
export function handleSummary(data) {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');

  return {
    [`load-test-baseline-${timestamp}.json`]: JSON.stringify(data, null, 2),
    stdout: textSummary(data),
  };
}

// Simple text summary
function textSummary(data) {
  const metrics = data.metrics;

  return `
================================================================================
CALIBER API BASELINE LOAD TEST RESULTS
================================================================================

Requests:
  Total:     ${metrics.http_reqs?.values?.count || 0}
  Rate:      ${(metrics.http_reqs?.values?.rate || 0).toFixed(1)}/s
  Failed:    ${((metrics.http_req_failed?.values?.rate || 0) * 100).toFixed(2)}%

Latency (http_req_duration):
  p50:       ${(metrics.http_req_duration?.values?.['p(50)'] || 0).toFixed(1)}ms
  p95:       ${(metrics.http_req_duration?.values?.['p(95)'] || 0).toFixed(1)}ms
  p99:       ${(metrics.http_req_duration?.values?.['p(99)'] || 0).toFixed(1)}ms
  max:       ${(metrics.http_req_duration?.values?.max || 0).toFixed(1)}ms

Custom Metrics:
  health_latency p95:       ${(metrics.health_latency?.values?.['p(95)'] || 0).toFixed(1)}ms
  trajectories_latency p95: ${(metrics.trajectories_latency?.values?.['p(95)'] || 0).toFixed(1)}ms
  error_rate:               ${((metrics.errors?.values?.rate || 0) * 100).toFixed(3)}%

Thresholds:
${Object.entries(data.thresholds || {})
  .map(([name, passed]) => `  ${passed ? '✓' : '✗'} ${name}`)
  .join('\n')}

================================================================================
`;
}
