/**
 * ═══════════════════════════════════════════════════════════════════════════
 * PHANTOM TYPES FOR COMPILE-TIME STATE TRACKING
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * Phantom types are type parameters that exist only in the type system,
 * not at runtime. They're used to track state transitions at compile time,
 * making invalid state transitions impossible to represent.
 *
 * Theory: Phantom types encode information in the type system that has no
 * runtime representation. This allows the compiler to verify invariants
 * that would otherwise require runtime checks.
 *
 * Common use cases:
 * 1. Form validation states (Validated | Unvalidated)
 * 2. Connection states (Connected | Disconnected)
 * 3. Resource lifecycle (Acquired | Released)
 * 4. Initialization states (Initialized | Uninitialized)
 *
 * @example
 * ```ts
 * // Only validated forms can be submitted
 * const form: Form<Unvalidated> = createForm();
 * form.submit();  // Compile error!
 *
 * const validated = form.validate(); // Form<Validated>
 * validated.submit();  // OK!
 * ```
 */

// ═══════════════════════════════════════════════════════════════════════════
// PHANTOM TYPE MARKERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Marker type for unvalidated state.
 * The readonly brand prevents assignability between markers.
 */
export interface Unvalidated {
  readonly _validated: false;
}

/**
 * Marker type for validated state.
 */
export interface Validated {
  readonly _validated: true;
}

/**
 * Marker type for disconnected state.
 */
export interface Disconnected {
  readonly _connected: false;
}

/**
 * Marker type for connected state.
 */
export interface Connected {
  readonly _connected: true;
}

/**
 * Marker type for uninitialized state.
 */
export interface Uninitialized {
  readonly _initialized: false;
}

/**
 * Marker type for initialized state.
 */
export interface Initialized {
  readonly _initialized: true;
}

/**
 * Marker type for locked state.
 */
export interface Locked {
  readonly _locked: true;
}

/**
 * Marker type for unlocked state.
 */
export interface Unlocked {
  readonly _locked: false;
}

/**
 * Marker type for draft state.
 */
export interface Draft {
  readonly _published: false;
}

/**
 * Marker type for published state.
 */
export interface Published {
  readonly _published: true;
}

// ═══════════════════════════════════════════════════════════════════════════
// FORM WITH VALIDATION STATE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Form errors type.
 */
export interface FormErrors {
  readonly [field: string]: string | undefined;
}

/**
 * Form with phantom type for validation state.
 * Submit is only callable on validated forms.
 *
 * @template S - The validation state (Validated | Unvalidated)
 * @template T - The shape of the form values
 */
export interface Form<S, T extends Record<string, unknown> = Record<string, unknown>> {
  /** The form values */
  readonly values: T;

  /** Validation errors (empty when validated) */
  readonly errors: FormErrors;

  /** Whether the form has been modified */
  readonly dirty: boolean;

  /** Whether the form has been touched */
  readonly touched: Record<keyof T, boolean>;

  /**
   * Updates a field value.
   * Returns a new unvalidated form.
   */
  setField<K extends keyof T>(field: K, value: T[K]): Form<Unvalidated, T>;

  /**
   * Validates the form.
   * Returns either a validated form or an unvalidated form with errors.
   */
  validate(): Form<Validated, T> | Form<Unvalidated, T>;

  /**
   * Submits the form.
   * Only callable on validated forms due to conditional type.
   */
  submit: S extends Validated ? () => Promise<void> : never;

  /**
   * Resets the form to initial state.
   */
  reset(): Form<Unvalidated, T>;
}

/**
 * Creates an unvalidated form with initial values.
 */
