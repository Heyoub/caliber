/**
 * CALIBER Client
 *
 * Main entry point for the CALIBER SDK.
 * Provides a unified interface to all CALIBER resources.
 */

import { HttpClient, type HttpClientConfig } from './http';
import {
  TrajectoryManager,
  ScopeManager,
  ArtifactManager,
  NoteManager,
  TurnManager,
  AgentManager,
  LockManager,
  MessageManager,
  DelegationManager,
  HandoffManager,
  SearchManager,
  DslManager,
  BatchManager,
} from './managers';
import { ContextHelper } from './context';
import type { AssembleContextOptions, ContextPackage, FormatContextOptions } from './context';
import type { Link, Linkable, HttpMethod } from './types/common';
import { CaliberError } from './errors';

/**
 * Configuration for the CALIBER client
 */
export interface CalibrClientConfig {
  /** Base URL for the CALIBER API (default: https://api.caliber.run) */
  baseUrl?: string;
  /** API key for authentication */
  apiKey: string;
  /** Tenant ID for multi-tenant isolation */
  tenantId: string;
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;
  /** Custom headers to include in all requests */
  headers?: Record<string, string>;
}

/**
 * CALIBER Client
 *
 * The main entry point for interacting with the CALIBER API.
 * Provides access to all resource managers through a single unified client.
 *
 * @example
 * ```typescript
 * import { CalibrClient } from '@caliber-run/sdk';
 *
 * const client = new CalibrClient({
 *   apiKey: process.env.CALIBER_API_KEY!,
 *   tenantId: 'your-tenant-id',
 * });
 *
 * // Create a trajectory (task)
 * const trajectory = await client.trajectories.create({
 *   name: 'Build feature X',
 *   description: 'Implement new feature',
 * });
 *
 * // Create a scope (context window)
 * const scope = await client.scopes.create({
 *   trajectory_id: trajectory.trajectory_id,
 *   name: 'Implementation',
 *   token_budget: 8000,
 * });
 *
 * // Store an artifact (important output)
 * await client.artifacts.create({
 *   trajectory_id: trajectory.trajectory_id,
 *   scope_id: scope.scope_id,
 *   artifact_type: 'Code',
 *   name: 'feature.ts',
 *   content: 'export function feature() {...}',
 *   source_turn: 1,
 *   extraction_method: 'Explicit',
 *   ttl: 'Persistent',
 * });
 * ```
 */
export class CalibrClient {
  private readonly http: HttpClient;

  /** Trajectory (task container) operations */
  public readonly trajectories: TrajectoryManager;

  /** Scope (context window) operations */
  public readonly scopes: ScopeManager;

  /** Artifact (extracted value) operations */
  public readonly artifacts: ArtifactManager;

  /** Note (cross-trajectory knowledge) operations */
  public readonly notes: NoteManager;

  /** Turn (ephemeral message) operations */
  public readonly turns: TurnManager;

  /** Agent registration and lifecycle operations */
  public readonly agents: AgentManager;

  /** Distributed lock operations */
  public readonly locks: LockManager;

  /** Inter-agent messaging operations */
  public readonly messages: MessageManager;

  /** Task delegation operations */
  public readonly delegations: DelegationManager;

  /** Context handoff operations */
  public readonly handoffs: HandoffManager;

  /** Global search operations */
  public readonly search: SearchManager;

  /** DSL validation and parsing operations */
  public readonly dsl: DslManager;

  /** Batch operations for bulk CRUD */
  public readonly batch: BatchManager;

  /** Context assembly helper for LLM prompts */
  private readonly contextHelper: ContextHelper;

  constructor(config: CalibrClientConfig) {
    const httpConfig: HttpClientConfig = {
      baseUrl: config.baseUrl ?? 'https://api.caliber.run',
      apiKey: config.apiKey,
      tenantId: config.tenantId,
      timeout: config.timeout,
      headers: config.headers,
    };

    this.http = new HttpClient(httpConfig);

    // Initialize all managers
    this.trajectories = new TrajectoryManager(this.http);
    this.scopes = new ScopeManager(this.http);
    this.artifacts = new ArtifactManager(this.http);
    this.notes = new NoteManager(this.http);
    this.turns = new TurnManager(this.http);
    this.agents = new AgentManager(this.http);
    this.locks = new LockManager(this.http);
    this.messages = new MessageManager(this.http);
    this.delegations = new DelegationManager(this.http);
    this.handoffs = new HandoffManager(this.http);
    this.search = new SearchManager(this.http);
    this.dsl = new DslManager(this.http);
    this.batch = new BatchManager(this.http);
    this.contextHelper = new ContextHelper(this.http);
  }

  /**
   * Get the tenant ID for this client
   */
  getTenantId(): string {
    return this.http.getTenantId();
  }

  // ===========================================================================
  // HATEOAS Link Navigation
  // ===========================================================================

