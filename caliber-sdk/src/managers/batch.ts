/**
 * Batch Operations Manager
 *
 * Provides bulk create/update/delete operations for improved performance.
 * Useful when working with many entities at once, such as during initial
 * setup or bulk imports.
 */

import type { HttpClient } from '../http';
import type {
  Trajectory,
  Artifact,
  Note,
  CreateTrajectoryParams,
  UpdateTrajectoryParams,
  CreateArtifactParams,
  UpdateArtifactParams,
  CreateNoteParams,
  UpdateNoteParams,
} from '../types';

/**
 * Batch operation type
 */
export type BatchOperation = 'create' | 'update' | 'delete';

/**
 * Result of a single batch item operation
 */
export type BatchItemResult<T> =
  | { status: 'success'; data: T }
  | { status: 'error'; message: string; code: string };

/**
 * Trajectory batch item for bulk operations
 */
export interface TrajectoryBatchItem {
  /** Operation to perform */
  operation: BatchOperation;
  /** Trajectory ID (required for update/delete) */
  trajectory_id?: string;
  /** Create data (required for create operation) */
  create?: CreateTrajectoryParams;
  /** Update data (required for update operation) */
  update?: UpdateTrajectoryParams;
}

/**
 * Artifact batch item for bulk operations
 */
export interface ArtifactBatchItem {
  /** Operation to perform */
  operation: BatchOperation;
  /** Artifact ID (required for update/delete) */
  artifact_id?: string;
  /** Create data (required for create operation) */
  create?: CreateArtifactParams;
  /** Update data (required for update operation) */
  update?: UpdateArtifactParams;
}

/**
 * Note batch item for bulk operations
 */
export interface NoteBatchItem {
  /** Operation to perform */
  operation: BatchOperation;
  /** Note ID (required for update/delete) */
  note_id?: string;
  /** Create data (required for create operation) */
  create?: CreateNoteParams;
  /** Update data (required for update operation) */
  update?: UpdateNoteParams;
}

/**
 * Batch request parameters
 */
export interface BatchRequestParams<T> {
  /** Items to process */
  items: T[];
  /** Stop processing on first error (default: false) */
  stop_on_error?: boolean;
}

/**
 * Batch response
 */
export interface BatchResponse<T> {
  /** Results for each item */
  results: BatchItemResult<T>[];
  /** Number of successful operations */
  succeeded: number;
  /** Number of failed operations */
  failed: number;
}

/**
 * Batch Manager
 *
 * Provides bulk operations for trajectories, artifacts, and notes.
 *
 * @example
 * ```typescript
 * import { CalibrClient } from '@caliber-run/sdk';
 *
 * const client = new CalibrClient({ apiKey: '...', tenantId: '...' });
 * const batch = new BatchManager(client);
 *
 * // Bulk create trajectories
 * const result = await batch.trajectories({
 *   items: [
 *     { operation: 'create', create: { name: 'Task 1' } },
 *     { operation: 'create', create: { name: 'Task 2' } },
 *     { operation: 'create', create: { name: 'Task 3' } },
 *   ],
 * });
 *
 * console.log(`Created ${result.succeeded} trajectories, ${result.failed} failed`);
 * ```
 */
export class BatchManager {
  private readonly http: HttpClient;

  constructor(http: HttpClient) {
    this.http = http;
  }

  /**
   * Perform batch operations on trajectories
   *
   * @param params - Batch request parameters
   * @returns Batch response with results for each item
   */
  async trajectories(
    params: BatchRequestParams<TrajectoryBatchItem>
  ): Promise<BatchResponse<Trajectory>> {
    return this.http.post<BatchResponse<Trajectory>>(
      '/api/v1/batch/trajectories',
      {
        items: params.items,
        stop_on_error: params.stop_on_error ?? false,
      }
    );
  }

