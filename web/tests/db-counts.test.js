// /db COUNTS THAT AGREE (R3-04; DB-048, DB-014, DB-046).
//
// `node --test web/tests/` — node's own runner, no dependency. The Bars
// badge, the Coverage badge, the pager, the Coverage grid and the header
// line beside the tabs each counted their own thing: 434 against 443 for
// one selection and one day window, 4 against 7 for one tab read on Bars and
// then on Coverage, a window that excluded a contract left Coverage full,
// and the header printed a strike in paisa. Every rule is a pure function
// in `$lib/db/notes-coverage.js`; the components are read as text only to
// prove they call those functions and send no request of their own.
// Every key, day and count below is invented in shape only.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  barsBadge,
  barsQueryKey,
  coverWindow,
  monthInWindow,
  windowText,
  windowEmptyWhy,
  coverTotals,
  missingText,
  headerRange,
  closeInWindow,
  contractTail,
  refusedFrontOf,
  whyText,
  noteText,
  bytesText,
  WHY,
  NOTE_TEXT,
  SOLE_WHY,
  STATE_WORD,
  COLS
} from '../src/lib/db/notes-coverage.js';
import { judgeRow } from '../src/lib/completeness.js';
import { owedLedger, epochDayOfIso } from '../src/lib/calendar-owed.js';
import { istDay } from '../src/lib/dates.js';

/** @param {string} path relative to this file */
const source = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const SUMMARY = source('../src/routes/db/DbSummary.svelte');
const COVERAGE = source('../src/routes/db/DbCoverageGrid.svelte');

/**
 * The text of one `$derived` (or `$derived.by`) declaration in a component,
 * up to the next `const`/`let`/`$effect`/`function` at the same indentation.
 *
 * @param {string} text
 * @param {string} name
 */
function declaration(text, name) {
  const at = text.indexOf(`const ${name} = $derived`);
  assert.ok(at >= 0, `${name} is declared`);
  const rest = text.slice(at + 1);
  const next = rest.search(/\n {4}(?:const |let |\$effect|function |\/\*\*)/);
  return text.slice(at, next < 0 ? undefined : at + 1 + next);
}

