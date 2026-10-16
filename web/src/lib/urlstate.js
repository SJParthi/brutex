/**
 * THE SELECTION, IN THE ADDRESS BAR — the arithmetic half, with no runes in
 * it, so `web/tests/urlstate.test.js` can drive it under `node --test`.
 *
 * Extracted for the reason `find.js`, `prefix.js`, `fold.js`, `pick.js` and
 * `receipt.js` were: the page that uses it imports `$lib/ask.js`, node cannot
 * resolve that alias, and a claim that cannot be driven is a claim nobody is
 * checking.
 *
 * # The failure this exists to remove
 *
 * Every control on `/ingest` was in-memory state and nothing wrote it down.
 * Reported from the running page, in the operator's words: *"whatever we have
 * selected before pull, even after pull it needs to be the same — if not, how
 * will I know which one got selected?"*
 *
 * Measured before this existed, by recording the strip, reloading, and diffing:
 *
 *     universe      F&O Underlyings  ->  NIFTY 50        RESET
 *     instruments   All 211 ticked   ->  All 50 ticked   RESET
 *     feed          Groww            ->  Groww           kept, by accident
 *     segments      Spot             ->  Spot            kept, by accident
 *     timeframe     1 minute         ->  1 minute        kept, by accident
 *
 * The four that "survived" survived only because they were already sitting on
 * the default. Nothing was remembered.
 *
 * # Why the FEED is the worst of them, and why it must be written explicitly
 *
 * The others reset to a CONSTANT default. The feed does not: `pick.js` chooses
 * it by "the store decides, most bars wins", so the default is a function of
 * what is on disk — and a pull changes what is on disk. Measured after one run:
 * groww 132,132 bars, dhan 0. So an operator who chose Dhan, pulled into Groww
 * and reloaded came back on GROWW, with every count on the page silently
 * scoped to a feed they never picked.
 *
 * That is the one case where a remembered value must beat a computed one, and
 * it is why `feed` is always written once known rather than only when it
 * differs from a default that will not hold still.
 *
 * # What this module does NOT do
 *
 * It does not validate. A universe token that no longer exists, a rung this
 * build dropped, a feed the server stopped serving — all of them come back out
 * of `decode` verbatim. The vocabularies live on the page and the page owns
 * saying what it could not honour; a parser that quietly dropped an unknown
 * token would be the fallback that hides a failure, and the operator would see
 * a selection silently narrow with nothing on screen to explain it.
 *
 * `decode` therefore distinguishes ABSENT from EMPTY, and the difference
 * carries meaning: no `seg=` at all means "the page keeps its default", while
 * `seg=` with nothing after it means "the operator ticked none".
 */

import { checkText, checkDay, checkTime, checkWhole, checkWord, windowWhy, timeWindowWhy, quote, MAX_PAGE } from './db/input-check.js';

/** The keys this module reads and writes, in the order `encode` emits them. */
export const KEYS = ['feed', 'universe', 'member', 'seg', 'tf', 'from', 'to'];

/**
 * A set of tokens as one comma-joined field, sorted.
 *
 * SORTED, because `CLAUDE.md` §3 rule 5 is idempotence: the same selection must
 * produce the same URL byte for byte, and `Set` iteration order is insertion
 * order — so ticking A then B would otherwise write a different address from
 * ticking B then A, and two bookmarks of one selection would not compare equal.
 *
 * @param {Iterable<string> | null | undefined} tokens
 * @returns {string}
 */
export function packSet(tokens) {
  if (!tokens) return '';
  const seen = new Set();
  for (const t of tokens) {
    const s = String(t).trim();
    if (s) seen.add(s);
  }
  return [...seen].sort().join(',');
}

/**
 * One comma-joined field back to a Set. Empty string is an EMPTY SET, which is
 * not the same answer as the field being absent — see the module header.
 *
 * @param {string | null | undefined} field
 * @returns {Set<string>}
 */
