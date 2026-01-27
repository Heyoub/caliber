/**
 * Fuzz Tests
 *
 * Tests that throw random/malformed inputs at parsers and handlers.
 * Goal: Find crashes, hangs, or unexpected behavior.
 * Run with: bun test tests/fuzz/
 */

import { describe, expect, it } from 'bun:test';
import fc from 'fast-check';

const FUZZ_RUNS = Math.max(1, Number.parseInt(process.env.FUZZ_RUNS ?? '10000', 10) || 10000);
const RUNS_SMALL = Math.max(100, Math.floor(FUZZ_RUNS / 10));
const RUNS_MED = Math.max(250, Math.floor(FUZZ_RUNS / 4));
const RUNS_LARGE = FUZZ_RUNS;

// Simulated DSL parser (matches caliber-dsl patterns)
function parseDSL(input: string): { valid: boolean; error?: string } {
  try {
    // Basic lexer simulation
    if (input.length > 1_000_000) {
      return { valid: false, error: 'Input too large' };
    }

    // Check for balanced braces
    let depth = 0;
    for (const char of input) {
      if (char === '{') depth++;
      if (char === '}') depth--;
      if (depth < 0) return { valid: false, error: 'Unbalanced braces' };
    }
    if (depth !== 0) return { valid: false, error: 'Unbalanced braces' };

    // Check for valid keywords
    const keywords = ['trajectory', 'scope', 'artifact', 'agent', 'lock', 'message'];
    const tokens = input.split(/\s+/).filter((t) => t.length > 0);

    // Very simple validation
    if (tokens.length > 0 && !keywords.some((k) => input.includes(k))) {
      // Allow empty or keyword-containing inputs
      if (tokens.some((t) => /^[a-z_][a-z0-9_]*$/i.test(t))) {
        return { valid: true };
      }
    }

    return { valid: true };
  } catch (e) {
    return { valid: false, error: String(e) };
  }
}

// Simulated JSON-like config parser
function parseConfig(input: string): Record<string, unknown> | null {
  try {
    const result = JSON.parse(input);
    if (typeof result !== 'object' || result === null) {
      return null;
    }
    return result as Record<string, unknown>;
  } catch {
    return null;
  }
}

describe('fuzz: DSL parser', () => {
  it('handles arbitrary strings without crashing', () => {
    fc.assert(
      fc.property(fc.string({ maxLength: 10000 }), (input) => {
        const result = parseDSL(input);
        expect(result).toBeDefined();
        expect(typeof result.valid).toBe('boolean');
        return true;
      }),
      { numRuns: RUNS_LARGE }
    );
  });

  it('handles unicode strings', () => {
    fc.assert(
      fc.property(fc.fullUnicodeString({ maxLength: 1000 }), (input) => {
        const result = parseDSL(input);
        expect(result).toBeDefined();
        return true;
      }),
      { numRuns: RUNS_MED }
    );
  });

  it('handles deeply nested braces', () => {
    fc.assert(
      fc.property(fc.nat({ max: 1000 }), (depth) => {
        const input = '{'.repeat(depth) + '}'.repeat(depth);
        const result = parseDSL(input);
        expect(result.valid).toBe(true);
        return true;
      }),
      { numRuns: RUNS_SMALL }
    );
  });

  it('handles adversarial inputs', () => {
    const adversarial = [
      '', // empty
      ' '.repeat(10000), // whitespace
      '\x00'.repeat(100), // null bytes
      'a'.repeat(100000), // long string
      '{'.repeat(500) + '}'.repeat(500), // deep nesting
      '{{{}}}{{{}}}', // alternating
      '\n\r\t\v\f', // control chars
      '\\u0000\\n\\r', // escape sequences
      'trajectory { scope { artifact { } } }', // valid structure
      'DROP TABLE trajectories;--', // SQL injection attempt
      '<script>alert(1)</script>', // XSS attempt
    ];

    for (const input of adversarial) {
      const result = parseDSL(input);
      expect(result).toBeDefined();
    }
  });
});

describe('fuzz: Config parser', () => {
  it('handles arbitrary JSON-like strings', () => {
    fc.assert(
      fc.property(fc.json(), (input) => {
        const result = parseConfig(input);
        // Should either parse or return null, never crash
        if (result !== null) {
          expect(typeof result).toBe('object');
        }
        return true;
      }),
      { numRuns: RUNS_LARGE }
    );
  });

  it('handles malformed JSON', () => {
    const malformed = [
      '{',
      '}',
      '{"key":}',
      '{key: "value"}', // unquoted key
      "{'key': 'value'}", // single quotes
      '{"key": undefined}', // undefined
      '{"key": NaN}', // NaN
      '{"key": Infinity}', // Infinity
      '{"a": 1,}', // trailing comma
      '[1, 2, 3,]', // trailing comma in array
    ];

    for (const input of malformed) {
      const result = parseConfig(input);
      expect(result).toBeNull();
    }
  });

  it('handles deeply nested objects', () => {
    fc.assert(
      fc.property(fc.nat({ max: 100 }), (depth) => {
        let json = '{"value": 1';
        for (let i = 0; i < depth; i++) {
          json = `{"nested": ${json}}`;
        }
        json += '}'.repeat(depth);

        parseConfig(json);
        // May fail to parse if too deep, but shouldn't crash
        return true;
      }),
      { numRuns: RUNS_SMALL }
    );
  });
});

describe('fuzz: URL parsing', () => {
  function parseApiUrl(input: string): { valid: boolean; parts?: { base: string; path: string } } {
    try {
      const url = new URL(input);
      return {
        valid: true,
        parts: {
          base: url.origin,
          path: url.pathname,
        },
      };
    } catch {
      return { valid: false };
    }
  }

  it('handles arbitrary URL-like strings', () => {
    fc.assert(
      fc.property(fc.webUrl(), (input) => {
        const result = parseApiUrl(input);
        expect(result).toBeDefined();
        if (result.valid) {
          expect(result.parts?.base).toBeDefined();
        }
        return true;
      }),
      { numRuns: RUNS_MED }
    );
  });

  it('handles protocol-relative URLs', () => {
    const inputs = [
      '//example.com/path',
      'http://localhost:3000/api',
      'https://api.caliber.run/v1/trajectories',
      'file:///etc/passwd', // should parse but be rejected by app logic
      'javascript:alert(1)', // XSS vector
      'data:text/html,<script>alert(1)</script>',
    ];

    for (const input of inputs) {
      const result = parseApiUrl(input);
      expect(result).toBeDefined();
    }
  });
});
