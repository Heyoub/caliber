<template>
  <div
    :class="[
      'p-4 border-b border-solid backdrop-blur-xl relative overflow-hidden rounded-lg',
      props.theme === 'emi'
        ? 'bg-emi-bg-dark/80 border-emi-primary/30'
        : 'bg-slate-600/40 border-slate-500/50 dark:border-white/10'
    ]">
    <!-- Top highlight line -->
    <div
      :class="[
        'absolute top-0 left-1/4 right-1/4 h-[0.0625rem]',
        props.theme === 'emi'
          ? 'bg-gradient-to-r from-emi-bg-dark/0 via-emi-primary/20 to-transparent'
          : 'bg-gradient-to-r from-slate-800/40 via-slate-300/20 dark:via-white/10 to-transparent'
      ]"></div>

    <!-- "Speed Dial" Label - Will be moved into the right-aligned block -->

    <div class="flex justify-between items-center relative z-10">
      <div class="flex items-center gap-3"> <!-- Increased gap slightly for new toggle -->
        <div
          :class="[
            'w-14 h-14 rounded-full flex items-stretch justify-center border-none shadow-md',
            props.theme === 'emi' ? 'bg-emi-primary/20' : 'bg-slate-500/75'
          ]">
          <img src="/IMG/CouncilChat.png" alt="Emi Ai Logo" class="!p-0 !m-0 w-full h-full object-contain" />
        </div>
        <div>
          <h1
            :class="[
              'text-base font-medium leading-tight',
              props.theme === 'emi' ? 'text-slate-100' : 'text-slate-300 dark:text-slate-200'
            ]">
            Emi Ai {{ props.theme === 'emi' && !props.isAuthenticated ? '(Guest Mode)' : '' }}
          </h1>
          <p
            :class="[
              'text-xs',
              props.theme === 'emi' ? 'text-slate-300' : 'text-slate-300 dark:text-slate-300'
            ]">
            {{ props.theme === 'emi' && !props.isAuthenticated ? 'Limited access. Sign in for full features.' : "Let's talk business" }}
          </p>
        </div>
        <!-- New Quick Actions Toggle Temporarily Commented Out for Debugging -->
        <!--
        <QuickActionsToggle
          :theme="props.theme"
          @quick-prompt-selected="handleQuickPromptSelect"
          @quick-template-selected="handleQuickTemplateSelect"
        />
        -->
      </div>

      <!-- View Switching Buttons (for default theme) -->
      <div v-if="props.theme === 'default'" class="flex items-center gap-1 border-t border-slate-500/30 pt-3 mt-3">
        <button @click="emit('set-view', 'chat')" :class="['px-2 py-1 text-xs rounded', props.currentView === 'chat' ? 'bg-slate-500 text-white' : 'bg-slate-700 hover:bg-slate-600 text-slate-300']">Chat</button>
        <button @click="emit('set-view', 'prompts')" :class="['px-2 py-1 text-xs rounded', props.currentView === 'prompts' ? 'bg-slate-500 text-white' : 'bg-slate-700 hover:bg-slate-600 text-slate-300']">Prompts</button>
        <button @click="emit('set-view', 'templates')" :class="['px-2 py-1 text-xs rounded', props.currentView === 'templates' ? 'bg-slate-500 text-white' : 'bg-slate-700 hover:bg-slate-600 text-slate-300']">Templates</button>
        <button @click="emit('set-view', 'history')" :class="['px-2 py-1 text-xs rounded', props.currentView === 'history' ? 'bg-slate-500 text-white' : 'bg-slate-700 hover:bg-slate-600 text-slate-300']">History</button>
        <button @click="emit('set-view', 'documents')" :class="['px-2 py-1 text-xs rounded', props.currentView === 'documents' ? 'bg-slate-500 text-white' : 'bg-slate-700 hover:bg-slate-600 text-slate-300']">Docs</button>
      </div>

      <!-- Speed Dial Area -->
      <div class="flex-shrink-0 flex flex-col items-end relative ml-auto"> <!-- Parent aligns content (label, button div) to the right -->
        <p class="text-[0.625rem] leading-tight font-medium opacity-80 mb-0.5 self-end"
           :class="props.theme === 'emi' ? 'text-emi-accent/80' : 'text-slate-400 dark:text-slate-300/80'">
          Speed Dial
        </p>
        <div class="flex flex-row flex-nowrap gap-2 justify-start max-w-7xl overflow-x-auto custom-scrollbar-horizontal-tight py-1"> 
          <PromptButtons
            v-if="combinedSpeedDialItems.prompts.length > 0"
            :quick-prompts="combinedSpeedDialItems.prompts"
            :theme="props.theme"
            @prompt-selected="handleQuickPromptSelect"
          />
          <QuickTemplateButtons
            v-if="combinedSpeedDialItems.templates.length > 0"
            :quick-templates="combinedSpeedDialItems.templates"
            :theme="props.theme"
            @template-selected="handleQuickTemplateSelect"
            class="ml-0"
          />
        </div>
      </div>

       <!-- Conditionally display Sending badge -->
       <span v-if="isSending" class="absolute bottom-1 right-2 inline-flex items-center rounded-md bg-red-50 px-1.5 py-0.5 text-[0.625rem] font-medium text-red-700 ring-1 ring-inset ring-red-600/10 dark:bg-red-900/20 dark:text-red-300 dark:ring-red-800 z-10">
         Sending...
       </span>

    </div>
  </div>
