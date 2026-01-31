<script lang="ts">
  import type { Size, ColorToken } from '../types';

  /**
   * Spinner - Loading indicator
   */
  interface Props {
    /** Size variant */
    size?: Size;
    /** Color token */
    color?: ColorToken;
    /** Label for accessibility */
    label?: string;
    /** Additional CSS classes */
    class?: string;
  }

  let {
    size = 'md',
    color = 'teal',
    label = 'Loading',
    class: className = '',
  }: Props = $props();

  // Size configurations
  const sizeConfigs: Record<Size, { spinner: string; stroke: number }> = {
    xs: { spinner: 'w-3 h-3', stroke: 3 },
    sm: { spinner: 'w-4 h-4', stroke: 3 },
    md: { spinner: 'w-6 h-6', stroke: 2.5 },
    lg: { spinner: 'w-8 h-8', stroke: 2 },
    xl: { spinner: 'w-10 h-10', stroke: 2 },
    '2xl': { spinner: 'w-12 h-12', stroke: 1.5 },
  };

  // Color configurations
  const colorConfigs: Record<string, string> = {
    teal: 'text-[hsl(var(--teal-400))]',
    coral: 'text-[hsl(var(--coral-400))]',
    purple: 'text-[hsl(var(--purple-400))]',
    pink: 'text-[hsl(var(--pink-400))]',
    mint: 'text-[hsl(var(--mint-400))]',
    amber: 'text-[hsl(var(--amber-400))]',
    slate: 'text-[hsl(var(--slate-400))]',
    ghost: 'text-current',
  };

  // Derived values
  let baseColor = $derived(color.split('-')[0] as string);
  let sizeConfig = $derived(sizeConfigs[size]);
  let colorClass = $derived(colorConfigs[baseColor] || colorConfigs.teal);

  // Computed classes
  let computedClasses = $derived([
    'animate-spin',
    sizeConfig.spinner,
    colorClass,
    className,
  ].filter(Boolean).join(' '));
</script>

<svg
  class={computedClasses}
  viewBox="0 0 24 24"
  fill="none"
  aria-label={label}
  role="status"
>
  <!-- Background circle -->
  <circle
    cx="12"
    cy="12"
    r="10"
    stroke="currentColor"
    stroke-width={sizeConfig.stroke}
    class="opacity-20"
  />
  <!-- Animated arc -->
  <path
    d="M12 2a10 10 0 0 1 10 10"
    stroke="currentColor"
    stroke-width={sizeConfig.stroke}
    stroke-linecap="round"
  />
</svg>
