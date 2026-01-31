<script lang="ts">
  /**
   * Breadcrumb - Navigation path component
   * Shows hierarchical navigation with separators
   */
  import type { Snippet } from 'svelte';

  type Size = 'sm' | 'md' | 'lg';
  type SeparatorType = 'slash' | 'chevron' | 'arrow' | 'dot';

  interface BreadcrumbItem {
    /** Unique identifier */
    id: string;
    /** Display label */
    label: string;
    /** Navigation href */
    href?: string;
    /** Icon snippet (optional) */
    icon?: Snippet;
    /** Current/active item */
    current?: boolean;
  }

  interface Props {
    /** Breadcrumb items */
    items: BreadcrumbItem[];
    /** Component size */
    size?: Size;
    /** Separator type */
    separator?: SeparatorType;
    /** Custom separator snippet */
    customSeparator?: Snippet;
    /** Show home icon for first item */
    showHomeIcon?: boolean;
    /** Max items before collapsing */
    maxItems?: number;
    /** Additional CSS classes */
    class?: string;
    /** Item click handler */
    onclick?: (item: BreadcrumbItem) => void;
  }

  let {
    items,
    size = 'md',
    separator = 'chevron',
    customSeparator,
    showHomeIcon = true,
    maxItems = 0,
    class: className = '',
    onclick
  }: Props = $props();

  // Size mappings
  const sizeClasses: Record<Size, { text: string; icon: string; gap: string }> = {
    sm: { text: 'text-xs', icon: 'w-3 h-3', gap: 'gap-1' },
    md: { text: 'text-sm', icon: 'w-4 h-4', gap: 'gap-1.5' },
    lg: { text: 'text-base', icon: 'w-5 h-5', gap: 'gap-2' }
  };

  // Separator SVG paths
  const separatorPaths: Record<SeparatorType, string> = {
    slash: 'M12 5l-1 14',
    chevron: 'm9 18 6-6-6-6',
    arrow: 'M5 12h14m-7-7 7 7-7 7',
    dot: 'M12 12m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0'
  };

  // Home icon path
  const homePath = 'm3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z M9 22V12h6v10';

  // Compute visible items (with collapse if needed)
  const visibleItems = $derived(() => {
    if (maxItems <= 0 || items.length <= maxItems) {
      return { items, collapsed: false, collapsedCount: 0 };
    }

    // Keep first, last, and some middle items
    const first = items.slice(0, 1);
    const last = items.slice(-Math.max(1, maxItems - 2));
    const collapsedCount = items.length - first.length - last.length;

    return {
      items: [...first, { id: '__collapsed__', label: '...' }, ...last],
      collapsed: true,
      collapsedCount
    };
  });

  function handleClick(item: BreadcrumbItem, event: MouseEvent) {
    if (item.id === '__collapsed__') return;
    if (item.current) {
      event.preventDefault();
      return;
    }
    onclick?.(item);
  }

  const styles = $derived(sizeClasses[size]);
</script>

<nav aria-label="Breadcrumb" class={`breadcrumb ${className}`}>
  <ol class={`flex items-center flex-wrap ${styles.gap}`}>
    {#each visibleItems().items as item, index (item.id)}
      <li class="flex items-center">
        <!-- Separator (not for first item) -->
        {#if index > 0}
          <span class="mx-2 text-slate-500" aria-hidden="true">
            {#if customSeparator}
              {@render customSeparator()}
            {:else}
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
                <path d={separatorPaths[separator]} />
              </svg>
            {/if}
          </span>
        {/if}

        <!-- Breadcrumb item -->
        {#if item.id === '__collapsed__'}
          <span class={`${styles.text} text-slate-500 px-1`} title="Collapsed items">
            {item.label}
          </span>
        {:else if item.href && !item.current}
          <a
            href={item.href}
            onclick={(e) => handleClick(item, e)}
            class={`
              flex items-center ${styles.gap} ${styles.text}
              text-slate-400 hover:text-teal-400
              transition-colors duration-200
              focus:outline-none focus:ring-2 focus:ring-teal-500/50 focus:ring-offset-1
              focus:ring-offset-slate-900 rounded px-1
            `}
          >
            {#if index === 0 && showHomeIcon}
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
                <path d={homePath} />
              </svg>
            {:else if item.icon}
              {@render item.icon()}
            {/if}
            <span>{item.label}</span>
          </a>
        {:else}
          <span
            class={`
              flex items-center ${styles.gap} ${styles.text}
              ${item.current ? 'text-slate-200 font-medium' : 'text-slate-400'}
              px-1
            `}
            aria-current={item.current ? 'page' : undefined}
          >
            {#if index === 0 && showHomeIcon}
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
                <path d={homePath} />
              </svg>
            {:else if item.icon}
              {@render item.icon()}
            {/if}
            <span>{item.label}</span>
          </span>
        {/if}
      </li>
    {/each}
  </ol>
</nav>
