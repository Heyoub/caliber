/**
 * ═══════════════════════════════════════════════════════════════════════════
 * CALIBER TEST SETUP
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * Global test setup for Vitest with Svelte 5 testing library support.
 *
 * Features:
 * - JSDOM environment configuration
 * - Custom matchers for component testing
 * - Mock utilities for stores and APIs
 * - CSS variables injection
 * - Accessibility testing helpers
 */

import { expect, vi, beforeAll, afterAll, afterEach } from 'vitest';
import { cleanup } from '@testing-library/svelte';
import '@testing-library/jest-dom';

// ═══════════════════════════════════════════════════════════════════════════
// JSDOM POLYFILLS
// ═══════════════════════════════════════════════════════════════════════════

// Mock ResizeObserver
class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
global.ResizeObserver = ResizeObserverMock;

// Mock IntersectionObserver
class IntersectionObserverMock {
  constructor(callback: IntersectionObserverCallback) {
    this.callback = callback;
  }
  callback: IntersectionObserverCallback;
  root = null;
  rootMargin = '';
  thresholds = [];
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  takeRecords = vi.fn(() => []);
}
global.IntersectionObserver = IntersectionObserverMock as unknown as typeof IntersectionObserver;

// Mock matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock scrollIntoView
Element.prototype.scrollIntoView = vi.fn();

// Mock getComputedStyle for CSS variable access
const originalGetComputedStyle = window.getComputedStyle;
window.getComputedStyle = (element: Element, pseudoElt?: string | null) => {
  const result = originalGetComputedStyle(element, pseudoElt);
  return new Proxy(result, {
    get(target, prop) {
      if (prop === 'getPropertyValue') {
        return (name: string) => {
          // Return sensible defaults for CSS variables
          if (name.startsWith('--')) {
            const cssVarDefaults: Record<string, string> = {
              '--teal-500': '175 70% 40%',
              '--coral-500': '15 85% 50%',
              '--mint-500': '165 70% 45%',
              '--lavender-500': '265 70% 55%',
              '--purple-500': '270 70% 60%',
              '--slate-800': '222 18% 20%',
              '--slate-900': '225 20% 10%',
            };
            return cssVarDefaults[name] || '';
          }
          return target.getPropertyValue(name);
        };
      }
      return Reflect.get(target, prop);
    },
  });
};

// ═══════════════════════════════════════════════════════════════════════════
// CSS VARIABLES INJECTION
// ═══════════════════════════════════════════════════════════════════════════

const CSS_VARIABLES = `
:root {
  /* Brand Colors */
  --teal-300: 175 60% 50%;
  --teal-400: 175 65% 45%;
  --teal-500: 175 70% 40%;
  --teal-600: 175 75% 35%;
  --teal-700: 175 80% 30%;

  --coral-300: 15 80% 60%;
  --coral-400: 15 82% 55%;
  --coral-500: 15 85% 50%;
  --coral-600: 15 87% 45%;
  --coral-700: 15 90% 40%;

  --mint-300: 165 60% 55%;
  --mint-400: 165 65% 50%;
  --mint-500: 165 70% 45%;
  --mint-600: 165 75% 40%;
  --mint-700: 165 80% 35%;

  --lavender-300: 265 60% 65%;
  --lavender-400: 265 65% 60%;
  --lavender-500: 265 70% 55%;
  --lavender-600: 265 75% 50%;
  --lavender-700: 265 80% 45%;

  --purple-300: 270 60% 70%;
  --purple-400: 270 65% 65%;
  --purple-500: 270 70% 60%;
  --purple-600: 270 75% 55%;
  --purple-700: 270 80% 50%;

  --pink-300: 330 65% 65%;
  --pink-400: 330 70% 60%;
  --pink-500: 330 75% 55%;
  --pink-600: 330 80% 50%;
  --pink-700: 330 85% 45%;

  --amber-300: 38 95% 55%;
  --amber-400: 38 92% 50%;
  --amber-500: 38 90% 45%;
  --amber-600: 38 87% 40%;
  --amber-700: 38 85% 35%;

  /* Surface Colors */
  --slate-100: 210 40% 96%;
  --slate-200: 214 32% 91%;
  --slate-300: 213 27% 84%;
  --slate-400: 215 20% 65%;
  --slate-500: 215 16% 47%;
  --slate-600: 215 19% 35%;
  --slate-700: 215 25% 27%;
  --slate-800: 222 18% 20%;
  --slate-900: 225 20% 10%;

  /* Fonts */
  --font-sans: Inter, system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
  --font-title: 'Cal Sans', Inter, sans-serif;
}
`;

