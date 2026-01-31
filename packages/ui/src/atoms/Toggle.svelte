<script lang="ts">
  import type { Size, ColorToken } from '../types';

  /**
   * Toggle - Switch toggle component
   */
  interface Props {
    /** Checked state */
    checked?: boolean;
    /** Size variant */
    size?: Size;
    /** Color when checked */
    color?: ColorToken;
    /** Disabled state */
    disabled?: boolean;
    /** Label text */
    label?: string;
    /** Label position */
    labelPosition?: 'left' | 'right';
    /** Name attribute */
    name?: string;
    /** ID attribute */
    id?: string;
    /** Additional CSS classes */
    class?: string;
    /** Change handler */
    onchange?: (checked: boolean) => void;
  }

  let {
    checked = $bindable(false),
    size = 'md',
    color = 'teal',
    disabled = false,
    label,
    labelPosition = 'right',
    name,
    id,
    class: className = '',
    onchange,
  }: Props = $props();

  // Size configurations
  const sizeConfigs: Record<Size, { track: string; thumb: string; translate: string; label: string }> = {
    xs: { track: 'w-6 h-3.5', thumb: 'w-2.5 h-2.5', translate: 'translate-x-2.5', label: 'text-xs' },
    sm: { track: 'w-8 h-4', thumb: 'w-3 h-3', translate: 'translate-x-4', label: 'text-sm' },
    md: { track: 'w-10 h-5', thumb: 'w-4 h-4', translate: 'translate-x-5', label: 'text-sm' },
    lg: { track: 'w-12 h-6', thumb: 'w-5 h-5', translate: 'translate-x-6', label: 'text-base' },
    xl: { track: 'w-14 h-7', thumb: 'w-6 h-6', translate: 'translate-x-7', label: 'text-lg' },
    '2xl': { track: 'w-16 h-8', thumb: 'w-7 h-7', translate: 'translate-x-8', label: 'text-xl' },
  };

  // Color configurations
  const colorConfigs: Record<string, { bg: string; glow: string }> = {
    teal: {
      bg: 'bg-[hsl(var(--teal-500))]',
      glow: 'shadow-[0_0_10px_hsl(var(--teal-500)/_0.4)]',
    },
    coral: {
      bg: 'bg-[hsl(var(--coral-500))]',
      glow: 'shadow-[0_0_10px_hsl(var(--coral-500)/_0.4)]',
    },
    purple: {
      bg: 'bg-[hsl(var(--purple-500))]',
      glow: 'shadow-[0_0_10px_hsl(var(--purple-500)/_0.4)]',
    },
    pink: {
      bg: 'bg-[hsl(var(--pink-500))]',
      glow: 'shadow-[0_0_10px_hsl(var(--pink-500)/_0.4)]',
    },
    mint: {
      bg: 'bg-[hsl(var(--mint-500))]',
      glow: 'shadow-[0_0_10px_hsl(var(--mint-500)/_0.4)]',
    },
    amber: {
      bg: 'bg-[hsl(var(--amber-500))]',
      glow: 'shadow-[0_0_10px_hsl(var(--amber-500)/_0.4)]',
    },
  };

  // Derived values
  let baseColor = $derived(color.split('-')[0] as string);
  let sizeConfig = $derived(sizeConfigs[size]);
  let colorConfig = $derived(colorConfigs[baseColor] || colorConfigs.teal);

  function handleClick() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      handleClick();
    }
  }
</script>

<label
  class="inline-flex items-center gap-2 cursor-pointer select-none {disabled ? 'opacity-50 cursor-not-allowed' : ''} {className}"
  class:flex-row-reverse={labelPosition === 'left'}
>
  <input
    type="checkbox"
    {name}
    {id}
    {disabled}
    bind:checked
    class="sr-only"
    onchange={() => onchange?.(checked)}
  />

  <button
    type="button"
    role="switch"
    aria-checked={checked}
    {disabled}
    tabindex={disabled ? -1 : 0}
    onclick={handleClick}
    onkeydown={handleKeydown}
    class="relative inline-flex items-center rounded-full
           transition-all duration-200 ease-out
           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--teal-500))] focus-visible:ring-offset-2 focus-visible:ring-offset-[hsl(var(--slate-900))]
           {sizeConfig.track}
           {checked ? colorConfig.bg : 'bg-[hsl(var(--slate-700))]'}
           {checked ? colorConfig.glow : ''}"
  >
    <span
      class="absolute left-0.5 rounded-full bg-white shadow-sm
             transition-transform duration-200 ease-out
             {sizeConfig.thumb}
             {checked ? sizeConfig.translate : 'translate-x-0'}"
    ></span>
  </button>

  {#if label}
    <span class="text-[hsl(var(--slate-300))] {sizeConfig.label}">
      {label}
    </span>
  {/if}
</label>
