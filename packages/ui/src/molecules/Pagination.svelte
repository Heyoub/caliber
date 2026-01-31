<script lang="ts">
  /**
   * Pagination - Page navigation component
   * Supports various page display modes
   */
  import type { Snippet } from 'svelte';

  type Size = 'sm' | 'md' | 'lg';

  interface Props {
    /** Current page (1-indexed) */
    currentPage?: number;
    /** Total number of pages */
    totalPages: number;
    /** Total items count (optional, for display) */
    totalItems?: number;
    /** Items per page (optional, for display) */
    perPage?: number;
    /** Number of page buttons to show */
    siblingCount?: number;
    /** Component size */
    size?: Size;
    /** Show first/last buttons */
    showFirstLast?: boolean;
    /** Show prev/next buttons */
    showPrevNext?: boolean;
    /** Show page info text */
    showInfo?: boolean;
    /** Disabled state */
    disabled?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Page change handler */
    onchange?: (page: number) => void;
  }

  let {
    currentPage = $bindable(1),
    totalPages,
    totalItems,
    perPage,
    siblingCount = 1,
    size = 'md',
    showFirstLast = true,
    showPrevNext = true,
    showInfo = false,
    disabled = false,
    class: className = '',
    onchange
  }: Props = $props();

  // Size mappings
  const sizeClasses: Record<Size, { button: string; icon: string }> = {
    sm: { button: 'h-7 min-w-[1.75rem] text-xs px-2', icon: 'w-3 h-3' },
    md: { button: 'h-9 min-w-[2.25rem] text-sm px-3', icon: 'w-4 h-4' },
    lg: { button: 'h-11 min-w-[2.75rem] text-base px-4', icon: 'w-5 h-5' }
  };

  // Calculate visible page numbers
  function getPageNumbers(): (number | 'ellipsis')[] {
    const pages: (number | 'ellipsis')[] = [];

    // Always show first page
    pages.push(1);

    // Calculate range around current page
    const leftSibling = Math.max(2, currentPage - siblingCount);
    const rightSibling = Math.min(totalPages - 1, currentPage + siblingCount);

    // Add ellipsis after first page if needed
    if (leftSibling > 2) {
      pages.push('ellipsis');
    }

    // Add page numbers in range
    for (let i = leftSibling; i <= rightSibling; i++) {
      if (i !== 1 && i !== totalPages) {
        pages.push(i);
      }
    }

    // Add ellipsis before last page if needed
    if (rightSibling < totalPages - 1) {
      pages.push('ellipsis');
    }

    // Always show last page (if more than 1 page)
    if (totalPages > 1) {
      pages.push(totalPages);
    }

    return pages;
  }

  function goToPage(page: number) {
    if (disabled || page < 1 || page > totalPages || page === currentPage) return;
    currentPage = page;
    onchange?.(page);
  }

  // Calculate info text
  const infoText = $derived(() => {
    if (!totalItems || !perPage) return '';
    const start = (currentPage - 1) * perPage + 1;
    const end = Math.min(currentPage * perPage, totalItems);
    return `${start}-${end} of ${totalItems}`;
  });

  const styles = $derived(sizeClasses[size]);
  const pageNumbers = $derived(getPageNumbers());

  // Icon paths
  const iconPaths = {
    first: 'm11 17-5-5 5-5 m7 10-5-5 5-5',
    prev: 'm15 18-6-6 6-6',
    next: 'm9 18 6-6-6-6',
    last: 'm13 17 5-5-5-5 m-7 10 5-5-5-5'
  };
</script>

<nav
  aria-label="Pagination"
  class={`pagination flex items-center gap-1 ${disabled ? 'opacity-50 pointer-events-none' : ''} ${className}`}
>
  <!-- Info text -->
  {#if showInfo && infoText()}
    <span class="mr-4 text-sm text-slate-400">
      {infoText()}
    </span>
  {/if}

  <!-- First page button -->
  {#if showFirstLast}
    <button
      type="button"
      onclick={() => goToPage(1)}
      disabled={currentPage === 1 || disabled}
      class={`
        ${styles.button}
        flex items-center justify-center rounded-lg
        text-slate-400 hover:text-white
        bg-slate-800/40 hover:bg-slate-700/60
        border border-slate-600/30
        disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-slate-800/40
        transition-all duration-200
        focus:outline-none focus:ring-2 focus:ring-teal-500/50
      `}
      aria-label="First page"
    >
      <svg
        class={styles.icon}
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d={iconPaths.first} />
      </svg>
    </button>
  {/if}

  <!-- Previous page button -->
  {#if showPrevNext}
    <button
      type="button"
      onclick={() => goToPage(currentPage - 1)}
      disabled={currentPage === 1 || disabled}
      class={`
        ${styles.button}
        flex items-center justify-center rounded-lg
        text-slate-400 hover:text-white
        bg-slate-800/40 hover:bg-slate-700/60
        border border-slate-600/30
        disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-slate-800/40
        transition-all duration-200
        focus:outline-none focus:ring-2 focus:ring-teal-500/50
      `}
      aria-label="Previous page"
    >
      <svg
        class={styles.icon}
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d={iconPaths.prev} />
      </svg>
    </button>
  {/if}

  <!-- Page numbers -->
  {#each pageNumbers as page, index (typeof page === 'number' ? page : `ellipsis-${index}`)}
    {#if page === 'ellipsis'}
      <span class={`${styles.button} flex items-center justify-center text-slate-500`}>
        ...
      </span>
    {:else}
      <button
        type="button"
        onclick={() => goToPage(page)}
        disabled={disabled}
        class={`
          ${styles.button}
          flex items-center justify-center rounded-lg
          border transition-all duration-200
          focus:outline-none focus:ring-2 focus:ring-teal-500/50
          ${currentPage === page
            ? 'text-white bg-teal-600 border-teal-500 shadow-[0_0_10px_hsl(176_55%_45%/0.3)]'
            : 'text-slate-400 hover:text-white bg-slate-800/40 hover:bg-slate-700/60 border-slate-600/30'
          }
        `}
        aria-label={`Page ${page}`}
        aria-current={currentPage === page ? 'page' : undefined}
      >
        {page}
      </button>
    {/if}
  {/each}

  <!-- Next page button -->
  {#if showPrevNext}
    <button
      type="button"
      onclick={() => goToPage(currentPage + 1)}
      disabled={currentPage === totalPages || disabled}
      class={`
        ${styles.button}
        flex items-center justify-center rounded-lg
        text-slate-400 hover:text-white
        bg-slate-800/40 hover:bg-slate-700/60
        border border-slate-600/30
        disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-slate-800/40
        transition-all duration-200
        focus:outline-none focus:ring-2 focus:ring-teal-500/50
      `}
      aria-label="Next page"
    >
      <svg
        class={styles.icon}
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d={iconPaths.next} />
      </svg>
    </button>
  {/if}

  <!-- Last page button -->
  {#if showFirstLast}
    <button
      type="button"
      onclick={() => goToPage(totalPages)}
      disabled={currentPage === totalPages || disabled}
      class={`
        ${styles.button}
        flex items-center justify-center rounded-lg
        text-slate-400 hover:text-white
        bg-slate-800/40 hover:bg-slate-700/60
        border border-slate-600/30
        disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-slate-800/40
        transition-all duration-200
        focus:outline-none focus:ring-2 focus:ring-teal-500/50
      `}
      aria-label="Last page"
    >
      <svg
        class={styles.icon}
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d={iconPaths.last} />
      </svg>
    </button>
  {/if}
</nav>
