import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: {
    proxy: {
      '/ws': { target: 'ws://localhost:8015', ws: true },
      '/mock-workspace': 'http://localhost:8015',
    },
  },
});
