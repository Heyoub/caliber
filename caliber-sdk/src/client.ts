/**
 * CALIBER Client
 *
 * Main entry point for the CALIBER SDK.
 * Provides a unified interface to all CALIBER resources.
 */

import { HttpClient, HttpClientConfig } from './http';
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
} from './managers';

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
  }

  /**
   * Get the tenant ID for this client
   */
  getTenantId(): string {
    return this.http.getTenantId();
  }
}
