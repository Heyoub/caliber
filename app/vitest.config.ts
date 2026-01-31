import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { resolve } from 'path';

export default defineConfig({
  plugins: [svelte({ hot: !process.env.VITEST })],
  test: {
    globals: true,
    environment: 'jsdom',
    include: ['tests/**/*.{test,spec}.{js,ts}'],
    exclude: ['node_modules', 'dist', '.svelte-kit'],
    setupFiles: ['./tests/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      reportsDirectory: './coverage',
      include: ['src/**/*.{ts,svelte}'],
      exclude: ['src/**/*.d.ts', 'src/**/index.ts', 'node_modules/**'],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 75,
        statements: 80,
      },
    },
    reporters: ['verbose', 'html'],
    outputFile: {
      html: './test-results/index.html',
    },
    typecheck: {
      enabled: true,
      tsconfig: './tsconfig.json',
    },
    // Deterministic test ordering
    sequence: {
      shuffle: false,
    },
    // Timeout settings
    testTimeout: 10000,
    hookTimeout: 10000,
  },
  resolve: {
    alias: {
      $lib: resolve('./src/lib'),
      '@caliber/ui': resolve('../packages/ui/src'),
    },
  },
});
