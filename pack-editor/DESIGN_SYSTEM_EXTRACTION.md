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

---

## 19. Mobile Menu Pattern (Hamburger → X Animation)

### 19.1 AuthLayout Mobile Button (docs/AuthLayout.vue:4-23)
Fixed position button with animated hamburger lines:

```vue
<button
  @click="toggleMobileMenu"
  :class="['w-[3rem] h-[3rem] flex flex-col items-center justify-center gap-[0.5rem]',
           'bg-emi-primary/20 hover:bg-emi-primary/30',
           'border border-transparent hover:border-emi-primary/50',
           'rounded-lg',
           'shadow-[0_0.25rem_0.75rem_rgba(79,209,197,0.25)]',
           'transition-all duration-300 group relative overflow-hidden']"
  :aria-expanded="isMobileMenuOpen"
>
  <!-- Hover glow overlay -->
  <div class="absolute inset-0 bg-gradient-to-r from-emi-primary/0 via-emi-primary/10 to-emi-primary/0
              opacity-0 group-hover:opacity-100 transition-opacity duration-500 blur-xl pointer-events-none"></div>

  <!-- Hamburger lines with transform animation -->
  <span :style="mobileMenuLine1Style" class="block w-5 h-[0.125rem] bg-slate-100 transition-transform duration-300 origin-center"></span>
  <span :style="mobileMenuLine2Style" class="block w-5 h-[0.125rem] bg-slate-100 transition-transform duration-300 origin-center"></span>
</button>
```

### 19.2 Hamburger → X Transform Logic
```typescript
const mobileMenuLine1Style = computed(() => ({
  transform: isMobileMenuOpen.value ? 'rotate(45deg) translateY(0.375rem)' : 'none',
}));

const mobileMenuLine2Style = computed(() => ({
  transform: isMobileMenuOpen.value ? 'rotate(-45deg) translateY(-0.375rem)' : 'none',
}));
```

**Key Values:**
- Rotation: ±45 degrees
- TranslateY: ±0.375rem (6px) to meet at center
- Duration: 300ms
- Origin: center

### 19.3 Mobile Menu Overlay
```vue
<div :class="['fixed inset-0 bg-emi-bg-dark/95 backdrop-blur-md z-50 transition-opacity duration-300',
              isMobileMenuOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none']">
  <nav class="flex flex-col items-center justify-center h-full px-6">
    <!-- Centered navigation links -->
  </nav>
</div>
```

**Pattern:** Body scroll lock with `overflow: hidden` on `body.mobile-menu-active`

---

## 20. Floating UI Dropdown with Laser Effect

### 20.1 Floating UI Setup (docs/AuthNav.vue:141-149)
```typescript
import { useFloating, offset, flip, shift, autoUpdate } from '@floating-ui/vue';

const { floatingStyles, update } = useFloating(buttonRef, dropdownRef, {
  placement: 'right-start',
  middleware: [
    offset(25),           // Gap from trigger
    flip(),               // Flip if overflows
    shift({ padding: 25 }) // Stay in viewport with padding
  ],
  whileElementsMounted: autoUpdate, // Keep position synced
});
```

### 20.2 Laser/Spotlight Mouse Effect (docs/AuthNav.vue:260-297)
```typescript
const handleMouseMove = (e: MouseEvent) => {
  if (!laserEffectRef.value || !dropdownRef.value) return;

  const rect = dropdownRef.value.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;

  const linkElement = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement;
  const linkTarget = linkElement?.tagName === 'A' ? linkElement : linkElement?.closest('a');

  if (linkTarget) {
    // Stronger glow when hovering a link
    laserEffectRef.value.style.background = `
      radial-gradient(
        circle at ${x}px ${y}px,
        rgba(79, 209, 197, 0.2) 0%,
        transparent 40%
      )
    `;
    laserEffectRef.value.style.opacity = '0.8';
  } else {
    // Subtle glow when not on a link
    laserEffectRef.value.style.background = `
      radial-gradient(
        circle at ${x}px ${y}px,
        rgba(79, 209, 197, 0.1) 0%,
        transparent 40%
      )
    `;
    laserEffectRef.value.style.opacity = '0.2';
  }
};

// Add/remove listener when menu opens/closes
document.addEventListener('mousemove', handleMouseMove);
document.removeEventListener('mousemove', handleMouseMove);
```

