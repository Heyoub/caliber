/**
 * HATEOAS Link Utilities
 * Helpers for working with API links
 */

import type { Link, Links, LinkWithRel } from './types';

const API_URL = import.meta.env.PUBLIC_API_URL || 'https://api.caliber.run';

/**
 * Get the auth token from storage.
 */
function getToken(): string | null {
  if (typeof localStorage === 'undefined') return null;
  return localStorage.getItem('caliber_token');
}

/**
 * Extract available actions from links (excluding 'self').
 */
export function getActions(links: Links | undefined, exclude: string[] = ['self']): LinkWithRel[] {
  if (!links) return [];
  return Object.entries(links)
    .filter(([rel]) => !exclude.includes(rel))
    .map(([rel, link]) => ({ rel, ...link }));
}

/**
 * Check if a link represents a specific HTTP method.
 */
export function isMethod(link: Link, method: string): boolean {
  const linkMethod = link.method?.toUpperCase() || 'GET';
  return linkMethod === method.toUpperCase();
}

/**
 * Check if a link is "safe" (GET or HEAD).
 */
export function isSafe(link: Link): boolean {
  const method = link.method?.toUpperCase() || 'GET';
  return method === 'GET' || method === 'HEAD';
}

/**
 * Check if a link is destructive (DELETE).
 */
export function isDestructive(link: Link): boolean {
  return link.method?.toUpperCase() === 'DELETE';
}

/**
 * Resolve a link href to an absolute URL.
 */
export function resolveHref(href: string): string {
  if (href.startsWith('http://') || href.startsWith('https://')) {
    return href;
  }
  return `${API_URL}${href.startsWith('/') ? '' : '/'}${href}`;
}

/**
 * Execute a link action.
 * @param link The link to execute
 * @param body Optional request body for POST/PUT/PATCH
 * @returns The response data
 */
export async function executeLink<T = unknown>(
  link: Link | LinkWithRel,
  body?: unknown
): Promise<T> {
  const token = getToken();
  const method = link.method?.toUpperCase() || 'GET';
  const url = resolveHref(link.href);

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  const options: RequestInit = {
    method,
    headers,
  };

  if (body && method !== 'GET' && method !== 'HEAD') {
    options.body = JSON.stringify(body);
  }

  const response = await fetch(url, options);

  if (!response.ok) {
    if (response.status === 401) {
      localStorage.removeItem('caliber_token');
      localStorage.removeItem('caliber_user');
      throw new Error('Session expired. Please log in again.');
    }

    const errorData = await response.json().catch(() => null);
    const message = errorData?.message || `Request failed: ${response.status}`;
    throw new Error(message);
  }

  // Handle no-content responses
  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

/**
 * Follow a link (navigation action).
 * Returns the URL to navigate to, or null if not applicable.
 */
export function getNavigationUrl(link: Link | LinkWithRel): string | null {
  if (!isSafe(link)) {
    return null;
  }

  // Check if this is an API link or a page link
  if (link.href.startsWith('/api/')) {
    return null;
  }

  return link.href;
}

/**
 * Format a relation name for display.
 */
export function formatRelation(rel: string): string {
  return rel
    .replace(/_/g, ' ')
    .replace(/-/g, ' ')
    .replace(/\b\w/g, (l) => l.toUpperCase());
}

/**
 * Get display text for a link.
 */
export function getLinkText(link: Link | LinkWithRel): string {
  if (link.title) {
    return link.title;
  }
  if ('rel' in link) {
    return formatRelation(link.rel);
  }
  return link.href;
}

/**
 * Group links by category for organized display.
 */
export function categorizeLinks(links: Links | undefined): {
  navigation: LinkWithRel[];
  actions: LinkWithRel[];
  dangerous: LinkWithRel[];
} {
  const all = getActions(links);

  return {
    navigation: all.filter((l) => isSafe(l)),
    actions: all.filter((l) => !isSafe(l) && !isDestructive(l)),
    dangerous: all.filter((l) => isDestructive(l)),
  };
}
