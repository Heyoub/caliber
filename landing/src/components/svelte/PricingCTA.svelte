<script lang="ts">
  /**
   * Pricing CTA Component
   * Handles LemonSqueezy checkout flow for CALIBER Cloud
   */

  interface Props {
    plan?: 'trial' | 'pro';
    label?: string;
    variant?: 'primary' | 'secondary';
  }

  let { plan = 'trial', label = 'Start Free Trial', variant = 'primary' }: Props = $props();

  const API_URL = import.meta.env.PUBLIC_API_URL || 'https://api.caliber.run';

  let loading = $state(false);
  let error = $state<string | null>(null);

  function getToken(): string | null {
    if (typeof localStorage === 'undefined') return null;
    return localStorage.getItem('caliber_token');
  }

  function isAuthenticated(): boolean {
    const token = getToken();
    if (!token) return false;

    try {
      const payload = JSON.parse(atob(token.split('.')[1]));
      const now = Math.floor(Date.now() / 1000);
      return payload.exp > now;
    } catch {
      return false;
    }
  }

  async function handleClick() {
    error = null;

    // If not authenticated, redirect to login first
    if (!isAuthenticated()) {
      window.location.href = `/login?return_url=${encodeURIComponent(window.location.pathname)}`;
      return;
    }

    // If it's a trial, just go to dashboard
    if (plan === 'trial') {
      window.location.href = '/dashboard';
      return;
    }

    // For paid plans, create a checkout session
    loading = true;

    try {
      const token = getToken();
      const response = await fetch(`${API_URL}/api/v1/billing/checkout`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          plan: plan,
          successUrl: `${window.location.origin}/dashboard/settings?upgraded=true`,
          cancelUrl: window.location.href,
        }),
      });

      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error(data.message || 'Failed to create checkout session');
      }

      const data = await response.json();

      if (data.checkout_url) {
        // Redirect to LemonSqueezy checkout
        window.location.href = data.checkout_url;
      } else {
        throw new Error('No checkout URL received');
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Something went wrong';
      loading = false;
    }
  }

  const buttonClass = $derived(
    variant === 'primary'
      ? 'bg-neon-purple/20 border-2 border-neon-purple text-text-primary hover:bg-neon-purple/30'
      : 'bg-bg-secondary border-2 border-border text-text-primary hover:border-neon-cyan'
  );
</script>

<div class="relative">
  <button
    type="button"
    onclick={handleClick}
    disabled={loading}
    class="w-full py-4 font-semibold brutalist-box transition-colors flex items-center justify-center gap-2 {buttonClass}"
    class:opacity-75={loading}
    class:cursor-not-allowed={loading}
  >
    {#if loading}
      <div class="w-5 h-5 border-2 border-current border-t-transparent rounded-full animate-spin"></div>
      <span>Processing...</span>
    {:else}
      <span>{label}</span>
    {/if}
  </button>

  {#if error}
    <p class="mt-2 text-sm text-red-400 text-center">
      {error}
    </p>
  {/if}
</div>
