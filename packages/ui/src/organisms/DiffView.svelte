<!--
  DiffView.svelte - Before/after comparison organism

  Features:
  - Side-by-side or unified view
  - Line highlighting (add/remove)
  - Line numbers
  - Syntax coloring
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { DiffLine, DiffLineType, DiffViewMode, CMSContent } from '../types/index.js';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Old/before content */
    oldContent: string;
    /** New/after content */
    newContent: string;
    /** Pre-computed diff lines (optional) */
    diffLines?: DiffLine[];
    /** View mode */
    mode?: DiffViewMode;
    /** Show line numbers */
    showLineNumbers?: boolean;
    /** Old file name */
    oldName?: string;
    /** New file name */
    newName?: string;
    /** Additional CSS classes */
    class?: string;
    /** Custom line renderer slot */
    lineRenderer?: Snippet<[{ line: DiffLine }]>;
    /** Event handlers */
    onModeChange?: (mode: DiffViewMode) => void;
  }

  let {
    cms = {},
    oldContent,
    newContent,
    diffLines,
    mode = 'unified',
    showLineNumbers = true,
    oldName = cms.oldLabel || 'Before',
    newName = cms.newLabel || 'After',
    class: className = '',
    lineRenderer,
    onModeChange
  }: Props = $props();

  // Simple diff algorithm (line-by-line comparison)
  // In production, use a proper diff library like diff-match-patch
  function computeDiff(oldText: string, newText: string): DiffLine[] {
    const oldLines = oldText.split('\n');
    const newLines = newText.split('\n');
    const result: DiffLine[] = [];

    let oldIdx = 0;
    let newIdx = 0;

    while (oldIdx < oldLines.length || newIdx < newLines.length) {
      const oldLine = oldLines[oldIdx];
      const newLine = newLines[newIdx];

      if (oldLine === undefined && newLine !== undefined) {
        // Addition
        result.push({
          type: 'add',
          content: newLine,
          newLineNumber: newIdx + 1
        });
        newIdx++;
      } else if (newLine === undefined && oldLine !== undefined) {
        // Removal
        result.push({
          type: 'remove',
          content: oldLine,
          oldLineNumber: oldIdx + 1
        });
        oldIdx++;
      } else if (oldLine === newLine) {
        // Unchanged
        result.push({
          type: 'unchanged',
          content: oldLine,
          oldLineNumber: oldIdx + 1,
          newLineNumber: newIdx + 1
        });
        oldIdx++;
        newIdx++;
      } else {
        // Changed - show as remove then add
        result.push({
          type: 'remove',
          content: oldLine,
          oldLineNumber: oldIdx + 1
        });
        result.push({
          type: 'add',
          content: newLine,
          newLineNumber: newIdx + 1
        });
        oldIdx++;
        newIdx++;
      }
    }

    return result;
  }

  // Computed diff lines
  const lines = $derived(diffLines ?? computeDiff(oldContent, newContent));

  // Stats
  const stats = $derived.by(() => {
    let additions = 0;
    let deletions = 0;
    for (const line of lines) {
      if (line.type === 'add') additions++;
      if (line.type === 'remove') deletions++;
    }
    return { additions, deletions };
  });

  // Line type styling
  const lineStyles: Record<DiffLineType, { bg: string; text: string; prefix: string }> = {
    add: {
      bg: 'bg-mint-500/10',
      text: 'text-mint-300',
      prefix: '+'
    },
    remove: {
      bg: 'bg-coral-500/10',
      text: 'text-coral-300',
      prefix: '-'
    },
    unchanged: {
      bg: '',
      text: 'text-slate-400',
      prefix: ' '
    },
    header: {
      bg: 'bg-purple-500/10',
      text: 'text-purple-300',
      prefix: '@'
    }
  };

  // For split view, separate old and new lines
  const splitLines = $derived.by(() => {
    const oldLines: (DiffLine | null)[] = [];
    const newLines: (DiffLine | null)[] = [];

    for (const line of lines) {
      if (line.type === 'remove') {
        oldLines.push(line);
        newLines.push(null);
      } else if (line.type === 'add') {
        oldLines.push(null);
        newLines.push(line);
      } else {
        oldLines.push(line);
        newLines.push(line);
      }
    }

    return { oldLines, newLines };
  });
</script>

