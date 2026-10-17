<script module>
  /**
   * /db — THE QUERY STRIP AND THE CONTRACT STRIP
   *
   * Every rung the operator sets: feed (the `feedRung` snippet, also drawn by the
   * shell in every state with no strip), universe, instrument, segment,
   * timeframe, the day and time window, expiry, strike and side — plus the
   * stranded-rung notice (`stranded`).
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
   * Owner after WP-00: WP-50 (selection model, controls and URL state).
   */
  import Picker from '$lib/Picker.svelte';
  import {
    UNIVERSES,
    SEG_VIEW,
    everythingWhy,
    instrumentListWhy,
    tierLeavesOutText,
    shortfallText,
    wholeTitle
  } from '$lib/db/notes-controls.js';
  import { fmt, tfNote, tfCmp } from '$lib/db/notes-labels.js';
  import DayField from '$lib/DayField.svelte';
  import { dayLabel } from '$lib/dates.js';
  import { strikeText } from '$lib/db/decoration.js';
  import { feeds, selectFeed } from '$lib/feeds.svelte.js';
  import { feedPickerRow, offeredFeeds, excludedSentence } from '$lib/feed-row.js';
  import { SECONDS } from '$lib/db/selection-model.js';
  import { survey } from '$lib/store.svelte.js';
  import DbSearch from './DbSearch.svelte';
  import DbMoneyness from './DbMoneyness.svelte';

  export { feedRung, stranded };
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();

  /**
   * A DAY'S FLAG FOR THE DAY FIELDS' GRID (DB-025): the day band's, inside the
   * window it read, else the month's own `/dayflags.json` read when a grid
   * showed that month — so a refused or incomplete day is marked in any month
   * the calendar shows, not only inside the chosen window.
   * @param {string} iso
   */
  const dayMark = (iso) => db.dayFlagged?.(iso) ?? db.monthFlagged(iso);
</script>

<!-- ==================================================================
     THE QUERY STRIP.

     WHAT STOOD HERE AND DOES NOT ANY MORE: a row of six summary tiles, a
     universe chip rail with a prose paragraph under it, a band of month
     cards, and — most recently — a `.sel` panel of `.pk` rungs copied out
     of /ingest. That copy was the mistake being undone: the owner asked for
     /ingest to have its OWN design, and two pages wearing one vocabulary is
     how a change to either becomes a change to both.

     EVERY RUNG IS NOW A `.cell` IN A `.strip`, which is the vocabulary the
     approved /db design speaks: a `.lead` naming the strip, a caption, the
     control, and one clipped `.count` clause under it carrying what the
     control cannot fit. Nothing the deleted blocks stated is lost — every
     figure the tiles carried is in the anchor's counted line, every
     sentence the paragraphs carried is the clause under the control it is
     about, with the whole of it on that control's `title`.

     READING ORDER IS THE CASCADE: feed → universe → month window → month →
     instrument → text → holes, then the contract strip below. Every control
     is authored after the one that narrows it, so tab order follows the
     markup for free.
     ================================================================== -->
<!-- THE `QUERY` LEAD IS GONE AND THE `aria-label` CARRIES IT.
     Every control under it is labelled — BROKER FEED, UNIVERSE, INSTRUMENT,
     SEGMENT, TIMEFRAME, and the dates and times — so the word named a group
     a reader had already identified from its contents, and spent a line of
     the fold doing it. The section is still announced as "Query" to
     anything reading the document rather than looking at it, which is the
     half of the label that was carrying information.
     `Contract` KEEPS ITS LEAD, and the difference is not inconsistency:
     that strip is USUALLY EMPTY, and its lead is what says a named section
     exists at all before there is a control in it to imply one. -->
