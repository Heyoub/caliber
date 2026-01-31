/**
 * ═══════════════════════════════════════════════════════════════════════════
 * CALIBER UI TYPE SYSTEM
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * Core type system for CALIBER's UI library with maximum type theory magic.
 *
 * This module re-exports all type definitions organized into categories:
 *
 * 1. **Brands** - Nominal/branded types for type-safe IDs and units
 * 2. **Unions** - Discriminated unions (Rust-like enums)
 * 3. **Modifiers** - Design system tokens and modifier types
 * 4. **Structs** - C-like struct patterns for components
 * 5. **Props** - Component prop interfaces
 * 6. **Builders** - Fluent API builder pattern
 * 7. **Phantom** - Compile-time state tracking
 *
 * Design Principles:
 * - Zero `any` types
 * - Template literal types for CSS classes
 * - `as const` everywhere for literal inference
 * - Make invalid states unrepresentable
 * - Exhaustive pattern matching with assertNever
 */

// ═══════════════════════════════════════════════════════════════════════════
// BRANDED/NOMINAL TYPES
// ═══════════════════════════════════════════════════════════════════════════

export type {
  Brand,
  Unbrand,
  // Entity IDs
  TrajectoryId,
  ScopeId,
  TurnId,
  ArtifactId,
  NoteId,
  SessionId,
  UserId,
  // CSS Units
  CSSValue,
  Pixels,
  Rem,
  Percent,
  ViewportWidth,
  ViewportHeight,
  Em,
  CSSLength,
  // Timestamps
  TimestampMs,
  DurationMs,
} from './brands';

export {
  // ID factories
  trajectoryId,
  scopeId,
  turnId,
  artifactId,
  noteId,
  sessionId,
  userId,
  // CSS unit factories
  px,
  rem,
  percent,
  vw,
  vh,
  em,
  cssValue,
  isCSSValue,
  addCSS,
  scaleCSS,
  // Timestamp factories
  timestampMs,
  durationMs,
  now,
} from './brands';

// ═══════════════════════════════════════════════════════════════════════════
// DISCRIMINATED UNIONS
// ═══════════════════════════════════════════════════════════════════════════

export type {
  // Content blocks
  TextBlock,
  ImageBlock,
  ResourceBlock,
  ToolUseBlock,
  ToolResultBlock,
  ContentBlock,
  ContentBlockType,
  // Button variants
  SolidButton,
  OutlineButton,
  GhostButton,
  LinkButton,
  ButtonVariant,
  ButtonVariantKind,
  // Tool call status
  PendingStatus,
  ApprovedStatus,
  RunningStatus,
  SuccessStatus,
  ErrorStatus,
  RejectedStatus,
  ToolCallStatus,
  ToolCallStatusType,
  // Editor modes
  EditMode,
  PreviewMode,
  SplitMode,
  DiffMode,
  EditorMode,
  EditorModeType,
  // File types
  MarkdownFile,
  YamlFile,
  TomlFile,
  JsonFile,
  XmlFile,
  CsvFile,
  FileType,
  FileTypeKind,
  FileExtension,
  // Notifications
  InfoNotification,
  SuccessNotification,
  WarningNotification,
  ErrorNotification,
  Notification,
  // Layouts
  FullLayout,
  SidebarLayout,
  ThreePanelLayout,
  GridLayout,
  LayoutVariant,
  // Async states
  IdleState,
  LoadingState,
  SuccessState,
  FailureState,
  AsyncState,
  // Option type
  None,
  Some,
  Option,
  // Result type
  Err,
  Ok,
  Result,
} from './unions';

export {
  // Async state guards
  isSuccess,
  isFailure,
  isLoading,
  // Option functions
  some,
  none,
  isSome,
  isNone,
  mapOption,
  // Result functions
  ok,
  err,
  isOk,
  isErr,
  mapResult,
  unwrap,
  unwrapOr,
} from './unions';

// ═══════════════════════════════════════════════════════════════════════════
// MODIFIER SYSTEM
// ═══════════════════════════════════════════════════════════════════════════