export function unpackSet(field) {
  const out = new Set();
  for (const part of String(field ?? '').split(',')) {
    const s = part.trim();
    if (s) out.add(s);
  }
  return out;
}

/**
 * @typedef {object} Selection
 * @property {Iterable<string>} [feeds]    feed wire names the run asks.
 * @property {string} [universe]           the page's own universe token.
 * @property {Iterable<string> | null} [members]
 *           the TICKED instrument symbols, or `null` for "every one of them".
 *           Null is written as an absent field, which is the same thing
 *           `wireBodyFor` does on the wire: naming all and naming none ask for
 *           the same set, and 750 fields to say "everything" is waste.
 * @property {Iterable<string>} [segs]     segment keys.
 * @property {Iterable<string>} [rungs]    rung directions.
 * @property {string} [from]               ISO day.
 * @property {string} [to]                 ISO day.
 */

/**
 * A selection as a query string, with no leading `?`.
 *
 * A field whose value is empty or `undefined` is OMITTED rather than written
 * blank, so a first load carries a bare `/ingest` and a shared address carries
 * exactly the choices somebody made.
 *
 * @param {Selection} sel
 * @returns {string}
 */
export function encode(sel) {
  const p = new URLSearchParams();
  // A SET FIELD IS WRITTEN WHENEVER IT WAS SUPPLIED, EVEN EMPTY, and that is
  // the whole of the absent/empty distinction surviving the round trip. The
  // first cut of this omitted any empty set, so `{segs: new Set()}` -- the
  // operator having deliberately un-ticked every segment, a state the ladder
  // gates on and names -- encoded to nothing and came back as the DEFAULT on
  // the next load. `urlstate.test.js` caught it on `same('seg=', '')`.
  //
  // `null` is the one value that still means "omit", and it reads as ALL: it is
  // what `members` uses for "every instrument", the same rule `wireBodyFor`
  // follows on the wire, where naming all and naming none ask for one set.
  /** @param {string} k @param {Iterable<string> | null | undefined} v */
  const putSet = (k, v) => {
    if (v === null || v === undefined) return;
    p.set(k, packSet(v));
  };
  // A SCALAR HAS NO "EMPTY BUT CHOSEN" STATE -- there is no such thing as
  // deliberately picking no universe and no date -- so a blank one is omitted.
  /** @param {string} k @param {string | null | undefined} v */
  const putText = (k, v) => {
    const s = v === null || v === undefined ? '' : String(v).trim();
    if (s) p.set(k, s);
  };
  putSet('feed', sel?.feeds);
  putText('universe', sel?.universe);
  putSet('member', sel?.members);
  putSet('seg', sel?.segs);
  putSet('tf', sel?.rungs);
  putText('from', sel?.from);
  putText('to', sel?.to);
  return p.toString();
}

/**
 * A query string back to a partial selection.
 *
 * ABSENT IS `undefined` AND EMPTY IS AN EMPTY SET. A caller must be able to
 * tell "the operator said none" from "the operator said nothing", because the
 * first is a choice to honour and the second is a default to keep.
 *
 * @param {string | null | undefined} search a query string, with or without `?`
 * @returns {{feeds?: Set<string>, universe?: string, members?: Set<string>,
 *            segs?: Set<string>, rungs?: Set<string>, from?: string, to?: string}}
 */
export function decode(search) {
  const p = new URLSearchParams(String(search ?? '').replace(/^\?/, ''));
  /** @type {Record<string, any>} */
  const out = {};
  if (p.has('feed')) out.feeds = unpackSet(p.get('feed'));
  if (p.has('universe')) out.universe = String(p.get('universe')).trim();
  if (p.has('member')) out.members = unpackSet(p.get('member'));
  if (p.has('seg')) out.segs = unpackSet(p.get('seg'));
  if (p.has('tf')) out.rungs = unpackSet(p.get('tf'));
  if (p.has('from')) out.from = String(p.get('from')).trim();
  if (p.has('to')) out.to = String(p.get('to')).trim();
  return out;
}

