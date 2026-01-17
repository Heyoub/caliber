/**
 * CALIBER Context Assembly Helper
 *
 * Provides utilities for assembling context from CALIBER memory
 * for use in LLM prompts. Handles hierarchical memory collection,
 * relevance filtering, and token budget management.
 */

import type { HttpClient } from './http';
import type {
  Trajectory,
  Scope,
  Artifact,
  Note,
  Turn,
} from './types';

/**
 * Options for context assembly
 */
export interface AssembleContextOptions {
  /**
   * Whether to include cross-trajectory notes.
   * These are long-term knowledge that persists beyond any single trajectory.
   * @default true
   */
  includeNotes?: boolean;

  /**
   * Maximum number of artifacts to include.
   * Artifacts are ordered by relevance if a query is provided, otherwise by recency.
   * @default 10
   */
  maxArtifacts?: number;

  /**
   * Maximum number of notes to include.
   * @default 5
   */
  maxNotes?: number;

  /**
   * Maximum number of turns to include from the current scope.
   * Turns are ordered by recency (most recent first).
   * @default 20
   */
  maxTurns?: number;

  /**
   * Optional relevance query for semantic search.
   * When provided, artifacts and notes will be filtered by relevance to this query.
   */
  relevanceQuery?: string;

  /**
   * Minimum relevance score (0-1) for semantic search results.
   * Only applies when relevanceQuery is provided.
   * @default 0.5
   */
  minRelevance?: number;

  /**
   * Whether to include the full trajectory hierarchy (parent trajectories).
   * @default false
   */
  includeHierarchy?: boolean;

  /**
   * Whether to include artifact content or just metadata.
   * Set to false for token-efficient context.
   * @default true
   */
  includeArtifactContent?: boolean;

  /**
   * Optional scope ID to gather context from a specific scope.
   * If not provided, uses the most recent active scope.
   */
  scopeId?: string;
}

/**
 * Assembled context package ready for LLM prompts
 */
export interface ContextPackage {
  /**
   * The trajectory this context belongs to
   */
  trajectory: Trajectory;

  /**
   * The scope this context was gathered from (if any)
   */
  scope?: Scope;

  /**
   * Parent trajectories (if includeHierarchy was true)
   */
  parentTrajectories?: Trajectory[];

  /**
   * Artifacts extracted from this trajectory
   */
  artifacts: Artifact[];

  /**
   * Cross-trajectory notes (long-term memory)
   */
  notes: Note[];

  /**
   * Recent turns (ephemeral conversation history)
   */
  turns: Turn[];

  /**
   * Metadata about the context assembly
   */
  meta: {
    /** Timestamp when context was assembled */
    assembledAt: string;
    /** Total number of items collected */
    totalItems: number;
    /** Whether any results were truncated due to limits */
    truncated: boolean;
    /** Query used for relevance filtering (if any) */
    relevanceQuery?: string;
  };
}

/**
 * Format options for rendering context as text
 */
export interface FormatContextOptions {
  /**
   * Format style for the output
   * - 'markdown': Formatted markdown with headers
   * - 'xml': XML tags (recommended for Claude)
   * - 'json': Raw JSON
   * @default 'xml'
   */
  format?: 'markdown' | 'xml' | 'json';

  /**
   * Whether to include artifact content in the output.
   * @default true
   */
  includeContent?: boolean;

  /**
   * Maximum characters for artifact content preview.
   * Set to 0 for no limit.
   * @default 1000
   */
  maxContentLength?: number;
}

/**
 * Context Assembly Helper
 *
 * Provides methods to collect and format context from CALIBER memory
 * for use in LLM prompts.
 *
 * @example
 * ```typescript
 * import { CalibrClient, ContextHelper } from '@caliber-run/sdk';
 *
 * const client = new CalibrClient({ apiKey: '...', tenantId: '...' });
 * const helper = new ContextHelper(client);
 *
 * // Assemble context for a trajectory
 * const context = await helper.assembleContext('trajectory-id', {
 *   includeNotes: true,
 *   maxArtifacts: 5,
 *   relevanceQuery: 'user authentication',
 * });
 *
 * // Format for LLM prompt
 * const formatted = helper.formatContext(context, { format: 'xml' });
 *
 * // Use in prompt
 * const prompt = `
 * You are an assistant with access to the following context:
 *
 * ${formatted}
 *
 * User: How should I implement the login flow?
 * `;
 * ```
 */
export class ContextHelper {
  private readonly http: HttpClient;

  constructor(http: HttpClient) {
    this.http = http;
  }