### 20.3 Laser Effect Layer HTML
```html
<div ref="laserEffectRef"
     class="absolute inset-0 pointer-events-none opacity-10 mix-blend-plus-lighter
            transform-gpu hover:opacity-20 transition-opacity duration-500">
</div>
```

### 20.4 Dropdown State Transitions (CSS)
```css
#forgestack-dropdown {
  position: fixed;
  transition: opacity 0.3s ease, transform 0.3s ease, box-shadow 0.3s ease;
  box-shadow: 0 0 1px 1px rgba(41, 171, 226, 0.1),
              0 0 1px 1px rgba(23, 32, 51, 0.5),
              0 0 5px 1px rgba(166, 109, 196, 0.1);
}

#forgestack-dropdown[data-state='closed'] {
  opacity: 0;
  transform: translateX(0.5rem);
  pointer-events: none;
}

#forgestack-dropdown[data-state='open'] {
  opacity: 1;
  transform: translateX(0);
  pointer-events: auto;
}
```

---

## 21. Nested Menu Structure

### 21.1 Menu Data Structure (docs/SidebarNav.vue:179-229)
```typescript
interface MenuItem {
  text: string;           // Category header text
  href: string;          // Parent link (usually '#')
  icon: Component;       // Lucide icon component
  children: {
    text: string;        // Child link text
    href: string;        // Route path
  }[];
}

const forgestackLinks: MenuItem[] = [
  {
    text: 'Menu',
    href: '#',
    icon: Home,
    children: [
      { text: 'Home: ForgeStack', href: '/' },
      { text: 'ChatFS: Business Ai', href: '/ai'},
      { text: 'Strategy: Fractional CTO', href: '/cto'},
      { text: 'Modules', href: '/modules' },
    ]
  },
  {
    text: 'Log-in',
    href: '#',
    icon: Lock,
    children: [
      { text: 'Log-in', href: '/forgestack.app/auth' }
    ]
  }
];
```

### 21.2 Nested Menu Template (docs/SidebarNav.vue:42-75)
```vue
<div v-for="(parentLink, index) in forgestackLinks" :key="parentLink.text" class="category-block">
  <!-- Separator between categories (not first) -->
  <hr v-if="index > 0" class="border-t border-white/10 border-solid my-1 mx-4" />

  <!-- Category Header with Icon -->
  <div v-if="parentLink.children?.length"
       class="flex items-center gap-2 px-[1rem] pt-[0.75rem] pb-[0.25rem]
              text-xs font-semibold text-white/60 uppercase tracking-wider">
    <component :is="parentLink.icon" class="w-4 h-4 opacity-70" />
    <span>{{ parentLink.text }}</span>
  </div>

  <!-- Child Links -->
  <template v-if="parentLink.children">
    <a v-for="childLink in parentLink.children"
       :key="childLink.href"
       :href="childLink.href"
       class="block px-[1rem] py-[0.5rem] text-white/80 hover:text-white text-left
              hover:bg-blue-400/5 transition-all duration-300
              relative group/link text-sm"
       @mousemove="handleMouseMove">
      <span class="relative z-10">{{ childLink.text }}</span>

      <!-- Animated underline on hover -->
      <span class="absolute bottom-0 left-1/2 right-1/2 h-px bg-white/80
                   group-hover/link:left-[1rem] group-hover/link:right-[1rem]
                   transition-all duration-300
                   shadow-[0_0.5rem_1.5rem_rgba(79,209,197,0.5)]">
      </span>
    </a>
  </template>
</div>
```

---

## 22. Footer Section Pattern in Sidebar

