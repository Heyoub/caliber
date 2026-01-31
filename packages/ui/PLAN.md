# UI Package Build Plan

> 6 parallel agents, no compile until done, maximum type theory magic

## Agent Assignments

1. **Agent: Content CMS** - YAML content system + asset registry
2. **Agent: Type System** - Struct-like types, branded IDs, discriminated unions
3. **Agent: Atoms** - Port all atomic components from Vue
4. **Agent: Molecules** - Port all molecule components
5. **Agent: Organisms** - Port all organism components
6. **Agent: App Shell** - SvelteKit routes, stores, layouts

## Type Theory Magic

### 1. Branded Types (Nominal Typing)
```typescript
// Prevent mixing up IDs
declare const __brand: unique symbol;
type Brand<T, B> = T & { [__brand]: B };

type TrajectoryId = Brand<string, 'TrajectoryId'>;
type ScopeId = Brand<string, 'ScopeId'>;
type TurnId = Brand<string, 'TurnId'>;

// Can't accidentally pass ScopeId where TrajectoryId expected
function getTrajectory(id: TrajectoryId) { ... }
getTrajectory(scopeId); // ❌ Type error!
```

### 2. Discriminated Unions (Tagged Unions like Rust enums)
```typescript
// Component variants as exhaustive union
type ButtonVariant =
  | { kind: 'solid'; color: ColorToken }
  | { kind: 'outline'; color: ColorToken; borderWidth?: 1 | 2 }
  | { kind: 'ghost'; hoverColor?: ColorToken }
  | { kind: 'link'; underline?: boolean };

// MCP content blocks
type ContentBlock =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mimeType: `image/${string}` }
  | { type: 'resource'; uri: `memory://${string}`; mimeType: string }
  | { type: 'tool_use'; id: string; name: string; input: unknown }
  | { type: 'tool_result'; tool_use_id: string; content: ContentBlock[] };
```

### 3. Template Literal Types (CSS class generation)
```typescript
// Generate valid class names at compile time
type SpacingValue = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 8 | 10 | 12 | 16 | 20 | 24;
type SpacingClass = `${'p' | 'm' | 'gap'}-${SpacingValue}`;
type DirectionalSpacing = `${'p' | 'm'}${'t' | 'r' | 'b' | 'l' | 'x' | 'y'}-${SpacingValue}`;

// Only valid color classes
type ColorClass = `${ColorPalette}-${ColorIntensity}`;
type TextColorClass = `text-${ColorClass}`;
type BgColorClass = `bg-${ColorClass}`;

// Glow classes
type GlowClass = `glow-${ColorPalette}` | `glow-${ColorPalette}-${'subtle' | 'medium' | 'intense' | 'pulse'}`;
```

### 4. Const Assertions + Exhaustive Checking
```typescript
const SIZES = ['xs', 'sm', 'md', 'lg', 'xl', '2xl'] as const;
type Size = typeof SIZES[number];

// Exhaustive switch helper
function assertNever(x: never): never {
  throw new Error(`Unexpected value: ${x}`);
}

function getSizeClass(size: Size): string {
  switch (size) {
    case 'xs': return 'text-xs px-2 py-1';
    case 'sm': return 'text-sm px-3 py-1.5';
    case 'md': return 'text-base px-4 py-2';
    case 'lg': return 'text-lg px-5 py-2.5';
    case 'xl': return 'text-xl px-6 py-3';
    case '2xl': return 'text-2xl px-8 py-4';
    default: return assertNever(size); // Compile error if case missing!
  }
}
```

### 5. Builder Pattern with Method Chaining
```typescript
// Fluent API for complex component config
const button = Button.create()
  .color('teal')
  .size('lg')
  .glow('pulse')
  .hover('lift')
  .when(isLoading, b => b.loading())
  .when(hasError, b => b.color('coral').glow('intense'))
  .build();
```

### 6. Phantom Types (Compile-time state tracking)
```typescript
// Track component state at compile time
type Unvalidated = { readonly _validated: false };
type Validated = { readonly _validated: true };

interface Form<State> {
  values: Record<string, unknown>;
  validate(): Form<Validated>;
  submit(): State extends Validated ? Promise<void> : never;
}

const form: Form<Unvalidated> = createForm();
form.submit(); // ❌ Type error - must validate first!
form.validate().submit(); // ✓ OK
```

### 7. Struct-like Svelte (C vibes)
```typescript
// Define components like C structs
interface ButtonStruct {
  readonly tag: 'Button';
  readonly color: ColorToken;
  readonly size: Size;
  readonly glow: GlowEffect;
  readonly state: {
    readonly pressed: boolean;
    readonly hovered: boolean;
  };
}

