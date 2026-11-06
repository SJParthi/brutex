// THE BROWSER'S EXPORT AGREES WITH THE SERVER'S (R3-10; moved here by
// BASE-REDS-1023, D-7153).
//
// `api::server::bars_export`'s tests read `web/` to check two facts the
// browser and the server must state alike, and gate 1e builds and tests the
// workspace with `web/` moved aside (CLAUDE.md §2: no crate may need the
// front end to build, test or run), so they failed there. The check lives on
// this side now, reading the Rust source as text: the export ceiling and the
// bar CSV columns, word for word and in order.

import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import { EXPORT_CAP } from '../src/lib/db/bars-plan.js';
import { BAR_CSV_COLS } from '../src/lib/db/export.js';

const server = readFileSync(new URL('../../crates/api/src/bars_export.rs', import.meta.url), 'utf8');

/**
 * The value of the `const NAME` item of the Rust source: the text after its
 * `=` up to the `;` that ends it (an array's own `[&str; N]` type holds a
 * `;` of its own, so the value is read after the `=`).
 */
const item = (/** @type {string} */ name) => {
  const at = server.indexOf(`const ${name}:`);
  assert.ok(at >= 0, `bars_export.rs declares ${name}`);
  const eq = server.indexOf('=', at);
  const end = server.indexOf(';', eq);
  assert.ok(eq > at && end > eq, `${name} has a value`);
  return server.slice(eq + 1, end);
};

test('the browser\'s export cap is the server\'s ceiling', () => {
  const ceiling = item('EXPORT_CEILING').trim().replaceAll('_', '');
  assert.equal(String(EXPORT_CAP), ceiling);
});

test('the browser\'s bar CSV columns are the server\'s, word for word and in order', () => {
  const body = item('COLUMNS');
  const columns = [...body.matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  assert.equal(columns.length, 35, 'the server declares 35 columns');
  assert.deepEqual(
    BAR_CSV_COLS.map((c) => c.label),
    columns
  );
});
