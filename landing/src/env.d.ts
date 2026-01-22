/// <reference path="../.astro/types.d.ts" />

/**
 * Type definitions for Astro locals (set by middleware)
 */
declare namespace App {
  interface Locals {
    user?: {
      id: string;
      email: string;
      firstName?: string;
      lastName?: string;
      tenantId?: string;
    };
    token?: string;
  }
}
