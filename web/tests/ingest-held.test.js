// R4-01 (V1): what the store holds for one /ingest row. Every fixture is
// invented — shaped like `/instruments.json?feed=gdfl` and
// `/store.json?feed=gdfl&encoding=census-tuples-v1` as the server sends them
// (crates/api/src/instruments_route.rs, crates/api/src/store_route.rs), with
// bar counts nobody could mistake for real ones. No vendor row is copied.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  censusKey, censusReadiness, censusStamp, createHeldJoin, foldDerivatives, heldMonth, rungStored, stopFold
} from '../src/lib/ingest-held.js';
import { decodeCensus } from '../src/lib/store-census-wire.js';
import { emptySeen, monthVerdict, seriesVerdict, settledMonth } from '../src/lib/ingest-months.js';
import { parseKey, segmentOf } from '../src/lib/instrument.js';

const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
const storeModule = readFileSync(new URL('../src/lib/store.svelte.js', import.meta.url), 'utf8');

/** 2018-09 .. 2026-09, newest first, as the census lists them: 97 months. */
const MONTHS = (() => {
  const out = [];
  for (let y = 2026, m = 9; y > 2018 || (y === 2018 && m >= 9); m === 1 ? ((m = 12), (y -= 1)) : (m -= 1)) {
    out.push(`${y}-${String(m).padStart(2, '0')}`);
  }
  return out;
})();

/** Microseconds of 09:15 IST on the first and 15:29 IST on the 20th of `ym`. */
const span = (/** @type {string} */ ym) => {
  const [y, m] = ym.split('-').map(Number);
  return [Date.UTC(y, m - 1, 1, 3, 45) * 1000, Date.UTC(y, m - 1, 20, 9, 59) * 1000];
};

/** The RELIANCE row `/instruments.json?feed=gdfl` sends: a census-derived CASH listing. */
const RELIANCE = {
  symbol: 'RELIANCE', key: 'NSE-RELIANCE', kind: 'Equity', exchange: 'NSE', segment: 'CASH',
  universe: 'fno+ntm', universes: ['fno', 'ntm'], bars: 9_700, bars_rung: '1s', rows_by_rung: { '1s': 9_700 },
  spot: true, options: true, master: null
};

/**
 * A census-tuples-v1 body: the NIFTY index, RELIANCE's 97 one-second months
 * (100 bars each, invented), and two RELIANCE option contracts in two months.
 */
function censusBody() {
  const instruments = ['NSE-INDEX-NIFTY', 'NSE-CASH-RELIANCE', 'NSE-FNO-RELIANCE-2026-09-29-102000-CE', 'NSE-FNO-RELIANCE-2026-09-29-104000-PE'];
  /** @type {any[]} */
  const rows = [];
  MONTHS.forEach((ym, mi) => {
    const [first, last] = span(ym);
    rows.push([0, mi, 0, 7, first, last, 0, null, 0, null]);
    rows.push([1, mi, 0, 100, first, last, 0, null, 0, null]);
    if (ym === '2026-09' || ym === '2026-08') {
      rows.push([2, mi, 0, 11, first, last, 0, null, 0, null]);
      rows.push([3, mi, 0, 13, first, last, 0, null, 0, null]);
    }
  });
  return { schema: 1, encoding: 'census-tuples-v1', feed: 'gdfl', instruments, months: MONTHS, timeframes: ['1s'], reasons: [], rows };
}

/** The shared index the page probes (`store.byCell`), spelled as store.svelte.js spells it. */
async function byCellOf() {
  const rows = await decodeCensus(censusBody(), 'gdfl', new AbortController().signal, async () => {});
  const byCell = new Map();
  for (const r of rows) byCell.set(`${r.instrument}|${r.timeframe}|${r.month}`, r.rows);
  return { rows, byCell };
}

test('the probe key here is the one store.svelte.js builds byCell with', () => {
  assert.match(storeModule, /export const cellKey = \(instrument, timeframe, month\) => `\$\{instrument\}\|\$\{timeframe\}\|\$\{month\}`;/);
  assert.equal(censusKey(RELIANCE), 'NSE-CASH-RELIANCE', 'the census spells EXCHANGE-SEGMENT-SYMBOL');
});

