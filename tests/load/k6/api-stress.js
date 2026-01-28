/**
 * k6 Load Test - API Stress Test
 *
 * Finds the breaking point by ramping from 10 to 1000 VUs.
 * Run with: k6 run tests/load/k6/api-stress.js
 *
 * Environment variables:
 *   CALIBER_API_URL - API base URL (default: http://localhost:3000)
 *   K6_MAX_VUS     - Maximum VUs to ramp to (default: 1000)
 */

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import {
  API_BASE_URL,
  stressThresholds,
  baseOptions,
  getHeaders,
  checks,
} from './config.js';

// Custom metrics for stress analysis
const requestsPerSecond = new Trend('requests_per_second', true);
const errorsByStage = new Counter('errors_by_stage');
const breakingPointVUs = new Trend('breaking_point_vus');
const latencyByLoad = new Trend('latency_by_load', true);

const MAX_VUS = parseInt(__ENV.K6_MAX_VUS || '1000', 10);

// Test configuration - Ramping stress test
export const options = {
  ...baseOptions,

  scenarios: {
    stress: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        // Warm up
        { duration: '30s', target: 10 },

        // Ramp up progressively
        { duration: '1m', target: 50 },
        { duration: '1m', target: 100 },
        { duration: '1m', target: 200 },
        { duration: '1m', target: 500 },
        { duration: '2m', target: MAX_VUS },

        // Hold at max
        { duration: '2m', target: MAX_VUS },

        // Ramp down (recovery test)
        { duration: '1m', target: 100 },
        { duration: '30s', target: 0 },
      ],
      gracefulRampDown: '30s',
    },
  },

  thresholds: {
    ...stressThresholds,
    // Track when we hit the breaking point
    http_req_failed: ['rate<0.05'], // Allow up to 5% errors in stress
    http_req_duration: ['p(95)<1000'], // 1s p95 acceptable under stress
  },
};

// Track state across iterations
let currentStage = 'warmup';
let peakRPS = 0;
let breakingPointFound = false;

