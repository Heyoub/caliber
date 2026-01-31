<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { handleAuthCallback } from '$stores/auth';
  import { Spinner } from '@caliber/ui';

  let error = $state<string | null>(null);

  onMount(() => {
    const result = handleAuthCallback($page.url.searchParams);

    if (result.success) {
      // Get return URL from state or default to dashboard
      const returnUrl = $page.url.searchParams.get('return_url') || '/dashboard';
      goto(returnUrl);
    } else {
      error = result.error || 'Authentication failed';
      // Redirect to login after showing error
      setTimeout(() => {
        goto('/login');
      }, 3000);
    }
  });
</script>

<div class="callback-container">
  <div class="callback-card">
    {#if error}
      <div class="error">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="error-icon">
          <circle cx="12" cy="12" r="10" />
          <line x1="15" y1="9" x2="9" y2="15" />
          <line x1="9" y1="9" x2="15" y2="15" />
        </svg>
        <h2>Authentication Failed</h2>
        <p>{error}</p>
        <p class="redirect-text">Redirecting to login...</p>
      </div>
    {:else}
      <Spinner size="lg" />
      <p>Completing sign in...</p>
    {/if}
  </div>
</div>

<style>
  .callback-container {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, hsl(225 20% 10%), hsl(228 22% 6%));
  }

  .callback-card {
    text-align: center;
    color: hsl(210 20% 98%);
  }

  .callback-card p {
    margin-top: 1rem;
    color: hsl(220 16% 72%);
  }

  .error {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
  }

  .error-icon {
    width: 48px;
    height: 48px;
    color: hsl(15 85% 58%);
    margin-bottom: 0.5rem;
  }

  .error h2 {
    color: hsl(15 85% 68%);
    font-size: 1.25rem;
    margin: 0;
  }

  .error p {
    color: hsl(220 16% 72%);
    margin: 0.5rem 0 0;
  }

  .redirect-text {
    font-size: 0.875rem;
    color: hsl(220 14% 54%) !important;
  }
</style>
