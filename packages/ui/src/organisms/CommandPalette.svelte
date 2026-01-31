<!--
  CommandPalette.svelte - Keyboard command interface
  Based on ARCHITECTURE.md MCP UI Patterns

  Features:
  - Fuzzy search
  - Keyboard navigation
  - Command categories
  - Shortcut display
  - For MCP prompts
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Command, Prompt, CMSContent } from '../types/index.js';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Whether palette is open */
    open?: boolean;
    /** Available commands */
    commands?: Command[];
    /** MCP prompts (alternative to commands) */
    prompts?: Prompt[];
    /** Placeholder text */
    placeholder?: string;
    /** Additional CSS classes */
    class?: string;
    /** Custom command item renderer */
    itemRenderer?: Snippet<[{ item: Command | Prompt; selected: boolean; index: number }]>;
    /** Custom empty state slot */
    emptyState?: Snippet;
    /** Event handlers */
    onClose?: () => void;
    onSelect?: (item: Command | Prompt) => void;
  }

  let {
    cms = {},
    open = false,
    commands = [],
    prompts = [],
    placeholder = cms.searchPlaceholder || 'Type a command or search...',
    class: className = '',
    itemRenderer,
    emptyState,
    onClose,
    onSelect
  }: Props = $props();

  // State
  let searchQuery = $state('');
  let selectedIndex = $state(0);
  let inputRef: HTMLInputElement | undefined = $state();

  // Combine commands and prompts into unified list
  const allItems = $derived([
    ...commands.map(c => ({ ...c, _type: 'command' as const })),
    ...prompts.map(p => ({ ...p, _type: 'prompt' as const, action: () => {} }))
  ]);

  // Fuzzy search filter
  function fuzzyMatch(query: string, text: string): boolean {
    if (!query) return true;

    const queryLower = query.toLowerCase();
    const textLower = text.toLowerCase();

    // Simple contains match (could be enhanced with proper fuzzy matching)
    if (textLower.includes(queryLower)) return true;

    // Character-by-character match
    let qi = 0;
    for (let ti = 0; ti < textLower.length && qi < queryLower.length; ti++) {
      if (textLower[ti] === queryLower[qi]) {
        qi++;
      }
    }
    return qi === queryLower.length;
  }

  // Filtered items based on search
  const filteredItems = $derived(
    allItems.filter(item =>
      fuzzyMatch(searchQuery, item.label || item.name || '') ||
      fuzzyMatch(searchQuery, item.description || '') ||
      fuzzyMatch(searchQuery, item.category || '')
    )
  );

  // Group items by category
  const groupedItems = $derived.by(() => {
    const groups: Record<string, typeof filteredItems> = {};

    for (const item of filteredItems) {
      const category = item.category || (item._type === 'prompt' ? 'Prompts' : 'Commands');
      if (!groups[category]) {
        groups[category] = [];
      }
      groups[category].push(item);
    }

    return groups;
  });

  // Flat list for keyboard navigation
  const flatFilteredItems = $derived(
    Object.values(groupedItems).flat()
  );

  // Reset selection when search changes
  $effect(() => {
    searchQuery;
    selectedIndex = 0;
  });

  // Focus input when opened
  $effect(() => {
    if (open && inputRef) {
      inputRef.focus();
      searchQuery = '';
    }
  });

  // Keyboard navigation
  function handleKeyDown(e: KeyboardEvent) {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, flatFilteredItems.length - 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        break;
      case 'Enter':
        e.preventDefault();
        if (flatFilteredItems[selectedIndex]) {
          handleSelect(flatFilteredItems[selectedIndex]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        onClose?.();
        break;
    }
  }

  // Handle item selection
  function handleSelect(item: typeof allItems[number]) {
    onSelect?.(item);
    if ('action' in item && typeof item.action === 'function') {
      item.action();
    }
    onClose?.();
  }

  // Get item index in flat list
  function getFlatIndex(item: typeof allItems[number]): number {
    return flatFilteredItems.indexOf(item);
  }
</script>

{#if open}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm"
    onclick={() => onClose?.()}
    role="presentation"
  ></div>

  <!-- Palette -->
  <div
    class={`fixed z-50 top-1/4 left-1/2 -translate-x-1/2 w-full max-w-xl bg-slate-900 rounded-xl border border-slate-800 shadow-2xl overflow-hidden ${className}`}
    role="dialog"
    aria-modal="true"
    aria-label={cms.paletteLabel || 'Command palette'}
  >
    <!-- Search input -->
    <div class="flex items-center gap-3 px-4 py-3 border-b border-slate-800">
      <svg class="w-5 h-5 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
      <input
        bind:this={inputRef}
        bind:value={searchQuery}
        type="text"
        class="flex-1 bg-transparent text-slate-100 text-lg placeholder-slate-500 focus:outline-none"
        {placeholder}
        onkeydown={handleKeyDown}
      />
      <kbd class="hidden sm:flex items-center gap-1 px-2 py-1 text-xs text-slate-500 bg-slate-800 rounded">
        <span>esc</span>
      </kbd>
    </div>

    <!-- Results -->
    <div class="max-h-80 overflow-y-auto">
      {#if flatFilteredItems.length === 0}
        {#if emptyState}
          {@render emptyState()}
        {:else}
          <div class="flex items-center justify-center py-8 text-slate-500">
            <div class="text-center">
              <svg class="w-12 h-12 mx-auto mb-2 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <p class="text-sm">{cms.noResults || 'No results found'}</p>
            </div>
          </div>
        {/if}
      {:else}
        {#each Object.entries(groupedItems) as [category, items]}
          <!-- Category header -->
          <div class="px-4 py-2 text-xs font-medium text-slate-500 uppercase tracking-wider bg-slate-850/50">
            {category}
          </div>

          <!-- Items -->
          {#each items as item, i (item.id || item.name)}
            {@const flatIndex = getFlatIndex(item)}
            {@const isSelected = flatIndex === selectedIndex}

            {#if itemRenderer}
              {@render itemRenderer({ item, selected: isSelected, index: flatIndex })}
            {:else}
              <button
                class={`
                  w-full flex items-center gap-3 px-4 py-3 text-left transition-colors
                  ${isSelected ? 'bg-teal-500/20 text-teal-100' : 'text-slate-300 hover:bg-slate-800/50'}
                `}
                onclick={() => handleSelect(item)}
                onmouseenter={() => selectedIndex = flatIndex}
              >
                <!-- Icon -->
                <div class={`
                  w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0
                  ${item._type === 'prompt' ? 'bg-purple-500/20 text-purple-400' : 'bg-slate-800 text-slate-400'}
                `}>
                  {#if item._type === 'prompt'}
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
                    </svg>
                  {:else if item.icon}
                    <span class="text-sm">{item.icon}</span>
                  {:else}
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                    </svg>
                  {/if}
                </div>

                <!-- Content -->
                <div class="flex-1 min-w-0">
                  <div class="font-medium truncate">
                    {item.label || item.name}
                  </div>
                  {#if item.description}
                    <div class="text-xs text-slate-500 truncate">
                      {item.description}
                    </div>
                  {/if}
                </div>

                <!-- Shortcut -->
                {#if 'shortcut' in item && item.shortcut}
                  <kbd class="hidden sm:flex items-center gap-1 px-2 py-1 text-xs text-slate-500 bg-slate-800 rounded flex-shrink-0">
                    {item.shortcut}
                  </kbd>
                {/if}

                <!-- Arrow indicator -->
                {#if isSelected}
                  <svg class="w-4 h-4 text-teal-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                  </svg>
                {/if}
              </button>
            {/if}
          {/each}
        {/each}
      {/if}
    </div>

    <!-- Footer -->
    <div class="flex items-center justify-between px-4 py-2 border-t border-slate-800 text-xs text-slate-500">
      <div class="flex items-center gap-4">
        <span class="flex items-center gap-1">
          <kbd class="px-1.5 py-0.5 bg-slate-800 rounded">↑</kbd>
          <kbd class="px-1.5 py-0.5 bg-slate-800 rounded">↓</kbd>
          <span>{cms.navigateLabel || 'navigate'}</span>
        </span>
        <span class="flex items-center gap-1">
          <kbd class="px-1.5 py-0.5 bg-slate-800 rounded">↵</kbd>
          <span>{cms.selectLabel || 'select'}</span>
        </span>
      </div>
      <span>{flatFilteredItems.length} {cms.resultsLabel || 'results'}</span>
    </div>
  </div>
{/if}

<style>
  /* Smooth scrollbar */
  .overflow-y-auto::-webkit-scrollbar {
    width: 6px;
  }

  .overflow-y-auto::-webkit-scrollbar-track {
    background: transparent;
  }

  .overflow-y-auto::-webkit-scrollbar-thumb {
    background: hsl(var(--slate-700));
    border-radius: 3px;
  }

  /* Keyboard shortcut styling */
  kbd {
    font-family: var(--font-mono, monospace);
  }
</style>
