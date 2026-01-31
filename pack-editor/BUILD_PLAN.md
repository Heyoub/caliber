# CALIBER Pack Editor - Complete Build Plan A→Z

> Memory Editor UI for the CALIBER cognitive architecture

---

## Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CALIBER PACK EDITOR                             │
├─────────────────────────────────────────────────────────────────────────┤
│  A thin, atomic UI layer in TypeScript/Svelte that connects to a        │
│  Rust API backend. The frontend handles rendering and user input;       │
│  all intelligence (model providers, memory ops) lives in Rust.          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 0: Foundation ✅ COMPLETE

### What We Have

| Asset | Location | Status |
|-------|----------|--------|
| **Type System** | `packages/ui/src/types/` | ✅ 8 files |
| **Atomic Components** | `packages/ui/src/atoms/` | ✅ 15 components |
| **Molecule Components** | `packages/ui/src/molecules/` | ✅ 14 components |
| **Organism Components** | `packages/ui/src/organisms/` | ✅ 16 components |
| **Animation Library** | `packages/ui/src/styles/animations.css` | ✅ 25+ keyframes |
| **Content CMS** | `packages/ui/content/` | ✅ YAML files |
| **Architecture Docs** | `pack-editor/ARCHITECTURE.md` | ✅ 1166 lines |
| **Structure Patterns** | `packages/ui/STRUCTURE_PATTERNS.md` | ✅ 488 lines |

### Component Inventory

```
atoms/          (15)        molecules/      (14)        organisms/      (16)
├── Avatar                  ├── Accordion               ├── Card
├── Badge                   ├── Breadcrumb              ├── ChatMessage
├── Button                  ├── ButtonGroup             ├── ChatPanel
├── CircularRings           ├── Dropdown                ├── CommandPalette
├── Divider                 ├── FileTreeItem            ├── DiffView
├── HamburgerButton         ├── InputGroup              ├── EditorPanel
├── Icon                    ├── Modal                   ├── FileTree
├── IconButton              ├── ModeSelector            ├── FormatViewer
├── Input                   ├── NestedMenu              ├── GridAnimation
├── Kbd                     ├── Pagination              ├── HexScroll
├── Spinner                 ├── PromptButtons           ├── IcosahedronAnimation
├── StyledHeading           ├── SearchInput             ├── MemoryGraph
├── TextArea                ├── Tabs                    ├── NeuralAnimation
├── Toggle                  └── Toast                   ├── ParallaxHero
└── Tooltip                                             ├── Sidebar
                                                        ├── ToolCallCard
                                                        └── TreeView
```

---

## Phase 1: SvelteKit App Shell

### 1A. Project Setup

```bash
# Create SvelteKit app
cd caliber
npx sv create app --template minimal --types ts

# Install dependencies
cd app
npm install -D tailwindcss postcss autoprefixer
npm install @floating-ui/dom lucide-svelte
npm install marked dompurify mermaid katex
npm install @codemirror/state @codemirror/view @codemirror/lang-json
npm install @codemirror/lang-yaml @codemirror/lang-xml @codemirror/lang-markdown

# Link local UI package
npm install ../packages/ui
```

### 1B. Directory Structure

```
app/
├── src/
│   ├── lib/
│   │   ├── api/              # Rust API client
│   │   │   ├── client.ts     # HTTP/WebSocket client
│   │   │   ├── memory.ts     # Memory operations
│   │   │   ├── mcp.ts        # MCP protocol client
│   │   │   └── types.ts      # API response types
│   │   │
│   │   ├── stores/           # Svelte stores
│   │   │   ├── auth.ts       # Auth state
│   │   │   ├── mode.ts       # Assistant/Playground mode
│   │   │   ├── memory.ts     # Memory tree state
│   │   │   ├── editor.ts     # Editor state (tabs, content)
│   │   │   └── chat.ts       # Chat messages
│   │   │
│   │   └── utils/            # Utilities
│   │       ├── markdown.ts   # Marked + DOMPurify
│   │       ├── formats.ts    # Format detection
│   │       └── keyboard.ts   # Keyboard shortcuts
│   │
│   ├── routes/
│   │   ├── +layout.svelte    # App shell
│   │   ├── +page.svelte      # Redirect to /dashboard
│   │   │
│   │   ├── dashboard/
│   │   │   └── +page.svelte  # Dashboard overview
│   │   │
│   │   └── editor/
│   │       ├── +layout.svelte # Editor layout (sidebar + main)
│   │       ├── assistant/
│   │       │   └── +page.svelte
│   │       └── playground/
│   │           └── +page.svelte
│   │
│   ├── app.css               # Tailwind + animations import
│   └── app.html              # HTML shell
│
├── static/
│   └── fonts/                # Custom fonts
│
├── svelte.config.js
├── tailwind.config.js
└── vite.config.ts
```