### 22.1 Full Footer (docs/SidebarNav.vue:81-140)
```vue
<div class="mt-auto px-0 space-y-3 flex-shrink-0 relative bottom-0 z-10 w-full">
  <!-- About Card -->
  <div class="px-2 py-3 rounded-lg bg-white/5 backdrop-blur-sm border border-white/10 border-solid mb-3">
    <h3 class="text-s font-semibold mb-2 text-forgestack-teal flex items-center">
      About:
    </h3>
    <p class="text-xs text-white/70 mb-3">
      ForgeStack is small business technology with human-centered design.
    </p>
    <p class="text-xs text-white/70 mb-1">
      Built by entrepreneurs, for entrepreneurs—with full data control.
    </p>
  </div>

  <!-- Contact Card -->
  <div class="px-2 py-2 rounded-lg flex flex-col items-center bg-white/5 backdrop-blur-sm border border-white/10 border-solid">
    <h3 class="text-s font-semibold mb-2 text-forgestack-teal">
      Let's Connect:
    </h3>
    <a href="mailto:info@forgestack.com" class="hover:text-forgestack-primary transition-colors text-xs">
      info@forgestack.com
    </a>

    <!-- Social Icons -->
    <div class="flex gap-3 px-1 mt-2">
      <a href="#" class="text-white/60 hover:text-forgestack-teal transition-colors">
        <!-- Twitter SVG -->
        <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24"
             fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 4s-.7 2.1-2 3.4c1.6 10-9.4 17.3-18 11.6 2.2.1 4.4-.6 6-2C3 15.5.5 9.6 3 5c2.2 2.6 5.6 4.1 9 4-.9-4.2 4-6.6 7-3.8 1.1 0 3-1.2 3-1.2z"/>
        </svg>
      </a>
      <!-- LinkedIn, GitHub SVGs... -->
    </div>
  </div>

  <!-- Legal Links -->
  <div class="px-1 pb-4">
    <div class="flex justify-center gap-4 text-xs mb-6">
      <a href="#" class="text-white/60 hover:text-forgestack-teal transition-colors">Privacy</a>
      <a href="#" class="text-white/60 hover:text-forgestack-teal transition-colors">Terms</a>
      <a href="#" class="text-white/60 hover:text-forgestack-teal transition-colors">Data</a>
    </div>
    <div class="flex justify-center text-xs text-white/50">
      {{ new Date().getFullYear() }} ForgeStack
    </div>
  </div>
</div>
```

### 22.2 Compact Footer (docs/AuthNav.vue:88-102)
```vue
<div class="mt-auto pt-4 px-2 pb-4 text-center">
  <div class="px-2 py-3 rounded-lg bg-slate-800/50 backdrop-blur-sm border border-[#4fd1c54d] border-solid">
    <h3 class="text-xs font-semibold mb-2 text-[#4FD1C5] uppercase tracking-wider">
      Hi, {{ user.name }}
    </h3>
    <div class="text-xs text-slate-300/70 space-y-1">
      <div class="flex justify-center gap-3">
        <router-link to="/s" class="hover:text-[#4FD1C5] transition-colors">Support</router-link>
        <span>&bull;</span>
        <a href="/terms" target="_blank" class="hover:text-[#4FD1C5] transition-colors">Terms</a>
      </div>
    </div>
  </div>
</div>
```

---

## 23. CTA Section with Gradient Backgrounds

### 23.1 Final CTA Pattern (docs/cto.vue:332-361)
```vue
<section class="w-full bg-slate-700 backdrop-blur-sm border-t border-white/5 border-solid py-12 rounded-t-3xl">
  <div class="container mx-auto max-w-8xl z-20 relative px-0
              bg-gradient-to-br from-[#a855f7]/30 via-[#06b6d4]/20 to-[#f59e42]/30
              dark:from-[#a855f7]/50 dark:via-[#06b6d4]/30 dark:to-[#f59e42]/50
              rounded-3xl">
    <div class="relative flex flex-col items-center bg-transparent backdrop-blur-xl p-0
                rounded-[1.5rem] border border-[#34D3EE]/20 shadow-lg overflow-hidden"
         style="box-shadow: 0 0 25px rgba(52, 211, 238, 0.1), 0 0 15px rgba(52, 211, 238, 0.05);">

      <!-- Decorative Background SVG -->
      <div class="absolute inset-0 opacity-[0.5] z-[-1]">
        <img src="/IMG/axisra-tech-animation copy.svg"
             alt="Decorative background"
             class="w-full h-full object-cover scale-[1.1] sm:scale-100"
             loading="lazy" />
      </div>

      <!-- CTA Content -->
      <div class="max-w-3xl mx-auto text-center relative z-10 py-4 md:py-8 px-4">
        <h2 class="text-4xl md:text-5xl font-bold text-slate-50 mb-8 leading-tight">
          Ready to Build a <span class="text-gradient-animated">Smarter, Simpler Business?</span>
        </h2>
        <p class="text-lg md:text-xl text-slate-300 mb-10 max-w-2xl mx-auto opacity-90">
          Stop wrestling with tech and start leveraging it.
        </p>
        <GlowPressButton href="mailto:contact@forgestack.com" size="lg" color="teal" class="px-10 py-4 text-lg">
          Schedule Your Free Strategy Call
          <ChevronRight class="ml-2 h-5 w-5"/>
        </GlowPressButton>
        <p class="text-sm text-slate-300 mt-6">No obligation, just a conversation about your future.</p>
      </div>
    </div>
  </div>
</section>
```

