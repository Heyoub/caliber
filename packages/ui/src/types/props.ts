/**
 * ═══════════════════════════════════════════════════════════════════════════
 * COMPONENT PROP TYPES
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * This module defines the prop interfaces for all components. Props are
 * organized in layers of increasing specificity:
 *
 * 1. BaseProps - Common to all components (class, style, aspects)
 * 2. InteractiveProps - For interactive elements (hover, press, focus)
 * 3. StyledProps - For visually styled components (color, size, effects)
 *
 * Additionally, we use conditional types to provide variant-specific props
 * that change based on the variant selected.
 *
 * Theory: This uses TypeScript's type composition (intersection types)
 * to build complex prop interfaces from simpler building blocks. It's
 * similar to traits/mixins in other languages.
 *
 * @example
 * ```ts
 * // Button props include all styling + button-specific options
 * interface ButtonProps extends StyledProps {
 *   variant?: ButtonVariantKind;
 *   // variant-specific props applied via conditional types
 * }
 * ```
 */

import type { Snippet } from 'svelte';
import type {
  Size,
  ColorToken,
  GlowEffect,
  GlassEffect,
  BorderEffect,
  HoverEffect,
  PressEffect,
  FocusEffect,
  Rounded,
  RoundedSize,
  Shadow,
  ShadowSize,
  AnimateIn,
  AnimateOut,
  FontFamily,
  FontWeight,
  FontSize,
  TextAlign,
  TextTransform,
  FlexAlign,
  FlexJustify,
  FlexDirection,
  Spacing,
} from './modifiers';

// ═══════════════════════════════════════════════════════════════════════════
// ASPECT FLAGS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * State aspect flags - component loading/error/success states.
 */
export interface StateAspects {
  /** Component is loading data or processing */
  readonly loading?: boolean;
  /** Component is disabled and not interactive */
  readonly disabled?: boolean;
  /** Component has an error state */
  readonly error?: boolean;
  /** Component shows success state */
  readonly success?: boolean;
  /** Component is selected */
  readonly selected?: boolean;
  /** Component is in active state */
  readonly active?: boolean;
}

/**
 * Visibility aspect flags - component show/hide states.
 */
export interface VisibilityAspects {
  /** Component is hidden (display: none) */
  readonly hidden?: boolean;
  /** Component is collapsed */
  readonly collapsed?: boolean;
  /** Component is expanded */
  readonly expanded?: boolean;
}

/**
 * Interaction aspect flags - drag/drop/resize capabilities.
 */
export interface InteractionAspects {
  /** Component responds to click/hover */
  readonly interactive?: boolean;
  /** Component can be dragged */
  readonly draggable?: boolean;
  /** Component accepts drops */
  readonly droppable?: boolean;
  /** Component can be resized */
  readonly resizable?: boolean;
}

/**
 * Content aspect flags - text handling.
 */
export interface ContentAspects {
  /** Truncate text with ellipsis */
  readonly truncate?: boolean;
  /** Allow text wrapping */
  readonly wrap?: boolean;
  /** Enable scrolling */
  readonly scrollable?: boolean;
}

/**
 * Layout aspect flags - sizing.
 */
export interface LayoutAspects {
  /** Take full width of container */
  readonly fullWidth?: boolean;
  /** Take full height of container */
  readonly fullHeight?: boolean;
  /** Center content */
  readonly centered?: boolean;
}

/**
 * Animation aspect flags - entry/exit animations.
 */
export interface AnimationAspects {
  /** Enable animations */
  readonly animate?: boolean;
  /** Entry animation type */
  readonly animateIn?: AnimateIn;
  /** Exit animation type */
  readonly animateOut?: AnimateOut;
}

/**
 * All aspect flags combined.
 * Use when you need to accept any aspect configuration.
 */
export interface AspectFlags extends
  StateAspects,
  VisibilityAspects,
  InteractionAspects,
  ContentAspects,
  LayoutAspects,
  AnimationAspects {}

// ═══════════════════════════════════════════════════════════════════════════
// BASE PROPS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Base props shared by all components.
 * Provides escape hatches for custom styling and all aspect flags.
 */
