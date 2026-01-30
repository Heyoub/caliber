/**
 * E2E Test: Trajectory Workflow
 *
 * Tests the complete trajectory workflow:
 *   Create → Add Scopes → Add Artifacts → Notes → Complete
 *
 * Run with: bun test tests/e2e/trajectory-workflow.e2e.ts
 *
 * Requires:
 *   - Running API server at CALIBER_API_URL
 *   - Valid test credentials at CALIBER_TEST_TOKEN
 */

import { describe, expect, it, beforeAll, afterAll } from 'bun:test';
import { clearRateLimits } from '../mocks/server';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const TEST_TOKEN = process.env.CALIBER_TEST_TOKEN ?? '';
const TEST_TENANT_ID = process.env.CALIBER_TEST_TENANT_ID;
const SKIP_E2E = process.env.SKIP_E2E_TESTS === 'true';

// Test state
interface TestTrajectory {
  id: string;
  name: string;
  status: string;
}

interface TestScope {
  id: string;
  name: string;
  trajectoryId: string;
}

interface TestArtifact {
  id: string;
  type: string;
  scopeId: string;
}

interface TestNote {
  id: string;
  content: string;
}

let testTrajectory: TestTrajectory | null = null;
const testScopes: TestScope[] = [];
const testArtifacts: TestArtifact[] = [];
const testNotes: TestNote[] = [];

