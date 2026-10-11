/**
 * THE STORE CENSUS — ONE FOLD, ONE CLOCK, ONE ARRAY.
 *
 * # What this replaces
 *
 * `/store.json` was read SIX times and folded FIVE different ways on SIX
 * independent clocks:
 *
 *   lib/feeds.svelte.js        a sum, to pick a default            once
 *   routes/+page.svelte        Map<instrument,[{month,rows,tf}]>   per feed
 *   routes/db/+page.svelte     the raw array                       per feed
 *   routes/db/+page.svelte     (a second, independent read)
 *   routes/ingest/+page.svelte Map<"inst|tf|month", rows>          per feed
 *   routes/ingest/+page.svelte {units,rows,byInstrument,held}      EVERY 5 s
 *   routes/autopilot/+page     per-month {cells,bars}              every 30 s
 *
 * Nothing reconciled them. During a pull `/db` and `/ingest` showed DIFFERENT
 * totals for the same store and both were "correct" as of their own snapshot;
 * `/ingest` alone held two independently-clocked copies of the same census and
 * could disagree with itself. That is not a slow fold — a faster fold would
 * still be six answers. It is one question asked six times, and the fix is to
 * ask it once.
 *
 * # The shape of the answer
 *
 * ONE fetch per (feed, generation). One stamp. Every derived shape any page
 * needs is built from THE SAME ARRAY in THE SAME PASS, so no page ever scans
 * and no two pages can disagree:
 *
 *   `rows`         the raw array, exactly what the wire sent
 *   `readable`     the rows whose fields parse, normalised
 *   `bad`          the rows whose fields do not, each carrying the reason
 *   `byInstrument` instrument      -> readable cell[], ascending (month, rung)
 *   `byCell`       "inst|tf|month" -> bar count                        — O(1)
 *   `byMonth`      "YYYY-MM"       -> { cells, bars, list }            — O(1)
 *
 * Building the indexes is O(n) ONCE per read. That is inherent — every row has
 * to be seen to be indexed — and it is now paid ONCE for the whole product
 * instead of five times per page-load and again every five seconds. After the
 * pass, every lookup any page makes is one Map probe.
 *
 * # The stamp, and why it carries the feed
 *
 * `$lib/index.svelte.js` learned this the expensive way: `catalogue.feed` was
 * read by two pages as the gate on whether a count could be printed, and it was
 * never DECLARED and never WRITTEN. `undefined === 'dhan'` is false forever, so
 * every instrument picker rendered "0 shown of 0" behind a refusal no operator
 * action could satisfy. A shared census without a feed stamp is that bug again,
 * one page wider: a fold of one broker's store under another broker's heading,
 * and every number in it really counted.
 *
 * So a read carries THREE things and never fewer: its stamp (`at`), its error,
 * and the feed it ANSWERED FOR (`feed`). A page compares `store.feed` against
 * the feed it is asking about before it believes a single number.
 *
 * # CLAUDE.md §4 — a failed read never leaves the previous value looking current
 *
 * On failure the stamp is CLEARED, the feed stamp is CLEARED, every index is
 * emptied and `error` names the reason. `lastOk` keeps the previous SUCCESSFUL
 * read's stamp under a name that cannot be mistaken for the current one — "last
 * good: 15:29" is an honest sentence; "as of 15:29" over a failed read is the
 * fallback that hides a failure, and it is worse than a blank because the
 * minute is real and only the claim is false.
 */

// THE WINDOW FOLD'S ARITHMETIC, in a module with no runes in it so a test can
// drive it. See `$lib/fold.js`.
import { foldWindow, foldKey } from '$lib/fold.js';
// A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
// ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
// threaded through every call site.
import { createCensusLoader, censusFailure, censusLag, createFacetsLoader, readOnPage, STORE_CENSUS_MS } from '$lib/store-census.js';
import { decodeCensus } from '$lib/store-census-wire.js';
import { createCensusWorker, inSlices } from '$lib/census-worker.js';
import { ask } from '$lib/ask.js';
import { readFeedHeader } from '$lib/feed-summary.js';
import { offeredFeeds } from '$lib/feed-row.js';
import { pooled } from '$lib/pooled.js';

/* ======================================================================
   THE RUNGS ON DISK — the store's own list, not a second copy of it.
   ----------------------------------------------------------------------
   `store::path::Timeframe::KNOWN` files under these seven directory names and
   `/store.json` puts the row's OWN rung on the wire. A rung this table cannot
   map is NEVER guessed at: ordering falls back to the wire's order and the
   page that cares reports the rung as unmapped, because guessing how many
   seconds a bar covers draws a convincing chart of the wrong buckets.
   ====================================================================== */
export const RUNG_SECONDS = new Map([
  // TEN RUNGS, WHICH IS WHAT `store::path::Timeframe::KNOWN` DECLARES.
  //
  // This carried seven. `1s`, `2min` and `10min` were absent, and everything
  // that reads this table treats an absent rung as unmappable:
  //
  //   * `/` (Markets) filters `storedRungs` on `rungSeconds(t) !== null`, so
  //     the "Rungs on disk" line read `1min + 3min + 5min + 15min + 30min +
  //     60min + 1day` for a store holding nine rungs — measured on the running
  //     server, 2min (1,880 bars) and 10min (380) simply absent from the list.
  //   * the same filter feeds `base`, the rung the chart is drawn at, so
  //     neither could ever be charted.
  //   * `byCell`'s sort at line 231 falls back to `?? 0`, which sorts an
  //     unmapped rung to the FRONT of the ladder rather than into its place.
  //
  // The seconds are `Timeframe::SECOND_1.secs` … `DAY_1.secs`, read from
  // crates/store/src/path.rs, not arithmetic done here.
  ['1s', 1],
  ['1min', 60],
  ['2min', 120],
  ['3min', 180],
  ['5min', 300],
  ['10min', 600],
  ['15min', 900],
  ['30min', 1800],
  ['60min', 3600],
  ['1day', 86400]
]);

