<script lang="ts">
  /**
   * Dashboard Page
   * Thin stats UI - pulls from API and displays
   */
  import { DashboardLayout } from '$layouts';
  import { Card, Badge, Spinner, Icon } from '@caliber/ui';
  import { apiClient, type DashboardStats } from '$api/client';

  // Content strings
  const content = {
    title: 'Dashboard',
    subtitle: 'Overview of your AI agent memory usage',
    stats: {
      trajectories: 'Trajectories',
      scopes: 'Active Scopes',
      events: 'Total Events',
      storage: 'Storage Used',
    },
    recentActivity: 'Recent Activity',
    quickActions: 'Quick Actions',
    actions: {
      newTrajectory: 'New Trajectory',
      viewScopes: 'View Scopes',
      openEditor: 'Open Editor',
    },
  };

  // Fetch dashboard stats
  let stats = $state<DashboardStats | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

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
    } finally {
      loading = false;
    }
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
</script>

<DashboardLayout>
  <div class="dashboard">
    <!-- Header -->
    <header class="dashboard-header">
      <h1 class="dashboard-title">{content.title}</h1>
      <p class="dashboard-subtitle">{content.subtitle}</p>
    </header>

    <!-- Stats Grid -->
    {#if loading}
      <div class="loading-container">
        <Spinner size="lg" color="teal" />
      </div>
    {:else if error}
      <Card color="coral" glass="medium" border="subtle">
        <div class="error-message">
          <Icon name="alert-circle" color="coral" />
          <span>{error}</span>
        </div>
      </Card>
    {:else if stats}
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
            <div class="stat-icon pink">
              <Icon name="activity" size="lg" />
            </div>
            <div class="stat-content">
              <span class="stat-value">{formatNumber(stats.eventCount)}</span>
              <span class="stat-label">{content.stats.events}</span>
            </div>
          </div>
        </Card>

        <Card glass="medium" border="subtle" hover="lift">
          <div class="stat-card">
            <div class="stat-icon amber">
              <Icon name="database" size="lg" />
            </div>
            <div class="stat-content">
              <span class="stat-value">{formatBytes(stats.storageUsedBytes)}</span>
              <span class="stat-label">{content.stats.storage}</span>
            </div>
          </div>
        </Card>
      </div>

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
                <span class="activity-time">{activity.timestamp}</span>
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
    margin-bottom: var(--space-8);
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

  .loading-container {
    display: flex;
    justify-content: center;
    padding: var(--space-16) 0;
  }

  .error-message {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    color: hsl(var(--coral-400));
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
