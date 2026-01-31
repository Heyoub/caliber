import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [
    svelte({
      compilerOptions: {
        runes: true,
      },
    }),
  ],
  build: {
    lib: {
      entry: './src/index.ts',
      name: 'CaliberUI',
      fileName: 'caliber-ui',
    },
    rollupOptions: {
      external: ['svelte', 'svelte/internal'],
    },
  },
});
