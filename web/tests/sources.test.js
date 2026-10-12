// /sources — ALL SOURCES FOR ONE INSTRUMENT (WP-58; D-3339, D-3323, D-3324;
// DB-079, DB-G41, DB-G58, DB-G99).
//
// Before this package the three All sources routes (WP-37) had no page: /db is
// one feed at a time, and the operator reconciled Zerodha against GDFL by hand.
// Every test below fails on the tree before it, where `$lib/sources.js` and
// `routes/sources/+page.svelte` did not exist.
//
// THE FIXTURES ARE INVENTED. They have the SHAPE of the three routes' answers
// (`crates/api/src/sources.rs`), with feeds' names as the server spells them
// and round prices no instrument traded at (10,000.00, 500.00, 250.00). Nothing
// below is market evidence. The one thing read from the repository is the
// server's own source, to prove this page has a word for every code it raises.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compile } from 'svelte/compiler';

import {
  SOURCES_DEFAULTS,
  SOURCES_KEYS,
  FLAG_WORDS,
  WHY_WORDS,
  TIMELINE_LIMIT,
  PAGE_DAYS,
  realDate,
  instrumentOf,
  decodeSources,
  encodeSources,
  sourcesHref,
  oneInstrument,
  sourcesHrefOfDb,
  rungsUrl,
  barsUrl,
  dayUrl,
  isDayRung,
  shiftMonth,
  outcomeOf,
  flagOf,
  whyOf,
  tagWord,
  priceText,
  countText,
  basisOf,
  dbDayHref,
  rungSeconds,
  namedList,
  commonLastDay,
  rungTable,
  cmpCell,
  factorCell,
  timeline,
  marksOf,
  dayTable,
  toleranceLine
} from '../src/lib/sources.js';

