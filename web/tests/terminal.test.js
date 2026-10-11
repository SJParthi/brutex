import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  QUOTE_CACHE, SPOT_CHUNK, STRIP_MAX, UNPARSED_SAMPLE, barHeader, behindSource, daySpan, fromBar, gated,
  monthSpan, moveFromBps, quoteFrom, quoteKey, quoteOn, quotesOn, servedIndices, spotsFor, stripOf, tabsAndRest,
  tabsFrom, timeframeOrder, timeframeSeconds, traffic, useTransport, windowUrl
} from '../src/lib/terminal.svelte.js';

// WP-60: /terminal. DB-G12, DB-G13, DB-G15, DB-G16, DB-G17, DB-G32, DB-G86.
// Generated shapes and invented figures only; no row here is market data.

const page = readFileSync(new URL('../src/routes/terminal/+page.svelte', import.meta.url), 'utf8');
const lib = readFileSync(new URL('../src/lib/terminal.svelte.js', import.meta.url), 'utf8');

/** An IST wall clock as unix seconds. */
const ist = (/** @type {number} */ y, /** @type {number} */ mo, /** @type {number} */ d, h = 0, mi = 0, s = 0) =>
  Date.UTC(y, mo - 1, d, h, mi, s) / 1000 - 19_800;
/** The same in the census's microseconds. */
const micros = (/** @type {number[]} */ ...a) => ist(.../** @type {[number, number, number]} */ (a)) * 1_000_000;

/** @param {any} body @param {number} [status] @returns {any} */
const reply = (body, status = 200) => ({ status, json: async () => body });
/** A window answer holding `bars`. @param {any[]} bars @param {any} [more] */
const windowOf = (bars, more = {}) => ({ total: bars.length, offset: 0, scanned: false, faults: null, bars, ...more });
/** One invented bar. */
const bar = (/** @type {number} */ t, /** @type {any} */ more = {}) =>
  ({ t, o: 77_701, h: 77_709, l: 77_700, c: 77_707, v: 31, v_why: null, oi: 4_100, chg: 13, chg_why: null, oichg: 250, oichg_why: null, ...more });

const NIFTY = 'NSE-INDEX-NIFTY';
const OPT = 'NSE-FNO-NIFTY-2099-11-28-1200000-CE';
const FUT = 'NSE-FNO-NIFTY-2099-11-28-FUT';

test('1s sorts below 1min, and a name that will not read is never the finest rung (DB-G32)', () => {
  assert.equal(timeframeSeconds('1s'), 1);
  assert.equal(timeframeSeconds('1min'), 60);
  assert.equal(timeframeSeconds('60min'), 3_600);
  assert.equal(timeframeSeconds('1day'), 86_400);
  for (const junk of ['', '0min', '01min', '1h', 'min', '1 s', '-1s', '1.5min', null, undefined, 60]) {
    assert.equal(timeframeSeconds(junk), null, String(junk));
    assert.equal(timeframeOrder(junk), Number.MAX_SAFE_INTEGER, String(junk));
  }
  const held = ['1day', '10min', '1s', '2min', '1min', '60min', 'weird'];
  assert.deepEqual([...held].sort((a, b) => timeframeOrder(a) - timeframeOrder(b)),
    ['1s', '1min', '2min', '10min', '60min', '1day', 'weird']);
  // The page orders its picker with this function and no private copy of it.
  assert.match(page, /timeframeOrder\(a\) - timeframeOrder\(b\)/);
  assert.doesNotMatch(page, /function timeframeOrder/);
});