export type {
  // Size system
  Size,
  Spacing,
  // Color system
  ColorPalette,
  ColorIntensity,
  ColorToken,
  SemanticColor,
  // Effects
  GlowEffect,
  GlowIntensity,
  GlassEffect,
  GlassIntensity,
  BorderEffect,
  BorderStyle,
  // Interactions
  HoverEffect,
  PressEffect,
  FocusEffect,
  // Animation
  AnimateIn,
  AnimateOut,
  Duration,
  Easing,
  // Layout
  FlexAlign,
  FlexJustify,
  FlexDirection,
  // Typography
  FontFamily,
  FontWeight,
  FontSize,
  TextAlign,
  TextTransform,
  // Shape
  Rounded,
  RoundedSize,
  Shadow,
  ShadowSize,
  Placement,
  Orientation,
  // Template literal classes
  PaddingClass,
  MarginClass,
  GapClass,
  TextColorClass,
  BgColorClass,
  BorderColorClass,
  GlowClass,
  GlassClass,
  TextSizeClass,
  FontWeightClass,
  SpacingClass,
  ColorClass,
  ModifierClassMap,
  ModifierClass,
  ModifierConfig,
} from './modifiers';

export {
  // Const arrays
  SIZES,
  SPACING_VALUES,
  COLOR_PALETTE,
  COLOR_INTENSITIES,
  SEMANTIC_COLORS,
  GLOW_EFFECTS,
  GLASS_EFFECTS,
  BORDER_EFFECTS,
  HOVER_EFFECTS,
  PRESS_EFFECTS,
  FOCUS_EFFECTS,
  ANIMATE_IN,
  ANIMATE_OUT,
  DURATIONS,
  EASINGS,
  FLEX_ALIGN,
  FLEX_JUSTIFY,
  FLEX_DIRECTION,
  FONT_FAMILY,
  FONT_WEIGHT,
  FONT_SIZE,
  TEXT_ALIGN,
  TEXT_TRANSFORM,
  ROUNDED,
  SHADOWS,
  PLACEMENTS,
  ORIENTATIONS,
  // Class mappings
  SIZE_CLASSES,
  GLOW_CLASSES,
  GLASS_CLASSES,
  HOVER_CLASSES,
  PRESS_CLASSES,
  // Utilities
  assertNever,
  isSize,
  isColorPalette,
  isColorToken,
  // Resolvers
  resolveGlow,
  resolveGlass,
  resolveSize,
  resolveHover,
  resolvePress,
  resolveColor,
  modifiersToClasses,
} from './modifiers';

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENT STRUCTS
// ═══════════════════════════════════════════════════════════════════════════

export type {
  // Tags
  ComponentTag,
  // State types
  InteractionState,
  AspectState,
  // Struct types
  ButtonStruct,
  ButtonInit,
  IconButtonStruct,
  IconButtonInit,
  InputStruct,
  InputInit,
  InputType,
  TextAreaStruct,
  TextAreaInit,
  SelectStruct,
  SelectInit,
  SelectOption,
  CheckboxStruct,
  CheckboxInit,
  ToggleStruct,
  ToggleInit,
  BadgeStruct,
  BadgeInit,
  AvatarStruct,
  AvatarInit,
  AvatarStatus,
  CardStruct,
  CardInit,
  ModalStruct,
  ModalInit,
  ModalSize,
  TooltipStruct,
  TooltipInit,
  TooltipPosition,
  DropdownStruct,
  DropdownInit,
  DropdownItem,
  DropdownGroup,
  ColorConfig,
  ComponentStruct,
  StructByTag,
} from './structs';

export {
  // Default states
  DEFAULT_INTERACTION_STATE,
  DEFAULT_ASPECT_STATE,
  // Default structs
  DEFAULT_BUTTON,
  DEFAULT_ICON_BUTTON,
  DEFAULT_INPUT,
  DEFAULT_TEXT_AREA,
  DEFAULT_SELECT,
  DEFAULT_CHECKBOX,
  DEFAULT_TOGGLE,
  DEFAULT_BADGE,
  DEFAULT_AVATAR,
  DEFAULT_CARD,
  DEFAULT_MODAL,
  DEFAULT_TOOLTIP,
  DEFAULT_DROPDOWN,
  // Factory functions
  createButton,
  createButtonStruct,
  createIconButton,
  createInput,
  createInputStruct,
  createTextArea,
  createSelect,
  createCheckbox,
  createToggle,
  createToggleStruct,
  createBadge,
  createBadgeStruct,
  createAvatar,
  createAvatarStruct,
  createCard,
  createModal,
  createTooltip,
  createDropdown,
  // Color configs
  COLOR_CONFIGS,
} from './structs';

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENT PROPS
// ═══════════════════════════════════════════════════════════════════════════

