import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// Tauri exposes its CLI env vars to the frontend at build time
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || '0.0.0.0',
    hmr: host
      ? { protocol: 'ws', host, port: 1430 }
      : undefined,
    watch: {
      // tell vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**']
    }
  }
}));
