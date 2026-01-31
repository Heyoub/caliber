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

---

## 11. Advanced Animation Patterns (Page-Level)

### 11.1 ParallaxHero (docs/ParallaxHero.vue)
Custom parallax implementation without heavy libraries:

```typescript
// Smooth scroll with lerp (linear interpolation)
const lerp = (start: number, end: number, factor: number): number =>
  start + (end - start) * factor;

// Three-layer parallax speeds
const MASTER_SPEED = 0.5;
const CONTENT_SPEED = MASTER_SPEED * 0.85;  // Foreground
const BG_SPEED = MASTER_SPEED * 0.5;        // Background
const FG_SPEED = MASTER_SPEED * 0.75;       // Overlay

// Hardware-accelerated transforms
element.style.transform = `translate3d(0, ${currentY}px, 0)`;
```

**Key CSS for Performance:**
```css
.rellax-bg, .rellax-fg {
  will-change: transform;
  transform: translateZ(0);
  backface-visibility: hidden;
  transform-style: preserve-3d;
  perspective: 1000px;
  contain: paint layout;
}
```

### 11.2 IcosahedronAnimation (docs/IcosahedronAnimation.vue)
Three.js integration pattern:

```typescript
import * as THREE from 'three';

// Setup with alpha for transparency
renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true });

// Vertex colors for gradient effect
const colors: number[] = [];
for (let i = 0; i < positionAttribute.count; i++) {
  const y = positionAttribute.getY(i);
  const color = y > 0 ? color1 : color2;
  colors.push(color.r, color.g, color.b);
}
geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));

// Material with vertex colors
const material = new THREE.MeshPhongMaterial({
  shininess: 100,
  vertexColors: true
});
```

### 11.3 NeuralAnimation (docs/NeuralAnimation.vue)
Node network with physics:

```typescript
// Center repulsion
function applyRepulsion(node) {
  const dx = node.x - centerX;
  const dy = node.y - centerY;
  const distance = Math.sqrt(dx * dx + dy * dy);
  const repulsionRadius = Math.min(width, height) * 0.3;

  if (distance < repulsionRadius) {
    const force = (1 - distance / repulsionRadius) * 0.8;
    node.vx += (dx / distance) * force;
    node.vy += (dy / distance) * force;
  }
}

// Connection drawing with distance-based opacity
function drawConnection(x1, y1, x2, y2, distance) {
  const maxDistance = 250;
  const opacity = 1 - distance / maxDistance;
  ctx.strokeStyle = `rgba(34, 211, 238, ${opacity * 0.3})`;
}
```

### 11.4 CircularRings (docs/CircularRings.vue)
Pure CSS concentric ring animation:

```css
.ring {
  position: absolute;
  border-radius: 50%;
  border: 2px solid rgba(45,212,191,0.25);
  animation: pulse-ring 2.5s cubic-bezier(0.4, 0, 0.2, 1) infinite;
}
.ring1 { width: 120px; animation-delay: 0s; }
.ring2 { width: 90px; animation-delay: 0.6s; }
.ring3 { width: 60px; animation-delay: 1.2s; }
.ring4 { width: 30px; animation-delay: 1.8s; }

@keyframes pulse-ring {
  0%, 100% { opacity: 0.8; transform: scale(1); }
  50% { opacity: 0.2; transform: scale(1.08); }
}
```

## 12. Button Variants Deep Dive

### 12.1 GlowBrandButton (docs/GlowBrandButton.vue)
Advanced button with blob/lava lamp effects:

```typescript
// Persistent press state with minimum duration
const PRESS_ANIMATION_DURATION = 888;

function usePersistentPress(isPressed) {
  function start() {
    pressStart = Date.now();
    isPressed.value = true;
  }
  function end() {
    const elapsed = Date.now() - pressStart;
    const remaining = PRESS_ANIMATION_DURATION - elapsed;
    if (remaining > 0) {
      setTimeout(() => { isPressed.value = false; }, remaining);
    }
  }
}
```

**Blob Animation CSS:**
```css
@keyframes blob-move {
  0%, 100% {
    border-radius: 60% 40% 30% 70% / 60% 30% 70% 40%;
    transform: translate(-10px, 10px) scale(1.05);
  }
  25% {
    border-radius: 40% 60% 70% 30% / 50% 60% 30% 60%;
    transform: translate(10px, 10px) scale(1.1);
  }
  50% {
    border-radius: 30% 60% 70% 40% / 50% 60% 30% 60%;
    transform: translate(10px, -10px) scale(1.05);
  }
}
```

## 13. Page Layout Patterns

### 13.1 Hero Section (docs/Hero.vue)
```vue
<ParallaxHero :show-neural="props.showNeural">
  <LogoAnimation class="animate-pulse-slow" />
  <h1 class="text-5xl md:text-6xl lg:text-7xl">
    <span class="text-gradient text-gradient-primary">Thinking</span>
  </h1>
  <img src="/arrow-down.gif" class="animate-float filter-arrow" />
</ParallaxHero>
```

**Text Gradient Animation:**
```css
@keyframes gradient-flow {
  0% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
  100% { background-position: 0% 50%; }
}

.animate-gradient-flow {
  background-size: 200% 200%;
  animation: gradient-flow 6s ease infinite;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}
```

### 13.2 Journey Timeline (docs/Journey.vue)
Vertical timeline with color-coded items:

```typescript
interface TimelineItem {
  time: string;
  title: string;
  icon: Component;
  iconColorClass: string;      // "text-[hsl(var(--coral-400))]"
  bgColor: string;             // "bg-slate-600/70"
  borderColor: string;         // "border-[hsla(var(--coral-500),0.6)]"
  glowColor: string;           // "bg-[hsla(var(--coral-500),0.4)]"
  accentColorVar: string;      // "--coral-500"
}
```

