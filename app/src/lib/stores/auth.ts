/**
 * Auth Store
 * Manages authentication state using Svelte stores
 * Reuses patterns from landing/src/stores/auth.ts
 */
import { writable, derived } from 'svelte/store';

// Types
export interface User {
  id: string;
  email: string;
  firstName?: string;
  lastName?: string;
  tenantId?: string;
}

export interface AuthState {
  isAuthenticated: boolean;
  isLoading: boolean;
  user: User | null;
  token: string | null;
  error: string | null;
}

// Storage keys
const TOKEN_KEY = 'caliber_token';
const USER_KEY = 'caliber_user';

// Check if we're in browser environment
const isBrowser = typeof window !== 'undefined';

/**
 * Get stored auth data from localStorage
 */
export function getStoredAuth(): { token: string | null; user: User | null } {
  if (!isBrowser) {
    return { token: null, user: null };
  }

  const token = localStorage.getItem(TOKEN_KEY);
  const userJson = localStorage.getItem(USER_KEY);
  const user = userJson ? JSON.parse(userJson) : null;

  return { token, user };
}

/**
 * Store auth data in localStorage
 */
export function storeAuth(token: string, user: User): void {
  if (!isBrowser) return;

  localStorage.setItem(TOKEN_KEY, token);
  localStorage.setItem(USER_KEY, JSON.stringify(user));
}

/**
 * Clear auth data from localStorage and cookie
 */
export function clearAuth(): void {
  if (!isBrowser) return;

  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
  // Also clear the cookie used for server-side auth
  document.cookie = 'caliber_token=; path=/; max-age=0; SameSite=Lax; Secure';
}

/**
 * Get the current auth token
 */
export function getToken(): string | null {
  if (!isBrowser) return null;
  return localStorage.getItem(TOKEN_KEY);
}

/**
 * Check if user is authenticated
 */
export function isAuthenticated(): boolean {
  const token = getToken();
  if (!token) return false;

  // Check if token is expired (JWT decode)
  try {
    const payload = JSON.parse(atob(token.split('.')[1]));
    const now = Math.floor(Date.now() / 1000);
    return payload.exp > now;
  } catch {
    return false;
  }
}

/**
 * Parse JWT token to extract user info
 */
export function parseToken(token: string): User | null {
  try {
    const payload = JSON.parse(atob(token.split('.')[1]));
    return {
      id: payload.user_id,
      email: payload.email,
      firstName: payload.first_name,
      lastName: payload.last_name,
      tenantId: payload.organization_id,
    };
  } catch {
    return null;
  }
}

/**
 * Initialize auth from callback URL params
 */
export function handleAuthCallback(searchParams: URLSearchParams): {
  success: boolean;
  error?: string;
} {
  const error = searchParams.get('error');
  if (error) {
    return { success: false, error: searchParams.get('error_description') || error };
  }

  const token = searchParams.get('token');
  if (!token) {
    return { success: false, error: 'No token received' };
  }

  const user = parseToken(token);
  if (!user) {
    return { success: false, error: 'Invalid token' };
  }

  storeAuth(token, user);
  return { success: true };
}

/**
 * Get user display name
 */
export function getUserDisplayName(user: User | null): string {
  if (!user) return '';
  if (user.firstName && user.lastName) {
    return `${user.firstName} ${user.lastName}`;
  }
  if (user.firstName) return user.firstName;
  return user.email;
}

/**
 * Get user initials for avatar
 */
export function getUserInitials(user: User | null): string {
  if (!user) return '?';
  if (user.firstName && user.lastName) {
    return `${user.firstName[0]}${user.lastName[0]}`.toUpperCase();
  }
  if (user.firstName) return user.firstName[0].toUpperCase();
  return user.email[0].toUpperCase();
}

// Create the auth store
function createAuthStore() {
  const initialState: AuthState = {
    isAuthenticated: false,
    isLoading: true,
    user: null,
    token: null,
    error: null,
  };

  const { subscribe, set, update } = writable<AuthState>(initialState);

  return {
    subscribe,

    /**
     * Initialize auth state from localStorage
     */
    initialize: () => {
      const { token, user } = getStoredAuth();
      const authenticated = isAuthenticated();

      set({
        isAuthenticated: authenticated,
        isLoading: false,
        user: authenticated ? user : null,
        token: authenticated ? token : null,
        error: null,
      });
    },

    /**
     * Set auth state after successful login
     */
    setAuth: (token: string, user: User) => {
      storeAuth(token, user);
      set({
        isAuthenticated: true,
        isLoading: false,
        user,
        token,
        error: null,
      });
    },

    /**
     * Set loading state
     */
    setLoading: (loading: boolean) => {
      update((state) => ({ ...state, isLoading: loading }));
    },

    /**
     * Set error state
     */
    setError: (error: string) => {
      update((state) => ({ ...state, error, isLoading: false }));
    },

    /**
     * Clear error
     */
    clearError: () => {
      update((state) => ({ ...state, error: null }));
    },

    /**
     * Logout - clear auth and redirect
     */
    logout: () => {
      clearAuth();
      set({
        isAuthenticated: false,
        isLoading: false,
        user: null,
        token: null,
        error: null,
      });

      if (isBrowser) {
        window.location.href = '/';
      }
    },

    /**
     * Redirect to login page
     */
    redirectToLogin: (returnUrl?: string) => {
      if (!isBrowser) return;

      const loginUrl = returnUrl
        ? `/login?return_url=${encodeURIComponent(returnUrl)}`
        : '/login';
      window.location.href = loginUrl;
    },
  };
}

export const authStore = createAuthStore();

// Derived stores for convenience
export const user = derived(authStore, ($auth) => $auth.user);
export const token = derived(authStore, ($auth) => $auth.token);
export const isLoading = derived(authStore, ($auth) => $auth.isLoading);
export const authError = derived(authStore, ($auth) => $auth.error);