/**
 * Whether two query strings name the same selection.
 *
 * Used to decide whether the address bar needs rewriting at all. A
 * `replaceState` per keystroke is not free and it is not silent — it is a
 * navigation entry the browser has to service — and this makes the write
 * conditional on the selection actually having changed rather than on a
 * derivation having re-run.
 *
 * @param {string} a
 * @param {string} b
 * @returns {boolean}
 */
export function same(a, b) {
  return encodeAgain(a) === encodeAgain(b);
}

/**
 * One query string normalised through this module's own rules, so two spellings
 * of one selection compare equal.
 *
 * @param {string} search
 * @returns {string}
 */
function encodeAgain(search) {
  return encode(decode(search));
}

/* ======================================================================
   /db's SELECTION IN THE ADDRESS BAR (WP-50; D-3337 item 4, D-3338)
   ----------------------------------------------------------------------
   No /db selection was in the URL (DB-077): a reload returned to the first
   instrument, nothing could be bookmarked or shared, and no log, audit or gap
   row could open the series it names. The names are D-3337's, in its order:

     feed  seg  sym  key  tf  exp  k (strikes in paisa, ONE comma field)  side
     from  to (YYYY-MM-DD)  tfrom  tto (HH:MM or HH:MM:SS)
     sort  dir  page  view  oow (D-3338)

   and, from R3-01, every other choice the page offers, so a reload, a
   bookmark, a shared link, Back and Forward restore all of it:

     universe (a membership universe key)  mny (moneyness rungs, one comma
     field)  spot (`prev` = last spot at or before t, D-3328)  rows (rows per
     page when not Fit)

   `sym` is the exact underlying the Instrument rung picked. `key` is an
   instrument key (`NSE-FNO-NIFTY-2019-03-28-1150000-CE`, `NSE-INDEX-NIFTY`)
   for a link from a row elsewhere; the page reads its rungs off it. A field
   is written only when it carries something, and a default (page 1, the Bars
   view, newest first, hidden out-of-window months) is not written, so a
   bookmark names exactly the choices that differ from opening the page.

   UNLIKE `decode` ABOVE, THIS ONE CHECKS SHAPES AND NAMES WHAT IT REFUSED.
   A malformed day or strike cannot be honoured, and dropping it silently
   would narrow nothing while the address claims it does; `refused` lists
   each one with its reason so the page can say so (CLAUDE.md §4). Whether a
   well-formed value exists in this feed's store is the page's question.
   ====================================================================== */

/** /db's query keys, in the order `encodeDb` writes them (D-3337 item 4, extended by R3-01). */
export const DB_KEYS = [
  'feed', 'universe', 'seg', 'sym', 'key', 'tf', 'exp', 'k', 'mny', 'side', 'spot',
  'from', 'to', 'tfrom', 'tto', 'sort', 'dir', 'rows', 'page', 'view', 'oow'
];

/**
 * The values `encodeDb` leaves out because the page opens on them: page 1,
 * the Bars view, newest first, the window applied, the exact-second spot
 * (D-2807) and rows sized to fit the window.
 */
export const DB_DEFAULTS = { sort: 'ts', dir: 'desc', page: 1, view: 'bars', oow: false, spot: 'exact', rows: 'fit' };

/**
 * The most characters a list field (`k`, `mny`) is read for: 64 values of 16
 * characters. A BOUND, NOT A VENDOR FACT — a chain lists far fewer strikes
 * than a person can tick, and a longer field is refused by its length before
 * it is split.
 */
export const MAX_LIST_FIELD = 16 * 64;

/**
 * The most refusals one address names one by one; the rest are counted in a
 * closing entry. A BOUND: a link holds a handful of fields, and a notice that
 * grows with the link is not read.
 */
export const MAX_REFUSED = 12;

/** How many unknown field names the one sentence about them quotes. */
const UNKNOWN_NAMED = 8;

