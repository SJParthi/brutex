// WHAT AN OPTION CELL SAYS WHEN IT HAS NO FIGURE, AND WHAT THE PAGE COUNTS.
//
// WP-53 (D-3328, D-3329, D-3330). The servers name every refusal; /db turns
// each into a short word on the cell, a sentence on its title and a count in
// the page's tally. This reads the codes the three routes can send OUT OF
// THEIR RUST SOURCE and fails when one has no word or no sentence — the shape
// `bar-why.test.js` uses for the change columns — and checks the tally, the
// per-family note, the Moneyness test of one bar and the ended-early rule on
// invented rows. No row here is market data.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  REFUSAL_WORD, PENDING, endedEarly, familyNote, greekInputs, reasonText, refusalWord, rungMatches, tallyOf, tallyRows
} from '../src/lib/db/notes-options.js';
import { GREEK_WHY } from '../src/lib/greeks.js';
import { RUNG_WHY, SPOT_WHY } from '../src/lib/moneyness.js';
import { dbSource } from './database-page-fixture.js';

/** A Rust file's code above its test module (a `#[cfg(test)]` helper above it stays). @param {string} name */
const rust = (name) =>
  readFileSync(new URL(`../../crates/api/src/${name}`, import.meta.url), 'utf8').split(/\n#\[cfg\(test\)\]\n(?:#\[path[^\n]*\n)?mod tests/)[0];

/** Every refusal code `/spot.json`, `/greeks.json` and `/chain.json` can send. */
function serverCodes() {
  const spot = rust('spot.rs');
  const why = spot.slice(spot.indexOf('pub const fn why(&self)'), spot.indexOf('pub const fn in_month'));
  const greeks = rust('optiongreeks.rs');
  const chain = rust('chain.rs');
  return {
    spot: [...why.matchAll(/Some\("([a-z_]+)"\)/g)].map((m) => m[1]),
    greeks: [
      ...[...greeks.matchAll(/(?:refused|refused_at|Cell::refused)\(\s*"([a-z_]+)"/g)].map((m) => m[1]),
      ...[...greeks.matchAll(/Err\(\("([a-z_]+)"/g)].map((m) => m[1])
    ],
    chain: [
      ...[...chain.matchAll(/"([a-z]+(?:_[a-z]+)+)",\s*\n\s*(?:format!\(|"|[a-z_]+\.to_owned)/g)].map((m) => m[1]),
      ...[...chain.matchAll(/r#""([a-z_]+)""#/g)].map((m) => m[1])
    ]
  };
}

test('every refusal code the option routes can send has a word on the cell and a sentence behind it', () => {
  const codes = serverCodes();
  // The extraction reads the right places: these are known to be sent.
  for (const c of ['no_spot_bar', 'no_spot_in_session', 't_outside_month', 'no_spot_file']) assert.ok(codes.spot.includes(c), c);
  for (const c of ['below_zero_carry_bound', 'iv_unsolved', 'rate_stale', 'no_option_bar']) assert.ok(codes.greeks.includes(c), c);
  for (const c of ['census_behind', 'ladder_too_sparse', 'no_strike_stored_by_then', 'strike_not_stored_by_then']) {
    assert.ok(codes.chain.includes(c), c);
  }
  const all = new Set([...codes.spot, ...codes.greeks, ...codes.chain]);
  const noWord = [...all].filter((c) => !Object.hasOwn(REFUSAL_WORD, c));
  assert.deepEqual(noWord, [], `codes the cell would show raw: ${noWord.join(', ')}`);
  const noSentence = [...all].filter((c) => reasonText(c).startsWith('unknown'));
  assert.deepEqual(noSentence, [], `codes with no sentence: ${noSentence.join(', ')}`);
  // Each table answers for its own column: a spot word in SPOT_WHY and
  // GREEK_WHY (greeks price from the same spot), a chain word in RUNG_WHY.
  for (const c of codes.spot) {
    assert.equal(typeof /** @type {any} */ (SPOT_WHY)[c], 'string', `SPOT_WHY.${c}`);
    // `/greeks.json` forwards the spot's own word (`found.why()`).
    assert.equal(typeof /** @type {any} */ (GREEK_WHY)[c], 'string', `GREEK_WHY.${c}`);
  }
  for (const c of codes.greeks) assert.equal(typeof /** @type {any} */ (GREEK_WHY)[c], 'string', `GREEK_WHY.${c}`);
  for (const c of codes.chain) assert.equal(typeof /** @type {any} */ (RUNG_WHY)[c], 'string', `RUNG_WHY.${c}`);
  // A word is short enough for a cell; an unknown code shows itself.
  // A word fits its column: a greek column holds ten characters, the
  // Moneyness column the rung's own longer words.
  const rungOnly = new Set(codes.chain.filter((c) => !codes.spot.includes(c)));
  for (const [code, w] of Object.entries(REFUSAL_WORD)) assert.ok(w.length <= (rungOnly.has(code) ? 13 : 10), `${code}: ${w}`);
  assert.equal(refusalWord('census_behind'), 'census behind');
  assert.equal(refusalWord('a_new_code'), 'a_new_code');
});

test('the tally counts each refusal per column, the flags, the waits and the earlier spots, once per page', () => {
  const rows = ['a', 'b', 'c', 'd', 'e', 'f'].map((id) => ({ id }));
  /** @type {Record<string, any>} */
  const oc = {
    a: { mny: 'ATM', mnyWhy: null, why: null, age: 0 },
    b: { mny: null, mnyWhy: 'census_behind', why: null, age: 4 },
    c: { mny: null, mnyWhy: 'no_spot_bar', why: 'no_spot_bar', age: null },
    d: { mny: null, mnyWhy: 'rung_pending', why: 'spot_pending', age: null },
    e: { mny: 'OTM+3', mnyWhy: null, why: null, age: 9 },
    f: null
  };
  /** @type {Record<string, any>} */
  const gc = {
    a: { why: null, flags: ['above_rate_tenor'] },
    b: { why: 'below_zero_carry_bound', flags: ['above_rate_tenor'] },
    c: { why: 'no_spot_bar', flags: [] },
    d: { why: 'greeks_pending', flags: [] },
    e: { why: null, flags: ['above_rate_tenor', 'below_band'] },
    f: null
  };
  const t = tallyOf(rows, (r) => oc[r.id], (r) => gc[r.id]);
  assert.equal(t.options, 5, 'a row that is not an option is not counted');
  assert.equal(t.named, 2);
  assert.equal(t.priced, 2);
  assert.equal(t.pending, 1);
  assert.deepEqual([...t.mny], [['census_behind', 1], ['no_spot_bar', 1]]);
  assert.deepEqual([...t.spot], [['no_spot_bar', 1]]);
  assert.deepEqual([...t.greeks], [['below_zero_carry_bound', 1], ['no_spot_bar', 1]]);
  assert.deepEqual([...t.flags], [['above_rate_tenor', 2], ['below_band', 1]], 'a refused cell’s flags are not counted as priced');
  assert.deepEqual([t.earlier, t.oldest], [2, 9]);
  assert.deepEqual(tallyRows(new Map([['iv_unsolved', 2], ['no_spot_bar', 5], ['census_behind', 2]])), [
    { code: 'no_spot_bar', word: 'no spot', n: 5 },
    { code: 'census_behind', word: 'census behind', n: 2 },
    { code: 'iv_unsolved', word: 'unsolved', n: 2 }
  ]);
  for (const p of PENDING) assert.ok(p.endsWith('_pending'));
});

test('a Moneyness pick keeps a bar by its own rung: exact, a family at any depth, never a bar with no rung', () => {
  assert.equal(rungMatches(new Set(), null), true, 'no pick keeps everything');
  assert.equal(rungMatches(new Set(['ATM']), 'ATM'), true);
  assert.equal(rungMatches(new Set(['ATM']), 'ITM-1'), false);
  assert.equal(rungMatches(new Set(['ITM']), 'ITM-17'), true);
  assert.equal(rungMatches(new Set(['ITM']), 'OTM+1'), false);
  assert.equal(rungMatches(new Set(['OTM']), 'OTM+2'), true);
  assert.equal(rungMatches(new Set(['ITM-2']), 'ITM-2'), true);
  assert.equal(rungMatches(new Set(['ITM-2']), 'ITM-20'), false);
  assert.equal(rungMatches(new Set(['ATM']), null), false, 'a refused or unread rung is on no rung');
  assert.equal(rungMatches(new Set(['ITM']), undefined), false);
});

test('an underlying with no spot file is named once, with only the cause the alias table states (DB-069, DB-G53)', () => {
  const read = { segment: 'CASH', key: 'TMPV' };
  const renamed = familyNote('TATAMOTORS', read, {
    matched: 'symbol_change', store_key: 'TMPV',
    changes: [{ old: 'TATAMOTORS', new: 'TMPV', effective: '2025-10-24' }], not_aliased: null
  }, ['2019-03', '2019-04']);
  assert.match(renamed, /^TATAMOTORS options: underlying spot not imported — \/spot\.json read CASH\/TMPV for 2019-03 to 2019-04/);
  assert.match(renamed, /Renamed: TATAMOTORS → TMPV on 2025-10-24/);
  assert.match(renamed, /D-3333 imports it/);
  const index = familyNote('FINNIFTY', { segment: 'INDEX', key: 'NIFTYFINSERVICE' },
    { matched: 'index', store_key: 'NIFTYFINSERVICE', index: { nse_name: 'NIFTY FINANCIAL SERVICES' }, changes: [] }, ['2021-01']);
  assert.match(index, /FINNIFTY is an index option: its index series \(NIFTY FINANCIAL SERVICES\) files under NIFTYFINSERVICE/);
  const kept = familyNote('HDFC', { segment: 'CASH', key: 'HDFC' },
    { matched: 'not_aliased', changes: [], not_aliased: { why: 'no row in NSE’s symbolchange.csv' } }, ['2019-05']);
  assert.match(kept, /Not renamed: no row in NSE’s symbolchange\.csv/);
  const none = familyNote('ACC', { segment: 'CASH', key: 'ACC' }, { matched: 'none', changes: [], not_aliased: null }, ['2019-05']);
  assert.match(none, /No symbol change or alias is recorded for ACC/);
  assert.match(none, /which of these applies is UNVERIFIED/);
  assert.doesNotMatch(none, /Renamed|index option/);
  const unread = familyNote('ACC', { segment: 'CASH', key: 'ACC' }, null, ['2019-05']);
  assert.match(unread, /the cause is not stated here/);
});

test('an expiry ended early only when the import reached it, the census is not behind and no contract traded on its day (DB-B14)', () => {
  assert.deepEqual(endedEarly('2099-03-28', '2099-02-28', '2100-09-16', null), { ended: true, last: '2099-02-28', days: 28 });
  assert.equal(endedEarly('2099-03-28', '2099-03-28', '2100-09-16', null).ended, false);
  assert.equal(endedEarly('2100-09-24', '2100-09-16', '2100-09-16', null).ended, false, 'the import stopped before the expiry');
  assert.equal(endedEarly('2099-03-28', '2099-02-28', '2100-09-16', '2099-03-01').ended, false, 'the census is behind');
  assert.equal(endedEarly('2099-03-28', '2099-02-28', '2100-09-16', '2099-04-01').ended, true, 'behind only after the expiry');
  assert.equal(endedEarly('2099-03-28', null, '2100-09-16', null).ended, false);
});

test('a priced cell’s title names the tenor in years and every flag (D-3330, DB-G64)', () => {
  const title = greekInputs({ premium: 79605, spot: 1194530, spot_age_s: 0, tenor_s: 143524681, tenor_y: 4.5511, rate: 0.0602,
    auction: '2099-06-04', above_rate_tenor: true, below_band: false });
  assert.match(title, /tenor 4\.55 y \(14,35,24,681 s\)/);
  assert.match(title, /FLAGGED above_rate_tenor: the contract runs longer than 91 days/);
  assert.doesNotMatch(title, /below_band/);
  assert.match(greekInputs({ premium: 1, spot: 2, spot_age_s: 3, tenor_s: 7, tenor_y: 7 / 31536000, rate: 0.06, auction: 'x', below_band: true }),
    /\(3 s old: last spot at or before t\).*tenor 7 s.*FLAGGED below_band/);
  assert.equal(greekInputs(null), '');
});

test('the page tallies its refusals above the grid and names each family once', () => {
  const notes = dbSource('../src/routes/db/DbOptionNotes.svelte');
  assert.match(notes, /db\.optionTally/);
  assert.match(notes, /familyNote\(f\.underlying/);
  assert.match(notes, /endedEarly\(/);
  assert.match(dbSource('../src/routes/db/DbBarsGrid.svelte'), /<DbOptionNotes \{db\} \/>/);
  const cells = dbSource('../src/routes/db/DbOptionCells.svelte');
  assert.match(cells, /ask\(`\/aliases\.json\?\$\{new URLSearchParams\(\{ symbol: u \}\)\}`/);
  assert.match(cells, /if \(!s \|\| s\.why !== 'no_spot_file'\) continue;/);
});
