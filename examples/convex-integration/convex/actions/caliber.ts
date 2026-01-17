/**
 * CALIBER Actions for Convex
 *
 * This module provides Convex actions that wrap the CALIBER SDK,
 * enabling AI agents running on Convex to use CALIBER's hierarchical
 * memory system.
 *
 * ## Architecture
 *
 * Convex Actions run in Node.js and can make HTTP requests to external
 * services. These actions call the CALIBER REST API via the SDK.
 *
 * ## Usage
 *
 * ```typescript
 * import { api } from "./_generated/api";
 *
 * // In a Convex function:
 * const task = await ctx.runAction(api.actions.caliber.startTask, {
 *   name: "Help user with code review",
 *   description: "Reviewing PR #123",
 * });
 *
 * await ctx.runAction(api.actions.caliber.addTurn, {
 *   scopeId: task.scopeId,
 *   role: "User",
 *   content: "Can you review this PR?",
 * });
 *
 * const context = await ctx.runAction(api.actions.caliber.getContext, {
 *   trajectoryId: task.trajectoryId,
 *   relevanceQuery: "code review best practices",
 * });
 * ```
 */

"use node";

import { action } from "../_generated/server";
import { v } from "convex/values";
import { CalibrClient } from "@caliber-run/sdk";

// ============================================================================
// CALIBER CLIENT INITIALIZATION
// ============================================================================

/**
 * Get a configured CALIBER client.
 * Uses environment variables for configuration.
 */
function getCalibrClient(): CalibrClient {
  const apiKey = process.env.CALIBER_API_KEY;
  const tenantId = process.env.CALIBER_TENANT_ID;
  const baseUrl = process.env.CALIBER_API_URL ?? "https://api.caliber.run";

  if (!apiKey) {
    throw new Error("CALIBER_API_KEY environment variable is required");
  }
  if (!tenantId) {
    throw new Error("CALIBER_TENANT_ID environment variable is required");
  }

  return new CalibrClient({
    baseUrl,
    apiKey,
    tenantId,
  });
}

// ============================================================================
// TASK LIFECYCLE ACTIONS
// ============================================================================

/**
 * Start a new task (trajectory + scope).
 *
 * This creates a trajectory (task container) and an initial scope (context window).
 * Use this when beginning a new agent task or conversation.
 */
