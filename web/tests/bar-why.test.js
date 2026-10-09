// EVERY REASON CODE THE BAR WINDOW CAN SEND HAS A SENTENCE ON /db.
//
// `crates/api/src/bars.rs` withholds a change with one of a small set of codes,
// and `/db` turns each into a sentence. `previous_unreadable` and `overflow`
// had none, so a real, explained withholding was shown as "unknown … a defect
// here" (CE-27, D-1769). This reads the server's codes and the page's table of
// sentences and fails when the server can send a code the page cannot explain.
//
// THE TABLE MOVED, NOT THE RULE. WP-00 split `routes/db/+page.svelte`, and
// `BAR_WHY` now lives in `$lib/db/notes-bars.js`, which the bar grid imports.
// The table is imported rather than cut out of the text, so the codes read are
// the keys the page actually has.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { BAR_WHY } from '../src/lib/db/notes-bars.js';

const BARS = readFileSync(new URL('../../crates/api/src/bars.rs', import.meta.url), 'utf8');

test('every withheld-change code bars.rs can send has a sentence on /db', () => {
  const code = BARS.split('\n#[cfg(test)]')[0];
  const sent = new Set([...code.matchAll(/\(None, "([a-z_]+)"\)/g)].map((m) => m[1]));
  assert.ok(sent.size >= 7, `found only ${[...sent].join(', ')}`);
  const explained = new Set(Object.keys(BAR_WHY));
  const missing = [...sent].filter((c) => !explained.has(c));
  assert.deepEqual(missing, [], `codes /db would call a defect: ${missing.join(', ')}`);
});
