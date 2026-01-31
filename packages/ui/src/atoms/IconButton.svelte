<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ColorToken, Size, GlowEffect, HoverEffect, PressEffect } from '../types';

  /**
   * IconButton - Icon-only button variant
   */
  interface Props {
    /** Color token */
    color?: ColorToken;
    /** Size variant */
    size?: Size;
    /** Glow effect */
    glow?: GlowEffect;
    /** Hover effect type */
    hover?: HoverEffect;
    /** Press effect type */
    press?: PressEffect;
    /** Loading state */
    loading?: boolean;
    /** Disabled state */
    disabled?: boolean;
    /** Tooltip text */
    tooltip?: string;
    /** Button type */
    type?: 'button' | 'submit' | 'reset';
    /** Optional href - renders as anchor */
    href?: string;
    /** Aria label for accessibility */
    ariaLabel?: string;
    /** Additional CSS classes */
    class?: string;
    /** Click handler */
    onclick?: (event: MouseEvent) => void;
    /** Icon content slot */
    children?: Snippet;
  }

  let {
    color = 'ghost',
    size = 'md',
    glow = false,
    hover = 'scale',
    press = 'scale',
    loading = false,
    disabled = false,
    tooltip,
    type = 'button',
    href,
    ariaLabel,
    class: className = '',
    onclick,
    children,
  }: Props = $props();

  // Color configurations
  const colorConfigs: Record<string, { bg: string; text: string; hover: string; glow: string }> = {
    teal: {
      bg: 'bg-[hsl(var(--teal-500)/_0.1)]',
      text: 'text-[hsl(var(--teal-400))]',
      hover: 'hover:bg-[hsl(var(--teal-500)/_0.2)]',
      glow: 'hsl(var(--teal-400))',
    },
    coral: {
      bg: 'bg-[hsl(var(--coral-500)/_0.1)]',
      text: 'text-[hsl(var(--coral-400))]',
      hover: 'hover:bg-[hsl(var(--coral-500)/_0.2)]',
      glow: 'hsl(var(--coral-400))',
    },
    purple: {
      bg: 'bg-[hsl(var(--purple-500)/_0.1)]',
      text: 'text-[hsl(var(--purple-400))]',
      hover: 'hover:bg-[hsl(var(--purple-500)/_0.2)]',
      glow: 'hsl(var(--purple-400))',
    },
    mint: {
      bg: 'bg-[hsl(var(--mint-500)/_0.1)]',
      text: 'text-[hsl(var(--mint-400))]',
      hover: 'hover:bg-[hsl(var(--mint-500)/_0.2)]',
      glow: 'hsl(var(--mint-400))',
    },
    slate: {
      bg: 'bg-[hsl(var(--slate-700))]',
      text: 'text-[hsl(var(--slate-300))]',
      hover: 'hover:bg-[hsl(var(--slate-600))]',
      glow: 'hsl(var(--slate-400))',
    },
    ghost: {
      bg: 'bg-transparent',
      text: 'text-[hsl(var(--slate-400))]',
      hover: 'hover:bg-[hsl(var(--slate-800))] hover:text-[hsl(var(--slate-200))]',
      glow: 'transparent',
    },
  };

  // Size configurations
  const sizeConfigs: Record<Size, { button: string; icon: string }> = {
    xs: { button: 'w-6 h-6', icon: 'w-3 h-3' },
    sm: { button: 'w-8 h-8', icon: 'w-4 h-4' },
    md: { button: 'w-10 h-10', icon: 'w-5 h-5' },
    lg: { button: 'w-12 h-12', icon: 'w-6 h-6' },
    xl: { button: 'w-14 h-14', icon: 'w-7 h-7' },
    '2xl': { button: 'w-16 h-16', icon: 'w-8 h-8' },
  };

  // Hover effect classes
  const hoverClasses: Record<HoverEffect, string> = {
    none: '',
    lift: 'hover:-translate-y-0.5',
    glow: 'hover-glow',
    scale: 'hover:scale-110',
    brighten: 'hover:brightness-125',
    border: 'hover:border-opacity-100',
  };

  // Press effect classes
  const pressClasses: Record<PressEffect, string> = {
    none: '',
    sink: 'active:translate-y-0.5',
    scale: 'active:scale-95',
    darken: 'active:brightness-75',
  };

  // Derived values
  let baseColor = $derived(color.split('-')[0] as string);
  let colorConfig = $derived(colorConfigs[baseColor] || colorConfigs.ghost);
  let sizeConfig = $derived(sizeConfigs[size]);

  // Glow class
  let glowClass = $derived(() => {
    if (!glow) return '';
    if (glow === true) return 'glow-default';
    return `glow-${glow}`;
  });

  // Computed classes
  let computedClasses = $derived([
    'inline-flex items-center justify-center',
    'rounded-lg',
    'transition-all duration-200',
    'cursor-pointer',
    sizeConfig.button,
    colorConfig.bg,
    colorConfig.text,
    colorConfig.hover,
    hoverClasses[hover],
    pressClasses[press],
    disabled ? 'opacity-50 cursor-not-allowed pointer-events-none' : '',
    loading ? 'opacity-70 pointer-events-none' : '',
    glowClass(),
    className,
  ].filter(Boolean).join(' '));

  let isLink = $derived(!!href);
</script>

{#if isLink}
  <a
    {href}
    class={computedClasses}
    style="--glow-color: {colorConfig.glow};"
    aria-label={ariaLabel || tooltip}
    title={tooltip}
    aria-disabled={disabled}
  >
    {#if loading}
      <svg class="animate-spin {sizeConfig.icon}" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
      </svg>
    {:else if children}
      <span class={sizeConfig.icon}>
        {@render children()}
      </span>
    {/if}
  </a>
{:else}
  <button
    {type}
    {disabled}
    class={computedClasses}
    style="--glow-color: {colorConfig.glow};"
    aria-label={ariaLabel || tooltip}
    title={tooltip}
    {onclick}
  >
    {#if loading}
      <svg class="animate-spin {sizeConfig.icon}" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
      </svg>
    {:else if children}
      <span class={sizeConfig.icon}>
        {@render children()}
      </span>
    {/if}
  </button>
{/if}

<style>
  .glow-default {
    box-shadow: 0 0 12px var(--glow-color, transparent);
  }
  .glow-subtle {
    box-shadow: 0 0 6px var(--glow-color, transparent);
  }
  .glow-medium {
    box-shadow: 0 0 12px var(--glow-color, transparent);
  }
  .glow-intense {
    box-shadow: 0 0 20px var(--glow-color, transparent);
  }
  .glow-pulse {
    animation: glow-pulse 2s ease-in-out infinite;
  }

  .hover-glow:hover {
    box-shadow: 0 0 16px var(--glow-color, transparent);
  }

  @keyframes glow-pulse {
    0%, 100% { box-shadow: 0 0 12px var(--glow-color, transparent); }
    50% { box-shadow: 0 0 20px var(--glow-color, transparent); }
  }

  button:focus-visible,
  a:focus-visible {
    outline: 2px solid var(--glow-color, hsl(var(--teal-500)));
    outline-offset: 2px;
  }
</style>
