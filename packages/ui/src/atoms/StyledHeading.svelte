<script lang="ts">
  /**
   * StyledHeading - Gradient text headings with glow effects
   * Reference: Vue StyledHeading.vue, CTOLandingPage.vue
   */
  import type { Snippet } from 'svelte';
  import type { ColorToken, Size } from '../types';

  type HeadingLevel = 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6';

  interface Props {
    /** Heading level */
    level?: HeadingLevel;
    /** Text size */
    size?: Size | 'display';
    /** Gradient from color */
    from?: ColorToken;
    /** Gradient via color (optional) */
    via?: ColorToken;
    /** Gradient to color */
    to?: ColorToken;
    /** Enable glow effect */
    glow?: boolean;
    /** Glow intensity */
    glowIntensity?: 'subtle' | 'medium' | 'intense';
    /** Enable gradient animation */
    animate?: boolean;
    /** Text alignment */
    align?: 'left' | 'center' | 'right';
    /** Additional CSS classes */
    class?: string;
    /** Children content */
    children?: Snippet;
  }

  let {
    level = 'h2',
    size = 'lg',
    from = 'teal',
    via,
    to = 'purple',
    glow = false,
    glowIntensity = 'medium',
    animate = false,
    align = 'left',
    class: className = '',
    children,
  }: Props = $props();

  // Size mappings
  const sizeClasses: Record<Size | 'display', string> = {
    xs: 'text-sm md:text-base',
    sm: 'text-base md:text-lg',
    md: 'text-lg md:text-xl',
    lg: 'text-xl md:text-2xl lg:text-3xl',
    xl: 'text-2xl md:text-3xl lg:text-4xl',
    '2xl': 'text-3xl md:text-4xl lg:text-5xl',
    display: 'text-4xl md:text-5xl lg:text-6xl xl:text-7xl',
  };

  // Glow intensity mappings
  const glowClasses: Record<string, string> = {
    subtle: 'drop-shadow-[0_0_10px_var(--glow-color)]',
    medium: 'drop-shadow-[0_0_20px_var(--glow-color)] drop-shadow-[0_0_40px_var(--glow-color)]',
    intense: 'drop-shadow-[0_0_30px_var(--glow-color)] drop-shadow-[0_0_60px_var(--glow-color)] drop-shadow-[0_0_90px_var(--glow-color)]',
  };

  // Alignment classes
  const alignClasses: Record<string, string> = {
    left: 'text-left',
    center: 'text-center',
    right: 'text-right',
  };

  // Color to HSL variable mapping
  function colorToVar(color: ColorToken): string {
    return `hsl(var(--${color}))`;
  }

  let gradientStyle = $derived(() => {
    const fromColor = colorToVar(from);
    const toColor = colorToVar(to);
    const viaColor = via ? colorToVar(via) : '';

    if (viaColor) {
      return `linear-gradient(135deg, ${fromColor} 0%, ${viaColor} 50%, ${toColor} 100%)`;
    }
    return `linear-gradient(135deg, ${fromColor} 0%, ${toColor} 100%)`;
  });

  let glowColor = $derived(colorToVar(from));
</script>

{#if level === 'h1'}
  <h1
    class="
      font-bold font-title tracking-tight
      bg-clip-text text-transparent bg-gradient-to-r
      {sizeClasses[size]}
      {alignClasses[align]}
      {glow ? glowClasses[glowIntensity] : ''}
      {animate ? 'animate-gradient-flow bg-[length:200%_auto]' : ''}
      {className}
    "
    style="background-image: {gradientStyle()}; --glow-color: {glowColor};"
  >
    {#if children}
      {@render children()}
    {/if}
  </h1>
{:else if level === 'h2'}
  <h2
    class="
      font-bold font-title tracking-tight
      bg-clip-text text-transparent bg-gradient-to-r
      {sizeClasses[size]}
      {alignClasses[align]}
      {glow ? glowClasses[glowIntensity] : ''}
      {animate ? 'animate-gradient-flow bg-[length:200%_auto]' : ''}
      {className}
    "
    style="background-image: {gradientStyle()}; --glow-color: {glowColor};"
  >
    {#if children}
      {@render children()}
    {/if}
  </h2>
{:else if level === 'h3'}
  <h3
    class="
      font-bold font-title tracking-tight
      bg-clip-text text-transparent bg-gradient-to-r
      {sizeClasses[size]}
      {alignClasses[align]}
      {glow ? glowClasses[glowIntensity] : ''}
      {animate ? 'animate-gradient-flow bg-[length:200%_auto]' : ''}
      {className}
    "
    style="background-image: {gradientStyle()}; --glow-color: {glowColor};"
  >
    {#if children}
      {@render children()}
    {/if}
  </h3>
{:else if level === 'h4'}
  <h4
    class="
      font-bold font-title tracking-tight
      bg-clip-text text-transparent bg-gradient-to-r
      {sizeClasses[size]}
      {alignClasses[align]}
      {glow ? glowClasses[glowIntensity] : ''}
      {animate ? 'animate-gradient-flow bg-[length:200%_auto]' : ''}
      {className}
    "
    style="background-image: {gradientStyle()}; --glow-color: {glowColor};"
  >
    {#if children}
      {@render children()}
    {/if}
  </h4>
{:else if level === 'h5'}
  <h5
    class="
      font-bold font-title tracking-tight
      bg-clip-text text-transparent bg-gradient-to-r
      {sizeClasses[size]}
      {alignClasses[align]}
      {glow ? glowClasses[glowIntensity] : ''}
      {animate ? 'animate-gradient-flow bg-[length:200%_auto]' : ''}
      {className}
    "
    style="background-image: {gradientStyle()}; --glow-color: {glowColor};"
  >
    {#if children}
      {@render children()}
    {/if}
  </h5>
{:else}
  <h6
    class="
      font-bold font-title tracking-tight
      bg-clip-text text-transparent bg-gradient-to-r
      {sizeClasses[size]}
      {alignClasses[align]}
      {glow ? glowClasses[glowIntensity] : ''}
      {animate ? 'animate-gradient-flow bg-[length:200%_auto]' : ''}
      {className}
    "
    style="background-image: {gradientStyle()}; --glow-color: {glowColor};"
  >
    {#if children}
      {@render children()}
    {/if}
  </h6>
{/if}

<style>
  @keyframes gradient-flow {
    0% {
      background-position: 0% 50%;
    }
    50% {
      background-position: 100% 50%;
    }
    100% {
      background-position: 0% 50%;
    }
  }

  .animate-gradient-flow {
    animation: gradient-flow 3s ease infinite;
  }
</style>
