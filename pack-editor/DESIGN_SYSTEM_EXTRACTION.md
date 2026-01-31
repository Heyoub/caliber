# Pack Editor Design System Extraction

Analysis of Vue components from `/docs/` for integration into pack-editor.

## 1. Color System (from tailwind.config.mjs)

### Primary Palette (HSL Variables)
```css
:root {
  /* Coral - Primary accent, warm */
  --coral-200: 10 80% 80%;
  --coral-300: 10 75% 70%;
  --coral-400: 15 75% 60%;
  --coral-500: 15 85% 50%;  /* Primary */
  --coral-600: 20 80% 45%;
  --coral-700: 20 70% 35%;

  /* Teal - Secondary accent, cool */
  --teal-300: 170 65% 60%;
  --teal-400: 175 60% 50%;
  --teal-500: 180 70% 40%;  /* Primary */
  --teal-600: 185 65% 35%;
  --teal-700: 185 60% 25%;

  /* Mint - Success/positive */
  --mint-300: 155 65% 65%;
  --mint-400: 160 60% 55%;
  --mint-500: 165 70% 45%;  /* Primary */
  --mint-600: 170 65% 40%;
  --mint-700: 170 60% 30%;

  /* Lavender - Info/neutral accent */
  --lavender-300: 255 65% 75%;
  --lavender-400: 260 60% 65%;
  --lavender-500: 265 70% 55%;  /* Primary */
  --lavender-600: 270 65% 50%;
  --lavender-700: 270 60% 40%;

  /* Extended Slate scale for dark UI */
  --slate-750: 220 15% 30%;
  --slate-850: 222 18% 15%;
  --slate-950: 225 20% 5%;
}
```

### Usage Pattern
```typescript
// In Tailwind classes
'bg-[hsl(var(--coral-500))]'
'text-[hsla(var(--teal-500),0.6)]'  // With alpha
'border-[hsla(var(--mint-500),0.5)]'
```

## 2. Glassmorphic Base Styles

### Card Pattern (HybridGlassMorphicCard.vue:117-125)
```css
.hybrid-glass-card {
  background: rgba(15, 23, 42, 0.4);
  -webkit-backdrop-filter: blur(14px);
  backdrop-filter: blur(14px);
  border: 1px solid rgba(52, 211, 238, 0.2);
  border-radius: 16px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  color: white;
}
```

### Button Pattern (GlowPressButton.vue)
```typescript
const colorMap: Record<string, ColorConfig> = {
  glassy: {
    bg: 'bg-surface/20',
    hover: 'hover:bg-surface/30',
    text: 'text-white',
    border: 'border-white/20',
    shadow: 'shadow-[0_0.25rem_0.75rem_rgba(0,0,0,0.2)]',
    gradient: 'from-white/10 via-transparent to-transparent',
    glow: 'rgba(255, 255, 255, 0.3)',
    outerGlow: 'rgba(79, 209, 197, 0.3)'
  },
  coral: {
    bg: 'bg-[hsl(var(--coral-600))]',
    hover: 'hover:bg-[hsl(var(--coral-500))]',
    text: 'text-white',
    border: 'border-[hsla(var(--coral-400),0.5)]',
    shadow: 'shadow-[0_0.25rem_0.75rem_hsla(var(--coral-500),0.3)]',
    gradient: 'from-[hsla(var(--coral-300),0.3)] via-transparent to-transparent',
    glow: 'hsla(var(--coral-500), 0.5)',
    outerGlow: 'hsla(var(--coral-400), 0.3)'
  },
  // ... similar for teal, mint, lavender, navy, slate
};
```

## 3. Component Patterns

### 3.1 GlowPressButton (Primary CTA)
**Location**: `docs/GlowPressButton.vue`
**Props**:
- `color`: 'coral' | 'teal' | 'mint' | 'lavender' | 'navy' | 'slate' | 'glassy' | 'danger' | ...
- `size`: 'sm' | 'md' | 'lg'
- `forcePressed`: boolean
- `disabled`: boolean
- `href`: string (renders as `<a>`)
- `type`: 'button' | 'submit' | 'reset'

**Key Features**:
- Multi-layer glow effects (inner glow, outer glow, top shine)
- Press state with transform and color change
- Accessible (aria-pressed, disabled states)
- Hover effects with CSS transitions

### 3.2 GlassyButton (Secondary Actions)
**Location**: `docs/GlassyButton.vue`
**Props**:
- `type`, `disabled`, `active`, `fullWidth`, `href`, `className`

