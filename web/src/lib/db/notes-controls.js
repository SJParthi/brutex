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
import { MAX_TEXT, checkText, checkDay, parseDay, dayBoundWhy, dayText, windowWhy, quote } from './input-check.js';

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


/* THE DAY WINDOW NARROWS A CONTRACT TOO (DB-046, DB-G89; D-3337 item 5).
 *
 * The page once said the dates were "spot only — a contract's window is its
 * expiry, so these are ignored once an expiry is chosen". The code and the
 * server did the opposite, correctly: with FROM = TO = 2019-03-20 on NIFTY
 * 2019-03-28 11,500 CE the grid asks `/bars.json?...&from=2019-03-20&to=
 * 2019-03-20` and every row returned is on that IST day. The labels now say
 * what happens: the window narrows a contract to the chosen days within its
 * life.
 *
 * What it must not do is narrow the CONTRACT RUNGS, and it cannot: it filters
 * the bars, which are downstream of every offer. `expiryOffered` folds
 * `matchedPreContract`; the strike and side offers fold `expiryGated` — all
 * upstream of the bars. So choosing a fortnight can never make an expiry
 * vanish from its own menu. */
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
 * THE SEGMENTS, ON THIS PAGE'S OWN ROWS (D-3337 item 6; DB-050, DB-G67).
 *
 * Re-derived from a field every row carries — its expiry — never relabelled
 * from the store's INDEX / CASH / FNO:
 *
 *   Spot                 `expiry === null`: INDEX and CASH, neither settles.
 *   Expired futures /    a contract whose expiry is before TODAY'S IST day
 *   Expired options      and on or before the feed's archive end.
 *   … settled after the  a contract that settled before today but after the
 *   archive ended        archive's last day: its final sessions are not held.
 *   Unexpired …          a contract expiring today or later: not settled.
 *
 * The last four rows are drawn only when the feed's store holds such a
 * contract; Expired futures and Expired options are always drawn.
 *
 * THE "503 — no transport in this build" CLAUSE IS GONE. It was false for a
 * store holding about 1.55 lakh option instrument-months imported by
 * `cli gdfl-import` (D-2804), and the server sentence it quoted ("expired F&O
 * has no local-archive path and no HTTP transport in this build") is no
 * longer anywhere in `crates/api` (checked 10 Oct 2026). What a feed holds and how it got there is counted on the row itself
 * (`segmentRows` in `selection.svelte.js`); these notes are the definitions.
 * Membership is computed by `segmentOf` in `selection-model.js`.
 */
export const SEG_VIEW = [
  { key: 'spot', name: 'Spot', note: 'continuous series — no expiry', always: true },
  {
    key: 'futures',
    name: 'Expired futures',
    note: 'futures that settled before today, with their final session inside the archive',
    always: true
  },
  {
    key: 'options',
    name: 'Expired options',
    note: 'options that settled before today, with their final session inside the archive',
    always: true
  },
  {
    key: 'futures-cut',
    name: 'Futures settled after the archive ended',
    note: "settled before today but after the archive's measured last day, so their last sessions are not imported",
    always: false
  },
  {
    key: 'options-cut',
    name: 'Options settled after the archive ended',
    note: "settled before today but after the archive's measured last day, so their last sessions are not imported",
    always: false
  },
  {
    key: 'futures-live',
    name: 'Unexpired futures',
    note: 'expiring today or later — not settled',
    always: false
  },
  {
    key: 'options-live',
    name: 'Unexpired options',
    note: 'expiring today or later — not settled',
    always: false
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

/* ======================================================================
   R3-02 — THE CONTROLS' OWN WORDS, LISTS AND LIMITS
   ----------------------------------------------------------------------
   Pure functions the day boxes, the time lists, the search boxes and the
   rung notes are built from, so `node --test` proves each sentence and each
   list with no compiler (`web/tests/db-controls.test.js`,
   `web/tests/dayfield.test.js`). Nothing here knows a vendor, an hour or an
   instrument: the session grid, the calendar of record's special windows and
   the bounds are handed in by the caller.
   ====================================================================== */

/** The longest text a day box keeps — what `checkDay` reads (32). */
export const DAY_TEXT_MAX = 32;
/** The longest text a list's search box keeps — `MAX_TEXT`, the instrument bound. */
export const SEARCH_TEXT_MAX = MAX_TEXT;

/**
 * TEXT LONGER THAN A BOX READS IS CUT, AND SAID IN ONE SENTENCE (R3-02).
 *
 * 6,000 pasted characters were kept whole: the day box echoed all of them
 * back inside its own refusal and a list's search box quoted them in "Nothing
 * matches “…”", a paragraph wider than the page. The box now keeps the first
 * `max` and the sentence names the length once — the words `checkText` says
 * for the same value carried by a link. Reads `max + 1` characters' worth of
 * work at most: the cut is one `slice`.
 *
 * @param {unknown} raw
 * @param {number} max
 * @param {string} label what the box is, for the sentence ("To date", "this search")
 * @returns {{ text: string, why: string }} `why` is `''` when nothing was cut
 */
export function cutText(raw, max, label) {
  const s = String(raw ?? '');
  if (s.length <= max) return { text: s, why: '' };
  /* Refused by construction: `s` is longer than `max`. */
  const long = /** @type {{ok: false, why: string}} */ (checkText(s, { label, max }));
  /* The cut counts UTF-16 units; it never ends on the first half of a
     surrogate pair, which would leave a lone half in the box (repair round 2). */
  const lead = s.charCodeAt(max - 1);
  return { text: s.slice(0, lead >= 0xd800 && lead <= 0xdbff ? max - 1 : max), why: long.why };
}

/**
 * WHAT A DAY BOX SAYS UNDER ITSELF (R3-02; DB-045 and the absurd-input
 * probes), as one string, or `null` when the day it holds is fine — or when it
 * holds nothing and nothing was typed into it, which is no bound, not a
 * malformed day (repair round 4).
 *
 * Four refusals, each naming what was actually crossed:
 *
 * - CUT: more than `DAY_TEXT_MAX` characters arrived (a paste). One sentence,
 *   the length named once, never the text echoed.
 * - SHAPE: not a day (`parseDay`, the parser a link's `from=` is read with),
 *   quoted at most 40 characters long (`quote`).
 * - HELD: a day outside what the selection holds (`min`/`max`), in the bound
 *   sentence a linked day gets (`dayBoundWhy`), after the box and the day are
 *   named: "To date 01 Jan 1969: Outside 03 Sep 2018 to 30 Sep 2026 — …".
 * - ORDER: inside the held span but on the wrong side of the OTHER box
 *   (`after` for a To box, `before` for a From box): the window's order is
 *   named in the sentence a backwards link gets (`windowWhy`) — never the held
 *   range, which the day is inside (the 01 Aug 2026 probe read "Outside 01 Sep
 *   2026 to 30 Sep 2026 — This selection holds …").
 *
 * The box itself adds "Not applied: the field still holds …" while the
 * reader's text is unapplied — except after a cut, whose one sentence already
 * says it.
 *
 * @param {{
 *   typed: string | null,
 *   value: string,
 *   label?: string,
 *   min?: string,
 *   max?: string,
 *   after?: string,
 *   before?: string,
 *   reason?: string,
 *   cut?: string
 * }} o `typed` is the box's unapplied text, `null` when it shows `value`;
 *   `cut` the sentence `cutText` gave for the last paste
 * @returns {string | null}
 */
export function dayFieldWhy(o) {
  const label = o.label ?? 'Date';
  if (o.cut) return `${o.cut} — cut, not applied: the field still holds ${o.value ? dayText(o.value) : 'no bound'}.`;
  const text = o.typed ?? o.value;
  /* A BOX THAT HOLDS NOTHING SAYS NOTHING (repair round 4). Blank typed text
     is an emptied box, and an untyped box holding no day is "no bound" — a
     selection with no held span leaves both boxes so. It fell through to the
     shape check and was refused as `To date "" is not a date …`, marked
     `aria-invalid`, over a box nothing was typed into. */
  if (text.trim() === '') return null;
  const iso = parseDay(text);
  if (iso === '') {
    const shape = /** @type {{ok: false, why: string}} */ (checkDay(text, { label, lo: o.min, hi: o.max }));
    return `${shape.why}.`;
  }
  const held = dayBoundWhy(iso, { lo: o.min, hi: o.max, reason: o.reason });
  if (held) return `${label} ${dayText(iso)}: ${held}`;
  /* `windowWhy` answers only a backwards pair, so it is asked both ways. */
  const order = windowWhy(o.after ?? '', iso) ?? windowWhy(iso, /** @type {string} */ (o.before));
  if (order) return `${order}.`;
  return null;
}

/**
 * Whether a day may be taken: inside `[min, max]` and on the right side of the
 * other box. The same predicate the box's grid strikes by and its typing
 * applies by, so a struck day and a refused typed day are one rule.
 *
 * @param {string} iso
 * @param {{min?: string, max?: string, after?: string, before?: string}} o
 */
export function dayTakes(iso, o) {
  return (!o.min || iso >= o.min) && (!o.max || iso <= o.max) && (!o.after || iso >= o.after) && (!o.before || iso <= o.before);
}

/**
 * WHY ONE STRUCK CELL IS STRUCK, for its title: the held span's reason, or
 * the other box's day. `null` for a day the box takes.
 *
 * @param {string} iso
 * @param {{min?: string, max?: string, after?: string, afterLabel?: string, before?: string, beforeLabel?: string, reason?: string}} o
 * @returns {string | null}
 */
export function dayStruckWhy(iso, o) {
  if ((o.min && iso < o.min) || (o.max && iso > o.max)) return o.reason || 'Outside what this field may take.';
  if (o.after && iso < o.after) return `Before the ${o.afterLabel ?? 'From date'}, ${dayText(o.after)}: a window ends on or after the day it starts.`;
  if (o.before && iso > o.before) return `After the ${o.beforeLabel ?? 'To date'}, ${dayText(o.before)}: a window starts on or before the day it ends.`;
  return null;
}

/**
 * THE SENTENCE UNDER THE CALENDAR'S GRID, NAMING EVERY KIND OF DAY IT STRIKES
 * (DB-045). It said "can be picked" over `min` and `max` and the caller's
 * held-span reason, while `min` was the OTHER box's day: with From and To on
 * 30 May 2019 the From grid struck 31 May, a stored day, under a sentence that
 * said only days outside the held span were struck. The order bound is now
 * its own clause, drawn only when it strikes a day the held span does not.
 *
 * @param {{min?: string, max?: string, after?: string, afterLabel?: string, before?: string, beforeLabel?: string, reason?: string}} o
 * @returns {string} `''` when nothing is struck
 */
export function calendarNote(o) {
  const parts = [];
  if (o.min || o.max) {
    parts.push(`${o.min ? dayText(o.min) : 'any earlier day'} – ${o.max ? dayText(o.max) : 'any later day'} can be picked.${o.reason ? ` ${o.reason}` : ''}`);
  }
  if (o.after && (!o.min || o.after > o.min)) {
    parts.push(`Days before the ${o.afterLabel ?? 'From date'}, ${dayText(o.after)}, are struck too: a window ends on or after the day it starts.`);
  }
  if (o.before && (!o.max || o.before < o.max)) {
    parts.push(`Days after the ${o.beforeLabel ?? 'To date'}, ${dayText(o.before)}, are struck too: a window starts on or before the day it ends.`);
  }
  return parts.join(' ');
}

/* ---- the time lists (DB-074) ---------------------------------------- */

/** Minutes in a day: a rung this wide or wider has no intraday time to list. */
const MINUTES_PER_DAY = 1440;

/** @param {number} m minutes since IST midnight */
const clockOf = (m) => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
/** @param {unknown} t `HH:MM` @returns {number | null} */
const minuteOf = (t) => {
  const m = /^([01]\d|2[0-3]):([0-5]\d)$/.exec(String(t));
  return m ? Number(m[1]) * 60 + Number(m[2]) : null;
};

/**
 * @typedef {{date: string, kind: string, windows: Array<{from: number, to: number}>, halt?: boolean, halted?: Array<{from: number, to: number}>, reopening?: Array<{from: number, to: number}>, suspected?: boolean}} SpecialDay
 *   `windows` the minutes that traded (a recorded halt cut out); `halt` the
 *   calendar of record's code is `H`: a regular-length session a recorded
 *   halt cut, which the day string already counts as regular (`specialsOf`
 *   always sets it; absent reads as not a halt day); `halted` the recorded
 *   halts INSIDE the day's session, the minutes cut out of `windows`
 *   (`specialsOf` always sets it; absent reads as none); `reopening` the
 *   part of `halted` that was the reopening call auction, where the record
 *   knows it (`pull::calendar::Halt::reopening`, the cash market's and the
 *   index's); `suspected` a halt of the day is a suspected one
 *   (`/calendar.json` `halts[].suspected`, D-3320), never assumed
 */

/**
 * The halted minute ranges of a `/calendar.json` answer's `halts` list, by
 * day (`calendar_record.rs` `halts_json`: `halted.from` and `halted.to` the
 * first and last halted minute, inclusive). An entry this reader cannot read
 * is left out, never guessed — which lists MORE minutes, never fewer.
 *
 * Each keeps its reopening call auction (`reopening`, `null` when the record
 * has none or it cannot be read) and whether the record calls it SUSPECTED
 * (`suspected: true` only; repair round 3) — the 13 and 23 Mar 2020 halts are
 * the NSE rule's shape read from measured minute holes, with no NSE circular
 * (D-3320), and a list must not say more of them than the record does.
 *
 * @param {unknown} halts
 * @returns {Map<string, Array<{from: number, to: number, reopening: {from: number, to: number} | null, suspected: boolean}>>}
 */
export function haltsOf(halts) {
  /** @type {Map<string, Array<{from: number, to: number, reopening: {from: number, to: number} | null, suspected: boolean}>>} */
  const by = new Map();
  if (!Array.isArray(halts)) return by;
  for (const h of halts) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(String(h?.date))) continue;
    const from = minuteOf(h.halted?.from);
    const to = minuteOf(h.halted?.to);
    if (from === null || to === null || from > to) continue;
    const rf = minuteOf(h.reopening?.from);
    const rt = minuteOf(h.reopening?.to);
    const halt = { from, to, reopening: rf !== null && rt !== null && rf <= rt ? { from: rf, to: rt } : null, suspected: h.suspected === true };
    const list = by.get(h.date);
    if (list) list.push(halt);
    else by.set(h.date, [halt]);
  }
  return by;
}

/**
 * `windows` with the inclusive minute range `cut` taken out: a window it
 * covers goes, one it splits becomes two, one it misses stays as it was.
 *
 * @param {Array<{from: number, to: number}>} windows
 * @param {{from: number, to: number}} cut
 */
export function cutWindows(windows, cut) {
  /** @type {Array<{from: number, to: number}>} */
  const out = [];
  for (const w of windows) {
    if (cut.to < w.from || cut.from > w.to) {
      out.push(w);
      continue;
    }
    if (w.from < cut.from) out.push({ from: w.from, to: cut.from - 1 });
    if (cut.to < w.to) out.push({ from: cut.to + 1, to: w.to });
  }
  return out;
}

/**
 * THE SPECIAL SESSIONS OUT OF A `/calendar.json` ANSWER (D-3319; DB-074): its
 * `special` list, one entry per recorded day with the windows that traded —
 * a Muhurat hour, a disaster-recovery Saturday's two windows, the 24 Feb 2021
 * outage's two — each window's `from` and `to` the first and last minute a
 * bar opens (`pull::calendar::Window`, inclusive). The answer the page already
 * reads for the closed days, so the time lists cost no request of their own.
 * An entry this reader cannot read is left out, never guessed.
 *
 * @param {any} body
 * @returns {SpecialDay[]}
 */
export function specialsOf(body) {
  if (!Array.isArray(body?.special)) return [];
  const halted = haltsOf(body?.halts);
  /** @type {SpecialDay[]} */
  const out = [];
  for (const s of body.special) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(String(s?.date)) || !Array.isArray(s.windows)) continue;
    /** @type {Array<{from: number, to: number}>} */
    let windows = [];
    for (const w of s.windows) {
      const from = minuteOf(w?.from);
      const to = minuteOf(w?.to);
      if (from !== null && to !== null && from <= to) windows.push({ from, to });
    }
    if (!windows.length) continue;
    /* A HALT IS CUT OUT OF THE DAY'S WINDOWS: the `special` list carries a
       halt day's whole session (09:15–15:29) while its `owed` leaves out every
       halted minute ("a halted minute is neither owed nor missing", D-3336
       §1), so a list built from the windows alone offered 375 minutes on
       13 Mar 2020 where 316 traded. Cutting a range a window does not reach
       changes nothing (24 Feb 2021's windows already stop at 11:39).

       WHAT IS CUT IS KEPT, AS HALTED (repair round 2): the part of a halt
       inside the day's session. Both stores hold bars there — on 13 and
       23 Mar 2020 Zerodha 1min holds 59 and GDFL 1s 3,537 inside the cut
       minutes (the re-check, read on a shadow of the store) — so a list
       that dropped them left those bars out of reach of From time, To time
       and Go to. `timeListRows` lists them labelled halted, never traded. */
    /** @type {Array<{from: number, to: number}>} */
    const inside = [];
    /* THE REOPENING CALL AUCTION, LABELLED (repair round 3): the part of a
       halt inside the session that the record names as its reopening auction
       (13 Mar 2020 10:06–10:20, 23 Mar 2020 10:42–10:56), so those minutes
       are not said as one undivided halt. */
    /** @type {Array<{from: number, to: number}>} */
    const reopening = [];
    let suspected = false;
    for (const h of halted.get(s.date) ?? []) {
      for (const w of windows) {
        const from = Math.max(w.from, h.from);
        const to = Math.min(w.to, h.to);
        if (from > to) continue;
        inside.push({ from, to });
        suspected ||= h.suspected;
        if (!h.reopening) continue;
        const af = Math.max(from, h.reopening.from);
        const at = Math.min(to, h.reopening.to);
        if (af <= at) reopening.push({ from: af, to: at });
      }
    }
    for (const h of halted.get(s.date) ?? []) windows = cutWindows(windows, h);
    out.push({
      date: s.date,
      kind: typeof s.kind === 'string' && s.kind ? s.kind : 'special session',
      windows,
      halt: s.code === 'H',
      halted: inside,
      reopening,
      suspected
    });
  }
  /* ISO days order as text; `sort` is stable, so one day's entries keep the answer's order. */
  return out.sort((a, b) => a.date.localeCompare(b.date));
}

