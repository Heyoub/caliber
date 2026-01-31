/**
 * ═══════════════════════════════════════════════════════════════════════════
 * MODIFIER SYSTEM - TYPED DESIGN TOKENS
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * This module defines all component modifiers as const arrays with derived
 * types. The pattern uses `as const` assertions to create "literal types"
 * that TypeScript can fully understand at compile time.
 *
 * Theory: Using const assertions, we create "singleton types" - types where
 * each value is its own unique type. Combined with `typeof` and indexed
 * access types, we can derive union types directly from runtime values.
 *
 * This approach provides:
 * 1. Single source of truth - values and types are always in sync
 * 2. IDE autocomplete - all valid values are suggested
 * 3. Compile-time validation - invalid values are caught early
 * 4. Runtime iteration - const arrays can be looped over
 *
 * @example
 * ```ts
 * // Type is automatically 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl'
 * type Size = typeof SIZES[number];
 *
 * // Safe to iterate at runtime
 * SIZES.forEach(size => console.log(size));
 * ```
 */

// ═══════════════════════════════════════════════════════════════════════════
// SIZE SYSTEM
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Standard size scale used across all components.
 * Each size maps to consistent spacing, typography, and padding values.
 */
export const SIZES = ['xs', 'sm', 'md', 'lg', 'xl', '2xl'] as const;

/**
 * Size type derived from SIZES array.
 * Use this when a component accepts a size prop.
 */
export type Size = (typeof SIZES)[number];

/**
 * Spacing scale values in Tailwind units.
 * Maps to rem values (e.g., 4 = 1rem = 16px).
 */
export const SPACING_VALUES = [0, 1, 2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 24] as const;

/**
 * Spacing type for padding, margin, gap props.
 */
export type Spacing = (typeof SPACING_VALUES)[number];

// ═══════════════════════════════════════════════════════════════════════════
// COLOR SYSTEM
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Primary color palette names.
 * Each color has semantic meaning in the CALIBER design system.
 */
export const COLOR_PALETTE = [
  'teal', // Primary - memory, scopes
  'coral', // Accent - warnings, highlights
  'purple', // Secondary - AI, intelligence
  'pink', // Accent - ephemeral, turns
  'mint', // Success - confirmations
  'amber', // Rust accent - semantic, notes
  'slate', // Neutral - backgrounds, borders
  'ghost', // Transparent with subtle hover
] as const;

/**
 * Color palette type.
 */
export type ColorPalette = (typeof COLOR_PALETTE)[number];

/**
 * Valid color intensity values (50-900 scale).
 */
export const COLOR_INTENSITIES = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900] as const;

/**
 * Color intensity type.
 */
export type ColorIntensity = (typeof COLOR_INTENSITIES)[number];

/**
 * Color token - either a base color or color-intensity combination.
 * Template literal type ensures only valid combinations compile.
 */
export type ColorToken = ColorPalette | `${ColorPalette}-${ColorIntensity}`;

/**
 * Semantic color roles for component states.
 */
export const SEMANTIC_COLORS = {
  primary: 'teal',
  secondary: 'purple',
  success: 'mint',
  warning: 'amber',
  error: 'coral',
  info: 'slate',
} as const;

/**
 * Semantic color type.
 */
export type SemanticColor = keyof typeof SEMANTIC_COLORS;

// ═══════════════════════════════════════════════════════════════════════════
// GLOW EFFECTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Glow intensity options.
 */
export const GLOW_EFFECTS = ['subtle', 'medium', 'intense', 'pulse'] as const;

/**
 * Glow effect type - can be boolean for default or specific intensity.
 */
export type GlowEffect = boolean | (typeof GLOW_EFFECTS)[number];

/**
 * Glow intensity type (non-boolean options only).
 */
export type GlowIntensity = (typeof GLOW_EFFECTS)[number];

// ═══════════════════════════════════════════════════════════════════════════
// GLASS EFFECTS (Glassmorphism)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Glass blur intensity options.
 */
export const GLASS_EFFECTS = ['subtle', 'medium', 'frosted', 'solid'] as const;

/**
 * Glass effect type - can be boolean for default or specific intensity.
 */
export type GlassEffect = boolean | (typeof GLASS_EFFECTS)[number];

/**
 * Glass intensity type (non-boolean options only).
 */
export type GlassIntensity = (typeof GLASS_EFFECTS)[number];

// ═══════════════════════════════════════════════════════════════════════════
// BORDER EFFECTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Border style options.
 */
export const BORDER_EFFECTS = ['none', 'subtle', 'medium', 'strong', 'glow'] as const;

/**
 * Border effect type.
 */
export type BorderEffect = boolean | (typeof BORDER_EFFECTS)[number];

/**
 * Border style type (non-boolean options only).
 */
