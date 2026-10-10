// THE /db BAR GRID'S PLAN, ITS WIRE AND ITS EXPORT (WP-52; D-3326, D-3322,
// D-3325, D-3343).
//
// Each test drives the real module the grid imports. The census rows and bars
// below are invented shapes, with prices nobody could take for real ones; no
// vendor row is quoted.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  WIRE_SORT, WINDOW_SORTS, LOCAL_SORTS, wireSort, seriesPlan, windowQuery, todWire, istInstant,
  parseGoto, pageOfIndex, censusBehind, localComparator, rowKey, MAX_SERIES, monthSpan
} from '../src/lib/db/bars-plan.js';
import { BAR_CSV_COLS, barCsvText, barCsvName, istClock } from '../src/lib/db/export.js';
import { BAR_COLS } from '../src/lib/db/notes-bars.js';
import { dbSource } from './database-page-fixture.js';

/** @param {string} instrument @param {string} month @param {number} rows @param {string} [timeframe] */
const census = (instrument, month, rows, timeframe = '1s') => ({ instrument, month, timeframe, rows });
const CE = 'NSE-FNO-NIFTY-2019-03-28-1150000-CE';
const PE = 'NSE-FNO-NIFTY-2019-03-28-1150000-PE';

test('Volume sorts on the server as v, Time as tod; only derived columns are ordered here (DB-062, DB-G94, DB-006)', () => {
  assert.equal(wireSort('vol'), 'v');
  assert.equal(wireSort('tod'), 'tod');
  assert.equal(wireSort('c'), 'c');
  assert.equal(wireSort('chg'), null);
  assert.ok(WINDOW_SORTS.has('vol') && WINDOW_SORTS.has('tod'));
  assert.ok(!WINDOW_SORTS.has('v'), 'the grid has no column keyed v');
  // EVERY SORTABLE COLUMN IS ORDERED SOMEWHERE: by the server, or here.
  for (const c of BAR_COLS) {
    if (c.none || c.nosort || c.key === 'tf') continue;
    assert.ok(Object.hasOwn(WIRE_SORT, c.key) || LOCAL_SORTS.has(c.key), `${c.key} has no order`);
  }
});

test('a selection plans its series, rung and months once, and refuses what one request cannot read', () => {
  const plan = seriesPlan([census(PE, '2019-03', 4), census(CE, '2019-02', 3), census(CE, '2019-03', 5)]);
  assert.deepEqual(plan.keys, [CE, PE], 'sorted, so one selection is one URL');
  assert.equal(plan.timeframe, '1s');
  assert.equal(plan.from, '2019-02');
  assert.equal(plan.to, '2019-03');
  assert.equal(plan.series.get(CE)?.rows, 8, 'census rows summed per series');
  assert.equal(plan.why, null);
  // A DAY WINDOW NARROWS THE MONTHS ASKED FOR.
  const narrowed = seriesPlan([census(CE, '2019-01', 1), census(CE, '2019-03', 1)], '2019-02-10', '2019-02-20');
  assert.deepEqual([narrowed.from, narrowed.to], ['2019-02', '2019-02']);
  assert.match(seriesPlan([census(CE, '2019-01', 1)], '2020-01-01', '2020-01-02').why ?? '', /outside the held months/);
  assert.match(seriesPlan([census(CE, '2019-01', 1, '1s'), census(CE, '2019-01', 1, '1min')]).why ?? '', /2 rungs/);
  const wide = Array.from({ length: MAX_SERIES + 1 }, (_, i) => census(`NSE-FNO-NIFTY-2019-03-28-${1000000 + i}-CE`, '2019-03', 1));
  assert.match(seriesPlan(wide).why ?? '', /^too_many_series: this selection reaches 1,025 series/);
  assert.equal(monthSpan('2018-09', '2026-09'), 97);
  assert.match(seriesPlan([census(CE, '2000-01', 1), census(CE, '2026-09', 1)]).why ?? '', /past the 240/);
  assert.deepEqual(seriesPlan([]).keys, []);
});