test('gdfl_stock_rows_count_their_1s_months', async () => {
  const { byCell } = await byCellOf();
  const probe = (/** @type {string} */ i, /** @type {string} */ t, /** @type {string} */ y) => byCell.get(`${i}|${t}|${y}`);
  const census = censusReadiness({ feed: 'gdfl', storeFeed: 'gdfl', state: 'ready' });
  assert.equal(census.ready, true);
  const join = createHeldJoin();
  const seen = emptySeen();
  let got = 0;
  let held = 0;
  let unproved = 0;
  for (const ym of MONTHS) {
    const h = heldMonth(join, probe, census, RELIANCE, 'spot', '1s', ym);
    assert.equal(h.loading, false);
    // ONE SECOND HAS NO PER-SESSION COUNT: `r.per === null` on the page.
    const k = monthVerdict({ beyond: false, bars: h.bars, expect: null, sessions: 20, refused: false, loading: h.loading, countOnly: true });
    seen[k] += 1;
    got += h.bars ?? 0;
    if ((h.bars ?? 0) > 0) held += 1;
    if (!settledMonth(k) && k !== 'loading') unproved += 1;
  }
  assert.equal(held, 97, 'every month the census holds is counted');
  assert.equal(got, 9_700, 'with its real count');
  assert.equal(unproved, 0, 'a held month at a rung with no yardstick is not "unproved"');
  assert.equal(seriesVerdict({ seen, months: 97, got, inFlight: false, halted: false }), 'held');
  // Before R4-01 the same month read `unknown`, and the row "97/97 unproved".
  assert.equal(monthVerdict({ beyond: false, bars: 100, expect: null, refused: false }), 'unknown');
});

test('the census not loaded reads "census loading", never 0 and never "never imported"', () => {
  const join = createHeldJoin();
  const probe = () => undefined;
  for (const state of [
    { feed: 'gdfl', storeFeed: null, state: 'reading' },
    { feed: 'gdfl', storeFeed: null, state: 'none' },
    // A FEED SWITCH MID-LOAD: the value on hand is the previous feed's.
    { feed: 'gdfl', storeFeed: 'zerodha', state: 'reading' }
  ]) {
    const census = censusReadiness(state);
    assert.equal(census.ready, false);
    assert.equal(census.word, 'loading');
    assert.match(census.why, /No month is judged until it lands/);
    const h = heldMonth(join, probe, census, RELIANCE, 'spot', '1s', '2026-09');
    assert.deepEqual(h, { bars: null, loading: true, stale: false });
    assert.equal(monthVerdict({ beyond: false, bars: h.bars, expect: null, refused: false, loading: h.loading, countOnly: true }), 'loading');
  }
  const failed = censusReadiness({ feed: 'gdfl', storeFeed: null, state: 'error', error: 'No complete answer from /store.json within 30 s' });
  assert.equal(failed.word, 'failed');
  assert.match(failed.why, /could not be read: No complete answer from \/store\.json within 30 s/);
  assert.equal(censusReadiness({ feed: null, storeFeed: 'gdfl' }).word, 'none');
  // A REFRESH OF THE SAME FEED KEEPS ITS OWN PREVIOUS ANSWER, labelled ready.
  assert.equal(censusReadiness({ feed: 'gdfl', storeFeed: 'gdfl', state: 'reading' }).ready, true);
});

test('a_1s_rung_of_an_archive_feed_is_stored', () => {
  const gdfl = { wire: 'gdfl', display: 'Global Datafeeds', rungs: ['1s'], derived_rungs: [] };
  const zerodha = { wire: 'zerodha', display: 'Zerodha', rungs: ['1min', '1day'], derived_rungs: ['2min', '3min', '5min', '10min', '15min', '30min', '60min'] };
  assert.equal(rungStored(gdfl, '1s').stored, true, 'the archive files its seconds (D-2802)');
  assert.equal(rungStored(gdfl, '1min').stored, false);
  assert.match(rungStored(gdfl, '1min').why, /Global Datafeeds files no 1min bars: \/feeds\.json lists what it stores as 1s/);
  assert.equal(rungStored(zerodha, '1s').stored, false);
  assert.equal(rungStored(zerodha, '1min').stored, true);
  assert.equal(rungStored(zerodha, '5min').stored, true, 'a derived rung is filed');
  assert.equal(rungStored({ wire: 'old' }, '1s').stored, null, 'no rungs field is a missing field, not a no');
  assert.equal(rungStored(null, '1s').stored, null);
  // The page reads it per feed; the constant ladder carries no `stored`.
  assert.doesNotMatch(page, /dir: '1s', label: 'Ticks', phrase: 'one second', stored:/);
  assert.match(page, /const s = rungStored\(\/\*\* @type \{any\} \*\/ \(active\), r\.dir\);/);
  assert.doesNotMatch(page, /store::path::Timeframe::KNOWN ships 1min, 3min, 5min, 15min, 30min, 60min and 1day, and nothing below a minute/,
    'the sentence that said nothing below a minute is stored is gone');
});

