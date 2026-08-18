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
 * @typedef {{
 *   wire: string,
 *   display: string,
 *   kind: string,
 *   kind_label: string,
 *   verb: string,
 *   ready: boolean,
 *   why: string,
 *   finest?: { rung: string, kind: string, label: string,
 *              tick_stream: boolean, conflated: boolean }
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
    const best =
      held.filter((f) => f.bars > 0).sort((a, b) => b.bars - a.bars)[0] ??
      held.find((f) => f.ready) ??
      held[0];
    feeds.active = best?.wire ?? null;
  } catch (why) {
    // LOUD, NOT SILENT. A feed list that quietly fails leaves a picker with no
    // options and no reason, which is the shape CLAUDE.md section 4 bans.
    feeds.error = String(why);
  }
}
