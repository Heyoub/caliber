/**
 * Root Layout Load Function
 * Handles authentication state and redirects
 */
import { redirect } from '@sveltejs/kit';
import { isAuthenticated, getStoredAuth } from '$stores/auth';
import type { LayoutLoad } from './$types';

export const ssr = false; // Client-side only for auth

export const load: LayoutLoad = async ({ url }) => {
  const publicPaths = ['/login', '/auth/callback'];
  const isPublicPath = publicPaths.some((path) => url.pathname.startsWith(path));

  // Check auth state
  const authenticated = isAuthenticated();
  const { user } = getStoredAuth();

  // Redirect unauthenticated users to login (except public paths)
  if (!authenticated && !isPublicPath) {
    const returnUrl = encodeURIComponent(url.pathname + url.search);
    throw redirect(302, `/login?return_url=${returnUrl}`);
  }

  // Redirect authenticated users away from login
  if (authenticated && isPublicPath) {
    throw redirect(302, '/dashboard');
  }

  return {
    isAuthenticated: authenticated,
    user,
    isLoading: false,
  };
};
