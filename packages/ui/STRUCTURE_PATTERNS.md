# CALIBER UI - Structural Patterns Reference

> Extracted from 44 Vue reference files for AI-friendly reproduction in Svelte

---

## 1. Z-Index Layer System

The design system uses a consistent z-index stack for layering effects:

```
┌─────────────────────────────────────────────────────────────────┐
│  z-[10+]  │ Content, text, interactive elements                │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[7]    │ Top glassmorphic gradient (active state only)      │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[6]    │ Blob/lava animation layer + bottom glassmorphic    │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[5]    │ Bottom border highlight (hover state)              │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[4]    │ Subtle base glow (fades on hover)                  │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[3]    │ Secondary bevel/highlight                          │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[2]    │ Top bevel effect                                   │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[1]    │ Dark overlay / surface tint                        │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[0]    │ Default layer (most content)                       │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[-1]   │ Outer glow effects (pulsing)                       │
├───────────┼─────────────────────────────────────────────────────┤
│  z-[-2]   │ Background gradients, decorative blurs             │
└───────────┴─────────────────────────────────────────────────────┘
```

---

## 2. Button Layer Stack (GlowBrandButton Pattern)

```svelte
<button class="group relative overflow-hidden">
  <!-- Layer -1: Outer glow (pulsing) -->
  <div class="absolute -inset-px rounded-lg opacity-50 z-[-1]
              bg-[radial-gradient(circle,var(--glow-color),transparent_70%)]
              animate-pulse-glow"></div>

  <!-- Layer 1: Dark overlay -->
  <div class="absolute inset-0 bg-surface/20 rounded-lg z-[1]"></div>

  <!-- Layer 2: Top bevel highlight -->
  <div class="absolute inset-x-0 top-0 h-px bg-white/30 rounded-t-lg z-[2]"></div>

  <!-- Layer 3: Secondary bevel gradient -->
  <div class="absolute inset-x-0 top-0 h-px bg-gradient-brand z-[3]"></div>

  <!-- Layer 4: Subtle base glow (fades on hover) -->
  <div class="absolute inset-0 bg-[color]/5 opacity-100
              group-hover:opacity-0 transition-opacity z-[4]"></div>

  <!-- Layer 5: Bottom border highlight (appears on hover) -->
  <div class="absolute inset-x-0 bottom-0 h-0 opacity-0
              group-hover:opacity-100 group-hover:h-1 z-[5]"></div>

  <!-- Layer 6: Blob/lava animation -->
  <div class="absolute inset-0 opacity-0 group-hover:opacity-85 z-[6] overflow-hidden">
    <div class="absolute inset-0 animate-blob-move opacity-70"></div>
  </div>

  <!-- Layer 6: Bottom glassmorphic gradient -->
  <div class="absolute bottom-0 inset-x-0 h-0 glassmorphic-gradient
              opacity-0 group-hover:opacity-100 group-hover:h-full z-[6]"></div>

  <!-- Layer 7: Top glassmorphic (active state) -->
  <div class="absolute top-0 inset-x-0 h-0 glassmorphic-gradient-reverse
              opacity-0 group-active:opacity-100 group-active:h-[35%] z-[7] blur-sm"></div>

  <!-- Layer 10: Content -->
  <span class="relative z-10 flex items-center gap-2
               group-active:translate-y-1 transition-transform">
    <slot />
  </span>
</button>
```

---

## 3. Section/Page Layout Pattern

```svelte
<section class="relative py-24 overflow-hidden bg-transparent">
  <!-- Background layer: Decorative gradients -->
  <div class="absolute inset-0 bg-gradient-to-br
              from-purple-500/20 via-teal-500/15 to-coral-500/20 z-[-2]"></div>

  <!-- Background layer: Floating blur blobs -->
  <div class="absolute top-1/4 left-10 w-24 h-24
              bg-mint-500 rounded-full opacity-20 blur-2xl"></div>
  <div class="absolute bottom-1/4 right-10 w-32 h-32
              bg-coral-500 rounded-full opacity-20 blur-2xl"></div>

  <!-- Optional: Animated vertical connector line -->
  <div class="absolute left-1/2 top-0 bottom-0 w-1 -translate-x-1/2 z-0">
    <div class="w-full h-full bg-gradient-to-b
                from-teal-500 via-lavender-500 to-coral-500"></div>
  </div>

  <!-- Main content container -->
  <div class="container relative z-10 mx-auto px-4">
    <!-- Section header -->
    <div class="max-w-3xl mx-auto mb-20 text-center">
      <Badge ... />
      <h2>...</h2>
      <p>...</p>
    </div>

    <!-- Content grid -->
    <div class="flex flex-col md:flex-row gap-16 max-w-6xl mx-auto">
      <Card>...</Card>
      <Card>...</Card>
    </div>
  </div>
</section>
```

---

## 4. Card Layer Stack (GlassMorphicCard Pattern)

