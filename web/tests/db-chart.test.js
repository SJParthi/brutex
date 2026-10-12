// THE /db CHART PANEL'S PLAN (WP-57; DB-080, RA-175).
//
// Each test drives the real module the panel imports. The census plans, bars,
// rung tables, greeks and day records below are invented shapes, with prices
// nobody could take for real ones; no vendor row is quoted.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  chartSubject, planChoice, rungChoices, sourcesUrl, pageRequest, pageBars, joinBars, keepRefusal, rupees,
  greekPlan, fileGreeks, greekLine, flagSpan, dayBands, holeBands, bandSlots, flagMarkers, tradeDriven, alphaHex,
  costText, CHART_PAGE, FOLD_PAGE, FOLD_PAGE_DAY, CHART_KEEP, CHART_AUTO_ALL, GREEK_REQUESTS, PAN_REQUESTS,
  OPEN_REQUESTS, BAND_SLOTS, GAP_BANDS, GREEK_CHUNK, DAYFLAGS_PAGE_DAYS, GREEK_LINES
} from '../src/lib/db/chart.js';
import { istInstant, WINDOW_PAGE_MAX } from '../src/lib/db/bars-plan.js';
import { greekKey } from '../src/lib/greeks.js';
import { dbSource } from './database-page-fixture.js';

const CE = 'NSE-FNO-NIFTY-2019-03-28-1150000-CE';
const IDX = 'NSE-INDEX-NIFTY';
const FEED_A = 'feed_a'; // a feed that stores seconds and folds the rest
const FEED_B = 'feed_b'; // a feed that stores minutes and days

/** @param {string[]} keys @param {string} [timeframe] */
const plan = (keys, timeframe = '1s', from = '2019-03', to = '2019-03') => ({ keys, timeframe, from, to, why: null });
/** One invented bar. @param {number} t @param {Partial<{o:number,h:number,l:number,c:number,v:number,oi:number|null,phase:string}>} [more] */
const row = (t, more = {}) => ({ t, o: 1111, h: 2222, l: 1010, c: 2020, v: 7, oi: null, phase: 'continuous', ...more });
/** Unix seconds of an IST day and clock. @param {string} day @param {string} clock */
const at = (day, clock) => /** @type {number} */ (istInstant(day, clock));
/** A chart bar as `pageBars` makes one. @param {number} t @param {object} [more] */
const bar = (t, more = {}) => ({ t, o: 1111, h: 2222, l: 1010, c: 2020, v: 7, oi: null, phase: null, at: t, seconds: null, ...more });

/** The rung table one feed pair answers with. */
const TABLE = {
  rungs: [
    { feed: FEED_A, rung: '1s', kind: 'stored', stored: true, first_month: '2018-09', last_month: '2019-03', first_day: '2018-09-19', last_day: '2019-03-28', state: 'held', tag: 'a_1s' },
    { feed: FEED_A, rung: '1day', kind: 'fold', stored: false, derived_from: '1s', first_month: '2018-09', last_month: '2019-03', first_day: '2018-09-19', last_day: '2019-03-28', state: 'on_request', tag: 'a_fold' },
    { feed: FEED_A, rung: '1min', kind: 'fold', stored: false, derived_from: '1s', first_month: '2018-09', last_month: '2019-03', first_day: '2018-09-19', last_day: '2019-03-28', state: 'on_request', tag: 'a_fold' },
    { feed: FEED_B, rung: '1min', kind: 'stored', stored: true, first_month: '2018-01', last_month: '2026-09', first_day: '2018-01-01', last_day: '2026-09-30', state: 'held', tag: 'b_stored' },
    { feed: FEED_B, rung: '5min', kind: 'derived', stored: true, first_month: '2018-01', last_month: '2026-09', first_day: '2018-01-01', last_day: '2026-09-30', state: 'partial', tag: 'b_derived' },
    { feed: FEED_B, rung: '1day', kind: 'stored', stored: true, first_month: '2015-01', last_month: '2026-09', first_day: '2015-01-01', last_day: '2026-09-30', state: 'held', tag: 'b_stored' }
  ]
};

test('the bounds a page keeps: one bar request, four greeks requests, one day-flag request', () => {
  assert.equal(CHART_PAGE, WINDOW_PAGE_MAX, 'a stored page is the window route\'s own maximum');
  assert.equal(GREEK_CHUNK, 400);
  assert.equal(PAN_REQUESTS, 1 + GREEK_REQUESTS + 1);
  assert.equal(PAN_REQUESTS, 6);
  assert.equal(OPEN_REQUESTS, 1 + Math.ceil(CHART_AUTO_ALL / CHART_PAGE) * PAN_REQUESTS);
  assert.ok(CHART_AUTO_ALL <= CHART_KEEP);
  assert.equal(costText(3, 41.6), '3 of at most 6 request(s), 42 ms');
});

