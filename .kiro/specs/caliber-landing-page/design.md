# Design Document: CALIBER Landing Page

## Overview

A single-page marketing site for CALIBER built with Astro + Svelte, featuring the "SynthBrute" aesthetic — a fusion of Neo-Brutalist structure with Synthwave/Vaporwave visuals and LiquidGlass effects. The centerpiece is an animated Memory Hierarchy Visualization that serves as both functional documentation and artistic expression.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Astro Static Site                        │
├─────────────────────────────────────────────────────────────┤
│  Layout.astro (base HTML, fonts, meta)                      │
│       │                                                     │
│  index.astro (page composition)                             │
│       │                                                     │
│  ┌────┴────────────────────────────────────────────┐       │
│  │ Sections (Astro components, zero JS)            │       │
│  │  - Nav.astro                                    │       │
│  │  - Hero.astro (contains Svelte island)          │       │
│  │  - Problems.astro                               │       │
│  │  - Solutions.astro                              │       │
│  │  - Architecture.astro                           │       │
│  │  - Pricing.astro                                │       │
│  │  - Footer.astro                                 │       │
│  └─────────────────────────────────────────────────┘       │
│       │                                                     │
│  ┌────┴────────────────────────────────────────────┐       │
│  │ Svelte Islands (hydrated, interactive)          │       │
│  │  - MemoryHierarchy.svelte (hero visualization)  │       │
│  │  - MobileNav.svelte (hamburger menu)            │       │
│  │  - CodeBlock.svelte (syntax highlighting)       │       │
│  └─────────────────────────────────────────────────┘       │
│       │                                                     │
│  TailwindCSS + Custom CSS (SynthBrute design system)       │
│  MotionOne (animations)                                     │
└─────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### Page Structure

```
/
├── src/
│   ├── layouts/
│   │   └── Layout.astro          # Base HTML, fonts, global styles
│   ├── pages/
│   │   └── index.astro           # Main landing page
│   ├── components/
│   │   ├── Nav.astro             # Fixed navigation bar
│   │   ├── Hero.astro            # Hero section wrapper
│   │   ├── Problems.astro        # Problem cards section
│   │   ├── Solutions.astro       # Solution/features section
│   │   ├── Architecture.astro    # Architecture diagram section
│   │   ├── Pricing.astro         # Pricing cards section
│   │   ├── Footer.astro          # Footer with links
│   │   └── svelte/
│   │       ├── MemoryHierarchy.svelte  # Animated visualization
│   │       ├── MobileNav.svelte        # Mobile hamburger menu
│   │       └── CodeBlock.svelte        # Syntax highlighted code
│   ├── styles/
│   │   ├── global.css            # Base styles, fonts
│   │   └── synthbrute.css        # Design system tokens
│   └── content/
│       └── pricing.json          # Pricing data (single source of truth)
├── public/
│   └── fonts/                    # Self-hosted fonts
├── astro.config.mjs
├── tailwind.config.mjs
├── package.json
└── vercel.json
```

### Memory Hierarchy Visualization Component

The centerpiece Svelte component that visualizes:

```
Trajectory (task container)
├── Scope (context partition)
│   ├── Turn (ephemeral buffer)
│   └── Artifact (preserved output)
└── Note (cross-trajectory knowledge)
```

**Behavior:**
- On load: Nodes fade in sequentially with spring animation
- On scroll: Parallax depth effect, nodes shift at different rates
- On hover: Node expands with glass panel showing description
- Connections: Animated lines with neon glow pulse

**Implementation:**
```svelte
<script>
  import { spring } from 'svelte/motion';
  import { inview } from 'svelte-inview';
  
  const nodes = [
    { id: 'trajectory', label: 'Trajectory', depth: 0 },
    { id: 'scope', label: 'Scope', depth: 1 },
    { id: 'turn', label: 'Turn', depth: 2 },
    { id: 'artifact', label: 'Artifact', depth: 2 },
    { id: 'note', label: 'Note', depth: 1 },
  ];
</script>
```

## Data Models

### Pricing Data (pricing.json)

```json
{
  "storage": {
    "monthly": { "amount": 1, "unit": "GB", "price": 1 },
    "annual": { "amount": 1, "unit": "GB", "price": 10, "savings": "2 months free" }
  },
  "hotCache": {
    "monthly": { "amount": 1, "unit": "MB", "price": 0.15 }
  },
  "agents": "unlimited",
  "trial": {
    "days": 14,
    "creditCard": false
  }
}
```

### Problem Cards Data

