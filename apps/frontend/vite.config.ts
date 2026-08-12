import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  // Prevent vite from obscuring Rust errors
  clearScreen: false,
  server: {
    // Tauri expects this port
    port: 5173,
    // Strict CSP for local-only operation
    strictPort: true,
    // Only listen on localhost — never expose to network
    host: '127.0.0.1'
  },
  // Env variables starting with TAURI_ will be exposed to the frontend
  envPrefix: ['VITE_', 'TAURI_']
});