/** Codes of the calendar of record that are a regular-length session. */
const REGULAR = new Set(['R', 'X', 'U', 'H']);
/** @type {WeakMap<object, {regular: Int32Array, unknown: Int32Array}>} */
const PREFIX = new WeakMap();

/**
 * The two prefix sums of a calendar's day string — regular sessions and days
 * the calendar does not reach — built once per calendar object and kept.
 *
 * @param {{days: string}} cal
 * @returns {{regular: Int32Array, unknown: Int32Array}}
 */
export function prefixOf(cal) {
  let p = PREFIX.get(cal);
  if (!p) {
    p = { regular: new Int32Array(cal.days.length + 1), unknown: new Int32Array(cal.days.length + 1) };
    let i = 0;
    for (const c of cal.days) {
      p.regular[i + 1] = p.regular[i] + (REGULAR.has(c) ? 1 : 0);
      p.unknown[i + 1] = p.unknown[i] + (c === '-' ? 1 : 0);
      i += 1;
    }
    PREFIX.set(cal, p);
  }
  return p;
}

/**
 * `YYYY-MM-DD` to days since 1970-01-01, on the calendar, or `NaN` for text
 * that is not a day of it — an impossible day such as 2024-02-30 included,
 * which `Date.UTC` would roll into March (repair round 2).
 * @param {string} iso
 */
