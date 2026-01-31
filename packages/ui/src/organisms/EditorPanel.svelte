<!--
  EditorPanel.svelte - Code editor container organism
  Based on ARCHITECTURE.md Editor Specification

  Features:
  - Tab bar for open files
  - CodeMirror 6 slot
  - Status bar (line, col, format)
  - Edit/Preview mode toggle
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { EditorTab, EditorPosition, FileFormatLiteral, CMSContent } from '../types/index.js';

  type ViewMode = 'edit' | 'preview' | 'split';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Open editor tabs */
    tabs?: EditorTab[];
    /** Currently active tab ID */
    activeTabId?: string;
    /** Current cursor position */
    position?: EditorPosition;
    /** Current view mode */
    viewMode?: ViewMode;
    /** File encoding */
    encoding?: string;
    /** Indent style (spaces/tabs) */
    indentStyle?: string;
    /** Additional CSS classes */
    class?: string;
    /** Tab bar slot */
    tabBar?: Snippet;
    /** Editor slot (for CodeMirror) */
    editor?: Snippet;
    /** Preview slot (for format viewers) */
    preview?: Snippet;
    /** Status bar slot */
    statusBar?: Snippet;
    /** Toolbar slot */
    toolbar?: Snippet;
    /** Event handlers */
    onTabSelect?: (tabId: string) => void;
    onTabClose?: (tabId: string) => void;
    onNewTab?: () => void;
    onViewModeChange?: (mode: ViewMode) => void;
    onSave?: () => void;
    onFormat?: () => void;
  }

  let {
    cms = {},
    tabs = [],
    activeTabId = '',
    position = { line: 1, column: 1 },
    viewMode = 'edit',
    encoding = 'UTF-8',
    indentStyle = '2 spaces',
    class: className = '',
    tabBar,
    editor,
    preview,
    statusBar,
    toolbar,
    onTabSelect,
    onTabClose,
    onNewTab,
    onViewModeChange,
    onSave,
    onFormat
  }: Props = $props();

  // Derived values
  const activeTab = $derived(tabs.find(t => t.id === activeTabId));
  const hasTabs = $derived(tabs.length > 0);
  const isSplitView = $derived(viewMode === 'split');

  // Format display names (partial - only common formats)
  const formatNames: Partial<Record<FileFormatLiteral, string>> = {
    markdown: 'Markdown',
    yaml: 'YAML',
    toml: 'TOML',
    json: 'JSON',
    xml: 'XML',
    html: 'HTML',
    css: 'CSS',
    javascript: 'JavaScript',
    typescript: 'TypeScript',
    python: 'Python',
    rust: 'Rust',
    go: 'Go',
    sql: 'SQL',
    shell: 'Shell',
    csv: 'CSV',
    latex: 'LaTeX',
    mermaid: 'Mermaid',
    plaintext: 'Plain Text',
    unknown: 'Unknown'
  };

  // Format icons (simplified)
  const formatIcons: Partial<Record<FileFormatLiteral, string>> = {
    markdown: 'M',
    yaml: 'Y',
    toml: 'T',
    json: 'J',
    xml: 'X',
    csv: 'C',
    mermaid: 'D'
  };

  // View mode options
  const viewModes: { id: ViewMode; label: string; icon: string }[] = [
    { id: 'edit', label: cms.editLabel || 'Edit', icon: 'edit' },
    { id: 'preview', label: cms.previewLabel || 'Preview', icon: 'eye' },
    { id: 'split', label: cms.splitLabel || 'Split', icon: 'columns' }
  ];
</script>

