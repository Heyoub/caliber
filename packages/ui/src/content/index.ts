/**
 * Content CMS Public API - CALIBER/ForgeStack
 * Export content access functions with full type safety
 */

import type { Component } from 'svelte';
import {
  ContentLoader,
  getContentLoader,
  resolveContentPath,
  findIcon,
  findImage,
  getIconsByCategory,
  interpolate,
  type LoaderConfig,
  type LoadResult,
} from './loader';

import type {
  ContentStore,
  ContentNamespace,
  ContentKey,
  ContentPath,
  ContentValue,
  AssetRegistry,
  AssetId,
  IconId,
  IconCategory,
  IconDefinition,
  ImageDefinition,
  ImagePath,
} from './types';

// Re-export types
export type {
  ContentStore,
  ContentNamespace,
  ContentKey,
  ContentPath,
  ContentValue,
  AssetRegistry,
  AssetId,
  IconId,
  IconCategory,
  IconDefinition,
  ImageDefinition,
  ImagePath,
  LoaderConfig,
  LoadResult,
};

// Re-export type utilities
export {
  contentKey,
  assetId,
  iconId,
  imagePath,
  isValidContentKey,
  isValidAssetId,
} from './types';

// ============================================================================
// CONTENT STORE STATE
// ============================================================================

/**
 * Content store state (loaded at runtime)
 */
let contentStore: ContentStore | null = null;
let assetRegistry: AssetRegistry | null = null;
let initializationPromise: Promise<void> | null = null;

/**
 * Initialize the content system
 */
export async function initializeContent(config?: Partial<LoaderConfig>): Promise<void> {
  if (initializationPromise) {
    return initializationPromise;
  }

  initializationPromise = (async () => {
    const loader = getContentLoader(config);

    const [contentResult, assetResult] = await Promise.all([
      loader.loadAll(),
      loader.loadAssets(),
    ]);

    if (!contentResult.success || !contentResult.data) {
      console.error('Failed to load content:', contentResult.errors);
      throw new Error(`Content initialization failed: ${contentResult.errors.join(', ')}`);
    }

    if (!assetResult.success || !assetResult.data) {
      console.error('Failed to load assets:', assetResult.errors);
      throw new Error(`Asset initialization failed: ${assetResult.errors.join(', ')}`);
    }

    contentStore = contentResult.data;
    assetRegistry = assetResult.data;
  })();

  return initializationPromise;
}

/**
 * Check if content is initialized
 */
export function isContentInitialized(): boolean {
  return contentStore !== null && assetRegistry !== null;
}

/**
 * Get the content store (throws if not initialized)
 */
function requireContentStore(): ContentStore {
  if (!contentStore) {
    throw new Error('Content not initialized. Call initializeContent() first.');
  }
  return contentStore;
}

/**
 * Get the asset registry (throws if not initialized)
 */
function requireAssetRegistry(): AssetRegistry {
  if (!assetRegistry) {
    throw new Error('Assets not initialized. Call initializeContent() first.');
  }
  return assetRegistry;
}

// ============================================================================
// CONTENT ACCESS FUNCTIONS
// ============================================================================

/**
 * Get content at a specific path
 *
 * @example
 * ```typescript
 * const title = useContent('landing.hero.title.prefix'); // "Software That Feels Like"
 * const button = useContent('common.buttons.submit'); // "Submit"
 * ```
 */
export function useContent<P extends string>(path: P): ContentValue<P> {
  const store = requireContentStore();
  const [namespace, ...rest] = path.split('.') as [ContentNamespace, ...string[]];

  if (!namespace || !(namespace in store)) {
    throw new Error(`Invalid content namespace: ${namespace}`);
  }

  const namespaceContent = store[namespace];
  const value = resolveContentPath<ContentValue<P>>(
    namespaceContent as Record<string, unknown>,
    rest.join('.')
  );

  if (value === undefined) {
    throw new Error(`Content not found at path: ${path}`);
  }

  return value;
}

/**
 * Get content with optional fallback
 *
 * @example
 * ```typescript
 * const text = getContent('landing.hero.badge', 'Default Badge');
 * ```
 */
export function getContent<P extends string, F>(
  path: P,
  fallback: F
): ContentValue<P> | F {
  try {
    return useContent(path);
  } catch {
    return fallback;
  }
}

/**
 * Get content with variable interpolation
 *
 * @example
 * ```typescript
 * const greeting = useContentInterpolated('dashboard.sidebar.footer.greeting', { name: 'John' });
 * // "Hi, John"
 * ```
 */
export function useContentInterpolated<P extends string>(
  path: P,
  variables: Record<string, string | number>
): string {
  const content = useContent(path);

  if (typeof content !== 'string') {
    throw new Error(`Content at path ${path} is not a string for interpolation`);
  }

  return interpolate(content, variables);
}

/**
 * Get a namespace's entire content object
 *
 * @example
 * ```typescript
 * const landing = useNamespace('landing');
 * console.log(landing.hero.title.prefix);
 * ```
 */