**Simpler pattern** - good for toolbar actions, less visual weight.

### 3.3 GlassMorphicCard
**Location**: `docs/GlassMorphicCard.vue`
**Props**:
- `glowColor`: 'purple' | 'blue' | 'teal' | 'coral' | 'mint' | 'lavender' | 'none'
- `hoverEffect`: boolean
- `theme`: 'light' | 'dark'

### 3.4 HybridGlassMorphicCard (Interactive)
**Location**: `docs/HybridGlassMorphicCard.vue`
**Props**: Same as GlassMorphicCard
**Key Features**:
- Mouse tracking for spotlight effect
- CSS variables `--mouse-x`, `--mouse-y` for radial gradient positioning
- `mix-blend-mode: soft-light` for overlay

```typescript
const handleMouseMove = (event: MouseEvent) => {
  const rect = cardRef.value.getBoundingClientRect();
  const percentX = (event.clientX - rect.left) / rect.width;
  const percentY = (event.clientY - rect.top) / rect.height;
  cardRef.value.style.setProperty('--mouse-x', `${percentX * 100}%`);
  cardRef.value.style.setProperty('--mouse-y', `${percentY * 100}%`);
};
```

## 4. Animation Patterns

### 4.1 Canvas Animations
Three patterns discovered:

**GridAnimation.vue** - Procedural grid with:
- Dynamic line rotation
- Traveling pulses along lines
- Collision detection
- Physics-based node movement

**NeuralAnimation.vue** - Node network:
- Center avoidance (repulsion from middle)
- Connection lines based on distance
- Velocity damping
- Edge bouncing

**HexScroll.vue** - Scrolling hex effect:
- Configurable density, intensity
- Node glow effects
- Gradient connections

### 4.2 CSS Keyframe Animations (from tailwind.config.mjs)
```javascript
animation: {
  'gradient-xy': 'gradient-xy 3s ease infinite',
  'glow-pulse': 'glow-pulse 2s ease-in-out infinite',
  'float': 'float 6s ease-in-out infinite',
  'shimmer': 'shimmer 2s linear infinite',
  'aurora': 'aurora 8s ease infinite',
  'spin-slow': 'spin 8s linear infinite',
  'bounce-subtle': 'bounce-subtle 2s ease-in-out infinite',
}
```

## 5. Composables & State Management

### 5.1 Pinia Stores Identified
| Store | Purpose | Key State |
|-------|---------|-----------|
| `useChatStore` | Chat messages & input | `selectedTextForInput`, messages |
| `useAiContextStore` | View mode | `viewMode: 'chat' | 'prompts' | 'templates' | ...` |
| `useSettingsStore` | User preferences | `quickPrompts`, `quickTemplates` |

### 5.2 Composables Identified
| Composable | Purpose |
|------------|---------|
| `useChatLogic` | Message sending, file handling, mic |
| `useVoiceChat` | Voice input state machine |
| `useRedactionStatus` | PII redaction state |
| `useClipboard` | Copy to clipboard |
| `useContextualSuggestionsGenerator` | AI suggestions |
| `useTemporaryRedactionSettings` | Redaction overrides |

### 5.3 Custom Directives
- `v-paste-redact`: Intercepts paste events, redacts PII

## 6. Layout Patterns

### 6.1 AuthLayout - Main Shell
```
┌─────────────────────────────────────────────┐
│ Mobile Menu (md:hidden)                     │
├──────────┬──────────────────────────────────┤
│          │ Content Area                     │
│ AuthNav  │ ┌────────────────────────────────┤
│ (sidebar)│ │ <router-view />                │
│ 20vw     │ │ with rounded corners           │
│          │ └────────────────────────────────┤
└──────────┴──────────────────────────────────┘
```

### 6.2 AuthNav - Sidebar Navigation
**Key patterns**:
- Floating UI (`@floating-ui/vue`) for dropdown positioning
- Hamburger → X animation via CSS transforms
- Laser/spotlight effect on mouse move
- Conditional tab components based on route

### 6.3 ChatPanel - View Switcher
```typescript
type ViewMode = 'chat' | 'prompts' | 'templates' | 'reasoning' | 'history' | 'documents';
```

## 7. Integration Mapping for Pack Editor

### 7.1 Direct Reuse (Copy & Adapt)
| Source Component | Target Location | Adaptations |
|------------------|-----------------|-------------|
| `GlowPressButton.vue` | `pack-editor/src/components/ui/` | None - generic |
| `GlassyButton.vue` | `pack-editor/src/components/ui/` | None - generic |
| `GlassMorphicCard.vue` | `pack-editor/src/components/ui/` | None - generic |
| `HybridGlassMorphicCard.vue` | `pack-editor/src/components/ui/` | None - generic |
| `tailwind.config.mjs` colors | `pack-editor/tailwind.config.ts` | Merge with existing |

