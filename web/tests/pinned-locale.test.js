// THE LOCALE AND THE ZONE ARE NAMED, NEVER INHERITED — CE-72 (D-2733).
//
// `$lib/money.js`: "Pinned, never inherited. The host's locale is not this
// product's." A `toLocaleString()` / `toLocaleDateString()` /
// `toLocaleTimeString()` with NO argument formats in the host browser's
// locale, and on a Date in the host's zone with no label: an en-US browser
// printed 87,828,617 beside 8,78,28,617, and a London one a master file's time
// 5h30m off IST. Every such call in `web/src` is refused here.

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const SRC = fileURLToPath(new URL('../src', import.meta.url));

/** @param {string} dir @returns {string[]} */
function sources(dir) {
  return readdirSync(dir)
    .sort()
    .flatMap((name) => {
      const full = join(dir, name);
      if (statSync(full).isDirectory()) return sources(full);
      return /\.(svelte|js|ts)$/.test(name) ? [full] : [];
    });
}

test('no source formats in the host browser\'s locale or zone', () => {
  const files = sources(SRC);
  assert.ok(files.length > 50, 'the walk found the source tree');
  const bare = /\.toLocale(?:Date|Time)?String\(\s*\)/;
  const hits = files.flatMap((f) =>
    readFileSync(f, 'utf8')
      .split('\n')
      .flatMap((line, i) => (bare.test(line) ? [`${f.slice(SRC.length + 1)}:${i + 1}`] : []))
  );
  assert.deepEqual(hits, []);
});

test('the four CE-72 sites use the pinned formatters', () => {
  const mapping = readFileSync(join(SRC, 'routes/mapping/+page.svelte'), 'utf8');
  assert.match(mapping, /\{stampLabel\(file\.modified_unix_millis\)\} IST/);
  assert.match(mapping, /\{group\(file\.bytes\)\} bytes/);
  assert.match(mapping, /\{group\(done\.bytes\)\} bytes/);
  const backtest = readFileSync(join(SRC, 'routes/backtest/+page.svelte'), 'utf8');
  for (const field of ['bars', 'minHits', 'candidates', 'priced']) {
    assert.match(backtest, new RegExp(`group\\(r\\.${field}\\)`));
  }
});
