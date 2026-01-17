/**
 * Note Manager
 *
 * Manages notes (cross-trajectory knowledge) in CALIBER.
 */

import { BaseManager } from './base';
import type { HttpClient } from '../http';
import type {
  Note,
  CreateNoteParams,
  UpdateNoteParams,
  ListNotesParams,
  ListNotesResponse,
  SearchNotesParams,
  SearchResponse,
} from '../types';

/**
 * Manager for note operations
 */
export class NoteManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/notes');
  }

  /**
   * Create a new note
   */
  async create(params: CreateNoteParams): Promise<Note> {
    return this.http.post<Note>(this.basePath, params);
  }

  /**
   * Get a note by ID
   */
  async get(noteId: string): Promise<Note> {
    return this.http.get<Note>(this.pathWithId(noteId));
  }

  /**
   * List notes with optional filters
   */
  async list(params: ListNotesParams = {}): Promise<ListNotesResponse> {
    return this.http.get<ListNotesResponse>(this.basePath, {
      params: this.buildParams(params),
    });
  }

  /**
   * Update a note
   */
  async update(noteId: string, params: UpdateNoteParams): Promise<Note> {
    return this.http.patch<Note>(this.pathWithId(noteId), params);
  }

  /**
   * Delete a note
   */
  async delete(noteId: string): Promise<void> {
    await this.http.delete(this.pathWithId(noteId));
  }

  /**
   * Search notes by content
   */
  async search(params: SearchNotesParams): Promise<SearchResponse> {
    return this.http.post<SearchResponse>(`${this.basePath}/search`, params);
  }
}