// Inject CSS variables
function injectCSSVariables(): void {
  const style = document.createElement('style');
  style.textContent = CSS_VARIABLES;
  document.head.appendChild(style);
}

// ═══════════════════════════════════════════════════════════════════════════
// CUSTOM MATCHERS
// ═══════════════════════════════════════════════════════════════════════════

interface CustomMatchers<R = unknown> {
  toHaveClass(className: string): R;
  toHaveStyle(styles: Record<string, string>): R;
  toBeAccessible(): R;
  toHaveAriaLabel(label: string): R;
  toHaveFocusVisible(): R;
  toHaveGlowEffect(): R;
  toBeInteractive(): R;
}

declare module 'vitest' {
  interface Assertion<T = unknown> extends CustomMatchers<T> {}
  interface AsymmetricMatchersContaining extends CustomMatchers {}
}

expect.extend({
  toHaveClass(received: Element, className: string) {
    const pass = received.classList.contains(className);
    return {
      pass,
      message: () => pass
        ? `Expected element not to have class "${className}"`
        : `Expected element to have class "${className}", but it has: ${Array.from(received.classList).join(', ')}`,
    };
  },

  toHaveStyle(received: Element, styles: Record<string, string>) {
    const computedStyle = window.getComputedStyle(received);
    const mismatches: string[] = [];

    for (const [property, expectedValue] of Object.entries(styles)) {
      const actualValue = computedStyle.getPropertyValue(property);
      if (actualValue !== expectedValue) {
        mismatches.push(`${property}: expected "${expectedValue}", got "${actualValue}"`);
      }
    }

    return {
      pass: mismatches.length === 0,
      message: () => mismatches.length === 0
        ? 'Expected element not to have the specified styles'
        : `Style mismatches:\n${mismatches.join('\n')}`,
    };
  },

  toBeAccessible(received: Element) {
    const issues: string[] = [];

    // Check for role
    if (received.tagName !== 'BUTTON' && received.tagName !== 'A' &&
        received.tagName !== 'INPUT' && !received.getAttribute('role')) {
      issues.push('Missing role attribute');
    }

    // Check for aria-label on interactive elements without visible text
    if (received.getAttribute('role') === 'button' || received.tagName === 'BUTTON') {
      const hasText = received.textContent?.trim();
      const hasAriaLabel = received.getAttribute('aria-label');
      const hasAriaLabelledBy = received.getAttribute('aria-labelledby');
      if (!hasText && !hasAriaLabel && !hasAriaLabelledBy) {
        issues.push('Interactive element missing accessible name');
      }
    }

    return {
      pass: issues.length === 0,
      message: () => issues.length === 0
        ? 'Expected element not to be accessible'
        : `Accessibility issues:\n${issues.join('\n')}`,
    };
  },

  toHaveAriaLabel(received: Element, label: string) {
    const ariaLabel = received.getAttribute('aria-label');
    return {
      pass: ariaLabel === label,
      message: () => ariaLabel === label
        ? `Expected element not to have aria-label "${label}"`
        : `Expected aria-label "${label}", got "${ariaLabel}"`,
    };
  },

  toHaveFocusVisible(received: Element) {
    const isFocused = document.activeElement === received;
    return {
      pass: isFocused,
      message: () => isFocused
        ? 'Expected element not to be focused'
        : 'Expected element to be focused',
    };
  },

  toHaveGlowEffect(received: Element) {
    const classList = Array.from(received.classList);
    const hasGlow = classList.some(c => c.includes('glow') || c.includes('shadow'));
    return {
      pass: hasGlow,
      message: () => hasGlow
        ? 'Expected element not to have glow effect'
        : 'Expected element to have glow effect (glow-* or shadow-* class)',
    };
  },

  toBeInteractive(received: Element) {
    const isDisabled = received.getAttribute('disabled') !== null ||
                       received.getAttribute('aria-disabled') === 'true';
    return {
      pass: !isDisabled,
      message: () => !isDisabled
        ? 'Expected element to be disabled'
        : 'Expected element to be interactive (not disabled)',
    };
  },
});

