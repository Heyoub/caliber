<script lang="ts">
  /**
   * Dashboard Stats Page
   *
   * Detailed statistics view showing:
   * - Trajectories count with breakdown
   * - Agents count and status
   * - Recent activity timeline
   * - Storage usage
   *
   * Uses Svelte 5 runes ($state, $effect) for reactive state management.
   */
  import { DashboardLayout } from '$layouts';
  import { Card, Badge, Spinner, Icon } from '@caliber/ui';
  import { apiClient, type DashboardStats } from '$api/client';

  // Content strings
  const content = {
    title: 'Statistics',
    subtitle: 'Detailed metrics for your AI agent memory',
    overview: 'Overview',
    trajectories: {
      title: 'Trajectories',
      description: 'Active conversation threads and memory contexts',
    },
    agents: {
      title: 'Agents',
      description: 'AI agents with registered memory access',
    },
    events: {
      title: 'Events',
      description: 'Total interactions and memory operations',
    },
    storage: {
      title: 'Storage',
      description: 'Memory consumed by your data',
    },
    recentActivity: {
      title: 'Recent Activity',
      description: 'Latest changes to your memory store',
      empty: 'No recent activity',
    },
    refresh: 'Refresh',
    lastUpdated: 'Last updated',
  };

  // Reactive state using Svelte 5 runes
  let stats = $state<DashboardStats | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let lastUpdated = $state<Date | null>(null);

  // Load stats on mount
  $effect(() => {
    loadStats();
  });

  async function loadStats() {
    try {
      loading = true;
      error = null;
      stats = await apiClient.getDashboardStats();
      lastUpdated = new Date();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load statistics';
      console.error('Failed to load dashboard stats:', e);
    } finally {
      loading = false;
    }
  }

  function formatNumber(num: number): string {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toLocaleString();
  }

  function formatBytes(bytes: number): string {
    if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(2)} GB`;
    if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(2)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(2)} KB`;
    return `${bytes} B`;
  }

  function formatTimestamp(timestamp: string): string {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  }

  function formatLastUpdated(date: Date | null): string {
    if (!date) return '';
    return date.toLocaleTimeString();
  }

  function getActionColor(action: string): 'teal' | 'purple' | 'coral' {
    switch (action) {
      case 'created':
        return 'teal';
      case 'updated':
        return 'purple';
      case 'deleted':
        return 'coral';
      default:
        return 'teal';
    }
  }

  function getTypeIcon(type: string): string {
    switch (type) {
      case 'trajectory':
        return 'git-branch';
      case 'scope':
        return 'layers';
      case 'event':
        return 'activity';
      default:
        return 'circle';
    }
  }
</script>

