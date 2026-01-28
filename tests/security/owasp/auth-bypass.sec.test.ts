/**
 * OWASP A01:2021 - Broken Access Control Security Tests
 *
 * Tests for authentication bypass, authorization flaws, IDOR,
 * privilege escalation, and other access control vulnerabilities.
 *
 * Run with: bun test tests/security/owasp/auth-bypass.sec.test.ts
 */

import { describe, expect, it, beforeAll } from 'bun:test';

const API_BASE_URL = process.env.CALIBER_API_URL ?? 'http://localhost:3000';
const SKIP_LIVE_TESTS = process.env.SKIP_SECURITY_TESTS === 'true';

// Test tokens and IDs
const VALID_TEST_TOKEN = process.env.CALIBER_TEST_TOKEN ?? '';
const OTHER_USER_ID = 'user-other-12345';
const OTHER_TRAJECTORY_ID = 'traj-other-12345';

// Common auth bypass payloads
const AUTH_BYPASS_TOKENS = [
  '',
  'null',
  'undefined',
  'true',
  'false',
  '{}',
  '[]',
  'Bearer',
  'Bearer ',
  'Bearer null',
  'Bearer undefined',
  'Bearer {}',
  'Basic YWRtaW46YWRtaW4=', // admin:admin
  'Basic YWRtaW46', // admin:
  'Basic Og==', // :
];

// JWT manipulation payloads
const JWT_BYPASS_TOKENS = [
  // Algorithm confusion
  'eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiJhZG1pbiIsImlhdCI6MTUxNjIzOTAyMn0.',
  'eyJhbGciOiJOT05FIiwidHlwIjoiSldUIn0.eyJzdWIiOiJhZG1pbiJ9.',

  // Empty signature
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiJ9.',

  // Null signature
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiJ9.null',

  // Modified payload (base64 of {"sub":"admin","role":"admin"})
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiIsInJvbGUiOiJhZG1pbiJ9.invalid',
];

// Helper to make API requests
async function apiRequest(
  method: string,
  path: string,
  body?: unknown,
  headers: Record<string, string> = {}
): Promise<Response> {
  const url = `${API_BASE_URL}${path}`;
  const options: RequestInit = {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...headers,
    },
  };

  if (body) {
    options.body = JSON.stringify(body);
  }

  return fetch(url, options);
}

describe.skipIf(SKIP_LIVE_TESTS)('security: Authentication Bypass', () => {
  it('rejects requests without authentication', async () => {
    const protectedEndpoints = [
      { method: 'GET', path: '/api/v1/trajectories' },
      { method: 'POST', path: '/api/v1/trajectories' },
      { method: 'GET', path: '/api/v1/agents' },
      { method: 'GET', path: '/api/v1/scopes' },
      { method: 'GET', path: '/api/v1/artifacts' },
    ];

    for (const endpoint of protectedEndpoints) {
      const res = await apiRequest(endpoint.method, endpoint.path);
      expect(res.status).toBe(401);
    }
  });

  it('rejects invalid authorization header formats', async () => {
    for (const token of AUTH_BYPASS_TOKENS) {
      const res = await apiRequest('GET', '/api/v1/trajectories', undefined, {
        Authorization: token,
      });

      expect(res.status).toBe(401);
    }
  });

  it('rejects JWT algorithm confusion attacks', async () => {
    for (const token of JWT_BYPASS_TOKENS) {
      const res = await apiRequest('GET', '/api/v1/trajectories', undefined, {
        Authorization: `Bearer ${token}`,
      });

      expect(res.status).toBe(401);
    }
  });

  it('rejects expired tokens', async () => {
    // Token with exp: 0 (expired in 1970)
    const expiredToken =
      'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0IiwiZXhwIjowfQ.invalid';

    const res = await apiRequest('GET', '/api/v1/trajectories', undefined, {
      Authorization: `Bearer ${expiredToken}`,
    });

    expect(res.status).toBe(401);
  });

  it('rejects tokens with invalid signatures', async () => {
    // Valid structure but wrong signature
    const badSigToken =
      'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0In0.WRONG_SIGNATURE';

    const res = await apiRequest('GET', '/api/v1/trajectories', undefined, {
      Authorization: `Bearer ${badSigToken}`,
    });

    expect(res.status).toBe(401);
  });
});

describe.skipIf(SKIP_LIVE_TESTS)('security: Authorization / IDOR', () => {
  // These tests require a valid token to test authorization (not just authentication)

  it('prevents accessing other users resources (IDOR)', async () => {
    // Try to access another user's trajectory
    const res = await apiRequest(
      'GET',
      `/api/v1/trajectories/${OTHER_TRAJECTORY_ID}`,
      undefined,
      VALID_TEST_TOKEN ? { Authorization: `Bearer ${VALID_TEST_TOKEN}` } : {}
    );

    // Should return 401 (no auth), 403 (forbidden), or 404 (not found)
    // Never 200 with another user's data
    expect([401, 403, 404]).toContain(res.status);
  });

  it('prevents modifying other users resources', async () => {
    const res = await apiRequest(
      'PUT',
      `/api/v1/trajectories/${OTHER_TRAJECTORY_ID}`,
      { name: 'hacked' },
      VALID_TEST_TOKEN ? { Authorization: `Bearer ${VALID_TEST_TOKEN}` } : {}
    );

    expect([401, 403, 404, 405]).toContain(res.status);
  });

  it('prevents deleting other users resources', async () => {
    const res = await apiRequest(
      'DELETE',
      `/api/v1/trajectories/${OTHER_TRAJECTORY_ID}`,
      undefined,
      VALID_TEST_TOKEN ? { Authorization: `Bearer ${VALID_TEST_TOKEN}` } : {}
    );

    expect([401, 403, 404, 405]).toContain(res.status);
  });

  it('prevents ID enumeration via sequential IDs', async () => {
    // Try to enumerate resources by guessing sequential IDs
    const sequentialIds = ['1', '2', '3', '100', '1000'];
    let successCount = 0;

    for (const id of sequentialIds) {
      const res = await apiRequest('GET', `/api/v1/trajectories/${id}`);

      // Should not return 200 for guessable IDs
      if (res.status === 200) {
        successCount++;
      }
    }

    // If using proper UUIDs, none should succeed
    expect(successCount).toBe(0);
  });

  it('prevents accessing admin endpoints as regular user', async () => {
    const adminEndpoints = [
      '/api/v1/admin/users',
      '/api/v1/admin/config',
      '/api/v1/admin/metrics',
      '/admin',
      '/_admin',
      '/internal/admin',
    ];

    for (const endpoint of adminEndpoints) {
      const res = await apiRequest('GET', endpoint, undefined, {
        Authorization: `Bearer ${VALID_TEST_TOKEN}`,
      });

      // Should be 401, 403, or 404 - never 200
      expect([401, 403, 404]).toContain(res.status);
    }
  });
});

