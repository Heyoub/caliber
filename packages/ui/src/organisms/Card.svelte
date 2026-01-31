<!--
  Card.svelte - Glassmorphic card organism
  Ported from GlassMorphicCard.vue + HybridGlassMorphicCard.vue

  Features:
  - Glass, glow, border effects
  - Header/footer slots
  - Mouse-tracking spotlight effect
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ColorPalette, GlassEffect, GlowEffect, BorderEffect, HoverEffect, CMSContent } from '../types/index.js';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Card color theme */
    color?: ColorPalette;
    /** Glass effect intensity */
    glass?: GlassEffect;
    /** Glow effect */
    glow?: GlowEffect;
    /** Border effect */
    border?: BorderEffect;
    /** Enable hover effects */
    hoverEffect?: boolean;
    /** Hover animation type */
    hover?: HoverEffect;
    /** Light or dark theme */
    theme?: 'light' | 'dark';
    /** Expanded state */
    expanded?: boolean;
    /** Interactive spotlight on mouse move */
    interactive?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Header slot content */
    header?: Snippet;
    /** Footer slot content */
    footer?: Snippet;
    /** Default slot content */
    children?: Snippet;
  }

  let {
    cms = {},
    color = 'teal',
    glass = 'medium',
    glow = false,
    border = 'subtle',
    hoverEffect = false,
    hover = 'none',
    theme = 'dark',
    expanded = false,
    interactive = false,
    class: className = '',
    header,
    footer,
    children
  }: Props = $props();

  // Mouse tracking state for interactive spotlight
  let cardElement: HTMLDivElement | undefined = $state();
  let mouseX = $state(50);
  let mouseY = $state(50);
  let isHovering = $state(false);

  // Glass effect classes
  const glassClasses = $derived.by(() => {
    if (!glass) return '';
    const effects: Record<string, string> = {
      subtle: 'backdrop-blur-sm bg-slate-900/5',
      medium: 'backdrop-blur-md bg-slate-900/10',
      frosted: 'backdrop-blur-xl bg-slate-900/15',
      solid: 'backdrop-blur-2xl bg-slate-900/20'
    };
    return typeof glass === 'boolean'
      ? effects.medium
      : effects[glass] || effects.medium;
  });

  // Glow effect classes
  const glowClasses = $derived.by(() => {
    if (!glow) return '';
    const colorMap: Record<ColorPalette, string> = {
      teal: 'shadow-[0_0_20px_hsl(var(--teal-500)/0.25)]',
      coral: 'shadow-[0_0_20px_hsl(var(--coral-500)/0.25)]',
      purple: 'shadow-[0_0_20px_hsl(var(--purple-500)/0.25)]',
      pink: 'shadow-[0_0_20px_hsl(var(--pink-500)/0.25)]',
      mint: 'shadow-[0_0_20px_hsl(var(--mint-500)/0.25)]',
      amber: 'shadow-[0_0_20px_hsl(var(--amber-500)/0.25)]',
      slate: 'shadow-[0_0_20px_hsl(var(--slate-500)/0.25)]',
      ghost: ''
    };
    if (glow === 'pulse') {
      return `${colorMap[color]} animate-pulse`;
    }
    return colorMap[color];
  });

  // Border effect classes
  const borderClasses = $derived.by(() => {
    if (border === 'none' || !border) return 'border-0';
    const colorMap: Record<ColorPalette, string> = {
      teal: 'border-teal-500/20',
      coral: 'border-coral-500/20',
      purple: 'border-purple-500/20',
      pink: 'border-pink-500/20',
      mint: 'border-mint-500/20',
      amber: 'border-amber-500/20',
      slate: 'border-slate-500/20',
      ghost: 'border-white/10'
    };
    const intensity: Record<string, string> = {
      subtle: '10',
      medium: '20',
      strong: '30',
      glow: '30'
    };
    const effect = typeof border === 'boolean' ? 'subtle' : border;
    return `border border-solid ${colorMap[color].replace('/20', `/${intensity[effect] || '20'}`)}`;
  });

  // Hover effect classes
  const hoverClasses = $derived.by(() => {
    if (!hoverEffect && hover === 'none') return '';
    const effects: Record<string, string> = {
      lift: 'hover:-translate-y-1 hover:shadow-xl',
      glow: 'hover:shadow-[0_0_30px_hsl(var(--teal-500)/0.4)]',
      scale: 'hover:scale-[1.02]',
      brighten: 'hover:brightness-110',
      border: 'hover:border-opacity-50'
    };
    if (hoverEffect) return 'hover:-translate-y-1 hover:scale-[1.01] hover:shadow-xl';
    return effects[hover] || '';
  });

  // Theme classes
  const themeClasses = $derived(
    theme === 'dark'
      ? 'bg-slate-900/40 text-white'
      : 'bg-white/80 text-slate-900'
  );

  // Combined classes
  const cardClasses = $derived(
    `relative p-6 overflow-hidden rounded-2xl transition-all duration-300 ${themeClasses} ${glassClasses} ${glowClasses} ${borderClasses} ${hoverClasses} ${className}`
  );

  // Mouse handlers for interactive spotlight
  function handleMouseMove(event: MouseEvent) {
    if (!interactive || !cardElement) return;

    const rect = cardElement.getBoundingClientRect();
    mouseX = ((event.clientX - rect.left) / rect.width) * 100;
    mouseY = ((event.clientY - rect.top) / rect.height) * 100;
  }

  function handleMouseEnter() {
    if (interactive) isHovering = true;
  }

  function handleMouseLeave() {
    if (interactive) isHovering = false;
  }

  // CSS variables for spotlight
  const spotlightStyle = $derived(
    interactive
      ? `--mouse-x: ${mouseX}%; --mouse-y: ${mouseY}%;`
      : ''
  );