/**
 * Seconds one bar of this rung covers, or `null` when this build cannot map it.
 *
 * @param {string} t the rung EXACTLY as the wire spells it — `1min`, `1day`.
 *        The table above is keyed on those spellings and on nothing else, so a
 *        rung that arrives spelled any other way is unmappable, which is the
 *        answer this returns rather than a guess at the seconds.
 */
export const rungSeconds = (t) => RUNG_SECONDS.get(t) ?? null;

// THE FAULT DETECTOR AND ITS REGEX LIVE IN `$lib/rows.js` so a test can drive
// them. This module imports `$lib/ask.js`, an alias node cannot resolve, so the
// one check standing between `/store.json` and every number two pages print was
// unreachable from `node --test`. Re-exported here because `routes/+page.svelte`
// imports `MONTH_KEY` from this module and has no reason to care where it moved.
import { MONTH_KEY, rowFault } from '$lib/rows.js';
export { MONTH_KEY };

/**
 * Three-way compare, over whatever `sort` is ordering on THIS line.
 *
 * `@template` rather than two overloads because the ladder sort below calls it
 * twice on one line with two different types — first on the month STRING, then
 * on the rung's SECONDS, a number — and a signature naming only one of them
 * would report the other as an error at a call site that is correct.
 *
 * @template {string | number} T
 * @param {T} a
 * @param {T} b
 */
const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

/**
 * The key `byCell` is probed with. ONE spelling, so two pages cannot differ.
 *
 * @param {string} instrument the census key, as the row carries it.
 * @param {string} timeframe the rung, as the row carries it.
 * @param {string} month `YYYY-MM`.
 */
export const cellKey = (instrument, timeframe, month) => `${instrument}|${timeframe}|${month}`;


/**
 * ONE INSTRUMENT-MONTH ROW, AS `/store.json` SENDS IT.
 *
 * Typed because the arrays below infer `never[]` without it, and every callback
 * that walks one — `.map((r) => …)`, `.filter((r) => …)` — is then reported as
 * `Parameter 'r' implicitly has an 'any' type` at the CALL SITE. 148 of those
 * across /db and /ingest, all of them this one absence, none of them fixable
 * where they are reported.
 *
 * Read off a live response rather than guessed. `crates/api` writes it; the
 * timestamps are MICROSECONDS, which is why `$lib/dates.js` refuses to sniff
 * the unit.
 *
 * @typedef {{
 *   instrument: string,
 *   month: string,
 *   timeframe: string,
 *   rows: number,
 *   first_ts: number,
 *   last_ts: number,
 *   chg_bps: number | null,
 *   chg_why: string | null,
 *   prev_chg_bps: number | null,
 *   prev_chg_why: string | null
 * }} StoreRow
 */

/**
 * ONE CENSUS ROW AS IT ARRIVES — before `rowFault` has looked at it.
 *
 * The same four fields `rowFault` tests, each of them merely POSSIBLE. It is
 * the honest description of a row on entry to that function: the endpoint says
 * it sends `StoreRow`, and this type says only what a reader may assume before
 * checking, which is very nearly nothing. The other six fields of `StoreRow`
 * are absent here because nothing on this path reads them before the fault
 * check has passed.
 *
 * @typedef {{ instrument?: string, month?: string, timeframe?: string, rows?: number }} WireRow
 */

/**
 * ONE READABLE INSTRUMENT-MONTH, NORMALISED — what `fold` keeps, not what the
 * wire sent.
 *
 * It is deliberately NOT `StoreRow`: the four census fields are carried
 * through, the timestamp pair is renamed and narrowed to `first`/`last` with
 * `null` for "the entry did not carry one", and the change-in-basis-points
 * fields are dropped because nothing that reads an index needs them. Three
 * views — `readable`, `byInstrument` and `byMonth.list` — hold these objects BY
 * REFERENCE, so this is one shape stored once, not three copies of a row.
 *
 * `first` and `last` are MICROSECONDS, the unit `crates/api` writes and the
 * reason `$lib/dates.js` refuses to sniff a unit.
 *
 * @typedef {{
 *   instrument: string,
 *   month: string,
 *   timeframe: string,
 *   rows: number,
 *   first: number | null,
 *   last: number | null
 * }} StoreCell
 */

/**
 * ONE ROW THAT WOULD NOT PARSE, AND THE REASON IN WORDS.
 *
 * The three identity fields are `unknown` and that is the whole point: they are
 * echoed off a row a fault has just DISPROVED, so the declaration that made
 * them strings is exactly the thing that turned out not to hold. `why` is the
 * only field this side authored and the only one a reader can rely on — which
 * is why `fold` re-tests `typeof it.instrument === 'string'` before using one
 * as a Map key, and why the pages print `why` and never the rest.
 *
 * @typedef {{ instrument: unknown, month: unknown, timeframe: unknown, why: string }} BadRow
 */

/**
 * ONE MONTH'S FOLD — the two totals plus the cells they were counted from.
 *
 * `cells` and `bars` are held rather than re-derived from `list` so that a page
 * asking "how much is in 2026-08" is one Map probe and two field reads, never a
 * pass over the month.
 *
 * @typedef {{ cells: number, bars: number, list: StoreCell[] }} MonthFold
 */

