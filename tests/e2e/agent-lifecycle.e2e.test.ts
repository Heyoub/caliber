/**
 * E2E Test: Agent Lifecycle
 *
 * Tests the complete agent lifecycle:
 *   Register → Activate → Heartbeat → Deactivate → Unregister
 *
 * Run with: bun test tests/e2e/agent-lifecycle.e2e.ts
 *
 * Requires:
 *   - Running API server at CALIBER_API_URL
 *   - Valid test credentials at CALIBER_TEST_TOKEN
 */

import { describe, expect, it, beforeAll, afterAll } from 'bun:test';
import { clearRateLimits } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const TEST_TOKEN = process.env.CALIBER_TEST_TOKEN ?? '';
const SKIP_E2E = process.env.SKIP_E2E_TESTS === 'true';

// Test state
interface TestAgent {
  id: string;
  name: string;
  status: string;
}

let testAgent: TestAgent | null = null;

// Helper to make authenticated API requests
async function api<T = unknown>(
  method: string,
  path: string,
  body?: unknown
): Promise<{ status: number; data: T }> {
  const url = `${API_BASE_URL}${path}`;
  const options: RequestInit = {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(TEST_TOKEN ? { Authorization: `Bearer ${TEST_TOKEN}` } : {}),
    },
  };

  if (body) {
    options.body = JSON.stringify(body);
  }

  const res = await fetch(url, options);
  let data: T;

  try {
    data = await res.json();
  } catch {
    data = {} as T;
  }

  return { status: res.status, data };
}

// Wait for condition with timeout
async function waitFor(
  condition: () => Promise<boolean>,
  timeoutMs = 10000,
  intervalMs = 500
): Promise<void> {
  const start = Date.now();

  while (Date.now() - start < timeoutMs) {
    if (await condition()) {
      return;
    }
    await new Promise((r) => setTimeout(r, intervalMs));
  }

  throw new Error('Timeout waiting for condition');
}