const PAISA = /^\d{1,15}$/;
/** A moneyness rung: ATM, ITM, OTM, or a numbered rung such as ITM-1 / OTM+2. */
const RUNG = /^(ATM|ITM|OTM)([+-]\d{1,3})?$/;

/**
 * @typedef {object} DbSelection
 * @property {string} [feed]
 * @property {string} [universe] a membership universe key (`n50`, `fno`, …); '' is Everything
 * @property {string} [seg]   a segment key (`spot`, `options`, `options-live`, …)
 * @property {string} [sym]   the exact underlying
 * @property {string} [key]   an instrument key, for a link from elsewhere
 * @property {string} [tf]    the store's rung spelling (`1s`, `1min`, `1day`)
 * @property {string} [exp]   `YYYY-MM-DD`
 * @property {Iterable<string|number>} [k] strikes in paisa
 * @property {Iterable<string>} [mny] moneyness rungs (`ATM`, `ITM-1`, …)
 * @property {string} [side]  `CE` or `PE`
 * @property {string} [spot]  `exact` (default) or `prev` (last spot at or before t, D-3328)
 * @property {string} [from]  `YYYY-MM-DD`
 * @property {string} [to]    `YYYY-MM-DD`
 * @property {string} [tfrom] `HH:MM` or `HH:MM:SS`
 * @property {string} [tto]   `HH:MM` or `HH:MM:SS`
 * @property {string} [sort]  a bar column key
 * @property {'asc'|'desc'} [dir]
 * @property {number | string} [rows] rows per page when fixed; '' or `fit` is Fit
 * @property {number} [page]
 * @property {string} [view]  `bars` or `census`
 * @property {boolean} [oow]  show months outside the operator's window
 */

/**
 * A /db selection as a query string, no leading `?`. Idempotent: one
 * selection is one address, byte for byte — strikes are sorted numerically
 * and deduplicated, moneyness rungs sorted, and keys written in `DB_KEYS`
 * order.
 *
 * STRIKES ARE ONE FIELD, `k=1160000,1200000` (R3-01). They were one `k=` per
 * strike, and the server refuses any query that names a field twice on every
 * route but two (`crates/api/src/server.rs` `request_bounds_refusal`,
 * probeapi-5) — so the address this page wrote while two strikes were ticked
 * answered 400 on reload and on a shared link. One field cannot be repeated.
 * The comma is written as itself, not `%2C`: it is legal in a query, every
 * reader (`URLSearchParams`) decodes both alike, and the address stays
 * readable.
 *
 * @param {DbSelection} sel
 * @returns {string}
 */
export function encodeDb(sel) {
  const p = new URLSearchParams();
  /** @param {string} k @param {unknown} v */
  const put = (k, v) => {
    const s = v === null || v === undefined ? '' : String(v).trim();
    if (s) p.set(k, s);
  };
  put('feed', sel?.feed);
  put('universe', sel?.universe);
  put('seg', sel?.seg);
  put('sym', sel?.sym);
  put('key', sel?.key);
  put('tf', sel?.tf);
  put('exp', sel?.exp);
  const strikes = [...new Set([...(sel?.k ?? [])].map((x) => String(x).trim()).filter((x) => PAISA.test(x)))]
    .sort((a, b) => Number(a) - Number(b));
  put('k', strikes.join(','));
  const rungs = [...new Set([...(sel?.mny ?? [])].map((x) => String(x).trim()).filter((x) => RUNG.test(x)))].sort();
  put('mny', rungs.join(','));
  put('side', sel?.side);
  if (sel?.spot && sel.spot !== DB_DEFAULTS.spot) put('spot', sel.spot);
  put('from', sel?.from);
  put('to', sel?.to);
  put('tfrom', sel?.tfrom);
  put('tto', sel?.tto);
  const sort = sel?.sort || DB_DEFAULTS.sort;
  const dir = sel?.dir || DB_DEFAULTS.dir;
  if (sort !== DB_DEFAULTS.sort || dir !== DB_DEFAULTS.dir) {
    put('sort', sort);
    put('dir', dir);
  }
  if (sel?.rows !== undefined && sel.rows !== null && sel.rows !== '' && String(sel.rows) !== DB_DEFAULTS.rows) put('rows', sel.rows);
  if (Number.isSafeInteger(sel?.page) && /** @type {number} */ (sel?.page) > 1) put('page', sel?.page);
  if (sel?.view && sel.view !== DB_DEFAULTS.view) put('view', sel.view);
  if (sel?.oow) put('oow', '1');
  return p.toString().replace(/%2C/g, ',');
}

