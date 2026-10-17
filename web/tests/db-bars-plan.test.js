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
  pageOfIndex, censusBehind, localComparator, rowKey, MAX_SERIES, monthSpan
} from '../src/lib/db/bars-plan.js';
import { parseGoto } from '../src/lib/db/goto.js';
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
  assert.deepEqual(parseGoto('2019-03-20 12:33:38', ''), { at: 1_553_065_418, label: '2019-03-20 12:33:38', minute: false });
  assert.deepEqual(parseGoto('12:33:38', '2019-03-20'), { at: 1_553_065_418, label: '2019-03-20 12:33:38', minute: false });
  assert.deepEqual(parseGoto('2019-03-20T12:33', ''), { at: 1_553_065_380, label: '2019-03-20 12:33:00', minute: true });
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
  assert.match(script, /windowQuery\(feed, p, \{ \.\.\.b, sort: wire, desc, offset: \(asked - 1\) \* size, limit: size \}\)/);
  assert.match(script, /const answer = await readWindowPage\(query, size, signal, asking\(seen\)\);/);
  // NOTHING IS READ BEFORE THE FIT SETTLES (DB-N02), AND PAGING DOES NOT
  // RE-MEASURE IT (DB-N11).
  assert.match(script, /const ready = !db\.linkDaysPending && \(db\.sizeMode !== 'fit' \|\| \(db\.fitLatched && db\.pageSize === db\.rowsThatFit\)\);/);
  assert.doesNotMatch(script, /void db\.barPlanKeys/);
  // A PAGE-SIZE CHANGE KEEPS THE READER'S ROW ON SCREEN (DB-N06, DB-N12).
  assert.match(script, /anchorRow = anchor && pageTotal > 0 \? firstRow : -1;/);
  assert.match(script, /box\.scrollTop = Math\.max\(0, tr\.offsetTop/);
});

test('the plan, the wire and the row helpers answer every edge in words or a sentinel, never a throw (R3-03)', async () => {
  const { paisaText, daysBetween } = await import('../src/lib/db/bars-plan.js');
  const { barWhyText, BAR_WHY } = await import('../src/lib/db/notes-bars.js');
  // A month that is not a month spans nothing; a series with no count holds 0.
  assert.equal(monthSpan('2019-xx', '2019-03'), 0);
  assert.equal(monthSpan('2019-04', '2019-03'), 0);
  const uncounted = seriesPlan([{ instrument: CE, month: '2019-03', timeframe: '1s' }, { instrument: CE, month: '2019-04', timeframe: '1s' }]);
  assert.equal(uncounted.series.get(CE)?.rows, 0);
  // Absent inputs are no bound, no instant and no lag.
  assert.equal(todWire(/** @type {any} */ (undefined), 'from'), '');
  assert.equal(istInstant(/** @type {any} */ (undefined), '10:00'), null);
  assert.equal(istInstant('2019-03-20', /** @type {any} */ (null)), null);
  assert.deepEqual(censusBehind(/** @type {any} */ (null), new Map()), []);
  // The local comparator: unknowns last both ways, text by text, ties by time then key.
  const asc = localComparator('side', false);
  const desc = localComparator('side', true);
  assert.equal(asc({ side: null, ts: 1, rk: 'a' }, { side: 'CE', ts: 1, rk: 'b' }), 1);
  assert.equal(desc({ side: 'PE', ts: 1, rk: 'a' }, { side: undefined, ts: 1, rk: 'b' }), -1);
  assert.equal(asc({ side: 'CE', ts: 1, rk: 'a' }, { side: 'PE', ts: 1, rk: 'b' }), -1);
  assert.equal(asc({ side: 'PE', ts: 1, rk: 'a' }, { side: 'CE', ts: 1, rk: 'b' }), 1);
  assert.equal(desc({ side: 'CE', ts: 1, rk: 'a' }, { side: 'PE', ts: 1, rk: 'b' }), 1);
  assert.equal(asc({ side: 'CE', ts: 2, rk: 'a' }, { side: 'CE', ts: 1, rk: 'b' }), 1, 'a tie falls to time');
  assert.equal(desc({ side: 'CE', ts: 1, rk: 'a' }, { side: 'CE', ts: 2, rk: 'b' }), -1, 'and does not invert');
  assert.equal(asc({ side: 'CE', ts: 1, rk: 'b' }, { side: 'CE', ts: 1, rk: 'a' }), 1, 'then to the key');
  assert.equal(asc({ side: 'CE', ts: 1, rk: 'a' }, { side: 'CE', ts: 1, rk: 'b' }), -1);
  assert.equal(asc({ side: 'CE', ts: 1, rk: 'a' }, { side: 'CE', ts: 1, rk: 'a' }), 0);
  assert.equal(asc({ side: null, ts: 1, rk: 'a' }, { side: null, ts: 1, rk: 'a' }), 0);
  assert.equal(localComparator('chg', false)({ chg: 5, ts: 1, rk: 'a' }, { chg: 5, ts: 1, rk: 'a' }), 0);
  // Paisa: the null is a dash, the minus a real minus, the two decimals always.
  assert.equal(paisaText(null), '-');
  assert.equal(paisaText(undefined), '-');
  assert.equal(paisaText(-1_234_505), '−12,345.05');
  assert.equal(paisaText(100), '1.00');
  // Whole days between two ISO days; a day that will not parse is null.
  assert.equal(daysBetween('2019-03-20', '2019-03-28'), 8);
  assert.equal(daysBetween('2019-03-28', '2019-03-20'), -8);
  assert.equal(daysBetween('nope', '2019-03-20'), null);
  assert.equal(daysBetween('2019-03-20', 'nope'), null);
  // Every code has its sentence; no code is null; an unknown code is a defect said.
  assert.equal(barWhyText('oi_null'), BAR_WHY.oi_null);
  assert.equal(barWhyText(null), 'unknown, and this page did not record why - which is a defect here, not a fact about the store.');
  assert.equal(barWhyText('no_such_code'), barWhyText(null));
});

