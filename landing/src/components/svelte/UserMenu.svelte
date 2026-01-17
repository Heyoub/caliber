<script lang="ts">
  /**
   * User Menu Component
   * Dropdown menu for authenticated user actions
   */
  import { onMount, onDestroy } from 'svelte';

  interface User {
    id: string;
    email: string;
    firstName?: string;
    lastName?: string;
    tenantId?: string;
  }

  let user: User | null = $state(null);
  let isOpen = $state(false);
  let menuRef: HTMLDivElement | null = $state(null);

  function getUserFromStorage(): User | null {
    if (typeof localStorage === 'undefined') return null;
    const userJson = localStorage.getItem('caliber_user');
    if (!userJson) return null;
    try {
      return JSON.parse(userJson);
    } catch {
      return null;
    }
  }

  function getDisplayName(user: User | null): string {
    if (!user) return 'User';
    if (user.firstName && user.lastName) {
      return `${user.firstName} ${user.lastName}`;
    }
    if (user.firstName) return user.firstName;
    return user.email.split('@')[0];
  }

  function getInitials(user: User | null): string {
    if (!user) return '?';
    if (user.firstName && user.lastName) {
      return `${user.firstName[0]}${user.lastName[0]}`.toUpperCase();
    }
    if (user.firstName) return user.firstName[0].toUpperCase();
    return user.email[0].toUpperCase();
  }

  function toggleMenu() {
    isOpen = !isOpen;
  }

  function closeMenu() {
    isOpen = false;
  }

  function logout() {
    localStorage.removeItem('caliber_token');
    localStorage.removeItem('caliber_user');
    window.location.href = '/';
  }

  function handleClickOutside(event: MouseEvent) {
    if (menuRef && !menuRef.contains(event.target as Node) && isOpen) {
      closeMenu();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isOpen) {
      closeMenu();
    }
  }

  onMount(() => {
    user = getUserFromStorage();
    if (typeof window !== 'undefined') {
      window.addEventListener('click', handleClickOutside);
      window.addEventListener('keydown', handleKeydown);
    }
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('click', handleClickOutside);
      window.removeEventListener('keydown', handleKeydown);
    }
  });
</script>

<div class="relative" bind:this={menuRef}>
  <!-- Trigger button -->
  <button
    type="button"
    onclick={toggleMenu}
    class="flex items-center gap-3 px-3 py-2 rounded hover:bg-white/5 transition-colors"
    aria-expanded={isOpen}
    aria-haspopup="true"
  >
    <div class="w-8 h-8 bg-neon-purple/20 border border-neon-purple/30 flex items-center justify-center text-sm font-medium text-text-primary">
      {getInitials(user)}
    </div>
    <div class="hidden sm:block text-left">
      <p class="text-sm font-medium text-text-primary">{getDisplayName(user)}</p>
      {#if user?.email}
        <p class="text-xs text-text-muted">{user.email}</p>
      {/if}
    </div>
    <svg
      class="w-4 h-4 text-text-muted transition-transform"
      class:rotate-180={isOpen}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  <!-- Dropdown menu -->
  {#if isOpen}
    <div
      class="absolute right-0 mt-2 w-56 bg-bg-card border-2 border-border brutalist-box shadow-xl z-50"
      style="animation: fadeSlideIn 0.15s ease-out forwards;"
    >
      <!-- User info -->
      <div class="px-4 py-3 border-b border-border">
        <p class="text-sm font-medium text-text-primary">{getDisplayName(user)}</p>
        {#if user?.email}
          <p class="text-xs text-text-muted truncate">{user.email}</p>
        {/if}
      </div>

      <!-- Menu items -->
      <div class="py-1">
        <a
          href="/dashboard"
          onclick={closeMenu}
          class="flex items-center gap-3 px-4 py-2 text-sm text-text-secondary hover:text-text-primary hover:bg-white/5"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
          </svg>
          Dashboard
        </a>
        <a
          href="/dashboard/settings"
          onclick={closeMenu}
          class="flex items-center gap-3 px-4 py-2 text-sm text-text-secondary hover:text-text-primary hover:bg-white/5"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          Settings
        </a>
      </div>

      <!-- Logout -->
      <div class="py-1 border-t border-border">
        <button
          type="button"
          onclick={logout}
          class="flex items-center gap-3 w-full px-4 py-2 text-sm text-red-400 hover:bg-red-500/10"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
          </svg>
          Sign out
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  @keyframes fadeSlideIn {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
