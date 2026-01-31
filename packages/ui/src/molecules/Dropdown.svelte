<script lang="ts">
  /**
   * Dropdown - Floating UI dropdown with laser/spotlight mouse effect
   * Features keyboard navigation and click outside to close
   */
  import type { Snippet } from 'svelte';
  import { onMount, onDestroy, tick } from 'svelte';

  type Placement = 'top' | 'top-start' | 'top-end' | 'bottom' | 'bottom-start' | 'bottom-end' |
                   'left' | 'left-start' | 'left-end' | 'right' | 'right-start' | 'right-end';

  interface DropdownItem {
    /** Unique identifier */
    id: string;
    /** Item label */
    label: string;
    /** Icon snippet (optional) */
    icon?: Snippet;
    /** Disabled state */
    disabled?: boolean;
    /** Separator before this item */
    separator?: boolean;
    /** Danger/destructive action styling */
    danger?: boolean;
  }

  interface Props {
    /** Dropdown items */
    items: DropdownItem[];
    /** Open state */
    open?: boolean;
    /** Placement relative to trigger */
    placement?: Placement;
    /** Offset from trigger in pixels */
    offset?: number;
    /** Enable laser/spotlight effect on hover */
    laserEffect?: boolean;
    /** Additional CSS classes for menu */
    class?: string;
    /** Trigger button snippet */
    trigger: Snippet;
    /** Item select handler */
    onselect?: (item: DropdownItem) => void;
    /** Open state change handler */
    onopenchange?: (open: boolean) => void;
  }

  let {
    items,
    open = $bindable(false),
    placement = 'bottom-start',
    offset: offsetValue = 8,
    laserEffect = true,
    class: className = '',
    trigger,
    onselect,
    onopenchange
  }: Props = $props();

  let triggerRef: HTMLElement;
  let menuRef: HTMLElement;
  let laserRef: HTMLElement;
  let focusedIndex = $state(-1);
  let menuStyle = $state('');

  // Position the menu using Floating UI middleware patterns
  // Implements offset, flip (collision detection), and shift (viewport boundary)
  function updatePosition() {
    if (!triggerRef || !menuRef || !open) return;

    const triggerRect = triggerRef.getBoundingClientRect();
    const menuRect = menuRef.getBoundingClientRect();
    const viewport = {
      width: window.innerWidth,
      height: window.innerHeight,
    };

    let currentPlacement = placement;
    let top = 0;
    let left = 0;

    // Calculate initial position based on placement
    function calculatePosition(pl: Placement): { top: number; left: number } {
      let t = 0;
      let l = 0;

      switch (pl) {
        case 'bottom':
        case 'bottom-start':
          t = triggerRect.bottom + offsetValue;
          l = pl === 'bottom'
            ? triggerRect.left + (triggerRect.width - menuRect.width) / 2
            : triggerRect.left;
          break;
        case 'bottom-end':
          t = triggerRect.bottom + offsetValue;
          l = triggerRect.right - menuRect.width;
          break;
        case 'top':
        case 'top-start':
          t = triggerRect.top - menuRect.height - offsetValue;
          l = pl === 'top'
            ? triggerRect.left + (triggerRect.width - menuRect.width) / 2
            : triggerRect.left;
          break;
        case 'top-end':
          t = triggerRect.top - menuRect.height - offsetValue;
          l = triggerRect.right - menuRect.width;
          break;
        case 'right':
        case 'right-start':
          t = pl === 'right'
            ? triggerRect.top + (triggerRect.height - menuRect.height) / 2
            : triggerRect.top;
          l = triggerRect.right + offsetValue;
          break;
        case 'right-end':
          t = triggerRect.bottom - menuRect.height;
          l = triggerRect.right + offsetValue;
          break;
        case 'left':
        case 'left-start':
          t = pl === 'left'
            ? triggerRect.top + (triggerRect.height - menuRect.height) / 2
            : triggerRect.top;
          l = triggerRect.left - menuRect.width - offsetValue;
          break;
        case 'left-end':
          t = triggerRect.bottom - menuRect.height;
          l = triggerRect.left - menuRect.width - offsetValue;
          break;
      }
      return { top: t, left: l };
    }

    // Get opposite placement for flip
    function getOppositePlacement(pl: Placement): Placement {
      const opposites: Record<string, Placement> = {
        'top': 'bottom',
        'top-start': 'bottom-start',
        'top-end': 'bottom-end',
        'bottom': 'top',
        'bottom-start': 'top-start',
        'bottom-end': 'top-end',
        'left': 'right',
        'left-start': 'right-start',
        'left-end': 'right-end',
        'right': 'left',
        'right-start': 'left-start',
        'right-end': 'left-end',
      };
      return opposites[pl] || pl;
    }

    // Calculate initial position
    let pos = calculatePosition(currentPlacement);
    top = pos.top;
    left = pos.left;

    // Flip middleware: check if menu overflows viewport and flip if needed
    const overflowsBottom = top + menuRect.height > viewport.height - 10;
    const overflowsTop = top < 10;
    const overflowsRight = left + menuRect.width > viewport.width - 10;
    const overflowsLeft = left < 10;

    // Flip vertically if needed
    if ((currentPlacement.startsWith('bottom') && overflowsBottom) ||
        (currentPlacement.startsWith('top') && overflowsTop)) {
      currentPlacement = getOppositePlacement(currentPlacement);
      pos = calculatePosition(currentPlacement);
      top = pos.top;
      left = pos.left;
    }

    // Flip horizontally if needed
    if ((currentPlacement.startsWith('right') && overflowsRight) ||
        (currentPlacement.startsWith('left') && overflowsLeft)) {
      currentPlacement = getOppositePlacement(currentPlacement);
      pos = calculatePosition(currentPlacement);
      top = pos.top;
      left = pos.left;
    }

    // Shift middleware: keep menu within viewport bounds
    const padding = 10;

    // Shift horizontally
    if (left < padding) {
      left = padding;
    } else if (left + menuRect.width > viewport.width - padding) {
      left = viewport.width - menuRect.width - padding;
    }

    // Shift vertically
    if (top < padding) {
      top = padding;
    } else if (top + menuRect.height > viewport.height - padding) {
      top = viewport.height - menuRect.height - padding;
    }

    menuStyle = `top: ${top}px; left: ${left}px;`;
  }

  // Laser effect tracking
  function handleMouseMove(e: MouseEvent) {
    if (!laserEffect || !laserRef || !menuRef) return;

    const rect = menuRef.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Check if hovering over an item
    const target = e.target as HTMLElement;
    const isOnItem = target.closest('[data-dropdown-item]');

    if (isOnItem) {
      laserRef.style.background = `
        radial-gradient(
          circle at ${x}px ${y}px,
          rgba(79, 209, 197, 0.2) 0%,
          transparent 40%
        )
      `;
      laserRef.style.opacity = '0.8';
    } else {
      laserRef.style.background = `
        radial-gradient(
          circle at ${x}px ${y}px,
          rgba(79, 209, 197, 0.1) 0%,
          transparent 40%
        )
      `;
      laserRef.style.opacity = '0.2';
    }
  }

  function handleMouseLeave() {
    if (laserRef) {
      laserRef.style.opacity = '0';
    }
  }

  // Toggle menu
  function toggleMenu() {
    open = !open;
    onopenchange?.(open);

    if (open) {
      focusedIndex = -1;
      tick().then(updatePosition);
    }
  }

  // Close menu
  function closeMenu() {
    open = false;
    onopenchange?.(false);
    focusedIndex = -1;
  }

  // Handle item selection
  function handleSelect(item: DropdownItem) {
    if (item.disabled) return;
    onselect?.(item);
    closeMenu();
  }

  // Keyboard navigation
  function handleKeydown(e: KeyboardEvent) {
    if (!open) {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault();
        toggleMenu();
      }
      return;
    }

    const enabledItems = items.filter(i => !i.disabled);

    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        closeMenu();
        triggerRef?.focus();
        break;
      case 'ArrowDown':
        e.preventDefault();
        focusedIndex = Math.min(focusedIndex + 1, enabledItems.length - 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        focusedIndex = Math.max(focusedIndex - 1, 0);
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (focusedIndex >= 0) {
          handleSelect(enabledItems[focusedIndex]);
        }
        break;
      case 'Tab':
        closeMenu();
        break;
    }
  }

  // Click outside handler
  function handleClickOutside(e: MouseEvent) {
    if (!open) return;
    const target = e.target as Node;
    if (!triggerRef?.contains(target) && !menuRef?.contains(target)) {
      closeMenu();
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside, true);
    document.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside, true);
    document.removeEventListener('keydown', handleKeydown);
  });

  // Update position when open changes
  $effect(() => {
    if (open) {
      tick().then(updatePosition);
    }
  });