export function useNamespace<N extends ContentNamespace>(
  namespace: N
): ContentStore[N] {
  const store = requireContentStore();
  return store[namespace];
}

// ============================================================================
// ASSET ACCESS FUNCTIONS
// ============================================================================

/**
 * Get an icon definition by ID
 *
 * @example
 * ```typescript
 * const icon = useAsset('icons', 'menu');
 * // { id: 'menu', lucide: 'Menu', description: 'Hamburger menu icon' }
 * ```
 */
export function useAsset(
  type: 'icons',
  id: IconId | string
): IconDefinition;
export function useAsset(
  type: 'images',
  id: AssetId | string
): ImageDefinition;
export function useAsset(
  type: 'icons' | 'images',
  id: string
): IconDefinition | ImageDefinition {
  const registry = requireAssetRegistry();

  if (type === 'icons') {
    const icon = findIcon(registry, id as IconId);
    if (!icon) {
      throw new Error(`Icon not found: ${id}`);
    }
    return icon;
  }

  const image = findImage(registry, id as AssetId);
  if (!image) {
    throw new Error(`Image not found: ${id}`);
  }
  return image;
}

/**
 * Get an icon's Lucide component name
 *
 * @example
 * ```typescript
 * const iconName = useIconName('menu'); // 'Menu'
 * ```
 */
export function useIconName(id: IconId | string): string {
  const icon = useAsset('icons', id);
  return icon.lucide;
}

/**
 * Get an image path
 *
 * @example
 * ```typescript
 * const logoPath = useImagePath('logo'); // '/IMG/AXLG.svg'
 * ```
 */
export function useImagePath(id: AssetId | string): ImagePath {
  const image = useAsset('images', id);
  return image.path;
}

/**
 * Get all icons in a category
 *
 * @example
 * ```typescript
 * const navIcons = useIconCategory('navigation');
 * ```
 */
export function useIconCategory(category: IconCategory): readonly IconDefinition[] {
  const registry = requireAssetRegistry();
  return getIconsByCategory(registry, category);
}

/**
 * Get the full asset registry
 */
export function useAssetRegistry(): AssetRegistry {
  return requireAssetRegistry();
}

// ============================================================================
// SVELTE STORE INTEGRATION
// ============================================================================

/**
 * Create a Svelte-compatible readable store for content
 *
 * @example
 * ```svelte
 * <script>
 *   import { createContentStore } from '@caliber/ui/content';
 *   const heroTitle = createContentStore('landing.hero.title.prefix');
 * </script>
 *
 * <h1>{$heroTitle}</h1>
 * ```
 */
export function createContentStore<P extends string>(path: P) {
  return {
    subscribe(callback: (value: ContentValue<P>) => void) {
      const value = useContent(path);
      callback(value);

      // Return unsubscribe function (no-op for static content)
      return () => {};
    },
  };
}

/**
 * Create a reactive content accessor for Svelte components
 *
 * @example
 * ```svelte
 * <script>
 *   import { t } from '@caliber/ui/content';
 * </script>
 *
 * <button>{t('common.buttons.submit')}</button>
 * ```
 */
export const t = useContent;

// ============================================================================
// DEVELOPMENT UTILITIES
// ============================================================================

/**
 * Debug: List all available content paths
 */
export function debugListContentPaths(): string[] {
  const store = requireContentStore();
  const paths: string[] = [];

  function traverse(obj: unknown, prefix: string): void {
    if (obj === null || obj === undefined) return;

    if (typeof obj === 'object' && !Array.isArray(obj)) {
      for (const [key, value] of Object.entries(obj)) {
        const path = prefix ? `${prefix}.${key}` : key;
        if (typeof value === 'string' || typeof value === 'number') {
          paths.push(path);
        } else {
          traverse(value, path);
        }
      }
    }
  }

  for (const [namespace, content] of Object.entries(store)) {
    traverse(content, namespace);
  }

  return paths;
}

/**
 * Debug: List all available asset IDs
 */
export function debugListAssetIds(): { icons: string[]; images: string[] } {
  const registry = requireAssetRegistry();

  const icons: string[] = [];
  for (const category of Object.keys(registry.icons) as IconCategory[]) {
    for (const icon of registry.icons[category]) {
      icons.push(`${category}/${icon.id}`);
    }
  }

  const images: string[] = [];
  for (const img of registry.images.brand) {
    images.push(`brand/${img.id}`);
  }
  for (const img of registry.images.decorative) {
    images.push(`decorative/${img.id}`);
  }

  return { icons, images };
}

// ============================================================================
// SSR SUPPORT
// ============================================================================

/**
 * Server-side content preloading
 * Call this in your SvelteKit load function
 */
export async function preloadContent(
  config?: Partial<LoaderConfig>
): Promise<{ content: ContentStore; assets: AssetRegistry }> {
  await initializeContent(config);
  return {
    content: requireContentStore(),
    assets: requireAssetRegistry(),
  };
}

/**
 * Hydrate content from server-rendered data
 */
export function hydrateContent(
  content: ContentStore,
  assets: AssetRegistry
): void {
  contentStore = content;
  assetRegistry = assets;
}
