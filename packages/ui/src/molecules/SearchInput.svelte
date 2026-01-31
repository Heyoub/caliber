<script lang="ts">
  /**
   * SearchInput - Input + Icon + Clear button molecule
   * Features debounced search and loading state
   */
  import type { Snippet } from 'svelte';

  // Types for design system modifiers
  type Size = 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  type GlassEffect = boolean | 'subtle' | 'medium' | 'frosted' | 'solid';
  type BorderEffect = boolean | 'none' | 'subtle' | 'medium' | 'strong' | 'glow';

  interface Props {
    /** Current search value */
    value?: string;
    /** Placeholder text */
    placeholder?: string;
    /** Component size */
    size?: Size;
    /** Glass effect variant */
    glass?: GlassEffect;
    /** Border effect variant */
    border?: BorderEffect;
    /** Debounce delay in ms */
    debounce?: number;
    /** Loading state */
    loading?: boolean;
    /** Disabled state */
    disabled?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Search event handler */
    onsearch?: (value: string) => void;
    /** Input event handler */
    oninput?: (event: Event) => void;
    /** Clear event handler */
    onclear?: () => void;
  }

  let {
    value = $bindable(''),
    placeholder = 'Search...',
    size = 'md',
    glass = 'medium',
    border = 'subtle',
    debounce = 300,
    loading = false,
    disabled = false,
    class: className = '',
    onsearch,
    oninput,
    onclear
  }: Props = $props();

  // Size mappings
  const sizeClasses: Record<Size, string> = {
    xs: 'h-7 text-xs px-2',
    sm: 'h-8 text-sm px-2.5',
    md: 'h-10 text-base px-3',
    lg: 'h-12 text-lg px-4',
    xl: 'h-14 text-xl px-5'
  };

  const iconSizes: Record<Size, string> = {
    xs: 'w-3 h-3',
    sm: 'w-3.5 h-3.5',
    md: 'w-4 h-4',
    lg: 'w-5 h-5',
    xl: 'w-6 h-6'
  };

  // Glass effect mappings
  const glassClasses: Record<string, string> = {
    'true': 'backdrop-blur-md bg-slate-800/60',
    'false': 'bg-slate-800',
    'subtle': 'backdrop-blur-sm bg-slate-800/40',
    'medium': 'backdrop-blur-md bg-slate-800/60',
    'frosted': 'backdrop-blur-xl bg-slate-800/70',
    'solid': 'backdrop-blur-2xl bg-slate-800/80'
  };

  // Border effect mappings
  const borderClasses: Record<string, string> = {
    'true': 'border border-slate-600/50',
    'false': 'border-0',
    'none': 'border-0',
    'subtle': 'border border-slate-600/30',
    'medium': 'border border-slate-600/50',
    'strong': 'border-2 border-slate-500/60',
    'glow': 'border border-teal-500/40 shadow-[0_0_10px_hsl(176_55%_45%/0.15)]'
  };

  // Debounce logic
  let debounceTimer: ReturnType<typeof setTimeout>;

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    value = target.value;

    oninput?.(event);

    // Debounced search
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      onsearch?.(value);
    }, debounce);
  }

  function handleClear() {
    value = '';
    onclear?.();
    onsearch?.('');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      clearTimeout(debounceTimer);
      onsearch?.(value);
    }
    if (event.key === 'Escape' && value) {
      handleClear();
    }
  }

  // Computed classes
  const containerClasses = $derived(
    `search-input relative flex items-center rounded-lg transition-all duration-200
    ${sizeClasses[size]}
    ${glassClasses[String(glass)]}
    ${borderClasses[String(border)]}
    ${disabled ? 'opacity-50 pointer-events-none' : ''}
    ${className}`.trim()
  );
</script>

<div class={containerClasses}>
  <!-- Search Icon -->
  <svg
    class={`flex-shrink-0 text-slate-400 ${iconSizes[size]}`}
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <circle cx="11" cy="11" r="8" />
    <path d="m21 21-4.3-4.3" />
  </svg>

  <!-- Input -->
  <input
    type="text"
    {value}
    {placeholder}
    {disabled}
    oninput={handleInput}
    onkeydown={handleKeydown}
    class="flex-1 bg-transparent border-0 outline-none text-slate-100 placeholder-slate-500 ml-2
           focus:ring-0 focus:outline-none"
    aria-label="Search"
  />

  <!-- Loading Spinner -->
  {#if loading}
    <svg
      class={`flex-shrink-0 text-teal-400 animate-spin ${iconSizes[size]}`}
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
    >
      <path d="M21 12a9 9 0 1 1-6.219-8.56" />
    </svg>
  {:else if value}
    <!-- Clear Button -->
    <button
      type="button"
      onclick={handleClear}
      class={`flex-shrink-0 text-slate-400 hover:text-slate-200 transition-colors ${iconSizes[size]}`}
      aria-label="Clear search"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="w-full h-full"
      >
        <circle cx="12" cy="12" r="10" />
        <path d="m15 9-6 6" />
        <path d="m9 9 6 6" />
      </svg>
    </button>
  {/if}
</div>

<style>
  .search-input:focus-within {
    --component-color: hsl(176 55% 45%);
    box-shadow: 0 0 0 2px hsl(176 55% 45% / 0.2);
  }
</style>