export type BorderStyle = (typeof BORDER_EFFECTS)[number];

// ═══════════════════════════════════════════════════════════════════════════
// INTERACTION EFFECTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Hover effect options.
 */
export const HOVER_EFFECTS = ['none', 'lift', 'glow', 'scale', 'brighten', 'border'] as const;

/**
 * Hover effect type.
 */
export type HoverEffect = (typeof HOVER_EFFECTS)[number];

/**
 * Press/active effect options.
 */
export const PRESS_EFFECTS = ['none', 'sink', 'scale', 'darken'] as const;

/**
 * Press effect type.
 */
export type PressEffect = (typeof PRESS_EFFECTS)[number];

/**
 * Focus effect options.
 */
export const FOCUS_EFFECTS = ['ring', 'glow', 'border'] as const;

/**
 * Focus effect type.
 */
export type FocusEffect = (typeof FOCUS_EFFECTS)[number];

// ═══════════════════════════════════════════════════════════════════════════
// ANIMATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Entry animation options.
 */
export const ANIMATE_IN = ['fade', 'slide', 'scale', 'spring'] as const;

/**
 * Entry animation type.
 */
export type AnimateIn = (typeof ANIMATE_IN)[number];

/**
 * Exit animation options.
 */
export const ANIMATE_OUT = ['fade', 'slide', 'scale'] as const;

/**
 * Exit animation type.
 */
export type AnimateOut = (typeof ANIMATE_OUT)[number];

/**
 * Animation duration presets.
 */
export const DURATIONS = ['fast', 'normal', 'slow'] as const;

/**
 * Animation duration type.
 */
export type Duration = (typeof DURATIONS)[number];

/**
 * Easing function presets.
 */
export const EASINGS = ['default', 'in', 'out', 'spring'] as const;

/**
 * Easing type.
 */
export type Easing = (typeof EASINGS)[number];

// ═══════════════════════════════════════════════════════════════════════════
// LAYOUT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Flex alignment options.
 */
export const FLEX_ALIGN = ['start', 'center', 'end', 'stretch', 'baseline'] as const;

/**
 * Flex align type.
 */
export type FlexAlign = (typeof FLEX_ALIGN)[number];

/**
 * Flex justify options.
 */
export const FLEX_JUSTIFY = ['start', 'center', 'end', 'between', 'around', 'evenly'] as const;

/**
 * Flex justify type.
 */
export type FlexJustify = (typeof FLEX_JUSTIFY)[number];

/**
 * Flex direction options.
 */
export const FLEX_DIRECTION = ['row', 'col', 'row-reverse', 'col-reverse'] as const;

/**
 * Flex direction type.
 */
export type FlexDirection = (typeof FLEX_DIRECTION)[number];

// ═══════════════════════════════════════════════════════════════════════════
// TYPOGRAPHY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Font family options.
 */
export const FONT_FAMILY = ['sans', 'mono', 'display'] as const;

/**
 * Font family type.
 */
export type FontFamily = (typeof FONT_FAMILY)[number];

/**
 * Font weight options.
 */
export const FONT_WEIGHT = ['normal', 'medium', 'semibold', 'bold'] as const;

/**
 * Font weight type.
 */
export type FontWeight = (typeof FONT_WEIGHT)[number];

/**
 * Font size options.
 */
export const FONT_SIZE = ['xs', 'sm', 'base', 'lg', 'xl', '2xl', '3xl', '4xl'] as const;

/**
 * Font size type.
 */
export type FontSize = (typeof FONT_SIZE)[number];

/**
 * Text alignment options.
 */
export const TEXT_ALIGN = ['left', 'center', 'right'] as const;

/**
 * Text align type.
 */
export type TextAlign = (typeof TEXT_ALIGN)[number];

/**
 * Text transform options.
 */
export const TEXT_TRANSFORM = ['none', 'uppercase', 'lowercase', 'capitalize'] as const;

/**
 * Text transform type.
 */
export type TextTransform = (typeof TEXT_TRANSFORM)[number];

// ═══════════════════════════════════════════════════════════════════════════
// ROUNDED CORNERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Border radius options.
 */
export const ROUNDED = ['none', 'sm', 'md', 'lg', 'xl', '2xl', 'full'] as const;

/**
 * Rounded corner type.
 */
export type Rounded = (typeof ROUNDED)[number];

/**
 * Legacy alias for compatibility.
 */
export type RoundedSize = Size | 'full' | 'none';

// ═══════════════════════════════════════════════════════════════════════════
// SHADOW
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Shadow size options.
 */
export const SHADOWS = ['none', 'sm', 'md', 'lg', 'xl', 'glow'] as const;

/**
 * Shadow type.
 */
export type Shadow = (typeof SHADOWS)[number];

/**
 * Legacy alias for compatibility.
 */
