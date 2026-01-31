/**
 * KaTeX Integration Utility
 * Provides math rendering for inline and display math expressions
 */

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

export interface KatexOptions {
  /** Display mode (block) vs inline mode */
  displayMode?: boolean;
  /** Enable error recovery (render errors as text) */
  throwOnError?: boolean;
  /** Error color for invalid expressions */
  errorColor?: string;
  /** Minimum fractional thickness */
  minRuleThickness?: number;
  /** Color commands allowed */
  colorIsTextColor?: boolean;
  /** Maximum expand passes to prevent infinite loops */
  maxExpand?: number;
  /** Maximum size in ems */
  maxSize?: number;
  /** Strict mode for security */
  strict?: boolean | 'warn' | 'ignore' | 'error';
  /** Trust callback for commands */
  trust?: boolean | ((context: { command: string }) => boolean);
  /** Output format */
  output?: 'html' | 'mathml' | 'htmlAndMathml';
}

export interface KatexRenderResult {
  html: string;
  error?: string;
}

export interface MathDelimiter {
  left: string;
  right: string;
  display: boolean;
}

// ═══════════════════════════════════════════════════════════════════════════
// DEFAULT CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════

const DEFAULT_OPTIONS: KatexOptions = {
  displayMode: false,
  throwOnError: false,
  errorColor: '#cc0000',
  strict: false,
  trust: false,
  output: 'htmlAndMathml',
  maxExpand: 1000,
  maxSize: 500,
};

const DISPLAY_OPTIONS: KatexOptions = {
  ...DEFAULT_OPTIONS,
  displayMode: true,
};

// Standard math delimiters
const DELIMITERS: MathDelimiter[] = [
  { left: '$$', right: '$$', display: true },
  { left: '\\[', right: '\\]', display: true },
  { left: '\\(', right: '\\)', display: false },
  { left: '$', right: '$', display: false },
];

// ═══════════════════════════════════════════════════════════════════════════
// CUSTOM MACROS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Custom LaTeX macros for convenience
 */
export const CUSTOM_MACROS: Record<string, string> = {
  // Greek letters shortcuts
  '\\eps': '\\varepsilon',
  '\\phi': '\\varphi',

  // Common operators
  '\\argmax': '\\operatorname*{argmax}',
  '\\argmin': '\\operatorname*{argmin}',
  '\\softmax': '\\operatorname{softmax}',
  '\\sigmoid': '\\sigma',

  // Set theory
  '\\N': '\\mathbb{N}',
  '\\Z': '\\mathbb{Z}',
  '\\Q': '\\mathbb{Q}',
  '\\R': '\\mathbb{R}',
  '\\C': '\\mathbb{C}',

  // Probability
  '\\E': '\\mathbb{E}',
  '\\Var': '\\operatorname{Var}',
  '\\Cov': '\\operatorname{Cov}',
  '\\P': '\\mathbb{P}',

  // Linear algebra
  '\\T': '^\\top',
  '\\norm': '\\left\\|#1\\right\\|',
  '\\abs': '\\left|#1\\right|',
  '\\inner': '\\langle#1,#2\\rangle',

  // Machine learning
  '\\Loss': '\\mathcal{L}',
  '\\grad': '\\nabla',
  '\\KL': 'D_{\\mathrm{KL}}',
};

// ═══════════════════════════════════════════════════════════════════════════
// LAZY LOADING
// ═══════════════════════════════════════════════════════════════════════════

let katexModule: typeof import('katex') | null = null;
let loadPromise: Promise<typeof import('katex')> | null = null;

/**
 * Lazily load KaTeX module
 */
async function loadKatex(): Promise<typeof import('katex')> {
  if (katexModule) {
    return katexModule;
  }

  if (loadPromise) {
    return loadPromise;
  }

  loadPromise = import('katex').then((module) => {
    katexModule = module;
    return module;
  });

  return loadPromise;
}

/**
 * Check if KaTeX is loaded
 */
export function isKatexLoaded(): boolean {
  return katexModule !== null;
}

/**
 * Preload KaTeX (call this early to speed up first render)
 */
