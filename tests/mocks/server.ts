/**
 * MSW Server Setup with Auto-Fallback
 *
 * Automatically uses mocks when the real server is unavailable.
 * Import this in test files to enable mocking.
 */

import { setupServer } from 'msw/node';
import { handlers, resetStores } from './handlers';

// Create the MSW server
export const server = setupServer(...handlers);

// Track mock state
let isMockActive = false;
let serverCheckDone = false;

/**
 * Check if the real server is available
 */
export async function isServerAvailable(
  baseUrl = 'http://localhost:3000'
): Promise<boolean> {
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 1000);

    const response = await fetch(`${baseUrl}/health`, {
      signal: controller.signal,
    });

    clearTimeout(timeout);
    return response.ok;
  } catch {
    return false;
  }
}

/**
 * Start mocking - use when server is unavailable
 */
export function startMocking() {
  if (!isMockActive) {
    server.listen({ onUnhandledRequest: 'bypass' });
    isMockActive = true;
    console.log('[MSW] Mock server started');
  }
}

/**
 * Stop mocking
 */
export function stopMocking() {
  if (isMockActive) {
    server.close();
    isMockActive = false;
    console.log('[MSW] Mock server stopped');
  }
}

/**
 * Reset mock handlers between tests (keeps store state for E2E tests)
 */
export function resetMocks() {
  if (isMockActive) {
    server.resetHandlers();
    // Don't reset stores by default - E2E tests need state persistence
    // Use resetStores() explicitly when needed
  }
}

/**
 * Full reset including stores (for isolated unit tests)
 */
export function resetAll() {
  if (isMockActive) {
    server.resetHandlers();
    resetStores();
  }
}

/**
 * Setup mocking with auto-detection
 * Call this in beforeAll() to automatically use mocks if server unavailable
 */
export async function setupMocking(
  baseUrl = 'http://localhost:3000'
): Promise<{ usingMocks: boolean }> {
  if (serverCheckDone) {
    return { usingMocks: isMockActive };
  }

  const available = await isServerAvailable(baseUrl);

  if (!available) {
    startMocking();
    console.log(`[MSW] Real server not available at ${baseUrl}, using mocks`);
  } else {
    console.log(`[MSW] Real server available at ${baseUrl}, using live API`);
  }

  serverCheckDone = true;
  return { usingMocks: !available };
}

/**
 * Force mocking regardless of server availability
 * Useful for testing mock behavior specifically
 */
export function forceMocking() {
  startMocking();
  serverCheckDone = true;
  return { usingMocks: true };
}

/**
 * Get a test token for authenticated requests
 * Works with both real server (if configured) and mocks
 */
export function getTestToken(): string {
  // In mock mode, any valid-looking token works
  // In real mode, this should come from env
  return (
    process.env.CALIBER_TEST_TOKEN ||
    'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0LXVzZXIiLCJpYXQiOjE3MDAwMDAwMDB9.dGVzdC1zaWduYXR1cmUtZm9yLW1vY2stdG9rZW4'
  );
}

/**
 * Check if currently using mocks
 */
export function isUsingMocks(): boolean {
  return isMockActive;
}

// Re-export for convenience
export { handlers, resetStores, clearRateLimits } from './handlers';
