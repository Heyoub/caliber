<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ColorToken, Size, GlowEffect } from '../types';

  /**
   * Badge - Status indicator with multiple variants
   */
  interface Props {
    /** Color token */
    color?: ColorToken;
    /** Size variant */
    size?: Size;
    /** Glow effect */
    glow?: GlowEffect;
    /** Show status dot */
    dot?: boolean;
    /** Removable badge */
    removable?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Remove handler */
    onremove?: () => void;
    /** Content slot */
    children?: Snippet;
  }

  let {
    color = 'teal',
    size = 'md',
    glow = false,
    dot = false,
    removable = false,
    class: className = '',
    onremove,
    children,
  }: Props = $props();

  // Color configurations
  const colorConfigs: Record<string, { bg: string; text: string; border: string; dot: string; glow: string }> = {
    teal: {
      bg: 'bg-[hsl(var(--teal-500)/_0.15)]',
      text: 'text-[hsl(var(--teal-400))]',
      border: 'border-[hsl(var(--teal-500)/_0.3)]',
      dot: 'bg-[hsl(var(--teal-400))]',
      glow: 'hsl(var(--teal-500))',
    },
    coral: {
      bg: 'bg-[hsl(var(--coral-500)/_0.15)]',
      text: 'text-[hsl(var(--coral-400))]',
      border: 'border-[hsl(var(--coral-500)/_0.3)]',
      dot: 'bg-[hsl(var(--coral-400))]',
      glow: 'hsl(var(--coral-500))',
    },
    purple: {
      bg: 'bg-[hsl(var(--purple-500)/_0.15)]',
      text: 'text-[hsl(var(--purple-400))]',
      border: 'border-[hsl(var(--purple-500)/_0.3)]',
      dot: 'bg-[hsl(var(--purple-400))]',
      glow: 'hsl(var(--purple-500))',
    },
    pink: {
      bg: 'bg-[hsl(var(--pink-500)/_0.15)]',
      text: 'text-[hsl(var(--pink-400))]',
      border: 'border-[hsl(var(--pink-500)/_0.3)]',
      dot: 'bg-[hsl(var(--pink-400))]',
      glow: 'hsl(var(--pink-500))',
    },
    mint: {
      bg: 'bg-[hsl(var(--mint-500)/_0.15)]',
      text: 'text-[hsl(var(--mint-400))]',
      border: 'border-[hsl(var(--mint-500)/_0.3)]',
      dot: 'bg-[hsl(var(--mint-400))]',
      glow: 'hsl(var(--mint-500))',
    },
    amber: {
      bg: 'bg-[hsl(var(--amber-500)/_0.15)]',
      text: 'text-[hsl(var(--amber-400))]',
      border: 'border-[hsl(var(--amber-500)/_0.3)]',
      dot: 'bg-[hsl(var(--amber-400))]',
      glow: 'hsl(var(--amber-500))',
    },
    slate: {
      bg: 'bg-[hsl(var(--slate-700))]',
      text: 'text-[hsl(var(--slate-300))]',
      border: 'border-[hsl(var(--slate-600))]',
      dot: 'bg-[hsl(var(--slate-400))]',
      glow: 'hsl(var(--slate-500))',
    },
    ghost: {
      bg: 'bg-transparent',
      text: 'text-[hsl(var(--slate-400))]',
      border: 'border-[hsl(var(--slate-700))]',
      dot: 'bg-[hsl(var(--slate-500))]',
      glow: 'transparent',
    },
  };

  // Size configurations
  const sizeConfigs: Record<Size, { badge: string; text: string; dot: string; remove: string }> = {
    xs: { badge: 'px-1.5 py-0.5', text: 'text-[10px]', dot: 'w-1.5 h-1.5', remove: 'w-3 h-3' },
    sm: { badge: 'px-2 py-0.5', text: 'text-xs', dot: 'w-2 h-2', remove: 'w-3.5 h-3.5' },
    md: { badge: 'px-2.5 py-1', text: 'text-sm', dot: 'w-2 h-2', remove: 'w-4 h-4' },
    lg: { badge: 'px-3 py-1.5', text: 'text-base', dot: 'w-2.5 h-2.5', remove: 'w-4.5 h-4.5' },
    xl: { badge: 'px-4 py-2', text: 'text-lg', dot: 'w-3 h-3', remove: 'w-5 h-5' },
    '2xl': { badge: 'px-5 py-2.5', text: 'text-xl', dot: 'w-3.5 h-3.5', remove: 'w-6 h-6' },
  };

  // Derived values
  let baseColor = $derived(color.split('-')[0] as string);
  let colorConfig = $derived(colorConfigs[baseColor] || colorConfigs.teal);
  let sizeConfig = $derived(sizeConfigs[size]);

  // Glow class
  let glowClass = $derived(() => {
    if (!glow) return '';
    if (glow === true) return 'shadow-[0_0_10px_var(--glow-color)]';
    if (glow === 'subtle') return 'shadow-[0_0_6px_var(--glow-color)]';
    if (glow === 'medium') return 'shadow-[0_0_10px_var(--glow-color)]';
    if (glow === 'intense') return 'shadow-[0_0_16px_var(--glow-color)]';
    if (glow === 'pulse') return 'glow-pulse';
    return '';
  });

  // Computed classes
  let computedClasses = $derived([
    'inline-flex items-center gap-1.5',
    'rounded-full',
    'border',
    'font-medium',
    'transition-all duration-200',
    sizeConfig.badge,
    sizeConfig.text,
    colorConfig.bg,
    colorConfig.text,
    colorConfig.border,
    glowClass(),
    className,
  ].filter(Boolean).join(' '));
</script>

<span
  class={computedClasses}
  style="--glow-color: {colorConfig.glow};"
>
  {#if dot}
    <span class="rounded-full {colorConfig.dot} {sizeConfig.dot} animate-pulse"></span>
  {/if}

  {#if children}
    {@render children()}
  {/if}

  {#if removable}
    <button
      type="button"
      class="inline-flex items-center justify-center rounded-full hover:bg-white/10 transition-colors ml-0.5 -mr-0.5"
      onclick={onremove}
      aria-label="Remove"
    >
      <svg class={sizeConfig.remove} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  {/if}
</span>

<style>
  .glow-pulse {
    animation: glow-pulse 2s ease-in-out infinite;
  }

  @keyframes glow-pulse {
    0%, 100% {
      box-shadow: 0 0 10px var(--glow-color);
    }
    50% {
      box-shadow: 0 0 20px var(--glow-color);
    }
  }
</style>
