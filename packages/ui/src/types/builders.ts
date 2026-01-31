/**
 * ═══════════════════════════════════════════════════════════════════════════
 * BUILDER PATTERN WITH FLUENT API
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * This module implements the Builder pattern with method chaining for
 * constructing component configurations. The fluent API allows for
 * expressive, readable component configuration.
 *
 * Key features:
 * 1. Method chaining - each method returns `this` for chaining
 * 2. Conditional methods - `.when()` for conditional configuration
 * 3. Type-safe build - `.build()` returns the final typed props
 * 4. Immutable by default - each method creates a new builder
 *
 * Theory: The Builder pattern separates construction of a complex object
 * from its representation. This implementation uses a "fluent interface"
 * pattern where methods return `this` to enable chaining.
 *
 * @example
 * ```ts
 * const button = ButtonBuilder.create()
 *   .color('teal')
 *   .size('lg')
 *   .glow('pulse')
 *   .when(isLoading, b => b.loading())
 *   .when(hasError, b => b.color('coral'))
 *   .build();
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
  AnimateIn,
  AnimateOut,
} from './modifiers';
import type {
  ButtonProps,
  InputProps,
  CardProps,
  BadgeProps,
  ModalProps,
  StyledProps,
  AspectFlags,
} from './props';
import type { ButtonVariantKind } from './unions';

// ═══════════════════════════════════════════════════════════════════════════
// BUILDER BASE CLASS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Base builder with common styling methods.
 * Extended by component-specific builders.
 *
 * @template T - The final props type this builder produces
 * @template Self - Self-type for fluent API (CRTP pattern)
 */
export abstract class BaseBuilder<T extends StyledProps, Self extends BaseBuilder<T, Self>> {
  protected props: Partial<T>;

  protected constructor(initial: Partial<T> = {}) {
    this.props = { ...initial };
  }

  /**
   * Returns `this` typed as the concrete builder type.
   * Enables method chaining in subclasses.
   */
  protected abstract self(): Self;

  /**
   * Builds the final props object.
   * Must be implemented by subclasses.
   */
  abstract build(): T;

  // ─────────────────────────────────────────────────────────────────────────
  // STYLING METHODS
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Sets the color token.
   */
  color(value: ColorToken): Self {
    this.props.color = value;
    return this.self();
  }

  /**
   * Sets the size.
   */
  size(value: Size): Self {
    this.props.size = value;
    return this.self();
  }

  /**
   * Sets the glow effect.
   */
  glow(value: GlowEffect = true): Self {
    this.props.glow = value;
    return this.self();
  }

  /**
   * Sets the glass effect.
   */
  glass(value: GlassEffect = true): Self {
    this.props.glass = value;
    return this.self();
  }

  /**
   * Sets the border effect.
   */
  border(value: BorderEffect = true): Self {
    this.props.border = value;
    return this.self();
  }

  /**
   * Sets the border radius.
   */
  rounded(value: Rounded): Self {
    this.props.rounded = value;
    return this.self();
  }