### 1C. Tailwind Configuration

```javascript
// tailwind.config.js
export default {
  content: [
    './src/**/*.{html,js,svelte,ts}',
    '../packages/ui/src/**/*.svelte'
  ],
  theme: {
    extend: {
      colors: {
        // CALIBER palette
        teal: { 500: 'hsl(175 70% 40%)' },
        coral: { 500: 'hsl(15 85% 50%)' },
        mint: { 500: 'hsl(165 70% 45%)' },
        lavender: { 500: 'hsl(265 70% 55%)' },
        purple: { 500: 'hsl(270 70% 60%)' },
        slate: {
          800: 'hsl(222 18% 20%)',
          900: 'hsl(225 20% 10%)',
        }
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
        title: ['Cal Sans', 'Inter', 'sans-serif'],
      },
      animation: {
        'blob-move': 'blob-move 20s ease-in-out infinite',
        'pulse-glow': 'pulse-glow 4s ease-in-out infinite',
        'gradient-flow': 'gradient-flow 15s ease infinite',
        'float-hero': 'float-hero 6s ease-in-out infinite',
        'spin-slow': 'spin 8s linear infinite',
        'spin-very-slow': 'spin 20s linear infinite',
      }
    }
  },
  plugins: []
}
```

---

## Phase 2: Core Layout & Navigation

### 2A. App Shell Layout

```svelte
<!-- src/routes/+layout.svelte -->
<script>
  import { Sidebar, CommandPalette } from '@caliber/ui/organisms';
  import { mode } from '$lib/stores/mode';
  import '../app.css';
</script>

<div class="flex h-screen bg-slate-900 text-slate-100">
  <!-- Sidebar (collapsible) -->
  <Sidebar {$mode} />

  <!-- Main content area -->
  <main class="flex-1 overflow-hidden">
    <slot />
  </main>

  <!-- Command palette (global) -->
  <CommandPalette />
</div>
```

### 2B. Editor Layout

```svelte
<!-- src/routes/editor/+layout.svelte -->
<script>
  import { FileTree, MemoryGraph } from '@caliber/ui/organisms';
  import { ModeSelector } from '@caliber/ui/molecules';
  import { memory } from '$lib/stores/memory';
  import { mode } from '$lib/stores/mode';
</script>

<div class="flex h-full">
  <!-- Left: Memory browser -->
  <aside class="w-64 border-r border-slate-700 flex flex-col">
    <div class="p-4 border-b border-slate-700">
      <ModeSelector bind:mode={$mode} />
    </div>
    <FileTree nodes={$memory.tree} />
  </aside>

  <!-- Center: Editor + Chat -->
  <div class="flex-1 flex flex-col">
    <slot />
  </div>

  <!-- Right: Context panel (optional) -->
  <aside class="w-80 border-l border-slate-700 hidden lg:block">
    <MemoryGraph nodes={$memory.graph} />
  </aside>
</div>
```

---

## Phase 3: API Client Layer

### 3A. Rust API Client

```typescript
// src/lib/api/client.ts
const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000';

interface ApiResponse<T> {
  data?: T;
  error?: { code: string; message: string };
}

class CaliberClient {
  private ws: WebSocket | null = null;

  async get<T>(path: string): Promise<T> {
    const res = await fetch(`${API_BASE}${path}`);
    const json: ApiResponse<T> = await res.json();
    if (json.error) throw new Error(json.error.message);
    return json.data!;
  }

  async post<T>(path: string, body: unknown): Promise<T> {
    const res = await fetch(`${API_BASE}${path}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    const json: ApiResponse<T> = await res.json();
    if (json.error) throw new Error(json.error.message);
    return json.data!;
  }

  // WebSocket for streaming
  connectStream(onMessage: (data: unknown) => void) {
    this.ws = new WebSocket(`${API_BASE.replace('http', 'ws')}/stream`);
    this.ws.onmessage = (e) => onMessage(JSON.parse(e.data));
    return () => this.ws?.close();
  }
}

