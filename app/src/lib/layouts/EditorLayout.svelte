<script lang="ts">
  /**
   * Editor Layout
   * Three-panel layout for the pack editor
   * Provides sidebar navigation between assistant/playground modes
   */
  import { Icon, Button, Badge, Toggle } from '@caliber/ui';
  import { modeStore, type EditorMode } from '$stores/mode';
  import { authStore, getUserDisplayName, getUserInitials } from '$stores/auth';
  import { page } from '$app/stores';

  interface Props {
    mode: EditorMode;
    children: import('svelte').Snippet;
  }

  let { mode, children }: Props = $props();

  // Content strings
  const content = {
    logo: 'CALIBER',
    modes: {
      assistant: 'Assistant',
      playground: 'Playground',
    },
    nav: {
      dashboard: 'Dashboard',
      assistant: 'Assistant Mode',
      playground: 'Playground Mode',
    },
    actions: {
      save: 'Save',
      settings: 'Settings',
    },
  };

  // Get current user and path
  let user = $derived($authStore.user);
  let currentPath = $derived($page.url.pathname);

  // Sidebar state
  let sidebarCollapsed = $state(false);

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
  }

  function switchMode(newMode: EditorMode) {
    modeStore.setMode(newMode);
  }

  function handleLogout() {
    authStore.logout();
  }
</script>