export interface BaseProps extends AspectFlags {
  /** Additional CSS classes (escape hatch) */
  class?: string;
  /** Inline styles (escape hatch) */
  style?: string;
  /** Unique identifier */
  id?: string;
  /** Data attributes for testing */
  testId?: string;
}

// ═══════════════════════════════════════════════════════════════════════════
// INTERACTIVE PROPS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Props for interactive components that respond to user input.
 */
export interface InteractiveProps extends BaseProps {
  /** Hover effect style */
  hover?: HoverEffect;
  /** Press/active effect style */
  press?: PressEffect;
  /** Focus effect style */
  focus?: FocusEffect;
  /** Tab index for keyboard navigation */
  tabIndex?: number;
  /** ARIA role override */
  role?: string;
  /** ARIA label for accessibility */
  ariaLabel?: string;
}

// ═══════════════════════════════════════════════════════════════════════════
// STYLED PROPS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Props for visually styled components.
 * Includes all design system modifiers.
 */
export interface StyledProps extends InteractiveProps {
  /** Primary color token */
  color?: ColorToken;
  /** Component size */
  size?: Size;
  /** Glow effect */
  glow?: GlowEffect;
  /** Glass/blur effect */
  glass?: GlassEffect;
  /** Border style */
  border?: BorderEffect;
  /** Border radius */
  rounded?: RoundedSize;
  /** Shadow depth */
  shadow?: ShadowSize;
}

// ═══════════════════════════════════════════════════════════════════════════
// SLOTTED PROPS (For components with child content)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Props for components that accept Svelte snippets as children.
 */
export interface SlottedProps extends StyledProps {
  children?: Snippet;
}

// ═══════════════════════════════════════════════════════════════════════════
// FORM PROPS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Props for form field components.
 */
export interface FormFieldProps extends StyledProps {
  name?: string;
  value?: string;
  placeholder?: string;
  required?: boolean;
  readonly?: boolean;
  autofocus?: boolean;
}

// ═══════════════════════════════════════════════════════════════════════════
// TYPOGRAPHY PROPS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Props for text styling.
 */
export interface TypographyProps {
  /** Font family */
  readonly font?: FontFamily;
  /** Font weight */
  readonly weight?: FontWeight;
  /** Font size */
  readonly fontSize?: FontSize;
  /** Text alignment */
  readonly align?: TextAlign;
  /** Text transform */
  readonly transform?: TextTransform;
  /** Muted/secondary text color */
  readonly muted?: boolean;
  /** Gradient text effect */
  readonly gradient?: boolean;
}

// ═══════════════════════════════════════════════════════════════════════════
// LAYOUT PROPS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Props for layout containers.
 */
export interface LayoutProps {
  /** Enable flexbox */
  readonly flex?: boolean | FlexDirection;
  /** Flex align-items */
  readonly alignItems?: FlexAlign;
  /** Flex justify-content */
  readonly justify?: FlexJustify;
  /** Gap between items */
  readonly gap?: Spacing;
  /** Enable flex wrap */
  readonly flexWrap?: boolean;
  /** Enable grid layout */
  readonly grid?: boolean | number;
  /** Padding */
  readonly padding?: Spacing;
  /** Margin */
  readonly margin?: Spacing;
}

// ═══════════════════════════════════════════════════════════════════════════
// EVENT HANDLERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Click event handler interface.
 */
export interface ClickHandler {
  onclick?: (event: MouseEvent) => void;
}

/**
 * Focus event handlers interface.
 */
export interface FocusHandlers {
  onfocus?: (event: FocusEvent) => void;
  onblur?: (event: FocusEvent) => void;
}

/**
 * Input event handlers interface.
 */
export interface InputHandlers {
  oninput?: (event: Event & { currentTarget: HTMLInputElement | HTMLTextAreaElement }) => void;
  onchange?: (event: Event) => void;
}

/**
 * Keyboard event handlers interface.
 */
export interface KeyboardHandlers {
  onkeydown?: (event: KeyboardEvent) => void;
  onkeyup?: (event: KeyboardEvent) => void;
}

/**
 * Standard event handler type.
 */
export type EventHandler<E extends Event = Event> = (event: E) => void;

/**
 * Drag event handler type.
 */
