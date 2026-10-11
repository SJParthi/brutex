<script module>
  /**
   * /db — THE CENSUS CLIENT, ITS LOADING SKELETON AND ITS BANNER
   *
   * The page's reading of `/store.json` (`censusState`), what the page draws
   * while that reading is in flight (`censusLoading`, the skeleton the shell
   * renders in its loading branch), and THE BANNER (WP-56, D-3327), mounted at
   * the top of the pane above every state. The banner says:
   *
   *   - when the census on screen was read, and at which generation — the
   *     page is a snapshot, and said nowhere that it was (DB-022);
   *   - that the store moved after that read, from a header-only HEAD of the
   *     census every few seconds, and what moved for the instrument on screen
   *     (`/census/facets.json?since=`, O(changes)), with Refresh (DB-022);
   *   - a census that is `capped` or `behind` its import, in the server's own
   *     sentence, with its capacity, its headroom and the import days it
   *     refused (DB-G03, DB-G11, DB-G60, DB-G78);
   *   - a census this page decoded or prepared on its own thread because no
   *     worker could, and why (DB-G84).
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
  import { untrack } from 'svelte';
  import { page } from '$app/state';
  import { store, syncStore, wholeStore, surveyStores, survey, refreshStore, watchCensus, checkCensusHead, facets } from '$lib/store.svelte.js';
  import { decodeDb, DB_DEFAULTS } from '$lib/urlstate.js';
  import { linkOfKey, DEFAULT_INSTRUMENT } from '$lib/db/selection-model.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { denominators } from '$lib/completeness.js';
  import { decorateRow, instrumentDescription } from '$lib/db/decoration.js';
  import { instrumentMemo, databasePreparation } from '$lib/database-preparation.js';
  import { yieldTask, inSlices } from '$lib/census-worker.js';
  import { censusMoved, sinceFor, censusLines, movedLines, refreshText, searchRefreshes } from '$lib/store-census.js';
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
    const live = $derived(store.feed === feeds.active ? store.rows : []);
    /** @type {{input:StoreRow[]|null,body:{rows:DecoRow[],observed:ReturnType<typeof denominators>,prefix:Map<string,DecoRow[]>,heldNames:number|null}|null,why:string|null,scope:import('$lib/store-census.js').CensusScope|null,feed:string|null}} */
    let prepared = $state.raw({ input: null, body: null, why: null, scope: null, feed: null });
    /* ONE INSTRUMENT'S ROWS STAY ON SCREEN WHILE THE WHOLE CENSUS IS PREPARED
       BEHIND THEM (WP-56b). The first paint is drawn from the census rows of
       the instrument on screen (`store.scope`, `readScope`); when the whole
       census then lands, decorating its rows takes about a second on GDFL,
       and blanking the page for it would take away the control the reader
       had just reached for. `behind` is true for exactly that interval: the
       store holds the whole census, the page still shows the instrument's
       prepared rows, and the banner still says so. */
    const behind = $derived(
      prepared.scope !== null && prepared.body !== null && prepared.feed === feeds.active &&
        store.scope === null && live.length > 0 && prepared.input !== live
    );
    const rows = $derived(behind ? /** @type {StoreRow[]} */ (prepared.input) : live);
    /** The scope of the rows ON SCREEN, or null when they are the whole census. */
    const shownScope = $derived(behind ? prepared.scope : store.feed === feeds.active ? store.scope : null);
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
    /* THE INSTRUMENT THE FIRST PAINT IS FOR (WP-56b): the one the address
       bar names, else the page's opening instrument (operator answer 11).
       Read once. The Coverage grid lists every instrument, so a link that
       opens on it asks for the whole census from the start. */
    /** @type {import('$lib/store-census.js').ScopeWanted | null | undefined} */
    let first;
    const firstScope = () => {
      const { sel } = decodeDb(page.url.search);
      if (sel.view && sel.view !== DB_DEFAULTS.view) return null;
      const link = sel.key ? linkOfKey(sel.key) : null;
      return { underlying: sel.sym ?? link?.sym ?? DEFAULT_INSTRUMENT, expiry: sel.exp ?? link?.expiry ?? null };
    };
    $effect(() => {
      if (first === undefined) first = untrack(firstScope);
      syncStore(feeds.active, first);
    });

    /* WHAT NEEDS EVERY ROW ASKS FOR EVERY ROW (WP-56b), and says why in the
       refresh log: the Coverage grid, an instrument other than the one read,
       a selection the rows read do not hold (a segment or an expiry that was
       not listed). Waited 60 ms and re-armed on every change, so a rung the
       seeds are still filling is not mistaken for an empty selection. */
    $effect(() => {
      const scope = store.scope;
      if (!scope || store.feed !== feeds.active || store.state !== 'ready' || preparing || deco.length === 0) return;
      const why = db.view !== DB_DEFAULTS.view
        ? 'the Coverage grid lists every instrument'
        : db.filter !== '' && db.filter !== scope.underlying
          ? `${db.filter} was chosen and only ${scope.underlying} had been read`
          : db.filter !== '' && db.kind !== '' && !db.reconciling && db.matched.length === 0
            ? `the rows read for ${scope.underlying} hold nothing for the selection on screen`
            : null;
      if (why === null) return;
      const timer = setTimeout(() => void wholeStore(why), 60);
      return () => clearTimeout(timer);
    });
    /** A control outside the bar rows was reached for: its lists need every row. */
    const reach = (/** @type {Event} */ event) => {
      if (store.scope === null) return;
      const target = event.target;
      if (target instanceof Element && target.closest('[data-census-rows]')) return;
      void wholeStore('a control outside the bar rows was used');
    };

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

    /* THE PREPARATION — DECORATED ROWS AND DENOMINATORS, NO PREFIX INDEX.
       ----------------------------------------------------------------------
       In slices of 1,024 rows (about 2 ms of decoration each), each followed
       by a real task (`yieldTask`). The old default, `scheduler.yield`,
       resumes AHEAD of timers and input, so a "yielding" preparation still
       froze the page for over two seconds (DB-G84). The census worker parses
       and decodes; decorating there and sending the rows back cost more than
       decorating here in slices (`$lib/census-worker.js` has the figures).

       NO PREFIX TOKENS (DB-B52): every row used to be filed under four
       prefixes of four tokens — 4,586,125 references, the largest bucket
       holding every row, about 8 times the preparation's time and 42 MB of
       heap — for a search box WP-51 replaced and nothing reads. The empty
       map stays the shape `DbSearch.svelte`'s unpublished `index` reads.

       THE NAMES HELD (DB-N07) are counted in the same slices, so the
       Everything universe row counts underlyings like every other row. */
    $effect(() => {
      const input = live;
      const scope = store.feed === feeds.active ? store.scope : null;
      const stated = store.observed;
      const feed = feeds.active;
      const controller = new AbortController();
      // THE WHOLE CENSUS BEHIND ONE INSTRUMENT'S ROWS is prepared without
      // taking those rows off the screen (`behind`, above); every other
      // input replaces what was prepared before it, as it always did.
      const before = untrack(() => prepared);
      const keep = scope === null && input.length > 0 && before.scope !== null && before.body !== null && before.feed === feed;
      if (!keep) prepared = { input, body: null, why: null, scope, feed };
      if (input.length > 0) {
        const describe = instrumentMemo(instrumentDescription);
        const work = scope !== null && stated !== null
          /* ONE INSTRUMENT'S ROWS, AGAINST THE FEED'S OWN DENOMINATORS
             (WP-56b): the fullest row and the key count of each (month,
             rung) are the server's, counted over the whole census, so a
             row's shortfall here is the number the whole census gives.
             The names held are NOT counted from one instrument: `null`. */
          ? (async () => {
            /** @type {DecoRow[]} */
            const out = new Array(input.length);
            await inSlices(input.length, (start, end) => {
              for (let i = start; i < end; i++) out[i] = decorateRow(input[i], stated, describe(input[i].instrument));
            }, controller.signal);
            return { rows: out, observed: stated, prefix: new Map(), heldNames: null };
          })()
          : databasePreparation.read(input, (row, observed) => decorateRow(row, observed, describe(row.instrument)), {
            signal: controller.signal, yieldControl: yieldTask, chunkSize: 1024
          }).then(async (body) => {
            const names = new Set();
            await inSlices(body.rows.length, (start, end) => {
              for (let i = start; i < end; i++) names.add(body.rows[i].underlying);
            }, controller.signal);
            return { ...body, heldNames: names.size };
          });
        void work.then((body) => {
          // Preparation stays outside reactive dependency tracking. Publish
          // only one complete immutable snapshot for this input.
          if (!controller.signal.aborted) prepared = { input, body, why: null, scope, feed };
        }).catch((why) => {
          if (!controller.signal.aborted) prepared = { input, body: null, why: `Database view preparation failed: ${String(why)}`, scope, feed };
        });
      }
      return () => controller.abort();
    });

    /* ======================================================================
       THE SNAPSHOT AND WHAT MOVED SINCE (DB-022)
       ====================================================================== */
    /* THE GENERATION WATCH: a HEAD of the census every few seconds while the
       tab is visible, released when the page goes or the feed changes. */
    $effect(() => watchCensus(feeds.active));

    const moved = $derived(store.feed === feeds.active ? censusMoved({ feed: store.feed, lag: store.lag }, store.head) : null);

    /* WHAT MOVED FOR THE INSTRUMENT ON SCREEN, at O(changes): the facets of
       its underlying in each segment whose part moved, asked `since` the part
       generation this page READ. Asked once per new generation the HEAD
       reports, never per tick. */
    let selectionDelta = $state.raw(/** @type {{key: string, answers: any[]} | null} */ (null));
    $effect(() => {
      const m = moved;
      const underlying = db.picked;
      const feed = store.feed;
      const lag = store.lag;
      if (!m || !underlying || !feed || !lag) {
        selectionDelta = null;
        return;
      }
      // WHICH SEGMENTS TO ASK. A partitioned census names the parts that
      // moved. A single file names none, and the segments it holds are the
      // SERVER'S to state (the root answer, `/census/facets.json?feed=`):
      // this page keeps no list of segments, so a new one needs no change
      // here (WP-56b).
      const key = `${feed}|${underlying}|${m.to}|${m.segments.join(',')}`;
      if (selectionDelta?.key === key) return;
      let live = true;
      const asked = m.segments.length > 0
        ? Promise.resolve({ segments: m.segments, why: null })
        : facets.root(feed).then(
          (root) => (root.ok
            ? { segments: root.body.segments.map((/** @type {any} */ row) => row.segment), why: null }
            : { segments: [], why: root.reason ?? 'the feed\'s segments could not be read' }),
          (why) => ({ segments: [], why: String(why?.message ?? why) }));
      asked.then(({ segments, why }) => (why !== null
        ? [{ ok: false, status: 0, query: { feed, segment: 'every segment', underlying }, reason: `which segments ${feed} holds could not be read (${why})`, body: null }]
        : Promise.all(segments.map((/** @type {string} */ segment) => {
        const since = sinceFor(lag, segment);
        return since === null
          ? Promise.resolve({ ok: false, status: 0, query: { feed, segment, underlying }, reason: 'this census names no generation for the segment, so no delta can be asked', body: null })
          : facets.changesSince({ feed, segment, underlying }, since).catch((why) => ({ ok: false, status: 0, query: { feed, segment, underlying }, reason: String(why?.message ?? why), body: null }));
      })))).then((answers) => {
        if (live) selectionDelta = { key, answers };
      });
      return () => { live = false; };
    });

    /** The banner's lines, from the census headers and the watch. */
    const banner = $derived.by(() => {
      const feed = store.feed;
      // ONE INSTRUMENT'S ROWS KEEP THEIR LINE while the whole census is read
      // and prepared behind them: the page still shows them, so it says so.
      const scope = shownScope;
      const reading = scope !== null && (store.state === 'reading' || behind);
      return feed !== null && feed === feeds.active && (store.state === 'ready' || reading)
        ? censusLines({ feed, at: store.at, lag: store.lag, worker: store.worker, head: store.head, how: store.how,
          scope, scopeWhy: store.scopeWhy, reading })
        : [];
    });
    const movedSaid = $derived(store.feed === feeds.active && moved
      ? movedLines(moved, store.head, store.at, db.picked ?? null, selectionDelta?.answers ?? null)
      : []);

    /** Distinct underlyings held — the unit every universe row counts (DB-N07). */
    const heldNamesNow = $derived(prepared.input === rows ? prepared.body?.heldNames ?? null : null);

    /* A LIVE FILE TOTAL IS NOT HELD AGAINST A SNAPSHOT (DB-022, WP-56b).
       The grid compares the files' totals with the census rows only while
       the watch says the census has NOT moved since this page read it
       (`db.censusMoved === false`). The watch asks every five seconds, so a
       difference may be a commit it has not seen yet: the moment one is
       drawn, the census is asked again, once per difference. If it moved,
       the "store moved" line replaces the warning and says what moved. */
    let disagreeChecked = '';
    $effect(() => {
      const first = db.barDisagree?.[0];
      const feed = store.feed;
      if (!first || !feed || moved !== false) return;
      const key = `${feed}|${first.key}|${first.file}|${first.census}`;
      if (disagreeChecked === key) return;
      disagreeChecked = key;
      void checkCensusHead(feed);
    });

    /* THE REFRESH LOG, FILTERED (WP-56b): how every read of the census
       ended, newest first, from the browser's storage. */
    let refreshFilter = $state('');
    const refreshesShown = $derived(searchRefreshes(store.refreshes, refreshFilter));

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
      deco: { get: () => deco },
      heldNames: { get: () => heldNamesNow },
      censusScope: { get: () => shownScope },
      // WHAT A COUNT OF "THE STORE" IS COUNTED FROM while one instrument's
      // rows are on screen (WP-56b): appended to every sentence that counts
      // rows, so none of them states the store's total from one instrument.
      censusScopeNote: { get: () => (shownScope
        ? ` Counted from the census rows read for ${shownScope.underlying} alone: the whole census is not read yet.`
        : '') },
      censusReach: { get: () => reach },
      readWholeCensus: { get: () => () => wholeStore('the reader asked for the whole census') },
      censusMoved: { get: () => moved },
      censusBanner: { get: () => banner },
      censusMovedSaid: { get: () => movedSaid },
      refreshCensus: { get: () => () => refreshStore() },
      refreshFilter: { get: () => refreshFilter, set: (v) => (refreshFilter = v) },
      refreshesShown: { get: () => refreshesShown }
    });
  }

  export { censusLoading };
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

