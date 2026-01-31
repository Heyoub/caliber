/**
 * Content CMS Types - CALIBER/ForgeStack
 * Type-safe content and asset types with branded types and template literals
 */

// ============================================================================
// BRANDED TYPES
// ============================================================================

/**
 * Brand type utility for creating nominal types
 */
declare const __brand: unique symbol;
type Brand<T, B> = T & { readonly [__brand]: B };

/**
 * Branded type for content keys - ensures type safety for content paths
 */
export type ContentKey = Brand<string, 'ContentKey'>;

/**
 * Branded type for asset IDs - ensures type safety for asset references
 */
export type AssetId = Brand<string, 'AssetId'>;

/**
 * Branded type for icon IDs from Lucide
 */
export type IconId = Brand<string, 'IconId'>;

/**
 * Branded type for image paths
 */
export type ImagePath = Brand<string, 'ImagePath'>;

// ============================================================================
// CONTENT PATH TYPES (Template Literal Types)
// ============================================================================

/**
 * Available content namespaces
 */
export type ContentNamespace = 'landing' | 'dashboard' | 'editor' | 'common';

/**
 * Landing page content paths
 */
export type LandingContentPath =
  | `hero.${string}`
  | `invitation.${string}`
  | `journey.${string}`
  | `choicePoint.${string}`
  | `stats.${string}`
  | `forgeStack.${string}`
  | `cta.${string}`;

/**
 * Dashboard content paths
 */
export type DashboardContentPath =
  | `page.${string}`
  | `sidebar.${string}`
  | `navigation.${string}`
  | `mobile.${string}`
  | `views.${string}`;

/**
 * Editor content paths
 */
export type EditorContentPath =
  | `chat.${string}`
  | `templateLibrary.${string}`
  | `message.${string}`
  | `promptEnhancer.${string}`;

/**
 * Common content paths
 */
export type CommonContentPath =
  | `buttons.${string}`
  | `labels.${string}`
  | `status.${string}`
  | `errors.${string}`
  | `validation.${string}`
  | `time.${string}`
  | `pagination.${string}`
  | `accessibility.${string}`
  | `adminPanel.${string}`
  | `branding.${string}`
  | `footer.${string}`;

/**
 * Full content path with namespace
 */
export type ContentPath<N extends ContentNamespace = ContentNamespace> =
  N extends 'landing' ? `landing.${LandingContentPath}` :
  N extends 'dashboard' ? `dashboard.${DashboardContentPath}` :
  N extends 'editor' ? `editor.${EditorContentPath}` :
  N extends 'common' ? `common.${CommonContentPath}` :
  never;

// ============================================================================
// ASSET TYPES
// ============================================================================

/**
 * Icon categories in the asset registry
 */
export type IconCategory =
  | 'navigation'
  | 'files'
  | 'actions'
  | 'features'
  | 'communication'
  | 'calendar'
  | 'utility';

/**
 * Icon definition from the registry
 */
export interface IconDefinition {
  readonly id: IconId;
  readonly lucide: string;
  readonly description: string;
}

/**
 * Image definition from the registry
 */
export interface ImageDefinition {
  readonly id: AssetId;
  readonly path: ImagePath;
  readonly alt: string;
  readonly description: string;
}

/**
 * Animation definition from the registry
 */
export interface AnimationDefinition {
  readonly id: AssetId;
  readonly keyframes: string;
  readonly duration: string;
  readonly timing: string;
  readonly iteration: string;
}

/**
 * Transition preset definition
 */
export interface TransitionDefinition {
  readonly id: AssetId;
  readonly duration: string;
  readonly timing: string;
}

/**
 * Gradient definition from the registry
 */
export interface GradientDefinition {
  readonly id: AssetId;
  readonly value: string;
  readonly description: string;
}

/**
 * Asset registry structure
 */
