/**
 * Stores Index
 * Re-exports all stores for convenient imports
 * Note: Some exports are renamed to avoid conflicts
 */

// Mode store
export * from './mode';

// Auth store
export * from './auth';

// Assistant store - rename conflicting exports
export {
  assistantStore,
  messages,
  isStreaming,
  pendingToolCalls,
  activeTrajectory as assistantActiveTrajectory,
  activeScope as assistantActiveScope,
  assistantError,
  type AssistantState,
} from './assistant';

// Playground store
export * from './playground';

// Editor store
export * from './editor';

// Memory store (primary source for activeScope)
export * from './memory';
