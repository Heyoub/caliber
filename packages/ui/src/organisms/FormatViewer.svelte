<!--
  FormatViewer.svelte - Smart format viewer organism
  Based on ARCHITECTURE.md Editor Specification

  Features:
  - Detects format and renders appropriately
  - Markdown -> rendered HTML
  - YAML/TOML/JSON/XML -> tree view
  - CSV -> table
  - Mermaid -> diagram
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { FileFormat, CMSContent } from '../types/index.js';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Content to render */
    content: string;
    /** File format (auto-detected if not provided) */
    format?: FileFormat;
    /** Additional CSS classes */
    class?: string;
    /** Custom markdown renderer slot */
    markdownRenderer?: Snippet<[{ content: string }]>;
    /** Custom tree view slot */
    treeRenderer?: Snippet<[{ data: unknown; format: FileFormat }]>;
    /** Custom table renderer slot */
    tableRenderer?: Snippet<[{ headers: string[]; rows: string[][] }]>;
    /** Custom diagram renderer slot */
    diagramRenderer?: Snippet<[{ content: string }]>;
  }

  let {
    cms = {},
    content,
    format,
    class: className = '',
    markdownRenderer,
    treeRenderer,
    tableRenderer,
    diagramRenderer
  }: Props = $props();

  // Auto-detect format from content if not provided
  const detectedFormat = $derived.by((): FileFormat => {
    if (format) return format;

    const trimmed = content.trim();

    // JSON detection
    if ((trimmed.startsWith('{') && trimmed.endsWith('}')) ||
        (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
      try {
        JSON.parse(trimmed);
        return 'json';
      } catch {
        // Not valid JSON
      }
    }

    // XML detection
    if (trimmed.startsWith('<?xml') || (trimmed.startsWith('<') && trimmed.includes('</'))) {
      return 'xml';
    }

    // YAML detection (basic - has colons with values)
    if (/^[\w-]+:\s*.+/m.test(trimmed) && !trimmed.includes('[') && trimmed.includes('\n')) {
      return 'yaml';
    }

    // TOML detection
    if (/^\[[\w.-]+\]/m.test(trimmed) || /^[\w-]+\s*=\s*.+/m.test(trimmed)) {
      return 'toml';
    }

    // CSV detection (has commas and multiple lines)
    if (trimmed.includes(',') && trimmed.includes('\n') && !trimmed.includes('{')) {
      return 'csv';
    }

    // Mermaid detection
    if (trimmed.startsWith('graph ') || trimmed.startsWith('sequenceDiagram') ||
        trimmed.startsWith('classDiagram') || trimmed.startsWith('flowchart')) {
      return 'mermaid';
    }

    // Default to markdown
    return 'markdown';
  });

  // Parse structured data
  const parsedData = $derived.by(() => {
    try {
      switch (detectedFormat) {
        case 'json':
          return JSON.parse(content);
        case 'csv':
          return parseCSV(content);
        default:
          return null;
      }
    } catch {
      return null;
    }
  });

  // Simple CSV parser
  function parseCSV(csv: string): { headers: string[]; rows: string[][] } {
    const lines = csv.trim().split('\n');
    const headers = lines[0]?.split(',').map(h => h.trim()) || [];
    const rows = lines.slice(1).map(line =>
      line.split(',').map(cell => cell.trim())
    );
    return { headers, rows };
  }

  // Escape HTML for display
  function escapeHtml(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  }

  // Format label
  const formatLabel = $derived(
    detectedFormat.toUpperCase()
  );
</script>

<div class={`format-viewer h-full overflow-auto ${className}`}>
  <!-- Format indicator -->
  <div class="sticky top-0 z-10 flex items-center justify-between px-4 py-2 bg-slate-900/95 backdrop-blur-sm border-b border-slate-800">
    <span class="text-xs font-medium text-teal-400">{formatLabel}</span>
    <span class="text-xs text-slate-500">{cms.viewerLabel || 'Viewer'}</span>
  </div>

  <div class="p-4">
    <!-- Markdown -->
    {#if detectedFormat === 'markdown'}
      {#if markdownRenderer}
        {@render markdownRenderer({ content })}
      {:else}
        <div class="prose prose-invert prose-sm max-w-none">
          <!-- In production, use a proper markdown renderer -->
          <pre class="whitespace-pre-wrap text-slate-300 text-sm">{content}</pre>
        </div>
      {/if}

    <!-- JSON/YAML/TOML/XML - Tree view -->
    {:else if ['json', 'yaml', 'toml', 'xml'].includes(detectedFormat)}
      {#if treeRenderer}
        {@render treeRenderer({ data: parsedData, format: detectedFormat })}
      {:else}
        <div class="font-mono text-sm">
          {#if detectedFormat === 'json' && parsedData}
            <pre class="text-slate-300 bg-slate-950/50 rounded-lg p-4 overflow-x-auto">{JSON.stringify(parsedData, null, 2)}</pre>
          {:else}
            <!-- Syntax-highlighted code view -->
            <pre class="text-slate-300 bg-slate-950/50 rounded-lg p-4 overflow-x-auto">{content}</pre>
          {/if}
        </div>
      {/if}

    <!-- CSV - Table view -->
    {:else if detectedFormat === 'csv' && parsedData}
      {#if tableRenderer}
        {@render tableRenderer(parsedData)}
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-sm">
            <thead>
              <tr class="bg-slate-800">
                {#each parsedData.headers as header}
                  <th class="px-3 py-2 text-left text-xs font-medium text-teal-400 uppercase border-b border-slate-700">
                    {header}
                  </th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each parsedData.rows as row, i}
                <tr class="border-b border-slate-800 hover:bg-slate-800/50 transition-colors">
                  {#each row as cell}
                    <td class="px-3 py-2 text-slate-300">
                      {cell}
                    </td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

    <!-- Mermaid - Diagram -->
    {:else if detectedFormat === 'mermaid'}
      {#if diagramRenderer}
        {@render diagramRenderer({ content })}
      {:else}
        <div class="mermaid-container bg-slate-950/50 rounded-lg p-4">
          <!-- In production, use mermaid.js to render -->
          <div class="text-center text-slate-500">
            <svg class="w-12 h-12 mx-auto mb-2 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" />
            </svg>
            <p class="text-sm">{cms.diagramPlaceholder || 'Mermaid diagram'}</p>
            <pre class="mt-4 text-xs text-left text-slate-400 bg-slate-900 rounded p-2">{content}</pre>
          </div>
        </div>
      {/if}

    <!-- Fallback - Raw content -->
    {:else}
      <pre class="whitespace-pre-wrap text-slate-300 text-sm font-mono">{escapeHtml(content)}</pre>
    {/if}
  </div>
</div>

<style>
  /* Table styling */
  table {
    border-spacing: 0;
  }

  /* Prose overrides for dark theme */
  .prose {
    --tw-prose-body: hsl(var(--slate-300));
    --tw-prose-headings: hsl(var(--slate-100));
    --tw-prose-links: hsl(var(--teal-400));
    --tw-prose-code: hsl(var(--purple-300));
  }
</style>
