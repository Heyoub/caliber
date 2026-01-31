/**
 * ═══════════════════════════════════════════════════════════════════════════
 * DISCRIMINATED UNIONS (TAGGED UNIONS / RUST-LIKE ENUMS)
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * Discriminated unions provide type-safe pattern matching similar to Rust's
 * enums or Haskell's ADTs (Algebraic Data Types). Each variant has a
 * discriminant property (usually 'type' or 'kind') that TypeScript uses
 * to narrow the type in conditionals.
 *
 * Key benefits:
 * 1. Exhaustive checking - compiler errors if you miss a case
 * 2. Type narrowing - accessing variant-specific properties is safe
 * 3. Self-documenting - all possible states are explicit in the type
 *
 * Theory: These are "sum types" in type theory terminology. A value of
 * type A | B | C is exactly one of those types, never multiple or none.
 * Combined with "product types" (objects/tuples), they give us full ADTs.
 *
 * @example
 * ```ts
 * function handleContent(block: ContentBlock): string {
 *   switch (block.type) {
 *     case 'text': return block.text;
 *     case 'image': return `<img src="${block.data}">`;
 *     case 'resource': return `Resource: ${block.uri}`;
 *     case 'tool_use': return `Tool: ${block.name}`;
 *     case 'tool_result': return `Result for ${block.tool_use_id}`;
 *     // TypeScript ensures all cases are handled
 *   }
 * }
 * ```
 */

import type { ColorToken } from './modifiers';

// ═══════════════════════════════════════════════════════════════════════════
// MCP CONTENT BLOCKS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Text content block - plain text or markdown.
 */
export interface TextBlock {
  readonly type: 'text';
  readonly text: string;
}

/**
 * Image content block - base64 encoded image data.
 * The mimeType is constrained to image/* subtypes.
 */
export interface ImageBlock {
  readonly type: 'image';
  readonly data: string;
  readonly mimeType: `image/${string}`;
}

/**
 * Resource content block - reference to a memory resource.
 * URI must follow the memory:// protocol.
 */
export interface ResourceBlock {
  readonly type: 'resource';
  readonly uri: `memory://${string}`;
  readonly mimeType: string;
  readonly text?: string;
}

/**
 * Tool use block - represents a tool invocation by the AI.
 */
export interface ToolUseBlock {
  readonly type: 'tool_use';
  readonly id: string;
  readonly name: string;
  readonly input: Record<string, unknown>;
}

/**
 * Tool result block - the output from a tool execution.
 * References the original tool_use by ID.
 */
export interface ToolResultBlock {
  readonly type: 'tool_result';
  readonly tool_use_id: string;
  readonly content: readonly ContentBlock[];
  readonly is_error?: boolean;
}

/**
 * Union of all MCP content block types.
 * Use pattern matching (switch/if) on the 'type' discriminant.
 */
export type ContentBlock = TextBlock | ImageBlock | ResourceBlock | ToolUseBlock | ToolResultBlock;

/**
 * Extract the discriminant values as a union type.
 * Useful for creating lookup maps or exhaustive checks.
 */
export type ContentBlockType = ContentBlock['type'];

// ═══════════════════════════════════════════════════════════════════════════
// BUTTON VARIANTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Solid button - filled background, primary CTA.
 */
export interface SolidButton {
  readonly kind: 'solid';
  readonly color: ColorToken;
}

/**
 * Outline button - bordered, transparent background.
 */
export interface OutlineButton {
  readonly kind: 'outline';
  readonly color: ColorToken;
  readonly borderWidth?: 1 | 2;
}

/**
 * Ghost button - no border, subtle hover state.
 */
export interface GhostButton {
  readonly kind: 'ghost';
  readonly hoverColor?: ColorToken;
}

/**
 * Link button - appears as inline link.
 */
export interface LinkButton {
  readonly kind: 'link';
  readonly underline?: boolean;
}

/**
 * Union of all button variant types.
 */
export type ButtonVariant = SolidButton | OutlineButton | GhostButton | LinkButton;

/**
 * Button variant discriminant values.
 */
export type ButtonVariantKind = ButtonVariant['kind'];

// ═══════════════════════════════════════════════════════════════════════════
// TOOL CALL STATUS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Tool call is awaiting user approval.
 */
export interface PendingStatus {
  readonly status: 'pending';
  readonly requestedAt: number;
}

/**
 * Tool call was approved by user.
 */
export interface ApprovedStatus {
  readonly status: 'approved';
  readonly approvedAt: number;
  readonly approvedBy?: string;
}

/**
 * Tool call is currently executing.
 */
export interface RunningStatus {
  readonly status: 'running';
  readonly startedAt: number;
  readonly progress?: number; // 0-100
}

/**
 * Tool call completed successfully.
 */