```svelte
<div class="relative group cursor-pointer overflow-hidden rounded-xl">
  <!-- Click capture layer (if needed) -->
  <div class="absolute inset-0 z-20" onclick={handleClick}></div>

  <!-- Hover glow effect (extends beyond card) -->
  <div class="absolute inset-0 -m-10 opacity-0 group-hover:opacity-100
              transition-opacity duration-500 bg-teal-200 blur-xl rounded-xl"></div>

  <!-- Top accent bar -->
  <div class="absolute top-0 left-0 w-full h-1
              bg-gradient-to-r from-teal-400 to-teal-600"></div>

  <!-- Card content -->
  <div class="p-8 text-center h-full flex flex-col">
    <!-- Icon with pulsing background -->
    <div class="relative inline-block mx-auto">
      <div class="absolute inset-0 bg-teal-100 rounded-full
                  opacity-30 animate-pulse"></div>
      <div class="relative z-10 flex items-center justify-center
                  w-20 h-20 bg-gradient-to-br from-teal-500 to-teal-700
                  rounded-full shadow-lg group-hover:scale-110
                  transition-transform duration-500">
        <Icon class="h-10 w-10 text-white" />
      </div>
    </div>

    <!-- Text content -->
    <h3>...</h3>
    <p class="flex-grow">...</p>

    <!-- Feature list -->
    <div class="mb-8 space-y-2 text-left">
      <div class="flex items-start space-x-2">
        <BoltIcon class="h-5 w-5 text-teal-500 mt-0.5 flex-shrink-0" />
        <span>Feature text</span>
      </div>
    </div>

    <!-- CTA Button -->
    <Button>...</Button>
  </div>
</div>
```

---

## 5. SVG Pattern Library

### 5.1 Inline Icon SVG (Lucide-style)
```svelte
<svg
  xmlns="http://www.w3.org/2000/svg"
  width="24"
  height="24"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  class="h-5 w-5"
>
  <path d="M22 4s-.7 2.1-2 3.4c1.6 10-9.4 17.3-18 11.6..." />
</svg>
```

### 5.2 Decorative SVG with Filters & Gradients
```svelte
<svg
  class="absolute inset-0 w-full h-full animate-spin-very-slow opacity-50"
  viewBox="0 0 64 64"
  fill="none"
>
  <defs>
    <!-- Clipping mask -->
    <clipPath id="clip">
      <rect width="100%" height="100%" rx="24" ry="24"/>
    </clipPath>

    <!-- Glow filter -->
    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="2" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>

    <!-- Gradient -->
    <linearGradient id="grad" x1="0" y1="0" x2="100%" y2="0">
      <stop offset="0%" stop-color="hsl(var(--teal-500))" stop-opacity="0.4"/>
      <stop offset="50%" stop-color="hsl(var(--coral-500))" stop-opacity="0.8"/>
      <stop offset="100%" stop-color="hsl(var(--teal-500))" stop-opacity="0.4"/>
    </linearGradient>
  </defs>

  <g filter="url(#glow)" clip-path="url(#clip)">
    <!-- Cross pattern -->
    <path d="M32 4 L32 60" stroke="currentColor" stroke-width="1" opacity="0.7"/>
    <path d="M4 32 L60 32" stroke="currentColor" stroke-width="1" opacity="0.7"/>
    <path d="M12 12 L52 52" stroke="currentColor" stroke-width="0.75" opacity="0.5"/>
    <path d="M12 52 L52 12" stroke="currentColor" stroke-width="0.75" opacity="0.5"/>

    <!-- Center dot -->
    <circle cx="32" cy="32" r="2.5" fill="currentColor" opacity="0.9"/>
    <circle cx="32" cy="32" r="5" stroke="currentColor" stroke-width="0.5" opacity="0.6"/>
  </g>
</svg>
```

### 5.3 Decorative Line SVG (for Hero sections)
```svelte
<svg
  class="absolute inset-0 w-full h-full pointer-events-none"
  viewBox="0 0 200 200"
  fill="none"
>
  <!-- Concentric circles -->
  <circle cx="100" cy="100" r="40" stroke="currentColor" stroke-width="0.5" opacity="0.3"/>
  <circle cx="100" cy="100" r="60" stroke="currentColor" stroke-width="0.5" opacity="0.2"/>
  <circle cx="100" cy="100" r="80" stroke="currentColor" stroke-width="0.5" opacity="0.1"/>
</svg>
```

---

## 6. Icon Import Pattern (from lucide)

```typescript
// Vue pattern (for reference):
import {
  Brain,
  Bolt,
  ArrowRight,
  Sparkles,
  Menu,
  ChevronRight
} from 'lucide-vue-next';

// Svelte equivalent (using lucide-svelte):
import {
  Brain,
  Bolt,
  ArrowRight,
  Sparkles,
  Menu,
  ChevronRight
} from 'lucide-svelte';

// Usage in template:
<Brain class="h-10 w-10 text-white" />
<Bolt class="h-5 w-5 text-teal-500 mt-0.5 flex-shrink-0" size={20} />
```

---

## 7. Glassmorphic Effects CSS

