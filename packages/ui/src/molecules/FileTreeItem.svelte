<script lang="ts">
  /**
   * FileTreeItem - Single tree node with icon, name, badge
   * Supports expand/collapse and context menu
   */
  import type { Snippet } from 'svelte';

  type FileType = 'file' | 'folder' | 'yaml' | 'toml' | 'json' | 'markdown' | 'xml' | 'csv';

  interface Props {
    /** Item name */
    name: string;
    /** File type for icon */
    type?: FileType;
    /** Custom icon snippet */
    icon?: Snippet;
    /** Badge content */
    badge?: string | number;
    /** Badge color */
    badgeColor?: 'teal' | 'coral' | 'purple' | 'mint' | 'amber';
    /** Has children (shows expand arrow) */
    hasChildren?: boolean;
    /** Expanded state */
    expanded?: boolean;
    /** Selected state */
    selected?: boolean;
    /** Indentation level */
    level?: number;
    /** Disabled state */
    disabled?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Children slot for nested items */
    children?: Snippet;
    /** Click handler */
    onclick?: () => void;
    /** Double-click handler */
    ondblclick?: () => void;
    /** Expand toggle handler */
    onexpand?: (expanded: boolean) => void;
    /** Context menu handler */
    oncontextmenu?: (event: MouseEvent) => void;
  }

  let {
    name,
    type = 'file',
    icon,
    badge,
    badgeColor = 'teal',
    hasChildren = false,
    expanded = $bindable(false),
    selected = false,
    level = 0,
    disabled = false,
    class: className = '',
    children,
    onclick,
    ondblclick,
    onexpand,
    oncontextmenu
  }: Props = $props();

  // File type icons (SVG paths)
  const fileIcons: Record<FileType, { path: string; color: string }> = {
    file: {
      path: 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z M14 2v6h6',
      color: 'text-slate-400'
    },
    folder: {
      path: 'M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z',
      color: 'text-amber-400'
    },
    yaml: {
      path: 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z M14 2v6h6 M8 13h2 M8 17h2 M14 13h2 M14 17h2',
      color: 'text-teal-400'
    },
    toml: {
      path: 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z M14 2v6h6 M10 13l4 4 M10 17l4-4',
      color: 'text-amber-400'
    },
    json: {
      path: 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z M14 2v6h6 M8 13h.01 M12 13h.01 M16 13h.01 M8 17h.01 M12 17h.01',
      color: 'text-purple-400'
    },
    markdown: {
      path: 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z M14 2v6h6 M8 13v4 M8 13l2 2 2-2 M16 17v-4 M14 15h4',
      color: 'text-slate-300'
    },
    xml: {
      path: 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z M14 2v6h6 M8 13l3 3-3 3 M16 13l-3 3 3 3',
      color: 'text-coral-400'
    },
    csv: {
      path: 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z M14 2v6h6 M8 13h8 M8 17h8 M12 13v4',
      color: 'text-mint-400'
    }
  };

  // Badge color classes
  const badgeColors: Record<string, string> = {
    teal: 'bg-teal-500/20 text-teal-300',
    coral: 'bg-coral-500/20 text-coral-300',
    purple: 'bg-purple-500/20 text-purple-300',
    mint: 'bg-mint-500/20 text-mint-300',
    amber: 'bg-amber-500/20 text-amber-300'
  };

  function handleClick(e: MouseEvent) {
    if (disabled) return;
    onclick?.();
  }

  function handleDblClick(e: MouseEvent) {
    if (disabled) return;
    ondblclick?.();
  }

  function handleExpandClick(e: MouseEvent) {
    e.stopPropagation();
    if (disabled || !hasChildren) return;
    expanded = !expanded;
    onexpand?.(expanded);
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    if (disabled) return;
    oncontextmenu?.(e);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (disabled) return;

    switch (e.key) {
      case 'Enter':
      case ' ':
        e.preventDefault();
        onclick?.();
        break;
      case 'ArrowRight':
        if (hasChildren && !expanded) {
          e.preventDefault();
          expanded = true;
          onexpand?.(true);
        }
        break;
      case 'ArrowLeft':
        if (hasChildren && expanded) {
          e.preventDefault();
          expanded = false;
          onexpand?.(false);
        }
        break;
    }
  }

  const iconDef = $derived(fileIcons[type]);
  const indent = $derived(level * 16);
</script>

<div class="file-tree-item">
  <!-- Tree item row -->
  <div
    class={`
      flex items-center gap-1.5 py-1 px-2 rounded-md cursor-pointer
      transition-all duration-150
      ${selected
        ? 'bg-teal-500/20 text-white'
        : 'text-slate-300 hover:bg-slate-700/50 hover:text-white'
      }
      ${disabled ? 'opacity-50 cursor-not-allowed' : ''}
      ${className}
    `}
    style={`padding-left: ${indent + 8}px`}
    onclick={handleClick}
    ondblclick={handleDblClick}
    oncontextmenu={handleContextMenu}
    onkeydown={handleKeydown}
    role="treeitem"
    tabindex={disabled ? -1 : 0}
    aria-selected={selected}
    aria-expanded={hasChildren ? expanded : undefined}
  >
    <!-- Expand/collapse arrow -->
    {#if hasChildren}
      <button
        type="button"
        onclick={handleExpandClick}
        class="flex-shrink-0 w-4 h-4 flex items-center justify-center
               text-slate-500 hover:text-slate-300 transition-transform duration-200"
        style={`transform: rotate(${expanded ? 90 : 0}deg)`}
        tabindex={-1}
        aria-label={expanded ? 'Collapse' : 'Expand'}
      >
        <svg
          class="w-3 h-3"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="m9 18 6-6-6-6" />
        </svg>
      </button>
    {:else}
      <span class="w-4 flex-shrink-0"></span>
    {/if}

    <!-- Icon -->
    <span class={`flex-shrink-0 ${iconDef.color}`}>
      {#if icon}
        {@render icon()}
      {:else}
        <svg
          class="w-4 h-4"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d={iconDef.path} />
        </svg>
      {/if}
    </span>

    <!-- Name -->
    <span class="flex-1 truncate text-sm">
      {name}
    </span>

    <!-- Badge -->
    {#if badge !== undefined}
      <span class={`flex-shrink-0 px-1.5 py-0.5 text-xs rounded ${badgeColors[badgeColor]}`}>
        {badge}
      </span>
    {/if}
  </div>

  <!-- Children (expanded) -->
  {#if hasChildren && expanded && children}
    <div role="group" class="file-tree-children">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .file-tree-item:focus-within > div:first-child {
    outline: 2px solid hsl(176 55% 45% / 0.5);
    outline-offset: -2px;
  }
</style>