</template>

<script setup lang="ts">
import { type PropType, computed } from 'vue'; // Added computed
import PromptButtons from './PromptButtons.vue';
import QuickTemplateButtons from './QuickTemplateButtons.vue'; // Import QuickTemplateButtons
// Icons are now handled within PromptButtons/QuickTemplateButtons if needed, or via settingsStore
// import { CheckCircle, Target, BarChart, LayoutDashboardIcon as Layout, Shield, Zap } from 'lucide-vue-next';
// import type { Component } from 'vue';

// QuickActionsToggle is no longer used
// import QuickActionsToggle from './QuickActionsToggle.vue';
import type { ViewMode } from '@/ai/store/aiContextStore';
import { useSettingsStore } from '@/ai/store/settingsStore';

// Define props
const props = defineProps<{
  isSending: boolean,
  // currentView is kept for now if ChatHeader's appearance might change based on it,
  // or if PromptButtons needs to know it. Otherwise, can be removed if view logic is fully in AuthNav.
  currentView: ViewMode,
  isAuthenticated: boolean,
  theme: 'default' | 'emi'
}>();

// Emit for when a quick prompt or template is selected
const emit = defineEmits(['quick-prompt-selected', 'quick-template-selected', 'set-view']);

function handleQuickPromptSelect(promptText: string) {
  emit('quick-prompt-selected', promptText);
}

function handleQuickTemplateSelect(templateContent: string) {
  emit('quick-template-selected', templateContent);
}

const settingsStore = useSettingsStore();

// The local 'headerPrompts' array and debug prompts are no longer needed.

const MAX_SPEED_DIAL_ITEMS = 8;

const combinedSpeedDialItems = computed(() => {
  const prompts = settingsStore.quickPrompts;
  const templates = settingsStore.quickTemplates;
  let availableSlots = MAX_SPEED_DIAL_ITEMS;

  const displayedPrompts = prompts.slice(0, availableSlots);
  availableSlots -= displayedPrompts.length;

  const displayedTemplates = templates.slice(0, availableSlots);
  
  return {
    prompts: displayedPrompts,
    templates: displayedTemplates,
  };
});

</script>

<style scoped>
/* Add any header-specific styles if needed */
.custom-scrollbar-horizontal-tight::-webkit-scrollbar {
  height: 4px; /* Even thinner scrollbar */
}
.custom-scrollbar-horizontal-tight::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar-horizontal-tight::-webkit-scrollbar-thumb {
  background-color: rgba(156, 163, 175, 0.3); /* gray-400 with opacity */
  border-radius: 2px;
}
.custom-scrollbar-horizontal-tight::-webkit-scrollbar-thumb:hover {
  background-color: rgba(156, 163, 175, 0.5);
}
.custom-scrollbar-horizontal-tight {
  scrollbar-width: thin;
  scrollbar-color: rgba(156, 163, 175, 0.3) transparent;
}
</style>