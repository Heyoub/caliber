/**
 * Playground Page Load Function
 * Loads data from IndexedDB (client-side storage)
 */
import { playgroundStore } from '$stores/playground';
import { modeStore } from '$stores/mode';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ depends }) => {
  // Mark this load as dependent on playground data
  depends('app:playground');

  // Set editor mode to playground
  modeStore.setMode('playground');

  try {
    // Initialize playground store and load files from IndexedDB
    await playgroundStore.initialize();
    const files = await playgroundStore.loadFiles();

    return {
      files,
      mode: 'playground' as const,
    };
  } catch (error) {
    console.error('Failed to load playground data:', error);

    return {
      files: [],
      mode: 'playground' as const,
      error: error instanceof Error ? error.message : 'Failed to load data',
    };
  }
};
