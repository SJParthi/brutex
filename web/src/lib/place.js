/**
 * WHERE A SYMBOL'S BAR FILES ARE FILED — exchange and segment, from a master.
 *
 * # Why this is a module rather than a closure on the page
 *
 * The rule is three lines and it lived inside `routes/backtest/+page.svelte`,
 * which `node --test` cannot import. `$lib/pick.js` was carved out of
 * `feeds.svelte.js` for exactly that reason and says so in its own header; this
 * is the same carve for the same reason, and `place.test.js` is the driver that
 * could not exist before it.
 *
 * # The rule, and the defect it encodes
 *
 * The ledger stores `underlying` alone — `NIFTY` — and a bar file's path needs
 * `NSE/INDEX/NIFTY`. Those two segments are RESOLVED from the feed's own
 * instrument master, never written as literals, because a literal is a
 * hardcoded instrument the store can contradict the moment a second exchange
 * is swept.
 *
 * THE MASTER MUST BE THE RUN'S OWN. `/backtest` loads one catalogue, for
 * `feeds.active`, and builds its bar request with `run.feed`. With "Show every
 * feed" on, an operator can open a run belonging to another vendor — and the
 * resolver then answered out of the ACTIVE vendor's master while the request
 * went to the run's. Whether two vendors agree on a symbol's exchange and
 * segment is a property of two masters nobody compared; the wiring is wrong
 * whatever the answer, and what it produces — a 4xx from the bar route, or
 * worse a plausible file that is the wrong series — is not a failure a reader
 * traces back to a three-line lookup.
 *
 * So a mismatch REFUSES. `null` is the value this already returns for "cannot
 * be resolved" and the caller already has a refusal arm for it; the arm names
 * the mismatch rather than blaming the symbol for being absent from a master it
 * was never meant to be looked up in.
 *
 * @typedef {{ symbol?: unknown, exchange?: unknown, segment?: unknown }} MasterRow
 */

/**
 * @param {readonly unknown[] | null | undefined} rows the loaded master's rows,
 *        as `/instruments.json` sends them. Nullable on purpose and guarded
 *        below: the catalogue is loaded asynchronously, so every caller has a
 *        window in which it holds nothing, and a resolver that throws during
 *        that window would turn a normal load into an error.
 * @param {string | null | undefined} catalogueFeed the feed those rows were loaded for
 * @param {string} symbol the ledger's `underlying`
 * @param {string | null | undefined} [forFeed] the feed the answer will be USED for.
 *        Omitted means "the caller has no second feed in play"; supplied and
 *        different from `catalogueFeed` is the refusal above.
 * @returns {{ exchange: string, segment: string } | null}
 */
export function placeIn(rows, catalogueFeed, symbol, forFeed) {
  if (forFeed && catalogueFeed !== forFeed) return null;
  for (const row of rows ?? []) {
    const r = /** @type {MasterRow} */ (row);
    if (r?.symbol !== symbol) continue;
    /* BOTH FIELDS OR NEITHER. A row carrying one of the two would otherwise
       yield `{exchange: 'NSE', segment: undefined}`, which builds the string
       `.../NSE/undefined/NIFTY` and asks the bar route for it — a request made
       of a value nobody supplied. A half-known place is not a place. */
    if (typeof r.exchange !== 'string' || typeof r.segment !== 'string') return null;
    return { exchange: r.exchange, segment: r.segment };
  }
  return null;
}