/**
 * THE WHOLE ANSWER, FOLDED — every shape any page asks for, from one pass.
 *
 * This is the shape `empty()` returns and `fold()` fills, and it is the reason
 * this typedef is written out rather than inferred: `empty()` is what the
 * `$state` below is seeded and re-seeded with, and an inferred `new Map()` is
 * `Map<any, any>`, which teaches every reader of every index nothing at all.
 *
 * `rows` is the array EXACTLY as the wire sent it, faults and all — it is typed
 * `StoreRow[]` because that is what the endpoint declares, and `bad` is the
 * count of how often that declaration did not hold.
 *
 * `byCell` maps to a BAR COUNT and not to a cell. It is the one index whose
 * only question is "how many bars are in this instrument-month at this rung",
 * and `/ingest` probes it per rendered row.
 *
 * @typedef {{
 *   rows: StoreRow[],
 *   readable: StoreCell[],
 *   bad: BadRow[],
 *   byInstrument: Map<string, StoreCell[]>,
 *   badByInstrument: Map<string, BadRow[]>,
 *   byCell: Map<string, number>,
 *   byMonth: Map<string, MonthFold>,
 *   cells: number,
 *   bars: number
 * }} StoreCensus
 */

/** @returns {StoreCensus} */
const empty = () => ({
  rows: [],
  readable: [],
  bad: [],
  byInstrument: new Map(),
  badByInstrument: new Map(),
  byCell: new Map(),
  byMonth: new Map(),
  cells: 0,
  bars: 0
});

/**
 * THE ONE READING. Every page reads this object and no page fetches.
 *
 *   state       'none' | 'reading' | 'ready' | 'error'
 *   feed        the feed the CURRENT value answered for — `null` unless ready
 *   at          when this browser OBSERVED that answer — `null` unless ready
 *   error       the reason the last read failed, in the words it failed with
 *   generation  bumped by `refreshStore`; the read key, and the shared clock
 *   reads       how many reads have SUCCEEDED — the memo key for window folds
 *   lastOk      { at, feed } of the last successful read. Never "current".
 *   lag         every census header of the CURRENT value's read (`censusLag`):
 *               its state word, the census's own generation and its parts',
 *               capacity, headroom, the import days it refused — `null`
 *               unless ready (WP-56, D-3327; DB-G03, DB-G11, DB-G60, DB-G78)
 *   worker      `null` when the census worker decoded the current value; the
 *               reason in words when this page decoded it on its own thread
 *               (WP-56; DB-G84) — never a silent fallback
 *   head        the newest HEAD of the census (`watchCensus`), or `null`:
 *               the generation the store has NOW, against `lag`'s, which is
 *               the generation the page READ (DB-022)
 *   how         how the CURRENT value's rows came to be current (`CensusHow`):
 *               read whole, merged from a delta of the rows changed since the
 *               generation held, or unchanged; and, when a refresh had to
 *               read the census whole, why, in words — `null` unless ready
 *               (WP-56, D-3327 item 3; DB-022, DB-060)
 *
 * @typedef {StoreCensus & {
 *   state: 'none' | 'reading' | 'ready' | 'error',
 *   feed: string | null,
 *   at: number | null,
 *   error: string | null,
 *   busy: string | null,
 *   generation: number,
 *   reads: number,
 *   lastOk: { at: number | null, feed: string | null },
 *   lag: import('$lib/store-census.js').CensusLag | null,
 *   worker: string | null,
 *   head: import('$lib/store-census.js').HeadCheck | null,
 *   how: import('$lib/store-census.js').CensusHow | null
 * }} StoreReading
 */

/* THE ANNOTATION IS LOAD-BEARING, NOT DECORATION. Every field seeded `null`
   here infers the type `null` and nothing wider, so `store.at = Date.now()` is
   "type 'number' is not assignable to type 'null'" — the read succeeds and the
   stamp will not go on. The four nullable fields are exactly the four this
   module's failure contract clears, so the type that admits both states is the
   type that lets the contract be written at all. */
// Census rows are immutable snapshots, replaced only after a complete read.
// Deep proxying 148,222 rows creates a reactive object for every inspected row
// and field even though none can change independently. Keep the array raw;
// its accessor remains reactive when the complete snapshot is replaced.
/** @type {StoreRow[]} */
let rawRows = $state.raw([]);
/** @type {StoreReading} */
export const store = $state({
  state: 'none',
  feed: null,
  at: null,
  error: null,
  busy: null,
  generation: 0,
  reads: 0,
  lastOk: { at: null, feed: null },
  lag: null,
  worker: null,
  head: null,
  how: null,
  ...empty(),
  get rows() { return rawRows; },
  set rows(value) { rawRows = value; }
});

/* ======================================================================
   THE CENSUS WORKER (WP-56, D-3327; DB-G84)
   ----------------------------------------------------------------------
   The census body is parsed and decoded in a Web Worker
   (`$lib/census-worker.js`), and the page builds its row objects from the
   columns the worker sends back in slices of at most 8 ms, then folds them
   in slices (`foldCensus`). Before, the page parsed 33 MB, built 385,442
   rows and folded them on its own thread: one continuous stall of over two
   seconds on GDFL with /db's preparation (DB-G84).

   A browser with no Worker, or a worker that fails, decodes on the page as
   before, and `store.worker` names that it did and why. `pageDecodes` is the
   reason of the read in flight, published with its rows.
   ====================================================================== */
const censusWorker = typeof Worker === 'function'
  ? createCensusWorker(() => new Worker(new URL('./census-worker.js', import.meta.url), { type: 'module' }))
  : null;

/** Why the read in flight was decoded on the page, or null. @type {string | null} */
let pageDecodes = null;

