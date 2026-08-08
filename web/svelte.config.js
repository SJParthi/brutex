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
export default {
  kit: {
    adapter: adapter({ pages: 'build', assets: 'build', fallback: 'index.html', precompress: false }),
    paths: { relative: false }
  }
};
