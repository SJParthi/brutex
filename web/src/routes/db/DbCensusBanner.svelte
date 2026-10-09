<script module>
  /**
   * /db — THE CENSUS CLIENT, ITS LOADING SKELETON AND ITS BANNER
   *
   * The page's reading of `/store.json` (`censusState`) and what the page draws
   * while that reading is in flight (`censusLoading`, the skeleton the shell
   * renders in its loading branch). The banner that will name a capped, degraded
   * or behind census renders nothing yet; it is mounted at the top of the pane,
   * above every state.
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
   * Owner after WP-00: WP-56 (census client: facets, worker, refresh, lag banner).
   */
  import { store, syncStore, surveyStores, survey } from '$lib/store.svelte.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { denominators } from '$lib/completeness.js';
  import { decorateRow, instrumentDescription } from '$lib/db/decoration.js';
  import { instrumentMemo, databasePreparation } from '$lib/database-preparation.js';
  import { fmt } from '$lib/db/notes-labels.js';
  import { COLS } from '$lib/db/notes-coverage.js';
  import { SKELETON } from '$lib/db/bars-plan.js';
  import { feedRung } from './DbControls.svelte';
  /** @typedef {import('$lib/store.svelte.js').StoreRow} StoreRow */
  /** @typedef {import('$lib/db/decoration.js').DecoRow} DecoRow */

  /**
   * THE CENSUS AS THIS PAGE READS IT — the shared `/store.json` reading for the active
   * feed, its preparation into decorated rows and the prefix index, and the feeds
   * that hold rows elsewhere.
   *
   * Creates the page's effects that stood at old lines 766, 768, 800 and 1211; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function censusState(db) {
    /* ======================================================================
       DATA
       ====================================================================== */
    /* THE ROWS ARE NOT THIS PAGE'S READ ANY MORE.
       ----------------------------------------------------------------------
       `/store.json` was fetched from SIX places across this product on six
       clocks, and TWO of them were on this page. Nothing reconciled them: while a
       pull ran, this page and `/ingest` printed different totals for the same
       disk and each was right about its own snapshot. `$lib/store.svelte.js`
       reads it ONCE per (feed, generation) and folds every shape any page needs
       in one pass; these four names are the same four this file always used,
       pointed at that reading.

       `rows` IS THE RAW ARRAY, exactly what the wire sent — this page reads ten
       fields off a row and derives its own denominators, so it takes the array
       and not the census's normalised view of it. */
    const rows = $derived(store.feed === feeds.active ? store.rows : []);
    /** @type {{input:StoreRow[]|null,body:{rows:DecoRow[],observed:ReturnType<typeof denominators>,prefix:Map<string,DecoRow[]>}|null,why:string|null}} */
    let prepared = $state.raw({ input: null, body: null, why: null });
    const preparing = $derived(rows.length > 0 && (prepared.input !== rows || (!prepared.body && !prepared.why)));
    const error = $derived(store.error ?? (prepared.input === rows ? prepared.why : null));
    /* A READING THIS FEED HAS NOT ANSWERED FOR YET IS STILL "IN FLIGHT", even in
       the instant between the selection moving and the effect that re-reads it.
       Without the second clause that instant renders as a finished read of an
       empty store — "Nothing stored for Groww" over a store nobody has asked
       about — which is a claim about the disk, not a state of the page. */
    const loading = $derived(
      preparing || store.state === 'reading' ||
        (feeds.active != null && store.state !== 'error' && store.feed !== feeds.active)
    );
    /* WHEN THIS FEED'S STORE LAST ANSWERED — 0 when it never has. `store.lastOk`
       is the last SUCCESSFUL read and carries the feed it was for, so a failed
       refresh keeps the minute (under the word "last good") and a feed switch
       drops it, which is what this page's `as of` line has always meant. */
    const fetchedAt = $derived(store.lastOk.feed === feeds.active ? (store.lastOk.at ?? 0) : 0);
    /* Bumped by Refresh; the store grows under a pull. IT IS THE SHARED CLOCK:
       the bump re-reads the census for every page at once, so Refresh here can no
       longer leave `/ingest` holding an older total than this table. */
    const nonce = $derived(store.generation);

    /**
     * WHICH FEED THE ROWS ON SCREEN BELONG TO.
     *
     * Deliberately a plain `let` and not `$state`: it is compared, never
     * rendered and never derived from, so making it reactive would only add a
     * dependency the fetch effect would then have to `untrack`.
     *
     * # The defect it removes
     *
     * `rows` was left standing while the new feed's request was in flight, so a
     * feed switch put the PREVIOUS feed's counts under the NEW feed's name: the
     * title read `DB · Groww`, every tile still held Dhan's totals, and `as of`
     * still held the minute Dhan was read. Every one of those numbers was
     * counted — from the wrong store. A count that survives the query that
     * produced it is exactly the stale value `CLAUDE.md` §4 bans, and it is
     * worse than a blank because it is plausible.
     *
     * Refresh is the deliberate exception: `nonce` re-runs this effect with the
     * SAME feed, `rows` is kept, and the table stays readable under the
     * "refreshing…" indicator while the same store is re-read.
     *
     * `null` is NOT A FEED and is the only value it holds before the first
     * selection lands, which is why the effect below tests `!f` before it
     * compares: the wire string of a feed is never empty.
     *
     * @type {string | null}
     */
    let loadedFeed = null;

    /* THE SUBSCRIPTION. `syncStore` reads `feeds.active` and the shared
       generation, so a feed change re-reads and any Refresh — this page's, or the
       five-second poll `/ingest` holds while a pull runs — lands here too. The
       staleness this comment block describes is now impossible by construction
       rather than by care: the shared reading CLEARS its value on a feed change
       before the request leaves, and `rows` above refuses to hand back anything
       the reading did not stamp with this feed. */
    $effect(() => syncStore(feeds.active));

    $effect(() => {
      const f = feeds.active;
      if (!f || loadedFeed === f) return;
      loadedFeed = f;
      // A FUNCTION DECLARATION, CALLED FROM ABOVE ITSELF ON PURPOSE. The state
      // it clears is declared further down beside the code that owns it, and a
      // `const` arrow there would be a temporal-dead-zone reference from here —
      // the exact thing the FORMATTING block above says was already found and
      // fixed once on this file. A `function` hoists whole, so this call is
      // valid wherever it appears in the script.
      db.forgetPreviousFeed();
    });

    // WHERE ELSE THE DATA IS. An empty page that only says "nothing here" makes
    // the operator hunt. This names the feeds that have rows in this store — a
    // fact about the store, and a place to go. It renders no other feed's
    // numbers: one feed is selected and only that one's data is shown.
    //
    // IT IS DROPPED ON A FEED CHANGE, in `forgetPreviousFeed`, for the same
    // reason the counts are. The list excludes whichever feed was active WHEN IT
    // RESOLVED, so carrying it across a switch renders "Rows exist under Groww"
    // while Groww is the selected feed and the panel above it says Groww holds
    // nothing — a live control offering a move the page has already made. It is
    // a small stale value, but it is one produced by a query nobody re-ran.
    //
    // THE SECOND READ OF `/store.json` ON THIS PAGE IS GONE. This was an
    // independent N-request fold answering a question `loadFeeds` had ALREADY
    // asked and folded at boot — "which feed holds anything at all" — on a second
    // clock, with no stamp, and re-fired every time the condition below held.
    // `$lib/store.svelte.js` folds it once, keyed by wire, stamped with the feed
    // list it answered for; `surveyStores` is a no-op when that answer is already
    // in hand for this generation, so the common case costs nothing at all.
    $effect(() => {
      if (!feeds.all.length) return;
      // A FEED WHOSE INGEST IS NOT READY IS STILL VIEWABLE IF ITS STORE HOLDS
      // BARS, and only the survey can say whether it does — so a list carrying
      // any not-ready feed is surveyed even while the active feed has rows.
      // Header-only reads, deduped per store generation by `surveyStores`.
      const notReady = feeds.all.some((f) => f.ready !== true);
      if (!notReady && (loading || error || rows.length)) return;
      void nonce; /* a Refresh re-asks it — the store may have gained a feed */
      surveyStores(feeds.all);
    });

    // DERIVED, SO IT CANNOT SURVIVE THE QUESTION THAT PRODUCED IT. The old list
    // excluded whichever feed was active WHEN IT RESOLVED, so carrying it across
    // a switch rendered "Rows exist under Groww" while Groww was selected — a
    // live control offering a move the page had already made. Filtering against
    // the CURRENT selection at read time makes that shape unrepresentable, which
    // is why `forgetPreviousFeed` no longer has to remember to drop it.
    const elsewhere = $derived(
      feeds.all
        .map((f) => ({ feed: f, any: survey.byFeed.get(f.wire)?.any === true }))
        .filter((x) => x.any && x.feed.wire !== feeds.active)
    );

    const deco = $derived(prepared.input === rows ? prepared.body?.rows ?? [] : []);

    $effect(() => {
      const input = rows;
      const controller = new AbortController();
      prepared = { input, body: null, why: null };
      if (input.length > 0) {
        const describe = instrumentMemo(instrumentDescription);
        // Preparation starts after a yield and stays outside reactive dependency
        // tracking. Publish only one complete immutable snapshot for this input.
        void databasePreparation.read(input, (row, observed) => decorateRow(row, observed, describe(row.instrument)), {
          signal: controller.signal, tokens: row => [row.instrument, row.sym, row.underlying, row.month]
        }).then(body => {
          if (!controller.signal.aborted) prepared = { input, body, why: null };
        }).catch(why => {
          if (!controller.signal.aborted) prepared = { input, body: null, why: `Database view preparation failed: ${String(why)}` };
        });
      }
      return () => controller.abort();
    });

    Object.defineProperties(db, {
      rows: { get: () => rows },
      prepared: { get: () => prepared, set: (v) => (prepared = v) },
      preparing: { get: () => preparing },
      error: { get: () => error },
      loading: { get: () => loading },
      fetchedAt: { get: () => fetchedAt },
      nonce: { get: () => nonce },
      loadedFeed: { get: () => loadedFeed, set: (v) => (loadedFeed = v) },
      elsewhere: { get: () => elsewhere },
      deco: { get: () => deco }
    });
  }

  export { censusLoading };
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>