/** @type {import('$lib/store-census.js').CensusBodyReader} */
async function readCensusBody(response, feed, signal) {
  if (!censusWorker || censusWorker.broken) {
    pageDecodes = censusWorker
      ? `the census worker failed (${censusWorker.broken}), so this page decoded the census on its own thread`
      : 'this browser runs no Web Worker, so this page decoded the census on its own thread';
    return readOnPage(response, feed, signal);
  }
  const bytes = await response.arrayBuffer();
  try {
    const rows = await censusWorker.decode(bytes, feed, signal);
    pageDecodes = null;
    return rows;
  } catch (why) {
    // A REFUSED CENSUS IS THE CENSUS'S FAULT, NOT THE WORKER'S: the worker
    // refuses a malformed body with the same sentence the page would, and
    // that refusal stands. Only a worker that died is stepped around.
    if (signal.aborted || !censusWorker.broken) throw why;
    pageDecodes = `the census worker failed (${censusWorker.broken}), so this page decoded the census on its own thread`;
    return decodeCensus(JSON.parse(new TextDecoder().decode(bytes)), feed, signal);
  }
}

const census = createCensusLoader(ask, STORE_CENSUS_MS, ({ nextAttempt, delayMs }) => {
  store.busy = `The database reader is busy. Retrying in ${delayMs / 1000} s (attempt ${nextAttempt} of 3).`;
}, readCensusBody);

/**
 * `/census/facets.json`, shared by every page (D-3327 item 3): a cold load
 * per selection, then `?since=` deltas.
 */
export const facets = createFacetsLoader(ask);

/**
 * Shared selected-feed response, including its HTTP status and headers. The
 * request and JSON body have one 30-second deadline. Refresh advances the same
 * generation used by the folded store, so Backtest and DB reuse one census.
 * Successful body rows are shared read-only; no server-summary endpoint is needed.
 * @param {string} feed
 * @returns {Promise<import('$lib/store-census.js').CensusAnswer>}
 */
export function readStoreCensus(feed) {
  return census.load(feed, store.generation);
}

/* `${feed}#${generation}` already asked for. NOT reactive: it is bookkeeping
   about a request, and an effect that reads what it writes re-runs forever. */
/** @type {string | null} */
let asked = null;
/* The read in flight, so a caller that needs the answer before it can measure
   anything — the ingest baseline — awaits the SHARED request. */
/** @type {Promise<void> | null} */
let flight = null;
/* THE IDENTITY OF THAT READ, and nothing else — `{}` is compared by reference
   and carries no fields, which is the whole of what a token is for. */
/** @type {object | null} */
let flightToken = null;
/* CANCELS THE SLICED FOLD OF A READ A NEWER ONE HAS REPLACED: its slices
   would otherwise keep spending the page's time on an answer nobody will
   publish. */
/** @type {AbortController | null} */
let flightAbort = null;
/* The feed the last `syncStore` asked about, so the poll and Refresh know what
   to re-read without importing the feed selection and making an import cycle. */
/** @type {string | null} */
let wanted = null;
/* THE FEED THE CURRENT VALUE BELONGS TO, MIRRORED OUTSIDE THE `$state`.
   `syncStore` is called FROM an effect, so everything `read` touches
   synchronously is inside that effect's tracking scope. Reading `store.feed`
   there — a field this function also WRITES — would make the subscription
   depend on its own output and re-run until Svelte aborts it. The mirror is
   written beside every write of `store.feed` and never diverges. */
/** @type {string | null} */
let valueFeed = null;

function clearValue() {
  Object.assign(store, empty());
  store.lag = null;
  store.worker = null;
  store.how = null;
}

/**
 * Fold ONE ROW into every shape any page asks for.
 *
 * The readable cells are shared BY REFERENCE between `readable`, `byInstrument`
 * and `byMonth.list` — three views of one object, never three copies of it.
 *
 * THE PARAMETER IS THE ENDPOINT'S PROMISE, `StoreRow`, and the `rowFault`
 * gate on the next line is what turns that promise into a fact one row at a
 * time. Nothing here reads a field of a row the gate has not already passed,
 * which is why the promise is safe to declare and why the two shapes below —
 * `StoreCell` for a row that parsed, `BadRow` for one that did not — are
 * separate types rather than one optional-everything type shared by both.
 *
 * @param {StoreCensus} out
 * @param {StoreRow} row
 */
function foldRow(out, row) {
  const why = rowFault(row);
  if (why !== null) {
    const it = { instrument: row?.instrument, month: row?.month, timeframe: row?.timeframe, why };
    out.bad.push(it);
    if (typeof it.instrument === 'string' && it.instrument !== '') {
      let list = out.badByInstrument.get(it.instrument);
      if (!list) out.badByInstrument.set(it.instrument, (list = []));
      list.push(it);
    }
    return;
  }
  const cell = {
    instrument: row.instrument,
    month: row.month,
    timeframe: row.timeframe,
    rows: row.rows,
    // THE WINDOW THE MONTH ACTUALLY COVERS, in MICROSECONDS, on the wire
    // since the census gained it. Deriving it from the month name would be a
    // guess: a month holding one bar covers one day, and which day is a fact
    // only the entry has.
    first: Number.isFinite(row.first_ts) ? row.first_ts : null,
    last: Number.isFinite(row.last_ts) ? row.last_ts : null
  };
  out.readable.push(cell);
  out.cells += 1;
  out.bars += cell.rows;

  let byIns = out.byInstrument.get(cell.instrument);
  if (!byIns) out.byInstrument.set(cell.instrument, (byIns = []));
  byIns.push(cell);

  out.byCell.set(cellKey(cell.instrument, cell.timeframe, cell.month), cell.rows);

  let byMo = out.byMonth.get(cell.month);
  if (!byMo) out.byMonth.set(cell.month, (byMo = { cells: 0, bars: 0, list: [] }));
  byMo.cells += 1;
  byMo.bars += cell.rows;
  byMo.list.push(cell);
}

/** ASCENDING BY (month, rung). @param {StoreCell} a @param {StoreCell} b */
const ladder = (a, b) => cmp(a.month, b.month) || cmp(rungSeconds(a.timeframe) ?? 0, rungSeconds(b.timeframe) ?? 0);

