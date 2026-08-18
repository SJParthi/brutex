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
import { ask } from '$lib/ask.js';

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

/** The shape a census month is written in. One spelling of the test, shared. */
export const MONTH_KEY = /^\d{4}-\d{2}$/;

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
 * `Number.isInteger`, restated as the type guard it already is at run time.
 *
 * The standard-library signature is `isInteger(number: unknown): boolean`, so a
 * checked value stays exactly as wide as it arrived and the comparison on the
 * VERY NEXT token — `row.rows < 0` — is then read as a comparison against a
 * count that may not be there. The predicate states what the call has already
 * proved; it adds no test, and at run time this is `Number.isInteger` and
 * nothing else.
 *
 * @param {unknown} v
 * @returns {v is number}
 */
const isWholeNumber = (v) => Number.isInteger(v);

/**
 * `null` when the row is readable; otherwise the reason it is not, in words.
 *
 * A ROW THAT FAILS IS KEPT, NEVER DROPPED AND NEVER COERCED. A census row whose
 * `rows` is absent, negative or fractional is a row whose count is UNKNOWN.
 * Dropping it makes it indistinguishable from a month that does not exist;
 * writing zero for it prints a measurement nobody took. It goes to `bad` with
 * the reason, and every total computed without it can say so.
 *
 * THE PARAMETER IS `WireRow` AND NOT `StoreRow`, DELIBERATELY. `StoreRow` is
 * what `/store.json` PROMISES; this function exists because a promise is not a
 * proof, and typing its input as the promise would make every test below dead
 * code to the checker — a fault detector the type system believes can never
 * fire. `WireRow` assumes only that the four fields MAY be present, which is
 * the state of knowledge on entry, and each `typeof` here is what carries a
 * field from "may be" to "is". Every `StoreRow` is a `WireRow`, so the two call
 * sites pass without a cast at either of them.
 *
 * @param {WireRow | null | undefined} row
 * @returns {string | null}
 */
function rowFault(row) {
  if (typeof row?.instrument !== 'string' || row.instrument === '')
    return `its \`instrument\` field is ${JSON.stringify(row?.instrument)}, which is not an instrument key`;
  if (typeof row.month !== 'string' || !MONTH_KEY.test(row.month))
    return `its \`month\` field is ${JSON.stringify(row.month)}, which is not a YYYY-MM month`;
  if (typeof row.timeframe !== 'string' || row.timeframe === '')
    return `its \`timeframe\` field is ${JSON.stringify(row.timeframe)}`;
  if (!isWholeNumber(row.rows) || row.rows < 0)
    return `its \`rows\` field is ${JSON.stringify(row.rows)}, which is not a whole number of records`;
  return null;
}

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
 *
 * @typedef {StoreCensus & {
 *   state: 'none' | 'reading' | 'ready' | 'error',
 *   feed: string | null,
 *   at: number | null,
 *   error: string | null,
 *   generation: number,
 *   reads: number,
 *   lastOk: { at: number | null, feed: string | null }
 * }} StoreReading
 */

/* THE ANNOTATION IS LOAD-BEARING, NOT DECORATION. Every field seeded `null`
   here infers the type `null` and nothing wider, so `store.at = Date.now()` is
   "type 'number' is not assignable to type 'null'" — the read succeeds and the
   stamp will not go on. The four nullable fields are exactly the four this
   module's failure contract clears, so the type that admits both states is the
   type that lets the contract be written at all. */
/** @type {StoreReading} */
export const store = $state({
  state: 'none',
  feed: null,
  at: null,
  error: null,
  generation: 0,
  reads: 0,
  lastOk: { at: null, feed: null },
  ...empty()
});

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
}

/**
 * Fold one answer into every shape any page asks for. ONE PASS.
 *
 * The readable cells are shared BY REFERENCE between `readable`, `byInstrument`
 * and `byMonth.list` — three views of one object, never three copies of it.
 *
 * THE PARAMETER IS THE ENDPOINT'S PROMISE, `StoreRow[]`, and the `rowFault`
 * gate on the next line is what turns that promise into a fact one row at a
 * time. Nothing here reads a field of a row the gate has not already passed,
 * which is why the promise is safe to declare and why the two shapes below —
 * `StoreCell` for a row that parsed, `BadRow` for one that did not — are
 * separate types rather than one optional-everything type shared by both.
 *
 * @param {StoreRow[]} rows the census array exactly as `/store.json` sent it.
 * @returns {StoreCensus}
 */
