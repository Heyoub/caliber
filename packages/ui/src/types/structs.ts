/**
 * ═══════════════════════════════════════════════════════════════════════════
 * C-LIKE STRUCT PATTERNS FOR COMPONENTS
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * This module defines component data structures using a pattern inspired by
 * C structs. Each component has:
 *
 * 1. A readonly struct interface - the data shape
 * 2. A factory function - like malloc + initialization
 * 3. Default values - sensible defaults for all properties
 *
 * Benefits:
 * - Immutable by default (all properties are readonly)
 * - Type-safe construction with partial initialization
 * - Tagged unions for component identification
 * - Easy to use with Svelte's $state() for reactivity
 *
 * Theory: This is a form of the "smart constructor" pattern from functional
 * programming. The factory function acts as a controlled entry point that
 * ensures all structs are well-formed.
 *
 * @example
 * ```ts
 * // Create with defaults
 * const btn = createButton({ color: 'coral' });
 *
 * // Use with Svelte state
 * let button = $state(createButton({ size: 'lg', glow: true }));
 *
 * // Access is type-safe
 * console.log(button.color); // 'teal' (default)
 * console.log(button.size);  // 'lg' (overridden)
 * ```
 */

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
  Shadow,
} from './modifiers';

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENT TAG TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * All possible component tags.
 * Used as discriminant in union types.
 */
export type ComponentTag =
  | 'Button'
  | 'IconButton'
  | 'Input'
  | 'TextArea'
  | 'Select'
  | 'Checkbox'
  | 'Toggle'
  | 'Badge'
  | 'Avatar'
  | 'Card'
  | 'Modal'
  | 'Tooltip'
  | 'Dropdown';

// ═══════════════════════════════════════════════════════════════════════════
// COMMON STATE STRUCTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Interaction state - tracks hover, focus, press states.
 */
export interface InteractionState {
  readonly hovered: boolean;
  readonly focused: boolean;
  readonly pressed: boolean;
}

/**
 * Default interaction state.
 */
export const DEFAULT_INTERACTION_STATE: InteractionState = {
  hovered: false,
  focused: false,
  pressed: false,
} as const;

/**
 * Aspect state - component status flags.
 */
export interface AspectState {
  readonly loading: boolean;
  readonly disabled: boolean;
  readonly error: boolean;
  readonly success: boolean;
  readonly selected: boolean;
  readonly active: boolean;
}

/**
 * Default aspect state.
 */
export const DEFAULT_ASPECT_STATE: AspectState = {
  loading: false,
  disabled: false,
  error: false,
  success: false,
  selected: false,
  active: false,
} as const;

// ═══════════════════════════════════════════════════════════════════════════
// BUTTON STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Button component struct.
 * Represents all state needed to render a button.
 */
