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
        load = {
          phase: 'failed',
          body: null,
          why:
            `/backtest.json answered ${response.status}. That is the API refusing the ` +
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
               TradingView's own layout, its own order, its own lock glyph
               for a cell that cannot be filled. See the block comment on
               `testerView` for why the padlock is the right idiom here.
               ========================================================== -->
          <section class="tester">
            <!-- ---- toolbar ---- -->
            <div class="tt-bar">
              <div class="tt-name">
                <svg viewBox="0 0 16 16" class="tt-ico" aria-hidden="true"
                  ><path
                    d="M2 12l3.5-4 3 3L13 4"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.6"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  /></svg
                >
                <b>Brute-force sweep</b>
                <span class="tt-dim">{openRun.underlying} · {openRun.timeframe}</span>
              </div>
              <div class="tt-views" role="group" aria-label="Report view">
                <button class="tt-view" class:on={testerView === 'metrics'} onclick={() => (testerView = 'metrics')}>Metrics</button>
                <button class="tt-view" class:on={testerView === 'trades'} onclick={() => (testerView = 'trades')}>Trades</button>
                <button class="tt-view" class:on={testerView === 'properties'} onclick={() => (testerView = 'properties')}>Properties</button>
              </div>
              <div class="tt-meta">
                <span class="tt-chip">{span(openRun)}</span>
                <span class="tt-chip">{exact(openRun.months_found)}/{exact(openRun.months_asked)} months</span>
                <span class="tt-chip">support ≥ {(supportPerMille(openRun) / 10).toFixed(1)}%</span>
                {#if openRun.halted}
                  <span class="tt-chip warn">halted · depth {openRun.depth} partial</span>
                {:else}
                  <span class="tt-chip good">complete · depth {openRun.depth}</span>
                {/if}
              </div>
            </div>

            {#if testerView === 'metrics'}
              <!-- ================= KEY STATS ================= -->
              <div class="tt-sec">
                <h4 class="tt-h">Key stats</h4>
                <div class="tt-stats">
                  <div class="tt-stat">
                    <span class="tt-k">Total PnL</span>
                    <span class="tt-v" class:up={openRun.pessimistic >= 0} class:down={openRun.pessimistic < 0}>
                      {money(openRun.pessimistic)}
                      <em class="tt-sub">{pct(strategyBps)}</em>
                    </span>
                    <span class="tt-note">worst-case fills · {money(openRun.optimistic)} at best</span>
                  </div>
                  <div class="tt-stat">
                    <span class="tt-k">Max drawdown</span>
                    <span class="tt-v down">
                      {money(Math.abs(openRun.max_drawdown))}
                      <em class="tt-sub">{drawdownBps === null ? '' : `${(drawdownBps / 100).toFixed(2)}%`}</em>
                    </span>
                    <span class="tt-note">worst peak-to-trough</span>
                  </div>
                  <div class="tt-stat">
                    <span class="tt-k">Profitable trades</span>
                    <span class="tt-v"
                      ><Lock why="No win count is recorded. A net total cannot be split into winners and losers after the fact." /></span
                    >
                    <span class="tt-note">of {exact(openRun.trades)} closed</span>
                  </div>
                  <div class="tt-stat">
                    <span class="tt-k">Profit factor</span>
                    <span class="tt-v"
                      ><Lock why="Needs gross profit and gross loss separately; the sweep records only the net." /></span
                    >
                    <span class="tt-note">gross profit ÷ gross loss</span>
                  </div>
                </div>
              </div>

              <!-- ================= PERFORMANCE ================= -->
              <div class="tt-sec">
                <h4 class="tt-h">Performance</h4>
                <div class="tt-perf">
                  <ul class="tt-plots">
                    <li class="off">Cumulative PnL <Lock small why="No equity series is recorded — four scalars cannot make a curve." /></li>
                    <li>Buy and hold <span class="tt-dot on" aria-hidden="true"></span></li>
                    <li class="off">Trades excursions <Lock small why="Three aggregates are recorded, not one column per trade." /></li>
                    <li class="off">Run-ups and drawdowns <Lock small why="One drawdown figure is recorded, not a series over time." /></li>
                  </ul>
                  <div class="tt-plot">
                    {#if bench.phase === 'loading'}
                      <div class="tt-empty"><span class="spin sm" aria-hidden="true"></span> Reading the span's opening price…</div>
                    {:else if outperformance && buyHold}
                      {@const scale = Math.max(1, Math.abs(outperformance.strategy), Math.abs(outperformance.hold))}
                      <div class="tt-bench">
                        <div class="tt-brow">
                          <span class="tt-blab">Strategy</span>
                          <span class="tt-bbar"
                            ><span class="tt-bfill" class:down={outperformance.strategy < 0} style="width:{(Math.abs(outperformance.strategy) / scale) * 100}%"></span></span
                          >
                          <span class="tt-bval strong">{money(outperformance.strategy)}</span>
                        </div>
                        <div class="tt-brow">
                          <span class="tt-blab">Buy and hold</span>
                          <span class="tt-bbar"
                            ><span class="tt-bfill hold" class:down={outperformance.hold < 0} style="width:{(Math.abs(outperformance.hold) / scale) * 100}%"></span></span
                          >
                          <span class="tt-bval">{money(outperformance.hold)}</span>
                        </div>
                      </div>
                      <p class="tt-plotnote">
                        The cumulative-PnL curve TradingView draws here needs an equity series the sweep never wrote.
                        <b>Buy and hold is the one plot on this list that is computable</b> — from the bars on disk — so it is
                        the one that is drawn.
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
                    <div class="tt-q"><span class="tt-k">Commission load</span><span class="tt-qv"><Lock why="`crates/costs` applies costs inside the sweep; the record keeps the net, not the fee line." /></span></div>
                  </div>
                  <!-- OURS, NOT TRADINGVIEW'S. Marked so, because a row nobody
                       can find in the Strategy Tester should say where it came
                       from rather than look like one they missed. -->
                  <h5 class="tt-h5">Fill models <span class="tt-own">brutex</span></h5>
                  <p class="tt-note2">
                    TradingView simulates one fill model. This sweep records the same combination under both ends of every
                    bar, and ranks on the worse — so the spread between them is a figure the Strategy Tester has no row for.
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
                    <div class="tt-q"><span class="tt-k">Sharpe ratio</span><span class="tt-qv"><Lock why="`crates/runner` computes significance, but no ratio reaches the record." /></span></div>
                    <div class="tt-q"><span class="tt-k">Sortino ratio</span><span class="tt-qv"><Lock why="Same — not written to the ledger." /></span></div>
                  </div>
                  <p class="tt-note2">
                    Return is on <b>one unit of the index</b>, against the price at the span's start
                    ({money(bench.open)}). The ledger records no capital, so a return on equity has no denominator on disk —
                    and inventing one would make every percentage here a number of my choosing. Annualised over the
                    {exact(openRun.months_found)} months actually found, not the {exact(openRun.months_asked)} asked for.
                  </p>
                  <div class="tt-lockpanel">
                    <Lock /> <span>Daily / weekly / quarterly / yearly PnL needs per-period totals. The sweep records one total for the whole span.</span>
                  </div>
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
                {:else if paTab === 'margin'}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Margin efficiency</span><span class="tt-qv"><Lock why="The engine models no account." /></span></div>
                    <div class="tt-q"><span class="tt-k">Average margin used</span><span class="tt-qv"><Lock why="The engine models no account." /></span></div>
                    <div class="tt-q"><span class="tt-k">Margin calls</span><span class="tt-qv"><Lock why="The engine models no account." /></span></div>
                    <div class="tt-q"><span class="tt-k">Total liquidated volume</span><span class="tt-qv"><Lock why="The engine models no account." /></span></div>
                  </div>
                  <div class="tt-lockpanel">
                    <Lock />
                    <span>
                      <b>Every row on this tab is locked, and that is a design fact rather than a gap to fill.</b> The engine
                      computes totals in index points with no capital, no position size and no broker. There is no margin to
                      use, so there is nothing here to record.
                    </span>
                  </div>
                {:else}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Average run-up duration</span><span class="tt-qv"><Lock why="Needs a run-up series over time." /></span></div>
                    <div class="tt-q"><span class="tt-k">Average drawdown duration</span><span class="tt-qv"><Lock why="Needs a drawdown series over time." /></span></div>
                    <div class="tt-q"><span class="tt-k">Max drawdown</span><span class="tt-qv down">{money(Math.abs(openRun.max_drawdown))}</span></div>
                    <div class="tt-q"><span class="tt-k">Max drawdown as % of opening price</span><span class="tt-qv">{drawdownBps === null ? '—' : `${(drawdownBps / 100).toFixed(2)}%`}</span></div>
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
                  <div class="tt-lockpanel">
                    <Lock />
                    <span>
                      The returns histogram and the winners/losers donut both need the outcome of every individual trade. The
                      sweep records <b>{exact(openRun.trades)}</b> as a count and keeps no list, so neither can be drawn — and
                      a donut split on a guess would be the most convincing wrong picture on this page.
                    </span>
                  </div>
                {:else if taTab === 'streaks'}
                  <div class="tt-quad">
                    <div class="tt-q"><span class="tt-k">Longest winning streak</span><span class="tt-qv"><Lock why="Needs the ordered win/loss outcome of every trade." /></span></div>
                    <div class="tt-q"><span class="tt-k">Longest losing streak</span><span class="tt-qv"><Lock why="Needs the ordered win/loss outcome of every trade." /></span></div>
                    <div class="tt-q"><span class="tt-k">Average winning streak</span><span class="tt-qv"><Lock why="Needs the ordered win/loss outcome of every trade." /></span></div>
                    <div class="tt-q"><span class="tt-k">Average losing streak</span><span class="tt-qv"><Lock why="Needs the ordered win/loss outcome of every trade." /></span></div>
                  </div>
                  <div class="tt-lockpanel">
                    <Lock />
                    <span>
                      Every figure on this tab is a property of the ORDER trades resolved in. The ledger keeps a count and a
                      net, both of which are order-independent, so nothing here is recoverable from it.
                    </span>
                  </div>
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
                        <tr><td>Average PnL</td><td class="n">{perTrade ? money(perTrade.worst) : '—'}</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average profit</td><td class="n"><Lock small why="Needs gross profit and a winner count." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average loss</td><td class="n"><Lock small why="Needs gross loss and a loser count." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average profit / average loss</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Largest profit</td><td class="n"><Lock small why="Only the worst single trade is kept." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Largest loss</td><td class="n down">{money(openRun.worst_trade)}</td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Outliers</td><td class="n"><Lock small why="Needs a per-trade list." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
                        <tr><td>Average bars in trades</td><td class="n"><Lock small why="bars ÷ trades is the average gap BETWEEN trades, a different quantity. Not shown rather than shown wrong." /></td><td class="n"><Lock small /></td><td class="n"><Lock small /></td></tr>
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
                <h4 class="tt-h">List of trades</h4>
                <div class="tt-tblwrap">
                  <table class="tt-tbl ghosted">
                    <thead>
                      <tr>
                        <th>Trade #</th><th>Type</th><th>Date and time</th><th>Signal</th><th class="n">Price</th>
                        <th class="n">Size</th><th class="n">Net PnL</th><th class="n">Return</th>
                        <th class="n">Favorable excursion</th><th class="n">Adverse excursion</th>
                        <th class="n">Cumulative PnL</th><th class="n">Duration (bars)</th>
                      </tr>
                    </thead>
                    <tbody>
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
