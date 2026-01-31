<script lang="ts">
  /**
   * Dashboard Page
   * Thin stats UI - pulls from real API endpoints at localhost:3000
   */
  import { DashboardLayout } from '$layouts';
  import { Card, Badge, Spinner, Icon } from '@caliber/ui';
  import { apiClient, type DashboardStats, type HealthResponse } from '$api/client';

  // Content strings
  const content = {
    title: 'Dashboard',
    subtitle: 'Overview of your AI agent memory usage',
    stats: {
      trajectories: 'Trajectories',
      agents: 'Active Agents',
      scopes: 'Active Scopes',
      events: 'Total Events',
      storage: 'Storage Used',
    },
    apiStatus: 'API Status',
    recentActivity: 'Recent Activity',
    quickActions: 'Quick Actions',
    actions: {
      newTrajectory: 'New Trajectory',
      viewScopes: 'View Scopes',
      openEditor: 'Open Editor',
    },
  };

  // State for dashboard data
  let stats = $state<DashboardStats | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Derived state for agent counts
  let activeAgentCount = $derived(
    stats?.agents?.filter((a) => a.status === 'active').length ?? 0
  );
  let totalAgentCount = $derived(stats?.agents?.length ?? 0);

  // Load stats on mount
  $effect(() => {
    loadStats();
  });

  async function loadStats() {
    try {
      loading = true;
      error = null;
      stats = await apiClient.getDashboardStats();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load stats';
      console.error('Dashboard load error:', e);
    } finally {
      loading = false;
    }
  }

  async function refreshStats() {
    await loadStats();
  }

  function formatNumber(num: number): string {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  }

  function formatBytes(bytes: number): string {
    if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(1)} GB`;
    if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(1)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${bytes} B`;
  }

  function formatUptime(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
    return `${Math.floor(seconds / 86400)}d`;
  }

  function getHealthColor(status: string | undefined): 'teal' | 'coral' | 'amber' {
    switch (status) {
      case 'healthy':
        return 'teal';
      case 'unhealthy':
        return 'coral';
      case 'degraded':
        return 'amber';
      default:
        return 'coral';
    }
  }

  function formatTimestamp(timestamp: string): string {
    try {
      const date = new Date(timestamp);
      const now = new Date();
      const diff = now.getTime() - date.getTime();
      const seconds = Math.floor(diff / 1000);
      const minutes = Math.floor(seconds / 60);
      const hours = Math.floor(minutes / 60);
      const days = Math.floor(hours / 24);

      if (days > 0) return `${days}d ago`;
      if (hours > 0) return `${hours}h ago`;
      if (minutes > 0) return `${minutes}m ago`;
      return 'just now';
    } catch {
      return timestamp;
    }
  }
</script>