export const api = new CaliberClient();
```

### 3B. Memory Operations

```typescript
// src/lib/api/memory.ts
import { api } from './client';
import type { Trajectory, Scope, Turn, MemoryNode } from '@caliber/ui/types';

export const memoryApi = {
  // List all trajectories
  async listTrajectories(): Promise<Trajectory[]> {
    return api.get('/memory/trajectories');
  },

  // Get trajectory with scopes
  async getTrajectory(id: string): Promise<Trajectory> {
    return api.get(`/memory/trajectories/${id}`);
  },

  // Get scope with turns
  async getScope(trajectoryId: string, scopeId: string): Promise<Scope> {
    return api.get(`/memory/trajectories/${trajectoryId}/scopes/${scopeId}`);
  },

  // Get turn content
  async getTurn(trajectoryId: string, scopeId: string, turnId: string): Promise<Turn> {
    return api.get(`/memory/trajectories/${trajectoryId}/scopes/${scopeId}/turns/${turnId}`);
  },

  // Create new trajectory
  async createTrajectory(name: string): Promise<Trajectory> {
    return api.post('/memory/trajectories', { name });
  },

  // Fork scope
  async forkScope(trajectoryId: string, scopeId: string, name: string): Promise<Scope> {
    return api.post(`/memory/trajectories/${trajectoryId}/scopes/${scopeId}/fork`, { name });
  },

  // Update turn
  async updateTurn(path: { trajectoryId: string; scopeId: string; turnId: string }, content: string): Promise<Turn> {
    return api.post(`/memory/trajectories/${path.trajectoryId}/scopes/${path.scopeId}/turns/${path.turnId}`, { content });
  }
};
```

### 3C. MCP Client

```typescript
// src/lib/api/mcp.ts
import { api } from './client';
import type { ToolCall, ToolResult, Resource, Prompt } from '@caliber/ui/types';

export const mcpApi = {
  // List available tools
  async listTools(): Promise<ToolCall[]> {
    return api.get('/mcp/tools');
  },

  // Execute tool
  async executeTool(name: string, args: Record<string, unknown>): Promise<ToolResult> {
    return api.post('/mcp/tools/execute', { name, arguments: args });
  },

  // List resources
  async listResources(): Promise<Resource[]> {
    return api.get('/mcp/resources');
  },

  // Read resource
  async readResource(uri: string): Promise<{ contents: string; mimeType: string }> {
    return api.get(`/mcp/resources?uri=${encodeURIComponent(uri)}`);
  },

  // List prompts
  async listPrompts(): Promise<Prompt[]> {
    return api.get('/mcp/prompts');
  },

  // Get prompt with arguments
  async getPrompt(name: string, args: Record<string, string>): Promise<{ messages: unknown[] }> {
    return api.post('/mcp/prompts/get', { name, arguments: args });
  }
};
```

---

## Phase 4: State Management (Stores)

### 4A. Mode Store

```typescript
// src/lib/stores/mode.ts
import { writable, derived } from 'svelte/store';

export type EditorMode = 'assistant' | 'playground';

export const mode = writable<EditorMode>('assistant');

// Derived store for mode-specific UI hints
export const modeConfig = derived(mode, ($mode) => ({
  title: $mode === 'assistant' ? 'Assistant Mode' : 'Playground Mode',
  description: $mode === 'assistant'
    ? 'AI-assisted editing with suggestions'
    : 'Direct editing without AI',
  showChat: $mode === 'assistant',
  showTools: $mode === 'assistant',
}));
```

### 4B. Memory Store

```typescript
// src/lib/stores/memory.ts
import { writable, derived } from 'svelte/store';
import { memoryApi } from '$lib/api/memory';
import type { Trajectory, Scope, Turn, TreeNode } from '@caliber/ui/types';

interface MemoryState {
  trajectories: Trajectory[];
  selectedTrajectory: string | null;
  selectedScope: string | null;
  selectedTurn: string | null;
  loading: boolean;
  error: string | null;
}

