/**
 * THE INSTRUMENT INDEX'S ARITHMETIC — the half with no runes in it, so
 * `web/tests/prefix.test.js` can drive it under `node --test`.
 *
 * `index.svelte.js` states an O(1) claim in its own opening line: "every
 * keystroke is ONE Map probe on a 1..4-character prefix bucket — not a scan of
 * 800, and not a request." That is the browser's version of CLAUDE.md §3
 * rule 4, and nothing measured it. It could not be measured, either: the module
 * holding it imports `$lib/ask.js`, which node cannot resolve, so the logic was
 * unreachable from a test runner even in principle.
 *
 * Extracted here for the same reason `completeness.js`, `fold.js`, `receipt.js`
 * and `bps.js` were: a claim that cannot be driven is a claim nobody is
 * checking.
 *
 * # Where the cost actually is
 *
 * `build` is O(rows × MAX_PREFIX) and runs ONCE per feed, at load. Every
 * keystroke after that is `probe`, which is one `Map.get` for 1..4 characters.
 * Beyond four the bucket is already small and `probe` filters it — that is the
 * one place cost is not constant, and it is bounded by the largest 4-character
 * bucket rather than by the catalogue.
 */

/** Prefix lengths the index holds. Beyond this, one filter over one bucket. */
export const MAX_PREFIX = 4;

/**
 * Every 1..4-character prefix of every symbol, to the rows carrying it.
 *
 * A row whose `symbol` is not a string is skipped rather than indexed under
 * `undefined`: a master that served one is a store problem, and burying it in a
 * bucket nobody probes hides it.
 *
 * @param {Array<{symbol?: unknown}>} rows
 * @returns {Map<string, Array<any>>}
 */
export function build(rows) {
  /** @type {Map<string, Array<any>>} */
  const byPrefix = new Map();
  for (const row of rows) {
    const s = row.symbol;
    if (typeof s !== 'string') continue;
    for (let n = 1; n <= Math.min(MAX_PREFIX, s.length); n += 1) {
      const key = s.slice(0, n);
      let bucket = byPrefix.get(key);
      if (!bucket) byPrefix.set(key, (bucket = []));
      bucket.push(row);
    }
  }
  return byPrefix;
}

/**
 * One probe for 1..4 characters; a filter over one bucket beyond that.
 *
 * An empty query is every row, not no rows — the picker opens on the whole
 * universe and narrows from there.
 *
 * @param {Map<string, Array<any>>} byPrefix
 * @param {Array<any>} rows the full set, for the empty query
 * @param {string | null | undefined} typed
 * @returns {Array<any>}
 */
export function probe(byPrefix, rows, typed) {
  const q = (typed ?? '').trim().toUpperCase();
  if (!q) return rows;
  if (q.length <= MAX_PREFIX) return byPrefix.get(q) ?? [];
  return (byPrefix.get(q.slice(0, MAX_PREFIX)) ?? []).filter((r) => r.symbol.startsWith(q));
}
