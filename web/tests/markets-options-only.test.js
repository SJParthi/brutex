// AH-04 (g-ui-dom-vs-json-5, D-5882): /markets counts an options-only
// listing as held. Rows are shaped like /instruments.json's (invented): a
// GDFL option underlying whose spot is not filed under its own name says
// `spot: false, options: true` beside `bars: 0`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { barStateOf, heldTally, isHeld, spotHome } from '../src/lib/markets-held.js';

const page = readFileSync(new URL('../src/routes/markets/+page.svelte', import.meta.url), 'utf8');
const master = { isThisFeed: true, wait: false, why: '' };
/** @param {Record<string, unknown>} over */
const row = (over) => ({ symbol: 'FINNIFTY', key: 'NSE-FNO-FINNIFTY', kind: null, exchange: 'NSE', segment: 'FNO', bars: 0, rows_by_rung: {}, spot: false, options: true, ...over });

test('an_options_only_listing_is_held_not_never_pulled', () => {
  assert.deepEqual(barStateOf(row({}), master), { kind: 'options' });
  assert.equal(isHeld(barStateOf(row({}), master)), true);
  // A spot listing with no bars and no options is the measured zero, as before.
  assert.deepEqual(barStateOf(row({ symbol: 'NOSUCH', options: false, spot: true }), master), { kind: 'zero' });
  // An older server that sends no `options` flag says nothing about options.
  assert.deepEqual(barStateOf(row({ options: undefined }), master), { kind: 'zero' });
  // Bars counted are a count whatever the options flag says.
  assert.deepEqual(barStateOf(row({ symbol: 'NIFTY', bars: 1_234, spot: true }), master), { kind: 'count', n: 1_234 });
  // options true and bars null: UNREAD, never 0 and never "options only".
  const unread = barStateOf(row({ bars: null }), master);
  assert.equal(unread.kind, 'unread');
  assert.match(/** @type {any} */ (unread).why, /option contracts are held/);
  assert.equal(isHeld(unread), false);
  // No master of this feed: nothing measured.
  assert.equal(barStateOf(row({}), { isThisFeed: false, wait: true, why: 'the master is Zerodha' }).kind, 'nomaster');

  // THE PARTITION SUMS, and the 17 options-only rows are in `held`.
  const rows = [
    ...Array.from({ length: 346 }, (_, i) => row({ symbol: `S${i}`, bars: 10 + i, spot: true, options: i % 2 === 0 })),
    ...Array.from({ length: 17 }, (_, i) => row({ symbol: `O${i}` })),
    row({ symbol: 'Z', options: false }),
    row({ symbol: 'U', bars: 'x' })
  ];
  const t = heldTally(rows, (r) => barStateOf(r, master));
  assert.deepEqual(t, { held: 363, optionsOnly: 17, zero: 1, unread: 1 });
  assert.equal(t.held + t.zero + t.unread, rows.length);
  assert.deepEqual(heldTally([], (r) => barStateOf(r, master)), { held: 0, optionsOnly: 0, zero: 0, unread: 0 });

  // THE PAGE READS THROUGH THIS: its state, its tally, its Held filter, and a
  // detail view that says "options only" before "not stored" and offers no Pull.
  assert.match(page, /return barStateOf\(row, \{ isThisFeed: masterIsThisFeed, wait: !catalogue\.error, why: countRefusal \}\);/);
  assert.match(page, /const \{ held, optionsOnly, zero, unread \} = heldTally\(rows, barState\);/);
  assert.match(page, /\(!heldOnly \|\| isHeld\(barState\(row\)\)\) &&/);
  const opt = page.indexOf("barState(picked).kind === 'options'");
  const nodata = page.indexOf("return { kind: 'nodata'");
  assert.ok(opt > 0 && opt < nodata, 'options-only is decided before not-stored');
  const branch = page.slice(page.indexOf("{:else if chartReason.kind === 'optonly'}"), page.indexOf("{:else if chartReason.kind === 'unreadable'}"));
  assert.match(branch, /Options only for/);
  assert.doesNotMatch(branch, /Pull it|can be pulled|never been pulled/);
  assert.match(page, /\{:else if st\.kind === 'options'\}\s*<span\s+class="nopt"/);
  assert.match(page, /options only = option contracts held, spot not stored under this name/);
  assert.match(page, /const RAIL_ANSWERS = new Set\(\['nodata', 'optonly',/, 'the right-hand bay answers an options-only row in its own words');
});

test('an_options_only_rows_spot_is_named_by_the_alias_table_never_guessed', () => {
  assert.equal(spotHome(null, 'FINNIFTY'), null, 'unread while the read is in flight');
  const index = spotHome({ matched: 'index', segment: 'INDEX', store_key: 'NIFTYFINSERVICE', key: 'NSE-INDEX-NIFTYFINSERVICE' }, 'FINNIFTY');
  assert.equal(index?.spotKey, 'NSE-INDEX-NIFTYFINSERVICE');
  assert.match(index?.text ?? '', /under INDEX NIFTYFINSERVICE/);
  const renamed = spotHome({ matched: 'symbol_change', key: 'NSE-CASH-OLDCO', changes: [{ old: 'OLDCO', new: 'NEWCO', effective: '2025-01-02', current: 'NEWCO', key: 'NSE-CASH-NEWCO' }], formerly: [] }, 'OLDCO');
  assert.equal(renamed?.spotKey, 'NSE-CASH-NEWCO');
  assert.match(renamed?.text ?? '', /renamed NEWCO from 2025-01-02/);
  const formerly = spotHome({ matched: 'symbol_change', key: 'NSE-CASH-NEWCO', changes: [], formerly: [{ old: 'OLDCO', new: 'NEWCO', effective: '2025-01-02' }] }, 'NEWCO');
  assert.match(formerly?.text ?? '', /formerly OLDCO/);
  const unknown = spotHome({ matched: 'none', key: 'NSE-CASH-ODDNAME' }, 'ODDNAME');
  assert.equal(unknown?.spotKey, null, 'an alias the table does not know names no key');
  assert.match(unknown?.text ?? '', /names no other key/);
  assert.match(spotHome({ matched: 'not_aliased', not_aliased: { why: 'an invented reason' } }, 'X')?.text ?? '', /an invented reason/);
  assert.match(spotHome(new Error('HTTP 503'), 'X')?.text ?? '', /could not be read: HTTP 503/);
});