export const startTask = action({
  args: {
    name: v.string(),
    description: v.optional(v.string()),
    tokenBudget: v.optional(v.number()),
    parentTrajectoryId: v.optional(v.string()),
    agentId: v.optional(v.string()),
    metadata: v.optional(v.any()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    // Create trajectory
    const trajectory = await caliber.trajectories.create({
      name: args.name,
      description: args.description,
      parent_trajectory_id: args.parentTrajectoryId,
      agent_id: args.agentId,
      metadata: args.metadata,
    });

    // Create initial scope
    const scope = await caliber.scopes.create({
      trajectory_id: trajectory.trajectory_id,
      name: "main",
      token_budget: args.tokenBudget ?? 8000,
    });

    return {
      trajectoryId: trajectory.trajectory_id,
      scopeId: scope.scope_id,
      trajectory,
      scope,
    };
  },
});

/**
 * Complete a task with an outcome.
 *
 * Use this when a task is finished, either successfully or with failure.
 */
export const completeTask = action({
  args: {
    trajectoryId: v.string(),
    status: v.union(v.literal("Success"), v.literal("Failure"), v.literal("Partial")),
    summary: v.optional(v.string()),
    metadata: v.optional(v.any()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    // Update trajectory with completion status
    const trajectory = await caliber.trajectories.update(args.trajectoryId, {
      status: "Completed",
      metadata: {
        outcome: {
          status: args.status,
          summary: args.summary,
        },
        ...args.metadata,
      },
    });

    return trajectory;
  },
});

// ============================================================================
// CONVERSATION ACTIONS (TURNS)
// ============================================================================

/**
 * Add a turn to the current scope.
 *
 * Turns are ephemeral conversation messages within a scope.
 */
export const addTurn = action({
  args: {
    scopeId: v.string(),
    role: v.union(v.literal("User"), v.literal("Agent"), v.literal("System"), v.literal("Tool")),
    content: v.string(),
    tokenCount: v.optional(v.number()),
    metadata: v.optional(v.any()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const turn = await caliber.turns.create({
      scope_id: args.scopeId,
      role: args.role,
      content: args.content,
      token_count: args.tokenCount,
      metadata: args.metadata,
    });

    return turn;
  },
});

// ============================================================================
// ARTIFACT ACTIONS (EXTRACTED VALUES)
// ============================================================================

/**
 * Extract an artifact from the current context.
 *
 * Artifacts are valuable outputs that should persist beyond the current
 * scope, such as code, decisions, plans, or other structured data.
 */
export const extractArtifact = action({
  args: {
    trajectoryId: v.string(),
    scopeId: v.string(),
    name: v.string(),
    content: v.string(),
    artifactType: v.union(
      v.literal("Code"),
      v.literal("Document"),
      v.literal("Decision"),
      v.literal("Plan"),
      v.literal("Data"),
      v.literal("Reference"),
      v.literal("Summary"),
      v.literal("Principle"),
      v.literal("Other")
    ),
    sourceTurn: v.number(),
    extractionMethod: v.optional(
      v.union(v.literal("Explicit"), v.literal("Inferred"), v.literal("Automatic"))
    ),
    ttl: v.optional(
      v.union(v.literal("Ephemeral"), v.literal("Session"), v.literal("Persistent"))
    ),
    confidence: v.optional(v.number()),
    metadata: v.optional(v.any()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const artifact = await caliber.artifacts.create({
      trajectory_id: args.trajectoryId,
      scope_id: args.scopeId,
      name: args.name,
      content: args.content,
      artifact_type: args.artifactType,
      source_turn: args.sourceTurn,
      extraction_method: args.extractionMethod ?? "Explicit",
      ttl: args.ttl ?? "Persistent",
      confidence: args.confidence,
      metadata: args.metadata,
    });

    return artifact;
  },
});

/**
 * List artifacts for a trajectory.
 */
export const listArtifacts = action({
  args: {
    trajectoryId: v.string(),
    limit: v.optional(v.number()),
    artifactType: v.optional(v.string()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const response = await caliber.artifacts.list({
      trajectory_id: args.trajectoryId,
      limit: args.limit ?? 20,
      artifact_type: args.artifactType as any,
    });

    return response;
  },
});

// ============================================================================
// NOTE ACTIONS (CROSS-TRAJECTORY KNOWLEDGE)
// ============================================================================

/**
 * Create a note (cross-trajectory knowledge).
 *
 * Notes persist beyond any single trajectory and represent long-term
 * knowledge that should be available across all tasks.
 */
export const createNote = action({
  args: {
    title: v.string(),
    content: v.string(),
    noteType: v.union(
      v.literal("Lesson"),
      v.literal("Fact"),
      v.literal("Preference"),
      v.literal("Context"),
      v.literal("Reference"),
      v.literal("Other")
    ),
    sourceTrajectoryId: v.optional(v.string()),
    sourceArtifactId: v.optional(v.string()),
    ttl: v.optional(
      v.union(v.literal("Ephemeral"), v.literal("Session"), v.literal("Persistent"))
    ),
    metadata: v.optional(v.any()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const note = await caliber.notes.create({
      title: args.title,
      content: args.content,
      note_type: args.noteType,
      source_trajectory_id: args.sourceTrajectoryId,
      source_artifact_id: args.sourceArtifactId,
      ttl: args.ttl ?? "Persistent",
      metadata: args.metadata,
    });

    return note;
  },
});

/**
 * List notes (cross-trajectory knowledge).
 */
export const listNotes = action({
  args: {
    limit: v.optional(v.number()),
    noteType: v.optional(v.string()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const response = await caliber.notes.list({
      limit: args.limit ?? 20,
      note_type: args.noteType as any,
    });

    return response;
  },
});

// ============================================================================
// CONTEXT RETRIEVAL ACTIONS
// ============================================================================

/**
 * Get context for a trajectory.
 *
 * This retrieves all relevant memory for building LLM prompts:
 * - Artifacts from the trajectory
 * - Cross-trajectory notes
 * - Recent conversation turns
 */
export const getContext = action({
  args: {
    trajectoryId: v.string(),
    relevanceQuery: v.optional(v.string()),
    includeNotes: v.optional(v.boolean()),
    maxArtifacts: v.optional(v.number()),
    maxNotes: v.optional(v.number()),
    maxTurns: v.optional(v.number()),
    format: v.optional(v.union(v.literal("xml"), v.literal("markdown"), v.literal("json"))),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    // Use the context assembly helper
    const formattedContext = await caliber.getFormattedContext(
      args.trajectoryId,
      {
        includeNotes: args.includeNotes ?? true,
        maxArtifacts: args.maxArtifacts ?? 10,
        maxNotes: args.maxNotes ?? 5,
        maxTurns: args.maxTurns ?? 20,
        relevanceQuery: args.relevanceQuery,
      },
      {
        format: args.format ?? "xml",
      }
    );

    return formattedContext;
  },
});

/**
 * Get raw context as structured data (not formatted string).
 */
export const getContextRaw = action({
  args: {
    trajectoryId: v.string(),
    relevanceQuery: v.optional(v.string()),
    includeNotes: v.optional(v.boolean()),
    maxArtifacts: v.optional(v.number()),
    maxNotes: v.optional(v.number()),
    maxTurns: v.optional(v.number()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const context = await caliber.assembleContext(args.trajectoryId, {
      includeNotes: args.includeNotes ?? true,
      maxArtifacts: args.maxArtifacts ?? 10,
      maxNotes: args.maxNotes ?? 5,
      maxTurns: args.maxTurns ?? 20,
      relevanceQuery: args.relevanceQuery,
    });

    return context;
  },
});

// ============================================================================
// SCOPE MANAGEMENT ACTIONS
// ============================================================================

/**
 * Create a new scope (context window) within a trajectory.
 *
 * Use this when you need to start a new context window, such as when
 * the current scope is running out of token budget.
 */
export const createScope = action({
  args: {
    trajectoryId: v.string(),
    name: v.string(),
    tokenBudget: v.optional(v.number()),
    parentScopeId: v.optional(v.string()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const scope = await caliber.scopes.create({
      trajectory_id: args.trajectoryId,
      name: args.name,
      token_budget: args.tokenBudget ?? 8000,
      parent_scope_id: args.parentScopeId,
    });

    return scope;
  },
});

/**
 * Close a scope.
 *
 * Closing a scope triggers summarization (if configured) and marks
 * the scope as no longer active.
 */
export const closeScope = action({
  args: {
    scopeId: v.string(),
    summary: v.optional(v.string()),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const scope = await caliber.scopes.update(args.scopeId, {
      status: "Closed",
      summary: args.summary,
    });

    return scope;
  },
});

// ============================================================================
// BATCH OPERATIONS
// ============================================================================

/**
 * Batch create artifacts.
 *
 * Use this for bulk imports or when extracting multiple artifacts at once.
 */
export const batchCreateArtifacts = action({
  args: {
    artifacts: v.array(
      v.object({
        trajectoryId: v.string(),
        scopeId: v.string(),
        name: v.string(),
        content: v.string(),
        artifactType: v.string(),
        sourceTurn: v.number(),
      })
    ),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const result = await caliber.batch.createArtifacts(
      args.artifacts.map((a) => ({
        trajectory_id: a.trajectoryId,
        scope_id: a.scopeId,
        name: a.name,
        content: a.content,
        artifact_type: a.artifactType as any,
        source_turn: a.sourceTurn,
        extraction_method: "Explicit" as const,
        ttl: "Persistent" as const,
      }))
    );

    return {
      succeeded: result.succeeded,
      failed: result.failed,
      results: result.results,
    };
  },
});

/**
 * Batch create notes.
 */
export const batchCreateNotes = action({
  args: {
    notes: v.array(
      v.object({
        title: v.string(),
        content: v.string(),
        noteType: v.string(),
      })
    ),
  },
  handler: async (_ctx, args) => {
    const caliber = getCalibrClient();

    const result = await caliber.batch.createNotes(
      args.notes.map((n) => ({
        title: n.title,
        content: n.content,
        note_type: n.noteType as any,
        ttl: "Persistent" as const,
      }))
    );

    return {
      succeeded: result.succeeded,
      failed: result.failed,
      results: result.results,
    };
  },
});