// ═══════════════════════════════════════════════════════════════════════════
// MOCK UTILITIES
// ═══════════════════════════════════════════════════════════════════════════

/** Create a mock Svelte store */
export function mockStore<T>(initialValue: T) {
  let value = initialValue;
  const subscribers = new Set<(v: T) => void>();

  return {
    subscribe: vi.fn((callback: (v: T) => void) => {
      subscribers.add(callback);
      callback(value);
      return () => subscribers.delete(callback);
    }),
    set: vi.fn((newValue: T) => {
      value = newValue;
      subscribers.forEach(cb => cb(value));
    }),
    update: vi.fn((updater: (v: T) => T) => {
      value = updater(value);
      subscribers.forEach(cb => cb(value));
    }),
    get: () => value,
  };
}

/** Create a mock API client */
export function mockApiClient() {
  return {
    get: vi.fn().mockResolvedValue({ data: {} }),
    post: vi.fn().mockResolvedValue({ data: {} }),
    put: vi.fn().mockResolvedValue({ data: {} }),
    delete: vi.fn().mockResolvedValue({ data: {} }),
    connectStream: vi.fn().mockReturnValue(() => {}),
  };
}

/** Create a mock MCP client */
export function mockMCPClient() {
  return {
    listTools: vi.fn().mockResolvedValue([]),
    executeTool: vi.fn().mockResolvedValue({ result: {} }),
    listResources: vi.fn().mockResolvedValue([]),
    readResource: vi.fn().mockResolvedValue({ contents: '', mimeType: 'text/plain' }),
    listPrompts: vi.fn().mockResolvedValue([]),
    getPrompt: vi.fn().mockResolvedValue({ messages: [] }),
  };
}

/** Create a mock timer with controlled async behavior */
export function mockTimer() {
  vi.useFakeTimers();
  return {
    advance: (ms: number) => vi.advanceTimersByTime(ms),
    advanceToNextTimer: () => vi.advanceTimersToNextTimer(),
    runAllTimers: () => vi.runAllTimers(),
    restore: () => vi.useRealTimers(),
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/** Wait for component to update after state change */
export async function tick(): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, 0));
}

/** Simulate keyboard event */
export function simulateKeyboard(element: Element, key: string, options?: Partial<KeyboardEventInit>) {
  const event = new KeyboardEvent('keydown', {
    key,
    bubbles: true,
    cancelable: true,
    ...options,
  });
  element.dispatchEvent(event);
}

/** Simulate focus/blur cycle */
export async function simulateFocusCycle(element: HTMLElement) {
  element.focus();
  await tick();
  element.blur();
  await tick();
}

/** Get all focusable elements within a container */
export function getFocusableElements(container: Element): HTMLElement[] {
  const selectors = [
    'button:not([disabled])',
    'input:not([disabled])',
    'select:not([disabled])',
    'textarea:not([disabled])',
    'a[href]',
    '[tabindex]:not([tabindex="-1"])',
  ];
  return Array.from(container.querySelectorAll<HTMLElement>(selectors.join(',')));
}

/** Check if element is visible (not display: none, visibility: hidden, or zero size) */
export function isVisible(element: Element): boolean {
  const style = window.getComputedStyle(element);
  if (style.display === 'none' || style.visibility === 'hidden') return false;
  const rect = element.getBoundingClientRect();
  return rect.width > 0 && rect.height > 0;
}

// ═══════════════════════════════════════════════════════════════════════════
// LIFECYCLE HOOKS
// ═══════════════════════════════════════════════════════════════════════════

beforeAll(() => {
  injectCSSVariables();
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  // Clear any remaining timers
  vi.clearAllTimers();
});

afterAll(() => {
  vi.restoreAllMocks();
});

// Export for use in tests
export { vi, expect };
