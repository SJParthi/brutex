<script module>
  /**
   * /db — THE SEARCH BOX
   *
   * The find list, the prefix index and the `/` shortcut. The box itself was
   * removed before WP-00 ("THE SEARCH BOX IS GONE" in DbControls), so this
   * component renders nothing yet; it is mounted where the box stood.
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
   * Owner after WP-00: WP-51 (one search box over tickers, natural names, aliases and expiries).
   */
  import { MAX_PREFIX, FIND_ROWS } from '$lib/db/search.js';
  import { fmt } from '$lib/db/notes-labels.js';
  import { monthLabel } from '$lib/dates.js';
  /** @typedef {import('$lib/db/decoration.js').DecoRow} DecoRow */

  /**
   * THE TEXT SEARCH — the census prefix index, the text-matched rows, the find
   * list and the page-wide keyboard shortcuts.
   *
   * Creates no effect, so where the shell calls it does not change behaviour.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function searchState(db) {
    /* THE BUCKET KEY AND THE PROBE KEY ARE ONE FORM, AND THEY WERE NOT.
       `typed` below is `filter.trim().toUpperCase()`, and these buckets were cut
       from the token EXACTLY AS THE STORE SPELLS IT. Two spellings of one key is
       the silent failure this whole file is written against: the day a store
       writes one instrument, one symbol or one month key in anything but upper
       case, its bucket is filed under a prefix no probe can ever ask for — the
       row is in `windowed`, the table below counts it, and the text box simply
       never finds it. Nothing errors and nothing looks wrong. Normalised on BOTH
       sides here, once per token rather than once per prefix, so the index and
       the box cannot spell the same key two ways.

       `it.sym` is the tail the row DISPLAYS, and it stays in the index on
       purpose: it is a substring of `it.instrument`, so typing `NIFTY` at
       `NSE-INDEX-NIFTY` is a real question. What it is not allowed to be is the
       key anything is COMPARED by — `picked` below resolves back to the store's
       own `instrument` spelling, never to what was typed. */
    // All prefix buckets are prepared in the same cancellable chunks as the
    // rows. Membership changes intersect a bucket rather than rebuilding the
    // whole index; no synchronous multi-million-reference build follows publish.
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
       THE FIND BOX IS A COMBOBOX, AND IT NOW SAYS SO IN THE DOCUMENT
       ----------------------------------------------------------------------
       The box has always offered a list — that is what a prefix index is for —
       and the document said it was a bare text field. `role="combobox"` with
       `aria-autocomplete="list"`, `aria-expanded` and `aria-controls` is the
       WAI-ARIA pattern for exactly this control, and every one of those
       attributes is a PROMISE: `aria-controls` names a listbox, so a listbox has
       to exist and has to be the thing that opens.

       SO THE LISTBOX IS REAL. It is not a decoration bolted on to justify an
       attribute — it is the same enumeration the instrument `Picker` beside it
       shows, cut to what has been typed, and it reaches something the Picker
       cannot: a MONTH. The box has always matched instrument OR month (that is
       what `index` is built over) and only the instruments were ever listed.

       IT COSTS NO SCAN OF THE STORE. Both lists it reads are already built and
       are bounded by the UNIVERSE rather than by the row count —
       `instrumentRows` is one entry per instrument the window holds and
       `monthsAll` is one per month — so a keystroke walks hundreds of entries,
       never 20,516 rows, and never 93,776. The page's O(1)-per-keystroke
       property is about the ROW probe and is untouched: `textMatched` is still
       one Map probe.

       THE KEYBOARD BELONGS TO THE INPUT, which is the pattern's whole point: the
       options are not tab stops, `aria-activedescendant` names the highlighted
       one, and ↑ ↓ Enter are handled on the field. Escape is handled in
       `onWindowKey` beside the menu's, so ONE place decides what Escape shuts
       first and the two cannot fight over it.
       ====================================================================== */

    /** Is the suggestion listbox showing? Never true with an empty box. */
    let findOpen = $state(false);
    /** Which suggestion is highlighted, or −1 for none. An INDEX, not a key. */
    let findCursor = $state(-1);

    /** Instrument keys starting with what is typed — bounded by the universe. */
    const findInstruments = $derived(
      db.typed ? db.instrumentRows.filter((/** @type {any} */ r) => r.key.toUpperCase().startsWith(db.typed)) : []
    );
    /** Month keys starting with what is typed. RAW `YYYY-MM` on both sides. */
    const findMonths = $derived(
      db.typed ? db.monthsAll.filter((/** @type {any} */ [m]) => m.toUpperCase().startsWith(db.typed)) : []
    );

    /**
     * The rows the listbox draws — instruments first, then months.
     *
     * `key` is the store's own spelling and it is what a click writes into the
     * box; `name` is the display form and it is written nowhere else. The month
     * rows are the case that makes this matter: `Sep 2024` is not a key the
     * store has ever heard of, so committing the label would filter to nothing
     * while the screen looked right.
     */
    const findRows = $derived.by(() => {
      /** @type {{ key: string, kind: 'instrument' | 'month', name: string, detail: string }[]} */
      const out = [];
      for (const r of findInstruments) {
        if (out.length >= FIND_ROWS) break;
        out.push({
          key: r.key,
          kind: 'instrument',
          name: r.key,
          detail:
            r.short > 0
              ? `${fmt(r.months)} month(s) held · −${fmt(r.short)}`
              : `${fmt(r.months)} month(s) held`
        });
      }
      for (const [m, n] of findMonths) {
        if (out.length >= FIND_ROWS) break;
        out.push({
          key: m,
          kind: 'month',
          /* `?? m` IS THE FALLBACK `monthLabel` ALREADY PROMISES, written where
             the checker can see it: it returns its input verbatim for a key it
             cannot parse, and its signature says `string | null | undefined`
             because it accepts those. `m` is a store key and is always a string,
             so this is a no-op at runtime — and on the day the store writes a
             month key nothing can parse, the raw key is what shows, which is the
             value worth seeing. */
          name: monthLabel(m) ?? m,
          detail: `${fmt(n)} row(s) held`
        });
      }
      return out;
    });

    /* HOW MANY MATCH THAT THE BUDGET DID NOT DRAW. Counted from the full
       filtered lists, so the foot of the menu never claims the ten it drew are
       all there are — a list silently truncated is a list that lies. */
    const findMore = $derived(findInstruments.length + findMonths.length - findRows.length);

    /** The listbox is open only when there is something typed to be a list ABOUT. */
    const findShown = $derived(findOpen && db.typed !== '');

    /**
     * Commit one suggestion. THE KEY, never the name.
     *
     * The box stays the single piece of state — writing `filter` is the whole
     * commit, exactly as typing it by hand would be — so there is nothing here
     * to keep in step with the instrument `Picker` beside it, which ticks a row
     * precisely when the box holds that row's key.
     *
     * @param {{ key: string }} row
     */
    function commitFind(row) {
      db.filter = row.key;
      findOpen = false;
      findCursor = -1;
      searchEl?.focus();
    }

    /**
     * ↑ ↓ move the highlight, Enter commits it. Nothing else is intercepted, so
     * every ordinary editing key still edits.
     *
     * ESCAPE IS NOT HERE. `onWindowKey` owns it, above the row drawer and below
     * the calendar, so the order in which the three things on this page close is
     * decided in ONE place instead of racing between a field handler and a
     * window handler that both fire for the same press.
     *
     * @param {KeyboardEvent} e
     */
    function onFindKey(e) {
      if (e.altKey || e.ctrlKey || e.metaKey) return;
      const n = findRows.length;
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        if (!findShown) {
          findOpen = true;
          return;
        }
        if (n === 0) return;
        const d = e.key === 'ArrowDown' ? 1 : -1;
        /* WRAPS, and the `+ n` is what makes it wrap in BOTH directions: `-1 %
           n` is `-1` in JavaScript, not `n - 1`, so ↑ from the first row would
           land on nothing and the highlight would vanish rather than move. */
        findCursor = findCursor < 0 ? (d > 0 ? 0 : n - 1) : (findCursor + d + n) % n;
        e.preventDefault();
        return;
      }
      if (e.key === 'Enter' && findShown && findCursor >= 0 && findCursor < n) {
        commitFind(findRows[findCursor]);
        e.preventDefault();
      }
    }

    /* `/` FOCUSES THE FILTER, from anywhere on the page that is not already a
       text field — the shortcut every terminal and every code host uses. */
    /** @param {KeyboardEvent} e */
    function onWindowKey(e) {
      const t = e.target;
      const typing =
        t instanceof HTMLElement &&
        (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
      /* AN OPEN MENU OWNS ESCAPE, for the same reason the calendar owns the
         keyboard above it: a menu left on screen after the key that dismisses
         things has been pressed is a control the reader has to click to be rid
         of. It is tested before `/` so a menu cannot be left standing while the
         focus is thrown to the filter box. */
      if (db.drop !== null && e.key === 'Escape') {
        db.drop = null;
        return;
      }
      /* AND THE FIND BOX'S OWN LISTBOX SHUTS BEFORE THE ROW DRAWER DOES, for the
         same reason: Escape dismisses the NEAREST thing, and the nearest thing
         to a reader typing in the box is the list under it. Handled here rather
         than on the field so that one function decides the order — a field
         handler and this one both fire for the same press, and two of them
         deciding would close both at once. */
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

    Object.defineProperties(db, {
      index: { get: () => index },
      textMatched: { get: () => textMatched },
      searchEl: { get: () => searchEl, set: (v) => (searchEl = v) },
      findOpen: { get: () => findOpen, set: (v) => (findOpen = v) },
      findCursor: { get: () => findCursor, set: (v) => (findCursor = v) },
      findInstruments: { get: () => findInstruments },
      findMonths: { get: () => findMonths },
      findRows: { get: () => findRows },
      findMore: { get: () => findMore },
      findShown: { get: () => findShown },
      commitFind: { get: () => commitFind },
      onFindKey: { get: () => onFindKey },
      onWindowKey: { get: () => onWindowKey }
    });
  }
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>
