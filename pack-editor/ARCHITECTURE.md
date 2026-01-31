# CALIBER Frontend Architecture

> Unified design system for landing (Astro+Svelte) and app (SvelteKit)
> "Tailwind meets Cosmic Atomic" - expressive, typed, aspect-oriented

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Atomic Design Hierarchy](#2-atomic-design-hierarchy)
3. [Typed Modifier System](#3-typed-modifier-system)
4. [Design Tokens](#4-design-tokens)
5. [Component Catalog](#5-component-catalog)
6. [Editor Specification](#6-editor-specification)
7. [MCP UI Patterns](#7-mcp-ui-patterns)
8. [File Structure](#8-file-structure)

---

## 1. System Overview

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SHARED DESIGN SYSTEM                               │
│                         @caliber/ui (npm package)                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Tokens → Atoms → Molecules → Organisms → Templates                      ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└──────────────────────────────┬──────────────────────────────────────────────┘
                               │
           ┌───────────────────┴───────────────────┐
           ▼                                       ▼
┌─────────────────────────┐             ┌─────────────────────────┐
│  landing/               │             │  app/                   │
│  Astro + Svelte Islands │             │  SvelteKit              │
│  ────────────────────── │             │  ────────────────────── │
│  • Marketing pages      │             │  • Dashboard (thin)     │
│  • Pricing              │             │  • Pack Editor          │
│  • Docs                 │             │  • AI Agent Interface   │
│  • Auth entry           │             │  • Memory Graph         │
└─────────────────────────┘             └─────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           RUST API (caliber-api)                             │
│  • Model provider abstraction    • MCP server                                │
│  • EventDag memory operations    • Auth/billing                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Principles

1. **Thin Frontend**: UI is dumb, Rust API is smart
2. **Atomic Composition**: Build up from primitives
3. **Type Everything**: Props, modifiers, variants - all typed
4. **Expressive Modifiers**: Semantic, not just utility classes
5. **Aspect-Oriented**: Cross-cutting concerns as flags (loading, disabled, error)

---

## 2. Atomic Design Hierarchy

### Level 0: Design Tokens
CSS custom properties, the DNA of the system.

### Level 1: Atoms
Indivisible UI primitives. Cannot be broken down further.

```
Button | Input | Icon | Badge | Avatar | Spinner | Divider | Tooltip
```

### Level 2: Molecules
Combinations of atoms that form functional units.

```
SearchInput = Input + Icon + Button
ToolCallCard = Badge + Icon + Text + Spinner
FileTreeItem = Icon + Text + Badge
```

### Level 3: Organisms
Complex, self-contained sections with business logic.

```
ChatPanel = Messages[] + ChatInput + ToolCallCard[]
EditorPanel = FileTree + CodeEditor + FormatViewer
MemoryGraph = ScopeCard[] + Connections + Legend
```

### Level 4: Templates
Page-level layouts without content.

```
EditorLayout = Sidebar + MainPanel + RightPanel
DashboardLayout = Header + StatsGrid + ContentArea
```

### Level 5: Pages
Templates filled with real content/data.

```
/editor/assistant → EditorLayout + AssistantMode
/editor/playground → EditorLayout + PlaygroundMode
```

---

## 3. Typed Modifier System

### Philosophy

Instead of:
```html
<button class="bg-teal-500 hover:bg-teal-600 text-white px-4 py-2 rounded-lg shadow-lg">
```

We write:
```svelte
<Button color="teal" size="md" glow hover="lift" />
```

All modifiers are **typed**, **composable**, and **semantic**.

### Core Types

```typescript
// ═══════════════════════════════════════════════════════════════════════════
// COLOR SYSTEM
// ═══════════════════════════════════════════════════════════════════════════

type ColorPalette =
  | 'teal'      // Primary - memory, scopes
  | 'coral'     // Accent - warnings, highlights
  | 'purple'    // Secondary - AI, intelligence
  | 'pink'      // Accent - ephemeral, turns
  | 'mint'      // Success - confirmations
  | 'amber'     // Rust accent - semantic, notes
  | 'slate'     // Neutral - backgrounds, borders
  | 'ghost';    // Transparent with subtle hover

type ColorIntensity = 50 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900;

type ColorToken = `${ColorPalette}-${ColorIntensity}` | ColorPalette;

// ═══════════════════════════════════════════════════════════════════════════
// SIZE SYSTEM
// ═══════════════════════════════════════════════════════════════════════════

type Size = 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl';

type Spacing = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 8 | 10 | 12 | 16 | 20 | 24;

// ═══════════════════════════════════════════════════════════════════════════
// EFFECT MODIFIERS
// ═══════════════════════════════════════════════════════════════════════════

type GlowEffect =
  | boolean           // Default glow
  | 'subtle'          // 0.1 opacity
  | 'medium'          // 0.25 opacity
  | 'intense'         // 0.5 opacity
  | 'pulse';          // Animated pulse

type GlassEffect =
  | boolean           // Default glass
  | 'subtle'          // blur-sm, 5% opacity
  | 'medium'          // blur-md, 10% opacity
  | 'frosted'         // blur-xl, 15% opacity
  | 'solid';          // blur-2xl, 20% opacity

type BorderEffect =
  | boolean           // Default 1px
  | 'none'
  | 'subtle'          // 1px, 10% opacity
  | 'medium'          // 1px, 20% opacity
  | 'strong'          // 2px, 30% opacity
  | 'glow';           // 1px + box-shadow

// ═══════════════════════════════════════════════════════════════════════════
// INTERACTION MODIFIERS
// ═══════════════════════════════════════════════════════════════════════════

type HoverEffect =
  | 'none'
  | 'lift'            // translateY(-2px)
  | 'glow'            // Increase glow
  | 'scale'           // scale(1.02)
  | 'brighten'        // Increase brightness
  | 'border';         // Border color change

type PressEffect =
  | 'none'
  | 'sink'            // translateY(1px)
  | 'scale'           // scale(0.98)
  | 'darken';         // Decrease brightness

type FocusEffect =
  | 'ring'            // Focus ring
  | 'glow'            // Focus glow
  | 'border';         // Border highlight

// ═══════════════════════════════════════════════════════════════════════════
// ASPECT-ORIENTED FLAGS (Cross-cutting concerns)
// ═══════════════════════════════════════════════════════════════════════════

type AspectFlags = {
  // State aspects
  loading?: boolean;
  disabled?: boolean;
  error?: boolean;
  success?: boolean;
  selected?: boolean;
  active?: boolean;

  // Visibility aspects
  hidden?: boolean;
  collapsed?: boolean;
  expanded?: boolean;

  // Interaction aspects
  interactive?: boolean;
  draggable?: boolean;
  droppable?: boolean;
  resizable?: boolean;

  // Content aspects
  truncate?: boolean;
  wrap?: boolean;
  scrollable?: boolean;

  // Layout aspects
  fullWidth?: boolean;
  fullHeight?: boolean;
  centered?: boolean;

  // Animation aspects
  animate?: boolean;
  animateIn?: 'fade' | 'slide' | 'scale' | 'spring';
  animateOut?: 'fade' | 'slide' | 'scale';
};

// ═══════════════════════════════════════════════════════════════════════════
// LAYOUT MODIFIERS
// ═══════════════════════════════════════════════════════════════════════════

type FlexAlign = 'start' | 'center' | 'end' | 'stretch' | 'baseline';
type FlexJustify = 'start' | 'center' | 'end' | 'between' | 'around' | 'evenly';
type FlexDirection = 'row' | 'col' | 'row-reverse' | 'col-reverse';

type LayoutProps = {
  flex?: boolean | FlexDirection;
  align?: FlexAlign;
  justify?: FlexJustify;
  gap?: Spacing;
  wrap?: boolean;
  grid?: boolean | number; // number = columns
};

// ═══════════════════════════════════════════════════════════════════════════
// TYPOGRAPHY MODIFIERS
// ═══════════════════════════════════════════════════════════════════════════

type FontFamily = 'sans' | 'mono' | 'display';
type FontWeight = 'normal' | 'medium' | 'semibold' | 'bold';
type FontSize = 'xs' | 'sm' | 'base' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl';
type TextAlign = 'left' | 'center' | 'right';
type TextTransform = 'none' | 'uppercase' | 'lowercase' | 'capitalize';

type TypographyProps = {
  font?: FontFamily;
  weight?: FontWeight;
  size?: FontSize;
  align?: TextAlign;
  transform?: TextTransform;
  muted?: boolean;
  gradient?: boolean;
};

// ═══════════════════════════════════════════════════════════════════════════
// COMPOSITE COMPONENT PROPS
// ═══════════════════════════════════════════════════════════════════════════

type BaseProps = AspectFlags & {
  class?: string;      // Escape hatch for custom classes
  style?: string;      // Escape hatch for custom styles
};

type InteractiveProps = BaseProps & {
  hover?: HoverEffect;
  press?: PressEffect;
  focus?: FocusEffect;
};

type StyledProps = InteractiveProps & {
  color?: ColorToken;
  size?: Size;
  glow?: GlowEffect;
  glass?: GlassEffect;
  border?: BorderEffect;
  rounded?: Size | 'full' | 'none';
  shadow?: Size | 'none';
};
```

### Usage Examples

```svelte
<!-- Atom: Button -->
<Button
  color="teal"
  size="lg"
  glow="pulse"
  hover="lift"
  press="sink"
  loading={isSubmitting}
>
  Save Changes
</Button>

<!-- Molecule: SearchInput -->
<SearchInput
  size="md"
  glass="frosted"
  border="glow"
  placeholder="Search memory..."
  loading={isSearching}
/>

<!-- Organism: ToolCallCard -->
<ToolCallCard
  tool={toolCall}
  color="purple"
  glass="medium"
  border="subtle"
  animateIn="spring"
  expanded={showDetails}
/>
```

### Generated CSS Classes

The modifier system generates semantic CSS:

```css
/* Color modifiers */
.color-teal { --component-color: var(--teal-500); }
.color-teal-300 { --component-color: var(--teal-300); }

/* Glow modifiers */
.glow { box-shadow: 0 0 20px hsl(var(--component-color) / 0.25); }
.glow-subtle { box-shadow: 0 0 10px hsl(var(--component-color) / 0.1); }
.glow-pulse { animation: glow-pulse 2s ease-in-out infinite; }

/* Glass modifiers */
.glass { backdrop-filter: blur(12px); background: hsl(var(--slate-900) / 0.8); }
.glass-frosted { backdrop-filter: blur(20px); background: hsl(var(--slate-900) / 0.85); }

/* Hover modifiers */
.hover-lift:hover { transform: translateY(-2px); }
.hover-glow:hover { --glow-opacity: 0.4; }
.hover-scale:hover { transform: scale(1.02); }

/* Aspect flags */
.loading { pointer-events: none; opacity: 0.7; }
.disabled { pointer-events: none; opacity: 0.5; filter: grayscale(0.3); }
.error { --component-color: var(--coral-500); }
```

---

## 4. Design Tokens

### CSS Custom Properties

```css
:root {
  /* ═══════════════════════════════════════════════════════════════════════
     COLOR PALETTE
     ═══════════════════════════════════════════════════════════════════════ */

  /* Teal - Primary (Memory, Scopes) */
  --teal-50:  176 80% 95%;
  --teal-100: 176 75% 90%;
  --teal-200: 176 70% 80%;
  --teal-300: 176 65% 70%;
  --teal-400: 176 60% 55%;
  --teal-500: 176 55% 45%;  /* Base */
  --teal-600: 176 55% 38%;
  --teal-700: 176 55% 30%;
  --teal-800: 176 55% 22%;
  --teal-900: 176 55% 15%;

  /* Coral - Accent (Warnings, Highlights) */
  --coral-50:  12 90% 95%;
  --coral-100: 12 85% 90%;
  --coral-200: 12 80% 80%;
  --coral-300: 12 75% 70%;
  --coral-400: 12 70% 60%;
  --coral-500: 12 65% 50%;  /* Base */
  --coral-600: 12 65% 42%;
  --coral-700: 12 65% 34%;
  --coral-800: 12 65% 26%;
  --coral-900: 12 65% 18%;

  /* Purple - Secondary (AI, Intelligence) */
  --purple-50:  270 80% 96%;
  --purple-100: 270 75% 92%;
  --purple-200: 270 70% 82%;
  --purple-300: 270 65% 72%;
  --purple-400: 270 60% 62%;
  --purple-500: 270 55% 52%;  /* Base */
  --purple-600: 270 55% 44%;
  --purple-700: 270 55% 36%;
  --purple-800: 270 55% 28%;
  --purple-900: 270 55% 20%;

  /* Pink - Accent (Ephemeral, Turns) */
  --pink-50:  330 85% 96%;
  --pink-100: 330 80% 92%;
  --pink-200: 330 75% 82%;
  --pink-300: 330 70% 72%;
  --pink-400: 330 65% 62%;
  --pink-500: 330 60% 52%;  /* Base */
  --pink-600: 330 60% 44%;
  --pink-700: 330 60% 36%;
  --pink-800: 330 60% 28%;
  --pink-900: 330 60% 20%;

  /* Mint - Success */
  --mint-50:  160 70% 95%;
  --mint-100: 160 65% 90%;
  --mint-200: 160 60% 80%;
  --mint-300: 160 55% 70%;
  --mint-400: 160 50% 55%;
  --mint-500: 160 45% 45%;  /* Base */
  --mint-600: 160 45% 38%;
  --mint-700: 160 45% 30%;
  --mint-800: 160 45% 22%;
  --mint-900: 160 45% 15%;

  /* Amber - Rust Accent (Semantic, Notes) */
  --amber-50:  38 95% 95%;
  --amber-100: 38 90% 88%;
  --amber-200: 38 85% 75%;
  --amber-300: 38 80% 62%;
  --amber-400: 38 75% 50%;
  --amber-500: 38 70% 42%;  /* Base */
  --amber-600: 38 70% 35%;
  --amber-700: 38 70% 28%;
  --amber-800: 38 70% 20%;
  --amber-900: 38 70% 14%;

  /* Slate - Neutral */
  --slate-50:  220 15% 96%;
  --slate-100: 220 14% 90%;
  --slate-200: 220 13% 80%;
  --slate-300: 220 12% 65%;
  --slate-400: 220 11% 50%;
  --slate-500: 220 10% 40%;
  --slate-600: 220 10% 30%;
  --slate-700: 220 12% 20%;
  --slate-800: 220 14% 12%;
  --slate-900: 220 16% 8%;
  --slate-950: 220 18% 5%;

  /* ═══════════════════════════════════════════════════════════════════════
     SEMANTIC TOKENS
     ═══════════════════════════════════════════════════════════════════════ */

  --bg-primary:    var(--slate-950);
  --bg-secondary:  var(--slate-900);
  --bg-card:       var(--slate-800);
  --bg-elevated:   var(--slate-700);

  --text-primary:   var(--slate-50);
  --text-secondary: var(--slate-300);
  --text-muted:     var(--slate-400);
  --text-disabled:  var(--slate-500);

  --border-subtle:  var(--slate-700);
  --border-medium:  var(--slate-600);
  --border-strong:  var(--slate-500);

  /* ═══════════════════════════════════════════════════════════════════════
     SPACING SCALE
     ═══════════════════════════════════════════════════════════════════════ */

  --space-0:  0;
  --space-1:  0.25rem;   /* 4px */
  --space-2:  0.5rem;    /* 8px */
  --space-3:  0.75rem;   /* 12px */
  --space-4:  1rem;      /* 16px */
  --space-5:  1.25rem;   /* 20px */
  --space-6:  1.5rem;    /* 24px */
  --space-8:  2rem;      /* 32px */
  --space-10: 2.5rem;    /* 40px */
  --space-12: 3rem;      /* 48px */
  --space-16: 4rem;      /* 64px */
  --space-20: 5rem;      /* 80px */
  --space-24: 6rem;      /* 96px */

  /* ═══════════════════════════════════════════════════════════════════════
     TYPOGRAPHY
     ═══════════════════════════════════════════════════════════════════════ */

  --font-sans:    'Inter', system-ui, sans-serif;
  --font-display: 'Space Grotesk', var(--font-sans);
  --font-mono:    'JetBrains Mono', 'Fira Code', monospace;

  --text-xs:   0.75rem;    /* 12px */
  --text-sm:   0.875rem;   /* 14px */
  --text-base: 1rem;       /* 16px */
  --text-lg:   1.125rem;   /* 18px */
  --text-xl:   1.25rem;    /* 20px */
  --text-2xl:  1.5rem;     /* 24px */
  --text-3xl:  1.875rem;   /* 30px */
  --text-4xl:  2.25rem;    /* 36px */

  /* ═══════════════════════════════════════════════════════════════════════
     EFFECTS
     ═══════════════════════════════════════════════════════════════════════ */

  --blur-sm:  4px;
  --blur-md:  8px;
  --blur-lg:  12px;
  --blur-xl:  20px;
  --blur-2xl: 40px;

  --shadow-sm:  0 1px 2px hsl(var(--slate-950) / 0.5);
  --shadow-md:  0 4px 6px hsl(var(--slate-950) / 0.4);
  --shadow-lg:  0 10px 15px hsl(var(--slate-950) / 0.3);
  --shadow-xl:  0 20px 25px hsl(var(--slate-950) / 0.25);
  --shadow-glow: 0 0 20px hsl(var(--teal-500) / 0.3);

  --radius-sm:   0.25rem;  /* 4px */
  --radius-md:   0.5rem;   /* 8px */
  --radius-lg:   0.75rem;  /* 12px */
  --radius-xl:   1rem;     /* 16px */
  --radius-2xl:  1.5rem;   /* 24px */
  --radius-full: 9999px;

  /* ═══════════════════════════════════════════════════════════════════════
     ANIMATION
     ═══════════════════════════════════════════════════════════════════════ */

  --duration-fast:   150ms;
  --duration-normal: 300ms;
  --duration-slow:   500ms;

  --ease-default: cubic-bezier(0.4, 0, 0.2, 1);
  --ease-in:      cubic-bezier(0.4, 0, 1, 1);
  --ease-out:     cubic-bezier(0, 0, 0.2, 1);
  --ease-spring:  cubic-bezier(0.34, 1.56, 0.64, 1);
}
```

---

## 5. Component Catalog

### Atoms

| Component | Props | Description |
|-----------|-------|-------------|
| `Button` | `color, size, glow, hover, press, loading, disabled` | Primary action trigger |
| `IconButton` | `icon, color, size, glow, tooltip` | Icon-only button |
| `Input` | `type, size, glass, border, error, prefix, suffix` | Text input |
| `TextArea` | `size, glass, border, rows, autoResize` | Multiline input |
| `Select` | `options, size, glass, searchable` | Dropdown select |
| `Checkbox` | `checked, size, color, indeterminate` | Boolean toggle |
| `Toggle` | `checked, size, color, labels` | Switch toggle |
| `Badge` | `color, size, glow, dot, removable` | Status indicator |
| `Avatar` | `src, name, size, status` | User/entity avatar |
| `Icon` | `name, size, color, spin` | SVG icon |
| `Spinner` | `size, color` | Loading indicator |
| `Divider` | `orientation, color, spacing` | Visual separator |
| `Tooltip` | `content, position, delay` | Hover tooltip |
| `Kbd` | `keys` | Keyboard shortcut display |

### Molecules

| Component | Composition | Description |
|-----------|-------------|-------------|
| `SearchInput` | Input + Icon + Button | Searchable input |
| `InputGroup` | Input + prefix/suffix slots | Grouped input |
| `ButtonGroup` | Button[] | Grouped actions |
| `Tabs` | Tab[] + TabPanel | Tabbed content |
| `Dropdown` | Button + Menu | Action menu |
| `Modal` | Overlay + Card | Dialog |
| `Toast` | Icon + Text + Actions | Notification |
| `Breadcrumb` | Link[] + Separator | Navigation path |
| `Pagination` | Button[] + Text | Page navigation |
| `FileTreeItem` | Icon + Text + Badge + Actions | Single tree node |
| `ToolCallHeader` | Badge + Icon + Text + Status | Tool execution header |
| `DiffLine` | LineNumber + Content + Highlight | Single diff line |

### Organisms

| Component | Description |
|-----------|-------------|
| `ChatPanel` | Full chat interface with messages, input, tools |
| `EditorPanel` | Code editor with tabs, format viewers |
| `FileTree` | Hierarchical file/memory browser |
| `MemoryGraph` | Interactive trajectory/scope visualization |
| `ToolCallCard` | Complete tool execution display |
| `DiffView` | Side-by-side or unified diff |
| `CommandPalette` | Keyboard-driven command interface |
| `SettingsPanel` | Configuration interface |

---

## 6. Editor Specification

### Supported Formats

| Format | Extension | Edit Mode | View Mode | Syntax Highlighting |
|--------|-----------|-----------|-----------|---------------------|
| Markdown | `.md` | CodeMirror | Rendered HTML + Mermaid + KaTeX | ✓ markdown |
| YAML | `.yaml`, `.yml` | CodeMirror | Collapsible tree | ✓ yaml |
| TOML | `.toml` | CodeMirror | Collapsible tree | ✓ toml |
| JSON | `.json` | CodeMirror | Collapsible tree | ✓ json |
| XML | `.xml` | CodeMirror | Collapsible tree | ✓ xml |
| CSV | `.csv` | CodeMirror | Table view | ✓ csv (custom) |
| Mermaid | (fenced block) | In markdown | Diagram render | ✓ mermaid |
| LaTeX | `$...$`, `$$...$$` | In markdown | KaTeX render | ✓ latex |

### CodeMirror 6 Configuration

```typescript
import { EditorState } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';

// Language support
import { markdown } from '@codemirror/lang-markdown';
import { yaml } from '@codemirror/lang-yaml';
import { json } from '@codemirror/lang-json';
import { xml } from '@codemirror/lang-xml';

// Custom extensions for TOML and CSV
import { toml } from '@codemirror-toml';  // or custom
import { csv } from './csv-mode';          // custom

const languageMap: Record<string, LanguageSupport> = {
  'md': markdown(),
  'yaml': yaml(),
  'yml': yaml(),
  'toml': toml(),
  'json': json(),
  'xml': xml(),
  'csv': csv(),
};

function createEditor(container: HTMLElement, options: EditorOptions) {
  const language = languageMap[options.fileType] || markdown();

  return new EditorView({
    state: EditorState.create({
      doc: options.content,
      extensions: [
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        language,
        syntaxHighlighting(calibrHighlightStyle),
        calibrTheme,
        EditorView.lineWrapping,
        // Custom extensions
        readOnlyMode(options.readOnly),
        saveOnChange(options.onSave),
      ],
    }),
    parent: container,
  });
}
```

### Custom Syntax Theme

```typescript
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';

const calibrHighlightStyle = HighlightStyle.define([
  // Keywords
  { tag: tags.keyword, color: 'hsl(var(--purple-400))' },
  { tag: tags.controlKeyword, color: 'hsl(var(--purple-400))' },

  // Strings
  { tag: tags.string, color: 'hsl(var(--mint-400))' },
  { tag: tags.special(tags.string), color: 'hsl(var(--mint-300))' },

  // Numbers
  { tag: tags.number, color: 'hsl(var(--coral-400))' },
  { tag: tags.bool, color: 'hsl(var(--coral-400))' },

  // Comments
  { tag: tags.comment, color: 'hsl(var(--slate-500))', fontStyle: 'italic' },
  { tag: tags.lineComment, color: 'hsl(var(--slate-500))' },

  // Properties/Keys (YAML, TOML, JSON)
  { tag: tags.propertyName, color: 'hsl(var(--teal-400))' },
  { tag: tags.definition(tags.propertyName), color: 'hsl(var(--teal-300))' },

  // Headings (Markdown)
  { tag: tags.heading1, color: 'hsl(var(--purple-300))', fontWeight: 'bold', fontSize: '1.4em' },
  { tag: tags.heading2, color: 'hsl(var(--purple-400))', fontWeight: 'bold', fontSize: '1.2em' },
  { tag: tags.heading3, color: 'hsl(var(--purple-500))', fontWeight: 'bold' },

  // Links
  { tag: tags.link, color: 'hsl(var(--teal-400))', textDecoration: 'underline' },
  { tag: tags.url, color: 'hsl(var(--teal-500))' },

  // Code
  { tag: tags.monospace, fontFamily: 'var(--font-mono)' },

  // XML/HTML tags
  { tag: tags.tagName, color: 'hsl(var(--pink-400))' },
  { tag: tags.attributeName, color: 'hsl(var(--amber-400))' },
  { tag: tags.attributeValue, color: 'hsl(var(--mint-400))' },

  // Punctuation
  { tag: tags.punctuation, color: 'hsl(var(--slate-400))' },
  { tag: tags.bracket, color: 'hsl(var(--slate-400))' },
]);
```

### Format Viewers

```svelte
<!-- MarkdownView.svelte -->
<script lang="ts">
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import MermaidBlock from './MermaidBlock.svelte';
  import KatexBlock from './KatexBlock.svelte';

  export let content: string;

  // Custom renderer for mermaid and latex
  const renderer = new marked.Renderer();

  renderer.code = (code, language) => {
    if (language === 'mermaid') {
      return `<mermaid-block data-code="${encodeURIComponent(code)}"></mermaid-block>`;
    }
    return `<pre><code class="language-${language}">${code}</code></pre>`;
  };

  $: html = DOMPurify.sanitize(marked(content, { renderer }));
</script>

<div class="markdown-view prose prose-invert">
  {@html html}
</div>

<!-- TreeView.svelte (for YAML, TOML, JSON, XML) -->
<script lang="ts">
  import { parse as parseYaml } from 'yaml';
  import { parse as parseToml } from 'smol-toml';
  import { XMLParser } from 'fast-xml-parser';

  export let content: string;
  export let format: 'yaml' | 'toml' | 'json' | 'xml';

  function parseContent(content: string, format: string) {
    switch (format) {
      case 'yaml': return parseYaml(content);
      case 'toml': return parseToml(content);
      case 'json': return JSON.parse(content);
      case 'xml': return new XMLParser().parse(content);
    }
  }

  $: data = parseContent(content, format);
</script>

<TreeNode {data} depth={0} />

<!-- CsvView.svelte -->
<script lang="ts">
  import Papa from 'papaparse';

  export let content: string;

  $: parsed = Papa.parse(content, { header: true });
  $: headers = parsed.meta.fields || [];
  $: rows = parsed.data;
</script>

<div class="csv-view overflow-auto">
  <table class="w-full border-collapse">
    <thead>
      <tr class="bg-slate-800">
        {#each headers as header}
          <th class="px-3 py-2 text-left text-xs font-medium text-teal-400 uppercase">
            {header}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each rows as row, i}
        <tr class="border-t border-slate-700 hover:bg-slate-800/50">
          {#each headers as header}
            <td class="px-3 py-2 text-sm text-slate-300">
              {row[header]}
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
```

### Editor Panel Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [scope-main.yaml] [notes.md] [config.toml] [+]            [Edit] [Preview] │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────┬────────────────────────────────────────┐│
│  │         Edit Mode              │           Preview Mode                 ││
│  │  ┌──────────────────────────┐  │  ┌──────────────────────────────────┐ ││
│  │  │ CodeMirror 6             │  │  │ Format-specific viewer           │ ││
│  │  │                          │  │  │                                  │ ││
│  │  │ name: main               │  │  │ ┌─ name: main                    │ ││
│  │  │ memory_limit: 2000       │  │  │ │  ├─ memory_limit: 2000         │ ││
│  │  │ tags:                    │  │  │ │  └─ tags: [...]                │ ││
│  │  │   - context              │  │  │                                  │ ││
│  │  │   - production           │  │  │                                  │ ││
│  │  │                          │  │  │                                  │ ││
│  │  └──────────────────────────┘  │  └──────────────────────────────────┘ ││
│  └────────────────────────────────┴────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────────────┤
│  Ln 4, Col 12 | YAML | UTF-8 | 2 spaces                          [Format]  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. MCP UI Patterns

### Tool Call Display

```svelte
<!-- ToolCallCard.svelte -->
<script lang="ts">
  import type { ToolCall, ToolResult } from '$lib/types/mcp';

  interface Props extends StyledProps {
    call: ToolCall;
    result?: ToolResult;
    expanded?: boolean;
    onApprove?: () => void;
    onReject?: () => void;
  }

  let { call, result, expanded = false, onApprove, onReject, ...style }: Props = $props();

  const statusColors: Record<ToolCall['status'], ColorToken> = {
    pending: 'amber',
    approved: 'teal',
    running: 'purple',
    success: 'mint',
    error: 'coral',
  };
</script>

<Card
  glass="medium"
  border="subtle"
  color={statusColors[call.status]}
  {expanded}
  {...style}
>
  <header slot="header" class="flex items-center gap-2">
    <Badge color={statusColors[call.status]} glow="subtle">
      <Icon name="tool" size="xs" />
      {call.name}
    </Badge>

    <Spinner size="xs" hidden={call.status !== 'running'} />

    <span class="text-muted text-xs ml-auto">
      {call.duration}ms
    </span>
  </header>

  <!-- Arguments -->
  <div class="font-mono text-xs">
    <TreeView data={call.arguments} format="json" />
  </div>

  <!-- Result (if available) -->
  {#if result}
    <Divider spacing={3} />
    <ToolResultView {result} />
  {/if}

  <!-- Approval buttons (if pending) -->
  {#if call.status === 'pending'}
    <footer slot="footer" class="flex gap-2 justify-end">
      <Button size="sm" color="ghost" press="sink" on:click={onReject}>
        Reject
      </Button>
      <Button size="sm" color="teal" glow hover="lift" on:click={onApprove}>
        Approve
      </Button>
    </footer>
  {/if}
</Card>
```

### Resource Browser

```svelte
<!-- ResourceBrowser.svelte -->
<script lang="ts">
  import type { Resource } from '$lib/types/mcp';

  export let resources: Resource[];
  export let onSelect: (resource: Resource) => void;

  const iconMap: Record<string, string> = {
    'application/x-yaml': 'file-yaml',
    'application/toml': 'file-toml',
    'text/markdown': 'file-markdown',
    'application/json': 'file-json',
  };
</script>

<div class="resource-browser">
  {#each resources as resource}
    <button
      class="resource-item"
      on:click={() => onSelect(resource)}
    >
      <Icon name={iconMap[resource.mimeType] || 'file'} color="teal" />
      <span class="resource-name">{resource.name}</span>
      <span class="resource-uri text-muted text-xs">{resource.uri}</span>
    </button>
  {/each}
</div>
```

### Prompt Palette

```svelte
<!-- PromptPalette.svelte -->
<script lang="ts">
  import type { Prompt } from '$lib/types/mcp';

  export let prompts: Prompt[];
  export let onSelect: (prompt: Prompt, args: Record<string, any>) => void;

  let search = $state('');
  let selected = $state<Prompt | null>(null);
  let args = $state<Record<string, string>>({});

  const filtered = $derived(
    prompts.filter(p =>
      p.name.includes(search) || p.description?.includes(search)
    )
  );
</script>

<CommandPalette open={true}>
  <SearchInput
    bind:value={search}
    placeholder="Search prompts..."
    size="lg"
    glass="frosted"
  />

  {#if !selected}
    <div class="prompt-list">
      {#each filtered as prompt}
        <button
          class="prompt-item"
          on:click={() => selected = prompt}
        >
          <Icon name="message-square" color="purple" />
          <div>
            <span class="font-medium">{prompt.name}</span>
            <span class="text-muted text-xs">{prompt.description}</span>
          </div>
          <Kbd keys={['Enter']} />
        </button>
      {/each}
    </div>
  {:else}
    <!-- Argument form -->
    <form on:submit|preventDefault={() => onSelect(selected, args)}>
      <h3 class="font-display text-lg mb-4">{selected.name}</h3>

      {#each selected.arguments as arg}
        <InputGroup label={arg.name} required={arg.required}>
          <Input
            bind:value={args[arg.name]}
            placeholder={arg.description}
          />
        </InputGroup>
      {/each}

      <div class="flex gap-2 mt-4">
        <Button color="ghost" on:click={() => selected = null}>
          Back
        </Button>
        <Button color="teal" type="submit" glow>
          Execute
        </Button>
      </div>
    </form>
  {/if}
</CommandPalette>
```

---

## 8. File Structure

```
caliber/
├── packages/
│   └── ui/                          # @caliber/ui - Shared design system
│       ├── package.json
│       ├── src/
│       │   ├── tokens/
│       │   │   ├── colors.css
│       │   │   ├── typography.css
│       │   │   ├── spacing.css
│       │   │   ├── effects.css
│       │   │   └── index.css
│       │   │
│       │   ├── types/
│       │   │   ├── modifiers.ts     # All typed modifiers
│       │   │   ├── props.ts         # Component prop types
│       │   │   └── index.ts
│       │   │
│       │   ├── atoms/
│       │   │   ├── Button.svelte
│       │   │   ├── Input.svelte
│       │   │   ├── Badge.svelte
│       │   │   ├── Icon.svelte
│       │   │   ├── Spinner.svelte
│       │   │   └── ...
│       │   │
│       │   ├── molecules/
│       │   │   ├── SearchInput.svelte
│       │   │   ├── Tabs.svelte
│       │   │   ├── Dropdown.svelte
│       │   │   ├── FileTreeItem.svelte
│       │   │   └── ...
│       │   │
│       │   ├── organisms/
│       │   │   ├── ChatPanel.svelte
│       │   │   ├── EditorPanel.svelte
│       │   │   ├── MemoryGraph.svelte
│       │   │   ├── ToolCallCard.svelte
│       │   │   └── ...
│       │   │
│       │   ├── templates/
│       │   │   ├── EditorLayout.svelte
│       │   │   ├── DashboardLayout.svelte
│       │   │   └── ...
│       │   │
│       │   ├── utils/
│       │   │   ├── cn.ts            # Class name merger
│       │   │   ├── modifiers.ts     # Modifier → class converter
│       │   │   └── ...
│       │   │
│       │   └── index.ts             # Public exports
│       │
│       └── vite.config.ts
│
├── landing/                          # Astro + Svelte Islands
│   ├── src/
│   │   ├── pages/
│   │   ├── layouts/
│   │   ├── components/
│   │   │   └── svelte/              # Uses @caliber/ui
│   │   └── styles/
│   │       └── global.css           # Imports @caliber/ui/tokens
│   └── ...
│
├── app/                              # SvelteKit
│   ├── src/
│   │   ├── routes/
│   │   │   ├── +layout.svelte
│   │   │   ├── dashboard/
│   │   │   │   └── +page.svelte
│   │   │   └── editor/
│   │   │       ├── +layout.svelte
│   │   │       ├── assistant/
│   │   │       │   └── +page.svelte
│   │   │       └── playground/
│   │   │           └── +page.svelte
│   │   │
│   │   ├── lib/
│   │   │   ├── components/          # App-specific components
│   │   │   │   ├── editor/
│   │   │   │   │   ├── CodeEditor.svelte
│   │   │   │   │   ├── FormatViewers/
│   │   │   │   │   │   ├── MarkdownView.svelte
│   │   │   │   │   │   ├── YamlView.svelte
│   │   │   │   │   │   ├── TomlView.svelte
│   │   │   │   │   │   ├── JsonView.svelte
│   │   │   │   │   │   ├── XmlView.svelte
│   │   │   │   │   │   ├── CsvView.svelte
│   │   │   │   │   │   └── MermaidView.svelte
│   │   │   │   │   └── DiffView.svelte
│   │   │   │   └── mcp/
│   │   │   │       ├── ToolCallCard.svelte
│   │   │   │       ├── ResourceBrowser.svelte
│   │   │   │       └── PromptPalette.svelte
│   │   │   │
│   │   │   ├── stores/
│   │   │   │   ├── mode.ts          # assistant | playground
│   │   │   │   ├── assistant.ts     # Real API
│   │   │   │   ├── playground.ts    # Client-side sandbox
│   │   │   │   └── editor.ts        # Editor state
│   │   │   │
│   │   │   ├── api/
│   │   │   │   └── client.ts        # API client (thin wrapper)
│   │   │   │
│   │   │   └── types/
│   │   │       ├── mcp.ts           # MCP protocol types
│   │   │       └── memory.ts        # Memory/trajectory types
│   │   │
│   │   └── app.css                  # Imports @caliber/ui/tokens
│   │
│   ├── svelte.config.js
│   └── vite.config.ts
│
└── pack-editor/                      # Planning docs (this folder)
    ├── ARCHITECTURE.md              # This document
    ├── DESIGN_SYSTEM_EXTRACTION.md  # Vue patterns reference
    └── README.md
```

---

## Next Steps

1. **Create `packages/ui`** - Initialize the shared component library
2. **Port design tokens** - Extract from Vue DESIGN_SYSTEM_EXTRACTION.md
3. **Build atoms first** - Button, Input, Badge, Icon
4. **Create type system** - All modifiers fully typed
5. **Build molecules** - Compose atoms
6. **Scaffold SvelteKit app** - `/app` directory
7. **Integrate with landing** - Import @caliber/ui into Astro

---

## References

- [Vue Design System Extraction](./DESIGN_SYSTEM_EXTRACTION.md)
- [Kilo Code GitHub](https://github.com/Kilo-Org/kilocode/)
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [Atomic Design Methodology](https://atomicdesign.bradfrost.com/)
