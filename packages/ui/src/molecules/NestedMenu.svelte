<script lang="ts">
  /**
   * NestedMenu - Menu with category headers and separators
   * Based on AuthNav.vue pattern with HR separators and category icons
   */
  import type { Snippet } from 'svelte';

  interface MenuItem {
    /** Unique identifier */
    id: string;
    /** Menu item label */
    label: string;
    /** Navigation href */
    href?: string;
    /** Icon snippet (optional) */
    icon?: Snippet;
    /** Disabled state */
    disabled?: boolean;
    /** Active/current state */
    active?: boolean;
  }

  interface MenuCategory {
    /** Category identifier */
    id: string;
    /** Category header text */
    title: string;
    /** Category icon snippet (optional) */
    icon?: Snippet;
    /** Menu items in this category */
    items: MenuItem[];
  }

  interface Props {
    /** Menu categories */
    categories: MenuCategory[];
    /** Show separators between categories */
    showSeparators?: boolean;
    /** Glass effect backdrop */
    glass?: boolean;
    /** Compact mode (less padding) */
    compact?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Item click handler */
    onitemclick?: (item: MenuItem) => void;
  }

  let {
    categories,
    showSeparators = true,
    glass = true,
    compact = false,
    class: className = '',
    onitemclick
  }: Props = $props();

  // Hover tracking for underline animation
  let hoveredItem = $state<string | null>(null);

  function handleItemClick(item: MenuItem, event: MouseEvent) {
    if (item.disabled) {
      event.preventDefault();
      return;
    }
    onitemclick?.(item);
  }

  function handleKeydown(item: MenuItem, event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (!item.disabled) {
        onitemclick?.(item);
      }
    }
  }
</script>

<nav
  class={`
    nested-menu py-1
    ${glass ? 'bg-slate-800/90 backdrop-blur-xl' : 'bg-slate-800'}
    ${className}
  `}
  role="menu"
>
  {#each categories as category, categoryIndex (category.id)}
    <!-- Separator (not for first category) -->
    {#if showSeparators && categoryIndex > 0}
      <hr class="my-2 mx-4 border-t border-slate-600/30" />
    {/if}

    <!-- Category block -->
    <div class="category-block">
      <!-- Category header -->
      <div
        class={`
          flex items-center gap-2
          ${compact ? 'px-3 pt-2 pb-1' : 'px-4 pt-3 pb-1'}
          text-xs font-semibold text-slate-400 uppercase tracking-wider
        `}
      >
        {#if category.icon}
          <span class="w-4 h-4 text-teal-400/70">
            {@render category.icon()}
          </span>
        {/if}
        <span>{category.title}</span>
      </div>

      <!-- Category items -->
      {#each category.items as item (item.id)}
        {#if item.href}
          <a
            href={item.href}
            class={`
              block relative ${compact ? 'px-3 py-1.5' : 'px-4 py-2'} text-sm
              transition-all duration-200
              ${item.disabled
                ? 'text-slate-500 cursor-not-allowed'
                : item.active
                  ? 'text-white bg-teal-500/10'
                  : 'text-slate-300 hover:text-white hover:bg-slate-700/30'
              }
              group/link
            `}
            onclick={(e) => handleItemClick(item, e)}
            onkeydown={(e) => handleKeydown(item, e)}
            onmouseenter={() => hoveredItem = item.id}
            onmouseleave={() => hoveredItem = null}
            aria-current={item.active ? 'page' : undefined}
            aria-disabled={item.disabled}
            role="menuitem"
          >
            <span class="relative z-10 flex items-center gap-2">
              {#if item.icon}
                <span class="w-4 h-4 flex-shrink-0">
                  {@render item.icon()}
                </span>
              {/if}
              {item.label}
            </span>

            <!-- Animated underline on hover -->
            <span
              class={`
                absolute bottom-0 h-px bg-teal-400/80
                transition-all duration-300
                shadow-[0_0_10px_hsl(176_55%_45%/0.5)]
                ${hoveredItem === item.id ? 'left-4 right-4' : 'left-1/2 right-1/2'}
              `}
            ></span>
          </a>
        {:else}
          <button
            type="button"
            class={`
              w-full text-left relative ${compact ? 'px-3 py-1.5' : 'px-4 py-2'} text-sm
              transition-all duration-200
              ${item.disabled
                ? 'text-slate-500 cursor-not-allowed'
                : item.active
                  ? 'text-white bg-teal-500/10'
                  : 'text-slate-300 hover:text-white hover:bg-slate-700/30'
              }
              group/link
            `}
            onclick={(e) => handleItemClick(item, e)}
            onkeydown={(e) => handleKeydown(item, e)}
            onmouseenter={() => hoveredItem = item.id}
            onmouseleave={() => hoveredItem = null}
            disabled={item.disabled}
            aria-current={item.active ? 'true' : undefined}
            role="menuitem"
          >
            <span class="relative z-10 flex items-center gap-2">
              {#if item.icon}
                <span class="w-4 h-4 flex-shrink-0">
                  {@render item.icon()}
                </span>
              {/if}
              {item.label}
            </span>

            <!-- Animated underline on hover -->
            <span
              class={`
                absolute bottom-0 h-px bg-teal-400/80
                transition-all duration-300
                shadow-[0_0_10px_hsl(176_55%_45%/0.5)]
                ${hoveredItem === item.id ? 'left-4 right-4' : 'left-1/2 right-1/2'}
              `}
            ></span>
          </button>
        {/if}
      {/each}
    </div>
  {/each}
</nav>

<style>
  .nested-menu {
    /* Subtle border glow */
    box-shadow:
      0 0 1px 1px hsl(176 55% 45% / 0.1),
      0 0 1px 1px hsl(220 16% 8% / 0.5);
  }

  .nested-menu:hover {
    box-shadow:
      0 0 5px 1px hsl(176 55% 45% / 0.1),
      0 0 5px 1px hsl(270 55% 52% / 0.05),
      0 0 5px 1px hsl(160 45% 45% / 0.05);
  }
</style>
