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
   * **No per-level ladder breakdown and no winning condition mask.** The record
   * holds `depth` and `combinations` as two scalars and holds the run's blake3
   * identity rather than the mask it was taken over. So "where did the frontier
   * empty" and "which conditions won" cannot be answered from this file. Both
   * are drawn as NAMED GAPS with the reason, because a missing drill-down that
   * says why is information and a missing drill-down that is simply absent is a
   * page that looks finished and is not.
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
  import { ask } from '$lib/ask.js';
  import { rupee, group, exact } from '$lib/money.js';
  import * as prefix from '$lib/prefix.js';
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

  async function fetchLedger() {
    load.phase = 'loading';
    try {
      const response = await ask(`/backtest.json?limit=${LIMIT}`, { cache: 'no-store' });
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
  });

  /* ====================================================================
     WHAT THE PAYLOAD SAYS
     ==================================================================== */

  const ledger = $derived(load.body);
  /** Every run the server returned, newest first — the server ordered them. */
  const allRuns = $derived(ledger?.runs ?? []);
  /** The server's own refusal sentence, when it had one. */
  const refusal = $derived(ledger?.refusal ?? null);

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

  /* ---- the summary ---------------------------------------------------- */
  const completeRuns = $derived(runs.filter((r) => !r.halted));
  const haltedRuns = $derived(runs.filter((r) => r.halted));
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
        const complete = present.filter((r) => !r.halted);
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
          haltedCount: present.length - complete.length
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
      const response = await ask(url, { cache: 'no-store', ms: 30_000 });
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
      const response = await ask(`/store.json?feed=${encodeURIComponent(run.feed)}`, {
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
      const response = await ask(
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

  /** The vertical extent a curve needs, symmetric so zero stays on a line. */
  function curveScale(points) {
    return Math.max(1, ...points.map((p) => Math.abs(p.v)));
  }

  /** The polyline through a cumulative series. */
  function linePath(points) {
    const scale = curveScale(points);
    const step = 1000 / Math.max(1, points.length - 1);
    return points
      .map((p, i) => `${i === 0 ? 'M' : 'L'}${(i * step).toFixed(1)} ${(130 - (p.v / scale) * 120).toFixed(1)}`)
      .join(' ');
  }

  /** The same polyline, closed to the zero line, for the area fill. */
  function areaPath(points) {
    const step = 1000 / Math.max(1, points.length - 1);
    return `${linePath(points)} L${((points.length - 1) * step).toFixed(1)} 130 L0 130 Z`;
  }

  /** The tallest bar in a set, so a column chart shares one scale. */
  function barScale(bars) {
    return Math.max(1, ...bars.map((b) => Math.abs(b.v)));
  }

  /** At most seven x labels, evenly spaced, so the axis never crowds. */
  function barLabels(bars) {
    if (bars.length === 0) return [];
    if (bars.length <= 7) return bars.map((b) => b.label);
    const out = [];
    for (let i = 0; i < 7; i += 1) out.push(bars[Math.round((i * (bars.length - 1)) / 6)].label);
    return out;
  }

  /** Where a basis-point value falls across the histogram's own range. */
  function histX(h, bps) {
    const span = Math.max(1, h.hi - h.lo);
    return (((bps - h.lo) / span) * 1000).toFixed(1);
  }

  /** Basis points as a signed percent, for the histogram's axis and legend. */
  const pctOf = (bps) => `${bps >= 0 ? '+' : ''}${(bps / 100).toFixed(2)}%`;

  /** A basis-point integer as a percent string, or the em dash. */
  const pct = (bps) => (bps === null || bps === undefined ? '—' : `${bps >= 0 ? '+' : ''}${(bps / 100).toFixed(2)}%`);

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

  /** @param {any} r */
  const span = (r) =>
    `${r.from_year}-${String(r.from_month).padStart(2, '0')} → ${r.to_year}-${String(r.to_month).padStart(2, '0')}`;

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
      <p class="sub">
        Every sweep this store has recorded, newest first. Ranked on <b>worst-case fills</b>,
        which is what selection ranks on everywhere in this workspace.
      </p>
    </div>
    <button class="btn ghost" onclick={fetchLedger} disabled={load.phase === 'loading'}>
      {load.phase === 'loading' ? 'Reading…' : 'Re-read ledger'}
    </button>
  </header>

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
      class:note={refusal.includes('not an error')}
    >
      <h2>
        {refusal.includes('not an error')
          ? 'Nothing has been swept yet'
          : 'The ledger could not be read'}
      </h2>
      <p>{refusal}</p>
      {#if ledger?.path}
        <p class="path"><span class="lab">file</span> <code>{ledger.path}</code></p>
      {/if}
    </div>
  {:else}
    <!-- ==============================================================
         SUMMARY — the four facts before any detail
         ============================================================== -->
    <section class="strip">
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
        <span class="n">comparable against each other</span>
      </div>
      <div class="fact" class:warn={haltedRuns.length > 0}>
        <span class="k">Halted</span>
        <span class="v">{exact(haltedRuns.length)}</span>
        <span class="n">
          {haltedRuns.length === 0
            ? 'no ladder was cut short'
            : 'excluded from ranking — depth is partial'}
        </span>
      </div>
      <div class="fact" class:warn={holedRuns.length > 0}>
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
      <div class="panel note">
        <h2>No runs under {activeFeed || 'this feed'}</h2>
        <p>
          The ledger holds {exact(allRuns.length)}
          {allRuns.length === 1 ? 'run' : 'runs'}, none of them recorded against
          <b>{activeFeed}</b>. A sweep is stamped with the feed its bars came from, so a run under
          another vendor is a different run and not this one seen differently.
        </p>
        {#if allRuns.length > 0}
          <button class="btn" onclick={() => (everyFeed = true)}>Show every feed</button>
        {/if}
      </div>
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
          <div class="panel note">
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
        <h2 class="bh">Which rung carries the edge</h2>
        <p class="bsub">
          One row per comparable span — same feed, same instrument, same window, same support
          threshold. Bars share one scale within a row, so height is comparable across rungs. A
          rung that was never swept for a span is <b>absent, and said to be</b>: it is not a zero.
        </p>

        {#each rungGroups as g (g.key)}
          <div class="rgroup">
            <div class="rgroup-head">
              <b>{g.underlying}</b>
              <span class="dim">{g.feed}</span>
              <span class="dim">{g.from} → {g.to}</span>
              <span class="dim">
                {g.perMille < 0 ? 'support ratio unknown — no bars' : `support ≥ ${(g.perMille / 10).toFixed(1)}% of bars`}
              </span>
              <span class="pill">
                {g.present.length}
                {g.present.length === 1 ? 'rung' : 'rungs'} swept at this ratio
              </span>
              {#if g.haltedCount > 0}
                <span class="pill warn">{g.haltedCount} halted</span>
              {/if}
            </div>
            <div class="multiples">
              {#each g.present as r (r.index)}
                <button
                  class="mult"
                  class:halted={r.halted}
                  class:leader={g.leader && r.index === g.leader.index}
                  onclick={() => toggle(r)}
                  title={r.halted
                    ? 'Halted — not comparable with the complete rungs beside it'
                    : `Worst-case ${money(r.pessimistic)} over ${exact(r.trades)} trades`}
                >
                  <span class="mult-bars">
                    <span
                      class="mult-ghost"
                      style="height:{Math.min(100, (Math.abs(r.optimistic) / g.scale) * 100)}%"
                    ></span>
                    <span
                      class="mult-bar"
                      class:neg={r.pessimistic < 0}
                      style="height:{Math.min(100, (Math.abs(r.pessimistic) / g.scale) * 100)}%"
                    ></span>
                  </span>
                  <span class="mult-rung">{r.timeframe}</span>
                  <span class="mult-fig">{money(r.pessimistic)}</span>
                  {#if r.halted}<span class="mult-flag">halted</span>{/if}
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
            <div class="spacer" style="height:{spacerH}px">
              {#each windowed as r, i (r.index)}
                <div
                  class="row"
                  class:halted={r.halted}
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
                    {#if r.halted}
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
                class="chart"
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
                    {#each [{ k: "Winners' adverse (MAE)", v: openRun.winner_mae, t: 'warnbar' }, { k: "Winners' favourable (MFE)", v: openRun.winner_mfe, t: '' }, { k: 'All trades, adverse', v: openRun.all_mae, t: 'downbar' }] as e (e.k)}
                      <div class="tt-plrow">
                        <span class="tt-pllab">{e.k}</span>
                        <span class="tt-plbar"><span class="tt-plfill {e.t}" style="width:{(Math.abs(e.v) / excursionTop) * 100}%"></span></span>
                        <span class="tt-plval">{group(e.v)} ppm</span>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>

              <!-- ================= TRADES ANALYSIS ================= -->
              <div class="tt-sec">
                <h4 class="tt-h">Trades analysis</h4>
                <div class="tt-pills" role="group" aria-label="Trades analysis">
                  {#each [['distribution', 'Distribution'], ['streaks', 'Streaks'], ['details', 'Trades analysis details']] as [key, label] (key)}
                    <button class="tt-pill" class:on={taTab === key} onclick={() => (taTab = key)}>{label}</button>
                  {/each}
                </div>

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
                        <tr class="own"><td>Winners' adverse excursion <span class="tt-own">brutex</span></td><td class="n">{group(openRun.winner_mae)} ppm</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr class="own"><td>Winners' favourable excursion <span class="tt-own">brutex</span></td><td class="n">{group(openRun.winner_mfe)} ppm</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr class="own"><td>All trades' adverse excursion <span class="tt-own">brutex</span></td><td class="n">{group(openRun.all_mae)} ppm</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr class="own"><td>Signal bars swept <span class="tt-own">brutex</span></td><td class="n">{exact(openRun.bars)}</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                      </tbody>
                    </table>
                  </div>
                {/if}
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
                      <tr><td>Support threshold</td><td class="n">{exact(openRun.min_hits)} hits · {(supportPerMille(openRun) / 10).toFixed(1)}% of bars</td></tr>
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

                <h5 class="tt-h5">The winning combination <span class="tt-own">not recorded</span></h5>
                <p class="tt-note2">
                  <b>Which market conditions actually fired cannot be named from this file.</b> The record stores the run's
                  identity — a blake3 over the mask and eight other terms — not the mask itself.
                </p>
                <p class="tt-ident"><span class="tt-k">identity</span><code>{openRun.identity}</code></p>
              </div>
            {/if}
          </section>
        </section>
      {/if}
    {/if}
  {/if}
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
  .panel.note {
    border-left: 3px solid var(--info);
  }
  .wait {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .path code,
  .ident code {
    font-size: 0.78rem;
    color: var(--n10);
    word-break: break-all;
  }
  .lab {
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--n8);
    margin-right: 0.5rem;
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
  .strip {
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
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--n8);
  }
  .fact .v {
    font-size: 1.5rem;
    font-variant-numeric: tabular-nums;
    color: var(--n12);
    line-height: 1.1;
  }
  .fact .n {
    font-size: 0.75rem;
    color: var(--n9);
  }
  .fact.good .v {
    color: var(--up);
  }
  .fact.warn .v {
    color: var(--warn);
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

  /* ---------------- blocks ---------------- */
  .block {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .bh {
    margin: 0;
    font-size: 1rem;
    color: var(--n12);
  }
  .bsub {
    margin: 0;
    font-size: 0.82rem;
    color: var(--n9);
    max-width: 78ch;
  }
  .bsub b,
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
    font-size: 0.62rem;
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
    font-size: 1.1rem;
    color: var(--n12);
  }
  .crown-figs {
    display: flex;
    gap: 1.5rem;
    flex-wrap: wrap;
  }
  .fig {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .fig .k {
    font-size: 0.64rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--n8);
  }
  .fig .v {
    font-size: 1rem;
    font-variant-numeric: tabular-nums;
    color: var(--n11);
  }
  .fig.lead .v {
    font-size: 1.35rem;
  }
  .fig .v.up {
    color: var(--up);
  }
  .fig .v.ghost {
    color: var(--n9);
  }
  .crown-note {
    margin: 0;
    font-size: 0.78rem;
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
    font-size: 0.72rem;
    color: var(--n11);
    font-variant-numeric: tabular-nums;
  }
  .mult-fig {
    font-size: 0.66rem;
    color: var(--n9);
    font-variant-numeric: tabular-nums;
  }
  .mult-flag {
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--warn);
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
    font-size: 0.66rem;
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

  .spacer {
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
  .sm {
    font-size: 0.7rem;
  }
  .rung {
    font-size: 0.72rem;
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
    font-size: 0.64rem;
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
  .card h3 {
    margin: 0;
    font-size: 0.88rem;
    color: var(--n12);
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
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
    font-size: 0.72rem;
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
    font-size: 0.66rem;
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
  .kv dt {
    color: var(--n8);
  }
  .kv dd {
    margin: 0;
    text-align: right;
    color: var(--n11);
    font-variant-numeric: tabular-nums;
  }
  .kv dd.neg {
    color: var(--down);
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
    font-size: 0.66rem;
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
    font-size: 0.74rem;
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
  .axis.unused .axis-v {
    color: var(--n8);
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
    font-size: 0.74rem;
    color: var(--n8);
    font-variant-numeric: tabular-nums;
  }
  .chart-facts .warnt {
    color: var(--warn);
  }
  /* A HEIGHT IN PIXELS AND A REFUSAL TO SHRINK, and both halves are a fix for
     a measured bug. `autoSize` measures its host, so a host with no height
     measures zero; and `.card` is a column flex container, where a child
     defaults to `flex-shrink: 1` — which took this box to 32px against its
     declared 340px and drew a chart that looked like one that had failed.
     Exactly the defect the `.page > *` rule above fixes one level up. */
  .chart {
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
  .report > * {
    flex: 0 0 auto;
  }
  .rep-head {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .rep-head h3 {
    margin: 0;
    font-size: 0.95rem;
    color: var(--n12);
  }
  .rep-sub {
    font-size: 0.74rem;
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
    font-size: 0.63rem;
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
  .bench h4 {
    margin: 0 0 0.5rem;
    font-size: 0.82rem;
    color: var(--n11);
  }
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
    font-size: 0.68rem;
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
  .cnote .up {
    color: var(--up);
  }
  .cnote .down {
    color: var(--down);
  }
  .spin.sm {
    width: 10px;
    height: 10px;
    display: inline-block;
    vertical-align: -1px;
    margin-right: 0.35rem;
  }

  /* ---- the coverage table ----
     A `<details>` because it is long and it is REFERENCE: an operator reads
     it once to learn what this surface can and cannot answer, then never
     again. Collapsed by default, and never removed -- the whole point is
     that the gaps are documented rather than merely absent. */
  .cover {
    border-top: 1px solid var(--n5);
    padding-top: 0.75rem;
  }
  .cover summary {
    cursor: pointer;
    font-size: 0.8rem;
    color: var(--n10);
    padding: 0.2rem 0;
  }
  .cover summary:hover {
    color: var(--acc);
  }
  .cover summary:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
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
  .covtbl th {
    text-align: left;
    background: var(--n2);
    color: var(--n8);
    font-size: 0.64rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-weight: 600;
    padding: 0.5rem 0.7rem;
    border-bottom: 1px solid var(--n6);
  }
  .covtbl td {
    padding: 0.45rem 0.7rem;
    border-bottom: 1px solid var(--n5);
    color: var(--n9);
    vertical-align: top;
  }
  .covtbl tr:last-child td {
    border-bottom: 0;
  }
  .covtbl td:first-child {
    color: var(--n11);
    white-space: nowrap;
  }
  .covtbl .pill.bad {
    background: var(--down-soft);
    color: var(--down);
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
    font-size: 0.68rem;
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
    font-size: 0.72rem;
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
    font-size: 0.68rem;
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
    font-size: 0.73rem;
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
    font-size: 0.6rem;
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
  .tester .tt-quad > .tt-q,
  .tester .tt-stats > .tt-q {
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
    .tester .tt-stats > .tt-q,
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
    font-size: 0.73rem;
    color: var(--n8);
    max-width: 96ch;
  }
  .cf-note b {
    color: var(--n10);
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
    font-size: 0.64rem;
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
    font-size: 0.71rem;
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
    font-size: 0.63rem;
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
  .tt-donutleg .v {
    margin-left: auto;
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
    font-size: 0.73rem;
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
    font-size: 0.72rem;
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
    font-size: 0.65rem;
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
    background: #2f3241;
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
    background: #1c2030;
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
    color: #f0b429;
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
    background: #434651;
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
    font-size: 0.6rem;
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
    font-size: 0.72rem;
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
  .tt-plots li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.76rem;
    color: var(--n11);
    padding: 0.22rem 0.45rem;
    border-radius: 5px;
    background: var(--n2);
  }
  .tt-plots li.off {
    color: var(--n8);
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
    font-size: 0.72rem;
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
    font-size: 0.68rem;
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
    font-size: 0.74rem;
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
    font-size: 0.72rem;
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
    font-size: 0.73rem;
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
    font-size: 0.72rem;
    color: var(--n8);
  }
  .lg {
    font-size: 0.79rem;
    color: var(--n11);
  }
  .lg i {
    font-style: normal;
    font-size: 0.66rem;
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

  .term .chart {
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
    font-size: 0.72rem;
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
    font-size: 0.72rem;
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
