<script>
  /**
   * THE DB VIEW — what the store actually holds, for ONE feed.
   *
   * Named `/db` because that is what it is: a database view. It never shows two
   * feeds side by side; the selector in the top bar chooses one and this shows
   * that one's rows.
   *
   * # Why it is built the way it is
   *
   * **Summary before detail.** Twenty thousand rows answer "which
   * instrument-month" and answer nothing about the shape of the store. The
   * strip at the top is the question an operator actually opens this page with:
   * how much is here, how much of it is complete, and where are the holes. The
   * month band underneath it is the same question at the only other altitude
   * that matters — a hole is almost never one instrument, it is a day the pull
   * did not finish.
   *
   * **Sortable, because the interesting rows are never alphabetical.** The
   * instrument missing 8,478 bars matters more than the one that is full, and
   * alphabetical order buries it at whatever letter it starts with. Every
   * column that HOLDS DATA sorts, and the direction is on screen — an arrow and
   * `aria-sort`, not a convention the operator has to remember.
   *
   * **Windowed.** Only the visible slice is in the DOM. Twenty thousand rows is
   * already past what a browser lays out comfortably and 93,776 — the census
   * size `docs/06-limits.md` §34 projects for the end of the stated backfill —
   * is not survivable at all. Rows are absolutely positioned inside a spacer of
   * the exact total height, so the scrollbar is honest at any row count and no
   * rounding accumulates down the list.
   *
   * **Indexed filtering.** A prefix Map is built once per dataset in yielding
   * chunks. Longer text and membership filters inspect only the chosen prefix
   * bucket; that bucket can be large. No request is made per character.
   *
   * **The two percentage columns are LIVE, and most of their cells are still a
   * dash.** They used to be refused outright, because `/store.json` carried no
   * price at all; D-0067 put the month's first and last close in the census and
   * D-0069 made the server compute the ratio. What arrives now is INTEGER BASIS
   * POINTS — `125` is +1.25% — or `null` and one of five reason codes.
   *
   * A dash is therefore never "nothing happened" and never a zero: it is one
   * named fact out of five, and hovering it says which. The commonest by far is
   * `corporate_action_unverified` — no split-and-bonus threshold is sourced
   * anywhere in this repository, so `docs/05-decisions.md` D-0018 refuses every
   * equity rather than printing an unadjusted 1:5 split as an 80% crash.
   * Indices never split and do show a number. `CLAUDE.md` §4: degrade loudly
   * and name the reason.
   */

  /* ======================================================================
     THE SHELL (WP-00).
     ----------------------------------------------------------------------
     This file used to BE the page: 13,592 lines of script, markup and style.
     WP-00 split it, by moving code only, into one component or module per
     later work package, so each of them owns exactly one set of files:

       selection / controls ..... $lib/db/selection.svelte.js, DbControls
       search ................... DbSearch, $lib/db/search.js
       bars, pager, export ...... DbBarsGrid, DbSummary, DbExtremes,
                                  $lib/db/bars-plan.js, export.js, notes-bars.js
       option cells, chain ...... DbOptionCells, DbMoneyness, $lib/db/chain.js,
                                  notes-options.js
       coverage ................. DbCoverageGrid, DbDayFlags, DbCalendar,
                                  $lib/db/decoration.js, notes-coverage.js
       labels ................... DbProvenance, DbBasisBadge, notes-labels.js
       census client ............ DbCensusBanner
       chart .................... DbChart

     NOTHING WAS FIXED IN THE MOVE. Every defect the audit recorded is still
     present byte for byte; the packages above fix them. The placeholders
     (DbExtremes, DbDayFlags, DbCalendar, DbProvenance, DbBasisBadge,
     DbCensusBanner's markup and DbChart) render nothing until their owner
     fills them, and are mounted where that owner will need them.
     ====================================================================== */
  import './db.css';
  import { store, refreshStore } from '$lib/store.svelte.js';
  import { feeds, selectFeed } from '$lib/feeds.svelte.js';
  import DbCensusBanner, { censusState, censusLoading } from './DbCensusBanner.svelte';
  import DbControls, { feedRung, stranded } from './DbControls.svelte';
  import DbSummary, { summaryState } from './DbSummary.svelte';
  import DbCoverageGrid, { coverageState, coverageFlash, coverageDrawer, coverageStamp, notesPanel } from './DbCoverageGrid.svelte';
  import DbBarsGrid, { barsLayout, barsRead, barsFit, barsMove, pager } from './DbBarsGrid.svelte';
  import { selectionState, selectionSeeds, selectionPaging } from '$lib/db/selection.svelte.js';
  import { searchState } from './DbSearch.svelte';
  import { chainState } from './DbMoneyness.svelte';
  import { optionCellsState } from './DbOptionCells.svelte';
  import { dayFlagsState } from './DbDayFlags.svelte';

  /* ONE OBJECT CARRIES THE PAGE'S STATE, AND THE ORDER BELOW IS THE POINT.
     Each call declares its runes and publishes them onto `db` as getters
     (and setters for `$state`), so a component reads `db.matched` exactly as
     the page's markup read `matched`. The calls run in the order the page
     script created its `$effect`s — the effect at old line 766 first, the
     one at 7505 last — so effects still run within a flush in the order they
     always did, and the same requests leave in the same order. All of this
     lives here, in the shell, because the page's state outlives any one view:
     switching Coverage and Bars destroys a grid component, never its state. */
  /** @type {any} */
  const db = {};
  censusState(db);
  selectionState(db);
  searchState(db);
  chainState(db);
  coverageState(db);
  barsLayout(db);
  summaryState(db);
  barsRead(db);
  optionCellsState(db);
  barsFit(db);
  coverageFlash(db);
  selectionSeeds(db);
  coverageDrawer(db);
  selectionPaging(db);
  coverageStamp(db);
  barsMove(db);
  // THE DAY BAND (WP-54): read by the band, the bar grid's empty state and
  // the day fields, so it lives here rather than in any one of them.
  dayFlagsState(db);
