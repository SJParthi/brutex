// THE /db PAGER, THE READ'S ONE QUESTION A CHANGE, AND THE WORDS UNDER A
// REFUSED OR EMPTY GRID (R3-03; DB-N02, DB-021, DB-016, DB-062, DB-063).
//
// Each test drives the real module the grid imports — `pager.js`,
// `notes-bars.js`, `bars-plan.js` — and, where the rule lives in the grid's
// own effect, reads the grid's source for the line that applies it, the way
// `db-address.test.js` does. The bars below are invented shapes with prices
// nobody could take for real ones; no vendor row is quoted.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  MAX_SORT_DEPTH, MAX_READ_HELD, servedPages, isServed, lastServed, clampServed, stepServed, pageItems,
  parsePageBox, pageToRead, askOnce, readFor, generationOf, joinPage, joinSlices
} from '../src/lib/db/pager.js';
import { windowQuery, seriesPlan, rowKey, acrossDay, FIT_SLACK, MIN_USEFUL_ROWS, ROW, istInstant } from '../src/lib/db/bars-plan.js';
import { sortRefusal, segmentOfKey, localSortAdvice, readFailure, NO_VOLUME_SORT, NO_OI_SORT } from '../src/lib/db/notes-bars.js';
import { dbSource } from './database-page-fixture.js';

const CE = 'NSE-FNO-NIFTY-2019-03-28-1160000-CE';
const PE = 'NSE-FNO-NIFTY-2019-03-28-1160000-PE';
const SPOT = 'NSE-INDEX-NIFTY';
const SHARE = 'NSE-CASH-FIXTUREA';

/** The grid's script with its comments cut, so a test reads code, not prose. */
const gridScript = () => {
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  return grid.slice(0, grid.indexOf('</script>')).replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '');
};

/**
 * A `const` in a Rust file of crates/api/src, any visibility.
 * @param {string} file @param {string} name
 */
function rustConst(file, name) {
  const text = readFileSync(new URL(`../../crates/api/src/${file}`, import.meta.url), 'utf8');
  const found = text.match(new RegExp(`^pub(?:\\(super\\))? const ${name}: [a-z0-9]+ = ([0-9_]+);$`, 'm'));
  assert.ok(found, `no const ${name} in crates/api/src/${file}`);
  return Number(found[1].replaceAll('_', ''));
}

/**
 * THE READ EFFECT'S DECISION, run over the sequence of states one change
 * produces — exactly the three lines the effect runs (`pageToRead`, the
 * query, `reads.fresh`) — counting the requests that leave.
 *
 * @param {{order: string, page: number, held?: boolean}[]} runs the states the effect sees, in order
 * @param {number} size
 */
function requestsFor(runs, size) {
  const plan = seriesPlan([{ instrument: CE, month: '2019-03', timeframe: '1s', rows: 153_107 }]);
  const reads = askOnce();
  let lastOrder = '';
  /** @type {{offset: number, order: string}[]} */
  const sent = [];
  for (const run of runs) {
    const asked = pageToRead({ page: run.page, order: run.order, lastOrder, held: Boolean(run.held) });
    lastOrder = run.order;
    const [, sort, desc] = run.order.split('|');
    const query = windowQuery('gdfl', plan, { sort, desc: desc === 'true', offset: (asked - 1) * size, limit: size });
    if (reads.fresh(`${query}|set|0`)) sent.push({ offset: (asked - 1) * size, order: run.order });
  }
  return sent;
}