export interface ButtonStruct {
  readonly tag: 'Button';
  readonly color: ColorToken;
  readonly size: Size;
  readonly variant: 'solid' | 'outline' | 'ghost' | 'link';
  readonly glow: GlowEffect;
  readonly glass: GlassEffect;
  readonly hover: HoverEffect;
  readonly press: PressEffect;
  readonly focus: FocusEffect;
  readonly rounded: Rounded;
  readonly fullWidth: boolean;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial button initialization options.
 */
export type ButtonInit = Partial<Omit<ButtonStruct, 'tag'>>;

/**
 * Default button values.
 */
export const DEFAULT_BUTTON: ButtonStruct = {
  tag: 'Button',
  color: 'teal',
  size: 'md',
  variant: 'solid',
  glow: false,
  glass: false,
  hover: 'lift',
  press: 'sink',
  focus: 'ring',
  rounded: 'lg',
  fullWidth: false,
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new ButtonStruct with defaults applied.
 * Acts like malloc + initialization in C.
 */
export function createButton(init: ButtonInit = {}): ButtonStruct {
  return {
    ...DEFAULT_BUTTON,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

/**
 * Legacy alias for backward compatibility.
 */
export const createButtonStruct = createButton;

// ═══════════════════════════════════════════════════════════════════════════
// ICON BUTTON STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Icon button component struct.
 */
export interface IconButtonStruct {
  readonly tag: 'IconButton';
  readonly icon: string;
  readonly color: ColorToken;
  readonly size: Size;
  readonly glow: GlowEffect;
  readonly rounded: Rounded;
  readonly tooltip?: string;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial icon button initialization options.
 */
export type IconButtonInit = Partial<Omit<IconButtonStruct, 'tag' | 'icon'>> & {
  readonly icon: string;
};

/**
 * Default icon button values.
 */
export const DEFAULT_ICON_BUTTON: Omit<IconButtonStruct, 'icon'> = {
  tag: 'IconButton',
  color: 'slate',
  size: 'md',
  glow: false,
  rounded: 'lg',
  tooltip: undefined,
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new IconButtonStruct.
 */
export function createIconButton(init: IconButtonInit): IconButtonStruct {
  return {
    ...DEFAULT_ICON_BUTTON,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// INPUT STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Input types.
 */
export type InputType = 'text' | 'email' | 'password' | 'number' | 'search' | 'url' | 'tel';

/**
 * Input component struct.
 */
export interface InputStruct {
  readonly tag: 'Input';
  readonly type: InputType;
  readonly size: Size;
  readonly glass: GlassEffect;
  readonly border: BorderEffect;
  readonly rounded: Rounded;
  readonly placeholder?: string;
  readonly prefix?: string;
  readonly suffix?: string;
  readonly value: string;
  readonly state: InteractionState & {
    readonly hasValue: boolean;
    readonly hasError: boolean;
  };
  readonly aspects: AspectState;
}

/**
 * Partial input initialization options.
 */
export type InputInit = Partial<Omit<InputStruct, 'tag'>>;

/**
 * Default input values.
 */
export const DEFAULT_INPUT: InputStruct = {
  tag: 'Input',
  type: 'text',
  size: 'md',
  glass: 'medium',
  border: 'subtle',
  rounded: 'lg',
  placeholder: undefined,
  prefix: undefined,
  suffix: undefined,
  value: '',
  state: { ...DEFAULT_INTERACTION_STATE, hasValue: false, hasError: false },
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new InputStruct.
 */
export function createInput(init: InputInit = {}): InputStruct {
  return {
    ...DEFAULT_INPUT,
    ...init,
    state: { ...DEFAULT_INPUT.state, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

/**
 * Legacy alias for backward compatibility.
 */
export const createInputStruct = createInput;

// ═══════════════════════════════════════════════════════════════════════════
// TEXT AREA STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * TextArea component struct.
 */
export interface TextAreaStruct {
  readonly tag: 'TextArea';
  readonly size: Size;
  readonly glass: GlassEffect;
  readonly border: BorderEffect;
  readonly rounded: Rounded;
  readonly rows: number;
  readonly autoResize: boolean;
  readonly placeholder?: string;
  readonly value: string;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial textarea initialization options.
 */
export type TextAreaInit = Partial<Omit<TextAreaStruct, 'tag'>>;

/**
 * Default textarea values.
 */
export const DEFAULT_TEXT_AREA: TextAreaStruct = {
  tag: 'TextArea',
  size: 'md',
  glass: 'subtle',
  border: 'subtle',
  rounded: 'lg',
  rows: 3,
  autoResize: false,
  placeholder: undefined,
  value: '',
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new TextAreaStruct.
 */
export function createTextArea(init: TextAreaInit = {}): TextAreaStruct {
  return {
    ...DEFAULT_TEXT_AREA,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// SELECT STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Select option item.
 */
export interface SelectOption {
  readonly value: string;
  readonly label: string;
  readonly disabled?: boolean;
}

/**
 * Select component struct.
 */
export interface SelectStruct {
  readonly tag: 'Select';
  readonly size: Size;
  readonly glass: GlassEffect;
  readonly border: BorderEffect;
  readonly rounded: Rounded;
  readonly options: readonly SelectOption[];
  readonly searchable: boolean;
  readonly placeholder?: string;
  readonly value: string | null;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial select initialization options.
 */
export type SelectInit = Partial<Omit<SelectStruct, 'tag'>>;

/**
 * Default select values.
 */
export const DEFAULT_SELECT: SelectStruct = {
  tag: 'Select',
  size: 'md',
  glass: 'subtle',
  border: 'subtle',
  rounded: 'lg',
  options: [],
  searchable: false,
  placeholder: 'Select...',
  value: null,
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new SelectStruct.
 */
export function createSelect(init: SelectInit = {}): SelectStruct {
  return {
    ...DEFAULT_SELECT,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// CHECKBOX STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Checkbox component struct.
 */
export interface CheckboxStruct {
  readonly tag: 'Checkbox';
  readonly size: Size;
  readonly color: ColorToken;
  readonly checked: boolean;
  readonly indeterminate: boolean;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial checkbox initialization options.
 */
export type CheckboxInit = Partial<Omit<CheckboxStruct, 'tag'>>;

/**
 * Default checkbox values.
 */
export const DEFAULT_CHECKBOX: CheckboxStruct = {
  tag: 'Checkbox',
  size: 'md',
  color: 'teal',
  checked: false,
  indeterminate: false,
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new CheckboxStruct.
 */
export function createCheckbox(init: CheckboxInit = {}): CheckboxStruct {
  return {
    ...DEFAULT_CHECKBOX,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TOGGLE STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Toggle component struct.
 */
export interface ToggleStruct {
  readonly tag: 'Toggle';
  readonly size: Size;
  readonly color: ColorToken;
  readonly checked: boolean;
  readonly labelOn?: string;
  readonly labelOff?: string;
  readonly state: InteractionState & {
    readonly checked: boolean;
  };
  readonly aspects: AspectState;
}

/**
 * Partial toggle initialization options.
 */
export type ToggleInit = Partial<Omit<ToggleStruct, 'tag'>>;

/**
 * Default toggle values.
 */
export const DEFAULT_TOGGLE: ToggleStruct = {
  tag: 'Toggle',
  size: 'md',
  color: 'teal',
  checked: false,
  labelOn: undefined,
  labelOff: undefined,
  state: { ...DEFAULT_INTERACTION_STATE, checked: false },
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new ToggleStruct.
 */
export function createToggle(init: ToggleInit = {}): ToggleStruct {
  return {
    ...DEFAULT_TOGGLE,
    ...init,
    state: { ...DEFAULT_TOGGLE.state, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

/**
 * Legacy alias for backward compatibility.
 */
export const createToggleStruct = createToggle;

// ═══════════════════════════════════════════════════════════════════════════
// BADGE STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Badge component struct.
 */
export interface BadgeStruct {
  readonly tag: 'Badge';
  readonly color: ColorToken;
  readonly size: Size;
  readonly glow: GlowEffect;
  readonly dot: boolean;
  readonly removable: boolean;
  readonly text: string;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial badge initialization options.
 */
export type BadgeInit = Partial<Omit<BadgeStruct, 'tag'>>;

/**
 * Default badge values.
 */
export const DEFAULT_BADGE: BadgeStruct = {
  tag: 'Badge',
  color: 'teal',
  size: 'md',
  glow: false,
  dot: false,
  removable: false,
  text: '',
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new BadgeStruct.
 */
export function createBadge(init: BadgeInit = {}): BadgeStruct {
  return {
    ...DEFAULT_BADGE,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

/**
 * Legacy alias for backward compatibility.
 */
export const createBadgeStruct = createBadge;

// ═══════════════════════════════════════════════════════════════════════════
// AVATAR STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Avatar status indicator.
 */
export type AvatarStatus = 'online' | 'offline' | 'busy' | 'away';

/**
 * Avatar component struct.
 */
export interface AvatarStruct {
  readonly tag: 'Avatar';
  readonly src?: string;
  readonly name?: string;
  readonly size: Size;
  readonly status?: AvatarStatus;
  readonly rounded: Rounded;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial avatar initialization options.
 */
export type AvatarInit = Partial<Omit<AvatarStruct, 'tag'>>;

/**
 * Default avatar values.
 */
export const DEFAULT_AVATAR: AvatarStruct = {
  tag: 'Avatar',
  src: undefined,
  name: undefined,
  size: 'md',
  status: undefined,
  rounded: 'full',
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new AvatarStruct.
 */
export function createAvatar(init: AvatarInit = {}): AvatarStruct {
  return {
    ...DEFAULT_AVATAR,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

/**
 * Legacy alias for backward compatibility.
 */
export const createAvatarStruct = createAvatar;

// ═══════════════════════════════════════════════════════════════════════════
// CARD STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Card component struct.
 */
export interface CardStruct {
  readonly tag: 'Card';
  readonly color: ColorToken;
  readonly glass: GlassEffect;
  readonly border: BorderEffect;
  readonly rounded: Rounded;
  readonly shadow: Shadow;
  readonly hover: HoverEffect;
  readonly padding: Size;
  readonly expanded: boolean;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial card initialization options.
 */
export type CardInit = Partial<Omit<CardStruct, 'tag'>>;

/**
 * Default card values.
 */
export const DEFAULT_CARD: CardStruct = {
  tag: 'Card',
  color: 'slate',
  glass: 'medium',
  border: 'subtle',
  rounded: 'xl',
  shadow: 'md',
  hover: 'none',
  padding: 'md',
  expanded: false,
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new CardStruct.
 */
export function createCard(init: CardInit = {}): CardStruct {
  return {
    ...DEFAULT_CARD,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// MODAL STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Modal size options.
 */
export type ModalSize = 'sm' | 'md' | 'lg' | 'xl' | 'full';

/**
 * Modal component struct.
 */
export interface ModalStruct {
  readonly tag: 'Modal';
  readonly size: ModalSize;
  readonly glass: GlassEffect;
  readonly border: BorderEffect;
  readonly rounded: Rounded;
  readonly open: boolean;
  readonly closable: boolean;
  readonly title?: string;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial modal initialization options.
 */
export type ModalInit = Partial<Omit<ModalStruct, 'tag'>>;

/**
 * Default modal values.
 */
export const DEFAULT_MODAL: ModalStruct = {
  tag: 'Modal',
  size: 'md',
  glass: 'frosted',
  border: 'subtle',
  rounded: 'xl',
  open: false,
  closable: true,
  title: undefined,
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new ModalStruct.
 */
export function createModal(init: ModalInit = {}): ModalStruct {
  return {
    ...DEFAULT_MODAL,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TOOLTIP STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Tooltip position options.
 */
export type TooltipPosition = 'top' | 'right' | 'bottom' | 'left';

/**
 * Tooltip component struct.
 */
export interface TooltipStruct {
  readonly tag: 'Tooltip';
  readonly content: string;
  readonly position: TooltipPosition;
  readonly delay: number;
  readonly glass: GlassEffect;
  readonly rounded: Rounded;
  readonly visible: boolean;
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial tooltip initialization options.
 */
export type TooltipInit = Partial<Omit<TooltipStruct, 'tag' | 'content'>> & {
  readonly content: string;
};

/**
 * Default tooltip values.
 */
export const DEFAULT_TOOLTIP: Omit<TooltipStruct, 'content'> = {
  tag: 'Tooltip',
  position: 'top',
  delay: 200,
  glass: 'solid',
  rounded: 'md',
  visible: false,
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new TooltipStruct.
 */
export function createTooltip(init: TooltipInit): TooltipStruct {
  return {
    ...DEFAULT_TOOLTIP,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// DROPDOWN STRUCT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Dropdown menu item.
 */
export interface DropdownItem {
  readonly id: string;
  readonly label: string;
  readonly icon?: string;
  readonly shortcut?: string;
  readonly disabled?: boolean;
  readonly danger?: boolean;
}

/**
 * Dropdown menu group.
 */
export interface DropdownGroup {
  readonly label?: string;
  readonly items: readonly DropdownItem[];
}

/**
 * Dropdown component struct.
 */
export interface DropdownStruct {
  readonly tag: 'Dropdown';
  readonly glass: GlassEffect;
  readonly border: BorderEffect;
  readonly rounded: Rounded;
  readonly shadow: Shadow;
  readonly groups: readonly DropdownGroup[];
  readonly open: boolean;
  readonly position: 'bottom-start' | 'bottom-end' | 'top-start' | 'top-end';
  readonly state: InteractionState;
  readonly aspects: AspectState;
}

/**
 * Partial dropdown initialization options.
 */
export type DropdownInit = Partial<Omit<DropdownStruct, 'tag'>>;

/**
 * Default dropdown values.
 */
export const DEFAULT_DROPDOWN: DropdownStruct = {
  tag: 'Dropdown',
  glass: 'frosted',
  border: 'subtle',
  rounded: 'lg',
  shadow: 'lg',
  groups: [],
  open: false,
  position: 'bottom-start',
  state: DEFAULT_INTERACTION_STATE,
  aspects: DEFAULT_ASPECT_STATE,
} as const;

/**
 * Creates a new DropdownStruct.
 */
export function createDropdown(init: DropdownInit = {}): DropdownStruct {
  return {
    ...DEFAULT_DROPDOWN,
    ...init,
    state: { ...DEFAULT_INTERACTION_STATE, ...init.state },
    aspects: { ...DEFAULT_ASPECT_STATE, ...init.aspects },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// COLOR CONFIG (For button/badge color mapping)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Color configuration for components.
 */
export interface ColorConfig {
  readonly bg: string;
  readonly hover: string;
  readonly text: string;
  readonly border: string;
  readonly shadow: string;
  readonly gradient: string;
  readonly pressGradient: string;
  readonly glow: string;
  readonly outerGlow: string;
}

/**
 * Color configurations for all palette colors.
 */
export const COLOR_CONFIGS: Record<string, ColorConfig> = {
  teal: {
    bg: 'bg-[hsl(var(--teal-500))]',
    hover: 'hover:bg-[hsl(var(--teal-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--teal-700))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--teal-400)_/_0.4)]',
    gradient: 'from-[hsl(var(--teal-300))] via-[hsl(var(--teal-500))] to-[hsl(var(--teal-700))]',
    pressGradient:
      'from-[hsl(var(--teal-700))] via-[hsl(var(--teal-500))] to-[hsl(var(--teal-300))]',
    glow: 'hsl(var(--teal-400))',
    outerGlow: 'rgba(45, 212, 191, 0.4)',
  },
  coral: {
    bg: 'bg-[hsl(var(--coral-500))]',
    hover: 'hover:bg-[hsl(var(--coral-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--coral-300))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--coral-400)_/_0.4)]',
    gradient: 'from-[hsl(var(--coral-300))] via-[hsl(var(--coral-500))] to-[hsl(var(--coral-700))]',
    pressGradient:
      'from-[hsl(var(--coral-700))] via-[hsl(var(--coral-500))] to-[hsl(var(--coral-300))]',
    glow: 'hsl(var(--coral-400))',
    outerGlow: 'rgba(255, 129, 112, 0.4)',
  },
  purple: {
    bg: 'bg-[hsl(var(--purple-500))]',
    hover: 'hover:bg-[hsl(var(--purple-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--purple-700))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--purple-400)_/_0.4)]',
    gradient:
      'from-[hsl(var(--purple-300))] via-[hsl(var(--purple-500))] to-[hsl(var(--purple-700))]',
    pressGradient:
      'from-[hsl(var(--purple-700))] via-[hsl(var(--purple-500))] to-[hsl(var(--purple-300))]',
    glow: 'hsl(var(--purple-400))',
    outerGlow: 'rgba(167, 139, 250, 0.4)',
  },
  pink: {
    bg: 'bg-[hsl(var(--pink-500))]',
    hover: 'hover:bg-[hsl(var(--pink-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--pink-700))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--pink-400)_/_0.4)]',
    gradient: 'from-[hsl(var(--pink-300))] via-[hsl(var(--pink-500))] to-[hsl(var(--pink-700))]',
    pressGradient:
      'from-[hsl(var(--pink-700))] via-[hsl(var(--pink-500))] to-[hsl(var(--pink-300))]',
    glow: 'hsl(var(--pink-400))',
    outerGlow: 'rgba(236, 72, 153, 0.4)',
  },
  mint: {
    bg: 'bg-[hsl(var(--mint-500))]',
    hover: 'hover:bg-[hsl(var(--mint-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--mint-700))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--mint-400)_/_0.4)]',
    gradient: 'from-[hsl(var(--mint-300))] via-[hsl(var(--mint-500))] to-[hsl(var(--mint-700))]',
    pressGradient:
      'from-[hsl(var(--mint-700))] via-[hsl(var(--mint-500))] to-[hsl(var(--mint-300))]',
    glow: 'hsl(var(--mint-400))',
    outerGlow: 'rgba(110, 231, 183, 0.4)',
  },
  amber: {
    bg: 'bg-[hsl(var(--amber-500))]',
    hover: 'hover:bg-[hsl(var(--amber-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--amber-700))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--amber-400)_/_0.4)]',
    gradient: 'from-[hsl(var(--amber-300))] via-[hsl(var(--amber-500))] to-[hsl(var(--amber-700))]',
    pressGradient:
      'from-[hsl(var(--amber-700))] via-[hsl(var(--amber-500))] to-[hsl(var(--amber-300))]',
    glow: 'hsl(var(--amber-400))',
    outerGlow: 'rgba(251, 191, 36, 0.4)',
  },
  slate: {
    bg: 'bg-[hsl(var(--slate-700))]',
    hover: 'hover:bg-[hsl(var(--slate-600))]',
    text: 'text-[hsl(var(--slate-100))]',
    border: 'border-[hsl(var(--slate-500))]',
    shadow: 'shadow-[0_0_8px_rgba(51,65,85,0.4)]',
    gradient: 'from-[hsl(var(--slate-500))] via-[hsl(var(--slate-700))] to-[hsl(var(--slate-900))]',
    pressGradient:
      'from-[hsl(var(--slate-900))] via-[hsl(var(--slate-700))] to-[hsl(var(--slate-500))]',
    glow: 'hsl(var(--slate-400))',
    outerGlow: 'rgba(51, 65, 85, 0.4)',
  },
  ghost: {
    bg: 'bg-transparent',
    hover: 'hover:bg-[hsl(var(--slate-800))]',
    text: 'text-[hsl(var(--slate-300))]',
    border: 'border-transparent',
    shadow: 'shadow-none',
    gradient: 'from-transparent via-[hsl(var(--slate-800)_/_0.5)] to-transparent',
    pressGradient: 'from-transparent via-[hsl(var(--slate-700)_/_0.5)] to-transparent',
    glow: 'transparent',
    outerGlow: 'transparent',
  },
} as const;

// ═══════════════════════════════════════════════════════════════════════════
// UNION OF ALL COMPONENT STRUCTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Union of all component struct types.
 * Use the 'tag' discriminant for pattern matching.
 */
export type ComponentStruct =
  | ButtonStruct
  | IconButtonStruct
  | InputStruct
  | TextAreaStruct
  | SelectStruct
  | CheckboxStruct
  | ToggleStruct
  | BadgeStruct
  | AvatarStruct
  | CardStruct
  | ModalStruct
  | TooltipStruct
  | DropdownStruct;

/**
 * Extract struct type by tag.
 */
export type StructByTag<T extends ComponentTag> = Extract<ComponentStruct, { tag: T }>;