/**
 * A query string back to a /db selection, plus every field it could not read.
 * Absent fields are absent (the page keeps its default); a malformed one is
 * left out of `sel` and named in `refused`, in the SAME sentence the box on
 * the page says for the same value (`db/input-check.js`).
 *
 * What is checked here is SHAPE: a calendar day, a time, a whole number, a
 * word from a closed set, a length. Whether a well-formed value exists — a
 * strike this expiry lists, a day this series holds, a universe or a sort
 * this build knows — is the page's question, asked once the census has
 * answered (`input-check.js` `vetAddress`, the link reader).
 *
 * Strikes are read from one comma-joined `k` (what `encodeDb` writes) and,
 * for a link written before R3-01, from repeated `k` fields alike.
 *
 * @param {string | null | undefined} search
 * @returns {{sel: DbSelection & {k?: string[], mny?: string[]}, refused: Array<{key: string, value: string, why: string}>}}
 */
export function decodeDb(search) {
  const p = new URLSearchParams(String(search ?? '').replace(/^\?/, ''));
  /** @type {Record<string, any>} */
  const sel = {};
  /** @type {Array<{key: string, value: string, why: string}>} */
  const refused = [];
  /* EACH PROBLEM ONCE, AND A BOUNDED FEW OF THEM (R3-01). `mny=a,a,a,…` was
     one identical sentence per comma and `x0=1&x1=1&…` one per field — the
     notice under the controls grew with the link (second agent, shadow 13
     Oct). A value refused twice is named once; past `MAX_REFUSED` the rest
     are COUNTED in one closing entry, never dropped in silence. */
  const seen = new Set();
  let more = 0;
  /** @param {string} key @param {string} value @param {string} why */
  const refuse = (key, value, why) => {
    const id = `${key}\u0000${value}`;
    if (seen.has(id)) return;
    seen.add(id);
    if (refused.length >= MAX_REFUSED) {
      more += 1;
      return;
    }
    refused.push({ key, value: value.length > 64 ? `${value.slice(0, 64)}…` : value, why });
  };
  /** @param {string} k */
  const raw = (k) => (p.has(k) ? String(p.get(k)) : '');
  for (const k of ['feed', 'universe', 'seg', 'sym', 'key', 'tf', 'sort', 'view']) {
    const v = raw(k);
    if (!v.trim()) continue;
    const r = checkText(v, { label: k });
    if (r.ok) sel[k] = r.value;
    else refuse(k, v, r.why);
  }
  if (sel.sym) sel.sym = sel.sym.toUpperCase();
  for (const k of ['exp', 'from', 'to']) {
    const v = raw(k).trim();
    if (!v) continue;
    const r = checkDay(v, { label: k });
    if (r.ok) sel[k] = r.value;
    else refuse(k, v, r.why);
  }
  const days = windowWhy(sel.from ?? '', sel.to ?? '');
  if (days) {
    refuse('from', `${sel.from}..${sel.to}`, days);
    delete sel.from;
    delete sel.to;
  }
  for (const k of ['tfrom', 'tto']) {
    const v = raw(k).trim();
    if (!v) continue;
    const r = checkTime(v, { label: k });
    if (r.ok) sel[k] = r.value;
    else refuse(k, v, r.why);
  }
  const times = timeWindowWhy(sel.tfrom ?? '', sel.tto ?? '');
  if (times) {
    refuse('tfrom', `${sel.tfrom}..${sel.tto}`, times);
    delete sel.tfrom;
    delete sel.tto;
  }
  const strikes = [];
  for (const field of p.getAll('k')) {
    if (field.length > MAX_LIST_FIELD) {
      refuse('k', field, `k is ${field.length.toLocaleString('en-IN')} characters long, and /db reads at most ${MAX_LIST_FIELD}`);
      continue;
    }
    for (const part of field.split(',')) {
      const v = part.trim();
      if (!v) continue;
      if (PAISA.test(v)) strikes.push(v);
      else refuse('k', v, `k ${quote(v)} is not a strike in whole paisa (11,600.00 is 1160000)`);
    }
  }
  if (strikes.length) sel.k = [...new Set(strikes)].sort((a, b) => Number(a) - Number(b));
  const mny = [];
  const rungField = raw('mny');
  if (rungField.length > MAX_LIST_FIELD) {
    refuse('mny', rungField, `mny is ${rungField.length.toLocaleString('en-IN')} characters long, and /db reads at most ${MAX_LIST_FIELD}`);
  }
  for (const part of rungField.length > MAX_LIST_FIELD ? [] : rungField.split(',')) {
    const v = part.trim();
    if (!v) continue;
    if (RUNG.test(v)) mny.push(v);
    else refuse('mny', v, `mny ${quote(v)} is not a moneyness rung (ATM, ITM, OTM, ITM-1, OTM+2, …)`);
  }
  if (mny.length) sel.mny = [...new Set(mny)].sort();
  /** @param {string} key @param {string[]} allowed @param {(v: string) => string} [fold] */
  const word = (key, allowed, fold = (v) => v) => {
    const v = raw(key).trim();
    if (!v) return;
    const r = checkWord(fold(v), allowed, { label: key });
    if (r.ok) sel[key] = r.value;
    else refuse(key, v, r.why);
  };
  word('side', ['CE', 'PE'], (v) => v.toUpperCase());
  word('dir', ['asc', 'desc']);
  word('spot', ['exact', 'prev']);
  const page = raw('page').trim();
  if (page) {
    const r = checkWhole(page, { label: 'page', min: 1, max: MAX_PAGE });
    if (r.ok) sel.page = r.value;
    else refuse('page', page, r.why);
  }
  const rows = raw('rows').trim();
  if (rows) {
    if (rows === 'fit') sel.rows = 'fit';
    else {
      const r = checkWhole(rows, { label: 'rows', min: 1, max: MAX_PAGE });
      if (r.ok) sel.rows = r.value;
      else refuse('rows', rows, r.why);
    }
  }
  const oow = raw('oow').trim();
  if (oow) {
    if (oow === '1' || oow === '0') sel.oow = oow === '1';
    else refuse('oow', oow, `oow ${quote(oow)} is 1 (show months outside the window) or 0`);
  }
  if (sel.view) {
    const r = checkWord(sel.view, ['bars', 'census'], { label: 'view' });
    if (!r.ok) {
      refuse('view', sel.view, r.why);
      delete sel.view;
    }
  }
  /* A FIELD THIS PAGE DOES NOT READ IS NAMED, not ignored in silence: a
     misspelt `strike=` would otherwise open every strike while the address
     reads as if it asked for one. */
  const known = new Set(DB_KEYS);
  /* ONE SENTENCE FOR EVERY FIELD IT DOES NOT READ, naming the first few and
     counting the rest: forty unknown fields were forty copies of the field
     list (R3-01, shadow 16 Oct). */
  const unknown = [...new Set(p.keys())].filter((k) => !known.has(k));
  if (unknown.length > 0) {
    const named = unknown.slice(0, UNKNOWN_NAMED);
    const rest = unknown.length - named.length;
    refuse(
      unknown[0],
      unknown.length === 1 ? raw(unknown[0]) : named.join(','),
      `${named.map((k) => quote(k)).join(', ')}${rest > 0 ? ` and ${rest.toLocaleString('en-IN')} more` : ''} ${unknown.length === 1 ? 'is not a field' : 'are not fields'} of a /db address (it reads ${DB_KEYS.join(', ')})`
    );
  }
  if (more > 0) {
    refused.push({
      key: '…',
      value: '',
      why: `and ${more.toLocaleString('en-IN')} more value(s) this page could not read, not listed one by one`
    });
  }
  return { sel, refused };
}

