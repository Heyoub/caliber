<script lang="ts">
  /**
   * Code Block Component
   * Syntax highlighted code display with SynthBrute aesthetic
   * Requirements: 3.2
   */

  interface Props {
    code: string;
    language?: string;
    filename?: string;
    showLineNumbers?: boolean;
  }

  let { code, language = 'rust', filename = 'example.rs', showLineNumbers = false }: Props = $props();

  // Simple syntax highlighting for Rust-like code
  function highlightCode(source: string): string {
    // Escape HTML first
    let highlighted = source
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');

    // Comments (// and /* */)
    highlighted = highlighted.replace(
      /(\/\/[^\n]*)/g,
      '<span class="token-comment">$1</span>'
    );
    highlighted = highlighted.replace(
      /(\/\*[\s\S]*?\*\/)/g,
      '<span class="token-comment">$1</span>'
    );

    // Strings
    highlighted = highlighted.replace(
      /("(?:[^"\\]|\\.)*")/g,
      '<span class="token-string">$1</span>'
    );
    highlighted = highlighted.replace(
      /('(?:[^'\\]|\\.)*')/g,
      '<span class="token-string">$1</span>'
    );

    // Keywords
    const keywords = [
      'let', 'const', 'fn', 'pub', 'struct', 'enum', 'impl', 'trait',
      'use', 'mod', 'crate', 'self', 'super', 'async', 'await',
      'if', 'else', 'match', 'for', 'while', 'loop', 'return',
      'true', 'false', 'mut', 'ref', 'where', 'type', 'dyn',
      'import', 'from', 'export', 'default', 'function', 'class',
      'interface', 'extends', 'implements'
    ];
    const keywordPattern = new RegExp(`\\b(${keywords.join('|')})\\b`, 'g');
    highlighted = highlighted.replace(
      keywordPattern,
      '<span class="token-keyword">$1</span>'
    );

    // Types (PascalCase words)
    highlighted = highlighted.replace(
      /\b([A-Z][a-zA-Z0-9_]*)\b/g,
      '<span class="token-type">$1</span>'
    );

    // Functions (word followed by parenthesis)
    highlighted = highlighted.replace(
      /\b([a-z_][a-z0-9_]*)\s*(?=\()/g,
      '<span class="token-function">$1</span>'
    );

    // Numbers
    highlighted = highlighted.replace(
      /\b(\d+(?:\.\d+)?)\b/g,
      '<span class="token-number">$1</span>'
    );

    // Operators and punctuation
    highlighted = highlighted.replace(
      /(\?|::|\.|,|;|:|\{|\}|\[|\]|\(|\)|&amp;|&lt;|&gt;|=|\+|-|\*|\/|!|\|)/g,
      '<span class="token-punctuation">$1</span>'
    );

    // Macros (word followed by !)
    highlighted = highlighted.replace(
      /\b([a-z_][a-z0-9_]*)!/g,
      '<span class="token-macro">$1!</span>'
    );

    return highlighted;
  }

  function getLines(source: string): string[] {
    return source.split('\n');
  }

  const highlightedCode = $derived(highlightCode(code));
  const lines = $derived(getLines(code));
</script>

<div class="code-block bg-[#18181b] border-2 border-[#27272a] overflow-hidden">
  <!-- Code header -->
  <div class="flex items-center gap-2 px-4 py-2 bg-[#111113] border-b border-[#27272a]">
    <div class="w-3 h-3 rounded-full bg-[#ec4899]/50"></div>
    <div class="w-3 h-3 rounded-full bg-[#f59e0b]/50"></div>
    <div class="w-3 h-3 rounded-full bg-[#22d3ee]/50"></div>
    <span class="ml-2 text-[#71717a] text-xs font-mono">{filename}</span>
  </div>
  
  <!-- Code content -->
  <div class="relative overflow-x-auto">
    {#if showLineNumbers}
      <div class="flex">
        <!-- Line numbers -->
        <div class="flex-shrink-0 py-4 pl-4 pr-2 text-right select-none border-r border-[#27272a]">
          {#each lines as _, i}
            <div class="text-xs text-[#71717a] font-mono leading-6">{i + 1}</div>
          {/each}
        </div>
        <!-- Code -->
        <pre class="flex-1 p-4 overflow-x-auto text-sm leading-6"><code class="font-mono">{@html highlightedCode}</code></pre>
      </div>
    {:else}
      <pre class="p-4 overflow-x-auto text-sm leading-6"><code class="font-mono">{@html highlightedCode}</code></pre>
    {/if}
  </div>
</div>

<style>
  .code-block {
    font-family: 'JetBrains Mono', monospace;
  }

  .code-block :global(.token-comment) {
    color: #71717a;
    font-style: italic;
  }

  .code-block :global(.token-string) {
    color: #22d3ee;
  }

  .code-block :global(.token-keyword) {
    color: #ec4899;
    font-weight: 500;
  }

  .code-block :global(.token-type) {
    color: #a855f7;
  }

  .code-block :global(.token-function) {
    color: #22d3ee;
  }

  .code-block :global(.token-number) {
    color: #f59e0b;
  }

  .code-block :global(.token-punctuation) {
    color: #a1a1aa;
  }

  .code-block :global(.token-macro) {
    color: #ec4899;
    font-weight: 600;
  }

  pre {
    margin: 0;
    white-space: pre;
  }

  code {
    color: #a1a1aa;
  }
</style>
