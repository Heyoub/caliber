<template>
  <div
    :class="[
      'template-library-container p-4 flex flex-col h-full', // Added flex flex-col h-full
      props.theme === 'emi'
        ? 'bg-emi-bg-dark/80 text-slate-100' // No border/rounding for emi theme here
        : 'backdrop-blur-md border border-solid rounded-lg bg-slate-900/70 border-slate-700/50 text-slate-200' // Default glassmorphic
    ]">
    <h2
      :class="[
        'text-xl font-semibold mb-2 text-center',  // Reduced mb for button
        props.theme === 'emi' ? 'text-emi-accent' : 'text-slate-100'
      ]">Template Library</h2>
    <div class="flex justify-center mb-3">
      <button
        @click="settingsStore.restoreDefaultQuickTemplates()"
        title="Restore default quick templates"
        :class="[
          'px-3 py-1.5 text-xs font-medium rounded-md transition-colors duration-150',
          props.theme === 'emi'
            ? 'bg-emi-primary/20 text-emi-accent hover:bg-emi-primary/30'
            : 'bg-slate-700 text-slate-200 hover:bg-slate-600'
        ]"
      >
        Restore Defaults
      </button>
    </div>

    <div v-if="isLoading" class="text-center text-slate-400 py-8">Loading templates...</div>
    <div v-else-if="error" class="text-center text-red-400 py-8">Error: {{ error }}</div>

    <!-- Layout container: flex for horizontal scroll (default), block for vertical scroll (emi) -->
    <div class="flex-grow" v-else <!-- This div should be to take remaining space -->
      :class="[
        'flex-grow', // Added flex-grow
        props.theme === 'emi'
          ? 'space-y-4 overflow-y-auto custom-scrollbar-vertical-emi pr-1' // Vertical scroll for emi
          : 'flex overflow-x-auto overflow-y-hidden space-x-6 pb-4 custom-scrollbar-horizontal' // Horizontal for default
      ]">
      <!-- Category Section -->
      <div v-for="category in templateCategories" :key="category.name"
           :class="[
                       'p-4 rounded-xl shadow-lg',
                       props.theme === 'emi'
                         ? 'bg-emi-bg-dark/60' // Full width for emi theme
                         : 'min-w-[20rem] flex-shrink-0 bg-slate-800/60' // Min-width for default theme cards
                      ]">
        <h3
          :class="[
            'text-lg font-semibold mb-4 border-b border-solid pb-2',
            props.theme === 'emi' ? 'text-emi-accent border-emi-primary/30' : 'text-teal-400 border-slate-700'
          ]">
          {{ category.name }}
        </h3>
        <!-- Templates within category -->
        <div
          :class="[
                      'gap-2',
                      props.theme === 'emi' ? 'flex flex-col space-y-2' : 'flex flex-wrap' // Vertical list for emi, flex-wrap for default
                    ]">
          <div v-for="template in category.templates" :key="template.id" class="flex items-center gap-2 w-full">
            <button
              @click="selectTemplate(template.content)"
              :class="[
                              'flex-grow text-left p-3 rounded-lg transition-colors duration-150 focus:outline-none',
                              props.theme === 'emi'
                                ? 'bg-emi-bg-dark/50 hover:bg-emi-accent/20 focus:ring-2 focus:ring-emi-accent/70'
                                : 'bg-slate-700/50 hover:bg-teal-500/20 dark:bg-slate-700/40 dark:hover:bg-teal-600/20 focus:ring-2 focus:ring-teal-500/70 min-w-[8.75rem]'
                            ]"
              :style="props.theme !== 'emi' ? 'min-width: 140px;' : {}"
            >
              <p :class="['font-medium text-sm', props.theme === 'emi' ? 'text-slate-100' : 'text-slate-100']">{{ template.title }}</p>
              <p :class="['text-xs mt-1', props.theme === 'emi' ? 'text-slate-300' : 'text-slate-400']">{{ template.description }}</p>
            </button>
            <button
              @click="toggleQuickTemplate(template)"
              :title="template.isQuickTemplate ? 'Remove from Quick Templates' : 'Add to Quick Templates'"
              :class="[
                'p-2 rounded-md transition-colors duration-150 flex-shrink-0',
                props.theme === 'emi'
                  ? (template.isQuickTemplate ? 'bg-emi-accent/30 hover:bg-emi-accent/40' : 'bg-emi-bg-dark/40 hover:bg-emi-primary/30')
                  : (template.isQuickTemplate ? 'bg-teal-500/30 hover:bg-teal-500/40' : 'bg-slate-600/50 hover:bg-slate-500/50')
              ]"
            >
              <CheckIcon v-if="template.isQuickTemplate" :class="['w-4 h-4', props.theme === 'emi' ? 'text-emi-accent' : 'text-teal-400']" />
              <PlusIcon v-else :class="['w-4 h-4', props.theme === 'emi' ? 'text-slate-300' : 'text-slate-400']" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, type PropType, watch, computed } from 'vue'; // Added computed
import { useSettingsStore, type TemplateDefinition, type QuickTemplate, type iconMap as IconMapType } from '@/ai/store/settingsStore'; // Added IconMapType
import { CheckIcon, PlusIcon } from 'lucide-vue-next';

const props = withDefaults(defineProps<{
  theme?: 'default' | 'emi';
}>(), {
  theme: 'default',
});