/**
 * Fold one answer into every shape any page asks for: ONE PASS, IN SLICES.
 *
 * The pass was one synchronous loop over every row, 83 to 137 ms for 246,147
 * rows in the audit's in-page benchmark (DB-G84) and more at today's 385,442:
 * a stall on its own. It is the same pass now, cut into slices of at most
 * 8 ms of work with a real task between them (`inSlices`), so timers, input
 * and rendering run while it folds. Nothing is published until the whole
 * fold is done: the caller assigns it in one step, so no page ever reads a
 * half-folded census.
 *
 * @param {StoreRow[]} rows the census array exactly as `/store.json` sent it.
 * @param {AbortSignal} [signal] cancels a fold a newer read has replaced.
 * @returns {Promise<StoreCensus>}
 */
export async function foldCensus(rows, signal) {
  const out = empty();
  out.rows = rows;
  await inSlices(rows.length, (start, end) => {
    for (let i = start; i < end; i++) foldRow(out, rows[i]);
  }, signal);
  // ASCENDING BY (month, rung), so "the latest month" is the last element
  // everywhere and a month held at two rungs keeps them adjacent. Sorted ONCE
  // here rather than once per page, per render.
  const lists = [...out.byInstrument.values()];
  await inSlices(lists.length, (start, end) => {
    for (let i = start; i < end; i++) lists[i].sort(ladder);
  }, signal);
  return out;
}

/**
 * The one fetch. At most ONE per (feed, generation), whatever asks for it.
 *
 * Returns the in-flight promise when one is already running for this key, so
 * three pages and a poll all await the SAME request rather than racing four.
 *
 * @param {string} feed the wire name of the feed to read. Never null: the one
 *        caller that can hold a null feed, `syncStore`, returns before here.
 * @param {number} generation the shared clock's value at the call.
 * @returns {Promise<void>} the SHARED request — the same promise every other
 *        caller of this (feed, generation) is given.
 */
function read(feed, generation) {
  const key = `${feed}#${generation}`;
  if (asked === key) return flight ?? Promise.resolve();
  asked = key;

  // A FEED CHANGE DROPS THE VALUE BEFORE THE REQUEST LEAVES. Leaving the
  // previous feed's counts standing under the new feed's name is the stale
  // value §4 bans, and it is worse than a blank because every number in it was
  // really counted — from the wrong store. A REFRESH of the same feed keeps the
  // value: `state` reads 'reading' and the page shows it under a "refreshing"
  // mark, because it is that feed's own previous answer and is labelled as one.
  if (valueFeed !== feed) {
    clearValue();
    valueFeed = null;
    store.feed = null;
    store.at = null;
  }
  store.state = 'reading';
  store.error = null;
  store.busy = null;

  const token = {};
  flightToken = token;
  flightAbort?.abort();
  const abort = new AbortController();
  flightAbort = abort;
  const mine = census.load(feed, generation)
    .then(async (r) => {
      if (!r.ok) return Promise.reject(new Error(censusFailure(r)));
      const body = r.body;
      if (asked !== key || flightToken !== token) return;
      if (!Array.isArray(body)) throw new Error('the body of /store.json is not a JSON array');
      // THE WORKER'S VERDICT IS THE ONE THIS READ WAS DECODED UNDER. Only a
      // whole read decodes a census: an unchanged answer and a merged delta
      // keep the rows decoded earlier, under the verdict already published.
      const decodedHow = r.how && r.how.kind !== 'whole' ? store.worker : pageDecodes;
      const folded = valueFeed !== feed || rawRows !== body ? await foldCensus(body, abort.signal) : null;
      if (asked !== key || flightToken !== token) return;
      if (folded) Object.assign(store, folded);
      valueFeed = feed;
      store.feed = feed;
      store.at = r.readAt ?? Date.now();
      store.lag = r.lag ?? censusLag(r.headers);
      store.worker = decodedHow;
      store.how = r.how ?? null;
      store.state = 'ready';
      store.error = null;
      store.busy = null;
      store.reads += 1;
      store.lastOk = { at: store.at, feed };
    })
    .catch((why) => {
      if (asked !== key || flightToken !== token) return;
      // NAMED, NOT SHRUGGED, AND THE STAMP GOES WITH IT. A page that says
      // "nothing stored" over a failed request states a fact about a disk it
      // never read. `lastOk` survives under its own name; `at` and `feed` do
      // not, because they are the claim that THIS value is current.
      clearValue();
      valueFeed = null;
      store.feed = null;
      store.at = null;
      store.state = 'error';
      store.error = String(why?.message ?? why);
      store.busy = null;
      // The key is released so a Refresh press can retry the same
      // (feed, generation) without needing a bump nobody asked for.
      asked = null;
    });

  // The token, not the promise, decides whether this read is still the current
  // one when it lands. A newer feed or generation replaces `flightToken` on its
  // way past, and this one then clears nothing.
  flight = mine.finally(() => {
    if (flightToken === token) flight = null;
  });
  return flight;
}

/**
 * SUBSCRIBE. Call inside a `$effect`, passing the feed the page is asking about.
 *
 *   $effect(() => syncStore(feeds.active));
 *
 * The effect reads `feeds.active` (through the argument) and `store.generation`
 * (inside), so it re-runs on a feed change AND on a generation bump — which is
 * how a Refresh pressed on one page, or the poll held by another, reaches every
 * subscriber without any page owning a timer of its own.
 *
 * The feed is a PARAMETER rather than an import: `/autopilot` deliberately
 * follows the autopilot's own target feed rather than the top bar's selection,
 * and taking the feed here keeps that possible while keeping this module free
 * of every import — no cycle with `feeds.svelte.js`, which reads it back.
 *
 * @param {string | null | undefined} feed `feeds.active` is `string | null`
 *        until the feed list has landed, and a page mounts before it does. A
 *        null feed is not an error here: it is "nothing to ask about yet", and
 *        it is recorded in `wanted` so a later Refresh knows there is nothing
 *        to re-read either.
 */
