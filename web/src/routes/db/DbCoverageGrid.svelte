<script module>
  /**
   * /db — THE COVERAGE GRID
   *
   * One row per stored instrument-month, windowed; the drill-through drawer; and
   * the % change notes panel (`notesPanel`, rendered by the shell above the view
   * split).
   *
   * Every line below was MOVED out of `routes/db/+page.svelte` by WP-00 (the /db
   * page split). The only edits to a moved line are the ones the move needs: a
   * page-level name that now lives in another file is imported, or, for state,
   * read as `db.<name>`; a plain module exports what it declares; and a callback
   * over such a value marks its parameter `any`, the way the page already did
   * elsewhere. Nothing was fixed in the move: every defect the audit found is
   * still present byte for byte, and is fixed by the package that owns this file
   * next.
   *
   * Owner after WP-00: WP-54 (coverage, coverage calendar and day flags).
   */
  import { isSole, rollUpMonths } from '$lib/completeness.js';
  import { fullestKey, OVER, heldWindow, dayText, pctText } from '$lib/db/decoration.js';
  import { txt, fmt, tfCmp, barsPerSession } from '$lib/db/notes-labels.js';
  import { bytesText, BYTES_PER_BAR, COLS, SOLE_WHY, whyText } from '$lib/db/notes-coverage.js';
  import { untrack } from 'svelte';
  import { ROW, HEAD } from '$lib/db/bars-plan.js';
  import { dayLabel, monthLabel } from '$lib/dates.js';
  import { dirOf, bpsText } from '$lib/bps.js';
  import DbCalendar from './DbCalendar.svelte';
  /** @typedef {import('$lib/store.svelte.js').StoreRow} StoreRow */
  /** @typedef {import('$lib/db/decoration.js').DecoRow} DecoRow */

  /**
   * THE COVERAGE GRID — its sort, its windowed rows, the drawer, the month cards
   * and the counted totals, and the % change notes panel.
   *
   * Creates no effect, so where the shell calls it does not change behaviour.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function coverageState(db) {
    let sortKey = $state('instrument');
    let desc = $state(false);

    /* ======================================================================
       THE TRADING CALENDAR, DERIVED FROM THE STORE ITSELF
       ====================================================================== */

    /**
     * How many bars a full month holds is not a constant and cannot be
     * calculated: it is 375 x the number of sessions NSE actually held, and
     * that depends on weekends, gazetted holidays, muhurat sessions and the
     * occasional unscheduled close. This repository does not invent an exchange
     * fact (`CLAUDE.md` section 3 rule 1), so the session count is OBSERVED:
     * for each month, the fullest instrument in the store is the month.
     *
     * O(n) once over the rows, then O(1) per lookup — a Map probe, not a scan,
     * so the 93,776-row census projected in `docs/06-limits.md` section 34
     * costs one pass and not one pass per row.
     *
     * IT IS BUILT FROM THE WHOLE STORE, NEVER FROM THE FILTERED VIEW. The
     * denominator is a fact about what NSE traded; narrowing the table to one
     * kind must not quietly lower the bar the remaining rows are judged against.
     */
    const monthFull = $derived.by(() => {
      // KEYED ON (MONTH, RUNG), NOT ON MONTH.
      //
      // "The fullest instrument in this month" is only a denominator among rows
      // of the SAME bar length. Keyed on the month alone, one 1min row (~8,250
      // bars) beside a 1day row (19) judges every daily row ~8,231 short — 0.23%
      // complete for a month that is actually full. Correct today only because
      // the store happens to hold one rung; that is not a property to rely on.
      return db.prepared.input === db.rows ? db.prepared.body?.observed.fullest ?? new Map() : new Map();
    });

    /**
     * HOW MANY ROWS STAND BEHIND EACH DENOMINATOR — the fact that decides
     * whether "0 short" is a measurement or a tautology.
     *
     * `monthFull` is "the fullest instrument in this (month, rung)". Where that
     * group holds ONE row, the row IS the denominator: it is short of itself by
     * zero, whatever it holds. The store holding `NSE-INDEX-NIFTY 2026-08 1min =
     * 1 bar` and nothing else for that month reported, on the one page built to
     * find holes: `Complete 1 of 1`, `Bars missing 0`, `Coverage 100.00%`, a
     * green `full` chip and a filled meter — over a single bar. The same
     * arithmetic reports a whole store of one bar as complete.
     *
     * A month held at a rung by one instrument is therefore UNVERIFIED, not full:
     * nothing in the store contradicts it and nothing corroborates it either, and
     * `CLAUDE.md` §3 rule 6 says which of those two to print. The same-month
     * many-instruments case was fixed by the D-0089-era rewrite; this is the
     * few-rows case it left behind.
     */
    const monthSupport = $derived(db.prepared.input === db.rows ? db.prepared.body?.observed.support ?? new Map() : new Map());

    /**
     * Whether this row is its own denominator — one row at this (month, rung),
     * so `short === 0` says nothing about the month.
     *
     * @param {StoreRow} r
     */
    const soleDenom = (r) => isSole(monthSupport, r);

    /**
     * Bars this instrument-month is short of the fullest one in the same month.
     *
     * # What this used to be, and why it was worse than useless
     *
     * `ceil(rows / 375) * 375 - rows`. That is `(-rows) mod 375`, so it is in
     * [0, 374] for EVERY row, whatever the month really held. It answered "is
     * this row a whole multiple of 375", never "does this row hold the month" —
     * and then the answer was labelled green "full" and counted in "Complete".
     *
     * Measured on the live store, both directions of the error:
     *
     * - `NSE-CASH-SHRIPISTON 2020-07` holds 147 bars against a 8,625-bar month.
     *   It is short 8,478 — 22.6 whole trading days. The old expression said
     *   228, understating the hole by 37x, and the row rendered as a minor gap.
     * - `NSE-INDEX-NIFTY 2020-03` holds 7,774 bars and IS the fullest instrument
     *   in its month — March 2020 held 20.73 sessions, because 2020-03-20 was a
     *   short day. It is complete. The old expression said it was short 101,
     *   inventing a hole in the one row that defines the month.
     *
     * A monitoring page whose entire stated job is "where are the holes"
     * reporting no hole — or a fictional one — is the fallback that hides a
     * failure `CLAUDE.md` section 4 bans, on the one surface built to prevent it.
     *
     * DO NOT "SIMPLIFY" THIS BACK INTO ARITHMETIC ON `rows` ALONE. There is no
     * expression over a single row that can know how many sessions a month had.
     *
     * @param {StoreRow} r
     */
    function shortBy(r) {
      return Math.max(0, (monthFull.get(fullestKey(r)) ?? r.rows) - r.rows);
    }

    const storeBars = $derived(db.deco.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + r.rows, 0));
    const kindCount = $derived.by(() => {
      const c = new Map();
      for (const r of db.deco) c.set(r.kind, (c.get(r.kind) ?? 0) + 1);
      return c;
    });

    /** How many of the whole dataset carry each reason code, per column. */
    const whyCount = $derived.by(() => {
      const chg = new Map();
      const prev = new Map();
      for (const r of db.deco) {
        if (r.chg === null) chg.set(r.chgWhy, (chg.get(r.chgWhy) ?? 0) + 1);
        if (r.prevChg === null) prev.set(r.prevChgWhy, (prev.get(r.prevChgWhy) ?? 0) + 1);
      }
      return { chg, prev };
    });

    const numbered = $derived(db.deco.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + (r.chg === null ? 0 : 1), 0));
    const refusedForSplits = $derived(whyCount.chg.get('corporate_action_unverified') ?? 0);

    /**
     * Every reason code present, counted in both columns, most common first.
     *
     * ENUMERATED, NEVER LISTED BY HAND. The version of this panel that named
     * `base_not_positive` and `overflow` and then said "neither appears in this
     * dataset" was rendered over a dataset containing a `base_not_positive` row.
     * A panel about honesty cannot afford a sentence that is not counted, so the
     * sentence became a count.
     */
    const reasonRoll = $derived.by(() => {
      const codes = new Set([...whyCount.chg.keys(), ...whyCount.prev.keys()]);
      return [...codes]
        .map((code) => ({
          code,
          chg: whyCount.chg.get(code) ?? 0,
          prev: whyCount.prev.get(code) ?? 0
        }))
        .sort((a, b) => b.chg + b.prev - (a.chg + a.prev) || txt(a.code, b.code));
    });

    const NOTES = $derived([
      {
        k: 'What the two columns are.',
        v: `% change is the month's OWN first close to its OWN last close. Prev % change is the previous calendar month's same statistic for the same instrument — not the previous bar, because a row here is an instrument-month. Both arrive as integer basis points and the decimal point is inserted for display: 125 on the wire is +1.25% on screen. CLAUDE.md §7 bans the float for money and a ratio derived from money gets the same treatment, so no percentage on this page has ever been a float, at either end.`
      },
      {
        k: 'No corporate-action threshold is sourced. That is why most cells are a dash.',
        v: `${fmt(refusedForSplits)} of ${fmt(db.deco.length)} rows are refused for this reason, and it is the only one of the five that is PERMANENT — no pull fixes it. docs/00-charter.md names no verified split-and-bonus feed, and docs/05-decisions.md D-0018 both requires a suspected corporate action to be refused loudly and rejects back-adjusting from an unverified source. An unadjusted 1:5 split is a fake 80% overnight crash that no stored price can be told apart from a real move. So every instrument that can be re-based — cash equities, and the F&O contracts NSE adjusts alongside them — is marked rather than numbered, in BOTH columns, whether or not its neighbour is held. Indices never split, and the ${fmt(kindCount.get('INDEX') ?? 0)} INDEX row(s) are the only ones eligible for a number at all.`
      },
      {
        k: 'A month-level field can flag a month. It cannot name the day.',
        v: `D-0018's stated detector is an unexplained OVERNIGHT gap, which is a bar-to-bar test over a month's ~8,250 bars — O(bars) work that cannot live in a census row (D-0067). Even with a threshold in hand, this column could only ever say "a corporate action may fall in this month", never "split on 2024-06-14". Anything stronger would have to be measured somewhere else and is not being claimed here.`
      },
      {
        k: 'Every reason on this dataset, counted.',
        v:
          reasonRoll.length === 0
            ? 'Nothing is refused: every row in this dataset carries a number in both columns.'
            : `${reasonRoll.map((r) => `${r.code}: ${fmt(r.chg)} in % change, ${fmt(r.prev)} in prev`).join(' · ')}. Counted from the rows themselves rather than listed here, so this line cannot claim a code is absent while the table below shows one.`
      },
      {
        k: 'What it cost to make this affordable.',
        v: `Reading the two closes from /bars.json instead would be ${fmt(db.deco.length)} request(s) and about ${bytesText(storeBars * BYTES_PER_BAR)} moved — measured at ${BYTES_PER_BAR.toFixed(1)} bytes per bar, from 668,251 bytes for 8,250 bars — to extract ${bytesText(db.deco.length * 16)} of signal. D-0067 put the two closes in the census entry instead, so a row and its neighbour are two hash probes and no file is opened. This page pays one request for the whole table, as it always did.`
      }
    ]);

    let noteOpen = $state(false);

    const sorted = $derived.by(() => {
      const k = sortKey;
      const dir = desc ? -1 : 1;
      return [...db.matched].sort((a, b) => {
        let d = 0;
        // `days` IS ITS OWN FIELD. Both keys used to sort on `a.rows`, so clicking
        // the Sessions header ordered the table by Bars — two different numbers,
        // and a column that sorts by a neighbour is worse than one that does not
        // sort at all, because the arrow says it worked.
        if (k === 'rows') d = a.rows - b.rows;
        // `?? 0` IS THE COERCION JAVASCRIPT ALREADY PERFORMS, WRITTEN DOWN.
        // `days` is `null` for every rung `barsPerSession` has no session size
        // for — 2min, 3min, 5min and the rest of the ladder — and `null - null`
        // is 0 while `5 - null` is 5, because `-` sends `null` to 0 before it
        // subtracts. Spelling it out changes no order and no output; it only
        // stops the expression from reading as though a null could not arrive.
        //
        // IT IS DELIBERATELY NOT THE `?? -1` RULE `pct` USES TWO LINES DOWN.
        // Sorting an unknown session count below every known one would be a
        // different table from the one this page renders today, and changing
        // which rows an operator sees first is not a typing change.
        else if (k === 'days') d = (a.days ?? 0) - (b.days ?? 0);
        else if (k === 'short') d = a.short - b.short;
        // AN UNKNOWN SORTS AS AN UNKNOWN. `a.pct` is null for a row that is its
        // own denominator, and `null - null` is 0 while `0.9 - null` is 0.9 —
        // a comparator that treats "not measured" as zero orders the table by a
        // number nobody computed. Below every real ratio, and stable among
        // themselves.
        else if (k === 'pct') d = (a.pct ?? -1) - (b.pct ?? -1);
        else if (k === 'month') d = txt(a.month, b.month);
        // BY LENGTH, NOT BY SPELLING. `txt` stood here, which ordered the rungs
        // `15min`, `1day`, `1min`, `30min` — by first character, which is the
        // same defect as a month column ordered by `Apr`. `tfCmp` is the ladder
        // the new Timeframe control is drawn from, so the column and the control
        // cannot disagree about what "next rung" means.
        else if (k === 'timeframe') d = tfCmp(a.timeframe, b.timeframe);
        else if (k === 'chg' || k === 'prevChg') {
          // AN UNKNOWN SORTS LAST IN BOTH DIRECTIONS, and the `return` rather
          // than a `d` is what makes that true: folding it into `dir * d` would
          // put every dash at the TOP on one click and the bottom on the next,
          // so "biggest fall first" would open on a screen of dashes. A dash is
          // not a very small number; it is not a number, and it has no place on
          // either end of an ordering of numbers.
          if (a[k] === null && b[k] === null) d = 0;
          else if (a[k] === null) return 1;
          else if (b[k] === null) return -1;
          else d = a[k] - b[k];
        } else d = txt(a.instrument, b.instrument);
        if (d !== 0) return dir * d;
        // Deterministic tie-break, NOT inverted with the direction: two runs of
        // the same sort produce byte-identical order, so equal values never
        // jitter when the direction is flipped and flipped back.
        return txt(a.instrument, b.instrument) || txt(a.month, b.month);
      });
    });

    /**
     * A click on a column heading: same column flips the direction, a new column
     * opens on the end that column is interesting at.
     *
     * @param {string} key a `COLS[].key` — the field name `sorted` switches on
     */
    function head(key) {
      if (sortKey === key) desc = !desc;
      else {
        sortKey = key;
        // Open a numeric column on its interesting end: most bars first, biggest
        // hole first, biggest gain first — but completeness ascending, because
        // the WORST row is the one being looked for.
        desc = key === 'rows' || key === 'days' || key === 'short' || key === 'chg' || key === 'prevChg';
      }
    }

    /**
     * The `aria-sort` value for one heading — the direction stated to a screen
     * reader, from the same two variables the arrow is drawn from.
     *
     * @param {string} key a `COLS[].key`
     */
    function ariaSort(key) {
      return sortKey === key ? (desc ? 'descending' : 'ascending') : 'none';
    }
    const censusPage = $derived(sorted.slice((db.pageNow - 1) * db.pageSize, db.pageNow * db.pageSize));

    /* ======================================================================
       THE SUMMARY, over the MATCHED set — the question actually on screen
       ====================================================================== */
    const total = $derived(db.matched.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + r.rows, 0));
    /**
     * COMPLETE MEANS COMPARED. A row that is the only one stored at its (month,
     * rung) is not counted here in either direction: it is not complete and it is
     * not short, because nothing in the store can tell which. See `SOLE_WHY`.
     */
    const full = $derived(db.matched.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + (r.short === 0 && !r.sole ? 1 : 0), 0));
    const unverified = $derived(db.matched.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + (r.sole ? 1 : 0), 0));
    const gaps = $derived(db.matched.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + r.short, 0));
    /** The rows a denominator other than themselves stands behind. */
    const comparable = $derived(db.matched.filter((/** @type {any} */ r) => !r.sole));
    /** Bars in those rows — the numerator Coverage is entitled to use. */
    const judgedBars = $derived(comparable.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + r.rows, 0));

    /**
     * Session-equivalents across the selection, each row converted AT ITS OWN
     * RUNG before summing.
     *
     * A single `total / 375` cannot be right for a store holding two rungs: 403
     * daily bars are 403 sessions, not 1.07. Rows at a rung with no recorded
     * session size contribute nothing rather than a guess, which is why this can
     * read lower than the bar count implies — and why the label beside it says
     * "session-equivalents" rather than "sessions".
     */
    const sessionEquivalents = $derived(
      db.matched.reduce((/** @type {any} */ a, /** @type {any} */ r) => {
        const per = barsPerSession(r.timeframe);
        return per === null ? a : a + r.rows / per;
      }, 0)
    );

    /** Whole sessions missing, each row at its own rung. Same rule as above. */
    const gapSessions = $derived(
      db.matched.reduce((/** @type {any} */ a, /** @type {any} */ r) => {
        const per = barsPerSession(r.timeframe);
        return per === null ? a : a + r.short / per;
      }, 0)
    );

    /**
     * How many matched rows sit at a rung with a recorded session size — the
     * denominator BEHIND the two session figures above, counted rather than
     * assumed.
     *
     * Both of those sums skip a row whose rung `barsPerSession` has no number
     * for. When every row on screen is at such a rung the sums are `0`, and `0`
     * is a measurement: it says the selection holds no sessions, when the truth
     * is that nobody can convert the bars it does hold. Counting the convertible
     * rows lets the sub-lines print a dash and name that, which is the same rule
     * `dayText` already enforces one row at a time.
     */
    const ranged = $derived(
      db.matched.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + (barsPerSession(r.timeframe) === null ? 0 : 1), 0)
    );

    /* WHAT THE COMPARABLE SET WOULD HOLD IF FULL.
       `total + gaps` counted the self-denominated rows into the numerator AND
       gave them a shortfall of zero, so a store holding one bar reported
       100.00% coverage with a full meter. Those rows are now outside both
       halves, and the tile says how many. */
    const owed = $derived(judgedBars + gaps);

    /**
     * Bars held over bars owed — or `null`, which is a THIRD answer and not a
     * bad hundred percent.
     *
     * `owed > 0 ? total / owed : 1` said 100.00% over a selection holding no
     * bars at all, and the meter beside it filled to the end. Every filter that
     * matches nothing reaches that branch, so the one tile an operator uses to
     * judge the store reported perfect coverage of nothing, in the confident
     * shape of a measurement. 0/0 is not one; it is undefined, `pctText` draws
     * the dash, and the tile names which absence it is.
     */
    const coverage = $derived(owed > 0 ? judgedBars / owed : null);
    const monthsIn = $derived.by(() => {
      const s = new Set();
      for (const r of db.matched) s.add(r.month);
      return [...s].sort();
    });

    /* ---- the month roll-up ---------------------------------------------
       WHICH MONTHS ARE COMPLETE, WHICH HAVE HOLES, HOW BIG. A hole is almost
       never one instrument — it is a session the pull did not finish — so the
       month is the altitude the answer actually lives at. Each card is also the
       filter for that month, because seeing a hole and then having to type its
       name is a page that shows a problem and hides the way in. */
    /**
     * THE DENOMINATOR IS SUMMED PER ROW, NOT SNAPSHOT FROM THE FIRST ONE.
     *
     * `fullest` was read once, inside `if (!m)` — from whichever row of that
     * month `/store.json` happened to send first — and then multiplied by the
     * row count. `monthFull` is keyed on (month, RUNG) and says why three lines
     * into its own comment; this took a (month, rung) figure and applied it to
     * every rung in the month. Measured on 2021-08 holding NIFTY@1day=23,
     * NIFTY@1min=8,625 and BANKNIFTY@1min=8,625 — a month that is genuinely
     * complete at both rungs — the card read `25,033.33%` with a meter 250 times
     * its own width and the title "every one of the 3 instruments holds all 23
     * bars" if the daily row arrived first, and `66.75%` tagged SHORT if the
     * minute row did. One headline number with two values, decided by row order.
     *
     * `owed` is the sum of each row's OWN (month, rung) denominator, so every row
     * is judged at its own rung and the ratio is the same whatever order the wire
     * used. `fullest` survives only as a display figure and only where the month
     * holds one rung — `sessions` already worked this way.
     */
    const monthCards = $derived.by(() =>
      rollUpMonths(db.windowed)
        .map((m) => {
          // ONE CALL, ONE VALUE — the hoist `deco` already makes for the same
          // reason. `barsPerSession` was called TWICE below, once to test for
          // `null` and once to divide by, so the test narrowed nothing that
          // could be checked and the second call was free to be a different
          // answer than the one that was tested. It is pure and it is not,
          // today; a guard that only happens to hold is not a guard.
          //
          // `m.tf` is the rung this month is stored at, or `false` once two
          // rungs disagree — see `rollUpMonths`. A month with no single rung has
          // no session size either, and asking for one is the question that
          // returns `null` here.
          const per = m.tf ? barsPerSession(m.tf) : null;
          return {
            ...m,
            // THE FULLEST ROW, AND ONLY WHERE ONE RUNG MAKES THAT A SINGLE
            // NUMBER. `null` draws a dash and the title says the month holds
            // two rungs.
            fullest: m.tf ? (m.n > 0 ? m.owed / m.n : 0) : null,
            // NULL RATHER THAN A NUMBER when the rung is unknown or mixed; the
            // renderer draws a dash. `dayText` owns that rule for every caller.
            sessions: per !== null && m.n > 0 ? m.owed / m.n / per : null
          };
        })
        .sort((a, b) => txt(b.month, a.month))
    ); /* newest first: that is where a pull lands */

    /* A HOLE IS A HOLE, AND AN UNKNOWN IS NOT ONE. `state !== 'full'` counted the
       months that hold too little AND the months nothing can be said about into
       one figure labelled "with holes". They are different answers and the strip
       now prints both. */
    const holeMonths = $derived(
      monthCards.filter((m) => m.state === 'near' || m.state === 'gap').length
    );
    const unprovenMonths = $derived(monthCards.filter((m) => m.state === 'sole').length);
    /** @type {HTMLElement | null} */
    let scroller = $state(null);
    let scrollTop = $state(0);
    let viewportH = $state(600);

    const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVER));
    const count = $derived(Math.ceil(viewportH / ROW) + OVER * 2);
    /* WINDOWED INSIDE THE PAGE, NOT INSTEAD OF IT. The pager decides WHICH
       rows are reachable; the window decides which of those are in the DOM.
       Dropping the window when the pager landed would have put 1,000 rows of
       nine cells into the document on the largest page size, which is the
       layout cost the window exists to refuse. */
    const slice = $derived(censusPage.slice(first, first + count));

    /* THE SEAM APPEARS ONLY ONCE THERE IS SOMETHING BEHIND IT. A permanent rule
       down the pinned column is a border pretending to be a shadow; the hairline
       is drawn when — and only when — columns are actually sliding under it.

       IT IS A `class:` DIRECTIVE AND NOT `classList.toggle`, WHICH IS NOT A
       STYLE CHOICE. Svelte prunes CSS whose selectors it cannot see used in the
       template, so a class added only from JavaScript compiles to a rule that is
       deleted — the class lands on the node and nothing happens. Measured: the
       element carried `xscrolled` while the `::after` stayed at opacity 0.
       A boolean that flips at most twice per gesture costs nothing anyway. */
    let xscrolled = $state(false);
    function onScroll() {
      if (!scroller) return;
      scrollTop = scroller.scrollTop;
      const x = scroller.scrollLeft > 0;
      if (x !== xscrolled) xscrolled = x;
    }

    /* ======================================================================
       KEYBOARD — the table is navigable without a mouse
       ----------------------------------------------------------------------
       A virtualised list cannot give every row a tab stop: 20,516 tab stops is
       not navigation. The scroll box is the single stop and carries the cursor,
       which is the `aria-activedescendant` pattern — the focused row is always
       scrolled into view, so the id it names is always in the DOM.
       ====================================================================== */
    let cursor = $state(-1);
    /** @type {string | null} */
    let openKey = $state(null); /* the row whose detail drawer is open */
    /** @type {HTMLElement | null} */
    let drawerEl = $state(null);

    /**
     * Put the keyboard cursor on row `next` of the current page, clamped, and
     * scroll it into view.
     *
     * @param {number} next a row index into `censusPage`, in or out of range
     */
    function moveTo(next) {
      const n = censusPage.length;
      if (n === 0) return;
      cursor = Math.max(0, Math.min(n - 1, next));
      // Written straight to the DOM, never back into `scrollTop` state, so the
      // cursor moving and the list scrolling cannot feed each other.
      const el = scroller;
      if (!el) return;
      const top = HEAD + cursor * ROW;
      if (top < el.scrollTop + HEAD) el.scrollTop = cursor * ROW;
      else if (top + ROW > el.scrollTop + viewportH) el.scrollTop = top + ROW - viewportH;
    }

    /**
     * Move the cursor by `d` rows. From no cursor at all it enters the list at
     * the end the movement came from — down enters at the top, up at the bottom.
     *
     * @param {number} d rows to move, signed
     */
    function step(d) {
      moveTo(cursor < 0 ? (d > 0 ? 0 : censusPage.length - 1) : cursor + d);
    }

    /**
     * Open the drawer on one census row.
     *
     * `undefined` IS A REAL ARGUMENT AND IT OPENS NOTHING. The Enter/Space path
     * passes `censusPage[cursor]`, and an index into a windowed page can be past
     * its end for the instant between a filter narrowing the list and the cursor
     * being reset — which is exactly why the guard is here rather than at the
     * one call site that can produce it.
     *
     * @param {DecoRow | undefined} it
     */
    function openRow(it) {
      if (it) openKey = it.key;
    }

    /** @param {KeyboardEvent} e */
    function onKey(e) {
      if (e.altKey || e.ctrlKey || e.metaKey) return;
      const page = Math.max(1, Math.floor(viewportH / ROW) - 1);
      switch (e.key) {
        case 'ArrowDown':
          step(1);
          break;
        case 'ArrowUp':
          step(-1);
          break;
        case 'PageDown':
          step(page);
          break;
        case 'PageUp':
          step(-page);
          break;
        case 'Home':
          moveTo(0);
          break;
        case 'End':
          moveTo(censusPage.length - 1);
          break;
        case 'Enter':
        case ' ':
          if (cursor >= 0) openRow(censusPage[cursor]);
          else return;
          break;
        case 'Escape':
          if (openKey) openKey = null;
          else if (cursor >= 0) cursor = -1;
          else return;
          break;
        default:
          return;
      }
      e.preventDefault();
    }

    /* ---- what the drawer shows -----------------------------------------
       EVERY MONTH THIS INSTRUMENT HOLDS, which is the drill-through the study
       asked for at the altitude this data actually supports. Built once as a
       Map, so opening a row is a probe and not a scan of 20,516 rows. */
    const byInstrument = $derived.by(() => {
      /* SAME REASON AS THE PREFIX INDEX ABOVE: an unannotated `new Map()` is
         `Map<any, any>`, and every month the drawer renders off `openMonths`
         would then be a shape nothing checks — on the one panel that reads nine
         fields off each row. */
      /** @type {Map<string, DecoRow[]>} */
      const by = new Map();
      for (const it of db.deco) {
        let a = by.get(it.instrument);
        if (!a) by.set(it.instrument, (a = []));
        a.push(it);
      }
      for (const a of by.values()) a.sort((x, y) => txt(y.month, x.month));
      return by;
    });

    /* AND A SECOND MAP, BECAUSE THE LINE BELOW WAS THE SCAN THE COMMENT ABOVE
       SAYS THERE ISN'T. `byInstrument` genuinely made the MONTH list a probe;
       resolving the opened ROW was still `deco.find(...)` — O(n) over the whole
       census, which `docs/06-limits.md` §34 projects at 93,776 rows — re-run on
       every `deco` change as well as on every open. One line under an invariant
       it did not hold, which is the shape §3 rule 4 exists to catch.

       Built in the same pass as `byInstrument` would have coupled two lookups
       with different keys; a second Map over the same array is one more pass at
       build time and turns the open into a probe for real. */
    const byKey = $derived.by(() => {
      /** @type {Map<string, DecoRow>} */
      const m = new Map();
      for (const it of db.deco) m.set(it.key, it);
      return m;
    });

    const openRowData = $derived(openKey ? (byKey.get(openKey) ?? null) : null);
    const openMonths = $derived(openRowData ? (byInstrument.get(openRowData.instrument) ?? []) : []);
    const openTotals = $derived.by(() => {
      let bars = 0;
      let missing = 0;
      let complete = 0;
      let unproven = 0;
      for (const m of openMonths) {
        bars += m.rows;
        missing += m.short;
        if (m.sole) unproven += 1;
        else if (m.short === 0) complete += 1;
      }
      return { bars, missing, complete, unproven, n: openMonths.length };
    });

    Object.defineProperties(db, {
      sortKey: { get: () => sortKey, set: (v) => (sortKey = v) },
      desc: { get: () => desc, set: (v) => (desc = v) },
      monthFull: { get: () => monthFull },
      monthSupport: { get: () => monthSupport },
      soleDenom: { get: () => soleDenom },
      shortBy: { get: () => shortBy },
      storeBars: { get: () => storeBars },
      kindCount: { get: () => kindCount },
      whyCount: { get: () => whyCount },
      numbered: { get: () => numbered },
      refusedForSplits: { get: () => refusedForSplits },
      reasonRoll: { get: () => reasonRoll },
      NOTES: { get: () => NOTES },
      noteOpen: { get: () => noteOpen, set: (v) => (noteOpen = v) },
      sorted: { get: () => sorted },
      head: { get: () => head },
      ariaSort: { get: () => ariaSort },
      censusPage: { get: () => censusPage },
      total: { get: () => total },
      full: { get: () => full },
      unverified: { get: () => unverified },
      gaps: { get: () => gaps },
      comparable: { get: () => comparable },
      judgedBars: { get: () => judgedBars },
      sessionEquivalents: { get: () => sessionEquivalents },
      gapSessions: { get: () => gapSessions },
      ranged: { get: () => ranged },
      owed: { get: () => owed },
      coverage: { get: () => coverage },
      monthsIn: { get: () => monthsIn },
      monthCards: { get: () => monthCards },
      holeMonths: { get: () => holeMonths },
      unprovenMonths: { get: () => unprovenMonths },
      scroller: { get: () => scroller, set: (v) => (scroller = v) },
      scrollTop: { get: () => scrollTop, set: (v) => (scrollTop = v) },
      viewportH: { get: () => viewportH, set: (v) => (viewportH = v) },
      first: { get: () => first },
      count: { get: () => count },
      slice: { get: () => slice },
      xscrolled: { get: () => xscrolled, set: (v) => (xscrolled = v) },
      onScroll: { get: () => onScroll },
      cursor: { get: () => cursor, set: (v) => (cursor = v) },
      openKey: { get: () => openKey, set: (v) => (openKey = v) },
      drawerEl: { get: () => drawerEl, set: (v) => (drawerEl = v) },
      moveTo: { get: () => moveTo },
      step: { get: () => step },
      openRow: { get: () => openRow },
      onKey: { get: () => onKey },
      byInstrument: { get: () => byInstrument },
      byKey: { get: () => byKey },
      openRowData: { get: () => openRowData },
      openMonths: { get: () => openMonths },
      openTotals: { get: () => openTotals }
    });
  }

  /**
   * WHAT MOVED BETWEEN TWO READS — the per-tile and per-row flash, and the reset
   * a feed change performs.
   *
   * Creates the page's effects that stood at old lines 6292; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function coverageFlash(db) {
    /* ======================================================================
       VALUE-CHANGE FLASH — THE ONLY GREEN AND RED ON THIS PAGE
       ----------------------------------------------------------------------
       `CLAUDE.md`-adjacent discipline, stated here because it is a rule this
       page now keeps and previously broke: GREEN AND RED MEAN DIRECTION. They
       mark a figure that MOVED between two reads of the store, and the two
       refused percentage columns they are reserved for. Data quality — short,
       near, gap — is a severity, not a direction, and wears the amber warning
       hue at two intensities instead. Red meaning both "the price fell" and
       "bars are missing" is two facts wearing one colour, and at a glance
       neither can be read.
       ====================================================================== */
    /* THE THREE MAPS THIS BLOCK KEEPS, AND WHAT EACH ONE'S VALUE MEANS.
       ----------------------------------------------------------------------
       `flash` is keyed by TILE NAME — `n`, `bars`, `full`, `gaps`, `cov`,
       `months`, the same six `snapshot` builds below — and its `n` is a
       generation counter, NOT a count of anything on screen: it is what re-keys
       the node so the animation restarts, and it is what the timeout compares
       against so a later movement's class is never cleared by an earlier one's
       timer. `dir` is `''` once the animation is over, which is the direction
       ceasing to be stated rather than a seventh direction.

       Declared rather than inferred because `$state({})` is `{}` — a type with
       no keys at all — so every one of the four reads below is
       `expression of type 'any' can't be used to index type '{}'`, and the
       object would silently accept a misspelt tile name for as long as nobody
       looked at the screen.

       `prev` holds the LAST VALUE each tile was seen at, and `null` is one of
       them: Coverage has no ratio when the selection has no denominator, and
       `mark` refuses to call the arrival or departure of an unknown a movement. */
    /** @type {Record<string, { n: number, dir: string }>} */
    let flash = $state({});
    /** @type {Map<string, number | null>} */
    const prev = new Map();
    /** @type {Map<string, ReturnType<typeof setTimeout>>} */
    const timers = new Map();
    let seeded = false;

    /**
     * Record one tile's new value, and tint it if it MOVED.
     *
     * @param {string} name the tile key — one of the six `snapshot` names
     * @param {number | null} v its value now, or `null` for "not measurable"
     */
    function mark(name, v) {
      const p = prev.get(name);
      prev.set(name, v);
      // AN UNKNOWN IS NOT A DIRECTION, IN EITHER POSITION. Coverage reports
      // `null` when the selection has no denominator, and `null > 0.9` is false
      // — so without this guard a figure becoming unknown would tint red and a
      // figure ceasing to be unknown would tint red as well, both stating a
      // movement nobody measured. `null` still lands in `prev` above, so the
      // baseline is current the moment a real number returns.
      if (p === undefined || p === v || p === null || v === null) return;
      const n = (flash[name]?.n ?? 0) + 1;
      flash[name] = { n, dir: v > p ? 'up' : 'down' };
      // DROP THE CLASS ONCE THE ANIMATION IS OVER. `n` is unchanged, so the node
      // is not re-keyed and nothing replays — the direction simply stops being
      // stated. A class that outlives its animation is a class waiting to fire
      // spuriously the next time the node happens to be re-created.
      clearTimeout(timers.get(name));
      timers.set(
        name,
        setTimeout(() => {
          if (flash[name]?.n === n) flash[name] = { n, dir: '' };
        }, 800)
      );
    }

    /* ---- ROW-LEVEL FLASH ------------------------------------------------
       The tiles flashed and the rows did not, which had it backwards: a running
       backfill lands bars in specific instrument-months, and the row is where
       that fact lives. `moved` maps a row key to the direction its bar count
       went between two reads, and it is CLEARED after the animation window so a
       row scrolled into view ten seconds later does not replay a stale tint. */
    let moved = $state(new Map());
    const barsBefore = new Map();
    let barsSeeded = false;
    /** `setTimeout` returns a `Timeout` under node's types and a `number` in the browser; naming the return type takes whichever this build resolves to rather than asserting one of them. */
    let movedTimer = /** @type {ReturnType<typeof setTimeout> | undefined} */ (undefined);

    /**
     * Drop every baseline the flash compares against, and the open drawer.
     *
     * Called from the fetch effect when — and only when — the ACTIVE FEED
     * changed. "This figure moved" is a claim about one store read twice; Dhan's
     * totals against Groww's baseline would tint six tiles and a screenful of
     * rows for a movement that never happened, because the two are not the same
     * instrument universe, the same session handling or the same price scale.
     *
     * Un-seeding rather than clearing-and-comparing is deliberate: the first
     * response from the new feed re-seeds and flashes nothing, which is the same
     * rule the initial page load already follows and for the same reason.
     *
     * The drawer goes too. It describes one instrument's every stored month, and
     * the new feed may not carry that instrument at all — `openRowData` would
     * fall to `null` on its own, but only after the response lands, leaving the
     * other feed's months on screen in between.
     *
     * And `elsewhere` goes, for the reason stated where it is declared: it names
     * the OTHER feeds that hold rows, computed against whichever feed was active
     * when it resolved, so across a switch it can offer the feed already chosen.
     * Its own effect re-runs and re-answers the moment the new store comes back
     * empty; until then the honest render is no list at all.
     *
     * THE CASCADE STATE DELIBERATELY SURVIVES, and that is not an oversight.
     * `filter`, `kind` and `month` have always survived a feed switch — they are
     * the operator's QUESTION, not the answer, and re-asking it by hand on every
     * switch is what makes a comparison impossible. `universe`, `fromMonth` and
     * `toMonth` join them for the same reason, and they cannot go stale
     * unnoticed the way a count can: the universe rung re-counts against the new
     * feed's master and REFUSES to count at all until that master arrives
     * (`universeRefusal`), and a month window is a plain string comparison that
     * is as true of one store as of another.
     */
    function forgetPreviousFeed() {
      seeded = false;
      barsSeeded = false;
      prev.clear();
      barsBefore.clear();
      /* THE HERO FIGURES FORGET TOO. Omitting these two was why `bars held` and
         `months covered` flashed on every feed change — and always green, since
         `rows` empties between feeds so the last leg is always upward. Cleared
         alongside `prev`, which is the same fact for the tiles. */
      db.factSeen.clear();
      db.factSeeded = false;
      for (const t of timers.values()) clearTimeout(t);
      timers.clear();
      clearTimeout(movedTimer);
      flash = {};
      moved = new Map();
      db.openKey = null;
      // `elsewhere` IS NOT DROPPED HERE ANY MORE — it is `$derived` off the
      // shared survey and filtered against the CURRENT selection, so the stale
      // shape this function used to have to remember cannot be built.
    }

    /**
     * THE FLASH MEANS "THE STORE MOVED", NOT "YOU TYPED".
     *
     * The effect depends on `fetchedAt` and NOTHING else — every figure is read
     * inside `untrack`. Two consequences, both deliberate:
     *
     * - Typing in the filter changes all six numbers and flashes none of them.
     *   A tile that tints on every keystroke is a tile whose tint carries no
     *   information, and it would tint over the exact figure being read.
     * - The first response seeds the baseline and flashes nothing. Comparing the
     *   arriving store against the empty page it replaced would flash all six
     *   green on every load, which is the same nothing said louder.
     */
    $effect(() => {
      const at = db.fetchedAt;
      untrack(() => {
        if (!at) return;
        /**
         * THE PAIRS ARE DECLARED AS PAIRS. Inferred, this array is
         * `(string | number | null)[][]` — every element of every row a union of
         * all three — so the destructured `k` below is as likely to be a number
         * as the tile name it actually always is, and `mark` cannot state what
         * it takes. The tuple says what the rows are: a name, and a figure or
         * the absence of one.
         *
         * @type {[string, number | null][]}
         */
        const snapshot = [
          ['n', db.matched.length],
          ['bars', db.total],
          ['full', db.full],
          ['gaps', db.gaps],
          /* `null` PASSES THROUGH AS `null`, never as a rounded zero.
             `Math.round(null * 10000)` is 0, which would enter the baseline as a
             coverage of 0.00% and flash red the instant a real ratio returned.
             `mark` refuses to give an unknown a direction. */
          ['cov', db.coverage === null ? null : Math.round(db.coverage * 10000)],
          ['months', db.monthsIn.length]
        ];
        if (!seeded) {
          seeded = true;
          for (const [k, v] of snapshot) prev.set(k, v);
        } else {
          for (const [k, v] of snapshot) mark(k, v);
        }

        /* One pass over the whole store per REFRESH — not per render, not per
           scroll frame, and never per row on screen. */
        const next = new Map();
        for (const it of db.deco) {
          const was = barsBefore.get(it.key);
          if (barsSeeded && was !== undefined && was !== it.rows) {
            next.set(it.key, it.rows > was ? 'up' : 'down');
          }
          barsBefore.set(it.key, it.rows);
        }
        barsSeeded = true;
        moved = next;
        clearTimeout(movedTimer);
        if (next.size) movedTimer = setTimeout(() => (moved = new Map()), 900);
      });
    });

    Object.defineProperties(db, {
      flash: { get: () => flash, set: (v) => (flash = v) },
      prev: { get: () => prev },
      timers: { get: () => timers },
      seeded: { get: () => seeded, set: (v) => (seeded = v) },
      mark: { get: () => mark },
      moved: { get: () => moved, set: (v) => (moved = v) },
      barsBefore: { get: () => barsBefore },
      barsSeeded: { get: () => barsSeeded, set: (v) => (barsSeeded = v) },
      movedTimer: { get: () => movedTimer, set: (v) => (movedTimer = v) },
      forgetPreviousFeed: { get: () => forgetPreviousFeed }
    });
  }

  /**
   * THE DRAWER TAKES FOCUS WHEN IT OPENS.
   *
   * Creates the page's effects that stood at old lines 7376; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function coverageDrawer(db) {
    /* THE CAST IS THE ONE FACT `querySelector` CANNOT KNOW AND THIS FILE CAN.
       `querySelector` answers `Element`, which is the interface an `<svg>` node
       and an XML node also satisfy, and `Element` has no `focus` — only
       `HTMLElement` does. `.dclose` is the drawer's own close control, declared
       forty lines into the markup below as a `<button>`, so it is an
       `HTMLButtonElement` and always has been; the type is narrowed here rather
       than the call being made conditional, because a conditional would turn
       "this selector matched nothing" — which would mean the drawer rendered
       without its close button, a real defect — into a keyboard trap that
       reports nothing. `?.` still guards the ordinary case: the effect can run
       in the frame before the drawer's children exist. */
    $effect(() => {
      if (db.openKey && db.drawerEl)
        /** @type {HTMLElement | null} */ (db.drawerEl.querySelector('.dclose'))?.focus();
    });
  }

  /**
   * A NEW VIEW STARTS AT THE TOP — the stamp that re-keys both grids and the
   * coverage grid's entrance.
   *
   * Creates the page's effects that stood at old lines 7440, 7444 and 7452; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function coverageStamp(db) {
    /* A NEW VIEW STARTS AT THE TOP. Keeping the old offset after a filter
       change lands the operator in the middle of a list they have not seen. */
    let stamp = $state(0);
    /* ONE PLACE THAT PUTS THE GRID BACK AT ITS TOP, called by both effects
       below. It was written out twice for one edit and that is exactly how the
       two copies drift — one of them gaining a reset the other never got. */
    function restartAtTop() {
      stamp += 1;
      if (db.scroller) db.scroller.scrollTop = 0;
      db.scrollTop = 0;
      db.cursor = -1;
    }

    /* MOVING BETWEEN PAGES, BETWEEN PAGE SIZES OR BETWEEN THE TWO GRIDS
       STARTS AT THE TOP TOO - and it is a SEPARATE effect, because this one
       must not write `page`. An effect that both reads `page` and resets it
       to 1 is a control that cannot leave its first page. */
    $effect(() => {
      void [db.page, db.pageSize, db.view];
      untrack(restartAtTop);
    });
    $effect(() => {
      void db.rows;
      untrack(() => (stamp += 1));
    });

    /* Rows animate in when the SET changes, and never while scrolling — a list
       that re-animates under the scrollbar is a list that cannot be read. */
    let entering = $state(false);
    $effect(() => {
      void stamp;
      entering = true;
      const t = setTimeout(() => (entering = false), 420);
      return () => clearTimeout(t);
    });

    Object.defineProperties(db, {
      stamp: { get: () => stamp, set: (v) => (stamp = v) },
      restartAtTop: { get: () => restartAtTop },
      entering: { get: () => entering, set: (v) => (entering = v) }
    });
  }

  export { notesPanel };
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