// Factory function (like malloc + init)
function Button(init: Partial<ButtonStruct>): ButtonStruct {
  return {
    tag: 'Button',
    color: init.color ?? 'teal',
    size: init.size ?? 'md',
    glow: init.glow ?? false,
    state: { pressed: false, hovered: false },
  };
}

// In Svelte - use $state with struct
let btn = $state(Button({ color: 'coral', size: 'lg' }));
```

### 8. Opaque Type Pattern
```typescript
// Hide implementation details
declare const opaqueTag: unique symbol;

interface CSSValue<T extends string> {
  readonly [opaqueTag]: T;
  readonly value: string;
}

type Pixels = CSSValue<'px'>;
type Rem = CSSValue<'rem'>;
type Percent = CSSValue<'%'>;

function px(n: number): Pixels {
  return { value: `${n}px` } as Pixels;
}

function rem(n: number): Rem {
  return { value: `${n}rem` } as Rem;
}

// Can't mix units!
function setWidth(w: Pixels | Rem) { ... }
setWidth(px(100)); // ✓
setWidth(rem(6)); // ✓
setWidth(50); // ❌ Must use px() or rem()
```

### 9. Mapped Types for Modifier Classes
```typescript
// Generate all modifier classes from type definitions
type ModifierMap<T extends Record<string, readonly string[]>> = {
  [K in keyof T]: T[K][number];
};

const MODIFIERS = {
  size: ['xs', 'sm', 'md', 'lg', 'xl'] as const,
  color: ['teal', 'coral', 'purple', 'pink', 'mint', 'amber'] as const,
  glow: ['none', 'subtle', 'medium', 'intense', 'pulse'] as const,
} as const;

type Modifiers = ModifierMap<typeof MODIFIERS>;
// { size: 'xs' | 'sm' | ... ; color: 'teal' | ... ; glow: 'none' | ... }
```

### 10. Conditional Types for Component Variants
```typescript
// Different props based on variant
type ButtonProps<V extends ButtonVariant['kind']> =
  V extends 'solid' ? { color: ColorToken; glow?: GlowEffect } :
  V extends 'outline' ? { color: ColorToken; borderWidth?: 1 | 2 } :
  V extends 'ghost' ? { hoverColor?: ColorToken } :
  V extends 'link' ? { underline?: boolean } :
  never;

// Usage
interface Button<V extends ButtonVariant['kind'] = 'solid'> {
  variant: V;
  props: ButtonProps<V>;
}
```

## CMS Architecture

### Content YAML Structure
```yaml
# content/en/landing.yaml
hero:
  title: "AI agents forget everything"
  subtitle: "CALIBER remembers"
  cta:
    primary: "Get Started"
    secondary: "Learn More"

features:
  - id: trajectories
    title: "Trajectory Memory"
    description: "Complete task journeys, preserved"
    icon: route

  - id: scopes
    title: "Scoped Context"
    description: "Isolated memory partitions"
    icon: layers
```

### Asset Registry
```yaml
# assets/registry.yaml
icons:
  route:
    src: /icons/route.svg
    alt: Route icon
  layers:
    src: /icons/layers.svg
    alt: Layers icon

images:
  hero-bg:
    src: /images/hero-bg.webp
    srcset:
      - /images/hero-bg-640.webp 640w
      - /images/hero-bg-1280.webp 1280w
    alt: Abstract neural network

  logo:
    src: /images/caliber-logo.svg
    alt: CALIBER logo

animations:
  memory-hierarchy:
    component: MemoryHierarchy
    props:
      parallax: true
```

### Type-safe Content Access
```typescript
// Generated from YAML schema
interface Content {
  landing: {
    hero: {
      title: string;
      subtitle: string;
      cta: { primary: string; secondary: string };
    };
    features: Array<{
      id: string;
      title: string;
      description: string;
      icon: AssetKey<'icons'>;
    }>;
  };
}

// Asset registry types
type AssetKey<T extends 'icons' | 'images' | 'animations'> =
  keyof AssetRegistry[T];

function asset<T extends 'icons' | 'images'>(
  type: T,
  key: AssetKey<T>
): AssetRegistry[T][AssetKey<T>] {
  return registry[type][key];
}

// Usage
asset('icons', 'route'); // ✓ Typed!
asset('icons', 'nonexistent'); // ❌ Type error
```