test('the_held_join_is_one_pass_per_census_stamp', async () => {
  const { rows } = await byCellOf();
  const join = createHeldJoin();
  let parsed = 0;
  const how = {
    parse: (/** @type {string} */ i) => { parsed += 1; return parseKey(i); },
    segmentOf,
    slices: async (/** @type {number} */ n, /** @type {(a: number, b: number) => void} */ work) => {
      // Two slices, as the census fold's own `inSlices` would cut them.
      const half = Math.floor(n / 2);
      work(0, half);
      await Promise.resolve();
      work(half, n);
    }
  };
  const stamp = censusStamp({ feed: 'gdfl', generation: 7, at: 1_000 });
  const first = foldDerivatives(join, stamp, rows, how);
  // A second ask while the first walk runs does not walk again.
  assert.equal(await foldDerivatives(join, stamp, rows, how), false);
  assert.equal(await first, true);
  assert.equal(join.passes, 1);
  assert.equal(parsed, rows.length, 'O(cells), once');
  // A tick change, a re-render: the same stamp, no walk.
  assert.equal(await foldDerivatives(join, stamp, rows, how), false);
  assert.equal(join.passes, 1);
  const census = censusReadiness({ feed: 'gdfl', storeFeed: 'gdfl' });
  const probe = () => undefined;
  const opt = heldMonth(join, probe, census, RELIANCE, 'options', '1s', '2026-09');
  assert.deepEqual(opt, { bars: 24, loading: false, stale: false }, 'every contract of the underlying, summed');
  assert.deepEqual(heldMonth(join, probe, census, RELIANCE, 'options', '1s', '2026-07'), { bars: null, loading: false, stale: false });
  // A new reading is a new stamp, and only then a second walk.
  await foldDerivatives(join, censusStamp({ feed: 'gdfl', generation: 8, at: 2_000 }), rows, how);
  assert.equal(join.passes, 2);
  assert.equal(parsed, rows.length * 2);
});

test('a contract fold that fails or is replaced is never published as the store', async () => {
  const join = createHeldJoin();
  const cells = [{ instrument: 'NSE-FNO-RELIANCE-2026-09-29-102000-CE', timeframe: '1s', month: '2026-09', rows: 5 }];
  const failing = { parse: parseKey, segmentOf, slices: async () => { throw new Error('This operation was aborted'); } };
  assert.equal(await foldDerivatives(join, 'gdfl#1#1', cells, failing), true);
  assert.equal(join.ready, false);
  assert.equal(join.error, 'This operation was aborted');
  const census = censusReadiness({ feed: 'gdfl', storeFeed: 'gdfl' });
  assert.deepEqual(heldMonth(join, () => undefined, census, RELIANCE, 'options', '1s', '2026-09'), { bars: null, loading: true, stale: false },
    'contract months stay unread, never "never imported"');
  // The same stamp is walked again once asked, and then publishes.
  assert.equal(await foldDerivatives(join, 'gdfl#1#1', cells, { parse: parseKey, segmentOf }), true);
  assert.equal(join.ready, true);
  assert.equal(join.error, null);
  // A newer stamp that starts while an older walk is between slices wins.
  const slow = createHeldJoin();
  /** @type {() => void} */
  let release = () => {};
  const gate = new Promise((r) => { release = () => r(undefined); });
  const older = foldDerivatives(slow, 'gdfl#1#1', cells, { parse: parseKey, segmentOf, slices: async (n, work) => { await gate; work(0, n); } });
  await foldDerivatives(slow, 'gdfl#2#2', [], { parse: parseKey, segmentOf });
  release();
  await older;
  assert.equal(slow.stamp, 'gdfl#2#2');
  assert.equal(slow.by.size, 0, 'the older walk did not overwrite the newer answer');
});

