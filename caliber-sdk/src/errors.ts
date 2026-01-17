/**
 * CALIBER SDK Error Types
 */

export interface ApiErrorDetails {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

/**
 * Base error class for all CALIBER SDK errors
 */
export class CaliberError extends Error {
  public readonly code: string;
  public readonly statusCode?: number;
  public readonly details?: Record<string, unknown>;

  constructor(message: string, code: string, statusCode?: number, details?: Record<string, unknown>) {
    super(message);
    this.name = 'CaliberError';
    this.code = code;
    this.statusCode = statusCode;
    this.details = details;
    Object.setPrototypeOf(this, CaliberError.prototype);
  }
}

/**
 * Error thrown when a resource is not found
 */
export class NotFoundError extends CaliberError {
  constructor(resource: string, id: string) {
    super(`${resource} not found: ${id}`, 'NOT_FOUND', 404);
    this.name = 'NotFoundError';
    Object.setPrototypeOf(this, NotFoundError.prototype);
  }
}

/**
 * Error thrown when authentication fails
 */
export class AuthenticationError extends CaliberError {
  constructor(message = 'Authentication failed') {
    super(message, 'UNAUTHORIZED', 401);
    this.name = 'AuthenticationError';
    Object.setPrototypeOf(this, AuthenticationError.prototype);
  }
}

/**
 * Error thrown when authorization fails
 */
export class AuthorizationError extends CaliberError {
  constructor(message = 'Permission denied') {
    super(message, 'FORBIDDEN', 403);
    this.name = 'AuthorizationError';
    Object.setPrototypeOf(this, AuthorizationError.prototype);
  }
}

/**
 * Error thrown when validation fails
 */
export class ValidationError extends CaliberError {
  public readonly validationErrors: string[];

  constructor(message: string, errors: string[] = []) {
    super(message, 'VALIDATION_ERROR', 400);
    this.name = 'ValidationError';
    this.validationErrors = errors;
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

/**
 * Error thrown when there's a conflict (e.g., lock contention)
 */
export class ConflictError extends CaliberError {
  constructor(message: string) {
    super(message, 'CONFLICT', 409);
    this.name = 'ConflictError';
    Object.setPrototypeOf(this, ConflictError.prototype);
  }
}

/**
 * Error thrown when rate limited
 */
export class RateLimitError extends CaliberError {
  public readonly retryAfter?: number;

  constructor(message = 'Rate limit exceeded', retryAfter?: number) {
    super(message, 'RATE_LIMITED', 429);
    this.name = 'RateLimitError';
    this.retryAfter = retryAfter;
    Object.setPrototypeOf(this, RateLimitError.prototype);
  }
}

/**
 * Convert API error response to appropriate error class
 */
export function parseApiError(statusCode: number, body: unknown): CaliberError {
  const error = body as ApiErrorDetails | undefined;
  const message = error?.message ?? 'Unknown error';
  const code = error?.code ?? 'UNKNOWN';
  const details = error?.details;

  switch (statusCode) {
    case 400:
      return new ValidationError(message, details?.errors as string[] | undefined);
    case 401:
      return new AuthenticationError(message);
    case 403:
      return new AuthorizationError(message);
    case 404:
      return new NotFoundError('Resource', 'unknown');
    case 409:
      return new ConflictError(message);
    case 429:
      return new RateLimitError(message);
    default:
      return new CaliberError(message, code, statusCode, details);
  }
}
