/**
 * Mode Store
 * Manages the current editor mode (assistant | playground)
 */
import { writable, derived } from 'svelte/store';

export type EditorMode = 'assistant' | 'playground';

interface ModeState {
  mode: EditorMode;
  isTransitioning: boolean;
}

function createModeStore() {
  const { subscribe, set, update } = writable<ModeState>({
    mode: 'assistant',
    isTransitioning: false,
  });

  return {
    subscribe,

    /**
     * Set the current editor mode
     */
    setMode: (mode: EditorMode) => {
      update((state) => ({
        ...state,
        mode,
        isTransitioning: false,
      }));
    },

    /**
     * Toggle between assistant and playground modes
     */
    toggle: () => {
      update((state) => ({
        ...state,
        mode: state.mode === 'assistant' ? 'playground' : 'assistant',
      }));
    },

    /**
     * Begin a mode transition (for animations)
     */
    startTransition: () => {
      update((state) => ({
        ...state,
        isTransitioning: true,
      }));
    },

    /**
     * End a mode transition
     */
    endTransition: () => {
      update((state) => ({
        ...state,
        isTransitioning: false,
      }));
    },
  };
}

export const modeStore = createModeStore();

// Derived stores for convenience
export const currentMode = derived(modeStore, ($mode) => $mode.mode);
export const isAssistantMode = derived(modeStore, ($mode) => $mode.mode === 'assistant');
export const isPlaygroundMode = derived(modeStore, ($mode) => $mode.mode === 'playground');
export const isTransitioning = derived(modeStore, ($mode) => $mode.isTransitioning);
