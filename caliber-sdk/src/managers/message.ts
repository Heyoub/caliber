/**
 * Message Manager
 *
 * Manages inter-agent messaging in CALIBER.
 */

import { BaseManager } from './base';
import { HttpClient } from '../http';
import type {
  Message,
  SendMessageParams,
  ListMessagesParams,
  ListMessagesResponse,
} from '../types';

/**
 * Manager for message operations
 */
export class MessageManager extends BaseManager {
  constructor(http: HttpClient) {
    super(http, '/api/v1/messages');
  }

  /**
   * Send a message
   */
  async send(params: SendMessageParams): Promise<Message> {
    return this.http.post<Message>(this.basePath, params);
  }

  /**
   * Get a message by ID
   */
  async get(messageId: string): Promise<Message> {
    return this.http.get<Message>(this.pathWithId(messageId));
  }

  /**
   * List messages with optional filters
   */
  async list(params: ListMessagesParams = {}): Promise<ListMessagesResponse> {
    return this.http.get<ListMessagesResponse>(this.basePath, {
      params: this.buildParams(params),
    });
  }

  /**
   * Acknowledge receipt of a message
   */
  async acknowledge(messageId: string): Promise<Message> {
    return this.http.post<Message>(`${this.pathWithId(messageId)}/acknowledge`);
  }
}
