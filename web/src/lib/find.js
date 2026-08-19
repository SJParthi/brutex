/**
 * FINDING ONE SYMBOL IN A TABLE TOO LONG TO READ — the arithmetic half, with
 * no runes in it, so `web/tests/find.test.js` can drive it under `node --test`.
 *
 * Extracted for the reason `prefix.js`, `fold.js`, `completeness.js`,
 * `receipt.js` and `bps.js` were: the modules that hold this kind of rule
 * import `$lib/ask.js`, node cannot resolve that alias, and a claim that cannot
 * be driven is a claim nobody is checking.
 *
 * # What this is for, and why `prefix.js` is not enough
 *
 * `prefix.js` answers "which symbol STARTS with what I typed" — the right
 * question for a typeahead, where the operator knows the name and wants to stop
 * spelling it. The census asks a different one. With NIFTY Total Market ticked
 * it draws 750 names across 30 pages, and the operator's questions are "do I
 * hold TCS?" and "show me the banks". The second is not a prefix: AXISBANK,
 * HDFCBANK, ICICIBANK and KOTAKBANK all CONTAIN `BANK` and not one of them
 * begins with it.
 *
 * So this indexes INFIXES, not prefixes. `prefix.js` keeps its job and keeps
 * its callers; neither module is the other with a flag on it.
 *
 * # Where the cost is, measured rather than asserted
 *
 * THE INDEX IS BUILT OVER DISTINCT SYMBOLS, NOT OVER ROWS, and that is the
 * whole reason an infix index is affordable here. A census row is one
 * (symbol × segment × timeframe) combination, so 750 names at three segments
 * and six rungs is 13,500 rows — but still 750 symbols. Indexing every 1..4
 * character infix of every symbol costs O(symbols × length × MAX_PREFIX) once
 * per change to the selection, and a keystroke after that is one `Map.get`.
 *
 * `find.test.js` drives both halves of that. What it also records, because
 * CLAUDE.md §3 rule 6 asks for honest limits rather than flattering ones: on
 * 13,500 census rows the SORT the caller runs afterwards measured 1,464,602 ns,
 * against 1,290 ns for the dearest probe here and 28 ns for the cheapest. This
 * index is not what makes that table fast. What makes it fast is that a query
 * narrows the set BEFORE the sort sees it — 13,500 rows sorted is 1.46 ms and
 * 25 rows sorted is 746 ns — so the honest claim for this module is that it
 * costs nothing next to the work it saves, not that it is the work that
 * mattered.
 *
 * # The one place cost is not constant
 *
 * Past `MAX_PREFIX` characters the probe filters the bucket it just fetched,
 * exactly as `prefix.js` does. That cost tracks the BUCKET and not the
 * catalogue, and the test pins it.
 */

// ONE SPELLING OF THE BOUND. `web/tests/twofrontends.test.js` already pins
// `prefix.js`'s MAX_PREFIX against the one `web/typeahead.js` builds to, on the
// grounds that two front ends indexing to different depths is a difference
// nobody can see from either. A third copy here would be the same defect with
// one more place to drift, so this imports it.
//
// RELATIVE, NOT `$lib/`. The alias is exactly what node cannot resolve, and
// resolving it is the entire reason these modules are extracted. `prefix.js`
// itself imports nothing, so this stays drivable from the test runner.
import { MAX_PREFIX } from './prefix.js';

export { MAX_PREFIX };

/**
 * @template T
 * @typedef {object} Index
 * @property {Map<string, Array<T>>} bySymbol every row carrying that symbol,
 *           in the order it arrived.
 * @property {Map<string, Set<string>>} byGram every 1..MAX_PREFIX character
 *           infix, upper-cased, to the symbols carrying it.
 */

/**
 * Index `rows` by every short infix of the symbol each one carries.
 *
 * A ROW WHOSE SYMBOL IS NOT A NON-EMPTY STRING IS SKIPPED, not indexed under
 * `undefined` — the same rule `prefix.js` states and for the same reason. A
 * master that served one is a store problem, and burying it in a bucket nobody
 * probes hides it. It stays visible in `rows`, so the empty query still draws
 * it, and no query can match it.
 *
 * @template T
 * @param {Array<T>} rows
 * @param {(row: T) => unknown} key the symbol on a row. The census spells it
 *        `sym` and the master spells it `symbol`; neither name is assumed.
 * @returns {Index<T>}
 */
export function build(rows, key) {
  /** @type {Map<string, Array<T>>} */
  const bySymbol = new Map();
  /** @type {Map<string, Set<string>>} */
  const byGram = new Map();

  for (const row of rows ?? []) {
    const s = key(row);
    if (typeof s !== 'string' || s === '') continue;
    let bucket = bySymbol.get(s);
    if (!bucket) bySymbol.set(s, (bucket = []));
    bucket.push(row);
  }

  // ONE PASS PER DISTINCT SYMBOL, not per row. 13,500 rows over 750 names walk
  // this loop 750 times.
  for (const s of bySymbol.keys()) {
    // FOLDED ONCE, HERE. The query is upper-cased in `probe`, so the index has
    // to be upper-cased too or a lower-case symbol in the master could never be
    // matched by any query at all.
    const up = s.toUpperCase();
    for (let i = 0; i < up.length; i += 1) {
      for (let n = 1; n <= Math.min(MAX_PREFIX, up.length - i); n += 1) {
        const gram = up.slice(i, i + n);
        let carriers = byGram.get(gram);
        if (!carriers) byGram.set(gram, (carriers = new Set()));
        carriers.add(s);
      }
    }
  }

  return { bySymbol, byGram };
}

/**
 * The rows whose symbol contains `typed`, anywhere in it.
 *
 * AN EMPTY QUERY IS EVERY ROW, not no rows — the table opens on the whole
 * census and narrows from there. It is returned BY REFERENCE, so an unfiltered
 * table costs nothing at all.
 *
 * ORDER: a matched result is GROUPED BY SYMBOL, not left in `rows` order.
 * Grouping is what makes this one probe instead of a scan, and it costs
 * nothing here because every caller in this tree sorts the result before
 * drawing it. A caller that needs input order must impose it.
 *
 * @template T
 * @param {Index<T>} index
 * @param {Array<T>} rows the full set, for the empty query
 * @param {string | null | undefined} typed
 * @returns {Array<T>}
 */
export function probe(index, rows, typed) {
  const q = (typed ?? '').trim().toUpperCase();
  if (!q) return rows;

  // ONE `Map.get`, whatever the query's length. Any string containing `q` also
  // contains q's first MAX_PREFIX characters, so this bucket is a superset of
  // the answer and never misses a row.
  const carriers = index.byGram.get(q.slice(0, MAX_PREFIX));
  if (!carriers) return [];

  /** @type {Array<T>} */
  const out = [];
  for (const sym of carriers) {
    // The documented non-constant path, and it is bounded by the bucket.
    if (q.length > MAX_PREFIX && !sym.toUpperCase().includes(q)) continue;
    const bucket = index.bySymbol.get(sym);
    if (bucket) for (const row of bucket) out.push(row);
  }
  return out;
}

/**
 * How many DISTINCT symbols an index holds — what the operator is choosing
 * among, which is never the row count when a name is drawn once per rung.
 *
 * @param {Index<any>} index
 * @returns {number}
 */
export function symbolCount(index) {
  return index.bySymbol.size;
}