test('the chart draws one series: none says nothing, several are refused by count, a refused plan repeats its reason', () => {
  assert.deepEqual([chartSubject(null).key, chartSubject(null).why], ['', null]);
  assert.deepEqual([chartSubject(plan([])).key, chartSubject(plan([])).why], ['', null]);
  const many = chartSubject(plan([CE, CE.replace('-CE', '-PE'), IDX]));
  assert.equal(many.key, '');
  assert.match(String(many.why), /one series and this selection reaches 3/);
  const refused = chartSubject({ ...plan([CE]), why: 'this selection reaches 2 rungs' });
  assert.equal(refused.why, 'this selection reaches 2 rungs');
  const garbage = chartSubject(plan(['not a key']));
  assert.equal(garbage.key, '');
  assert.match(String(garbage.why), /could not be read/);
});

test('a series is named to the server by its own parts: an index, a hyphenated share, an option, a future', () => {
  const idx = chartSubject(plan([IDX]));
  assert.deepEqual([idx.key, idx.contract, idx.option], [IDX, '', false]);
  assert.equal(sourcesUrl(idx), '/sources.json?exchange=NSE&segment=INDEX&symbol=NIFTY');
  const share = chartSubject(plan(['NSE-CASH-BAJAJ-AUTO']));
  assert.equal(sourcesUrl(share), '/sources.json?exchange=NSE&segment=CASH&symbol=BAJAJ-AUTO');
  const opt = chartSubject(plan([CE]));
  assert.deepEqual([opt.contract, opt.option], ['2019-03-28-1150000-CE', true]);
  assert.equal(sourcesUrl(opt), '/sources.json?exchange=NSE&segment=FNO&symbol=NIFTY&contract=2019-03-28-1150000-CE');
  const fut = chartSubject(plan(['NSE-FNO-M&M-2019-03-28-FUT']));
  assert.deepEqual([fut.contract, fut.option], ['2019-03-28-FUT', false]);
  assert.equal(new URLSearchParams(sourcesUrl(fut).split('?')[1]).get('symbol'), 'M&M');
});

test('the selection\'s own rung is charted from the census plan alone, through the window route', () => {
  const c = planChoice(plan([CE], '1s', '2018-09', '2019-03'));
  assert.deepEqual([c?.rung, c?.secs, c?.source, c?.firstMonth, c?.lastMonth], ['1s', 1, 'window', '2018-09', '2019-03']);
  assert.equal(planChoice(plan([CE], 'fortnight')), null, 'a rung with no known length is not charted');
});

test('the rungs on offer are the server\'s table for the active feed: stored through the window route, folds through the fold route', () => {
  const a = rungChoices(TABLE, FEED_A);
  assert.deepEqual(a.choices.map((c) => [c.rung, c.source, c.from]), [['1s', 'window', ''], ['1min', 'fold', '1s'], ['1day', 'fold', '1s']]);
  const b = rungChoices(TABLE, FEED_B);
  assert.deepEqual(b.choices.map((c) => [c.rung, c.source, c.kind]), [['1min', 'window', 'stored'], ['5min', 'window', 'derived'], ['1day', 'window', 'stored']]);
  assert.equal(b.choices[2].firstMonth, '2015-01');
  assert.deepEqual(rungChoices(TABLE, 'feed_c').choices, [], 'a feed the table does not name is offered nothing');
});

test('a rung row that cannot be charted is left out and counted, and a stored rung outranks a fold of the same length', () => {
  const odd = {
    rungs: [
      { feed: FEED_A, rung: '7min', kind: 'stored', stored: true, first_month: '2019-01', last_month: '2019-03' },
      { feed: FEED_A, rung: '1min', kind: 'stored', stored: true, first_month: '', last_month: '2019-03' },
      { feed: FEED_A, rung: '5min', kind: 'unexpected', stored: false, first_month: '2019-01', last_month: '2019-03' },
      { feed: FEED_A, rung: '1day', kind: 'fold', stored: false, derived_from: '1s', first_month: '2019-01', last_month: '2019-03' },
      { feed: FEED_A, rung: '1day', kind: 'stored', stored: true, first_month: '2019-02', last_month: '2019-03' },
      { feed: FEED_A, rung: '1day', kind: 'fold', stored: false, derived_from: '1s', first_month: '2019-01', last_month: '2019-03' }
    ]
  };
  const got = rungChoices(odd, FEED_A);
  assert.equal(got.skipped, 3);
  assert.deepEqual(got.choices.map((c) => [c.rung, c.source, c.firstMonth]), [['1day', 'window', '2019-02']]);
  assert.throws(() => rungChoices({ error: 'no such instrument' }, FEED_A), /without a rung table: no such instrument/);
  assert.throws(() => rungChoices(null, FEED_A), /without a rung table/);
});