test('a_sort_change_at_a_deep_page_sends_one_request', () => {
  // Page 900 newest first, then Close. The read effect is created BEFORE the
  // page reset in the shell's order, so it sees the new order at the OLD page
  // first and page one only after the reset — two requests, the first at
  // offset 899 × 8, before R3-03.
  const runs = [
    { order: 'set|ts|true', page: 900 },
    { order: 'set|c|true', page: 900 },
    { order: 'set|c|true', page: 1 }
  ];
  const sent = requestsFor(runs, 8);
  assert.deepEqual(sent.slice(1), [{ offset: 0, order: 'set|c|true' }], 'one request for the change, at page one');
  // The other effect order (reset first) is the same one request.
  assert.deepEqual(requestsFor([runs[0], { order: 'set|c|true', page: 1 }, runs[2]], 8).slice(1), [{ offset: 0, order: 'set|c|true' }]);
  // A narrowing is a change of order too: one request, at page one.
  assert.deepEqual(requestsFor([runs[0], { order: 'set2|ts|true', page: 900 }, { order: 'set2|ts|true', page: 1 }], 8).slice(1), [{ offset: 0, order: 'set2|ts|true' }]);
  // A page an act chose with the order (an address's, a Go to's) is kept.
  assert.deepEqual(requestsFor([runs[0], { order: 'set|c|true', page: 5, held: true }], 8).slice(1), [{ offset: 32, order: 'set|c|true' }]);
  // A page turn in one order is one request; asking the same page again is none.
  assert.equal(requestsFor([runs[0], { order: 'set|ts|true', page: 901 }, { order: 'set|ts|true', page: 901 }], 8).length, 2);
  // The first read of a page keeps its page.
  assert.equal(pageToRead({ page: 7, order: 'a', lastOrder: '', held: false }), 7);
  assert.equal(pageToRead({ page: Number.NaN, order: 'a', lastOrder: 'a', held: false }), 1);
  // A forgotten question is asked again.
  const once = askOnce();
  assert.equal(once.fresh('q'), true);
  assert.equal(once.fresh('q'), false);
  assert.equal(once.asked, 'q');
  once.forget();
  assert.equal(once.asked, '');
  assert.equal(once.fresh('q'), true);

  // AND THE GRID'S EFFECT RUNS EXACTLY THESE LINES, waits for the selection to
  // settle, and is not torn down by a re-run that asks nothing new.
  const script = gridScript();
  assert.match(script, /const asked = pageToRead\(\{ page, order, lastOrder, held \}\);/);
  assert.match(script, /if \(!reads\.fresh\(`\$\{query\}\|\$\{set\}\|\$\{again\}`\)\) return;/);
  assert.match(script, /if \(!ready \|\| db\.reconciling \|\| jumping\) return;/);
  assert.match(script, /const held = Boolean\(db\.linkPending\?\.page\) \|\| pinned;/);
  assert.match(script, /\$effect\(\(\) => \(\) => inFlight\?\.abort\(\)\);/);
  assert.match(script, /if \(inFlight && inFlightSet !== set\) stopRead\(\);\s*if \(!ready/, 'a read for another row set never lands under the next');
  const read = script.slice(script.indexOf('const ready = !db.linkDaysPending'), script.indexOf('$effect(() => () => inFlight?.abort());'));
  assert.doesNotMatch(read, /return \(\) => controller\.abort\(\);/, 'a re-run must not cancel the question in flight');
});

test('one change of the selection sends one page read and one extremes read, and nothing while it settles', () => {
  const script = gridScript();
  const extremes = dbSource('../src/routes/db/DbExtremes.svelte');
  // The newest close is taken from the extremes read or page one newest first.
  assert.match(script, /if \(e && e\.for === setKey && !e\.loading && e\.newest !== undefined\) return newestOf\(e\.newest, planAll\);/);
  assert.match(script, /if \(a && front && Number\(a\.offset\) === 0\) return newestOf\(a\.bars\[0\], planAll\);/);
  // Asked alone only after both have had their say, once per selection.
  assert.match(script, /if \(barState\.loading \|\| \(!barState\.answer && !barState\.error\) \|\| db\.extremesRead\?\.loading\) return;\s*if \(!newestAsk\.fresh\(key\)\) return;/);
  assert.match(extremes, /newest: answer\.bars\[0\] \?\? null/);
  // The fold waits on the bar read's readiness and asks a set once.
  assert.match(extremes, /if \(!db\.barsReady\) return;/);
  assert.match(extremes, /if \(!asks\.fresh\(key\)\) return;/);
  assert.doesNotMatch(extremes, /return \(\) => controller\.abort\(\);/);
  assert.match(script, /const barsReady = \$derived\(\s*!db\.linkDaysPending && \(db\.sizeMode !== 'fit' \|\| \(db\.fitLatched && db\.pageSize === db\.rowsThatFit\)\) && !db\.reconciling\s*\);/);
});

test('the extremes of the set before are not offered for the set on screen', () => {
  // R3-03 repair 1. The fold waits for a new row set to settle before it asks,
  // and until then its last answer was the set BEFORE's: the panel said its
  // "over all N bar(s) of this query", offered its Go to buttons, and the grid
  // scaled its range bars by its widest bar.
  const before = { for: 'set-a', loading: false, value: { range: 9 } };
  assert.equal(readFor(before, 'set-a'), before);
  assert.equal(readFor(before, 'set-b'), null, 'a new set is not answered by the old one');
  assert.equal(readFor(before, ''), null, 'no set, no answer');
  assert.equal(readFor({ for: '', loading: false, value: null }, ''), null);
  assert.equal(readFor(null, 'set-a'), null);
  assert.equal(readFor(undefined, 'set-a'), null);
  const asking = { for: 'set-b', loading: true, value: null };
  assert.equal(readFor(asking, 'set-b'), asking, 'the read in flight for the set is its own');
  // Every reader of the fold reads it through this.
  const extremes = dbSource('../src/routes/db/DbExtremes.svelte');
  assert.match(extremes, /const readNow = \$derived\(readFor\(extremesRead, want\)\);/);
  assert.match(extremes, /const extremesNow = \$derived\(readNow && !readNow\.loading \? readNow\.value : null\);/);
  assert.match(extremes, /extremesRead: \{ get: \(\) => readNow \},/);
  assert.doesNotMatch(extremes, /extremesRead: \{ get: \(\) => extremesRead \}/);
});

test('the_pager_offers_no_page_past_the_served_depth', () => {
  // THE SERVER'S OWN NUMBERS, read out of the Rust source.
  assert.equal(MAX_SORT_DEPTH, rustConst('bars.rs', 'MAX_SORT_DEPTH'));
  assert.equal(MAX_READ_HELD, rustConst('bars_sort.rs', 'MAX_READ_HELD'));

  // A 153,107-row contract by time of day at 8 rows a page: 19,139 pages, and
  // the server serves rows 1 to 100,000 — pages 1 to 12,500.
  const tod = servedPages({ total: 153_107, size: 8, wire: 'tod' });
  assert.equal(tod.count, 19_139);
  assert.equal(tod.front, 12_500);
  assert.equal(tod.tail, null);
  assert.match(tod.why ?? '', /sort_too_deep/);
  assert.equal(lastServed(tod), 12_500, 'Last lands on the last page served');
  assert.equal(stepServed(tod, 12_500, 1), null, 'Next stops at it');
  assert.equal(stepServed(tod, 12_500, -1), 12_499);
  assert.equal(stepServed(tod, 1, -1), null);
  assert.equal(isServed(tod, 12_501), false);
  assert.equal(isServed(tod, 0), false);
  assert.equal(clampServed(tod, 19_139), 12_500, 'a page past the depth is never shown');
  assert.equal(clampServed(tod, -4), 1);
  assert.equal(clampServed(tod, 77), 77);
  const items = pageItems(12_500, tod);
  assert.ok(items.every((i) => i.gap || isServed(tod, i.p)), 'no button for a page the server refuses');
  assert.deepEqual(items.at(-1), { k: 'dead-end', p: 12_501, gap: true, dead: true }, 'the stretch past it says why');
  assert.ok(items.length <= 12, 'constant per paint');
  for (const typed of ['12501', '19139']) {
    const got = parsePageBox(typed, tod);
    assert.equal(got.ok, false, typed);
  }
  assert.match(/** @type {any} */ (parsePageBox('12501', tod)).why, /Page 12,501 is not served: .*sort_too_deep.* Type 1 to 12,500\./);
  assert.deepEqual(parsePageBox(' 12500 ', tod), { ok: true, page: 12_500 });

  // A VALUE ORDER the server took wholly from sort indexes serves every page.
  const indexed = servedPages({ total: 43_942_544, size: 50, wire: 'c', report: { by_time: false, indexed_rows: 43_942_544, scanned_rows: 0 } });
  assert.deepEqual(indexed, { count: 878_851, front: 878_851, tail: null, why: null });
  // Read rows within what one request orders whole: every page too.
  assert.equal(servedPages({ total: 153_107, size: 8, wire: 'c', report: { indexed_rows: 0, scanned_rows: 153_107 } }).why, null);
  // Open interest that is time order serves every page.
  assert.equal(servedPages({ total: 400_000, size: 8, wire: 'oi', report: { by_time: true } }).why, null);
  // Past it, with nothing indexed: both ends.
  const both = servedPages({ total: 2_000_000, size: 1000, wire: 'v', report: { indexed_rows: 0, scanned_rows: 2_000_000 } });
  assert.deepEqual([both.count, both.front, both.tail], [2000, 100, 1901]);
  assert.equal(isServed(both, 1900), false);
  assert.equal(isServed(both, 1901), true);
  assert.equal(lastServed(both), 2000);
  assert.equal(stepServed(both, 100, 1), 1901, 'Next jumps the refused stretch');
  assert.equal(stepServed(both, 1901, -1), 100);
  assert.equal(clampServed(both, 500), 100);
  const bothItems = pageItems(100, both);
  assert.ok(bothItems.some((i) => i.dead && i.p === 1901), 'the refused stretch is one item');
  assert.ok(bothItems.every((i) => i.gap || isServed(both, i.p)));
  assert.match(/** @type {any} */ (parsePageBox('1000', both)).why, /Type 1 to 100 or 1,901 to 2,000\./);
  // Mixed with index rows, or open interest: the front only.
  assert.equal(servedPages({ total: 2_000_000, size: 1000, wire: 'v', report: { indexed_rows: 9, scanned_rows: 1_500_000 } }).tail, null);
  assert.equal(servedPages({ total: 2_000_000, size: 1000, wire: 'oi', report: { indexed_rows: 0, scanned_rows: 1_500_000 } }).tail, null);
  // An order with no answer yet is held to the front depth.
  assert.equal(servedPages({ total: 153_107, size: 8, wire: 'h' }).front, 12_500);
  // A tail that meets the front serves every page; a tail past the count is none.
  assert.equal(servedPages({ total: 100_007, size: 8, wire: 'l', report: { indexed_rows: 0, scanned_rows: 2_000_000 } }).why, null);
  assert.equal(servedPages({ total: 100_100, size: 1000, wire: 'l', report: { indexed_rows: 0, scanned_rows: 2_000_000 } }).why, null);
  // Time order, a local sort and the census grid: every page.
  assert.equal(servedPages({ total: 32_43_567, size: 8, wire: 'ts' }).why, null);
  assert.equal(servedPages({ total: 5, size: 8, wire: null }).count, 1);
  assert.equal(servedPages({ total: 0, size: 0, wire: 'tod' }).count, 1);
  assert.equal(servedPages({ total: -3, size: 8, wire: 'tod' }).count, 1);
  // A short run draws every page.
  assert.deepEqual(pageItems(2, servedPages({ total: 30, size: 8, wire: 'ts' })).map((i) => i.p), [1, 2, 3, 4]);
  // A long one elides on both sides, and a gap is not a dead stretch.
  const long = pageItems(500, servedPages({ total: 8_000, size: 8, wire: 'ts' }));
  assert.deepEqual(long.map((i) => (i.gap ? '…' : i.p)), [1, '…', 499, 500, 501, '…', 1000]);
  assert.ok(long.every((i) => !i.dead));
  assert.deepEqual(pageItems(1000, servedPages({ total: 8_000, size: 8, wire: 'ts' })).map((i) => (i.gap ? '…' : i.p)), [1, '…', 997, 998, 999, 1000]);

  // AT A PAGE SIZE THAT DOES NOT DIVIDE THE DEPTH (R3-03 repair 1). Every
  // depth above is at 8 or 1,000 rows a page, which divide 100,000, so a
  // rounding the wrong way at either end gave the same answer and survived as
  // a mutant. The live Fit size on /db is 9.
  // A time-of-day order over the 153,107-row contract: 17,012 pages, the
  // front ends at 11,111 (offset + limit 99,999); 11,112 keeps 100,008 rows.
  const tod9 = servedPages({ total: 153_107, size: 9, wire: 'tod' });
  assert.deepEqual([tod9.count, tod9.front, tod9.tail], [17_012, 11_111, null]);
  assert.equal(isServed(tod9, 11_111), true);
  assert.equal(isServed(tod9, 11_112), false, 'the server refuses it sort_too_deep');
  assert.equal(lastServed(tod9), 11_111);
  assert.equal(clampServed(tod9, 17_012), 11_111);
  // A value order no index serves, over 1,153,107 read rows: 128,123 pages,
  // the tail starts at 117,013 (99,999 rows from the end); 117,012 starts
  // 100,008 rows from it.
  const tail9 = servedPages({ total: 1_153_107, size: 9, wire: 'c', report: { indexed_rows: 0, scanned_rows: 1_153_107 } });
  assert.deepEqual([tail9.count, tail9.front, tail9.tail], [128_123, 11_111, 117_013]);
  assert.equal(isServed(tail9, 117_012), false, 'the server refuses it sort_too_deep');
  assert.equal(isServed(tail9, 117_013), true);
  assert.equal(stepServed(tail9, 11_111, 1), 117_013, 'Next jumps to the first page the tail serves');
  assert.equal(stepServed(tail9, 117_013, -1), 11_111);
  // And against the route's own rule (`bars_sort.rs`, D-3349), page by page at
  // each edge, for sizes that do and do not divide the depth: a page is served
  // while offset + limit <= MAX_SORT_DEPTH, or, nothing indexed, while
  // total - offset <= MAX_SORT_DEPTH.
  const route = (/** @type {number} */ total, /** @type {number} */ size, /** @type {number} */ p, /** @type {boolean} */ fromEnd) =>
    (p - 1) * size + size <= MAX_SORT_DEPTH || (fromEnd && total - (p - 1) * size <= MAX_SORT_DEPTH);
  for (const size of [1, 3, 7, 8, 9, 11, 13, 25, 50, 97, 999, 1000, 1001]) {
    for (const total of [153_107, 1_153_107, 2_000_003]) {
      for (const [wire, report, fromEnd] of /** @type {[string, any, boolean][]} */ ([
        ['tod', undefined, false],
        ['c', { indexed_rows: 0, scanned_rows: 2_000_003 }, true],
        ['v', { indexed_rows: 4, scanned_rows: 2_000_003 }, false]
      ])) {
        const s = servedPages({ total, size, wire, report });
        const edges = [s.front - 1, s.front, s.front + 1, s.count];
        if (s.tail !== null) edges.push(s.tail - 1, s.tail, s.tail + 1);
        for (const p of edges.filter((x) => x >= 1 && x <= s.count)) {
          assert.equal(isServed(s, p), route(total, size, p, fromEnd), `${wire} ${total} rows at ${size} a page: page ${p}`);
        }
      }
    }
  }

  // THE PAGER DRAWS THESE: no button, box or step that reaches a dead page.
  const pager = dbSource('../src/routes/db/DbPager.svelte');
  assert.match(pager, /\{#each pageItems\(db\.pageNow, db\.served\) as item \(item\.k\)\}/);
  assert.match(pager, /const next = \$derived\(stepServed\(db\.served, db\.pageNow, 1\)\);/);
  assert.match(pager, /onclick=\{\(\) => db\.goPage\(last\)\}/);
  assert.match(pager, /const got = parsePageBox\(el\.value, db\.served\);/);
  assert.match(gridScript(), /db\.page = clampServed\(served, n\);/);
});

test('the page box refuses 0, -5, 2.5, 1e308, empty, letters and 6,000 digits in words', () => {
  const s = servedPages({ total: 39_115_464, size: 8, wire: 'ts' });
  assert.equal(s.count, 4_889_433);
  for (const typed of ['0', '-5', '2.5', '1e308', '', '   ', 'abc', '9'.repeat(6000), '999999999999', '4889434']) {
    const got = parsePageBox(typed, s);
    assert.equal(got.ok, false, JSON.stringify(typed.slice(0, 20)));
    const why = /** @type {any} */ (got).why;
    assert.ok(why.length < 260, 'never the 6,000 digits back');
    assert.match(why, /Type (a page number, |a whole page number, )?1 to 48,89,433\.$/);
  }
  assert.match(/** @type {any} */ (parsePageBox('9'.repeat(6000), s)).why, /6,000 characters long/);
  assert.match(/** @type {any} */ (parsePageBox('0', s)).why, /is outside 1 to 48,89,433/);
  assert.deepEqual(parsePageBox('4889433', s), { ok: true, page: 4_889_433 });
  assert.deepEqual(parsePageBox(null, servedPages({ total: 10, size: 8, wire: 'ts' })), { ok: false, why: 'Type a page number, 1 to 2.' });
  // A number is read as what it prints: 0 is the page it names, not an empty box.
  assert.deepEqual(parsePageBox(0, servedPages({ total: 10, size: 8, wire: 'ts' })), { ok: false, why: 'Page "0" is outside 1 to 2: this selection has 2 page(s) at this many rows a page. Type 1 to 2.' });
});

test('the store state two pages are compared by is the total and every series count', () => {
  assert.equal(generationOf(null), '');
  assert.equal(generationOf({ total: 9, series_totals: [{ key: CE, total: 4 }, { key: PE, total: 5 }] }), `9|${CE}=4,${PE}=5`);
  assert.equal(generationOf({ total: 0 }), '0|');
});

/**
 * A store of one series, newest first, as the window route pages it.
 * @param {number} n @param {number} offset @param {number} limit
 */
function newestFirst(n, offset, limit) {
  const all = Array.from({ length: n }, (_, i) => ({ t: 1_552_620_600 + i, c: 10_000 + i })).reverse();
  return { total: n, series_totals: [{ key: CE, total: n }], bars: all.slice(offset, offset + limit) };
}

test('the_store_growing_between_two_pages_repeats_no_row', () => {
  const key = (/** @type {any} */ b) => rowKey(CE, b.t);
  // Page 1 of a 100-bar series, 10 a page; an import adds 3 newer bars; page 2.
  const one = newestFirst(100, 0, 10);
  const two = newestFirst(103, 10, 10);
  const prev = { rows: one.bars, generation: generationOf(one), total: one.total };
  const naive = two.bars.filter((b) => one.bars.some((a) => a.t === b.t));
  assert.equal(naive.length, 3, 'unchecked, page 2 repeats the last three rows of page 1');
  const joined = joinPage(prev, { rows: two.bars, generation: generationOf(two), total: two.total }, key);
  assert.equal(joined.dropped, 3);
  assert.equal(joined.rows.length, 7);
  assert.ok(joined.rows.every((b) => !one.bars.some((a) => a.t === b.t)), 'no row of page 1 again');
  assert.match(joined.moved ?? '', /grew by 3 bar\(s\) between the page before and this one \(100 then 103\)/);
  assert.match(joined.moved ?? '', /3 row\(s\) already shown on the page before are dropped here/);
  // The same store: nothing dropped, nothing said.
  const same = newestFirst(100, 10, 10);
  assert.deepEqual(joinPage(prev, { rows: same.bars, generation: generationOf(same), total: 100 }, key), { rows: same.bars, dropped: 0, moved: null });
  assert.deepEqual(joinPage(null, { rows: same.bars, generation: 'x', total: 100 }, key).moved, null);
  // A store that shrank, or changed with no repeat, is still said.
  const shrank = newestFirst(98, 10, 10);
  assert.match(joinPage(prev, { rows: shrank.bars, generation: generationOf(shrank), total: 98 }, key).moved ?? '', /shrank by 2 bar\(s\).*no row of the page before came back/);
  assert.match(joinPage(prev, { rows: [], generation: 'other', total: 100 }, key).moved ?? '', /The store changed bar\(s\)/);

  // A WHOLE READ (a sort the browser does): a slice shifted by a write repeats
  // rows; each is dropped and the write is named.
  const head = { total: 4, series_totals: [{ key: CE, total: 4 }] };
  const later = { total: 5, series_totals: [{ key: CE, total: 5 }] };
  const rows = [{ t: 1 }, { t: 2 }, { t: 2 }, { t: 3 }, { t: 4 }];
  const slices = joinSlices(rows, [head, head, later], key);
  assert.deepEqual(slices.bars.map((b) => b.t), [1, 2, 3, 4]);
  assert.equal(slices.dropped, 1);
  assert.match(slices.moved ?? '', /The store changed while this sort read its rows \(4 bar\(s\) when it began, 5 at its last slice\); 1 repeated row\(s\) were dropped\./);
  assert.deepEqual(joinSlices([{ t: 1 }, { t: 2 }], [head, head], key), { bars: [{ t: 1 }, { t: 2 }], dropped: 0, moved: null });
  assert.match(joinSlices([{ t: 1 }], [head, later], key).moved ?? '', /^The store changed while this sort read its rows \(4 bar\(s\) when it began, 5 at its last slice\)\. The order/);
  assert.match(joinSlices([{ t: 1 }, { t: 1 }], [], key).moved ?? '', /\(0 bar\(s\) when it began, 0 at its last slice\)/);

  // THE GRID JOINS EVERY PAGE TO THE ONE BEFORE IN THE SAME ORDER, AND EVERY
  // WHOLE READ'S SLICES, AND SAYS THE MOVE UNDER THE GRID.
  const script = gridScript();
  assert.match(script, /const stepped = !next\.local && before && before\.order === order && Math\.abs\(before\.page - asked\) === 1;/);
  assert.match(script, /joinPage\(before, \{ rows: next\.rows, generation, total \}/);
  assert.match(script, /const joined = joinSlices\(got\.bars, \[head, \.\.\.bodies\]/);
  assert.match(dbSource('../src/routes/db/DbBarsGrid.svelte'), /\{#if db\.barState\.moved\}\s*<p class="bnote warn" title=\{db\.barState\.moved\}>\{db\.barState\.moved\}<\/p>/);
});

test('no_sort_is_offered_on_a_column_without_values', () => {
  assert.equal(segmentOfKey(SPOT), 'INDEX');
  assert.equal(segmentOfKey(''), '');
  assert.equal(segmentOfKey(/** @type {any} */ (null)), '');
  assert.equal(sortRefusal('vol', [SPOT]), NO_VOLUME_SORT, 'an index has no traded volume');
  assert.equal(sortRefusal('vol', [SPOT, CE]), null, 'a selection with a derivative keeps it');
  assert.equal(sortRefusal('vol', [SHARE]), null, 'a share trades');
  assert.equal(sortRefusal('oi', [SPOT]), NO_OI_SORT);
  assert.equal(sortRefusal('oi', [SHARE, SPOT]), NO_OI_SORT, 'an index or a share has no open interest');
  assert.equal(sortRefusal('oichg', [SHARE]), NO_OI_SORT);
  assert.equal(sortRefusal('oi', [CE]), null);
  assert.equal(sortRefusal('c', [SPOT]), null);
  assert.equal(sortRefusal('vol', []), null, 'nothing selected refuses nothing');
  // The server's own rule, read out of its source: an INDEX bar has no
  // volume, an INDEX or CASH bar no open interest.
  const rules = readFileSync(new URL('../../crates/api/src/bars_change.rs', import.meta.url), 'utf8');
  assert.match(rules, /fn not_a_derivative\(&self\) -> bool \{\s*matches!\(self\.segment, Some\(Segment::Index \| Segment::Cash\)\)/);
  assert.match(rules, /fn index_no_volume\(&self\) -> bool \{\s*self\.segment == Some\(Segment::Index\)/);
  // The grid disables the button with the reason, reads time order for a sort
  // carried onto such a selection, and checks before a press moves the sort.
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  assert.match(grid, /disabled=\{Boolean\(c\.none \|\| c\.nosort \|\| db\.sortOff\(c\.key\)\)\}/);
  assert.match(grid, /Not sortable on this selection: \$\{db\.sortOff\(c\.key\)\}/);
  const script = gridScript();
  assert.match(script, /const sortNow = \$derived\(sortOff\(barSortKey\) \? \{ key: 'ts', desc: true \} : \{ key: barSortKey, desc: barDesc \}\);/);
  assert.match(script, /if \(!col \|\| col\.none \|\| col\.nosort \|\| sortOff\(key\)\) return;/);
});

test('a sort the page cannot do keeps the sort that was on, with advice that works for the rung', () => {
  // One-second NIFTY: a day can hold 86,400 bars, past the 20,000 the browser
  // orders — so "narrow to a day" alone is never the advice.
  const second = localSortAdvice({ label: 'Change %', total: 382_496, budget: 20_000, tfSecs: 1, series: 1, stored: ['Date', 'Close'] });
  assert.doesNotMatch(second, /narrow to one day \(/);
  assert.match(second, /narrow to one day AND a time of day of at most 333 minute\(s\) — a whole day of this series can hold 86,400 bar\(s\) at this rung/);
  assert.match(second, /The grid keeps the order it had\./);
  assert.match(second, /or sort by a stored column \(Date, Close\)/);
  // One-minute: a day is 1,440 bars, so a day is the advice.
  const minute = localSortAdvice({ label: 'Change %', total: 30_000, budget: 20_000, tfSecs: 60, series: 1, stored: ['Date'] });
  assert.match(minute, /narrow to one day \(a day holds at most 1,440 bar\(s\) of this series at this rung\)/);
  assert.doesNotMatch(minute, /one strike or one side/);
  assert.match(localSortAdvice({ label: 'Strike', total: 30_000, budget: 20_000, tfSecs: 60, series: 2, stored: ['Date'] }), /narrow to one day \(a day holds at most 2,880 bar\(s\) of these 2 series at this rung\)/);
  // Many series: the series count is named, and the day bound counts them all.
  const chain = localSortAdvice({ label: 'Strike', total: 900_000, budget: 20_000, tfSecs: 60, series: 40, stored: ['Date'] });
  assert.match(chain, /a whole day of these 40 series can hold 57,600 bar\(s\)/);
  assert.match(chain, /at most 500 minute\(s\)/);
  assert.match(chain, /choose one strike or one side \(40 series now\)/);
  const wide = localSortAdvice({ label: 'Strike', total: 9e6, budget: 20_000, tfSecs: 1, series: 1000, stored: ['Date'] });
  assert.match(wide, /choose fewer series — one minute of these 1,000 series is already past it/);
  assert.match(localSortAdvice({ label: 'x', total: 1, budget: 20_000, tfSecs: 0, series: 0, stored: [] }), /a whole day of this series can hold 86,400/);
  // The grid refuses before the sort moves when the count on screen is past it.
  const script = gridScript();
  assert.match(script, /if \(!wireSort\(key\) && total > LOCAL_SORT_BUDGET\) \{\s*sortWhy = sortAdvice\(key, total\);\s*return;\s*\}/);
  assert.doesNotMatch(dbSource('../src/routes/db/DbBarsGrid.svelte'), /Narrow to a day, a time of day, a strike or a side/);
});

test('a sort already on that the page cannot do says the grid went back to time order, and no sentence loses a space', () => {
  // Shadow :9502, 17 Oct: `&sort=dte&dir=desc` on the 153,107-row 11,600 CE
  // went back to time order (caret on Date) under "The grid keeps the order it
  // had." A header press keeps the sort that was on; a sort already on cannot.
  const args = { label: 'Days to exp', total: 153_107, budget: 20_000, tfSecs: 1, series: 1, stored: ['Date', 'Close'] };
  const left = localSortAdvice({ ...args, kept: false });
  assert.match(left, /of a column the server cannot sort\. The grid is back in time order, newest first\. To order by Days to exp:/);
  assert.doesNotMatch(left, /keeps the order it had/);
  assert.match(localSortAdvice(args), /of a column the server cannot sort\. The grid keeps the order it had\. To order by Days to exp:/);
  assert.equal(localSortAdvice({ ...args, kept: true }), localSortAdvice(args));
  const script = gridScript();
  assert.match(script, /sortWhy = sortAdvice\(key, next\.refused, false\);\s*barState = \{ \.\.\.untrack\(\(\) => barState\), loading: false \};\s*barSortKey = 'ts';\s*barDesc = true;/);
  assert.match(script, /function sortAdvice\(key, total, kept = true\) \{\s*return localSortAdvice\(\{\s*label: BAR_META\[key\]\?\.label \?\? key,\s*total,\s*kept,/);
  // Svelte drops the leading space of a block's text, so the shadow read
  // "page 1 of 48,92,505· opening page 2…" and "inside that time of dayon 15 Mar
  // 2019". The space now travels inside an expression, where it is kept.
  assert.match(dbSource('../src/routes/db/DbPager.svelte'), /\{#if opening\}\{` · opening page \$\{fmt\(opening\)\}…`\}\{\/if\}/);
  assert.match(dbSource('../src/routes/db/DbBarsGrid.svelte'), /opens inside that time of day\{#if db\.emptyNote\.open\.length\}\{' on '\}\{db\.emptyNote\.open\.map/);
});

test('a_page_made_refusal_is_not_called_the_servers', () => {
  const page = readFailure({ made: 'page', status: 0 });
  assert.equal(page.kind, 'page');
  assert.equal(page.retry, false, 'asking again asks the same impossible question');
  assert.doesNotMatch(`${page.title} ${page.says}`, /server's own|failed/);
  assert.match(page.says, /before asking the server, so nothing was read/);
  const refused = readFailure({ made: 'read', status: 400 });
  assert.deepEqual([refused.kind, refused.retry], ['server', false]);
  assert.match(refused.says, /the server's own \(HTTP 400\)/);
  for (const status of [0, 200, 408, 429, 500, 503]) {
    const failed = readFailure({ made: 'read', status });
    assert.deepEqual([failed.kind, failed.retry], ['failed', true], String(status));
  }
  assert.match(readFailure({ made: 'read', status: 503 }).says, /HTTP 503/);
  assert.match(readFailure({ made: 'read', status: 200 }).says, /did not check/);
  // The grid says what `readFailure` says, offers Try again only when it can
  // help, records who refused, and asks the PAGE again rather than the census.
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  assert.match(grid, /<h3>\{db\.barFailure\?\.title\}<\/h3>/);
  assert.match(grid, /\{#if db\.barFailure\?\.retry\}\s*<button class="btn" onclick=\{\(\) => db\.retryRead\(\)\}>Try again<\/button>/);
  assert.doesNotMatch(grid, /The\s+sentence above is the server's own\./);
  const script = gridScript();
  assert.match(script, /barState = \{ loading: false, error: p\.why, answer: null, rows: \[\], local: false, page: 1, set, made: 'page', status: 0, moved: null \};/);
  assert.match(script, /seen\.status = response\.status;/);
});

test('an empty answer is said only once it is in, and a time window is not blamed on the import (DB-016)', () => {
  const script = gridScript();
  assert.match(script, /if \(db\.view !== 'bars' \|\| barState\.loading \|\| barState\.error \|\| !a \|\| a\.total > 0 \|\| barState\.set !== setKey\) return null;/);
  assert.match(script, /time: timeNarrows,/);
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  // No answer yet draws the loading rows, never an empty or a census accusation.
  assert.match(grid, /\{:else if \(db\.barState\.loading && db\.barPage\.length === 0\) \|\| \(!db\.barState\.answer && !db\.barState\.error\) \|\| \(db\.barPage\.length === 0 && db\.barState\.set !== db\.barSetKey\)\}/);
  // The census and the files are said to disagree only when the census has not moved.
  assert.match(grid, /\{#if db\.barsClaimed > 0 && db\.censusMoved === false\}/);
  // A time window: the window is named, the import is not, nor the day band's import reasons.
  const time = grid.slice(grid.indexOf('{#if db.emptyNote.time}'), grid.indexOf('{:else if db.emptyNote.open.length}'));
  assert.match(time, /The time window is what keeps nothing/);
  assert.doesNotMatch(time, /import may have refused/);
  assert.match(grid, /\{#if db\.emptyNote\.time\}\s*<!--[^>]*-->\s*\{:else if db\.dayEmptyWhy\}/);
});

test('Rows per page: no "Fit · 50" before the box is measured, and the heading is measured with it (DB-N02)', () => {
  const pager = dbSource('../src/routes/db/DbPager.svelte');
  assert.match(pager, /summary=\{db\.sizeMode === 'fit' \? \(db\.fitLatched \? `Fit · \$\{fmt\(db\.pageSize\)\}` : 'Fit · measuring'\) : fmt\(db\.pageSize\)\}/);
  assert.equal(FIT_SLACK, 8);
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  assert.match(grid, /fitHead = \/\*\* @type \{HTMLElement \| null\} \*\/ \(el\.querySelector\('thead'\)\)\?\.offsetHeight \|\| HEAD;/);
  assert.match(grid, /Math\.max\(1, Math\.floor\(\(\(fitBoxH \|\| barBoxH\) - fitHead - FIT_SLACK\) \/ ROW\)\)/);
  // The notes strip is always in the box at one line, so no note cuts a row.
  assert.match(grid, /<div class="bnotes" class:open=\{notesOpen\} data-notes=\{noteCount\}>/);
  assert.match(grid, /height: calc\(2 \* var\(--s3\) \+ 1\.55 \* var\(--fs-xs\) \+ 1px\);/);
  assert.match(grid, /\.bnotes\.open \.bnotes-list \{\s*position: absolute;/);
  // The readout is the page the rows are, not the page asked.
  assert.match(pager, /<b>\{fmt\(db\.pageShown\)\}<\/b> of <b>\{fmt\(db\.pageCount\)\}<\/b>\{#if opening\}\{` · opening page \$\{fmt\(opening\)\}…`\}\{\/if\}/);
  assert.match(gridScript(), /db\.view !== 'bars' \|\| barState\.local \|\| !barState\.answer \? pageNow : clampServed\(served, barState\.page\)/);
  // The pager is its own file, rendered by the snippet the shell renders.
  assert.match(grid, /\{#snippet pager\(\/\*\* @type \{any\} \*\/ db\)\}[\s\S]*<DbPager \{db\} \/>\s*\{\/snippet\}/);
});

test('the grid box at its floor holds ten rows by the Fit’s own count', () => {
  // R3-03 repair 1. The floor named ten rows and gave nine: every box is
  // border-box, so the `.tbl` floor holds its own 1px border above and below,
  // and the scroll box under it was 458px — 458 - 52 - 8 = 398, nine 40px rows.
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  const theme = dbSource('../src/lib/theme.css');
  assert.match(theme, /\*\s*\{\s*box-sizing: border-box;/, 'every box is border-box');
  /** A theme length, in px. @param {string} name */
  const token = (name) => {
    const m = theme.match(new RegExp(`${name}: ([0-9.]+)px;`));
    if (!m) throw new Error(`no ${name} in theme.css`);
    return Number(m[1]);
  };
  /** A `calc()` of `Npx`, `N * Npx` and `N * var(--x)` terms, in px. @param {string} expr */
  const calc = (expr) => expr.split(' + ').reduce((sum, term) => {
    const m = term.trim().match(/^(?:([0-9.]+) \* )?(?:([0-9.]+)px|var\((--[a-z0-9-]+)\))$/);
    if (!m) throw new Error(`a calc() term this test cannot read: ${term}`);
    return sum + Number(m[1] ?? 1) * (m[2] !== undefined ? Number(m[2]) : token(/** @type {string} */ (m[3])));
  }, 0);
  const rule = (/** @type {string} */ selector) => {
    const m = grid.match(new RegExp(`\\n  ${selector.replace('.', '\\.')} \\{([^}]*)\\}`));
    if (!m) throw new Error(`no ${selector} rule`);
    return m[1];
  };
  const tbl = rule('.tbl');
  const floor = calc(/** @type {string[]} */ (tbl.match(/min-height: calc\((.*)\);/))[1]);
  const border = Number(/** @type {string[]} */ (tbl.match(/border: ([0-9]+)px solid/))[1]);
  const notes = calc(/** @type {string[]} */ (rule('.bnotes').match(/height: calc\((.*)\);/))[1]);
  // The scroll box is what is left inside the border once the notes strip stands.
  const scroll = floor - 2 * border - notes;
  // The heading's two rows as the floor's own comment names them, measured at
  // 52px on the shadow (22px band, 30px labels).
  const head = 22 + 30;
  assert.equal(scroll, 460);
  assert.equal(ROW, 40);
  assert.equal(Math.max(1, Math.floor((scroll - head - FIT_SLACK) / ROW)), 10);
  assert.ok(Math.floor((scroll - head - FIT_SLACK) / ROW) >= MIN_USEFUL_ROWS, 'the fold is not asked for by the floor itself');
});

test('a Change % measured across the overnight gap says so on the row', () => {
  const day = (/** @type {string} */ d, /** @type {string} */ t) => /** @type {number} */ (istInstant(d, t)) * 1000;
  /** @param {string} key @param {string} d @param {string} t @param {number | null} chg */
  const row = (key, d, t, chg) => ({ rk: rowKey(key, day(d, t) / 1000), key, ts: day(d, t), day: d, chg });
  const rows = [
    row(SPOT, '2026-09-02', '09:15', 812),
    row(SPOT, '2026-09-01', '15:29', 3),
    row(SPOT, '2026-09-02', '09:16', -4),
    row(SPOT, '2026-09-01', '15:28', null),
    row(CE, '2026-09-02', '09:15', 40),
    row(CE, '2026-09-01', '09:15', null)
  ];
  const marks = acrossDay(rows, { timeNarrows: false, dayRung: false, edgeEarlier: false, contiguous: true });
  assert.deepEqual([...marks.keys()].sort(), [rows[0].rk, rows[4].rk].sort());
  assert.equal(marks.get(rows[0].rk), rows[1].ts, 'the bar it was measured against');
  // The first row of a read that began at a day window's first day.
  const edge = acrossDay([rows[0], rows[2]], { timeNarrows: false, dayRung: false, edgeEarlier: true, contiguous: true });
  assert.equal(edge.get(rows[0].rk), null);
  assert.equal(edge.has(rows[2].rk), false);
  assert.equal(acrossDay([rows[0], rows[2]], { timeNarrows: false, dayRung: false, edgeEarlier: false, contiguous: true }).size, 0, 'not known, not said');
  assert.equal(acrossDay(rows, { timeNarrows: true, dayRung: false, edgeEarlier: true, contiguous: true }).size, 0, 'a time window proves nothing');
  assert.equal(acrossDay(rows, { timeNarrows: false, dayRung: true, edgeEarlier: true, contiguous: true }).size, 0, 'a day rung is close to close');
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  assert.match(grid, /\{#if db\.overnight\.has\(b\.rk\)\}/);
  assert.match(grid, />overnight<\/span/);
  assert.match(gridScript(), /acrossDay\(barRows, \{ timeNarrows, dayRung, edgeEarlier: barState\.local && dayWindowNarrows, contiguous: rowsInTime \}\)/);
});

test('a page the server ordered by a value marks no overnight row: its rows are not neighbours in their series', () => {
  // Shadow :9502, 17 Oct: NIFTY 28 Mar 2019 9,400 PE sorted by Volume put
  // 19 Mar 15:11:34 and 22 Mar 09:41:14 on one page, and the second was marked
  // "overnight" against the first, whatever traded between them.
  const day = (/** @type {string} */ d, /** @type {string} */ t) => /** @type {number} */ (istInstant(d, t)) * 1000;
  /** @param {string} d @param {string} t @param {number} chg */
  const row = (d, t, chg) => ({ rk: rowKey(PE, day(d, t) / 1000), key: PE, ts: day(d, t), day: d, chg });
  const byVolume = [row('2019-03-19', '15:11:34', 0), row('2019-03-22', '09:41:14', -714)];
  const o = { timeNarrows: false, dayRung: false, edgeEarlier: false };
  assert.equal(acrossDay(byVolume, { ...o, contiguous: false }).size, 0, 'a value order: not known, not said');
  assert.deepEqual([...acrossDay(byVolume, { ...o, contiguous: true }).keys()], [byVolume[1].rk], 'the same two rows read in time order are neighbours');
  assert.equal(acrossDay(byVolume, { ...o, edgeEarlier: true, contiguous: false }).size, 0, 'nor is the edge known');
  // The grid says the rows are neighbours only for a read in time order or a
  // read of every kept row, set with the rows it describes.
  const script = gridScript();
  assert.match(script, /pageBefore = next\.local \? null : \{ order, page: asked, rows: next\.rows, generation, total \};\s*rowsInTime = next\.local \|\| wire === 'ts';\s*barState = \{/);
  assert.match(script, /let rowsInTime = \$state\(false\);/);
});

test('the pager arithmetic at every boundary, and every sentence it says, exactly', () => {
  // A total that is not a whole number is no rows; a negative one is none.
  assert.equal(servedPages({ total: Number.NaN, size: 8, wire: 'ts' }).count, 1);
  assert.equal(servedPages({ total: 9.5, size: 8, wire: 'ts' }).count, 1);
  // A local sort (no wire) serves every page, however deep.
  assert.equal(servedPages({ total: 5_000_000, size: 8, wire: null }).why, null);
  // A time-of-day order ignores any report; a report of null is no report.
  const tod = { indexed_rows: 0, scanned_rows: 10 };
  assert.deepEqual(servedPages({ total: 2_000_000, size: 1000, wire: 'tod', report: tod }), servedPages({ total: 2_000_000, size: 1000, wire: 'tod' }));
  assert.equal(servedPages({ total: 2_000_000, size: 1000, wire: 'tod', report: { indexed_rows: 0, scanned_rows: 2_000_000 } }).tail, null);
  assert.equal(servedPages({ total: 2_000_000, size: 1000, wire: 'c', report: null }).tail, null);
  // The read count: exactly MAX_READ_HELD serves all; one more does not; nonsense is not "few".
  const read = (/** @type {number} */ n) => servedPages({ total: 2_000_000, size: 1000, wire: 'c', report: { indexed_rows: 7, scanned_rows: n } });
  assert.equal(read(1_000_000).why, null);
  assert.equal(read(1_000_001).front, 100);
  assert.equal(read(-5).front, 100);
  assert.equal(servedPages({ total: 2_000_000, size: 1000, wire: 'c', report: { indexed_rows: 7 } }).front, 100);
  // A front that reaches the count serves every page.
  assert.deepEqual(servedPages({ total: 100_000, size: 8, wire: 'tod' }), { count: 12_500, front: 12_500, tail: null, why: null });
  // A tail that starts one past the front serves every page; one later does not.
  const noIndex = { indexed_rows: 0, scanned_rows: 2_000_000 };
  assert.equal(servedPages({ total: 200_000, size: 1000, wire: 'c', report: noIndex }).why, null);
  assert.deepEqual(servedPages({ total: 201_000, size: 1000, wire: 'c', report: noIndex }).tail, 102);
  // The sentences, whole.
  assert.equal(servedPages({ total: 153_107, size: 8, wire: 'tod' }).why,
    'the server reads a time-of-day order rather than taking it from a sort index, and serves it from the front only: rows 1 to 1,00,000 ' +
    '(1,00,000 rows deep, bars::MAX_SORT_DEPTH). Pages between are refused sort_too_deep, so they are not offered: ' +
    'page in time order, or narrow the days, the time of day or the series to reach every row.');
  assert.equal(servedPages({ total: 2_000_000, size: 1000, wire: 'v', report: noIndex }).why,
    'the server reads this value order rather than taking it from a sort index, and serves it within 1,00,000 rows of either end: pages 1 to 100 and 1,901 to 2,000 ' +
    '(1,00,000 rows deep, bars::MAX_SORT_DEPTH). Pages between are refused sort_too_deep, so they are not offered: ' +
    'page in time order, or narrow the days, the time of day or the series to reach every row.');
  assert.equal(servedPages({ total: 2_000_000, size: 1000, wire: 'h', report: { indexed_rows: 4, scanned_rows: 2_000_000 } }).why?.includes('this value order rather than taking it from a sort index, and serves it from the front only: rows 1 to 1,00,000 '), true);

  const both = servedPages({ total: 2_000_000, size: 1000, wire: 'v', report: noIndex });
  const front = servedPages({ total: 153_107, size: 8, wire: 'tod' });
  // isServed past the count, even with a tail.
  assert.equal(isServed(both, 2001), false);
  assert.equal(isServed(both, 2000), true);
  // clampServed: nonsense is page 1; a page past the end the last served; a dead page the front.
  assert.equal(clampServed(front, /** @type {any} */ ('x')), 1);
  assert.equal(clampServed(front, 0), 1);
  assert.equal(clampServed(front, 1), 1);
  assert.equal(clampServed(front, 2.9), 2);
  assert.equal(clampServed(front, 12_500), 12_500);
  assert.equal(clampServed(both, 5000), 2000);
  assert.equal(clampServed(both, 1901), 1901);
  // stepServed at both ends, and across the dead stretch both ways.
  assert.equal(stepServed(front, 2, -1), 1);
  assert.equal(stepServed(front, 12_499, 1), 12_500);
  assert.equal(stepServed(both, 2000, 1), null);
  assert.equal(stepServed(both, 1999, 1), 2000);
  assert.equal(stepServed(both, 101, -1), 100, 'a page inside the stretch steps back to the front');

  // THE BUTTON ROW, item by item.
  const row = (/** @type {number} */ cur, /** @type {any} */ s) =>
    pageItems(cur, s).map((i) => (i.dead ? `x${i.p}` : i.gap ? `…${i.p}` : i.p)).join(' ');
  const flat = servedPages({ total: 8_000, size: 8, wire: 'ts' });
  assert.equal(row(1, flat), '1 2 3 4 …1000 1000');
  assert.equal(row(3, flat), '1 2 3 4 …1000 1000');
  assert.equal(row(4, flat), '1 …3 3 4 5 …1000 1000');
  assert.equal(row(997, flat), '1 …996 996 997 998 …1000 1000');
  assert.equal(row(998, flat), '1 …997 997 998 999 1000');
  assert.equal(row(1000, flat), '1 …997 997 998 999 1000');
  assert.equal(row(1, front), '1 2 3 4 …12500 12500 x12501');
  assert.equal(row(1, both), '1 2 3 4 …100 100 x1901 1901 …2000 2000');
  assert.equal(row(1950, both), '1 …100 100 x1901 1901 …1949 1949 1950 1951 …2000 2000');
  assert.deepEqual(pageItems(1, both).slice(0, 2), [
    { k: 'p1', p: 1, gap: false, dead: false },
    { k: 'p2', p: 2, gap: false, dead: false }
  ]);
  assert.deepEqual(pageItems(1, both).filter((i) => i.gap).map((i) => i.k), ['gap100', 'dead1901', 'gap2000']);
  assert.deepEqual(pageItems(1, flat).filter((i) => i.gap).map((i) => i.k), ['gap1000']);
  assert.deepEqual(pageItems(1, servedPages({ total: 1, size: 8, wire: 'ts' })), [{ k: 'p1', p: 1, gap: false, dead: false }]);

  // THE PAGE BOX'S SENTENCES, whole.
  const s = servedPages({ total: 80, size: 8, wire: 'ts' });
  assert.deepEqual(parsePageBox('   ', s), { ok: false, why: 'Type a page number, 1 to 10.' });
  assert.deepEqual(parsePageBox('0', s), { ok: false, why: 'Page "0" is outside 1 to 10: this selection has 10 page(s) at this many rows a page. Type 1 to 10.' });
  assert.deepEqual(parsePageBox('x', s), { ok: false, why: 'Page "x" is not a whole number from 1: this selection has 10 page(s) at this many rows a page. Type 1 to 10.' });
  assert.deepEqual(parsePageBox('500', both), { ok: false, why: `Page 500 is not served: ${both.why} Type 1 to 100 or 1,901 to 2,000.` });

  // THE READ'S MEMORY: an unchanged order keeps its page; a fresh memory has asked nothing.
  assert.equal(pageToRead({ page: 7, order: 'a', lastOrder: 'a', held: false }), 7);
  assert.equal(pageToRead({ page: 7, order: 'b', lastOrder: 'a', held: false }), 1);
  assert.equal(pageToRead({ page: 7, order: 'b', lastOrder: 'a', held: true }), 7);
  assert.equal(askOnce().asked, '');
  assert.equal(askOnce().fresh(''), false, 'nothing asked is the empty question');

  // THE MOVE BETWEEN TWO PAGES, said whole.
  const key = (/** @type {any} */ b) => String(b.t);
  assert.deepEqual(joinPage({ rows: [{ t: 3 }, { t: 2 }], generation: 'a', total: 10 }, { rows: [{ t: 2 }, { t: 1 }], generation: 'b', total: 11 }, key), {
    rows: [{ t: 1 }],
    dropped: 1,
    moved: 'The store grew by 1 bar(s) between the page before and this one (10 then 11), so the offsets moved under the pager; ' +
      '1 row(s) already shown on the page before are dropped here rather than shown twice. Read again for pages counted against the store as it is now.'
  });
});

test('the sort offer, the advice and the failure words at every boundary, said whole', () => {
  assert.equal(sortRefusal('oi', [SPOT, CE]), null, 'open interest with a derivative in the selection');
  assert.equal(localSortAdvice({ label: 'Change %', total: 30_000, budget: 20_000, tfSecs: 60, series: 1, stored: ['Date', 'Close'] }),
    'Change % cannot be ordered here: this query keeps 30,000 bar(s), and the browser orders at most 20,000 of a column the server cannot sort. ' +
    'The grid keeps the order it had. To order by Change %: narrow to one day (a day holds at most 1,440 bar(s) of this series at this rung); ' +
    'or sort by a stored column (Date, Close), which the server orders over any number.');
  // A day that holds exactly the budget is a day that fits.
  assert.match(localSortAdvice({ label: 'x', total: 9e9, budget: 1440, tfSecs: 60, series: 1, stored: [] }), /narrow to one day \(a day holds at most 1,440/);
  // One minute that fits exactly is still a minute.
  assert.match(localSortAdvice({ label: 'x', total: 9e9, budget: 60, tfSecs: 1, series: 1, stored: [] }), /a time of day of at most 1 minute\(s\)/);
  assert.deepEqual(readFailure({ made: 'read', status: 0 }), {
    kind: 'failed',
    title: 'This page of bars could not be read',
    says: 'No answer arrived, or the answer did not check. Nothing is drawn rather than a plausible blank; asking again may answer.',
    retry: true
  });
  assert.equal(readFailure({ made: 'read', status: 200 }).says, readFailure({ made: 'read', status: 0 }).says);
  assert.equal(readFailure({ made: 'read', status: 503 }).says, 'The server answered HTTP 503 and no page. Nothing is drawn rather than a plausible blank; asking again may answer.');
  assert.deepEqual(readFailure({ made: 'read', status: 404 }), {
    kind: 'server',
    title: 'The server refused this page',
    says: "The sentence above is the server's own (HTTP 404). Asking again asks the same question, so it is not offered: change what it names.",
    retry: false
  });
  assert.deepEqual(readFailure({ made: 'page', status: 0 }), {
    kind: 'page',
    title: 'This selection is not read as one page',
    says: 'This page refused it before asking the server, so nothing was read and no store was consulted. Change the selection as the sentence above says.',
    retry: false
  });
});
