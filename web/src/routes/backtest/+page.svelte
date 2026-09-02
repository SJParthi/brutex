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
   * The filter is an infix probe into a Map built once per payload — the same
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
  import { untrack, tick } from 'svelte';
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
  import { placeIn } from '$lib/place.js';
  import { monthLabel } from '$lib/dates.js';
  import { rupee, group, exact } from '$lib/money.js';
  import {
    compareRuns,
    exactIntegerDelta,
    roundedScaledRatio,
    supportBasisPoints,
    supportRatioKey,
    validateLedgerPayload,
    validateRunForComputation
  } from '$lib/comparison.js';
  import { validateFrontierPayload } from '$lib/frontier-analytics.js';
  import { decodeMaskWords } from '$lib/mask.js';
  import { createRequestGate } from '$lib/request-gate.js';
  import { reduceLiveProgress } from '$lib/live-progress';
  import {
    TIME_GRAINS,
    equityMaxDrawdown,
    streakSeries,
    timePatternRows,
    validateTradePayload,
    windowSeries
  } from '$lib/trade-analytics.js';
  import * as find from '$lib/find.js';
  import { sweepOutcome, ledgerBlock } from '$lib/sweep.js';
  /* THE FEED'S OWN MASTER, ALREADY ON HAND. `+layout.svelte` calls
     `loadCatalogue(feeds.active)` on every feed change, so resolving a
     symbol's exchange and segment for a bar-file path costs this page NO
     request -- and resolving it is what stops `NSE`/`INDEX` being written
     here as literals the store can contradict. */
  import { catalogue } from '$lib/index.svelte.js';
  /* A neutral unavailable marker. The historical component name is retained,
     but its visible output is text/dash—not an access-control symbol. */
  import Lock from '$lib/Lock.svelte';

  /* ====================================================================
     THE PAYLOAD
     ==================================================================== */

  /** @type {{ phase: 'loading'|'ready'|'failed', body: any, why: string }} */
  let load = $state({ phase: 'loading', body: null, why: '' });
  /** Generation of the only ledger response still allowed to publish. */
  let ledgerSeq = 0;

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

  /**
   * The ranked combinations of the finished sweep — the thing the whole engine
   * exists to produce, and the one thing this page never asked for.
   *
   * `record_frontier` writes up to `rules.top` rows per rung into
   * `results/frontier.bin` — twenty-five per rung, two hundred for an eight-rung
   * press — each carrying the mask, hits, observations, mean, `t`, payoff and
   * win count. `GET /engine/top.json` reads them back and renders them WITH
   * condition names already resolved server-side.
   *
   * Every part of that worked. No page ever called the route, so the rows were
   * written correctly, read correctly by code nobody reached, and never seen.
   * The ledger table below shows one row per RUN — the single winning
   * combination — so the other twenty-four of each twenty-five have been
   * invisible since the frontier file was added.
   */
  let top = $state({ phase: 'idle', report: '', why: '' });

  /**
   * Live progress, taken from the event log because the status endpoint has none.
   *
   * `GET /backtest/run.json` reads a struct written exactly TWICE — once when a
   * sweep is accepted and once when it returns. It carries `in_flight` and a
   * start time and nothing else, so an eight-rung run over eighty-one months is
   * a boolean for its whole duration. Measured: two overnight runs each held
   * thirteen cores for seven hours, and there was no way from this page to tell
   * a working sweep from a hung one.
   *
   * The events carry what the struct does not — `stored span loaded` per rung
   * with its bar count and derived `min_hits`, and `rung finished` per rung with
   * its outcome and, on a refusal, the reason. `/logs.json` has served them all
   * along; nothing here ever asked.
   *
   * Polled on the same 2-second tick as the sweep status, so it costs one extra
   * request per tick and stops the moment the run does.
   */
  /**
   * ONE RUNG'S STATE, as `fetchLive` folds it out of `/logs.json`. `recorded`
   * is optional because it is only written when the rung FINISHES — a rung
   * still running has no answer for it, and a default of `false` would claim
   * one.
   *
   * `phase` is the coarse state the table renders, and it exists because
   * `done` was the only one: a rung four hours into its exit grid and one that
   * had just loaded its span were the same boolean. The grid is 87.6% of a
   * run's wall clock, so that covered the overwhelming majority of every run.
   * `loading` → `pricing` → `priced` → `done`.
   *
   * @typedef {{
   *   key: string, rung: string, bars: number, minHits: number, supportPpm: number,
   *   done: boolean, why: string, recorded?: boolean,
   *   validating: boolean, phase: string, candidates: number, priced: number
   * }} LiveRung
   */
  /* THE CAST IS INSIDE `$state(...)`, AND THAT PLACEMENT IS THE WHOLE FIX.
     A `@type` written ABOVE the declaration is ignored here, so `rungs: []`
     inferred as `never[]` and every downstream read of `.rung`, `.bars`,
     `.minHits`, `.done`, `.recorded` and `.why` became "does not exist on type
     'never'". One untyped empty array, fifteen errors. */
  let live = $state(
    /** @type {{ phase: string, attempt: number | null, rungs: LiveRung[], why: string }} */ ({
      phase: 'idle',
      attempt: null,
      rungs: [],
      why: ''
    })
  );

  /**
   * The best rows a RUNNING sweep has found so far, from `/live.json`.
   *
   * # The hole this fills
   *
   * `crates/cli/src/live.rs` has written this file since it was added — a
   * bounded min-heap of the best `keep` combinations by |t|, rewritten whenever
   * the top-N actually moves — and `/live.json` has served it. **No front-end
   * code ever fetched it.** So a sweep that runs for hours showed the operator
   * nothing but a spinner, and the one question worth asking during a run —
   * "has anything cleared the bar yet?" — had no surface at all.
   *
   * # Why the rows are not a finding
   *
   * `runner::rank` orders by |t| and explicitly does not bless: the bar a row
   * must clear is `bar_milli`, and each row carries its own `clears_bar`. A
   * table that showed rank alone would read as a result. Both are rendered.
   *
   * The cast sits INSIDE `$state(...)` for the reason the block above gives.
   */
  let liveTop = $state(
    /** @type {{ phase: string, rows: any[], trials: number, barMilli: number, stale: boolean, why: string }} */ ({
      phase: 'idle',
      rows: [],
      trials: 0,
      barMilli: 0,
      stale: false,
      why: ''
    })
  );

  /**
   * Read `/live.json` and keep the newest non-stale run's rows.
   *
   * Latest-request-wins is unnecessary here: the endpoint is a whole-file read
   * of at most `keep` rows and the poll interval is 2 s, so a late reply can
   * only carry an older heap of the same run — which is what the next tick
   * replaces anyway. A refusal is NAMED and never rendered as "no rows".
   */
  async function fetchLiveTop() {
    try {
      const response = await ask_('/live.json', { cache: 'no-store' });
      if (!response.ok) {
        liveTop = {
          ...liveTop,
          phase: 'failed',
          why: `/live.json answered ${response.status}. The heap is still being written to the store — this page could not read it back.`
        };
        return;
      }
      const body = await response.json();
      const runs = Array.isArray(body?.runs) ? body.runs : [];
      // THE FRESHEST RUN, and freshness is the file's own claim. `stale` is set
      // by the server when the heap has not moved for long enough that the run
      // is probably gone; picking a stale run over a live one would show the
      // PREVIOUS sweep's winners during this one, which is the exact defect
      // `live.rs` exists to remove.
      const best = runs.find((/** @type {any} */ r) => r && r.stale === false) ?? runs[0] ?? null;
      if (!best || !Array.isArray(best.rows)) {
        liveTop = { phase: 'empty', rows: [], trials: 0, barMilli: 0, stale: false, why: '' };
        return;
      }
      liveTop = {
        phase: 'ready',
        rows: best.rows.slice(0, 25),
        trials: Number(best.trials) || 0,
        barMilli: Number(best.bar_milli) || 0,
        stale: best.stale === true,
        why: ''
      };
    } catch (error) {
      liveTop = {
        ...liveTop,
        phase: 'failed',
        why: error instanceof Error ? error.message : String(error)
      };
    }
  }

  /**
   * One run's round trips — the rows the trade table below has been drawing
   * padlocks into since it was written.
   *
   * The table, its columns and its two-rows-per-trade shape were all built; the
   * file it reads was never written and no route ever served it. `cli::trades`
   * writes it now and `GET /trades.json?identity=…` serves it, keyed on the same
   * identity the ledger already prints on every row.
   *
   * Fetched when a run is OPENED rather than with the ledger: a run can hold
   * tens of thousands of trades and only one run is ever looked at.
   */
  /* THE CAST GOES INSIDE `$state(...)`, as it does for `live` and `board`.
     `rows: []` inferred as `never[]`, so `t.worst` and `{ ...t }` in
     `tradeRows` were both errors about a type nothing had declared.
     `any[]` IS THE HONEST ELEMENT TYPE HERE: these are `/trades.json`'s own
     rows and no type in this tree declares that payload. Naming a shape would
     be a claim about a document this file does not own — the same reason
     `BoardGroup.rows` is `any[]`. */
  let tradeList = $state(
    /** @type {{ phase: string, rows: any[], periods: any, policy: string|null, direction: string|null, why: string }} */ ({
      phase: 'idle',
      rows: [],
      periods: null,
      policy: null,
      direction: null,
      why: ''
    })
  );

  /**
   * One run's ranked combinations, with every measurement they were priced on.
   *
   * `/engine/top.json` serves a rendered table and this serves the numbers, for
   * the reason the route's own doc gives: nothing in a picture of a table can be
   * re-sorted, and the operator's ranking is a weighted ordering over eleven
   * quantities whose weights are theirs to move.
   */
  /* Same placement, same reason as `tradeList` above: inside `$state(...)`, and
     `any[]` because these are the ledger's own rows. */
  let combos = $state(
    /** @type {{ phase: string, rows: any[], rules: any, why: string }} */ ({
      phase: 'idle',
      rows: [],
      rules: null,
      why: ''
    })
  );

  // Independent request generations for the two drill-down detail streams.
  // A response belongs to the run that started it, not whichever run happens
  // to be open when the network finishes. These counters are the correctness
  // gates are the correctness mechanism; aborting a fetch would only be an
  // optimisation.
  const comboGate = createRequestGate();
  const tradeGate = createRequestGate();

  /**
   * How much each measurement counts toward the ranking.
   *
   * These are the operator's own words, one weight per clause:
   *
   *   "very less max drawdown, less max stop loss, less losing percentage,
   *    less losing trades, less losing ratio, and on the win side massive max
   *    profit, higher winning trades, higher winning percentage, higher winning
   *    ratio, average maximum profit, average less loss."
   *
   * They live here rather than in Rust deliberately. A score compiled into the
   * binary is one more number nobody can see — the failure this whole console
   * exists to avoid — and a weight the reader can move is a question they can
   * ask twice. Every entry is a plain multiplier: raise one and that measurement
   * matters more, set it to zero and it drops out of the ranking entirely.
   */
  let weights = $state({
    drawdown: 1, // less is better
    worstTrade: 1, // less is better  (the "max stop loss" clause)
    losingPct: 1, // less is better
    losingTrades: 1, // less is better
    lossRatio: 1, // less is better (gross loss over gross win — the 11th)
    profit: 1, // more is better
    winningTrades: 1, // more is better
    winRate: 1, // more is better
    rewardRisk: 1, // more is better
    avgWin: 1, // more is better
    avgLoss: 1 // less is better (avg_loss is negative, so nearer zero wins)
  });

  /**
   * THE SLIDER ROW, AND ITS KEYS ARE `weights`' OWN.
   *
   * These ten pairs were written inline in the `{#each}` that draws the
   * sliders, where the array's element type widened to `string[]` — so `key`
   * was a bare `string` indexing an object with ten literal keys, and both
   * `bind:value={weights[key]}` bindings resolved to `any`. A two-way binding
   * the compiler cannot name is the one place an `any` actually costs
   * something: a typo in a key would have bound a slider to a property that
   * does not exist, silently, and the ranking would have ignored it.
   *
   * `keyof typeof weights` rather than a repeated union, for the reason
   * `KNOB_FIELDS` gives: two copies of one list drift, and the drift is
   * invisible.
   *
   * @type {[keyof typeof weights, string][]}
   */
  const WEIGHT_FIELDS = [
    ['drawdown', 'less max drawdown'],
    ['worstTrade', 'less max stop loss'],
    ['losingPct', 'less losing %'],
    ['losingTrades', 'less losing trades'],
    ['lossRatio', 'less losing ratio'],
    ['profit', 'more max profit'],
    ['winningTrades', 'more winning trades'],
    ['winRate', 'higher win %'],
    ['rewardRisk', 'higher win:loss ratio'],
    ['avgWin', 'higher average win'],
    ['avgLoss', 'smaller average loss']
  ];

  /** How many rows to show per run. The operator asked for ten. */
  let topN = $state(10);

  /**
   * `topN` as a count that `Array.prototype.slice` can be trusted with.
   *
   * # What the bare binding did
   *
   * `min="1" max="100"` on a number input bounds the SPINNER ARROWS and nothing
   * else. The field is not inside a `<form>`, nothing calls `checkValidity`, and
   * Svelte's own coercion is `value === '' ? null : +value`. Measured:
   *
   * | typed | `topN` | `slice(0, n)` did |
   * |---|---|---|
   * | cleared | `null` | returned nothing, on every rung at once |
   * | `0` | `0` | the same |
   * | `-5` | `-5` | dropped the LAST five and showed the rest |
   * | `2.5` | `2.5` | truncated to two |
   * | `1e999` | `Infinity` | every row |
   *
   * The first two replaced all eight timeframes with *"No priced combination
   * was recorded for this rung"* — a sentence about the DATA, printed because of
   * a keystroke. The third is worse than useless: it is a top-N list that
   * silently drops the top rows.
   *
   * Clamped to `1..=100` — the bounds the input already claimed — with a
   * non-number reading as the operator's stated ten rather than as zero.
   */
  const topShown = $derived.by(() => {
    const n = Math.floor(Number(topN));
    if (!Number.isFinite(n)) return 10;
    return Math.min(100, Math.max(1, n));
  });

  /**
   * The engine knobs this page can set on a sweep, with what each does and what
   * the server falls back to when the field is left blank.
   *
   * # Why the defaults are printed rather than pre-filled
   *
   * A pre-filled field says "this is the value" and a blank one beside a stated
   * default says "the server decides, and here is what it will decide". Those
   * are different claims and only the second is true of an empty form.
   *
   * Pre-filling would also make every sweep name every knob, which puts fourteen
   * terms into the run identity that the operator never chose — two runs that
   * differ in nothing would then differ in everything.
   *
   * `default` is the SERVER's fallback, read from the Rust rather than guessed:
   * `screen_cap` is `DEFAULT: usize = 10_000`, `top` is `at("BRUTEX_TOP", 25)`,
   * and `validate` is `raw.is_none_or(|v| v.trim() != "0")` — unset means ON.
   */
  /**
   * `keyof typeof engine` AND NOT A REPEATED LIST OF NAMES. The knobs and the
   * state they write are declared side by side and nothing checked that they
   * agreed; spelling the fourteen names again here would be a second copy of
   * one fact, correct the day it is written and silently wrong the first time a
   * knob is added to one and forgotten in the other.
   *
   * Tying the key to `engine`'s own keys makes that mismatch a COMPILE ERROR
   * rather than an `any` — which is what both `engine[k.key]` sites had been
   * falling back to.
   *
   * @type {{ key: keyof typeof engine, label: string, fallback?: string, note?: string }[]}
   */
  const KNOB_FIELDS = [
    {
      key: 'support_ppm',
      label: 'support floor (ppm of this rung’s bars)',
      fallback: 'an affordability probe picks it — landed near 220,000 (22%)',
      note: 'The single biggest lever. 100000 is 10%, 50000 is 5%. Anything below the floor is never enumerated at all, so this decides what CAN be found, not just how long it takes.'
    },
    {
      key: 'screen_cap',
      label: 'combinations priced against the exit grid',
      fallback: '10,000',
      note: 'Of the ~430,000 enumerated per rung, this many get the 625-variant stop/target/trail grid. Cost scales with it.'
    },
    {
      key: 'top',
      label: 'combinations recorded to disk',
      fallback: '25',
      note: 'What the top-10 board below can rank. At 25 your eleven criteria only re-order a set the exit grid already picked.'
    },
    {
      key: 'validate',
      label: 'walk-forward, PBO and bootstrap',
      fallback: 'ON — unset means true',
      note: 'The expensive half. Off gives CANDIDATES, and the report says so in its own banner. On, with a high screen cap, is what does not finish.'
    },
    {
      key: 'ceiling',
      label: 'candidate ceiling',
      fallback: 'derived from this machine’s memory',
      note: 'Roughly the bytes you can spare divided by 146, and divided again among the rungs in flight.'
    },
    {
      key: 'grid_rungs',
      label: 'exit grid rungs',
      fallback: 'derived from the bar ranges',
      note: 'How many stop / target / trail levels the grid holds. Variants grow QUADRATICALLY in this.'
    },
    {
      key: 'grid_resolution',
      label: 'exit grid spacing',
      fallback: '20',
      note: 'The median bar range divided by this sets the ladder spacing.'
    },
    {
      key: 'sizing_rate_bp',
      label: 'sizing rate (bp)',
      fallback: '7,500',
      note: 'Must sit above the 5,000 bp coin-flip line and below 10,000.'
    },
    // MEASURED WRONG BY A FACTOR OF A HUNDRED. `Rules::operator` is
    // `at("BRUTEX_MIN_RR_BP", 125)` and 125 hundredths IS 1.25x -- the label
    // said "12,500 (1.25x)", so an operator reading it and typing 12,500 to
    // "keep the default" would have demanded a 125x reward-to-risk and screened
    // out everything. A placeholder is a claim about the server.
    { key: 'min_rr_bp', label: 'minimum reward:risk (bp)', fallback: '125 (1.25×)', note: '' },
    { key: 'min_win_rate_bp', label: 'minimum win rate (bp)', fallback: '5,000 (50%)', note: '' },
    { key: 'min_trades', label: 'minimum trades', fallback: '0', note: '' },
    // `0` CLAIMED THE RULE WAS OFF. It is `at("BRUTEX_MIN_RET_OVER_DD_BP", 500)`
    // -- a live 5x floor -- so the form told the operator a rule was disabled
    // while it was filtering every cell they got back.
    {
      key: 'min_ret_over_dd_bp',
      label: 'minimum return over drawdown (bp)',
      fallback: '500 (5×)',
      note: ''
    },
    { key: 'min_weakest_bp', label: 'minimum weakest fold (bp)', fallback: '0', note: '' },
    {
      key: 'max_mae_ppm',
      label: 'maximum adverse excursion (ppm)',
      fallback: '0, which drops the rule',
      note: 'The only rule where zero means OFF rather than a floor of zero — it is a `<=` test, so a literal zero would admit nothing.'
    }
  ];

  /** What the operator has typed. Blank means "leave it to the server". */
  let engine = $state({
    support_ppm: '',
    screen_cap: '',
    top: '',
    validate: '',
    ceiling: '',
    grid_rungs: '',
    grid_resolution: '',
    sizing_rate_bp: '',
    min_rr_bp: '',
    min_win_rate_bp: '',
    min_trades: '',
    min_ret_over_dd_bp: '',
    min_weakest_bp: '',
    max_mae_ppm: ''
  });

  /**
   * Only the knobs the operator actually filled in.
   *
   * A blank field is ABSENCE, not a value. Sending `"top": ""` would reach a
   * server that reads it as SET and then fails every parse of it into a default
   * — a silent fallback wearing a setting's clothes. `cli::knobs::set` makes the
   * same reading from the other side, so the two agree, but this skips the round
   * trip rather than relying on it.
   */
  const engineKnobs = $derived.by(() => {
    /** @type {Record<string, string>} */
    const out = {};
    for (const { key } of KNOB_FIELDS) {
      const typed = String(engine[key] ?? '').trim();
      if (typed !== '') out[key] = typed;
    }
    return out;
  });

  /** How many knobs this sweep will carry — shown beside the button. */
  const knobCount = $derived(Object.keys(engineKnobs).length);

  /** @param {string|undefined} identity */
  async function fetchCombos(identity) {
    const ticket = comboGate.begin(identity);
    if (!identity) {
      combos = { phase: 'idle', rows: [], rules: null, why: '' };
      return;
    }
    combos = { phase: 'loading', rows: [], rules: null, why: '' };
    try {
      const response = await ask_(`/frontier.json?identity=${encodeURIComponent(identity)}`);
      const body = await response.json();
      if (!comboGate.admits(ticket, openRun?.identity)) return;
      if (!response.ok) {
        combos = {
          phase: 'failed',
          rows: [],
          rules: null,
          why: body.refusal ?? `/frontier.json answered ${response.status}`
        };
        return;
      }
      const checked = validateFrontierPayload(body, identity);
      if (!checked.ok) {
        combos = {
          phase: 'failed',
          rows: [],
          rules: null,
          why: `Frontier analytics refused this answer: ${checked.why}`
        };
        return;
      }
      combos = {
        phase: 'ready',
        rows: checked.rows,
        // THE THRESHOLDS THIS RUN'S ROWS WERE JUDGED AGAINST, carried so a PASS
        // is never shown without the bar it cleared.
        rules: checked.rules,
        why: checked.why
      };
    } catch (why) {
      if (!comboGate.admits(ticket, openRun?.identity)) return;
      combos = {
        phase: 'failed',
        rows: [],
        rules: null,
        why: `The combinations could not be fetched: ${why instanceof Error ? why.message : String(why)}`
      };
    }
  }

  /**
   * The top N by the operator's weighted criteria.
   *
   * # Scored on RANK WITHIN THE SET, not on raw magnitude
   *
   * The eleven measurements are in different units — paisa, basis points, plain
   * counts — and adding them directly would let drawdown in paisa drown a win
   * rate in basis points purely because paisa are numerically larger. Each
   * measurement is therefore turned into its position among the rows being
   * compared, 0 for the worst and 1 for the best, and the weights combine those.
   * The ordering a weight produces is then about the measurement and not about
   * its unit.
   *
   * # Unpriced rows are excluded, and that is not a detail
   *
   * `screen_cap` means most ranked combinations never meet an exit grid, and
   * those store zeros. A zero drawdown is a SPECTACULAR result — so ranking
   * "smallest drawdown" over a set that includes them puts the combinations
   * nobody measured at the very top, looking perfect. `priced` is the field that
   * separates the two facts and this is where it earns its place.
   *
   * # One definition, two callers
   *
   * A plain function rather than a `$derived` body, because the SAME ordering
   * has to serve the open run's own frontier AND the top ten of every timeframe
   * at once. Written twice it would be two definitions of "best" that agree the
   * day they are written and disagree the first time a weight is added — the
   * failure mode this repository has hit five times in other files.
   *
   * @param {any[]} rows every combination recorded for one run
   * @param {any} w the operator's weights
   * @param {number} n how many to keep
   */
  /**
   * Share of round trips that lost. Less is better.
   *
   * A row with no trades is scored as wholly losing rather than as unmeasurable,
   * because an unpriced row is filtered out before this is ever reached — the
   * guard is here so the function is total, not because the branch is expected.
   *
   * @param {any} r
   */
  const losingPct = (r) => (r.trades > 0 ? r.losses / r.trades : 1);

  /**
   * THE ELEVENTH CRITERION, and it was the missing one.
   *
   * The operator's ranking names eleven quantities; this file carried ten. The
   * absentee is *"less losing ratio"*, which sits in the list directly after
   * *"less losing percentage"* and *"less losing trades"* — so it is a third,
   * distinct loss-side measurement and not a restatement of either.
   *
   * It is **total lost over total won** — the reciprocal of the profit factor.
   * `gross_loss` is negative, so the negation makes a positive ratio where less
   * is better: `0.5` means the winners paid for the losers twice over, `2.0`
   * means the reverse.
   *
   * # Why not `1 / reward_to_risk_bp`
   *
   * That was the tempting reading, and it is wrong: `reward_to_risk_bp` is
   * `min_win / -worst_trade`, two EXTREMES, so its reciprocal is perfectly
   * anti-correlated with the "winning ratio" slider. Adding it would have
   * doubled one criterion's weight while appearing to add a new one — a slider
   * that silently amplifies its neighbour is worse than the missing slider it
   * replaced.
   *
   * `gross_win` and `gross_loss` are the two sums `/frontier.json` began sending
   * when v3 stored them; before that this criterion was not computable at all.
   * `null` when nothing was won, because a ratio with no base is undefined —
   * never `0`, which `norm` would read as "measured, and best".
   *
   * @param {any} r
   */
  const lossRatio = (r) => (r.gross_win > 0 ? -r.gross_loss / r.gross_win : null);

  /**
   * Which measurements separate nothing across this set of rows.
   *
   * # Why the operator has to be told
   *
   * When every row agrees on a measurement, `norm` returns `0.5` for all of
   * them and the term becomes `w * 0.5` — an ADDITIVE CONSTANT, identical for
   * every row, which cannot change the sort no matter how far the slider moves.
   *
   * MEASURED, 2026-08-29: `avg_win` and `avg_loss` were `0` on all seventeen
   * rows in this operator's store, because v2 of the frontier did not store the
   * two sums they derive from. So two of the eleven sliders moved a visible
   * score number and changed the order of nothing, and the page said so
   * nowhere. A control that appears to work and does not is the failure
   * `CLAUDE.md` §4 bans — degrade loudly, or refuse, never both silently.
   *
   * Returns the KEYS of the collapsed criteria, so the slider row can mark them
   * rather than the page quietly rendering ten live controls.
   *
   * @param {any[]} rows
   * @returns {Set<string>}
   */
  function inertCriteria(rows) {
    const priced = rows.filter((r) => r.priced);
    const dead = new Set();
    if (priced.length < 2) return dead;
    /** @param {(row: any) => number|null} pick */
    const flat = (pick) => {
      let lo = Infinity;
      let hi = -Infinity;
      for (const r of priced) {
        const v = pick(r);
        if (v === null || v === undefined || !Number.isFinite(v)) continue;
        if (v < lo) lo = v;
        if (v > hi) hi = v;
      }
      return !Number.isFinite(lo) || lo === hi;
    };
    /** @type {[string, (row: any) => number|null][]} */
    const probes = [
      ['drawdown', (r) => r.max_drawdown],
      ['worstTrade', (r) => r.worst_trade],
      ['losingPct', losingPct],
      ['losingTrades', (r) => r.losses],
      ['lossRatio', lossRatio],
      ['profit', (r) => r.pessimistic],
      ['winningTrades', (r) => r.wins],
      ['winRate', (r) => r.win_rate_bp],
      ['rewardRisk', (r) => r.reward_to_risk_bp],
      ['avgWin', (r) => r.avg_win],
      ['avgLoss', (r) => r.avg_loss]
    ];
    for (const [key, pick] of probes) {
      if (flat(pick)) dead.add(key);
    }
    return dead;
  }

  /**
   * The operator's eleven criteria, weighted, applied to one run's rows.
   *
   * Each measurement is normalised to its own range ACROSS THIS SET and
   * multiplied by its weight, so the score answers *"best of these"* and never
   * *"good in absolute terms"* — which is why the PASS/FAIL verdict is rendered
   * beside it rather than folded into it. A score of 9.8 on a set where nothing
   * meets a single rule is still the best of a bad set.
   *
   * @param {any[]} rows the run's recorded combinations
   * @param {typeof weights} w the operator's current weights
   * @param {number} n how many to return, already clamped
   * @returns {any[]}
   */
  function rankRows(rows, w, n) {
    const priced = rows.filter((/** @type {any} */ r) => r.priced);
    if (priced.length === 0) return [];

    // One pass per measurement to learn its range, so the normalisation below is
    // O(rows) overall rather than O(rows) per row.
    /**
     * TYPING `pick` IS WHAT TYPES THE TEN CALLERS BELOW. Each `span((r) => ...)`
     * takes its parameter from this signature, so with `pick` untyped every one
     * of those `r`s was an implicit `any` — one missing annotation, ten errors,
     * and none of them at the line that caused it.
     *
     * `/**` AND NOT `/*`. The first attempt at this wrote the block as a plain
     * comment, which TypeScript does not read at all: the annotation was
     * present, correct, and invisible, and the ten errors did not move.
     *
     * `number|null` AND NOT `number`. `lossRatio` returns `null` when nothing
     * was won — a ratio with no base — and the body already skips every
     * non-finite value, so the narrower signature was a promise the callers
     * could not keep rather than a guarantee the body relied on.
     *
     * @param {(row: any) => number|null} pick
     * @returns {{ lo: number, hi: number }}
     */
    // `null` IS EXCLUDED FROM THE RANGE, NOT COERCED INTO IT.
    //
    // `/frontier.json` sends `null` for a ratio the data could not settle —
    // reward-to-risk on a combination that never lost a trade, return-over-
    // drawdown on one that never drew down. `Math.min(null, x)` is 0 and
    // `Number(null)` is 0, so leaving them in would put "could not be measured"
    // at the bottom of a range where 0 means "measured, and worst". That is the
    // same conflation the route stopped making when it stopped sending
    // `i64::MAX`, and undoing it here would make the fix pointless.
    const span = (pick) => {
      let lo = Infinity;
      let hi = -Infinity;
      for (const r of priced) {
        const v = pick(r);
        if (v === null || v === undefined || !Number.isFinite(v)) continue;
        if (v < lo) lo = v;
        if (v > hi) hi = v;
      }
      return { lo, hi };
    };
    // `hi === lo` means every row agrees on this measurement, so it separates
    // nothing and contributes 0.5 to all of them rather than dividing by zero.
    // An unmeasurable value takes the same 0.5: it is neither evidence for the
    // row nor against it, and any other number would be an opinion the data
    // does not support. `lo === Infinity` means NOTHING was measurable.
    /** @param {number|null|undefined} v @param {{ lo: number, hi: number }} range */
    const norm = (v, { lo, hi }) => {
      if (v === null || v === undefined || !Number.isFinite(v)) return 0.5;
      if (!Number.isFinite(lo) || hi === lo) return 0.5;
      return (v - lo) / (hi - lo);
    };

    const ranges = {
      drawdown: span((r) => r.max_drawdown),
      worstTrade: span((r) => r.worst_trade),
      losingPct: span(losingPct),
      losingTrades: span((r) => r.losses),
      lossRatio: span(lossRatio),
      profit: span((r) => r.pessimistic),
      winningTrades: span((r) => r.wins),
      winRate: span((r) => r.win_rate_bp),
      rewardRisk: span((r) => r.reward_to_risk_bp),
      avgWin: span((r) => r.avg_win),
      avgLoss: span((r) => r.avg_loss)
    };

    const scored = priced.map((r) => {
      // LESS IS BETTER for the first four, so their normalised value is
      // inverted. `worst_trade` and `avg_loss` are negative or zero, so a LARGER
      // value is already the better one and they are not inverted — reading them
      // as "less is better" on the raw number would rank the worst rows first.
      const score =
        w.drawdown * (1 - norm(r.max_drawdown, ranges.drawdown)) +
        w.worstTrade * norm(r.worst_trade, ranges.worstTrade) +
        w.losingPct * (1 - norm(losingPct(r), ranges.losingPct)) +
        w.losingTrades * (1 - norm(r.losses, ranges.losingTrades)) +
        w.lossRatio * (1 - norm(lossRatio(r), ranges.lossRatio)) +
        w.profit * norm(r.pessimistic, ranges.profit) +
        w.winningTrades * norm(r.wins, ranges.winningTrades) +
        w.winRate * norm(r.win_rate_bp, ranges.winRate) +
        w.rewardRisk * norm(r.reward_to_risk_bp, ranges.rewardRisk) +
        w.avgWin * norm(r.avg_win, ranges.avgWin) +
        w.avgLoss * norm(r.avg_loss, ranges.avgLoss);
      return { ...r, score };
    });

    // Sorted on score, ties broken by the sweep's own rank so the order is
    // total and two runs of the same data list the same rows in the same order.
    scored.sort((a, b) => b.score - a.score || a.rank - b.rank);
    return scored.slice(0, n);
  }

  const ranked = $derived.by(() => rankRows(combos.rows, weights, topShown));

  /**
   * Every comparable question's timeframe frontier at once, so “top ten per
   * timeframe” never collapses feeds, instruments, spans, or support ratios.
   *
   * # Why the NEWEST run per rung and not every run
   *
   * The ledger is append-only, so one rung swept three times holds three rows,
   * and stacking their combinations into one ranking would compare a 22%-support
   * search against a 10% one as though they answered the same question. They do
   * not: support decides which combinations were ENUMERATED AT ALL. One run per
   * comparable question and rung, the newest, and every question dimension is
   * printed beside it.
   */
  /**
   * ONE RUNG'S BOARD ROW. `run` and `rows` are the ledger's own JSON and are
   * `any` deliberately: nothing in this tree declares that shape, and inventing
   * a type for it here would be a claim about a payload this file does not own.
   *
   * `rules` is the threshold set `/frontier.json` judged this rung's rows
   * against, echoed so a PASS is shown beside the bar it cleared. `null` when
   * the route refused before it could read them.
   *
   * `key` is the whole comparison question plus the rung. A rung alone is not
   * a question: NIFTY and BANKNIFTY at 5min, or the same instrument over two
   * spans, must never audition for the same slot.
   *
   * @typedef {{ key: string, question: string, rung: string, run: any, rows: any[], rules: any, admitted: number, why: string }} BoardGroup
   */
  /* INSIDE `$state(...)`, for the same reason `live` is — see its comment. */
  let board = $state(
    /** @type {{ phase: string, groups: BoardGroup[], why: string }} */ ({
      phase: 'idle',
      groups: [],
      why: ''
    })
  );

  /**
   * The immutable parts of one comparison question. The support ratio is in
   * the key because it decides which combinations were eligible to exist at
   * all. Unit separators make this an unambiguous machine key even if a future
   * feed or instrument name contains spaces.
   *
  * @param {any} r
  */
 function comparisonQuestionKey(r) {
    const supportRatio = supportRatioKey(r);
    return [
      r.feed,
      r.underlying,
      `${r.from_year}-${r.from_month}`,
      `${r.to_year}-${r.to_month}`,
      supportRatio === null ? 'support-refused' : `support-ratio:${supportRatio}`
    ].join('\u001f');
  }

  /** @param {any[]} rowsIn the ledger rows currently in view */
  async function fetchBoard(rowsIn) {
    // Invalidate every older batch BEFORE the empty fast path. Otherwise an
    // in-flight prior feed keeps the current ticket and can republish after the
    // scope becomes empty.
    boardSeq += 1;
    const mine = boardSeq;
    // O(1) PER ROW AND ONE PASS. A Map keyed by the WHOLE comparison question
    // plus rung keeps the newest comparable answer. Keying only by rung made a
    // newer BANKNIFTY 5min run erase NIFTY 5min, and made feed/span/support
    // changes race for the same slot. Those are different questions, not
    // reruns. The same technique `/db` uses keeps the fold O(1) per row.
    const newest = new Map();
    for (const r of rowsIn) {
      const key = `${comparisonQuestionKey(r)}\u001f${r.timeframe}`;
      const had = newest.get(key);
      if (!had || r.finished_micros > had.finished_micros) newest.set(key, r);
    }
    if (newest.size === 0) {
      board = { phase: 'ready', groups: [], why: 'No completed run is in view.' };
      return;
    }
    // ONE SEQUENCE NUMBER, THE SHAPE THE OTHER THREE LOADERS ON THIS PAGE
    // ALREADY USE. `loadSeries`, `loadRungs` and the benchmark load each guard
    // their write with a counter; this one did not, and it is the loader most
    // likely to race — `runs` is a filtered derived, so switching feed rebuilds
    // the ARRAY IDENTITY and refires the effect even when the contents are
    // unchanged. Two batches in flight, last writer wins, and the loser was
    // whichever the network happened to favour. The phase still read `ready`,
    // so a board showing the previous feed's numbers was indistinguishable from
    // a correct one.
    board = { phase: 'loading', groups: [], why: '' };

    // FETCHED IN PARALLEL because the eight are independent: eight sequential
    // round trips would make the board eight times slower for no reason.
    //
    // `allSettled` AND NOT `all`. `Promise.all` rejects on the FIRST failure and
    // discards every sibling result, so one rung answering 502 replaced all
    // eight tables with a single sentence naming one URL. A rung that fails now
    // fails alone and says so in its own section.
    const selected = [...newest.values()];
    const settled = await Promise.allSettled(
      selected.map(async (run) => {
        const question = comparisonQuestionKey(run);
        const key = `${question}\u001f${run.timeframe}`;
        const response = await ask_(`/frontier.json?identity=${encodeURIComponent(run.identity)}`);
        // `ok` IS CHECKED BEFORE THE BODY IS PARSED. Reading `.json()` first
        // turned a clean 502 with an HTML body into `Unexpected token '<'`,
        // which names the parser instead of the failure.
        if (!response.ok && response.status !== 200) {
          return {
            key,
            question,
            rung: run.timeframe,
            run,
            rows: [],
            rules: null,
            admitted: 0,
            why: `/frontier.json answered ${response.status} for ${run.timeframe}`
          };
        }
        const body = await response.json();
        const checked = validateFrontierPayload(body, run.identity);
        if (!checked.ok) {
          return {
            key,
            question,
            rung: run.timeframe,
            run,
            rows: [],
            rules: null,
            admitted: 0,
            why: `Frontier analytics refused ${run.timeframe}: ${checked.why}`
          };
        }
        return {
          key,
          question,
          rung: run.timeframe,
          run,
          rows: checked.rows,
          rules: checked.rules,
          admitted: checked.admitted,
          why: checked.why
        };
      })
    );
    if (mine !== boardSeq) return; // a newer board started; this answer is stale

    const groups = settled.map((s, i) =>
      s.status === 'fulfilled'
        ? s.value
        : {
            key: selected[i]
              ? `${comparisonQuestionKey(selected[i])}\u001f${selected[i].timeframe}`
              : `missing-${i}`,
            question: selected[i] ? comparisonQuestionKey(selected[i]) : '',
            rung: selected[i]?.timeframe ?? '?',
            run: selected[i],
            rows: [],
            rules: null,
            admitted: 0,
            why: `This rung could not be fetched: ${
              s.reason instanceof Error ? s.reason.message : String(s.reason)
            }`
          }
    );
    // Questions are kept together, then ordered by the rung's own minutes, so
    // the board reads one feed/instrument/span/support comparison at a time.
    groups.sort(
      (a, b) =>
        a.question.localeCompare(b.question) || minutesOf(a.rung) - minutesOf(b.rung)
    );
    const allFailed = groups.every((g) => g.rows.length === 0 && g.why);
    board = {
      phase: 'ready',
      groups,
      why: allFailed ? 'No rung answered with rows. Each section names its own reason.' : ''
    };
  }

  /** @param {string} rung e.g. "15min" */
  function minutesOf(rung) {
    const digits = String(rung).match(/^\d+/);
    return digits ? Number(digits[0]) : Number.MAX_SAFE_INTEGER;
  }

  /**
   * One monotonic counter for the board's fetches, so a slow batch cannot
   * overwrite a fast one. Module-scope rather than `$state` because nothing
   * renders it — writing it must not invalidate anything.
   */
  let boardSeq = 0;

  const board10 = $derived.by(() =>
    board.groups.map((g) => {
      const top = rankRows(g.rows, weights, topShown);
      return {
        ...g,
        priced: g.rows.filter((r) => r.priced).length,
        top,
        // COUNTED OVER THE ROWS ON SCREEN, not over the whole response. The
        // envelope's `admitted` counts every recorded row; the sentence beneath
        // the table is about the ones the operator can see, and quoting the
        // larger number under a shorter table would be the wrong answer to
        // "how many of these pass".
        admittedShown: top.filter((r) => r.meets?.all).length,
        inert: inertCriteria(g.rows)
      };
    })
  );

  /**
   * The criteria that separate nothing anywhere on the board.
   *
   * A slider is called dead only when EVERY rung with rows agrees on that
   * measurement. Inert on one rung and live on another is a real difference
   * between timeframes, not a broken control, and greying it would hide the
   * finding rather than report it.
   */
  const inertAll = $derived.by(() => {
    const withRows = board10.filter((g) => g.rows.length > 0);
    if (withRows.length === 0) return new Set();
    /** @type {Set<string>} */
    const dead = new Set(withRows[0].inert);
    for (const g of withRows.slice(1)) {
      for (const key of [...dead]) if (!g.inert.has(key)) dead.delete(key);
    }
    return dead;
  });

  // FETCHED WHEN THE LEDGER CHANGES, NOT WHEN A WEIGHT MOVES. The effect reads
  // `runs` and nothing else, so dragging a weight re-ranks in the browser
  // without asking the server for the same rows again — `board10` is derived and
  // recomputes on its own. Re-ranking is the cheap half and refetching is the
  // expensive one; tying them together would have made every slider a round trip.
  $effect(() => {
    // The board is a ranking surface. It receives the same exact-signal,
    // sealed, whole-span, non-halted, priced-result admission as the headline,
    // never merely every structurally parseable ledger row.
    void fetchBoard(rankableRuns);
  });

  /** @param {string|undefined} identity */
  async function fetchTrades(identity) {
    const ticket = tradeGate.begin(identity);
    if (!identity) {
      tradeList = {
        phase: 'idle',
        rows: [],
        periods: null,
        policy: null,
        direction: null,
        why: ''
      };
      return;
    }
    // Snapshot the ledger parent this request must reconcile with. The request
    // gate prevents publication after `openRun` changes; this value prevents a
    // structurally valid child from being labelled by the wrong parent.
    const expectedRun = openRun?.identity === identity ? openRun : undefined;
    if (!expectedRun) {
      tradeList = {
        phase: 'failed',
        rows: [],
        periods: null,
        policy: null,
        direction: null,
        why: 'Trade analytics refused an unbound request: no open ledger run owns this identity.'
      };
      return;
    }
    tradeList = {
      phase: 'loading',
      rows: [],
      periods: null,
      policy: null,
      direction: null,
      why: ''
    };
    try {
      const response = await ask_(`/trades.json?identity=${encodeURIComponent(identity)}`);
      const body = await response.json();
      if (!tradeGate.admits(ticket, openRun?.identity)) return;
      if (!response.ok) {
        tradeList = {
          phase: 'failed',
          rows: [],
          periods: null,
          policy: null,
          direction: null,
          why: body.refusal ?? `/trades.json answered ${response.status}`
        };
        return;
      }
      const checked = validateTradePayload(body, expectedRun);
      if (!checked.ok) {
        tradeList = {
          phase: 'failed',
          rows: [],
          periods: null,
          policy: null,
          direction: null,
          why: `Trade analytics refused the response: ${checked.why}`
        };
        return;
      }
      // THE ROUTE ANSWERS 200 WITH AN EMPTY LIST AND A REASON when the file is
      // absent, so an empty store and a broken server never look alike. Both
      // are carried: the rows if any, the reason if any.
      tradeList = {
        phase: 'ready',
        rows: checked.rows,
        // THE PERIOD BUCKETS WERE BEING DROPPED. `/trades.json` sends eight of
        // them -- day, week, month, quarter, half, year, weekday, hour -- each
        // carrying `trades`, `wins`, `worst_wins`, `largest_win` and
        // `largest_loss`. Four metrics this page renders as "not recorded"
        // are sums of that object.
        periods: checked.periods,
        policy: checked.policy,
        direction: checked.direction,
        why: body.refusal ?? ''
      };
    } catch (why) {
      if (!tradeGate.admits(ticket, openRun?.identity)) return;
      tradeList = {
        phase: 'failed',
        rows: [],
        periods: null,
        policy: null,
        direction: null,
        why: `The trades could not be fetched: ${why instanceof Error ? why.message : String(why)}`
      };
    }
  }

  /**
   * Everything the reference's tester reports, from the per-trade results.
   *
   * # `best` and `worst` are REALISED P&L, not excursions
   *
   * This is the correction the rest of this block hangs on, and it is stated in
   * `crates/cli/src/trades.rs` in as many words:
   *
   * > `best` — *"Paisa per unit with both legs filled at the bar OPEN — the
   * > BEST-CASE realised P&L of this round trip, **not an excursion**."*
   * > `worst` — *"Paisa per unit with both legs filled at the bar PRINTED
   * > EXTREME — the WORST-CASE realised P&L, and the figure selection actually
   * > ranks on."*
   *
   * That doc carries its own correction notice — it once said *"the most
   * favourable excursion reached"*, and warns that a reader trusting it *"would
   * have filtered on `worst <= 0` and found nothing, or read an excursion where
   * a result was."* **This page was that reader.** It rendered the two fields
   * under `Favorable excursion` / `Adverse excursion`, locked `Net PnL` with
   * *"Realised net P&L per trade is not stored"*, and summed `worst` as an
   * adverse excursion.
   *
   * It never held together: `/trades.json`'s buckets count `wins` and
   * `worst_wins` off these same two fields, and a winner cannot be counted from
   * an excursion.
   *
   * # Worst-case is the headline throughout
   *
   * The page's own header reads *"ranked on worst-case fills"*, and `worst` is
   * the figure the engine selects on. Every figure below is therefore computed
   * on `worst`, with the best-case reading carried beside it rather than
   * substituted for it.
   */
  const tradeStats = $derived.by(() => {
    const rows = tradeList.rows;
    if (!Array.isArray(rows) || rows.length === 0) return null;

    let grossProfit = 0;
    let grossLoss = 0; // kept POSITIVE, as the reference prints it
    let wins = 0;
    let losses = 0;
    let breakevens = 0;
    let barsWin = 0;
    let barsLoss = 0;
    let largestWin = 0;
    let largestLoss = 0; // positive magnitude
    let bars = 0;
    let bestNet = 0;
    // Streaks need the ORDER trades resolved in, which is `seq`, so the list is
    // walked in sequence rather than in display order. The pure fold retains
    // every run for the chart as well as its four summary numbers.
    const streak = streakSeries(rows);
    const ordered = streak.ordered;

    for (const t of ordered) {
      const net = t.worst ?? 0;
      bars += t.bars_held ?? 0;
      bestNet += t.best ?? 0;
      if (net > 0) {
        wins += 1;
        grossProfit += net;
        barsWin += t.bars_held ?? 0;
        if (net > largestWin) largestWin = net;
      } else if (net < 0) {
        losses += 1;
        grossLoss += -net;
        barsLoss += t.bars_held ?? 0;
        if (-net > largestLoss) largestLoss = -net;
      } else {
        // A FLAT TRADE IS ITS OWN OUTCOME. The reference counts `Breakevens` as
        // a third slice of the donut, and folding them into losers would move
        // the win rate without any trade having lost anything.
        breakevens += 1;
      }
    }

    const n = ordered.length;
    const net = grossProfit - grossLoss;
    return {
      trades: n,
      wins,
      losses,
      breakevens,
      net,
      bestNet,
      grossProfit,
      grossLoss,
      // A RATIO WITH NO DENOMINATOR IS `null`, NEVER `0` and never `Infinity`.
      // A run that never lost has no profit factor to quote — that is not a
      // profit factor of zero, and it is not an infinitely good one.
      profitFactor: grossLoss > 0 ? grossProfit / grossLoss : null,
      rateBp: n ? Math.round((wins / n) * 10_000) : 0,
      avgNet: Math.round(net / n),
      avgWin: wins ? Math.round(grossProfit / wins) : null,
      avgLoss: losses ? Math.round(grossLoss / losses) : null,
      winLossRatio: wins && losses ? grossProfit / wins / (grossLoss / losses) : null,
      largestWin: largestWin || null,
      largestLoss: largestLoss || null,
      avgBars: Math.round(bars / n),
      avgBarsWin: wins ? Math.round(barsWin / wins) : null,
      avgBarsLoss: losses ? Math.round(barsLoss / losses) : null,
      longestWin: streak.longestWin,
      longestLoss: streak.longestLoss,
      avgWinStreak: streak.avgWinStreak,
      avgLossStreak: streak.avgLossStreak,
      streaks: streak.streaks,
      ordered,
      // The equity curve the `Performance` plot list has been calling
      // unrecordable: a running sum of realised worst-case results.
      curve: (() => {
        let run = 0;
        return ordered.map((t) => {
          run += t.worst ?? 0;
          return run;
        });
      })()
    };
  });

  /** Which performance plot is drawn. `equity` is the strategy's own curve. */
  let plot = $state('equity');
  let performancePage = $state(0);
  const PERFORMANCE_PAGE_SIZE = 320;
  /** A bounded window for dense alternating strategy or benchmark swings. */
  let swingPage = $state(0);
  const SWING_PAGE_SIZE = 96;

  /**
   * The equity curve, and everything that is a property of its SHAPE.
   *
   * # What this unlocks, and why it was locked
   *
   * Nine rows across three tabs carried a padlock reading some form of *"needs
   * an equity series"* — `Sharpe ratio`, `Sortino ratio`, `Average run-up
   * duration`, `Average drawdown duration`, the three `Run-up` rows, `Drawdown
   * Average`, and the `Cumulative PnL` plot. Every one of those sentences was
   * true of the RESULTS LEDGER, which keeps four scalars per run and no series.
   *
   * The trade file is a series. `worst` is each round trip's realised worst-case
   * result in `seq` order, so the running sum IS the curve and one walk of it
   * yields every shape statistic above.
   *
   * # Annualisation, and the assumption under it
   *
   * Sharpe and Sortino are ratios per unit of time, so they need a period count:
   * trades over the span's length in years. **The risk-free rate is taken as
   * ZERO**, and the page says so rather than leaving it here — a Sharpe quoted
   * without its convention is how two correct numbers end up disagreeing.
   */
  const equity = $derived.by(() => {
    if (!tradeStats || tradeStats.curve.length < 2) return null;
    const curve = tradeStats.curve;
    const rows = [...tradeList.rows].sort((a, b) => (a.seq ?? 0) - (b.seq ?? 0));

    // ── MAX DRAWDOWN: underwater from the running peak ──
    // The standard definition, and the one the ledger uses: the deepest fall
    // from any high to any subsequent low. Kept separate from the segmentation
    // below because it answers a different question — "what is the worst this
    // ever got" rather than "how long is a typical decline".
    const maxDrawdown = equityMaxDrawdown(curve);
    if (maxDrawdown === null) return null;

    // ── RUN-UPS AND DRAWDOWNS: alternating SWINGS, not new highs ──
    //
    // MEASURED, and the first attempt at this was wrong. Segmenting on a NEW
    // ALL-TIME HIGH finds nothing on a curve that never makes one: this
    // operator's 2,101-trade run peaks at −Rs 10.85 and bottoms at −Rs 20,949.95,
    // so it is underwater from the first trade to the last. Under that rule the
    // run reported ZERO run-ups, ZERO closed drawdowns, and a padlock on five
    // rows that had just been unlocked.
    //
    // The reference's own heading says which rule it means: "Alternating growth
    // and decline". A run-up is a RISING STRETCH and a drawdown a FALLING one,
    // bounded by the local turns between them — which exist on any curve that
    // is not monotonic. The same run yields 476 run-ups and 477 drawdowns.
    //
    // This is also the rule `holdSwings` already uses for the benchmark chart on
    // this page, so the two now segment alike rather than two ways.
    /** @type {number[]} */ const runUps = [];
    /** @type {number[]} */ const runUpLens = [];
    /** @type {number[]} */ const drawdowns = [];
    /** @type {number[]} */ const drawdownLens = [];
    /** @type {Array<{up: boolean, size: number, current?: boolean}>} */ const swingSegments = [];
    let dir = 0;
    let anchor = curve[0];
    let anchorAt = 0;
    for (let i = 1; i < curve.length; i += 1) {
      const up = curve[i] > curve[i - 1];
      const down = curve[i] < curve[i - 1];
      if (dir === 0) {
        dir = up ? 1 : down ? -1 : 0;
        continue;
      }
      // A flat step is not a turn — it extends whichever stretch is open.
      if ((dir === 1 && down) || (dir === -1 && up)) {
        const magnitude = Math.abs(curve[i - 1] - anchor);
        const length = i - 1 - anchorAt;
        if (dir === 1) {
          runUps.push(magnitude);
          runUpLens.push(length);
        } else {
          drawdowns.push(magnitude);
          drawdownLens.push(length);
        }
        if (magnitude > 0) swingSegments.push({ up: dir === 1, size: magnitude });
        anchor = curve[i - 1];
        anchorAt = i - 1;
        dir = up ? 1 : -1;
      }
    }
    const mean = (/** @type {number[]} */ xs) =>
      xs.length === 0 ? null : xs.reduce((a, b) => a + b, 0) / xs.length;
    // REDUCE AND NOT `Math.max(...xs)`. The spread pushes every element onto the
    // argument stack, and this operator has just re-pulled the whole store — a
    // sweep over seven years of one-minute bars can record tens of thousands of
    // trades, where the spread throws `Maximum call stack size exceeded` and
    // takes the whole drill-in down. A reduce is O(n) either way and has no
    // such ceiling.
    const maxOf = (/** @type {number[]} */ xs) =>
      xs.reduce((a, b) => (b > a ? b : a), Number.NEGATIVE_INFINITY);
    const minOf = (/** @type {number[]} */ xs) =>
      xs.reduce((a, b) => (b < a ? b : a), Number.POSITIVE_INFINITY);
    const maxRunUp = runUps.length > 0 ? maxOf(runUps) : 0;

    // THE OPEN STRETCH IS SIGNED, AND ITS SIGN IS THE WHOLE POINT.
    //
    // The curve ends mid-swing, and that last stretch is a run-up only if it is
    // rising. `last - anchor` is negative when the run finished while falling —
    // which on this operator's data is the common case — and rendering a
    // negative under a green heading reading "Run-up · Current" states the
    // opposite of what happened. It travels with its direction so the row can
    // say which of the two it is.
    const openStretch = curve[curve.length - 1] - anchor;
    if (dir !== 0 && openStretch !== 0) {
      swingSegments.push({ up: openStretch > 0, size: Math.abs(openStretch), current: true });
    }

    // ── Sharpe and Sortino on per-trade returns ──
    const base = Number(bench.open);
    const returns =
      Number.isFinite(base) && base ? rows.map((t) => (t.worst ?? 0) / base) : [];
    let sharpe = null;
    let sortino = null;
    if (returns.length > 1) {
      const m = returns.reduce((a, b) => a + b, 0) / returns.length;
      const sd = Math.sqrt(
        returns.reduce((a, r) => a + (r - m) * (r - m), 0) / (returns.length - 1)
      );
      // DOWNSIDE deviation only — the whole difference between the two ratios is
      // that Sortino does not punish a strategy for the size of its gains.
      const down = returns.filter((r) => r < 0);
      const dsd =
        down.length > 1
          ? Math.sqrt(down.reduce((a, r) => a + r * r, 0) / down.length)
          : null;
      // `spanYears` counts the months FOUND, not the months asked for — a span
      // with a hole is a shorter sample, and annualising it against the longer
      // window would overstate both ratios.
      const years = spanYears;
      const scale = years !== null && years > 0 ? Math.sqrt(returns.length / years) : null;
      if (sd > 0 && scale !== null) sharpe = (m / sd) * scale;
      if (dsd !== null && dsd > 0 && scale !== null) sortino = (m / dsd) * scale;
    }

    return {
      // `{t, v}` is the shape `areaChart` already eats, `t` in SECONDS off each
      // trade's own exit stamp.
      // TradingView overlays the resolved trade result as a zero-centred
      // histogram above the cumulative curve. `pnl` is the same exact
      // worst-fill integer added into `v`, not a second estimate.
      points: rows.map((t, i) => ({
        t: Math.round((t.exit_micros ?? 0) / 1e6),
        v: curve[i],
        pnl: t.worst ?? 0
      })),
      swingPlot:
        swingSegments.length > 0
          ? {
              segs: swingSegments,
              max: swingSegments.reduce((largest, segment) => Math.max(largest, segment.size), 1)
            }
          : null,
      maxRunUp: runUps.length > 0 ? maxRunUp : null,
      avgRunUp: mean(runUps),
      openStretch,
      openIsRunUp: openStretch >= 0,
      maxDrawdown,
      avgDrawdown: mean(drawdowns),
      avgRunUpDuration: mean(runUpLens),
      avgDrawdownDuration: mean(drawdownLens),
      sharpe,
      sortino,
      // Reduce, not spread — same stack-overflow reason as `maxOf` above.
      low: Math.min(0, minOf(curve)),
      high: Math.max(0, maxOf(curve)),
      // How many turns the curve made, which is what makes the averages above
      // readable: an average over four hundred swings is a different claim from
      // an average over two.
      swings: runUps.length + drawdowns.length
    };
  });

  /* Keep exact per-trade bars wide enough to read. Cumulative values remain
     absolute across pages, so paging changes only the visible window. */
  const performanceWindow = $derived(
    windowSeries(equity?.points ?? [], performancePage, PERFORMANCE_PAGE_SIZE)
  );

  /**
   * When this run made its money — by weekday and by hour of the session.
   *
   * # The operator's question, computed and then thrown away
   *
   * *"every day how many wins how many loss every week every month every
   * quarter every half every year"* is quoted in `crates/api/src/trades.rs`,
   * and `cli::trades` answers it: `/trades.json` has been serving eight grains
   * on every request. The page stored the object and rendered none of it.
   *
   * MEASURED on the 60min NIFTY run: every one of its 177 trades falls on a
   * **Friday**, and the IST 09:00–10:00 bucket alone is worth more at worst-case
   * fills than the entire run. A concentration like that is the difference
   * between an edge and an artefact, and it was one `{#each}` away from being
   * visible.
   *
   * Sorted by worst-case money rather than by key, because the question is
   * *"which"* and not *"in what order do Mondays come"*.
   */
  const timePatterns = $derived(timePatternRows(tradeList.periods));

  /** Which grain the `Results by time` chart is drawing. */
  let timeGrain = $state('hour');
  let timePage = $state(0);
  const TIME_PAGE_SIZE = 24;

  /** The streak chart's unit and bounded visible window. */
  let streakMode = $state('count');
  let streakPage = $state(0);
  const STREAK_PAGE_SIZE = 96;

  /** Only a bounded page enters the DOM; the durable series itself is complete. */
  const timeRows = $derived.by(() => {
    /** @type {any[]} */
    const all = timePatterns?.grains?.[timeGrain] ?? [];
    const window = windowSeries(all, timePage, TIME_PAGE_SIZE);
    const rows = window.rows;
    // One scale for the COMPLETE grain. Paging must move the viewing window,
    // not make an identical bucket appear taller or shorter on the next page.
    const tallest = all.reduce((largest, row) => Math.max(largest, row.trades), 1);
    return { ...window, tallest };
  });

  const streakRows = $derived.by(() => {
    /** @type {Array<{index:number, runIndex:number, seq:number, kind:'win'|'loss', count:number, amount:number}>} */
    const all = [];
    /** @type {'win'|'loss'|null} */ let kind = null;
    let count = 0;
    let amount = 0;
    let runIndex = 0;
    for (const row of tradeStats?.ordered ?? []) {
      const nextKind = row.worst > 0 ? 'win' : 'loss';
      if (nextKind !== kind) {
        kind = nextKind;
        count = 0;
        amount = 0;
        runIndex += 1;
      }
      count += 1;
      amount += row.worst;
      all.push({ index: all.length + 1, runIndex, seq: row.seq, kind, count, amount });
    }
    const tallest = all.reduce(
      (largest, step) =>
        Math.max(largest, streakMode === 'amount' ? Math.abs(step.amount) : step.count),
      1
    );
    const window = windowSeries(all, streakPage, STREAK_PAGE_SIZE);
    const rows = window.rows.map((step) => ({
      ...step,
      value: streakMode === 'amount' ? Math.abs(step.amount) : step.count
    }));
    return { ...window, rows, tallest };
  });

  /** Mean bars held, for the reference's fourth tile. */
  const averageDuration = $derived.by(() => {
    if (tradeRows.length === 0) return null;
    const sum = tradeRows.reduce((n, t) => n + (t.bars_held ?? 0), 0);
    return Math.round(sum / tradeRows.length);
  });

  /**
   * The counts four locked metrics said were not recorded.
   *
   * # What was wrong
   *
   * `Total winners`, `Total losers`, `Percent profitable` and `Largest profit`
   * each rendered a padlock reading *"No win count is recorded"* or *"Only the
   * worst single trade is kept"*. Both sentences were true of the RESULTS
   * ledger, which keeps four scalars per run — and both stopped being true when
   * `cli::trades` began writing per-trade rows and `/trades.json` began serving
   * `periods`. Every bucket carries `trades`, `wins`, `worst_wins`,
   * `largest_win` and `largest_loss`; the totals are their sums.
   *
   * MEASURED on this store: 177 trades, and the `year` buckets sum to exactly
   * 177 — so the buckets partition the trade list rather than sampling it.
   *
   * # `worst_wins` is the headline, and that is not a detail
   *
   * `wins` counts trades that won at the BEST fill; `worst_wins` counts those
   * that won at the WORST. On this run the two are 99 and 68 — a win rate of
   * 55.93% or 38.42% depending on which is quoted. The page's header says
   * *"ranked on worst-case fills"*, so the worst is the headline and the best
   * travels beside it. Quoting only the flattering one is the failure this
   * whole console exists to avoid.
   *
   * `year` and not `day`: the coarsest bucket set is the fewest rows to add,
   * and every set totals the same because each partitions the same list.
   */
  const tradeTotals = $derived.by(() => {
    const buckets = tradeList.periods?.year;
    if (!Array.isArray(buckets) || buckets.length === 0) return null;
    let trades = 0;
    let wins = 0;
    let worstWins = 0;
    let largestWin = -Infinity;
    let largestLoss = Infinity;
    for (const b of buckets) {
      trades += b.trades ?? 0;
      wins += b.wins ?? 0;
      worstWins += b.worst_wins ?? 0;
      if (Number.isFinite(b.largest_win)) largestWin = Math.max(largestWin, b.largest_win);
      if (Number.isFinite(b.largest_loss)) largestLoss = Math.min(largestLoss, b.largest_loss);
    }
    if (trades === 0) return null;
    return {
      trades,
      wins,
      worstWins,
      losses: trades - worstWins,
      bestLosses: trades - wins,
      // Basis points, so the two rates are compared as integers rather than as
      // floats the way §7 keeps money.
      rateBp: Math.round((worstWins / trades) * 10_000),
      bestRateBp: Math.round((wins / trades) * 10_000),
      largestWin: Number.isFinite(largestWin) ? largestWin : null,
      largestLoss: Number.isFinite(largestLoss) ? largestLoss : null
    };
  });

  /**
   * The running total, so the table can show a cumulative column.
   *
   * Computed here and not on the wire: it is derived from the rows the server
   * already sent, and sending it too would be a second copy of one fact.
   * Excursions are paisa, so this is paisa.
   */
  const tradeRows = $derived.by(() => {
    let running = 0;
    return tradeList.rows.map((t) => {
      // A REAL EQUITY CURVE. This comment used to read "`worst` is the adverse
      // excursion and is negative or zero" and call the running total an
      // excursion sum. `crates/cli/src/trades.rs` says `worst` is "the
      // WORST-CASE realised P&L, and the figure selection actually ranks on",
      // and that it is "positive on a trade that wins even at the worst fill" —
      // so it is neither an excursion nor sign-constrained, and the running sum
      // is the strategy's realised equity under the pessimistic fill model.
      running += t.worst ?? 0;
      return { ...t, cumulative: running };
    });
  });

  /**
   * The trades table's rows, NEWEST FIRST.
   *
   * The reference opens on trade 321 and counts down, because the question a
   * trades list is opened with is *"what did it just do"* and not *"what did it
   * do first"*. The header carries the `↓` that says so.
   *
   * Reversed for DISPLAY only — `tradeRows` accumulates chronologically, and
   * running that sum backwards would make every cumulative figure wrong.
   */
  /**
   * Bound the live table, not the evidence.
   *
   * The chosen-grid file can hold tens of thousands of trades. Rendering two
   * table rows for every one made the 850-trade reference run create 1,700 DOM
   * rows and made accessible-name queries time out. The complete validated
   * series still feeds every statistic and curve; only this newest-first slice
   * enters the table DOM.
   */
  const TRADE_PAGE_SIZE = 20;
  let tradePage = $state(0);
  const tradeRowsShown = $derived.by(() => {
    const newest = [...tradeRows].reverse();
    const pages = Math.max(1, Math.ceil(newest.length / TRADE_PAGE_SIZE));
    const page = Math.max(0, Math.min(tradePage, pages - 1));
    const start = page * TRADE_PAGE_SIZE;
    return newest.slice(start, start + TRADE_PAGE_SIZE);
  });
  const tradeRowsWindow = $derived.by(() => {
    const window = windowSeries(tradeRows, tradePage, TRADE_PAGE_SIZE);
    return { ...window, rows: tradeRowsShown };
  });

  /** Latest-request-wins generation for the independent live-event stream. */
  let liveSeq = 0;

  /** @param {any} run */
  function liveRunKey(run) {
    return JSON.stringify([
      run?.attempt,
      run?.kind,
      run?.feed,
      run?.underlying,
      run?.from_year,
      run?.from_month,
      run?.to_year,
      run?.to_month
    ]);
  }

  /** Revoke every pending live request, optionally clearing the prior attempt. */
  function invalidateLive(clear = false) {
    liveSeq += 1;
    if (clear) live = { phase: 'idle', attempt: null, rungs: [], why: '' };
  }

  /**
   * Publish only if this is still the newest request for the exact status run.
   * Publication also retires the ticket, on success and every failure path.
   * @param {number} seq
   * @param {any} run
   * @param {any} next
   */
  function publishLive(seq, run, next) {
    if (seq !== liveSeq || liveRunKey(sweep.run) !== liveRunKey(run)) return;
    live = next;
    liveSeq += 1;
  }

  /**
   * Rungs seen so far, newest first, for one exact opaque attempt token.
   * @param {any} run
   */
  async function fetchLive(run) {
    const seq = ++liveSeq;
    if (!Number.isSafeInteger(run?.attempt) || run.attempt <= 0) {
      publishLive(seq, run, {
        phase: 'failed',
        attempt: null,
        rungs: [],
        why: 'The status endpoint supplied no positive safe attempt token, so no log event can be bound to this run.'
      });
      return;
    }
    try {
      const response = await ask_(
        `/logs.json?limit=200&run=${encodeURIComponent(String(run.attempt))}`,
        { cache: 'no-store' }
      );
      if (seq !== liveSeq || liveRunKey(sweep.run) !== liveRunKey(run)) return;
      if (!response.ok) {
        publishLive(seq, run, {
          phase: 'failed',
          attempt: run.attempt,
          rungs: [],
          why: `/logs.json answered ${response.status} for exact attempt ${run.attempt}.`
        });
        return;
      }
      const body = await response.json();
      if (seq !== liveSeq || liveRunKey(sweep.run) !== liveRunKey(run)) return;
      publishLive(seq, run, reduceLiveProgress(run, body));
    } catch (why) {
      publishLive(seq, run, {
        phase: 'failed',
        attempt: Number.isSafeInteger(run?.attempt) ? run.attempt : null,
        rungs: [],
        why: `The event feed could not be read: ${why instanceof Error ? why.message : String(why)}`
      });
    }
  }

  /** Six ledger mask words, each 64 bits wide. */
  const MASK_CAPACITY = 6 * 64;

  /**
   * Admit one complete vocabulary envelope or none of it.
   *
   * A partial table is more dangerous than no table: bit 17 decoded against a
   * shifted or duplicated row confidently names the wrong condition. Dense
   * own indices, a reconciled count, unique names, exact booleans and a version
   * are therefore one atomic admission decision.
   *
   * @param {unknown} payload
   * @returns {{ ok: true, version: number, bits: Map<number, {name: string, live: boolean}>, why: '' } | { ok: false, version: 0, bits: Map<number, {name: string, live: boolean}>, why: string }}
   */
  function validateVocabEnvelope(payload) {
    /** @param {string} why */
    const refuse = (why) => ({
      ok: /** @type {const} */ (false),
      version: /** @type {const} */ (0),
      /** @type {Map<number, {name: string, live: boolean}>} */
      bits: new Map(),
      why
    });
    if (payload === null || typeof payload !== 'object' || Array.isArray(payload)) {
      return refuse('the response is not an object');
    }
    const body = /** @type {Record<string, unknown>} */ (payload);
    if (!Number.isSafeInteger(body.vocab_version) || Number(body.vocab_version) < 1) {
      return refuse('vocab_version is not a positive safe integer');
    }
    if (
      !Number.isSafeInteger(body.count) ||
      Number(body.count) < 1 ||
      Number(body.count) > MASK_CAPACITY
    ) {
      return refuse(`count is not an integer between 1 and ${MASK_CAPACITY}`);
    }
    if (!Array.isArray(body.bits) || body.bits.length !== body.count) {
      return refuse('bits is not an array whose length equals count');
    }

    /** @type {Map<number, {name: string, live: boolean}>} */
    const bits = new Map();
    const names = new Set();
    for (let at = 0; at < body.bits.length; at += 1) {
      const raw = body.bits[at];
      if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
        return refuse(`bits[${at}] is not an object`);
      }
      const bit = /** @type {Record<string, unknown>} */ (raw);
      if (!Number.isSafeInteger(bit.i) || Object.is(bit.i, -0) || bit.i !== at) {
        return refuse(`bits[${at}].i is not its dense append-only position`);
      }
      if (typeof bit.name !== 'string' || bit.name.length === 0 || bit.name.trim() !== bit.name) {
        return refuse(`bits[${at}].name is not a non-empty canonical string`);
      }
      if (names.has(bit.name)) {
        return refuse(`bits[${at}].name duplicates an earlier condition`);
      }
      if (typeof bit.live !== 'boolean') {
        return refuse(`bits[${at}].live is not a boolean`);
      }
      names.add(bit.name);
      bits.set(at, { name: bit.name, live: bit.live });
    }
    return {
      ok: /** @type {const} */ (true),
      version: Number(body.vocab_version),
      bits,
      why: /** @type {const} */ ('')
    };
  }

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
      const checked = validateVocabEnvelope(body);
      if (!checked.ok) {
        vocab = {
          phase: 'failed',
          version: 0,
          bits: new Map(),
          why:
            `The condition table was refused: ${checked.why}. No mask is decoded ` +
            `against a partial or malformed vocabulary.`
        };
        return;
      }
      vocab = { phase: 'ready', version: checked.version, bits: checked.bits, why: '' };
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
   * Reads the ranked combinations of a finished sweep.
   *
   * `feed` and `underlying` filter TOGETHER — the route refuses one without the
   * other rather than guessing, so both are checked here before a request is
   * made. Missing either is not an error worth showing: it means the sweep
   * carried no identity to ask about, and the block simply stays hidden.
   *
   * The response is `{report, refusal}`, exactly one of which is non-null. A
   * refusal is DISPLAYED rather than swallowed, for the reason `fetchVocab`
   * gives beside it: a page that silently shows nothing looks identical to a
   * sweep that found nothing, and those are opposite facts. A 404 in particular
   * means the running binary predates the route — the page comes off disk and
   * the route does not.
   */
  /**
   * @param {string | null | undefined} feed
   * @param {string | null | undefined} underlying
   *   Both come off `running`, where every field but `in_flight` is optional —
   *   so both may be absent, and the guard below is the reason they are typed
   *   nullable rather than as plain strings.
   */
  async function fetchTop(feed, underlying) {
    if (!feed || !underlying) {
      top = { phase: 'idle', report: '', why: '' };
      return;
    }
    top = { phase: 'loading', report: '', why: '' };
    try {
      const response = await ask_(
        `/engine/top.json?feed=${encodeURIComponent(feed)}` +
          `&underlying=${encodeURIComponent(underlying)}`
      );
      const body = await response.json();
      if (!response.ok || !body.report) {
        top = {
          phase: 'failed',
          report: '',
          why:
            body.refusal ??
            `/engine/top.json answered ${response.status}. The ranked ` +
              `combinations were written to the store either way — this page ` +
              `could not read them back. A 404 means the running binary is ` +
              `older than this page.`
        };
        return;
      }
      top = { phase: 'ready', report: body.report, why: '' };
    } catch (why) {
      top = {
        phase: 'failed',
        report: '',
        why: `The ranked combinations could not be fetched: ${why instanceof Error ? why.message : String(why)}`
      };
    }
  }

  async function fetchLedger() {
    const seq = ++ledgerSeq;
    load.phase = 'loading';
    try {
      const response = await ask_(`/backtest.json?limit=${LIMIT}`, { cache: 'no-store' });
      if (seq !== ledgerSeq) return;
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
        if (seq !== ledgerSeq) return;
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
      const body = await response.json();
      if (seq !== ledgerSeq) return;
      const checked = validateLedgerPayload(body);
      if (!checked.ok) {
        if (seq !== ledgerSeq) return;
        load = {
          phase: 'failed',
          body: null,
          why: `${checked.why} Nothing from it was ranked or opened.`
        };
        return;
      }
      if (seq !== ledgerSeq) return;
      load = { phase: 'ready', body: checked.body, why: '' };
    } catch (error) {
      if (seq !== ledgerSeq) return;
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
    // AND WHETHER ONE IS ALREADY RUNNING. See `adoptRunning`: without this the
    // in-flight view survived exactly one browser reload, which is none.
    untrack(() => adoptRunning());
    // Component teardown (or a future effect reset) revokes every pending read.
    return () => {
      ledgerSeq += 1;
    };
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

  /**
   * `YYYY-MM` as the two numbers the route wants, or null.
   * @param {string | null | undefined} text
   */
  function months(text) {
    const m = /^(\d{4})-(\d{2})$/.exec(text ?? '');
    if (!m) return null;
    const year = Number(m[1]);
    const month = Number(m[2]);
    return month >= 1 && month <= 12 ? { year, month } : null;
  }


  /**
   * A SWEEP THIS PAGE DID NOT START IS STILL A SWEEP THIS PAGE IS FOR.
   *
   * `pollSweep` was reachable from exactly one place — `startSweep` — so the
   * in-flight banner, the rung table and the started stamp existed only for the
   * tab that pressed Run. A reload, a second tab, or a sweep launched from a
   * terminal left `sweep.phase` at `idle` for the whole run, and
   * `/backtest/run.json` was never asked at all.
   *
   * Measured: the exit grid is 87.6% of a run's wall clock, and a `range-all`
   * over eight rungs ran five hours on this machine. For all of it the page
   * would render the PREVIOUS run's ledger and say nothing about the one
   * actually happening — the same "cannot tell a working sweep from a hung one"
   * the rung table was written to end, arriving through the one door that table
   * cannot cover.
   *
   * IT ADOPTS ONLY A RUNNING SWEEP. `sweepOutcome` is total over the payload,
   * so `done` and `failed` are reachable here too — and printing either on load
   * would be a claim about something that ended before this page was open, over
   * a ledger it has only just read. `idle` is what a fresh console should show.
   */
  let adoptWhy = $state('');

  async function adoptRunning() {
    try {
      const response = await ask_('/backtest/run.json', { cache: 'no-store' });
      if (!response.ok) {
        adoptWhy =
          `/backtest/run.json answered ${response.status}, so this page cannot say whether a ` +
          `sweep is running. The idle console below is what this build shows when it could not ` +
          `ask — not a claim that nothing is in flight.`;
        return;
      }
      const body = await response.json();
      const next = sweepOutcome(body.running);
      if (next.phase !== 'running') return;
      invalidateLive(liveRunKey(sweep.run) !== liveRunKey(next.run));
      sweep = next;
      // THE EVENT FEED ON THE SAME TICK, exactly as `pollSweep` does it: the
      // status payload carries no progress and the rungs live in the log.
      fetchLive(next.run);
      pollAt = setTimeout(pollSweep, 2000);
    } catch (error) {
      // NAMED, NOT SWALLOWED. A page that could not ask is not a page with no
      // run, and rendering the idle state over a live sweep is the fallback
      // that hides a failure §4 bans.
      adoptWhy =
        `Could not ask whether a sweep is already running: ` +
        `${error instanceof Error ? error.message : String(error)}`;
    }
  }
  async function pollSweep() {
    try {
      const response = await ask_(`/backtest/run.json`, { cache: 'no-store' });
      if (!response.ok) {
        invalidateLive();
        return;
      }
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
        // AND THE EVENT FEED, on the same tick. `next` carries no progress —
        // see `live` — so this is where the page learns which rungs have
        // loaded, what threshold each derived, and which have finished.
        fetchLive(next.run);
        // AND THE BEST ROWS FOUND SO FAR, on the same tick. `fetchLive` folds
        // per-rung PROGRESS out of `/logs.json`; this reads the ranked heap
        // itself. They answer different questions and neither is awaited, so a
        // slow read of one cannot hold up the other.
        fetchLiveTop();
        pollAt = setTimeout(pollSweep, 2000);
        return;
      }
      // DONE IS A COMPLETE REPORT, NOT A PROMISE THAT EVERY RUNG APPENDED.
      // The report carries each rung's recorded/refused outcome. Re-read the
      // ledger because one or more rows MAY have committed; a whole-run refusal
      // does not take this branch.
      if (next.phase === 'done') {
        fetchLedger();
        // ONE LAST READ, so the finished state shows every rung's outcome
        // rather than freezing on whatever the last poll happened to catch.
        fetchLive(next.run);
        // ONE FINAL READ OF THE HEAP TOO, so the panel freezes on the run's
        // real best rows rather than on whatever the last 2-second tick caught.
        fetchLiveTop();
        // AND THE RANKED COMBINATIONS, which is what was actually being asked
        // for. `fetchLedger` re-reads one row per run; this reads the twenty-five
        // rows behind each of them. Fired together and awaited by neither,
        // because they answer different questions and a slow frontier read must
        // not hold up the ledger table.
        fetchTop(next.run?.feed, next.run?.underlying);
      } else {
        invalidateLive();
      }
    } catch (error) {
      invalidateLive();
      sweep = {
        phase: 'failed',
        run: null,
        why: error instanceof Error ? error.message : String(error)
      };
    }
  }

  async function startSweep() {
    // WHICH COMMAND THIS TAB PRESSED, before anything can be asked about it.
    // `/backtest/run.json` carries `kind` and is the authority, but it arrives
    // one poll later — and on a refusal raised HERE it never arrives at all.
    // See `pressed`.
    pressed = 'sweep';
    invalidateLive(true);
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
          rungs: [...pickedRungs],
          // THE ENGINE'S OWN KNOBS, FROM THIS FORM.
          //
          // Until these travelled, every one of them lived in the server's
          // PROCESS ENVIRONMENT — which meant the only way to change how a
          // sweep searched was to restart the server, and the only way to see
          // how it had searched was to read that process's environment from
          // outside it.
          //
          // MEASURED, 2026-08-29: a server started from the IDE with none of
          // them set gave `screen_cap` 10,000 and `validate` ON, which composes
          // into a run that does not finish. The button was reachable and the
          // configuration was not.
          //
          // Spread LAST so a knob can never overwrite the span or the feed:
          // `engineKnobs` only ever yields names from its own table, but the
          // ordering makes that a property of the code rather than of the
          // table's current contents.
          ...engineKnobs
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

  /* ====================================================================
     DESCENDING ONE RUNG — the second command, and the door it had none of
     --------------------------------------------------------------------
     `POST /backtest/descend` has been registered in `crates/api/src/server.rs`
     and handled by `sweeprun::descend` with NOTHING IN THIS BROWSER CALLING
     IT. Measured: `grep -rn descend web/src/` matched `aria-activedescendant`
     and nothing else. Built, routed, tested, unreachable — so the only thing
     this console could launch was `/backtest/run`.

     WHY THAT MATTERS RATHER THAN BEING A MISSING BUTTON. The bar above sweeps
     every selected rung at a threshold each one derives from its own bars,
     which is the right shape for asking *which timeframe carries the edge*:
     rungs are only comparable on equal terms. `cli::elite_descend`'s own doc
     records what that shape cannot do — a run launched at a twentieth of
     200,000 ppm "cannot report a once-a-week setup no matter how long it runs
     — the setup was pruned in the first level of the ladder, and the report
     says nothing about it because nothing counted it." A once-a-week cadence
     is about 3,656 ppm. The descent walks the threshold DOWN on ONE rung
     instead, which is roughly eight times the effective budget because it is
     spent on one timeframe rather than spread across eight.

     IT SHARES THE SLOT, BECAUSE IT SHARES THE LEDGER. Both commands append to
     the same append-only file, so `api::sweeprun` holds ONE `Progress` slot
     and refuses the second press — "two of them finishing together can
     interleave two records" — rather than queueing it. That is why the poll,
     the event feed and the outcome plumbing below are REUSED here and not
     copied: a second poller would be a second reader of one slot, and the two
     would disagree about which run they were watching. What the page owes in
     exchange is saying WHICH command is in that slot, which is what `runKind`
     is for and why the banners below read off it rather than off a heading.

     A CEILING TRAVELS IN POINTS, NEVER IN PPM. `sweeprun::descent_from` states
     the rule in its own comment: `cli` converts the ceiling against the
     midpoint of the span's OWN bars, "and this side never sees a ppm, which is
     the whole reason that entry point exists". A ppm computed in a browser
     would be a second answer to a question only the bars can settle — the
     paisa/points confusion that already ran a stop ladder a hundred times too
     tight, arriving through a form field.
     ==================================================================== */

  /**
   * A whole number above zero, or `null` for anything else.
   *
   * THE DIGITS ARE MATCHED FIRST AND CONVERTED SECOND, and the order is the
   * whole function. `Number('')` is `0`, `Number(' 12 ')` is `12` and
   * `parseInt('12abc')` is `12` — all three of the obvious readings accept
   * text the operator did not type a number into, and the third turns a
   * fat-fingered ceiling into a plausible one.
   *
   * ZERO IS REFUSED HERE AS WELL AS ON THE SERVER, and the duplication is
   * deliberate rather than a second copy of a rule. `descent_from` is the
   * authority — it refuses `max_points` at zero because "a ceiling of zero
   * admits no trade and a negative one is not a distance", and `top` at zero
   * because "a listing of no rows is not a shorter answer, it is no answer" —
   * and those refusals still arrive if this is wrong. This exists so the
   * control can name WHICH field is empty while the operator is looking at it,
   * instead of after a round trip. It never widens what the server accepts and
   * it never narrows it silently: anything this lets through is still judged
   * there.
   *
   * @param {string} text
   * @returns {number | null}
   */
  function wholeNumber(text) {
    const digits = (text ?? '').trim();
    if (!/^\d+$/.test(digits)) return null;
    const n = Number(digits);
    return Number.isSafeInteger(n) && n > 0 ? n : null;
  }

  /**
   * What a descent sends over and above the feed, instrument and span the bar
   * above already holds.
   *
   * NOTHING IS SEEDED, for the reason `ask` is not seeded either: a rung and a
   * span are terms in the run's identity, and a figure the operator did not
   * choose would name a run after something nobody asked for. The stop ceiling
   * is worse than that — it decides which trades the run is allowed to keep —
   * so a default here would be a silent parameter, which is the shape §6
   * refuses for `k` and the reason the support control on the bar above was
   * deleted rather than given a sensible value.
   *
   * The three are STRINGS because they are what was typed. Parsing happens in
   * `wholeNumber`, once, where the refusal can be named.
   */
  let descent = $state({ rung: '', points: '', top: '' });

  /**
   * WHICH COMMAND THIS PAGE PRESSED, for the window in which nothing else can
   * say.
   *
   * `/backtest/run.json` carries `kind` — `sweeprun::Kind::word()`, one of
   * `sweep`, `descent` and `command` — and it is the authority. But it only
   * arrives on the first poll, and both start functions set `phase` to
   * `starting` and then `running` with `run: null` before that. A banner
   * reading "Sweeping" over a descent for the first 1.2 seconds is a small
   * lie; a banner reading it over a LOCAL refusal — a malformed month, a 409
   * from the slot — is a lasting one, because that state carries no payload at
   * all and never will.
   *
   * So this is the fallback and never the override: `runKind` prefers the
   * server's word the moment there is one.
   */
  let pressed = $state(/** @type {'sweep' | 'descent' | null} */ (null));

  /**
   * The word the slot's own payload uses for what is in it, or what this tab
   * pressed when there is no payload yet.
   *
   * NOT INFERRED FROM `support_ppm`, and `sweeprun::Kind`'s own doc says why:
   * a sweep fixes that number and a descent WALKS it, so the field means "the
   * threshold" in one case and "where the walk began" in the other. Encoding
   * the difference in a magic value of a numeric field is the shape §4 bans.
   */
  const runKind = $derived(
    typeof sweep.run?.kind === 'string' ? sweep.run.kind : pressed
  );

  /**
   * The three words `Kind::word()` can send, as a reader meets them.
   *
   * AN UNKNOWN WORD PRINTS ITSELF. A fourth kind added to that enum would
   * otherwise render as an empty heading, which reads as a run with no command
   * — and this page has already paid for one absent field looking like an
   * absent fact.
   *
   * @type {Record<string, string>}
   */
  const RUN_WORDS = { sweep: 'Sweep', descent: 'Descent', command: 'Command' };

  /** What to call the run in the slot, in a sentence. */
  const runLabel = $derived(runKind ? (RUN_WORDS[runKind] ?? runKind) : 'Run');

  /**
   * Whether ANY run holds the slot — this tab pressed one, or another did.
   *
   * Both controls read it, because both are refused by the same lock on the
   * server: "a sweep or descent is already running in this process. Both append
   * to the same append-only ledger". A button that can only earn a 409 is not a
   * button, so neither is left pressable.
   */
  const inFlight = $derived(sweep.phase === 'starting' || sweep.phase === 'running');

  /**
   * Start a descent: one rung, the threshold walked down from a ceiling stated
   * in index points.
   *
   * EVERY FIELD IS REFUSED BY NAME, in order, and none of the four checks is a
   * clamp. "The form is wrong" is not a sentence anybody can act on, and a
   * silently corrected ceiling is worse than a refused one — it produces a run
   * whose identity records a number nobody chose.
   *
   * THE ENGINE KNOBS ARE NOT SENT, and their absence is not an oversight.
   * `sweeprun::conduct` calls `apply_knobs` before `range_over`; `conduct_descent`
   * calls `elite_descend_in_points` and applies nothing. Spreading `engineKnobs`
   * into this body would put fourteen inert fields on the wire and let the
   * Engine-settings panel above imply it had configured a run it did not touch
   * — a control that quietly does something other than what it shows, which is
   * exactly the failure §4 bans. The note under the button says so instead.
   */
  async function startDescent() {
    pressed = 'descent';
    invalidateLive(true);
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
    if (!descent.rung) {
      sweep = {
        phase: 'failed',
        run: null,
        why:
          'A descent walks ONE rung down, so a timeframe is required rather than defaulted. ' +
          'The route refuses a body without one and names the rungs it sweeps in the refusal.'
      };
      return;
    }
    const ceiling = wholeNumber(descent.points);
    if (ceiling === null) {
      sweep = {
        phase: 'failed',
        run: null,
        why:
          'The stop ceiling must be a whole number of INDEX POINTS above zero — the widest ' +
          'adverse excursion this run may accept. It is never a ppm: the engine converts it ' +
          'against the midpoint of the span’s own bars, and that conversion is the reason ' +
          'this command exists.'
      };
      return;
    }
    const listRows = wholeNumber(descent.top);
    if (listRows === null) {
      sweep = {
        phase: 'failed',
        run: null,
        why: 'The list length must be a whole number of rows above zero.'
      };
      return;
    }
    sweep = { phase: 'starting', run: null, why: '' };
    try {
      const response = await ask_('/backtest/descend', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          feed: activeFeed,
          underlying: sweepSymbol,
          // ONE RUNG, AND THE FIELD IS SINGULAR ON PURPOSE. `descent_from`
          // reads `rung`; `asked_from` underneath it reads `rungs`, and a
          // `rungs` key present and EMPTY is refused outright. This body sends
          // neither a list nor an empty one.
          rung: descent.rung,
          from_year: from.year,
          from_month: from.month,
          to_year: to.year,
          to_month: to.month,
          max_points: ceiling,
          top: listRows
        })
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok || body.accepted !== true) {
        // THE SERVER'S OWN SENTENCE, NEVER A PARAPHRASE. A refusal here names
        // the rung it will not sweep, or the span that runs backwards, or that
        // a run is already in the slot — 400, 409 and 503 all arrive this way,
        // and each says what to do about it.
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

  // A POLL MUST NOT OUTLIVE THE PAGE. Without this a navigation away leaves a
  // timer firing against a component that is gone.
  $effect(() => () => {
    if (pollAt !== null) clearTimeout(pollAt);
    invalidateLive();
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
   * The five exit axes as rung indices, `-1` where an axis is unused.
   * `crates/cli` holds it as `[i*; 5]` and reads it with `.first()` and
   * `.get(n).unwrap_or(-1)`, so it is five signed numbers on the wire. It was
   * `unknown`, which made the panel's `{#each … ?? []}` an iteration over a
   * value the checker could not call iterable. OPTIONAL, because that `?? []`
   * is the reader's own statement that a build may not send it.
   * @property {number[]} [exit_rungs]
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
   * @property {number | null} best_complete
   * @property {string[]} signal_rungs exact `cli::EVERY_RUNG` values published by the server
   * @property {string | null} refusal
   * @property {Run[]} runs
   */

  /** @type {Ledger | null | undefined} */
  const ledger = $derived(load.body);
  /** Every run the server returned, newest first — the server ordered them. */
  const allRuns = $derived(Array.isArray(ledger?.runs) ? ledger.runs : []);
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
  /* `!activeFeed` NO LONGER MEANS "SHOW EVERYTHING" -- that was a fallback
     that hid a fact, which is the exact thing the paragraph above forbids.

     It was unreachable when written: `feeds.active` could not be null once
     `/feeds.json` answered, because every writer in the tree assigned a
     concrete wire. It is reachable NOW -- `+layout.svelte` lets the operator
     clear the feed by choosing the chosen one again -- and the moment it was,
     this line began mixing every vendor's runs into one table.

     Silently, and doubly so. `hiddenByFeed` became `allRuns.length -
     runs.length` = 0, which suppresses the "recorded under another feed" note;
     and `everyFeed` was false, which suppresses the "Showing every feed" note.
     Both escape hatches closed over a table holding four vendors' records
     under one heading -- while the run bar three inches above it read "choose
     a feed first". The two halves of the page disagreed about whether a feed
     was selected, and only one of them was right.

     No feed now yields NO rows, and the empty-state branch below names the
     reason and offers the same way out. */
  const scopedRuns = $derived(
    everyFeed
      ? allRuns
      : activeFeed
        ? allRuns.filter((r) => {
            // A malformed row with no usable feed cannot be attributed to a
            // different vendor and silently hidden. Keep it in the active
            // scope so the admission door can name and refuse it.
            if (r === null || typeof r !== 'object' || Array.isArray(r)) return true;
            return typeof r.feed !== 'string' || r.feed === activeFeed;
          })
        : []
  );
  /* ONE ADMISSION DOOR BEFORE EVERY ANSWER, RUNG, SORT AND DRILL COMPUTATION.
     A JSON parser has already rounded an out-of-range i64/u64 by this point;
     no formatter can recover it. Refused rows stay visible in the comparison
     as metadata with the exact reason, but do not enter any arithmetic. */
  const runAdmissions = $derived(
    scopedRuns.map((run) => ({ run, admission: validateRunForComputation(run) }))
  );
  const runs = $derived(
    runAdmissions.filter(({ admission }) => admission.ok).map(({ run }) => run)
  );
  const refusedNumericRuns = $derived(
    runAdmissions.filter(({ admission }) => !admission.ok)
  );
  /**
   * The plain-language comparison, sorted on the same conservative result the
   * rest of this page leads with. Ineligible rows remain visible after the
   * rankable ones with their exact reason; a stopped or unsealed run does not
   * disappear merely because it cannot be ranked.
   *
   * `compareRuns` is a plain tested fold rather than markup-side conditionals,
   * so the status, reference row and deltas cannot disagree across columns.
   */
  const comparedRuns = $derived.by(() => compareRuns(scopedRuns, ledger?.signal_rungs));
  /**
   * The server owns the signal-rung vocabulary.  A suffix is not authority:
   * `garbagemin` ends in `min` and is still not a rung the engine swept.
   * Malformed or duplicate wire values withhold every headline and leader.
   */
  const signalRungSet = $derived.by(() => {
    const values = ledger?.signal_rungs;
    if (
      !Array.isArray(values) ||
      values.length === 0 ||
      Array.from(values).some((value) => typeof value !== 'string' || value.length === 0)
    ) return null;
    const exact = new Set(values);
    return exact.size === values.length ? exact : null;
  });
  /** A comparison count either stays exact or says why no number is shown. */
  const comparisonCount = (/** @type {unknown} */ value) =>
    typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
      ? exact(value)
      : 'not exact';
  /** @param {Run} run */
  const comparisonSpan = (run) =>
    Number.isSafeInteger(run.from_year) &&
    Number.isSafeInteger(run.to_year) &&
    Number.isSafeInteger(run.from_month) &&
    run.from_month >= 1 &&
    run.from_month <= 12 &&
    Number.isSafeInteger(run.to_month) &&
    run.to_month >= 1 &&
    run.to_month <= 12
      ? span(run)
      : 'span not exact';
  const rankableRunCount = $derived(comparedRuns.filter((r) => r.eligible).length);
  const positiveRankableCount = $derived(
    comparedRuns.filter((r) => r.eligible && r.pessimistic > 0).length
  );
  const negativeRankableCount = $derived(
    comparedRuns.filter((r) => r.eligible && r.pessimistic < 0).length
  );
  const breakevenRankableCount = $derived(
    rankableRunCount - positiveRankableCount - negativeRankableCount
  );
  const COMPARISON_STEP = 12;
  let comparisonShown = $state(COMPARISON_STEP);
  const comparisonDatasetKey = $derived(
    `${everyFeed}:${activeFeed ?? ''}:${scopedRuns.length}:${scopedRuns[0]?.identity ?? ''}:${scopedRuns[scopedRuns.length - 1]?.identity ?? ''}`
  );
  $effect(() => {
    // Expanding one feed must not make the next feed inherit a hundreds-row
    // DOM. The ledger is append-only, so its length and end identities are a
    // sufficient dataset change signal without walking every row again.
    comparisonDatasetKey;
    comparisonShown = COMPARISON_STEP;
  });
  const visibleComparedRuns = $derived(comparedRuns.slice(0, comparisonShown));
  const comparisonRemaining = $derived(
    Math.max(0, comparedRuns.length - visibleComparedRuns.length)
  );
  const hiddenByFeed = $derived(allRuns.length - scopedRuns.length);

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
   * `monthList` AND `daily` WERE ALWAYS BUILT AND WERE NEVER DECLARED.
   *
   * `loadCatalog` has emitted both on every entry since it was written —
   * `monthList` is the sorted month keys the span control offers, `daily` the
   * non-`min` rungs kept beside the swept ones. Three readers use them:
   * `monthRows` maps `monthList`, and `coverNote` reads `daily.length` and
   * maps its names.
   *
   * The checker reported those three as reads of a property that does not
   * exist, which is the shape of a crash and was not one: the CONSTRUCTOR is
   * the authority on what a value carries, and it carries them. The type was
   * the stale half. Recording that here because the opposite reading — delete
   * the reads — was equally available from the error alone, and would have
   * removed two working controls.
   *
   * @typedef {{ name: string, months: number, from: string, to: string }} Rung
   * @typedef {{
   *   leaf: string,
   *   full: string,
   *   months: number,
   *   monthList: string[],
   *   from: string,
   *   to: string,
   *   rungs: Rung[],
   *   daily: Rung[]
   * }} Held
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
  /**
   * Has the operator worked the TIMEFRAMES menu himself? Same rule again, and
   * this one was missing — see the seed effect and the menu's `onchange`.
   * Reset alongside `spanTouched` when the instrument changes, because which
   * rungs exist is a property of the instrument, so a new one must re-seed.
   */
  let rungsTouched = $state(false);

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
   *
   * IT LIVES BELOW `pickedSymbols` BECAUSE IT READS IT. This sat four
   * hundred lines earlier, above the declaration, and the checker called it
   * a use before declaration. Nothing crashed: `$derived` compiles to a
   * thunk and is not evaluated until something reads it, and the four
   * readers all run later. But "correct because the evaluation happens to be
   * deferred" is a property of the compiler, not of the code, and it stops
   * being true the moment anything above needs an eager read. Ordering the
   * declaration after the thing it depends on costs nothing and removes the
   * question.
   */
  const sweepSymbol = $derived([...pickedSymbols][0] ?? '');

  /**
   * Toggle one key in a chosen set.
   *
   * THE EMPTY SET IS REACHABLE ON PURPOSE, and this function used to refuse
   * it — the last selected chip would not turn itself off, on the argument
   * that a control can refuse structurally rather than by validation.
   *
   * That argument was wrong HERE, and the menus made it obvious: `Picker`
   * ships a **Clear all**, so the same rule turned a labelled control into
   * one that silently did nothing. A control whose label and behaviour
   * disagree is the failure §4 bans, and it is worse than the state it was
   * protecting against — an empty selection is a legal thing to want on the
   * way to picking something else, and it is visible.
   *
   * The refusal moved to the Run control, which is where the operator is
   * looking when it matters and which can say why.
   *
   * @param {Set<string>} set
   * @param {string} key
   * @returns {Set<string>} a NEW set, because `$state` tracks identity
   */
  function toggled(set, key) {
    const next = new Set(set);
    if (next.has(key)) next.delete(key);
    else next.add(key);
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
          // INTRADAY ONLY, AND THE TEST IS THE SHAPE OF THE NAME.
          //
          // `1day` is on disk and is NOT a signal timeframe. It is there to
          // feed `indicators::daily` -- "the pivot ladder, the central pivot
          // range, and yesterday's high and low", bits 13-18 -- so a daily
          // bar DEFINES the previous session's OHLC for the intraday rungs
          // rather than being swept itself. `cli::EVERY_RUNG` is the eight
          // `*min` rungs and `cli::descend` refuses anything else by name;
          // `range_all`'s own banner reads "ALL EIGHT INTRADAY RUNGS".
          //
          // Matching on the `min` SUFFIX rather than listing the eight is
          // what keeps this from being a second copy of that const: a rung
          // the store gains tomorrow is swept if it is intraday and skipped
          // if it is not, with nothing here to update. The store's whole
          // rung set is kept beside it, because "what is on disk" is still
          // a fact this page states.
          rungs: [...h.rungs.values()]
            .filter((r) => /min$/.test(r.name))
            .sort((a, b) => byRung(a.name, b.name)),
          daily: [...h.rungs.values()]
            .filter((r) => !/min$/.test(r.name))
            .sort((a, b) => byRung(a.name, b.name))
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
      const target = (body?.targets ?? []).find(
        (/** @type {{ target?: string } | null | undefined} */ t) => t?.target === 'swept'
      );
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
      //
      // `!rungsTouched` IS WHAT MAKES "until he says otherwise" TRUE. Keyed on
      // `size === 0` alone, this could not tell a stale selection from a
      // deliberate Clear all -- and since the effect tracks `spanTouched`, the
      // next edit to `from` or `to` refilled all eight rungs behind him. The
      // argument above is about STALENESS; clearing is a decision, and the
      // latch is the only thing that separates the two.
      if (held && !rungsTouched && pickedRungs.size === 0) {
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

  /**
   * Do the timeframes disagree about how much history they hold?
   *
   * WHEN THEY DO NOT, THE GAUGES ARE EIGHT COPIES OF 100% and the strip is
   * a comparison with nothing to compare — the same objection this file
   * already raises against drawing a bar for a solo rung. This is what
   * decides whether the picture is worth its space.
   */
  const rungsDiffer = $derived.by(() => {
    const rungs = heldNow?.rungs ?? [];
    if (rungs.length < 2) return false;
    const first = rungs[0].months;
    return rungs.some((r) => r.months !== first);
  });

  /**
   * Months a sweep can actually see — the widest INTRADAY coverage.
   *
   * `heldNow.months` counts every month the instrument holds at any rung,
   * and `1day` reaches further than the intraday ones on this store:
   * INDIAVIX is 121 daily and 119 at every `*min` rung. Comparing a
   * timeframe against 121 then puts `−2` on all eight of them, which is
   * both noise and slightly wrong — the two extra months exist only as
   * daily bars, and no sweep will ever read them.
   *
   * This is the number a run is measured against.
   */
  const sweepMonths = $derived(
    (heldNow?.rungs ?? []).reduce((most, r) => (r.months > most ? r.months : most), 0)
  );

  /**
   * The one line under the timeframe chips: what is on disk, in a sentence.
   *
   * Assembled here rather than as three `{#if}` fragments in the template.
   * Interleaving text runs and block opens around a `·` separator rendered
   * `only· 1day` — Svelte collapses the newline between them, so the
   * separator sat against the previous clause. An array and a `join` put
   * the spacing in ONE place, where it cannot drift per branch.
   */
  const coverNote = $derived.by(() => {
    if (!heldNow) return '';
    const parts = [
      `${exact(sweepMonths)} months on the intraday timeframes`,
      'spot indices only'
    ];
    if (heldNow.daily.length > 0) {
      parts.push(
        `${heldNow.daily.map((r) => r.name).join(', ')} held for the previous session's OHLC, never swept`
      );
    }
    if (sweptSurface) {
      parts.push(`${exact(sweptSurface.matched)} of ${exact(catalog.held.length)} are swept`);
    }
    return parts.join(' · ');
  });

  /** Is the asked-for span narrower than what the store holds? A FACT. */
  const spanShortfall = $derived.by(() => {
    if (!heldNow || askedMonths === null) return 0;
    return heldNow.months - askedMonths;
  });


  /* ---- what stops a descent, named one field at a time ---------------- */

  /**
   * The ceiling and the listing bound, as numbers, or `null` where the box
   * does not hold one.
   *
   * Derived rather than parsed at press time so the control can say what is
   * missing BEFORE the press — the same reason `blocked` is read at page load
   * rather than discovered after a five-hour sweep refuses its append.
   */
  const descentPoints = $derived(wholeNumber(descent.points));
  const descentRows = $derived(wholeNumber(descent.top));

  /**
   * Why the Descend control cannot be pressed, or `null` when it can.
   *
   * ONE REASON AT A TIME, IN THE ORDER THE ANSWERS DEPEND ON EACH OTHER —
   * feed, then census, then instrument, then span, then the three fields this
   * command adds. Listing every fault at once would put five sentences under a
   * button when the operator can only act on the first, and the cascade is
   * real: which rungs exist is a property of the instrument, which is a
   * property of the feed.
   *
   * IT DISABLES AND SAYS WHY; it never presses anyway and hopes. A control
   * whose label and behaviour disagree is the failure §4 bans — that is the
   * lesson `toggle` records above, where a chip that refused to clear itself
   * looked exactly like one that was broken.
   */
  const descentStop = $derived.by(() => {
    if (blocked !== null) return 'ledger';
    if (!activeFeed) return 'feed';
    if (catalog.phase !== 'ready') return 'census';
    if (pickedSymbols.size === 0) return 'instrument';
    if (!months(ask.from) || !months(ask.to)) return 'span';
    if (!descent.rung) return 'rung';
    if (descentPoints === null) return 'points';
    if (descentRows === null) return 'rows';
    return null;
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
  /** @param {{ sealed?: boolean }} r */
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
  /**
   * Is this run on a timeframe the engine actually sweeps?
   *
   * `1day` IS NOT A SIGNAL TIMEFRAME. It is on disk to feed
   * `indicators::daily` — the pivot ladder, the central pivot range and
   * **yesterday's high and low**, vocabulary bits 13–18 — so a daily bar
   * DEFINES the previous session's OHLC for the intraday rungs rather than
   * being swept itself. `cli::EVERY_RUNG` is the eight `*min` rungs and
   * `cli::descend` refuses anything else by name.
   *
   * The ledger can still HOLD such a run, because `cli::sweep_stored`
   * takes a rung string and this store has one recorded: NIFTY at `1day`,
   * 593,599 combinations, **0 trades, ₹0.00**. The page was crowning it
   * `BEST COMPLETE RUN` — the single most prominent thing on the surface,
   * naming a zero on a timeframe the engine does not sweep as the answer.
   *
   * The row is NOT hidden. §4 bans the fallback that hides a fact, and a
   * recorded run is a fact. It is excluded from RANKING and from the rung
   * comparison, exactly as a halted row is, and it says why where it
   * appears.
   *
   * Membership comes from `/backtest.json.signal_rungs`, the exact
   * `cli::EVERY_RUNG` values emitted by the server. A suffix test would admit
   * `garbagemin`; a browser list would become a stale second vocabulary.
   *
   * @param {any} r
   */
  const swept = (r) =>
    signalRungSet !== null && signalRungSet.has(String(r?.timeframe ?? ''));

  /** Recorded runs on a timeframe the engine never sweeps. */
  const offSurfaceRuns = $derived(
    signalRungSet === null ? [] : runs.filter((r) => !swept(r))
  );

  const completeRuns = $derived(runs.filter((r) => !r.halted && trustworthy(r) && swept(r)));
  /** Rows allowed to supply a product answer, not merely rows that terminated. */
  const rankableRuns = $derived(comparedRuns.filter((r) => r.eligible));
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
    for (const run of rankableRuns) {
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
   * the count was a real bug this page shipped with for one screenshot. When
   * this was written a `range-all` sweep held the support PERCENTAGE constant
   * across rungs, so the absolute `min_hits` necessarily differed at every rung
   * because the bar count does. Measured on the store's own ledger at the time:
   * 4,324/21,620 at 30min, 2,328/11,643 at 60min and 334/1,671 at 1day are all
   * 20.0% — three views of one question — and keying on `min_hits` split them
   * into three groups of one, which is exactly the comparison this section
   * exists to make.
   *
   * **D-0303 INVERTED THIS AND THE GROUPING HAS NOT BEEN REVISITED.** With the
   * floor now derived per rung, it is `min_hits` that comes out constant and
   * the percentage that varies: measured 2026-08-28, all eight rungs of one
   * NIFTY run were handed 28 hits, which is 0.0045% at 1min and 0.243% at
   * 60min — a factor of 54 apart. So grouping by ratio now splits one run into
   * eight rows where it used to join three, and grouping by `min_hits` would
   * join them. Which is right depends on what the reader is comparing, and
   * that is a design question rather than a typo; recorded here so the next
   * reader is not misled by the paragraph above it.
   *
   * This per-thousand value is for the one-decimal percentage display. The
   * comparison key above uses the exact reduced `min_hits / bars` fraction;
   * display rounding never identifies a search. `-1` is a visible refusal
   * sentinel and never a real ratio.
   *
   * @param {any} r
   */
  const supportPerMille = (r) => roundedScaledRatio(r.min_hits, r.bars, 1000) ?? -1;

  /**
   * One key per comparable question: same feed, same instrument, same span,
   * same support RATIO. Only runs sharing all four are comparable across rungs.
   *
   * @param {any} r
   */
  const groupKey = comparisonQuestionKey;

  /** @param {any} r */
  const runKey = (r) => `${r.index}:${r.identity}`;
  const rankableRunKeys = $derived(new Set(rankableRuns.map(runKey)));

  /**
   * The rung comparison: one entry per comparable span, each holding the runs
   * at each rung. **One pass**, into a Map — not a nested compare.
   */
  const rungGroups = $derived.by(() => {
    /** @type {Map<string, any>} */
    const byKey = new Map();
    for (const run of runs) {
      // "WHICH RUNG CARRIES THE EDGE" IS A QUESTION ABOUT SIGNAL RUNGS.
      // A `1day` record has no place in that comparison: the engine does
      // not sweep it, so ranking it against the eight it does sweep would
      // answer a question nobody asked with a row nobody can act on. It
      // stays in the ledger below, where it is listed and labelled.
      if (!swept(run)) continue;
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
        const complete = present.filter((r) => rankableRunKeys.has(runKey(r)));
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
  /* `$lib/find.js` AND NOT `$lib/prefix.js`, BECAUSE THIS FILTER WAS DEAD.
     `prefix.build` keys on `row.symbol` and SKIPS any row without one
     (`prefix.js:42-43`). A `Run` has `underlying`, `feed`, `timeframe` and
     `index` — never `symbol` — so the Map was built EMPTY and
     `prefix.probe` returned `[]` for every query. Reproduced against this
     exact shape: index size 0; `n`, `nifty`, `bank`, `zer`, `15m` all 0 rows.
     Typing anything printed "Nothing matches …" forever, and the header three
     paragraphs up called it "a prefix probe into a Map built once per payload"
     — the Map existed and held nothing.

     A second, independent break sat on the same two lines: this lower-cased
     the query while `prefix.probe` upper-cases it, against an `id` that was
     also lower-cased. Even with the field renamed the two cases never met.

     Renaming the field would not have been enough anyway. The behaviour
     promised above is INFIX — `zer` is not a prefix of `NIFTY zerodha 30min` —
     and `prefix.js` cannot serve that at any key. `find.js` is the module that
     does, takes an explicit key accessor (its own header names this exact
     "the census spells it `sym`, the master spells it `symbol`" trap), and
     upper-cases both sides itself. Verified against this shape: `bank`, `zer`,
     `15m` and `30` each narrow to one row, `xyz` to none, empty to all.

     The `searchable` spread goes with it — an O(rows x fields) copy per
     payload that existed only to carry the `id` this now derives on the fly.
     `find.probe` groups by key rather than preserving input order, which costs
     nothing here: `sorted` re-sorts `[...matched]` immediately below. */
  const byPrefix = $derived(
    find.build(runs, (r) => `${r.underlying} ${r.feed} ${r.timeframe}`)
  );
  const matched = $derived(find.probe(byPrefix, runs, query));

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
      /* THE COLUMN IS A RUNTIME STRING, SO THE INDEX IS SPELLED AS ONE.
         `col` comes from `sortBy`, which the header buttons call with a
         `COLUMNS` key — it cannot be narrowed to `keyof Run` from here. This
         read as untyped while `matched` was the `searchable` spread; dropping
         that spread (see the filter above) put real `Run` objects in and made
         the dynamic index visible to the checker. The cast states what was
         already true rather than widening anything: `sort.col` is only ever
         set from `COLUMNS`, and every branch below tests the VALUE's type
         before comparing it. */
      const rec = /** @type {Record<string, unknown>} */ (
        /** @type {unknown} */ (a)
      );
      const rec2 = /** @type {Record<string, unknown>} */ (
        /** @type {unknown} */ (b)
      );
      const av = /** @type {any} */ (rec[col]);
      const bv = /** @type {any} */ (rec2[col]);
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
  /* `matched` AND NOT `runs`: THE DRILL BELONGS TO A ROW, AND MUST GO WITH IT.
     `runs` is the feed-filtered set; `matched` is what the table actually
     shows after the search box. Keyed on `runs`, the panel stayed fully
     mounted — chart, tester and all — while the table above it had been
     replaced by "Nothing matches …", so the page simultaneously said it had no
     such row and drew a thousand lines describing one.

     It is live now rather than theoretical: the ledger filter was dead until
     this session (`prefix.build` keyed on a field a `Run` has not), so nothing
     could empty `matched` and the divergence never showed. With the filter
     working, one keystroke opens it.

     The FEED path was already right — a run leaving `runs` nulls this and the
     series effect resets — so only the displayed-set path needed the change,
     and `matched` is the narrowest set that covers both. Not `sorted`: that
     would couple the drill to the sort order for nothing. */
  const openRun = $derived(matched.find((r) => r.index === openIndex) ?? null);

  /**
   * Did the open run never open a trade?
   *
   * ZERO BY ABSENCE IS NOT ZERO BY MEASUREMENT, and the drill-down renders
   * the two identically in a dozen places. This store's one run swept 903
   * bars and fired nothing, so every strategy-derived figure computes to
   * `0` — and `+0.00%` on a line labelled `Total return` says the strategy
   * broke even. It did not participate.
   *
   * The worst of them was `Strategy outperformance -33.09%`, an active
   * claim that the sweep LOST to buy and hold by a third. Nothing lost to
   * anything; one side never entered.
   *
   * Buy-and-hold is exempt wherever it appears: it is a property of the
   * bars on disk and stays true whether or not a trade was ever opened.
   */
  const noTrades = $derived(openRun !== null && openRun.trades === 0);

  /**
   * Exact trade-evidence state, shared by every neutral value.
   *
   * A null fold is not one fact: it can mean a request is still in flight, a
   * refusal, a valid empty response, an absent historical child, or a series
   * that is present but too short for a curve. Call sites use this state rather
   * than asserting that every null means “no trade file”.
   */
  const tradeEvidenceWhy = $derived.by(() => {
    if (tradeList.phase === 'loading') return 'Trade evidence is still being read.';
    if (tradeList.phase === 'failed') return tradeList.why || 'The trade response was refused.';
    if (tradeList.phase === 'ready' && tradeRows.length === 0) {
      if (noTrades) return 'The ledger records zero trades, so the set is known to be empty.';
      return tradeList.why || 'The validated trade response is empty.';
    }
    if (tradeRows.length === 1) {
      return 'One completed trade is present; at least two trades are required to form a curve.';
    }
    if (tradeList.phase === 'idle') return 'Open a run to read its chosen-grid trades.';
    return 'The required trade evidence is not available for this recorded run.';
  });

  const equityEvidenceWhy = $derived(
    tradeRows.length === 1
      ? 'One completed trade is an insufficient sample; a curve needs at least two trades.'
      : tradeEvidenceWhy
  );

  /**
   * Are the padlocked metrics expanded?
   *
   * # OPEN by default now, and the reason it was closed has gone
   *
   * It was closed because *"68% of its cells are locks"* and folding them put
   * the seven real rows where they could be read. That was true and is not any
   * more: the trade file made `Average profit`, `Average loss`, their ratio,
   * `Largest profit` and all three `Average bars in …` rows real, so most of
   * what the fold hid is now measured.
   *
   * The reference shows all twenty rows at once with no disclosure control at
   * all, and a fold that hides real numbers is worse than one that hides
   * padlocks. The control stays — a reader who wants the short list can still
   * collapse it — but the default is the reference's.
   */
  let showAllMetrics = $state(true);

  /** The control that opened the report, so Close can return keyboard focus. @type {HTMLElement | null} */
  let drillTrigger = null;

  /** Close the report and return focus to its opening control when it still exists. */
  async function closeDrill() {
    periodOpen = false;
    openIndex = null;
    // Invalidate both in-flight requests before either can publish under a
    // later run's heading. The calls reset their visible state synchronously.
    void fetchTrades(undefined);
    void fetchCombos(undefined);
    await tick();
    if (drillTrigger?.isConnected) drillTrigger.focus();
    drillTrigger = null;
  }

  /**
   * Open or close a run's drill-down, AND GO TO IT WHEN IT OPENS.
   *
   * MEASURED, WHICH IS THE ONLY REASON THIS IS NOT STILL A ONE-LINER.
   * `Drill in` on the answer card sits at y=517 in a 954px viewport. The
   * panel it opens starts at y=1011 -- **57px below the fold** -- and the
   * page did not scroll. So the only thing that changed on screen was the
   * button's own label flipping to `Close`, while **3,221px** of content --
   * a live candlestick chart off the store's own bars, the rung switcher,
   * the whole tester -- appeared entirely off-screen and the page silently
   * became four times longer.
   *
   * That is a fact present in the DOM and invisible on screen, which is
   * the failure `CLAUDE.md` §4 bans and the third time it has been found
   * on this page. A control that reports success by changing its own
   * caption and nothing else is indistinguishable from one that did
   * nothing.
   *
   * `await tick()` AND NOT `requestAnimationFrame`, measured. The first
   * version used a frame and did not scroll at all -- `.page.scrollTop`
   * stayed 0 with the panel still 1,011px down. Svelte flushes on a
   * microtask, a frame fires after that, and the panel was there by then;
   * what was NOT settled was its layout, so the scroll resolved against a
   * node with no height yet. `tick` is the framework's own answer to
   * "the DOM now matches the state".
   *
   * The container is scrolled EXPLICITLY rather than through
   * `scrollIntoView`. `theme.css` makes `body` and `.main` `overflow:
   * hidden` and this page owns its own scroller, so the ancestor walk
   * `scrollIntoView` does has three candidates and picked none of them.
   * Naming `.page` removes the guess.
   *
   * @param {any} run
   * @param {HTMLElement | null} [trigger]
   */
  async function toggle(run, trigger = null) {
    // Defense in depth behind every button. An unsafe row can still be
    // retained as visible metadata in the comparison table, but cannot become
    // `openRun` or trigger child requests even if a caller invokes this
    // function directly.
    if (!validateRunForComputation(run).ok) return;
    const opening = openRun?.index !== run.index;
    if (!opening) {
      await closeDrill();
      return;
    }
    // The answer and comparison controls sit outside the searchable ledger.
    // If that ledger's query currently excludes this run, clear the query so
    // `openRun` can resolve it instead of claiming the report expanded while
    // rendering no report at all.
    if (opening && !matched.some((row) => row.index === run.index)) query = '';
    periodOpen = false;
    plot = 'equity';
    performancePage = 0;
    swingPage = 0;
    tradePage = 0;
    drillTrigger = trigger;
    openIndex = run.index;
    // THE TRADES OF THIS RUN, fetched when it is opened rather than for every
    // row in the ledger. A run can hold tens of thousands of them and the list
    // is only ever looked at one run at a time.
    fetchTrades(run.identity);
    // AND THE RANKED COMBINATIONS. The ledger row is the ONE the exit grid
    // chose; these are the twenty-five behind it, with every measurement the
    // operator ranks on.
    fetchCombos(run.identity);
    await tick();
    const page = document.querySelector('.page');
    if (!page) return;

    /* THE PANEL EXISTS BEFORE IT HAS A SIZE, and that is what defeated the
       two previous attempts. Traced: `.drill` is in the DOM by 657ms, and
       `.page.scrollTop` never left 0 with either `requestAnimationFrame`
       or `await tick()`. Both fire while the panel is still EMPTY — its
       chart is waiting on `/bars/window.json` — so the scroll target was a
       collapsed node at the bottom of a page that had not grown yet, and
       the browser clamped the request to the scrollHeight of the moment.

       So the wait is on HEIGHT, not on a tick or a frame. Bounded at ~1.2s
       and it scrolls to whatever it has when the deadline passes: a slow
       fetch must not mean the page never moves, and arriving at a partly
       drawn panel still tells the reader where the click went. */
    const deadline = 1200;
    const started = performance.now();
    /** @returns {Promise<void>} */
    const settle = () =>
      new Promise((done) => {
        const look = () => {
          const panel = document.querySelector('.drill');
          const tall = panel !== null && panel.getBoundingClientRect().height > 200;
          if (tall || performance.now() - started > deadline) done();
          else requestAnimationFrame(look);
        };
        look();
      });
    await settle();

    const panel = /** @type {HTMLElement | null} */ (document.querySelector('.drill'));
    if (!panel) return;
    const delta = panel.getBoundingClientRect().top - page.getBoundingClientRect().top;
    /* ASSIGNED, NOT `scrollTo({behavior:'smooth'})`, AND THIS IS THE THIRD
       AND ACTUAL CAUSE. Measured on this container:

         page.scrollTo({top: 600, behavior: 'smooth'})  ->  scrollTop 0
         page.scrollTo({top: 600, behavior: 'auto'})    ->  scrollTop 600

       Smooth is a silent no-op here, so the version that asked for it did
       nothing at all and reported no error — the same shape as the dead
       `Clear all`, and found the same way, by measuring instead of reading
       the code and believing it.

       A plain assignment always moves. The animation moves to CSS
       (`scroll-behavior` on `.page`, behind a reduced-motion guard), where
       a browser that will not animate simply jumps and the reader still
       arrives. Motion is the part that may be dropped; the navigation is
       not. `- 8` keeps the panel's top edge off the rim of the scrollport
       so it reads as arriving rather than as clipped. */
    page.scrollTop = page.scrollTop + delta - 8;
    panel.focus({ preventScroll: true });
  }

  /* ====================================================================
     THE CURRENT STORE PRICE REFERENCE — TradingView's own library
     --------------------------------------------------------------------
     THE BARS ARE REAL AND THE TRADES ARE NOT DRAWN, and the second half
     of that sentence is the important one.

     `/bars/window.json` serves the OHLCV in the store NOW at the run's
     instrument, rung and span. It does not echo or prove the run's recorded
     `data_digest`, so this chart is a current-store reference and must never be
     described as the exact bytes the historical result used.

     Entries and exits are deliberately not overlaid. The chosen-trade file now
     owns exact execution-series indices and times for the same selected cell as
     the headline. This chart can show another rung and only a clipped window,
     while the row carries neither a chart-alignment key nor an exit-cause tag.
     Guessing either marker would still make a convincing wrong picture; the
     separate trade list shows every durable fact instead.
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

  /** @param {unknown} value */
  const integer = (value) => Number.isSafeInteger(value) && !Object.is(value, -0);
  /** @param {unknown} value */
  const whole = (value) => integer(value) && Number(value) >= 0;

  /** @param {any} run */
  function runMonthCount(run) {
    if (
      !whole(run?.from_year) ||
      !whole(run?.from_month) ||
      !whole(run?.to_year) ||
      !whole(run?.to_month) ||
      run.from_month < 1 ||
      run.from_month > 12 ||
      run.to_month < 1 ||
      run.to_month > 12
    ) {
      return null;
    }
    const span = (run.to_year - run.from_year) * 12 + run.to_month - run.from_month + 1;
    return Number.isSafeInteger(span) && span > 0 ? span : null;
  }

  /**
   * Admit the complete `/bars/window.json` contract before a single bar is
   * used by a chart or benchmark. This is intentionally one door shared by
   * both consumers: otherwise the benchmark can accept a value the series
   * refuses and the two displays disagree while each looks internally valid.
   *
   * @param {unknown} payload
   * @param {'asc'|'desc'} direction the wire order explicitly requested
   * @param {number} limit the exact request limit (offset is always zero here)
   * @param {number} expectedMonths the inclusive month range in the request
   * @returns {{ ok: true, bars: any[], total: number, monthsRead: number, monthsMissing: number, why: '' } | { ok: false, bars: never[], total: 0, monthsRead: 0, monthsMissing: 0, why: string }}
   */
  function validateBarsWindow(payload, direction, limit, expectedMonths) {
    /** @param {string} why */
    const refuse = (why) => ({
      ok: /** @type {const} */ (false),
      bars: /** @type {never[]} */ ([]),
      total: /** @type {const} */ (0),
      monthsRead: /** @type {const} */ (0),
      monthsMissing: /** @type {const} */ (0),
      why
    });
    if (payload === null || typeof payload !== 'object' || Array.isArray(payload)) {
      return refuse('the response is not an object');
    }
    const body = /** @type {Record<string, unknown>} */ (payload);
    if (!whole(body.total)) return refuse('total is not a non-negative safe integer');
    if (!whole(body.months_read)) {
      return refuse('months_read is not a non-negative safe integer');
    }
    if (!whole(body.months_missing)) {
      return refuse('months_missing is not a non-negative safe integer');
    }
    if (
      !Number.isSafeInteger(expectedMonths) ||
      expectedMonths < 1 ||
      Number(body.months_read) + Number(body.months_missing) !== expectedMonths
    ) {
      return refuse('months_read plus months_missing does not equal the requested span');
    }
    if (body.scanned !== false || body.extremes !== null || body.faults !== null) {
      return refuse('the timestamp window is scanned, partial, or carries unexpected extremes');
    }
    if (!Array.isArray(body.bars)) return refuse('bars is not an array');
    if (!Number.isSafeInteger(limit) || limit < 1 || body.bars.length > limit) {
      return refuse('bars exceeds the exact request limit');
    }
    const total = Number(body.total);
    if (body.bars.length !== Math.min(total, limit)) {
      return refuse('bars length does not reconcile with total and limit at offset zero');
    }

    /** @type {any[]} */
    const bars = [];
    let previous = null;
    for (let at = 0; at < body.bars.length; at += 1) {
      const raw = body.bars[at];
      if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
        return refuse(`bars[${at}] is not an object`);
      }
      const bar = /** @type {Record<string, unknown>} */ (raw);
      for (const field of ['t', 'o', 'h', 'l', 'c', 'v']) {
        if (!whole(bar[field])) return refuse(`bars[${at}].${field} is not a safe whole number`);
      }
      if (!(bar.oi === null || whole(bar.oi))) {
        return refuse(`bars[${at}].oi is neither null nor a safe whole number`);
      }
      if (
        Number(bar.h) < Number(bar.o) ||
        Number(bar.h) < Number(bar.l) ||
        Number(bar.h) < Number(bar.c) ||
        Number(bar.l) > Number(bar.o) ||
        Number(bar.l) > Number(bar.c)
      ) {
        return refuse(`bars[${at}] has impossible OHLC ordering`);
      }
      for (const [valueField, whyField] of [
        ['chg', 'chg_why'],
        ['oichg', 'oichg_why']
      ]) {
        const value = bar[valueField];
        const why = bar[whyField];
        if (value === null) {
          if (typeof why !== 'string' || why.length === 0) {
            return refuse(`bars[${at}].${whyField} does not explain a null ${valueField}`);
          }
        } else if (!integer(value) || why !== null) {
          return refuse(`bars[${at}].${valueField}/${whyField} is not an exclusive value/reason pair`);
        }
      }
      const timestamp = Number(bar.t);
      if (
        previous !== null &&
        ((direction === 'asc' && timestamp <= previous) ||
          (direction === 'desc' && timestamp >= previous))
      ) {
        return refuse(`bars[${at}].t is duplicated or out of requested ${direction} order`);
      }
      previous = timestamp;
      bars.push(raw);
    }
    return {
      ok: /** @type {const} */ (true),
      bars,
      total,
      monthsRead: Number(body.months_read),
      monthsMissing: Number(body.months_missing),
      why: /** @type {const} */ ('')
    };
  }

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
   * THE RUN'S FEED IS THE PARENT HERE, NOT THE PAGE'S. `catalogue` is loaded
   * for `feeds.active` (`+layout.svelte`), and this used it whatever run it
   * was asked about — while the request it feeds is built with `run.feed`. Turn
   * on "Show every feed" and a run from another vendor resolved its exchange
   * and segment out of the ACTIVE vendor's master, then asked its own vendor
   * for that path. Whether the two agree for a given symbol is a property of
   * two masters nobody compared; the wiring is wrong either way, and the
   * failure it produces — a 4xx from the bar route, or worse, a plausible file
   * that is the wrong series — is not one a reader could trace back to here.
   *
   * So it refuses instead. `null` is the answer this already has for "cannot
   * be resolved", and the caller's refusal arm names the reason.
   *
   * @param {string} symbol
   * @param {string} [forFeed] the feed the answer will be USED for
   */
  /* THE RULE ITSELF IS IN `$lib/place.js` SO A TEST CAN DRIVE IT — the same
     carve, for the same reason, that `$lib/pick.js` records for
     `feeds.svelte.js`: `node --test` cannot import a `.svelte` file, so a rule
     that lives in one is a rule nothing checks. `place.test.js` asserts the
     cross-feed refusal WITHOUT a populated ledger, which the runtime path
     needs and which needs a real sweep to produce. This wrapper is the page's
     two pieces of state and nothing else. */
  function place(symbol, forFeed) {
    return placeIn(catalogue.rows, catalogue.feed, symbol, forFeed);
  }

  /* ONE COUNTER PER LOADER: THE LAST ASKED WINS, NOT THE LAST TO RETURN.

     All three drill loaders assigned their result to page state with no
     ordering check. Open run A, then B before A's response lands, and A's
     slower answer overwrote B's — the panel then drew one run's bars, rungs or
     benchmark under another run's heading. The snippet design further down
     states that "a chart cannot silently render one panel's data under another
     panel's heading"; that held in the template and was defeated at the fetch.

     `$lib/index.svelte.js` already solved this and says why in the same words:
     "without this the last to RETURN wins instead of the last one ASKED, and
     the page answers confidently for the wrong broker."

     THREE COUNTERS AND NOT ONE, because all three run concurrently for the
     same open run — a single shared counter would have each cancel the other
     two. Each loader stamps its own call and drops only its own stale answers.
     The checks sit after the awaits; the writes before them (`idle`,
     `loading`) are synchronous and already correctly ordered. */
  let seriesSeq = 0;
  let rungsSeq = 0;
  let benchSeq = 0;

  /**
   * @param {any} run
   * @param {string} rung the timeframe to draw, which is not always the run's own
   */
  async function loadSeries(run, rung) {
    const seq = ++seriesSeq;
    const at = place(run.underlying, run.feed);
    if (!at) {
      series = {
        phase: 'failed',
        bars: [],
        total: 0,
        months_read: 0,
        months_missing: 0,
        /* THREE CAUSES NOW, AND THE NEW ONE GOES FIRST because it is the only
           one that is not about the instrument. A run from another vendor —
           reachable the moment "Show every feed" is on — cannot have its path
           resolved out of the ACTIVE vendor's master, and saying "not in the
           master" would blame the symbol for a mismatch of feeds. */
        why:
          catalogue.ready && catalogue.feed !== run.feed
            ? `This run is ${run.feed}'s and the instrument master loaded here is ` +
              `${catalogue.feed}'s. The exchange and segment a bar file is filed under come ` +
              `from the run's OWN vendor, so nothing was resolved rather than resolved from ` +
              `the wrong one. Choose ${run.feed} in the feed control to load its series.`
            : catalogue.ready
              ? `${run.underlying} is not in ${catalogue.feed}'s instrument master, so the ` +
                `exchange and segment its bar files are filed under cannot be resolved. The run ` +
                `is still shown above — only its price series cannot be located.`
              : `The instrument master for this feed has not loaded yet, so ${run.underlying}'s ` +
                `exchange and segment are not known. Nothing was guessed.`
      };
      return;
    }
    const expectedMonths = runMonthCount(run);
    if (expectedMonths === null) {
      series = {
        phase: 'failed',
        bars: [],
        total: 0,
        months_read: 0,
        months_missing: 0,
        why: 'The recorded run does not name a valid inclusive month span.'
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
      `&sort=ts&dir=desc&limit=${MAX_WINDOW_LIMIT}`;
    try {
      // 30 s rather than the default 15: 81 months of 30-minute bars is 21,620
      // records off a cold page cache, and giving up on a read that is working
      // would report a wedged server that is not one.
      const response = await ask_(url, { cache: 'no-store', ms: 30_000 });
      /* THE LAST ASKED WINS, NOT THE LAST TO RETURN — see `seriesSeq`. Open run
         A, then B before A's window lands, and A's slower answer used to
         overwrite B's: one run's bars under another run's heading, which is
         exactly what the snippet design three hundred lines down says it makes
         impossible, defeated one layer lower at the fetch. */
      if (seq !== seriesSeq) return;
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
      if (seq !== seriesSeq) return;
      const checked = validateBarsWindow(body, 'desc', MAX_WINDOW_LIMIT, expectedMonths);
      if (!checked.ok) {
        series = {
          phase: 'failed',
          bars: [],
          total: 0,
          months_read: 0,
          months_missing: 0,
          why: `The bar window was refused before charting: ${checked.why}.`
        };
        return;
      }
      // The wire was explicitly requested and admitted newest-first. Reversal
      // is therefore sufficient and cannot conceal a duplicate or unordered
      // timestamp the admission door should have refused.
      const bars = [...checked.bars].reverse();
      series = {
        phase: 'ready',
        bars,
        total: checked.total,
        months_read: checked.monthsRead,
        months_missing: checked.monthsMissing,
        why: ''
      };
    } catch (error) {
      if (seq !== seriesSeq) return;
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
    if (!run) {
      rungsSeq += 1;
      storeRungs = [];
      return;
    }
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
      seriesSeq += 1;
      series = { phase: 'idle', bars: [], total: 0, months_read: 0, months_missing: 0, why: '' };
      return;
    }
    const rung = chartRung || run.timeframe;
    void catalogue.ready; // re-resolve once the master lands
    untrack(() => {
      loadSeries(run, rung);
    });
  });

  // The benchmark QUERY is fixed to the run's own rung, so the chart-view
  // switcher cannot rewrite it. Its bytes are a CURRENT STORE reference: the
  // bars endpoint does not echo the run's recorded data_digest.
  $effect(() => {
    const run = openRun;
    if (!run) {
      benchSeq += 1;
      bench = { phase: 'idle', open: 0, close: 0, why: '' };
      return;
    }
    void catalogue.ready;
    untrack(() => loadBenchmark(run));
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
  /**
   * The bar under the crosshair, for the legend. Null when the pointer is off.
   *
   * FIVE FIELDS, WHICH IS WHAT THE LEGEND READS. `$state(null)` inferred
   * `null`, so the guarded `{#if ohlc}` narrowed to `never` and every `ohlc.c`
   * under it was a read on a type with no properties at all. Declaring more
   * than the legend uses would be describing the wire from memory rather than
   * from a reader.
   *
   * @type {{ t: number, o: number, h: number, l: number, c: number } | null}
   */
  let ohlc = $state(null);
  /** Which rung the CHART shows. Starts at the run's own and is switchable. */
  let chartRung = $state('');
  /**
   * Every rung the store holds for this instrument, newest census first.
   * @type {{ name: string, months: number }[]}
   */
  let storeRungs = $state([]);

  /**
   * The rungs the store actually holds for one instrument.
   *
   * **Discovered, never declared.** `store::path::Timeframe::KNOWN` can gain a
   * rung tomorrow; a list written here would be a list the store contradicts.
   * `/store.json` is the census `/db` already reads — one row per
   * (instrument, month, rung) — so the rung set is a fold over it.
   *
   * `rung` WAS DOCUMENTED AND WAS NEVER A PARAMETER. The doc described a
   * second argument — "the timeframe to draw, which is not always the run's
   * own" — that this function has never taken; the rung it draws is
   * `chartRung`, which the switcher writes. A parameter tag naming nothing is
   * a reader being told to pass something no caller passes.
   *
   * @param {any} run
   */
  async function loadRungs(run) {
    const seq = ++rungsSeq;
    try {
      const response = await ask_(`/store.json?feed=${encodeURIComponent(run.feed)}`, {
        cache: 'no-store',
        ms: 30_000
      });
      if (seq !== rungsSeq) return;
      if (!response.ok) return;
      const rows = await response.json();
      if (seq !== rungsSeq) return;
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
      // INTRADAY, PLUS THE RUN'S OWN RUNG WHATEVER IT IS.
      //
      // The `min` filter alone was a REGRESSION and it was measured: on a
      // `1day` run the chart draws 903 daily bars and the switcher offered
      // eight intraday rungs with **none of them active**. The reader was
      // shown eight buttons, none matching what was on screen, and any
      // click navigated away from the run's own rung with no way back.
      //
      // Which rungs may be SWEPT and which a chart may DISPLAY are two
      // different questions, and the filter answered the second with the
      // first. A run that exists was swept at some rung; refusing to draw
      // that rung does not un-record it, it just hides which bars are on
      // screen. So: every intraday rung, and always the open run's own.
      const own = String(run.timeframe ?? '');
      storeRungs = [...months.entries()]
        .filter(([name]) => /min$/.test(name) || name === own)
        .map(([name, count]) => ({ name, months: count }))
        .sort((a, b) => byRung(a.name, b.name));
    } catch {
      if (seq !== rungsSeq) return;
      // A census that will not load costs the SWITCHER and nothing else: the
      // chart still draws the run's own rung, which is the one that matters.
      storeRungs = [];
    }
  }

  /* ====================================================================
     BUY & HOLD — TradingView's headline comparison, and ours was missing
     --------------------------------------------------------------------
     This is a CURRENT-STORE REFERENCE, not a run-bound metric. Close of the
     current span's first stored bar against its last is buy-and-hold, but the
     bars route does not echo the run's data_digest. It cannot prove those are
     the historical bytes on which the persisted sweep result was computed.

     IT NEEDS ITS OWN REQUEST, and the reason is the 1,000-bar ceiling.
     `series` holds the NEWEST 1,000 bars of the span, so its first bar is
     three weeks old, not seven years. Buy-and-hold over the visible window
     is a different and much smaller number than buy-and-hold over the named
     full span, so the endpoint pair is fetched separately and labelled as
     current-store evidence everywhere it is used.

     ONE UNIT, AND IT IS STATED. The ledger records totals in paisa of
     index points with no position size and no capital, so the comparison
     is one unit of the index against one unit held. A percentage return
     would need initial capital, which nothing on disk carries.
     ==================================================================== */

  /** @type {{ phase: 'idle'|'loading'|'ready'|'failed', open: number, close: number, why: string }} */
  let bench = $state({ phase: 'idle', open: 0, close: 0, why: '' });

  /**
   * The closes of the span's current-store first and last stored bars.
   *
   * Both requests cover the whole recorded span so a sparse edge month does
   * not turn “first” into “missing”. `limit=1` plus an explicit timestamp
   * direction asks the indexed store for the endpoint itself; downloading a
   * 1,000-row tail and sorting it in the browser does not.
   *
   * @param {any} run
   */
  async function loadBenchmark(run) {
    const seq = ++benchSeq;
    const at = place(run.underlying, run.feed);
    if (!at) {
      bench = { phase: 'idle', open: 0, close: 0, why: '' };
      return;
    }
    const expectedMonths = runMonthCount(run);
    if (expectedMonths === null) {
      bench = {
        phase: 'failed',
        open: 0,
        close: 0,
        why: 'The recorded run does not name a valid inclusive month span.'
      };
      return;
    }
    bench = { phase: 'loading', open: 0, close: 0, why: '' };
    const from = `${run.from_year}-${String(run.from_month).padStart(2, '0')}`;
    const to = `${run.to_year}-${String(run.to_month).padStart(2, '0')}`;
    const base =
      `/bars/window.json?feed=${encodeURIComponent(run.feed)}` +
      `&exchange=${encodeURIComponent(at.exchange)}&segment=${encodeURIComponent(at.segment)}` +
      `&symbol=${encodeURIComponent(run.underlying)}` +
      `&timeframe=${encodeURIComponent(run.timeframe)}&from=${from}&to=${to}` +
      '&sort=ts';
    try {
      const [firstResponse, lastResponse] = await Promise.all([
        ask_(`${base}&dir=asc&limit=1`, { cache: 'no-store', ms: 30_000 }),
        ask_(`${base}&dir=desc&limit=1`, { cache: 'no-store', ms: 30_000 })
      ]);
      if (seq !== benchSeq) return;
      if (!firstResponse.ok || !lastResponse.ok) {
        bench = {
          phase: 'failed',
          open: 0,
          close: 0,
          why:
            `The span endpoints answered ${firstResponse.status}/${lastResponse.status}, ` +
            'so the current-store buy-and-hold reference has no price pair.'
        };
        return;
      }
      const [firstBody, lastBody] = await Promise.all([
        firstResponse.json(),
        lastResponse.json()
      ]);
      if (seq !== benchSeq) return;
      const first = validateBarsWindow(firstBody, 'asc', 1, expectedMonths);
      const last = validateBarsWindow(lastBody, 'desc', 1, expectedMonths);
      if (!first.ok || !last.ok) {
        bench = {
          phase: 'failed',
          open: 0,
          close: 0,
          why:
            'The benchmark bar window was refused before computation: ' +
            `${!first.ok ? first.why : last.ok ? '' : last.why}.`
        };
        return;
      }
      const opening = first.bars[0];
      const closing = last.bars[0];
      if (!opening || !closing) {
        bench = {
          phase: 'failed',
          open: 0,
          close: 0,
          why: `The store holds no ${run.timeframe} bars in ${from} through ${to}, so buy-and-hold has no price pair.`
        };
        return;
      }
      bench = { phase: 'ready', open: opening.c, close: closing.c, why: '' };
    } catch (error) {
      if (seq !== benchSeq) return;
      bench = {
        phase: 'failed',
        open: 0,
        close: 0,
        why: error instanceof Error ? error.message : String(error)
      };
    }
  }

  /**
   * Current-store buy-and-hold over the run's named span, in paisa.
   *
   * Both endpoints come from [`loadBenchmark`]'s own timestamp-indexed
   * requests at the run's recorded rung. Switching the display rung therefore
   * cannot rewrite the query. The endpoint does not bind those current bytes to
   * the run's data_digest, so every derived comparison remains explicitly a
   * current-store reference.
   */
  /* ====================================================================
     THE SERIES THE CHARTS ACTUALLY DRAW
     --------------------------------------------------------------------
     WHAT THESE ARE, SAID ONCE AND SAID PLAINLY: every curve, bar and
     column below is the CURRENT STORE's — one unit of the index, held. Not the
     strategy's. The strategy has no series on disk and cannot be given
     one, so drawing its equity curve is off the table permanently.

     The reference series is the closes of the current bars already loaded for
     the price chart. Cumulative P&L of holding, P&L per
     week, the distribution of per-bar returns, the run-up and drawdown
     segments — all of it folds out of `series.bars`. None is claimed to match
     the sealed run's data_digest.

     That is the difference between an empty frame and a full one, and it
     costs nothing in honesty as long as every plot says whose line it is.
     Each chart carries that label; none of them claims to be the strategy.
     ==================================================================== */

  /** Cumulative P&L of holding one unit, in paisa, bar by bar. */
  const holdCurve = $derived.by(() => {
    const bars = series.bars;
    if (bars.length < 2 || bench.phase !== 'ready') return [];
    // `series` is only the newest bounded chart window. Rebasing that tail to
    // its own first close made its last point disagree with the full-span
    // current-store buy-and-hold headline. Anchor every visible point to the
    // separately fetched current-store opening. Its headline endpoint remains
    // the separately fetched run-rung query even while another rung is viewed.
    return bars.map((b) => ({ t: b.t, v: b.c - bench.open }));
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
      // `t` is epoch seconds on the wire. Shift only for calendar projection;
      // the stored instant remains unchanged and timezone neutral.
      const d = new Date(t * 1000 + 19_800_000);
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
    /* NOTHING TO DISTRIBUTE IS `null`, NOT A HISTOGRAM WITH HALF ITS FIELDS.
       These two arms returned `{bins: [], max: 0, avgLoss: null, avgGain:
       null}` — no `lo`, no `hi`, no `losers`, no `winners` — so the value's
       type was a union in which the range was sometimes absent, and the panel
       that reads `h.lo` was only correct because its caller happened to guard
       on `bins.length`. An empty set of returns HAS no range: `lo: 0` would be
       a measurement nobody took. One value for "there is no distribution" says
       that, and the guard at the render site becomes the same question. */
    if (bars.length < 2) return null;
    const rets = [];
    for (const b of bars) {
      if (b.o > 0) rets.push(Math.round(((b.c - b.o) / b.o) * 10_000));
    }
    if (rets.length === 0) return null;
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
    /** @param {number[]} xs */
    const mean = (xs) =>
      xs.length === 0 ? null : Math.round(xs.reduce((a, b) => a + b, 0) / xs.length);
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
   * a property of the CURRENT STORE PRICE — a reference's growth and decline,
   * not the strategy's and not a data-digest-bound historical replay.
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
    if (bench.phase !== 'ready' || bench.open <= 0) return null;
    const gain = exactIntegerDelta(bench.close, bench.open);
    const bps = roundedScaledRatio(gain, bench.open, 10_000);
    if (gain === null || bps === null) return null;
    return {
      from: bench.open,
      to: bench.close,
      gain,
      bps
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
    const edge = exactIntegerDelta(openRun.pessimistic, buyHold.gain);
    if (edge === null) return null;
    return {
      strategy: openRun.pessimistic,
      hold: buyHold.gain,
      edge,
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
    const worst = roundedScaledRatio(openRun.pessimistic, openRun.trades, 1);
    const best = roundedScaledRatio(openRun.optimistic, openRun.trades, 1);
    if (worst === null || best === null) return null;
    return {
      worst,
      best
    };
  });

  /* ====================================================================
     THE STRATEGY TESTER — TradingView's analysis hierarchy, Brutex facts
     --------------------------------------------------------------------
     The supplied tester screenshots contain no padlock inside the report;
     the visible locks belong to TradingView's separate drawing toolbar.
     A report lock therefore invents a permission/paywall meaning that the
     recorded data does not have. Rows keep their order, but unavailable
     arithmetic and absent evidence use neutral text/dashes with the reason.
     Known zeroes and inapplicable account metrics say exactly that.
     ==================================================================== */

  /** Which of the tester's three views is showing. */
  let testerView = $state('metrics');
  /**
   * The period scale on every periodic chart. TradingView's four.
   * @type {(typeof PERIOD_SCALES)[number]}
   */
  let periodScale = $state('weekly');
  /** "Profits and losses" split. TradingView's two. */
  let plSplit = $state('signals');
  /** Whether the testing-period menu is open. */
  let periodOpen = $state(false);
  /** @type {HTMLElement | null} */
  let periodRoot = $state(null);
  /** @type {HTMLButtonElement | null} */
  let periodTrigger = $state(null);
  /** @type {HTMLElement | null} */
  let periodDialog = $state(null);

  /** Open the non-modal period dialog and put focus on its current choice. */
  async function togglePeriodMenu() {
    if (periodOpen) {
      periodOpen = false;
      return;
    }
    periodOpen = true;
    await tick();
    const current = periodDialog?.querySelector('button:not(:disabled)');
    if (current instanceof HTMLButtonElement) current.focus();
  }

  /** @param {boolean} [restoreFocus] */
  async function closePeriodMenu(restoreFocus = false) {
    periodOpen = false;
    if (!restoreFocus) return;
    await tick();
    if (periodTrigger?.isConnected) periodTrigger.focus();
  }

  /** @param {MouseEvent} event */
  function dismissPeriodOnOutsideClick(event) {
    if (
      periodOpen &&
      periodRoot &&
      event.target instanceof Node &&
      !periodRoot.contains(event.target)
    ) {
      periodOpen = false;
    }
  }

  /** @param {FocusEvent} event */
  function dismissPeriodOnFocusOut(event) {
    if (
      periodOpen &&
      periodRoot &&
      (!(event.relatedTarget instanceof Node) || !periodRoot.contains(event.relatedTarget))
    ) {
      periodOpen = false;
    }
  }

  /** @param {KeyboardEvent} event */
  function closePeriodOnEscape(event) {
    if (!periodOpen || event.key !== 'Escape') return;
    event.preventDefault();
    void closePeriodMenu(true);
  }
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
  let taTab = $state('distribution');

  /**
   * Run P&L normalised by the CURRENT STORE opening close, in basis points.
   *
   * The ledger records totals in paisa of index points and no capital, so
   * "return" here is the total against the current store price one unit cost at
   * the span's start — the only denominator available to this page. The bars
   * endpoint does not bind it to the run's data_digest; stated wherever it shows.
   */
  const strategyBps = $derived.by(() => {
    if (!openRun || bench.phase !== 'ready' || bench.open <= 0) return null;
    return roundedScaledRatio(openRun.pessimistic, bench.open, 10_000);
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
    const result = Math.round((total ** (1 / spanYears) - 1) * 10_000);
    return Number.isSafeInteger(result) ? result : null;
  });

  /** Max drawdown against the span's opening price, in basis points. */
  const drawdownBps = $derived.by(() => {
    if (!openRun || bench.phase !== 'ready' || bench.open <= 0) return null;
    return roundedScaledRatio(Math.abs(openRun.max_drawdown), bench.open, 10_000);
  });

  /** A difference of two exact basis-point endpoints may itself be inexact. */
  const outperformanceBps = $derived(
    strategyBps === null || buyHold === null
      ? null
      : exactIntegerDelta(strategyBps, buyHold.bps)
  );

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

  /**
   * THE FOUR SCALES, WRITTEN ONCE.
   *
   * This list was an inline array literal at two `{#each}` sites and its
   * labels were a third object here, so the four names existed in three
   * places and a fifth scale would have had to be added to all of them. It is
   * also what makes `PERIOD_LABEL[periodScale]` legal: indexing a four-key
   * object with a `string` is an error precisely because `string` is not one
   * of the four, and the fix is for `periodScale` to say that it is.
   */
  const PERIOD_SCALES = /** @type {const} */ (['daily', 'weekly', 'quarterly', 'yearly']);

  /** The heading TradingView puts above each periodic chart. */
  const PERIOD_LABEL = {
    daily: 'Daily',
    weekly: 'Weekly',
    quarterly: 'Quarterly',
    yearly: 'Yearly'
  };

  /**
   * The same four as a NOUN, for a sentence that buckets "by" one of them.
   *
   * The empty-state message built this with `periodScale.replace('ly', '')`,
   * which is correct for three of the four and renders the fourth as
   * **"bucket by dai"** — `'daily'` has no `ly` to strip except the one inside
   * the word. A four-entry map cannot be wrong about a fifth case because there
   * is not one.
   */
  const PERIOD_NOUN = {
    daily: 'day',
    weekly: 'week',
    quarterly: 'quarter',
    yearly: 'year'
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
   * One plotted value. `pnl` is present only on a resolved strategy trade.
   *
   * @typedef {{ t?: number, v: number, pnl?: number }} Point
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
   * The shared vertical extent of a curve and any per-point PnL bars,
   * symmetric so both use the same visible zero and currency axis.
   *
   * @param {Point[]} points
   */
  function curveScale(points) {
    return points.reduce(
      (largest, point) => Math.max(largest, Math.abs(point.v), Math.abs(point.pnl ?? 0)),
      1
    );
  }

  /** At most seven exact timestamps across a plotted curve window. @param {Point[]} points */
  function curveLabels(points) {
    if (points.length === 0) return [];
    const count = Math.min(7, points.length);
    return Array.from({ length: count }, (_, index) => {
      const at = count === 1 ? 0 : Math.round((index * (points.length - 1)) / (count - 1));
      const stamp = points[at]?.t;
      return typeof stamp === 'number' && Number.isFinite(stamp) ? istLabel(stamp, true) : '';
    });
  }

  /**
   * The polyline through a cumulative series.
   *
   * @param {Point[]} points
   * @param {number} scale
   */
  function linePath(points, scale) {
    const resolvedScale = Math.max(1, scale);
    const step = 1000 / Math.max(1, points.length - 1);
    return points
      .map((p, i) => `${i === 0 ? 'M' : 'L'}${(i * step).toFixed(1)} ${(130 - (p.v / resolvedScale) * 120).toFixed(1)}`)
      .join(' ');
  }

  /**
   * The same polyline, closed to the zero line, for the area fill.
   *
   * @param {Point[]} points
   * @param {number} scale
   */
  function areaPath(points, scale) {
    const step = 1000 / Math.max(1, points.length - 1);
    return `${linePath(points, scale)} L${((points.length - 1) * step).toFixed(1)} 130 L0 130 Z`;
  }

  /**
   * The tallest bar in a set, so a column chart shares one scale.
   *
   * @param {Bar[]} bars
   */
  function barScale(bars) {
    return bars.reduce((largest, bar) => Math.max(largest, Math.abs(bar.v)), 1);
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
   * The same, UNSIGNED — for a share of a whole rather than a change.
   *
   * A `+` belongs on a figure that could have gone the other way: a return, an
   * outperformance, a P&L. It is meaningless on a proportion. The reference
   * writes `31.87%` for profitable trades and `43.90% winners` for the best
   * hour, never `+31.87%`, because no share of a total is ever negative and the
   * sign carries no information.
   *
   * @param {number | null | undefined} bps
   */
  const share = (bps) =>
    bps === null || bps === undefined ? '—' : `${(bps / 100).toFixed(2)}%`;

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

  /**
   * The drill-down's local palette.
   *
   * The reference is one dark analysis surface even when the surrounding
   * browser chrome is light. Reading the chart host rather than `:root` keeps
   * the candlesticks on the same red/teal/grid ramp as that surface.
   */
  function chartTokens() {
    const s = getComputedStyle(chartHost ?? document.documentElement);
    /**
     * @param {string} name a CSS custom property
     * @param {string} fallback used when the theme does not define it
     */
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

  /**
   * The chart handle, kept so the range presets can drive the time scale.
   * @type {any} matching the loose handles on `/` — see the note there.
   */
  let chartApi = null;
  /** True only after the chart handle exists, so range controls never become enabled no-ops. */
  let chartReady = $state(false);

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
    /** @type {any} the chart handle, same looseness as `chartApi` above. */
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
            tickMarkFormatter: (/** @type {unknown} */ time, /** @type {number} */ type) =>
              istLabel(Number(time), type <= 2)
          },
          // The crosshair's own time label, same clock.
          localization: {
            locale: 'en-IN',
            timeFormatter: (/** @type {unknown} */ time) => `${istLabel(Number(time), false)} IST`
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
        made.subscribeCrosshairMove((/** @type {{ time?: unknown } | undefined} */ param) => {
          const at = param?.time == null ? null : byTime.get(Number(param.time));
          ohlc = at ?? null;
        });

        made.timeScale().fitContent();
        chartApi = made;
        chartReady = true;
        chartError = '';
      } catch (why) {
        // A chart library that will not load is a NAMED failure, not a blank
        // rectangle. Every figure above this panel still stands.
        /* `catch` BINDS `unknown`. The same guard `loadVocab` already uses
           above, for the same reason: `?.message` on a thrown non-Error is
           `undefined` and falls through, but on an object whose `message` is
           not a string it prints `[object Object]` as the failure reason. */
        if (!dead) {
          chartReady = false;
          chartError = why instanceof Error ? why.message : String(why);
        }
      }
    })();
    return () => {
      dead = true;
      chartApi = null;
      chartReady = false;
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
   * One trade leg's moment, in the shape TradingView's List of trades prints it.
   *
   * # Why a second formatter and not [`when`]
   *
   * `when` is `en-IN`, which resolves `hour: '2-digit'` to a TWELVE-hour clock
   * with `am`/`pm`. The reference prints `Aug 28, 2026, 14:12` — twenty-four
   * hour, no meridiem — and a trades table is read by scanning a column of
   * times for the gap between two of them, which a meridiem suffix makes
   * slower, not clearer. `hour12: false` is the whole difference.
   *
   * # This column was printing a bar INDEX
   *
   * `/trades.json` has been sending `entry_micros` and `exit_micros` since
   * `cli::trades` wrote them — MEASURED on this store, `1584074700000000`,
   * a real March 2020 timestamp. The column headed *"Date and time"* rendered
   * `bar {exact(t.exit_bar)}` regardless: an ordinal into the bar file, under a
   * header promising a date. The index is still worth having, so it moves to
   * the cell's `title` rather than being dropped.
   *
   * @param {number} micros
   */
  function tradeWhen(micros) {
    if (!Number.isFinite(micros) || micros <= 0) return 'not stamped';
    // `en-US` FOR THE ORDER, `IST` FOR THE CLOCK. The reference prints
    // `Aug 28, 2026, 14:12` — month first. `en-IN` orders it day-first and
    // gives `28 Aug 2026, 14:12`, which is the right convention for this
    // operator's locale and the wrong one for matching the reference. The time
    // ZONE is unaffected and stays Asia/Kolkata: this changes the order of the
    // fields, never which moment they name.
    return new Date(micros / 1000).toLocaleString('en-US', {
      timeZone: IST,
      year: 'numeric',
      month: 'short',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      hour12: false
    });
  }

  /**
   * A paisa figure as a percentage of the span's opening price.
   *
   * The reference prints every money column as two lines — `450.8 INR` above,
   * `0.31%` below — because a figure in currency says nothing about whether it
   * was a large move without a base to divide by. `bench.open` is that base:
   * the current store's first close over the run's named span, already fetched
   * for the buy-and-hold reference. The bars route does not bind it to the
   * run's recorded data_digest.
   *
   * `null` when the base is not loaded yet or is zero. NOT `0` — a ratio with
   * no base is undefined, and `0.00%` reads as "measured, and flat".
   *
   * @param {number|null|undefined} paisa
   * @returns {string|null}
   */
  function shareOfOpen(paisa) {
    const base = bench.open;
    if (!Number.isFinite(paisa) || !Number.isFinite(base) || !base) return null;
    return `${((Number(paisa) / Number(base)) * 100).toFixed(2)}%`;
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
  /** @param {{ months_asked: number, months_found: number }} r */
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

<svelte:window onclick={dismissPeriodOnOutsideClick} onkeydown={closePeriodOnEscape} />

<svelte:head>
  <title>Backtest · brutex</title>
  <meta
    name="description"
    content="Compare recorded brutex runs, inspect conservative fill results, and open the complete backtest evidence for each run."
  />
</svelte:head>

<!-- ============================================================
     A CHART FRAME WITH NO SERIES IN IT.
     Axes, gridlines, the zero line, the legend and the pager are all
     drawn, because they are part of the design and they are all TRUE --
     the shape of the plot is not in doubt, only its contents. The plot
     area carries the sentence saying which field the sweep would have to
     write for the series to appear.

     Drawing the frame rather than hiding the panel keeps the thing in its
     place while the reason remains legible instead of invisible.
     ============================================================ -->

<!-- ============================================================
     THE DRAWN CHARTS.
     Each takes a real series folded out of the bars on disk and each
     carries a label saying whose series it is. Bar-clock series are the
     BENCHMARK's one-unit hold; the strategy's separate series comes from the
     exact chosen-grid trades and advances when each trade resolves.
     ============================================================ -->

<!-- ══ THE COMBINATION'S NAMES, ON EVERY RANKED ROW ══
     This is the thing the operator asked for more times than anything else on
     this page — "no name has been given as i asked you what's the combination
     topped" — and the thing neither ranked table showed. `/frontier.json` has
     always sent `mask_words` on every row; nothing read them. The single
     winning mask was decoded, in the Properties tab, two clicks from the
     default view, which is why the answer existed and was never seen.

     Retired bits are shown rather than filtered: the mask CARRIES them, and a
     combination that reads as three conditions when it was recorded on four is
     a different claim about what won. -->
<!-- ══ ONE SELECTED CELL, ONE DURABLE DETAIL STREAM ══
     D-0414 replaced the legacy level-less `trades.bin` policy. The current
     child replays the exact finally admitted grid cell, including its own
     stop/target/trailing exits and exclusivity, and refuses unless its full
     Cell plus independently folded count/sums/worst/max-drawdown reconcile.
     The policy and selected direction live in the sealed receipt even when the
     exact cell takes zero trades. -->
{#snippet chosenGridTradeNote()}
  <p class="tt-warnnote">
    <b>Exact chosen-grid rows.</b> The sealed <code>{tradeList.policy ?? 'chosen-grid-v1'}</code>
    detail replays the finally admitted <b>{tradeList.direction ?? 'selected'}</b> cell, including
    its stop, target, trailing exits, and one-position-at-a-time exclusivity. Its rows are published
    only after their count, both P&amp;L sums, worst trade, max drawdown, and complete grid cell
    reconcile with the headline result.
  </p>
{/snippet}

{#snippet conditionNames(/** @type {any} */ words)}
  {@const decoded = decodeMaskWords(words)}
  {@const bits = decoded.positions}
  {#if !decoded.ok}
    <span class="cnames dim"><b>Combination cannot be decoded.</b> {decoded.why}</span>
  {:else if vocab.phase === 'failed'}
    <span class="cnames dim">Shown as raw positions — {vocab.why}: {bits.join(' · ')}</span>
  {:else if bits.length === 0}
    <span class="cnames dim">No condition bits are set on this row.</span>
  {:else}
    <span class="cnames">
      {#each bits as position, i (position)}{#if i > 0}<span class="cdot"> · </span>{/if}{@const bit =
          vocab.bits.get(position)}{#if !bit}<span class="cunk"
            >position {position} — not named by this vocabulary</span
          >{:else if !bit.live}<span class="cret">{bit.name} <i>retired</i></span>{:else}<span
            class="cname">{bit.name}</span
          >{/if}{/each}
    </span>
  {/if}
{/snippet}

<!-- ══ DOES THIS ROW MEET THE OPERATOR'S RULES ══
     `record_frontier` writes the top `top` by ranking lens and consults NO rule,
     so every row arrived looking like a candidate. The verdict is computed in
     `cli` and served on the row — it is not recomputed here, because a
     threshold copied into JavaScript is the second definition CLAUDE.md §5
     refuses.

     `stop` is shown as UNCHECKED rather than as a pass. `Rules::max_mae_ppm` is
     judged on `Cell::worst_mae`, which a row does not store; drawing a tick for
     it would be a claim about a number nobody wrote down. -->
{#snippet verdictPill(/** @type {any} */ meets)}
  {#if !meets}
    <span class="vp vp-none" title="This server does not send a verdict. Rebuild and restart it."
      >—</span
    >
  {:else if !meets.priced && meets.priced !== undefined}
    <span class="vp vp-unpriced" title="Never met an exit grid — screen_cap cut it before pricing."
      >unpriced</span
    >
  {:else if meets.all}
    <span class="vp vp-pass" title="Every checkable rule met. The stop rule is not checked.">PASS</span>
  {:else}
    {@const failed = [
      !meets.win_rate && 'win rate',
      !meets.reward_to_risk && 'reward:risk',
      !meets.return_over_drawdown && 'return/drawdown',
      !meets.trades && 'trade count',
      !meets.assurance && 'assurance'
    ].filter(Boolean)}
    <span class="vp vp-fail" title="Fails: {failed.join(', ')}. The stop rule is not checked."
      >FAIL <i>{failed.length}</i></span
    >
  {/if}
{/snippet}

<!-- THE DATA PARAMETERS BIND TO THE TYPE OF WHAT IS PASSED, with `typeof`,
     rather than to a shape restated here. A snippet is called from one place
     with one series; a second spelling of that series' shape is a second
     thing to update when the derivation that builds it changes, and the one
     that would not be updated is this one. -->
{#snippet areaChart(
  /** @type {Point[]} */ points,
  /** @type {string[]} */ ticks,
  /** @type {string[]} */ xLabels,
  /** @type {string} */ note,
  /** @type {number} */ verticalScale
)}
  {@const losing = (points[points.length - 1]?.v ?? 0) < 0}
  {@const resolvedScale = Math.max(1, verticalScale)}
  <div class="cf">
    <div class="cf-plot tall">
      <svg class="cf-svg" viewBox="0 0 1000 260" preserveAspectRatio="none" aria-hidden="true">
        {#each [0, 65, 130, 195, 260] as y (y)}
          <line x1="0" y1={y} x2="1000" y2={y} stroke="var(--n5)" stroke-width="1" vector-effect="non-scaling-stroke" />
        {/each}
        {#if points.some((point) => point.pnl !== undefined)}
          <!-- One exact bar per resolved trade, centred around its own zero
               line like the TradingView reference. The curve below is the
               cumulative sum of these same values. -->
          <line x1="0" y1="130" x2="1000" y2="130" stroke="var(--n7)" stroke-width="1" vector-effect="non-scaling-stroke" />
          {#each points as point, i (i)}
            {#if point.pnl !== undefined}
              {@const width = 1000 / Math.max(1, points.length)}
              {@const height = (Math.abs(point.pnl) / resolvedScale) * 120}
              <rect
                x={i * width + width * 0.12}
                y={point.pnl >= 0 ? 130 - height : 130}
                width={Math.max(0.55, width * 0.76)}
                height={Math.max(0.7, height)}
                fill={point.pnl >= 0 ? 'var(--up)' : 'var(--down)'}
                opacity="0.72"
              />
            {/if}
          {/each}
        {/if}
        {#if points.length > 1}
          <path
            d={areaPath(points, resolvedScale)}
            fill={losing
              ? 'color-mix(in srgb, var(--down) 16%, transparent)'
              : 'color-mix(in srgb, var(--up) 14%, transparent)'}
          />
          <path
            d={linePath(points, resolvedScale)}
            fill="none"
            stroke={losing ? 'var(--down)' : 'var(--up)'}
            stroke-width="2"
            vector-effect="non-scaling-stroke"
            stroke-linejoin="round"
          />
        {/if}
      </svg>
      <div class="cf-axis">{#each ticks as t, ti (ti)}<span>{t}</span>{/each}</div>
    </div>
    <div class="cf-x">{#each xLabels as x, xi (xi)}<span>{x}</span>{/each}</div>
    <p class="cf-note">{note}</p>
  </div>
{/snippet}

{#snippet barChart(
  /** @type {typeof holdPeriods} */ bars,
  /** @type {string[]} */ ticks,
  /** @type {string} */ note,
  /** @type {string[]} */ legend
)}
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
      <div class="cf-axis">{#each ticks as t, ti (ti)}<span>{t}</span>{/each}</div>
    </div>
    <div class="cf-x">
      {#each barLabels(bars) as x (x)}<span>{x}</span>{/each}
    </div>
    {#if legend}
      <ul class="cf-legend">
        {#each legend as l, i (i)}<li><span class="cf-sw s{i}"></span>{l}</li>{/each}
      </ul>
    {/if}
    <p class="cf-note">{note}</p>
  </div>
{/snippet}

{#snippet histogram(
  /** @type {NonNullable<typeof returnHistogram>} */ h,
  /** @type {string} */ note
)}
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

{#snippet swingChart(/** @type {typeof holdSwings} */ sw, /** @type {string} */ note)}
  {@const swingWindow = windowSeries(sw.segs, swingPage, SWING_PAGE_SIZE)}
  {@const visibleMax = sw.max}
  <div class="cf">
    <div class="cf-plot">
      <svg class="cf-svg" viewBox="0 0 1000 200" preserveAspectRatio="none" aria-hidden="true">
        {#each [0, 50, 100, 150, 200] as y (y)}
          <line x1="0" y1={y} x2="1000" y2={y} stroke="var(--n5)" stroke-width="1" vector-effect="non-scaling-stroke" />
        {/each}
        {#each swingWindow.rows as s, i (swingWindow.start + i)}
          {@const w = 1000 / Math.max(1, swingWindow.rows.length)}
          {@const h = (s.size / visibleMax) * 90}
          <rect
            x={i * w + w * 0.16}
            y={s.up ? 100 - h : 100}
            width={w * 0.68}
            height={Math.max(1, h)}
            fill={s.up ? 'var(--up)' : 'var(--down)'}
            stroke={s.current ? 'var(--warn)' : 'none'}
            stroke-width={s.current ? '2' : '0'}
            vector-effect="non-scaling-stroke"
            rx="1"
          />
        {/each}
      </svg>
      <div class="cf-axis">
        <span>{money(visibleMax)}</span><span>{money(Math.round(visibleMax / 2))}</span><span>0</span><span>{money(-Math.round(visibleMax / 2))}</span><span>{money(-visibleMax)}</span>
      </div>
    </div>
    <ul class="cf-legend">
      <li><span class="cf-sw s0"></span>Run-up</li>
      <li><span class="cf-sw s1"></span>Drawdown</li>
      <li><span class="cf-sw s3"></span>Current open stretch</li>
    </ul>
    {#if swingWindow.pages > 1}
      <div class="series-nav" aria-label="Swing pages">
        <button
          aria-label="Earlier swings"
          disabled={swingWindow.page === 0}
          onclick={() => (swingPage = Math.max(0, swingWindow.page - 1))}>‹</button
        >
        <span
          >Swings {exact(swingWindow.start + 1)}–{exact(
            Math.min(swingWindow.start + swingWindow.rows.length, swingWindow.total)
          )} of {exact(swingWindow.total)}</span
        >
        <button
          aria-label="Later swings"
          disabled={swingWindow.page + 1 >= swingWindow.pages}
          onclick={() => (swingPage = Math.min(swingWindow.pages - 1, swingWindow.page + 1))}>›</button
        >
      </div>
    {/if}
    <p class="cf-note">{note}</p>
  </div>
{/snippet}
<!-- `legend` TAKES TWO FORMS AND ALWAYS HAS. The body reads `l.label ?? l`
     and `l.dash` and `l.value`, which is written exactly so a caller can pass
     a bare string OR a row with a dashed swatch and a trailing figure — and
     both kinds of caller exist. Typing it `string[]`, as a first pass did,
     was not a tightening: it was a claim that contradicted the `?? l` two
     lines into the snippet it described. -->
{#snippet chartFrame(
  /** @type {string} */ why,
  /** @type {Array<string | { label: string, dash?: boolean, value?: string }>} */ legend,
  /** @type {string[]} */ ticks,
  /** @type {string[]} */ xLabels,
  /** @type {boolean} */ pager,
  /** @type {'undefined' | 'unavailable'} */ kind
)}
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
        {#each ticks as t, ti (ti)}<span>{t}</span>{/each}
      </div>
      <div class="cf-msg"><Lock {kind} /> <span>{why}</span></div>
    </div>
    {#if xLabels.length > 0}
      <div class="cf-x">
        {#each xLabels as x, xi (xi)}<span>{x}</span>{/each}
      </div>
    {/if}
    {#if legend.length > 0}
      <ul class="cf-legend">
        <!-- NORMALISED ONCE, INSTEAD OF `l.label ?? l` AT THREE READS. The
             two forms are widened to the richer one at the top of the body,
             so everything below reads one shape and the string case is
             handled in exactly one place rather than implied by a `??` that
             a reader has to decode three times. -->
        {#each legend as raw, i (i)}
          {@const l =
            typeof raw === 'string'
              ? /** @type {{ label: string, dash?: boolean, value?: string }} */ ({ label: raw })
              : raw}
          <li class:dash={l.dash}>
            <span class="cf-sw s{i}" class:dashed={l.dash}></span>{l.label}{#if l.value}<b>{l.value}</b>{/if}
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
      <!-- "1 runs shown" IS WHAT A SCREEN READER WAS SAYING, and it is the
           one sentence on this page nobody sighted ever proofreads. The
           count is announced on every ledger read; a page holding exactly
           one run announced it ungrammatically every time. The not-swept
           total is announced too, because a reader who cannot see the strip
           has no other route to it. -->
      {exact(runs.length)}
      {runs.length === 1 ? 'run' : 'runs'} shown, {exact(completeRuns.length)} complete, {exact(
        haltedRuns.length
      )} halted{#if offSurfaceRuns.length > 0}, {exact(offSurfaceRuns.length)} on a timeframe the engine never sweeps{/if}.
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
    <!-- WHICH OF THE TWO BARS HOLDS THE SLOT. There are two commands below
         this line now and one `Progress` slot on the server for both, so "a
         run is going" is no longer enough to know what is going. -->
    <span class="runbar-k">
      New sweep
      {#if inFlight && runKind === 'sweep'}<b class="rk-live">· running</b>{/if}
    </span>
    <!-- ══ THE FEED, FIRST, AND ON THIS PAGE RATHER THAN IN THE TOP BAR ══

         `+layout.svelte`'s `FEED_OWNED` states the protocol: exactly one
         control for one value on every route, and a route joins that list ON
         THE DAY IT GROWS A CONTROL OF ITS OWN. This is that control, so
         `/backtest` joins the list in the same commit — drawing it here while
         the bar still drew it too would be the two-controls-for-one-value
         defect the comment names, which is worse than none.

         FIRST, because it is first in the cascade and not merely first in the
         layout. A frequency run's identity is `blake3(mask ‖ Undirected ‖
         instrument ‖ timeframe ‖ params ‖ data_digest ‖ vocab_version ‖ commit
         ‖ feed)`. The finally selected long/short direction is sealed result
         metadata, not something the browser can reverse out of that hash. The
         feed decides the bars every one of those terms is measured over: the
         instruments below are whichever ones this feed's store holds, and the
         ledger is filtered to runs stamped with it. Reading the strip left to
         right now follows the order the answers actually depend on each other,
         which is what /ingest and /db already do.

         IT MEETS THE CONDITION FOR JOINING. `/db` qualified because it draws
         its rung in every state — the failed read, the unchosen feed, the
         empty store — and a control that vanishes with its data is the dead
         end the list exists to prevent. `.bt-shell` is an unconditional child
         of `.page` with no branch between, so this renders on the loading, the
         failed, the refusal, the no-feed, the empty-ledger and the ready
         state alike.

         `single`, and the same `Picker` /db and /ingest use — the feed is a
         SELECTION, exactly one, never a set. `detail` carries the fact THIS
         page owns: how many runs the ledger holds under each feed, the way
         /db's rung carries held cells. A refused feed is drawn, dead, and
         quotes `/feeds.json`'s own reason rather than paraphrasing it. ══ -->
    <div class="runf">
      <span>feed</span>
      <Picker
        single
        filter={feeds.all.length > 8}
        label="feeds"
        title={`Every run below is this feed's. A sweep is stamped with the feed its bars came from, so changing this changes which runs the ledger shows and which instruments the sweep can be built over — it is not a different view of one set.${feeds.error ? ` The feed list itself could not be read: ${feeds.error}.` : ''}`}
        summary={feeds.error
          ? 'Feed list unread'
          : feeds.all.length === 0
            ? 'No feeds'
            : (feeds.all.find((f) => f.wire === feeds.active)?.display ?? 'Select a feed')}
        rows={feeds.all.map((f) => {
          const ready = f.ready === true;
          /* COUNTED OFF THE LEDGER THIS PAGE ALREADY HOLDS — `allRuns` is the
             whole file before the feed filter, so this is a measurement and
             not a second request. Zero runs is a measured zero and says so. */
          const held = allRuns.filter((r) => r.feed === f.wire).length;
          return {
            key: f.wire,
            name: f.display,
            detail: !ready
              ? 'unavailable'
              : load.phase !== 'ready'
                ? 'ledger not read'
                : `${exact(held)} run${held === 1 ? '' : 's'}`,
            disabled: !ready,
            why: ready
              ? undefined
              : (f.why ??
                'the server marked this feed not ready and stated no reason, which is itself the thing to fix.'),
            title: ready
              ? `Scopes this page to ${f.display}. The ledger shows the runs stamped with it, and a new sweep is built over the instruments its store holds.`
              : `${f.display} is refused by the server, and this is /feeds.json's own reason rather than a paraphrase of it: ${f.why ?? 'no reason stated.'}`
          };
        })}
        selected={new Set([feeds.active])}
        onchange={(/** @type {Set<string>} */ sel) => {
          /* `if (next)` AND NOT AN UNGUARDED WRITE: `Picker` in `single` mode
             always emits exactly one key, so an empty set cannot arrive here —
             but writing `feeds.active = undefined` if it ever did would clear
             the scope for the whole product from a control that never meant
             to. Clearing the feed is the top bar's job, deliberately. */
          const next = [...sel][0];
          if (next) feeds.active = next;
        }}
      />
    </div>
    <!-- A PICKER, NOT A FREE-TEXT BOX. The set of instruments is a fact the
         census already states; typing one lets an operator name something the
         store does not hold and learn about it from a refusal a minute later.
         While the census is loading the control says so rather than offering
         an empty list that looks like "none". -->
    <!-- INSTRUMENTS ARE A SET, SO THE CONTROL IS A SET. A `<select>` made a
         single choice the only representable one; picking two meant two
         visits. Chips show every instrument the store holds at once, and a
         single choice is a set of one -- so there is no mode to switch. -->
    <!-- BOTH CHOICES ARE MENUS, and both are MULTI. `$lib/Picker.svelte` is
         already the console's multi-select dropdown -- checkboxes, a filter, a
         Select all, and a row that can be drawn dead with its reason on it --
         and `/ingest` uses it for exactly this shape of choice. A row of chips
         put the choice on the page instead of in a control, which is a
         different thing from the rest of this console and gets wider with
         every instrument the store gains.

         Each row carries its own coverage in `detail`, so the menu answers
         "what does the store hold for this one" without being opened twice. -->
    <div class="runf">
      <span>instruments</span>
      {#if catalog.phase === 'ready' && catalog.held.length > 0}
        <Picker
          filter={catalog.held.length > 8}
          label="instruments"
          summary={pickedSymbols.size === 1
            ? ([...pickedSymbols][0] ?? '—')
            : `${exact(pickedSymbols.size)} of ${exact(catalog.held.length)}`}
          title="Every spot index this feed holds on disk. Pick one or several; the run route takes one at a time today."
          rows={catalog.held.map((h) => ({
            key: h.leaf,
            name: h.leaf,
            detail: `${exact(h.months)} months · ${monthLabel(h.from)} – ${monthLabel(h.to)}`,
            title: h.full
          }))}
          selected={pickedSymbols}
          onchange={(/** @type {Set<string>} */ next) => {
            // THE EMPTY SET IS ACCEPTED, AND THE FIRST VERSION REFUSED IT HERE.
            // `if (next.size === 0) return;` made "Clear all" a control that
            // silently did nothing while its label said otherwise -- which is
            // the fallback-that-hides-a-failure §4 bans, and worse than the
            // state it was avoiding. An empty selection is a legal thing to
            // want on the way to picking something else. The Run control is
            // what refuses it, in words, where the operator is looking.
            pickedSymbols = next;
            symbolTouched = true;
            spanTouched = false;
            /* THE RUNG LATCH RELEASES WITH THE SPAN LATCH. Which timeframes
               exist is a property of the instrument, so a new one must re-seed
               them — holding the latch across a switch would leave the
               previous instrument's selection, or an empty menu, standing over
               a different series. Cleared here and nowhere else, exactly like
               `spanTouched`. */
            rungsTouched = false;
          }}
        />
      {:else}
        <span class="runf-wait">{catalog.phase === 'loading' ? 'reading census…' : '—'}</span>
      {/if}
    </div>
    <div class="runf">
      <span>timeframes</span>
      {#if heldNow}
        <Picker
          label="timeframes"
          summary={pickedRungs.size === heldNow.rungs.length
            ? `all ${exact(heldNow.rungs.length)}`
            : `${exact(pickedRungs.size)} of ${exact(heldNow.rungs.length)}`}
          title="Every timeframe this instrument holds on disk. Execution is always one-minute, whichever are picked."
          rows={heldNow.rungs.map((r) => ({
            key: r.name,
            name: r.name,
            detail:
              r.months < heldNow.months
                ? `${exact(r.months)} months · ${exact(heldNow.months - r.months)} short`
                : `${exact(r.months)} months`,
            why:
              r.months < heldNow.months
                ? 'fewer months on disk than the instrument holds, so a sweep here is a shorter sample rather than a corrected one.'
                : undefined,
            title: `${monthLabel(r.from)} – ${monthLabel(r.to)}`
          }))}
          selected={pickedRungs}
          onchange={(/** @type {Set<string>} */ next) => {
            pickedRungs = next;
            /* THE THIRD LATCH, and it was the missing one. `symbolTouched` and
               `spanTouched` exist because a default that keeps reasserting
               itself is a control the operator does not own; the rung seed had
               no such latch and keyed on `pickedRungs.size === 0` alone —
               which is exactly the state "he just cleared them".

               The seed effect tracks `spanTouched`. So: Clear all here, then
               pick a month in `from`, and `spanTouched` flips false -> true,
               the effect re-runs, the set is still empty, and ALL EIGHT
               TIMEFRAMES COME BACK with no notice. Editing a control two rungs
               DOWN the strip silently rewrote the one above it. A change of
               feed or instrument did the same, by the same route. */
            rungsTouched = true;
          }}
        />
      {:else}
        <!-- WHICH TIMEFRAMES EXIST IS A PROPERTY OF THE INSTRUMENT, so with
             none chosen there is no list to offer. It says that rather than
             rendering an empty menu, which would read as "this instrument
             holds nothing". -->
        <span class="runf-wait">
          {catalog.phase === 'loading'
            ? 'reading census…'
            : pickedSymbols.size === 0
              ? 'pick an instrument'
              : '—'}
        </span>
      {/if}
    </div>
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
          onchange={(/** @type {Set<string>} */ next) => {
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
          onchange={(/** @type {Set<string>} */ next) => {
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
         silently." The engine holds NO percentage at all: D-0303 deleted the
         `const SUPPORT_PPM = 200_000` this comment used to describe, and
         `sweeprun::conduct` passes `None`, so each rung derives its own floor
         from its own bars through `cli::statistical_support_floor`. A support
         named in the request body is IGNORED rather than obeyed -- there is a
         test called exactly that.

         THIS COMMENT SAID 20% UNTIL 2026-08-28, and so did the sentence under
         the button. Measured against the run the operator started that day:
         every rung was handed 28 hits, which on the 1-minute rung's 617,921
         bars is 0.0045% -- not 20%, and not within four orders of magnitude of
         it. A control that quietly does something other than what it shows is
         the failure §4 bans, and this page was showing the wrong thing. -->
    <!-- DISABLED WHEN THE LEDGER CANNOT TAKE THE RESULT. A sweep that cannot
         record is EIGHT rungs of real work thrown away, and the refusal
         arrives AFTER it. `blocked` is the server's own answer -- see
         `ledgerBlock`.

         EIGHT AND NOT NINE, because this counts what is SWEPT rather than what
         is on disk: `cli::EVERY_RUNG` is the eight `*min` rungs and the store
         holds nine, `1day` among them to feed `indicators::daily`. The line
         under the button already said eight; this one had not been corrected
         with it. -->
    <!-- ============ THE ENGINE'S OWN KNOBS ============
         Every one of these lived in the SERVER'S PROCESS ENVIRONMENT until
         2026-08-29, which meant the only way to change how a sweep searched was
         to restart the server, and the only way to see how it HAD searched was
         to read that process's environment from outside it.

         MEASURED that day: a server started from the IDE with none of them set
         gave `screen_cap` 10,000 and `validate` ON -- a pair that composes into
         a run which does not finish. The button below was reachable. The
         configuration was not.

         Blank means ABSENCE, and the stated fallback is what the server will
         then decide. Pre-filling would put fourteen terms into the run identity
         the operator never chose. -->
    <details class="eng">
      <summary class="eng-sum">
        Engine settings
        <span class="eng-n">
          {#if knobCount === 0}
            every knob left to the server
          {:else}
            <b>{knobCount}</b> set for this run
          {/if}
        </span>
      </summary>
      <div class="eng-grid">
        {#each KNOB_FIELDS as k (k.key)}
          <label class="eng-row">
            <span class="eng-lab">{k.label}</span>
            <input
              class="eng-in"
              name={`engine-${k.key}`}
              type="text"
              inputmode="numeric"
              placeholder={k.fallback}
              bind:value={engine[k.key]}
              aria-label={k.label}
            />
            {#if k.note}<span class="eng-note">{k.note}</span>{/if}
          </label>
        {/each}
      </div>
      <p class="eng-foot">
        These reach the engine as its own <code>BRUTEX_*</code> names and are written to the audit
        trail before the run starts, under <code>api.sweep · knobs set for this run</code>. They also
        enter the run identity, so two runs at different settings are two runs and never overwrite
        each other. <b>The store root and the log directory are deliberately not settable here</b> —
        a request that could move them would make every provenance banner on this page a claim about
        a directory nobody chose.
      </p>
    </details>

    <button
      class="btn run"
      onclick={startSweep}
      disabled={sweep.phase === 'starting' ||
        sweep.phase === 'running' ||
        !activeFeed ||
        blocked !== null ||
        pickedSymbols.size === 0 ||
        pickedRungs.size === 0}
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
      {:else if pickedSymbols.size === 0}
        <!-- THE REFUSAL THE MENUS USED TO MAKE SILENTLY. `Clear all` is a real
             control and it now really clears; this is where that state is
             named, beside the button it disables. -->
        <b class="warnish">No instrument selected.</b> Pick at least one — a brute force needs
        something to run over.
      {:else if pickedRungs.size === 0}
        <b class="warnish">No timeframe selected.</b> Pick at least one — every one of them was
        cleared.
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
        timeframe, at a support each timeframe <b>derives from its own bars</b> —
        no fixed percentage is sent or held.
        {#if pickedSymbols.size > 1 || (heldNow && pickedRungs.size < heldNow.rungs.length)}
          <b class="warnish">The rest of the selection is not sent yet</b> — the run route takes one
          instrument and no timeframe list.
        {/if}
        <!-- THE SPAN GUARD MOVED HERE WHEN THE COVERAGE SECTION WENT.
             It is the one thing in that section an operator acts on, and it
             exists because a hardcoded `2019-12` once cost 40 of 121 months
             while the run still reported `81/81` with no hole. Losing it with
             the section would have re-opened exactly that hole -- so it sits
             in the line that already says what the press will do, which is
             where it is read. -->
        {#if spanShortfall > 0 && askedMonths !== null && heldNow}
          <b class="warnish">
            Asking for {exact(askedMonths)} of {exact(heldNow.months)} months
          </b>
          — {exact(spanShortfall)} on disk will not be swept.
          <button class="linky" onclick={useWholeSpan}>Use the whole span</button>
        {:else if askedMonths !== null && spanShortfall < 0 && heldNow}
          <b class="warnish">
            Asking for {exact(askedMonths)} months, more than the {exact(heldNow.months)} on disk
          </b>
          — the months that do not exist are simply absent from the sample.
        {/if}
        {#if heldNow && rungsDiffer}
          <b class="warnish">Not every timeframe holds the same history</b> — the short ones are a
          shorter sample, not a corrected one.
        {/if}
      {/if}
    </span>
  </section>

  <!-- ================================================================
       DESCEND ONE RUNG — the command this console could not reach.

       `POST /backtest/descend` was registered, handled, tested and had no
       caller in this browser: the only work the operator could start was
       `/backtest/run`, which sweeps every selected rung at a threshold each
       one derives from its own bars. That is the right question for *which
       timeframe carries the edge* and the wrong one for *is there a rare
       setup here at all* — `cli::elite_descend`'s own doc records that a run
       launched at a twentieth of 200,000 ppm "cannot report a once-a-week
       setup no matter how long it runs", and a once-a-week cadence is about
       3,656 ppm.

       BESIDE THE BAR ABOVE AND NOT INSIDE IT, because they are two commands
       and not two settings of one. They share the feed, the instrument and
       the span — those are facts about what is being studied, and one copy of
       each is the point — and they take the SAME slot, so only one can be in
       flight. The label says which is running.
       ================================================================ -->
  <section class="runbar">
    <span class="runbar-k">
      Descend one rung
      {#if inFlight && runKind === 'descent'}<b class="rk-live">· running</b>{/if}
    </span>

    <!-- ONE RUNG, FROM THE CENSUS, NEVER FROM A LIST IN THIS FILE.
         The rows are whatever `/store.json` says this instrument holds, in the
         census's own order — the same source the TIMEFRAMES menu above reads.
         `cli::EVERY_RUNG` is the engine's list and it is EIGHT; the store holds
         nine, and no endpoint publishes the eight. Keeping a copy of them here
         is the two-vocabularies failure CLAUDE.md §5 exists to refuse: correct
         the day it is written, silently wrong the first time a rung is added.

         So a rung the engine does not sweep is OFFERED with the fact on it, and
         the route answers it by name — `descent_from` refuses and lists the
         eight in the refusal, which is rendered verbatim below. The shape test
         is `swept`, the same one the ledger table uses, and it is a rule about
         the NAME rather than a second copy of the const. -->
    <div class="runf">
      <span>timeframe</span>
      {#if heldNow}
        <Picker
          single
          label="timeframe"
          summary={descent.rung || 'Pick one'}
          title="The one rung this descent walks. A descent spends its whole budget on a single timeframe, which is what buys the lower threshold — so this is a choice, not a filter."
          rows={heldNow.rungs.map((r) => ({
            key: r.name,
            name: r.name,
            detail: `${exact(r.months)} months`,
            why: swept({ timeframe: r.name })
              ? undefined
              : 'the engine sweeps the intraday rungs only — this one is on disk to feed the daily indicators. The route will refuse it by name and list the ones it does sweep.',
            title: `${monthLabel(r.from)} – ${monthLabel(r.to)}`
          }))}
          selected={new Set(descent.rung ? [descent.rung] : [])}
          onchange={(/** @type {Set<string>} */ next) => {
            const [m] = [...next];
            if (m) descent.rung = m;
          }}
        />
      {:else}
        <span class="runf-wait">
          {catalog.phase === 'loading'
            ? 'reading census…'
            : pickedSymbols.size === 0
              ? 'pick an instrument'
              : '—'}
        </span>
      {/if}
    </div>

    <!-- POINTS, AND THE UNIT IS ON THE CONTROL BECAUSE IT HAS BEEN GOT WRONG.
         `descent_from` takes `max_points` and states the rule in its own
         comment: `cli` converts it against the midpoint of the span's OWN
         bars, "and this side never sees a ppm, which is the whole reason that
         entry point exists". A browser that sent a ppm would be answering a
         question only the bars can settle. -->
    <div class="runf">
      <span>stop ceiling</span>
      <label class="dnum" class:wrong={descent.points !== '' && descentPoints === null}>
        <input
          class="dnum-in"
          name="descent-stop-ceiling-points"
          type="text"
          inputmode="numeric"
          autocomplete="off"
          spellcheck="false"
          placeholder="whole number"
          bind:value={descent.points}
          aria-label="stop ceiling, in whole index points"
          title="The widest adverse excursion this run may accept, in INDEX POINTS. Never a ppm and never paisa — the engine converts it against the midpoint of this span's own bars."
        />
        <span class="dnum-u">pts</span>
      </label>
    </div>

    <div class="runf">
      <span>list top</span>
      <label class="dnum" class:wrong={descent.top !== '' && descentRows === null}>
        <input
          class="dnum-in"
          name="descent-ranked-row-count"
          type="text"
          inputmode="numeric"
          autocomplete="off"
          spellcheck="false"
          placeholder="whole number"
          bind:value={descent.top}
          aria-label="how many ranked rows to list"
          title="How many ranked rows the report lists. Zero is refused by the route: a listing of no rows is not a shorter answer, it is no answer."
        />
        <span class="dnum-u">rows</span>
      </label>
    </div>

    <!-- DISABLED FOR ONE NAMED REASON AT A TIME — see `descentStop`. It is
         disabled while ANY run is in flight, sweep or descent, because both
         append to the same append-only ledger and the server refuses the
         second press rather than queueing it. A button that can only earn a
         409 is not a button. -->
    <button
      class="btn run"
      onclick={startDescent}
      disabled={inFlight || descentStop !== null}
    >
      {#if pressed === 'descent' && sweep.phase === 'starting'}Starting…{:else if pressed === 'descent' && sweep.phase === 'running'}Descending…{:else}Descend{/if}
    </button>

    <span class="runbar-n">
      {#if descentStop === 'ledger'}
        this ledger cannot take a result — see the note above the table
      {:else if descentStop === 'feed'}
        choose a feed first — a descent is stamped with the feed its bars came from
      {:else if descentStop === 'census'}
        {catalog.phase === 'failed'
          ? catalog.why
          : 'reading what is on disk — the rung and the span come from the census, not from this page'}
      {:else if descentStop === 'instrument'}
        <b class="warnish">No instrument selected.</b> Pick one above — a descent runs over the same
        instrument the sweep bar names.
      {:else if descentStop === 'span'}
        <b class="warnish">No span chosen.</b> The two month menus above set it, and a descent uses
        the same one — a span is two of the nine terms in the run's identity.
      {:else if descentStop === 'rung'}
        <b class="warnish">No timeframe chosen.</b> A descent walks <b>one</b> rung's threshold
        down, so this is required rather than defaulted — that single rung is what buys the lower
        floor.
      {:else if descentStop === 'points'}
        <b class="warnish">The stop ceiling is not a whole number of points.</b> It is the widest
        adverse excursion this run may accept, in <b>index points</b> — not a ppm and not paisa.
      {:else if descentStop === 'rows'}
        <b class="warnish">The list length is not a whole number of rows.</b> Zero is refused by the
        route: a listing of no rows is no answer.
      {:else}
        this press descends <b>{sweepSymbol}</b> on <b>{activeFeed}</b> at
        <b>{descent.rung}</b>, walking the support threshold <b>down</b> from a ceiling of
        <b>{exact(descentPoints ?? 0)} points</b> and listing the top
        <b>{exact(descentRows ?? 0)}</b>.
        <b>The Engine settings above are not sent with it</b> — `conduct_descent` applies no knobs,
        so spreading them here would let that panel imply it had configured a run it never touched.
        It appends to the <b>same ledger</b> as the sweep and takes the same slot, so only one of
        the two can be running.
      {/if}
    </span>
  </section>


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

  <!-- A PAGE THAT COULD NOT ASK IS NOT A PAGE WITH NO RUN. Rendering the idle
       console silently over a live sweep is the fallback §4 bans. -->
  {#if adoptWhy && sweep.phase === "idle"}
    <p class="inline-note bad runstate">{adoptWhy}</p>
  {/if}
  {#if sweep.phase === 'running'}
    <!-- IT IS NO LONGER INDETERMINATE, AND THIS TABLE IS WHY.
         The note below still says the TOTAL is unknowable, which is true — the
         ladder walks until the frontier empties. What was ALSO unknowable, and
         did not have to be, is which of the eight timeframes had loaded, what
         threshold each derived from its own bars, and which had finished. Two
         overnight runs held thirteen cores for seven hours with no way to tell
         a working sweep from a hung one. The events carried it; nothing asked. -->
    {#if live.phase === 'ready' && live.rungs.length > 0}
      <div class="tt-tblwrap">
        <table class="tt-tbl rungprog">
          <thead>
            <tr><th>timeframe</th><th class="num">bars</th><th class="num">needs</th><th class="num">support</th><th class="num">candidates</th><th>state</th></tr>
          </thead>
          <tbody>
            {#each live.rungs as r (r.key)}
              <tr>
                <td><b>{r.rung}</b>{#if r.validating} <span class="dim">validation</span>{/if}</td>
                <td class="num">{r.bars ? r.bars.toLocaleString() : '—'}</td>
                <td class="num">{r.minHits ? r.minHits.toLocaleString() : '—'}</td>
                <td class="num">
                  {r.bars && r.minHits ? `${((r.minHits / r.bars) * 100).toFixed(2)}%` : '—'}
                </td>
                <td class="num">{r.candidates ? r.candidates.toLocaleString() : '—'}</td>
                <td>
                  <!--
                    FIVE STATES, NOT TWO. This read `recorded / refused /
                    sweeping…`, so a rung four hours into its exit grid rendered
                    "sweeping…" — identical to one that had just loaded its span.
                    The grid is 87.6% of a run's wall clock, measured, so that
                    one word covered the overwhelming majority of every run and
                    said nothing about it.
                  -->
                  {#if r.done && r.recorded}<span class="ok">recorded</span>
                  {:else if r.done}<span class="warnish">refused — {r.why || 'no reason given'}</span>
                  {:else if r.phase === 'priced'}<span class="dim"
                      >priced {r.priced.toLocaleString()} — validating…</span
                    >
                  {:else if r.phase === 'pricing'}<span class="dim">pricing the exit grid…</span>
                  {:else}<span class="dim">sweeping…</span>{/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if live.phase === 'failed'}
      <p class="inline-note bad runstate">{live.why}</p>
    {/if}

    <!-- The TOTAL is still indeterminate, and saying so. A sweep is 43 ms to
         hours depending on the rung; a percentage bar over the whole run would
         be a guess. The table above is measured; the sentence below is honest
         about what is not. -->
    <!-- IT NAMES THE COMMAND NOW, AND IT HAD TO. Two controls append to one
         append-only ledger through one slot, so this banner covers both — and
         a descent rendered under "Sweeping" would be labelling a hunt for a
         rare setup as an EIGHT-rung comparison, which is `sweeprun::Kind`'s
         own stated reason for putting the word on the wire at all — that doc
         says "nine-rung", and it is counting the store rather than
         `EVERY_RUNG`, which is eight. `runKind`
         prefers the payload's `kind` and falls back to what this tab pressed,
         which is the only reading available in the second before the first
         poll answers. -->
    <p class="inline-note runstate">
      <span class="spin sm" aria-hidden="true"></span>
      {#if runKind === 'descent'}
        <b>Descending.</b> One rung, with the support threshold walked <b>down</b> from the ceiling
        given in points — a full screen at each step rather than one, which is why it is slower per
        rung than the sweep and why it can see a setup the sweep prunes in its first level. How long
        it takes is decided by the data, not by a setting.
      {:else}
        <b>Sweeping.</b> The ladder walks upward until the frequent frontier empties, so how long
        it takes is decided by the data rather than by a setting — a one-day span finishes in
        milliseconds and a fine rung over seven years takes minutes per month.
      {/if}
      This page is polling and will refresh itself the moment a record lands.
      {#if sweep.run}
        <span class="dim">· started {when(sweep.run.started_micros)}</span>
      {/if}
    </p>
    <!-- THE BEST ROWS FOUND SO FAR, WHICH THE SERVER HAS ALWAYS SENT AND THIS
         PAGE HAS NEVER SHOWN.

         `cli::live` has kept a bounded top-`keep` heap by |t| since it was
         added, and `/live.json` has served it; nothing here fetched it. A sweep
         running for hours therefore showed a spinner and nothing else, and the
         one question worth asking DURING a run -- has anything cleared the bar
         yet -- had no surface at all.

         `clears_bar` IS RENDERED BESIDE THE RANK, and that pairing is the whole
         point. `runner::rank` orders by |t| and its own doc says it "does not
         bless them"; a table showing rank alone would read as a result. The bar
         itself rises with the number of hypotheses tested, so a row that leads
         this table can still be nothing. -->
    {#if liveTop.phase === 'ready' && liveTop.rows.length > 0}
      <div class="livetop">
        <p class="livetop-head">
          <b>Best so far</b>
          <span class="dim">
            · {exact(liveTop.trials)} weighed · bar |t| ≥ {(liveTop.barMilli / 1000).toFixed(2)}
            {#if liveTop.stale}· <span class="pill warn">stale</span>{/if}
          </span>
        </p>
        <div class="livetop-scroll">
          <table class="livetop-table">
            <thead>
              <tr>
                <th>#</th><th>side</th><th>hits</th><th>trades</th>
                <th>|t|</th><th>mean paisa</th><th>payoff</th><th>wins</th><th>clears bar</th>
              </tr>
            </thead>
            <tbody>
              {#each liveTop.rows as row (row.rank)}
                <tr class={row.clears_bar ? 'clears' : ''}>
                  <td>{row.rank}</td>
                  <td>{row.direction}</td>
                  <td>{exact(row.hits)}</td>
                  <td>{exact(row.n)}</td>
                  <td>{(Math.abs(row.t_milli) / 1000).toFixed(3)}</td>
                  <td>{(row.mean_milli_paisa / 1000).toFixed(1)}</td>
                  <td>{row.payoff_bp} bp</td>
                  <td>{exact(row.edge_wins)}</td>
                  <td>{row.clears_bar ? 'YES' : 'no'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="livetop-foot dim">
          Ordered by <b>|t|</b> — absolute, so a setup that precedes a fall ranks beside one that
          precedes a rise; the sign is in <b>mean paisa</b>. Rank is not a finding: a row counts
          only where <b>clears bar</b> says YES, and that bar rises with every hypothesis the run
          tests. Rewritten only when the top actually moves.
        </p>
      </div>
    {:else if liveTop.phase === 'failed'}
      <p class="inline-note warn">
        The best-so-far rows could not be read: {liveTop.why}
      </p>
    {/if}
  {:else if sweep.phase === 'done' && sweep.run}
    <!-- THE COMMAND'S OWN NAME, from the payload's `kind`. This read "Sweep
         finished" for every ended run, and with a second writer on the same
         slot that sentence would have printed over a descent — a different
         question, a different threshold rule, and the same green tick. -->
    <p class="inline-note runstate good">
      <b>{runLabel} finished</b> — {sweep.run.underlying} on {sweep.run.feed}, and the ledger below
      has been re-read.
    </p>
    <!-- THE EIGHT-RUNG TABLE, WHICH THE SERVER HAS ALWAYS SENT AND THIS PAGE
         HAS NEVER SHOWN.

         EIGHT, and this line said nine while the paragraph fourteen lines
         below it already said "the eight-rung SUMMARY". One block, two counts.
         `range_all` prints one row per rung it swept and `cli::EVERY_RUNG` is
         eight; the ninth timeframe is on disk, not in the sweep.

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

    <!-- THE RANKED COMBINATIONS, WHICH IS THE ANSWER THE SWEEP WAS RUN FOR.
         The block above is the eight-rung SUMMARY — one line per timeframe. The
         ledger table below is one row per RUN, carrying the single winning
         combination. Between them sat the thing actually being looked for: the
         top twenty-five per rung, with hits, observations, mean, `t`, payoff,
         win count and their conditions NAMED.
         Those rows have been written to `results/frontier.bin` all along and
         `/engine/top.json` has been able to read them all along. No page ever
         called it, so twenty-four of every twenty-five results were invisible. -->
    {#if top.phase === 'ready'}
      <p class="inline-note runstate">
        <b>The ranked combinations</b> — best first, conditions named.
      </p>
      <pre class="runlog">{top.report}</pre>
    {:else if top.phase === 'loading'}
      <p class="inline-note runstate dim">Reading the ranked combinations…</p>
    {:else if top.phase === 'failed'}
      <!-- SHOWN, NOT SWALLOWED. A page that quietly renders nothing here looks
           exactly like a sweep that ranked nothing, and those are opposite
           facts. -->
      <p class="inline-note bad runstate">
        <b>The ranked combinations could not be read.</b>
        {top.why}
      </p>
    {/if}
  {:else if sweep.phase === 'failed'}
    <!-- WHICH COMMAND WAS REFUSED, and this is the arm where the payload most
         often cannot say: a malformed month, a dead fetch and a 409 from the
         slot all land here with `sweep.run` null, so `runKind` falls back to
         what this tab pressed. A refusal attributed to the wrong control is a
         reason the operator cannot act on. -->
    <p class="inline-note bad runstate">
      <b>The {runLabel.toLowerCase()} was refused.</b>
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
    <!-- `{:else if ledger}` AND NOT A BARE `{:else}`, AND THE NARROWING IS
         LOAD-BEARING RATHER THAN COSMETIC.

         `ledger` is `Ledger | null | undefined` (`:482`) and this branch
         dereferences it bare — `ledger.total`, `ledger.hit_scan_cap`,
         `ledger.scanned`, `ledger.partial_tail`. The checker could not tie
         `load.phase` to `load.body`, so those read as possible throws, and a
         blanket `ledger?.` would have silenced the report while leaving the
         hole: `refusal` is `ledger?.refusal ?? null` (`:486`), so a null
         ledger yields a FALSY refusal, falls straight past the arm above, and
         lands here — where the first read throws.

         Traced before changing it: every writer of `load` sets `body: null`
         only alongside `phase: 'failed'`, and the 503 path deliberately does
         not early-return because the server sends a full object with
         `refusal` set. So no code reaches it today. The gap is real in the
         types and shut here rather than left to the next writer to rediscover
         — and the arm below names the state instead of rendering blank. -->
  {:else if ledger}
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
      <!-- ONE DENOMINATOR ACROSS THE ROW. This tile read `ledger.total` — the
           WHOLE file, every feed — while the three beside it read
           `completeRuns`, `haltedRuns` and `holedRuns`, all of which fold
           `runs`, the feed-FILTERED set. On a store whose active feed holds 3
           of 500 records the strip said "Runs recorded 500 · Complete 3 ·
           Halted 0 · Span holes 0" as one row of four, and the only way to
           know the first number was counted differently was to already know.
           The whole-file figure is not lost: it moves to this tile's own note,
           where the scan cap already lives, and it appears only when the two
           actually differ. -->
      <div class="fact rise">
        <span class="k">Runs recorded</span>
        <span class="v">{exact(runs.length)}</span>
        <span class="n">
          {#if ledger.hit_scan_cap}
            showing the newest {exact(ledger.scanned)} — the read stopped at its ceiling
          {:else if runs.length !== ledger.total}
            of {exact(ledger.total)} in the file — the rest are under another feed
          {:else}
            all of them read
          {/if}
        </span>
      </div>
      <div class="fact rise" class:good={completeRuns.length > 0}>
        <span class="k">Complete</span>
        <span class="v">{exact(completeRuns.length)}</span>
        <span class="n">
          {#if unsealedRuns.length > 0}
            ran to extinction AND read back intact
          {:else}
            finished result records
          {/if}
        </span>
      </div>
      <div class="fact rise" class:warn={haltedRuns.length > 0} class:nil={haltedRuns.length === 0}>
        <span class="k">Halted</span>
        <span class="v">{exact(haltedRuns.length)}</span>
        <span class="n">
          {haltedRuns.length === 0
            ? 'no ladder was cut short'
            : 'excluded from ranking — depth is partial'}
        </span>
      </div>
      <div class="fact rise" class:warn={holedRuns.length > 0} class:nil={holedRuns.length === 0}>
        <span class="k">Span holes</span>
        <span class="v">{exact(holedRuns.length)}</span>
        <span class="n">
          {holedRuns.length === 0 ? 'every span was whole' : 'a shorter sample, not a corrected one'}
        </span>
      </div>
      <!-- A COUNT THAT ONLY EXISTS WHEN IT IS NOT ZERO. Every other cell in
           this strip is a permanent axis of the ledger; a run on a
           non-signal timeframe is an anomaly, and a permanent `0` beside the
           others would imply it is a dimension anyone should expect to
           populate. It appears when the ledger holds one and says what it
           means, and is absent otherwise. -->
      {#if offSurfaceRuns.length > 0}
        <div class="fact warn rise">
          <span class="k">Not swept</span>
          <span class="v">{exact(offSurfaceRuns.length)}</span>
          <span class="n">
            on a timeframe the engine never sweeps — listed, never ranked
          </span>
        </div>
      {/if}
    </section>

    {#if ledger.partial_tail}
      <p class="inline-note warn">
        The last bytes of the ledger are not a whole record — a writer was interrupted mid-append.
        Every whole record before it is shown; the ragged tail is left alone, because this page
        never writes to that file.
      </p>
    {/if}

    <!-- THE NO-FEED ARM COMES FIRST, because "under another feed" is the wrong
         sentence when there is no feed to be other than. Before this arm
         existed the state produced NO note at all — see the comment on `runs`
         — so the table simply widened to every vendor in silence. -->
    {#if !activeFeed && !everyFeed}
      <p class="inline-note">
        No feed is chosen, so nothing is scoped to one and no run is shown.
        {exact(allRuns.length)}
        {allRuns.length === 1 ? 'run is' : 'runs are'} recorded across every feed. Choose a feed in
        the top bar, or
        <button class="linky" onclick={() => (everyFeed = true)}>show every feed</button>.
      </p>
    {:else if hiddenByFeed > 0}
      <p class="inline-note">
        {exact(hiddenByFeed)}
        {hiddenByFeed === 1 ? 'run is' : 'runs are'} recorded under another feed and not shown.
        <button class="linky" onclick={() => (everyFeed = true)}>Show every feed</button>
      </p>
    {:else if everyFeed}
      <p class="inline-note">
        Showing every feed.
        <!-- THE BUTTON NEEDS A FEED TO NAME. Dropping the old `&& activeFeed`
             from this arm rendered the label as "Back to " with nothing after
             it — a button offering to return somewhere it could not name. -->
        {#if activeFeed}
          <button class="linky" onclick={() => (everyFeed = false)}>Back to {activeFeed}</button>
        {:else}
          No feed is chosen, so there is none to narrow back to — pick one in the top bar.
        {/if}
      </p>
    {/if}

    {#if refusedNumericRuns.length > 0 && runs.length > 0}
      <div class="inline-note warn" role="alert">
        <b>{exact(refusedNumericRuns.length)} ledger {refusedNumericRuns.length === 1 ? 'row was' : 'rows were'} refused before computation.</b>
        At least one 64-bit fact, mask, or required field cannot be represented exactly by this
        browser. The records remain visible as metadata in the comparison table, but they cannot
        enter the answer, rung charts, sorting, or Full report.
      </div>
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
      {:else if scopedRuns.length > 0}
        <div class="panel bt-note" role="alert">
          <h2>No scoped run is safe to compute</h2>
          <p>
            The ledger contains {exact(scopedRuns.length)} scoped
            {scopedRuns.length === 1 ? 'record' : 'records'}, but every one failed the exact
            integer and shape admission door. Nothing below is ranked, averaged, charted, or
            opened as a Full report. The escaped identifiers remain visible for diagnosis.
          </p>
          <ul>
            {#each refusedNumericRuns.slice(0, 8) as refused}
              <li>
                <code>{String(refused.run?.identity ?? 'identity unavailable')}</code>
                · {String(refused.run?.feed ?? 'feed unavailable')}
                · {String(refused.run?.underlying ?? 'instrument unavailable')}
                · {String(refused.run?.timeframe ?? 'timeframe unavailable')}
                — {refused.admission.why}
              </li>
            {/each}
          </ul>
          {#if refusedNumericRuns.length > 8}
            <p>{exact(refusedNumericRuns.length - 8)} more refused metadata rows are not expanded.</p>
          {/if}
        </div>
      <!-- NO FEED IS NOT "NO RUNS UNDER THIS FEED". Without this arm the panel
           below rendered its heading as "No runs under this feed" and its body
           as "none of them recorded against <b></b>" — an empty bold where a
           vendor name belongs, directly contradicting the note above it, which
           had just said no feed was chosen at all. The filter did not empty
           this table; the absence of a filter did. -->
      {:else if !activeFeed}
        <div class="panel bt-note">
          <h2>No feed is chosen</h2>
          <p>
            The ledger holds {exact(allRuns.length)}
            {allRuns.length === 1 ? 'run' : 'runs'} across every feed. A sweep is stamped with the
            feed its bars came from, so these are not one set seen differently — choose a feed in
            the top bar to see its own, or show them all together.
          </p>
          <button class="btn" onclick={() => (everyFeed = true)}>Show every feed</button>
        </div>
      {:else}
        <div class="panel bt-note">
          <h2>No runs under {activeFeed}</h2>
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
      <section class="block rise">
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
              <!-- AND WHICH COMBINATION WON IT. "The answer" named a feed, an
                   instrument, a rung, a span and four figures, and never the
                   thing that produced them. A best run whose conditions are not
                   named is not an answer to any question worth asking. -->
              <div class="crown-combo">
                <span class="crown-combo-k">won by</span>
                <span class="crown-combo-v">
                  {#if load.body?.has_mask === undefined}
                    <span class="dim"
                      >the running server does not send <code>has_mask</code> — rebuild and restart
                      it</span
                    >
                  {:else if !load.body?.has_mask}
                    <span class="dim">this record predates the condition mask</span>
                  {:else}
                    {@render conditionNames(best.mask_words)}
                  {/if}
                </span>
              </div>
              <p class="crown-note">
                Ranked on the worst-case number. The best case is
                <b>{money(best.fillGap)}</b> higher, which is how much of
                the headline is fill assumption rather than edge.
              </p>
            </div>
            <button class="btn ghost sm" onclick={(event) => toggle(best, event.currentTarget)}>
              {openIndex === best.index ? 'Close' : 'Drill in'}
            </button>
          </div>
        {:else}
          <div class="panel bt-note">
            <h2>NO COMPLETE RUN</h2>
            <p>
              {#if signalRungSet === null}
                The server did not publish one non-empty, duplicate-free signal-rung set, so this
                browser has no authority to decide which records the engine actually swept. Every
                headline and rung leader is withheld; the comparison table keeps each row and names
                the missing signal surface.
              {:else if haltedRuns.length > 0}
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
           THE HUMAN COMPARISON — ONE ROW, ONE DECISION

           The charts and the thirteen-column frontier board below are the
           operator surface. This is the customer surface: every recorded run
           in view, the setting that made it a different question, the result
           under unfavorable fills, the distance between the two fill models,
           and the two loss figures a person needs before opening the report.

           `#1` means only "highest unfavorable-fill result among the eligible
           rows shown". It does not mean profitable, and the sentence directly
           above the table says so when every row is below zero. Stopped,
           partial, zero-trade and unsealed rows stay in the ordered result with
           no rank. The view reveals twelve at a time and names every remainder;
           hiding one silently would turn a refusal into a cleaner-looking answer.
           ============================================================ -->
      <section class="block rise compare-block" aria-labelledby="recorded-compare-title">
        <div class="bh-row compare-heading">
          <div>
            <h2 class="bh" id="recorded-compare-title">Compare recorded runs</h2>
            <p class="bsub">
              Automatically ordered by the result when allowed fills go against you. The pattern
              minimum stays beside every row because a different minimum is a different search.
              <b>These are recorded totals, not like-for-like strategy scores; #1 is a reading
                reference, not a promise.</b>
            </p>
          </div>
          <span class="count">
            {exact(rankableRunCount)} finished · {exact(visibleComparedRuns.length)} of
            {exact(comparedRuns.length)} shown
          </span>
        </div>

        {#if rankableRunCount > 0}
          <div
            class="compare-callout"
            class:negative={positiveRankableCount === 0}
            role={positiveRankableCount === 0 ? 'note' : undefined}
          >
            <span class="compare-callout-mark" aria-hidden="true">
              {positiveRankableCount === 0 ? '!' : '✓'}
            </span>
            <div>
              {#if positiveRankableCount === 0}
                {#if negativeRankableCount === rankableRunCount}
                  <b>Every finished result loses under unfavorable fills.</b>
                  <span>
                    The first row is the least-negative result in this view, not a profitable
                    winner.
                  </span>
                {:else}
                  <b>No finished result is profitable under unfavorable fills.</b>
                  <span>
                    {exact(breakevenRankableCount)} break even and
                    {exact(negativeRankableCount)} lose. The first row is only a reading reference.
                  </span>
                {/if}
              {:else}
                <b>
                  {exact(positiveRankableCount)} of {exact(rankableRunCount)} finished
                  {rankableRunCount === 1 ? 'result stays' : 'results stay'} positive under
                  unfavorable fills.
                </b>
                <span>The table keeps the more favorable fill result beside it for context.</span>
              {/if}
              {#if ledger?.hit_scan_cap}
                <span>
                  This is the newest {exact(ledger.scanned)} records only; the ledger read stopped
                  at its ceiling.
                </span>
              {/if}
            </div>
          </div>
        {/if}

        {#if comparedRuns.length === 0}
          <p class="inline-note">No recorded runs are available to compare yet.</p>
        {:else}
          <div
            class="compare-wrap"
            role="region"
            aria-label="Recorded run comparison table"
          >
          <table class="compare-table">
            <caption class="sr-only">
              Recorded runs sorted by unfavorable-fill result. Rows that stopped early, failed
              integrity, cover only part of their span, or took no trades remain visible but are
              not ranked.
            </caption>
            <thead>
              <tr>
                <th scope="col">Rank &amp; status</th>
                <th scope="col">Run &amp; setting</th>
                <th scope="col" class="num">Unfavorable fills</th>
                <th scope="col" class="num">Fill impact</th>
                <th scope="col" class="num">Loss &amp; drawdown</th>
                <th scope="col">Pattern selected</th>
                <th scope="col"><span class="sr-only">Open full report</span></th>
              </tr>
            </thead>
            <tbody>
              {#each visibleComparedRuns as r (r.sourceIndex)}
                <tr class:reference={r.reference} class:excluded={!r.eligible}>
                  <th scope="row" class="compare-rank">
                    <span class="compare-place">{r.rank === null ? '—' : `#${r.rank}`}</span>
                    <span
                      class="compare-state"
                      class:good={r.status.key === 'rankable'}
                      class:warn={[
                        'halted',
                        'non_signal_timeframe',
                        'partial_span',
                        'no_trades'
                      ].includes(r.status.key)}
                      class:bad={[
                        'integrity_failed',
                        'signal_surface_unavailable',
                        'contradictory_fill_totals',
                        'missing_numeric_results'
                      ].includes(r.status.key)}
                    >
                      {r.reference ? 'Reference' : r.status.label}
                    </span>
                    {#if r.reference}<small>{r.status.label}</small>{/if}
                  </th>
                  <td class="compare-run">
                    <div class="compare-id">
                      <b>{r.underlying}</b>
                      <span class="rung">{r.timeframe}</span>
                      <span class="dim">{r.feed}</span>
                    </div>
                    <span>{r.admitted ? comparisonSpan(r) : 'span not computed'}</span>
                    <span class="compare-setting">
                      pattern minimum
                      <b>{!r.admitted || r.supportBp === null ? 'not measured' : `${(r.supportBp / 100).toFixed(2)}%`}</b>
                      · months <b>{r.admitted ? `${comparisonCount(r.months_found)}/${comparisonCount(r.months_asked)}` : 'not computed'}</b>
                      · depth <b>{r.admitted ? comparisonCount(r.depth) : 'not computed'}</b> ·
                      <b>{r.admitted ? comparisonCount(r.trades) : 'not computed'}</b> trades
                    </span>
                  </td>
                  <td
                    class="num compare-money"
                    class:up={r.eligible && r.pessimistic >= 0}
                    class:down={r.eligible && r.pessimistic < 0}
                    data-label="Unfavorable fills"
                  >
                    <b>
                      {!r.admitted
                        ? 'not admitted'
                        : r.status.key === 'integrity_failed'
                        ? 'not trusted'
                        : Number.isSafeInteger(r.pessimistic)
                          ? money(r.pessimistic)
                          : 'not exact'}
                    </b>
                    {#if !r.admitted}
                      <small>{r.admissionWhy}</small>
                    {:else if r.status.key === 'integrity_failed'}
                      <small>{r.status.reason}</small>
                    {:else if r.reference}
                      <small>highest finished recorded total shown</small>
                    {:else if r.eligible && r.deltas?.pessimistic !== null}
                      <small>{money(Math.abs(r.deltas.pessimistic))} below reference</small>
                    {:else}
                      <small>{r.status.reason}</small>
                    {/if}
                  </td>
                  <td class="num compare-money" data-label="Fill impact">
                    <b>
                      {!r.admitted
                        ? 'not admitted'
                        : r.status.key === 'integrity_failed'
                        ? 'not trusted'
                        : r.fillGap === null
                          ? 'not usable'
                          : money(r.fillGap)}
                    </b>
                    <small>
                      {!r.admitted
                        ? r.admissionWhy
                        : r.status.key === 'integrity_failed'
                        ? 'the record failed its integrity seal'
                        : r.fillGap === null
                        ? 'the two fill totals contradict, are absent, or are not exact integers'
                        : `more favorable fills: ${money(r.optimistic)}`}
                    </small>
                  </td>
                  <td class="num compare-risk" data-label="Loss &amp; drawdown">
                    {#if !r.admitted}
                      <b>not admitted</b>
                      <small>{r.admissionWhy}</small>
                    {:else if r.status.key === 'integrity_failed'}
                      <b>not trusted</b>
                      <small>the record failed its integrity seal</small>
                    {:else}
                      <b>{Number.isSafeInteger(r.max_drawdown) ? money(Math.abs(r.max_drawdown)) : 'not exact'}</b>
                      <small>biggest fall from a peak</small>
                      <span>worst trade {Number.isSafeInteger(r.worst_trade) ? money(r.worst_trade) : 'not exact'}</span>
                    {/if}
                  </td>
                  <td class="compare-pattern" data-label="Pattern selected">
                    {#if !r.admitted}
                      <span class="dim">not decoded — {r.admissionWhy}</span>
                    {:else if r.status.key === 'integrity_failed'}
                      <span class="dim">not trusted — the record failed its integrity seal</span>
                    {:else if ledger?.has_mask === undefined}
                      <span class="dim">the running server does not identify mask-era records</span>
                    {:else if !ledger.has_mask}
                      <span class="dim">this record predates the condition mask</span>
                    {:else}
                      {@render conditionNames(r.mask_words)}
                    {/if}
                  </td>
                  <td class="compare-action">
                    <button
                      class="btn ghost sm"
                      aria-expanded={openRun?.index === r.index}
                      aria-label={`${openRun?.index === r.index ? 'Close' : 'Open'} the full ${r.underlying} ${r.timeframe} report`}
                      disabled={!r.admitted}
                      title={r.admitted
                        ? 'Open the exact recorded report'
                        : `Full report refused: ${r.admissionWhy}`}
                      onclick={(event) => toggle(r, event.currentTarget)}
                    >
                      {openRun?.index === r.index
                        ? 'Close'
                        : r.admitted
                          ? 'Full report'
                          : 'Report refused'}
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
            </table>
          </div>
          {#if comparisonRemaining > 0}
            <button
              class="btn ghost sm compare-more"
              onclick={() => (comparisonShown += COMPARISON_STEP)}
            >
              Show next {exact(Math.min(COMPARISON_STEP, comparisonRemaining))} records ·
              {exact(comparisonRemaining)} remain
            </button>
          {/if}
        {/if}
        <p class="compare-foot">
          “Unfavorable” and “more favorable” are the two allowed fill models recorded for the same
          trades. They are not confidence bounds. Open a row for charts, trade detail, validation
          and every technical metric.
        </p>
      </section>

      <!-- ============================================================
           LEVEL 2 — EIGHT RUNGS SIDE BY SIDE

           EIGHT IS WHAT IS SWEPT, and this banner said nine. The store holds
           nine timeframes; `cli::EVERY_RUNG` is the eight `*min` ones, and
           `1day` is on disk to feed `indicators::daily` rather than to be
           swept — `swept` a few hundred lines above states the same rule and
           excludes such a row from ranking. The ROWS here are still whatever
           the ledger holds for a span, which is the honest count and is
           printed beside each group; the banner names the ceiling.
           ============================================================ -->
      <section class="block rise">
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
                <!-- "support" is the engine's word for it. What it MEANS is how
                     often a pattern had to show up before the sweep would keep
                     it, and that is what the reader needs. This said
                     "support ≥ 20.0% of bars" as though 20% were the figure;
                     the engine names no percentage at all and each rung derives
                     its own. The number below is computed from the run's OWN
                     `min_hits` and `bars`, so it was always right even while
                     the sentence beside it was not. -->
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
                  onclick={(event) => toggle(r, event.currentTarget)}
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
           LEVEL 0.5 — THE RECORDED PREFIX OF EVERY TIMEFRAME

           The ledger below shows ONE row per run: the single combination the
           exit grid picked. This board reorders the PRICED ROWS IN THE STORED
           FRONTIER PREFIX on the operator's own eleven criteria. It must not be
           called a global Top-N: `record_frontier` receives a bounded evidence
           prefix, so a combination outside that prefix never reached the exit
           grid and cannot audition here.

           Ranked in the BROWSER on purpose. A score compiled into the binary is
           one more number nobody can see -- the exact failure that cost this
           project two nights, from a hardcoded support floor to a hardcoded
           ranking lens. Every weight below is a control.
           ============================================================ -->
      <section class="block rise">
        <div class="bh-row">
          <h2 class="bh">Preview: top {topShown} within each stored prefix</h2>
          <span class="count">
            {#if board.phase === 'loading'}reading…{:else}{exact(board10.length)} comparable question/timeframe groups{/if}
          </span>
        </div>

        <p class="inline-note">
          <b>This is not yet a global Top {topShown}.</b> Each table re-ranks only the combinations
          retained, priced, and persisted by that run. A run may be called globally complete only
          after Apriori extinction and after every eligible closed combination has been priced and
          judged by the same versioned Rust ranking policy. Until that completion receipt exists,
          rows omitted before the exit grid remain unknown—not losers.
        </p>

        <div class="wrow">
          <label class="wlab">
            show top
            <input class="wnum" name="frontier-top-count" type="number" min="1" max="100" bind:value={topN} />
          </label>
          <!-- ELEVEN PAIRS, and the eleventh is the one that was missing. The
               operator's ranking names eleven quantities; this row carried ten,
               and the absentee was "less losing ratio".

               A SLIDER THAT CANNOT MOVE THE ORDER SAYS SO. When every row on
               every rung agrees on a measurement, its normalised term is the
               same constant for all of them and the sort cannot see it — the
               control still moves a visible score and changes nothing. Measured
               on this operator's store: `avg_win` and `avg_loss` were zero on
               all seventeen rows, so two of these were dead and the page said
               it nowhere. -->
          {#each WEIGHT_FIELDS as [key, label] (key)}
            <label class="wlab" class:winert={inertAll.has(key)}>
              {label}{#if inertAll.has(key)}<span
                  class="wdead"
                  title="Every row on every rung reports the same value here, so this weight cannot change the order.">inert</span
                >{/if}
              <input
                class="wrange"
                name={`frontier-weight-${key}`}
                type="range"
                min="0"
                max="3"
                step="0.5"
                bind:value={weights[key]}
              />
              <b class="wval">{weights[key]}</b>
            </label>
          {/each}
        </div>
        <!-- ALL ZERO IS NOT A RANKING. Every weight at zero makes every score
             zero, the sort falls through to the tie-break, and the table shows
             the ENGINE's own payoff order under a heading that says it is the
             operator's. Saying so is the difference between a control and a
             trap. -->
        {#if WEIGHT_FIELDS.every(([key]) => Number(weights[key]) === 0)}
          <p class="inline-note">
            <b>Every weight is zero, so this is not your ranking.</b> With no criterion carrying any
            weight the rows fall back to the sweep's own order — ranked on unstopped forward payoff,
            which is the ordering these sliders exist to replace.
          </p>
        {/if}

        {#if board.phase === 'failed' || board.why}
          <p class="inline-note">{board.why}</p>
        {/if}

        {#each board10 as g (g.key)}
          <div class="tt-sec">
            <div class="tt-hrow">
              <h3 class="tt-h">
                {g.run.feed} · {g.run.underlying} · {span(g.run)} · {g.rung}
                <span class="dim">
                  · {exact(g.run.bars)} bars · support {(
                    (g.run.min_hits / Math.max(1, g.run.bars)) *
                    100
                  ).toFixed(1)}% · depth {g.run.depth} · {exact(g.run.combinations)} combinations enumerated
                </span>
              </h3>
            </div>
            {#if g.top.length > 0}
              <div class="tt-tblwrap">
                <table class="tt-tbl rungprog">
                  <thead>
                    <tr>
                      <th>#</th><th>rules</th><th class="n">score</th><th class="n">trades</th>
                      <th class="n">win %</th><th class="n">win / loss</th>
                      <th class="n">worst fills</th>
                      <th class="n">max drawdown</th><th class="n">worst trade</th>
                      <th class="n">smallest win</th>
                      <th class="n">avg win</th><th class="n">avg loss</th>
                      <th class="n">reward:risk</th>
                    </tr>
                  </thead>
                  <tbody>
                    <!-- TWO ROWS PER COMBINATION. The second carries the condition
                         NAMES, which is the one thing the operator asked for more
                         often than anything else on this page and the one thing
                         neither ranked table showed: `/frontier.json` sends
                         `mask_words` on every row and nothing here read them. -->
                    {#each g.top as c, i (c.rank)}
                      <tr class="cmbrow">
                        <td><b>{i + 1}</b></td>
                        <td>{@render verdictPill(c.meets)}</td>
                        <td class="n">{c.score.toFixed(2)}</td>
                        <td class="n">{exact(c.trades)}</td>
                        <td class="n">{(c.win_rate_bp / 100).toFixed(1)}%</td>
                        <td class="n"><span class="up">{exact(c.wins)}</span> / <span class="down">{exact(c.losses)}</span></td>
                        <td class="n {c.pessimistic < 0 ? 'down' : 'up'}">{money(c.pessimistic)}</td>
                        <td class="n down">{money(c.max_drawdown)}</td>
                        <td class="n down">{money(c.worst_trade)}</td>
                        <td class="n up">{money(c.min_win)}</td>
                        <td class="n up">{money(c.avg_win)}</td>
                        <td class="n down">{money(c.avg_loss)}</td>
                        <td class="n">{c.reward_to_risk_bp === null ? "—" : (c.reward_to_risk_bp / 100).toFixed(2) + "×"}</td>
                      </tr>
                      <tr class="namerow">
                        <td colspan="13">{@render conditionNames(c.mask_words)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
              <!-- `best fills` IS GONE, AND ITS REMOVAL IS THE HONEST FIX.
                   The column rendered `c.optimistic`, and `/frontier.json` has
                   never sent that field — `money(undefined)` is an em dash, and
                   `undefined < 0` is false, so every cell on every rung drew an
                   empty value wearing the profit colour. The note that used to
                   sit here explained at length why both fill models must be
                   shown side by side, above a column that showed nothing.
                   `smallest win` takes its place: it IS sent, and it is the
                   numerator of the reward-to-risk rule beside it. -->
              <p class="tt-note2 dim">
                {exact(g.priced)} of {exact(g.rows.length)} recorded combinations were priced
                against the exit grid; the rest store zeros and are excluded, because a zero
                drawdown outranks every real one.
                <b>This ordering is complete only within those {exact(g.priced)} priced rows.</b>
                {#if g.rules}
                  <b>{exact(g.admittedShown)} of {exact(g.top.length)} shown meet your rules</b>
                  — win rate ≥ {(g.rules.min_win_rate_bp / 100).toFixed(0)}%, reward:risk ≥
                  {(g.rules.min_rr_bp / 100).toFixed(2)}×, return over drawdown ≥
                  {(g.rules.min_ret_over_dd_bp / 100).toFixed(2)}×.
                {/if}
              </p>
            {:else}
              <p class="tt-note2">
                {#if g.why}{g.why}{:else}No priced combination was recorded for this rung.{/if}
              </p>
            {/if}
          </div>
        {/each}

        {#if board.phase === 'ready' && board10.length === 0}
          <p class="inline-note">
            No completed run is in view, so there is nothing to rank yet.
          </p>
        {/if}
      </section>


      <!-- ============================================================
           LEVEL 0 — THE LEDGER
           ============================================================ -->
      <section class="block rise">
        <div class="bh-row">
          <h2 class="bh">The ledger</h2>
          <input
            class="find"
            name="ledger-filter"
            type="search"
            placeholder="instrument, feed or rung…"
            bind:value={query}
            aria-label="Filter runs"
          />
          <span class="count">{exact(sorted.length)} of {exact(runs.length)} shown</span>
        </div>

        {#if sorted.length === 0}
          <p class="inline-note">
            Nothing matches <b>{query}</b>. The filter matches any part of the instrument, feed and
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
                  onclick={(event) => toggle(r, event.currentTarget)}
                  onkeydown={(event) => {
                    if (event.key !== 'Enter' && event.key !== ' ') return;
                    event.preventDefault();
                    void toggle(r, event.currentTarget);
                  }}
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
                    {:else if !swept(r)}
                      <!-- NOT A SIGNAL TIMEFRAME, so `complete` is the wrong
                           question about it. `1day` feeds
                           `indicators::daily` — the previous session's OHLC
                           for the intraday rungs — and the engine sweeps the
                           eight `*min` rungs only. The row stays, because a
                           recorded run is a fact and §4 bans hiding one; it
                           is simply never ranked and never compared. -->
                      <span
                        class="pill warn"
                        title="{r.timeframe} is not a signal timeframe. It is held to define the previous session's OHLC for the intraday rungs (indicators::daily, vocabulary bits 13–18) and cli::EVERY_RUNG sweeps the eight *min rungs only. This row is listed but never ranked."
                        >not swept · {r.timeframe}</span
                      >
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
        <section class="block drill" tabindex="-1">
          <div class="bh-row">
            <h2 class="bh">{openRun.underlying} · {openRun.timeframe} · {span(openRun)}</h2>
            <button class="btn ghost sm" onclick={closeDrill}>Close</button>
          </div>

          {#if tradeList.phase === 'failed'}
            <p class="inline-note bad" role="alert">
              <b>The trade analytics payload was refused.</b> {tradeList.why} No trade row,
              equity, streak, time bucket, win count, or derived money figure from that response is
              displayed. This is different from an honestly empty trade file.
            </p>
          {/if}

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
                      aria-pressed={chartRung === r.name}
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
                  Reading the current store for {span(openRun)} at {chartRung}. This endpoint does
                  not prove those bytes match the run's recorded data digest.
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
                )} bars ending {span(openRun)}. Prices in rupees. Entry and exit markers are omitted because chosen rows use execution-series bar indices while this chart may show another rung or a clipped window, and no durable alignment or exit-cause marker is stored."
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
                    <button class="rangebtn" disabled={!chartReady} onclick={() => showLast(p.bars)}>{p.label}</button>
                  {/each}
                </div>
              {/if}
            </div>

            {#if series.phase === 'ready' && series.bars.length > 0}
              <p class="cnote faint term-note">
                <b>No entries or exits are marked, and that is deliberate.</b> Chosen-grid rows carry
                exact execution-series bar indices and times, but this chart may show another rung
                or only the newest window. The durable row stores no chart-alignment key and no
                exit-cause marker, so overlaying one would guess. Use List of trades for the exact
                timestamps and MAE/MFE. No indicator is overlaid either — one computed here would
                be a second implementation of something <code>crates/indicators</code> already owns.
              </p>
            {/if}
          </div>

          <!-- ==========================================================
               THE STRATEGY TESTER
               Built to the operator's screenshots: TradingView's toolbar,
               its three views, its five and three pill tabs, its stat
               quads with the percentage under the absolute, and its chart
               frames with axes, legends and toggles. Unavailable data is
               named neutrally; a lock would falsely imply a permission gate.

               A CHART WITHOUT A SOURCE STILL DRAWS ITS FRAME. Axes,
               legend, period toggles and pagers are all present and all
               work; what is missing is the series, and the plot area says
               so. That is the same design with an honest interior, and it
               is the only version of this panel that cannot mislead
               somebody about money.
               ========================================================== -->
          <section class="tester">
            <!-- ══ WHICH COMBINATION IS THIS? ══
                 THE QUESTION THE WHOLE PANEL ANSWERED NOWHERE.

                 Opening a run showed Key stats, a performance chart, five
                 analysis tabs, a trades table and forty-odd numbers — under a
                 header reading "Brute-force sweep · NIFTY · 30min". Not one of
                 them said WHICH combination of conditions produced any of it.
                 The winning mask was decoded in the Properties tab, which is a
                 third tab behind a third click, and the default view is
                 `metrics`.

                 The operator has asked for this more times than for anything
                 else on this page — "no name has been given as i asked you
                 what's the combination topped" — and the answer has been one
                 `{@render}` away since `/backtest.json` started sending
                 `mask_words` and `/vocab.json` started naming the bits.

                 It sits above the toolbar rather than inside it because it is
                 not a control: it is the subject every control below operates
                 on, and it must be true of `metrics`, `trades` and `properties`
                 alike. -->
            <div class="tt-subject">
              <span class="tt-subject-k">Combination</span>
              {#if load.body?.has_mask === undefined}
                <span class="tt-subject-v dim"
                  >The running server does not send <code>has_mask</code>. Rebuild and restart it to
                  name the conditions here.</span
                >
              {:else if !load.body?.has_mask}
                <span class="tt-subject-v dim"
                  >This ledger predates the condition mask, so the run's own record cannot say which
                  conditions won.</span
                >
              {:else}
                <span class="tt-subject-v">{@render conditionNames(openRun.mask_words)}</span>
              {/if}
            </div>

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
                <button class="tt-view ic" class:on={testerView === 'metrics'} aria-label="Metrics report" aria-pressed={testerView === 'metrics'} onclick={() => (testerView = 'metrics')} title="Metrics">
                  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M2 11l3.5-4.5L8.5 9 14 3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
                </button>
                <button class="tt-view ic" class:on={testerView === 'trades'} aria-label="List of trades" aria-pressed={testerView === 'trades'} onclick={() => (testerView = 'trades')} title="List of trades">
                  <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="2" y="3" width="12" height="10" rx="1" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M2 6.5h12M6 6.5V13" stroke="currentColor" stroke-width="1.1" /></svg>
                </button>
                <button class="tt-view" class:on={testerView === 'properties'} aria-pressed={testerView === 'properties'} onclick={() => (testerView = 'properties')}>Properties</button>
              </div>

              <!-- TESTING PERIOD, with TradingView's own menu -->
              <div class="tt-period" bind:this={periodRoot} onfocusout={dismissPeriodOnFocusOut}>
                <button
                  class="tt-ctl"
                  bind:this={periodTrigger}
                  aria-label="Testing period: {span(openRun)}"
                  aria-haspopup="dialog"
                  aria-controls="testing-period-choices"
                  aria-expanded={periodOpen}
                  onclick={togglePeriodMenu}
                >
                  <svg viewBox="0 0 16 16" class="tt-cico" aria-hidden="true"><rect x="2" y="3" width="12" height="11" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M2 6.5h12M5.5 2v2.5M10.5 2v2.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" /></svg>
                  {span(openRun)}
                  <span class="tt-caret" aria-hidden="true">⌄</span>
                </button>
                {#if periodOpen}
                  <div
                    class="tt-menu"
                    id="testing-period-choices"
                    role="dialog"
                    aria-labelledby="testing-period-title"
                    bind:this={periodDialog}
                  >
                    <div class="tt-menuhead">
                      <span id="testing-period-title">Testing period</span>
                    </div>
                    {#each TESTING_PERIODS as p (p)}
                      <button
                        class="tt-menuitem"
                        class:sel={p === 'Available chart range'}
                        aria-pressed={p === 'Available chart range'}
                        disabled={p !== 'Available chart range'}
                        onclick={() => closePeriodMenu(true)}
                      >
                        {p}
                        {#if p === 'Available chart range'}<span class="tt-default">Default</span>{/if}
                        {#if p !== 'Available chart range'}<span class="tt-default">new run required</span>{/if}
                      </button>
                    {/each}
                    <!-- TradingView's last item sits under a divider and
                         carries a calendar. -->
                    <div class="tt-menudiv"></div>
                    <button class="tt-menuitem" disabled>
                      <span class="tt-menuic">
                        <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="2" y="3" width="12" height="11" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M2 6.5h12M5.5 2v2.5M10.5 2v2.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" /></svg>
                        Custom date range
                      </span>
                      <span class="tt-default">new run required</span>
                    </button>
                  </div>
                {/if}
              </div>

              <span class="tt-fact" title="Recorded unit; this report does not model an account">
                <svg viewBox="0 0 16 16" class="tt-cico" aria-hidden="true"><circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M8 5v6M6.3 6.5h3.4M6.3 9.5h3.4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></svg>
                1 unit <span class="tt-dim2">index points</span>
              </span>
              <span class="tt-fact" title="Recorded signal and execution resolution">
                <svg viewBox="0 0 16 16" class="tt-cico" aria-hidden="true"><path d="M3 13V7M8 13V3M13 13V9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg>
                {openRun.timeframe} signal · 1min execution
              </span>

              <div class="tt-meta">
                <!-- THE LEDGER ROW AND THIS BAR MUST NOT DISAGREE ABOUT ONE
                     RECORD. The row says `not swept · 1day` and this said
                     `complete · depth 15` about the same run -- two surfaces,
                     one fact, opposite readings, and this is the one an
                     operator is looking at when they drill in.

                     `complete` is a true statement about the LADDER: it ran
                     to extinction. It is the wrong headline for a rung the
                     engine never sweeps, because it invites the row to be
                     compared with rungs that were. The depth survives beside
                     it; only the crown word goes. -->
                {#if openRun.halted}
                  <span class="tt-chip warn">halted · depth {openRun.depth} partial</span>
                {:else if !swept(openRun)}
                  <span
                    class="tt-chip warn"
                    title="{openRun.timeframe} is not a signal timeframe — it is held to define the previous session's OHLC for the intraday rungs (indicators::daily, vocabulary bits 13–18). The ladder did run to extinction at depth {openRun.depth}; this run is listed and never ranked."
                    >not swept · depth {openRun.depth}</span
                  >
                {:else}
                  <span class="tt-chip good">complete · depth {openRun.depth}</span>
                {/if}
                <span class="tt-chip" class:warn={!openRun.whole_span}>{exact(openRun.months_found)}/{exact(openRun.months_asked)} months</span>
              </div>
            </div>

            {#if testerView === 'metrics'}
              <!-- ================= KEY STATS ================= -->
              <div class="tt-sec">
                <h3 class="tt-h">Key stats</h3>
                <div class="tt-quad">
                  <div class="tt-q">
                    <span class="tt-k">Total PnL</span>
                    <span class="tt-qv big" class:up={openRun.pessimistic >= 0} class:down={openRun.pessimistic < 0}>
                      {money(openRun.pessimistic)}<em class="tt-unit">POINTS</em>
                      <em class="tt-pc">{noTrades ? 'none' : pct(strategyBps)}</em>
                    </span>
                    <span class="tt-note">worst-case fills · {money(openRun.optimistic)} at best</span>
                  </div>
                  <div class="tt-q">
                    <span class="tt-k">Max drawdown</span>
                    <!-- ZERO HERE IS A PHASE THAT DID NOT RUN, NOT A RUN THAT
                         NEVER LOST. `publish_ranked` writes before the exit
                         grid, so a run the grid never priced carries
                         `max_drawdown: 0` — which this rendered as
                         "0.00 POINTS · 0.00% · worst peak-to-trough", the best
                         reading available anywhere on this page.
                         `crates/api/src/livejson.rs` names those eight fields
                         as structural zeros and refuses to judge them for
                         exactly this reason: figures nobody measured wearing
                         the shape of figures somebody did. The two tiles to the
                         right already guard on this same `noTrades`; this one
                         did not. -->
                    {#if noTrades}
                      <span class="tt-qv big dim">none</span>
                      <span class="tt-note"
                        >no position was opened, so there is no peak and no trough</span
                      >
                    {:else}
                      <span class="tt-qv big down">
                        {money(Math.abs(openRun.max_drawdown))}<em class="tt-unit">POINTS</em>
                        <em class="tt-pc"
                          >{drawdownBps === null ? '' : `${(drawdownBps / 100).toFixed(2)}%`}</em
                        >
                      </span>
                      <span class="tt-note">worst peak-to-trough</span>
                    {/if}
                  </div>
                  <!-- TWO VALUES SIDE BY SIDE, as TradingView sets this one:
                       the percentage and the fraction it came from. The
                       denominator is recorded and the numerator is not, so the
                       fraction is drawn with the half that exists. -->
                  <div class="tt-q">
                    <span class="tt-k">Profitable trades</span>
                    <!-- A FRACTION OVER ZERO IS NOT UNKNOWN, IT IS MEANINGLESS.
                         With no trades this rendered a padlock, a slash and a
                         zero -- `/0` -- which reads as a broken value rather
                         than as an absent one. The padlock is the right mark
                         for "the numerator was never recorded"; it is the
                         wrong mark for "there is no set to take a fraction
                         of". Those are different facts and they now render
                         differently. -->
                    {#if openRun.trades === 0}
                      <span class="tt-qv big dim">none</span>
                      <span class="tt-note">no trade was opened, so there is nothing to divide</span>
                    {:else if tradeTotals}
                      <!-- REAL, AND THE PADLOCK BESIDE IT WAS THE STALE HALF.
                           This read "No win count is recorded" while the details
                           table three sections below printed that very count from
                           `/trades.json`'s period buckets. One page cannot say a
                           number is unrecorded and then show it. -->
                      <span class="tt-qv big">
                        {share(tradeTotals.rateBp)}
                        <em class="tt-frac">{exact(tradeTotals.worstWins)}/{exact(tradeTotals.trades)}</em>
                      </span>
                      <span class="tt-note"
                        >at worst-case fills · {share(tradeTotals.bestRateBp)} at best</span
                      >
                    {:else}
                      <span class="tt-qv big">
                        <Lock why={tradeEvidenceWhy} />
                        <em class="tt-frac"><Lock small why="The numerator — how many of these trades won — needs the trade file." />/{exact(openRun.trades)}</em>
                      </span>
                      <span class="tt-note">of {exact(openRun.trades)} closed</span>
                    {/if}
                  </div>
                  <div class="tt-q">
                    <span class="tt-k">Profit factor</span>
                    <!-- REAL. "the sweep records only the net" was true of the
                         results ledger; the trade file records each round trip's
                         realised worst-case result, so both gross halves are
                         sums over it. -->
                    <span class="tt-qv big"
                      >{#if tradeStats?.profitFactor !== null && tradeStats}{tradeStats.profitFactor?.toFixed(3)}{:else if tradeStats}<Lock kind="undefined" why="Needs a losing trade to divide by. This run recorded none at worst-case fills, and a ratio with no denominator is undefined rather than infinite." />{:else}<Lock why={tradeEvidenceWhy} />{/if}</span
                    >
                    <span class="tt-note">gross profit ÷ gross loss, at worst-case fills</span>
                  </div>
                </div>
              </div>

              <!-- ================= PERFORMANCE ================= -->
              <div class="tt-sec">
                <div class="tt-hrow">
                  <h3 class="tt-h">Performance</h3>
                </div>
                <div class="tt-perf">
                  <!-- TradingView lists the plots as PLAIN TEXT down the left
                       of the chart, each with an eye that toggles it, and a
                       collapse chevron underneath. No boxes, no bullets. -->
                  <div class="tt-plots">
                    <!-- CUMULATIVE PnL IS DRAWN NOW. Its padlock read "No equity
                         series is recorded — four scalars cannot make a curve",
                         which was true of the LEDGER and untrue of the trade
                         file: `worst` in `seq` order is the series, and its
                         running sum is the curve. The eye picks which of the two
                         drawable plots is shown, as the reference's does. -->
                    <div class="tt-plotrow" class:off={!equity || plot !== 'equity'}>
                      <span>Cumulative PnL</span>
                      {#if equity}
                        <button
                          class="tt-eye"
                          title={plot === 'equity' ? 'Shown' : 'Show'}
                          aria-label="Show cumulative PnL"
                          aria-pressed={plot === 'equity'}
                          onclick={() => {
                            plot = 'equity';
                            performancePage = 0;
                          }}
                        >
                          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M1.5 8s2.4-4 6.5-4 6.5 4 6.5 4-2.4 4-6.5 4S1.5 8 1.5 8z" fill="none" stroke="currentColor" stroke-width="1.3" /><circle cx="8" cy="8" r="1.9" fill="currentColor" /></svg>
                        </button>
                      {:else}
                        <Lock small why={equityEvidenceWhy} />
                      {/if}
                    </div>
                    <div class="tt-plotrow" class:off={equity && plot !== 'hold'}>
                      <span>Buy and hold · current store</span>
                      <button
                        class="tt-eye"
                        title={plot === 'hold' || !equity ? 'Shown' : 'Show'}
                        aria-label="Show buy and hold"
                        aria-pressed={plot === 'hold' || !equity}
                        onclick={() => {
                          plot = 'hold';
                          performancePage = 0;
                        }}
                      >
                        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M1.5 8s2.4-4 6.5-4 6.5 4 6.5 4-2.4 4-6.5 4S1.5 8 1.5 8z" fill="none" stroke="currentColor" stroke-width="1.3" /><circle cx="8" cy="8" r="1.9" fill="currentColor" /></svg>
                      </button>
                    </div>
                    <div class="tt-plotrow">
                      <span>Trade MAE / MFE</span>
                      <small>recorded per trade · table below</small>
                    </div>
                    <div class="tt-plotrow" class:off={!equity?.swingPlot || plot !== 'swings'}>
                      <span>Run-ups and drawdowns</span>
                      <button
                        class="tt-eye"
                        title={equity?.swingPlot
                          ? plot === 'swings'
                            ? 'Shown'
                            : 'Show'
                          : equity
                            ? 'The strategy curve is flat, so it contains no rising or falling stretch.'
                            : equityEvidenceWhy}
                        aria-label="Show strategy run-ups and drawdowns"
                        aria-pressed={plot === 'swings'}
                        disabled={!equity?.swingPlot}
                        onclick={() => {
                          if (equity?.swingPlot) {
                            plot = 'swings';
                            swingPage = 0;
                          }
                        }}
                      >
                        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M1.5 8s2.4-4 6.5-4 6.5 4 6.5 4-2.4 4-6.5 4S1.5 8 1.5 8z" fill="none" stroke="currentColor" stroke-width="1.3" /><circle cx="8" cy="8" r="1.9" fill="currentColor" /></svg>
                      </button>
                    </div>
                  </div>
                  <div class="tt-plot">
                    {#if equity && plot === 'equity'}
                      <!-- `equity.points` is `{t, v}` — the shape `areaChart`
                           already eats — with `t` in seconds off each trade's
                           own exit stamp, so the axis needs no new formatter. -->
                      {@const performanceScale = curveScale(equity.points)}
                      {@render areaChart(
                        performanceWindow.rows,
                        [
                          money(performanceScale),
                          money(Math.round(performanceScale / 2)),
                          '0',
                          money(-Math.round(performanceScale / 2)),
                          money(-performanceScale)
                        ],
                        curveLabels(performanceWindow.rows),
                        '',
                        performanceScale
                      )}
                      {#if performanceWindow.pages > 1}
                        <div class="series-nav" aria-label="Performance trade pages">
                          <button
                            aria-label="Earlier performance trades"
                            disabled={performanceWindow.page === 0}
                            onclick={() => (performancePage = Math.max(0, performanceWindow.page - 1))}>‹</button
                          >
                          <span
                            >Trades {exact(performanceWindow.start + 1)}–{exact(
                              Math.min(
                                performanceWindow.start + performanceWindow.rows.length,
                                performanceWindow.total
                              )
                            )} of {exact(performanceWindow.total)}</span
                          >
                          <button
                            aria-label="Later performance trades"
                            disabled={performanceWindow.page + 1 >= performanceWindow.pages}
                            onclick={() =>
                              (performancePage = Math.min(
                                performanceWindow.pages - 1,
                                performanceWindow.page + 1
                              ))}>›</button
                          >
                        </div>
                      {/if}
                      <p class="tt-plotnote">
                        Up to <b>{exact(PERFORMANCE_PAGE_SIZE)} exact trades</b> are shown per page;
                        paging changes only the visible window, while cumulative values stay absolute.
                        The running sum uses each trade's realised <b>worst-case</b> result in the
                        exact chosen-grid resolution order. The sealed writer refuses the result unless
                        this curve ends at the ledger headline
                        <b>{money(openRun.pessimistic)}</b> and its count, optimistic sum, worst trade,
                        max drawdown, and complete cell also reconcile.
                      </p>
                    {:else if equity?.swingPlot && plot === 'swings'}
                      {@render swingChart(
                        equity.swingPlot,
                        'Alternating rising and falling stretches of the strategy equity curve, on the same trade-resolution clock as cumulative P&L. The final open stretch is included and marked by its actual direction.'
                      )}
                    {:else if bench.phase === 'loading'}
                      <div class="tt-empty"><span class="spin sm" aria-hidden="true"></span> Reading the current store's span endpoints…</div>
                    {:else if holdCurve.length > 1}
                      {@const holdScale = curveScale(holdCurve)}
                      {@render areaChart(
                        holdCurve,
                        [
                          money(holdScale),
                          money(Math.round(holdScale / 2)),
                          '0',
                          money(-Math.round(holdScale / 2)),
                          money(-holdScale)
                        ],
                        curveLabels(holdCurve),
                        '',
                        holdScale
                      )}
                      <p class="tt-plotnote">
                        This is the newest current-store window tail of holding one unit from the
                        current store's first close over the named span. The headline endpoint
                        <b>{buyHold ? money(buyHold.gain) : '—'}</b> is independently fetched at the
                        run's recorded rung, so switching this display rung cannot rewrite the query.
                        The bars endpoint does not bind either price to the run's data digest. The
                        strategy curve remains on its separate trade-resolution clock; the two are not
                        falsely aligned point by point.
                      </p>
                    {:else}
                      <div class="tt-empty"><Lock /> {bench.why || 'No plot on this list can be drawn from what is recorded.'}</div>
                    {/if}
                  </div>
                </div>
              </div>

              <!-- ================= PERFORMANCE ANALYSIS ================= -->
              <div class="tt-sec">
                <h3 class="tt-h">Performance analysis</h3>
                <div class="tt-pills" role="group" aria-label="Performance analysis">
                  {#each [['breakdown', 'Breakdown'], ['periodical', 'Periodical'], ['benchmarking', 'Benchmarking'], ['margin', 'Margin usage'], ['growth', 'Growth and decline']] as [key, label] (key)}
                    <button class="tt-pill" class:on={paTab === key} aria-pressed={paTab === key} onclick={() => (paTab = key)}>{label}</button>
                  {/each}
                </div>

              {#key paTab}
                <div class="bt-pane">
                  {#if paTab === 'breakdown'}
                  <div class="tt-quad">
                    <!-- REAL. "Only the net total is recorded" was true of the
                         results ledger and stopped being true of this page when
                         the trade file arrived: `worst` is each trade's realised
                         worst-case result, so the gross halves are two sums over
                         the list and the factor is their quotient. -->
                    <div class="tt-q">
                      <span class="tt-k">Gross profit</span>
                      <span class="tt-qv"
                        >{#if noTrades}{money(0)}{:else if tradeStats}{money(tradeStats.grossProfit)}{:else}<Lock why={tradeEvidenceWhy} />{/if}</span
                      >
                    </div>
                    <div class="tt-q">
                      <span class="tt-k">Gross loss</span>
                      <span class="tt-qv"
                        >{#if noTrades}{money(0)}{:else if tradeStats}{money(tradeStats.grossLoss)}{:else}<Lock why={tradeEvidenceWhy} />{/if}</span
                      >
                    </div>
                    <div class="tt-q">
                      <span class="tt-k">Profit factor</span>
                      <span class="tt-qv"
                        >{#if tradeStats?.profitFactor !== null && tradeStats}{tradeStats.profitFactor?.toFixed(3)}{:else if tradeStats}<Lock kind="undefined" why="This run recorded no losing trade at worst-case fills, so gross loss is zero — a profit factor with no denominator is undefined, not infinite." />{:else}<Lock why={tradeEvidenceWhy} />{/if}</span
                      >
                    </div>
                    <!-- TWO DIFFERENT THINGS, AND THIS LOCK USED TO LUMP THEM.
                         It read "every figure on this page is GROSS of
                         brokerage, spread and the statutory charges, and the
                         size of that omission is UNMEASURED" — alarming the
                         operator about a half that is CORRECT.

                         `costs::scope::is_cost_free(Segment::IndexSpot)` is
                         TRUE, and its own doc says why: a spot index is not
                         tradeable and no order is placed, so there is no
                         brokerage, no STT, no stamp duty and no GST to charge.
                         `crates/runner/src/audit.rs` states the same. The
                         engine sweeps exactly two instruments, both spot
                         indices (CLAUDE.md §1), so zero is the right answer and
                         `costs::trip` having no caller on this path is the
                         right shape — not a gap.

                         What IS absent is the SPREAD, which is a fill-model
                         question and not a levy. `trade.rs`' own measured table
                         records two horizons flipping from profitable to
                         −27,883p and −8,658p once a tick was charged, so it is
                         a bound and not a rounding. Kept separate, and kept
                         visible. -->
                    <div class="tt-q">
                      <span class="tt-k">Commission load</span>
                      <span class="tt-qv">0.00%</span>
                      <span class="tt-note">statutory charges are zero for spot indices · spread is not modeled</span>
                    </div>
                  </div>

                  <div class="tt-hrow tight">
                    <h4 class="tt-h5">Profits and losses</h4>
                    <div class="tt-seg" role="group" aria-label="Split">
                      <button class="tt-segbtn" class:on={plSplit === 'signals'} aria-pressed={plSplit === 'signals'} onclick={() => (plSplit = 'signals')}>By signals</button>
                      <button class="tt-segbtn" class:on={plSplit === 'side'} aria-pressed={plSplit === 'side'} onclick={() => (plSplit = 'side')}>By side</button>
                    </div>
                  </div>
                  {#if plSplit === 'signals'}
                    <div class="tt-unavailable-panel">
                      <span>
                        Splitting profit and loss <b>by signal</b> needs each trade tagged with the entry that opened it. The
                        sweep records one total for the whole run.
                      </span>
                    </div>
                  {:else if tradeStats && tradeList.direction}
                    {@const sideGross = Math.max(1, tradeStats.grossProfit + tradeStats.grossLoss)}
                    <div class="tt-side">
                      <div class="tt-sidehead">
                        <b>{tradeList.direction === 'long' ? 'Long' : 'Short'}</b>
                        <span>selected direction only</span>
                        <strong class:up={openRun.pessimistic >= 0} class:down={openRun.pessimistic < 0}
                          >{money(openRun.pessimistic)}</strong
                        >
                      </div>
                      <div
                        class="tt-sidebar"
                        role="img"
                        aria-label="Selected {tradeList.direction} side: {money(tradeStats.grossLoss)} gross loss and {money(tradeStats.grossProfit)} gross profit at worst-case fills"
                      >
                        <span class="loss" style="width:{(tradeStats.grossLoss / sideGross) * 100}%"></span>
                        <span class="profit" style="width:{(tradeStats.grossProfit / sideGross) * 100}%"></span>
                      </div>
                      <div class="tt-sidefacts">
                        <span><i class="loss"></i>Gross loss <b>{money(tradeStats.grossLoss)}</b></span>
                        <span><i class="profit"></i>Gross profit <b>{money(tradeStats.grossProfit)}</b></span>
                      </div>
                      <p>
                        Every sealed row agrees on this one selected direction. A simultaneous
                        Long/Short comparison would require a second recorded run, so no missing side is fabricated.
                      </p>
                    </div>
                  {:else}
                    <div class="tt-unavailable-panel">
                      <span>{tradeEvidenceWhy}</span>
                    </div>
                  {/if}

                  <h4 class="tt-h5">Fill models <span class="tt-own">brutex</span></h4>
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
                    <div class="tt-q"><span class="tt-k">Annualized return (CAGR) · current-store normalized</span><span class="tt-qv" class:up={(cagrBps ?? 0) >= 0} class:down={(cagrBps ?? 0) < 0}>{noTrades ? 'none' : cagrBps === null ? '—' : pct(cagrBps)}</span></div>
                    <div class="tt-q"><span class="tt-k">Total return · current-store normalized</span><span class="tt-qv" class:up={!noTrades && (strategyBps ?? 0) >= 0} class:down={!noTrades && (strategyBps ?? 0) < 0}>{noTrades ? "none" : pct(strategyBps)}</span></div>
                    <!-- REAL, ON PER-TRADE RETURNS. Both read "no ratio reaches
                         the record" — true of the ledger's four scalars, untrue
                         of the trade file, which is a series. Annualised by the
                         span's own length, risk-free rate zero, and the note
                         under this row says so. -->
                    <div class="tt-q">
                      <span class="tt-k">Sharpe ratio</span>
                      <span class="tt-qv"
                        >{#if equity && equity.sharpe !== null}{equity.sharpe.toFixed(3)}{:else}<Lock why="Needs at least two trades and the span's opening price to divide by." />{/if}</span
                      >
                    </div>
                    <div class="tt-q">
                      <span class="tt-k">Sortino ratio</span>
                      <span class="tt-qv"
                        >{#if equity && equity.sortino !== null}{equity.sortino.toFixed(3)}{:else}<Lock why="Needs at least two LOSING trades — Sortino divides by downside deviation, and a run with no losses has none to measure." />{/if}</span
                      >
                    </div>
                  </div>

                  <div class="tt-hrow tight">
                    <h4 class="tt-h5">{PERIOD_LABEL[periodScale]} PnL</h4>
                    <div class="tt-seg" role="group" aria-label="Period">
                      {#each PERIOD_SCALES as s (s)}
                        <button class="tt-segbtn" class:on={periodScale === s} aria-pressed={periodScale === s} onclick={() => (periodScale = s)}>{s[0].toUpperCase() + s.slice(1)}</button>
                      {/each}
                    </div>
                  </div>
                  {#if holdPeriods.length > 1}
                    {@render barChart(
                      holdPeriods,
                      [money(barScale(holdPeriods)), money(Math.round(barScale(holdPeriods) / 2)), "0", money(-Math.round(barScale(holdPeriods) / 2)), money(-barScale(holdPeriods))],
                      "Per-period P&L of HOLDING one unit, from the current store. The bars endpoint does not bind these bytes to the run's data digest. The strategy has no per-period series — it records one total for the whole span — so only this current-store reference can be bucketed.",
                      ["Current-store gain", "Current-store loss"]
                    )}
                  {:else}
                    {@render chartFrame(
                      'Fewer than two ' + PERIOD_NOUN[periodScale] + ' buckets are present, so a period-to-period chart is undefined.',
                      ['Realized profit', 'Realized loss', 'Favorable excursion', 'Adverse excursion'],
                      PNL_TICKS,
                      periodTicks,
                      periodScale === 'daily',
                      'undefined'
                    )}
                  {/if}
                  <p class="tt-note2">
                    <b>Current-store reference; not bound to the run's data digest.</b> Return is on
                    one unit of the index, against the current store price at the span's start {bench.phase === 'ready' && bench.open > 0 ? ` (${money(bench.open)})` : ' — not loaded, so every percentage on this tab is withheld rather than divided by zero'}.
                    The ledger records no capital, so a return on equity has no denominator on disk. Annualised over the
                    {exact(openRun.months_found)} months actually found.
                  </p>
                {:else if paTab === 'benchmarking'}
                  <p class="tt-warnnote">
                    <b>Current-store reference; not bound to this run's data digest.</b> The bars
                    endpoint proves its own envelope, not that these are the exact historical bytes
                    named by the sealed run. Persisted strategy paisa remain the run-authoritative
                    values; percentages and comparisons below use today's stored endpoint prices.
                  </p>
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Strategy return · current-store normalized</span><span class="tt-qv" class:up={!noTrades && (strategyBps ?? 0) >= 0} class:down={!noTrades && (strategyBps ?? 0) < 0}>{noTrades ? "none" : pct(strategyBps)}</span></div>
                    <div class="tt-q"><span class="tt-k">Buy and hold return · current store</span><span class="tt-qv" class:up={(buyHold?.bps ?? 0) >= 0} class:down={(buyHold?.bps ?? 0) < 0}>{buyHold ? pct(buyHold.bps) : '—'}</span></div>
                    <div class="tt-q">
                      <span class="tt-k">Strategy outperformance · vs current store</span>
                      <span class="tt-qv" class:up={outperformance?.beat} class:down={outperformance && !outperformance.beat}>
                        {noTrades ? 'none — the sweep never entered' : outperformanceBps !== null ? pct(outperformanceBps) : '—'}
                      </span>
                    </div>
                    <div class="tt-q"><span class="tt-k">Correlation</span><span class="tt-qv"><Lock why="The strategy curve advances at trade exits and the benchmark advances at every bar. No recorded alignment and sampling rule makes those two clocks comparable." /></span></div>
                  </div>
                  <!-- A COMPARISON NEEDS TWO PARTICIPANTS. With no trades this
                       read "The sweep falls short of buy and hold by ₹6,021.60
                       ... 0 trades across 3.6 years to end up behind holding
                       the index" — a verdict on a contest one side never
                       entered. Losing and not playing are different outcomes
                       and this sentence reported them as the same one. -->
                  {#if noTrades}
                    <p class="tt-note2">
                      <b>No comparison is possible.</b> The sweep opened no position, so it neither
                      beat nor lost to holding the index — it was not in the market. The current-store
                      holding reference above is a property of today's endpoint bars, not of this run.
                    </p>
                  {:else if outperformance}
                    <p class="tt-note2" class:badnote={!outperformance.beat}>
                      {#if outperformance.beat}
                        Against the unbound current-store reference, the sweep's persisted worst-case
                        total is <b>higher by {money(outperformance.edge)}</b>.
                      {:else}
                        Against the unbound current-store reference, the sweep's persisted worst-case
                        total is <b>lower by {money(-outperformance.edge)}</b> —
                        {exact(openRun.trades)} trades across {(spanYears ?? 0).toFixed(1)} years to end up behind holding the index.
                      {/if}
                    </p>
                  {/if}
                  <h4 class="tt-h5">Strategy vs current-store reference</h4>
                  {#if holdCurve.length > 1}
                    {@render areaChart(
                      holdCurve,
                      [money(curveScale(holdCurve)), money(Math.round(curveScale(holdCurve) / 2)), "0", money(-Math.round(curveScale(holdCurve) / 2)), money(-curveScale(holdCurve))],
                      curveLabels(holdCurve),
                      "The newest plotted tail of HOLDING one unit from the current store's full-span opening close, bar by bar. It is not data-digest-bound to the run. The strategy curve is recorded on trade-resolution timestamps, so the two series remain separate until a clock-alignment rule is defined.",
                      curveScale(holdCurve)
                    )}
                  {:else}
                    {@render chartFrame(
                      'The benchmark contains fewer than two plotted bars, so it cannot form a curve.',
                      ['Strategy PnL', 'Buy and hold PnL'],
                      PNL_TICKS,
                      periodTicks,
                      false,
                      'unavailable'
                    )}
                  {/if}
                {:else if paTab === 'margin'}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Margin efficiency</span><span class="tt-qv dim">Not applicable</span></div>
                    <div class="tt-q"><span class="tt-k">Average margin used</span><span class="tt-qv dim">Not applicable</span></div>
                    <div class="tt-q"><span class="tt-k">Margin calls</span><span class="tt-qv dim">Not applicable</span></div>
                    <div class="tt-q"><span class="tt-k">Total liquidated volume</span><span class="tt-qv dim">Not applicable</span></div>
                  </div>
                  <h4 class="tt-h5">Margin utilization</h4>
                  <div class="tt-unavailable-panel not-applicable">
                    <span>
                      <b>Not applicable.</b> The engine computes one-unit spot-index totals with no
                      account, position size or broker, so there is no capital, margin call or
                      liquidation to measure.
                    </span>
                  </div>
                {:else}
                  <div class="tt-quad">
                    <!-- REAL. Durations are in TRADES, not bars: the equity
                         curve advances one point per trade, so a duration in
                         bars would need the gaps between them. Naming the unit
                         is the difference between a measurement and a guess. -->
                    <div class="tt-q">
                      <span class="tt-k">Average run-up duration</span>
                      <span class="tt-qv"
                        >{#if equity && equity.avgRunUpDuration !== null}{equity.avgRunUpDuration.toFixed(1)}<em
                            class="tt-unit2">trades</em
                          >{:else}<Lock why="This run has no completed rising stretch — the curve turned fewer than twice." />{/if}</span
                      >
                    </div>
                    <div class="tt-q">
                      <span class="tt-k">Average drawdown duration</span>
                      <span class="tt-qv"
                        >{#if equity && equity.avgDrawdownDuration !== null}{equity.avgDrawdownDuration.toFixed(1)}<em
                            class="tt-unit2">trades</em
                          >{:else}<Lock why="This run has no completed falling stretch — the curve turned fewer than twice." />{/if}</span
                      >
                    </div>
                    <div class="tt-q"><span class="tt-k">Max drawdown</span><span class="tt-qv down">{money(Math.abs(openRun.max_drawdown))}<em class="tt-pc">{drawdownBps === null ? '' : `${(drawdownBps / 100).toFixed(2)}%`}</em></span></div>
                    <div class="tt-q"><span class="tt-k">Max drawdown / current-store open</span><span class="tt-qv">{drawdownBps === null ? '—' : `${(drawdownBps / 100).toFixed(2)}%`}</span></div>
                  </div>
                  <h4 class="tt-h5">Alternating growth and decline</h4>
                  {#if holdSwings.segs.length > 0}
                    {@render swingChart(
                      holdSwings,
                      "Alternating run-up and drawdown of the CURRENT-STORE REFERENCE — one unit held — segmented from the cumulative endpoint curve. It is not bound to the run's data digest. The strategy's own trade-resolution swings are selectable in Performance and summarized below."
                    )}
                  {:else}
                    {@render chartFrame(
                      holdCurve.length < 2
                        ? 'The benchmark contains fewer than two plotted bars, so it cannot form an alternating swing.'
                        : 'The benchmark has no completed alternating turn, so a run-up or drawdown duration is undefined.',
                      ['Run-up', 'Drawdown', 'Current run-up'],
                      PCT_TICKS,
                      periodTicks,
                      false,
                      holdCurve.length < 2 ? 'unavailable' : 'undefined'
                    )}
                  {/if}
                  <!-- TradingView's second block on this tab: run-up and
                       drawdown, each as maximum / average / current, on one
                       shared scale. The maximum drawdown is the one figure
                       recorded, so it is the one bar drawn. -->
                  <h4 class="tt-h5">Comparison of growth and decline periods</h4>
                  <!-- ALL FIVE REAL. "Run-up needs an equity series to measure a
                       rise across" — the trade file IS that series. Every bar is
                       scaled against the largest magnitude in the block, so a
                       run-up and a drawdown are drawn at ONE scale and can be
                       compared by eye, which is the only reason to group them. -->
                  {#if equity}
                    {@const reach = Math.max(
                      1,
                      equity.maxRunUp ?? 0,
                      equity.avgRunUp ?? 0,
                      Math.abs(equity.openStretch),
                      equity.maxDrawdown,
                      equity.avgDrawdown ?? 0
                    )}
                    <div class="tt-cmp">
                      <span class="tt-cmpgrp">Run-up</span>
                      <!-- `Current` CARRIES ITS DIRECTION. The curve ends
                           mid-swing, and that open stretch is a run-up only if it
                           is rising — on this data it usually is not. A negative
                           under a green "Run-up · Current" would state the
                           opposite of what happened, so the row says which it is
                           and takes the matching colour. -->
                      {#each [{ k: 'Maximum', v: equity.maxRunUp, down: false }, { k: 'Average', v: equity.avgRunUp, down: false }, { k: equity.openIsRunUp ? 'Current' : 'Current — falling', v: Math.abs(equity.openStretch), down: !equity.openIsRunUp }] as r (r.k)}
                        <div class="tt-cmprow">
                          <span class="tt-cmplab">{r.k}</span>
                          <span class="tt-cmpbar"
                            >{#if r.v !== null}<span class="tt-cmpfill" class:up={!r.down} class:down={r.down} style="width:{(Math.abs(r.v) / reach) * 100}%"></span>{/if}</span
                          >
                          <span class="tt-cmpval" class:up={!r.down} class:down={r.down}
                            >{#if r.v === null}<Lock small why="This run has no completed rising stretch — the curve turned fewer than twice." />{:else}{money(r.v)}{/if}</span
                          >
                        </div>
                      {/each}
                      <span class="tt-cmpgrp">Drawdown</span>
                      {#each [{ k: 'Maximum', v: equity.maxDrawdown }, { k: 'Average', v: equity.avgDrawdown }] as r (r.k)}
                        <div class="tt-cmprow">
                          <span class="tt-cmplab">{r.k}</span>
                          <span class="tt-cmpbar"
                            >{#if r.v !== null}<span class="tt-cmpfill down" style="width:{(Math.abs(r.v) / reach) * 100}%"></span>{/if}</span
                          >
                          <span class="tt-cmpval down"
                            >{#if r.v === null}<Lock small why="This run has no completed falling stretch — the curve turned fewer than twice." />{:else}{money(r.v)}{/if}</span
                          >
                        </div>
                      {/each}
                    </div>
                    <p class="tt-note2 dim">
                      Run-ups and drawdowns are <b>alternating swings</b> of the equity curve — a
                      rising stretch and a falling one, bounded by the turns between them —
                      averaged over {exact(equity.swings)} of them, and measured in trades because
                      the curve advances one point per trade. <b>Maximum drawdown is different</b>:
                      it is the deepest fall from any high to any later low, which is the figure the
                      ledger also keeps. The chosen rows are accepted only when that reconstructed
                      maximum is exactly the ledger's
                      <b>{money(Math.abs(openRun.max_drawdown))}</b>.
                    </p>
                  {:else}
                    <div class="tt-cmp">
                      <span class="tt-cmpgrp">Run-up</span>
                      {#each ['Maximum', 'Average', 'Current'] as k (k)}
                        <div class="tt-cmprow">
                          <span class="tt-cmplab">{k}</span>
                          <span class="tt-cmpbar"></span>
                          <span class="tt-cmpval"><Lock small why={equityEvidenceWhy} /></span>
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
                        <span class="tt-cmpval"><Lock small why={equityEvidenceWhy} /></span>
                      </div>
                    </div>
                  {/if}

                  <h4 class="tt-h5">Excursion <span class="tt-own">brutex</span></h4>
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
                        <span class="tt-plval" title="{group(e.v)} ppm">{openRun.trades === 0 ? "none" : ppmPct(e.v)}</span>
                      </div>
                    {/each}
                  </div>
                  {/if}
                </div>
              {/key}
              </div>

              <!-- ================= TRADES ANALYSIS ================= -->
              <div class="tt-sec">
                <h3 class="tt-h">Trades analysis</h3>
                {@render chosenGridTradeNote()}
                <div class="tt-pills" role="group" aria-label="Trades analysis">
                  <!-- `Time patterns` IS THE REFERENCE'S FOURTH PILL, and it was
                       the one missing. Its data has been on the wire the whole
                       time: `/trades.json` serves a `weekday` and an `hour`
                       bucket set on every request. -->
                  {#each [['distribution', 'Distribution'], ['streaks', 'Streaks'], ['time', 'Time patterns'], ['details', 'Trades analysis details']] as [key, label] (key)}
                    <button class="tt-pill" class:on={taTab === key} aria-pressed={taTab === key} onclick={() => (taTab = key)}>{label}</button>
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
                    <div class="tt-q"><span class="tt-k">Outliers PnL</span><span class="tt-qv"><Lock why="No outlier rule is part of the recorded run. Choosing a threshold after seeing the results would invent both the subset and its PnL." /></span></div>
                    <div class="tt-q"><span class="tt-k">Largest profit</span><span class="tt-qv">{#if tradeStats?.largestWin}{money(tradeStats.largestWin)}{:else}<Lock why="This run recorded no winning trade at worst-case fills." />{/if}</span></div>
                    <div class="tt-q"><span class="tt-k">Largest loss</span><span class="tt-qv down">{money(Math.abs(openRun.worst_trade))}</span></div>
                  </div>
                  <div class="tt-two">
                    <div>
                      <h4 class="tt-h5">Bar returns distribution</h4>
                      {#if returnHistogram}{@render histogram(returnHistogram,'The distribution of per-BAR returns over the window on screen, from the bars on disk — NOT per trade. The trade file does carry a per-trade result and the donut beside this is cut from it; what it does not carry is a per-trade RETURN, which needs each trade\'s own entry price rather than the span\'s opening one.')}{:else}{@render chartFrame('No bars are loaded, so there is nothing to distribute.', HIST_LEGEND, COUNT_TICKS, RETURN_TICKS, false, 'unavailable')}{/if}
                    </div>
                    <div>
                      <h4 class="tt-h5">Trades distribution</h4>
                      <!-- THE DONUT, DRAWN AS A RING WITH NO SPLIT. The total is
                           real and sits in the middle where TradingView puts it;
                           the arc is not divided because the winner/loser split
                           is exactly what is not recorded. A guessed split here
                           would be the most convincing wrong picture on the page. -->
                      <!-- THE RING IS CUT NOW, AND ON DATA. It was undivided
                           because the split "is the thing that is not recorded"
                           — true of the results ledger, and untrue since the
                           trade file. `worst` is each trade's realised
                           worst-case result, so the three slices are a walk of
                           the list, not a guess. The arc lengths are
                           `stroke-dasharray` over a 276.46 circumference
                           (2*pi*44), offset by the arcs before them. -->
                      <div class="tt-donutwrap">
                        {#if noTrades}
                          <svg class="tt-donut" viewBox="0 0 120 120" role="img" aria-label="0 trades: 0 winners, 0 losers, 0 breakevens.">
                            <circle cx="60" cy="60" r="44" fill="none" stroke="var(--n5)" stroke-width="16" />
                          </svg>
                          <div class="tt-donutmid">
                            <b>{exact(0)}</b>
                            <span>Total trades</span>
                          </div>
                          <ul class="tt-donutleg">
                            <li>
                              <span class="sw up"></span><span class="nm">Winners</span>
                              <span class="ct">{exact(0)} trades</span><span class="pc">—</span>
                            </li>
                            <li>
                              <span class="sw down"></span><span class="nm">Losers</span>
                              <span class="ct">{exact(0)} trades</span><span class="pc">—</span>
                            </li>
                            <li>
                              <span class="sw flat"></span><span class="nm">Breakevens</span>
                              <span class="ct">{exact(0)} trades</span><span class="pc">—</span>
                            </li>
                          </ul>
                        {:else if tradeStats}
                          {@const C = 2 * Math.PI * 44}
                          {@const w = (tradeStats.wins / tradeStats.trades) * C}
                          {@const l = (tradeStats.losses / tradeStats.trades) * C}
                          {@const b = (tradeStats.breakevens / tradeStats.trades) * C}
                          <svg
                            class="tt-donut"
                            viewBox="0 0 120 120"
                            role="img"
                            aria-label="{exact(tradeStats.trades)} trades: {exact(tradeStats.wins)} winners, {exact(tradeStats.losses)} losers, {exact(tradeStats.breakevens)} breakevens, at worst-case fills."
                          >
                            <circle cx="60" cy="60" r="44" fill="none" stroke="var(--up)" stroke-width="16" stroke-dasharray="{w} {C - w}" transform="rotate(-90 60 60)" />
                            <circle cx="60" cy="60" r="44" fill="none" stroke="var(--down)" stroke-width="16" stroke-dasharray="{l} {C - l}" stroke-dashoffset={-w} transform="rotate(-90 60 60)" />
                            <circle cx="60" cy="60" r="44" fill="none" stroke="var(--warn)" stroke-width="16" stroke-dasharray="{b} {C - b}" stroke-dashoffset={-(w + l)} transform="rotate(-90 60 60)" />
                          </svg>
                          <div class="tt-donutmid">
                            <b>{exact(tradeStats.trades)}</b>
                            <span>Total trades</span>
                          </div>
                          <!-- THREE COLUMNS, as TradingView sets it: name,
                               trade count, share of the total. -->
                          <ul class="tt-donutleg">
                            <li>
                              <span class="sw up"></span><span class="nm">Winners</span>
                              <span class="ct">{exact(tradeStats.wins)} trades</span>
                              <span class="pc">{share(Math.round((tradeStats.wins / tradeStats.trades) * 10000))}</span>
                            </li>
                            <li>
                              <span class="sw down"></span><span class="nm">Losers</span>
                              <span class="ct">{exact(tradeStats.losses)} trades</span>
                              <span class="pc">{share(Math.round((tradeStats.losses / tradeStats.trades) * 10000))}</span>
                            </li>
                            <li>
                              <span class="sw flat"></span><span class="nm">Breakevens</span>
                              <span class="ct">{exact(tradeStats.breakevens)} trades</span>
                              <span class="pc">{share(Math.round((tradeStats.breakevens / tradeStats.trades) * 10000))}</span>
                            </li>
                          </ul>
                        {:else}
                          <svg class="tt-donut" viewBox="0 0 120 120" role="img" aria-label="{exact(openRun.trades)} total trades. This run recorded no trade file, so the split is unknown.">
                            <circle cx="60" cy="60" r="44" fill="none" stroke="var(--n5)" stroke-width="16" />
                            <circle cx="60" cy="60" r="44" fill="none" stroke="var(--n7)" stroke-width="16" stroke-dasharray="4 6" opacity="0.7" />
                          </svg>
                          <div class="tt-donutmid">
                            <b>{exact(openRun.trades)}</b>
                            <span>Total trades</span>
                          </div>
                          <ul class="tt-donutleg">
                            <li>
                              <span class="sw up"></span><span class="nm">Winners</span>
                              <span class="ct"><Lock small why={tradeEvidenceWhy} /></span>
                              <span class="pc"><Lock small why="Needs the win count above." /></span>
                            </li>
                            <li>
                              <span class="sw down"></span><span class="nm">Losers</span>
                              <span class="ct"><Lock small why={tradeEvidenceWhy} /></span>
                              <span class="pc"><Lock small why="Needs the loss count above." /></span>
                            </li>
                            <li>
                              <span class="sw flat"></span><span class="nm">Breakevens</span>
                              <span class="ct"><Lock small why={tradeEvidenceWhy} /></span>
                              <span class="pc"><Lock small why="Needs the breakeven count above." /></span>
                            </li>
                          </ul>
                        {/if}
                      </div>
                      {#if noTrades}
                        <p class="tt-note2 dim">
                          The ledger proves this run opened no trade. All three outcome counts are
                          therefore zero; their percentage shares are undefined because zero trades
                          provide no denominator.
                        </p>
                      {:else if tradeStats}
                        <p class="tt-note2 dim">
                          Cut at <b>worst-case fills</b> — a trade counts as a winner only if it
                          won with both legs filled at the printed extreme. At best-case fills the
                          same list holds {exact(tradeTotals ? tradeTotals.wins : tradeStats.wins)}
                          winners. A breakeven is a trade whose worst-case result was exactly zero;
                          folding those into the losers would move the win rate without any trade
                          having lost anything.
                        </p>
                      {:else}
                        <p class="tt-note2">The ring is undivided. {tradeEvidenceWhy}</p>
                      {/if}
                    </div>
                  </div>
                {:else if taTab === 'time'}
                  <!-- ══ RESULTS BY TIME ══
                       The operator's question, quoted verbatim in
                       `crates/api/src/trades.rs`: "every day how many wins how
                       many loss every week every month every quarter every half
                       every year". `cli::trades` answers it and `/trades.json`
                       serves eight grains on every request; this page stored the
                       object and drew none of it.

                       Shaped after the reference: four tiles, then one stacked
                       column per bucket with winners below and losers above —
                       NOT a table. A column chart answers "which bucket" at a
                       glance, which is the whole question. -->
                  {#if timePatterns}
                    <div class="tt-quad">
                      <div class="tt-q">
                        <span class="tt-k">Best hour for entries</span>
                        <span class="tt-qv"
                          >{#if timePatterns.bestHour}{timePatterns.bestHour.label},<em class="tt-unit2"
                              >{share(timePatterns.bestHour.rateBp)} winners</em
                            >{:else}<Lock small why="No hour bucket carries enough trades to name a best one." />{/if}</span
                        >
                      </div>
                      <div class="tt-q">
                        <span class="tt-k">Best day for entries</span>
                        <span class="tt-qv"
                          >{timePatterns.bestWeekday.label},<em class="tt-unit2"
                            >{share(timePatterns.bestWeekday.rateBp)} winners</em
                          ></span
                        >
                      </div>
                      <div class="tt-q">
                        <span class="tt-k">Best month for entries</span>
                        <span class="tt-qv"
                          >{timePatterns.bestCalendarMonth.label},<em class="tt-unit2"
                            >{share(timePatterns.bestCalendarMonth.rateBp)} winners</em
                          ></span
                        >
                      </div>
                      <div class="tt-q">
                        <span class="tt-k">Average trade duration</span>
                        <span class="tt-qv"
                          >{averageDuration === null ? 'not exact' : exact(averageDuration)}<em class="tt-unit2">bars</em></span
                        >
                      </div>
                    </div>

                    <div class="tt-hrow tight">
                      <h4 class="tt-h5">Results by time</h4>
                      <div class="tt-seg" role="group" aria-label="Time grain">
                        {#each TIME_GRAINS as grain (grain.key)}
                          <button
                            class="tt-segbtn"
                            class:on={timeGrain === grain.key}
                            aria-pressed={timeGrain === grain.key}
                            onclick={() => {
                              timeGrain = grain.key;
                              timePage = 0;
                            }}>{grain.label}</button
                          >
                        {/each}
                      </div>
                    </div>

                    {#if timeRows.rows.length > 0}
                      <div class="rbt">
                        <div class="rbt-plot">
                          <div class="rbt-axis" aria-hidden="true">
                            <span>{exact(timeRows.tallest)}</span>
                            <span>{exact(Math.round(timeRows.tallest / 2))}</span>
                            <span>0</span>
                          </div>
                          {#each timeRows.rows as r (r.key)}
                            <div
                              class="rbt-col"
                              role="img"
                              aria-label="{r.label}: {exact(r.trades)} trades, {exact(r.wins)} winners and {exact(r.losses)} losers at worst-case fills"
                              title="{r.label} · {exact(r.trades)} trades · {exact(r.wins)} won at worst-case fills"
                            >
                              <div class="rbt-stack">
                                <!-- LOSERS ABOVE, WINNERS BELOW, as the reference
                                     stacks them: the green base is what the
                                     bucket kept and the red is what sat on top
                                     of it. -->
                                <span
                                  class="rbt-loss"
                                  style="height:{(r.losses / timeRows.tallest) * 100}%"
                                ></span>
                                <span
                                  class="rbt-win"
                                  style="height:{(r.wins / timeRows.tallest) * 100}%"
                                ></span>
                              </div>
                              <span class="rbt-x">{r.label}</span>
                            </div>
                          {/each}
                        </div>
                        <ul class="rbt-leg">
                          <li><i class="rbt-dot win"></i>Winners</li>
                          <li><i class="rbt-dot loss"></i>Losers</li>
                        </ul>
                        {#if timeRows.pages > 1}
                          <div class="series-nav" aria-label="Time bucket pages">
                            <button
                              aria-label="Earlier time buckets"
                              disabled={timeRows.page === 0}
                              onclick={() => (timePage = Math.max(0, timeRows.page - 1))}>‹</button
                            >
                            <span
                              >Buckets {exact(timeRows.start + 1)}–{exact(
                                Math.min(timeRows.start + timeRows.rows.length, timeRows.total)
                              )} of {exact(timeRows.total)}</span
                            >
                            <button
                              aria-label="Later time buckets"
                              disabled={timeRows.page + 1 >= timeRows.pages}
                              onclick={() => (timePage = Math.min(timeRows.pages - 1, timeRows.page + 1))}>›</button
                            >
                          </div>
                        {/if}
                      </div>
                      <p class="tt-note2 dim">
                        <!-- THIS PRINTED THE WORST-CASE WINNER TOTAL AS A
                             DIFFERENCE. `shape()` sets `losses = trades - wins`
                             where `wins` is already `worst_wins`, so
                             `trades - losses` IS the worst-case winner count —
                             the sentence said "N more at best-case" while N was
                             the number already shown as the worst-case
                             numerator. `shape()` discards `b.wins`, so the
                             best-case figure was not even in scope. It comes
                             from `tradeTotals`, which counts both. -->
                        Winners are counted at <b>worst-case fills</b>, the reading this page is
                        headed by{#if tradeTotals} — the same trades count
                          <b>{exact(tradeTotals.wins - tradeTotals.worstWins)}</b> more at
                          best-case{/if}. A bucket must hold at least {exact(timePatterns.floor)} trades
                        — a twentieth of the run's {exact(timePatterns.total)} — before it can be
                        named a best one above, so a single lucky trade cannot take the tile.
                        {#if timeGrain === 'hour'}
                          Buckets are keyed by whole <b>IST exchange-calendar hours</b>; the stored
                          epoch instant is shifted exactly once before categorisation.
                        {:else if timeGrain === 'week'}
                          Weekly buckets begin on <b>Monday in IST</b>. Saturday and Sunday retain
                          deterministic slots for validation, while normal NSE trading is Monday–Friday.
                        {/if}
                      </p>
                    {:else}
                      <p class="tt-note2">This grain recorded no bucket for this run.</p>
                    {/if}
                  {:else}
                    <p class="tt-note2">
                      {#if tradeList.why}{tradeList.why}
                      {:else if tradeList.phase === 'loading'}Reading this run's trades…
                      {:else}This run recorded no trade file, so it has no time pattern to show.{/if}
                    </p>
                  {/if}
                {:else if taTab === 'streaks'}
                  <!-- REAL. These four read "Needs the ordered win/loss outcome
                       of every trade" — which the trade file IS. `seq` is the
                       order trades resolved in, and `worst` is each one's
                       realised result, so the sequence of outcomes is a walk. -->
                  <div class="tt-quad">
                    <div class="tt-q">
                      <span class="tt-k">Longest winning streak</span>
                      <span class="tt-qv"
                        >{#if noTrades}{exact(0)}<em class="tt-unit2">trades</em
                          >{:else if tradeStats}{exact(tradeStats.longestWin)}<em class="tt-unit2">trades</em
                          >{:else}<Lock why={tradeEvidenceWhy} />{/if}</span
                      >
                    </div>
                    <div class="tt-q">
                      <span class="tt-k">Longest losing streak</span>
                      <span class="tt-qv"
                        >{#if noTrades}{exact(0)}<em class="tt-unit2">trades</em
                          >{:else if tradeStats}{exact(tradeStats.longestLoss)}<em class="tt-unit2">trades</em
                          >{:else}<Lock why={tradeEvidenceWhy} />{/if}</span
                      >
                    </div>
                    <div class="tt-q">
                      <span class="tt-k">Average winning streak</span>
                      <span class="tt-qv"
                        >{#if tradeStats?.avgWinStreak !== null && tradeStats}{tradeStats.avgWinStreak?.toFixed(1)}<em
                            class="tt-unit2">trades</em
                          >{:else}<Lock why="This run recorded no winning trade, so it has no winning streak." />{/if}</span
                      >
                    </div>
                    <div class="tt-q">
                      <span class="tt-k">Average losing streak</span>
                      <span class="tt-qv"
                        >{#if tradeStats?.avgLossStreak !== null && tradeStats}{tradeStats.avgLossStreak?.toFixed(1)}<em
                            class="tt-unit2">trades</em
                          >{:else}<Lock why="This run recorded no losing trade, so it has no losing streak." />{/if}</span
                      >
                    </div>
                  </div>
                  <div class="tt-hrow tight">
                    <h4 class="tt-h5">Winning and losing streaks</h4>
                    <div class="tt-seg" role="group" aria-label="Streak unit">
                      <button class="tt-segbtn" class:on={streakMode === 'count'} aria-pressed={streakMode === 'count'} disabled={streakRows.rows.length === 0} onclick={() => (streakMode = 'count')}>Count</button>
                      <button class="tt-segbtn" class:on={streakMode === 'amount'} aria-pressed={streakMode === 'amount'} disabled={streakRows.rows.length === 0} onclick={() => (streakMode = 'amount')}>Amount</button>
                    </div>
                  </div>
                  {#if streakRows.rows.length > 0}
                    <div class="streak-chart">
                      <div class="streak-plot">
                        <div class="streak-axis" aria-hidden="true">
                          <span>{streakMode === 'amount' ? money(streakRows.tallest) : exact(streakRows.tallest)}</span>
                          <span>0</span>
                          <span>{streakMode === 'amount' ? money(-streakRows.tallest) : exact(streakRows.tallest)}</span>
                        </div>
                        {#each streakRows.rows as run, i (run.seq)}
                          <div
                            class="streak-col"
                            role="img"
                            aria-label="Trade {run.index}: {run.kind === 'win' ? 'winning' : 'losing or flat'} streak step {exact(run.count)}, {money(run.amount)} absolute worst-fill PnL in streak {run.runIndex}"
                            title="Trade {run.index} · streak {run.runIndex} · {exact(run.count)} {run.kind === 'win' ? 'winning' : 'losing'} in sequence · {money(run.amount)}"
                          >
                            <div class="streak-half top">
                              {#if run.kind === 'win'}
                                <span style="height:{(run.value / streakRows.tallest) * 100}%"></span>
                              {/if}
                            </div>
                            <div class="streak-half bottom">
                              {#if run.kind === 'loss'}
                                <span style="height:{(run.value / streakRows.tallest) * 100}%"></span>
                              {/if}
                            </div>
                            <small>{i % 12 === 0 ? exact(run.index) : ''}</small>
                          </div>
                        {/each}
                      </div>
                      <ul class="rbt-leg">
                        <li><i class="rbt-dot win"></i>Winning streak</li>
                        <li><i class="rbt-dot loss"></i>Losing or flat streak</li>
                      </ul>
                      {#if streakRows.pages > 1}
                        <div class="series-nav" aria-label="Streak pages">
                          <button
                            aria-label="Earlier streaks"
                            disabled={streakRows.page === 0}
                            onclick={() => (streakPage = Math.max(0, streakRows.page - 1))}>‹</button
                          >
                          <span
                            >Trades {exact(streakRows.start + 1)}–{exact(
                              Math.min(streakRows.start + streakRows.rows.length, streakRows.total)
                            )} of {exact(streakRows.total)}</span
                          >
                          <button
                            aria-label="Later streaks"
                            disabled={streakRows.page + 1 >= streakRows.pages}
                            onclick={() => (streakPage = Math.min(streakRows.pages - 1, streakRows.page + 1))}>›</button
                          >
                        </div>
                      {/if}
                      <p class="tt-note2 dim">
                        Each bar is the running streak at that resolved trade. Count shows its current
                        length; Amount shows its cumulative absolute worst-fill P&amp;L. A flat trade ends a winning run
                        and joins the losing run, matching the Rust grid's conservative streak rule.
                      </p>
                    </div>
                  {:else}
                    <p class="tt-note2">This run recorded no ordered trade from which to form a streak.</p>
                  {/if}
                {:else}
                  <p class="tt-note2 run-scope">
                    Selected direction:
                    <b>{tradeList.direction ?? 'not available for this uncommitted/legacy result'}</b>.
                    It is sealed receipt metadata and every chosen row must agree. The frequency
                    sweep identity remains undirected, so direction is never guessed from its hash.
                    A directional comparison needs two selected results—not two columns fabricated
                    from this one.
                  </p>
                  <div class="tt-tblwrap">
                    <table class="tt-tbl run-metrics">
                      <thead>
                        <tr><th>Metric</th><th class="n">This run</th></tr>
                      </thead>
                      <tbody>
                        <tr><td>Total trades</td><td class="n">{exact(openRun.trades)}</td></tr>
                        {#if showAllMetrics}
                        <tr><td>Total open trades</td><td class="n"><span class="tv">0</span><span class="tp">closed at span end</span></td></tr>
                        <!-- REAL SINCE `/trades.json` STARTED SERVING `periods`.
                             These three read "No win count is recorded", which
                             was true of the RESULTS ledger and stopped being
                             true of the trade file. Both fill models are shown
                             because they disagree by seventeen points of win
                             rate on this run, and the page's header commits to
                             the worst one. -->
                        <tr>
                          <td>Total winners</td>
                          <td class="n up">{#if noTrades}{exact(0)}{:else if tradeTotals}{exact(tradeTotals.worstWins)}<em class="tt-pc2">{exact(tradeTotals.wins)} at best fills</em>{:else}<Lock small why={tradeEvidenceWhy} />{/if}</td>
                        </tr>
                        <tr>
                          <td>Total losers</td>
                          <td class="n down">{#if noTrades}{exact(0)}{:else if tradeTotals}{exact(tradeTotals.losses)}<em class="tt-pc2">{exact(tradeTotals.bestLosses)} at best fills</em>{:else}<Lock small why={tradeEvidenceWhy} />{/if}</td>
                        </tr>
                        <tr>
                          <td>Percent profitable</td>
                          <td class="n">{#if noTrades}<Lock small kind="undefined" why="Zero trades have no profitable percentage denominator." />{:else if tradeTotals}{share(tradeTotals.rateBp)}<em class="tt-pc2">{share(tradeTotals.bestRateBp)} at best fills</em>{:else}<Lock small why={tradeEvidenceWhy} />{/if}</td>
                        </tr>
                        {/if}
                        <tr><td>Average PnL</td><td class="n"><span class="tv">{perTrade ? money(perTrade.worst) : "—"}</span><span class="tp">{perTrade && bench.open > 0 ? `${((perTrade.worst / bench.open) * 100).toFixed(2)}% of current-store open` : ""}</span></td></tr>
                        {#if showAllMetrics}
                        <!-- FOUR MORE THE TRADE FILE MAKES REAL. Each needed
                             "gross profit and a winner count", both of which are
                             one walk of `worst` — the realised worst-case result
                             per round trip. The reference prints `Average loss`
                             as a POSITIVE magnitude, so it is not negated here. -->
                        <tr><td>Average profit</td><td class="n up">{#if tradeStats?.avgWin !== null && tradeStats}{money(tradeStats.avgWin)}{:else}<Lock small why="No winning trade at worst-case fills." />{/if}</td></tr>
                        <tr><td>Average loss</td><td class="n">{#if tradeStats?.avgLoss !== null && tradeStats}{money(tradeStats.avgLoss)}{:else}<Lock small why="No losing trade at worst-case fills." />{/if}</td></tr>
                        <tr><td>Average profit / average loss</td><td class="n">{#if tradeStats?.winLossRatio !== null && tradeStats}{tradeStats.winLossRatio?.toFixed(3)}{:else}<Lock small why="Needs both a winning and a losing trade to form the ratio." />{/if}</td></tr>
                        <tr><td>Largest profit</td><td class="n up">{#if tradeStats?.largestWin}{money(tradeStats.largestWin)}{:else}<Lock small why="No winning trade at worst-case fills." />{/if}</td></tr>
                        <!-- TWO OF THESE WERE BARE PADLOCKS OVER ARITHMETIC THE
                             PAGE ALREADY DOES ONE ROW AWAY. `largestWin` and
                             `grossProfit` are both on `tradeStats`, and the
                             identical division against `bench.open` is written
                             out for `Largest loss` below. A lock with no `why`
                             is what `Lock.svelte` calls "a lock that teaches an
                             operator to stop asking". -->
                        <tr><td>Largest profit / current-store open</td><td class="n">{#if tradeStats?.largestWin && shareOfOpen(tradeStats.largestWin)}{shareOfOpen(tradeStats.largestWin)}{:else}<Lock small why="Needs a winning trade and the current store's span opening price to divide by." />{/if}</td></tr>
                        <tr><td>Largest profit as % of gross profit</td><td class="n">{#if tradeStats?.largestWin && tradeStats.grossProfit > 0}{share(Math.round((tradeStats.largestWin / tradeStats.grossProfit) * 10000))}{:else}<Lock small why="Needs a winning trade — with no gross profit there is nothing for the largest one to be a share of." />{/if}</td></tr>
                        {/if}
                        <tr><td>Largest loss</td><td class="n down"><span class="tv">{money(openRun.worst_trade)}</span><span class="tp">{bench.open > 0 ? `${((openRun.worst_trade / bench.open) * 100).toFixed(2)}% of current-store open` : ""}</span></td></tr>
                        {#if showAllMetrics}
                        <tr><td>Largest loss %</td><td class="n"><Lock small why="Needs the entry price of that specific trade; the durable trade row records its result and bar positions, not its price." /></td></tr>
                        <tr><td>Largest loss as % of gross loss</td><td class="n">{#if tradeStats?.largestLoss && tradeStats.grossLoss > 0}{share(Math.round((tradeStats.largestLoss / tradeStats.grossLoss) * 10000))}{:else}<Lock small why="Needs a losing trade — with no gross loss there is nothing for the largest one to be a share of." />{/if}</td></tr>
                        <tr><td>Outliers</td><td class="n"><Lock small why="No outlier rule is part of the recorded run. Choosing a threshold after seeing the results would invent one." /></td></tr>
                        <tr><td>Outliers P&amp;L</td><td class="n"><Lock small why="No outlier rule is part of the recorded run, so there is no honest subset whose P&amp;L can be summed." /></td></tr>
                        <!-- THE OLD LOCK WAS RIGHT ABOUT THE WRONG ARITHMETIC.
                             It refused `bars / trades` because that is the mean
                             gap BETWEEN trades, not the mean length OF one —
                             correct, and the reason it stayed locked. The trade
                             file carries `bars_held` per round trip, so the mean
                             of THAT is the quantity the row names, and the
                             winners/losers split falls out of the same walk. -->
                        <tr><td>Average bars in trades</td><td class="n">{#if noTrades}<Lock small kind="undefined" why="Zero trades have no average duration." />{:else if tradeStats}{exact(tradeStats.avgBars)}{:else}<Lock small why={tradeEvidenceWhy} />{/if}</td></tr>
                        <tr><td>Average bars in winners</td><td class="n">{#if tradeStats?.avgBarsWin !== null && tradeStats}{exact(tradeStats.avgBarsWin)}{:else}<Lock small why="No winning trade at worst-case fills." />{/if}</td></tr>
                        <tr><td>Average bars in losers</td><td class="n">{#if tradeStats?.avgBarsLoss !== null && tradeStats}{exact(tradeStats.avgBarsLoss)}{:else}<Lock small why="No losing trade at worst-case fills." />{/if}</td></tr>
                        {/if}
                        <tr class="own"><td title="MAE — maximum adverse excursion, winners only">How far a winner fell before it paid <span class="tt-own">brutex</span></td><td class="n">{openRun.trades === 0 ? "none" : ppmPct(openRun.winner_mae)}</td></tr>
                        <tr class="own"><td title="MFE — maximum favourable excursion, winners only">How far a winner rose at its best <span class="tt-own">brutex</span></td><td class="n">{openRun.trades === 0 ? "none" : ppmPct(openRun.winner_mfe)}</td></tr>
                        <tr class="own"><td title="MAE — maximum adverse excursion, every trade">How far any trade fell <span class="tt-own">brutex</span></td><td class="n">{openRun.trades === 0 ? "none" : ppmPct(openRun.all_mae)}</td></tr>
                        <tr class="own"><td>Signal bars swept <span class="tt-own">brutex</span></td><td class="n">{exact(openRun.bars)}</td></tr>
                        <!-- SEVENTEEN ROWS OF PADLOCK, BEHIND ONE LINE.
                             Measured: this table was 24 rows and 96 cells with
                             **65 of them locked -- 68%** -- 1,406px of panel to
                             deliver seven real values. Every lock is honest and
                             carries its own reason, and seventeen of them in a
                             row is still a wall that hides the seven.

                             They are NOT deleted. §4 bans hiding a fact and
                             "this metric needs a trade list the sweep does not
                             keep" IS one -- it is the difference between this
                             tool and the one it is modelled on. It is one
                             click away instead of unavoidable, and the line
                             says how many and why, so nothing is a surprise
                             behind it. -->
                        <tr class="tt-more">
                          <td colspan="2">
                            <button
                              class="linky"
                              aria-expanded={showAllMetrics}
                              onclick={() => (showAllMetrics = !showAllMetrics)}
                            >
                              {showAllMetrics ? 'Show only the headline rows' : 'Show every metric'}
                            </button>
                            <span class="dim sm">
                              — unavailable values stay in place as neutral dashes; activate or hover one for
                              the exact missing field or undefined denominator.
                            </span>
                          </td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                  {/if}
                </div>
              {/key}
              </div>
            {:else if testerView === 'trades'}
              <!-- ================= LIST OF TRADES ================= -->
              <div class="tt-sec trade-list">
                <div class="tt-hrow">
                  <h3 class="tt-h">List of trades</h3>
                  {@render chosenGridTradeNote()}
                </div>
                <p class="tt-note2 trade-schema">
                  One trade keeps its entry/exit times and bar positions, duration, worst-fill and
                  best-fill P&amp;L, cumulative worst-fill result, selected direction, and exact MAE/MFE
                  in paisa and ppm. Entry/exit prices and the exit-cause tag are not recorded, so
                  those are not invented. Statutory charges are zero for this spot-index sweep;
                  spread remains unmodeled.
                </p>
                {#if tradeRowsWindow.total > 0}
                  <div class="series-nav trade-pages" aria-label="Trade pages">
                    <button
                      aria-label="Newer trades"
                      disabled={tradeRowsWindow.page === 0}
                      onclick={() => (tradePage = Math.max(0, tradeRowsWindow.page - 1))}>‹</button
                    >
                    <span
                      >Trades {exact(tradeRowsWindow.start + 1)}–{exact(
                        Math.min(
                          tradeRowsWindow.start + tradeRowsWindow.rows.length,
                          tradeRowsWindow.total
                        )
                      )} of {exact(tradeRowsWindow.total)} · newest first</span
                    >
                    <button
                      aria-label="Older trades"
                      disabled={tradeRowsWindow.page + 1 >= tradeRowsWindow.pages}
                      onclick={() =>
                        (tradePage = Math.min(
                          tradeRowsWindow.pages - 1,
                          tradeRowsWindow.page + 1
                        ))}>›</button
                    >
                  </div>
                {/if}
                <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                <div
                  class="tt-tblwrap trade-ledger-wrap"
                  role="region"
                  aria-label="Trade list; scroll horizontally and vertically"
                  tabindex="0"
                >
                  <table class="tt-tbl trade-ledger">
                    <thead>
                      <tr>
                        <th aria-sort="descending">Trade <span class="lot-sort" aria-hidden="true">↓</span><span class="sr-only">, newest first</span></th>
                        <th>Leg</th>
                        <th>Direction</th>
                        <th>Date and time</th>
                        <th class="n">Bar</th>
                        <th class="n">Worst-fill PnL</th>
                        <th class="n">Best-fill PnL</th>
                        <th class="n" title="Worst-fill PnL divided by the current store's span opening close; the endpoint does not bind it to the run data digest and entry price is not recorded.">PnL / current-store open</th>
                        <th class="n" title="A running sum of each trade's realised worst-fill result.">Cumulative PnL</th>
                        <th class="n" title="Maximum adverse excursion from entry.">MAE</th>
                        <th class="n" title="Maximum favourable excursion from entry.">MFE</th>
                        <th class="n">Duration</th>
                      </tr>
                    </thead>
                    <tbody>
                      <!-- One trade stays a two-row Exit/Entry block like the
                           reference. Only durable fields get columns. -->
                      {#if tradeList.phase === 'ready' && tradeRowsShown.length > 0}
                        {#each tradeRowsShown as t (t.seq)}
                          <tr class="lot-a">
                            <td rowspan="2" class="lot-num">{t.seq + 1}</td>
                            <td>Exit</td>
                            <td rowspan="2">{t.direction}</td>
                            <td>{tradeWhen(t.exit_micros)}</td>
                            <td class="n">{exact(t.exit_bar)}</td>
                            <td rowspan="2" class="n {t.worst < 0 ? 'down' : 'up'}">{money(t.worst)}</td>
                            <td rowspan="2" class="n {t.best < 0 ? 'down' : 'up'}">{money(t.best)}</td>
                            <td rowspan="2" class="n {t.worst < 0 ? 'down' : 'up'}">
                              {shareOfOpen(t.worst) ?? '—'}
                            </td>
                            <td rowspan="2" class="n">
                              {money(t.cumulative)}
                              {#if shareOfOpen(t.cumulative)}<em class="lot-pc">{shareOfOpen(t.cumulative)}</em>{/if}
                            </td>
                            <td rowspan="2" class="n down">
                              {money(t.adverse_paisa)}<em class="lot-pc">{ppmPct(t.adverse_ppm)}</em>
                            </td>
                            <td rowspan="2" class="n up">
                              {money(t.favourable_paisa)}<em class="lot-pc">{ppmPct(t.favourable_ppm)}</em>
                            </td>
                            <td rowspan="2" class="n">{exact(t.bars_held)} bars</td>
                          </tr>
                          <tr class="lot-b">
                            <td>Entry</td>
                            <td>{tradeWhen(t.entry_micros)}</td>
                            <td class="n" title="signal bar {exact(t.signal_bar)}">{exact(t.entry_bar)}</td>
                          </tr>
                        {/each}
                      {:else if tradeList.phase === 'loading'}
                        <tr><td colspan="12" class="dim">Reading this run's trades…</td></tr>
                      {:else}
                        <!-- NAMED, NOT BLANK. An empty table and a failed fetch
                             look identical unless one of them says so, and the
                             route answers 200-with-a-reason precisely so the
                             two can be told apart here. -->
                        <tr>
                          <td colspan="12">
                            <p class="tt-note2">
                              {#if tradeList.why}{tradeList.why}
                              {:else if tradeList.phase === 'ready'}This run recorded no round trips.
                              {:else}Open a run to read its trades.{/if}
                            </p>
                          </td>
                        </tr>
                      {/if}
                    </tbody>
                  </table>
                </div>
              </div>

              <!-- ============ TOP COMBINATIONS, RANKED ON THE OPERATOR'S OWN CRITERIA ============
                   The ledger above is the ONE combination the exit grid
                   chose. These are the rest of the frontier, ranked by the
                   eleven measurements the operator named -- and ranked HERE
                   rather than in Rust, because a score compiled into the binary
                   is one more number nobody can see. -->
              <div class="tt-sec combo-ranking">
                <div class="tt-hrow">
                  <h3 class="tt-h">Top {topShown} combinations — ranked on your criteria</h3>
                </div>
                {#if combos.phase === 'ready' && ranked.length > 0}
                  <div class="tt-tblwrap">
                    <table class="tt-tbl rungprog">
                      <thead>
                        <tr>
                          <th>#</th><th>rules</th><th class="n">score</th><th class="n">trades</th>
                          <th class="n">win %</th><th class="n">losses</th>
                          <th class="n">worst fills</th><th class="n">max drawdown</th>
                          <th class="n">worst trade</th><th class="n">smallest win</th>
                          <th class="n">avg win</th><th class="n">avg loss</th>
                          <th class="n">reward:risk</th>
                        </tr>
                      </thead>
                      <tbody>
                        {#each ranked as c, i (c.rank)}
                          <tr class="cmbrow">
                            <td><b>{i + 1}</b><span class="dim"> (rank {c.rank})</span></td>
                            <td>{@render verdictPill(c.meets)}</td>
                            <td class="n">{c.score.toFixed(2)}</td>
                            <td class="n">{exact(c.trades)}</td>
                            <td class="n">{(c.win_rate_bp / 100).toFixed(1)}%</td>
                            <td class="n">{exact(c.losses)}</td>
                            <td class="n {c.pessimistic < 0 ? 'down' : 'up'}">{money(c.pessimistic)}</td>
                            <td class="n down">{money(c.max_drawdown)}</td>
                            <td class="n down">{money(c.worst_trade)}</td>
                            <td class="n up">{money(c.min_win)}</td>
                            <td class="n up">{money(c.avg_win)}</td>
                            <td class="n down">{money(c.avg_loss)}</td>
                            <td class="n">{c.reward_to_risk_bp === null ? "—" : (c.reward_to_risk_bp / 100).toFixed(2) + "×"}</td>
                          </tr>
                          <tr class="namerow">
                            <td colspan="13">{@render conditionNames(c.mask_words)}</td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </div>
                  <!-- SAID OUT LOUD, because the difference decides whether the
                       table above is a finding or an artefact. -->
                  <p class="tt-note2">
                    {ranked.length} of {combos.rows.length} shown.
                    {#if combos.rules}
                      <b
                        >{ranked.filter((r) => r.meets?.all).length} of {ranked.length} meet your rules</b
                      >
                      — win rate ≥ {(combos.rules.min_win_rate_bp / 100).toFixed(0)}%, reward:risk ≥
                      {(combos.rules.min_rr_bp / 100).toFixed(2)}×, return over drawdown ≥
                      {(combos.rules.min_ret_over_dd_bp / 100).toFixed(2)}×, at least
                      {exact(combos.rules.min_trades)} trades.
                      <b>The stop rule is not checked here</b> — it is judged on the worst adverse
                      excursion across every trade, and a ranked row does not store that.
                    {/if}
                    {#if combos.rows.filter((r) => !r.priced).length > 0}
                      <b>{combos.rows.filter((r) => !r.priced).length} excluded as never priced</b> —
                      the screen prices only the strongest few hundred of the hundreds of thousands
                      the sweep enumerates, and an unpriced row stores zeros. A zero drawdown is a
                      spectacular result, so including them would put the combinations nobody
                      measured at the top of this table.
                    {/if}
                  </p>
                {:else if combos.phase === 'loading'}
                  <p class="tt-note2 dim">Reading this run's combinations…</p>
                {:else}
                  <p class="tt-note2">
                    {#if combos.why}{combos.why}
                    {:else if combos.phase === 'ready'}No priced combinations were recorded for this run.
                    {:else}Open a run to rank its combinations.{/if}
                  </p>
                {/if}
              </div>
            {:else}
              <!-- ================= PROPERTIES ================= -->
              <div class="tt-sec">
                <h3 class="tt-h">Properties</h3>
                <p class="tt-note2">
                  What the sweep was actually asked to do. TradingView shows the author's inputs here; this shows the run's
                  own terms, which are the nine that make up its identity.
                </p>
                <div class="tt-tblwrap">
                  <table class="tt-tbl">
                    <caption class="sr-only">Recorded run properties</caption>
                    <tbody>
                      <tr><th class="tt-rowhead" scope="row">Feed</th><td class="n">{openRun.feed}</td></tr>
                      <tr><th class="tt-rowhead" scope="row">Instrument</th><td class="n">{openRun.underlying}</td></tr>
                      <tr><th class="tt-rowhead" scope="row">Signal rung</th><td class="n">{openRun.timeframe}</td></tr>
                      <tr><th class="tt-rowhead" scope="row">Execution rung</th><td class="n">1min — always</td></tr>
                      <tr><th class="tt-rowhead" scope="row">Span asked</th><td class="n">{span(openRun)} · {exact(openRun.months_asked)} months</td></tr>
                      <tr class:warnrow={!openRun.whole_span}><th class="tt-rowhead" scope="row">Span found</th><td class="n">{exact(openRun.months_found)} months{openRun.whole_span ? '' : ' — a SHORTER sample, not a corrected one'}</td></tr>
                      <tr><th class="tt-rowhead" scope="row">Signal bars swept</th><td class="n">{exact(openRun.bars)}</td></tr>
                      <tr><th class="tt-rowhead" scope="row" title="The support threshold. A combination had to hit at least this many bars to survive the ladder.">How often a pattern had to appear</th><td class="n">{exact(openRun.min_hits)} times · {(supportPerMille(openRun) / 10).toFixed(1)}% of bars</td></tr>
                      <tr><th class="tt-rowhead" scope="row">Combinations enumerated</th><td class="n">{exact(openRun.combinations)}</td></tr>
                      <tr class:warnrow={openRun.halted}><th class="tt-rowhead" scope="row">Ladder depth</th><td class="n">{openRun.depth}{openRun.halted ? ' — PARTIAL, a budget stopped the walk' : ' — ran to extinction'}</td></tr>
                      <tr><th class="tt-rowhead" scope="row">Recorded</th><td class="n">{when(openRun.finished_micros)}</td></tr>
                    </tbody>
                  </table>
                </div>

                <h4 class="tt-h5">Exit geometry</h4>
                <div class="tt-tblwrap">
                  <table class="tt-tbl">
                    <caption class="sr-only">Exit geometry</caption>
                    <tbody>
                      {#each openRun.exit_rungs ?? [] as rung, i (i)}
                        <tr class:offrow={rung < 0}>
                          <th class="tt-rowhead" scope="row">{EXIT_AXES[i] ?? `axis ${i} — not named by this build`}</th>
                          <td class="n">{rung < 0 ? 'no rung — not used' : `rung ${rung}`}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>

                <h4 class="tt-h5">The ladder <span class="tt-own">not recorded</span></h4>
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
                  <h4 class="tt-h5">The winning combination <span class="tt-own">server too old</span></h4>
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
                  <h4 class="tt-h5">The winning combination <span class="tt-own">not in this file</span></h4>
                  <p class="tt-note2">
                    <b>This ledger predates the condition mask.</b> It is format version
                    <b>{load.body?.version ?? '?'}</b>, which stored the run's identity — a blake3 over the mask and
                    eight other terms — and not the mask itself. This is <b>not</b> a run that found no conditions:
                    the field did not exist when it was recorded. A run swept by this build records it.
                  </p>
                {:else}
                  {@const decoded = decodeMaskWords(openRun.mask_words)}
                  {@const positions = decoded.positions}
                  <h4 class="tt-h5">
                    The winning combination
                    <span class="tt-own"
                      >{decoded.ok
                        ? `${positions.length} condition${positions.length === 1 ? '' : 's'}`
                        : 'not decodable'}</span
                    >
                  </h4>
                  {#if !decoded.ok}
                    <p class="tt-note2">
                      <b>The winning combination cannot be decoded.</b> {decoded.why} No subset of
                      the mask is shown because a partial condition list would name a different
                      strategy while looking valid.
                    </p>
                  {:else if positions.length === 0}
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
                        <caption class="sr-only">Winning combination conditions</caption>
                        <tbody>
                          {#each positions as position (position)}
                            {@const bit = vocab.bits.get(position)}
                            <tr class:offrow={bit ? !bit.live : false}>
                              <th class="tt-rowhead" scope="row">{bit ? bit.name : `position ${position} — not named by this vocabulary`}</th>
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
  {:else}
    <!-- THE ARM THAT SHOULD NEVER DRAW, AND SAYS SO RATHER THAN SHOWING
         NOTHING. Reached only if the ledger read reports `ready` while its
         body is null — no writer in `crates/api/src/backtest.rs` produces
         that, and every writer here pairs `body: null` with `phase: 'failed'`.
         It exists because the alternative to narrowing was `ledger?.`
         everywhere, which would have turned a throw into a silently blank
         page: §4 allows a degraded answer that names its reason and forbids
         one that hides it. -->
    <div class="panel bt-note" role="alert">
      <h2>The ledger read returned nothing to show</h2>
      <p>
        The request reported success and carried no ledger. That is a state this page has no
        reading for — not an empty ledger, which says so itself, and not a failed read, which
        carries its reason. Press <b>Re-read ledger</b>; if it recurs, the response body is the
        thing to look at.
      </p>
    </div>
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
    /* AND `min-width: 0` FOR THE SAME REASON ON THE OTHER AXIS, which was
       missing and cost 147px. A flex item's `min-width` defaults to `auto`,
       so this column floored at its own min-content and pushed `.main`
       wider than the shell that contains it: MEASURED at a 760px viewport,
       `.shell` 760, `.main` 907, body scroll 147px. `/db` and `/ingest`
       measure 760/760/0 at the same width, so this was not the layout --
       it was this page, and only this page.

       A sideways-scrolling body moves every column of every section below
       it. Wide content still scrolls, but inside its own container: that is
       what `.tbl-scroll` is for. */
    min-width: 0;
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
  /* THE HEADER HAD NO GROUND OF ITS OWN. Title, one grey line and a ghost
     button, floating on the page colour above a panel that carried all the
     weight -- so the eye started at the panel and the page had no top. It
     takes the accent rail every section heading here already uses, and the
     standfirst sits on the rail rather than under a title that is not
     connected to anything. */
  .head {
    position: relative;
    padding: 0.1rem 0 0.3rem 0.85rem;
    margin-bottom: 0.9rem;
    align-items: center;
  }
  .head::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0.2em;
    bottom: 0.35em;
    width: 3px;
    border-radius: 2px;
    background: linear-gradient(to bottom, var(--acc), transparent);
  }
  h1 {
    font-size: var(--fs-xl);
    font-weight: var(--w-bold);
    letter-spacing: -0.03em;
    line-height: 1.05;
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
  /* AN EMPTY LEDGER IS THE FIRST THING A NEW OPERATOR SEES, and it was a
     grey box of prose in the same weight as everything else -- the page's
     largest block, saying the least. It gets a ground of its own and a
     heading that reads as a state rather than as another paragraph. The
     `info` rail stays: this is a fact, not a fault, and the colour is what
     says which. */
  .panel.bt-note {
    border-left: 3px solid var(--info);
    background: linear-gradient(135deg, var(--acc-soft) 0%, transparent 46%), var(--n3);
  }
  .panel.bt-note h2 {
    font-size: var(--fs-md);
    letter-spacing: -0.01em;
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

  /* The eight-rung table `cli` prints, and the refusal it prints instead.
     Eight because it is one row per rung SWEPT; the store's ninth timeframe
     is never among them. It also carries the descent's report now, which is
     one rung and a threshold ladder rather than eight rungs — same fixed
     columns, same reason it must not wrap.
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
  /* THE BEST-SO-FAR PANEL. Tokens only -- no literal colour -- so it follows
     the light/dark switch the rest of the page already answers to. */
  .livetop {
    margin: 10px 0 14px;
    border: 1px solid var(--n4);
    border-radius: 8px;
    background: var(--n1);
    overflow: hidden;
  }
  .livetop-head {
    margin: 0;
    padding: 9px 12px;
    border-bottom: 1px solid var(--n4);
    font-size: 13px;
  }
  /* THE TABLE SCROLLS INSIDE ITS OWN BOX. Nine columns on a narrow window must
     not make the PAGE scroll sideways. */
  .livetop-scroll {
    overflow-x: auto;
  }
  .livetop-table {
    width: 100%;
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
    font-size: 12px;
  }
  .livetop-table th,
  .livetop-table td {
    padding: 5px 12px;
    text-align: right;
    white-space: nowrap;
  }
  .livetop-table th:nth-child(2),
  .livetop-table td:nth-child(2),
  .livetop-table th:last-child,
  .livetop-table td:last-child {
    text-align: left;
  }
  .livetop-table thead th {
    border-bottom: 1px solid var(--n4);
    color: var(--n9);
    font-weight: 600;
  }
  .livetop-table tbody tr:nth-child(even) {
    background: var(--n2);
  }
  /* A ROW THAT CLEARS THE BAR IS THE ONLY ONE THAT COUNTS, so it is the only
     one marked. Marking every row by rank would say the opposite. */
  /* `--up` / `--up-soft`, NOT `--good`. This block first wrote `var(--good-soft)`
     and `var(--good)`, and NEITHER TOKEN EXISTS in `src/lib/theme.css` -- an
     undefined custom property resolves to nothing, so the one row that matters
     would have rendered with a transparent background and inherited ink. The
     build does not catch that; only reading the token table does. */
  .livetop-table tbody tr.clears {
    background: var(--up-soft);
    color: var(--up);
    font-weight: 600;
  }
  .livetop-foot {
    margin: 0;
    padding: 8px 12px;
    border-top: 1px solid var(--n4);
    font-size: 11px;
    line-height: 1.5;
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
  /* The rung strip. Nine chips read as one measurement when they share a
     baseline and a width; as nine boxes they read as nine things. */
  /* THE INSTRUMENT CHIP RULES ARE GONE WITH THEIR MARKUP. A row of chips put
     the choice on the page rather than in a control, which is not the shape
     the rest of this console uses and grows wider with every instrument the
     store gains. `$lib/Picker.svelte` is the console's own multi-select
     dropdown and it does this job everywhere else. `.rungs li` survives --
     the coverage strip still uses it. */
  /* A CHIP IS A TOGGLE NOW, so it must look pressable and read its own state.
     Unselected recedes rather than disappearing -- the rung is still ON DISK
     and its coverage is still a fact, it is simply not in this run. */
  /* The gauge. Months this rung holds against months the instrument holds --
     the same question the ledger's coverage column answers per run, asked
     here per rung before the run exists. */
  /* A rung holding fewer months than its instrument is a SHORTER SAMPLE.
     It takes the warn rail rather than a flash, for the same reason
     `complete: NO` does: the row is not comparable, and that stays true
     for as long as it is on screen. */
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
  /* THE `.pbtn` OVERRIDE THAT WAS HERE IS DELETED, AND IT WAS THE WHOLE
     PROBLEM. It read:

       font-size: var(--fs-mini); padding: 5px 28px 5px 8px; border-radius: 6px

     written to make `Picker` match a small `select.find.sm` in the same
     row -- a select that has since been replaced by another `Picker`, so
     the override outlived the only thing it was matching. It shrank a
     48px/16px control to 31px/12.5px, which is why this page read as a
     cramped utility strip beside `/db`'s query panel built from the same
     component.

     Sizing DOWN to the smallest thing in a row is the wrong direction:
     the row should have been built at the component's own size. Nothing
     replaces this rule -- native `Picker` is the console's size. */
  /* The span menus stand in for two text inputs, so the placeholder that
     replaces them while the census loads must hold the same line. */
  .runf-wait {
    font-size: var(--fs-micro);
    color: var(--n8);
    padding: 0.3rem 0;
  }
  /* ==================================================================
     MOTION — WHERE A FACT MOVED, AND NOWHERE ELSE
     ------------------------------------------------------------------
     The page's standing rule is that motion carries meaning. These three
     are the moments something actually changed:

       · the drill-down panel arriving — a run was opened
       · a tester tab's body swapping — the content under it changed
       · the folded metrics unfolding — seventeen rows appeared

     Deliberately NOT animated, and each for a stated reason: numbers do
     not count up (a figure in motion reads as data still arriving, and
     every figure here is final the moment it is drawn); nothing has a
     hover that moves the layout; and no section re-plays its entrance on
     a re-render, because a page that moves when nothing happened teaches
     the reader to ignore movement.
     ================================================================== */
  /* NO `.drill` ANIMATION HERE — one already exists further down as
     `drillopen`, and a second would have been a duplicate that lost the
     cascade anyway. Checked before adding rather than after. */
  /* The tab bodies are keyed, so this replays on every switch — which is
     exactly when what is under them became different. Shorter than the
     panel's own arrival: a swap inside a panel that is already open is a
     smaller event than the panel opening. */
  .tester .tt-sec {
    animation: secin 0.22s ease-out backwards;
  }
  @keyframes secin {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  .tt-more td {
    padding-top: 0.7rem;
  }
  .tt-more button {
    font-size: var(--fs-mini);
  }
  @media (prefers-reduced-motion: reduce) {
    .drill,
    .tester .tt-sec {
      animation: none;
    }
  }

  /* THE COVERAGE SECTION IS GONE, AND SO IS EVERYTHING THAT DRESSED IT.
     `.coverbar*`, `.rungs-head`, `.rungs li`, every `.rungchip*` rule, the
     `rungin` keyframe and this reduced-motion guard over them: all removed
     with the markup they styled.

     `.rungs` SURVIVES and is not this one -- the drill-down's rung switcher
     uses the same class name with its own complete rule further down, so
     only the `<ul>` variant went.

     One of these was the trap this repository has already been bitten by:
     the compiler named `.rungchip-w` as unused, and that selector was the
     SECOND half of `.rungchip-m, .rungchip-w { ... }`. Deleting by the line
     it reported would have left `.rungchip-m,` dangling and taken the next
     rule with it. Every removal here was checked for a preceding line
     ending in a comma first. */

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
  /* FOUR CELLS OF IDENTICAL GREY TEXT read as a paragraph broken into
     columns rather than as four separate readings. Each takes a hairline of
     its own meaning at the top -- neutral for a count, green for the
     comparable ones, amber where a defect counter is non-zero -- so the
     strip can be scanned rather than read. The rail is 2px and sits above
     the label, which is the least ink that can carry a state. */
  .fact::before {
    content: '';
    position: absolute;
    inset: 0 0 auto 0;
    height: 2px;
    background: var(--n6);
    transition: background 0.2s ease;
  }
  .fact.good::before {
    background: var(--up);
  }
  .fact.warn::before {
    background: var(--warn);
  }
  .fact.nil::before {
    background: var(--n6);
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
  /* THE ONE CONTROL ON THIS PAGE THAT STARTS WORK, and it was the same size
     as the two menus beside it. It keeps their height -- the row has one
     baseline and that is worth more than a tall button -- and takes weight
     instead: a wider block, tighter tracking, and a glow that says pressable
     without moving anything. */
  .btn.run {
    padding-inline: 1.1rem;
    letter-spacing: -0.01em;
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
  /* The report-mode control in the supplied Strategy Tester references is a
     dark toolbar selection, not the white analysis-tab pill used below it. */
  .tt-views .tt-view.on {
    background: var(--n5);
    color: var(--n12);
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
  .tt-ctl,
  .tt-fact {
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
    white-space: nowrap;
  }
  .tt-ctl {
    cursor: pointer;
    transition: background 0.14s ease;
  }
  .tt-fact {
    cursor: default;
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
  .tt-menuitem:disabled {
    opacity: 0.52;
    cursor: not-allowed;
  }
  .tt-menuitem:disabled:hover {
    background: transparent;
  }
  .tt-menuitem.sel {
    border-color: var(--acc);
    color: var(--n12);
  }
  .tt-default {
    font-size: var(--fs-micro);
    color: var(--n8);
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
  .tt-segbtn:not(:disabled):hover {
    background: var(--n4);
    color: var(--n11);
  }
  .tt-segbtn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
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

  /* Animate the whole donut without touching each arc's data-driven
     dasharray/dashoffset. Animating those properties replaced every real
     winner/loser/breakeven proportion with one fixed 276/400 ring. */
  .tester .tt-donut {
    animation: donut-in 0.5s cubic-bezier(0.33, 0.8, 0.35, 1) both;
  }
  @keyframes donut-in {
    from {
      opacity: 0.25;
      transform: scale(0.94);
    }
    to {
      opacity: 1;
      transform: none;
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
    .tester .tt-donut {
      animation: none !important;
      opacity: 1 !important;
      transform: none !important;
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
  /* `backwards`, NOT `both` — the same fill-mode trap already fixed once on
     this page. `both` keeps the last keyframe applied after the animation
     ends, and `transform: none` there RESOLVES to `matrix(1,0,0,1,0,0)`.
     An identity matrix is still a transform: it leaves `.drill` a
     containing block and a stacking context permanently, which is exactly
     what trapped `Picker`'s menu inside `.runbar`. Nothing needs to escape
     this panel today; it is corrected because the next thing added inside
     it should not have to discover this. */
  .drill {
    animation: drillopen 0.42s cubic-bezier(0.22, 0.75, 0.3, 1) backwards;
    transform-origin: top center;
    /* The supplied reference is one dark, docked analysis surface even when
       the browser chrome around it is light. Scope the ramp here so the rest
       of the console keeps the operator's chosen theme. */
    --n0: #090b0d;
    --n2: #16191c;
    --n3: #0f1113;
    --n4: #23272b;
    --n5: #2b3035;
    --n6: #363c42;
    --n7: #5e666d;
    --n8: #868f96;
    --n9: #a7afb5;
    --n10: #c0c6ca;
    --n11: #d1d5d8;
    --n12: #f2f4f5;
    --panel: #16191c;
    --ink: #f2f4f5;
    --muted: #868f96;
    --faint: #5e666d;
    --line: #363c42;
    --line-soft: #2b3035;
    --acc: #2962ff;
    --acc-soft: color-mix(in srgb, #2962ff 14%, transparent);
    --focus: #2962ff;
    --on-acc: #ffffff;
    --up: #089981;
    --up-soft: color-mix(in srgb, #089981 15%, transparent);
    --down: #f23645;
    --down-soft: color-mix(in srgb, #f23645 14%, transparent);
    --warn: #ffb23e;
    --warn-soft: color-mix(in srgb, #ffb23e 14%, transparent);
    background: #0f1113;
    border-color: #363c42;
    color: #d1d5d8;
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
    height: 300px;
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
    width: 172px;
    height: 172px;
    flex: none;
  }
  .tt-donutmid {
    position: absolute;
    left: 86px;
    top: 86px;
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
  /* CLICK TARGETS, MEASURED. `.tt-info` was **13x13** and `.tt-eye`
     **14x14** — both under the 24x24 floor a pointer target needs, and
     these are the two controls that reveal WHY a row is locked and WHICH
     plot is drawn. A control that explains the page should not be the
     hardest thing on it to hit.

     The icon keeps its size; the TARGET grows around it with padding and a
     negative margin, so nothing in the layout moves. `-webkit-tap-highlight`
     is untouched: this is about the hit box, not about how it flashes. */
  .tt-eye {
    padding: 6px;
    margin: -6px;
    border-radius: 4px;
  }
  .tt-eye {
    background: none;
    border: 0;
    line-height: 0;
    color: var(--acc);
    cursor: pointer;
  }
  .tt-eye svg {
    width: 14px;
    height: 14px;
  }
  .tt-eye:disabled {
    color: var(--n7);
    cursor: not-allowed;
    opacity: 0.55;
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

  /* ---- list of trades: the two-row block ----
     TWO DIVIDER WEIGHTS, which is what makes this read as trades rather than as
     rows. The reference draws a FAINT hairline between a trade's Exit and Entry
     legs that spans only the per-leg band — Type through Price — and stops dead
     before the spanning columns and before the trade number. Between trades it
     draws a full-bleed line at normal weight. This had only the second, so the
     two legs of one trade floated with nothing tying them together. */
  .tt-tbl tr.lot-a td:not([rowspan]) {
    border-bottom: 1px solid var(--line-soft);
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
  /* HOVER LIFTS THE WHOLE TRADE, NOT ONE LEG. Highlighting `lot-a` alone split
     a trade in half under the cursor, which is exactly the reading the two-row
     block exists to prevent. `:has` ties the pair; the plain rules under it are
     the fallback where `:has` is unsupported. */
  .tt-tbl tr.lot-a:hover,
  .tt-tbl tr.lot-b:hover {
    background: var(--n4);
  }
  .tt-tbl tr.lot-a:hover + tr.lot-b,
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
    border-radius: 0;
    box-shadow: none;
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

  /* ==================================================================
     SCALE — THE PAGE WAS BUILT SMALL AND STAYED SMALL
     ------------------------------------------------------------------
     Measured before this block, at 1440px: the three controls in the
     sweep bar were **31–32px tall carrying 12.5px text**, the rung gauges
     were **3px**, every section padded to **13.6px**, and the four
     summary numbers sat at 24px. Nothing was WRONG -- every one of those
     had passed a contrast and a floor check -- and the whole thing still
     read as a dense utility strip rather than as the console's main
     surface.

     Density was the goal and it was taken too far. The earlier note in
     this file argued that "every line of chrome above the data is a line
     of data that did not fit"; that is true when there IS data below,
     and this page's own data lives in a panel further down. The bar an
     operator ACTS in was paying for space the page had already saved.

     This block sits before the responsive overrides deliberately, so a
     narrow viewport still wins -- the scale goes up on a desktop, not on
     a phone.
     ================================================================== */

  /* ---- the sweep bar: MEASURED OFF `/db`, not invented here ----------
     This bar was a cramped flex ROW while `/db`'s query panel -- the same
     job, the same component, the same console -- is a GRID of labelled
     fields. Measured on `/db` at 1440px:

       .strip   grid · 3 × 228px · gap 16px 12px · padding 16px
       .field   flex column · gap 4px
       .lab     12px / 600 / 1.2px tracking / uppercase / --n8
       .pbtn    48px tall · 16px / 600 · radius 9px

     And this page had a `:global(.pbtn)` override shrinking that same
     button to 38px and 13.5px, written to make it match a small `select`
     that is no longer even in the markup. The override is gone: `Picker`
     at its native size IS the console's size, and every other page uses
     it that way. Fighting a design system is what made this page look
     unlike the product it is part of.

     `auto-fit` rather than a fixed three, because this bar has five cells
     and `/db` has seven -- the column COUNT is a consequence of the
     width, and only the track floor is a decision.

     ------------------------------------------------------------------
     FOUR NAMED TRACKS AND `subgrid`, BECAUSE `align-items: end` IS NOT AN
     ALIGNMENT. This bar was a grid of columns whose rows were whatever
     each cell happened to be, bottom-aligned. That holds a row together
     only while every cell is the same height, and this bar's cells are
     NOT: a field renders `Picker` at 48px once the census answers, and
     renders a `reading census...` line at 28px until it does -- and the
     page's own logic guarantees both states are on screen at once,
     because `heldNow` is null until an instrument is picked, so
     `instruments` has its menu while `from` and `to` are still waiting.

     MEASURED on this build at 1600px, giving field one a 48px control and
     leaving the other three waiting:

       label tops before   177 · 177 · 177 · 177
       label tops after    177 · 197 · 197 · 197      <- 20px apart
       control track       49.59px -> 70px

     Four labels on two lines, in the ordinary intermediate state of the
     page. `web/design/backtest.html` states the fix and both older
     references state the rule: "a grid with named rows, not a flex row of
     variable-height columns".

     THE NAME ROW IS PINNED AND NOTHING ELSE IS, and both halves of that
     were paid for while writing `web/design/backtest.html`:

     1. Auto-place everything, and the strip silently re-indexes -- the
        name takes the label track, every label lands in the CONTROL
        track, and the row still looks internally consistent because
        subgrid keeps the fields agreeing with each other while all of
        them are wrong together.

     2. Pin every field to `grid-row: 2 / span 2`, and wrapping breaks. A
        pinned field cannot move to a new row, so when only three columns
        fit the auto-placement algorithm invents implicit columns for the
        rest. MEASURED on the reference at a 752px bar:
        `grid-template-columns` computed to `210px 210px 210px 0px 0px`
        and the Run button landed 81px outside the page -- horizontal body
        scroll, which moves every column on every section below it.

     So the NAME row is pinned, because it is the one that steals a track.
     The fields span two rows and auto-place, so a field that does not fit
     wraps to a FRESH label/control pair with its own baseline -- which is
     correct, and is what `/db` does by stacking two `.strip`s rather than
     ragging one. `.runbar-n` is last in the DOM and spans every column,
     so it settles below whatever the fields used. */
  .runbar {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    /* name · label · control — further label/control pairs are implicit */
    grid-template-rows: auto 16px 48px;
    grid-auto-rows: auto;
    align-items: start;
    gap: 4px 12px;
    padding: 16px;
  }
  /* The two full-width rows: the panel's own name above the fields, and
     what the press will do below them. */
  /* THERE IS DELIBERATELY NO `scroll-behavior: smooth` ON `.page`.

     It was added here to give the drill-down's arrival scroll its
     animation back after `scrollTo({behavior:'smooth'})` turned out to be
     a no-op on this container -- and it reintroduced the identical bug by
     the other door, because CSS `scroll-behavior` makes even a direct
     `scrollTop` ASSIGNMENT animate. Measured: `scrollTop` stayed 0 with
     this rule present and moves to the panel without it.

     Smooth scrolling on this scroller does not work in this build. An
     instant arrival is not as pretty and it is the one that always
     happens, which is the property that matters for navigation. */

  .runbar-k,
  .runbar-n {
    grid-column: 1 / -1;
  }
  .runbar-k {
    grid-row: 1;
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--n8);
    padding: 0;
    margin-bottom: 8px;
  }
  /* NOT PINNED TO A ROW. It is last in the DOM and spans every column, so
     it auto-places below whatever the fields used -- one row-pair on a
     desktop, two when the bar is narrow enough to wrap. */
  .runbar-n {
    font-size: var(--fs-xs);
    line-height: 1.55;
    padding: 0;
    margin-top: 8px;
  }
  /* THE FIELD IS A SUBGRID OF THE LABEL AND CONTROL TRACKS, so the two
     tracks are shared across the whole bar rather than recomputed per
     cell. A field that is waiting and a field that has its menu now sit
     on the same two lines by construction, not by coincidence of height. */
  .runf {
    display: grid;
    grid-template-rows: subgrid;
    grid-row: span 2;
    min-width: 0;
  }
  .runf > span:first-child {
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--n8);
    line-height: 16px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* A CONTROL THAT IS NOT THERE YET STILL OCCUPIES THE CONTROL TRACK.
     This was a bare 28px line of text, which is what let the row collapse
     around it. It is a dashed box at the control's own height now: the
     place is held, and the absence is legible instead of being a gap that
     reads as a narrower control. */
  .runf-wait {
    display: flex;
    align-items: center;
    height: 48px;
    padding: 0 14px;
    border: 1px dashed var(--n6);
    border-radius: 9px;
    background: var(--n3);
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--n8);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* THE RUN BUTTON SPANS A PAIR AND SITS ON THE CONTROL TRACK. It has no
     label of its own, so a bare one-row item would auto-place into the
     first free cell -- which is a LABEL track, not a control one -- and
     pinning it to row 3 is worse: with the fields already filling that
     row it forces an implicit column and the 81px overflow above. It
     takes a label+control pair like every other cell and aligns to the
     bottom of it, so it lands on the control baseline by construction and
     travels with the fields when they wrap. */
  .runbar .btn.run {
    grid-row: span 2;
    align-self: end;
    height: 48px;
    padding-inline: 1.4rem;
    font-size: var(--fs-sm);
    font-weight: var(--w-semi);
    border-radius: 9px;
  }

  /* ---- the descend bar's two typed fields ----------------------------
     A NUMBER IS TYPED, NOT PICKED, AND THAT IS THE WHOLE DIFFERENCE. Every
     other control in this bar chooses from a set the store already states —
     feeds, instruments, rungs, months — so a `Picker` is right for them and
     wrong for these two: a stop ceiling is a quantity the operator decides,
     with no list to offer.

     IT MATCHES THE MENU RATHER THAN OVERRIDING IT. `$lib/Picker.svelte`'s own
     `.pbtn` is 9px radius, `--line` hairline, `--panel` ground, mono at
     `--fs-base` — and it is a SHARED component, so none of that is reached
     into from here. These are separate rules that land on the same geometry,
     which is the only way to sit on one baseline without a local override
     changing the menu on five other pages. The height is 48px because the
     grid above pins the control track to 48px.

     THE DIGITS ARE MONO AND TABULAR, the way `/db`'s page-jump box is: a
     ceiling of 120 and one of 1,200 must be tellable apart at a glance, and a
     proportional font makes that a reading rather than a look.

     NOT `type="number"`. Its spinner adds a control nobody asked for, its
     silent coercion accepts `1e3` and `12.5`, and `valueAsNumber` on an
     unparseable value is `NaN` — three ways for a figure the operator did not
     type to reach the run's identity. `wholeNumber` is the one reading. ---- */
  .dnum {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 48px;
    padding: 0 14px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 9px;
    min-width: 0;
  }
  .dnum:focus-within {
    outline: 2px solid var(--acc);
    outline-offset: 2px;
  }
  /* TYPED AND NOT YET A NUMBER IS ITS OWN STATE. An empty box is waiting; a
     box holding `12px` is wrong, and the two must not look alike. The rail
     says so at the control, the sentence under the button says which field. */
  .dnum.wrong {
    border-color: var(--warn);
  }
  .dnum-in {
    flex: 1 1 auto;
    min-width: 0;
    width: 100%;
    background: none;
    border: 0;
    padding: 0;
    color: var(--ink);
    font: inherit;
    font-family: var(--mono);
    font-size: var(--fs-base);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .dnum-in:focus,
  .dnum-in:focus-visible {
    outline: none;
  }
  .dnum-in::placeholder {
    color: var(--faint);
    font-family: inherit;
    font-size: var(--fs-micro);
    font-weight: 400;
    font-variant-numeric: normal;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .dnum-u {
    flex: 0 0 auto;
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--n8);
  }

  /* WHICH BAR HOLDS THE SLOT. Two controls append to one append-only ledger
     and the server keeps one `Progress` for both, so "a run is going" is not
     enough — the operator has to be able to see WHICH command is going, at the
     control that started it. No keyframe: nothing is moving, a fact is simply
     stated where it is read. */
  .rk-live {
    color: var(--acc);
    font-weight: var(--w-semi);
    letter-spacing: 0.06em;
  }

  /* ---- the coverage bar ---- */
  /* A 3px gauge is a hairline, not a measurement. Six reads as a bar and
     leaves the chip's numbers room above it. */

  /* ---- the summary strip ---- */
  .bt-strip .fact {
    padding: 1.15rem 1.35rem;
    gap: 0.3rem;
  }
  .fact .k {
    font-size: var(--fs-mini);
  }
  /* THE COUNTERS GO TO THE DISPLAY STEP. They dropped to `--fs-data-lg`
     when the answer figure was raised above them, which fixed the
     inversion by shrinking the wrong half -- the answer is 34px and lives
     in its own accented card, so it does not need these kept small to
     stay ahead of them. */
  .fact .v {
    font-size: var(--fs-data-xl);
    line-height: 1.05;
  }
  .fact .n {
    font-size: var(--fs-mini);
  }

  /* ---- section headings and the page title ---- */
  .bh {
    font-size: var(--fs-lg);
  }
  .block {
    gap: 0.9rem;
  }

  @media (max-width: 860px) {
    .tester .tt-quad,
    .tester .tt-stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (max-width: 520px) {
    .tester .tt-quad,
    .tester .tt-stats {
      grid-template-columns: 1fr;
    }
    .tester .tt-bar {
      align-items: stretch;
      gap: 0.55rem;
    }
    .tester .tt-name,
    .tester .tt-meta {
      width: 100%;
    }
    .tester .tt-meta {
      margin-left: 0;
    }
    .tester .tt-pills {
      flex-wrap: nowrap;
      overflow-x: auto;
      padding-bottom: 0.2rem;
      scrollbar-width: thin;
    }
    .tester .tt-pill {
      flex: 0 0 auto;
    }
    .tt-menu {
      right: 0;
      left: auto;
      width: min(230px, calc(100vw - 2rem));
      max-width: calc(100vw - 2rem);
    }
    .term .bt-chart {
      height: clamp(260px, 50svh, 420px);
      flex-basis: clamp(260px, 50svh, 420px);
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
  /* THE UNIT SUFFIX WAS 10px, which is the size TradingView sets it at
     and two below the floor `theme.css` raised twice. `POINTS` is a WORD
     a reader parses, not a glyph, and it sits directly beside the largest
     number in the panel -- the one place a unit must not be guessed at.
     The tester keeps TradingView's scale everywhere it is legible; this
     is the one step of it that was not. */
  .tester .tt-unit {
    font-size: var(--fs-micro);
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
    background: #f2f2f2;
    border-color: #f2f2f2;
    color: #111315;
    font-weight: 600;
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
  .tester .tt-fact,
  .tester .tt-view {
    font-size: 13px;
    color: var(--tv-label);
  }
  .tester .tt-view.on {
    background: var(--tv-seg-on);
    color: var(--tv-text);
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

  /* TradingView's compact geometry is already matched above. On screens with
     room for the full report, lift the reading scale by one pixel without
     changing pill heights, table padding, section spacing, or plot geometry.
     The phone layout keeps the reference's denser type scale below 521px. */
  @media (min-width: 521px) {
    .tester {
      font-size: 14px;
      line-height: 1.45;
    }
    .tester .tt-h {
      font-size: 16px;
    }
    .tester .tt-h5 {
      font-size: 15px;
    }
    .tester .tt-k {
      font-size: 14px;
    }
    .tester .tt-qv {
      font-size: 15px;
    }
    .tester .tt-note,
    .tester .tt-note2,
    .tester .tt-tbl td .tp {
      font-size: 13px;
    }
    .tester .tt-pill,
    .tester .tt-segbtn,
    .tester .tt-tbl,
    .tester .tt-tbl th,
    .tester .tt-ctl,
    .tester .tt-fact,
    .tester .tt-view,
    .tester .tt-plotrow,
    .tester .tt-blab,
    .tester .tt-pllab,
    .tester .tt-cmplab,
    .tester .tt-bval,
    .tester .tt-plval,
    .tester .tt-cmpval {
      font-size: 14px;
    }
    .tester .tt-name b {
      font-size: 15px;
    }
    .tester .tt-chip,
    .tester .tt-dim,
    .tester .tt-menuhead,
    .tester .cf-axis,
    .tester .cf-x,
    .tester .cf-legend,
    .tester .cf-note,
    .tester .tt-donutmid span {
      font-size: 13px;
    }
    .tester .tt-menuitem,
    .tester .tt-donutleg {
      font-size: 14px;
    }
  }

  /* These labels were 10-11px in the inherited console scale. Keep their
     compact roles, but give every report annotation a legible 12px floor. */
  .tester .rbt-x,
  .tester .streak-col small,
  .tester .lot-pc,
  .tester .tt-pc2,
  .tester .tt-subject-k {
    font-size: 12px;
  }
  .tester .rbt-leg,
  .tester .series-nav,
  .tester .tt-plotnote,
  .tester .tt-warnnote {
    font-size: 13px;
  }
  .tester .tt-subject-v {
    font-size: 14px;
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
    grid-template-columns: minmax(150px, 180px) 1fr;
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
  /* THE SELECTED PILL IS FILLED, NOT OUTLINED. The reference fills it with a
     near-white and inverts the text; this drew a transparent pill with an
     accent border, which reads as "focused" rather than as "selected" and is
     the one control state a reader scans a tab strip for. */
  .tt-pill.on {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--bg-2);
    font-weight: var(--w-semi);
  }
  .tt-pill:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  /* ---- the customer comparison ------------------------------------

     A MARKET BLOTTER, NOT ANOTHER CARD DECK. The reference is a persistent
     accent rail down one row, while every other distinction is written as a
     word: Comparable, Stopped early, Partial history. The rail is the one
     aesthetic gesture this layer spends; the table around it stays quiet so
     seven different facts can still be read left to right.

     This owns its horizontal scroll. `.page` must never grow wider than the
     shell, so a narrow screen moves this one comparison rather than the whole
     product. */
  .compare-heading {
    justify-content: space-between;
    align-items: flex-start;
  }
  .compare-heading > div {
    flex: 1 1 460px;
  }
  .compare-heading .bsub {
    margin-top: 0.3rem;
  }
  .compare-heading .bsub b {
    color: var(--n11);
  }
  .compare-callout {
    display: flex;
    align-items: flex-start;
    gap: 0.65rem;
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--up);
    border-left-width: 3px;
    border-radius: 8px;
    background: var(--up-soft);
    color: var(--n10);
    font-size: var(--fs-mini);
  }
  .compare-callout.negative {
    border-color: var(--down);
    background: var(--down-soft);
  }
  .compare-callout-mark {
    display: grid;
    place-items: center;
    width: 1.35rem;
    height: 1.35rem;
    flex: 0 0 1.35rem;
    border-radius: 999px;
    background: var(--up);
    color: var(--n2);
    font-family: var(--num);
    font-weight: var(--w-bold);
  }
  .compare-callout.negative .compare-callout-mark {
    background: var(--down);
  }
  .compare-callout > div {
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
  }
  .compare-callout b {
    color: var(--n12);
  }
  .compare-wrap {
    overflow-x: auto;
    border: 1px solid var(--n6);
    border-radius: 8px;
    background: var(--n3);
    scrollbar-gutter: stable;
  }
  .compare-table {
    width: 100%;
    min-width: 1040px;
    border-collapse: collapse;
    font-size: var(--fs-mini);
  }
  .compare-table th,
  .compare-table td {
    padding: 0.72rem 0.75rem;
    border-bottom: 1px solid var(--n5);
    text-align: left;
    vertical-align: top;
  }
  .compare-table thead th {
    background: var(--n2);
    color: var(--n8);
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .compare-table tbody tr:last-child > * {
    border-bottom: 0;
  }
  .compare-table tbody tr.reference > * {
    background: var(--acc-soft);
  }
  .compare-table tbody tr.reference > :first-child {
    box-shadow: inset 3px 0 0 var(--acc);
  }
  .compare-table tbody tr.excluded > * {
    color: var(--n9);
    background: var(--n2);
  }
  .compare-table .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .compare-rank {
    width: 92px;
    min-width: 92px;
  }
  .compare-place {
    display: block;
    color: var(--n12);
    font-family: var(--num);
    font-size: var(--fs-data);
    line-height: 1;
  }
  .compare-state {
    display: inline-flex;
    margin-top: 0.4rem;
    padding: 0.2rem 0.42rem;
    border-radius: 999px;
    color: var(--n10);
    background: var(--n4);
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    white-space: nowrap;
  }
  .compare-state.good {
    color: var(--up);
    background: var(--up-soft);
  }
  .compare-state.warn {
    color: var(--warn);
    background: var(--warn-soft);
  }
  .compare-state.bad {
    color: var(--down);
    background: var(--down-soft);
  }
  .compare-rank small,
  .compare-money small,
  .compare-risk small {
    display: block;
    margin-top: 0.28rem;
    color: var(--n8);
    font-size: var(--fs-micro);
    font-weight: var(--w-reg);
    line-height: 1.35;
  }
  .compare-run {
    min-width: 205px;
  }
  .compare-id {
    display: flex;
    align-items: baseline;
    gap: 0.38rem;
    flex-wrap: wrap;
    margin-bottom: 0.22rem;
  }
  .compare-id b {
    color: var(--n12);
    font-size: var(--fs-base);
  }
  .compare-setting {
    display: block;
    margin-top: 0.3rem;
    color: var(--n8);
    line-height: 1.45;
  }
  .compare-setting b {
    color: var(--n10);
    font-family: var(--num);
  }
  .compare-money,
  .compare-risk {
    min-width: 122px;
    font-family: var(--num);
  }
  .compare-money > b,
  .compare-risk > b {
    color: var(--n11);
    font-size: var(--fs-data);
    white-space: nowrap;
  }
  .compare-money.up > b {
    color: var(--up);
  }
  .compare-money.down > b {
    color: var(--down);
  }
  .compare-risk span {
    display: block;
    margin-top: 0.35rem;
    color: var(--down);
    font-size: var(--fs-micro);
    white-space: nowrap;
  }
  .compare-pattern {
    min-width: 210px;
    max-width: 330px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .compare-action {
    width: 84px;
    text-align: right;
    white-space: nowrap;
  }
  .compare-foot {
    margin: 0;
    max-width: 96ch;
    color: var(--n8);
    font-size: var(--fs-micro);
    line-height: 1.45;
  }
  .compare-more {
    width: fit-content;
  }

  @media (max-width: 1100px) {
    .compare-wrap {
      overflow: visible;
      border: 0;
      background: transparent;
    }
    .compare-table {
      display: block;
      min-width: 0;
    }
    .compare-table thead {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }
    .compare-table tbody {
      display: grid;
      gap: 0.65rem;
    }
    .compare-table tbody tr {
      display: grid;
      grid-template-columns: minmax(0, 1fr);
      overflow: hidden;
      border: 1px solid var(--n6);
      border-radius: 8px;
      background: var(--n3);
    }
    .compare-table th,
    .compare-table td {
      width: auto;
      min-width: 0;
      max-width: none;
      padding: 0.68rem 0.72rem;
      border-bottom: 1px solid var(--n5);
    }
    .compare-table tbody tr > :nth-child(2) {
      border-left: 0;
    }
    .compare-table tbody tr > * {
      grid-column: 1 / -1;
    }
    .compare-table tbody tr > :last-child {
      border-bottom: 0;
    }
    .compare-table td[data-label]::before {
      display: block;
      margin-bottom: 0.3rem;
      color: var(--n8);
      font-size: var(--fs-micro);
      font-weight: var(--w-semi);
      letter-spacing: 0.04em;
      text-transform: uppercase;
      content: attr(data-label);
    }
    .compare-table .num {
      text-align: left;
    }
    .compare-rank {
      display: flex;
      align-items: center;
      gap: 0.45rem;
    }
    .compare-state,
    .compare-rank small {
      margin-top: 0;
    }
    .compare-action {
      text-align: left;
    }
    .compare-action .btn {
      width: 100%;
    }
    .compare-heading .count {
      width: 100%;
    }
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
  /* Label/value tables use native row headers without borrowing the visual
     treatment of a column-heading band. */
  .tt-tbl th.tt-rowhead {
    background: transparent;
    color: var(--n10);
    font: inherit;
    font-weight: 400;
    padding: 0.46rem 0.75rem;
    border-bottom: 1px solid var(--n5);
    white-space: normal;
  }
  .tester .tt-tbl th.tt-rowhead {
    color: var(--tv-text);
    padding: 0.95rem 1rem;
    border-bottom-color: var(--tv-line-soft);
  }
  .tt-tbl tbody tr:last-child :is(th, td) {
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
  .tt-tbl tr.warnrow :is(th, td) {
    color: var(--warn);
  }
  .tt-tbl tr.offrow :is(th, td) {
    color: var(--n8);
  }
  /* ══ THE PADLOCK'S RULES GO WITH THE PADLOCK ══
     `4b3220b` — "the trade table shows trades, not padlocks" — deleted
     `<tr class="lockrow">`, `<div class="tt-lockbig">` and `<p class="tt-fix">`
     from the markup and left six rules behind styling nothing. Gate W4 is a
     FLOOR AT ZERO on orphaned CSS, not a ratchet, so this was a red build
     waiting to be noticed rather than untidiness.

     Removed by exact text and verified by the compiler, not by line number: it
     reports one line per SELECTOR and a rule may list several, which is how an
     earlier line-based deletion took 375 lines and broke a build. */

  /* A single explanation replaces a field of repeated unavailable marks. */
  .tt-unavailable-panel {
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
  .tt-unavailable-panel b {
    color: var(--n11);
  }

  /* One truthful selected-side bar where TradingView draws All/Long/Short.
     It uses gross magnitudes, while the value at the right remains the exact
     signed net. A second side is never implied. */
  .tt-side {
    margin-top: 0.8rem;
  }
  .tt-sidehead,
  .tt-sidefacts {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .tt-sidehead b {
    min-width: 4rem;
    color: var(--n12);
  }
  .tt-sidehead span {
    color: var(--n8);
    font-size: 0.78rem;
  }
  .tt-sidehead strong {
    margin-left: auto;
    font-weight: 500;
  }
  .tt-sidebar {
    display: flex;
    height: 12px;
    margin: 0.65rem 0 0.55rem;
    overflow: hidden;
    border-radius: 3px;
    background: var(--n5);
  }
  .tt-sidebar .loss,
  .tt-sidefacts i.loss {
    background: var(--down);
  }
  .tt-sidebar .profit,
  .tt-sidefacts i.profit {
    background: var(--up);
  }
  .tt-sidefacts {
    flex-wrap: wrap;
    color: var(--n9);
    font-size: 0.78rem;
  }
  .tt-sidefacts span {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }
  .tt-sidefacts i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .tt-sidefacts b {
    color: var(--n11);
    font-weight: 500;
  }
  .tt-side p {
    margin: 0.6rem 0 0;
    max-width: 96ch;
    color: var(--n8);
    font-size: 0.78rem;
  }

  .run-scope,
  .trade-schema {
    margin: 0 0 0.8rem;
    max-width: 100ch;
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
    outline-offset: -2px;
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
  .rangebtn:not(:disabled):hover {
    background: var(--n4);
    color: var(--n11);
  }
  .rangebtn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
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

  /* THE LIVE RUNG TABLE. Inherits `tt-tbl` for grid and type; this adds only
     the one thing it needs -- room to breathe while a run is in flight, and
     tabular figures so bar counts and thresholds line up down the column as
     they change. */
  .rungprog {
    margin: 0.6rem 0 0.9rem;
  }
  .rungprog .num {
    font-variant-numeric: tabular-nums;
  }

  /* ---- the ranking weights ----------------------------------------------
     Ten controls, one per criterion the operator named. Laid out as a wrapping
     flex row rather than a grid so a narrow window reflows instead of scrolling
     sideways -- the page body must never scroll horizontally. */
  .wrow {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 0.9rem;
    align-items: center;
    margin: 0.35rem 0 0.9rem;
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--n6);
    border-radius: 8px;
    background: var(--n2);
  }
  .wlab {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.72rem;
    letter-spacing: 0.02em;
    color: var(--n9);
    white-space: nowrap;
  }
  .wrange {
    width: 5.5rem;
    accent-color: var(--acc);
  }
  .wnum {
    width: 3.6rem;
    padding: 0.15rem 0.3rem;
    font: inherit;
    font-variant-numeric: tabular-nums;
    color: var(--n11);
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: 5px;
  }
  .wval {
    min-width: 1.6rem;
    font-variant-numeric: tabular-nums;
    color: var(--n11);
  }

  /* ══ A CONTROL THAT CANNOT MOVE THE ORDER LOOKS DIFFERENT ══
     Dimmed rather than disabled: the weight is still settable, and the reason
     it does nothing is a property of TODAY'S DATA, not of the control. Disabling
     it would claim the page had removed a criterion. */
  .wlab.winert {
    opacity: 0.55;
  }
  .wdead {
    font-size: 0.6rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.05rem 0.25rem;
    border-radius: 3px;
    background: var(--n4);
    color: var(--n8);
    cursor: help;
  }

  /* ══ THE VERDICT PILL ══ */
  .vp {
    display: inline-block;
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 0.1rem 0.32rem;
    border-radius: 3px;
    white-space: nowrap;
    cursor: help;
  }
  .vp i {
    font-style: normal;
    opacity: 0.75;
  }
  .vp-pass {
    background: color-mix(in srgb, var(--up) 18%, transparent);
    color: var(--up);
  }
  .vp-fail {
    background: color-mix(in srgb, var(--down) 16%, transparent);
    color: var(--down);
  }
  .vp-unpriced,
  .vp-none {
    background: var(--n4);
    color: var(--n8);
  }

  /* ══ THE CONDITION NAMES ROW ══
     A second row per combination rather than a column, because a mask can carry
     a dozen conditions and a twelfth column would push every number off screen.
     The row is unpadded at the top so it reads as a continuation of the row
     above it rather than as a separate record. */
  tr.namerow > td {
    padding: 0 0.55rem 0.45rem;
    border-top: 0;
  }
  tr.cmbrow > td {
    padding-bottom: 0.2rem;
  }
  .cnames {
    display: block;
    font-size: 0.68rem;
    line-height: 1.5;
    color: var(--n9);
    white-space: normal;
  }
  .cname {
    color: var(--n10);
  }
  .cdot {
    color: var(--n7);
  }
  .cret {
    color: var(--n8);
  }
  .cret i {
    font-style: normal;
    font-size: 0.58rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    opacity: 0.8;
  }
  .cunk {
    color: var(--warn);
  }

  /* ══ TWO LINES PER MONEY CELL ══
     The reference pairs every currency figure with what share of the price it
     is, because a number in rupees does not say whether the move was large.
     `display:block` so the percentage takes its own line under the figure. */
  /* ══ THE UNIT SITS ON THE VALUE'S LINE ══
     `4 trades`, `35 bars`, `Monday, 40.26% winners` — the reference keeps the
     unit and the qualifier on ONE line beside the number, at the same size.
     `.tt-pc2` puts them on a dim second line, which is right for a genuinely
     separate fact ("… at best fills") and wrong for the word that names the
     number's unit. */
  .tt-unit2 {
    font-style: normal;
    font-size: inherit;
    color: var(--n9);
    margin-left: 0.3em;
  }

  .lot-pc,
  .tt-pc2 {
    display: block;
    font-style: normal;
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
    color: var(--n8);
    margin-top: 0.1rem;
  }
  .lot-sort {
    color: var(--n8);
    font-weight: 400;
  }

  /* ══ RESULTS BY TIME ══
     One stacked column per bucket, winners below and losers above, as the
     reference draws it. Columns are laid out by the grid rather than by a
     width calculation, so seven weekdays and five session hours both fill the
     plot without a per-grain constant. */
  /* The winning combination on the headline card. Its own row rather than a
     fifth figure, because it is a list of names and not a number. */
  .crown-combo {
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    flex-wrap: wrap;
    margin-top: var(--s5);
    padding-top: var(--s4);
    border-top: 1px solid var(--line-soft);
  }
  .crown-combo-k {
    flex: 0 0 auto;
    font-size: 0.62rem;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    font-weight: var(--w-semi);
    color: var(--n8);
  }
  .crown-combo-v {
    flex: 1 1 18rem;
    min-width: 0;
    font-size: 0.76rem;
    line-height: 1.5;
  }
  .crown-combo-v code {
    font-family: var(--mono);
    font-size: 0.94em;
  }

  /* ══ THE SUBJECT LINE ══
     Above the toolbar and full width, because it is what every control below
     operates on. Accent-tinted rather than neutral so it reads as the panel's
     subject and not as one more row of metadata. */
  .tt-subject {
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    flex-wrap: wrap;
    padding: var(--s4) var(--s5);
    background: var(--acc-soft);
    border-bottom: 1px solid var(--line);
  }
  .tt-subject-k {
    flex: 0 0 auto;
    font-size: 0.62rem;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    font-weight: var(--w-semi);
    color: var(--acc);
  }
  .tt-subject-v {
    flex: 1 1 20rem;
    min-width: 0;
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--n11);
  }
  .tt-subject-v code {
    font-family: var(--mono);
    font-size: 0.94em;
  }

  /* The two-walks disclosure. Loud enough not to be skimmed past, because a
     reader who misses it will compare two numbers that cannot be compared. */
  .tt-warnnote {
    margin: 0 0 var(--s5);
    padding: var(--s4) var(--s5);
    background: var(--warn-soft);
    border-left: 3px solid var(--warn);
    border-radius: var(--r1);
    font-size: 0.78rem;
    line-height: 1.55;
    color: var(--n10);
  }
  .tt-warnnote code {
    font-family: var(--mono);
    font-size: 0.94em;
  }

  .rbt {
    margin: var(--s5) 0 var(--s4);
  }
  .rbt-plot {
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: var(--s4);
    height: 17rem;
    padding: var(--s4) 4.5rem 0 var(--s3);
    border-bottom: 1px solid var(--line);
    background-image: repeating-linear-gradient(
      to bottom,
      transparent 0,
      transparent calc(25% - 1px),
      var(--line-soft) 25%
    );
    overflow-x: auto;
  }
  .rbt-axis,
  .streak-axis {
    position: absolute;
    z-index: 2;
    right: 0.5rem;
    top: var(--s3);
    bottom: 1.7rem;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    color: var(--n8);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }
  .rbt-col {
    flex: 1 1 0;
    min-width: 2.6rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 100%;
    justify-content: flex-end;
  }
  .rbt-stack {
    width: 100%;
    max-width: 2.2rem;
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }
  .rbt-loss,
  .rbt-win {
    display: block;
    width: 100%;
    min-height: 0;
  }
  .rbt-loss {
    background: var(--down);
    border-radius: var(--r1) var(--r1) 0 0;
  }
  .rbt-win {
    background: var(--up);
  }
  .rbt-x {
    margin-top: var(--s3);
    font-size: 0.64rem;
    color: var(--n8);
    white-space: nowrap;
  }
  .rbt-leg {
    display: flex;
    justify-content: center;
    gap: var(--s6);
    list-style: none;
    margin: var(--s4) 0 0;
    padding: 0;
    font-size: 0.7rem;
    color: var(--n9);
  }
  .rbt-leg li {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
  .rbt-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: var(--r-full);
  }
  .rbt-dot.win {
    background: var(--up);
  }
  .rbt-dot.loss {
    background: var(--down);
  }

  /* Eight durable calendar grains need to wrap rather than force the whole
     tester wider than a phone. */
  .tt-hrow.tight .tt-seg {
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .series-nav {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s4);
    margin: var(--s4) 0 0;
    color: var(--n9);
    font-size: 0.72rem;
  }
  .series-nav button {
    width: 1.8rem;
    height: 1.8rem;
    border: 1px solid var(--line);
    border-radius: var(--r1);
    background: var(--n2);
    color: var(--n11);
    cursor: pointer;
  }
  .series-nav button:disabled {
    opacity: 0.38;
    cursor: default;
  }
  .series-nav button:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .trade-pages {
    justify-content: flex-end;
    margin: var(--s4) 0 var(--s3);
  }

  .streak-chart {
    margin: var(--s5) 0 var(--s4);
  }
  .streak-plot {
    position: relative;
    display: flex;
    gap: 2px;
    min-height: 17rem;
    padding: var(--s3) 4.8rem var(--s3) var(--s3);
    background-image: repeating-linear-gradient(
      to bottom,
      transparent 0,
      transparent calc(25% - 1px),
      var(--line-soft) 25%
    );
    overflow-x: auto;
  }
  .streak-axis {
    bottom: 1.8rem;
  }
  .streak-col {
    flex: 1 1 0;
    min-width: 0.42rem;
    display: grid;
    grid-template-rows: 7rem 7rem auto;
    text-align: center;
  }
  .streak-half {
    display: flex;
    justify-content: center;
  }
  .streak-half.top {
    align-items: flex-end;
    border-bottom: 1px solid var(--line);
  }
  .streak-half.bottom {
    align-items: flex-start;
  }
  .streak-half span {
    display: block;
    width: min(100%, 1.15rem);
    min-height: 1px;
  }
  .streak-half.top span {
    background: var(--up);
    border-radius: var(--r1) var(--r1) 0 0;
  }
  .streak-half.bottom span {
    background: var(--down);
    border-radius: 0 0 var(--r1) var(--r1);
  }
  .streak-col small {
    margin-top: var(--s2);
    color: var(--n8);
    font-size: 0.62rem;
  }

  /* Twenty two-row blocks are dense enough for comparison without turning
     the page into an unbounded wall. The header stays visible while the exact
     wider ledger scrolls in either direction. */
  .tester .trade-ledger-wrap {
    max-height: min(70vh, 760px);
    overflow: auto;
    border-top: 1px solid var(--tv-line);
    border-bottom: 1px solid var(--tv-line);
  }
  .tester .trade-ledger-wrap:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .tester .trade-ledger {
    min-width: 1300px;
    white-space: nowrap;
  }
  .tester .trade-ledger thead {
    position: sticky;
    top: 0;
    z-index: 3;
    background: var(--tv-bg);
  }

  /* Final readability floor for dense selectors and machine-like annotations
     that inherited 9-11px console tokens. */
  .tester .tick,
  .tester .tt-caret,
  .tester .vp,
  .tester .cnames,
  .tester .cret i,
  .tester .tt-sidehead span,
  .tester .tt-sidefacts,
  .tester .tt-side p {
    font-size: 12px;
  }

  @media (min-width: 521px) {
    .tester .tt-quad .tt-qv {
      font-size: 15px;
      line-height: 1.25;
    }
    .tester .tt-quad .tt-qv.big {
      font-size: 21px;
    }
    .tester .tt-sec {
      padding: 1rem 1.1rem 1.15rem;
    }
  }

  @media (max-width: 520px) {
    .tester .tt-sec {
      padding: 1rem 0.8rem 1.1rem;
    }
  }

  /* ---- the engine's own knobs -------------------------------------------
     A disclosure rather than an always-open block: fourteen fields above the
     Run button would bury the button, and the common case is leaving every one
     of them to the server. */
  .eng {
    margin: 0.5rem 0 0.75rem;
    border: 1px solid var(--n6);
    border-radius: 8px;
    background: var(--n2);
  }
  .eng-sum {
    cursor: pointer;
    padding: 0.55rem 0.75rem;
    font-size: var(--fs-mini);
    letter-spacing: 0.02em;
    color: var(--n11);
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .eng-sum::marker {
    color: var(--n8);
  }
  .eng-n {
    font-size: var(--fs-micro);
    color: var(--n9);
  }
  .eng-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 0.7rem 1rem;
    padding: 0.25rem 0.75rem 0.75rem;
  }
  .eng-row {
    display: grid;
    gap: 0.2rem;
  }
  .eng-lab {
    font-size: var(--fs-micro);
    color: var(--n9);
    letter-spacing: 0.02em;
  }
  .eng-in {
    padding: 0.35rem 0.5rem;
    font: inherit;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    color: var(--n11);
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: 6px;
  }
  .eng-in::placeholder {
    color: var(--n8);
    font-variant-numeric: normal;
  }
  .eng-in:focus,
  .eng-in:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: 1px;
  }
  .eng-note {
    font-size: var(--fs-micro);
    line-height: 1.45;
    color: var(--n8);
  }
  .eng-foot {
    margin: 0;
    padding: 0 0.75rem 0.8rem;
    font-size: var(--fs-micro);
    line-height: 1.5;
    color: var(--n9);
  }
  @media (max-width: 520px) {
    .term .bt-chart {
      height: clamp(260px, 50svh, 420px);
      flex-basis: clamp(260px, 50svh, 420px);
    }
  }
</style>
