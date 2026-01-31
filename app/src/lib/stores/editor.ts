/**
 * Editor Store
 * Manages editor state (open files, active tab, editor mode)
 */
import { writable, derived } from 'svelte/store';

export interface FileDescriptor {
  path: string;
  name: string;
  content?: string;
  mimeType: string;
  isDirty: boolean;
  isLoading: boolean;
}

export type ViewMode = 'edit' | 'preview' | 'split';

export interface EditorState {
  openFiles: FileDescriptor[];
  activeFile: FileDescriptor | null;
  viewMode: ViewMode;
  isFullscreen: boolean;
  showMinimap: boolean;
  wordWrap: boolean;
  fontSize: number;
}

function createEditorStore() {
  const initialState: EditorState = {
    openFiles: [],
    activeFile: null,
    viewMode: 'edit',
    isFullscreen: false,
    showMinimap: true,
    wordWrap: true,
    fontSize: 14,
  };

  const { subscribe, set, update } = writable<EditorState>(initialState);

  return {
    subscribe,

    /**
     * Reset the store to initial state
     */
    reset: () => {
      set(initialState);
    },

    /**
     * Open a file in the editor
     */
    openFile: (
      file: Omit<FileDescriptor, 'isDirty' | 'isLoading'> &
        Partial<Pick<FileDescriptor, 'isDirty' | 'isLoading'>>
    ) => {
      update((state) => {
        // Check if file is already open
        const existingIndex = state.openFiles.findIndex((f) => f.path === file.path);

        const fileDescriptor: FileDescriptor = {
          ...file,
          isDirty: file.isDirty ?? false,
          isLoading: file.isLoading ?? false,
        };

        if (existingIndex !== -1) {
          // File already open, just make it active
          return {
            ...state,
            activeFile: state.openFiles[existingIndex],
          };
        }

        // Add to open files and make active
        return {
          ...state,
          openFiles: [...state.openFiles, fileDescriptor],
          activeFile: fileDescriptor,
        };
      });
    },

    /**
     * Close a file
     */
    closeFile: (path: string) => {
      update((state) => {
        const newOpenFiles = state.openFiles.filter((f) => f.path !== path);
        let newActiveFile = state.activeFile;

        // If closing the active file, switch to another
        if (state.activeFile?.path === path) {
          const closedIndex = state.openFiles.findIndex((f) => f.path === path);
          newActiveFile = newOpenFiles[closedIndex] || newOpenFiles[closedIndex - 1] || null;
        }

        return {
          ...state,
          openFiles: newOpenFiles,
          activeFile: newActiveFile,
        };
      });
    },

    /**
     * Set the active file
     */
    setActiveFile: (file: FileDescriptor | null) => {
      update((state) => ({
        ...state,
        activeFile: file,
      }));
    },

    /**
     * Update the active file's content
     */
    updateActiveFileContent: (content: string) => {
      update((state) => {
        if (!state.activeFile) return state;

        const updatedFile: FileDescriptor = {
          ...state.activeFile,
          content,
          isDirty: true,
        };

        return {
          ...state,
          activeFile: updatedFile,
          openFiles: state.openFiles.map((f) => (f.path === updatedFile.path ? updatedFile : f)),
        };
      });
    },

    /**
     * Mark a file as saved (not dirty)
     */
    markFileSaved: (path: string) => {
      update((state) => {
        const updateFile = (f: FileDescriptor): FileDescriptor =>
          f.path === path ? { ...f, isDirty: false } : f;

        return {
          ...state,
          activeFile: state.activeFile ? updateFile(state.activeFile) : null,
          openFiles: state.openFiles.map(updateFile),
        };
      });
    },

    /**
     * Set loading state for a file
     */
    setFileLoading: (path: string, loading: boolean) => {
      update((state) => {
        const updateFile = (f: FileDescriptor): FileDescriptor =>
          f.path === path ? { ...f, isLoading: loading } : f;

        return {
          ...state,
          activeFile: state.activeFile ? updateFile(state.activeFile) : null,
          openFiles: state.openFiles.map(updateFile),
        };
      });
    },

    /**
     * Set the view mode
     */
    setViewMode: (mode: ViewMode) => {
      update((state) => ({
        ...state,
        viewMode: mode,
      }));
    },

    /**
     * Toggle view mode between edit and preview
     */
    toggleViewMode: () => {
      update((state) => ({
        ...state,
        viewMode: state.viewMode === 'edit' ? 'preview' : 'edit',
      }));
    },

    /**
     * Toggle fullscreen mode
     */
    toggleFullscreen: () => {
      update((state) => ({
        ...state,
        isFullscreen: !state.isFullscreen,
      }));
    },

    /**
     * Toggle minimap
     */
    toggleMinimap: () => {
      update((state) => ({
        ...state,
        showMinimap: !state.showMinimap,
      }));
    },

    /**
     * Toggle word wrap
     */
    toggleWordWrap: () => {
      update((state) => ({
        ...state,
        wordWrap: !state.wordWrap,
      }));
    },

    /**
     * Set font size
     */
    setFontSize: (size: number) => {
      update((state) => ({
        ...state,
        fontSize: Math.max(10, Math.min(24, size)),
      }));
    },

    /**
     * Increase font size
     */
    increaseFontSize: () => {
      update((state) => ({
        ...state,
        fontSize: Math.min(24, state.fontSize + 1),
      }));
    },

    /**
     * Decrease font size
     */
    decreaseFontSize: () => {
      update((state) => ({
        ...state,
        fontSize: Math.max(10, state.fontSize - 1),
      }));
    },

    /**
     * Close all files
     */
    closeAllFiles: () => {
      update((state) => ({
        ...state,
        openFiles: [],
        activeFile: null,
      }));
    },

    /**
     * Close other files (keep active)
     */
    closeOtherFiles: () => {
      update((state) => ({
        ...state,
        openFiles: state.activeFile ? [state.activeFile] : [],
      }));
    },

    /**
     * Reorder open files
     */
    reorderFiles: (fromIndex: number, toIndex: number) => {
      update((state) => {
        const files = [...state.openFiles];
        const [removed] = files.splice(fromIndex, 1);
        files.splice(toIndex, 0, removed);

        return {
          ...state,
          openFiles: files,
        };
      });
    },
  };
}

export const editorStore = createEditorStore();

// Derived stores for convenience
export const openFiles = derived(editorStore, ($editor) => $editor.openFiles);
export const activeFile = derived(editorStore, ($editor) => $editor.activeFile);
export const viewMode = derived(editorStore, ($editor) => $editor.viewMode);
export const hasDirtyFiles = derived(editorStore, ($editor) =>
  $editor.openFiles.some((f) => f.isDirty)
);
export const dirtyFileCount = derived(
  editorStore,
  ($editor) => $editor.openFiles.filter((f) => f.isDirty).length
);
