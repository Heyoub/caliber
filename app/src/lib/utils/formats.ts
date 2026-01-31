/**
 * Format Detection Utility
 * Auto-detects file formats from content, extensions, and MIME types
 */

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

export type FileFormat =
  | 'markdown'
  | 'yaml'
  | 'toml'
  | 'json'
  | 'xml'
  | 'csv'
  | 'tsv'
  | 'mermaid'
  | 'latex'
  | 'html'
  | 'plaintext';

export interface FormatInfo {
  format: FileFormat;
  confidence: number; // 0-1, how confident we are in the detection
  hasFrontmatter: boolean;
  frontmatterFormat?: 'yaml' | 'toml' | 'json';
}

export interface ExtensionMapping {
  extensions: string[];
  format: FileFormat;
}

// ═══════════════════════════════════════════════════════════════════════════
// EXTENSION MAPPINGS
// ═══════════════════════════════════════════════════════════════════════════

const EXTENSION_MAP: Record<string, FileFormat> = {
  // Markdown
  '.md': 'markdown',
  '.mdx': 'markdown',
  '.markdown': 'markdown',
  '.mdown': 'markdown',
  '.mkd': 'markdown',
  '.mkdn': 'markdown',

  // YAML
  '.yaml': 'yaml',
  '.yml': 'yaml',

  // TOML
  '.toml': 'toml',

  // JSON
  '.json': 'json',
  '.jsonc': 'json',
  '.json5': 'json',

  // XML
  '.xml': 'xml',
  '.xsl': 'xml',
  '.xslt': 'xml',
  '.svg': 'xml',
  '.xhtml': 'xml',
  '.rss': 'xml',
  '.atom': 'xml',
  '.plist': 'xml',

  // CSV/TSV
  '.csv': 'csv',
  '.tsv': 'tsv',
  '.tab': 'tsv',

  // LaTeX
  '.tex': 'latex',
  '.latex': 'latex',

  // HTML
  '.html': 'html',
  '.htm': 'html',

  // Mermaid
  '.mmd': 'mermaid',
  '.mermaid': 'mermaid',
};

// ═══════════════════════════════════════════════════════════════════════════
// MIME TYPE MAPPINGS
// ═══════════════════════════════════════════════════════════════════════════

const MIME_TYPE_MAP: Record<string, FileFormat> = {
  // Markdown
  'text/markdown': 'markdown',
  'text/x-markdown': 'markdown',

  // YAML
  'text/yaml': 'yaml',
  'text/x-yaml': 'yaml',
  'application/x-yaml': 'yaml',
  'application/yaml': 'yaml',

  // TOML
  'application/toml': 'toml',
  'text/x-toml': 'toml',

  // JSON
  'application/json': 'json',
  'text/json': 'json',
  'application/ld+json': 'json',
  'application/schema+json': 'json',

  // XML
  'application/xml': 'xml',
  'text/xml': 'xml',
  'application/xhtml+xml': 'xml',
  'image/svg+xml': 'xml',
  'application/rss+xml': 'xml',
  'application/atom+xml': 'xml',

  // CSV/TSV
  'text/csv': 'csv',
  'text/tab-separated-values': 'tsv',

  // LaTeX
  'application/x-latex': 'latex',
  'text/x-latex': 'latex',

  // HTML
  'text/html': 'html',

  // Plain text
  'text/plain': 'plaintext',
};

// ═══════════════════════════════════════════════════════════════════════════
// CONTENT SNIFFING PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