export function syncStore(feed) {
  const generation = store.generation; // TRACKED — this is the shared clock
  wanted = feed ?? null;
  if (!feed) {
    /* NO FEED NOW CLEARS, BECAUSE NULL STOPPED MEANING ONLY "NOT YET".
       The parameter doc above is accurate for the case it was written for — a
       page mounting before `/feeds.json` lands — and in that case there is
       nothing to clear, so this is a no-op.

       It is no longer the only case. The operator can now clear the feed
       outright (`+layout.svelte`: choosing the chosen feed again), and this
       function returned WITHOUT touching `store`, leaving the previous feed's
       fold live — `state === 'ready'`, its rows, its counts, its `error`.
       Pages that stamp their readings against `feeds.active` survived that;
       three do not, and rendered the old feed's numbers under a heading
       saying no feed was selected: `/db`'s `error`, `/audit`'s `payload`,
       `/mapping`'s `load`. `read()` is the only other writer that clears, and
       it is precisely what this branch skips.

       Clearing at the one place all of them share beats stamping three pages
       separately. `empty()` is the same unasked state the module starts in, so
       "no feed" and "before the first feed" become one state — which is what
       they actually are. The request token and stamp are also cleared: an
       older response cannot publish again after this explicit clear. */
    clearValue();
    asked = null;
    flightToken = null;
    flightAbort?.abort();
    flightAbort = null;
    flight = null;
    valueFeed = null;
    store.feed = null;
    store.at = null;
    store.error = null;
    store.busy = null;
    store.state = 'none';
    return;
  }
  read(feed, generation);
}

/**
 * REFRESH IS A GENERATION BUMP, NOT A PER-PAGE FETCH.
 *
 * The bump re-runs every subscriber's effect, which calls `syncStore`, which
 * finds a key it has not asked for and issues exactly ONE request. Awaiting the
 * returned promise waits for that same request — the caller that needs a
 * reading before it can measure anything gets the shared one, not a seventh.
 *
 * @returns {Promise<void>}
 */
export function refreshStore() {
  store.generation += 1;
  if (!wanted) return Promise.resolve();
  return read(wanted, store.generation);
}

/* ======================================================================
   THE POLL — ONE CLOCK, HELD BY WHOEVER NEEDS IT, SEEN BY EVERYONE.
   ----------------------------------------------------------------------
   `/ingest` wants five seconds while a pull is running; `/autopilot` wants
   thirty. Two timers reading the same endpoint is how two pages come to hold
   two different totals for one store. There is ONE timer, its period is the
   finest any holder asked for, and it bumps the generation — so the answer it
   fetches is the answer every page is already reading.
   ====================================================================== */
/**
 * ONE PAGE'S CLAIM ON THE SHARED TIMER. `live` is not the same fact as
 * membership of `holders`: the release is idempotent, and `live` is what makes
 * a second call to it a no-op rather than a second `filter` pass.
 *
 * @typedef {{ everyMs: number, live: boolean }} PollHolder
 */

/** @type {PollHolder[]} */
let holders = [];
let ticking = false;

/**
 * Hold the poll at `everyMs`. Returns the release — call it from cleanup.
 *
 * @param {number} everyMs the period this holder wants. Floored at one second,
 *        and a non-number floors with it, because the timer is shared and the
 *        finest holder sets the period for every page at once.
 * @returns {() => void} the release. Calling it twice is safe.
 */
export function watchStore(everyMs) {
  const holder = { everyMs: Math.max(1000, Math.trunc(everyMs) || 1000), live: true };
  holders.push(holder);
  armWake();
  if (!ticking) {
    ticking = true;
    tick();
  }
  return () => {
    if (!holder.live) return;
    holder.live = false;
    holders = holders.filter((h) => h !== holder);
  };
}

/* A HIDDEN TAB IS NOBODY READING, AND THIS POLL IS NOT CHEAP.
   ----------------------------------------------------------------------
   `/store.json` is the whole census and it grows with the store: measured on
   the running binary mid-pull, 377,735 bytes in 375 ms for one feed, up from
   329 KB minutes earlier. At a five-second period that is ~75 KB/s of transfer
   plus a JSON parse of the same size on the main thread — sustained, for as
   long as a run lasts, whether or not the tab is in front of anyone.

   `/audit` already reached this conclusion and wrote it down: "stopped while
   the tab is hidden — a monitoring page that keeps a" clock running in the
   background is spending someone's battery to answer a question nobody asked.
   That page carries its own visibility check; this poll is the one every OTHER
   page shares, and it had none.

   SKIPPED, NOT STOPPED. The loop keeps its cadence so a holder is still held
   and the release contract is unchanged; only the READ is skipped. And
   `wake()` fires one immediately on becoming visible, because the first thing
   a returning reader deserves is a current number rather than one up to a
   period old. */
function visible() {
  return typeof document === 'undefined' || !document.hidden;
}

/* REGISTERED ONCE FOR THE MODULE, not once per holder: several pages can hold
   the clock at the same time and N listeners would fire N reads for one
   return-to-tab, which is the stampede this whole module exists to prevent. */
let woken = false;
function armWake() {
  if (woken || typeof document === 'undefined') return;
  woken = true;
  document.addEventListener('visibilitychange', () => {
    if (holders.length > 0 && visible()) void refreshStore();
  });
}

async function tick() {
  while (holders.length > 0) {
    const period = Math.min(...holders.map((h) => h.everyMs));
    await new Promise((done) => setTimeout(done, period));
    if (holders.length === 0) break;
    if (!visible()) continue;
    // AWAITED, SO A SLOW ANSWER CANNOT STACK REQUESTS ON TOP OF ITSELF. The
    // period is the gap BETWEEN reads, never the rate they are fired at.
    await refreshStore();
  }
  ticking = false;
}