/**
 * A link into /db for one selection — what /gaps, /audit, /logs and /sources
 * put on a row (D-3337 item 4).
 *
 * @param {DbSelection} sel
 */
export function dbHref(sel) {
  const q = encodeDb(sel);
  return q ? `/db?${q}` : '/db';
}

/* ======================================================================
   BACK AND FORWARD (R3-01; DB-077)
   ----------------------------------------------------------------------
   The address was written with `replaceState` only, so a session of
   selections was ONE history entry: Back left /db altogether, and Back from
   the next page returned to /db's first address, not the last selection.

   Worse, SvelteKit's shallow entries record the URL the PAGE was loaded with
   (`PAGE_URL_KEY`, kit `client.js` `pushState`/`replaceState`), and a
   popstate navigation back onto /db hands the page that URL as `page.url` —
   so a reader of `page.url.search` re-opened the address /db was ENTERED
   with (a bare `/db` from the top bar) while the address bar showed the
   selection. The browser's own `location.search` is the address on screen,
   and `addressNow` is what every reader asks.

   `dbHistory` decides, for each settled selection, whether its address is a
   NEW entry or a correction of the current one:

     - the first write after the page opens completes the address it opened
       with (defaults filled in): a correction, never an entry, or Back would
       step from `/db?feed=…&sym=NIFTY…` to a bare `/db` that opens the same;
     - a write after Back or Forward restored an entry is a correction too:
       pushing there would erase every Forward entry;
     - a write with no press or key since the last one (a seed filling a
       rung, the pager clamping past the end) is a correction — and only a
       press or key ON A CONTROL counts (`isChoiceGesture`): a click on blank
       space, Tab, Escape or a modifier alone chooses nothing;
     - otherwise — the operator pressed or typed something and the selection
       moved — it is a new entry. One press is one entry however many
       settled states it passes through: the press is spent by the first.
   ====================================================================== */