function createMemoryStore() {
  const { subscribe, set, update } = writable<MemoryState>({
    trajectories: [],
    selectedTrajectory: null,
    selectedScope: null,
    selectedTurn: null,
    loading: false,
    error: null,
  });

  return {
    subscribe,

    async load() {
      update(s => ({ ...s, loading: true }));
      try {
        const trajectories = await memoryApi.listTrajectories();
        update(s => ({ ...s, trajectories, loading: false }));
      } catch (e) {
        update(s => ({ ...s, error: (e as Error).message, loading: false }));
      }
    },

    selectTrajectory(id: string) {
      update(s => ({ ...s, selectedTrajectory: id, selectedScope: null, selectedTurn: null }));
    },

    selectScope(id: string) {
      update(s => ({ ...s, selectedScope: id, selectedTurn: null }));
    },

    selectTurn(id: string) {
      update(s => ({ ...s, selectedTurn: id }));
    },
  };
}

export const memory = createMemoryStore();

// Derived: Build tree structure for FileTree component
export const memoryTree = derived(memory, ($memory) => {
  return $memory.trajectories.map(t => ({
    id: t.id,
    label: t.name,
    type: 'trajectory' as const,
    expanded: t.id === $memory.selectedTrajectory,
    children: t.scopes?.map(s => ({
      id: s.id,
      label: s.name,
      type: 'scope' as const,
      expanded: s.id === $memory.selectedScope,
      children: s.turns?.map(turn => ({
        id: turn.id,
        label: `Turn ${turn.index}`,
        type: 'turn' as const,
        selected: turn.id === $memory.selectedTurn,
      })) ?? [],
    })) ?? [],
  }));
});
```

### 4C. Editor Store

```typescript
// src/lib/stores/editor.ts
import { writable, derived } from 'svelte/store';

interface EditorTab {
  id: string;
  title: string;
  content: string;
  format: 'markdown' | 'yaml' | 'json' | 'toml' | 'xml' | 'csv';
  dirty: boolean;
  path?: { trajectoryId: string; scopeId: string; turnId: string };
}

interface EditorState {
  tabs: EditorTab[];
  activeTabId: string | null;
}

function createEditorStore() {
  const { subscribe, update } = writable<EditorState>({
    tabs: [],
    activeTabId: null,
  });

  return {
    subscribe,

    openTab(tab: Omit<EditorTab, 'dirty'>) {
      update(s => {
        const existing = s.tabs.find(t => t.id === tab.id);
        if (existing) {
          return { ...s, activeTabId: tab.id };
        }
        return {
          tabs: [...s.tabs, { ...tab, dirty: false }],
          activeTabId: tab.id,
        };
      });
    },

    closeTab(id: string) {
      update(s => {
        const tabs = s.tabs.filter(t => t.id !== id);
        const activeTabId = s.activeTabId === id
          ? tabs[tabs.length - 1]?.id ?? null
          : s.activeTabId;
        return { tabs, activeTabId };
      });
    },

    updateContent(id: string, content: string) {
      update(s => ({
        ...s,
        tabs: s.tabs.map(t =>
          t.id === id ? { ...t, content, dirty: true } : t
        ),
      }));
    },

    markSaved(id: string) {
      update(s => ({
        ...s,
        tabs: s.tabs.map(t =>
          t.id === id ? { ...t, dirty: false } : t
        ),
      }));
    },
  };
}

export const editor = createEditorStore();

export const activeTab = derived(editor, ($editor) =>
  $editor.tabs.find(t => t.id === $editor.activeTabId) ?? null
);
```

### 4D. Chat Store

```typescript
// src/lib/stores/chat.ts
import { writable } from 'svelte/store';
import type { ChatMessage, ToolCall } from '@caliber/ui/types';

interface ChatState {
  messages: ChatMessage[];
  pendingToolCalls: ToolCall[];
  streaming: boolean;
  streamBuffer: string;
}

