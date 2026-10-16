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
// D-3325 MOVED THE REFUSALS THAT DEPEND ON THE SERIES INTO `bars_change.rs`
// (`Some("corporate_action_unverified")` and its siblings), and the first-bar
// code split into `no_earlier_stored_bar`, `earlier_month_not_read` and
// `first_bar_in_window`. /db had a sentence for none of them, so the fake
// crash the server now refuses read "unknown … a defect here" (DB-010,
// DB-G43, DB-G74, DB-G65, DB-G90).
const CHANGE = readFileSync(new URL('../../crates/api/src/bars_change.rs', import.meta.url), 'utf8');

/** The codes a Rust file's non-test code can send as a change reason. @param {string} text */
const codesOf = (text) => {
  const code = text.split('\n#[cfg(test)]')[0];
  return [
    ...[...code.matchAll(/\(None, "([a-z_]+)"\)/g)].map((m) => m[1]),
    ...[...code.matchAll(/(?<![a-z_])Some\("([a-z_]+)"\)/g)].map((m) => m[1]),
    // `Behind::NotRead("first_bar_in_window")` and the zero-base code a ratio
    // is refused with, `ratio(before.close, bar.close, "previous_close_zero")`.
    ...[...code.matchAll(/NotRead\("([a-z_]+)"\)/g)].map((m) => m[1]),
    ...[...code.matchAll(/ratio\([^;]*?, "([a-z_]+)"\)/g)].map((m) => m[1])
  ];
};

test('every withheld-change code bars.rs and bars_change.rs can send has a sentence on /db', () => {
  const sent = new Set([...codesOf(BARS), ...codesOf(CHANGE)]);
  assert.ok(sent.size >= 16, `found only ${[...sent].join(', ')}`);
  for (const code of ['corporate_action_unverified', 'previous_session_not_adjacent', 'no_earlier_stored_bar', 'not_a_derivative']) {
    assert.ok(sent.has(code), `${code} is no longer sent; this test reads the wrong file`);
  }
  const explained = new Set(Object.keys(BAR_WHY));
  const missing = [...sent].filter((c) => !explained.has(c));
  assert.deepEqual(missing, [], `codes /db would call a defect: ${missing.join(', ')}`);
});