function fold(rows) {
  const out = empty();
  out.rows = rows;
  for (const row of rows) {
    const why = rowFault(row);
    if (why !== null) {
      const it = { instrument: row?.instrument, month: row?.month, timeframe: row?.timeframe, why };
      out.bad.push(it);
      if (typeof it.instrument === 'string' && it.instrument !== '') {
        let list = out.badByInstrument.get(it.instrument);
        if (!list) out.badByInstrument.set(it.instrument, (list = []));
        list.push(it);
      }
      continue;
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
  // ASCENDING BY (month, rung), so "the latest month" is the last element
  // everywhere and a month held at two rungs keeps them adjacent. Sorted ONCE
  // here rather than once per page, per render.
  for (const list of out.byInstrument.values())
    list.sort(
      (a, b) => cmp(a.month, b.month) || cmp(rungSeconds(a.timeframe) ?? 0, rungSeconds(b.timeframe) ?? 0)
    );
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

  const mine = ask(`/store.json?feed=${encodeURIComponent(feed)}`)
    .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status} from /store.json`))))
    .then((body) => {
      if (asked !== key) return; // a newer feed or generation won; this answer is stale
      if (!Array.isArray(body)) throw new Error('the body of /store.json is not a JSON array');
      Object.assign(store, fold(body));
      valueFeed = feed;
      store.feed = feed;
      store.at = Date.now();
      store.state = 'ready';
      store.error = null;
      store.reads += 1;
      store.lastOk = { at: store.at, feed };
    })
    .catch((why) => {
      if (asked !== key) return;
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
      // The key is released so a Refresh press can retry the same
      // (feed, generation) without needing a bump nobody asked for.
      asked = null;
    });

  // The token, not the promise, decides whether this read is still the current
  // one when it lands. A newer feed or generation replaces `flightToken` on its
  // way past, and this one then clears nothing.
  const token = {};
  flightToken = token;
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
  if (!feed) return;
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

async function tick() {
  while (holders.length > 0) {
    const period = Math.min(...holders.map((h) => h.everyMs));
    await new Promise((done) => setTimeout(done, period));
    if (holders.length === 0) break;
    // AWAITED, SO A SLOW ANSWER CANNOT STACK REQUESTS ON TOP OF ITSELF. The
    // period is the gap BETWEEN reads, never the rate they are fired at.
    await refreshStore();
  }
  ticking = false;
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
   spans every feed, it is what picks the default selection on load, and it is
   what `/db` names when the selected feed is empty and the operator needs
   somewhere to go. It was two independent N-request folds — one in
   `loadFeeds`, one in `/db` — answering one question on two clocks.
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
 * `/feeds.json`. At most one pass per (feed list, generation), so the boot pass
 * that picks the default is the SAME pass `/db` reads when it finds the selected
 * feed empty — and a Refresh, which bumps the generation, re-asks it.
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
  const feedsIn = [...(list ?? [])];
  const wires = feedsIn.map((f) => f.wire).join(',');
  const key = `${wires}#${store.generation}`;
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

  surveyFlight = Promise.all(
    feedsIn.map((f) =>
      fetch(`/store.json?feed=${encodeURIComponent(f.wire)}`)
        .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`))))
        .then((d) => {
          const rows = Array.isArray(d) ? d : [];
          // THE SAME TEST FOR "READABLE" AS THE CENSUS FOLD, and deliberately
          // the same function. Two definitions of which rows count is how the
          // feed picker comes to report a bar total the page it opens cannot
          // reproduce, and the operator has no way to tell which one lied.
          let bars = 0;
          let cells = 0;
          for (const row of rows) {
            if (rowFault(row) !== null) continue;
            bars += row.rows;
            cells += 1;
          }
          // `any` IS "THIS FEED HAS ENTRIES", NOT "THIS FEED HAS COUNTABLE
          // ONES". It is what `/db` points at when the selected feed is empty,
          // and a store whose rows this build cannot parse is still somewhere
          // to go — saying otherwise would hide a store behind a parse fault.
          return { wire: f.wire, ready: f.ready, bars, cells, any: rows.length > 0, error: null };
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