  /**
   * Assemble context from a trajectory for use in LLM prompts.
   *
   * This method collects:
   * - The trajectory metadata
   * - Artifacts (structured outputs from previous interactions)
   * - Notes (cross-trajectory knowledge)
   * - Recent turns (conversation history)
   *
   * @param trajectoryId - The trajectory to gather context from
   * @param options - Options for context assembly
   * @returns Assembled context package
   */
  async assembleContext(
    trajectoryId: string,
    options: AssembleContextOptions = {}
  ): Promise<ContextPackage> {
    const {
      includeNotes = true,
      maxArtifacts = 10,
      maxNotes = 5,
      maxTurns = 20,
      relevanceQuery,
      minRelevance = 0.5,
      includeHierarchy = false,
      scopeId,
    } = options;

    // Fetch trajectory
    const trajectory = await this.http.get<Trajectory>(
      `/api/v1/trajectories/${trajectoryId}`
    );

    // Fetch parent trajectories if requested
    let parentTrajectories: Trajectory[] | undefined;
    if (includeHierarchy && trajectory.parent_trajectory_id) {
      parentTrajectories = await this.fetchHierarchy(trajectory.parent_trajectory_id);
    }

    // Determine which scope to use
    let scope: Scope | undefined;
    if (scopeId) {
      scope = await this.http.get<Scope>(`/api/v1/scopes/${scopeId}`);
    } else {
      // Get most recent active scope for this trajectory
      const scopesResponse = await this.http.get<{ scopes: Scope[] }>(
        `/api/v1/trajectories/${trajectoryId}/scopes`,
        { params: { limit: 1, status: 'Active' } }
      );
      scope = scopesResponse.scopes[0];
    }

    // Fetch artifacts
    let artifacts: Artifact[] = [];
    if (relevanceQuery) {
      // Use semantic search
      const searchResponse = await this.http.post<{ results: Array<{ artifact: Artifact; score: number }> }>(
        '/api/v1/artifacts/search',
        {
          query: relevanceQuery,
          trajectory_id: trajectoryId,
          limit: maxArtifacts,
          min_score: minRelevance,
        }
      );
      artifacts = searchResponse.results
        .filter((r) => r.score >= minRelevance)
        .map((r) => r.artifact);
    } else {
      // Fetch by recency
      const artifactsResponse = await this.http.get<{ artifacts: Artifact[] }>(
        '/api/v1/artifacts',
        { params: { trajectory_id: trajectoryId, limit: maxArtifacts } }
      );
      artifacts = artifactsResponse.artifacts;
    }

    // Fetch notes
    let notes: Note[] = [];
    if (includeNotes) {
      if (relevanceQuery) {
        // Use semantic search
        const searchResponse = await this.http.post<{ results: Array<{ note: Note; score: number }> }>(
          '/api/v1/notes/search',
          {
            query: relevanceQuery,
            limit: maxNotes,
            min_score: minRelevance,
          }
        );
        notes = searchResponse.results
          .filter((r) => r.score >= minRelevance)
          .map((r) => r.note);
      } else {
        // Fetch by recency
        const notesResponse = await this.http.get<{ notes: Note[] }>(
          '/api/v1/notes',
          { params: { limit: maxNotes } }
        );
        notes = notesResponse.notes;
      }
    }

    // Fetch turns from the current scope
    let turns: Turn[] = [];
    if (scope) {
      const turnsResponse = await this.http.get<{ turns: Turn[] }>(
        '/api/v1/turns',
        { params: { scope_id: scope.scope_id, limit: maxTurns } }
      );
      turns = turnsResponse.turns;
    }

    const totalItems = artifacts.length + notes.length + turns.length;
    const truncated =
      artifacts.length >= maxArtifacts ||
      notes.length >= maxNotes ||
      turns.length >= maxTurns;

    return {
      trajectory,
      scope,
      parentTrajectories,
      artifacts,
      notes,
      turns,
      meta: {
        assembledAt: new Date().toISOString(),
        totalItems,
        truncated,
        relevanceQuery,
      },
    };
  }

  /**
   * Format assembled context as a string for LLM prompts.
   *
   * @param context - The assembled context package
   * @param options - Formatting options
   * @returns Formatted context string
   */
  formatContext(context: ContextPackage, options: FormatContextOptions = {}): string {
    const { format = 'xml', includeContent = true, maxContentLength = 1000 } = options;

    switch (format) {
      case 'markdown':
        return this.formatMarkdown(context, includeContent, maxContentLength);
      case 'xml':
        return this.formatXml(context, includeContent, maxContentLength);
      case 'json':
        return JSON.stringify(context, null, 2);
      default:
        throw new Error(`Unknown format: ${format}`);
    }
  }