const epochOf = (iso) => {
  /* The shape first: text that is no date at all would make `toISOString` throw. */
  if (!/^\d{4}-\d{2}-\d{2}$/.test(iso)) return NaN;
  const ms = Date.UTC(Number(iso.slice(0, 4)), Number(iso.slice(5, 7)) - 1, Number(iso.slice(8, 10)));
  return new Date(ms).toISOString().slice(0, 10) === iso ? Math.round(ms / 86_400_000) : NaN;
};

/**
 * HOW MANY REGULAR SESSIONS THE DAYS IN VIEW HOLD, by the calendar of record:
 * `R`, `X`, `U` and `H` days in `[lo, hi]` (`P`, a special session, is
 * counted by its windows instead; `.` is closed). `null` when the calendar is
 * missing or does not reach every day in view — unknown, never zero.
 *
 * O(1) per question: the two prefix sums are built once per calendar object
 * (one pass over its day string) and kept in a `WeakMap`, so the time lists
 * re-asking on every change of the window cost two subtractions.
 *
 * @param {{from: number, days: string} | null} cal
 * @param {string} lo `YYYY-MM-DD`, or `''` for the calendar's first day
 * @param {string} hi `YYYY-MM-DD`, or `''` for its last
 * @returns {number | null} also `null` for a day that is not `YYYY-MM-DD`
 */
