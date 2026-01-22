// @ts-check
import { defineConfig } from 'astro/config';

import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';
import vercel from '@astrojs/vercel';

// https://astro.build/config
export default defineConfig({
  site: 'https://caliber.run',
  // Static by default, opt-in SSR for dashboard pages (prerender = false)
  adapter: vercel(),
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