/** How long a press or key stays the cause of the next write, ms. A BOUND, not
 *  a measured figure: every choice on /db moves the selection in the press's
 *  own task (a pick sets a field `encodeDb` writes), so the first write after
 *  a press lands within milliseconds; a selection that first moves later than
 *  this is written as a correction — the address is still exact, only that
 *  step's own Back entry is not made (D-3337 as amended by R3-01). */
export const GESTURE_MS = 30_000;

/**
 * What on /db can make a choice: a form control, a link, a label (which
 * toggles its box), a menu or grid item, or anything that takes focus. A
 * press anywhere else — the page's background, a heading, a chart's canvas —
 * changes no selection and arms nothing (R3-01 repair 1: any pointerdown or
 * keydown used to arm a push, so a seed or a clamp settling within
 * `GESTURE_MS` of a click on blank space became a Back entry of its own).
 */
export const CHOICE_TARGET = [
  'input', 'select', 'textarea', 'button', 'a[href]', 'label', 'summary',
  '[role="option"]', '[role="tab"]', '[role="row"]', '[role="gridcell"]', '[role="button"]',
  '[role="checkbox"]', '[role="radio"]', '[role="switch"]', '[role="menuitem"]',
  '[role="menuitemradio"]', '[role="menuitemcheckbox"]', '[role="slider"]',
  '[role="spinbutton"]', '[role="combobox"]', '[role="listbox"]', '[tabindex]'
].join(', ');