export interface AssetRegistry {
  readonly icons: {
    readonly [K in IconCategory]: readonly IconDefinition[];
  };
  readonly images: {
    readonly brand: readonly ImageDefinition[];
    readonly decorative: readonly ImageDefinition[];
  };
  readonly animations: {
    readonly css: readonly AnimationDefinition[];
    readonly transitions: readonly TransitionDefinition[];
  };
  readonly gradients: {
    readonly brand: readonly GradientDefinition[];
    readonly ui: readonly GradientDefinition[];
  };
}

// ============================================================================
// CONTENT STRUCTURE TYPES
// ============================================================================

/**
 * Landing page content structure
 */
export interface LandingContent {
  readonly hero: {
    readonly badge: string;
    readonly title: {
      readonly prefix: string;
      readonly highlight: string;
    };
    readonly scrollAlt: string;
  };
  readonly invitation: {
    readonly title: {
      readonly line1: { text: string; highlight: string };
      readonly line2: { text: string; highlight: string };
      readonly line3: { prefix: string; suffix: string };
    };
    readonly form: {
      readonly emailLabel: string;
      readonly emailPlaceholder: string;
      readonly companySizeLabel: string;
      readonly companySizePlaceholder: string;
      readonly companySizeOptions: readonly { value: string; label: string }[];
      readonly submitButton: string;
      readonly disclaimer: string;
    };
    readonly testimonial: {
      readonly title: string;
      readonly quote: string;
      readonly author: string;
    };
    readonly links: {
      readonly demo: { title: string; subtitle: string };
      readonly contact: { title: string; subtitle: string };
    };
  };
  readonly journey: {
    readonly badge: string;
    readonly title: { prefix: string; highlight: string };
    readonly subtitle: string;
    readonly timeline: readonly {
      time: string;
      title: string;
      description: string;
      status: string;
      mockup: Record<string, unknown>;
    }[];
  };
  readonly choicePoint: {
    readonly badge: string;
    readonly title: { prefix: string; highlight: string; suffix: string };
    readonly subtitle: string;
    readonly placeholder: string;
    readonly paths: {
      readonly focus: PathDefinition;
      readonly ownership: PathDefinition;
    };
  };
  readonly stats: {
    readonly cognitiveLoad: { text: string };
    readonly switchCost: { text: string };
  };
  readonly forgeStack: {
    readonly hero: { title: string; subtitle: string };
  };
  readonly cta: {
    readonly title: string;
    readonly subtitle: string;
  };
}

interface PathDefinition {
  readonly title: string;
  readonly description: string;
  readonly features: readonly string[];
  readonly button: string;
}

/**
 * Dashboard content structure
 */
export interface DashboardContent {
  readonly page: {
    readonly title: string;
    readonly welcome: string;
  };
  readonly sidebar: {
    readonly brand: string;
    readonly sections: {
      readonly chats: { title: string; newChat: string };
      readonly cases: { title: string };
      readonly projects: { title: string };
      readonly knowledgeBase: { title: string };
    };
    readonly footer: {
      readonly settings: string;
      readonly logout: string;
      readonly greeting: string;
      readonly links: { support: string; terms: string };
    };
  };
  readonly navigation: {
    readonly menu: {
      readonly title: string;
      readonly items: readonly { text: string; href: string }[];
    };
    readonly logout: { title: string; text: string };
  };
  readonly mobile: {
    readonly menuAriaLabel: string;
    readonly links: {
      readonly dashboard: string;
      readonly forgeStack: string;
      readonly emiChat: string;
      readonly support: string;
      readonly account: string;
    };
  };
  readonly views: {
    readonly chat: string;
    readonly prompts: string;
    readonly templates: string;
    readonly history: string;
    readonly documents: string;
  };
}

/**
 * Editor content structure
 */
