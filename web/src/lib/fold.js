/**
 * THE WINDOW FOLD `/ingest` MEASURES A RUN AGAINST — the arithmetic half, with
 * no runes in it, so `web/tests/fold.test.js` can drive it under `node --test`.
 *
 * # The 100% meter this exists to remove
 *
 * The fold counted EVERY row in the window months — every instrument, at every
 * rung — while `/ingest` divided it by a denominator built from its own request
 * (instruments × segments × rungs × months). With anything else already held in
 * those months (the 750-name NIFTY Total Market backfill is the ordinary case)
 * the numerator was in the hundreds against a denominator of 2,
 * `Math.min(1, …)` pinned the bar at 100.0%, `unitsLeft` clamped to 0 — which
 * silently removed the ETA line as well — and the page read "100.0% of 2
 * instrument-months are now held" with half the request still on the wire. Two
 * pre-existing rows were enough, not 750.
 *
 * A numerator and a denominator over two different populations is not a ratio.
 * `scope` is the request's own population, and what falls outside it is COUNTED
 * rather than dropped: `outside` is what the page names beside the meter, so a
 * row this request never asked for is neither credited to it nor hidden.
 */

/**
 * Fold the named months of a `byMonth` index, optionally scoped.
 *
 * @param byMonth `Map<"YYYY-MM", { list: cell[] }>` — the shared store index.
 * @param months the month keys to walk. Only these are visited: the index is
 *        the index, so a one-month window costs one month.
 * @param scope `{ instruments?: Set<string>, rungs?: Set<string> }`. Either half
 *        may be absent, which means "all of them".
 */
export function foldWindow(byMonth, months, scope) {
  const wants = scope?.instruments instanceof Set ? scope.instruments : null;
  const rungs = scope?.rungs instanceof Set ? scope.rungs : null;
  const byInstrument = new Map();
  let units = 0;
  let rows = 0;
  let outside = 0;
  for (const m of months ?? []) {
    const bucket = byMonth?.get(m);
    if (!bucket) continue;
    for (const cell of bucket.list) {
      if ((wants && !wants.has(cell.instrument)) || (rungs && !rungs.has(cell.timeframe))) {
        outside += 1;
        continue;
      }
      units += 1;
      rows += cell.rows;
      byInstrument.set(cell.instrument, (byInstrument.get(cell.instrument) ?? 0) + cell.rows);
    }
  }
  return { units, rows, byInstrument, outside, scoped: Boolean(wants || rungs) };
}

/**
 * The memo key for one fold. The SCOPE IS PART OF THE QUESTION: two scopes over
 * one window are two different answers and must never share a cached one.
 */
export function foldKey(reads, months, scope) {
  const wants = scope?.instruments instanceof Set ? [...scope.instruments].sort().join('|') : '*';
  const rungs = scope?.rungs instanceof Set ? [...scope.rungs].sort().join('|') : '*';
  return `${reads} ${[...(months ?? [])].join(',')} ${wants}#${rungs}`;
}
