/**
 * Assistant Store
 * Manages state for the real API-backed assistant mode
 */
import { writable, derived } from 'svelte/store';
import { apiClient } from '$api/client';
import type { ToolCall, Message, Trajectory, Scope } from '$api/types';

export interface AssistantState {
  // Conversation
  messages: Message[];
  isStreaming: boolean;
  pendingToolCalls: ToolCall[];

  // Current context
  activeTrajectory: Trajectory | null;
  activeScope: Scope | null;

  // Loading states
  isLoading: boolean;
  isSending: boolean;

  // Errors
  error: string | null;
}

function createAssistantStore() {
  const initialState: AssistantState = {
    messages: [],
    isStreaming: false,
    pendingToolCalls: [],
    activeTrajectory: null,
    activeScope: null,
    isLoading: false,
    isSending: false,
    error: null,
  };

  const { subscribe, set, update } = writable<AssistantState>(initialState);

  return {
    subscribe,

    /**
     * Reset the store to initial state
     */
    reset: () => {
      set(initialState);
    },

    /**
     * Set the active trajectory
     */
    setTrajectory: (trajectory: Trajectory | null) => {
      update((state) => ({
        ...state,
        activeTrajectory: trajectory,
        activeScope: trajectory?.scopes[0] || null,
      }));
    },

    /**
     * Set the active scope
     */
    setScope: (scope: Scope | null) => {
      update((state) => ({
        ...state,
        activeScope: scope,
      }));
    },

    /**
     * Send a message to the assistant
     */
    sendMessage: async (content: string) => {
      update((state) => ({
        ...state,
        isSending: true,
        error: null,
      }));

      // Add user message immediately
      const userMessage: Message = {
        id: crypto.randomUUID(),
        role: 'user',
        content,
        timestamp: new Date().toISOString(),
      };

      update((state) => ({
        ...state,
        messages: [...state.messages, userMessage],
      }));

      try {
        // Send to API and get response
        const response = await apiClient.sendMessage(content);

        // Add assistant response
        const assistantMessage: Message = {
          id: response.id,
          role: 'assistant',
          content: response.content,
          timestamp: response.timestamp,
          toolCalls: response.toolCalls,
        };

        update((state) => ({
          ...state,
          messages: [...state.messages, assistantMessage],
          pendingToolCalls: response.toolCalls?.filter((tc) => tc.status === 'pending') || [],
          isSending: false,
        }));
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to send message',
          isSending: false,
        }));
      }
    },

    /**
     * Approve a pending tool call
     */
    approveToolCall: async (toolCallId: string) => {
      try {
        await apiClient.approveToolCall(toolCallId);

        update((state) => ({
          ...state,
          pendingToolCalls: state.pendingToolCalls.filter((tc) => tc.id !== toolCallId),
        }));
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to approve tool call',
        }));
      }
    },

    /**
     * Reject a pending tool call
     */
    rejectToolCall: async (toolCallId: string) => {
      try {
        await apiClient.rejectToolCall(toolCallId);

        update((state) => ({
          ...state,
          pendingToolCalls: state.pendingToolCalls.filter((tc) => tc.id !== toolCallId),
        }));
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to reject tool call',
        }));
      }
    },

    /**
     * Clear the conversation
     */
    clearMessages: () => {
      update((state) => ({
        ...state,
        messages: [],
        pendingToolCalls: [],
      }));
    },

    /**
     * Set loading state
     */
    setLoading: (loading: boolean) => {
      update((state) => ({ ...state, isLoading: loading }));
    },

    /**
     * Set error
     */
    setError: (error: string | null) => {
      update((state) => ({ ...state, error }));
    },

    /**
     * Clear error
     */
    clearError: () => {
      update((state) => ({ ...state, error: null }));
    },
  };
}

export const assistantStore = createAssistantStore();

// Derived stores for convenience
export const messages = derived(assistantStore, ($assistant) => $assistant.messages);
export const isStreaming = derived(assistantStore, ($assistant) => $assistant.isStreaming);
export const pendingToolCalls = derived(assistantStore, ($assistant) => $assistant.pendingToolCalls);
export const activeTrajectory = derived(assistantStore, ($assistant) => $assistant.activeTrajectory);
export const activeScope = derived(assistantStore, ($assistant) => $assistant.activeScope);
export const assistantError = derived(assistantStore, ($assistant) => $assistant.error);
