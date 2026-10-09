/**
 * /db — THE FIXED VOCABULARY OF THE CONTROL STRIPS
 *
 * The universes, the three segments, the two views and the membership-token
 * readers the selection rungs are built from, plus the shapes (`StoreRow`,
 * `Feed`, `MasterRow`) the page reads off the wire.
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
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */

/* ======================================================================
   THE SHAPES, NAMED ONCE — imported where they already exist
   ----------------------------------------------------------------------
   `StoreRow` and `Feed` are DECLARED BY THE MODULES THAT FETCH THEM, and
   they are imported here rather than restated. A second spelling of one
   wire shape is the drift this whole file is written against: the day
   `/store.json` gains an eleventh field, the copy that is not the fetcher's
   goes quietly stale and every reader downstream of it believes the stale
   one. `import('…').Name` in a JSDoc type is a compile-time reference and
   emits nothing — it costs the bundle zero bytes.

   `MasterRow` is the exception, and it is one only because no module owns
   `/instruments.json`'s row shape yet: `$lib/index.svelte.js` types
   `catalogue.rows` as `any[]` and `masterRows()` re-states that. It is
   declared HERE, from a live response read off the running server rather
   than guessed, so the two names this page actually joins on —
   `exchange`/`segment`/`symbol`, which is the census key, and `universes`,
   which is the membership token list — are checked at the join and not at
   the crash. The other fields are on the wire and are written down because
   leaving them out would make the typedef read as a claim that they are
   absent.

   The response the shape was read from, `GET /instruments.json?feed=dhan`:

     {"symbol":"NIFTY","key":"NSE-NIFTY","kind":"Index","exchange":"NSE",
      "segment":"INDEX","universe":"index+fno","universes":["index","fno"],
      "bars":8470,"href":"/instruments?q=NSE-NIFTY"}

   `universes` is optional and `universe` is the legacy `+`-joined string
   that predates it — see `tokensOf`, which reads the array when the row has
   one and splits the string when it does not. Both are written optional
   because a master row served by an older API binary carries only the
   second, and the type has to admit the row this page is built to survive.
   ====================================================================== */
/** @typedef {import('$lib/store.svelte.js').StoreRow} StoreRow */
/** @typedef {import('$lib/feeds.svelte.js').Feed} Feed */
/**
 * @typedef {{
 *   symbol: string,
 *   key: string,
 *   kind: string,
 *   exchange: string,
 *   segment: string,
 *   universe?: string,
 *   universes?: string[],
 *   bars: number,
 *   href: string
 * }} MasterRow
 */


/* THE WINDOW IS ALWAYS DRAWN, AND IT NEVER TOUCHES THE RUNGS ABOVE IT.
 *
 * An earlier draft hid this pair the moment an expiry was chosen. That was a
 * misreading of "the dates must not be applicable to a contract". The order
 * the owner set is feed, universe, instrument, segment, timeframe, expiry,
 * strike, moneyness, side, FROM DATE, TO DATE — the dates are in the F&O
 * list, so they are present for a contract too.
 *
 * What must not happen is the window narrowing the CONTRACT RUNGS. It cannot:
 * this filters `barRows`, which is downstream of every offer.
 * `expiryOffered` folds `matchedPreContract`; `strikeOffered` and
 * `sideOffered` fold `expiryGated` — all upstream of the bars. So choosing a
 * fortnight can never make an expiry vanish from its own menu, which is the
 * failure that instruction was guarding against.
 *
 * A contract's extent is still its expiry. The window only says which of its
 * days to draw. */
export const dayWindowApplies = true;

/**
 * THE FEED, WRITTEN FROM THE PANEL'S SCOPE RUNG UNDER THE TOP BAR'S OWN RULE.
 *
 * `feeds.active` is the one selection the whole app reads and this page's own
 * fetch effect already re-reads `/store.json` from it, so this needs no reload
 * of its own — it is the same write the empty-store panel's "Rows exist under"
 * buttons have always made. A feed the list marks not ready is REFUSED rather
 * than hidden: the row is drawn, disabled, and its `title` quotes the server's
 * reason, which is the same test the top bar applies rather than a second
 * opinion about the same feed.
 */

