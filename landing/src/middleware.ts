/**
 * Astro Middleware for Server-Side Auth Guards
 * Protects /dashboard/* routes by verifying JWT token
 */
import { defineMiddleware } from 'astro:middleware';

// Routes that require authentication
const PROTECTED_ROUTES = ['/dashboard'];

// Routes to exclude from protection (e.g., public API health checks)
const PUBLIC_ROUTES = ['/auth/callback', '/login'];

export const onRequest = defineMiddleware(async (context, next) => {
  const { pathname } = context.url;

  // Skip middleware for public routes
  if (PUBLIC_ROUTES.some(route => pathname.startsWith(route))) {
    return next();
  }

  // Check if route requires authentication
  const isProtected = PROTECTED_ROUTES.some(route => pathname.startsWith(route));

  if (!isProtected) {
    return next();
  }

  // Get token from cookie or Authorization header
  const token = context.cookies.get('caliber_token')?.value
    || context.request.headers.get('Authorization')?.replace('Bearer ', '');

  if (!token) {
    // Redirect to login with return URL
    const returnUrl = encodeURIComponent(pathname);
    return context.redirect(`/login?return_url=${returnUrl}`);
  }

  // Validate JWT token (basic expiry check)
  try {
    const payload = JSON.parse(atob(token.split('.')[1]));
    const now = Math.floor(Date.now() / 1000);

    if (payload.exp <= now) {
      // Token expired - redirect to login
      context.cookies.delete('caliber_token');
      const returnUrl = encodeURIComponent(pathname);
      return context.redirect(`/login?return_url=${returnUrl}`);
    }

    // Attach user info to locals for use in pages
    context.locals.user = {
      id: payload.user_id,
      email: payload.email,
      firstName: payload.first_name,
      lastName: payload.last_name,
      tenantId: payload.organization_id,
    };
    context.locals.token = token;
  } catch {
    // Invalid token format - redirect to login
    context.cookies.delete('caliber_token');
    const returnUrl = encodeURIComponent(pathname);
    return context.redirect(`/login?return_url=${returnUrl}`);
  }

  return next();
});
