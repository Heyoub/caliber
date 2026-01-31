<template>
  <div class="flex flex-col md:flex-row h-full bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-gray-100">
    <!-- Sidebar Navigation -->
    <div class="w-full md:w-64 border-r border-gray-200 border-solid dark:border-gray-700 p-4 flex flex-col gap-2 flex-shrink-0">
      <h2 class="text-xl font-semibold mb-4 text-gray-800 dark:text-gray-200">Admin Panel</h2>
      <button
        v-for="tab in tabs"
        :key="tab.id"
        @click="handleTabChange(tab.id)"
        :class="[
          'w-full text-left px-4 py-2 rounded-md transition-colors duration-150 ease-in-out focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 dark:focus:ring-offset-gray-900',
          activeTab === tab.id
            ? 'bg-indigo-600 text-white shadow-md hover:bg-indigo-700' // Active tab style
            : 'bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 border border-gray-300 dark:border-gray-600' // Inactive tab style
        ]"
      >
        {{ tab.label }}
      </button>
    </div>

    <!-- Content Area -->
    <div class="flex-1 overflow-auto p-6">
      <component :is="activeComponent" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, shallowRef, Component } from 'vue';
import { logAuditEvent } from '../../lib/auditLogger'; // Adjusted path

// Import placeholder components using shallowRef for performance if they become complex
import ModelControls from './ModelControls.vue';
import TokenManagement from './TokenManagement.vue';
import PluginManagement from './PluginManagement.vue';
import UsageCostPanel from './UsageCostPanel.vue';
import FallbackConfigPanel from './FallbackConfigPanel.vue';
import UserControlsPanel from './UserControlsPanel.vue';
import CompliancePanel from './CompliancePanel.vue';
import PluginInsightsPanel from './PluginInsightsPanel.vue';

type TabId = 'models' | 'tokens' | 'plugins' | 'usage' | 'fallback' | 'users' | 'compliance' | 'pluginInsights';

interface TabInfo {
  id: TabId;
  label: string;
  component: Component;
}

// Define tabs and their corresponding components
const tabs: TabInfo[] = [
  { id: 'models', label: 'Model Controls', component: shallowRef(ModelControls) },
  { id: 'tokens', label: 'Token Management', component: shallowRef(TokenManagement) },
  { id: 'plugins', label: 'Plugin Management', component: shallowRef(PluginManagement) },
  { id: 'usage', label: 'Usage & Cost', component: shallowRef(UsageCostPanel) },
  { id: 'fallback', label: 'Fallback Config', component: shallowRef(FallbackConfigPanel) },
  { id: 'users', label: 'User Controls', component: shallowRef(UserControlsPanel) },
  { id: 'compliance', label: 'Compliance', component: shallowRef(CompliancePanel) },
  { id: 'pluginInsights', label: 'Plugin Insights', component: shallowRef(PluginInsightsPanel) },
];

const activeTab = ref<TabId>('models');

// Compute the currently active component based on activeTab
const activeComponent = computed(() => {
  return tabs.find(tab => tab.id === activeTab.value)?.component;
});

const handleTabChange = (tabId: TabId) => {
  activeTab.value = tabId;
  try {
    logAuditEvent({
      eventType: 'ADMIN_TAB_SWITCH',
      userId: 'currentUser', // TODO: Replace with actual user ID from auth state
      action: 'switch_tab',
      resourceType: 'admin_panel',
      resourceId: tabId, // Use tabId as resourceId for context
      metadata: { newTab: tabId }
    });
  } catch (error) {
      console.error("Failed to log audit event:", error);
      // Handle logging error if necessary
  }
};
</script>

<style scoped>
/* Add any specific styles for AdminPanel if needed */
</style>