<section class="strip rise" aria-label="Query">

  <!-- THE FEED IS THE PAGE'S SCOPE AND IT IS CHOSEN HERE — the strip's
       first cell, because every rung after it is this feed's answer. One
       control for one value, on this route and in this bar: the shell's
       `FEED_OWNED` list carries `/db`, so the top bar draws no picker
       here. See the snippet at the foot of this file for why it is a
       snippet and not markup in place. -->
  {@render feedRung(db)}

  <!-- THE UNIVERSE RUNG — the cascade's second step. `tierRefusal` is
       unchanged and still whole on the row it belongs to, and a refusal is
       DRAWN AND DISABLED rather than hidden. -->
  <div class="field">
    <span class="lab" title="Which membership to hold this store against — the cascade's second rung."
      >Universe</span
    >
    <!-- THE SAME SEARCHABLE `Picker` AS EVERY OTHER RUNG. It was a
         hand-rolled menu with no filter box; with 200+ NIFTY Total Market
         members behind some universes, a list you can only scroll is a list
         you cannot use. `single`, like everything on this page. -->
    <Picker
      single
      filter
      label="universes"
      summary={db.chosenUniverse.label}
      rows={UNIVERSES.map((u) => ({
        key: u.key,
        name: u.label,
        detail: db.censusScope
          /* ONE INSTRUMENT WAS READ, NOT THE STORE (WP-56b): a count of names
             held from its rows alone would be a statement about the store. */
          ? `only ${db.censusScope.underlying} read so far; counted when the whole census is read`
          : u.key
          ? db.universeCount.get(u.key)
            ? `${fmt(db.universeCount.get(u.key).held)} of ${fmt(db.universeCount.get(u.key).members)} held`
            : 'not counted'
          : /* NAMES, THE UNIT EVERY OTHER TIER COUNTS (WP-56; DB-N07): it read
               "2,50,025 held" — instrument-months — beside "184 of 200 held"
               instruments. The month count stays, under its own name. */
            db.heldNames === null
            ? `${fmt(db.deco.length)} instrument-month(s); names not yet counted`
            : `${fmt(db.heldNames)} held · ${fmt(db.deco.length)} instrument-month(s)`,
        /* TRUE OF THIS FEED (DB-041): GDFL has no master to "no longer list" a name. */
        why: u.key ? undefined : everythingWhy(db.noMaster, db.feedName)
      }))}
      selected={new Set([db.universe])}
      onchange={(/** @type {Set<string>} */ sel) => (db.universe = [...sel][0] ?? '')}
    />
    <!-- THE COUNTED CLAUSE THAT WAS A PARAGRAPH. Every figure comes from the
         rows and the master on hand, and the one sentence that is not a
         figure is a refusal naming why a figure is absent. It clips to one
         line; the whole of it is on the button's `title` above and in this
         node for anything that reads the document rather than looks at it.
         `CLAUDE.md` §4: degrade loudly and name the reason. -->
    <!-- ══ THE COUNTED CLAUSE GOES; THE TWO SENTENCES THAT ARE NOT COUNTS
         STAY ══
         `N held · no membership list` drew on every load under a button
         reading `Everything`, and the picked-universe branch counted members
         held against members known — both facts, neither of them news, and
         both now on the control's `title`.
         WHAT STAYS IS WHAT A COUNT CANNOT SAY. `universeRefusal` is a
         refusal, and `outsideMaster` names rows that ARE held and that no
         membership universe can reach — an operator who does not read that
         sentence concludes the store is smaller than it is. Both draw only
         when true.
         THE `{#if}` IS OUTSIDE THE SPAN, not inside it. A `.note` carries
         its marker as `::before`, so an emptied one is not invisible — it is
         a bare `·` under a control. /ingest learned this the hard way. -->
    <!-- THE STORED NAMES A TIER LEAVES OUT ARE NAMED (DB-041), and the whole
         note is its own hover text (`wholeTitle`): it clips at two lines. -->
    {#if db.universeRefusal || db.noMaster || (db.outsideMaster && db.outsideMaster.months > 0) || db.leftOut?.count > 0}
      <span class="note" class:warn={Boolean(db.universeRefusal)} use:wholeTitle>
        {#if db.universeRefusal}
          {db.chosenUniverse.label} is not narrowing this table — {db.universeRefusal}
        {/if}
        <!-- A FEED WITH NO MASTER SAYS SO (DB-041). GDFL publishes none; its
             list is its census, with membership from core::universe's own
             lists, so the tiers join and no row claims "in this master". -->
        {#if db.noMaster}
          {db.feedName} has no instrument master: the list is its census (stored, no master) and the
          tiers are core::universe's own membership lists.
        {/if}
        {#if db.outsideMaster && db.outsideMaster.months > 0}
          {fmt(db.outsideMaster.months)} stored instrument-month(s) across {fmt(
            db.outsideMaster.instruments
          )} underlying(s) are not listed by {db.feedName}'s instrument list — they are held, no
          membership universe can reach them, and Everything is where they show.
        {/if}
        {#if db.leftOut?.count > 0}
          {tierLeavesOutText(db.chosenUniverse.label, db.leftOut, fmt)}
        {/if}
      </span>
    {/if}
  </div>

  <!-- THE INSTRUMENT RUNG. It WRITES THE TEXT BOX beside it and owns no
       state of its own — a row is ticked exactly when the box holds that
       row's key, so typing over a pick unticks it with nothing to
       synchronise and the two can never disagree.

       HIDDEN, NOT DISABLED, WHEN THERE IS NOTHING TO ENUMERATE. Its parent
       rungs can leave it with no members at all; an empty dropdown is a
       control offering a choice that does not exist. The text box stays
       either way — it is the `/` target and it can reach a month as well as
       an instrument.

       ONE `Picker`, THE SHARED COMPONENT, unchanged: it draws its own
       `.picker` root, its own filter box and its own tick per row, so this
       rung cannot drift from the same control on /markets and /ingest. -->
  <!-- GATED ON WHAT IT ACTUALLY RENDERS. This read `instrumentRows` — the
       STORE — while the `Picker` below folds `instrumentOffered`, which is
       the MASTER whenever a universe is chosen. A universe whose members
       this feed's store holds none of therefore removed the entire rung
       from the DOM, taking with it every name `instrumentOffered` was
       rewritten to expose. Its own header states the intent that gate
       defeated: "a rung that can only name what you already have cannot
       tell you what you are missing."

       It compounds with the seeding effect below, which can seat `filter`
       on an unheld member — and the search box that used to be the other
       way out was deleted, so the only escape left was the Universe rung.

       The `title` said "Held, not offered", which was the pre-rewrite
       promise and is now false for the rows underneath it. Corrected here
       rather than left to contradict them. -->
  {#if db.instrumentOffered.length > 0}
    <div class="field">
      <span
        class="lab"
        title={instrumentListWhy(db.noMaster, db.feedName)}
        >Instrument</span
      >
      <Picker
        single
        filter
        label="instruments"
        summary={db.filter
          ? (db.instrumentOffered.find((/** @type {any} */ r) => r.key === db.filter)?.sym ?? db.filter)
          : `${fmt(db.instrumentOffered.length)} offered`}
        rows={[
          /* NO "ALL INSTRUMENTS" ROW, for the reason `All rungs` went from
             Timeframe: this rung is `single`, so "all of them" is not a
             member of the set it offers. One name is chosen at all times and
             the head row was the only way to leave it unchosen. */
          ...db.instrumentOffered.map((/** @type {any} */ r) => ({
            key: r.key,
            name: r.sym,
            /* BACKWARD-LOOKING, ALWAYS. "held" is what /store.json can
               prove; what the vendor could serve is /ingest's question and
               no endpoint this page calls can answer it. */
            /* THE KIND, WHICH IS WHAT /ingest PUTS HERE — `equity`, `index`.
               The held count is a fact about THIS page and belongs on the
               row's title beside it, not in the column /ingest fills with
               the instrument's type. */
            /* WHAT IT IS AND WHAT IS HELD OF IT (DB-041). "in this master"
               was printed for every GDFL row, which has no master; an
               option-only underlying (ACC, DHFL) says so instead of reading
               like a spot series, and the counts are split by segment —
               "11,668 month(s) held, 4,84,12,39,862 short" summed a 1s index
               denominator over option contract-months. */
            detail:
              r.months === 0
                ? 'nothing stored'
                : r.spotMonths === 0
                  ? 'contracts only'
                  : r.kind || (r.contractMonths > 0 ? 'spot + contracts' : 'spot'),
            title:
              r.months === 0
                ? `${r.sym} — ${r.key}. Nothing stored for it at this selection.`
                : `${r.sym} — ${r.key}. ${fmt(r.spotMonths)} spot month(s)${shortfallText(r.shortBy, tfCmp, fmt)} and ${fmt(r.contractMonths)} contract-month(s) held.${r.master === null && db.noMaster ? ` ${db.feedName} has no instrument master; this row is its census.` : ''}`
          }))
        ]}
        selected={new Set([db.filter])}
        onchange={(/** @type {Set<string>} */ sel) => (db.filter = [...sel][0] ?? '')}
      />
      <!-- RESTATEMENT, BOTH WAYS. `1 instrument(s) held here` under a
           button reading `All · 1 held`, and the picked branch reprinted the
           name the button already shows. Neither fact is lost: the `h1.sym`
           below already carries both on its own title, in fuller words. -->
    </div>
  {/if}

  <!-- THE SEARCH BOX IS GONE. It was the instrument rung's text input, not a
       rung of its own, and the Instrument picker beside it has its own filter
       box — two search fields for one question. Both wrote `filter`, so the
       picker keeps the state and `typed`/`textMatched` are unchanged; what is
       lost is the `/` shortcut and prefix matching against a MONTH, which the
       month rung no longer exists to use. -->
  <DbSearch {db} />

<!-- ==================================================================
     THE SEGMENT TABLIST — `.segs`, AND IT IS ABOVE WHAT IT FILTERS.

     A tablist belongs over the set it narrows, which is where the approved
     design puts it, so it is the first thing under the pane head and ahead
     of the query strip. The FEED still outranks it: this whole branch only
     renders once a store has answered, so there is never a tab here
     counting a feed nobody chose.

     IT WAS A `Picker`, AND WHAT IT LOST IS ITS OWN RESTATEMENT. The clause
     under it read "INDEX · CASH · FNO — showing INDEX" and "3 segment(s)
     held": the enumeration and the count of an enumeration that is now
     drawn, one tab per segment, with each tab's own counted total on its
     face. A line naming what the row beside it already draws is the page
     repeating a control it has already drawn.

     "SEGMENT" IS THE STORE'S OWN SECOND FIELD — INDEX, CASH, FNO — and NOT
     the mockup's spot/futures/options taxonomy. Relabelling it would rename
     three sets into three other sets that do not have the same members.
     ================================================================== -->
  <!-- ALWAYS DRAWN, DISABLED WHEN THERE IS NOTHING TO CHOOSE.
       It was `{#if kinds.length > 1}`, on the reasoning that one segment is
       not a choice between segments. True, and it is the wrong conclusion
       twice over. /ingest draws Segments unconditionally — three rows, two
       of them refusing with their reason on their face — so a reader
       comparing the two strips found a rung on one page and a gap on the
       other. And it is this file's own rule besides: a rung that cannot be
       used is DRAWN AND DISABLED, never hidden, because "why is there no
       segment control" is exactly the question `CLAUDE.md` §4 says must be
       answered out loud rather than by an absence. The single segment the
       store holds is now stated rather than implied. -->
  <!-- NOT DISABLED, AND /ingest IS THE PRECEDENT. Its segments menu carries
       no `disabled` at all: it draws all three rows, `tuck`s the two it
       cannot serve, and lets the operator open them. This cell greyed itself
       out whenever the store held fewer than two segments — which is the
       ordinary case — so the control that answers "what segments are in
       here" was dead exactly when somebody would ask it. A live control
       holding one row answers the question; a grey one refuses it. -->
  <div class="field">
    <span class="lab" title="The store's own second field — INDEX, CASH, FNO.">Segment</span>
    <!-- A PICKER, NOT A TAB ROW. It was the only rung drawn as tabs, which
         made it look like a different kind of thing from the eight around
         it; it writes the same `kind` the cascade reads. Two or three
         options today, and the filter box costs nothing — nine rungs that
         behave nine ways is nine things to learn. -->
    <!-- THE FACE COUNTS SEGMENTS, NOT ROWS. Every other rung's "All ·" names
         how many OPTIONS its menu holds — 1 instrument, 9 rungs — and this
         one printed `textMatched.length`, the row count, so it read
         "All · 9" beside a store that holds exactly one segment. -->
    <Picker
      single
      filter
      label="segments"
      title={`${db.filter || 'This selection'} holds ${fmt(db.instrumentMatched.length)} instrument-month(s) in ${db.feedName}'s store, under ${fmt(db.kinds.length)} of the store's segments (${db.kinds.map((/** @type {any} */ k) => `${k[0]} ${fmt(k[1])}`).join(', ') || 'none'}). Spot is every series with no expiry; a contract is expired, settled after the archive ended, or unexpired, by its expiry against today's IST day (${db.today}).${db.censusScopeNote}`}
      summary={db.kind
        ? (SEG_VIEW.find((s) => s.key === db.kind)?.name ?? db.kind)
        : `${fmt(SEG_VIEW.length)} offered`}
      rows={db.segmentRows}
      selected={new Set([db.kind])}
      onchange={(/** @type {Set<string>} */ sel) => (db.kind = [...sel][0] ?? '')}
    />
    <!-- BOTH BRANCHES RESTATED THE FACE. `INDEX — the only segment this
         store holds` under a button reading `All · 1`, and it wrapped to
         two lines, so this was the one cell that made the strip's row
         heights ragged. On the control's `title` now. The cell already
         carries `.off` when there is nothing to choose, which is the
         visible refusal. -->
  </div>

  <!-- THE BAR-LENGTH RUNG — the cascade's third step, and the control the
       approved design draws as `<select id="tf">` beside the others.

       IT IS A `.picker` AND NOT A `<select>`, like every other rung in
       this strip. The design's `<select>` is shorthand for "a dropdown
       goes here"; a native one cannot carry a per-row count, cannot carry
       a per-row `title`, and cannot be searched — and the counted row is
       the whole reason this page's menus exist.

       EVERY RUNG THE STORE HOLDS IS OFFERED AND NO OTHER. The bar-length
       ladder in `TF_SECS` is an ORDERING, not a list of choices: offering
       `5min` because the ladder names it, on a store that holds no
       five-minute bar, would be a control promising a set that does not
       exist. The rows are counted off the rows. -->
  <!-- ALWAYS DRAWN, DISABLED WHEN EMPTY — the same correction as Segment
       above, for the same two reasons. `tfRefusal` already existed to say
       WHY there is no rung to pick, and gating the whole cell on
       `tfAll.length > 0` deleted the control that was carrying that
       sentence, so the one state the refusal was written for was the one
       state the reader never saw it in. The `Picker` is already
       `disabled={Boolean(tfRefusal)}`. -->
  <div class="field" class:off={Boolean(db.tfRefusal)} title={db.tfRefusal ?? undefined}>
    <span class="lab" title="Bar length — the rung each row is stored at.">Timeframe</span>
    <!-- SEARCHABLE, LIKE THE REST. Nine rungs today and ten in
         `store::path::Timeframe::KNOWN`; the list is short now and the
         filter costs nothing, and consistency across the nine rungs is
         worth more than saving a filter box on one of them. -->
    <Picker
      single
      filter
      label="rungs"
      disabled={Boolean(db.tfRefusal)}
      title={db.tfRefusal
        ? `No rung stored — ${db.tfRefusal}`
        : `${db.timeframe ? `${fmt(db.timeframed.length)} of ${fmt(db.universed.length)} instrument-month(s) at ${db.timeframe}. ${tfNote(db.timeframe)}` : `${fmt(db.tfAll.length)} rung(s) held, ${fmt(db.segmented.length)} instrument-month(s) across them`} Only rungs this store actually holds are listed — the list is counted off the rows, never off a fixed ladder.${db.censusScopeNote}`}
      summary={db.tfRefusal ? 'No rung stored' : db.timeframe || 'Pick a rung'}
      rows={[
        /* NO "ALL RUNGS" ROW. A table mixing rungs is a table whose columns
           stop meaning one thing: this file already records that "a month
           holding both 1min and 1day rows has no single session size", so
           `All · 9` produced a session count, a bars-per-session and a
           change-% that belonged to no rung in particular. Exactly one rung
           is selected at all times — seeded from the store by the effect
           beside `stranded`. */
        /* WITH THE MONTHS HELD OUTSIDE THE WINDOW SAID ON EACH ROW (D-3338;
           R3-02), and a rung held only outside it listed and refused. */
        ...db.timeframeRows
      ]}
      selected={new Set([db.timeframe])}
      onchange={(/** @type {Set<string>} */ sel) => (db.timeframe = [...sel][0] ?? '')}
    />
    <!-- ══ THE COUNTS GO; THE REFUSAL STAYS ══
         `1 rung(s) held` drew under a button reading `All · 1`, and the
         picked branch counted rows at a rung the button already names. Both
         are on the control's `title` now — the same cut /ingest made to its
         five captions, for the same reason.
         `tfRefusal` is NOT a count and does not move: it is the §4 case and
         draws only when there is a refusal to name. The `{#if}` is OUTSIDE
         the span, because a `.note` carries its marker as `::before` and an
         emptied one paints a bare `·` under the control. -->
    {#if db.tfRefusal}
      <span class="note warn" use:wholeTitle>No bar length — {db.tfRefusal}</span>
    {/if}
  </div>
  <!-- ==================================================================
       THE DAY WINDOW — FOR SPOT AND FOR A CONTRACT (DB-046, DB-G89).
       It said "spot only"; the server has always narrowed a contract's bars
       to the chosen days, and now the words say so. The fields open on the
       span the census says the selection holds and are bounded by it (or by
       the feed's era before anything is counted); closed days are struck
       from the calendar of record (DB-044, DB-045, DB-N04).
       ================================================================== -->
  <!-- THE TWO ENDS ARE ONE RUNG, AND THE RUNG TAKES THE WHOLE ROW.
       As two independent grid items they were placed like any other pair,
       so at five tracks From landed in column 5 and To wrapped to column 1
       of the next row — the two ends of one window at opposite corners of
       the panel, with three unrelated rungs between them. /ingest draws
       the identical control and answers this with `.pk.wide2`: the day
       window is `grid-column: 1 / -1` and holds both ends in a `.dates`
       flex row, capped so a pair of date fields cannot spread across
       1,200px and read as the panel's most important control when it is
       its last one. Same rule, same reason, same numbers. -->
  <div class="field wide">
    <!-- "DAYS TO SHOW" IS GONE AND ONLY THE HEADING WAS — the same cut
         /ingest made to "Days to pull". It stacked a group label over two
         fields that already carry their own, FROM DATE and TO DATE, and no
         other control on this strip has that extra level.
         The sentence it held moves to `.dates`, the element that IS the
         window, rather than being deleted with the heading that carried
         it. -->
    <!-- THE PRODUCT'S OWN CALENDAR, AND THIS FILE HAS DEMANDED ONE IN
         WRITING FOR LONGER THAN IT HAD ONE. Twenty lines below, the month
         window's comment reads "The product's OWN calendar, never the
         platform's ... There is not one on this page and there must never
         be" — and directly above it sat two `<input type="date">`.

         `<input type="date">` renders in the OS locale, so `02/09/2024` is
         the 2nd of September to one reader and the 9th of February to
         another, and no rule on this page can reach its text or its popup
         to say which. `dd Mon yyyy` cannot be misread in any locale, and it
         is what `$lib/dates.js` already renders every other date in this
         product as — so until now the DISPLAYED day and the EDITABLE day
         disagreed on the same screen.

         `$lib/DayField.svelte` is that control and it was written for
         exactly this: its own header names /db's platform input as the
         defect it exists to remove. It had no importer anywhere in the
         tree. One component, both pages, and the two can no longer drift —
         the same argument `Picker` settles for every other rung here. -->
    <div
      class="dates"
      title={`The window in days, IST. It narrows a spot series and a contract alike: a contract shows only its bars on the chosen days, within its life. ${db.dayReason}`}
    >
      <!-- THE HELD SPAN IS THE BOUND AND THE OTHER BOX IS THE ORDER (DB-045).
           `min`/`max` were the other box's day, so a To before From was
           refused in the held span's words, and the grid struck days its
           sentence never named. Each is now its own bound with its own words;
           an emptied box takes the held span's end, so a To box always holds
           a day; each month a grid shows is read for its flags. -->
      <DayField
        label="From date"
        value={db.fromDay}
        min={db.dayBounds2.lo}
        max={db.dayBounds2.hi}
        before={db.toDay}
        beforeLabel="To date"
        empty={db.dayBounds2.lo}
        closed={db.dayClosed}
        flagged={dayMark}
        onshow={db.showMonth}
        flagWhy={db.monthFlagWhy}
        onchange={(/** @type {string} */ d) => (db.fromDay = d)}
        boundsReason={`${db.dayReason} Only bars on or after this day.`}
      />
      <DayField
        label="To date"
        value={db.toDay}
        min={db.dayBounds2.lo}
        max={db.dayBounds2.hi}
        after={db.fromDay}
        afterLabel="From date"
        empty={db.dayBounds2.hi}
        closed={db.dayClosed}
        flagged={dayMark}
        onshow={db.showMonth}
        flagWhy={db.monthFlagWhy}
        onchange={(/** @type {string} */ d) => (db.toDay = d)}
        boundsReason={`${db.dayReason} Only bars on or before this day.`}
      />

      <!-- THE MINUTE, WHICH THE DATE FIELDS CANNOT REACH.
           A date at `1min` is 375 rows, so "27 Aug at 14:32" meant setting
           both dates to that day and paging sixty-three times at six rows
           a page. The grid always had the minute in a column; there was no
           way to ASK for one.

           ABSENT ON A DAILY RUNG rather than disabled. Every bar at `1day`
           opens at the same minute, so a time bound there keeps everything
           or empties the grid — and the Time column is already hidden. A
           control whose only two settings are "all" and "none" is not a
           control, which is the rule the Contract strip above follows. -->
      {#if !db.dayRung}
        <!-- THE THREE TIME CONTROLS TAKE THEIR OWN LINE, TOGETHER.
             `DayField` is `flex: 1 1 190px` and GROWS, so with all five in
             one wrapping row the two dates ate the line and the break fell
             between FROM TIME and TO TIME — a pair split across rows, which
             reads as two unrelated controls. `flex-basis: 100%` puts the
             break where the MEANING is: days on one line, minutes on the
             next, and the trio can never be separated from each other. -->
        <!-- PICKED, NOT TYPED — the same control as every other rung.
             These were three text boxes wanting `HH:MM`, which asked a
             reader to KNOW a minute before they could look at one, and to
             type it correctly. Every other control on this strip is a
             `Picker`; there was no reason these were not, and two
             consequences followed from that alone. The list can only offer
             minutes the loaded rows actually hold, so a choice cannot land
             on nothing. And `.pbtn` is already 48px here — measured, the
             same as the day fields beside it — so the alignment comes for
             free rather than being re-derived. -->
        <!-- THE RUNG'S SESSION GRID, NOT THE ROWS ON SCREEN (DB-074). The
             lists offered only the minutes of the fifty loaded rows — "15:29"
             and nothing else on a 1s page — so 09:15 could not be asked for
             without paging. They now list every minute a bar of this rung can
             open on, 09:15 to 15:29 (charter §3, pull::session), plus any
             pre-open or closing-auction minute the loaded rows hold. On the
             1s rung a bound carries its second (HH:MM:SS): the minute picks
             :00 for From and :59 for To, and the second beside it narrows.
             AND EVERY MINUTE THE DAYS IN VIEW TRADED (R3-02): a special
             session's windows (a Muhurat hour, a DR Saturday) come from the
             calendar of record's own list, and a window of special days only
             lists no regular minute — `sessionRows` in selection.svelte.js. -->
        <div class="times">
          <div class="tcell">
            <span class="tlbl" id="lab-tfrom">From time</span>
            <Picker
              single
              filter
              label="minutes"
              disabled={db.sessionRows.length === 0}
              summary={db.fromTime || 'Any'}
              rows={[{ key: '', name: 'Any', detail: 'no bound' }, ...db.sessionRows]}
              selected={new Set([db.fromTime.slice(0, 5)])}
              onchange={(/** @type {Set<string>} */ s) =>
                db.setTimeBound('from', [...s][0] ?? '', db.fromTime.length === 8 ? db.fromTime.slice(6, 8) : null)}
            />
          </div>
          {#if db.secondRung && db.fromTime}
            <div class="tcell sec">
              <span class="tlbl">Second</span>
              <Picker
                single
                filter
                label="seconds"
                summary={`:${db.fromTime.slice(6, 8) || '00'}`}
                rows={SECONDS.map((x) => ({ key: x, name: `:${x}` }))}
                selected={new Set([db.fromTime.slice(6, 8)])}
                onchange={(/** @type {Set<string>} */ s) => db.setTimeBound('from', db.fromTime.slice(0, 5), [...s][0] ?? '00')}
              />
            </div>
          {/if}
          <div class="tcell">
            <span class="tlbl" id="lab-tto">To time</span>
            <Picker
              single
              filter
              label="minutes"
              disabled={db.sessionRows.length === 0}
              summary={db.toTime || 'Any'}
              rows={[{ key: '', name: 'Any', detail: 'no bound' }, ...db.sessionRows]}
              selected={new Set([db.toTime.slice(0, 5)])}
              onchange={(/** @type {Set<string>} */ s) =>
                db.setTimeBound('to', [...s][0] ?? '', db.toTime.length === 8 ? db.toTime.slice(6, 8) : null)}
            />
          </div>
          {#if db.secondRung && db.toTime}
            <div class="tcell sec">
              <span class="tlbl">Second</span>
              <Picker
                single
                filter
                label="seconds"
                summary={`:${db.toTime.slice(6, 8) || '59'}`}
                rows={SECONDS.map((x) => ({ key: x, name: `:${x}` }))}
                selected={new Set([db.toTime.slice(6, 8)])}
                onchange={(/** @type {Set<string>} */ s) => db.setTimeBound('to', db.toTime.slice(0, 5), [...s][0] ?? '59')}
              />
            </div>
          {/if}
          <div class="tcell gocell">
            <span class="tlbl" id="lab-goto">Go to</span>
            <!-- THE SAME GRID. A minute the loaded rows do not hold lands on the
                 nearest loaded bar and the line under the strip says how far
                 away it is — never a silent jump. -->
            <Picker
              single
              filter
              label="minutes"
              disabled={db.sessionRows.length === 0}
              summary={db.goTime || 'Pick a minute'}
              rows={db.sessionRows}
              selected={new Set([db.goTime])}
              onchange={(/** @type {Set<string>} */ s) => {
                const at = [...s][0] ?? '';
                db.goTime = at;
                if (at) db.jumpToTime(at);
              }}
            />
          </div>
        </div>
      {/if}
    </div>
    <!-- THE BOUND'S OWN REACH, SAID BEFORE IT MISLEADS. A time filter over
         a server-paged read can only see the page in memory, and the
         visible result of that is an empty grid — which reads as "the
         store has no bars there" when it means "this read never fetched
         there". `CLAUDE.md` §4: degrade loudly and name the reason. -->
    {#if db.timePartial}
      <p class="jwhy warn" role="status">
        This read was paged by the server, so the time bound searched only
        the <b>{fmt(db.barRows.length)}</b> row(s) it returned — not the
        {fmt(db.windowSaid?.total ?? 0)} the query matches. Narrow the dates until
        the days are read whole and the bound covers every minute in them.
      </p>
    {/if}
    {#if db.jumpWhy}
      <p class="jwhy" role="status">{db.jumpWhy}</p>
    {/if}
    <!-- MONTHS OUTSIDE THE OPERATOR'S WINDOW (D-3338): hidden by default,
         counted, and one press away — labelled "outside your window" when
         shown. Nothing is deleted. -->
    {#if db.oow || db.oowHere > 0}
      <p class="jwhy" role="status">
        {#if db.oow}
          Showing months <b>outside your window</b> ({db.activeFeed?.window?.from} to
          {db.activeFeed?.window?.to}): {fmt(db.oowHere)} of this selection's, {fmt(db.oowMonths)}
          instrument-month(s) across {db.censusScope ? `${db.censusScope.underlying}, the only instrument read so far` : 'the feed'}.
          <button class="btn ghost sm" type="button" onclick={() => (db.oow = false)}>Hide them</button>
        {:else}
          {fmt(db.oowHere)} of this selection's months ({fmt(db.oowHidden)} instrument-month(s) across
          {db.censusScope ? `${db.censusScope.underlying}, the only instrument read so far` : 'the feed'}) lie outside your window ({db.activeFeed?.window?.from} to
          {db.activeFeed?.window?.to}); they are held and hidden.
          <button class="btn ghost sm" type="button" onclick={() => (db.oow = true)}>Show them, labelled</button>
        {/if}
      </p>
    {/if}
    {#if db.linkNotes.length > 0}
      <p class="jwhy warn" role="status">
        The link asked for something this page could not read: {db.linkNotes.join('; ')}.
      </p>
    {/if}
    {#if db.urlWhy}
      <p class="jwhy warn" role="status">This selection is not in the address bar: {db.urlWhy}.</p>
    {/if}
  </div>
  



  <!-- THE MONTH WINDOW — two ends, so it is TWO `.cell`s, exactly as the
       design draws a From and a To. The product's OWN calendar, never the
       platform's: `<input type="date">` renders in the OS locale, so
       `02/09/2024` is two different days depending on who is reading it,
       and no rule in this file can reach either its text or its popup.
       There is not one on this page and there must never be.

       IT WORKS IN WHOLE MONTHS, and that is the unit of the data rather
       than a simplification — a row here IS an instrument-month.

       IT RENDERS ONLY WHEN THERE IS A WINDOW TO CHOOSE. One month is not a
       range, and a control whose every setting produces the same table is a
       control that has nothing to say. -->
  <!-- THE FROM/TO RANGE IS GONE. ONE MONTH CONTROL, SINGLE SELECT.
       ==================================================================
       MONTH was secretly TWO controls: this from/to range, and the single
       month picker below. A range is a selection of many months, which is
       a multi-select — and every control on this page is single select.
       Two controls for one rung also meant the rung was applied at two
       different depths of the cascade, which is the one thing the prefix
       rule cannot express.

       `fromMonth` and `toMonth` stay declared and stay empty, so
       `windowed` is `timeframed` by definition and the cascade collapses
       to one month step. The calendar snippet and its handlers are dead
       and are removed in the commit that follows this one, separately,
       because deleting ~250 lines across thirty-three sites in the same
       edit as a behaviour change is how a page stops rendering with
       nothing to point at.
       ================================================================== -->

  <!-- MONTH IS NOT ONE OF THE RUNGS.

       The nine are feed, universe, instrument, segment, timeframe,
       expiry, strike, moneyness, side. Month was never among them, and it
       was the most confused rung on the page besides: two controls for one
       thing, a from/to range that was a multi-select on a single-select
       page, and a single-month picker beneath it.

       `month` stays declared and permanently empty, so `scoped` is
       `windowed` by definition and the cascade runs feed -> universe ->
       instrument -> segment -> timeframe and straight on to the contract
       rungs. Every month the selection reaches is in the table; the row
       count and the pager are what say how many. -->



  <!-- THE HOLES-ONLY TOGGLE IS GONE. It was a tenth rung on a page whose
       rungs are nine, and it filtered on a property of the DATA rather
       than on an identity, which is not what this strip is for.
       `holesOnly` stays declared and permanently false, so
       `matchedPreContract` is `scoped` by definition. -->

</section>

<!-- ==================================================================
     THE CONTRACT STRIP. Authored after the query strip because a strike is
     finer than a segment and a rung is finer than a strike.

     IT IS ALWAYS OPEN AND ITS CELLS GO `.off` INSTEAD. A strip that
     collapsed when there is no chain would take the REFUSAL with it, and
     "why is there no strike control" is exactly the question `CLAUDE.md` §4
     says must be answered out loud rather than by an absence.

     BOTH PANELS ARE `Picker` — one renderer, two data sources. The filter
     box, the checkbox on every row, Select-all-shown and the right-aligned
     detail column all come from it and cannot drift away from the other
     pages.

     THE REASON IS ON THE CELL AND ALSO UNDER IT, and the second is the one
     that matters: `Picker` takes no `title`, and a refusal that lives only
     on a tooltip is loud to a pointer and silent to everything else.
     `undefined` rather than `''` so a rung with nothing to explain carries
     no attribute at all.
     ================================================================== -->
<!-- ==================================================================
     DRAWN ONLY WHEN A CONTRACT EXISTS TO CHOOSE.

     This strip used to render unconditionally, so a store holding nothing
     but indices and equities — which is every store this build can
     produce — showed FOUR controls reading "No contract stored", "No
     strike stored", "No ladder" and "No option stored", above a paragraph
     explaining why each was dead. Four dead controls and their obituary,
     on every page load, for a rung that cannot exist here.

     `CLAUDE.md` §4 says degrade LOUDLY and name the reason, and that is
     why this was built that way. But §4 is about a FAILURE being hidden.
     "This store has no futures" is not a failure — it is the normal and
     permanent state of a spot-index store, and `expiryRefusal`'s own text
     says so: nothing in this build can fetch a contract at all. Drawing
     four disabled controls to announce a non-event is not loudness, it is
     noise, and it pushed the actual query controls off the first screen.

     The reason is not lost. The moment `expiriesAll` holds anything the
     strip returns with every refusal it ever had, and the one-line summary
     under the table still counts what is stored.

     ------------------------------------------------------------------
     THE GATE ABOVE IS WITHDRAWN, ON THE OWNER'S INSTRUCTION, AND THE
     ARGUMENT FOR IT IS LEFT STANDING BECAUSE IT WAS NOT A BAD ONE.

     `{#if !expiryRefusal}` collapsed this whole strip whenever no contract
     could be reached — which, on today's wire, is always. The reasoning
     above is sound as far as it goes: four permanently dead controls
     announcing a non-event is noise, not loudness, and the strip did cost
     the query rungs their place on the first screen.

     Two things overrule it.

     The owner asked for these four by name and in order — feed, universe,
     instruments, segments, timeframes, expiry, strike, moneyness, side —
     and asked for each to carry the same view, the same dropdown and the
     same search as every rung above it. A control that is absent has no
     view to be the same as. That is a decision about what this page IS,
     and it is the owner's to make, not this file's.

     And the consistency argument runs the other way now. /ingest draws its
     two contract segments unconditionally, both refusing, both with the
     reason on their face; the two pages are meant to read as one product.
     The cost the comment measured is also mostly gone: the strip is a
     wrapping grid since D-0153, so four rungs are one row, not four.

     What made this cheap to reverse is that nothing was deleted when it
     was hidden. Every cell still carries its own `.off` and its own
     refusal text, exactly as the `.strip.sub` rule below still describes
     ("it is ALWAYS DRAWN ... a strip that collapsed would take the two
     REFUSALS with it") — a comment that had been true of the CSS and false
     of the markup ever since the gate went in. It is true of both again.
     ================================================================== -->
<!-- ══ THE WHOLE STRIP FOLDS WHEN THERE IS NO CONTRACT TO NARROW ══

     The comment above argues this strip must be DRAWN because "/ingest
     draws its two contract segments unconditionally, both refusing, both
     with the reason on their face; the two pages are meant to read as one
     product." That reasoning is right and its premise is out of date:
     /ingest passes `tuck` to both its universe and its segments menus, so
     the rows it cannot serve sit behind ONE counted line reading "2 rows
     this feed cannot serve — show why". Folding here is what restores the
     consistency that comment was reaching for, not what breaks it.

     AND NOTHING IS HIDDEN, which is the distinction §4 turns on. The
     section is not gated away: it is drawn, its count is on the line, its
     reason is on the line, and one click opens all four cells with every
     refusal intact. That is `Picker`'s own `tuck` argument applied to a
     strip — "a list whose first screenful is entirely unusable teaches the
     operator no more than a hidden row does. It teaches less: it reads as a
     broken build."

     WHY IT IS EMPTY IS A FACT ABOUT THE BUILD, NOT ABOUT THE STORE, and
     `expiryRefusal` already says so in full: POST /pull/fno answers 503,
     "expired F&O has no local-archive path and no HTTP transport in this
     build", and every FUT/CE/PE master row is declined as
     `Skip::LiveContract`. No pull an operator can start will populate these
     four controls, which is exactly why four of them standing open with
     eight lines of amber text was reading as a broken page.

     OPEN WHENEVER THERE IS SOMETHING TO CHOOSE. `open={!expiryRefusal}` —
     the fold is the exception, not the default, so a store that does hold
     contracts is unchanged from before this commit. -->
<!-- ══ ALWAYS DRAWN. WHAT VARIES IS WHICH RUNGS ARE IN IT ══
     It folded behind one line, and the fold carried its OWN `Contract` lead
     while the section inside kept another — so the page printed the word
     twice, which read as a stray empty box beside the heading.
     It does not fold now. The section is always open and each rung is drawn
     only when the chosen segment actually HAS it. That is better than the
     fold: a control that does not exist for a thing is not the same as a
     control disabled for it. See `contractRungs`. -->
<!-- `bare` WHEN THERE IS NOTHING TO CHOOSE. With Spot selected this strip
     has a heading, one sentence, and no controls at all — and it still took
     181px of a 960px viewport (measured) to explain that there was nothing
     to configure. The sentence stays and says the same thing; only the
     panel around it stops behaving like a panel of rungs. -->
<!-- THE STRIP DOES NOT RENDER WITHOUT RUNGS, AND THE LEAD GOES WITH THE
     SENTENCE THAT STOOD IN FOR THEM.
     It drew on every load: a `CONTRACT` heading over one line saying spot
     has no expiry, strike, ladder or side — a whole strip of the fold
     spent announcing that a section is empty, in the state this page opens
     in, because spot is the default.
     THIS DEPARTS FROM §4 AND IS THE OPERATOR'S CALL, RECORDED HERE.
     "A rung that cannot be used is drawn and disabled, never hidden — 'why
     is there no segment control' is a question an absence answers badly"
     is the rule this file states and follows everywhere else, and the
     removed sentence was that rule's answer for this strip. Nothing
     replaces it: picking Expired futures or Expired options in SEGMENT
     above brings the rungs back, and a reader who has not is no longer
     told why they are missing.
     `aria-label` stays for the state where it does render. -->
{#if db.contractRungs.expiry}
  <section class="strip sub rise" aria-label="Contract">

  <!-- THE EXPIRY, AND IT IS THE STRIP'S FIRST CELL BECAUSE IT IS ITS
       COARSEST RUNG: a strike belongs to a contract, and a contract is an
       expiry. The approved design draws it first for the same reason.

       IT IS THE ONE CONTROL HERE THAT REFUSES BEFORE IT IS TOUCHED. The
       owner's rule is that a future or an option without the chosen expiry
       shows nothing, so `''` holds every contract row back rather than
       passing them all. The clause under it is therefore never decoration:
       it is the count of what is being withheld and the reason, which is
       the whole of `CLAUDE.md` §4 on one line.

       DISABLED ONLY WHEN THERE IS NOTHING TO CHOOSE, and it says which of
       two things that is on its own face — an unread store, or a store
       that holds no contract. A dead control with no reason on it is the
       thing §4 forbids.

       THE REASON IS SAID THREE TIMES AND THAT IS NOT REDUNDANCY. A
       DISABLED button does not fire pointer events in every browser, so
       its own `title` is the one tooltip that may never appear — the cell
       carries it too, and the clause under the control carries it in the
       document where anything that reads rather than looks will find it.
       `undefined` rather than `''` so a rung with nothing to explain
       carries no attribute at all. -->
  {#if db.contractRungs.expiry}
    <div class="field" class:off={Boolean(db.expiryRefusal)} title={db.expiryRefusal ?? undefined}>
      <span class="lab" title="Which contract — futures and options show nothing without one.">Expiry</span>
      <!-- THE SAME `Picker` THE OTHER THREE USE, so this control is
           searchable like them. It was 77 lines of hand-rolled dropdown with
           no filter box, which is why an expiry could only be found by
           scrolling. `single` because every control on this page is single
           select; `filter` because that is the whole point of the change. -->
      <Picker
        single
        filter
        label="expiries"
        disabled={Boolean(db.expiryRefusal)}
        summary={db.expiryRefusal
          ? 'No contract stored'
          : db.expiry
            ? dayLabel(db.expiry)
            : `None chosen · ${fmt(db.expiryOffered.length)} here`}
        rows={[
          {
            /* THE UNCHOSEN ROW IS DRAWN AND IT DOES NOT SAY "ALL". It is the
               state that HOLDS CONTRACTS BACK — a bar belongs to one
               contract, and two expiries under one heading would be a series
               no contract ever traded. */
            key: '',
            name: 'None chosen',
            detail: `${fmt(db.contractHere)} contract row(s) held back`,
            why:
              db.contractHere > 0
                ? 'This is a refusal, not an empty store. Indices and equities carry no expiry and are unaffected — they are in the table below already.'
                : undefined
          },
          /* WEEKLY OR MONTHLY, THE WEEKDAY, AND ANY SHIFT (DB-076), each
             measured from this underlying's stored expiries and the calendar
             of record (`expiryTags`) — never a written weekday rule. */
          ...db.expiryOffered.map((/** @type {any} */ d) => {
            const t = db.expiryTagMap.get(d);
            return {
              key: d,
              name: dayLabel(d),
              detail: `${t ? `${t.weekday} · ${t.cadence} · ` : ''}${fmt(db.expiryCount.get(d) ?? 0)} instrument-month(s)`,
              why: t?.shift ? `${t.shift.from ? 'Shifted' : 'Off its neighbours'}: ${t.shift.why}.` : undefined,
              title: `Contracts expiring ${dayLabel(d)}${t ? `, a ${t.weekday}, the ${t.cadence} expiry (${t.cadence === 'monthly' ? 'the last stored expiry of its month' : 'not the last stored expiry of its month'})` : ''} — the store's own key for it is ${d}.`
            };
          })
        ]}
        selected={new Set(db.expiry ? [db.expiry] : [])}
        onchange={(/** @type {Set<string>} */ sel) => (db.expiry = [...sel][0] ?? '')}
      />
      <span class="note" class:warn={Boolean(db.expiryRefusal) || db.expiryHeld > 0} use:wholeTitle>
        {#if db.expiryRefusal}
          No contract — {db.expiryRefusal}
        {:else if db.expiry}
          {fmt(db.expiryCount.get(db.expiry) ?? 0)} row(s) on this contract{#if db.expiryHeld > 0}
            · {fmt(db.expiryHeld)} on another expiry are not shown{/if}{#if db.censusScope?.read.some((/** @type {any} */ part) => part.expiry !== null)}
            · the other expiries are not read yet{/if}
        {:else if db.expiryHeld > 0}
          {fmt(db.expiryHeld)} future/option row(s) HELD BACK until an expiry is chosen — a bar
          belongs to one contract, so nothing here blends two
        {:else}
          {fmt(db.expiriesAll.length)} expiry/expiries stored · no contract row is in this selection,
          so nothing is being held back
        {/if}
      </span>
      {#if db.lifeNote}
        <span class="note warn" title={db.lifeNote}>{db.lifeNote}</span>
      {/if}
    </div>
  {/if}

  {#if db.contractRungs.strike}
    <div class="field mcell" class:off={Boolean(db.strikeRefusal)} title={db.strikeRefusal ?? undefined}>
      <span class="lab" title="An absolute price on the chain.">Strike</span>
      <!-- MULTI-SELECT (DB-G98): 11,500 and 11,600 can be picked together.
           `strikePick` was always a Set and `matched` always filtered by one;
           only `single` stood in the way. -->
      <Picker
        filter
        label="strikes"
        disabled={db.chainAll.length === 0}
        summary={db.strikePick.size > 1
          ? `${fmt(db.strikePick.size)} strikes: ${[...db.strikePick]
              .map(Number)
              .sort((/** @type {number} */ a, /** @type {number} */ b) => a - b)
              .map((/** @type {number} */ p) => strikeText(p))
              .join(', ')}`
          : db.strikePick.size === 1
            ? db.strikeSummary
            : `Any strike · ${fmt(db.strikesHere.list.length)} here`}
        rows={db.strikeRows}
        selected={db.strikePick}
        onchange={(/** @type {Set<string>} */ sel) => (db.strikePick = new Set(sel))}
      />
      <span class="note" class:warn={Boolean(db.strikeRefusal)} use:wholeTitle>
        {#if db.strikeRefusal}
          No strike chain — {db.strikeRefusal}
        {:else if db.strikesHere.list.length === 0}
          no strike in this selection{#if !db.expiry && db.expiryHeld > 0}
            — choose an expiry and its chain is listed{/if}
        {:else}
          <!-- THE STRIKES THIS SELECTION REACHES — this underlying, this
               expiry — not the whole store's 1,211 strikes on a 0.05 grid. -->
          {fmt(db.strikesHere.list.length)} strike(s) · {strikeText(db.strikesHere.list[0])} to {strikeText(
            db.strikesHere.list[db.strikesHere.list.length - 1]
          )}{#if db.strikesHere.step !== null}
            · grid {strikeText(db.strikesHere.step)} apart{/if}<!--
            WHY EVERY ROW IN THE PANEL READS `no bars` WHILE THE GATE IS
            SHUT. The counts here are taken over `expiryGated`, so with no
            expiry chosen they are all zero — correctly, because that is what
            clicking one returns. Left unsaid, a panel of zeroes over a chain
            the store demonstrably holds reads as a broken count.
            -->{#if !db.expiry && db.expiryHeld > 0}
            · every count here is zero until an expiry is chosen — {fmt(db.expiryHeld)} contract
            row(s) are held back{/if}
        {/if}
      </span>
    </div>
  {/if}

  <DbMoneyness {db} />

  <!-- OPTION TYPE — the strip's last and finest cell, where the approved
       design draws it. Two values the store can hold and one that means no
       join: `CE`, `PE`, and `Both`, which is `''`.

       `Both` IS NOT "EVERY OPTION". It is the absence of a side filter, so
       it leaves every index, equity and future in the table — the same
       distinction the strike panel's empty Set carries, and the reason
       picking `CE` legitimately drops everything that is not a call.

       DISABLED ONLY ON A FACT ABOUT THE STORE — no option in it at all —
       and NEVER on a fact about the selection. Greying it because the
       current rungs left no option would be a filter with no way back: pick
       `CE`, narrow to a month with no calls, and the one control that could
       undo it is the one that just went dead. "This selection reaches no
       option" is said instead, on the clause, with the control still open
       and the expiry gate named as the usual cause. -->
  {#if db.contractRungs.side}
    <div
      class="field"
      class:off={Boolean(db.sideRefusal || db.sideEmpty)}
      title={db.sideRefusal ?? db.sideEmpty ?? undefined}
    >
      <span class="lab" title="Which side of the contract — calls or puts.">Side</span>
      <!-- SEARCHABLE AND SINGLE, LIKE EVERY OTHER RUNG. Two options today,
           and the consistency is the point: nine rungs that behave nine ways
           is nine things to learn. -->
      <Picker
        single
        filter
        label="sides"
        disabled={Boolean(db.sideRefusal)}
        summary={db.sideRefusal ? 'No option stored' : db.side ? db.side : `Both \u00b7 ${fmt(db.sideOffered.length)}`}
        rows={[
          { key: '', name: 'Both', detail: `${fmt(db.mnyFilteredCount)} row(s)` },
          ...db.sideOffered.map((/** @type {any} */ s) => ({
            key: s,
            name: s,
            detail: `${fmt(db.sideCount.get(s) ?? 0)} row(s)`,
            title: s === 'CE' ? 'Calls.' : 'Puts.'
          }))
        ]}
        selected={new Set([db.side])}
        onchange={(/** @type {Set<string>} */ sel) => (db.side = [...sel][0] ?? '')}
      />
      <span class="note" class:warn={Boolean(db.sideRefusal || db.sideEmpty)} use:wholeTitle>
        {#if db.sideRefusal}
          No side — {db.sideRefusal}
        {:else if db.sideEmpty}
          No option here — {db.sideEmpty}
        {:else if db.side}
          {fmt(db.sideCount.get(db.side) ?? 0)} of {fmt(db.sidedHere)} option row(s) here are {db.side} — every
          row without a side is out of the table too
        {:else}
          {fmt(db.sidedHere)} option row(s) here · both sides, which narrows nothing
        {/if}
      </span>
    </div>
  {/if}
  </section>
{/if}

{#snippet stranded(/** @type {any} */ db)}
  <!-- ══ ABOVE THE VIEW SPLIT, BECAUSE A STRANDED RUNG IS NOT A PROPERTY OF
       THE VIEW ══
       This sat inside the `{:else}` below — the Bars branch — so it drew in
       one of the two views and not the other. Switch to Coverage, strand a
       Timeframe, an Expiry or a Side, and the table emptied with only the
       generic `narrowing` list to explain it: no statement that the rung is
       no longer offered, and no Clear button, because the refusal was
       attached to the grid it happened to sit beside rather than to the rung
       it is about. `view` is live in both directions again — the tablist
       writes it — so "one view has it" means "half the time it is missing".
       `stranded` folds the rungs, not the rows, so it is the same answer in
       either view and belongs before the split. ══ -->
  {#if db.stranded.length > 0}
    <p class="stranded" role="status">
      {#each db.stranded as st (st.rung)}
        <span class="sitem">
          <!-- WHAT IT DOES TO THE TABLE, TRULY (DB-020): "no row can match"
               only when none does — never over a full grid. -->
          <b>{st.rung}</b> is set to <b>{st.value}</b>, which the rungs above it no longer offer —
          {st.effect}
          <button class="btn ghost sm" type="button" onclick={st.clear}>Clear {st.rung}</button>
        </span>
      {/each}
    </p>
  {/if}
{/snippet}

<!-- ======================================================================
     THE FEED RUNG — FIRST IN THE STRIP, AND A CONTROL RATHER THAN A READOUT.
     ----------------------------------------------------------------------
     WHAT STOOD HERE AND DOES NOT ANY MORE: `.cell.scopeline`, a caption over
     a `.feedname` span of plain text. It was drawn to `.mnyb`'s exact metrics
     — same size, same weight, same mono family — so it sat in a row of six
     dropdowns looking like the seventh and doing nothing, and the one real
     selector was two rows away in the top bar. A face that looks like a
     control and refuses the press is worse than no control at all.

     SO THE CONTROL IS HERE AND THE TOP BAR YIELDS. `/db` is on the shell's
     `FEED_OWNED` list as of the same commit that added this rung, so the
     count of feed pickers on this route is exactly one, in the place the
     approved design draws it: the FIRST cell of the query strip, because the
     feed is this page's whole scope and every rung after it is that feed's
     answer.

     A SNIPPET RATHER THAN MARKUP IN PLACE, because the strip is not the only
     place it has to appear. The failed read, the unchosen feed, the loading
     store and the empty store are four states with no strip in them, and each
     is exactly the state in which an operator most needs another feed. A
     control that vanishes with its data is the dead end this snippet exists
     to make impossible.

     IT IS `.picker`/`.mnyb`/`.menu`/`.opt` AND NOT A `Picker`, for the reason
     Universe, Month and Timeframe are not: a `Picker` row is a checkbox, the
     feed is a SELECTION — exactly one, never a set — and a feed the server
     refuses needs a row that is DRAWN, DEAD, and carrying `/feeds.json`'s own
     reason. A tick cannot say why it is unavailable.
     ====================================================================== -->
{#snippet feedRung(/** @type {any} */ db)}
  <div class="field">
    <!-- THE CAPTION IS THE CAPTION. What the italic sub-clause used to say —
         that this is the page's whole scope, and where it is chosen — is on
         the control's `title`, one hover from the thing it is about, and the
         clause below states the same in the document. -->
    <span class="lab">Broker feed</span>
    <!-- THE SAME `Picker` /ingest'S FEED RUNG NOW USES, AND THAT SYMMETRY IS
         THE WHOLE POINT OF THIS CHANGE.

         BOTH pages hand-rolled this one control and no other. Every other rung
         here was already `Picker`; /ingest's were `.dd`/`.ddm`. So the feed
         menu was the last place the two pages drew one question two ways — a
         plain ✓ list with no filter here, a different plain ✓ list with no
         filter there — and it is the menu, not the label, that a reader
         actually looks at. Unifying the class names in 673e7da made the labels
         match and left this untouched, which is exactly why the pages still
         did not look alike afterwards.

         EVERY BEHAVIOUR IS KEPT AND NONE RE-IMPLEMENTED:

         * A feed is selectable when its ingest is ready OR its store holds
           bars (`$lib/feed-row.js`): viewing stored bars never touches the
           archive folder `/feeds.json`'s `ready` is about. A not-ready feed
           that holds data carries the ingest reason as a note, not a strike.
         * A refused feed is DRAWN AND DISABLED with `/feeds.json`'s own reason,
           never omitted — `CLAUDE.md` §4, and `Picker`'s `disabled` + `why` is
           the shape every other rung on this page already uses.
         * The per-feed survey count keeps its four states — unavailable, not
           read, N held, not surveyed — as the row's `detail`, so a dash is
           still a dash and never a zero.
         * The control is NEVER disabled, which is what keeps it reachable when
           the list is empty; the empty case is stated below rather than behind
           a dead press.

         SINGLE, because this page shows one store at a time and the operator
         asked for exactly that: every rung on /db is `single`. -->
    <Picker
      single
      filter
      label="feeds"
      title={`${db.feedName}. THE PAGE'S WHOLE SCOPE: every count on this page is this feed's store — every row read from /store.json?feed=${feeds.active ?? ''} and every membership count from /instruments.json?feed=${feeds.active ?? ''} — and no page in this product puts one feed's numbers beside another's, because the two are not the same instrument universe, the same session handling or the same price scale. A bar belongs to the vendor that supplied it, so changing this changes the store, not the view of one.${feeds.error ? ` The feed list itself could not be read: ${feeds.error}. Nothing below this line has been scoped to anything.` : ''}`}
      summary={feeds.error
        ? 'Feed list unread'
        : feeds.all.length === 0
          ? 'No feeds'
          : feeds.active
            ? db.feedName
            : 'Select a feed'}
      rows={offeredFeeds(feeds.all).map((f) => feedPickerRow(f, survey.byFeed.get(f.wire), fmt))}
      selected={new Set([feeds.active])}
      onchange={(/** @type {Set<string>} */ sel) => {
        const next = [...sel][0];
        if (next) selectFeed(next);
      }}
    />
    <!-- THE STATES BEFORE A STORE ANSWERS ARE NAMED RATHER THAN COUNTED: a
         "0 held" under a feed nobody chose, or under a read that failed, is a
         claim about a disk nobody reached. -->
    <!-- `N held · read <clock>` IS GONE. The pane head above already carries
         `as of <clock>` beside a Refresh button, so this restated the one fact
         on screen twice, and the held count is the face of every rung below it.
         Every branch that remains is a refusal or an in-flight state.
         THE `{#if}` WRAPS THE SPAN. Left inside it, the normal state — no error,
         a feed chosen, not loading — rendered an EMPTY `.note`, and a `.note`
         carries its marker as `::before`, so "empty" paints a bare `·` under the
         control. Emptying the element and keeping the element is how a cut
         leaves litter. -->
    <!-- ONLY THE OPERATOR'S FEEDS (DB-051, D-3338). The others are not
         listed, and one line says they exist and why — never a silent
         omission. -->
    {#if excludedSentence(feeds.all)}
      <span class="note" title={excludedSentence(feeds.all)}>{excludedSentence(feeds.all)}</span>
    {/if}
    {#if feeds.error || db.error || !feeds.active || (db.loading && db.rows.length === 0)}
      <span class="note" class:warn={Boolean(db.error) || Boolean(feeds.error) || !feeds.active} use:wholeTitle>
        {#if feeds.error}
          the feed list could not be read, so nothing below is scoped to anything
        {:else if db.error}
          the store could not be read for this feed — every count on this page is this feed's store,
          so none is shown
        {:else if !feeds.active}
          no feed is chosen, so no store has been read — this control is the page's whole scope
        {:else}
          reading this feed's store — every count below is this feed's store
        {/if}
      </span>
    {/if}
  </div>
{/snippet}

<!-- ======================================================================
     ONE END OF THE MONTH WINDOW — the field, the ▦, and the calendar.
     ----------------------------------------------------------------------
     ONE SNIPPET, RENDERED TWICE. From and To differ in three things and only
     three: which state they write, which side the panel hangs off, and which
     end of the store they open at when they have no bound yet. Writing the
     panel out twice is how the two ends drift into two controls.

     `value` is the raw `YYYY-MM` and it is never anything else. `monthLabel`
     is applied at the three places text reaches a screen — the closed field,
     the grid's own `title`, and the footer — and nowhere near the `{#each}`
     key, the `aria-selected` comparison or the assignment.
     ====================================================================== -->

<style>
  /* ---- the time window and the landing ---------------------------------
     THESE SIT INSIDE `.dates`, BESIDE TWO `DayField`s, SO THEY ARE BUILT TO
     `DayField`'s RECIPE AND NOT TO A DIFFERENT ONE.

     A first version used `.cell` and the shared `.search`, and every single
     metric came out wrong against the control six pixels to its left —
     MEASURED on the running page: 40px tall against 48, padding 7/6 against
     11/13, weight 400 against 600, radius 8 against 9, and the label in 12px
     MONO against 12.5px sans at 700. The cell was `display: block` while
     `.dcell` is a flex column with a 4px gap, so the label did not even sit on
     the same line as its neighbour's.

     That is a STRUCTURAL mismatch, and `theme.css` says why token-nudging
     cannot fix one: the strip's fields are subgrid rows precisely so that
     label, control and note share a baseline rather than being "three flex
     children per column, each free to be its own height". A control that opts
     out of the structure cannot be aligned back into it.

     `line-height` IS THE ONE THAT HIDES. `DayField`'s own comment records
     agreeing on 16px, 600, 11px/13px, 9px radius and mono and STILL rendering
     48 against 43, because it set no line-height and inherited a smaller one.
     It is listed here for the same reason it is listed there.

     Not pushed into `DayField`: that component is a DAY control — a calendar
     popup, day bounds, a struck-through rejection line — and a time box shares
     its metrics, not its behaviour. Copying six declarations is cheaper than a
     second mode on a shared component, and this comment is the link between
     them. */
  /* ONE ROW WITH THE DATES WHERE THERE IS ROOM, AND STILL A GROUP WHERE THERE
     IS NOT.
     A first version forced `flex-basis: 100%` — always its own line — because
     with the three loose among the DayFields the wrap fell between FROM TIME
     and TO TIME, and a pair split across rows reads as two unrelated controls.
     That fixed the split by spending a whole row on it, which is the wrong
     trade on a window that is one thought: a day AND a minute.
     `1 1 auto` puts them beside TO DATE and keeps the grouping, because the
     wrapper is still a flex container: the three can only break INSIDE it, and
     the group moves as one when the row runs out. The `.dates` gap is reused
     rather than restated so the six controls sit on a single rhythm. */
  /* NO LAYOUT OF ITS OWN — its children are the grid items. The wrapper stays
     because it is the group `{#if !dayRung}` hangs off, and `display: contents`
     says exactly what its relationship to the row is: this box is not a box,
     these three belong to my parent's tracks. Every flex rule it carried —
     `nowrap`, a basis, a gap — was arithmetic standing in for a grid that was
     one declaration away. */
  .times {
    display: contents;
  }
  .tcell {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  /* `.dlbl`, declaration for declaration. */
  .tlbl {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    white-space: nowrap;
  }
  /* THE INPUT RULES ARE GONE WITH THE INPUTS. `.tin`, `.gorow` and `.gobtn`
     styled three text boxes and a confirm button into agreeing with a day
     field; `Picker` renders `.pbtn`, which already measures 48px on this strip
     and needs no help. Six declarations that had to be kept in step with a
     component they only resembled are now zero. */
  /* Not a refusal and not a success — it reports what the press DID, including
     "it landed elsewhere, and here is how far". `--dim` rather than `--warn`:
     nothing has gone wrong when the market had no print at the minute asked
     for. */
  .jwhy {
    /* A WHOLE ROW OF THE STRIP. The wrapper is `display: contents`, so a
       sentence here is a grid item; left to auto-placement it took one narrow
       track beside Go to and wrapped to a column of words. */
    grid-column: 1 / -1;
    margin: var(--s3) 0 0;
    color: var(--dim);
    font-size: var(--fs-mini);
  }
  /* THE REACH NOTE EARNS `--warn` WHERE THE LANDING NOTE DOES NOT. One reports
     a press that worked and moved somewhere near; this one reports an answer
     that is incomplete in a way the grid cannot show, which is the case §4
     asks to be made loud. */
  .jwhy.warn {
    color: var(--warn);
  }
  .jwhy b {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }

  /* ---- THE STRIPS ----------------------------------------------------
     One row of controls on one raised board. `overflow: visible` is
     load-bearing and not an oversight: every menu and every calendar in the
     strip hangs BELOW its own cell, and a clipped popup is a control that opens
     into nothing.

     ONE ROW, AND THE `nowrap` IS THE WHOLE FIX FOR A BROKEN BASELINE.
     ----------------------------------------------------------------------
     This was `flex-wrap: wrap` over cells with a 232px flex BASIS, and nine
     rungs at 232px cannot share a line at any width this pane is ever given —
     so the strip broke into two flex lines. A wrapped flex line is stretched
     to ITS OWN tallest cell, not the container's, so the second line began
     wherever the first one ended: two captions, two heights, and a row that
     reads as an accident rather than a design. Nothing was wrong with any
     individual rung — there were simply more of them than the basis allowed.

     `nowrap` here plus `flex: 1 1 0` on the cell below is what makes ONE line
     unconditional: a zero basis means the sum of the hypothetical sizes is
     zero, which is never greater than the line, so there is no width at which
     the browser may wrap. Every cell then takes an equal share of what is
     left, which is the same statement as "one baseline and one height" —
     every caption starts at the same y, every control sits 2px under it, and
     every clause closes the cell on the same line.

     WHAT PAYS FOR IT IS CLIPPING, WHICH THIS STRIP ALREADY CHOSE. The
     caption, the `.mnyb` face, `Picker`'s face and the `.note` clause are all
     `nowrap`/ellipsis with the whole text on a `title` and in the document —
     that was the rule before this change and it is why narrowing a cell costs
     no fact. */
  /* THE STRIP IS /INGEST'S `.pickers` GRID, AND THAT IS THE WHOLE POINT.
   *
   * This used to be a single unbroken horizontal BAR: `display: flex`,
   * `flex-wrap: nowrap`, every rung an equal `flex: 1 1 0` share divided by
   * hairlines, with a vertical `.lead` down the left. It was a different
   * object from the control strip on /ingest, and the owner asked for one
   * object on both pages — same view, same dropdown, same search, same data.
   *
   * An earlier comment in this file recorded the opposite instruction ("the
   * owner asked for /ingest to have its OWN design, and two pages wearing one
   * vocabulary is how a change to either becomes a change to both"). That
   * decision is SUPERSEDED, on the record, by D-0153. The hazard it named is
   * real and is answered rather than ignored: the shared thing is
   * `$lib/Picker.svelte`, one component, so a change to the look lands on both
   * pages ON PURPOSE instead of drifting. What is NOT shared is either page's
   * content — the rungs, their refusals and their cascade are still this
   * file's own.
   *
   * Every metric below is /ingest's `.pickers`, copied because it is the same
   * rule and not because it is a nice number: `auto-fit` + a 248px floor is
   * what lets nine rungs wrap onto as many rows as the window allows instead
   * of nine cells fighting over one line. The bar could not wrap, so at nine
   * rungs each one got a ninth of the width and every caption clipped
   * mid-word. */
  .strip {
    /* `flex: none` IS ABOUT THE STRIP AS AN ITEM, NOT AS A CONTAINER, and the
       two roles are easy to confuse into a bug. `.board` is a flex COLUMN and
       this is one of its children; the default `flex-shrink: 1` lets a child
       be compressed below its content when the column overflows, which is what
       a tall table under it guarantees. Dropped by mistake in the grid rewrite
       below, and measured: the strip reported `height: 124px` against a
       `scrollHeight` of 409px — Timeframe and the day window were cut off at
       the panel's edge with nothing to scroll them into view. */
    flex: none;
    display: grid;
    /* 216px, AND THE NUMBER IS ARITHMETIC RATHER THAN TASTE.
       MEASURED on the running page: the strip's CONTENT box is 712px — 744
       border box less 16px of padding each side — with a 12px column gap. Three
       columns therefore need `3 x W + 24 <= 712`, so `W <= 229`.
       At the 248 this read before, the third column did not fit and `auto-fit`
       fell back to two: five rungs laid out 2 + 2 + 1 with the last row half
       empty, and the table pushed to 828px on a 960px viewport — three rows of
       data visible. At 216 the same five lay out 3 + 2, one row shorter, and it
       costs nothing because a rung's control is `--ctl-h` tall whatever column
       it sits in.
       216 AND NOT 229 ON PURPOSE. The ceiling is where it breaks, so sitting on
       it means a scrollbar appearing or a border changing drops it silently
       back to two. 216 also stays clear of the other edge: four columns would
       need `4 x W + 36 <= 712`, `W <= 169`, which 216 cannot reach.
       A FIRST ATTEMPT AT 232 STILL DREW TWO, because it was measured against
       the 744 border box instead of the 712 content box. Padding is not part of
       the track budget.
       `--ctl-h` ITSELF IS NOT TOUCHED. `theme.css` sets it to 42px under a
       recorded reason — "a 34px control cannot hold 15px text with a border and
       still look deliberate" — so the height of a control is a decision this
       file does not get to relitigate for a few pixels. */
    /* NOW 180, AND THE 200 THE NOTE ABOVE REVERTED WAS REVERTED FOR A REASON
       THAT NO LONGER HOLDS. That note is kept because its reasoning was sound:
       a narrower minimum alone changes nothing, because the day window was
       `grid-column: 1 / -1` and took the whole last line at EVERY track count.
       Five rungs plus a full-width row is two rows at six tracks and two rows
       at seven.
       WHAT CHANGED IS THE PREMISE. `.field.wide` no longer spans; it and its
       `.dates` child are `display: contents`, so the day window's own controls
       are grid items of THIS grid rather than a row of their own. Seven tracks
       then means seven controls on one line, which is what the earlier attempt
       wanted and could not reach.
       MEASURED ON THE RUNNING PAGE at a 1440px viewport, 1382px content box:
       216 gave six tracks of 220px, the strip 189px tall, the first bar row at
       y=309, `Fit` 10 rows. 180 gives seven tracks of 187px, the strip 105px,
       the first row at y=225 and `Fit` 12. Eighty-four pixels, two rows.
       THE NARROW END IS UNCHANGED, which is the whole risk of moving it. The
       arithmetic above still governs: at a 744px viewport the content box is
       712px with a 12px gap, three columns need `3 x W + 24 <= 712` so W <= 229,
       and FOUR would need `4 x W + 36 <= 712` so W <= 169. 180 is above that
       floor, so four can never appear there — measured at 744: three tracks of
       221px, the same layout 216 produced. The margin to the four-column edge
       is 11px rather than 47px, and that is the cost of the change, stated.
       `--ctl-h` ITSELF IS STILL NOT TOUCHED, for the reason the note above
       gives: `theme.css` owns it under a recorded decision. */
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    /* ---- RESET A ROW TEMPLATE THIS FILE DID NOT WRITE ----------------------
     * `theme.css` §8 also styles `.strip`, and it declares
     * `grid-template-rows: var(--lab-h) var(--ctl-h) auto` for a SUBGRID
     * vocabulary: three named rows — label, control, note — that every
     * `.field` child spans with `grid-template-rows: subgrid`, so a two-line
     * refusal under one control cannot push that control off the baseline its
     * neighbours sit on.
     *
     * That rule was always matching this element. It was INERT only because
     * this file said `display: flex`, and a row template does nothing to a
     * flex container. Switching to `display: grid` woke it, and the symptom
     * was measured before it was understood: the row holding the rungs
     * resolved to `--ctl-h` (42px) while the rungs in it were 88–104px, so
     * every one of them overflowed its own track and the day window below
     * drew straight through the feed rung's note.
     *
     * `none` rather than adoption, and the reason is wrapping. The shared
     * vocabulary is built for ONE row of fields — three explicit tracks, every
     * field pinned to `grid-row: span 3`. This strip has six rungs plus a
     * full-width day window and has to wrap to a second band at any window
     * narrower than six tracks; pinned rows cannot do that, and a dense flow of
     * repeating three-row bands is a much larger thing than the alignment it
     * would buy. /ingest reaches the same look with plain auto rows and a flex
     * column per rung, so that is what this takes. */
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
  /* ONE TRACK'S WORTH, AS A WIDTH RATHER THAN A FLEX BASIS. `flex: 0 1 300px`
     described a flex item and this is a grid item now, so it did nothing at
     all — the lone feed rung stretched to `max-content`, which on a blank page
     is the width of its longest caption. /ingest solves the identical problem
     with `.blankpick { max-width: 320px }`; this is that rule. */
  :global(.strip.solo) .field {
    width: 300px;
    max-width: 100%;
  }
  /* THE CONTRACT STRIP IS THE QUIETER OF THE TWO, and it is ALWAYS DRAWN. The
     approved design collapses it when the segment carries no contract; here it
     may not, because on today's wire there is never a chain — `/store.json`
     names no strike and carries no price — and a strip that collapsed would
     take the two REFUSALS with it. "Why is there no strike control" is exactly
     the question `CLAUDE.md` §4 says must be answered out loud rather than by
     an absence, so the strip stays and its cells go `.off`. */
  /* THE FOLD'S CSS IS GONE WITH THE FOLD — six rules, removed BY NAME rather
     than as a line range. A first attempt cut from the fold's comment to the
     next selector and took the seven ported `:global(.pmenu …)` rules with it,
     which un-hid every radio and collapsed the bar table's own container. That
     is the hazard `CLAUDE.md`-adjacent notes keep restating: a CSS range is not
     a rule, and only the rule's own text bounds it. */
  /* ══ THE SAME SELECTION /ingest DRAWS, PORTED VERBATIM ══

     `Picker` is shared, so both pages already had the same MARKUP. What they
     did not share is the page-scoped treatment: /ingest carries nine
     `.field :global(.pmenu …)` rules and this file carried none, so the two
     strips rendered the same component two different ways — /ingest with the
     row carrying its own state, /db still with a gutter of hollow radio rings
     down the left edge of every menu.

     THE ROW IS THE CONTROL. Every rung on this page is `single`, so every menu
     was a column of rings with exactly one filled — a second mark saying what
     the tinted, railed row already says, and the thing that read as dated. The
     input is moved OUT OF THE FLOW, not removed: same type, same :checked, same
     change event, same tab order, so the keyboard and the reader are untouched.

     THE CHECKBOX RULES COME TOO, even though nothing here is multi-select
     today. They cost nothing while unused and they mean the two pages cannot
     drift the next time a menu on either one gains a second tick.

     Kept as page rules rather than pushed into `Picker`: the component is
     shared with /markets and /autopilot, and this is the boundary /ingest
     already established. */
  .field :global(.pmenu .plist input[type='radio']) {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    padding: 0;
    opacity: 0;
    pointer-events: none;
  }
  /* `order: 1` KEEPS THE CHECK ON THE FIRST LINE. `.pwhy` is `flex: 0 0 100%`
     so it claims a row of its own, and the check is an `::after` — last in
     source order, which lands it on a THIRD line under the sentence, alone at
     the left. Ordering the sentence after it puts the mark back beside the name
     with no pixel offset to go stale. */
  .field :global(.pmenu .plist label:has(input[type='radio']) .pwhy) {
    padding-left: 0;
    order: 1;
  }
  /* THE CHECK, DRAWN RATHER THAN TYPED — two borders on a rotated box, the
     technique `Picker` already uses for its own tick, so it cannot come out as
     a missing glyph. */
  .field :global(.pmenu .plist label:has(input[type='radio']:checked))::after {
    content: '';
    flex: 0 0 auto;
    align-self: center;
    width: 5px;
    height: 10px;
    margin: 0 2px 3px 0;
    border: solid var(--acc);
    border-width: 0 2px 2px 0;
    transform: rotate(43deg);
  }
  /* The input carried the focus ring and the input is out of the flow, so the
     row takes it — inset, so it reads as the row being focused. */
  .field :global(.pmenu .plist label:has(input[type='radio']:focus-visible)) {
    outline: 2px solid var(--acc);
    outline-offset: -2px;
  }
  /* A row that cannot be chosen shows no check: it is struck through and
     dimmed, which `Picker` already does. */
  .field :global(.pmenu .plist label.pdis)::after {
    content: none;
  }
  .field :global(.pmenu .plist input[type='checkbox']) {
    width: 14px;
    height: 14px;
    /* 4px, not Picker's 6. On a 14px square a 6px radius is nearly half the
       side and the box renders as a ROUNDEL — the one shape a checkbox may not
       have, because round reads as radio and radio means the others untick. */
    border-radius: 4px;
  }
  .field :global(.pmenu .plist input[type='checkbox']:checked)::after {
    width: 3.5px;
    height: 7px;
    margin-top: -1px;
  }

  .strip.sub {
    background: var(--panel);
  }

  /* ---- A RUNG ---------------------------------------------------------
     Caption, control, and one clipped clause. The clause CLIPS RATHER THAN
     WRAPS: this is where a fact that used to be a paragraph now lives, and a
     clause that can grow to three lines would reflow the whole strip the moment
     a universe answered with a longer refusal. The text is WHOLE in the
     document, so anything reading rather than looking gets all of it, and the
     full sentence is also on the control's own `title`. */
  /* THE CELL IS THE SAME THREE ROWS EVERYWHERE — caption, control, clause —
     and that is what makes "one height" true of the CONTENT and not only of
     the boxes. `align-items: stretch` has always given the boxes one height;
     it cannot give the text one, and the two month fields carried no clause,
     so their content stopped one line short and left a hole under the field
     in a row that was otherwise closed. They have a counted clause now. */
  /* A RUNG IS /INGEST'S `.pk` — a three-row flex column and nothing else.
     The hairline dividers and the cell padding both belonged to the bar: a
     divider separates cells sharing one box, and there is no shared box now,
     just tracks with a real gap between them. Keeping either would draw the
     bar's furniture around a grid that is not a bar. */
  :global(.strip) .field {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-width: 0;
  }
  /* NO CELL-LEVEL FOCUS UNDERLINE. It was the bar's signal — one 2px rule
     under a group of controls sharing a hairline box — and under a grid cell
     it draws across the gap below, pointing at whatever rung wrapped onto the
     next row. Every control in here already states its own focus: `.pbtn`,
     which is the signal /ingest shows and is the one a keyboard reader
     actually needs — per control, not per group. */
  /* A RUNG THAT CANNOT BE USED SAYS SO ON ITS CAPTION AND ITS CONTROL, NEVER ON
     ITS CLAUSE. The clause is the refusal; dimming it would be the page
     whispering the one sentence that has to be read. */
  :global(.strip) .field.off > span:first-of-type {
    opacity: 0.55;
  }
  /* THE CAPTION IS /INGEST'S `.plbl`, WHICH IS THE SHARED `.lab` IN
     `theme.css` §8. Four properties differed and every one of them was
     visible: this drew the sans face at `--w-bold` in `--dim` with the default
     line-height, /ingest draws MONO at `--w-semi` in `--faint` on `--lab-h`.
     Two faces for one label is most of why the two strips did not read as the
     same control. `--lab-h` is the height the grid reserves for a label row,
     so it is also what keeps every rung's control on one baseline. */
  :global(.strip) .field > span:first-of-type {
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    line-height: var(--lab-h);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* THERE IS NO `> span:first-of-type i` RULE ANY MORE, and the node it drew
     is gone rather than hidden. Every caption in this strip carried an italic
     sub-clause — "which membership to hold this store against", "bar length —
     the rung each row is stored at" — set inside a `nowrap`/ellipsis label.
     At a full-width basis it merely looked busy; at an EQUAL SHARE of one
     unbroken row it clips mid-word, and a caption that reads "UNIVERSE which
     membe…" is a label the page spent its width failing to say. Each clause
     moved onto the caption's own `title`, whole, which is where a gloss on a
     word belongs — and the substantive fact was never in the gloss anyway: it
     is the `.note` clause under the control. A rule left standing for a node
     nothing builds is how the sub-clause finds its way back. */
  /* THE CLAUSE IS /INGEST'S `.pknote`: MONO at `--fs-micro`, and it opens with
     a `· ` the way every caption in that strip does. The bullet is generated
     rather than typed into each of the nine strings, so a rung added later
     cannot forget it. Two lines are kept from the bar-era rule below — see
     the measurement there; /ingest reaches the same place with `.pknote.wrap`
     on the notes that need it. */
  :global(.strip) .field > .note {
    margin: 0;
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--faint);
    min-width: 0;
    /* `max-width` IS THE ONE THAT MAKES THE ELLIPSIS FIRE.
     *
     * `overflow` and `text-overflow` were already here and did nothing: with
     * no upper bound the caption simply grew past its cell — FIND's measured
     * 372px inside a 239px cell — and clipped at its OWN 372, which is not a
     * clip at all. It printed straight over the ROWS cell to its right, so
     * "a prefix of up to 4 characters, one Map probe per keystroke" sat on top
     * of "0 of 9 row(s) here are short" and neither could be read.
     *
     * The full sentence is on the `title`, so nothing is lost to the pointer
     * or to a screen reader; what is lost is one caption overwriting another. */
    max-width: 100%;
    /* TWO LINES, NOT ONE CLIPPED ONE.
     *
     * Measured: these captions need 156px to 581px and the cell gives 143px, so
     * every one of the six was cut mid-word -- "9 rung(s) held · 1min 1 · 3m…",
     * "every count below is this fe…". A caption that stops before its first
     * fact is not a shorter caption, it is a blank one that costs a line.
     *
     * Clamped at two so a long one cannot push the strip open, and the whole
     * sentence is still on the `title`. The leading count -- which is the fact
     * these carry -- now fits in every one of them. */
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: normal;
    line-height: 1.35;
  }
  :global(.strip) .field > .note::before {
    content: '· ';
    color: var(--faint);
  }
  :global(.strip) .field > .note.warn {
    color: var(--warn);
  }
  :global(.strip) .field > .note.warn::before {
    color: var(--warn);
  }
  /* THE FIND BOX AND THE TWO CONTRACT RUNGS ASK FOR MORE OF THE ROW, and they
     are the only three that do: the box holds a typed instrument key and the
     two `Picker`s hold a summary like "412 of 1,204 strikes". Every other cell
     is a label and a short face.

     A LARGER `flex-grow` AND STILL A ZERO BASIS. The basis is what decides
     whether the row may break; the grow is what decides how the one row is
     divided. Asking for more of the line therefore costs nothing here — a
     wider Find box cannot push a rung onto a second baseline, which is
     exactly what a 264px basis used to do. */
  /* A GRID ITEM HAS NO FLEX BASIS, so `flex: 1.5 1 0` and `flex: 1.3 1 0` were
     inert the moment `.strip` stopped being a flex container. They asked for a
     larger share of ONE line, and the grid's answer to "this rung needs more
     room" is a whole track — `span 2` where two tracks exist, and nothing
     where they do not, which is what `auto-fit` already handles by wrapping.
     Left as a no-op they would read as live sizing to the next reader.

     THAT SENTENCE AND THE TERMINATOR AFTER IT WENT WITH `.combo`. `126152af`
     dropped `.strip .field.combo,` from the selector list below — correctly,
     the class is gone — and took the two prose lines above it in the same
     contiguous cut, the closing terminator among them. The comment then ran
     on to the next terminator it could find, at the end of THE DAY WINDOW's
     heading below, so the rule here and that whole heading were comment text.
     The terminator cannot be shown to you inside this sentence: a CSS comment
     has no escape, and the star-slash pair ends the comment wherever it
     appears.

     `min-width: 0` IS KEPT, AND THAT IT RESTATES `.strip .field` IS THE POINT
     OF SAYING SO. The base rule above sets the same property to the same value
     on every field of the strip, and has done since `e355455b` — the commit
     that wrote this one — so this has never been the declaration that lets an
     `.mcell` shrink. Nothing between the two sets `min-width` to anything
     else, and neither rule sits inside a media query, so the extra specificity
     buys nothing today. It stays because its selector still matches: `.mcell`
     is live markup on the two contract rungs and the skeleton that stands in
     for them, and naming the cells the grid is allowed to squeeze is where a
     reader will look first when one of them refuses to. */
  :global(.strip) .field.mcell {
    min-width: 0;
  }

  /* ---- THE DAY WINDOW, WHICH IS /INGEST'S `.pk.wide` AND `.dates` --------
     `1 / -1` rather than `span 2` is the rule /ingest states and the reason is
     worth keeping: a spanning item is placed on the first line that FITS it,
     so `span 2` let the window finish a row of rungs whenever an odd count
     left a gap — and the row it finished was the row it then broke. `-1` is
     whatever the last line happens to be, so it is right at every track count,
     including one, where `span 2` grows an implicit second column and pushes
     the panel past its own box. */
  :global(.strip) .field.wide {
    /* `display: contents` AND NOT `grid-column: 1 / -1`, WHICH IS WHAT LET THE
       STRIP CLOSE TO ONE ROW. Spanning every track meant the day window owned
       the last line whatever the track count, so the strip could not be
       shorter than two rows however narrow the tracks got — the note over
       `grid-template-columns` records a 200px attempt reverted for exactly
       that. Dissolving this box and its `.dates` child makes the day window's
       controls grid items of the strip itself, so they flow into the same
       tracks as the five rungs and the row closes when they fit.
       BOTH LEVELS HAVE TO GO. `.dates` is between this and the controls, and
       one `display: contents` short of the bottom leaves a single grid item
       where the controls should be. */
    display: contents;
  }
  /* THE WINDOW JOINS THE STRIP'S GRID INSTEAD OF INVENTING A WIDTH.
     MEASURED, and it is why every attempt to align this row by tuning numbers
     failed: the strip is `repeat(auto-fit, minmax(216px, 1fr))`, which at this
     width resolves to SIX TRACKS OF 247px, and the five controls in its top
     row are 247px each because of that. This row was a flex line under a
     hand-computed 1042px cap, so its children measured 327, 327, 83, 81 and
     177 — five widths, none of them 247, in a strip whose whole first row is
     one number.
     A CAP IS AN ANSWER TO THE WRONG QUESTION. Every version of it — 560, then
     1042 — was arithmetic reproducing what the grid already computes, and each
     was correct only until a control was added. Declaring the same template
     here makes the row a continuation of the one above it: the day and time
     fields land in the same six tracks, on the same 247px, and adding a sixth
     control needs no new number.
     `.times` BECOMES `display: contents` SO ITS CHILDREN ARE THE GRID ITEMS.
     Kept as an element rather than deleted because it is still the group the
     `{#if !dayRung}` hangs off, and because `display: contents` is precisely
     "this box has no layout of its own, its children belong to my parent" —
     which is the relationship it has. */
  :global(.strip) .field.wide .dates {
    /* DISSOLVED, LIKE ITS PARENT. This used to be a grid of its own, and the
       long note it carried was about making its tracks line up with the row
       above: `auto-fill` and not `auto-fit` because auto-fit collapses the
       tracks it has no item for and divided the width among the survivors —
       measured 299px against a top row of 247.
       That whole problem was self-inflicted. Two grids can only be kept in
       step by matching their track rules and re-measuring whenever either
       moves; ONE grid cannot disagree with itself. With both wrappers
       `display: contents` these controls sit in the strip's own tracks, so
       they are the same width as the rungs above them by construction rather
       than by a number copied between two rules.
       It also fixes what the old arrangement looked like on a day rung: the
       time controls are hidden there, so this grid drew six tracks for two
       items and left roughly 950px of empty row under a nearly full one. */
    display: contents;
  }
  .dates {
    display: flex;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: var(--s5);
  }

  /* THE SHARED `Picker` NOW WEARS ITS OWN FACE, AND DELETING A RULE IS THE
   * WHOLE OF THE FIX.
   *
   * What stood here stripped `$lib/Picker.svelte`'s `.pbtn` back to a 13px
   * borderless inline face — `border: 0`, `padding: 0 20px 0 0`, a 19px line
   * box, the caret pulled in to 6px. Its reasoning was sound for the strip it
   * was written against: that strip's other rungs were hand-rolled `.mnyb`
   * faces at those metrics, and a bordered box among them would have read as
   * two kinds of control in one bar.
   *
   * The premise is gone. Every rung in this strip is a `Picker` now, so there
   * is nothing left to match DOWN to — and Picker's untouched `.pbtn` is
   * already, property for property, /ingest's `.ddb`: 15px semibold mono,
   * 11px/14px padding, 9px radius, the caret at 17px/12px. /ingest says so in
   * its own comment ("every measurement below is Picker's, so the five read as
   * one control"). The two pages were drawing the SAME COMPONENT and this file
   * was the only reason they did not look it.
   *
   * The one rule kept is /ingest's, and for /ingest's stated reason: `.pbtn`
   * does not clip, so a long summary wraps to a second line and makes that ONE
   * rung taller than its neighbours — the row broken by a string rather than
   * by a rule. Picker is shared and is not edited from here, so the clip is
   * applied from this page, to Pickers inside this strip only. */
  :global(.strip) .field :global(.pbtn) {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* ---- THE DAY FIELDS TAKE /INGEST'S BOX ------------------------------
     A COMMENT ATE THIS RULE, AND A RULE INSIDE A COMMENT IS EMITTED NOWHERE.
     The terminator that should have closed the sentence below went missing —
     and it cannot be written here either, because a CSS comment has no escape
     and the star-slash pair ends this one wherever it appears. So the
     selector, five declarations and closing brace were all comment text:
     shipped by no stylesheet, and invisible on a page that still rendered.
     Measured before the repair: `max-width: none` occurred exactly ONCE in
     this file, inside the comment. It is restored here, outside it.

     WHY THE RULE EXISTS, which is the Picker face's reason: an earlier strip
     de-styled these fields to a borderless 19px line to match hand-rolled
     neighbours that are gone. /ingest's day field is `font-size:
     var(--fs-base); padding: 11px 13px; border-radius: 9px`, and restating
     that here is what makes a row of dropdowns and a pair of dates read as
     ONE strip instead of two kinds of control in one bar.

     THOSE THREE CHANGE NOTHING, AND SAYING SO IS THE POINT. They are
     `$lib/DayField.svelte`'s own values, declaration for declaration, so this
     page does not move a shared component's metrics — it only refuses to let
     them drift. `width: 100%` is the one that does work: `.din` is
     `flex: 1 1 auto`, so its flex basis is its width, and this is what makes
     it fill the grid track the strip hands it rather than size to its text.
     `max-width: none` is a reset with nothing to reset today — kept because
     it is the shared theme's bordered-box rules this field must outrank if it
     is ever given one, and it costs a line to say so.

     `:global()` IS LOAD-BEARING. `.din` is DayField's class, not this file's,
     so a bare `.strip .field .din` compiles to a scoped selector that matches
     no element in this component — dead in the stylesheet and reported by the
     compiler as an unused selector. The Picker clip twenty lines up is the
     same shape for the same reason.

     THE MONTH HALF OF THIS RULE'S OLD HEADING IS GONE. /db carried a month
     field that was a button rather than a text box; the control was removed
     with the from/to range (see the markup note at `THE MONTH WINDOW`), so
     the `.dbtn` and `.dval` overrides that stood here went with it and only
     the two `DayField`s remain to box. */
  :global(.strip) .field :global(.din) {
    width: 100%;
    max-width: none;
    font-size: var(--fs-base);
    padding: 11px 13px;
    border-radius: 9px;
  }

  .btn.sm {
    padding: var(--s2) var(--s4);
    font-size: var(--fs-xs);
  }

  /* ---- MOTION. Every transition on this page is in here, so a reader who
     asked for less motion gets a completely static one. */
  @media (prefers-reduced-motion: no-preference) {
    /* THE CELL'S UNDERLINE WIPES IN FROM NOTHING. It is a focus signal, so it
       is a transition on a transform and never on the outline itself — the
       ring the control inside owns has to be right on the frame it lands. */
    :global(.strip) .field::after {
      transition: transform var(--d-state) var(--ease-out);
    }
  }
  /* THE BAR IS A BACKGROUND, NOT AN ELEMENT.
   *
   * One gradient on the cell that already exists, so there is no extra node
   * per row and nothing to keep in sync with the text. It grows from the
   * RIGHT, under the right-aligned figure, so the number stays the thing the
   * eye lands on and the bar is read second.
   *
   * `--mag` is set inline per cell and defaults to 0 here, so a cell that
   * carries no change — the nulls, and every column that is not this one —
   * draws nothing at all rather than a zero-width artefact.
   *
   * `prefers-reduced-motion` is honoured below: the width is a paint, not an
   * animation, but the transition on it is.
   */
  /* ---------------------------------------------------------------------
     THE CHANGE COLUMN IS A SIGNED SILHOUETTE.

     It was a bar growing from the right, coloured up or down. Colour alone is a
     poor encoding: it dies in greyscale and it dies for a red-green dichromat,
     and it gave the reader no shape to scan. Now the axis is the cell's centre
     and the bar grows RIGHT for up and LEFT for down, so direction is carried
     by POSITION and the column resolves into a distribution down fifty rows.

     THE TRANSITION HERE HAD NEVER RUN. It named `background-image`, which is a
     discrete property — no engine interpolates two gradients. `--mag` is now a
     registered property (theme.css) so the number the gradient is built from is
     what animates, and it does.
     --------------------------------------------------------------------- */
  /* THE STRANDED CLAUSE. Warm, not red: nothing is broken — a choice has simply
     been overtaken by one above it. */
  .stranded {
    margin: 0 0 var(--s3);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3) var(--s4);
    border: 1px solid var(--warn);
    border-left-width: 3px;
    border-radius: var(--r3);
    background: color-mix(in srgb, var(--warn) 8%, transparent);
    font-size: var(--fs-xs);
  }
  .stranded .sitem {
    display: flex;
    align-items: center;
    gap: var(--s3);
    flex-wrap: wrap;
  }
</style>