  /**
   * Sets the shadow.
   */
  shadow(value: Shadow): Self {
    this.props.shadow = value;
    return this.self();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // INTERACTION METHODS
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Sets the hover effect.
   */
  hover(value: HoverEffect): Self {
    this.props.hover = value;
    return this.self();
  }

  /**
   * Sets the press effect.
   */
  press(value: PressEffect): Self {
    this.props.press = value;
    return this.self();
  }

  /**
   * Sets the focus effect.
   */
  focus(value: FocusEffect): Self {
    this.props.focus = value;
    return this.self();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // ASPECT METHODS
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Sets loading state.
   */
  loading(value = true): Self {
    this.props.loading = value;
    return this.self();
  }

  /**
   * Sets disabled state.
   */
  disabled(value = true): Self {
    this.props.disabled = value;
    return this.self();
  }

  /**
   * Sets error state.
   */
  error(value = true): Self {
    this.props.error = value;
    return this.self();
  }

  /**
   * Sets success state.
   */
  success(value = true): Self {
    this.props.success = value;
    return this.self();
  }

  /**
   * Sets selected state.
   */
  selected(value = true): Self {
    this.props.selected = value;
    return this.self();
  }

  /**
   * Sets active state.
   */
  active(value = true): Self {
    this.props.active = value;
    return this.self();
  }

  /**
   * Sets hidden state.
   */
  hidden(value = true): Self {
    this.props.hidden = value;
    return this.self();
  }

  /**
   * Sets full width.
   */
  fullWidth(value = true): Self {
    this.props.fullWidth = value;
    return this.self();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // ANIMATION METHODS
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Sets animate-in effect.
   */
  animateIn(value: AnimateIn): Self {
    this.props.animateIn = value;
    return this.self();
  }

  /**
   * Sets animate-out effect.
   */
  animateOut(value: AnimateOut): Self {
    this.props.animateOut = value;
    return this.self();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // CONDITIONAL METHODS
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Conditionally applies modifications.
   * Only executes the callback if condition is true.
   *
   * @example
   * ```ts
   * builder
   *   .when(isLoading, b => b.loading())
   *   .when(hasError, b => b.color('coral').glow('intense'))
   * ```
   */
  when(condition: boolean, fn: (builder: Self) => Self): Self {
    if (condition) {
      return fn(this.self());
    }
    return this.self();
  }

  /**
   * Conditionally applies one of two modifications.
   *
   * @example
   * ```ts
   * builder.either(
   *   isDarkMode,
   *   b => b.color('slate'),
   *   b => b.color('ghost')
   * )
   * ```
   */
  either(
    condition: boolean,
    ifTrue: (builder: Self) => Self,
    ifFalse: (builder: Self) => Self
  ): Self {
    return condition ? ifTrue(this.self()) : ifFalse(this.self());
  }

  /**
   * Applies modifications from a partial props object.
   */
  merge(partial: Partial<T>): Self {
    this.props = { ...this.props, ...partial };
    return this.self();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // CUSTOM CLASS/STYLE
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Adds custom CSS classes.
   */
  addClass(value: string): Self {
    const existing = this.props.class || '';
    this.props.class = existing ? `${existing} ${value}` : value;
    return this.self();
  }

  /**
   * Adds inline styles.
   */
  addStyle(value: string): Self {
    const existing = this.props.style || '';
    this.props.style = existing ? `${existing}; ${value}` : value;
    return this.self();
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// BUTTON BUILDER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Builder for Button component props.
 */
export class ButtonBuilder extends BaseBuilder<ButtonProps, ButtonBuilder> {
  private constructor(initial: Partial<ButtonProps> = {}) {
    super(initial);
  }

  protected self(): ButtonBuilder {
    return this;
  }

  /**
   * Creates a new ButtonBuilder.
   */
  static create(initial: Partial<ButtonProps> = {}): ButtonBuilder {
    return new ButtonBuilder(initial);
  }

  /**
   * Sets the button variant.
   */
  variant(value: ButtonVariantKind): ButtonBuilder {
    this.props.variant = value;
    return this;
  }

  /**
   * Sets solid variant (shorthand).
   */
  solid(): ButtonBuilder {
    return this.variant('solid');
  }

  /**
   * Sets outline variant (shorthand).
   */
  outline(): ButtonBuilder {
    return this.variant('outline');
  }

  /**
   * Sets ghost variant (shorthand).
   */
  ghost(): ButtonBuilder {
    return this.variant('ghost');
  }

  /**
   * Sets link variant (shorthand).
   */
  link(): ButtonBuilder {
    return this.variant('link');
  }

  /**
   * Sets the button type.
   */
  type(value: 'button' | 'submit' | 'reset'): ButtonBuilder {
    this.props.type = value;
    return this;
  }

  /**
   * Adds left icon.
   */
  iconLeft(value: string): ButtonBuilder {
    this.props.iconLeft = value;
    return this;
  }

  /**
   * Adds right icon.
   */
  iconRight(value: string): ButtonBuilder {
    this.props.iconRight = value;
    return this;
  }

  /**
   * Sets click handler.
   */
  onClick(handler: (event: MouseEvent) => void): ButtonBuilder {
    this.props.onclick = handler;
    return this;
  }

  /**
   * Builds the final ButtonProps.
   */
  build(): ButtonProps {
    return {
      variant: 'solid',
      type: 'button',
      size: 'md',
      color: 'teal',
      hover: 'lift',
      press: 'sink',
      focus: 'ring',
      rounded: 'lg',
      ...this.props,
    } as ButtonProps;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// INPUT BUILDER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Builder for Input component props.
 */
export class InputBuilder extends BaseBuilder<InputProps, InputBuilder> {
  private constructor(initial: Partial<InputProps> = {}) {
    super(initial);
  }

  protected self(): InputBuilder {
    return this;
  }

  /**
   * Creates a new InputBuilder.
   */
  static create(initial: Partial<InputProps> = {}): InputBuilder {
    return new InputBuilder(initial);
  }

  /**
   * Sets the input type.
   */
  type(value: InputProps['type']): InputBuilder {
    this.props.type = value;
    return this;
  }

  /**
   * Sets placeholder text.
   */
  placeholder(value: string): InputBuilder {
    this.props.placeholder = value;
    return this;
  }

  /**
   * Sets input name.
   */
  name(value: string): InputBuilder {
    this.props.name = value;
    return this;
  }

  /**
   * Sets current value.
   */
  value(value: string): InputBuilder {
    this.props.value = value;
    return this;
  }

  /**
   * Sets prefix content.
   */
  prefix(value: string): InputBuilder {
    this.props.prefix = value;
    return this;
  }

  /**
   * Sets suffix content.
   */
  suffix(value: string): InputBuilder {
    this.props.suffix = value;
    return this;
  }

  /**
   * Marks as required.
   */
  required(value = true): InputBuilder {
    this.props.required = value;
    return this;
  }

  /**
   * Sets min/max length constraints.
   */
  length(min?: number, max?: number): InputBuilder {
    if (min !== undefined) this.props.minlength = min;
    if (max !== undefined) this.props.maxlength = max;
    return this;
  }

  /**
   * Sets validation pattern.
   */
  pattern(value: string): InputBuilder {
    this.props.pattern = value;
    return this;
  }

  /**
   * Sets input handler.
   */
  onInput(handler: (event: Event) => void): InputBuilder {
    this.props.oninput = handler;
    return this;
  }

  /**
   * Builds the final InputProps.
   */
  build(): InputProps {
    return {
      type: 'text',
      size: 'md',
      glass: 'subtle',
      border: 'subtle',
      rounded: 'lg',
      focus: 'ring',
      ...this.props,
    } as InputProps;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// CARD BUILDER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Builder for Card component props.
 */
export class CardBuilder extends BaseBuilder<CardProps, CardBuilder> {
  private constructor(initial: Partial<CardProps> = {}) {
    super(initial);
  }

  protected self(): CardBuilder {
    return this;
  }

  /**
   * Creates a new CardBuilder.
   */
  static create(initial: Partial<CardProps> = {}): CardBuilder {
    return new CardBuilder(initial);
  }

  /**
   * Sets padding size.
   */
  padding(value: Size): CardBuilder {
    this.props.padding = value;
    return this;
  }

  /**
   * Makes card clickable.
   */
  clickable(value = true): CardBuilder {
    this.props.clickable = value;
    if (value && !this.props.hover) {
      this.props.hover = 'lift';
    }
    return this;
  }

  /**
   * Sets click handler.
   */
  onClick(handler: (event: MouseEvent) => void): CardBuilder {
    this.props.onclick = handler;
    return this.clickable();
  }

  /**
   * Builds the final CardProps.
   */
  build(): CardProps {
    return {
      glass: 'medium',
      border: 'subtle',
      rounded: 'xl',
      shadow: 'md',
      padding: 'md',
      ...this.props,
    } as CardProps;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// BADGE BUILDER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Builder for Badge component props.
 */
export class BadgeBuilder extends BaseBuilder<BadgeProps, BadgeBuilder> {
  private constructor(initial: Partial<BadgeProps> = {}) {
    super(initial);
  }

  protected self(): BadgeBuilder {
    return this;
  }

  /**
   * Creates a new BadgeBuilder.
   */
  static create(initial: Partial<BadgeProps> = {}): BadgeBuilder {
    return new BadgeBuilder(initial);
  }

  /**
   * Sets badge text.
   */
  text(value: string): BadgeBuilder {
    this.props.text = value;
    return this;
  }

  /**
   * Shows status dot.
   */
  dot(value = true): BadgeBuilder {
    this.props.dot = value;
    return this;
  }

  /**
   * Makes badge removable.
   */
  removable(value = true): BadgeBuilder {
    this.props.removable = value;
    return this;
  }

  /**
   * Sets remove handler.
   */
  onRemove(handler: () => void): BadgeBuilder {
    this.props.onremove = handler;
    return this.removable();
  }

  /**
   * Builds the final BadgeProps.
   */
  build(): BadgeProps {
    return {
      size: 'sm',
      color: 'slate',
      rounded: 'full',
      ...this.props,
    } as BadgeProps;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// MODAL BUILDER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Builder for Modal component props.
 */
export class ModalBuilder extends BaseBuilder<ModalProps, ModalBuilder> {
  private constructor(initial: Partial<ModalProps> = {}) {
    super(initial);
  }

  protected self(): ModalBuilder {
    return this;
  }

  /**
   * Creates a new ModalBuilder.
   */
  static create(initial: Partial<ModalProps> = {}): ModalBuilder {
    return new ModalBuilder(initial);
  }

  /**
   * Opens the modal.
   */
  open(value = true): ModalBuilder {
    this.props.open = value;
    return this;
  }

  /**
   * Sets modal size.
   */
  modalSize(value: ModalProps['modalSize']): ModalBuilder {
    this.props.modalSize = value;
    return this;
  }

  /**
   * Sets modal title.
   */
  title(value: string): ModalBuilder {
    this.props.title = value;
    return this;
  }

  /**
   * Sets whether modal can be closed.
   */
  closable(value = true): ModalBuilder {
    this.props.closable = value;
    return this;
  }

  /**
   * Sets close handler.
   */
  onClose(handler: () => void): ModalBuilder {
    this.props.onclose = handler;
    return this;
  }

  /**
   * Builds the final ModalProps.
   */
  build(): ModalProps {
    return {
      modalSize: 'md',
      glass: 'frosted',
      border: 'subtle',
      rounded: 'xl',
      closable: true,
      ...this.props,
    } as ModalProps;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// BUILDER FACTORY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Factory for creating component builders.
 * Provides a unified entry point for all builders.
 */
export const Builder = {
  /**
   * Creates a Button builder.
   */
  button: (initial?: Partial<ButtonProps>) => ButtonBuilder.create(initial),

  /**
   * Creates an Input builder.
   */
  input: (initial?: Partial<InputProps>) => InputBuilder.create(initial),

  /**
   * Creates a Card builder.
   */
  card: (initial?: Partial<CardProps>) => CardBuilder.create(initial),

  /**
   * Creates a Badge builder.
   */
  badge: (initial?: Partial<BadgeProps>) => BadgeBuilder.create(initial),

  /**
   * Creates a Modal builder.
   */
  modal: (initial?: Partial<ModalProps>) => ModalBuilder.create(initial),
} as const;

// ═══════════════════════════════════════════════════════════════════════════
// UTILITY TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Extracts the props type from a builder.
 */
export type PropsFromBuilder<B> = B extends BaseBuilder<infer T, infer _> ? T : never;

/**
 * Type for a builder configuration function.
 */
export type BuilderFn<B extends BaseBuilder<StyledProps, B>> = (builder: B) => B;