```css
/* Glassmorphic gradient (bottom-up) */
.glassmorphic-gradient {
  background: linear-gradient(
    to top,
    rgba(15, 23, 42, 0.4) 0%,
    rgba(15, 23, 42, 0.2) 40%,
    rgba(15, 23, 42, 0.05) 100%
  );
  backdrop-filter: blur(3px);
}

/* Glassmorphic gradient (top-down, for active state) */
.glassmorphic-gradient-reverse {
  background: linear-gradient(
    to bottom,
    rgba(15, 23, 42, 0.5) 0%,
    rgba(15, 23, 42, 0.25) 40%,
    rgba(15, 23, 42, 0.1) 100%
  );
  backdrop-filter: blur(3px);
}

/* Outer glow effect */
.outer-glow-effect {
  background: radial-gradient(
    circle at center,
    var(--glow-color) 0%,
    transparent 70%
  );
  animation: pulse-glow 4s ease-in-out infinite;
}

/* Frosted glass panel */
.glass-panel {
  background: rgba(15, 23, 42, 0.7);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.1);
}
```

---

## 8. Animation Keyframes

```css
/* Blob/lava lamp morphing */
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
  75% {
    border-radius: 40% 30% 50% 60% / 30% 40% 60% 50%;
    transform: translate(-10px, -10px) scale(1.1);
  }
}

/* Pulsing glow */
@keyframes pulse-glow {
  0%, 100% { opacity: 0.4; filter: blur(12px); }
  50% { opacity: 0.7; filter: blur(18px); }
}

/* Gradient flow */
@keyframes gradient-flow {
  0%, 100% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
}

/* Float hero */
@keyframes float-hero {
  0%, 100% { transform: translateY(0) rotate(0deg); }
  25% { transform: translateY(-5px) rotate(1deg); }
  50% { transform: translateY(-10px) rotate(0deg); }
  75% { transform: translateY(-5px) rotate(-1deg); }
}

/* Slow spin */
@keyframes spin-very-slow {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
```

---

## 9. Tailwind Class Patterns

### Common Combinations

```
/* Floating blur blob */
"absolute top-1/4 left-10 w-24 h-24 bg-teal-500 rounded-full opacity-20 blur-2xl"

/* Gradient background */
"bg-gradient-to-br from-purple-500/20 via-teal-500/15 to-coral-500/20"

/* Glassmorphic container */
"bg-slate-800/70 backdrop-blur-xl rounded-xl border border-slate-600/50"

/* Interactive card */
"relative group cursor-pointer overflow-hidden rounded-xl transition-all duration-300"

/* Hover glow */
"opacity-0 group-hover:opacity-100 transition-opacity duration-500"

/* Press effect */
"group-active:translate-y-1 transition-transform duration-300"

/* Icon container with pulse */
"relative inline-block" + child: "absolute inset-0 bg-teal-100 rounded-full opacity-30 animate-pulse"

/* Content centering */
"flex items-center justify-center"

/* Section container */
"container relative z-10 mx-auto px-4"
```

---

## 10. Component Composition Hierarchy

```
Page/Route
├── <section> (relative, py-24, overflow-hidden)
│   ├── Background decorations (absolute, z-[-2])
│   │   ├── Gradient overlays
│   │   └── Blur blobs
│   │
│   ├── Animated connectors (absolute, z-0)
│   │   └── Gradient lines
│   │
│   └── Content container (relative, z-10)
│       ├── Header (max-w-3xl, text-center)
│       │   ├── Badge
│       │   ├── Heading
│       │   └── Description
│       │
│       └── Content grid (flex, gap-16)
│           └── Cards (relative, group)
│               ├── Click layer (absolute, z-20)
│               ├── Hover glow (absolute, -m-10, blur-xl)
│               ├── Top accent (absolute, h-1)
│               └── Content (p-8, flex-col)
│                   ├── Icon block
│                   │   ├── Pulse bg (absolute)
│                   │   └── Icon (relative, z-10)
│                   ├── Text content
│                   ├── Feature list
│                   └── CTA Button
```

---

## 11. CSS Variable References

```css
:root {
  /* Brand colors (HSL values without hsl()) */
  --teal-500: 175 70% 40%;
  --coral-500: 15 85% 50%;
  --mint-500: 165 70% 45%;
  --lavender-500: 265 70% 55%;
  --purple-500: 270 70% 60%;

  /* Surface colors */
  --slate-800: 222 18% 20%;
  --slate-900: 225 20% 10%;
  --navy-800: 220 30% 15%;
  --navy-900: 225 35% 10%;

  /* Brand gradient stops */
  --brand-1: hsl(var(--coral-500));
  --brand-2: hsl(var(--mint-500));
  --brand-3: hsl(var(--teal-500));
  --brand-4: hsl(var(--lavender-500));
}

/* Usage */
.element {
  background: hsl(var(--teal-500));
  border-color: hsl(var(--teal-500) / 0.3);
  box-shadow: 0 0 20px hsl(var(--teal-500) / 0.25);
}
```

---

This document serves as the structural DNA for reproducing the Vue design patterns in Svelte.