export interface SuccessStatus {
  readonly status: 'success';
  readonly completedAt: number;
  readonly durationMs: number;
}

/**
 * Tool call failed with an error.
 */
export interface ErrorStatus {
  readonly status: 'error';
  readonly failedAt: number;
  readonly error: string;
  readonly code?: string;
}

/**
 * Tool call was rejected by user.
 */
export interface RejectedStatus {
  readonly status: 'rejected';
  readonly rejectedAt: number;
  readonly reason?: string;
}

/**
 * Union of all tool call status types.
 */
export type ToolCallStatus =
  | PendingStatus
  | ApprovedStatus
  | RunningStatus
  | SuccessStatus
  | ErrorStatus
  | RejectedStatus;

/**
 * Tool call status discriminant values.
 */
export type ToolCallStatusType = ToolCallStatus['status'];

// ═══════════════════════════════════════════════════════════════════════════
// EDITOR MODE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Edit mode - CodeMirror editor is active.
 */
export interface EditMode {
  readonly mode: 'edit';
  readonly cursorPosition?: { line: number; column: number };
}

/**
 * Preview mode - rendered view is active.
 */
export interface PreviewMode {
  readonly mode: 'preview';
  readonly scrollPosition?: number;
}

/**
 * Split mode - editor and preview side by side.
 */
export interface SplitMode {
  readonly mode: 'split';
  readonly ratio: number; // 0-1, how much space editor takes
  readonly syncScroll: boolean;
}

/**
 * Diff mode - comparing two versions.
 */
export interface DiffMode {
  readonly mode: 'diff';
  readonly baseContent: string;
  readonly diffStyle: 'unified' | 'split';
}

/**
 * Union of all editor modes.
 */
export type EditorMode = EditMode | PreviewMode | SplitMode | DiffMode;

/**
 * Editor mode discriminant values.
 */
export type EditorModeType = EditorMode['mode'];

// ═══════════════════════════════════════════════════════════════════════════
// FILE TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Markdown file type with specific options.
 */
export interface MarkdownFile {
  readonly type: 'markdown';
  readonly extension: 'md' | 'mdx';
  readonly frontmatter?: boolean;
}

/**
 * YAML file type.
 */
export interface YamlFile {
  readonly type: 'yaml';
  readonly extension: 'yaml' | 'yml';
}

/**
 * TOML file type.
 */
export interface TomlFile {
  readonly type: 'toml';
  readonly extension: 'toml';
}

/**
 * JSON file type with optional schema.
 */
export interface JsonFile {
  readonly type: 'json';
  readonly extension: 'json' | 'jsonc';
  readonly schema?: string;
}

/**
 * XML file type.
 */
export interface XmlFile {
  readonly type: 'xml';
  readonly extension: 'xml';
  readonly namespace?: string;
}

/**
 * CSV file type with delimiter options.
 */
export interface CsvFile {
  readonly type: 'csv';
  readonly extension: 'csv' | 'tsv';
  readonly delimiter: ',' | '\t' | ';';
  readonly hasHeader: boolean;
}

/**
 * Union of all supported file types.
 */
export type FileType = MarkdownFile | YamlFile | TomlFile | JsonFile | XmlFile | CsvFile;

/**
 * File type discriminant values.
 */
export type FileTypeKind = FileType['type'];

/**
 * All supported file extensions.
 */
export type FileExtension = FileType['extension'];

// ═══════════════════════════════════════════════════════════════════════════
// NOTIFICATION TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Info notification - general information.
 */
export interface InfoNotification {
  readonly type: 'info';
  readonly message: string;
  readonly title?: string;
}

/**
 * Success notification - operation completed.
 */
export interface SuccessNotification {
  readonly type: 'success';
  readonly message: string;
  readonly title?: string;
}

/**
 * Warning notification - something needs attention.
 */
export interface WarningNotification {
  readonly type: 'warning';
  readonly message: string;
  readonly title?: string;
  readonly action?: { label: string; href: string };
}

/**
 * Error notification - something went wrong.
 */
export interface ErrorNotification {
  readonly type: 'error';
  readonly message: string;
  readonly title?: string;
  readonly retry?: () => void;
}

/**
 * Union of all notification types.
 */
export type Notification =
  | InfoNotification
  | SuccessNotification
  | WarningNotification
  | ErrorNotification;

// ═══════════════════════════════════════════════════════════════════════════
// LAYOUT VARIANTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Full-width layout.
 */
export interface FullLayout {
  readonly layout: 'full';
}

/**
 * Sidebar + main content layout.
 */
export interface SidebarLayout {
  readonly layout: 'sidebar';
  readonly sidebarPosition: 'left' | 'right';
  readonly sidebarWidth: number;
  readonly collapsible: boolean;
}

/**
 * Three-panel layout (sidebar + main + right panel).
 */