// The LibraryTemplateCategory and LibraryTemplate types will now come from the store or be aligned with it.
// We'll use the store's definition of LibraryTemplateCategory (ImportedLibraryTemplateCategory)
// and TemplateDefinition for individual templates.
// No need for local LibraryTemplate and LibraryTemplateCategory interfaces.

const emit = defineEmits<{
  (e: 'template-selected', templateContent: string): void;
}>();

const settingsStore = useSettingsStore();
const isLoading = ref(true);
const error = ref<string | null>(null);

// templateCategories will now be a computed property based on the store's library
const templateCategories = computed(() => {
  return settingsStore.templateLibrary.map(category => ({
    ...category,
    templates: category.templates.map(template => ({
      ...template,
      // Ensure isQuickTemplate status is reactive based on quickTemplates list
      isQuickTemplate: settingsStore.quickTemplates.some(qt => qt.id === template.id)
    }))
  }));
});

// Remove local masterTemplateList as it's now in the store
// const masterTemplateList: LibraryTemplateCategory[] = [ ... ];

// syncQuickTemplateStatus is no longer needed as templateCategories computed property handles reactivity

const fetchTemplates = async () => {
  isLoading.value = true;
  error.value = null;
  try {
    await settingsStore.loadTemplateLibrary();
    // The computed property templateCategories will update automatically
  } catch (err: any) {
    console.error('Failed to load templates:', err);
    error.value = err.message || 'Could not load templates. Please try again later.';
  } finally {
    isLoading.value = false;
  }
};

const selectTemplate = (templateContent: string) => {
  emit('template-selected', templateContent);
};

const toggleQuickTemplate = (template: TemplateDefinition & { isQuickTemplate?: boolean }) => {
  // template.isQuickTemplate is now derived from the computed property,
  // so we check against the store's quickTemplates directly for the source of truth
  const isCurrentlyQuick = settingsStore.quickTemplates.some(qt => qt.id === template.id);

  if (!isCurrentlyQuick) { // User wants to ADD it
    const quickTemplateToAdd: QuickTemplate = {
      id: template.id,
      title: template.title,
      description: template.description,
      content: template.content,
      // icon mapping if needed, for now, quick templates might not show icons in the quick bar
      // icon: template.iconName ? settingsStore.iconMap[template.iconName] : undefined
    };
    settingsStore.addQuickTemplate(quickTemplateToAdd);
    // The computed property will update the template's isQuickTemplate status
  } else { // User wants to REMOVE it
    settingsStore.removeQuickTemplate(template.id);
    // The computed property will update
  }
};

onMounted(async () => {
  await fetchTemplates();
  // No need for explicit watch on settingsStore.quickTemplates to call syncQuickTemplateStatus,
  // as the `templateCategories` computed property will react to changes in `settingsStore.quickTemplates`
  // and `settingsStore.templateLibrary`.
});
</script>

<style scoped>
/* Add any component-specific styles if needed */
.template-library-container {
  /* max-height: calc(100vh - 200px); /* Adjust if needed, but horizontal scroll might change height requirements */
}

.custom-scrollbar-horizontal::-webkit-scrollbar {
  height: 8px;
}
.custom-scrollbar-horizontal::-webkit-scrollbar-track,
.custom-scrollbar-horizontal-emi::-webkit-scrollbar-track {
  background: transparent;
  border-radius: 4px;
}
.custom-scrollbar-horizontal::-webkit-scrollbar-thumb {
  background-color: theme('colors.slate.600 / 0.7');
  border-radius: 4px;
}
.custom-scrollbar-horizontal::-webkit-scrollbar-thumb:hover {
  background-color: theme('colors.teal.500 / 0.7');
}
.custom-scrollbar-horizontal {
  scrollbar-width: thin;
  scrollbar-color: theme('colors.slate.600 / 0.7') transparent;
}

/* Emi themed scrollbar */
.custom-scrollbar-horizontal-emi::-webkit-scrollbar {
  height: 8px;
}
.custom-scrollbar-horizontal-emi::-webkit-scrollbar-thumb {
  background-color: theme('colors.emi-primary / 0.7'); /* Use emi-primary */
  border-radius: 4px;
}
.custom-scrollbar-horizontal-emi::-webkit-scrollbar-thumb:hover {
  background-color: theme('colors.emi-accent / 0.7'); /* Use emi-accent */
}
.custom-scrollbar-horizontal-emi {
  scrollbar-width: thin;
  scrollbar-color: theme('colors.emi-primary / 0.7') transparent;
}

/* Emi themed VERTICAL scrollbar */
.custom-scrollbar-vertical-emi::-webkit-scrollbar {
  width: 8px; /* Width for vertical scrollbar */
}
.custom-scrollbar-vertical-emi::-webkit-scrollbar-track {
  background: transparent; /* Already covered by shared rule, but explicit for clarity */
  border-radius: 4px;
}
.custom-scrollbar-vertical-emi::-webkit-scrollbar-thumb {
  background-color: theme('colors.emi-primary / 0.7');
  border-radius: 4px;
}
.custom-scrollbar-vertical-emi::-webkit-scrollbar-thumb:hover {
  background-color: theme('colors.emi-accent / 0.7');
}
.custom-scrollbar-vertical-emi { /* Fallback for Firefox */
  scrollbar-width: thin;
  scrollbar-color: theme('colors.emi-primary / 0.7') transparent;
}
</style>
