<script module>
  /**
   * /db — THE ANCHOR
   *
   * What this query is looking at: the instrument, the view tabs, Reset, Export
   * CSV, the compact toggle, the window's facts and the read-budget notice.
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
  import { untrack } from 'svelte';
  import { CSV_COLS, csvQuote, barCsvText, barCsvName } from '$lib/db/export.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { fmt } from '$lib/db/notes-labels.js';
  import { VIEWS } from '$lib/db/notes-controls.js';
  import { paisaText, windowQuery, wireSort, EXPORT_CAP, WINDOW_PAGE_MAX } from '$lib/db/bars-plan.js';
  import { readAllRows } from '$lib/database-pages.js';
  import { optionCells, readSpots, readRungs, spotKey, rungKey } from '$lib/moneyness.js';
  import { rungMatches } from '$lib/db/notes-options.js';
  import { ask } from '$lib/ask.js';
  import { readGreeks, greekCell, greekKey } from '$lib/greeks.js';
  import { dayLabel, barTimeLabel } from '$lib/dates.js';
  import DbBasisBadge from './DbBasisBadge.svelte';

  /**
   * THE ANCHOR'S READOUTS AND THE EXPORT — the window band's facts and their
   * flash, and what Export CSV writes and names.
   *
   * Creates the page's effects that stood at old lines 4285; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function summaryState(db) {
    /* ---------------------------------------------------------------------
       WHICH OF THOSE FILES THE PAGE ON SCREEN ACTUALLY NEEDS.

       `barPlan.read` is every matched instrument-month, and reading all of them
       to draw fifty rows is the waste this exists to remove: one month file at
       one minute holds about 7,875 bars, so a fifty-row page is served by ONE
       file and the other 2,186 are fetched, parsed, and then sliced away.

       THE CENSUS ALREADY KNOWS HOW MANY ROWS EACH FILE HOLDS - `r.rows`, in the
       single `/store.json` response the page has loaded before any of this runs.
       So the file that holds row N is found by running the counts, not by
       opening anything: a prefix sum over integers already in memory.

       WHEN THIS IS EXACT, AND IT IS NOT ALWAYS. A file's position in the plan
       predicts its rows' positions in the table only when the table is in the
       store's own order and nothing removes rows after they are fetched. Two
       things break that and both fall back to reading everything:

         · a sort on any column but `ts` - the store is indexed by TIME, and
           `CLAUDE.md` §4 bans a query planner, so there is no index that could
           answer "the fifty largest closes" without reading the closes. A scan
           is the honest cost of that question and the fallback pays it.
         · a day window - `barWindowed` filters AFTER the fetch, so the census
           count for a month stops predicting how many of its rows survive.

       WHAT IS AND IS NOT O(1) HERE, stated rather than implied. The REQUESTS
       and the SOCKETS are constant: one page needs one file, two when a page
       straddles a boundary, whatever the store holds. The prefix sum itself is
       O(files) integer arithmetic - about 2,187 additions on this store, tens of
       microseconds - which is not constant and does not need to be, because it
       is in memory and the thing it replaces was 2,187 HTTP round trips.
       --------------------------------------------------------------------- */
    /* ---------------------------------------------------------------------
       THE SHAPE OF THE WINDOW, BEFORE A SINGLE NUMBER IS READ.

       `/ingest` shows a reach lane and a coverage column, so an operator sees
       WHAT THEY HOLD before they read anything. This page showed a form and then
       a wall of digits: 699px of chrome above the table on a 960px viewport, so
       four rows were visible and none of them said whether the months either side
       of them existed at all.

       Built from `matched`, which is already in memory off the ONE `/store.json`
       the page loads. No request, no bar file, no new endpoint - the census
       carries `rows` per instrument-month and that is the whole input.

       A MONTH IS SHORT RELATIVE TO ITS OWN NEIGHBOURS, not to a calendar. This
       page cannot know how many sessions a month held without the NSE calendar
       `/ingest` uses, so it compares each month against the FULLEST month in the
       same selection and says so in the legend. A month at less than half the
       fullest is drawn short; that is a ratio this page can defend, and
       "complete" is a claim it cannot make and does not.
       --------------------------------------------------------------------- */
    const coverBand = $derived.by(() => {
      /** @type {Map<string, number>} */
      const perMonth = new Map();
      for (const r of db.matched) {
        const n = Number(r.rows) || 0;
        perMonth.set(r.month, (perMonth.get(r.month) ?? 0) + n);
      }
      if (perMonth.size === 0) return { cells: [], fullest: 0, months: 0, bars: 0 };
      const months = [...perMonth.keys()].sort();
      let fullest = 0;
      let bars = 0;
      for (const n of perMonth.values()) {
        if (n > fullest) fullest = n;
        bars += n;
      }
      const cells = months.map((m) => {
        const n = perMonth.get(m) ?? 0;
        const share = fullest > 0 ? n / fullest : 0;
        return {
          month: m,
          rows: n,
          share,
          /* THREE STATES AND NO FOURTH. `absent` is a month the selection names
             with nothing behind it; `short` is one holding less than half the
             fullest; `held` is everything else. A month is never called COMPLETE
             because this page has no session calendar to check that against. */
          state: n === 0 ? 'absent' : share < 0.5 ? 'short' : 'held'
        };
      });
      return { cells, fullest, months: months.length, bars };
    });

    /* ---------------------------------------------------------------------
       THE FACTS ANNOUNCE THEMSELVES WHEN THEY CHANGE.

       `bx-flash-up` and `bx-flash-down` have been in `theme.css` since it was
       written and the Coverage grid uses them on a month whose bar count moved.
       The Bars view's four hero figures had nothing: change the instrument and
       "6,23,498 bars held" becomes "4,11,114" with no more ceremony than a
       repaint, which on a page that also re-renders fifty rows at the same moment
       is a change nobody sees.

       THE MOTION CARRIES THE FACT, which is the operator's own rule for this
       console: green when the number went UP, red when it went DOWN, and nothing
       at all when it held still. A flash that fired on every render would be
       decoration; one that fires only on a real change is the number telling you
       it is a different number.

       `null` ON THE FIRST PASS, so arriving at the page is not a change. The
       comparison starts once there is something to compare against.
       --------------------------------------------------------------------- */
    /** @type {Map<string, 'up' | 'down'>} */
    let factFlash = $state(new Map());
    /** @type {Map<string, number>} */
    const factSeen = new Map();
    /** Whether `factSeen` yet holds a REAL reading. See the effect below. */
    let factSeeded = false;
    /* THREE FAULTS, AND THE PARAGRAPH ABOVE DESCRIBED A BEHAVIOUR THIS NEVER HAD.
       It claims "`null` ON THE FIRST PASS, so arriving at the page is not a
       change". The first pass is not the first READING — it is MOUNT, when
       `coverBand` is still its empty default, so `factSeen` was seeded with 0
       rather than left unset. The census then landed and `0 -> 6,23,498` counted
       as a change: both hero figures flashed GREEN on every single arrival.

       It repeated on every feed switch, and always green in BOTH directions.
       `rows` collapses to `[]` the moment the selection moves, so a change of
       feed runs `N -> 0 -> M` and the last leg is upward whichever way the store
       actually went. `forgetPreviousFeed` clears `prev`, `barsBefore`, `flash`
       and `moved` — it never knew about `factSeen`, and now does.

       And it fired on TYPING. The effect read `coverBand` reactively, so
       narrowing the filter changed both numbers and lit them as though the store
       had moved. The sibling effect below fixed precisely this and says so —
       "Typing in the filter changes all six numbers and flashes none of them" —
       by depending on `fetchedAt` alone and reading every figure under
       `untrack`. This now does the same.

       `fetchedAt` is already stamped against the live feed
       (`store.lastOk.feed === feeds.active`) and is 0 when there is no good
       reading for it, so it is both the correct dependency and the correct
       guard: no reading, no comparison, no flash. */
    $effect(() => {
      const at = db.fetchedAt;
      if (!at) return;
      /** @type {Map<string, 'up' | 'down'>} */
      const lit = new Map();
      untrack(() => {
        const now = new Map([
          ['bars', coverBand.bars],
          ['months', coverBand.months]
        ]);
        for (const [k, v] of now) {
          const was = factSeen.get(k);
          if (factSeeded && was !== undefined && was !== v) lit.set(k, v > was ? 'up' : 'down');
          factSeen.set(k, v);
        }
        factSeeded = true;
      });
      if (lit.size === 0) return;
      factFlash = lit;
      /* CLEARED SO IT CAN FIRE AGAIN. An animation class that stays on never
         replays, and the next change would land silently — which is the failure
         this exists to fix, arriving by the other road. 900ms is the keyframe's
         own length plus room to finish. */
      const t = setTimeout(() => (factFlash = new Map()), 900);
      return () => clearTimeout(t);
    });

    /**
     * THE NEWEST BAR IN THE SELECTION — counted, and it is the one fact the
     * mockup's removed "newest bar" notice carried that nothing else on this page
     * stated.
     *
     * `/store.json` has sent `last_ts` per row since the census gained it, and
     * until now this page read it only per row (the month cell's tooltip) and per
     * instrument (the drawer's "Days held"). Nothing said how fresh the SET on
     * screen is, which is the question a banner over a table is actually asked.
     * It is in the count line at the foot instead of in a band of its own: a
     * standing notice is a row of chrome, and the same figure beside the row
     * count is read by the same glance.
     *
     * `null` is an absence and not an epoch — a row with no stamp, or no row at
     * all, and the foot names which. Milliseconds, because `deco` already divided
     * the wire's microseconds where the unit was known.
     *
     * @type {number | null}
     */
    const newestBar = $derived(
      db.matched.reduce(
        (/** @type {number | null} */ a, /** @type {any} */ r) =>
          r.lastAt !== null && (a === null || r.lastAt > a) ? r.lastAt : a,
        /** @type {number | null} */ (null)
      )
    );

    /**
     * The file's text: the census view's instrument-months, or the bar view's
     * rows with their option analytics (DB-047, DB-G101).
     * @param {any[]} out
     * @param {(r: any) => any} [context] each bar row's `{ feed, oc, gc }`
     * @returns {string}
     */
    function csvText(out, context = pageContext) {
      if (db.view === 'bars') return barCsvText(out, context);
      const lines = [CSV_COLS.map((c) => csvQuote(c.label)).join(',')];
      for (const r of out) lines.push(CSV_COLS.map((c) => csvQuote(c.v(r))).join(','));
      /* CRLF, AND A TRAILING ONE. RFC 4180 §2. */
      return `${lines.join('\r\n')}\r\n`;
    }

    /**
     * A bar row on screen with the option cells the grid drew for it: the
     * spot and greeks read for THIS PAGE (`spotMap`, `greekMap`), so the file
     * says exactly what the grid says, reasons included.
     * @param {any} r
     */
    function pageContext(r) {
      return {
        feed: feeds.active ?? '',
        oc: r.side ? optionCells(r, db.spotMap.get(spotKey(r)), db.rungMap) : null,
        gc: r.side ? greekCell(r, db.greekMap.get(greekKey(r))) : null
      };
    }

    /** The months the census view's file covers. */
    const csvMonths = $derived(db.view === 'bars' ? [] : db.monthsIn);

    /**
     * What Export writes: the census view's whole matched set, or THE ROWS ON
     * THIS PAGE of the bar view — which is what the name below now says.
     */
    const csvRows = $derived(db.view === 'bars' ? db.barPage : db.sorted);

    /**
     * The file name, from keys only. The bar view's names the rows it holds:
     * the first and last IST day written and their count (DB-047).
     */
    const csvName = $derived(
      db.view === 'bars'
        ? barCsvName(feeds.active ?? '', db.timeframe, db.picked, csvRows)
        : `brutex-db-${db.view}_${feeds.active ?? 'no-feed'}_${db.timeframe || 'all-rungs'}_${csvMonths[0] ?? 'none'}_${csvMonths[csvMonths.length - 1] ?? 'none'}.csv`
    );

    /** How many rows the query keeps that the page file will NOT contain. */
    const csvShortBy = $derived(db.view === 'bars' ? Math.max(0, db.pageTotal - csvRows.length) : 0);

    /** @param {string} text @param {string} name */
    function download(text, name) {
      const url = URL.createObjectURL(new Blob([text], { type: 'text/csv;charset=utf-8' }));
      const a = document.createElement('a');
      a.href = url;
      a.download = name;
      a.click();
      URL.revokeObjectURL(url);
    }

    function exportCsv() {
      /* THE BUTTON IS DISABLED IN THIS STATE AND SAYS WHY; this is the second
         guard, for the keyboard path. A header with no records under it would
         be a file that looks like an answer. */
      if (csvRows.length === 0) return;
      download(csvText(csvRows), csvName);
    }

    /* ---------------------------------------------------------------------
       THE WHOLE QUERY, WRITTEN — BOUNDED (DB-047, D-3326).

       The page export writes the rows on screen. This one writes every row the
       query keeps, in the order on screen, when there are at most `EXPORT_CAP`
       of them: read through the window route `WINDOW_PAGE_MAX` at a time
       (a locally ordered sort already holds them), then the spot and greeks of
       every option row asked of `/spot.json` and `/greeks.json` in each bar's
       own month, exactly as the grid asks for a page. Past the cap it is
       refused by name with the count, before anything is read.
       --------------------------------------------------------------------- */
    /** @type {{ busy: boolean, read: number, why: string }} */
    let exportState = $state({ busy: false, read: 0, why: '' });
    /** Why the whole-query export is not offered, or `''`. */
    const exportRefusal = $derived(
      db.view !== 'bars'
        ? 'the census view exports its whole matched set already'
        : db.pageTotal === 0
          ? 'no bar is kept by this query'
          : db.pageTotal > EXPORT_CAP
            ? `this query keeps ${fmt(db.pageTotal)} bar(s), past the ${fmt(EXPORT_CAP)} one export writes — narrow the dates, the time of day or the contracts`
            : ''
    );

    async function exportQuery() {
      if (exportRefusal || exportState.busy) return;
      const feed = feeds.active ?? '';
      const plan = db.barQueryPlan;
      const bounds = db.barBounds;
      const total = db.pageTotal;
      const signal = new AbortController().signal;
      exportState = { busy: true, read: 0, why: '' };
      try {
        /** @type {any[]} */
        let rows;
        if (db.barState.local) rows = db.barSorted;
        else {
          const wire = wireSort(db.barSortKey) ?? 'ts';
          const got = await readAllRows(
            (offset, limit) => windowQuery(feed, plan, { ...bounds, sort: wire, desc: db.barDesc, offset, limit }),
            total, EXPORT_CAP, WINDOW_PAGE_MAX, signal, undefined,
            (read) => (exportState = { ...exportState, read })
          );
          rows = db.barRowsOf(got.bars, plan);
        }
        /* EACH BAR IN ITS OWN MONTH, AT ITS OWN INSTANT, UNDER THE PAGE'S
           SPOT RULE, and the Moneyness pick applied as the grid applies it
           (WP-53; D-3328, D-3329). */
        const asof = db.spotAsof ?? 'exact';
        let options = rows.filter((r) => r.side);
        const rungs = options.length ? (await readRungs(feed, options, signal, ask, asof)).rungs : new Map();
        if (db.mnyPick.size > 0) {
          rows = rows.filter((r) => Boolean(r.side) && rungMatches(db.mnyPick, rungs.get(rungKey(r))?.rung));
          options = rows;
        }
        const spots = options.length ? await readSpots(feed, options, signal, ask, asof) : new Map();
        const greeks = options.length ? (await readGreeks(feed, options, signal, ask, asof)).cells : new Map();
        const context = (/** @type {any} */ r) => ({
          feed,
          oc: r.side ? optionCells(r, spots.get(spotKey(r)), rungs) : null,
          gc: r.side ? greekCell(r, greeks.get(greekKey(r))) : null
        });
        download(barCsvText(rows, context), barCsvName(feed, db.timeframe, db.picked, rows));
        exportState = { busy: false, read: rows.length, why: '' };
      } catch (why) {
        exportState = { busy: false, read: 0, why: String(/** @type {any} */ (why)?.message ?? why) };
      }
    }

    Object.defineProperties(db, {
      coverBand: { get: () => coverBand },
      factFlash: { get: () => factFlash, set: (v) => (factFlash = v) },
      factSeen: { get: () => factSeen },
      factSeeded: { get: () => factSeeded, set: (v) => (factSeeded = v) },
      newestBar: { get: () => newestBar },
      csvText: { get: () => csvText },
      csvMonths: { get: () => csvMonths },
      csvName: { get: () => csvName },
      csvRows: { get: () => csvRows },
      csvShortBy: { get: () => csvShortBy },
      exportCsv: { get: () => exportCsv },
      exportState: { get: () => exportState },
      exportRefusal: { get: () => exportRefusal },
      exportQuery: { get: () => exportQuery }
    });
  }
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