test('a stored page is one window request: newest first, the page maximum, the offset, and the selection\'s own bounds', () => {
  const subject = chartSubject(plan([CE]));
  const choice = /** @type {any} */ (planChoice(plan([CE], '1s', '2018-09', '2019-03')));
  const first = pageRequest(FEED_A, subject, choice, { fromDay: '2019-03-20', toDay: '2019-03-20', fromTime: '09:15', toTime: '09:20' }, null);
  assert.equal(first.route, '/bars/window.json');
  assert.equal(first.why, null);
  const q = new URLSearchParams(first.query);
  assert.deepEqual(
    [q.get('feed'), q.get('timeframe'), q.get('from'), q.get('to'), q.getAll('key'), q.get('sort'), q.get('dir'), q.get('offset'), q.get('limit')],
    [FEED_A, '1s', '2019-03', '2019-03', [CE], 'ts', 'desc', '0', String(CHART_PAGE)]
  );
  assert.deepEqual([q.get('from_day'), q.get('to_day'), q.get('tod_from'), q.get('tod_to'), q.get('extremes')], ['2019-03-20', '2019-03-20', '09:15:00', '09:20:59', '0']);
  const next = new URLSearchParams(pageRequest(FEED_A, subject, choice, {}, 3000).query);
  assert.deepEqual([next.get('offset'), next.get('from'), next.get('to'), next.get('from_day')], ['3000', '2018-09', '2019-03', null]);
});

test('a day rung has no time of day to bound, and says the bound was not applied', () => {
  const subject = chartSubject(plan([IDX], '1min'));
  const day = rungChoices(TABLE, FEED_B).choices[2];
  const got = pageRequest(FEED_B, subject, day, { fromTime: '09:15', toTime: '10:00' }, null);
  const q = new URLSearchParams(got.query);
  assert.deepEqual([q.get('timeframe'), q.get('tod_from'), q.get('tod_to')], ['1day', null, null]);
  assert.deepEqual(got.notes, ['the time-of-day bound is not applied to a day rung']);
});

test('a folded page is one fold request: 200 buckets, 31 at the day rung, continued by the server\'s cursor', () => {
  const subject = chartSubject(plan([CE]));
  const [, minute, day] = rungChoices(TABLE, FEED_A).choices;
  const first = pageRequest(FEED_A, subject, minute, { fromDay: '2019-03-20', toDay: '2019-03-20' }, null);
  assert.equal(first.route, '/sources/bars.json');
  const q = new URLSearchParams(first.query);
  assert.deepEqual(
    [q.get('exchange'), q.get('segment'), q.get('symbol'), q.get('contract'), q.get('rung'), q.get('from'), q.get('to'), q.get('dir'), q.get('limit'), q.get('cursor')],
    ['NSE', 'FNO', 'NIFTY', '2019-03-28-1150000-CE', '1min', '2019-03-20', '2019-03-20', 'desc', String(FOLD_PAGE), null]
  );
  const more = pageRequest(FEED_A, subject, day, {}, 1549823400);
  const d = new URLSearchParams(more.query);
  assert.deepEqual([d.get('rung'), d.get('from'), d.get('to'), d.get('limit'), d.get('cursor')], ['1day', '2018-09-19', '2019-03-28', String(FOLD_PAGE_DAY), '1549823400']);
  const timed = pageRequest(FEED_A, subject, minute, { fromTime: '09:15' }, null);
  assert.match(timed.notes[0], /time-of-day bound is not applied to 1min: the server folds whole buckets of 1s/);
});

test('a page that cannot be asked is refused before any request, by name', () => {
  const subject = chartSubject(plan([CE]));
  const stored = /** @type {any} */ (planChoice(plan([CE], '1s', '2018-09', '2019-03')));
  const [, minute] = rungChoices(TABLE, FEED_A).choices;
  assert.match(String(pageRequest(FEED_A, chartSubject(null), stored, {}, null).why), /no series is chosen/);
  assert.match(String(pageRequest(FEED_A, subject, stored, { fromDay: '2020-01-01' }, null).why), /outside the held months 2018-09 to 2019-03/);
  assert.match(String(pageRequest(FEED_A, subject, minute, { toDay: '2018-01-01' }, null).why), /outside the held days 2018-09-19 to 2019-03-28/);
  const wide = { ...stored, firstMonth: '1990-01', lastMonth: '2026-09' };
  assert.match(String(pageRequest(FEED_A, subject, wide, {}, null).why), /span 441, past the 240 one window request reads/);
  const spanless = { ...minute, firstDay: '', lastDay: '' };
  assert.match(String(pageRequest(FEED_A, subject, spanless, {}, null).why), /names no first or last day/);
  for (const refused of [pageRequest(FEED_A, subject, wide, {}, null), pageRequest(FEED_A, subject, spanless, {}, null)]) {
    assert.deepEqual([refused.route, refused.query, refused.limit], ['', '', 0]);
  }
});

test('a stored page becomes bars oldest first, and says where the next older page starts', () => {
  const body = { total: 2500, bars: [row(300, { oi: 99 }), row(200), row(100, { phase: 'cas' })], faults: null };
  const page = pageBars('window', body, FEED_A, 1000);
  assert.deepEqual(page.bars.map((b) => b.t), [100, 200, 300]);
  assert.deepEqual([page.more, page.total, page.unread, page.dupes], [1003, 2500, 0, 0]);
  assert.deepEqual([page.bars[2].oi, page.bars[0].phase, page.bars[0].at, page.bars[0].seconds], [99, 'cas', 100, null]);
  assert.equal(rupees(page.bars[0].c), 20.2);
});