<div class={`diff-view bg-slate-900 rounded-xl overflow-hidden border border-slate-800 ${className}`}>
  <!-- Header -->
  <header class="flex items-center justify-between px-4 py-2 bg-slate-850 border-b border-slate-800">
    <div class="flex items-center gap-4">
      <!-- Stats -->
      <div class="flex items-center gap-2 text-xs">
        <span class="text-mint-400">+{stats.additions}</span>
        <span class="text-coral-400">-{stats.deletions}</span>
      </div>
    </div>

    <!-- Mode toggle -->
    <div class="flex items-center gap-1">
      <button
        class={`px-2.5 py-1 text-xs font-medium rounded transition-colors ${
          mode === 'unified'
            ? 'bg-teal-500/20 text-teal-300'
            : 'text-slate-500 hover:text-slate-300 hover:bg-slate-800/50'
        }`}
        onclick={() => onModeChange?.('unified')}
      >
        {cms.unifiedLabel || 'Unified'}
      </button>
      <button
        class={`px-2.5 py-1 text-xs font-medium rounded transition-colors ${
          mode === 'split'
            ? 'bg-teal-500/20 text-teal-300'
            : 'text-slate-500 hover:text-slate-300 hover:bg-slate-800/50'
        }`}
        onclick={() => onModeChange?.('split')}
      >
        {cms.splitLabel || 'Split'}
      </button>
    </div>
  </header>

  <!-- Diff content -->
  <div class="overflow-auto max-h-[600px]">
    {#if mode === 'unified'}
      <!-- Unified view -->
      <table class="w-full font-mono text-xs">
        <tbody>
          {#each lines as line, i (i)}
            {#if lineRenderer}
              {@render lineRenderer({ line })}
            {:else}
              <tr class={lineStyles[line.type].bg}>
                {#if showLineNumbers}
                  <td class="w-12 px-2 py-0.5 text-right text-slate-600 select-none border-r border-slate-800">
                    {line.oldLineNumber ?? ''}
                  </td>
                  <td class="w-12 px-2 py-0.5 text-right text-slate-600 select-none border-r border-slate-800">
                    {line.newLineNumber ?? ''}
                  </td>
                {/if}
                <td class="w-6 px-1 py-0.5 text-center select-none {lineStyles[line.type].text}">
                  {lineStyles[line.type].prefix}
                </td>
                <td class="px-2 py-0.5 whitespace-pre {lineStyles[line.type].text}">
                  {line.content}
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    {:else}
      <!-- Split view -->
      <div class="flex">
        <!-- Old side -->
        <div class="w-1/2 border-r border-slate-800">
          <div class="sticky top-0 px-3 py-1.5 bg-slate-850 text-xs font-medium text-coral-400 border-b border-slate-800">
            {oldName}
          </div>
          <table class="w-full font-mono text-xs">
            <tbody>
              {#each splitLines.oldLines as line, i (i)}
                {#if line === null}
                  <tr class="h-6">
                    <td colspan="3"></td>
                  </tr>
                {:else}
                  <tr class={line.type === 'remove' ? lineStyles.remove.bg : ''}>
                    {#if showLineNumbers}
                      <td class="w-10 px-2 py-0.5 text-right text-slate-600 select-none border-r border-slate-800">
                        {line.oldLineNumber ?? ''}
                      </td>
                    {/if}
                    <td class="w-4 px-1 py-0.5 text-center select-none {line.type === 'remove' ? lineStyles.remove.text : 'text-slate-600'}">
                      {line.type === 'remove' ? '-' : ''}
                    </td>
                    <td class="px-2 py-0.5 whitespace-pre {line.type === 'remove' ? lineStyles.remove.text : 'text-slate-400'}">
                      {line.content}
                    </td>
                  </tr>
                {/if}
              {/each}
            </tbody>
          </table>
        </div>

        <!-- New side -->
        <div class="w-1/2">
          <div class="sticky top-0 px-3 py-1.5 bg-slate-850 text-xs font-medium text-mint-400 border-b border-slate-800">
            {newName}
          </div>
          <table class="w-full font-mono text-xs">
            <tbody>
              {#each splitLines.newLines as line, i (i)}
                {#if line === null}
                  <tr class="h-6">
                    <td colspan="3"></td>
                  </tr>
                {:else}
                  <tr class={line.type === 'add' ? lineStyles.add.bg : ''}>
                    {#if showLineNumbers}
                      <td class="w-10 px-2 py-0.5 text-right text-slate-600 select-none border-r border-slate-800">
                        {line.newLineNumber ?? ''}
                      </td>
                    {/if}
                    <td class="w-4 px-1 py-0.5 text-center select-none {line.type === 'add' ? lineStyles.add.text : 'text-slate-600'}">
                      {line.type === 'add' ? '+' : ''}
                    </td>
                    <td class="px-2 py-0.5 whitespace-pre {line.type === 'add' ? lineStyles.add.text : 'text-slate-400'}">
                      {line.content}
                    </td>
                  </tr>
                {/if}
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Custom scrollbar */
  .overflow-auto::-webkit-scrollbar {
    width: 8px;
    height: 8px;
  }

  .overflow-auto::-webkit-scrollbar-track {
    background: hsl(var(--slate-900));
  }

  .overflow-auto::-webkit-scrollbar-thumb {
    background: hsl(var(--slate-700));
    border-radius: 4px;
  }

  .overflow-auto::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--slate-600));
  }

  /* Table styling */
  table {
    border-collapse: collapse;
  }

  tr {
    transition: background-color 0.1s;
  }

  tr:hover {
    background-color: hsl(var(--slate-800) / 0.3);
  }
</style>