<!-- ==================================================================
     THE ANCHOR. What this query is looking at, the one headline figure,
     the two presses that are actions rather than filters, and the counted
     line under all three.

     THERE IS NO PRICE HERE AND NO SPARKLINE, and neither is an omission:
     `/store.json` serves this page no price at all — `chg_bps` is a RATIO
     and a ratio has no scale — so a `.px` carrying a traded number would be
     a figure this page invented. What `.px` carries instead is the one
     number the whole strip above exists to move: how many instrument-months
     the selection matched, with its own measured up/down flash, and how
     many the store holds behind it. It is lifted OUT of the counted line
     rather than repeated in it.
     ================================================================== -->
<div class="anchor rise">
  <h1
    class="sym"
    title={db.picked
      ? `${db.picked} — one instrument of the ${fmt(db.instrumentRows.length)} the rungs above leave, in the store's own spelling.${db.censusScopeNote}`
      : `${db.feedName}'s whole store at this selection. No single instrument is named: the Find box holds ${db.typed ? `“${db.filter.trim()}”, which is not one of the ${fmt(db.instrumentRows.length)} instrument keys on offer` : 'nothing'}.`}
  >
    {db.picked || db.feedName}
  </h1>
  <DbBasisBadge {db} />

  <!-- THE COUNT CHIP AND THE "% change — what these are" HELP LINK ARE GONE.
       Neither is a rung and neither is a column: the first restates a number
       the row count under the grid already gives, and the second is help text
       on a page whose columns say what they are. -->

  <!-- THE TWO PRESSES THAT ARE ACTIONS RATHER THAN FILTERS. The count is ON
       the button: "% change — what these are" invites a click nobody makes,
       and "2,206 refused" is the fact an operator wants explained the moment
       they see a column of dashes. -->
  <span class="actions">
    <!-- ==============================================================
         THE SECOND VIEW WAS UNREACHABLE, AND THE COMMENT ABOVE `view`
         SAID IT WAS "ONE CLICK AWAY".

         `view` was declared `$state('bars')` and never assigned again --
         the only writes to it were gone, so `VIEWS` carried two entries,
         every `view === 'census'` branch was dead markup, and the coverage
         survey was reachable by editing the source. Its own title says
         what that cost: it "is the only view that can survey the whole
         store for a hole", and the page holding a hole survey nobody can
         open is worse than not having one, because the code reads as
         though the capability is there.

         THE CONTROL IS NOT INVENTED HERE. `.segs`, `.seg`,
         `.seg[aria-selected='true']` and `.seg .c` were all still in the
         stylesheet below, orphaned -- the compiler flagged three of them as
         unused selectors, which is how the original shape was recovered
         rather than guessed. It is a TABLIST with a count on each tab, and
         that CSS's own comment says why: the clause that used to sit under
         the control said how many segments were held, and the tabs are that
         enumeration. Restoring the markup the styles were written for also
         clears the three warnings.
         ============================================================== -->
    <span class="segs" role="tablist" aria-label="View">
      <!-- THE COUNT COMES FROM THE STORE, NOT FROM WHAT IS LOADED.
           It read `barSorted.length`, which is the FETCHED rows -- and
           leaving the Bars view runs `barState = { files: [] }` by design,
           because a view nobody is looking at should not hold 58,305 rows
           in memory. The side effect was that the tab read `Bars 0` while
           standing on Coverage: the one moment the number exists to be
           read, since a count on a tab is there to say what is on the OTHER
           side.
           `0` is not a formatting slip, it is a measurement claim, and it
           was false -- the same `null`-versus-zero rule this page keeps
           everywhere else. `total` is `matched.reduce((a, r) => a + r.rows)`
           off /store.json, which carries `rows` per instrument-month, so the
           honest number is available without opening a single bar file.
           THE PAGER STILL READS `barSorted.length`, and must: it can only
           page through rows that are actually loaded. -->
      {#each VIEWS as v (v.key)}
        <button
          class="seg"
          type="button"
          role="tab"
          aria-selected={db.view === v.key}
          tabindex={db.view === v.key ? 0 : -1}
          title={v.title}
          onclick={() => (db.view = v.key)}
          >{v.label}<span
            class="c"
            title={v.key === 'bars'
              ? db.windowSaid
                ? `${fmt(db.pageTotal)} bar(s) this query keeps — the server's count over every matched month and series, after the date and time bounds; the same number the pager counts.`
                : `${fmt(db.total)} bar(s) stored in the matched months, by the census; the server has not answered for this query yet.`
              : `${fmt(db.sorted.length)} instrument-month(s) matched.`}
            >{fmt(v.key === 'bars' ? (db.windowSaid ? db.pageTotal : db.total) : db.sorted.length)}</span
          ></button
        >
      {/each}
    </span>
    {#if db.filtered}
      <button class="btn ghost sm" type="button" onclick={db.reset}>Reset all filters</button>
    {/if}
    <!-- EXPORT WRITES THE MATCHED SET, NOT THE WINDOW. Only ~30 rows exist
         in the DOM at any moment; the file is every row the selection
         matched, in the order the table is sorted in.

         DISABLED WITH THE REASON ON ITS FACE rather than sitting inert: a
         primary press that does nothing and says nothing is the silent
         failure `CLAUDE.md` §4 names, and "nothing matched" and "no feed
         chosen" are different sentences that a grey button cannot tell
         apart on its own. -->
    <button
      class="btn ghost sm"
      type="button"
      disabled={db.csvRows.length === 0}
      title={db.csvRows.length === 0
        ? db.view === 'bars'
          ? 'Nothing to export — no bar has been read into this grid, so there are no records to write.'
          : `Nothing to export — ${db.blocked?.why ?? 'no instrument-month matches this selection'}`
        : db.view === 'bars'
          ? `Write the ${fmt(db.csvRows.length)} bar(s) ON THIS PAGE as CSV, named ${db.csvName} for the rows it holds.${
              db.csvShortBy > 0
                ? ` The query keeps ${fmt(db.pageTotal)} bar(s); ${fmt(db.csvShortBy)} of them are on other pages — Export query writes them all, up to its cap.`
                : ` That is every bar this query keeps.`
            } Columns follow the grid: the feed first, then the bar, its change and open interest with their reasons, the contract, and for an option the spot, moneyness, intrinsic, extrinsic, IV and greeks the grid shows. Every cell is the wire form — microsecond stamps, prices and strikes in paisa, percentages as integer basis points, an EMPTY field for an unknown beside its reason, never a zero. Nothing is written into this repository.`
          : `Write the ${fmt(db.csvRows.length)} matched instrument-month(s) as CSV, named ${db.csvName}. Built in the browser from rows already in memory: no request is made, and nothing is written into this repository — the bytes go to your downloads. Every cell is the store's own wire form — raw YYYY-MM months, raw ISO expiries, strikes in paisa, percentages as the integer basis points they arrived as — so the file joins against the store. Only the header row is words.`}
      onclick={db.exportCsv}>{db.view === 'bars' ? 'Export page' : 'Export CSV'}</button
    >
    {#if db.view === 'bars'}
      <!-- THE WHOLE QUERY, BOUNDED (DB-047): every row the query keeps, in the
           order on screen, with the option analytics, at most EXPORT_CAP rows
           and refused by name past it. -->
      <button
        class="btn ghost sm"
        type="button"
        disabled={Boolean(db.exportRefusal) || db.exportState.busy}
        title={db.exportRefusal
          ? `Not offered: ${db.exportRefusal}.`
          : `Write all ${fmt(db.pageTotal)} bar(s) this query keeps, in the order on screen, with the same columns as Export page — read ${fmt(1000)} at a time through the server's window, then each option bar's spot and greeks in its own month.`}
        onclick={db.exportQuery}
        >{db.exportState.busy ? `Exporting ${fmt(db.exportState.read)} of ${fmt(db.pageTotal)}…` : 'Export query'}</button
      >
      {#if db.exportState.why}<span class="xwhy" role="status">Export refused: {db.exportState.why}</span>{/if}
    {/if}
    <!-- THE FOLD, AND THE BUTTON CARRIES ITS OWN PRICE. `Fit` gives the
         grid exactly the rows the window can show and cannot give it more;
         past that, a row costs forty pixels of chrome and the only chrome
         a reader can spend is the part that DISPLAYS rather than asks. So
         the label says what it is worth — "+4 rows" — measured from the two
         panels as they stand, not written here. A toggle whose effect you
         have to try in order to learn is a toggle nobody presses twice. -->
    {#if db.view === 'bars' && db.foldRows > 0}
      <button
        class="btn ghost sm"
        type="button"
        aria-pressed={db.compact}
        title={db.compact
          ? db.autoFolded
            ? `Folded by the page, not by you: this window could not hold the readouts AND the grid, so it was overflowing by a screen-fraction — long enough to scroll, too short to be worth scrolling, which reads as shaking. Press to bring them back and accept that scroll. Costs ${fmt(db.foldRows)} row(s) of grid here.`
            : `Bring back the counted line and the window band. Costs ${fmt(db.foldRows)} row(s) of grid at this window size.`
          : `Fold the counted line and the window band away and give the grid the space — worth ${fmt(db.foldRows)} more row(s) at this window size. Both are readouts; nothing that narrows the selection is hidden.`}
        onclick={() => {
          db.compact = !db.compact;
          /* THE PRESS ENDS THE AUTOMATIC BEHAVIOUR FOR GOOD — see
             `readerChose`. Without it, unfolding on a short window restores
             the panels, the page overflows, and the rule folds them back in
             the same frame: a button that visibly refuses the press. */
          db.readerChose = true;
          db.autoFolded = false;
        }}
        >{db.compact ? 'Show readouts' : 'Compact'}{#if db.foldRows > 0}<span class="c"
            >{db.compact ? '−' : '+'}{fmt(db.foldRows)}</span
          >{/if}</button
      >
    {/if}
  </span>

  <!-- THE LAST TWO READOUTS MOVE ONTO THIS LINE, AND THE ROW UNDER IT GOES.
       MEASURED: the counted line was 62px tall and held two items, each
       stretched to 787px — the range hard against the left edge and the
       close hard against the right, with half a screen of nothing between
       them. Two facts cannot fill a row that wide, and stretching them to
       try is what made it read as two stranded things rather than one
       readout. The anchor had 852px of unused width on the same line as
       the instrument and its tabs.
       SO THEY SIT WHERE THEY ARE READ FROM. The range describes the
       selection the tabs above count; the close is the newest row of the
       grid below. Both belong beside them, not in a band of their own. -->
  {#if db.view === 'bars' && db.coverBand.cells.length > 0}
    <span class="afacts">
      <span class="af">
        <b>{db.coverBand.cells[0].month}</b><span class="arw">→</span><b
          >{db.coverBand.cells[db.coverBand.cells.length - 1].month}</b
        >
        <i>on disk</i>
      </span>
      {#if db.newestClose && 'bar' in db.newestClose}
        <!-- THE SELECTION'S NEWEST STORED CLOSE, NOT THE PAGE'S FIRST ROW
             (DB-007). One seek over every series and month of the selection,
             asked once per selection; the date and time bounds do not move it,
             as they do not move the "on disk" range beside it. -->
        {@const nb = db.newestClose.bar}
        {@const at = db.dayRung ? dayLabel(nb.t * 1000) : `${dayLabel(nb.t * 1000)} ${barTimeLabel(nb.t * 1000, db.rungSecs)}`}
        {@const tail = (db.barQueryAll?.keys.length ?? 0) > 1 ? db.newestClose.key.split('-').slice(-2).join(' ') : ''}
        <span
          class="af"
          title="The newest stored bar of this selection: {at}{db.dayRung ? '' : ' IST'}, {db.newestClose.key}. Read once for the selection, over every series and month — not the first row of the page."
        >
          <b class="num" data-dir={nb.c > nb.o ? 'up' : nb.c < nb.o ? 'down' : 'flat'}>{paisaText(nb.c)}</b>
          <i>newest close · {at}{tail ? ` · ${tail}` : ''}</i>
        </span>
      {/if}
    </span>
  {/if}

  <!-- ==============================================================
       THE COUNTED LINE, AND IT IS WHAT THE SIX TILES WERE. Bars,
       complete, bars missing, coverage and months — the same figures, the
       same sub-clauses, the same `{#key}`-driven flash on each of them, in
       one line under the anchor instead of six boxes above the fold. The
       sixth, instrument-months, is the `.px` above: promoted, not dropped.

       A COUNTED ZERO STAYS A ZERO. The store WAS read, so "0
       instrument-months match this filter" is a measurement and printing a
       dash for it would hide a fact the page holds. What may never be
       printed is a figure nothing produced — a ratio with no denominator,
       or a session count at a rung nobody has a session size for. Those
       render the em dash and `blocked.tsub` names which absence it is.
       ============================================================== -->
  <!-- ==================================================================
       THE NUMBERS ARE THE HEADLINE; THE CLAUSES ARE ON HOVER.

       Every figure here carried a trailing `.u` clause, and read end to end
       the line came out as three hundred and nineteen characters of running
       prose: "BARS 8,470 · 20 session-equivalents COMPLETE 0 of 9 · 0
       short · 9 not comparable BARS MISSING 0 · 0 whole sessions, vs the
       fullest instrument each month COVERAGE — every row here is the only
       one stored at its (month, rung), so each is its own denominator and
       there is no ratio MONTHS 1 Aug 2026 → Aug 2026 · 0 with holes".

       Five numbers matter and they were buried in it. `terse` hides the
       clauses and keeps every one of them on the title, so the qualifier is
       one hover away rather than in the way of the figure it qualifies.
       Nothing is deleted — the markup, the counts and the reasons are all
       still built; only their default visibility changed.
       ================================================================== -->
  <!-- THE STATS LINE IS GONE. BARS / COMPLETE / BARS MISSING / COVERAGE /
       MONTHS was a fifth band of numbers about the numbers below it. The
       row count under the grid is the one figure a reader needs, and the
       grid itself is the rest. -->

  <span class="sr-only" aria-live="polite"
    >{db.viewNow.showing}. {#if db.view === 'bars' && db.barState.error}This page of bars could not be read: {db.barState.error}{:else}{fmt(db.pageTotal)} row(s) matched, showing {fmt(db.pageFrom)} to {fmt(
      db.pageTo
    )} on page {fmt(db.pageNow)} of {fmt(db.pageCount)}.{/if}</span
  >

  <!-- THE READ-BUDGET NOTICE IS GONE: the grid reads one page per request over
       every matched month (D-3326), so no month is held back to be named. -->
</div>

<style>
  /* ====================================================================
     THE CONTROL VOCABULARY — `.segs`, `.strip`, `.lead`, `.cell`, `.picker`,
     `.mnyb`, `.menu`, `.opt`, `.anchor`, `.sym`, `.px`, `.note`, `.pager`.

     WHAT THIS BLOCK REPLACES: `.sel`, `.pickers`, `.pk`, `.plbl`, `.pknote`,
     `.dd`, `.ddb`, `.ddm`, `.ddr`, `.ddnone`, `.dates`, `.dlbl`, `.ask` and
     `.foot` — /ingest's control panel, copied wholesale into /db. That copy is
     the mistake being undone. The two pages ask the same SHAPE of question and
     were told to answer it in their own voices; one vocabulary across both is
     how an edit to either silently becomes an edit to both, and it is why a
     grid of 248px boxes ended up standing in for a row of strip cells.

     NOTHING BELOW IS A LITERAL. Every colour, size, radius and space is a
     `theme.css` token, so both themes are one definition and the page has no
     second theme system the toggle cannot reach. The three tokens the old
     panel used to alias locally — `--raise`, `--panel2`, `--accdeep` — are
     still NOT aliased here: `theme.css` defines them, which is what lets
     `$lib/Picker.svelte` theme its own menus, and a local copy is the copy
     that goes stale.
     ==================================================================== */

  /* ---- THE SEGMENT TABLIST -------------------------------------------
     A tablist, sized to its own content, above what it filters. It scrolls
     rather than wraps: three tabs never need it, and a fourth segment landing
     in the store must not push the strip below it down a row. */
  .segs {
    flex: none;
    display: flex;
    gap: var(--s2);
    width: max-content;
    max-width: 100%;
    padding: var(--s1);
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r3);
    overflow-x: auto;
  }
  .seg {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    font-size: var(--fs-base);
    font-weight: var(--w-semi);
    padding: var(--s3) var(--s6);
    border-radius: var(--r2);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: var(--s4);
    white-space: nowrap;
  }
  .seg:hover {
    color: var(--ink);
  }
  .seg:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .seg[aria-selected='true'] {
    color: var(--on-acc);
    background: var(--acc);
  }
  /* COUNTED, PER SEGMENT, AND IT IS THE WHOLE REASON THIS IS A TABLIST AND NOT
     A ROW OF WORDS. The clause that used to sit under the control said how many
     segments were held; the tabs are that enumeration, and each one carries its
     own total. */
  .seg .c {
    font-family: var(--mono);
    font-size: var(--fs-mini);
    font-variant-numeric: tabular-nums;
    color: var(--faint);
    background: var(--panel-2);
    padding: 1px var(--s4);
    border-radius: var(--r-full);
  }
  .seg[aria-selected='true'] .c {
    background: color-mix(in srgb, var(--ink-hi) 22%, transparent);
    color: var(--on-acc);
  }
  /* ---- THE ANCHOR -----------------------------------------------------
     What the query is looking at, the one headline figure, the two presses that
     are actions rather than filters, and the counted line under all three.

     `.px` CARRIES NO PRICE, AND THAT IS A FINDING RATHER THAN A GAP.
     `/store.json` serves this page no price at all — `chg_bps` is a RATIO, and a
     ratio has no scale — so a traded number here would be one this page
     invented. What it carries instead is the number the whole strip exists to
     move, with its own measured up/down flash. */
  .anchor {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    padding: 0 var(--s2);
  }
  /* THE TWO READOUTS THAT WERE A ROW. `margin-left: auto` puts them at the far
     end of the anchor rather than in a band of their own — the 852px this line
     was already carrying empty. They do NOT stretch: a fact is as wide as its
     figure, and the 787px each was given in the old row is what made two
     readouts look like two unrelated things at opposite ends of the page. */
  .afacts {
    margin-left: auto;
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    min-width: 0;
  }
  .af {
    display: inline-flex;
    align-items: baseline;
    gap: var(--s2);
    white-space: nowrap;
  }
  .af b {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-weight: var(--w-semi);
    color: var(--ink);
  }
  /* THE CLOSE IS THE ONE PRICE OUTSIDE THE GRID, so it takes the same up/down
     hues the grid's own closes take — one meaning, one colour, wherever it is
     drawn. */
  .af b.num[data-dir='up'] {
    color: var(--up);
  }
  .af b.num[data-dir='down'] {
    color: var(--down);
  }
  /* The label is the quiet half: what the figure IS, never competing with it. */
  .af i {
    font-style: normal;
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .af .arw {
    color: var(--faint);
    padding: 0 var(--s1);
  }
  .sym {
    margin: 0;
    min-width: 0;
    font-family: var(--mono);
    font-size: var(--fs-xl);
    font-weight: var(--w-heavy);
    letter-spacing: -0.03em;
    color: var(--ink-hi);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* THE INSTRUMENT IS THE PAGE'S SUBJECT AND NOW LOOKS LIKE IT. A gradient
     across the word rather than a flat colour — it is the largest type on the
     page and the only place a graphic treatment costs nothing, because no
     figure is being compared to it.

     `@supports` AND `color` DECLARED BEFORE IT, deliberately. `color:
     transparent` with `background-clip: text` is the whole trick and it is also
     the whole risk: where the clip is unsupported, `transparent` text on a
     transparent background is an INVISIBLE HEADING — the page would lose the
     name of what it is showing and nothing would report it. The flat
     `--ink-hi` above stands on its own, and only a browser that has already
     proven it can clip to the text ever sets `transparent`. */
  @supports (background-clip: text) or (-webkit-background-clip: text) {
    .sym {
      background: linear-gradient(
        96deg,
        var(--ink-hi) 34%,
        color-mix(in srgb, var(--acc) 82%, var(--ink-hi))
      );
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
    }
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--s4);
    flex-wrap: wrap;
  }
  .btn.sm {
    padding: var(--s2) var(--s4);
    font-size: var(--fs-xs);
  }
  .xwhy {
    font-size: var(--fs-xs);
    color: var(--warn);
  }
</style>
