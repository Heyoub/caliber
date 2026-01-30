/**
 * Convex Schema for CALIBER Integration
 *
 * This schema defines optional local cache tables for CALIBER entities.
 * These tables can be used to cache CALIBER data in Convex for faster
 * reads and real-time subscriptions.
 *
 * NOTE: These tables are OPTIONAL. You can use CALIBER actions without
 * any local storage. Use these tables when you need:
 * - Real-time Convex subscriptions on CALIBER data
 * - Offline-first access to recent memories
 * - Reduced latency for frequently-accessed context
 */

import { defineSchema, defineTable } from 'convex/server';
import { v } from 'convex/values';

export default defineSchema({
  /**
   * Local cache of active trajectories.
   *
   * Sync via webhooks or polling from CALIBER.
   */
  trajectoryCache: defineTable({
    // CALIBER ID (external)
    caliberTrajectoryId: v.string(),
    // Snapshot of trajectory data
    name: v.string(),
    description: v.optional(v.string()),
    status: v.string(),
    parentTrajectoryId: v.optional(v.string()),
    // Sync metadata
    syncedAt: v.number(),
  })
    .index('by_caliber_id', ['caliberTrajectoryId'])
    .index('by_status', ['status']),

  /**
   * Local cache of artifacts.
   *
   * Useful for showing recent artifacts in UI without API calls.
   */
  artifactCache: defineTable({
    // CALIBER ID (external)
    caliberArtifactId: v.string(),
    caliberTrajectoryId: v.string(),
    // Snapshot of artifact data
    name: v.string(),
    content: v.string(),
    artifactType: v.string(),
    // Sync metadata
    syncedAt: v.number(),
  })
    .index('by_caliber_id', ['caliberArtifactId'])
    .index('by_trajectory', ['caliberTrajectoryId']),

  /**
   * Local cache of notes.
   *
   * Cross-trajectory knowledge for offline access.
   */
  noteCache: defineTable({
    // CALIBER ID (external)
    caliberNoteId: v.string(),
    // Snapshot of note data
    title: v.string(),
    content: v.string(),
    noteType: v.string(),
    // Sync metadata
    syncedAt: v.number(),
  })
    .index('by_caliber_id', ['caliberNoteId'])
    .index('by_type', ['noteType']),

  /**
   * Active sessions linking Convex users to CALIBER trajectories.
   *
   * Use this to track which user is working on which task.
   */
  activeSessions: defineTable({
    userId: v.string(),
    caliberTrajectoryId: v.string(),
    caliberScopeId: v.string(),
    startedAt: v.number(),
    lastActiveAt: v.number(),
  })
    .index('by_user', ['userId'])
    .index('by_trajectory', ['caliberTrajectoryId']),
});
