/**
 * k6 Load Testing - Shared Configuration
 *
 * Common thresholds, options, and utilities for load tests.
 * Import this in test files: import { thresholds, baseOptions } from './config.js';
 */

// API Configuration
export const API_BASE_URL = __ENV.CALIBER_API_URL || 'http://localhost:3000';
export const API_KEY = __ENV.CALIBER_API_KEY || '';
export const TENANT_ID = __ENV.CALIBER_TENANT_ID || '';

// Quality Gates - These are our SLOs
export const thresholds = {
  // Latency thresholds
  http_req_duration: [
    'p(50)<50',   // 50th percentile < 50ms
    'p(95)<100',  // 95th percentile < 100ms (primary SLO)
    'p(99)<200',  // 99th percentile < 200ms
    'max<1000',   // No request > 1s
  ],

  // Error rate threshold
  http_req_failed: [
    'rate<0.001', // < 0.1% error rate
  ],

  // Throughput (set per-test)
  http_reqs: [
    'rate>100', // At least 100 RPS baseline
  ],
};

// Thresholds for stress testing (more lenient)
export const stressThresholds = {
  http_req_duration: [
    'p(95)<500',  // Allow higher latency under stress
    'p(99)<1000',
  ],
  http_req_failed: [
    'rate<0.01', // < 1% error rate under stress
  ],
};

// Base options for all tests
export const baseOptions = {
  // Don't throw on HTTP errors (we want to count them)
  throw: false,

  // Tags for filtering results
  tags: {
    env: __ENV.K6_ENV || 'local',
  },
};

// Common headers
export function getHeaders(authenticated = false) {
  const headers = {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
  };

  if (authenticated && API_KEY) {
    headers['Authorization'] = `Bearer ${API_KEY}`;
  }

  if (TENANT_ID) {
    headers['X-Tenant-ID'] = TENANT_ID;
  }

  return headers;
}

// Helper to check response and tag failures
export function checkResponse(res, checks) {
  const results = {};

  for (const [name, condition] of Object.entries(checks)) {
    results[name] = condition(res);
  }

  return results;
}

// Common check functions
export const checks = {
  isOk: (r) => r.status >= 200 && r.status < 300,
  is200: (r) => r.status === 200,
  is201: (r) => r.status === 201,
  is401: (r) => r.status === 401,
  is404: (r) => r.status === 404,
  hasBody: (r) => r.body && r.body.length > 0,
  isJson: (r) => {
    try {
      JSON.parse(r.body);
      return true;
    } catch {
      return false;
    }
  },
  durationBelow: (ms) => (r) => r.timings.duration < ms,
};

// Scenarios for different load patterns
export const scenarios = {
  // Constant load - good for baseline
  constant: (vus, duration) => ({
    executor: 'constant-vus',
    vus,
    duration,
  }),

  // Ramping load - good for finding limits
  ramping: (stages) => ({
    executor: 'ramping-vus',
    startVUs: 0,
    stages,
    gracefulRampDown: '30s',
  }),

  // Spike test - sudden load increase
  spike: (baseVUs, spikeVUs, duration) => ({
    executor: 'ramping-vus',
    startVUs: baseVUs,
    stages: [
      { duration: '1m', target: baseVUs },    // Warm up
      { duration: '10s', target: spikeVUs },  // Spike!
      { duration: '3m', target: spikeVUs },   // Hold spike
      { duration: '10s', target: baseVUs },   // Recovery
      { duration: '2m', target: baseVUs },    // Cool down
    ],
  }),

  // Soak test - sustained load for long period
  soak: (vus, duration) => ({
    executor: 'constant-vus',
    vus,
    duration,
  }),
};

// Utility to generate random test data
export function randomString(length = 10) {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789';
  let result = '';
  for (let i = 0; i < length; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
}

export function randomId() {
  return `test-${randomString(8)}-${Date.now()}`;
}
