<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ColorToken, Size, GlowEffect, GlassEffect, HoverEffect, PressEffect } from '../types';

  /**
   * Button Props Interface
   */
  interface Props {
    /** Color token for the button */
    color?: ColorToken;
    /** Size variant */
    size?: Size;
    /** Glow effect intensity */
    glow?: GlowEffect;
    /** Glass morphism effect */
    glass?: GlassEffect;
    /** Hover effect type */
    hover?: HoverEffect;
    /** Press effect type */
    press?: PressEffect;
    /** Loading state - shows spinner */
    loading?: boolean;
    /** Disabled state */
    disabled?: boolean;
    /** Force pressed visual state */
    forcePressed?: boolean;
    /** Full width button */
    fullWidth?: boolean;
    /** Button type for form submission */
    type?: 'button' | 'submit' | 'reset';
    /** Optional href - renders as anchor */
    href?: string;
    /** Hamburger menu variant - animates to X */
    hamburger?: boolean;
    /** Hamburger open state */
    hamburgerOpen?: boolean;
    /** Enable blob/lava lamp animation */
    blob?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Click handler */
    onclick?: (event: MouseEvent) => void;
    /** Children content */
    children?: Snippet;
  }

  let {
    color = 'teal',
    size = 'md',
    glow = false,
    glass = false,
    hover = 'lift',
    press = 'sink',
    loading = false,
    disabled = false,
    forcePressed = false,
    fullWidth = false,
    type = 'button',
    href,
    hamburger = false,
    hamburgerOpen = false,
    blob = false,
    class: className = '',
    onclick,
    children,
  }: Props = $props();

  // Reactive state
  let isPressed = $state(false);
  let isHovered = $state(false);

  // Color configuration map
  const colorConfigs: Record<string, { bg: string; hover: string; text: string; border: string; glow: string; outerGlow: string; gradient: string; pressGradient: string }> = {
    teal: {
      bg: 'bg-[hsl(var(--teal-500))]',
      hover: 'hover:bg-[hsl(var(--teal-600))]',
      text: 'text-white',
      border: 'border-[hsl(var(--teal-700))]',
      glow: 'hsl(var(--teal-400))',
      outerGlow: 'rgba(45, 212, 191, 0.4)',
      gradient: 'from-[hsl(var(--teal-300))] via-[hsl(var(--teal-500))] to-[hsl(var(--teal-700))]',
      pressGradient: 'from-[hsl(var(--teal-700))] via-[hsl(var(--teal-500))] to-[hsl(var(--teal-300))]',
    },
    coral: {
      bg: 'bg-[hsl(var(--coral-500))]',
      hover: 'hover:bg-[hsl(var(--coral-600))]',
      text: 'text-white',
      border: 'border-[hsl(var(--coral-300))]',
      glow: 'hsl(var(--coral-400))',
      outerGlow: 'rgba(255, 129, 112, 0.4)',
      gradient: 'from-[hsl(var(--coral-300))] via-[hsl(var(--coral-500))] to-[hsl(var(--coral-700))]',
      pressGradient: 'from-[hsl(var(--coral-700))] via-[hsl(var(--coral-500))] to-[hsl(var(--coral-300))]',
    },
    purple: {
      bg: 'bg-[hsl(var(--purple-500))]',
      hover: 'hover:bg-[hsl(var(--purple-600))]',
      text: 'text-white',
      border: 'border-[hsl(var(--purple-700))]',
      glow: 'hsl(var(--purple-400))',
      outerGlow: 'rgba(167, 139, 250, 0.4)',
      gradient: 'from-[hsl(var(--purple-300))] via-[hsl(var(--purple-500))] to-[hsl(var(--purple-700))]',
      pressGradient: 'from-[hsl(var(--purple-700))] via-[hsl(var(--purple-500))] to-[hsl(var(--purple-300))]',
    },
    pink: {
      bg: 'bg-[hsl(var(--pink-500))]',
      hover: 'hover:bg-[hsl(var(--pink-600))]',
      text: 'text-white',
      border: 'border-[hsl(var(--pink-700))]',
      glow: 'hsl(var(--pink-400))',
      outerGlow: 'rgba(236, 72, 153, 0.4)',
      gradient: 'from-[hsl(var(--pink-300))] via-[hsl(var(--pink-500))] to-[hsl(var(--pink-700))]',
      pressGradient: 'from-[hsl(var(--pink-700))] via-[hsl(var(--pink-500))] to-[hsl(var(--pink-300))]',
    },
    mint: {
      bg: 'bg-[hsl(var(--mint-500))]',
      hover: 'hover:bg-[hsl(var(--mint-600))]',
      text: 'text-white',
      border: 'border-[hsl(var(--mint-700))]',
      glow: 'hsl(var(--mint-400))',
      outerGlow: 'rgba(110, 231, 183, 0.4)',
      gradient: 'from-[hsl(var(--mint-300))] via-[hsl(var(--mint-500))] to-[hsl(var(--mint-700))]',
      pressGradient: 'from-[hsl(var(--mint-700))] via-[hsl(var(--mint-500))] to-[hsl(var(--mint-300))]',
    },
    amber: {
      bg: 'bg-[hsl(var(--amber-500))]',
      hover: 'hover:bg-[hsl(var(--amber-600))]',
      text: 'text-white',
      border: 'border-[hsl(var(--amber-700))]',
      glow: 'hsl(var(--amber-400))',
      outerGlow: 'rgba(251, 191, 36, 0.4)',
      gradient: 'from-[hsl(var(--amber-300))] via-[hsl(var(--amber-500))] to-[hsl(var(--amber-700))]',
      pressGradient: 'from-[hsl(var(--amber-700))] via-[hsl(var(--amber-500))] to-[hsl(var(--amber-300))]',
    },
    slate: {
      bg: 'bg-[hsl(var(--slate-700))]',
      hover: 'hover:bg-[hsl(var(--slate-600))]',
      text: 'text-[hsl(var(--slate-100))]',
      border: 'border-[hsl(var(--slate-500))]',
      glow: 'hsl(var(--slate-400))',
      outerGlow: 'rgba(51, 65, 85, 0.4)',
      gradient: 'from-[hsl(var(--slate-500))] via-[hsl(var(--slate-700))] to-[hsl(var(--slate-900))]',
      pressGradient: 'from-[hsl(var(--slate-900))] via-[hsl(var(--slate-700))] to-[hsl(var(--slate-500))]',
    },
    ghost: {
      bg: 'bg-transparent',
      hover: 'hover:bg-[hsl(var(--slate-800))]',
      text: 'text-[hsl(var(--slate-300))]',
      border: 'border-transparent',
      glow: 'transparent',
      outerGlow: 'transparent',
      gradient: 'from-transparent via-[hsl(var(--slate-800)_/_0.5)] to-transparent',
      pressGradient: 'from-transparent via-[hsl(var(--slate-700)_/_0.5)] to-transparent',
    },
  };

  // Derived values
  let baseColor = $derived(color.split('-')[0] as string);
  let colorConfig = $derived(colorConfigs[baseColor] || colorConfigs.teal);

  // Size classes
  const sizeClasses: Record<Size, string> = {
    xs: 'px-2 py-1 text-xs',
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-6 py-3 text-lg',
    xl: 'px-8 py-4 text-xl',
    '2xl': 'px-10 py-5 text-2xl',
  };

  // Hamburger specific size classes
  const hamburgerSizeClasses: Record<Size, string> = {
    xs: 'w-8 h-8',
    sm: 'w-10 h-10',
    md: 'w-12 h-12',
    lg: 'w-14 h-14',
    xl: 'w-16 h-16',
    '2xl': 'w-20 h-20',
  };

  // Glow classes
  let glowClass = $derived(() => {
    if (!glow) return '';
    if (glow === true) return 'glow-default';
    return `glow-${glow}`;
  });

  // Glass classes
  let glassClass = $derived(() => {
    if (!glass) return '';
    if (glass === true) return 'glass-default';
    return `glass-${glass}`;
  });

  // Hover effect classes
  const hoverClasses: Record<HoverEffect, string> = {
    none: '',
    lift: 'hover:-translate-y-0.5',
    glow: 'hover-glow',
    scale: 'hover:scale-[1.02]',
    brighten: 'hover:brightness-110',
    border: 'hover:border-opacity-100',
  };

  // Press effect classes
  const pressClasses: Record<PressEffect, string> = {
    none: '',
    sink: 'active:translate-y-0.5',
    scale: 'active:scale-[0.98]',
    darken: 'active:brightness-90',
  };

  // Computed classes
  let computedClasses = $derived([
    'relative inline-flex items-center justify-center',
    'font-medium rounded-lg',
    'transition-all duration-300',
    'cursor-pointer',
    'overflow-hidden',
    'border-b-4',
    hamburger ? hamburgerSizeClasses[size] : sizeClasses[size],
    hamburger ? 'flex-col gap-1.5' : '',
    colorConfig.bg,
    colorConfig.hover,
    colorConfig.text,
    colorConfig.border,
    hoverClasses[hover],
    pressClasses[press],
    fullWidth ? 'w-full' : '',
    disabled ? 'opacity-50 cursor-not-allowed pointer-events-none' : '',
    loading ? 'opacity-70 pointer-events-none' : '',
    forcePressed ? 'translate-y-1 border-b-2' : 'active:border-b-2',
    glowClass(),
    glassClass(),
    className,
  ].filter(Boolean).join(' '));

  // Hamburger line styles
  let line1Transform = $derived(hamburgerOpen ? 'rotate(45deg) translateY(0.375rem)' : 'none');
  let line2Transform = $derived(hamburgerOpen ? 'rotate(-45deg) translateY(-0.375rem)' : 'none');

  // Determine tag
  let isLink = $derived(!!href);