export type ShadowSize = Size | 'none';

// ═══════════════════════════════════════════════════════════════════════════
// PLACEMENT & ORIENTATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Placement options for tooltips, popovers, etc.
 */
export const PLACEMENTS = [
  'top',
  'top-start',
  'top-end',
  'bottom',
  'bottom-start',
  'bottom-end',
  'left',
  'left-start',
  'left-end',
  'right',
  'right-start',
  'right-end',
] as const;

/**
 * Placement type.
 */
export type Placement = (typeof PLACEMENTS)[number];

/**
 * Orientation options.
 */
export const ORIENTATIONS = ['horizontal', 'vertical'] as const;

/**
 * Orientation type.
 */
export type Orientation = (typeof ORIENTATIONS)[number];

// ═══════════════════════════════════════════════════════════════════════════
// TEMPLATE LITERAL TYPES FOR CSS CLASSES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Padding class with direction.
 */
export type PaddingClass =
  | `p-${Spacing}`
  | `pt-${Spacing}`
  | `pr-${Spacing}`
  | `pb-${Spacing}`
  | `pl-${Spacing}`
  | `px-${Spacing}`
  | `py-${Spacing}`;

/**
 * Margin class with direction.
 */
export type MarginClass =
  | `m-${Spacing}`
  | `mt-${Spacing}`
  | `mr-${Spacing}`
  | `mb-${Spacing}`
  | `ml-${Spacing}`
  | `mx-${Spacing}`
  | `my-${Spacing}`;

/**
 * Gap class.
 */
export type GapClass = `gap-${Spacing}`;

/**
 * Text color class.
 */
export type TextColorClass = `text-${ColorToken}`;

/**
 * Background color class.
 */
export type BgColorClass = `bg-${ColorToken}`;

/**
 * Border color class.
 */
export type BorderColorClass = `border-${ColorToken}`;

/**
 * Glow class with optional color and intensity.
 */
export type GlowClass =
  | 'glow'
  | `glow-${GlowIntensity}`
  | `glow-${ColorPalette}`
  | `glow-${ColorPalette}-${GlowIntensity}`;

/**
 * Glass class with optional intensity.
 */
export type GlassClass = 'glass' | `glass-${GlassIntensity}`;

/**
 * Font size class.
 */
export type TextSizeClass = `text-${FontSize}`;

/**
 * Font weight class.
 */
export type FontWeightClass = `font-${FontWeight}`;

/**
 * Union of all spacing-related classes.
 */
export type SpacingClass = PaddingClass | MarginClass | GapClass;

/**
 * Union of all color-related classes.
 */
export type ColorClass = TextColorClass | BgColorClass | BorderColorClass;

// ═══════════════════════════════════════════════════════════════════════════
// MAPPED TYPES FOR MODIFIER → CLASS CONVERSION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Maps modifier names to their CSS class prefixes.
 */
export interface ModifierClassMap {
  size: `size-${Size}`;
  color: `color-${ColorPalette}`;
  glow: GlowClass;
  glass: GlassClass;
  hover: `hover-${HoverEffect}`;
  press: `press-${PressEffect}`;
  focus: `focus-${FocusEffect}`;
  rounded: `rounded-${Rounded}`;
  shadow: `shadow-${Shadow}`;
}

/**
 * Extracts all possible class values from the modifier map.
 */
export type ModifierClass = ModifierClassMap[keyof ModifierClassMap];

/**
 * Size to class mapping.
 */
export const SIZE_CLASSES: Record<Size, string> = {
  xs: 'text-xs px-2 py-1',
  sm: 'text-sm px-3 py-1.5',
  md: 'text-base px-4 py-2',
  lg: 'text-lg px-5 py-2.5',
  xl: 'text-xl px-6 py-3',
  '2xl': 'text-2xl px-8 py-4',
} as const;

/**
 * Glow effect to class mapping.
 */
export const GLOW_CLASSES: Record<GlowIntensity, string> = {
  subtle: 'shadow-[0_0_10px_hsl(var(--component-color)/0.1)]',
  medium: 'shadow-[0_0_20px_hsl(var(--component-color)/0.25)]',
  intense: 'shadow-[0_0_30px_hsl(var(--component-color)/0.5)]',
  pulse: 'animate-glow-pulse shadow-[0_0_20px_hsl(var(--component-color)/0.25)]',
} as const;

/**
 * Glass effect to class mapping.
 */
export const GLASS_CLASSES: Record<GlassIntensity, string> = {
  subtle: 'backdrop-blur-sm bg-slate-900/5',
  medium: 'backdrop-blur-md bg-slate-900/10',
  frosted: 'backdrop-blur-xl bg-slate-900/15',
  solid: 'backdrop-blur-2xl bg-slate-900/20',
} as const;

/**
 * Hover effect to class mapping.
 */
