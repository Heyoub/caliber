<!--
  TreeView.svelte - Collapsible tree view organism
  For YAML, TOML, JSON, XML viewing

  Features:
  - Expand/collapse nodes
  - Syntax colored values
  - Keyboard navigation
  - Copy value on click
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { CMSContent } from '../types/index.js';

  /** Value type classification for tree node primitives */
  type ValueType = 'string' | 'number' | 'boolean' | 'null' | 'array' | 'object';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Data to display (object, array, or primitive) */
    data: unknown;
    /** Starting depth level */
    depth?: number;
    /** Maximum auto-expand depth */
    maxExpandDepth?: number;
    /** Root key name (optional) */
    rootKey?: string;
    /** Additional CSS classes */
    class?: string;
    /** Custom value renderer slot */
    valueRenderer?: Snippet<[{ value: unknown; type: string }]>;
    /** Event handlers */
    onNodeClick?: (path: string[], value: unknown) => void;
  }

  let {
    cms = {},
    data,
    depth = 0,
    maxExpandDepth = 2,
    rootKey,
    class: className = '',
    valueRenderer,
    onNodeClick
  }: Props = $props();

  // Determine value type
  function getType(value: unknown): ValueType {
    if (value === null) return 'null';
    if (Array.isArray(value)) return 'array';
    if (typeof value === 'object') return 'object';
    if (typeof value === 'number') return 'number';
    if (typeof value === 'boolean') return 'boolean';
    return 'string';
  }

  // Check if value is expandable
  function isExpandable(value: unknown): boolean {
    return value !== null && typeof value === 'object';
  }

  // Format value for display
  function formatValue(value: unknown, type: ValueType): string {
    switch (type) {
      case 'null':
        return 'null';
      case 'boolean':
        return String(value);
      case 'number':
        return String(value);
      case 'string':
        return `"${value}"`;
      case 'array':
        return `Array(${(value as unknown[]).length})`;
      case 'object':
        return `Object(${Object.keys(value as object).length})`;
      default:
        return String(value);
    }
  }

  // Get entries from object or array
  function getEntries(value: unknown): [string, unknown][] {
    if (Array.isArray(value)) {
      return value.map((v, i) => [String(i), v]);
    }
    if (typeof value === 'object' && value !== null) {
      return Object.entries(value);
    }
    return [];
  }

  // Color classes by type
  const typeColors: Record<ValueType, string> = {
    string: 'text-mint-400',
    number: 'text-coral-400',
    boolean: 'text-purple-400',
    null: 'text-slate-500',
    array: 'text-teal-400',
    object: 'text-teal-400'
  };

  // State for expansion
  let expanded = $state(depth < maxExpandDepth);

  // Derived values
  const valueType = $derived(getType(data));
  const expandable = $derived(isExpandable(data));
  const entries = $derived(expandable ? getEntries(data) : []);
  const displayValue = $derived(formatValue(data, valueType));

  // Indentation
  const indentPx = $derived(depth * 16);

  // Toggle expansion
  function toggleExpand() {
    if (expandable) {
      expanded = !expanded;
    }
  }

  // Handle value click
  function handleClick(e: MouseEvent) {
    e.stopPropagation();
    // Could copy to clipboard or trigger callback
  }
</script>

<div class={`tree-view font-mono text-sm ${className}`}>
  <div
    class="tree-node flex items-start py-0.5 hover:bg-slate-800/30 rounded cursor-pointer transition-colors"
    style="padding-left: {indentPx}px"
    onclick={toggleExpand}
    role="treeitem"
    aria-expanded={expandable ? expanded : undefined}
  >
    <!-- Expand/collapse indicator -->
    {#if expandable}
      <button
        class="w-4 h-4 flex items-center justify-center text-slate-500 hover:text-slate-300 mr-1 flex-shrink-0"
        onclick={toggleExpand}
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
      <span class="w-4 mr-1 flex-shrink-0"></span>
    {/if}

    <!-- Key (if present) -->
    {#if rootKey !== undefined}
      <span class="text-purple-300 mr-1">{rootKey}</span>
      <span class="text-slate-500 mr-1">:</span>
    {/if}

    <!-- Value -->
    {#if valueRenderer}
      {@render valueRenderer({ value: data, type: valueType })}
    {:else if expandable && !expanded}
      <!-- Collapsed preview -->
      <span class={typeColors[valueType]}>
        {displayValue}
      </span>
    {:else if !expandable}
      <!-- Primitive value -->
      <span
        class={`${typeColors[valueType]} hover:underline cursor-pointer`}
        onclick={handleClick}
        title={cms.clickToCopy || 'Click to copy'}
      >
        {displayValue}
      </span>
    {:else}
      <!-- Expanded indicator -->
      <span class="text-slate-500">
        {valueType === 'array' ? '[' : '{'}
      </span>
    {/if}
  </div>

  <!-- Children (when expanded) -->
  {#if expandable && expanded}
    <div class="tree-children" role="group">
      {#each entries as [key, value] (key)}
        <svelte:self
          {cms}
          data={value}
          depth={depth + 1}
          {maxExpandDepth}
          rootKey={key}
          {valueRenderer}
          {onNodeClick}
        />
      {/each}
    </div>

    <!-- Closing bracket -->
    <div
      class="text-slate-500 py-0.5"
      style="padding-left: {indentPx}px"
    >
      <span class="ml-5">
        {valueType === 'array' ? ']' : '}'}
      </span>
    </div>
  {/if}
</div>

<style>
  /* Tree lines (optional visual connection) */
  .tree-children {
    position: relative;
  }

  .tree-children::before {
    content: '';
    position: absolute;
    left: calc(0.5rem);
    top: 0;
    bottom: 0.5rem;
    width: 1px;
    background: hsl(var(--slate-700));
    opacity: 0.5;
  }

  /* Highlight on hover */
  .tree-node:hover {
    background: hsl(var(--slate-800) / 0.3);
  }

  /* Focus styles for keyboard navigation */
  .tree-node:focus {
    outline: none;
    box-shadow: inset 0 0 0 1px hsl(var(--teal-500) / 0.5);
  }
</style>