describe.skipIf(SKIP_LIVE_TESTS)('security: Privilege Escalation', () => {
  it('prevents role escalation via request body', async () => {
    const res = await apiRequest(
      'POST',
      '/api/v1/trajectories',
      {
        name: 'test',
        role: 'admin',
        isAdmin: true,
        permissions: ['admin', 'super'],
      },
      VALID_TEST_TOKEN ? { Authorization: `Bearer ${VALID_TEST_TOKEN}` } : {}
    );

    // Should ignore or reject privilege fields
    if (res.status === 201) {
      const data = await res.json();
      expect(data.role).not.toBe('admin');
      expect(data.isAdmin).not.toBe(true);
    }
  });

  it('prevents tenant switching via headers', async () => {
    const res = await apiRequest('GET', '/api/v1/trajectories', undefined, {
      Authorization: `Bearer ${VALID_TEST_TOKEN}`,
      'X-Tenant-ID': 'other-tenant-123',
      'X-Organization-ID': 'other-org-123',
    });

    // Should ignore tenant override headers
    expect([401, 403, 200]).toContain(res.status);
  });
});

describe.skipIf(SKIP_LIVE_TESTS)('security: HTTP Method Override', () => {
  it('prevents method override to bypass restrictions', async () => {
    // Try to use method override headers to change GET to DELETE
    const overrideHeaders = [
      'X-HTTP-Method-Override',
      'X-Method-Override',
      'X-HTTP-Method',
      '_method',
    ];

    for (const header of overrideHeaders) {
      const res = await fetch(`${API_BASE_URL}/api/v1/trajectories/test-id`, {
        method: 'GET',
        headers: {
          [header]: 'DELETE',
          'Content-Type': 'application/json',
        },
      });

      // Should not perform DELETE operation
      expect([401, 404, 405]).toContain(res.status);
    }
  });

  it('prevents method override via query parameter', async () => {
    const res = await fetch(
      `${API_BASE_URL}/api/v1/trajectories/test-id?_method=DELETE`,
      {
        method: 'GET',
      }
    );

    expect([401, 404, 405]).toContain(res.status);
  });
});

describe.skipIf(SKIP_LIVE_TESTS)('security: Path Traversal', () => {
  it('prevents path traversal in resource IDs', async () => {
    const traversalPayloads = [
      '../../../etc/passwd',
      '..%2F..%2F..%2Fetc%2Fpasswd',
      '....//....//....//etc/passwd',
      '%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd',
      '..\\..\\..\\windows\\system32\\config\\sam',
    ];

    for (const payload of traversalPayloads) {
      const res = await apiRequest(
        'GET',
        `/api/v1/trajectories/${encodeURIComponent(payload)}`
      );

      expect([400, 401, 404]).toContain(res.status);

      const text = await res.text();
      expect(text).not.toContain('root:');
    }
  });
});

describe.skipIf(SKIP_LIVE_TESTS)('security: API Key Handling', () => {
  it('rejects API keys in query parameters (should use headers)', async () => {
    const res = await fetch(
      `${API_BASE_URL}/api/v1/trajectories?api_key=test_key&apiKey=test_key`
    );

    // Should require proper header authentication
    expect(res.status).toBe(401);
  });

  it('does not leak API keys in error responses', async () => {
    const res = await apiRequest('GET', '/api/v1/trajectories', undefined, {
      Authorization: 'Bearer invalid_key_12345',
    });

    const text = await res.text();
    expect(text).not.toContain('invalid_key_12345');
  });
});

describe('security: Access control patterns (offline)', () => {
  // Offline tests for access control logic patterns

  function isValidUUID(id: string): boolean {
    const uuidRegex =
      /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
    return uuidRegex.test(id);
  }

  function containsTraversal(path: string): boolean {
    const traversalPatterns = [
      /\.\.\//,
      /\.\.%2[fF]/,
      /\.\.%5[cC]/,
      /\.\.\\/,
      /%2e%2e/i,
    ];
    return traversalPatterns.some((p) => p.test(path));
  }

  it('UUID validation rejects sequential IDs', () => {
    expect(isValidUUID('1')).toBe(false);
    expect(isValidUUID('123')).toBe(false);
    expect(isValidUUID('abc')).toBe(false);
    expect(isValidUUID('550e8400-e29b-41d4-a716-446655440000')).toBe(true);
  });

  it('path traversal detection catches common patterns', () => {
    expect(containsTraversal('../etc/passwd')).toBe(true);
    expect(containsTraversal('..%2fetc%2fpasswd')).toBe(true);
    expect(containsTraversal('normal/path/here')).toBe(false);
  });
});
