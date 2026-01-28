/**
 * Test Setup - Bun Preload
 *
 * Automatically configures MSW mocking when server is unavailable.
 * This file is loaded before all tests via bunfig.toml preload.
 *
 * IMPORTANT: Server check and mock setup happens synchronously at import time
 * so that describe.skipIf() conditions in test files work correctly.
 */

import { afterAll, afterEach } from 'bun:test';
import { server, resetStores, startMocking, stopMocking, resetMocks } from './mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';

// Synchronously check server availability at import time
// This must happen before test files' describe.skipIf() conditions are evaluated
async function checkServerAndSetupMocks(): Promise<boolean> {
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 1000);

    const response = await fetch(`${API_BASE_URL}/health`, {
      signal: controller.signal,
    });

    clearTimeout(timeout);
    return response.ok;
  } catch {
    return false;
  }
}

// Run setup immediately (top-level await)
const serverAvailable = await checkServerAndSetupMocks();

if (!serverAvailable) {
  // Start mocking BEFORE any test files load
  startMocking();
  console.log(`[MSW] Real server not available at ${API_BASE_URL}, using mocks`);

  // Remove skip flags when using mocks - tests can run
  process.env.SKIP_SECURITY_TESTS = 'false';
  process.env.SKIP_E2E_TESTS = 'false';
  process.env.SKIP_BENCH_TESTS = 'false';

  // Provide a working test token for mocks
  if (!process.env.CALIBER_TEST_TOKEN) {
    // Test token with valid JWT structure (base64url encoded HS256 signature)
    process.env.CALIBER_TEST_TOKEN =
      'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0LXVzZXIiLCJpYXQiOjE3MDAwMDAwMDB9.dGVzdC1zaWduYXR1cmUtZm9yLW1vY2stdG9rZW4';
  }
} else {
  console.log(`[MSW] Real server available at ${API_BASE_URL}, using live API`);
}

// Reset mocks between each test
afterEach(() => {
  resetMocks();
});

// Cleanup after all tests
afterAll(() => {
  resetStores(); // Clear state before stopping
  stopMocking();
});

// Export for manual use in tests
export { startMocking, stopMocking, resetMocks, resetStores, server };
export const isUsingMocks = () => !serverAvailable;
