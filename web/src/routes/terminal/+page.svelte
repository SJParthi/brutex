<script>
  /**
   * `/terminal` — THE MARKETS TERMINAL, RECREATED, FED BY THIS STORE.
   *
   * # What this page is
   *
   * A faithful recreation of a broker's markets terminal: its chrome, its
   * index strip, its watchlist rail, its tab strip, its filter chips and its
   * grid. The geometry and the colour are copied deliberately and measured
   * rather than guessed — see THE PALETTE below for how every value here was
   * obtained.
   *
   * # What it is NOT, and this is the whole of the design
   *
   * It is not fed by a tick stream, because there is no tick stream. Every
   * figure on it is read from THIS store, at run time, through the same
   * endpoints every other page reads. Three consequences, none of them
   * optional:
   *
   *   * NOTHING IS HARDCODED. There is no list of instruments in this file, no
   *     list of months, no list of exchanges. The rail, the strip, the tabs and
   *     the grid are all folds over `/store.json`'s census, so the day a symbol
   *     is pulled is the day it appears here, and no edit to this file is part
   *     of that.
   *
   *   * AN ABSENCE IS NAMED, NEVER ZEROED. The terminal this recreates prints
   *     `0.00` for a price it does not have and `-100.00 %` for a change it
   *     cannot compute — measured across the captures this page was built
   *     from, whole screens of them. `CLAUDE.md` section 4 bans that shape, so
   *     every cell here is either a number that was read or a reason it was
   *     not, and `$lib/terminal.svelte.js` is where the rule is enforced.
   *
   *   * THE INCREMENT IS THE VIEWPORT. A price exists only in a bar file, one
   *     request per series, so a grid of 215 instruments would be 215
   *     requests. Quotes are fetched for the rows on screen and cached; see
   *     `quotesFor`.
   *
   * # The columns this store cannot fill, and why they are still drawn
   *
   * `Spot Price`, `Premium`, `OI Change`, `OI Change %` and `Value (Cr.)` have
   * no source in a store of OHLCV bars. They are drawn anyway, at their real
   * widths, carrying a dash and the reason — because the alternative is a grid
   * that silently changes shape depending on what is missing, and a reader who
   * has never seen the full grid cannot tell a narrow table from a complete
   * one. The layout is the fact; the dash is the honesty.
   */

  import { feeds, loadFeeds } from '$lib/feeds.svelte.js';
  import { store, syncStore } from '$lib/store.svelte.js';
  import { tabsFrom, quotesFor, forgetQuotes, TABS } from '$lib/terminal.svelte.js';
  import { parseKey } from '$lib/instrument.js';
  import { group, rupee } from '$lib/money.js';
  import { monthLabel } from '$lib/dates.js';

  /* ==================================================================
     THE FEED, AND THE CENSUS UNDER IT
     ------------------------------------------------------------------
     Exactly the arrangement every other page uses: the feed list loads
     once, and the census re-reads whenever the chosen feed changes.
     `syncStore` de-duplicates by (feed, generation), so calling it from
     an effect costs one request per real change and nothing per repaint.
     ================================================================== */
  $effect(() => {
    loadFeeds();
  });
  $effect(() => {
    syncStore(feeds.active);
  });
  /* A PULL WRITES BARS UNDER KEYS THE QUOTE CACHE ALREADY ANSWERED FOR.
     `generation` is the census's own clock and it advances on a refresh, so
     that is the moment a held quote stops being current. Clearing here rather
     than on a timer means a quote is re-read exactly when there is a reason to
     believe it changed, and never otherwise. */
  $effect(() => {
    void store.generation;
    forgetQuotes();
  });

  /** Every instrument key the census holds, sorted, deduplicated.
      @type {string[]} */
  const keys = $derived(/** @type {string[]} */ ([...store.byInstrument.keys()]).sort());

  /** The tab strip, with each tab's rows decided by the store. */
  const tabs = $derived(tabsFrom(keys));

  let activeTab = $state('index');
  const tab = $derived(tabs.find((t) => t.id === activeTab) ?? tabs[0]);

  /** The rail's free-text filter, and the row it has selected. */
  let filter = $state('');
  let picked = $state('');

  /* ==================================================================
     THE RUNG AND THE MONTH — the two axes a bar file is filed under.
     Both are derived from the census, so the pickers can only ever offer
     something the store actually holds. A hardcoded default here is the
     defect `/db` recorded: a fixed span threw away 40 of 121 months and
     reported no hole while doing it.
     ================================================================== */
  const rungs = $derived(
    [...new Set(store.readable.map((c) => c.timeframe))].sort(
      (a, b) => rungOrder(a) - rungOrder(b)
    )
  );
  const months = $derived([...store.byMonth.keys()].sort());

  /** Rung order by duration, so `2min` sorts before `10min` and not after it.
      @param {string} t @returns {number} */
  function rungOrder(t) {
    const m = String(t).match(/^(\d+)(min|day)$/);
    if (!m) return Number.MAX_SAFE_INTEGER;
    return Number(m[1]) * (m[2] === 'day' ? 1440 : 1);
  }

  let rung = $state('');
  let month = $state('');
  /* THE PICKERS FOLLOW THE STORE RATHER THAN LEADING IT. When the census
     lands, or the feed changes under a chosen value, the current choice may no
     longer exist. Falling back to the newest month and the coarsest rung the
     store holds is a choice this page can defend; keeping a value the store
     does not have would render an empty grid that reads as "no data" when it
     means "you asked for a month nobody pulled". */
  $effect(() => {
    if (!months.includes(month)) month = months[months.length - 1] ?? '';
  });
  $effect(() => {
    if (!rungs.includes(rung)) rung = rungs[rungs.length - 1] ?? '';
  });

  /* ==================================================================
     THE ROWS ON SCREEN, AND THE QUOTES FOR THEM
     ------------------------------------------------------------------
     `PAGE` is what the source terminal shows without scrolling. It is the
     increment: the grid asks for this many quotes, and asks for more when
     the reader asks for more. Not a limit on the store — a limit on what
     is in flight.
     ================================================================== */
  const PAGE = 17;
  let shown = $state(PAGE);
  $effect(() => {
    void activeTab;
    void month;
    void rung;
    shown = PAGE;
  });

  const rows = $derived((tab?.keys ?? []).filter(holdsChosenCell).slice(0, shown));
  const totalRows = $derived((tab?.keys ?? []).filter(holdsChosenCell).length);

  /* THE RAIL IS A SECOND CONSUMER OF QUOTES AND WAS NOT ASKING FOR ANY.
     Measured in the browser: with the Index tab open, ADANIENT and BAJAJ-AUTO
     sat in the rail showing an em dash — the same glyph this page uses for "the
     store has nothing" — while the store held bars for both. The reader cannot
     tell "absent" from "not requested", which is the failure section 4 bans
     arrived at by omission rather than by a wrong number.

     Two halves to the fix. The rail's own visible head is quoted here, joined
     to the grid's rows so an instrument in both costs one request; and the
     rows past that head render a dash that SAYS it was never asked for. A
     bounded head rather than the whole rail, because the rail lists everything
     held and the engine surface is 215 instruments. */
  const railKeys = $derived(
    keys.filter((k) => labelOf(k).toLowerCase().includes(filter.toLowerCase()))
  );
  const railQuoted = $derived(new Set(railKeys.slice(0, PAGE)));

  /** The union the quote pool actually asks for: the grid's page and the
      rail's head, deduplicated, in a stable order. */
  const wanted = $derived([...new Set([...rows, ...railQuoted])]);

  /** Whether this instrument has a cell at the chosen (rung, month). O(1).
      @param {string} key @returns {boolean} */
  function holdsChosenCell(key) {
    if (!rung || !month) return false;
    return store.byCell.has(`${key}|${rung}|${month}`);
  }

  /** key -> Quote, filled as the pool answers. */
  let quotes = $state(new Map());

  $effect(() => {
    const feed = store.feed;
    const want = wanted;
    const tf = rung;
    const mo = month;
    if (!feed || !tf || !mo || want.length === 0) return;
    let live = true;
    quotesFor(
      feed,
      want.map((key) => ({ key, timeframe: tf, month: mo }))
    ).then((answers) => {
      if (!live) return;
      const next = new Map(quotes);
      for (const q of answers) next.set(q.key, q);
      quotes = next;
    });
    return () => {
      live = false;
    };
  });

  /* ==================================================================
     THE INDEX STRIP — the census's INDEX-segment instruments.
     Five cells in the source terminal; here it is however many indices
     this store holds, because inventing a sixth would be inventing a
     market. ================================================================== */
  const indices = $derived(
    keys.filter((k) => parseKey(k).segment === 'INDEX' && holdsChosenCell(k)).slice(0, 6)
  );

  /* ==================================================================
     THE COLUMN MODEL — per tab, exactly as the source terminal draws it.
     `src` says where a cell comes from:
        'name'    the instrument itself
        a field   read off the quote
        null      NO SOURCE IN THIS STORE — drawn, dashed, and explained.
     ================================================================== */
  const NO_SRC = {
    spot: 'A spot price is the underlying’s level at this bar. This store files a contract’s own bars and does not join them to the underlying’s series, so nothing here has read one.',
    premium: 'Premium is a future’s price minus its underlying’s. It needs the underlying’s bar for the same timestamp, which is a join this store does not make.',
    oichg: 'Open-interest change needs the previous bar’s open interest. `/bars/window.json` carries a change field for price and none for OI, so this was never measured.',
    value: 'Turnover in crore is price times quantity summed over the session. A bar file carries volume and close but not per-trade value, and multiplying the two would be an estimate presented as a measurement.'
  };

  /**
   * ONE COLUMN OF THE GRID.
   *
   * `src` is the whole contract: a field name reads off the quote, `'name'` is
   * the instrument itself, and `null` means THIS STORE HAS NO SOURCE — which is
   * a different statement from "the value is missing today" and is why `why`
   * exists beside it.
   *
   * @typedef {object} Col
   * @property {string} h        the header, exactly as the source terminal writes it
   * @property {string|null} src the quote field, `'name'`, or `null` for no source
   * @property {'left'|'right'} align
   * @property {string} [kind]   how to format: `price`, `int`, `signed`, `pct`
   * @property {string} [why]    why there is no source, shown on hover
   */

  /** @type {Record<string, Col[]>} */
  const COLUMNS = {
    index: [
      { h: 'Name', src: 'name', align: 'left' },
      { h: 'LTP', src: 'close', align: 'right', kind: 'price' },
      { h: 'Change', src: 'chgAbs', align: 'right', kind: 'signed' },
      { h: 'Change %', src: 'chg', align: 'right', kind: 'pct' },
      { h: 'Volume', src: 'volume', align: 'right', kind: 'int' }
    ],
    stocks: [
      { h: 'Name', src: 'name', align: 'left' },
      { h: 'LTP', src: 'close', align: 'right', kind: 'price' },
      { h: 'Change', src: 'chgAbs', align: 'right', kind: 'signed' },
      { h: 'Change %', src: 'chg', align: 'right', kind: 'pct' },
      { h: 'Value (Cr.)', src: null, why: NO_SRC.value, align: 'right' },
      { h: 'Volume', src: 'volume', align: 'right', kind: 'int' }
    ],
    options: [
      { h: 'Name', src: 'name', align: 'left' },
      { h: 'Spot Price', src: null, why: NO_SRC.spot, align: 'right' },
      { h: 'LTP', src: 'close', align: 'right', kind: 'price' },
      { h: 'Change', src: 'chgAbs', align: 'right', kind: 'signed' },
      { h: 'Change %', src: 'chg', align: 'right', kind: 'pct' },
      { h: 'Open Interest', src: 'oi', align: 'right', kind: 'int' },
      { h: 'OI Change', src: null, why: NO_SRC.oichg, align: 'right' },
      { h: 'OI Change %', src: null, why: NO_SRC.oichg, align: 'right' },
      { h: 'Volume', src: 'volume', align: 'right', kind: 'int' }
    ],
    /* FUTURES CARRIES NO `Premium` AND NO `Spot Price` IN ITS BASE SET. Both
       are inserted by `columnsFor` below, and only under the two chips that
       show them. See the note there — this was measured wrong the first time. */
    futures: [
      { h: 'Name', src: 'name', align: 'left' },
      { h: 'LTP', src: 'close', align: 'right', kind: 'price' },
      { h: 'Change', src: 'chgAbs', align: 'right', kind: 'signed' },
      { h: 'Change %', src: 'chg', align: 'right', kind: 'pct' },
      { h: 'Open Interest', src: 'oi', align: 'right', kind: 'int' },
      { h: 'OI Change', src: null, why: NO_SRC.oichg, align: 'right' },
      { h: 'OI Change %', src: null, why: NO_SRC.oichg, align: 'right' },
      { h: 'Volume', src: 'volume', align: 'right', kind: 'int' }
    ]
  };
  COLUMNS.live = COLUMNS.stocks;
  COLUMNS.commodity = COLUMNS.stocks;
  COLUMNS.etfs = COLUMNS.stocks;
  COLUMNS.bonds = COLUMNS.stocks;

  /**
   * THE COLUMN SET IS A FUNCTION OF THE TAB **AND THE CHIP**, NOT THE TAB ALONE.
   *
   * This was wrong on first build and the correction came from measuring the
   * source captures rather than from reading them: on Futures, the `Premium`
   * chip shows `Premium` and `Spot Price`, the `Discount` chip shows `Discount`
   * and `Spot Price`, and the other five chips — Top Volume, OI Gainers, OI
   * Losers, Price Gainers, Price Losers — show NEITHER. The frozen Name-column
   * divider was measured at x 725.5 under `Premium`, 740.5 under `Top Volume`
   * and 782 under `Price Losers`, so the whole grid reflows when the chip
   * changes. A table built with one fixed schema per tab is wrong on Futures,
   * and it is wrong invisibly — it renders a plausible grid with two columns
   * that should not be there.
   *
   * Only Futures varies today. The function exists rather than a second table
   * so that the next tab found to vary is one branch, not a second mechanism.
   *
   * @param {string} tab @param {string|null} chip @returns {Col[]}
   */
  function columnsFor(tab, chip) {
    const base = COLUMNS[tab] ?? COLUMNS.stocks;
    if (tab !== 'futures') return base;
    /** @type {Col[]} */
    let extra = [];
    if (chip === 'Premium') {
      extra = [
        { h: 'Premium', src: null, why: NO_SRC.premium, align: 'right' },
        { h: 'Spot Price', src: null, why: NO_SRC.spot, align: 'right' }
      ];
    } else if (chip === 'Discount') {
      extra = [
        { h: 'Discount', src: null, why: NO_SRC.premium, align: 'right' },
        { h: 'Spot Price', src: null, why: NO_SRC.spot, align: 'right' }
      ];
    }
    // AFTER `Name`, WHICH IS ALWAYS FIRST AND ALWAYS FROZEN.
    return [base[0], ...extra, ...base.slice(1)];
  }

  /* THE CHIP ROW IS PER TAB, and the chip sets genuinely differ between them
     in the terminal this recreates — Options ranks by open interest, Futures by
     basis, Stocks by intraday behaviour. They are drawn because they are part
     of the design; they do not filter, because ranking a grid this store can
     only partly fill would rank the dashes. The one that is lit is the one
     whose ordering this page actually applies. */
  /** @type {Record<string, string[]>} */
  const CHIPS = {
    index: ['Price Gainers', 'Price Losers', 'Top Volume'],
    stocks: ['Intraday Movers', 'Outperformers', 'MTF', 'Extreme Openings', 'Price Movers', 'Breakouts', 'By Value'],
    options: ['Highest OI', 'OI Gainers', 'OI Losers', 'Top Volume', 'Top Value', 'Price Gainers', 'Price Losers'],
    futures: ['Premium', 'Discount', 'Top Volume', 'OI Gainers', 'OI Losers', 'Price Gainers', 'Price Losers'],
    live: ['Price Gainers', 'Price Losers', 'Top Volume'],
    commodity: [],
    etfs: [],
    bonds: []
  };
  const chips = $derived(CHIPS[activeTab] ?? []);

  /* THE CHIP IS SELECTABLE, BECAUSE IT CHANGES THE GRID.
     It was drawn lit-but-inert on first build, which was defensible only while
     the column set depended on the tab alone. It does not: see `columnsFor`.
     A chip that reflows the table cannot be decoration.
     The selection resets with the tab, because a chip belongs to its tab —
     `Highest OI` has no meaning on Stocks and index 4 on one strip names a
     different filter on the next. */
  let chipIndex = $state(0);
  $effect(() => {
    void activeTab;
    chipIndex = 0;
  });
  /** @type {string|null} */
  const activeChip = $derived(chips[chipIndex] ?? null);

  const columns = $derived(columnsFor(activeTab, activeChip));

  /* ==================================================================
     RENDERING ONE CELL
     ------------------------------------------------------------------
     Prices are PAISA INTEGERS on the wire and `rupee` is the one place
     the decimal point is inserted — CLAUDE.md section 7. Change is
     INTEGER BASIS POINTS and the same rule applies to the ratio.
     ================================================================== */

  /** ONE DEFINITION OF A QUOTE, AND IT IS NOT THIS FILE'S. `$lib/terminal.svelte.js`
      owns the shape because it owns the fetch; restating the fields here would put
      a second copy of one contract in the tree, and two copies drift the first time
      a field is added.
      @typedef {import('$lib/terminal.svelte.js').Quote} Quote */

  /** A signed percentage from integer basis points, at 2dp, with the space
      before the sign the source terminal renders.
      @param {number} bps @returns {string} */
  function pctText(bps) {
    const v = bps / 100;
    return `${v >= 0 ? '' : '-'}${Math.abs(v).toFixed(2)} %`;
  }

  /** The absolute move implied by a close and its change in basis points.
      DERIVED, AND SAID TO BE: the wire carries the ratio and the close, not the
      difference, so this is arithmetic on two measured numbers rather than a
      third measurement. It is exact for the pair it is computed from.
      @param {Quote|undefined} quote @returns {number|null} */
  function absMove(quote) {
    if (!quote || quote.close === null || quote.chg === null) return null;
    const before = quote.close / (1 + quote.chg / 10_000);
    return quote.close - before;
  }

  /** One rendered cell: its text, its direction for colour, and the reason it
      is a dash when it is one.
      @param {Quote|undefined} quote @param {Col} col
      @returns {{text: string, why: string|null, dir: number, pending?: boolean}} */
  function cellOf(quote, col) {
    if (col.src === null) return { text: '—', why: col.why ?? null, dir: 0 };
    if (!quote) return { text: '', why: null, dir: 0, pending: true };
    if (quote.why) return { text: '—', why: quote.why, dir: 0 };

    if (col.src === 'chgAbs') {
      const v = absMove(quote);
      if (v === null) {
        return { text: '—', why: quote.chgWhy ?? 'No change was recorded for this bar.', dir: 0 };
      }
      return { text: rupee(Math.round(v)), dir: Math.sign(v), why: null };
    }
    if (col.src === 'chg') {
      if (quote.chg === null) {
        return { text: '—', why: quote.chgWhy ?? 'No change was recorded for this bar.', dir: 0 };
      }
      return { text: pctText(quote.chg), dir: Math.sign(quote.chg), why: null };
    }
    /* INDEXED THROUGH `any` DELIBERATELY, AND ONLY HERE. `col.src` is a field
       name chosen by the column table above, so the lookup is a fact about that
       table rather than about the wire; widening the Quote type to admit a
       string index would weaken it everywhere else to buy nothing. */
    const v = /** @type {Record<string, number|null>} */ (/** @type {unknown} */ (quote))[
      col.src
    ];
    if (v === null || v === undefined) {
      return {
        text: '—',
        why:
          col.src === 'oi'
            ? 'The store filed the open-interest null sentinel for this bar, which means absent. Zero would mean zero.'
            : 'This field was not on the bar.',
        dir: 0
      };
    }
    if (col.kind === 'price') return { text: rupee(v), dir: 0, why: null };
    if (col.kind === 'int') return { text: group(v), dir: 0, why: null };
    return { text: String(v), dir: 0, why: null };
  }

  /** The display name for a key — the underlying, plus a contract tail.
      @param {string} key @returns {string} */
  function labelOf(key) {
    const p = parseKey(key);
    if (!p.underlying) return key;
    if (p.kind === 'future') return `${p.underlying} ${p.expiry} FUT`;
    if (p.kind === 'option') {
      return `${p.underlying} ${p.expiry} ${Math.round((p.strike ?? 0) / 100)} ${p.side}`;
    }
    return p.underlying;
  }
