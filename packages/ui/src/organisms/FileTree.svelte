<!--
  FileTree.svelte - Memory structure browser organism
  For Trajectory/Scope/Turn hierarchy

  Features:
  - Hierarchical tree structure
  - Expand/collapse folders
  - File type icons
  - Selection state
  - Keyboard navigation
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { TreeNode, CMSContent } from '../types/index.js';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Tree nodes */
    nodes: TreeNode[];
    /** Currently selected node ID */
    selectedId?: string;
    /** Expanded node IDs */
    expandedIds?: string[];
    /** Show icons */
    showIcons?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Custom node renderer slot */
    nodeRenderer?: Snippet<[{ node: TreeNode; depth: number; selected: boolean; expanded: boolean }]>;
    /** Event handlers */
    onSelect?: (node: TreeNode) => void;
    onToggle?: (node: TreeNode) => void;
    onContextMenu?: (node: TreeNode, event: MouseEvent) => void;
  }

  let {
    cms = {},
    nodes,
    selectedId,
    expandedIds = [],
    showIcons = true,
    class: className = '',
    nodeRenderer,
    onSelect,
    onToggle,
    onContextMenu
  }: Props = $props();

  // Icon mapping by type
  const typeIcons: Record<TreeNode['type'], { icon: string; color: string }> = {
    folder: {
      icon: 'M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z',
      color: 'text-amber-400'
    },
    file: {
      icon: 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z',
      color: 'text-slate-400'
    },
    trajectory: {
      icon: 'M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z',
      color: 'text-teal-400'
    },
    scope: {
      icon: 'M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z',
      color: 'text-purple-400'
    },
    turn: {
      icon: 'M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z',
      color: 'text-pink-400'
    }
  };

  // Check if a node is expanded
  function isExpanded(nodeId: string): boolean {
    return expandedIds.includes(nodeId);
  }

  // Check if a node is selected
  function isSelected(nodeId: string): boolean {
    return selectedId === nodeId;
  }

  // Handle node click
  function handleClick(node: TreeNode) {
    onSelect?.(node);
  }

  // Handle toggle click
  function handleToggle(e: MouseEvent, node: TreeNode) {
    e.stopPropagation();
    onToggle?.(node);
  }

  // Handle context menu
  function handleContextMenu(e: MouseEvent, node: TreeNode) {
    e.preventDefault();
    onContextMenu?.(node, e);
  }

  // Keyboard navigation
  function handleKeyDown(e: KeyboardEvent, node: TreeNode) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onSelect?.(node);
    } else if (e.key === 'ArrowRight' && node.children?.length) {
      e.preventDefault();
      if (!isExpanded(node.id)) {
        onToggle?.(node);
      }
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      if (isExpanded(node.id)) {
        onToggle?.(node);
      }
    }
  }
</script>

{#snippet renderNode(node: TreeNode, depth: number)}
  {@const expanded = isExpanded(node.id)}
  {@const selected = isSelected(node.id)}
  {@const hasChildren = node.children && node.children.length > 0}
  {@const iconConfig = typeIcons[node.type] || typeIcons.file}

  <div class="tree-item" role="treeitem" aria-expanded={hasChildren ? expanded : undefined}>
    {#if nodeRenderer}
      {@render nodeRenderer({ node, depth, selected, expanded })}
    {:else}
      <button
        class={`
          w-full flex items-center gap-2 px-2 py-1.5 text-left text-sm rounded-md
          transition-colors cursor-pointer group
          ${selected
            ? 'bg-teal-500/20 text-teal-100'
            : 'text-slate-300 hover:bg-slate-800/50 hover:text-slate-100'
          }
        `}
        style="padding-left: {depth * 12 + 8}px"
        onclick={() => handleClick(node)}
        oncontextmenu={(e) => handleContextMenu(e, node)}
        onkeydown={(e) => handleKeyDown(e, node)}
      >
        <!-- Expand/collapse toggle -->
        {#if hasChildren}
          <button
            class="w-4 h-4 flex items-center justify-center text-slate-500 hover:text-slate-300 rounded transition-colors"
            onclick={(e) => handleToggle(e, node)}
            tabindex="-1"
          >
            <svg
              class="w-3 h-3 transition-transform duration-150"
              class:rotate-90={expanded}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
            </svg>
          </button>
        {:else}
          <span class="w-4"></span>
        {/if}

        <!-- Icon -->
        {#if showIcons}
          <svg
            class={`w-4 h-4 flex-shrink-0 ${iconConfig.color}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={iconConfig.icon} />
          </svg>
        {/if}

        <!-- Label -->
        <span class="flex-1 truncate">{node.label}</span>

        <!-- Optional badge/metadata -->
        {#if node.metadata?.count}
          <span class="text-xs text-slate-500 bg-slate-800 px-1.5 py-0.5 rounded">
            {node.metadata.count}
          </span>
        {/if}

        <!-- Hover actions -->
        <div class="opacity-0 group-hover:opacity-100 flex items-center gap-1 transition-opacity">
          <button
            class="p-0.5 text-slate-500 hover:text-slate-300 rounded transition-colors"
            onclick={(e) => { e.stopPropagation(); }}
            title={cms.moreActions || 'More actions'}
          >
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
            </svg>
          </button>
        </div>
      </button>
    {/if}

    <!-- Children -->
    {#if hasChildren && expanded}
      <div class="tree-children" role="group">
        {#each node.children as child (child.id)}
          {@render renderNode(child, depth + 1)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<div
  class={`file-tree overflow-auto ${className}`}
  role="tree"
  aria-label={cms.treeLabel || 'File tree'}
>
  {#if nodes.length === 0}
    <div class="flex items-center justify-center h-32 text-slate-500 text-sm">
      {cms.emptyLabel || 'No items'}
    </div>
  {:else}
    {#each nodes as node (node.id)}
      {@render renderNode(node, 0)}
    {/each}
  {/if}
</div>

<style>
  /* Tree connector lines (optional) */
  .tree-children {
    position: relative;
  }

  /* Smooth expand/collapse */
  .tree-children {
    animation: expand 0.15s ease-out;
  }

  @keyframes expand {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Custom scrollbar */
  .file-tree::-webkit-scrollbar {
    width: 6px;
  }

  .file-tree::-webkit-scrollbar-track {
    background: transparent;
  }

  .file-tree::-webkit-scrollbar-thumb {
    background: hsl(var(--slate-700));
    border-radius: 3px;
  }

  .file-tree::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--slate-600));
  }
</style>
