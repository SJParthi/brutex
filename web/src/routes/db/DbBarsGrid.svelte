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
  import { ROW, HEAD, FIT_SLACK, MIN_USEFUL_ROWS, istDayKey, IST_OFFSET_MS, wireSort, LOCAL_SORT_BUDGET, WINDOW_PAGE_MAX, seriesPlan, windowQuery, rowKey, parseGoto, pageOfIndex, censusBehind, localComparator, paisaText, daysBetween, MIN_SPINE, CHG_FLOOR_BPS, SKELETON, PAGE_SIZES, pageList } from '$lib/db/bars-plan.js';
  import { untrack } from 'svelte';
  import { TF_SECS, txt, tfCmp, fmt } from '$lib/db/notes-labels.js';
  import { BAR_COLS, FUTURE_COLS, OPTION_COLS, BAR_GROUPS, BAR_META, BAR_UNGROUPED, barWhyText } from '$lib/db/notes-bars.js';
  import { dayWindowApplies } from '$lib/db/notes-controls.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { readWindowPage, readAllRows, sortIndexNote } from '$lib/database-pages.js';
  import { dirOf, bpsText } from '$lib/bps.js';
  import { timeLabel, stampLabel, dayLabel, barTimeLabel } from '$lib/dates.js';
  import { refreshStore } from '$lib/store.svelte.js';
  import Picker from '$lib/Picker.svelte';
  import DbChart from './DbChart.svelte';
  import DbExtremes, { extremesState } from './DbExtremes.svelte';
  import DbDayFlags from './DbDayFlags.svelte';
  import DbOptionCells from './DbOptionCells.svelte';
  import DbOptionNotes from './DbOptionNotes.svelte';
  import DbProvenance from './DbProvenance.svelte';
  // WHAT THE FULL-BLOCK SCRUB FOUND IN THE SERIES ON THE PAGE (DB-B74, WP-81).
  import DbScrubNote from './DbScrubNote.svelte';

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
    /** The box height Fit sizes the page by: taken from a SETTLED reading only
        (250 ms after the selection settles or the fold changes, and on every
        window resize), so paging cannot change the page size (DB-N11). */
    let fitBoxH = $state(0);
    /** Whether a settled reading has been taken. In Fit mode the grid reads
        nothing before it, so a load asks for one page size, once (DB-N02). */
    let fitLatched = $state(false);
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
      /* NOT ON EVERY READ ANY MORE (DB-N02, DB-N11). Re-measuring after every
         fetch made the page size follow the chrome: a note line under the
         grid on one page and not the next turned five rows into four between
         two clicks, so page N stopped meaning the same rows, and the first
         load asked for 4, then 3, then 8 rows. The box is measured when the
         selection settles, when the fold changes and when the window is
         resized — the reader's acts — and the Fit size is LATCHED from the
         settled reading below. */
      void db.settled;
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
      /** The settled reading: what Fit sizes the page by, until the next one. */
      const latch = () => {
        measure();
        if (el.clientHeight > 0) {
          fitBoxH = el.clientHeight;
          fitLatched = true;
        }
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
      const afterFold = setTimeout(latch, 250);
      window.addEventListener('resize', latch);
      return () => {
        window.removeEventListener('resize', latch);
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
       which is why the effect above re-measures when the selection settles.

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
      Math.max(1, Math.floor(((fitBoxH || barBoxH) - HEAD - FIT_SLACK) / ROW))
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
      rowsThatFit: { get: () => rowsThatFit },
      fitBoxH: { get: () => fitBoxH },
      fitLatched: { get: () => fitLatched }
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
    /* THE READ BUDGET IS GONE AS A CONTROL, AND STAYS AS A NAME.
     *
     * It capped how many month FILES the grid opened when the grid opened
     * files. The grid now asks the server one question per page (D-3326), and
     * every matched month is in that question, so there is nothing left to
     * cap. `readBudget` is still published, at 0 ("every one"), because the
     * selection's page-reset effect reads it. */
    let readBudget = $state(0);

    /**
     * The matched census rows, and nothing more is decided here: which series,
     * which months and which bounds the request names is `plan` below.
     *
     * `read` and `held` keep their shape for the readers that still count
     * them; nothing is held back.
     */
    const barPlan = $derived.by(() => {
      // Default pickers settle after a census arrives. Until their initial
      // instrument, segment and rung fields settle, planning would read a
      // selection nobody chose.
      if (db.view !== 'bars' || !db.filter || !db.kind || !db.timeframe) {
        return { all: [], read: [], held: 0 };
      }
      const all = db.matched;
      return { all, read: all, held: 0 };
    });

    /**
     * IS THE DAY WINDOW NARROWING ANYTHING? The fields open seeded at the
     * selection's own census span (WP-50, DB-045), so only a value differing
     * from that span narrows.
     */
    const dayWindowNarrows = $derived(dayWindowApplies && db.dayNarrowing);

    /**
     * IS A TIME BOUND NARROWING ANYTHING. Suppressed on a day rung, where every
     * bar opens at the same minute and the Time column is hidden.
     */
    const timeNarrows = $derived(!dayRung && Boolean(db.fromTime || db.toTime));
    /* KEPT FOR ITS READERS: whether the day window narrows, under its old name. */
    const dayNarrows = $derived(dayWindowNarrows);

    /**
     * THE BOUNDS THE SERVER APPLIES, to every month of every series (DB-004,
     * DB-G91, DB-063). They used to reach only the one or two files the page
     * opened, and the pager counted that subset as the answer: "of 6,120 rows"
     * for GDFL NIFTY 1s 09:15–09:20, every one of them from the newest month.
     */
    const bounds = $derived({
      fromDay: dayWindowNarrows ? db.fromDay : '',
      toDay: dayWindowNarrows ? db.toDay : '',
      fromTime: timeNarrows ? db.fromTime : '',
      toTime: timeNarrows ? db.toTime : ''
    });

    /** The request's series, rung and months (D-3326), narrowed by the day window. */
    const plan = $derived(seriesPlan(barPlan.all, bounds.fromDay, bounds.toDay));
    /** The same selection unbounded, for the facts that describe it whole. */
    const planAll = $derived(seriesPlan(barPlan.all));

    /** The row set's identity — series, rung, months and bounds; never the page or the order. */
    const setKey = $derived(
      [feeds.active ?? '', plan.timeframe, plan.from, plan.to, plan.keys.join(','),
        bounds.fromDay, bounds.toDay, bounds.fromTime, bounds.toTime, db.nonce].join('|')
    );

    /* ---------------------------------------------------------------------
       THE BAR GRID'S SORT. `barSortKey` is the GRID's column key; `wireSort`
       translates it for the server (`vol` is `v`), and a column the server
       does not order is ordered here, under a budget (DB-B50).
       --------------------------------------------------------------------- */
    let barSortKey = $state('ts');
    let barDesc = $state(true);
    /** Why the last sort asked for was refused, said beside the grid until the next header press. */
    let sortWhy = $state('');

    /* ---------------------------------------------------------------------
       THE READ. ONE REQUEST A PAGE, for one series or a thousand.
       --------------------------------------------------------------------- */
    /** @type {{ loading: boolean, error: string | null, answer: any, rows: any[], local: boolean }} */
    let barState = $state.raw({ loading: false, error: null, answer: null, rows: [], local: false });
    let barToken = 0;
    /** How long the last page read took, request to rows, in milliseconds. */
    let pageMs = $state(0);
    /** The last full read a local sort made, by row-set identity: page turns of that sort read nothing. */
    let localCache = { key: '', rows: /** @type {any[]} */ ([]), answer: /** @type {any} */ (null) };

    /**
     * One page in an order the server keeps: one request, whatever the number
     * of series, months or bounds (D-3326; DB-002, DB-003, DB-G82, DB-G92,
     * DB-G93, DB-G102, DB-B49). The total is the server's, the files' own
     * count after the bounds — so it is the same in every order (DB-G02,
     * DB-G61) — and no extremes are folded on a page turn (DB-G83).
     *
     * @param {string} feed @param {any} p @param {any} b @param {string} wire
     * @param {boolean} desc @param {number} page @param {number} size @param {AbortSignal} signal
     */
    async function readServerPage(feed, p, b, wire, desc, page, size, signal) {
      const offset = (page - 1) * size;
      const answer = await readWindowPage(windowQuery(feed, p, { ...b, sort: wire, desc, offset, limit: size }), size, signal);
      return { loading: false, error: null, answer, rows: answer.bars, local: false };
    }

    /**
     * One page in an order only this page can compute — a change column, or a
     * contract column — from every kept row, read in time order through the
     * window route, under `LOCAL_SORT_BUDGET` rows (DB-063, DB-N16, DB-A29,
     * DB-B50). The count is asked first, one seek, so a selection past the
     * budget is refused before a single row of it is read.
     *
     * @param {string} feed @param {any} p @param {any} b @param {string} key
     * @param {AbortSignal} signal
     */
    async function readLocalPage(feed, p, b, key, signal) {
      if (localCache.key === key) {
        return { loading: false, error: null, answer: localCache.answer, rows: localCache.rows, local: true };
      }
      const head = await readWindowPage(windowQuery(feed, p, { ...b, sort: 'ts', desc: false, offset: 0, limit: 1 }), 1, signal);
      if (head.total > LOCAL_SORT_BUDGET) {
        return { refused: head.total, answer: head };
      }
      const got = await readAllRows(
        (offset, limit) => windowQuery(feed, p, { ...b, sort: 'ts', desc: false, offset, limit }),
        head.total, LOCAL_SORT_BUDGET, WINDOW_PAGE_MAX, signal
      );
      const answer = { ...head, faults: got.faults.length ? got.faults.join('; ') : null };
      localCache = { key, rows: got.bars, answer };
      return { loading: false, error: null, answer, rows: got.bars, local: true };
    }

    $effect(() => {
      const active = db.view === 'bars';
      const feed = feeds.active;
      const p = plan;
      const b = bounds;
      const key = barSortKey;
      const desc = barDesc;
      const set = setKey;
      const page = Math.max(1, Math.trunc(Number(db.page)) || 1);
      const size = db.pageSize;
      /* NOTHING IS READ BEFORE THE FIT HAS SETTLED AND BEEN APPLIED (DB-N02).
         The box measured 4, then 3, then 8 rows on one load, and each was a
         request. In Fit mode the page waits for the settled measurement AND
         for the page size to have taken it — measured without the second
         half, a load still sent one 50-row request and aborted it for the
         9-row one — and then asks once. */
      const ready = db.sizeMode !== 'fit' || (db.fitLatched && db.pageSize === db.rowsThatFit);
      if (!active || !feed || p.keys.length === 0) {
        barState = { loading: false, error: null, answer: null, rows: [], local: false };
        return;
      }
      if (p.why) {
        barState = { loading: false, error: p.why, answer: null, rows: [], local: false };
        return;
      }
      if (!ready) return;
      const controller = new AbortController();
      const mine = ++barToken;
      barState = { ...untrack(() => barState), loading: true, error: null };
      const wire = wireSort(key);
      const started = performance.now();
      (wire
        ? readServerPage(feed, p, b, wire, desc, page, size, controller.signal)
        : readLocalPage(feed, p, b, set, controller.signal)
      )
        .then((/** @type {any} */ next) => {
          if (mine !== barToken || controller.signal.aborted) return;
          pageMs = Math.round(performance.now() - started);
          if (next.refused !== undefined) {
            /* REFUSED BEFORE ANY ROW WAS READ, AND THE GRID GOES BACK TO TIME
               ORDER — a sort the selection cannot afford is not kept (DB-B50). */
            sortWhy = `${BAR_META[key]?.label ?? key} cannot be ordered here: this query keeps ${fmt(next.refused)} bar(s), and the browser orders at most ${fmt(LOCAL_SORT_BUDGET)} of a column the server cannot sort. Narrow to a day, a time of day, a strike or a side — or sort by a stored column (Date, Time, Open, High, Low, Close, Volume, Open interest), which the server orders over any number.`;
            barSortKey = 'ts';
            barDesc = true;
            return;
          }
          barState = next;
          /* A PAGE PAST THE END OF A SHORTER ANSWER GOES TO THE LAST PAGE, so a
             narrowing never strands the reader on a blank page. */
          const total = next.answer?.total ?? 0;
          if (total > 0 && (page - 1) * size >= total) db.page = pageOfIndex(total - 1, size);
        })
        .catch((why) => {
          if (mine !== barToken || controller.signal.aborted) return;
          barState = {
            loading: false,
            error: String(why && why.message ? why.message : why),
            answer: null,
            rows: [],
            local: false
          };
        });
      return () => controller.abort();
    });

    /** The read's refusal, as a list, for the notes under the grid. */
    const barFails = $derived(barState.error ? [{ key: 'read', error: barState.error }] : []);
    /** A read answered in part (206): the rows are real, the set is not whole, and the fault is named. */
    const barFaults = $derived(barState.answer?.faults ? [{ key: 'faults', faults: barState.answer.faults }] : []);
    /** What a value sort read instead of serving from sort indexes, in the server's words (D-3349, DB-N14). */
    const barSortNote = $derived(sortIndexNote(barState.answer));

    /* ======================================================================
       THE DIGITS THAT REPEAT DOWN EVERY ROW EARN NONE OF THEIR WEIGHT
       ======================================================================
       The shared opening of the four prices is dimmed — DIMMED, NOT REMOVED:
       the text stays in the cell, so a copy, an export and a screen reader all
       still get the whole number. Computed from the page, which is the honest
       scope: "these rows open alike". */
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
      /* FOUR CHARACTERS ARE NEVER DIMMED, so a run of identical bars reads as
         a quiet column rather than a grid of ghosts. */
      const cap = Math.max(0, shortest - 4);
      const out = (pfx ?? '').slice(0, cap);
      return out.length >= 2 ? out : '';
    });

    /** One price, split into the run every row shares and the part that is this
        row's own. @param {number | null | undefined} v */
    function priceSplit(v) {
      const s = paisaText(v);
      return pricePfx && s.startsWith(pricePfx) ? [pricePfx, s.slice(pricePfx.length)] : ['', s];
    }

    /**
     * THE ROWS, FROM THE WIRE. Every derived figure the server decides travels
     * with the bar and is taken as it came — the change and its reason folded
     * against the previous stored bar wherever it lies (D-3325; DB-011,
     * DB-G65, DB-G90), open interest `null` with `not_a_derivative` for an
     * index or a share (D-3322; DB-036, DB-G28, DB-G44, DB-G100), and an
     * index's volume with `index_no_volume` (DB-057). The contract fields come
     * from the census row of the bar's own series; the month is the bar's own
     * IST month, so its spot and greeks are asked in it.
     */
    /**
     * Wire bars as grid rows, under the plan that asked for them. Published
     * as `barRowsOf` so a whole-query export builds the same rows the grid
     * draws, not a second spelling of them.
     * @param {any[]} wire @param {any} [asked] the plan; the current one by default
     */
    function barRowsOf(wire, asked = plan) {
      /** @type {any[]} */
      const out = [];
      const only = asked.keys.length === 1 ? asked.keys[0] : null;
      for (const b of wire) {
        const key = typeof b.key === 'string' ? b.key : only;
        if (key === null) continue;
        const c = asked.series.get(key)?.census ?? null;
        const ts = b.t * 1000;
        const day = istDayKey(ts);
        const exp = c?.expiry ?? null;
        const oi = typeof b.oi === 'number' ? b.oi : null;
        out.push({
          rk: rowKey(key, b.t),
          key,
          instrument: key,
          head: c?.head,
          sym: c?.sym,
          month: typeof b.month === 'string' ? b.month : day.slice(0, 7),
          ts,
          day,
          tod: (b.t + IST_OFFSET_MS / 1000) % 86400,
          tf: asked.timeframe,
          o: b.o,
          h: b.h,
          l: b.l,
          c: b.c,
          vol: b.v,
          vWhy: b.v_why ?? null,
          chg: typeof b.chg === 'number' ? b.chg : null,
          chgWhy: typeof b.chg === 'number' ? null : (b.chg_why ?? null),
          oi,
          /* WHY THE OPEN INTEREST IS NOT A NUMBER: not applicable, or the null
             sentinel. A ZERO IS A ZERO and carries no reason. */
          oiWhy: b.oichg_why === 'not_a_derivative' ? 'not_a_derivative' : oi === null ? 'oi_null' : null,
          oichg: typeof b.oichg === 'number' ? b.oichg : null,
          oichgWhy: typeof b.oichg === 'number' ? null : (b.oichg_why ?? null),
          phase: b.phase ?? null,
          segment: c?.kind ?? null,
          exp,
          dte: exp === null ? null : daysBetween(day, exp),
          side: c?.side ?? null,
          strike: c?.strike ?? null
        });
      }
      return out;
    }
    const barRows = $derived(barRowsOf(barState.rows, plan));

    /** Bars on this page, and what the census claimed for the same series. */
    const barsRead = $derived(barRows.length);
    const barsClaimed = $derived([...plan.series.values()].reduce((a, s) => a + s.rows, 0));

    /**
     * EVERY MINUTE THE LOADED ROWS HOLD, for the time pickers' extra entries
     * (pre-open or closing-auction minutes). O(rows on the page).
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
      return [...seen.keys()].sort().map((t) => ({ key: t, name: t, detail: `${fmt(seen.get(t) ?? 0)} bar(s)` }));
    });
    const timeBounds = $derived([{ key: '', name: 'Any', detail: 'no bound' }, ...timeOptions]);

    /* THE BOUNDS ARE APPLIED ON THE SERVER, so the rows that came back are the
       rows the bounds keep; nothing is filtered after the fetch. */
    const barWindowed = $derived(barRows);
    const spanCeil = $derived(istDayKey(Date.now()));

    const dayBounds = $derived.by(() => {
      let lo = '';
      let hi = '';
      for (const b of barRows) {
        if (!b.day) continue;
        if (lo === '' || b.day < lo) lo = b.day;
        if (hi === '' || b.day > hi) hi = b.day;
      }
      return [lo, hi];
    });

    /**
     * THE ORDER ON SCREEN. A server-ordered page is drawn in the server's
     * order and never re-sorted here — re-sorting a page is how a page of
     * contract 1 and a page of contract 2 were once shown as one order
     * (DB-002). A locally ordered sort orders every row it read.
     */
    const barSorted = $derived(
      barState.local ? [...barRows].sort(localComparator(barSortKey, barDesc)) : barRows
    );

    /** @param {string} key */
    function barHead(key) {
      const col = BAR_COLS.find((c) => c.key === key);
      if (!col || col.none || col.nosort) return; /* nothing to order, and the button says why */
      sortWhy = '';
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
       PAGING — the server's total, the same arithmetic for both grids.
       --------------------------------------------------------------------- */
    /** The server's answer as the controls read it: its total and whether it scanned. */
    const windowSaid = $derived(
      barState.answer
        ? { total: Number(barState.answer.total) || 0, extremes: null, scanned: barState.answer.scanned === true }
        : null
    );

    /**
     * THE ROWS THE QUERY KEEPS, FROM THE SERVER — after every bound, over every
     * matched month and series (DB-004, DB-G02, DB-G61, DB-G93). A local sort
     * orders exactly those rows, so its total is the same number.
     */
    const pageTotal = $derived(
      db.view === 'bars' ? (windowSaid?.total ?? 0) : db.sorted.length
    );

    /* THE TIME BOUND IS THE SERVER'S NOW, over the whole query, so it is never
       partial. Kept as a name for the control that used to warn about it. */
    const timePartial = $derived(false);
    const pageCount = $derived(Math.max(1, Math.ceil(pageTotal / db.pageSize)));
    const pageNow = $derived(Math.min(Math.max(1, db.page), pageCount));
    const pageFrom = $derived(pageTotal === 0 ? 0 : (pageNow - 1) * db.pageSize + 1);
    const pageTo = $derived(Math.min(pageNow * db.pageSize, pageTotal));

    /**
     * THE CENSUS AGAINST THE FILES, BY SERIES (DB-G02, DB-G61, DB-022). The
     * pager counts the files; a census row behind them (an import running, a
     * month written after the census was read) is said here, as a lag, never
     * as a corrupt store. Asked only when no bound narrows the read.
     */
    const barDisagree = $derived(
      !barState.answer || dayWindowNarrows || timeNarrows ? [] : censusBehind(barState.answer.series_totals, plan.series)
    );

    /** Months inside a series' span with no file — the holes the server names. */
    const holeMonths = $derived(
      barState.answer
        ? Math.max(0, (Number(barState.answer.months_missing) || 0) - (Number(barState.answer.months_outside_life) || 0))
        : 0
    );

    /**
     * WHY AN EMPTY ANSWER IS EMPTY (DB-016): the days a narrowing window asked
     * for, each as the exchange calendar of record calls it — closed, a
     * session, or beyond its reach — so an empty grid names its reason and
     * never accuses the store of what a weekend explains. At most 62 days are
     * named; a wider empty window is said as a window.
     */
    const emptyNote = $derived.by(() => {
      const a = barState.answer;
      if (db.view !== 'bars' || barState.loading || barState.error || !a || a.total > 0) return null;
      if (!dayWindowNarrows && !timeNarrows) return { kind: 'none', closed: [], open: [], unknown: [] };
      /** @type {string[]} */
      const days = [];
      if (bounds.fromDay && bounds.toDay) {
        const from = Date.parse(`${bounds.fromDay}T00:00:00Z`);
        const to = Date.parse(`${bounds.toDay}T00:00:00Z`);
        for (let at = from; Number.isFinite(at) && at <= to && days.length < 62; at += 86_400_000) {
          days.push(new Date(at).toISOString().slice(0, 10));
        }
      }
      const said = days.map((d) => ({ d, closed: db.dayClosed(d) }));
      return {
        kind: 'window',
        closed: said.filter((x) => x.closed === true).map((x) => x.d),
        open: said.filter((x) => x.closed === false).map((x) => x.d),
        unknown: said.filter((x) => x.closed === null).map((x) => x.d)
      };
    });

    /** The page as the server (or the local sort) gives it. */
    const barPageRead = $derived(
      barState.local
        ? barSorted.slice((pageNow - 1) * db.pageSize, pageNow * db.pageSize)
        : barSorted
    );
    /* THE MONEYNESS PICK KEEPS THE BARS WHOSE OWN RUNG IT IS (WP-53; D-3329,
       DB-075). A rung is the server's answer at each bar's own instant, so it
       is applied to the page read, after the server paged: the pager still
       counts the query's bars, and DbMoneyness says so. The option cells read
       `barPageRead`, so a bar the pick sets aside is still asked about. */
    const barPage = $derived(db.rungKeep ? barPageRead.filter(db.rungKeep) : barPageRead);

    /* ---------------------------------------------------------------------
       THE NEWEST STORED BAR OF THE SELECTION — one seek per selection, never
       the first row of a page (DB-007). It describes the selection whole, so
       it ignores the day and time bounds, as the "on disk" range beside it
       does.
       --------------------------------------------------------------------- */
    let newestClose = $state.raw(/** @type {{ bar: any, key: string } | { why: string } | null} */ (null));
    $effect(() => {
      const feed = feeds.active;
      const p = planAll;
      void db.nonce;
      if (db.view !== 'bars' || !feed || p.keys.length === 0 || p.why) {
        newestClose = null;
        return;
      }
      const controller = new AbortController();
      readWindowPage(windowQuery(feed, p, { sort: 'ts', desc: true, offset: 0, limit: 1 }), 1, controller.signal)
        .then((answer) => {
          if (controller.signal.aborted) return;
          const bar = answer.bars[0];
          newestClose = bar ? { bar, key: typeof bar.key === 'string' ? bar.key : p.keys[0] } : { why: 'no bar is stored for this selection' };
        })
        .catch((why) => {
          if (controller.signal.aborted) return;
          newestClose = { why: String(why && why.message ? why.message : why) };
        });
      return () => controller.abort();
    });

    /** What the last jump has to say — a refusal, or how far it had to go. */
    let jumpWhy = $state('');

    /**
     * GO TO AN INSTANT (DB-005): the page holding the first bar at or after it
     * (ascending) or at or before it (descending), across the WHOLE query.
     *
     * The server finds it — `at=` answers the global index in one rank per
     * series — and the page is that index over the page size. It used to search
     * only the fifty rows in memory and turn to a page computed from the first
     * FILE's offset, landing about 17,450 one-second bars from where it said.
     * The row it lands on is marked; when no bar opens exactly there, the line
     * under the strip says which bar was taken and how far away it is.
     *
     * A value sort has no position in time, so a jump puts the grid back in
     * time order first and says so.
     *
     * @param {number} at unix seconds @param {string} label the instant as typed
     * @param {string} [key] the series whose bar at `at` is meant, when the
     *   caller knows it (an extreme): several series can share one second
     */
    async function jumpToInstant(at, label, key = '') {
      db.landedRk = '';
      jumpWhy = '';
      const feed = feeds.active;
      const p = plan;
      if (!feed || p.keys.length === 0 || p.why) {
        jumpWhy = p.why ?? 'No selection is read, so there is nothing to go to.';
        return;
      }
      const resorted = barSortKey !== 'ts';
      if (resorted) {
        barSortKey = 'ts';
        barDesc = true;
      }
      const desc = barDesc;
      try {
        const answer = await readWindowPage(
          windowQuery(feed, p, { ...bounds, sort: 'ts', desc, at, limit: 1 }), 1, new AbortController().signal
        );
        const index = Number(answer.at_index);
        const total = Number(answer.total);
        const side = desc ? 'at or before' : 'at or after';
        if (total === 0) {
          jumpWhy = 'No bar is kept by this query, so there is nothing to land on.';
          return;
        }
        if (!Number.isSafeInteger(index) || index >= total) {
          db.page = pageOfIndex(total - 1, db.pageSize);
          jumpWhy = `No bar lies ${side} ${label} in this query: it ${desc ? 'begins after' : 'ends before'} that instant. Showing its ${desc ? 'oldest' : 'newest'} page.`;
          return;
        }
        const bar = answer.bars[0];
        db.page = pageOfIndex(index, db.pageSize);
        if (bar) {
          db.landedRk = key && bar.t === at ? rowKey(key, at) : rowKey(typeof bar.key === 'string' ? bar.key : p.keys[0], bar.t);
        }
        const lead = resorted ? 'The grid is back in time order, which is the order an instant has a place in. ' : '';
        jumpWhy = !bar || bar.t === at
          ? lead
          : `${lead}No bar opens at ${label}. Landed on ${dayLabel(bar.t * 1000)} ${barTimeLabel(bar.t * 1000, 1)} IST, the first bar ${side} it, ${fmt(Math.abs(bar.t - at))} second(s) away — row ${fmt(index + 1)} of ${fmt(total)}.`;
      } catch (why) {
        jumpWhy = `Go to could not be answered: ${String(/** @type {any} */ (why)?.message ?? why)}`;
      }
    }

    /**
     * THE TIME PICKER'S GO TO: a minute (or, on 1s, a second) on the day the
     * reader is looking at — the first row on screen — else the window's own
     * last day. `HH:MM` and `HH:MM:SS` are both accepted (DB-005).
     *
     * @param {string} [at] the time to land on
     */
    function jumpToTime(at = db.goTime) {
      if (dayRung) {
        jumpWhy = `Every bar at ${db.timeframe} opens at the same minute, so there is no minute to choose between them.`;
        return;
      }
      const day = barPage[0]?.day ?? (db.toDay || db.fromDay || '');
      const got = parseGoto(String(at ?? ''), day);
      if ('why' in got) {
        jumpWhy = got.why;
        return;
      }
      void jumpToInstant(got.at, got.label);
    }

    /**
     * THE GO-TO BOX'S TEXT: `YYYY-MM-DD HH:MM[:SS]`, or a time alone for the
     * day on screen.
     * @param {string} text
     */
    function jumpToText(text) {
      const day = barPage[0]?.day ?? (db.toDay || db.fromDay || '');
      const got = parseGoto(text, day);
      if ('why' in got) {
        db.landedRk = '';
        jumpWhy = got.why;
        return;
      }
      void jumpToInstant(got.at, got.label);
    }

    /** @param {number} n */
    function goPage(n) {
      const want = Number(n);
      db.page = Math.min(pageCount, Math.max(1, Number.isFinite(want) ? Math.trunc(want) : 1));
    }

    /** The global row a page-size change promised to keep on screen, until it is shown. */
    let anchorRow = $state(-1);
    /**
     * KEEP THE READER'S PLACE ACROSS A PAGE-SIZE CHANGE, NOT THEIR ORDINAL: the
     * row they were standing on — the first row of the old page — is on the
     * new page, and it is scrolled into view and marked when the page arrives
     * (DB-N06, DB-N12). It used to land on the right page with the box
     * scrolled to the top, the row 92 rows below it, off screen.
     * @param {number} n @param {boolean} [anchor] whether the reader chose it
     */
    function setPageSize(n, anchor = true) {
      const firstRow = (pageNow - 1) * db.pageSize;
      db.pageSize = Number(n);
      db.page = Math.floor(firstRow / db.pageSize) + 1;
      anchorRow = anchor && pageTotal > 0 ? firstRow : -1;
    }
    $effect(() => {
      const want = anchorRow;
      /* THE SERVER'S PAGE, which `want` indexes; the row drawn for it is
         found in what the grid shows, which a Moneyness pick may have
         narrowed (WP-53). */
      const rows = barPageRead;
      const box = db.barBox;
      if (want < 0 || barState.loading || !box || rows.length === 0) return;
      const index = want - (pageNow - 1) * db.pageSize;
      untrack(() => {
        anchorRow = -1;
        const row = rows[index];
        if (!row) return;
        db.landedRk = row.rk;
        const drawn = barPage.indexOf(row);
        if (drawn < 0) return;
        /* AFTER THE ROWS ARE IN THE DOCUMENT: the next turn of the loop. */
        setTimeout(() => {
          const tr = /** @type {HTMLElement | null} */ (box.querySelectorAll('tbody tr.brow')[drawn] ?? null);
          if (!tr) return;
          box.scrollTop = Math.max(0, tr.offsetTop - HEAD - Math.max(0, (box.clientHeight - HEAD - tr.offsetHeight) / 2));
        }, 0);
      });
    });

    /* ONE SCALE PER RUNG, because a rung is what makes two moves comparable,
     * and each is measured ON THIS PAGE — said so on every title (DB-049).
     *
     * Volume: a LOG ruler over the page's own span. A quantity that moves over
     * two orders of magnitude has no linear picture; the title names the
     * ruler and both its ends. */
    const volFullByTf = $derived.by(() => {
      /** @type {Map<string, { lo: number, hi: number }>} */
      const span = new Map();
      for (const b of barPage) {
        if (b.vWhy === 'index_no_volume') continue;
        const v = b.vol ?? 0;
        /* ZERO IS OUT OF THE RULER, NOT CLAMPED INTO IT: it has no logarithm. */
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
     * Where one volume sits on its rung's log ruler, 0..100. O(1).
     * @param {number} v @param {string} tf
     */
    function volMag(v, tf) {
      const s = volFullByTf.get(tf);
      if (s === undefined || !(v > 0)) return 0;
      if (s.hi <= s.lo) return 100;
      /* A FLOOR OF 4%, so the smallest volume on the page is a mark and only
         zero draws nothing. */
      const FLOOR = 4;
      const lo = Math.log(s.lo);
      const x = Math.log(Math.min(Math.max(v, s.lo), s.hi));
      return FLOOR + Math.round(((x - lo) / (Math.log(s.hi) - lo)) * (100 - FLOOR));
    }

    /** @param {number} v @param {string} tf */
    function volStyle(v, tf) {
      const m = volMag(v, tf);
      return `--vmag:${m}%;--vmagpx:${Math.round((m / 100) * 56)}px`;
    }

    /**
     * WHAT THE VOLUME CELL MEANS, IN ITS UNITS (D-3343, DB-057, DB-G76).
     *
     * A share's and an option's or a future's volume is in SHARES of the
     * underlying — GDFL's last traded quantity is shares, not lots — and no lot
     * is shown: no lot size is served to this page, from `costs::lot` or from
     * the inferred-lot ledger, so a lots figure would be a number this page
     * invented. An index has no traded volume at all.
     *
     * @param {any} b the row
     */
    function volTitle(b) {
      const v = b.vol;
      const tf = b.tf;
      if (b.vWhy === 'index_no_volume') {
        return `Not applicable: an index has no traded volume (D-2802, D-3322). The store holds ${fmt(v)} here, which is not a measurement.`;
      }
      const unit = b.side
        ? 'shares of the underlying (an option’s traded quantity, lots × lot size — not lots). No lot is shown: no lot size is served for this contract, sourced or inferred (D-3343)'
        : b.exp
          ? 'shares of the underlying (a future’s traded quantity, not lots). No lot is shown (D-3343)'
          : 'shares';
      const s = volFullByTf.get(tf);
      if (s === undefined || !(v > 0)) {
        return v === 0
          ? `0 ${unit}: nothing traded in this bar. Zero has no place on a log ruler, so no bar is drawn — this is a reading, not a gap.`
          : `${fmt(v)} ${unit}.`;
      }
      return (
        `${fmt(v)} ${unit}. The bar is a LOG scale across the ${tf} rows ON THIS PAGE — ` +
        `${fmt(s.lo)} at no bar to ${fmt(s.hi)} at full width — so a bar twice as long is far more ` +
        `than twice the volume, and turning the page can change what a full bar means.`
      );
    }

    /* THE BAR'S OWN RANGE, high minus low, as a share of the widest range at that
     * rung ON THIS PAGE — raised to the query's widest when the extremes panel
     * has answered, since a page is a subset of the query and its own widest
     * can never legitimately exceed it. */
    const rngFullByTf = $derived.by(() => {
      /** @type {Map<string, number>} */
      const top = new Map();
      for (const b of barPage) {
        const r = b.h - b.l;
        if (r > (top.get(b.tf) ?? 0)) top.set(b.tf, r);
      }
      const wide = db.extremesNow?.range;
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
      /* SQUARE ROOT, NOT LINEAR: it keeps the order exact and spends the
         height where the rows are. The widest bar is still full height. */
      return Math.max(MIN_SPINE, Math.sqrt(Math.min(r, full) / full));
    }

    /* THE CHANGE SCALE IS THE LARGEST MOVE ON THIS PAGE, per rung (DB-049). The
       header used to say "scaled across every matched row rather than this
       page" while the fold read only the page; it now says what it does. */
    const chgFullByTf = $derived.by(() => {
      /** @type {Map<string, number>} */
      const top = new Map();
      for (const b of barPage) {
        if (b.chg === null) continue;
        const a = Math.abs(b.chg);
        if (a > (top.get(b.tf) ?? 0)) top.set(b.tf, a);
      }
      for (const [tf, v] of top) top.set(tf, Math.max(v, CHG_FLOOR_BPS));
      return top;
    });

    /** The rungs on screen and their scales, for the header's own explanation. */
    const chgScaleText = $derived([...chgFullByTf].map(([tf, v]) => `${tf} ${fmt(v)} bps`).join(', '));

    /** @param {number} bps @param {string} tf */
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
      barQueryPlan: { get: () => plan },
      barQueryAll: { get: () => planAll },
      barBounds: { get: () => bounds },
      barSetKey: { get: () => setKey },
      barState: { get: () => barState, set: (v) => (barState = v) },
      barFails: { get: () => barFails },
      barFaults: { get: () => barFaults },
      barSortNote: { get: () => barSortNote },
      pageMs: { get: () => pageMs },
      pricePfx: { get: () => pricePfx },
      priceSplit: { get: () => priceSplit },
      barRows: { get: () => barRows },
      barRowsOf: { get: () => barRowsOf },
      barsRead: { get: () => barsRead },
      barsClaimed: { get: () => barsClaimed },
      barSortKey: { get: () => barSortKey, set: (v) => (barSortKey = v) },
      barDesc: { get: () => barDesc, set: (v) => (barDesc = v) },
      sortWhy: { get: () => sortWhy, set: (v) => (sortWhy = v) },
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
      barPageRead: { get: () => barPageRead },
      barHoleMonths: { get: () => holeMonths },
      emptyNote: { get: () => emptyNote },
      newestClose: { get: () => newestClose },
      jumpWhy: { get: () => jumpWhy, set: (v) => (jumpWhy = v) },
      jumpToTime: { get: () => jumpToTime },
      jumpToText: { get: () => jumpToText },
      jumpToInstant: { get: () => jumpToInstant },
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

    /* THE EXTREMES PANEL'S STATE (DbExtremes, DB-G95), created here so it reads
       the plan and the bounds above and runs in the shell's order. */
    extremesState(db);
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
      if (!db.fitLatched) return;
      const want = db.rowsThatFit;
      untrack(() => {
        /* NO ANCHOR: the reader did not ask for this size, so no row is marked
           or scrolled to — the place is kept by the arithmetic alone. */
        if (want > 0 && want !== db.pageSize) db.setPageSize(want, false);
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
      void db.barSetKey;
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
<DbOptionNotes {db} />
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
                      ? `${c.label} — ${c.why}. The bar behind each figure is that figure's size against the largest move ON THIS PAGE, measured PER RUNG so a minute bar is never judged against a daily one (${db.chgScaleText || 'no move on this page'}). It is the page's scale, not the query's: turning the page can change what a full bar means. Click to sort; click again to reverse — the browser orders this column itself, over at most ${fmt(LOCAL_SORT_BUDGET)} bars.`
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
                  <h3>This page of bars could not be read</h3>
                  <p class="err">{db.barState.error}</p>
                  <p>
                    Nothing is drawn rather than a plausible blank: a grid with no rows and no
                    reason reads as an empty store, and this is a failed or refused read. The
                    sentence above is the server's own.
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
          {:else if db.barPage.length === 0 && db.rungHidden?.hidden > 0}
            <!-- THE MONEYNESS PICK SET ASIDE EVERY BAR OF THIS PAGE (WP-53,
                 DB-075): the page is not empty, its bars sit on other rungs. -->
            <tr class="bstate">
              <td colspan={BAR_COLS.length}>
                <div class="bnone">
                  <h3>No bar of this page is {[...db.mnyPick][0]}</h3>
                  <p>
                    The server sent {fmt(db.rungHidden.of)} bar(s) for this page and none sits on {[...db.mnyPick][0]}
                    at its own instant{#if db.rungHidden.unread > 0} — {fmt(db.rungHidden.unread)} rung(s) are still being
                      read{/if}. The rung is applied to each page after the server pages it, so another page may hold
                    some; or clear the Moneyness pick.
                  </p>
                </div>
              </td>
            </tr>
          {:else if db.barPage.length === 0}
            <tr class="bstate">
              <td colspan={BAR_COLS.length}>
                <div class="bnone">
                  <!-- WHY NO BAR CAME BACK, AND IT IS NEVER A STORE ACCUSATION
                       WHEN A WINDOW EXPLAINS IT (DB-016).

                       The bounds go to the server, so an empty window is an
                       empty ANSWER: 0 bars for a day nothing traded on. This
                       branch used to read "1 instrument-month(s) were read and
                       0 bar(s) arrived … a disagreement between the manifest
                       and the file" for GDFL NIFTY 1s on 2024-03-02, a
                       Saturday, accusing an intact file of what the calendar
                       explains. It now names the days asked for and what the
                       exchange calendar says of each. -->
                  {#if db.emptyNote?.kind === 'window'}
                    <h3>No bar in this window</h3>
                    <p>
                      Nothing is stored for <b>{db.barBounds.fromDay || 'the start'}</b> to
                      <b>{db.barBounds.toDay || 'the end'}</b>{#if db.barBounds.fromTime || db.barBounds.toTime}, between
                        <b>{db.barBounds.fromTime || '00:00'}</b> and <b>{db.barBounds.toTime || '23:59'}</b> IST{/if}
                      in this selection.
                      {#if db.emptyNote.closed.length}
                        The exchange calendar marks {db.emptyNote.closed.map((/** @type {string} */ d) => dayLabel(d)).join(', ')} closed — a
                        weekend or a holiday — so no bar is owed there.
                      {/if}
                      {#if db.emptyNote.open.length}
                        {db.emptyNote.open.map((/** @type {string} */ d) => dayLabel(d)).join(', ')}
                        {db.emptyNote.open.length === 1 ? 'was a session' : 'were sessions'}, and this selection
                        stores no bar there: the import may have refused the day, or the store has a gap. The day
                        flags name a refusal where one was recorded.
                      {/if}
                      {#if db.emptyNote.unknown.length}
                        The exchange calendar does not reach {db.emptyNote.unknown.map((/** @type {string} */ d) => dayLabel(d)).join(', ')}, so
                        whether a session was owed there is not known here.
                      {/if}
                      Widening the window brings the selection's other bars back.
                    </p>
                    <!-- EACH ASKED DAY'S OWN REASON, FROM THE DAY BAND (DB-016,
                         DB-025, WP-54): a refused import with its words, a
                         closed day, a store gap, and the other feed's count.
                         `null` while the band has not answered this window. -->
                    {#if db.dayEmptyWhy}
                      <p class="bwhy">{db.dayEmptyWhy}</p>
                    {:else if db.dayBandLoading}
                      <p class="bwhy">Reading why these days hold no bar from /dayflags.json…</p>
                    {/if}
                  {:else}
                    <h3>No bar came back</h3>
                    <p>
                      {#if db.barsClaimed > 0}
                        The census counts <b>{fmt(db.barsClaimed)}</b> bar(s) for this selection and the files
                        it names hold none. That is a disagreement between the census and the files, not an
                        empty query — read again, and if it stands, the census has run ahead of a file that
                        was replaced.
                      {:else}
                        No bar is stored for this selection.
                      {/if}
                    </p>
                  {/if}
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
                  {#if b.vWhy === 'index_no_volume'}
                    <!-- AN INDEX HAS NO TRADED VOLUME (D-2802, D-3322; DB-057):
                         the stored figure is not a measurement, so it is not
                         printed as one. -->
                    <td class="bn na" title={db.volTitle(b)}
                      ><span class="unk" aria-label="Volume: not applicable — {db.volTitle(b)}">n/a</span></td
                    >
                  {:else}
                    <td class="bn bvol" style={db.volStyle(b.vol, b.tf)} title={db.volTitle(b)}>{fmt(b.vol)}</td>
                  {/if}

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
                    {#if b.oiWhy === 'not_a_derivative'}
                      <!-- N/A, NOT A DERIVATIVE (D-3322; DB-036, DB-G28,
                           DB-G44, DB-G100): an index or a share has no open
                           interest on any feed, whatever was stored. -->
                      <span
                        class="unk"
                        title={barWhyText('not_a_derivative')}
                        aria-label="Open interest: {barWhyText('not_a_derivative')}">n/a</span
                      >
                    {:else if b.oi === null}
                      <span
                        class="unk"
                        title={barWhyText('oi_null')}
                        aria-label={barWhyText('oi_null')}>unknown</span
                      >
                    {:else}
                      <span
                        title={b.oi === 0
                          ? 'zero contracts open, in shares of the underlying, stored as zero — not the null sentinel'
                          : 'open interest as stored, in SHARES of the underlying — not lots (D-3343). No lot is shown: no lot size is served for this contract.'}>{fmt(b.oi)}</span
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

    <!-- EVERY REFUSED SORT, PARTIAL READ, LAG AND HOLE, NAMED. A refused or
         failed read is the grid's own state above, in the server's words, so
         it is not said twice here. None of these is allowed to be a shorter
         grid. -->
    {#if db.sortWhy || db.barSortNote || db.barFaults.length || db.barDisagree.length || db.barHoleMonths > 0}
      <div class="bnotes">
        {#if db.sortWhy}
          <p class="bnote warn">{db.sortWhy}</p>
        {/if}
        {#if db.barSortNote}
          <p class="bnote">{db.barSortNote}</p>
        {/if}
        {#if db.barDisagree.length}
          <!-- THE CENSUS IS BEHIND THE FILE, SAID AS A LAG (DB-G02, DB-G61,
               DB-022). The pager counts the files, so no bar past the census
               count is unreachable; the census is the snapshot the page read,
               and a running import moves a file ahead of it. -->
          <p class="bnote">
            The census differs from the files for <b>{fmt(db.barDisagree.length)}</b> series —
            <b>{db.barDisagree[0].key}</b>: the file holds {fmt(db.barDisagree[0].file)} bar(s), the census
            counted {fmt(db.barDisagree[0].census)}{#if db.barDisagree.length > 1}, and {fmt(db.barDisagree.length - 1)} more{/if}.
            The pager counts the files' own rows, so every bar is reachable in either order; the census
            is the snapshot this page read, and an import running since moves a file ahead of it.
          </p>
        {/if}
        {#if db.barHoleMonths > 0}
          <p class="bnote">
            <b>{fmt(db.barHoleMonths)}</b> month(s) inside a series' span hold no file{#if db.barState.answer?.missing_months?.length}:
              {db.barState.answer.missing_months.join(', ')}{#if db.barHoleMonths > db.barState.answer.missing_months.length} and more{/if}{/if}.
            {#if db.barState.answer?.months_outside_life}{fmt(db.barState.answer.months_outside_life)} more lie outside a contract's life and are not owed.{/if}
          </p>
        {/if}
        {#each db.barFaults as f (f.key)}
          <p class="bnote warn">Read in part: {f.faults}</p>
        {/each}
      </div>
    {/if}
  </div>
<DbProvenance {db} />
<DbScrubNote {db} />

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

    {#if db.view === 'bars'}
      <!-- GO TO AN INSTANT, ACROSS THE WHOLE QUERY (DB-005). A day and a
           time, to the second — `2019-03-20 12:33:38` — or a time alone for
           the day on screen. The server finds the page that holds it; the row
           it lands on is marked. -->
      <label class="pgjump pgat">
        <span>Go to</span>
        <input
          type="text"
          inputmode="numeric"
          placeholder={db.secondRung ? '2019-03-20 12:33:38' : '2019-03-20 12:33'}
          disabled={db.pageTotal === 0}
          title="An instant to open the page of: a day and a time to the second, 2019-03-20 12:33:38, or a time alone for the day on screen. Lands on the first bar at or after it (oldest first) or at or before it (newest first), across every month and series of the query, and marks it. Press Enter."
          onkeydown={(e) => {
            if (e.key === 'Enter') db.jumpToText(e.currentTarget.value);
          }}
        />
      </label>
      {#if db.jumpWhy}<span class="pgof2 pgwhy">{db.jumpWhy}</span>{/if}
    {/if}

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
    /* TEN ROWS AT LEAST (DB-N11). 220px held four option rows at 1440x900,
       and Fit then paged a 1,53,107-row contract four rows at a time. The
       floor is the sticky head, the fit's slack and ten 40px rows; on a window
       too short for it the page scrolls, which is the honest outcome. */
    min-height: calc(30px + 24px + 10 * 40px);
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
  /* THE DAY BAND'S OWN REASON FOR AN EMPTY WINDOW (DB-016, WP-54). */
  .bnone p.bwhy {
    color: var(--ink-2);
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
  .pgat input {
    width: 200px;
    text-align: left;
  }
  .pgwhy {
    flex-basis: 100%;
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