/** What a request in a component looks like, any of the page's readers. */
const REQUEST = /\bask\(|\bfetch\(|fetchAdmitted\(|readWindowPage\(|readAllRows\(|readLedger\(/;

test('the_bars_badge_is_the_servers_count_on_both_tabs (DB-048)', () => {
  const plan = { timeframe: '1s', from: '2019-03', to: '2019-03', keys: ['NSE-FNO-NIFTY-2019-03-28-1160000-CE'] };
  const bounds = { fromDay: '2019-03-15', toDay: '2019-03-15', fromTime: '', toTime: '' };
  const key = barsQueryKey('gdfl', plan, bounds, 0);
  // The server counted 434 bars that day; the census holds 443 in the month.
  const server = 434;
  const census = 443;
  // BEFORE: on the Coverage tab the grid has dropped its answer, so the badge
  // fell back to the census — 443 beside the pager's 434.
  const before = (/** @type {number | null} */ said) => (said !== null ? said : census);
  assert.equal(before(null), 443);
  // AFTER, on Bars: the live answer.
  const live = barsBadge({ said: server, remembered: null, key, census, narrows: true });
  assert.deepEqual([live.n, live.from], [434, 'server']);
  // AFTER, on Coverage: the answer kept from Bars for this same query.
  const kept = barsBadge({ said: null, remembered: { key, total: server }, key, census, narrows: true });
  assert.deepEqual([kept.n, kept.from], [434, 'server']);
  assert.match(kept.title, /on either tab/);
  // A kept count stands only for its own query: another window, another
  // series or a re-read is not counted by it.
  for (const other of [
    barsQueryKey('gdfl', plan, { ...bounds, toDay: '2019-03-20' }, 0),
    barsQueryKey('gdfl', { ...plan, keys: ['NSE-FNO-NIFTY-2019-03-28-1160000-PE'] }, bounds, 0),
    barsQueryKey('gdfl', plan, bounds, 1),
    barsQueryKey('zerodha', plan, bounds, 0)
  ]) {
    assert.notEqual(other, key);
    const stale = barsBadge({ said: null, remembered: { key, total: server }, key: other, census, narrows: true });
    assert.equal(stale.n, null, 'never another query\'s count');
    assert.equal(stale.from, 'none');
    assert.match(stale.title, /Not counted yet/);
  }
  // The page and the order are not in the key: one count for every page.
  assert.equal(barsQueryKey('gdfl', plan, bounds, 0), key);
  // With no day or time bound the census counts the same set, and says so.
  const unbounded = barsBadge({ said: null, remembered: null, key, census, narrows: false });
  assert.deepEqual([unbounded.n, unbounded.from], [443, 'census']);
  assert.match(unbounded.title, /census/);
  // A plan the page has not built yet still keys.
  assert.equal(barsQueryKey('', null, null, undefined), '|||||||||');

  // THE COMPONENT READS THESE AND SENDS NOTHING FOR THEM. The badge and the
  // kept count are derived from the grid's own answer; no request is made.
  assert.match(SUMMARY, /barsBadge\(\{/);
  assert.match(SUMMARY, /barsQueryKey\(feeds\.active \?\? '', db\.barQueryPlan, db\.barBounds, db\.nonce\)/);
  assert.doesNotMatch(declaration(SUMMARY, 'barsCount'), REQUEST);
  assert.doesNotMatch(declaration(SUMMARY, 'barsKey'), REQUEST);
  const keep = SUMMARY.slice(SUMMARY.indexOf('let barsCounted'), SUMMARY.indexOf('const barsCount ='));
  assert.doesNotMatch(keep, REQUEST);
  // Only a settled answer is kept, never the last query's while a new one loads.
  assert.match(keep, /!db\.barState\?\.loading/);
  // Both tabs print the one count each.
  assert.match(SUMMARY, /v\.key === 'bars' \? \(db\.barsCount\.n === null \? '—' : fmt\(db\.barsCount\.n\)\) : fmt\(db\.covCount\)/);
  assert.doesNotMatch(SUMMARY, /db\.windowSaid \? db\.pageTotal : db\.total/, 'the census fallback on Coverage is gone');
  assert.doesNotMatch(SUMMARY, /: db\.sorted\.length\)/, 'the Coverage badge no longer reads the view-dependent rows');
});

test('coverage_follows_the_day_window_or_says_it_does_not (DB-046, R3-04)', () => {
  // The window narrows only when the fields differ from their seeded span.
  assert.equal(coverWindow('2024-10-01', '2024-10-31', false), null);
  assert.equal(coverWindow('', '', true), null);
  assert.deepEqual(coverWindow('2024-10-01', '2024-10-31', true), { from: '2024-10-01', to: '2024-10-31' });
  // A window typed backwards is put in order, so its range never reads To before From.
  assert.deepEqual(coverWindow('2024-10-31', '2024-10-01', true), { from: '2024-10-01', to: '2024-10-31' });
  assert.deepEqual(coverWindow('2024-10-01', '', true), { from: '2024-10-01', to: '' });
  // A month is listed when the window touches one of its days.
  const oct = coverWindow('2024-10-15', '2024-11-02', true);
  assert.deepEqual(['2024-09', '2024-10', '2024-11', '2024-12'].filter((m) => monthInWindow(m, oct)), ['2024-10', '2024-11']);
  assert.equal(monthInWindow('2019-03', null), true, 'no window narrows nothing');
  assert.equal(monthInWindow('2019-03', { from: '', to: '2019-02-28' }), false);
  assert.equal(monthInWindow('2019-03', { from: '2019-03-31', to: '' }), true);
  assert.equal(windowText({ from: '2024-10-01', to: '' }), '01 Oct 2024 → the last day held');
  assert.equal(windowText({ from: '', to: '2024-10-31' }), 'the first day held → 31 Oct 2024');
  // A contract that expired in March 2019 under an October 2024 window: the
  // window empties Coverage, and the grid says so once, both ranges in order.
  const matched = ['2019-01', '2019-02', '2019-03'];
  const window = coverWindow('2024-10-31', '2024-10-01', true);
  const listed = matched.filter((m) => monthInWindow(m, window));
  assert.equal(listed.length, 0, 'BEFORE, Coverage listed all three months under this window');
  const why = windowEmptyWhy({ matched: matched.length, listed: listed.length, window, held: { lo: '2019-03-28', hi: '2019-01-01' } });
  assert.equal(
    why,
    'The From and To window, 01 Oct 2024 → 31 Oct 2024, holds no month of the 3 instrument-month(s) this selection matches; this selection holds 01 Jan 2019 → 28 Mar 2019. Coverage follows the window, as the Bars grid does: widen the dates to see them.'
  );
  // Said only when the window is what emptied it.
  assert.equal(windowEmptyWhy({ matched: 3, listed: 1, window, held: null }), '');
  assert.equal(windowEmptyWhy({ matched: 0, listed: 0, window, held: null }), '');
  assert.equal(windowEmptyWhy({ matched: 3, listed: 0, window: null, held: null }), '');
  assert.doesNotMatch(windowEmptyWhy({ matched: 3, listed: 0, window, held: null }), /this selection holds/);

  // THE GRID LISTS, COUNTS AND PAGES ONE SET. The listed rows are the
  // matched rows and the absent months, each in a month the window touches,
  // and the count is read on either tab.
  assert.match(COVERAGE, /const covWindow = \$derived\(coverWindow\(db\.fromDay \?\? '', db\.toDay \?\? '', Boolean\(db\.dayNarrowing\)\)\)/);
  assert.match(COVERAGE, /const listedStored = \$derived\(db\.matched\.filter\(\(\/\*\* @type \{any\} \*\/ r\) => monthInWindow\(r\.month, covWindow\)\)\)/);
  assert.match(declaration(COVERAGE, 'listedAbsent'), /if \(monthInWindow\(m, covWindow\)\) out\.push/);
  assert.match(COVERAGE, /const covCount = \$derived\(listedStored\.length \+ listedAbsent\.length\)/);
  assert.match(declaration(COVERAGE, 'judged'), /for \(const r of listedStored\)[\s\S]*for \(const a of listedAbsent\)/);
  // The count reads an index of the matched instruments only, on either tab;
  // the whole-census index stays the Coverage view's, so Bars pays a Set probe a row.
  assert.match(declaration(COVERAGE, 'matchedSeries'), /names\.add\(r\.instrument\);\s*rungs\.add\(r\.timeframe\);\s*if \(derivedRungs\.has\(r\.timeframe\)\) rungs\.add\('1min'\);[\s\S]*return indexSeries\(db\.deco, names, rungs\);/);
  assert.match(COVERAGE, /const seriesIndex = \$derived\(db\.view === 'census' \? indexSeries\(db\.deco, null, null\) : new Map\(\)\);/);
  assert.match(declaration(COVERAGE, 'listedAbsent'), /absentMonthsOf\(matchedSeries, r\.instrument, r\.timeframe\)/);
  assert.doesNotMatch(declaration(COVERAGE, 'listedAbsent'), REQUEST);
  // It says plainly that dates narrow Coverage, and why the grid is empty.
  assert.match(COVERAGE, /\{#if db\.covWindow\}[\s\S]*months touching <b>\{windowText\(db\.covWindow\)\}<\/b>/);
  assert.match(COVERAGE, /\{#if db\.covWindowEmpty\}\s*<!--[\s\S]*?-->\s*<p>\{db\.covWindowEmpty\}<\/p>/);
});

test('the_header_prints_a_strike_as_a_price, and describes the window in view (R3-04)', () => {
  const key = 'NSE-FNO-NIFTY-2019-03-28-1165000-CE';
  // BEFORE: the last two dash-separated fields of the key, the strike in paisa.
  assert.equal(key.split('-').slice(-2).join(' '), '1165000 CE');
  // AFTER: the strike in rupees.
  assert.equal(contractTail(key), '11,650 CE');
  assert.equal(contractTail('NSE-FNO-BANKNIFTY-2020-03-26-2795050-PE'), '27,950.50 PE', 'paisa kept when a strike has them');
  assert.equal(contractTail('NSE-FNO-NIFTY-2019-03-28-FUT'), 'FUT');
  assert.equal(contractTail('NSE-INDEX-NIFTY'), '');
  // The range: the months on disk, in order, with no window.
  assert.deepEqual(headerRange(['2019-03', '2019-01', '2019-02'], null), { from: 'Jan 2019', to: 'Mar 2019', word: 'on disk' });
  assert.equal(headerRange([], null), null);
  // The window in view when it narrows, in order, an open end from the months.
  assert.deepEqual(headerRange(['2019-01', '2019-03'], { from: '2019-03-15', to: '2019-03-15' }), { from: '15 Mar 2019', to: '15 Mar 2019', word: 'in view' });
  assert.deepEqual(headerRange(['2019-01', '2019-03'], { from: '2019-02-01', to: '' }), { from: '01 Feb 2019', to: 'Mar 2019', word: 'in view' });
  assert.deepEqual(headerRange(['2019-03', '2019-01'], { from: '', to: '2019-02-28' }), { from: 'Jan 2019', to: '28 Feb 2019', word: 'in view' });
  assert.deepEqual(headerRange([], { from: '', to: '2019-02-28' }), { from: '—', to: '28 Feb 2019', word: 'in view' });
  assert.deepEqual(headerRange([], { from: '2019-02-01', to: '' }), { from: '01 Feb 2019', to: '—', word: 'in view' });
  // The newest close is this window's only if it falls in it.
  const at = Date.parse('2019-03-28T09:59:59Z');
  assert.equal(closeInWindow(at, null, istDay), 'in');
  assert.equal(closeInWindow(at, { from: '2019-03-01', to: '2019-03-28' }, istDay), 'in');
  assert.equal(closeInWindow(at, { from: '2019-03-01', to: '2019-03-15' }, istDay), 'after');
  assert.equal(closeInWindow(at, { from: '2024-10-01', to: '2024-10-31' }, istDay), 'before');
  // The component prints the tail and the range from these, and names a close outside the window.
  assert.match(SUMMARY, /tail: contractTail\(nc\.key\)/);
  assert.doesNotMatch(SUMMARY, /split\('-'\)\.slice\(-2\)/, 'the paisa tail is gone');
  assert.match(SUMMARY, /headerRange\(db\.monthsIn \?\? \[\], db\.covWindow \?\? null\)/);
  assert.match(SUMMARY, /closeInWindow\(ms, db\.covWindow \?\? null, istDay\)/);
  assert.match(SUMMARY, /\{:else if db\.header\.outside\}/);
});

/** A 2026-09 ledger of 21 regular sessions at 1min, for the drawer rows. */
function septemberRows() {
  const days = 'RR..RRRRR..RRRRR..RRRRR..RRRR.';
  const L = owedLedger({
    status: 'ok', feed: 'zerodha', rung: '1min', from: epochDayOfIso('2026-09-01'), days, held: null,
    heldBy: ['NIFTY'], target: { kind: 'exchange' }, owed: { rung: '1min', regular: 375, casEligible: 360 }, special: [], halts: []
  });
  const base = { instrument: 'NSE-INDEX-NIFTY', timeframe: '1min', firstAt: null, lastAt: null, expiry: null, strike: null, underlying: 'NIFTY' };
  const rows = [
    { ...base, month: '2026-09', rows: 21 * 375 },
    { ...base, month: '2026-09', rows: 21 * 375 - 40 },
    { ...base, month: '2026-09', rows: 0, absent: true },
    { ...base, month: '2026-09', rows: 21 * 375 + 9 },
    { ...base, instrument: 'NSE-FNO-NIFTY-2026-09-29-2500000-CE', month: '2026-09', timeframe: '1min', rows: 0, strike: 2500000, absent: true }
  ];
  return rows.map((r) => ({ ...r, cover: judgeRow(r, { ledger: L }) }));
}

test("the drawer's totals agree with the rows it prints (R3-04, DB-014)", () => {
  const rows = septemberRows();
  const printed = rows.map(missingText);
  assert.deepEqual(printed, ['full', '−40', 'absent −7,875', '+9', 'no trade']);
  const t = coverTotals(rows);
  // Missing is the sum of the numbers printed under it: 40 + 7,875.
  assert.equal(t.missing, 40 + 7875);
  // BEFORE the absent month printed "absent" alone while the total counted its 7,875.
  assert.equal(t.full, 1);
  assert.equal(t.over, 1, 'the unplaced surplus is counted on its own');
  assert.equal(t.absent, 1);
  assert.equal(t.noTrade, 1);
  assert.equal(t.stored, 3);
  assert.equal(t.trade, 1);
  assert.equal(t.owed, 3 * 7875, 'the over row joins neither sum');
  assert.equal(t.held, 7875 + 7835);
  assert.equal(t.coverage, (7875 + 7835) / (3 * 7875));
  assert.equal(t.bars, 7875 + 7835 + 7884);
  assert.equal(t.heldExact, false);
  assert.equal(coverTotals([]).coverage, null);
  assert.equal(coverTotals(/** @type {any} */ (null)).n, 0);
  // A row with no calendar prints a dash and joins no sum.
  const none = { rows: 5, cover: judgeRow(rows[0], { ledger: null }) };
  assert.equal(missingText(none), '—');
  assert.equal(coverTotals([none]).unverified, 1);
  // The grid and the drawer both count with this one function.
  assert.match(COVERAGE, /const totals = \$derived\(coverTotals\(judged\)\)/);
  assert.match(declaration(COVERAGE, 'openTotals'), /coverTotals\(openMonths\)/);
  assert.match(COVERAGE, /<span class="dshort">\{missingText\(m\)\}<\/span>/);
  assert.match(COVERAGE, /coverage <b>\{coverPct\(db\.coverage\)\}<\/b>/);
  assert.doesNotMatch(COVERAGE, /pctText\(/, 'no percentage on the grid rounds a short month up to 100.00%');
});

test('a refused front is named beside the coverage figure, and the empty grid says the same words (D-3315, R3-04)', () => {
  const cells = [
    { day: '2020-01-10', kinds: ['session'] },
    { day: '2020-01-13', kinds: ['front', 'gap'] },
    { day: '2020-01-14', kinds: ['front'] },
    { day: '2020-01-15' }
  ];
  const front = refusedFrontOf(cells, 'NSE-INDEX-ZZOIL');
  assert.ok(front);
  assert.equal(front.n, 2);
  assert.equal(front.from, '2020-01-13');
  assert.equal(front.to, '2020-01-14');
  assert.match(front.text, /^2 day\(s\) of a refused front, 13 Jan 2020 → 14 Jan 2020 \(NSE-INDEX-ZZOIL\)/);
  assert.match(front.text, /never counted as held/);
  assert.doesNotMatch(refusedFrontOf(cells, '')?.text ?? '', /\(\)/);
  assert.equal(refusedFrontOf([{ day: '2020-01-10', kinds: ['session'] }], 'X'), null);
  assert.equal(refusedFrontOf(undefined, 'X'), null);
  // From the band's own answer — no request of its own — in the box and in the empty grid.
  assert.match(COVERAGE, /const refusedFront = \$derived\(refusedFrontOf\(db\.dayBandCells, db\.dayBandInstrument \?\? ''\)\)/);
  assert.match(COVERAGE, /<span class="cs risk" title=\{db\.covRefusedFront\.text\}>/);
  assert.match(COVERAGE, /<p class="tgate">\{db\.covRefusedFront\.text\}<\/p>/);
});

test('the coverage words at their edges: an unknown code is named as one, bytes read as a quantity', () => {
  assert.match(whyText('absent'), /^No file/);
  assert.match(whyText(null), /none given/);
  assert.match(whyText('zz_new'), /does not have a sentence for: zz_new/);
  assert.match(noteText('surplus_unplaced'), /never called full or 100\.00%/);
  assert.match(noteText('unheld_sessions'), /never pays for them/);
  assert.match(noteText('zz_new'), /no sentence for: zz_new/);
  assert.deepEqual([5e9, 5e6, 5e3, 12.4].map(bytesText), ['5.0 GB', '5.0 MB', '5.0 kB', '12 B']);
  // An ordered held span is printed as it is.
  assert.match(windowEmptyWhy({ matched: 1, listed: 0, window: { from: '2024-10-01', to: '2024-10-31' }, held: { lo: '2019-01-01', hi: '2019-03-28' } }), /holds 01 Jan 2019 → 28 Mar 2019/);
  // The counted line's own notes: an unread auction flag, a store-wide hole, a derived month behind or partial.
  const cover = (/** @type {Partial<import('../src/lib/completeness.js').Cover>} */ c) =>
    /** @type {import('../src/lib/completeness.js').Cover} */ ({ basis: 'calendar', state: 'near', owed: 10, missing: 1, sessions: 1, heldSessions: 1, heldExact: true, notes: [], extra: 0, ...c });
  const t = coverTotals([
    { rows: 9, cover: cover({ state: 'unmeasured', missing: null, notes: ['cas_unresolved'] }) },
    { rows: 9, cover: cover({ notes: ['cas_unresolved'] }) },
    { rows: 9, cover: cover({ notes: ['store_wide'] }) },
    { rows: 9, cover: cover({ notes: ['derived_behind'] }) },
    { rows: 9, cover: cover({ notes: ['derived_partial'] }) }
  ]);
  assert.equal(t.unresolved, 1, 'only an unresolved row with no missing count is the flag\'s to decide');
  assert.equal(t.storeWide, 1);
  assert.equal(t.derivedLagging, 2);
  assert.equal(t.missing, 4);
  assert.equal(t.heldExact, true);
});

test('the coverage words are pinned: every sentence present, every column and state word as the grid prints it', () => {
  for (const table of [WHY, NOTE_TEXT]) {
    for (const [code, sentence] of Object.entries(table)) {
      assert.ok(sentence.length > 40 && /[.)]$/.test(sentence), `${code} carries a whole sentence`);
    }
  }
  assert.match(SOLE_WHY, /^This is the only instrument-month stored at this rung[\s\S]*a tautology, not a measurement[\s\S]*Pull a second instrument/);
  assert.deepEqual(STATE_WORD, { full: 'full', near: 'near', gap: 'short', absent: 'absent', trade: 'trade-driven', unmeasured: 'unmeasured', over: 'over' });
  assert.deepEqual(COLS.map((c) => [c.key, c.label, c.num]), [
    ['instrument', 'Instrument', false],
    ['month', 'Month', false],
    ['timeframe', 'TF', false],
    ['rows', 'Bars', true],
    ['days', 'Sessions', true],
    ['short', 'Missing', true],
    ['pct', 'Coverage', true],
    ['chg', '% in month', true],
    ['prevChg', 'Prev month %', true]
  ]);
  for (const c of COLS.slice(4)) assert.ok(typeof c.why === 'string' && c.why.length > 40, `${c.key} says what it is`);
  // An inherited name is no reason and no note.
  assert.match(whyText('constructor'), /does not have a sentence for: constructor/);
  assert.match(noteText('toString'), /no sentence for: toString/);
  // Bytes at each boundary.
  assert.deepEqual([1e9, 1e6, 1e3, 999].map(bytesText), ['1.0 GB', '1.0 MB', '1.0 kB', '999 B']);
});

test('the window and the header at their boundaries', () => {
  assert.deepEqual(coverWindow('', '2024-10-31', true), { from: '', to: '2024-10-31' });
  // A To on the 1st holds its month; a From on the 31st holds its month.
  assert.equal(monthInWindow('2024-10', { from: '', to: '2024-10-01' }), true);
  assert.equal(monthInWindow('2024-11', { from: '', to: '2024-10-01' }), false);
  assert.equal(monthInWindow('2024-10', { from: '2024-10-31', to: '' }), true);
  assert.equal(monthInWindow('2024-09', { from: '2024-10-31', to: '' }), false);
  assert.equal(monthInWindow('2030-01', { from: '2024-10-31', to: '' }), true, 'an open To holds every later month');
  assert.equal(monthInWindow('2019-03', { from: '', to: '' }), true);
  // A held span with an end missing names no span.
  assert.doesNotMatch(windowEmptyWhy({ matched: 1, listed: 0, window: { from: '2024-10-01', to: '' }, held: { lo: '', hi: '2019-01-01' } }), /this selection holds/);
  // The close on the window's first day is in it; an open To has nothing after it.
  const at = Date.parse('2019-03-28T09:59:59Z');
  assert.equal(closeInWindow(at, { from: '2019-03-28', to: '' }, istDay), 'in');
  assert.equal(closeInWindow(at, { from: '2019-03-29', to: '' }, istDay), 'before');
  assert.equal(closeInWindow(at, { from: '', to: '' }, istDay), 'in');
  assert.deepEqual(headerRange(['2019-02', '2019-03', '2019-01'], null), { from: 'Jan 2019', to: 'Mar 2019', word: 'on disk' });
  assert.deepEqual(headerRange(['2019-02'], { from: '', to: '' }), { from: 'Feb 2019', to: 'Feb 2019', word: 'in view' });
  // The key tells two series lists apart, not merely their letters.
  assert.notEqual(barsQueryKey('gdfl', { keys: ['NSE-A', 'NSE-B'] }, null, 0), barsQueryKey('gdfl', { keys: ['NSE-ANSE-B'] }, null, 0));
});

test('the counted line counts each kind once and the right way round', () => {
  const cover = (/** @type {Partial<import('../src/lib/completeness.js').Cover>} */ c) =>
    /** @type {import('../src/lib/completeness.js').Cover} */ ({ basis: 'calendar', state: 'near', owed: 10, missing: 1, sessions: 2, heldSessions: 1.5, heldExact: false, notes: [], extra: 0, ...c });
  const t = coverTotals([
    { rows: 0, absent: true, cover: cover({ state: 'absent', missing: 10, sessions: 3, heldSessions: 0 }) },
    { rows: 0, absent: true, cover: cover({ state: 'absent', missing: 10, sessions: 3, heldSessions: 0 }) },
    { rows: 0, absent: true, cover: cover({ basis: 'trade', state: 'absent', owed: null, missing: null, sessions: 4, heldSessions: null }) },
    { rows: 7, cover: cover({ state: 'unmeasured', missing: null, notes: ['cas_unresolved'] }) },
    { rows: 7, cover: cover({ state: 'unmeasured', missing: null, notes: ['cas_unresolved'] }) },
    { rows: 9, cover: cover({ notes: ['cas_unresolved'] }) },
    // Covers no judgeRow writes, to prove each condition of the sums on its own.
    { rows: 9, cover: cover({ basis: 'trade', owed: 5, missing: 1 }) },
    { rows: 9, cover: cover({ owed: null, missing: 1 }) }
  ]);
  assert.equal(t.n, 8);
  assert.equal(t.absent, 2);
  assert.equal(t.noTrade, 1);
  assert.equal(t.stored, 5);
  assert.equal(t.trade, 2);
  assert.equal(t.unresolved, 2);
  assert.equal(t.sessions, 3 + 3 + 4 + 2 * 5);
  assert.equal(t.heldSessions, 1.5 * 5);
  assert.equal(t.missing, 10 + 10 + 1, 'only calendar rows with a count owed and a count missing');
  assert.equal(t.owed, 30);
  assert.equal(t.held, 9);
  // A row no calendar measured prints a dash, whatever it carries.
  assert.equal(missingText({ cover: cover({ basis: 'none', missing: 3 }) }), '—');
  assert.equal(missingText({ cover: cover({ state: 'unmeasured', missing: null }) }), '—');
  assert.equal(missingText({ cover: cover({ state: 'over', missing: null, extra: 4 }) }), '+4');
});
