/**
 * Assistant Page Load Function
 * Loads data from CALIBER API
 */
import { apiClient } from '$api/client';
import { modeStore } from '$stores/mode';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, depends }) => {
  // Mark this load as dependent on assistant data
  depends('app:assistant');

  // Set editor mode to assistant
  modeStore.setMode('assistant');

  try {
    // Fetch initial data from API
    const [trajectories, activeTrajectory] = await Promise.all([
      apiClient.getTrajectories(),
      apiClient.getActiveTrajectory(),
    ]);

    return {
      trajectories,
      activeTrajectory,
      mode: 'assistant' as const,
    };
  } catch (error) {
    console.error('Failed to load assistant data:', error);

    return {
      trajectories: [],
      activeTrajectory: null,
      mode: 'assistant' as const,
      error: error instanceof Error ? error.message : 'Failed to load data',
    };
  }
};