test('the grid’s columns, groups and sentences are pinned, so a change to any of them is a deliberate one (R3-03)', async () => {
  const { createHash } = await import('node:crypto');
  const nb = await import('../src/lib/db/notes-bars.js');
  const bp = await import('../src/lib/db/bars-plan.js');
  // THE SCHEMA: every column, in order, which are numbers, which have no source or no order.
  assert.deepEqual(nb.BAR_COLS.map((c) => c.key), ['ts', 'tod', 'tf', 'o', 'h', 'l', 'c', 'vol', 'premkt', 'chg', 'oi', 'oichg', 'exp', 'dte', 'side', 'strike', 'mny', 'intr', 'extr', 'iv', 'delta', 'gamma', 'theta', 'vega', 'rho']);
  assert.deepEqual(nb.BAR_COLS.map((c) => c.label), ['Date', 'Time', 'TF', 'Open', 'High', 'Low', 'Close', 'Volume', 'Pre-mkt %', 'Change %', 'Open interest', 'OI chg %', 'Expiry', 'Days to exp', 'Type', 'Strike', 'Moneyness', 'Intrinsic', 'Extrinsic', 'IV %', 'Delta', 'Gamma', 'Theta', 'Vega', 'Rho']);
  assert.deepEqual(nb.BAR_COLS.filter((c) => c.num === true).map((c) => c.key), ['o', 'h', 'l', 'c', 'vol', 'premkt', 'chg', 'oi', 'oichg', 'dte', 'strike', 'mny', 'intr', 'extr', 'iv', 'delta', 'gamma', 'theta', 'vega', 'rho']);
  assert.deepEqual(nb.BAR_COLS.filter((c) => c.none).map((c) => c.key), ['premkt']);
  assert.deepEqual(nb.BAR_COLS.filter((c) => c.nosort).map((c) => c.key), ['mny', 'intr', 'extr', 'iv', 'delta', 'gamma', 'theta', 'vega', 'rho']);
  for (const c of nb.BAR_COLS) {
    assert.equal(nb.BAR_META[c.key], c, `${c.key} reaches its own column`);
    assert.ok(c.label.length > 0 && (c.none ?? c.why).length > 20, `${c.key} says what it is`);
  }
  assert.equal(Object.keys(nb.BAR_META).length, 25);
  assert.equal(nb.BAR_SOURCED, 24);
  assert.equal(nb.BAR_UNSOURCED, 1);
  assert.deepEqual([...nb.OPTION_COLS], ['side', 'strike', 'mny', 'intr', 'extr', 'iv', 'delta', 'gamma', 'theta', 'vega', 'rho']);
  assert.deepEqual([...nb.FUTURE_COLS], ['exp', 'dte']);
  assert.deepEqual(nb.BAR_GROUPS, [
    { id: 'when', label: 'When', keys: ['ts', 'tod', 'tf'] },
    { id: 'price', label: 'Price', keys: ['o', 'h', 'l', 'c'] },
    { id: 'flow', label: 'Flow', keys: ['vol', 'premkt', 'chg', 'oi', 'oichg'] },
    { id: 'contract', label: 'Contract', keys: ['exp', 'dte', 'side', 'strike', 'mny', 'intr', 'extr'] },
    { id: 'greeks', label: 'Greeks', keys: ['iv', 'delta', 'gamma', 'theta', 'vega', 'rho'] }
  ]);
  assert.deepEqual(nb.BAR_UNGROUPED, []);
  assert.deepEqual(nb.ungrouped([{ key: 'a' }, { key: 'b' }, { key: 'c' }], [{ keys: ['b'] }]), ['a', 'c']);
  // THE PROSE: every sentence a column, a reason code or a refused sort says,
  // byte for byte. A changed sentence changes this digest; change both together.
  const prose = JSON.stringify({ cols: nb.BAR_COLS, why: nb.BAR_WHY, premkt: nb.NO_PREMKT, spot: nb.NO_SPOT_SORT, greek: nb.NO_GREEK_SORT, vol: nb.NO_VOLUME_SORT, oi: nb.NO_OI_SORT });
  assert.equal(createHash('sha256').update(prose).digest('hex'), 'd4dc48c6537d0b50b7882ab99d887827714ccc15a5429c2d7d37b090a8caa2d0');
  assert.equal(Object.keys(nb.BAR_WHY).length, 19);
  // The wire's own spelling of every column the server orders.
  assert.deepEqual({ ...bp.WIRE_SORT }, { ts: 'ts', tod: 'tod', o: 'o', h: 'h', l: 'l', c: 'c', vol: 'v', oi: 'oi' });
  assert.equal(bp.wireSort('constructor'), null, 'no inherited name is a column');
  assert.deepEqual(bp.SKELETON, Array.from({ length: 24 }, (_, i) => i));
  assert.equal(bp.spanFloor, '2015-01-01');
  assert.equal(bp.paisaText(0), '0.00');
});

