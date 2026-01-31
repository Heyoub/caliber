import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  css: {
    devSourcemap: true,
  },
  server: {
    port: 5173,
    strictPort: false,
  },
  build: {
    sourcemap: true,
  },
});