function createChatStore() {
  const { subscribe, update } = writable<ChatState>({
    messages: [],
    pendingToolCalls: [],
    streaming: false,
    streamBuffer: '',
  });

  return {
    subscribe,

    addMessage(message: ChatMessage) {
      update(s => ({ ...s, messages: [...s.messages, message] }));
    },

    startStreaming() {
      update(s => ({ ...s, streaming: true, streamBuffer: '' }));
    },

    appendStream(chunk: string) {
      update(s => ({ ...s, streamBuffer: s.streamBuffer + chunk }));
    },

    endStreaming() {
      update(s => {
        if (s.streamBuffer) {
          return {
            ...s,
            streaming: false,
            messages: [...s.messages, {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: s.streamBuffer,
              timestamp: new Date().toISOString(),
            }],
            streamBuffer: '',
          };
        }
        return { ...s, streaming: false };
      });
    },

    addToolCall(toolCall: ToolCall) {
      update(s => ({ ...s, pendingToolCalls: [...s.pendingToolCalls, toolCall] }));
    },

    resolveToolCall(id: string, result: unknown) {
      update(s => ({
        ...s,
        pendingToolCalls: s.pendingToolCalls.map(tc =>
          tc.id === id ? { ...tc, status: 'completed', result } : tc
        ),
      }));
    },

    clear() {
      update(() => ({
        messages: [],
        pendingToolCalls: [],
        streaming: false,
        streamBuffer: '',
      }));
    },
  };
}

export const chat = createChatStore();
```

---

## Phase 5: Editor Pages

### 5A. Assistant Mode Page

```svelte
<!-- src/routes/editor/assistant/+page.svelte -->
<script>
  import { EditorPanel, ChatPanel, ToolCallCard } from '@caliber/ui/organisms';
  import { editor, activeTab } from '$lib/stores/editor';
  import { chat } from '$lib/stores/chat';
  import { memory } from '$lib/stores/memory';

  async function handleSend(message: string) {
    chat.addMessage({
      id: crypto.randomUUID(),
      role: 'user',
      content: message,
      timestamp: new Date().toISOString(),
    });

    // API call handled by chat store + streaming
  }

  function handleToolApprove(toolCall) {
    // Execute via MCP API
  }
</script>

<div class="flex h-full">
  <!-- Editor panel (left/center) -->
  <div class="flex-1 flex flex-col">
    <EditorPanel
      tabs={$editor.tabs}
      activeTabId={$editor.activeTabId}
      onTabChange={(id) => editor.openTab({ id })}
      onTabClose={(id) => editor.closeTab(id)}
    >
      {#if $activeTab}
        <!-- CodeMirror instance -->
        <CodeEditor
          content={$activeTab.content}
          format={$activeTab.format}
          onChange={(c) => editor.updateContent($activeTab.id, c)}
        />
      {/if}
    </EditorPanel>
  </div>

  <!-- Chat panel (right) -->
  <div class="w-96 border-l border-slate-700 flex flex-col">
    <ChatPanel
      messages={$chat.messages}
      streaming={$chat.streaming}
      streamBuffer={$chat.streamBuffer}
      onSend={handleSend}
    />

    <!-- Pending tool calls -->
    {#each $chat.pendingToolCalls.filter(tc => tc.status === 'pending') as toolCall}
      <ToolCallCard
        {toolCall}
        onApprove={() => handleToolApprove(toolCall)}
        onReject={() => chat.resolveToolCall(toolCall.id, { rejected: true })}
      />
    {/each}
  </div>
</div>
```

### 5B. Playground Mode Page

```svelte
<!-- src/routes/editor/playground/+page.svelte -->
<script>
  import { EditorPanel, FormatViewer, DiffView } from '@caliber/ui/organisms';
  import { Tabs } from '@caliber/ui/molecules';
  import { editor, activeTab } from '$lib/stores/editor';

  let viewMode: 'edit' | 'preview' | 'diff' = 'edit';
</script>

<div class="flex h-full flex-col">
  <!-- View mode tabs -->
  <div class="border-b border-slate-700 px-4">
    <Tabs
      tabs={[
        { id: 'edit', label: 'Edit' },
        { id: 'preview', label: 'Preview' },
        { id: 'diff', label: 'Diff' },
      ]}
      activeTab={viewMode}
      onTabChange={(id) => viewMode = id}
    />
  </div>

  <!-- Editor/Preview area -->
  <div class="flex-1 overflow-hidden">
    {#if viewMode === 'edit'}
      <EditorPanel
        tabs={$editor.tabs}
        activeTabId={$editor.activeTabId}
        onTabChange={(id) => editor.openTab({ id })}
        onTabClose={(id) => editor.closeTab(id)}
      >
        {#if $activeTab}
          <CodeEditor
            content={$activeTab.content}
            format={$activeTab.format}
            onChange={(c) => editor.updateContent($activeTab.id, c)}
          />
        {/if}
      </EditorPanel>

    {:else if viewMode === 'preview'}
      {#if $activeTab}
        <FormatViewer
          content={$activeTab.content}
          format={$activeTab.format}
        />
      {/if}

    {:else if viewMode === 'diff'}
      <DiffView
        original={$activeTab?.originalContent ?? ''}
        modified={$activeTab?.content ?? ''}
      />
    {/if}
  </div>
</div>
```

---

## Phase 6: CodeMirror Integration

### 6A. CodeEditor Component

```svelte
<!-- src/lib/components/CodeEditor.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView, basicSetup } from 'codemirror';
  import { EditorState } from '@codemirror/state';
  import { json } from '@codemirror/lang-json';
  import { yaml } from '@codemirror/lang-yaml';
  import { xml } from '@codemirror/lang-xml';
  import { markdown } from '@codemirror/lang-markdown';
  import { oneDark } from '@codemirror/theme-one-dark';

  export let content: string;
  export let format: string;
  export let onChange: (content: string) => void;

  let container: HTMLDivElement;
  let view: EditorView;

  const languageMap = {
    json: json(),
    yaml: yaml(),
    xml: xml(),
    markdown: markdown(),
    toml: [], // Add TOML extension when available
    csv: [],  // Plain text for CSV
  };

  onMount(() => {
    const state = EditorState.create({
      doc: content,
      extensions: [
        basicSetup,
        oneDark,
        languageMap[format] ?? [],
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onChange(update.state.doc.toString());
          }
        }),
      ],
    });

    view = new EditorView({ state, parent: container });
  });

  onDestroy(() => {
    view?.destroy();
  });

  // Update content when prop changes externally
  $: if (view && content !== view.state.doc.toString()) {
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: content }
    });
  }