test('the plan, the query and the instant at every boundary (R3-03)', () => {
  // Months: a span that runs backwards is none; a held range in any order is sorted.
  assert.equal(monthSpan('2019-05', '2019-03'), 0);
  assert.equal(monthSpan('2019-03', '2019-03'), 1);
  const plan = seriesPlan([census(CE, '2019-03', 1), census(CE, '2019-01', 1), census(CE, '2019-04', 1), census(CE, '2019-02', 1)]);
  assert.deepEqual([plan.from, plan.to, plan.why], ['2019-01', '2019-04', null]);
  // A window narrows, never widens; an empty From or To keeps the held end.
  assert.deepEqual([seriesPlan([census(CE, '2019-03', 1)], '2019-01-05', '').from, seriesPlan([census(CE, '2019-03', 1)], '', '2019-06-01').to], ['2019-03', '2019-03']);
  assert.deepEqual([seriesPlan([census(CE, '2019-03', 1), census(CE, '2019-05', 1)], '2019-04-02', '2019-04-28').from, seriesPlan([census(CE, '2019-03', 1), census(CE, '2019-05', 1)], '2019-04-02', '2019-04-28').to], ['2019-04', '2019-04']);
  // Nothing matched is no refusal, whatever the window.
  assert.equal(seriesPlan([], '2019-01-01', '2019-01-02').why, null);
  assert.equal(seriesPlan([]).timeframe, '');
  // Two rungs: no rung, and the refusal names both.
  const two = seriesPlan([census(CE, '2019-01', 1, '1s'), census(PE, '2019-01', 1, '1min')]);
  assert.equal(two.timeframe, '');
  assert.equal(two.why, 'this selection reaches 2 rungs (1s, 1min); a page is read at one rung, so choose one');
  // Exactly the most series and the most months one request reads are read.
  const series = Array.from({ length: MAX_SERIES }, (_, i) => census(`NSE-FNO-NIFTY-2019-03-28-${1000000 + i}-CE`, '2019-03', 1));
  assert.equal(seriesPlan(series).why, null);
  assert.equal(seriesPlan([census(CE, '2000-01', 1), census(CE, '2019-12', 1)]).why, null, '240 months');
  assert.equal(seriesPlan([census(CE, '2000-01', 1), census(CE, '2020-01', 1)]).why, 'the months 2000-01 to 2020-01 span 241, past the 240 one window request reads — narrow the dates');
  // A window outside the held months, said with the end it was given.
  assert.equal(seriesPlan([census(CE, '2019-01', 1)], '2020-01-01', '').why, 'the date window 2020-01-01 to … lies outside the held months 2019-01 to 2019-01, so there is nothing to read');
  assert.equal(seriesPlan([census(CE, '2019-05', 1)], '', '2019-02-01').why, 'the date window … to 2019-02-01 lies outside the held months 2019-05 to 2019-05, so there is nothing to read');
  // The query: every name, absent bounds absent, and no instant when none is given.
  const one = seriesPlan([census(CE, '2019-03', 1)]);
  assert.equal(windowQuery('gdfl', one, {}), `feed=gdfl&timeframe=1s&from=2019-03&to=2019-03&key=${CE}&sort=ts&dir=desc&offset=0&limit=50&extremes=0`);
  assert.equal(windowQuery('gdfl', one, { at: null, offset: 8, limit: 8, desc: false, sort: 'c', toDay: '2019-03-09', fromTime: '09:15', toTime: '10:00:05' }),
    `feed=gdfl&timeframe=1s&from=2019-03&to=2019-03&key=${CE}&sort=c&dir=asc&offset=8&limit=8&to_day=2019-03-09&tod_from=09%3A15%3A00&tod_to=10%3A00%3A05&extremes=0`);
  // A time bound: trimmed, anchored at both ends.
  assert.equal(todWire(' 10:00 ', 'to'), '10:00:59');
  assert.equal(todWire('10:00', 'from'), '10:00:00');
  assert.equal(todWire('x10:00:07', 'to'), '');
  assert.equal(todWire('10:00:07x', 'to'), '');
  // An instant: anchored, every part bounded, a rolled-over day refused.
  for (const [day, time] of [['x2019-03-20', '10:00'], ['2019-03-20x', '10:00'], ['2019-03-20', 'x10:00'], ['2019-03-20', '10:00x'], ['2019-03-20', '10:60'], ['2019-03-20', '10:00:60'], ['2019-02-29', '10:00']]) {
    assert.equal(istInstant(day, time), null, `${day} ${time}`);
  }
  assert.equal(istInstant('2020-02-29', '23:59:59'), 1_583_000_999);
  assert.equal(istInstant('2019-03-20', '00:00'), 1_553_020_200);
  // The census against the files: an unknown key, a count that is not a number, an equal count.
  assert.deepEqual(censusBehind([{ key: 'NSE-INDEX-OTHER', total: 5 }, { key: CE, total: /** @type {any} */ (undefined) }, { key: CE, total: 1 }], one.series), []);
  assert.deepEqual(censusBehind([{ key: CE, total: 2 }], one.series), [{ key: CE, file: 2, census: 1 }]);
  // The comparator: an undefined is an unknown, and ordered last.
  assert.equal(localComparator('chg', false)({ chg: undefined, ts: 1, rk: 'a' }, { chg: 5, ts: 1, rk: 'b' }), 1);
  assert.equal(localComparator('chg', true)({ chg: 7, ts: 1, rk: 'a' }, { chg: 5, ts: 1, rk: 'b' }), -1);
  assert.equal(localComparator('chg', false)({ chg: 7, ts: 1, rk: 'a' }, { chg: 5, ts: 1, rk: 'b' }), 1);
});