export async function preloadKatex(): Promise<void> {
  await loadKatex();
}

// ═══════════════════════════════════════════════════════════════════════════
// RENDERING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Render a LaTeX expression to HTML
 */
export async function renderLatex(
  expression: string,
  options: KatexOptions = {}
): Promise<KatexRenderResult> {
  try {
    const katex = await loadKatex();
    const mergedOptions = {
      ...DEFAULT_OPTIONS,
      ...options,
      macros: { ...CUSTOM_MACROS },
    };

    const html = katex.renderToString(expression, mergedOptions);
    return { html };
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown error';
    return {
      html: `<span class="katex-error" title="${escapeHtml(errorMessage)}" style="color: ${options.errorColor || '#cc0000'}">${escapeHtml(expression)}</span>`,
      error: errorMessage,
    };
  }
}

/**
 * Render inline math ($...$)
 */
export async function renderInlineMath(expression: string): Promise<KatexRenderResult> {
  return renderLatex(expression, { ...DEFAULT_OPTIONS, displayMode: false });
}

/**
 * Render display math ($$...$$)
 */
export async function renderDisplayMath(expression: string): Promise<KatexRenderResult> {
  return renderLatex(expression, { ...DISPLAY_OPTIONS, displayMode: true });
}

/**
 * Synchronous render (only works if KaTeX is already loaded)
 */
