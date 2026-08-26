<script>
  /**
   * THE BACKTEST CONSOLE — what the brute force actually found.
   *
   * # What this closes
   *
   * `/audit` is the INGEST console: what a PULL did. Until this page nothing in
   * the front end said anything about the ENGINE's output. `cli::results`
   * recorded every completed sweep into an append-only file and the only reader
   * was a terminal command, so "which rung carries the edge on NIFTY" was a
   * question answerable only by rerunning a CLI.
   *
   * # The one fact that governs every other one
   *
   * `halted` decides whether a run's other figures mean anything. A halted
   * ladder stopped short: its `depth` is PARTIAL and its `combinations` covers
   * LESS of the search while reading LARGER. Ranking a halted row against a
   * complete one proposes a winner no other surface in this workspace agrees
   * with, so it is excluded from ranking outright — never dimmed, never
   * sorted-to-the-bottom, EXCLUDED — and the exclusion is visible as a
   * persistent amber rail rather than as an absence.
   *
   * # Ranking is on the WORST number, everywhere
   *
   * `pessimistic` is adverse-extreme fills; `optimistic` is open fills. Every
   * selection surface in this workspace ranks on the first, and a page that led
   * with the second would name a different winner from the CLI reading the same
   * file. So `worst` is the solid bar and the sorted column; `best` is drawn
   * behind it as a ghost. The GAP between them is a fact in its own right —
   * how much of the headline is fill assumption.
   *
   * # What is deliberately NOT drawn
   *
   * **No equity curve.** The ledger carries four scalars per run — total worst,
   * total best, worst single trade, max drawdown — and no series at all. A
   * sparkline of an equity curve would be a shape invented from four numbers,
   * which is the invention `CLAUDE.md` §3 rule 1 bans. What IS drawn is those
   * four scalars to one scale, which is a real comparison.
   *
   * **No per-level ladder breakdown.** The record holds `depth` and
   * `combinations` as two scalars, so "where did the frequent frontier empty"
   * cannot be answered from this file. It is drawn as a NAMED GAP with the
   * reason, because a missing drill-down that says why is information and a
   * missing drill-down that is simply absent is a page that looks finished and
   * is not.
   *
   * **The winning mask WAS that gap and is not any more.** This paragraph said
   * "which conditions won" could not be answered either, because the record
   * held only the run's blake3 identity — a hash over the mask and eight other
   * terms, which cannot be turned back into conditions. Format version 3
   * appends the mask itself, so the ledger became the thing it was always
   * supposed to be: a permanent record of what was FOUND, not only of what it
   * was worth.
   *
   * Three states, and they must not be collapsed into one. A version-2 ledger
   * PREDATES the field; a version-3 run may genuinely have found no
   * combination; and a version-3 run with bits set names them. The first two
   * are byte-identical — six zero words — which is why `has_mask` travels on
   * the ledger and is read here before the mask is.
   *
   * Names come from `/vocab.json`, fetched once. They are deliberately NOT in
   * `/backtest.json`: decoding there would repeat 370 rows of vocabulary on
   * every run in every response, and a hand-kept copy in this file would be two
   * vocabularies for one fact — right the day it was written and silently wrong
   * the first time a bit is appended.
   *
   * # O(1)
   *
   * The filter is a prefix probe into a Map built once per payload — the same
   * technique `/db` and `web/typeahead.js` use — never a scan of the ledger and
   * never a request per character. Rows are windowed: only the visible slice is
   * in the DOM, inside a spacer of the exact total height, so the scrollbar is
   * honest at any run count. The rung grouping is one pass keyed on
   * `feed·symbol·span·support ratio`.
   *
   * # The rung list is DERIVED, never declared
   *
   * `store::path::Timeframe::KNOWN` can gain a rung without this file changing:
   * the set of rungs on screen is whatever the ledger holds, ordered by parsing
   * `<n><unit>` into seconds. A hardcoded list would be a list the store can
   * contradict.
   */
  import { untrack } from 'svelte';
  import { feeds } from '$lib/feeds.svelte.js';
  /* RENAMED ON IMPORT. This page's Run control owns a state object called
     `ask` — what the operator is asking the sweep for — and the fetch helper
     is a different thing entirely. One name for one value. */
  import { ask as ask_ } from '$lib/ask.js';
  /* THE PRODUCT'S OWN MONTH, AND THE PRODUCT'S OWN MENU.
     This page drew its span as two bare `YYYY-MM` text boxes while `/db` used
     `$lib/DayField.svelte` and `/ingest` used `$lib/Picker.svelte`, so the one
     console showed three different date controls and `2016-08` in a form beside
     `Aug 2016` everywhere else. `DayField`'s own header records why that is a
     defect rather than an inconsistency, and `monthLabel` is what every other
     surface here already renders a store month with. A span is MONTH-granular,
     so `Picker` in single-choice mode is the control `/ingest` already proved,
     and `DayField` -- which is day-granular -- is not. */
  import Picker from '$lib/Picker.svelte';
  import { monthLabel } from '$lib/dates.js';
  import { rupee, group, exact } from '$lib/money.js';
  import * as prefix from '$lib/prefix.js';
  import { sweepOutcome, ledgerBlock } from '$lib/sweep.js';
  /* THE FEED'S OWN MASTER, ALREADY ON HAND. `+layout.svelte` calls
     `loadCatalogue(feeds.active)` on every feed change, so resolving a
     symbol's exchange and segment for a bar-file path costs this page NO
     request -- and resolving it is what stops `NSE`/`INDEX` being written
     here as literals the store can contradict. */
  import { catalogue } from '$lib/index.svelte.js';
  /* TRADINGVIEW'S OWN IDIOM FOR A METRIC IT CANNOT SHOW. See the component
     for why the padlock is borrowed and where the two meanings differ. */
  import Lock from '$lib/Lock.svelte';

  /* ====================================================================
     THE PAYLOAD
     ==================================================================== */

  /** @type {{ phase: 'loading'|'ready'|'failed', body: any, why: string }} */
  let load = $state({ phase: 'loading', body: null, why: '' });

  /** How many runs to ask for. The server clamps; this is what we request. */
  const LIMIT = 500;

  /**
   * The condition table, once, so a stored mask can be read as names.
   *
   * `/backtest.json` serves `mask_words` as six raw 64-bit words and no names:
   * decoding them there would repeat 370 rows of vocabulary on every run in
   * every response. The table is static for the life of a `vocab_version`, so
   * it is fetched once here and every mask the page ever shows is decoded
   * against it.
   *
   * NOT A HAND-KEPT COPY IN THIS FILE, deliberately. That would be two
   * vocabularies for one fact — correct the day it was written and wrong the
   * first time a bit is appended, and the failure is silent: a mask decoded
   * against a stale table names the WRONG conditions and looks exactly like an
   * answer.
   *
   * @type {{ phase: 'idle'|'ready'|'failed', version: number, bits: Map<number, {name: string, live: boolean}>, why: string }}
   */
  let vocab = $state({ phase: 'idle', version: 0, bits: new Map(), why: '' });

  async function fetchVocab() {
    try {
      const response = await ask_('/vocab.json');
      if (!response.ok) {
        // NAMED, NOT SWALLOWED. Without the table the page can still show the
        // raw words, and it must say WHY it is showing numbers instead of
        // names rather than looking like a run with no conditions.
        vocab = {
          phase: 'failed',
          version: 0,
          bits: new Map(),
          why:
            `/vocab.json answered ${response.status}. Masks below are shown as raw ` +
            `words because the condition table could not be read. A 404 means this ` +
            `page is newer than the running binary — the page comes off disk and the ` +
            `route does not.`
        };
        return;
      }
      const body = await response.json();
      const bits = new Map();
      for (const bit of body.bits ?? []) {
        bits.set(bit.i, { name: bit.name, live: bit.live });
      }
      vocab = { phase: 'ready', version: body.vocab_version ?? 0, bits, why: '' };
    } catch (why) {
      vocab = {
        phase: 'failed',
        version: 0,
        bits: new Map(),
        why: `The condition table could not be fetched: ${why instanceof Error ? why.message : String(why)}`
      };
    }
  }

  /**
   * The positions a mask has set, in ascending order.
   *
   * Six words of 64 bits, little-endian in word order: word 0 holds positions
   * 0..63, word 1 holds 64..127, and so on. `BigInt` and not `Number` because a
   * word is 64 bits and a JS number carries 53 — the words arrive as decimal
   * STRINGS for exactly that reason, and parsing one with `Number()` would set
   * the wrong bits without throwing.
   *
   * @param {string[]} words
   * @returns {number[]}
   */
  function positionsIn(words) {
    /** @type {number[]} */
    const out = [];
    (words ?? []).forEach((word, index) => {
      let value;
      try {
        value = BigInt(word);
      } catch {
        // A WORD THAT WILL NOT PARSE IS SKIPPED AND THE REST ARE READ. One
        // malformed word must not blank a combination that is otherwise
        // perfectly legible.
        return;
      }
      for (let bit = 0; bit < 64 && value !== 0n; bit += 1) {
        if ((value & 1n) === 1n) out.push(index * 64 + bit);
        value >>= 1n;
      }
    });
    return out;
  }

  async function fetchLedger() {
    load.phase = 'loading';
    try {
      const response = await ask_(`/backtest.json?limit=${LIMIT}`, { cache: 'no-store' });
      if (!response.ok && response.status !== 503) {
        // A NON-503 FAILURE IS THE SERVER, NOT THE CONFIGURATION. 503 still
        // carries a parseable body with the refusal in it, so it is read
        // rather than thrown away.
        // A 404 HERE HAS EXACTLY ONE CAUSE AND THE PAGE MUST NAME IT.
        //
        // The route is registered unconditionally in `server::router_serving`,
        // so a running binary either has it or predates it. There is no
        // configuration that removes it and no state that hides it. A 404
        // therefore means the process was started before the route existed —
        // and the page it serves comes off DISK, so the front end updates
        // while the binary does not, which is precisely the trap: the page
        // looks new and the API behind it is old.
        //
        // This cost a long session once: the operator's server had been up 36
        // hours, every rebuild of this page reached them instantly, and none
        // of the route did. The message said "the API refused the request",
        // which is true and useless. It now names the cause and the fix.
        load = {
          phase: 'failed',
          body: null,
          why:
            response.status === 404
              ? `/backtest.json is not a route on the running server, which means the API ` +
                `process was started before this route existed. RESTART THE API — the ` +
                `binary on disk already has it. Nothing is wrong with the store, the ` +
                `ledger or this page: the front end is served off disk and updates on ` +
                `every build, while the binary only changes when it is restarted, so this ` +
                `page can be hours newer than the server answering it.`
              : `/backtest.json answered ${response.status}. That is the API refusing the ` +
                `request itself, not the ledger being empty — the two are different facts ` +
                `and only one of them is fixable by sweeping something.`
        };
        return;
      }
      load = { phase: 'ready', body: await response.json(), why: '' };
    } catch (error) {
      load = {
        phase: 'failed',
        body: null,
        why:
          error instanceof Error
            ? error.message
            : 'The request for /backtest.json failed and threw a value that is not an Error.'
      };
    }
  }

  $effect(() => {
    untrack(() => fetchLedger());
    // ALONGSIDE, NOT AFTER. The table is static and the ledger is not, so
    // neither waits for the other — and a failed table still leaves a readable
    // page, which is why it is not awaited here.
    untrack(() => fetchVocab());
  });

  /* ====================================================================
     STARTING A SWEEP FROM HERE
     --------------------------------------------------------------------
     Until `POST /backtest/run` existed this page reported on work it
     could not cause: the operator read it, decided to try another rung,
     left for a terminal, and came back to refresh. Half a console.

     THE POLL IS THE SHAPE `/pull/run.json` ALREADY USES, and it is one
     lock take on the server — never a store read — so refreshing it
     every two seconds through an hour-long sweep costs the disk nothing.
     It stops the moment the run is no longer in flight, because a poll
     that outlives its subject is a request nobody is waiting for.
     ==================================================================== */

  /**
   * The span the Run control will send.
   *
   * EMPTY UNTIL THE CENSUS SAYS OTHERWISE. This held `2019-12` and
   * `2026-08` as literals; the store holds 2016-08 → 2026-08, so the
   * default threw away 40 of 121 months and every run it produced then
   * reported `81/81` with no hole. See the catalog block below. A span is
   * two of the nine terms in a run's identity, so it is seeded from what
   * is on disk or it is not seeded at all.
   */
  let ask = $state({ from: '', to: '' });
  /** @type {{ phase: 'idle'|'starting'|'running'|'done'|'failed', run: any, why: string }} */
  let sweep = $state({ phase: 'idle', run: null, why: '' });
  /** @type {ReturnType<typeof setTimeout> | null} */
  let pollAt = null;

  /** `YYYY-MM` as the two numbers the route wants, or null. */
  function months(text) {
    const m = /^(\d{4})-(\d{2})$/.exec(text ?? '');
    if (!m) return null;
    const year = Number(m[1]);
    const month = Number(m[2]);
    return month >= 1 && month <= 12 ? { year, month } : null;
  }

  async function pollSweep() {
    try {
      const response = await ask_(`/backtest/run.json`, { cache: 'no-store' });
      if (!response.ok) return;
      const body = await response.json();
      // ENDED IS NOT FINISHED, AND THIS TESTED ONLY THAT IT HAD ENDED.
      //
      // The branch was `if (run.in_flight) … else done`, which is two readings
      // of a payload that carries three. `cli::range_all` refuses the WHOLE
      // command when every rung refused, and such a run is not in flight —
      // exactly like one that swept. The green "Sweep finished" printed over a
      // ledger that gained no row. `sweepOutcome` is total over the payload and
      // is proved in `web/tests/sweep.test.js`; see its module comment. It also
      // absorbs the `no run at all` case this function used to answer inline.
      const next = sweepOutcome(body.running);
      sweep = next;
      if (next.phase === 'running') {
        pollAt = setTimeout(pollSweep, 2000);
        return;
      }
      // FINISHED: the ledger now has one more record, so the table is stale.
      // A REFUSAL DOES NOT RE-READ IT — nothing was appended, and re-reading
      // would redraw the same table under a red note as though it had changed.
      if (next.phase === 'done') fetchLedger();
    } catch (error) {
      sweep = {
        phase: 'failed',
        run: null,
        why: error instanceof Error ? error.message : String(error)
      };
    }
  }

  async function startSweep() {
    const from = months(ask.from);
    const to = months(ask.to);
    if (!from || !to) {
      sweep = {
        phase: 'failed',
        run: null,
        why: 'Both months must be written as YYYY-MM, and the month must be 01 to 12.'
      };
      return;
    }
    sweep = { phase: 'starting', run: null, why: '' };
    try {
      const response = await ask_('/backtest/run', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          feed: activeFeed,
          underlying: sweepSymbol,
          from_year: from.year,
          from_month: from.month,
          to_year: to.year,
          to_month: to.month,
          // SENT AHEAD OF BEING HONOURED, DELIBERATELY. `sweeprun::Asked`
          // parses `feed`, `underlying`, `from_*` and `to_*` and ignores
          // anything else, so these two are inert on today's server — and
          // the page says so under the button rather than letting the
          // operator believe a selection travelled that did not. The moment
          // the route reads them, it reads them from a client that has been
          // sending them all along.
          underlyings: [...pickedSymbols],
          rungs: [...pickedRungs]
        })
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok || body.accepted !== true) {
        sweep = {
          phase: 'failed',
          run: null,
          why: body.refusal ?? `The server answered ${response.status} and gave no reason.`
        };
        return;
      }
      sweep = { phase: 'running', run: null, why: '' };
      pollAt = setTimeout(pollSweep, 1200);
    } catch (error) {
      sweep = {
        phase: 'failed',
        run: null,
        why: error instanceof Error ? error.message : String(error)
      };
    }
  }

  /**
   * Which instrument a new sweep runs over.
   *
   * EMPTY UNTIL THE CENSUS SAYS OTHERWISE, and this was `'NIFTY'`. The
   * store also holds BANKNIFTY at the same 121 months and nine rungs, and
   * a literal here made it unreachable from this page entirely.
   *
   * IT IS DERIVED NOW, NOT SET. The chips own the choice; this is the one
   * instrument the run route can take today, read off the front of that
   * set. Keeping it as its own `$state` beside the set would be two
   * sources for one fact, and the first to drift would be the one the
   * request is built from.
   */
  const sweepSymbol = $derived([...pickedSymbols][0] ?? '');

  // A POLL MUST NOT OUTLIVE THE PAGE. Without this a navigation away leaves a
  // timer firing against a component that is gone.
  $effect(() => () => {
    if (pollAt !== null) clearTimeout(pollAt);
  });

  /* ====================================================================
     WHAT THE PAYLOAD SAYS
     ==================================================================== */

  /**
   * One ledger row, exactly the fields `api::backtest`'s `to_json` writes.
   *
   * MIRRORED FROM THE WRITER, not inferred from what this page happens to read.
   * The payload arrives as JSON so every field is `any` until something says
   * otherwise, and a shape written from the reader's side records what the page
   * uses rather than what the server sends — the two drift apart silently, and
   * the first symptom is a field that reads `undefined` on a row that has it.
   *
   * @typedef {object} Run
   * @property {number} index
   * @property {string} identity
   * @property {number} finished_micros
   * @property {string} feed
   * @property {string} underlying
   * @property {string} timeframe
   * @property {number} from_year
   * @property {number} from_month
   * @property {number} to_year
   * @property {number} to_month
   * @property {number} months_asked
   * @property {number} months_found
   * @property {boolean} whole_span
   * @property {number} bars
   * @property {number} min_hits
   * @property {number} combinations
   * @property {number} depth
   * @property {boolean} halted
   * @property {boolean} sealed
   * @property {number} trades
   * @property {number} pessimistic
   * @property {number} optimistic
   * @property {number} worst_trade
   * @property {number} max_drawdown
   * @property {number} winner_mae
   * @property {number} winner_mfe
   * @property {number} all_mae
   * @property {unknown} exit_rungs
   * @property {string[]} mask_words
   */

  /**
   * The whole `/backtest.json` body.
   *
   * @typedef {object} Ledger
   * @property {string} path
   * @property {number} version
   * @property {boolean} has_mask
   * @property {number} total
   * @property {number} scanned
   * @property {boolean} hit_scan_cap
   * @property {boolean} partial_tail
   * @property {number} max_runs
   * @property {boolean} halted
   * @property {number} unsealed
   * @property {Run | null} best_complete
   * @property {string | null} refusal
   * @property {Run[]} runs
   */

  /** @type {Ledger | null | undefined} */
  const ledger = $derived(load.body);
  /** Every run the server returned, newest first — the server ordered them. */
  const allRuns = $derived(ledger?.runs ?? []);
  /** The server's own refusal sentence, when it had one. */
  const refusal = $derived(ledger?.refusal ?? null);
  /**
   * The two versions, when this ledger cannot take a new run — else `null`.
   *
   * READ BEFORE THE BUTTON IS PRESSED, which is the whole value of it. The
   * append refuses on a version mismatch and the sweep only finds out after it
   * has run; this is the same fact, sixteen header bytes deep, at page load.
   */
  const blocked = $derived(ledgerBlock(ledger));

  /* ---- the feed rung -------------------------------------------------
     THE TOP BAR'S PICKER IS LIVE ON THIS PAGE, and this is what makes it
     live. `/backtest` is not in the layout's FEED_OWNED list, so the bar
     draws its picker here; a page that ignored it would leave a control
     on screen that changes nothing, which is the dead-end that list
     exists to prevent from the other direction.

     NOTHING IS HIDDEN SILENTLY. When the active feed has no runs the page
     says so AND says how many exist under other feeds AND offers the way
     out. A filter that empties a table without saying why is the fallback
     that hides a fact. ------------------------------------------------ */
  let everyFeed = $state(false);
  const activeFeed = $derived(feeds.active ?? '');
  const runs = $derived(
    everyFeed || !activeFeed ? allRuns : allRuns.filter((r) => r.feed === activeFeed)
  );
  const hiddenByFeed = $derived(allRuns.length - runs.length);

  /* ====================================================================
     WHAT THE STORE ACTUALLY HOLDS — the sweep form's only source of truth
     --------------------------------------------------------------------
     THE FORM USED TO OPEN ON `NIFTY`, `2019-12` AND `2026-08`, ALL THREE
     WRITTEN INTO THIS FILE. Measured against the census on 2026-08-26 the
     store holds **121 months, 2016-08 → 2026-08**, at all nine rungs, for
     NIFTY *and* BANKNIFTY. The hardcoded span asked for 81 of them.

     So the default silently discarded **40 months — a third of the pulled
     history** — and every run it produced then reported `81/81` and
     `span holes: 0`. Both true, and together they read as complete
     coverage of a window that was cut short by a literal. That is the
     failure §4 bans wearing a success's clothes: a number that is correct
     about the wrong question. BANKNIFTY was never reachable at all.

     `loadRungs` below already carries the rule this block applies to the
     form: *"Discovered, never declared. `store::path::Timeframe::KNOWN`
     can gain a rung tomorrow; a list written here would be a list the
     store contradicts."* The same is true of an instrument and of a span.

     COST. One request per feed at load, folded in ONE pass into a map
     keyed by instrument; every read after that is a map hit. That is
     `docs/07-o1-architecture.md` layer 12 applied here — D-0042 moved
     every ordering and filter into a build at load time for exactly this
     reason. The fold is O(census rows) once, never per keystroke and
     never per render.

     WHAT THIS PAGE STILL DOES NOT DECIDE: which instruments are legal to
     sweep. `CLAUDE.md` §1 names the engine surface and `costs::venue`
     enforces it, but no endpoint enumerates the two symbols — only
     `/universes.json` says how many there are. So the picker offers what
     the STORE holds and the SERVER refuses what it will not sweep, with
     its refusal printed verbatim. A list written here would be a fourth
     copy of the surface and the first one to go stale.
     ==================================================================== */

  /**
   * @typedef {{ name: string, months: number, from: string, to: string }} Rung
   * @typedef {{ leaf: string, full: string, months: number, from: string, to: string, rungs: Rung[] }} Held
   */

  /** @type {{ phase: 'idle'|'loading'|'ready'|'failed', why: string, held: Held[] }} */
  let catalog = $state({ phase: 'idle', why: '', held: [] });

  /**
   * The engine surface, IN THE SERVER'S OWN WORDS.
   *
   * The store holds three spot indices and the engine sweeps two of them —
   * `CLAUDE.md` §1 makes INDIAVIX reference-only. This page must not be the
   * fourth copy of that rule. `crates/api/src/coverage.rs` calls
   * `SpotTarget::Swept` "the engine surface (`CLAUDE.md` §1, two
   * `(exchange, symbol)` pairs)", so the server can enumerate it — but
   * `/universes.json` publishes the swept target with `universe: null` and
   * only a prose `note`, so the two symbols are NOT on the wire as data.
   *
   * What is on the wire is that sentence and a `matched` count, and those
   * are shown verbatim. The page therefore states the surface without
   * deciding it, and the run route stays the authority that refuses.
   *
   * The alternative was to read `/instruments.json` and treat
   * `universes: ["index","fno"]` as "swept" — it selects exactly NIFTY and
   * BANKNIFTY today. It is also a DIFFERENT FACT that happens to coincide:
   * F&O membership is not the engine surface, and a rule that is right by
   * coincidence is the invention §3 rule 1 forbids.
   *
   * @type {{ label: string, note: string, matched: number } | null}
   */
  let sweptSurface = $state(null);

  /** Has the operator edited the span themselves? Then never overwrite it. */
  let spanTouched = $state(false);
  /** Has the operator picked an instrument themselves? Same rule. */
  let symbolTouched = $state(false);

  /* ====================================================================
     WHAT THE BRUTE FORCE WILL ACTUALLY RUN OVER
     --------------------------------------------------------------------
     Instruments and rungs are both SETS, chosen by the operator. A single
     choice is a set of one, so one control shape covers both and there is
     no mode to switch between.

     `sweepSymbol` above stays as the ONE instrument the run route can take
     today — `sweeprun::Asked` carries `feed`, `underlying`, `from` and `to`
     and nothing plural — and it is derived from this set rather than being
     a second source of truth.

     TWO THINGS THIS PAGE CANNOT DERIVE, AND BOTH ARE SERVER GAPS:

     * **Which rungs the engine sweeps.** `cli::EVERY_RUNG` is EIGHT --
       1min…60min -- and `1day` is not among them: `cli::descend` refuses it
       in as many words, and `range_all`'s own banner reads "ALL EIGHT
       INTRADAY RUNGS". The store holds nine. That const is private to
       `crates/cli` and no endpoint publishes it, so a page that wanted to
       grey out `1day` would have to keep a second copy of the list --
       which is the two-vocabularies failure `CLAUDE.md` §5 exists to
       refuse. Until it is on the wire, `1day` is offered, and selecting it
       is answered by the route rather than pre-empted here.

     * **Which instruments it sweeps.** Same shape, same reason, already
       recorded on `sweptSurface` above.
     ==================================================================== */

  /** @type {Set<string>} instruments the run will cover. */
  let pickedSymbols = $state(new Set());
  /** @type {Set<string>} rungs the run will cover. */
  let pickedRungs = $state(new Set());

  /**
   * Toggle one key in a chosen set, and never allow the empty set.
   *
   * A SWEEP OVER NOTHING IS NOT A SWEEP, so the last selected chip does not
   * turn itself off. That is a refusal the control can make structurally
   * rather than a validation message it has to raise afterwards — the same
   * argument §6 makes for a depth parameter that cannot be set wrongly.
   *
   * @param {Set<string>} set
   * @param {string} key
   * @returns {Set<string>} a NEW set, because `$state` tracks identity
   */
  function toggled(set, key) {
    const next = new Set(set);
    if (next.has(key)) {
      if (next.size === 1) return next;
      next.delete(key);
    } else {
      next.add(key);
    }
    return next;
  }

  /**
   * Fold the census into one entry per instrument, with its rungs and its
   * real month bounds.
   *
   * `YYYY-MM` compares correctly as a string, so the bounds need no date
   * parsing — which is also why a malformed month cannot throw here.
   *
   * @param {string} feed
   */
  async function loadCatalog(feed) {
    if (!feed) {
      catalog = { phase: 'idle', why: '', held: [] };
      return;
    }
    catalog = { phase: 'loading', why: '', held: [] };
    try {
      const response = await ask_(`/store.json?feed=${encodeURIComponent(feed)}`, {
        cache: 'no-store',
        ms: 30_000
      });
      if (!response.ok) {
        catalog = {
          phase: 'failed',
          why:
            `/store.json answered ${response.status}, so this form cannot say what the store ` +
            `holds. The span and the instrument are two of the nine terms in a run's identity, ` +
            `so neither is defaulted — naming a run after a guess is the invention §3 rule 1 forbids.`,
          held: []
        };
        return;
      }
      const rows = await response.json();
      /** @type {Map<string, { leaf: string, full: string, months: Set<string>, from: string, to: string, rungs: Map<string, Rung> }>} */
      const byLeaf = new Map();
      for (const row of rows ?? []) {
        const full = String(row.instrument ?? '');
        // The census names an instrument `NSE-INDEX-NIFTY`; the ledger and
        // the run route both name it `NIFTY`. Matching on the LAST segment
        // is what joins them without this page holding an exchange or a
        // segment literal — the same join `loadRungs` already makes.
        const leaf = full.split('-').pop() ?? '';
        const month = String(row.month ?? '');
        const rung = String(row.timeframe ?? '');
        if (!leaf || !month || !rung) continue;
        let held = byLeaf.get(leaf);
        if (!held) {
          held = { leaf, full, months: new Set(), from: month, to: month, rungs: new Map() };
          byLeaf.set(leaf, held);
        }
        held.months.add(month);
        if (month < held.from) held.from = month;
        if (month > held.to) held.to = month;
        let r = held.rungs.get(rung);
        if (!r) {
          r = { name: rung, months: 0, from: month, to: month };
          held.rungs.set(rung, r);
        }
        r.months += 1;
        if (month < r.from) r.from = month;
        if (month > r.to) r.to = month;
      }
      // The surface, best effort and never blocking the catalog: a form that
      // cannot say which two are swept is worse than one that cannot say it
      // YET, and neither is a reason to withhold the span.
      untrack(() => loadSurface(feed));
      const held = [...byLeaf.values()]
        .map((h) => ({
          leaf: h.leaf,
          full: h.full,
          months: h.months.size,
          // THE MONTHS THEMSELVES, so the span control can offer what EXISTS
          // rather than accept anything that parses. `YYYY-MM` sorts correctly
          // as a string, so this needs no comparator and no date object.
          monthList: [...h.months].sort(),
          from: h.from,
          to: h.to,
          rungs: [...h.rungs.values()].sort((a, b) => byRung(a.name, b.name))
        }))
        // Widest history first — the instrument with the most to sweep is
        // the one an operator most likely wants, and it is a FACT about the
        // store rather than an opinion written here.
        .sort((a, b) => b.months - a.months || a.leaf.localeCompare(b.leaf));
      catalog = { phase: 'ready', why: '', held };
    } catch (error) {
      catalog = {
        phase: 'failed',
        why:
          `The census could not be read: ${error instanceof Error ? error.message : String(error)}. ` +
          `Nothing is defaulted in its absence.`,
        held: []
      };
    }
  }

  /**
   * Read the swept target off `/universes.json` and keep its own words.
   *
   * @param {string} feed
   */
  async function loadSurface(feed) {
    try {
      const response = await ask_(`/universes.json?feed=${encodeURIComponent(feed)}`, {
        cache: 'no-store',
        ms: 15_000
      });
      if (!response.ok) {
        sweptSurface = null;
        return;
      }
      const body = await response.json();
      const target = (body?.targets ?? []).find((t) => t?.target === 'swept');
      sweptSurface = target
        ? {
            label: String(target.label ?? 'Swept'),
            note: String(target.note ?? ''),
            matched: Number(target.matched ?? 0)
          }
        : null;
    } catch {
      // An absent surface costs the SENTENCE and nothing else. The span, the
      // rungs and the run all still work, and the route still refuses what it
      // will not sweep.
      sweptSurface = null;
    }
  }

  /** The entry for whatever instrument is selected, or null. O(1) per read. */
  const heldNow = $derived(catalog.held.find((h) => h.leaf === sweepSymbol) ?? null);

  /** Re-read the census whenever the feed changes. One request per feed. */
  $effect(() => {
    const feed = activeFeed;
    untrack(() => loadCatalog(feed));
  });

  /**
   * Seed the form from the store, ONCE, and never over the operator.
   *
   * A default that keeps reasserting itself is a control the operator does
   * not own. `spanTouched` and `symbolTouched` latch on the first edit and
   * this effect then leaves both alone for the rest of the session.
   */
  $effect(() => {
    if (catalog.phase !== 'ready' || catalog.held.length === 0) return;
    /* READ TRACKED, WRITE UNTRACKED — and the first version read BOTH inside
       `untrack`, which made this effect depend on the catalog alone. Switching
       instrument then changed the rungs and left the span from the PREVIOUS
       instrument sitting in the form: picking INDIAVIX after narrowing
       BANKNIFTY to Jun 2024 kept Jun 2024, on a different instrument, with the
       shortfall banner still counting against the old one. Naming the two
       dependencies here is what makes a switch re-seed. */
    const symbol = sweepSymbol;
    const touched = spanTouched;
    untrack(() => {
      if (!symbolTouched && !catalog.held.some((h) => h.leaf === symbol)) {
        // THE NEWEST RUN'S INSTRUMENT FIRST, when the store still holds it.
        // That is what the operator was last looking at, and it beats any
        // ordering this page could invent. Only when the ledger is empty --
        // or names something no longer on disk -- does the widest history
        // win, and that is a fact about the store rather than a preference.
        const last = allRuns[0]?.underlying ?? '';
        const seen = catalog.held.find((h) => h.leaf === last);
        pickedSymbols = new Set([seen ? seen.leaf : catalog.held[0].leaf]);
      }
      const held = catalog.held.find((h) => h.leaf === ([...pickedSymbols][0] ?? ''));
      if (held && !touched) {
        ask.from = held.from;
        ask.to = held.to;
      }
      // EVERY RUNG THE INSTRUMENT HOLDS, until the operator says otherwise.
      // The run route sweeps them all today, so an empty or stale selection
      // would show fewer than the run covers -- which is the one direction
      // this control must never be wrong in.
      if (held && pickedRungs.size === 0) {
        pickedRungs = new Set(held.rungs.map((r) => r.name));
      }
    });
  });

  /**
   * The exact `cli` line that would fill an empty ledger with the span this
   * form is showing. Built, never written — see the empty-state comment.
   */
  const sweepCommand = $derived.by(() => {
    const from = months(ask.from);
    const to = months(ask.to);
    if (!activeFeed || !heldNow || !from || !to) return '';
    return `cli range-all ${activeFeed} ${heldNow.leaf} ${from.year} ${from.month} ${to.year} ${to.month} 500`;
  });

  /**
   * The span menus' rows — every month this instrument actually holds.
   *
   * A MONTH THE STORE DOES NOT HOLD IS NOT OFFERED, and one that would
   * invert the span is offered DEAD with the reason on it rather than
   * omitted. That is `Picker`'s own rule — "a row drawn dead in its own
   * place says *this exists and you cannot have it*; the same row omitted
   * says *this does not exist*" — and here the second sentence would be a
   * lie, because the month is on disk and it is only this end of the span
   * that cannot take it.
   *
   * @param {'from'|'to'} end
   */
  function monthRows(end) {
    if (!heldNow) return [];
    const other = end === 'from' ? ask.to : ask.from;
    return heldNow.monthList.map((m) => {
      const inverts =
        other !== '' && (end === 'from' ? m > other : m < other);
      return {
        key: m,
        name: monthLabel(m) ?? m,
        detail: m,
        disabled: inverts,
        why: inverts
          ? end === 'from'
            ? `later than the last month of the span, ${monthLabel(other) ?? other}.`
            : `earlier than the first month of the span, ${monthLabel(other) ?? other}.`
          : undefined
      };
    });
  }

  /** Put the span back to everything the store holds for this instrument. */
  function useWholeSpan() {
    if (!heldNow) return;
    ask.from = heldNow.from;
    ask.to = heldNow.to;
    spanTouched = false;
  }

  /**
   * Months the asked-for span covers, inclusive, or null when either end is
   * malformed. Arithmetic on year·12 + month — no calendar, no library.
   */
  const askedMonths = $derived.by(() => {
    const from = months(ask.from);
    const to = months(ask.to);
    if (!from || !to) return null;
    const n = (to.year * 12 + to.month - (from.year * 12 + from.month)) + 1;
    return n > 0 ? n : null;
  });

  /** Is the asked-for span narrower than what the store holds? A FACT. */
  const spanShortfall = $derived.by(() => {
    if (!heldNow || askedMonths === null) return 0;
    return heldNow.months - askedMonths;
  });

  /* ---- the summary ---------------------------------------------------- */

  /**
   * Whether a row's bytes can be vouched for by its own `blake3` seal.
   *
   * `=== false` and NOT `!r.sealed`, deliberately. A payload from an API
   * that predates the seal carries no `sealed` field at all, and
   * `!undefined` is `true` — which would paint every row on the page as
   * damaged the instant this page is newer than the binary serving it.
   * That exact page-ahead-of-binary disagreement has already cost this
   * console a day. An absent flag means NOT KNOWN, and not-known must never
   * render as damaged.
   *
   * The failure mode this protects against is worth naming: crying wolf on
   * a healthy ledger is not a smaller error than missing a damaged one. An
   * alarm that fires on every row teaches the operator to ignore it, and
   * then the real one goes unread too.
   */
  const trustworthy = (r) => r.sealed !== false;

  /**
   * The rows eligible to be RANKED: ran to extinction, and readable back.
   *
   * Two different disqualifications, and they are not the same fault. A
   * halted run is honest and incomplete — the engine stopped it. An
   * unsealed run is bytes that parse but cannot be vouched for, and it may
   * be neither halted nor complete but noise that happens to be the right
   * length.
   */
  const completeRuns = $derived(runs.filter((r) => !r.halted && trustworthy(r)));
  const haltedRuns = $derived(runs.filter((r) => r.halted));
  const unsealedRuns = $derived(runs.filter((r) => !trustworthy(r)));
  const holedRuns = $derived(runs.filter((r) => !r.whole_span));

  /**
   * The best COMPLETE run, by worst-case total.
   *
   * Recomputed here rather than taken from the payload's `best_complete`
   * index, because that index is over the whole ledger and this page may be
   * showing one feed. Same rule, same tie-break: halted excluded, ranked on
   * `pessimistic`, ties to the lower index.
   */
  const best = $derived.by(() => {
    let winner = null;
    for (const run of completeRuns) {
      if (
        winner === null ||
        run.pessimistic > winner.pessimistic ||
        (run.pessimistic === winner.pessimistic && run.index < winner.index)
      ) {
        winner = run;
      }
    }
    return winner;
  });

  /* ====================================================================
     RUNGS — parsed, never declared
     ==================================================================== */

  /** The rung name grammar the store's own timeframe table follows. */
  const RUNG_NAME = /^(\d+)(sec|min|hour|day)$/;
  /** @type {Record<string, number>} */
  const UNIT_SECONDS = { sec: 1, min: 60, hour: 3600, day: 86_400 };

  /**
   * A rung name as seconds, so nine of them can be put in time order without
   * this file holding a list the store can contradict.
   *
   * Returns `null` for a name this parser does not recognise, and a null sorts
   * LAST under its own raw name rather than being dropped. A rung the store
   * grows tomorrow appears here tomorrow, in the right place if its name
   * follows the pattern and at the end if it does not — never missing.
   *
   * @param {string} name
   * @returns {number | null}
   */
  function rungSeconds(name) {
    const m = (name ?? '').match(RUNG_NAME);
    if (!m) return null;
    const n = Number(m[1]);
    const unit = UNIT_SECONDS[m[2]];
    return Number.isFinite(n) && unit ? n * unit : null;
  }

  /** @param {string} a @param {string} b */
  function byRung(a, b) {
    const sa = rungSeconds(a);
    const sb = rungSeconds(b);
    if (sa === null && sb === null) return a.localeCompare(b);
    if (sa === null) return 1;
    if (sb === null) return -1;
    return sa - sb;
  }

  /**
   * Support as parts per thousand of the bars swept.
   *
   * **THE RATIO IS THE COMPARABLE QUANTITY, NOT THE COUNT**, and grouping on
   * the count was a real bug this page shipped with for one screenshot. A
   * `range-all` sweep holds the support PERCENTAGE constant across rungs, so
   * the absolute `min_hits` necessarily differs at every rung because the bar
   * count does. Measured on the store's own ledger: 4,324/21,620 at 30min,
   * 2,328/11,643 at 60min and 334/1,671 at 1day are all 20.0% — three views of
   * one question — and keying on `min_hits` split them into three groups of
   * one, which is exactly the comparison this section exists to make.
   *
   * Per-thousand rather than per-cent so two genuinely different thresholds a
   * tenth of a percent apart stay apart. `bars === 0` yields `-1`, a value no
   * real ratio takes, so a degenerate run groups with other degenerate runs
   * rather than dividing by zero.
   *
   * @param {any} r
   */
  const supportPerMille = (r) =>
    r.bars > 0 ? Math.round((r.min_hits / r.bars) * 1000) : -1;

  /**
   * One key per comparable question: same feed, same instrument, same span,
   * same support RATIO. Only runs sharing all four are comparable across rungs.
   *
   * @param {any} r
   */
  const groupKey = (r) =>
    `${r.feed} ${r.underlying} ${r.from_year}-${r.from_month} ${r.to_year}-${r.to_month} ${supportPerMille(r)}`;

  /**
   * The rung comparison: one entry per comparable span, each holding the runs
   * at each rung. **One pass**, into a Map — not a nested compare.
   */
  const rungGroups = $derived.by(() => {
    /** @type {Map<string, any>} */
    const byKey = new Map();
    for (const run of runs) {
      const key = groupKey(run);
      let g = byKey.get(key);
      if (!g) {
        g = {
          key,
          feed: run.feed,
          underlying: run.underlying,
          from: `${run.from_year}-${String(run.from_month).padStart(2, '0')}`,
          to: `${run.to_year}-${String(run.to_month).padStart(2, '0')}`,
          perMille: supportPerMille(run),
          monthsAsked: run.months_asked,
          rungs: new Map()
        };
        byKey.set(key, g);
      }
      // NEWEST WINS PER RUNG. The payload is newest first, so the first run
      // seen at a rung is the most recent one; a re-sweep of the same rung
      // should not show the older answer.
      if (!g.rungs.has(run.timeframe)) g.rungs.set(run.timeframe, run);
    }
    // Order groups by how many rungs they carry — the fullest comparison is
    // the one an operator opened this page for.
    return [...byKey.values()]
      .map((g) => {
        const names = [...g.rungs.keys()].sort(byRung);
        const present = names.map((n) => g.rungs.get(n));
        const complete = present.filter((r) => !r.halted && trustworthy(r));
        // ONE SHARED SCALE, so bar heights are comparable across the row.
        // Taken over the absolute value so a losing rung is as legible as a
        // winning one.
        const scale = Math.max(1, ...present.map((r) => Math.abs(r.pessimistic)));
        let leader = null;
        for (const r of complete) {
          if (leader === null || r.pessimistic > leader.pessimistic) leader = r;
        }
        return {
          ...g,
          names,
          present,
          scale,
          leader,
          // COUNTED APART, because they disqualify a rung for different
          // reasons and the operator's next move differs: a halted rung
          // wants a larger budget, an unsealed one wants the run repeated.
          haltedCount: present.filter((r) => r.halted).length,
          unsealedCount: present.filter((r) => !trustworthy(r)).length
        };
      })
      .sort(
        (a, b) => b.present.length - a.present.length || a.underlying.localeCompare(b.underlying)
      );
  });

  /* ====================================================================
     THE FILTER — O(1) per keystroke
     ==================================================================== */

  let query = $state('');

  /**
   * The prefix index, rebuilt only when the run set changes.
   *
   * The searchable text here is the instrument, the feed and the rung joined,
   * so typing `bank`, `zer` or `15m` all narrow. Built once per payload,
   * probed per keystroke — never a scan, never a request per character.
   */
  const searchable = $derived(
    runs.map((r) => ({ ...r, id: `${r.underlying} ${r.feed} ${r.timeframe}`.toLowerCase() }))
  );
  const byPrefix = $derived(prefix.build(searchable));
  const matched = $derived.by(() => {
    const typed = query.trim().toLowerCase();
    if (!typed) return searchable;
    return prefix.probe(byPrefix, searchable, typed);
  });

  /* ====================================================================
     SORTING
     ==================================================================== */

  /**
   * The table's columns, in order, with the record field each one sorts on.
   *
   * **Declared once and looped, rather than nine hand-written header cells.**
   * A column whose label and sort key are written in two places is a column
   * that can sort on something it does not display, and `aria-sort` written
   * nine times is `aria-sort` correct eight times. `key: null` marks a column
   * that carries no single orderable value — the span is two months, and
   * sorting on "2019-12 → 2026-08" as a string would order by the first digit.
   *
   * `why` becomes the header's tooltip, because "Worst" and "Best" are the two
   * labels on this page most likely to be read backwards.
   */
  const COLUMNS = [
    { label: 'Instrument', key: 'underlying', num: false, why: 'The index the sweep ran over' },
    { label: 'Rung', key: 'timeframe', num: false, why: 'The SIGNAL timeframe. Execution is always one-minute.' },
    { label: 'Span', key: null, num: false, why: '' },
    {
      label: 'Coverage',
      key: 'months_found',
      num: false,
      why: 'Months the store actually held, against the months the span asked for'
    },
    {
      label: 'Complete',
      key: 'halted',
      num: false,
      why: 'Whether the ladder ran to extinction. A halted run is not comparable with a complete one.'
    },
    { label: 'Trades', key: 'trades', num: true, why: 'Round trips the chosen combination took' },
    {
      label: 'Worst',
      key: 'pessimistic',
      num: true,
      why: 'Total under adverse-extreme fills. THIS is what selection ranks on.'
    },
    {
      label: 'Best',
      key: 'optimistic',
      num: true,
      why: 'Total under open fills. The flattering number — never the one ranked on.'
    },
    { label: 'Finished', key: 'finished_micros', num: false, why: 'When the run was recorded' }
  ];

  /** @type {{ col: string, desc: boolean }} */
  let sort = $state({ col: 'finished_micros', desc: true });

  /** @param {string} col */
  function sortBy(col) {
    if (sort.col === col) sort = { col, desc: !sort.desc };
    else sort = { col, desc: true };
  }

  const sorted = $derived.by(() => {
    const rows = [...matched];
    const { col, desc } = sort;
    rows.sort((a, b) => {
      const av = a[col];
      const bv = b[col];
      let d;
      if (typeof av === 'string') d = av.localeCompare(bv);
      else if (typeof av === 'boolean') d = Number(av) - Number(bv);
      else d = (av ?? 0) - (bv ?? 0);
      return desc ? -d : d;
    });
    return rows;
  });

  /* ====================================================================
     WINDOWING — O(1) DOM regardless of run count
     ==================================================================== */

  const ROW_H = 44;
  const OVERSCAN = 6;
  let scrollTop = $state(0);
  let viewportH = $state(560);

  const firstVisible = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
  const visibleCount = $derived(Math.ceil(viewportH / ROW_H) + OVERSCAN * 2);
  const windowed = $derived(sorted.slice(firstVisible, firstVisible + visibleCount));
  const spacerH = $derived(sorted.length * ROW_H);

  /** @param {Event} e */
  function onScroll(e) {
    const el = /** @type {HTMLElement} */ (e.currentTarget);
    scrollTop = el.scrollTop;
    viewportH = el.clientHeight;
  }

  /* ====================================================================
     THE DRILL-DOWN
     ==================================================================== */

  /** @type {number | null} */
  let openIndex = $state(null);
  const openRun = $derived(runs.find((r) => r.index === openIndex) ?? null);

  /** @param {any} run */
  function toggle(run) {
    openIndex = openIndex === run.index ? null : run.index;
  }

  /* ====================================================================
     THE PRICE THE RUN WAS SWEPT OVER — TradingView's own library
     --------------------------------------------------------------------
     THE BARS ARE REAL AND THE TRADES ARE NOT DRAWN, and the second half
     of that sentence is the important one.

     `/bars/window.json` serves the actual OHLCV the sweep read, at the
     run's own instrument, rung and span — so this chart is the price
     series the result was computed against, not an illustration of one.
     The store holds 81 month files for this NIFTY span and the ledger
     record says `months_found: 81`; the chart reads the same files.

     What CANNOT be drawn on it is entries and exits. The ledger records
     `trades: 5093` as a COUNT and keeps no trade list, so there is no
     timestamp to put a marker at. Drawing markers would be the invention
     CLAUDE.md §3 rule 1 bans, and a chart with plausible markers is far
     worse than one with none: it would look like the answer to "when did
     it trade", which nothing on disk can answer.
     ==================================================================== */

  /**
   * `crates/api/src/bars.rs`'s `MAX_WINDOW_LIMIT`, asked for explicitly.
   *
   * The route's own default is 200 and its ceiling is 1,000; asking for more
   * is clamped, not refused. It is NOT raised to cover a whole span, and that
   * is deliberate: `/db` reads the same route, so the ceiling is a shared cost
   * decision and not this page's to change on its own.
   *
   * The consequence is stated on screen rather than hidden. 81 months of
   * 30-minute bars is 21,620 records and this draws the newest 1,000 of them,
   * so the chart is a WINDOW on the span the run swept — the same contract the
   * ledger's own `hit_scan_cap` keeps, for the same reason.
   */
  const MAX_WINDOW_LIMIT = 1000;

  /** @type {{ phase: 'idle'|'loading'|'ready'|'failed', bars: any[], total: number, months_read: number, months_missing: number, why: string }} */
  let series = $state({
    phase: 'idle',
    bars: [],
    total: 0,
    months_read: 0,
    months_missing: 0,
    why: ''
  });

  /**
   * The exchange and segment for a symbol, from the feed's own master.
   *
   * **Resolved, never assumed.** The ledger stores `underlying` alone —
   * `NIFTY` — and a bar file's path needs `NSE/INDEX/NIFTY`. Writing those two
   * segments as literals here would be a hardcoded instrument the store can
   * contradict the moment a second exchange is swept. The layout already loads
   * this catalogue for the active feed, so the join costs no request.
   *
   * @param {string} symbol
   */
  function place(symbol) {
    const row = catalogue.rows.find((r) => r.symbol === symbol);
    return row ? { exchange: row.exchange, segment: row.segment } : null;
  }

  /** @param {any} run */
  async function loadSeries(run, rung) {
    const at = place(run.underlying);
    if (!at) {
      series = {
        phase: 'failed',
        bars: [],
        total: 0,
        months_read: 0,
        months_missing: 0,
        why: catalogue.ready
          ? `${run.underlying} is not in ${catalogue.feed}'s instrument master, so the ` +
            `exchange and segment its bar files are filed under cannot be resolved. The run ` +
            `is still shown above — only its price series cannot be located.`
          : `The instrument master for this feed has not loaded yet, so ${run.underlying}'s ` +
            `exchange and segment are not known. Nothing was guessed.`
      };
      return;
    }
    series = { phase: 'loading', bars: [], total: 0, months_read: 0, months_missing: 0, why: '' };
    const from = `${run.from_year}-${String(run.from_month).padStart(2, '0')}`;
    const to = `${run.to_year}-${String(run.to_month).padStart(2, '0')}`;
    const url =
      `/bars/window.json?feed=${encodeURIComponent(run.feed)}` +
      `&exchange=${encodeURIComponent(at.exchange)}&segment=${encodeURIComponent(at.segment)}` +
      `&symbol=${encodeURIComponent(run.underlying)}` +
      `&timeframe=${encodeURIComponent(rung)}&from=${from}&to=${to}` +
      `&limit=${MAX_WINDOW_LIMIT}`;
    try {
      // 30 s rather than the default 15: 81 months of 30-minute bars is 21,620
      // records off a cold page cache, and giving up on a read that is working
      // would report a wedged server that is not one.
      const response = await ask_(url, { cache: 'no-store', ms: 30_000 });
      if (!response.ok) {
        series = {
          phase: 'failed',
          bars: [],
          total: 0,
          months_read: 0,
          months_missing: 0,
          why: `The bar window answered ${response.status}. The run's own figures above are unaffected — they were computed when the sweep ran, not now.`
        };
        return;
      }
      const body = await response.json();
      // ASCENDING, BECAUSE THE LIBRARY REQUIRES IT AND THE ROUTE DOES NOT
      // PROMISE IT. `/bars/window.json` answers newest first, which is right
      // for a table and wrong for a time axis; lightweight-charts throws on
      // unordered data rather than drawing it wrong.
      const bars = [...(body.bars ?? [])].sort((a, b) => a.t - b.t);
      series = {
        phase: 'ready',
        bars,
        total: body.total ?? bars.length,
        months_read: body.months_read ?? 0,
        months_missing: body.months_missing ?? 0,
        why: ''
      };
    } catch (error) {
      series = {
        phase: 'failed',
        bars: [],
        total: 0,
        months_read: 0,
        months_missing: 0,
        why: error instanceof Error ? error.message : String(error)
      };
    }
  }

  // OPENING A RUN RESETS THE CHART TO THAT RUN'S OWN RUNG. Leaving the
  // previous run's rung selected would draw one run's figures beside another
  // run's price, which is the stale-value shape §4 bans.
  $effect(() => {
    const run = openRun;
    if (!run) return;
    untrack(() => {
      chartRung = run.timeframe;
      storeRungs = [];
      loadRungs(run);
    });
  });

  // The series follows the open run AND the selected rung, and is dropped when
  // no run is open.
  $effect(() => {
    const run = openRun;
    if (!run) {
      series = { phase: 'idle', bars: [], total: 0, months_read: 0, months_missing: 0, why: '' };
      return;
    }
    const rung = chartRung || run.timeframe;
    void catalogue.ready; // re-resolve once the master lands
    untrack(() => {
      loadSeries(run, rung);
      loadBenchmark(run, rung);
    });
  });

  /* ====================================================================
     THE CHART — a terminal panel, not a thumbnail
     --------------------------------------------------------------------
     Everything below is chrome a trading terminal has and a small
     embedded chart does not: a live OHLC legend bound to the crosshair, a
     rung switcher over the rungs the STORE actually holds, range presets,
     and a stats strip folded from the bars on screen.

     Every number in it comes from the bars. There is no indicator, no
     overlay and no marker that is not a value in the file — an indicator
     computed here would be a second implementation of something
     `crates/indicators` already owns, and a marker would be the invented
     figure §3 rule 1 bans.
     ==================================================================== */

  /** @type {HTMLElement | null} */
  let chartHost = $state(null);
  let chartError = $state('');
  /** The bar under the crosshair, for the legend. Null when the pointer is off. */
  let ohlc = $state(null);
  /** Which rung the CHART shows. Starts at the run's own and is switchable. */
  let chartRung = $state('');
  /** Every rung the store holds for this instrument, newest census first. */
  let storeRungs = $state([]);

  /**
   * The rungs the store actually holds for one instrument.
   *
   * **Discovered, never declared.** `store::path::Timeframe::KNOWN` can gain a
   * rung tomorrow; a list written here would be a list the store contradicts.
   * `/store.json` is the census `/db` already reads — one row per
   * (instrument, month, rung) — so the rung set is a fold over it.
   *
   * @param {any} run
   * @param {string} rung the timeframe to draw, which is not always the run’s own
   */
  async function loadRungs(run) {
    try {
      const response = await ask_(`/store.json?feed=${encodeURIComponent(run.feed)}`, {
        cache: 'no-store',
        ms: 30_000
      });
      if (!response.ok) return;
      const rows = await response.json();
      /** @type {Map<string, number>} */
      const months = new Map();
      for (const row of rows ?? []) {
        // The census names an instrument `NSE-INDEX-NIFTY`; the ledger names
        // it `NIFTY`. Matching on the LAST segment is what joins them without
        // this page holding an exchange or a segment literal.
        const leaf = String(row.instrument ?? '').split('-').pop();
        if (leaf !== run.underlying) continue;
        months.set(row.timeframe, (months.get(row.timeframe) ?? 0) + 1);
      }
      storeRungs = [...months.entries()]
        .map(([name, count]) => ({ name, months: count }))
        .sort((a, b) => byRung(a.name, b.name));
    } catch {
      // A census that will not load costs the SWITCHER and nothing else: the
      // chart still draws the run's own rung, which is the one that matters.
      storeRungs = [];
    }
  }

  /* ====================================================================
     BUY & HOLD — TradingView's headline comparison, and ours was missing
     --------------------------------------------------------------------
     Every strategy report worth reading answers "did this beat simply
     holding the thing", and until this the page could not. It is the one
     TradingView metric this ledger does NOT carry that is nevertheless
     computable here, because the bars are on disk: close of the span's
     first bar against close of its last is buy-and-hold, exactly as
     TradingView defines it.

     IT NEEDS ITS OWN REQUEST, and the reason is the 1,000-bar ceiling.
     `series` holds the NEWEST 1,000 bars of the span, so its first bar is
     three weeks old, not seven years. Buy-and-hold over the visible window
     is a different and much smaller number than buy-and-hold over the run,
     and quietly using the first would understate the benchmark the run is
     measured against — flattering the strategy, which is the direction
     this page must never err in.

     ONE UNIT, AND IT IS STATED. The ledger records totals in paisa of
     index points with no position size and no capital, so the comparison
     is one unit of the index against one unit held. A percentage return
     would need initial capital, which nothing on disk carries.
     ==================================================================== */

  /** @type {{ phase: 'idle'|'loading'|'ready'|'failed', open: number, close: number, why: string }} */
  let bench = $state({ phase: 'idle', open: 0, close: 0, why: '' });

  /**
   * The close of the span's FIRST bar, fetched from its first month alone.
   *
   * @param {any} run
   * @param {string} rung
   */
  async function loadBenchmark(run, rung) {
    const at = place(run.underlying);
    if (!at) {
      bench = { phase: 'idle', open: 0, close: 0, why: '' };
      return;
    }
    bench = { phase: 'loading', open: 0, close: 0, why: '' };
    const first = `${run.from_year}-${String(run.from_month).padStart(2, '0')}`;
    try {
      const response = await ask_(
        `/bars/window.json?feed=${encodeURIComponent(run.feed)}` +
          `&exchange=${encodeURIComponent(at.exchange)}&segment=${encodeURIComponent(at.segment)}` +
          `&symbol=${encodeURIComponent(run.underlying)}` +
          `&timeframe=${encodeURIComponent(rung)}&from=${first}&to=${first}` +
          `&limit=${MAX_WINDOW_LIMIT}`,
        { cache: 'no-store', ms: 30_000 }
      );
      if (!response.ok) {
        bench = {
          phase: 'failed',
          open: 0,
          close: 0,
          why: `The span's first month answered ${response.status}, so buy-and-hold has no starting price.`
        };
        return;
      }
      const body = await response.json();
      const bars = [...(body.bars ?? [])].sort((a, b) => a.t - b.t);
      if (bars.length === 0) {
        bench = {
          phase: 'failed',
          open: 0,
          close: 0,
          why: `The store holds no ${rung} bars for ${first}, the span's first month, so there is no price to start the comparison from.`
        };
        return;
      }
      bench = { phase: 'ready', open: bars[0].c, close: 0, why: '' };
    } catch (error) {
      bench = {
        phase: 'failed',
        open: 0,
        close: 0,
        why: error instanceof Error ? error.message : String(error)
      };
    }
  }

  /**
   * Buy-and-hold over the run's whole span, in paisa of index points.
   *
   * The start comes from [`loadBenchmark`]'s own request; the end is the
   * newest bar already on screen, which IS the span's last bar because
   * `/bars/window.json` answers newest first.
   */
  /* ====================================================================
     THE SERIES THE CHARTS ACTUALLY DRAW
     --------------------------------------------------------------------
     WHAT THESE ARE, SAID ONCE AND SAID PLAINLY: every curve, bar and
     column below is the BENCHMARK's — one unit of the index, held. Not the
     strategy's. The strategy has no series on disk and cannot be given
     one, so drawing its equity curve is off the table permanently.

     But the benchmark's series IS on disk: it is the closes of the bars
     already loaded for the price chart. Cumulative P&L of holding, P&L per
     week, the distribution of per-bar returns, the run-up and drawdown
     segments — all of it folds out of `series.bars` and all of it is true.

     That is the difference between an empty frame and a full one, and it
     costs nothing in honesty as long as every plot says whose line it is.
     Each chart carries that label; none of them claims to be the strategy.
     ==================================================================== */

  /** Cumulative P&L of holding one unit, in paisa, bar by bar. */
  const holdCurve = $derived.by(() => {
    const bars = series.bars;
    if (bars.length < 2) return [];
    const base = bars[0].c;
    return bars.map((b) => ({ t: b.t, v: b.c - base }));
  });

  /**
   * The bars bucketed by the selected period, each bucket's close-to-close
   * change in paisa.
   *
   * Buckets are keyed by a STRING derived from the bar's IST date, so a
   * week that straddles a month or a year stays one bucket. Ordered by first
   * appearance, which is chronological because the bars are.
   */
  const holdPeriods = $derived.by(() => {
    const bars = series.bars;
    if (bars.length < 2) return [];
    /** @param {number} t */
    const key = (t) => {
      const d = new Date(t * 1000);
      const y = d.getUTCFullYear();
      if (periodScale === 'yearly') return `${y}`;
      if (periodScale === 'quarterly') return `Q${Math.floor(d.getUTCMonth() / 3) + 1} '${String(y).slice(2)}`;
      if (periodScale === 'daily') return `${d.getUTCDate()}/${d.getUTCMonth() + 1}`;
      // Weekly: the Monday that starts the bar's week.
      const monday = new Date(d);
      monday.setUTCDate(d.getUTCDate() - ((d.getUTCDay() + 6) % 7));
      return `${monday.getUTCDate()}/${monday.getUTCMonth() + 1}`;
    };
    /** @type {Map<string, {label: string, first: number, last: number}>} */
    const buckets = new Map();
    for (const b of bars) {
      const k = key(b.t);
      const at = buckets.get(k);
      if (at) at.last = b.c;
      else buckets.set(k, { label: k, first: b.c, last: b.c });
    }
    return [...buckets.values()].map((x) => ({ label: x.label, v: x.last - x.first }));
  });

  /**
   * The distribution of per-bar returns, in basis points, bucketed.
   *
   * Per BAR, not per trade — the ledger has no trades to distribute. Said on
   * the chart, because a histogram labelled "returns" that is silently a
   * different population is exactly the quiet substitution this page refuses.
   */
  const returnHistogram = $derived.by(() => {
    const bars = series.bars;
    if (bars.length < 2) return { bins: [], max: 0, avgLoss: null, avgGain: null };
    const rets = [];
    for (const b of bars) {
      if (b.o > 0) rets.push(Math.round(((b.c - b.o) / b.o) * 10_000));
    }
    if (rets.length === 0) return { bins: [], max: 0, avgLoss: null, avgGain: null };
    const lo = Math.min(...rets);
    const hi = Math.max(...rets);
    const width = Math.max(1, Math.ceil((hi - lo) / 18));
    /** @type {Map<number, number>} */
    const counts = new Map();
    for (const r of rets) {
      const slot = Math.floor((r - lo) / width);
      counts.set(slot, (counts.get(slot) ?? 0) + 1);
    }
    const bins = [];
    for (let i = 0; i <= Math.floor((hi - lo) / width); i += 1) {
      const from = lo + i * width;
      bins.push({ from, mid: from + width / 2, n: counts.get(i) ?? 0 });
    }
    const losses = rets.filter((r) => r < 0);
    const gains = rets.filter((r) => r > 0);
    const mean = (xs) => (xs.length === 0 ? null : Math.round(xs.reduce((a, b) => a + b, 0) / xs.length));
    return {
      bins,
      max: Math.max(1, ...bins.map((b) => b.n)),
      lo,
      hi,
      avgLoss: mean(losses),
      avgGain: mean(gains),
      losers: losses.length,
      winners: gains.length,
      flat: rets.length - losses.length - gains.length,
      total: rets.length
    };
  });

  /**
   * Alternating run-up and drawdown segments of the hold curve.
   *
   * A segment runs from one running extreme to the next reversal. Real, and
   * a property of the PRICE — which is what makes it the benchmark's growth
   * and decline rather than the strategy's.
   */
  const holdSwings = $derived.by(() => {
    const curve = holdCurve;
    if (curve.length < 3) return { segs: [], max: 1 };
    const segs = [];
    let anchor = curve[0].v;
    let extreme = curve[0].v;
    let dir = 0;
    for (const p of curve) {
      const rising = p.v > extreme;
      const falling = p.v < extreme;
      if (dir === 0) {
        if (rising) dir = 1;
        else if (falling) dir = -1;
        if (rising || falling) extreme = p.v;
        continue;
      }
      if ((dir === 1 && rising) || (dir === -1 && falling)) {
        extreme = p.v;
        continue;
      }
      // A move of at least 1% of the whole range counts as a reversal, so the
      // chart shows swings rather than every tick of noise.
      const span = Math.abs(p.v - extreme);
      if (span > Math.abs(extreme - anchor) * 0.35 && span > 0) {
        segs.push({ up: dir === 1, size: Math.abs(extreme - anchor) });
        anchor = extreme;
        extreme = p.v;
        dir = -dir;
      }
    }
    segs.push({ up: dir === 1, size: Math.abs(extreme - anchor) });
    const kept = segs.filter((s) => s.size > 0).slice(-16);
    return { segs: kept, max: Math.max(1, ...kept.map((s) => s.size)) };
  });

  const buyHold = $derived.by(() => {
    if (bench.phase !== 'ready' || series.bars.length === 0) return null;
    const last = series.bars[series.bars.length - 1];
    if (!last || bench.open <= 0) return null;
    const gain = last.c - bench.open;
    return {
      from: bench.open,
      to: last.c,
      gain,
      bps: Math.round((gain / bench.open) * 10_000)
    };
  });

  /**
   * The strategy against buy-and-hold, on the figure selection ranks on.
   *
   * Compared on `pessimistic` and not on `optimistic`, for the same reason
   * everything else here is: the flattering number would flatter this
   * comparison most of all.
   */
  const outperformance = $derived.by(() => {
    if (!buyHold || !openRun) return null;
    return {
      strategy: openRun.pessimistic,
      hold: buyHold.gain,
      edge: openRun.pessimistic - buyHold.gain,
      beat: openRun.pessimistic > buyHold.gain
    };
  });

  /**
   * Expected payoff per trade — TradingView's own name for it.
   *
   * Integer division of paisa by trades, so it stays in paisa. `null` at
   * zero trades rather than a division by zero rendered as `Infinity`.
   */
  const perTrade = $derived.by(() => {
    if (!openRun || openRun.trades === 0) return null;
    return {
      worst: Math.round(openRun.pessimistic / openRun.trades),
      best: Math.round(openRun.optimistic / openRun.trades)
    };
  });

  /* ====================================================================
     THE STRATEGY TESTER — TradingView's own layout, lock glyphs and all
     --------------------------------------------------------------------
     THE PADLOCK IS THEIR IDIOM AND IT IS EXACTLY THE ONE THIS PAGE NEEDS.
     TradingView renders a metric it cannot show as a LOCK GLYPH in the
     cell — not a blank, not "N/A", not a dropped row. Their lock means
     "your plan does not include this"; ours means "the sweep never wrote
     this". The meaning differs and the discipline is identical: the row
     stays, in its place, in its order, and the cell says it is unavailable
     rather than pretending the metric does not exist.

     So every row TradingView shows is here, in TradingView's order, and
     the ones this ledger cannot fill carry a lock whose tooltip says why.
     A reader who knows the Strategy Tester can read this without learning
     anything new, and can see at a glance exactly how much of it the
     engine currently records.
     ==================================================================== */

  /** Which of the tester's three views is showing. */
  let testerView = $state('metrics');
  /** The period scale on every periodic chart. TradingView's four. */
  let periodScale = $state('weekly');
  /** "Profits and losses" split. TradingView's two. */
  let plSplit = $state('signals');
  /** The streak chart's unit. TradingView's two. */
  let streakMode = $state('count');
  /** Whether the testing-period menu is open. */
  let periodOpen = $state(false);
  /** Which testing period is selected. TradingView's list, verbatim. */
  let testingPeriod = $state('Available chart range');
  /** TradingView's testing-period menu, in its order. */
  const TESTING_PERIODS = [
    'Available chart range',
    'Last 7 days',
    'Last 30 days',
    'Last 90 days',
    'Last 365 days',
    'Entire history'
  ];
  /** Which "Performance analysis" tab. TradingView's five, in its order. */
  let paTab = $state('breakdown');
  /** Which "Trades analysis" tab. TradingView's three, in its order. */
  let taTab = $state('details');

  /**
   * Return on ONE UNIT of the index, in basis points.
   *
   * The ledger records totals in paisa of index points and no capital, so
   * "return" here is the total against the price one unit cost at the span's
   * start — which is the only denominator on disk. Stated wherever it shows.
   */
  const strategyBps = $derived.by(() => {
    if (!openRun || bench.phase !== 'ready' || bench.open <= 0) return null;
    return Math.round((openRun.pessimistic / bench.open) * 10_000);
  });

  /** Years the span covers, for the annualised figure. */
  const spanYears = $derived.by(() => {
    if (!openRun || openRun.months_found === 0) return null;
    return openRun.months_found / 12;
  });

  /**
   * Annualised return (CAGR) in basis points, over the months actually found.
   *
   * Over the months FOUND, not the months asked for. A span with a hole is a
   * shorter sample, and annualising a shorter sample against the longer window
   * would inflate the figure by exactly the size of the hole.
   */
  const cagrBps = $derived.by(() => {
    if (strategyBps === null || !spanYears || spanYears <= 0) return null;
    const total = 1 + strategyBps / 10_000;
    if (total <= 0) return null;
    return Math.round((total ** (1 / spanYears) - 1) * 10_000);
  });

  /** Max drawdown against the span's opening price, in basis points. */
  const drawdownBps = $derived.by(() => {
    if (!openRun || bench.phase !== 'ready' || bench.open <= 0) return null;
    return Math.round((Math.abs(openRun.max_drawdown) / bench.open) * 10_000);
  });

  /** One scale for the two fill-model bars, so their lengths are comparable. */
  const fillScale = $derived(
    Math.max(1, Math.abs(openRun?.optimistic ?? 0), Math.abs(openRun?.pessimistic ?? 0))
  );

  /** One scale for the three excursion bars, over absolute values. */
  const excursionTop = $derived(
    Math.max(
      1,
      Math.abs(openRun?.winner_mae ?? 0),
      Math.abs(openRun?.winner_mfe ?? 0),
      Math.abs(openRun?.all_mae ?? 0)
    )
  );

  /* ---- AXIS TICKS ------------------------------------------------------
     A plot's SCALE is a true statement about it even when the series is
     missing: these are the gridlines the data would be read against, and
     drawing them is what makes an empty frame read as a chart rather than
     as a broken one. TradingView puts the value axis on the right, top
     value first, so these are ordered top-down. -------------------------- */

  /** Money axis, symmetric about zero — the periodic and benchmark plots. */
  const PNL_TICKS = ['+2K', '+1K', '0', '−1K', '−2K'];
  /** Percentage axis — margin utilisation and the growth/decline plot. */
  const PCT_TICKS = ['100%', '75%', '50%', '25%', '0%'];
  /** Streak counts run outward in BOTH directions from zero, as TradingView
      draws them: wins above the line, losses below, both counted positive. */
  const STREAK_TICKS = ['8', '4', '0', '4', '8'];
  /** A histogram counts trades, so its axis starts at zero and only rises. */
  const COUNT_TICKS = ['20', '15', '10', '5', '0'];
  /** The returns histogram's own x-axis, in percent, as TradingView labels it. */
  const RETURN_TICKS = ['−0.8%', '−0.4%', '0%', '0.4%', '0.8%', '1.2%'];
  /** The histogram legend carries two DASHED entries for the two averages. */
  const HIST_LEGEND = [
    { label: 'Losers' },
    { label: 'Winners' },
    { label: 'Average loss', dash: true },
    { label: 'Average profit', dash: true }
  ];

  /**
   * The x-axis labels for a periodic chart, spread across the run's own span.
   *
   * Derived from the span rather than written down: a run over four months and
   * a run over seven years must not share an axis, and a hardcoded date row
   * would be a claim about a window this page does not choose.
   */
  const periodTicks = $derived.by(() => {
    if (!openRun) return [];
    const from = openRun.from_year * 12 + (openRun.from_month - 1);
    const to = openRun.to_year * 12 + (openRun.to_month - 1);
    const span = Math.max(1, to - from);
    const steps = 6;
    const out = [];
    for (let i = 0; i <= steps; i += 1) {
      const m = from + Math.round((span * i) / steps);
      const y = Math.floor(m / 12);
      const mo = (m % 12) + 1;
      out.push(`${String(mo).padStart(2, '0')}/${String(y).slice(2)}`);
    }
    return out;
  });

  /** The heading TradingView puts above each periodic chart. */
  const PERIOD_LABEL = {
    daily: 'Daily',
    weekly: 'Weekly',
    quarterly: 'Quarterly',
    yearly: 'Yearly'
  };

  /** One scale for the two benchmark bars. */
  const benchScale = $derived(
    Math.max(1, Math.abs(outperformance?.strategy ?? 0), Math.abs(outperformance?.hold ?? 0))
  );

  /* ---- chart geometry -------------------------------------------------
     Plain arithmetic over a 1000×260 viewBox, so the SVG scales with its
     container and no measurement is needed. Every one takes the series it
     draws as an argument rather than reading state, so a chart cannot
     silently render one panel's data under another panel's heading. ---- */

  /**
   * One plotted value. `v` is the only field the geometry reads.
   *
   * @typedef {{ v: number }} Point
   */

  /**
   * One column: a value and the label under it.
   *
   * @typedef {{ v: number, label: string }} Bar
   */

  /**
   * A histogram's own range, which is what its x positions are relative to.
   *
   * @typedef {{ lo: number, hi: number }} Hist
   */

  /**
   * The vertical extent a curve needs, symmetric so zero stays on a line.
   *
   * @param {Point[]} points
   */
  function curveScale(points) {
    return Math.max(1, ...points.map((p) => Math.abs(p.v)));
  }

  /**
   * The polyline through a cumulative series.
   *
   * @param {Point[]} points
   */
  function linePath(points) {
    const scale = curveScale(points);
    const step = 1000 / Math.max(1, points.length - 1);
    return points
      .map((p, i) => `${i === 0 ? 'M' : 'L'}${(i * step).toFixed(1)} ${(130 - (p.v / scale) * 120).toFixed(1)}`)
      .join(' ');
  }

  /**
   * The same polyline, closed to the zero line, for the area fill.
   *
   * @param {Point[]} points
   */
  function areaPath(points) {
    const step = 1000 / Math.max(1, points.length - 1);
    return `${linePath(points)} L${((points.length - 1) * step).toFixed(1)} 130 L0 130 Z`;
  }

  /**
   * The tallest bar in a set, so a column chart shares one scale.
   *
   * @param {Bar[]} bars
   */
  function barScale(bars) {
    return Math.max(1, ...bars.map((b) => Math.abs(b.v)));
  }

  /**
   * At most seven x labels, evenly spaced, so the axis never crowds.
   *
   * @param {Bar[]} bars
   */
  function barLabels(bars) {
    if (bars.length === 0) return [];
    if (bars.length <= 7) return bars.map((b) => b.label);
    const out = [];
    for (let i = 0; i < 7; i += 1) out.push(bars[Math.round((i * (bars.length - 1)) / 6)].label);
    return out;
  }

  /**
   * Where a basis-point value falls across the histogram's own range.
   *
   * @param {Hist} h
   * @param {number} bps
   */
  function histX(h, bps) {
    const span = Math.max(1, h.hi - h.lo);
    return (((bps - h.lo) / span) * 1000).toFixed(1);
  }

  /**
   * Basis points as a signed percent, for the histogram's axis and legend.
   *
   * @param {number} bps
   */
  const pctOf = (bps) => `${bps >= 0 ? '+' : ''}${(bps / 100).toFixed(2)}%`;

  /**
   * A basis-point integer as a percent string, or the em dash.
   *
   * @param {number | null | undefined} bps
   */
  const pct = (bps) => (bps === null || bps === undefined ? '—' : `${bps >= 0 ? '+' : ''}${(bps / 100).toFixed(2)}%`);

  /**
   * An excursion in parts per million, as the percent a reader can act on.
   *
   * THE PAGE PRINTED `-4,200 ppm` IN FOUR PLACES AND EXPLAINED IT IN NONE.
   * Parts-per-million is the unit the record stores because it is an integer
   * and §7 bans floats for money; it is not a unit anybody reads a drawdown
   * in. `-4,200 ppm` is `-0.42%`, and the second form is the one that says
   * whether the number matters.
   *
   * The raw figure is not thrown away -- it goes in the `title` at every
   * call site, so nothing that was on the page has left it.
   *
   * @param {number | null | undefined} ppm
   */
  const ppmPct = (ppm) =>
    ppm === null || ppm === undefined
      ? '—'
      : `${ppm >= 0 ? '+' : ''}${(ppm / 10_000).toFixed(2)}%`;

  /** Rungs that carry a recorded run, so the switcher can mark them. */
  const sweptRungs = $derived(
    new Set(runs.filter((r) => r.underlying === openRun?.underlying).map((r) => r.timeframe))
  );

  /** The console's own palette, read off the document so both themes follow. */
  function chartTokens() {
    const s = getComputedStyle(document.documentElement);
    const v = (name, fallback) => s.getPropertyValue(name).trim() || fallback;
    return {
      text: v('--n9', '#8a95ab'),
      grid: v('--n5', '#1b212e'),
      border: v('--n6', '#232a3a'),
      up: v('--up', '#00e19b'),
      down: v('--down', '#ff4d6a'),
      acc: v('--acc', '#22d3ee'),
      faint: v('--n7', '#333c51')
    };
  }

  /** The chart handle, kept so the range presets can drive the time scale. */
  let chartApi = null;

  /**
   * Show the last `n` bars, or everything.
   *
   * Sets the VISIBLE RANGE rather than refetching: the window is already in
   * memory, so a preset is a pan and not a request. `null` fits everything.
   *
   * @param {number | null} n
   */
  function showLast(n) {
    if (!chartApi) return;
    const total = series.bars.length;
    if (n === null || n >= total) {
      chartApi.timeScale().fitContent();
      return;
    }
    chartApi.timeScale().setVisibleLogicalRange({ from: total - n, to: total });
  }

  /**
   * The presets, in bars, derived from the rung's own length.
   *
   * A month is a different number of bars at every rung — roughly 1,375 at
   * 30-minute and 21 at daily — so a preset list in BARS would mean a
   * different span at each rung. These are computed from the rung's seconds
   * against a 6.25-hour session, so "3M" is three months whatever the rung.
   */
  const presets = $derived.by(() => {
    const secs = rungSeconds(chartRung || openRun?.timeframe || '');
    if (!secs) return [];
    // 555 minutes of session, 21 sessions a month — the same 555 the store's
    // own alignment notes use.
    const perMonth = secs >= 86_400 ? 21 : Math.max(1, Math.round((555 * 60) / secs) * 21);
    return [
      { label: '1M', bars: perMonth },
      { label: '3M', bars: perMonth * 3 },
      { label: '6M', bars: perMonth * 6 },
      { label: '1Y', bars: perMonth * 12 },
      { label: 'All', bars: null }
    ].filter((p) => p.bars === null || p.bars < series.bars.length);
  });

  /** What the bars on screen fold to — high, low, range, net change. */
  const windowStats = $derived.by(() => {
    const bars = series.bars;
    if (bars.length === 0) return null;
    let hi = -Infinity;
    let lo = Infinity;
    for (const b of bars) {
      if (b.h > hi) hi = b.h;
      if (b.l < lo) lo = b.l;
    }
    const first = bars[0];
    const last = bars[bars.length - 1];
    // BASIS POINTS, INTEGER. `CLAUDE.md` §7 keeps prices in paisa and this
    // crate's own rule is that a ratio is not a float until it is displayed.
    const netBps = first.c > 0 ? Math.round(((last.c - first.c) / first.c) * 10_000) : null;
    return { hi, lo, range: hi - lo, first, last, netBps };
  });

  $effect(() => {
    const host = chartHost;
    const bars = series.bars;
    if (!host || bars.length === 0) return;
    let dead = false;
    let made = null;
    (async () => {
      try {
        // TRADINGVIEW'S OWN LIBRARY, MIT, BUNDLED — no CDN and nothing fetched
        // at runtime, exactly as the Markets page takes it.
        const mod = await import('lightweight-charts');
        if (dead) return;
        const c = chartTokens();
        made = mod.createChart(host, {
          autoSize: true,
          layout: {
            background: { color: 'transparent' },
            textColor: c.text,
            attributionLogo: false
          },
          grid: { vertLines: { color: c.grid }, horzLines: { color: c.grid } },
          rightPriceScale: { borderColor: c.border, scaleMargins: { top: 0.08, bottom: 0.08 } },
          timeScale: {
            borderColor: c.border,
            timeVisible: true,
            secondsVisible: false,
            rightOffset: 2,
            // MAX HAS TO MEAN MAX — the Markets page's own note. The default
            // half-pixel floor on bar spacing makes `fitContent` silently draw
            // only the last few hundred bars while the axis claims the span.
            minBarSpacing: 0.02,
            // 0 Year, 1 Month, 2 DayOfMonth, 3 Time, 4 TimeWithSeconds. Without
            // this the axis labels in UTC while the legend beside it reads IST,
            // and the two name different times for the bar they both point at.
            tickMarkFormatter: (time, type) => istLabel(Number(time), type <= 2)
          },
          // The crosshair's own time label, same clock.
          localization: {
            locale: 'en-IN',
            timeFormatter: (time) => `${istLabel(Number(time), false)} IST`
          },
          crosshair: {
            mode: 1, // magnet: snaps to OHLC, which is what the price under the pointer means
            vertLine: { color: c.faint, labelBackgroundColor: c.acc, width: 1 },
            horzLine: { color: c.faint, labelBackgroundColor: c.acc, width: 1 }
          },
          handleScroll: { mouseWheel: true, pressedMouseMove: true },
          handleScale: { mouseWheel: true, pinch: true }
        });
        const cs = made.addSeries(mod.CandlestickSeries, {
          upColor: c.up,
          downColor: c.down,
          wickUpColor: c.up,
          wickDownColor: c.down,
          borderVisible: false,
          priceFormat: { type: 'price', precision: 2, minMove: 0.01 }
        });
        // PAISA DIVIDE ONCE, AT THE DISPLAY BOUNDARY. `CLAUDE.md` §7 keeps
        // prices as `i64` paisa everywhere; this is the only place a
        // fractional rupee may exist, and it exists because the library takes
        // floats.
        cs.setData(
          bars.map((b) => ({
            time: b.t,
            open: b.o / 100,
            high: b.h / 100,
            low: b.l / 100,
            close: b.c / 100
          }))
        );

        // THE LEGEND READS THE PAISA, NOT THE CHART. Only the TIME comes back
        // off the crosshair; the bar itself is looked up in a Map of the
        // integers, so no number the legend prints has been through the
        // library's floats. The Markets page takes the same care in the same
        // words.
        const byTime = new Map(bars.map((b) => [b.t, b]));
        made.subscribeCrosshairMove((param) => {
          const at = param?.time == null ? null : byTime.get(Number(param.time));
          ohlc = at ?? null;
        });

        made.timeScale().fitContent();
        chartApi = made;
        chartError = '';
      } catch (why) {
        // A chart library that will not load is a NAMED failure, not a blank
        // rectangle. Every figure above this panel still stands.
        if (!dead) chartError = String(why?.message ?? why);
      }
    })();
    return () => {
      dead = true;
      chartApi = null;
      ohlc = null;
      try {
        made?.remove();
      } catch {
        // Already disposed by the library's own teardown; nothing to undo.
      }
    };
  });


  /* ====================================================================
     FORMATTING
     ==================================================================== */

  /**
   * A paisa integer as a rupee string — or the em dash when it is not a number.
   *
   * **`rupee` alone is NOT safe here, and `money.js` says so in its own words**:
   * `Intl` formats `NaN` as the string `"NaN"` and `Infinity` as `"∞"`, which is
   * "a non-answer rendered as though it were one — the failure CLAUDE.md §4
   * names". `exact` and `whole` are the guarded helpers in that module and
   * `rupee` is not, because every other caller reaches it through a value the
   * store guarantees. This page reads a JSON body, and a field that arrives
   * absent, null or malformed would print `₹NaN` beside twelve correct figures.
   *
   * @param {number} paisa
   * @returns {string}
   */
  function money(paisa) {
    return Number.isFinite(paisa) ? `₹${rupee(paisa)}` : '—';
  }

  /**
   * Whether a run's two fill models contradict each other.
   *
   * Best-case fills can never total LESS than worst-case fills — they are the
   * same trades priced at the friendlier end of the same bars. A run where they
   * do is a contradiction in the record itself, not a result, and it is named
   * rather than plotted: the ghost bar would render shorter than the solid one
   * and read as a merely unusual run.
   *
   * @param {any} r
   */
  const contradicts = (r) =>
    Number.isFinite(r?.optimistic) &&
    Number.isFinite(r?.pessimistic) &&
    r.optimistic < r.pessimistic;

  /**
   * The exchange's own clock, pinned.
   *
   * **NSE bars are IST and the browser's zone is not a fact about them.**
   * Without `timeZone` this used whatever zone the reader's machine is in, so
   * the same bar read 10:45 in Mumbai and 05:15 in London — and the chart's own
   * axis, which labels in UTC unless told otherwise, disagreed with the legend
   * beside it on a bar they were both pointing at. One clock, and it is the
   * exchange's.
   */
  const IST = 'Asia/Kolkata';

  /** @param {number} micros */
  function when(micros) {
    if (!Number.isFinite(micros) || micros <= 0) return 'not stamped';
    return new Date(micros / 1000).toLocaleString('en-IN', {
      timeZone: IST,
      year: 'numeric',
      month: 'short',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  /**
   * A unix second as the chart's own axis label, in IST.
   *
   * `lightweight-charts` labels its time axis in UTC unless a formatter says
   * otherwise, so this is what stops the axis and the legend naming two
   * different times for one bar.
   *
   * @param {number} seconds
   * @param {boolean} dateOnly a tick mark coarser than a day needs no clock
   */
  function istLabel(seconds, dateOnly) {
    return new Date(seconds * 1000).toLocaleString('en-IN', {
      timeZone: IST,
      month: 'short',
      day: '2-digit',
      ...(dateOnly ? {} : { hour: '2-digit', minute: '2-digit' })
    });
  }

  /**
   * A run's span, in the product's month format.
   *
   * THIS IS WHERE `2016-08 → 2026-08` WAS COMING FROM, and it feeds SEVEN
   * call sites: the answer card, the ledger's Span column, the rung group
   * header, the drill-down heading, two chart captions and the detail
   * table. Fixing the sweep form's own controls left every one of them
   * still rendering the store's key rather than the product's month, so
   * the page showed `Aug 2016` in the form and `2016-08` in the answer
   * directly beneath it.
   *
   * `monthLabel` returns its input UNCHANGED when it does not parse, so a
   * malformed year or month still renders as the value it actually is --
   * which is the behaviour `$lib/dates.js` documents and the reason this
   * needs no guard of its own.
   *
   * Nothing sorts on this string. The `Span` column is `nosort` precisely
   * because ordering it as text was already known to be wrong, so changing
   * what it reads cannot change what it ranks.
   *
   * @param {any} r
   */
  const span = (r) =>
    `${monthLabel(`${r.from_year}-${String(r.from_month).padStart(2, '0')}`)} → ${monthLabel(`${r.to_year}-${String(r.to_month).padStart(2, '0')}`)}`;

  /** Coverage as a whole percent, for the bar's width only — never displayed alone. */
  const coverPct = (r) =>
    r.months_asked > 0 ? Math.round((r.months_found / r.months_asked) * 100) : 0;

  /**
   * One row as a sentence, for a screen reader.
   *
   * **Without this a row announces nine bare values**: "NIFTY zerodha 30min
   * 2019-12 → 2026-08 81/81 yes · depth 16 5,093 ₹11,333.39 ₹16,613.81" — with
   * no way to know which money is which, and the whole point of this page is
   * that one of those two figures is the one to rank on and the other is not.
   * The visual reader has column headings; this is the same information for a
   * reader who has none.
   *
   * @param {any} r
   */
  function rowLabel(r) {
    const complete = r.halted
      ? `halted at depth ${r.depth}, not comparable`
      : `complete to depth ${r.depth}`;
    const cover = r.whole_span
      ? 'whole span'
      : `${r.months_found} of ${r.months_asked} months on disk`;
    return (
      `${r.underlying} on ${r.feed}, ${r.timeframe} rung, ${span(r)}, ${cover}, ${complete}, ` +
      `${exact(r.trades)} trades, worst-case ${money(r.pessimistic)}, ` +
      `best-case ${money(r.optimistic)}. Activate to drill in.`
    );
  }

  /**
   * The five exit axes, in the order the record stores them.
   *
   * `-1` is "no rung on this axis" and is rendered as those words. A dash would
   * read as missing data, and the difference between "no stop was used" and "the
   * stop was not recorded" is the whole point of the field.
   */
  const EXIT_AXES = ['stop', 'target', 'trailing stop', 'TTP arm', 'TTP trail'];
</script>

<svelte:head><title>Backtest · brutex</title></svelte:head>

<!-- ============================================================
     A CHART FRAME WITH NO SERIES IN IT.
     Axes, gridlines, the zero line, the legend and the pager are all
     drawn, because they are part of the design and they are all TRUE --
     the shape of the plot is not in doubt, only its contents. The plot
     area carries the sentence saying which field the sweep would have to
     write for the series to appear.

     Drawing the frame rather than hiding the panel is the same choice
     the padlock makes one level down: the thing keeps its place, and the
     absence is legible instead of invisible.
     ============================================================ -->

<!-- ============================================================
     THE DRAWN CHARTS.
     Each takes a real series folded out of the bars on disk and each
     carries a label saying whose series it is. They are the BENCHMARK's --
     one unit of the index, held -- because the strategy has none on disk
     and never will until the sweep writes one.
     ============================================================ -->

{#snippet areaChart(points, ticks, xLabels, note)}
  <div class="cf">
    <div class="cf-plot tall">
      <svg class="cf-svg" viewBox="0 0 1000 260" preserveAspectRatio="none" aria-hidden="true">
        {#each [0, 65, 130, 195, 260] as y (y)}
          <line x1="0" y1={y} x2="1000" y2={y} stroke="var(--n5)" stroke-width="1" vector-effect="non-scaling-stroke" />
        {/each}
        {#if points.length > 1}
          <path d={areaPath(points)} fill="var(--acc-soft)" />
          <path d={linePath(points)} fill="none" stroke="var(--acc)" stroke-width="2" vector-effect="non-scaling-stroke" stroke-linejoin="round" />
        {/if}
      </svg>
      <div class="cf-axis">{#each ticks as t (t)}<span>{t}</span>{/each}</div>
    </div>
    <div class="cf-x">{#each xLabels as x (x)}<span>{x}</span>{/each}</div>
    <p class="cf-note">{note}</p>
  </div>
{/snippet}

{#snippet barChart(bars, ticks, note, legend)}
  <div class="cf">
    <div class="cf-plot tall">
      <svg class="cf-svg" viewBox="0 0 1000 260" preserveAspectRatio="none" aria-hidden="true">
        {#each [0, 65, 130, 195, 260] as y (y)}
          <line x1="0" y1={y} x2="1000" y2={y} stroke="var(--n5)" stroke-width="1" vector-effect="non-scaling-stroke" />
        {/each}
        <line x1="0" y1="130" x2="1000" y2="130" stroke="var(--n7)" stroke-width="1" vector-effect="non-scaling-stroke" />
        {#each bars as b, i (i)}
          {@const w = 1000 / Math.max(1, bars.length)}
          {@const h = (Math.abs(b.v) / barScale(bars)) * 120}
          <rect
            x={i * w + w * 0.18}
            y={b.v >= 0 ? 130 - h : 130}
            width={w * 0.64}
            height={Math.max(1, h)}
            fill={b.v >= 0 ? 'var(--up)' : 'var(--down)'}
            rx="1"
          />
        {/each}
      </svg>
      <div class="cf-axis">{#each ticks as t (t)}<span>{t}</span>{/each}</div>
    </div>
    <div class="cf-x">
      {#each barLabels(bars) as x (x)}<span>{x}</span>{/each}
    </div>
    {#if legend}
      <ul class="cf-legend">
        {#each legend as l, i (l)}<li><span class="cf-sw s{i}"></span>{l}</li>{/each}
      </ul>
    {/if}
    <p class="cf-note">{note}</p>
  </div>
{/snippet}

{#snippet histogram(h, note)}
  <div class="cf">
    <div class="cf-plot">
      <svg class="cf-svg" viewBox="0 0 1000 200" preserveAspectRatio="none" aria-hidden="true">
        {#each [0, 50, 100, 150, 200] as y (y)}
          <line x1="0" y1={y} x2="1000" y2={y} stroke="var(--n5)" stroke-width="1" vector-effect="non-scaling-stroke" />
        {/each}
        {#each h.bins as b, i (i)}
          {@const w = 1000 / Math.max(1, h.bins.length)}
          {@const bh = (b.n / h.max) * 190}
          <rect x={i * w + w * 0.12} y={200 - bh} width={w * 0.76} height={Math.max(1, bh)} fill={b.mid < 0 ? 'var(--down)' : 'var(--up)'} rx="1" />
        {/each}
        {#if h.avgLoss !== null}
          <line x1={histX(h, h.avgLoss)} y1="0" x2={histX(h, h.avgLoss)} y2="200" stroke="var(--down)" stroke-width="1.5" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" />
        {/if}
        {#if h.avgGain !== null}
          <line x1={histX(h, h.avgGain)} y1="0" x2={histX(h, h.avgGain)} y2="200" stroke="var(--up)" stroke-width="1.5" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" />
        {/if}
      </svg>
      <div class="cf-axis">
        <span>{h.max}</span><span>{Math.round(h.max / 2)}</span><span>0</span>
      </div>
    </div>
    <div class="cf-x">
      <span>{pctOf(h.lo)}</span><span>{pctOf(Math.round((h.lo + h.hi) / 2))}</span><span>{pctOf(h.hi)}</span>
    </div>
    <ul class="cf-legend">
      <li><span class="cf-sw s1"></span>Losers<b>{exact(h.losers)}</b></li>
      <li><span class="cf-sw s0"></span>Winners<b>{exact(h.winners)}</b></li>
      <li class="dash"><span class="cf-sw dashed"></span>Average loss<b>{h.avgLoss === null ? '—' : pctOf(h.avgLoss)}</b></li>
      <li class="dash"><span class="cf-sw dashed"></span>Average profit<b>{h.avgGain === null ? '—' : pctOf(h.avgGain)}</b></li>
    </ul>
    <p class="cf-note">{note}</p>
  </div>
{/snippet}

{#snippet swingChart(sw, note)}
  <div class="cf">
    <div class="cf-plot">
      <svg class="cf-svg" viewBox="0 0 1000 200" preserveAspectRatio="none" aria-hidden="true">
        {#each [0, 50, 100, 150, 200] as y (y)}
          <line x1="0" y1={y} x2="1000" y2={y} stroke="var(--n5)" stroke-width="1" vector-effect="non-scaling-stroke" />
        {/each}
        {#each sw.segs as s, i (i)}
          {@const w = 1000 / Math.max(1, sw.segs.length)}
          {@const h = (s.size / sw.max) * 190}
          <rect x={i * w + w * 0.16} y={200 - h} width={w * 0.68} height={Math.max(1, h)} fill={s.up ? 'var(--up)' : 'var(--down)'} rx="1" />
        {/each}
      </svg>
      <div class="cf-axis">
        <span>{money(sw.max)}</span><span>{money(Math.round(sw.max / 2))}</span><span>0</span>
      </div>
    </div>
    <ul class="cf-legend">
      <li><span class="cf-sw s0"></span>Run-up</li>
      <li><span class="cf-sw s1"></span>Drawdown</li>
    </ul>
    <p class="cf-note">{note}</p>
  </div>
{/snippet}
{#snippet chartFrame(why, legend, ticks, xLabels, pager)}
  <div class="cf">
    <div class="cf-plot">
      {#if pager}
        <!-- TradingView pages a dense periodic chart rather than squeezing it.
             The arrows sit inside the plot, vertically centred, on both edges. -->
        <button class="cf-page l" aria-label="Earlier periods" disabled>‹</button>
        <button class="cf-page r" aria-label="Later periods" disabled>›</button>
      {/if}
      <svg class="cf-grid" viewBox="0 0 100 60" preserveAspectRatio="none" aria-hidden="true">
        {#each [0, 15, 30, 45, 60] as y (y)}
          <line x1="0" y1={y} x2="100" y2={y} stroke="var(--n5)" stroke-width="0.3" />
        {/each}
        <line x1="0" y1="30" x2="100" y2="30" stroke="var(--n7)" stroke-width="0.5" />
      </svg>
      <!-- THE VALUE AXIS IS ON THE RIGHT, as it is on every TradingView plot,
           and it carries real tick labels rather than a plus and a minus. The
           SCALE is a true statement about the plot even when the series is
           not there: these are the gridlines the data would be read against. -->
      <div class="cf-axis">
        {#each ticks as t (t)}<span>{t}</span>{/each}
      </div>
      <div class="cf-msg"><Lock /> <span>{why}</span></div>
    </div>
    {#if xLabels.length > 0}
      <div class="cf-x">
        {#each xLabels as x (x)}<span>{x}</span>{/each}
      </div>
    {/if}
    {#if legend.length > 0}
      <ul class="cf-legend">
        {#each legend as l, i (l.label ?? l)}
          <li class:dash={l.dash}>
            <span class="cf-sw s{i}" class:dashed={l.dash}></span>{l.label ?? l}{#if l.value}<b>{l.value}</b>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/snippet}

<div class="page">
  <!-- WHAT CHANGED, FOR A READER WHO CANNOT SEE IT CHANGE.
       Every state below swaps whole blocks of the page — loading to ready,
       ready to refused, a drill-down opening. A sighted reader sees the swap;
       without a live region a screen-reader user gets silence and a page that
       is suddenly different. `polite` rather than `assertive`: none of this is
       urgent enough to interrupt what is being read. -->
  <p class="sr-only" role="status" aria-live="polite">
    {#if load.phase === 'loading'}
      Reading the results ledger.
    {:else if load.phase === 'failed'}
      The ledger could not be asked for. {load.why}
    {:else if refusal}
      {refusal}
    {:else}
      {exact(runs.length)} runs shown, {exact(completeRuns.length)} complete, {exact(
        haltedRuns.length
      )} halted.
      {best ? `Best complete run: ${best.underlying} at the ${best.timeframe} rung.` : 'No complete run to rank.'}
    {/if}
  </p>

  <!-- ================================================================
       HEADER
       ================================================================ -->
  <header class="head">
    <div>
      <h1>Backtest</h1>
      <!-- THE STANDFIRST BECOMES A TOOLTIP. It said something true and said
           it in three lines above the data; TradingView's tester carries no
           page copy at all. The fact survives where a reader who wants it
           will look for it, and the space goes back to the numbers. -->
      <p class="sub" title="Ranked on worst-case fills — adverse-extreme execution — because that is what selection ranks on everywhere in this workspace. The best-case column is open fills and is never the figure a winner is chosen by.">
        ranked on <b>worst-case fills</b>
      </p>
    </div>
    <button class="btn ghost" onclick={fetchLedger} disabled={load.phase === 'loading'}>
      {load.phase === 'loading' ? 'Reading…' : 'Re-read ledger'}
    </button>
  </header>

  <!-- ================================================================
       RUN ONE FROM HERE.
       The console reported on work it could not cause until this; the
       operator read the page, decided to try another rung, and left for
       a terminal. Every field is required and none is defaulted on the
       server: the span and the feed are two of the nine terms in a run's
       identity, so guessing either would name a run after something
       nobody asked for.
       ================================================================ -->
  <!-- ONE PANEL FROM HERE DOWN. Every section below owns only a bottom
       hairline; the shell owns the border, the radius and the shadow. A
       gap between two boxes says they are separate things, and every
       section here is a different view of ONE run. -->
  <div class="bt-shell">
  <section class="runbar">
    <span class="runbar-k">New sweep</span>
    <!-- A PICKER, NOT A FREE-TEXT BOX. The set of instruments is a fact the
         census already states; typing one lets an operator name something the
         store does not hold and learn about it from a refusal a minute later.
         While the census is loading the control says so rather than offering
         an empty list that looks like "none". -->
    <!-- INSTRUMENTS ARE A SET, SO THE CONTROL IS A SET. A `<select>` made a
         single choice the only representable one; picking two meant two
         visits. Chips show every instrument the store holds at once, and a
         single choice is a set of one -- so there is no mode to switch. -->
    <fieldset class="runf pickset">
      <legend>instruments</legend>
      {#if catalog.phase === 'ready' && catalog.held.length > 0}
        <div class="chiprow">
          {#each catalog.held as h, i (h.leaf)}
            <button
              type="button"
              class="pchip"
              class:on={pickedSymbols.has(h.leaf)}
              style="--i:{i}"
              aria-pressed={pickedSymbols.has(h.leaf)}
              title="{h.full} · {exact(h.months)} months on disk, {monthLabel(h.from)} to {monthLabel(h.to)}"
              onclick={() => {
                pickedSymbols = toggled(pickedSymbols, h.leaf);
                symbolTouched = true;
                spanTouched = false;
              }}
            >
              {h.leaf}
              <span class="pchip-n">{exact(h.months)}</span>
            </button>
          {/each}
        </div>
      {:else}
        <span class="runf-wait">{catalog.phase === 'loading' ? 'reading census…' : '—'}</span>
      {/if}
    </fieldset>
    <!-- TWO MONTH MENUS, NOT TWO TEXT BOXES. `2016-08` typed into a bare input
         is the one date format this console does not otherwise use, and it let
         an operator name a month the store does not hold. These offer the
         months on disk, labelled the way `$lib/dates.js` labels every other
         month in the product. -->
    <div class="runf">
      <span>from</span>
      {#if heldNow}
        <Picker
          single
          filter
          label="month"
          summary={monthLabel(ask.from) ?? 'First month'}
          title="The first month of the span. Only months this instrument actually holds are offered."
          rows={monthRows('from')}
          selected={new Set(ask.from ? [ask.from] : [])}
          onchange={(next) => {
            const [m] = [...next];
            if (m) {
              ask.from = m;
              spanTouched = true;
            }
          }}
        />
      {:else}
        <span class="runf-wait">{catalog.phase === 'loading' ? 'reading…' : '—'}</span>
      {/if}
    </div>
    <div class="runf">
      <span>to</span>
      {#if heldNow}
        <Picker
          single
          filter
          label="month"
          summary={monthLabel(ask.to) ?? 'Last month'}
          title="The last month of the span. Only months this instrument actually holds are offered."
          rows={monthRows('to')}
          selected={new Set(ask.to ? [ask.to] : [])}
          onchange={(next) => {
            const [m] = [...next];
            if (m) {
              ask.to = m;
              spanTouched = true;
            }
          }}
        />
      {:else}
        <span class="runf-wait">{catalog.phase === 'loading' ? 'reading…' : '—'}</span>
      {/if}
    </div>
    <!-- THE SUPPORT CONTROL IS GONE, AND ITS ABSENCE IS THE FEATURE.

         An instrument and a span are facts about what the operator wants to
         study. A support percentage is a knob on the machine, and offering it
         made the form ask a question only the machine can answer well: too low
         and the frequent frontier never empties, too high and the sweep finds
         nothing and reports that as a result. Both look like an answer.

         This is CLAUDE.md §6's argument for `k`, applied where it applies
         equally: "a parameter that can be set can be set wrongly and
         silently." The engine holds it at 20% and derives the actual hit count
         per rung from that rung's own bars, so nine rungs get nine different
         thresholds without anyone typing one. It is printed beside the result
         rather than hidden -- see the note at the end of this bar. -->
    <!-- DISABLED WHEN THE LEDGER CANNOT TAKE THE RESULT. A sweep that cannot
         record is nine rungs of real work thrown away, and the refusal arrives
         AFTER it. `blocked` is the server's own answer -- see `ledgerBlock`. -->
    <button
      class="btn run"
      onclick={startSweep}
      disabled={sweep.phase === 'starting' ||
        sweep.phase === 'running' ||
        !activeFeed ||
        blocked !== null}
    >
      <!-- "RUN ALL NINE RUNGS" NAMED A COUNT AND A UNIT, and both were wrong.
           Nine was what the store held; the ENGINE sweeps eight, because
           `cli::EVERY_RUNG` has no `1day` in it. And the control does not run
           rungs, it runs a brute force -- over whatever is selected above.
           The label says the ACTION now, and the count lives beside the
           chips where it can be checked against them. -->
      {#if sweep.phase === 'starting'}Starting…{:else if sweep.phase === 'running'}Sweeping…{:else}Run
        brute force{/if}
    </button>
    <span class="runbar-n">
      {#if blocked}
        this ledger cannot take a result — see the note above the table
      {:else if !activeFeed}
        choose a feed first — a run is stamped with the feed its bars came from
      {:else if catalog.phase === 'loading'}
        reading what <b>{activeFeed}</b> holds — the span and the rungs come from the census, not from
        this page
      {:else if catalog.phase === 'failed'}
        {catalog.why}
      {:else if catalog.held.length === 0}
        <b>{activeFeed}</b> has no bars on disk, so there is nothing to sweep. Pull a month first —
        this page never defaults a span it cannot read.
      {:else}
        <!-- WHAT THE PRESS WILL ACTUALLY DO, not what the controls express.
             The selection above is real and the route is not there yet:
             `sweeprun::Asked` carries ONE `underlying` and no rung field at
             all, so a press sweeps the first selected instrument over every
             intraday rung whatever the chips say.

             Saying so is the whole point. A control that quietly does
             something other than what it shows is the failure §4 bans, and
             the fix while the gap exists is a sentence, not a disabled
             control -- the selection still travels in the request, so the
             day the route reads it nothing here has to change. -->
        this press sweeps <b>{sweepSymbol || '—'}</b> on <b>{activeFeed}</b>, every intraday
        timeframe, a pattern needing <b>20%</b> of that timeframe's own bars.
        {#if pickedSymbols.size > 1 || (heldNow && pickedRungs.size < heldNow.rungs.length)}
          <b class="warnish">The rest of the selection is not sent yet</b> — the run route takes one
          instrument and no timeframe list.
        {/if}
      {/if}
    </span>
  </section>

  <!-- ================================================================
       WHAT THE STORE HOLDS FOR THIS INSTRUMENT.
       The rungs, each with its OWN month count and bounds, because a rung
       is not obliged to cover the same span as its neighbour — INDIAVIX
       carries 121 months at `1day` and 119 at every intraday rung, and a
       row that averaged them would hide the two missing months.
       ================================================================ -->
  {#if catalog.phase === 'ready' && heldNow}
    <section class="coverbar">
      <div class="coverbar-head">
        <span class="coverbar-k">On disk</span>
        <b>{heldNow.leaf}</b>
        <span class="dim sm">{heldNow.full}</span>
        <!-- THE PRODUCT'S MONTH, HERE TOO. This line read `2016-08 → 2026-08`
             while the form two rows above it read `Aug 2016`, which is the
             same split this page was just fixed for -- reintroduced by the
             section that was added to fix it. -->
        <span class="dim sm">{monthLabel(heldNow.from)} → {monthLabel(heldNow.to)}</span>
        <span class="pill">{exact(heldNow.months)} months</span>
        <!-- NO `n rungs` PILL. The strip directly beneath is that count, named
             and measured; a pill saying `9 rungs` above nine labelled rungs is
             the same fact twice, and the second copy reads as a different one
             the reader then has to reconcile. -->

        {#if spanShortfall > 0}
          <span class="pill warn">
            asking for {exact(askedMonths ?? 0)} — {exact(spanShortfall)} fewer than the store holds
          </span>
          <button class="linky" onclick={useWholeSpan}>Use the whole span</button>
        {:else if askedMonths !== null && spanShortfall < 0}
          <span class="pill warn">
            asking for {exact(askedMonths)} months — more than the {exact(heldNow.months)} on disk
          </span>
        {/if}
      </div>
      <!-- THE TIMEFRAMES, AS A MEASUREMENT RATHER THAN A LIST OF WORDS.
           Every rung the run will cover, each with its own coverage gauge
           against the instrument's month count. The gauge is the fact: a rung
           that is short shows a short bar, and the eye reads nine bars faster
           than it reads nine numbers. There is no rung PICKER because
           `sweeprun::Asked` carries `feed`, `underlying`, `from` and `to` and
           no rung field -- the run sweeps every one of them, so a control
           offering a choice would be a control the route cannot honour. -->
      <!-- THE TIMEFRAMES ARE THE CONTROL, not a caption above one. They were a
           read-only strip that said what the run would cover; picking which
           ones to run needed a control that did not exist. Each chip is now a
           toggle, and it keeps its coverage gauge -- so the same element
           answers "what does the store hold here" and "is it in this run".

           The last selected chip will not turn itself off: a brute force over
           no timeframe is not a run, and refusing it in the control's SHAPE
           beats raising a validation message after the press. -->
      <div class="rungs-head">
        <span class="coverbar-k">Timeframes to sweep</span>
        <span class="dim sm">
          {exact(pickedRungs.size)} of {exact(heldNow.rungs.length)} — execution is always
          one-minute, whichever you pick
        </span>
        <button
          class="linky"
          onclick={() => (pickedRungs = new Set(heldNow.rungs.map((r) => r.name)))}
          disabled={pickedRungs.size === heldNow.rungs.length}>Select all</button
        >
      </div>
      <ul class="rungs">
        {#each heldNow.rungs as r, i (heldNow.leaf + r.name)}
          <li>
            <button
              type="button"
              class="rungchip"
              class:short={r.months < heldNow.months}
              class:on={pickedRungs.has(r.name)}
              style="--i:{i}"
              aria-pressed={pickedRungs.has(r.name)}
              title="{r.name} — {exact(r.months)} months on disk, {monthLabel(r.from)} to {monthLabel(
                r.to
              )}"
              onclick={() => (pickedRungs = toggled(pickedRungs, r.name))}
            >
              <span class="rungchip-top">
                <span class="rungchip-n">{r.name}</span>
                <span class="rungchip-m">{exact(r.months)}</span>
                {#if r.months < heldNow.months}
                  <span class="rungchip-w">−{exact(heldNow.months - r.months)}</span>
                {/if}
              </span>
              <span class="rungchip-track">
                <span
                  class="rungchip-fill"
                  style="width:{Math.max(2, Math.round((r.months / Math.max(1, heldNow.months)) * 100))}%"
                ></span>
              </span>
            </button>
          </li>
        {/each}
      </ul>
      <!-- TWO SHORT LINES, NOT NINETY WORDS OF GREY.
           This bar carried three dense paragraphs at 12px, and a wall of small
           prose above a form is not read -- it is skipped, which makes the
           facts in it worth nothing. Every sentence that survived is one an
           operator acts on; the provenance behind each moved to `title`, where
           the reader who wants it will look and the reader who does not is not
           charged for it. -->
      <p
        class="coverbar-note"
        title="Folded from /store.json — the census /db reads, one row per instrument, month and rung. Nothing in this bar is a default written into the page. Futures, options and single stocks may be stored and are never swept."
      >
        From the census on disk — <b>spot indices only</b>.
        {#if heldNow.rungs.some((r) => r.months < heldNow.months)}
          A rung marked <b>−n</b> is a <b>shorter sample</b>, not a corrected one.
        {/if}
      </p>
      <!-- THE SURFACE, IN THE SERVER'S WORDS AND NOT THIS PAGE'S. The store
           holds more spot indices than the engine sweeps, and which two are
           swept is `CLAUDE.md` §1 — enforced by `costs::venue`, stated by
           `/universes.json`. Printing the server's own sentence means the page
           can say it without holding a copy that goes stale. -->
      {#if sweptSurface && sweptSurface.note}
        <p
          class="coverbar-note surface"
          title="{sweptSurface.note}. Stated by /universes.json, enforced by costs::venue, and CLAUDE.md §1 is where it is decided. This page holds no list of its own — a run on an instrument outside the surface is refused by the route, and the refusal is printed above."
        >
          <span class="pill acc">
            {sweptSurface.label} · {exact(sweptSurface.matched)} of {exact(catalog.held.length)}
          </span>
          {sweptSurface.note} — anything else here is refused by the route.
        </p>
      {/if}
    </section>
  {/if}

  {#if blocked}
    <!-- ==============================================================
         THE LEDGER CANNOT TAKE A RESULT — said BEFORE the work, not after
         ==============================================================
         `Results::append` refuses when the file's version is not the one this
         build writes, and CLAUDE.md section 3 rule 8 is why: a 261-byte record
         appended to a file addressed every 213 bytes corrupts every record
         after it, and it still parses.

         Measured on this operator's store: version 2 against a build writing
         3. Pressing Run swept nine rungs over 121 months, refused every
         append, and refused the whole command -- after the work. Nothing was
         wrong with the engine and nothing was wrong with the refusal; the only
         defect was that it arrived hours late. -->
    <div class="panel bt-note blocked">
      <h2>This ledger cannot record a new run</h2>
      <p>
        The results file is <b>version {blocked.version ?? 'unknown'}</b> and this build writes
        <b>version {blocked.writes ?? 'a newer one'}</b>. A run appended at the wrong stride would
        corrupt every record after it <em>and still parse</em>, so the engine refuses instead — which
        is correct, but it refuses <b>after</b> the sweep has run.
      </p>
      <p>
        Run is disabled for that reason. Move the old file aside and the next sweep starts a fresh
        ledger. <b>Nothing is deleted</b> — the runs already recorded stay readable in the moved
        file, they simply leave this table.
      </p>
      {#if blocked.path}
        <pre class="cmd">mv {blocked.path} {blocked.path}.v{blocked.version ?? 'old'}</pre>
        <p class="path"><span class="bt-lab">file</span> <code>{blocked.path}</code></p>
      {/if}
    </div>
  {/if}

  {#if sweep.phase === 'running'}
    <!-- INDETERMINATE, AND SAYING SO. A sweep is 43 ms to hours depending on
         the rung; a percentage bar would be a guess and a spinner alone says
         nothing. The elapsed clock is the one honest progress signal. -->
    <p class="inline-note runstate">
      <span class="spin sm" aria-hidden="true"></span>
      <b>Sweeping.</b> The ladder walks upward until the frequent frontier empties, so how long
      it takes is decided by the data rather than by a setting — a one-day span finishes in
      milliseconds and a fine rung over seven years takes minutes per month. This page is
      polling and will refresh itself the moment a record lands.
      {#if sweep.run}
        <span class="dim">· started {when(sweep.run.started_micros)}</span>
      {/if}
    </p>
  {:else if sweep.phase === 'done' && sweep.run}
    <p class="inline-note runstate good">
      <b>Sweep finished</b> — {sweep.run.underlying} on {sweep.run.feed}, and the ledger below has
      been re-read.
    </p>
    <!-- THE NINE-RUNG TABLE, WHICH THE SERVER HAS ALWAYS SENT AND THIS PAGE
         HAS NEVER SHOWN.

         `/backtest/run.json` carries `report` — the exact text `cli range-all`
         prints, banner included — and nothing read it. That was not merely a
         wasted payload: `range_all` prints `REFUSED: why` on the row of any
         rung that refused while OTHERS succeeded, and that is a partial run
         with no whole-command refusal to catch it. The ledger gains rows, the
         green note above is true, and which rungs never ran was unknowable
         from this page. It is knowable here. -->
    {#if sweep.run.report}
      <pre class="runlog">{sweep.run.report}</pre>
    {/if}
  {:else if sweep.phase === 'failed'}
    <p class="inline-note bad runstate">
      <b>The sweep was refused.</b>
      {#if !sweep.run}{sweep.why}{/if}
    </p>
    <!-- A SERVER REFUSAL IS MULTI-LINE AND NAMES THE FIRST CAUSE ON ITS OWN
         LINE, so it goes in the block that preserves newlines rather than
         being collapsed into the sentence above. A refusal raised HERE — a
         malformed month, a 409, a dead fetch — is one line and stays inline. -->
    {#if sweep.run}
      <pre class="runlog bad">{sweep.why}</pre>
    {/if}
  {/if}

  {#if load.phase === 'loading'}
    <!-- ==============================================================
         LOADING — a state, with a sentence
         ============================================================== -->
    <div class="panel wait">
      <span class="spin" aria-hidden="true"></span>
      <p>Reading the results ledger at a computed offset. This is one seek per run, not a scan.</p>
    </div>
  {:else if load.phase === 'failed'}
    <!-- ==============================================================
         THE REQUEST ITSELF FAILED — distinct from an empty ledger
         ============================================================== -->
    <div class="panel err">
      <h2>The ledger could not be asked for</h2>
      <p>{load.why}</p>
      <button class="btn" onclick={fetchLedger}>Try again</button>
    </div>
  {:else if refusal}
    <!-- ==============================================================
         THE SERVER REFUSED, AND SAID WHY. Never a blank table.
         ============================================================== -->
    <div
      class="panel"
      class:err={!refusal.includes('not an error')}
      class:bt-note={refusal.includes('not an error')}
    >
      <h2>
        {refusal.includes('not an error')
          ? 'Nothing has been swept yet'
          : 'The ledger could not be read'}
      </h2>
      <p>{refusal}</p>
      {#if ledger?.path}
        <p class="path"><span class="bt-lab">file</span> <code>{ledger.path}</code></p>
      {/if}
    </div>
  {:else}
    <!-- ==============================================================
         INTEGRITY — shown ONLY when it has something to say

         Every other panel on this page renders its zero: "0 halted", "0
         span holes". This one does not, and the difference is deliberate.
         Halting and short spans are ORDINARY outcomes of ordinary
         operation, so their zero is information — it says the sweep had
         room. A broken seal is not ordinary; on a healthy store this reads
         zero forever, and a figure that reads zero forever trains the eye
         to skip it. By the time it matters, it has been furniture for
         months.

         So it is absent until it is not, and when it appears it is a
         full-width bar above the summary rather than a fourth tile inside
         it — because the summary is where the eye goes to be reassured,
         and this is the one thing on the page that must interrupt.
         ============================================================== -->
    {#if unsealedRuns.length > 0}
      <section class="bt-integrity" role="alert">
        <span class="bt-integrity-mark" aria-hidden="true">⚠</span>
        <div class="bt-integrity-say">
          <strong>
            {exact(unsealedRuns.length)}
            {unsealedRuns.length === 1 ? 'record' : 'records'} failed the integrity seal
          </strong>
          <span>
            Each one read back whole and every field holds a legal value — which is exactly why
            this check exists. The bytes do not match the <code>blake3</code> seal written beside
            them, so they are shown, marked, and kept out of every ranking on this page. Treat
            their figures as unknown rather than wrong, and run the sweep again to replace them.
          </span>
        </div>
      </section>
    {/if}

    <!-- ==============================================================
         SUMMARY — the four facts before any detail
         ============================================================== -->
    <section class="bt-strip">
      <div class="fact">
        <span class="k">Runs recorded</span>
        <span class="v">{exact(ledger.total)}</span>
        <span class="n">
          {#if ledger.hit_scan_cap}
            showing the newest {exact(ledger.scanned)} — the read stopped at its ceiling
          {:else}
            all of them read
          {/if}
        </span>
      </div>
      <div class="fact" class:good={completeRuns.length > 0}>
        <span class="k">Complete</span>
        <span class="v">{exact(completeRuns.length)}</span>
        <span class="n">
          {#if unsealedRuns.length > 0}
            ran to extinction AND read back intact
          {:else}
            comparable against each other
          {/if}
        </span>
      </div>
      <div class="fact" class:warn={haltedRuns.length > 0} class:nil={haltedRuns.length === 0}>
        <span class="k">Halted</span>
        <span class="v">{exact(haltedRuns.length)}</span>
        <span class="n">
          {haltedRuns.length === 0
            ? 'no ladder was cut short'
            : 'excluded from ranking — depth is partial'}
        </span>
      </div>
      <div class="fact" class:warn={holedRuns.length > 0} class:nil={holedRuns.length === 0}>
        <span class="k">Span holes</span>
        <span class="v">{exact(holedRuns.length)}</span>
        <span class="n">
          {holedRuns.length === 0 ? 'every span was whole' : 'a shorter sample, not a corrected one'}
        </span>
      </div>
    </section>

    {#if ledger.partial_tail}
      <p class="inline-note warn">
        The last bytes of the ledger are not a whole record — a writer was interrupted mid-append.
        Every whole record before it is shown; the ragged tail is left alone, because this page
        never writes to that file.
      </p>
    {/if}

    {#if hiddenByFeed > 0}
      <p class="inline-note">
        {exact(hiddenByFeed)}
        {hiddenByFeed === 1 ? 'run is' : 'runs are'} recorded under another feed and not shown.
        <button class="linky" onclick={() => (everyFeed = true)}>Show every feed</button>
      </p>
    {:else if everyFeed && activeFeed}
      <p class="inline-note">
        Showing every feed.
        <button class="linky" onclick={() => (everyFeed = false)}>Back to {activeFeed}</button>
      </p>
    {/if}

    {#if runs.length === 0}
      <!-- ============================================================
           THE FEED FILTER EMPTIED THE TABLE — a fact, with the way out
           ============================================================ -->
      <!-- TWO EMPTY STATES, AND THEY ARE NOT THE SAME FACT.
           "the ledger is empty" and "this feed has none of the runs the
           ledger holds" have different causes and different fixes, and the
           second one blames a filter that is not responsible when the first
           is true. This shipped conflated: with zero runs recorded it
           announced "No runs under zerodha", which reads as a feed problem
           and sends the operator to change a picker that will not help. -->
      {#if allRuns.length === 0}
        <div class="panel bt-note">
          <h2>Nothing has been swept yet</h2>
          <p>
            The results ledger exists and is readable — it is simply empty. It is written by
            <code>cli</code> when a sweep completes, not by this server, so the way to fill it is
            to run one:
          </p>
          <!-- THE EXAMPLE WAS `cli range-all zerodha NIFTY 2019 12 2026 8 500`,
               and every argument in it was a literal. A copyable command that
               names a feed the operator is not on, an instrument the store may
               not hold and a span 40 months short of what is on disk is worse
               than no example: it looks authoritative and it is a guess. It is
               built from the census now, and when the census cannot be read the
               page says that rather than printing a command it cannot stand
               behind. -->
          {#if activeFeed && heldNow && askedMonths !== null}
            <pre class="cmd">{sweepCommand}</pre>
          {:else}
            <p class="inline-note">
              The command that fills this ledger names a feed, an instrument and a span. This page
              cannot print one until the census says what is on disk — see the note in the sweep bar
              above.
            </p>
          {/if}
          <p>
            Every run that finishes appends one record here and appears on this page on the next
            read. <b>This is not an error</b> — an empty ledger and an unreadable one are different
            facts, and this is the first.
          </p>
          {#if ledger?.path}
            <p class="path"><span class="bt-lab">file</span> <code>{ledger.path}</code></p>
          {/if}
        </div>
      {:else}
        <div class="panel bt-note">
          <h2>No runs under {activeFeed || 'this feed'}</h2>
          <p>
            The ledger holds {exact(allRuns.length)}
            {allRuns.length === 1 ? 'run' : 'runs'}, none of them recorded against
            <b>{activeFeed}</b>. A sweep is stamped with the feed its bars came from, so a run under
            another vendor is a different run and not this one seen differently.
          </p>
          <button class="btn" onclick={() => (everyFeed = true)}>Show every feed</button>
        </div>
      {/if}
    {:else}
      <!-- ============================================================
           LEVEL 1 — THE ANSWER
           ============================================================ -->
      <section class="block">
        <h2 class="bh">The answer</h2>
        {#if best}
          <div class="crown">
            <div class="crown-mark">best complete run</div>
            <div class="crown-body">
              <div class="crown-id">
                <b>{best.underlying}</b>
                <span class="rung">{best.timeframe}</span>
                <span class="dim">{best.feed}</span>
                <span class="dim">{span(best)}</span>
              </div>
              <div class="crown-figs">
                <div class="fig lead">
                  <span class="k">worst-case total</span>
                  <span class="v up">{money(best.pessimistic)}</span>
                </div>
                <div class="fig">
                  <span class="k">best-case total</span>
                  <span class="v ghost">{money(best.optimistic)}</span>
                </div>
                <div class="fig">
                  <span class="k">trades</span>
                  <span class="v">{exact(best.trades)}</span>
                </div>
                <div class="fig">
                  <span class="k">combinations</span>
                  <span class="v">{exact(best.combinations)}</span>
                </div>
              </div>
              <p class="crown-note">
                Ranked on the worst-case number. The best case is
                <b>{money(best.optimistic - best.pessimistic)}</b> higher, which is how much of
                the headline is fill assumption rather than edge.
              </p>
            </div>
            <button class="btn ghost sm" onclick={() => toggle(best)}>
              {openIndex === best.index ? 'Close' : 'Drill in'}
            </button>
          </div>
        {:else}
          <div class="panel bt-note">
            <h2>NO COMPLETE RUN</h2>
            <p>
              {#if haltedRuns.length > 0}
                All {exact(haltedRuns.length)}
                {haltedRuns.length === 1 ? 'run' : 'runs'} shown were halted by a budget, so every
                one of them searched less of the ladder than its <code>combinations</code> figure
                suggests. None is comparable with a complete run, so none is crowned. Sweeping
                without a budget cap is what fills this slot.
              {:else}
                There is nothing to rank.
              {/if}
            </p>
          </div>
        {/if}
      </section>

      <!-- ============================================================
           LEVEL 2 — NINE RUNGS SIDE BY SIDE
           ============================================================ -->
      <section class="block">
        <h2
          class="bh"
          title="One row per comparable span — same feed, instrument, window and support ratio. Bars share one scale within a row, so height is comparable across rungs. A rung never swept for a span is absent rather than zero."
        >
          Which rung carries the edge
        </h2>

        {#each rungGroups as g (g.key)}
          <div class="rgroup">
            <div class="rgroup-head">
              <b>{g.underlying}</b>
              <span class="dim">{g.feed}</span>
              <!-- THE GROUP HEADER BUILDS ITS SPAN SEPARATELY from `span(r)`,
                   so fixing that helper left this one line still printing the
                   store's key. Same month, same product, same format. -->
              <span class="dim">{monthLabel(g.from)} → {monthLabel(g.to)}</span>
              <span class="dim">
                <!-- "support ≥ 20.0% of bars" is the engine's word for it. What
                     it MEANS is how often a pattern had to show up before the
                     sweep would keep it, and that is what the reader needs. -->
                {g.perMille < 0
                  ? 'how often a pattern had to appear — unknown, no bars were read'
                  : `a pattern had to appear on ${(g.perMille / 10).toFixed(1)}% of bars to be kept`}
              </span>
              <span class="pill">
                {g.present.length}
                {g.present.length === 1 ? 'rung' : 'rungs'} swept at this ratio
              </span>
              {#if g.haltedCount > 0}
                <span class="pill warn">{g.haltedCount} halted</span>
              {/if}
              {#if g.unsealedCount > 0}
                <span class="pill seal">{g.unsealedCount} unsealed</span>
              {/if}
            </div>
            <!-- SOLO IS NOT A CHART, AND THE BAR WOULD BE A TAUTOLOGY.
                 `scale` is `max(|pessimistic|)` over the rungs present, so with
                 ONE rung the scale IS that rung and the bar is always exactly
                 100% tall — the same picture for ₹13,504 as for ₹1. A shape
                 that cannot vary is not a measurement, and drawing it in a
                 1375px band with a 92px tile in it spends 93% of the row on
                 nothing while looking like a comparison that was made.
                 The row still ranks, still opens, still carries its flags; it
                 simply stops pretending to plot. -->
            <div class="multiples" class:solo={g.present.length === 1}>
              {#each g.present as r (r.index)}
                <button
                  class="mult"
                  class:halted={r.halted}
                  class:unsealed={!trustworthy(r)}
                  class:leader={g.leader && r.index === g.leader.index}
                  onclick={() => toggle(r)}
                  title={!trustworthy(r)
                    ? 'Failed its integrity seal — this figure may not be what the sweep wrote, so it is charted but never ranked'
                    : r.halted
                      ? 'Halted — not comparable with the complete rungs beside it'
                      : `Worst-case ${money(r.pessimistic)} over ${exact(r.trades)} trades`}
                >
                  <!-- A FLOOR OF 4%, SO NOTHING VANISHES. On this store's own
                       ledger 15min returns twenty times what 1day does, which
                       put the daily bar at three pixels and the eye read it as
                       an empty slot rather than a small number. A floor is not
                       a distortion as long as the FIGURE is printed beneath it,
                       which it is — the bar ranks, the number states.

                       The best-case fill is an OUTLINE rather than a filled
                       block: it is always the taller of the two, so as a solid
                       it covered the number that actually ranks. -->
                  <span class="mult-bars">
                    <span
                      class="mult-ghost"
                      style="height:{Math.max(4, Math.min(100, (Math.abs(r.optimistic) / g.scale) * 100))}%"
                    ></span>
                    <span
                      class="mult-bar"
                      class:neg={r.pessimistic < 0}
                      style="height:{Math.max(4, Math.min(100, (Math.abs(r.pessimistic) / g.scale) * 100))}%"
                    ></span>
                  </span>
                  <span class="mult-rung">{r.timeframe}</span>
                  <span class="mult-fig">{money(r.pessimistic)}</span>
                  {#if !trustworthy(r)}<span class="mult-flag seal">seal</span>{:else if r.halted}<span
                      class="mult-flag">halted</span>{/if}
                </button>
              {/each}
            </div>
            {#if !g.leader}
              <p class="rgroup-note warn">
                Every rung in this span was halted, so no rung can be called the leader.
              </p>
            {:else if g.present.length === 1}
              <!-- ONE RUNG IS NOT A COMPARISON, and saying "1day carries the
                   most" of a set of one would be true and useless. The
                   sentence names what is missing instead. -->
              <p class="rgroup-note">
                Only <b>{g.leader.timeframe}</b> has been swept at this span and ratio, so there is
                nothing to compare it against. <code>cli range-all</code> is what fills the rest of
                this row.
              </p>
            {:else}
              <p class="rgroup-note">
                <b>{g.leader.timeframe}</b> carries the most under worst-case fills across the
                {g.present.length} rungs swept here: {money(g.leader.pessimistic)} over {exact(
                  g.leader.trades
                )} trades.
              </p>
            {/if}
          </div>
        {:else}
          <p class="inline-note">
            No run in view can be grouped for comparison. Every group needs at least one run, so
            this is only reachable when the table above is itself empty.
          </p>
        {/each}
      </section>

      <!-- ============================================================
           LEVEL 0 — THE LEDGER
           ============================================================ -->
      <section class="block">
        <div class="bh-row">
          <h2 class="bh">The ledger</h2>
          <input
            class="find"
            type="search"
            placeholder="instrument, feed or rung…"
            bind:value={query}
            aria-label="Filter runs"
          />
          <span class="count">{exact(sorted.length)} of {exact(runs.length)} shown</span>
        </div>

        {#if sorted.length === 0}
          <p class="inline-note">
            Nothing matches <b>{query}</b>. The filter is a prefix probe over instrument, feed and
            rung — it does not search identities.
          </p>
        {:else}
          <div class="tbl-scroll" onscroll={onScroll}>
            <table class="tbl">
              <thead>
                <tr>
                  <!-- ONE LOOP, SO THE HEADER CANNOT DISAGREE WITH ITSELF.
                       Nine hand-written cells is nine chances for a column to
                       sort on a key it does not display, and for `aria-sort`
                       to be right on eight of them. -->
                  {#each COLUMNS as col (col.label)}
                    <th
                      class:num={col.num}
                      aria-sort={col.key === null
                        ? undefined
                        : sort.col === col.key
                          ? sort.desc
                            ? 'descending'
                            : 'ascending'
                          : 'none'}
                    >
                      {#if col.key === null}
                        <span class="nosort">{col.label}</span>
                      {:else}
                        <button
                          class="sortbtn"
                          class:active={sort.col === col.key}
                          onclick={() => sortBy(col.key)}
                          title={col.why}
                        >
                          {col.label}<span class="caret" aria-hidden="true"
                            >{sort.col === col.key ? (sort.desc ? '▾' : '▴') : '⋅'}</span
                          >
                        </button>
                      {/if}
                    </th>
                  {/each}
                </tr>
              </thead>
            </table>
            <div class="bt-spacer" style="height:{spacerH}px">
              {#each windowed as r, i (r.index)}
                <div
                  class="row"
                  class:halted={r.halted}
                  class:unsealed={!trustworthy(r)}
                  class:crowned={best && r.index === best.index}
                  class:open={openIndex === r.index}
                  style="top:{(firstVisible + i) * ROW_H}px"
                  role="button"
                  tabindex="0"
                  aria-expanded={openIndex === r.index}
                  aria-label={rowLabel(r)}
                  onclick={() => toggle(r)}
                  onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && toggle(r)}
                >
                  <span class="cell inst"
                    ><b>{r.underlying}</b><span class="dim sm">{r.feed}</span></span
                  >
                  <span class="cell"><span class="rung">{r.timeframe}</span></span>
                  <span class="cell dim sm">{span(r)}</span>
                  <span class="cell">
                    <span class="cover" title="{r.months_found} of {r.months_asked} months on disk">
                      <span
                        class="cover-fill"
                        class:short={!r.whole_span}
                        style="width:{coverPct(r)}%"
                      ></span>
                    </span>
                    <span class="sm dim">{r.months_found}/{r.months_asked}</span>
                  </span>
                  <span class="cell">
                    {#if !trustworthy(r)}
                      <!-- SEAL BEFORE DEPTH. A row that failed its seal has a
                           `depth` that is itself one of the bytes in doubt, so
                           printing "depth 15 partial" would be reporting a
                           number as the reason it cannot be trusted. -->
                      <span class="pill seal">NO · seal failed</span>
                    {:else if r.halted}
                      <span class="pill warn">NO · depth {r.depth} partial</span>
                    {:else}
                      <span class="pill good">yes · depth {r.depth}</span>
                    {/if}
                  </span>
                  <span class="cell num">{exact(r.trades)}</span>
                  <span class="cell num strong" class:neg={r.pessimistic < 0}
                    >{money(r.pessimistic)}</span
                  >
                  <span class="cell num dim">{money(r.optimistic)}</span>
                  <span class="cell dim sm">{when(r.finished_micros)}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </section>

      <!-- ============================================================
           LEVELS 3–7 — THE DRILL-DOWN
           ============================================================ -->
      {#if openRun}
        <section class="block drill">
          <div class="bh-row">
            <h2 class="bh">{openRun.underlying} · {openRun.timeframe} · {span(openRun)}</h2>
            <button class="btn ghost sm" onclick={() => (openIndex = null)}>Close</button>
          </div>

          {#if contradicts(openRun)}
            <!-- A CONTRADICTION IN THE RECORD, not an unusual result. Named
                 rather than plotted: the ghost bar would simply render shorter
                 and read as a merely surprising run. -->
            <p class="inline-note bad">
              <b>This record contradicts itself.</b> Best-case fills total
              {money(openRun.optimistic)}, which is LESS than the worst-case
              {money(openRun.pessimistic)} — and cannot be, because both price the same
              trades over the same bars and one end of a bar is never worse than the other. One
              of the two figures is wrong at the point it was written. Nothing below can be
              trusted for this run.
            </p>
          {/if}
          {#if openRun.trades === 0}
            <p class="inline-note warn">
              This run took <b>no trades at all</b>. Its totals are therefore zero by absence,
              not by breaking even: the combination it chose never fired an entry over
              {exact(openRun.bars)} bars. A zero here is not a result to compare.
            </p>
          {/if}
          {#if openRun.halted}
            <p class="inline-note warn">
              This run was <b>halted by a budget</b>. Its depth of {openRun.depth} is as far as the
              ladder got, not as far as it goes, and its {exact(openRun.combinations)} combinations
              cover less of the search than that figure suggests while reading larger. Nothing below
              is comparable with a complete run.
            </p>
          {/if}
          {#if !openRun.whole_span}
            <p class="inline-note warn">
              The store held {openRun.months_found} of the {openRun.months_asked} months this span
              asked for. Every figure below is over the <b>shorter sample</b> — it is not a
              corrected one, and the missing months are not zeros.
            </p>
          {/if}

          <!-- ==========================================================
          <!-- ==========================================================
               THE PRICE TERMINAL
               A panel, not a thumbnail: a live OHLC legend bound to the
               crosshair, a rung switcher over the rungs the STORE holds,
               range presets, and a stats strip folded from the bars on
               screen. Every number in it is a value in the file.
               ========================================================== -->
          <div class="term">
            <div class="term-bar">
              <div class="term-id">
                <b>{openRun.underlying}</b>
                <span class="term-feed">{openRun.feed}</span>
                {#if chartRung !== openRun.timeframe}
                  <span class="pill warn" title="The run was swept at {openRun.timeframe}">
                    viewing {chartRung} · run is {openRun.timeframe}
                  </span>
                {/if}
              </div>
              <!-- THE RUNG SWITCHER, over what the store HOLDS rather than
                   over a list written here. A ✓ marks a rung that carries a
                   recorded run, so "I can look at 5-minute price but nothing
                   has been swept there" is readable at a glance. -->
              {#if storeRungs.length > 0}
                <div class="rungs" role="group" aria-label="Timeframe">
                  {#each storeRungs as r (r.name)}
                    <button
                      class="rungbtn"
                      class:on={chartRung === r.name}
                      class:swept={sweptRungs.has(r.name)}
                      onclick={() => (chartRung = r.name)}
                      title="{r.months} months on disk{sweptRungs.has(r.name)
                        ? ' · a run is recorded at this rung'
                        : ' · no run recorded at this rung'}"
                    >
                      {r.name}{#if sweptRungs.has(r.name)}<span class="tick" aria-hidden="true">✓</span>{/if}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>

            <!-- THE LEGEND. Reads the paisa integers, never the chart's
                 floats: only the TIME comes back off the crosshair. -->
            <div class="legend" aria-live="off">
              {#if ohlc}
                {@const up = ohlc.c >= ohlc.o}
                {@const bps = ohlc.o > 0 ? Math.round(((ohlc.c - ohlc.o) / ohlc.o) * 10_000) : null}
                <span class="lg-t">{when(ohlc.t * 1_000_000)}</span>
                <span class="lg"><i>O</i>{money(ohlc.o)}</span>
                <span class="lg"><i>H</i>{money(ohlc.h)}</span>
                <span class="lg"><i>L</i>{money(ohlc.l)}</span>
                <span class="lg"><i>C</i>{money(ohlc.c)}</span>
                <!-- ONE SIGN, NOT TWO. This carried a `+` from the up/down
                     test AND another from the sign of the ratio, so every
                     rising bar read `++0.14%`. The sign now comes from the
                     number alone. Basis points as an integer, divided once
                     for display — the ratio is not a float until it is
                     printed. -->
                <span class="lg-chg" class:up class:down={!up}>
                  {#if bps === null}—{:else}{bps >= 0 ? '+' : ''}{(bps / 100).toFixed(2)}%{/if}
                </span>
              {:else if windowStats}
                <span class="lg-t">hover the chart for a bar</span>
                <span class="lg"><i>HIGH</i>{money(windowStats.hi)}</span>
                <span class="lg"><i>LOW</i>{money(windowStats.lo)}</span>
                <span class="lg"><i>RANGE</i>{money(windowStats.range)}</span>
                {#if windowStats.netBps !== null}
                  <span
                    class="lg-chg"
                    class:up={windowStats.netBps >= 0}
                    class:down={windowStats.netBps < 0}
                  >
                    {windowStats.netBps >= 0 ? '+' : ''}{(windowStats.netBps / 100).toFixed(2)}%
                    over the window
                  </span>
                {/if}
              {:else}
                <span class="lg-t">no bars on screen</span>
              {/if}
            </div>

            {#if series.phase === 'loading'}
              <div class="chart-state">
                <span class="spin" aria-hidden="true"></span>
                <p>
                  Reading {span(openRun)} at {chartRung} off the store — the same files the sweep
                  read.
                </p>
              </div>
            {:else if series.phase === 'failed'}
              <div class="chart-state bad">
                <p><b>The price series could not be read.</b> {series.why}</p>
              </div>
            {:else if series.phase === 'ready' && series.bars.length === 0}
              <div class="chart-state">
                <p>
                  <b>The store returned no bars at {chartRung} for this span.</b> The run's figures
                  above were computed when the sweep ran and are unaffected.
                </p>
              </div>
            {:else if chartError}
              <div class="chart-state bad">
                <p>
                  <b>The chart library did not load</b>, so the price cannot be drawn:
                  {chartError}. Every figure above is unaffected — none of them comes from the
                  chart.
                </p>
              </div>
            {:else if series.phase === 'ready'}
              <div
                class="bt-chart"
                bind:this={chartHost}
                role="img"
                aria-label="Candlestick chart of {openRun.underlying} at the {chartRung} rung, {exact(
                  series.bars.length
                )} bars ending {span(openRun)}. Prices in rupees. No entries or exits are marked because the ledger records no trade list."
              ></div>
            {/if}

            <div class="term-foot">
              <div class="term-facts">
                {#if series.phase === 'ready' && series.bars.length > 0}
                  {#if series.total > series.bars.length}
                    <span class="pill warn">newest {exact(series.bars.length)} of {exact(series.total)}</span>
                  {:else}
                    <span class="pill good">all {exact(series.total)} bars</span>
                  {/if}
                  <span class="fnote">{exact(series.months_read)} months read</span>
                  {#if series.months_missing > 0}
                    <span class="pill warn">{exact(series.months_missing)} months missing</span>
                  {:else}
                    <span class="fnote">no month missing</span>
                  {/if}
                  <!-- VOLUME IS NOT DRAWN BECAUSE THERE IS NONE. Every bar in
                       this index series carries v=0: NSE publishes no volume
                       on a spot index. An empty histogram pane would read as
                       "no trading happened", which is a different claim. -->
                  <span class="fnote">no volume pane — a spot index publishes none</span>
                {/if}
              </div>
              {#if presets.length > 1 && series.phase === 'ready'}
                <div class="ranges" role="group" aria-label="Visible range">
                  {#each presets as p (p.label)}
                    <button class="rangebtn" onclick={() => showLast(p.bars)}>{p.label}</button>
                  {/each}
                </div>
              {/if}
            </div>

            {#if series.phase === 'ready' && series.bars.length > 0}
              <p class="cnote faint term-note">
                <b>No entries or exits are marked, and that is deliberate.</b> The ledger records
                {exact(openRun.trades)} trades as a COUNT and keeps no trade list, so there is no
                timestamp to put a marker at. A chart with plausible markers would look like the
                answer to "when did it trade", which nothing on disk can answer. No indicator is
                overlaid either — one computed here would be a second implementation of something
                <code>crates/indicators</code> already owns.
              </p>
            {/if}
          </div>

          <!-- ==========================================================
               THE STRATEGY TESTER
               Built to the operator's screenshots: TradingView's toolbar,
               its three views, its five and three pill tabs, its stat
               quads with the percentage under the absolute, its chart
               frames with axes and legends and toggles, and its padlock
               for a cell that cannot be filled.

               A CHART WITHOUT A SOURCE STILL DRAWS ITS FRAME. Axes,
               legend, period toggles and pagers are all present and all
               work; what is missing is the series, and the plot area says
               so. That is the same design with an honest interior, and it
               is the only version of this panel that cannot mislead
               somebody about money.
               ========================================================== -->
          <section class="tester">
            <!-- ---- toolbar ---- -->
            <div class="tt-bar">
              <div class="tt-name">
                <svg viewBox="0 0 16 16" class="tt-ico" aria-hidden="true"
                  ><path d="M2 12l3.5-4 3 3L13 4" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg
                >
                <b>Brute-force sweep</b>
                <span class="tt-dim">{openRun.underlying} · {openRun.timeframe}</span>
              </div>

              <div class="tt-views" role="group" aria-label="Report view">
                <button class="tt-view ic" class:on={testerView === 'metrics'} onclick={() => (testerView = 'metrics')} title="Metrics">
                  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M2 11l3.5-4.5L8.5 9 14 3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
                </button>
                <button class="tt-view ic" class:on={testerView === 'trades'} onclick={() => (testerView = 'trades')} title="List of trades">
                  <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="2" y="3" width="12" height="10" rx="1" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M2 6.5h12M6 6.5V13" stroke="currentColor" stroke-width="1.1" /></svg>
                </button>
                <button class="tt-view" class:on={testerView === 'properties'} onclick={() => (testerView = 'properties')}>Properties</button>
              </div>

              <!-- TESTING PERIOD, with TradingView's own menu -->
              <div class="tt-period">
                <button class="tt-ctl" onclick={() => (periodOpen = !periodOpen)} aria-expanded={periodOpen}>
                  <svg viewBox="0 0 16 16" class="tt-cico" aria-hidden="true"><rect x="2" y="3" width="12" height="11" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M2 6.5h12M5.5 2v2.5M10.5 2v2.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" /></svg>
                  {span(openRun)}
                  <span class="tt-caret" aria-hidden="true">⌄</span>
                </button>
                {#if periodOpen}
                  <div class="tt-menu">
                    <div class="tt-menuhead">
                      <span>Testing period</span>
                      <button class="tt-reset" onclick={() => { testingPeriod = 'Available chart range'; periodOpen = false; }}>Reset</button>
                    </div>
                    {#each TESTING_PERIODS as p (p)}
                      <button
                        class="tt-menuitem"
                        class:sel={testingPeriod === p}
                        onclick={() => { testingPeriod = p; periodOpen = false; }}
                      >
                        {p}
                        {#if p === 'Available chart range'}<span class="tt-default">Default</span>{/if}
                        {#if p !== 'Available chart range'}<Lock small why="The run's span is fixed at the moment the sweep ran and recorded. Re-testing a different window means running the sweep again, not re-reading this record." />{/if}
                      </button>
                    {/each}
                    <!-- TradingView's last item sits under a divider and
                         carries a calendar. -->
                    <div class="tt-menudiv"></div>
                    <button class="tt-menuitem">
                      <span class="tt-menuic">
                        <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="2" y="3" width="12" height="11" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M2 6.5h12M5.5 2v2.5M10.5 2v2.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" /></svg>
                        Custom date range
                      </span>
                      <Lock small why="A different window is a different sweep, not a different reading of this record. The span is one of the nine terms in the run's identity." />
                    </button>
                  </div>
                {/if}
              </div>

              <button class="tt-ctl" title="Initial capital">
                <svg viewBox="0 0 16 16" class="tt-cico" aria-hidden="true"><circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M8 5v6M6.3 6.5h3.4M6.3 9.5h3.4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></svg>
                1 unit <span class="tt-dim2">index points</span>
              </button>
              <button class="tt-ctl" title="Detalization">
                <svg viewBox="0 0 16 16" class="tt-cico" aria-hidden="true"><path d="M3 13V7M8 13V3M13 13V9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg>
                {openRun.timeframe} signal · 1min execution
              </button>

              <div class="tt-meta">
                {#if openRun.halted}
                  <span class="tt-chip warn">halted · depth {openRun.depth} partial</span>
                {:else}
                  <span class="tt-chip good">complete · depth {openRun.depth}</span>
                {/if}
                <span class="tt-chip" class:warn={!openRun.whole_span}>{exact(openRun.months_found)}/{exact(openRun.months_asked)} months</span>
              </div>
            </div>

            {#if testerView === 'metrics'}
              <!-- ================= KEY STATS ================= -->
              <div class="tt-sec">
                <h4 class="tt-h">Key stats</h4>
                <div class="tt-quad">
                  <div class="tt-q">
                    <span class="tt-k">Total PnL</span>
                    <span class="tt-qv big" class:up={openRun.pessimistic >= 0} class:down={openRun.pessimistic < 0}>
                      {money(openRun.pessimistic)}<em class="tt-unit">POINTS</em>
                      <em class="tt-pc">{pct(strategyBps)}</em>
                    </span>
                    <span class="tt-note">worst-case fills · {money(openRun.optimistic)} at best</span>
                  </div>
                  <div class="tt-q">
                    <span class="tt-k">Max drawdown</span>
                    <span class="tt-qv big down">
                      {money(Math.abs(openRun.max_drawdown))}<em class="tt-unit">POINTS</em>
                      <em class="tt-pc">{drawdownBps === null ? '' : `${(drawdownBps / 100).toFixed(2)}%`}</em>
                    </span>
                    <span class="tt-note">worst peak-to-trough</span>
                  </div>
                  <!-- TWO VALUES SIDE BY SIDE, as TradingView sets this one:
                       the percentage and the fraction it came from. The
                       denominator is recorded and the numerator is not, so the
                       fraction is drawn with the half that exists. -->
                  <div class="tt-q">
                    <span class="tt-k">Profitable trades</span>
                    <span class="tt-qv big">
                      <Lock why="No win count is recorded. A net total cannot be split into winners and losers after the fact." />
                      <em class="tt-frac"><Lock small why="The numerator — how many of these trades won — is not recorded." />/{exact(openRun.trades)}</em>
                    </span>
                    <span class="tt-note">of {exact(openRun.trades)} closed</span>
                  </div>
                  <div class="tt-q">
                    <span class="tt-k">Profit factor</span>
                    <span class="tt-qv big"><Lock why="Needs gross profit and gross loss separately; the sweep records only the net." /></span>
                    <span class="tt-note">gross profit ÷ gross loss</span>
                  </div>
                </div>
              </div>

              <!-- ================= PERFORMANCE ================= -->
              <div class="tt-sec">
                <div class="tt-hrow">
                  <h4 class="tt-h">Performance <button class="tt-info" title="What these four plots are" aria-label="About the performance plots"><svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M8 7.2v4M8 4.9v.1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg></button></h4>
                  <div class="tt-icons">
                    <button class="tt-iconbtn" title="Chart settings" aria-label="Chart settings"><svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="2.2" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M8 1.6v2M8 12.4v2M1.6 8h2M12.4 8h2M3.5 3.5l1.4 1.4M11.1 11.1l1.4 1.4M12.5 3.5l-1.4 1.4M4.9 11.1l-1.4 1.4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></svg></button>
                    <button class="tt-iconbtn" title="Snapshot" aria-label="Snapshot"><svg viewBox="0 0 16 16"><rect x="1.8" y="4.5" width="12.4" height="8.5" rx="1.4" fill="none" stroke="currentColor" stroke-width="1.3" /><circle cx="8" cy="8.7" r="2.4" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M5.6 4.5l1-1.5h2.8l1 1.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" /></svg></button>
                    <button class="tt-iconbtn" title="Expand" aria-label="Expand"><svg viewBox="0 0 16 16"><path d="M6 2H2v4M10 14h4v-4" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" /></svg></button>
                  </div>
                </div>
                <div class="tt-perf">
                  <!-- TradingView lists the plots as PLAIN TEXT down the left
                       of the chart, each with an eye that toggles it, and a
                       collapse chevron underneath. No boxes, no bullets. -->
                  <div class="tt-plots">
                    <div class="tt-plotrow off">
                      <span>Cumulative PnL</span>
                      <Lock small why="No equity series is recorded — four scalars cannot make a curve." />
                    </div>
                    <div class="tt-plotrow">
                      <span>Buy and hold</span>
                      <button class="tt-eye" title="Shown" aria-label="Buy and hold is shown">
                        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M1.5 8s2.4-4 6.5-4 6.5 4 6.5 4-2.4 4-6.5 4S1.5 8 1.5 8z" fill="none" stroke="currentColor" stroke-width="1.3" /><circle cx="8" cy="8" r="1.9" fill="currentColor" /></svg>
                      </button>
                    </div>
                    <div class="tt-plotrow off">
                      <span>Trades excursions</span>
                      <Lock small why="Three aggregates are recorded, not one column per trade." />
                    </div>
                    <div class="tt-plotrow off">
                      <span>Run-ups and drawdowns</span>
                      <Lock small why="One drawdown figure is recorded, not a series over time." />
                    </div>
                    <button class="tt-collapse" aria-label="Collapse plot list">
                      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 10l4-4 4 4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
                    </button>
                  </div>
                  <div class="tt-plot">
                    {#if bench.phase === 'loading'}
                      <div class="tt-empty"><span class="spin sm" aria-hidden="true"></span> Reading the span's opening price…</div>
                    {:else if outperformance && buyHold}
                      <div class="tt-bench">
                        <div class="tt-brow">
                          <span class="tt-blab">Strategy</span>
                          <span class="tt-bbar"><span class="tt-bfill" class:down={outperformance.strategy < 0} style="width:{(Math.abs(outperformance.strategy) / benchScale) * 100}%"></span></span>
                          <span class="tt-bval strong">{money(outperformance.strategy)}</span>
                        </div>
                        <div class="tt-brow">
                          <span class="tt-blab">Buy and hold</span>
                          <span class="tt-bbar"><span class="tt-bfill hold" class:down={outperformance.hold < 0} style="width:{(Math.abs(outperformance.hold) / benchScale) * 100}%"></span></span>
                          <span class="tt-bval">{money(outperformance.hold)}</span>
                        </div>
                      </div>
                      <p class="tt-plotnote">
                        Three of the four plots above need a series the sweep never wrote. <b>Buy and hold is the one that is
                        computable</b> — from the bars on disk — so it is the one drawn.
                      </p>
                    {:else}
                      <div class="tt-empty"><Lock /> {bench.why || 'No plot on this list can be drawn from what is recorded.'}</div>
                    {/if}
                  </div>
                </div>
              </div>

              <!-- ================= PERFORMANCE ANALYSIS ================= -->
              <div class="tt-sec">
                <h4 class="tt-h">Performance analysis</h4>
                <div class="tt-pills" role="group" aria-label="Performance analysis">
                  {#each [['breakdown', 'Breakdown'], ['periodical', 'Periodical'], ['benchmarking', 'Benchmarking'], ['margin', 'Margin usage'], ['growth', 'Growth and decline']] as [key, label] (key)}
                    <button class="tt-pill" class:on={paTab === key} onclick={() => (paTab = key)}>{label}</button>
                  {/each}
                </div>

              {#key paTab}
                <div class="bt-pane">
                  {#if paTab === 'breakdown'}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Gross profit</span><span class="tt-qv"><Lock why="Only the net total is recorded." /></span></div>
                    <div class="tt-q"><span class="tt-k">Gross loss</span><span class="tt-qv"><Lock why="Only the net total is recorded." /></span></div>
                    <div class="tt-q"><span class="tt-k">Profit factor</span><span class="tt-qv"><Lock why="Needs gross profit and gross loss." /></span></div>
                    <div class="tt-q"><span class="tt-k">Commission load</span><span class="tt-qv"><Lock why="crates/costs applies costs inside the sweep; the record keeps the net, not the fee line." /></span></div>
                  </div>

                  <div class="tt-hrow tight">
                    <h5 class="tt-h5">Profits and losses</h5>
                    <div class="tt-seg" role="group" aria-label="Split">
                      <button class="tt-segbtn" class:on={plSplit === 'signals'} onclick={() => (plSplit = 'signals')}>By signals</button>
                      <button class="tt-segbtn" class:on={plSplit === 'side'} onclick={() => (plSplit = 'side')}>By side</button>
                    </div>
                  </div>
                  <div class="tt-lockpanel">
                    <Lock />
                    <span>
                      {#if plSplit === 'signals'}
                        Splitting profit and loss <b>by signal</b> needs each trade tagged with the entry that opened it. The
                        sweep records one total for the whole run.
                      {:else}
                        Splitting <b>by side</b> needs a long/short tag per trade. Direction is one of the nine terms inside
                        the run's identity hash, not a field beside it.
                      {/if}
                    </span>
                  </div>

                  <h5 class="tt-h5">Fill models <span class="tt-own">brutex</span></h5>
                  <p class="tt-note2">
                    TradingView simulates one fill model. This sweep records the same combination under both ends of every
                    bar and ranks on the worse, so the spread between them is a figure the Strategy Tester has no row for.
                  </p>
                  <div class="tt-pl">
                    <div class="tt-plrow">
                      <span class="tt-pllab">Worst-case (adverse extreme)</span>
                      <span class="tt-plbar"><span class="tt-plfill" style="width:{(Math.abs(openRun.pessimistic) / fillScale) * 100}%"></span></span>
                      <span class="tt-plval strong">{money(openRun.pessimistic)}</span>
                    </div>
                    <div class="tt-plrow">
                      <span class="tt-pllab">Best-case (open fills)</span>
                      <span class="tt-plbar"><span class="tt-plfill ghost" style="width:{(Math.abs(openRun.optimistic) / fillScale) * 100}%"></span></span>
                      <span class="tt-plval">{money(openRun.optimistic)}</span>
                    </div>
                    <div class="tt-plrow">
                      <span class="tt-pllab">Spread — how much is fill assumption</span>
                      <span class="tt-plbar"><span class="tt-plfill warnbar" style="width:{(Math.abs(openRun.optimistic - openRun.pessimistic) / fillScale) * 100}%"></span></span>
                      <span class="tt-plval warnt">{money(openRun.optimistic - openRun.pessimistic)}</span>
                    </div>
                  </div>
                {:else if paTab === 'periodical'}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Annualized return (CAGR)</span><span class="tt-qv" class:up={(cagrBps ?? 0) >= 0} class:down={(cagrBps ?? 0) < 0}>{cagrBps === null ? '—' : pct(cagrBps)}</span></div>
                    <div class="tt-q"><span class="tt-k">Total return</span><span class="tt-qv" class:up={(strategyBps ?? 0) >= 0} class:down={(strategyBps ?? 0) < 0}>{pct(strategyBps)}</span></div>
                    <div class="tt-q"><span class="tt-k">Sharpe ratio</span><span class="tt-qv"><Lock why="crates/runner computes significance, but no ratio reaches the record." /></span></div>
                    <div class="tt-q"><span class="tt-k">Sortino ratio</span><span class="tt-qv"><Lock why="Same — not written to the ledger." /></span></div>
                  </div>

                  <div class="tt-hrow tight">
                    <h5 class="tt-h5">{PERIOD_LABEL[periodScale]} PnL</h5>
                    <div class="tt-seg" role="group" aria-label="Period">
                      {#each ['daily', 'weekly', 'quarterly', 'yearly'] as s (s)}
                        <button class="tt-segbtn" class:on={periodScale === s} onclick={() => (periodScale = s)}>{s[0].toUpperCase() + s.slice(1)}</button>
                      {/each}
                    </div>
                  </div>
                  {#if holdPeriods.length > 1}
                    {@render barChart(
                      holdPeriods,
                      [money(barScale(holdPeriods)), money(Math.round(barScale(holdPeriods) / 2)), "0", money(-Math.round(barScale(holdPeriods) / 2)), money(-barScale(holdPeriods))],
                      "Per-period P&L of HOLDING one unit, from the bars on disk. The strategy has no per-period series — it records one total for the whole span — so this is the benchmark, and it is the only one of the two that can be bucketed.",
                      ["Benchmark gain", "Benchmark loss"]
                    )}
                  {:else}
                    {@render chartFrame(
                      'No bars are loaded, so there is nothing to bucket by ' + periodScale.replace('ly', '') + '.',
                      ['Realized profit', 'Realized loss', 'Favorable excursion', 'Adverse excursion'],
                      PNL_TICKS,
                      periodTicks,
                      periodScale === 'daily'
                    )}
                  {/if}
                  <p class="tt-note2">
                    Return is on <b>one unit of the index</b>, against the price at the span's start ({money(bench.open)}).
                    The ledger records no capital, so a return on equity has no denominator on disk. Annualised over the
                    {exact(openRun.months_found)} months actually found.
                  </p>
                {:else if paTab === 'benchmarking'}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Strategy return</span><span class="tt-qv" class:up={(strategyBps ?? 0) >= 0} class:down={(strategyBps ?? 0) < 0}>{pct(strategyBps)}</span></div>
                    <div class="tt-q"><span class="tt-k">Buy and hold return</span><span class="tt-qv" class:up={(buyHold?.bps ?? 0) >= 0} class:down={(buyHold?.bps ?? 0) < 0}>{buyHold ? pct(buyHold.bps) : '—'}</span></div>
                    <div class="tt-q">
                      <span class="tt-k">Strategy outperformance</span>
                      <span class="tt-qv" class:up={outperformance?.beat} class:down={outperformance && !outperformance.beat}>
                        {outperformance && buyHold && strategyBps !== null ? pct(strategyBps - buyHold.bps) : '—'}
                      </span>
                    </div>
                    <div class="tt-q"><span class="tt-k">Correlation</span><span class="tt-qv"><Lock why="Needs a strategy return series to correlate against the benchmark's." /></span></div>
                  </div>
                  {#if outperformance}
                    <p class="tt-note2" class:badnote={!outperformance.beat}>
                      {#if outperformance.beat}
                        The sweep <b>beats buy and hold by {money(outperformance.edge)}</b> under worst-case fills.
                      {:else}
                        <b>The sweep falls short of buy and hold by {money(-outperformance.edge)}</b> under worst-case fills —
                        {exact(openRun.trades)} trades across {(spanYears ?? 0).toFixed(1)} years to end up behind holding the index.
                      {/if}
                    </p>
                  {/if}
                  <div class="tt-hrow tight">
                    <h5 class="tt-h5">Strategy vs benchmark</h5>
                    <div class="tt-seg" role="group" aria-label="Period">
                      {#each ['daily', 'weekly', 'quarterly', 'yearly'] as s (s)}
                        <button class="tt-segbtn" class:on={periodScale === s} onclick={() => (periodScale = s)}>{s[0].toUpperCase() + s.slice(1)}</button>
                      {/each}
                    </div>
                  </div>
                  {#if holdCurve.length > 1}
                    {@render areaChart(
                      holdCurve,
                      [money(curveScale(holdCurve)), money(Math.round(curveScale(holdCurve) / 2)), "0", money(-Math.round(curveScale(holdCurve) / 2)), money(-curveScale(holdCurve))],
                      periodTicks,
                      "Cumulative P&L of HOLDING one unit across the window on screen, bar by bar. The strategy line TradingView draws beside this one needs an equity series the sweep never wrote — its whole-span total is the bar under Performance above."
                    )}
                  {:else}
                    {@render chartFrame(
                      'No bars are loaded, so the benchmark curve has nothing to draw.',
                      ['Strategy PnL', 'Buy and hold PnL'],
                      PNL_TICKS,
                      periodTicks,
                      false
                    )}
                  {/if}
                {:else if paTab === 'margin'}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Margin efficiency</span><span class="tt-qv"><Lock why="The engine models no account." /></span></div>
                    <div class="tt-q"><span class="tt-k">Average margin used</span><span class="tt-qv"><Lock why="The engine models no account." /></span></div>
                    <div class="tt-q"><span class="tt-k">Margin calls</span><span class="tt-qv"><Lock why="The engine models no account." /></span></div>
                    <div class="tt-q"><span class="tt-k">Total liquidated volume</span><span class="tt-qv"><Lock why="The engine models no account." /></span></div>
                  </div>
                  <h5 class="tt-h5">Margin utilization</h5>
                  {@render chartFrame(
                    'Every row on this tab is locked, and that is a DESIGN FACT rather than a gap to fill. The engine computes totals in index points with no capital, no position size and no broker. There is no margin to use, so there is nothing here to record.',
                    [],
                    PCT_TICKS,
                    periodTicks,
                    false
                  )}
                {:else}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Average run-up duration</span><span class="tt-qv"><Lock why="Needs a run-up series over time." /></span></div>
                    <div class="tt-q"><span class="tt-k">Average drawdown duration</span><span class="tt-qv"><Lock why="Needs a drawdown series over time." /></span></div>
                    <div class="tt-q"><span class="tt-k">Max drawdown</span><span class="tt-qv down">{money(Math.abs(openRun.max_drawdown))}<em class="tt-pc">{drawdownBps === null ? '' : `${(drawdownBps / 100).toFixed(2)}%`}</em></span></div>
                    <div class="tt-q"><span class="tt-k">Max drawdown as % of opening price</span><span class="tt-qv">{drawdownBps === null ? '—' : `${(drawdownBps / 100).toFixed(2)}%`}</span></div>
                  </div>
                  <h5 class="tt-h5">Alternating growth and decline</h5>
                  {#if holdSwings.segs.length > 0}
                    {@render swingChart(
                      holdSwings,
                      "Alternating run-up and drawdown of the BENCHMARK — one unit held — segmented from the cumulative curve on disk. The strategy has no equity series to segment, so its single recorded drawdown is the figure in the quad above."
                    )}
                  {:else}
                    {@render chartFrame(
                      'No bars are loaded, so there is no curve to segment into run-ups and drawdowns.',
                      ['Run-up', 'Drawdown', 'Current run-up'],
                      PCT_TICKS,
                      periodTicks,
                      false
                    )}
                  {/if}
                  <!-- TradingView's second block on this tab: run-up and
                       drawdown, each as maximum / average / current, on one
                       shared scale. The maximum drawdown is the one figure
                       recorded, so it is the one bar drawn. -->
                  <h5 class="tt-h5">Comparison of growth and decline periods</h5>
                  <div class="tt-cmp">
                    <span class="tt-cmpgrp">Run-up</span>
                    {#each ['Maximum', 'Average', 'Current'] as k (k)}
                      <div class="tt-cmprow">
                        <span class="tt-cmplab">{k}</span>
                        <span class="tt-cmpbar"></span>
                        <span class="tt-cmpval"><Lock small why="Run-up needs an equity series to measure a rise across." /></span>
                      </div>
                    {/each}
                    <span class="tt-cmpgrp">Drawdown</span>
                    <div class="tt-cmprow">
                      <span class="tt-cmplab">Maximum</span>
                      <span class="tt-cmpbar"><span class="tt-cmpfill down" style="width:100%"></span></span>
                      <span class="tt-cmpval down">{drawdownBps === null ? money(Math.abs(openRun.max_drawdown)) : `${(drawdownBps / 100).toFixed(2)}%`}</span>
                    </div>
                    <div class="tt-cmprow">
                      <span class="tt-cmplab">Average</span>
                      <span class="tt-cmpbar"></span>
                      <span class="tt-cmpval"><Lock small why="Only the single worst drawdown is recorded, not every one to average." /></span>
                    </div>
                  </div>

                  <h5 class="tt-h5">Excursion <span class="tt-own">brutex</span></h5>
                  <p class="tt-note2">
                    How far trades went against the position before resolving, in parts per million. The winners' adverse
                    excursion is <b>the tightest stop that would not have killed a winner</b> — a figure TradingView reports
                    per trade and this sweep folds into three aggregates.
                  </p>
                  <div class="tt-pl">
                    <!-- SAY WHAT THE NUMBER IS, NOT WHAT THE TEXTBOOK CALLS IT.
                         These read "Winners' adverse (MAE)" and "Winners'
                         favourable (MFE)" -- two acronyms an operator has to
                         already know to get anything from the row. The plain
                         sentence is what the field measures; the acronym and
                         the stored parts-per-million both survive in `title`
                         for the reader who wants them. -->
                    {#each [{ k: 'How far a winner fell before it paid', a: 'MAE — maximum adverse excursion, winners only', v: openRun.winner_mae, t: 'warnbar' }, { k: 'How far a winner rose at its best', a: 'MFE — maximum favourable excursion, winners only', v: openRun.winner_mfe, t: '' }, { k: 'How far any trade fell, winners and losers', a: 'MAE — maximum adverse excursion, every trade', v: openRun.all_mae, t: 'downbar' }] as e (e.k)}
                      <div class="tt-plrow">
                        <span class="tt-pllab" title="{e.a}. Stored as {group(e.v)} parts per million of the entry price."
                          >{e.k}</span
                        >
                        <span class="tt-plbar"><span class="tt-plfill {e.t}" style="width:{(Math.abs(e.v) / excursionTop) * 100}%"></span></span>
                        <span class="tt-plval" title="{group(e.v)} ppm">{ppmPct(e.v)}</span>
                      </div>
                    {/each}
                  </div>
                  {/if}
                </div>
              {/key}
              </div>

              <!-- ================= TRADES ANALYSIS ================= -->
              <div class="tt-sec">
                <h4 class="tt-h">Trades analysis</h4>
                <div class="tt-pills" role="group" aria-label="Trades analysis">
                  {#each [['distribution', 'Distribution'], ['streaks', 'Streaks'], ['details', 'Trades analysis details']] as [key, label] (key)}
                    <button class="tt-pill" class:on={taTab === key} onclick={() => (taTab = key)}>{label}</button>
                  {/each}
                </div>

              <!-- KEYED, SO SWITCHING A TAB IS A MOVE RATHER THAN A SWAP.
                   Without the key the block re-renders in place and the
                   content simply becomes different content, which reads as
                   a glitch. Keyed, the old pane leaves and the new one
                   arrives, and the eye follows it. -->
              {#key taTab}
                <div class="bt-pane">
                  {#if taTab === 'distribution'}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Expected payoff</span><span class="tt-qv">{perTrade ? money(perTrade.worst) : '—'}</span></div>
                    <div class="tt-q"><span class="tt-k">Outliers PnL</span><span class="tt-qv"><Lock why="Needs a per-trade list to find outliers in." /></span></div>
                    <div class="tt-q"><span class="tt-k">Largest profit</span><span class="tt-qv"><Lock why="Only the worst single trade is kept." /></span></div>
                    <div class="tt-q"><span class="tt-k">Largest loss</span><span class="tt-qv down">{money(openRun.worst_trade)}</span></div>
                  </div>
                  <div class="tt-two">
                    <div>
                      <h5 class="tt-h5">Returns distribution</h5>
                      {#if returnHistogram.bins.length > 0}{@render histogram(returnHistogram, 'The distribution of per-BAR returns over the window on screen, from the bars on disk. NOT per trade — the sweep records ' + exact(openRun.trades) + ' as a count and keeps no list, so a per-trade histogram has no population to draw.')}{:else}{@render chartFrame('No bars are loaded, so there is nothing to distribute.', HIST_LEGEND, COUNT_TICKS, RETURN_TICKS, false)}{/if}
                    </div>
                    <div>
                      <h5 class="tt-h5">Trades distribution</h5>
                      <!-- THE DONUT, DRAWN AS A RING WITH NO SPLIT. The total is
                           real and sits in the middle where TradingView puts it;
                           the arc is not divided because the winner/loser split
                           is exactly what is not recorded. A guessed split here
                           would be the most convincing wrong picture on the page. -->
                      <div class="tt-donutwrap">
                        <svg class="tt-donut" viewBox="0 0 120 120" role="img" aria-label="{exact(openRun.trades)} total trades. The winner and loser split is not recorded.">
                          <circle cx="60" cy="60" r="44" fill="none" stroke="var(--n5)" stroke-width="16" />
                          <circle cx="60" cy="60" r="44" fill="none" stroke="var(--n7)" stroke-width="16" stroke-dasharray="4 6" opacity="0.7" />
                        </svg>
                        <div class="tt-donutmid">
                          <b>{exact(openRun.trades)}</b>
                          <span>Total trades</span>
                        </div>
                        <!-- THREE COLUMNS, as TradingView sets it: name,
                             trade count, share of the total. -->
                        <ul class="tt-donutleg">
                          <li>
                            <span class="sw up"></span><span class="nm">Winners</span>
                            <span class="ct"><Lock small why="No win count is recorded." /></span>
                            <span class="pc"><Lock small why="Needs the win count above." /></span>
                          </li>
                          <li>
                            <span class="sw down"></span><span class="nm">Losers</span>
                            <span class="ct"><Lock small why="No loss count is recorded." /></span>
                            <span class="pc"><Lock small why="Needs the loss count above." /></span>
                          </li>
                          <li>
                            <span class="sw flat"></span><span class="nm">Breakevens</span>
                            <span class="ct"><Lock small why="No breakeven count is recorded." /></span>
                            <span class="pc"><Lock small why="Needs the breakeven count above." /></span>
                          </li>
                        </ul>
                      </div>
                      <p class="tt-note2">
                        The ring is <b>undivided on purpose</b>. The total is real; the split is the thing that is not
                        recorded, and a donut cut on a guess would look exactly like one cut on data.
                      </p>
                    </div>
                  </div>
                {:else if taTab === 'streaks'}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Longest winning streak</span><span class="tt-qv"><Lock why="Needs the ordered win/loss outcome of every trade." /></span></div>
                    <div class="tt-q"><span class="tt-k">Longest losing streak</span><span class="tt-qv"><Lock why="Needs the ordered win/loss outcome of every trade." /></span></div>
                    <div class="tt-q"><span class="tt-k">Average winning streak</span><span class="tt-qv"><Lock why="Needs the ordered win/loss outcome of every trade." /></span></div>
                    <div class="tt-q"><span class="tt-k">Average losing streak</span><span class="tt-qv"><Lock why="Needs the ordered win/loss outcome of every trade." /></span></div>
                  </div>
                  <div class="tt-hrow tight">
                    <h5 class="tt-h5">Winning and losing streaks</h5>
                    <div class="tt-seg" role="group" aria-label="Streak unit">
                      <button class="tt-segbtn" class:on={streakMode === 'count'} onclick={() => (streakMode = 'count')}>Count</button>
                      <button class="tt-segbtn" class:on={streakMode === 'amount'} onclick={() => (streakMode = 'amount')}>Amount</button>
                    </div>
                  </div>
                  {@render chartFrame(
                    'Every figure on this tab is a property of the ORDER trades resolved in. The ledger keeps a count and a net, both order-independent, so nothing here is recoverable from it.',
                    [],
                    STREAK_TICKS,
                    [],
                    false
                  )}
                {:else}
                  <div class="tt-tblwrap">
                    <table class="tt-tbl">
                      <thead>
                        <tr><th>Metric</th><th class="n">All</th><th class="n">Long</th><th class="n">Short</th></tr>
                      </thead>
                      <tbody>
                        <tr><td>Total trades</td><td class="n">{exact(openRun.trades)}</td><td class="n"><Lock small why="Direction is one of the nine terms inside the run's identity hash, not a field beside it." /></td><td class="n"><Lock small why="Direction is one of the nine terms inside the run's identity hash, not a field beside it." /></td></tr>
                        <tr><td>Total open trades</td><td class="n"><Lock small why="The sweep closes every position at the span's end; open positions are not recorded." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Total winners</td><td class="n"><Lock small why="No win count is recorded." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Total losers</td><td class="n"><Lock small why="No loss count is recorded." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Percent profitable</td><td class="n"><Lock small why="Cannot be inferred from a net total." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average PnL</td><td class="n"><span class="tv">{perTrade ? money(perTrade.worst) : "—"}</span><span class="tp">{perTrade && bench.open > 0 ? `${((perTrade.worst / bench.open) * 100).toFixed(2)}%` : ""}</span></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average profit</td><td class="n"><Lock small why="Needs gross profit and a winner count." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average loss</td><td class="n"><Lock small why="Needs gross loss and a loser count." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average profit / average loss</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Largest profit</td><td class="n"><Lock small why="Only the worst single trade is kept." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Largest profit %</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Largest profit as % of gross profit</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Largest loss</td><td class="n down"><span class="tv">{money(openRun.worst_trade)}</span><span class="tp">{bench.open > 0 ? `${((openRun.worst_trade / bench.open) * 100).toFixed(2)}%` : ""}</span></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Largest loss %</td><td class="n"><Lock small why="Needs the entry price of that trade." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Largest loss as % of gross loss</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Outliers</td><td class="n"><Lock small why="Needs a per-trade list." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Outliers P&amp;L</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average bars in trades</td><td class="n"><Lock small why="bars ÷ trades is the average gap BETWEEN trades, a different quantity. Not shown rather than shown wrong." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average bars in winners</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average bars in losers</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr class="own"><td title="MAE — maximum adverse excursion, winners only">How far a winner fell before it paid <span class="tt-own">brutex</span></td><td class="n">{ppmPct(openRun.winner_mae)}</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr class="own"><td title="MFE — maximum favourable excursion, winners only">How far a winner rose at its best <span class="tt-own">brutex</span></td><td class="n">{ppmPct(openRun.winner_mfe)}</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr class="own"><td title="MAE — maximum adverse excursion, every trade">How far any trade fell <span class="tt-own">brutex</span></td><td class="n">{ppmPct(openRun.all_mae)}</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr class="own"><td>Signal bars swept <span class="tt-own">brutex</span></td><td class="n">{exact(openRun.bars)}</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                      </tbody>
                    </table>
                  </div>
                  {/if}
                </div>
              {/key}
              </div>
            {:else if testerView === 'trades'}
              <!-- ================= LIST OF TRADES ================= -->
              <div class="tt-sec">
                <div class="tt-hrow">
                  <h4 class="tt-h">List of trades</h4>
                  <div class="tt-icons">
                    <button class="tt-iconbtn" title="Download" aria-label="Download"><svg viewBox="0 0 16 16"><path d="M8 2v8m0 0L5 7m3 3l3-3M3 13h10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" /></svg></button>
                    <button class="tt-iconbtn" title="Columns" aria-label="Columns"><svg viewBox="0 0 16 16"><rect x="2" y="3" width="3.2" height="10" rx="1" fill="none" stroke="currentColor" stroke-width="1.3" /><rect x="6.4" y="3" width="3.2" height="10" rx="1" fill="none" stroke="currentColor" stroke-width="1.3" /><rect x="10.8" y="3" width="3.2" height="10" rx="1" fill="none" stroke="currentColor" stroke-width="1.3" /></svg></button>
                  </div>
                </div>
                <div class="tt-tblwrap">
                  <table class="tt-tbl ghosted">
                    <thead>
                      <tr>
                        <th>Trade number</th><th>Type</th><th>Date and time</th><th>Signal</th><th class="n">Price</th>
                        <th class="n">Size</th><th class="n">Net PnL</th><th class="n">Return</th>
                        <th class="n">Favorable excursion</th><th class="n">Adverse excursion</th>
                        <th class="n">Cumulative PnL</th><th class="n">Duration (bars)</th>
                      </tr>
                    </thead>
                    <tbody>
                      <!-- ONE TRADE IS TWO ROWS: exit above, entry below, in a
                           bordered block, with the per-TRADE columns spanning
                           both and the per-LEG columns differing. That shape is
                           the whole reason this table reads as trades rather
                           than as a log, so it is drawn even though every leg
                           is a padlock. The columns to the right of Price
                           belong to the trade and are set with rowspan. -->
                      <tr class="lot-a">
                        <td rowspan="2" class="lot-num"><Lock small why="No trade number — no trade list is recorded." /></td>
                        <td>Exit</td>
                        <td><Lock small why="No exit timestamp is recorded." /></td>
                        <td><Lock small why="No exit signal is recorded." /></td>
                        <td class="n"><Lock small why="No exit price is recorded." /></td>
                        <td rowspan="2" class="n"><Lock small why="Position size is not modelled — the engine computes index points, not contracts." /></td>
                        <td rowspan="2" class="n"><Lock small why="Per-trade PnL is not recorded, only the run's net." /></td>
                        <td rowspan="2" class="n"><Lock small why="Needs per-trade PnL and its entry price." /></td>
                        <td rowspan="2" class="n"><Lock small why="Favourable excursion is recorded as a fold over winners, not per trade." /></td>
                        <td rowspan="2" class="n"><Lock small why="Adverse excursion is recorded as a fold, not per trade." /></td>
                        <td rowspan="2" class="n"><Lock small why="A cumulative series needs every trade in order." /></td>
                        <td rowspan="2" class="n"><Lock small why="Needs the entry and exit bar of each trade." /></td>
                      </tr>
                      <tr class="lot-b">
                        <td>Entry</td>
                        <td><Lock small why="No entry timestamp is recorded." /></td>
                        <td><Lock small why="No entry signal is recorded." /></td>
                        <td class="n"><Lock small why="No entry price is recorded." /></td>
                      </tr>
                      <tr class="lockrow">
                        <td colspan="12">
                          <div class="tt-lockbig">
                            <Lock big />
                            <div>
                              <b>No trade list is recorded, so no row here can be filled.</b>
                              <p>
                                The sweep took <b>{exact(openRun.trades)}</b> round trips and wrote that number and nothing
                                else about them — no entry, no exit, no timestamp, no per-trade result. Twelve columns of
                                plausible rows could be generated from the totals; none of them would be a trade that
                                happened, and that is the one thing this console must never print.
                              </p>
                              <p class="tt-fix">
                                <b>What closes it:</b> a second append-only file beside <code>runs.bin</code>, at a fixed
                                stride, one record per trade. That is <code>cli</code>'s file to write — the same change that
                                unlocks percent profitable, profit factor, the streaks, the distribution and the equity curve.
                              </p>
                            </div>
                          </div>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            {:else}
              <!-- ================= PROPERTIES ================= -->
              <div class="tt-sec">
                <h4 class="tt-h">Properties</h4>
                <p class="tt-note2">
                  What the sweep was actually asked to do. TradingView shows the author's inputs here; this shows the run's
                  own terms, which are the nine that make up its identity.
                </p>
                <div class="tt-tblwrap">
                  <table class="tt-tbl">
                    <tbody>
                      <tr><td>Feed</td><td class="n">{openRun.feed}</td></tr>
                      <tr><td>Instrument</td><td class="n">{openRun.underlying}</td></tr>
                      <tr><td>Signal rung</td><td class="n">{openRun.timeframe}</td></tr>
                      <tr><td>Execution rung</td><td class="n">1min — always</td></tr>
                      <tr><td>Span asked</td><td class="n">{span(openRun)} · {exact(openRun.months_asked)} months</td></tr>
                      <tr class:warnrow={!openRun.whole_span}><td>Span found</td><td class="n">{exact(openRun.months_found)} months{openRun.whole_span ? '' : ' — a SHORTER sample, not a corrected one'}</td></tr>
                      <tr><td>Signal bars swept</td><td class="n">{exact(openRun.bars)}</td></tr>
                      <tr><td title="The support threshold. A combination had to hit at least this many bars to survive the ladder.">How often a pattern had to appear</td><td class="n">{exact(openRun.min_hits)} times · {(supportPerMille(openRun) / 10).toFixed(1)}% of bars</td></tr>
                      <tr><td>Combinations enumerated</td><td class="n">{exact(openRun.combinations)}</td></tr>
                      <tr class:warnrow={openRun.halted}><td>Ladder depth</td><td class="n">{openRun.depth}{openRun.halted ? ' — PARTIAL, a budget stopped the walk' : ' — ran to extinction'}</td></tr>
                      <tr><td>Recorded</td><td class="n">{when(openRun.finished_micros)}</td></tr>
                    </tbody>
                  </table>
                </div>

                <h5 class="tt-h5">Exit geometry</h5>
                <div class="tt-tblwrap">
                  <table class="tt-tbl">
                    <tbody>
                      {#each openRun.exit_rungs ?? [] as rung, i (i)}
                        <tr class:offrow={rung < 0}>
                          <td>{EXIT_AXES[i] ?? `axis ${i} — not named by this build`}</td>
                          <td class="n">{rung < 0 ? 'no rung — not used' : `rung ${rung}`}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>

                <h5 class="tt-h5">The ladder <span class="tt-own">not recorded</span></h5>
                <p class="tt-note2">
                  This run walked to depth <b>{openRun.depth}</b> and produced <b>{exact(openRun.combinations)}</b>
                  combinations. <b>Where the frequent frontier emptied cannot be shown</b> — the ledger stores those two
                  numbers as scalars and not the per-level survivor counts.
                </p>

                {#if load.body?.has_mask === undefined}
                  <!-- THE BINARY IS OLDER THAN THIS PAGE, WHICH IS A THIRD
                       STATE AND NOT THE SAME AS A LEDGER WITHOUT A MASK.
                       This page comes off disk and the route does not, so a
                       rebuilt front end reaches the operator instantly while
                       the server that answers it does not — the trap
                       `fetchLedger`'s 404 arm was written for. Reporting a
                       missing FIELD as an absent MASK would blame the data for
                       a stale process. -->
                  <h5 class="tt-h5">The winning combination <span class="tt-own">server too old</span></h5>
                  <p class="tt-note2">
                    <b>The running server does not send <code>has_mask</code>.</b> This page can read the winning
                    combination, but the binary answering it predates the field — so whether this ledger holds a mask
                    cannot be told from what arrived. Rebuild and restart the server; nothing is wrong with the data.
                  </p>
                {:else if !load.body.has_mask}
                  <!-- A LEDGER THAT PREDATES THE MASK, AND SAYING SO IS THE
                       WHOLE POINT OF `has_mask`. A version-2 record carries six
                       zero words because version 2 had no mask; a version-3 run
                       may carry six zero words because it found no combination.
                       The bytes are identical. Rendering both as "no conditions"
                       is the fallback CLAUDE.md §4 bans. -->
                  <h5 class="tt-h5">The winning combination <span class="tt-own">not in this file</span></h5>
                  <p class="tt-note2">
                    <b>This ledger predates the condition mask.</b> It is format version
                    <b>{load.body?.version ?? '?'}</b>, which stored the run's identity — a blake3 over the mask and
                    eight other terms — and not the mask itself. This is <b>not</b> a run that found no conditions:
                    the field did not exist when it was recorded. A run swept by this build records it.
                  </p>
                {:else}
                  {@const positions = positionsIn(openRun.mask_words)}
                  <h5 class="tt-h5">
                    The winning combination
                    <span class="tt-own">{positions.length} condition{positions.length === 1 ? '' : 's'}</span>
                  </h5>
                  {#if positions.length === 0}
                    <p class="tt-note2">
                      <b>This run recorded no combination.</b> The mask is empty and the ledger is new enough to mean
                      it — the frequent frontier emptied before any combination survived, so there is nothing to name.
                    </p>
                  {:else}
                    {#if vocab.phase === 'failed'}
                      <p class="tt-note2"><b>Shown as raw positions.</b> {vocab.why}</p>
                    {/if}
                    <div class="tt-tblwrap">
                      <table class="tt-tbl">
                        <tbody>
                          {#each positions as position (position)}
                            {@const bit = vocab.bits.get(position)}
                            <tr class:offrow={bit ? !bit.live : false}>
                              <td>{bit ? bit.name : `position ${position} — not named by this vocabulary`}</td>
                              <td class="n">
                                {#if bit && !bit.live}
                                  bit {position} — RETIRED, kept because the mask carries it
                                {:else}
                                  bit {position}
                                {/if}
                              </td>
                            </tr>
                          {/each}
                        </tbody>
                      </table>
                    </div>
                    <p class="tt-note2">
                      Every bar this run counted as a hit satisfied <b>all {positions.length}</b> of these at once — a
                      mask hits a bar iff <code>(bits &amp; mask) == mask</code>, so adding a condition can only remove
                      hits, never add them.
                    </p>
                  {/if}
                {/if}
                <p class="tt-ident"><span class="tt-k">identity</span><code>{openRun.identity}</code></p>
              </div>
            {/if}
          </section>
        </section>
      {/if}
    {/if}
  {/if}
  </div>
</div>

<style>
  /* ==================================================================
     Tokens come from `$lib/theme.css`. Nothing here declares a colour
     literal, so both themes follow the console's own palette.
     ================================================================== */
  .page {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding-bottom: 4rem;
  }
  /* NO CHILD SHRINKS BELOW ITS CONTENT, and this is a fix for a measured bug,
     not a precaution. A flex item defaults to `flex-shrink: 1`; the layout's
     own `<main>` constrains this column's height, so the summary strip
     computed to 34px against 93.8px of content and `overflow: hidden` — which
     it needs for its rounded corners — silently clipped the four facts to
     four empty boxes. A page whose facts are present in the DOM and invisible
     on screen is the quietest possible version of the failure §4 bans. */
  .page > * {
    flex: 0 0 auto;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
  }
  h1 {
    margin: 0;
    font-size: 1.5rem;
    letter-spacing: -0.01em;
    color: var(--n12);
  }
  .sub {
    margin: 0.35rem 0 0;
    color: var(--n9);
    font-size: 0.875rem;
    max-width: 62ch;
  }
  .sub b {
    color: var(--n11);
  }

  /* ==================================================================
     ONE PANEL, NOT A STACK OF CARDS
     ------------------------------------------------------------------
     This page was a column of rounded cards with 1.5rem between them,
     and it read as a DASHBOARD OF WIDGETS. TradingView's tester is one
     continuous instrument: sections separated by a hairline, nothing
     floating, no gap for the page's ground to show through. The
     difference is not decoration — a gap says "these are separate
     things", and every section here is a different view of ONE run.

     So the outer container owns the border, the radius and the shadow,
     and every section inside owns only a bottom hairline. Density comes
     from the same change: the space the gaps were using goes back to
     the content.
     ================================================================== */
  .page {
    display: block;
    gap: 0;
    padding-bottom: 3rem;
    /* THIS PAGE OWNS ITS SCROLL, BECAUSE THE SHELL DELIBERATELY DOES NOT.
       `theme.css`'s `.main` is `overflow: hidden` by design — its comment
       says a page whose root is `.split` or `.pane` "keeps the exact height
       it had" — so every page is expected to provide its own scroller. This
       one's root is neither, and it provided none.

       Measured at 1440x900 before this line: `.main` had clientHeight 854
       against scrollHeight 1102. **248px was clipped with no way to reach
       it** — `overflow: hidden` scrolls under `scrollTop` from script but
       gives a person no wheel, no scrollbar and no keyboard. What was past
       the cut was the whole of "The ledger": every run this console has
       recorded, present in the DOM and unreachable.

       That is the same failure the `flex-shrink` comment above this block
       records — "facts present in the DOM and invisible on screen" — caught
       there for the summary strip and left standing for the section that
       lists the runs. The strip was patched; the cause was not.

       `min-height: 0` is load-bearing beside it: without it a flex child
       floors at its content height, the box never becomes smaller than what
       it holds, and `auto` has nothing to scroll. */
    min-height: 0;
    overflow-y: auto;
  }
  /* NO `overflow: hidden`, AND THE RADIUS IS KEPT ANOTHER WAY.

     The shell clipped to its own rounded corners, which is the obvious way
     to get them and the reason the span menus were cut off at the panel
     edge: `$lib/Picker.svelte` positions its menu absolutely, and an
     ancestor that clips clips it too. A menu that is trimmed to the box it
     opens out of is unusable past the first few rows.

     The corners come from the first and last sections instead, which is
     where the rounding is actually visible -- the sections between them are
     square either way. */
  .bt-shell {
    border: 1px solid var(--n6);
    border-radius: 10px;
    background: var(--n3);
    box-shadow: var(--e1);
  }
  .bt-shell > *:first-child {
    border-top-left-radius: 9px;
    border-top-right-radius: 9px;
  }
  .bt-shell > *:last-child {
    border-bottom-left-radius: 9px;
    border-bottom-right-radius: 9px;
  }
  /* Every direct section of the shell: a hairline below, no border of
     its own, no radius, no shadow, no gap. */
  .bt-shell > * {
    border: 0;
    border-bottom: 1px solid var(--n6);
    border-radius: 0;
    box-shadow: none;
    margin: 0;
  }
  .bt-shell > *:last-child {
    border-bottom: 0;
  }

  /* ---- density: the sections themselves ---------------------------- */
  .runbar {
    padding: 0.6rem 0.9rem;
    border-left: 0;
    background: linear-gradient(120deg, var(--acc-soft) 0%, var(--n3) 34%);
  }
  .bt-strip {
    border-radius: 0;
    gap: 1px;
  }
  .fact {
    padding: 0.7rem 0.9rem;
  }
  .fact .v {
    font-size: 1.3rem;
  }
  .block {
    padding: 0.9rem 0.9rem 1rem;
    gap: 0.55rem;
  }
  .bh {
    font-size: 0.9rem;
  }
  .crown {
    padding: 0.8rem 0.95rem;
    gap: 0.85rem;
  }
  .rgroup {
    padding: 0.75rem 0.85rem;
    gap: 0.5rem;
  }
  .tbl-scroll {
    border: 1px solid var(--n6);
    border-radius: 8px;
    max-height: 420px;
  }
  .drill {
    padding: 0.9rem;
  }
  .drill-grid {
    gap: 0.7rem;
  }
  .card {
    padding: 0.75rem 0.85rem;
    gap: 0.45rem;
  }
  .term,
  .tester {
    border: 1px solid var(--n6);
    border-radius: 8px;
  }
  .term .bt-chart {
    height: 400px;
    flex: 0 0 400px;
  }

  /* The page's own heading loses its standfirst paragraph: TradingView's
     tester has no page header at all, and every line of chrome above the
     data is a line of data that did not fit.

     IT ALSO SHRANK THE TITLE TO 1.15rem, AND THAT IS THE HALF THAT WENT
     WRONG. Dropping the standfirst buys space; dropping the title's RANK
     buys nothing and costs the page its top note. Measured before this
     change, the four summary counters rendered at 24px and the title at
     18.4px, so the largest text on a page about one number was a row of
     zeroes. `--fs-lg` puts the title back above the counters, which now
     sit at the data scale where they belong. */
  .head {
    padding-bottom: 0.15rem;
    margin-bottom: 0.7rem;
  }
  h1 {
    font-size: var(--fs-lg);
    font-weight: var(--w-bold);
    letter-spacing: -0.02em;
  }

  /* ---------------- panels ---------------- */
  .panel {
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: 10px;
    padding: 1.1rem 1.25rem;
    box-shadow: var(--e1);
  }
  .panel h2 {
    margin: 0 0 0.4rem;
    font-size: 1rem;
    color: var(--n12);
  }
  .panel p {
    margin: 0 0 0.6rem;
    color: var(--n9);
    font-size: 0.875rem;
    max-width: 72ch;
  }
  .panel p:last-child {
    margin-bottom: 0;
  }
  .panel.err {
    border-left: 3px solid var(--down);
    background: var(--down-soft);
  }
  .panel.bt-note {
    border-left: 3px solid var(--info);
  }
  .wait {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .path code {
    font-size: 0.78rem;
    color: var(--n10);
    word-break: break-all;
  }
  .bt-lab {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--n8);
    margin-right: 0.5rem;
  }

  /* A COMMAND IS COPIED, so it is set as one: monospace, selectable, and
     wide enough that it never wraps mid-flag. */
  .cmd {
    margin: 0 0 0.8rem;
    padding: 0.6rem 0.8rem;
    background: var(--n0);
    border: 1px solid var(--n6);
    border-radius: 6px;
    font-size: 0.79rem;
    color: var(--n11);
    overflow-x: auto;
    user-select: all;
  }

  /* The nine-rung table `cli` prints, and the refusal it prints instead.
     COLUMN-ALIGNED TEXT, so it must not wrap and must not widen the page:
     `range_all` lays its table out in fixed-width columns and a wrap turns
     one row into two that no longer line up under their headings. It scrolls
     inside its own box instead, which is the only way wide content stays
     readable without the body scrolling sideways with it. */
  .runlog {
    margin: 0 0 0.8rem;
    padding: 0.7rem 0.85rem;
    background: var(--n0);
    border: 1px solid var(--n6);
    border-radius: 6px;
    font-size: var(--fs-mini);
    line-height: 1.5;
    color: var(--n11);
    overflow-x: auto;
    white-space: pre;
    max-height: 28rem;
    overflow-y: auto;
  }
  .runlog.bad {
    border-color: var(--down);
    background: var(--down-soft);
    /* A refusal is prose in fixed width, not a table, so it wraps rather than
       running off the edge — the sentence is the payload here. */
    white-space: pre-wrap;
  }

  .spin {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    flex: none;
    border: 2px solid var(--n6);
    border-top-color: var(--acc);
    animation: spin 0.75s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* ---------------- summary strip ---------------- */
  .bt-strip {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 1px;
    background: var(--n6);
    border: 1px solid var(--n6);
    border-radius: 10px;
    overflow: hidden;
  }
  .fact {
    background: var(--n3);
    padding: 0.85rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .fact .k {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--n8);
  }
  .fact .v {
    font-family: var(--num);
    font-size: var(--fs-data-lg);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    color: var(--n12);
    line-height: 1.1;
  }
  .fact .n {
    font-size: var(--fs-micro);
    color: var(--n9);
  }
  .fact.good .v {
    color: var(--up);
  }
  .fact.warn .v {
    color: var(--warn);
  }
  /* A ZERO THAT MEANS "NOTHING WENT WRONG" MUST NOT SHOUT LIKE A COUNT.
     `halted` and `span holes` are DEFECT counters -- the good reading is 0 --
     and a 0 was rendering at full ink in the same size and weight as the run
     count beside it. Four cells of equal loudness is the state this whole
     page was in: if everything is emphasised, nothing is. The quiet case
     recedes here so that the loud case (>0, which takes `.warn` above) is
     the one thing in the strip that catches an eye. */
  .fact.nil .v {
    color: var(--n8);
    font-weight: var(--w-reg);
  }

  .inline-note {
    margin: 0;
    font-size: 0.8rem;
    color: var(--n9);
    padding: 0.6rem 0.8rem;
    border-left: 2px solid var(--n7);
    background: var(--n4);
    border-radius: 0 6px 6px 0;
  }
  .inline-note.warn {
    border-left-color: var(--warn);
    background: var(--warn-soft);
  }
  .inline-note b {
    color: var(--n11);
  }
  .linky {
    background: none;
    border: 0;
    padding: 0;
    color: var(--acc);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }

  /* ---- the Run control ---- */
  .runbar {
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
    flex-wrap: wrap;
    padding: 0.85rem 1rem;
    background: var(--n3);
    border: 1px solid var(--n6);
    border-left: 3px solid var(--acc);
    border-radius: 10px;
  }
  .runbar-k {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--n8);
    padding-bottom: 0.45rem;
  }
  .runbar-n {
    font-size: 0.75rem;
    color: var(--n9);
    padding-bottom: 0.45rem;
  }
  .runbar-n b {
    color: var(--n11);
  }
  /* The one phrase in this bar that is a WARNING rather than a statement. */
  .warnish {
    color: var(--warn);
  }

  /* ---------------- what the store holds ----------------
     A strip, not a card: it is a CONDITION of the sweep above it, so it
     shares the shell's hairline rather than floating away from the form
     it describes. */
  .coverbar {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.85rem 1rem 0.9rem;
  }
  .coverbar-head {
    display: flex;
    align-items: baseline;
    gap: 0.55rem;
    flex-wrap: wrap;
  }
  .coverbar-k {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--n8);
  }
  .coverbar-head b {
    font-size: var(--fs-sm);
    color: var(--n12);
  }
  /* The rung strip. Nine chips read as one measurement when they share a
     baseline and a width; as nine boxes they read as nine things. */
  .rungs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }
  .rungs-head {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  /* ---- the instrument chips ---- */
  .pickset {
    border: 0;
    margin: 0;
    padding: 0;
    min-width: 0;
  }
  .pickset legend {
    padding: 0;
    font-size: var(--fs-micro);
    color: var(--n8);
    text-transform: lowercase;
  }
  .chiprow {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
    margin-top: 0.2rem;
  }
  .pchip {
    display: inline-flex;
    align-items: baseline;
    gap: 0.32rem;
    font-family: var(--mono);
    font-size: var(--fs-mini);
    font-weight: var(--w-mid);
    padding: 5px 9px;
    border-radius: 6px;
    border: 1px solid var(--n6);
    background: var(--n3);
    color: var(--n9);
    cursor: pointer;
    animation: rungin 0.3s cubic-bezier(0.22, 0.7, 0.3, 1) both;
    animation-delay: calc(var(--i, 0) * 40ms);
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease,
      transform 0.15s cubic-bezier(0.22, 0.7, 0.3, 1);
  }
  .pchip:hover {
    border-color: var(--acc);
    transform: translateY(-1px);
  }
  /* SELECTED IS A FILLED CHIP, NOT A TICK. The state has to be readable at a
     glance across a row, and a mark small enough to sit inside a chip is not.
     Colour AND weight AND the border all move together, so the state does not
     rest on hue alone. */
  .pchip.on {
    background: var(--acc-soft);
    border-color: var(--acc);
    color: var(--acc);
    font-weight: var(--w-semi);
  }
  .pchip:focus-visible {
    outline: 2px solid var(--focus, var(--acc));
    outline-offset: 1px;
  }
  .pchip-n {
    font-size: var(--fs-micro);
    font-variant-numeric: tabular-nums;
    opacity: 0.72;
  }
  .rungs li {
    list-style: none;
  }
  .rungchip {
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
    min-width: 5.6rem;
    padding: 0.35rem 0.55rem 0.4rem;
    border: 1px solid var(--n6);
    border-radius: 7px;
    background: var(--n3);
    /* A RUNG ARRIVING IS A FACT ARRIVING. The strip is rebuilt whenever the
       instrument changes -- the key carries the leaf -- so this replays on a
       switch, which is exactly when the numbers under it changed. Staggered
       by index so the strip reads left to right the way it is read. */
    animation: rungin 0.34s cubic-bezier(0.22, 0.7, 0.3, 1) both;
    animation-delay: calc(var(--i, 0) * 34ms);
    transition:
      border-color 0.15s ease,
      transform 0.15s ease;
  }
  .rungchip:hover {
    border-color: var(--acc);
    transform: translateY(-1px);
  }
  /* A CHIP IS A TOGGLE NOW, so it must look pressable and read its own state.
     Unselected recedes rather than disappearing -- the rung is still ON DISK
     and its coverage is still a fact, it is simply not in this run. */
  .rungchip {
    cursor: pointer;
    text-align: left;
    font: inherit;
    color: inherit;
    opacity: 0.55;
  }
  .rungchip.on {
    opacity: 1;
    border-color: var(--acc);
    background: var(--acc-soft);
  }
  .rungchip:focus-visible {
    outline: 2px solid var(--focus, var(--acc));
    outline-offset: 1px;
  }
  @keyframes rungin {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .rungchip-top {
    display: flex;
    align-items: baseline;
    gap: 0.35rem;
  }
  /* The gauge. Months this rung holds against months the instrument holds --
     the same question the ledger's coverage column answers per run, asked
     here per rung before the run exists. */
  .rungchip-track {
    display: block;
    height: 3px;
    border-radius: 2px;
    background: var(--n0);
    overflow: hidden;
  }
  .rungchip-fill {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: linear-gradient(to right, var(--up), var(--acc));
    /* GROWS TO ITS WIDTH, because the width IS the fact. Same reasoning as
       `.cover-fill`, and the same keyframe. */
    animation: fill 0.5s cubic-bezier(0.22, 0.7, 0.3, 1) both;
    animation-delay: calc(var(--i, 0) * 34ms + 60ms);
    transform-origin: left;
  }
  .rungchip.short .rungchip-fill {
    background: linear-gradient(to right, var(--warn), var(--down));
  }
  /* A rung holding fewer months than its instrument is a SHORTER SAMPLE.
     It takes the warn rail rather than a flash, for the same reason
     `complete: NO` does: the row is not comparable, and that stays true
     for as long as it is on screen. */
  .rungchip.short {
    border-color: var(--warn);
    background: var(--warn-soft, transparent);
  }
  .rungchip-n {
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    color: var(--acc);
  }
  .rungchip-m {
    font-family: var(--num);
    font-size: var(--fs-micro);
    font-variant-numeric: tabular-nums;
    color: var(--n11);
  }
  .rungchip-w {
    font-family: var(--num);
    font-size: var(--fs-micro);
    font-variant-numeric: tabular-nums;
    color: var(--warn);
  }
  .coverbar-note {
    margin: 0;
    font-size: var(--fs-micro);
    color: var(--n9);
    max-width: 92ch;
  }
  .coverbar-note b {
    color: var(--n11);
  }
  /* THE THREE CONTROLS IN THIS ROW WERE THREE DIFFERENT HEIGHTS AND TWO
     DIFFERENT FONT SIZES. Measured: the instrument `select.find.sm` at 29px
     and 12.48px, each span menu's `.pbtn` at 48px and 16px, the Run button
     at 35px -- bottom-aligned, so their tops staggered across 44px and the
     row read as three unrelated widgets that happened to be adjacent.

     `Picker` is sized for `/ingest`'s header, where it is the biggest thing
     on the line. Here it sits beside a small select, so it takes the small
     control's metrics. `:global` because the button belongs to the child
     component; the OVERRIDE lives here because the mismatch is a property of
     this row rather than of `Picker`, which is right as it is where it came
     from. */
  .runf :global(.pbtn) {
    font-size: var(--fs-mini);
    font-weight: var(--w-mid);
    border-radius: 6px;
    padding: 5px 28px 5px 8px;
    background-position: calc(100% - 14px) 55%, calc(100% - 9px) 55%;
  }
  /* Same height as the fields it sits beside, so the row has ONE baseline. */
  .runbar .btn.run {
    padding-block: 5px;
    align-self: flex-end;
  }
  /* The span menus stand in for two text inputs, so the placeholder that
     replaces them while the census loads must hold the same line. */
  .runf-wait {
    font-size: var(--fs-micro);
    color: var(--n8);
    padding: 0.3rem 0;
  }
  /* MOTION IS OFF WHEN IT IS ASKED TO BE. The strip still arrives and the
     gauges still show their width -- only the travel is removed, so nothing
     the animation was carrying is lost with it. */
  @media (prefers-reduced-motion: reduce) {
    .rungchip,
    .rungchip-fill {
      animation: none;
    }
    .rungchip:hover {
      transform: none;
    }
  }

  .runf {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .runf > span {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--n8);
  }
  .find.sm {
    flex: 0 0 auto;
    max-width: 11rem;
    padding: 0.34rem 0.5rem;
    font-size: 0.78rem;
  }
  /* THE ONE BUTTON THAT CAUSES WORK wears the accent. Everything else on
     this page reads; this one writes to an append-only ledger. */
  .btn.run {
    background: var(--acc);
    border-color: var(--acc);
    color: var(--on-acc);
    font-weight: 600;
  }
  .btn.run:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .btn.run:disabled {
    opacity: 0.5;
  }
  .runstate {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .runstate.good {
    border-left-color: var(--up);
    background: var(--up-soft);
  }
  .inline-note.bad {
    border-left-color: var(--down);
    background: var(--down-soft);
  }

  /* ---------------- blocks ---------------- */
  .block {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  /* A SECTION HEADING AT 1rem IS THE BODY SIZE, and it was doing the job of
     dividing the page into "the answer", "which rung", "the ledger" while
     weighing exactly as much as the prose under it. The accent rail below
     was carrying the whole division on its own. `--fs-md` is one step up
     the prose scale -- still under the title, clearly over the copy. */
  .bh {
    margin: 0;
    font-size: var(--fs-md);
    font-weight: var(--w-bold);
    letter-spacing: -0.01em;
    color: var(--n12);
  }
  .bsub {
    margin: 0;
    font-size: var(--fs-mini);
    color: var(--n9);
    max-width: 78ch;
  }
  /* `.bsub b` WAS THE OTHER HALF OF THIS RULE AND ITS MARKUP IS GONE — 761934b
     replaced the stack of cards with one panel and took the prose with it. Only
     the orphaned SELECTOR is removed, not the rule: `.rgroup-note b` still has
     markup and still needs this colour. Deleting by the line the compiler names
     would have taken the surviving half with it. */
  .rgroup-note b {
    color: var(--n11);
  }
  .bh-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .count {
    font-size: 0.75rem;
    color: var(--n8);
    font-variant-numeric: tabular-nums;
  }
  .find {
    flex: 1 1 200px;
    max-width: 320px;
    padding: 0.4rem 0.65rem;
    background: var(--n2);
    border: 1px solid var(--n6);
    border-radius: 7px;
    color: var(--n11);
    font: inherit;
    font-size: 0.82rem;
  }
  .find:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }

  /* ---------------- the crown ---------------- */
  .crown {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
    flex-wrap: wrap;
    background: var(--n3);
    border: 1px solid var(--acc);
    border-left: 3px solid var(--acc);
    border-radius: 10px;
    padding: 1rem 1.2rem;
    box-shadow: var(--e2);
  }
  .crown-mark {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--acc);
    align-self: flex-start;
    padding-top: 0.2rem;
    white-space: nowrap;
  }
  .crown-body {
    flex: 1 1 320px;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .crown-id {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .crown-id b {
    font-size: var(--fs-md);
    letter-spacing: -0.01em;
    color: var(--n12);
  }
  /* THE LEAD FIGURE GETS ITS OWN COLUMN, not a slot in a flat row of four.
     `worst-case total` is the number this page is ranked on and the other
     three are context for it; laid out side by side at one gap they read as
     four equal readings and the eye picks the one that happens to be first.
     A divider and a wider gap after the lead say which is which without
     needing a label to explain it. */
  .crown-figs {
    display: flex;
    gap: 1.5rem;
    flex-wrap: wrap;
    align-items: flex-end;
  }
  .fig {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .fig.lead {
    padding-right: 1.5rem;
    border-right: 1px solid var(--n6);
  }
  .fig .k {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--n8);
  }
  .fig .v {
    font-family: var(--num);
    font-size: var(--fs-data);
    font-variant-numeric: tabular-nums;
    color: var(--n11);
  }
  /* 1.35rem WAS 21.6px AT WEIGHT 400, AND THE SUMMARY COUNTERS ABOVE IT WERE
     24px. The single number this entire page exists to produce was rendering
     SMALLER than a row of zeroes, in the same weight as the labels beside it.
     `--fs-data-xl` is the display step of the data scale -- the step the
     token was defined for -- and it is the only thing on the page that uses
     it, which is what makes it read as the answer. */
  .fig.lead .v {
    font-size: var(--fs-data-xl);
    font-weight: var(--w-bold);
    letter-spacing: -0.02em;
    line-height: 1.05;
  }
  .fig .v.up {
    color: var(--up);
  }
  .fig .v.ghost {
    color: var(--n9);
    font-weight: var(--w-reg);
  }
  .crown-note {
    margin: 0;
    font-size: var(--fs-mini);
    color: var(--n9);
    max-width: 70ch;
  }
  .crown-note b {
    color: var(--n11);
  }

  /* ---------------- rung multiples ---------------- */
  .rgroup {
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: 10px;
    padding: 0.9rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  .rgroup-head {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    flex-wrap: wrap;
    font-size: 0.82rem;
  }
  .rgroup-head b {
    color: var(--n12);
    font-size: 0.95rem;
  }
  .rgroup-note {
    margin: 0;
    font-size: 0.78rem;
    color: var(--n9);
  }
  .rgroup-note.warn {
    color: var(--warn);
  }

  .multiples {
    display: flex;
    gap: 0.5rem;
    overflow-x: auto;
    padding-bottom: 0.3rem;
  }
  .mult {
    flex: 0 0 84px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    background: var(--n2);
    border: 1px solid var(--n6);
    border-radius: 8px;
    padding: 0.55rem 0.4rem;
    cursor: pointer;
    font: inherit;
    transition:
      border-color 0.15s ease,
      transform 0.15s ease;
  }
  .mult:hover {
    border-color: var(--acc);
    transform: translateY(-1px);
  }
  .mult:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  .mult.leader {
    border-color: var(--acc);
    background: var(--acc-soft);
  }
  /* A PERSISTENT RAIL, NOT A FLASH. This row is not comparable, permanently. */
  .mult.halted {
    border-left: 3px solid var(--warn);
  }
  .mult-bars {
    position: relative;
    width: 34px;
    height: 66px;
    display: block;
    background: var(--n0);
    border-radius: 3px;
    overflow: hidden;
  }
  .mult-ghost,
  .mult-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    border-radius: 3px 3px 0 0;
  }
  .mult-ghost {
    background: var(--n6);
  }
  .mult-bar {
    background: var(--up);
  }
  .mult-bar.neg {
    background: var(--down);
  }
  .mult-rung {
    font-size: var(--fs-mini);
    color: var(--n11);
    font-variant-numeric: tabular-nums;
  }
  .mult-fig {
    font-size: var(--fs-micro);
    color: var(--n9);
    font-variant-numeric: tabular-nums;
  }
  .mult-flag {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--warn);
  }

  /* ====================================================================
     INTEGRITY — a register of its own, not a louder amber

     The page already spends amber on halted and on span holes, and both
     mean "this figure is real but does not cover what you asked for". A
     failed seal means something categorically different: the figure may
     not be a figure. Painting it amber would file it under "incomplete"
     beside the two, which is precisely the merge the API refuses to make
     when it counts `unsealed` apart from `halted`.

     It borrows `--down` rather than inventing a token, and that IS a
     collision worth naming: `--down` is the loss red, so a struck-out
     PROFIT will render in the colour this page otherwise reserves for
     losing money. The alternative was a fourth semantic colour, which
     would have to earn its place in `theme.css` across three theme states
     for one state that should never occur.

     The distinction is therefore carried STRUCTURALLY, not chromatically:
     nothing else on this page strikes a figure through. Amber says "mind
     this number", a strike says "do not use this number", and those read
     apart at a glance even when the hue does not.
     ==================================================================== */
  .bt-integrity {
    display: flex;
    gap: 0.75rem;
    align-items: flex-start;
    padding: 0.8rem 1rem;
    border-left: 3px solid var(--down);
    background: var(--down-soft);
    /* A FACT ARRIVED, so something moves — once, briefly, and never
       again. The banner is absent on every healthy load, so this plays
       only when there is something new to say. */
    animation: bt-integrity-in 260ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }

  @keyframes bt-integrity-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .bt-integrity-mark {
    font-size: 1.05rem;
    line-height: 1.35;
    color: var(--down);
  }

  .bt-integrity-say {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    /* 65ch, so the sentence is read rather than scanned past. */
    max-width: 68ch;
  }

  .bt-integrity-say strong {
    font-size: 0.82rem;
    letter-spacing: 0.01em;
    color: var(--down);
  }

  .bt-integrity-say span {
    font-size: 0.76rem;
    line-height: 1.5;
    color: var(--n9);
  }

  .bt-integrity-say code {
    font-family: var(--mono);
    font-size: 0.92em;
  }

  .pill.seal {
    background: var(--down-soft);
    color: var(--down);
  }

  .mult-flag.seal {
    color: var(--down);
  }

  /* THE FIGURE ITSELF IS STRUCK, not the whole row. The rung name and the
     span are facts about which record this is and are not in doubt; only
     the number the seal covers is. Striking the row would deny the
     operator the very labels they need to go and re-run it. */
  .mult.unsealed .mult-fig,
  .row.unsealed .cell.num {
    text-decoration: line-through;
    text-decoration-color: var(--down);
    text-decoration-thickness: 1px;
    opacity: 0.72;
  }

  .mult.unsealed .mult-bar,
  .mult.unsealed .mult-ghost {
    /* Hatched rather than solid: a bar drawn to a height derived from a
       number in doubt must not read as a measurement. */
    opacity: 0.45;
  }

  .row.unsealed {
    background: var(--down-soft);
  }

  @media (prefers-reduced-motion: reduce) {
    .bt-integrity {
      animation: none;
    }
  }

  /* ---------------- the table ---------------- */
  .tbl-scroll {
    max-height: 560px;
    overflow: auto;
    position: relative;
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: 10px;
  }
  .tbl {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }
  .tbl thead th {
    position: sticky;
    top: 0;
    z-index: 3;
    background: var(--n2);
    text-align: left;
    padding: 0.5rem 0.6rem;
    border-bottom: 1px solid var(--n6);
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--n8);
    font-weight: 600;
  }
  .tbl thead th.num {
    text-align: right;
  }
  .sortbtn {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: inherit;
    cursor: pointer;
    text-transform: inherit;
    letter-spacing: inherit;
  }
  .sortbtn:hover {
    color: var(--acc);
  }
  .sortbtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }

  .bt-spacer {
    position: relative;
  }
  .row {
    position: absolute;
    left: 0;
    right: 0;
    height: 44px;
    display: grid;
    grid-template-columns: 1.3fr 0.7fr 1.2fr 1.1fr 1.2fr 0.7fr 1fr 1fr 1.2fr;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.6rem;
    border-bottom: 1px solid var(--n5);
    cursor: pointer;
    border-left: 3px solid transparent;
    animation: arrive 0.28s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  /* A RUN JUST FINISHED — the row slides in and settles. Nothing else moves. */
  @keyframes arrive {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .row:hover {
    background: var(--n4);
  }
  .row:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .row.halted {
    border-left-color: var(--warn);
  }
  .row.crowned {
    background: var(--acc-soft);
  }
  .row.open {
    background: var(--n4);
    border-left-color: var(--acc);
  }

  .cell {
    font-size: 0.8rem;
    color: var(--n10);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cell.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .cell.strong {
    color: var(--n12);
    font-weight: 600;
  }
  .cell.neg {
    color: var(--down);
  }
  .inst {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .inst b {
    color: var(--n11);
    font-size: 0.82rem;
  }
  .dim {
    color: var(--n9);
  }
  /* `.sm` IS NOT DECORATION -- it carries the feed name, the span, the
     coverage count and the finish timestamp in every ledger row. At 0.7rem
     it rendered at 11.2px, under the 12px floor `theme.css` raised twice
     for exactly this reason, and it was the smallest real TEXT on the page. */
  .sm {
    font-size: var(--fs-micro);
  }
  .rung {
    font-size: var(--fs-mini);
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    background: var(--n4);
    color: var(--n10);
    font-variant-numeric: tabular-nums;
  }

  .cover {
    display: block;
    height: 5px;
    background: var(--n0);
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 2px;
  }
  .cover-fill {
    display: block;
    height: 100%;
    background: var(--up);
    transition: width 0.3s ease;
  }
  .cover-fill.short {
    background: var(--warn);
  }

  .pill {
    display: inline-block;
    font-size: var(--fs-micro);
    padding: 0.12rem 0.42rem;
    border-radius: 4px;
    background: var(--n4);
    color: var(--n9);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
  }
  .pill.good {
    background: var(--up-soft);
    color: var(--up);
  }
  .pill.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }
  .pill.flat {
    background: var(--n4);
    color: var(--n8);
  }

  /* ==================================================================
     THE VISUAL PASS
     ------------------------------------------------------------------
     Everything below is surface: depth, gradient, glow and entrance.
     None of it moves a number or hides one. The rule the rest of this
     file keeps — a fact gets a sentence, an absence gets a padlock —
     is untouched; this only decides how the facts LOOK while they say
     what they already said.

     Where a colour carries meaning it is the same colour it was: green
     is still worst-case-positive, amber is still halted, the accent is
     still "this is the answer". Gradients run between a token and a
     transparent version of the SAME token, never between two hues, so
     nothing here invents a third state the reader has to learn.
     ================================================================== */

  /* ---- section headings get a rule that starts at the accent -------- */
  .bh {
    position: relative;
    padding-left: 0.75rem;
  }
  .bh::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0.15em;
    bottom: 0.15em;
    width: 3px;
    border-radius: 2px;
    background: linear-gradient(to bottom, var(--acc), transparent);
  }

  /* ---- the summary strip: depth, and a hover that lifts ------------- */
  .bt-strip {
    box-shadow: var(--e1);
  }
  .fact {
    position: relative;
    transition:
      background 0.2s ease,
      transform 0.2s cubic-bezier(0.22, 0.7, 0.3, 1);
  }
  .fact::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: linear-gradient(to right, var(--acc), transparent);
    opacity: 0;
    transition: opacity 0.2s ease;
  }
  .fact:hover {
    background: var(--n4);
  }
  .fact:hover::after {
    opacity: 0.7;
  }
  .fact.good::after {
    background: linear-gradient(to right, var(--up), transparent);
  }
  .fact.warn::after {
    background: linear-gradient(to right, var(--warn), transparent);
  }

  /* ---- THE ANSWER. The one card on the page that is a verdict, so it
     is the one that gets a glow. A gradient ground away from the accent
     edge, and a soft outer light that says "start here" without moving. */
  .crown {
    position: relative;
    background:
      linear-gradient(135deg, var(--acc-soft) 0%, transparent 42%),
      var(--n3);
    box-shadow:
      var(--e2),
      0 0 0 1px var(--acc),
      0 8px 32px -18px var(--acc);
    overflow: hidden;
  }
  /* A single sweep of light across the card as it arrives, then gone.
     It runs ONCE — a card that keeps shimmering is a card still loading,
     and this one has finished. */
  .crown::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(105deg, transparent 35%, rgba(255, 255, 255, 0.06) 50%, transparent 65%);
    transform: translateX(-100%);
    animation: sheen 1.1s cubic-bezier(0.3, 0.7, 0.4, 1) 0.25s 1 forwards;
    pointer-events: none;
  }
  @keyframes sheen {
    to {
      transform: translateX(100%);
    }
  }
  .crown-mark {
    position: relative;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    background: var(--acc-soft);
    border: 1px solid var(--acc);
  }
  .fig.lead .v {
    text-shadow: 0 0 24px var(--up-soft);
  }

  /* ---- the rung multiples, as a chart rather than four boxes -------- */
  .rgroup {
    background: linear-gradient(180deg, var(--n3) 0%, var(--n2) 100%);
    box-shadow: var(--e1);
  }
  .multiples {
    padding: 0.4rem 0.1rem 0.5rem;
  }
  .mult {
    flex: 0 0 92px;
    background: linear-gradient(180deg, var(--n2) 0%, var(--n0) 100%);
    transition:
      border-color 0.18s ease,
      transform 0.18s cubic-bezier(0.22, 0.7, 0.3, 1),
      box-shadow 0.18s ease;
  }
  .mult:hover {
    transform: translateY(-3px);
    box-shadow: var(--e2);
  }
  /* ---- ONE RUNG: A ROW, NOT A PLOT ---------------------------------
     See the comment on `.multiples.solo` in the markup for why the bar is
     withdrawn rather than shrunk: at n=1 its height is fixed at 100% by
     construction, so it is a picture that cannot carry a number.
     The tile becomes a horizontal chip — rung, figure, flags — which is
     everything the chart was actually telling the reader. */
  .multiples.solo .mult {
    flex: 0 1 auto;
    flex-direction: row;
    align-items: baseline;
    gap: var(--s3, 0.5rem);
    padding: 0.45rem 0.7rem;
  }
  .multiples.solo .mult-bars {
    display: none;
  }
  .multiples.solo .mult-fig {
    font-size: var(--fs-1, 1rem);
  }
  /* NO LIFT ON HOVER. The lift reads as "compare me with the one beside
     me"; there is nothing beside it. The border still answers the cursor,
     so the row is still visibly a control. */
  .multiples.solo .mult:hover {
    transform: none;
    box-shadow: none;
    border-color: var(--acc);
  }
  /* THE LEADER GLOWS. It is the rung that carries the edge for this span,
     and on a row of four bars the eye should land on it first. */
  .mult.leader {
    border-color: var(--acc);
    box-shadow: 0 0 0 1px var(--acc), 0 6px 22px -14px var(--acc);
  }
  .mult-bars {
    width: 40px;
    height: 92px;
    background: linear-gradient(180deg, var(--n0) 0%, rgba(0, 0, 0, 0.25) 100%);
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.4);
  }
  /* The best-case fill is an OUTLINE. As a solid it always covered the
     figure that actually ranks, because it is always the taller of the
     two. */
  .mult-ghost {
    background: transparent;
    border: 1px dashed var(--n7);
    border-bottom: 0;
    border-radius: 3px 3px 0 0;
  }
  .mult-bar {
    background: linear-gradient(180deg, var(--up) 0%, var(--up-soft) 140%);
    box-shadow: 0 -2px 12px -4px var(--up);
  }
  .mult-bar.neg {
    background: linear-gradient(180deg, var(--down) 0%, var(--down-soft) 140%);
    box-shadow: 0 -2px 12px -4px var(--down);
  }
  .mult.leader .mult-bar {
    box-shadow: 0 -3px 18px -3px var(--up);
  }
  .mult-fig {
    font-weight: 600;
    color: var(--n11);
  }
  .mult.leader .mult-rung {
    color: var(--acc);
    font-weight: 600;
  }

  /* ---- the coverage bar reads as a gauge --------------------------- */
  .cover {
    height: 6px;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.35);
  }
  .cover-fill {
    background: linear-gradient(to right, var(--up), var(--acc));
  }
  .cover-fill.short {
    background: linear-gradient(to right, var(--warn), var(--down));
  }

  /* ---- the ledger table ------------------------------------------- */
  .tbl-scroll {
    box-shadow: var(--e1);
  }
  .row {
    transition:
      background 0.16s ease,
      border-left-color 0.16s ease,
      box-shadow 0.16s ease;
  }
  .row:hover {
    background: var(--n4);
    box-shadow: inset 3px 0 0 var(--acc);
  }
  .row.crowned {
    background: linear-gradient(to right, var(--acc-soft), transparent 45%);
    border-left-color: var(--acc);
  }
  .row.halted {
    background: linear-gradient(to right, var(--warn-soft), transparent 30%);
  }

  /* ---- the Run bar: the one control that causes work --------------- */
  .runbar {
    background: linear-gradient(120deg, var(--acc-soft) 0%, var(--n3) 38%);
    box-shadow: var(--e1);
  }
  .btn.run {
    box-shadow: 0 4px 18px -8px var(--acc);
    transition:
      filter 0.16s ease,
      transform 0.16s cubic-bezier(0.22, 0.7, 0.3, 1),
      box-shadow 0.16s ease;
  }
  .btn.run:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 7px 24px -8px var(--acc);
  }
  .btn.run:active:not(:disabled) {
    transform: translateY(0);
  }

  /* ---- SELECTS: gone with the support control ----------------------
     Twenty-seven lines dressed the native dropdown arrow and turned it on
     focus. The support threshold was the only <select> on this page, so
     they now style nothing. Kept as a note rather than silently deleted:
     if a select ever returns here, it should look like that again. ---- */

  /* Text inputs get the same focus treatment, so the strip reads as one
     set of controls rather than as two kinds. */
  .find {
    transition:
      border-color 0.16s ease,
      box-shadow 0.16s ease;
  }
  .find:hover {
    border-color: var(--n7);
  }
  .find:focus,
  .find:focus-visible {
    border-color: var(--acc);
    box-shadow: 0 0 0 3px var(--acc-soft);
    outline: none;
  }

  /* ---- the drill-down cards ---------------------------------------- */
  .card {
    background: linear-gradient(180deg, var(--n3) 0%, var(--n2) 100%);
    transition:
      border-color 0.18s ease,
      transform 0.18s cubic-bezier(0.22, 0.7, 0.3, 1),
      box-shadow 0.18s ease;
  }
  .card:hover {
    border-color: var(--n7);
    transform: translateY(-2px);
    box-shadow: var(--e2);
  }
  /* A card that is a stated GAP does not lift: there is nothing in it to
     inspect, and inviting a hover would promise otherwise. */
  .card.gap:hover {
    transform: none;
    box-shadow: none;
    border-color: var(--n6);
  }

  /* ---- the price terminal ------------------------------------------ */
  .term {
    box-shadow: var(--e2);
  }
  .term-bar {
    background: linear-gradient(180deg, var(--n2) 0%, var(--n3) 100%);
  }
  .rungbtn.on {
    box-shadow: 0 2px 10px -4px var(--acc);
  }

  /* ---- the page arrives in reading order ---------------------------

     `backwards` AND NOT `both`, AND THE DIFFERENCE IS A RENDERING BUG.

     With `both` the final keyframe stays applied after the animation ends,
     and `transform: none` in a keyframe RESOLVES to `matrix(1,0,0,1,0,0)`.
     An identity matrix is still a transform: it makes the element a
     containing block and a STACKING CONTEXT, permanently. Measured on
     `.runbar` after the animation finished: `matrix(1, 0, 0, 1, 0, 0)`.

     Every element in this selector list is a section of the shell, so all
     three became stacking contexts in DOM order -- and `$lib/Picker.svelte`
     hangs its menu on `position: absolute; z-index: 40`. Trapped inside
     `.runbar`'s context, a z-index of 40 cannot lift the menu over
     `.bt-strip` or `.block`, which paint later. Opening either span menu
     drew the summary strip THROUGH it.

     `backwards` holds the `from` state through the delay, which is all the
     fill was ever needed for: the `to` state is the element's own resting
     style, so there is nothing to hold afterwards. When the animation ends
     the transform is genuinely `none` and no context is left behind. */
  .runbar,
  .bt-strip,
  .block {
    animation: pagein 0.45s cubic-bezier(0.22, 0.7, 0.3, 1) backwards;
  }
  .runbar {
    animation-delay: 0.02s;
  }
  .bt-strip {
    animation-delay: 0.08s;
  }
  .block:nth-of-type(1) {
    animation-delay: 0.14s;
  }
  .block:nth-of-type(2) {
    animation-delay: 0.2s;
  }
  .block:nth-of-type(3) {
    animation-delay: 0.26s;
  }
  @keyframes pagein {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  /* Nothing above moves for a reader who asked for stillness, and the
     sheen in particular must not leave the card mid-sweep. */
  @media (prefers-reduced-motion: reduce) {
    .crown::before {
      animation: none;
      opacity: 0;
    }
    .runbar,
    .bt-strip,
    .block,
    .mult,
    .card,
    .fact,
    .row,
    .btn.run,
    .find {
      animation: none !important;
      transition: none !important;
      transform: none !important;
    }
  }

  /* ---------------- the drill-down ---------------- */
  .drill {
    scroll-margin-top: 1rem;
  }
  .drill-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(290px, 1fr));
    gap: 0.9rem;
  }
  .card {
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: 10px;
    padding: 0.9rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    animation: arrive 0.3s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .card.gap {
    border-style: dashed;
  }
  .cnote {
    margin: 0;
    font-size: 0.76rem;
    color: var(--n9);
  }
  .cnote b {
    color: var(--n11);
  }
  .cnote.faint {
    color: var(--n8);
    font-size: var(--fs-mini);
  }

  .paired {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .prow {
    display: grid;
    grid-template-columns: 8.5ch 1fr auto;
    gap: 0.5rem;
    align-items: center;
  }
  .plab {
    font-size: var(--fs-micro);
    color: var(--n8);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .pbar {
    height: 9px;
    background: var(--n0);
    border-radius: 4px;
    overflow: hidden;
  }
  .pfill {
    display: block;
    height: 100%;
    background: var(--up);
  }
  .pfill.ghosted {
    background: var(--n6);
  }
  .pval {
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    color: var(--n10);
  }
  .pval.strong {
    color: var(--n12);
    font-weight: 600;
  }

  .kv {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.15rem 0.6rem;
    margin: 0;
    font-size: 0.76rem;
  }

  .exc {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .erow {
    display: grid;
    grid-template-columns: 11ch 1fr auto;
    gap: 0.5rem;
    align-items: center;
  }
  .elab {
    font-size: var(--fs-micro);
    color: var(--n8);
  }
  .ebar {
    height: 9px;
    background: var(--n0);
    border-radius: 4px;
    overflow: hidden;
  }
  .efill {
    display: block;
    height: 100%;
    animation: grow 0.4s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .efill.up {
    background: var(--up);
  }
  .efill.warn {
    background: var(--warn);
  }
  .efill.down {
    background: var(--down);
  }
  @keyframes grow {
    from {
      transform: scaleX(0);
      transform-origin: left;
    }
    to {
      transform: none;
    }
  }
  .eval {
    font-size: var(--fs-mini);
    color: var(--n10);
    font-variant-numeric: tabular-nums;
  }

  .axes {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .axis {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.76rem;
    padding: 0.3rem 0.5rem;
    background: var(--n4);
    border-radius: 5px;
  }
  .axis-k {
    color: var(--n9);
  }
  .axis-v {
    color: var(--n11);
    font-variant-numeric: tabular-nums;
  }

  /* ---------------- buttons ---------------- */
  .btn {
    background: var(--n2);
    border: 1px solid var(--n6);
    border-radius: 7px;
    padding: 0.42rem 0.8rem;
    color: var(--n11);
    font: inherit;
    font-size: 0.82rem;
    cursor: pointer;
    transition: border-color 0.15s ease;
  }
  .btn:hover:not(:disabled) {
    border-color: var(--acc);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .btn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  .btn.ghost {
    background: transparent;
  }
  .btn.sm {
    font-size: 0.75rem;
    padding: 0.3rem 0.6rem;
  }

  /* A CONTRADICTION IN THE DATA reads as loud as a failed request, because
     that is what it is: a figure that cannot be true beside twelve that look
     exactly like the ones that are. */
  .inline-note.bad {
    border-left-color: var(--down);
    background: var(--down-soft);
    color: var(--n10);
  }
  .cnote.warn-t {
    color: var(--warn);
  }

  /* ---------------- the price chart ---------------- */
  .chartcard {
    gap: 0.7rem;
  }
  .chart-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .chart-facts {
    font-size: var(--fs-mini);
    color: var(--n8);
    font-variant-numeric: tabular-nums;
  }
  /* A HEIGHT IN PIXELS AND A REFUSAL TO SHRINK, and both halves are a fix for
     a measured bug. `autoSize` measures its host, so a host with no height
     measures zero; and `.card` is a column flex container, where a child
     defaults to `flex-shrink: 1` — which took this box to 32px against its
     declared 340px and drew a chart that looked like one that had failed.
     Exactly the defect the `.page > *` rule above fixes one level up. */
  .bt-chart {
    height: 340px;
    flex: 0 0 340px;
    width: 100%;
    border: 1px solid var(--n6);
    border-radius: 8px;
    overflow: hidden;
    background: var(--n2);
  }
  .chart-state {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    min-height: 96px;
    padding: 0.9rem 1rem;
    border: 1px dashed var(--n6);
    border-radius: 8px;
    background: var(--n2);
  }
  .chart-state p {
    margin: 0;
    font-size: 0.78rem;
    color: var(--n9);
    max-width: 78ch;
  }
  .chart-state b {
    color: var(--n11);
  }
  .chart-state.bad {
    border-style: solid;
    border-color: var(--down);
    background: var(--down-soft);
  }

  /* ================= THE STRATEGY REPORT ================= */
  .report {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: 10px;
    padding: 1rem 1.1rem 1.1rem;
    animation: arrive 0.35s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .rep-head {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .rep-sub {
    font-size: var(--fs-mini);
    color: var(--n8);
  }

  /* ---- key stats ---- */
  .keystats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 1px;
    background: var(--n6);
    border: 1px solid var(--n6);
    border-radius: 8px;
    overflow: hidden;
  }
  .ks {
    background: var(--n2);
    padding: 0.7rem 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .ks-k {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--n8);
  }
  .ks-v {
    font-size: 1.15rem;
    font-variant-numeric: tabular-nums;
    color: var(--n12);
    line-height: 1.15;
  }
  .ks-v.up {
    color: var(--up);
  }
  .ks-v.down {
    color: var(--down);
  }
  .ks-n {
    font-size: 0.7rem;
    color: var(--n9);
  }

  /* ---- benchmark ---- */
  .bcmp {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-bottom: 0.6rem;
  }
  .brow {
    display: grid;
    grid-template-columns: 9ch 1fr auto;
    gap: 0.6rem;
    align-items: center;
  }
  .blab {
    font-size: var(--fs-micro);
    color: var(--n8);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .bbar {
    height: 12px;
    background: var(--n0);
    border-radius: 4px;
    overflow: hidden;
  }
  .bfill {
    display: block;
    height: 100%;
    background: var(--up);
    animation: grow 0.5s cubic-bezier(0.22, 0.7, 0.3, 1) both;
    transform-origin: left;
  }
  .bfill.hold {
    background: var(--n7);
  }
  .bfill.down {
    background: var(--down);
  }
  .bval {
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
    color: var(--n10);
  }
  .bval.strong {
    color: var(--n12);
    font-weight: 600;
  }
  .spin.sm {
    width: 10px;
    height: 10px;
    display: inline-block;
    vertical-align: -1px;
    margin-right: 0.35rem;
  }

  /* ---- the coverage bar ----
     `.cover` now styles a `<span>` progress bar in the run table — "N of M
     months on disk". It used to head a `<details>` reference table, and the
     comment here said that table was "never removed -- the whole point is
     that the gaps are documented rather than merely absent". IT IS REMOVED.
     The markup went, these two properties stayed, and the four `summary`
     rules under them styled nothing until Gate W4 refused the build. The
     claim went with them, because a comment that keeps asserting a guarantee
     the page no longer delivers is the exact failure that gate is for. If the
     documented gaps are wanted back they are a surface to rebuild, not a
     sentence to restore.

     THE TWO PROPERTIES ARE NOW GONE TOO, AND LEAVING THEM WAS NOT HARMLESS.
     They were `border-top: 1px solid var(--n5)` and `padding-top: 0.75rem`,
     written for a `<details>` heading and left behind when its markup went.
     `.cover` was then REUSED for the coverage gauge in the ledger row, and
     this rule sits after the two that size it. Measured: `box-sizing` is
     `border-box`, so 12px of padding plus a 1px border pushed the declared
     `height: 6px` out to 13px and collapsed the CONTENT box to zero --
     `.cover-fill { height: 100% }` computed to `0px`.

     Every row on this page reads 81/81 and every gauge drew nothing. That is
     a fact present in the DOM and invisible on screen, which is the failure
     `CLAUDE.md` §4 bans and the one the `flex-shrink` comment at the top of
     this block already records catching once. An orphaned selector is dead
     code until its class name is reused; after that it is a silent defect. */
  .covtbl-wrap {
    overflow-x: auto;
    margin: 0.7rem 0;
    border: 1px solid var(--n6);
    border-radius: 8px;
  }
  .covtbl {
    width: 100%;
    min-width: 560px;
    border-collapse: collapse;
    font-size: 0.78rem;
  }

  /* ================= THE STRATEGY TESTER =================
     TradingView's docked-panel proportions: a thin toolbar, then flat
     sections separated by hairlines, generous horizontal padding, and
     four-across stat quads. Deliberately FLATTER than the cards above --
     the Strategy Tester is a dense readout, not a set of tiles, and the
     density is what makes it scannable. */
  .tester {
    display: flex;
    flex-direction: column;
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--e2);
    animation: arrive 0.35s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .tester > * {
    flex: 0 0 auto;
  }

  /* ---- toolbar ---- */
  .tt-bar {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    flex-wrap: wrap;
    padding: 0.6rem 0.95rem;
    background: var(--n2);
    border-bottom: 1px solid var(--n6);
  }
  .tt-name {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }
  .tt-ico {
    width: 15px;
    height: 15px;
    color: var(--acc);
  }
  .tt-name b {
    font-size: 0.88rem;
    color: var(--n12);
  }
  .tt-dim {
    font-size: 0.75rem;
    color: var(--n8);
  }
  .tt-views {
    display: flex;
    gap: 2px;
    background: var(--n0);
    padding: 2px;
    border-radius: 7px;
  }
  .tt-view {
    background: transparent;
    border: 0;
    border-radius: 5px;
    padding: 0.28rem 0.65rem;
    font: inherit;
    font-size: 0.76rem;
    color: var(--n9);
    cursor: pointer;
    transition:
      background 0.14s ease,
      color 0.14s ease;
  }
  .tt-view:hover {
    background: var(--n4);
    color: var(--n11);
  }
  .tt-view.on {
    background: var(--acc);
    color: var(--on-acc);
  }
  .tt-view:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  .tt-meta {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin-left: auto;
  }
  .tt-chip {
    font-size: var(--fs-micro);
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
    background: var(--n4);
    color: var(--n9);
    white-space: nowrap;
  }
  .tt-chip.good {
    background: var(--up-soft);
    color: var(--up);
  }
  .tt-chip.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }

  /* ---- toolbar controls (TradingView's pill-shaped buttons) ---- */
  .tt-ctl {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 0;
    border-radius: 6px;
    padding: 0.3rem 0.55rem;
    font: inherit;
    font-size: 0.75rem;
    color: var(--n10);
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.14s ease;
  }
  .tt-ctl:hover {
    background: var(--n4);
  }
  .tt-ctl:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  .tt-cico {
    width: 14px;
    height: 14px;
    color: var(--n8);
    flex: none;
  }
  .tt-caret {
    color: var(--n8);
    font-size: 0.7rem;
  }
  .tt-dim2 {
    color: var(--n8);
  }
  .tt-period {
    position: relative;
  }
  .tt-menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 20;
    min-width: 230px;
    background: var(--n2);
    border: 1px solid var(--n6);
    border-radius: 8px;
    box-shadow: var(--e3);
    padding: 0.35rem;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .tt-menuhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0.5rem 0.45rem;
    font-size: 0.75rem;
    color: var(--n11);
    border-bottom: 1px solid var(--n5);
    margin-bottom: 0.25rem;
  }
  .tt-reset {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    font-size: var(--fs-mini);
    color: var(--n8);
    cursor: pointer;
  }
  .tt-reset:hover {
    color: var(--acc);
  }
  .tt-menuitem {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 5px;
    padding: 0.35rem 0.5rem;
    font: inherit;
    font-size: 0.76rem;
    color: var(--n10);
    cursor: pointer;
    text-align: left;
  }
  .tt-menuitem:hover {
    background: var(--n4);
  }
  .tt-menuitem.sel {
    border-color: var(--acc);
    color: var(--n12);
  }
  .tt-default {
    font-size: var(--fs-micro);
    color: var(--n8);
  }

  .tt-info {
    background: none;
    border: 0;
    padding: 0;
    margin-left: 0.3rem;
    line-height: 0;
    color: var(--n8);
    cursor: help;
    vertical-align: -2px;
  }
  .tt-info svg {
    width: 13px;
    height: 13px;
  }
  .tt-info:hover {
    color: var(--n10);
  }
  .tt-menudiv {
    height: 1px;
    background: var(--n5);
    margin: 0.3rem 0;
  }
  .tt-menuic {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
  }
  .tt-menuic svg {
    width: 13px;
    height: 13px;
    color: var(--n8);
  }

  /* ---- icon buttons on a panel header ---- */
  .tt-hrow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .tt-hrow.tight {
    margin-top: 1rem;
  }
  .tt-icons {
    display: flex;
    gap: 0.2rem;
  }
  .tt-iconbtn {
    background: transparent;
    border: 0;
    border-radius: 5px;
    padding: 0.28rem;
    color: var(--n8);
    cursor: pointer;
    line-height: 0;
    transition:
      background 0.14s ease,
      color 0.14s ease;
  }
  .tt-iconbtn svg {
    width: 15px;
    height: 15px;
  }
  .tt-iconbtn:hover {
    background: var(--n4);
    color: var(--n11);
  }
  .tt-iconbtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  .tt-view.ic {
    padding: 0.28rem 0.5rem;
    line-height: 0;
  }
  .tt-view.ic svg {
    width: 15px;
    height: 15px;
  }

  /* ---- segmented toggles (Daily/Weekly/…, By signals/By side, Count/Amount) ---- */
  .tt-seg {
    display: flex;
    gap: 2px;
    background: var(--n0);
    padding: 2px;
    border-radius: 7px;
  }
  .tt-segbtn {
    background: transparent;
    border: 0;
    border-radius: 5px;
    padding: 0.26rem 0.62rem;
    font: inherit;
    font-size: var(--fs-mini);
    color: var(--n9);
    cursor: pointer;
    white-space: nowrap;
    transition:
      background 0.14s ease,
      color 0.14s ease;
  }
  .tt-segbtn:hover {
    background: var(--n4);
    color: var(--n11);
  }
  .tt-segbtn.on {
    background: var(--n3);
    color: var(--n12);
    box-shadow: var(--e1);
  }
  .tt-segbtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }

  /* ---- the two-line stat value ---- */
  .tt-qv.big {
    font-size: 1.24rem;
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .tt-unit {
    font-size: var(--fs-micro);
    font-style: normal;
    letter-spacing: 0.08em;
    color: var(--n8);
  }
  .tt-pc {
    font-size: 0.82rem;
    font-style: normal;
    color: inherit;
    opacity: 0.85;
  }

  /* ==================================================================
     MOTION IN THIS PANEL
     ------------------------------------------------------------------
     Every animation here is an ARRIVAL: the thing was not on screen and
     now it is, and the motion carries it in from where its value comes
     from. Nothing loops, nothing pulses, nothing moves once it has
     landed. A figure that keeps moving reads as a figure still changing,
     and on a page about money that is a lie told with easing curves.

     So: the curve DRAWS itself left to right, the way it was
     accumulated. Bars GROW from the zero line, which is the axis they
     are measured against. Rows and sections rise a few pixels into
     place. Switching a tab re-runs the arrival, because the numbers
     genuinely changed. And every one of them stops.
     ================================================================== */

  /* THE CURVE DRAWS ITSELF. `stroke-dasharray` at the path's own length
     with the offset animated to zero is the only way to do this without
     measuring in JavaScript; 2400 is comfortably longer than any path
     this 1000-unit viewBox produces, and an over-long dash simply starts
     fully hidden, which is what is wanted. */
  .tester .cf-svg path[stroke] {
    stroke-dasharray: 2400;
    stroke-dashoffset: 2400;
    animation: draw 1.1s cubic-bezier(0.33, 0.8, 0.35, 1) forwards;
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  /* The area under it fades in behind the line rather than with it, so
     the line reads as leading and the fill as following. */
  .tester .cf-svg path[fill]:not([stroke]) {
    opacity: 0;
    animation: wash 0.9s ease-out 0.35s forwards;
  }
  @keyframes wash {
    to {
      opacity: 1;
    }
  }

  /* BARS GROW FROM THE ZERO LINE. `transform-box: fill-box` makes the
     origin the rect's own box rather than the SVG root, which is what
     lets a bar below the axis grow downward and one above grow up. */
  .tester .cf-svg rect {
    transform-box: fill-box;
    transform-origin: center bottom;
    animation: sprout 0.55s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  @keyframes sprout {
    from {
      transform: scaleY(0);
      opacity: 0.4;
    }
    to {
      transform: none;
      opacity: 1;
    }
  }
  /* Left to right, capped: past a dozen the last bar waits longer than
     a reader will, and these charts can hold sixty. */
  .tester .cf-svg rect:nth-of-type(1) { animation-delay: 0.02s; }
  .tester .cf-svg rect:nth-of-type(2) { animation-delay: 0.05s; }
  .tester .cf-svg rect:nth-of-type(3) { animation-delay: 0.08s; }
  .tester .cf-svg rect:nth-of-type(4) { animation-delay: 0.11s; }
  .tester .cf-svg rect:nth-of-type(5) { animation-delay: 0.14s; }
  .tester .cf-svg rect:nth-of-type(6) { animation-delay: 0.17s; }
  .tester .cf-svg rect:nth-of-type(7) { animation-delay: 0.2s; }
  .tester .cf-svg rect:nth-of-type(8) { animation-delay: 0.23s; }
  .tester .cf-svg rect:nth-of-type(9) { animation-delay: 0.26s; }
  .tester .cf-svg rect:nth-of-type(10) { animation-delay: 0.29s; }
  .tester .cf-svg rect:nth-of-type(n + 11) { animation-delay: 0.32s; }

  /* SECTIONS RISE INTO PLACE, in the order they are read. */
  .tester .tt-sec {
    animation: liftin 0.42s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .tester .tt-sec:nth-of-type(1) { animation-delay: 0.02s; }
  .tester .tt-sec:nth-of-type(2) { animation-delay: 0.08s; }
  .tester .tt-sec:nth-of-type(3) { animation-delay: 0.14s; }
  .tester .tt-sec:nth-of-type(4) { animation-delay: 0.2s; }
  @keyframes liftin {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  /* A STAT ARRIVES WITH ITS LABEL, staggered across the row so the eye
     is carried left to right rather than hit with four at once. */
  .tester .tt-quad > .tt-q {
    animation: liftin 0.4s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .tester .tt-quad > .tt-q:nth-child(1) { animation-delay: 0.04s; }
  .tester .tt-quad > .tt-q:nth-child(2) { animation-delay: 0.1s; }
  .tester .tt-quad > .tt-q:nth-child(3) { animation-delay: 0.16s; }
  .tester .tt-quad > .tt-q:nth-child(4) { animation-delay: 0.22s; }

  /* TABLE ROWS COME IN DOWN THE COLUMN. Capped at ten steps: the details
     table is twenty-four rows and the last must not wait a second. */
  .tester .tt-tbl tbody tr {
    animation: liftin 0.34s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .tester .tt-tbl tbody tr:nth-child(1) { animation-delay: 0.02s; }
  .tester .tt-tbl tbody tr:nth-child(2) { animation-delay: 0.045s; }
  .tester .tt-tbl tbody tr:nth-child(3) { animation-delay: 0.07s; }
  .tester .tt-tbl tbody tr:nth-child(4) { animation-delay: 0.095s; }
  .tester .tt-tbl tbody tr:nth-child(5) { animation-delay: 0.12s; }
  .tester .tt-tbl tbody tr:nth-child(6) { animation-delay: 0.145s; }
  .tester .tt-tbl tbody tr:nth-child(7) { animation-delay: 0.17s; }
  .tester .tt-tbl tbody tr:nth-child(8) { animation-delay: 0.195s; }
  .tester .tt-tbl tbody tr:nth-child(9) { animation-delay: 0.22s; }
  .tester .tt-tbl tbody tr:nth-child(n + 10) { animation-delay: 0.245s; }

  /* The horizontal bar rows sweep out from their own left edge, which is
     the axis they are measured from. */
  .tester .tt-brow,
  .tester .tt-plrow,
  .tester .tt-cmprow {
    animation: liftin 0.38s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .tester .tt-pl .tt-plrow:nth-child(2) { animation-delay: 0.06s; }
  .tester .tt-pl .tt-plrow:nth-child(3) { animation-delay: 0.12s; }

  /* The donut ring sweeps round once, from the top, then holds. */
  .tester .tt-donut circle {
    transform-origin: 50% 50%;
    animation: sweep 0.9s cubic-bezier(0.33, 0.8, 0.35, 1) both;
  }
  @keyframes sweep {
    from {
      stroke-dasharray: 0 400;
      transform: rotate(-90deg);
    }
    to {
      stroke-dasharray: 276 400;
      transform: rotate(-90deg);
    }
  }

  /* Controls respond, but only while the pointer is on them. */
  .tester .tt-pill,
  .tester .tt-segbtn,
  .tester .tt-view,
  .tester .tt-ctl {
    transition:
      background 0.16s ease,
      color 0.16s ease,
      border-color 0.16s ease;
  }
  .tester .tt-iconbtn {
    transition:
      background 0.16s ease,
      color 0.16s ease,
      transform 0.16s ease;
  }
  .tester .tt-iconbtn:hover {
    transform: translateY(-1px);
  }

  /* NOTHING MOVES FOR A READER WHO ASKED FOR STILLNESS, and a bar that
     would have grown from zero must end at its full height rather than
     at its starting one — `animation: none` on a `both`-filled keyframe
     leaves the element at its natural state, which is what is wanted. */
  @media (prefers-reduced-motion: reduce) {
    .tester .cf-svg path[stroke],
    .tester .cf-svg path[fill],
    .tester .cf-svg rect,
    .tester .tt-sec,
    .tester .tt-quad > .tt-q,
    .tester .tt-tbl tbody tr,
    .tester .tt-brow,
    .tester .tt-plrow,
    .tester .tt-cmprow,
    .tester .tt-donut circle {
      animation: none !important;
      opacity: 1 !important;
      transform: none !important;
      stroke-dasharray: none !important;
      stroke-dashoffset: 0 !important;
    }
  }

  /* ---- a tab pane arrives, it does not appear ----------------------
     `{#key}` remounts the pane on every switch, so this runs each time.
     It comes in from the LEFT because the pills it belongs to sit above
     and to the left, and a pane that slid the other way would read as
     going back. 18ms of stagger on the quad inside it carries the eye
     across the row after the pane itself has landed. */
  .bt-pane {
    animation: panein 0.34s cubic-bezier(0.22, 0.75, 0.3, 1) both;
  }
  @keyframes panein {
    from {
      opacity: 0;
      transform: translateX(-10px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  /* The pane's own stats re-stagger, because they are new numbers and
     not the same numbers moved. */
  .bt-pane .tt-quad > .tt-q {
    animation: liftin 0.36s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .bt-pane .tt-quad > .tt-q:nth-child(1) { animation-delay: 0.08s; }
  .bt-pane .tt-quad > .tt-q:nth-child(2) { animation-delay: 0.12s; }
  .bt-pane .tt-quad > .tt-q:nth-child(3) { animation-delay: 0.16s; }
  .bt-pane .tt-quad > .tt-q:nth-child(4) { animation-delay: 0.2s; }

  /* ---- the drill-down opens rather than appearing ------------------
     It is the largest thing on the page and it arrives under a row the
     operator just clicked, so it grows from that direction: down, and
     from slightly behind. */
  .drill {
    animation: drillopen 0.42s cubic-bezier(0.22, 0.75, 0.3, 1) both;
    transform-origin: top center;
  }
  @keyframes drillopen {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.994);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  /* ---- below the fold, a block waits until it is looked at ---------
     Everything above the fold animates on load, which is right; a block
     three screens down that has already finished animating by the time
     it is reached might as well not have. `animation-timeline: view()`
     is the declarative form of an IntersectionObserver and costs no
     JavaScript. Browsers without it simply show the block, which is the
     correct fallback: visible beats animated. */
  @supports (animation-timeline: view()) {
    @media (prefers-reduced-motion: no-preference) {
      /* `.drill-grid > .card` went the same way as `.bsub b` and for the same
         reason: 761934b removed the cards. `.rgroup` is what scroll-reveals
         now, and it keeps the whole rule. */
      .rgroup {
        animation: reveal 1ms linear both;
        animation-timeline: view();
        animation-range: entry 0% entry 40%;
      }
      @keyframes reveal {
        from {
          opacity: 0;
          transform: translateY(14px);
        }
        to {
          opacity: 1;
          transform: none;
        }
      }
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .bt-pane,
    .bt-pane .tt-quad > .tt-q,
    .drill {
      animation: none !important;
      opacity: 1 !important;
      transform: none !important;
    }
  }

  /* ---- drawn charts ---- */
  .cf-plot.tall {
    height: 250px;
  }
  .cf-svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
  }
  .cf-note {
    margin: 0.5rem 0 0;
    font-size: var(--fs-mini);
    color: var(--n8);
    max-width: 96ch;
  }

  /* ---- the chart frame ---- */
  .cf {
    margin-top: 0.55rem;
  }
  .cf-plot {
    position: relative;
    height: 190px;
    border: 1px solid var(--n6);
    border-radius: 8px;
    background: var(--n2);
    overflow: hidden;
  }
  .cf-grid {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  /* The axis labels sit where TradingView puts them -- right edge, three
     ticks around the zero line -- so the frame reads as a real plot. */
  .cf-axis {
    position: absolute;
    right: 0.55rem;
    top: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 0.35rem 0;
    font-size: var(--fs-micro);
    color: var(--n8);
    font-variant-numeric: tabular-nums;
  }
  .cf-msg {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    padding: 1rem 2.5rem;
    text-align: center;
    font-size: 0.77rem;
    color: var(--n9);
  }
  .cf-msg span {
    max-width: 72ch;
  }
  .cf-legend {
    list-style: none;
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    justify-content: center;
    margin: 0.5rem 0 0;
    padding: 0;
    font-size: var(--fs-mini);
    color: var(--n9);
  }
  .cf-legend li {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .cf-sw {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--n7);
  }
  .cf-sw.s0 {
    background: var(--up);
  }
  .cf-sw.s1 {
    background: var(--down);
  }
  .cf-sw.s2 {
    background: var(--acc);
  }
  .cf-sw.s3 {
    background: var(--warn);
  }

  /* ---- two-column analysis row ---- */
  .tt-two {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1.4rem;
    margin-top: 0.3rem;
  }

  /* ---- the donut ---- */
  .tt-donutwrap {
    position: relative;
    display: flex;
    align-items: center;
    gap: 1.2rem;
    flex-wrap: wrap;
    margin-top: 0.55rem;
  }
  .tt-donut {
    width: 132px;
    height: 132px;
    flex: none;
  }
  .tt-donutmid {
    position: absolute;
    left: 66px;
    top: 66px;
    transform: translate(-50%, -50%);
    text-align: center;
    pointer-events: none;
  }
  .tt-donutmid b {
    display: block;
    font-size: 1.1rem;
    color: var(--n12);
    font-variant-numeric: tabular-nums;
    line-height: 1.1;
  }
  .tt-donutmid span {
    font-size: var(--fs-micro);
    color: var(--n8);
  }
  .tt-donutleg {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    font-size: 0.76rem;
    color: var(--n10);
    min-width: 170px;
  }
  .tt-donutleg li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .tt-donutleg .sw {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex: none;
  }
  .tt-donutleg .sw.up {
    background: var(--up);
  }
  .tt-donutleg .sw.down {
    background: var(--down);
  }
  .tt-donutleg .sw.flat {
    background: var(--warn);
  }

  /* ---- the plot list: plain text, an eye, a collapse chevron ---- */
  .tt-plots {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .tt-plotrow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    font-size: 0.77rem;
    color: var(--n11);
    padding: 0.2rem 0.1rem;
  }
  .tt-plotrow.off {
    color: var(--n8);
  }
  .tt-eye {
    background: none;
    border: 0;
    padding: 0;
    line-height: 0;
    color: var(--acc);
    cursor: pointer;
  }
  .tt-eye svg {
    width: 14px;
    height: 14px;
  }
  .tt-collapse {
    align-self: flex-start;
    margin-top: 0.3rem;
    background: var(--n4);
    border: 0;
    border-radius: 5px;
    padding: 0.18rem 0.4rem;
    color: var(--n9);
    cursor: pointer;
    line-height: 0;
  }
  .tt-collapse svg {
    width: 13px;
    height: 13px;
  }
  .tt-collapse:hover {
    color: var(--n11);
  }

  /* ---- the two-value key stat ---- */
  .tt-frac {
    font-size: 0.82rem;
    font-style: normal;
    color: var(--n9);
    display: inline-flex;
    align-items: center;
    gap: 0.1rem;
  }

  /* ---- two-line table cells ---- */
  .tt-tbl td .tv {
    display: block;
    line-height: 1.25;
  }
  .tt-tbl td .tp {
    display: block;
    font-size: 0.7rem;
    color: var(--n8);
    line-height: 1.2;
  }

  /* ---- growth / decline comparison ---- */
  .tt-cmp {
    display: flex;
    flex-direction: column;
    gap: 0.32rem;
    margin-top: 0.5rem;
  }
  .tt-cmpgrp {
    font-size: var(--fs-mini);
    color: var(--n11);
    margin-top: 0.35rem;
  }
  .tt-cmpgrp:first-child {
    margin-top: 0;
  }
  .tt-cmprow {
    display: grid;
    grid-template-columns: 8ch 1fr auto;
    gap: 0.7rem;
    align-items: center;
  }
  .tt-cmplab {
    font-size: var(--fs-mini);
    color: var(--n8);
  }
  .tt-cmpbar {
    height: 11px;
    background: var(--n0);
    border-radius: 3px;
    overflow: hidden;
  }
  .tt-cmpfill {
    display: block;
    height: 100%;
    background: var(--up);
    animation: grow 0.5s cubic-bezier(0.22, 0.7, 0.3, 1) both;
    transform-origin: left;
  }
  .tt-cmpfill.down {
    background: var(--down);
  }
  .tt-cmpval {
    font-size: 0.76rem;
    color: var(--n10);
    font-variant-numeric: tabular-nums;
    min-width: 6ch;
    text-align: right;
  }
  .tt-cmpval.down {
    color: var(--down);
  }

  /* ---- chart frame: x axis and the pager ---- */
  .cf-x {
    display: flex;
    justify-content: space-between;
    padding: 0.32rem 0.6rem 0;
    font-size: var(--fs-micro);
    color: var(--n8);
    font-variant-numeric: tabular-nums;
  }
  .cf-page {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    z-index: 2;
    width: 20px;
    height: 26px;
    border: 1px solid var(--n6);
    border-radius: 5px;
    background: var(--n3);
    color: var(--n9);
    font-size: 0.8rem;
    line-height: 1;
    cursor: pointer;
  }
  .cf-page.l {
    left: 6px;
  }
  .cf-page.r {
    right: 6px;
  }
  .cf-page:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .cf-sw.dashed {
    border-radius: 0;
    height: 0;
    width: 12px;
    border-top: 2px dashed currentColor;
    background: none;
  }
  .cf-legend li.dash {
    color: var(--n8);
  }
  .cf-legend li b {
    color: var(--n11);
    font-weight: 600;
    margin-left: 0.2rem;
  }

  /* ---- donut legend, three columns ---- */
  .tt-donutleg li {
    display: grid;
    grid-template-columns: 10px 1fr auto auto;
    gap: 0.5rem;
    align-items: center;
  }
  .tt-donutleg .nm {
    color: var(--n10);
  }
  .tt-donutleg .ct,
  .tt-donutleg .pc {
    min-width: 4.5ch;
    text-align: right;
    color: var(--n9);
  }

  /* ---- list of trades: the two-row block ---- */
  .tt-tbl tr.lot-a td {
    border-bottom: 0;
    padding-bottom: 0.2rem;
  }
  .tt-tbl tr.lot-b td {
    padding-top: 0.2rem;
    border-bottom: 1px solid var(--n6);
  }
  /* The per-trade columns span both legs, so their divider is the block's. */
  .tt-tbl tr.lot-a td[rowspan] {
    border-bottom: 1px solid var(--n6);
    vertical-align: middle;
    padding-bottom: 0.46rem;
  }
  .tt-tbl tr.lot-a:hover,
  .tt-tbl tr.lot-b:hover {
    background: var(--n4);
  }
  .lot-num {
    color: var(--n11);
  }

  /* ==================================================================
     TRADINGVIEW'S OWN MEASUREMENTS, SCOPED TO THIS PANEL ONLY
     ------------------------------------------------------------------
     Read off the operator's screenshots rather than approximated. They
     are declared as LOCAL custom properties on `.tester`, so everything
     inside inherits them and NOTHING outside changes: `/db`, `/ingest`,
     `/audit` and the price terminal above keep the console's own
     palette. One panel imitates another product; the console does not.

     Why the console's tokens are not simply reused: they are close but
     not the same, and the two differences are the ones the eye catches
     first. The console's green is #00e19b, a neon that TradingView never
     uses; its red is #ff4d6a, which is pink beside TradingView's #F23645.
     Everything else here is within a few points and is matched anyway,
     because a panel that is 90% right reads as wrong rather than as
     nearly right.
     ================================================================== */
  .tester {
    --tv-bg: #131722;
    --tv-panel: #1e222d;
    --tv-line: #2a2e39;
    --tv-line-soft: #22262f;
    --tv-text: #d1d4dc;
    --tv-label: #b2b5be;
    --tv-muted: #787b86;
    --tv-green: #089981;
    --tv-red: #f23645;
    --tv-blue: #2962ff;
    --tv-teal: #26a69a;
    /* FOUR SURFACES THAT WERE WRITTEN AS LITERALS INSIDE THE RULES and so
       never flipped with the theme. This block's own banner says "nothing
       here declares a colour literal"; these are why that was not true.
       Measured on the light theme: `.tt-segbtn.on` put `--tv-text` --
       near-black ink -- on `#2f3241`, a contrast ratio of **1.47:1**, and it
       was the only failure below 3:1 on the whole page. The row hover was
       the same defect one interaction away: `#1c2030` under the same ink,
       invisible until a pointer landed on it, which is why a static sweep
       does not find it. */
    --tv-seg-on: #2f3241;
    --tv-row-hover: #1c2030;
    --tv-ghost: #434651;

    /* TradingView's own stack. `Trebuchet MS` is the one that gives their
       numerals their particular width; without it the tables read wider. */
    font-family: -apple-system, BlinkMacSystemFont, 'Trebuchet MS', Roboto, Ubuntu, sans-serif;
    background: var(--tv-bg);
    border-color: var(--tv-line);
    color: var(--tv-text);
  }
  /* A LIGHT-THEME READER GETS THE CONSOLE'S PALETTE, not a dark panel
     dropped into a light page. TradingView's tester is dark because
     TradingView is; ours has to survive both. */
  :root[data-theme='light'] .tester,
  :root:not([data-theme='dark']) .tester {
    --tv-bg: var(--n3);
    --tv-panel: var(--n2);
    --tv-line: var(--n6);
    --tv-line-soft: var(--n5);
    --tv-text: var(--n11);
    --tv-label: var(--n9);
    --tv-muted: var(--n8);
    --tv-green: #089981;
    --tv-red: #d1263a;
    /* A RAISED CHIP AND A HOVER ON A WHITE PANEL are a light step UP, not a
       dark one. `--n4` is the console's own panel-interior/hover surface, so
       the tester borrows the same one every other page uses. */
    --tv-seg-on: var(--n4);
    --tv-row-hover: var(--n4);
    --tv-ghost: var(--n7);
  }
  @media (prefers-color-scheme: dark) {
    :root:not([data-theme='light']) .tester {
      --tv-bg: #131722;
      --tv-panel: #1e222d;
      --tv-line: #2a2e39;
      --tv-line-soft: #22262f;
      --tv-text: #d1d4dc;
      --tv-label: #b2b5be;
      --tv-muted: #787b86;
      --tv-green: #089981;
      --tv-red: #f23645;
      --tv-seg-on: #2f3241;
      --tv-row-hover: #1c2030;
      --tv-ghost: #434651;
    }
  }

  .tester .tt-bar {
    background: var(--tv-bg);
    border-bottom-color: var(--tv-line);
    padding: 0.7rem 1rem;
  }
  .tester .tt-sec {
    padding: 1.35rem 1rem 1.5rem;
    border-bottom-color: var(--tv-line);
  }
  /* 15px, semibold — measured off "Key stats" and "Performance analysis". */
  .tester .tt-h {
    font-size: 15px;
    font-weight: 600;
    color: var(--tv-text);
    margin-bottom: 1.05rem;
  }
  .tester .tt-h5 {
    font-size: 14px;
    font-weight: 600;
    color: var(--tv-text);
    margin: 1.5rem 0 0.65rem;
  }
  /* THE QUAD IS FOUR EVEN COLUMNS ACROSS THE FULL WIDTH, not auto-fit
     boxes. TradingView spreads them regardless of content length, which
     is what makes the row scan as one line rather than four cards. */
  .tester .tt-quad,
  .tester .tt-stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 1.1rem 1.5rem;
  }
  @media (max-width: 860px) {
    .tester .tt-quad,
    .tester .tt-stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .tester .tt-k {
    font-size: 13px;
    color: var(--tv-label);
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
  }
  /* 20px and NOT BOLD. The weight is the detail most imitations get
     wrong; TradingView sets these at normal weight and lets size carry
     the hierarchy. */
  .tester .tt-qv.big {
    font-size: 20px;
    font-weight: 400;
    color: var(--tv-text);
    line-height: 1.35;
    gap: 0.4rem;
  }
  .tester .tt-qv {
    font-size: 14px;
    color: var(--tv-text);
  }
  .tester .tt-qv.up,
  .tester .tt-v.up {
    color: var(--tv-green);
  }
  .tester .tt-qv.down,
  .tester .tt-v.down {
    color: var(--tv-red);
  }
  /* The unit suffix: ~10px, uppercase, muted, tight against the number. */
  .tester .tt-unit {
    font-size: 10px;
    color: var(--tv-muted);
    letter-spacing: 0.02em;
    margin-left: 0.1rem;
  }
  .tester .tt-pc {
    font-size: 14px;
    color: inherit;
    opacity: 1;
  }
  .tester .tt-note {
    font-size: 12px;
    color: var(--tv-muted);
  }
  .tester .tt-note2 {
    font-size: 12px;
    color: var(--tv-muted);
    line-height: 1.55;
  }
  .tester .tt-note2 b {
    color: var(--tv-label);
  }

  /* ---- pills: TradingView's are 30px tall with a 15px radius ---- */
  .tester .tt-pills {
    gap: 0.5rem;
    margin-bottom: 1.15rem;
  }
  .tester .tt-pill {
    background: var(--tv-panel);
    border: 1px solid transparent;
    border-radius: 15px;
    padding: 0.4rem 0.85rem;
    font-size: 13px;
    color: var(--tv-label);
    height: 30px;
    line-height: 1;
    display: inline-flex;
    align-items: center;
  }
  .tester .tt-pill:hover {
    color: var(--tv-text);
  }
  .tester .tt-pill.on {
    background: transparent;
    border-color: var(--tv-blue);
    color: var(--tv-text);
  }
  .tester .tt-seg {
    background: var(--tv-panel);
    border-radius: 7px;
  }
  .tester .tt-segbtn {
    font-size: 13px;
    color: var(--tv-label);
    padding: 0.32rem 0.85rem;
  }
  .tester .tt-segbtn.on {
    background: var(--tv-seg-on);
    color: var(--tv-text);
    box-shadow: none;
  }

  /* ---- tables: 13px, and rows with room to breathe ---- */
  .tester .tt-tblwrap {
    border: 0;
    border-radius: 0;
    margin-top: 0;
  }
  .tester .tt-tbl {
    font-size: 13px;
  }
  .tester .tt-tbl th {
    background: transparent;
    color: var(--tv-muted);
    font-size: 13px;
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
    padding: 0.85rem 1rem;
    border-bottom: 1px solid var(--tv-line);
  }
  /* ~48px single-line, ~62px when a cell carries its percentage under
     the value. Measured off the details table in the screenshots. */
  .tester .tt-tbl td {
    padding: 0.95rem 1rem;
    border-bottom: 1px solid var(--tv-line-soft);
    color: var(--tv-text);
  }
  .tester .tt-tbl td:first-child {
    color: var(--tv-text);
  }
  .tester .tt-tbl tbody tr:hover {
    background: var(--tv-row-hover);
  }
  .tester .tt-tbl td .tp {
    font-size: 12px;
    color: var(--tv-muted);
    margin-top: 0.1rem;
  }
  .tester .tt-tbl td.down {
    color: var(--tv-red);
  }

  /* Every digit in this panel is tabular, so columns of numbers line up
     down the page the way they do in the screenshots. */
  .tester {
    font-variant-numeric: tabular-nums;
  }

  .tester .tt-chip {
    font-size: 12px;
    background: var(--tv-panel);
    color: var(--tv-label);
  }
  .tester .tt-chip.good {
    color: var(--tv-green);
  }
  .tester .tt-chip.warn {
    color: var(--warn);
  }
  .tester .tt-ctl,
  .tester .tt-view {
    font-size: 13px;
    color: var(--tv-label);
  }
  .tester .tt-view.on {
    background: var(--tv-blue);
    color: #fff;
  }
  .tester .tt-name b {
    font-size: 14px;
    color: var(--tv-text);
  }
  .tester .tt-plotrow {
    font-size: 13px;
    color: var(--tv-text);
  }
  .tester .tt-plotrow.off {
    color: var(--tv-muted);
  }
  .tester .cf-plot {
    border-color: var(--tv-line);
    background: transparent;
  }
  .tester .cf-axis,
  .tester .cf-x,
  .tester .cf-legend {
    font-size: 12px;
    color: var(--tv-muted);
  }
  .tester .cf-note {
    font-size: 12px;
    color: var(--tv-muted);
  }
  .tester .tt-blab,
  .tester .tt-pllab,
  .tester .tt-cmplab {
    font-size: 13px;
    color: var(--tv-label);
    text-transform: none;
    letter-spacing: 0;
  }
  .tester .tt-bval,
  .tester .tt-plval,
  .tester .tt-cmpval {
    font-size: 13px;
    color: var(--tv-text);
  }
  .tester .tt-bfill,
  .tester .tt-plfill,
  .tester .tt-cmpfill {
    background: var(--tv-green);
  }
  .tester .tt-bfill.down,
  .tester .tt-plfill.downbar,
  .tester .tt-cmpfill.down {
    background: var(--tv-red);
  }
  .tester .tt-plfill.ghost,
  .tester .tt-bfill.hold {
    background: var(--tv-ghost);
  }
  .tester .tt-donutmid b {
    font-size: 22px;
    color: var(--tv-text);
    font-weight: 400;
  }
  .tester .tt-donutmid span {
    font-size: 12px;
    color: var(--tv-muted);
  }
  .tester .tt-donutleg {
    font-size: 13px;
    color: var(--tv-text);
    gap: 0.7rem;
  }

  /* ---- sections ---- */
  .tt-sec {
    padding: 0.95rem 0.95rem 1.1rem;
    border-bottom: 1px solid var(--n5);
  }
  .tt-sec:last-child {
    border-bottom: 0;
  }
  .tt-h {
    margin: 0 0 0.7rem;
    font-size: 0.92rem;
    color: var(--n12);
    font-weight: 600;
  }
  .tt-h5 {
    margin: 1.1rem 0 0.4rem;
    font-size: 0.8rem;
    color: var(--n11);
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }
  /* A ROW THAT IS OURS SAYS SO. A metric nobody can find in the Strategy
     Tester should name where it came from rather than look like one they
     missed. */
  .tt-own {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
    background: var(--info-soft);
    color: var(--info);
    font-weight: 600;
  }
  .tt-note2 {
    margin: 0.5rem 0 0;
    font-size: 0.76rem;
    color: var(--n9);
    max-width: 92ch;
  }
  .tt-note2 b {
    color: var(--n11);
  }
  .tt-note2.badnote {
    color: var(--down);
  }
  .tt-note2.badnote b {
    color: var(--down);
  }

  /* ---- key stats & quads ---- */
  .tt-stats,
  .tt-quad {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.9rem 1.4rem;
  }
  .tt-stat,
  .tt-q {
    display: flex;
    flex-direction: column;
    gap: 0.16rem;
    min-width: 0;
  }
  .tt-k {
    font-size: var(--fs-mini);
    color: var(--n8);
  }
  .tt-v {
    font-size: 1.28rem;
    color: var(--n12);
    font-variant-numeric: tabular-nums;
    line-height: 1.15;
    display: flex;
    align-items: baseline;
    gap: 0.45rem;
    flex-wrap: wrap;
  }
  .tt-qv {
    font-size: 0.98rem;
    color: var(--n11);
    font-variant-numeric: tabular-nums;
  }
  .tt-v.up,
  .tt-qv.up {
    color: var(--up);
  }
  .tt-v.down,
  .tt-qv.down {
    color: var(--down);
  }
  .tt-sub {
    font-size: 0.82rem;
    font-style: normal;
    opacity: 0.85;
  }
  .tt-note {
    font-size: 0.7rem;
    color: var(--n8);
  }

  /* ---- the performance plot area ---- */
  .tt-perf {
    display: grid;
    grid-template-columns: minmax(180px, 220px) 1fr;
    gap: 1rem;
    align-items: start;
  }
  @media (max-width: 720px) {
    .tt-perf {
      grid-template-columns: 1fr;
    }
  }
  /* TradingView lists the plots down the LEFT of the chart, each one
     toggleable. Ours lists the same four and shows which are drawable. */
  .tt-plots {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .tt-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--n7);
    flex: none;
  }
  .tt-dot.on {
    background: var(--acc);
  }
  .tt-plot {
    min-width: 0;
  }
  .tt-plotnote {
    margin: 0.7rem 0 0;
    font-size: 0.75rem;
    color: var(--n9);
    max-width: 88ch;
  }
  .tt-plotnote b {
    color: var(--n11);
  }
  .tt-empty {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    min-height: 90px;
    padding: 0.9rem 1rem;
    border: 1px dashed var(--n6);
    border-radius: 8px;
    font-size: 0.78rem;
    color: var(--n9);
    background: var(--n2);
  }

  /* ---- benchmark & bar rows ---- */
  .tt-bench,
  .tt-pl {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }
  .tt-pl {
    margin-top: 0.55rem;
  }
  .tt-brow,
  .tt-plrow {
    display: grid;
    grid-template-columns: minmax(9ch, 22ch) 1fr auto;
    gap: 0.7rem;
    align-items: center;
  }
  .tt-blab,
  .tt-pllab {
    font-size: var(--fs-mini);
    color: var(--n8);
  }
  .tt-bbar,
  .tt-plbar {
    height: 12px;
    background: var(--n0);
    border-radius: 4px;
    overflow: hidden;
  }
  .tt-bfill,
  .tt-plfill {
    display: block;
    height: 100%;
    background: var(--up);
    animation: grow 0.5s cubic-bezier(0.22, 0.7, 0.3, 1) both;
    transform-origin: left;
  }
  .tt-bfill.hold,
  .tt-plfill.ghost {
    background: var(--n7);
  }
  .tt-bfill.down,
  .tt-plfill.downbar {
    background: var(--down);
  }
  .tt-plfill.warnbar {
    background: var(--warn);
  }
  .tt-bval,
  .tt-plval {
    font-size: 0.8rem;
    color: var(--n10);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .tt-bval.strong,
  .tt-plval.strong {
    color: var(--n12);
    font-weight: 600;
  }
  .tt-plval.warnt {
    color: var(--warn);
  }

  /* ---- pill tabs ---- */
  .tt-pills {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin-bottom: 0.85rem;
  }
  .tt-pill {
    background: var(--n4);
    border: 1px solid transparent;
    border-radius: 999px;
    padding: 0.3rem 0.75rem;
    font: inherit;
    font-size: 0.75rem;
    color: var(--n9);
    cursor: pointer;
    transition:
      background 0.14s ease,
      color 0.14s ease,
      border-color 0.14s ease;
  }
  .tt-pill:hover {
    color: var(--n11);
  }
  .tt-pill.on {
    background: var(--n2);
    border-color: var(--acc);
    color: var(--n12);
  }
  .tt-pill:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }

  /* ---- tables ---- */
  .tt-tblwrap {
    overflow-x: auto;
    margin-top: 0.6rem;
    border: 1px solid var(--n6);
    border-radius: 8px;
  }
  .tt-tbl {
    width: 100%;
    min-width: 520px;
    border-collapse: collapse;
    font-size: 0.79rem;
  }
  .tt-tbl th {
    text-align: left;
    background: var(--n2);
    color: var(--n8);
    font-size: var(--fs-micro);
    font-weight: 600;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--n6);
    white-space: nowrap;
  }
  .tt-tbl th.n,
  .tt-tbl td.n {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .tt-tbl td {
    padding: 0.46rem 0.75rem;
    border-bottom: 1px solid var(--n5);
    color: var(--n10);
  }
  .tt-tbl tbody tr:last-child td {
    border-bottom: 0;
  }
  .tt-tbl tbody tr:hover {
    background: var(--n4);
  }
  .tt-tbl td.down {
    color: var(--down);
  }
  .tt-tbl tr.own td:first-child {
    color: var(--n11);
  }
  .tt-tbl tr.warnrow td {
    color: var(--warn);
  }
  .tt-tbl tr.offrow td {
    color: var(--n8);
  }
  /* The trade list's header stays legible while its body is one locked
     row: the COLUMNS are what the operator is being told they cannot see. */
  .tt-tbl.ghosted th {
    color: var(--n8);
    opacity: 0.75;
  }
  .tt-tbl tr.lockrow:hover {
    background: transparent;
  }
  .tt-tbl tr.lockrow td {
    padding: 1.2rem 1rem;
  }

  /* ---- locked panels ---- */
  .tt-lockpanel {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    margin-top: 0.8rem;
    padding: 0.75rem 0.9rem;
    border: 1px dashed var(--n6);
    border-radius: 8px;
    background: var(--n2);
    font-size: 0.77rem;
    color: var(--n9);
    max-width: 96ch;
  }
  .tt-lockpanel b {
    color: var(--n11);
  }
  .tt-lockbig {
    display: flex;
    align-items: flex-start;
    gap: 0.85rem;
    max-width: 92ch;
  }
  .tt-lockbig b {
    color: var(--n11);
    font-size: 0.86rem;
  }
  .tt-lockbig p {
    margin: 0.4rem 0 0;
    font-size: 0.78rem;
    color: var(--n9);
  }
  .tt-fix {
    border-left: 2px solid var(--acc);
    padding-left: 0.7rem;
  }
  .tt-ident {
    margin: 0.6rem 0 0;
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .tt-ident code {
    font-size: var(--fs-mini);
    color: var(--n10);
    word-break: break-all;
  }

  /* ================= THE PRICE TERMINAL =================
     Full-bleed within the drill-down and taller than everything around it,
     because it is the only element here that rewards being looked AT rather
     than read. Everything else on this page is a number with a label. */
  .term {
    display: flex;
    flex-direction: column;
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--e2);
    animation: arrive 0.35s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .term > * {
    flex: 0 0 auto;
  }

  .term-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
    padding: 0.6rem 0.85rem;
    background: var(--n2);
    border-bottom: 1px solid var(--n6);
  }
  .term-id {
    display: flex;
    align-items: baseline;
    gap: 0.55rem;
    flex-wrap: wrap;
  }
  .term-id b {
    font-size: 1.05rem;
    color: var(--n12);
    letter-spacing: -0.01em;
  }
  .term-feed {
    font-size: var(--fs-mini);
    color: var(--n8);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  /* ---- the rung switcher ---- */
  .rungs {
    display: flex;
    gap: 2px;
    background: var(--n0);
    padding: 2px;
    border-radius: 7px;
    overflow-x: auto;
  }
  .rungbtn {
    background: transparent;
    border: 0;
    border-radius: 5px;
    padding: 0.26rem 0.5rem;
    font: inherit;
    font-size: var(--fs-mini);
    font-variant-numeric: tabular-nums;
    color: var(--n9);
    cursor: pointer;
    white-space: nowrap;
    transition:
      background 0.14s ease,
      color 0.14s ease;
  }
  .rungbtn:hover {
    color: var(--n11);
    background: var(--n4);
  }
  .rungbtn.on {
    background: var(--acc);
    color: var(--on-acc);
  }
  .rungbtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  /* A rung that carries a recorded run wears its tick. A rung that does not
     is still selectable -- looking at price the sweep never ran on is a
     legitimate thing to want, and hiding it would answer a question nobody
     asked. */
  .tick {
    margin-left: 0.22rem;
    font-size: 0.72em;
    opacity: 0.75;
  }
  .rungbtn.on .tick {
    opacity: 1;
  }

  /* ---- the OHLC legend ---- */
  .legend {
    display: flex;
    align-items: baseline;
    gap: 0.85rem;
    flex-wrap: wrap;
    padding: 0.5rem 0.85rem;
    border-bottom: 1px solid var(--n5);
    font-variant-numeric: tabular-nums;
    min-height: 2.1rem;
  }
  .lg-t {
    font-size: var(--fs-mini);
    color: var(--n8);
  }
  .lg {
    font-size: 0.79rem;
    color: var(--n11);
  }
  .lg i {
    font-style: normal;
    font-size: var(--fs-micro);
    letter-spacing: 0.09em;
    color: var(--n8);
    margin-right: 0.28rem;
  }
  .lg-chg {
    font-size: 0.79rem;
    font-weight: 600;
  }
  .lg-chg.up {
    color: var(--up);
  }
  .lg-chg.down {
    color: var(--down);
  }

  .term .bt-chart {
    height: 520px;
    flex: 0 0 520px;
    border: 0;
    border-radius: 0;
    background: transparent;
  }
  .term .chart-state {
    border: 0;
    border-radius: 0;
    min-height: 200px;
    background: transparent;
  }

  .term-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
    padding: 0.55rem 0.85rem;
    background: var(--n2);
    border-top: 1px solid var(--n6);
  }
  .term-facts {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .fnote {
    font-size: var(--fs-mini);
    color: var(--n8);
  }
  .ranges {
    display: flex;
    gap: 2px;
    background: var(--n0);
    padding: 2px;
    border-radius: 7px;
  }
  .rangebtn {
    background: transparent;
    border: 0;
    border-radius: 5px;
    padding: 0.24rem 0.52rem;
    font: inherit;
    font-size: var(--fs-mini);
    color: var(--n9);
    cursor: pointer;
    transition:
      background 0.14s ease,
      color 0.14s ease;
  }
  .rangebtn:hover {
    background: var(--n4);
    color: var(--n11);
  }
  .rangebtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  .term-note {
    padding: 0.6rem 0.85rem 0.75rem;
    margin: 0;
    border-top: 1px solid var(--n5);
  }

  /* ---------------- the sort indicator ---------------- */
  /* THE TABLE SORTED AND SAID NOTHING ABOUT IT. Nine sortable columns with no
     caret and no `aria-sort` means the operator has to click twice to learn
     which way it went, and a screen reader could not learn it at all. The
     caret is the sighted half, `aria-sort` on the `th` is the other. */
  .caret {
    display: inline-block;
    margin-left: 0.3rem;
    opacity: 0.45;
    font-size: 0.9em;
  }
  .sortbtn.active {
    color: var(--acc);
  }
  .sortbtn.active .caret {
    opacity: 1;
  }
  .nosort {
    opacity: 0.65;
  }

  /* ---------------- entrance, staggered ---------------- */
  /* ROWS ARRIVE IN SEQUENCE rather than all at once, so the eye is carried
     down the table in the order the table is sorted in. Capped at eight steps:
     past that the last row waits longer than a reader will, and a windowed
     table can hold hundreds. */
  .row:nth-child(1)  { animation-delay: 0ms; }
  .row:nth-child(2)  { animation-delay: 22ms; }
  .row:nth-child(3)  { animation-delay: 44ms; }
  .row:nth-child(4)  { animation-delay: 66ms; }
  .row:nth-child(5)  { animation-delay: 88ms; }
  .row:nth-child(6)  { animation-delay: 110ms; }
  .row:nth-child(7)  { animation-delay: 132ms; }
  .row:nth-child(n + 8) { animation-delay: 154ms; }

  /* THE COVERAGE BAR GROWS TO ITS WIDTH, because the width IS the fact — how
     much of the asked-for window is real. A bar that is simply present reads
     as decoration; one that arrives at a length reads as a measurement. */
  .cover-fill {
    animation: fill 0.5s cubic-bezier(0.22, 0.7, 0.3, 1) both;
    transform-origin: left;
  }
  @keyframes fill {
    from {
      transform: scaleX(0);
    }
    to {
      transform: none;
    }
  }

  /* THE ANSWER SETTLES IN, once, and then holds. A steady highlight, never a
     pulse: the crown is a standing fact about which run to believe, and
     anything that keeps moving reads as a value still changing. */
  .crown {
    animation: arrive 0.45s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .mult {
    animation: arrive 0.35s cubic-bezier(0.22, 0.7, 0.3, 1) both;
  }
  .mult.leader .mult-bar {
    box-shadow: 0 0 0 1px var(--acc);
  }
  /* The bars grow from the axis they are measured against. */
  .mult-bar,
  .mult-ghost {
    animation: rise 0.45s cubic-bezier(0.22, 0.7, 0.3, 1) both;
    transform-origin: bottom;
  }
  @keyframes rise {
    from {
      transform: scaleY(0);
    }
    to {
      transform: none;
    }
  }

  /* A live region has no box of its own. */
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }

  /* NOTHING MOVES FOR A READER WHO ASKED FOR STILLNESS. */
  /* NOTHING MOVES FOR A READER WHO ASKED FOR STILLNESS.
     Every animated selector on this page is listed, and `animation: none`
     rather than a zero duration — a zero-duration animation still applies its
     `from` state on some engines, which for `scaleX(0)` means a bar that is
     permanently invisible. Every bar here encodes a length that IS the fact,
     so a collapsed one is worse than an unanimated one. */
  @media (prefers-reduced-motion: reduce) {
    .row,
    .card,
    .crown,
    .mult,
    .mult-bar,
    .mult-ghost,
    .cover-fill,
    .efill,
    .tile,
    .spin {
      animation: none !important;
      transform: none !important;
    }
    .mult,
    .cover-fill,
    .btn,
    .row {
      transition: none;
    }
  }
</style>