```typescript
interface ProblemCard {
  icon: string;
  title: string;
  description: string;
  solution: string;
}

const problems: ProblemCard[] = [
  {
    icon: "🧠",
    title: "Context Amnesia",
    description: "Agents lose context between sessions.",
    solution: "Hierarchical memory: Trajectory → Scope → Artifact → Note"
  },
  // ... 5 more
];
```

## Design System: SynthBrute

### Color Palette

```css
:root {
  /* Base (Brutalist) */
  --bg-primary: #0a0a0b;
  --bg-secondary: #111113;
  --bg-card: #18181b;
  --border: #27272a;
  
  /* Text */
  --text-primary: #fafafa;
  --text-secondary: #a1a1aa;
  --text-muted: #71717a;
  
  /* Synthwave (muted, digestible) */
  --neon-pink: #ec4899;
  --neon-purple: #a855f7;
  --neon-cyan: #22d3ee;
  
  /* Industrial Rust */
  --rust-primary: #b45309;
  --rust-secondary: #92400e;
  --rust-accent: #f59e0b;
  
  /* Glass */
  --glass-bg: rgba(24, 24, 27, 0.7);
  --glass-border: rgba(255, 255, 255, 0.1);
}
```

### Typography

```css
/* Titles: Brutalist grotesque */
font-family: 'Space Grotesk', system-ui, sans-serif;
font-weight: 700;
letter-spacing: -0.02em;

/* Body: Clean, readable */
font-family: 'Inter', system-ui, sans-serif;
font-weight: 400;

/* Code: Monospace */
font-family: 'JetBrains Mono', monospace;
```

### Brutalist Structure

```css
/* Hard edges */
border-radius: 0px; /* or 2px max */

/* Visible grid */
.brutalist-grid {
  display: grid;
  gap: 2px;
  background: var(--border);
}

/* High contrast borders */
border: 2px solid var(--border);
```

### Glass Panels (that bleed)

```css
.glass-panel {
  background: var(--glass-bg);
  backdrop-filter: blur(12px);
  border: 1px solid var(--glass-border);
  
  /* Intentional bleed outside container */
  margin: -8px;
  padding: calc(1rem + 8px);
}

.glass-glow {
  box-shadow: 
    0 0 20px rgba(236, 72, 153, 0.3),
    0 0 40px rgba(168, 85, 247, 0.2);
}
```

### Animation Patterns

```css
/* Brutalist: snap-in, no easing */
.brutalist-enter {
  animation: snap-in 0.1s steps(1);
}

/* Glass: spring motion */
.glass-enter {
  animation: spring-in 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
}

/* Neon pulse */
@keyframes neon-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do.*

Based on the prework analysis, most acceptance criteria for this landing page are visual/design requirements that cannot be property-tested. However, we can define one key property:

### Property 1: Responsive Layout Integrity

*For any* viewport width between 320px and 2560px, the landing page SHALL render without horizontal scrollbar overflow.

**Validates: Requirements 8.1**

This property ensures the responsive design works across all device sizes without breaking the layout.

## Error Handling

### Build-Time Errors

- Missing pricing.json → Build fails with clear error message
- Invalid Svelte component → Astro build error with component path
- Missing fonts → Fallback to system fonts, console warning

### Runtime Errors

- Animation library fails to load → Graceful degradation to static content
- Svelte hydration fails → Static HTML remains visible

## Testing Strategy

### Unit Tests (Vitest)

- Pricing data validation (correct structure, positive numbers)
- Problem/solution data completeness (6 items each)
- Navigation link validity

### Integration Tests (Playwright)

- Page loads without errors
- All sections render
- Navigation links work
- Mobile menu toggles
- Pricing displays correct values
- Footer links are valid

### Visual Regression (optional)

- Percy or Chromatic for design consistency
- Snapshot key sections at multiple breakpoints

### Performance Testing

- Lighthouse CI in GitHub Actions
- Target: 90+ performance score
- Core Web Vitals monitoring

### Property-Based Test

```typescript
// Property 1: Responsive layout integrity
import { test, expect } from '@playwright/test';
import fc from 'fast-check';

test('responsive layout has no horizontal overflow', async ({ page }) => {
  await fc.assert(
    fc.asyncProperty(
      fc.integer({ min: 320, max: 2560 }),
      async (viewportWidth) => {
        await page.setViewportSize({ width: viewportWidth, height: 800 });
        await page.goto('/');
        
        const hasHorizontalScroll = await page.evaluate(() => {
          return document.documentElement.scrollWidth > document.documentElement.clientWidth;
        });
        
        expect(hasHorizontalScroll).toBe(false);
      }
    ),
    { numRuns: 50 }
  );
});
```