<div class={`flex flex-col h-full bg-slate-900 rounded-xl overflow-hidden border border-slate-800 ${className}`}>
  <!-- Tab Bar -->
  <div class="flex items-center justify-between bg-slate-850 border-b border-slate-800">
    {#if tabBar}
      {@render tabBar()}
    {:else}
      <div class="flex items-center overflow-x-auto" role="tablist">
        {#each tabs as tab (tab.id)}
          <div
            class={`group flex items-center gap-2 px-4 py-2.5 text-sm border-r border-slate-800 transition-colors cursor-pointer ${
              tab.id === activeTabId
                ? 'bg-slate-900 text-slate-100'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
            }`}
            role="tab"
            tabindex="0"
            aria-selected={tab.id === activeTabId}
            onclick={() => onTabSelect?.(tab.id)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onTabSelect?.(tab.id); }}
          >
            <!-- Format indicator -->
            <span class="w-5 h-5 flex items-center justify-center rounded bg-slate-700 text-[10px] font-bold text-teal-400">
              {formatIcons[tab.format] || '?'}
            </span>

            <!-- Tab name -->
            <span class="max-w-32 truncate">{tab.name}</span>

            <!-- Dirty indicator -->
            {#if tab.dirty || tab.isDirty}
              <span class="w-2 h-2 rounded-full bg-coral-400"></span>
            {/if}

            <!-- Close button -->
            <button
              type="button"
              class="ml-1 p-0.5 rounded hover:bg-slate-700 opacity-0 group-hover:opacity-100 transition-opacity"
              onclick={(e) => { e.stopPropagation(); onTabClose?.(tab.id); }}
              aria-label="Close tab"
            >
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        {/each}

        <!-- New tab button -->
        <button
          class="px-3 py-2.5 text-slate-500 hover:text-slate-300 hover:bg-slate-800/50 transition-colors"
          onclick={() => onNewTab?.()}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
        </button>
      </div>

      <!-- View mode toggle -->
      <div class="flex items-center gap-1 px-2">
        {#each viewModes as mode}
          <button
            class={`px-2.5 py-1.5 text-xs font-medium rounded transition-colors ${
              viewMode === mode.id
                ? 'bg-teal-500/20 text-teal-300'
                : 'text-slate-500 hover:text-slate-300 hover:bg-slate-800/50'
            }`}
            onclick={() => onViewModeChange?.(mode.id)}
          >
            {mode.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Toolbar (optional) -->
  {#if toolbar}
    <div class="flex items-center gap-2 px-4 py-2 bg-slate-850/50 border-b border-slate-800">
      {@render toolbar()}
    </div>
  {/if}

  <!-- Main Editor Area -->
  <div class="flex-1 flex overflow-hidden">
    {#if !hasTabs}
      <!-- Empty state -->
      <div class="flex-1 flex items-center justify-center text-slate-500">
        <div class="text-center">
          <svg class="w-12 h-12 mx-auto mb-3 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
          <p class="text-sm">{cms.emptyLabel || 'No files open'}</p>
          <p class="text-xs mt-1 text-slate-600">{cms.emptyHint || 'Open a file from the sidebar'}</p>
        </div>
      </div>
    {:else}
      <!-- Edit view -->
      {#if viewMode === 'edit' || viewMode === 'split'}
        <div class={isSplitView ? 'w-1/2 border-r border-slate-800' : 'flex-1'}>
          {#if editor}
            {@render editor()}
          {:else}
            <div class="h-full bg-slate-950 p-4 font-mono text-sm text-slate-300">
              <!-- CodeMirror would mount here -->
              <div class="text-slate-500">{cms.editorPlaceholder || 'Editor content'}</div>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Preview view -->
      {#if viewMode === 'preview' || viewMode === 'split'}
        <div class={isSplitView ? 'w-1/2' : 'flex-1'}>
          {#if preview}
            {@render preview()}
          {:else}
            <div class="h-full bg-slate-900 p-4 overflow-auto">
              <div class="prose prose-invert prose-sm max-w-none">
                <div class="text-slate-500">{cms.previewPlaceholder || 'Preview content'}</div>
              </div>
            </div>
          {/if}
        </div>
      {/if}
    {/if}
  </div>

  <!-- Status Bar -->
  <div class="flex items-center justify-between px-4 py-1.5 bg-slate-850 border-t border-slate-800 text-xs">
    {#if statusBar}
      {@render statusBar()}
    {:else}
      <div class="flex items-center gap-4 text-slate-500">
        <!-- Position -->
        <span>
          Ln {position.line}, Col {position.column}
        </span>

        <!-- Format -->
        {#if activeTab}
          <span class="text-teal-400">
            {formatNames[activeTab.format] || activeTab.format}
          </span>
        {/if}

        <!-- Encoding -->
        <span>{encoding}</span>

        <!-- Indent -->
        <span>{indentStyle}</span>
      </div>

      <div class="flex items-center gap-2">
        <!-- Save button -->
        {#if activeTab?.dirty || activeTab?.isDirty}
          <button
            type="button"
            class="px-2 py-0.5 text-xs text-teal-400 hover:text-teal-300 hover:bg-teal-500/10 rounded transition-colors"
            onclick={() => onSave?.()}
          >
            {cms.saveLabel || 'Save'}
          </button>
        {/if}

        <!-- Format button -->
        <button
          class="px-2 py-0.5 text-xs text-slate-400 hover:text-slate-300 hover:bg-slate-700/50 rounded transition-colors"
          onclick={() => onFormat?.()}
        >
          {cms.formatLabel || 'Format'}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Custom scrollbar for tabs */
  .overflow-x-auto::-webkit-scrollbar {
    height: 4px;
  }

  .overflow-x-auto::-webkit-scrollbar-track {
    background: transparent;
  }

  .overflow-x-auto::-webkit-scrollbar-thumb {
    background: hsl(var(--slate-700));
    border-radius: 2px;
  }
</style>
