import { AppError, errorMessages } from './error';
import { isValidRoute } from '../config/routes';

export async function navigate(path: string) {
  try {
    // Skip navigation if already on the path
    if (window.location.pathname === path.split('#')[0]) {
      return;
    }

    // Validate route
    if (!isValidRoute(path)) {
      throw new AppError(errorMessages.NAVIGATION_ERROR, 'INVALID_ROUTE');
    }

    // Handle hash navigation
    if (path.includes('#')) {
      const [routePath, hash] = path.split('#');
      setTimeout(() => {
        document.getElementById(hash)?.scrollIntoView({ behavior: 'smooth' });
      }, 100);
    }

    // Update URL
    window.history.pushState({}, '', path);

    // Dispatch Astro navigation event
    window.dispatchEvent(new CustomEvent('astro:page-load'));
  } catch (error) {
    console.warn('Navigation error:', error);
    throw error;
  }
}

// Handle back/forward navigation
window.addEventListener('popstate', () => {
  window.dispatchEvent(new CustomEvent('astro:page-load'));
});
