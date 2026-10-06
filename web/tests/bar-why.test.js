// EVERY REASON CODE THE BAR WINDOW CAN SEND HAS A SENTENCE ON /db.
//
// `crates/api/src/bars.rs` withholds a change with one of a small set of codes,
// and `/db` turns each into a sentence. `previous_unreadable` and `overflow`
// had none, so a real, explained withholding was shown as "unknown … a defect
// here" (CE-27, D-1769). This reads both files and fails when the server can
// send a code the page cannot explain.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const BARS = readFileSync(new URL('../../crates/api/src/bars.rs', import.meta.url), 'utf8');
const PAGE = readFileSync(new URL('../src/routes/db/+page.svelte', import.meta.url), 'utf8');

test('every withheld-change code bars.rs can send has a sentence on /db', () => {
  const code = BARS.split('\n#[cfg(test)]')[0];
  const sent = new Set([...code.matchAll(/\(None, "([a-z_]+)"\)/g)].map((m) => m[1]));
  assert.ok(sent.size >= 7, `found only ${[...sent].join(', ')}`);
  const table = PAGE.split('const BAR_WHY = {')[1]?.split('\n  };')[0] ?? '';
  const explained = new Set([...table.matchAll(/^\s{4}([a-z_]+):/gm)].map((m) => m[1]));
  const missing = [...sent].filter((c) => !explained.has(c));
  assert.deepEqual(missing, [], `codes /db would call a defect: ${missing.join(', ')}`);
});
