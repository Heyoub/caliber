/**
 * Content Loader - CALIBER/ForgeStack
 * YAML loader with validation for type-safe content access
 */

import { parse as parseYaml } from 'yaml';
import type {
  ContentStore,
  ContentNamespace,
  ContentKey,
  AssetRegistry,
  AssetId,
  IconId,
  IconCategory,
  IconDefinition,
  ImageDefinition,
  LandingContent,
  DashboardContent,
  EditorContent,
  CommonContent,
} from './types';

// ============================================================================
// CONTENT LOADER
// ============================================================================

/**
 * Content loading result with validation status
 */
export interface LoadResult<T> {
  readonly success: boolean;
  readonly data: T | null;
  readonly errors: readonly string[];
}

/**
 * Content loader configuration
 */
export interface LoaderConfig {
  /** Base path for content files */
  readonly basePath: string;
  /** Locale for content loading */
  readonly locale: string;
  /** Whether to throw on validation errors */
  readonly strict: boolean;
  /** Cache TTL in milliseconds (0 = no cache) */
  readonly cacheTtl: number;
}

const DEFAULT_CONFIG: LoaderConfig = {
  basePath: '/content',
  locale: 'en',
  strict: false,
  cacheTtl: 300000, // 5 minutes
};

/**
 * Content cache entry
 */
interface CacheEntry<T> {
  data: T;
  timestamp: number;
}

/**
 * Content loader class with caching and validation
 */
export class ContentLoader {
  private readonly config: LoaderConfig;
  private readonly cache: Map<string, CacheEntry<unknown>> = new Map();

