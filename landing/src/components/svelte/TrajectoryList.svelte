<script lang="ts">
  /**
   * Trajectory List Component
   * Displays a paginated list of trajectories
   */
  import { onMount } from 'svelte';

  interface Trajectory {
    id: string;
    tenant_id: string;
    name: string;
    description?: string;
    created_at: string;
    updated_at: string;
    scope_count?: number;
    turn_count?: number;
    status?: 'active' | 'completed' | 'archived';
  }

  interface ApiResponse {
    data: Trajectory[];
    meta?: {
      total?: number;
      page?: number;
      limit?: number;
    };
  }

  const API_URL = import.meta.env.PUBLIC_API_URL || 'https://api.caliber.run';

  let trajectories: Trajectory[] = $state([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let total = $state(0);
  let page = $state(1);
  let limit = $state(10);

  function getToken(): string | null {
    if (typeof localStorage === 'undefined') return null;
    return localStorage.getItem('caliber_token');
  }

  async function fetchTrajectories() {
    const token = getToken();
    if (!token) {
      error = 'Not authenticated';
      loading = false;
      return;
    }

    loading = true;
    error = null;

    try {
      const response = await fetch(
        `${API_URL}/api/v1/trajectories?page=${page}&limit=${limit}`,
        {
          headers: {
            Authorization: `Bearer ${token}`,
          },
        }
      );

      if (!response.ok) {
        if (response.status === 401) {
          localStorage.removeItem('caliber_token');
          localStorage.removeItem('caliber_user');
          window.location.href = '/login';
          return;
        }
        throw new Error(`Failed to fetch trajectories: ${response.status}`);
      }

      const data: ApiResponse = await response.json();
      trajectories = data.data || [];
      total = data.meta?.total || 0;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load trajectories';
    } finally {
      loading = false;
    }
  }

  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function getStatusColor(status?: string): string {
    switch (status) {
      case 'active':
        return 'bg-green-500/20 text-green-400 border-green-500/30';
      case 'completed':
        return 'bg-neon-cyan/20 text-neon-cyan border-neon-cyan/30';
      case 'archived':
        return 'bg-text-muted/20 text-text-muted border-text-muted/30';
      default:
        return 'bg-neon-purple/20 text-neon-purple border-neon-purple/30';
    }
  }

  function nextPage() {
    if ((page * limit) < total) {
      page++;
      fetchTrajectories();
    }
  }

  function prevPage() {
    if (page > 1) {
      page--;
      fetchTrajectories();
    }
  }

  onMount(() => {
    fetchTrajectories();
  });
</script>

<div class="space-y-4">
  <!-- Loading state -->
  {#if loading}
    <div class="bg-bg-card border-2 border-border brutalist-box p-8 text-center">
      <div class="flex flex-col items-center gap-4">
        <div class="w-8 h-8 border-2 border-neon-purple border-t-transparent rounded-full animate-spin"></div>
        <p class="text-text-secondary">Loading trajectories...</p>
      </div>
    </div>
  {:else if error}
    <!-- Error state -->
    <div class="bg-bg-card border-2 border-red-500/30 brutalist-box p-8 text-center">
      <div class="flex flex-col items-center gap-4">
        <div class="w-12 h-12 bg-red-500/10 border border-red-500/30 flex items-center justify-center">
          <svg class="w-6 h-6 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>
        <p class="text-red-400">{error}</p>
        <button
          type="button"
          onclick={() => fetchTrajectories()}
          class="px-4 py-2 bg-bg-secondary border border-border text-text-secondary text-sm hover:border-neon-purple/50 transition-colors"
        >
          Try Again
        </button>
      </div>
    </div>
  {:else if trajectories.length === 0}
    <!-- Empty state -->
    <div class="bg-bg-card border-2 border-border brutalist-box p-8 text-center">
      <div class="flex flex-col items-center gap-4">
        <div class="w-16 h-16 bg-neon-purple/10 border border-neon-purple/30 flex items-center justify-center">
          <svg class="w-8 h-8 text-neon-purple" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
        </div>
        <div>
          <p class="text-text-primary font-medium">No trajectories yet</p>
          <p class="text-text-muted text-sm mt-1">Create your first trajectory using the SDK or API</p>
        </div>
        <a
          href="https://docs.caliber.run/quickstart"
          target="_blank"
          rel="noopener noreferrer"
          class="px-4 py-2 bg-neon-purple/20 border border-neon-purple text-text-primary text-sm hover:bg-neon-purple/30 transition-colors"
        >
          View Quickstart Guide
        </a>
      </div>
    </div>
  {:else}
    <!-- Trajectory list -->
    <div class="bg-bg-card border-2 border-border brutalist-box overflow-hidden">
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr class="border-b border-border bg-bg-secondary">
              <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider">
                Name
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider hidden sm:table-cell">
                Status
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider hidden md:table-cell">
                Created
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider hidden lg:table-cell">
                Scopes
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider hidden lg:table-cell">
                Turns
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border">
            {#each trajectories as trajectory (trajectory.id)}
              <tr class="hover:bg-white/5 transition-colors">
                <td class="px-4 py-4">
                  <div>
                    <p class="text-text-primary font-medium">{trajectory.name}</p>
                    {#if trajectory.description}
                      <p class="text-text-muted text-xs mt-1 truncate max-w-xs">
                        {trajectory.description}
                      </p>
                    {/if}
                    <p class="text-text-muted text-xs font-mono mt-1 sm:hidden">
                      {trajectory.id.slice(0, 8)}...
                    </p>
                  </div>
                </td>
                <td class="px-4 py-4 hidden sm:table-cell">
                  <span class="px-2 py-1 text-xs font-medium border {getStatusColor(trajectory.status)}">
                    {trajectory.status || 'active'}
                  </span>
                </td>
                <td class="px-4 py-4 text-text-secondary text-sm hidden md:table-cell">
                  {formatDate(trajectory.created_at)}
                </td>
                <td class="px-4 py-4 text-text-secondary text-sm hidden lg:table-cell">
                  {trajectory.scope_count ?? '-'}
                </td>
                <td class="px-4 py-4 text-text-secondary text-sm hidden lg:table-cell">
                  {trajectory.turn_count ?? '-'}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <!-- Pagination -->
      {#if total > limit}
        <div class="flex items-center justify-between px-4 py-3 border-t border-border bg-bg-secondary">
          <p class="text-text-muted text-sm">
            Showing {(page - 1) * limit + 1} to {Math.min(page * limit, total)} of {total}
          </p>
          <div class="flex gap-2">
            <button
              type="button"
              onclick={prevPage}
              disabled={page <= 1}
              class="px-3 py-1 bg-bg-primary border border-border text-text-secondary text-sm disabled:opacity-50 disabled:cursor-not-allowed hover:border-neon-purple/50 transition-colors"
            >
              Previous
            </button>
            <button
              type="button"
              onclick={nextPage}
              disabled={(page * limit) >= total}
              class="px-3 py-1 bg-bg-primary border border-border text-text-secondary text-sm disabled:opacity-50 disabled:cursor-not-allowed hover:border-neon-purple/50 transition-colors"
            >
              Next
            </button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