</script>

<div bind:this={container} class="h-full w-full overflow-hidden"></div>
```

---

## Phase 7: Markdown & Format Rendering

### 7A. Markdown Utility

```typescript
// src/lib/utils/markdown.ts
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import mermaid from 'mermaid';
import katex from 'katex';

// Initialize mermaid
mermaid.initialize({
  startOnLoad: false,
  theme: 'dark',
  securityLevel: 'loose',
});

// Custom renderer for code blocks
const renderer = new marked.Renderer();

renderer.code = (code, language) => {
  if (language === 'mermaid') {
    const id = `mermaid-${Math.random().toString(36).slice(2)}`;
    return `<div class="mermaid" id="${id}">${code}</div>`;
  }

  if (language === 'math' || language === 'latex') {
    try {
      return katex.renderToString(code, { displayMode: true });
    } catch {
      return `<pre class="error">${code}</pre>`;
    }
  }

  return `<pre><code class="language-${language}">${code}</code></pre>`;
};

// Inline math: $...$
renderer.text = (text) => {
  return text.replace(/\$([^$]+)\$/g, (_, math) => {
    try {
      return katex.renderToString(math, { displayMode: false });
    } catch {
      return `<code class="error">${math}</code>`;
    }
  });
};

marked.setOptions({ renderer });

export async function renderMarkdown(content: string): Promise<string> {
  const html = marked.parse(content);
  const clean = DOMPurify.sanitize(html, {
    ADD_TAGS: ['iframe'],
    ADD_ATTR: ['allow', 'allowfullscreen', 'frameborder'],
  });

  // Process mermaid diagrams after DOM insertion
  // (caller should call mermaid.run() after inserting HTML)

  return clean;
}

export async function renderMermaidDiagrams() {
  await mermaid.run();
}
```

---

## Phase 8: Keyboard Shortcuts

### 8A. Keyboard Handler

```typescript
// src/lib/utils/keyboard.ts
type ShortcutHandler = () => void | Promise<void>;

interface Shortcut {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  handler: ShortcutHandler;
  description: string;
}

const shortcuts: Shortcut[] = [];