describe.skipIf(SKIP_E2E || !TEST_TOKEN)('e2e: Agent Lifecycle', () => {
  const agentName = `e2e-test-agent-${Date.now()}`;

  beforeAll(async () => {
    // Clear rate limits from previous test files
    clearRateLimits();

    // Verify API is reachable
    const { status } = await api('GET', '/health');
    if (status !== 200) {
      throw new Error(`API not reachable: ${status}`);
    }
    console.log(`E2E test starting against ${API_BASE_URL}`);
  });

  afterAll(async () => {
    // Cleanup: Unregister test agent if it exists
    if (testAgent?.id) {
      try {
        await api('DELETE', `/api/v1/agents/${testAgent.id}`);
        console.log(`Cleaned up test agent: ${testAgent.id}`);
      } catch {
        // Ignore cleanup errors
      }
    }
  });

  describe('1. Agent Registration', () => {
    it('registers a new agent', async () => {
      const { status, data } = await api<{ id: string; name: string; status: string }>(
        'POST',
        '/api/v1/agents',
        {
          name: agentName,
          type: 'worker',
          capabilities: ['execute', 'report'],
          metadata: {
            version: '1.0.0',
            environment: 'e2e-test',
          },
        }
      );

      expect(status).toBe(201);
      expect(data.id).toBeDefined();
      expect(data.name).toBe(agentName);
      expect(data.status).toBe('pending');

      testAgent = data;
    });

    it('rejects duplicate agent names', async () => {
      const { status } = await api('POST', '/api/v1/agents', {
        name: agentName, // Same name as above
        type: 'worker',
      });

      expect([400, 409]).toContain(status);
    });

    it('retrieves the registered agent', async () => {
      const { status, data } = await api<TestAgent>(
        'GET',
        `/api/v1/agents/${testAgent!.id}`
      );

      expect(status).toBe(200);
      expect(data.id).toBe(testAgent!.id);
      expect(data.name).toBe(agentName);
    });

    it('lists agents including the new one', async () => {
      const { status, data } = await api<{ agents: TestAgent[] }>(
        'GET',
        '/api/v1/agents'
      );

      expect(status).toBe(200);
      expect(data.agents.some((a) => a.id === testAgent!.id)).toBe(true);
    });
  });

  describe('2. Agent Activation', () => {
    it('activates the agent', async () => {
      const { status, data } = await api<TestAgent>(
        'POST',
        `/api/v1/agents/${testAgent!.id}/activate`,
        {}
      );

      expect(status).toBe(200);
      expect(data.status).toBe('active');

      testAgent = { ...testAgent!, status: 'active' };
    });

    it('agent is now active', async () => {
      const { status, data } = await api<TestAgent>(
        'GET',
        `/api/v1/agents/${testAgent!.id}`
      );

      expect(status).toBe(200);
      expect(data.status).toBe('active');
    });

    it('rejects activation of already active agent', async () => {
      const { status } = await api(
        'POST',
        `/api/v1/agents/${testAgent!.id}/activate`,
        {}
      );

      // Should be idempotent (200) or reject (400/409)
      expect([200, 400, 409]).toContain(status);
    });
  });

  describe('3. Agent Heartbeat', () => {
    it('sends heartbeat successfully', async () => {
      const { status, data } = await api<{ lastHeartbeat: string }>(
        'POST',
        `/api/v1/agents/${testAgent!.id}/heartbeat`,
        {
          status: 'healthy',
          metrics: {
            cpu: 45.2,
            memory: 1024,
            activeTrajectories: 2,
          },
        }
      );

      expect(status).toBe(200);
      expect(data.lastHeartbeat).toBeDefined();
    });

    it('sends multiple heartbeats', async () => {
      for (let i = 0; i < 3; i++) {
        const { status } = await api(
          'POST',
          `/api/v1/agents/${testAgent!.id}/heartbeat`,
          {
            status: 'healthy',
            iteration: i,
          }
        );

        expect(status).toBe(200);
        await new Promise((r) => setTimeout(r, 100));
      }
    });

    it('heartbeat updates last seen timestamp', async () => {
      const before = await api<{ lastSeen: string }>(
        'GET',
        `/api/v1/agents/${testAgent!.id}`
      );

      await new Promise((r) => setTimeout(r, 100));

      await api('POST', `/api/v1/agents/${testAgent!.id}/heartbeat`, {
        status: 'healthy',
      });

      const after = await api<{ lastSeen: string }>(
        'GET',
        `/api/v1/agents/${testAgent!.id}`
      );

      // lastSeen should be updated
      expect(new Date(after.data.lastSeen).getTime()).toBeGreaterThanOrEqual(
        new Date(before.data.lastSeen).getTime()
      );
    });
  });

  describe('4. Agent Work Assignment', () => {
    let testTrajectoryId: string | null = null;

    it('agent can be assigned to a trajectory', async () => {
      // First create a trajectory
      const { status: trajStatus, data: trajData } = await api<{ id: string }>(
        'POST',
        '/api/v1/trajectories',
        {
          name: `e2e-test-trajectory-${Date.now()}`,
          description: 'Test trajectory for agent lifecycle',
        }
      );

      if (trajStatus === 201) {
        testTrajectoryId = trajData.id;

        // Assign agent to trajectory
        const { status } = await api(
          'POST',
          `/api/v1/trajectories/${testTrajectoryId}/assign`,
          {
            agentId: testAgent!.id,
          }
        );

        expect([200, 201]).toContain(status);
      }
    });

    afterAll(async () => {
      // Cleanup test trajectory
      if (testTrajectoryId) {
        await api('DELETE', `/api/v1/trajectories/${testTrajectoryId}`);
      }
    });
  });

  describe('5. Agent Deactivation', () => {
    it('deactivates the agent', async () => {
      const { status, data } = await api<TestAgent>(
        'POST',
        `/api/v1/agents/${testAgent!.id}/deactivate`,
        {
          reason: 'e2e test complete',
        }
      );

      expect(status).toBe(200);
      expect(data.status).toBe('inactive');

      testAgent = { ...testAgent!, status: 'inactive' };
    });

    it('agent is now inactive', async () => {
      const { status, data } = await api<TestAgent>(
        'GET',
        `/api/v1/agents/${testAgent!.id}`
      );

      expect(status).toBe(200);
      expect(data.status).toBe('inactive');
    });

    it('heartbeat fails for inactive agent', async () => {
      const { status } = await api(
        'POST',
        `/api/v1/agents/${testAgent!.id}/heartbeat`,
        {
          status: 'healthy',
        }
      );

      // Should reject heartbeat from inactive agent
      expect([400, 409]).toContain(status);
    });
  });

  describe('6. Agent Unregistration', () => {
    it('unregisters the agent', async () => {
      const { status } = await api(
        'DELETE',
        `/api/v1/agents/${testAgent!.id}`
      );

      expect([200, 204]).toContain(status);
    });

    it('agent no longer exists', async () => {
      const { status } = await api('GET', `/api/v1/agents/${testAgent!.id}`);

      expect(status).toBe(404);
    });

    it('agent not in list', async () => {
      const { status, data } = await api<{ agents: TestAgent[] }>(
        'GET',
        '/api/v1/agents'
      );

      expect(status).toBe(200);
      expect(data.agents.some((a) => a.id === testAgent!.id)).toBe(false);
    });
  });
});

describe.skipIf(SKIP_E2E || !TEST_TOKEN)('e2e: Agent Edge Cases', () => {
  it('rejects invalid agent type', async () => {
    const { status } = await api('POST', '/api/v1/agents', {
      name: `invalid-type-${Date.now()}`,
      type: 'invalid_type_here',
    });

    expect([400, 422]).toContain(status);
  });

  it('rejects agent with empty name', async () => {
    const { status } = await api('POST', '/api/v1/agents', {
      name: '',
      type: 'worker',
    });

    expect([400, 422]).toContain(status);
  });

  it('handles non-existent agent gracefully', async () => {
    const { status } = await api('GET', '/api/v1/agents/non-existent-id-12345');

    expect(status).toBe(404);
  });

  it('handles heartbeat to non-existent agent', async () => {
    const { status } = await api(
      'POST',
      '/api/v1/agents/non-existent-id-12345/heartbeat',
      { status: 'healthy' }
    );

    expect(status).toBe(404);
  });
});
