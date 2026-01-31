<template>
  <div class="flex flex-col items-center"> <!-- Main container: flex column, items-center for button centering -->
    <div class="flex flex-wrap gap-2 justify-center"> <!-- Button container: flex-wrap, justify-center -->
      <button
        class="rounded-[1.25rem]"
               v-for="(prompt, index) in quickPrompts"
          :key="index"
          :class="[
                    'flex items-center gap-1.5 h-9 px-3 py-1.5 transition-all shadow-sm hover:shadow rounded-md text-xs font-medium',
                    props.theme === 'emi'
                      ? 'bg-emi-bg-dark/70 border border-emi-primary/30 text-slate-100 hover:bg-emi-accent/20 hover:border-emi-accent/50'
                      : 'bg-slate-700/40 dark:bg-slate-800/50 hover:bg-teal-500/20 hover:border-teal-500/70 dark:hover:bg-teal-600/20 dark:hover:border-teal-600/70 border border-slate-600 border-solid dark:border-slate-700 text-slate-200 dark:text-slate-100'
                  ]"
          @click="$emit('prompt-selected', prompt.prompt)"
          :title="prompt.prompt"
        >
          <component :is="prompt.icon"
            :class="['h-4 w-4', props.theme === 'emi' ? 'text-emi-accent' : 'text-slate-200 dark:text-slate-100']" />
          <span>{{ prompt.title }}</span>
        </button>
      </div>
      <!-- QUICK PROMPTS label removed as per new UI design -->
    </div>
  </template>

  <script setup lang="ts">
import { type PropType } from 'vue';
import { CheckCircle, Target, BarChart, LayoutDashboardIcon as Layout, Shield, Zap } from 'lucide-vue-next';

interface PromptButton {
  title: string;
  prompt: string;
  icon: import('vue').Component; // Keep this as Vue Component for Lucide icons
}

const props = defineProps({
  theme: {
    type: String as PropType<'default' | 'emi'>,
    default: 'default'
  },
  quickPrompts: { // New prop for the list of prompts
    type: Array as PropType<PromptButton[]>,
    required: true
  }
});

defineEmits(['prompt-selected']);

// The hardcoded 'prompts' array is removed.
// It will now be passed via the 'quickPrompts' prop.

</script>

<style scoped>
/* Add any component-specific styles if needed */
</style>