  /**
   * Perform batch operations on artifacts
   *
   * @param params - Batch request parameters
   * @returns Batch response with results for each item
   */
  async artifacts(
    params: BatchRequestParams<ArtifactBatchItem>
  ): Promise<BatchResponse<Artifact>> {
    return this.http.post<BatchResponse<Artifact>>(
      '/api/v1/batch/artifacts',
      {
        items: params.items,
        stop_on_error: params.stop_on_error ?? false,
      }
    );
  }

  /**
   * Perform batch operations on notes
   *
   * @param params - Batch request parameters
   * @returns Batch response with results for each item
   */
  async notes(
    params: BatchRequestParams<NoteBatchItem>
  ): Promise<BatchResponse<Note>> {
    return this.http.post<BatchResponse<Note>>(
      '/api/v1/batch/notes',
      {
        items: params.items,
        stop_on_error: params.stop_on_error ?? false,
      }
    );
  }

  // ============================================================================
  // Convenience Methods
  // ============================================================================

  /**
   * Bulk create trajectories
   *
   * @param trajectories - Array of trajectory creation parameters
   * @param stopOnError - Stop on first error (default: false)
   * @returns Batch response with created trajectories
   */
  async createTrajectories(
    trajectories: CreateTrajectoryParams[],
    stopOnError = false
  ): Promise<BatchResponse<Trajectory>> {
    return this.trajectories({
      items: trajectories.map((create) => ({
        operation: 'create',
        create,
      })),
      stop_on_error: stopOnError,
    });
  }

  /**
   * Bulk create artifacts
   *
   * @param artifacts - Array of artifact creation parameters
   * @param stopOnError - Stop on first error (default: false)
   * @returns Batch response with created artifacts
   */
  async createArtifacts(
    artifacts: CreateArtifactParams[],
    stopOnError = false
  ): Promise<BatchResponse<Artifact>> {
    return this.artifacts({
      items: artifacts.map((create) => ({
        operation: 'create',
        create,
      })),
      stop_on_error: stopOnError,
    });
  }

  /**
   * Bulk create notes
   *
   * @param notes - Array of note creation parameters
   * @param stopOnError - Stop on first error (default: false)
   * @returns Batch response with created notes
   */
  async createNotes(
    notes: CreateNoteParams[],
    stopOnError = false
  ): Promise<BatchResponse<Note>> {
    return this.notes({
      items: notes.map((create) => ({
        operation: 'create',
        create,
      })),
      stop_on_error: stopOnError,
    });
  }

  /**
   * Bulk delete trajectories
   *
   * @param trajectoryIds - Array of trajectory IDs to delete
   * @param stopOnError - Stop on first error (default: false)
   * @returns Batch response with deleted trajectories
   */
  async deleteTrajectories(
    trajectoryIds: string[],
    stopOnError = false
  ): Promise<BatchResponse<Trajectory>> {
    return this.trajectories({
      items: trajectoryIds.map((trajectory_id) => ({
        operation: 'delete',
        trajectory_id,
      })),
      stop_on_error: stopOnError,
    });
  }

  /**
   * Bulk delete artifacts
   *
   * @param artifactIds - Array of artifact IDs to delete
   * @param stopOnError - Stop on first error (default: false)
   * @returns Batch response with deleted artifacts
   */
  async deleteArtifacts(
    artifactIds: string[],
    stopOnError = false
  ): Promise<BatchResponse<Artifact>> {
    return this.artifacts({
      items: artifactIds.map((artifact_id) => ({
        operation: 'delete',
        artifact_id,
      })),
      stop_on_error: stopOnError,
    });
  }

  /**
   * Bulk delete notes
   *
   * @param noteIds - Array of note IDs to delete
   * @param stopOnError - Stop on first error (default: false)
   * @returns Batch response with deleted notes
   */
  async deleteNotes(
    noteIds: string[],
    stopOnError = false
  ): Promise<BatchResponse<Note>> {
    return this.notes({
      items: noteIds.map((note_id) => ({
        operation: 'delete',
        note_id,
      })),
      stop_on_error: stopOnError,
    });
  }
}
