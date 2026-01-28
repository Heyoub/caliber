/**
 * Component Tests: Typestate enforcement
 *
 * Validates state machine constraints for Delegation, Lock, and Handoff flows.
 */

import { describe, expect, it, beforeAll, afterAll } from 'bun:test';
import { clearRateLimits, getTestToken, isUsingMocks, setupMocking } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const USING_MOCKS = isUsingMocks();
const TEST_TOKEN = USING_MOCKS
  ? process.env.CALIBER_TEST_TOKEN ?? getTestToken()
  : process.env.CALIBER_TEST_TOKEN ?? '';
const SKIP_COMPONENT = process.env.SKIP_COMPONENT_TESTS === 'true';

interface ApiResult<T> {
  status: number;
  data: T;
}

async function api<T = unknown>(
  method: string,
  path: string,
  body?: unknown
): Promise<ApiResult<T>> {
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

function idOf(data: Record<string, unknown>, fields: string[]): string {
  for (const field of fields) {
    const value = data[field];
    if (typeof value === 'string' && value.length > 0) return value;
  }
  throw new Error(`Could not find id in fields: ${fields.join(', ')}`);
}

let agentA: string | null = null;
let agentB: string | null = null;
let trajectoryId: string | null = null;
let scopeId: string | null = null;
let lockId: string | null = null;

beforeAll(async () => {
  await setupMocking(API_BASE_URL);
  clearRateLimits();

  if (USING_MOCKS) return;

  // Create two agents
  const agentNameA = `typestate-agent-a-${Date.now()}`;
  const agentNameB = `typestate-agent-b-${Date.now()}`;

  const agentARes = await api<Record<string, unknown>>('POST', '/api/v1/agents', {
    name: agentNameA,
    type: 'worker',
    capabilities: ['execute'],
  });
  expect(agentARes.status).toBe(201);
  agentA = idOf(agentARes.data, ['id', 'agent_id']);

  const agentBRes = await api<Record<string, unknown>>('POST', '/api/v1/agents', {
    name: agentNameB,
    type: 'worker',
    capabilities: ['execute'],
  });
  expect(agentBRes.status).toBe(201);
  agentB = idOf(agentBRes.data, ['id', 'agent_id']);

  // Create trajectory + scope
  const trajRes = await api<Record<string, unknown>>('POST', '/api/v1/trajectories', {
    name: `typestate-trajectory-${Date.now()}`,
  });
  expect(trajRes.status).toBe(201);
  trajectoryId = idOf(trajRes.data, ['id', 'trajectory_id']);

  const scopeRes = await api<Record<string, unknown>>(
    'POST',
    `/api/v1/trajectories/${trajectoryId}/scopes`,
    {
      name: 'typestate-scope',
      description: 'scope for typestate tests',
    }
  );
  expect(scopeRes.status).toBe(201);
  scopeId = idOf(scopeRes.data, ['id', 'scope_id']);
});

afterAll(async () => {
  if (USING_MOCKS) return;

  if (lockId && agentA) {
    await api('POST', `/api/v1/locks/${lockId}/release`, {
      releasing_agent_id: agentA,
    });
  }

  if (trajectoryId) {
    await api('DELETE', `/api/v1/trajectories/${trajectoryId}`);
  }

  if (agentA) {
    await api('DELETE', `/api/v1/agents/${agentA}`);
  }
  if (agentB) {
    await api('DELETE', `/api/v1/agents/${agentB}`);
  }
});

describe.skipIf(SKIP_COMPONENT || USING_MOCKS || !TEST_TOKEN)(
  'component: Typestate enforcement',
  () => {
    it('delegation requires accept before complete', async () => {
      const createRes = await api<Record<string, unknown>>(
        'POST',
        '/api/v1/delegations',
        {
          from_agent_id: agentA,
          to_agent_id: agentB,
          trajectory_id: trajectoryId,
          scope_id: scopeId,
          task_description: 'typestate delegation test',
        }
      );
      expect(createRes.status).toBe(201);
      const delegationId = idOf(createRes.data, ['delegation_id', 'id']);

      const completeRes = await api(
        'POST',
        `/api/v1/delegations/${delegationId}/complete`,
        {
          result: {
            status: 'Success',
            output: 'done',
            artifacts: [],
            error: null,
          },
        }
      );
      expect([400, 409]).toContain(completeRes.status);

      const acceptRes = await api('POST', `/api/v1/delegations/${delegationId}/accept`, {
        accepting_agent_id: agentB,
      });
      expect([200, 204]).toContain(acceptRes.status);

      const completeRes2 = await api(
        'POST',
        `/api/v1/delegations/${delegationId}/complete`,
        {
          result: {
            status: 'Success',
            output: 'done',
            artifacts: [],
            error: null,
          },
        }
      );
      expect([200, 204]).toContain(completeRes2.status);
    });

    it('rejects double-acquire on exclusive lock', async () => {
      const resourceId = crypto.randomUUID();
      const acquire1 = await api<Record<string, unknown>>('POST', '/api/v1/locks/acquire', {
        resource_type: 'test',
        resource_id: resourceId,
        holder_agent_id: agentA,
        timeout_ms: 10000,
        mode: 'exclusive',
      });
      expect(acquire1.status).toBe(201);
      lockId = idOf(acquire1.data, ['lock_id', 'id']);

      const acquire2 = await api('POST', '/api/v1/locks/acquire', {
        resource_type: 'test',
        resource_id: resourceId,
        holder_agent_id: agentB,
        timeout_ms: 10000,
        mode: 'exclusive',
      });
      expect([409, 400]).toContain(acquire2.status);
    });

    it('handoff requires accept before complete', async () => {
      const createRes = await api<Record<string, unknown>>('POST', '/api/v1/handoffs', {
        from_agent_id: agentA,
        to_agent_id: agentB,
        trajectory_id: trajectoryId,
        scope_id: scopeId,
        reason: 'Timeout',
        context_snapshot: [1, 2, 3],
      });
      expect(createRes.status).toBe(201);
      const handoffId = idOf(createRes.data, ['handoff_id', 'id']);

      const completeRes = await api('POST', `/api/v1/handoffs/${handoffId}/complete`);
      expect([409, 400]).toContain(completeRes.status);

      const acceptRes = await api('POST', `/api/v1/handoffs/${handoffId}/accept`, {
        accepting_agent_id: agentB,
      });
      expect([200, 204]).toContain(acceptRes.status);

      const completeRes2 = await api('POST', `/api/v1/handoffs/${handoffId}/complete`);
      expect([200, 204]).toContain(completeRes2.status);
    });
  }
);