<!-- THE BANNER. Every line is a fact the census headers or the watch
     stated, in the server's own words where it has them; a line with
     nothing to say is not drawn. `role="status"` for the quiet lines, an
     alert for a census that cannot be called complete. -->
{#if db.censusBanner.length || db.censusMovedSaid.length}
  <section class="cbanner" aria-label="Census" data-census-rows>
    {#each db.censusBanner as line (line.key)}
      <p class="cline {line.tone}" role={line.tone === 'warn' ? 'alert' : 'status'}>{line.text}{#if line.key === 'read' && db.censusScope && store.state === 'ready' && store.scope}{' '}<button class="btn" onclick={() => db.readWholeCensus()}>Read the whole census now</button>{/if}</p>
    {/each}
    {#if db.censusMovedSaid.length}
      <div class="cline moved" role="status">
        <!-- ONE SPACE BETWEEN SENTENCES, IN THE TEXT ITSELF (WP-56b): the
             spans were adjacent, so a reader (and a screen reader, and a
             copy) got "…IST.Nothing changed". -->
        {#each db.censusMovedSaid as line, i (line.key)}{i > 0 ? ' ' : ''}<span>{line.text}</span>{/each}
        {' '}<button class="btn" onclick={() => db.refreshCensus()}>Refresh</button>
      </div>
    {/if}
    <!-- THE REFRESH LOG (WP-56b): closed until asked for; every read of the
         census this browser made, newest first, with a filter over its
         sentences. A log that could not be saved says so. -->
    {#if store.refreshes.length || store.refreshFault}
      <details class="clog">
        <summary>Census reads logged in this browser: {store.refreshes.length}</summary>
        {#if store.refreshFault}<p class="cline warn" role="alert">{store.refreshFault[0].toUpperCase()}{store.refreshFault.slice(1)}.</p>{/if}
        <label class="clog-find">Filter <input type="search" bind:value={db.refreshFilter} placeholder="whole, delta, failed, a feed, a reason…" /></label>
        <p class="cline info" role="status">{db.refreshesShown.length} of {store.refreshes.length} shown, newest first.</p>
        <ol class="clog-list">
          {#each db.refreshesShown.slice(0, 50) as entry (`${entry.at}|${entry.feed}|${entry.kind}`)}
            <li class={entry.kind === 'failed' || (entry.kind === 'whole' && entry.why) ? 'warn' : ''}>{refreshText(entry)}</li>
          {/each}
        </ol>
        {#if db.refreshesShown.length > 50}<p class="cline info">The newest 50 are listed; narrow the filter to see older ones.</p>{/if}
      </details>
    {/if}
  </section>
{/if}


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
  /* ---- THE BANNER ------------------------------------------------------
     Above every state, one line per fact. Quiet lines (`info`) are the
     snapshot stamp and where the census was decoded; `warn` is a census that
     cannot be called complete; `moved` carries the Refresh it offers. */
  .cbanner {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: var(--s1);
    padding: var(--s2) var(--s5);
  }
  .cline {
    margin: 0;
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--faint);
    word-break: break-word;
  }
  .cline.warn {
    color: var(--ink);
    background: var(--warn-soft);
    border: 1px solid var(--warn);
    border-radius: var(--r2);
    padding: var(--s2) var(--s4);
  }
  .cline.moved {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s2) var(--s4);
    color: var(--ink);
    background: var(--info-soft);
    border: 1px solid var(--info);
    border-radius: var(--r2);
    padding: var(--s2) var(--s4);
  }
  .cline.moved .btn {
    margin-left: auto;
  }
  /* The refresh log: closed by default, one line per read. */
  .clog {
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .clog summary {
    cursor: pointer;
  }
  .clog-find {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin: var(--s2) 0;
  }
  .clog-find input {
    flex: 1;
    min-width: 0;
  }
  .clog-list {
    margin: 0;
    padding-left: var(--s5);
    max-height: 30vh;
    overflow-y: auto;
  }
  .clog-list li.warn {
    color: var(--ink);
  }
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