<div class="editor-layout" class:sidebar-collapsed={sidebarCollapsed}>
  <!-- Sidebar -->
  <aside class="sidebar">
    <!-- Header -->
    <div class="sidebar-header">
      <a href="/dashboard" class="logo">
        <Icon name="arrow-left" size="sm" />
        {#if !sidebarCollapsed}
          <span class="logo-text">{content.logo}</span>
        {/if}
      </a>
      <button class="collapse-btn" onclick={toggleSidebar}>
        <Icon name={sidebarCollapsed ? 'chevrons-right' : 'chevrons-left'} size="sm" />
      </button>
    </div>

    <!-- Mode switcher -->
    <div class="mode-switcher">
      <button
        class="mode-btn"
        class:active={mode === 'assistant'}
        onclick={() => switchMode('assistant')}
      >
        <Icon name="message-square" size="sm" color={mode === 'assistant' ? 'purple' : undefined} />
        {#if !sidebarCollapsed}
          <span>{content.modes.assistant}</span>
        {/if}
      </button>
      <button
        class="mode-btn"
        class:active={mode === 'playground'}
        onclick={() => switchMode('playground')}
      >
        <Icon name="code" size="sm" color={mode === 'playground' ? 'teal' : undefined} />
        {#if !sidebarCollapsed}
          <span>{content.modes.playground}</span>
        {/if}
      </button>
    </div>

    <!-- Navigation -->
    <nav class="sidebar-nav">
      <a
        href="/editor/assistant"
        class="nav-item"
        class:active={currentPath === '/editor/assistant'}
      >
        <Icon name="bot" size="sm" />
        {#if !sidebarCollapsed}
          <span>{content.nav.assistant}</span>
        {/if}
      </a>
      <a
        href="/editor/playground"
        class="nav-item"
        class:active={currentPath === '/editor/playground'}
      >
        <Icon name="flask" size="sm" />
        {#if !sidebarCollapsed}
          <span>{content.nav.playground}</span>
        {/if}
      </a>

      <div class="nav-divider"></div>

      <a href="/dashboard" class="nav-item">
        <Icon name="layout-dashboard" size="sm" />
        {#if !sidebarCollapsed}
          <span>{content.nav.dashboard}</span>
        {/if}
      </a>
    </nav>

    <!-- Footer -->
    <div class="sidebar-footer">
      {#if !sidebarCollapsed}
        <div class="user-info">
          <div class="user-avatar">
            {getUserInitials(user)}
          </div>
          <div class="user-details">
            <span class="user-name">{getUserDisplayName(user)}</span>
          </div>
        </div>
      {/if}
      <button class="icon-btn" onclick={handleLogout} title="Sign out">
        <Icon name="log-out" size="sm" />
      </button>
    </div>
  </aside>

  <!-- Main area -->
  <div class="main-area">
    <!-- Top bar -->
    <header class="topbar">
      <div class="topbar-left">
        <Badge
          color={mode === 'assistant' ? 'purple' : 'teal'}
          glow="subtle"
        >
          {mode === 'assistant' ? content.modes.assistant : content.modes.playground}
        </Badge>
      </div>

      <div class="topbar-right">
        <Button size="sm" color="ghost">
          <Icon name="settings" size="sm" />
          {content.actions.settings}
        </Button>
        <Button size="sm" color="teal" glow="subtle">
          <Icon name="save" size="sm" />
          {content.actions.save}
        </Button>
      </div>
    </header>

    <!-- Content area -->
    <div class="content-area">
      {@render children()}
    </div>
  </div>
</div>

<style>
  .editor-layout {
    display: flex;
    min-height: 100vh;
    background: hsl(var(--bg-primary));
  }

  /* Sidebar */
  .sidebar {
    display: flex;
    flex-direction: column;
    width: 240px;
    background: hsl(var(--bg-secondary));
    border-right: 1px solid hsl(var(--border-subtle));
    transition: width var(--duration-normal) var(--ease-default);
  }

  .sidebar-collapsed .sidebar {
    width: 64px;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    padding: 0 var(--space-3);
    border-bottom: 1px solid hsl(var(--border-subtle));
  }

  .logo {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: hsl(var(--text-secondary));
    text-decoration: none;
    font-size: var(--text-sm);
    transition: color var(--duration-fast) var(--ease-default);
  }

  .logo:hover {
    color: hsl(var(--text-primary));
  }

  .logo-text {
    font-family: var(--font-display);
    font-weight: 600;
  }

  .collapse-btn,
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: hsl(var(--text-muted));
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .collapse-btn:hover,
  .icon-btn:hover {
    background: hsl(var(--slate-700) / 0.5);
    color: hsl(var(--text-secondary));
  }

  /* Mode switcher */
  .mode-switcher {
    display: flex;
    gap: var(--space-1);
    padding: var(--space-3);
    border-bottom: 1px solid hsl(var(--border-subtle));
  }

  .sidebar-collapsed .mode-switcher {
    flex-direction: column;
  }

  .mode-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    color: hsl(var(--text-muted));
    font-size: var(--text-xs);
    font-weight: 500;
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .mode-btn:hover {
    background: hsl(var(--slate-800) / 0.5);
    color: hsl(var(--text-secondary));
  }

  .mode-btn.active {
    background: hsl(var(--slate-800));
    border-color: hsl(var(--slate-700));
    color: hsl(var(--text-primary));
  }

  .sidebar-collapsed .mode-btn {
    padding: var(--space-2);
  }

  .sidebar-collapsed .mode-btn span {
    display: none;
  }

  /* Navigation */
  .sidebar-nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
    overflow-y: auto;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    color: hsl(var(--text-secondary));
    text-decoration: none;
    font-size: var(--text-sm);
    transition: all var(--duration-fast) var(--ease-default);
  }

  .nav-item:hover {
    background: hsl(var(--slate-800) / 0.5);
    color: hsl(var(--text-primary));
  }

  .nav-item.active {
    background: hsl(var(--teal-500) / 0.15);
    color: hsl(var(--teal-400));
  }

  .sidebar-collapsed .nav-item {
    justify-content: center;
    padding: var(--space-2);
  }

  .sidebar-collapsed .nav-item span {
    display: none;
  }

  .nav-divider {
    height: 1px;
    background: hsl(var(--border-subtle));
    margin: var(--space-2) 0;
  }

  /* Sidebar footer */
  .sidebar-footer {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    border-top: 1px solid hsl(var(--border-subtle));
  }

  .user-info {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .user-avatar {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: hsl(var(--purple-500) / 0.2);
    border: 1px solid hsl(var(--purple-500) / 0.3);
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    font-weight: 600;
    color: hsl(var(--text-primary));
  }

  .user-details {
    flex: 1;
    min-width: 0;
  }

  .user-name {
    font-size: var(--text-sm);
    font-weight: 500;
    color: hsl(var(--text-primary));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Main area */
  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    padding: 0 var(--space-4);
    background: hsl(var(--bg-secondary));
    border-bottom: 1px solid hsl(var(--border-subtle));
  }

  .topbar-left {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .topbar-right {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .content-area {
    flex: 1;
    padding: var(--space-4);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .content-area > :global(*) {
    flex: 1;
    min-height: 0;
  }
</style>
