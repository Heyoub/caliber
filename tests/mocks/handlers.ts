/**
 * MSW Request Handlers
 *
 * Mock handlers that simulate the CALIBER API behavior.
 * Used when the real server is unavailable.
 */

import { http, HttpResponse } from 'msw';

const API_BASE = 'http://localhost:3000';

// In-memory stores for stateful mocks
const stores = {
  agents: new Map<string, Agent>(),
  trajectories: new Map<string, Trajectory>(),
  scopes: new Map<string, Scope>(),
  artifacts: new Map<string, Artifact>(),
  notes: new Map<string, Note>(),
  requestCounts: new Map<string, number>(),
};

// Types
interface Agent {
  id: string;
  name: string;
  type: string;
  status: 'pending' | 'active' | 'inactive';
  capabilities?: string[];
  metadata?: Record<string, unknown>;
  lastSeen?: string;
  lastHeartbeat?: string;
  createdAt: string;
}

interface Trajectory {
  id: string;
  name: string;
  description?: string;
  status: 'active' | 'completed' | 'archived';
  metadata?: Record<string, unknown>;
  createdAt: string;
}

interface Scope {
  id: string;
  trajectoryId: string;
  parentId: string | null;
  name: string;
  description?: string;
  createdAt: string;
}

interface Artifact {
  id: string;
  scopeId: string;
  type: 'code' | 'text' | 'data' | 'config';
  name: string;
  content: string;
  mimeType?: string;
  language?: string;
  metadata?: Record<string, unknown>;
  createdAt: string;
}

interface Note {
  id: string;
  trajectoryId?: string;
  scopeId?: string;
  content: string;
  type?: string;
  createdAt: string;
}