  constructor(config: Partial<LoaderConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * Load content for a specific namespace
   */
  async loadNamespace<N extends ContentNamespace>(
    namespace: N
  ): Promise<LoadResult<ContentStore[N]>> {
    const cacheKey = `${this.config.locale}/${namespace}`;

    // Check cache
    const cached = this.getFromCache<ContentStore[N]>(cacheKey);
    if (cached) {
      return { success: true, data: cached, errors: [] };
    }

    try {
      const path = `${this.config.basePath}/${this.config.locale}/${namespace}.yaml`;
      const response = await fetch(path);

      if (!response.ok) {
        throw new Error(`Failed to load content: ${response.statusText}`);
      }

      const yamlContent = await response.text();
      const parsed = parseYaml(yamlContent) as ContentStore[N];

      // Validate structure
      const errors = this.validateNamespace(namespace, parsed);

      if (errors.length > 0 && this.config.strict) {
        return { success: false, data: null, errors };
      }

      // Cache result
      this.setCache(cacheKey, parsed);

      return { success: true, data: parsed, errors };
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      return { success: false, data: null, errors: [message] };
    }
  }

  /**
   * Load all content namespaces
   */
  async loadAll(): Promise<LoadResult<ContentStore>> {
    const namespaces: ContentNamespace[] = ['landing', 'dashboard', 'editor', 'common'];
    const results = await Promise.all(namespaces.map((ns) => this.loadNamespace(ns)));

    const errors: string[] = [];
    const data: Partial<ContentStore> = {};

    for (let i = 0; i < namespaces.length; i++) {
      const result = results[i];
      if (result.success && result.data) {
        (data as Record<string, unknown>)[namespaces[i]] = result.data;
      }
      errors.push(...result.errors.map((e) => `[${namespaces[i]}] ${e}`));
    }

    const success = namespaces.every((_, i) => results[i].success);

    return {
      success,
      data: success ? (data as ContentStore) : null,
      errors,
    };
  }

  /**
   * Load asset registry
   */
  async loadAssets(): Promise<LoadResult<AssetRegistry>> {
    const cacheKey = 'assets/registry';

    const cached = this.getFromCache<AssetRegistry>(cacheKey);
    if (cached) {
      return { success: true, data: cached, errors: [] };
    }

    try {
      const path = `${this.config.basePath}/../assets/registry.yaml`;
      const response = await fetch(path);

      if (!response.ok) {
        throw new Error(`Failed to load assets: ${response.statusText}`);
      }

      const yamlContent = await response.text();
      const parsed = parseYaml(yamlContent) as AssetRegistry;

      // Validate structure
      const errors = this.validateAssets(parsed);

      if (errors.length > 0 && this.config.strict) {
        return { success: false, data: null, errors };
      }

      this.setCache(cacheKey, parsed);

      return { success: true, data: parsed, errors };
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      return { success: false, data: null, errors: [message] };
    }
  }

  /**
   * Get a value from cache
   */
  private getFromCache<T>(key: string): T | null {
    if (this.config.cacheTtl === 0) return null;

    const entry = this.cache.get(key) as CacheEntry<T> | undefined;
    if (!entry) return null;

    const isExpired = Date.now() - entry.timestamp > this.config.cacheTtl;
    if (isExpired) {
      this.cache.delete(key);
      return null;
    }

    return entry.data;
  }

  /**
   * Set a value in cache
   */
  private setCache<T>(key: string, data: T): void {
    if (this.config.cacheTtl === 0) return;

    this.cache.set(key, {
      data,
      timestamp: Date.now(),
    });
  }

  /**
   * Clear the cache
   */
  clearCache(): void {
    this.cache.clear();
  }

  /**
   * Validate content namespace structure
   */
  private validateNamespace(namespace: ContentNamespace, data: unknown): string[] {
    const errors: string[] = [];

    if (!data || typeof data !== 'object') {
      errors.push(`Invalid content structure for namespace: ${namespace}`);
      return errors;
    }

    // Namespace-specific validation
    switch (namespace) {
      case 'landing':
        errors.push(...this.validateLandingContent(data as LandingContent));
        break;
      case 'dashboard':
        errors.push(...this.validateDashboardContent(data as DashboardContent));
        break;
      case 'editor':
        errors.push(...this.validateEditorContent(data as EditorContent));
        break;
      case 'common':
        errors.push(...this.validateCommonContent(data as CommonContent));
        break;
    }

    return errors;
  }

  private validateLandingContent(data: LandingContent): string[] {
    const errors: string[] = [];
    const requiredKeys = ['hero', 'invitation', 'journey', 'choicePoint', 'stats'];

    for (const key of requiredKeys) {
      if (!(key in data)) {
        errors.push(`Missing required key: landing.${key}`);
      }
    }

    return errors;
  }

  private validateDashboardContent(data: DashboardContent): string[] {
    const errors: string[] = [];
    const requiredKeys = ['page', 'sidebar', 'navigation', 'mobile', 'views'];

    for (const key of requiredKeys) {
      if (!(key in data)) {
        errors.push(`Missing required key: dashboard.${key}`);
      }
    }

    return errors;
  }

  private validateEditorContent(data: EditorContent): string[] {
    const errors: string[] = [];
    const requiredKeys = ['chat', 'templateLibrary', 'message', 'promptEnhancer'];

    for (const key of requiredKeys) {
      if (!(key in data)) {
        errors.push(`Missing required key: editor.${key}`);
      }
    }

    return errors;
  }

  private validateCommonContent(data: CommonContent): string[] {
    const errors: string[] = [];
    const requiredKeys = ['buttons', 'labels', 'status', 'errors', 'validation'];

    for (const key of requiredKeys) {
      if (!(key in data)) {
        errors.push(`Missing required key: common.${key}`);
      }
    }

    return errors;
  }

  /**
   * Validate asset registry structure
   */
  private validateAssets(data: AssetRegistry): string[] {
    const errors: string[] = [];

    if (!data.icons) {
      errors.push('Missing required key: icons');
    }
    if (!data.images) {
      errors.push('Missing required key: images');
    }
    if (!data.animations) {
      errors.push('Missing required key: animations');
    }
    if (!data.gradients) {
      errors.push('Missing required key: gradients');
    }

    return errors;
  }
}

// ============================================================================
// STATIC CONTENT RESOLVER
// ============================================================================

/**
 * Resolve a content value at a given path
 */
export function resolveContentPath<T>(
  content: Record<string, unknown>,
  path: string
): T | undefined {
  const parts = path.split('.');
  let current: unknown = content;

  for (const part of parts) {
    if (current === null || current === undefined) {
      return undefined;
    }
    if (typeof current !== 'object') {
      return undefined;
    }
    current = (current as Record<string, unknown>)[part];
  }

  return current as T;
}

/**
 * Find an icon by ID in the registry
 */
export function findIcon(registry: AssetRegistry, id: IconId): IconDefinition | undefined {
  const categories = Object.keys(registry.icons) as IconCategory[];

  for (const category of categories) {
    const icon = registry.icons[category].find((i) => i.id === id);
    if (icon) return icon;
  }

  return undefined;
}

/**
 * Find an image by ID in the registry
 */
export function findImage(registry: AssetRegistry, id: AssetId): ImageDefinition | undefined {
  const brandImage = registry.images.brand.find((i) => i.id === id);
  if (brandImage) return brandImage;

  return registry.images.decorative.find((i) => i.id === id);
}

/**
 * Get all icons in a category
 */
export function getIconsByCategory(
  registry: AssetRegistry,
  category: IconCategory
): readonly IconDefinition[] {
  return registry.icons[category] ?? [];
}

// ============================================================================
// INTERPOLATION
// ============================================================================

/**
 * Interpolate variables in a content string
 * Supports {variable} syntax
 */
export function interpolate(template: string, variables: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (match, key) => {
    return key in variables ? String(variables[key]) : match;
  });
}

// ============================================================================
// SINGLETON INSTANCE
// ============================================================================

let loaderInstance: ContentLoader | null = null;

/**
 * Get the singleton content loader instance
 */
export function getContentLoader(config?: Partial<LoaderConfig>): ContentLoader {
  if (!loaderInstance || config) {
    loaderInstance = new ContentLoader(config);
  }
  return loaderInstance;
}

/**
 * Reset the content loader instance (useful for testing)
 */
export function resetContentLoader(): void {
  loaderInstance = null;
}