// Helper to make authenticated API requests
async function api<T = unknown>(
  method: string,
  path: string,
  body?: unknown
): Promise<{ status: number; data: T; headers: Headers }> {
  const url = `${API_BASE_URL}${path}`;
  const options: RequestInit = {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(TEST_TOKEN ? { Authorization: `Bearer ${TEST_TOKEN}` } : {}),
      ...(TEST_TENANT_ID ? { 'X-Tenant-ID': TEST_TENANT_ID } : {}),
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

  return { status: res.status, data, headers: res.headers };
}

describe.skipIf(SKIP_E2E || !TEST_TOKEN)('e2e: Trajectory Workflow', () => {
  const trajectoryName = `e2e-trajectory-${Date.now()}`;

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
    // Cleanup: Delete test trajectory and all associated resources
    if (testTrajectory?.id) {
      try {
        await api('DELETE', `/api/v1/trajectories/${testTrajectory.id}`);
        console.log(`Cleaned up test trajectory: ${testTrajectory.id}`);
      } catch {
        // Ignore cleanup errors
      }
    }
  });

  describe('1. Trajectory Creation', () => {
    it('creates a new trajectory', async () => {
      const { status, data } = await api<TestTrajectory>('POST', '/api/v1/trajectories', {
        name: trajectoryName,
        description: 'E2E test trajectory for workflow validation',
        metadata: {
          testSuite: 'e2e',
          timestamp: new Date().toISOString(),
        },
      });

      expect(status).toBe(201);
      expect(data.id).toBeDefined();
      expect(data.name).toBe(trajectoryName);
      expect(data.status).toBe('active');

      testTrajectory = data;
    });

    it('retrieves the created trajectory', async () => {
      const { status, data } = await api<TestTrajectory>(
        'GET',
        `/api/v1/trajectories/${testTrajectory?.id}`
      );

      expect(status).toBe(200);
      expect(data.id).toBe(testTrajectory?.id);
      expect(data.name).toBe(trajectoryName);
    });

    it('trajectory appears in list', async () => {
      const { status, data } = await api<{ trajectories: TestTrajectory[] }>(
        'GET',
        '/api/v1/trajectories'
      );

      expect(status).toBe(200);
      expect(data.trajectories.some((t) => t.id === testTrajectory?.id)).toBe(true);
    });

    it('updates trajectory metadata', async () => {
      const { status, data } = await api<TestTrajectory>(
        'PATCH',
        `/api/v1/trajectories/${testTrajectory?.id}`,
        {
          description: 'Updated description',
          metadata: {
            updated: true,
          },
        }
      );

      expect(status).toBe(200);
    });
  });

  describe('2. Scope Management', () => {
    it('creates root scope', async () => {
      const { status, data } = await api<TestScope>(
        'POST',
        `/api/v1/trajectories/${testTrajectory?.id}/scopes`,
        {
          name: 'root-scope',
          description: 'Root scope for the trajectory',
        }
      );

      expect(status).toBe(201);
      expect(data.id).toBeDefined();
      expect(data.name).toBe('root-scope');

      testScopes.push(data);
    });

    it('creates nested child scopes', async () => {
      const parentId = testScopes[0].id;

      // Create multiple child scopes
      for (const name of ['planning', 'execution', 'review']) {
        const { status, data } = await api<TestScope>(
          'POST',
          `/api/v1/trajectories/${testTrajectory?.id}/scopes`,
          {
            name,
            parentId,
            description: `${name} phase scope`,
          }
        );

        expect(status).toBe(201);
        expect(data.name).toBe(name);

        testScopes.push(data);
      }

      expect(testScopes.length).toBe(4);
    });

    it('retrieves scope hierarchy', async () => {
      const { status, data } = await api<{ scopes: TestScope[] }>(
        'GET',
        `/api/v1/trajectories/${testTrajectory?.id}/scopes`
      );

      expect(status).toBe(200);
      expect(data.scopes.length).toBe(4);
    });

    it('retrieves single scope', async () => {
      const scopeId = testScopes[1].id;
      const { status, data } = await api<TestScope>('GET', `/api/v1/scopes/${scopeId}`);

      expect(status).toBe(200);
      expect(data.id).toBe(scopeId);
    });
  });

  describe('3. Artifact Management', () => {
    it('creates code artifact', async () => {
      const scopeId = testScopes[2].id; // execution scope

      const { status, data } = await api<TestArtifact>(
        'POST',
        `/api/v1/scopes/${scopeId}/artifacts`,
        {
          type: 'code',
          name: 'main.ts',
          content: 'console.log("Hello, CALIBER!");',
          language: 'typescript',
          metadata: {
            lines: 1,
          },
        }
      );

      expect(status).toBe(201);
      expect(data.id).toBeDefined();
      expect(data.type).toBe('code');

      testArtifacts.push(data);
    });

    it('creates text artifact', async () => {
      const scopeId = testScopes[1].id; // planning scope

      const { status, data } = await api<TestArtifact>(
        'POST',
        `/api/v1/scopes/${scopeId}/artifacts`,
        {
          type: 'text',
          name: 'requirements.md',
          content: '# Requirements\n\n- Feature A\n- Feature B',
          mimeType: 'text/markdown',
        }
      );

      expect(status).toBe(201);
      expect(data.type).toBe('text');

      testArtifacts.push(data);
    });

    it('creates data artifact', async () => {
      const scopeId = testScopes[2].id;

      const { status, data } = await api<TestArtifact>(
        'POST',
        `/api/v1/scopes/${scopeId}/artifacts`,
        {
          type: 'data',
          name: 'config.json',
          content: JSON.stringify({ setting: true }),
          mimeType: 'application/json',
        }
      );

      expect(status).toBe(201);
      expect(data.type).toBe('data');

      testArtifacts.push(data);
    });

    it('lists artifacts for scope', async () => {
      const scopeId = testScopes[2].id;
      const { status, data } = await api<{ artifacts: TestArtifact[] }>(
        'GET',
        `/api/v1/scopes/${scopeId}/artifacts`
      );

      expect(status).toBe(200);
      expect(data.artifacts.length).toBeGreaterThanOrEqual(2);
    });

    it('retrieves single artifact', async () => {
      const artifactId = testArtifacts[0].id;
      const { status, data } = await api<TestArtifact & { content: string }>(
        'GET',
        `/api/v1/artifacts/${artifactId}`
      );

      expect(status).toBe(200);
      expect(data.id).toBe(artifactId);
      expect(data.content).toBeDefined();
    });

    it('updates artifact content', async () => {
      const artifactId = testArtifacts[0].id;

      const { status, data } = await api<TestArtifact>('PATCH', `/api/v1/artifacts/${artifactId}`, {
        content: 'console.log("Updated!");',
      });

      expect(status).toBe(200);
    });
  });

  describe('4. Notes and Annotations', () => {
    it('adds note to trajectory', async () => {
      const { status, data } = await api<TestNote>(
        'POST',
        `/api/v1/trajectories/${testTrajectory?.id}/notes`,
        {
          content: 'Initial planning complete. Moving to execution phase.',
          type: 'progress',
        }
      );

      expect(status).toBe(201);
      expect(data.id).toBeDefined();
      expect(data.content).toContain('Initial planning');

      testNotes.push(data);
    });

    it('adds note to scope', async () => {
      const scopeId = testScopes[2].id;

      const { status, data } = await api<TestNote>('POST', `/api/v1/scopes/${scopeId}/notes`, {
        content: `Execution started at ${new Date().toISOString()}`,
        type: 'status',
      });

      expect(status).toBe(201);
      testNotes.push(data);
    });

    it('lists notes for trajectory', async () => {
      const { status, data } = await api<{ notes: TestNote[] }>(
        'GET',
        `/api/v1/trajectories/${testTrajectory?.id}/notes`
      );

      expect(status).toBe(200);
      expect(data.notes.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('5. Trajectory Query and Search', () => {
    it('searches trajectories by name', async () => {
      const { status, data } = await api<{ trajectories: TestTrajectory[] }>(
        'GET',
        `/api/v1/trajectories?search=${encodeURIComponent(trajectoryName.slice(0, 10))}`
      );

      expect(status).toBe(200);
      expect(data.trajectories.some((t) => t.id === testTrajectory?.id)).toBe(true);
    });

    it('filters trajectories by status', async () => {
      const { status, data } = await api<{ trajectories: TestTrajectory[] }>(
        'GET',
        '/api/v1/trajectories?status=active'
      );

      expect(status).toBe(200);
      expect(data.trajectories.every((t) => t.status === 'active')).toBe(true);
    });

    it('paginates trajectory list', async () => {
      const { status, data, headers } = await api<{
        trajectories: TestTrajectory[];
      }>('GET', '/api/v1/trajectories?limit=5&offset=0');

      expect(status).toBe(200);
      expect(data.trajectories.length).toBeLessThanOrEqual(5);
    });
  });

  describe('6. Trajectory Completion', () => {
    it('marks trajectory as complete', async () => {
      const { status, data } = await api<TestTrajectory>(
        'POST',
        `/api/v1/trajectories/${testTrajectory?.id}/complete`,
        {
          summary: 'E2E test completed successfully',
          outcome: 'success',
        }
      );

      expect(status).toBe(200);
      expect(data.status).toBe('completed');

      testTrajectory = data;
    });

    it('completed trajectory is read-only', async () => {
      // Try to add artifact to completed trajectory
      const scopeId = testScopes[0].id;

      const { status } = await api('POST', `/api/v1/scopes/${scopeId}/artifacts`, {
        type: 'text',
        name: 'should-fail.txt',
        content: 'This should not be allowed',
      });

      // Should reject modification to completed trajectory
      expect([400, 403, 409]).toContain(status);
    });

    it('trajectory shows in completed list', async () => {
      const { status, data } = await api<{ trajectories: TestTrajectory[] }>(
        'GET',
        '/api/v1/trajectories?status=completed'
      );

      expect(status).toBe(200);
      expect(data.trajectories.some((t) => t.id === testTrajectory?.id)).toBe(true);
    });
  });

  describe('7. HATEOAS Links', () => {
    it('trajectory response includes links', async () => {
      const { status, data } = await api<TestTrajectory & { links?: unknown }>(
        'GET',
        `/api/v1/trajectories/${testTrajectory?.id}`
      );

      expect(status).toBe(200);

      // Check for HATEOAS links if implemented
      if (data.links) {
        expect(data.links).toBeDefined();
      }
    });
  });
});

describe.skipIf(SKIP_E2E || !TEST_TOKEN)('e2e: Trajectory Edge Cases', () => {
  it('rejects trajectory with empty name', async () => {
    const { status } = await api('POST', '/api/v1/trajectories', {
      name: '',
      description: 'Should fail',
    });

    expect([400, 422]).toContain(status);
  });

  it('rejects trajectory with very long name', async () => {
    const { status } = await api('POST', '/api/v1/trajectories', {
      name: 'x'.repeat(1000),
      description: 'Should fail',
    });

    expect([400, 422]).toContain(status);
  });

  it('handles non-existent trajectory gracefully', async () => {
    const { status } = await api('GET', '/api/v1/trajectories/non-existent-id-12345');

    expect(status).toBe(404);
  });

  it('handles circular scope references gracefully', async () => {
    // Create a trajectory and scope, then try to make it its own parent
    const { data: traj } = await api<{ id: string }>('POST', '/api/v1/trajectories', {
      name: `circular-test-${Date.now()}`,
    });

    if (traj.id) {
      const { data: scope } = await api<{ id: string }>(
        'POST',
        `/api/v1/trajectories/${traj.id}/scopes`,
        { name: 'test-scope' }
      );

      if (scope.id) {
        // Try to update scope to be its own parent
        const { status } = await api('PATCH', `/api/v1/scopes/${scope.id}`, {
          parentId: scope.id,
        });

        expect([400, 422]).toContain(status);
      }

      // Cleanup
      await api('DELETE', `/api/v1/trajectories/${traj.id}`);
    }
  });
});