/* ======================================================================
   THE GENERATION WATCH — "the store moved since this page read it" (DB-022)
   ----------------------------------------------------------------------
   /db read the census once per feed and never looked again, so during an
   import or a pull every list on it was the opening snapshot with nothing
   saying so. This asks `HEAD /store.json` — headers only, the census's own
   generation — every `HEAD_POLL_MS` while the tab is visible, and keeps the
   answer in `store.head`. It never re-reads the census itself: a page
   compares `store.head` with `store.lag` (`censusMoved`) and OFFERS the
   refresh, because re-downloading 33 MB every few seconds during an import
   is the cost DB-060 names, and a list that changes under the operator's
   cursor is worse than one that says it is a snapshot.
   ====================================================================== */
/** How often the watch asks, in milliseconds. */
export const HEAD_POLL_MS = 5_000;

/**
 * One HEAD of `feed`'s census into `store.head`. A failed HEAD is kept as
 * one, with its reason; it never clears the census the page holds, which is
 * still the snapshot it says it is.
 * @param {string} feed
 * @param {typeof ask} [request]
 */
export async function checkCensusHead(feed, request = ask) {
  try {
    const response = await request(`/store.json?feed=${encodeURIComponent(feed)}`, {
      method: 'HEAD', cache: 'no-store', ms: 15_000
    });
    if (!response.ok) throw new Error(`HEAD /store.json?feed=${feed} answered HTTP ${response.status}`);
    store.head = { feed, at: Date.now(), lag: censusLag(response.headers), error: null };
  } catch (why) {
    store.head = { feed, at: Date.now(), lag: null, error: String(/** @type {any} */ (why)?.message ?? why) };
  }
}

/**
 * WATCH ONE FEED'S GENERATION. Call from an `$effect` and return the release:
 *
 *   $effect(() => watchCensus(feeds.active));
 *
 * A feed change re-runs the effect, which releases the old watch and starts
 * one for the new feed. Skipped while the tab is hidden, and asked once at
 * once when it becomes visible again.
 *
 * @param {string | null | undefined} feed
 * @param {number} [everyMs]
 * @returns {() => void} the release; calling it twice is safe
 */
export function watchCensus(feed, everyMs = HEAD_POLL_MS) {
  if (!feed) return () => {};
  let live = true;
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let timer;
  const tick = async () => {
    if (!live) return;
    if (visible()) await checkCensusHead(feed);
    if (live) timer = setTimeout(tick, everyMs);
  };
  timer = setTimeout(tick, everyMs);
  const onVisible = () => {
    if (live && visible()) void checkCensusHead(feed);
  };
  if (typeof document !== 'undefined') document.addEventListener('visibilitychange', onVisible);
  return () => {
    if (!live) return;
    live = false;
    clearTimeout(timer);
    if (typeof document !== 'undefined') document.removeEventListener('visibilitychange', onVisible);
  };
}

/* ======================================================================
   THE FOLD THAT DEPENDS ON A QUESTION ONE PAGE ASKS.
   ====================================================================== */
/**
 * THE REQUEST'S OWN POPULATION, or the absence of one.
 *
 * Either half absent means "every one of them". A present-but-empty Set is NOT
 * the same statement — it is "none of them" — and `$lib/fold.js` tests
 * `instanceof Set` rather than truthiness for exactly that reason.
 *
 * @typedef {{ instruments?: Set<string>, rungs?: Set<string> } | null | undefined} FoldScope
 */

/**
 * ONE WINDOW FOLD. `at` and `held` are this module's — the stamp the reading
 * carries and the WHOLE store's row count — and the rest is `foldWindow`'s
 * arithmetic over the named months.
 *
 * `held` is deliberately not the window's count; see the note on `foldMonths`.
 *
 * @typedef {{
 *   at: number | null,
 *   held: number,
 *   units: number,
 *   rows: number,
 *   byInstrument: Map<string, number>,
 *   outside: number,
 *   scoped: boolean
 * }} WindowFold
 */

/** @type {{ key: string | null, value: WindowFold | null }} */
let foldMemo = { key: null, value: null };

/**
 * The window fold `/ingest` measures a run against: how many instrument-months
 * inside these months are held, how many bars they hold, and how many bars each
 * instrument holds inside them.
 *
 * It walks ONLY the months named — `byMonth` is the index, so a one-month window
 * costs one month and never a pass over the whole store — and it is memoised on
 * (successful read, months), so re-rendering never re-folds.
 *
 * `held` is the WHOLE store's row count, not the window's: it is the figure
 * "3,353 instrument-month row(s) read" is printed from, and narrowing it to the
 * window would silently change what that sentence means.
 *
 * # THE SCOPE, AND THE 100% METER IT REMOVES
 *
 * `scope` is `{ instruments: Set, rungs: Set }` — the census keys and the rung
 * names the caller's own request reaches — and either half may be absent, which
 * means "every one of them".
 *
 * It exists because the month was the ONLY predicate here. `/ingest` divides
 * this fold's `units` by a denominator it computes from its own request
 * (instruments × segments × rungs × months), so with a 750-name equity backfill
 * already in the window month, a two-index request measured 751 held units
 * against a denominator of 2, `Math.min(1, …)` pinned the meter at 100.0%, and
 * `unitsLeft` clamped to 0 — which also removed the ETA line — while half the
 * request was still outstanding. A numerator and a denominator over two
 * different populations is not a ratio. The threshold was two pre-existing rows
 * in the window, not 750.
 *
 * @param {Iterable<string> | null | undefined} months the month keys to walk.
 *        Taken as an iterable and copied once, because the caller's own value
 *        is a `$derived` array that may be replaced while this runs.
 * @param {FoldScope} scope
 * @returns {WindowFold}
 */
