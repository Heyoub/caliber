<script lang="ts">
  /**
   * HamburgerButton - Animated hamburger menu toggle
   * Animates between hamburger and X states
   * Reference: Vue AuthLayout.vue mobile menu button
   */
  import type { ColorToken, Size } from '../types';

  interface Props {
    /** Open state - controls animation */
    open?: boolean;
    /** Size of the button */
    size?: Size;
    /** Color token */
    color?: ColorToken;
    /** Additional CSS classes */
    class?: string;
    /** Click handler */
    onclick?: (event: MouseEvent) => void;
    /** Aria label */
    ariaLabel?: string;
  }

  let {
    open = false,
    size = 'md',
    color = 'teal',
    class: className = '',
    onclick,
    ariaLabel = 'Toggle menu',
  }: Props = $props();

  // Size mappings
  const sizeConfig: Record<Size, { button: string; line: string; gap: string }> = {
    xs: { button: 'w-8 h-8', line: 'w-4 h-[2px]', gap: 'gap-1' },
    sm: { button: 'w-10 h-10', line: 'w-5 h-[2px]', gap: 'gap-1.5' },
    md: { button: 'w-12 h-12', line: 'w-5 h-[2px]', gap: 'gap-[0.5rem]' },
    lg: { button: 'w-14 h-14', line: 'w-6 h-[2px]', gap: 'gap-2' },
    xl: { button: 'w-16 h-16', line: 'w-7 h-[2.5px]', gap: 'gap-2.5' },
    '2xl': { button: 'w-20 h-20', line: 'w-8 h-[3px]', gap: 'gap-3' },
  };

  // Color mappings
  const colorConfig: Record<string, { bg: string; border: string; shadow: string; line: string }> = {
    teal: {
      bg: 'bg-[hsl(var(--teal-500)/0.2)] hover:bg-[hsl(var(--teal-500)/0.3)]',
      border: 'border-transparent hover:border-[hsl(var(--teal-500)/0.5)]',
      shadow: 'shadow-[0_0.25rem_0.75rem_rgba(79,209,197,0.25)] hover:shadow-[0_0.5rem_1.5rem_rgba(79,209,197,0.35)]',
      line: 'bg-slate-100',
    },
    coral: {
      bg: 'bg-[hsl(var(--coral-500)/0.2)] hover:bg-[hsl(var(--coral-500)/0.3)]',
      border: 'border-transparent hover:border-[hsl(var(--coral-500)/0.5)]',
      shadow: 'shadow-[0_0.25rem_0.75rem_rgba(255,129,112,0.25)] hover:shadow-[0_0.5rem_1.5rem_rgba(255,129,112,0.35)]',
      line: 'bg-slate-100',
    },
    purple: {
      bg: 'bg-[hsl(var(--purple-500)/0.2)] hover:bg-[hsl(var(--purple-500)/0.3)]',
      border: 'border-transparent hover:border-[hsl(var(--purple-500)/0.5)]',
      shadow: 'shadow-[0_0.25rem_0.75rem_rgba(167,139,250,0.25)] hover:shadow-[0_0.5rem_1.5rem_rgba(167,139,250,0.35)]',
      line: 'bg-slate-100',
    },
    slate: {
      bg: 'bg-slate-800/50 hover:bg-slate-700/50',
      border: 'border-slate-600/50 hover:border-slate-500/50',
      shadow: 'shadow-lg hover:shadow-xl',
      line: 'bg-slate-100',
    },
  };

  let config = $derived(sizeConfig[size]);
  let colors = $derived(colorConfig[color.split('-')[0]] || colorConfig.teal);

  // Animation transforms
  let line1Transform = $derived(open ? 'rotate(45deg) translateY(0.375rem)' : 'none');
  let line2Transform = $derived(open ? 'rotate(-45deg) translateY(-0.375rem)' : 'none');
</script>

<button
  type="button"
  class="
    flex flex-col items-center justify-center
    {config.button}
    {config.gap}
    {colors.bg}
    {colors.border}
    {colors.shadow}
    border rounded-lg
    transition-all duration-300
    group relative overflow-hidden
    focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-[hsl(var(--teal-500))]
    {className}
  "
  aria-expanded={open}
  aria-label={ariaLabel}
  aria-haspopup="true"
  {onclick}
>
  <!-- Hover glow effect -->
  <div class="absolute inset-0 bg-gradient-to-r from-transparent via-[hsl(var(--teal-500)/0.1)] to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 blur-xl pointer-events-none"></div>

  <!-- Animated lines -->
  <span
    class="block {config.line} {colors.line} transition-transform duration-300 origin-center"
    style="transform: {line1Transform};"
  ></span>
  <span
    class="block {config.line} {colors.line} transition-transform duration-300 origin-center"
    style="transform: {line2Transform};"
  ></span>

  <!-- Shine effect on hover -->
  <div class="absolute inset-0 bg-gradient-to-b from-white/0 via-slate-100/5 to-white/0 opacity-0 group-hover:opacity-100 transition-opacity"></div>

  <!-- Border glow on hover -->
  <div class="absolute inset-0 border border-[hsl(var(--teal-500)/0.3)] rounded-lg opacity-0 group-hover:opacity-100 transition-opacity scale-[1.02]"></div>
</button>
