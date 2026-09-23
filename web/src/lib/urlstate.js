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