<DashboardLayout>
  <div class="dashboard">
    <!-- Header -->
    <header class="dashboard-header">
      <div class="header-content">
        <h1 class="dashboard-title">{content.title}</h1>
        <p class="dashboard-subtitle">{content.subtitle}</p>
      </div>
      <button class="refresh-btn" onclick={refreshStats} disabled={loading}>
        <Icon name="refresh-cw" size="sm" />
        Refresh
      </button>
    </header>

    <!-- Stats Grid -->
    {#if loading}
      <div class="loading-container">
        <Spinner size="lg" color="teal" />
        <p class="loading-text">Connecting to API...</p>
      </div>
    {:else if error}
      <Card color="coral" glass="medium" border="subtle">
        <div class="error-message">
          <Icon name="alert-circle" color="coral" />
          <div class="error-content">
            <span class="error-title">Connection Error</span>
            <span class="error-detail">{error}</span>
            <button class="retry-btn" onclick={refreshStats}>
              Try Again
            </button>
          </div>
        </div>
      </Card>
    {:else if stats}
      <!-- API Health Status -->
      <section class="api-status">
        <h2 class="section-title">{content.apiStatus}</h2>
        <Card glass="medium" border="subtle">
          <div class="health-card">
            <div class="health-indicator {stats.apiHealth?.status ?? 'unhealthy'}">
              <Icon name={stats.apiHealth?.status === 'healthy' ? 'check-circle' : 'alert-circle'} size="lg" />
            </div>
            <div class="health-info">
              <span class="health-status">
                {#if stats.apiHealth}
                  <Badge color={getHealthColor(stats.apiHealth.status)} size="sm">
                    {stats.apiHealth.status}
                  </Badge>
                {:else}
                  <Badge color="coral" size="sm">offline</Badge>
                {/if}
              </span>
              {#if stats.apiHealth?.details}
                <span class="health-details">
                  v{stats.apiHealth.details.version} |
                  uptime: {formatUptime(stats.apiHealth.details.uptime_seconds)} |
                  db: {stats.apiHealth.details.database.latency_ms ?? 0}ms
                </span>
              {/if}
            </div>
          </div>
        </Card>
      </section>

      <!-- Stats Grid -->
      <div class="stats-grid">
        <Card glass="medium" border="subtle" hover="lift">
          <div class="stat-card">
            <div class="stat-icon teal">
              <Icon name="git-branch" size="lg" />
            </div>
            <div class="stat-content">
              <span class="stat-value">{formatNumber(stats.trajectoryCount)}</span>
              <span class="stat-label">{content.stats.trajectories}</span>
            </div>
          </div>
        </Card>

        <Card glass="medium" border="subtle" hover="lift">
          <div class="stat-card">
            <div class="stat-icon purple">
              <Icon name="users" size="lg" />
            </div>
            <div class="stat-content">
              <span class="stat-value">{activeAgentCount}/{totalAgentCount}</span>
              <span class="stat-label">{content.stats.agents}</span>
            </div>
          </div>
        </Card>

        <Card glass="medium" border="subtle" hover="lift">
          <div class="stat-card">
            <div class="stat-icon pink">
              <Icon name="layers" size="lg" />
            </div>
            <div class="stat-content">
              <span class="stat-value">{formatNumber(stats.scopeCount)}</span>
              <span class="stat-label">{content.stats.scopes}</span>
            </div>
          </div>
        </Card>

        <Card glass="medium" border="subtle" hover="lift">
          <div class="stat-card">
            <div class="stat-icon amber">
              <Icon name="activity" size="lg" />
            </div>
            <div class="stat-content">
              <span class="stat-value">{formatNumber(stats.eventCount)}</span>
              <span class="stat-label">{content.stats.events}</span>
            </div>
          </div>
        </Card>
      </div>

      <!-- Agents List -->
      {#if stats.agents && stats.agents.length > 0}
        <section class="agents-section">
          <h2 class="section-title">Registered Agents</h2>
          <div class="agents-list">
            {#each stats.agents as agent}
              <Card glass="light" border="subtle">
                <div class="agent-item">
                  <div class="agent-info">
                    <span class="agent-type">{agent.agent_type}</span>
                    <Badge
                      color={agent.status === 'active' ? 'teal' : agent.status === 'idle' ? 'purple' : 'amber'}
                      size="sm"
                    >
                      {agent.status}
                    </Badge>
                  </div>
                  <div class="agent-details">
                    <span class="agent-capabilities">
                      {agent.capabilities.slice(0, 3).join(', ')}{agent.capabilities.length > 3 ? '...' : ''}
                    </span>
                    <span class="agent-heartbeat">
                      Last seen: {formatTimestamp(agent.last_heartbeat)}
                    </span>
                  </div>
                </div>
              </Card>
            {/each}
          </div>
        </section>
      {/if}

      <!-- Quick Actions -->
      <section class="quick-actions">
        <h2 class="section-title">{content.quickActions}</h2>
        <div class="actions-grid">
          <a href="/editor/assistant" class="action-card">
            <Icon name="message-square" color="purple" size="lg" />
            <span>{content.actions.openEditor}</span>
          </a>
          <a href="/dashboard/trajectories" class="action-card">
            <Icon name="git-branch" color="teal" size="lg" />
            <span>{content.actions.viewScopes}</span>
          </a>
        </div>
      </section>

      <!-- Recent Activity -->
      {#if stats.recentActivity && stats.recentActivity.length > 0}
        <section class="recent-activity">
          <h2 class="section-title">{content.recentActivity}</h2>
          <div class="activity-list">
            {#each stats.recentActivity as activity}
              <div class="activity-item">
                <Badge color={activity.type === 'trajectory' ? 'teal' : 'purple'} size="sm">
                  {activity.type}
                </Badge>
                <span class="activity-name">{activity.name}</span>
                <span class="activity-time">{formatTimestamp(activity.timestamp)}</span>
              </div>
            {/each}
          </div>
        </section>
      {/if}
    {/if}
  </div>
</DashboardLayout>

<style>
  .dashboard {
    max-width: 1200px;
    margin: 0 auto;
  }

  .dashboard-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--space-8);
  }

  .header-content {
    flex: 1;
  }

  .dashboard-title {
    font-family: var(--font-display);
    font-size: var(--text-3xl);
    font-weight: 700;
    color: hsl(var(--text-primary));
    margin: 0 0 var(--space-2) 0;
  }

  .dashboard-subtitle {
    color: hsl(var(--text-secondary));
    font-size: var(--text-base);
    margin: 0;
  }

  .refresh-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: hsl(var(--slate-800) / 0.5);
    border: 1px solid hsl(var(--slate-700));
    border-radius: var(--radius-md);
    color: hsl(var(--text-secondary));
    font-size: var(--text-sm);
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .refresh-btn:hover:not(:disabled) {
    background: hsl(var(--slate-700) / 0.5);
    border-color: hsl(var(--slate-600));
    color: hsl(var(--text-primary));
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .loading-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-16) 0;
  }

  .loading-text {
    color: hsl(var(--text-muted));
    font-size: var(--text-sm);
    margin: 0;
  }

  .error-message {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    color: hsl(var(--coral-400));
  }

  .error-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .error-title {
    font-weight: 600;
    color: hsl(var(--coral-400));
  }

  .error-detail {
    font-size: var(--text-sm);
    color: hsl(var(--text-secondary));
  }

  .retry-btn {
    align-self: flex-start;
    padding: var(--space-2) var(--space-4);
    background: hsl(var(--coral-500) / 0.2);
    border: 1px solid hsl(var(--coral-500) / 0.4);
    border-radius: var(--radius-md);
    color: hsl(var(--coral-400));
    font-size: var(--text-sm);
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .retry-btn:hover {
    background: hsl(var(--coral-500) / 0.3);
    border-color: hsl(var(--coral-500) / 0.6);
  }

  /* API Status Section */
  .api-status {
    margin-bottom: var(--space-6);
  }

  .health-card {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
  }

  .health-indicator {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-lg);
  }

  .health-indicator.healthy {
    background: hsl(var(--teal-500) / 0.15);
    color: hsl(var(--teal-400));
  }

  .health-indicator.unhealthy {
    background: hsl(var(--coral-500) / 0.15);
    color: hsl(var(--coral-400));
  }

  .health-indicator.degraded {
    background: hsl(var(--amber-500) / 0.15);
    color: hsl(var(--amber-400));
  }

  .health-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .health-status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .health-details {
    font-size: var(--text-xs);
    color: hsl(var(--text-muted));
  }

  /* Agents Section */
  .agents-section {
    margin-bottom: var(--space-8);
  }

  .agents-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--space-3);
  }

  .agent-item {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
  }

  .agent-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .agent-type {
    font-weight: 600;
    color: hsl(var(--text-primary));
    font-size: var(--text-sm);
  }

  .agent-details {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .agent-capabilities {
    font-size: var(--text-xs);
    color: hsl(var(--text-secondary));
  }

  .agent-heartbeat {
    font-size: var(--text-xs);
    color: hsl(var(--text-muted));
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-8);
  }

  .stat-card {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
  }

  .stat-icon {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-lg);
  }

  .stat-icon.teal {
    background: hsl(var(--teal-500) / 0.15);
    color: hsl(var(--teal-400));
  }

  .stat-icon.purple {
    background: hsl(var(--purple-500) / 0.15);
    color: hsl(var(--purple-400));
  }

  .stat-icon.pink {
    background: hsl(var(--pink-500) / 0.15);
    color: hsl(var(--pink-400));
  }

  .stat-icon.amber {
    background: hsl(var(--amber-500) / 0.15);
    color: hsl(var(--amber-400));
  }

  .stat-content {
    display: flex;
    flex-direction: column;
  }

  .stat-value {
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: 600;
    color: hsl(var(--text-primary));
  }

  .stat-label {
    font-size: var(--text-sm);
    color: hsl(var(--text-muted));
  }

  .section-title {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    color: hsl(var(--text-primary));
    margin: 0 0 var(--space-4) 0;
  }

  .quick-actions {
    margin-bottom: var(--space-8);
  }

  .actions-grid {
    display: flex;
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  .action-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-6);
    background: hsl(var(--slate-800) / 0.5);
    border: 1px solid hsl(var(--slate-700));
    border-radius: var(--radius-lg);
    color: hsl(var(--text-primary));
    text-decoration: none;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .action-card:hover {
    background: hsl(var(--slate-700) / 0.5);
    border-color: hsl(var(--slate-600));
    transform: translateY(-2px);
    text-decoration: none;
  }

  .recent-activity {
    margin-bottom: var(--space-8);
  }

  .activity-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .activity-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: hsl(var(--slate-800) / 0.3);
    border-radius: var(--radius-md);
  }

  .activity-name {
    flex: 1;
    color: hsl(var(--text-primary));
    font-size: var(--text-sm);
  }

  .activity-time {
    color: hsl(var(--text-muted));
    font-size: var(--text-xs);
  }
</style>