/**
 * The eight names, in the owner's order, with NO COUNT IN ANY LABEL.
 *
 * `token` is the three-state field the whole rung turns on:
 *   · `null` — no membership source exists. The row is shown and disabled.
 *   · `''`   — Everything: NO JOIN AT ALL, which is not the same as a join
 *              that matches everything. A stored instrument this feed's
 *              master no longer lists — BSE history the charter keeps, a
 *              symbol delisted since the pull — is absent from
 *              `/instruments.json` and would silently vanish under any
 *              membership test. Under Everything it cannot, because nothing
 *              is tested.
 *   · a word — one token out of `universe_label`, matched against the `+`
 *              joined set on the catalogue row.
 */
/* THE FLOOR IS NAMED, not indexed off the end of the list. `chosenUniverse`
   falls back to it for an unrecognised slug, and a fallback expressed as
   `UNIVERSES.at(-1)` is one that can be `undefined` — every reader would then
   need a guard, and a missing guard on the row that decides whether the
   table is filtered at all is a blank page. A concrete object cannot be
   absent. */
export const EVERYTHING = { key: '', label: 'Everything', token: '' };

export const UNIVERSES = [
  { key: 'n50', label: 'NIFTY 50', token: 'n50' },
  { key: 'n100', label: 'NIFTY 100', token: 'n100' },
  { key: 'n200', label: 'NIFTY 200', token: 'n200' },
  { key: 'n500', label: 'NIFTY 500', token: 'n500' },
  { key: 'ntm', label: 'NIFTY Total Market', token: 'ntm' },
  { key: 'fno', label: 'F&O underlyings', token: 'fno' },
  { key: 'index', label: 'NSE indices', token: 'index' },
  EVERYTHING
];

/**
 * THE TOKENS ONE CATALOGUE ROW CARRIES, out of the field that carries them.
 *
 * `/instruments.json` sends TWO membership fields, and they are not the same
 * fact. `crates/api/src/server.rs:813` emits both on every row:
 *
 *   · `universe`  — a `+`-joined STRING, deliberately frozen at the first
 *                   `LEGACY_UNIVERSE_TOKENS` = 3 entries of `UNIVERSE_TOKENS`
 *                   (`index`, `fno`, `ntm`). It is a compatibility field and
 *                   it will never name a NIFTY tier.
 *   · `universes` — the full ARRAY, `["index","fno","ntm","n500","n200",
 *                   "n100","n50"]` in that declared order. D-0089 / D-0090.
 *
 * So the array is read when the row has one and the legacy string is split
 * when it does not. THAT IS NOT A FALLBACK THAT HIDES A FAILURE: the legacy
 * string is a complete, correct answer for the three tokens it names, and the
 * four tokens it cannot name are REFUSED BY NAME in `tokensHere` below rather
 * than guessed at. Nothing here ever synthesises a tier.
 *
 * The consequence worth stating: this page needs no edit at all on the day
 * the API binary carrying `universes` is running. The four NIFTY rows light
 * up because the field arrived, which is the only evidence that would justify
 * lighting them.
 *
 * @param {any} row
 * @returns {string[]}
 */
export const tokensOf = (row) =>
  Array.isArray(row?.universes)
    ? row.universes.map(String)
    : String(row?.universe ?? '')
        .split('+')
        .filter(Boolean);

/**
 * The census spells an instrument `EXCHANGE-SEGMENT-SYMBOL`, and that is the
 * ONLY key this join may use.
 *
 * `/instruments.json` also sends a `key`, and it is `EXCHANGE-SYMBOL` — one
 * segment short. Joining on it matches nothing at all, and a join that
 * matches nothing produces a universe holding zero instruments rather than
 * an error: the rung would look like it worked and report "0 of 750 held"
 * over a full store. BUILT FROM THE THREE PARTS, never parsed out of a
 * string, because a symbol may legally contain `-` (`NIFTY-50`, `M&M`).
 * `/markets` proves the same key the same way.
 *
 * @param {MasterRow} row a row of `/instruments.json`, NEVER a census row
 */
export const censusKeyOf = (row) => `${row.exchange}-${row.segment}-${row.symbol}`;

/**
 * The four tokens the LEGACY `universe` string is structurally unable to
 * name, whatever the data holds. `LEGACY_UNIVERSE_TOKENS = 3` in
 * `crates/api/src/server.rs` cuts `UNIVERSE_TOKENS` after `index`, `fno` and
 * `ntm`, so a row without a `universes` array cannot report these four even
 * when the instrument is in all of them.
 */
export const LEGACY_BLIND = new Set(['n50', 'n100', 'n200', 'n500']);

