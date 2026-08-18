import adapter from '@sveltejs/adapter-static';

/**
 * STATIC OUTPUT, AND THAT IS THE WHOLE INTEGRATION STORY.
 *
 * `adapter-static` emits plain files into `web/build`. The Rust binary serves
 * them and never runs Node — CLAUDE.md section 2 and D-0053: `cargo build` must
 * succeed on a machine that has never installed a package manager, and gate 1e
 * checks it by building with `web/` moved aside.
 *
 * `fallback` makes every unknown path serve the shell, so client-side routing
 * works without the server knowing every route.
 */
/**
 * THE PAGES RUST OWNS, WHICH THIS BUILD MUST NOT TRY TO RESOLVE.
 *
 * `crates/api/src/render.rs` serves plain server-rendered HTML on these paths,
 * with ordinary form POSTs and no script -- they are what `app.html`'s
 * `<noscript>` block sends an operator to when the bundle cannot run. They are
 * real URLs on the same origin and they are NOT SvelteKit routes, so the
 * prerenderer crawls the links, gets a 404 from a dev server that does not
 * serve them, and fails the build.
 *
 * Named one by one rather than silenced with a blanket `handleHttpError:
 * 'warn'`. A blanket ignore would also swallow a genuine broken link between
 * two Svelte pages, which is exactly the failure prerendering is useful for
 * catching. Anything not on this list still throws.
 */
const SERVER_RENDERED = new Set([
  '/dashboard',
  '/instruments',
  '/store',
  '/bars',
  '/logs'
]);

export default {
  kit: {
    adapter: adapter({ pages: 'build', assets: 'build', fallback: 'index.html', precompress: false }),
    paths: { relative: false },
    prerender: {
      handleHttpError: ({ path, message }) => {
        if (SERVER_RENDERED.has(path)) return;
        throw new Error(message);
      }
    }
  }
};