const YAML_FRONTMATTER_REGEX = /^---\s*\n([\s\S]*?)\n---/;
const TOML_FRONTMATTER_REGEX = /^\+\+\+\s*\n([\s\S]*?)\n\+\+\+/;
const JSON_FRONTMATTER_REGEX = /^{\s*".*":\s*(?:"[^"]*"|{[\s\S]*?})\s*}\s*\n/;

// Mermaid diagram types
const MERMAID_DIAGRAM_STARTS = [
  'graph ',
  'graph\n',
  'flowchart ',
  'flowchart\n',
  'sequenceDiagram',
  'classDiagram',
  'stateDiagram',
  'stateDiagram-v2',
  'erDiagram',
  'journey',
  'gantt',
  'pie',
  'quadrantChart',
  'requirementDiagram',
  'gitGraph',
  'mindmap',
  'timeline',
  'sankey',
  'xychart',
  'block-beta',
];

// ═══════════════════════════════════════════════════════════════════════════
// DETECTION FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Detect format from file extension
 */
export function detectFromExtension(filename: string): FileFormat | null {
  const lastDot = filename.lastIndexOf('.');
  if (lastDot === -1) return null;

  const ext = filename.slice(lastDot).toLowerCase();
  return EXTENSION_MAP[ext] ?? null;
}

/**
 * Detect format from MIME type
 */
export function detectFromMimeType(mimeType: string): FileFormat | null {
  // Normalize MIME type (remove charset, etc.)
  const normalized = mimeType.split(';')[0].trim().toLowerCase();
  return MIME_TYPE_MAP[normalized] ?? null;
}

/**
 * Check for frontmatter and return its type
 */
export function detectFrontmatter(content: string): {
  hasFrontmatter: boolean;
  format?: 'yaml' | 'toml' | 'json';
  content: string;
  frontmatter?: string;
} {
  const trimmed = content.trimStart();

  // Check YAML frontmatter (---)
  const yamlMatch = trimmed.match(YAML_FRONTMATTER_REGEX);
  if (yamlMatch) {
    return {
      hasFrontmatter: true,
      format: 'yaml',
      frontmatter: yamlMatch[1],
      content: trimmed.slice(yamlMatch[0].length).trimStart(),
    };
  }

  // Check TOML frontmatter (+++)
  const tomlMatch = trimmed.match(TOML_FRONTMATTER_REGEX);
  if (tomlMatch) {
    return {
      hasFrontmatter: true,
      format: 'toml',
      frontmatter: tomlMatch[1],
      content: trimmed.slice(tomlMatch[0].length).trimStart(),
    };
  }

  // Check JSON frontmatter (rare, but possible)
  const jsonMatch = trimmed.match(JSON_FRONTMATTER_REGEX);
  if (jsonMatch) {
    return {
      hasFrontmatter: true,
      format: 'json',
      frontmatter: jsonMatch[0].trim(),
      content: trimmed.slice(jsonMatch[0].length).trimStart(),
    };
  }

  return {
    hasFrontmatter: false,
    content,
  };
}

/**
 * Detect format from content by analyzing patterns
 */
export function detectFromContent(content: string): FormatInfo {
  const trimmed = content.trim();

  if (!trimmed) {
    return { format: 'plaintext', confidence: 0.5, hasFrontmatter: false };
  }

  // Check for frontmatter (indicates Markdown)
  const frontmatterResult = detectFrontmatter(content);
  if (frontmatterResult.hasFrontmatter) {
    return {
      format: 'markdown',
      confidence: 0.95,
      hasFrontmatter: true,
      frontmatterFormat: frontmatterResult.format,
    };
  }

  // Check for Mermaid diagrams
  for (const start of MERMAID_DIAGRAM_STARTS) {
    if (trimmed.startsWith(start)) {
      return { format: 'mermaid', confidence: 0.95, hasFrontmatter: false };
    }
  }

  // Check for JSON
  if ((trimmed.startsWith('{') && trimmed.endsWith('}')) ||
      (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
    try {
      JSON.parse(trimmed);
      return { format: 'json', confidence: 0.95, hasFrontmatter: false };
    } catch {
      // Not valid JSON, might be something else
    }
  }

  // Check for XML
  if (trimmed.startsWith('<?xml') ||
      (trimmed.startsWith('<') && trimmed.includes('</') && /<\w+[^>]*>/.test(trimmed))) {
    // Additional checks for XML
    if (trimmed.includes('<!DOCTYPE html') || /<html[\s>]/i.test(trimmed)) {
      return { format: 'html', confidence: 0.95, hasFrontmatter: false };
    }
    return { format: 'xml', confidence: 0.9, hasFrontmatter: false };
  }

  // Check for TOML
  // TOML has section headers [section] and key = value pairs
  if (/^\[[\w.-]+\]\s*$/m.test(trimmed) ||
      /^[\w-]+\s*=\s*(?:"[^"]*"|'[^']*'|true|false|\d+|\[[\s\S]*?\]|\{[\s\S]*?\})/m.test(trimmed)) {
    // Make sure it's not INI (INI has = but usually no typed values)
    const hasTypedValues = /=\s*(?:true|false|\d+\.\d+|\[\s*[^\]]*\]|\{[^}]*\})/m.test(trimmed);
    if (hasTypedValues || /^\[[\w.-]+\]$/m.test(trimmed)) {
      return { format: 'toml', confidence: 0.85, hasFrontmatter: false };
    }
  }

  // Check for YAML
  // YAML has key: value pairs, often with indentation
  if (/^[\w-]+:\s*(?:.+|$)/m.test(trimmed)) {
    // Additional YAML indicators
    const hasYamlIndicators =
      trimmed.includes(': |') ||
      trimmed.includes(': >') ||
      /^[\w-]+:\s*$/m.test(trimmed) ||
      /^\s+-\s+/m.test(trimmed) ||
      /^---\s*$/m.test(trimmed);

    if (hasYamlIndicators) {
      return { format: 'yaml', confidence: 0.85, hasFrontmatter: false };
    }

    // Basic key: value without JSON indicators
    if (!trimmed.includes('{') && !trimmed.includes('[')) {
      return { format: 'yaml', confidence: 0.7, hasFrontmatter: false };
    }
  }

  // Check for CSV/TSV
  const lines = trimmed.split('\n');
  if (lines.length >= 2) {
    const firstLine = lines[0];
    const secondLine = lines[1];

    // Count delimiters
    const commaCount = (firstLine.match(/,/g) || []).length;
    const tabCount = (firstLine.match(/\t/g) || []).length;

    // CSV: multiple commas, consistent across lines
    if (commaCount >= 2 && !firstLine.includes('{')) {
      const secondLineCommas = (secondLine.match(/,/g) || []).length;
      if (Math.abs(commaCount - secondLineCommas) <= 1) {
        return { format: 'csv', confidence: 0.85, hasFrontmatter: false };
      }
    }

    // TSV: multiple tabs
    if (tabCount >= 1) {
      const secondLineTabs = (secondLine.match(/\t/g) || []).length;
      if (tabCount === secondLineTabs) {
        return { format: 'tsv', confidence: 0.85, hasFrontmatter: false };
      }
    }
  }

  // Check for LaTeX
  if (trimmed.includes('\\documentclass') ||
      trimmed.includes('\\begin{document}') ||
      /\\(?:section|chapter|subsection)\{/.test(trimmed)) {
    return { format: 'latex', confidence: 0.95, hasFrontmatter: false };
  }

  // Check for Markdown indicators
  const markdownIndicators = [
    /^#{1,6}\s+/m,           // Headers
    /\*\*[^*]+\*\*/,         // Bold
    /\*[^*]+\*/,             // Italic
    /\[[^\]]+\]\([^)]+\)/,   // Links
    /```[\s\S]*?```/,        // Code blocks
    /^\s*[-*+]\s+/m,         // Unordered lists
    /^\s*\d+\.\s+/m,         // Ordered lists
    /^>\s+/m,                // Blockquotes
    /!\[[^\]]*\]\([^)]+\)/,  // Images
  ];

  const markdownMatches = markdownIndicators.filter(pattern => pattern.test(trimmed)).length;
  if (markdownMatches >= 2) {
    return { format: 'markdown', confidence: 0.8, hasFrontmatter: false };
  } else if (markdownMatches === 1) {
    return { format: 'markdown', confidence: 0.6, hasFrontmatter: false };
  }

  // Default to plaintext
  return { format: 'plaintext', confidence: 0.3, hasFrontmatter: false };
}