const read = (/** @type {string} */ path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const PAGE = read('../src/routes/sources/+page.svelte');
const SERVER = read('../../crates/api/src/sources.rs');

// ══ Fixtures (invented) ══════════════════════════════════════════════════════

const INDEX_BASIS = { price: 'as_published', price_label: 'as published, an index level', volume: 'none', volume_label: 'none, an index has no traded volume', adjusted_as_of: null, source: 'an invented source line' };
const LAST = { ...INDEX_BASIS, close: 'last_print', close_label: 'last print' };
const OFFICIAL = { ...INDEX_BASIS, close: 'official', close_label: 'official close' };
const ADJUSTED = { price: 'vendor_adjusted', price_label: 'back-adjusted by the vendor as of the pull date', volume: 'vendor_own', volume_label: "the vendor's own basis", adjusted_as_of: 'pull_date', close: 'last_print', close_label: 'last print', source: 'invented' };
const RAW = { price: 'raw', price_label: 'raw, as traded', volume: 'raw', volume_label: 'raw, the traded quantity', adjusted_as_of: null, close: 'last_print', close_label: 'last print', source: 'invented' };

/** A rung table shaped like `/sources.json`, rows deliberately OUT OF ORDER. @returns {any} */
function rungsBody() {
  const db = (/** @type {string} */ feed, /** @type {string} */ tf) => `/db?feed=${feed}&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=${tf}`;
  const stored = (/** @type {any} */ over) => ({
    kind: 'stored', stored: true, derived_from: null, behind_source: null, holes: [], months_absent: 0, outside_window: null, state: 'held',
    census: { months_counted: 3, months_uncounted: 0, rows: 1000, uncounted: [] }, basis: LAST, ...over
  });
  return {
    schema: 1,
    instrument: { exchange: 'NSE', segment: 'INDEX', symbol: 'TESTIDX', contract: null, key: 'NSE-INDEX-TESTIDX' },
    feeds: [
      { feed: 'gdfl', display: 'Global Datafeeds', rungs_held: 1, archive: { from: '2020-01-01', to: '2020-03-20', source: 'invented' }, window: null, census: { state: 'held', generation: 7, note: 'gdfl: invented', degraded: null, behind_days: 0, behind_from: null } },
      { feed: 'zerodha', display: 'Zerodha', rungs_held: 3, archive: null, window: { from: '2020-01-01', to: '2020-03-31', source: 'invented', label_outside: 'outside your window' }, census: { state: 'held', generation: 9, note: 'zerodha: invented', degraded: null, behind_days: null, behind_from: null } }
    ],
    rungs: [
      // a fold listed before its own stored seconds, and Zerodha's day before its minutes
      { feed: 'gdfl', rung: '5min', kind: 'fold', stored: false, tag: 'gdfl_fold', derived_from: '1s', state: 'on_request', computed: 'on_request', fold: 'pull::fold: open first, high max, low min, close last', first_day: '2020-01-01', last_day: '2020-03-20', months_held: 3, sessions: {}, basis: LAST, bars: '/sources/bars.json?x' },
      stored({ feed: 'zerodha', rung: '1day', tag: 'zerodha_stored', first_day: '2017-01-02', last_day: '2020-03-31', first_month: '2017-01', last_month: '2020-03', months_held: 39, basis: OFFICIAL, db: db('zerodha', '1day'),
        outside_window: { first: '2017-01', last: '2019-12', months: 36, label: 'outside your window', db: `${db('zerodha', '1day')}&oow=1` },
        sessions: { calendar: '/calendar.json?feed=zerodha&symbol=TESTIDX&rung=1day', from: '2017-01-02', to: '2020-03-31', owed: 800, not_held: 0, not_held_named: [], partly_held: 0 } }),
      stored({ feed: 'zerodha', rung: '5min', kind: 'derived', tag: 'zerodha_derived', derived_from: '1min', state: 'partial', first_day: '2020-01-01', last_day: '2020-03-04', months_held: 3, db: db('zerodha', '5min'),
        behind_source: { source_rung: '1min', source_last_day: '2020-03-31', last_day: '2020-03-04', days_behind: 27, months_missing: 0, months_missing_named: [], sessions_missing: 2, sessions_missing_exact: true, sessions_missing_named: ['2020-02-05', '2020-02-06'] },
        sessions: { calendar: '/calendar.json?feed=zerodha&symbol=TESTIDX&rung=5min', owed: 60, not_held: 5, not_held_named: ['2020-01-10', '2020-02-05', '2020-02-06'], partly_held: 0 } }),
      stored({ feed: 'gdfl', rung: '1s', tag: 'gdfl_1s', first_day: '2020-01-01', last_day: '2020-03-20', months_held: 3, db: db('gdfl', '1s'),
        sessions: { calendar: '/calendar.json?feed=gdfl&symbol=TESTIDX&rung=1s', owed: 55, not_held: 0, not_held_named: [], partly_held: 0 } }),
      stored({ feed: 'zerodha', rung: '1min', tag: 'zerodha_stored', first_day: '2020-01-01', last_day: '2020-03-31', months_held: 2, months_absent: 1, holes: ['2020-02'], db: db('zerodha', '1min'),
        sessions: { calendar: '/calendar.json?feed=zerodha&symbol=TESTIDX&rung=1min', owed: 62, not_held: 9, not_held_named: ['2020-01-10', '2020-01-13'], partly_held: 0 } })
    ],
    tolerances: { price_bps: 1, volume_percent: 0.5, factor_percent: 0.2, rule: 'D-3339: invented wording' },
    notes: ['a note', 'a note']
  };
}

const cellOf = (/** @type {number} */ c, /** @type {string} */ tag, /** @type {any} */ over = {}) => ({ o: 1000000, h: 1000500, l: 999500, c, v: 0, oi: null, oi_why: 'not_a_derivative', close: 'last_print', first_at: null, seconds: null, tag, ...over });
const same = { bps: 0.0, equal: true, flag: false, paisa: 0 };

/** A timeline page shaped like `/sources/bars.json`: three buckets, the middle one held by one feed only. @returns {any} */
function barsBody() {
  return {
    schema: 1,
    instrument: { exchange: 'NSE', segment: 'INDEX', symbol: 'TESTIDX', contract: null, key: 'NSE-INDEX-TESTIDX' },
    rung: '1min', range: { from: '2020-03-20', to: '2020-03-20' }, dir: 'desc', offset: 0, limit: 50, cursor: null, next_cursor: 1584697500, complete: false, records_read: 187,
    sources: [
      { feed: 'gdfl', kind: 'fold', read_rung: '1s', tag: 'gdfl_fold', basis: LAST, buckets_read: 3, exhausted: false },
      { feed: 'zerodha', kind: 'stored', read_rung: '1min', tag: 'zerodha_stored', basis: LAST, buckets_read: 3, exhausted: false }
    ],
    rows: [
      { t: 1584698340, ist: '2020-03-20 15:29:00', only: null, flags: [],
        cells: { gdfl: cellOf(1000000, 'gdfl_fold', { first_at: '15:29:00', seconds: 60 }), zerodha: cellOf(1000000, 'zerodha_stored') },
        cmp: { base: { against: 'gdfl', of: 'zerodha' }, basis: 'same', factor: null, factor_of: null, o: same, h: same, l: same, c: same, v: { why: 'index_no_volume' } } },
      { t: 1584698280, ist: '2020-03-20 15:28:00', only: 'zerodha', flags: [], cells: { gdfl: null, zerodha: cellOf(1000200, 'zerodha_stored') }, cmp: null },
      { t: 1584698220, ist: '2020-03-20 15:27:00', only: null, flags: ['c', 'a_flag_from_the_future'],
        cells: { gdfl: cellOf(1000300, 'gdfl_fold', { first_at: '15:27:02', seconds: 41 }), zerodha: cellOf(1000000, 'zerodha_stored') },
        cmp: { base: { against: 'gdfl', of: 'zerodha' }, basis: 'same', factor: null, factor_of: null, o: same, h: same, l: same, c: { bps: 3.0, equal: false, flag: true, paisa: 300 }, v: { why: 'index_no_volume' } } }
    ],
    summary: { rows: 3, both_feeds: 2, one_feed: 1, compared_on_one_basis: 2, prices_within_tolerance: 1, prices_disagree: 1, basis_differs: 0, basis_unmeasured: 0, exactly_equal: { o: 2, h: 2, l: 2, c: 1 } },
    tolerances: { price_bps: 1, volume_percent: 0.5, factor_percent: 0.2, rule: 'invented' },
    notes: ['an index has no traded volume: volume is never compared']
  };
}

/** One day of `/sources/day.json` for an index: every field the day table reads. @returns {any} */
function indexDay(/** @type {string} */ day, /** @type {number} */ official, /** @type {number} */ lastPrint) {
  const equal = official === lastPrint;
  const bps = equal ? 0.0 : Number((((lastPrint - official) / official) * 10000).toFixed(2));
  return {
    day, absent: {}, auction: null, basis: null, flags: equal ? [] : ['official_vs_last_print'], records_read: 20000, zerodha_day_close: 'official',
    zerodha_day: { o: 1000000, h: 1002000, l: 998000, c: official, v: 0 },
    zerodha_minutes_fold: { o: 1000000, h: 1001900, l: 998100, c: lastPrint, v: 0, first_at: '09:15:00', last_at: '15:29:00', minutes: 375 },
    gdfl_seconds_fold: { o: 1000000, h: 1002000, l: 998000, c: lastPrint, v: 0, first_at: '09:15:00', last_at: '15:29:59', seconds: 21000 },
    open: { gdfl_first_print: 1000000, gdfl_first_print_at: '09:15:00', gdfl_first_print_is_open: true, gdfl_vs_zerodha_day: { bps: 0.0, flag: false }, zerodha_day: 1000000, zerodha_first_minute: 1000000, zerodha_first_minute_at: '09:15:00', zerodha_minute_vs_day: { bps: 0.0, flag: false } },
    high: { gdfl_seconds: 1002000, gdfl_vs_zerodha_day: { bps: 0.0, flag: false }, minute_above_day: false, zerodha_day: 1002000, zerodha_minutes: 1001900 },
    low: { gdfl_seconds: 998000, gdfl_vs_zerodha_day: { bps: 0.0, flag: false }, minute_below_day: false, zerodha_day: 998000, zerodha_minutes: 998100 },
    close: { equal, official, gdfl_last_print: lastPrint, zerodha_last_minute: lastPrint,
      official_vs_gdfl_last_print: { bps, flag: false, semantics: 'official_vs_last_print' },
      official_vs_zerodha_last_minute: { bps, flag: false, semantics: 'official_vs_last_print' },
      last_common_minute: { at: '15:29:00', gdfl: lastPrint, zerodha: lastPrint, gdfl_vs_zerodha: { bps: 0.0, flag: false } } },
    last_print: { gdfl_last_print: lastPrint, gdfl_last_print_at: '15:29:59', zerodha_last_minute: lastPrint, zerodha_last_minute_at: '15:29:00' },
    volume: { gdfl_sum: 0, zerodha_day: 0, why: 'index_no_volume' },
    marks: { calendar: { code: 'R', kind: 'regular', weekday: 'Fri', holiday: null, special: null, halted_minutes: 0, owed_minutes: 375, unverified_minutes: 0 }, degraded: [], says: ['Zerodha holds 375 bar(s)'] }
  };
}

/** @returns {any} */
function daysBody(/** @type {any[]} */ days, /** @type {any} */ summary = {}) {
  return {
    schema: 1,
    instrument: { exchange: 'NSE', segment: 'INDEX', symbol: 'TESTIDX', contract: null, key: 'NSE-INDEX-TESTIDX' },
    asked: { from: '2020-03-01', to: '2020-03-31', order: 'asc' }, page: 0, pages: 1, page_days: 31, page_from: '2020-03-01', page_to: '2020-03-31', records_read: 60000,
    sources: {
      zerodha_day: { feed: 'zerodha', rung: '1day', tag: 'zerodha_stored', folded_to_day: false, basis: OFFICIAL },
      zerodha_minutes: { feed: 'zerodha', rung: '1min', tag: 'zerodha_stored', folded_to_day: true, basis: LAST },
      gdfl_seconds: { feed: 'gdfl', rung: '1s', tag: 'gdfl_1s', folded_to_day: true, basis: LAST }
    },
    days,
    summary: { days: days.length, both_feeds: days.length, zerodha_only: 0, gdfl_only: 0, neither: 0, open_equal: days.length, high_equal: days.length, low_equal: days.length, close_equal: 0, close_differs: days.length, close_compared_as: 'official (Zerodha 1day) against last print (GDFL seconds): labelled, never a disagreement (D-3324)', basis_differs: 0, basis_steps: 0, minute_extremes_outside_day: 0, minute_volume_off_day: 0, gdfl_first_print_after_open: 0, volume_compared: 0, volume_within_tolerance: 0, days_from_cache: 0, ...summary },
    notes: ['a day-table note']
  };
}

// ══ The address ══════════════════════════════════════════════════════════════

test('an empty address names nothing, and the page opens on NIFTY at 1min, newest first', () => {
  for (const empty of ['', '?', null, undefined]) {
    assert.deepEqual(decodeSources(empty), { sel: {}, refused: [] });
  }
  assert.deepEqual({ ...SOURCES_DEFAULTS }, { key: 'NSE-INDEX-NIFTY', rung: '1min', dir: 'desc' });
  assert.equal(sourcesHref({}), '/sources');
  assert.equal(sourcesHref(null), '/sources');
});

test('one instrument is read from three spellings: this page, the routes and /db', () => {
  assert.equal(decodeSources('?key=nse-cash-reliance').sel.key, 'NSE-CASH-RELIANCE');
  assert.equal(decodeSources('exchange=NSE&segment=INDEX&symbol=NIFTY').sel.key, 'NSE-INDEX-NIFTY');
  assert.equal(
    decodeSources('exchange=NSE&segment=FNO&symbol=NIFTY&contract=2019-03-28-1150000-CE').sel.key,
    'NSE-FNO-NIFTY-2019-03-28-1150000-CE'
  );
  // /db's own link: key, tf for the rung, from for the day; its feed and seg are not this page's and are ignored
  const fromDb = decodeSources('feed=gdfl&seg=spot&sym=NIFTY&key=NSE-INDEX-NIFTY&tf=1s&from=2019-03-29&to=2019-03-29');
  assert.deepEqual(fromDb, { sel: { key: 'NSE-INDEX-NIFTY', rung: '1s', day: '2019-03-29' }, refused: [] });
  // a hyphenated symbol keeps its hyphen
  assert.deepEqual(instrumentOf('NSE-CASH-BAJAJ-AUTO'), { ok: true, key: 'NSE-CASH-BAJAJ-AUTO', exchange: 'NSE', segment: 'CASH', symbol: 'BAJAJ-AUTO', contract: null });
  assert.deepEqual(instrumentOf('NSE-FNO-NIFTY-2019-03-28-1150000-CE'), { ok: true, key: 'NSE-FNO-NIFTY-2019-03-28-1150000-CE', exchange: 'NSE', segment: 'FNO', symbol: 'NIFTY', contract: '2019-03-28-1150000-CE' });
});

test('garbage in the address is named field by field and never coerced', () => {
  const { sel, refused } = decodeSources('key=NIFTY&rung=7%20min&day=2019-02-30&month=2019-13&dir=up&cursor=abc');
  assert.deepEqual(sel, {});
  assert.deepEqual(refused.map((/** @type {any} */ r) => r.key), ['key', 'rung', 'day', 'month', 'dir', 'cursor']);
  for (const r of refused) assert.ok(r.why.length > 10 && r.value.length > 0, `${r.key} says why`);
  // a future is refused by name, not sent as a symbol it never was
  const future = instrumentOf('NSE-FNO-NIFTY-2019-03-28-FUT');
  assert.equal(future.ok, false);
  assert.match(/** @type {any} */ (future).why, /is a future/);
  for (const bad of ['', '   ', null, undefined, 42, 'NSE-INDEX-', '<script>']) assert.equal(instrumentOf(bad).ok, false, String(bad));
  // script-shaped input comes back only inside JSON quotes in the reason
  assert.ok(/** @type {any} */ (instrumentOf('<b>x</b>')).why.includes('"<B>X</B>"'));
  assert.equal(realDate('2020-02-29'), true);
  assert.equal(realDate('2019-02-29'), false);
  assert.equal(realDate('2019-00'), false);
  assert.equal(realDate('nonsense'), false);
});

test('one selection is one address, byte for byte, and survives a round trip', () => {
  const sel = { key: 'NSE-CASH-RELIANCE', rung: '1day', day: '2019-05-30', dir: /** @type {'asc'} */ ('asc'), cursor: '1559154600', month: '2019-05' };
  const q = encodeSources(sel);
  assert.equal(q, 'key=NSE-CASH-RELIANCE&rung=1day&day=2019-05-30&dir=asc&cursor=1559154600&month=2019-05');
  assert.deepEqual(decodeSources(q), { sel, refused: [] });
  assert.equal(encodeSources(decodeSources(q).sel), q);
  // defaults are left out, in SOURCES_KEYS order
  assert.equal(encodeSources({ key: 'nse-index-nifty', rung: '1min', dir: 'desc' }), 'key=NSE-INDEX-NIFTY');
  assert.deepEqual([...new URLSearchParams(q).keys()], [...SOURCES_KEYS]);
});

test('/db links to the same instrument, rung and first day, or draws no link at all', () => {
  assert.equal(sourcesHrefOfDb({ key: 'NSE-INDEX-NIFTY', tf: '1s', from: '2019-03-29' }), '/sources?key=NSE-INDEX-NIFTY&rung=1s&day=2019-03-29');
  assert.equal(sourcesHrefOfDb({ key: 'NSE-CASH-RELIANCE', tf: '1min', from: '' }), '/sources?key=NSE-CASH-RELIANCE');
  assert.equal(sourcesHrefOfDb({ key: 'NSE-FNO-NIFTY-2019-03-28-1150000-CE', tf: '1s' }), '/sources?key=NSE-FNO-NIFTY-2019-03-28-1150000-CE&rung=1s');
  // no single instrument picked on /db: no link, never one that opens on another instrument
  for (const none of [{}, { key: '' }, { key: null }, { key: 'NIFTY' }, { key: 'NSE-FNO-NIFTY-2019-03-28-FUT' }]) assert.equal(sourcesHrefOfDb(none), null);
  // a rung or day /db spells wrongly is dropped, not passed on
  assert.equal(sourcesHrefOfDb({ key: 'NSE-INDEX-NIFTY', tf: 'weekly', from: '2019-02-30' }), '/sources?key=NSE-INDEX-NIFTY');
  const summary = read('../src/routes/db/DbSummary.svelte');
  assert.match(summary, /sourcesHrefOfDb\(\{ key: oneInstrument\(db\.matched\), tf: db\.timeframe, from: db\.fromDay \}\)/);
  // the series is the census key of the selection's matched rows: exactly one, or no link
  const month = (/** @type {string} */ instrument, /** @type {string} */ m) => ({ instrument, month: m, timeframe: '1s', rows: 10 });
  assert.equal(oneInstrument([month('NSE-INDEX-NIFTY', '2019-03'), month('NSE-INDEX-NIFTY', '2019-04')]), 'NSE-INDEX-NIFTY');
  assert.equal(oneInstrument([month('NSE-INDEX-NIFTY', '2019-03')]), 'NSE-INDEX-NIFTY');
  assert.equal(oneInstrument([month('NSE-FNO-NIFTY-2019-03-28-1150000-CE', '2019-03'), month('NSE-FNO-NIFTY-2019-03-28-1150000-PE', '2019-03')]), '');
  for (const none of [[], null, undefined, 'NSE-INDEX-NIFTY', [{}], [{ instrument: 7 }], [month('NSE-INDEX-NIFTY', '2019-03'), {}]]) assert.equal(oneInstrument(none), '', JSON.stringify(none));
  assert.equal(sourcesHrefOfDb({ key: oneInstrument([month('NSE-INDEX-NIFTY', '2019-03')]), tf: '1s', from: '2019-03-29' }), '/sources?key=NSE-INDEX-NIFTY&rung=1s&day=2019-03-29');
  assert.match(summary, /\{#if allSources\}\s*<a class="allsrc" href=\{allSources\}/);
});

// ══ The three requests ═══════════════════════════════════════════════════════

test('each request is bounded by its page: one day of minutes, one month of days', () => {
  assert.equal(rungsUrl('NSE-INDEX-NIFTY'), '/sources.json?exchange=NSE&segment=INDEX&symbol=NIFTY');
  assert.equal(rungsUrl('NSE-FNO-NIFTY-2019-03-28-1150000-CE'), '/sources.json?exchange=NSE&segment=FNO&symbol=NIFTY&contract=2019-03-28-1150000-CE');
  assert.equal(
    barsUrl({ key: 'NSE-INDEX-NIFTY', rung: '1min', day: '2026-09-24' }),
    `/sources/bars.json?exchange=NSE&segment=INDEX&symbol=NIFTY&rung=1min&from=2026-09-24&to=2026-09-24&limit=${TIMELINE_LIMIT}&dir=desc`
  );
  assert.equal(
    barsUrl({ key: 'NSE-CASH-RELIANCE', rung: '5min', day: '2019-05-30', dir: 'asc', cursor: '1559186100' }),
    `/sources/bars.json?exchange=NSE&segment=CASH&symbol=RELIANCE&rung=5min&from=2019-05-30&to=2019-05-30&limit=${TIMELINE_LIMIT}&dir=asc&cursor=1559186100`
  );
  assert.equal(dayUrl('NSE-INDEX-NIFTY', '2019-03'), '/sources/day.json?exchange=NSE&segment=INDEX&symbol=NIFTY&from=2019-03');
  assert.equal(TIMELINE_LIMIT, 50);
  assert.equal(PAGE_DAYS, 31);
  // the limits are the server's own
  assert.match(SERVER, /pub const DEFAULT_LIMIT: usize = 50;/);
  assert.match(SERVER, /pub const PAGE_DAYS: u32 = 31;/);
  assert.match(SERVER, /pub const MAX_DAY_LIMIT: usize = 31;/);
  // nothing to ask is null, never a URL with a hole in it
  assert.equal(rungsUrl('NIFTY'), null);
  assert.equal(barsUrl({ key: 'NSE-INDEX-NIFTY', rung: '1min' }), null);
  assert.equal(barsUrl({ key: 'NSE-INDEX-NIFTY', rung: '', day: '2026-09-24' }), null);
  assert.equal(barsUrl({ key: 'NSE-INDEX-NIFTY', rung: '1min', day: '24 Sep' }), null);
  assert.equal(dayUrl('NSE-INDEX-NIFTY', '2019-3'), null);
  assert.equal(dayUrl('', '2019-03'), null);
  assert.equal(isDayRung('1day'), true);
  assert.equal(isDayRung('60min'), false);
});

test('at the day rung the page reads the kept day table and never asks for a month of seconds to be folded', () => {
  // measured on the operator's store: /sources/bars.json?rung=1day folds a month of seconds per request, unkept
  for (const rung of ['1day', '2day']) assert.equal(barsUrl({ key: 'NSE-INDEX-NIFTY', rung, day: '2019-03-29' }), null, rung);
  assert.equal(dayUrl('NSE-INDEX-NIFTY', '2019-03'), '/sources/day.json?exchange=NSE&segment=INDEX&symbol=NIFTY&from=2019-03');
  assert.match(PAGE, /const dayRung = \$derived\(isDayRung\(rung\)\);/);
  assert.match(PAGE, /\{#if dayRung\}\s*<p class="state" data-day-rung>/);
  // every intraday page is one IST day: a fold reads at most that day's seconds, whatever the rung
  for (const rung of ['1s', '1min', '5min', '60min']) {
    const q = new URLSearchParams(String(barsUrl({ key: 'NSE-INDEX-NIFTY', rung, day: '2019-03-29' })).split('?')[1]);
    assert.deepEqual([q.get('from'), q.get('to'), q.get('limit')], ['2019-03-29', '2019-03-29', '50'], rung);
  }
});

test('months step across a year end and refuse garbage', () => {
  assert.equal(shiftMonth('2019-03', -1), '2019-02');
  assert.equal(shiftMonth('2019-01', -1), '2018-12');
  assert.equal(shiftMonth('2019-12', 1), '2020-01');
  assert.equal(shiftMonth('2019-12', 13), '2021-01');
  assert.equal(shiftMonth('0000-01', -1), '');
  assert.equal(shiftMonth('2019-3', 1), '');
  assert.equal(shiftMonth('', 1), '');
});

test('an answer, a refusal and a failure are three different things', () => {
  assert.deepEqual(outcomeOf(200, { schema: 1 }), { phase: 'ok', body: { schema: 1 } });
  assert.deepEqual(outcomeOf(400, { error: 'rung="7min" is not a rung this store has.' }), { phase: 'refused', status: 400, why: 'rung="7min" is not a rung this store has.' });
  assert.equal(outcomeOf(429, null).phase, 'refused');
  assert.match(/** @type {any} */ (outcomeOf(429, null)).why, /429/);
  assert.deepEqual(outcomeOf(503, { error: 'the store is not readable' }), { phase: 'failed', status: 503, why: 'the store is not readable' });
  assert.equal(outcomeOf(500, null).phase, 'failed');
  // a 200 that is not this route's JSON is a failure, never an empty table
  assert.equal(outcomeOf(200, null).phase, 'failed');
  assert.equal(outcomeOf(200, 'text').phase, 'failed');
  assert.equal(outcomeOf(200, { error: 'said on a 200' }).phase, 'failed');
});

// ══ Words ════════════════════════════════════════════════════════════════════

test('every flag, reason and state the server can raise has words here', () => {
  const flags = new Set([...SERVER.matchAll(/flags\.push\("([a-z_]+)"\)/g)].map((/** @type {any} */ m) => m[1]));
  // the timeline's per-field flags are a match on the field name
  for (const m of SERVER.matchAll(/"([ohlc])" => "\1"|_ => "(c)"/g)) flags.add(m[1] ?? m[2]);
  assert.ok(flags.size >= 14, `found ${flags.size} flags in sources.rs`);
  for (const code of flags) assert.ok(Object.hasOwn(FLAG_WORDS, code), `no words for the flag ${code}`);
  for (const code of Object.keys(FLAG_WORDS)) assert.ok(flags.has(code), `${code} is worded here and raised nowhere in sources.rs`);

  const whys = new Set([...SERVER.matchAll(/"why": "([a-z_]+)"/g)].map((/** @type {any} */ m) => m[1]));
  for (const m of SERVER.matchAll(/return "([a-z_]+)";/g)) whys.add(m[1]);
  for (const m of SERVER.matchAll(/then_some\("(not_[a-z_]+)"\)/g)) whys.add(m[1]);
  assert.ok(whys.has('after_archive') && whys.has('basis_unmeasured') && whys.has('index_no_volume'));
  for (const code of whys) assert.ok(Object.hasOwn(WHY_WORDS, code), `no words for the reason ${code}`);

  const table = rungTable({ feeds: [], rungs: ['held', 'partial', 'uncounted', 'sessions_unchecked', 'unexpected', 'on_request'].map((state) => ({ feed: 'f', rung: '1min', kind: 'stored', state })) });
  assert.deepEqual(table.rows.map((/** @type {any} */ r) => r.state.word), ['held', 'partial', 'uncounted', 'sessions unchecked', 'unexpected', 'folded on request']);
  for (const state of ['"partial"', '"uncounted"', '"sessions_unchecked"', '"unexpected"', '"held"', '"on_request"']) assert.ok(SERVER.includes(state), state);
  assert.deepEqual(table.rows.map((/** @type {any} */ r) => r.state.tone), ['ok', 'warn', 'warn', 'warn', 'bad', 'quiet']);
});

test('a code this page has no words for is shown as sent, loudly, never dropped', () => {
  assert.deepEqual(flagOf('a_flag_from_the_future'), { code: 'a_flag_from_the_future', word: 'a_flag_from_the_future', tone: 'warn', why: 'the server raised "a_flag_from_the_future", which this page has no words for; shown as sent' });
  assert.equal(flagOf(null).word, 'unnamed flag');
  assert.equal(flagOf('constructor').tone, 'warn'); // not an inherited property
  assert.equal(whyOf('some_new_reason'), 'some new reason');
  assert.equal(whyOf('toString'), 'toString');
  assert.equal(whyOf(''), 'no reason was sent');
  assert.equal(flagOf('official_vs_last_print').tone, 'info');
  assert.equal(flagOf('basis_differs').tone, 'info');
  assert.equal(flagOf('c').tone, 'bad');
  const unknown = rungTable({ feeds: [], rungs: [{ feed: 'f', rung: '1min', kind: 'stored', state: 'half_written' }] }).rows[0].state;
  assert.deepEqual([unknown.word, unknown.tone], ['half written', 'warn']);
});

test('a source is tagged in the plan’s words by one rule for every feed', () => {
  assert.equal(tagWord({ feed: 'zerodha', kind: 'stored', rung: '1min' }), 'zerodha 1min stored');
  assert.equal(tagWord({ feed: 'zerodha', kind: 'derived', rung: '5min', from: '1min' }), 'zerodha 5min derived');
  assert.equal(tagWord({ feed: 'gdfl', kind: 'stored', rung: '1s' }), 'gdfl 1s stored');
  assert.equal(tagWord({ feed: 'gdfl', kind: 'fold', rung: '1min', from: '1s' }), 'gdfl 1s→1min fold');
  assert.equal(tagWord({ feed: 'another', kind: 'fold', rung: '1day', from: '1min' }), 'another 1min→1day fold');
  assert.equal(tagWord({ feed: 'x', kind: '', rung: '1min' }), 'x 1min kind not sent');
  assert.equal(tagWord({}), '? ? kind not sent');
});

test('prices are exact paisa, never a float, and a missing one is a dash', () => {
  assert.equal(priceText(1000000), '10,000.00');
  assert.equal(priceText(1234505), '12,345.05');
  assert.equal(priceText(5), '0.05');
  assert.equal(priceText(0), '0.00');
  assert.equal(priceText(-150), '-1.50');
  for (const none of [null, undefined, '1000000', 10.5, Number.NaN, 2 ** 60]) assert.equal(priceText(none), '—');
  assert.equal(countText(0), '0');
  assert.equal(countText(null), '—');
  assert.equal(rungSeconds('1s'), 1);
  assert.equal(rungSeconds('60min'), 3600);
  assert.equal(rungSeconds('1day'), 86400);
  assert.equal(rungSeconds('weekly'), Number.MAX_SAFE_INTEGER);
});

test('a basis is three badges, and an unverified one says so', () => {
  assert.deepEqual(basisOf(ADJUSTED), { price: 'back-adjusted by the vendor as of the pull date', volume: "the vendor's own basis", close: 'last print', adjusted: true, source: 'invented', unverified: '', tone: 'info' });
  assert.equal(basisOf(RAW).tone, 'quiet');
  assert.equal(basisOf(OFFICIAL).close, 'official close');
  const unverified = basisOf({ price: null, volume: null, close: null, close_label: null, unverified: 'no charter row' });
  assert.deepEqual([unverified.price, unverified.volume, unverified.close, unverified.tone, unverified.unverified], ['UNVERIFIED', 'UNVERIFIED', 'not sent', 'warn', 'no charter row']);
  assert.deepEqual([basisOf(null).price, basisOf(undefined).close], ['not sent', 'not sent']);
});

// ══ The rung table ═══════════════════════════════════════════════════════════

test('the rung table lists every feed and rung, stored first and finest first, whatever order they arrive in', () => {
  const table = rungTable(rungsBody());
  assert.equal(table.key, 'NSE-INDEX-TESTIDX');
  assert.deepEqual(table.rows.map((/** @type {any} */ r) => r.tagWord), ['gdfl 1s stored', 'gdfl 1s→5min fold', 'zerodha 1min stored', 'zerodha 5min derived', 'zerodha 1day stored']);
  assert.deepEqual(table.feeds.map((/** @type {any} */ f) => [f.name, f.rungsHeld, f.census.state, f.census.tone]), [['Global Datafeeds', 1, 'held', 'ok'], ['Zerodha', 3, 'held', 'ok']]);
  assert.deepEqual(table.feeds[0].archive, { from: '2020-01-01', to: '2020-03-20', source: 'invented' });
  assert.equal(table.feeds[0].window, null);
  assert.equal(table.empty, '');
  // every stored row links to its own feed's rows on /db, in the server's words; a fold has none
  assert.deepEqual(table.rows.map((/** @type {any} */ r) => r.db), [
    '/db?feed=gdfl&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=1s', null,
    '/db?feed=zerodha&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=1min',
    '/db?feed=zerodha&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=5min',
    '/db?feed=zerodha&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=1day'
  ]);
  assert.equal(table.rows[1].timeline, '/sources?key=NSE-INDEX-TESTIDX&rung=5min');
  assert.deepEqual(table.notes, ['a note', 'a note']); // a repeated note is kept, not deduplicated away
});

test('the folds of one feed are one line that still names every rung, each with its own timeline link', () => {
  const body = rungsBody();
  const fold = body.rungs[0];
  body.rungs.push({ ...fold, rung: '1day' }, { ...fold, rung: '1min' }, { ...fold, rung: '15min', last_day: '2020-02-28' });
  const table = rungTable(body);
  // every row is still in `rows`, one per feed and rung
  assert.equal(table.rows.filter((/** @type {any} */ r) => r.kind === 'fold').length, 4);
  assert.deepEqual(table.lines.map((/** @type {any} */ r) => [r.tagWord, r.rungs.map((/** @type {any} */ x) => x.rung)]), [
    ['gdfl 1s stored', ['1s']],
    ['gdfl 1s→ fold', ['1min', '5min', '1day']],
    ['gdfl 1s→15min fold', ['15min']], // a fold whose span differs is not folded into the line
    ['zerodha 1min stored', ['1min']],
    ['zerodha 5min derived', ['5min']],
    ['zerodha 1day stored', ['1day']]
  ]);
  assert.deepEqual(table.lines[1].rungs.map((/** @type {any} */ x) => x.timeline), ['/sources?key=NSE-INDEX-TESTIDX', '/sources?key=NSE-INDEX-TESTIDX&rung=5min', '/sources?key=NSE-INDEX-TESTIDX&rung=1day']);
  // stored and derived rungs are never merged: each has its own bars, holes and /db link
  assert.equal(table.lines.filter((/** @type {any} */ r) => r.kind !== 'fold').every((/** @type {any} */ r) => r.rungs.length === 1), true);
  // with no fold at all the lines are the rows
  const none = rungsBody();
  none.rungs = none.rungs.filter((/** @type {any} */ r) => r.kind !== 'fold');
  assert.deepEqual(rungTable(none).lines.map((/** @type {any} */ r) => r.tagWord), rungTable(none).rows.map((/** @type {any} */ r) => r.tagWord));
  assert.match(PAGE, /\{#each rungs\.lines as r, i \(i\)\}/);
});

test('holes are named: absent months, owed sessions not held, and when the list is not whole', () => {
  const minute = rungTable(rungsBody()).rows[2];
  assert.equal(minute.monthsAbsent, 1);
  assert.equal(minute.monthHoles.text, '2020-02');
  assert.equal(minute.sessions.notHeld, 9);
  assert.equal(minute.sessions.owed, 62);
  assert.equal(minute.sessions.holes.text, '2020-01-10, 2020-01-13 and 7 more');
  assert.equal(minute.sessions.holes.all, false);
  assert.equal(minute.sessions.calendar, '/calendar.json?feed=zerodha&symbol=TESTIDX&rung=1min');
  assert.deepEqual(namedList(['a', 'b'], 2), { text: 'a, b', all: true, names: ['a', 'b'] });
  assert.deepEqual(namedList([], 0), { text: '', all: true, names: [] });
  assert.deepEqual(namedList(undefined, 4), { text: '', all: false, names: [] });
  assert.equal(namedList(['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'], 8).text, 'a, b, c, d, e, f and 2 more');
});

test('a derived rung behind its minutes is partial and says exactly what it lacks', () => {
  const derived = rungTable(rungsBody()).rows[3];
  assert.deepEqual([derived.state.code, derived.state.word, derived.state.tone], ['partial', 'partial', 'warn']);
  assert.deepEqual(derived.state.why, [
    'lacks 2 sessions its 1min source holds: 2020-02-05, 2020-02-06',
    'ends 2020-03-04, 27 days behind its 1min source (2020-03-31)'
  ]);
  const fold = rungTable(rungsBody()).rows[1];
  assert.deepEqual([fold.state.word, fold.state.tone, fold.stored], ['folded on request', 'quiet', false]);
  assert.match(fold.state.why[0], /pull::fold/);
  // months the census has not counted are named too
  const body = rungsBody();
  body.rungs[3].census = { months_counted: 1, months_uncounted: 2, rows: 10, uncounted: ['2020-02', '2020-03'] };
  body.rungs[3].state = 'uncounted';
  const uncounted = rungTable(body).rows[0];
  assert.equal(uncounted.state.word, 'uncounted');
  assert.deepEqual(uncounted.state.why, ['2 month files on disk the census has not counted: 2020-02, 2020-03']);
});

test('months outside the operator’s window are shaded and labelled, never dropped', () => {
  const day = rungTable(rungsBody()).rows[4];
  assert.deepEqual(day.outside, { from: '2017-01', to: '2019-12', months: 36, label: 'outside your window', db: '/db?feed=zerodha&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=1day&oow=1' });
  assert.equal(day.first, '2017-01-02'); // the true first day is still on the row
  assert.equal(day.insideFrom, '2020-01-01'); // and the in-window span starts at the window
  assert.equal(day.basis.close, 'official close');
  assert.equal(rungTable(rungsBody()).rows[2].outside, null);
  assert.match(PAGE, /class="oow"/);
  assert.match(PAGE, /\{r\.outside\.label\}/);
});

test('an instrument no feed holds, a single feed and an empty answer are each said in words', () => {
  const none = rungTable({ instrument: { key: 'NSE-INDEX-NOSUCH' }, feeds: [{ feed: 'gdfl', display: 'Global Datafeeds', rungs_held: 0, census: { state: 'held' } }], rungs: [], notes: ['gdfl holds no rung of it'] });
  assert.equal(none.empty, 'No offered feed holds a rung of NSE-INDEX-NOSUCH in this store.');
  assert.equal(none.lastDay, '');
  for (const empty of [null, undefined, {}, { rungs: 'x', feeds: 7 }]) {
    const table = rungTable(empty);
    assert.deepEqual([table.rows.length, table.feeds.length], [0, 0]);
    assert.match(table.empty, /No offered feed holds a rung of this instrument/);
  }
  const single = rungsBody();
  single.rungs = single.rungs.filter((/** @type {any} */ r) => r.feed === 'gdfl');
  assert.equal(rungTable(single).lastDay, '2020-03-20');
  const degraded = rungsBody();
  degraded.feeds[1].census = { state: 'behind', degraded: 'manifest unreadable', note: 'n', behind_days: 3, behind_from: '2020-03-28' };
  assert.deepEqual(rungTable(degraded).feeds[1].census, { state: 'behind', tone: 'warn', note: 'n', degraded: 'manifest unreadable', behindDays: 3, behindFrom: '2020-03-28' });
});

test('the page opens on the newest day EVERY holding feed has, so the first screen has every feed on it', () => {
  assert.equal(commonLastDay(rungsBody()), '2020-03-20'); // gdfl ends 03-20, zerodha 03-31
  assert.equal(rungTable(rungsBody()).lastDay, '2020-03-20');
  // a fold's last day is never read: only stored rungs count
  const body = rungsBody();
  body.rungs[0].last_day = '2021-01-01';
  assert.equal(commonLastDay(body), '2020-03-20');
  assert.equal(commonLastDay({ rungs: [{ feed: 'a', stored: true, last_day: 'garbage' }] }), '');
  assert.equal(commonLastDay(null), '');
});

// ══ Comparisons ══════════════════════════════════════════════════════════════

test('a comparison is the server’s: equal, within tolerance, or a disagreement', () => {
  assert.deepEqual(cmpCell(same), { text: '=', note: '', tone: 'ok', title: 'the two values are the same paisa' });
  const within = cmpCell({ bps: -0.28, equal: false, flag: false, paisa: -30 });
  assert.deepEqual([within.text, within.tone], ['−0.28 bps', 'quiet']);
  assert.match(within.title, /within the price tolerance \(−0\.30\)/);
  const beyond = cmpCell({ bps: 3.0, equal: false, flag: true, paisa: 300 });
  assert.deepEqual([beyond.text, beyond.note, beyond.tone], ['+3.00 bps', 'disagrees', 'bad']);
  assert.match(beyond.title, /disagrees/);
  // the day table's comparisons carry no `equal`: the two paisa values decide it, exactly
  assert.equal(cmpCell({ bps: 0.0, flag: false }, 1000000, 1000000).text, '=');
  assert.equal(cmpCell({ bps: 0.0, flag: false }, 1000000, 1000001).text, '0.00 bps'); // rounds to 0.00 and is still not "="
  assert.equal(cmpCell({ bps: 0.0, flag: false }, 1000000, 1000001).tone, 'quiet');
  for (const none of [null, undefined, 'x', 7]) assert.deepEqual([cmpCell(none).text, cmpCell(none).tone], ['', 'quiet']);
  assert.equal(cmpCell({}).text, '');
});

test('official against last print is labelled and is never a disagreement, however far apart', () => {
  const cell = cmpCell({ bps: -9.38, flag: false, semantics: 'official_vs_last_print' }, 1161300, 1162390);
  assert.deepEqual([cell.text, cell.note], ['−9.38 bps', 'official vs last print']);
  assert.equal(cell.tone, 'info');
  assert.match(cell.title, /never counted as a disagreement \(D-3324\)/);
  const far = cmpCell({ bps: 250.0, flag: false, semantics: 'official_vs_last_print' }, 1025000, 1000000);
  assert.notEqual(far.tone, 'bad');
  const equal = cmpCell({ bps: 0.0, flag: false, semantics: 'official_vs_last_print' }, 2306310, 2306310);
  assert.deepEqual([equal.text, equal.tone], ['=', 'ok']);
  assert.match(equal.title, /official close and the last print are the same price/);
});

test('where the bases differ a factor stands in place of a disagreement', () => {
  const differs = cmpCell({ why: 'basis_differs' });
  assert.deepEqual([differs.text, differs.tone], ['basis differs', 'info']);
  const volume = cmpCell({ factor: 2.0992, why: 'basis_differs' });
  assert.deepEqual([volume.text, volume.note], ['basis differs', '×2.0992']);
  const unmeasured = cmpCell({ bps: 10986.19, flag: null, why: 'basis_unmeasured' });
  assert.deepEqual([unmeasured.text, unmeasured.tone], ['basis not measured', 'warn']);
  assert.match(unmeasured.title, /which is the basis, not a disagreement/);
  assert.notEqual(unmeasured.tone, 'bad');
  assert.deepEqual([cmpCell({ why: 'index_no_volume' }).text, cmpCell({ why: 'gdfl_first_print_after_open' }).tone], ['an index has no volume', 'warn']);
  assert.deepEqual(factorCell(2.0982, "GDFL first print ÷ Zerodha day open (D-3323's measure)", { differs: true }), { text: '×2.0982', tone: 'info', title: "GDFL first print ÷ Zerodha day open (D-3323's measure)" });
  assert.deepEqual([factorCell(1, 'x').text, factorCell(1, 'x').tone], ['×1.0000', 'quiet']);
  assert.equal(factorCell(2.5, 'x', { differs: true, step: true }).tone, 'warn');
  assert.deepEqual(factorCell(null, 'unmeasured: no pair is the same instant'), { text: 'not measured', tone: 'warn', title: 'unmeasured: no pair is the same instant' });
  assert.deepEqual(factorCell(null, null), { text: '', tone: 'quiet', title: 'no factor on this row' });
  assert.equal(factorCell(Number.NaN, null).text, '');
  // a volume comparison is a percent
  assert.deepEqual([cmpCell({ percent: -0.02, flag: false }).text, cmpCell({ percent: -0.02, flag: false }).tone], ['−0.02%', 'quiet']);
  assert.deepEqual([cmpCell({ percent: 7.89, flag: true }).text, cmpCell({ percent: 7.89, flag: true }).tone], ['+7.89%', 'bad']);
  assert.equal(cmpCell({ percent: 0, flag: false, equal: true }).text, '=');
});

// ══ The timeline ═════════════════════════════════════════════════════════════

test('every bucket of the timeline has one row per feed, and every row is tagged by its source', () => {
  const view = timeline(barsBody(), rungTable(rungsBody()));
  assert.equal(view.groups.length, 3);
  for (const g of view.groups) {
    assert.deepEqual(g.rows.map((/** @type {any} */ r) => r.feed), ['gdfl', 'zerodha']);
    assert.deepEqual(g.rows.map((/** @type {any} */ r) => r.tag), ['gdfl 1s→1min fold', 'zerodha 1min stored']);
  }
  assert.deepEqual(view.sources.map((/** @type {any} */ s) => s.tag), ['gdfl 1s→1min fold', 'zerodha 1min stored']);
  const [first] = view.groups;
  assert.equal(first.label, '2020-03-20 15:29:00');
  assert.deepEqual([first.rows[0].c, first.rows[1].c], ['10,000.00', '10,000.00']);
  assert.deepEqual([first.rows[0].count, first.rows[0].from, first.rows[0].close], ['60 s folded', '15:29:00', 'last print']);
  assert.equal(first.rows[0].oi, 'not a derivative');
  assert.deepEqual([first.cmp?.label, first.cmp?.c.text, first.cmp?.v.text], ['gdfl against zerodha', '=', 'an index has no volume']);
  // each source row links to ITS feed's stored rows of that day on /db: the seconds for the fold, the minutes for Zerodha
  assert.equal(first.rows[0].db, '/db?feed=gdfl&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=1s&from=2020-03-20&to=2020-03-20');
  assert.equal(first.rows[1].db, '/db?feed=zerodha&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=1min&from=2020-03-20&to=2020-03-20');
  // without a rung table there is no link, and nothing else changes
  assert.equal(timeline(barsBody()).groups[0].rows[0].db, null);
});

test('a bucket one feed lacks says so on that feed’s row, and a disagreement is flagged', () => {
  const view = timeline(barsBody());
  const [, missing, disagree] = view.groups;
  assert.equal(missing.only, 'zerodha');
  assert.deepEqual([missing.rows[0].absent, missing.rows[0].c, missing.rows[1].absent, missing.rows[1].c], ['no bar in this bucket', '', '', '10,002.00']);
  assert.equal(missing.cmp, null);
  assert.deepEqual([disagree.cmp?.c.text, disagree.cmp?.c.tone], ['+3.00 bps', 'bad']);
  assert.deepEqual(disagree.flags.map((/** @type {any} */ f) => [f.code, f.tone]), [['c', 'bad'], ['a_flag_from_the_future', 'warn']]);
  assert.deepEqual(view.summary, [
    '3 buckets on this page: 2 held by both feeds, 1 by one',
    '1 of 2 compared on one basis are within tolerance, 1 disagree',
    'exactly equal: open 2, high 2, low 2, close 1 of 2'
  ]);
  assert.deepEqual([view.nextCursor, view.complete, view.recordsRead, view.showFactor, view.dir], ['1584697500', false, 187, false, 'desc']);
});

test('a share timeline shows the factor column, and an option with one feed compares nothing', () => {
  const share = barsBody();
  share.sources[0].basis = RAW;
  share.sources[1].basis = ADJUSTED;
  share.rows = [{ t: 1, ist: '2020-03-20 15:29:00', only: null, flags: ['basis_differs'],
    cells: { gdfl: cellOf(50000, 'gdfl_fold', { v: 1000 }), zerodha: cellOf(25000, 'zerodha_stored', { v: 2000 }) },
    cmp: { base: { against: 'gdfl', of: 'zerodha' }, basis: 'basis_differs', factor: 2.0, factor_of: 'GDFL ÷ Zerodha close of this bucket', o: { why: 'basis_differs' }, h: { why: 'basis_differs' }, l: { why: 'basis_differs' }, c: { why: 'basis_differs' }, v: { factor: 2.0, why: 'basis_differs' } } }];
  const view = timeline(share);
  assert.equal(view.showFactor, true);
  assert.deepEqual([view.groups[0].cmp?.factor.text, view.groups[0].cmp?.factor.tone, view.groups[0].cmp?.c.text], ['×2.0000', 'info', 'basis differs']);
  assert.notEqual(view.groups[0].cmp?.c.tone, 'bad');
  // same price basis on both feeds and a factor of 1: still a factor column, because the bases are worded differently
  share.rows[0].cmp = { base: { against: 'gdfl', of: 'zerodha' }, basis: 'same', factor: 1.0, factor_of: 'x', o: same, h: same, l: same, c: same, v: { percent: 0, flag: false, equal: true } };
  assert.equal(timeline(share).showFactor, true);

  const option = barsBody();
  option.sources = [option.sources[0]];
  option.rows = [{ t: 2, ist: '2020-03-20 15:29:00', only: null, flags: [], cells: { gdfl: cellOf(7000, 'gdfl_fold', { oi: 789000, oi_why: null, v: 87000 }) }, cmp: null }];
  const one = timeline(option);
  assert.deepEqual([one.groups[0].rows.length, one.groups[0].cmp, one.groups[0].rows[0].oi, one.groups[0].rows[0].v, one.showFactor], [1, null, '7,89,000', '87,000', false]);
  // the open-interest column is drawn only where a cell carries one: an option's page, never an index's or a share's
  assert.deepEqual([one.showOi, timeline(share).showOi, timeline(barsBody()).showOi, timeline(null).showOi], [true, false, false, false]);
  assert.match(PAGE, /\{#if bars\.showOi\}<th scope="col" class="s-num">Open interest<\/th>\{\/if\}/);
});

test('an empty page, a duplicated bucket, out-of-order rows and a maximum page are drawn as sent', () => {
  const empty = barsBody();
  empty.rows = [];
  empty.next_cursor = /** @type {any} */ (null);
  empty.complete = true;
  const none = timeline(empty);
  assert.equal(none.empty, 'No source holds a 1min bucket of NSE-INDEX-TESTIDX in 2020-03-20.');
  assert.deepEqual([none.nextCursor, none.complete], ['', true]);
  for (const garbage of [null, undefined, {}, { rows: 'x', sources: 3 }]) assert.match(timeline(garbage).empty, /No source holds a/);

  // a bucket the server sent twice is drawn twice; rows keep the server's order, never re-sorted here
  const dup = barsBody();
  dup.rows = [dup.rows[2], dup.rows[0], dup.rows[0]];
  assert.deepEqual(timeline(dup).groups.map((/** @type {any} */ g) => g.label), ['2020-03-20 15:27:00', '2020-03-20 15:29:00', '2020-03-20 15:29:00']);
  assert.doesNotMatch(PAGE, /\{#each [^}]* \((?:g|d|r|f|m)\.[a-z]+\)\}/, 'no row is keyed by a value the server could send twice');

  const big = barsBody();
  big.rows = Array.from({ length: 200 }, (_, i) => ({ ...big.rows[0], t: 1584698340 - 60 * i }));
  const view = timeline(big);
  assert.equal(view.groups.length, 200);
  assert.equal(view.groups.reduce((/** @type {number} */ n, /** @type {any} */ g) => n + g.rows.length, 0), 400);
  // a cell with fields missing is dashes, not a crash and not a zero
  const torn = barsBody();
  torn.rows = [{ t: 3, ist: '2020-03-20 15:29:00', flags: /** @type {any} */ (null), only: null, cells: /** @type {any} */ ({ gdfl: {}, zerodha: 'x' }), cmp: /** @type {any} */ ('x') }];
  const t = timeline(torn).groups[0];
  assert.deepEqual([t.rows[0].o, t.rows[0].v, t.rows[0].oi, t.rows[1].absent, t.cmp, t.flags.length], ['—', '—', '—', 'no bar in this bucket', null, 0]);
});

// ══ The day table ════════════════════════════════════════════════════════════

test('each day shows the official bar, the minutes folded and the seconds folded as three tagged rows', () => {
  const view = dayTable(daysBody([indexDay('2020-03-19', 1000500, 1000400), indexDay('2020-03-20', 1000000, 1000000)]), rungTable(rungsBody()));
  assert.equal(view.days.length, 2);
  for (const d of view.days) {
    assert.deepEqual(d.rows.map((/** @type {any} */ r) => r.tag), ['zerodha 1day stored', 'zerodha 1min→1day fold', 'gdfl 1s→1day fold']);
    assert.deepEqual(d.rows.map((/** @type {any} */ r) => r.close), ['official close', 'last print', 'last print']);
    assert.deepEqual(d.cmps.map((/** @type {any} */ c) => c.label), ['gdfl 1s→1day fold against zerodha 1day stored', 'zerodha 1min→1day fold against zerodha 1day stored']);
  }
  const [differs, equal] = view.days;
  assert.deepEqual(differs.rows.map((/** @type {any} */ r) => r.c), ['10,005.00', '10,004.00', '10,004.00']);
  assert.deepEqual(differs.rows.map((/** @type {any} */ r) => r.count), ['', '375 min', '21,000 s']);
  assert.deepEqual([differs.rows[2].from, differs.rows[2].to], ['09:15:00', '15:29:59']);
  // each row links to its own feed's stored rows of THAT day on /db
  assert.deepEqual(differs.rows.map((/** @type {any} */ r) => r.db), [
    '/db?feed=zerodha&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=1day&from=2020-03-19&to=2020-03-19',
    '/db?feed=zerodha&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=1min&from=2020-03-19&to=2020-03-19',
    '/db?feed=gdfl&seg=INDEX&sym=TESTIDX&key=NSE-INDEX-TESTIDX&tf=1s&from=2020-03-19&to=2020-03-19'
  ]);
  assert.equal(differs.timeline, '/sources?key=NSE-INDEX-TESTIDX&day=2020-03-19');
  assert.deepEqual([differs.label, differs.sub], ['2020-03-19', 'Fri']);
  // an index's volume is not compared, and the row says why instead of showing a blank
  assert.equal(differs.cmps[0].v.text, 'an index has no volume');
  assert.deepEqual(differs.marks, [{ tone: 'quiet', text: 'Zerodha holds 375 bar(s)' }]);
  assert.deepEqual([equal.closeEqual, equal.closes], [true, { official: '10,000.00', minute: '10,000.00', gdfl: '10,000.00' }]);
  assert.equal(dayTable(daysBody([indexDay('2020-03-19', 1000500, 1000400)])).days[0].rows[0].db, null);
});

test('a close that differs is labelled official against last print on every day, and an equal one is "="', () => {
  const body = daysBody(Array.from({ length: 19 }, (_, i) => indexDay(`2020-03-${String(i + 1).padStart(2, '0')}`, 1000000 + 100 * (i + 1), 1000000)));
  const view = dayTable(body);
  assert.equal(view.days.length, 19);
  for (const d of view.days) {
    const [gdflVsDay, minuteVsDay] = d.cmps;
    assert.match(gdflVsDay.c.text, /^−\d+\.\d\d bps$/);
    assert.equal(gdflVsDay.c.note, 'official vs last print');
    assert.equal(gdflVsDay.c.tone, 'info');
    assert.equal(minuteVsDay.c.tone, 'info');
    assert.deepEqual([gdflVsDay.o.text, gdflVsDay.h.text, gdflVsDay.l.text], ['=', '=', '=']);
    assert.deepEqual(d.flags.map((/** @type {any} */ f) => [f.code, f.tone]), [['official_vs_last_print', 'info']]);
    assert.equal(d.closeEqual, false);
  }
  assert.deepEqual(view.summary, [
    '19 days on this page: 19 held by both feeds, 0 by Zerodha only, 0 by GDFL only, 0 by neither',
    'open equal on 19 of 19, high on 19, low on 19',
    'close equal on 0 of 19 and different on 19 — official (Zerodha 1day) against last print (GDFL seconds): labelled, never a disagreement (D-3324)'
  ]);
  const equal = dayTable(daysBody([indexDay('2020-03-20', 1000000, 1000000)], { close_equal: 1, close_differs: 0 }));
  assert.deepEqual([equal.days[0].cmps[0].c.text, equal.days[0].cmps[0].c.tone, equal.days[0].flags.length], ['=', 'ok', 0]);
  assert.match(equal.summary[2], /^close equal on 1 of 1 and different on 0/);
});

test('a share day shows its factor instead of a disagreement, and the minutes against their own day bar', () => {
  const d = indexDay('2020-03-20', 25100, 50000);
  d.basis = /** @type {any} */ ({ bases: 'differ', basis_differs: true, basis_step: false, basis_step_why: null, factor: 2.0, factor_of: "GDFL first print ÷ Zerodha day open (D-3323's measure)", minute_factor: 2.01, minute_factor_at: '15:29:00', minute_basis_differs_from_day: true, volume_factor: 2.1, volume_not_scaled_with_price: true });
  d.flags = ['minute_open_mismatch', 'minute_extremes_outside_day', 'official_vs_last_print', 'minute_volume_off_day', 'basis_differs', 'minute_basis_differs_from_day'];
  d.open.gdfl_vs_zerodha_day = /** @type {any} */ ({ why: 'basis_differs' });
  d.high.gdfl_vs_zerodha_day = /** @type {any} */ ({ why: 'basis_differs' });
  d.low.gdfl_vs_zerodha_day = /** @type {any} */ ({ why: 'basis_differs' });
  d.close.official_vs_gdfl_last_print = /** @type {any} */ ({ why: 'basis_differs' });
  d.open.zerodha_minute_vs_day = { bps: -93.26, flag: true };
  d.open.zerodha_first_minute = 990674;
  d.low.minute_below_day = true;
  d.volume = /** @type {any} */ ({ gdfl_sum: 1000, zerodha_day: 2100, zerodha_minutes_sum: 2112, gdfl_vs_zerodha_day: { factor: 2.1, why: 'basis_differs' }, minute_over_day: 1.0059, minute_vs_day: { percent: 0.59, flag: true } });
  const view = dayTable(daysBody([d], { basis_differs: 1, minute_extremes_outside_day: 1, minute_volume_off_day: 1, close_equal: 0, close_differs: 0 }));
  const [day] = view.days;
  assert.equal(view.showFactor, true);
  assert.deepEqual([day.factor?.text, day.factor?.tone, day.factor?.title], ['×2.0000', 'info', "GDFL first print ÷ Zerodha day open (D-3323's measure)"]);
  assert.deepEqual([day.factor?.volume, day.factor?.minute], ['volume ×2.1000 · not scaled with price', 'minutes ×2.0100 at 15:29:00']);
  const [gdflVsDay, minuteVsDay] = day.cmps;
  for (const cell of [gdflVsDay.o, gdflVsDay.h, gdflVsDay.l, gdflVsDay.c]) assert.deepEqual([cell.text, cell.tone], ['basis differs', 'info']);
  assert.deepEqual([gdflVsDay.v.text, gdflVsDay.v.note], ['basis differs', '×2.1000']);
  assert.deepEqual([minuteVsDay.o.text, minuteVsDay.o.tone], ['−93.26 bps', 'bad']);
  assert.deepEqual([minuteVsDay.h.text, minuteVsDay.l.text, minuteVsDay.l.note, minuteVsDay.l.tone], ['inside', 'below', 'the day range', 'warn']);
  assert.deepEqual([minuteVsDay.v.text, minuteVsDay.v.note, minuteVsDay.v.tone], ['÷ day 1.0059', '+0.59%', 'bad']);
  assert.deepEqual(day.flags.map((/** @type {any} */ f) => f.tone), ['warn', 'warn', 'info', 'warn', 'info', 'warn']);
  assert.deepEqual(view.summary.slice(1), [
    '1 day on a different price basis: the factor is shown instead of a disagreement',
    "Zerodha's minutes go outside its own day range on 1 day",
    "Zerodha's minute volume is off its day volume on 1 day"
  ]);
  // an index page has no factor column at all
  assert.equal(dayTable(daysBody([indexDay('2020-03-20', 1000000, 1000000)])).showFactor, false);
  // a day whose factor could not be read on one instant says so
  d.basis = /** @type {any} */ ({ basis_differs: null, factor: null, factor_of: 'unmeasured: no pair is the same instant' });
  assert.deepEqual([dayTable(daysBody([d])).days[0].factor?.text, dayTable(daysBody([d])).days[0].factor?.tone], ['not measured', 'warn']);
});

test('a day a source does not hold names the reason: closed, after the archive, or not sent', () => {
  const after = indexDay('2020-03-23', 1000000, 1000000);
  after.absent = { gdfl_seconds: 'after_archive' };
  after.gdfl_seconds_fold = /** @type {any} */ (null);
  after.close.official_vs_gdfl_last_print = /** @type {any} */ (null);
  after.open.gdfl_vs_zerodha_day = after.high.gdfl_vs_zerodha_day = after.low.gdfl_vs_zerodha_day = /** @type {any} */ (null);
  const closed = indexDay('2020-03-21', 0, 0);
  closed.absent = { gdfl_seconds: 'after_archive', zerodha_day: 'closed', zerodha_minutes: 'closed' };
  closed.zerodha_day = closed.zerodha_minutes_fold = closed.gdfl_seconds_fold = /** @type {any} */ (null);
  // the server sends nulls for every comparison of a day nothing holds
  closed.open = /** @type {any} */ ({ gdfl_vs_zerodha_day: null, zerodha_minute_vs_day: null });
  closed.high = /** @type {any} */ ({ gdfl_vs_zerodha_day: null, minute_above_day: null });
  closed.low = /** @type {any} */ ({ gdfl_vs_zerodha_day: null, minute_below_day: null });
  closed.close = /** @type {any} */ ({ equal: null, official: null, gdfl_last_print: null, zerodha_last_minute: null, official_vs_gdfl_last_print: null, official_vs_zerodha_last_minute: null });
  closed.volume = /** @type {any} */ ({ gdfl_sum: null, zerodha_day: null, why: 'index_no_volume' });
  closed.marks = /** @type {any} */ ({ calendar: { kind: 'closed', weekday: 'Sat', holiday: null, special: null, halted_minutes: 0, unverified_minutes: 0 }, degraded: ['gap ledger unreadable'], says: ['closed: a weekend'] });
  const bare = { day: '2020-03-22' };
  const view = dayTable(daysBody([after, closed, bare]));
  assert.deepEqual(view.days[0].rows.map((/** @type {any} */ r) => r.absent), ['', '', 'after the feed’s archive ends']);
  // a day NO source holds is ONE row that still names every source's own reason
  assert.deepEqual(view.days[1].rows.map((/** @type {any} */ r) => [r.tag, r.absent]), [['every source', 'zerodha 1day stored, zerodha 1min→1day fold: the exchange was closed; gdfl 1s→1day fold: after the feed’s archive ends']]);
  assert.deepEqual([view.days[1].cmps.length, view.days[1].sub], [0, 'Sat']);
  // a comparison with nothing in it is not drawn: only the minutes-against-day row is left on the first day
  assert.deepEqual(view.days[0].cmps.map((/** @type {any} */ c) => c.label), ['zerodha 1min→1day fold against zerodha 1day stored']);
  assert.deepEqual(view.days[1].marks, [
    { tone: 'info', text: 'closed' },
    { tone: 'quiet', text: 'closed: a weekend' },
    { tone: 'warn', text: 'degraded: gap ledger unreadable' }
  ]);
  // a day record with nothing in it is three "no record" rows, not a crash
  assert.deepEqual(view.days[2].rows.map((/** @type {any} */ r) => r.absent), ['no source holds this day: no record of this day']);
  assert.deepEqual([view.days[2].flags.length, view.days[2].marks.length, view.days[2].factor], [0, 0, null]);
  for (const garbage of [null, undefined, {}, { days: 'x' }]) assert.match(dayTable(garbage).empty, /The day table holds no day of/);
  assert.deepEqual(marksOf(null), []);
  assert.deepEqual(marksOf({ calendar: { kind: 'special_session', special: 'Muhurat', holiday: 'Diwali', halted_minutes: 45, unverified_minutes: 1 } }), [
    { tone: 'info', text: 'special session: Diwali, Muhurat' },
    { tone: 'warn', text: 'halted for 45 minutes' },
    { tone: 'warn', text: '1 minute the calendar cannot verify' }
  ]);
});

test('a /db link is narrowed to one day through /db’s own address codec', () => {
  assert.equal(dbDayHref('/db?feed=gdfl&seg=INDEX&sym=NIFTY&key=NSE-INDEX-NIFTY&tf=1s', '2019-03-29'), '/db?feed=gdfl&seg=INDEX&sym=NIFTY&key=NSE-INDEX-NIFTY&tf=1s&from=2019-03-29&to=2019-03-29');
  assert.equal(dbDayHref('/db?feed=gdfl&tf=1s', ''), '/db?feed=gdfl&tf=1s');
  assert.equal(dbDayHref('/db?feed=gdfl&tf=1s', 'not a day'), '/db?feed=gdfl&tf=1s');
  for (const none of [null, undefined, 7, 'https://elsewhere.example/db', 'javascript:alert(1)']) assert.equal(dbDayHref(none, '2019-03-29'), null);
});

test('the tolerances are the server’s and are said to be chosen, not sourced', () => {
  assert.equal(
    toleranceLine({ price_bps: 1, volume_percent: 0.5, factor_percent: 0.2 }),
    "Tolerances (chosen, not sourced; D-3339): a price disagrees past 1 bp, a volume past 0.5%, a share's factor differs past 0.2%."
  );
  assert.match(toleranceLine({}), /past not sent, a volume past not sent/);
  assert.equal(toleranceLine(null), '');
});

// ══ The page ═════════════════════════════════════════════════════════════════

test('the page compiles, draws the three parts and reads each from its own route through `ask`', () => {
  const out = compile(PAGE, { filename: '+page.svelte', generate: 'client' });
  assert.ok(out.js.code.length > 1000);
  for (const section of ['rungs', 'timeline', 'days']) assert.match(PAGE, new RegExp(`data-section="${section}"`));
  for (const fn of ['rungsUrl(key)', 'barsUrl({ key, rung, day, dir, cursor })', 'dayUrl(key, month)']) assert.ok(PAGE.includes(fn), fn);
  assert.match(PAGE, /await ask\(url, \{ signal: ticket\.signal \}\)/);
  // GET only, and no request that bypasses the deadline
  assert.doesNotMatch(PAGE, /\bfetch\(/);
  assert.doesNotMatch(PAGE, /method:|POST|PUT|DELETE/);
  // a refusal and a failure are drawn, each with the request that produced it
  assert.match(PAGE, /r\.phase === 'refused'/);
  assert.match(PAGE, /r\.phase === 'failed'/);
  // an answer to an earlier question is never drawn under a later one
  for (const guard of ['rungsRead.url === rungsAsk', 'barsRead.url === barsAsk', 'daysRead.url === daysAsk']) assert.ok(PAGE.includes(guard), guard);
  assert.match(PAGE, /owner\.cancel\(\);/);
});

test('the page keeps no list the data already knows: no feed, rung or instrument is spelled in its markup', () => {
  const script = PAGE.slice(PAGE.indexOf('</script>'));
  for (const word of ['zerodha', 'gdfl', 'NIFTY', 'RELIANCE', '5min', '1day']) assert.doesNotMatch(script, new RegExp(`['"\`>]${word}['"\`<]`), word);
  // the timeline's rung choices come from the rung table's rows
  assert.match(PAGE, /new Set\(rungs \? rungs\.rows\.map\(\(\/\*\* @type \{any\} \*\/ r\) => r\.rung\) : \[\]\)/);
  // every source row carries its tag and, where the feed stores it, its own /db link
  assert.match(PAGE, /<a class="s-tag" href=\{r\.db\}/);
  assert.match(PAGE, /data-flag=\{f\.code\}/);
});