</script>

<div
  bind:this={cardElement}
  class={cardClasses}
  style={spotlightStyle}
  onmousemove={handleMouseMove}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  data-expanded={expanded}
  data-color={color}
>
  <!-- Glow layer (before pseudo-element equivalent) -->
  {#if glow && glow !== 'pulse'}
    <div
      class="absolute -inset-1 rounded-full opacity-50 blur-3xl -z-10 pointer-events-none
        {color === 'teal' ? 'bg-teal-500/30' : ''}
        {color === 'coral' ? 'bg-coral-500/30' : ''}
        {color === 'purple' ? 'bg-purple-500/30' : ''}
        {color === 'pink' ? 'bg-pink-500/30' : ''}
        {color === 'mint' ? 'bg-mint-500/30' : ''}
        {color === 'amber' ? 'bg-amber-500/30' : ''}
        {color === 'slate' ? 'bg-slate-500/30' : ''}"
    ></div>
  {/if}

  <!-- Interactive spotlight overlay -->
  {#if interactive}
    <div
      class="absolute inset-0 pointer-events-none transition-opacity duration-300 rounded-2xl"
      class:opacity-0={!isHovering}
      class:opacity-100={isHovering}
      style="
        background: radial-gradient(
          circle at var(--mouse-x) var(--mouse-y),
          hsl(var(--{color}-500) / 0.15) 0%,
          transparent 60%
        );
        mix-blend-mode: soft-light;
      "
    ></div>
  {/if}

  <!-- Header slot -->
  {#if header}
    <header class="mb-4 pb-4 border-b border-slate-700/50 flex items-center gap-2">
      {@render header()}
    </header>
  {/if}

  <!-- Default content slot -->
  {#if children}
    <div class="card-content">
      {@render children()}
    </div>
  {/if}

  <!-- Footer slot -->
  {#if footer}
    <footer class="mt-4 pt-4 border-t border-slate-700/50">
      {@render footer()}
    </footer>
  {/if}
</div>

<style>
  /* Card hover glow enhancement by color */
  [data-color="teal"]:hover {
    box-shadow:
      0 10px 30px rgba(0, 0, 0, 0.2),
      0 0 20px hsl(var(--teal-500) / 0.2),
      0 0 0 1px hsl(var(--teal-500) / 0.3);
  }

  [data-color="coral"]:hover {
    box-shadow:
      0 10px 30px rgba(0, 0, 0, 0.2),
      0 0 20px hsl(var(--coral-500) / 0.2),
      0 0 0 1px hsl(var(--coral-500) / 0.3);
  }

  [data-color="purple"]:hover {
    box-shadow:
      0 10px 30px rgba(0, 0, 0, 0.2),
      0 0 20px hsl(var(--purple-500) / 0.2),
      0 0 0 1px hsl(var(--purple-500) / 0.3);
  }

  [data-color="mint"]:hover {
    box-shadow:
      0 10px 30px rgba(0, 0, 0, 0.2),
      0 0 20px hsl(var(--mint-500) / 0.2),
      0 0 0 1px hsl(var(--mint-500) / 0.3);
  }
</style>
