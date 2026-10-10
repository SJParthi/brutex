<script module>
  /**
   * /db — THE BAR GRID AND THE PAGER
   *
   * One row per stored bar, read from `/bars.json` and `/bars/window.json`, and the
   * pager (`pager`, rendered by the shell under either grid).
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
   * Owner after WP-00: WP-52 (bars data path, pager, go-to, extremes, export).
   */
  import { ROW, HEAD, FIT_SLACK, MIN_USEFUL_ROWS, istDayKey, pooled, IN_FLIGHT, WINDOW_SORTS, paisaText, daysBetween, minuteOf, MIN_SPINE, CHG_FLOOR_BPS, SKELETON, PAGE_SIZES, pageList } from '$lib/db/bars-plan.js';
  import { untrack } from 'svelte';
  import { TF_SECS, txt, tfCmp, fmt } from '$lib/db/notes-labels.js';
  import { BAR_COLS, FUTURE_COLS, OPTION_COLS, BAR_GROUPS, BAR_META, BAR_UNGROUPED, barWhyText } from '$lib/db/notes-bars.js';
  import { dayWindowApplies } from '$lib/db/notes-controls.js';
  // A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
  // ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
  // threaded through every call site.
  import { ask } from '$lib/ask.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { readDatabasePage } from '$lib/database-pages.js';
  import { basisPoints, dirOf, bpsText } from '$lib/bps.js';
  import { timeLabel, stampLabel, dayLabel, barTimeLabel } from '$lib/dates.js';
  import { refreshStore } from '$lib/store.svelte.js';
  import Picker from '$lib/Picker.svelte';
  import DbChart from './DbChart.svelte';
  import DbExtremes from './DbExtremes.svelte';
  import DbDayFlags from './DbDayFlags.svelte';
  import DbOptionCells from './DbOptionCells.svelte';
  import DbProvenance from './DbProvenance.svelte';

  /**
   * THE BAR GRID'S MEASURED FIT — the box height, the fold and the automatic
   * compact mode.
   *
   * Creates the page's effects that stood at old lines 3378, 3512 and 3596; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function barsLayout(db) {
    /* ======================================================================
       COMPACT — THE ONE HONEST WAY TO BUY ROWS, AND IT IS A TRADE NOT A FIX
       ======================================================================
       `Fit` above makes the grid hold exactly what the screen can show. It
       cannot make the screen bigger, and on a 1,200px window the chrome above
       the grid is about 600px — the query strip 223, the coverage band 103, the
       counted line 62, plus the contract line, the anchor and the gaps. That
       leaves ten rows.

       EVERY EXTRA ROW COSTS FORTY PIXELS OF CHROME. There is no arrangement, no
       tightening and no cleverness that avoids that; three passes over this page
       tried and the arithmetic won each time. What CAN be done is to let the
       reader spend it: the coverage band and the counted line are DISPLAY, they
       answer questions about the selection rather than change it, and a reader
       who has read them once and now wants rows should be able to put them away.

       Measured at 1,200px: ten rows becomes fourteen.

       IT DEFAULTS TO OFF, deliberately. The band and the counted line were asked
       for and built; a page that hides them until asked has decided on the
       reader's behalf which half of the trade they want. The control names the
       gain — the button says how many rows it is worth right now — so the choice
       is made with the number in hand rather than by experiment. */
    let compact = $state(false);

    /* THE FOLD IS INSTANT, AND AN ANIMATED ONE WAS BUILT, MEASURED BREAKING THE
       FOLD ENTIRELY, AND REVERTED.
       ---------------------------------------------------------------------
       `{#if}` removes the panel the instant `compact` flips, so a CSS keyframe
       can only ever animate it BACK — a control whose effect is a smooth arrival
       and an abrupt disappearance reads as two different controls. The framework's
       own answer is `transition:slide`, and it was the first use of
       `svelte/transition` in this app.

       IT DID NOT MERELY FAIL TO ANIMATE, IT STOPPED THE FOLD FROM HAPPENING. A
       Svelte transition holds the element in the document until its OUT
       transition completes, and that completion is driven by the framework's own
       animation loop. Measured with the render surface backgrounded: 113
       animations all sitting at `currentTime: 0`, the out transition never
       starting, and `.facts` still 62px tall at full opacity after the press —
       the fold, which had been verified taking the grid from ten rows to
       fifteen, became a no-op. Forcing every animation to `finish()` did not
       release it either, so the element's removal was gated on something this
       environment cannot advance.

       A backgrounded tab is not exotic — it is where a page sits while its
       operator is doing something else — and an animation that can withhold a
       LAYOUT change is a different class of thing from one that merely does not
       play. The instant fold is verified and correct in every state; a
       transition that is only correct while someone is watching is not an
       improvement on it.

       WHAT WOULD MAKE IT SAFE, for whoever picks this up: collapse a wrapper
       from `grid-template-rows: 1fr` to `0fr` and keep the panel mounted. That
       animates in both directions in pure CSS, sits inside the same
       `prefers-reduced-motion` guard as every other animation here, and — the
       point — its end state is a static rule, so the layout is correct whether or
       not a single frame ever renders. */
    /* WHAT THE FOLD IS WORTH, IN THE UNIT THE READER IS SPENDING. Measured live
       rather than stated: the two panels' heights come from the elements, so a
       band that grew a legend line or a counted line that wrapped reports its own
       new price and the label cannot go stale. Zero until measured. */
    let foldPx = $state(0);

    let sizeMode = $state(/** @type {'fit' | 'fixed'} */ ('fit'));
    /** The bar box's measured inner height. Layout is the only honest source for
        it: `.board`'s height less whatever the chrome above happens to occupy,
        which depends on the reader's window, their font size and how many rungs
        the strip wrapped onto a second line. */
    let barBoxH = $state(0);
    /** @type {HTMLElement | null} */
    let barBox = $state(null);
    /** @type {HTMLElement | null} */
    let boardEl = $state(null);
    /** How much the page overflows its own scroller. Zero is the target state. */
    let pageOver = $state(0);
    /* ---------------------------------------------------------------------
       WHAT TRIGGERS THE MEASUREMENT IS THE WHOLE PROBLEM, AND TWO OBVIOUS
       ANSWERS WERE BUILT AND MEASURED FAILING BEFORE THIS ONE.

       1. `bind:clientHeight={barBoxH}` — WRONG IN THE WORST AVAILABLE WAY: it
          reports on mount and never again. Measured — first paint at a 1,100px
          window bound 382 and fitted 8 rows, correctly; the window was taken to
          1,400, the box grew to 665, the bound value stayed 382 and the grid
          stayed at 8. No error, no warning, and a first paint that passes the
          only check anyone thinks to run.

       2. A `ResizeObserver` ON THE BOX — ran away. Instrumented with a counter:
          ONE THOUSAND firings on a single page load. The reason is structural and
          it is worth stating exactly, because the same trap is available to
          anything else on this page that measures itself:

            the page size decides the ROW COUNT,
            the row count changes the PAGER'S TEXT — "of 38,969" against
              "of 12,470" — and the facts row's "newest close on this page",
            both of those are CHROME, and the chrome sits ABOVE the box,
            so the chrome's height decides the BOX'S height,
            and the box's height decides the PAGE SIZE.

          The sensor was wired to its own output. A tolerance damps JITTER; it
          cannot fix a loop whose gain is structural, which is why the eight
          pixels of slack below did not stop it and should not be expected to.

       SO THE TRIGGER IS THE WINDOW, WHICH THE LOOP CANNOT CAUSE. A resize is the
       reader's act. Nothing this component writes can produce one, so the cycle
       is cut at the only place it can be cut — the measurement is no longer
       downstream of the thing it decides.

       THE COST IS NAMED RATHER THAN HIDDEN: a chrome height that changes without
       a resize — the strip wrapping to a second line because a longer instrument
       was chosen — is not picked up until the next resize. The slack below
       absorbs a few pixels of that and the rest is one row, which is why this is
       the right trade and not merely the safe one.

       AND NOT `requestAnimationFrame`, WHICH WAS THE THIRD FAILURE AND THE ONLY
       ONE THAT BREAKS WHEN NOBODY IS LOOKING. A double-rAF was written here to
       let the panel settle before reading. rAF DOES NOT RUN IN A BACKGROUND TAB:
       measured with the pane hidden, the callback never fired, `barBoxH` stayed
       at its initial 0, and `rowsThatFit`'s `max(1, …)` floor turned that into a
       grid holding ONE ROW under a pager reporting six hundred thousand. A reader
       who opens `/db` in a second tab and switches to it later gets that. A timer
       is throttled in a background tab; it is not cancelled.

       SO: READ NOW, AND READ AGAIN ON THE NEXT TURN OF THE LOOP. The immediate
       read is correct in the ordinary case — a Svelte effect runs after the DOM
       is updated — and the deferred one catches what settles late: web fonts, the
       panel animations, the notes line under the table.
       --------------------------------------------------------------------- */
    $effect(() => {
      const el = barBox;
      if (!el) return;
      /* `compact` IS READ SO THE FOLD RE-MEASURES, and it is loop-safe for the
         same reason the window's resize is: it changes only when the reader
         presses the button. The two panels' heights come out of the layout in the
         same pass, so the button's label is priced from what was on screen a
         moment ago rather than from a number typed here. `foldPx` is only written
         while the panels EXIST — folded, they measure zero, and a label reading
         "+0 rows" on the control that would give them back is worse than stale. */
      void compact;
      /* AND RE-MEASURE ON EVERY NEW READ, because the chrome changes without a
         resize and the fit was going stale in ordinary use.
         ---------------------------------------------------------------------
         MEASURED: sorting by Close raised a notes line under the grid, the box
         shrank from 465 to 444 and the fit dropped to nine rows — correct. Then
         sorting BACK to Date removed the line, the box returned to 465, and the
         grid stayed at NINE, because nothing re-measured. A row lost to a sort
         and not recovered by undoing it, until the window was resized.

         WHY THIS IS SAFE NOW AND WOULD NOT HAVE BEEN BEFORE. This is the feedback
         edge the `ResizeObserver` was removed for: if the chrome's height depends
         on the row count, measuring after every read makes the measurement its
         own cause. The notes line WAS such a dependency — the false manifest
         warning above fired because the page size differed from a month's row
         count, so the chrome literally grew with the page size. Guarding that
         warning to full reads is what cut the edge; with it gone, the notes line
         reflects the STORE and not the paging, and nothing above the grid counts
         rows any more. The two fixes are one fix, and this one must not be kept
         if the other is ever reverted. */
      void db.barPlanKeys;
      /* `settled` — WHETHER THIS READ IS ENTITLED TO AN OPINION ON OVERFLOW.
         ---------------------------------------------------------------------
         The box's height can be read at any moment: it comes from `flex` against
         a column whose size is set by the viewport, so it is right even mid-update.
         The page's OVERFLOW cannot. It is content against container, and during a
         read the content is briefly whatever the previous rows plus the new state
         happen to make — a transient that is not what the reader will see.

         That distinction became load-bearing the moment this effect started
         re-measuring on every fetch. MEASURED: sorting by Close and back at a
         1,200px window — which fits the readouts comfortably — sampled a
         mid-update overflow, and the auto-fold, which is deliberately monotonic
         and fires once, folded the readouts away and left them folded. A
         transient became permanent, on a window with room to spare.

         So only a read that is genuinely after the fact writes `pageOver`: the
         250ms one, and the resize handler, whose event fires after layout. The
         immediate and next-tick reads keep the box honest and stay out of a
         decision they cannot see clearly. */
      const measure = () => {
        barBoxH = el.clientHeight;
        /* NOTHING FOLDS ANY MORE, SO THE FOLD IS WORTH NOTHING.
           This summed two terms. The coverage band was one and left the page; the
           counted line was the other and its last two readouts are now inline on
           the anchor, which is always drawn. `foldPx` stays 0, `foldRows` with
           it, and the Compact button — whose entire job was to trade those two
           panels for grid rows — is hidden rather than left as a control that
           presses and changes nothing.
           THE MEASUREMENT ITSELF STAYS. `barBoxH` is what `rowsThatFit` is
           computed from, and that is the whole of `Fit`. Only the fold term
           went. */
      };
      measure();
      const settle = setTimeout(measure, 0);
      /* A THIRD READ, LATER, AND IT IS KEPT THOUGH THE THING IT WAS WRITTEN FOR
         IS GONE.
         ---------------------------------------------------------------------
         It was added because an animated fold moved the panel's height over
         190ms, so both reads above landed mid-slide and whatever they caught was
         LATCHED until the next resize. Measured then: a fold-then-unfold cycle
         left the box at 444px where it had been 465, and the grid came back with
         nine rows where it had had ten — a row lost to an animation frame,
         permanently.

         That transition was reverted (see `compact`), so the fold is instant and
         the two reads above are enough for it. This stays because the defect it
         caught is not specific to a transition: `compact` changes the layout, and
         ANY late reflow — a web font landing, a scrollbar arriving, a strip
         rewrapping — latches the same way. It costs one timer per mount and it is
         the difference between a fit that is right and a fit that was right at
         the moment it was taken. */
      const afterFold = setTimeout(measure, 250);
      window.addEventListener('resize', measure);
      return () => {
        window.removeEventListener('resize', measure);
        clearTimeout(settle);
        clearTimeout(afterFold);
      };
    });
    /* `CBAND_SLIM_H` STOOD HERE AND WENT WITH THE BAND IT MEASURED. It was the
       slim rail's 36px, used to price how much height the fold reclaimed from a
       band that STAYED and shed only its furniture. The band has left the page,
       so the fold has one term and a constant describing an element that no
       longer renders is exactly the stale copy this file spends its comments
       avoiding. */
    /** What the fold is worth right now, in rows. */
    const foldRows = $derived(Math.max(0, Math.floor(foldPx / ROW)));

    /* ======================================================================
       THE OVERFLOW IS MEASURED ON ITS OWN CLOCK, AND DELIBERATELY NOT ON THE
       DATA'S
       ======================================================================
       Two different questions were being answered by one measurement, and only
       one of them survives a mid-fetch reading.

       The BOX'S HEIGHT is safe to read at any moment: it comes from `flex`
       against a column the viewport sizes, so it does not depend on what is in
       it. That read belongs with every layout change, including every new fetch,
       which is why the effect above takes `barPlanKeys` as a dependency.

       The PAGE'S OVERFLOW is content against container. During a read the content
       is briefly whatever the old rows and the new state make between them, and
       that transient is not what the reader will see. Wiring it to the same clock
       was measured doing real damage TWICE: sorting by Close and back at a
       1,200px window — which fits the readouts with room to spare — sampled a
       mid-update overflow and the auto-fold, which is monotonic and fires once by
       design, folded the readouts away permanently. Deferring the sample by 250ms
       did not fix it; the fetch simply outlasts 250ms.

       A monotonic rule cannot afford a false positive, because it has no way back.
       So this effect depends on `boardEl` ALONE: it runs on mount and on the
       window's own resize, both of which are settled moments the reader caused,
       and it never re-runs because data arrived. The fold now answers "is this
       window too small", which is the question it was written for, rather than
       "was the page mid-update when someone looked". */
    let settledBoxH = $state(0);
    $effect(() => {
      const el = boardEl;
      if (!el) return;
      const read = () => {
        pageOver = el.scrollHeight - el.clientHeight;
        /* THE BOX'S HEIGHT IS TAKEN HERE TOO, AND NOT REUSED FROM `barBoxH`.
           `barBoxH` is deliberately re-read on every fetch so the grid's own fit
           tracks the layout closely — which means it also holds the transients.
           The fold must not see those: it is monotonic, so one bad reading is
           permanent. Same instant, same element, different clock. */
        settledBoxH = barBox ? barBox.clientHeight : 0;
      };
      /* Not immediate: on mount the grid has not had its first rows yet, so the
         board is short and reports no overflow whatever the window's size. */
      const t = setTimeout(read, 250);
      window.addEventListener('resize', read);
      return () => {
        window.removeEventListener('resize', read);
        clearTimeout(t);
      };
    });
    /** The row count the FOLD reasons about — from the settled height only. */
    const settledRows = $derived(
      settledBoxH > 0 ? Math.max(1, Math.floor((settledBoxH - HEAD - FIT_SLACK) / ROW)) : 0
    );

    /* ======================================================================
       THE FOLD HAPPENS BY ITSELF WHEN THE WINDOW CANNOT HOLD THE PAGE
       ======================================================================
       `Fit` sizes the grid to the space it is given and the table has a floor —
       it may not shrink to nothing. On a short window those two meet: at 900px
       the box sits ON its floor and the page still overflows by 136px.

       A 136-PIXEL SCROLL RANGE IS THE WORST LENGTH THERE IS. Every visible
       element moves exactly 1:1 with it — verified across 330 elements, one
       offender and it is the invisible `.sr-only` — so nothing is JUMPING. It
       still reads as shaking, because a wheel gesture is longer than the range:
       the page lurches a fraction of a screen, hits the end and rebounds. The
       defect is not the motion of any element, it is that the range exists at
       all when the page was supposed to fit.

       So when the page overflows, the readouts fold themselves and the space
       goes to the grid. At 900px that turns a 136px scroll range into none, and
       six rows into eight.

       IT ONLY EVER FOLDS — NEVER UNFOLDS — AND THAT IS WHAT MAKES IT TERMINATE.
       Folding changes the layout, which changes the overflow, which is the input
       to this decision: a rule that could also unfold would sit on the boundary
       and alternate forever, which is the same trap the `ResizeObserver` above
       fell into. Monotonic, it runs at most once. Unfolding is the reader's, on
       the button, and it stays unfolded — `autoFolded` exists so the control can
       say the page did this rather than let it look like a setting that changed
       on its own.

       `pageOver > 1` RATHER THAN `> 0` because sub-pixel layout leaves a stray
       pixel of "overflow" on a page that visually fits, and folding two panels
       over one pixel would be absurd. */
    let autoFolded = $state(false);
    /* ONCE THE READER HAS PRESSED THE BUTTON, THE PAGE STOPS DECIDING. Without
       this the automatic rule is not merely persistent, it is UNDOABLE-PROOF:
       unfolding restores the panels, the page overflows again, and the rule
       folds them back inside the same frame — a button that visibly refuses the
       press. Monotonic-until-overridden is the shape that has both properties,
       the page helping by default and never overruling a choice. */
    let readerChose = $state(false);
    $effect(() => {
      if (readerChose || compact) return;
      /* NOTHING IS DECIDED BEFORE THE SETTLED READ HAS HAPPENED. `settledRows` is
         0 until then, and 0 is not "no rows fit" — it is "nobody has looked". */
      if (settledRows === 0) return;
      /* `settledRows`, NOT `rowsThatFit`. The grid's own fit is re-read on every
         fetch so it tracks the layout closely; this rule is monotonic and one
         transient reading is permanent. MEASURED with `rowsThatFit` here: sorting
         by Close and back at a 1,200px window folded the readouts away and left
         them folded, because the row count dipped for one update in the middle of
         the switch back. */
      if (pageOver <= 1 && settledRows >= MIN_USEFUL_ROWS) return;
      /* Nothing to gain by folding what is not there: on a window so short that
         the grid is small even WITHOUT the readouts, hiding them buys nothing and
         the reader loses them for no rows. */
      if (foldPx <= 0) return;
      untrack(() => {
        compact = true;
        autoFolded = true;
      });
    });
    /* EIGHT PIXELS OF SLACK, AND IT IS A DAMPER RATHER THAN A FUDGE.
       ---------------------------------------------------------------------
       MEASURED without it: a 1,400px window computed sixteen rows from a 670px
       box, rendered them, and the box then settled at 665 — five pixels short,
       so the table carried a five-pixel scrollbar. Late settling is normal here:
       web fonts land, the panel animations finish, a scrollbar comes or goes.

       THE REASON IT IS NOT SIMPLY RE-MEASURED is that re-measuring can spin. The
       pager's own text carries the page COUNT, which changes when the page SIZE
       changes — "of 38,969" against "of 12,470" — so a narrow window can wrap it
       to a second line and change the chrome's height. Then: sixteen rows makes
       the box short, fifteen makes it tall enough for sixteen, and the fit
       alternates for as long as the tab is open. A tolerance is what stops a
       control loop hunting across a boundary, and eight pixels is a fifth of a
       row: it absorbs the jitter that was measured and it can only ever cost a
       row when the box is within eight pixels of holding one anyway.

       `- HEAD` because the sticky header sits INSIDE the scroll box and covers
       the first rows; a fit computed without it is one row too many, which is
       precisely one row of hidden scrolling. `max(1, …)` because a window short
       enough to fit no rows must still ask for a row, or the pager reports zero
       of zero on a store holding four million bars. */
    /* 24, AND IT IS SIZED TO A NAMED THING RATHER THAN ROUNDED UP UNTIL THE BUG
       STOPPED. `.bnotes` — the warnings strip under the grid — is 17px when it
       carries a line and renders AFTER the bars load, so a box measured at 682
       is 665 by the time the rows are in it. That is the seventeen; the rest is
       the few pixels of font-and-animation settle measured earlier. It cannot be
       observed away: the notes list one entry per file whose manifest disagrees
       with what the file returned, and which files are read depends on the page
       size, so watching that strip re-enters the loop this whole block exists to
       cut. 24px is 0.6 of a row — it costs a row only when the box was within a
       hair of holding one, and it guarantees the table never carries a five-pixel
       scrollbar, which is the thing the reader actually notices.
       THE DECLARATION ITSELF IS UP BESIDE `ROW` AND `HEAD`, because the fold's
       `settledRows` divides by it too and is written above this point. The
       reasoning stays here, where the 24 was measured. */
    const rowsThatFit = $derived(
      Math.max(1, Math.floor((barBoxH - HEAD - FIT_SLACK) / ROW))
    );

    Object.defineProperties(db, {
      compact: { get: () => compact, set: (v) => (compact = v) },
      foldPx: { get: () => foldPx, set: (v) => (foldPx = v) },
      sizeMode: { get: () => sizeMode, set: (v) => (sizeMode = v) },
      barBoxH: { get: () => barBoxH, set: (v) => (barBoxH = v) },
      barBox: { get: () => barBox, set: (v) => (barBox = v) },
      boardEl: { get: () => boardEl, set: (v) => (boardEl = v) },
      pageOver: { get: () => pageOver, set: (v) => (pageOver = v) },
      foldRows: { get: () => foldRows },
      settledBoxH: { get: () => settledBoxH, set: (v) => (settledBoxH = v) },
      settledRows: { get: () => settledRows },
      autoFolded: { get: () => autoFolded, set: (v) => (autoFolded = v) },
      readerChose: { get: () => readerChose, set: (v) => (readerChose = v) },
      rowsThatFit: { get: () => rowsThatFit }
    });
  }

  /**
   * THE BAR READ — the plan, the page plan, the requests, the rows, the sort, the
   * pager arithmetic, the go-to jump and the magnitude rulers.
   *
   * Creates the page's effects that stood at old lines 4783; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function barsRead(db) {
    /** The natural width of the bar grid - SUMMED from the columns, never typed. */
    /* THE COLUMNS ACTUALLY DRAWN, WHICH IS NOT ALL OF THEM.
     *
     * Ten of the twenty-four have NO SOURCE on this wire — the pre-market
     * percentage, moneyness, intrinsic, extrinsic and the six greeks — and four
     * more describe a CONTRACT: expiry, days-to-expiry, type and strike. A store
     * of indices and equities has neither kind, so fourteen of twenty-four
     * columns were being drawn across every row of every page purely to say
     * "NO SOURCE" or "—". At fifty rows a page that is seven hundred cells of
     * nothing, and it pushed close and volume off the side of the screen.
     *
     * `CLAUDE.md` §4 — degrade loudly, name the reason — is why they were drawn.
     * §4 is about a FAILURE being concealed, and neither of these is one: a feed
     * that never sends a greek has not failed, and a spot index has no expiry to
     * report. The reason is still named, once, in the line under the table
     * rather than seven hundred times inside it.
     *
     * The contract four come back the moment a contract is stored, on the same
     * `expiryRefusal` the Contract strip uses, so the two cannot disagree about
     * whether this store holds one.
     */
    /* WHAT KIND OF THING IS ON SCREEN, which is what decides the columns.
     *
     * A spot index has no expiry and no strike. A FUTURE has an expiry and no
     * strike. An OPTION has both, plus a side. Drawing all four contract columns
     * for a spot store meant four columns of "-" on every row; drawing strike and
     * type for a futures selection would mean two more.
     *
     * Read off the rows the query actually leaves (`scoped`) rather than off the
     * page being viewed, so paging to row 51 cannot change which columns exist.
     * `side` is the option marker because only an option has one; an expiry with
     * no side is a future. Both come from the name, parsed once per row when the
     * store is read — see `contractOf` and `expiryOf`.
     */
    const shape = $derived.by(() => {
      let future = false;
      let option = false;
      for (const r of db.scoped) {
        if (r.side) option = true;
        else if (r.expiry) future = true;
        if (option && future) break;
      }
      return { future, option, contract: future || option };
    });
    /**
     * A DAY RUNG HAS NO MINUTE WORTH A COLUMN.
     *
     * Every 1day bar opens at the same instant — the session open — so the Time
     * column would repeat one value down every row and earn none of its width. It
     * is dropped rather than blanked: a column of identical stamps is not
     * information, and a column of dashes is worse, since a dash on this table
     * means "no source" everywhere else.
     *
     * `rungSeconds` rather than a string test on '1day', so anything coarser than
     * a day — were a rung ever added above it — is covered by the same rule
     * instead of needing a second one.
     */
    const dayRung = $derived((TF_SECS.get(db.timeframe) ?? 0) >= 86400);
    const BAR_SHOWN = $derived(
      BAR_COLS.filter(
        (c) =>
          !c.none &&
          !(dayRung && c.key === 'tod') &&
          !(!shape.contract && FUTURE_COLS.has(c.key)) &&
          !(!shape.option && OPTION_COLS.has(c.key)) &&
          /* THE RUNG COLUMN GOES WHEN A RUNG IS CHOSEN, because every row then
             carries the same value. `.norung` in the stylesheet already hid it;
             this filter did not know, so `BAR_WIDTH` kept reserving its 68px and
             the table's `min-width` stayed 68px wider than anything drawn. Two
             mechanisms for one decision, disagreeing — the exact hazard the note
             over the hide rules warns about. */
          !(db.timeframe !== '' && c.key === 'tf')
      )
    );
    const BAR_HIDDEN = $derived(BAR_COLS.length - BAR_SHOWN.length);

    const BAR_WIDTH = $derived(BAR_SHOWN.reduce((a, c) => a + c.w, 0));

    /**
     * The band label for each column that STARTS a visible group.
     *
     * O(1) per column: one Set probe. The whole derivation is a single pass over
     * the five groups, not a scan of the grid.
     */
    const GROUP_HEAD = $derived.by(() => {
      const shown = new Set(BAR_SHOWN.map((c) => c.key));
      /** @type {Map<string, { label: string, n: number }>} */
      const head = new Map();
      for (const g of BAR_GROUPS) {
        const members = g.keys.filter((k) => shown.has(k));
        if (members.length > 0) head.set(members[0], { label: g.label, n: members.length });
      }
      return head;
    });
    /* THE DEFAULT IS EVERY MATCHED MONTH, and it was six.
     *
     * Operator's rule, 2026-08-19: the grid shows what the QUERY selected — the
     * chosen feed, universe, segments, timeframes and date range — not a slice of
     * it. A default of six read the six NEWEST months and rendered the rest as
     * "absent rather than empty", so a January-to-August query opened on March
     * and an operator reasonably read that as January never having been pulled.
     * It had been: 7,500 bars, on disk, structurally perfect.
     *
     * A page that answers a narrower question than the one it was asked is worse
     * than a slow one, because nothing on it is wrong — it is just not the
     * answer. The budget stays as a CONTROL for anyone who wants to cap a very
     * wide query; it is no longer the default. */
    let readBudget = $state(0);

    /**
     * The instrument-months the bar grid would read, NEWEST MONTH FIRST.
     *
     * Newest first because a store is read forward: the question an operator
     * opens this grid with is "what landed", and the newest month is where it
     * landed. "Backfill runs oldest-first" is a rule about WRITING and has no
     * bearing on which file a reader opens first.
     */
    const barPlan = $derived.by(() => {
      // Default pickers settle after a census arrives. Until their initial
      // instrument, segment and rung fields settle, planning every series sorts
      // the entire census and may open unrelated files before those effects run.
      if (db.view !== 'bars' || !db.filter || !db.kind || !db.timeframe) {
        return { all: [], read: [], held: 0 };
      }
      /* MONTHS THE WINDOW CANNOT CONTAIN ARE NOT READ AT ALL.

         A day window forces the exact-plan fallback — the note below says why —
         and the fallback read EVERY matched instrument-month. With the window on
         August, that opened January through July as well: seven files fetched,
         parsed, and discarded whole by `barWindowed`, which is the exact waste
         the prefix-sum plan exists to avoid, reintroduced by the one control
         that disables it.

         They were not merely wasted. Each contributed a manifest disagreement —
         "returned 0 bar(s), the census claims 575" — because a month outside the
         window contributes nothing while the census still counts it. The guard
         on `barDisagree` stops the accusation; this stops the read that produced
         it.

         MONTH GRAIN, AND ONLY WHOLLY-OUTSIDE MONTHS GO. A month is `YYYY-MM` and
         the bounds are `YYYY-MM-DD`, so comparing the first seven characters is
         an ISO-safe overlap test: the partial months at each end are kept whole
         and `barWindowed` trims them by day, exactly as before. Nothing that
         could contribute a row is dropped. */
      const lo = dayWindowNarrows && db.fromDay ? db.fromDay.slice(0, 7) : '';
      const hi = dayWindowNarrows && db.toDay ? db.toDay.slice(0, 7) : '';
      const inWindow = (/** @type {any} */ r) =>
        (!lo || r.month >= lo) && (!hi || r.month <= hi);
      const all = [...db.matched].filter(inWindow).sort(
        (/** @type {any} */ a, /** @type {any} */ b) =>
          txt(b.month, a.month) || txt(a.instrument, b.instrument) || tfCmp(a.timeframe, b.timeframe)
      );
      const take = readBudget === 0 ? all.length : Math.min(readBudget, all.length);
      return { all, read: all.slice(0, take), held: all.length - take };
    });

    /* ---------------------------------------------------------------------
       WHETHER THE DAY WINDOW EXCLUDES ANYTHING, WHICH IS NOT THE SAME QUESTION
       `pageExact` ANSWERS.

       `pageExact` is false for either of two unrelated reasons — a sort no index
       covers, or a day window that narrows — and the window route can serve the
       first but NOT the second. `/bars/window.json` takes a MONTH range; it has
       no day filter and cannot have one without the store gaining a day index.

       WHAT THAT COST, measured by typing a window into the date fields: with the
       grid sorted by CLOSE and the range set to 2016 — entirely before this
       store's first bar in 2019-12 — the route returned its fifty rows for the
       month range, `barWindowed` filtered every one of them out on the day, and
       the grid drew NOTHING under a pager still reading `4,49,751-4,50,000 of
       6,23,498`. The empty-state then blamed "a disagreement between" the file
       and the census, which is not what happened at all: the filter did it.

       So the day window sends the read back to the full path, where the client
       holds every row and can filter them itself. Slower, and it answers. */
    const dayNarrows = $derived.by(() => {
      if (!dayWindowApplies || (!db.fromDay && !db.toDay)) return false;
      for (const r of barPlan.read) {
        const lo = r.first_ts;
        const hi = r.last_ts;
        /* NO SPAN MEANS IT CANNOT BE PROVED HARMLESS, so it is treated as
           narrowing — the same conservative reading `pageExact` takes. */
        if (!Number.isFinite(lo) || !Number.isFinite(hi)) return true;
        if (db.fromDay && istDayKey(lo / 1000) < db.fromDay) return true;
        if (db.toDay && istDayKey(hi / 1000) > db.toDay) return true;
      }
      return false;
    });

    /* WHETHER THE PREFIX SUM CAN BE TRUSTED - read by both derivations below,
       so the two can never disagree about which mode the grid is in.

       A WINDOW BEING SET IS NOT A WINDOW NARROWING ANYTHING, and treating the
       two as one is what made this fall back on every load: the date controls
       open populated - 01 Jan 2015 to today on this build - so `fromDay ||
       toDay` is true before the operator has touched either, and the exact path
       would never once have run.

       What matters is whether the window EXCLUDES a row, and the census answers
       that without opening a file: every entry carries the span it actually
       covers, so a month lying wholly inside the window loses nothing and its
       census count still predicts its rows. A month that straddles either edge,
       or falls outside entirely, does not - and that is the fallback's case,
       because no count in the census says how many of a month's rows sit on one
       side of a day. */
    const pageExact = $derived.by(() => {
      if (barSortKey !== 'ts') return false;
      if (!dayWindowApplies || (!db.fromDay && !db.toDay)) return true;
      for (const r of barPlan.read) {
        /* THE WIRE'S OWN SPELLING, AND ONLY IT. `store.rows` is the array as
           `/store.json` sent it, so the span is `first_ts`/`last_ts`. A first
           attempt read `first`/`last` - the names `$lib/store.svelte.js` gives
           its NORMALISED cells, which are a different array - found `undefined`
           on every row and fell back on every load, which is the silent
           fallback this branch exists to avoid. `svelte-check` names the type's
           real field as `firstAt`, so neither alternative spelling belongs
           here and both are gone rather than left as a dead `??`. */
        const lo = r.first_ts;
        const hi = r.last_ts;
        /* AN ENTRY WITH NO SPAN CANNOT BE PROVED TO SURVIVE, so it is not
           assumed to. Older census images predate the two timestamps. */
        if (!Number.isFinite(lo) || !Number.isFinite(hi)) return false;
        /* MICROSECONDS IN, MILLISECONDS OUT, AND THE DIVISION IS NOT OPTIONAL.
           The census writes microseconds - `crates/api`'s unit, which
           `$lib/store.svelte.js` documents and `$lib/dates.js` refuses to sniff -
           and `istDayKey` hands its argument straight to `new Date`, which reads
           milliseconds. Passing the census figure raw dated every month to the
           year 58555, so `istDayKey(hi) > toDay` was true on every row and the
           exact path never once ran. A thousand-fold unit error does not throw;
           it just quietly answers the wrong question. */
        if (db.fromDay && istDayKey(lo / 1000) < db.fromDay) return false;
        if (db.toDay && istDayKey(hi / 1000) > db.toDay) return false;
      }
      return true;
    });

    /* THE ROW COUNT THE PAGER COUNTS IN, AND IT MUST NOT DEPEND ON THE PAGE.
       `pageTotal` feeds `pageCount` feeds `pageNow`, and `pagePlan` reads
       `pageNow` - so computing the total inside `pagePlan` closes a cycle that
       Svelte resolves by throwing. Summed here, from the census alone, the total
       is a property of the QUERY and the cycle does not exist. */
    const barTotalRows = $derived.by(() => {
      if (!pageExact) return null;
      let t = 0;
      for (const r of barPlan.read) t += Number(r.rows) || 0;
      return t;
    });

    const pagePlan = $derived.by(() => {
      const all = barPlan.read;
      if (!pageExact) {
        return { files: all, base: 0, exact: false };
      }
      /* NEWEST FIRST IS THE PLAN'S ORDER, which is `ts` DESCENDING. Ascending
         asks for the same files from the other end, so the list is walked
         reversed rather than re-sorted. */
      const order = barDesc ? all : [...all].reverse();
      const first = (pageNow - 1) * db.pageSize;
      const last = first + db.pageSize - 1;
      /** @type {any[]} */
      const need = [];
      let seen = 0;
      let base = 0;
      for (const r of order) {
        const n = Number(r.rows) || 0;
        if (n === 0) continue;
        const lo = seen;
        const hi = seen + n - 1;
        seen += n;
        if (hi < first) continue;
        /* PAST THE PAGE, SO NOTHING FURTHER CAN OVERLAP IT. The list is walked
           in row order, so once a file starts after the last row wanted, every
           file behind it does too. `barTotalRows` already has the count, so
           there is nothing left for this loop to learn. */
        if (lo > last) break;
        if (need.length === 0) base = lo;
        need.push(r);
      }
      return { files: need, base, exact: true };
    });

    /** A PRIMITIVE, so the fetch effect re-runs when the SET changes and not
        on every keystroke that leaves the same set standing.
     *
     * THE PAGE IS PART OF THE KEY ON THE WINDOW ROUTE, AND LEAVING IT OUT SHOWED
     * THE WRONG ROWS UNDER THE RIGHT PAGER.
     *
     * On the seek route the file set IS the page — turn to page 6,000 and a
     * different month is wanted, so this string changes and the effect re-runs.
     * On the window route it is not: every matched month is named whatever page
     * you are on, because the server does the slicing. So the key held still, the
     * effect never re-ran, and the request that had already been made — `offset=0`
     * — remained the only one.
     *
     * MEASURED: page 6,000 of a CLOSE-sorted grid drew `2,99,951-3,00,000 of
     * 6,23,498` over the fifty rows of page ONE, having sent exactly one request
     * for offset 0. The pager was right, the rows were wrong, and nothing said so
     * — which is worse than an empty grid, because an empty grid cannot be
     * misread.
     *
     * `page` AND NOT `pageNow`. `pageNow` is clamped through `pageCount` from
     * `pageTotal`, which on this route reads `windowSaid`, which reads the files
     * this effect WRITES. Keying on it would close a loop through the effect:
     * fetch, new total, new clamp, new key, fetch again. `page` is raw `$state`
     * and depends on nothing downstream, so it triggers without feeding back. */
    /* `$derived.by` AND NOT `$derived`, for a reason about the CHECKER rather
       than the runtime. `barSortKey` and `barDesc` are declared further down this
       file, and both forms read them lazily, so both work. But svelte-check reads
       `$derived(expr)` as an expression evaluated HERE and reports "Block-scoped
       variable 'barSortKey' used before its declaration", while it reads
       `$derived.by(() => …)` as the deferred body it is — which is why
       `pageExact`, a hundred lines above and reading the same variable, was never
       flagged. Same semantics; one of them provable. */
    const barPlanKeys = $derived.by(() => {
      const files = pagePlan.files.map((/** @type {any} */ r) => r.key).join('\n');
      if (pageExact && timeNarrows) return files;
      return `${files} ${db.page}|${db.pageSize}|${barSortKey}|${barDesc}`;
    });

    /* A `barSetKey` OF THE FILES ALONE WAS WRITTEN HERE AND REMOVED, and the
       reasoning is kept because it is convincing and wrong. It was meant to
       answer "are these different rows" for the entrance direction — the files
       look like the set, and a re-sort of one instrument-month is the same bars
       in a new order. But sorting by Close cannot be answered from the census
       prefix sums, so the grid opens a DIFFERENT set of files in order to sort
       globally: the files change because of the sort, which is the single case
       the test existed to distinguish. Measured — the sort reported `set`.
       `pageTotal` is what carries that fact; see `barMove`. */

    /* ---------------------------------------------------------------------
       THE READ. One request per instrument-month, cached for as long as the
       feed and the Refresh nonce hold still, so narrowing the query does not
       re-ask for a file already in memory.
       --------------------------------------------------------------------- */
    /** @type {Map<string, any>} */
    const barCache = new Map();
    let barCacheStamp = '';
    /** @type {{ loading: boolean, error: string | null, files: any[] }} */
    let barState = $state.raw({ loading: false, error: null, files: [] });
    let barToken = 0;

    /**
     * One month file, for one series — CACHED ON THE PROMISE, NOT ON THE ANSWER.
     *
     * THE DUPLICATE THIS REMOVES. `barCache` held finished reads, so two askers
     * for the same file deduplicated only if the first had already COME BACK.
     * The fetch effect re-runs as the query settles, and those runs overlap:
     * measured on a fresh pull, page 1 fired seven requests where the settled
     * plan needs three — BANKNIFTY's 2026-08 and 2026-07 were each fetched twice,
     * because the second ask was made while the first was still in flight and the
     * cache was therefore still empty.
     *
     * Caching the PROMISE closes that: the second asker joins the first request
     * instead of starting a second one. Deduplication becomes a property of
     * asking rather than of timing.
     *
     * ONLY SUCCESSES SURVIVE, which is the rule the old cache kept and this one
     * must not lose. A refusal is a VALUE here, not a rejection — one unreadable
     * file must not blank the twelve that read — so the entry is dropped once a
     * failed read resolves, and the next ask is a real retry rather than the same
     * failure served from memory for ever.
     *
     * @param {string} feed
     * @param {any} r
     * @param {AbortSignal} signal
     * @returns {Promise<any>}
     */
    function readBarFile(feed, r, signal) {
      const hit = barCache.get(r.key);
      if (hit && !hit.signal?.aborted) return hit.promise ?? Promise.resolve(hit);
      /** @type {{signal:AbortSignal,promise:Promise<any>|null}} */
      const inflight = { signal, promise: null };
      inflight.promise = fetchBarFile(feed, r, signal).then((out) => {
        if (barCache.get(r.key) !== inflight) return out;
        if (!signal.aborted && out && out.error === null) barCache.set(r.key, out);
        else barCache.delete(r.key);
        return out;
      });
      barCache.set(r.key, inflight);
      return inflight.promise;
    }

    /**
     * The read itself. Called only by [`readBarFile`], which owns the cache.
     *
     * A FAILURE IS A VALUE HERE, NOT A REJECTION: one unreadable file must not
     * blank the twelve that read, so the grid lists each refusal under its own
     * instrument-month.
     *
     * @param {string} feed
     * @param {any} r
     * @param {AbortSignal} signal
     */
    async function fetchBarFile(feed, r, signal) {
      /* EXCHANGE-SEGMENT-SYMBOL, AND THE SYMBOL KEEPS ITS OWN HYPHENS. The
         store's series name is `Exchange-Segment-Symbol`, and a symbol is
         legally `ABB-III` or `NIFTY-2026-08-27-2500000-CE`. Only the FIRST
         TWO separators are structural; everything after them is the symbol.
         Splitting on the last one instead is how `-CE` becomes a segment. */
      const parts = String(r.instrument).split('-');
      /* THE CONTRACT IS ITS OWN SEGMENT, AND THIS USED TO SWALLOW IT.
         `symbol: parts.slice(2).join('-')` put everything after the segment into
         the symbol, so an option asked for
         `symbol=BANKNIFTY-2026-07-28-5410000-CE` — 31 bytes against the store's
         24-byte cap. Measured 2026-08-20, journal seq 910–922: thirteen 400s in
         three seconds, one per contract this page tried to draw, every one
         reading "path segment symbol is 31 bytes, max 24". The store's refusal
         was correct; the question was malformed, and had been since F&O bars
         could not exist to ask about.
         WHERE THE SPLIT IS: a contract segment always opens with a `YYYY-MM-DD`
         triple — `crates/core/src/instrument.rs` renders it `{:04}-{:02}-{:02}`
         before the strike and side. So the symbol runs until a four-digit part
         FOLLOWED BY two two-digit parts, and the contract is everything from
         there. Matching on the four-digit year alone would be looser than it
         needs to be; requiring the whole triple means an instrument whose name
         merely contains digits cannot be split in the middle.
         No triple means no contract, which is spot — and spot's path is one
         level shallower, so an empty `contract` is the right answer rather than
         a missing one. */
      const two = (/** @type {string} */ s) => /^\d{2}$/.test(s);
      let at = -1;
      for (let i = 2; i + 2 < parts.length; i += 1) {
        if (/^\d{4}$/.test(parts[i] ?? '') && two(parts[i + 1] ?? '') && two(parts[i + 2] ?? '')) {
          at = i;
          break;
        }
      }
      const q = new URLSearchParams({
        feed,
        exchange: parts[0] ?? '',
        segment: parts[1] ?? '',
        symbol: at === -1 ? parts.slice(2).join('-') : parts.slice(2, at).join('-'),
        contract: at === -1 ? '' : parts.slice(at).join('-'),
        /* THE RUNG THE ROW ITSELF CARRIES. Omitted, the server defaults to
           `1min`, and a daily file is then asked for at a rung that has no
           file - the exact refusal /markets already paid for once. */
        timeframe: r.timeframe,
        month: r.month,
        /* THE DAY WINDOW GOES ON THE WIRE, and until it did this page asked for
           a whole MONTH per row and threw ~98% of it away in the browser.

           Measured on this build: 81 bytes a bar, so a one-minute month is about
           670 KB, and `/db` reads one row per instrument-month — 549 of them on
           the operator's store. Asking for a single day therefore moved roughly
           370 MB and parsed every byte of it to render about 375 bars. That is
           what "the page is stuck" was: not a hang, arithmetic.

           Empty strings are omitted rather than sent blank, because the server
           treats an unparseable day as "no bound" and a blank one would read as a
           typo it is right to ignore — sending nothing says the same thing
           without relying on that. */
        ...(db.fromDay ? { from: db.fromDay } : {}),
        ...(db.toDay ? { to: db.toDay } : {})
      });
      try {
        signal.throwIfAborted();
        const res = await ask(`/bars.json?${q}`, { signal });
        let body = null;
        try {
          body = await res.json();
        } catch {
          return {
            key: r.key,
            row: r,
            bars: [],
            faults: null,
            error: `HTTP ${res.status}, and the body was not JSON`
          };
        }
        /* THE SERVER'S OWN SENTENCE, not a status code. `/bars.json` refuses
           with `{"error":"...2026-08.bin does not exist, opening it"}`, which
           names the file; "HTTP 400" names nothing and sends the operator to
           the logs. */
        if (body && typeof body === 'object' && !Array.isArray(body) && body.error) {
          return { key: r.key, row: r, bars: [], faults: null, error: String(body.error) };
        }
        if (!res.ok && res.status !== 206) {
          return { key: r.key, row: r, bars: [], faults: null, error: `HTTP ${res.status}` };
        }
        /* A PARTIAL read answers 206 with `{bars, faults}`. Both shapes are
           handled and a faulty record is surfaced, never silently dropped - a
           grid with an unexplained hole is worse than one with a warning. */
        const out = {
          key: r.key,
          row: r,
          bars: Array.isArray(body) ? body : (body?.bars ?? []),
          faults: Array.isArray(body) ? null : (body?.faults ?? null),
          error: null
        };
        /* NOT CACHED HERE ANY MORE — `readBarFile` owns the cache, and writing it
           from both places is how the two spellings drift. */
        return out;
      } catch (why) {
        const w = /** @type {any} */ (why);
        return {
          key: r.key,
          row: r,
          bars: [],
          faults: null,
          error: String(w && w.message ? w.message : w)
        };
      }
    }

    /**
       ONE REQUEST FOR A PAGE NO INDEX CAN ANSWER.

       Sorting by a price column is the one question the prefix sum cannot serve:
       the store is indexed by TIME — the path IS the index, and `CLAUDE.md` §4
       bans a query planner because there is no second path to choose — so "the
       fifty largest closes" cannot be known without reading the closes.

       The browser used to read them ALL. MEASURED: 76 requests and 9.4 seconds
       for one sorted page, and the network was never the cost — parsing 623,498
       rows into objects to keep fifty was. `/bars/window.json` answers the same
       question against files already on local disk: MEASURED at 1.23s for the
       identical sort, moving 4 KB instead of ~50 MB, and returning the same top
       rows the browser found.

       ONE INSTRUMENT PER CALL, because the endpoint is keyed on one series. A
       query matching three instruments is three calls, not 2,187 — still a
       constant per instrument rather than a file per month.

       THE CHANGE COLUMN COMES BACK WITH THE ROWS. It is folded server-side in
       time order before the sort (D-0265), which is the only place it can be
       computed for a sorted page. `barRows` prefers the wire's value wherever the
       wire sent one.
       ------------------------------------------------------------------------
       @param {string} feed
       @param {any[]} plan every matched instrument-month, for the series it names
       @param {number} offset @param {number} limit
       @param {AbortSignal} signal
       @returns {Promise<any[]>} one synthetic file, in `barState.files` shape */
    async function readWindow(feed, plan, offset, limit, signal) {
      if (plan.length === 0) return [];
      /* THE SERIES IS ONE PER CALL AND THE PLAN MAY NAME SEVERAL. Grouped by the
         instrument key the census carries, so each series asks once for its own
         slice and the page stitches them. */
      /** @type {Map<string, any[]>} */
      const bySeries = new Map();
      for (const r of plan) {
        const k = `${r.instrument}|${r.timeframe}`;
        if (!bySeries.has(k)) bySeries.set(k, []);
        (bySeries.get(k) ?? []).push(r);
      }
      const series = [...bySeries.values()];
      const out = await pooled(series, IN_FLIGHT, async (group) => {
        const first = group[0];
        const months = group.map((g) => String(g.month)).sort();
        const parts = String(first.instrument).split('-');
        const two = (/** @type {string} */ s) => /^\d{2}$/.test(s);
        let at = -1;
        for (let i = 2; i + 2 < parts.length; i += 1) {
          if (/^\d{4}$/.test(parts[i] ?? '') && two(parts[i + 1] ?? '') && two(parts[i + 2] ?? '')) {
            at = i;
            break;
          }
        }
        const q = new URLSearchParams({
          feed,
          exchange: parts[0] ?? '',
          segment: parts[1] ?? '',
          symbol: at === -1 ? parts.slice(2).join('-') : parts.slice(2, at).join('-'),
          contract: at === -1 ? '' : parts.slice(at).join('-'),
          timeframe: first.timeframe,
          from: months[0],
          to: months[months.length - 1],
          sort: barSortKey === 'ts' ? 'ts' : barSortKey,
          dir: barDesc ? 'desc' : 'asc',
          offset: String(offset),
          limit: String(limit),
          extremes: '1'
        });
        try {
          const res = await ask(`/bars/window.json?${q}`, { signal });
          const body = await res.json();
          if (!res.ok && !body?.bars) {
            return { key: `${first.key}|window`, row: first, bars: [], faults: null,
                     error: body?.error ?? `HTTP ${res.status}` };
          }
          return {
            key: `${first.key}|window`,
            row: first,
            bars: body.bars ?? [],
            faults: body.faults ?? null,
            error: null,
            /* CARRIED SO THE PAGER AND THE SCALE CAN READ THEM WITHOUT A SECOND
               CALL. `total` is every row the window holds, not the page. */
            total: body.total ?? 0,
            extremes: body.extremes ?? null,
            scanned: body.scanned === true
          };
        } catch (why) {
          const w = /** @type {any} */ (why);
          return { key: `${first.key}|window`, row: first, bars: [], faults: null,
                   error: String(w && w.message ? w.message : w) };
        }
      }, signal);
      return out.filter(Boolean);
    }

    $effect(() => {
      const active = db.view === 'bars';
      const feed = feeds.active;
      const keys = barPlanKeys;
      void db.nonce;
      /* THE CACHE IS KEYED ON THE FEED AND THE NONCE. Refresh means "read the
         store again", and a cache that survived it would answer the re-read
         out of the copy the re-read was asked to replace. */
      const stampNow = `${feed ?? ''} ${db.nonce} ${db.fromDay}|${db.toDay}`;
      if (barCacheStamp !== stampNow) {
        barCache.clear();
        barCacheStamp = stampNow;
      }
      if (!active || !feed || keys === '') {
        barState = { loading: false, error: null, files: [] };
        return;
      }
      const want = untrack(() => pagePlan.files);
      const exact = untrack(() => pagePlan.exact);
      const first = untrack(() => (pageNow - 1) * db.pageSize);
      const take = untrack(() => db.pageSize);
      const base = untrack(() => pagePlan.base);
      const bounded = exact && !untrack(() => timeNarrows);
      const controller = new AbortController();
      const mine = ++barToken;
      let dead = false;
      barState = { loading: true, error: null, files: untrack(() => barState.files) };
      /* TWO ROUTES, AND THE PLAN ALREADY KNOWS WHICH. `exact` is the prefix sum's
         own verdict: true when the grid is in the store's order and nothing
         filters after the fetch, which is when one file answers one page. False
         is the sort no index covers, and that is what the window endpoint is for
         — one request per series rather than one per instrument-month. */
      /* THE WINDOW ROUTE ONLY FOR AN ORDER IT CAN TAKE. A derived column — the
         change columns, the expiry arithmetic, the greeks — is not a field the
         store holds, so the endpoint cannot order by it and answers 400. Falling
         back to the full read is slower and returns the right rows; sending the
         key anyway returned none. */
      /* AND NOT WHEN THE DAY WINDOW NARROWS. The route has a month range and no
         day filter, so it would answer the wider question and let the client
         throw the difference away — which drew an empty grid under a full
         pager. */
      const canWindow = !exact && !untrack(() => dayNarrows) && WINDOW_SORTS.has(barSortKey);
      (bounded
        ? readDatabasePage(feed, want, first, take, base, barDesc, controller.signal)
        : canWindow
        ? readWindow(feed, want, first, take, controller.signal)
        : pooled(want, IN_FLIGHT, (/** @type {any} */ r) => readBarFile(feed, r, controller.signal), controller.signal)
      )
        .then((files) => {
          if (dead || mine !== barToken) return;
          barState = { loading: false, error: null, files };
        })
        .catch((why) => {
          if (dead || mine !== barToken) return;
          /* NAMED, NOT SHRUGGED. `readBarFile` returns its failures as values,
             so reaching here at all is this page failing rather than a file. */
          barState = {
            loading: false,
            error: String(why && why.message ? why.message : why),
            files: []
          };
        });
      return () => { dead = true; controller.abort(); };
    });

    /** Files that answered with a refusal rather than with bars. */
    const barFails = $derived(barState.files.filter((/** @type {any} */ f) => f.error !== null));
    /** Files that answered 206 - read in part, with the part that failed named. */
    const barFaults = $derived(barState.files.filter((/** @type {any} */ f) => f.faults));

    /* ======================================================================
       THE DIGITS THAT REPEAT DOWN EVERY ROW EARN NONE OF THEIR WEIGHT
       ======================================================================
       This is the `dayrep` rule applied to prices, and prices need it more. A
       page of BANKNIFTY minute bars reads `57,647.65` / `57,647.65` /
       `57,761.95`: eight characters, of which the first four are the same on
       every row and in all four price columns. The eye has to walk past the part
       that never changes to reach the part that does, on every cell, forty times
       a page — and the part that changes is the entire reason the column exists.

       So the shared opening is dimmed. DIMMED, NOT REMOVED, for exactly the
       reasons the day column already states: the text stays in the cell, so a
       copy, an export and a screen reader all still get the whole number, and
       only its weight in the eye changes. Nothing is hidden and no figure is
       rounded — `CLAUDE.md` §7 keeps the value, this changes how it is drawn.

       ONE PREFIX ACROSS ALL FOUR PRICE COLUMNS, not one per column. They are the
       same instrument at the same magnitude, so a shared prefix makes the dimmed
       run line up across the row and the bright tails sit in a column of their
       own — which is what turns four numbers into a shape a reader can scan
       DOWN. Per-column prefixes would each be a different length and the effect
       would be noise.

       IT IS COMPUTED FROM THE PAGE, NOT THE STORE, and that is the honest scope:
       it says "these rows open alike", which is a fact about what is on screen.
       Cost is the page's own row count times a short string — the same order as
       drawing them. */
    const pricePfx = $derived.by(() => {
      if (barPage.length === 0) return '';
      /** @type {string | null} */
      let pfx = null;
      let shortest = Infinity;
      for (const b of barPage) {
        for (const v of [b.o, b.h, b.l, b.c]) {
          const s = paisaText(v);
          if (s.length < shortest) shortest = s.length;
          if (pfx === null) {
            pfx = s;
            continue;
          }
          let i = 0;
          while (i < pfx.length && i < s.length && pfx[i] === s[i]) i += 1;
          if (i === 0) return '';
          pfx = pfx.slice(0, i);
        }
      }
      /* FOUR CHARACTERS ARE NEVER DIMMED, whatever the data says. On a run of
         genuinely identical bars — which this store has, at the flat tail of a
         session — the common prefix IS the whole number, and dimming all of it
         would draw a grid of ghosts. The cap keeps a bright tail on every cell,
         so "nothing changed here" reads as a quiet column rather than a broken
         one. */
      const cap = Math.max(0, shortest - 4);
      const out = (pfx ?? '').slice(0, cap);
      /* A ONE-CHARACTER DIM IS NOT WORTH THE TWO-TONE CELL IT COSTS. */
      return out.length >= 2 ? out : '';
    });

    /** One price, split into the run every row shares and the part that is this
        row's own. Returns `['', whole]` when there is nothing worth dimming.
        @param {number | null | undefined} v */
    function priceSplit(v) {
      const s = paisaText(v);
      return pricePfx && s.startsWith(pricePfx)
        ? [pricePfx, s.slice(pricePfx.length)]
        : ['', s];
    }

    const barRows = $derived.by(() => {
      /** @type {any[]} */
      const out = [];
      for (const f of barState.files) {
        if (f.error !== null) continue;
        const r = f.row;
        const bars = f.bars;
        for (let i = 0; i < bars.length; i++) {
          const b = bars[i];
          const before = i > 0 ? bars[i - 1] : null;
          /* `typeof ... === 'number'` AND NEVER A TRUTHINESS TEST. The server
             writes `null` for `i64::MIN` and the integer for everything else,
             so `0` is a STORED ZERO and `b.oi || null` would erase it. */
          const oi = typeof b.oi === 'number' ? b.oi : null;
          const prevOi = before && typeof before.oi === 'number' ? before.oi : null;
          const ts = b.t * 1000;
          const day = istDayKey(ts);
          /* THE WIRE'S OWN CHANGE WINS WHERE THE WIRE SENT ONE.
             `/bars/window.json` folds the change against the previous bar IN TIME,
             per file, before it sorts — which is the only place that number can be
             computed for a PRICE-ORDERED page, because fifty rows sorted by close
             are fifty rows from fifty different minutes and none of them is the
             neighbour of the one above it.
             `/bars.json` sends no such field, so the local fold below still runs
             for every read on that route. `undefined` is the test and not `null`:
             null is a real answer from the server — "no change, and here is why" —
             and treating it as absent would recompute a value the server has
             already refused. */
          const wireChg = b.chg !== undefined || b.chg_why !== undefined;
          const chgWhy = wireChg
            ? (b.chg_why ?? null)
            : before === null
              ? 'first_bar_in_file'
              : before.c === 0
                ? 'previous_close_zero'
                : null;
          const oiWhy = wireChg
            ? (b.oichg_why ?? null)
            : oi === null
              ? 'oi_null'
              : before === null
                ? 'first_bar_in_file'
                : prevOi === null
                  ? 'oi_null_before'
                  : prevOi === 0
                    ? 'previous_oi_zero'
                    : null;
          /* THE ENGINE'S ROUNDING AND THE ENGINE'S OVERFLOW VERDICT, BOTH.
             `basisPoints` rounds half away from zero the way
             `server.rs::basis_points` does — `Math.round` rounded a half toward
             +Infinity here, so a gain and its mirror-image loss printed
             different magnitudes in a column that sorts. It returns `null` where
             the engine returns `Unknown::Overflow`, and that null has to become
             a REASON rather than a bare dash: `WHY.overflow` already holds the
             sentence, and a cell nobody can compute must say why. */
          const chgBps = wireChg
            ? (b.chg ?? null)
            : chgWhy === null
              ? basisPoints(before.c, b.c)
              : null;
          const oichgBps = wireChg
            ? (b.oichg ?? null)
            : oiWhy === null
              ? basisPoints(prevOi, oi)
              : null;
          out.push({
            /* THE KEY IS THE SERIES, THE MONTH, THE RUNG AND THE BAR'S OWN
               SECOND. An `{#each}` key: unique across every file on screen and
               stable across a re-sort. */
            rk: `${r.key} ${b.t} ${i}`,
            instrument: r.instrument,
            head: r.head,
            sym: r.sym,
            month: r.month,
            ts,
            day,
            tf: r.timeframe,
            o: b.o,
            h: b.h,
            l: b.l,
            c: b.c,
            vol: b.v,
            chg: chgBps,
            chgWhy: chgWhy ?? (chgBps === null ? 'overflow' : null),
            oi,
            oichg: oichgBps,
            oichgWhy: oiWhy ?? (oichgBps === null ? 'overflow' : null),
            /* THE RAW ISO DAY, which is what the sort compares and what the
               expiry arithmetic reads. `dayLabel` is applied at the text node
               and nowhere else. */
            exp: r.expiry,
            dte: r.expiry === null ? null : daysBetween(day, r.expiry),
            side: r.side,
            strike: r.strike
          });
        }
      }
      return out;
    });

    /** Bars actually read, and what the census claimed for the same files.
        A PARTITION, and both halves are printed under the grid. */
    const barsRead = $derived(barRows.length);
    const barsClaimed = $derived(
      barState.files.reduce(
        (/** @type {number} */ a, /** @type {any} */ f) => a + (f.error === null ? f.row.rows : 0),
        0
      )
    );

    /* ---------------------------------------------------------------------
       THE BAR GRID'S SORT. Three-way on the KEY, never on the formatted text,
       and an unknown sorts LAST IN BOTH DIRECTIONS - a dash is not a very
       small number, it is not a number.
       --------------------------------------------------------------------- */
    let barSortKey = $state('ts');
    let barDesc = $state(true);

    /* THE DAY WINDOW, APPLIED TO BARS AND NOT TO INSTRUMENT-MONTHS.
     *
     * `scoped` and everything above it counts instrument-MONTHS; a day window
     * over those could only ever narrow to whole months. The question is "show me
     * the 12th", so it is answered where days exist, which is here. */
    /**
     * IS A TIME BOUND ACTUALLY NARROWING ANYTHING.
     *
     * `dayRung` suppresses it rather than testing it: every bar at `1day` opens
     * at the same minute, so a time window there keeps everything or empties the
     * grid, and the Time column is already hidden.
     *
     * Read by the filter AND by the row count, which is the point of naming it —
     * see `pageTotal`, where getting these two out of step drew an empty grid
     * under the words "6,18,296 rows".
     */
    const timeNarrows = $derived(!dayRung && Boolean(db.fromTime || db.toTime));

    /**
     * EVERY MINUTE THE LOADED ROWS ACTUALLY HOLD, newest logic aside — sorted,
     * with how many bars open on each.
     *
     * THE LIST IS THE DATA, NOT A CLOCK. A generic 24x60 picker offers 1,440
     * minutes of which a trading day has 375, so nine out of ten choices would
     * land on nothing and the control would spend most of its surface being
     * wrong. Folding the loaded rows means every row offered is a row that
     * exists, and the count beside it says how many days share that minute.
     *
     * IT SIZES ITSELF TO THE RUNG for free: 375 entries at `1min`, 25 at
     * `15min`, and `Picker`'s filter box handles the long case — the same
     * control, and the same interaction, as every other rung on this strip.
     *
     * O(barRows) AND ONLY WHEN THEY CHANGE. It is a `$derived`, so the walk
     * happens on a new read and not on a re-render; the rows it walks are the
     * ones already in memory for the grid.
     */
    const timeOptions = $derived.by(() => {
      if (dayRung) return [];
      /** @type {Map<string, number>} */
      const seen = new Map();
      for (const b of barRows) {
        const t = timeLabel(b.ts);
        if (t === '—') continue;
        seen.set(t, (seen.get(t) ?? 0) + 1);
      }
      return [...seen.keys()].sort().map((t) => ({
        key: t,
        name: t,
        detail: `${fmt(seen.get(t) ?? 0)} bar(s)`
      }));
    });

    /**
     * THE SAME LIST WITH AN EXPLICIT WAY OUT.
     *
     * `Picker` shows its bulk header only in multi-select, so a `single` menu has
     * no Clear all — and a bound you can set but not unset is a trap. `Any` is a
     * row like the others, in the place a reader already looks, rather than a
     * second gesture they have to know about.
     */
    const timeBounds = $derived([
      { key: '', name: 'Any', detail: 'no bound' },
      ...timeOptions
    ]);

    const barWindowed = $derived.by(() => {
      /* THE TIME PAIR IS PART OF THE SAME WINDOW AND IS APPLIED IN THE SAME
         PASS. A second `.filter` would walk the rows twice to answer one
         question, and on a month of one-minute bars that is 6,840 extra
         comparisons for nothing. */
      const useTime = timeNarrows;
      if (!dayWindowApplies || (!db.fromDay && !db.toDay && !useTime)) return barRows;
      return barRows.filter((b) => {
        if (db.fromDay && b.day < db.fromDay) return false;
        if (db.toDay && b.day > db.toDay) return false;
        if (!useTime) return true;
        /* AT THE RUNG'S RESOLUTION (WP-50, DB-074): a 1s bound is HH:MM:SS. */
        const at = barTimeLabel(b.ts, TF_SECS.get(db.timeframe));
        if (db.fromTime && at < db.fromTime) return false;
        if (db.toTime && at > db.toTime) return false;
        return true;
      });
    });
    const spanCeil = $derived(istDayKey(Date.now()));

    const dayBounds = $derived.by(() => {
      let lo = '';
      let hi = '';
      for (const b of barRows) {
        /* String compare is sound and deliberate: `YYYY-MM-DD` is
           lexicographically ordered, which is the same reason line 216's
           comparison is written this way. No `Date`, so no timezone can move a
           day. */
        if (!b.day) continue;
        if (lo === '' || b.day < lo) lo = b.day;
        if (hi === '' || b.day > hi) hi = b.day;
      }
      return [lo, hi];
    });


    const barSorted = $derived.by(() => {
      const k = barSortKey;
      const dir = barDesc ? -1 : 1;
      return [...barWindowed].sort((a, b) => {
        const A = a[k];
        const B = b[k];
        let d = 0;
        if (A === null || A === undefined) {
          if (B === null || B === undefined) d = 0;
          else return 1;
        } else if (B === null || B === undefined) {
          return -1;
        } else if (k === 'tf') {
          /* BY DURATION, NOT BY SPELLING. `txt` here would order the rungs
             15min, 1day, 1min, 30min - the same defect as a month column
             ordered by `Apr`. */
          d = tfCmp(A, B);
        } else if (typeof A === 'string' || typeof B === 'string') {
          /* `exp` IS THE RAW ISO DAY, so this comparison is chronological. On
             `29 Oct 2026` it would be alphabetical, and wrong. */
          d = txt(String(A), String(B));
        } else {
          /* THREE-WAY, AND NEVER `A - B`. A subtraction hands a comparator a
             magnitude where it asked for a sign, and it is the shape that
             returns a non-zero for two equal values the moment either end
             stops being a small integer. */
          d = A < B ? -1 : A > B ? 1 : 0;
        }
        if (d !== 0) return dir * d;
        /* Deterministic tie-break, NOT inverted with the direction, so two
           runs of one sort produce byte-identical order. */
        return (
          txt(a.instrument, b.instrument) ||
          (a.ts < b.ts ? -1 : a.ts > b.ts ? 1 : 0) ||
          txt(a.rk, b.rk)
        );
      });
    });

    /** @param {string} key */
    function barHead(key) {
      const col = BAR_COLS.find((c) => c.key === key);
      if (!col || col.none || col.nosort) return; /* nothing to order, and the button says why */
      if (barSortKey === key) barDesc = !barDesc;
      else {
        barSortKey = key;
        /* A new column opens on the end that is useful first: newest and
           largest at the top, text A-Z. */
        barDesc = Boolean(col.num) || key === 'ts';
      }
    }
    /** @param {string} key */
    const barAriaSort = (key) => (barSortKey === key ? (barDesc ? 'descending' : 'ascending') : 'none');

    /* ---------------------------------------------------------------------
       PAGING - the same arithmetic for both grids, so the readout under one
       cannot mean something different under the other.
       --------------------------------------------------------------------- */
    /* THE CENSUS COUNT WHEN THE GRID READS ONE PAGE AT A TIME, because
       `barSorted` then holds only the page's own rows and paging by it would
       report "1-50 of 50" over a store of four million bars. When the prefix sum
       does not apply every matched row is in memory and `barSorted` IS the
       count. */
    /* WHAT THE WINDOW ROUTE ALREADY DECIDED, read back off the files it returned.
       `total` is every row the window holds and `extremes` is the widest move in
       it — both folded server-side, so the pager and the magnitude scale get them
       without a second request. Null when the seek route ran.

       DECLARED ABOVE `pageTotal`, WHICH READS IT. `const` is hoisted but not
       initialised, so a derivation evaluated before this line hits the temporal
       dead zone — svelte-check named it exactly: "Block-scoped variable
       'windowSaid' used before its declaration". Order is not decoration where
       one derivation feeds another.

       `pageExact` AND NOT `pagePlan.exact`, AND THE DIFFERENCE IS A CRASH.
       `pagePlan` reads `pageNow`, `pageNow` comes from `pageCount` which comes
       from `pageTotal` — and `pageTotal` reads THIS. Reading `pagePlan` here
       closes that ring, and Svelte resolves it by recursing until `RangeError:
       Maximum call stack size exceeded` and a blank page. It was hit, and the
       page went white. `pageExact` is the same verdict computed one step earlier,
       from the sort key and the day window alone, and depends on no paging state
       — it exists because this cycle was already hit once, on `barTotalRows`. */
    const windowSaid = $derived.by(() => {
      if (pageExact) return null;
      const f = barState.files.find((/** @type {any} */ x) => x && x.total !== undefined);
      return f
        ? { total: Number(f.total) || 0, extremes: f.extremes ?? null, scanned: f.scanned === true }
        : null;
    });

    /**
     * A TIME BOUND IS APPLIED IN THE BROWSER, SO ONLY THE BROWSER CAN COUNT IT.
     *
     * MEASURED, and it is why this arm exists: with a time bound of 09:15–09:20
     * against a page holding 15:26–15:29, the grid drew NOTHING and the line
     * under it read `1–0 of 6,18,296 rows · page 1 of 1,54,574`. Every one of
     * those numbers came from the server, which had never heard of the bound.
     * An empty table under a six-lakh count is the worst reading on this page:
     * it says the store lost the rows.
     *
     * `windowSaid` cannot be made to answer this. It holds the total for the
     * SLICE the window route returned, and the route has no time parameter —
     * see `timePartial` for what that costs and what the page says about it.
     */
    const pageTotal = $derived(
      db.view === 'bars'
        ? timeNarrows
          ? barSorted.length
          : /* THE WINDOW'S OWN TOTAL WHERE IT RAN. Without it the pager counts
               the fifty rows in memory and reports "1-50 of 50" over a store of
               four million — the same defect `barTotalRows` fixes on the seek
               route, arriving by the other road. */
            (windowSaid?.total ?? barTotalRows ?? barSorted.length)
        : db.sorted.length
    );

    /**
     * THE TIME BOUND SAW ONE PAGE, NOT THE QUERY.
     *
     * `/bars/window.json` pages by offset and takes no time parameter, so when
     * it answered, the rows in memory ARE the page — and a bound applied here
     * filters those and nothing else. That is a real limit, not a bug to hide:
     * the same bound over a whole-month read is complete, and the difference is
     * whether the day window is narrow enough to have forced the exact path.
     *
     * Naming it lets the control say which of the two it is doing instead of
     * quietly meaning different things on different days.
     */
    const timePartial = $derived(timeNarrows && windowSaid !== null);
    const pageCount = $derived(Math.max(1, Math.ceil(pageTotal / db.pageSize)));
    const pageNow = $derived(Math.min(Math.max(1, db.page), pageCount));
    const pageFrom = $derived(pageTotal === 0 ? 0 : (pageNow - 1) * db.pageSize + 1);
    const pageTo = $derived(Math.min(pageNow * db.pageSize, pageTotal));
    /* THE SLICE IS RELATIVE TO WHAT WAS FETCHED, NOT TO THE WHOLE QUERY.
       `pagePlan.base` is the global row index the first fetched file opens on,
       so page 900's rows sit at the FRONT of `barSorted` and not 44,950 rows
       into it. Subtracting the base is what turns a global ordinal into an
       offset within the two files actually in memory; without it every page past
       the first slices past the end and the grid draws nothing. */
    /* ======================================================================
       THE MANIFEST DISAGREEMENT — AND IT MUST NOT BE ASKED ON THE WINDOW ROUTE
       ======================================================================
       A file whose bars do not number what the census manifest claims is a fact
       about the STORE and one of the few things this page can notice on the
       operator's behalf, so it is drawn as a warning under the grid.

       THE TEST IS ONLY MEANINGFUL WHEN THE WHOLE MONTH WAS READ. `f.bars.length`
       against `f.row.rows` compares what came back to what the month should
       hold — true for a full read, and nonsense on the window route, where the
       ENDPOINT slices and deliberately returns one page. Measured after sorting
       by Close: "the file returned 9 bar(s) and the census manifest claims
       5,625" — nine being the page size. A false alarm about store integrity,
       raised by the page's own paging.

       It also cost a row. The warning is 38px where the empty notes strip is 17,
       so it pushed `.bnotes` up by 21 and the fit came back with nine rows
       instead of ten — a false warning charging the reader real space.

       `windowSaid !== null` is the same condition `barPage` below uses to decide
       the server already sliced, and for the same reason: it is the one signal
       that says the rows in hand are a page rather than a file. That is why this
       block had to move down here — it now depends on a value declared above
       `barPage` and below where it used to sit. */
    /**
     * IS THE DAY WINDOW NARROWING ANYTHING? One definition, read by the
     * disagreement test below and by `filtered`.
     *
     * NOT "are the fields set": both date controls open seeded at the store's own
     * floor and ceiling, so they are ALWAYS set, and "set" would be true before
     * the operator had touched either. Only a value differing from the full span
     * narrows.
     */
    /* AGAINST THE SPAN THE FIELDS WERE SEEDED WITH (WP-50, DB-045): they open
       on the census span of the selection now, not on 2015 and today. */
    const dayWindowNarrows = $derived(dayWindowApplies && db.dayNarrowing);

    /* AND A DAY WINDOW IS THE SECOND CONDITION THIS TEST CANNOT SURVIVE.
       The paragraph above has the principle exactly right — the comparison means
       something only when the WHOLE MONTH was read — and then guards one of the
       two ways that stops being true.

       `barPlan`'s own notes state the other: "a day window — `barWindowed`
       filters AFTER the fetch, so the census count for a month stops predicting
       how many of its rows survive." Every matched instrument-month is read,
       INCLUDING months lying wholly outside the window, and each of those
       contributes nothing while the census still claims its full count.

       Measured on the operator's screen: ADANIENT 15min narrowed to
       03–28 Aug 2026 drew seven warnings at once — `2026-07` "returned 0 bar(s)
       and the census manifest claims 575", then 06, 05, 04, 03, 02 and 01, each
       with its own number. Seven accusations of a corrupt store, produced
       entirely by the reader's own date filter, under a grid that was answering
       the question correctly. The same false alarm the sort-by-Close case
       raised, arriving by the other road.

       The store fact this exists to catch is not lost: clear the window and the
       whole month is read again, and a real shortfall still says so. */
    const barDisagree = $derived(
      windowSaid !== null || dayWindowNarrows
        ? []
        : barState.files.filter(
            (/** @type {any} */ f) => f.error === null &&
              (f.paged ? f.held !== f.row.rows : f.bars.length !== f.row.rows)
          )
    );

    const barPage = $derived.by(() => {
      /* THE SERVER ALREADY SLICED, SO THE CLIENT MUST NOT SLICE AGAIN — BUT ONLY
         WHERE THE SERVER ACTUALLY ANSWERED.
         On the window route `barSorted` holds the fifty rows that were asked for
         and nothing else, so an offset of 2,99,950 into fifty rows is the empty
         set and the grid would draw nothing on every page but the first.
         `windowSaid` AND NOT `!pagePlan.exact`, because those are no longer the
         same condition: a sort on a derived column is not exact AND does not use
         the window route, so it comes back as a full read that still needs
         slicing. Keying on `!exact` there would pin the grid to page one for
         every column the endpoint cannot order by. */
      if (windowSaid !== null || barState.files.some((f) => f.paged)) return barSorted.slice(0, db.pageSize);
      const first = (pageNow - 1) * db.pageSize;
      const start = Math.max(0, first - pagePlan.base);
      return barSorted.slice(start, start + db.pageSize);
    });

    /** What the last jump has to say — a refusal, or how far it had to go. */
    let jumpWhy = $state('');

    /**
     * PUT THE PAGE ON THE BAR NEAREST A MOMENT. It filters nothing.
     *
     * NEAREST, NOT EXACT, because an exact match is not guaranteed to exist and
     * the honest failure is not "no rows". A halted session, a rung coarser than
     * a minute, or a market that simply had no print at 14:32 all mean the same
     * thing to a reader: show me around there. So it lands on the closest loaded
     * bar and SAYS how far it moved, rather than returning nothing and letting
     * the operator conclude the store is missing the day.
     *
     * IT CAN ONLY REACH WHAT IS IN MEMORY, and that limit is stated rather than
     * worked around. The bars view is server-paged: when the window route
     * answered, `barSorted` IS the page, so there is nothing off-screen to land
     * on and the note says which control fixes that. When the read was whole
     * months, every minute in them is reachable and the page is arithmetic —
     * `pagePlan.base` is the offset of the first loaded row, so the page holding
     * row N is a division, not a search.
     */
    /**
     * @param {string} [at] the minute to land on. Defaults to `goTime` so the
     *   parameter is only ever passed by the control that HAS the new value —
     *   a `$state` write and a read of it in the same handler is a sequencing
     *   question nobody should have to answer while reading a click handler.
     */
    function jumpToTime(at = db.goTime) {
      db.landedRk = '';
      jumpWhy = '';
      const want = minuteOf(at);
      if (want === null) {
        /* UNREACHABLE FROM THE CONTROL, KEPT FOR THE FUNCTION. The picker offers
           only minutes folded out of the loaded rows, so it cannot produce a
           value this rejects. This is a function boundary, and the next caller
           is not required to have read the picker. */
        jumpWhy = `“${String(at).trim()}” is not a time. It must be HH:MM on a 24-hour clock — 14:32.`;
        return;
      }
      if (dayRung) {
        jumpWhy = `Every bar at ${db.timeframe} opens at the same minute, so there is no minute to choose between them.`;
        return;
      }
      if (barSorted.length === 0) {
        jumpWhy = 'No bar is loaded, so there is nothing to land on.';
        return;
      }
      let bestIdx = -1;
      let bestGap = Infinity;
      for (let i = 0; i < barSorted.length; i++) {
        const at = minuteOf(timeLabel(barSorted[i].ts));
        if (at === null) continue;
        const gap = Math.abs(at - want);
        if (gap < bestGap) {
          bestGap = gap;
          bestIdx = i;
        }
      }
      if (bestIdx < 0) {
        jumpWhy = 'No loaded bar carries a minute to compare against.';
        return;
      }
      const hit = barSorted[bestIdx];
      db.landedRk = hit.rk;
      if (windowSaid !== null) {
        jumpWhy = `The server paged this read, so only the ${fmt(barSorted.length)} row(s) on screen could be searched. Narrow the dates until the day is read whole, and every minute in it becomes reachable.`;
        return;
      }
      db.page = Math.floor((pagePlan.base + bestIdx) / db.pageSize) + 1;
      jumpWhy =
        bestGap === 0
          ? ''
          : `No bar opens at ${String(at).trim()}. Landed on ${timeLabel(hit.ts)} — the nearest loaded, ${fmt(bestGap)} minute(s) away.`;
    }

    /** @param {number} n */
    function goPage(n) {
      const want = Number(n);
      db.page = Math.min(pageCount, Math.max(1, Number.isFinite(want) ? Math.trunc(want) : 1));
    }
    /** Keep the reader's PLACE across a page-size change, not their ordinal:
        the row they were standing on stays on screen.
        @param {number} n */
    function setPageSize(n) {
      const firstRow = (pageNow - 1) * db.pageSize;
      db.pageSize = Number(n);
      db.page = Math.floor(firstRow / db.pageSize) + 1;
    }

    /* ONE SCALE PER RUNG, because a rung is what makes two moves comparable.
     *
     * A single scale over the whole selection was the second draft and it was
     * also wrong: with 1min and 1day rows matched together the daily move — the
     * whole session in one bar — set full width, and every minute row drew at
     * under a tenth of it. The column went blank again for the rung anybody
     * actually scans.
     *
     * A minute bar and a daily bar are not two sizes of the same thing, so they
     * do not share a ruler. Keyed on `tf`, each rung is measured against its own
     * largest move and the bar answers "big for a bar of this length".
     */
    /* VOLUME AND RANGE GET THEIR OWN PER-RUNG SCALES, for the reason the change
     * scale has one: a minute's volume and a day's are not two sizes of the same
     * thing, so they do not share a ruler. Folded over `barRows` and never
     * `barPage`, so the scale is every row LOADED and not only the fifty drawn -
     * which since `pagePlan` is the page's own month rather than the whole
     * query. See the change-scale block above for why that trade was taken and
     * what it costs. */
    /* THE SPAN, NOT THE CEILING — AND THE RULER IS LOGARITHMIC. MEASURED.
     *
     * This fold used to keep only the largest volume at each rung and `volMag`
     * divided by it. Against one real page — NSE-INDEX-NIFTY 1min 2026-08, 250
     * rows — the volumes run 2,68,832 to 2,65,70,396: a 98.8x spread around a
     * median of 6,18,458. Divide by the ceiling and the median row gets 2.3% of
     * 56px, which rounds to ONE PIXEL. Measured across those 250 rows:
     *
     *     linear   median  1px   147 of 250 at or under 1px    8 distinct widths
     *     log      median 10px     6 of 250 at or under 1px   30 distinct widths
     *
     * So the bar was drawn on every row and told 59% of them nothing. One
     * afternoon's spike set the ruler for the whole month. A quantity that moves
     * over two orders of magnitude has no linear picture.
     *
     * WHAT THE LOG RULER COSTS, and it is named on the cell rather than left for
     * the reader to assume: a bar twice as long is NOT twice the volume. Reading
     * it as linear is the same error D-0377 deleted a chart for, so the `title`
     * says "log" and gives both ends of the ruler in figures.
     *
     * The floor is the window's own smallest volume rather than zero, because
     * `log(0)` is not a number and a floor of 1 would spend most of the bar's
     * length on decades no bar in the window occupies.
     */
    const volFullByTf = $derived.by(() => {
      /** @type {Map<string, { lo: number, hi: number }>} */
      const span = new Map();
      for (const b of barRows) {
        const v = b.vol ?? 0;
        /* ZERO IS OUT OF THE RULER, NOT CLAMPED INTO IT. A zero volume is a real
           reading — a minute in which nothing traded — and it has no logarithm.
           It draws no bar, which is the truthful picture of it. */
        if (!(v > 0)) continue;
        const s = span.get(b.tf);
        if (s === undefined) span.set(b.tf, { lo: v, hi: v });
        else {
          if (v < s.lo) s.lo = v;
          if (v > s.hi) s.hi = v;
        }
      }
      return span;
    });

    /**
     * Where one volume sits on its rung's log ruler, 0..100.
     *
     * O(1): one Map probe and two logarithms, whatever the window holds.
     *
     * @param {number} v @param {string} tf
     */
    function volMag(v, tf) {
      const s = volFullByTf.get(tf);
      if (s === undefined || !(v > 0)) return 0;
      /* ONE DISTINCT VOLUME IN THE WINDOW IS A FULL BAR, NOT A DIVISION BY ZERO.
         Every row carries the same figure, so every row is the largest. */
      if (s.hi <= s.lo) return 100;
      /* A FLOOR OF 4%, BECAUSE THE SMALLEST VOLUME IN THE WINDOW IS NOT NO
         VOLUME. Mapping straight onto 0..100 gives the minimum row exactly zero
         width — the same picture a bar with no trades gets from the early return
         above. Two different readings, one appearance, which is the collision
         this whole change exists to remove. 4% of 56px is 2px: visible as a mark,
         too short to be mistaken for a quantity. Zero still draws nothing, and
         now that means only zero. */
      const FLOOR = 4;
      const lo = Math.log(s.lo);
      const x = Math.log(Math.min(Math.max(v, s.lo), s.hi));
      return FLOOR + Math.round(((x - lo) / (Math.log(s.hi) - lo)) * (100 - FLOOR));
    }

    /**
     * The volume cell's inline scale, computed ONCE.
     *
     * The markup called `volMag` twice for the same number — once for `--vmag`
     * and once for `--vmagpx`. Two logarithms per cell for one answer.
     *
     * @param {number} v @param {string} tf
     */
    function volStyle(v, tf) {
      const m = volMag(v, tf);
      return `--vmag:${m}%;--vmagpx:${Math.round((m / 100) * 56)}px`;
    }

    /**
     * WHAT THE BAR IN THE VOLUME CELL MEANS, IN WORDS, BECAUSE IT IS NOT LINEAR.
     *
     * A reader who assumes a linear bar reads every ratio wrong, and a picture
     * that invites that is the defect D-0377 removed a chart for. This names the
     * ruler and both its ends, so the bar can be checked rather than trusted.
     *
     * @param {number} v @param {string} tf
     */
    function volTitle(v, tf) {
      const s = volFullByTf.get(tf);
      if (s === undefined || !(v > 0)) {
        return v === 0
          ? 'No volume in this bar. Zero has no place on a log ruler, so no bar is drawn — this is a reading, not a gap.'
          : `${fmt(v)} shares.`;
      }
      return (
        `${fmt(v)} shares. The bar is a LOG scale across every ${tf} row loaded — ` +
        `${fmt(s.lo)} at no bar to ${fmt(s.hi)} at full width — so a bar twice as ` +
        `long is far more than twice the volume. Linear was tried and put 147 of ` +
        `250 rows under one pixel.`
      );
    }

    /* THE BAR'S OWN RANGE, high minus low, as a share of the widest range at that
     * rung. This is what the spine down the left edge is scaled by: a doji reads
     * as a short tick and a wide bar as a tall one, so the column answers "where
     * did anything actually happen" before a single figure is read. */
    const rngFullByTf = $derived.by(() => {
      /** @type {Map<string, number>} */
      const top = new Map();
      for (const b of barRows) {
        const r = b.h - b.l;
        if (r > (top.get(b.tf) ?? 0)) top.set(b.tf, r);
      }
      /* THE WINDOW'S OWN WIDEST RANGE, WHERE THE WINDOW ANSWERED.
         D-0260 traded this away: the seek route reads one month, so the scale
         became "widest move among the bars LOADED" rather than "in the query",
         and both comment blocks here say so. `extremes=1` buys it back — the
         server folds the widest high-low over every month in the window while it
         is already reading them for the sort, and sends one number.
         MEASURED on the running store: 1,50,700 paisa — Rs 1,507.00 — against
         623,498 bars, for no request of its own.
         IT ONLY RAISES. The loaded rows are a SUBSET of the window, so their own
         maximum can never legitimately exceed it; taking the larger of the two
         means a scale that is correct when the server answered and unchanged when
         it did not, rather than a second source of truth. */
      const wide = windowSaid?.extremes?.range;
      if (typeof wide === 'number' && wide > 0) {
        for (const tf of top.keys()) {
          if (wide > (top.get(tf) ?? 0)) top.set(tf, wide);
        }
      }
      return top;
    });

    /** @param {number} r @param {string} tf */
    function rngMag(r, tf) {
      const full = rngFullByTf.get(tf) ?? 0;
      if (full <= 0) return MIN_SPINE;
      /* SQUARE ROOT, NOT LINEAR. Measured on the real page: against the widest
       * 1-minute range in the month, an ordinary minute is under a fifth of it,
       * so a linear scale put all fifty rows on the floor — distinct values: 1.
       * A range is a length and the eye compares lengths by area, so the square
       * root is the honest curve here: it keeps the ordering exact and spends the
       * height where the rows actually are. The widest bar is still full height. */
      return Math.max(MIN_SPINE, Math.sqrt(Math.min(r, full) / full));
    }

    const chgFullByTf = $derived.by(() => {
      /** @type {Map<string, number>} */
      const top = new Map();
      for (const b of barRows) {
        if (b.chg === null) continue;
        const a = Math.abs(b.chg);
        if (a > (top.get(b.tf) ?? 0)) top.set(b.tf, a);
      }
      for (const [tf, v] of top) top.set(tf, Math.max(v, CHG_FLOOR_BPS));
      return top;
    });

    /** The rungs on screen and their scales, for the header's own explanation. */
    const chgScaleText = $derived(
      [...chgFullByTf].map(([tf, v]) => `${tf} ${fmt(v)} bps`).join(', ')
    );

    /**
     * @param {number} bps
     * @param {string} tf
     */
    function chgMag(bps, tf) {
      const full = chgFullByTf.get(tf) ?? CHG_FLOOR_BPS;
      return Math.round((Math.min(Math.abs(bps), full) / full) * 100);
    }

    Object.defineProperties(db, {
      shape: { get: () => shape },
      dayRung: { get: () => dayRung },
      BAR_SHOWN: { get: () => BAR_SHOWN },
      BAR_HIDDEN: { get: () => BAR_HIDDEN },
      BAR_WIDTH: { get: () => BAR_WIDTH },
      GROUP_HEAD: { get: () => GROUP_HEAD },
      readBudget: { get: () => readBudget, set: (v) => (readBudget = v) },
      barPlan: { get: () => barPlan },
      dayNarrows: { get: () => dayNarrows },
      pageExact: { get: () => pageExact },
      barTotalRows: { get: () => barTotalRows },
      pagePlan: { get: () => pagePlan },
      barPlanKeys: { get: () => barPlanKeys },
      barCache: { get: () => barCache },
      barCacheStamp: { get: () => barCacheStamp, set: (v) => (barCacheStamp = v) },
      barState: { get: () => barState, set: (v) => (barState = v) },
      barToken: { get: () => barToken, set: (v) => (barToken = v) },
      readBarFile: { get: () => readBarFile },
      fetchBarFile: { get: () => fetchBarFile },
      readWindow: { get: () => readWindow },
      barFails: { get: () => barFails },
      barFaults: { get: () => barFaults },
      pricePfx: { get: () => pricePfx },
      priceSplit: { get: () => priceSplit },
      barRows: { get: () => barRows },
      barsRead: { get: () => barsRead },
      barsClaimed: { get: () => barsClaimed },
      barSortKey: { get: () => barSortKey, set: (v) => (barSortKey = v) },
      barDesc: { get: () => barDesc, set: (v) => (barDesc = v) },
      timeNarrows: { get: () => timeNarrows },
      timeOptions: { get: () => timeOptions },
      timeBounds: { get: () => timeBounds },
      barWindowed: { get: () => barWindowed },
      spanCeil: { get: () => spanCeil },
      dayBounds: { get: () => dayBounds },
      barSorted: { get: () => barSorted },
      barHead: { get: () => barHead },
      barAriaSort: { get: () => barAriaSort },
      windowSaid: { get: () => windowSaid },
      pageTotal: { get: () => pageTotal },
      timePartial: { get: () => timePartial },
      pageCount: { get: () => pageCount },
      pageNow: { get: () => pageNow },
      pageFrom: { get: () => pageFrom },
      pageTo: { get: () => pageTo },
      dayWindowNarrows: { get: () => dayWindowNarrows },
      barDisagree: { get: () => barDisagree },
      barPage: { get: () => barPage },
      jumpWhy: { get: () => jumpWhy, set: (v) => (jumpWhy = v) },
      jumpToTime: { get: () => jumpToTime },
      goPage: { get: () => goPage },
      setPageSize: { get: () => setPageSize },
      volFullByTf: { get: () => volFullByTf },
      volMag: { get: () => volMag },
      volStyle: { get: () => volStyle },
      volTitle: { get: () => volTitle },
      rngFullByTf: { get: () => rngFullByTf },
      rngMag: { get: () => rngMag },
      chgFullByTf: { get: () => chgFullByTf },
      chgScaleText: { get: () => chgScaleText },
      chgMag: { get: () => chgMag }
    });
  }

  /**
   * FIT HOLDS THE PAGE SIZE TO WHAT THE WINDOW CAN SHOW.
   *
   * Creates the page's effects that stood at old lines 5687; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function barsFit(db) {
    /* THE FIT, APPLIED. Tracks exactly two things — the mode and the measured
       row count — and does everything else inside `untrack`, because
       `setPageSize` READS `pageNow` and `pageSize` and WRITES both. Tracked,
       those reads would make this effect its own trigger and the page would
       re-enter until the stack gave out; this file has closed that ring three
       times already in one session and each time the symptom was a blank page
       rather than a warning.

       THE FEEDBACK PATH THAT WOULD ACTUALLY BITE IS THE OTHER ONE, and it is
       worth naming because it is not obvious: if the box's height depended on how
       many rows were in it, then setting the page size would resize the box,
       which would recompute the fit, which would set the page size. It does not.
       `.tbl` takes its height from `flex` against `.board` — the viewport less
       the chrome — and the chrome does not count rows. The measurement is
       independent of the thing it decides, which is what makes one pass enough. */
    $effect(() => {
      if (db.sizeMode !== 'fit') return;
      /* NOTHING IS APPLIED BEFORE THE BOX HAS BEEN MEASURED. `barBoxH` starts at
         0 and an unmeasured box is not a box that fits nothing — it is a box
         nobody has looked at yet. Without this line the arithmetic reads
         `(0 - 30 - 8) / 40`, the `max(1, …)` floor rounds that up to one, and the
         grid renders a SINGLE row under a pager reporting six hundred thousand.
         Measured, with the tab in the background so the measurement was late.
         Until the first read lands the page keeps its declared default, which is
         a sane grid rather than a broken one. */
      if (db.barBoxH <= 0) return;
      const want = db.rowsThatFit;
      untrack(() => {
        if (want > 0 && want !== db.pageSize) db.setPageSize(want);
      });
    });
  }

  /**
   * THE BAR GRID'S ENTRANCE AND ITS DIRECTION.
   *
   * Creates the page's effects that stood at old lines 7505; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function barsMove(db) {
    /* ---------------------------------------------------------------------
       THE BAR GRID HAS ITS OWN ENTRANCE, and until now it had none at all.

       `entering` above is driven by `stamp`, which is bumped by an effect
       watching the CENSUS rows. Paging the bar grid, re-sorting it, or narrowing
       the query never touched `stamp` — so `class:row-in={entering}` sat on every
       bar row and fired on none of them. MEASURED on a page change: 0 of 50 rows
       animating, with the class in the markup the whole time.

       WHAT IT WATCHES IS THE PAGE'S IDENTITY, not its contents: which page, in
       which order, over which set. Those are the four things that make the rows
       on screen a DIFFERENT fifty rather than the same fifty re-rendered, and an
       entrance that fired on anything less would re-animate under the reader.

       THE MOTION CARRIES A FACT — the operator's own rule. These rows are new;
       the ones that were here are gone. A grid that swaps fifty numbers with no
       transition leaves a reader unsure whether the press registered at all,
       which is the same question `CLAUDE.md` §4 asks about silent failure.
       --------------------------------------------------------------------- */
    let barEntering = $state(false);
    /* ---------------------------------------------------------------------
       AND THE ENTRANCE SAYS WHICH WAY, BECAUSE THE FOUR CAUSES ARE NOT ONE.

       `barEntering` fires for four different reasons and drew the same 4px fade
       for all of them, so a page FORWARD, a page BACK, a re-SORT and a new
       result set were visually identical events. That is motion that announces
       something happened and says nothing about what — the failure this file's
       own note warns about two paragraphs up, arriving by a different door.

       The direction is the fact. Turning to a later page pulls rows up from
       below, so they arrive from below; turning back sends them the other way.
       A re-sort is not travel at all — the same rows in a new order — so it
       settles sideways instead, which reads as "re-ranked" rather than "moved".
       A new set keeps the theme's plain fade, because nothing about it is
       directional.

       THE CAUSE IS DERIVED FROM WHAT CHANGED, not passed in by the caller. Every
       control that can move the grid — the pager, the jump box, the rows-per-page
       picker, a column header, the query — already writes the state this watches,
       so none of them has to remember to describe itself, and one added later
       cannot forget to. The plain `let`s below are deliberately not `$state`:
       they are this effect's memory of its own last run, and making them
       reactive would make the effect its own trigger. */
    let barMove = $state(/** @type {'fwd' | 'back' | 'sort' | 'set'} */ ('set'));
    let lastPageSeen = 1;
    let lastSortSeen = '';
    $effect(() => {
      const p = db.pageNow;
      const s = `${db.barSortKey}|${db.barDesc}`;
      /* `pageTotal` IS THE SET'S IDENTITY, AND TWO BETTER-LOOKING CANDIDATES WERE
         BUILT AND MEASURED FAILING FIRST.
         -------------------------------------------------------------------
         `barPlanKeys` carries the page, the page size and the sort, because its
         job is "does this need a new fetch" — so testing it reported every sort
         AND every page turn as a brand new set.
         `barSetKey`, the FILES alone, looked exactly right and is not: sorting by
         Close cannot be answered from the census prefix sums, so the grid reads a
         different set of files to sort globally. The files change BECAUSE of the
         sort, which is the one thing this test must not confuse.
         The count of matched rows is immune to both. A re-sort is the same rows
         in a new order and a page turn is a window onto them, so neither moves
         it; narrowing the query moves it almost always, and in the case where it
         does not, the page is also unchanged and the last branch says `set`
         anyway. */
      /* Read, not tested: this is what changes when a fetch is needed, so reading
         it keeps the entrance firing on one. */
      void db.barPlanKeys;
      /* THE SORT IS TESTED FIRST, AND IT TOOK THREE TRIES TO PUT IT THERE.
         -------------------------------------------------------------------
         Each earlier order led with a proxy for "is this a different set" and
         each proxy turned out to move BECAUSE of the sort: `barPlanKeys` carries
         the sort by construction; the file list changes because a global sort
         cannot be answered from the census prefix sums; and `pageTotal` changes
         because that same sort is answered by the window endpoint, which reports
         its own count. Three plausible signals, all measured reporting `set` for
         a re-rank.

         `barSortKey` and `barDesc` are not a proxy for the sort — they ARE the
         sort, and nothing else on this page writes them. A narrowing never
         touches them, so leading with them cannot misread a query change, and it
         settles the page-versus-sort ambiguity the same way: a re-sort resets the
         page to 1, and testing the page first would call every sort a backwards
         turn.

         The lesson is the shape rather than the three cases: an event was being
         identified by its side effects when the event itself was already in a
         variable. */
      /* NOTHING IDENTIFIABLE CHANGED IS `null`, NOT `set`, AND THAT ONE LINE
         REPLACES A LATCH, A TIMER AND A ROW-COUNT TEST.
         -------------------------------------------------------------------
         One press produces several runs of this effect: the click, then the
         fetch it triggered landing, then the fit re-applying. Only the FIRST of
         those has anything to identify — by the second, the sort and the page
         have already been recorded as seen. Every earlier version had that run
         fall through to `set` and overwrite the reason the first run got right.

         Each fix for that defended the fall-through instead of removing it: a
         `pageTotal` test to catch the narrowing case (which the fetch also
         trips, since a sorted read is answered by the window endpoint reporting
         its own count), then a latch to suppress re-runs, then a narrower latch
         suppressing only `set`. MEASURED after all three: the first sort of a
         session still reported `set`, because the fetch outlives the latch's
         420ms and writes over it once the latch has lifted.

         `null` says what is actually true — this run identified nothing — and
         leaving `barMove` alone is exactly right, because the previous reason is
         still the reason these rows are on screen. No timer, no latch, no
         inferred state.

         THE COST IS NAMED: a query narrowing is not detected here. It resets the
         page to 1, so from page 5 it reads `back` and from page 1 it keeps the
         previous reason. Both are wrong labels on a real event and neither is
         visible as anything worse than a 10px settle in the wrong direction —
         against a sort cue that was measurably wrong on the first use of the
         page, every session. */
      const reason =
        lastSortSeen === ''
          ? 'set'
          : s !== lastSortSeen
            ? 'sort'
            : p > lastPageSeen
              ? 'fwd'
              : p < lastPageSeen
                ? 'back'
                : null;
      /* THE FIRST RUN OF AN EVENT OWNS THE REASON, AND THE REST OF IT DOES NOT.
         -------------------------------------------------------------------
         One press produces SEVERAL runs of this effect: the click changes the
         sort, then the fetch it triggers settles and `pageTotal` lands, then the
         fit may re-apply. Each re-run recomputed the reason against state the
         press had already moved, so the second run saw the sort as unchanged —
         it had been recorded — and the newly-arrived total as changed, and wrote
         `set` over the `sort` the first run got right. MEASURED: the first sort
         of a column reported `set`, while REVERSING that same column reported
         `sort` correctly, because reversing needs no new fetch and produces only
         one run. Two behaviours for one control, decided by whether the data
         happened to be in hand.

         So the reason latches for the length of the entrance. `entranceActive` is
         a plain `let` and not `$state` deliberately: this effect writes
         `barEntering`, and reading a reactive twin of it here would make the
         effect its own trigger. The cleanup clears the timer without clearing the
         latch, so a burst of re-runs from one press extends the window rather
         than reopening it, and the latch lifts 420ms after the last of them. */
      if (reason) barMove = reason;
      lastPageSeen = p;
      lastSortSeen = s;
      barEntering = true;
      const t = setTimeout(() => (barEntering = false), 420);
      return () => clearTimeout(t);
    });

    Object.defineProperties(db, {
      barEntering: { get: () => barEntering, set: (v) => (barEntering = v) },
      barMove: { get: () => barMove, set: (v) => (barMove = v) },
      lastPageSeen: { get: () => lastPageSeen, set: (v) => (lastPageSeen = v) },
      lastSortSeen: { get: () => lastSortSeen, set: (v) => (lastSortSeen = v) }
    });
  }

  export { pager };
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

  <!-- ==================================================================
       THE BAR GRID — ONE ROW PER BAR, twenty-four columns, read from
       `/bars.json`.

       A REAL `<table>`, and the census grid beside it is still a grid of
       divs. The two are not inconsistent by accident: the census grid is
       WINDOWED — rows absolutely positioned inside a spacer — and a
       `<tbody>` cannot be windowed that way without lying to the row
       model. This grid is PAGED instead, so the whole page is in the
       document and the browser's own column algorithm can do the work
       that the census grid has to do with `grid-template-columns`.

       TEN OF THE TWENTY-FOUR COLUMNS HAVE NO SOURCE ON THIS WIRE and
       every one of them says so, on the header and in every cell.
       `CLAUDE.md` §4: degrade loudly and name the reason.
       ================================================================== -->