test('the last page, an empty page and a single bar end the paging', () => {
  assert.equal(pageBars('window', { total: 3, bars: [row(30), row(20), row(10)] }, FEED_A, 0).more, null);
  const empty = pageBars('window', { total: 0, bars: [] }, FEED_A, 0);
  assert.deepEqual([empty.bars, empty.more, empty.total], [[], null, 0]);
  const short = pageBars('window', { total: 5000, bars: [] }, FEED_A, 1000);
  assert.equal(short.more, null, 'a page that came back empty short of the total does not loop');
  const one = pageBars('window', { total: 1, bars: [row(10)] }, FEED_A, 0);
  assert.deepEqual([one.bars.length, one.more], [1, null]);
});

test('a duplicate stamp, a torn row and a garbage body are counted or refused, never drawn', () => {
  const body = { total: 9, bars: [row(30), row(30, { c: 9999 }), row(20, { c: 12.5 }), { t: 10 }, row(5), null], faults: 'one bar was withheld' };
  const page = pageBars('window', body, FEED_A, 0);
  assert.deepEqual(page.bars.map((b) => [b.t, b.c]), [[5, 2020], [30, 2020]]);
  assert.deepEqual([page.dupes, page.unread, page.faults], [1, 3, 'one bar was withheld']);
  assert.throws(() => pageBars('window', { error: 'unknown feed' }, FEED_A), /unknown feed/);
  assert.throws(() => pageBars('window', { bars: [row(1)] }, FEED_A), /without bars/);
  assert.throws(() => pageBars('fold', { error: 'over_budget' }, FEED_A), /over_budget/);
  assert.throws(() => pageBars('fold', null, FEED_A), /without rows/);
});

test('a folded page takes this feed\'s cell of each bucket, and prices its greeks at the bucket\'s first stored second', () => {
  const t0 = at('2019-03-20', '15:28');
  const body = {
    next_cursor: t0 - 60,
    rows: [
      { t: t0 + 60, cells: { [FEED_A]: { o: 1111, h: 2222, l: 1010, c: 2020, v: 40, oi: 77, seconds: 40, first_at: '15:29:02' } } },
      { t: t0, cells: { [FEED_A]: { o: 1111, h: 2222, l: 1010, c: 2020, v: 3, oi: null, seconds: 12, first_at: '15:28:01' }, [FEED_B]: { o: 1, h: 1, l: 1, c: 1, v: 1 } } },
      { t: t0 - 60, cells: { [FEED_A]: null, [FEED_B]: { o: 1, h: 1, l: 1, c: 1, v: 1 } } }
    ]
  };
  const page = pageBars('fold', body, FEED_A);
  assert.deepEqual(page.bars.map((b) => [b.t, b.at - b.t, b.seconds, b.oi]), [[t0, 1, 12, null], [t0 + 60, 2, 40, 77]]);
  assert.deepEqual([page.more, page.total, page.other], [t0 - 60, null, 1]);
  assert.equal(pageBars('fold', { rows: [], next_cursor: null }, FEED_A).more, null);
  const dayBucket = at('2019-03-20', '00:00');
  const day = pageBars('fold', { rows: [{ t: dayBucket, cells: { [FEED_A]: { o: 1, h: 2, l: 1, c: 2, v: 0, first_at: '09:15:01' } } }] }, FEED_A);
  assert.equal(day.bars[0].at, at('2019-03-20', '09:15:01'));
});

test('pages join oldest first, one bar per stamp: in front, behind, overlapping, and the same page twice', () => {
  const held = [bar(300), bar(400)];
  assert.deepEqual(joinBars([], held), { bars: held, added: 2, dupes: 0 });
  assert.deepEqual(joinBars(held, []), { bars: held, added: 0, dupes: 0 });
  assert.deepEqual(joinBars(held, [bar(100), bar(200)]).bars.map((b) => b.t), [100, 200, 300, 400]);
  assert.deepEqual(joinBars(held, [bar(500)]).bars.map((b) => b.t), [300, 400, 500], 'a late page is laid behind');
  const mixed = joinBars(held, [bar(250), bar(300, { c: 1 }), bar(350), bar(450)]);
  assert.deepEqual(mixed.bars.map((b) => [b.t, b.c]), [[250, 2020], [300, 2020], [350, 2020], [400, 2020], [450, 2020]]);
  assert.deepEqual([mixed.added, mixed.dupes], [3, 1], 'the bar already held is kept');
  const twice = joinBars(held, held);
  assert.deepEqual([twice.bars.length, twice.added, twice.dupes], [2, 0, 2]);
});

test('the chart refuses the page that would pass its ceiling, by name', () => {
  assert.equal(keepRefusal(CHART_KEEP - CHART_PAGE, CHART_PAGE), null);
  assert.match(String(keepRefusal(CHART_KEEP - CHART_PAGE + 1, CHART_PAGE)), /holds 23,?001 bars and keeps at most 24,?000, so the earlier bars are not read/);
  assert.ok(CHART_KEEP >= 23_100, 'one whole one-second session fits');
  assert.equal(keepRefusal(0, 0), null);
});

