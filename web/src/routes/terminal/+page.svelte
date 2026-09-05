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
   *     list of months, no list of exchanges. The strip, the tabs and
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
  import {
    tabsFrom,
    quotesOn,
    daysIn,
    timesOn,
    forgetQuotes,
    DAY_BUDGET
  } from '$lib/terminal.svelte.js';
  import { parseKey } from '$lib/instrument.js';
  import { group, rupee } from '$lib/money.js';
  import { monthLabel, dayLabel } from '$lib/dates.js';

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

  /* STOCKS OPENS, because it is the tab the engine surface is ABOUT: section 1
     names the 213 F&O cash equities as the thing being swept, and a future or
     an option is stored and never swept. */
  let activeTab = $state('stocks');
  const tab = $derived(tabs.find((t) => t.id === activeTab) ?? tabs[0]);


  /* ==================================================================
     THE TIMEFRAME AND THE MONTH — the two axes a bar file is filed under.
     --------------------------------------------------------------------
     `timeframe` AND NOT `rung`, THROUGHOUT. The engine calls these rungs and
     is right to — `EVERY_RUNG` is its own vocabulary for the ladder it sweeps.
     This page is not the engine. It reads `/store.json`, whose rows carry a
     field literally named `timeframe`, and it asks `/bars/window.json` with a
     parameter literally named `timeframe`. A page that renders `Rung` above a
     control bound to `timeframe` is carrying two words for one fact, and the
     operator is the one who has to hold both.
     Both are derived from the census, so the pickers can only ever offer
     something the store actually holds. A hardcoded default here is the
     defect `/db` recorded: a fixed span threw away 40 of 121 months and
     reported no hole while doing it.
     ================================================================== */
  const timeframes = $derived(
    [...new Set(store.readable.map((c) => c.timeframe))].sort(
      (a, b) => timeframeOrder(a) - timeframeOrder(b)
    )
  );
  const months = $derived([...store.byMonth.keys()].sort());

  /** Timeframe order by duration, so `2min` sorts before `10min`, not after it.
      @param {string} t @returns {number} */
  function timeframeOrder(t) {
    const m = String(t).match(/^(\d+)(min|day)$/);
    if (!m) return Number.MAX_SAFE_INTEGER;
    return Number(m[1]) * (m[2] === 'day' ? 1440 : 1);
  }

  /* THE YEAR IS A PICKER OF ITS OWN, AND IT IS FIRST, BECAUSE THIS IS A STORE
     OF HISTORY RATHER THAN A LIVE SESSION.
     --------------------------------------------------------------------
     The terminal this recreates has one month picker and needs no more: a
     live book only ever offers the near expiries, so `September / October /
     November` is the whole axis. This store is the opposite shape. It holds
     every month that has ever been pulled — 2020 through 2026 on a filled
     store, which is roughly eighty entries — and eighty options in one flat
     `<select>` is a list nobody can aim at.

     Splitting it turns one 80-item list into a 7-item list and a 12-item one,
     and the split is free because the census already carries the month key in
     `YYYY-MM`: the year is a prefix, not a second fact to store or derive.

     Both lists are DERIVED FROM THE CENSUS and never enumerated. A year with
     nothing pulled in it does not appear, so the picker cannot offer a window
     the store has no answer for — the defect `/db` recorded when a hardcoded
     span threw away 40 of 121 months and reported no hole while doing it. */
  const years = $derived([...new Set(months.map((m) => m.slice(0, 4)))].sort());

  let year = $state('');
  let timeframe = $state('');
  let month = $state('');
  let segment = $state('All');

  /** The months the census holds INSIDE the chosen year, ascending. */
  const monthsInYear = $derived(months.filter((m) => m.startsWith(`${year}-`)));

  /* THE PICKERS FOLLOW THE STORE RATHER THAN LEADING IT. When the census
     lands, or the feed changes under a chosen value, the current choice may no
     longer exist. Falling back to the newest year, the newest month within it
     and the coarsest timeframe the store holds is a choice this page can defend;
     keeping a value the store does not have would render an empty grid that
     reads as "no data" when it means "you asked for a month nobody pulled".

     The year clamps FIRST and the month clamps against `monthsInYear`, so
     changing the year cannot leave a month from the previous one selected —
     which would query a (timeframe, month) cell the year picker says is not in
     view. Two effects rather than one, because they answer to different
     inputs and collapsing them would re-run the year clamp on every month
     change. */
  $effect(() => {
    if (!years.includes(year)) year = years[years.length - 1] ?? '';
  });
  $effect(() => {
    if (!monthsInYear.includes(month)) month = monthsInYear[monthsInYear.length - 1] ?? '';
  });
  $effect(() => {
    if (!timeframes.includes(timeframe)) timeframe = timeframes[timeframes.length - 1] ?? '';
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
    void year;
    void month;
    void timeframe;
    void segment;
    shown = PAGE;
  });

  /* THE SEGMENTS PRESENT IN THIS TAB, AND ONLY THOSE.
     Derived from the parsed keys the tab already holds, so the picker offers
     `CASH` on a store that holds equities and does not on one that does not.
     `All` is prepended rather than stored, because "no filter" is not a
     segment and a store with one segment must not look like a choice.

     It is a SECOND axis to the tab, not a duplicate of it. Tabs cut by KIND —
     spot, future, option — and this cuts by the exchange segment a series is
     filed under. They coincide on Stocks (CASH) and Index (INDEX) and come
     apart on `Live`, which holds everything, and that is the tab where this
     picker earns its place. */
  const segments = $derived(
    [
      ...new Set(
        (tab?.keys ?? []).map((k) => parseKey(k).segment).filter((s) => typeof s === 'string')
      )
    ].sort()
  );

  /* A SEGMENT THE NEW TAB DOES NOT HAVE IS NOT CARRIED INTO IT. Leaving `FNO`
     selected on a tab holding only `INDEX` would empty the grid and blame the
     store for it. */
  $effect(() => {
    void activeTab;
    if (segment !== 'All' && !segments.includes(segment)) segment = 'All';
  });

  /** @param {string} key */
  const inChosenSegment = (key) => segment === 'All' || parseKey(key).segment === segment;

  const matching = $derived((tab?.keys ?? []).filter(inChosenSegment).filter(holdsChosenCell));
  const totalRows = $derived(matching.length);

  /* ==================================================================
     SORTING, AND THE ONE THING THAT MAKES IT DIFFERENT FROM DHAN'S
     ------------------------------------------------------------------
     The source terminal sorts a table it already holds — it is fed by a
     stream, so every row's price is in hand before anybody clicks a header.
     This page is not. A price is ONE REQUEST PER INSTRUMENT and the grid
     deliberately asks only for the rows on screen.

     So the two kinds of column are genuinely different:

       `Name` needs no quote. It sorts instantly, over the whole matching
       set, and is always exact.

       `LTP`, `Change`, `Change %`, `Open Interest` and `Volume` need every
       matching row QUOTED before they can be ordered. Sorting only what
       happens to be loaded would put "the highest volume among the first
       seventeen" under a heading that says Volume — a wrong answer wearing a
       right one's shape, which is the defect this page exists to avoid.

     Therefore a quote-backed sort widens the fetch to the whole matching set,
     and the set is bounded. 250 covers the engine surface — 213 F&O equities
     plus the indices — and refuses anything pathological rather than firing
     an unbounded number of requests at a click. The quotes are cached per
     (feed, key, timeframe, month), so the cost is paid once per window and
     every later sort of the same window is free.

     THE SORT ITSELF IS O(n log n) OVER A BOUNDED n, and that is not the O(1)
     rule's subject: `CLAUDE.md` §3 rule 4 governs the sweep's per-bar
     operations — bar lookup, condition lookup, mask evaluation, duplicate
     rejection, result append. This is a browser reordering at most 250
     strings after a human clicked something.
     ================================================================== */
  const SORT_BUDGET = 250;

  /** The column being sorted on: `'name'`, a Quote field, or `''` for none. */
  let sortKey = $state('');
  /** @type {'asc'|'desc'} */
  let sortDir = $state('desc');

  /* A SORT DOES NOT SURVIVE A CHANGE OF TAB. The columns differ between tabs,
     so a key sorted on Futures may not exist on Stocks, and a sort indicator
     pointing at a column that is no longer drawn is a control with no subject. */
  $effect(() => {
    void activeTab;
    sortKey = '';
    sortDir = 'desc';
  });

  /** Whether a quote-backed sort can be afforded for the current matching set. */
  const canSortByQuote = $derived(totalRows > 0 && totalRows <= SORT_BUDGET);

  /** Whether this column can be sorted at all. @param {Col} c */
  function sortableCol(c) {
    if (c.src === null) return false; // no source, nothing to order by
    if (c.src === 'name') return true; // free, always exact
    return canSortByQuote;
  }

  /**
   * Click a header: sort by it, or flip the direction if it is already the one.
   *
   * FIRST CLICK ON A NUMBER SORTS DESCENDING, on a name ASCENDING. That is what
   * the source terminal does and it is what the question means: "top by volume"
   * wants the largest first, "sort by name" wants A before Z.
   *
   * @param {string|null} key
   */
  function toggleSort(key) {
    if (key === null) return;
    if (sortKey === key) {
      sortDir = sortDir === 'desc' ? 'asc' : 'desc';
      return;
    }
    sortKey = key;
    sortDir = key === 'name' ? 'asc' : 'desc';
  }

  /** The value a row sorts by, or null when it has none. @param {string} key @param {string} on */
  function sortValue(key, on) {
    if (on === 'name') return labelOf(key);
    const q = quotes.get(key);
    if (!q) return null;
    if (on === 'chgAbs') return absMove(q);
    /* READ THROUGH `any` FOR THE SAME REASON `cellOf` DOES: the field name
       comes from the column table, so the lookup is a fact about that table
       rather than about the wire. */
    const v = /** @type {Record<string, number|null>} */ (/** @type {unknown} */ (q))[on];
    return typeof v === 'number' ? v : null;
  }

  /* A ROW WITH NO VALUE SORTS LAST IN BOTH DIRECTIONS, and never as zero.
     Treating an absent price as 0 would float every unquoted instrument to the
     top of an ascending sort and bury it in a descending one — the same
     zero-for-absent lie the dashes in the grid exist to refuse. */
  /* A SORT THAT CANNOT BE AFFORDED IS NOT APPLIED, AND THAT IS A CORRECTNESS
     RULE BEFORE IT IS A COST ONE.
     ------------------------------------------------------------------------
     Ordering by a quote field means READING `quotes`. `rows` comes from this
     sort, and the fetch effect depends on `rows` and assigns `quotes` a fresh
     Map. So while a quote sort is applied without the widened fetch, the graph
     closes on itself: sorted → rows → effect → quotes → sorted, forever.

     MEASURED, not theorised: with a quote column sorted and the matching set
     pushed to 300 — past the 250 budget — the page stopped responding
     entirely, and every later call into it timed out.

     It never fired while the sort was affordable, because the effect reads
     `matching` in that case and `matching` does not depend on the sort. Only
     the over-budget path completes the cycle, which is why it survived every
     earlier test: that path had never been walked.

     So the gate is here rather than only on the header: an unaffordable sort
     leaves the order untouched and reads no quote at all. */
  const sortActive = $derived(sortKey === 'name' || (sortKey !== '' && canSortByQuote));

  /* AND THE INDICATOR DOES NOT OUTLIVE ITS SORT. If the matching set grows past
     the budget — a wider segment, a fuller month — a ▼ left on a column the
     page is no longer ordering by would claim an order that is not there. */
  $effect(() => {
    if (sortKey !== '' && sortKey !== 'name' && !canSortByQuote) {
      sortKey = '';
      sortDir = 'desc';
    }
  });

  /* THE ACTIVE CHIP'S RANKING, read through a hoisted function rather than a
     binding. `CHIP_RULES` and `activeChip` are declared further down beside the
     chip row they belong to; a function defers the read until this derivation
     actually runs, which is after the whole script body has evaluated. Moving
     110 lines to satisfy declaration order would be the larger change and would
     put the chip's rules a screen away from the chips. */
  function chipRule() {
    return (activeChip && CHIP_RULES[activeChip]) || null;
  }

  /* THE CHIP FILTERS AND ORDERS; A CLICKED COLUMN ONLY ORDERS.
     ------------------------------------------------------------------------
     `Price Gainers` means two things — keep the ones that rose, put the biggest
     first — and only the first survives a column click. So a chip's FILTER is
     always applied and its ORDER is the default that an explicit sort replaces.
     Clicking `Volume` while `Price Gainers` is lit answers "the biggest volume
     AMONG the gainers", which is the question both controls together ask.

     A chip that ranks needs a value for every matching row, exactly as a
     column sort does, so it is gated on the same budget. Over it, the chip
     falls back to naming nothing rather than ranking the page against itself. */
  const sorted = $derived.by(() => {
    const rule = canSortByQuote ? chipRule() : null;

    let base = matching;
    if (rule?.keep) {
      const keep = rule.keep;
      base = base.filter((k) => {
        const v = sortValue(k, rule.by);
        return typeof v === 'number' && keep(v);
      });
    }

    const on = sortActive ? sortKey : rule ? rule.by : '';
    if (!on) return base;

    /* MAGNITUDE, NOT DIRECTION, when the chip asks for movers — and only then.
       An explicit column click always means the signed value, because the
       column shows the sign. */
    const useAbs = !sortActive && rule?.abs === true;
    const sign = (sortActive ? sortDir : (rule?.dir ?? 'desc')) === 'asc' ? 1 : -1;

    return [...base].sort((a, b) => {
      let va = sortValue(a, on);
      let vb = sortValue(b, on);
      if (useAbs && typeof va === 'number') va = Math.abs(va);
      if (useAbs && typeof vb === 'number') vb = Math.abs(vb);
      if (va === null && vb === null) return 0;
      if (va === null) return 1;
      if (vb === null) return -1;
      if (typeof va === 'string' || typeof vb === 'string') {
        return sign * String(va).localeCompare(String(vb));
      }
      return sign * (va - vb);
    });
  });

  const rows = $derived(sorted.slice(0, shown));

  /* THE POOL ASKS FOR THE GRID'S PAGE **AND THE INDEX STRIP**.
     The strip is the second consumer of quotes, and dropping the rail's head
     from this union dropped the strip with it: on the Options tab the indices
     are not in `rows`, so BANKNIFTY and NIFTY rendered their names with no
     level and no move beside them — MEASURED in the browser, and visible as
     two bare labels in a strip whose whole purpose is to carry numbers.

     Deduplicated, so an index that is also a grid row on the Stocks or Index
     view costs one request rather than two; and `indices` is at most six, so
     the union is bounded by the page size plus six however large the store
     grows. */

  /** Whether this instrument has a cell at the chosen (timeframe, month). O(1).
      @param {string} key @returns {boolean} */
  function holdsChosenCell(key) {
    if (!timeframe || !month) return false;
    return store.byCell.has(`${key}|${timeframe}|${month}`);
  }

  /** How many bars the census says this cell holds. O(1) — one Map probe.
      @param {string} key @returns {number} */
  const cellBars = (key) => store.byCell.get(`${key}|${timeframe}|${month}`) ?? 0;

  /* WHETHER A DAY CAN BE ASKED FOR AT ALL, DECIDED BEFORE ANYTHING IS FETCHED.
     ------------------------------------------------------------------------
     `/bars/window.json` takes a MONTH and has no day parameter, so answering
     "what did this do on the 12th" means reading the month and looking. There
     is no seek to a day, and multiplying a bars-per-day guess by a day index
     would be arithmetic on an assumption — section 3 rule 1.

     Reading a month costs what the month holds, and that spans three orders of
     magnitude: ~21 bars at `1day`, ~150 at `60min`, ~8,000 at `1min`. The
     census already carries that count per cell, so the decision is a Map probe
     and the fetch never starts on the wrong side of it.

     `heaviest` is carried so the refusal can NAME the number. A control that
     simply greys out teaches nothing; one that says "9,187 bars, over a budget
     of 512" tells the operator to coarsen the timeframe. */
  const heaviest = $derived(rows.reduce((most, key) => Math.max(most, cellBars(key)), 0));
  const dayCapable = $derived(rows.length > 0 && heaviest > 0 && heaviest <= DAY_BUDGET);

  /** The chosen day, `YYYY-MM-DD`, or `''` for the month's last bar. */
  let day = $state('');
  /* A DAY DOES NOT SURVIVE A CHANGE OF WINDOW. Every one of these changes which
     bars exist, and a date carried into a month that has no such day would
     empty the grid and blame the store for it. */
  $effect(() => {
    void activeTab;
    void year;
    void month;
    void timeframe;
    void segment;
    day = '';
  });

  /** The days the visible rows actually traded, ascending. */
  let days = $state(/** @type {string[]} */ ([]));

  $effect(() => {
    const feed = store.feed;
    const want = rows;
    const tf = timeframe;
    const mo = month;
    if (!feed || !tf || !mo || !dayCapable || want.length === 0) {
      days = [];
      return;
    }
    let live = true;
    /* THE UNION ACROSS THE VISIBLE ROWS, not one row's calendar. Instruments do
       not all trade the same days — a halt, a listing, a suspension — and
       taking the first row's days as the month's would hide every day the
       others traded and it did not. */
    Promise.all(want.map((key) => daysIn(feed, key, tf, mo))).then((lists) => {
      if (!live) return;
      days = [...new Set(lists.flat())].sort();
    });
    return () => {
      live = false;
    };
  });

  /** The chosen minute, `HH:MM` in IST, or `''` for the day's last bar. */
  let time = $state('');
  /* THE MINUTE DOES NOT SURVIVE A CHANGE OF DAY, and clearing the day clears it
     too — a time with no date is a minute of nothing. */
  $effect(() => {
    void day;
    time = '';
  });

  /** The minutes the visible rows are stamped with on the chosen day. */
  let times = $state(/** @type {string[]} */ ([]));

  $effect(() => {
    const feed = store.feed;
    const want = rows;
    const tf = timeframe;
    const mo = month;
    const on = day;
    if (!feed || !tf || !mo || !on || !dayCapable || want.length === 0) {
      times = [];
      return;
    }
    let live = true;
    /* THE UNION AGAIN, for the reason the day list gives: two instruments need
       not be stamped with the same minutes, and a series that halted at noon
       would erase the afternoon from a picker built on it alone. */
    Promise.all(want.map((key) => timesOn(feed, key, tf, mo, on))).then((lists) => {
      if (!live) return;
      times = [...new Set(lists.flat())].sort();
    });
    return () => {
      live = false;
    };
  });

  /** key -> Quote, filled as the pool answers. */
  let quotes = $state(new Map());

  $effect(() => {
    const feed = store.feed;
    /* A QUOTE-BACKED SORT WIDENS THE FETCH TO THE WHOLE MATCHING SET, because
       ordering rows by a value means having the value for every row. Without
       this the sort would rank the page against itself and label the result
       with the column's name. `Name` does not widen it — that ordering needs
       no quote at all — and neither does an unaffordable set, which is why
       `sortableCol` refuses those headers rather than letting the click
       through to a fetch nobody bounded. */
    const byQuote = sortKey !== '' && sortKey !== 'name';
    const byChip = chipRule() !== null;
    const needsAll = canSortByQuote && (byQuote || byChip);
    const want = [...new Set([...(needsAll ? matching : rows), ...indices])];
    const tf = timeframe;
    const mo = month;
    const on = day;
    const at = time;
    if (!feed || !tf || !mo || want.length === 0) return;
    let live = true;
    quotesOn(
      feed,
      want.map((key) => ({ key, timeframe: tf, month: mo })),
      on,
      at
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
    stocks: ['Intraday Movers', 'Outperformers', 'MTF', 'Extreme Openings', 'Price Movers', 'Breakouts', 'By Value'],
    futures: ['Premium', 'Discount', 'Top Volume', 'OI Gainers', 'OI Losers', 'Price Gainers', 'Price Losers'],
    options: ['Highest OI', 'OI Gainers', 'OI Losers', 'Top Volume', 'Top Value', 'Price Gainers', 'Price Losers']
  };
  const chips = $derived(CHIPS[activeTab] ?? []);

  /* ==================================================================
     WHAT A CHIP ACTUALLY DOES, AND WHICH ONES THIS STORE CAN DO AT ALL
     ------------------------------------------------------------------
     A chip is a RANKING: a filter and an order, applied to the matching set.
     `Price Gainers` is "the ones that rose, biggest first" — two operations,
     not a label.

     A quote carries `close`, `open`, `high`, `low`, `volume`, `oi` and `chg`.
     Six of the source terminal's chips fall straight out of those. The rest
     need something a bar file does not hold, and each is refused BY NAME
     rather than left inert:

       OI Gainers / OI Losers  need the CHANGE in open interest.
         `/bars/window.json` carries a change field for price and none for OI,
         so there is no second reading to difference against.

       Top Value / By Value    need turnover, which is price times quantity
         summed over the session. `close × volume` is a plausible-looking
         estimate and is not the same number — the `Value (Cr.)` column already
         refuses it for the same reason, and a chip that ranked on it would
         order the grid by a figure nobody measured.

       Premium / Discount      need the underlying's bar joined to the
         contract's at the same timestamp. This store files a contract's own
         series and makes no such join.

       Extreme Openings        need the gap: this bar's open against the
         PREVIOUS bar's close. A single bar cannot answer it.

       Breakouts               need a range over a trailing window, which is an
         indicator. `crates/indicators` owns those, and computing one here
         would be a second implementation of something the engine already has.

       Outperformers           need a benchmark join — each equity's move
         against the index's over the same window.

       MTF                     is vendor metadata about margin eligibility. No
         bar carries it and no endpoint here serves it.

     `abs` ranks by the SIZE of the move regardless of direction, which is what
     "movers" means and what separates it from "gainers".
     ================================================================== */

  /**
   * @typedef {object} ChipRule
   * @property {string} by     the Quote field, or `chg` for the move
   * @property {'asc'|'desc'} dir
   * @property {boolean} [abs] rank by magnitude, ignoring sign
   * @property {(v: number) => boolean} [keep] which rows the chip admits
   */

  /** @type {Record<string, ChipRule>} */
  const CHIP_RULES = {
    'Highest OI': { by: 'oi', dir: 'desc' },
    'Top Volume': { by: 'volume', dir: 'desc' },
    'Price Gainers': { by: 'chg', dir: 'desc', keep: (v) => v > 0 },
    'Price Losers': { by: 'chg', dir: 'asc', keep: (v) => v < 0 },
    'Intraday Movers': { by: 'chg', dir: 'desc', abs: true },
    'Price Movers': { by: 'chg', dir: 'desc', abs: true }
  };

  /** @type {Record<string, string>} */
  const CHIP_WHY = {
    'OI Gainers':
      'Ranking by a rise in open interest needs the CHANGE in open interest. /bars/window.json carries a change field for price and none for OI, so there is no second reading to difference against.',
    'OI Losers':
      'Ranking by a fall in open interest needs the CHANGE in open interest, and no endpoint here serves one — the same absence the OI Change column reports.',
    'Top Value':
      'Value is turnover: price times quantity, summed over the session. A bar carries close and volume but not per-trade value, and multiplying the two is an estimate rather than a measurement. The Value (Cr.) column refuses it for the same reason.',
    'By Value':
      'Value is turnover: price times quantity, summed over the session. A bar carries close and volume but not per-trade value, and ranking on their product would order this grid by a figure nobody measured.',
    Premium:
      'A future’s premium is its price minus its underlying’s at the same timestamp. This store files a contract’s own series and makes no join to the underlying, so there is nothing to rank.',
    Discount:
      'A future’s discount is its underlying’s price minus its own at the same timestamp. That join does not exist in this store.',
    'Extreme Openings':
      'An opening gap is this bar’s open against the PREVIOUS bar’s close. One bar cannot answer it, and /bars/window.json returns the window asked for rather than the bar before it.',
    Breakouts:
      'A breakout is a close against a trailing range, which is an indicator. crates/indicators owns those, and computing one in the browser would be a second implementation of something the engine already holds.',
    Outperformers:
      'Outperformance is an instrument’s move against a benchmark’s over the same window. This page reads each series independently and makes no benchmark join.',
    MTF: 'Margin trading eligibility is vendor metadata about an instrument, not a property of its bars. No endpoint here serves it.'
  };

  /* THE CHIP IS SELECTABLE, BECAUSE IT CHANGES THE GRID.
     It was drawn lit-but-inert on first build, which was defensible only while
     the column set depended on the tab alone. It does not: see `columnsFor`.
     A chip that reflows the table cannot be decoration.
     The selection resets with the tab, because a chip belongs to its tab —
     `Highest OI` has no meaning on Stocks and index 4 on one strip names a
     different filter on the next. */
  let chipIndex = $state(0);
  /* THE TAB OPENS ON A CHIP THAT CAN ACTUALLY RANK, not on index 0.
     Futures leads with `Premium`, which needs a join this store does not make,
     so opening there would light a chip that orders nothing and leave the
     reader to wonder whether the ranking was applied. The first chip carrying a
     rule is the one that opens; if a tab has none, index 0 stands and the
     refusal on it explains itself. */
  $effect(() => {
    void activeTab;
    const list = CHIPS[activeTab] ?? [];
    const first = list.findIndex((c) => CHIP_RULES[c]);
    chipIndex = first === -1 ? 0 : first;
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
    <!-- `/markets` AND NOT `/`. The root redirects here, so a Console link
         pointing at it would send the reader back to this page — a button that
         appears to leave and does not. -->
    <a class="back" href="/markets">Console</a>
  </header>

  <!-- THE INDEX STRIP. Whatever indices this store holds, and no more. -->
  <div class="tstrip">
    {#if indices.length === 0}
      <span class="tstrip-empty">
        No index series is held for this feed at {timeframe || 'any timeframe'} in {month || 'any month'}.
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
    <!-- ============================================================ THE GRID
         THE WATCHLIST RAIL WAS REMOVED, ON THE OPERATOR'S INSTRUCTION, AND THE
         GRID TOOK ITS 396px.

         It listed every instrument the store held, with its last close and its
         move — which is what the grid already is, one tab at a time. Two
         surfaces answering one question, and the narrower of them had to
         explain in a paragraph why it was empty. The removal is not only
         subtraction: every column right of `Name` gains a third of the window,
         which is the width the source terminal's own captures were measured at.
         ============================================================ -->
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
          <!-- A CHIP THIS STORE CANNOT RANK IS DISABLED AND SAYS WHY, rather
               than lighting up and ordering nothing. An inert control that
               looks live is the failure section 4 names; one that refuses and
               names the missing field is a fact the reader can act on. -->
          {#each chips as c, i (c)}
            <button
              class="chip"
              class:on={i === chipIndex}
              disabled={!CHIP_RULES[c]}
              title={CHIP_RULES[c]
                ? canSortByQuote
                  ? `Rank by ${c.toLowerCase()}.`
                  : `Ranking needs a price for every matching row, and this view holds ${totalRows.toLocaleString('en-IN')} against a budget of ${SORT_BUDGET.toLocaleString('en-IN')}.`
                : CHIP_WHY[c]}
              onclick={() => CHIP_RULES[c] && (chipIndex = i)}>{c}</button
            >
          {/each}
        </div>
        <!-- THE WINDOW, WIDEST FIRST: year, then month, then segment, then
             timeframe. The order is the order the answers depend on each other —
             the month list is the chosen year's, and the segment list is the
             chosen tab's — so reading left to right is reading the query being
             narrowed. Each is labelled, because four bare pills side by side
             name nothing: `2026`, `Sep 2026`, `CASH` and `1day` are four
             different KINDS of value and only the last two are self-evident. -->
        <div class="ctrls">
          <!-- THE FEED, AND THIS PAGE HAD NONE — A DEAD END THIS PAGE CREATED.
               `+layout.svelte`'s FEED_OWNED comment states the rule it broke:
               "ZERO controls is the other, and it is worse: a page that shows
               one feed's answer and offers no way to change it is a dead end,
               and the operator's only move is the browser's back button." That
               is exactly what standing the console bar down did here — the bar
               carried the only picker, and this page replaced the bar without
               replacing the control.

               So it joins FEED_OWNED's list in substance: the feed is the FIRST
               control of this row because it is the first rung of the cascade,
               and everything to its right is that feed's answer. The census,
               the years, the months, the segments and the timeframes are all
               folds over the store OF THE CHOSEN FEED.

               AN UNAVAILABLE FEED IS NAMED, NOT HIDDEN — the same reason the
               bar used a listbox rather than a native `<select>`: an option has
               to carry the server's REASON when a feed cannot be used, and a
               feed dropped from the list reads as a feed that does not exist. -->
          <label class="tctl">
            <span>Feed</span>
            <select class="pill" bind:value={feeds.active}>
              {#each feeds.all as f (f.wire)}
                <option value={f.wire} disabled={!f.ready} title={f.why ?? undefined}>
                  {f.display}{f.ready ? '' : ' · unavailable'}
                </option>
              {/each}
            </select>
          </label>
          <label class="tctl">
            <span>Year</span>
            <select class="pill" bind:value={year}>
              {#each years as y (y)}<option value={y}>{y}</option>{/each}
            </select>
          </label>
          <label class="tctl">
            <span>Month</span>
            <select class="pill" bind:value={month}>
              {#each monthsInYear as m (m)}<option value={m}>{monthLabel(m)}</option>{/each}
            </select>
          </label>
          <!-- THE DAY. Present only when the census says the month can be read
               within budget, and carrying the reason when it cannot — a control
               that greys out silently teaches nothing, while one that names the
               bar count tells the operator to coarsen the timeframe. -->
          <label class="tctl">
            <span>Date</span>
            <select
              class="pill"
              bind:value={day}
              disabled={!dayCapable || days.length === 0}
              title={dayCapable
                ? 'The last bar on the chosen day. “Month” is the month’s last bar.'
                : `A day needs the month read, and the heaviest visible series holds ${heaviest.toLocaleString('en-IN')} ${timeframe} bars against a budget of ${DAY_BUDGET.toLocaleString('en-IN')}. Choose a coarser timeframe to pick a date.`}
            >
              <option value="">Month</option>
              {#each days as d (d)}<option value={d}>{dayLabel(d)}</option>{/each}
            </select>
          </label>
          <!-- THE MINUTE. Only meaningful once a day is chosen, and only
               offers a choice when the timeframe puts more than one bar in a
               day: `1day` yields exactly one, and a picker with one option is
               a control that cannot be used. It says which of those it is
               rather than greying out for an unstated reason. -->
          <label class="tctl">
            <span>Time</span>
            <select
              class="pill"
              bind:value={time}
              disabled={!day || times.length < 2}
              title={!day
                ? 'Choose a date first — a time with no date is a minute of nothing.'
                : times.length < 2
                  ? `At ${timeframe} a day holds ${times.length === 1 ? 'one bar' : 'no bars'}, so there is no minute to choose between. A finer timeframe puts more bars in the day.`
                  : 'The bar stamped at this minute. “Close” is the day’s last bar.'}
            >
              <option value="">Close</option>
              {#each times as t (t)}<option value={t}>{t}</option>{/each}
            </select>
          </label>
          <label class="tctl">
            <span>Segment</span>
            <select class="pill" bind:value={segment}>
              <option value="All">All</option>
              {#each segments as s (s)}<option value={s}>{s}</option>{/each}
            </select>
          </label>
          <label class="tctl">
            <span>Timeframe</span>
            <select class="pill" bind:value={timeframe}>
              {#each timeframes as r (r)}<option value={r}>{r}</option>{/each}
            </select>
          </label>
        </div>
      </div>

      <div class="panel">
        {#if tab?.why}
          <p class="tnote wide">{tab.why}</p>
        {:else if rows.length === 0}
          <p class="tnote wide">
            Nothing to show for {tab?.label} at {timeframe || 'no timeframe'} in
            {month ? monthLabel(month) : 'no month'}. The store holds no series
            matching all three.
          </p>
        {:else}
          <table>
            <thead>
              <tr>
                <th class="cb"><input type="checkbox" disabled /></th>
                {#each columns as c (c.h)}
                  <!-- A SORTABLE HEADER IS A CONTROL AND IS REACHABLE AS ONE.
                       `tabindex` and the key handler only appear when the
                       column can actually be sorted, so a keyboard lands on
                       nothing that does not respond. `aria-sort` carries the
                       direction to a screen reader, which the ▲▼ glyph alone
                       does not. -->
                  <th
                    class={c.align}
                    class:sortable={sortableCol(c)}
                    class:sorted={sortKey !== '' && sortKey === c.src}
                    aria-sort={sortKey !== '' && sortKey === c.src
                      ? sortDir === 'asc'
                        ? 'ascending'
                        : 'descending'
                      : undefined}
                    tabindex={sortableCol(c) ? 0 : undefined}
                    title={c.src === null
                      ? c.why
                      : sortableCol(c)
                        ? `Sort by ${c.h}`
                        : `Sorting by ${c.h} needs a price for every matching row, and this view holds ${totalRows.toLocaleString('en-IN')} of them against a budget of ${SORT_BUDGET.toLocaleString('en-IN')}. Narrow the segment, or sort by Name, which needs no price.`}
                    onclick={() => sortableCol(c) && toggleSort(c.src)}
                    onkeydown={(e) => {
                      if (!sortableCol(c)) return;
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        toggleSort(c.src);
                      }
                    }}
                    >{c.h}{#if c.src === null}<span class="nosrc">*</span>{/if}{#if sortKey !== '' && sortKey === c.src}<span
                        class="sortmark">{sortDir === 'asc' ? '▲' : '▼'}</span
                      >{/if}</th
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

          {#if sorted.length > rows.length}
            <!-- `sorted.length` AND NOT `totalRows`. A chip that filters —
                 `Price Gainers` keeps only the ones that rose — makes the set
                 on screen smaller than the set that matched, and a pager
                 counting the unfiltered total would promise rows the filter has
                 already removed. `totalRows` remains what the BUDGET is
                 measured against, which is a different question. -->
            <button class="more" onclick={() => (shown += PAGE)}>
              Show {Math.min(PAGE, sorted.length - rows.length)} more · {rows.length} of {sorted.length}
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

    /* DARK UNCONDITIONALLY, AND IT HAS TO SAY SO ITSELF.
       `theme.css` puts `color-scheme: light` on the bare `:root` and flips it
       only inside its dark blocks, so on a machine set to light with the
       console's toggle on `auto` the whole document resolves to light. That is
       right for the console and wrong here: this page has no light variant —
       every surface above is a measured near-black — and `color-scheme` is
       what the PLATFORM reads to paint the things CSS cannot reach.

       MEASURED before this line: the four `<select>` pickers rendered as white
       rounded boxes on a pure-black terminal while their computed
       `background` was #121212, their `border` #242424 and their `outline`
       `none`. Nothing in the cascade was white; the OS was painting the
       control. `appearance: none` suppresses the fill and not the ring, and it
       does nothing at all for the open option list or the scrollbars. */
    color-scheme: dark;

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
  td.up {
    color: var(--up);
  }
  .tick-c.down,
  .c-c.down,
  .c-a.down,
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

  /* ---- body ----
     ONE COLUMN. It was `var(--w-rail) minmax(0, 1fr)` while the watchlist rail
     stood on the left; with the rail gone the grid is the only child and takes
     the window. `minmax(0, 1fr)` and not `1fr`, for the reason `.tgrid` states
     at length below: a `fr` track still floors at its item's automatic minimum,
     and this column holds a table that is deliberately wider than the screen. */
  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-height: 0;
  }

  .dim {
    color: var(--dim);
  }

  /* ---- grid ---- */
  .tgrid {
    display: grid;
    /* `minmax(0, 1fr)` AND NOT AN IMPLIED AUTO COLUMN. With only rows declared
       this grid took one AUTO column, which sizes to its widest content — the
       chip row — and grew past its container, pushing the pickers off the
       right edge and scrolling the whole document sideways. MEASURED at 800px:
       the controls ended at x 1139 inside a grid that ended at 828.

       `1fr` alone is not the fix; a `fr` track still floors at the item's
       automatic minimum. `minmax(0, 1fr)` is what states the floor is zero,
       which is what lets the column be the container and hands the overflow to
       the chip row, where `overflow-x: auto` deals with it. `theme.css`
       records the identical lesson on `.shell`. */
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto auto 1fr auto;
    min-height: 0;
    /* A LEFT INSET OF ITS OWN, WHICH THE RAIL USED TO PROVIDE. This read
       `12px 12px 0 0` — no left padding — because a 396px watchlist stood
       there. With the rail gone the grid starts at the window edge, and a
       table flush against the glass reads as clipped rather than as wide. */
    padding: 12px;
    padding-bottom: 0;
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
    /* The row itself must be allowed to be narrower than its content, or the
       `overflow-x: auto` on `.chips` below never engages — a flex item's
       automatic minimum is its content, and that is what was widening the
       grid before the column above was pinned. */
    min-width: 0;
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
  /* A REFUSED CHIP READS AS REFUSED BEFORE IT IS CLICKED. Dimmer text, a
     dashed edge and the default cursor, so the difference between "this ranks"
     and "this store cannot rank this" is visible rather than discovered. */
  .chip:disabled {
    color: #5a5a5a;
    border-style: dashed;
    border-color: #2a2a2a;
    cursor: default;
  }
  .chip.on {
    color: var(--acc);
    border-color: var(--acc);
  }
  .ctrls {
    margin-left: auto;
    display: flex;
    gap: 10px;
    align-items: center;
    /* NOT SHRINKABLE. The chip row beside this is the thing that yields — it
       is a list and a list can scroll. These four are the window the grid is
       showing, and a picker sliced off the right edge answers "which month am
       I looking at" with silence. MEASURED at an 800px viewport: `Year` was
       half-drawn and `Month` was a truncated word. */
    flex: none;
  }
  /* The label sits ABOVE its control rather than beside it: four side-by-side
     pairs would eat the width the chip row needs, and the chip row is the
     thing that scrolls. */
  /* `tctl`, NOT `ctl` — AND THIS ONE GOT THROUGH THE AUDIT THAT WAS RUN TO
     CATCH EXACTLY IT. `theme.css:2166` gives `.ctl` a `--bg-2` fill and a
     `--line-hard` border, which in the light ramp is a WHITE box with a light
     rule. Four of them appeared around the pickers on a pure-black terminal.

     The audit that renamed nine other colliding classes was run BEFORE this
     control existed, so it could not have seen it. That is the actual lesson
     and it is not "audit once": every class added to this page after the fact
     has to be checked against `theme.css` again, because the collision is a
     property of the NAME and names are added one at a time. Prefixing new
     names on sight is the cheaper habit. */
  .tctl {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .tctl > span {
    font-size: 10px;
    color: var(--dim);
    letter-spacing: 0.02em;
  }
  /* `appearance: none` IS NOT COSMETIC HERE. A bare `<select>` on macOS paints
     the platform's own control -- a light rounded box with a blue-tinted
     gradient and its own caret -- and it ignores `background` and `border`
     while doing it. MEASURED on the running page: four light pills sitting in
     a pure-black terminal, the single most obviously wrong thing on it. The
     caret has to be redrawn for the same reason: switching the appearance off
     removes the platform's, and a dropdown with no caret does not read as one.
     Drawn as a data URI rather than a pseudo-element because a `<select>` has
     no `::after` to hang one on. */
  .pill {
    appearance: none;
    -webkit-appearance: none;
    background: var(--panel)
      url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="10" height="6" viewBox="0 0 10 6"><path d="M1 1l4 4 4-4" fill="none" stroke="%238E8E8E" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>')
      no-repeat right 9px center;
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--ink);
    font: inherit;
    font-size: 13px;
    padding: 6px 26px 6px 10px;
    cursor: pointer;
    /* A `<select>` SIZES TO ITS WIDEST OPTION, WHICH IS NOT A LAYOUT DECISION.
       MEASURED at 1714px: the Feed control came out 221px wide beside siblings
       of 65, 68, 71 and 98, because one option reads "Global Datafeeds ·
       unavailable". The row then reads as lopsided — the widest control is the
       one whose value is shortest — and the eye takes that as misalignment
       rather than as a long word.

       A floor and a ceiling: the floor keeps `2026` and `All` from collapsing
       into stubs so the five read as one set, and the ceiling stops the longest
       option deciding the row. The full text stays in the option itself and in
       its `title`, so nothing is lost — only the box stops growing. */
    min-width: 78px;
    max-width: 150px;
  }
  .pill:hover {
    border-color: #3a3a3a;
  }
  /* The OPEN menu is the platform's and cannot be styled, so the options are
     told to paint dark rather than left to inherit a white list. */
  .pill option {
    background: var(--panel);
    color: var(--ink);
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
  /* THE NO-SOURCE MARK COSTS AS LITTLE WIDTH AS IT CAN.
     It is this page's addition, not the source terminal's, and it sat inline at
     full size on three of nine headers — `Spot Price*`, `OI Change*`, `OI
     Change %*` — widening exactly those columns and pushing every column right
     of them out of step with the captures. Superscripted at 9px it still reads
     as a footnote mark and stops moving the grid. */
  /* A HEADER THAT SORTS LOOKS LIKE ONE ONLY WHEN IT CAN.
     The cursor and the hover are the affordance; a column with no source, or
     one whose set is too large to quote, keeps the default cursor and stays
     dim, so the difference is visible before the click rather than after it. */
  th.sortable {
    cursor: pointer;
    user-select: none;
  }
  th.sortable:hover {
    color: var(--ink);
  }
  th.sorted {
    color: var(--ink);
  }
  th.sortable:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: -2px;
  }
  /* THE MARK DOES NOT MOVE THE COLUMN. Absolutely the same problem the
     no-source asterisk had: an inline glyph appearing on click would widen its
     header and shift every column right of it, so the grid would twitch each
     time the sort changed. Reserved space, not added space. */
  .sortmark {
    display: inline-block;
    width: 0;
    margin-left: 4px;
    font-size: 8px;
    color: var(--acc);
    vertical-align: middle;
  }
  .nosrc {
    color: var(--dim);
    margin-left: 2px;
    font-size: 9px;
    vertical-align: super;
    line-height: 0;
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

  /* ---- narrow windows ---------------------------------------------------
     THE TWO STEPS THAT SHRANK AND THEN HID THE RAIL ARE GONE WITH IT.

     What remains is the one thing a narrow window still needs: the grid's own
     left inset, which the rail used to provide. The table itself does not need
     a breakpoint — it is wider than the screen by design at every width, and
     `.panel` scrolls it inside its own box rather than scrolling the page. */
  @media (max-width: 900px) {
    .tgrid {
      padding-left: 12px;
    }
  }

  /* MOTION IS ABSENT ON PURPOSE. Nothing on this page moves on its own, so
     there is no `prefers-reduced-motion` block to write: a terminal that
     animates a number invites a reader to believe it just changed, and every
     number here is a bar that was filed months ago. */
</style>