test('a gdfl row with options but no spot reads its option months and no spot', async () => {
  const { byCell } = await byCellOf();
  const probe = (/** @type {string} */ i, /** @type {string} */ t, /** @type {string} */ y) => byCell.get(`${i}|${t}|${y}`);
  // instruments_route.rs files an underlying with options and no spot under FNO.
  const noSpot = { ...RELIANCE, segment: 'FNO', spot: false };
  const join = createHeldJoin();
  await foldDerivatives(join, 's', (await byCellOf()).rows, { parse: parseKey, segmentOf });
  const census = censusReadiness({ feed: 'gdfl', storeFeed: 'gdfl' });
  assert.deepEqual(heldMonth(join, probe, census, noSpot, 'spot', '1s', '2026-09'), { bars: null, loading: false, stale: false });
  assert.deepEqual(heldMonth(join, probe, census, noSpot, 'options', '1s', '2026-09'), { bars: 24, loading: false, stale: false });
});

test('the page reads the census state before it calls a month never imported', () => {
  assert.match(page, /const censusNow = \$derived\(\s+censusReadiness\(\{ feed: feeds\.active, storeFeed: store\.feed, state: store\.state, error: store\.error \}\)\s+\);/);
  assert.match(page, /return heldMonth\(heldJoin, \(i, t, y\) => store\.byCell\.get\(cellKey\(i, t, y\)\), censusNow, m, seg, dir, ym\);/);
  assert.match(page, /\{#if r\.state === 'loading'\}\s+<span class="dim">\{r\.loadingWord\}<\/span>/);
  assert.match(page, /loadingWord: censusNow\.word === 'failed' \|\| \(s\.key !== 'spot' && heldJoin\.error\) \? 'census not read' : 'census loading',/);
  assert.match(page, /\{n\(r\.held\)\}\/\{n\(r\.months\.length\)\} held/);
  // An underlying with options and no spot: its spot row says "spot not stored".
  assert.match(page, /noSpot: s\.key === 'spot' && \/\*\* @type \{any\} \*\/ \(m\)\.spot === false,/);
  assert.match(page, /\{:else if r\.noSpot\}\s+<span class="warn"[^>]*>spot not stored<\/span>/);
  // The contract fold is once per census stamp, in slices, and never a $derived over the ticked set.
  assert.doesNotMatch(page, /const derivativeBars = \$derived\.by/);
  assert.match(page, /void foldDerivatives\(heldJoin, stamp, store\.readable, \{/);
  assert.match(page, /onDestroy\(\(\) => stopFold\(heldJoin\)\);/);
  // The effect never aborts the walk it started: a re-run would kill it.
  assert.doesNotMatch(page, /void foldDerivatives\([\s\S]{0,400}return \(\) => abort\.abort\(\);/);
});

test('a re-render mid-walk never kills the walk; a newer stamp or leaving the page does', async () => {
  const cells = [{ instrument: 'NSE-FNO-RELIANCE-2026-09-29-102000-CE', timeframe: '1s', month: '2026-09', rows: 5 }];
  /** @type {() => void} */
  let release = () => {};
  const how = {
    parse: parseKey,
    segmentOf,
    // Like `inSlices`: the signal is checked before the slice runs.
    slices: async (/** @type {number} */ n, /** @type {any} */ work, /** @type {AbortSignal} */ signal) => {
      await new Promise((r) => { release = () => r(undefined); });
      signal.throwIfAborted();
      work(0, n);
    }
  };
  const join = createHeldJoin();
  const walk = foldDerivatives(join, 'gdfl#1#1', cells, how);
  // The page's effect re-runs (a refresh flag, a tick): same stamp, no new walk, no abort.
  assert.equal(await foldDerivatives(join, 'gdfl#1#1', cells, how), false);
  release();
  assert.equal(await walk, true);
  assert.equal(join.ready, true, 'the walk finished and published');
  assert.equal(join.by.get('RELIANCE|options|1s|2026-09'), 5);
  // Leaving the page stops the walk in flight; a later ask of the stamp walks again.
  const next = foldDerivatives(join, 'gdfl#2#2', cells, how);
  stopFold(join);
  release();
  await next;
  assert.equal(join.ready, false);
  assert.match(join.error ?? '', /abort/i);
  const again = foldDerivatives(join, 'gdfl#2#2', cells, how);
  release();
  assert.equal(await again, true);
  assert.equal(join.ready, true);
  assert.equal(join.passes, 3);
});

// R4-01 repair round 1: the contract fold is keyed on the census's OWN
// generation, so a re-read of an unchanged census walks nothing, and while a
// newer census of the same feed is walked the previous sums stand, labelled.
test('a_reread_of_an_unchanged_census_walks_nothing_and_the_previous_sums_stand_labelled', async () => {
  const { rows } = await byCellOf();
  const census = censusReadiness({ feed: 'gdfl', storeFeed: 'gdfl' });
  const probe = () => undefined;
  // The same server census read twice — another client read, another time —
  // is ONE stamp. Before this, every re-read was a new stamp and a new walk.
  const a = censusStamp({ feed: 'gdfl', generation: 7, at: 1_000, server: 4_242, epoch: 'e1' });
  const reread = censusStamp({ feed: 'gdfl', generation: 8, at: 9_000, server: 4_242, epoch: 'e1' });
  assert.equal(a, reread, 'an unchanged census is one stamp, however often it is read');
  assert.notEqual(a, censusStamp({ feed: 'gdfl', generation: 8, at: 9_000, server: 4_243, epoch: 'e1' }), 'a new census generation is a new stamp');
  assert.notEqual(a, censusStamp({ feed: 'gdfl', generation: 8, at: 9_000, server: 4_242, epoch: 'e2' }), 'a new epoch (a rebuilt census) is a new stamp');
  assert.notEqual(a, censusStamp({ feed: 'zerodha', generation: 7, at: 1_000, server: 4_242, epoch: 'e1' }), 'another feed is another stamp');
  // No server generation: the read itself is the stamp, as before — walked again, never assumed unchanged.
  assert.notEqual(censusStamp({ feed: 'gdfl', generation: 7, at: 1_000, server: null }), censusStamp({ feed: 'gdfl', generation: 8, at: 9_000, server: null }));
  assert.equal(censusStamp({ feed: 'gdfl', generation: 7, at: 1_000, server: -1 }), 'gdfl#7#1000', 'a malformed server generation is not trusted');
  const join = createHeldJoin();
  let parsed = 0;
  const how = { parse: (/** @type {string} */ i) => { parsed += 1; return parseKey(i); }, segmentOf };
  assert.equal(await foldDerivatives(join, a, rows, how), true);
  assert.equal(await foldDerivatives(join, reread, rows, how), false, 'the re-read walks nothing');
  assert.equal(parsed, rows.length);
  assert.equal(join.published, a);
  // A newer census of the SAME feed is walked; until it lands the previous
  // sums are answered, flagged stale — never `loading`, never passed as new.
  /** @type {() => void} */
  let release = () => {};
  const gate = new Promise((r) => { release = () => r(undefined); });
  const newer = censusStamp({ feed: 'gdfl', generation: 9, at: 10_000, server: 4_300, epoch: 'e1' });
  const walking = foldDerivatives(join, newer, rows, { parse: parseKey, segmentOf, slices: async (n, work) => { await gate; work(0, n); } });
  assert.deepEqual(heldMonth(join, probe, census, RELIANCE, 'options', '1s', '2026-09'), { bars: 24, loading: false, stale: true });
  release();
  await walking;
  assert.deepEqual(heldMonth(join, probe, census, RELIANCE, 'options', '1s', '2026-09'), { bars: 24, loading: false, stale: false });
  // ANOTHER FEED's walk never shows this feed's sums.
  /** @type {() => void} */
  let release2 = () => {};
  const gate2 = new Promise((r) => { release2 = () => r(undefined); });
  const other = foldDerivatives(join, censusStamp({ feed: 'zerodha', server: 1, epoch: 'z' }), [], { parse: parseKey, segmentOf, slices: async (n, work) => { await gate2; work(0, n); } });
  assert.deepEqual(heldMonth(join, probe, census, RELIANCE, 'options', '1s', '2026-09'), { bars: null, loading: true, stale: false });
  release2();
  await other;
  // A walk that FAILED never leaves the previous sums standing either.
  const failed = createHeldJoin();
  await foldDerivatives(failed, a, rows, how);
  await foldDerivatives(failed, newer, rows, { parse: parseKey, segmentOf, slices: async () => { throw new Error('PROBE: walk failed'); } });
  assert.equal(failed.error, 'PROBE: walk failed');
  assert.deepEqual(heldMonth(failed, probe, census, RELIANCE, 'options', '1s', '2026-09'), { bars: null, loading: true, stale: false });
  // The page passes the census's own generation and epoch, and labels stale rows.
  assert.match(page, /censusStamp\(\{ feed: store\.feed, generation: store\.generation, at: store\.at, server: store\.lag\?\.generation \?\? null, epoch: store\.lag\?\.epoch \?\? null \}\)/);
  assert.match(page, /if \(held\.stale\) staleCount \+= 1;/);
  assert.match(page, /\{#if r\.stale > 0\}<span class="dim"[^>]*> · previous census, re-counting<\/span>\{\/if\}/);
});