export function regularDaysIn(cal, lo, hi) {
  if (typeof cal?.days !== 'string' || cal.days.length === 0) return null;
  const p = prefixOf(cal);
  const first = lo ? epochOf(lo) - cal.from : 0;
  const last = hi ? epochOf(hi) - cal.from : cal.days.length - 1;
  if (!Number.isSafeInteger(first) || !Number.isSafeInteger(last)) return null;
  if (first > last) return 0;
  if (first < 0 || last >= cal.days.length) return null;
  if (p.unknown[last + 1] - p.unknown[first] > 0) return null;
  return p.regular[last + 1] - p.regular[first];
}

/**
 * THE TIME LISTS: EVERY MINUTE THE DAYS IN VIEW REALLY TRADED (R3-02; DB-074).
 *
 * `grid` is the rung's session grid (`sessionGrid`: the regular session with
 * its dated close, plus any minute the loaded rows hold) built with the
 * special windows' minutes already passed in as held; this relabels and trims
 * it against the days in view:
 *
 * - a special session's minute that is not a regular-session minute (an
 *   evening Muhurat hour, the 24 Feb 2021 15:45–16:59 resumption) says which
 *   recorded day it belongs to and why it is listed — it read "listed because a
 *   loaded bar opens here, such as a closing-auction print", and was listed
 *   only when one of the fifty rows on screen happened to open there;
 * - when the days in view hold NO regular session (a window of Muhurat days
 *   only), the regular minutes go: nothing traded at 09:15 on 01 Nov 2024, and
 *   a list offering it would land a bound on nothing. Held minutes stay.
 *
 * O(rows + special minutes): the list is at most the 375–385 regular minutes
 * plus the windows of the special days in view.
 *
 * @param {Array<{key: string, name: string, detail: string, why?: string}>} grid
 * @param {{specials: SpecialDay[], regular: number | null, held: Iterable<string>, step: number, open: number}} o
 *   `specials` the special days in view, `regular` how many regular sessions
 *   are in view (`null` unknown), `held` the loaded rows' minutes, `step` the
 *   rung's minutes, `open` the session's first minute (the grid's anchor)
 */