<!-- ==================================================================
     THE WINDOW'S SHAPE, ABOVE THE NUMBERS.

     One block per month in the selection, oldest to newest, its height the
     month's share of the fullest month in the same selection. It answers
     at a glance the question the table answers one row at a time: is this
     window solid, or does it have holes in it.

     DISPLAY ONLY. No block is a click target, and that is deliberate: the
     page already has one control that chooses what is DRAWN (the query) and
     adding a second that narrows it from here would be two answers to one
     question — the same rule `/ingest` keeps about its verdict strip.
     ================================================================== -->
<!-- ==================================================================
     THE FOUR FACTS, BEFORE ANY DIGIT.

     WHAT THIS FIXES. The page went straight from a form to five hundred
     numbers. Nothing above the grid said what you were looking at, so the
     first thing a reader did on every visit was reconstruct it from the
     controls: which instrument, how deep the history goes, how much of it
     there is, where it ends. Four questions, answered by reading a form.

     They are answered here instead, at a size that is read rather than
     scanned. Every one is already in memory — `coverBand` folds the census
     the page has loaded, and the last bar is the first row of the grid when
     it is in the store's own order. No request is added.

     THE NUMBERS ARE THE DESIGN. There is no ornament here: each figure is
     the largest thing in its own cell because it is the thing worth
     reading, and the label under it is small because you only need it once.
     ================================================================== -->