<DashboardLayout>
  <div class="stats-page">
    <!-- Header -->
    <header class="page-header">
      <div class="header-content">
        <h1 class="page-title">{content.title}</h1>
        <p class="page-subtitle">{content.subtitle}</p>
      </div>
      <div class="header-actions">
        {#if lastUpdated}
          <span class="last-updated">
            {content.lastUpdated}: {formatLastUpdated(lastUpdated)}
          </span>
        {/if}
        <button
          class="refresh-btn"
          onclick={loadStats}
          disabled={loading}
          aria-label={content.refresh}
        >
          <Icon name="refresh-cw" size="sm" />
          <span>{content.refresh}</span>
        </button>
      </div>
    </header>

    <!-- Loading State -->
    {#if loading && !stats}
      <div class="loading-container">
        <Spinner size="lg" color="teal" />
        <p class="loading-text">Loading statistics...</p>
      </div>

    <!-- Error State -->
    {:else if error && !stats}
      <Card color="coral" glass="medium" border="subtle">
        <div class="error-message">
          <Icon name="alert-circle" color="coral" size="lg" />
          <div class="error-content">
            <h3>Failed to load statistics</h3>
            <p>{error}</p>
          </div>
          <button class="retry-btn" onclick={loadStats}>
            <Icon name="refresh-cw" size="sm" />
            Try Again
          </button>
        </div>
      </Card>

    <!-- Stats Content -->
    {:else if stats}
      <!-- Overview Section -->
      <section class="section">
        <h2 class="section-title">{content.overview}</h2>
        <div class="stats-grid">
          <!-- Trajectories Card -->
          <Card glass="medium" border="subtle" hover="lift">
            <div class="stat-card">
              <div class="stat-header">
                <div class="stat-icon teal">
                  <Icon name="git-branch" size="lg" />
                </div>
                <div class="stat-trend positive">
                  <Icon name="trending-up" size="sm" />
                </div>
              </div>
              <div class="stat-body">
                <span class="stat-value">{formatNumber(stats.trajectoryCount)}</span>
                <span class="stat-label">{content.trajectories.title}</span>
                <span class="stat-description">{content.trajectories.description}</span>
              </div>
            </div>
          </Card>

          <!-- Agents Card -->
          <Card glass="medium" border="subtle" hover="lift">
            <div class="stat-card">
              <div class="stat-header">
                <div class="stat-icon purple">
                  <Icon name="users" size="lg" />
                </div>
              </div>
              <div class="stat-body">
                <span class="stat-value">{formatNumber(stats.agents?.length ?? 0)}</span>
                <span class="stat-label">{content.agents.title}</span>
                <span class="stat-description">{content.agents.description}</span>
              </div>
            </div>
          </Card>

          <!-- Events Card -->
          <Card glass="medium" border="subtle" hover="lift">
            <div class="stat-card">
              <div class="stat-header">
                <div class="stat-icon pink">
                  <Icon name="activity" size="lg" />
                </div>
              </div>
              <div class="stat-body">
                <span class="stat-value">{formatNumber(stats.eventCount)}</span>
                <span class="stat-label">{content.events.title}</span>
                <span class="stat-description">{content.events.description}</span>
              </div>
            </div>
          </Card>

          <!-- Storage Card -->
          <Card glass="medium" border="subtle" hover="lift">
            <div class="stat-card">
              <div class="stat-header">
                <div class="stat-icon amber">
                  <Icon name="database" size="lg" />
                </div>
              </div>
              <div class="stat-body">
                <span class="stat-value">{formatBytes(stats.storageUsedBytes)}</span>
                <span class="stat-label">{content.storage.title}</span>
                <span class="stat-description">{content.storage.description}</span>
              </div>
            </div>
          </Card>
        </div>
      </section>

      <!-- Recent Activity Section -->
      <section class="section">
        <div class="section-header">
          <div>
            <h2 class="section-title">{content.recentActivity.title}</h2>
            <p class="section-description">{content.recentActivity.description}</p>
          </div>
        </div>

        {#if stats.recentActivity && stats.recentActivity.length > 0}
          <Card glass="subtle" border="subtle">
            <div class="activity-list">
              {#each stats.recentActivity as activity}
                <div class="activity-item">
                  <div class="activity-icon {activity.type}">
                    <Icon name={getTypeIcon(activity.type)} size="sm" />
                  </div>
                  <div class="activity-content">
                    <div class="activity-main">
                      <span class="activity-name">{activity.name}</span>
                      <Badge color={getActionColor(activity.action)} size="sm">
                        {activity.action}
                      </Badge>
                    </div>
                    <div class="activity-meta">
                      <Badge color="slate" size="sm">
                        {activity.type}
                      </Badge>
                      <span class="activity-time">{formatTimestamp(activity.timestamp)}</span>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </Card>
        {:else}
          <Card glass="subtle" border="subtle">
            <div class="empty-state">
              <Icon name="inbox" color="slate" size="xl" />
              <p>{content.recentActivity.empty}</p>
            </div>
          </Card>
        {/if}
      </section>
    {/if}
  </div>
</DashboardLayout>

<style>
  .stats-page {
    max-width: 1200px;
    margin: 0 auto;
    padding-bottom: var(--space-8);
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--space-8);
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .header-content {
    flex: 1;
    min-width: 200px;
  }

  .page-title {
    font-family: var(--font-display);
    font-size: var(--text-3xl);
    font-weight: 700;
    color: hsl(var(--text-primary));
    margin: 0 0 var(--space-2) 0;
  }

  .page-subtitle {
    color: hsl(var(--text-secondary));
    font-size: var(--text-base);
    margin: 0;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .last-updated {
    color: hsl(var(--text-muted));
    font-size: var(--text-sm);
  }

  .refresh-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: hsl(var(--slate-800) / 0.5);
    border: 1px solid hsl(var(--slate-700));
    border-radius: var(--radius-md);
    color: hsl(var(--text-primary));
    font-size: var(--text-sm);
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .refresh-btn:hover:not(:disabled) {
    background: hsl(var(--slate-700) / 0.5);
    border-color: hsl(var(--slate-600));
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .loading-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-16) 0;
    gap: var(--space-4);
  }

  .loading-text {
    color: hsl(var(--text-secondary));
    font-size: var(--text-sm);
  }

  .error-message {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
  }

  .error-content {
    flex: 1;
  }

  .error-content h3 {
    color: hsl(var(--coral-400));
    font-size: var(--text-base);
    font-weight: 600;
    margin: 0 0 var(--space-1) 0;
  }

  .error-content p {
    color: hsl(var(--text-secondary));
    font-size: var(--text-sm);
    margin: 0;
  }

  .retry-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: hsl(var(--coral-500) / 0.2);
    border: 1px solid hsl(var(--coral-500) / 0.5);
    border-radius: var(--radius-md);
    color: hsl(var(--coral-400));
    font-size: var(--text-sm);
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .retry-btn:hover {
    background: hsl(var(--coral-500) / 0.3);
  }

  .section {
    margin-bottom: var(--space-8);
  }

  .section-header {
    margin-bottom: var(--space-4);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 600;
    color: hsl(var(--text-primary));
    margin: 0 0 var(--space-1) 0;
  }

  .section-description {
    color: hsl(var(--text-muted));
    font-size: var(--text-sm);
    margin: 0;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: var(--space-4);
  }

  .stat-card {
    padding: var(--space-4);
  }

  .stat-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--space-4);
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

  .stat-trend {
    display: flex;
    align-items: center;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
  }

  .stat-trend.positive {
    background: hsl(var(--teal-500) / 0.15);
    color: hsl(var(--teal-400));
  }

  .stat-body {
    display: flex;
    flex-direction: column;
  }

  .stat-value {
    font-family: var(--font-display);
    font-size: var(--text-3xl);
    font-weight: 700;
    color: hsl(var(--text-primary));
    line-height: 1;
    margin-bottom: var(--space-1);
  }

  .stat-label {
    font-size: var(--text-base);
    font-weight: 500;
    color: hsl(var(--text-primary));
    margin-bottom: var(--space-1);
  }

  .stat-description {
    font-size: var(--text-sm);
    color: hsl(var(--text-muted));
  }

  .activity-list {
    display: flex;
    flex-direction: column;
  }

  .activity-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-4);
    border-bottom: 1px solid hsl(var(--slate-700) / 0.5);
  }

  .activity-item:last-child {
    border-bottom: none;
  }

  .activity-icon {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    flex-shrink: 0;
  }

  .activity-icon.trajectory {
    background: hsl(var(--teal-500) / 0.15);
    color: hsl(var(--teal-400));
  }

  .activity-icon.scope {
    background: hsl(var(--purple-500) / 0.15);
    color: hsl(var(--purple-400));
  }

  .activity-icon.event {
    background: hsl(var(--pink-500) / 0.15);
    color: hsl(var(--pink-400));
  }

  .activity-content {
    flex: 1;
    min-width: 0;
  }

  .activity-main {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-1);
    flex-wrap: wrap;
  }

  .activity-name {
    font-size: var(--text-sm);
    font-weight: 500;
    color: hsl(var(--text-primary));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .activity-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .activity-time {
    color: hsl(var(--text-muted));
    font-size: var(--text-xs);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-8);
    gap: var(--space-3);
    color: hsl(var(--text-muted));
  }

  .empty-state p {
    margin: 0;
    font-size: var(--text-sm);
  }

  /* Responsive */
  @media (max-width: 640px) {
    .page-header {
      flex-direction: column;
    }

    .header-actions {
      width: 100%;
      justify-content: space-between;
    }

    .stats-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