/** Keys that choose nothing: a modifier alone, focus moving on, a dismissal. */
const NOT_A_CHOICE_KEY = new Set([
  'Tab', 'Shift', 'Control', 'Alt', 'AltGraph', 'Meta', 'OS', 'Super', 'Hyper', 'Fn', 'FnLock',
  'CapsLock', 'NumLock', 'ScrollLock', 'Escape', 'ContextMenu', 'Dead', 'Unidentified'
]);

/**
 * Whether one `pointerdown` or `keydown` can be the cause of the next
 * address: the primary button or a key that is not in `NOT_A_CHOICE_KEY`,
 * on (or inside) a `CHOICE_TARGET`. Pure: it reads only the event's own
 * fields, so `web/tests/db-history.test.js` drives it with plain objects.
 *
 * @param {{type?: string, key?: string, button?: number, target?: any} | null | undefined} e
 */
export function isChoiceGesture(e) {
  if (!e || typeof e !== 'object') return false;
  const t = e.target;
  const control = t && typeof t.closest === 'function' ? t.closest(CHOICE_TARGET) : null;
  if (!control) return false;
  if (e.type === 'pointerdown') return (e.button ?? 0) === 0;
  if (e.type === 'keydown') {
    const key = String(e.key ?? '');
    return key !== '' && !NOT_A_CHOICE_KEY.has(key) && !/^F\d{1,2}$/.test(key);
  }
  return false;
}

/**
 * The address on screen, without `?`: the browser's `location.search` where
 * there is one (see above), else `fallback`.
 *
 * @param {string} [fallback]
 */
export function addressNow(fallback = '') {
  const s = typeof location !== 'undefined' && location ? location.search : fallback;
  return String(s ?? '').replace(/^\?/, '');
}

/**
 * @typedef {object} HistoryIo
 * @property {() => string} search the address on screen (`addressNow`)
 * @property {(kind: 'push' | 'replace', q: string) => void} write
 * @property {(search: string) => void} apply restore the selection an entry names
 * @property {() => number} [clock] ms
 */

/**
 * The history rule above, with no runes and no DOM, so
 * `web/tests/db-history.test.js` drives the same object the page holds.
 *
 * @param {HistoryIo} io
 */
export function dbHistory(io) {
  const clock = io.clock ?? (() => Date.now());
  let armed = false;
  let restoring = false;
  let pressedAt = -Infinity;
  return {
    /**
     * A press or a key on the page. With its event, only one that
     * `isChoiceGesture` accepts counts; without one (a caller that has
     * already decided, as the tests do) it counts.
     * @param {Parameters<typeof isChoiceGesture>[0]} [e]
     */
    gesture(e) {
      if (e !== undefined && !isChoiceGesture(e)) return false;
      pressedAt = clock();
      return true;
    },
    /**
     * Back or Forward landed on another entry of this page: restore what it
     * names. `current` is the selection on screen, encoded.
     * @param {string} current
     */
    popped(current) {
      const there = io.search();
      if (encodeDb(decodeDb(there).sel) === current) return false;
      restoring = true;
      io.apply(there);
      return true;
    },
    /**
     * The selection settled on `q` (an `encodeDb` string). Writes the address
     * when it differs and says how.
     * @param {string} q
     * @returns {'none' | 'push' | 'replace'}
     */
    settled(q) {
      const now = io.search();
      const pressed = clock() - pressedAt <= GESTURE_MS;
      /** @type {'none' | 'push' | 'replace'} */
      const kind = now === q ? 'none' : armed && !restoring && pressed ? 'push' : 'replace';
      if (kind !== 'none') io.write(kind, q);
      armed = true;
      if (kind !== 'none' || now === q) restoring = false;
      if (kind === 'push' || kind === 'replace') pressedAt = -Infinity;
      return kind;
    },
    get state() {
      return { armed, restoring, pressed: clock() - pressedAt <= GESTURE_MS };
    }
  };
}
