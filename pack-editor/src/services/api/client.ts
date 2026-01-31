/**
 * Base API client for caliber-api communication
 *
 * NOTE: Backend endpoints will be added later to avoid merge conflicts.
 * This client is set up for when those endpoints are ready.
 */

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000';

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
    public code?: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/**
 * Get the API key from storage
 */
function getApiKey(): string {
  if (typeof window !== 'undefined') {
    return localStorage.getItem('caliber_api_key') || '';
  }
  return '';
}

/**
 * Make an authenticated API request
 */
export async function apiRequest<T>(path: string, options: RequestInit = {}): Promise<T> {
  const url = `${API_BASE}${path}`;

  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'X-API-Key': getApiKey(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({
      message: response.statusText,
    }));
    throw new ApiError(response.status, error.message || 'Request failed', error.code);
  }

  return response.json();
}

/**
 * Make a multipart form request (for file uploads)
 */
export async function apiMultipart<T>(path: string, formData: FormData): Promise<T> {
  const url = `${API_BASE}${path}`;

  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'X-API-Key': getApiKey(),
      // Don't set Content-Type - browser will set it with boundary
    },
    body: formData,
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({
      message: response.statusText,
    }));
    throw new ApiError(response.status, error.message || 'Request failed', error.code);
  }

  return response.json();
}

export { API_BASE };