export function setup() {
  // Verify API is reachable
  const res = http.get(`${API_BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`API not reachable: ${res.status}`);
  }

  console.log(`Stress test starting against ${API_BASE_URL}`);
  console.log(`Max VUs: ${MAX_VUS}`);

  return {
    startTime: Date.now(),
    stages: [],
  };
}

export default function (data) {
  const vus = __VU;
  const iteration = __ITER;

  // Determine current stage based on VU count
  if (vus <= 10) currentStage = 'warmup';
  else if (vus <= 50) currentStage = 'light';
  else if (vus <= 100) currentStage = 'moderate';
  else if (vus <= 200) currentStage = 'heavy';
  else if (vus <= 500) currentStage = 'very_heavy';
  else currentStage = 'extreme';

  // Mixed workload to stress different parts of the system
  group('Health Check', () => {
    const res = http.get(`${API_BASE_URL}/health`, {
      headers: getHeaders(),
      tags: { stage: currentStage, endpoint: 'health' },
    });

    latencyByLoad.add(res.timings.duration, { vus: vus.toString() });

    const passed = check(res, {
      'health is ok': checks.is200,
      'health fast enough': checks.durationBelow(500),
    });

    if (!passed) {
      errorsByStage.add(1, { stage: currentStage });

      // Detect breaking point
      if (!breakingPointFound && res.status >= 500) {
        breakingPointFound = true;
        breakingPointVUs.add(vus);
        console.log(`Breaking point detected at ${vus} VUs`);
      }
    }
  });

  // Small pause between requests
  sleep(0.1);

  group('Ready Check', () => {
    const res = http.get(`${API_BASE_URL}/health/ready`, {
      headers: getHeaders(),
      tags: { stage: currentStage, endpoint: 'ready' },
    });

    latencyByLoad.add(res.timings.duration, { vus: vus.toString() });

    check(res, {
      'ready is ok': (r) => r.status < 500,
    });
  });

  sleep(0.1);

  // Test auth boundary under load
  group('Auth Endpoint', () => {
    const res = http.get(`${API_BASE_URL}/api/v1/trajectories`, {
      headers: getHeaders(false),
      tags: { stage: currentStage, endpoint: 'trajectories' },
    });

    latencyByLoad.add(res.timings.duration, { vus: vus.toString() });

    const passed = check(res, {
      'auth boundary works': (r) => r.status === 401 || r.status < 500,
    });

    if (!passed) {
      errorsByStage.add(1, { stage: currentStage });
    }
  });

  // Vary sleep based on load to avoid thundering herd
  sleep(0.2 + Math.random() * 0.3);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`\nStress test completed in ${duration.toFixed(1)}s`);

  if (breakingPointFound) {
    console.log(`Breaking point was detected during the test.`);
  } else {
    console.log(`No breaking point detected - system handled ${MAX_VUS} VUs.`);
  }
}

export function handleSummary(data) {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');

  return {
    [`load-test-stress-${timestamp}.json`]: JSON.stringify(data, null, 2),
    stdout: textSummary(data),
  };
}

function textSummary(data) {
  const metrics = data.metrics;
  const thresholdResults = data.thresholds || {};

  // Calculate key statistics
  const totalRequests = metrics.http_reqs?.values?.count || 0;
  const avgRPS = metrics.http_reqs?.values?.rate || 0;
  const errorRate = (metrics.http_req_failed?.values?.rate || 0) * 100;
  const p50 = metrics.http_req_duration?.values?.['p(50)'] || 0;
  const p95 = metrics.http_req_duration?.values?.['p(95)'] || 0;
  const p99 = metrics.http_req_duration?.values?.['p(99)'] || 0;
  const maxLatency = metrics.http_req_duration?.values?.max || 0;

  // Determine overall status
  const allPassed = Object.values(thresholdResults).every(t => t);
  const status = allPassed ? 'PASSED' : 'FAILED (thresholds exceeded)';

  return `
================================================================================
CALIBER API STRESS TEST RESULTS
================================================================================

Status: ${status}

Configuration:
  Max VUs:   ${MAX_VUS}
  Target:    ${API_BASE_URL}

Request Summary:
  Total:     ${totalRequests.toLocaleString()}
  Rate:      ${avgRPS.toFixed(1)} req/s
  Failed:    ${errorRate.toFixed(2)}%

Latency Distribution:
  p50:       ${p50.toFixed(1)}ms
  p95:       ${p95.toFixed(1)}ms
  p99:       ${p99.toFixed(1)}ms
  max:       ${maxLatency.toFixed(1)}ms

Errors by Stage:
${['warmup', 'light', 'moderate', 'heavy', 'very_heavy', 'extreme']
  .map(stage => {
    const count = metrics.errors_by_stage?.values?.[stage] || 0;
    return `  ${stage.padEnd(12)}: ${count}`;
  })
  .join('\n')}

Threshold Results:
${Object.entries(thresholdResults)
  .map(([name, passed]) => `  ${passed ? '✓' : '✗'} ${name}`)
  .join('\n')}

Recommendations:
${generateRecommendations(metrics)}

================================================================================
`;
}

function generateRecommendations(metrics) {
  const recommendations = [];
  const errorRate = (metrics.http_req_failed?.values?.rate || 0) * 100;
  const p95 = metrics.http_req_duration?.values?.['p(95)'] || 0;
  const p99 = metrics.http_req_duration?.values?.['p(99)'] || 0;

  if (errorRate > 1) {
    recommendations.push('  - High error rate detected. Check server logs and increase resources.');
  }

  if (p95 > 500) {
    recommendations.push('  - p95 latency is high. Consider caching, query optimization, or horizontal scaling.');
  }

  if (p99 > 1000) {
    recommendations.push('  - p99 latency exceeds 1s. Investigate slow queries and add circuit breakers.');
  }

  if (p99 / p95 > 3) {
    recommendations.push('  - Large gap between p95 and p99. Some requests are hitting edge cases - investigate outliers.');
  }

  if (recommendations.length === 0) {
    recommendations.push('  - System performed well under stress. Consider increasing MAX_VUS for next test.');
  }

  return recommendations.join('\n');
}