</script>

<svelte:head><title>Database · brutex</title></svelte:head>

<svelte:window onkeydown={db.onWindowKey} onpointerdown={db.onWindowDown} />

<!-- A CONTROL OUTSIDE THE BAR ROWS NEEDS EVERY ROW (WP-56b): the first paint
     is drawn from the census rows of the instrument on screen, and the lists
     of every other instrument are read when a control that shows them is
     reached for (a press or a focus anywhere but the bar rows and the
     census banner). `db.censusReach` is a no-op once the whole census is
     held. -->
<div class="pane db" onpointerdowncapture={db.censusReach} onfocusincapture={db.censusReach}>
  {#if store.busy}<p class="inline-note" role="status">{store.busy}</p>{/if}
  <DbCensusBanner {db} />
  <!-- THE PANE HEAD IS GONE FROM THIS PAGE, ON THE OPERATORS CALL. It carried
       DB-Zerodha (already the lit nav tab plus the BROKER FEED picker), the
       as-of read stamp, and Refresh — a CONVENIENCE not a capability, since a
       page reload performs the same read and the error panel keeps its own
       Try again, which is the one place a retry is not optional.
       ACCEPTED IN RETURN: this is the only one of the four pages using
       .pane-head, so /db no longer states when its numbers were read. That was
       the argument for keeping it and it was overruled deliberately; recorded
       here because the next reader will notice the asymmetry. -->

  {#if db.error}
    <!-- LOUD, AND NAMED. The actual failure, and the way to retry it. -->
    <div class="blank fade-in" role="alert">
      <h2>The store manifest could not be read</h2>
      <p class="err">{db.error}</p>
      <p>
        This page shows nothing rather than guessing. Until <code>/store.json</code> answers, no
        count on this page would be trustworthy — including a zero.
      </p>
      <button class="btn primary" onclick={() => refreshStore()}>Try again</button>
      <!-- THE FEED RUNG SURVIVES THE FAILURE, and this is the state that
           most needs it: one feed's store is unreadable and the operator's
           next move is another feed. Without it the only way out of a failed
           read would be the browser's back button. -->
      <section class="strip solo" aria-label="Query">{@render feedRung(db)}</section>
    </div>
  {:else if !feeds.active}
    <!-- AN UNMADE CHOICE IS NOT AN EMPTY STORE, and this branch exists because
         the page could not tell them apart. With no feed selected the fetch
         effect returns before it asks for anything, so `rows` stays empty and
         the next branch down said "Nothing stored for —" and "this feed has
         never landed a bar" — a definite claim about a feed nobody had picked,
         over a store nobody had read. The sentence comes from `blocked`, so it
         cannot drift from what the tiles would say. -->
    <div class="blank fade-in">
      <h2>No feed chosen</h2>
      <p>{db.blocked?.why ?? 'No feed is selected, so no store has been read.'}</p>
      <!-- THE CONTROL THAT ANSWERS THE HEADING, on the panel that states the
           problem. The shortcut buttons below are a different offer — they
           name the feeds that HOLD rows — and neither replaces the other: a
           feed with nothing in it is still a feed an operator may want to
           select before pulling into it. -->
      <section class="strip solo" aria-label="Query">{@render feedRung(db)}</section>
      {#if db.elsewhere.length}
        <div class="alt">
          <span class="lbl">Rows exist under</span>
          {#each db.elsewhere as e (e.feed.wire)}
            <button class="utab" onclick={() => selectFeed(e.feed.wire)}>
              {e.feed.display}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {:else if db.loading && (db.rows.length === 0 || db.preparing)}
    {@render censusLoading(db)}
  {:else if db.rows.length === 0}
    <div class="blank fade-in">
      <h2>Nothing stored for {db.feedName}</h2>
      <p>This feed has never landed a bar in this store.</p>
      <!-- THE RUNG, ON THE PANEL THAT NAMES THE FEED IT IS ABOUT. The
           shortcuts below name only the feeds that HOLD rows; this names
           every feed the server lists, including the empty ones and the
           refused ones with the server's reason on them. -->
      <section class="strip solo" aria-label="Query">{@render feedRung(db)}</section>
      {#if db.elsewhere.length}
        <div class="alt">
          <span class="lbl">Rows exist under</span>
          {#each db.elsewhere as e (e.feed.wire)}
            <button class="utab" onclick={() => selectFeed(e.feed.wire)}>
              {e.feed.display}
            </button>
          {/each}
        </div>
      {/if}
      <a class="cta" href="/ingest">Pull data on Ingest →</a>
    </div>
  {:else}
    <!-- ==================================================================
         THE BOARD. Every band below is a PANEL on a dark field rather than a
         full-bleed row against a hairline, which is the whole difference
         between this and a settings page: the eye finds a group by its edge,
         and the gaps between the groups are what make a dense row inside one
         readable. It is a flex column with `min-height: 0` for exactly one
         reason — the table below is `flex: 1` and windows itself against the
         height it is given, and a column that will not shrink hands it the
         whole document instead.
         ================================================================== -->
    <div class="board rise" bind:this={db.boardEl}>

    <DbControls {db} />

    <DbSummary {db} />

    {@render notesPanel(db)}

      <!-- THE VIEW BAR IS GONE. The Coverage/Bars tabs, the "Showing one row
           per BAR" sentence, the read-budget picker and the "6 read + 3 over
           budget" clause were four pieces of chrome between the rungs and the
           grid, and none of them is a rung or a column. `view` stays pinned to
           `bars`, which is the grid this page is for. -->

    {@render stranded(db)}
    {#if db.view === 'census'}
    <DbCoverageGrid {db} />
    {:else}
    <!-- THE BAR ROWS AND THEIR PAGER NEED ONLY THE SERIES ON SCREEN: paging,
         sorting and Go to never ask for the whole census. `display:
         contents` keeps the board's flex column exactly as it was. -->
    <div data-census-rows style="display: contents">
    <DbBarsGrid {db} />
    </div>
    {/if}

    <div data-census-rows style="display: contents">
    {@render pager(db)}
    </div>
    </div>
  {/if}
</div>


<style>
  /* ---- THE BOARD ------------------------------------------------------
     The dark field the panels sit on. The wash is two very wide radial tints
     mixed OUT OF THE THEME'S OWN ACCENTS rather than typed in — a literal
     colour here would be a second theme the toggle cannot reach, which is what
     `theme.css` lines 20-27 forbid, and it would be the wrong colour in one of
     the two modes by construction. At 5% and 4% it is a gradient the eye reads
     as depth and never as a hue. */
  .board {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    /* THE PAGE SCROLLS HERE, AND UNTIL NOW NOTHING DID.
       ---------------------------------------------------------------------
       `.tbl`'s own comment states the intended design — "past the floor the
       page scrolls, which is the honest outcome". It did not. `body` is
       `overflow: hidden` at a fixed viewport height and `.main` is
       `overflow: hidden` too, so past the floor the content was simply CUT
       OFF, with no scroller anywhere able to reach it.

       MEASURED at a 900px viewport: this column's children run to 983px.
       `.pgbar` sits at 870-936 and `.pager` at 944-983 — the rows-per-page
       control and the page buttons, the two things a reader needs to move
       through 12,470 pages, BELOW THE FOLD AND UNREACHABLE. The only scroller
       on the page was the 201px slot inside the table, so scrolling moved a
       strip and never the page: "sometimes it scrolls, sometimes it is stuck".

       `overflow-y: auto` here makes this column the page's own scroller, which
       is what `.main` clipping it always assumed existed. The table keeps its
       inner scroll for the rows; what changes is that everything BELOW the
       table can now be reached.
       --------------------------------------------------------------------- */
    overflow-y: auto;
    gap: var(--s4);
    padding: var(--s5);
    background-color: var(--bg);
    background-image:
      radial-gradient(
        1100px 620px at 6% 0%,
        color-mix(in srgb, var(--acc) 5%, transparent),
        transparent 60%
      ),
      radial-gradient(
        900px 520px at 96% 0%,
        color-mix(in srgb, var(--info) 4%, transparent),
        transparent 58%
      );
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .strip {
    flex: none;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    grid-template-rows: none;
    gap: var(--s6) var(--s5);
    align-items: start;
    min-width: 0;
    overflow: visible;
    padding: var(--s6);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: linear-gradient(180deg, var(--panel-2), var(--panel));
    box-shadow: var(--e1);
  }
  /* THE ONE-CELL STRIP THE BLANK STATES DRAW. The feed rung is the only
     control on a page that has no table yet, so it is given a width rather
     than the whole panel: `flex: 1 1 0` would stretch one cell across a
     centred empty state, and a lone dropdown 900px wide reads as a mistake. */
  .strip.solo {
    width: max-content;
    max-width: 100%;
    margin: var(--s6) auto 0;
    /* `.blank` CENTRES ITS PROSE AND THIS IS NOT PROSE. Without this the
       caption, the face and the clause would each centre inside the cell,
       which is the one place on this page a control would not line up with
       the control it is the same control as. */
    text-align: left;
  }

  .blank .err {
    font-family: var(--mono);
    font-size: var(--fs-sm);
    color: var(--down);
    background: var(--down-soft);
    border: 1px solid var(--down);
    border-radius: var(--r2);
    padding: var(--s4) var(--s5);
    margin-bottom: var(--s5);
    word-break: break-word;
  }

  code {
    font-family: var(--mono);
    font-size: 0.94em;
    color: var(--ink-2);
  }
</style>