/**
 * Comprehensive format detection combining all methods
 */
export function detectFormat(
  content: string,
  options: {
    filename?: string;
    mimeType?: string;
    preferContentDetection?: boolean;
  } = {}
): FormatInfo {
  const { filename, mimeType, preferContentDetection = false } = options;

  // Content-based detection
  const contentResult = detectFromContent(content);

  // If preferring content detection and we have high confidence, use it
  if (preferContentDetection && contentResult.confidence >= 0.8) {
    return contentResult;
  }

  // Extension-based detection (most reliable when available)
  if (filename) {
    const extFormat = detectFromExtension(filename);
    if (extFormat) {
      // Combine with content detection for frontmatter info
      return {
        format: extFormat,
        confidence: 0.95,
        hasFrontmatter: contentResult.hasFrontmatter,
        frontmatterFormat: contentResult.frontmatterFormat,
      };
    }
  }

  // MIME type detection
  if (mimeType) {
    const mimeFormat = detectFromMimeType(mimeType);
    if (mimeFormat) {
      return {
        format: mimeFormat,
        confidence: 0.9,
        hasFrontmatter: contentResult.hasFrontmatter,
        frontmatterFormat: contentResult.frontmatterFormat,
      };
    }
  }

  // Fall back to content detection
  return contentResult;
}

