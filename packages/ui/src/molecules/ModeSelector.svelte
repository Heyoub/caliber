<script lang="ts">
  /**
   * ModeSelector - AI mode toggle buttons
   * Based on ChatInput.vue pattern with standard, superThink, deepResearch, deepWork modes
   */
  import type { Snippet } from 'svelte';

  type AIMode = 'standard' | 'superThink' | 'deepResearch' | 'deepWork';

  interface ModeOption {
    /** Mode identifier */
    id: AIMode;
    /** Display label */
    label: string;
    /** Mode description */
    description?: string;
    /** Icon snippet */
    icon?: Snippet;
    /** Disabled state */
    disabled?: boolean;
  }

  interface Props {
    /** Currently selected mode */
    selected?: AIMode;
    /** Available modes (defaults to all) */
    modes?: ModeOption[];
    /** Compact display */
    compact?: boolean;
    /** Full width */
    fullWidth?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Mode change handler */
    onchange?: (mode: AIMode) => void;
  }

  // Default mode configurations
  const defaultModes: ModeOption[] = [
    {
      id: 'standard',
      label: 'Standard',
      description: 'Balanced speed and quality'
    },
    {
      id: 'superThink',
      label: 'SuperThink',
      description: 'Enhanced reasoning'
    },
    {
      id: 'deepResearch',
      label: 'DeepResearch',
      description: 'Web search and analysis'
    },
    {
      id: 'deepWork',
      label: 'DeepWork',
      description: 'Extended focus tasks'
    }
  ];

  let {
    selected = $bindable<AIMode>('standard'),
    modes = defaultModes,
    compact = false,
    fullWidth = false,
    class: className = '',
    onchange
  }: Props = $props();

  // Mode icon paths (Lucide-style)
  const modeIconPaths: Record<AIMode, string> = {
    standard: 'm6 9 6 6 6-6', // chevron-down
    superThink: 'M9 18V5l12-2v13 M9 9a3 3 0 0 0-3 3v1a3 3 0 0 0 3 3h0 M21 16a3 3 0 0 1-3-3v-1a3 3 0 0 1 3-3h0', // lightbulb approx
    deepResearch: 'M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z M2 12h20 M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z', // globe
    deepWork: 'M12 2v10l4.5 4.5 M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z' // clock
  };

  // Mode colors
  const modeColors: Record<AIMode, { active: string; inactive: string; glow: string }> = {
    standard: {
      active: 'bg-slate-600 text-white',
      inactive: 'text-slate-400 hover:text-white hover:bg-slate-700/50',
      glow: 'shadow-none'
    },
    superThink: {
      active: 'bg-amber-600 text-white',
      inactive: 'text-slate-400 hover:text-amber-300 hover:bg-amber-900/30',
      glow: 'shadow-[0_0_15px_hsl(38_70%_42%/0.4)]'
    },
    deepResearch: {
      active: 'bg-purple-600 text-white',
      inactive: 'text-slate-400 hover:text-purple-300 hover:bg-purple-900/30',
      glow: 'shadow-[0_0_15px_hsl(270_55%_52%/0.4)]'
    },
    deepWork: {
      active: 'bg-teal-600 text-white',
      inactive: 'text-slate-400 hover:text-teal-300 hover:bg-teal-900/30',
      glow: 'shadow-[0_0_15px_hsl(176_55%_45%/0.4)]'
    }
  };

  function handleModeClick(mode: AIMode) {
    const modeOption = modes.find(m => m.id === mode);
    if (modeOption?.disabled) return;

    selected = mode;
    onchange?.(mode);
  }

  function handleKeydown(mode: AIMode, event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleModeClick(mode);
    }
  }
</script>

<div
  class={`
    mode-selector flex items-center gap-1
    ${compact ? 'p-0.5' : 'p-1'}
    rounded-full
    bg-slate-800/50 backdrop-blur-md
    border border-slate-700/50
    ${fullWidth ? 'w-full justify-center' : 'inline-flex'}
    ${className}
  `}
  role="radiogroup"
  aria-label="AI Mode Selection"
>
  {#each modes as mode (mode.id)}
    {@const colors = modeColors[mode.id]}
    {@const isActive = selected === mode.id}

    <button
      type="button"
      class={`
        flex items-center justify-center gap-1.5
        ${compact ? 'px-2 py-1' : 'px-3 py-1.5'}
        ${compact ? 'text-xs' : 'text-sm'}
        rounded-full font-medium
        transition-all duration-200
        ${isActive ? `${colors.active} ${colors.glow}` : colors.inactive}
        ${mode.disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
        focus:outline-none focus:ring-2 focus:ring-teal-500/50
      `}
      onclick={() => handleModeClick(mode.id)}
      onkeydown={(e) => handleKeydown(mode.id, e)}
      disabled={mode.disabled}
      role="radio"
      aria-checked={isActive}
      title={mode.description}
    >
      <!-- Icon -->
      {#if mode.icon}
        <span class="w-4 h-4 flex-shrink-0">
          {@render mode.icon()}
        </span>
      {:else}
        <svg
          class={`${compact ? 'w-3 h-3' : 'w-4 h-4'} flex-shrink-0`}
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d={modeIconPaths[mode.id]} />
        </svg>
      {/if}

      <!-- Label (hide in very compact mode on mobile) -->
      <span class={compact ? 'hidden sm:inline' : ''}>
        {mode.label}
      </span>
    </button>
  {/each}
</div>

<style>
  .mode-selector {
    box-shadow: 0 2px 8px hsl(220 16% 8% / 0.3);
  }

  /* Active mode button animation */
  .mode-selector button[aria-checked="true"] {
    animation: mode-activate 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  @keyframes mode-activate {
    0% {
      transform: scale(0.95);
    }
    50% {
      transform: scale(1.02);
    }
    100% {
      transform: scale(1);
    }
  }
</style>