export function foldMonths(months, scope) {
  const list = [...(months ?? [])];
  const key = foldKey(store.reads, list, scope);
  // THE VALUE IS CHECKED, NOT ONLY THE KEY. `foldMemo` starts
  // `{ key: null, value: null }`, so a hit is only a hit if something was
  // actually stored — and a memo that returns null on a hit would hand every
  // caller `shot.units` on nothing. The checker flagged exactly that: six
  // "possibly null" reads on `/ingest` traced back through this one return.
  if (foldMemo.key === key && foldMemo.value) return foldMemo.value;
  // THE ARITHMETIC IS IN `$lib/fold.js`, WHERE A TEST CAN REACH IT. This
  // function owns the shared reading, the stamp and the memo; what a scoped
  // window actually counts is a pure function with no runes in it.
  const value = { at: store.at, held: store.rows.length, ...foldWindow(store.byMonth, list, scope) };
  foldMemo = { key, value };
  return value;
}

/* ======================================================================
   THE SURVEY — a DIFFERENT question, asked once.
   ----------------------------------------------------------------------
   "Which feed holds anything at all" is not "what does this feed hold". It
   spans every feed and is requested explicitly by `/db` when the operator
   needs to find held data. It no longer delays the shared feed selection.
   ====================================================================== */
/**
 * WHAT ONE FEED HOLDS, as the survey found it.
 *
 * `any` is "this feed has ENTRIES", which is not "this feed has countable
 * ones": a store whose rows this build cannot parse is still somewhere to go.
 * `error` carries one feed's failure without making it every feed's, so a
 * caller can say "this feed could not be read" instead of the different and
 * false sentence "this feed holds nothing".
 *
 * @typedef {{
 *   wire: string,
 *   ready: boolean,
 *   bars: number,
 *   cells: number,
 *   any: boolean,
 *   error: string | null
 * }} FeedHolding
 */

/**
 * @typedef {{
 *   state: 'none' | 'reading' | 'ready' | 'error',
 *   at: number | null,
 *   error: string | null,
 *   wires: string,
 *   byFeed: Map<string, FeedHolding>
 * }} SurveyReading
 */

/** @type {SurveyReading} */
export const survey = $state({
  state: 'none', // 'none' | 'reading' | 'ready' | 'error'
  at: null,
  error: null,
  wires: '', // THE FEED LIST IT ANSWERED FOR — the same stamp lesson as `feed`
  byFeed: new Map() // wire -> FeedHolding
});

/** @type {string | null} */
let surveyAsked = null;
/** @type {Promise<SurveyReading> | null} */
let surveyFlight = null;

/**
 * Read every feed's store once. `list` is `[{ wire, ready }, …]` from
 * `/feeds.json`. At most one pass per (feed list, generation). A Refresh,
 * which bumps the generation, re-asks it. This is not part of shared startup.
 *
 * `Feed` is REFERENCED, NOT REDEFINED. `$lib/feeds.svelte.js` owns that shape
 * and both callers pass it their `feeds.all`; a second local spelling of it
 * here is the drift where one copy gains a field and the other does not. A
 * JSDoc `import(…)` is erased before anything runs, so the acyclic import graph
 * this module keeps — nothing imported but `$lib/fold.js` — is untouched by it.
 *
 * @param {import('$lib/feeds.svelte.js').Feed[] | null | undefined} list
 * @returns {Promise<SurveyReading>}
 */
export function surveyStores(list) {
  // ONLY THE FEEDS `/feeds.json` OFFERS (D-3338): the operator uses Zerodha
  // and GDFL, and the empty Dhan, Groww and TrueData stores were HEADed on
  // every /db load all the same (DB-G84: five HEADs per load).
  const feedsIn = offeredFeeds([...(list ?? [])]);
  const wires = feedsIn.map((f) => f.wire).join(',');
  const generation = store.generation;
  const key = `${wires}#${generation}`;
  if (surveyAsked === key) return surveyFlight ?? Promise.resolve(survey);
  surveyAsked = key;
  // A DIFFERENT FEED LIST IS A DIFFERENT QUESTION, so the previous answer is
  // dropped before the new one is asked rather than sitting under it.
  if (survey.wires !== wires) {
    survey.byFeed = new Map();
    survey.at = null;
    survey.wires = '';
  }
  survey.state = 'reading';
  survey.error = null;

  surveyFlight = pooled(feedsIn, 2, (f) =>
      readFeedHeader(f.wire, ask)
        .then(({ bars, cells }) => {
          // Finding which feed holds data needs committed header counters,
          // never five complete inventory downloads and folds.
          return { wire: f.wire, ready: f.ready, bars, cells, any: cells > 0, error: null };
        })
        // ONE FEED'S FAILURE IS NOT EVERY FEED'S. The reason rides on the row so
        // a caller can say "this feed could not be read" rather than "this feed
        // holds nothing", which is a claim about a disk nobody reached.
        .catch((why) => ({
          wire: f.wire,
          ready: f.ready,
          bars: 0,
          cells: 0,
          any: false,
          error: String(why?.message ?? why)
        }))
  )
    .then((all) => {
      if (surveyAsked !== key) return survey;
      survey.byFeed = new Map(all.map((f) => [f.wire, f]));
      survey.wires = wires;
      survey.at = Date.now();
      survey.state = 'ready';
      survey.error = null;
      return survey;
    })
    .catch((why) => {
      if (surveyAsked !== key) return survey;
      survey.byFeed = new Map();
      survey.wires = '';
      survey.at = null;
      survey.state = 'error';
      survey.error = String(why?.message ?? why);
      surveyAsked = null;
      return survey;
    })
    .finally(() => {
      surveyFlight = null;
    });
  return surveyFlight;
}
