/**
 * ═══════════════════════════════════════════════════════════════════════════
 * BRANDED/NOMINAL TYPES
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * TypeScript uses structural typing - two types with the same shape are
 * considered equivalent. This is problematic when we have semantically
 * different values that happen to share the same underlying type (strings).
 *
 * Branded types (also called "nominal types" or "opaque types") solve this
 * by adding a phantom property that exists only at compile time. The runtime
 * value is still a plain string/number, but TypeScript treats them as
 * incompatible types.
 *
 * Theory: This is similar to Haskell's newtype or Rust's tuple structs for
 * creating distinct types with zero runtime overhead.
 *
 * @example
 * ```ts
 * const trajectory = trajectoryId('traj_123');
 * const scope = scopeId('scope_456');
 *
 * function loadTrajectory(id: TrajectoryId) { ... }
 *
 * loadTrajectory(trajectory); // OK
 * loadTrajectory(scope);      // Compile error! Can't mix ID types
 * ```
 */

// ═══════════════════════════════════════════════════════════════════════════
// BRAND INFRASTRUCTURE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Unique symbol used as a key for brand properties.
 * This ensures brands can't accidentally collide with real properties.
 */
declare const __brand: unique symbol;

/**
 * Brand type constructor - intersects base type T with a phantom brand B.
 * The brand exists only at compile time; at runtime it's just T.
 *
 * @template T - The underlying runtime type (string, number, etc.)
 * @template B - The brand identifier (a string literal type)
 */
export type Brand<T, B extends string> = T & {
  readonly [__brand]: B;
};

/**
 * Helper type to extract the underlying type from a branded type.
 * Useful when you need to "unwrap" a branded value.
 */
export type Unbrand<T> = T extends Brand<infer U, string> ? U : T;

// ═══════════════════════════════════════════════════════════════════════════
// ENTITY IDENTIFIERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Trajectory ID - identifies a complete task journey through the system.
 * Trajectories are the top-level container for agent interactions.
 */
export type TrajectoryId = Brand<string, 'TrajectoryId'>;

/**
 * Scope ID - identifies an isolated memory partition.
 * Scopes contain turns and can have memory limits.
 */
export type ScopeId = Brand<string, 'ScopeId'>;

/**
 * Turn ID - identifies a single conversational turn.
 * A turn contains the user message and assistant response.
 */
export type TurnId = Brand<string, 'TurnId'>;

/**
 * Artifact ID - identifies a generated artifact (code, file, etc.)
 * Artifacts are outputs produced during a trajectory.
 */
export type ArtifactId = Brand<string, 'ArtifactId'>;

/**
 * Note ID - identifies a semantic note attached to memory.
 * Notes provide human-readable annotations.
 */
export type NoteId = Brand<string, 'NoteId'>;

/**
 * Session ID - identifies an active user session.
 */
export type SessionId = Brand<string, 'SessionId'>;

/**
 * User ID - identifies a user account.
 */
export type UserId = Brand<string, 'UserId'>;

// ═══════════════════════════════════════════════════════════════════════════
// ID FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Creates a TrajectoryId from a raw string.
 * Acts as the single point of entry for trajectory ID creation.
 */
export function trajectoryId(raw: string): TrajectoryId {
  return raw as TrajectoryId;
}

/**
 * Creates a ScopeId from a raw string.
 */
export function scopeId(raw: string): ScopeId {
  return raw as ScopeId;
}

/**
 * Creates a TurnId from a raw string.
 */
export function turnId(raw: string): TurnId {
  return raw as TurnId;
}

/**
 * Creates an ArtifactId from a raw string.
 */
export function artifactId(raw: string): ArtifactId {
  return raw as ArtifactId;
}

/**
 * Creates a NoteId from a raw string.
 */
export function noteId(raw: string): NoteId {
  return raw as NoteId;
}

/**
 * Creates a SessionId from a raw string.
 */
export function sessionId(raw: string): SessionId {
  return raw as SessionId;
}

/**
 * Creates a UserId from a raw string.
 */
export function userId(raw: string): UserId {
  return userId as unknown as UserId;
}

// ═══════════════════════════════════════════════════════════════════════════
// CSS UNIT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Opaque type tag for CSS values.
 * This is separate from the brand symbol to allow for value extraction.
 */
declare const __cssUnit: unique symbol;

/**
 * Base interface for CSS values with unit safety.
 * Contains both the opaque tag and the actual value.
 *
 * @template U - The unit identifier ('px' | 'rem' | '%' | etc.)
 */
export interface CSSValue<U extends string> {
  readonly [__cssUnit]: U;
  readonly value: string;
  readonly raw: number;
}

/**
 * Pixel value - absolute CSS unit.
 * Use for fixed dimensions, borders, shadows.
 */