</script>

{#if isLink}
  <a
    {href}
    class={computedClasses}
    style="--glow-color: {colorConfig.glow}; --outer-glow-color: {colorConfig.outerGlow};"
    data-pressed={forcePressed}
    aria-disabled={disabled}
  >
    <!-- Outer glow effect -->
    <div class="absolute -inset-px rounded-lg opacity-15 transition-all duration-500 ease-out -z-10 outer-glow-effect" class:forced-glow={forcePressed}></div>

    <!-- Border highlight -->
    <div class={`absolute inset-0 border border-white/20 group-hover:border-white/50 rounded-lg transition-all duration-300 z-[1] ${forcePressed ? 'border-white/60' : ''}`}></div>

    <!-- Animated gradient background -->
    <div class="absolute inset-0 bg-gradient-to-br transition-all duration-700 ease-out z-[4] opacity-0 group-hover:opacity-75 {colorConfig.gradient}" class:opacity-60={forcePressed}></div>

    <!-- Pressed gradient effect -->
    <div class="absolute inset-0 bg-gradient-to-tr transition-all duration-300 ease-out z-[5] opacity-0 active:opacity-60 {colorConfig.pressGradient}" class:opacity-60={forcePressed}></div>

    <!-- Content -->
    <span class="relative z-10 flex items-center gap-2 transition-transform duration-150" class:translate-y-px={forcePressed}>
      {#if loading}
        <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
      {/if}
      {#if children}
        {@render children()}
      {/if}
    </span>
  </a>
{:else}
  <button
    {type}
    {disabled}
    class={computedClasses}
    style="--glow-color: {colorConfig.glow}; --outer-glow-color: {colorConfig.outerGlow};"
    data-pressed={forcePressed}
    aria-pressed={forcePressed}
    aria-expanded={hamburger ? hamburgerOpen : undefined}
    onclick={onclick}
    onmouseenter={() => isHovered = true}
    onmouseleave={() => isHovered = false}
    onmousedown={() => isPressed = true}
    onmouseup={() => isPressed = false}
  >
    {#if hamburger}
      <!-- Hamburger lines -->
      <span
        class="block w-5 h-0.5 bg-current transition-transform duration-300 origin-center"
        style="transform: {line1Transform};"
      ></span>
      <span
        class="block w-5 h-0.5 bg-current transition-transform duration-300 origin-center"
        style="transform: {line2Transform};"
      ></span>
    {:else}
      <!-- Outer glow effect -->
      <div class="absolute -inset-px rounded-lg opacity-15 transition-all duration-500 ease-out -z-10 outer-glow-effect" class:forced-glow={forcePressed}></div>

      <!-- Blob/Lava lamp animation -->
      {#if blob}
        <div class="absolute inset-0 overflow-hidden rounded-lg z-[2] pointer-events-none">
          <div class="blob-container">
            <div class="blob blob-1" style="--blob-color: {colorConfig.glow};"></div>
            <div class="blob blob-2" style="--blob-color: {colorConfig.glow};"></div>
            <div class="blob blob-3" style="--blob-color: {colorConfig.glow};"></div>
          </div>
        </div>
      {/if}

      <!-- Border highlight -->
      <div class={`absolute inset-0 border border-white/20 hover:border-white/50 rounded-lg transition-all duration-300 z-[1] ${forcePressed ? 'border-white/60' : ''}`}></div>

      <!-- Animated gradient background -->
      <div class="absolute inset-0 bg-gradient-to-br transition-all duration-700 ease-out z-[4] opacity-0 hover:opacity-75 {colorConfig.gradient}" class:opacity-60={forcePressed}></div>

      <!-- Pressed gradient effect -->
      <div class="absolute inset-0 bg-gradient-to-tr transition-all duration-300 ease-out z-[5] opacity-0 active:opacity-60 {colorConfig.pressGradient}" class:opacity-60={forcePressed}></div>

      <!-- Glassmorphic bottom effect -->
      <div class="absolute bottom-0 left-0 right-0 glassmorphic-gradient transition-all duration-300 ease-in-out rounded-lg z-[6] opacity-0 h-0 hover:opacity-100 hover:h-full" class:hidden={forcePressed}></div>

      <!-- Glassmorphic top effect (pressed) -->
      <div class="absolute top-0 left-0 right-0 glassmorphic-gradient-reverse transition-all duration-200 ease-out rounded-lg z-[7] opacity-0 h-0 active:opacity-100 active:h-[70%]" class:opacity-100={forcePressed} class:h-[70%]={forcePressed}></div>

      <!-- Content -->
      <span class="relative z-10 flex items-center gap-2 transition-transform duration-150" class:translate-y-px={forcePressed || isPressed}>
        {#if loading}
          <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
        {/if}
        {#if children}
          {@render children()}
        {/if}
      </span>
    {/if}
  </button>
{/if}

<style>
  /* Glow effects */
  .glow-default {
    box-shadow: 0 0 20px hsl(var(--component-color, var(--teal-500)) / 0.25);
  }
  .glow-subtle {
    box-shadow: 0 0 10px hsl(var(--component-color, var(--teal-500)) / 0.1);
  }
  .glow-medium {
    box-shadow: 0 0 20px hsl(var(--component-color, var(--teal-500)) / 0.25);
  }
  .glow-intense {
    box-shadow: 0 0 30px hsl(var(--component-color, var(--teal-500)) / 0.5);
  }
  .glow-pulse {
    animation: glow-pulse 2s ease-in-out infinite;
  }

  /* Glass effects */
  .glass-default,
  .glass-medium {
    backdrop-filter: blur(12px);
    background: hsl(var(--slate-900) / 0.8);
  }
  .glass-subtle {
    backdrop-filter: blur(4px);
    background: hsl(var(--slate-900) / 0.5);
  }
  .glass-frosted {
    backdrop-filter: blur(20px);
    background: hsl(var(--slate-900) / 0.85);
  }
  .glass-solid {
    backdrop-filter: blur(40px);
    background: hsl(var(--slate-900) / 0.9);
  }

  /* Outer glow effect */
  .outer-glow-effect {
    box-shadow: 0 0 12px 3px var(--outer-glow-color);
  }
  button:hover .outer-glow-effect,
  a:hover .outer-glow-effect {
    box-shadow: 0 0 18px 5px var(--outer-glow-color), 0 0 30px 10px var(--outer-glow-color);
    opacity: 0.3;
  }
  .forced-glow {
    box-shadow: 0 0 15px 4px var(--outer-glow-color), 0 0 25px 8px var(--outer-glow-color);
    opacity: 0.35 !important;
  }

  /* Glassmorphic gradients */
  .glassmorphic-gradient {
    background: linear-gradient(to top, rgba(0, 0, 0, 0.25) 0%, rgba(0, 0, 0, 0.15) 40%, transparent 100%);
    backdrop-filter: blur(3px);
  }
  .glassmorphic-gradient-reverse {
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0.3) 0%, rgba(0, 0, 0, 0.15) 40%, transparent 100%);
    backdrop-filter: blur(3px);
  }

  /* Hover glow effect */
  .hover-glow:hover {
    --glow-opacity: 0.4;
  }

  /* Animations */
  @keyframes glow-pulse {
    0%, 100% {
      box-shadow: 0 0 20px hsl(var(--component-color, var(--teal-500)) / 0.25);
    }
    50% {
      box-shadow: 0 0 30px hsl(var(--component-color, var(--teal-500)) / 0.5);
    }
  }

  /* Focus styles */
  button:focus-visible,
  a:focus-visible {
    outline: 2px solid var(--glow-color);
    outline-offset: 2px;
  }

  /* Blob/Lava lamp animation */
  .blob-container {
    position: absolute;
    inset: -50%;
    width: 200%;
    height: 200%;
    filter: blur(20px);
    opacity: 0.6;
  }

  .blob {
    position: absolute;
    width: 40%;
    height: 40%;
    border-radius: 50%;
    background: radial-gradient(circle, var(--blob-color) 0%, transparent 70%);
    animation: blob-float 8s ease-in-out infinite;
  }

  .blob-1 {
    top: 20%;
    left: 20%;
    animation-delay: 0s;
  }

  .blob-2 {
    top: 40%;
    right: 20%;
    animation-delay: -2.5s;
    animation-duration: 10s;
  }

  .blob-3 {
    bottom: 20%;
    left: 40%;
    animation-delay: -5s;
    animation-duration: 12s;
  }

  @keyframes blob-float {
    0%, 100% {
      transform: translate(0, 0) scale(1);
    }
    25% {
      transform: translate(20%, -20%) scale(1.1);
    }
    50% {
      transform: translate(-10%, 20%) scale(0.9);
    }
    75% {
      transform: translate(-20%, -10%) scale(1.05);
    }
  }

  button:hover .blob-container,
  a:hover .blob-container {
    opacity: 0.8;
  }

  button:active .blob-container,
  a:active .blob-container {
    opacity: 1;
  }
</style>