</script>

<div class="dropdown relative inline-block">
  <!-- Trigger -->
  <div
    bind:this={triggerRef}
    onclick={toggleMenu}
    onkeydown={handleKeydown}
    role="button"
    tabindex="0"
    aria-haspopup="true"
    aria-expanded={open}
  >
    {@render trigger()}
  </div>

  <!-- Menu -->
  {#if open}
    <div
      bind:this={menuRef}
      class={`
        fixed z-50 min-w-[10rem] py-1
        bg-slate-800/95 backdrop-blur-xl
        border border-slate-600/30 rounded-lg
        shadow-[0_4px_20px_rgba(0,0,0,0.4)]
        transition-all duration-200
        ${className}
      `}
      style={menuStyle}
      role="menu"
      aria-orientation="vertical"
      onmousemove={handleMouseMove}
      onmouseleave={handleMouseLeave}
    >
      <!-- Laser effect layer -->
      {#if laserEffect}
        <div
          bind:this={laserRef}
          class="absolute inset-0 pointer-events-none opacity-0 mix-blend-plus-lighter transition-opacity duration-300 rounded-lg"
        ></div>
      {/if}

      <!-- Menu items -->
      <div class="relative z-10">
        {#each items as item, index (item.id)}
          {#if item.separator}
            <hr class="my-1 mx-2 border-t border-slate-600/50" />
          {/if}

          <button
            type="button"
            data-dropdown-item
            class={`
              w-full flex items-center gap-2 px-3 py-2 text-left text-sm
              transition-all duration-200
              ${item.disabled
                ? 'opacity-50 cursor-not-allowed text-slate-500'
                : item.danger
                  ? 'text-coral-400 hover:bg-coral-500/10 hover:text-coral-300'
                  : 'text-slate-200 hover:bg-slate-700/50 hover:text-white'
              }
              ${focusedIndex === index ? 'bg-slate-700/50 text-white' : ''}
              focus:outline-none focus:bg-slate-700/50
            `}
            onclick={() => handleSelect(item)}
            disabled={item.disabled}
            role="menuitem"
            tabindex={-1}
          >
            {#if item.icon}
              <span class="w-4 h-4 flex-shrink-0">
                {@render item.icon()}
              </span>
            {/if}
            <span>{item.label}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .dropdown [role="menu"] {
    animation: dropdown-in 0.2s ease-out;
  }

  @keyframes dropdown-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
