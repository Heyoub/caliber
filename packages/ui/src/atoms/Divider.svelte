<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ColorToken, Spacing } from '../types';

  /**
   * Divider - Visual separator
   */
  interface Props {
    /** Orientation */
    orientation?: 'horizontal' | 'vertical';
    /** Color token */
    color?: ColorToken;
    /** Spacing around divider */
    spacing?: Spacing;
    /** Label text or slot */
    label?: string;
    /** Additional CSS classes */
    class?: string;
    /** Label slot content */
    children?: Snippet;
  }

  let {
    orientation = 'horizontal',
    color = 'slate',
    spacing = 4,
    label,
    class: className = '',
    children,
  }: Props = $props();

  // Color configurations
  const colorConfigs: Record<string, string> = {
    teal: 'border-[hsl(var(--teal-500)/_0.3)]',
    coral: 'border-[hsl(var(--coral-500)/_0.3)]',
    purple: 'border-[hsl(var(--purple-500)/_0.3)]',
    pink: 'border-[hsl(var(--pink-500)/_0.3)]',
    mint: 'border-[hsl(var(--mint-500)/_0.3)]',
    amber: 'border-[hsl(var(--amber-500)/_0.3)]',
    slate: 'border-[hsl(var(--slate-700))]',
    ghost: 'border-[hsl(var(--slate-800))]',
  };

  // Spacing configurations
  const spacingClasses: Record<Spacing, { horizontal: string; vertical: string }> = {
    0: { horizontal: 'my-0', vertical: 'mx-0' },
    1: { horizontal: 'my-1', vertical: 'mx-1' },
    2: { horizontal: 'my-2', vertical: 'mx-2' },
    3: { horizontal: 'my-3', vertical: 'mx-3' },
    4: { horizontal: 'my-4', vertical: 'mx-4' },
    5: { horizontal: 'my-5', vertical: 'mx-5' },
    6: { horizontal: 'my-6', vertical: 'mx-6' },
    8: { horizontal: 'my-8', vertical: 'mx-8' },
    10: { horizontal: 'my-10', vertical: 'mx-10' },
    12: { horizontal: 'my-12', vertical: 'mx-12' },
    16: { horizontal: 'my-16', vertical: 'mx-16' },
    20: { horizontal: 'my-20', vertical: 'mx-20' },
    24: { horizontal: 'my-24', vertical: 'mx-24' },
  };

  // Derived values
  let baseColor = $derived(color.split('-')[0] as string);
  let colorClass = $derived(colorConfigs[baseColor] || colorConfigs.slate);
  let spacingClass = $derived(spacingClasses[spacing][orientation]);

  // Check if has content
  let hasContent = $derived(!!label || !!children);
</script>

{#if orientation === 'horizontal'}
  {#if hasContent}
    <div class="flex items-center {spacingClass} {className}">
      <div class="flex-1 border-t {colorClass}"></div>
      <span class="px-3 text-xs text-[hsl(var(--slate-500))] uppercase tracking-wider font-medium">
        {#if children}
          {@render children()}
        {:else}
          {label}
        {/if}
      </span>
      <div class="flex-1 border-t {colorClass}"></div>
    </div>
  {:else}
    <hr class="border-t {colorClass} {spacingClass} {className}" />
  {/if}
{:else}
  <div
    class="inline-block border-l h-full min-h-[1rem] {colorClass} {spacingClass} {className}"
    role="separator"
    aria-orientation="vertical"
  ></div>
{/if}