### 7.2 Pattern Extraction (Adapt Logic)
| Pattern | From | To | Use Case |
|---------|------|----|-----------|
| View switching | ChatPanel.vue | PackEditorShell.vue | Editor/Graph/History views |
| File tree collapse | Sidebar.vue | FileTreePanel.vue | Agent/Profile sections |
| Floating dropdown | AuthNav.vue | EntitySelector.vue | Agent picker, toolset picker |
| Canvas animation | GridAnimation.vue | DependencyGraph.vue | Background decoration |

### 7.3 Store Pattern Mapping
| Docs Store | Pack Editor Equivalent |
|------------|----------------------|
| `useAiContextStore.viewMode` | `usePackStore.activeView` |
| `useChatStore.messages` | `usePackStore.validationErrors` |
| `useSettingsStore.quickPrompts` | `usePackStore.recentlyEdited` |

## 8. File Structure Recommendation

```
pack-editor/src/
├── components/
│   ├── ui/                    # Extracted from docs/
│   │   ├── GlowPressButton.vue
│   │   ├── GlassyButton.vue
│   │   ├── GlassMorphicCard.vue
│   │   ├── HybridGlassMorphicCard.vue
│   │   ├── Badge.vue
│   │   └── index.ts           # Re-exports
│   │
│   ├── layout/
│   │   ├── PackEditorShell.vue    # Main layout (from AuthLayout pattern)
│   │   ├── SidebarPanel.vue       # From Sidebar.vue pattern
│   │   └── ViewSwitcher.vue       # From ChatHeader pattern
│   │
│   ├── editor/
│   │   ├── CodeMirrorEditor.vue   # Main TOML/MD editor
│   │   ├── EditorTabs.vue         # Open file tabs
│   │   └── EditorToolbar.vue      # Save, validate, format
│   │
│   ├── tree/
│   │   ├── FileTree.vue           # Pack file browser
│   │   ├── FileTreeNode.vue       # Individual node
│   │   └── EntitySection.vue      # Collapsible section (from Sidebar)
│   │
│   ├── graph/
│   │   ├── DependencyGraph.vue    # D3 visualization
│   │   └── GraphBackground.vue    # From GridAnimation pattern
│   │
│   └── validation/
│       ├── ValidationPanel.vue    # Error list
│       └── ValidationBadge.vue    # Status indicator
│
├── composables/
│   ├── usePackStore.ts            # Already created
│   ├── useCodeMirror.ts           # Editor integration
│   └── useFloatingUI.ts           # From AuthNav pattern
│
└── styles/
    ├── variables.css              # HSL color system
    └── glassmorphic.css           # Shared effects
```

## 9. Priority Order for Implementation

1. **Phase 1: UI Foundation** (Copy directly)
   - [ ] GlowPressButton.vue
   - [ ] GlassyButton.vue
   - [ ] GlassMorphicCard.vue
   - [ ] Badge.vue
   - [ ] Tailwind color config

2. **Phase 2: Layout Shell**
   - [ ] PackEditorShell.vue (adapt AuthLayout)
   - [ ] SidebarPanel.vue (adapt Sidebar)
   - [ ] ViewSwitcher.vue (adapt ChatHeader)

3. **Phase 3: Tree Components**
   - [ ] FileTree.vue
   - [ ] EntitySection.vue (adapt collapsible pattern)

4. **Phase 4: Editor Integration**
   - [ ] CodeMirrorEditor.vue
   - [ ] EditorTabs.vue

5. **Phase 5: Graph & Polish**
   - [ ] DependencyGraph.vue
   - [ ] GraphBackground.vue (optional decoration)

## 10. Key Patterns to Preserve

### Accessibility
- `aria-expanded`, `aria-pressed`, `aria-current`
- Keyboard navigation (Escape to close)
- Focus management

### Performance
- `markRaw()` for icon components
- `requestAnimationFrame` for canvas
- Cleanup in `onUnmounted`

### Theming
- Always support `theme: 'light' | 'dark'` prop
- Use CSS variables for runtime theming
- Glassmorphic effects scale with opacity

### Responsive
- Mobile menu pattern (hamburger toggle)
- Flexible widths with min/max constraints
- Touch-friendly targets (min 44px)
