<script module>
  /**
   * /db — THE SEARCH BOX (WP-51; D-3332, D-3337; DB-001, DB-040, DB-G97)
   *
   * ONE box that finds anything this feed's store holds, by any name a reader
   * would type for it:
   *
   * - a vendor ticker: GDFL's `NIFTY28MAR1911500CE` (day form) and
   *   `ACC18DEC1500CE` (monthly form), and Kite's weekly `NIFTY1932811500CE`
   *   (UNVERIFIED form, named on every hit it yields);
   * - natural text: `NIFTY 11500 CE`, `NIFTY 11,500 CE`, `nifty 28/03/2019
   *   11500 call`, `BANKNIFTY 28MAR19 26500 PE`;
   * - a strike or an expiry alone: `11500`, `2019-03-28`, `28 Mar 2019`,
   *   `28MAR19`, `NIFTY Mar 2019`;
   * - an index by any of its names (`/aliases.json`, D-3332): `NIFTY 50`,
   *   `NIFTY BANK`, `FINNIFTY`, `MIDCPNIFTY`, `NIFTYNXT50`, `INDIA VIX`;
   * - an old symbol (sourced symbol changes): `ZOMATO` finds ETERNAL, "formerly
   *   ZOMATO until 08 Apr 2025"; `TATAMOTORS` finds TMPV and the options GDFL
   *   holds under TATAMOTORS; `IIFLWAM` finds 360ONE;
   * - any part of a name (`BANK` finds BANKNIFTY, BANKBARODA, AXISBANK, …).
   *
   * Results are GROUPED BY UNDERLYING, so NESTLEIND's 11,500 CE is never drawn
   * under NIFTY's. PICKING A HIT SETS THE WHOLE SELECTION IN ONE STEP — the
   * underlying (an exact key, D-3337 item 1), the segment, the expiry, the
   * strike and the side — through WP-50's selection state, on this feed (the
   * census searched is this feed's own). What the box TYPES never filters the
   * grid: only a picked hit does (D-3337 item 1, DB-001), so typing `NIFT`
   * lists NIFTY, NIFTYIT and NIFTY100 without blending their rows.
   *
   * The indexes and the search are `$lib/db/search.js` and
   * `$lib/db/contract-parse.js` (plain modules, driven under `node --test` by
   * `web/tests/db-search.test.js`); this file holds the state, the effects
   * that build the indexes, the selection setter and the markup.
   *
   * # Cost, and how it is measured on the page
   *
   * The catalog is folded once per census in yielding chunks of
   * `CATALOG_CHUNK` rows; the names once per catalog and alias table. A
   * keystroke is one `search` call, bounded by the names and by `MAX_COUNT`,
   * never by the census. Each search is recorded as a `db-search` performance
   * measure (its compute) and each keystroke as `db-search-keystroke` (input to
   * the list drawn), so `performance.getEntriesByName('db-search-keystroke')`
   * in the browser gives the p50 and p99 the plan asks for; the list's foot
   * prints the running p50 and p99 of this page's own keystrokes.
   */
  import { tick } from 'svelte';
  import { MAX_PREFIX, buildCatalog, buildNames, search, selectionOf, segmentName, windowOf } from '$lib/db/search.js';
  import { yieldToPage } from '$lib/database-preparation.js';
  import { ask } from '$lib/ask.js';
  import { fmt } from '$lib/db/notes-labels.js';
  /** @typedef {import('$lib/db/decoration.js').DecoRow} DecoRow */
  /** @typedef {import('$lib/db/search.js').Hit} Hit */

  /**
   * THE ALIAS TABLE, READ ONCE PER PAGE LOAD. `/aliases.json` is rendered once
   * per server process (`crates/api/src/aliases.rs`), so one read serves every
   * visit to /db until the page is reloaded. A failed read is not cached: the
   * next mount asks again.
   * @type {Promise<any> | null}
   */
  let aliasRead = null;
  function readAliases() {
    if (aliasRead === null) {
      aliasRead = ask('/aliases.json')
        .then(async (r) => {
          if (!r.ok) throw new Error(`/aliases.json answered HTTP ${r.status}`);
          const body = await r.json();
          if (!body || !Array.isArray(body.index) || !Array.isArray(body.symbol_changes)) {
            throw new Error('/aliases.json answered without its index and symbol_changes tables');
          }
          return body;
        })
        .catch((why) => {
          aliasRead = null;
          throw why;
        });
    }
    return aliasRead;
  }

  /** The last keystroke timings, in milliseconds, for the list's foot. */
  const KEY_SAMPLES = 256;

  /**
   * The p-th percentile of `xs` (nearest rank), or `null` for none.
   * @param {number[]} xs @param {number} p
   */
  function percentile(xs, p) {
    if (xs.length === 0) return null;
    const s = [...xs].sort((a, b) => a - b);
    return s[Math.min(s.length - 1, Math.max(0, Math.ceil((p / 100) * s.length) - 1))];
  }

  /**
   * THE SEARCH — its indexes, its answer, the combobox's own state, the
   * selection setter and the page-wide keyboard shortcuts.
   *
   * Creates the effects that build the catalog and read the alias table; the
   * shell calls it in the position the page's search state always held.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function searchState(db) {
    /* ======================================================================
       THE CENSUS PREFIX SEAM, KEPT FOR ITS TEST AND READ BY NOTHING HERE
       ----------------------------------------------------------------------
       `index` and `textMatched` were the old box's whole search: a prefix
       match over four tokens of every census row, the match that made `NIFTY`
       blend NIFTY100 (DB-001) and found no ticker, natural name or expiry
       (DB-G97). The box no longer reads them and nothing on the page does (a
       `$derived` nobody reads is never computed). They stay, unpublished,
       because `web/tests/database-page-fixture.js` (WP-52's) cuts
       `textMatched` out of this file by name to drive the prefix buckets
       `DbCensusBanner` (WP-56's) still builds; retiring them is those
       owners' change, named in WP-51's report. */
    const index = $derived(db.prepared.input === db.rows ? db.prepared.body?.prefix ?? new Map() : new Map());

    const textMatched = $derived.by(() => {
      // Empty text uses the selected universe. Nonempty text intersects its
      // saved full-store bucket with the same membership, never widening scope.
      if (!db.typed) return db.universed;
      /** @type {DecoRow[]} */
      const seed = index.get(db.typed.slice(0, MAX_PREFIX)) ?? [];
      if (db.typed.length <= MAX_PREFIX && db.universeNames === null) return seed;
      /* THE SAME NORMALISATION AS THE INDEX ABOVE, for the same reason: past four
         characters this is the probe, and a bucket cut one way cannot be filtered
         the other. */
      return seed.filter(
        (r) => (db.universeNames === null || db.universeNames.has(r.underlying)) &&
          (db.typed.length <= MAX_PREFIX || r.instrument.toUpperCase().startsWith(db.typed) ||
          r.sym.toUpperCase().startsWith(db.typed) ||
          r.underlying.toUpperCase().startsWith(db.typed) ||
          r.month.toUpperCase().startsWith(db.typed))
      );
    });

    /** @type {HTMLInputElement | null} */
    let searchEl = $state(null);

    /* ======================================================================
       THE INDEXES
       ====================================================================== */

    /** @type {import('$lib/db/search.js').Catalog | null} */
    let catalog = $state.raw(null);
    /** Census rows the catalog is being folded from, `0` when idle. */
    let building = $state(0);
    /** Why the catalog could not be built, or `''`. */
    let catalogWhy = $state('');
    /** The `/aliases.json` body, or `null` until (or unless) it is read. */
    let aliases = $state.raw(/** @type {any} */ (null));
    /** Why the alias table is not in force, or `''`. */
    let aliasWhy = $state('');

    /* THE CATALOG FOLLOWS THE CENSUS: one fold per decorated census, in
       yielding chunks, cancelled when the census moves on before it is done.
       Until it lands the box says it is indexing rather than answering "no
       match" over an index that does not exist yet (`CLAUDE.md` §4). */
    $effect(() => {
      const rows = db.deco;
      const window = windowOf(db.activeFeed);
      if (!rows || rows.length === 0) {
        catalog = null;
        building = 0;
        return;
      }
      const controller = new AbortController();
      /* THE OLD CATALOG GOES FIRST: a census that moved (another feed, a
         refresh) must not be answered from the one before it while the new
         fold runs. */
      catalog = null;
      building = rows.length;
      catalogWhy = '';
      buildCatalog(rows, { signal: controller.signal, yieldControl: yieldToPage, window })
        .then((cat) => {
          if (controller.signal.aborted) return;
          catalog = cat;
          building = 0;
        })
        .catch((why) => {
          if (controller.signal.aborted) return;
          catalog = null;
          building = 0;
          catalogWhy = String(why?.message ?? why);
        });
      return () => controller.abort();
    });

    $effect(() => {
      let live = true;
      readAliases().then(
        (body) => {
          if (!live) return;
          aliases = body;
          aliasWhy = '';
        },
        (why) => {
          if (live) aliasWhy = String(why?.message ?? why);
        }
      );
      return () => {
        live = false;
      };
    });

    const names = $derived(catalog ? buildNames(catalog, aliases, aliasWhy || null) : null);

    /* ======================================================================
       THE ANSWER — one search per keystroke, measured
       ====================================================================== */

    /** Keystroke timings of this page, newest last (not state: read by the foot only). */
    /** @type {number[]} */
    const keySamples = [];
    let keyStats = $state({ n: 0, p50: /** @type {number | null} */ (null), p99: /** @type {number | null} */ (null) });

    const answer = $derived.by(() => {
      const text = String(db.query ?? '');
      if (!catalog || !names || text.trim() === '') return null;
      const t0 = performance.now();
      const a = search(catalog, names, text, { today: db.today, archiveTo: db.archiveTo });
      const t1 = performance.now();
      try {
        performance.measure('db-search', { start: t0, end: t1, detail: { text, total: a.total } });
      } catch {
        /* A browser without the options form of `measure` still gets the
           figure on the list's foot; nothing else depends on the entry. */
      }
      return { ...a, ms: t1 - t0 };
    });

    /** Every drawn hit in list order — what ↑ ↓ walk. */
    const flat = $derived(answer ? answer.groups.flatMap((g) => g.hits) : []);

    /* ======================================================================
       THE COMBOBOX
       ----------------------------------------------------------------------
       `role="combobox"` with `aria-autocomplete="list"`, `aria-expanded` and
       `aria-controls` naming a real listbox; the options are not tab stops,
       `aria-activedescendant` names the highlighted one, ↑ ↓ Enter are handled
       on the field and Escape in `onWindowKey`, so ONE place decides what
       Escape shuts first.
       ====================================================================== */

    /** Is the suggestion listbox showing? Never true with an empty box. */
    let findOpen = $state(false);
    /** Which hit is highlighted, or −1 for none. An INDEX into `flat`. */
    let findCursor = $state(-1);
    /** What the last pick set, said under the box; `null` before any. */
    let committed = $state(/** @type {{label: string, seg: string, notes: string[]} | null} */ (null));

    const findShown = $derived(findOpen && String(db.query ?? '').trim() !== '');

    /**
     * THE SELECTION SETTER: one hit becomes every rung it names, in cascade
     * order, through WP-50's selection state. `selectionOf` (search.js)
     * computes the values; here they are assigned.
     *
     * The rungs a hit does not name are set to what lets it show: the holes
     * toggle off; the universe back to Everything when it would hide the
     * underlying and the window toggle on when the target is held only
     * outside the operator's window (both said in `committed.notes`); the
     * moneyness pick cleared (a strike is chosen instead); the day window
     * returned to empty so it re-seeds on the new selection's held span.
     *
     * @param {Hit} hit
     */
    function commitHit(hit) {
      if (!catalog) return;
      const sel = selectionOf(hit, catalog, {
        timeframe: db.timeframe,
        universe: db.universe,
        universeNames: db.universeNames,
        oow: db.oow,
        universeLabel: db.chosenUniverse?.label
      });
      db.universe = sel.universe;
      db.oow = sel.oow;
      db.holesOnly = false;
      db.month = '';
      db.filter = sel.filter;
      db.kind = sel.kind;
      db.timeframe = sel.timeframe;
      db.expiry = sel.expiry;
      db.strikePick = sel.strikes;
      db.mnyPick = new Set();
      db.side = sel.side;
      db.fromDay = '';
      db.toDay = '';
      db.daySeed = { from: '', to: '' };
      committed = { label: hit.label, seg: hit.seg, notes: sel.notes };
      findOpen = false;
      findCursor = -1;
    }

    /** The input changed: open the list and time the keystroke to the drawn list. */
    function onFindInput() {
      findOpen = true;
      findCursor = -1;
      const t0 = performance.now();
      void tick().then(() => {
        const t1 = performance.now();
        try {
          performance.measure('db-search-keystroke', { start: t0, end: t1 });
        } catch {
          /* See `answer`: the foot carries the figure either way. */
        }
        keySamples.push(t1 - t0);
        if (keySamples.length > KEY_SAMPLES) keySamples.shift();
        keyStats = { n: keySamples.length, p50: percentile(keySamples, 50), p99: percentile(keySamples, 99) };
      });
    }

    /**
     * ↑ ↓ move the highlight, Enter commits it (or the first hit when none is
     * highlighted). Nothing else is intercepted, so every editing key edits.
     *
     * @param {KeyboardEvent} e
     */
    function onFindKey(e) {
      if (e.altKey || e.ctrlKey || e.metaKey) return;
      const n = flat.length;
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        if (!findShown) {
          findOpen = true;
          return;
        }
        if (n === 0) return;
        const d = e.key === 'ArrowDown' ? 1 : -1;
        /* WRAPS IN BOTH DIRECTIONS: `-1 % n` is `-1` in JavaScript. */
        findCursor = findCursor < 0 ? (d > 0 ? 0 : n - 1) : (findCursor + d + n) % n;
        e.preventDefault();
        return;
      }
      if (e.key === 'Enter' && findShown && n > 0) {
        commitHit(flat[findCursor >= 0 && findCursor < n ? findCursor : 0]);
        e.preventDefault();
      }
    }

    /* `/` FOCUSES THE SEARCH, from anywhere on the page that is not already a
       text field — the shortcut every terminal and every code host uses. */
    /** @param {KeyboardEvent} e */
    function onWindowKey(e) {
      const t = e.target;
      const typing =
        t instanceof HTMLElement &&
        (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
      /* AN OPEN MENU OWNS ESCAPE: a menu left on screen after the key that
         dismisses things is a control the reader has to click to be rid of. */
      if (db.drop !== null && e.key === 'Escape') {
        db.drop = null;
        return;
      }
      /* THE SEARCH LIST SHUTS BEFORE THE ROW DRAWER DOES: Escape dismisses the
         nearest thing, and one function decides the order. */
      if (findOpen && e.key === 'Escape') {
        findOpen = false;
        findCursor = -1;
        return;
      }
      if (e.key === '/' && !typing && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        searchEl?.focus();
        searchEl?.select();
      } else if (e.key === 'Escape' && db.openKey) {
        db.openKey = null;
        db.scroller?.focus();
      }
    }

    /** A press outside the search closes its list. @param {FocusEvent} e */
    function onFindBlur(e) {
      const next = e.relatedTarget;
      if (next instanceof Node && /** @type {HTMLElement} */ (e.currentTarget).closest('.search')?.contains(next)) return;
      findOpen = false;
      findCursor = -1;
    }

    Object.defineProperties(db, {
      searchEl: { get: () => searchEl, set: (v) => (searchEl = v) },
      searchCatalog: { get: () => catalog },
      searchNames: { get: () => names },
      searchBuilding: { get: () => building },
      searchWhy: { get: () => catalogWhy },
      aliasWhy: { get: () => aliasWhy },
      aliasTable: { get: () => aliases },
      findAnswer: { get: () => answer },
      findFlat: { get: () => flat },
      findOpen: { get: () => findOpen, set: (v) => (findOpen = v) },
      findCursor: { get: () => findCursor, set: (v) => (findCursor = v) },
      findShown: { get: () => findShown },
      findCommitted: { get: () => committed },
      findKeyStats: { get: () => keyStats },
      commitHit: { get: () => commitHit },
      onFindInput: { get: () => onFindInput },
      onFindKey: { get: () => onFindKey },
      onFindBlur: { get: () => onFindBlur },
      onWindowKey: { get: () => onWindowKey }
    });
  }

  /** @param {number | null} ms */
  const msText = (ms) => (ms === null ? '—' : ms < 1 ? `${ms.toFixed(2)} ms` : `${ms.toFixed(1)} ms`);
  /** @param {Hit} hit */
  const kindWord = (hit) => (hit.kind === 'instrument' ? 'instrument' : hit.kind === 'expiry' ? 'expiry' : 'contract');
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