test('a page of an option asks its greeks in at most four requests of 400 instants, newest first', () => {
  const subject = chartSubject(plan([CE]));
  const start = at('2019-03-20', '09:15');
  const bars = Array.from({ length: 1000 }, (_, i) => bar(start + i));
  const got = greekPlan(bars, subject, '1s');
  assert.deepEqual([got.rows.length, got.requests, got.skipped], [1000, 3, 0]);
  assert.deepEqual([got.rows[0].t, got.rows[0].ts, got.rows[0].tf, got.rows[0].side, got.rows[0].month], [start + 999, (start + 999) * 1000, '1s', 'CE', '2019-03']);
  assert.equal(greekPlan(bars, chartSubject(plan([IDX])), '1s').rows.length, 0, 'an index is not priced');
  assert.equal(greekPlan([], subject, '1s').requests, 0);
});

test('a page that crosses months or outgrows its requests asks the newest instants and counts the rest', () => {
  const subject = chartSubject(plan([CE]));
  // 31 day buckets across three months: three requests, one a month.
  const days = Array.from({ length: 31 }, (_, i) => bar(at('2019-01-25', '09:15:01') + i * 86_400));
  const across = greekPlan(days, subject, '1s');
  assert.deepEqual([across.rows.length, across.requests, across.skipped], [31, 2, 0]);
  assert.deepEqual([...new Set(across.rows.map((r) => r.month))], ['2019-02', '2019-01']);
  // Five months of one instant each against a ceiling of four requests.
  const months = ['2018-11-15', '2018-12-14', '2019-01-15', '2019-02-15', '2019-03-15'].map((d) => bar(at(d, '10:00')));
  const capped = greekPlan(months, subject, '1s');
  assert.deepEqual([capped.requests, capped.rows.length, capped.skipped], [GREEK_REQUESTS, 4, 1]);
  assert.deepEqual(capped.rows.map((r) => r.month), ['2019-03', '2019-02', '2019-01', '2018-12'], 'the oldest month is the one not asked');
  // 1,700 instants of one month need five requests: the oldest 100 are not asked.
  const dense = Array.from({ length: 1700 }, (_, i) => bar(at('2019-03-20', '09:15') + i));
  const most = greekPlan(dense, subject, '1s');
  assert.deepEqual([most.requests, most.rows.length, most.skipped], [4, 1600, 100]);
  assert.ok(most.requests <= GREEK_REQUESTS);
});

test('a folded bucket is priced at its first stored second, filed under the bucket it belongs to', () => {
  const subject = chartSubject(plan([CE]));
  const t = at('2019-03-20', '15:29');
  const bars = [bar(t, { at: t + 2 })];
  const asked = greekPlan(bars, subject, '1s');
  assert.equal(asked.rows[0].ts, (t + 2) * 1000);
  const cells = new Map([[greekKey(asked.rows[0]), { iv: 0.1234, delta: 0.5, why: null }]]);
  const filed = fileGreeks(asked.rows, cells, new Map());
  assert.deepEqual([...filed.keys()], [t]);
  assert.deepEqual(greekLine(bars, filed, 'iv').points, [{ time: t, value: 12.34 }]);
});

test('a line has a point where a bar is priced, and says why the rest have none', () => {
  const bars = [bar(10), bar(20), bar(30), bar(40), bar(50)];
  const cells = new Map([
    [10, { iv: 0.2, delta: 0.25, why: null }],
    [20, { iv: null, delta: null, why: 'rate_unset' }],
    [30, { iv: null, delta: null, why: 'rate_unset' }],
    [40, { iv: null, delta: null, why: 'no_spot_bar' }]
  ]);
  const iv = greekLine(bars, cells, 'iv');
  assert.deepEqual(iv.points, [{ time: 10, value: 20 }]);
  assert.deepEqual(iv.whys, [['rate_unset', 2], ['no_spot_bar', 1]]);
  assert.equal(iv.unasked, 1);
  assert.deepEqual(greekLine(bars, cells, 'delta').points, [{ time: 10, value: 0.25 }]);
  assert.deepEqual(greekLine([], cells, 'iv'), { points: [], whys: [], unasked: 0 });
  assert.deepEqual(GREEK_LINES.map((g) => g.field), ['delta', 'gamma', 'theta', 'vega', 'rho']);
});

test('day flags are asked for the newest 92 days of a page, and a longer page says it was clipped', () => {
  assert.equal(flagSpan([]), null);
  const day = [bar(at('2020-03-13', '09:15')), bar(at('2020-03-13', '15:29'))];
  assert.deepEqual(flagSpan(day), { from: '2020-03-13', to: '2020-03-13', clipped: false });
  const years = [bar(at('2015-01-01', '00:00')), bar(at('2020-03-13', '00:00'))];
  const span = /** @type {any} */ (flagSpan(years));
  assert.deepEqual([span.to, span.clipped], ['2020-03-13', true]);
  assert.equal((Date.parse(span.to) - Date.parse(span.from)) / 86_400_000, DAYFLAGS_PAGE_DAYS - 1);
});

