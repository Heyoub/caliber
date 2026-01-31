/**
 * Dashboard Stats API Endpoint
 *
 * Aggregates data from caliber-api backend to provide dashboard statistics.
 * This is a proxy/aggregation endpoint that fetches from:
 * - GET /api/v1/trajectories
 * - GET /api/v1/agents
 *
 * If the backend is unavailable, returns mock data for development.
 */
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import type { DashboardStats, ActivityItem, ApiAgentResponse, HealthResponse } from '$api/types';

/** Backend API base URL */
const BACKEND_URL = import.meta.env.VITE_BACKEND_URL || 'http://localhost:3000';

/** Request timeout in milliseconds */
const REQUEST_TIMEOUT = 10000;

interface BackendTrajectory {
  id: string;
  name: string;
  status?: string;
  created_at?: string;
  updated_at?: string;
  scope_count?: number;
  event_count?: number;
}

interface BackendAgent {
  agent_id: string;
  tenant_id: string;
  agent_type: string;
  capabilities: string[];
  memory_access: {
    read: Array<{ memory_type: string; scope: string; filter?: Record<string, unknown> }>;
    write: Array<{ memory_type: string; scope: string; filter?: Record<string, unknown> }>;
  };
  can_delegate_to: string[];
  reports_to?: string;
  status: 'idle' | 'active' | 'blocked' | 'failed' | 'offline';
  current_trajectory_id?: string;
  current_scope_id?: string;
  last_heartbeat: string;
  created_at: string;
  updated_at: string;
}

interface BackendListResponse<T> {
  data: T[];
  meta?: {
    total?: number;
    page?: number;
    per_page?: number;
  };
}

/**
 * Fetch from backend with timeout and auth forwarding.
 */