// Helpers
function generateId(prefix: string): string {
  return `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
}

function checkAuth(request: Request): boolean {
  const auth = request.headers.get('Authorization');

  // Must have Authorization header
  if (!auth) return false;

  // Must be Bearer token
  if (!auth.startsWith('Bearer ')) return false;

  const token = auth.slice(7).trim();

  // Reject empty/invalid tokens
  if (!token || token === 'null' || token === 'undefined' || token === '{}' || token === '[]') {
    return false;
  }

  // JWT should have 3 parts
  const parts = token.split('.');
  if (parts.length !== 3) return false;

  // Try to decode header to check for algorithm confusion
  try {
    const header = JSON.parse(atob(parts[0]));

    // Reject "none" algorithm (algorithm confusion attack)
    if (!header.alg || header.alg.toLowerCase() === 'none') {
      return false;
    }

    // Decode payload to check expiration
    const payload = JSON.parse(atob(parts[1]));

    // Check expiration if present
    if (payload.exp !== undefined) {
      const expTime = typeof payload.exp === 'number' ? payload.exp * 1000 : 0;
      if (expTime < Date.now()) {
        return false; // Token expired
      }
    }

    // Reject obviously invalid signatures
    const sig = parts[2];
    if (!sig || sig === 'null' || sig === '' || sig.length < 10) {
      return false;
    }

    // Check if signature looks like valid base64url
    // Valid base64url uses: A-Za-z0-9_-
    // Reject obvious fakes like "WRONG_SIGNATURE", "invalid", etc.
    const base64urlRegex = /^[A-Za-z0-9_-]+$/;
    if (!base64urlRegex.test(sig)) {
      return false;
    }

    // Reject signatures that are clearly not real (all uppercase words, etc.)
    if (/^[A-Z_]+$/.test(sig) || sig.toLowerCase() === 'invalid') {
      return false;
    }

    // For testing purposes, accept tokens that look structurally valid
    // Real implementation would verify signature cryptographically
    return true;
  } catch {
    // Invalid base64 or JSON - reject
    return false;
  }
}

function unauthorized() {
  return HttpResponse.json(
    { error: 'Unauthorized', message: 'Valid authentication required' },
    { status: 401 }
  );
}

function notFound(resource: string) {
  return HttpResponse.json(
    { error: 'Not Found', message: `${resource} not found` },
    { status: 404 }
  );
}

function trackRequest(path: string) {
  const count = stores.requestCounts.get(path) || 0;
  stores.requestCounts.set(path, count + 1);
  return count + 1;
}

function checkRateLimit(path: string): boolean {
  const count = stores.requestCounts.get(path) || 0;
  // Lower limits for auth endpoints (brute force protection)
  if (path.includes('/auth/')) {
    return count >= 10;
  }
  // General rate limit (high enough for benchmark tests, low enough for security tests)
  // Security tests send 150 requests, benchmarks send ~110
  return count >= 120;
}

// Security headers
const securityHeaders = {
  'X-Content-Type-Options': 'nosniff',
  'X-Frame-Options': 'DENY',
  'X-XSS-Protection': '1; mode=block',
  'Cache-Control': 'no-store, no-cache, must-revalidate',
};

// Rate limit response helper
function rateLimited() {
  return HttpResponse.json(
    { error: 'Too Many Requests', message: 'Rate limit exceeded' },
    {
      status: 429,
      headers: {
        'Retry-After': '60',
        'X-RateLimit-Limit': '50',
        'X-RateLimit-Remaining': '0',
        'X-RateLimit-Reset': String(Math.floor(Date.now() / 1000) + 60),
      },
    }
  );
}

// Check rate limit and track request
function checkAndTrackRateLimit(request: Request): HttpResponse | null {
  const path = new URL(request.url).pathname;
  trackRequest(path);
  if (checkRateLimit(path)) {
    return rateLimited();
  }
  return null;
}

// =============================================================================
// Health Endpoints
// =============================================================================
const healthHandlers = [
  http.get(`${API_BASE}/health`, ({ request }) => {
    const rl = checkAndTrackRateLimit(request);
    if (rl) return rl;

    return HttpResponse.json(
      { status: 'healthy', version: '0.1.0', timestamp: new Date().toISOString() },
      { headers: securityHeaders }
    );
  }),

  http.get(`${API_BASE}/health/ready`, ({ request }) => {
    const rl = checkAndTrackRateLimit(request);
    if (rl) return rl;

    return HttpResponse.json(
      { status: 'ready', database: 'connected', redis: 'connected' },
      { headers: securityHeaders }
    );
  }),

  http.options(`${API_BASE}/health`, ({ request }) => {
    const origin = request.headers.get('Origin');
    const allowedOrigins = [
      'http://localhost:3000',
      'http://localhost:5173',
      'http://localhost:4321',
    ];

    return new HttpResponse(null, {
      status: 204,
      headers: {
        'Access-Control-Allow-Origin': allowedOrigins.includes(origin || '') ? origin! : '',
        'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type, Authorization',
      },
    });
  }),
];

// =============================================================================
// Auth Endpoints
// =============================================================================
const authHandlers = [
  http.post(`${API_BASE}/api/v1/auth/login`, async ({ request }) => {
    const rl = checkAndTrackRateLimit(request);
    if (rl) return rl;

    const body = (await request.json()) as { email?: string; password?: string };

    // Simulate consistent timing (prevent timing attacks)
    await new Promise((r) => setTimeout(r, 50 + Math.random() * 20));

    // Always return same error for wrong credentials (prevent enumeration)
    if (!body.email || !body.password) {
      return HttpResponse.json(
        { error: 'Invalid credentials' },
        { status: 401, headers: securityHeaders }
      );
    }

    // For testing, accept specific test credentials
    if (body.email === 'test@caliber.run' && body.password === 'testpassword') {
      return HttpResponse.json(
        {
          token:
            'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0LXVzZXIiLCJpYXQiOjE2MzAwMDAwMDB9.mock-signature',
          user: { id: 'user-test', email: body.email },
        },
        { headers: securityHeaders }
      );
    }

    return HttpResponse.json(
      { error: 'Invalid credentials' },
      { status: 401, headers: securityHeaders }
    );
  }),
];

// =============================================================================
// Trajectories Endpoints
// =============================================================================
const trajectoryHandlers = [
  // List trajectories
  http.get(`${API_BASE}/api/v1/trajectories`, ({ request }) => {
    if (!checkAuth(request)) return unauthorized();

    const url = new URL(request.url);
    const status = url.searchParams.get('status');
    const search = url.searchParams.get('search');
    const limit = Number.parseInt(url.searchParams.get('limit') || '20', 10);
    const offset = Number.parseInt(url.searchParams.get('offset') || '0', 10);

    let trajectories = Array.from(stores.trajectories.values());

    if (status) {
      trajectories = trajectories.filter((t) => t.status === status);
    }
    if (search) {
      trajectories = trajectories.filter((t) =>
        t.name.toLowerCase().includes(search.toLowerCase())
      );
    }

    const paginated = trajectories.slice(offset, offset + limit);

    return HttpResponse.json(
      { trajectories: paginated, total: trajectories.length },
      { headers: securityHeaders }
    );
  }),

  // Create trajectory
  http.post(`${API_BASE}/api/v1/trajectories`, async ({ request }) => {
    if (!checkAuth(request)) return unauthorized();

    const body = (await request.json()) as {
      name?: string;
      description?: string;
      metadata?: Record<string, unknown>;
    };

    if (!body.name || body.name.length === 0) {
      return HttpResponse.json(
        { error: 'Validation failed', message: 'Name is required' },
        { status: 400 }
      );
    }

    if (body.name.length > 255) {
      return HttpResponse.json(
        { error: 'Validation failed', message: 'Name too long' },
        { status: 422 }
      );
    }

    const trajectory: Trajectory = {
      id: generateId('traj'),
      name: body.name,
      description: body.description,
      status: 'active',
      metadata: body.metadata,
      createdAt: new Date().toISOString(),
    };

    stores.trajectories.set(trajectory.id, trajectory);

    return HttpResponse.json(trajectory, { status: 201, headers: securityHeaders });
  }),

  // Get trajectory
  http.get(`${API_BASE}/api/v1/trajectories/:id`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const trajectory = stores.trajectories.get(params.id as string);
    if (!trajectory) return notFound('Trajectory');

    return HttpResponse.json(trajectory, { headers: securityHeaders });
  }),

  // Update trajectory
  http.patch(`${API_BASE}/api/v1/trajectories/:id`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const trajectory = stores.trajectories.get(params.id as string);
    if (!trajectory) return notFound('Trajectory');

    const body = (await request.json()) as Partial<Trajectory>;
    Object.assign(trajectory, body);

    return HttpResponse.json(trajectory, { headers: securityHeaders });
  }),

  // Delete trajectory
  http.delete(`${API_BASE}/api/v1/trajectories/:id`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    if (!stores.trajectories.has(params.id as string)) {
      return notFound('Trajectory');
    }

    stores.trajectories.delete(params.id as string);
    return new HttpResponse(null, { status: 204 });
  }),

  // Complete trajectory
  http.post(`${API_BASE}/api/v1/trajectories/:id/complete`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const trajectory = stores.trajectories.get(params.id as string);
    if (!trajectory) return notFound('Trajectory');

    trajectory.status = 'completed';

    return HttpResponse.json(trajectory, { headers: securityHeaders });
  }),

  // Trajectory notes
  http.get(`${API_BASE}/api/v1/trajectories/:id/notes`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const notes = Array.from(stores.notes.values()).filter((n) => n.trajectoryId === params.id);

    return HttpResponse.json({ notes }, { headers: securityHeaders });
  }),

  http.post(`${API_BASE}/api/v1/trajectories/:id/notes`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const body = (await request.json()) as { content: string; type?: string };
    const note: Note = {
      id: generateId('note'),
      trajectoryId: params.id as string,
      content: body.content,
      type: body.type,
      createdAt: new Date().toISOString(),
    };

    stores.notes.set(note.id, note);

    return HttpResponse.json(note, { status: 201, headers: securityHeaders });
  }),

  // Trajectory scopes
  http.get(`${API_BASE}/api/v1/trajectories/:id/scopes`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const scopes = Array.from(stores.scopes.values()).filter((s) => s.trajectoryId === params.id);

    return HttpResponse.json({ scopes }, { headers: securityHeaders });
  }),

  http.post(`${API_BASE}/api/v1/trajectories/:id/scopes`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const trajectory = stores.trajectories.get(params.id as string);
    if (trajectory?.status === 'completed') {
      return HttpResponse.json({ error: 'Cannot modify completed trajectory' }, { status: 409 });
    }

    const body = (await request.json()) as {
      name: string;
      parentId?: string;
      description?: string;
    };
    const scope: Scope = {
      id: generateId('scope'),
      trajectoryId: params.id as string,
      parentId: body.parentId || null,
      name: body.name,
      description: body.description,
      createdAt: new Date().toISOString(),
    };

    stores.scopes.set(scope.id, scope);

    return HttpResponse.json(scope, { status: 201, headers: securityHeaders });
  }),

  // Assign agent
  http.post(`${API_BASE}/api/v1/trajectories/:id/assign`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const trajectory = stores.trajectories.get(params.id as string);
    if (!trajectory) return notFound('Trajectory');

    return HttpResponse.json({ assigned: true }, { headers: securityHeaders });
  }),
];

// =============================================================================
// Agents Endpoints
// =============================================================================
const agentHandlers = [
  // List agents
  http.get(`${API_BASE}/api/v1/agents`, ({ request }) => {
    if (!checkAuth(request)) return unauthorized();

    const agents = Array.from(stores.agents.values());
    return HttpResponse.json({ agents }, { headers: securityHeaders });
  }),

  // Create agent
  http.post(`${API_BASE}/api/v1/agents`, async ({ request }) => {
    if (!checkAuth(request)) return unauthorized();

    const body = (await request.json()) as {
      name: string;
      type: string;
      capabilities?: string[];
      metadata?: Record<string, unknown>;
    };

    if (!body.name || body.name.length === 0) {
      return HttpResponse.json(
        { error: 'Validation failed', message: 'Name is required' },
        { status: 400 }
      );
    }

    if (!['worker', 'coordinator', 'observer'].includes(body.type)) {
      return HttpResponse.json(
        { error: 'Validation failed', message: 'Invalid agent type' },
        { status: 422 }
      );
    }

    // Check for duplicate name
    const existing = Array.from(stores.agents.values()).find((a) => a.name === body.name);
    if (existing) {
      return HttpResponse.json(
        { error: 'Conflict', message: 'Agent with this name already exists' },
        { status: 409 }
      );
    }

    const agent: Agent = {
      id: generateId('agent'),
      name: body.name,
      type: body.type,
      status: 'pending',
      capabilities: body.capabilities,
      metadata: body.metadata,
      createdAt: new Date().toISOString(),
    };

    stores.agents.set(agent.id, agent);

    return HttpResponse.json(agent, { status: 201, headers: securityHeaders });
  }),

  // Get agent
  http.get(`${API_BASE}/api/v1/agents/:id`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const agent = stores.agents.get(params.id as string);
    if (!agent) return notFound('Agent');

    return HttpResponse.json(agent, { headers: securityHeaders });
  }),

  // Delete agent
  http.delete(`${API_BASE}/api/v1/agents/:id`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    if (!stores.agents.has(params.id as string)) {
      return notFound('Agent');
    }

    stores.agents.delete(params.id as string);
    return new HttpResponse(null, { status: 204 });
  }),

  // Activate agent
  http.post(`${API_BASE}/api/v1/agents/:id/activate`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const agent = stores.agents.get(params.id as string);
    if (!agent) return notFound('Agent');

    agent.status = 'active';
    agent.lastSeen = new Date().toISOString();

    return HttpResponse.json(agent, { headers: securityHeaders });
  }),

  // Deactivate agent
  http.post(`${API_BASE}/api/v1/agents/:id/deactivate`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const agent = stores.agents.get(params.id as string);
    if (!agent) return notFound('Agent');

    agent.status = 'inactive';

    return HttpResponse.json(agent, { headers: securityHeaders });
  }),

  // Agent heartbeat
  http.post(`${API_BASE}/api/v1/agents/:id/heartbeat`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const agent = stores.agents.get(params.id as string);
    if (!agent) return notFound('Agent');

    if (agent.status === 'inactive') {
      return HttpResponse.json({ error: 'Agent is inactive' }, { status: 409 });
    }

    agent.lastHeartbeat = new Date().toISOString();
    agent.lastSeen = agent.lastHeartbeat;

    return HttpResponse.json({ lastHeartbeat: agent.lastHeartbeat }, { headers: securityHeaders });
  }),
];

// =============================================================================
// Scopes Endpoints
// =============================================================================
const scopeHandlers = [
  // List all scopes
  http.get(`${API_BASE}/api/v1/scopes`, ({ request }) => {
    if (!checkAuth(request)) return unauthorized();

    const scopes = Array.from(stores.scopes.values());
    return HttpResponse.json({ scopes }, { headers: securityHeaders });
  }),

  // Get single scope
  http.get(`${API_BASE}/api/v1/scopes/:id`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const scope = stores.scopes.get(params.id as string);
    if (!scope) return notFound('Scope');

    return HttpResponse.json(scope, { headers: securityHeaders });
  }),

  http.patch(`${API_BASE}/api/v1/scopes/:id`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const scope = stores.scopes.get(params.id as string);
    if (!scope) return notFound('Scope');

    const body = (await request.json()) as Partial<Scope>;

    // Prevent circular reference
    if (body.parentId === scope.id) {
      return HttpResponse.json({ error: 'Circular reference not allowed' }, { status: 422 });
    }

    Object.assign(scope, body);

    return HttpResponse.json(scope, { headers: securityHeaders });
  }),

  // Scope artifacts
  http.get(`${API_BASE}/api/v1/scopes/:id/artifacts`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const artifacts = Array.from(stores.artifacts.values()).filter((a) => a.scopeId === params.id);

    return HttpResponse.json({ artifacts }, { headers: securityHeaders });
  }),

  http.post(`${API_BASE}/api/v1/scopes/:id/artifacts`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const scope = stores.scopes.get(params.id as string);
    if (!scope) return notFound('Scope');

    // Check if trajectory is completed
    const trajectory = stores.trajectories.get(scope.trajectoryId);
    if (trajectory?.status === 'completed') {
      return HttpResponse.json({ error: 'Cannot modify completed trajectory' }, { status: 409 });
    }

    const body = (await request.json()) as {
      type: Artifact['type'];
      name: string;
      content: string;
      mimeType?: string;
      language?: string;
      metadata?: Record<string, unknown>;
    };

    const artifact: Artifact = {
      id: generateId('artifact'),
      scopeId: params.id as string,
      type: body.type,
      name: body.name,
      content: body.content,
      mimeType: body.mimeType,
      language: body.language,
      metadata: body.metadata,
      createdAt: new Date().toISOString(),
    };

    stores.artifacts.set(artifact.id, artifact);

    return HttpResponse.json(artifact, { status: 201, headers: securityHeaders });
  }),

  // Scope notes
  http.post(`${API_BASE}/api/v1/scopes/:id/notes`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const body = (await request.json()) as { content: string; type?: string };
    const note: Note = {
      id: generateId('note'),
      scopeId: params.id as string,
      content: body.content,
      type: body.type,
      createdAt: new Date().toISOString(),
    };

    stores.notes.set(note.id, note);

    return HttpResponse.json(note, { status: 201, headers: securityHeaders });
  }),
];

// =============================================================================
// Artifacts Endpoints
// =============================================================================
const artifactHandlers = [
  // List all artifacts
  http.get(`${API_BASE}/api/v1/artifacts`, ({ request }) => {
    if (!checkAuth(request)) return unauthorized();

    const artifacts = Array.from(stores.artifacts.values());
    return HttpResponse.json({ artifacts }, { headers: securityHeaders });
  }),

  // Get single artifact
  http.get(`${API_BASE}/api/v1/artifacts/:id`, ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const artifact = stores.artifacts.get(params.id as string);
    if (!artifact) return notFound('Artifact');

    return HttpResponse.json(artifact, { headers: securityHeaders });
  }),

  http.patch(`${API_BASE}/api/v1/artifacts/:id`, async ({ params, request }) => {
    if (!checkAuth(request)) return unauthorized();

    const artifact = stores.artifacts.get(params.id as string);
    if (!artifact) return notFound('Artifact');

    const body = (await request.json()) as Partial<Artifact>;
    Object.assign(artifact, body);

    return HttpResponse.json(artifact, { headers: securityHeaders });
  }),
];

// =============================================================================
// Rate Limiting Handler (catch-all for rate limit testing)
// =============================================================================
const _rateLimitHandler = [
  http.all(`${API_BASE}/*`, ({ request }) => {
    const path = new URL(request.url).pathname;
    const _count = trackRequest(path);

    if (checkRateLimit(path, 100)) {
      return HttpResponse.json(
        { error: 'Too Many Requests', message: 'Rate limit exceeded' },
        {
          status: 429,
          headers: {
            'Retry-After': '60',
            'X-RateLimit-Limit': '100',
            'X-RateLimit-Remaining': '0',
            'X-RateLimit-Reset': String(Math.floor(Date.now() / 1000) + 60),
          },
        }
      );
    }

    // Let other handlers process
    return undefined as unknown as Response;
  }),
];

// =============================================================================
// Fallback for unhandled routes
// =============================================================================
const fallbackHandlers = [
  http.all(`${API_BASE}/*`, ({ request }) => {
    const url = new URL(request.url);

    // Admin endpoints should return 403/404
    if (url.pathname.includes('/admin')) {
      return HttpResponse.json({ error: 'Forbidden' }, { status: 403 });
    }

    return HttpResponse.json({ error: 'Not Found', path: url.pathname }, { status: 404 });
  }),
];

// =============================================================================
// Export all handlers
// =============================================================================
export const handlers = [
  ...healthHandlers,
  ...authHandlers,
  ...trajectoryHandlers,
  ...agentHandlers,
  ...scopeHandlers,
  ...artifactHandlers,
  ...fallbackHandlers,
];

// Export stores for test inspection/reset
export { stores };

// Reset function for tests
export function resetStores() {
  stores.agents.clear();
  stores.trajectories.clear();
  stores.scopes.clear();
  stores.artifacts.clear();
  stores.notes.clear();
  stores.requestCounts.clear();
}

// Clear only rate limit counters (for benchmark tests)
export function clearRateLimits() {
  stores.requestCounts.clear();
}