test('an unknown on either side of a comparison is last, an empty plan names no month, and a day key is zero-padded (R3-03)', async () => {
  const { istDayKey, acrossDay } = await import('../src/lib/db/bars-plan.js');
  assert.equal(localComparator('chg', false)({ chg: 5, ts: 1, rk: 'b' }, { chg: undefined, ts: 1, rk: 'a' }), -1, 'not by the tie-break');
  assert.equal(localComparator('chg', true)({ chg: 5, ts: 1, rk: 'a' }, { chg: null, ts: 1, rk: 'b' }), -1);
  assert.deepEqual([seriesPlan([]).from, seriesPlan([]).to], ['', '']);
  assert.equal(istDayKey(Date.UTC(2019, 2, 4, 20, 0)), '2019-03-05', 'IST is ahead: 20:00 UTC on the 4th is the 5th');
  assert.equal(istDayKey(Date.UTC(2019, 8, 9, 0, 0)), '2019-09-09');
  // A row with no change is never marked, even straight after a day break.
  const rows = [
    { rk: 'a', key: CE, ts: 1, day: '2019-03-04', chg: 5 },
    { rk: 'b', key: CE, ts: 2, day: '2019-03-05', chg: null },
    { rk: 'c', key: CE, ts: 3, day: '2019-03-06', chg: undefined }
  ];
  assert.equal(acrossDay(rows, { timeNarrows: false, dayRung: false, edgeEarlier: false }).size, 0);
});
