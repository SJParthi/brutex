import adapter from '@sveltejs/adapter-static';
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

/**
 * THE BUILD IS A FUNCTION OF ITS SOURCE, AND UNTIL THIS IT WAS NOT.
 *
 * SvelteKit's default `version.name` is `Date.now()`. It is baked into the
 * client manifest, so it changes the manifest's bytes, so it changes that
 * chunk's content hash, so it changes every filename that references it.
 * Measured: rebuilding IDENTICAL source rewrote 18 of 39 tracked files.
 *
 * That made the one check this repository most needs impossible to write.
 * `web/build` is committed under D-0068 and served from disk by the Rust
 * binary, so the artifact in the tree is what an operator actually runs -- and
 * nothing verified it corresponds to `web/src`. `31cd9a1` is the standing
 * proof: 1,150 source lines changed, zero build files, CI green throughout.
 * Any gate that rebuilt and diffed would have failed on every run for a reason
 * that was never the source, which trains a reader to ignore it.
 *
 * So the version is a DIGEST OF THE SOURCE instead of the clock. Two things
 * follow. The build becomes reproducible, so `npm run build && git diff
 * --exit-code web/build` is a real check. And `version.json` becomes a
 * fingerprint of the tree that produced it, so staleness is legible without a
 * rebuild at all.
 *
 * It covers `src/`, the saved-selection helper imported by the main app, the
 * lockfile, and the two config files. An imported helper outside `src/` is
 * still executable input: leaving it out lets the client bundle change while
 * its version stays the same. `package.json` is not read directly --
 * `package-lock.json` carries it.
 */
function sourceDigest() {
  const hash = createHash('sha256');
  const walk = (dir) => {
    for (const name of readdirSync(dir).sort()) {
      const full = join(dir, name);
      if (statSync(full).isDirectory()) walk(full);
      else {
        hash.update(full);
        hash.update(readFileSync(full));
      }
    }
  };
  walk('src');
  for (const file of ['package-lock.json', 'svelte.config.js', 'vite.config.js', 'saved-backtest/selection.js']) {
    hash.update(file);
    hash.update(readFileSync(file));
  }
  return hash.digest('hex').slice(0, 16);
}

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
    // Not the clock. See `sourceDigest` above -- this is what makes the build
    // reproducible, and therefore what makes a stale-bundle gate possible.
    version: { name: sourceDigest(), pollInterval: 0 },
    prerender: {
      handleHttpError: ({ path, message }) => {
        if (SERVER_RENDERED.has(path)) return;
        throw new Error(message);
      }
    }
  }
};