export interface EditorContent {
  readonly chat: {
    readonly header: {
      readonly title: string;
      readonly guestMode: string;
      readonly guestSubtitle: string;
      readonly defaultSubtitle: string;
      readonly speedDial: string;
      readonly sendingBadge: string;
    };
    readonly input: {
      readonly placeholder: string;
      readonly guestMode: string;
      readonly signupPrompt: string;
      readonly signupSuffix: string;
    };
    readonly modes: {
      readonly [key: string]: { title: string; label: string };
    };
    readonly heuristics: {
      readonly intent: HeuristicConfig;
      readonly scope: HeuristicConfig;
      readonly urgency: HeuristicConfig;
    };
    readonly actions: {
      readonly [key: string]: string;
    };
    readonly dragDrop: { dropText: string };
    readonly processing: { message: string };
    readonly views: {
      readonly [key: string]: { title: string; placeholder?: string };
    };
  };
  readonly templateLibrary: {
    readonly title: string;
    readonly restoreDefaults: string;
    readonly loading: string;
    readonly error: string;
    readonly addToQuick: string;
    readonly removeFromQuick: string;
  };
  readonly message: {
    readonly labels: { user: string; assistant: string };
    readonly typing: string;
  };
  readonly promptEnhancer: {
    readonly suggestions: {
      readonly [key: string]: string;
    };
  };
}

interface HeuristicConfig {
  readonly label: string;
  readonly options: readonly { value: string; label: string }[];
}

/**
 * Common content structure
 */
export interface CommonContent {
  readonly buttons: { readonly [key: string]: string };
  readonly labels: { readonly [key: string]: string };
  readonly status: { readonly [key: string]: string };
  readonly errors: { readonly [key: string]: string };
  readonly validation: { readonly [key: string]: string };
  readonly time: { readonly [key: string]: string };
  readonly pagination: { readonly [key: string]: string };
  readonly accessibility: { readonly [key: string]: string };
  readonly adminPanel: {
    readonly title: string;
    readonly tabs: { readonly [key: string]: string };
  };
  readonly branding: {
    readonly companyName: string;
    readonly productName: string;
    readonly tagline: string;
    readonly copyright: string;
  };
  readonly footer: {
    readonly links: { readonly [key: string]: string };
    readonly copyright: string;
  };
}

/**
 * Complete content store type
 */
export interface ContentStore {
  readonly landing: LandingContent;
  readonly dashboard: DashboardContent;
  readonly editor: EditorContent;
  readonly common: CommonContent;
}

// ============================================================================
// UTILITY TYPES
// ============================================================================

/**
 * Extract the type at a given path in an object
 */
export type PathValue<T, P extends string> =
  P extends `${infer K}.${infer Rest}`
    ? K extends keyof T
      ? PathValue<T[K], Rest>
      : never
    : P extends keyof T
      ? T[P]
      : never;

/**
 * Get content value type for a given path
 */
export type ContentValue<P extends string> =
  P extends `landing.${infer Rest}` ? PathValue<LandingContent, Rest> :
  P extends `dashboard.${infer Rest}` ? PathValue<DashboardContent, Rest> :
  P extends `editor.${infer Rest}` ? PathValue<EditorContent, Rest> :
  P extends `common.${infer Rest}` ? PathValue<CommonContent, Rest> :
  never;

/**
 * Type guard for content key validation
 */
export function isValidContentKey(key: string): key is ContentKey {
  const [namespace] = key.split('.');
  return ['landing', 'dashboard', 'editor', 'common'].includes(namespace);
}

/**
 * Type guard for asset ID validation
 */
export function isValidAssetId(id: string): id is AssetId {
  return typeof id === 'string' && id.length > 0;
}

/**
 * Create a branded content key
 */
export function contentKey<P extends string>(path: P): ContentKey {
  return path as unknown as ContentKey;
}

/**
 * Create a branded asset ID
 */
export function assetId(id: string): AssetId {
  return id as AssetId;
}

/**
 * Create a branded icon ID
 */
export function iconId(id: string): IconId {
  return id as IconId;
}

/**
 * Create a branded image path
 */
export function imagePath(path: string): ImagePath {
  return path as ImagePath;
}