/**
 * THE THREE SEGMENTS THE STORE KEYS ON, ALWAYS ALL THREE.
 *
 * `kinds` below counts what is PRESENT, which is the right input for a total
 * and the wrong one for a menu: with one segment stored the rung offered one
 * row, so "does this store hold any CASH?" was a question the control could
 * not be asked. The comment above the rung already states the intended shape —
 * "/ingest draws Segments unconditionally — three rows, two of them refusing
 * with their reason on their face" — and this is the half of it that was
 * missing.
 *
 * The names are the store's own second field, not the mockup's
 * spot/futures/options taxonomy: relabelling would rename three sets into
 * three other sets that do not have the same members.
 */
export const SEGMENT_KEYS = ['INDEX', 'CASH', 'FNO'];

/**
 * /ingest's THREE SEGMENTS, ON THIS PAGE'S OWN ROWS.
 *
 * The rung offered INDEX / CASH / FNO — the store's own second field — and
 * /ingest offers Spot / Expired futures / Expired options. Same question, two
 * vocabularies, so a reader moving between the pages had to translate.
 *
 * THE FILE ARGUED AGAINST THIS and the objection was real: "relabelling would
 * rename three sets into three other sets that do not have the same members".
 * It is answered rather than ignored — the sets are not relabelled, they are
 * RE-DERIVED from a field every row already carries. `expiry` is what splits
 * them:
 *
 *   Spot            a continuous series — `expiry === null`. INDEX and CASH
 *                   both land here, which is correct: neither settles.
 *   Expired futures a settled contract, `-FUT`.
 *   Expired options a settled contract, `-CE` or `-PE`.
 *
 * So Spot is not "INDEX renamed" — it is every row with no expiry, whatever
 * the store keyed it under, and the membership is computed rather than
 * asserted. The operator's instruction, and the objection's terms are met.
 *
 * The notes are /ingest's own words, including the 503, because the reason an
 * expired contract is absent is the same reason on both pages.
 */
export const SEG_VIEW = [
  { key: 'spot', name: 'Spot', note: 'continuous series — no expiry' },
  {
    key: 'futures',
    name: 'Expired futures',
    note: 'contracts that have already settled · 503 — no transport in this build'
  },
  {
    key: 'options',
    name: 'Expired options',
    note: 'contracts that have already settled · 503 — no transport in this build'
  }
];

/* ======================================================================
   TWO GRIDS OVER ONE STORE - the CENSUS, and the BARS THEMSELVES
   ----------------------------------------------------------------------
   THE CENSUS GRID ANSWERS "WHAT IS HELD" AND CANNOT ANSWER "WHAT IS IN
   IT". One row per instrument-month, counted off `/store.json`, no bar
   file opened: it is the only view that can survey 20,516 rows for a
   hole. It has no open, no close, no volume and no open interest, because
   the census carries none.

   So a SECOND grid is added rather than the first being replaced. One row
   per BAR, read from `/bars.json` - the endpoint `/markets` already
   reads, one request per instrument-month, the path being the index.
   Which grid is showing is a choice the reader makes and the page states
   in words; neither is a default that hides the other.

   WHAT THE BAR GRID WILL NOT DO. Ten of its twenty-four columns have no
   source on this wire, and every one of them says so on its own face
   rather than printing a number. `CLAUDE.md` section 3 rule 1: a Greek
   this page invented would be indistinguishable, on screen, from one a
   vendor sent.
   ====================================================================== */
export const VIEWS = [
  {
    // KEY STAYS `census`. It is an identifier -- it keys `view`, `VIEWS.find`
    // and every comparison in this file -- and renaming an id to match a
    // label is how a rename becomes a bug. Only the words the operator reads
    // change.
    key: 'census',
    // "Census" was internal vocabulary. The operator read the tab and could
    // not tell what the view was FOR. "Coverage" says it: this is the view
    // that answers what you hold and what is missing.
    label: 'Coverage',
    sub: 'what you hold, per instrument-month',
    showing: 'one row per INSTRUMENT-MONTH, counted from /store.json',
    title:
      'What the store HOLDS. One row per instrument-month, off /store.json - bars, sessions, completeness, % change. No bar file is opened, so this is the only view that can survey the whole store for a hole.'
  },
  {
    key: 'bars',
    label: 'Bars',
    sub: 'the stored values, one row per bar',
    showing: 'Stored candles, one row at a time, shown in pages',
    title:
      'Inspect saved candle prices and volume for your selected instrument and dates. Each page reads only the requested rows, and prices come from the saved data.'
  }
];
