<script lang="ts">
  /**
   * Accordion - Collapsible content panel with glassmorphic styling
   * Reference: Vue Accordion.vue
   */
  import { onMount, onDestroy } from 'svelte';
  import type { Snippet } from 'svelte';

  interface Props {
    /** Accordion title */
    title: string;
    /** Preview text when collapsed */
    preview?: string;
    /** Open state */
    open?: boolean;
    /** Close on click outside */
    closeOnClickOutside?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Title slot content */
    titleContent?: Snippet;
    /** Main content */
    children?: Snippet;
  }

  let {
    title,
    preview = '',
    open = $bindable(false),
    closeOnClickOutside = true,
    class: className = '',
    titleContent,
    children,
  }: Props = $props();

  let accordionRef: HTMLDivElement | null = $state(null);

  function toggle() {
    open = !open;
  }

  function handleClickOutside(event: MouseEvent) {
    if (!closeOnClickOutside || !open) return;
    if (accordionRef && !accordionRef.contains(event.target as Node)) {
      open = false;
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    if (typeof document !== 'undefined') {
      document.removeEventListener('click', handleClickOutside);
    }
  });

  // Computed preview text
  let displayPreview = $derived(() => {
    if (!preview || open) return '';
    const truncated = preview.split(' and innovate')[0];
    return truncated.length < preview.length ? truncated + ' and...' : preview;
  });
</script>

<div bind:this={accordionRef} class="mb-2 w-[90%] mx-auto {className}">
  <div
    class="
      rounded-lg bg-transparent border border-slate-600/40
      transition-all duration-200 ease-out overflow-hidden
      {open
        ? 'shadow-[inset_0_-2px_8px_rgba(30,41,59,0.6)]'
        : 'shadow-[inset_0_-1px_2px_rgba(30,41,59,0.4)] hover:shadow-[inset_0_-1px_4px_rgba(30,41,59,0.5)]'
      }
    "
  >
    <!-- Accordion Header -->
    <button
      type="button"
      onclick={(e) => { e.stopPropagation(); toggle(); }}
      class="w-full text-left py-1.5 px-2 relative group focus:outline-none focus-visible:ring-2 focus-visible:ring-teal-500/50"
    >
      <div class="flex items-center justify-between">
        <div class="flex-1 pr-4">
          {#if preview && !open}
            <div class="text-base text-slate-300/90 leading-relaxed pr-8">
              {displayPreview()}
            </div>
          {:else if titleContent}
            {@render titleContent()}
          {:else}
            <span class="text-base text-slate-200 font-medium">{title}</span>
          {/if}
        </div>

        <!-- Chevron icon -->
        <div
          class="w-5 h-5 flex items-center justify-center transition-transform duration-200 ease-out"
          class:rotate-180={open}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="w-4 h-4 text-teal-500/60 group-hover:text-teal-500 transition-colors duration-200"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 9l-7 7-7-7"
            />
          </svg>
        </div>
      </div>
    </button>

    <!-- Accordion Content -->
    <div
      class="transition-all duration-150 ease-in-out overflow-hidden"
      style="
        max-height: {open ? '1000px' : '0'};
        opacity: {open ? '1' : '0'};
        transform: {open ? 'translateY(0)' : 'translateY(-2px)'};
      "
    >
      <div class="px-2 pb-2">
        <div class="text-base text-slate-300 leading-relaxed accordion-content">
          {#if children}
            {@render children()}
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .rotate-180 {
    transform: rotate(180deg);
  }

  .accordion-content {
    text-align: justify;
    max-width: 95%;
    margin: 0 auto;
    padding: 0.25em 0;
  }

  .accordion-content :global(p) {
    margin: 0;
    padding: 0.25em 0;
    line-height: 1.6;
  }

  .accordion-content :global(p:first-of-type) {
    max-width: 75%;
    margin: 0 auto;
  }

  .accordion-content :global(p:nth-of-type(2)) {
    max-width: 85%;
    margin: 0 auto;
  }

  .accordion-content :global(p:nth-of-type(3)) {
    max-width: 90%;
    margin: 0 auto;
  }

  .accordion-content :global(p:nth-of-type(n+4)) {
    max-width: 95%;
    margin: 0 auto;
  }
</style>
