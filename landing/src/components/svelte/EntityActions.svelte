<script lang="ts">
  /**
   * EntityActions Component
   * Renders HATEOAS links as actionable buttons
   */
  import type { Links, Link, LinkWithRel } from '$lib/types';

  interface Props {
    links: Links | undefined;
    onAction: (link: LinkWithRel) => Promise<void>;
    loading?: boolean;
    exclude?: string[];
  }

  let { links, onAction, loading = false, exclude = ['self'] }: Props = $props();

  let executingAction = $state<string | null>(null);

  const availableActions = $derived(
    links
      ? Object.entries(links)
          .filter(([rel]) => !exclude.includes(rel))
          .map(([rel, link]) => ({ rel, ...link } as LinkWithRel))
      : []
  );

  function getMethodClass(method?: string): string {
    switch (method?.toUpperCase()) {
      case 'POST':
        return 'action-post';
      case 'PUT':
      case 'PATCH':
        return 'action-patch';
      case 'DELETE':
        return 'action-delete';
      default:
        return 'action-get';
    }
  }

  function getMethodIcon(method?: string): string {
    switch (method?.toUpperCase()) {
      case 'POST':
        return 'M12 4v16m8-8H4'; // plus
      case 'PUT':
      case 'PATCH':
        return 'M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z'; // edit
      case 'DELETE':
        return 'M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16'; // trash
      default:
        return 'M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1'; // link
    }
  }

  function formatRelation(rel: string): string {
    return rel
      .replace(/_/g, ' ')
      .replace(/-/g, ' ')
      .replace(/\b\w/g, l => l.toUpperCase());
  }

  async function handleClick(action: LinkWithRel) {
    if (loading || executingAction) return;

    executingAction = action.rel;
    try {
      await onAction(action);
    } finally {
      executingAction = null;
    }
  }
</script>

{#if availableActions.length > 0}
  <div class="entity-actions">
    {#each availableActions as action (action.rel)}
      <button
        type="button"
        onclick={() => handleClick(action)}
        disabled={loading || !!executingAction}
        class="action-button {getMethodClass(action.method)}"
        title={action.title || formatRelation(action.rel)}
      >
        {#if executingAction === action.rel}
          <span class="action-spinner"></span>
        {:else}
          <svg class="action-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={getMethodIcon(action.method)} />
          </svg>
        {/if}
        <span class="action-label">{action.title || formatRelation(action.rel)}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .entity-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .action-button {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    font-size: 0.75rem;
    font-weight: 500;
    border: 1px solid;
    transition: all 0.15s ease;
    cursor: pointer;
  }

  .action-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-button:hover:not(:disabled) {
    transform: translateY(-1px);
  }

  .action-icon {
    width: 0.875rem;
    height: 0.875rem;
    flex-shrink: 0;
  }

  .action-spinner {
    width: 0.875rem;
    height: 0.875rem;
    border: 2px solid transparent;
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* GET actions - neutral/info */
  .action-get {
    background-color: rgba(139, 92, 246, 0.1);
    border-color: rgba(139, 92, 246, 0.3);
    color: rgb(167, 139, 250);
  }

  .action-get:hover:not(:disabled) {
    background-color: rgba(139, 92, 246, 0.2);
    border-color: rgba(139, 92, 246, 0.5);
  }

  /* POST actions - create/add */
  .action-post {
    background-color: rgba(34, 197, 94, 0.1);
    border-color: rgba(34, 197, 94, 0.3);
    color: rgb(74, 222, 128);
  }

  .action-post:hover:not(:disabled) {
    background-color: rgba(34, 197, 94, 0.2);
    border-color: rgba(34, 197, 94, 0.5);
  }

  /* PATCH/PUT actions - update/modify */
  .action-patch {
    background-color: rgba(6, 182, 212, 0.1);
    border-color: rgba(6, 182, 212, 0.3);
    color: rgb(34, 211, 238);
  }

  .action-patch:hover:not(:disabled) {
    background-color: rgba(6, 182, 212, 0.2);
    border-color: rgba(6, 182, 212, 0.5);
  }

  /* DELETE actions - destructive */
  .action-delete {
    background-color: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.3);
    color: rgb(248, 113, 113);
  }

  .action-delete:hover:not(:disabled) {
    background-color: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.5);
  }

  .action-label {
    white-space: nowrap;
  }

  /* Responsive: hide labels on small screens */
  @media (max-width: 640px) {
    .action-label {
      display: none;
    }

    .action-button {
      padding: 0.5rem;
    }
  }
</style>