export type DragHandler = EventHandler<DragEvent>;

// ═══════════════════════════════════════════════════════════════════════════
// BUTTON VARIANT PROPS (Conditional Types)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Button variant kinds.
 */
export type ButtonVariantKind = 'solid' | 'outline' | 'ghost' | 'link';

/**
 * Props specific to solid button variant.
 */
export interface SolidButtonProps {
  readonly variant: 'solid';
  readonly color: ColorToken;
  readonly glow?: GlowEffect;
}

/**
 * Props specific to outline button variant.
 */
export interface OutlineButtonProps {
  readonly variant: 'outline';
  readonly color: ColorToken;
  readonly borderWidth?: 1 | 2;
}

/**
 * Props specific to ghost button variant.
 */
export interface GhostButtonProps {
  readonly variant: 'ghost';
  readonly hoverColor?: ColorToken;
}

/**
 * Props specific to link button variant.
 */
export interface LinkButtonProps {
  readonly variant: 'link';
  readonly underline?: boolean;
}

/**
 * Union of all button variant-specific props.
 */
export type ButtonVariantProps =
  | SolidButtonProps
  | OutlineButtonProps
  | GhostButtonProps
  | LinkButtonProps;

/**
 * Conditional type to get props for a specific button variant.
 */
export type ButtonPropsForVariant<V extends ButtonVariantKind> =
  V extends 'solid' ? SolidButtonProps :
  V extends 'outline' ? OutlineButtonProps :
  V extends 'ghost' ? GhostButtonProps :
  V extends 'link' ? LinkButtonProps :
  never;

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENT-SPECIFIC PROPS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Button component props.
 */
export interface ButtonProps extends StyledProps, ClickHandler {
  /** Button variant style */
  variant?: ButtonVariantKind;
  /** Button type attribute */
  type?: 'button' | 'submit' | 'reset';
  /** Icon name for left icon */
  iconLeft?: string;
  /** Icon name for right icon */
  iconRight?: string;
  /** Button content */
  children?: Snippet;
}

/**
 * Icon button props.
 */
export interface IconButtonProps extends StyledProps, ClickHandler {
  /** Icon name (required) */
  icon: string;
  /** Tooltip text */
  tooltip?: string;
}

/**
 * Input component props.
 */
export interface InputProps extends FormFieldProps, InputHandlers, FocusHandlers {
  /** Input type */
  type?: 'text' | 'email' | 'password' | 'number' | 'search' | 'url' | 'tel';
  /** Prefix text/icon */
  prefix?: string;
  /** Suffix text/icon */
  suffix?: string;
  /** Autocomplete attribute */
  autocomplete?: string;
  /** Minimum length */
  minlength?: number;
  /** Maximum length */
  maxlength?: number;
  /** Pattern for validation */
  pattern?: string;
}

/**
 * TextArea component props.
 */
export interface TextAreaProps extends FormFieldProps, InputHandlers, FocusHandlers {
  /** Number of visible rows */
  rows?: number;
  /** Auto-resize based on content */
  autoResize?: boolean;
  /** Minimum length */
  minlength?: number;
  /** Maximum length */
  maxlength?: number;
}

/**
 * Select option type.
 */
export interface SelectOption {
  readonly value: string;
  readonly label: string;
  readonly disabled?: boolean;
}

/**
 * Select component props.
 */
export interface SelectProps extends FormFieldProps, FocusHandlers {
  /** Available options */
  options: readonly SelectOption[];
  /** Enable search/filter */
  searchable?: boolean;
  /** Multiple selection */
  multiple?: boolean;
  /** Change handler */
  onchange?: (value: string) => void;
}

/**
 * Checkbox component props.
 */
export interface CheckboxProps extends StyledProps {
  /** Checkbox name attribute */
  name?: string;
  /** Whether checked */
  checked?: boolean;
  /** Indeterminate state (for trees) */
  indeterminate?: boolean;
  /** Change handler */
  onchange?: (checked: boolean) => void;
}

/**
 * Toggle component props.
 */
export interface ToggleProps extends StyledProps {
  /** Toggle name attribute */
  name?: string;
  /** Whether toggled on */
  checked?: boolean;
  /** Label for on state */
  labelOn?: string;
  /** Label for off state */
  labelOff?: string;
  /** Change handler */
  onchange?: (checked: boolean) => void;
}