export const HOVER_CLASSES: Record<HoverEffect, string> = {
  none: '',
  lift: 'hover:-translate-y-0.5 transition-transform',
  glow: 'hover:shadow-lg transition-shadow',
  scale: 'hover:scale-[1.02] transition-transform',
  brighten: 'hover:brightness-110 transition-[filter]',
  border: 'hover:border-current transition-colors',
} as const;

/**
 * Press effect to class mapping.
 */
export const PRESS_CLASSES: Record<PressEffect, string> = {
  none: '',
  sink: 'active:translate-y-px transition-transform',
  scale: 'active:scale-[0.98] transition-transform',
  darken: 'active:brightness-90 transition-[filter]',
} as const;

// ═══════════════════════════════════════════════════════════════════════════
// EXHAUSTIVE CHECKING UTILITIES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Assert that a value is never reached.
 * Use in switch statements to ensure all cases are handled.
 *
 * @example
 * ```ts
 * function handleSize(size: Size): number {
 *   switch (size) {
 *     case 'xs': return 12;
 *     case 'sm': return 14;
 *     case 'md': return 16;
 *     case 'lg': return 18;
 *     case 'xl': return 20;
 *     case '2xl': return 24;
 *     default: return assertNever(size);
 *   }
 * }
 * ```
 */
export function assertNever(x: never, message?: string): never {
  throw new Error(message ?? `Unexpected value: ${x}`);
}

/**
 * Check if a value is a valid size.
 */
export function isSize(value: unknown): value is Size {
  return typeof value === 'string' && SIZES.includes(value as Size);
}

/**
 * Check if a value is a valid color palette.
 */
export function isColorPalette(value: unknown): value is ColorPalette {
  return typeof value === 'string' && COLOR_PALETTE.includes(value as ColorPalette);
}

/**
 * Check if a value is a valid color token.
 */
export function isColorToken(value: unknown): value is ColorToken {
  if (typeof value !== 'string') return false;
  if (isColorPalette(value)) return true;

  const parts = value.split('-');
  if (parts.length !== 2) return false;

  const [palette, intensityStr] = parts;
  const intensity = parseInt(intensityStr, 10);

  return isColorPalette(palette) && COLOR_INTENSITIES.includes(intensity as ColorIntensity);
}

// ═══════════════════════════════════════════════════════════════════════════
// MODIFIER RESOLUTION FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Resolves a glow effect to its CSS class.
 */
export function resolveGlow(glow: GlowEffect): string {
  if (glow === false) return '';
  if (glow === true) return GLOW_CLASSES.medium;
  return GLOW_CLASSES[glow];
}

/**
 * Resolves a glass effect to its CSS class.
 */
export function resolveGlass(glass: GlassEffect): string {
  if (glass === false) return '';
  if (glass === true) return GLASS_CLASSES.medium;
  return GLASS_CLASSES[glass];
}

/**
 * Resolves a size to its CSS classes.
 */
export function resolveSize(size: Size): string {
  return SIZE_CLASSES[size];
}

/**
 * Resolves a hover effect to its CSS class.
 */
export function resolveHover(hover: HoverEffect): string {
  return HOVER_CLASSES[hover];
}

/**
 * Resolves a press effect to its CSS class.
 */
export function resolvePress(press: PressEffect): string {
  return PRESS_CLASSES[press];
}

/**
 * Resolves a color to a CSS variable reference.
 */
export function resolveColor(color: ColorToken): string {
  return `hsl(var(--${color}))`;
}

// ═══════════════════════════════════════════════════════════════════════════
// MODIFIER OBJECT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Complete modifier configuration object.
 */
export interface ModifierConfig {
  readonly size?: Size;
  readonly color?: ColorToken;
  readonly glow?: GlowEffect;
  readonly glass?: GlassEffect;
  readonly border?: BorderEffect;
  readonly hover?: HoverEffect;
  readonly press?: PressEffect;
  readonly focus?: FocusEffect;
  readonly rounded?: Rounded;
  readonly shadow?: Shadow;
}

/**
 * Converts a modifier config to CSS classes.
 */
export function modifiersToClasses(config: ModifierConfig): string {
  const classes: string[] = [];

  if (config.size) classes.push(SIZE_CLASSES[config.size]);
  if (config.glow) classes.push(resolveGlow(config.glow));
  if (config.glass) classes.push(resolveGlass(config.glass));
  if (config.hover) classes.push(HOVER_CLASSES[config.hover]);
  if (config.press) classes.push(PRESS_CLASSES[config.press]);
  if (config.rounded) classes.push(`rounded-${config.rounded}`);
  if (config.shadow && config.shadow !== 'none') {
    classes.push(`shadow-${config.shadow}`);
  }

  return classes.filter(Boolean).join(' ');
}
