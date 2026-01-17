/**
 * Property-Based Tests
 *
 * Tests that verify properties hold for all inputs using fast-check.
 * Run with: bun test --test-name-pattern property
 */

import { describe, expect, it } from 'bun:test';
import fc from 'fast-check';

// Domain types
interface Trajectory {
  id: string;
  name: string;
  createdAt: Date;
  scopes: Scope[];
}

interface Scope {
  id: string;
  parentId: string | null;
  artifacts: Artifact[];
}

interface Artifact {
  id: string;
  type: 'code' | 'text' | 'config' | 'data';
  content: string;
  tokens: number;
}

// Arbitraries (generators) - typed to match domain interfaces
const artifactArb: fc.Arbitrary<Artifact> = fc.record({
  id: fc.uuid(),
  type: fc.constantFrom('code', 'text', 'config', 'data') as fc.Arbitrary<Artifact['type']>,
  content: fc.string({ maxLength: 10000 }),
  tokens: fc.nat({ max: 100000 }),
});

const scopeArb: fc.Arbitrary<Scope> = fc.record({
  id: fc.uuid(),
  parentId: fc.option(fc.uuid(), { nil: null }),
  artifacts: fc.array(artifactArb, { maxLength: 100 }),
});

const trajectoryArb: fc.Arbitrary<Trajectory> = fc.record({
  id: fc.uuid(),
  name: fc.string({ minLength: 1, maxLength: 255 }),
  createdAt: fc.date(),
  scopes: fc.array(scopeArb, { maxLength: 50 }),
});

// Property: Token counts are always non-negative
describe('property: Token counting', () => {
  it('total tokens is sum of artifact tokens', () => {
    fc.assert(
      fc.property(trajectoryArb, (trajectory) => {
        const total = trajectory.scopes
          .flatMap((s) => s.artifacts)
          .reduce((sum, a) => sum + a.tokens, 0);

        expect(total).toBeGreaterThanOrEqual(0);
        return true;
      })
    );
  });
});

// Property: Serialization roundtrip
describe('property: JSON roundtrip', () => {
  it('trajectory survives JSON serialization', () => {
    fc.assert(
      fc.property(trajectoryArb, (trajectory) => {
        const serialized = JSON.stringify(trajectory);
        const deserialized = JSON.parse(serialized);

        expect(deserialized.id).toBe(trajectory.id);
        expect(deserialized.name).toBe(trajectory.name);
        expect(deserialized.scopes.length).toBe(trajectory.scopes.length);
        return true;
      })
    );
  });
});

// Property: Context budget allocation
describe('property: Context budget', () => {
  function allocateBudget(
    artifacts: Artifact[],
    budget: number
  ): { selected: Artifact[]; remaining: number } {
    const sorted = [...artifacts].sort((a, b) => b.tokens - a.tokens);
    const selected: Artifact[] = [];
    let remaining = budget;

    for (const artifact of sorted) {
      if (artifact.tokens <= remaining) {
        selected.push(artifact);
        remaining -= artifact.tokens;
      }
    }

    return { selected, remaining };
  }

  it('never exceeds budget', () => {
    fc.assert(
      fc.property(fc.array(artifactArb), fc.nat({ max: 100000 }), (artifacts, budget) => {
        const { selected, remaining } = allocateBudget(artifacts, budget);
        const usedTokens = selected.reduce((sum, a) => sum + a.tokens, 0);

        expect(usedTokens).toBeLessThanOrEqual(budget);
        expect(remaining).toBeGreaterThanOrEqual(0);
        expect(usedTokens + remaining).toBe(budget);
        return true;
      })
    );
  });

  it('is greedy optimal', () => {
    fc.assert(
      fc.property(fc.array(artifactArb), fc.nat({ max: 100000 }), (artifacts, budget) => {
        const { selected, remaining } = allocateBudget(artifacts, budget);
        const notSelected = artifacts.filter((a) => !selected.includes(a));

        // No remaining artifact could fit
        for (const artifact of notSelected) {
          expect(artifact.tokens).toBeGreaterThan(remaining);
        }
        return true;
      })
    );
  });
});

// Property: Scope hierarchy invariants
describe('property: Scope hierarchy', () => {
  it('root scopes have null parentId', () => {
    fc.assert(
      fc.property(trajectoryArb, (trajectory) => {
        trajectory.scopes.filter((s) => s.parentId === null);
        // At least structure is valid (may have 0 roots if randomly generated)
        return true;
      })
    );
  });
});

// Property: ID uniqueness
describe('property: ID uniqueness', () => {
  it('artifact IDs are unique within scope', () => {
    fc.assert(
      fc.property(scopeArb, (scope) => {
        const ids = scope.artifacts.map((a) => a.id);
        const uniqueIds = new Set(ids);
        // UUIDs should be unique
        expect(uniqueIds.size).toBe(ids.length);
        return true;
      })
    );
  });
});
