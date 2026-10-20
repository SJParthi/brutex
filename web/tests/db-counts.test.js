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
  countedLine,
  headerRange,
  closeInWindow,
  contractTail,
  refusedFrontOf,
  coverPct,
  selectionKey,
  SELECTION_KEY_ROWS_MAX,
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
import { fmt } from '../src/lib/db/notes-labels.js';

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

// DB-048.
test('the_bars_badge_is_the_servers_count_on_both_tabs', () => {
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
  // WITH NO DAY OR TIME BOUND the census's count is still not this query's:
  // it counts the files, and on the real store it differs from the server's
  // with no bound at all (GDFL NIFTY 1s: 4,41,02,990 by a link straight to
  // Coverage, 4,40,32,544 on Bars). A dash, the census's figure in the title only.
  const unbounded = barsBadge({ said: null, remembered: null, key, census, narrows: false });
  assert.deepEqual([unbounded.n, unbounded.from], [null, 'none']);
  assert.match(unbounded.title, /^Not counted yet: only the server counts the bars this query keeps\. The census stores 443 bar\(s\)/);
  assert.match(unbounded.title, /never shown here as its count/);
  assert.match(barsBadge({ said: null, remembered: null, key, census, narrows: true }).title, /^Not counted yet: a date or time bound narrows/);
  // A SETTLED SELECTION THAT MATCHES NOTHING keeps 0 bars, counted without a
  // read (shadow, 20 Oct: Expired options with no expiry read "—" and "open
  // Bars" on the Bars tab itself, where nothing is ever read).
  const empty = barsBadge({ said: null, remembered: null, key, census: 0, narrows: false, empty: true });
  assert.deepEqual([empty.n, empty.from], [0, 'empty']);
  assert.match(empty.title, /keeps 0 bars/);
  // Not settled yet, or something matched: never a 0 of its own.
  assert.equal(barsBadge({ said: null, remembered: null, key, census: 0, narrows: false, empty: false }).n, null);
  assert.equal(barsBadge({ said: null, remembered: null, key, census: 0, narrows: false }).n, null);
  // A server count still wins: the read is the count.
  assert.equal(barsBadge({ said: 5, remembered: null, key, census: 0, narrows: false, empty: true }).n, 5);
  assert.match(SUMMARY, /empty: Boolean\(db\.settled\) && \(db\.matched\?\.length \?\? 0\) === 0/);
  // A plan the page has not built yet still keys.
  assert.equal(barsQueryKey('', null, null, undefined), '|||||||||');

  // THE COMPONENT READS THESE AND SENDS NOTHING FOR THEM. The badge and the
  // kept count are derived from the grid's own answer; no request is made.
  assert.match(SUMMARY, /barsBadge\(\{/);
  // Keyed by the matched rows' identity, never the grid's plan (empty off Bars).
  assert.match(SUMMARY, /const matchedKey = \$derived\(selectionKey\(db\.matched\)\);/);
  assert.match(SUMMARY, /barsQueryKey\(feeds\.active \?\? '', \{ keys: \[matchedKey\] \}, db\.barBounds, db\.nonce\)/);
  assert.doesNotMatch(declaration(SUMMARY, 'barsKey'), /barQueryPlan/);
  assert.doesNotMatch(declaration(SUMMARY, 'barsCount'), REQUEST);
  assert.doesNotMatch(declaration(SUMMARY, 'barsKey'), REQUEST);
  const keep = SUMMARY.slice(SUMMARY.indexOf('let barsCounted'), SUMMARY.indexOf('const barsCount ='));
  assert.doesNotMatch(keep, REQUEST);
  // Only a settled answer is kept, never the last query's while a new one loads.
  assert.match(keep, /!db\.barState\?\.loading/);
  // Both tabs print the one count each.
  assert.match(SUMMARY, /v\.key === 'bars' \? \(db\.barsCount\.n === null \? '—' : fmt\(db\.barsCount\.n\)\) : db\.covCount === null \? '…' : fmt\(db\.covCount\)/);
  // While the Bars tab's index is built the Coverage badge says so, never a part count.
  assert.match(SUMMARY, /: db\.covCount === null\s*\? db\.covCountWhy/);
  assert.doesNotMatch(SUMMARY, /db\.windowSaid \? db\.pageTotal : db\.total/, 'the census fallback on Coverage is gone');
  assert.doesNotMatch(SUMMARY, /: db\.sorted\.length\)/, 'the Coverage badge no longer reads the view-dependent rows');
});

// DB-046, R3-04.
test('coverage_follows_the_day_window_or_says_it_does_not', () => {
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
  assert.equal(
    windowEmptyWhy({ matched: 3, listed: 0, window, held: null }),
    'The From and To window, 01 Oct 2024 → 31 Oct 2024, holds no month of the 3 instrument-month(s) this selection matches. Coverage follows the window, as the Bars grid does: widen the dates to see them.'
  );
  assert.equal(windowEmptyWhy({ matched: 0, listed: 0, window, held: null }), '');
  assert.equal(windowEmptyWhy({ matched: 3, listed: 0, window: null, held: null }), '');
  assert.doesNotMatch(windowEmptyWhy({ matched: 3, listed: 0, window, held: null }), /this selection holds/);

  // THE GRID LISTS, COUNTS AND PAGES ONE SET. The listed rows are the
  // matched rows and the absent months, each in a month the window touches,
  // and the count is read on either tab.
  assert.match(COVERAGE, /const covWindow = \$derived\(coverWindow\(db\.fromDay \?\? '', db\.toDay \?\? '', Boolean\(db\.dayNarrowing\)\)\)/);
  assert.match(COVERAGE, /const listedStored = \$derived\(db\.matched\.filter\(\(\/\*\* @type \{any\} \*\/ r\) => monthInWindow\(r\.month, covWindow\)\)\)/);
  assert.match(declaration(COVERAGE, 'listedAbsent'), /if \(monthInWindow\(m, covWindow\)\) out\.push/);
  assert.match(COVERAGE, /const covCount = \$derived\(listedAbsent === null \? null : listedStored\.length \+ listedAbsent\.length\)/);
  assert.match(declaration(COVERAGE, 'judged'), /for \(const r of listedStored\)[\s\S]*for \(const a of listedAbsent \?\? \[\]\)/);
  // ON COVERAGE the count reads the grid's own whole-census index; ON BARS it
  // reads the matched instruments' rows from an index of the census built in
  // slices once per census read (R3-04 Mac pass: the whole-census walk per
  // change of selection measured p99 59-177 ms on the real census).
  assert.match(declaration(COVERAGE, 'matchedSeries'), /if \(db\.view === 'census'\) return seriesIndex;/);
  assert.match(declaration(COVERAGE, 'matchedSeries'), /if \(censusIndex\.deco !== db\.deco \|\| censusIndex\.by === null\) return null;/);
  assert.match(declaration(COVERAGE, 'matchedSeries'), /for \(const name of names\) for \(const r of censusIndex\.by\.get\(name\) \?\? \[\]\) rows\.push\(r\);\s*return indexSeries\(rows, null, rungs\);/);
  assert.doesNotMatch(declaration(COVERAGE, 'matchedSeries'), /db\.deco\)/, 'no walk of the whole census per selection');
  assert.match(COVERAGE, /inSlices\(deco\.length, \(from, to\) => \{/);
  assert.match(COVERAGE, /const seriesIndex = \$derived\(db\.view === 'census' \? indexSeries\(db\.deco, null, null\) : new Map\(\)\);/);
  assert.match(declaration(COVERAGE, 'listedAbsent'), /const index = matchedSeries;\s*if \(index === null\) return null;/);
  assert.match(declaration(COVERAGE, 'listedAbsent'), /absentMonthsOf\(index, r\.instrument, r\.timeframe\)/);
  assert.doesNotMatch(declaration(COVERAGE, 'listedAbsent'), REQUEST);
  // It says plainly that dates narrow Coverage, and why the grid is empty.
  assert.match(COVERAGE, /\{#if db\.covWindow\}[\s\S]*months touching <b>\{windowText\(db\.covWindow\)\}<\/b>/);
  assert.match(COVERAGE, /\{#if db\.covWindowEmpty\}\s*<!--[\s\S]*?-->\s*<p>\{db\.covWindowEmpty\}<\/p>/);
});

// R3-04: and the header describes the window in view.
test('the_header_prints_a_strike_as_a_price', () => {
  const key = 'NSE-FNO-NIFTY-2019-03-28-1165000-CE';
  // BEFORE: the last two dash-separated fields of the key, the strike in paisa.
  assert.equal(key.split('-').slice(-2).join(' '), '1165000 CE');
  // AFTER: the strike in rupees.
  assert.equal(contractTail(key), '11,650 CE');
  assert.equal(contractTail('NSE-FNO-BANKNIFTY-2020-03-26-2795050-PE'), '27,950.50 PE', 'paisa kept when a strike has them');
  assert.equal(contractTail('NSE-FNO-NIFTY-2019-03-28-FUT'), 'FUT');
  assert.equal(contractTail('NSE-INDEX-NIFTY'), '');
  assert.equal(contractTail('NSE-FNO-NIFTY-2019-03-28-FUTURE'), '', 'a future is the whole tail, not its start');
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

// R3-04 Mac pass: GDFL NIFTY 1s, 01-31 Mar 2020 on the shadow read "0 bar(s)
// missing" and "coverage — of 0 owed" for a month whose surplus is unplaced and
// whose 19 regular days lack 558 seconds. A row with no known missing count
// joins neither sum, and the line says so rather than read as nothing missing.
test('a_month_no_total_can_judge_never_reads_as_nothing_missing', () => {
  const rows = septemberRows();
  const over = rows.filter((r) => r.cover.state === 'over');
  assert.equal(over.length, 1);
  // ONLY UNJUDGEABLE ROWS: a dash, never 0, and never "of 0 owed".
  const only = coverTotals(over);
  assert.deepEqual([only.missing, only.owed, only.uncounted, only.over, only.counted], [0, 0, 1, 1, 0]);
  const line = countedLine(only);
  assert.equal(line.missing, '—');
  assert.equal(line.none, true);
  assert.equal(line.partial, false);
  assert.equal(line.owed, "(no row's missing count is known: 1 a month total cannot judge)");
  // BEFORE: the same totals printed as a count and an owed figure.
  assert.equal(`${fmt(only.missing)} bar(s) missing`, '0 bar(s) missing');
  // SOME COUNTED, SOME NOT: the sum stands and says what it covers.
  const all = coverTotals(rows);
  assert.equal(all.uncounted, 1, 'the over row; the trade-driven row is not a calendar row');
  assert.equal(all.counted, 3, 'full, short and absent: each a known missing count');
  assert.equal(all.n, all.counted + all.uncounted + all.trade + all.unverified, 'every row is one kind');
  const mixed = countedLine(all);
  assert.equal(mixed.missing, fmt(40 + 7875));
  assert.equal(mixed.partial, true);
  assert.equal(mixed.owed, 'of 23,625 owed, over the rows whose missing count is known; 1 more a month total cannot judge');
  // NOTHING UNJUDGEABLE: the plain figures, as before.
  const judged = coverTotals(rows.filter((r) => r.cover.state !== 'over'));
  assert.deepEqual(countedLine(judged), { missing: fmt(40 + 7875), owed: 'of 23,625 owed', why: '1 trade-driven, owed no fixed count', partial: false, none: false });
  // A TRADE-DRIVEN SELECTION (an option at 1s, owed no fixed count; shadow:
  // NIFTY 28 Mar 2019 11,600 CE on 15 Mar 2019 read "0 bar(s) missing" and
  // "coverage — of 0 owed"): a dash, and the kind named.
  const trade = coverTotals(rows.filter((r) => r.cover.basis === 'trade'));
  assert.deepEqual([trade.n, trade.trade, trade.counted], [1, 1, 0]);
  assert.deepEqual(countedLine(trade), { missing: '—', owed: "(no row's missing count is known: 1 trade-driven, owed no fixed count)", why: '1 trade-driven, owed no fixed count', partial: false, none: true });
  // NO CALENDAR, AND EVERY KIND AT ONCE: each named, in one order.
  const noCal = { rows: 5, cover: judgeRow(rows[0], { ledger: null }) };
  assert.equal(countedLine(coverTotals([noCal])).owed, "(no row's missing count is known: 1 no calendar could judge)");
  const every = countedLine(coverTotals([noCal, ...rows.filter((r) => r.cover.basis === 'trade' || r.cover.state === 'over')]));
  assert.equal(every.owed, "(no row's missing count is known: 1 a month total cannot judge; 1 trade-driven, owed no fixed count; 1 no calendar could judge)");
  // A TRADE ROW BESIDE COUNTED ONES is not "a month total cannot judge": the sum stands plain.
  assert.equal(countedLine(coverTotals(rows.filter((r) => r.cover.state !== 'over'))).partial, false);
  // NOTHING AT ALL: zero rows owe zero, and say 0, never a dash.
  assert.deepEqual(countedLine(coverTotals([])), { missing: '0', owed: 'of 0 owed', why: '', partial: false, none: false });
  assert.match(COVERAGE, /No row here has a known missing count \(\$\{db\.covLine\.why\}\)/);
  // A HALT MONTH THAT MEETS WHAT IT OWES is unmeasured, not over, and is still uncounted.
  const halt = { rows: 10, cover: { basis: 'calendar', state: 'unmeasured', owed: 10, missing: null, notes: ['halt_unplaced'], sessions: 1, heldSessions: 1, heldExact: true, extra: 0, surplus: 0 } };
  const h = coverTotals([/** @type {any} */ (halt)]);
  assert.deepEqual([h.uncounted, h.over, h.unresolved, countedLine(h).missing], [1, 0, 0, '—']);
  // The grid's line and the drawer read this one rule.
  assert.match(COVERAGE, /covLine: \{ get: \(\) => countedLine\(totals\) \}/);
  assert.match(COVERAGE, /<b>\{db\.covLine\.missing\}<\/b> bar\(s\) missing/);
  assert.match(COVERAGE, /coverage <b>\{coverPct\(db\.coverage\)\}<\/b> \{db\.covLine\.owed\}<\/span/);
  assert.match(declaration(COVERAGE, 'openTotals'), /line: countedLine\(t\)/);
  assert.match(COVERAGE, /\{db\.openTotals\.line\.missing\}/);
  assert.doesNotMatch(COVERAGE, /\{fmt\(db\.gaps\)\}<\/b> bar\(s\) missing/);
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
  assert.match(refusedFrontOf(cells, '')?.text ?? '', /^2 day\(s\) of a refused front, 13 Jan 2020 → 14 Jan 2020: the vendor served them/);
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
  assert.equal(missingText({ cover: cover({ state: 'over', missing: null, extra: 4, surplus: 9 }) }), '+9', 'the surplus above what is owed, not the part past the ceiling');
});

/* ======================================================================
   THE BADGE AS THE PAGE RUNS IT (R3-04 Mac pass, DB-048). The block of
   `DbSummary.svelte` from `let barsCounted` to the end of the `barsCount`
   declaration is cut out of the component and compiled as a rune module,
   so Svelte's client runtime runs its `$effect` and `$derived` exactly as
   on the page, driven through the states the Bars grid leaves `db` in: a
   deep link straight to Coverage, a read in flight, a read that settles,
   a second query asked while the first one's answer is still shown, and a
   tab left before that second answer lands. Every count is invented.
   ====================================================================== */

/** The client entry of Svelte (node resolves `svelte` to its server entry, whose effects never run). */
const SVELTE_CLIENT = import.meta.resolve('svelte').replace(/index-server\.js$/, 'index-client.js');

async function badgePage() {
  const { compileModule } = await import('svelte/compiler');
  const from = SUMMARY.indexOf('    let barsCounted = $state.raw(null);');
  const decl = SUMMARY.indexOf('    const barsCount = $derived(', from);
  const end = SUMMARY.indexOf('\n    );\n', decl);
  assert.ok(from > 0 && decl > from && end > decl, 'the badge block of DbSummary.svelte is where this test cuts it');
  const block = SUMMARY.slice(from, end + '\n    );\n'.length);
  const lib = new URL('../src/lib/db/notes-coverage.js', import.meta.url).href;
  const source = `import { untrack } from 'svelte';
import { barsBadge, barsQueryKey, selectionKey } from '${lib}';
export function mount(init) {
  const feeds = $state({ active: init.feed });
  const db = $state(init.db);
  /** @type {any} */
  let api = null;
  const stop = $effect.root(() => {
${block}
    api = { count: () => barsCount, key: () => barsKey };
  });
  return { feeds, db, stop, api };
}`;
  const { js } = compileModule(source, { filename: 'bars-badge.svelte.js', generate: 'client' });
  const code = js.code.replace(/from (["'])([^"']+)\1/g, (_m, q, p) => `from ${q}${p === 'svelte' ? SVELTE_CLIENT : import.meta.resolve(p)}${q}`);
  return import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${Math.random()}`);
}

// Runtime, DB-048: the badge keeps the server count across tabs and never files one query's count under another.
test('the_kept_bars_count_survives_the_tab_and_never_crosses_queries', async () => {
  const { flushSync } = await import(SVELTE_CLIENT);
  const plan = { timeframe: '1s', from: '2019-03', to: '2019-03', keys: ['NSE-FNO-SAMPLE-2019-03-28-1160000-CE'] };
  const day = { fromDay: '2019-03-15', toDay: '2019-03-15', fromTime: '', toTime: '' };
  const mod = await badgePage();
  // A DEEP LINK STRAIGHT TO COVERAGE: the Bars grid has read nothing.
  // THE GRID PLANS NOTHING OFF BARS: `barPlan` is empty unless the view is
  // `bars` (DbBarsGrid.svelte), so `db.barQueryPlan` is an empty plan on
  // Coverage. The first cloud build keyed the kept count by that plan and
  // so never matched it on Coverage (shadow, 19 Oct: "Bars —" on Coverage
  // after Bars counted 4,71,828). `db.matched` does not depend on the view.
  const empty = { keys: [], timeframe: '', from: '', to: '' };
  const matched = [{ instrument: 'NSE-FNO-SAMPLE-2019-03-28-1160000-CE', month: '2019-03', timeframe: '1s', rows: 443 }];
  const page = mod.mount({
    feed: 'gdfl',
    db: { view: 'census', matched, barState: { loading: false, answer: null }, windowSaid: null, barQueryPlan: empty, barBounds: day, nonce: 0, total: 443 }
  });
  const { db } = page;
  const count = () => page.api.count();
  /** What the Bars grid leaves in `db`: `barState`, the `windowSaid` derived from it, and its plan, empty off Bars. */
  const grid = (/** @type {boolean} */ loading, /** @type {number | null} */ total) => {
    db.barQueryPlan = db.view === 'bars' ? plan : empty;
    db.barState = { loading, answer: total === null ? null : { total } };
    db.windowSaid = total === null ? null : { total };
    flushSync();
  };
  flushSync();
  assert.deepEqual([count().n, count().from], [null, 'none'], 'a narrowed query nobody counted reads a dash, never the census 443');

  // Bars opens: its read is out, then settles at 434.
  db.view = 'bars';
  grid(true, null);
  assert.equal(count().n, null, 'nothing counted while the first read is out');
  grid(false, 434);
  assert.deepEqual([count().n, count().from], [434, 'server']);

  // Coverage: the grid drops its answer by design; the badge keeps 434.
  db.view = 'census';
  grid(false, null);
  assert.deepEqual([count().n, count().from], [434, 'server'], 'the Coverage tab keeps the Bars count, not the census 443');
  // OPENING COVERAGE READS THE WHOLE CENSUS, which rebuilds the matched rows as a
  // new array of the same rows (shadow, 19 Oct): the count is kept across it.
  db.matched = matched.map((r) => ({ ...r }));
  flushSync();
  assert.deepEqual([count().n, count().from], [434, 'server'], 'the same rows rebuilt are the same set');

  // Back on Bars, the read is out again (answer dropped): still 434.
  db.view = 'bars';
  grid(true, null);
  assert.equal(count().n, 434);
  grid(false, 434);

  // A SECOND QUERY WHILE THE FIRST ANSWER IS SHOWN: the To day moves, the
  // grid's read is out and it still holds the 434 answer. Nothing is kept
  // under the new query while it loads.
  const first = page.api.key();
  db.barBounds = { ...day, toDay: '2019-03-20' };
  grid(true, 434);
  assert.notEqual(page.api.key(), first);
  // The tab is left before the second answer lands: never the first query's 434.
  db.view = 'census';
  grid(false, null);
  assert.deepEqual([count().n, count().from], [null, 'none'], "one query's count is never shown for another");

  // The second answer lands on Bars: kept under its own query.
  db.view = 'bars';
  grid(true, null);
  grid(false, 2_170);
  db.view = 'census';
  grid(false, null);
  assert.equal(count().n, 2_170);

  // Back to the first window: its own count is no longer kept, so a dash.
  db.barBounds = day;
  flushSync();
  assert.equal(count().n, null);

  // A re-read of the store (the page's stamp moves) counts again.
  db.barBounds = { ...day, toDay: '2019-03-20' };
  flushSync();
  assert.equal(count().n, 2_170);
  db.nonce = 1;
  flushSync();
  assert.equal(count().n, null, 'a count taken before a re-read is not this read\'s count');

  // No bound at all and no server count: a dash, never the census's 443.
  db.barBounds = { fromDay: '', toDay: '', fromTime: '', toTime: '' };
  flushSync();
  assert.deepEqual([count().n, count().from], [null, 'none']);
  // Bars counts the unbounded query: that count, on either tab.
  db.view = 'bars';
  grid(true, null);
  grid(false, 440);
  db.view = 'census';
  grid(false, null);
  assert.deepEqual([count().n, count().from], [440, 'server'], 'the server\'s count, never the census\'s 443');
  // A time bound alone narrows too.
  db.barBounds = { fromDay: '', toDay: '', fromTime: '09:15', toTime: '' };
  flushSync();
  assert.equal(count().n, null);
  page.stop();
});

// R3-04 Mac pass: empty, one row, the largest figures, boundaries and garbage.
test('the_counts_at_their_extremes', async () => {
  // EMPTY: nothing judged is nothing owed, never 0.00% or 100.00%.
  const none = coverTotals([]);
  assert.deepEqual([none.n, none.owed, none.missing, none.coverage, none.heldExact], [0, 0, 0, null, true]);
  assert.equal(coverTotals(/** @type {any} */ (null)).n, 0, 'no rows at all reads as none');
  assert.equal(windowEmptyWhy({ matched: 0, listed: 0, window: { from: '2024-10-01', to: '2024-10-31' }, held: null }), '',
    'nothing matched before the window: the window is not what emptied it');
  assert.equal(headerRange([], null), null, 'no month on disk prints no range');
  assert.deepEqual(headerRange([], { from: '2024-10-01', to: '' }), { from: '01 Oct 2024', to: '—', word: 'in view' });
  assert.equal(refusedFrontOf([], ''), null);
  assert.equal(refusedFrontOf(null, ''), null);

  // ONE ROW: an absent month alone is its whole owed count missing.
  const cover = (/** @type {any} */ c) => ({ basis: 'calendar', state: 'gap', owed: 4_72_500, missing: 4_72_500, sessions: 21, heldSessions: 0, heldExact: true, notes: [], extra: 0, ...c });
  const one = coverTotals([{ rows: 0, absent: true, cover: cover({ state: 'absent' }) }]);
  assert.deepEqual([one.n, one.absent, one.missing, one.held, one.coverage], [1, 1, 4_72_500, 0, 0]);
  assert.equal(missingText({ absent: true, cover: cover({ state: 'absent' }) }), 'absent −4,72,500');

  // THE LARGEST FIGURES: a month of 1s bars (22,500 a session x 23 sessions)
  // missing one bar, and a sum over 2,13,000 such months, never round up.
  const whole = 22_500 * 23;
  assert.equal(coverPct((whole - 1) / whole), '99.99%');
  assert.equal(coverPct(1 - Number.EPSILON), '99.99%');
  assert.equal(coverPct(Number.MIN_VALUE), '<0.01%');
  assert.equal(coverPct(0), '0.00%');
  const many = coverTotals(Array.from({ length: 2_13_000 }, (_, i) =>
    ({ rows: whole - (i === 0 ? 1 : 0), cover: cover({ state: i === 0 ? 'near' : 'full', owed: whole, missing: i === 0 ? 1 : 0 }) })));
  assert.equal(many.missing, 1);
  assert.equal(many.owed, whole * 2_13_000);
  assert.ok(Number.isSafeInteger(many.owed), 'the largest owed sum is still an exact integer');
  assert.equal(coverPct(many.coverage), '99.99%', 'one bar missing in 110 billion is still not 100.00%');

  // MONTH AND YEAR BOUNDARIES: a window over New Year touches both months, no third.
  const ny = coverWindow('2019-12-31', '2020-01-01', true);
  assert.deepEqual(['2019-11', '2019-12', '2020-01', '2020-02'].map((m) => monthInWindow(m, ny)), [false, true, true, false]);
  // A leap day and a 30-day month's end: `-31` bounds every day as text.
  assert.equal(monthInWindow('2020-02', coverWindow('2020-02-29', '2020-02-29', true)), true);
  assert.equal(monthInWindow('2024-04', coverWindow('2024-04-30', '2024-05-01', true)), true);
  assert.equal(monthInWindow('2024-03', coverWindow('2024-04-01', '2024-04-30', true)), false);
  // An expiry on the last day of a month, at the window's edge.
  const expiry = Date.parse('2019-03-28T10:00:00Z');
  assert.equal(closeInWindow(expiry, coverWindow('2019-03-28', '2019-03-28', true), istDay), 'in');
  assert.equal(closeInWindow(expiry, coverWindow('2019-03-29', '2019-04-30', true), istDay), 'before');
  // IST, not UTC: 18:40 UTC on the 27th is 00:10 IST on the 28th.
  assert.equal(closeInWindow(Date.parse('2019-03-27T18:40:00Z'), coverWindow('2019-03-28', '2019-03-28', true), istDay), 'in');

  // OUT OF ORDER AND DUPLICATED INPUT: the range is ordered and each month once.
  assert.deepEqual(headerRange(['2020-03', '2018-09', '2020-03', '2019-01'], null), { from: 'Sep 2018', to: 'Mar 2020', word: 'on disk' });
  assert.deepEqual(coverWindow('2024-10-31', '2024-10-01', true), { from: '2024-10-01', to: '2024-10-31' }, 'typed backwards, put in order');
  const front = refusedFrontOf([{ day: '2018-09-03', kinds: ['front'] }, { day: '2018-09-04', kinds: ['front', 'gap'] }, { day: '2018-09-05' }], 'NSE-INDEX-SAMPLE');
  assert.deepEqual([front?.n, front?.from, front?.to], [2, '2018-09-03', '2018-09-04']);

  // A RENAMED OR ODD KEY: a strike is a price whatever the symbol holds.
  assert.equal(contractTail('NSE-FNO-NIFTY-2019-03-28-1165000-CE'), '11,650 CE');
  assert.equal(contractTail('NSE-FNO-M&M-2024-10-31-280000-PE'), '2,800 PE');
  assert.equal(contractTail('NSE-FNO-BANKNIFTY-2020-03-26-FUT'), 'FUT');
  assert.equal(contractTail('NSE-INDEX-NIFTY'), '', 'a spot key is no contract');
  assert.equal(contractTail(''), '');

  // GARBAGE PARAMETERS: an unknown window is no window, a non-number is a dash.
  assert.equal(coverWindow('', '', true), null);
  assert.equal(coverWindow('2024-10-01', '2024-10-31', false), null, 'seeded fields that narrow nothing are no window');
  for (const junk of [NaN, Infinity, -Infinity, null, undefined, /** @type {any} */ ('0.5')]) assert.equal(coverPct(junk), '—');
  assert.equal(barsBadge({ said: 0, remembered: null, key: 'k', census: 9, narrows: true }).n, 0, 'a server count of 0 is a count, never a dash');
  assert.equal(barsBadge({ said: null, remembered: { key: 'k', total: 0 }, key: 'k', census: 9, narrows: true }).n, 0, 'a kept 0 is a count too');
  assert.equal(whyText(null).startsWith('The server named a reason'), true);
  assert.equal(noteText('__proto__'), 'A coverage note this page has no sentence for: __proto__.');
  // THE SELECTION'S CONTENT NAMES IT: equal rows rebuilt in a new array are
  // one key, and no array yet is `none`, never a throw.
  const rowsA = [{ instrument: 'NSE-INDEX-SAMPLE', month: '2020-03', timeframe: '1s', rows: 1 }];
  const rowsB = [{ instrument: 'NSE-INDEX-SAMPLE', month: '2020-03', timeframe: '1s', rows: 1 }];
  assert.equal(selectionKey(rowsA), selectionKey(rowsB), 'equal rows rebuilt are the same set');
  const two = [{ instrument: 'NSE-INDEX-SAMPLE', month: '2020-03', timeframe: '1s', rows: 5 }, { instrument: 'NSE-INDEX-SAMPLE', month: '2020-04', timeframe: '1s', rows: 7 }];
  assert.equal(selectionKey(two), selectionKey([two[1], two[0]]), 'order is not identity');
  assert.notEqual(selectionKey(two), selectionKey([two[0]]), 'a month fewer is another set');
  assert.notEqual(selectionKey(two), selectionKey([two[0], { ...two[1], month: '2020-05' }]), 'another month is another set');
  assert.notEqual(selectionKey(two), selectionKey([two[0], { ...two[1], timeframe: '1min' }]), 'another rung is another set');
  assert.notEqual(selectionKey(two), selectionKey([two[0], { ...two[1], rows: 8 }]), 'a census that moved is another set');
  assert.notEqual(selectionKey(two), selectionKey([two[0], { ...two[1], instrument: 'NSE-INDEX-OTHER' }]), 'another series is another set');
  assert.equal(selectionKey([]), '0:0:0:0', 'nothing matched is its own set');
  // A row whose census count is not a number counts no bars, never NaN.
  const noCount = selectionKey([/** @type {any} */ ({ ...two[0], rows: undefined })]);
  assert.equal(noCount, selectionKey([{ ...two[0], rows: 0 }]));
  assert.match(String(noCount), /^1:0:/);
  // Two rows never trade parts: (A, March) and (B, April) are not (A, April) and (B, March).
  const ab = (/** @type {string} */ ma, /** @type {string} */ mb) => [
    { instrument: 'NSE-INDEX-SAMPLEA', month: ma, timeframe: '1s', rows: 5 },
    { instrument: 'NSE-INDEX-SAMPLEB', month: mb, timeframe: '1s', rows: 5 }
  ];
  assert.notEqual(selectionKey(ab('2020-03', '2020-04')), selectionKey(ab('2020-04', '2020-03')));
  for (const none of [null, undefined, 0, '']) assert.equal(selectionKey(/** @type {any} */ (none)), 'none');
  // THE BOUND: past MAX_SERIES x MAX_WINDOW_MONTHS rows nothing is read and
  // nothing is keyed (rows past the length would throw if they were read).
  const { MAX_SERIES, MAX_WINDOW_MONTHS } = await import('../src/lib/db/bars-plan.js');
  assert.equal(SELECTION_KEY_ROWS_MAX, MAX_SERIES * MAX_WINDOW_MONTHS);
  assert.equal(selectionKey(/** @type {any} */ ({ length: SELECTION_KEY_ROWS_MAX + 1 })), null);
  assert.notEqual(selectionKey(Array.from({ length: SELECTION_KEY_ROWS_MAX }, () => two[0])), null, 'at the bound it still keys');
  const unkeyed = barsBadge({ said: null, remembered: { key: 'k', total: 9 }, key: null, census: 443, narrows: true });
  assert.deepEqual([unkeyed.n, unkeyed.from], [null, 'none'], 'an unkeyed selection never reads a kept count');
});

/* ======================================================================
   THE BARS TAB'S CENSUS INDEX AS THE PAGE RUNS IT (R3-04 Mac pass). The
   block of `DbCoverageGrid.svelte` from `let censusIndex` to the end of
   `matchedSeries`, and its `indexSeries`, are cut out and compiled as a
   rune module, so Svelte's client runtime and the real `inSlices` run them.
   ====================================================================== */
async function indexPage() {
  const { compileModule } = await import('svelte/compiler');
  const from = COVERAGE.indexOf('    /** @type {{ deco: any[] | null, by: Map<string, any[]> | null, why: string }} */');
  const end = COVERAGE.indexOf('\n    });\n', COVERAGE.indexOf('    const matchedSeries = $derived.by(')) + '\n    });\n'.length;
  const fn = COVERAGE.indexOf('    function indexSeries(rows, names, rungs) {');
  const fnEnd = COVERAGE.indexOf('\n    }\n', COVERAGE.indexOf('      return by;', fn)) + '\n    }\n'.length;
  assert.ok(from > 0 && end > from && fn > 0 && fnEnd > fn, 'the index block of DbCoverageGrid.svelte is where this test cuts it');
  const lib = (/** @type {string} */ p) => new URL(`../src/lib/${p}`, import.meta.url).href;
  const source = `import { untrack } from 'svelte';
import { inSlices } from '${lib('census-worker.js')}';
import { istDayOf } from '${lib('completeness.js')}';
export function mount(init) {
  const db = $state(init.db);
  const derivedRungs = new Set(['2min', '1day']);
  const seriesIndex = 'the grid\\'s own whole-census index';
  /** @type {any} */
  let api = null;
  const stop = $effect.root(() => {
${COVERAGE.slice(from, end)}
${COVERAGE.slice(fn, fnEnd)}
    api = { series: () => matchedSeries };
  });
  return { db, stop, api };
}`;
  const { js } = compileModule(source, { filename: 'census-index.svelte.js', generate: 'client' });
  const code = js.code.replace(/from (["'])([^"']+)\1/g, (_m, q, p) => `from ${q}${p === 'svelte' ? SVELTE_CLIENT : import.meta.resolve(p)}${q}`);
  return import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${Math.random()}`);
}

test('the_bars_tab_counts_from_a_sliced_census_index_and_never_from_part_of_it', async () => {
  const { flushSync } = await import(SVELTE_CLIENT);
  const settle = async () => { for (let i = 0; i < 400; i++) await new Promise((done) => setImmediate(done)); flushSync(); };
  // An invented census: one index series at 1s and 2min (2min derived, so its
  // 1min source is read too), 20,000 rows of other instruments so the build
  // takes more than one slice.
  /** @param {string} instrument @param {string} timeframe @param {string} month */
  const row = (instrument, timeframe, month) => ({ instrument, timeframe, month, rows: 1, firstAt: null, lastAt: null });
  const mine = [row('NSE-INDEX-SAMPLE', '1s', '2020-01'), row('NSE-INDEX-SAMPLE', '1s', '2020-03'), row('NSE-INDEX-SAMPLE', '1min', '2019-12'), row('NSE-INDEX-SAMPLE', '2min', '2020-02')];
  const others = Array.from({ length: 20_000 }, (_, i) => row(`NSE-CASH-OTHER${i}`, '1s', '2020-01'));
  const deco = [...others.slice(0, 10_000), ...mine, ...others.slice(10_000)];
  const mod = await indexPage();
  const page = mod.mount({ db: { view: 'bars', deco, matched: [mine[0], mine[1]] } });
  flushSync();
  assert.equal(page.api.series(), null, 'nothing counted before the index is whole: no number for part of the census');
  await settle();
  const one = page.api.series();
  assert.ok(one instanceof Map);
  assert.deepEqual([...one.keys()], ['NSE-INDEX-SAMPLE\u00001s'], 'only the matched series at the matched rung');
  assert.deepEqual([...one.get('NSE-INDEX-SAMPLE\u00001s').months].sort(), ['2020-01', '2020-03']);
  // A derived rung brings its 1min source with it.
  page.db.matched = [mine[3]];
  flushSync();
  assert.deepEqual([...page.api.series().keys()].sort(), ['NSE-INDEX-SAMPLE\u00001min', 'NSE-INDEX-SAMPLE\u00002min']);
  // A NEW CENSUS: nothing is counted from the old one's index while the new one is built.
  page.db.deco = [...deco, row('NSE-INDEX-SAMPLE', '2min', '2020-04')];
  flushSync();
  assert.equal(page.api.series(), null);
  await settle();
  assert.deepEqual([...page.api.series().get('NSE-INDEX-SAMPLE\u00002min').months].sort(), ['2020-02', '2020-04']);
  // On Coverage the grid's own index is read, and no slice is started for it.
  page.db.view = 'census';
  flushSync();
  assert.equal(page.api.series(), "the grid's own whole-census index");
  page.stop();
});
