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

     feed  seg  sym  key  tf  exp  k (repeatable, a strike in paisa)  side
     from  to (YYYY-MM-DD)  tfrom  tto (HH:MM or HH:MM:SS)
     sort  dir  page  view  oow (D-3338)

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

/** /db's query keys, in the order `encodeDb` writes them. */
export const DB_KEYS = [
  'feed', 'seg', 'sym', 'key', 'tf', 'exp', 'k', 'side', 'from', 'to',
  'tfrom', 'tto', 'sort', 'dir', 'page', 'view', 'oow'
];

/** The values `encodeDb` leaves out because the page opens on them. */
export const DB_DEFAULTS = { sort: 'ts', dir: 'desc', page: 1, view: 'bars', oow: false };

const DAY = /^\d{4}-\d{2}-\d{2}$/;
const TIME = /^([01]\d|2[0-3]):[0-5]\d(:[0-5]\d)?$/;
const PAISA = /^\d{1,15}$/;

/**
 * @typedef {object} DbSelection
 * @property {string} [feed]
 * @property {string} [seg]   a segment key (`spot`, `options`, `options-live`, …)
 * @property {string} [sym]   the exact underlying
 * @property {string} [key]   an instrument key, for a link from elsewhere
 * @property {string} [tf]    the store's rung spelling (`1s`, `1min`, `1day`)
 * @property {string} [exp]   `YYYY-MM-DD`
 * @property {Iterable<string|number>} [k] strikes in paisa
 * @property {string} [side]  `CE` or `PE`
 * @property {string} [from]  `YYYY-MM-DD`
 * @property {string} [to]    `YYYY-MM-DD`
 * @property {string} [tfrom] `HH:MM` or `HH:MM:SS`
 * @property {string} [tto]   `HH:MM` or `HH:MM:SS`
 * @property {string} [sort]  a bar column key
 * @property {'asc'|'desc'} [dir]
 * @property {number} [page]
 * @property {string} [view]  `bars` or `census`
 * @property {boolean} [oow]  show months outside the operator's window
 */

/**
 * A /db selection as a query string, no leading `?`. Idempotent: one
 * selection is one address, byte for byte — strikes are sorted numerically
 * and deduplicated, and keys are written in `DB_KEYS` order.
 *
 * @param {DbSelection} sel
 * @returns {string}
 */
export function encodeDb(sel) {
  const p = new URLSearchParams();
  /** @param {string} k @param {unknown} v */
  const put = (k, v) => {
    const s = v === null || v === undefined ? '' : String(v).trim();
    if (s) p.append(k, s);
  };
  put('feed', sel?.feed);
  put('seg', sel?.seg);
  put('sym', sel?.sym);
  put('key', sel?.key);
  put('tf', sel?.tf);
  put('exp', sel?.exp);
  const strikes = [...new Set([...(sel?.k ?? [])].map((x) => String(x).trim()).filter((x) => PAISA.test(x)))]
    .sort((a, b) => Number(a) - Number(b));
  for (const k of strikes) p.append('k', k);
  put('side', sel?.side);
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
  if (Number.isSafeInteger(sel?.page) && /** @type {number} */ (sel?.page) > 1) put('page', sel?.page);
  if (sel?.view && sel.view !== DB_DEFAULTS.view) put('view', sel.view);
  if (sel?.oow) put('oow', '1');
  return p.toString();
}

/**
 * A query string back to a /db selection, plus every field it could not read.
 * Absent fields are absent (the page keeps its default); a malformed one is
 * left out of `sel` and named in `refused`.
 *
 * @param {string | null | undefined} search
 * @returns {{sel: DbSelection & {k?: string[]}, refused: Array<{key: string, value: string, why: string}>}}
 */
export function decodeDb(search) {
  const p = new URLSearchParams(String(search ?? '').replace(/^\?/, ''));
  /** @type {Record<string, any>} */
  const sel = {};
  /** @type {Array<{key: string, value: string, why: string}>} */
  const refused = [];
  /** @param {string} k */
  const text = (k) => (p.has(k) ? String(p.get(k)).trim() : '');
  for (const k of ['feed', 'seg', 'sym', 'key', 'tf', 'sort', 'view']) {
    const v = text(k);
    if (v) sel[k] = v;
  }
  if (sel.sym) sel.sym = sel.sym.toUpperCase();
  for (const k of ['exp', 'from', 'to']) {
    const v = text(k);
    if (!v) continue;
    if (DAY.test(v) && !Number.isNaN(Date.parse(`${v}T00:00:00Z`)) && new Date(`${v}T00:00:00Z`).toISOString().slice(0, 10) === v) sel[k] = v;
    else refused.push({ key: k, value: v, why: 'not a calendar day in YYYY-MM-DD form' });
  }
  for (const k of ['tfrom', 'tto']) {
    const v = text(k);
    if (!v) continue;
    if (TIME.test(v)) sel[k] = v;
    else refused.push({ key: k, value: v, why: 'not a time in HH:MM or HH:MM:SS form' });
  }
  const strikes = [];
  for (const raw of p.getAll('k')) {
    const v = String(raw).trim();
    if (!v) continue;
    if (PAISA.test(v)) strikes.push(v);
    else refused.push({ key: 'k', value: v, why: 'not a strike in whole paisa' });
  }
  if (strikes.length) sel.k = [...new Set(strikes)].sort((a, b) => Number(a) - Number(b));
  const side = text('side').toUpperCase();
  if (side) {
    if (side === 'CE' || side === 'PE') sel.side = side;
    else refused.push({ key: 'side', value: side, why: 'a side is CE or PE' });
  }
  const dir = text('dir');
  if (dir) {
    if (dir === 'asc' || dir === 'desc') sel.dir = dir;
    else refused.push({ key: 'dir', value: dir, why: 'a direction is asc or desc' });
  }
  const page = text('page');
  if (page) {
    if (/^\d{1,9}$/.test(page) && Number(page) >= 1) sel.page = Number(page);
    else refused.push({ key: 'page', value: page, why: 'a page is a whole number from 1' });
  }
  const oow = text('oow');
  if (oow) {
    if (oow === '1' || oow === '0') sel.oow = oow === '1';
    else refused.push({ key: 'oow', value: oow, why: 'oow is 1 or 0' });
  }
  if (sel.view && sel.view !== 'bars' && sel.view !== 'census') {
    refused.push({ key: 'view', value: sel.view, why: 'a view is bars or census' });
    delete sel.view;
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