export type Pixels = CSSValue<'px'>;

/**
 * Rem value - relative to root font size.
 * Use for typography, spacing, component sizing.
 */
export type Rem = CSSValue<'rem'>;

/**
 * Percentage value - relative to parent.
 * Use for fluid layouts, responsive widths.
 */
export type Percent = CSSValue<'%'>;

/**
 * Viewport width value.
 * Use for full-width layouts.
 */
export type ViewportWidth = CSSValue<'vw'>;

/**
 * Viewport height value.
 * Use for full-height layouts.
 */
export type ViewportHeight = CSSValue<'vh'>;

/**
 * Em value - relative to current font size.
 * Use for component-local scaling.
 */
export type Em = CSSValue<'em'>;

/**
 * Any CSS length value - union of all length types.
 */
export type CSSLength = Pixels | Rem | Percent | ViewportWidth | ViewportHeight | Em;

// ═══════════════════════════════════════════════════════════════════════════
// CSS UNIT FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Creates a Pixels value from a number.
 *
 * @example
 * ```ts
 * const width = px(100);     // { value: '100px', raw: 100 }
 * const border = px(1);      // { value: '1px', raw: 1 }
 * ```
 */
export function px(n: number): Pixels {
  return { value: `${n}px`, raw: n } as Pixels;
}

/**
 * Creates a Rem value from a number.
 *
 * @example
 * ```ts
 * const fontSize = rem(1.5); // { value: '1.5rem', raw: 1.5 }
 * const padding = rem(2);    // { value: '2rem', raw: 2 }
 * ```
 */
export function rem(n: number): Rem {
  return { value: `${n}rem`, raw: n } as Rem;
}

/**
 * Creates a Percent value from a number.
 *
 * @example
 * ```ts
 * const width = percent(100); // { value: '100%', raw: 100 }
 * const opacity = percent(50); // { value: '50%', raw: 50 }
 * ```
 */
export function percent(n: number): Percent {
  return { value: `${n}%`, raw: n } as Percent;
}

/**
 * Creates a viewport width value.
 */
export function vw(n: number): ViewportWidth {
  return { value: `${n}vw`, raw: n } as ViewportWidth;
}

/**
 * Creates a viewport height value.
 */
export function vh(n: number): ViewportHeight {
  return { value: `${n}vh`, raw: n } as ViewportHeight;
}

/**
 * Creates an Em value from a number.
 */
export function em(n: number): Em {
  return { value: `${n}em`, raw: n } as Em;
}

// ═══════════════════════════════════════════════════════════════════════════
// UTILITY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Extracts the CSS string value from any CSSValue type.
 * Use this when passing values to style properties.
 */
export function cssValue<U extends string>(v: CSSValue<U>): string {
  return v.value;
}

/**
 * Type guard to check if a value is a CSSValue.
 */
export function isCSSValue(v: unknown): v is CSSValue<string> {
  return (
    typeof v === 'object' &&
    v !== null &&
    'value' in v &&
    'raw' in v &&
    typeof (v as CSSValue<string>).value === 'string' &&
    typeof (v as CSSValue<string>).raw === 'number'
  );
}

/**
 * Adds two CSS values of the same unit type.
 * Returns a new value with the sum.
 */
export function addCSS<U extends string>(
  a: CSSValue<U>,
  b: CSSValue<U>
): CSSValue<U> {
  const unit = a.value.replace(/[\d.-]/g, '');
  return {
    value: `${a.raw + b.raw}${unit}`,
    raw: a.raw + b.raw,
  } as CSSValue<U>;
}

/**
 * Multiplies a CSS value by a scalar.
 */
export function scaleCSS<U extends string>(
  v: CSSValue<U>,
  factor: number
): CSSValue<U> {
  const unit = v.value.replace(/[\d.-]/g, '');
  return {
    value: `${v.raw * factor}${unit}`,
    raw: v.raw * factor,
  } as CSSValue<U>;
}

// ═══════════════════════════════════════════════════════════════════════════
// TIMESTAMP BRANDS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Unix timestamp in milliseconds.
 * Branded to prevent confusion with seconds or other numeric values.
 */
export type TimestampMs = Brand<number, 'TimestampMs'>;

/**
 * Duration in milliseconds.
 */
export type DurationMs = Brand<number, 'DurationMs'>;

/**
 * Creates a millisecond timestamp.
 */
export function timestampMs(ms: number): TimestampMs {
  return ms as TimestampMs;
}

/**
 * Creates a millisecond duration.
 */
export function durationMs(ms: number): DurationMs {
  return ms as DurationMs;
}

/**
 * Gets the current timestamp.
 */
export function now(): TimestampMs {
  return Date.now() as TimestampMs;
}