// ═══════════════════════════════════════════════════════════════════════════
// UTILITY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Get file extension for a format
 */
export function getExtensionForFormat(format: FileFormat): string {
  const map: Record<FileFormat, string> = {
    markdown: '.md',
    yaml: '.yaml',
    toml: '.toml',
    json: '.json',
    xml: '.xml',
    csv: '.csv',
    tsv: '.tsv',
    mermaid: '.mmd',
    latex: '.tex',
    html: '.html',
    plaintext: '.txt',
  };
  return map[format];
}

/**
 * Get MIME type for a format
 */
export function getMimeTypeForFormat(format: FileFormat): string {
  const map: Record<FileFormat, string> = {
    markdown: 'text/markdown',
    yaml: 'text/yaml',
    toml: 'application/toml',
    json: 'application/json',
    xml: 'application/xml',
    csv: 'text/csv',
    tsv: 'text/tab-separated-values',
    mermaid: 'text/plain',
    latex: 'application/x-latex',
    html: 'text/html',
    plaintext: 'text/plain',
  };
  return map[format];
}

/**
 * Get display name for a format
 */
export function getFormatDisplayName(format: FileFormat): string {
  const map: Record<FileFormat, string> = {
    markdown: 'Markdown',
    yaml: 'YAML',
    toml: 'TOML',
    json: 'JSON',
    xml: 'XML',
    csv: 'CSV',
    tsv: 'TSV',
    mermaid: 'Mermaid',
    latex: 'LaTeX',
    html: 'HTML',
    plaintext: 'Plain Text',
  };
  return map[format];
}

/**
 * Get icon name for a format (for use with icon components)
 */
export function getFormatIcon(format: FileFormat): string {
  const map: Record<FileFormat, string> = {
    markdown: 'file-text',
    yaml: 'file-code',
    toml: 'file-cog',
    json: 'braces',
    xml: 'file-code-2',
    csv: 'table',
    tsv: 'table',
    mermaid: 'git-branch',
    latex: 'sigma',
    html: 'globe',
    plaintext: 'file',
  };
  return map[format];
}

/**
 * Check if format supports syntax highlighting
 */
export function supportsSyntaxHighlighting(format: FileFormat): boolean {
  return !['plaintext', 'csv', 'tsv'].includes(format);
}

/**
 * Check if format supports tree/structured view
 */
export function supportsTreeView(format: FileFormat): boolean {
  return ['yaml', 'toml', 'json', 'xml'].includes(format);
}

/**
 * Check if format supports table view
 */
export function supportsTableView(format: FileFormat): boolean {
  return ['csv', 'tsv'].includes(format);
}

/**
 * Check if format supports diagram rendering
 */
export function supportsDiagramView(format: FileFormat): boolean {
  return format === 'mermaid';
}

/**
 * Check if format supports rendered preview
 */
export function supportsPreview(format: FileFormat): boolean {
  return ['markdown', 'html', 'latex', 'mermaid'].includes(format);
}

/**
 * Get CodeMirror language mode name
 */
export function getCodeMirrorMode(format: FileFormat): string | null {
  const map: Record<FileFormat, string | null> = {
    markdown: 'markdown',
    yaml: 'yaml',
    toml: 'toml',
    json: 'json',
    xml: 'xml',
    csv: null,
    tsv: null,
    mermaid: null,
    latex: 'stex',
    html: 'html',
    plaintext: null,
  };
  return map[format];
}