<!-- THE SEARCH. One field, spanning two tracks of the query strip, with its
     grouped list under it. The placeholder names what it reads; the clause
     under it says what the last pick set, or why the box cannot answer yet. -->
<div class="field search">
  <label class="lab" for="db-search" title="Search this feed's store by ticker, name, alias, old symbol, strike or expiry. Press / to focus.">
    Search <span class="key" aria-hidden="true">/</span>
  </label>
  <div class="box">
    <input
      id="db-search"
      class="ctl"
      type="search"
      autocomplete="off"
      spellcheck="false"
      placeholder="NIFTY28MAR1911500CE · NIFTY 11500 CE · 28 Mar 2019 · FINNIFTY · ZOMATO"
      role="combobox"
      aria-autocomplete="list"
      aria-expanded={db.findShown}
      aria-controls="db-search-list"
      aria-activedescendant={db.findShown && db.findCursor >= 0 ? `db-hit-${db.findCursor}` : undefined}
      bind:this={db.searchEl}
      bind:value={db.query}
      oninput={db.onFindInput}
      onkeydown={db.onFindKey}
      onfocus={() => (db.findOpen = true)}
      onblur={db.onFindBlur}
    />
    {#if db.findShown}
      <div id="db-search-list" class="list" role="listbox" aria-label="Search results, grouped by underlying" tabindex="-1">
        {#if db.searchBuilding > 0 && !db.searchCatalog}
          <p class="state" role="status">Indexing {fmt(db.searchBuilding)} instrument-month(s) of this feed's census…</p>
        {:else if db.searchWhy}
          <p class="state warn" role="alert">The search index could not be built: {db.searchWhy}</p>
        {:else if !db.findAnswer}
          <p class="state" role="status">Reading the census…</p>
        {:else if db.findAnswer.total === 0}
          <p class="state" role="status">{db.findAnswer.why}</p>
        {:else}
          {#each db.findAnswer.groups as g, gi (g.u)}
            {@const base = db.findAnswer.groups.slice(0, gi).reduce((/** @type {number} */ n, /** @type {any} */ x) => n + x.hits.length, 0)}
            <div class="group" role="group" aria-labelledby={`db-group-${gi}`}>
              <div class="ghead" id={`db-group-${gi}`}>
                <span class="gname">{g.u}</span>
                {#if g.total > g.hits.length}<span class="gmore">{fmt(g.hits.length)} of {fmt(g.total)}</span>{/if}
                {#each g.notes as n (n)}<span class="gnote" title={n}>{n}</span>{/each}
              </div>
              {#each g.hits as h, hi (h.id)}
                {@const at = base + hi}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <div
                  id={`db-hit-${at}`}
                  class="opt"
                  class:on={db.findCursor === at}
                  role="option"
                  aria-selected={db.findCursor === at}
                  tabindex="-1"
                  title={h.title}
                  onpointerdown={(e) => e.preventDefault()}
                  onclick={() => db.commitHit(h)}
                  onpointerenter={() => (db.findCursor = at)}
                >
                  <span class="kind">{kindWord(h)}</span>
                  <span class="name">{h.label}</span>
                  <span class="detail">{h.detail}</span>
                </div>
              {/each}
            </div>
          {/each}
          <p class="foot">
            {fmt(db.findAnswer.drawn)} of {db.findAnswer.capped ? 'more than ' : ''}{fmt(db.findAnswer.total)} match(es) drawn{#if db.findAnswer.hiddenGroups > 0}, {fmt(db.findAnswer.hiddenGroups)} more underlying(s) not drawn — type more to narrow{/if}.
            {#if db.findAnswer.forms.length}Read as {db.findAnswer.forms.join(', ')}.{/if}
            Search {msText(db.findAnswer.ms)}{#if db.findKeyStats.n > 0}; keystroke p50 {msText(db.findKeyStats.p50)}, p99 {msText(db.findKeyStats.p99)} over {fmt(db.findKeyStats.n)}{/if}.
          </p>
        {/if}
      </div>
    {/if}
  </div>
  {#if db.aliasWhy}
    <span class="note warn">Index names and old symbols are not searched: {db.aliasWhy}</span>
  {:else if db.findCommitted}
    <span class="note quiet" title={[db.findCommitted.label, segmentName(db.findCommitted.seg), ...db.findCommitted.notes].join('. ')}>
      Opened {db.findCommitted.label}{db.findCommitted.seg ? ` · ${segmentName(db.findCommitted.seg)}` : ''}{#each db.findCommitted.notes as n (n)} · {n}{/each}
    </span>
  {/if}
</div>

<style>
  /* TWO TRACKS, NOT ONE: a ticker and its natural spelling are 20-odd
     characters, and the hit list under it carries a name and a counted
     clause per row. One track is 180-220px (DbControls' `.strip`), which
     clips the placeholder that tells a reader what the box reads. */
  .search {
    grid-column: span 2;
    position: relative;
    min-width: 0;
  }
  @media (max-width: 640px) {
    .search {
      grid-column: 1 / -1;
    }
  }
  .lab .key {
    display: inline-block;
    margin-left: var(--s2);
    padding: 0 var(--s2);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    color: var(--dim);
  }
  .box {
    position: relative;
    min-width: 0;
  }
  .box input.ctl {
    width: 100%;
    box-sizing: border-box;
    font-family: var(--mono);
    font-size: var(--fs-sm);
    color: var(--ink);
  }
  .box input.ctl::placeholder {
    color: var(--faint);
  }
  .box input.ctl:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  /* THE LIST FLOATS OVER THE STRIP, exactly as wide as the field (two
     tracks), and scrolls inside a viewport-bounded height: a list that pushed
     the grid down would move the thing a reader is about to look at. Wider
     than the field it ran off the right edge whenever the field sat in the
     strip's last tracks (measured at an 800px viewport); a row's clause wraps
     instead. */
  .list {
    position: absolute;
    z-index: 30;
    top: calc(100% + var(--s2));
    left: 0;
    right: 0;
    max-height: min(60vh, 560px);
    overflow-y: auto;
    padding: var(--s3) 0;
    background: var(--panel);
    border: 1px solid var(--line-hard);
    border-radius: var(--r3, 8px);
    box-shadow: var(--e3);
  }
  .state {
    margin: 0;
    padding: var(--s4) var(--s5);
    font-size: var(--fs-xs);
    color: var(--dim);
    white-space: normal;
  }
  .state.warn {
    color: var(--warn);
  }
  .group + .group {
    border-top: 1px solid var(--line-soft);
    margin-top: var(--s2);
    padding-top: var(--s2);
  }
  .ghead {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--s2) var(--s4);
    padding: var(--s2) var(--s5);
  }
  .gname {
    font-family: var(--mono);
    font-size: var(--fs-xs);
    font-weight: var(--w-bold, 700);
    color: var(--ink-hi);
  }
  .gmore {
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--faint);
  }
  .gnote {
    flex-basis: 100%;
    font-size: var(--fs-micro);
    color: var(--dim);
    white-space: normal;
  }
  .opt {
    display: grid;
    grid-template-columns: 76px minmax(0, 1fr);
    grid-template-rows: auto auto;
    column-gap: var(--s4);
    padding: var(--s2) var(--s5);
    cursor: pointer;
  }
  .opt.on {
    background: var(--acc-soft);
    box-shadow: inset 3px 0 0 var(--acc);
  }
  .opt .kind {
    grid-row: 1 / span 2;
    align-self: center;
    font-family: var(--mono);
    font-size: var(--fs-micro);
    text-transform: uppercase;
    color: var(--faint);
  }
  /* THE NAME WRAPS RATHER THAN CLIPS: its tail is the strike and the side,
     the only part that tells one contract from the next (the census grid
     learned this, `decoration.js` on `head`/`sym`). */
  .opt .name {
    font-family: var(--mono);
    font-size: var(--fs-sm);
    color: var(--ink);
    white-space: normal;
    overflow-wrap: anywhere;
  }
  .opt .detail {
    font-size: var(--fs-micro);
    color: var(--dim);
    white-space: normal;
    overflow-wrap: anywhere;
  }
  .foot {
    margin: var(--s3) 0 0;
    padding: var(--s3) var(--s5) 0;
    border-top: 1px solid var(--line-soft);
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--faint);
    white-space: normal;
  }
</style>
