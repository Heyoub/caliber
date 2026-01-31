/**
 * CALIBER UI Atoms
 *
 * Atomic UI components - the building blocks of the design system.
 * These are indivisible primitives that cannot be broken down further.
 */

// Button components
export { default as Button } from './Button.svelte';
export { default as IconButton } from './IconButton.svelte';

// Form components
export { default as Input } from './Input.svelte';
export { default as TextArea } from './TextArea.svelte';
export { default as Toggle } from './Toggle.svelte';

// Display components
export { default as Badge } from './Badge.svelte';
export { default as Icon } from './Icon.svelte';
export { default as Spinner } from './Spinner.svelte';
export { default as Avatar } from './Avatar.svelte';

// Layout components
export { default as Divider } from './Divider.svelte';

// Overlay components
export { default as Tooltip } from './Tooltip.svelte';

// Typography components
export { default as Kbd } from './Kbd.svelte';

// Re-export types
export type {
  // Modifiers
  ColorPalette,
  ColorIntensity,
  ColorToken,
  Size,
  Spacing,
  GlowEffect,
  GlassEffect,
  BorderEffect,
  HoverEffect,
  PressEffect,
  FocusEffect,
  Placement,
  Orientation,
  // Props
  BaseProps,
  InteractiveProps,
  StyledProps,
  AspectFlags,
} from '../types';