export function timeListRows(grid, o) {
  /** @type {Map<string, SpecialDay[]>} */
  const by = new Map();
  for (const t of specialMinutes(o.specials, o.step, o.open)) by.set(t.key, t.days);
  /* The buckets a recorded halt inside a day's session holds (repair round 2): listed, said as halted. */
  /** @type {Map<string, SpecialDay[]>} */
  const stoppedBy = new Map();
  for (const t of specialMinutes(o.specials, o.step, o.open, haltedOf)) stoppedBy.set(t.key, t.days);
  /* The buckets of a halt's reopening call auction (repair round 3): said as the auction, inside the halt. */
  /** @type {Map<string, SpecialDay[]>} */
  const auctionBy = new Map();
  for (const t of specialMinutes(o.specials, o.step, o.open, reopeningOf)) auctionBy.set(t.key, t.days);
  const held = new Set(o.held);
  /* A HALT DAY IS NOT A WHOLE REGULAR SESSION: the day string counts code `H`
     as regular, but its halted minutes traded nothing, so a window holding
     only halt and special days lists only what their windows hold. */
  const halts = o.specials.reduce((n, d) => n + (d.halt ? 1 : 0), 0);
  const none = o.regular !== null && o.regular - halts === 0;
  /* On a derived rung a bar stamped before a window holds that window's first
     minutes (the fold stamps a bucket at its start), so "at this minute"
     would be false of it. */
  const at = o.step > 1 ? `inside the ${o.step}-minute bar stamped here (a bar carries the first minute of its bucket on the 09:15 grid, pull::fold)` : 'at this minute';
  const spans = (/** @type {Array<{from: number, to: number}>} */ ws) => ws.map((w) => `${clockOf(w.from)}–${clockOf(w.to)}`).join(' and ');
  /* A halt day's traded windows are not where its bars open — both stores hold bars inside its halt — so it says what traded and what was halted. */
  const said = (/** @type {SpecialDay} */ d) =>
    `${d.kind}, ${dayText(d.date)}: ${d.halted?.length ? `traded ${spans(d.windows)}, halted ${spans(d.halted)}${d.reopening?.length ? ` with the reopening call auction at ${spans(d.reopening)}` : ''}` : `bars open ${spans(d.windows)}`}`;
  const named = (/** @type {SpecialDay[]} */ days) =>
    `${days.slice(0, 3).map(said).join('; ')}${days.length > 3 ? `; and ${days.length - 3} more` : ''} (the exchange calendar of record, /calendar.json)`;
  /** @type {Array<{key: string, name: string, detail: string, why?: string}>} */
  const out = [];
  for (const row of grid) {
    const days = by.get(row.key);
    const stopped = stoppedBy.get(row.key);
    if (none && !days && !stopped && !held.has(row.key)) continue;
    /* A plain regular minute is the one row `sessionGrid` gives no reason. */
    if ((!days && !stopped) || (!none && !row.why)) {
      out.push(row);
      continue;
    }
    if (!days) {
      /* Only halted minutes in this bucket: listed so a stored bar here is reachable, said as halted — and
         NEVER as "nothing traded" (repair round 3): a halt labels, it never removes (`pull::calendar::Halt`),
         and the record's own evidence for 13 Mar 2020 has shares outside the F&O list and stock options
         printing at 09:22. A minute of the reopening call auction says so; a suspected halt says it is
         suspected and UNVERIFIED, as the record does (D-3320). */
      const s = /** @type {SpecialDay[]} */ (stopped);
      const auction = auctionBy.get(row.key);
      const what = auction?.length === s.length ? 'reopening auction' : 'halted';
      /* THE DOUBT NAMES ITS DAY (repair round 4): with several halt days sharing a minute, "it" left the
         reader to guess which, and one suspected day among sourced ones is still said — a minute never
         carries less doubt than the record holds. */
      const doubted = s.filter((d) => d.suspected);
      const which = doubted.map((d) => dayText(d.date)).join(' and ');
      const whose = s.length === 1 ? 'it a suspected halt' : doubted.length === 1 ? `the halt of ${which} a suspected halt` : `the halts of ${which} suspected halts`;
      const doubt = doubted.length ? ` — the record calls ${whose}, UNVERIFIED (D-3320): no NSE circular is recorded` : '';
      out.push({
        key: row.key,
        name: row.name,
        detail: s.length === 1 ? `${what} · ${dayText(s[0].date)}` : `${what} · ${s.length} days in view`,
        why: `${named(s)}. Halted ${at}${auction ? `, in the reopening call auction of ${auction.map((d) => dayText(d.date)).join(' and ')}` : ''}${doubt}. A halted minute is neither owed nor missing (D-3336 §1), and a print inside a halt is still a bar (pull::calendar); listed so a bar the store holds here can still be reached.`
      });
      continue;
    }
    const first = days[0];
    out.push({
      key: row.key,
      name: row.name,
      detail: days.length === 1 ? `${first.halt ? 'halt day' : 'special session'} · ${dayText(first.date)}` : `special session · ${days.length} days in view`,
      why: `${named(days)}. Listed because a day in view traded ${at}.`
    });
  }
  return out;
}

/**
 * Every bar a special day's windows fill, on the rung's grid, keyed by the
 * minute the bar is STAMPED at, each with the days that traded it.
 *
 * THE STAMP IS THE START OF THE BUCKET (`pull::fold::bucket_start`: every
 * intraday rung is anchored on the 09:15 open, so a bucket is
 * `[open + k·step, open + (k+1)·step)` and its bar carries the first minute of
 * that span, `crates/pull/src/fold.rs`). A window that starts off the grid
 * therefore fills the bucket it starts IN, stamped BEFORE the window: the
 * 01 Nov 2024 Muhurat hour (18:00–18:59) at 60min is two bars, 17:15 (its
 * 18:00–18:14) and 18:15, which is the `owed` /calendar.json gives for it.
 * This used to begin at the first grid point INSIDE the window (a ceiling)
 * and so left out the bar holding a window's first minutes on 24 special-day
 * and rung pairs, every one one bar short of `owed` (DB-074, repair round 1).
 *
 * A day or wider rung has no intraday minute to list (its bar is stamped at
 * IST midnight, and `sessionGrid` gives it no time list): `[]`.
 *
 * @param {SpecialDay[]} specials
 * @param {number} step minutes per bar (1 on 1s and 1min)
 * @param {number} open the session's first minute, the grid's anchor
 * @param {(d: SpecialDay) => Array<{from: number, to: number}>} [rangesOf] which
 *   minutes of a day to fold: its traded `windows` (the default), or
 *   `haltedOf` for the halts inside its session
 * @returns {Array<{key: string, days: SpecialDay[]}>}
 */
export function specialMinutes(specials, step, open, rangesOf = windowsOf) {
  const st = Math.max(1, Math.round(step));
  if (st >= MINUTES_PER_DAY) return [];
  /** @type {Map<string, SpecialDay[]>} */
  const by = new Map();
  for (const d of specials) {
    for (const w of rangesOf(d)) {
      /* The floor of the window's first minute on the grid: the bucket that holds it. */
      const lead = (((w.from - open) % st) + st) % st;
      for (let m = w.from - lead; m <= w.to; m += st) {
        /* A bucket that began on the day before (a window inside the first
           step of midnight) carries that day's clock, never a negative one. */
        const k = clockOf(((m % MINUTES_PER_DAY) + MINUTES_PER_DAY) % MINUTES_PER_DAY);
        const list = by.get(k);
        if (!list) by.set(k, [d]);
        else if (list[list.length - 1] !== d) list.push(d);
      }
    }
  }
  return [...by.entries()].map(([key, days]) => ({ key, days })).sort((a, b) => a.key.localeCompare(b.key));
}