/**
 * Badge component props.
 */
export interface BadgeProps extends StyledProps {
  /** Badge text content */
  text?: string;
  /** Show status dot */
  dot?: boolean;
  /** Show remove button */
  removable?: boolean;
  /** Remove handler */
  onremove?: () => void;
}

/**
 * Avatar component props.
 */
export interface AvatarProps extends BaseProps {
  /** Image source URL */
  src?: string;
  /** User/entity name for fallback */
  name: string;
  /** Avatar size */
  size?: Size;
  /** Online/offline status */
  status?: 'online' | 'offline' | 'busy' | 'away';
}

/**
 * Card component props.
 */
export interface CardProps extends StyledProps, ClickHandler {
  /** Card padding size */
  padding?: Size;
  /** Whether card is clickable */
  clickable?: boolean;
  /** Card content */
  children?: Snippet;
}

/**
 * Modal component props.
 */
export interface ModalProps extends StyledProps {
  /** Whether modal is open */
  open?: boolean;
  /** Modal width size */
  modalSize?: 'sm' | 'md' | 'lg' | 'xl' | 'full';
  /** Modal title */
  title?: string;
  /** Whether modal can be closed */
  closable?: boolean;
  /** Close handler */
  onclose?: () => void;
  /** Modal content */
  children?: Snippet;
}

/**
 * Tooltip component props.
 */
export interface TooltipProps extends BaseProps {
  /** Tooltip content */
  content: string;
  /** Tooltip position */
  position?: 'top' | 'right' | 'bottom' | 'left';
  /** Delay before showing (ms) */
  delay?: number;
}

/**
 * Icon component props.
 */
export interface IconProps extends BaseProps {
  /** Icon name */
  name: string;
  /** Icon size */
  size?: Size;
  /** Icon color */
  color?: ColorToken;
  /** Spinning animation */
  spin?: boolean;
}

/**
 * Spinner component props.
 */
export interface SpinnerProps extends BaseProps {
  /** Spinner size */
  size?: Size;
  /** Spinner color */
  color?: ColorToken;
}

/**
 * Divider component props.
 */
export interface DividerProps extends BaseProps {
  /** Divider orientation */
  orientation?: 'horizontal' | 'vertical';
  /** Divider color */
  color?: ColorToken;
  /** Spacing around divider */
  spacing?: Spacing;
}

// ═══════════════════════════════════════════════════════════════════════════
// UTILITY TYPES FOR PROPS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Makes all props optional except specified keys.
 */
export type WithRequired<T, K extends keyof T> = T & Required<Pick<T, K>>;

/**
 * Makes specified props required, rest optional.
 */
export type PartialExcept<T, K extends keyof T> = Partial<Omit<T, K>> & Pick<T, K>;

/**
 * Extracts only the aspect flags from a props type.
 */
export type ExtractAspects<T> = Pick<T, keyof T & keyof AspectFlags>;

/**
 * Omits aspect flags from a props type.
 */
export type WithoutAspects<T> = Omit<T, keyof AspectFlags>;

/**
 * Props with children slot.
 */
export interface WithChildren {
  readonly children?: Snippet;
}

/**
 * Props with named slots.
 */
export interface WithSlots<T extends string> {
  readonly slots?: Partial<Record<T, Snippet>>;
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENT PROP MAP
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Map of component names to their prop types.
 * Useful for generic component rendering.
 */
export interface ComponentPropMap {
  Button: ButtonProps;
  IconButton: IconButtonProps;
  Input: InputProps;
  TextArea: TextAreaProps;
  Select: SelectProps;
  Checkbox: CheckboxProps;
  Toggle: ToggleProps;
  Badge: BadgeProps;
  Avatar: AvatarProps;
  Card: CardProps;
  Modal: ModalProps;
  Tooltip: TooltipProps;
  Icon: IconProps;
  Spinner: SpinnerProps;
  Divider: DividerProps;
}

/**
 * Get props type for a component by name.
 */
export type PropsFor<K extends keyof ComponentPropMap> = ComponentPropMap[K];