async function fetchBackend<T>(
  endpoint: string,
  request: Request
): Promise<T | null> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), REQUEST_TIMEOUT);

  try {
    // Forward authorization header from original request
    const authHeader = request.headers.get('Authorization');
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
    };
    if (authHeader) {
      headers['Authorization'] = authHeader;
    }

    const response = await fetch(`${BACKEND_URL}${endpoint}`, {
      method: 'GET',
      headers,
      signal: controller.signal,
    });

    if (!response.ok) {
      console.error(`Backend request failed: ${endpoint} - ${response.status}`);
      return null;
    }

    return await response.json();
  } catch (err) {
    if (err instanceof Error && err.name === 'AbortError') {
      console.error(`Backend request timed out: ${endpoint}`);
    } else {
      console.error(`Backend request error: ${endpoint}`, err);
    }
    return null;
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Generate mock agents for development/fallback.
 */
function generateMockAgents(): ApiAgentResponse[] {
  const now = new Date();
  return [
    {
      agent_id: 'agent-mock-1',
      tenant_id: 'tenant-mock',
      agent_type: 'code-assistant',
      capabilities: ['code_review', 'refactoring', 'documentation'],
      memory_access: {
        read: [{ memory_type: 'code', scope: 'project' }],
        write: [{ memory_type: 'notes', scope: 'project' }],
      },
      can_delegate_to: [],
      status: 'active',
      last_heartbeat: now.toISOString(),
      created_at: new Date(now.getTime() - 1000 * 60 * 60 * 24).toISOString(),
      updated_at: now.toISOString(),
    },
    {
      agent_id: 'agent-mock-2',
      tenant_id: 'tenant-mock',
      agent_type: 'research-assistant',
      capabilities: ['web_search', 'summarization', 'citation'],
      memory_access: {
        read: [{ memory_type: 'documents', scope: 'workspace' }],
        write: [{ memory_type: 'research', scope: 'workspace' }],
      },
      can_delegate_to: ['agent-mock-1'],
      status: 'idle',
      last_heartbeat: new Date(now.getTime() - 1000 * 60 * 5).toISOString(),
      created_at: new Date(now.getTime() - 1000 * 60 * 60 * 48).toISOString(),
      updated_at: new Date(now.getTime() - 1000 * 60 * 30).toISOString(),
    },
  ];
}

/**
 * Generate mock dashboard stats for development/fallback.
 */
function generateMockStats(): DashboardStats {
  const now = new Date();

  const recentActivity: ActivityItem[] = [
    {
      id: 'activity-1',
      type: 'trajectory',
      name: 'Feature Development Session',
      action: 'created',
      timestamp: new Date(now.getTime() - 1000 * 60 * 5).toISOString(), // 5 min ago
    },
    {
      id: 'activity-2',
      type: 'scope',
      name: 'Code Review Context',
      action: 'created',
      timestamp: new Date(now.getTime() - 1000 * 60 * 15).toISOString(), // 15 min ago
    },
    {
      id: 'activity-3',
      type: 'trajectory',
      name: 'Bug Investigation',
      action: 'updated',
      timestamp: new Date(now.getTime() - 1000 * 60 * 30).toISOString(), // 30 min ago
    },
    {
      id: 'activity-4',
      type: 'event',
      name: 'Agent completed analysis',
      action: 'created',
      timestamp: new Date(now.getTime() - 1000 * 60 * 45).toISOString(), // 45 min ago
    },
    {
      id: 'activity-5',
      type: 'scope',
      name: 'Planning Phase',
      action: 'updated',
      timestamp: new Date(now.getTime() - 1000 * 60 * 60).toISOString(), // 1 hour ago
    },
  ];

  return {
    trajectoryCount: 12,
    scopeCount: 47,
    eventCount: 1284,
    storageUsedBytes: 15728640, // ~15 MB
    recentActivity,
    apiHealth: {
      status: 'healthy',
      message: 'Mock data - backend unavailable',
    },
    agents: generateMockAgents(),
  };
}

/**
 * Convert backend agent to ApiAgentResponse format.
 */
function mapBackendAgent(agent: BackendAgent): ApiAgentResponse {
  return {
    agent_id: agent.agent_id,
    tenant_id: agent.tenant_id,
    agent_type: agent.agent_type,
    capabilities: agent.capabilities,
    memory_access: agent.memory_access,
    can_delegate_to: agent.can_delegate_to,
    reports_to: agent.reports_to,
    status: agent.status,
    current_trajectory_id: agent.current_trajectory_id,
    current_scope_id: agent.current_scope_id,
    last_heartbeat: agent.last_heartbeat,
    created_at: agent.created_at,
    updated_at: agent.updated_at,
  };
}

/**
 * Convert backend trajectories and agents to recent activity items.
 */
function buildRecentActivity(
  trajectories: BackendTrajectory[],
  agents: BackendAgent[]
): ActivityItem[] {
  const items: ActivityItem[] = [];

  // Add recent trajectories
  for (const traj of trajectories.slice(0, 5)) {
    items.push({
      id: `traj-${traj.id}`,
      type: 'trajectory',
      name: traj.name || `Trajectory ${traj.id.slice(0, 8)}`,
      action: 'created',
      timestamp: traj.created_at || new Date().toISOString(),
    });
  }

  // Add recent agents as activity
  for (const agent of agents.slice(0, 3)) {
    items.push({
      id: `agent-${agent.agent_id}`,
      type: 'scope', // Map agent activity to scope type for UI
      name: `${agent.agent_type} Agent`,
      action: 'created',
      timestamp: agent.created_at || new Date().toISOString(),
    });
  }

  // Sort by timestamp descending and take top 10
  return items
    .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
    .slice(0, 10);
}

/**
 * GET /api/dashboard/stats
 *
 * Returns aggregated dashboard statistics including:
 * - Trajectory count
 * - Scope count (estimated from trajectories)
 * - Event count (estimated)
 * - Storage used (estimated)
 * - Recent activity
 * - API health status
 * - Registered agents
 */
export const GET: RequestHandler = async ({ request }) => {
  try {
    // Fetch data from backend in parallel
    const [trajectoriesResponse, agentsResponse, healthResponse] = await Promise.all([
      fetchBackend<BackendListResponse<BackendTrajectory>>('/api/v1/trajectories?limit=100', request),
      fetchBackend<BackendListResponse<BackendAgent>>('/api/v1/agents?limit=50', request),
      fetchBackend<HealthResponse>('/health/ready', request),
    ]);

    // If backend is unavailable, return mock data
    if (!trajectoriesResponse && !agentsResponse) {
      console.log('Backend unavailable, returning mock stats');
      return json({
        data: generateMockStats(),
      });
    }

    const trajectories = trajectoriesResponse?.data || [];
    const agents = agentsResponse?.data || [];

    // Calculate stats from backend data
    const trajectoryCount = trajectoriesResponse?.meta?.total ?? trajectories.length;

    // Sum up scope counts from trajectories, or estimate
    const scopeCount = trajectories.reduce((sum, t) => sum + (t.scope_count ?? 1), 0);

    // Sum up event counts from trajectories, or estimate
    const eventCount = trajectories.reduce((sum, t) => sum + (t.event_count ?? 10), 0);

    // Estimate storage: ~1KB per event (rough estimate)
    const storageUsedBytes = eventCount * 1024;

    // Build recent activity from trajectories and agents
    const recentActivity = buildRecentActivity(trajectories, agents);

    // Map agents to API response format
    const mappedAgents: ApiAgentResponse[] = agents.map(mapBackendAgent);

    // Build health status
    const apiHealth: HealthResponse = healthResponse || {
      status: trajectoriesResponse ? 'healthy' : 'degraded',
      message: trajectoriesResponse ? 'Backend connected' : 'Partial data available',
    };

    const stats: DashboardStats = {
      trajectoryCount,
      scopeCount,
      eventCount,
      storageUsedBytes,
      recentActivity,
      apiHealth,
      agents: mappedAgents,
    };

    return json({ data: stats });
  } catch (err) {
    console.error('Failed to fetch dashboard stats:', err);

    // Return mock data on error for better UX
    return json({
      data: generateMockStats(),
    });
  }
};
