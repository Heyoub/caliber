/**
 * Common Types
 *
 * Shared types used across the SDK including HATEOAS link support.
 */

// =============================================================================
// HATEOAS Link Types
//
// These types enable hypermedia-driven navigation of API responses.
// When API responses include `_links`, clients can follow them without
// hardcoding URLs.
// =============================================================================

/**
 * HATEOAS link representing an available action or related resource.
 *
 * @example
 * ```typescript
 * const link: Link = {
 *   href: '/api/v1/trajectories/123',
 *   method: 'GET',
 *   title: 'Get trajectory details',
 * };
 * ```
 */
export interface Link {
  /** The URL to follow for this link */
  href: string;
  /** HTTP method to use (default: GET) */
  method?: string;
  /** Human-readable description of the link */
  title?: string;
}

/**
 * Collection of HATEOAS links keyed by relation name.
 *
 * Common relation names:
 * - `self`: Link to the current resource
 * - `parent`: Link to the parent resource
 * - `children`: Link to child resources
 * - `next`: Link to the next page (pagination)
 * - `prev`: Link to the previous page (pagination)
 *
 * @example
 * ```typescript
 * const links: Links = {
 *   self: { href: '/api/v1/trajectories/123' },
 *   parent: { href: '/api/v1/trajectories/100' },
 *   scopes: { href: '/api/v1/trajectories/123/scopes' },
 * };
 * ```
 */
export type Links = Record<string, Link>;

/**
 * Mixin interface for responses that include HATEOAS links.
 *
 * API responses may include `_links` to enable hypermedia-driven navigation.
 * Use the client's `follow()` and `discover()` methods to navigate these links.
 *
 * @example
 * ```typescript
 * interface TrajectoryResponse extends Linkable {
 *   trajectory_id: string;
 *   name: string;
 *   // ... other fields
 * }
 *
 * const trajectory = await client.trajectories.get('123');
 * if (trajectory._links?.scopes) {
 *   const scopes = await client.follow<Scope[]>(trajectory._links.scopes);
 * }
 * ```
 */
export interface Linkable {
  /** HATEOAS links for hypermedia navigation */
  _links?: Links;
}

/**
 * HTTP methods supported for HATEOAS link following.
 */
export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
