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

/**
 * Every file under `web/src`, at any depth.
 *
 * @param {string} dir
 * @returns {string[]}
 */
function sources(dir) {
  /** @type {string[]} */
  const out = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) out.push(...sources(full));
    else out.push(full);
  }
  return out;
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
    //
    // AND WHATEVER THE FILE RENAMED `ask` TO ON THE WAY IN. Two files import it
    // under a local alias -- `import { ask as request }` in
    // `ingest/+page.svelte` and `import { ask as ask_ }` in
    // `backtest/+page.svelte` -- which put 15 of the tree's 28 call sites
    // outside this scan entirely. `request(` matched nothing; `ask_(` failed
    // because the old pattern required `ask` to be followed immediately by `(`.
    //
    // WHAT THE BLIND SPOT COST: this suite passed 145/145 while
    // `/calendar.json`, `/vocab.json` and `/universes.json` were each called,
    // each served, and each absent from ROUTES. `/calendar.json` is the
    // expensive one -- under `npm run dev` the HTML fallback answers 200,
    // `.json()` throws, the holiday table stays empty, and `/ingest` reverts to
    // "weekday = session", so every shortfall figure on that page is computed
    // against an invented calendar. Silently.
    //
    // A green suite from a scanner that cannot see half the call sites is worse
    // than no suite: it is exactly the false assurance this file exists to
    // prevent. So the alias is read out of the import and joined to the pattern.
    const callees = new Set(['fetch', 'ask']);
    for (const a of text.matchAll(/import\s*\{[^}]*\bask\s+as\s+([A-Za-z_$][\w$]*)/g)) {
      callees.add(a[1]);
    }
    const names = [...callees].map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|');
    // `(?![\w$])` rather than a trailing `\b`: `\b` between `ask` and `_` does
    // not exist, but the bare name still matched the alias's prefix and then
    // failed on the `(`, so `ask_(` was invisible twice over.
    for (const m of text.matchAll(
      new RegExp(String.raw`\b(?:${names})(?![\w$])\s*\(\s*[\`'"](\/[^\`'"?]*)`, 'g')
    )) {
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
