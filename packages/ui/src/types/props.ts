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
  loading?: boolean;
  /** Component is disabled and not interactive */
  disabled?: boolean;
  /** Component has an error state */
  error?: boolean;
  /** Component shows success state */
  success?: boolean;
  /** Component is selected */
  selected?: boolean;
  /** Component is in active state */
  active?: boolean;
}

/**
 * Visibility aspect flags - component show/hide states.
 */
export interface VisibilityAspects {
  /** Component is hidden (display: none) */
  hidden?: boolean;
  /** Component is collapsed */
  collapsed?: boolean;
  /** Component is expanded */
  expanded?: boolean;
}

/**
 * Interaction aspect flags - drag/drop/resize capabilities.
 */
export interface InteractionAspects {
  /** Component responds to click/hover */
  interactive?: boolean;
  /** Component can be dragged */
  draggable?: boolean;
  /** Component accepts drops */
  droppable?: boolean;
  /** Component can be resized */
  resizable?: boolean;
}

/**
 * Content aspect flags - text handling.
 */
export interface ContentAspects {
  /** Truncate text with ellipsis */
  truncate?: boolean;
  /** Allow text wrapping */
  wrap?: boolean;
  /** Enable scrolling */
  scrollable?: boolean;
}

/**
 * Layout aspect flags - sizing.
 */
export interface LayoutAspects {
  /** Take full width of container */
  fullWidth?: boolean;
  /** Take full height of container */
  fullHeight?: boolean;
  /** Center content */
  centered?: boolean;
}

/**
 * Animation aspect flags - entry/exit animations.
 */
export interface AnimationAspects {
  /** Enable animations */
  animate?: boolean;
  /** Entry animation type */
  animateIn?: AnimateIn;
  /** Exit animation type */
  animateOut?: AnimateOut;
}

/**
 * All aspect flags combined.
 * Use when you need to accept any aspect configuration.
 */
export interface AspectFlags
  extends StateAspects,
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
  shadow?: Shadow;
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
export type ButtonPropsForVariant<V extends ButtonVariantKind> = V extends 'solid'
  ? SolidButtonProps
  : V extends 'outline'
    ? OutlineButtonProps
    : V extends 'ghost'
      ? GhostButtonProps
      : V extends 'link'
        ? LinkButtonProps
        : never;

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

// ═══════════════════════════════════════════════════════════════════════════
// APPLICATION TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * CMS content strings for internationalization.
 */
export interface CMSContent {
  /** Title text */
  title?: string;
  /** Subtitle text */
  subtitle?: string;
  /** Description text */
  description?: string;
  /** Placeholder text */
  placeholder?: string;
  /** Input placeholder */
  inputPlaceholder?: string;
  /** View placeholder */
  viewPlaceholder?: string;
  /** Submit button label */
  submitLabel?: string;
  /** Cancel button label */
  cancelLabel?: string;
  /** Error message */
  errorLabel?: string;
  /** Success message */
  successLabel?: string;
  /** Loading message */
  loadingLabel?: string;
  /** Chat label */
  chatLabel?: string;
  /** Prompts label */
  promptsLabel?: string;
  /** Templates label */
  templatesLabel?: string;
  /** History label */
  historyLabel?: string;
  /** Send message label */
  sendLabel?: string;
  /** Sending message label */
  sendingLabel?: string;
  /** Dismiss label */
  dismissLabel?: string;
  /** Drop label */
  dropLabel?: string;
  /** Tool calls label */
  toolCallsLabel?: string;
  /** Processing label */
  processingLabel?: string;
  /** User label */
  userLabel?: string;
  /** Assistant label */
  assistantLabel?: string;
  /** Copying label */
  copyingLabel?: string;
  /** Copy label */
  copyLabel?: string;
  /** Pending label */
  pendingLabel?: string;
  /** Approved label */
  approvedLabel?: string;
  /** Running label */
  runningLabel?: string;
  /** Index signature for any additional labels */
  [key: string]: string | undefined;
}

/**
 * Chat message data type.
 * Named ChatMessageData to avoid conflict with ChatMessage component.
 */
export interface ChatMessageData {
  /** Message ID */
  id: string;
  /** Message role */
  role: 'user' | 'assistant' | 'system';
  /** Message content */
  content: string;
  /** Timestamp */
  timestamp: Date;
  /** Tool calls in this message */
  toolCalls?: ToolCall[];
  /** Whether message is streaming */
  isStreaming?: boolean;
  /** Whether assistant is typing */
  typing?: boolean;
  /** Model used for this message */
  model?: string;
}

/**
 * Tool call status type (string literal union).
 */
export type ToolCallStatusLiteral =
  | 'pending'
  | 'approved'
  | 'running'
  | 'success'
  | 'error'
  | 'rejected';

/**
 * Tool call type.
 */
export interface ToolCall {
  /** Tool call ID */
  id: string;
  /** Tool name */
  name: string;
  /** Tool arguments */
  arguments: Record<string, unknown>;
  /** Tool call status */
  status: ToolCallStatusLiteral;
  /** Result if completed */
  result?: ToolResult;
  /** Execution duration in milliseconds */
  duration?: number;
  /** Start timestamp */
  startedAt?: Date;
  /** End timestamp */
  endedAt?: Date;
}

/**
 * Tool result type.
 */
export interface ToolResult {
  /** Whether tool succeeded */
  success: boolean;
  /** Result data */
  data?: unknown;
  /** Error message if failed */
  error?: string;
}