<!-- THE COVERAGE CALENDAR'S PLACE, HELD BY WP-00. It renders nothing until
     WP-54 fills DbCalendar.svelte; mounted now so that package edits only the
     calendar and never this grid. -->
<DbCalendar {db} />
<!-- ==================================================================
     THE CENSUS TABLE. Windowed: only the visible slice exists in the DOM.
     ================================================================== -->
<div class="tbl rise">
  <!-- The scroll region is FOCUSABLE ON PURPOSE and carries the row
       cursor. A keyboard operator has no other way to reach 20,516 rows —
       giving each row a tab stop is not navigation — so the box is the
       single stop and `aria-activedescendant` names the row inside it.
       Only Firefox focuses an overflow container by itself, so without the
       explicit stop PageDown does nothing in Chrome and WCAG 2.1.1 is
       unmet on the page's only content. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="tbl-scroll"
    class:xscrolled={db.xscrolled}
    bind:this={db.scroller}
    bind:clientHeight={db.viewportH}
    onscroll={db.onScroll}
    onkeydown={db.onKey}
    tabindex="0"
    role="grid"
    aria-rowcount={db.censusPage.length}
    aria-activedescendant={db.cursor >= 0 ? `dbrow-${db.cursor}` : undefined}
    aria-label="Stored instrument-months, {fmt(db.censusPage.length)} row(s) on page {fmt(
      db.pageNow
    )} of {fmt(db.pageCount)}. Arrow keys move, Enter opens."
  >
    <!-- `presentation` because this div exists for `min-width` and for the
         spacer's positioning context, and for nothing an assistive reader
         needs. Left with an implicit role it would sit between `grid` and
         its `row`s and break the ownership chain. -->
    <div class="tbl-inner" role="presentation">
      <div class="thead" role="row">
        {#each COLS as c, ci (ci)}
          <div
            class="th"
            class:numh={c.num}
            class:inst={ci === 0}
            role="columnheader"
            aria-sort={db.ariaSort(c.key)}
          >
            <button
              class="sortbtn"
              type="button"
              onclick={() => db.head(c.key)}
              title={c.why ? `${c.label} — ${c.why}. Click to sort.` : `Sort by ${c.label}`}
            >
              <span>{c.label}</span>
              <span class="caret" class:on={db.sortKey === c.key} class:desc={db.desc} aria-hidden="true"
              ></span>
            </button>
          </div>
        {/each}
      </div>

      {#if db.sorted.length === 0}
        <div class="tnone">
          <h3>No row matches</h3>
          <!-- THE NARROWINGS ARE LISTED FROM `narrowing`, THE SAME DERIVED
               THE TILES CONSULT. This used to be a hand-glued chain of
               `{#if}`s inserting its own commas, so the panel that explains
               an empty table and the strip that reports it were two
               independent spellings of one fact and could drift apart on
               any edit. The emphasis and the raw-key tooltip both survive;
               only the glue is gone. -->
          <p>
            {db.feedName} holds {fmt(db.deco.length)} instrument-months, and none of them match
            {#each db.narrowing as n, i (n.k)}{i > 0 ? ', ' : ''}<b
                title={n.key ? `the store's own key for it is ${n.key}` : undefined}
                >{n.text}</b
              >{/each}.
          </p>
          <!-- THE GATE IS NOT A FILTER AND THE BUTTON BELOW WILL NOT OPEN
               IT, so it is said here in full rather than left to be
               discovered by pressing the button and watching nothing
               change. `CLAUDE.md` §4: degrade loudly and name the reason —
               and name the way out, which is a control, not this button. -->
          {#if db.expiryHeld > 0}
            <p class="tgate">
              {fmt(db.expiryHeld)} of them are futures or options and are being <b>held back</b>,
              not filtered:
              {#if db.expiry}
                they are on a contract other than {dayLabel(db.expiry)}. Choose theirs on the
                Contract strip above.
              {:else}
                no expiry is chosen. A bar belongs to <b>one contract</b>, so this page will not
                put two expiries under one heading — choose an expiry on the Contract strip and
                they appear.
              {/if}
              Clearing the filters does not release them; only choosing an expiry does.
            </p>
          {/if}
          <button class="btn" onclick={db.reset}>Clear the filters</button>
        </div>
      {:else}
        <!-- The spacer is the FULL height of THE PAGE, so the scrollbar
             tells the truth about 1,000 rows while 30 of them exist. Rows
             are absolutely positioned at their own offset — no accumulated
             rounding down a long list. It was the full height of the whole
             sorted set before the pager landed; leaving it there would have
             drawn a scrollbar for 20,516 rows over a page holding 50. -->
        <div class="tspace" role="rowgroup" style="height:{db.censusPage.length * ROW}px">
          {#key db.stamp}
            {#each db.slice as it, i (it.key)}
              {@const n = db.first + i}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_interactive_supports_focus -->
              <div
                class="trow"
                class:odd={n % 2 === 1}
                class:row-in={db.entering}
                class:cur={db.cursor === n}
                class:open={db.openKey === it.key}
                data-state={it.state}
                id="dbrow-{n}"
                role="row"
                aria-rowindex={n + 1}
                aria-selected={db.cursor === n}
                onclick={() => {
                  db.cursor = n;
                  db.openRow(it);
                }}
                style="top:{n * ROW}px; animation-delay:{Math.min(i * 8, 180)}ms"
              >
                <!-- THE STORE'S OWN KEY IS ONE HOVER AWAY. This cell had
                     no title at all, so the 137px the column clips were
                     simply gone -- not moved, not abbreviated, gone. -->
                <span class="cell inst" role="gridcell" title={it.instrument}>
                  <span class="pip" aria-hidden="true"></span>
                  <span class="ihead">{it.head}</span><span class="isym">{it.sym}</span>
                </span>
                <!-- THE RAW KEY LEADS THE TOOLTIP AND THE DAY WINDOW
                     FOLLOWS IT. `2020-03` is the store's own spelling and
                     has to stay one hover away and copyable; the window
                     after it is the fact `first_ts` / `last_ts` have been
                     on the wire for and no page has read — it tells a month
                     held from the 2nd to the 31st from one held on the 2nd
                     alone, which no bar count can. -->
                <span
                  class="cell mono"
                  role="gridcell"
                  title={heldWindow(it)
                    ? `${monthLabel(it.month)} · held ${heldWindow(it)} — the store's own key for it is ${it.month}`
                    : `${monthLabel(it.month)} — the store's own key for it is ${it.month}`}
                  >{monthLabel(it.month)}</span
                >
                <span class="cell tf" role="gridcell">{it.timeframe}</span>
                <span
                  class="cell num"
                  role="gridcell"
                  class:flash-up={db.moved.get(it.key) === 'up'}
                  class:flash-down={db.moved.get(it.key) === 'down'}>{fmt(it.rows)}</span
                >
                <span class="cell num dimnum" role="gridcell">{dayText(it.days)}</span>
                <span class="cell num short" role="gridcell">
                  <!-- A TAUTOLOGY IS NOT A VERDICT. `short === 0` against a
                       denominator that is this row itself printed the green
                       word `full` over a month holding one bar. -->
                  {#if it.sole}
                    <span class="unproven" title={SOLE_WHY}>unverified</span>
                  {:else if it.short === 0}
                    <span class="okword">full</span>
                  {:else}
                    <span class="missnum">−{fmt(it.short)}</span>
                    <!-- THE NULL TEST IS WRITTEN OUT, NOT RELIED ON.
                         `lost` is `null` for a rung with no recorded
                         session size, and `null >= 1` is false — so the
                         days-lost chip was already suppressed for those
                         rows and still is. Stating the absence is what
                         lets `Math.floor` below be read as taking a
                         number, which is the only thing it can take. -->
                    {#if it.lost !== null && it.lost >= 1}
                      <span class="lost">{Math.floor(it.lost)}d</span>
                    {/if}
                  {/if}
                </span>
                <span class="cell comp" role="gridcell">
                  <!-- NO METER FOR A RATIO THAT DOES NOT EXIST — the rule
                       the Coverage tile already keeps. An empty track reads
                       as 0% and a full one as complete; both are claims. -->
                  {#if it.pct !== null}
                    <span class="meter" aria-hidden="true"
                      ><span class="fill" style="width:{it.pct * 100}%"></span></span
                    >
                  {/if}
                  <span class="cpct" title={it.pct === null ? SOLE_WHY : undefined}
                    >{pctText(it.pct)}</span
                  >
                </span>
                <!-- THE TWO PERCENTAGES. A number carries its sign, two
                     decimals and its direction's colour; an unknown is a
                     dash that NAMES ITSELF on hover and never a 0.00%.
                     `title` for the pointer, `aria-label` for everything
                     else — the reason has to reach both or the loud
                     degradation is only loud to one of them. -->
                <span class="cell num pc" role="gridcell" data-dir={dirOf(it.chg)}>
                  {#if it.chg === null}
                    <span class="unk" title={whyText(it.chgWhy)} aria-label={whyText(it.chgWhy)}
                      >—</span
                    >
                  {:else}
                    {bpsText(it.chg)}
                  {/if}
                </span>
                <span class="cell num pc" role="gridcell" data-dir={dirOf(it.prevChg)}>
                  {#if it.prevChg === null}
                    <span
                      class="unk"
                      title={whyText(it.prevChgWhy)}
                      aria-label={whyText(it.prevChgWhy)}>—</span
                    >
                  {:else}
                    {bpsText(it.prevChg)}
                  {/if}
                </span>
              </div>
            {/each}
          {/key}
        </div>
      {/if}
    </div>
  </div>

  {#if db.openRowData}
    <!-- ==============================================================
         THE DRILL-THROUGH, at the altitude the data supports. Not a
         chart — this store serves no price to this page — but every
         month this instrument holds, which is the question a row raises.
         ============================================================== -->
    <!-- A `div`, NOT an `aside`. `role="dialog"` on a complementary
         landmark is the a11y contradiction `svelte-check` names: this is a
         transient overlay the operator dismisses, not a standing region of
         the page. `aria-modal="false"` because it traps nothing — the
         table behind it stays readable and reachable. -->
    <div
      class="drawer panel-in"
      bind:this={db.drawerEl}
      role="dialog"
      aria-modal="false"
      aria-label="{db.openRowData.instrument} — every stored month"
    >
      <div class="dhead">
        <div class="dtitle">
          <span class="ihead">{db.openRowData.head}</span><span class="isym"
            >{db.openRowData.sym}</span
          >
        </div>
        <button class="link dclose" onclick={() => (db.openKey = null)}>close ·<kbd class="kbd">Esc</kbd></button>
      </div>

      <div class="dstats">
        <div class="dstat">
          <span class="k">Months held</span><span class="v">{fmt(db.openTotals.n)}</span>
        </div>
        <div class="dstat">
          <span class="k">Complete</span><span class="v"
            >{fmt(db.openTotals.complete)}{#if db.openTotals.unproven > 0}<span
                class="unproven"
                title={SOLE_WHY}>&nbsp;+{fmt(db.openTotals.unproven)} unverified</span
              >{/if}</span
          >
        </div>
        <div class="dstat">
          <span class="k">Bars</span><span class="v">{fmt(db.openTotals.bars)}</span>
        </div>
        <div class="dstat">
          <span class="k">Short by</span
          ><span class="v" class:risk={db.openTotals.missing > 0}>{fmt(db.openTotals.missing)}</span>
        </div>
      </div>

      <!-- WHICH DAYS THE OPENED MONTH ACTUALLY COVERS. This is the one
           place `/store.json`'s `first_ts` / `last_ts` are shown at full
           size, and this drawer is the right place for them: the owner
           named ONE instrument, and the per-instrument detail this page
           already has is the surface that answers about it. A second OHLC
           table would be `/markets` written twice, and `/store.json` serves
           no price to build one from.

           DEGRADES LOUDLY. A row without stamps says so and names the
           payload as the reason; it does not print the month's first and
           last calendar day, which are very often not trading days. -->
      <p class="dspan">
        <span class="k">Days held</span>
        {#if heldWindow(db.openRowData)}
          <span class="v" title="first_ts → last_ts from /store.json, microseconds, rendered IST"
            >{heldWindow(db.openRowData)}</span
          >
          <span class="n"
            >in <span
              title={`${monthLabel(db.openRowData.month)} — the store's own key for it is ${db.openRowData.month}`}
              >{monthLabel(db.openRowData.month)}</span
            ></span
          >
        {:else}
          <span class="v">—</span>
          <span class="n"
            >/store.json carried no first/last stamp for this row, so the days it covers are
            unknown — not the whole month, and not one day</span
          >
        {/if}
      </p>

      <div class="dlist">
        {#each db.openMonths as m (m.key)}
          <div class="drow" data-state={m.state} class:this={m.key === db.openKey}>
            <span
              class="dm"
              title={heldWindow(m)
                ? `${monthLabel(m.month)} · held ${heldWindow(m)} — the store's own key for it is ${m.month}`
                : `${monthLabel(m.month)} — the store's own key for it is ${m.month}`}
              >{monthLabel(m.month)}</span
            >
            {#if m.pct !== null}
              <span class="meter" aria-hidden="true"
                ><span class="fill" style="width:{m.pct * 100}%"></span></span
              >
            {/if}
            <span class="dpct" title={m.pct === null ? SOLE_WHY : undefined}
              >{pctText(m.pct)}</span
            >
            <span class="dshort" title={m.sole ? SOLE_WHY : undefined}
              >{#if m.sole}unverified{:else if m.short === 0}full{:else}−{fmt(m.short)}{/if}</span
            >
          </div>
        {/each}
      </div>

      <p class="dfoot">
        Each month is measured against the fullest instrument stored for that month —
        <span
          title={`${monthLabel(db.openRowData.month)} — the store's own key for it is ${db.openRowData.month}`}
          >{monthLabel(db.openRowData.month)}</span
        >
        {#if db.openRowData.sole}
          held {fmt(db.openRowData.denom)} bars at its fullest — and this row IS that fullest, and
          the only one stored at this rung for that month. The denominator is this row, so no
          share of the month can be stated for it. {SOLE_WHY}
        {:else}
          held {fmt(db.openRowData.denom)} bars at its fullest, so this row's {fmt(
            db.openRowData.rows
          )} is {pctText(db.openRowData.pct)} of the month.
        {/if}
      </p>
    </div>
  {/if}
</div>

{#snippet notesPanel(/** @type {any} */ db)}
  {#if db.noteOpen}
    <!-- ================================================================
         THE REFUSAL, IN FULL. `CLAUDE.md` §4: degrade loudly and name the
         reason. A column silently missing reads as forgotten; a column that
         states the endpoint, the cost, the decision that blocks it and the
         one server change that would unblock it reads as blocked — and the
         next person does not have to re-derive any of it.
         ================================================================ -->
    <section
      class="why panel-in"
      id="db-blocked"
      aria-label="What the percentage columns are, and why most cells are a dash"
    >
      <div class="whyhead">
        <span class="tag info">{fmt(db.numbered)} of {fmt(db.deco.length)} rows show a number</span>
        <h3>
          <b>% Change</b> and <b>Prev % Change</b> — integer basis points, or a dash that says
          which of five things it is.
        </h3>
        <button class="link" onclick={() => (db.noteOpen = false)}>close</button>
      </div>
      <ol class="whylist">
        {#each db.NOTES as b (b.k)}
          <li><b>{b.k}</b> <span>{b.v}</span></li>
        {/each}
      </ol>
      <p class="whyfoot">
        A dash is never a zero and never "nothing happened" — hover any one of them for the reason
        that cell in particular is unknown. Green and red mean DIRECTION and nothing else: they
        appear on a percentage and on a figure that moved between two reads of the store. A missing
        bar is a severity, not a direction, and wears amber.
      </p>
    </section>
  {/if}
{/snippet}

<style>
  /* ---- SEMANTIC COLOUR, STATED ONCE ----------------------------------
     `--up` and `--down` appear on this page in exactly two places: the tick
     flash, which marks a figure that MOVED between two reads, and the two
     percentage columns — which were RESERVING them and now use them.
     Everything about DATA QUALITY — short, near, gap — is a severity and uses
     `--warn` at two intensities. `.risk` is that hue with a name, so the
     intent is legible at the call site and cannot be mistaken for a direction.

     GREEN IS UP AND RED IS DOWN, which is the NSE convention every Indian
     broker and TradingView's India locale use — not the Japanese or mainland
     Chinese inversion. Stated because it is a convention and not a fact, and
     because a page that silently picked the other one would be unreadable to
     the only operator this is built for. */
  .risk {
    color: var(--warn);
  }

  /* ---- the completeness meter ---------------------------------------
     COLOUR IS THE SECOND SIGNAL, NEVER THE ONLY ONE. The track is hatched, so
     the missing portion has a TEXTURE and survives a greyscale print, a
     projector and every form of colour blindness; the fill is solid. The pip
     beside it is a third encoding — filled, half, hollow — because a 6px bar
     at a glance is a length and not a state. */
  .meter {
    position: relative;
    display: block;
    height: 6px;
    border-radius: var(--r-full);
    background-color: var(--well);
    background-image: repeating-linear-gradient(
      135deg,
      transparent 0 3px,
      var(--line-hard) 3px 4px
    );
    overflow: hidden;
    flex: none;
  }
  .meter .fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: var(--r-full);
    background: var(--n7);
  }
  [data-state='full'] .meter .fill{
    background: var(--n7);
  }
  [data-state='near'] .meter .fill{
    background: color-mix(in srgb, var(--warn) 55%, var(--n7));
  }
  [data-state='gap'] .meter .fill{
    background: var(--warn);
  }

  .pip {
    width: 9px;
    height: 9px;
    flex: none;
    border-radius: 2px;
    border: 1.5px solid var(--n7);
    background: var(--n7);
  }
  [data-state='near'] .pip {
    border-color: var(--warn);
    background: linear-gradient(90deg, var(--warn) 0 50%, transparent 50% 100%);
  }
  [data-state='gap'] .pip {
    border-color: var(--warn);
    background: transparent;
  }

  /* ---- the contract rung ----------------------------------------------
     THE SAME BOX AS THE FACET BAR AND THE UNIVERSE RUNG, declared with the same
     tokens and no second spacing vocabulary — `.strip` already carries the
     frame, the gradient and the `overflow: visible` the panels hang out of, and
     this adds only the one thing that differs.

     ------------------------------------------------------------------------
     THE CHECKBOX PAINT BUG THE MOCKUP CARRIED CANNOT OCCUR HERE, and this is
     the record of the check rather than an assumption.

     In the mockup the ladder lived inside `<div class="cell mcell">`, so the
     stylesheet's own `.cell select,.cell input` reached every input in the
     panel and set `appearance:none`, `background:transparent` and `border:0`.
     The rule below it re-declared `accent-color`, `width`, `height` and
     `cursor` at the same specificity and won — so every rung carried a real,
     correctly sized, fully clickable checkbox made of transparent pixels with
     no border and no tick. Present, placed, unpainted.

     Three facts make that impossible on this page, not a control cell. The controls live in
          `.qcell`, and neither selector has an `input` or `select` descendant
          rule at all.
       3. THE CHECKBOX IS NOT IN THIS FILE'S SCOPE. It is drawn by
          `$lib/Picker.svelte`, whose `.plist input` sets `accent-color`,
          `width`, `height` and `cursor` and never touches `appearance`,
          `background` or `border`. Svelte scopes styles per component, so no
          rule written here can reach it — which is the structural reason, not
          a lucky ordering.

     The one GLOBAL rule that does reach it is `theme.css`'s `* { box-sizing;
     margin: 0; padding: 0 }`. It changes a checkbox's box and not its paint:
     `appearance` is untouched, so the UA still draws the control. Nothing is
     re-declared here to "fix" that, because a rule written against a bug this
     page does not have is a rule nobody can later tell from one that matters.
     ---------------------------------------------------------------------- */
  /* ---- THE MONTH WINDOW'S OWN CONTROL, AND WHY NONE OF IT IS LEFT ------
     What stood here styled a value that read as a value and a button that
     opened a calendar. /db has neither. The month control went with the
     from/to range — see the markup note at THE MONTH WINDOW — and both ends
     of the day window are `$lib/DayField.svelte` now.

     `.dcell` WENT WITH THE MARKUP IT POSITIONED. It was the last rule in this
     block still matching anything, and only because /db's own day inputs
     borrowed the class; `DayField` owns that element now and carries its own
     `position: relative`. Its siblings had been dead since the month calendar
     was removed. That is the sentence `26a45dd0` wrote here — and the
     sentence `126152af` then deleted, together with the closing terminator
     that ended it and the `.dwrap` selector line beneath. One contiguous cut —
     and `126152af` made it NINE times across this file. Eight were still open
     when this repair began, and Gate W5 could name only three of them, because
     it prints one finding per comment and seven of the eight are crowded into
     two comments. The terminator cannot be shown to you inside this sentence:
     a CSS comment has no escape, and the star-slash pair ends the comment
     wherever it appears.

     FIVE RULE BODIES WERE ORPHANED, NOT ONE. That cut landed on `.dwrap`,
     `.dval.unset`, `.cal`, `.cmon:disabled` and `.calft`, so everything from
     `.dcell`'s note down to the refusal notice below was a single comment
     body: five rule bodies, thirty-one declarations, emitted by no stylesheet
     since that commit. Gate W5 prints one finding per comment, which is why
     it named only the first of them.

     THEY ARE DELETED RATHER THAN GIVEN THEIR SELECTORS BACK. The selectors
     are recoverable from git; the markup is not. `dwrap`, `dval`, `cal`,
     `calhd`, `calnav`, `calsel`, `calgrid`, `cmon`, `calft`, `dbtn`, `dcell`
     and `dright` occur ZERO times in this file outside comment prose — no
     element, no `class:` directive, no string a class name is built from.
     Restoring them would bring back styling for a control nothing builds,
     which is how a removed control returns without anyone deciding to bring
     it back. Nothing on screen moves: these have painted nothing for as long
     as the defect has existed.
     ---------------------------------------------------------------------- */

  /* ---- the refusal notice --------------------------------------------- */
  .why {
    flex: none;
    padding: var(--s5) var(--s6);
    border: 1px solid color-mix(in srgb, var(--info) 32%, var(--line));
    border-radius: var(--r4);
    background: var(--panel);
    box-shadow:
      inset 3px 0 0 var(--info),
      var(--e1);
  }
  .whyhead {
    display: flex;
    align-items: center;
    gap: var(--s4);
    flex-wrap: wrap;
    margin-bottom: var(--s4);
  }
  .whyhead h3 {
    font-size: var(--fs-md);
    font-weight: var(--w-semi);
    flex: 1;
    min-width: 12rem;
  }
  .whyhead h3 b {
    font-family: var(--mono);
    color: var(--ink-hi);
  }
  .whylist {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(19rem, 1fr));
    gap: var(--s4) var(--s6);
    list-style: none;
    counter-reset: why;
  }
  .whylist li {
    counter-increment: why;
    font-size: var(--fs-sm);
    line-height: 1.55;
    color: var(--dim);
    padding-left: var(--s6);
    position: relative;
  }
  .whylist li::before {
    content: counter(why);
    position: absolute;
    left: 0;
    top: 1px;
    font-family: var(--mono);
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    color: var(--info);
  }
  .whylist li b {
    color: var(--ink);
    font-weight: var(--w-semi);
  }
  .whyfoot {
    margin-top: var(--s5);
    padding-top: var(--s4);
    border-top: 1px solid var(--line-soft);
    font-size: var(--fs-xs);
    color: var(--faint);
    line-height: 1.6;
  }

  /* ---- the windowed table --------------------------------------------
     A grid of elements rather than a `<table>`, for one reason: windowing
     needs an EXACT row height, and a table row's height is negotiated between
     collapsed borders, cell padding and line-height. Here it is
     `var(--dbrow)`, stated once, and the arithmetic that places row N at
     `N * 32px` cannot drift from what the browser draws. ARIA carries the
     grid semantics that the markup no longer states. */
  /* THE TABLE IS THE LAST AND LARGEST PANEL. `overflow: hidden` is what keeps
     the sticky header and the drawer inside the rounded corners; the scrolling
     happens one level in, on `.tbl-scroll`, so nothing about the windowing
     changes. */
  .tbl {
    flex: 1;
    /* A FLOOR, NOT `0`. `min-height: 0` lets this flex child give up ALL of its
       height when the siblings above it want more, and `.board` is a
       fixed-height flex column — so on a short viewport the strip, the contract
       section, the anchor and the pager between them took 709 of 727 pixels and
       this box was left with NINETEEN. The table inside was 2,030px tall and
       fully rendered; fifty rows sat in the DOM and none of them was on screen,
       under a pager cheerfully reporting "1–50 of 4,500".
       Measured, and it is why a folded Contract strip appeared to fix a table
       that was never broken. `min-height: 0` is still what stops a flex child
       from refusing to shrink at all, so the floor replaces it rather than
       removing it: shrink, yes — to nothing, no. Past the floor the page
       scrolls, which is the honest outcome.

       THE FLOOR WAS 220px, AND THAT WAS A SURVIVAL HEIGHT, NOT A USEFUL ONE.
       ---------------------------------------------------------------------
       220 was chosen when NOTHING on this page could scroll: it was the last
       resort that stopped the table disappearing entirely, and at that job it
       worked. It was never a size anyone would choose for the thing the page
       exists to show.

       MEASURED at a 900px viewport, before this line changed: 477px of chrome
       above — two strips, the anchor, the facts row and the window band — left
       the table at its 220px floor with a 201px scrolling area. At 40px a row
       that is FIVE ROWS VISIBLE out of fifty rendered, on a screen tall enough
       for twenty. The table had 22% of the viewport and the controls above it
       had the rest.

       Now that `.board` scrolls (see its own note), the table no longer has to
       fit in what the chrome leaves. The page scrolls to reach whatever will
       not fit, which is the arrangement the pager already assumes, and
       `flex: 1` still lets it take MORE where a tall window offers it.

       THE CEILING WAS 680px AND A REAL WINDOW IS TALLER THAN THAT.
       Measured on the operator's own screen — a maximised window, not the
       900px test viewport — 72vh comes to about 930px and the 680 cap threw
       the difference away: the box stayed letterbox-shaped on a display with
       room for twice the rows. A cap that binds on every real monitor is not a
       safety rail, it is the size.

       `min(80vh, 1100px)` keeps a cap for the case the vh term is written for —
       a very tall or rotated display, where 80vh would otherwise leave no room
       for the pager under it — while letting an ordinary large screen give the
       table the space it obviously has. On a 900px viewport this is 720px, and
       on a 1300px one it is 1040px: about twenty-six rows, which is what a
       reader of a price table is actually looking for.
       --------------------------------------------------------------------- */
    /* AND NOW THE FLOOR IS A ROW COUNT, WHICH IS THE UNIT THE READER ASKED IN.
       ---------------------------------------------------------------------
       Every number above is the history of a box guessing at a height — 220px
       to stop it vanishing, then 680, then min(80vh, 1100px), then no floor at
       all when the box grew to all fifty rows and the page scrolled 2,030px to
       pass them. Each was a PIXEL answer to a question asked in ROWS, which is
       why none of them survived contact: nobody looking at a price table wants
       "eighty percent of the viewport", they want to see twenty-five rows and
       reach the next page without hunting for the button.

       So the floor is written in rows, and it is the same two constants the
       windowing arithmetic uses — `ROW` and `HEAD` in the script — rather than
       a third number that could disagree with them. Twenty-five rows plus the
       sticky header is 1,030px at the current 40px row.

       `flex: none`, AND "STATIC" IS THE WORD THE REQUEST USED. An earlier pass
       here wrote `flex: 1` so a tall window would give the box MORE than 25
       rows. That is a different feature and a worse one: a box that grows with
       the viewport puts the pager somewhere different on every screen, and
       turning pages is the frequent act this layout is being cheap for. 25 rows
       always means the pager is always in the same place.

       IT ALSO BROKE, WHICH IS HOW THE POINT GOT MADE. `flex: 1` here with
       `overflow: hidden` below is a trap worth naming: a flex item's automatic
       minimum size — the rule that stops it shrinking under its own content —
       applies ONLY while overflow is `visible`. `hidden` resolves `min-height:
       auto` to zero, so `flex: 1` shrank this box to the 484px the chrome left
       and the 1,030px scroll box inside it was CLIPPED. Measured: the page
       reported nothing to scroll and the pager sat above the fold, both true,
       both because fourteen rows had been thrown away rather than fitted.
       A layout that reports success by discarding its content is the failure
       wearing a success's clothes that `CLAUDE.md` §4 bans.

       AND IT DOES NOT ALWAYS FIT, WHICH IS THE HONEST PART. `min-height` on a
       flex child is a promise the PARENT pays for: when 25 rows plus the chrome
       exceeds `.board`, the board overflows and the page scrolls the
       difference. Measured on a 1,200px viewport after the trims in this
       commit, the page runs to about 1,550px, so roughly 350 of it scrolls.
       "Twenty-five rows minimum" and "nothing scrolls" are the same constraint
       pulled in opposite directions, and on a short window the row count is the
       one that was asked for. Everything fits outright at a window height of
       about 1,550px.
       --------------------------------------------------------------------- */
    /* THE FLOOR IS ON `.tbl-scroll`, NOT HERE, AND THE FIRST ATTEMPT PUT IT
       HERE AND CAME UP HALF A ROW SHORT. Measured: `.tbl` obeyed 1,030px
       exactly, then spent 17 of them on `.bnotes` and 2 on its own border, so
       the scroll box got 1,011 and showed 24.5 rows under a comment promising
       25. The floor is a statement about how many rows the SCROLLER shows, so
       it belongs on the scroller; this box is then whatever that needs plus its
       own furniture, and a note added under the table later cannot silently
       eat a row. */
    /* ---------------------------------------------------------------------
       AND THE HEIGHT IS THE SCREEN'S TO GIVE, NOT THIS BOX'S TO DEMAND.

       A 25-row box is 1,030px. On a 1,200px window the chrome above leaves
       about 500. Asking for 1,030 of a 500px hole does not fail loudly — it
       hangs 461px of table BELOW THE FOLD and starts a second scrollbar to
       reach it, which is the exact arrangement this page was told twice to
       stop having. MEASURED in that state: two scrollers, and THIRTEEN rows on
       screen under a box whose CSS said twenty-five. The number was true of the
       box and false of the screen, and only the screen counts.

       So the box takes what is left and no more. `flex: 1` against `.board`'s
       remaining height, `min-height` only as a floor against vanishing —
       not as a row count, because a row count is a promise about a viewport
       this rule cannot see. How many rows that comes to is MEASURED at runtime
       and the page size is set from it, so the grid holds exactly the rows that
       fit and the box never scrolls at all. See `rowsThatFit`.

       220px IS A SURVIVAL FLOOR AND NOT A PROMISE. It exists only so a very
       short window degrades to a small table rather than to the nineteen-pixel
       one this file recorded earlier. It is about five rows.

       IT WAS 320 AND THAT TURNED OUT TO BE THE LAST THING HOLDING THE SCROLL
       OPEN. Measured at an 800px window with the readouts already folded away:
       the box sat ON the 320 floor and the page still overflowed by 35px — a
       range far too short to be worth scrolling and long enough to lurch, which
       is the whole complaint. The floor was not protecting a readable table at
       that point, it was buying three rows at the price of the page not
       fitting, which is the wrong side of the trade this page has now made
       three times. Below 220 the page scrolls, which is the honest outcome and
       needs a window shorter than any this was tested on.
       --------------------------------------------------------------------- */
    flex: 1;
    min-height: 220px;
    display: flex;
    flex-direction: column;
    position: relative; /* the drawer's containing block */
    border: 1px solid var(--line);
    border-radius: var(--r4);
    /* `hidden` AGAIN, AND NOW IT IS WANTED RATHER THAN TOLERATED. With the
       rows scrolling inside `.tbl-scroll` once more, the sticky `<th>` should
       pin to THAT box — which is the whole point of a fixed-height table — so
       an ancestor clip here is no longer the trap it was when the header had to
       see the page. It buys back the rounded corners the drawer and the header
       were spilling out of. */
    overflow: hidden;
    background: var(--panel);
    box-shadow: var(--e2);
  }
  .tbl-scroll {
    /* TWENTY-FIVE ROWS AND THE STICKY HEADER — `height`, NOT `min-height`, AND
       BOTH WRONG ANSWERS WERE TRIED FIRST.
       ---------------------------------------------------------------------
       `--dbrow` and `--dbhead` are the same two measurements the windowing
       arithmetic reads as `ROW` and `HEAD`, so the box cannot promise a row
       count the script places differently. That part was right from the start;
       the PROPERTY took two goes.

       `min-height` with `flex: 1` on `.tbl` CLIPPED — `overflow: hidden` there
       zeroes a flex item's automatic minimum size, so the panel shrank to 484px
       and cut fourteen rows off the bottom.
       `min-height` with `flex: none` on `.tbl` OVERGREW — a floor is not a
       ceiling, nothing capped the box, and it stood at 2,030px with all fifty
       rows and an inner scroll range of ZERO. Measured both.

       AND THE THIRD ANSWER IS NEITHER: THE BOX FILLS `.tbl` AND `.tbl` FILLS
       WHAT THE SCREEN LEFT. A fixed `height: calc(25 * var(--dbrow) + ...)`
       stood here and was measured hanging 461px below the fold with a second
       scrollbar to reach it. Any height stated HERE is a guess about a viewport
       this rule cannot see; the only honest source for it is the viewport
       itself, so `.tbl` reads it via `flex` and this box takes all of `.tbl`.

       `min-height: 0` IS BACK AND IS NOW LOAD-BEARING IN THE OTHER DIRECTION:
       a flex child will not shrink under its own content without it, and this
       box's content is fifty rows. It must shrink; the rows are what scrolls —
       except that `rowsThatFit` sets the page size from this box's measured
       height, so in practice there is nothing to scroll and the range is zero.
       The overflow stays `auto` rather than `hidden` so the rare case that
       overruns it — a short window under the 320px floor — degrades to a
       scrollbar instead of to hidden rows. */
    flex: 1;
    min-height: 0;
    /* ---------------------------------------------------------------------
       THIS SCROLLS AGAIN, AND THE ROUND TRIP IS WORTH STATING PLAINLY BECAUSE
       BOTH ARRANGEMENTS ARE DEFENSIBLE AND ONLY ONE ANSWERS THIS PAGE.

       It scrolled originally by ACCIDENT: `.board` could not scroll at all, so
       the rows were the only thing that could, and reaching the last one
       stopped the wheel dead with the pager stranded below. That was fixed
       twice — first by letting the scroll chain out (d34bd03), then by removing
       this scroller entirely so the page held exactly one (8692c60).

       ONE SCROLLER WAS THE RIGHT FIX FOR THE WRONG COMPLAINT. It did make the
       page legible — no more guessing which box the wheel was driving — but it
       bought that by letting the table grow to all fifty rows, 2,030px, which
       put the pager 2,030px below the header. The reader who wants page 6,001
       now scrolls the length of a page they did not want in order to leave it.
       Turning pages is the frequent act on this surface; reading to the bottom
       of one is the rare act. The layout should be cheap for the frequent one.

       So it scrolls, by CHOICE this time, inside a box with a floor written in
       rows — see `.tbl`. Twenty-five rows is enough to read and short enough
       that the pager stays near the fold. The `<th>`s pin to this box, which is
       what a fixed-height table is for.

       THE PAGE IS STILL A SCROLLER TOO, and that is unavoidable rather than a
       regression: 25 rows plus the chrome exceeds a 1,200px window. What has
       changed is that the inner box is now a KNOWN size the reader can learn,
       instead of one that grew with the row count, and the page beneath it
       scrolls ~350px instead of ~1,700px. `overscroll-behavior` is deliberately
       NOT set: the chain from the last row into the page is what makes two
       scrollers survivable, and `contain` is what broke it the first time.
       --------------------------------------------------------------------- */
    overflow: auto;
  }
  .tbl-scroll:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* 1044px WAS THE OLD SUM AND IT IS RE-DERIVED, NOT NUDGED. The two
     percentage columns went from 96px of dead ground each to 116px of signed
     six-to-nine-character numbers under a `Prev % Change` heading, so the
     table's natural width is 224 + 92 + 48 + 104 + 86 + 128 + 150 + 116 + 116
     = 1064. Below that the table scrolls horizontally and the instrument
     column pins, exactly as before. */
  .tbl-inner {
    min-width: 1064px;
    position: relative;
  }

  /* THE PADDING LIVES IN THE CELLS, NOT ON THE ROW. A sticky column pins to
     the scroll box's left edge; with padding on the row the first cell starts
     12px in and, once pinned, leaves a 12px gutter for the other columns to
     scroll through. Moving the padding inward means the sticky cell's own box
     reaches the edge and the seam is clean. */
  .thead,
  .trow {
    display: grid;
    grid-template-columns:
      minmax(224px, 2.2fr) 92px 48px 104px 86px 128px
      minmax(150px, 1fr) 116px 116px;
    align-items: center;
  }
  .th,
  .trow .cell {
    padding-right: var(--s5);
  }
  .th.inst,
  .trow .cell.inst {
    padding-left: var(--s5);
  }

  /* 30px, AND IT IS THE `HEAD` THE CURSOR ARITHMETIC USES. `moveTo` subtracts
     this height to keep the focused row clear of the sticky header; the two
     numbers are one measurement written in two languages and neither moves
     alone. */
  .thead {
    position: sticky;
    top: 0;
    z-index: 3;
    height: 30px;
    background: linear-gradient(180deg, var(--panel-2), var(--bg-2));
    border-bottom: 1px solid var(--line);
  }
  .th {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    min-width: 0;
  }
  .th.numh {
    text-align: right;
  }
  /* IDENTITY DOES NOT LEAVE THE SCREEN. Below ~1044px the table scrolls
     horizontally — measured, not theoretical — and without this the
     instrument name is the first thing to go, leaving nine columns of numbers
     belonging to nothing. */
  .th.inst,
  .trow .cell.inst {
    position: sticky;
    left: 0;
    z-index: 2;
    background: inherit;
  }
  /* THE PINNED HEADER CELL REPAINTS THE HEAD'S OWN GRADIENT, not a flat
     approximation of it. Both boxes are 30px tall and the gradient runs top to
     bottom, so a 224px slice and a 1064px one are the same pixels — a flat
     colour here would show as a seam down the pinned column the moment the
     table scrolls sideways. */
  .th.inst {
    background: linear-gradient(180deg, var(--panel-2), var(--bg-2));
  }
  /* THE SEAM IS AN INHERITED CUSTOM PROPERTY, NOT A DESCENDANT OVERRIDE.
     Driving `opacity` from `.tbl-scroll.xscrolled .cell.inst::after` put the
     switch and the thing switched in different rules whose cascade had to be
     reasoned about — and it lost silently: the class was on the element, the
     compiled rule matched it, and the pseudo-element stayed at 0. A custom
     property inherits down to the pseudo-element unconditionally, so there is
     one declaration site and nothing to out-specify. */
  .tbl-scroll {
    --seam: transparent;
  }
  .tbl-scroll.xscrolled {
    --seam: var(--line-hard);
  }
  .th.inst::after,
  .trow .cell.inst::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    right: 0;
    width: 1px;
    background: var(--seam);
  }
  .sortbtn {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    cursor: pointer;
    white-space: nowrap;
  }
  /* THE LABEL STAYS FLUSH RIGHT. The caret holds its 7px whether it is lit or
     not — reserving the space is what stops the header jumping sideways when
     a column is sorted — so on a numeric column it goes to the LEFT of the
     label, and the header text lands on the same pixel as the digits under
     it. Right-aligning the whole button instead pushed every numeric heading
     11px off its column. */
  .th.numh .sortbtn {
    justify-content: flex-end;
    flex-direction: row-reverse;
  }
  .sortbtn:hover {
    color: var(--ink);
  }
  .sortbtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
    border-radius: var(--r1);
  }
  /* THE DIRECTION IS ON SCREEN. A dimmed caret on every column says "this
     sorts"; the lit one says which way, and `aria-sort` says the same thing
     to a reader that cannot see it. */
  .caret {
    width: 0;
    height: 0;
    border-left: 3.5px solid transparent;
    border-right: 3.5px solid transparent;
    border-bottom: 5px solid currentColor;
    opacity: 0;
    flex: none;
  }
  .sortbtn:hover .caret {
    opacity: 0.35;
  }
  .caret.on {
    opacity: 1;
    color: var(--acc);
  }
  .caret.on.desc {
    transform: rotate(180deg);
  }

  .tspace {
    position: relative;
  }
  .trow {
    position: absolute;
    left: 0;
    right: 0;
    height: var(--dbrow);
    border-bottom: 1px solid var(--line-soft);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    /* OPAQUE ON PURPOSE. `.cell.inst` inherits this background to sit over the
       columns sliding beneath it; a translucent row would let them show
       through the pinned column. */
    background: var(--bg-2);
  }
  .trow.odd {
    background: var(--panel-2);
  }
  .trow:hover {
    background: color-mix(in srgb, var(--acc) 7%, var(--bg-2));
  }
  /* OPAQUE, NOT `--acc-soft`. The soft tokens are alpha colours, and the
     pinned instrument cell inherits this background to cover the columns
     sliding beneath it — a translucent cursor row let the Month column show
     straight through the instrument name. Measured at scrollLeft 224. */
  .trow.cur {
    background: color-mix(in srgb, var(--acc) 14%, var(--bg-2));
    box-shadow: inset 0 0 0 1px var(--acc);
  }
  .trow.open {
    background: color-mix(in srgb, var(--acc) 14%, var(--bg-2));
  }
  /* A WHOLE SESSION MISSING GETS AN EDGE, AND IT TRAVELS WITH THE PINNED
     COLUMN. Rows with a partial-day gap get a lighter one: the marker has to
     rank, or it means nothing. Amber at two intensities — never red, which on
     this page means a number went down. */
  .trow .cell.inst {
    box-shadow: inset 3px 0 0 transparent;
  }
  .trow[data-state='near'] .cell.inst {
    box-shadow: inset 3px 0 0 color-mix(in srgb, var(--warn) 45%, transparent);
  }
  .trow[data-state='gap'] .cell.inst {
    box-shadow: inset 3px 0 0 var(--warn);
  }

  .trow .cell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* EVERY NUMERIC COLUMN IS TABULAR AND RIGHT-ALIGNED — no exceptions, so a
     digit in one row sits over the same digit in the next. */
  .trow .cell.num,
  .th.numh {
    font-variant-numeric: tabular-nums;
  }
  .trow .cell.num {
    font-family: var(--mono);
    text-align: right;
  }
  .trow .cell.mono,
  .trow .cell.tf {
    font-family: var(--mono);
    color: var(--dim);
  }
  .trow .cell.dimnum {
    color: var(--dim);
  }

  /* ---- THE TWO PERCENTAGE COLUMNS ------------------------------------
     Right-aligned and monospaced through `.cell.num`, so every glyph — the
     sign included — occupies one cell of the same grid and the decimal point
     of `+1.25%` lands on the decimal point of `-12.50%` in the row below.
     Two decimals always, from `bpsText`; nothing here is adaptive. */
  .trow .cell.pc[data-dir='up'] {
    color: var(--up);
  }
  .trow .cell.pc[data-dir='down'] {
    color: var(--down);
  }
  /* A FLAT MONTH IS NOT A DIRECTION. It is a real 0.00% and it reads as one:
     dimmed, so a screen of them does not compete with the moves. */
  .trow .cell.pc[data-dir='flat'] {
    color: var(--dim);
  }
  /* THE DASH IS AN INVITATION, NOT A SHRUG. The dotted underline and the help
     cursor say there is a reason behind it — a bare em-dash in a numeric
     column reads as "zero, rendered lazily", which is the one thing it must
     never be mistaken for. The reason itself is on `title` and `aria-label`.
     COLOUR IS NOT THE ONLY SIGNAL: the dash's shape and its underline carry
     the state with no hue at all, which is what makes the column legible in
     greyscale and under every form of colour blindness. */
  .unk {
    color: var(--faint);
    cursor: help;
    border-bottom: 1px dotted currentColor;
    padding-bottom: 1px;
  }
  .inst {
    display: flex;
    align-items: center;
    gap: var(--s3);
  }
  /* The store's own key stays whole and copyable; the part that repeats on
     every row is dimmed so the eye lands on the symbol. Rendered as text
     nodes, never markup — `M&M` is a legal symbol. */
  .ihead {
    color: var(--faint);
    font-family: var(--mono);
    font-size: var(--fs-xs);
  }
  .isym {
    font-family: var(--mono);
    font-weight: 650;
    color: var(--ink);
  }

  .short {
    display: flex;
    align-items: baseline;
    justify-content: flex-end;
    gap: var(--s3);
  }
  .okword {
    color: var(--dim);
    font-family: var(--sans);
    font-size: var(--fs-xs);
    letter-spacing: 0.04em;
  }
  /* NOT A SEVERITY AND NOT A PASS. `unverified` is the absence of a
     comparison, so it wears neither the amber of a hole nor the plain weight
     of `full`: it is dimmed and italic, and it carries its reason on hover. */
  .unproven {
    color: var(--dim);
    font-family: var(--sans);
    font-size: var(--fs-xs);
    font-style: italic;
    letter-spacing: 0.04em;
  }
  .missnum {
    color: var(--warn);
  }
  .lost {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    padding: 1px 4px;
    border-radius: 3px;
    background: var(--warn-soft);
    color: var(--warn);
  }
  .comp {
    display: flex;
    align-items: center;
    gap: var(--s4);
  }
  .comp .meter {
    flex: 1;
    min-width: 40px;
  }
  .cpct {
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--dim);
    width: 4.6rem;
    text-align: right;
    flex: none;
  }
  [data-state='near'] .cpct,
  [data-state='gap'] .cpct {
    color: var(--warn);
  }

  .tnone {
    padding: var(--s8) var(--s6);
    text-align: center;
  }
  .tnone h3 {
    font-size: var(--fs-md);
    margin-bottom: var(--s3);
  }
  .tnone p {
    color: var(--dim);
    font-size: var(--fs-sm);
    margin-bottom: var(--s5);
    line-height: 1.6;
  }
  /* THE GATE'S OWN LINE, AND IT IS THE ONE MESSAGE ON THIS PANEL THAT IS NOT
     GREY. Everything else here explains a filter the button below undoes; this
     explains rows the button CANNOT release, so it is drawn as the warning it
     is rather than as another line of the same paragraph. Boxed and bounded so
     a long sentence cannot run the width of a 1560px table. */
  .tnone .tgate {
    max-width: 46rem;
    margin: 0 auto var(--s5);
    padding: var(--s4) var(--s5);
    border: 1px solid color-mix(in srgb, var(--warn) 45%, var(--line));
    border-radius: var(--r2);
    background: color-mix(in srgb, var(--warn) 8%, transparent);
    color: var(--warn);
    font-size: var(--fs-sm);
    line-height: 1.6;
  }
  .tnone .tgate b {
    font-weight: var(--w-bold);
  }

  /* ---- the row drill-through ------------------------------------------ */
  .drawer {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(24rem, 92%);
    z-index: 5;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border-left: 1px solid var(--line-hard);
    box-shadow: var(--e3);
  }
  .dhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    padding: var(--s4) var(--s5);
    border-bottom: 1px solid var(--line);
    background: var(--bg-2);
    flex: none;
  }
  .dtitle {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-md);
  }
  .dclose {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    white-space: nowrap;
  }
  .dstats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1px;
    background: var(--line);
    border-bottom: 1px solid var(--line);
    flex: none;
  }
  .dstat {
    background: var(--panel);
    padding: var(--s4) var(--s5);
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .dstat .k {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .dstat .v {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-md);
    font-weight: var(--w-bold);
  }
  /* ---- the day window, under the four counters ------------------------
     ONE LINE, NOT A FIFTH TILE. `.dstats` is a four-column grid and a fifth
     counter would leave three columns of empty beside it; and the window is a
     RANGE rather than a count, so it does not read as one of them anyway. */
  .dspan {
    flex: none;
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--s3);
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }
  .dspan .k {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .dspan .v {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-xs);
    color: var(--ink);
  }
  .dspan .n {
    font-size: var(--fs-mini);
    color: var(--faint);
    line-height: 1.5;
  }

  .dlist {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .drow {
    display: grid;
    grid-template-columns: 4.6rem 1fr 4.6rem 5.4rem;
    align-items: center;
    gap: var(--s4);
    padding: 0 var(--s5);
    height: 28px;
    border-bottom: 1px solid var(--line-soft);
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
  }
  .drow.this {
    background: var(--acc-soft);
    box-shadow: inset 3px 0 0 var(--acc);
  }
  .dm,
  .dpct,
  .dshort {
    font-family: var(--mono);
  }
  .dpct,
  .dshort {
    text-align: right;
    color: var(--dim);
  }
  [data-state='near'] .dshort,
  [data-state='gap'] .dshort {
    color: var(--warn);
  }
  .dfoot {
    flex: none;
    padding: var(--s4) var(--s5);
    border-top: 1px solid var(--line);
    background: var(--bg-2);
    color: var(--faint);
    font-size: var(--fs-mini);
    line-height: 1.55;
  }

  /* ---- THE PAGER ------------------------------------------------------
     A BAR UNDER THE TABLE AND NOT A LINE ON THE BOARD, which is the one place
     this page follows the approved design rather than the rule it used to hold:
     the design puts the pager inside the grid's own frame, and a footer that
     belongs to the table has to look like it does. It carries no page buttons —
     there are no pages: the table is WINDOWED, the scrollbar is the full height
     of the sorted set, and a row of numbered buttons would be a control with
     nothing to point at. Everything a pager's `.of` says is here: where you are
     standing in the run, how much of it is real right now, how fresh it is, and
     the keys that move you through it. */
  /* `.pager`, `.pager .of` AND `.pager .of b` ARE GONE WITH THE ELEMENT THEY
     STYLED — see the markup note where the second pager stood. Svelte reports
     an unused selector as a warning rather than an error, so dead rules here
     survive a build and accumulate; three of them for one deleted `<p>` is
     exactly the drift worth not starting. `.pgbar` carries the surviving pager
     and its own rules are untouched. */
  .dclose .kbd {
    margin: 0 1px;
  }

  /* ---- MOTION. Every transition on this page is in here, so a reader who
     asked for less motion gets a completely static one. */
  @media (prefers-reduced-motion: no-preference) {
    /* THE CARET IS NOT IN HERE, DELIBERATELY. It was, and its opacity was
       animated from 0 to 1 — which meant the sort indicator was a
       *transition* rather than a *state*, and a transition can be left
       unfinished: with the tab backgrounded mid-animation the compositor
       stopped advancing it and the lit caret stayed on the previously sorted
       column while `aria-sort` had already moved. The page then said one
       thing to a screen reader and the opposite to an eye. A state indicator
       has to be right on the frame it changes, so it changes on that frame. */
    .sortbtn{
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        color var(--d-hover) var(--ease-out),
        opacity var(--d-hover) var(--ease-out),
        box-shadow var(--d-state) var(--ease-out);
    }
    /* A ROW ANIMATES ITS HOVER TINT AND NOTHING ELSE. Its `box-shadow` is the
       severity spine — a state, by the same argument as the caret — so it is
       left out of the list rather than faded between two meanings. */
    .trow {
      transition: background-color var(--d-hover) var(--ease-out);
    }
    /* ROW ENTRY IS AN OPACITY FADE AND NOT A SLIDE, and that is load-bearing
       rather than taste: `theme.css`'s `.row-in` translates on the Y axis, and
       a transform on the row makes the row the containing block for its own
       `position: sticky` cell — the pinned instrument column would come
       unpinned for the 220ms the animation runs, every time the set changed. */
    .trow.row-in {
      animation: db-row-in var(--d-enter) var(--ease-out) backwards;
    }
    .th.inst::after,
    .trow .cell.inst::after {
      transition: background-color var(--d-state) var(--ease-out);
    }
    /* The bar GROWS to its new value. A pull that lands 375 bars moves every
       meter it touched, which is the difference between a page that reports
       and a page that is watched. */
    .meter .fill {
      transition:
        width var(--d-state) var(--ease-out),
        background-color var(--d-state) var(--ease-out);
    }
  }

  @keyframes db-row-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
