/**
 * The feed selection, and it is a SELECTION — never a comparison.
 *
 * Exactly one feed is active. No page in this product puts one feed's numbers
 * beside another's: the two are not the same instrument universe, the same
 * session handling or the same price scale, so a difference between them reads
 * as signal and is noise.
 *
 * The list is fetched from the Rust side, which builds it from its descriptor
 * table. A fifth feed appears here the day its row exists — nothing in this
 * file names a vendor.
 */
import { survey, surveyStores } from '$lib/store.svelte.js';
// A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
// ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
// threaded through every call site.
import { ask } from '$lib/ask.js';
import { pickDefaultFeed } from '$lib/pick.js';

/**
 * ONE FEED, AS `/feeds.json` SENDS IT.
 *
 * Typed because `$state({ all: [] })` infers `never[]`, and every read of a
 * field off a member of that array is then an error the checker reports at the
 * READER rather than here — `Property 'wire' does not exist on type 'never'`,
 * once per use, across /db, /ingest, /autopilot and the layout. One annotation
 * at the source clears the lot.
 *
 * The shape is the server's, read off a live response rather than guessed:
 * `crates/api` builds it from the descriptor table in `pull::vendor`.
 *
 * RE-DERIVED FROM A LIVE BODY, AND IT HAD DRIFTED IN THREE PLACES AT ONCE.
 * This annotation is the thing that stops a field being invented, so when it
 * falls behind the server the effect is the opposite of what it exists for:
 * every reader of the missing field becomes an error located at the reader.
 * Measured against `/feeds.json` on 21 Aug 2026, all five descriptor rows:
 *
 *   * `fno` was absent. It is sent by every row, and `/ingest` reads it twice
 *     to decide which derivative segments a feed can even be asked for —
 *     `svelte-check` reported both as "Property 'fno' does not exist".
 *   * `history` was absent, and `/ingest` had already worked around it with a
 *     local intersection, `Feed & { history?: HistoryRow[] }`, plus its own
 *     copy of the row shape. A second copy of the server's contract is what
 *     this typedef's own first paragraph exists to prevent, so the row type
 *     moves here and that page imports it.
 *   * `finest` was missing `because` and `source` — the two fields that carry
 *     WHY a feed's floor is what it is, which is the half a reader cannot
 *     reconstruct.
 *
 * @typedef {{
 *   rung: string,
 *   served: boolean,
 *   kind: string,
 *   unit: string | null,
 *   n: number | null,
 *   from: string | null,
 *   oldest: string | null,
 *   source: string,
 *   standing: string,
 *   binds_because: string,
 *   contested: {
 *     kind: string, unit: string | null, n: number | null,
 *     from: string | null, oldest: string | null,
 *     source: string, standing: string
 *   } | null
 * }} HistoryRow
 */

/**
 * HOW A FEED ADDRESSES DERIVATIVES, IN THE SERVER'S OWN WORDS.
 *
 * A union rather than `string`, because the four values are not free text: each
 * one selects a different request shape, and `/ingest` branches on them by
 * name. Measured across all five rows — `by_strike_offset` (Dhan),
 * `by_name` (Groww), `local_folder` (TrueData, GDFL), `none` (Zerodha).
 *
 * `by_strike_offset` is the ATM-relative one: 21 index offsets — ATM and ten
 * either side — and 7 for stocks, ATM and three either side. See
 * `crates/pull/src/rolling.rs`.
 *
 * @typedef {'by_name' | 'by_strike_offset' | 'local_folder' | 'none'} FnoMode
 */

/**
 * @typedef {{
 *   wire: string,
 *   display: string,
 *   kind: string,
 *   kind_label: string,
 *   verb: string,
 *   ready: boolean,
 *   why: string,
 *   fno: FnoMode,
 *   history?: HistoryRow[],
 *   finest?: { rung: string, kind: string, label: string,
 *              tick_stream: boolean, conflated: boolean,
 *              because: string, source: string }
 * }} Feed
 */

/** @type {{ all: Feed[], active: string | null, error: string | null }} */
export const feeds = $state({ all: [], active: null, error: null });

/* ONE FLIGHT, AND ONE ANSWER PER PAGE LIFE.
 *
 * `/feeds.json` carries no parameter — every call asks the identical question —
 * and it was answered FIVE times on a single load, measured with
 * `performance.getEntriesByType`. Two call sites in the layout and a navigation
 * apiece is all it takes, because nothing here checked whether the answer was
 * already in hand or already on its way.
 *
 * `flight` de-duplicates the concurrent case; the `feeds.all.length` check
 * de-duplicates the sequential one. `force` is the escape, and `retryFeeds` in
 * the layout is why it exists — a retry after an error must actually re-ask.
 *
 * The same shape `surveyStores` already uses in `store.svelte.js`; this file
 * simply never got it. */
/** @type {Promise<void> | null} */
let flight = null;

/** @param {boolean} [force] */
export async function loadFeeds(force = false) {
  if (!force) {
    if (flight) return flight;
    if (feeds.all.length > 0 && !feeds.error) return;
  }
  flight = loadFeedsNow();
  try {
    await flight;
  } finally {
    flight = null;
  }
}

async function loadFeedsNow() {
  try {
    const r = await ask('/feeds.json');
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    feeds.all = await r.json();
    // THE FEED THAT ACTUALLY HOLDS DATA, preferred over the one that merely
    // could.
    //
    // "first ready feed" opened every page on Dhan — ready because a broker's
    // credential proves entitlement, and holding nothing because it had never
    // pulled. Groww sat one dropdown away with 8,922 bars and the operator saw
    // an empty DB, an empty chart and "With data 0", which reads as a broken
    // build rather than as an unselected feed.
    //
    // So the store decides the default and the credential is the fallback. One
    // pass, and it answers the only question worth asking on load: which of
    // these can show me something right now.
    //
    // THE PASS IS `$lib/store.svelte.js`'s AND NOT THIS FILE'S. It used to be
    // one of SIX independent reads of `/store.json` across the product, and one
    // of TWO that asked this same across-every-feed question — `/db` asked it
    // again, on its own clock, to name where the rows are when the selected
    // feed is empty. Two folds of one answer is how two surfaces come to
    // disagree about the same disk. `surveyStores` folds it once and stamps it
    // with the feed list it answered for; both readers read that.
    await surveyStores(feeds.all);
    const held = feeds.all.map(
      (f) => survey.byFeed.get(f.wire) ?? { wire: f.wire, ready: f.ready, bars: 0 }
    );
    // THE RULE ITSELF IS IN `$lib/pick.js` so a test can drive it. This module
    // imports `$lib/ask.js`, which node cannot resolve, so the choice that
    // decides which feed every page opens on was unreachable from `node --test`.
    feeds.active = pickDefaultFeed(held);
  } catch (why) {
    // LOUD, NOT SILENT. A feed list that quietly fails leaves a picker with no
    // options and no reason, which is the shape CLAUDE.md section 4 bans.
    feeds.error = String(why);
  }
}