  /**
   * Follow a HATEOAS link to retrieve a related resource.
   *
   * This method enables hypermedia-driven navigation of API responses.
   * When a response includes `_links`, you can follow them without
   * constructing URLs manually.
   *
   * @typeParam T - The expected response type
   * @param link - The link object to follow
   * @returns Promise resolving to the linked resource
   *
   * @example
   * ```typescript
   * const trajectory = await client.trajectories.get('123');
   *
   * // Follow a link to get related scopes
   * if (trajectory._links?.scopes) {
   *   const scopes = await client.follow<Scope[]>(trajectory._links.scopes);
   * }
   *
   * // Follow a POST link to create a resource
   * const createLink = trajectory._links?.['create-scope'];
   * if (createLink) {
   *   // Note: follow() doesn't support request bodies - use managers for POST/PATCH
   *   const newScope = await client.follow<Scope>(createLink);
   * }
   * ```
   */
  async follow<T>(link: Link): Promise<T> {
    const method = (link.method || 'GET').toUpperCase() as HttpMethod;

    switch (method) {
      case 'GET':
        return this.http.get<T>(link.href);
      case 'POST':
        return this.http.post<T>(link.href);
      case 'PUT':
        return this.http.put<T>(link.href);
      case 'PATCH':
        return this.http.patch<T>(link.href);
      case 'DELETE':
        return this.http.delete<T>(link.href);
      default:
        throw new CaliberError(`Unsupported HTTP method: ${method}`, 'INVALID_LINK_METHOD');
    }
  }

  /**
   * Discover and follow a named link from a response.
   *
   * This is a convenience method that combines link lookup and following.
   * It extracts a link by relation name from a response and follows it.
   *
   * @typeParam T - The expected response type
   * @typeParam R - The response type containing links (must extend Linkable)
   * @param response - A response object that may contain `_links`
   * @param rel - The relation name of the link to follow (e.g., 'self', 'parent', 'scopes')
   * @returns Promise resolving to the linked resource
   * @throws CaliberError if the link is not found in the response
   *
   * @example
   * ```typescript
   * const trajectory = await client.trajectories.get('123');
   *
   * // Discover and follow the 'parent' link
   * try {
   *   const parent = await client.discover<Trajectory, typeof trajectory>(
   *     trajectory,
   *     'parent'
   *   );
   *   console.log('Parent trajectory:', parent.name);
   * } catch (e) {
   *   console.log('No parent trajectory');
   * }
   *
   * // Discover and follow the 'scopes' link
   * const scopes = await client.discover<Scope[], typeof trajectory>(
   *   trajectory,
   *   'scopes'
   * );
   * ```
   */
  async discover<T, R extends Linkable>(response: R, rel: string): Promise<T> {
    const link = response._links?.[rel];
    if (!link) {
      throw new CaliberError(`Link '${rel}' not found in response`, 'LINK_NOT_FOUND');
    }
    return this.follow<T>(link);
  }

  /**
   * Assemble context from a trajectory for use in LLM prompts.
   *
   * This method collects hierarchical memory from a trajectory including:
   * - Trajectory metadata and hierarchy
   * - Artifacts (structured outputs from previous interactions)
   * - Notes (cross-trajectory knowledge)
   * - Recent turns (conversation history)
   *
   * @param trajectoryId - The trajectory to gather context from
   * @param options - Options for context assembly
   * @returns Assembled context package
   *
   * @example
   * ```typescript
   * const context = await client.assembleContext('trajectory-id', {
   *   includeNotes: true,
   *   maxArtifacts: 5,
   *   relevanceQuery: 'user authentication',
   * });
   *
   * // Use context for prompt building
   * const formatted = client.formatContext(context, { format: 'xml' });
   * ```
   */
  async assembleContext(
    trajectoryId: string,
    options?: AssembleContextOptions
  ): Promise<ContextPackage> {
    return this.contextHelper.assembleContext(trajectoryId, options);
  }

  /**
   * Format assembled context as a string for LLM prompts.
   *
   * @param context - The assembled context package
   * @param options - Formatting options
   * @returns Formatted context string
   */
  formatContext(context: ContextPackage, options?: FormatContextOptions): string {
    return this.contextHelper.formatContext(context, options);
  }

  /**
   * Quick method to get formatted context in one call.
   * Combines assembleContext and formatContext.
   *
   * @param trajectoryId - The trajectory to gather context from
   * @param assembleOptions - Options for context assembly
   * @param formatOptions - Options for formatting
   * @returns Formatted context string ready for LLM prompts
   *
   * @example
   * ```typescript
   * const contextXml = await client.getFormattedContext('trajectory-id', {
   *   includeNotes: true,
   *   relevanceQuery: 'authentication',
   * });
   *
   * const prompt = `
   * You have access to the following context:
   * ${contextXml}
   *
   * Please help with: How should I implement the login flow?
   * `;
   * ```
   */
  async getFormattedContext(
    trajectoryId: string,
    assembleOptions?: AssembleContextOptions,
    formatOptions?: FormatContextOptions
  ): Promise<string> {
    return this.contextHelper.getFormattedContext(trajectoryId, assembleOptions, formatOptions);
  }
}
