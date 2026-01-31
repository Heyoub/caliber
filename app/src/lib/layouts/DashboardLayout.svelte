<script lang="ts">
  /**
   * Dashboard Layout
   * Simple header + sidebar + content layout for dashboard pages
   */
  import { Icon, Button, Avatar, Dropdown } from '@caliber/ui';
  import { authStore, getUserDisplayName, getUserInitials } from '$stores/auth';
  import { page } from '$app/stores';

  interface Props {
    children: import('svelte').Snippet;
  }

  let { children }: Props = $props();

  // Content strings
  const content = {
    logo: 'CALIBER',
    nav: {
      overview: 'Overview',
      trajectories: 'Trajectories',
      settings: 'Settings',
    },
    user: {
      signOut: 'Sign out',
    },
  };

  // Navigation items
  const navItems = [
    { href: '/dashboard', label: content.nav.overview, icon: 'home' },
    { href: '/dashboard/trajectories', label: content.nav.trajectories, icon: 'git-branch' },
    { href: '/dashboard/settings', label: content.nav.settings, icon: 'settings' },
  ];

  // Get current user from auth store
  let user = $derived($authStore.user);
  let currentPath = $derived($page.url.pathname);

  // Mobile menu state
  let mobileMenuOpen = $state(false);

  function isActivePath(href: string): boolean {
    if (href === '/dashboard') {
      return currentPath === '/dashboard';
    }
    return currentPath.startsWith(href);
  }

  function handleLogout() {
    authStore.logout();
  }

  function toggleMobileMenu() {
    mobileMenuOpen = !mobileMenuOpen;
  }

  function closeMobileMenu() {
    mobileMenuOpen = false;
  }
</script>

<div class="dashboard-layout">
  <!-- Sidebar (desktop) -->
  <aside class="sidebar">
    <!-- Logo -->
    <div class="sidebar-header">
      <a href="/dashboard" class="logo">
        <span class="logo-text">{content.logo}</span>
      </a>
    </div>

    <!-- Navigation -->
    <nav class="sidebar-nav">
      {#each navItems as item}
        <a
          href={item.href}
          class="nav-item"
          class:active={isActivePath(item.href)}
        >
          <Icon name={item.icon} size="sm" />
          <span>{item.label}</span>
        </a>
      {/each}
    </nav>

    <!-- User section -->
    <div class="sidebar-footer">
      <div class="user-info">
        <Avatar name={user?.email || ''} size="sm" />
        <div class="user-details">
          <span class="user-name">{getUserDisplayName(user)}</span>
          <span class="user-email">{user?.email}</span>
        </div>
      </div>
      <button class="logout-btn" onclick={handleLogout}>
        <Icon name="log-out" size="sm" />
        <span>{content.user.signOut}</span>
      </button>
    </div>
  </aside>

  <!-- Mobile header -->
  <header class="mobile-header">
    <a href="/dashboard" class="logo">
      <span class="logo-text">{content.logo}</span>
    </a>
    <button class="menu-btn" onclick={toggleMobileMenu}>
      <Icon name={mobileMenuOpen ? 'x' : 'menu'} size="md" />
    </button>
  </header>

  <!-- Mobile menu overlay -->
  {#if mobileMenuOpen}
    <div class="mobile-overlay" onclick={closeMobileMenu}></div>
    <div class="mobile-menu">
      <nav class="mobile-nav">
        {#each navItems as item}
          <a
            href={item.href}
            class="nav-item"
            class:active={isActivePath(item.href)}
            onclick={closeMobileMenu}
          >
            <Icon name={item.icon} size="sm" />
            <span>{item.label}</span>
          </a>
        {/each}
      </nav>
      <div class="mobile-footer">
        <button class="logout-btn" onclick={handleLogout}>
          <Icon name="log-out" size="sm" />
          <span>{content.user.signOut}</span>
        </button>
      </div>
    </div>
  {/if}

  <!-- Main content -->
  <main class="main-content">
    {@render children()}
  </main>
</div>

<style>
  .dashboard-layout {
    display: flex;
    min-height: 100vh;
    background: hsl(var(--bg-primary));
  }

  /* Sidebar */
  .sidebar {
    display: none;
    flex-direction: column;
    width: 256px;
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    background: hsl(var(--bg-secondary));
    border-right: 1px solid hsl(var(--border-subtle));
    z-index: 40;
  }

  @media (min-width: 1024px) {
    .sidebar {
      display: flex;
    }
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    height: 64px;
    padding: 0 var(--space-6);
    border-bottom: 1px solid hsl(var(--border-subtle));
  }

  .logo {
    text-decoration: none;
  }

  .logo-text {
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 700;
    color: hsl(var(--text-primary));
  }

  .sidebar-nav {
    flex: 1;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    overflow-y: auto;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    color: hsl(var(--text-secondary));
    text-decoration: none;
    font-size: var(--text-sm);
    font-weight: 500;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .nav-item:hover {
    background: hsl(var(--slate-800) / 0.5);
    color: hsl(var(--text-primary));
  }

  .nav-item.active {
    background: hsl(var(--purple-500) / 0.15);
    color: hsl(var(--text-primary));
    border: 1px solid hsl(var(--purple-500) / 0.3);
  }

  .sidebar-footer {
    padding: var(--space-4);
    border-top: 1px solid hsl(var(--border-subtle));
  }

  .user-info {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    margin-bottom: var(--space-2);
  }

  .user-details {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .user-name {
    font-size: var(--text-sm);
    font-weight: 500;
    color: hsl(var(--text-primary));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .user-email {
    font-size: var(--text-xs);
    color: hsl(var(--text-muted));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .logout-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: hsl(var(--text-muted));
    font-size: var(--text-sm);
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .logout-btn:hover {
    background: hsl(var(--slate-800) / 0.5);
    color: hsl(var(--text-secondary));
  }

  /* Mobile header */
  .mobile-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 64px;
    padding: 0 var(--space-4);
    background: hsl(var(--bg-secondary));
    border-bottom: 1px solid hsl(var(--border-subtle));
    z-index: 40;
  }

  @media (min-width: 1024px) {
    .mobile-header {
      display: none;
    }
  }

  .menu-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-2);
    background: transparent;
    border: none;
    color: hsl(var(--text-secondary));
    cursor: pointer;
  }

  .menu-btn:hover {
    color: hsl(var(--text-primary));
  }

  /* Mobile menu */
  .mobile-overlay {
    position: fixed;
    inset: 0;
    background: hsl(var(--slate-950) / 0.6);
    backdrop-filter: blur(4px);
    z-index: 45;
  }

  .mobile-menu {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 280px;
    max-width: 80vw;
    background: hsl(var(--bg-secondary));
    border-left: 1px solid hsl(var(--border-subtle));
    z-index: 50;
    display: flex;
    flex-direction: column;
  }

  .mobile-nav {
    flex: 1;
    padding: var(--space-4);
    padding-top: calc(64px + var(--space-4));
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .mobile-footer {
    padding: var(--space-4);
    border-top: 1px solid hsl(var(--border-subtle));
  }

  /* Main content */
  .main-content {
    flex: 1;
    min-height: 100vh;
    padding-top: 64px;
  }

  @media (min-width: 1024px) {
    .main-content {
      margin-left: 256px;
      padding-top: 0;
    }
  }

  .main-content > :global(*) {
    padding: var(--space-4);
  }

  @media (min-width: 640px) {
    .main-content > :global(*) {
      padding: var(--space-6);
    }
  }

  @media (min-width: 1024px) {
    .main-content > :global(*) {
      padding: var(--space-8);
    }
  }
</style>