/** A day record as `/dayflags.json` sends one. @param {string} day @param {string} code @param {object} [more] */
const rec = (day, code, more = {}) => ({
  day, instrument: IDX, calendar: { code }, held: { bars: 375, rung: '1min' }, import: { outcome: 'none' },
  events: { halt: null, phases: [{ phase: 'pre_open', from: '09:00', to: '09:14' }, { phase: 'continuous', from: '09:15', to: '15:29' }] },
  says: [], ...more
});
const HALT = {
  standing: 'suspected', halted: { from: '09:22', to: '10:20' }, reopening: { from: '10:06', to: '10:20' },
  seconds: { from: '09:21:10', resume: '10:21:01' }, evidence: 'an invented measurement'
};

test('a halt is a band from its own first second to the second trading resumed', () => {
  const { bands, cells } = dayBands([rec('2020-03-12', 'R'), rec('2020-03-13', 'H', { events: { halt: HALT, phases: [] }, says: ['halt (suspected) 09:22-10:20'] })]);
  assert.equal(bands.length, 1);
  assert.deepEqual([bands[0].kind, bands[0].from, bands[0].to, bands[0].day], ['halt', at('2020-03-13', '09:21:10'), at('2020-03-13', '10:21:01'), '2020-03-13']);
  assert.equal(bands[0].label, 'halt (suspected) 09:21:10–10:21:01 IST');
  assert.equal(bands[0].detail, 'an invented measurement');
  assert.deepEqual(cells.map((c) => [c.day, c.kind]), [['2020-03-12', 'session'], ['2020-03-13', 'halt']]);
});

test('a closing auction is a band only where the day\'s own record gives the series one', () => {
  const phases = [{ phase: 'continuous', from: '09:15', to: '15:14' }, { phase: 'cas', from: '15:15', to: '15:39' }];
  const share = rec('2026-08-03', 'X', { instrument: 'NSE-CASH-RELIANCE', events: { halt: null, phases } });
  const index = rec('2026-08-03', 'X', { events: { halt: null, phases } });
  const got = dayBands([share]).bands;
  assert.deepEqual(got.map((b) => [b.kind, b.from, b.to]), [['cas', at('2026-08-03', '15:15'), at('2026-08-03', '15:40')]]);
  assert.deepEqual(dayBands([index]).bands, [], 'an index has no closing auction of its own');
  // WHERE THE RECORD DOES NOT KNOW THE SHARE'S ELIGIBILITY it calls the minutes unverified, and so does the band.
  const unknown = rec('2026-08-03', 'X', {
    instrument: 'NSE-CASH-RELIANCE',
    events: { halt: null, phases: [{ phase: 'continuous', from: '09:15', to: '15:14' }, { phase: 'unverified', from: '15:15', to: '15:29' }] }
  });
  const window = dayBands([unknown]).bands;
  assert.deepEqual(window.map((b) => [b.kind, b.from, b.to]), [['cas', at('2026-08-03', '15:15'), at('2026-08-03', '15:30')]]);
  assert.match(window[0].label, /closing-auction window 15:15–15:29 IST, unverified for this series/);
  const ordinary = rec('2019-03-20', 'R', { instrument: 'NSE-CASH-RELIANCE', events: { halt: null, phases: [{ phase: 'unverified', from: '15:15', to: '15:29' }] } });
  assert.deepEqual(dayBands([ordinary]).bands, [], 'unverified minutes of a day with no auction are not an auction');
});

test('a session the store holds no bar of is a gap band over its session, and garbage records make no band', () => {
  const missing = rec('2019-03-19', 'R', { held: { bars: 0, rung: '1min' }, says: ['no bar is held'] });
  const got = dayBands([missing]).bands;
  assert.deepEqual(got.map((b) => [b.kind, b.from, b.to, b.detail]), [['gap', at('2019-03-19', '09:15'), at('2019-03-19', '15:30'), 'no bar is held']]);
  const option = rec('2019-03-19', 'R', { instrument: CE, held: { bars: 0, rung: '1s' } });
  assert.deepEqual(dayBands([option]).bands, [], 'a contract that did not trade is not a hole');
  assert.deepEqual(dayBands(/** @type {any} */ (null)), { bands: [], cells: [] });
  assert.deepEqual(dayBands([{}, null, { day: '2019-03-19', events: { halt: { seconds: { from: 'soon', resume: 'later' } } } }]).bands, []);
});

