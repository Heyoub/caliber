<script lang="ts">
  /**
   * Tabs - Tab navigation with animated indicator
   * Supports horizontal/vertical layouts with smooth transitions
   */
  import type { Snippet } from 'svelte';

  type Size = 'sm' | 'md' | 'lg';
  type ColorPalette = 'teal' | 'coral' | 'purple' | 'pink' | 'mint' | 'amber';

  interface TabItem {
    /** Unique identifier */
    id: string;
    /** Tab label */
    label: string;
    /** Icon snippet (optional) */
    icon?: Snippet;
    /** Disabled state */
    disabled?: boolean;
    /** Badge count */
    badge?: number | string;
  }

  interface Props {
    /** Tab items */
    items: TabItem[];
    /** Currently active tab id */
    activeTab?: string;
    /** Component size */
    size?: Size;
    /** Color theme for active indicator */
    color?: ColorPalette;
    /** Full width tabs */
    fullWidth?: boolean;
    /** Vertical orientation */
    vertical?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Tab change handler */
    onchange?: (tabId: string) => void;
  }

  let {
    items,
    activeTab = $bindable(items[0]?.id ?? ''),
    size = 'md',
    color = 'teal',
    fullWidth = false,
    vertical = false,
    class: className = '',
    onchange
  }: Props = $props();

  // Track indicator position
  let tabsContainer: HTMLElement;
  let indicatorStyle = $state('');

  // Size mappings
  const sizeClasses: Record<Size, string> = {
    sm: 'h-8 px-3 text-sm',
    md: 'h-10 px-4 text-base',
    lg: 'h-12 px-5 text-lg'
  };

  // Color mappings for indicator
  const colorClasses: Record<ColorPalette, string> = {
    teal: 'bg-teal-500 shadow-[0_0_10px_hsl(176_55%_45%/0.5)]',
    coral: 'bg-coral-500 shadow-[0_0_10px_hsl(12_65%_50%/0.5)]',
    purple: 'bg-purple-500 shadow-[0_0_10px_hsl(270_55%_52%/0.5)]',
    pink: 'bg-pink-500 shadow-[0_0_10px_hsl(330_60%_52%/0.5)]',
    mint: 'bg-mint-500 shadow-[0_0_10px_hsl(160_45%_45%/0.5)]',
    amber: 'bg-amber-500 shadow-[0_0_10px_hsl(38_70%_42%/0.5)]'
  };

  function updateIndicator() {
    if (!tabsContainer) return;

    const activeElement = tabsContainer.querySelector(`[data-tab-id="${activeTab}"]`) as HTMLElement;
    if (!activeElement) return;

    if (vertical) {
      indicatorStyle = `
        top: ${activeElement.offsetTop}px;
        height: ${activeElement.offsetHeight}px;
        width: 3px;
        left: 0;
      `;
    } else {
      indicatorStyle = `
        left: ${activeElement.offsetLeft}px;
        width: ${activeElement.offsetWidth}px;
        height: 3px;
        bottom: 0;
      `;
    }
  }

  function handleTabClick(tabId: string) {
    const item = items.find(i => i.id === tabId);
    if (item?.disabled) return;

    activeTab = tabId;
    onchange?.(tabId);

    // Update indicator after DOM updates
    requestAnimationFrame(updateIndicator);
  }

  // Update indicator when active tab changes
  $effect(() => {
    if (tabsContainer && activeTab) {
      updateIndicator();
    }
  });

  // Computed classes
  const containerClasses = $derived(
    `tabs relative ${vertical ? 'flex flex-col' : 'flex flex-row'}
    bg-slate-800/40 backdrop-blur-sm rounded-lg border border-slate-700/30
    ${fullWidth ? 'w-full' : 'inline-flex'}
    ${className}`.trim()
  );
</script>

<div
  bind:this={tabsContainer}
  class={containerClasses}
  role="tablist"
  aria-orientation={vertical ? 'vertical' : 'horizontal'}
>
  {#each items as item (item.id)}
    <button
      type="button"
      role="tab"
      data-tab-id={item.id}
      class={`
        flex items-center justify-center gap-2 transition-all duration-200
        ${sizeClasses[size]}
        ${fullWidth ? 'flex-1' : ''}
        ${activeTab === item.id ? 'text-white' : 'text-slate-400 hover:text-slate-200'}
        ${item.disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
        focus:outline-none focus:ring-2 focus:ring-teal-500/50 focus:ring-inset
        relative z-10
      `}
      onclick={() => handleTabClick(item.id)}
      disabled={item.disabled}
      aria-selected={activeTab === item.id}
      aria-controls={`panel-${item.id}`}
      tabindex={activeTab === item.id ? 0 : -1}
    >
      {#if item.icon}
        {@render item.icon()}
      {/if}
      <span>{item.label}</span>
      {#if item.badge !== undefined}
        <span class="ml-1.5 px-1.5 py-0.5 text-xs rounded-full bg-slate-700 text-slate-300">
          {item.badge}
        </span>
      {/if}
    </button>
  {/each}

  <!-- Animated indicator -->
  <div
    class={`absolute transition-all duration-300 ease-spring rounded-full ${colorClasses[color]}`}
    style={indicatorStyle}
  ></div>
</div>

<style>
  .ease-spring {
    transition-timing-function: cubic-bezier(0.34, 1.56, 0.64, 1);
  }
</style>