<!-- COMPACT FOLDS THE COUNTED LINE AND SLIMS THE BAND. IT DOES NOT DELETE
     THE BAND, AND THE FIRST VERSION DID.
     ------------------------------------------------------------------
     Both panels answer questions about the selection rather than change
     it, which is what makes them the part a reader may spend for rows —
     but they are not the same KIND of thing, and treating them as one
     decision threw away the wrong one. The counted line is four figures
     and its labels: text, and text a reader has already read. The window
     band is the only PICTURE on this page — eighty-one months of coverage
     in one glance, the thing that answers "where are the holes" without
     reading a single number.

     Folding both bought rows and removed the visualisation, on exactly the
     windows too small to show much else. So the band stays and loses its
     heading and its legend instead: 103px becomes ~40, the rail itself is
     untouched, and what goes is the chrome around it rather than the data
     in it. The legend's key is recoverable — the band keeps its `aria-label`
     and the rail its per-cell titles.

     Nothing that narrows the query is inside this block. -->

<!-- THE COUNTED LINE IS GONE; ITS LAST TWO READOUTS ARE ON THE ANCHOR.
     It began as six tiles, became four facts, and this sequence removed
     the two that were the anchor tabs said twice. What was left could not
     fill a 1576px row: two items stretched to 787px each, one at each
     edge. They are inline on the anchor now, beside the tabs they
     describe, and the row and its 62px are gone. -->

