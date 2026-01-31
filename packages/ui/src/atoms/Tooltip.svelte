<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { Snippet } from 'svelte';
  import type { Placement } from '../types';

  /**
   * Tooltip - Hover tooltip with positioning
   * Note: For full Floating UI support, install @floating-ui/dom
   */
  interface Props {
    /** Tooltip content text */
    content?: string;
    /** Tooltip placement */
    placement?: Placement;
    /** Delay before showing (ms) */
    delay?: number;
    /** Delay before hiding (ms) */
    hideDelay?: number;
    /** Disabled state */
    disabled?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Trigger element slot */
    children?: Snippet;
    /** Content slot for rich tooltips */
    tooltip?: Snippet;
  }

  let {
    content,
    placement = 'top',
    delay = 200,
    hideDelay = 0,
    disabled = false,
    class: className = '',
    children,
    tooltip,
  }: Props = $props();

  // State
  let isVisible = $state(false);
  let triggerRef: HTMLElement;
  let tooltipRef: HTMLElement;
  let showTimeout: ReturnType<typeof setTimeout>;
  let hideTimeout: ReturnType<typeof setTimeout>;

  // Placement position classes
  const placementClasses: Record<Placement, string> = {
    'top': 'bottom-full left-1/2 -translate-x-1/2 mb-2',
    'top-start': 'bottom-full left-0 mb-2',
    'top-end': 'bottom-full right-0 mb-2',
    'bottom': 'top-full left-1/2 -translate-x-1/2 mt-2',
    'bottom-start': 'top-full left-0 mt-2',
    'bottom-end': 'top-full right-0 mt-2',
    'left': 'right-full top-1/2 -translate-y-1/2 mr-2',
    'left-start': 'right-full top-0 mr-2',
    'left-end': 'right-full bottom-0 mr-2',
    'right': 'left-full top-1/2 -translate-y-1/2 ml-2',
    'right-start': 'left-full top-0 ml-2',
    'right-end': 'left-full bottom-0 ml-2',
  };

  // Arrow placement classes
  const arrowClasses: Record<Placement, string> = {
    'top': 'top-full left-1/2 -translate-x-1/2 border-t-[hsl(var(--slate-800))] border-x-transparent border-b-transparent',
    'top-start': 'top-full left-3 border-t-[hsl(var(--slate-800))] border-x-transparent border-b-transparent',
    'top-end': 'top-full right-3 border-t-[hsl(var(--slate-800))] border-x-transparent border-b-transparent',
    'bottom': 'bottom-full left-1/2 -translate-x-1/2 border-b-[hsl(var(--slate-800))] border-x-transparent border-t-transparent',
    'bottom-start': 'bottom-full left-3 border-b-[hsl(var(--slate-800))] border-x-transparent border-t-transparent',
    'bottom-end': 'bottom-full right-3 border-b-[hsl(var(--slate-800))] border-x-transparent border-t-transparent',
    'left': 'left-full top-1/2 -translate-y-1/2 border-l-[hsl(var(--slate-800))] border-y-transparent border-r-transparent',
    'left-start': 'left-full top-2 border-l-[hsl(var(--slate-800))] border-y-transparent border-r-transparent',
    'left-end': 'left-full bottom-2 border-l-[hsl(var(--slate-800))] border-y-transparent border-r-transparent',
    'right': 'right-full top-1/2 -translate-y-1/2 border-r-[hsl(var(--slate-800))] border-y-transparent border-l-transparent',
    'right-start': 'right-full top-2 border-r-[hsl(var(--slate-800))] border-y-transparent border-l-transparent',
    'right-end': 'right-full bottom-2 border-r-[hsl(var(--slate-800))] border-y-transparent border-l-transparent',
  };

  function showTooltip() {
    if (disabled) return;
    clearTimeout(hideTimeout);
    showTimeout = setTimeout(() => {
      isVisible = true;
    }, delay);
  }

  function hideTooltip() {
    clearTimeout(showTimeout);
    hideTimeout = setTimeout(() => {
      isVisible = false;
    }, hideDelay);
  }

  onDestroy(() => {
    clearTimeout(showTimeout);
    clearTimeout(hideTimeout);
  });

  // Computed classes
  let tooltipClasses = $derived([
    'absolute z-50',
    'px-2 py-1',
    'text-xs text-[hsl(var(--slate-200))]',
    'bg-[hsl(var(--slate-800))]',
    'rounded-md',
    'shadow-lg shadow-black/20',
    'whitespace-nowrap',
    'pointer-events-none',
    'transition-all duration-150',
    isVisible ? 'opacity-100 scale-100' : 'opacity-0 scale-95',
    placementClasses[placement],
    className,
  ].filter(Boolean).join(' '));
</script>

<div
  class="relative inline-flex"
  bind:this={triggerRef}
  onmouseenter={showTooltip}
  onmouseleave={hideTooltip}
  onfocus={showTooltip}
  onblur={hideTooltip}
>
  {#if children}
    {@render children()}
  {/if}

  {#if (content || tooltip) && isVisible}
    <div
      bind:this={tooltipRef}
      class={tooltipClasses}
      role="tooltip"
    >
      <!-- Arrow -->
      <div class="absolute w-0 h-0 border-4 {arrowClasses[placement]}"></div>

      {#if tooltip}
        {@render tooltip()}
      {:else}
        {content}
      {/if}
    </div>
  {/if}
</div>
