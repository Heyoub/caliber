<script lang="ts">
  /**
   * PromptButtons - Quick action prompt buttons
   * Reference: Vue PromptButtons.vue
   */
  import type { Snippet } from 'svelte';

  interface PromptButton {
    /** Button title */
    title: string;
    /** Prompt text to emit */
    prompt: string;
    /** Icon snippet */
    icon?: Snippet;
  }

  type Theme = 'default' | 'emi' | 'teal' | 'coral' | 'purple';

  interface Props {
    /** Array of prompt buttons */
    prompts: PromptButton[];
    /** Theme variant */
    theme?: Theme;
    /** Additional CSS classes */
    class?: string;
    /** Selection handler */
    onselect?: (prompt: string) => void;
  }

  let {
    prompts,
    theme = 'default',
    class: className = '',
    onselect,
  }: Props = $props();

  // Theme-based styling
  const themeStyles: Record<Theme, { button: string; icon: string }> = {
    default: {
      button: 'bg-slate-700/40 hover:bg-teal-500/20 hover:border-teal-500/70 border-slate-600 text-slate-200',
      icon: 'text-slate-200',
    },
    emi: {
      button: 'bg-slate-900/70 border-teal-500/30 text-slate-100 hover:bg-teal-500/20 hover:border-teal-500/50',
      icon: 'text-teal-400',
    },
    teal: {
      button: 'bg-teal-500/10 hover:bg-teal-500/20 border-teal-500/30 hover:border-teal-500/50 text-teal-200',
      icon: 'text-teal-400',
    },
    coral: {
      button: 'bg-coral-500/10 hover:bg-coral-500/20 border-coral-500/30 hover:border-coral-500/50 text-coral-200',
      icon: 'text-coral-400',
    },
    purple: {
      button: 'bg-purple-500/10 hover:bg-purple-500/20 border-purple-500/30 hover:border-purple-500/50 text-purple-200',
      icon: 'text-purple-400',
    },
  };

  let styles = $derived(themeStyles[theme]);

  function handleSelect(prompt: string) {
    onselect?.(prompt);
  }
</script>

<div class="flex flex-col items-center {className}">
  <div class="flex flex-wrap gap-2 justify-center">
    {#each prompts as prompt, index (index)}
      <button
        type="button"
        class="
          flex items-center gap-1.5 h-9 px-3 py-1.5
          transition-all shadow-sm hover:shadow
          rounded-md text-xs font-medium
          border border-solid
          {styles.button}
        "
        onclick={() => handleSelect(prompt.prompt)}
        title={prompt.prompt}
      >
        {#if prompt.icon}
          <span class="h-4 w-4 {styles.icon}">
            {@render prompt.icon()}
          </span>
        {/if}
        <span>{prompt.title}</span>
      </button>
    {/each}
  </div>
</div>
