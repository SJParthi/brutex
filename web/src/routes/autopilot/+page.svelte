<script>
  /**
   * THE AUTOPILOT — what it is doing right now, and the proof it is doing it.
   *
   * The operator presses Run api in IntelliJ and walks away. This page is how
   * he finds out what happened. It answers three questions and refuses
   * everything else: WHAT IS HAPPENING, WHAT HAPPENED, WHAT IS STUCK.
   *
   * The failure mode of "one click and no manual step" is not a crash; it is a
   * process that is up, answering, and quietly doing nothing, which from the
   * outside is indistinguishable from a process that is working. Everything
   * below exists to make those two states distinguishable at a glance.
   *
   * ---------------------------------------------------------------------------
   * THREE SOURCES, NEVER MERGED, EVERY FIGURE LABELLED WITH ITS OWN
   * ---------------------------------------------------------------------------
   *
   *   reported  `/autopilot.json` — what the process SAYS. A claim. Live, small,
   *             polled every 2s. The only thing that can answer "what is in
   *             flight", "why did it stop" and "what failed".
   *
   *   measured  `/store.json` — what the disk HOLDS. The census, measured off
   *             disk, polled every 30s. It owes the autopilot nothing and would
   *             report the same numbers with the autopilot deleted.
   *
   *   observed  this page — successive polls. The only thing the browser itself
   *             can honestly claim to know, and what answers "what changed
   *             while I was away". It dies with the tab.
   *
   * Coverage is drawn from the second, never from the first. A progress bar fed
   * by the process reporting its own progress reads 100% when the process is
   * lying to itself. `CLAUDE.md` §3 rule 6: "reported" and "measured" are not
   * the same claim, so they are not the same chip.
   *
   * ---------------------------------------------------------------------------
   * THE CLAIM AND ITS EVIDENCE ARE ONE ELEMENT
   * ---------------------------------------------------------------------------
   *
   * `verdict` displays scoped stored observations beside their evidence.
   * The current API supplies an instrument count, not an exact membership
   * list or expected sessions. No aggregate count can certify completion.
   * A reported `complete` remains a report, even if stored counts are larger
   * than the reported instrument count multiplied by the target months.
   *
   * That is not hypothetical. With the instrument masters missing, the process
   * reports `complete` and the store holds nothing: both a run that fetched
   * everything and a run whose masters never loaded find nothing missing. The
   * layout this replaces rendered the claim in one panel and the count in
   * another, so the two could disagree in silence. They are one element now.
   *
   * ---------------------------------------------------------------------------
   * WHEN THE ENDPOINT IS NOT THERE
   * ---------------------------------------------------------------------------
   *
   * `/autopilot.json` is a route the served binary either has or does not have.
   * When it does not, this page says exactly that and withholds target coverage
   * because its feed, timeframe and date window cannot be established. A
   * missing autopilot and an idle autopilot look identical in a blank, and
   * `CLAUDE.md` §4 bans a fallback that hides a failure.
   *
   * The same applies to a 200 with the wrong shape. The reader below validates
   * the payload field by field and reports the FIRST field that was not what
   * the contract says, by name. A page that silently renders `undefined` as an
   * em-dash turns a broken server into a design quirk.
   *
   * ---------------------------------------------------------------------------
   * THE CONTRACT — docs/05-decisions.md D-0057
   * ---------------------------------------------------------------------------
   *
   *   GET  /autopilot.json     the state, the current cell, the target, failures
   *   POST /autopilot/control  `action=stop` finishes the cell in flight and
   *                            stops; `action=resume` continues from the census,
   *                            not from a variable. Both answer
   *                            `{action, accepted, why, status}` — and `why` is
   *                            the server's own sentence, which this page prints
   *                            verbatim and never paraphrases. See `send`.
   *
   * Stop is "finish this cell and stop", not "abort". An aborted write is a
   * half-month the append-only store can never correct, and the whole reason
   * the backfill runs oldest-first is that the store cannot prepend.
   */
  import { feeds } from '$lib/feeds.svelte.js';
  import { store, syncStore, watchStore } from '$lib/store.svelte.js';
  // A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
  // ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
  // threaded through every call site.
  import { ask } from '$lib/ask.js';
  import { watchVisible } from '$lib/page-requests.js';
  import { untrack } from 'svelte';

  /* ======================================================================
     THE SHAPES, WRITTEN DOWN ONCE
     ----------------------------------------------------------------------
     Every shape below is the shape `readState` PRODUCES, not the shape the
     wire sends — the wire's shape is `unknown` until that reader has looked
     at every field, and writing a typedef for the raw body would be a claim
     about a payload this page explicitly refuses to trust. What is written
     here is the post-validation contract of D-0057, field for field, and it
     is the same list the reader checks: if a field moves there, it moves
     here, and the checker reports every reader of it rather than none.

     Without these, `let ap = $state(null)` is typed `null`, and every one of
     the ~90 reads of `ap.state`, `ap.target`, `ap.failures` and `ap.now`
     below is reported as `Property … does not exist on type 'never'` at the
     READER — a hundred reports of one absence, none of them fixable where
     they are reported. `$lib/feeds.svelte.js` records the same lesson about
     the same `$state` inference, one file over.
     ====================================================================== */

  /** The six states of D-0057. `SAYS` is keyed by exactly this union. */
  /** @typedef {'starting' | 'running' | 'waiting' | 'paused' | 'halted' | 'complete'} ApState */

  /** `target` — the span being written, and what it is being written for. */
  /**
   * @typedef {{
   *   from: string | null,
   *   to: string | null,
   *   instruments: number | null,
   *   timeframe: string | null,
   *   feed: string | null
   * }} Target
   */

  /** `now` — the cell in flight, or the whole object is null. */
  /**
   * @typedef {{
   *   instrument: string | null,
   *   month: string | null,
   *   timeframe: string | null,
   *   feed: string | null,
   *   elapsed_ms: number | null,
   *   index: number | null,
   *   of: number | null
   * }} Flight
   */

  /** One row of `failures`. Only `instrument` is substituted when absent. */
  /**
   * @typedef {{
   *   instrument: string,
   *   month: string | null,
   *   why: string | null,
   *   at: string | null
   * }} Failure
   */

  /** A payload that PASSED the reader. Nothing else in this file is one. */
  /**
   * @typedef {{
   *   state: ApState,
   *   why: string | null,
   *   cursor: string | null,
   *   target: Target,
   *   now: Flight | null,
   *   waiting_ms: number | null,
   *   absorbed_ms: number | null,
   *   journal: string | null,
   *   failures: Failure[]
   * }} Autopilot
   */

  /**
   * The reader's own answer. `ok` is a LITERAL on both arms, which is what
   * makes `if (!parsed.ok) return` narrow — a bare `boolean` there and the
   * checker cannot tell the refusal from the value, which is exactly the
   * confusion the two-armed shape exists to prevent.
   *
   * @typedef {{ ok: true, value: Autopilot } | { ok: false, why: string }} ReadResult
   */

  /** The four link states, spelled out — see `link` for why it is not a boolean. */
  /**
   * @typedef {{
   *   kind: 'probing' | 'ok' | 'absent' | 'broken',
   *   why: string | null,
   *   at: number,
   *   ms: number
   * }} Link
   */

  /** `span`'s refusal, carried whole into `ladder.refuse` and printed there. */
  /** @typedef {{ ok: false, field: string, raw: unknown, why: string }} SpanRefusal */
  /** @typedef {{ ok: true, months: string[] } | SpanRefusal} SpanResult */

  /**
   * One target month. Stored presence is not target membership or completeness.
   *
   * @typedef {{
   *   key: string,
   *   cells: number,
   *   bars: number,
   *   unknownCells: number,
   *   unmeasuredBars: number
   * }} Rung
   */

  /** @typedef {Pick<Target, 'feed' | 'timeframe' | 'from' | 'to'>} CoverageTarget */
  /** @typedef {Pick<typeof store, 'state' | 'error' | 'at' | 'feed' | 'readable'>} CensusInput */
  /** @typedef {{ kind: 'ok' | 'broken' | 'probing' | 'unscoped', why: string | null,
   * at: number | null, feed: string | null, months: Map<string, Omit<Rung, 'key'>>,
   * cells: number, bars: number, unknownCells: number, unmeasuredBars: number,
   * duplicates: number }} TargetCensus */

  /** One answer behind a dial: its key, its face, and why it is that answer. */
  /** @typedef {{ k: string, n: string, y: string, off?: boolean }} DialOption */

  /**
   * ONE LINE OF EVIDENCE under the claim: what it is, what it reads, and WHICH
   * OF THE THREE SOURCES it came from. `s` is the union and not a string on
   * purpose — the whole design of this page is that a measured number and a
   * reported one never wear the same chip, and a plain `string` there would
   * let a typo render a fact with no provenance at all.
   *
   * `v` may be null: a fact whose value is unknown is dropped by `evidence`
   * rather than drawn as a blank, and the claim above it is refused when that
   * leaves the list empty.
   *
   * @typedef {{ k: string, v: string | null, s: 'mea' | 'rep' | 'obs' }} Fact
   */

  /** The one verdict, and the facts that are drawn in the same element. */
  /**
   * @typedef {{
   *   tone: 'good' | 'warn' | 'bad' | 'unknown',
   *   claim: string,
   *   sub: string,
   *   facts: Fact[]
   * }} Verdict
   */

  /** The control's receipt. `tone` comes from the SENTENCE — see `classify`. */
  /**
   * @typedef {{
   *   action: string,
   *   tone: 'done' | 'partial' | 'refused' | 'unknown',
   *   code: number,
   *   why: string,
   *   at: number
   * }} Receipt
   */

  /** The live state. Small payload, so a short period is cheap. */
  const TICK_MS = 2000;
  /** The census size depends on the store. Read on the shared slow clock. */
  const CENSUS_MS = 30000;
  /** One route for stop and resume, and the only one that answers a sentence. */
  const CONTROL = '/autopilot/control';
  /** Where the durable record is, when the payload does not say. */
  const JOURNAL = '~/.brutex/store/audit/pull.journal';
  /** `crates/api/src/autopilot.rs` — the grace window before the first socket. */
  const GRACE_SECS = 20;

  /** What each state is DOING, in one clause, for the deck's own face. */
  const SAYS = {
    starting: 'opening the store and reading the census',
    running: 'fetching',
    waiting: 'waiting on the rate governor — absorbing throttle rather than failing',
    paused: 'paused by the operator',
    halted: 'halted, and it named why',
    complete: 'reported done — the beam above is whether the store agrees'
  };

  /* ======================================================================
     NUMBERS — Indian grouping, by hand.

     Not `toLocaleString`: this has to produce the same digits on every
     runtime, and a non-finite value must NOT come back as "NaN" — it comes
     back as null, which forces the caller to name what it does not know.
     There is no path in this file that can print the three characters N a N.
     ====================================================================== */

  /* THE PARAMETER IS `unknown` AND THAT IS THE POINT. Every one of these
     takes whatever the payload carried — the reader hands them fields it has
     not vouched for — and the `typeof` line inside each is the whole reason
     the function exists. Typing the parameter `number` would move the refusal
     to the caller and leave the guard below unreachable. */

  /** @param {unknown} v */
  function inr(v) {
    if (typeof v !== 'number' || !Number.isFinite(v)) return null;
    const neg = v < 0;
    const s = String(Math.round(Math.abs(v)));
    if (s.length <= 3) return (neg ? '-' : '') + s;
    const last3 = s.slice(-3);
    const rest = s.slice(0, -3).replace(/\B(?=(\d{2})+(?!\d))/g, ',');
    return (neg ? '-' : '') + rest + ',' + last3;
  }
  /**
   * The same, as a string, for the places that must be one (title=, sentences).
   *
   * `dash` is OPTIONAL rather than defaulted in the signature, because most
   * callers want the em-dash and the handful that want their own word say so.
   *
   * @param {unknown} v
   * @param {string} [dash]
   */
  const inrs = (v, dash) => {
    const t = inr(v);
    return t === null ? (dash ?? '—') : t;
  };
  /** The default sentence on an unnamed unknown. Every dash carries one. */
  const NO_NUMBER = 'this page was not given a number here, and will not invent one';

  /* ======================================================================
     MONTHS — a literal three-letter table, never `Intl`.

     `en-IN` and `en-GB` both render September as "Sept" — four letters where
     every other month is three, which breaks a monospace column on one row a
     year. The KEY form (`YYYY-MM`) is what every comparison, every `{#each}`
     key and every request carries; the label is only ever a text node.
     ====================================================================== */

  const MON = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const MKEY = /^(\d{4})-(0[1-9]|1[0-2])$/;
  const DKEY = /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$/;

  /**
   * The API publishes civil-day bounds; the coverage table groups those days
   * by their stored month. Validate a whole date before projecting it, so an
   * impossible day or a timestamp cannot become a plausible month silently.
   * The raw target stays unchanged for day-order checks and inspection.
   * @param {unknown} value
   * @returns {string | null}
   */
  function monthKey(value) {
    if (typeof value !== 'string') return null;
    if (value.length === 7 && MKEY.test(value)) return value;
    if (value.length !== 10) return null;
    const d = DKEY.exec(value);
    if (!d) return null;
    const year = Number(d[1]);
    if (year === 0) return null;
    const month = Number(d[2]);
    const day = Number(d[3]);
    const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
    const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    return day <= days[month - 1] ? value.slice(0, 7) : null;
  }

  /**
   * A valid month key or civil date -> `Apr 2024`; invalid input is null.
   * @param {unknown} k
   */
  function monthLabel(k) {
    const key = monthKey(k);
    return key ? `${MON[Number(key.slice(5)) - 1]} ${key.slice(0, 4)}` : null;
  }
  /**
   * `YYYY-MM-DD` -> `11 Aug 2026`.
   * @param {unknown} k
   */
  function dayLabel(k) {
    const m = typeof k === 'string' ? DKEY.exec(k) : null;
    return m ? `${m[3]} ${MON[+m[2] - 1]} ${m[1]}` : null;
  }

  /**
   * A duration the SERVER measured, plus local drift bounded by the poll.
   * @param {unknown} ms
   */
  function clock(ms) {
    if (typeof ms !== 'number' || !Number.isFinite(ms) || ms < 0) return null;
    const s = Math.floor(ms / 1000);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const r = s % 60;
    const p2 = (/** @type {number} */ x) => String(x).padStart(2, '0');
    return h ? `${h}:${p2(m)}:${p2(r)}` : `${p2(m)}:${p2(r)}`;
  }
  /**
   * IST, always, and it says IST. An operator abroad should not do arithmetic.
   * @param {unknown} epoch
   */
  function istTime(epoch) {
    if (typeof epoch !== 'number' || !Number.isFinite(epoch) || epoch <= 0) return null;
    const d = new Date(epoch + 19800000);
    const p2 = (/** @type {number} */ x) => String(x).padStart(2, '0');
    return `${p2(d.getUTCHours())}:${p2(d.getUTCMinutes())}:${p2(d.getUTCSeconds())}`;
  }
  const ago = (/** @type {number} */ ms) =>
    ms < 1500 ? 'just now' : ms < 60000 ? `${Math.floor(ms / 1000)}s ago` : `${Math.floor(ms / 60000)}m ago`;

  /**
   * THREE-WAY, ALWAYS. Returning 1 for equal makes a sort unstable and a
   * "did this change" test lie; it has reappeared three times in this
   * repository and it does not appear here.
   *
   * Both axes it is asked to order are ordered by `<`: month keys, which are
   * strings chosen so that lexical order IS chronological order, and the
   * original index of a failure, which is a number. One signature covers
   * both, and nothing else is ever passed to it.
   *
   * @param {string | number} a
   * @param {string | number} b
   */
  const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

  /**
   * The feed's own name, from `/feeds.json`. Nothing here names a vendor.
   * @param {string | null | undefined} w
   */
  const feedName = (w) => (feeds.all ?? []).find((f) => f.wire === w)?.display ?? (w ? String(w) : null);

  /* ======================================================================
     THE SPAN — the single reason 1,200 unreadable rows can no longer be drawn.

     The ladder this replaces walked a guarded loop from `from` to `to` and
     checked only two of the four numbers it had parsed:

         if (!Number.isFinite(fy) || !Number.isFinite(tm)) return out;

     `ty` and `fm` were never checked, so a `to` of "20x6-13" left `ty` NaN,
     every `y > ty` comparison false, and the loop ran to its 1,200 ceiling —
     1,200 rows of months the payload never named, in one unreadable block.

     Both endpoints accept the API's civil dates or the legacy month keys.
     Validation precedes month projection; failures retain the original value.
     There is no path from a bad target to a row.
     ====================================================================== */

  /** Fifty years. A target wider than this is a typo, not a backfill. */
  const MAX_SPAN = 600;

  /**
   * `null` IS ONE OF THE INPUTS, not an accident of the signature. `target.from`
   * is `string | null` after the reader, and the whole job of this function is
   * to say which endpoint was not a date or month key — so it must be reachable with
   * the value that is not one.
   *
   * The declared return type is what makes `if (!sp.ok) return …` narrow at
   * every caller: inferred from the returns alone, `ok` widens to `boolean`,
   * the four object shapes merge, and `sp.months` becomes possibly-undefined
   * for every reader of a span that has already been proven good.
   *
   * @param {string | null} from
   * @param {string | null} to
   * @returns {SpanResult}
   */
  function span(from, to) {
    const f = monthKey(from);
    if (!f)
      return {
        ok: false,
        field: 'target.from',
        raw: from,
        why: 'expected a valid calendar date (YYYY-MM-DD) or month key (YYYY-MM).'
      };
    const t = monthKey(to);
    if (!t)
      return {
        ok: false,
        field: 'target.to',
        raw: to,
        why: 'expected a valid calendar date (YYYY-MM-DD) or month key (YYYY-MM).'
      };
    // Projecting first must not hide reversed days inside the same month.
    // A legacy month endpoint denotes its whole month, so compare exact days
    // only when both endpoints carry them.
    if (cmp(f, t) > 0 || (from?.length === 10 && to?.length === 10 && cmp(from, to) > 0))
      return {
        ok: false,
        field: 'target.from … target.to',
        raw: `${from} → ${to}`,
        why: 'the span ends before it begins. The backfill runs oldest first, so the order is not a preference.'
      };
    const n = (Number(t.slice(0, 4)) - Number(f.slice(0, 4))) * 12
      + Number(t.slice(5)) - Number(f.slice(5)) + 1;
    if (n > MAX_SPAN)
      return {
        ok: false,
        field: 'target.from … target.to',
        raw: `${from} → ${to}`,
        why: `that is ${inrs(n)} month files. The ladder is not drawn for a span this wide — read the number, not ${inrs(n)} rows.`
      };
    const out = [];
    let y = Number(f.slice(0, 4));
    let m = Number(f.slice(5));
    for (let i = 0; i < n; i += 1) {
      out.push(`${String(y).padStart(4, '0')}-${String(m).padStart(2, '0')}`);
      m += 1;
      if (m > 12) {
        m = 1;
        y += 1;
      }
    }
    return { ok: true, months: out };
  }

  // Neither the target payload nor the store census supplies expected session
  // counts. An exchange calendar must not be invented from observed bars.
  const BARS_NO_DENOM =
    'expected trading sessions are not supplied by the target or census; bar completeness is unverified';

  /* ======================================================================
     THE CONTRACT READER
     ----------------------------------------------------------------------
     Strict by construction. Every refusal below names the field, because "the
     autopilot looks wrong" is not a bug report and "the payload has no
     `target.from`" is.
     ====================================================================== */

  /** The states the contract defines. Anything else is a contract violation. */
  /** @type {ApState[]} */
  const STATES = ['starting', 'running', 'waiting', 'paused', 'halted', 'complete'];

  /** States in which a REASON is mandatory. A stop with no why is banned. */
  const MUST_EXPLAIN = ['paused', 'halted'];

  /** @param {unknown} v */
  function str(v) {
    return typeof v === 'string' && v.length > 0 ? v : null;
  }
  /** @param {unknown} v */
  function num(v) {
    return typeof v === 'number' && Number.isFinite(v) ? v : null;
  }

  /**
   * Validate one payload. Returns `{ ok: true, value }` or `{ ok: false, why }`.
   *
   * Deliberately not a schema library: the shape is nine fields and a list, and
   * a dependency that turns a named refusal into `/target/from: required` is a
   * worse message for the person who has to fix it.
   *
   * THE INPUT IS `unknown` AND STAYS UNTRUSTED. It arrives from `r.json()` and
   * from the control's embedded `status`, and neither is a promise about a
   * shape — writing a parameter type for the raw body would hand this function
   * the very assumption it exists to check. What the guard below establishes is
   * that it is a non-null, non-array object, and `Record<string, unknown>` is
   * the widest thing that is then TRUE: every field read off it is still
   * `unknown` and still has to go through `str` or `num` before it is believed.
   *
   * @param {unknown} input
   * @returns {ReadResult}
   */
  function readState(input) {
    if (input === null || typeof input !== 'object' || Array.isArray(input)) {
      return { ok: false, why: 'the body is not a JSON object' };
    }
    const raw = /** @type {Record<string, unknown>} */ (input);
    const state = str(raw.state);
    if (!state) return { ok: false, why: 'no `state` field' };
    // `Array<ApState>.includes` will not accept a plain `string`, and a plain
    // string is exactly what is being asked about. The list is widened for the
    // question; the ANSWER to it is what licenses the `ApState` at the bottom.
    if (!(/** @type {readonly string[]} */ (STATES)).includes(state)) {
      return { ok: false, why: `\`state\` is "${state}", which is not one of ${STATES.join(', ')}` };
    }
    const why = str(raw.why);
    if (MUST_EXPLAIN.includes(state) && !why) {
      return {
        ok: false,
        why: `\`state\` is "${state}" and \`why\` is empty — a stop that does not name its reason is the failure CLAUDE.md §4 bans`
      };
    }

    const raw_target = raw.target;
    if (raw_target === null || typeof raw_target !== 'object')
      return { ok: false, why: 'no `target` object' };
    const t = /** @type {Record<string, unknown>} */ (raw_target);
    const target = {
      from: str(t.from),
      to: str(t.to),
      instruments: num(t.instruments),
      timeframe: str(t.timeframe),
      feed: str(t.feed)
    };
    // THE LIST IS THE KEYS, not strings that look like them: a plain `string`
    // cannot index `target`, and the day a field is renamed the checker should
    // report THIS line rather than let a loop look for a key that is gone.
    for (const k of /** @type {('from' | 'to' | 'instruments')[]} */ (['from', 'to', 'instruments'])) {
      if (target[k] === null) return { ok: false, why: `no \`target.${k}\`` };
    }
    if (!Number.isSafeInteger(target.instruments) || /** @type {number} */ (target.instruments) < 0)
      return { ok: false, why: '`target.instruments` must be an exact non-negative integer' };

    // `now` is null when nothing is in flight, and that is a legal answer —
    // but only for a state that is not running. A "running" autopilot with
    // nothing in flight is exactly the hang this page exists to expose.
    /** @type {Flight | null} */
    let flight = null;
    if (raw.now !== null && raw.now !== undefined) {
      const raw_now = raw.now;
      if (typeof raw_now !== 'object') return { ok: false, why: '`now` is neither null nor an object' };
      const n = /** @type {Record<string, unknown>} */ (raw_now);
      // `elapsed_ms` IS A DURATION MEASURED BY THE SERVER, not a timestamp.
      // A start instant would have to be compared against the browser's clock,
      // and two clocks that disagree by a minute render a cell that has been
      // running for "-00:47". A duration the server measured has no such term.
      flight = {
        instrument: str(n.instrument),
        month: str(n.month),
        timeframe: str(n.timeframe),
        feed: str(n.feed),
        elapsed_ms: num(n.elapsed_ms),
        index: num(n.index),
        of: num(n.of)
      };
      for (const k of /** @type {('instrument' | 'month')[]} */ (['instrument', 'month'])) {
        if (flight[k] === null) return { ok: false, why: `no \`now.${k}\`` };
      }
    }
    if (state === 'running' && flight === null) {
      return {
        ok: false,
        why: '`state` is "running" and `now` is null — nothing is in flight, so it is not running'
      };
    }

    // `failures` IS MANDATORY, and defaulting it to `[]` was the one fallback
    // that had crept into this reader. A payload with no `failures` key would
    // have rendered "Nothing has failed" — the exact sentence CLAUDE.md §4
    // bans. A missing list and an empty list are different facts; only the
    // second is good news.
    if (!Array.isArray(raw.failures)) {
      return {
        ok: false,
        why: 'no `failures` array — an absent failure list rendered as "none" is a fallback that hides a failure'
      };
    }
    return {
      ok: true,
      value: {
        // ONE OF THE SIX, PROVEN BY THE MEMBERSHIP TEST AT THE TOP. `str`
        // hands back a `string` because that is all it can know; the check
        // against `STATES` is what narrows it, and the checker cannot carry
        // that result through a widened `includes`, so it is stated here.
        state: /** @type {ApState} */ (state),
        why,
        cursor: str(raw.cursor),
        target,
        now: flight,
        waiting_ms: num(raw.waiting_ms),
        absorbed_ms: num(raw.absorbed_ms),
        journal: str(raw.journal),
        // `month`, `why` and `at` STAY NULL WHEN THEY ARE ABSENT. Substituting
        // a sentence here would make a malformed payload indistinguishable
        // from a well-formed one; the snag rows name each absence instead, on
        // the row it happened to.
        failures: raw.failures.map((f) => ({
          instrument: str(f?.instrument) ?? '(unnamed)',
          month: str(f?.month),
          why: str(f?.why),
          at: str(f?.at)
        }))
      }
    };
  }

  /* ======================================================================
     OBSERVED BY THIS PAGE — the third source, and the only one the browser
     can honestly claim. It is what answers "what happened while I was away".
     ====================================================================== */

  /** @type {{ t: number, text: string }[]} */
  let trail = $state([]);
  const watchAt = Date.now();

  /** @param {string} text */
  function note(text) {
    trail = [{ t: Date.now(), text }, ...trail].slice(0, 60);
  }

  /* ======================================================================
     POLL 1 — the live state
     ====================================================================== */

  // `link` is how this page reaches the server, and it is four states rather
  // than a boolean because they need four different sentences:
  //   probing  — first read has not landed
  //   ok       — a valid payload
  //   absent   — 404. The binary has no autopilot.
  //   broken   — it answered, and the answer was not the contract.
  /** @type {Link} */
  let link = $state({ kind: 'probing', why: null, at: 0, ms: 0 });
  /**
   * THE VALIDATED PAYLOAD, OR NOTHING. Null is not "empty" — it is every one
   * of the four link states that is not `ok`, and the difference is the whole
   * page: an absent autopilot and an idle one must not render the same.
   *
   * THE CAST IS ON THE INITIAL VALUE AND NOT ONLY ON THE DECLARATION, because
   * the two say different things to the checker. A `@type` alone declares the
   * union, and then flow analysis observes that the value assigned right here
   * is `null` and that nothing at the top level assigns it again — every
   * `ap?.…` in a `$derived(…)` below is read as `null` narrowed by `?.`, which
   * is `never`, and the field lookups on it are reported one per use. Every
   * write to `ap` happens inside `adopt` and `tick`, which flow analysis does
   * not follow. Stating the union on the value itself is what says the
   * variable HOLDS the union rather than merely permitting it.
   *
   * @type {Autopilot | null}
   */
  let ap = $state(/** @type {Autopilot | null} */ (null));
  let now = $state(Date.now());

  /**
   * Adopt a validated payload, and note what moved since the last one.
   * @param {Autopilot} next
   */
  function adopt(next) {
    const prev = ap;
    if (!prev) {
      note(`the first read of /autopilot.json landed — state ${next.state}.`);
    } else {
      if (prev.state !== next.state) {
        note(`state moved from ${prev.state} to ${next.state}${next.why ? ` — ${next.why}` : '.'}`);
      }
      if (prev.cursor !== next.cursor && next.cursor) {
        const was = monthLabel(prev.cursor) ?? prev.cursor ?? 'nothing';
        note(`the cursor moved from ${was} to ${monthLabel(next.cursor) ?? next.cursor}.`);
      }
      const grew = next.failures.length - prev.failures.length;
      if (grew > 0) {
        const fresh = next.failures.slice(0, Math.min(grew, 3));
        for (const f of fresh) {
          const when = monthLabel(f.month) ?? f.month ?? 'no month';
          const cause = f.why ?? 'the payload carried no reason, which is itself the bug';
          note(`${f.instrument} · ${when} stalled — ${cause}`);
        }
        if (grew > fresh.length) {
          note(`${inrs(grew - fresh.length)} further failure(s) were reported in the same round.`);
        }
      }
    }
    ap = next;
  }

  /**
   * Move `link`, and say so once, when the kind actually changes.
   * @param {Link} next
   */
  function setLink(next) {
    if (link.kind !== next.kind) {
      note(
        next.kind === 'ok'
          ? '/autopilot.json answered the contract.'
          : next.kind === 'absent'
            ? 'GET /autopilot.json answered 404 — this binary has no autopilot route.'
            : next.kind === 'broken'
              ? `/autopilot.json stopped being readable — ${next.why}`
              : 'probing /autopilot.json.'
      );
    }
    link = next;
  }

  /** @param {import('$lib/page-requests.js').ReadTicket} ticket */
  async function tick(ticket) {
    const t0 = performance.now();
    try {
      const r = await ask('/autopilot.json', { cache: 'no-store', signal: ticket.signal });
      if (!ticket.current()) return;
      const ms = Math.round(performance.now() - t0);
      if (r.status === 404) {
        setLink({
          kind: 'absent',
          why: 'GET /autopilot.json answered 404 — this binary has no autopilot route',
          at: Date.now(),
          ms
        });
        ap = null;
        return;
      }
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      // THE CONTENT-TYPE CHECK IS NOT PEDANTRY. In development the Vite proxy
      // forwards a fixed list of routes; a route missing from that list is
      // answered by the dev server's own HTML fallback with a 200. `r.ok`
      // cannot see it, and a green light wired to an HTML page is worse than
      // a red one.
      const ct = r.headers.get('content-type') ?? '';
      if (!ct.includes('json')) {
        throw new Error(
          `answered ${ct || 'no content-type'}, not JSON — the API is not behind this route (in development, add /autopilot.json to the proxy list in web/vite.config.js)`
        );
      }
      const parsed = readState(await r.json());
      if (!ticket.current()) return;
      if (!parsed.ok) {
        setLink({
          kind: 'broken',
          why: `the autopilot answered, and the payload does not match the contract: ${parsed.why}`,
          at: Date.now(),
          ms
        });
        return;
      }
      adopt(parsed.value);
      setLink({ kind: 'ok', why: null, at: Date.now(), ms });
    } catch (e) {
      if (!ticket.current()) return;
      // A CAUGHT VALUE IS `unknown`, AND THIS LINE ALREADY KNEW THAT. Both
      // throws above are `Error`s, but a rejected `fetch` can settle with
      // anything at all, which is why the reason is read off the value with
      // `?.` and falls back to the value itself. The cast says only what those
      // two operators already assume — that a `message` MAY be there — rather
      // than asserting a class the runtime has not checked.
      const cause = /** @type {{ message?: unknown } | null | undefined} */ (e);
      setLink({
        kind: 'broken',
        why: String(cause?.message ?? e),
        at: Date.now(),
        ms: Math.round(performance.now() - t0)
      });
    }
  }

  // The delay starts after a read completes. Hidden pages abort their read;
  // a late response cannot publish or restart a poll after navigation.
  $effect(() => watchVisible(tick, TICK_MS, {
    visible: () => document.visibilityState === 'visible',
    listen: (wake) => {
      document.addEventListener('visibilitychange', wake);
      return () => document.removeEventListener('visibilitychange', wake);
    }
  }));

  // ONE SECOND, ALWAYS. Three readings on this page are durations rather than
  // values — the cell in flight, how long this tab has watched, and how stale
  // the census read is — and a duration that only moves when a poll lands is a
  // clock that is wrong between polls.
  $effect(() => {
    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });

  /* ======================================================================
     POLL 2 — the census, which is the truth
     ----------------------------------------------------------------------
     One fetch, then ONE PASS folding cells into per-month totals. The fold is
     O(cells returned) and happens on arrival, not on render: a derived value
     recomputed per keystroke over 60,000 cells is a page that stutters for a
     number that changed thirty seconds ago.

     IT IS STAMPED WITH THE FEED IT ANSWERED FOR. Without that stamp one feed's
     numbers render under another feed's name for a whole poll period after the
     selection moves — a stale value shown as current, which CLAUDE.md §4 bans.
     `scope` below is the only reader of that stamp, and the beam refuses a
     fraction whenever it disagrees with the target.
     ====================================================================== */

  // The feed is part of the question: the brokers do not hold the same
  // instruments, so a census is per feed and re-reads when the selection moves.
  // THE SELECTION ITSELF IS THE TOP BAR'S, and this page never writes it — one
  // feed picker per product, in `+layout.svelte`, and no second one here. The
  // autopilot's OWN target wins when it names one, because a fraction of a
  // target has to be counted out of the store that target is being written to.
  // Primitive scope values stay equal across status polls. The 2-second
  // status reader must not refold an unchanged census for a new target object.
  const targetFeed = $derived(ap?.target?.feed ?? null);
  const targetTimeframe = $derived(ap?.target?.timeframe ?? null);
  const targetFrom = $derived(ap?.target?.from ?? null);
  const targetTo = $derived(ap?.target?.to ?? null);
  const wire = $derived(targetFeed ?? feeds.active);

  /* THE FETCH, THE FOLD AND THE TIMER ARE ALL SHARED NOW.
     ----------------------------------------------------------------------
     This page used to own the third of SIX independent reads of `/store.json`,
     on the third of six clocks — thirty seconds here, five on `/ingest` during
     a pull, once per feed change on `/db` and `/`. Nothing reconciled them, so
     two surfaces could hold two totals for one disk and each was right about
     its own snapshot. `$lib/store.svelte.js` reads it ONCE per (feed,
     generation). This page projects that same readable census onto the target
     feed, timeframe and window; all-timeframe totals are not target coverage.

     `watchStore` is the thirty seconds, held rather than owned: the shared poll
     runs at the finest period any holder asked for, and every subscriber sees
     the answer it fetches. */
  $effect(() => syncStore(wire));
  $effect(() => watchStore(CENSUS_MS));

  /** @param {string} key */
  function monthLastDay(key) {
    for (const day of ['31', '30', '29', '28']) {
      const value = `${key}-${day}`;
      if (monthKey(value)) return value;
    }
    return null;
  }

  /** Census timestamps are exact microseconds; no unit guessing or rollover.
   * @param {number | null} micros @returns {string | null} */
  function censusDay(micros) {
    if (micros === null || !Number.isSafeInteger(micros) || micros < 0) return null;
    return new Date(Math.floor(micros / 1000) + 19800000).toISOString().slice(0, 10);
  }

  /** One O(readable rows) projection per census or scope change. Distinct
   * instrument-months are counted once after filtering the exact target rung.
   * Partial-month bar counts are withheld unless both stored endpoints lie
   * inside the target days. No bar file or vendor request is made here.
   * @param {CensusInput} snapshot @param {CoverageTarget} target
   * @param {string | null} askedFeed @returns {TargetCensus} */
  function foldTargetCensus(snapshot, target, askedFeed) {
    /** @type {TargetCensus} */
    const out = { kind: 'probing', why: null, at: null, feed: null, months: new Map(),
      cells: 0, bars: 0, unknownCells: 0, unmeasuredBars: 0, duplicates: 0 };
    if (snapshot.state === 'error') return { ...out, kind: 'broken', why: snapshot.error };
    if (!target.feed || !target.timeframe)
      return { ...out, kind: 'unscoped', why: 'The reported target must name a feed and timeframe before stored coverage can be compared.' };
    if (snapshot.state !== 'ready' || snapshot.feed !== askedFeed) return out;
    if (snapshot.feed !== target.feed)
      return { ...out, kind: 'unscoped', why: 'The stored census and the reported target name different feeds.' };
    const sp = span(target.from, target.to);
    if (!sp.ok) return { ...out, kind: 'unscoped', why: `${sp.field}: ${sp.why}` };
    const from = /** @type {string} */ (target.from);
    const to = /** @type {string} */ (target.to);
    const firstDay = from.length === 7 ? `${from}-01` : from;
    const lastDay = to.length === 7 ? monthLastDay(to) : to;
    if (!lastDay) return { ...out, kind: 'unscoped', why: 'The final month has no valid calendar boundary.' };
    const wholeMonths = new Set();
    for (const key of sp.months) {
      out.months.set(key, { cells: 0, bars: 0, unknownCells: 0, unmeasuredBars: 0 });
      const end = monthLastDay(key);
      if (`${key}-01` >= firstDay && end !== null && end <= lastDay) wholeMonths.add(key);
    }
    /** @type {Map<string, Map<string, import('$lib/store.svelte.js').StoreCell>>} */
    const seen = new Map();
    for (const cell of snapshot.readable) {
      const month = out.months.get(cell.month);
      if (!month || cell.timeframe !== target.timeframe || cell.rows === 0) continue;
      let instruments = seen.get(cell.month);
      if (!instruments) { instruments = new Map(); seen.set(cell.month, instruments); }
      const prior = instruments.get(cell.instrument);
      if (prior) {
        if (prior.rows !== cell.rows || prior.first !== cell.first || prior.last !== cell.last)
          return { ...out, kind: 'broken', why: `Conflicting census entries for ${cell.instrument}, ${cell.timeframe}, ${cell.month}; counts are withheld.` };
        out.duplicates += 1;
        continue;
      }
      instruments.set(cell.instrument, cell);
      if (!Number.isSafeInteger(cell.rows) || cell.rows < 0)
        return { ...out, kind: 'broken', why: 'A stored bar count is not an exact non-negative integer.' };
      const first = censusDay(cell.first), last = censusDay(cell.last);
      if ((first !== null && first.slice(0, 7) !== cell.month) || (last !== null && last.slice(0, 7) !== cell.month)
        || (first !== null && last !== null && (first > last || /** @type {number} */ (cell.first) > /** @type {number} */ (cell.last))))
        return { ...out, kind: 'broken', why: `Stored timestamps contradict the census month for ${cell.instrument}, ${cell.month}.` };
      if (wholeMonths.has(cell.month) || (first !== null && last !== null && first >= firstDay && last <= lastDay)) {
        month.cells += 1; month.bars += cell.rows;
        out.cells += 1; out.bars += cell.rows;
        if (!Number.isSafeInteger(out.bars))
          return { ...out, kind: 'broken', why: 'The scoped bar total exceeds exact browser integer precision.' };
        continue;
      }
      if (first !== null && last !== null && (last < firstDay || first > lastDay)) continue;
      // At least one real stored endpoint inside the window proves presence;
      // endpoints bracketing the window do not prove any bar exists inside it.
      if ((first !== null && first >= firstDay && first <= lastDay) || (last !== null && last >= firstDay && last <= lastDay)) {
        month.cells += 1; out.cells += 1;
      } else { month.unknownCells += 1; out.unknownCells += 1; }
      month.unmeasuredBars += 1; out.unmeasuredBars += 1;
    }
    return { ...out, kind: 'ok', at: snapshot.at, feed: snapshot.feed };
  }

  const census = $derived.by(() => {
    const snapshot = { state: store.state, error: store.error, at: store.at, feed: store.feed, readable: store.readable };
    const target = { feed: targetFeed, timeframe: targetTimeframe, from: targetFrom, to: targetTo };
    const askedFeed = wire;
    return untrack(() => foldTargetCensus(snapshot, target, askedFeed));
  });

  /* THE TRAIL STILL SAYS WHAT MOVED. The note used to be written by the reader;
     with the reader shared it is written by whoever is WATCHING the reading,
     which is this page. `store.reads` counts successful reads, so this fires
     once per landed answer and never on a re-render. */
  let notedRead = 0;
  let notedCells = 0;
  let notedBars = 0;
  /** @type {string | null} */
  let notedFeed = null;
  /** @type {string | null} */
  let notedError = null;
  $effect(() => {
    const reads = store.reads;
    const state = store.state;
    const why = store.error;
    untrack(() => {
      if (state === 'error' && why !== null && why !== notedError) {
        notedError = why;
        notedRead = 0;
        note(`/store.json could not be read for ${feedName(wire)} — ${why}`);
        return;
      }
      if (state !== 'ready' || reads === notedRead) return;
      notedError = null;
      const first = notedRead === 0 || notedFeed !== store.feed;
      notedRead = reads;
      if (first) {
        note(
          `the whole-store census was read for ${feedName(store.feed)} — ${inrs(store.cells)} instrument-timeframe-month records and ${inrs(store.bars)} bars, across all stored dates and timeframes.`
        );
      } else if (store.cells !== notedCells || store.bars !== notedBars) {
        note(
          `the whole-store census changed by ${inrs(store.cells - notedCells)} instrument-timeframe-month record(s) and ${inrs(store.bars - notedBars)} bar(s) for ${feedName(store.feed)}, across all stored dates and timeframes.`
        );
      }
      notedFeed = store.feed;
      notedCells = store.cells;
      notedBars = store.bars;
    });
  });

  /* ======================================================================
     DERIVED — and every one of them gated on whether it was actually MEASURED.

     `census.cells` is 0 before the first read lands and 0 again after a refused
     one. Zero is also a legitimate answer. The number alone cannot say which,
     so nothing below reads the number without reading `counted` first.
     ====================================================================== */

  const counted = $derived(census.kind === 'ok');
  /* THREE CAUSES, NOT TWO. This split `broken` from everything else, so every
     other kind fell into "reading /store.json…" — including the state where no
     feed is chosen, in which `syncStore` returns before it reads and NOTHING
     IS IN FLIGHT. Both headline gauges then reported a read that was not
     happening, indefinitely, with no press or wait that could resolve it.

     The refusal to print a zero was always right — an unread month and an
     empty month are different facts, and this page says so at length. The
     CAUSE was the part that lied, which is the same defect one layer along: a
     reason an operator cannot act on because it is not the reason. */
  const notCounted = $derived(
    census.kind === 'broken'
      ? `/store.json could not be read — ${census.why}`
      : census.kind === 'unscoped'
        ? census.why ?? 'The reported target scope is unavailable.'
      : !wire
        ? 'no feed is chosen, so no census has been asked for — choose one in the top bar'
        : 'reading /store.json…'
  );

  /** Is the census answering the same question the target asks? */
  const scope = $derived.by(() => {
    const t = ap?.target;
    if (!t || !t.feed || !counted) return { same: true, why: null };
    if (census.feed === t.feed) return { same: true, why: null };
    return {
      same: false,
      why: `the autopilot is working ${feedName(t.feed)} and this census is ${feedName(census.feed)} — two different questions, so nothing here is a fraction of anything there.`
    };
  });

  /**
   * The ladder, or a named refusal. Never a row whose month is not a month.
   *
   * When the target is known it runs the whole span, so a month with nothing in
   * it is a VISIBLE GAP rather than a row that was never drawn. When it is not
   * known, only the observed months appear — and the page says so, rather than
   * inventing a floor.
   *
   * `refuse` IS THE REFUSAL OR IT IS NULL, and the two arms are keyed on the
   * same literal `ok` that `span` uses, so the panel that prints
   * `ladder.refuse.field` is only reachable on the arm that has one.
   *
   * @type {{ ok: true, refuse: null, months: string[], framed: boolean }
   *      | { ok: false, refuse: SpanRefusal, months: string[], framed: boolean }}
   */
  const ladder = $derived.by(() => {
    const t = ap?.target;
    if (t) {
      const sp = span(t.from, t.to);
      if (!sp.ok) return { ok: false, refuse: sp, months: [], framed: true };
      return { ok: true, refuse: null, months: sp.months, framed: true };
    }
    if (!counted) return { ok: true, refuse: null, months: [], framed: false };
    return { ok: true, refuse: null, months: [...census.months.keys()].sort(cmp), framed: false };
  });

  /**
   * One month's scoped stored observations. A count cannot identify which
   * instruments belong to the target, so it is never used as a denominator.
   * @param {string} key
   * @returns {Rung}
   */
  function rowOf(key) {
    const h = census.months.get(key) ?? { cells: 0, bars: 0, unknownCells: 0, unmeasuredBars: 0 };
    return { key, ...h };
  }

  const rows = $derived(ladder.ok ? ladder.months.map((k) => rowOf(k)) : []);

  const MEMBERSHIP_UNKNOWN = 'The target reports an instrument count but no exact instrument list. Stored records cannot prove that every target instrument is present, so a completion percentage is unavailable.';

  /** What one rung is, as one word. The tape, the buckets and the filter read
      the same call, so a row cannot be counted as one thing and drawn as
      another.
      @param {Rung} r */
  function classOf(r) {
    if (r.cells > 0) return 'nod';
    return r.unknownCells > 0 ? 'part' : 'none';
  }

  /**
   * The partition of the ladder. It must sum, and the foot prints the sum.
   *
   * `nod` is its own bucket: without a denominator a month cannot be called
   * partial any more than it can be called complete, and folding it into `part`
   * is a claim the census did not support.
   */
  const buckets = $derived.by(() => {
    const b = { part: 0, none: 0, nod: 0 };
    for (const r of rows) b[classOf(r)] += 1;
    return b;
  });
  const bucketSum = $derived(buckets.part + buckets.none + buckets.nod);

  /** The headline uses the same scoped fold as its month rows, never a global
   * total or the reported count as a proof of target membership.
   * @param {Target} target @param {TargetCensus} reading @param {Rung[]} months
   * @param {ApState} state @returns {Verdict} */
  function targetCoverageVerdict(target, reading, months, state) {
    const present = months.filter((r) => r.cells > 0).length;
    const empty = months.filter((r) => r.cells === 0 && r.unknownCells === 0).length;
    const uncertain = months.length - present - empty;
    /** @type {Fact[]} */
    const facts = [
      { k: 'distinct stored instrument-months', v: inrs(reading.cells), s: 'mea' },
      { k: 'bars confirmed inside the target dates', v: inrs(reading.bars), s: 'mea' },
      { k: 'months with records · no records · overlap unknown', v: `${inrs(present)} · ${inrs(empty)} · ${inrs(uncertain)}`, s: 'mea' },
      { k: 'target timeframe', v: target.timeframe, s: 'rep' },
      { k: 'reported instrument count (membership unverified)', v: inrs(target.instruments), s: 'rep' },
      { k: 'reported state', v: state, s: 'rep' }
    ];
    if (reading.unmeasuredBars) facts.push({ k: 'boundary files with unmeasured in-range bar counts', v: inrs(reading.unmeasuredBars), s: 'mea' });
    if (reading.unknownCells) facts.push({ k: 'boundary files whose date overlap is unverified', v: inrs(reading.unknownCells), s: 'mea' });
    if (reading.duplicates) facts.push({ k: 'duplicate census entries counted once', v: inrs(reading.duplicates), s: 'mea' });
    return {
      tone: state === 'halted' ? 'bad' : empty > 0 ? 'warn' : 'unknown',
      claim: `${inrs(reading.cells)} distinct stored instrument-months at ${target.timeframe} — ${inrs(empty)} months have no stored records in the target window.`,
      sub: `${target.from} to ${target.to}. ${MEMBERSHIP_UNKNOWN}${reading.unmeasuredBars ? ` Exact bar counts for ${inrs(reading.unmeasuredBars)} boundary files are unmeasured; their whole-file totals are excluded.` : ''}`,
      facts
    };
  }

  /* ======================================================================
     THE CLAIM, AND WHY IT CANNOT BE MADE WITHOUT THE EVIDENCE

     This is the ONLY place on the page that produces a verdict. It is handed
     measured numbers rather than a sentence, and the facts it returns are the
     ones the beam draws underneath the claim — one element, one pass.
     ====================================================================== */

  /** @type {Verdict} */
  const verdict = $derived.by(() => {
    const lk = link;

    if (lk.kind === 'absent' || lk.kind === 'broken') {
      /** @type {Fact[]} */
      const facts = [];
      if (counted) {
        facts.push({ k: 'instrument-months held', v: inrs(census.cells), s: 'mea' });
        facts.push({ k: 'bars held', v: inrs(census.bars), s: 'mea' });
        facts.push({ k: 'census read', v: `${istTime(census.at) ?? '—'} IST`, s: 'mea' });
      } else {
        facts.push({ k: 'the census', v: notCounted, s: 'mea' });
      }
      return {
        tone: 'bad',
        claim:
          lk.kind === 'absent'
            ? 'This process is serving pages, and nothing is driving the pull.'
            : 'The autopilot answered, and the answer cannot be trusted.',
        sub: `${lk.why}. What is in flight, what failed and whether anything is advancing are all unknown. Coverage below is still measured off the store — but a number that is stalled and a number that is finished look identical from here. Until the route answers, the backfill advances only for as long as somebody drives it, oldest month first: a later month written first permanently blocks the earlier days inside that same month-file.`,
        facts
      };
    }

    if (lk.kind === 'probing' || !ap) {
      return {
        tone: 'unknown',
        claim: 'Nothing can be claimed yet.',
        sub: 'The first read of /autopilot.json has not landed. An unread store and an empty store are different facts, so no figure is drawn from either.',
        facts: [
          { k: 'the state', v: 'reading /autopilot.json…', s: 'rep' },
          {
            k: 'the census',
            v: counted ? `${inrs(census.cells)} instrument-months held` : notCounted,
            s: 'mea'
          }
        ]
      };
    }

    if (!scope.same) {
      return {
        tone: 'unknown',
        claim: 'The census and the target are about different feeds.',
        sub: `${scope.why} The ladder is drawn without a denominator until they agree.`,
        facts: [
          { k: 'target feed', v: feedName(ap.target.feed), s: 'rep' },
          { k: 'census feed', v: feedName(census.feed), s: 'mea' },
          { k: 'instrument-months held', v: inrs(census.cells), s: 'mea' }
        ]
      };
    }

    if (!counted) {
      return {
        tone: 'unknown',
        claim: 'How much is held cannot be stated.',
        sub: `${notCounted} The reported state “${ap.state}” does not establish stored target coverage.`,
        facts: [
          { k: 'reported state', v: ap.state, s: 'rep' },
          { k: 'the census', v: notCounted, s: 'mea' }
        ]
      };
    }

    if (!ladder.ok) {
      return {
        tone: 'bad',
        claim: 'The target the autopilot reports is not a span.',
        sub: `${ladder.refuse.field} carries ${JSON.stringify(ladder.refuse.raw)} — ${ladder.refuse.why} The ladder is refused rather than filled with rows nobody can read.`,
        facts: [
          { k: 'instrument-months held', v: inrs(census.cells), s: 'mea' },
          { k: 'bars held', v: inrs(census.bars), s: 'mea' },
          { k: 'reported state', v: ap.state, s: 'rep' }
        ]
      };
    }

    return targetCoverageVerdict(ap.target, census, rows, ap.state);
  });

  /**
   * THE GUARD. A claim with nothing under it is not renderable here: the beam
   * takes this list, and when it is empty it prints the refusal instead of the
   * claim. A completeness claim over an unread store cannot reach the screen,
   * because the claim and its evidence are one element.
   */
  const evidence = $derived(
    (verdict.facts ?? []).filter((f) => f && f.v !== null && f.v !== undefined && f.v !== '')
  );

  /* ======================================================================
     THE CONTROL, AND ITS RECEIPT

     THERE IS NO SUCCESS TEMPLATE ON THIS PAGE. `send` prints the server's own
     sentence, verbatim, in mono, and takes its TONE from that sentence — so
     "started, and it is NOT a full recovery: 1 of the feeds below is terminal
     for the life of this process and this did not clear it" cannot be rendered
     as a plain green "resume accepted", which is what the control this replaces
     did with it. The three tones are done / partial / refused, and the middle
     one exists precisely because the server has an answer that is accepted and
     incomplete at the same time.

     ONE ROUTE, `POST /autopilot/control`, because it is the one that answers
     `{action, accepted, why, status}` for BOTH words. `/autopilot/pause`
     answers a bare status with no sentence, and a receipt with no sentence
     would have to be written here — which is the success template again.
     ====================================================================== */

  let control = $state({ busy: false });
  /** @type {Receipt | null} */
  let receipt = $state(null);

  /**
   * The tone comes from the SENTENCE, never from the status code alone.
   * @param {boolean} accepted
   * @param {string} why
   * @returns {'done' | 'partial' | 'refused'}
   */
  const classify = (accepted, why) =>
    !accepted ? 'refused' : /NOT a full recovery|terminal|could not|poisoned/.test(why) ? 'partial' : 'done';

  /** @param {string} action */
  async function send(action) {
    control = { busy: true };
    let code = 0;
    try {
      const r = await ask(CONTROL, {
        method: 'POST',
        headers: { 'content-type': 'application/x-www-form-urlencoded' },
        body: `action=${encodeURIComponent(action)}`
      });
      code = r.status;
      if (r.status === 404) {
        throw new Error(`POST ${CONTROL} answered 404 — this binary has no autopilot control`);
      }
      const ct = r.headers.get('content-type') ?? '';
      if (!ct.includes('json')) {
        throw new Error(
          `POST ${CONTROL} answered ${ct || 'no content-type'}, not JSON — the API is not behind this route (in development, add ${CONTROL} to the proxy list in web/vite.config.js)`
        );
      }
      const body = await r.json();
      const accepted = body?.accepted === true;
      const why = str(body?.why);
      if (why === null) {
        // A CONTROL THAT ANSWERED WITHOUT A SENTENCE. Refused rather than
        // dressed up: the sentence is the only thing that says what happened,
        // and writing one here is the success template this page bans.
        throw new Error(
          `POST ${CONTROL} answered HTTP ${r.status} with no \`why\` — the control's own answer carries the reason, and this one carried none`
        );
      }
      const tone = classify(accepted, why);
      receipt = { action, tone, code: r.status, why, at: Date.now() };
      note(
        `POST ${CONTROL} action=${action} answered ${r.status} — ${
          tone === 'partial'
            ? 'accepted, and NOT a full recovery'
            : tone === 'refused'
              ? 'refused'
              : 'accepted'
        }.`
      );

      // THE ANSWER EMBEDS THE STATUS, so the page re-reads from that rather
      // than fetching again and racing a round that landed in between.
      const parsed = readState(body?.status);
      if (parsed.ok) {
        adopt(parsed.value);
        link = { kind: 'ok', why: null, at: Date.now(), ms: link.ms };
      }
    } catch (e) {
      // A control that failed and said nothing is a button that lies. The
      // reason goes on the page beside the button that produced it, in the
      // same element the server's own sentence would have used.
      //
      // Read by SHAPE and not by class, for the reason `tick` gives: a
      // rejected `fetch` is not required to settle with an `Error`.
      const cause = /** @type {{ message?: unknown } | null | undefined} */ (e);
      const why = String(cause?.message ?? e);
      // `unknown` AND NOT `refused`, WHICH IS THE WHOLE OF THIS BRANCH.
      //
      // A refusal is a claim about what the SERVER did. Reaching here is a
      // claim about what THIS PAGE knows: the request may have arrived, been
      // acted on, and had its answer lost on the way back. Writing `refused`
      // stated the stronger fact, and the 2-second poll below then adopted a
      // status showing the autopilot RUNNING -- so the page displayed "refused"
      // and "running" at once, with nothing reconciling them and no way for an
      // operator to tell which was true.
      //
      // The tone the page can defend is that it does not know. The status deck
      // is the authority and says so in the receipt.
      receipt = { action, tone: 'unknown', code, why, at: Date.now() };
      note(`POST ${CONTROL} action=${action} did not complete — ${why}`);
    } finally {
      control = { busy: false };
    }
  }

  /** Which button is offered, what it does, and why — one place, four answers. */
  const offer = $derived.by(() => {
    if (!ap) return { label: null, action: null, go: false, why: 'No control: /autopilot.json did not answer.' };
    if (ap.state === 'paused') {
      return {
        label: 'Resume',
        action: 'resume',
        go: true,
        why: 'Resumes from the store’s census, not a progress variable.'
      };
    }
    if (ap.state === 'halted') {
      return {
        label: 'Retry',
        action: 'resume',
        go: true,
        why: 'It halted rather than skipping — nothing was passed over. Fix the reason above first.'
      };
    }
    if (ap.state === 'complete') {
      return {
        label: null,
        action: null,
        go: false,
        why: 'Nothing to pause. Whether the store agrees is the beam to the left.'
      };
    }
    return {
      label: 'Pause',
      action: 'stop',
      go: false,
      why: 'Finishes the cell in flight, then stops. Never aborts mid-write.'
    };
  });

  /**
   * How long the current cell has been running.
   *
   * The server's own measurement, plus the local drift since that measurement
   * arrived. Only the drift is the browser's guess, and it is bounded by the
   * poll period — so the number ticks smoothly between polls without ever being
   * derived from a comparison of two machines' clocks.
   */
  const elapsed = $derived(
    ap?.now && ap.now.elapsed_ms !== null ? ap.now.elapsed_ms + Math.max(0, now - link.at) : null
  );

  /** Seconds of the grace window left, by this page's own clock. */
  const graceLeft = $derived(Math.max(0, GRACE_SECS - Math.floor((now - watchAt) / 1000)));

  /* ======================================================================
     THE DIALS — this page's one control primitive.

     A dial shows its CURRENT VALUE on its face. It is not a row of links and it
     is not a segmented bar: there are three to fourteen answers behind each one
     and the face has to survive all of them.
     ====================================================================== */

  /** Which dial is open, by its id — `null` is "none of them". */
  /** @type {string | null} */
  let drop = $state(null);
  let show = $state('all');
  let group = $state('month');
  /** The month key of the expanded rung, or `null`. */
  /** @type {string | null} */
  let openRung = $state(null);

  /** @type {DialOption[]} */
  const SHOW = [
    { k: 'all', n: 'every month', y: 'the whole target span, gaps included' },
    { k: 'gap', n: 'without confirmed records', y: 'empty months or boundary months whose date overlap is unknown' },
    { k: 'done', n: 'with stored records', y: 'at least one distinct stored instrument has a record in the target window; completeness is unverified' },
    // OFFERED AND DEAD, WITH THE REASON ON ITS FACE. Dropping the row would say
    // the question cannot be asked; drawing it live would promise a filter that
    // can never match. CLAUDE.md §4: name the reason on the control it is about.
    { k: 'short', n: 'only bars short', y: BARS_NO_DENOM, off: true }
  ];
  /** @type {DialOption[]} */
  const GROUPS = [
    { k: 'month', n: 'by month', y: 'oldest first — the order the backfill needs them' },
    { k: 'instrument', n: 'by instrument', y: 'one heading per symbol that is stalled' },
    { k: 'reason', n: 'by reason', y: 'the vendor code, or the store refusal' }
  ];

  const shown = $derived(
    rows.filter((r) => {
      const c = classOf(r);
      if (show === 'gap') return c !== 'nod';
      if (show === 'done') return c === 'nod';
      if (show === 'short') return false;
      return true;
    })
  );

  /**
   * A pointerdown anywhere that is not inside a dial closes the open one.
   *
   * `e.target` is an `EventTarget`, and an `EventTarget` is not necessarily an
   * `Element` — a pointerdown can be delivered to the document itself, which
   * is precisely why the call below is written `?.closest?.(…)` and not
   * `.closest(…)`. The cast is to THAT shape, an optional `closest`, rather
   * than to `Element`: asserting the element would be claiming the thing the
   * two question marks are there to doubt.
   *
   * @param {PointerEvent} e
   */
  function onWindowDown(e) {
    const hit = /** @type {{ closest?: (selector: string) => unknown } | null} */ (e.target);
    if (drop && !hit?.closest?.('.dial')) drop = null;
  }
  /** @param {KeyboardEvent} e */
  function onWindowKey(e) {
    if (e.key === 'Escape' && drop) drop = null;
  }

  /* ======================================================================
     THE SNAGS — what is stuck. A failure is never a count on its own.
     ====================================================================== */

  /** @param {Failure} f */
  function reasonOf(f) {
    if (!f.why) return 'no reason in the payload';
    const m = /^([A-Z]{2}-\d{3})/.exec(f.why);
    if (m) return m[1];
    return f.why.startsWith('the store refused') ? 'store refused the batch' : 'transport';
  }
  /** @param {Failure} f */
  const groupKey = (f) =>
    group === 'month'
      ? (monthLabel(f.month) ?? f.month ?? 'no month')
      : group === 'instrument'
        ? f.instrument
        : reasonOf(f);
  /** THE SORT KEY IS THE RAW MONTH, so the order is chronological and not
      alphabetical — "Apr 2021" before "Feb 2020" is what the label sorts to.
      @param {Failure} f */
  const sortKey = (f) => (group === 'month' ? (f.month ?? '') : groupKey(f));

  const snags = $derived(
    (ap?.failures ?? [])
      .map((f, i) => ({ ...f, i }))
      .sort((a, b) => cmp(sortKey(a), sortKey(b)) || cmp(a.i, b.i))
  );