/** A day's traded windows. @param {SpecialDay} d */
const windowsOf = (d) => d.windows;
/** The recorded halts inside a day's session, `[]` when none. @param {SpecialDay} d */
export const haltedOf = (d) => d.halted ?? [];
/** The reopening call auctions inside a day's halts, `[]` when none. @param {SpecialDay} d */
export const reopeningOf = (d) => d.reopening ?? [];

/**
 * The special days of `specials` inside `[lo, hi]` (either end `''` for
 * open). O(specials): the calendar records a few dozen.
 *
 * @param {SpecialDay[]} specials
 * @param {string} lo
 * @param {string} hi
 */
export function specialsIn(specials, lo, hi) {
  return specials.filter((d) => (!lo || d.date >= lo) && (!hi || d.date <= hi));
}

/**
 * WHY THE THREE TIME LISTS ARE DISABLED, said on their buttons (R3-02 repair
 * round 1). An empty `sessionRows` disabled From time, To time and Go to with
 * no word at all, which reads as a broken control. `''` while they hold a
 * minute. A case this cannot name is said to be one, never passed off as an
 * ordinary empty window.
 *
 * @param {{rows: number, rung?: string, rungSecs: unknown, regular: number | null, specials: SpecialDay[], lo: string, hi: string}} o
 *   `rows` the list's length, `rung` the chosen timeframe as the selection holds it (`''` for none),
 *   `regular` the regular sessions in view, `specials` the special days in view
 */
export function timeListWhy(o) {
  if (o.rows > 0) return '';
  const secs = Number(o.rungSecs);
  if (!Number.isFinite(secs) || secs <= 0) {
    /* A chosen timeframe with no bar length is not "none chosen" (repair round 2): tf=60MIN names it. */
    return o.rung
      ? `Timeframe ${quote(o.rung)} is not a timeframe this page knows, so there is no bar length to list minutes on — pick one under Timeframe.`
      : 'No timeframe is chosen, so there is no bar length to list minutes on — pick one under Timeframe.';
  }
  const span = o.lo && o.hi ? ` (${dayText(o.lo)} to ${dayText(o.hi)})` : '';
  const halts = o.specials.filter((d) => d.halt).length;
  if (o.regular !== null && o.regular - halts === 0) {
    return `The days in view${span} hold no trading session in the exchange calendar of record (/calendar.json)${halts ? ' beyond halted minutes' : ''} and no loaded bar, so there is no minute to pick — move the dates onto a trading day.`;
  }
  return `UNEXPLAINED: the time lists hold no minute for the days in view${span}, although the calendar counts ${o.regular === null ? 'an unknown number of' : o.regular} regular session(s) there — a defect to report.`;
}

/* ---- the calendar's month flags (DB-025) ----------------------------- */

/**
 * The first and last ISO day of a `YYYY-MM` month, or `null`.
 *
 * @param {string} ym
 */
