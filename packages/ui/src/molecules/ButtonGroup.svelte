<script lang="ts">
  /**
   * ButtonGroup - Grouped action buttons
   * Renders a set of buttons with connected styling
   */
  import type { Snippet } from 'svelte';

  type Size = 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  type ColorPalette = 'teal' | 'coral' | 'purple' | 'pink' | 'mint' | 'amber' | 'slate' | 'ghost';

  interface ButtonItem {
    /** Unique identifier */
    id: string;
    /** Button label */
    label: string;
    /** Icon snippet (optional) */
    icon?: Snippet;
    /** Disabled state */
    disabled?: boolean;
  }

  interface Props {
    /** Button items */
    items: ButtonItem[];
    /** Currently selected button id (for toggle groups) */
    selected?: string;
    /** Allow multiple selections */
    multiple?: boolean;
    /** Selected items for multiple mode */
    selectedItems?: string[];
    /** Component size */
    size?: Size;
    /** Color theme */
    color?: ColorPalette;
    /** Full width buttons */
    fullWidth?: boolean;
    /** Vertical orientation */
    vertical?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Selection change handler */
    onchange?: (selected: string | string[]) => void;
    /** Click handler for individual buttons */
    onclick?: (id: string) => void;
  }

  let {
    items,
    selected = $bindable(''),
    multiple = false,
    selectedItems = $bindable([]),
    size = 'md',
    color = 'teal',
    fullWidth = false,
    vertical = false,
    class: className = '',
    onchange,
    onclick
  }: Props = $props();

  // Size mappings
  const sizeClasses: Record<Size, string> = {
    xs: 'h-7 px-2 text-xs',
    sm: 'h-8 px-2.5 text-sm',
    md: 'h-10 px-3 text-base',
    lg: 'h-12 px-4 text-lg',
    xl: 'h-14 px-5 text-xl'
  };

  // Color mappings
  const colorClasses: Record<ColorPalette, { base: string; selected: string; hover: string }> = {
    teal: {
      base: 'text-slate-300 bg-slate-700/50',
      selected: 'text-white bg-teal-600 shadow-[0_0_10px_hsl(176_55%_45%/0.3)]',
      hover: 'hover:bg-slate-600/50'
    },
    coral: {
      base: 'text-slate-300 bg-slate-700/50',
      selected: 'text-white bg-coral-600 shadow-[0_0_10px_hsl(12_65%_50%/0.3)]',
      hover: 'hover:bg-slate-600/50'
    },
    purple: {
      base: 'text-slate-300 bg-slate-700/50',
      selected: 'text-white bg-purple-600 shadow-[0_0_10px_hsl(270_55%_52%/0.3)]',
      hover: 'hover:bg-slate-600/50'
    },
    pink: {
      base: 'text-slate-300 bg-slate-700/50',
      selected: 'text-white bg-pink-600 shadow-[0_0_10px_hsl(330_60%_52%/0.3)]',
      hover: 'hover:bg-slate-600/50'
    },
    mint: {
      base: 'text-slate-300 bg-slate-700/50',
      selected: 'text-white bg-mint-600 shadow-[0_0_10px_hsl(160_45%_45%/0.3)]',
      hover: 'hover:bg-slate-600/50'
    },
    amber: {
      base: 'text-slate-300 bg-slate-700/50',
      selected: 'text-white bg-amber-600 shadow-[0_0_10px_hsl(38_70%_42%/0.3)]',
      hover: 'hover:bg-slate-600/50'
    },
    slate: {
      base: 'text-slate-300 bg-slate-700/50',
      selected: 'text-white bg-slate-500',
      hover: 'hover:bg-slate-600/50'
    },
    ghost: {
      base: 'text-slate-400 bg-transparent',
      selected: 'text-white bg-slate-700/60',
      hover: 'hover:bg-slate-700/40'
    }
  };

  function isSelected(id: string): boolean {
    if (multiple) {
      return selectedItems.includes(id);
    }
    return selected === id;
  }

  function handleClick(id: string) {
    const item = items.find(i => i.id === id);
    if (item?.disabled) return;

    onclick?.(id);

    if (multiple) {
      if (selectedItems.includes(id)) {
        selectedItems = selectedItems.filter(i => i !== id);
      } else {
        selectedItems = [...selectedItems, id];
      }
      onchange?.(selectedItems);
    } else {
      selected = id;
      onchange?.(selected);
    }
  }

  // Computed classes
  const containerClasses = $derived(
    `button-group inline-flex ${vertical ? 'flex-col' : 'flex-row'} rounded-lg overflow-hidden
    border border-slate-600/30 backdrop-blur-sm
    ${fullWidth ? 'w-full' : ''}
    ${className}`.trim()
  );

  function getButtonClasses(item: ButtonItem, index: number): string {
    const colors = colorClasses[color];
    const isFirst = index === 0;
    const isLast = index === items.length - 1;
    const active = isSelected(item.id);

    return `
      flex items-center justify-center gap-2 transition-all duration-200
      ${sizeClasses[size]}
      ${active ? colors.selected : colors.base}
      ${!active && !item.disabled ? colors.hover : ''}
      ${item.disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
      ${fullWidth ? 'flex-1' : ''}
      ${!isFirst ? (vertical ? 'border-t border-slate-600/30' : 'border-l border-slate-600/30') : ''}
      focus:outline-none focus:ring-2 focus:ring-teal-500/50 focus:z-10
    `.trim();
  }
</script>

<div class={containerClasses} role="group">
  {#each items as item, index (item.id)}
    <button
      type="button"
      class={getButtonClasses(item, index)}
      onclick={() => handleClick(item.id)}
      disabled={item.disabled}
      aria-pressed={isSelected(item.id)}
    >
      {#if item.icon}
        {@render item.icon()}
      {/if}
      <span>{item.label}</span>
    </button>
  {/each}
</div>

<style>
  .button-group {
    box-shadow: 0 2px 4px hsl(220 16% 8% / 0.3);
  }
</style>
