<script lang="ts">
  /**
   * Root Page - Redirect to Dashboard
   * This page immediately redirects authenticated users to the dashboard.
   * The redirect logic is handled in +page.ts for proper SSR support.
   */
  import { Spinner } from '@caliber/ui';
</script>

<div class="redirect-container">
  <Spinner size="lg" color="teal" />
  <p class="redirect-text">Redirecting...</p>
</div>

<style>
  .redirect-container {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    background: hsl(var(--bg-primary));
  }

  .redirect-text {
    color: hsl(var(--text-secondary));
    font-size: var(--text-sm);
    margin: 0;
  }
</style>
