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
   *     `quotesOn`.
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
  import { parseKey, strikeExact } from '$lib/instrument.js';
  import { group, rupee } from '$lib/money.js';
  import { monthLabel, dayLabel, MON } from '$lib/dates.js';

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

  /* THE EXPIRY, WHICH ONLY TWO TABS HAVE.
     ------------------------------------------------------------------------
     The source puts a `September ▾` beside the segment picker on Futures and
     Options and omits it on Stocks — a cash equity has no expiry to choose.
     Its capture opens it to September · October · November, three consecutive
     contract months.

     DERIVED FROM THE STORE, NOT FROM A CALENDAR. Listing the next three months
     would offer expiries no contract in this store has, and picking one would
     empty the grid while the picker insisted the expiry existed. The list is
     the set of expiries actually present on the tab's own keys, so every option
     has at least one contract behind it — the same rule the Date picker keeps
     about days.

     GROUPED TO THE MONTH, because that is the unit the source names and the
     unit a contract series is identified by. `2026-09` sorts chronologically as
     a plain string for the reason `$lib/dates.js` opens with, and is rendered
     through `monthLabel` at the render site and nowhere else. */
  const expiries = $derived(
    [
      ...new Set(
        (tab?.keys ?? [])
          .map((k) => parseKey(k).expiry)
          .filter((e) => typeof e === 'string' && /^\d{4}-\d{2}/.test(e))
          .map((e) => String(e).slice(0, 7))
      )
    ].sort()
  );
  /** Cash equities have no expiry; the control is absent rather than empty. */
  const hasExpiry = $derived(activeTab === 'futures' || activeTab === 'options');
  let expiry = $state('');
  /** @param {string} key */
  const inChosenExpiry = (key) => {
    if (!expiry) return true;
    const e = parseKey(key).expiry;
    return typeof e === 'string' && e.slice(0, 7) === expiry;
  };

  /* A CHOSEN EXPIRY DOES NOT SURVIVE THE TAB OR THE FEED THAT OFFERED IT, for
     the reason every other clamp on this page exists: an expiry no longer on
     offer filters the grid to nothing while the control still displays it, and
     the empty state then blames the store for the page's own stale value. */
  $effect(() => {
    void activeTab;
    void store.feed;
    expiry = '';
  });
  $effect(() => {
    if (expiry && expiries.length > 0 && !expiries.includes(expiry)) expiry = '';
  });

  const matching = $derived(
    (tab?.keys ?? []).filter(inChosenSegment).filter(inChosenExpiry).filter(holdsChosenCell)
  );
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

  /* GAINERS OR LOSERS — the Stocks tab's own filter, and it is BINARY because
     the source's is.
     ------------------------------------------------------------------------
     Both Stocks captures show `Gainers ●—○ Losers` at the right of the chip row
     with one side always chosen; there is no neutral position, so the tab shows
     one half of the movers and the operator undoes that in one click.

     It FILTERS BY QUOTE, so it costs exactly what a ranking chip costs and is
     gated by the same budget: below `SORT_BUDGET` it applies and widens the
     fetch to the whole matching set; above it the control is disabled and names
     the reason rather than filtering against prices it does not hold.

     Stocks only — neither the Futures nor the Options capture carries it; they
     put the expiry picker in that space instead.

     DECLARED HERE, above `quotesWidened`, because that derived reads `moverTab`
     and a `$derived` referencing a later `$derived` is a temporal-dead-zone
     error rather than a hoisting convenience. */
  let mover = $state(/** @type {'gainers'|'losers'} */ ('gainers'));
  const moverTab = $derived(activeTab === 'stocks');
  const moverActive = $derived(moverTab && canSortByQuote);

  /* DOES THIS VIEW QUOTE THE WHOLE MATCHING SET, OR ONLY THE ROWS ON SCREEN?
     ------------------------------------------------------------------------
     A quote-backed sort widens the fetch to the whole matching set, because
     ordering rows by a value means having the value for every row — otherwise
     the sort ranks the page against itself and labels the result with the
     column's name. A ranking chip widens it for the same reason. `Name` does
     NOT: that ordering reads no quote at all. Neither does an unaffordable set,
     which is why `sortableCol` refuses those headers rather than letting the
     click through to a fetch nobody bounded.

     ONE SPELLING, BECAUSE THERE WERE TWO AND THEY DISAGREED. The fetch tested
     `sortKey !== '' && sortKey !== 'name'`; the pager caption tested
     `sortKey !== ''` and so counted a Name sort as widening. Sorting by Name
     therefore fetched only the rows on screen while the caption underneath
     announced "every matching row is quoted, because the order depends on it"
     — a claim about cost, made about an ordering that reads no quote, and one
     the reader has no way to check. The predicate lives here now and both
     sites read it. */
  const quotesWidened = $derived(
    canSortByQuote &&
      ((sortKey !== '' && sortKey !== 'name') || chipRule() !== null || moverTab)
  );

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
      /* A VALUE THAT IS NOT KNOWN YET DOES NOT FAIL THE FILTER.
         ---------------------------------------------------------------------
         This read `typeof v === 'number' && keep(v)`, which treats "no quote
         yet" as "does not pass" — and quotes are cleared on every change of
         feed, timeframe, month, date or time. So with a filtering chip lit,
         changing the month emptied the grid outright for the length of the
         refetch, and the empty state then announced "The store holds no series
         matching all three" about a store that demonstrably held them.
         MEASURED: Options, `Price Gainers`, three held contracts — 3 rows, then
         0 rows and that sentence, then 3 rows again.

         Unknown is not false. It is the same rule the dashes keep in the grid
         and the same one `sortValue` keeps for ordering, where null sorts last
         rather than as zero. A row with no value yet is CARRIED, renders its
         pending dot, and is filtered on the next pass once its quote lands. */
      base = base.filter((k) => {
        const v = sortValue(k, rule.by);
        if (v === null) return true;
        return typeof v === 'number' && keep(v);
      });
    }

    /* THE GAINERS/LOSERS SWITCH, filtering on the same terms as the chip above.
       An unknown change is CARRIED, not dropped — the paragraph above gives the
       reason and it applies here identically: quotes are cleared on every change
       of feed, timeframe, month, date or time, so treating "no quote yet" as
       "not a gainer" would empty the grid for the length of every refetch and
       then blame the store for it.
       Zero is neither: a bar that closed exactly where the previous one closed
       has not gained and has not lost, and `chg` is integer basis points so zero
       also covers a real move under half a basis point. It is excluded from both
       sides rather than assigned to one. */
    if (moverActive) {
      base = base.filter((k) => {
        const v = sortValue(k, 'chg');
        if (v === null) return true;
        if (typeof v !== 'number') return false;
        return mover === 'gainers' ? v > 0 : v < 0;
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
  /* THE DAY SCOPE IS FOLDED OVER `matching`, NOT OVER `rows`, AND THAT IS A
     CORRECTNESS FIX RATHER THAN A TIDY-UP.
     ------------------------------------------------------------------------
     `rows` comes from `sorted`, `sorted` reads `quotes`, and the quotes-
     clearing effect depends on `day`. So folding the day list over `rows` made
     the LIST depend on the SELECTION: choosing a date emptied the quote map,
     which flattened the ranking to census order, which changed which
     seventeen instruments were visible, which refolded `days` over a different
     set — and the clamp below then cleared the date that had just been picked.
     Nothing restored it when the quotes landed a round trip later.

     The same loop moved `heaviest` and `dayCapable`, so a transient flip could
     disable the Date control while the day it no longer showed was still being
     applied to every quote on the page.

     `matching` is the tab's keys filtered by segment and by the chosen cell.
     It reads no quote, so it cannot move because of a selection — the list is
     a property of the WINDOW, which is what a date picker's options should be.
     Capped at the page size for the same reason the grid is: this is a fold
     that costs one month read per key. */
  const dayScope = $derived(matching.slice(0, PAGE));
  const heaviest = $derived(dayScope.reduce((most, key) => Math.max(most, cellBars(key)), 0));
  const dayCapable = $derived(dayScope.length > 0 && heaviest > 0 && heaviest <= DAY_BUDGET);

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
  /** The reason at least one series in view could not be read. */
  let daysWhy = $state(/** @type {string|null} */ (null));
  /** How many of the series in view failed to read, against how many were asked.
      `daysWhy` alone cannot say whether the list below it is EMPTY or merely
      SHORT, and those want opposite sentences. */
  let daysFailed = $state(0);
  let daysAsked = $state(0);

  $effect(() => {
    const feed = store.feed;
    const want = dayScope;
    const tf = timeframe;
    const mo = month;
    if (!feed || !tf || !mo || !dayCapable || want.length === 0) {
      days = [];
      daysWhy = null;
      daysFailed = 0;
      daysAsked = 0;
      return;
    }
    let live = true;
    /* CLEARED BEFORE THE READ, NOT AFTER IT.
       This list survived until the new answers landed, so between changing the
       month and the fetch resolving the picker went on offering the PREVIOUS
       month's days — dates that are not in the month now selected. Picking one
       in that window asks for a day the month does not contain and gets the
       absence message for it, which blames the store for a list this page had
       already replaced. An empty list for the width of a fetch is a control
       that is briefly not ready; a wrong list is one that is confidently
       wrong. */
    days = [];
    daysWhy = null;
    daysFailed = 0;
    daysAsked = 0;
    /* THE UNION ACROSS THE VISIBLE ROWS, not one row's calendar. Instruments do
       not all trade the same days — a halt, a listing, a suspension — and
       taking the first row's days as the month's would hide every day the
       others traded and it did not. */
    Promise.all(want.map((key) => daysIn(feed, key, tf, mo))).then((answers) => {
      if (!live) return;
      days = [...new Set(answers.flatMap((a) => a.days))].sort();
      /* A FAILED READ IS NOT AN EMPTY MONTH. `daysIn` now carries the reason
         the month could not be read, and without it a transport error looked
         exactly like a month that holds no days — a silently disabled Date
         picker with nothing to explain it.
         THE COUNT TRAVELS WITH IT, and that is the correction. `days` is a
         UNION across the rows in view and this was `find` — a "some row
         failed" test — so one bad series out of forty set a reason beside a
         list that was populated from the other thirty-nine. The tooltip then
         said no day could be offered while the picker was offering days. The
         two reducers disagreed; the counts let the render site tell an empty
         list from a short one. */
      daysFailed = answers.filter((a) => a.why).length;
      daysAsked = answers.length;
      daysWhy = answers.find((a) => a.why)?.why ?? null;
    });
    return () => {
      live = false;
    };
  });

  /* A CHOSEN DAY MUST STILL BE ON OFFER, AND NOTHING CHECKED.
     ------------------------------------------------------------------------
     `day` was cleared only when the tab, year, month, timeframe or segment
     changed. The option list is a fold over the VISIBLE rows, so it also moves
     when the rows do — a chip filter, a sort, a segment narrowing, or the pager
     — and none of those clears the day. The control then rendered BLANK, with
     the picked date absent from its own list, while that same date was still
     being applied to every quote on the page: a filter in force that the
     control no longer showed.
     Clearing is right rather than keeping: the day came from a list that no
     longer contains it, so there is nothing to re-select. `days.length > 0`
     guards the transient window where the list has not been folded yet, which
     would otherwise clear a perfectly good day on every refetch. */
  $effect(() => {
    if (!day) return;
    /* THE CASE THE GUARD USED TO EXCLUDE. When `dayCapable` goes false the days
       effect sets `days = []`, and the old `days.length > 0` guard then BLOCKED
       the clear — so the chosen day stayed in force against every quote on the
       page while the Date control sat disabled and empty. That is exactly the
       "a filter in force that the control no longer shows" defect this clamp
       was added to prevent, reached through the clamp's own guard.
       A window that cannot answer a day at all must not keep applying one. */
    if (!dayCapable) {
      day = '';
      return;
    }
    if (days.length > 0 && !days.includes(day)) day = '';
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
  /** The reason at least one series in view could not be read. */
  let timesWhy = $state(/** @type {string|null} */ (null));
  /** The same two counts, for the same reason the day list states. */
  let timesFailed = $state(0);
  let timesAsked = $state(0);

  /* The same re-validation for the minute, for the same reason: its list is a
     fold over the same moving set of rows. */
  $effect(() => {
    if (!time) return;
    // The same case, for the same reason: a minute cannot outlive the day it
    // belongs to, nor a window that can no longer read one.
    if (!day || !dayCapable) {
      time = '';
      return;
    }
    if (times.length > 0 && !times.includes(time)) time = '';
  });

  $effect(() => {
    const feed = store.feed;
    const want = dayScope;
    const tf = timeframe;
    const mo = month;
    const on = day;
    if (!feed || !tf || !mo || !on || !dayCapable || want.length === 0) {
      times = [];
      timesWhy = null;
      timesFailed = 0;
      timesAsked = 0;
      return;
    }
    let live = true;
    // Cleared before the read, for the reason the day list states — a stale
    // minute list is worse here, because minutes look alike across days and
    // nothing on screen would show it belonged to the previous one.
    times = [];
    timesWhy = null;
    timesFailed = 0;
    timesAsked = 0;
    /* THE UNION AGAIN, for the reason the day list gives: two instruments need
       not be stamped with the same minutes, and a series that halted at noon
       would erase the afternoon from a picker built on it alone. */
    Promise.all(want.map((key) => timesOn(feed, key, tf, mo, on))).then((answers) => {
      if (!live) return;
      times = [...new Set(answers.flatMap((a) => a.times))].sort();
      /* THE `why` IS KEPT HERE TOO. `timesOn` returns it for exactly this
         purpose and this caller was throwing it away, so a failed month read
         disabled the Time picker with the words "a day holds one bar" —
         a statement about the timeframe, made when nothing had been read. */
      timesFailed = answers.filter((a) => a.why).length;
      timesAsked = answers.length;
      timesWhy = answers.find((a) => a.why)?.why ?? null;
    });
    return () => {
      live = false;
    };
  });

  /** key -> Quote, filled as the pool answers. */
  let quotes = $state(new Map());

  /* THE MAP IS KEYED BY INSTRUMENT; THE ANSWER IS KEYED BY THE WINDOW.
     ------------------------------------------------------------------------
     `quotes` maps an instrument key to a Quote, but the Quote in it was
     fetched for a particular (feed, timeframe, month, day, time) — five things
     this key does not carry. `$lib/terminal.svelte.js` keys its own cache on
     four of them and says why: "the four things that change the answer, and no
     more". This map drops all of them.

     Nothing cleared it. An `$effect` runs AFTER the DOM updates, so changing
     the month repainted the grid from the OLD map first and only then fired
     the fetch — seventeen rows at six outstanding is several round trips, and
     up to 250 when a sort or a chip widens it. For that whole window every
     LTP, Change, Change %, Volume and Open Interest on screen was a real
     measurement from a DIFFERENT window, sitting under controls naming the new
     one. Not a stale number: a true number filed under the wrong heading,
     which is worse because nothing about it looks wrong.

     Clearing on a window change is what makes `cellOf`'s `pending` branch
     reachable. It already renders `·` for "asked, not yet answered" and was
     unreachable for any instrument fetched once, because the old value was
     always there to render instead. */
  $effect(() => {
    void store.feed;
    void timeframe;
    void month;
    void day;
    void time;
    quotes = new Map();
  });

  $effect(() => {
    const feed = store.feed;
    const want = [...new Set([...(quotesWidened ? matching : rows), ...indices])];
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
    oichg: 'Open-interest change needs the previous bar’s open interest. `/bars/window.json` carries a change field for price and none for OI, so this was never measured.',
    value: 'Turnover in crore is price times quantity summed over the session. A bar file carries volume and close but not per-trade value, and multiplying the two would be an estimate presented as a measurement.',
    premium: 'Premium is the contract’s price less the underlying’s level at the same bar. It needs both series joined, and this store files a contract’s own bars without that join — the same reason Spot Price beside it has no source. The two chips that rank by it are refused for exactly this.'
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
   * @property {string} [kind]   how to format: `price` or `int`. `chgAbs` and
   *           `chg` carry their own formatting in `cellOf` and return before
   *           `kind` is read, so they declare none — two values that named a
   *           format nobody consulted were removed rather than left to imply
   *           a switch that does not exist.
   * @property {string} [why]    why there is no source, shown on hover
   */

  /** @type {Record<string, Col[]>} */
  const COLUMNS = {
    stocks: [
      { h: 'Name', src: 'name', align: 'left' },
      { h: 'LTP', src: 'close', align: 'right', kind: 'price' },
      { h: 'Change', src: 'chgAbs', align: 'right' },
      { h: 'Change %', src: 'chg', align: 'right' },
      { h: 'Value (Cr.)', src: null, why: NO_SRC.value, align: 'right' },
      { h: 'Volume', src: 'volume', align: 'right', kind: 'int' }
    ],
    options: [
      { h: 'Name', src: 'name', align: 'left' },
      { h: 'Spot Price', src: null, why: NO_SRC.spot, align: 'right' },
      { h: 'LTP', src: 'close', align: 'right', kind: 'price' },
      { h: 'Change', src: 'chgAbs', align: 'right' },
      { h: 'Change %', src: 'chg', align: 'right' },
      { h: 'Open Interest', src: 'oi', align: 'right', kind: 'int' },
      { h: 'OI Change', src: null, why: NO_SRC.oichg, align: 'right' },
      { h: 'OI Change %', src: null, why: NO_SRC.oichg, align: 'right' },
      { h: 'Volume', src: 'volume', align: 'right', kind: 'int' }
    ],
    /* FUTURES CARRIES NO `Premium` AND NO `Spot Price` IN ITS BASE SET. Both
       are inserted by `columnsFor` below, and only under the two chips that
       show them. See the note there — this was measured wrong the first time. */
    /* `Premium` AND `Spot Price` WERE MISSING FROM THIS ROW, and the capture is
       what says so: the source's Futures grid reads Name · Premium · Spot Price
       · LTP · Change · Change % · Open Interest · OI Change · OI Change % ·
       Volume, and this table jumped from the name straight to the LTP. Both are
       sourceless here for one reason — neither can be had without joining the
       contract to its underlying — which is the same reason `Premium` and
       `Discount` are the two refused chips on this tab. Stating them as columns
       with that reason on hover is what makes the refusal legible; omitting the
       columns hid a gap the source terminal shows plainly. */
    futures: [
      { h: 'Name', src: 'name', align: 'left' },
      { h: 'Premium', src: null, why: NO_SRC.premium, align: 'right' },
      { h: 'Spot Price', src: null, why: NO_SRC.spot, align: 'right' },
      { h: 'LTP', src: 'close', align: 'right', kind: 'price' },
      { h: 'Change', src: 'chgAbs', align: 'right' },
      { h: 'Change %', src: 'chg', align: 'right' },
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
    /* THE PER-CHIP BRANCHES ARE GONE, BECAUSE THEY BECAME UNREACHABLE.
       -----------------------------------------------------------------------
       This function existed for one measured fact: on Futures the source
       terminal shows `Premium` and `Spot Price` under the `Premium` chip,
       `Discount` and `Spot Price` under `Discount`, and neither under the other
       five. That is still true of the source terminal.

       It is no longer reachable here. Both chips need the underlying's bar
       joined to the contract's, a join this store does not make, so both carry
       a refusal instead of a rule and are rendered DISABLED — `chip` can never
       arrive as `Premium` or `Discount`. The branches were two column sets that
       could not be selected and a comment describing behaviour the page no
       longer had.

       The shape stays: a function of (tab, chip) rather than a lookup on tab,
       because the fact it encoded is real and returns the day the join does.
       `CHIP_WHY.Premium` and `CHIP_WHY.Discount` carry what is missing. */
    void chip;
    return COLUMNS[tab] ?? COLUMNS.stocks;
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

  /** THE ACTION BAR, PER TAB — the captures' sets, plus one deliberate
      divergence that is stated rather than smuggled.
      ------------------------------------------------------------------------
      The source hangs `Option Chain · Snapshot · Chart` under Options,
      `Snapshot · Chart` under Futures, and NOTHING under Stocks. Copying that
      exactly is what the first version did, and it produced a page where the
      only button that works could never be pressed: `Option Chain` and `Chart`
      are dark because this repository has neither feature, so `Snapshot` is the
      whole live bar — and it appeared on precisely the two tabs this store
      holds no contracts for, while Stocks, the tab with all the data and all
      the checkboxes, had no bar at all. Seventeen rows selectable and nothing
      to do with a selection is the inert control §4 bans, arrived at by being
      faithful.
      So Stocks gets `Snapshot` and only that. It is not in the capture; the
      alternative was a selection column that selects for nobody. The two dark
      buttons stay tab-accurate.
      @type {Record<string, string[]>} */
  const BAR = {
    stocks: ['Snapshot'],
    futures: ['Snapshot', 'Chart'],
    options: ['Option Chain', 'Snapshot', 'Chart']
  };

  /** WHY THE OTHER TWO ARE DARK. Neither is a missing button; both are features
      this repository does not have, and saying which is the difference between
      a control that is off and one that is broken.
      @type {Record<string, string>} */
  const BAR_WHY = {
    'Option Chain':
      'An option chain is every strike of one expiry laid out around the underlying’s level, with calls and puts facing each other. It needs the whole expiry’s contracts joined to the underlying’s series, and this store files each contract’s bars on their own — the same join that leaves Spot Price and Premium without a source in the grid above.',
    Chart:
      'This page renders no chart. brutex draws one on /backtest, against a run’s own execution series, and it is not a price chart of an arbitrary instrument — pointing this button there would open something that does not show what was clicked.'
  };

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
  /* -1 MEANS NO CHIP, AND IT IS REACHABLE — WHICH IT WAS NOT.
     The click handler only ever assigned `chipIndex = i`, so clicking the lit
     chip re-selected it and there was no way back to an unfiltered grid. That
     mattered because a chip FILTERS as well as ranks: an operator one click
     from `none of them passes Intraday Movers` was told to "clear the chip"
     by the empty state below, and no control on this page could do it. The
     only real escapes were to pick a different chip — still a chip — or to
     widen the set past `SORT_BUDGET` so the rule is dropped, which is not
     clearing anything.
     `chips[-1]` is `undefined`, so `activeChip` falls to `null`, `activeRule`
     to `null`, and the filter is skipped. No branch needed a new case. */
  let chipIndex = $state(0);

  /* ROW SELECTION — a plain object used as a set, and the shape is the point.
     ------------------------------------------------------------------------
     An array with `includes` would be O(n) per row and O(n²) to paint a grid,
     which is the scan `CLAUDE.md` §3 rule 4 refuses on the engine side and has
     no business here either at 250 rows. A key on an object is one hash probe,
     and Svelte 5 deep-proxies `$state` objects so adding or deleting a property
     is reactive without a `SvelteSet` import.

     THE SELECTION IS NOT DECORATION. The source's checkboxes feed its watchlist
     and comparison tools, neither of which exists here — a checkbox that
     selects nothing anybody can act on is exactly the inert control §4 bans. It
     is wired to `Snapshot` in the bar below, which copies what is ticked. */
  let picked = $state(/** @type {Record<string, true>} */ ({}));
  const pickedKeys = $derived(Object.keys(picked));
  const pickedCount = $derived(pickedKeys.length);
  /** Every row ON SCREEN is ticked — the header box's state, and it answers for
      the rendered page rather than the whole matching set, because that is what
      the box can actually toggle. */
  const allPicked = $derived(rows.length > 0 && rows.every((k) => picked[k]));
  /** @param {string} key */
  function togglePick(key) {
    if (picked[key]) delete picked[key];
    else picked[key] = true;
  }
  function toggleAll() {
    if (allPicked) for (const k of rows) delete picked[k];
    else for (const k of rows) picked[k] = true;
  }

  /* A SELECTION DOES NOT SURVIVE THE SET IT WAS MADE IN. Changing tab, feed,
     timeframe or month replaces the instruments on offer, and a key ticked
     under the old window would keep counting toward `Snapshot` while being
     invisible and unreachable — a hidden selection acting on a copy. */
  $effect(() => {
    void activeTab;
    void store.feed;
    void timeframe;
    void month;
    picked = {};
  });

  /** What the bar last did, so an action that succeeded or failed says which
      instead of appearing to do nothing. Cleared by the next action. */
  let barSaid = $state('');

  /* SNAPSHOT — the one button in the source's bottom bar that maps to something
     this page can actually do. It copies the ticked rows, or the whole rendered
     page when nothing is ticked, as tab-separated text: the headers as the
     source writes them, then one line per row, using the SAME `cellOf` the grid
     paints so the copy cannot disagree with the screen. A dash stays a dash —
     an absence copied as an empty cell would read as a zero in a spreadsheet. */
  async function snapshot() {
    const cols = columns;
    const take = pickedCount > 0 ? rows.filter((k) => picked[k]) : rows;
    if (take.length === 0) {
      barSaid = 'Nothing to copy — the grid is empty.';
      return;
    }
    const lines = [cols.map((c) => c.h).join('\t')];
    for (const key of take) {
      const q = quotes.get(key);
      lines.push(
        cols
          .map((c) => (c.src === 'name' ? labelOf(key) : cellOf(q, c).text || '—'))
          .join('\t')
      );
    }
    const text = lines.join('\n');
    try {
      await navigator.clipboard.writeText(text);
      barSaid = `Copied ${take.length} ${take.length === 1 ? 'row' : 'rows'} to the clipboard.`;
    } catch (err) {
      /* NAMED, NOT SWALLOWED. The clipboard is refusable — a permission, a
         non-secure origin, a browser that wants a user gesture it did not see —
         and a copy button that silently does nothing is indistinguishable from
         one that worked. */
      barSaid = `The clipboard refused the copy: ${err instanceof Error ? err.message : String(err)}`;
    }
  }

  /* THE INDEX STRIP COLLAPSES, which is the `^` the source draws under it.
     A backtesting operator scrolling a long grid wants the vertical space, and
     the strip is the least urgent thing on the page once a window is chosen. */
  let stripOpen = $state(true);

  /** The chip row's scroller, so the `‹ ›` buttons have something to drive. */
  let chipRail = $state(/** @type {HTMLElement|null} */ (null));

  /* RE-MEASURED WHEN THE CHIPS CHANGE, not only when they are scrolled.
     Each tab carries its own seven chips at its own widths, and the mover
     switch appears on exactly one tab — so both the travel and whether there is
     any travel at all change on a tab switch, with no scroll event to announce
     it. Without this the arrows keep the previous tab's enabled state. */
  $effect(() => {
    void chips;
    void moverTab;
    void chipRail;
    measureRail();
  });
  /** How far the rail is scrolled, and how far it can go. Held as state so the
      arrows can disable themselves at the end they cannot travel — an arrow
      that clicks and does nothing is the inert control section 4 names. */
  let railAt = $state(0);
  let railMax = $state(0);
  function measureRail() {
    if (!chipRail) return;
    railAt = chipRail.scrollLeft;
    railMax = Math.max(0, chipRail.scrollWidth - chipRail.clientWidth);
  }

  /** How far a `‹`/`›` press travels: most of a screenful, keeping a chip of
      overlap so the reader can see where they came from.

      ASSIGNS `scrollLeft` RATHER THAN CALLING `scrollBy({behavior:'smooth'})`,
      and that is a correction rather than a preference. MEASURED: `scrollBy`
      with `behavior:'instant'` moved the rail 150px and the identical call with
      `behavior:'smooth'` moved it ZERO — so the arrows fired their handler,
      reported no error, and did nothing. A control whose only motion path can be
      silently unavailable is not a control.

      And the fix does not survive a `scroll-behavior: smooth` in the CSS, which
      is how it was broken a second time: that property governs the assignment
      below as well as the scroll methods, so declaring it sent this line back
      through the path that had just been measured failing. `.chips` deliberately
      declares none.
      @param {number} dir `-1` for left, `1` for right */
  function railBy(dir) {
    if (!chipRail) return;
    const step = Math.max(160, chipRail.clientWidth * 0.8);
    chipRail.scrollLeft = Math.max(0, Math.min(railMax, chipRail.scrollLeft + dir * step));
    measureRail();
  }
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
    /* -10000 BASIS POINTS IS -100%, AND THE DIVISOR IS EXACTLY ZERO THERE.
       `close / (1 + chg / 10_000)` is a division by zero at that one input,
       giving ±Infinity — or NaN when the close is also 0 — and the result
       flowed on to `rupee`, which answers a non-safe-integer with an em dash.
       So the cell rendered a plain dash carrying the DERIVATION tooltip, which
       reads as "the store has no change here" when the truth is that the
       previous close was zero and the move cannot be reconstructed from a
       ratio at all.
       It is not hypothetical on this data: a contract that fell to nothing is
       exactly the `-100.00 %` row the source terminal prints all over its
       Futures tab, and section 3 rule 1 forbids answering it with a number
       nobody computed. `null` here reaches the named-absence branch instead. */
    if (quote.chg === -10_000) return null;
    const before = quote.close / (1 + quote.chg / 10_000);
    if (!Number.isFinite(before)) return null;
    return quote.close - before;
  }

  /** THE DERIVED ABSOLUTE MOVE, AS A RENDERED CELL — one spelling, three
      surfaces.
      ------------------------------------------------------------------------
      This was inline in `cellOf` and so belonged to the GRID alone, which is
      why the strip and the lead ticker showed a percentage with no rupee move
      beside it while the source terminal shows both everywhere it shows either.
      Lifting it out is what lets those two render the same figure with the same
      disclosure rather than a second, quieter version of this arithmetic.

      THE FIGURE IS DERIVED AND SAYS SO ON HOVER — AND THE SLACK IS FAR LARGER
      THAN THIS COMMENT FIRST CLAIMED. `chg` is INTEGER basis points and `close`
      is a PAISA integer, so the reconstruction is quantised to about
      `close / 10000` — and that quotient is in PAISA, not rupees. This block
      used to read "roughly five paise of slack" for a ₹52,265 contract. The
      arithmetic is 5,226,500 / 10,000 = 522.65 paisa, which is ₹5.23: the
      estimate was wrong by a factor of a hundred, and the sentence that followed
      it — "the last digit is arithmetic" — understated the reach by two more
      places. On that price the RUPEES are uncertain, not the paise.

      So the note states the slack in the same units it renders, and says which
      digits it reaches. The figure is still not rounded away: a reader comparing
      it against the percentage beside it should see a number consistent with
      that percentage, and rounding to the honest precision would break the
      correspondence. It is LABELLED instead.
      @param {Quote} quote
      @returns {{text: string, why: string|null, dir: number}} */
  function absCell(quote) {
    const v = absMove(quote);
    if (v === null) {
      return { text: '—', why: quote.chgWhy ?? 'No change was recorded for this bar.', dir: 0 };
    }
    const slackPaisa = Math.max(1, Math.round((quote.close ?? 0) / 10_000));
    return {
      text: rupee(Math.round(v)),
      dir: Math.sign(v),
      why:
        'Derived, not measured. The wire carries the close and the change as ' +
        'integer basis points, not the previous close, so this move is ' +
        'reconstructed from the two. Whole basis points leave about ' +
        rupee(slackPaisa) +
        ' of slack on this price, so the figure is uncertain from its ' +
        (slackPaisa >= 100 ? 'rupees' : 'paise') +
        ' down — it is shown at full precision only to stay consistent with ' +
        'the change percentage beside it, which is the figure the store holds.'
    };
  }

  /** One rendered cell: its text, its direction for colour, and the reason it
      is a dash when it is one.
      @param {Quote|undefined} quote @param {Col} col
      @returns {{text: string, why: string|null, dir: number, pending?: boolean}} */
  function cellOf(quote, col) {
    if (col.src === null) return { text: '—', why: col.why ?? null, dir: 0 };
    if (!quote) return { text: '', why: null, dir: 0, pending: true };
    if (quote.why) return { text: '—', why: quote.why, dir: 0 };

    if (col.src === 'chgAbs') return absCell(quote);
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
  /** THE EXPIRY AS THE SOURCE WRITES IT: `29 SEP`, not `2026-09-29`.
     Measured off the captures — an option reads `NIFTY 29 SEP 25000 CALL` and a
     future reads `RELIANCE SEP FUT`, so a future carries the month alone and an
     option carries the day with it. This page was printing the raw ISO key,
     which is the KEY form: right for a Map and wrong for a human, and the one
     thing `$lib/dates.js` opens by warning about.
     Parsed from the string rather than through a Date: an expiry is a calendar
     day, not an instant, so putting it through a timezone would be inventing a
     conversion it does not need.
     @param {string|null} iso @returns {string} */
  function expiryLabel(iso) {
    const m = String(iso ?? '').match(/^(\d{4})-(\d{2})-(\d{2})$/);
    if (!m) return String(iso ?? '');
    const mon = MON[Number(m[2]) - 1];
    return mon ? `${m[3]} ${mon.toUpperCase()}` : String(iso);
  }

  /** The expiry month alone, which is how the source names a future.
      @param {string|null} iso @returns {string} */
  function expiryMonth(iso) {
    const label = expiryLabel(iso);
    const parts = label.split(' ');
    return parts.length === 2 ? parts[1] : label;
  }

  /** The display name for a key — the underlying, plus a contract tail.
      @param {string} key @returns {string} */
  function labelOf(key) {
    const p = parseKey(key);
    if (!p.underlying) return key;
    if (p.kind === 'future') return `${p.underlying} ${expiryMonth(p.expiry)} FUT`;
    if (p.kind === 'option') {
      /* `strikeExact`, NOT `Math.round(strike / 100)`.
         The strike is a PAISA INTEGER and the rounded form printed a strike
         that was never listed: a 2,462.50 strike rendered as `2463`, and any
         two strikes inside the same rupee collapsed onto one label — two rows
         with the same name and different prices, which reads as a duplicate
         rather than as two contracts. `$lib/instrument.js` already owns the
         exact rendering and keeps the paise only when they are non-zero, so
         whole strikes stay short. `CLAUDE.md` §7: prices are paisa integers,
         never a float. */
      /* `CALL` and `PUT`, not `CE` and `PE`. The store files the exchange's own
         two-letter side and the source terminal spells it out; both name the
         same thing, and the one a human reads is the spelled one. */
      const side = p.side === 'CE' ? 'CALL' : p.side === 'PE' ? 'PUT' : (p.side ?? '');
      return `${p.underlying} ${expiryLabel(p.expiry)} ${strikeExact(p.strike)} ${side}`;
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
          <!-- AN ABSENT CHANGE GETS THE SAME DASH THE STRIP GIVES IT. Neither
               `if` here had an `:else`, so a known level beside an unknown
               change printed the level and nothing after it — the `chgWhy` was
               on the quote and never rendered. -->
          {#if lead.chg != null}
            <!-- Same three terms as the strip and the same derived figure. -->
            {@const move = absCell(lead)}
            <span
              class="tick-m"
              class:up={move.dir > 0}
              class:down={move.dir < 0}
              title={move.why}>{move.text}</span
            >
            <span class="tick-c" class:up={lead.chg > 0} class:down={lead.chg < 0}>
              ({pctText(lead.chg).replace(' %', '%')})
            </span>
          {:else}
            <span class="tick-w" title={lead.chgWhy ?? 'No change was computed for this bar.'}
              >—</span
            >
          {/if}
        {:else if lead?.why}
          <!-- THE HEADER OBEYS THE SAME RULE AS EVERY CELL BELOW IT.
               This branch did not exist: when the quote carried a `why` — every
               named refusal `quote` and `quoteOn` produce — the bar showed the
               instrument's name and silently omitted the number, reporting an
               absence with neither a marker nor a reason. The strip thirty lines
               down, rendering THE SAME quote, has always shown both. So the two
               surfaces disagreed about one value, and the more prominent of them
               was the one saying nothing.
               It contradicted this page's own stated rule — "every cell here is
               either a number that was read or a reason it was not" — at the
               one place a reader looks first. -->
          <span class="tick-w" title={lead.why}>—</span>
        {:else}
          <span class="tick-w" title="Nothing has been asked for this index yet.">·</span>
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
  <div class="tstrip" class:shut={!stripOpen}>
    {#if indices.length === 0}
      <!-- THE MESSAGE MUST NOT CLAIM MORE THAN WAS ASKED. It substituted "any
           timeframe" and "any month" when either was unset, turning "I have not
           asked yet" into "this feed holds no index series at ANY timeframe in
           ANY month" — a sweeping absence across the whole store, asserted from
           a query that was never run. The window is either named or the
           sentence says the window is not chosen yet. -->
      <span class="tstrip-empty">
        {#if !timeframe || !month}
          No window is chosen yet, so no index has been read.
        {:else}
          No index series is held for this feed at {timeframe} in {monthLabel(month)}.
        {/if}
      </span>
    {:else}
      {#each indices as key (key)}
        {@const q = quotes.get(key)}
        <span class="tcell">
          <span class="c-n">{labelOf(key)}</span>
          {#if q?.close != null}
            <span class="c-v">{rupee(q.close)}</span>
            {#if q.chg != null}
              <!-- THE RUPEE MOVE, WHICH THIS STRIP DID NOT SHOW.
                   The source writes `72,880.90  -170.95 (-0.23%)` — the level,
                   the absolute move, then the ratio in parentheses. This strip
                   showed the level and the ratio and dropped the middle term,
                   so the one figure that answers "how much" was missing from
                   the surface an operator reads first.
                   It is the SAME `absCell` the grid's Change column uses, which
                   is the point of lifting it out: two spellings of a derived
                   figure are two answers the day one of them is edited, and
                   this one carries the same hover note saying it is
                   reconstructed rather than measured. -->
              {@const move = absCell(q)}
              <span class="c-m" class:up={move.dir > 0} class:down={move.dir < 0} title={move.why}>
                {move.text}
              </span>
              <span class="c-c" class:up={q.chg > 0} class:down={q.chg < 0}>
                ({pctText(q.chg).replace(' %', '%')})
              </span>
              <!-- THREE STATES, BECAUSE UNCHANGED IS ONE OF THEM. This read
                   `chg >= 0 ? '↗' : '↘'`, so an index that closed exactly
                   where it opened drew a RISING arrow — while the colour
                   classes on the same span, testing `> 0` and `< 0`, correctly
                   showed neutral. The glyph and the colour disagreed about the
                   same number, and the glyph was the one that was wrong.
                   `chg` is INTEGER basis points, so `0` also covers a real move
                   smaller than half a basis point; `→` claims no direction
                   rather than inventing one. -->
              <span class="c-a" class:up={q.chg > 0} class:down={q.chg < 0}>
                {q.chg > 0 ? '↗' : q.chg < 0 ? '↘' : '→'}
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

  <!-- THE STRIP'S OWN HANDLE — the `^` the source centres on the strip's lower
       edge. It is a real control here rather than an ornament: the grid is the
       page and a backtesting operator scrolling 208 rows wants the 40px back,
       while the index levels are the least urgent thing once a window is
       chosen. `aria-expanded` states which way it will go, so it does not rely
       on the reader inferring direction from a glyph. -->
  <div class="striptab">
    <button
      type="button"
      class="stripbtn"
      aria-expanded={stripOpen}
      title={stripOpen ? 'Hide the index strip' : 'Show the index strip'}
      onclick={() => (stripOpen = !stripOpen)}
    >
      <span aria-hidden="true">{stripOpen ? '⌃' : '⌄'}</span>
      <span class="vh">{stripOpen ? 'Hide the index strip' : 'Show the index strip'}</span>
    </button>
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
    <!-- A `div`, NOT A SECOND `main`. The root layout renders
         `<main class="main" id="main">` OUTSIDE the block that stands the
         console bar down, so it is present on this route too — and this element
         sat inside it, giving the document two nested `main` landmarks. A
         screen reader's landmark list then offers two "main"s for one page and
         the skip link's target is the outer one, which is not the grid. No
         other route in this app emits a `main` of its own. The class carries
         all the styling; the tag carried only the defect. -->
    <div class="tgrid">
      <div class="ttabs">
        {#each tabs as t (t.id)}
          <!-- `aria-pressed` CARRIES THE STATE THE GREEN PILL CARRIES.
               Which tab is active lived only in a CSS class, so assistive
               technology was read three identical buttons with no indication
               which one the grid below belongs to. These are toggle buttons
               rather than a tablist: the grid is not a labelled tabpanel and
               claiming that relationship would be a second untruth. -->
          <button
            class="ttab"
            class:on={t.id === activeTab}
            aria-pressed={t.id === activeTab}
            onclick={() => (activeTab = t.id)}
          >
            {t.label}
          </button>
        {/each}
      </div>

      <div class="chiprow">
        <div class="chips" bind:this={chipRail} onscroll={measureRail}>
          <!-- A CHIP THIS STORE CANNOT RANK IS DISABLED AND SAYS WHY, rather
               than lighting up and ordering nothing. An inert control that
               looks live is the failure section 4 names; one that refuses and
               names the missing field is a fact the reader can act on. -->
          {#each chips as c, i (c)}
            <!-- The lit chip FILTERS rows out of the grid and reorders the
                 rest, and that state was visible only as a colour. -->
            <button
              class="chip"
              class:on={i === chipIndex}
              aria-pressed={i === chipIndex}
              disabled={!CHIP_RULES[c] || !canSortByQuote}
              title={CHIP_RULES[c]
                ? canSortByQuote
                  ? `Rank by ${c.toLowerCase()}.`
                  : `Ranking needs a price for every matching row, and this view holds ${totalRows.toLocaleString('en-IN')} against a budget of ${SORT_BUDGET.toLocaleString('en-IN')}.`
                : CHIP_WHY[c]}
              onclick={() =>
                CHIP_RULES[c] && canSortByQuote && (chipIndex = chipIndex === i ? -1 : i)}
              >{c}</button
            >
          {/each}
        </div>
        <!-- THE `‹ ›` PAIR THE SOURCE DRAWS BESIDE THE CHIPS.
             The row already scrolled — `overflow-x: auto` on `.chips` — but a
             trackpad swipe is the only way to drive it, and on a mouse there is
             no affordance at all that the seventh chip exists. The source shows
             both arrows unconditionally, so these do too; each disables itself
             at the end it cannot travel, which is the honest version of an
             arrow that would otherwise click and do nothing. -->
        <!-- NOT `aria-hidden`, which is where this started. Two focusable
             buttons inside an `aria-hidden` container is the one combination
             the attribute must never wrap: the control stays in the tab order
             and is announced as nothing. They carry real labels instead, and
             the glyph is what is hidden. -->
        <div class="rail">
          <button
            class="railbtn"
            type="button"
            aria-label="Scroll the ranking chips left"
            title="Scroll the chips left"
            disabled={railAt <= 0}
            onclick={() => railBy(-1)}><span aria-hidden="true">‹</span></button
          >
          <button
            class="railbtn"
            type="button"
            aria-label="Scroll the ranking chips right"
            title="Scroll the chips right"
            disabled={railAt >= railMax}
            onclick={() => railBy(1)}><span aria-hidden="true">›</span></button
          >
        </div>
        {#if moverTab}
          <!-- GAINERS / LOSERS. A radiogroup rather than two buttons: the two
               are mutually exclusive and always one of them is chosen, which is
               exactly what a radiogroup announces and what a pair of toggles
               would not. Disabled together when the set is past the budget,
               carrying the reason, because filtering by change means holding a
               change for every matching row. -->
          <div
            class="mover"
            role="radiogroup"
            aria-label="Show gainers or losers"
            title={canSortByQuote
              ? 'Filters the grid to rows that rose, or to rows that fell. A bar that closed unchanged is in neither.'
              : `Filtering by change needs a price for every matching row, and this view holds ${totalRows.toLocaleString('en-IN')} against a budget of ${SORT_BUDGET.toLocaleString('en-IN')}. Narrow the segment to use this.`}
          >
            <button
              type="button"
              class="mlabel"
              class:on={mover === 'gainers'}
              role="radio"
              aria-checked={mover === 'gainers'}
              disabled={!canSortByQuote}
              onclick={() => (mover = 'gainers')}>Gainers</button
            >
            <button
              type="button"
              class="mtrack"
              class:right={mover === 'losers'}
              aria-hidden="true"
              tabindex="-1"
              disabled={!canSortByQuote}
              onclick={() => (mover = mover === 'gainers' ? 'losers' : 'gainers')}
            ><span class="mknob"></span></button>
            <button
              type="button"
              class="mlabel"
              class:on={mover === 'losers'}
              role="radio"
              aria-checked={mover === 'losers'}
              disabled={!canSortByQuote}
              onclick={() => (mover = 'losers')}>Losers</button
            >
          </div>
        {/if}
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
                ? daysWhy
                  ? days.length === 0
                    ? `No day can be offered — every series in view failed to read: ${daysWhy}`
                    : `${daysFailed} of ${daysAsked} series in view could not be read, so this list may be short a day they traded: ${daysWhy}`
                  : 'The last bar on the chosen day. “Month” is the month’s last bar.'
                : rows.length === 0
                  ? 'There are no rows in view, so there is no month to read for a date. This is not a budget refusal — the grid is empty.'
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
                : timesWhy
                  ? times.length === 0
                    ? `No minute can be offered — every series in view failed to read: ${timesWhy}`
                    : `${timesFailed} of ${timesAsked} series in view could not be read, so this list may be short a minute they are stamped with: ${timesWhy}`
                  : times.length < 2
                    ? `At ${timeframe} a day holds ${times.length === 1 ? 'one bar' : 'no bars'}, so there is no minute to choose between. A finer timeframe puts more bars in the day.`
                    : 'The bar stamped at this minute. “Close” is the day’s last bar.'}
            >
              <option value="">Close</option>
              {#each times as t (t)}<option value={t}>{t}</option>{/each}
            </select>
          </label>
          <!-- THE EXPIRY, on the two tabs that have one. Every option is an
               expiry some contract on this tab actually carries, so the picker
               cannot offer a month the store has never filed. When the store
               holds no contracts at all it says that rather than sitting empty
               and disabled with nothing to explain it. -->
          {#if hasExpiry}
            <label class="tctl">
              <span>Expiry</span>
              <select
                class="pill"
                bind:value={expiry}
                disabled={expiries.length === 0}
                title={expiries.length === 0
                  ? `The store holds no ${tab?.label.toLowerCase()} contracts for this feed, so there is no expiry to choose. This is an empty store, not a refused control.`
                  : 'Show only contracts expiring in this month. “All” keeps every expiry.'}
              >
                <option value="">All</option>
                {#each expiries as e (e)}<option value={e}>{monthLabel(e)}</option>{/each}
              </select>
            </label>
          {/if}
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
          <!-- TWO EMPTIES, AND THEY ARE NOT THE SAME FACT.
               `rows` is post-chip-filter; the old sentence asserted an absence
               in the STORE, which is a fact about `matching`. The two cannot
               coincide on the filtered path: a chip's rule only applies while
               `canSortByQuote`, which requires `totalRows > 0`, so whenever a
               chip empties the grid the store provably holds matching series
               and the claim was false.
               The page already draws this distinction for the pager —
               "`sorted.length` AND NOT `totalRows`" — and did not draw it here. -->
          {#if totalRows === 0}
            <p class="tnote wide">
              Nothing to show for {tab?.label} at {timeframe || 'no timeframe'} in
              {month ? monthLabel(month) : 'no month'}. The store holds no series
              matching all three.
            </p>
          {:else}
            <p class="tnote wide">
              {totalRows.toLocaleString('en-IN')}
              <!-- The instruction names the control that performs it. It used to
                   say "clear the chip" while no control on the page could, which
                   left the operator's only real move a guess. -->
              {tab?.label.toLowerCase()} series match this window, and none of them passes
              <strong>{activeChip}</strong>. That is the filter's answer, not the store's —
              click <strong>{activeChip}</strong> again to clear it and see them.
            </p>
          {/if}
        {:else}
          <table>
            <thead>
              <tr>
                <!-- THE SELECTION COLUMN IS GONE, AND IT WAS THE ONE CONTROL
                     ON THIS PAGE THAT DID NOTHING.
                     Nothing in the file ever read a checkbox's state, so it was
                     inert — the shape section 4 bans and the shape this page's
                     own comments refuse everywhere else. It was also the only
                     control here with NO accessible name: the input had no
                     label, no aria-label and no title, and the header that
                     would have supplied one held a control and no text, so a
                     screen reader read the grid as seventeen consecutive
                     "checkbox, not checked" with nothing to tell them apart.
                     WCAG 4.1.2, Level A.
                     Two fixes were available — name it, or make it do
                     something — and both are wrong while nothing selects
                     anything. The source terminal has the column because its
                     checkboxes feed an order ticket; this page has no ticket to
                     feed. It returns the day something reads it.

                     THAT DAY IS THIS ONE, and the condition the paragraph above
                     set is the condition that has been met rather than waived:
                     `Snapshot` in the bar below the grid copies what is ticked.
                     The column is back with BOTH defects fixed — it drives a
                     real action, and every box carries an accessible name, the
                     header's saying what it toggles and each row's naming its
                     own instrument, so the seventeen anonymous "checkbox, not
                     checked" that failed WCAG 4.1.2 cannot recur. -->
                <th class="pick">
                  <input
                    type="checkbox"
                    checked={allPicked}
                    indeterminate={pickedCount > 0 && !allPicked}
                    disabled={rows.length === 0}
                    aria-label={allPicked
                      ? 'Clear the selection on every row shown'
                      : 'Select every row shown'}
                    title={allPicked
                      ? 'Clear the selection on every row shown'
                      : 'Select every row shown'}
                    onchange={toggleAll}
                  />
                </th>
                {#each columns as c (c.h)}
                  <!-- A SORTABLE HEADER IS A BUTTON INSIDE A `th`, NOT A `th`
                       WEARING A TABINDEX.
                       It used to be the latter: click and key handlers and
                       `tabindex="0"` on the cell itself. That is reachable by
                       Tab and it does respond — but its role stays
                       `columnheader`, so it appears in no screen-reader
                       controls rotor, and a reader listing what can be operated
                       on this page is told the grid has nothing. This is the
                       pattern `aria-sort` was designed to sit beside: the CELL
                       carries the sort state, the BUTTON is the control that
                       changes it, and the browser supplies the keyboard
                       behaviour that was hand-rolled before.

                       THE BUTTON IS ALWAYS RENDERED AND DISABLED WHEN THE
                       COLUMN CANNOT SORT, rather than branching. One spelling
                       of the header's content, so a marker added to it cannot
                       appear in one branch and not the other — and `disabled`
                       states the fact directly: not focusable, not clickable,
                       and announced as unavailable rather than as a plain
                       heading that mysteriously does nothing. -->
                  <th
                    class={c.align}
                    class:sortable={sortableCol(c)}
                    class:sorted={sortKey !== '' && sortKey === c.src}
                    aria-sort={sortKey !== '' && sortKey === c.src
                      ? sortDir === 'asc'
                        ? 'ascending'
                        : 'descending'
                      : undefined}
                    title={c.src === null
                      ? c.why
                      : sortableCol(c)
                        ? `Sort by ${c.h}`
                        : `Sorting by ${c.h} needs a price for every matching row, and this view holds ${totalRows.toLocaleString('en-IN')} of them against a budget of ${SORT_BUDGET.toLocaleString('en-IN')}. Narrow the segment, or sort by Name, which needs no price.`}
                    ><button
                      type="button"
                      class="sortbtn"
                      disabled={!sortableCol(c)}
                      onclick={() => toggleSort(c.src)}
                      >{c.h}{#if c.src === null}<span class="nosrc" aria-hidden="true">*</span
                        >{/if}{#if sortKey !== '' && sortKey === c.src}<span
                          class="sortmark"
                          aria-hidden="true">{sortDir === 'asc' ? '▲' : '▼'}</span
                        >{/if}</button
                    ></th
                  >
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each rows as key (key)}
                {@const q = quotes.get(key)}
                <tr>
                  {#each columns as c (c.h)}
                    {#if c.src === 'name'}
                      <!-- A ROW HEADER, NOT A PLAIN CELL. The table had column
                           headers and no row headers, so no number was
                           programmatically tied to the instrument it belongs
                           to: a screen reader moving across a row announced
                           "793.05, -1.27, -0.16 %" with the instrument named
                           only once, at the start, and never repeated. With
                           `scope="row"` every cell in the row is associated
                           with its instrument. -->
                      <td class="pick">
                        <!-- NAMED BY THE INSTRUMENT IT SELECTS. `labelOf` is
                             the same text the row's own header cell shows, so
                             a reader hears "NIFTY 29 SEP 25000 CALL, checkbox"
                             rather than the anonymous box this column was
                             removed for. -->
                        <input
                          type="checkbox"
                          checked={!!picked[key]}
                          aria-label={`Select ${labelOf(key)}`}
                          onchange={() => togglePick(key)}
                        />
                      </td>
                      <th scope="row" class="nm">{labelOf(key)}</th>
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
              <!-- THE SENTENCE HAS TO FOLLOW THE FETCH IT DESCRIBES. It read
                   "quotes are fetched only for rows on screen" unconditionally,
                   and that stopped being true the moment a sort or a ranking
                   chip widened the fetch to the whole matching set — which is
                   exactly when this pager is most likely to be visible. A
                   caption that describes the cheap path while the expensive one
                   is running is a claim about cost that the reader cannot
                   check. -->
              Show {Math.min(PAGE, sorted.length - rows.length)} more · {rows.length} of {sorted.length}
              {#if quotesWidened}
                — every matching row is quoted, because the order depends on it
              {:else}
                — quotes are fetched only for rows on screen
              {/if}
            </button>
          {/if}
        {/if}
      </div>

      <!-- ===================================================== THE ACTION BAR
           The source hangs `Option Chain · Snapshot · Chart` under the Options
           grid and `Snapshot · Chart` under Futures, and hangs nothing under
           Stocks. Same three, same two, same none — and each is either wired to
           something real or disabled with the reason it is not, which is the
           only honest way to draw a button whose feature does not exist here.
           A bar of three live-looking buttons where one works is the failure
           wearing a success's clothes that section 4 names. -->
      {#if BAR[activeTab]?.length}
        <div class="abar">
          {#each BAR[activeTab] as b (b)}
            {#if b === 'Snapshot'}
              <button
                class="abtn"
                type="button"
                disabled={rows.length === 0}
                title={pickedCount > 0
                  ? `Copy the ${pickedCount} selected ${pickedCount === 1 ? 'row' : 'rows'} as tab-separated text.`
                  : 'Copy every row shown as tab-separated text. Tick rows to copy only those.'}
                onclick={snapshot}
                >Snapshot{#if pickedCount > 0}<span class="acount">{pickedCount}</span>{/if}</button
              >
            {:else}
              <button class="abtn" type="button" disabled title={BAR_WHY[b]}>{b}</button>
            {/if}
          {/each}
          {#if barSaid}
            <!-- `aria-live`, so the outcome reaches a reader who cannot see the
                 bar change. `polite`: a copy is not an interruption. -->
            <span class="asaid" aria-live="polite">{barSaid}</span>
          {/if}
        </div>
      {/if}

      <!-- `bnote` AND `bkey` ARE GONE. Both were emitted with no rule in this
           file and none in `theme.css`, so they styled nothing and named
           nothing — a hook is only a hook if something holds it. The two spans
           inherit `.bbar` exactly as they did with the classes on them. -->
      <footer class="bbar">
        <span>
          Every figure is read from this store through <code>/store.json</code> and
          <code>/bars/window.json</code>. A dash is an absence with a reason on hover,
          never a zero.
        </span>
        {#if columns.some((c) => c.src === null)}
          <span><span class="nosrc" aria-hidden="true">*</span> no source in a bar store</span>
        {/if}
      </footer>
    </div>
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
    --line: #242424;
    --pill: #1b2c27;
    --ink: #dadada;
    --dim: #8e8e8e;
    /* One step below `--dim`, for a declined answer. Not sampled from the
       source — the source never declines — so it is this page's own value,
       chosen to sit below the number colour without vanishing on #121212. */
    --faint: #5a5a5a;
    --up: #5da81e;
    --down: #e15858;
    --acc: #06b878;
    /* The accent AS AN EDGE rather than as ink — measured #0C4A30 on the lit
       chip's border. A separate token because it is a separate decision: a
       border painted in the text's own green reads as a second label. */
    --edge: #0c4a30;
    /* The strip's vertical rule, measured at its brightest point. It is drawn
       as a gradient, so this is the value at the middle of the line and not a
       flat fill — see `.tcell + .tcell::before`. */
    --rule: #5d5d5d;

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
  /* SELECTED TEXT, DECLARED HERE BECAUSE THE GLOBAL ONE IS UNREADABLE ON THIS
     PAGE.
     ------------------------------------------------------------------------
     `theme.css:500` sets `::selection { background: var(--acc-soft); color:
     var(--ink-hi) }`, and this page redefines neither token — it redefines
     eleven, and those two are not among them. So the console's values apply,
     and under the LIGHT ramp `--ink-hi` resolves to #000814 while `--acc-soft`
     is a 10%-alpha teal that composites over #121212 to about #121d1f.
     Selected text then renders near-black on near-black at roughly 1.17:1:
     invisible.

     The light ramp is reachable here. `app.html` ships `data-theme="dark"`, but
     the layout defaults the mode to `auto` when storage is empty and then
     REMOVES the attribute, handing the decision to `prefers-color-scheme` — the
     same path the `color-scheme: dark` block above was written for. That
     declaration cannot help: its own comment says it exists for surfaces CSS
     cannot reach, and this is an explicit CSS colour, not a platform paint.

     Nine classes were renamed to dodge exactly this shape, and `thead th` and
     `tbody td` were reset because an element selector has no name to change.
     `::selection` is the same kind of global and was the one that was missed. */
  .term ::selection {
    background: var(--pill);
    color: var(--ink);
  }

  /* SCROLLBARS, FOR THE SAME REASON AND FROM THE SAME PLACE.
     `theme.css:506` sets `scrollbar-color: var(--n7) transparent` on the
     universal selector and paints `::-webkit-scrollbar-thumb` with `--n7` too.
     Under the light ramp `--n7` is #c7cfdd — a pale grey thumb on this page's
     black panels — and an explicit `scrollbar-color` OVERRIDES the
     `color-scheme: dark` above it, so that declaration cannot correct this
     either. Three surfaces here scroll: the table panel, the chip row and the
     index strip. */
  .term *,
  .term {
    scrollbar-color: var(--line) transparent;
  }
  .term ::-webkit-scrollbar-thumb {
    background: var(--line);
    background-clip: content-box;
  }
  .term ::-webkit-scrollbar-thumb:hover {
    background: #3a3a3a;
    background-clip: content-box;
  }

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
  /* The header's absence marker, stated rather than inherited — the mistake
     `.c-v` made. `--faint` sits one step below `--dim` so a dash reads as LESS
     than a figure that was read, which is the whole distinction it draws. */
  .tick-w {
    font-size: 15px;
    color: var(--faint);
  }
  /* `.c-m` and `.tick-m` ARE ON BOTH LISTS, which is the whole reason they are
     declared rather than left to inherit — a move carries a direction exactly
     as the ratio beside it does, and an uncoloured rupee figure next to a green
     percentage reads as two facts about different things. `--dim` is the
     neutral, matching the label and the level, so an unchanged bar stays quiet. */
  .c-m,
  .tick-m {
    color: var(--dim);
  }
  .tick-c.up,
  .tick-m.up,
  .c-c.up,
  .c-m.up,
  .c-a.up,
  td.up {
    color: var(--up);
  }
  .tick-c.down,
  .tick-m.down,
  .c-c.down,
  .c-m.down,
  .c-a.down,
  td.down {
    color: var(--down);
  }
  .tnav {
    display: flex;
    gap: 22px;
    margin-left: 26px;
  }
  /* #8E8E8E measured on `Home`, against #0F0F0F. Same correction as `.ttab`
     directly below, and for the same reason. */
  .tnav a {
    color: var(--dim);
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
    /* 66px, not 56px: measured 33 CSS px of clearance on each side of the rule
       that now sits in this gap. At 56px the rule would crowd both neighbours. */
    gap: 66px;
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
    position: relative;
  }
  /* THE RULE BETWEEN STRIP ITEMS, WHICH THIS STRIP DID NOT HAVE.
     ------------------------------------------------------------------------
     The source separates its six index cells with five vertical rules and this
     page separated them with whitespace alone, so `Nifty Next 50 72,880.90`
     ran into `Nifty Bank 57,369.65` as one undifferentiated line of numbers —
     the reading problem the rules exist to solve.

     IT IS A GRADIENT, NOT A LINE, and that is measured rather than styled to
     taste. Scanning the separator column top to bottom: #141414 at its top
     edge, brightening through #292929 · #3F3F3F · #515151 to #5D5D5D at the
     middle, then symmetrically back down to #141414. A flat 1px rule in the
     peak colour would be markedly heavier than the source at both ends.

     Extent, in the CSS pixels of the 2x capture: the run is 47 device px tall
     inside an 80 device px band — 23.5 against 40 — and each side clears 33
     CSS px to the neighbouring text, which is what sets the 66px gap above.

     `::before` on the SECOND cell of each pair rather than `::after` on the
     first, so the strip cannot end with a trailing rule against its own
     padding. */
  .tcell + .tcell::before {
    content: '';
    position: absolute;
    left: -33px;
    top: 50%;
    transform: translateY(-50%);
    width: 1px;
    height: 24px;
    background: linear-gradient(to bottom, transparent, var(--rule), transparent);
  }
  .c-n {
    color: var(--dim);
  }
  /* `.c-v` HAD NO RULE ANYWHERE — not here, not in `theme.css`, not in the
     built bundle — while its three siblings all did. The index level was
     therefore the one value in the strip whose appearance was an accident of
     inheritance rather than a decision, and it read at the same weight as the
     dim label beside it. In the source terminal the level is the emphasis of
     the cell; stated, so it stays that way. */
  /* MEASURED, AND IT IS NOT WHAT THE FIRST VERSION OF THIS RULE CLAIMED.
     That rule set the index level to `--ink` at weight 600 and said the source
     makes the level the emphasis of the cell. Sampling the capture: the strip's
     value reads #8E8E8E — the same `--dim` as its label. The source does NOT
     emphasise the level here; it is the top-bar ticker that carries the bright
     one. The rule stays only to state that deliberately, rather than leaving
     the strip's most important figure styled by inheritance. */
  .c-v {
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

  /* AN ABSENCE IS FAINTER THAN A VALUE, not the same weight as one.
     Numbers now render at `--dim` to match the source, which is the colour the
     dashes used — so an em dash and a real figure would have been
     indistinguishable, and this page's whole argument is that they are
     different facts. `--faint` sits one step below, so a dash reads as LESS
     than a number rather than as another one. The source has no dashes to
     match here; it never declines to answer. */
  .dim {
    color: var(--faint);
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
  /* MEASURED OFF THE CAPTURE: an unselected tab is #8E8E8E, not #DADADA.
     `--ink` is the brightest ink on the page and it was spent here, on the six
     tabs the operator is NOT looking at — which flattened the one distinction
     the strip exists to draw. The source spends `--ink` only on the instrument
     name in the grid and keeps every secondary label at `--dim`. */
  .ttab {
    background: none;
    border: 0;
    color: var(--dim);
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
  /* Read by a screen reader, never painted. The chevron and the switch both
     carry a glyph for the eye and a sentence for everything else. */
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }

  /* THE STRIP, SHUT. Height and padding to nothing rather than `display: none`,
     so the collapse is a movement the eye can follow to the handle that caused
     it — the page's own rule that motion carries meaning. `visibility` stops
     the collapsed content taking focus while it is folded away. */
  .tstrip.shut {
    height: 0;
    padding-top: 0;
    padding-bottom: 0;
    border-bottom-color: transparent;
    overflow: hidden;
    visibility: hidden;
  }
  .tstrip {
    transition:
      height 140ms ease,
      padding 140ms ease;
  }
  .striptab {
    display: flex;
    justify-content: center;
    background: var(--ground);
    /* Pulled up onto the strip's own edge, exactly as the source hangs it. */
    margin-top: -1px;
  }
  .stripbtn {
    background: var(--panel);
    border: 1px solid var(--line);
    border-top: 0;
    border-radius: 0 0 7px 7px;
    color: var(--dim);
    font: inherit;
    font-size: 11px;
    line-height: 1;
    padding: 3px 20px 4px;
    cursor: pointer;
  }
  .stripbtn:hover {
    color: var(--ink);
    border-color: #3a3a3a;
  }

  /* THE CHIP RAIL'S ARROWS. Sized to the chips they sit beside rather than to
     the text inside them, so the three controls share one baseline. */
  .rail {
    display: flex;
    gap: 4px;
    flex: 0 0 auto;
  }
  .railbtn {
    background: var(--ground);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--dim);
    font: inherit;
    font-size: 15px;
    line-height: 1;
    padding: 5px 9px 7px;
    cursor: pointer;
  }
  .railbtn:hover:not(:disabled) {
    color: var(--ink);
    border-color: #3a3a3a;
  }
  /* At the end it cannot travel. Dimmer and inert, matching the vocabulary a
     refused chip already uses, so "nothing that way" reads the same everywhere. */
  .railbtn:disabled {
    color: var(--faint);
    border-color: #2a2a2a;
    cursor: default;
  }

  /* GAINERS ⟷ LOSERS. The lit side takes the accent and the other stays dim,
     which is the same two-state vocabulary the tabs and chips already use, so
     a third spelling of "this one is chosen" is not introduced. */
  .mover {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    margin-left: 14px;
  }
  .mlabel {
    background: none;
    border: 0;
    color: var(--dim);
    font: inherit;
    font-size: 13px;
    padding: 0;
    cursor: pointer;
  }
  .mlabel.on {
    color: var(--acc);
    font-weight: 600;
  }
  .mlabel:disabled,
  .mtrack:disabled {
    color: var(--faint);
    cursor: default;
  }
  /* The track is a 30x16 pill with an 11px knob that travels 13px — measured
     off the capture's switch rather than chosen. */
  .mtrack {
    position: relative;
    width: 30px;
    height: 16px;
    border-radius: 999px;
    border: 1px solid var(--edge);
    background: var(--pill);
    padding: 0;
    cursor: pointer;
  }
  .mknob {
    position: absolute;
    top: 1px;
    left: 2px;
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background: var(--acc);
    transition: transform 120ms ease;
  }
  .mtrack.right .mknob {
    transform: translateX(13px);
  }
  .mtrack:disabled .mknob {
    background: var(--faint);
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
    /* IT WRAPS, AND IT HAS TO SINCE THE ROW GAINED TWO CONTROLS.
       -----------------------------------------------------------------------
       `.ctrls` is deliberately unshrinkable (its own rule says so, and the
       seven pickers need their width), so `.chips` was the only item that could
       yield — and with `.rail` at 57px and `.mover` at 134px added beside it,
       yielding took it to ZERO. Measured at an 854px row: chips 0 · rail 57 ·
       mover 134 · ctrls 738, which is 929 of content in 854 of space. Every
       chip was still in the DOM and none of them was on screen.
       Wrapping puts the pickers on their own line when the four cannot share
       one, instead of silently deleting the control the line exists for. Above
       about 1200px nothing wraps and the row reads exactly as the capture. */
    flex-wrap: wrap;
    row-gap: 14px;
  }
  .chips {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    min-width: 0;
    /* GROWS INTO THE SPARE WIDTH, and carries a basis so it is not the item the
       browser chooses to collapse. The basis is a floor for the wrap decision,
       not a minimum width — `min-width: 0` still lets it scroll internally once
       it is the narrowest it will get. */
    flex: 1 1 260px;
    /* The scrollbar is the arrows' job now; a visible track under seven chips
       is 15px of chrome the source does not draw. */
    scrollbar-width: none;
    /* NO `scroll-behavior: smooth` HERE, AND THAT IS THE POINT.
       It was added on the reasoning that the animation would be "asked for in
       CSS and depended on nowhere", since `railBy` assigns `scrollLeft` rather
       than calling `scrollBy`. That reasoning is wrong: `scroll-behavior`
       governs the PROPERTY ASSIGNMENT too, not just the scroll methods, so it
       routed the direct assignment straight back through the same smooth path
       that had already been measured doing nothing — the arrows went dead a
       second time, for the same reason, one line after the comment explaining
       why they must not. The rail jumps. A control that moves is worth more
       than one that glides in the browsers where gliding happens to work. */
  }
  .chips::-webkit-scrollbar {
    display: none;
  }
  /* #8E8E8E on #000000 with a #242424 edge, all three measured. The border was
     already right; the text was `--ink` for the reason `.ttab` states. */
  .chip {
    background: var(--ground);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--dim);
    font: inherit;
    font-size: 13px;
    padding: 7px 14px;
    white-space: nowrap;
    cursor: pointer;
  }
  /* A REFUSED CHIP READS AS REFUSED BEFORE IT IS CLICKED. Dimmer text, a
     dashed edge and the default cursor, so the difference between "this ranks"
     and "this store cannot rank this" is visible rather than discovered. */
  /* `.chip.on:disabled` IS LISTED TOO, AND IT IS THE WHOLE POINT.
     `.chip:disabled` and `.chip.on` are both specificity (0,2,0), so the later
     one wins — and `.chip.on` is later. A chip that was BOTH lit and disabled
     therefore painted accent green with an accent border, reading as the live
     ranking chip while refusing every click.

     MEASURED with 300 instruments, past the 250 budget: `Intraday Movers` came
     back `disabled: true` and `#06B878`, while the six chips beside it were
     correctly `#5A5A5A` and dashed. So the one chip an operator would look at
     to learn the ranking was the one lying about it.

     (0,3,0) settles it in both directions rather than relying on source order,
     which the next edit to this block would silently change. */
  .chip:disabled,
  .chip.on:disabled {
    color: #5a5a5a;
    border-style: dashed;
    border-color: #2a2a2a;
    cursor: default;
  }
  /* THE LIT CHIP'S TEXT AND ITS EDGE ARE NOT THE SAME GREEN.
     Both were `--acc`, which drew a chip ringed in full #06B878 — brighter
     than the source and loud enough to compete with the value column it is
     meant to be ranking. Measured on `Highest OI`: the text is #06B878 and the
     border is #0C4A30, a green dark enough to read as an edge rather than as a
     second piece of ink. */
  .chip.on {
    color: var(--acc);
    border-color: var(--edge);
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
    /* THE CHEVRON IS A FILLED AMBER TRIANGLE, AND BOTH HALVES OF THAT WERE
       WRONG HERE.
       ------------------------------------------------------------------------
       This drew a STROKED grey caret — two 1.4px strokes meeting at a point, in
       #8E8E8E. Rasterising the source's control and reading the pixels row by
       row gives a solid wedge instead:

           ##############      y 634   14 device px
           ##############      y 635
           ##############      y 636
            ############       y 637
             ##########        y 638
              ########         y 639
               ######          y 640
                ####           y 641

       — contiguous on every row, so filled and not stroked; 13-14 device px
       across and 8 tall, which is 7 x 4 in CSS pixels at this capture's 2x.

       And it is ORANGE. #EA9324, which I did not believe from one sample and
       so scanned the whole picker band for: exactly two runs of that colour
       exist, x 3220-3232 and x 3351-3363, one under each of the two pickers,
       65 pixels each. It is the only warm colour anywhere in this chrome, which
       is presumably the point — the pickers are the one control that changes
       what every other surface is showing.

       The ground is `--head` (#181818), not `--panel` (#121212): measured as
       the run this pill's own box paints, x 3060-3385 on its centre row.

       Offset 11px rather than 9px — the triangle's right edge clears the pill's
       by 22 device px. */
    background: var(--head)
      url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="7" height="4" viewBox="0 0 7 4"><path d="M0 0h7L3.5 4z" fill="%23EA9324"/></svg>')
      no-repeat right 11px center;
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
    /* ZERO, BECAUSE THE BUTTON INSIDE CARRIES THE INSET. This read `0 10px`
       while a SECOND `thead th` block further down — identical selector, so it
       simply won — set `padding: 0`. Two rules for one property, and the loser
       was the one carrying the explanation. Consolidated here. */
    padding: 0;
    /* THE CELL'S OWN STRUT IS COLLAPSED, because it no longer carries text —
       all the header's content moved into the button, which sets its own
       line-height.
       IT IS NOT WHY THE HEADER MEASURES 42.5px, and an earlier version of this
       comment said it was. Measured: with the strut collapsed the row still
       reports 42.5, and it still does with the cell at `height: auto` or the
       button shrunk to 41px — so the content is not driving it at all. The
       half pixel is half of the 1px border this row COLLAPSES with the first
       body row: removing that row's `border-top` drops the header to exactly
       42. The painted rhythm is 42px throughout; `getBoundingClientRect`
       attributes a shared edge to both sides. Nothing to correct. */
    line-height: 0;
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
  /* `.nm` ALONE. `thead th.left` was listed here and is now inert: a header's
     text lives inside a flex button, which positions it with
     `justify-content`, so `text-align` on the cell reaches nothing. The body
     cell holds its text directly and still needs this. */
  .nm {
    text-align: left;
  }
  /* `tbody th` JOINS `tbody td` HERE, because the instrument cell became a row
     header for accessibility and a `th` in a body row otherwise takes the user
     agent's own bold, centred defaults — and matches none of the rules below,
     which are all written for `td`. */
  tbody td,
  tbody th {
    height: var(--h-row);
    padding: 0 10px;
    font-weight: 400;
    border-top: 1px solid var(--line);
    /* THE SAME RESET, FOR THE SAME REASON. `theme.css`'s `tbody td` carries a
       BOTTOM rule; this grid separates rows with a TOP one, and leaving the
       global in place drew both — a two-pixel band between every pair of rows
       that the source terminal does not have. */
    border-bottom: 0;
    text-align: right;
    white-space: nowrap;
    /* NUMBERS ARE DIM; THE NAME IS INK. MEASURED OFF THE SOURCE CAPTURE, and
       the opposite of what this page shipped.
       ---------------------------------------------------------------------
       Sampling the Options capture: the instrument name reads #DADADA, and the
       LTP and Open Interest values both read #8E8E8E. The original palette
       pass sampled the NAME and generalised it to the row, so every figure in
       this grid was a shade too bright and the name lost the emphasis the
       source gives it. The source spends its brightest ink on WHICH
       instrument, and renders the numbers quietly. */
    color: var(--dim);
  }
  /* The hover is declared here rather than inherited for the third time: the
     console's own `tbody tr:hover td` resolves to ITS token, which is a
     different colour on a page that does not use its ramp. */
  tbody tr:hover td,
  tbody tr:hover th {
    background: var(--head);
  }
  /* THE ACTION BAR. Left-aligned under the grid, as the source hangs it. */
  .abar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    padding: 14px 0 4px;
  }
  .abtn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    background: var(--head);
    border: 1px solid var(--line);
    border-radius: 7px;
    color: var(--ink);
    font: inherit;
    font-size: 13px;
    padding: 8px 15px;
    cursor: pointer;
  }
  .abtn:hover:not(:disabled) {
    border-color: #3a3a3a;
  }
  /* Dashed and dim, the same vocabulary a refused chip uses — so "this store
     cannot do that" reads identically wherever the page has to say it. */
  .abtn:disabled {
    color: var(--faint);
    border-style: dashed;
    border-color: #2a2a2a;
    cursor: default;
  }
  .acount {
    background: var(--pill);
    border: 1px solid var(--edge);
    border-radius: 999px;
    color: var(--acc);
    font-size: 11px;
    line-height: 1;
    padding: 3px 7px;
  }
  .asaid {
    font-size: 12px;
    color: var(--dim);
  }

  /* THE SELECTION COLUMN. Fixed width so the Name column starts at the same x
     on every row whatever the box is doing, and `accent-color` so the tick uses
     the page's own green instead of the platform blue that is the one colour
     nothing else here wears. */
  .pick {
    width: 34px;
    padding: 0 0 0 12px;
    vertical-align: middle;
  }
  .pick input {
    accent-color: var(--acc);
    width: 13px;
    height: 13px;
    margin: 0;
    cursor: pointer;
  }
  .pick input:disabled {
    cursor: default;
    opacity: 0.4;
  }
  .nm {
    color: var(--ink);
    border-right: 1px solid var(--line);
  }
  /* AND THE HEADER CELL ABOVE IT, WHICH THE DIVIDER USED TO STOP SHORT OF.
     ------------------------------------------------------------------------
     `.nm` is on the body's `th[scope=row]` only — the header cell is rendered
     with `class={c.align}`, so it takes `left` and never `nm`, and the rule
     above could not reach it. The divider therefore began one row BELOW the
     header, leaving `Name` and `Spot Price` sharing an unbroken strip of
     header while every row under them was divided.

     Measured on the source: the divider is continuous at #222222 across the
     header/body boundary — scanned unbroken for 161 device px spanning both.

     THE SELECTOR MOVED WHEN THE SELECTION COLUMN CAME BACK. It was
     `:first-child`, which was the Name cell right up until a checkbox `th` was
     inserted ahead of it — after which the rule drew the divider down the wrong
     side of the checkbox and the Name header lost it again, reopening exactly
     the gap it was written to close. It is pinned to `.nm`'s own header
     position now, which is the second cell, and `.pick` is excluded from the
     divider it never had in the source either. */
  thead th:nth-child(2) {
    border-right: 1px solid var(--line);
  }
  /* THE NAME CELL CARRIES THE LEFT INSET THE SELECTION COLUMN USED TO. With
     that column gone the first thing in a row is the instrument, and it would
     otherwise sit flush against the panel edge. */
  /* THE NAME COLUMN'S LEFT INSET IS APPLIED ONCE, ON WHICHEVER ELEMENT ACTUALLY
     HOLDS THE TEXT.
     ------------------------------------------------------------------------
     This listed `th.left` alongside `.nm`, and that survived the move of header
     padding onto the button — where it became a DOUBLE inset. `thead th` sets
     `padding: 0` at specificity (0,0,2) and `th.left` sets `padding-left: 14px`
     at (0,1,1), so the class wins and the header cell KEEPS its 14px; the
     button inside then adds its own. MEASURED: the header label sat at x 40
     while its column's cells sat at 26 — the one column whose alignment is
     most visible, out by exactly one inset.
     Body cells hold their text directly and take it from `.nm`. Header cells
     hold a button and take it from the button. Neither takes it twice. */
  .nm {
    padding-left: 14px;
  }
  th.left .sortbtn {
    padding-left: 14px;
  }
  /* `.right` IS EMITTED ON EVERY NUMERIC COLUMN AND HAD NO RULE. The default
     `text-align: right` on `thead th` and `tbody td` made it look correct, so
     the binding was right by accident rather than by declaration — and the
     first time that default changed, twenty-four columns would have moved with
     nothing naming them. Stated, so the binding means something. */
  /* `td.right` ONLY. `th.right` was listed and is inert for the same reason
     `thead th.left` is: the header's text is inside a flex button. The
     button's own `justify-content: flex-end` is what right-aligns a header,
     and `.sortbtn` declares it. */
  td.right {
    text-align: right;
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
  .sortbtn:disabled {
    cursor: default;
  }
  /* THE BUTTON IS THE WHOLE CELL, so the click target is what it looks like and
     the measured column geometry is unchanged: the padding that used to sit on
     the `th` moves here, and `inherit` keeps the alignment the column asked
     for rather than a button's centred default. */
  /* `display: block` AND `text-align`, NOT FLEX.
     Flex was the obvious way to fill the cell, and it blockified the header's
     children: `.nosrc { vertical-align: super }` became inert, so the
     no-source asterisk stopped being a superscript in the headers while
     staying one in the footer legend — the same mark, two shapes, in the two
     places a reader compares. `vertical-align` only has meaning between inline
     boxes, so the button stays an inline formatting context and gets its
     vertical centring from `line-height` instead of `align-items`. */
  .sortbtn {
    all: unset;
    display: block;
    box-sizing: border-box;
    width: 100%;
    /* THE ROW HEIGHT, PINNED — not `100%`. A `th`'s height is a MINIMUM, so a
       `100%` child cannot constrain it, and the superscripted no-source mark
       raises its inline box far enough to extend the line box: the header row
       measured 43px against the 42px every other row holds. `overflow: hidden`
       keeps the raised glyph from pushing the box back out; at 9px inside 42
       there is nothing to clip. */
    height: var(--h-row);
    line-height: var(--h-row);
    overflow: hidden;
    padding: 0 10px;
    text-align: right;
    cursor: pointer;
    white-space: nowrap;
  }
  th.left .sortbtn {
    text-align: left;
  }
  .sortbtn:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: -2px;
  }
  th.sortable:hover {
    color: var(--ink);
  }
  th.sorted {
    color: var(--ink);
  }
  /* The focus ring moved to `.sortbtn`: the cell is no longer the control, so
     focusing it is not a state that can occur. */
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
     THERE IS NO BREAKPOINT, AND THE ONE THAT WAS HERE DID NOTHING.

     It set `.tgrid { padding-left: 12px }` below 900px — the exact value the
     base rule already applies since the grid gained a left inset of its own,
     so the whole block was inert while its comment told the reader it was
     doing something. An inert rule that claims to be load-bearing is worse
     than no rule.

     The layout genuinely does not need one now the rail is gone: the grid is a
     single `minmax(0, 1fr)` column, the chip row scrolls inside itself, the
     pickers refuse to shrink, and `.panel` scrolls the table inside its own
     box rather than scrolling the page. Verified at 1714, 1280, 1024, 900 and
     760: controls inside the grid at every one. */

  /* MOTION IS ABSENT ON PURPOSE. Nothing on this page moves on its own, so
     there is no `prefers-reduced-motion` block to write: a terminal that
     animates a number invites a reader to believe it just changed, and every
     number here is a bar that was filed months ago. */
</style>