export function renderLatexSync(expression: string, options: KatexOptions = {}): KatexRenderResult {
  if (!katexModule) {
    return {
      html: `<span class="katex-loading">${escapeHtml(expression)}</span>`,
      error: 'KaTeX not loaded',
    };
  }

  try {
    const mergedOptions = {
      ...DEFAULT_OPTIONS,
      ...options,
      macros: { ...CUSTOM_MACROS },
    };

    const html = katexModule.renderToString(expression, mergedOptions);
    return { html };
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown error';
    return {
      html: `<span class="katex-error" title="${escapeHtml(errorMessage)}" style="color: ${options.errorColor || '#cc0000'}">${escapeHtml(expression)}</span>`,
      error: errorMessage,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// TEXT PROCESSING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Find all math expressions in text
 */
export function findMathExpressions(text: string): Array<{
  expression: string;
  display: boolean;
  start: number;
  end: number;
  delimiter: MathDelimiter;
}> {
  const results: Array<{
    expression: string;
    display: boolean;
    start: number;
    end: number;
    delimiter: MathDelimiter;
  }> = [];

  // Sort delimiters by length (longest first) to avoid partial matches
  const sortedDelimiters = [...DELIMITERS].sort((a, b) => b.left.length - a.left.length);

  let pos = 0;
  while (pos < text.length) {
    let found = false;

    for (const delimiter of sortedDelimiters) {
      if (text.slice(pos).startsWith(delimiter.left)) {
        const contentStart = pos + delimiter.left.length;
        const endIndex = text.indexOf(delimiter.right, contentStart);

        if (endIndex !== -1) {
          // For single $ delimiter, make sure it's not $$ being partially matched
          if (delimiter.left === '$' && text[pos + 1] === '$') {
            continue;
          }

          const expression = text.slice(contentStart, endIndex);

          // Skip empty expressions
          if (expression.trim()) {
            results.push({
              expression,
              display: delimiter.display,
              start: pos,
              end: endIndex + delimiter.right.length,
              delimiter,
            });
          }

          pos = endIndex + delimiter.right.length;
          found = true;
          break;
        }
      }
    }

    if (!found) {
      pos++;
    }
  }

  return results;
}

/**
 * Process text and render all math expressions
 */
export async function processTextWithMath(text: string): Promise<string> {
  const expressions = findMathExpressions(text);

  if (expressions.length === 0) {
    return text;
  }

  // Render all expressions in parallel
  const renderedExpressions = await Promise.all(
    expressions.map(async ({ expression, display }) => {
      const result = display
        ? await renderDisplayMath(expression)
        : await renderInlineMath(expression);
      return result.html;
    })
  );

  // Build result string
  let result = '';
  let lastEnd = 0;

  for (let i = 0; i < expressions.length; i++) {
    const { start, end, display } = expressions[i];
    const rendered = renderedExpressions[i];

    // Add text before this expression
    result += text.slice(lastEnd, start);

    // Add rendered expression
    if (display) {
      result += `<div class="katex-display">${rendered}</div>`;
    } else {
      result += rendered;
    }

    lastEnd = end;
  }

  // Add remaining text
  result += text.slice(lastEnd);

  return result;
}

/**
 * Synchronous version of processTextWithMath (requires KaTeX to be preloaded)
 */
export function processTextWithMathSync(text: string): string {
  const expressions = findMathExpressions(text);

  if (expressions.length === 0) {
    return text;
  }

  let result = '';
  let lastEnd = 0;

  for (const { expression, display, start, end } of expressions) {
    // Add text before this expression
    result += text.slice(lastEnd, start);

    // Render expression
    const rendered = renderLatexSync(expression, { displayMode: display });

    // Add rendered expression
    if (display) {
      result += `<div class="katex-display">${rendered.html}</div>`;
    } else {
      result += rendered.html;
    }

    lastEnd = end;
  }

  // Add remaining text
  result += text.slice(lastEnd);

  return result;
}

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Validate a LaTeX expression without rendering
 */
export async function validateLatex(expression: string): Promise<{
  valid: boolean;
  error?: string;
}> {
  try {
    const katex = await loadKatex();
    katex.renderToString(expression, {
      ...DEFAULT_OPTIONS,
      throwOnError: true,
      macros: { ...CUSTOM_MACROS },
    });
    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    };
  }
}

/**
 * Check if text contains any math expressions
 */
export function containsMath(text: string): boolean {
  return findMathExpressions(text).length > 0;
}

// ═══════════════════════════════════════════════════════════════════════════
// CSS GENERATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Get KaTeX CSS URL for loading in the head
 */
export function getKatexCssUrl(version = '0.16.9'): string {
  return `https://cdn.jsdelivr.net/npm/katex@${version}/dist/katex.min.css`;
}

/**
 * Generate inline styles for KaTeX (when you can't load the CSS)
 */
export function getKatexInlineStyles(): string {
  return `
    .katex-display {
      display: block;
      margin: 1em 0;
      text-align: center;
    }
    .katex-error {
      color: #cc0000;
      font-family: monospace;
      white-space: pre-wrap;
    }
    .katex-loading {
      color: #888;
      font-family: monospace;
      white-space: pre-wrap;
    }
  `;
}

// ═══════════════════════════════════════════════════════════════════════════
// UTILITY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Escape HTML entities
 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/**
 * Extract math expression from a code block
 */
export function extractMathFromCodeBlock(code: string, language: string): string | null {
  const mathLanguages = ['math', 'latex', 'tex', 'katex'];
  if (mathLanguages.includes(language.toLowerCase())) {
    return code.trim();
  }
  return null;
}

/**
 * Create a KaTeX render function for use in marked
 */
export function createMarkedKatexExtension() {
  return {
    name: 'katex',
    level: 'inline' as const,
    start(src: string) {
      const match = src.match(/\$[^$]/);
      return match ? match.index : undefined;
    },
    tokenizer(src: string) {
      // Display math: $$...$$
      const displayMatch = src.match(/^\$\$([^$]+)\$\$/);
      if (displayMatch) {
        return {
          type: 'katex',
          raw: displayMatch[0],
          text: displayMatch[1],
          display: true,
        };
      }

      // Inline math: $...$
      const inlineMatch = src.match(/^\$([^$\n]+)\$/);
      if (inlineMatch) {
        return {
          type: 'katex',
          raw: inlineMatch[0],
          text: inlineMatch[1],
          display: false,
        };
      }

      return undefined;
    },
    renderer(token: { text: string; display: boolean }) {
      const result = renderLatexSync(token.text, { displayMode: token.display });
      if (token.display) {
        return `<div class="katex-display">${result.html}</div>`;
      }
      return result.html;
    },
  };
}