<!-- THE COVERAGE BAND IS GONE.
     It drew one bar per month, scaled against the fullest month in the
     selection, and at 81 months that is 81 slivers a few pixels wide — a
     shape rather than a reading. Every figure it carried is still on the
     page and in a form that can be read rather than estimated: the month
     count and the bar total are the counted line above it, and a month's
     own rows are a row of the census grid, which is the view the coverage
     tab exists to show.
     WHAT IT COST WAS THE FOLD. On an 780px window the table was drawing
     THREE rows, and this rail plus its heading and legend was the tallest
     thing between the query and the data. A picture that is only a picture
     does not outrank the rows on a page whose whole job is the rows.
     `coverBand` ITSELF STAYS AND IS STILL READ — the anchor prints the
     first and last month of the selection out of its cells, and `factSeen`
     folds its bar and month totals. Only the rail was removed.
     This line used to say `.facts` reads those totals. That row moved to
     the anchor in the same sequence, so the sentence outlived its subject;
     it is corrected here rather than left to be believed. -->


<DbChart {db} />
<DbExtremes {db} />
<DbDayFlags {db} />
<div class="tbl">
    <!-- THE FIT IS PUBLISHED ON THE ELEMENT, AND IT IS KEPT RATHER THAN
         SWEPT UP AFTERWARDS. `data-boxh` and `data-fit` went in as debug
         scaffolding and earned their place: every failure in this one
         measurement — a binding that reported only on mount, an observer
         that fired a thousand times in a loop, a background tab that left
         the height at zero — presented as a PLAUSIBLE row count with
         nothing to check it against. Each was pinned only once the measured
         height and the derived fit could be read from outside. Two
         attributes, and they are the difference between "the grid looks
         short" and "it read 682 and fitted 15 rows into 665". -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="tbl-scroll bscroll"
      bind:this={db.barBox}
      data-fit={db.rowsThatFit}
      data-boxh={db.barBoxH}
      tabindex="0"
      role="region"
      aria-label="Stored bars, {fmt(db.barPage.length)} on this page of {fmt(
        db.pageTotal
      )}. Sorted by {BAR_META[db.barSortKey]?.label ?? db.barSortKey}."
    >
      <table
        class="bgrid lean"
        class:noday={db.dayRung}
        class:nofuture={!db.shape.contract}
        class:nooption={!db.shape.option}
        class:norung={db.timeframe !== ''}
        data-move={db.barMove}
        data-ungrouped={BAR_UNGROUPED.length ? BAR_UNGROUPED.join(',') : undefined}
        style="min-width:{db.BAR_WIDTH}px"
      >
        <colgroup>
          {#each BAR_COLS as c (c.key)}
            <col style="width:{c.w}px" />
          {/each}
        </colgroup>
        <thead>
          <!-- THE GROUP BAND. One cell per column so it cannot drift out of
               step with the grid — see the note over `BAR_GROUPS` for why
               `colspan` is wrong here. `aria-hidden` because the band is a
               visual chunking of headings the screen reader already reads
               in order; announcing "When" before every date would be noise,
               and the column headers below carry the real names. -->
          <tr class="gband" aria-hidden="true">
            {#each BAR_COLS as c (c.key)}
              <th scope="col">
                {#if db.GROUP_HEAD.has(c.key)}
                  <span class="glab">{db.GROUP_HEAD.get(c.key)?.label}</span>
                {/if}
              </th>
            {/each}
          </tr>
          <tr>
            {#each BAR_COLS as c (c.key)}
              <th
                scope="col"
                class:numh={c.num}
                class:deadh={Boolean(c.none)}
                aria-sort={c.none || c.nosort ? undefined : db.barAriaSort(c.key)}
              >
                <!-- A COLUMN WITH NO SOURCE DOES NOT PRETEND TO SORT. The
                     button is still here, still reachable and still
                     labelled — it is DISABLED and it carries the reason on
                     its own title, which is the difference between a
                     control that is missing and one that is refused. -->
                <button
                  class="sortbtn"
                  type="button"
                  disabled={Boolean(c.none || c.nosort)}
                  title={c.none
                    ? `${c.label} — no source, so there is nothing to order: ${c.none}`
                    : c.nosort
                      ? `${c.label} — ${c.why}. Not sortable: ${c.nosort}`
                      : c.key === 'chg'
                      ? `${c.label} — ${c.why}. The bar behind each figure is that figure's size against the largest move in this selection, measured PER RUNG so a minute bar is never judged against a daily one (${db.chgScaleText}). Scaled across every matched row rather than this page, so turning the page does not change what a full bar means. Click to sort; click again to reverse.`
                      : `${c.label} — ${c.why}. Click to sort; click again to reverse.`}
                  onclick={() => db.barHead(c.key)}
                >
                  <span>{c.label}</span>
                  <span
                    class="caret"
                    class:on={db.barSortKey === c.key && !c.none && !c.nosort}
                    class:desc={db.barDesc}
                    aria-hidden="true"
                  ></span>
                </button>
              </th>
            {/each}
          </tr>
        </thead>

        <tbody>
          {#if db.barState.error}
            <tr class="bstate">
              <td colspan={BAR_COLS.length}>
                <div class="bnone" role="alert">
                  <h3>The bar files could not be read</h3>
                  <p class="err">{db.barState.error}</p>
                  <p>
                    Nothing is drawn rather than a plausible blank: a grid with no rows and no
                    reason reads as an empty store, and this is a failed read.
                  </p>
                  <button class="btn" onclick={() => refreshStore()}>Try again</button>
                </div>
              </td>
            </tr>
          {:else if db.barPlan.all.length === 0}
            <tr class="bstate">
              <td colspan={BAR_COLS.length}>
                <div class="bnone">
                  <h3>No instrument-month matches this query</h3>
                  <p>
                    A bar lives in an instrument-month file, so with none matched there is no
                    file to open — this grid has made no request. {db.feedName} holds {fmt(
                      db.deco.length
                    )} instrument-month(s) in total.
                  </p>
                  <button class="btn" onclick={db.reset}>Clear the filters</button>
                </div>
              </td>
            </tr>
          {:else if db.barState.loading && db.barPage.length === 0}
            {#each SKELETON.slice(0, 12) as i (i)}
              <tr class="brow skelrow">
                {#each BAR_COLS as c, ci (c.key)}
                  <td class:bn={c.num}
                    ><span
                      class="skel line"
                      style="width:{46 + ((i * 13 + ci * 7) % 44)}%;animation-delay:{i * 60}ms"
                    ></span></td
                  >
                {/each}
              </tr>
            {/each}
          {:else if db.barPage.length === 0}
            <tr class="bstate">
              <td colspan={BAR_COLS.length}>
                <div class="bnone">
                  <!-- THE DAY WINDOW GETS ITS OWN BRANCH, ABOVE THE ONE
                       THAT ACCUSES THE STORE.

                       MEASURED by typing a 2016 range into the date fields
                       over a store whose first bar is 2019-12: the read
                       worked, 81 files opened, 6,23,498 bars arrived, and
                       `barWindowed` then filtered every one of them out on
                       the day. The message drawn was:

                         "6,23,498 bar(s) arrived. The file(s) opened and
                          held no record. The census says they hold
                          6,23,498, so this is a disagreement between the
                          manifest and the file, not an empty query."

                       Self-contradictory in its first two sentences — bars
                       arrived AND no record was held — and then it accuses
                       the store of CORRUPTION for what a filter did. An
                       operator reading that would go looking for a damaged
                       file that is perfectly intact.

                       The bars-arrived count is what separates the two: if
                       rows came back and none survived, the window is the
                       answer, and the store is not in question at all. -->
                  <h3>
                    {db.barsRead > 0 && db.dayNarrows
                      ? 'No bar in this date window'
                      : 'No bar came back'}
                  </h3>
                  <p>
                    {fmt(db.barPlan.read.length)} instrument-month(s) were read and
                    <b>{fmt(db.barsRead)}</b> bar(s) arrived.
                    {#if db.barFails.length}
                      {fmt(db.barFails.length)} of those file(s) refused — each refusal is named
                      under the grid.
                    {:else if db.barsRead > 0 && db.dayNarrows}
                      Every one of them falls outside <b>{db.fromDay || 'the start'}</b> to
                      <b>{db.toDay || 'the end'}</b>, so the grid has nothing to draw. The files
                      and the census agree — it is the date window above that is empty, and
                      widening it brings these {fmt(db.barsRead)} bar(s) back.
                    {:else}
                      The file(s) opened and held no record. The census says they hold {fmt(
                        db.barsClaimed
                      )}, so this is a disagreement between the manifest and the file, not an
                      empty query.
                    {/if}
                  </p>
                  <button class="btn" onclick={() => refreshStore()}>Read again</button>
                </div>
              </td>
            </tr>
          {:else}
            {#key db.stamp}
              {#each db.barPage as b, i (b.rk)}
                <tr
                  class="brow"
                  class:odd={i % 2 === 1}
                  class:landed={b.rk === db.landedRk}
                  class:row-in={db.barEntering}
                  class:dayfirst={db.barSortKey === 'ts' &&
                    i > 0 &&
                    db.barPage[i - 1].day !== b.day}
                  data-dir={b.c > b.o ? 'up' : b.c < b.o ? 'down' : 'flat'}
                  style="--rng:{db.rngMag(b.h - b.l, b.tf)};--spine:{Math.round(
                    db.rngMag(b.h - b.l, b.tf) * 24
                  )}px;--in-delay:{Math.min(i * 8, 160)}ms"
                >
                  <!-- DATE / TIME. The stamp is IST and says so; the rung
                       and the instrument that produced the bar are on the
                       title, because a grid that mixes two files must be
                       able to say which row came from which. -->
                  <!-- THE DAY IS DIMMED WHERE IT REPEATS, AND NEVER
                       REMOVED. Measured on this page: fifty rows of
                       one-minute bars carried ONE distinct date — the same
                       `21 Aug 2026` fifty times — which is the shape line
                       3308 already refuses for another column, "repeat one
                       value down every row and earn none of its width".

                       DIMMED, NOT BLANKED. The text stays in the cell so a
                       copy, an export and a screen reader all still get a
                       date on every row; only its weight in the eye
                       changes. A blank cell would make the fiftieth row
                       unidentifiable on its own.

                       ONLY WHEN THE GRID IS IN TIME ORDER. Sorted by close,
                       two adjacent rows are two unrelated minutes, so
                       "same as the one above" is not a day boundary and
                       dimming on it would hide dates at random. -->
                  <td
                    class="bt"
                    class:dayrep={db.barSortKey === 'ts' && i > 0 && db.barPage[i - 1].day === b.day}
                    title="{stampLabel(b.ts)} IST · {b.tf} bar · {b.instrument} · month file {b.month}"
                    >{dayLabel(b.ts)}</td
                  >
                  <!-- ALWAYS RENDERED, HIDDEN BY CLASS. This grid hides a
                       column with positional CSS on `.bgrid`, so header and
                       body must agree cell-for-cell; an `{#if}` here dropped
                       the body's cell while the header kept its own and the
                       two rows slid apart by one. -->
                  <!-- TO THE SECOND ON A ONE-SECOND RUNG: sixty 1s bars
                       share every HH:MM, so a minute label could not tell
                       them apart. `barTimeLabel` keys on the rung's length
                       from `TF_SECS`, the one table of rung lengths. -->
                  <td class="bt" title="{dayLabel(b.ts)}, {barTimeLabel(b.ts, TF_SECS.get(b.tf))} IST — the bar's opening {(TF_SECS.get(b.tf) ?? 60) < 60 ? 'second' : 'minute'}"
                    >{barTimeLabel(b.ts, TF_SECS.get(b.tf))}</td
                  >
                  <td class="bt">{b.tf}</td>
                  <!-- THE SHARED OPENING IS DIMMED IN PLACE. `.pfx` is a
                       span inside the cell, so the cell's own text is
                       unchanged for a copy, an export, `textContent` and a
                       screen reader — see `pricePfx`. -->
                  <!-- THE POSITION DOTS ARE GONE, AND THE MEASUREMENT THAT
                       REMOVED THEM IS WORTH KEEPING. Each price cell drew a
                       mark at its value's place in the page's low-to-high
                       range. Counted on a live page: 20 dots, TWO distinct
                       positions — 18 at 0% and 2 at 100%.
                       That is the encoding collapsing, not a rendering
                       fault. Within one page of a quiet minute series every
                       price IS either the page low or the page high, so
                       position carries one bit and spends twenty marks
                       saying it. A visual that is noise plus a rationale is
                       worse than plain noise: the rationale is what stops
                       the next reader deleting it.
                       The window's shape is on the price line above the
                       grid, where 240 samples make it visible. Per-cell,
                       the honest treatment is the one that was already
                       here: the shared prefix dimmed so only the digits
                       that differ are dark. -->
                  <td class="bn" title="{fmt(b.o)} paisa, as stored"
                    ><span class="pfx">{db.priceSplit(b.o)[0]}</span>{db.priceSplit(b.o)[1]}</td
                  >
                  <td class="bn" title="{fmt(b.h)} paisa, as stored"
                    ><span class="pfx">{db.priceSplit(b.h)[0]}</span>{db.priceSplit(b.h)[1]}</td
                  >
                  <td class="bn" title="{fmt(b.l)} paisa, as stored"
                    ><span class="pfx">{db.priceSplit(b.l)[0]}</span>{db.priceSplit(b.l)[1]}</td
                  >
                  <!-- THE CLOSE IS THE FIGURE THE TABLE IS READ FOR, and it
                       was the same weight and colour as the other three.
                       `bclose` lets the row's own `data-dir` reach it — the
                       convention `bars.rs` already renders on the
                       server-side price table, where `td.close` is bold and
                       tinted by `close >= open`. The two surfaces show the
                       same store and disagreed about that. -->
                  <td class="bn bclose" title="{fmt(b.c)} paisa, as stored"
                    ><span class="pfx">{db.priceSplit(b.c)[0]}</span>{db.priceSplit(b.c)[1]}</td
                  >
                  <!-- VOLUME. The bar behind the figure is a LOG ruler and
                       the title says so — see `volMag` for the measurement
                       that forced it off linear, and D-0377 for what an
                       unlabelled scale costs. `volStyle` folds what used to
                       be two identical `volMag` calls into one. -->
                  <td class="bn bvol" style={db.volStyle(b.vol, b.tf)} title={db.volTitle(b.vol, b.tf)}>{fmt(b.vol)}</td>

                  <!-- PRE-MARKET %: no source on this wire. -->
                  <td class="bn na"
                    ><span
                      class="unk"
                      title={BAR_META.premkt.none}
                      aria-label="Pre-mkt %: no source — {BAR_META.premkt.none}">no source</span
                    ></td
                  >

                  <!-- THE NUMBER, AND THE SAME NUMBER AS A LENGTH.
                       A column of ±0.05% reads as a column of identical
                       text; the bar behind it is the only thing that makes
                       an outlier findable by eye at fifty rows a page. It
                       is drawn from the SAME `b.chg` the text renders and
                       carries no scale of its own — see `--mag`. -->
                  <td
                    class="bn bpc"
                    data-dir={dirOf(b.chg)}
                    style={b.chg === null
                      ? undefined
                      : `--mag:${db.chgMag(b.chg, b.tf)}%;--magpx:${Math.round(
                          (db.chgMag(b.chg, b.tf) / 100) * 44
                        )}px`}
                  >
                    {#if b.chg === null}
                      <span
                        class="unk"
                        title={barWhyText(b.chgWhy)}
                        aria-label={barWhyText(b.chgWhy)}>—</span
                      >
                    {:else}
                      {bpsText(b.chg)}
                    {/if}
                  </td>

                  <!-- OPEN INTEREST. `i64::MIN` is the null sentinel and it
                       renders as an unknown that names itself. A ZERO IS A
                       ZERO and renders as one — the two are different facts
                       and this column has to keep them apart. -->
                  <td class="bn">
                    {#if b.oi === null}
                      <span
                        class="unk"
                        title={barWhyText('oi_null')}
                        aria-label={barWhyText('oi_null')}>unknown</span
                      >
                    {:else}
                      <span
                        title={b.oi === 0
                          ? 'zero, and stored as zero — not the null sentinel'
                          : 'open interest, as stored'}>{fmt(b.oi)}</span
                      >
                    {/if}
                  </td>

                  <td class="bn bpc" data-dir={dirOf(b.oichg)}>
                    {#if b.oichg === null}
                      <span
                        class="unk"
                        title={barWhyText(b.oichgWhy)}
                        aria-label={barWhyText(b.oichgWhy)}>—</span
                      >
                    {:else}
                      {bpsText(b.oichg)}
                    {/if}
                  </td>

                  <!-- EXPIRY. The row keeps the raw ISO day — that is what
                       the sort compares and what the day arithmetic reads —
                       and it becomes `29 Oct 2026` here and only here. -->
                  <td class="bt">
                    {#if b.exp === null}
                      <span
                        class="unk"
                        title="this instrument is not a contract — an index and an equity have no expiry. It is not an unknown date."
                        >—</span
                      >
                    {:else}
                      <span title="the store's own key for it is {b.exp}">{dayLabel(b.exp)}</span
                      >
                    {/if}
                  </td>
                  <td class="bn">
                    {#if b.dte === null}
                      <span
                        class="unk"
                        title="no expiry on this instrument, so there is no distance to one."
                        >—</span
                      >
                    {:else}
                      <span
                        title={b.dte < 0
                          ? `${fmt(-b.dte)} day(s) AFTER expiry — this bar is from the contract's life, read out of a month file that outlives it`
                          : 'whole calendar days from this bar to expiry'}>{fmt(b.dte)}d</span
                      >
                    {/if}
                  </td>
                  <DbOptionCells {db} {b} />
                </tr>
              {/each}
            {/key}
          {/if}
        </tbody>
      </table>
    </div>

    <!-- EVERY REFUSAL, EVERY PARTIAL READ AND EVERY DISAGREEMENT, NAMED.
         None of these is allowed to be a shorter grid. -->
    {#if db.barFails.length || db.barFaults.length || db.barDisagree.length || db.barsRead > 0}
      <div class="bnotes">
        {#if db.barsRead > 0}
  <!-- THE COLUMNS NOTE IS GONE. "10 of 24 columns drawn, the rest have
       nothing to put in them" is a sentence about the page rather than
       about the store, and the columns that are drawn are self-evident. -->

        {/if}
        {#each db.barDisagree as f (f.key)}
          <p class="bnote warn">
            <b>{f.row.instrument} {f.row.month} {f.row.timeframe}</b> — the file reports {fmt(
              f.paged ? f.held : f.bars.length
            )} bar(s) and the census manifest claims {fmt(f.row.rows)}. The grid shows what the
            FILE returned; its recorded total differs from the census.
          </p>
        {/each}
        {#each db.barFaults as f (f.key)}
          <p class="bnote warn">
            <b>{f.row.instrument} {f.row.month} {f.row.timeframe}</b> — read in part: {f.faults}
          </p>
        {/each}
        {#each db.barFails as f (f.key)}
          <p class="bnote err">
            <b>{f.row.instrument} {f.row.month} {f.row.timeframe}</b> — {f.error}
          </p>
        {/each}
      </div>
    {/if}
  </div>
<DbProvenance {db} />

{#snippet pager(/** @type {any} */ db)}
  <!-- ==================================================================
       THE PAGER — how you reach the rows that are not on screen.

       IT GOVERNS BOTH GRIDS, so `1–50 of 8,250 rows` means the same thing
       under either one and the arithmetic is written once. `pageNow` is a
       CLAMP and not a write-back: a narrowing that shortens the run
       cannot strand the reader on a page that no longer exists.

       EVERY DISABLED BUTTON SAYS WHY, on its own title. A greyed control
       with no reason is the silent degradation §4 bans; "already on the
       first page" is one word longer and answers the question.
       ================================================================== -->
  <div class="pgbar" role="group" aria-label="Paging">
    <span class="pgof">
      {#if db.pageTotal === 0}
        <b>0</b> rows — {db.view === 'bars'
          ? 'no bar was read, so there is no page to turn'
          : 'nothing matched, so there is no page to turn'}
      {:else}
        <b>{fmt(db.pageFrom)}–{fmt(db.pageTo)}</b> of <b>{fmt(db.pageTotal)}</b> rows · page
        <b>{fmt(db.pageNow)}</b> of <b>{fmt(db.pageCount)}</b>
      {/if}
    </span>

    <!-- THE LAST OS-DRAWN MENU IN THIS PRODUCT. `<select>` styles its closed
         face and NOT its open list -- that popup belongs to the operating
         system -- so this one control kept rendering the OS blue highlight
         under a page that draws everything else itself. Same fix as both
         calendars: `Picker`, single, with each row's own sentence carried
         across as its `why` rather than an `<option title>` no reader hovers. -->
    <div class="pgsize">
      <span>Rows per page</span>
      <!-- `why` MOVED TO `title`, AND THE MENU STOPPED COVERING THE PAGE.
           Every one of the five rows carried a three-line paragraph, so each
           row stood about 120px and the panel wanted 600 — capped to 380 and,
           opening upward from a control at the foot of the page, it
           blanketed the entire query strip. Reported exactly that way: the
           sections above were hidden.
           `why` IS FOR A REFUSAL, WHICH NONE OF THESE ARE. `Picker`'s own
           header says it is the visible sentence on a row that "exists and
           you cannot have it" — a row drawn dead in its own place carrying
           its reason. `Fit`, `25`, `50`, `100` and `250` are all choosable,
           and four of them are a number that explains itself.
           The sentences are not deleted: each is on its row's `title`, which
           is where an explanation of a WORKING control belongs. `Fit` keeps
           its measured count as `detail`, because that is the row's value
           rather than a note about it. -->
      <Picker
        single
        label="page sizes"
        summary={db.sizeMode === 'fit' ? `Fit · ${fmt(db.pageSize)}` : fmt(db.pageSize)}
        title="How many rows this page holds. The reader's place is kept across a change: the row you were standing on stays on screen."
        rows={[
          {
            key: 'fit',
            name: 'Fit',
            detail: `${fmt(db.rowsThatFit)} row(s)`,
            title: `As many rows as this window can SHOW — measured now at ${fmt(db.rowsThatFit)}. The grid then holds nothing the screen cannot, so the table does not scroll and the page is one surface rather than two. Re-measured when the window changes.`
          },
          ...PAGE_SIZES.map((n) => ({
            key: String(n),
            name: fmt(n),
            detail: n >= 1000 ? `${fmt(n * BAR_COLS.length)} cells` : undefined,
            title:
              n >= 1000
                ? `${fmt(n)} rows in one document. On the bar grid that is ${fmt(n * BAR_COLS.length)} cells — legible, and slow to lay out.`
                : n > db.rowsThatFit
                  ? `${fmt(n)} rows into a box that can show ${fmt(db.rowsThatFit)}. The remainder scrolls INSIDE the table, which is a second scrollbar — deliberate here, and the reason Fit exists.`
                  : undefined
          }))
        ]}
        selected={new Set([db.sizeMode === 'fit' ? 'fit' : String(db.pageSize)])}
        onchange={(/** @type {Set<string>} */ sel) => {
          const key = [...sel][0];
          if (key === 'fit') {
            /* No `setPageSize` here: the effect owns the value in this mode,
               and setting it twice would move the reader's place twice. */
            db.sizeMode = 'fit';
            return;
          }
          const n = Number(key);
          if (Number.isFinite(n) && n > 0) {
            db.sizeMode = 'fixed';
            db.setPageSize(n);
          }
        }}
      />
    </div>

    <label class="pgjump">
      <span>Go to page</span>
      <input
        type="number"
        min="1"
        max={db.pageCount}
        value={db.pageNow}
        disabled={db.pageTotal === 0}
        title={db.pageTotal === 0
          ? 'There is no page to go to: nothing is on this grid.'
          : `Any page from 1 to ${fmt(db.pageCount)}. Out-of-range is clamped, never refused.`}
        onchange={(e) => db.goPage(Number(e.currentTarget.value))}
      />
      <span class="pgof2">of {fmt(db.pageCount)}</span>
    </label>

    <span class="pgnav">
      <button
        class="pg"
        type="button"
        disabled={db.pageNow === 1}
        title={db.pageNow === 1 ? 'Already on the first page' : 'First page'}
        aria-label="First page"
        onclick={() => db.goPage(1)}>«</button
      >
      <button
        class="pg"
        type="button"
        disabled={db.pageNow === 1}
        title={db.pageNow === 1 ? 'Already on the first page' : `Page ${fmt(db.pageNow - 1)}`}
        aria-label="Previous page"
        onclick={() => db.goPage(db.pageNow - 1)}>‹</button
      >
      {#each pageList(db.pageNow, db.pageCount) as item (item.k)}
        {#if item.gap}
          <!-- ELIDED, AND IT IS NOT A CONTROL. A 1,840-page run must not
               render 1,840 buttons; the jump box above reaches the pages
               this row does not draw, which is why the elision is honest. -->
          <span class="pg gap" aria-hidden="true">…</span>
        {:else}
          <button
            class="pg"
            type="button"
            aria-current={item.p === db.pageNow ? 'page' : undefined}
            title="Page {fmt(item.p)} of {fmt(db.pageCount)}"
            onclick={() => db.goPage(item.p)}>{fmt(item.p)}</button
          >
        {/if}
      {/each}
      <button
        class="pg"
        type="button"
        disabled={db.pageNow === db.pageCount}
        title={db.pageNow === db.pageCount
          ? db.pageCount === 1
            ? 'There is only one page'
            : 'Already on the last page'
          : `Page ${fmt(db.pageNow + 1)}`}
        aria-label="Next page"
        onclick={() => db.goPage(db.pageNow + 1)}>›</button
      >
      <button
        class="pg"
        type="button"
        disabled={db.pageNow === db.pageCount}
        title={db.pageNow === db.pageCount
          ? db.pageCount === 1
            ? 'There is only one page'
            : 'Already on the last page'
          : `Last page, ${fmt(db.pageCount)}`}
        aria-label="Last page"
        onclick={() => db.goPage(db.pageCount)}>»</button
      >
    </span>
  </div>

  <!-- THE STANDING BAR UNDER THE PAGER. It is not a second pager and it
       draws no page buttons: the pager above owns the ordinals. What this
       carries is what the pager cannot — how much of the page is actually
       in the document, the newest instant in the whole matched set, and
       the keys that move a reader through it.

       IT GAINED THE NEWEST BAR, which is the one fact the standing "newest
       bar" notice carried that nothing else on this page stated: `last_ts`
       was read per row (the month cell's tooltip) and per instrument (the
       drawer's "Days held") and never for the SET on screen, which is the
       question a banner over a table is actually asked. Counted from the
       rows, so it narrows with them; an absence is named and never rendered
       as an epoch. -->
  <!-- THE SECOND PAGER IS GONE, AND IT WAS SAYING THE FIRST ONE'S NUMBER.
       `<p class="pager">` stood here holding one span: `{fmt(pageTotal)}
       row(s)`. Thirty pixels above it `.pgbar`'s own readout already ends
       "…of <b>4,225,185</b> rows", from the SAME `pageTotal`. Two elements,
       one value, stacked — a reader who spots the difference is looking for
       a difference that cannot exist.

       It had already been trimmed once, down to the row count from a banner
       carrying DOM counts and the newest bar; the honest end of that trim is
       zero, because what remained was a duplicate rather than a small fact.
       Worth 47px of the height the table's 25-row floor needs, but it would
       have gone anyway. -->
{/snippet}

<style>
  /* ---------------------------------------------------------------------
     THE WINDOW BAND. One block per month, height = share of the fullest month
     in the same selection.

     EVERY COLOUR IS A TOKEN, never a literal, so the band resolves in both
     themes from the same ramp the rest of the console uses. `--up` for a month
     that is held, `--warn` for one that is short, `--line-hard` for one holding
     nothing — the same three readings `/ingest` gives a coverage cell.
     --------------------------------------------------------------------- */
  /* ---------------------------------------------------------------------
     TWO THINGS THIS GRID ALREADY MEASURED AND NEVER DREW.

     `data-dir` has been on every row since the grid was written — `up`, `down`
     or `flat`, from `close` against `open` — and no rule read it. `--rng` is
     set from `rngMag()` on every row, which folds the widest high-low at that
     rung, and no rule read that either. Both were computed per row, per paint,
     and thrown away.

     So the close — the one figure a price table is read FOR — rendered at the
     same weight and the same colour as the other three, while `bars.rs` was
     rendering the SAME STORE server-side with `td.close` bold and tinted by
     direction. Two surfaces over one file, disagreeing about which number
     matters.
     --------------------------------------------------------------------- */
  .bclose {
    font-weight: var(--w-semi);
    color: var(--ink-hi);
  }
  .brow[data-dir='up'] .bclose {
    color: var(--up);
  }
  .brow[data-dir='down'] .bclose {
    color: var(--down);
  }
  /* FLAT KEEPS THE NEUTRAL INK AND THAT IS THE POINT. A bar that closed where
     it opened is not a small rise; tinting it either way would invent a
     direction the data does not have. */

  /* THE RANGE SPINE. A left-edge tick whose height is this bar's high-low
     against the widest at the same rung, so a doji reads short and a wide bar
     reads tall and "where did anything actually happen" is answerable before a
     single figure is read.
     ON THE ROW, NOT IN A COLUMN, because it describes the whole row and a
     column of its own would cost horizontal space in a grid that already has
     twenty-five.

     `--rng` IS A FRACTION IN 0..1, NOT A PERCENTAGE. `rngMag` returns
     `max(MIN_SPINE, sqrt(range / widest))` — a square root because the eye
     compares lengths by area and a linear scale put every row on the floor, and
     a floor of 0.08 because a flat minute is a real state and a row with no
     mark at all reads as a rendering fault. Multiplying by `1%` drew every
     spine at half a pixel; the unit is `100%`. */
  /* ---------------------------------------------------------------------
     THE SPINE AND THE CHANGE BAR GROW INTO PLACE RATHER THAN APPEARING.

     Both are driven by a pixel value computed per row — `--spine` from the
     bar's range, `--magpx` from its move — and both were painted at their final
     size the instant the row existed. On a page change fifty of them appeared
     fully formed, which reads as a static picture being swapped rather than as
     measurements arriving.

     Animating WIDTH and HEIGHT here rather than transform, because these are
     one to three pixels wide: a transform-scaled 2px bar is a blurred 2px bar,
     and the crispness is the whole reason they read as measurements. Fifty rows
     of a 2px box is nothing to lay out — this is not the case the
     transform-only rule is written for.

     THEY INHERIT THE ROW'S OWN STAGGER. `--in-delay` already spaces the rows
     top-down; the bars ride the same clock, so a row and its marks arrive
     together instead of the marks chasing the text. */
  @media (prefers-reduced-motion: no-preference) {
    .brow > td:first-child::before {
      animation: cb-spine 320ms var(--ease-out) both;
      animation-delay: var(--in-delay, 0ms);
    }
    .bpc::after {
      animation: cb-mag 320ms var(--ease-out) both;
      animation-delay: var(--in-delay, 0ms);
    }
  }
  @keyframes cb-spine {
    from {
      height: 0;
      opacity: 0;
    }
  }
  @keyframes cb-mag {
    from {
      width: 0;
      opacity: 0;
    }
  }

  /* THE ENTRANCE READS TOP-DOWN, AND IS CAPPED AT 160ms.
     Fifty rows arriving in the same instant is a flash, not a transition: it
     tells a reader something changed and nothing about what. An 8ms step lets
     the eye follow the page filling from the top, which is the direction it
     reads in anyway.
     CAPPED, because the stagger is a cue and not a queue. Uncapped, row 50
     would start 400ms after row 1 and the last rows would still be arriving
     after the reader had begun reading the first — the animation would become
     something to WAIT for. 160ms is under the ~200ms at which a delay stops
     reading as motion and starts reading as latency. */
  .brow.row-in {
    animation-delay: var(--in-delay, 0ms);
  }
  /* ---------------------------------------------------------------------
     THE DIRECTION OF THE ENTRANCE, SET BY WHAT CAUSED IT — see `barMove`.

     Only `animation-name` is overridden. The duration, the easing, the
     `backwards` fill and the per-row delay all stay where they were, so these
     four cases cannot drift apart in timing and a change to the entrance's
     pace is still made in one place.

     INSIDE THE REDUCED-MOTION GUARD, and that is not a formality here: this is
     the one entrance on the page that MOVES the row rather than fading it, so
     it is precisely what someone who asked for stillness is asking to be
     spared. With the guard unmatched, theme.css's `.row-in` does not apply
     either and the rows simply appear.

     10px AND NOT 4. The theme's default is a 4px settle, which is right for a
     row appearing in place and too small to read as a direction — at 4px the
     forward and backward cases are indistinguishable, which would make this
     whole distinction decorative. 10px is still under the row height, so
     nothing travels far enough to be mistaken for scrolling.

     TRANSFORM ON THE ROW IS SAFE HERE AND IS NOT ON THE CENSUS GRID. `.trow`
     carries a `position: sticky` instrument cell, and a transform on its row
     would make the row the containing block and unpin it mid-animation — that
     is why `.trow.row-in` is an opacity fade and says so. Measured on this
     grid: 250 body cells, ZERO of them sticky; the 25 sticky elements are the
     column headers, which are in `<thead>` and never inside a `.brow`. */
  @media (prefers-reduced-motion: no-preference) {
    .bgrid[data-move='fwd'] .brow.row-in {
      animation-name: brow-in-fwd;
    }
    .bgrid[data-move='back'] .brow.row-in {
      animation-name: brow-in-back;
    }
    .bgrid[data-move='sort'] .brow.row-in {
      animation-name: brow-in-sort;
    }
  }
  /* LATER PAGES ARE FURTHER DOWN THE SET, so their rows come UP from below. */
  @keyframes brow-in-fwd {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes brow-in-back {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  /* A RE-SORT IS NOT TRAVEL. The same rows in a new order, so they settle
     sideways — off the axis the pager moves on, which is what keeps the two
     events distinguishable rather than merely both animated. */
  @keyframes brow-in-sort {
    from {
      opacity: 0;
      transform: translateX(-9px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  /* A REPEATED DAY STAYS LEGIBLE AND STOPS COMPETING.
     Two thirds of the ink, not none of it: the date is still readable if you
     look at it and no longer the first thing you see on fifty consecutive rows.
     The row that BEGINS a day keeps full strength, so the boundaries are what
     the column now marks — which is the only thing it was ever telling you
     across a page of one-minute bars. */
  .bt.dayrep {
    /* A COLOUR, NOT AN OPACITY, AND THE RULE ABOVE EXPLAINS WHY IT MATTERS.
       `opacity: 0.42` stood here. It multiplied with the 0.45 alpha the
       `data-dir` rule was putting on this cell's `color`, and the repeated date
       composited to 1.33 against a 4.5 floor — the comment above this block
       promises "DIMMED, NOT BLANKED ... only its weight in the eye changes",
       and at 1.33 it was blanked in everything but the DOM.

       `--faint` is a token that was chosen to be legible on these panels:
       measured 5.47 on the plain row and 4.87 on the striped one, against
       9.65 for a date at full strength. So it still reads as clearly quieter —
       which is the whole design — and it is still readable, which the opacity
       version was not. Dimming with a colour also cannot multiply with another
       alpha further up, which is the failure mode that produced 1.33 out of two
       individually reasonable numbers. */
    color: var(--faint);
    font-weight: var(--w-reg);
  }
  /* AND THE BOUNDARY GETS A LINE ACROSS THE WHOLE ROW. A day change is the one
     structural break in a minute series, and a reader scanning for "where does
     the 20th start" was reading fifty identical strings to find it.
     ON THE ROW AND NOT ON THE DATE CELL. A first attempt put the rule on
     `td.bt:not(.dayrep)`, which draws a 116px stub under one column instead of
     a break across the grid — a line that stops in the middle reads as a
     rendering fault, not a boundary. `dayfirst` is on the `<tr>`, and every
     cell in it carries the top edge. */
  .brow.dayfirst > td {
    box-shadow: inset 0 1px 0 0 var(--line);
  }

  /* THE ROW A JUMP LANDED ON — the same tint-and-rail pair the grid already
     uses to mark a row, so a reader who has seen one recognises the other.
     IT DOES NOT SCROLL AND DOES NOT TAKE FOCUS. The page already moved to hold
     this row; moving the viewport as well would push the neighbours off, and
     the neighbours are the whole reason this lands rather than filters. */
  .brow.landed > td {
    background: var(--acc-soft);
  }
  .brow.landed > td:first-child {
    box-shadow: inset 2px 0 0 0 var(--acc);
  }

  /* ON THE FIRST CELL, AND IN PIXELS, AND BOTH ARE FORCED.
     A `<tr>` is not a containing block an absolutely-positioned child can
     resolve a PERCENTAGE height against — its own height is content-driven, so
     `height: calc(var(--rng) * 100%)` computed to 0px on every row and the
     spine was invisible twice over. The first `<td>` takes `position: relative`
     cleanly, and `--rng` inherits down to it from the row.
     THE PIXELS ARE MULTIPLIED IN THE MARKUP, NOT IN `calc()`. `--rng` is a
     unitless fraction and `calc(var(--rng) * 24px)` measured 0px on every row
     even though the same rule's COLOURS applied — so the pseudo-element existed
     and only its height did not resolve. `--spine` carries the finished pixel
     value instead, computed beside the fraction that produces it, which removes
     the question rather than answering it. 24px against a 40px row leaves the
     tallest spine clear of the cell's own padding. */
  .brow > td:first-child {
    position: relative;
  }
  .brow > td:first-child::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    /* 3px AND 0.9, UP FROM 2px AND 0.55. This mark is the bar's own HIGH-LOW
       RANGE — the one thing in the row that is a picture rather than a number —
       and at two pixels of `--dim` at just over half opacity it was present in
       the DOM and absent to the eye. Measured on the running page: an 11px
       spine on a moving bar and a 2px one on a flat bar, both correct, neither
       legible against the row's own background at that weight.
       A quantity drawn too faintly to compare is not a visualisation, it is a
       decoration of one, and this column's whole job is to let a reader see
       which minutes had range without reading two price columns and
       subtracting. */
    width: 3px;
    height: var(--spine, 0px);
    transform: translateY(-50%);
    /* NO `background` HERE. The second declaration of this pseudo-element sets
       `currentColor`, and the `data-dir` rules colour the cell — so writing
       `var(--dim)` here was a flat grey that the later rule overrode on every
       row anyway. Removing it means one place decides the colour. */
    opacity: 0.9;
    border-radius: 2px;
    pointer-events: none;
  }

  .brow[data-dir='up'] > td:first-child::before {
    background: var(--up);
  }
  .brow[data-dir='down'] > td:first-child::before {
    background: var(--down);
  }
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
  .tbl {
    flex: 1;
    flex: 1;
    min-height: 220px;
    display: flex;
    flex-direction: column;
    position: relative;
    border: 1px solid var(--line);
    border-radius: var(--r4);
    overflow: hidden;
    background: var(--panel);
    box-shadow: var(--e2);
  }
  .tbl-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .tbl-scroll:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
  .tbl-scroll {
    --seam: transparent;
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
  .sortbtn:hover {
    color: var(--ink);
  }
  .sortbtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
    border-radius: var(--r1);
  }
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
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
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
  .unk {
    color: var(--faint);
    cursor: help;
    border-bottom: 1px dotted currentColor;
    padding-bottom: 1px;
  }

  .skelrow {
    position: static;
    align-items: center;
    background: none;
    cursor: default;
  }
  @media (prefers-reduced-motion: no-preference) {
    /* Also in DbCoverageGrid.svelte, where its reasons are written. */
    .sortbtn{
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        color var(--d-hover) var(--ease-out),
        opacity var(--d-hover) var(--ease-out),
        box-shadow var(--d-state) var(--ease-out);
    }
  }

  /* ====================================================================
     THE VIEW SWITCH, THE BAR GRID AND THE PAGER
     ==================================================================== */

  /* WHICH GRID IS SHOWING, AND THE SENTENCE THAT SAYS SO, ON ONE ROW. The
     tab and the sentence are one derived value read twice, so a pressed tab
     with a contradicting caption is not a state this page can reach.

     `.viewbar`, `.views` AND `.vsum` ARE GONE WITH THE VIEW SWITCH, and two
     comments had eaten them. `126152af` deleted each selector line together
     with the closing terminator of the comment written above it — the same
     contiguous cut counted at THE MONTH WINDOW'S OWN CONTROL above — so from
     that commit until now everything between here and THE BAR GRID was one
     comment body: five rule bodies that no stylesheet emitted. The terminator
     cannot be shown to you inside this sentence, because a CSS comment has no
     escape and the star-slash pair ends the comment wherever it appears.

     THE BODIES ARE DELETED RATHER THAN GIVEN THEIR SELECTORS BACK, on the
     measurement `cb5a24d3` settled this question with: not one of these names
     is in this file any more. `viewbar`, `views`, `vtab` and `vsum` occur
     ZERO times outside comment prose — no element, no `class:` directive, no
     string a class name is built from — so restoring them would put back
     rules that match nothing and have matched nothing since the switch was
     removed.

     `.pgsize select` WENT THE SAME WAY AND FOR A SHARPER REASON. There is no
     `<select>` left in this file at all; the rows-per-page control is a
     `Picker`, as the markup note at THE LAST OS-DRAWN MENU records. That one
     is a DESCENDANT selector, so un-swallowing it would not merely restore
     dead CSS — the compiler reports it, and Gate W4 goes from zero unused
     selectors to one against a ceiling of zero.

     AND THE FOCUS RULE IS NOT A LOSS, WHICH IS THE HALF WORTH SAYING, because
     the other half of its selector list DID match: `.pgjump input` is the
     live "Go to page" number field. Its ring is drawn regardless. `theme.css`
     sets a global `:focus-visible` to `2px solid var(--focus)` at `2px` of
     offset — the same two declarations this rule restated, under a heading
     that says every interactive element gets the ring without a page opting
     in. No keyboard behaviour changes here. A duplicate stops claiming to be
     the source of a ring it never drew. */

  /* ---- THE BAR GRID. A real table: paged, not windowed, so the browser's
     own column algorithm can do the work. */
  /* THE BOX SCROLLS AGAIN, AND THIS TIME IT IS THE DECISION RATHER THAN THE
     ACCIDENT.
     ---------------------------------------------------------------------
     It scrolled before because nobody had chosen; the page could not scroll at
     all, so the rows had to. That produced two scrollers and the pointer
     deciding which one moved.
     The operator chose the other arrangement, and it is the better one for this
     page: a box of fixed size that holds a useful number of rows, with the
     pager ALWAYS in reach beneath it rather than at the end of two thousand
     pixels of scrolling. You turn pages far more often than you read to the
     bottom of one.
     Both classes are set here — `class="tbl-scroll bscroll"` — because setting
     one and leaving the other is how the previous change landed on half the
     element and the sticky header rode off to -857px.
     --------------------------------------------------------------------- */
  .bscroll {
    overflow: auto;
  }
  .bgrid {
    border-collapse: separate;
    border-spacing: 0;
    width: 100%;
    table-layout: fixed;
    /* 14px, UP FROM 12.5. The body sets 16px and this grid was rendering the
       only numbers on the page two and a half pixels under it, in a mono face
       whose figures are already narrower than the sans around them. Measured
       before changing anything: header 11.5px, cells 12.5px, row 32px. */
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    /* Figures line up in a column whatever their digits -- a 1 takes the width
       of a 9. Already implied by the mono face; stated so a future face change
       cannot quietly break the column. */
    font-feature-settings: 'tnum' 1;
  }

  /* ------------------------------------------------------------------
     THE GROUP BAND, AND THE TWO SEPARATORS THAT CARRY IT DOWN THE GRID.

     22px above the column headings. The band is the only place on this grid
     where a colour is spent on saying WHAT KIND of number a column holds
     rather than on a value, which is why it is the accent and everything under
     it stays ink and dim.

     BOTH HEAD ROWS ARE STICKY AND THEY DO NOT OVERLAP: the band pins at 0 and
     the headings at 22px, which is the band's own height. Writing the heading
     offset as anything else lets one row slide under the other on scroll, and
     the seam is invisible until a row is halfway through it.
     ------------------------------------------------------------------ */
  .bgrid thead tr.gband > th {
    top: 0;
    height: 22px;
    /* FLAT, NOT THE HEADING ROW'S GRADIENT. Two stacked gradients read as two
       competing surfaces; one flat band under one shaded heading reads as a
       label attached to it. */
    background: var(--panel-2);
    border-bottom: 1px solid var(--line-soft);
    box-shadow: none;
    text-transform: none;
    letter-spacing: normal;
    vertical-align: middle;
  }
  .bgrid thead tr:not(.gband) > th {
    top: 22px;
  }
  .glab {
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--acc);
  }

  /* THE SEPARATOR RUNS THE FULL HEIGHT OF THE GRID, NOT JUST THE BAND.
     A label with nothing under it groups the headings and leaves the figures
     as undifferentiated as before; the point is to chunk the ROWS. One hairline
     on the first column of each group does that at any row count and costs no
     height.

     BY POSITION, BECAUSE THE BODY CELLS ARE HAND-AUTHORED — the same reason
     the hide rules above are positional, and `:nth-child` counts hidden
     siblings, so 4/8/13/20 stay put whatever is displayed. 4 = Open starts
     Price, 8 = Volume starts Flow, 13 = Expiry starts Contract, 20 = IV starts
     Greeks. Column 1 needs none: nothing precedes it.

     WHAT THIS DOES NOT COVER, stated rather than left to be found: if a
     group's FIRST column is hidden while a later one is not, the separator
     goes with it. That is reachable only for Contract — expiry and days-to-exp
     hidden by `.nofuture` while type and strike remain — so that one case has
     its own rule below. An option store with no expiry is the only shape that
     reaches it. */
  .bgrid > thead > tr > th:nth-child(4),
  .bgrid > thead > tr > th:nth-child(8),
  .bgrid > thead > tr > th:nth-child(13),
  .bgrid > thead > tr > th:nth-child(20),
  .bgrid > tbody > tr > td:nth-child(4),
  .bgrid > tbody > tr > td:nth-child(8),
  .bgrid > tbody > tr > td:nth-child(13),
  .bgrid > tbody > tr > td:nth-child(20) {
    border-left: 1px solid var(--line);
  }
  .bgrid.nofuture > thead > tr > th:nth-child(15),
  .bgrid.nofuture > tbody > tr > td:nth-child(15) {
    border-left: 1px solid var(--line);
  }
  .bgrid thead th {
    position: sticky;
    top: 0;
    z-index: 3;
    height: 30px;
    padding: 0 var(--s4);
    text-align: left;
    background: linear-gradient(180deg, var(--panel-2), var(--bg-2));
    /* THE ACCENT IN THE HAIRLINE, because this line separates the two halves of
       the grid — the names above it and the measurements below — and drawing it
       in `--line` made it the same weight as every other border on the page.
       Mixed rather than pure: at full strength it becomes a rule the eye lands
       on first, and a column header's job is to be found when looked for, not
       to compete with the figures under it. */
    border-bottom: 1px solid color-mix(in srgb, var(--acc) 34%, var(--line));
    /* THE ONE MEASURED GAP IN THIS GRID, and it is depth rather than type. The
       header is already opaque and already sticky -- measured `rgb(26,32,48)`
       -- so a row scrolling under it is correctly clipped and nothing bleeds
       through. What it had no cue for was that the clipping is a LAYER:
       `box-shadow` measured `none`, so the join between the pinned header and a
       half-scrolled row read as a cut through the row rather than as the header
       sitting over it. One hairline says which is in front. */
    box-shadow: 0 6px 12px -8px rgb(0 0 0 / 0.55);
    /* THE HEADER IS MONO BECAUSE THE COLUMN UNDER IT IS.
     *
     * The alignment was already exact -- both the header text and the cell text
     * ended on the same pixel, measured. It LOOKED wrong because the header was
     * a 11.5px sans and the figures a 12.5px mono, and two right-aligned runs
     * in different faces at different sizes do not read as one edge. Matching
     * the family and closing the size gap fixes the appearance, which is the
     * thing that was actually broken. */
    font-family: var(--mono);
    font-size: 12px;
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--dim);
    white-space: nowrap;
  }
  .bgrid thead th.numh {
    text-align: right;
    /* The header's figures sit on the same grid as the column's. */
    font-variant-numeric: tabular-nums;
  }

  /* ------------------------------------------------------------------
     THE HEADING LANDS ON THE SAME PIXEL AS THE DIGITS UNDER IT.

     Every heading in this grid is a `<button class="sortbtn">`, and that button
     is `display: inline-flex; width: 100%`. `text-align: right` on the `th`
     therefore reaches the BUTTON and stops: the label is a flex item, and a
     flex item is placed by `justify-content`, which defaulted to the start. So
     every numeric heading sat at the LEFT of a column whose data is at the
     RIGHT -- measured across the ten drawn columns, gaps of 34px to 92px, on
     every one.

     A rule for exactly this already existed and had never once applied. It was
     written `.th.numh .sortbtn`, which needs a class literally named `th`; the
     census grid above builds its headings as `<div class="th numh">` and gets
     it, while this grid builds real `<th class="numh">` elements and never
     matched. The comment over that rule promises "the header text lands on the
     same pixel as the digits under it" -- a promise the selector could not
     keep, in a file where the promise had been read as done.

     Measured after: gap 0 on all ten.
     ------------------------------------------------------------------ */
  .bgrid thead th.numh .sortbtn {
    justify-content: flex-end;
  }
  .bgrid thead th:not(.numh) .sortbtn {
    justify-content: flex-start;
  }

  /* AND THE CARET MOVES TO THE OTHER SIDE OF THE LABEL.
   *
   * The button is `<span>Open</span><span class="caret">`: a 7px caret plus the
   * flex `gap`. Reserved whether or not the column is the sorted one, which is
   * what stops the heading jumping sideways when you click it -- so it cannot
   * simply be removed. Left after the label it holds the heading 11px short of
   * the column's right edge while the digits sit flush against it, which is a
   * gap the eye reads on all eight numeric columns at once.
   *
   * `order: -1` puts it before the label, so the label itself ends on the
   * right edge -- the same pixel the digits end on. This is what the older
   * `.th.numh .sortbtn` comment described and what that selector never
   * delivered. */
  .bgrid thead th.numh .caret {
    order: -1;
  }

  /* ------------------------------------------------------------------
     THE FOURTEEN COLUMNS THAT SAY NOTHING ARE NOT DRAWN.

     Ten have NO SOURCE on this wire -- pre-market %, moneyness, intrinsic,
     extrinsic and the six greeks -- and four describe a CONTRACT: expiry,
     days-to-expiry, type and strike. A store of indices and equities has
     neither kind, so fourteen of twenty-four columns were rendered on every
     row of every page purely to read "NO SOURCE" or "-". At fifty rows a page
     that is seven hundred cells of nothing, and close and volume were pushed
     off the side of the screen behind them.

     HIDDEN BY POSITION, NOT BY FILTERING THE COLUMN LIST. The body cells are
     hand-authored in column order rather than looped, so filtering the header
     and the colgroup alone would have shifted every value one column left --
     a far worse bug than the one being fixed, and a silent one. `nth-child`
     over the fixed order keeps header, colgroup and body in lockstep by
     construction.

     `CLAUDE.md` §4 -- degrade loudly, name the reason -- is why they were
     drawn in the first place. §4 is about a FAILURE being concealed, and
     neither of these is one: a feed that never sends a greek has not failed,
     and a spot index has no expiry to report. The reason is still named once,
     in the line under the table, instead of seven hundred times inside it.
     ------------------------------------------------------------------ */
  /* 9 AND 17+, BOTH +1 FROM WHAT THEY WERE — `Time` was inserted at position 2
     and every index after it moved. The comment above is exactly right that
     `nth-child` keeps header, colgroup and body in lockstep; what it cannot do
     is keep THESE NUMBERS in step with the column list itself, which is two
     thousand lines away and has no link to them. Three separate rules were
     silently pointing one column short until the table was rendered and read. */
  /* 20 ONWARD — IV AND THE FIVE GREEKS — WERE HIDDEN HERE FOR EVERY
     SELECTION while they had no source. Since D-2830 they are solved per bar
     and belong to an OPTION, so `.nooption` below hides them with the other
     option columns and this rule keeps only pre-market % (9). */
  .bgrid.lean > colgroup > col:nth-child(9),
  .bgrid.lean > thead > tr > th:nth-child(9),
  .bgrid.lean > tbody > tr > td:nth-child(9) {
    display: none;
  }

  /* TIME (2) IS ABSENT ON A DAY RUNG. Every 1day bar opens at the same instant,
     so the column would repeat one value down every row and earn none of its
     width. Hidden rather than blanked: a column of identical stamps is not
     information, and a column of dashes is worse, because a dash means "no
     source" everywhere else on this table. */
  .bgrid.lean.noday > colgroup > col:nth-child(2),
  .bgrid.lean.noday > thead > tr > th:nth-child(2),
  .bgrid.lean.noday > tbody > tr > td:nth-child(2) {
    display: none;
  }

  /* EXPIRY (13) AND DAYS-TO-EXPIRY (14) belong to anything with a contract, so
     they appear for a future and for an option and for nothing else.

     THESE INDICES MOVED +1 WHEN `Time` WAS INSERTED AT POSITION 2, and that is
     the hazard of hiding by position: the numbers here are a silent duplicate of
     the column ORDER two thousand lines up, and nothing checks them against it.
     Adding a column at the front shifted every rule below and broke both
     contract hidings with no error anywhere — caught by rendering the table, not
     by a gate. */
  /* OPEN INTEREST (11) AND OI CHG % (12) JOIN THEM, AND SHOULD ALWAYS HAVE.
     Open interest is the count of contracts outstanding. A spot index has no
     contracts, so the number is not "zero interest" — the QUANTITY DOES NOT
     EXIST. Both columns were drawing `0` and `—` on every spot row, which is
     the page stating a measurement of something that cannot be measured, and
     they are two of the eleven columns a reader scans on the commonest view
     this page has.
     MEASURED, which is what made it worth doing: of the eleven columns visible
     on NIFTY 1min, six carry information. These two are a third of the waste,
     and the only part of it that is structural rather than a property of the
     instrument — an equity has real volume, an index does not, and no rule
     here can know that in advance. A contract's OI is real; spot's is not,
     always, for every feed. */
  .bgrid.lean.nofuture > colgroup > col:nth-child(11),
  .bgrid.lean.nofuture > colgroup > col:nth-child(12),
  .bgrid.lean.nofuture > colgroup > col:nth-child(13),
  .bgrid.lean.nofuture > colgroup > col:nth-child(14),
  .bgrid.lean.nofuture > thead > tr > th:nth-child(11),
  .bgrid.lean.nofuture > thead > tr > th:nth-child(12),
  .bgrid.lean.nofuture > thead > tr > th:nth-child(13),
  .bgrid.lean.nofuture > thead > tr > th:nth-child(14),
  .bgrid.lean.nofuture > tbody > tr > td:nth-child(11),
  .bgrid.lean.nofuture > tbody > tr > td:nth-child(12),
  .bgrid.lean.nofuture > tbody > tr > td:nth-child(13),
  .bgrid.lean.nofuture > tbody > tr > td:nth-child(14) {
    display: none;
  }

  /* THE RUNG COLUMN (3) GOES WHEN ONE RUNG IS CHOSEN. `TIMEFRAME` in the strip
     above already says `1min`, and the column then prints that same word once
     per row — eleven times on a full screen, in a fixed-width cell, next to
     the two columns that actually vary. When the rung is `Everything` it is
     the only thing distinguishing a 1min row from a 1day one and it stays.
     GATED ON THE CONTROL, NOT ON THE DATA. `timeframe !== ''` is a fact about
     the query; testing whether the loaded rows happen to share a rung would
     make the column appear and vanish as pages turn. */
  .bgrid.lean.norung > colgroup > col:nth-child(3),
  .bgrid.lean.norung > thead > tr > th:nth-child(3),
  .bgrid.lean.norung > tbody > tr > td:nth-child(3) {
    display: none;
  }

  /* TYPE (15), STRIKE (16) and — since D-2807 sourced them per bar —
     MONEYNESS (17), INTRINSIC (18) and EXTRINSIC (19) belong to an OPTION
     only. A future has an expiry and no strike, so a futures selection keeps
     13 and 14 and loses 15 onward — IV AND THE GREEKS (20 to 25) included,
     since D-2830 solved them per bar. */
  .bgrid.lean.nooption > colgroup > col:nth-child(n + 15),
  .bgrid.lean.nooption > thead > tr > th:nth-child(n + 15),
  .bgrid.lean.nooption > tbody > tr > td:nth-child(n + 15) {
    display: none;
  }

  /* A COLUMN WITH NO SOURCE IS DIMMER AND ITS BUTTON IS REFUSED, not absent.
     `not-allowed` plus the reason on `title` is the pair `CLAUDE.md` §4 asks
     for: the control is visible, it does not work, and it says why. */
  .bgrid thead th.deadh {
    color: var(--faint);
    opacity: 0.62;
  }
  .bgrid thead th .sortbtn:disabled {
    cursor: not-allowed;
  }
  .brow > td {
    padding: 0 var(--s4);
    height: var(--dbrow);
    border-bottom: 1px solid var(--line-soft);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    background: var(--bg-2);
  }
  .brow.odd > td {
    background: var(--panel-2);
  }
  /* OPAQUE, AND MIXED OUT OF THE ACCENT the same way the census grid's
     cursor row is. `--d-hover` was written here first and it is a DURATION
     token, 110ms — a background that resolves to a time is dropped at
     computed-value time and the row simply never lit. */
  .brow:hover > td {
    background: color-mix(in srgb, var(--acc) 7%, var(--bg-2));
  }
  /* THE HOVER LANDS INSTEAD OF SNAPPING. Background ONLY -- never `all`, which
     would animate the row's geometry and reflow a 50-row grid on a mouse move.
     Inside `no-preference` because an operator who asked for stillness gets a
     grid that highlights instantly and never moves. */
  @media (prefers-reduced-motion: no-preference) {
    .brow > td {
      transition: background-color var(--d-hover) var(--ease-out);
    }
  }
  .bt {
    font-family: var(--mono);
    color: var(--ink-2);
  }
  .bn {
    text-align: right;
    font-family: var(--mono);
    color: var(--ink);
  }
  /* THE UNSOURCED CELLS ARE LEGIBLE AND UNMISTAKABLE. They read "no source",
     never a dash that could pass for a lazy zero, and the reason is on the
     title and the aria-label both. */
  .na {
    text-align: right;
  }
  .na .unk {
    font-family: var(--sans);
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    border-bottom-style: dotted;
  }

  /* A PSEUDO-ELEMENT, NOT A GRADIENT, AND THE GRADIENT NEVER ONCE DREW.
     ---------------------------------------------------------------------
     This was `background-image: linear-gradient(color-mix(in srgb,
     currentColor 20%, transparent) …)` sized by `background-size: var(--mag)
     3px`. MEASURED on the running page: of fifty rows, twenty-nine carried a
     real `--mag` between 1% and 20%, and every one of them computed
     `background-image: none; background-size: auto`. Not a zero-width bar — no
     bar at all.

     It is not the value and it is not the selector. Both were tested in
     isolation on the same element: setting `background-size: var(--mag) 3px`
     inline resolved to `20% 3px`, setting the gradient inline resolved to a
     real gradient, `@property --mag` is registered in `theme.css` and reaches
     the built sheet, the rule `.bpc.svelte-d8w2tr` is in the loaded stylesheet
     carrying the declaration, and the cell matches that selector. Every part
     works alone and the rule still does not apply — the remaining suspect being
     `color-mix()` over `currentcolor` inside a gradient, in a stylesheet rather
     than inline.

     So the encoding stopped depending on that. A `::before` with an explicit
     WIDTH is the pattern the direction spine on this same grid already proves,
     and a percentage width resolves against a positioned ancestor without
     needing the definite HEIGHT that caught the spine out. It draws.

     The design is unchanged: a 3px band on the cell's floor, the axis at the
     centre, growing RIGHT for up and LEFT for down — direction carried by
     POSITION, so the column reads as a distribution down fifty rows and
     survives greyscale and a red-green dichromat. */
  .bpc {
    --mag: 0%;
    position: relative;
  }
  .bpc::after {
    content: '';
    position: absolute;
    bottom: 2px;
    /* 5px AT 0.5, UP FROM 3px AT 0.28. Same finding as the spine above: the bar
       behind the percentage is how a reader compares this move to the largest
       move in the selection WITHOUT reading every figure, and at three pixels
       of `currentColor` at twenty-eight percent it did not survive the row
       stripe behind it. `currentColor` already carries the direction, so
       raising the weight strengthens the comparison and adds no new hue —
       which is what keeps this readable in greyscale and to a dichromat, the
       property the note above this rule exists to protect. */
    height: 5px;
    width: var(--magpx, 0px);
    border-radius: 2px;
    background: currentColor;
    opacity: 0.5;
    pointer-events: none;
  }
  /* THE AXIS IS THE CELL'S CENTRE. `left: 50%` grows right; `right: 50%` grows
     left. Neither needs to know the bar's own width, which is what the old
     `calc(50% - var(--mag))` background-position had to compute. */
  .bpc[data-dir='up']::after {
    left: 50%;
  }
  .bpc[data-dir='down']::after {
    right: 50%;
  }
  /* FLAT DRAWS NOTHING. A bar that closed where it opened has no direction and
     no magnitude, and a tick at the axis would read as a small move. */
  .bpc[data-dir='flat']::after {
    display: none;
  }

  /* VOLUME, THE SAME IDEA AND DELIBERATELY NEUTRAL. Length reads as size; it
     must never read as direction, so this one has no `data-dir` and takes
     `--dim` rather than the up/down pair. Grows from the right, under the
     figure, so the number stays what the eye lands on. */
  .bvol {
    --vmag: 0%;
    position: relative;
  }
  .bvol::after {
    content: '';
    position: absolute;
    right: 0;
    bottom: 3px;
    height: 3px;
    width: var(--vmagpx, 0px);
    border-radius: 1px;
    background: var(--dim);
    opacity: 0.34;
    pointer-events: none;
  }

  /* ---------------------------------------------------------------------
     THE DIRECTION SPINE — one tick per row, down the left edge.

     Height is the bar's own RANGE against the widest range at its rung, so the
     left margin becomes a profile of where the session actually moved. Colour
     is close against open. It lands inside the 8px padding the first cell
     already has, so it displaces nothing, and `overflow: hidden` on the cell
     clips it to the row.
     --------------------------------------------------------------------- */
  .brow > td:first-child {
    position: relative;
  }
  /* THIS IS THE SECOND DECLARATION OF THIS PSEUDO-ELEMENT AND IT USED TO
     APPLY THE RANGE A SECOND TIME.
     ---------------------------------------------------------------------
     Two rules for one selector do not simply shadow each other — they MERGE,
     property by property, and only the properties they share resolve by order.
     So the earlier rule's `height: var(--spine)` survived here, and this rule
     then added `transform: scaleY(var(--rng))` on top of it. `--spine` is
     already `--rng * 24px`, computed in the markup: the range was applied
     TWICE. Measured on a bar with `--rng: 0.45` — 24 x 0.45 x 0.45 = 4.9px
     where the encoding says 10.8. Every bar was squashed toward zero, worst
     for the small ones, which is exactly backwards for a mark whose job is
     making small differences visible. Neither rule is wrong read alone.

     The height carries the range and the transform is gone. What stays is
     `background: currentColor`, which is this rule's real contribution: the
     `data-dir` rules below colour the CELL, and the mark inherits it, so
     direction and range are encoded by two different channels rather than one
     doing both badly. `inset` goes with the transform — the earlier rule
     centres a bar of an explicit height, and stretching it edge to edge here
     was only ever scaffolding for the scale. */
  .brow > td:first-child::before {
    content: '';
    /* `var(--dirc)` — set by the three `data-dir` rules below, which no longer
       write `color` for exactly this purpose. `--dim` is the fallback for a row
       with no direction at all rather than a colour anything normally uses. */
    background: var(--dirc, var(--dim));
  }
  /* THE SHARED OPENING OF EVERY PRICE ON THE PAGE. Weight and colour only —
     the same size, the same family and the same tabular figures as the rest of
     the cell, so the digits stay on their grid and the number keeps its shape.
     A smaller prefix would break the column alignment that makes the bright
     tails scannable, which is the entire point. `--faint` rather than a lower
     opacity so it composites the same on the striped rows as on the plain
     ones. */
  .pfx {
    color: var(--faint);
    font-weight: var(--w-reg);
  }
  /* THE CLOSE'S PREFIX IS NOT DIMMED HARDER, AND IT WAS, AND IT FAILED AA.
     `opacity: 0.75` stood here to widen the gap between the run that repeats
     and the digits that moved. Measured after compositing: 3.62 against the
     plain row and 3.35 against the striped one, where the floor is 4.5.

     IT PASSED THE FIRST CHECK, which is the part worth recording. A contrast
     reader that takes `color` and the background and ignores `opacity` reports
     5.47 for text that renders at 3.62 — and `opacity` is the most common way
     to dim anything in CSS. Effective alpha is the PRODUCT of every ancestor's
     opacity down to the first opaque background and has to be composited before
     luminance, or the instrument passes exactly the cases it exists to catch.

     The close is already the figure: the `nth-child` rule above lowers O/H/L to
     `--ink-2` and leaves the close at full weight, so the emphasis is there
     without spending it out of the one budget on this page that is not a matter
     of taste. */

  /* A DIRECTION YOU CAN SEE WITHOUT READING. The spine states the RANGE and the
     percentage states the MOVE; neither says at a glance which way the bar
     went, and colour on a 3px mark is the least visible channel available. This
     is a wash across the first third of the row, mixed out of the same `--up` /
     `--down` tokens, at a strength that reads as a tint rather than a
     highlight — a full-row fill would fight the alternating stripe and the
     cursor row, both of which mean something else here. */
  .brow[data-dir='up'] > td:first-child,
  .brow[data-dir='down'] > td:first-child {
    /* `--dirw`, NOT `--dirc` — see the two-strengths note where both are set.
       This is the wash; `--dirc` is the 3px spine, and they want opposite
       weights out of the same hue. */
    background-image: linear-gradient(90deg, var(--dirw), transparent 70%);
  }
  /* ---------------------------------------------------------------------
     `--dirc` AND NOT `color`, AND THE DIFFERENCE WAS AN ACCESSIBILITY BUG.

     These three rules exist so the range mark can say `background:
     var(--dirc)` and pick up its row's direction. They used to say `color`,
     which works for the mark — and the CELL'S TEXT INHERITS IT. The date in
     that cell was therefore drawn in a 58%-transparent green.

     MEASURED, compositing the alpha: 4.27 against a floor of 4.5, and on the
     rows where `.dayrep` dims a repeated day as well the two alphas multiply —
     0.45 x 0.42 — and the date rendered at 1.33. Not marginal: barely there.

     A custom property carries the direction to the one element that wants it
     and stops at every element that does not. The text goes back to inheriting
     the row's own colour, which measures 9.65. */
  /* TWO STRENGTHS, BECAUSE ONE TOKEN WAS DOING TWO JOBS THAT WANT OPPOSITE
     ONES. `--dirc` at 58% is right for the SPINE — a 3px mark needs weight to
     register, and colour on something that thin is the least visible channel
     available. The same 58% behind a WASH across 82% of the cell is not a
     tint, it is a fill: measured on the running page as
     `color(srgb 0 0.462745 0.305882 / 0.58)` spread over the Date column,
     which is why that column reads as permanently selected and why the
     question asked of it was "why is this highlighted".
     The rule drawing the wash states the requirement in its own words — "at a
     strength that reads as a tint rather than a highlight; a full-row fill
     would fight the alternating stripe and the cursor row, both of which mean
     something else here" — and then borrowed the spine's number and did the
     thing it forbade.
     `--dirw` is that wash at roughly a sixth of the strength. Same hue, same
     token source, same meaning, and it stops competing with the stripe and the
     cursor row over what a background signifies. */
  .brow[data-dir='up'] > td:first-child {
    --dirc: color-mix(in srgb, var(--up) 58%, transparent);
    --dirw: color-mix(in srgb, var(--up) 9%, transparent);
  }
  .brow[data-dir='down'] > td:first-child {
    --dirc: color-mix(in srgb, var(--down) 58%, transparent);
    --dirw: color-mix(in srgb, var(--down) 9%, transparent);
  }
  /* A FLAT BAR GETS NO WASH AT ALL. Its spine keeps a muted mark because the
     RANGE is still a fact, but "the close equalled the open" is not a
     direction and a tint saying it is would be inventing one. */
  .brow[data-dir='flat'] > td:first-child {
    --dirc: color-mix(in srgb, var(--dim) 45%, transparent);
    --dirw: transparent;
  }

  /* O / H / L ARE CONTEXT; CLOSE IS THE FIGURE. Four prices at one weight makes
     a reader parse all four to find the one they came for. */
  .bgrid > tbody > tr > td:nth-child(3),
  .bgrid > tbody > tr > td:nth-child(4),
  .bgrid > tbody > tr > td:nth-child(5) {
    color: var(--ink-2);
  }

  @media (prefers-reduced-motion: no-preference) {
    /* `--mag`, `--vmag` and `--rng` are registered, so these interpolate. The
       rule this replaces named `background-image` and never ran. */
    .bpc,
    .bvol {
      transition:
        --mag 160ms var(--ease-out),
        --vmag 160ms var(--ease-out);
    }
    .brow > td:first-child::before {
      transition: transform 160ms var(--ease-out);
    }
  }
  .bpc[data-dir='up'] {
    color: var(--up);
  }
  .bpc[data-dir='down'] {
    color: var(--down);
  }
  .bpc[data-dir='flat'] {
    color: var(--dim);
  }
  .bstate > td {
    padding: 0;
  }
  .bnone {
    padding: var(--s8) var(--s6);
    text-align: center;
  }
  .bnone h3 {
    font-size: var(--fs-md);
    margin-bottom: var(--s3);
  }
  .bnone p {
    color: var(--dim);
    font-size: var(--fs-sm);
    margin: 0 auto var(--s5);
    max-width: 74ch;
    line-height: 1.6;
    white-space: normal;
  }
  .bnone p.err {
    color: var(--down);
    font-family: var(--mono);
  }
  .bnotes {
    flex: none;
    border-top: 1px solid var(--line);
    background: var(--panel-2);
    padding: var(--s4) var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    max-height: 168px;
    overflow-y: auto;
  }
  .bnote {
    margin: 0;
    font-size: var(--fs-xs);
    line-height: 1.55;
    color: var(--dim);
  }
  .bnote b {
    font-family: var(--mono);
    color: var(--ink-2);
    font-weight: var(--w-semi);
  }
  .bnote.warn {
    color: var(--warn);
  }
  .bnote.err {
    color: var(--down);
  }

  /* ---- THE PAGER ---- */
  .pgbar {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    padding: var(--s4) var(--s5);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel-2);
    /* ---------------------------------------------------------------------
       NOT STICKY, AND THE STICKY VERSION IS WHY THE PAGE SHOOK.

       It was pinned here — `position: sticky; bottom: calc(-1 * var(--s5))` —
       to answer a real complaint: 25 rows in a fixed box put this bar 553px
       below the fold, and pinning it meant the reader never had to travel to
       reach the next page. It worked, and it was the wrong layer to fix it at.

       A STICKY ELEMENT INSIDE A PADDED SCROLLER ALWAYS SNAPS BY THE PADDING.
       `.board` carries 12px of it. Stuck, this bar held at the scrollport's
       edge; released at the end of the scroll, it settled 12px higher.
       MEASURED on a 980px window with a 56px scroll range: the bar sat still
       through 55 pixels of scrolling and then JUMPED TWELVE at the last one.
       Cross that threshold with the wheel and the whole bottom of the page
       shakes, which is exactly what the operator reported and exactly what a
       control the eye uses as a fixed landmark must never do.

       AND IT WAS ALREADY REDUNDANT. `Fit` sizes the grid to what the window can
       show, so the page fits and this bar is on screen without help — the
       problem the pin was hired for no longer exists. What remained was its
       side effect. A workaround kept after its cause is fixed is strictly worse
       than no workaround, because it now only does the harm.

       WHAT HAPPENS ON A WINDOW TOO SHORT FOR `Fit`, said plainly: the table
       reaches its 320px floor, the page overflows by a few dozen pixels, and
       this bar scrolls with everything else. That is a short, honest scroll to
       a bar that does not move relative to the thing above it, and it is the
       better of the two failures. `Compact` recovers ~180px and removes even
       that. */
  }
  .pgof,
  .pgof2 {
    font-size: var(--fs-xs);
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .pgof b {
    color: var(--ink-2);
    font-family: var(--mono);
    font-weight: var(--w-semi);
  }
  .pgsize,
  .pgjump {
    display: inline-flex;
    align-items: center;
    gap: var(--s3);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  /* BIG ENOUGH TO TYPE IN. A 46px box with 11px digits is a control that
     technically accepts a page number; this one is legible at arm's length
     and its own spinner is not the only way to reach page 37. */
  .pgjump input {
    width: 92px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    color: var(--ink);
    font: inherit;
    font-family: var(--mono);
    font-size: 15px;
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    padding: 10px 12px;
    text-align: right;
  }
  .pgjump input:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .pgnav {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    margin-left: auto;
  }
  .pg {
    appearance: none;
    min-width: 34px;
    height: 32px;
    padding: 0 var(--s3);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    background: var(--panel);
    color: var(--dim);
    font: inherit;
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .pg:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--dim);
  }
  .pg:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .pg:disabled {
    opacity: 0.34;
    cursor: not-allowed;
  }
  .pg[aria-current='page'] {
    background: var(--acc);
    border-color: var(--acc);
    color: var(--on-acc);
  }
  .pg.gap {
    border: 0;
    background: none;
    cursor: default;
    color: var(--faint);
    min-width: 18px;
    padding: 0;
  }
</style>