</script>

<svelte:window onkeydown={onWindowKey} onpointerdown={onWindowDown} />

<!-- =====================================================================
     THE HELPERS. Every number, every month and every provenance chip on this
     page goes through one of these three, so an unknown is NAMED and never
     blanked, and a month is never rendered from anything but its key.
     ===================================================================== -->

<!-- `why` IS OPTIONAL AND SAYS SO IN THE SIGNATURE. Most callers have no
     sentence of their own and want the default one, which the body already
     supplies with `why ?? NO_NUMBER`; the explicit `= undefined` changes
     nothing at run time and tells the checker that a one-argument render is
     the intended call and not a missing argument. -->
{#snippet N(/** @type {unknown} */ v, /** @type {string | undefined} */ why = undefined)}
  {#if inr(v) !== null}
    <span class="mono">{inr(v)}</span>
  {:else}
    <span class="unk" title={why ?? NO_NUMBER}>—</span>
  {/if}
{/snippet}

{#snippet monthNode(/** @type {string | null} */ k)}
  {#if monthLabel(k)}
    <span class="mono" title={k}>{monthLabel(k)}</span>
  {:else}
    <span class="unk" title="not a month key this page can read — the store writes YYYY-MM"
      >{k === null || k === undefined ? 'no month' : String(k)}</span
    >
  {/if}
{/snippet}

{#snippet src(/** @type {'mea' | 'rep' | 'obs'} */ kind)}
  <span
    class="src"
    class:mea={kind === 'mea'}
    class:rep={kind === 'rep'}
    class:obs={kind === 'obs'}
    title={kind === 'mea'
      ? 'Measured off disk by /store.json. True whether or not the autopilot exists.'
      : kind === 'rep'
        ? 'Reported by /autopilot.json. This is what the process says about itself — a claim, not evidence.'
        : 'Observed by this page across successive polls. Nothing else knows it, and it dies with this tab.'}
    >{kind === 'mea' ? 'measured' : kind === 'rep' ? 'reported' : 'observed'}</span
  >
{/snippet}

{#snippet dial(
  /** @type {string} */ id,
  /** @type {string} */ label,
  /** @type {string} */ value,
  /** @type {DialOption[]} */ opts,
  /** @type {(k: string) => void} */ pick
)}
  {@const cur = opts.find((o) => o.k === value) ?? { n: String(value), y: '' }}
  <span class="dial" class:open={drop === id}>
    <button
      class="dial-face"
      type="button"
      aria-haspopup="listbox"
      aria-expanded={drop === id}
      title="{label}: {cur.n}{cur.y ? ` — ${cur.y}` : ''}"
      onclick={(e) => {
        e.stopPropagation();
        drop = drop === id ? null : id;
      }}
    >
      <span class="dl">{label}</span>
      <span class="dv">{cur.n}</span>
      <span class="dc" aria-hidden="true"></span>
    </button>
    {#if drop === id}
      <div class="dial-list" role="listbox" aria-label={label}>
        {#each opts as o (o.k)}
          <button
            class="dial-opt"
            type="button"
            role="option"
            aria-selected={o.k === value}
            disabled={Boolean(o.off)}
            title={o.y}
            onclick={() => {
              drop = null;
              pick(o.k);
            }}
          >
            <span class="on">{o.n}</span>
            <span class="oy">{o.y}</span>
          </button>
        {/each}
      </div>
    {/if}
  </span>
{/snippet}

<div class="pane autopilot">
  <div class="pane-head">
    <span class="pane-title">Autopilot</span>

    <span class="spacer"></span>

    <!-- THE LINK, ON THE PAGE'S OWN HEAD, AT THE RIGHT END OF THE BAR. Two
         chips, both OBSERVED by this page and by nothing else: whether it is
         being answered, and how long this tab has been the one watching.
         THE ROUND TRIP IS NOT HERE. It is the deck's sixth gauge, where it
         carries its `observed` chip — a millisecond count wearing no
         provenance beside a state that came off the wire is the merge of two
         sources this page exists to keep apart. -->
    <span
      class="chip"
      class:up={link.kind === 'ok'}
      class:am={link.kind === 'probing'}
      class:dn={link.kind === 'absent' || link.kind === 'broken'}
      title={link.why ?? `GET /autopilot.json every ${TICK_MS / 1000}s; GET /store.json every ${CENSUS_MS / 1000}s.`}
    >
      {link.kind === 'ok'
        ? 'polling'
        : link.kind === 'probing'
          ? 'probing'
          : link.kind === 'absent'
            ? 'no route'
            : 'contract broken'}
    </span>
    <span
      class="chip"
      title="How long this tab has been open and polling. Observed by this page; it dies with the tab."
      >watching {clock(now - watchAt) ?? '00:00'}</span
    >
  </div>

  <div class="board">
    <!-- ==============================================================
         THE BEAM. One claim, and the evidence it was made from, in one
         element. Nothing else on this page states a verdict.
         ============================================================== -->
    {#if evidence.length === 0}
      <!-- THE GUARD. If a claim ever reaches here with nothing to stand on, the
           page says THAT, loudly, instead of publishing the claim. -->
      <div class="beam bad" role="alert">
        <div>
          <p class="claim">This page tried to state a verdict it has no measurement for.</p>
          <p class="claim-sub">
            The claim was: “{verdict.claim}”. It was refused rather than printed, because a claim and the
            evidence for it are one element here, and this one arrived with no evidence. That is a defect in
            this page, not in the store.
          </p>
        </div>
      </div>
    {:else}
      <div
        class="beam"
        class:good={verdict.tone === 'good'}
        class:warn={verdict.tone === 'warn'}
        class:bad={verdict.tone === 'bad'}
        class:unknown={verdict.tone === 'unknown'}
        role={verdict.tone === 'bad' ? 'alert' : undefined}
      >
        <div>
          <p class="claim">{verdict.claim}</p>
          {#if verdict.sub}<p class="claim-sub">{verdict.sub}</p>{/if}
          <div class="because">
            {#each evidence as f (f.k)}
              <div class="fact">
                {@render src(f.s)}
                <b>{f.v}</b>
                <span>{f.k}</span>
              </div>
            {/each}
          </div>
        </div>

        <div class="beam-act">
          <div class="act">
            {#if offer.label}
              <button
                class="gobtn"
                class:go={offer.go}
                type="button"
                disabled={control.busy}
                onclick={() => send(offer.action)}
                title="POST {CONTROL} action={offer.action}"
              >
                {control.busy ? 'sending…' : offer.label}
              </button>
            {/if}
            <span class="act-why">{offer.why}</span>
          </div>

          {#if receipt}
            <!-- THE RECEIPT. The server's own sentence, verbatim, and the tone
                 comes from that sentence rather than from the status code. -->
            <div
              class="receipt"
              class:done={receipt.tone === 'done'}
              class:partial={receipt.tone === 'partial'}
              class:refused={receipt.tone === 'refused'}
              class:unknown={receipt.tone === 'unknown'}
              role="status"
            >
              <b
                >POST /autopilot/control action={receipt.action} → {receipt.code || 'no answer'} · {receipt.tone ===
                'unknown'
                  ? 'NO ANSWER — it may still have taken effect'
                  : receipt.tone === 'refused'
                    ? 'refused'
                    : receipt.tone === 'partial'
                      ? 'accepted, and NOT complete'
                      : 'accepted'}</b
              >
              {receipt.why}
              <!-- THE PROVENANCE LINE IS NOT ONE SENTENCE. On every other tone
                   `why` IS the server's sentence and saying so is what makes it
                   trustworthy. On `unknown` there is no server sentence at all —
                   `why` is this page's own error text — and printing "the
                   server's own sentence, verbatim" over it is a false
                   attribution on top of a false verdict. -->
              <span class="verb"
                >{receipt.tone === 'unknown'
                  ? `This page’s own words at ${istTime(receipt.at)} IST — the request did not complete, so the server said nothing. The state above is the authority.`
                  : `The server’s own sentence, verbatim, at ${istTime(receipt.at)} IST.`}</span
              >
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- ==============================================================
         THE DECK. Six readings, each carrying the source it came from.
         ============================================================== -->
    <div class="deck">
      <!-- 1 — STATE. The word never travels alone: when the census disagrees
           with a reported "complete", the gauge carries the disagreement on
           its own face rather than leaving it to the beam. -->
      <div class="gauge">
        <span class="g-k">State</span>
        <!-- `ap !== null` RATHER THAN `Boolean(ap)`, and it is the same test:
             `ap` is either the validated payload or null, so the two agree on
             every value it can hold. The difference is only that the checker
             follows the comparison into the rest of the `&&` chain and knows
             `ap.state` is reachable there, where a `Boolean(…)` call is an
             ordinary function call it cannot see through. -->
        <div
          class="g-v"
          class:up={ap?.state === 'running'}
          class:am={ap !== null && (ap.state === 'waiting' || ap.state === 'paused' || ap.state === 'complete')}
          class:cy={ap?.state === 'starting'}
          class:dn={ap?.state === 'halted'}
        >
          {#if ap}
            <span>{ap.state === 'complete' ? 'reported done' : ap.state}</span>
          {:else}
            <span class="unk" title={link.why ?? 'the first read has not landed'}
              >{link.kind === 'probing' ? 'reading…' : 'unknown'}</span
            >
          {/if}
        </div>
        <div class="g-n">
          {@render src('rep')}
          <em
            >{#if !ap}{link.kind === 'absent'
                ? 'GET /autopilot.json answered 404'
                : link.kind === 'broken'
                  ? 'the payload did not match the contract'
                  : 'the first read has not landed'}{:else}{SAYS[ap.state] ??
                ap.state}{/if}</em
          >
        </div>
      </div>

      <!-- 2 — IN FLIGHT -->
      <div class="gauge">
        <span class="g-k">In flight</span>
        <div class="g-v" class:cy={Boolean(ap?.now)}>
          {#if ap?.now}
            <span class="mono">{clock(elapsed) ?? '—'}</span>
          {:else if ap}
            <span class="unk" title="the autopilot reports no current cell">nothing</span>
          {:else}
            <span class="unk" title="unknown — the route did not answer">—</span>
          {/if}
        </div>
        <div class="g-n">
          {@render src('rep')}
          <em title={ap?.now?.month ?? undefined}
            >{#if ap?.now}{ap.now.instrument} · {monthLabel(ap.now.month) ?? ap.now.month}{:else if ap}nothing
              is in flight — the reason is in Right now{:else}unknown — the route did not answer{/if}</em
          >
        </div>
      </div>

      <!-- 3 — INSTRUMENT-MONTHS HELD. Measured, or not drawn. -->
      <div class="gauge">
        <span class="g-k">Distinct stored instrument-months</span>
        <div class="g-v" class:am={!counted}>
          {#if counted}{@render N(census.cells)}{:else}{@render N(null, notCounted)}{/if}
        </div>
        <div class="g-n">
          {@render src('mea')}
          <!-- `census.at ?? 0` IS THE SUBTRACTION THIS LINE ALREADY DID. The
               stamp is typed `number | null`, because the shared census
               declares the field by its initial value alone — `$state({ at:
               null })` in `$lib/store.svelte.js` — although every ready read
               writes `Date.now()` into it, and `counted` above is a boolean
               rather than a narrowing, so the checker cannot see that this
               branch only runs on a landed read. `null` coerces to `0` in a
               subtraction, so the `?? 0` states the coercion the expression
               was relying on and computes the identical number. -->
          <em
            title={MEMBERSHIP_UNKNOWN}
            >{#if counted}{targetTimeframe} · target dates only · membership unverified · read {ago(now - (census.at ?? 0))}{:else}{notCounted}{/if}</em
          >
        </div>
      </div>

      <!-- 4 — Confirmed in-window bars; boundary-file counts may be unmeasured. -->
      <div class="gauge">
        <span class="g-k">Bars confirmed inside target dates</span>
        <div class="g-v" class:am={!counted}>
          {#if counted}{@render N(census.bars)}{:else}{@render N(null, notCounted)}{/if}
        </div>
        <div class="g-n">
          {@render src('mea')}
          <em title={counted ? BARS_NO_DENOM : undefined}
            >{#if counted}{feedName(census.feed) ?? 'no feed'} · {ap?.target?.timeframe ??
              'timeframe not stated'}{census.unmeasuredBars ? ` · ${inrs(census.unmeasuredBars)} boundary-file counts unmeasured` : ' · completeness unverified'}{:else}{notCounted}{/if}</em
          >
        </div>
      </div>

      <!-- 5 — STALLED. Counted here, NAMED to the right, never counted only. -->
      <div class="gauge">
        <span class="g-k">Stalled months</span>
        <div class="g-v" class:dn={Boolean(ap?.failures?.length)}>
          {#if ap}{@render N(ap.failures.length)}{:else}{@render N(
              null,
              `/autopilot.json did not answer, so the failure list is unknown. The durable record is the journal at ${JOURNAL}`
            )}{/if}
        </div>
        <div class="g-n">
          {@render src('rep')}
          <em
            >{#if !ap}unknown — which is not zero{:else if ap.failures.length}every one named to the right{:else}a
              real zero from the payload, not a missing field{/if}</em
          >
        </div>
      </div>

      <!-- 6 — THE LINK ITSELF, measured by this page and by nothing else. -->
      <div class="gauge">
        <span class="g-k">/autopilot.json</span>
        <div class="g-v" class:dn={link.kind !== 'ok'}>
          {#if link.at}
            <span class="mono">{link.ms} ms</span>
          {:else}
            <span class="unk" title="never read — a round trip of 0 ms is a measurement nobody took">—</span>
          {/if}
        </div>
        <div class="g-n">
          {@render src('obs')}
          <em
            >{#if link.at}last read {istTime(link.at)} IST · polled every {TICK_MS / 1000}s{:else}never read —
              a round trip of 0 ms is a measurement nobody took{/if}</em
          >
        </div>
      </div>
    </div>

    <div class="floor">
      <div class="col">
        <!-- ==========================================================
             THE WATCH BAY — what is happening right now, and the partition
             that makes a full meter over outstanding work impossible to draw.
             ========================================================== -->
        <section class="bay rise">
          <div class="bay-h">
            <span class="bay-t">Right now</span>
            {#if ap?.state === 'running' && ap.now}
              <span class="chip cy" role="status" aria-live="polite">
                <span class="dot acc live"></span> fetching
              </span>
            {/if}
            <span class="sp"></span>
            {@render src('rep')}
          </div>

          {#if !ap}
            <div class="void">
              <b>Nothing can be said about the current cell.</b>
              {link.kind === 'probing'
                ? 'The first read has not landed, so what is in flight is unknown — and unknown is not "nothing".'
                : 'No readable payload. The reason is in the beam above.'}
            </div>
          {:else if ap.state === 'starting'}
            <div class="void">
              <b>Opening the store. Nothing has been asked of a vendor yet.</b>
              There are {@render N(
                graceLeft,
                'the grace countdown, measured by this page since it opened'
              )} seconds of the {GRACE_SECS}-second grace window left — the one chance to say no before the first
              socket opens.
            </div>
          {:else if !ap.now}
            <div class="void">
              <b>Nothing is in flight.</b>
              {#if ap.state === 'complete'}The autopilot reports that its work is done.
                Stored target coverage is unverified: {MEMBERSHIP_UNKNOWN}
                {#if ap.why}<p>{ap.why}</p>{/if}{:else if ap.why}{ap.why}{:else}The
                autopilot reported state “{ap.state}” and no current cell.{/if}
            </div>
          {:else}
            <div class="flight">
              <div class="fclock mono">{clock(elapsed) ?? '—'}</div>
              <div class="fwho">
                <b>{ap.now.instrument}</b>
                <span
                  >{monthLabel(ap.now.month) ?? ap.now.month} · {ap.now.timeframe ??
                    'timeframe not stated'} · {feedName(ap.now.feed) ?? 'feed not stated'}</span
                >
              </div>
            </div>

            {#if typeof ap.now.of === 'number' && typeof ap.now.index === 'number' && ap.now.of > 0}
              <!-- THE PARTITION. held + in flight + outstanding = the instruments
                   in this month. Outstanding is a SEGMENT with a width and a
                   printed count, so the meter cannot fill while anything is
                   owed — and the sum line under it says so, or shouts MISMATCH
                   when the three parts do not reach the whole. -->
              {@const of = ap.now.of}
              {@const idx = Math.max(0, Math.min(of, ap.now.index))}
              {@const inflight = 1}
              {@const outstanding = Math.max(0, of - idx - inflight)}
              {@const total = idx + inflight + outstanding}
              <div class="meter" aria-hidden="true">
                <i class="seg done" style="width:{(idx / of) * 100}%"></i>
                <i class="seg now" style="width:{(inflight / of) * 100}%"></i>
                <i class="seg out" style="width:{(outstanding / of) * 100}%"></i>
              </div>
              <div class="mkey">
                <div class="mk"><i class="done"></i><b class="mono">{inrs(idx)}</b>held in this month</div>
                <div class="mk"><i class="now"></i><b class="mono">{inrs(inflight)}</b>in flight</div>
                <div class="mk"><i class="out"></i><b class="mono">{inrs(outstanding)}</b>outstanding</div>
              </div>
              <div class="msum" class:bad={total !== of}>
                {#if total === of}adds to <b>{inrs(of)}</b> instruments in {monthLabel(ap.now.month) ??
                    ap.now.month}. Full only when outstanding is zero.{:else}MISMATCH — <b>{inrs(of)}</b>
                  instruments in this month, {inrs(total)} accounted for.{/if}
              </div>
            {:else}
              <div class="bay-note">
                <b>No position inside the month.</b>
                No <b>now.index</b> / <b>now.of</b> in the payload, so how
                far into {monthLabel(ap.now.month) ?? 'this month'} it has reached is not drawn — neither as zero
                nor as full.
              </div>
            {/if}
          {/if}

          {#if ap?.waiting_ms}
            <div class="bay-note">
              <span class="chip am">throttled</span> Waiting <b>{clock(ap.waiting_ms) ?? '—'}</b> on the rate
              governor — it waits rather than refusing, so a throttled window costs time and never costs a month.
            </div>
          {/if}
          {#if ap?.absorbed_ms}
            <div class="bay-note">
              Throttle absorbed since start <b>{clock(ap.absorbed_ms) ?? '—'}</b> — waiting, not failing.
            </div>
          {/if}
        </section>

        <!-- ==========================================================
             THE TAPE — the month ladder, oldest at the top, because that is
             the order the work runs in. The store is append-only with one file
             per month: a later month written first permanently blocks the
             earlier days inside that file.
             ========================================================== -->
        <section class="bay rise">
          <div class="bay-h">
            <span class="bay-t">Coverage — oldest first</span>
            {#if ap?.target}
              {@const sp = span(ap.target.from, ap.target.to)}
              <span
                class="chip"
                class:dn={!sp.ok}
                title="{String(ap.target.from)} → {String(ap.target.to)} — the span reported by /autopilot.json"
                >{sp.ok
                  ? `${monthLabel(ap.target.from)} → ${monthLabel(ap.target.to)}`
                  : 'target is not a span'}</span
              >
            {/if}
            {#if counted && census.feed}
              <span
                class="chip"
                title="The feed this census answered for. It follows the target the autopilot reports, and otherwise the top bar's selection — this page never writes that selection."
                >{feedName(census.feed)}</span
              >
            {/if}
            <span class="sp"></span>
            {@render dial('show', 'Show', show, SHOW, (v) => (show = v))}
          </div>

          {#if !scope.same}
            <div class="bay-note"><span class="chip am">different question</span> {scope.why}</div>
          {:else if !counted}
            <div class="void">
              <b>Target coverage is unavailable.</b>
              {notCounted} Counts are withheld while the scope is unverified.
            </div>
          {:else if !ladder.ok}
            <!-- NO ROWS. The ladder this replaces walked a guarded loop and
                 emitted 1,200 rows of months the payload never named. Here the
                 refusal names the field, prints the raw value it carried, and
                 stops. There is no path from a bad target to a row. -->
            <div class="refuse" role="alert">
              <b>The coverage table cannot show this date range: {ladder.refuse.field}.</b>
              <div>
                /autopilot.json reported <code>{JSON.stringify(ladder.refuse.raw)}</code> — {ladder.refuse.why}
              </div>
              <div class="pad">
                What the store holds is still measured, in the deck above. Fix the target; there is nothing to
                fix here.
              </div>
            </div>
          {:else if rows.length === 0}
            <div class="void">
              <b>The store holds nothing for {feedName(census.feed) ?? 'this feed'} yet.</b>
              /store.json answered
              with no rows — the state before the first month lands, not an error.
            </div>
          {:else}
            <div class="tape">
              {#each shown as r (r.key)}
                {@const k = classOf(r)}
                {@const here = ap?.cursor === r.key}
                <div
                  class="rung"
                  class:here
                  class:open={openRung === r.key}
                  class:void={k === 'none'}
                  role="button"
                  tabindex="0"
                  aria-expanded={openRung === r.key}
                  onclick={() => (openRung = openRung === r.key ? null : r.key)}
                  onkeydown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      openRung = openRung === r.key ? null : r.key;
                    }
                  }}
                >
                  <div class="rung-m">
                    {@render monthNode(r.key)}
                    {#if here}
                      <!-- VIOLET, because the cursor is the one REPORTED fact in
                           a row of measured ones. The colour is the provenance. -->
                      <span class="chip vi" title="the cursor /autopilot.json reports — reported, not measured"
                        >here</span
                      >
                    {/if}
                  </div>

                  <!-- Hatching means the completion denominator is unknown,
                       even when the number of stored records is large. -->
                  <div class="rung-bar nod" title={MEMBERSHIP_UNKNOWN}></div>

                  <div class="rung-n" title="Distinct stored instruments with a record inside the target dates; {inrs(r.unknownCells)} boundary-file overlaps are unknown.">
                    {@render N(r.cells, 'not measured')} records{r.unknownCells ? ` + ${inrs(r.unknownCells)} unknown` : ''}
                  </div>
                  <div class="rung-n" title="Bars confirmed inside target dates; {inrs(r.unmeasuredBars)} boundary-file counts are unmeasured. {BARS_NO_DENOM}">
                    {r.unmeasuredBars ? '≥ ' : ''}{@render N(r.bars, 'not measured')}
                  </div>
                  <div class="rung-x">{openRung === r.key ? '▾' : '▸'}</div>

                  {#if openRung === r.key}
                    <div class="rung-why">
                      <b>{monthLabel(r.key) ?? r.key}</b> — {inrs(r.cells)} distinct stored instruments
                      have records inside the target dates at {targetTimeframe}; {inrs(r.bars)} bars are confirmed.
                      {#if r.unknownCells}{inrs(r.unknownCells)} boundary-file overlaps are unknown.{/if}
                      {#if r.unmeasuredBars}Exact in-range bar counts for {inrs(r.unmeasuredBars)} boundary files
                        cannot be read from the census, so their whole-file totals are excluded.{/if}
                      {MEMBERSHIP_UNKNOWN}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>

            <!-- THE FOOT. The buckets partition the ladder and the total says
                 so — or says MISMATCH, because a partition that does not add up
                 is a tidy number hiding a month nobody accounted for. -->
            <div class="tape-foot">
              <div class="bkt" title={MEMBERSHIP_UNKNOWN}>
                <i class="nod"></i><b class="mono">{inrs(buckets.nod)}</b>with records
              </div>
              <div class="bkt"><i class="part"></i><b class="mono">{inrs(buckets.part)}</b>overlap unknown</div>
              <div class="bkt"><i class="none"></i><b class="mono">{inrs(buckets.none)}</b>no records</div>
              <span class="tot" class:bad={bucketSum !== rows.length}
                >{bucketSum === rows.length
                  ? `of ${inrs(rows.length)} ${ladder.framed ? 'months in the target span' : 'months the store holds'}`
                  : `MISMATCH — ${inrs(rows.length)} months, ${inrs(bucketSum)} accounted for`}</span
              >
            </div>

            {#if shown.length !== rows.length}
              <div class="bay-note">
                Showing {inrs(shown.length)} of {inrs(rows.length)} rows — Show is “{SHOW.find(
                  (s) => s.k === show
                )?.n ?? show}”. The buckets count the whole span either way.
              </div>
            {/if}

            <div class="bay-note">
              {@render src('mea')} Counts use the reported feed, timeframe and date window only. Each stored
              instrument is counted once per month. Other timeframes and dates cannot fill an empty month here.
              The hatched area means completion is unverified. {MEMBERSHIP_UNKNOWN}
              <a class="link" href="/audit">Audit</a> shows the separate store inventory.
            </div>
          {/if}
        </section>
      </div>

      <div class="col">
        <!-- ==========================================================
             THE SNAG BAY — what is stuck. A failure is never a count alone.
             ========================================================== -->
        <section class="bay rise">
          <div class="bay-h">
            <span class="bay-t">What is stuck</span>
            {#if ap}
              <span class="chip" class:dn={ap.failures.length > 0} class:up={ap.failures.length === 0}
                >{ap.failures.length ? `${inrs(ap.failures.length)} stalled` : 'none'}</span
              >
            {:else}
              <span class="chip am" title={link.why ?? 'the first read has not landed'}>unknown</span>
            {/if}
            <span class="sp"></span>
            {#if ap?.failures.length}
              {@render dial('grp', 'Group', group, GROUPS, (v) => (group = v))}
            {/if}
          </div>

          {#if !ap}
            <div class="void">
              <b>Unknown — and unknown is not zero.</b>
              The failure list lives in /autopilot.json, which did not
              answer. The durable record is <code>{JOURNAL}</code>, rendered at
              <a class="link" href="/audit">Audit</a>.
            </div>
          {:else if ap.failures.length === 0}
            <div class="void">
              <b>Nothing has failed.</b>
              A real zero from the payload — a body with no <code>failures</code> key
              is refused by the reader above, never shown as none.
            </div>
          {:else}
            {#each snags as f, i (`${f.instrument}-${f.month}-${f.i}`)}
              {#if i === 0 || groupKey(snags[i - 1]) !== groupKey(f)}
                <div class="grp">{groupKey(f)}</div>
              {/if}
              <div class="snag">
                <div class="snag-who">{f.instrument} · {@render monthNode(f.month)}</div>
                <div class="snag-when">
                  {#if f.at === null}
                    <span
                      class="unk"
                      title="no `at` on this failure — the payload carried an empty string, which is a different fact from “no timestamp needed”."
                      >—</span
                    >
                  {:else if dayLabel(f.at)}
                    <span class="mono" title="{f.at} — the form a journal line carries, which is the form to copy"
                      >{dayLabel(f.at)}</span
                    >
                  {:else}
                    <span
                      class="unk"
                      title="not a day this page can read. The raw value is shown rather than swallowed: accepting it silently would throw away the only evidence that the payload is malformed."
                      >{f.at}</span
                    >
                  {/if}
                </div>
                <div class="snag-why">
                  {#if f.why}{f.why}{:else}<span
                      class="unk"
                      title="A stop that does not name its reason is the failure CLAUDE.md §4 bans."
                      >the payload carried no reason — that is itself the bug</span
                    >{/if}
                </div>
              </div>
            {/each}

            <div class="bay-note">
              {@render src('rep')} Each of these stalled after its attempts and was passed over, so later months
              were not blocked behind it. This list dies with the process; the durable record is
              <code>{ap.journal ?? JOURNAL}</code>, rendered at <a class="link" href="/audit">Audit</a>. A failure
              here and not there was never written down, and that is a defect in the journal, not in this page.
            </div>
          {/if}
        </section>

        <!-- ==========================================================
             THE TRAIL — what changed while nobody was looking. Observed by
             this page, and by nothing else. This is the panel the owner reads
             when he comes back.
             ========================================================== -->
        <section class="bay rise">
          <div class="bay-h">
            <span class="bay-t">While you were away</span>
            <span class="chip" title="How long this tab has been watching.">{clock(now - watchAt) ?? '00:00'}</span>
            <span class="sp"></span>
            {@render src('obs')}
          </div>

          <div class="bay-note top">
            Changes this page saw between two of its own reads. Not a server log, and gone when the tab is. The
            durable record is <code>{ap?.journal ?? JOURNAL}</code>.
          </div>

          <div class="trail">
            {#if trail.length === 0}
              <div class="void">
                Nothing has changed since this page opened — an observation, not a verdict. A process quietly
                doing nothing looks exactly like this.
              </div>
            {:else}
              {#each trail as t (`${t.t}-${t.text}`)}
                <div class="tr">
                  <div class="tr-t">{istTime(t.t)}</div>
                  <div class="tr-x">{t.text}</div>
                </div>
              {/each}
            {/if}
          </div>
        </section>
      </div>
    </div>
  </div>
</div>

<style>
  /* THE RAMP IS THE APP'S OWN, VALUE FOR VALUE, off `$lib/theme.css`. Not one
     literal colour appears below: the design's `--cy` is `--acc`, its
     `--up`/`--dn`/`--amber`/`--violet` are `--up`/`--down`/`--warn`/`--info`,
     and its greys are the neutral ramp through `--bg`, `--panel`, `--panel-2`,
     `--well`, `--line`, `--line-soft`, `--line-hard`, `--ink`, `--dim` and
     `--faint`. A literal here would be a second theme the top bar's toggle
     cannot reach, and it would be the wrong colour in one of the two modes by
     construction.

     THE CONTROL VOCABULARY IS THIS PAGE'S OWN — beam, deck, gauge, bay, meter,
     tape, rung, snag, trail, dial — because the question is its own: /db and
     /ingest narrow a selection, and this page watches unattended work. None of
     these names collide with `.strip`, `.cell`, `.picker`, `.menu` or
     `.pager`. */

  /* ---- THE BOARD — the field everything sits on ----------------------
     The wash is two very wide radial tints mixed OUT OF THE THEME'S OWN
     accents rather than typed in, exactly as /db builds its board. At 5% and
     4% the eye reads it as depth and never as a hue. */
  .board {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: var(--s5);
    background-color: var(--bg);
    background-image:
      radial-gradient(1100px 620px at 6% 0%, color-mix(in srgb, var(--acc) 5%, transparent), transparent 60%),
      radial-gradient(900px 520px at 96% 0%, color-mix(in srgb, var(--info) 4%, transparent), transparent 58%);
  }

  .chip {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: 10.5px;
    font-weight: var(--w-bold);
    padding: 2px var(--s4);
    border-radius: var(--r-full);
    background: var(--panel-2);
    color: var(--dim);
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: var(--s3);
  }
  .chip.up {
    color: var(--up);
    background: var(--up-soft);
  }
  .chip.dn {
    color: var(--down);
    background: var(--down-soft);
  }
  .chip.am {
    color: var(--warn);
    background: var(--warn-soft);
  }
  .chip.cy {
    color: var(--acc);
    background: var(--acc-soft);
  }
  .chip.vi {
    color: var(--info);
    background: var(--info-soft);
  }

  /* ---- PROVENANCE. Three sources, never merged, never unlabelled ------
     .mea  measured off disk       — /store.json, true whether or not it runs
     .rep  reported by the process — /autopilot.json, a claim, not evidence
     .obs  observed by this page   — successive polls, the only thing it owns */
  .src {
    font-size: 9px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    font-weight: var(--w-heavy);
    padding: 2px 7px;
    border-radius: var(--r-full);
    white-space: nowrap;
    flex: none;
  }
  .src.mea {
    color: var(--acc);
    background: var(--acc-soft);
  }
  .src.rep {
    color: var(--info);
    background: var(--info-soft);
  }
  .src.obs {
    color: var(--faint);
    background: var(--panel-2);
  }

  /* AN UNKNOWN IS NAMED, NEVER BLANKED. Every em dash on this page is this
     element, it is amber, and it carries the reason on its title. */
  .unk {
    color: var(--warn);
    border-bottom: 1px dotted var(--warn);
    cursor: help;
    font-weight: var(--w-bold);
  }

  /* ---- THE BEAM — the one claim, and the evidence it is made of -------
     Structurally one element: `.claim` and `.because` are siblings inside it,
     and the template cannot build one without the other. */
  .beam {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(250px, auto);
    gap: var(--s6);
    padding: var(--s5) var(--s6);
    border-radius: var(--r4);
    border: 1px solid var(--line);
    border-left: 3px solid var(--tone);
    background: linear-gradient(180deg, var(--panel-2), var(--panel));
    box-shadow: var(--e2);
    margin: 0 0 var(--s5);
    --tone: var(--line-hard);
  }
  @media (max-width: 860px) {
    .beam {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .beam.good {
    --tone: var(--up);
  }
  .beam.warn {
    --tone: var(--warn);
  }
  .beam.bad {
    --tone: var(--down);
  }
  .beam.unknown {
    --tone: var(--line-hard);
  }
  .claim {
    font-size: var(--fs-lg);
    font-weight: var(--w-bold);
    letter-spacing: -0.02em;
    line-height: 1.28;
    margin: 0;
    color: var(--ink);
  }
  .beam.good .claim {
    color: var(--up);
  }
  .beam.bad .claim {
    color: var(--down);
  }
  .beam.warn .claim {
    color: var(--warn);
  }
  .beam.unknown .claim {
    color: var(--dim);
  }
  .claim-sub {
    margin: var(--s3) 0 0;
    font-size: var(--fs-sm);
    color: var(--dim);
    max-width: 92ch;
    line-height: var(--lh-base);
  }
  .because {
    display: flex;
    flex-wrap: wrap;
    gap: 7px var(--s6);
    margin: var(--s5) 0 0;
    padding: var(--s5) 0 0;
    border-top: 1px solid var(--line-soft);
  }
  .fact {
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    font-size: var(--fs-sm);
    color: var(--faint);
    min-width: 0;
  }
  .fact b {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-base);
    font-weight: var(--w-bold);
    color: var(--ink);
  }
  .beam-act {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: var(--s4);
    justify-content: flex-start;
  }

  /* ---- THE CONTROL, and its receipt ---- */
  .act {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: var(--s4);
  }
  .gobtn {
    appearance: none;
    border: 1px solid var(--line);
    background: var(--panel-2);
    color: var(--ink);
    font: inherit;
    font-weight: var(--w-bold);
    font-size: var(--fs-base);
    padding: 9px var(--s6);
    border-radius: var(--r3);
    cursor: pointer;
    white-space: nowrap;
  }
  .gobtn:hover:not(:disabled) {
    border-color: var(--acc);
  }
  .gobtn.go {
    background: linear-gradient(135deg, var(--acc), var(--acc-hi));
    color: var(--on-acc);
    border-color: transparent;
  }
  .gobtn:disabled {
    opacity: 0.42;
    cursor: not-allowed;
  }
  .act-why {
    font-size: var(--fs-xs);
    color: var(--faint);
    text-align: right;
    max-width: 36ch;
    line-height: var(--lh-base);
  }
  /* THE RECEIPT PRINTS THE SERVER'S OWN SENTENCE, VERBATIM, AND NOTHING ELSE.
     There is no success template on this page — see `send`. */
  .receipt {
    margin: 0;
    padding: var(--s5);
    border-radius: var(--r3);
    border-left: 3px solid var(--tone);
    background: var(--bgc);
    font-family: var(--mono);
    font-size: var(--fs-sm);
    line-height: var(--lh-base);
    color: var(--ink);
    max-width: 44ch;
    --tone: var(--line-hard);
    --bgc: var(--panel-2);
  }
  .receipt.done {
    --tone: var(--up);
    --bgc: var(--up-soft);
  }
  .receipt.partial {
    --tone: var(--warn);
    --bgc: var(--warn-soft);
  }
  .receipt.refused {
    --tone: var(--down);
    --bgc: var(--down-soft);
  }
  /* AMBER AND NOT `--down`. Red on this product means a refusal or a price that
     fell — both facts about what happened. Not knowing is a severity, not a
     direction, and it must not read as the refusal it replaced. */
  .receipt.unknown {
    --tone: var(--warn);
    --bgc: var(--warn-soft);
  }
  .receipt b {
    display: block;
    font-family: var(--sans);
    font-size: var(--fs-mini);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    margin-bottom: var(--s3);
    color: var(--tone);
  }
  .receipt .verb {
    display: block;
    margin-top: 7px;
    font-family: var(--sans);
    font-size: var(--fs-xs);
    color: var(--faint);
  }

  /* ---- THE DECK — one band, hairline divisions, never tiles ---- */
  .deck {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(196px, 1fr));
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    overflow: hidden;
    margin: 0 0 var(--s6);
  }
  .gauge {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: var(--s5) var(--s6);
    border-right: 1px solid var(--line-soft);
    min-width: 0;
  }
  .gauge:last-child {
    border-right: 0;
  }
  .g-k {
    font-size: 9px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-heavy);
    white-space: nowrap;
  }
  .g-v {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-xl);
    font-weight: var(--w-bold);
    letter-spacing: -0.025em;
    line-height: 1.15;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink);
  }
  .g-v.up {
    color: var(--up);
  }
  .g-v.dn {
    color: var(--down);
  }
  .g-v.am {
    color: var(--warn);
  }
  .g-v.cy {
    color: var(--acc);
  }
  .g-n {
    font-size: var(--fs-xs);
    color: var(--faint);
    display: flex;
    align-items: center;
    gap: 7px;
    flex-wrap: wrap;
    min-width: 0;
  }
  .g-n em {
    font-style: normal;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ---- THE FLOOR: what is happening | what needs you ---- */
  .floor {
    display: grid;
    grid-template-columns: minmax(0, 1.24fr) minmax(0, 0.86fr);
    gap: var(--s5);
    align-items: start;
  }
  @media (max-width: 1080px) {
    .floor {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: var(--s5);
    min-width: 0;
  }
  .bay {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    min-width: 0;
  }
  .bay-h {
    display: flex;
    align-items: center;
    gap: var(--s5);
    padding: var(--s4) var(--s6);
    border-bottom: 1px solid var(--line-soft);
    flex-wrap: wrap;
  }
  .bay-t {
    font-size: var(--fs-xs);
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-heavy);
  }
  .bay-h .sp {
    margin-left: auto;
  }
  .bay-note {
    padding: var(--s4) var(--s6);
    font-size: var(--fs-sm);
    color: var(--faint);
    line-height: var(--lh-base);
    border-top: 1px solid var(--line-soft);
  }
  .bay-note.top {
    border-top: 0;
  }
  .bay-note b {
    color: var(--dim);
    font-family: var(--mono);
  }
  .void {
    padding: var(--s6);
    font-size: var(--fs-sm);
    color: var(--dim);
    line-height: 1.55;
  }
  .void b {
    color: var(--ink);
  }
  code {
    font-family: var(--mono);
    font-size: 0.93em;
    padding: 0 var(--s2);
    border-radius: var(--r1);
    background: var(--well);
    color: var(--dim);
  }

  /* ---- THE WATCH — the cell in flight ---- */
  .flight {
    display: flex;
    align-items: baseline;
    gap: var(--s6);
    padding: var(--s5) var(--s6) var(--s2);
    flex-wrap: wrap;
  }
  .fclock {
    font-size: var(--fs-2xl);
    font-weight: var(--w-heavy);
    letter-spacing: -0.035em;
    line-height: 1;
    color: var(--ink);
  }
  .fwho {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .fwho b {
    font-family: var(--mono);
    font-size: var(--fs-md);
    font-weight: var(--w-bold);
  }
  .fwho span {
    font-size: var(--fs-xs);
    color: var(--faint);
  }

  /* ---- THE METER — a partition that owns pixels for every unit --------
     Outstanding work is a SEGMENT, not the absence of one. A meter cannot read
     full while anything is outstanding, because the outstanding units are drawn
     and counted in the sum line under it. */
  .meter {
    display: flex;
    height: 9px;
    border-radius: var(--r-full);
    overflow: hidden;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-soft);
    margin: var(--s5) var(--s6) 0;
  }
  .seg {
    height: 100%;
    min-width: 0;
  }
  .seg.done {
    background: var(--up);
  }
  .seg.now {
    background: var(--acc);
  }
  .seg.out {
    background-image: repeating-linear-gradient(135deg, var(--line) 0 3px, transparent 3px 8px);
  }
  .mkey {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3) var(--s6);
    padding: var(--s4) var(--s6) 0;
    font-size: var(--fs-xs);
  }
  .mk {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--faint);
  }
  .mk i {
    width: 9px;
    height: 9px;
    border-radius: 3px;
    flex: none;
  }
  .mk i.done {
    background: var(--up);
  }
  .mk i.now {
    background: var(--acc);
  }
  .mk i.out {
    background-image: repeating-linear-gradient(135deg, var(--line) 0 3px, transparent 3px 8px);
    box-shadow: inset 0 0 0 1px var(--line-soft);
  }
  .mk b {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
    font-weight: var(--w-bold);
  }
  .msum {
    padding: var(--s4) var(--s6) var(--s5);
    font-size: var(--fs-xs);
    color: var(--faint);
    line-height: var(--lh-base);
  }
  .msum b {
    font-family: var(--mono);
    color: var(--dim);
  }
  .msum.bad {
    color: var(--down);
    font-weight: var(--w-bold);
  }
  .msum.bad b {
    color: var(--down);
  }

  /* ---- THE TAPE — the month ladder, oldest at the top ---- */
  .tape {
    max-height: 520px;
    overflow: auto;
  }
  .rung {
    display: grid;
    grid-template-columns: 104px minmax(80px, 1fr) 124px 96px 16px;
    gap: var(--s5);
    align-items: center;
    padding: 7px var(--s6);
    border-bottom: 1px solid var(--line-soft);
    cursor: pointer;
    border-left: 2px solid transparent;
  }
  .rung:hover {
    background: var(--panel-2);
  }
  .rung.here {
    border-left-color: var(--acc);
    background: linear-gradient(90deg, var(--acc-soft), transparent 70%);
  }
  .rung.void .rung-n {
    color: var(--faint);
  }
  .rung-m {
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: var(--s3);
  }
  .rung.void .rung-m {
    color: var(--dim);
    font-weight: var(--w-mid);
  }
  .rung-bar {
    position: relative;
    height: 9px;
    border-radius: var(--r-full);
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-soft);
  }
  .rung-bar.nod {
    background-image: repeating-linear-gradient(135deg, var(--line) 0 3px, transparent 3px 8px);
  }
  .rung-n {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-xs);
    text-align: right;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rung-x {
    color: var(--faint);
    font-size: var(--fs-xs);
    text-align: center;
  }
  .rung.open .rung-x {
    color: var(--acc);
  }
  .rung-why {
    grid-column: 1 / -1;
    font-size: var(--fs-sm);
    color: var(--faint);
    line-height: var(--lh-base);
    padding: 3px 0 var(--s2);
    cursor: default;
  }
  .rung-why b {
    color: var(--ink);
    font-family: var(--mono);
  }
  .tape-foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s4) var(--s5);
    padding: var(--s5) var(--s6);
    border-top: 1px solid var(--line);
    background: var(--panel-2);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .tape-foot .tot {
    margin-left: auto;
    font-family: var(--mono);
    font-weight: var(--w-bold);
    color: var(--dim);
  }
  .tape-foot .tot.bad {
    color: var(--down);
  }
  .bkt {
    display: flex;
    align-items: center;
    gap: var(--s3);
  }
  .bkt i {
    width: 9px;
    height: 9px;
    border-radius: 3px;
    flex: none;
  }
  .bkt b {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
    font-weight: var(--w-bold);
  }
  .bkt i.part {
    background: var(--acc);
  }
  .bkt i.none {
    background: var(--line-hard);
  }
  .bkt i.nod {
    background-image: repeating-linear-gradient(135deg, var(--line) 0 3px, transparent 3px 8px);
    box-shadow: inset 0 0 0 1px var(--line-soft);
  }
  .refuse {
    margin: 0;
    padding: var(--s6);
    font-size: var(--fs-sm);
    line-height: 1.55;
    color: var(--down);
    background: var(--down-soft);
    border-left: 3px solid var(--down);
  }
  .refuse b {
    display: block;
    color: var(--ink);
    font-size: var(--fs-md);
    margin-bottom: var(--s2);
  }
  .refuse code {
    background: var(--panel);
    color: var(--down);
  }
  .refuse .pad {
    margin-top: 7px;
  }

  /* ---- SNAGS — what is stuck, and it is never only counted ---- */
  .snag {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 3px var(--s5);
    padding: var(--s5) var(--s6);
    border-bottom: 1px solid var(--line-soft);
  }
  .snag-who {
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .snag-when {
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--faint);
    text-align: right;
    white-space: nowrap;
  }
  .snag-why {
    grid-column: 1 / -1;
    font-size: var(--fs-sm);
    color: var(--down);
    line-height: 1.45;
  }
  .grp {
    padding: var(--s4) var(--s6) var(--s3);
    font-size: 9px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-heavy);
    background: var(--panel-2);
    border-bottom: 1px solid var(--line-soft);
  }

  /* ---- THE TRAIL — what this page itself saw while nobody watched ---- */
  .trail {
    max-height: 270px;
    overflow: auto;
  }
  .tr {
    display: grid;
    grid-template-columns: 78px minmax(0, 1fr);
    gap: var(--s5);
    padding: 7px var(--s6);
    font-size: var(--fs-sm);
    border-bottom: 1px solid var(--line-soft);
  }
  .tr-t {
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--faint);
    white-space: nowrap;
  }
  .tr-x {
    color: var(--dim);
    line-height: 1.45;
  }

  /* ---- THE DIAL — this page's one control primitive -------------------
     A dial shows its CURRENT VALUE on its face. It is not a row of links and it
     is not a segmented bar: there are three to fourteen answers behind each one
     and the face has to survive all of them. The menu wears the same tokens
     /db's menus wear, so the four pages read as one product. */
  .dial {
    position: relative;
    display: inline-flex;
  }
  .dial-face {
    appearance: none;
    display: flex;
    align-items: center;
    gap: var(--s4);
    background: var(--panel-2);
    border: 1px solid var(--line);
    border-radius: var(--r3);
    color: var(--ink);
    font: inherit;
    padding: var(--s3) var(--s5);
    cursor: pointer;
    max-width: 100%;
  }
  .dial-face:hover {
    border-color: var(--line-hard);
  }
  .dial-face .dl {
    font-size: 9px;
    letter-spacing: 0.13em;
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-heavy);
    white-space: nowrap;
  }
  .dial-face .dv {
    font-family: var(--mono);
    font-weight: var(--w-bold);
    font-size: var(--fs-base);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dial-face .dc {
    width: 0;
    height: 0;
    flex: none;
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-top: 5px solid var(--acc);
  }
  /* RIGHT-ANCHORED, because both dials sit at the right edge of their bay
     header: a left-anchored menu there runs past the panel and clips its own
     rows. */
  .dial-list {
    position: absolute;
    top: calc(100% + var(--s3));
    right: 0;
    z-index: 40;
    min-width: 290px;
    max-width: min(92vw, 420px);
    max-height: 360px;
    overflow: auto;
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: var(--r3);
    padding: var(--s2);
    box-shadow: var(--e3);
  }
  .dial-opt {
    display: flex;
    align-items: baseline;
    gap: var(--s5);
    width: 100%;
    text-align: left;
    background: transparent;
    border: 0;
    color: var(--ink);
    font: inherit;
    padding: var(--s4) var(--s5);
    border-radius: var(--r2);
    cursor: pointer;
    min-height: 34px;
  }
  .dial-opt:hover:not(:disabled) {
    background: var(--panel-2);
  }
  .dial-opt[aria-selected='true'] {
    background: var(--acc-soft);
    color: var(--acc);
  }
  /* A DEAD ROW STAYS IN PLACE AND CARRIES ITS REASON. Omitting it would say the
     question cannot be asked; this says it cannot be answered, and why. */
  .dial-opt:disabled {
    cursor: not-allowed;
    color: var(--faint);
  }
  .dial-opt .on {
    font-family: var(--mono);
    font-weight: var(--w-bold);
    font-size: var(--fs-base);
    flex: none;
  }
  .dial-opt .oy {
    color: var(--faint);
    font-size: var(--fs-xs);
    flex: 1;
    min-width: 0;
  }
  .dial-opt[aria-selected='true'] .oy {
    color: var(--acc);
  }
</style>