export function monthDays(ym) {
  const m = /^(\d{4})-(\d{2})$/.exec(String(ym));
  if (!m) return null;
  const y = Number(m[1]);
  const mo = Number(m[2]);
  if (mo < 1 || mo > 12) return null;
  const last = [31, (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0 ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][mo - 1];
  /* Every month has at least 28 days, so the last day is two digits. */
  return { from: `${m[1]}-${m[2]}-01`, to: `${m[1]}-${m[2]}-${last}` };
}

/**
 * WHETHER A MONTH THE CALENDAR SHOWS MUST BE ASKED FOR (DB-025): not when the
 * day band's own answer covers every day of it, so opening the calendar on
 * the window's month costs no request at all, and any other month costs one.
 *
 * @param {string} ym
 * @param {{from: string, to: string, clipped?: boolean} | null} band the span the day band read, or `null`
 */
export function monthNeedsRead(ym, band) {
  const days = monthDays(ym);
  if (!days) return false;
  return !(band && band.from <= days.from && band.to >= days.to);
}

/** The most months of day flags the calendar keeps before the oldest goes. */
export const MONTH_FLAGS_KEPT = 48;

/**
 * One flagged cell as the calendar's title says it — the band's own words.
 *
 * @param {{word: string, says: string[]}} c
 */
export const flagText = (c) => `${c.word}${c.says.length ? ` — ${c.says.join('; ')}` : ''}`;

/* ---- the rung notes (DB-041, DB-G63, DB-020, D-3338) ----------------- */

/**
 * WHAT THE INSTRUMENT LIST IS, IN WORDS THAT ARE TRUE OF THIS FEED (DB-041).
 * "Every name this feed's master lists" was printed over GDFL, which has no
 * master: its list is its census, with membership from core::universe.
 *
 * @param {boolean} noMaster
 * @param {string} feedName
 */
export function instrumentListWhy(noMaster, feedName) {
  return noMaster
    ? `Every name ${feedName} stores — it publishes no instrument master, so this list is its census (stored, no master), narrowed by core::universe's own membership lists for the chosen universe. A name with nothing stored is drawn with a zero rather than omitted.`
    : `Every name ${feedName}'s instrument master lists for the chosen universe. One the store does not hold is drawn with a zero rather than omitted — a rung that can only name what you already have cannot tell you what you are missing.`;
}

/**
 * Why Everything is the one universe that reaches every stored row (DB-041).
 *
 * @param {boolean} noMaster
 * @param {string} feedName
 */
export function everythingWhy(noMaster, feedName) {
  return noMaster
    ? `No membership join at all: every name ${feedName} stores, including the ones no core::universe tier lists (option-only and renamed underlyings).`
    : `No membership join at all: the only setting that reaches stored data whose instrument ${feedName}'s master no longer lists.`;
}

/**
 * THE STORED NAMES A CHOSEN TIER LEAVES OUT, named (DB-041). "F&O
 * underlyings" on GDFL reaches 208 names and leaves out the option-only and
 * renamed underlyings the census holds (ALBK, DHFL, …); the note counted
 * nothing of them. Up to `most` names, sorted, then a count.
 *
 * O(held names): one probe per name the census holds.
 *
 * @param {Iterable<string>} held every underlying the census holds
 * @param {Set<string> | null} members the tier's members, `null` for no join
 * @param {number} [most]
 * @returns {{ count: number, names: string[] } | null} `null` with no join
 */
export function tierLeavesOut(held, members, most = 8) {
  if (members === null) return null;
  const out = [];
  for (const u of held) if (!members.has(u)) out.push(u);
  /* Names out of a Set are distinct, so the default code-unit order is the whole comparison. */
  out.sort();
  return { count: out.length, names: out.slice(0, most) };
}

/**
 * The sentence for `tierLeavesOut`, or `''` when it leaves nothing out.
 *
 * @param {string} tier the universe's label
 * @param {{ count: number, names: string[] } | null} left
 * @param {(n: number) => string} fmt
 */
export function tierLeavesOutText(tier, left, fmt) {
  if (!left || left.count === 0) return '';
  const rest = left.count - left.names.length;
  return `${tier} leaves out ${fmt(left.count)} stored name(s): ${left.names.join(', ')}${rest > 0 ? ` and ${fmt(rest)} more` : ''} — held, outside this tier, and listed under Everything.`;
}

/** A census row with no expiry: a spot series. @param {{expiry?: string | null}} r */
const isSpot = (r) => r.expiry === null || r.expiry === undefined;

/**
 * Whether a census row's series is TRADE-DRIVEN: a bar exists only where a
 * trade printed (D-3336 §2) — a contract, or a share at one second. Its bar
 * count has no fixed denominator, so no "short" is said of it (DB-G63). The
 * rule `dayflags.js` `kindsOf` applies to a day, applied to a month.
 *
 * @param {{segment?: string, kind?: string, timeframe: string, expiry?: string | null}} r
 */
export function tradeDriven(r) {
  const seg = r.kind ?? r.segment;
  return !isSpot(r) || seg === 'FNO' || (seg === 'CASH' && r.timeframe === '1s');
}

/**
 * SPOT SHORTFALLS AGAINST THE FULLEST SERIES OF THE SAME SEGMENT, PER RUNG
 * (DB-G63). The census's `short` is the fullest series at (month, rung) less
 * this one — every segment in one pool, so a share was short of an index's
 * second-by-second count and an index of a share that printed a pre-open
 * minute, and the instrument list summed 1s and 1min shortfalls into one
 * figure of "bars". Here the fullest is keyed (month, rung, segment), a
 * trade-driven series (a share at 1s) is counted apart and never short, and
 * the result is per rung.
 *
 * Two passes: the fullest by (month, rung, segment) over the whole census
 * (`fullestSpot`), then each spot row's shortfall into its underlying's rung
 * (`shortfallsAgainst`). The page keeps them apart (R3-02 repair round 1): the
 * first depends on the census alone and was rebuilt on every universe or
 * window change too — 98.9 ms at 2.1M rows in the re-check — so a change
 * that leaves the census alone now pays only the second.
 *
 * @param {Iterable<any>} all every census row (the fullest is the store's)
 * @param {Iterable<any>} rows the rows to fold (the universe's)
 * @returns {Map<string, Map<string, {months: number, short: number, trade: boolean, seg: string}>>}
 */
export function spotShortfalls(all, rows) {
  return shortfallsAgainst(fullestSpot(all), rows);
}

/**
 * The fullest spot series' bar count at each (month, rung, segment), keyed
 * `month|rung|segment`: the census's own denominator for `shortfallsAgainst`.
 * One pass over `all`; the page derives it from the census alone.
 *
 * @param {Iterable<any>} all every census row
 * @returns {Map<string, number>}
 */
export function fullestSpot(all) {
  /** @type {Map<string, number>} */
  const fullest = new Map();
  for (const r of all) {
    if (!isSpot(r)) continue;
    const k = `${r.month}|${r.timeframe}|${r.kind}`;
    fullest.set(k, Math.max(fullest.get(k) ?? 0, r.rows));
  }
  return fullest;
}

/**
 * Each spot row of `rows` folded into its underlying's rung, short of
 * `fullest` (from `fullestSpot`). One pass over `rows`.
 *
 * @param {Map<string, number>} fullest
 * @param {Iterable<any>} rows the rows to fold (the universe's)
 * @returns {Map<string, Map<string, {months: number, short: number, trade: boolean, seg: string}>>}
 */
export function shortfallsAgainst(fullest, rows) {
  /** @type {Map<string, Map<string, {months: number, short: number, trade: boolean, seg: string}>>} */
  const out = new Map();
  for (const r of rows) {
    if (!isSpot(r)) continue;
    let by = out.get(r.underlying);
    if (!by) out.set(r.underlying, (by = new Map()));
    let a = by.get(r.timeframe);
    if (!a) by.set(r.timeframe, (a = { months: 0, short: 0, trade: tradeDriven(r), seg: String(r.kind ?? '') }));
    a.months += 1;
    if (!a.trade) a.short += Math.max(0, (fullest.get(`${r.month}|${r.timeframe}|${r.kind}`) ?? r.rows) - r.rows);
  }
  return out;
}

/**
 * The shortfall clause of one instrument's title: per rung, in ladder order,
 * "1min: 1,234 bar(s) short over 105 month(s), against the fullest INDEX
 * series of each" — the segment the rung's rows carry — a trade-driven rung
 * said as that. `''` when nothing is short and nothing is trade-driven.
 *
 * THE FIGURE IS A SUM AND SAYS SO (Mac re-check, 17 Oct). It is added up
 * over every month of the rung, and the first draft said "N bar(s) short of
 * the fullest INDEX series of each month", which reads as N short in EACH
 * month: Zerodha NIFTY 1min read "78 bar(s) short of the fullest INDEX series
 * of each month" for 78 bars short across 105 months. The months it sums over
 * are now named beside it.
 *
 * @param {Map<string, {months: number, short: number, trade: boolean, seg: string}> | undefined} by
 * @param {(a: string, b: string) => number} order the rung ladder's comparator
 * @param {(n: number) => string} fmt
 */
export function shortfallText(by, order, fmt) {
  if (!by) return '';
  const parts = [];
  for (const tf of [...by.keys()].sort(order)) {
    const a = /** @type {{months: number, short: number, trade: boolean, seg: string}} */ (by.get(tf));
    if (a.trade) parts.push(`${tf}: trade-driven, a bar only where a trade printed, so no fixed bar count`);
    else if (a.short > 0) parts.push(`${tf}: ${fmt(a.short)} bar(s) short over ${fmt(a.months)} month(s), against the fullest ${a.seg} series of each`);
  }
  return parts.length ? ` (${parts.join('; ')})` : '';
}

/**
 * MONTHS OUTSIDE THE OPERATOR'S WINDOW, COUNTED PER LIST ROW (D-3338; R3-02).
 * The Timeframe and Segment lists counted only the months inside the window,
 * so NIFTY 1day on Zerodha read "105 held" with 36 months of 2015–2017 held
 * and hidden, and a rung held only outside the window was not listed at all.
 *
 * O(out-of-window rows): the rows the window already split off.
 *
 * @param {Iterable<any>} out the census rows outside the window
 * @param {string} underlying the picked instrument
 * @param {(r: any) => string} segOf the row's segment key
 * @param {string} kind the picked segment, `''` for none
 * @returns {{ bySegment: Map<string, number>, byRung: Map<string, number> }}
 */
export function hiddenMonths(out, underlying, segOf, kind) {
  const bySegment = new Map();
  const byRung = new Map();
  for (const r of out) {
    if (r.underlying !== underlying) continue;
    const s = segOf(r);
    bySegment.set(s, (bySegment.get(s) ?? 0) + 1);
    if (kind && s !== kind) continue;
    byRung.set(r.timeframe, (byRung.get(r.timeframe) ?? 0) + 1);
  }
  return { bySegment, byRung };
}

/**
 * The clause a list row adds for its hidden months, `''` for none.
 *
 * @param {number} n
 * @param {{from?: string, to?: string} | null | undefined} window
 * @param {(n: number) => string} fmt
 */
export function hiddenText(n, window, fmt) {
  if (!n) return '';
  return ` ${fmt(n)} more month(s) are held outside your window${window?.from && window.to ? ` (${window.from} to ${window.to})` : ''} and hidden — "Show them, labelled" under the dates brings them in.`;
}

/**
 * A Timeframe row's hover text: the rung's note (`tfNote`, which has no stop
 * of its own) closed as a sentence, then the hidden-months clause. It read
 * "60s bars · 375/session 36 more month(s) …", two sentences run together
 * (R3-02 repair round 1).
 *
 * @param {string} note
 * @param {number} hidden months of this rung held outside the window
 * @param {{from?: string, to?: string} | null | undefined} window
 * @param {(n: number) => string} fmt
 */
export const timeframeTitle = (note, hidden, window, fmt) => `${note}.${hiddenText(hidden, window, fmt)}`;

/**
 * WHAT A STRANDED RUNG DOES TO THE TABLE, IN WORDS THAT ARE TRUE OF IT
 * (DB-020). Every stranded rung said "so no row can match", including over a
 * grid of 4,39,42,544 rows (a stale expiry under Spot, where no row carries
 * an expiry and the gate passes every one) and over a strike pick whose other
 * strikes still matched. The claim is now made only when the table IS empty.
 *
 * THE COUNT IS NAMED FOR WHAT IT COUNTS (Mac re-check, 17 Oct). The caller
 * passes `matched.length` — instrument-months, the census rows the selection
 * matches — and the first draft called that number "the table's row(s)". On
 * the Bars view the table's rows are bars (4,39,42,544 of them for the probe
 * that found DB-020) while the number said was the handful of instrument-months
 * under them: a second false sentence in the line written to remove one.
 *
 * @param {number} months instrument-months the selection matches now
 * @param {number} [kept] for a multi-value rung, how many of its values still match
 * @param {(n: number) => string} [fmt]
 */
export function strandedEffect(months, kept = 0, fmt = String) {
  if (months === 0) return 'so no row can match.';
  if (kept > 0) return `so it matches nothing; the table shows the ${fmt(kept)} other value(s) still chosen on this rung.`;
  return `so it narrows nothing on screen: ${fmt(months)} instrument-month(s) still match the selection.`;
}

/**
 * WHETHER THE SIDE PICK IS STRANDED, judged against what is really above it
 * (DB-020; control 110 of the 12 Oct re-check, NIFTY 02 Jan 2020 weekly).
 *
 * The side list counts under the strike pick (the facet rule), so a strike
 * left over from another expiry — 8,500, off the 02 Jan 2020 chain — emptied
 * the side's base and "Side is set to PE, which the rungs above it no longer
 * offer — so no row can match" stood beside the Strike banner, although that
 * expiry offers PE and only the strike emptied the grid. Here a strike that is
 * no longer on the chain does not count: the side is stranded only when no row
 * of the expiry-gated set carries it among the strikes still picked AND on the
 * chain (or among all of them, when none is).
 *
 * O(rows of the expiry-gated set), once per selection change — the class of
 * the side list it sits beside.
 *
 * @param {string} side the side pick, `''` for none
 * @param {Iterable<{side: string | null, strike: number | null}>} rows the expiry-gated rows
 * @param {Set<string>} strikePick the strikes picked, as text
 * @param {Set<string>} onChain the strikes the chosen expiry still offers, as text
 * @returns {boolean}
 */
export function sideStranded(side, rows, strikePick, onChain) {
  if (!side) return false;
  const live = new Set([...strikePick].filter((k) => onChain.has(k)));
  for (const r of rows) {
    if (r.side !== side) continue;
    /* `onChain` holds the texts of real strikes, so a row with no strike ('null') is never in `live`. */
    if (live.size === 0 || live.has(String(r.strike))) return false;
  }
  return true;
}

/* ---- the whole sentence on a clipped note ----------------------------- */

/**
 * A NOTE CUT TO TWO LINES CARRIES ITS WHOLE SENTENCE AS HOVER TEXT (R3-02).
 * The strip's `.note` clips at two lines (`line-clamp: 2`), and six of its
 * notes carried no `title`, so the clipped half of a refusal was unreadable
 * at 1600 × 1000. A Svelte action: the node's own text, whitespace folded, is
 * its title, kept current as the text changes — one rule for every note, so
 * the hover text cannot drift from what is drawn.
 *
 * @param {{textContent: string | null, title: string}} node
 * @returns {{destroy: () => void}}
 */
export function wholeTitle(node) {
  const set = () => {
    node.title = String(node.textContent ?? '').replace(/\s+/g, ' ').trim();
  };
  set();
  const Observer = /** @type {any} */ (globalThis).MutationObserver;
  const watch = Observer ? new Observer(set) : null;
  watch?.observe(node, { childList: true, characterData: true, subtree: true });
  return { destroy: () => watch?.disconnect() };
}
