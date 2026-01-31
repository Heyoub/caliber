<script lang="ts">
  /**
   * Settings Page
   * User preferences, theme configuration, and API endpoint display
   */
  import { DashboardLayout } from '$layouts';
  import { Card, Toggle, Icon, Input } from '@caliber/ui';

  // Content strings
  const content = {
    title: 'Settings',
    subtitle: 'Configure your preferences and view system information',
    sections: {
      appearance: 'Appearance',
      api: 'API Configuration',
      preferences: 'User Preferences',
    },
    labels: {
      darkMode: 'Dark Mode',
      darkModeDescription: 'Use dark theme across the application',
      apiEndpoint: 'API Endpoint',
      wsEndpoint: 'WebSocket Endpoint',
      notifications: 'Enable Notifications',
      notificationsDescription: 'Receive notifications for important events',
      autoSave: 'Auto-save',
      autoSaveDescription: 'Automatically save changes as you work',
      compactView: 'Compact View',
      compactViewDescription: 'Use a more condensed layout',
    },
    placeholders: {
      comingSoon: 'More preferences coming soon...',
    },
  };

  // Theme state - defaults to dark mode
  let darkMode = $state(true);

  // User preferences state
  let notificationsEnabled = $state(true);
  let autoSaveEnabled = $state(true);
  let compactViewEnabled = $state(false);

  // API configuration - read from environment
  const apiEndpoint = import.meta.env.VITE_API_URL || '/api';
  const wsEndpoint =
    import.meta.env.VITE_WS_URL ||
    (typeof window !== 'undefined'
      ? `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`
      : 'ws://localhost:3000/ws');

  // Handle theme toggle
  function handleThemeChange(checked: boolean) {
    darkMode = checked;
    // In a real app, persist this to localStorage or user settings
    if (typeof document !== 'undefined') {
      document.documentElement.classList.toggle('light-mode', !checked);
    }
  }
</script>

<DashboardLayout>
  <div class="settings">
    <!-- Header -->
    <header class="settings-header">
      <h1 class="settings-title">{content.title}</h1>
      <p class="settings-subtitle">{content.subtitle}</p>
    </header>

    <!-- Appearance Section -->
    <section class="settings-section">
      <h2 class="section-title">
        <Icon name="palette" size="md" color="purple" />
        {content.sections.appearance}
      </h2>
      <Card glass="medium" border="subtle">
        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-label">{content.labels.darkMode}</span>
            <span class="setting-description">{content.labels.darkModeDescription}</span>
          </div>
          <Toggle
            bind:checked={darkMode}
            color="purple"
            onchange={handleThemeChange}
          />
        </div>
      </Card>
    </section>

    <!-- API Configuration Section -->
    <section class="settings-section">
      <h2 class="section-title">
        <Icon name="server" size="md" color="teal" />
        {content.sections.api}
      </h2>
      <Card glass="medium" border="subtle">
        <div class="api-config">
          <div class="config-item">
            <label class="config-label" for="api-endpoint">{content.labels.apiEndpoint}</label>
            <Input
              id="api-endpoint"
              value={apiEndpoint}
              readonly
              size="sm"
              glass="subtle"
            />
          </div>
          <div class="config-item">
            <label class="config-label" for="ws-endpoint">{content.labels.wsEndpoint}</label>
            <Input
              id="ws-endpoint"
              value={wsEndpoint}
              readonly
              size="sm"
              glass="subtle"
            />
          </div>
        </div>
      </Card>
    </section>

    <!-- User Preferences Section -->
    <section class="settings-section">
      <h2 class="section-title">
        <Icon name="sliders" size="md" color="pink" />
        {content.sections.preferences}
      </h2>
      <Card glass="medium" border="subtle">
        <div class="preferences-list">
          <div class="setting-item">
            <div class="setting-info">
              <span class="setting-label">{content.labels.notifications}</span>
              <span class="setting-description">{content.labels.notificationsDescription}</span>
            </div>
            <Toggle
              bind:checked={notificationsEnabled}
              color="teal"
            />
          </div>

          <div class="divider"></div>

          <div class="setting-item">
            <div class="setting-info">
              <span class="setting-label">{content.labels.autoSave}</span>
              <span class="setting-description">{content.labels.autoSaveDescription}</span>
            </div>
            <Toggle
              bind:checked={autoSaveEnabled}
              color="teal"
            />
          </div>

          <div class="divider"></div>

          <div class="setting-item">
            <div class="setting-info">
              <span class="setting-label">{content.labels.compactView}</span>
              <span class="setting-description">{content.labels.compactViewDescription}</span>
            </div>
            <Toggle
              bind:checked={compactViewEnabled}
              color="teal"
            />
          </div>
        </div>
      </Card>

      <!-- Placeholder for future preferences -->
      <div class="coming-soon">
        <Icon name="sparkles" size="sm" color="amber" />
        <span>{content.placeholders.comingSoon}</span>
      </div>
    </section>
  </div>
</DashboardLayout>

<style>
  .settings {
    max-width: 800px;
    margin: 0 auto;
  }

  .settings-header {
    margin-bottom: var(--space-8);
  }

  .settings-title {
    font-family: var(--font-display);
    font-size: var(--text-3xl);
    font-weight: 700;
    color: hsl(var(--text-primary));
    margin: 0 0 var(--space-2) 0;
  }

  .settings-subtitle {
    color: hsl(var(--text-secondary));
    font-size: var(--text-base);
    margin: 0;
  }

  .settings-section {
    margin-bottom: var(--space-8);
  }

  .section-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    color: hsl(var(--text-primary));
    margin: 0 0 var(--space-4) 0;
  }

  .setting-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4);
  }

  .setting-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .setting-label {
    font-size: var(--text-base);
    font-weight: 500;
    color: hsl(var(--text-primary));
  }

  .setting-description {
    font-size: var(--text-sm);
    color: hsl(var(--text-muted));
  }

  .api-config {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
  }

  .config-item {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .config-label {
    font-size: var(--text-sm);
    font-weight: 500;
    color: hsl(var(--text-secondary));
  }

  .preferences-list {
    display: flex;
    flex-direction: column;
  }

  .divider {
    height: 1px;
    background: hsl(var(--slate-700) / 0.5);
    margin: 0 var(--space-4);
  }

  .coming-soon {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-4);
    padding: var(--space-3) var(--space-4);
    background: hsl(var(--slate-800) / 0.3);
    border-radius: var(--radius-md);
    color: hsl(var(--text-muted));
    font-size: var(--text-sm);
  }
</style>
