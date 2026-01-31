/**
 * Memory Store
 * Manages current trajectory and scope state
 */
import { writable, derived } from 'svelte/store';
import { apiClient } from '$api/client';
import type { Trajectory, Scope, Event } from '$api/types';

export interface MemoryState {
  // Trajectories
  trajectories: Trajectory[];
  trajectory: Trajectory | null;

  // Scopes
  activeScope: Scope | null;
  scopeEvents: Event[];

  // Navigation
  selectedEventId: string | null;
  breadcrumb: Array<{ id: string; name: string; type: 'trajectory' | 'scope' | 'event' }>;

  // Loading states
  isLoadingTrajectories: boolean;
  isLoadingScope: boolean;
  isLoadingEvents: boolean;

  // Error
  error: string | null;
}

function createMemoryStore() {
  const initialState: MemoryState = {
    trajectories: [],
    trajectory: null,
    activeScope: null,
    scopeEvents: [],
    selectedEventId: null,
    breadcrumb: [],
    isLoadingTrajectories: false,
    isLoadingScope: false,
    isLoadingEvents: false,
    error: null,
  };

  const { subscribe, set, update } = writable<MemoryState>(initialState);

  return {
    subscribe,

    /**
     * Reset the store to initial state
     */
    reset: () => {
      set(initialState);
    },

    /**
     * Load all trajectories
     */
    loadTrajectories: async () => {
      update((state) => ({ ...state, isLoadingTrajectories: true, error: null }));

      try {
        const trajectories = await apiClient.getTrajectories();

        update((state) => ({
          ...state,
          trajectories,
          isLoadingTrajectories: false,
        }));

        return trajectories;
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to load trajectories',
          isLoadingTrajectories: false,
        }));
        return [];
      }
    },

    /**
     * Set the active trajectory
     */
    setTrajectory: (trajectory: Trajectory | null) => {
      update((state) => ({
        ...state,
        trajectory,
        activeScope: trajectory?.scopes[0] || null,
        scopeEvents: [],
        selectedEventId: null,
        breadcrumb: trajectory
          ? [{ id: trajectory.id, name: trajectory.name, type: 'trajectory' as const }]
          : [],
      }));
    },

    /**
     * Load a trajectory by ID
     */
    loadTrajectory: async (id: string) => {
      update((state) => ({ ...state, isLoadingTrajectories: true, error: null }));

      try {
        const trajectory = await apiClient.getTrajectory(id);

        update((state) => ({
          ...state,
          trajectory,
          activeScope: trajectory.scopes[0] || null,
          breadcrumb: [{ id: trajectory.id, name: trajectory.name, type: 'trajectory' as const }],
          isLoadingTrajectories: false,
        }));

        return trajectory;
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to load trajectory',
          isLoadingTrajectories: false,
        }));
        return null;
      }
    },

    /**
     * Set the active scope
     */
    setScope: async (scope: Scope | null) => {
      update((state) => ({
        ...state,
        activeScope: scope,
        scopeEvents: [],
        selectedEventId: null,
      }));

      // Update breadcrumb
      if (scope) {
        update((state) => ({
          ...state,
          breadcrumb: [
            ...state.breadcrumb.filter((b) => b.type === 'trajectory'),
            { id: scope.id, name: scope.name, type: 'scope' as const },
          ],
        }));

        // Load events for the scope
        await memoryStore.loadScopeEvents(scope.id);
      }
    },

    /**
     * Load events for a scope
     */
    loadScopeEvents: async (scopeId: string) => {
      update((state) => ({ ...state, isLoadingEvents: true }));

      try {
        const events = await apiClient.getScopeEvents(scopeId);

        update((state) => ({
          ...state,
          scopeEvents: events,
          isLoadingEvents: false,
        }));

        return events;
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to load events',
          isLoadingEvents: false,
        }));
        return [];
      }
    },

    /**
     * Select an event
     */
    selectEvent: (eventId: string | null) => {
      update((state) => {
        if (!eventId) {
          return {
            ...state,
            selectedEventId: null,
            breadcrumb: state.breadcrumb.filter((b) => b.type !== 'event'),
          };
        }

        const event = state.scopeEvents.find((e) => e.id === eventId);
        if (!event) return state;

        return {
          ...state,
          selectedEventId: eventId,
          breadcrumb: [
            ...state.breadcrumb.filter((b) => b.type !== 'event'),
            { id: eventId, name: event.type, type: 'event' as const },
          ],
        };
      });
    },

    /**
     * Navigate breadcrumb
     */
    navigateTo: (index: number) => {
      update((state) => {
        const item = state.breadcrumb[index];
        if (!item) return state;

        // Truncate breadcrumb to this point
        const newBreadcrumb = state.breadcrumb.slice(0, index + 1);

        return {
          ...state,
          breadcrumb: newBreadcrumb,
          // Reset selected event if navigating above event level
          selectedEventId: item.type === 'event' ? item.id : null,
        };
      });
    },

    /**
     * Create a new trajectory
     */
    createTrajectory: async (name: string, description?: string) => {
      try {
        const trajectory = await apiClient.createTrajectory(name, description);

        update((state) => ({
          ...state,
          trajectories: [trajectory, ...state.trajectories],
        }));

        return trajectory;
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to create trajectory',
        }));
        return null;
      }
    },

    /**
     * Delete a trajectory
     */
    deleteTrajectory: async (id: string) => {
      try {
        await apiClient.deleteTrajectory(id);

        update((state) => ({
          ...state,
          trajectories: state.trajectories.filter((t) => t.id !== id),
          trajectory: state.trajectory?.id === id ? null : state.trajectory,
        }));
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to delete trajectory',
        }));
      }
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

export const memoryStore = createMemoryStore();

// Derived stores for convenience
export const trajectories = derived(memoryStore, ($memory) => $memory.trajectories);
export const currentTrajectory = derived(memoryStore, ($memory) => $memory.trajectory);
export const activeScope = derived(memoryStore, ($memory) => $memory.activeScope);
export const scopeEvents = derived(memoryStore, ($memory) => $memory.scopeEvents);
export const selectedEventId = derived(memoryStore, ($memory) => $memory.selectedEventId);
export const breadcrumb = derived(memoryStore, ($memory) => $memory.breadcrumb);
export const memoryError = derived(memoryStore, ($memory) => $memory.error);

// Get the currently selected event
export const selectedEvent = derived(memoryStore, ($memory) =>
  $memory.selectedEventId
    ? $memory.scopeEvents.find((e) => e.id === $memory.selectedEventId) || null
    : null
);