test('a hole in a minute series is a gap band on the days the server flagged, and never for a trade-driven series', () => {
  const d = '2019-03-20';
  const bars = [bar(at(d, '09:15')), bar(at(d, '09:16')), bar(at(d, '09:20')), bar(at(d, '09:21'))];
  const flagged = dayBands([rec(d, 'R', { gaps: { missing: 3, first_missing: '09:17', runs: 1 } })]).cells;
  const got = holeBands(bars, 60, flagged, [], false);
  assert.deepEqual(got.bands.map((b) => [b.kind, b.from, b.to, b.label]), [['gap', at(d, '09:17'), at(d, '09:20'), '3 bar(s) missing on 2019-03-20']]);
  assert.deepEqual(holeBands(bars, 60, flagged, [], true).bands, [], 'bars of a trade-driven series are its trades');
  assert.deepEqual(holeBands(bars, 60, dayBands([rec(d, 'R')]).cells, [], false).bands, [], 'a day the server did not flag is not accused');
  const halt = [{ kind: /** @type {const} */ ('halt'), from: at(d, '09:16:30'), to: at(d, '09:20'), day: d, label: '', detail: '' }];
  assert.deepEqual(holeBands(bars, 60, flagged, halt, false).bands, [], 'a hole inside a halt is the halt');
  assert.deepEqual(holeBands(bars, 86_400, flagged, [], false).bands, [], 'a day rung has no hole inside a day');
  assert.equal(tradeDriven(chartSubject(plan([CE])), 60), true);
  assert.equal(tradeDriven(chartSubject(plan(['NSE-CASH-RELIANCE'])), 1), true);
  assert.equal(tradeDriven(chartSubject(plan(['NSE-CASH-RELIANCE'])), 60), false);
  assert.equal(tradeDriven(chartSubject(plan([IDX])), 1), false);
});

test('a seconds series calls five missing seconds a hole, never one, and stops at its band ceiling', () => {
  const d = '2019-03-20';
  const t = at(d, '10:00');
  const flagged = dayBands([rec(d, 'R', { quality: { indices: { holes_5s: 2 } } })]).cells;
  const bars = [bar(t), bar(t + 2), bar(t + 7), bar(t + 13), bar(t + 14)];
  assert.deepEqual(holeBands(bars, 1, flagged, [], false).bands.map((b) => [b.from - t, b.to - t]), [[8, 13]]);
  const ragged = Array.from({ length: GAP_BANDS + 51 }, (_, i) => bar(t + i * 10));
  const many = holeBands(ragged, 1, flagged, [], false);
  assert.deepEqual([many.bands.length, many.more], [GAP_BANDS, 50]);
});

test('a halt with no bar in it is a shaded block of empty slots, at most 120 of them, on the rung\'s grid', () => {
  const d = '2020-03-13';
  const band = { kind: /** @type {const} */ ('halt'), from: at(d, '09:21:10'), to: at(d, '10:21:01'), day: d, label: '', detail: '' };
  const bars = [bar(at(d, '09:21:09')), bar(at(d, '10:21:01'))];
  const slots = bandSlots([band], bars, 1);
  assert.ok(slots.length > 60 && slots.length <= BAND_SLOTS, `${slots.length} slots`);
  assert.ok(slots.every((s) => s.kind === 'halt' && s.time >= band.from && s.time < band.to));
  assert.deepEqual(slots.map((s) => s.time), [...slots.map((s) => s.time)].sort((a, b) => a - b));
  assert.equal(new Set(slots.map((s) => s.time)).size, slots.length);
  // At the minute rung the halt is its 59 missing minutes, and the minute it began in.
  const minutes = [bar(at(d, '09:21')), bar(at(d, '10:21'))];
  const m = bandSlots([band], minutes, 60);
  assert.equal(m.length, 60);
  assert.equal(m[0].time, at(d, '09:21'), 'the bar the halt begins in is shaded');
  assert.equal(m[59].time, at(d, '10:20'));
});

test('a band shades the bars it holds and adds no slot where a bar already is', () => {
  const d = '2020-03-13';
  const from = at(d, '09:21:10');
  const band = { kind: /** @type {const} */ ('halt'), from, to: from + 100, day: d, label: '', detail: '' };
  const bars = [bar(from - 5), ...Array.from({ length: 100 }, (_, i) => bar(from + i)), bar(from + 100)];
  const slots = bandSlots([band], bars, 1);
  assert.equal(slots.length, 100);
  assert.ok(slots.every((s) => bars.some((b) => b.t === s.time)), 'every slot is a held bar');
});

test('bands are clipped to the bars held, a bar\'s own phase shades it, and a halt outranks an auction', () => {
  const d = '2026-08-03';
  const bars = [bar(at(d, '15:14')), bar(at(d, '15:15'), { phase: 'cas' }), bar(at(d, '15:16'), { phase: 'halted' })];
  assert.deepEqual(bandSlots([], bars, 60), [{ time: at(d, '15:15'), kind: 'cas' }, { time: at(d, '15:16'), kind: 'halt' }]);
  const cas = { kind: /** @type {const} */ ('cas'), from: at(d, '15:15'), to: at(d, '15:40'), day: d, label: '', detail: '' };
  const halt = { kind: /** @type {const} */ ('halt'), from: at(d, '15:15'), to: at(d, '15:16'), day: d, label: '', detail: '' };
  const both = bandSlots([cas, halt], bars, 60);
  assert.deepEqual(both, [{ time: at(d, '15:15'), kind: 'halt' }, { time: at(d, '15:16'), kind: 'halt' }], 'nothing is drawn after the last held bar');
  const before = { kind: /** @type {const} */ ('gap'), from: at(d, '09:15'), to: at(d, '10:00'), day: d, label: '', detail: '' };
  assert.deepEqual(bandSlots([before], bars, 60), bandSlots([], bars, 60), 'nothing is drawn before the first held bar');
  assert.deepEqual(bandSlots([cas], [], 60), []);
});