  /**
   * Quick method to get formatted context in one call.
   * Combines assembleContext and formatContext.
   */
  async getFormattedContext(
    trajectoryId: string,
    assembleOptions?: AssembleContextOptions,
    formatOptions?: FormatContextOptions
  ): Promise<string> {
    const context = await this.assembleContext(trajectoryId, assembleOptions);
    return this.formatContext(context, formatOptions);
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async fetchHierarchy(trajectoryId: string): Promise<Trajectory[]> {
    const hierarchy: Trajectory[] = [];
    let currentId: string | undefined = trajectoryId;

    while (currentId) {
      const trajectory = await this.http.get<Trajectory>(
        `/api/v1/trajectories/${currentId}`
      );
      hierarchy.push(trajectory);
      currentId = trajectory.parent_trajectory_id ?? undefined;

      // Safety limit to prevent infinite loops
      if (hierarchy.length > 10) break;
    }

    return hierarchy;
  }

  private formatMarkdown(
    context: ContextPackage,
    includeContent: boolean,
    maxContentLength: number
  ): string {
    const lines: string[] = [];

    lines.push(`# Context for: ${context.trajectory.name}`);
    lines.push('');

    if (context.trajectory.description) {
      lines.push(`> ${context.trajectory.description}`);
      lines.push('');
    }

    if (context.parentTrajectories && context.parentTrajectories.length > 0) {
      lines.push('## Parent Tasks');
      for (const parent of context.parentTrajectories) {
        lines.push(`- **${parent.name}**: ${parent.description ?? 'No description'}`);
      }
      lines.push('');
    }

    if (context.artifacts.length > 0) {
      lines.push('## Artifacts (Extracted Knowledge)');
      for (const artifact of context.artifacts) {
        lines.push(`### ${artifact.name} (${artifact.artifact_type})`);
        if (includeContent) {
          const content = this.truncateContent(artifact.content, maxContentLength);
          lines.push('```');
          lines.push(content);
          lines.push('```');
        }
        lines.push('');
      }
    }

    if (context.notes.length > 0) {
      lines.push('## Notes (Long-term Memory)');
      for (const note of context.notes) {
        lines.push(`### ${note.title} (${note.note_type})`);
        if (includeContent) {
          const content = this.truncateContent(note.content, maxContentLength);
          lines.push(content);
        }
        lines.push('');
      }
    }

    if (context.turns.length > 0) {
      lines.push('## Recent Conversation');
      for (const turn of context.turns.slice().reverse()) {
        lines.push(`**${turn.role}**: ${turn.content}`);
      }
      lines.push('');
    }

    return lines.join('\n');
  }

  private formatXml(
    context: ContextPackage,
    includeContent: boolean,
    maxContentLength: number
  ): string {
    const lines: string[] = [];

    lines.push('<context>');
    lines.push(`  <trajectory name="${this.escapeXml(context.trajectory.name)}">`);
    if (context.trajectory.description) {
      lines.push(`    <description>${this.escapeXml(context.trajectory.description)}</description>`);
    }
    lines.push(`    <status>${context.trajectory.status}</status>`);
    lines.push('  </trajectory>');

    if (context.parentTrajectories && context.parentTrajectories.length > 0) {
      lines.push('  <parent_tasks>');
      for (const parent of context.parentTrajectories) {
        lines.push(`    <task name="${this.escapeXml(parent.name)}">`);
        if (parent.description) {
          lines.push(`      <description>${this.escapeXml(parent.description)}</description>`);
        }
        lines.push('    </task>');
      }
      lines.push('  </parent_tasks>');
    }

    if (context.artifacts.length > 0) {
      lines.push('  <artifacts>');
      for (const artifact of context.artifacts) {
        lines.push(`    <artifact name="${this.escapeXml(artifact.name)}" type="${artifact.artifact_type}">`);
        if (includeContent) {
          const content = this.truncateContent(artifact.content, maxContentLength);
          lines.push(`      <content>${this.escapeXml(content)}</content>`);
        }
        lines.push('    </artifact>');
      }
      lines.push('  </artifacts>');
    }

    if (context.notes.length > 0) {
      lines.push('  <notes>');
      for (const note of context.notes) {
        lines.push(`    <note title="${this.escapeXml(note.title)}" type="${note.note_type}">`);
        if (includeContent) {
          const content = this.truncateContent(note.content, maxContentLength);
          lines.push(`      <content>${this.escapeXml(content)}</content>`);
        }
        lines.push('    </note>');
      }
      lines.push('  </notes>');
    }

    if (context.turns.length > 0) {
      lines.push('  <conversation>');
      for (const turn of context.turns.slice().reverse()) {
        lines.push(`    <turn role="${turn.role}">${this.escapeXml(turn.content)}</turn>`);
      }
      lines.push('  </conversation>');
    }

    lines.push('</context>');

    return lines.join('\n');
  }

  private truncateContent(content: string, maxLength: number): string {
    if (maxLength === 0 || content.length <= maxLength) {
      return content;
    }
    return content.slice(0, maxLength) + '... [truncated]';
  }

  private escapeXml(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&apos;');
  }
}