export function registerShortcut(shortcut: Shortcut) {
  shortcuts.push(shortcut);
  return () => {
    const idx = shortcuts.indexOf(shortcut);
    if (idx > -1) shortcuts.splice(idx, 1);
  };
}

export function initKeyboardHandler() {
  document.addEventListener('keydown', (e) => {
    for (const s of shortcuts) {
      const keyMatch = e.key.toLowerCase() === s.key.toLowerCase();
      const ctrlMatch = !!s.ctrl === (e.ctrlKey || e.metaKey);
      const shiftMatch = !!s.shift === e.shiftKey;
      const altMatch = !!s.alt === e.altKey;

      if (keyMatch && ctrlMatch && shiftMatch && altMatch) {
        e.preventDefault();
        s.handler();
        return;
      }
    }
  });
}

// Common shortcuts
export const defaultShortcuts: Shortcut[] = [
  { key: 'k', ctrl: true, handler: () => {/* open command palette */}, description: 'Open command palette' },
  { key: 's', ctrl: true, handler: () => {/* save */}, description: 'Save' },
  { key: 'p', ctrl: true, shift: true, handler: () => {/* toggle preview */}, description: 'Toggle preview' },
  { key: 'b', ctrl: true, handler: () => {/* toggle sidebar */}, description: 'Toggle sidebar' },
];
```

---

## Phase 9: Landing Page (Astro + Svelte Islands)

### 9A. Landing Setup

```bash
# Create Astro landing
npx create-astro@latest landing --template minimal

cd landing
npx astro add svelte tailwind
```

### 9B. Landing Structure

```
landing/
├── src/
│   ├── components/
│   │   ├── Hero.astro
│   │   ├── Features.astro
│   │   └── interactive/      # Svelte islands
│   │       ├── NeuralAnimation.svelte
│   │       ├── ParallaxHero.svelte
│   │       └── GridAnimation.svelte
│   │
│   ├── layouts/
│   │   └── BaseLayout.astro
│   │
│   ├── pages/
│   │   ├── index.astro       # Home
│   │   ├── features.astro
│   │   └── pricing.astro
│   │
│   └── styles/
│       └── global.css
│
└── astro.config.mjs
```

---

## Phase 10: Testing & Polish

### 10A. Test Setup

```bash
npm install -D vitest @testing-library/svelte jsdom
```

### 10B. Component Tests

```typescript
// tests/components/Button.test.ts
import { render, fireEvent } from '@testing-library/svelte';
import { Button } from '@caliber/ui/atoms';

test('Button fires click event', async () => {
  const { getByRole } = render(Button, { props: { children: 'Click me' } });
  const button = getByRole('button');
  await fireEvent.click(button);
  // assertions...
});
```

---

## Build Order Summary

```
Phase 0: Foundation ✅
    └── Types, Components, Animations, Content

Phase 1: SvelteKit App Shell
    ├── 1A. Project setup
    ├── 1B. Directory structure
    └── 1C. Tailwind config

Phase 2: Core Layout
    ├── 2A. App shell layout
    └── 2B. Editor layout

Phase 3: API Client
    ├── 3A. Base client
    ├── 3B. Memory operations
    └── 3C. MCP client

Phase 4: State Management
    ├── 4A. Mode store
    ├── 4B. Memory store
    ├── 4C. Editor store
    └── 4D. Chat store

Phase 5: Editor Pages
    ├── 5A. Assistant mode
    └── 5B. Playground mode

Phase 6: CodeMirror
    └── 6A. CodeEditor component

Phase 7: Markdown
    └── 7A. Rendering utilities

Phase 8: Keyboard
    └── 8A. Shortcut handler

Phase 9: Landing Page
    ├── 9A. Astro setup
    └── 9B. Svelte islands

Phase 10: Testing
    ├── 10A. Test setup
    └── 10B. Component tests
```

---

## Commands Reference

```bash
# Development
cd app && npm run dev           # SvelteKit dev server
cd landing && npm run dev       # Astro dev server

# Build
cd app && npm run build         # Build SvelteKit app
cd landing && npm run build     # Build landing

# Test
npm run test                    # Run all tests
npm run test:watch              # Watch mode

# Type check
npm run check                   # Svelte check
```

---

*This plan is the complete roadmap from foundation to production-ready Pack Editor.*
