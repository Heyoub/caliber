// @ts-check
import { defineConfig } from 'astro/config';

import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';

// https://astro.build/config
export default defineConfig({
  site: 'https://caliber.run',
  integrations: [svelte()],

  vite: {
    plugins: [tailwindcss()],
    build: {
      // Optimize CSS
      cssMinify: 'lightningcss',
      // Optimize JS chunks
      rollupOptions: {
        output: {
          manualChunks: {
            svelte: ['svelte'],
            motion: ['motion']
          }
        }
      }
    }
  },

  // Compress HTML output
  compressHTML: true,

  build: {
    // Inline small assets
    inlineStylesheets: 'auto'
  }
});