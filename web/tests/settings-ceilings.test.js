// THE BROWSER'S COPIES OF THE REQUEST CEILINGS (R3-05).
//
// `/settings.json` serves every ceiling of one request from the constant that
// enforces it (`crates/api/src/settings_route.rs` `ceilings_json`). Until /db
// reads them from there, `web/src/lib/db/bars-plan.js` keeps its own copies,
// and this test is what keeps each copy equal to the served constant: a drift
// on either side fails here.
//
// It reads the Rust SOURCE as text and runs no Rust: the check lives on the
// browser's side because gate 1e runs the cargo suite with `web/` detached,
// so a Rust test may not open a page's file (CLAUDE.md section 2).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  LOCAL_SORT_BUDGET,
  WINDOW_PAGE_MAX,
  MAX_SERIES,
  MAX_WINDOW_MONTHS,
  EXPORT_CAP
} from '../src/lib/db/bars-plan.js';

/** The value of `pub const <name>: <type> = <digits>;` in a Rust file. */
function rustConst(file, name) {
  const text = readFileSync(new URL(`../../crates/api/src/${file}`, import.meta.url), 'utf8');
  const pattern = new RegExp(`^pub const ${name}: [a-z0-9]+ = ([0-9_]+);$`, 'm');
  const found = text.match(pattern);
  assert.ok(found, `no \`pub const ${name}\` in crates/api/src/${file}`);
  return Number(found[1].replaceAll('_', ''));
}

test('each ceiling the browser copies equals the constant /settings.json serves', () => {
  for (const [copy, file, name] of [
    [LOCAL_SORT_BUDGET, 'settings_route.rs', 'BROWSER_SORT_BUDGET'],
    [EXPORT_CAP, 'settings_route.rs', 'EXPORT_CAP'],
    [WINDOW_PAGE_MAX, 'bars.rs', 'MAX_WINDOW_LIMIT'],
    [MAX_WINDOW_MONTHS, 'bars.rs', 'MAX_WINDOW_MONTHS'],
    [MAX_SERIES, 'bars_series.rs', 'MAX_SERIES']
  ]) {
    const served = rustConst(file, name);
    assert.ok(Number.isSafeInteger(served) && served > 0, `${name}: ${served}`);
    assert.equal(copy, served, `bars-plan.js holds ${copy} where ${name} is ${served}`);
  }
});

test('a constant that is not there is refused by name, never read as zero', () => {
  assert.throws(() => rustConst('bars.rs', 'NO_SUCH_CEILING'), /no `pub const NO_SUCH_CEILING`/);
});
