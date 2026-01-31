<script lang="ts">
  /**
   * Root Layout
   * Handles global auth check and redirects
   */
  import '../app.css';
  import { Spinner } from '@caliber/ui';
  import type { LayoutData } from './$types';

  interface Props {
    data: LayoutData;
    children: import('svelte').Snippet;
  }

  let { data, children }: Props = $props();
</script>

<svelte:head>
  <title>CALIBER</title>
  <meta name="description" content="AI Agent Memory Management" />
</svelte:head>

{#if data.isLoading}
  <div class="auth-loading">
    <Spinner size="lg" color="teal" />
    <p class="loading-text">Loading...</p>
  </div>
{:else}
  {@render children()}
{/if}

<style>
  .auth-loading {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    background: hsl(var(--bg-primary));
  }

  .loading-text {
    color: hsl(var(--text-secondary));
    font-size: var(--text-sm);
  }
</style>