export interface ThreePanelLayout {
  readonly layout: 'three-panel';
  readonly leftWidth: number;
  readonly rightWidth: number;
}

/**
 * Grid layout with configurable columns.
 */
export interface GridLayout {
  readonly layout: 'grid';
  readonly columns: 1 | 2 | 3 | 4;
  readonly gap: number;
}

/**
 * Union of all layout variants.
 */
export type LayoutVariant = FullLayout | SidebarLayout | ThreePanelLayout | GridLayout;

// ═══════════════════════════════════════════════════════════════════════════
// ASYNC OPERATION STATES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Operation has not started.
 */
export interface IdleState {
  readonly state: 'idle';
}

/**
 * Operation is in progress.
 */
export interface LoadingState {
  readonly state: 'loading';
  readonly progress?: number;
}

/**
 * Operation completed successfully with data.
 */
export interface SuccessState<T> {
  readonly state: 'success';
  readonly data: T;
  readonly timestamp: number;
}

/**
 * Operation failed with error.
 */
export interface FailureState {
  readonly state: 'failure';
  readonly error: Error;
  readonly timestamp: number;
}

/**
 * Generic async state - represents any async operation.
 * This is a "Result" type pattern from Rust.
 */
export type AsyncState<T> = IdleState | LoadingState | SuccessState<T> | FailureState;

/**
 * Type guard for success state.
 */
export function isSuccess<T>(state: AsyncState<T>): state is SuccessState<T> {
  return state.state === 'success';
}

/**
 * Type guard for failure state.
 */
export function isFailure<T>(state: AsyncState<T>): state is FailureState {
  return state.state === 'failure';
}

/**
 * Type guard for loading state.
 */
export function isLoading<T>(state: AsyncState<T>): state is LoadingState {
  return state.state === 'loading';
}

// ═══════════════════════════════════════════════════════════════════════════
// OPTION TYPE (MAYBE/OPTIONAL)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Represents absence of value.
 */
export interface None {
  readonly _tag: 'None';
}

/**
 * Represents presence of value.
 */
export interface Some<T> {
  readonly _tag: 'Some';
  readonly value: T;
}

/**
 * Option type - explicit nullable.
 * Prefer this over T | null | undefined for clearer semantics.
 */
export type Option<T> = None | Some<T>;

/**
 * Creates a Some value.
 */
export function some<T>(value: T): Some<T> {
  return { _tag: 'Some', value };
}

/**
 * The singleton None value.
 */
export const none: None = { _tag: 'None' };

/**
 * Type guard for Some.
 */
export function isSome<T>(opt: Option<T>): opt is Some<T> {
  return opt._tag === 'Some';
}

/**
 * Type guard for None.
 */
export function isNone<T>(opt: Option<T>): opt is None {
  return opt._tag === 'None';
}

/**
 * Maps over an Option.
 */
export function mapOption<T, U>(opt: Option<T>, fn: (value: T) => U): Option<U> {
  return isSome(opt) ? some(fn(opt.value)) : none;
}

// ═══════════════════════════════════════════════════════════════════════════
// RESULT TYPE (EITHER/ERROR HANDLING)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Represents a failed operation.
 */
export interface Err<E> {
  readonly _tag: 'Err';
  readonly error: E;
}

/**
 * Represents a successful operation.
 */
export interface Ok<T> {
  readonly _tag: 'Ok';
  readonly value: T;
}

/**
 * Result type - explicit error handling without exceptions.
 * Similar to Rust's Result<T, E>.
 */
export type Result<T, E = Error> = Ok<T> | Err<E>;

/**
 * Creates an Ok result.
 */
export function ok<T>(value: T): Ok<T> {
  return { _tag: 'Ok', value };
}

/**
 * Creates an Err result.
 */
export function err<E>(error: E): Err<E> {
  return { _tag: 'Err', error };
}

/**
 * Type guard for Ok.
 */
export function isOk<T, E>(result: Result<T, E>): result is Ok<T> {
  return result._tag === 'Ok';
}

/**
 * Type guard for Err.
 */
export function isErr<T, E>(result: Result<T, E>): result is Err<E> {
  return result._tag === 'Err';
}

/**
 * Maps over a Result.
 */
export function mapResult<T, U, E>(result: Result<T, E>, fn: (value: T) => U): Result<U, E> {
  return isOk(result) ? ok(fn(result.value)) : result;
}

/**
 * Unwraps a Result or throws.
 */
export function unwrap<T, E>(result: Result<T, E>): T {
  if (isOk(result)) return result.value;
  throw result.error;
}

/**
 * Unwraps a Result or returns default.
 */
export function unwrapOr<T, E>(result: Result<T, E>, defaultValue: T): T {
  return isOk(result) ? result.value : defaultValue;
}