export function createForm<T extends Record<string, unknown>>(
  initial: T
): Form<Unvalidated, T> {
  const touched = Object.keys(initial).reduce(
    (acc, key) => ({ ...acc, [key]: false }),
    {} as Record<keyof T, boolean>
  );

  return {
    values: initial,
    errors: {},
    dirty: false,
    touched,
    setField(field, value) {
      return createForm({ ...initial, [field]: value });
    },
    validate() {
      // Actual validation logic would go here
      // For now, always returns validated
      return this as unknown as Form<Validated, T>;
    },
    submit: undefined as never,
    reset() {
      return createForm(initial);
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// CONNECTION WITH STATE TRACKING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Connection with phantom type for connection state.
 * Send/receive are only callable on connected connections.
 *
 * @template S - The connection state (Connected | Disconnected)
 * @template T - The message type
 */
export interface Connection<S, T = unknown> {
  /** The connection URL */
  readonly url: string;

  /** Current connection state */
  readonly state: S extends Connected ? 'connected' : 'disconnected';

  /**
   * Connects to the server.
   * Returns a connected connection.
   */
  connect(): Promise<Connection<Connected, T>>;

  /**
   * Disconnects from the server.
   * Returns a disconnected connection.
   */
  disconnect(): Promise<Connection<Disconnected, T>>;

  /**
   * Sends a message.
   * Only callable on connected connections.
   */
  send: S extends Connected ? (message: T) => Promise<void> : never;

  /**
   * Receives a message.
   * Only callable on connected connections.
   */
  receive: S extends Connected ? () => Promise<T> : never;

  /**
   * Registers a message handler.
   * Only callable on connected connections.
   */
  onMessage: S extends Connected ? (handler: (message: T) => void) => void : never;
}

/**
 * Creates a disconnected connection.
 */
export function createConnection<T>(url: string): Connection<Disconnected, T> {
  return {
    url,
    state: 'disconnected',
    async connect() {
      // Actual connection logic would go here
      return {
        url,
        state: 'connected',
        async connect() { return this; },
        async disconnect() { return createConnection<T>(url); },
        send: async () => {},
        receive: async () => ({} as T),
        onMessage: () => {},
      } as Connection<Connected, T>;
    },
    async disconnect() { return this; },
    send: undefined as never,
    receive: undefined as never,
    onMessage: undefined as never,
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// RESOURCE LIFECYCLE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Marker for acquired resource.
 */
export interface Acquired {
  readonly _acquired: true;
}

/**
 * Marker for released resource.
 */
export interface Released {
  readonly _acquired: false;
}

/**
 * Resource with phantom type for lifecycle state.
 * Use is only callable on acquired resources.
 *
 * @template S - The resource state (Acquired | Released)
 * @template T - The resource type
 */
export interface Resource<S, T> {
  /** The resource handle */
  readonly handle: T | null;

  /**
   * Acquires the resource.
   */
  acquire(): Promise<Resource<Acquired, T>>;

  /**
   * Releases the resource.
   */
  release(): Promise<Resource<Released, T>>;

  /**
   * Uses the resource.
   * Only callable on acquired resources.
   */
  use: S extends Acquired ? (fn: (resource: T) => void) => void : never;
}

/**
 * Creates a released resource.
 */
export function createResource<T>(
  acquireFn: () => Promise<T>,
  releaseFn: (resource: T) => Promise<void>
): Resource<Released, T> {
  return {
    handle: null,
    async acquire() {
      const handle = await acquireFn();
      return {
        handle,
        async acquire() { return this; },
        async release() {
          await releaseFn(handle);
          return createResource(acquireFn, releaseFn);
        },
        use(fn) { fn(handle); },
      } as Resource<Acquired, T>;
    },
    async release() { return this; },
    use: undefined as never,
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// BUILDER STATE TRACKING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Builder state with required fields tracking.
 * Uses phantom type to track which required fields have been set.
 */
export interface BuilderState<Required extends string> {
  readonly _required: Required;
}

/**
 * Complete builder state - all required fields set.
 */
export type Complete = BuilderState<never>;

/**
 * Incomplete builder state - some required fields missing.
 */
export type Incomplete<Missing extends string> = BuilderState<Missing>;

/**
 * Type-safe builder that tracks required fields.
 *
 * @template S - Current builder state (which fields are still required)
 * @template T - The type being built
 */
export interface TypedBuilder<S extends BuilderState<string>, T> {
  /** Current partial value */
  readonly current: Partial<T>;

  /**
   * Sets a field.
   * Removes the field from required list if it was required.
   */
  set<K extends keyof T>(
    field: K,
    value: T[K]
  ): K extends S['_required']
    ? TypedBuilder<BuilderState<Exclude<S['_required'], K>>, T>
    : TypedBuilder<S, T>;

  /**
   * Builds the final value.
   * Only callable when all required fields are set.
   */
  build: S extends Complete ? () => T : never;
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE MACHINE WITH PHANTOM TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Generic state machine with phantom type state tracking.
 * Transitions are type-checked at compile time.
 *
 * @template S - Current state marker
 * @template T - The data payload
 */
export interface StateMachine<S, T> {
  /** Current data */
  readonly data: T;

  /** Current state name */
  readonly stateName: string;
}

/**
 * Transition function type.
 * Defines valid state transitions.
 */
export type Transition<From, To, T, Args extends unknown[] = []> = (
  machine: StateMachine<From, T>,
  ...args: Args
) => StateMachine<To, T>;

/**
 * Example: Document workflow states.
 */
export interface DocumentDraft { readonly _state: 'draft'; }
export interface DocumentReview { readonly _state: 'review'; }
export interface DocumentApproved { readonly _state: 'approved'; }
export interface DocumentPublished { readonly _state: 'published'; }
export interface DocumentArchived { readonly _state: 'archived'; }

/**
 * Document workflow with typed transitions.
 */
export interface DocumentWorkflow<S> extends StateMachine<S, { content: string }> {
  /** Submit for review - only from draft */
  submitForReview: S extends DocumentDraft
    ? () => DocumentWorkflow<DocumentReview>
    : never;

  /** Approve - only from review */
  approve: S extends DocumentReview
    ? () => DocumentWorkflow<DocumentApproved>
    : never;

  /** Reject back to draft - only from review */
  reject: S extends DocumentReview
    ? (reason: string) => DocumentWorkflow<DocumentDraft>
    : never;

  /** Publish - only from approved */
  publish: S extends DocumentApproved
    ? () => DocumentWorkflow<DocumentPublished>
    : never;

  /** Archive - from published */
  archive: S extends DocumentPublished
    ? () => DocumentWorkflow<DocumentArchived>
    : never;

  /** Edit - back to draft from review or approved */
  edit: S extends DocumentReview | DocumentApproved
    ? () => DocumentWorkflow<DocumentDraft>
    : never;
}

/**
 * Creates a new document in draft state.
 */
export function createDocument(content: string): DocumentWorkflow<DocumentDraft> {
  const workflow: DocumentWorkflow<DocumentDraft> = {
    data: { content },
    stateName: 'draft',
    submitForReview() {
      return { ...workflow, stateName: 'review' } as unknown as DocumentWorkflow<DocumentReview>;
    },
    approve: undefined as never,
    reject: undefined as never,
    publish: undefined as never,
    archive: undefined as never,
    edit: undefined as never,
  };
  return workflow;
}

// ═══════════════════════════════════════════════════════════════════════════
// CAPABILITY-BASED SECURITY PATTERN
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Capability markers for access control.
 */
export interface CanRead { readonly _canRead: true; }
export interface CanWrite { readonly _canWrite: true; }
export interface CanDelete { readonly _canDelete: true; }
export interface CanAdmin { readonly _canAdmin: true; }

/**
 * Full access includes all capabilities.
 */
export type FullAccess = CanRead & CanWrite & CanDelete & CanAdmin;

/**
 * Read-only access.
 */
export type ReadOnly = CanRead;

/**
 * Read-write access.
 */
export type ReadWrite = CanRead & CanWrite;

/**
 * Capability-protected resource.
 * Methods are only available if the capability is present.
 */
export interface ProtectedResource<C, T> {
  /** The resource data */
  readonly data: T;

  /** Read operation - requires CanRead */
  read: C extends CanRead ? () => T : never;

  /** Write operation - requires CanWrite */
  write: C extends CanWrite ? (data: Partial<T>) => ProtectedResource<C, T> : never;

  /** Delete operation - requires CanDelete */
  delete: C extends CanDelete ? () => void : never;

  /** Admin operation - requires CanAdmin */
  grantAccess: C extends CanAdmin
    ? <NewC>(capabilities: NewC) => ProtectedResource<NewC, T>
    : never;
}

/**
 * Creates a protected resource with specified capabilities.
 */
export function protect<C, T>(data: T): ProtectedResource<C, T> {
  return {
    data,
    read: (() => data) as ProtectedResource<C, T>['read'],
    write: ((partial: Partial<T>) =>
      protect<C, T>({ ...data, ...partial })) as ProtectedResource<C, T>['write'],
    delete: (() => {}) as ProtectedResource<C, T>['delete'],
    grantAccess: (<NewC>() =>
      protect<NewC, T>(data)) as unknown as ProtectedResource<C, T>['grantAccess'],
  };
}