{#snippet censusLoading(/** @type {any} */ db)}
  {#if db.preparing}
    <p class="inline-note" role="status">Preparing the database view for {fmt(db.rows.length)} stored month rows… Totals appear after the complete snapshot is ready.</p>
  {/if}
  <!-- SKELETONS, SHAPED LIKE WHAT IS COMING. A spinner claims "something is
       happening"; a skeleton claims "a summary, a month band and a table go
       here", which is the useful half of the claim.

       IT IS ON THE SAME BOARD AS THE CONTENT. A skeleton laid out to
       different metrics than the thing it stands in for is a placeholder
       that has to be re-read once the data lands, and the reflow when it
       does is the exact jolt the skeleton existed to prevent. -->
  <div class="board">
  <!-- THE SKELETON IS THE PANEL, TRACK FOR TRACK. It stands in for the two
       control STRIPS and the anchor beneath them, which is what actually
       lands — a placeholder laid out to different metrics than the thing it
       replaces has to be re-read once the data arrives, and the reflow when
       it does is the exact jolt the skeleton existed to prevent. Same
       `.segs`, same `.strip` of `.cell` rungs, same `.anchor` at the foot. -->
  <div class="segs" aria-hidden="true">
    {#each [0, 1, 2] as i (i)}
      <span class="seg"><span class="skel line" style="width:{54 + i * 9}px"></span></span>
    {/each}
  </div>
  <!-- THE FIRST CELL IS REAL WHILE THE REST ARE PLACEHOLDERS, and the count
       is unchanged: six cells, one of them the feed rung and five standing
       in. A read that is taking a long time is a state in which the operator
       may well want a different feed, and a skeleton cannot be pressed.

       SO THE SECTION IS NOT `aria-hidden` ANY MORE — it holds a live control
       — and the attribute moved onto the five placeholder cells, which is
       where it belonged: a `.skel` line has nothing to announce. -->
  <!-- THE `QUERY` LEAD IS GONE AND THE `aria-label` CARRIES IT.
       Every control under it is labelled — BROKER FEED, UNIVERSE, INSTRUMENT,
       SEGMENT, TIMEFRAME, the dates and the times — so the word named a group
       a reader had already identified from its contents, and spent a line of
       the fold doing it. The section is still announced as "Query" to anything
       reading the document rather than looking at it, which is the half of
       the label that was carrying information. -->
  <section class="strip" aria-label="Query">
    {@render feedRung(db)}
    {#each [0, 1, 2, 3, 4] as i (i)}
      <div class="field" aria-hidden="true">
        <span class="skel line" style="width:{44 + ((i * 11) % 26)}px"></span>
        <span class="skel" style="width:{96 + ((i * 17) % 40)}px;height:18px;border-radius:5px"
        ></span>
        <span class="skel line" style="width:{56 + ((i * 9) % 24)}px"></span>
      </div>
    {/each}
  </section>
  <section class="strip sub" aria-hidden="true">
    <span class="lead">Contract</span>
    {#each [0, 1] as i (i)}
      <div class="field mcell">
        <span class="skel line" style="width:{40 + i * 14}px"></span>
        <span class="skel" style="width:{110 + i * 20}px;height:18px;border-radius:5px"></span>
        <span class="skel line" style="width:{64 + i * 10}px"></span>
      </div>
    {/each}
  </section>
  <div class="anchor" aria-hidden="true">
    <span class="skel line" style="width:150px;height:19px"></span>
    <span class="px"><span class="skel line" style="width:78px;height:19px"></span></span>
    <p class="note quiet"><span class="skel line" style="width:64%"></span></p>
  </div>
  <div class="tbl">
    <div class="tbl-scroll">
      <div class="tbl-inner">
        <div class="thead">
          {#each COLS as c, i (i)}
            <div class="th" class:numh={c.num} class:inst={i === 0}>{c.label}</div>
          {/each}
        </div>
        <div>
          {#each SKELETON as i (i)}
            <div class="trow skelrow" class:odd={i % 2 === 1}>
              <span class="cell inst"><span class="skel line" style="width:{52 + ((i * 7) % 30)}%"></span></span>
              <span class="cell"><span class="skel line" style="width:80%"></span></span>
              <span class="cell"><span class="skel line" style="width:60%"></span></span>
              <span class="cell"><span class="skel line" style="width:74%"></span></span>
              <span class="cell"><span class="skel line" style="width:52%"></span></span>
              <span class="cell"><span class="skel line" style="width:66%"></span></span>
              <span class="cell"><span class="skel line" style="width:88%"></span></span>
              <span class="cell"><span class="skel line" style="width:50%"></span></span>
              <span class="cell"><span class="skel line" style="width:50%"></span></span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
  <p class="loadnote" role="status">Reading <code>/store.json?feed={feeds.active ?? ''}</code>…</p>
  </div>
{/snippet}

<style>
  /* Also in the shell, +page.svelte, where its reasons are written. */
  .board {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
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
  /* Also in DbSummary.svelte, where its reasons are written. */
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
  /* Also in DbControls.svelte, where its reasons are written. */
  .field :global(.pmenu .plist input[type='radio']) {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    padding: 0;
    opacity: 0;
    pointer-events: none;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .field :global(.pmenu .plist label:has(input[type='radio']) .pwhy) {
    padding-left: 0;
    order: 1;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
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
  /* Also in DbControls.svelte, where its reasons are written. */
  .field :global(.pmenu .plist label:has(input[type='radio']:focus-visible)) {
    outline: 2px solid var(--acc);
    outline-offset: -2px;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .field :global(.pmenu .plist label.pdis)::after {
    content: none;
  }
  .field :global(.pmenu .plist input[type='checkbox']) {
    width: 14px;
    height: 14px;
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
  /* THE STRIP'S NAME IS A HEADING ABOVE IT, NOT A GUTTER BESIDE IT. As a flex
     child with a right hairline it was a tenth column competing with the nine
     rungs for the one line. `1 / -1` takes the full track count whatever
     `auto-fit` resolves to — the same rule /ingest uses for `.pk.wide`, and
     the reason it is `-1` rather than `span 9` is that the last line is
     wherever the window says it is. */
  .lead {
    grid-column: 1 / -1;
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    /* A SECTION LABEL THAT LOOKS LIKE ONE. Ten-pixel grey caps on a dark panel
       read as a note about the panel rather than as the name of it, so a reader
       scanning for structure finds no structure. The mark gives the label a
       left edge to start from and a colour that says "this is a heading"
       without adding a rule across the panel — which would be a second horizontal
       line under a panel that already has a border. It is the accent because
       the accent means "the console is speaking" everywhere else on this page. */
    display: flex;
    align-items: center;
    gap: var(--s3);
  }
  .lead::before {
    content: '';
    flex: none;
    width: 3px;
    height: 0.85em;
    border-radius: 2px;
    background: var(--acc);
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .strip .field {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-width: 0;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .strip .field > span:first-of-type {
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
  /* Also in DbControls.svelte, where its reasons are written. */
  .strip .field.mcell {
    min-width: 0;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .strip .field :global(.pbtn) {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .strip .field :global(.din) {
    width: 100%;
    max-width: none;
    font-size: var(--fs-base);
    padding: 11px 13px;
    border-radius: 9px;
  }
  /* Also in DbSummary.svelte, where its reasons are written. */
  .anchor {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    padding: 0 var(--s2);
  }
  .px {
    margin-left: auto;
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    min-width: 0;
  }

  /* ---- THE COUNTED LINE, AND IT IS WHAT THE SIX TILES WERE ------------
     Bars, complete, bars missing, coverage and months — the same figures with
     the same sub-clauses and the same flash, on one rule under the anchor
     instead of in six boxes above the fold. The sixth is the `.px` above. */
  .anchor .note {
    width: 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--s2) var(--s4);
    margin: 0;
    padding-top: var(--s4);
    border-top: 1px solid var(--line);
    font-size: var(--fs-xs);
    color: var(--faint);
  }

  .loadnote {
    padding: 0 var(--s2);
    color: var(--faint);
    font-size: var(--fs-xs);
    flex: none;
  }
  code {
    font-family: var(--mono);
    font-size: 0.94em;
    color: var(--ink-2);
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
  .tbl-inner {
    min-width: 1064px;
    position: relative;
  }
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
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
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
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
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
  .th.inst,
  .trow .cell.inst {
    position: sticky;
    left: 0;
    z-index: 2;
    background: inherit;
  }
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
  .th.inst {
    background: linear-gradient(180deg, var(--panel-2), var(--bg-2));
  }
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
  .tbl-scroll {
    --seam: transparent;
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
  .trow {
    position: absolute;
    left: 0;
    right: 0;
    height: var(--dbrow);
    border-bottom: 1px solid var(--line-soft);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    background: var(--bg-2);
  }
  .trow.odd {
    background: var(--panel-2);
  }
  .trow:hover {
    background: color-mix(in srgb, var(--acc) 7%, var(--bg-2));
  }
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
  .trow .cell.inst {
    box-shadow: inset 3px 0 0 transparent;
  }
  .trow .cell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Also in DbCoverageGrid.svelte, where its reasons are written. */
  .th.numh {
    font-variant-numeric: tabular-nums;
  }
  .inst {
    display: flex;
    align-items: center;
    gap: var(--s3);
  }
  .skelrow {
    position: static;
    align-items: center;
    background: none;
    cursor: default;
  }
  @media (prefers-reduced-motion: no-preference) {
    /* Also in DbControls.svelte, where its reasons are written. */
    .strip .field::after {
      transition: transform var(--d-state) var(--ease-out);
    }
    /* Also in DbCoverageGrid.svelte, where its reasons are written. */
    .trow {
      transition: background-color var(--d-hover) var(--ease-out);
    }
    .th.inst::after,
    .trow .cell.inst::after {
      transition: background-color var(--d-state) var(--ease-out);
    }
  }
</style>