test('at the day rung a missing session is one empty slot, and a halt shades its own day candle', () => {
  const days = ['2020-03-12', '2020-03-13', '2020-03-17'].map((d) => bar(at(d, '00:00')));
  const gap = { kind: /** @type {const} */ ('gap'), from: at('2020-03-16', '09:15'), to: at('2020-03-16', '15:30'), day: '2020-03-16', label: '', detail: '' };
  const halt = { kind: /** @type {const} */ ('halt'), from: at('2020-03-13', '09:21:10'), to: at('2020-03-13', '10:21:01'), day: '2020-03-13', label: '', detail: '' };
  assert.deepEqual(bandSlots([gap, halt], days, 86_400), [
    { time: at('2020-03-13', '00:00'), kind: 'halt' },
    { time: at('2020-03-16', '09:15'), kind: 'gap' }
  ]);
});

test('a flagged day is marked on its first held bar, or on its empty slot; a lone auction and an unseen day are not', () => {
  const cells = dayBands([
    rec('2020-03-13', 'H', { events: { halt: HALT, phases: [] }, says: ['halt (suspected) 09:22-10:20'] }),
    rec('2020-03-16', 'R', { held: { bars: 0, rung: '1min' } }),
    rec('2020-03-17', 'R'),
    rec('2026-08-03', 'X', { instrument: 'NSE-CASH-RELIANCE' }),
    rec('2020-03-18', 'R', { held: { bars: 0, rung: '1min' } })
  ]).cells;
  const bars = [bar(at('2020-03-13', '09:15')), bar(at('2020-03-13', '09:16')), bar(at('2020-03-17', '09:15')), bar(at('2026-08-03', '09:15'))];
  const slots = [{ time: at('2020-03-16', '09:15'), kind: 'gap' }];
  const marks = flagMarkers(cells, bars, slots);
  assert.deepEqual(marks.map((m) => [m.day, m.time, m.kind, m.tone]), [
    ['2020-03-13', at('2020-03-13', '09:15'), 'halt', 'info'],
    ['2020-03-16', at('2020-03-16', '09:15'), 'store_gap', 'bad']
  ]);
  assert.deepEqual(marks[0].says, ['halt (suspected) 09:22-10:20']);
  assert.deepEqual(flagMarkers([], bars, slots), []);
});

test('a theme colour takes an alpha as #rrggbbaa, and one it cannot read is drawn as it came', () => {
  assert.equal(alphaHex('#945d05', 0.5), '#945d0580');
  assert.equal(alphaHex('#abc', 1), '#aabbccff');
  assert.equal(alphaHex(' #00E19B ', 0), '#00E19B00');
  assert.equal(alphaHex('rgb(1, 2, 3)', 0.5), 'rgb(1, 2, 3)');
  assert.equal(alphaHex('', 0.5), '');
  assert.equal(alphaHex('#945d05', 7), '#945d05ff');
});

test('the plan names no feed and no instrument: every rung and source comes from the server\'s table', () => {
  const source = dbSource('../src/lib/db/chart.js');
  assert.doesNotMatch(source, /['"`](gdfl|zerodha|dhan|groww|truedata)['"`]/i);
  assert.doesNotMatch(source, /['"`](NIFTY|BANKNIFTY)['"`]/);
  assert.doesNotMatch(source, /\bfetch\(/, 'the plan sends nothing itself');
});

test('the panel is closed until asked, reads through the counted request, and loads one page a gesture', () => {
  const panel = dbSource('../src/routes/db/DbChart.svelte');
  assert.match(panel, /getItem\(OPEN_KEY\) === '1'/, 'closed unless the reader opened it before');
  assert.match(panel, /open && showing && subject\.key && choice && feed/, 'nothing is read while the panel is closed');
  assert.doesNotMatch(panel, /\bfetch\(/, 'every request goes through ask');
  assert.equal(panel.match(/\bask\(/g)?.length, 2, 'ask is called in two places: the rung table and the counted page request');
  assert.match(panel, /requests \+= 1;\s+return ask\(url, options\);/, 'every request of a page is counted where it is sent');
  assert.match(panel, /if \(response\.status === 429\) throw/, 'a busy server is said, not retried past the bound');
  assert.match(panel, /if \(!armed \|\| !range \|\| range\.from > EDGE_BARS\) return;\s+armed = false;\s+earlier\('pan'\);/);
  assert.match(panel, /if \(s\.pages === before\) break;/, 'a page that did not arrive is never asked again in a loop');
  assert.match(panel, /import\('lightweight-charts'\)/);
  assert.match(panel, /role="img" aria-label=\{summary\}/);
  assert.match(panel, />Load earlier<\/button>/, 'a pan has a button that does the same');
  assert.doesNotMatch(panel, /['"`](gdfl|zerodha)['"`]/i);
});
