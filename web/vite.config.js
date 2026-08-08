import { sveltekit } from '@sveltejs/vite-plugin-svelte';

/**
 * `/api` proxies to the Rust server in development so the browser talks to one
 * origin. In production the Rust binary serves both the built assets and the
 * JSON, so there is no proxy and no CORS.
 */
export default {
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/instruments.json': 'http://127.0.0.1:8731',
      '/bars.json': 'http://127.0.0.1:8731',
      '/store.json': 'http://127.0.0.1:8731'
    }
  }
};