test('the window query names every series, bounds every month, and sends a minute’s To end as its :59 (DB-004, DB-B48)', () => {
  const plan = seriesPlan([census(CE, '2018-09', 1), census(PE, '2026-09', 1)]);
  const q = new URLSearchParams(windowQuery('gdfl', plan, {
    sort: 'v', desc: false, offset: 100, limit: 25, fromDay: '2019-03-20', toDay: '2019-03-20', fromTime: '09:15', toTime: '15:29'
  }));
  assert.deepEqual(q.getAll('key'), [CE, PE]);
  assert.equal(q.get('sort'), 'v');
  assert.equal(q.get('dir'), 'asc');
  assert.equal(q.get('from_day'), '2019-03-20');
  assert.equal(q.get('tod_from'), '09:15:00');
  assert.equal(q.get('tod_to'), '15:29:59', 'the whole 15:29 minute, all sixty one-second bars');
  assert.equal(q.get('offset'), '100');
  const found = new URLSearchParams(windowQuery('gdfl', plan, { at: 1_553_065_418, limit: 1 }));
  assert.equal(found.get('at'), '1553065418');
  assert.equal(found.get('offset'), null, 'Go to lets the server pick the page holding the instant');
  assert.equal(found.get('from_day'), null);
  assert.equal(found.get('tod_from'), null);
  assert.equal(found.get('dir'), 'desc', 'newest first unless asked otherwise');
  assert.equal(new URLSearchParams(windowQuery('gdfl', plan, { extremes: true })).get('extremes'), '1');
  assert.equal(todWire('10:00:07', 'to'), '10:00:07');
  assert.equal(todWire('nonsense', 'from'), '');
});

test('Go to reads a day and a time to the second, or a time on the day on screen (DB-005)', () => {
  assert.equal(istInstant('2019-03-20', '12:33:38'), 1_553_065_418);
  assert.equal(istInstant('2019-03-20', '12:33'), 1_553_065_380);
  assert.equal(istInstant('2019-02-30', '10:00'), null, 'no such day');
  assert.equal(istInstant('2019-03-20', '24:00'), null);
  assert.deepEqual(parseGoto('2019-03-20 12:33:38', ''), { at: 1_553_065_418, label: '2019-03-20 12:33:38' });
  assert.deepEqual(parseGoto('12:33:38', '2019-03-20'), { at: 1_553_065_418, label: '2019-03-20 12:33:38' });
  assert.deepEqual(parseGoto('2019-03-20T12:33', ''), { at: 1_553_065_380, label: '2019-03-20 12:33:00' });
  assert.match(/** @type {any} */ (parseGoto('12:33', '')).why, /names no day/);
  assert.match(/** @type {any} */ (parseGoto('noon', '2019-03-20')).why, /is not an instant/);
  assert.equal(pageOfIndex(0, 50), 1);
  assert.equal(pageOfIndex(49, 50), 1);
  assert.equal(pageOfIndex(50, 50), 2);
  assert.equal(pageOfIndex(399_950, 50), 8_000);
});

test('the census is compared with the files by series, and a lag is named without blocking a bar (DB-G02, DB-G61)', () => {
  const plan = seriesPlan([census(CE, '2019-11', 60_070), census(PE, '2019-11', 9)]);
  assert.deepEqual(censusBehind([{ key: CE, total: 192_062 }, { key: PE, total: 9 }], plan.series), [
    { key: CE, file: 192_062, census: 60_070 }
  ]);
  assert.deepEqual(censusBehind([], plan.series), []);
});

test('a locally ordered column puts unknowns last in both directions and breaks ties the same way each time', () => {
  const rows = [
    { chg: null, ts: 1, rk: 'a' },
    { chg: 5, ts: 2, rk: 'b' },
    { chg: -5, ts: 3, rk: 'c' },
    { chg: 5, ts: 0, rk: 'd' }
  ];
  assert.deepEqual([...rows].sort(localComparator('chg', true)).map((r) => r.rk), ['d', 'b', 'c', 'a']);
  assert.deepEqual([...rows].sort(localComparator('chg', false)).map((r) => r.rk), ['c', 'd', 'b', 'a']);
  assert.equal(rowKey(CE, 12), `${CE}|12`);
});