/**
 * Editor tab type.
 */
export interface EditorTab {
  /** Tab ID */
  id: string;
  /** File name (for display) */
  name: string;
  /** File path or title */
  path: string;
  /** Tab label */
  label: string;
  /** Content */
  content: string;
  /** File format */
  format: FileFormatLiteral;
  /** Language for syntax highlighting */
  language?: string;
  /** Whether file is modified (alias for isDirty) */
  dirty?: boolean;
  /** Whether file is modified */
  isDirty?: boolean;
  /** Whether tab is active */
  isActive?: boolean;
}

/**
 * Editor cursor position.
 */
export interface EditorPosition {
  /** Line number (1-indexed) */
  line: number;
  /** Column number (1-indexed) */
  column: number;
}

/**
 * File format literal type (string union).
 */
export type FileFormatLiteral =
  | 'markdown'
  | 'yaml'
  | 'json'
  | 'xml'
  | 'html'
  | 'css'
  | 'javascript'
  | 'typescript'
  | 'python'
  | 'rust'
  | 'go'
  | 'sql'
  | 'shell'
  | 'toml'
  | 'csv'
  | 'latex'
  | 'mermaid'
  | 'plaintext'
  | 'unknown';

/**
 * File format detection result (detailed info).
 */
export interface FileFormatInfo {
  /** Format name */
  name: FileFormatLiteral;
  /** MIME type */
  mimeType: string;
  /** File extension */
  extension: string;
  /** Language for syntax highlighting */
  language?: string;
}

/**
 * Alias for backwards compatibility.
 * @deprecated Use FileFormatLiteral for string values or FileFormatInfo for detailed info
 */
export type FileFormat = FileFormatLiteral;

// ═══════════════════════════════════════════════════════════════════════════
// TREE VIEW TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Tree node value - can be any JSON-serializable value.
 */
export type TreeNodeValue = string | number | boolean | null | undefined | object | unknown[];

/**
 * Tree node for hierarchical data display.
 */
export interface TreeNode {
  /** Node ID */
  id: string;
  /** Display label */
  label: string;
  /** Node value */
  value?: TreeNodeValue;
  /** Icon name */
  icon?: string;
  /** Child nodes */
  children?: TreeNode[];
  /** Whether node is expanded */
  expanded?: boolean;
  /** Whether node is selected */
  selected?: boolean;
  /** Whether node is disabled */
  disabled?: boolean;
  /** Node type for styling */
  type?: 'file' | 'folder' | 'scope' | 'event' | 'trajectory' | 'turn' | string;
  /** Additional metadata */
  meta?: Record<string, unknown>;
  /** Node metadata (alias for meta) */
  metadata?: Record<string, unknown>;
}

// ═══════════════════════════════════════════════════════════════════════════
// DIFF VIEW TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Diff line type.
 */
export type DiffLineType = 'add' | 'remove' | 'unchanged' | 'header';

/**
 * Diff line data.
 */
export interface DiffLine {
  /** Line type */
  type: DiffLineType;
  /** Line content */
  content: string;
  /** Original line number (for removed/unchanged) */
  oldLineNumber?: number;
  /** New line number (for added/unchanged) */
  newLineNumber?: number;
}

/**
 * Diff view mode.
 */
export type DiffViewMode = 'unified' | 'split' | 'inline';

// ═══════════════════════════════════════════════════════════════════════════
// MEMORY GRAPH TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Graph node for memory visualization.
 */
export interface GraphNode {
  /** Node ID */
  id: string;
  /** Display label */
  label: string;
  /** Node type */
  type: 'trajectory' | 'scope' | 'turn' | 'event' | 'artifact';
  /** Color palette for styling */
  color: import('./modifiers').ColorPalette;
  /** X position */
  x: number;
  /** Y position */
  y: number;
  /** Connected node IDs */
  connections: string[];
  /** Node metadata */
  metadata?: Record<string, unknown>;
}

/**
 * Graph edge for memory visualization.
 */
export interface GraphEdge {
  /** Source node ID */
  source: string;
  /** Target node ID */
  target: string;
  /** Edge label */
  label?: string;
  /** Edge type */
  type?: 'parent' | 'fork' | 'reference';
}

// ═══════════════════════════════════════════════════════════════════════════
// COMMAND PALETTE TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Command for command palette.
 */
export interface Command {
  /** Command ID */
  id: string;
  /** Display label */
  label: string;
  /** Optional description */
  description?: string;
  /** Keyboard shortcut */
  shortcut?: string;
  /** Icon name */
  icon?: string;
  /** Command category */
  category?: string;
  /** Whether command is disabled */
  disabled?: boolean;
  /** Action to execute */
  action: () => void | Promise<void>;
}

// ═══════════════════════════════════════════════════════════════════════════
// MCP PROMPT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * MCP Prompt argument.
 */
export interface PromptArgument {
  /** Argument name */
  name: string;
  /** Argument description */
  description?: string;
  /** Whether argument is required */
  required?: boolean;
}

/**
 * MCP Prompt definition.
 */
export interface Prompt {
  /** Prompt name */
  name: string;
  /** Prompt description */
  description?: string;
  /** Prompt arguments */
  arguments?: PromptArgument[];
}

/**
 * MCP Resource definition.
 */
export interface MCPResource {
  /** Resource URI */
  uri: string;
  /** Resource name */
  name: string;
  /** Resource description */
  description?: string;
  /** MIME type */
  mimeType?: string;
}