### 23.2 Animated Text Gradient
```css
.text-gradient-animated {
  background-size: 200% auto;
  background-image: linear-gradient(
    to right,
    theme('colors.teal.300') 0%,
    theme('colors.purple.300') 50%,
    theme('colors.coral.300') 100%
  );
  animation: shine 5s linear infinite;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

@keyframes shine {
  to { background-position: 200% center; }
}
```

---

## 24. SVG Decoration Patterns

### 24.1 Inline Social Icons (docs/SidebarNav.vue:110-118)
```html
<!-- Twitter -->
<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24"
     fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M22 4s-.7 2.1-2 3.4c1.6 10-9.4 17.3-18 11.6 2.2.1 4.4-.6 6-2C3 15.5.5 9.6 3 5c2.2 2.6 5.6 4.1 9 4-.9-4.2 4-6.6 7-3.8 1.1 0 3-1.2 3-1.2z"/>
</svg>

<!-- LinkedIn -->
<svg ...>
  <path d="M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-2-2 2 2 0 0 0-2 2v7h-4v-7a6 6 0 0 1 6-6z"/>
  <rect width="4" height="12" x="2" y="9"/>
  <circle cx="4" cy="4" r="2"/>
</svg>

<!-- GitHub -->
<svg ...>
  <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4"/>
  <path d="M9 18c-4.51 2-5-2-7-2"/>
</svg>
```

### 24.2 Decorative Background SVG (docs/cto.vue:79-81)
```html
<div class="absolute inset-0 opacity-5 pointer-events-none">
  <img src="/IMG/axisra-tech-animation.svg"
       alt="Tech Background"
       class="w-full h-full object-cover opacity-10 mix-blend-overlay"/>
</div>
```

**Mix-blend modes used:**
- `mix-blend-overlay` - Subtle texture
- `mix-blend-screen` - Lighter integration
- `mix-blend-plus-lighter` - Additive glow

### 24.3 Hero Spinning SVG (docs/cto.vue:19-36)
```html
<svg class="absolute inset-0 w-full h-full animate-spin-very-slow opacity-50 text-white/70"
     viewBox="0 0 64 64" fill="none">
  <defs>
    <clipPath id="heroSvgLaserClip">
      <rect width="100%" height="100%" rx="24" ry="24"/>
    </clipPath>
    <filter id="ctoHeroGlow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="1" result="coloredBlur"/>
      <feMerge>
        <feMergeNode in="coloredBlur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
  </defs>
  <g filter="url(#ctoHeroGlow)" clip-path="url(#heroSvgLaserClip)">
    <path d="M32 4 L32 60" stroke="currentColor" stroke-width="1" opacity="0.7"/>
    <path d="M4 32 L60 32" stroke="currentColor" stroke-width="1" opacity="0.7"/>
    <path d="M12 12 L52 52" stroke="currentColor" stroke-width="0.75" opacity="0.5"/>
    <path d="M12 52 L52 12" stroke="currentColor" stroke-width="0.75" opacity="0.5"/>
    <circle cx="32" cy="32" r="2.5" fill="currentColor" opacity="0.9"/>
    <circle cx="32" cy="32" r="5" stroke="currentColor" stroke-width="0.5" opacity="0.6"/>
  </g>
</svg>
```

```css
@keyframes spin-very-slow { to { transform: rotate(360deg); } }
.animate-spin-very-slow { animation: spin-very-slow 60s linear infinite; }
```

---

## 25. Intersection Observer Pattern

### 25.1 Scroll-Triggered Fade-In (docs/cto.vue:372-380)
```typescript
onMounted(() => {
  const observerOptions = {
    root: null,           // Viewport as root
    rootMargin: "0px",
    threshold: 0.1         // Trigger at 10% visibility
  };

  const observer = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) {
        entry.target.classList.add("visible");
      }
    });
  }, observerOptions);

  document.querySelectorAll('.fade-in-late').forEach(el => observer.observe(el));
});
```

### 25.2 CSS for Fade-In Animation
```css
.fade-in-late {
  opacity: 0;
  transform: translateY(25px);
  transition: opacity 0.7s ease-out, transform 0.7s ease-out;
}
.fade-in-late.visible {
  opacity: 1;
  transform: translateY(0);
}
```

---

## 26. markRaw() Icon Optimization