export type {
  // Aspect flags
  StateAspects,
  VisibilityAspects,
  InteractionAspects,
  ContentAspects,
  LayoutAspects,
  AnimationAspects,
  AspectFlags,
  // Base props
  BaseProps,
  InteractiveProps,
  StyledProps,
  SlottedProps,
  FormFieldProps,
  // Typography & Layout props
  TypographyProps,
  LayoutProps,
  // Event handlers
  ClickHandler,
  FocusHandlers,
  InputHandlers,
  KeyboardHandlers,
  EventHandler,
  DragHandler,
  // Button variant props (ButtonVariantKind exported from unions)
  SolidButtonProps,
  OutlineButtonProps,
  GhostButtonProps,
  LinkButtonProps,
  ButtonVariantProps,
  ButtonPropsForVariant,
  // Component props
  ButtonProps,
  IconButtonProps,
  InputProps,
  TextAreaProps,
  SelectOption as SelectOptionProps,
  SelectProps,
  CheckboxProps,
  ToggleProps,
  BadgeProps,
  AvatarProps,
  CardProps,
  ModalProps,
  TooltipProps,
  IconProps,
  SpinnerProps,
  DividerProps,
  // Utility types
  WithRequired,
  PartialExcept,
  ExtractAspects,
  WithoutAspects,
  WithChildren,
  WithSlots,
  ComponentPropMap,
  PropsFor,
  // Application types
  CMSContent,
  ChatMessageData,
  ToolCall,
  ToolCallStatusLiteral,
  ToolResult,
  EditorTab,
  EditorPosition,
  FileFormat,
  FileFormatLiteral,
  FileFormatInfo,
  // Tree view types
  TreeNode,
  TreeNodeValue,
  // Diff view types
  DiffLine,
  DiffLineType,
  DiffViewMode,
  // Memory graph types
  GraphNode,
  GraphEdge,
  // Command palette types
  Command,
  // MCP types
  Prompt,
  PromptArgument,
  MCPResource,
} from './props';

// ═══════════════════════════════════════════════════════════════════════════
// BUILDER PATTERN
// ═══════════════════════════════════════════════════════════════════════════

export type {
  PropsFromBuilder,
  BuilderFn,
} from './builders';

export {
  BaseBuilder,
  ButtonBuilder,
  InputBuilder,
  CardBuilder,
  BadgeBuilder,
  ModalBuilder,
  Builder,
} from './builders';

// ═══════════════════════════════════════════════════════════════════════════
// PHANTOM TYPES
// ═══════════════════════════════════════════════════════════════════════════

export type {
  // Validation states
  Unvalidated,
  Validated,
  // Connection states
  Disconnected,
  Connected,
  // Initialization states
  Uninitialized,
  Initialized,
  // Lock states
  Locked,
  Unlocked,
  // Publication states
  Draft,
  Published,
  // Form with phantom type
  Form,
  FormErrors,
  // Connection with phantom type
  Connection,
  // Resource lifecycle
  Acquired,
  Released,
  Resource,
  // Builder state tracking
  BuilderState,
  Complete,
  Incomplete,
  TypedBuilder,
  // State machine
  StateMachine,
  Transition,
  // Document workflow
  DocumentDraft,
  DocumentReview,
  DocumentApproved,
  DocumentPublished,
  DocumentArchived,
  DocumentWorkflow,
  // Capability-based security
  CanRead,
  CanWrite,
  CanDelete,
  CanAdmin,
  FullAccess,
  ReadOnly,
  ReadWrite,
  ProtectedResource,
} from './phantom';

export {
  createForm,
  createConnection,
  createResource,
  createDocument,
  protect,
} from './phantom';
