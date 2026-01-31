<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { isAuthenticated } from '$stores/auth';
  import { Button, Spinner } from '@caliber/ui';

  let isLoading = $state(false);
  let error = $state<string | null>(null);

  // Get return URL from query params
  const returnUrl = $derived($page.url.searchParams.get('return_url') || '/dashboard');

  // API base URL - in production this would come from env
  const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';

  onMount(() => {
    // If already authenticated, redirect
    if (isAuthenticated()) {
      goto(returnUrl);
    }
  });

  function handleLogin() {
    isLoading = true;
    error = null;

    // Build the SSO authorize URL with redirect back to our callback
    const callbackUrl = `${window.location.origin}/auth/callback`;
    const authUrl = `${API_URL}/auth/sso/authorize?redirect_uri=${encodeURIComponent(callbackUrl)}`;

    // Redirect to WorkOS SSO
    window.location.href = authUrl;
  }

  function handleDevLogin() {
    // For development: create a mock token
    if (import.meta.env.DEV) {
      const mockToken = btoa(JSON.stringify({
        user_id: 'dev-user-123',
        email: 'dev@caliber.run',
        first_name: 'Dev',
        last_name: 'User',
        organization_id: 'dev-tenant',
        exp: Math.floor(Date.now() / 1000) + 3600
      }));
      const fakeJwt = `eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.${mockToken}.dev-signature`;

      // Store and redirect
      localStorage.setItem('caliber_token', fakeJwt);
      localStorage.setItem('caliber_user', JSON.stringify({
        id: 'dev-user-123',
        email: 'dev@caliber.run',
        firstName: 'Dev',
        lastName: 'User',
        tenantId: 'dev-tenant'
      }));
      goto(returnUrl);
    }
  }
</script>

<div class="login-container">
  <div class="login-card">
    <div class="logo">
      <svg viewBox="0 0 40 40" fill="none" xmlns="http://www.w3.org/2000/svg" class="logo-icon">
        <circle cx="20" cy="20" r="18" stroke="currentColor" stroke-width="2" fill="none" />
        <path d="M12 20 L18 26 L28 14" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" fill="none" />
      </svg>
      <h1>CALIBER</h1>
    </div>

    <p class="tagline">Persistent context for AI agents</p>

    {#if error}
      <div class="error-message">
        {error}
      </div>
    {/if}

    <div class="actions">
      <Button
        onclick={handleLogin}
        disabled={isLoading}
        size="lg"
        variant="primary"
      >
        {#if isLoading}
          <Spinner size="sm" />
          <span>Connecting...</span>
        {:else}
          <span>Sign in with SSO</span>
        {/if}
      </Button>

      {#if import.meta.env.DEV}
        <button class="dev-login" onclick={handleDevLogin}>
          Dev Login (skip SSO)
        </button>
      {/if}
    </div>

    <p class="footer-text">
      Don't have an account? <a href="https://caliber.run" target="_blank">Learn more</a>
    </p>
  </div>
</div>

<style>
  .login-container {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, hsl(225 20% 10%), hsl(228 22% 6%));
    padding: 1rem;
  }

  .login-card {
    background: hsl(222 18% 12% / 0.8);
    border: 1px solid hsl(222 18% 20%);
    border-radius: 1rem;
    padding: 3rem;
    width: 100%;
    max-width: 400px;
    text-align: center;
    backdrop-filter: blur(10px);
  }

  .logo {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .logo-icon {
    width: 40px;
    height: 40px;
    color: hsl(175 70% 50%);
  }

  .logo h1 {
    font-size: 1.75rem;
    font-weight: 700;
    letter-spacing: 0.1em;
    color: hsl(210 20% 98%);
    margin: 0;
  }

  .tagline {
    color: hsl(220 16% 72%);
    margin-bottom: 2rem;
    font-size: 0.95rem;
  }

  .error-message {
    background: hsl(15 85% 50% / 0.15);
    border: 1px solid hsl(15 85% 50% / 0.3);
    color: hsl(15 85% 68%);
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    margin-bottom: 1.5rem;
    font-size: 0.875rem;
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .dev-login {
    background: transparent;
    border: 1px dashed hsl(220 14% 40%);
    color: hsl(220 14% 54%);
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    cursor: pointer;
    font-size: 0.75rem;
    transition: all 0.2s;
  }

  .dev-login:hover {
    border-color: hsl(175 70% 50%);
    color: hsl(175 70% 50%);
  }

  .footer-text {
    margin-top: 2rem;
    color: hsl(220 14% 54%);
    font-size: 0.875rem;
  }

  .footer-text a {
    color: hsl(175 70% 50%);
    text-decoration: none;
  }

  .footer-text a:hover {
    text-decoration: underline;
  }
</style>