</script>

<!-- ====================================================================
     THE CHROME. This page draws its own, and `+layout.svelte` stands its
     console bar down for exactly this route — see `BARE` there.
     ==================================================================== -->
<div class="term">
  <header class="tbar">
    <span class="mark" aria-hidden="true"></span>
    {#if indices.length}
      {@const lead = quotes.get(indices[0])}
      <span class="tick">
        <span class="tick-n">{labelOf(indices[0])}</span>
        {#if lead?.close != null}
          <span class="tick-v">{rupee(lead.close)}</span>
          {#if lead.chg != null}
            <span class="tick-c" class:up={lead.chg > 0} class:down={lead.chg < 0}>
              {pctText(lead.chg)}
            </span>
          {/if}
        {/if}
      </span>
    {/if}

    <nav class="tnav" aria-label="Terminal">
      <a href="/terminal" aria-current="page">Markets</a>
      <a href="/db">Store</a>
      <a href="/ingest">Ingest</a>
      <a href="/backtest">Backtest</a>
      <a href="/audit">Audit</a>
    </nav>

    <span class="tgap"></span>

    <span class="closed" title="This terminal reads a store of historical bars. There is no live session and there is no tick stream — every figure on this page is a bar that was pulled and filed.">
      <span class="cdot"></span> Store only
    </span>
    <a class="back" href="/">Console</a>
  </header>

  <!-- THE INDEX STRIP. Whatever indices this store holds, and no more. -->
  <div class="tstrip">
    {#if indices.length === 0}
      <span class="tstrip-empty">
        No index series is held for this feed at {rung || 'any rung'} in {month || 'any month'}.
      </span>
    {:else}
      {#each indices as key (key)}
        {@const q = quotes.get(key)}
        <span class="tcell">
          <span class="c-n">{labelOf(key)}</span>
          {#if q?.close != null}
            <span class="c-v">{rupee(q.close)}</span>
            {#if q.chg != null}
              <span class="c-c" class:up={q.chg > 0} class:down={q.chg < 0}>
                {pctText(q.chg)}
              </span>
              <span class="c-a" class:up={q.chg > 0} class:down={q.chg < 0}>
                {q.chg >= 0 ? '↗' : '↘'}
              </span>
            {/if}
          {:else if q?.why}
            <span class="c-w" title={q.why}>—</span>
          {:else}
            <span class="c-w">·</span>
          {/if}
        </span>
      {/each}
    {/if}
  </div>

  <div class="body">
    <!-- ============================================================
         THE RAIL. A watchlist of what the store holds, not a watchlist
         somebody curated — there is no per-user list in this product and
         inventing one would be inventing a preference.
         ============================================================ -->
    <aside class="rail">
      <div class="tsearch">
        <input type="search" placeholder="Filter held instruments" bind:value={filter} />
      </div>

      <div class="wl">
        <div class="wl-h">
          <span class="wl-t">Held · {feeds.active ?? 'no feed'}</span>
          <span class="wl-c">{keys.length}</span>
        </div>

        <div class="wl-tabs">
          {#each rungs as r (r)}
            <button class="wl-tab" class:on={r === rung} onclick={() => (rung = r)}>{r}</button>
          {/each}
        </div>

        <div class="wl-list">
          {#if store.state === 'error'}
            <p class="tnote bad">The census could not be read. {store.error}</p>
          {:else if store.state !== 'ready'}
            <p class="tnote">Reading /store.json…</p>
          {:else if keys.length === 0}
            <p class="tnote">
              This feed’s store holds nothing. Nothing has been pulled into it —
              which is a different fact from a market with nothing to show.
            </p>
          {:else}
            {#each railKeys as key (key)}
              {@const q = quotes.get(key)}
              {@const p = parseKey(key)}
              <button
                class="wl-row"
                class:on={key === picked}
                onclick={() => (picked = key)}
              >
                <span class="wl-name">
                  {labelOf(key)}
                  <span class="wl-ex">{p.exchange} · {p.segment}</span>
                </span>
                <span class="wl-num">
                  {#if q?.close != null}
                    <span class="wl-ltp">{rupee(q.close)}</span>
                    {#if q.chg != null}
                      <span class="wl-chg" class:up={q.chg > 0} class:down={q.chg < 0}>
                        {pctText(q.chg)}
                      </span>
                    {/if}
                  {:else if q?.why}
                    <!-- A REAL ABSENCE: the store was asked and holds nothing. -->
                    <span class="wl-ltp dim" title={q.why}>—</span>
                  {:else if railQuoted.has(key)}
                    <!-- Asked for, not yet answered. -->
                    <span class="wl-ltp dim">·</span>
                  {:else}
                    <!-- NEVER ASKED, AND THE GLYPH SAYS SO. An em dash here
                         would be the same mark this page uses for "the store
                         holds nothing", and the two facts are not the same. -->
                    <span
                      class="wl-ltp dim"
                      title="No quote was requested for this row. A price is one request per series, so the terminal reads only the grid’s current page and the first {PAGE} rows of this rail. Filter to this instrument, or open its tab, to have it read."
                      >⋯</span
                    >
                  {/if}
                </span>
              </button>
            {/each}
          {/if}
        </div>
      </div>

      <div class="wl-f">
        <span>Name</span><span>LTP</span><span>LTP %</span>
      </div>
    </aside>

    <!-- ============================================================ THE GRID -->
    <main class="tgrid">
      <div class="ttabs">
        {#each tabs as t (t.id)}
          <button class="ttab" class:on={t.id === activeTab} onclick={() => (activeTab = t.id)}>
            {t.label}
          </button>
        {/each}
      </div>

      <div class="chiprow">
        <div class="chips">
          {#each chips as c, i (c)}
            <button class="chip" class:on={i === chipIndex} onclick={() => (chipIndex = i)}
              >{c}</button
            >
          {/each}
        </div>
        <div class="ctrls">
          <select class="pill" bind:value={month}>
            {#each months as m (m)}<option value={m}>{monthLabel(m)}</option>{/each}
          </select>
          <select class="pill" bind:value={rung}>
            {#each rungs as r (r)}<option value={r}>{r}</option>{/each}
          </select>
        </div>
      </div>

      <div class="panel">
        {#if tab?.why}
          <p class="tnote wide">{tab.why}</p>
        {:else if rows.length === 0}
          <p class="tnote wide">
            Nothing to show for {tab?.label} at {rung || 'no rung'} in
            {month ? monthLabel(month) : 'no month'}. The store holds no series
            matching all three.
          </p>
        {:else}
          <table>
            <thead>
              <tr>
                <th class="cb"><input type="checkbox" disabled /></th>
                {#each columns as c (c.h)}
                  <th class={c.align} title={c.src === null ? c.why : undefined}
                    >{c.h}{#if c.src === null}<span class="nosrc">*</span>{/if}</th
                  >
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each rows as key (key)}
                {@const q = quotes.get(key)}
                <tr>
                  <td class="cb"><input type="checkbox" /></td>
                  {#each columns as c (c.h)}
                    {#if c.src === 'name'}
                      <td class="nm">{labelOf(key)}</td>
                    {:else}
                      {@const cell = cellOf(q, c)}
                      <td
                        class={c.align}
                        class:up={cell.dir > 0}
                        class:down={cell.dir < 0}
                        class:dim={cell.text === '—' || cell.pending}
                        title={cell.why ?? undefined}
                      >
                        {cell.pending ? '·' : cell.text}
                      </td>
                    {/if}
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>

          {#if totalRows > rows.length}
            <button class="more" onclick={() => (shown += PAGE)}>
              Show {Math.min(PAGE, totalRows - rows.length)} more · {rows.length} of {totalRows}
              — quotes are fetched only for rows on screen
            </button>
          {/if}
        {/if}
      </div>

      <footer class="bbar">
        <span class="bnote">
          Every figure is read from this store through <code>/store.json</code> and
          <code>/bars/window.json</code>. A dash is an absence with a reason on hover,
          never a zero.
        </span>
        {#if columns.some((c) => c.src === null)}
          <span class="bkey"><span class="nosrc">*</span> no source in a bar store</span>
        {/if}
      </footer>
    </main>
  </div>
</div>

<style>
  /* ====================================================================
     THE PALETTE — MEASURED, NOT CHOSEN.
     --------------------------------------------------------------------
     Every value below was sampled from the source captures pixel by pixel:
     each screenshot cropped to 1x1 with `sips`, converted to a 32-bit BMP
     and read back as BGRA bytes. The method was calibrated against an
     independently sampled pixel and agreed to the byte.
     Two findings that an eye would have got wrong, and did:
       * the page ground is PURE BLACK, not a near-black neutral;
       * there are TWO greens. `--up` is the price-up lime and `--acc` is
         the active-state emerald. Using one for both is the tell.
     A plateau of 6-21 identical samples along a glyph stroke is what
     separates a fill from an antialiasing artefact; every signal colour
     here sat on such a plateau.
     ==================================================================== */
  .term {
    --ground: #000000;
    --bar: #0f0f0f;
    --panel: #121212;
    --head: #181818;
    --field: #1b1b1b;
    --line: #242424;
    --pill: #1b2c27;
    --ink: #dadada;
    --dim: #8e8e8e;
    --up: #5da81e;
    --down: #e15858;
    --acc: #06b878;

    /* Geometry, in the CSS pixels it was measured in. */
    --h-bar: 57px;
    --h-strip: 42px;
    --h-row: 42px;
    --w-rail: 396px;

    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: var(--h-bar) var(--h-strip) 1fr;
    background: var(--ground);
    color: var(--ink);
    font:
      14px/1.35 -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial,
      sans-serif;
    font-variant-numeric: tabular-nums;
    overflow: hidden;
  }

  /* ---- top bar ---- */
  .tbar {
    display: flex;
    align-items: center;
    gap: 15px;
    padding: 0 26px 0 28px;
    background: var(--bar);
    border-bottom: 1px solid var(--line);
  }
  .mark {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--acc);
    flex: none;
  }
  .tick {
    display: inline-flex;
    align-items: baseline;
    gap: 9px;
    white-space: nowrap;
  }
  .tick-n {
    font-size: 13px;
    color: var(--dim);
  }
  .tick-v {
    font-size: 15px;
    font-weight: 600;
  }
  .tick-c.up,
  .c-c.up,
  .c-a.up,
  .wl-chg.up,
  td.up {
    color: var(--up);
  }
  .tick-c.down,
  .c-c.down,
  .c-a.down,
  .wl-chg.down,
  td.down {
    color: var(--down);
  }
  .tnav {
    display: flex;
    gap: 22px;
    margin-left: 26px;
  }
  .tnav a {
    color: var(--ink);
    text-decoration: none;
    font-size: 15px;
    padding: 18px 0;
    position: relative;
  }
  .tnav a[aria-current='page'] {
    color: var(--acc);
  }
  .tnav a[aria-current='page']::after {
    content: '';
    position: absolute;
    left: -8px;
    right: -8px;
    bottom: 6px;
    height: 2px;
    background: var(--acc);
  }
  .tgap {
    flex: 1;
  }
  .closed {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--dim);
  }
  .cdot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--dim);
  }
  .back {
    color: var(--dim);
    text-decoration: none;
    font-size: 12px;
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 4px 9px;
  }
  .back:hover {
    color: var(--ink);
  }

  /* ---- index strip ---- */
  /* ---- index strip ----
     EVERY CLASS BELOW THAT COLLIDES WITH `theme.css` CARRIES A `t` PREFIX, AND
     THE PREFIX IS THE FIX RATHER THAN A STYLE CHOICE.

     Scoped styles raise specificity, so a property this page DECLARES always
     wins. A property it does not declare is inherited from the console's rule
     of the same name, and that is where the damage was. Measured against
     `theme.css`: `.search` (894) would have put a border, a background and
     padding on the WRAPPER around the input, boxing it twice; `.tabs` (1354)
     would have drawn a hairline under the tab group; `.grid` (962) would have
     made this column a second scroll container with `overflow: auto`.

     Nine of this page's names collided — cell, down, grid, search, strip, tab,
     tabs, tick, up. Renaming beats resetting: a reset has to enumerate every
     property the other rule happens to set today and silently rots when one is
     added to it, while a name the console does not use cannot collide at all.
     `up` and `down` keep their names deliberately: the only thing the console
     sets on them is `color`, this page declares `color` on every element that
     carries them, and the two vocabularies genuinely mean the same thing. */
  .tstrip {
    display: flex;
    align-items: center;
    gap: 56px;
    padding: 0 33px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
    overflow-x: auto;
    white-space: nowrap;
  }
  .tcell {
    display: inline-flex;
    align-items: baseline;
    gap: 10px;
    font-size: 13px;
  }
  .c-n {
    color: var(--dim);
  }
  .c-w {
    color: var(--dim);
  }
  .tstrip-empty {
    font-size: 13px;
    color: var(--dim);
  }

  /* ---- body split ---- */
  .body {
    display: grid;
    grid-template-columns: var(--w-rail) minmax(0, 1fr);
    min-height: 0;
  }

  /* ---- rail ---- */
  .rail {
    display: grid;
    grid-template-rows: auto 1fr auto;
    min-height: 0;
    padding: 12px;
    gap: 10px;
  }
  .tsearch input {
    width: 100%;
    height: 40px;
    background: var(--field);
    border: 1px solid var(--line);
    border-radius: 8px;
    color: var(--ink);
    padding: 0 12px;
    font: inherit;
  }
  .tsearch input::placeholder {
    color: var(--dim);
  }
  .wl {
    display: grid;
    grid-template-rows: auto auto 1fr;
    min-height: 0;
    background: var(--panel);
    border-radius: 8px;
    overflow: hidden;
  }
  .wl-h {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 11px 13px;
    font-size: 13px;
  }
  .wl-c {
    color: var(--dim);
  }
  .wl-tabs {
    display: flex;
    gap: 6px;
    padding: 0 13px 10px;
    flex-wrap: wrap;
  }
  .wl-tab {
    background: var(--ground);
    border: 1px solid var(--line);
    border-radius: 4px;
    color: var(--dim);
    font: inherit;
    font-size: 11px;
    padding: 3px 7px;
    cursor: pointer;
  }
  .wl-tab.on {
    color: var(--acc);
    border-color: var(--acc);
  }
  .wl-list {
    overflow-y: auto;
    min-height: 0;
  }
  .wl-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    background: none;
    border: 0;
    border-top: 1px solid var(--line);
    color: inherit;
    font: inherit;
    text-align: left;
    padding: 9px 13px;
    cursor: pointer;
  }
  .wl-row:hover,
  .wl-row.on {
    background: var(--head);
  }
  .wl-name {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .wl-ex {
    font-size: 11px;
    color: var(--dim);
  }
  .wl-num {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }
  .wl-chg {
    font-size: 11px;
  }
  .dim {
    color: var(--dim);
  }
  .wl-f {
    display: flex;
    gap: 18px;
    font-size: 11px;
    color: var(--dim);
    padding: 4px 4px 0;
  }

  /* ---- grid ---- */
  .tgrid {
    display: grid;
    grid-template-rows: auto auto 1fr auto;
    min-height: 0;
    padding: 12px 12px 0 0;
    gap: 0;
  }
  .ttabs {
    display: flex;
    gap: 2px;
    background: var(--panel);
    border-radius: 8px;
    padding: 4px;
    align-self: start;
  }
  .ttab {
    background: none;
    border: 0;
    color: var(--ink);
    font: inherit;
    font-size: 14px;
    padding: 8px 18px;
    border-radius: 6px;
    cursor: pointer;
  }
  .ttab.on {
    background: var(--pill);
    color: var(--acc);
  }
  .chiprow {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 16px 0 25px;
  }
  .chips {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    min-width: 0;
  }
  .chip {
    background: var(--ground);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--ink);
    font: inherit;
    font-size: 13px;
    padding: 7px 14px;
    white-space: nowrap;
    cursor: pointer;
  }
  .chip.on {
    color: var(--acc);
    border-color: var(--acc);
  }
  .ctrls {
    margin-left: auto;
    display: flex;
    gap: 8px;
  }
  .pill {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--ink);
    font: inherit;
    font-size: 13px;
    padding: 6px 10px;
  }
  .panel {
    background: var(--panel);
    border-radius: 8px 8px 0 0;
    overflow: auto;
    min-height: 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  thead th {
    position: sticky;
    top: 0;
    background: var(--head);
    color: var(--dim);
    font-weight: 400;
    height: var(--h-row);
    padding: 0 10px;
    text-align: right;
    white-space: nowrap;
    z-index: 1;
    /* RESET, NOT OMISSION — and the difference is why these three lines exist.
       `theme.css` styles the BARE `thead th` selector for the console's own
       grids: uppercase, tracked, bold, 12.5px, with a bottom rule. Those are
       the console's decisions and this page is not the console — the terminal
       it recreates writes its headers in title case at the body size.

       A scoped block only overrides what it DECLARES. Everything it leaves
       unsaid still comes from the global rule, so each of the console's
       choices has to be named to be undone. MEASURED in the browser before
       this block existed: the header rendered UPPERCASE at 12.5px, which is
       the console's table wearing the terminal's colours. */
    font-size: 13px;
    text-transform: none;
    letter-spacing: normal;
    border-bottom: 0;
  }
  thead th.left,
  td.nm {
    text-align: left;
  }
  tbody td {
    height: var(--h-row);
    padding: 0 10px;
    border-top: 1px solid var(--line);
    /* THE SAME RESET, FOR THE SAME REASON. `theme.css`'s `tbody td` carries a
       BOTTOM rule; this grid separates rows with a TOP one, and leaving the
       global in place drew both — a two-pixel band between every pair of rows
       that the source terminal does not have. */
    border-bottom: 0;
    text-align: right;
    white-space: nowrap;
  }
  /* The hover is declared here rather than inherited for the third time: the
     console's own `tbody tr:hover td` resolves to ITS token, which is a
     different colour on a page that does not use its ramp. */
  tbody tr:hover td {
    background: var(--head);
  }
  td.nm {
    color: var(--ink);
    border-right: 1px solid var(--line);
  }
  th.cb,
  td.cb {
    width: 44px;
    padding-left: 13px;
    text-align: left;
  }
  .nosrc {
    color: var(--dim);
    margin-left: 3px;
  }
  .more {
    display: block;
    width: 100%;
    background: var(--head);
    border: 0;
    border-top: 1px solid var(--line);
    color: var(--dim);
    font: inherit;
    font-size: 12px;
    padding: 12px;
    cursor: pointer;
  }
  .more:hover {
    color: var(--ink);
  }
  /* NAMED `tnote` AND NOT `note`, BECAUSE `note` IS ALREADY TAKEN AND NOT BY
     THIS PAGE. `theme.css:2198` gives the bare `.note` class the console's
     monospace face, its `--warn` colour, `display: flex` and a `::before`
     carrying a `▲` glyph. MEASURED on the served build: every empty state on
     this page rendered in monospace with a warning triangle in front of it —
     the console's caution note wearing the terminal's colours, on sentences
     that are not cautions.

     Renamed rather than overridden. Fighting a global with four resets leaves
     the next reader wondering which rule wins; a name the console does not use
     cannot collide at all. The same reasoning the `thead th` block above did
     NOT get to use, because an element selector has no name to change. */
  .tnote {
    color: var(--dim);
    font-size: 13px;
    padding: 14px;
    margin: 0;
    line-height: 1.5;
  }
  .tnote.bad {
    color: var(--down);
  }
  .tnote.wide {
    max-width: 62ch;
  }
  .bbar {
    display: flex;
    align-items: center;
    gap: 18px;
    height: 44px;
    font-size: 12px;
    color: var(--dim);
  }
  .bbar code {
    color: var(--ink);
  }

  /* MOTION IS ABSENT ON PURPOSE. Nothing on this page moves on its own, so
     there is no `prefers-reduced-motion` block to write: a terminal that
     animates a number invites a reader to believe it just changed, and every
     number here is a bar that was filed months ago. */
</style>