### 13.3 Choice Point (docs/ChoicePoint.vue)
A/B path selection with conditional feature display:

```vue
<GlassMorphicCard
  glowColor="mint"
  :class="selectedPath === 'focus' ? 'ring-4 ring-mint-400' : ''"
  @click="selectPath('focus')"
>
  <!-- Path content -->
</GlassMorphicCard>

<transition name="fade-slide" mode="out-in">
  <FeaturesCloud v-if="selectedFeatures === 'cloud'" />
  <FeaturesPrem v-else-if="selectedFeatures === 'prem'" />
</transition>
```

### 13.4 Dashboard Mockup (docs/dashboob.vue)
Animated UI skeleton with floating 3D effect:

```css
@keyframes floatCard {
  0% { transform: translateY(0px) rotateX(2.5deg) rotateY(-1.5deg) scale(0.97); }
  50% { transform: translateY(-12px) rotateX(1deg) rotateY(1.5deg) scale(1); }
  100% { transform: translateY(0px) rotateX(2.5deg) rotateY(-1.5deg) scale(0.97); }
}

@keyframes chartBar {
  0% { opacity: 0; transform: scaleY(0); }
  100% { opacity: 1; transform: scaleY(1); }
}

.animate-chart-bar {
  animation: chartBar 1.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  transform-origin: bottom;
}
```

## 14. Form & Input Patterns

### 14.1 Glassmorphic Input (docs/Invitation.vue)
```html
<input
  class="w-full px-4 py-3
         bg-slate-800/30 backdrop-blur-xl
         text-white rounded-lg
         shadow-[0_0.25rem_0.75rem_rgba(79,209,197,0.15)]
         border-white/10 border-solid
         focus:ring-2 focus:border-teal-500 focus:ring-teal-500/50
         transition-all duration-300"
/>
```

### 14.2 Custom Select with Laser Glow
```css
.dropdown-wrapper:hover .laser-glow,
.dropdown-wrapper:focus-within .laser-glow {
  opacity: 0.15;
}

.laser-glow {
  background: radial-gradient(circle at center, #4FD1C5 0%, transparent 70%);
  box-shadow: 0 0 15px 2px rgba(79, 209, 197, 0.5),
              inset 0 0 8px rgba(79, 209, 197, 0.3);
}
```

## 15. Admin & Data Patterns

### 15.1 AdminPanel Tab System (docs/AdminPanel.vue)
Dynamic component loading with shallowRef:

```typescript
import { shallowRef, Component } from 'vue';

const tabs: TabInfo[] = [
  { id: 'models', label: 'Model Controls', component: shallowRef(ModelControls) },
  { id: 'tokens', label: 'Token Management', component: shallowRef(TokenManagement) },
  // ...
];

const activeComponent = computed(() =>
  tabs.find(tab => tab.id === activeTab.value)?.component
);
```

### 15.2 ChatHistory List (docs/ChatHistory.vue)
Sorted list with active state:

```typescript
const sortedChats = computed(() =>
  [...chats.value].sort((a, b) =>
    new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
  )
);
```

## 16. Accordion Pattern (docs/Accordion.vue)

Click-outside detection with inverted triangle text:

```typescript
const handleClickOutside = (event: MouseEvent) => {
  if (accordionRef.value && !accordionRef.value.contains(event.target as Node)) {
    isOpen.value = false;
  }
};

onMounted(() => document.addEventListener('click', handleClickOutside));
onUnmounted(() => document.removeEventListener('click', handleClickOutside));
```

**Inverted Triangle Text Layout:**
```css
.inverted-triangle :deep(p:first-of-type) { max-width: 75%; margin: 0 auto; }
.inverted-triangle :deep(p:nth-of-type(2)) { max-width: 85%; margin: 0 auto; }
.inverted-triangle :deep(p:nth-of-type(3)) { max-width: 90%; margin: 0 auto; }
.inverted-triangle :deep(p:nth-of-type(n+4)) { max-width: 95%; margin: 0 auto; }
```

## 17. SVG Gradient Patterns

### Underline with Brand Gradient (docs/Invitation.vue)
```html
<svg viewBox="0 0 200 8" class="absolute -bottom-2.5 w-full">
  <path
    d="M1 5.5C40 1.5 160 1.5 199 5.5"
    stroke="url(#paint0_linear_invitation)"
    stroke-width="4"
    stroke-linecap="round"
  />
  <defs>
    <linearGradient id="paint0_linear_invitation" x1="1" y1="3" x2="199" y2="3">
      <stop stop-color="hsl(var(--coral-500))"/>
      <stop offset="0.33" stop-color="hsl(var(--teal-500))"/>
      <stop offset="0.66" stop-color="hsl(var(--mint-500))"/>
      <stop offset="1" stop-color="hsl(var(--lavender-500))"/>
    </linearGradient>
  </defs>
</svg>
```

## 18. Summary: Components Ready for Pack Editor

| Category | Components | Status |
|----------|------------|--------|
| **Buttons** | GlowPressButton, GlassyButton, GlowBrandButton | Extracted |
| **Cards** | GlassMorphicCard, HybridGlassMorphicCard | Extracted |
| **Layout** | AuthLayout, AiMainLayout, Sidebar, AdminPanel | Pattern documented |
| **Animation** | NeuralAnimation, GridAnimation, CircularRings | Pattern documented |
| **3D** | IcosahedronAnimation, ParallaxHero | Pattern documented |
| **Forms** | Invitation (inputs), TemplateLibrary | Pattern documented |
| **Data** | ChatHistory, Accordion, ChatPanel | Pattern documented |

All 44 Vue files have been analyzed. The design system is now fully documented for pack-editor integration.
