/**
 * Playground Store
 * Client-side sandbox store using IndexedDB for persistence
 */
import { writable, derived } from 'svelte/store';
import { openDB, type IDBPDatabase } from 'idb';

export interface PlaygroundFile {
  path: string;
  name: string;
  content: string;
  mimeType: string;
  createdAt: string;
  updatedAt: string;
}

export interface PlaygroundState {
  files: PlaygroundFile[];
  isInitialized: boolean;
  isLoading: boolean;
  error: string | null;
}

// IndexedDB configuration
const DB_NAME = 'caliber-playground';
const DB_VERSION = 1;
const FILES_STORE = 'files';

// MIME type mapping
const MIME_TYPES: Record<string, string> = {
  yaml: 'application/x-yaml',
  yml: 'application/x-yaml',
  toml: 'application/toml',
  json: 'application/json',
  md: 'text/markdown',
  csv: 'text/csv',
  xml: 'application/xml',
  txt: 'text/plain',
};

function getMimeType(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() || '';
  return MIME_TYPES[ext] || 'text/plain';
}

function createPlaygroundStore() {
  const initialState: PlaygroundState = {
    files: [],
    isInitialized: false,
    isLoading: false,
    error: null,
  };

  const { subscribe, set, update } = writable<PlaygroundState>(initialState);

  let db: IDBPDatabase | null = null;

  return {
    subscribe,

    /**
     * Initialize IndexedDB connection
     */
    initialize: async () => {
      if (typeof window === 'undefined') return;

      update((state) => ({ ...state, isLoading: true }));

      try {
        db = await openDB(DB_NAME, DB_VERSION, {
          upgrade(database) {
            if (!database.objectStoreNames.contains(FILES_STORE)) {
              const store = database.createObjectStore(FILES_STORE, { keyPath: 'path' });
              store.createIndex('name', 'name', { unique: false });
              store.createIndex('updatedAt', 'updatedAt', { unique: false });
            }
          },
        });

        update((state) => ({
          ...state,
          isInitialized: true,
          isLoading: false,
        }));
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to initialize database',
          isLoading: false,
        }));
      }
    },

    /**
     * Load all files from IndexedDB
     */
    loadFiles: async (): Promise<PlaygroundFile[]> => {
      if (!db) {
        throw new Error('Database not initialized');
      }

      update((state) => ({ ...state, isLoading: true }));

      try {
        const files = await db.getAll(FILES_STORE);

        // Sort by updatedAt descending
        files.sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime());

        update((state) => ({
          ...state,
          files,
          isLoading: false,
        }));

        return files;
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to load files',
          isLoading: false,
        }));
        return [];
      }
    },

    /**
     * Create a new file
     */
    createFile: async (name: string, content: string = '') => {
      if (!db) {
        throw new Error('Database not initialized');
      }

      const now = new Date().toISOString();
      const file: PlaygroundFile = {
        path: `/${name}`,
        name,
        content,
        mimeType: getMimeType(name),
        createdAt: now,
        updatedAt: now,
      };

      try {
        await db.put(FILES_STORE, file);

        update((state) => ({
          ...state,
          files: [file, ...state.files],
        }));

        return file;
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to create file',
        }));
        throw error;
      }
    },

    /**
     * Update a file's content
     */
    updateFile: async (path: string, content: string) => {
      if (!db) {
        throw new Error('Database not initialized');
      }

      try {
        const file = await db.get(FILES_STORE, path);
        if (!file) {
          throw new Error('File not found');
        }

        const updatedFile: PlaygroundFile = {
          ...file,
          content,
          updatedAt: new Date().toISOString(),
        };

        await db.put(FILES_STORE, updatedFile);

        update((state) => ({
          ...state,
          files: state.files.map((f) => (f.path === path ? updatedFile : f)),
        }));

        return updatedFile;
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to update file',
        }));
        throw error;
      }
    },

    /**
     * Rename a file
     */
    renameFile: async (oldPath: string, newName: string) => {
      if (!db) {
        throw new Error('Database not initialized');
      }

      try {
        const file = await db.get(FILES_STORE, oldPath);
        if (!file) {
          throw new Error('File not found');
        }

        const newPath = `/${newName}`;
        const updatedFile: PlaygroundFile = {
          ...file,
          path: newPath,
          name: newName,
          mimeType: getMimeType(newName),
          updatedAt: new Date().toISOString(),
        };

        // Delete old and create new
        await db.delete(FILES_STORE, oldPath);
        await db.put(FILES_STORE, updatedFile);

        update((state) => ({
          ...state,
          files: state.files.map((f) => (f.path === oldPath ? updatedFile : f)),
        }));

        return updatedFile;
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to rename file',
        }));
        throw error;
      }
    },

    /**
     * Delete a file
     */
    deleteFile: async (path: string) => {
      if (!db) {
        throw new Error('Database not initialized');
      }

      try {
        await db.delete(FILES_STORE, path);

        update((state) => ({
          ...state,
          files: state.files.filter((f) => f.path !== path),
        }));
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to delete file',
        }));
        throw error;
      }
    },

    /**
     * Get a single file
     */
    getFile: async (path: string): Promise<PlaygroundFile | null> => {
      if (!db) {
        throw new Error('Database not initialized');
      }

      try {
        const file = await db.get(FILES_STORE, path);
        return file || null;
      } catch (error) {
        return null;
      }
    },

    /**
     * Import files (from JSON)
     */
    importFiles: async (files: Array<{ name: string; content: string }>) => {
      for (const file of files) {
        await playgroundStore.createFile(file.name, file.content);
      }
    },

    /**
     * Export all files
     */
    exportFiles: async (): Promise<Array<{ name: string; content: string }>> => {
      if (!db) {
        throw new Error('Database not initialized');
      }

      const files = await db.getAll(FILES_STORE);
      return files.map((f) => ({ name: f.name, content: f.content }));
    },

    /**
     * Clear all files
     */
    clearAll: async () => {
      if (!db) {
        throw new Error('Database not initialized');
      }

      try {
        await db.clear(FILES_STORE);

        update((state) => ({
          ...state,
          files: [],
        }));
      } catch (error) {
        update((state) => ({
          ...state,
          error: error instanceof Error ? error.message : 'Failed to clear files',
        }));
        throw error;
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

export const playgroundStore = createPlaygroundStore();

// Derived stores for convenience
export const playgroundFiles = derived(playgroundStore, ($playground) => $playground.files);
export const isPlaygroundInitialized = derived(
  playgroundStore,
  ($playground) => $playground.isInitialized
);
export const playgroundError = derived(playgroundStore, ($playground) => $playground.error);
