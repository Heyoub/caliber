// ═══════════════════════════════════════════════════════════════════════════
// CALIBER UI - Organisms
// Complex, self-contained sections with business logic
// ═══════════════════════════════════════════════════════════════════════════

// Glassmorphic card with glass, glow, border effects
// Features: Header/footer slots, mouse-tracking spotlight
export { default as Card } from './Card.svelte';

// Full chat interface with messages, input, tools
// Features: Message list, input area, tool call display, drag-drop upload
export { default as ChatPanel } from './ChatPanel.svelte';

// Single chat message bubble
// Features: User vs assistant styling, markdown rendering, streaming indicator
export { default as ChatMessage } from './ChatMessage.svelte';

// MCP tool execution display
// Features: Status badge, arguments/result display, approve/reject buttons
export { default as ToolCallCard } from './ToolCallCard.svelte';

// Code editor container with tabs
// Features: Tab bar, CodeMirror slot, status bar, edit/preview toggle
export { default as EditorPanel } from './EditorPanel.svelte';

// Smart format viewer for multiple formats
// Features: Auto-detection, markdown/tree/table/diagram rendering
export { default as FormatViewer } from './FormatViewer.svelte';

// Collapsible tree for structured data
// Features: Expand/collapse, syntax colored values, keyboard navigation
export { default as TreeView } from './TreeView.svelte';

// Before/after comparison view
// Features: Side-by-side or unified mode, line highlighting
export { default as DiffView } from './DiffView.svelte';

// Memory structure browser (Trajectory/Scope/Turn)
// Features: Hierarchical tree, file type icons, selection state
export { default as FileTree } from './FileTree.svelte';

// Interactive memory graph visualization
// Features: Parallax scrolling, hover tooltips, connection lines with glow
export { default as MemoryGraph } from './MemoryGraph.svelte';

// Keyboard command interface
// Features: Fuzzy search, keyboard navigation, MCP prompts support
export { default as CommandPalette } from './CommandPalette.svelte';

// Navigation sidebar
// Features: Collapsible sections, footer, laser effect on hover
export { default as Sidebar } from './Sidebar.svelte';

// Canvas-based neural network particle animation
// Features: Floating particles, connection lines, mouse interaction
export { default as NeuralAnimation } from './NeuralAnimation.svelte';

// Multi-layer parallax scrolling hero section
// Features: Scroll-responsive layers, mouse parallax, gradient overlays
export { default as ParallaxHero } from './ParallaxHero.svelte';

// Retro/cyberpunk animated grid background
// Features: Perspective lines, horizon glow, pulse animation
export { default as GridAnimation } from './GridAnimation.svelte';

// Three.js 3D rotating icosahedron
// Features: Vertex coloring, wireframe mode, mouse interaction
export { default as IcosahedronAnimation } from './IcosahedronAnimation.svelte';

// Hexagonal particle network animation
// Features: Node connections, glow effects, edge bouncing
export { default as HexScroll } from './HexScroll.svelte';

// ═══════════════════════════════════════════════════════════════════════════
// Type Re-exports (for convenience)
// ═══════════════════════════════════════════════════════════════════════════

export type {
  // Core types
  ColorPalette,
  ColorToken,
  Size,
  GlowEffect,
  GlassEffect,
  BorderEffect,
  HoverEffect,
  StyledProps,
  AspectFlags,

  // MCP types
  ToolCall,
  ToolCallStatus,
  ToolResult,
  Resource,
  Prompt,
  PromptArgument,

  // Chat types
  ChatMessage,
  MessageRole,

  // Editor types
  EditorTab,
  EditorPosition,
  FileFormat,

  // Tree types
  TreeNode,
  TreeNodeValue,

  // Diff types
  DiffLine,
  DiffLineType,
  DiffViewMode,

  // Graph types
  GraphNode,

  // Command types
  Command,

  // CMS types
  CMSContent
} from '../types/index.js';
