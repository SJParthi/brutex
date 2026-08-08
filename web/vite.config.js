import { sveltekit } from '@sveltejs/kit/vite';

/**
 * `/api` proxies to the Rust server in development so the browser talks to one
 * origin. In production the Rust binary serves both the built assets and the
 * JSON, so there is no proxy and no CORS.
 */
export default {
  plugins: [sveltekit()],
  server: {
    proxy: {
      // EVERY JSON ROUTE THE RUST SIDE SERVES. Listing them one by one is how
      // `/feeds.json` got missed and the feed picker showed a bare 404 —
      // loudly, which is right, but it should not have been possible. A prefix
      // would be tidier; these routes are not under one.
      '/instruments.json': 'http://127.0.0.1:8731',
      '/feeds.json': 'http://127.0.0.1:8731',
      '/bars.json': 'http://127.0.0.1:8731',
      '/store.json': 'http://127.0.0.1:8731',
      '/audit.json': 'http://127.0.0.1:8731'
    }
  }
};