test('a derived rung whose month ends before its source is marked behind, with the day it stopped (DB-G12)', () => {
  // 60min ends on the 4th at 15:15; 1min reaches the 30th at 15:29.
  const cells = [
    { month: '2099-08', timeframe: '1min', last: micros(2099, 8, 31, 15, 29) },
    { month: '2099-08', timeframe: '60min', last: micros(2099, 8, 31, 15, 15) },
    { month: '2099-09', timeframe: '1min', last: micros(2099, 9, 30, 15, 29) },
    { month: '2099-09', timeframe: '2min', last: micros(2099, 9, 30, 15, 28) },
    { month: '2099-09', timeframe: '60min', last: micros(2099, 9, 4, 15, 15) },
    { month: '2099-09', timeframe: '1day', last: micros(2099, 9, 30) }
  ];
  const behind = behindSource(cells, '60min', '2099-09');
  assert.ok(behind);
  assert.equal(behind.text, 'behind 1min (last 2099-09-04)');
  assert.equal(behind.source, '1min');
  assert.equal(behind.lastDay, '2099-09-04');
  assert.equal(behind.sourceDay, '2099-09-30');
  // The source is the FINEST rung held that month, not the nearest finer one.
  assert.equal(behindSource([...cells].reverse(), '60min', '2099-09')?.source, '1min');
  // A last bucket that still covers the source's last bar is not behind:
  // 15:15 + 60 min is after 15:29, 15:28 + 2 min is after 15:29, and the
  // day bar stamped at midnight covers that whole day.
  assert.equal(behindSource(cells, '60min', '2099-08'), null);
  assert.equal(behindSource(cells, '2min', '2099-09'), null);
  assert.equal(behindSource(cells, '1day', '2099-09'), null);
  // Exactly one bucket short is behind; one microsecond less is not.
  const edge = (/** @type {number} */ source) => behindSource([
    { month: '2099-09', timeframe: '1min', last: source },
    { month: '2099-09', timeframe: '60min', last: micros(2099, 9, 4, 14, 15) }
  ], '60min', '2099-09');
  assert.ok(edge(micros(2099, 9, 4, 15, 15)));
  assert.equal(edge(micros(2099, 9, 4, 15, 15) - 1), null);
  // The finest rung has no source to be behind; neither has a lone rung, an
  // unheld month, an unreadable name, a cell with no stamp or no cells at all.
  assert.equal(behindSource(cells, '1min', '2099-09'), null);
  assert.equal(behindSource([{ month: '2099-09', timeframe: '60min', last: 1 }], '60min', '2099-09'), null);
  assert.equal(behindSource(cells, '60min', '2099-10'), null);
  assert.equal(behindSource(cells, 'hourly', '2099-09'), null);
  assert.equal(behindSource([{ month: '2099-09', timeframe: '1min', last: null }, { month: '2099-09', timeframe: '60min', last: 5 }], '60min', '2099-09'), null);
  assert.equal(behindSource(undefined, '60min', '2099-09'), null);
  assert.equal(behindSource([], '60min', '2099-09'), null);
  // The page draws it on the grid row, the strip and the lead ticker.
  assert.equal((page.match(/= behindOf\(/g) ?? []).length, 3);
  assert.equal((page.match(/class="stale" title=\{behindWhy\(/g) ?? []).length, 3);
});

test('the strip starts NIFTY, BANKNIFTY, then the F&O index underlyings, in the order served (DB-G13)', () => {
  // The census's INDEX keys, sorted by name as the page holds them.
  const keys = ['BANKNIFTY', 'BHARATBOND-APR30', 'BHARATBOND-APR31', 'BHARATBOND-APR32', 'BHARATBOND-APR33', 'INDIAVIX',
    'NIFTY', 'NIFTY100', 'NIFTYFINSERVICE', 'NIFTYIT', 'NIFTYMIDSELECT', 'NIFTYNEXT50'].map((s) => `NSE-INDEX-${s}`);
  const served = [['NIFTY', true], ['BANKNIFTY', true], ['NIFTYFINSERVICE', true], ['NIFTYMIDSELECT', true],
    ['NIFTYNEXT50', true], ['NIFTYIT', false], ['NIFTYFPI150', false], ['INDIAVIX', false]]
    .map(([s, fno]) => ({ key: `NSE-INDEX-${s}`, fno: /** @type {boolean} */ (fno) }));
  const all = () => true;
  assert.deepEqual(stripOf(keys, all, served),
    ['NIFTY', 'BANKNIFTY', 'NIFTYFINSERVICE', 'NIFTYMIDSELECT', 'NIFTYNEXT50', 'NIFTYIT'].map((s) => `NSE-INDEX-${s}`));
  // What the page drew before: the first six names in alphabetical order, NIFTY absent.
  assert.equal(keys.filter(all).slice(0, 6).includes(NIFTY), false);
  // An F&O index underlying appears even when the chosen cell is absent (its
  // cell then names the absence); any other index must hold the cell.
  const onlyVix = (/** @type {string} */ k) => k === 'NSE-INDEX-INDIAVIX' || k === 'NSE-INDEX-NIFTY100';
  assert.deepEqual(stripOf(keys, onlyVix, served, 8),
    ['NIFTY', 'BANKNIFTY', 'NIFTYFINSERVICE', 'NIFTYMIDSELECT', 'NIFTYNEXT50', 'INDIAVIX', 'NIFTY100'].map((s) => `NSE-INDEX-${s}`));
  // A served name the census does not hold is not invented into the strip.
  assert.deepEqual(stripOf([NIFTY], all, served), [NIFTY]);
  // With the served order unread the strip is name order, and never longer than the cap.
  assert.deepEqual(stripOf(keys, all, []), keys.slice(0, STRIP_MAX));
  assert.deepEqual(stripOf([], all, served), []);
  assert.deepEqual(stripOf(keys, all, [...served, ...served]).length, STRIP_MAX);
  assert.equal(new Set(stripOf(keys, all, [...served, ...served], 99)).size, keys.length);
  assert.match(page, /stripOf\(indexKeys, holdsChosenCell, stripServed\)/);
  assert.doesNotMatch(page, /holdsChosenCell\(k\)\)\.slice\(0, 6\)/);
});

test('the strip order is read from /aliases.json once, and a failed read is an empty order beside its reason', async () => {
  /** @type {string[]} */ const asked = [];
  useTransport(/** @type {any} */ (async (/** @type {string} */ url) => {
    asked.push(url);
    return reply({ index: [{ fno: 'NIFTY', key: NIFTY, in_fno_universe: true }, { key: 'NSE-INDEX-INDIAVIX', in_fno_universe: false, reference_only: true }, { fno: 'X' }] });
  }));
  const [a, b] = await Promise.all([servedIndices(), servedIndices()]);
  assert.deepEqual(a, { served: [{ key: NIFTY, fno: true }, { key: 'NSE-INDEX-INDIAVIX', fno: false }], why: null });
  assert.equal(a, b);
  assert.deepEqual(asked, ['/aliases.json']);
  useTransport(/** @type {any} */ (async () => reply({ error: 'aliases unavailable: test' }, 503)));
  assert.deepEqual(await servedIndices(), { served: [], why: 'aliases unavailable: test' });
  useTransport(/** @type {any} */ (async () => reply(null, 502)));
  assert.match(String((await servedIndices()).why), /HTTP 502 with no index list/);
  // A failure is not kept: the next call asks again, and a success is then held for good.
  let calls = 0;
  useTransport(/** @type {any} */ (async () => {
    calls += 1;
    if (calls === 1) throw new Error('offline');
    return reply({ index: [{ key: NIFTY, in_fno_universe: true }] });
  }));
  assert.deepEqual(await servedIndices(), { served: [], why: 'offline' });
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual((await servedIndices()).served, [{ key: NIFTY, fno: true }]);
  await servedIndices();
  assert.equal(calls, 2);
  // The page says so instead of silently falling back.
  assert.match(page, /stripWhy/);
});

test('fromBar keeps OI change and its reason, and an index volume of 0 is not a traded volume (DB-G15)', () => {
  const q = fromBar(OPT, bar(100, { oichg: -1_250, oichg_why: null }));
  assert.equal(q.oichg, -1_250);
  assert.equal(q.oichgWhy, null);
  assert.equal(q.volume, 31);
  const none = fromBar(OPT, bar(100, { oichg: null, oichg_why: 'previous_oi_zero', oi: null }));
  assert.equal(none.oichg, null);
  assert.equal(none.oichgWhy, 'previous_oi_zero');
  assert.equal(none.oi, null);
  const index = fromBar(NIFTY, bar(100, { v: 0, v_why: 'index_no_volume', oichg: null, oichg_why: 'not_a_derivative' }));
  assert.equal(index.volume, null);
  assert.equal(index.volumeWhy, 'index_no_volume');
  assert.equal(index.oichgWhy, 'not_a_derivative');
  // A zero is a zero.
  assert.equal(fromBar(OPT, bar(100, { oichg: 0, v: 0, oi: 0 })).oichg, 0);
  assert.equal(fromBar(OPT, bar(100, { oichg: 0, v: 0, oi: 0 })).volume, 0);
  assert.equal(fromBar(OPT, bar(100, { oichg: 0, v: 0, oi: 0 })).oi, 0);
});

test('a change in units is derived from the value and its basis points, and refused where it cannot be', () => {
  assert.equal(moveFromBps(11_000, 1_000), 1_000);
  assert.equal(moveFromBps(9_000, -1_000), -1_000);
  assert.equal(moveFromBps(5_000, 0), 0);
  assert.equal(moveFromBps(0, -10_000), null);
  assert.equal(moveFromBps(700, -10_000), null);
  assert.equal(moveFromBps(null, 5), null);
  assert.equal(moveFromBps(5, null), null);
  assert.equal(moveFromBps(undefined, undefined), null);
});

test('the OI and spot columns have a source, and the OI chips rank on it (DB-G15)', () => {
  assert.doesNotMatch(page, /carries a change field for price and none for OI/);
  assert.doesNotMatch(page, /does not join them to the underlying/);
  assert.match(page, /\{ h: 'OI Change %', src: 'oichg'/);
  assert.match(page, /\{ h: 'OI Change', src: 'oichgAbs'/);
  assert.match(page, /\{ h: 'Spot Price', src: 'spot'/);
  assert.match(page, /'OI Gainers': \{ by: 'oichg', dir: 'desc', keep: \(v\) => v > 0 \}/);
  assert.match(page, /'OI Losers': \{ by: 'oichg', dir: 'asc', keep: \(v\) => v < 0 \}/);
  assert.match(page, /spotsFor\(/);
});

test('spots are asked once per underlying and month, at most 400 instants a request, through the page pool (DB-G15)', async () => {
  assert.equal(SPOT_CHUNK, 400);
  /** @type {URLSearchParams[]} */ const asked = [];
  useTransport(/** @type {any} */ (async (/** @type {string} */ url) => {
    const q = new URL(url, 'http://x').searchParams;
    asked.push(q);
    const t = String(q.get('t')).split(',').map(Number);
    return reply({ asof: q.get('asof') ?? 'exact', segment: 'INDEX', key: 'NIFTY', faults: null,
      spots: t.map((s) => (s % 2 ? { t: s, close: 1_000_000 + s, spot_t: s, spot_age_s: 0, why: null }
        : { t: s, close: null, spot_t: null, spot_age_s: null, why: 'no_spot_bar' })) });
  }));
  const base = ist(2099, 11, 28, 9, 15);
  // 401 option contracts of one underlying, each at its own second, one future, one index and one unanswered row.
  const quotes = Array.from({ length: 401 }, (_, i) => fromBar(`NSE-FNO-NIFTY-2099-11-28-${(1000 + i) * 100}-CE`, bar(base + i)));
  quotes.push(fromBar(FUT, bar(base + 1)), fromBar(NIFTY, bar(base + 1)));
  quotes.push({ ...fromBar(OPT, bar(base)), at: null, why: 'nothing' });
  const spots = await spotsFor('gdfl', quotes, '1s', new AbortController().signal);
  assert.equal(asked.length, 2, 'one request per 400 instants');
  assert.deepEqual(asked.map((q) => String(q.get('t')).split(',').length), [400, 1]);
  for (const q of asked) {
    assert.equal(q.get('feed'), 'gdfl');
    assert.equal(q.get('exchange'), 'NSE');
    assert.equal(q.get('symbol'), 'NIFTY');
    assert.equal(q.get('timeframe'), '1s');
    assert.equal(q.get('month'), '2099-11');
    assert.equal(q.get('asof'), null, 'exact is the default (D-2807)');
  }
  assert.equal(traffic.sent, 2, 'both went through the page pool');
  assert.equal(spots.size, 402, '401 options and the future; the index and the unanswered row are not asked');
  assert.equal(spots.get(quotes[1].key)?.close, 1_000_000 + base + 1);
  assert.equal(spots.get(quotes[0].key)?.close, null);
  assert.equal(spots.get(quotes[0].key)?.why, 'no_spot_bar');
  assert.equal(spots.get(FUT)?.close, 1_000_000 + base + 1);
  assert.equal(spots.has(NIFTY), false);
  // Opted in, the request says so and the answer keeps the spot's age.
  asked.length = 0;
  useTransport(/** @type {any} */ (async (/** @type {string} */ url) => {
    const q = new URL(url, 'http://x').searchParams;
    asked.push(q);
    return reply({ asof: 'prev', segment: 'INDEX', key: 'NIFTY', faults: null,
      spots: [{ t: base, close: 990_000, spot_t: base - 7, spot_age_s: 7, why: null }] });
  }));
  const prev = await spotsFor('gdfl', [fromBar(OPT, bar(base))], '1s', new AbortController().signal, 'prev');
  assert.equal(asked[0].get('asof'), 'prev');
  assert.deepEqual([prev.get(OPT)?.close, prev.get(OPT)?.age, prev.get(OPT)?.asof], [990_000, 7, 'prev']);
  // No contract on screen is no request at all.
  asked.length = 0;
  assert.equal((await spotsFor('gdfl', [fromBar(NIFTY, bar(base))], '1s', new AbortController().signal)).size, 0);
  assert.equal(asked.length, 0);
  // A failed spot request is a named absence per row, never a thrown batch.
  useTransport(/** @type {any} */ (async () => reply({ error: 'spot store unavailable: test' }, 503)));
  const failed = await spotsFor('gdfl', [fromBar(OPT, bar(base))], '1s', new AbortController().signal);
  assert.equal(failed.get(OPT)?.why, 'spot_request_failed');
  assert.match(String(failed.get(OPT)?.detail), /spot store unavailable: test/);
});

test('a day is asked by its own bounds and a time by at=, one bar each, never a month read (DB-G16, DB-G86)', () => {
  const month = /** @type {{url: string, at: number|null}} */ (windowUrl('gdfl', NIFTY, '1s', '2099-10'));
  const mq = new URL(month.url, 'http://x').searchParams;
  assert.deepEqual([mq.get('from'), mq.get('to'), mq.get('sort'), mq.get('dir'), mq.get('limit'), mq.get('offset')],
    ['2099-10', '2099-10', 'ts', 'desc', '1', '0']);
  assert.equal(mq.get('from_day'), null);
  assert.equal(mq.get('at'), null);
  assert.equal(month.at, null);

  const day = /** @type {{url: string, at: number|null}} */ (windowUrl('gdfl', NIFTY, '1s', '2099-10', '2099-10-15'));
  const dq = new URL(day.url, 'http://x').searchParams;
  assert.deepEqual([dq.get('from_day'), dq.get('to_day'), dq.get('limit'), dq.get('dir'), dq.get('at')],
    ['2099-10-15', '2099-10-15', '1', 'desc', null]);

  const second = /** @type {{url: string, at: number|null}} */ (windowUrl('gdfl', OPT, '1s', '2099-10', '2099-10-15', '10:15:07'));
  const sq = new URL(second.url, 'http://x').searchParams;
  assert.equal(second.at, ist(2099, 10, 15, 10, 15, 7));
  assert.equal(sq.get('at'), String(ist(2099, 10, 15, 10, 15, 7)));
  assert.equal(sq.get('offset'), null, 'the server picks the page that holds at=');
  assert.deepEqual([sq.get('segment'), sq.get('symbol'), sq.get('contract')], ['FNO', 'NIFTY', '2099-11-28-1200000-CE']);
  assert.equal(/** @type {any} */ (windowUrl('zerodha', NIFTY, '1min', '2099-10', '2099-10-15', '10:15')).at, ist(2099, 10, 15, 10, 15));

  // Garbage is refused by name before any request.
  for (const [d, t, says] of [['2099-11-15', '', /not a day of 2099-10/], ['15/10/2099', '', /not a day of 2099-10/],
    ['2099-10-15', '25:00', /not an IST time of day/], ['2099-10-15', '10:15:99', /not an IST time of day/],
    ['2099-10-15', 'noon', /not an IST time of day/], ['2099-10-32', '10:15', /not an IST time of day|not a day/],
    ['', '10:15', /A time needs a day/]]) {
    const refused = /** @type {any} */ (windowUrl('gdfl', NIFTY, '1s', '2099-10', String(d), String(t)));
    assert.match(String(refused.why), /** @type {RegExp} */ (says), `${d} ${t}`);
    assert.equal(refused.url, undefined);
  }
  assert.match(String(/** @type {any} */ (windowUrl('gdfl', 'garbage', '1s', '2099-10')).why), /not a series key/);

  // The 512-bar month read and everything built on it are gone.
  for (const source of [page, lib]) {
    assert.doesNotMatch(source, /DAY_BUDGET/);
    assert.doesNotMatch(source, /monthBars|daysIn\(|timesOn\(/);
    assert.doesNotMatch(source, /[Cc]oarser timeframe/);
  }
});

test('one window answer becomes a quote, or one named absence: empty, damaged, before the first bar, not a window', () => {
  const asked = { timeframe: '1s', month: '2099-10', day: '2099-10-15', time: '10:15:07', at: ist(2099, 10, 15, 10, 15, 7) };
  // The last bar at or before the instant: an exact hit, then one 5 s earlier.
  const exact = quoteFrom(OPT, windowOf([bar(asked.at)]), 200, asked);
  assert.deepEqual([exact.why, exact.close, exact.at, exact.asked, exact.lag], [null, 77_707, asked.at, asked.at, 0]);
  const earlier = quoteFrom(OPT, windowOf([bar(asked.at - 5)]), 200, asked);
  assert.deepEqual([earlier.why, earlier.at, earlier.lag], [null, asked.at - 5, 5]);
  // No bar at or before it: the window's page holds the day's FIRST bar, which is later. Never shown.
  const later = quoteFrom(OPT, windowOf([bar(asked.at + 60)]), 200, asked);
  assert.equal(later.close, null);
  assert.match(String(later.why), /no 1s bar at or before 10:15:07 on 2099-10-15: its first bar that day is stamped 10:16:07\. A later bar is never shown/);
  assert.match(String(quoteFrom(OPT, windowOf([bar(asked.at, { t: null })]), 200, asked).why), /no 1s bar at or before 10:15:07/);
  // A day with no bars, a month with none, and a damaged month are three different sentences.
  const day = { timeframe: '1s', month: '2099-10', day: '2099-10-19' };
  assert.match(String(quoteFrom(OPT, windowOf([]), 200, day).why), /holds no 1s bars for this series on 2099-10-19\. The day was asked by its own bounds/);
  assert.match(String(quoteFrom(OPT, windowOf([]), 200, { timeframe: '1s', month: '2099-10' }).why), /holds no 1s bars for this series in 2099-10\.$/);
  assert.match(String(quoteFrom(OPT, windowOf([], { faults: 'record 7 short' }), 206, day).why), /a record would not read: record 7 short\. Whether it holds 2099-10-19 is not something this read can say/);
  assert.doesNotMatch(String(quoteFrom(OPT, windowOf([], { faults: 'record 7 short' }), 206, day).why), /did not trade/);
  // A bar beside a fault keeps the fault.
  assert.equal(quoteFrom(OPT, windowOf([bar(5)], { faults: 'record 9 short' }), 206, { timeframe: '1s', month: '2099-10' }).faults, 'record 9 short');
  // Not a window at all: the server's own word, or the status.
  assert.equal(quoteFrom(OPT, { error: 'too_many_series: test' }, 400, day).why, 'too_many_series: test');
  assert.match(String(quoteFrom(OPT, null, 502, day).why), /HTTP 502 with no window/);
  assert.match(String(quoteFrom(OPT, { bars: 'no' }, 200, day).why), /HTTP 200 with no window/);
  // A day's last bar carries no asked instant.
  const close = quoteFrom(OPT, windowOf([bar(ist(2099, 10, 19, 15, 29, 59))]), 200, day);
  assert.deepEqual([close.asked, close.lag, close.why], [null, null, null]);
});

test('quoteOn makes one seek per series and window, caches the promise, never throws and bounds its cache', async () => {
  /** @type {string[]} */ const asked = [];
  useTransport(/** @type {any} */ (async (/** @type {string} */ url) => {
    asked.push(url);
    const q = new URL(url, 'http://x').searchParams;
    if (q.get('symbol') === 'BROKEN') throw new Error('socket closed');
    return reply(windowOf([bar(Number(q.get('at') ?? ist(2099, 10, 15, 15, 29, 59)))]));
  }));
  const [a, b] = await Promise.all([quoteOn('gdfl', NIFTY, '1s', '2099-10', '2099-10-15'), quoteOn('gdfl', NIFTY, '1s', '2099-10', '2099-10-15')]);
  assert.equal(a, b);
  assert.equal(asked.length, 1, 'two askers, one request');
  assert.equal(a.at, ist(2099, 10, 15, 15, 29, 59));
  // Day and time are part of the identity: a different second is a different request.
  await quoteOn('gdfl', NIFTY, '1s', '2099-10', '2099-10-15', '10:15:07');
  await quoteOn('gdfl', NIFTY, '1s', '2099-10', '2099-10-15', '10:15:08');
  await quoteOn('gdfl', NIFTY, '1s', '2099-10');
  assert.equal(asked.length, 4);
  assert.notEqual(quoteKey('f', 'k', '1s', '2099-10', '2099-10-15', '10:15:07'), quoteKey('f', 'k', '1s', '2099-10', '2099-10-15', '10:15:08'));
  // A transport failure is a quote with a reason, and it is cached as one.
  const broken = await quoteOn('gdfl', 'NSE-CASH-BROKEN', '1s', '2099-10');
  assert.deepEqual([broken.close, broken.why], [null, 'socket closed']);
  await quoteOn('gdfl', 'NSE-CASH-BROKEN', '1s', '2099-10');
  assert.equal(asked.length, 5);
  // A refused question never reaches the wire.
  assert.match(String((await quoteOn('gdfl', NIFTY, '1s', '2099-10', '2099-12-01')).why), /not a day of 2099-10/);
  assert.equal(asked.length, 5);
  // The cache is bounded: the oldest entry goes, so asking it again is a new request.
  useTransport(/** @type {any} */ (async (/** @type {string} */ url) => { asked.push(url); return reply(windowOf([bar(7)])); }));
  asked.length = 0;
  await quoteOn('gdfl', 'NSE-CASH-FIRST', '1s', '2099-10');
  await Promise.all(Array.from({ length: QUOTE_CACHE }, (_, i) => quoteOn('gdfl', `NSE-CASH-S${i}`, '1s', '2099-10')));
  assert.equal(asked.length, QUOTE_CACHE + 1);
  await quoteOn('gdfl', `NSE-CASH-S${QUOTE_CACHE - 1}`, '1s', '2099-10');
  assert.equal(asked.length, QUOTE_CACHE + 1, 'a recent entry is still held');
  await quoteOn('gdfl', 'NSE-CASH-FIRST', '1s', '2099-10');
  assert.equal(asked.length, QUOTE_CACHE + 2, 'the oldest entry was evicted');
});

test('every request shares one pool: never more than the cap in flight, however many batches are alive (DB-G86, RA-143)', async () => {
  let live = 0;
  let most = 0;
  /** @type {(() => void)[]} */ const held = [];
  useTransport(/** @type {any} */ (async () => {
    live += 1;
    most = Math.max(most, live);
    await new Promise((resolve) => held.push(() => resolve(undefined)));
    live -= 1;
    return reply(windowOf([bar(7)]));
  }));
  const rows = (/** @type {string} */ tag, /** @type {number} */ n) =>
    Array.from({ length: n }, (_, i) => ({ key: `NSE-CASH-${tag}${i}`, timeframe: '1s', month: '2099-10' }));
  // Three batches alive at once: 210 rows (the widened stocks view), 19 and 19.
  const batches = [quotesOn('gdfl', rows('A', 210), ''), quotesOn('gdfl', rows('B', 19), ''), quotesOn('gdfl', rows('C', 19), '2099-10-15')];
  const spot = gated('/spot.json?x=1');
  for (let turn = 0; turn < 400 && (held.length > 0 || live > 0 || turn < 3); turn += 1) {
    await new Promise((resolve) => setImmediate(resolve));
    assert.ok(traffic.inFlight <= traffic.cap, `in flight ${traffic.inFlight} over the cap`);
    for (const done of held.splice(0)) done();
  }
  const answers = await Promise.all(batches);
  await spot;
  assert.deepEqual(answers.map((a) => a.length), [210, 19, 19]);
  assert.equal(most, traffic.cap, 'the pool was filled, and never over-filled');
  assert.equal(traffic.peak, traffic.cap);
  assert.equal(traffic.sent, 249, '248 quote seeks and one spot request, each counted once');
  assert.equal(traffic.inFlight, 0, 'every slot came back');
  assert.ok(traffic.waited > 0);
  // An aborted request gives its slot back, and is not sent.
  useTransport(/** @type {any} */ (async () => reply({})));
  const gone = new AbortController();
  gone.abort();
  await assert.rejects(gated('/bars/window.json?x=1', { signal: gone.signal }), /abort/i);
  assert.deepEqual([traffic.inFlight, traffic.sent], [0, 0]);
  // A transport that throws gives its slot back too.
  useTransport(/** @type {any} */ (async () => { throw new Error('reset'); }));
  await assert.rejects(gated('/x'), /reset/);
  assert.equal(traffic.inFlight, 0);
  // The page prints the count.
  assert.match(page, /traffic\.sent|sentNow/);
});

test('headers name the last bar and its timeframe, and the doc table no longer claims a census change (DB-G17)', () => {
  assert.equal(barHeader('Change %', '1s'), 'Change % · last bar (1s)');
  assert.equal(barHeader('Volume', '60min'), 'Volume · last bar (60min)');
  assert.equal(barHeader('Change', ''), 'Change · last bar (no timeframe)');
  assert.match(page, /barHeader\(/);
  for (const name of ['Change', 'Change %', 'Volume', 'OI Change', 'OI Change %']) {
    assert.match(page, new RegExp(`h: '${name.replace('%', '\\%')}', src: '[A-Za-z]+', bar: true`), name);
  }
  assert.doesNotMatch(lib, /the census's own `chg_bps`/);
  assert.match(lib, /against the previous STORED bar at this timeframe, never a day change/);
});

test('there is one feed control and it persists through selectFeed (DB-G32)', () => {
  assert.doesNotMatch(page, /bind:value=\{feeds\.active\}/);
  assert.match(page, /selectFeed\(/);
  assert.match(page, /import \{ feeds, loadFeeds, selectFeed \} from '\$lib\/feeds\.svelte\.js'/);
  // Only the feeds the operator still uses are listed (D-3338), and the excluded ones are named, not hidden.
  assert.match(page, /\{#each offeredFeeds\(feeds\.all\) as f \(f\.wire\)\}/);
  assert.doesNotMatch(page, /\{#each feeds\.all as f/);
  assert.match(page, /title=\{excludedSentence\(feeds\.all\) \|\| undefined\}/);
  const layout = readFileSync(new URL('../src/routes/+layout.svelte', import.meta.url), 'utf8');
  assert.match(layout, /const FEED_OWNED = \[[^\]]*'\/terminal'[^\]]*\]/);
});

test('a census key that will not parse is counted and named, not dropped from every count (DB-G32)', () => {
  const keys = ['NSE-CASH-RELIANCE', 'NSE-CASH-BAJAJ-AUTO', NIFTY, OPT, FUT, '', 'garbage', 'NSE', 'x-y', 'a', 'b', 'c'];
  const { tabs, unparsed } = tabsAndRest(keys);
  const by = Object.fromEntries(tabs.map((t) => [t.id, t.keys]));
  assert.deepEqual(by.stocks, ['NSE-CASH-RELIANCE', 'NSE-CASH-BAJAJ-AUTO']);
  assert.deepEqual(by.futures, [FUT]);
  assert.deepEqual(by.options, [OPT]);
  const filed = tabs.reduce((n, t) => n + t.keys.length, 0);
  assert.ok(unparsed.count > 0);
  assert.ok(unparsed.sample.length <= UNPARSED_SAMPLE);
  assert.equal(unparsed.sample.length, Math.min(unparsed.count, UNPARSED_SAMPLE));
  // Every key is in a tab, is the index (the strip's), or is counted unparsed: none vanishes.
  assert.equal(filed + 1 + unparsed.count, keys.length);
  assert.deepEqual(tabsFrom(keys), tabs);
  assert.deepEqual(tabsAndRest(keys).indices, [NIFTY]);
  assert.deepEqual(tabsAndRest([]).unparsed, { count: 0, sample: [] });
  assert.match(tabsAndRest([]).tabs[0].why ?? '', /holds no stocks series/);
  assert.match(page, /unparsed\.count/);
});

test('the Date control takes any day of the month and says what the cells in view hold, with no request', () => {
  assert.deepEqual(monthSpan('2099-10'), { min: '2099-10-01', max: '2099-10-31' });
  assert.deepEqual(monthSpan('2096-02'), { min: '2096-02-01', max: '2096-02-29' });
  assert.deepEqual(monthSpan('2099-02'), { min: '2099-02-01', max: '2099-02-28' });
  for (const junk of ['', '2099-13', '2099-00', '2099-1', '99-10', '2099-10-15', null, undefined]) {
    assert.equal(monthSpan(/** @type {any} */ (junk)), null, String(junk));
  }
  assert.match(page, /const dayBounds = \$derived\(dayScope\.length > 0 \? monthSpan\(month\) : null\);/);
  assert.match(page, /if \(time && !timeCapable\) time = '';/);
  assert.deepEqual(daySpan([{ first: micros(2099, 10, 1, 9, 15), last: micros(2099, 10, 18, 15, 29) },
    { first: micros(2099, 10, 3, 9, 15), last: micros(2099, 10, 31, 15, 29) }, undefined]), { min: '2099-10-01', max: '2099-10-31' });
  // 23:59:59 IST and the next second fall on different IST days.
  assert.deepEqual(daySpan([{ first: micros(2099, 10, 1, 23, 59), last: micros(2099, 10, 2, 0, 0) }]), { min: '2099-10-01', max: '2099-10-02' });
  assert.equal(daySpan([]), null);
  assert.equal(daySpan([undefined, { first: null, last: null }]), null);
  assert.match(page, /type="date"/);
  assert.match(page, /type="time"/);
});

test('contract tabs can be narrowed to one underlying, from the names the tab itself holds', () => {
  assert.match(page, /const inChosenUnderlying = \(key\) => !underlying \|\| parseKey\(key\)\.underlying === underlying;/);
  assert.match(page, /\.filter\(inChosenSegment\)\s+\.filter\(inChosenUnderlying\)\s+\.filter\(inChosenExpiry\)\s+\.filter\(holdsChosenCell\)/);
  assert.match(page, /\{#each underlyings as u \(u\)\}<option value=\{u\}>\{u\}<\/option>\{\/each\}/);
  // No instrument name is typed into the page or the library for the strip or the picker.
  for (const source of [page, lib]) {
    assert.doesNotMatch(source.replace(/\/\*[\s\S]*?\*\/|<!--[\s\S]*?-->|\/\/.*$/gm, ''), /['"`](NSE-INDEX-)?(NIFTY|BANKNIFTY|FINNIFTY|MIDCPNIFTY|NIFTYNXT50)['"`]/);
  }
});