### 26.1 Pattern (docs/Journey.vue:1-46)
```typescript
import { ref, markRaw } from 'vue';
import {
  Sun as SunIcon,
  Clock as ClockIcon,
  Sparkles as SparklesIcon,
  // ... more icons
} from 'lucide-vue-next';

// Mark icons as raw to prevent Vue reactivity overhead
const Sun = markRaw(SunIcon);
const Clock = markRaw(ClockIcon);
const Sparkles = markRaw(SparklesIcon);
// ... use in template
```

**Why markRaw()?**
- Lucide icons are stateless components
- Vue's reactivity system adds overhead for tracking
- `markRaw()` tells Vue to skip making them reactive
- Improves performance with many icon instances

### 26.2 Usage in Data Structures
```typescript
interface TimelineItem {
  icon: any;  // markRaw'd component
  iconColorClass: string;
  // ...
}

const timelineItems: TimelineItem[] = [
  {
    icon: Sun,  // Already marked raw
    iconColorClass: "text-[hsl(var(--coral-400))]",
    // ...
  },
];
```

---

## 27. Additional Animation Patterns

### 27.1 Hover Lift Effect (docs/cto.vue:493-499)
```css
.hover-lift {
  transition: transform 0.3s ease-out, box-shadow 0.3s ease-out;
}
.hover-lift:hover {
  transform: translateY(-8px);
  box-shadow: 0 15px 30px hsla(var(--slate-900), 0.2),
              0 8px 15px hsla(var(--slate-900), 0.15);
}
```

### 27.2 Hero Float Animation (docs/cto.vue:448-453)
```css
@keyframes float-hero {
  0%, 100% { transform: translateY(0px); }
  50% { transform: translateY(-15px); }
}
.animate-float-hero { animation: float-hero 7s ease-in-out infinite; }
```

### 27.3 Pulse Orb Animation (docs/cto.vue:455-459)
```css
@keyframes pulse-orb {
  0%, 100% { transform: scale(1); opacity: 0.7; }
  50% { transform: scale(1.08); opacity: 1; }
}
.animate-pulse-orb { animation: pulse-orb 3s ease-in-out infinite; }
```

### 27.4 Subtle Float for Background Elements
```css
@keyframes float-subtle {
  0%, 100% { transform: translateY(0px) translateX(0px) rotate(0deg); }
  25% { transform: translateY(-10px) translateX(5px) rotate(15deg); }
  50% { transform: translateY(5px) translateX(-10px) rotate(-10deg); }
  75% { transform: translateY(-5px) translateX(10px) rotate(5deg); }
}
.animate-float-subtle { animation: float-subtle 15s ease-in-out infinite; }
```

---

## 28. Link Hover Effects

### 28.1 Expanding Underline (docs/SidebarNav.vue:67-72)
```html
<span class="absolute bottom-0 left-1/2 right-1/2 h-px bg-white/80
             group-hover/link:left-[1rem] group-hover/link:right-[1rem]
             transition-all duration-300
             shadow-[0_0.5rem_1.5rem_rgba(79,209,197,0.5)]
             after:absolute after:inset-0 after:bg-white/20 after:blur-sm
             after:opacity-0 after:group-hover/link:opacity-100 after:transition-opacity">
</span>
```

**Pattern:**
- Start: `left-1/2 right-1/2` (0 width, centered)
- Hover: `left-[1rem] right-[1rem]` (expands to full width with padding)
- Shadow creates glow effect
- Pseudo-element adds blur overlay

---

## Summary: Complete Pattern Coverage

| Pattern | Files | Section |
|---------|-------|---------|
| **Mobile Menu (hamburger→X)** | AuthLayout.vue | §19 |
| **Floating UI Dropdown** | AuthNav.vue, SidebarNav.vue | §20 |
| **Laser/Spotlight Effect** | AuthNav.vue, SidebarNav.vue | §20.2 |
| **Nested Menu Structure** | AuthNav.vue, SidebarNav.vue | §21 |
| **Footer Section** | AuthNav.vue, SidebarNav.vue | §22 |
| **CTA Gradient Section** | cto.vue | §23 |
| **SVG Decorations** | cto.vue, SidebarNav.vue | §24 |
| **Intersection Observer** | cto.vue | §25 |
| **markRaw() Optimization** | Journey.vue | §26 |
| **Animation Patterns** | cto.vue | §27 |
| **Link Hover Effects** | SidebarNav.vue | §28 |

All critical patterns from the 44 Vue files are now documented for pack-editor integration.