test('the bar CSV leads with the feed, carries the option analytics, and is named for the rows it holds (DB-047, DB-G101)', () => {
  const labels = BAR_CSV_COLS.map((c) => c.label);
  assert.equal(labels[0], 'feed');
  for (const want of ['spot_paisa', 'moneyness', 'intrinsic_paisa', 'extrinsic_paisa', 'iv', 'delta', 'gamma', 'theta', 'vega', 'rho', 'greeks_why', 'open_interest_why', 'volume_why']) {
    assert.ok(labels.includes(want), `${want} is not a column`);
  }
  const option = {
    instrument: CE, month: '2019-03', tf: '1s', ts: 1_553_065_418_000, day: '2019-03-20',
    o: 1111, h: 1122, l: 1100, c: 1115, vol: 375, vWhy: null, chg: null, chgWhy: 'no_earlier_stored_bar',
    oi: 750, oiWhy: null, oichg: 12, oichgWhy: null, exp: '2019-03-28', dte: 8, side: 'CE', strike: 1_150_000
  };
  const index = { ...option, instrument: 'NSE-INDEX-NIFTY', vol: 0, vWhy: 'index_no_volume', oi: null, oiWhy: 'not_a_derivative', exp: null, dte: null, side: null, strike: null };
  const text = barCsvText([option, index], (r) => ({
    feed: 'gdfl',
    oc: r.side ? { mny: 'OTM+1', mnyWhy: null, intr: 0, extr: 1115, why: null, spot: 1_140_000 } : null,
    gc: r.side ? { iv: 0.25, delta: 0.1, gamma: 0.001, theta: -1.5, vega: 2.25, rho: 0.5, why: null } : null
  }));
  const [head, one, two] = text.split('\r\n');
  assert.ok(text.endsWith('\r\n'));
  const cell = (/** @type {string} */ line, /** @type {string} */ label) => line.split(',')[labels.indexOf(label)];
  assert.equal(cell(head, 'feed'), 'feed');
  assert.equal(cell(one, 'feed'), 'gdfl');
  assert.equal(cell(one, 'ist_time'), '12:33:38');
  assert.equal(cell(one, 'spot_paisa'), '1140000');
  assert.equal(cell(one, 'moneyness'), 'OTM+1');
  assert.equal(cell(one, 'iv'), '0.25');
  assert.equal(cell(one, 'chg_bps'), '');
  assert.equal(cell(one, 'chg_why'), 'no_earlier_stored_bar');
  assert.equal(cell(two, 'volume_shares'), '', 'an index volume is not written as a measurement');
  assert.equal(cell(two, 'volume_why'), 'index_no_volume');
  assert.equal(cell(two, 'open_interest_shares'), '');
  assert.equal(cell(two, 'open_interest_why'), 'not_a_derivative');
  assert.equal(cell(two, 'iv'), '');
  assert.equal(barCsvName('gdfl', '1s', 'NIFTY', [option, { ...option, day: '2019-03-21' }]), 'brutex-db-bars_gdfl_1s_NIFTY_2019-03-20_2019-03-21_2rows.csv');
  assert.equal(barCsvName('zerodha', '1day', '', []), 'brutex-db-bars_zerodha_1day_selection_none_none_0rows.csv');
  assert.equal(istClock(1_553_065_418_000), '12:33:38');
});

test('/db reads bars through one window request a page and never the whole-month /bars.json (DB-063, DB-N16, DB-A29, DB-B49)', () => {
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  // THE CODE, NOT THE PROSE: the comments still tell the story of /bars.json.
  const script = grid.slice(0, grid.indexOf('</script>')).replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '');
  assert.doesNotMatch(script, /\/bars\.json/);
  assert.doesNotMatch(script, /extremes: '1'/);
  assert.doesNotMatch(script, /pooled\(/, 'no fan-out of one request per series or file');
  assert.match(script, /readWindowPage\(windowQuery\(feed, p, \{ \.\.\.b, sort: wire, desc, offset, limit: size \}\), size, signal\)/);
  // NOTHING IS READ BEFORE THE FIT SETTLES (DB-N02), AND PAGING DOES NOT
  // RE-MEASURE IT (DB-N11).
  assert.match(script, /const ready = db\.sizeMode !== 'fit' \|\| \(db\.fitLatched && db\.pageSize === db\.rowsThatFit\);/);
  assert.doesNotMatch(script, /void db\.barPlanKeys/);
  // A PAGE-SIZE CHANGE KEEPS THE READER'S ROW ON SCREEN (DB-N06, DB-N12).
  assert.match(script, /anchorRow = anchor && pageTotal > 0 \? firstRow : -1;/);
  assert.match(script, /box\.scrollTop = Math\.max\(0, tr\.offsetTop/);
});
