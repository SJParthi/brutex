// THE DEV PROXY LIST AGAINST WHAT THE PAGES ACTUALLY CALL.
//
// `node --test web/tests/` — the runner is node's own, so this costs no
// dependency and no toolchain. It reads the two files directly; nothing here
// is a copy of either list.
//
// This drift has happened four times. `/feeds.json` was missed and the feed
// picker showed a bare 404. `/autopilot/control` was missed and the page
// printed a refusal for a POST that never reached Rust. `/folder.json` and
// `/ingest/status.json` were missed and — because neither has a content-type
// guard — `r.ok` was true for the dev server's HTML fallback and `r.json()`
// threw, so a routing gap was reported to the operator as a JSON parse error.
//
// Every one of those was found by a person noticing. This is the check that
// notices instead.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const web = join(dirname(fileURLToPath(import.meta.url)), '..');

/** Every `ROUTES` entry in `vite.config.js`, comments and all stripped by the regex. */
function proxied() {
  const src = readFileSync(join(web, 'vite.config.js'), 'utf8');
  const block = src.slice(src.indexOf('const ROUTES = ['), src.indexOf('];'));
  return [...block.matchAll(/^\s*'([^']+)'/gm)].map((m) => m[1]);
}

/** Every file under `web/src`, at any depth. */
function sources(dir) {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    return statSync(full).isDirectory() ? sources(full) : [full];
  });
}

/**
 * Every absolute path handed to `fetch`, query string removed.
 *
 * Only absolute paths: a relative one is a Svelte route and is never proxied.
 */
function fetched() {
  const out = new Set();
  for (const file of sources(join(web, 'src'))) {
    const text = readFileSync(file, 'utf8');
    // `ask(` as well as `fetch(`: `$lib/ask.js` wraps every request so it has a
    // ceiling, so the call sites spell it `ask`. Scanning for only the bare word
    // would report a clean list while covering nothing — which is precisely how
    // this list drifted four times.
    for (const m of text.matchAll(/\b(?:fetch|ask)\(\s*[`'"](\/[^`'"?]*)/g)) {
      out.add({ path: m[1], file: file.slice(web.length + 1) }.path);
    }
  }
  return [...out];
}

test('every route the pages fetch is proxied to the Rust server in dev', () => {
  const routes = proxied();
  assert.ok(routes.length > 5, 'the ROUTES block did not parse');
  for (const path of fetched()) {
    // Vite matches a proxy key by prefix, so `/pull` covers `/pull/spot`.
    const covered = routes.some((r) => path === r || path.startsWith(r));
    assert.ok(
      covered,
      `web/src fetches ${path}, which no entry in vite.config.js ROUTES covers. ` +
        `In \`npm run dev\` that request is answered by the dev server's HTML ` +
        `fallback with a 200, and the page reports a JSON error for a routing gap.`
    );
  }
});

test('the proxy list carries no entry nothing fetches', () => {
  // A stale entry is not a failure, but it is a claim that something calls a
  // route. `/audit.json` and `/bars.json` are reached through helpers rather
  // than a literal `fetch(`, so absence from the scan is not proof of death —
  // this asserts only that the list has not become mostly fiction.
  const routes = proxied();
  const paths = fetched();
  const live = routes.filter((r) => paths.some((p) => p === r || p.startsWith(r)));
  assert.ok(
    live.length >= 4,
    `only ${live.length} of ${routes.length} proxy entries match any fetch in web/src`
  );
});
