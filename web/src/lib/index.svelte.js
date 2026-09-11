/**
 * The selected feed's instrument index. A held catalogue is reused until the
 * feed changes or the operator explicitly re-reads its master. Concurrent
 * readers share one request; stale responses never rebuild the current index.
 *
 * Building the index costs O(rows). Search normalizes the query, probes one
 * prefix bucket for 1..4 characters, and filters that bucket for longer text.
 * Its output and longest matching bucket are not constant in the number of
 * instruments. No request is made for each keystroke.
 */

// A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
// ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
// threaded through every call site.
import { ask } from '$lib/ask.js';
// THE ARITHMETIC LIVES IN `$lib/prefix.js` so a test can drive it. This module
// imports `$lib/ask.js`, which node cannot resolve, so the O(1) claim in the
// opening comment was unreachable from `node --test` even in principle.
import { createCatalogueLoader } from '$lib/catalogue-loader.js';

// `feed` IS NOT BOOKKEEPING — IT IS THE GATE. /ingest:1449 and /db:1013 both
// derive `catalogueIsThisFeed = catalogue.ready && catalogue.feed === feeds.active`
// and refuse to count a single name unless it holds, because counting one
// broker's master under another broker's name is exactly the §4 failure they
// exist to prevent. The field was never DECLARED and never WRITTEN, so that
// comparison was `undefined === 'dhan'` — false forever — and every instrument
// picker on both pages rendered "0 shown of 0" behind a refusal that could not
// be satisfied by any action the operator took.
/**
 * ONE ROW OF `/instruments.json`, TRANSCRIBED FROM THE `write!` THAT EMITS IT.
 *
 * `crates/api/src/server.rs` builds every row from a single unconditional
 * format string, so the field names and their JSON types are not a guess about
 * the wire — they are a reading of the only code that writes it:
 *
 *   {"symbol":str,"key":str,"kind":str,"exchange":str,"segment":str,
 *    "universe":str,"universes":[str],"bars":num,"href":str}
 *
 * EVERY FIELD IS OPTIONAL, AND THAT IS NOT HEDGING. This value comes off the
 * network, so the only build whose format string is evidence is the one
 * currently serving — and the pages already know it may not be. `uniTokens` on
 * `/` says so in its own words: "An API binary that predates D-0089 emits no
 * `universes` array." Declaring the fields required would let a reader index
 * one that is absent and be told by the checker that it cannot be, which is
 * the fallback-that-hides-a-failure §4 forbids, wearing a type annotation.
 *
 * The VALUES are typed and the PRESENCE is not, which is the split that makes
 * this useful: `row.symbol` stops being `Property 'symbol' does not exist on
 * type 'unknown'` and starts being `string | undefined` — a question the
 * reader must still answer, but the right question.
 *
 * `$lib/place.js` keeps its own `MasterRow` with `unknown` values on purpose
 * and is NOT unified with this one. It VALIDATES rather than reads — its whole
 * job is `typeof r.exchange !== 'string'` — and a parameter typed to the shape
 * it exists to check would make its own guard look redundant to the checker
 * while remaining necessary at runtime.
 *
 * @typedef {{
 *   symbol?: string,
 *   key?: string,
 *   kind?: string,
 *   exchange?: string,
 *   segment?: string,
 *   universe?: string,
 *   universes?: string[],
 *   bars?: number,
 *   href?: string
 * }} MasterRow
 */

/**
 * ANNOTATED, BECAUSE THE INITIAL VALUE IS NOT THE TYPE. `feed` and `error` start
 * `null` and are later a `string` — inferred from the initialiser alone they are
 * `null` and nothing else, so `catalogue.error = String(why)` in the failure arm
 * was an error against a type the successful path never exercises. The states
 * are three: not asked, asked and holding a name, asked and holding a reason.
 *
 * @type {{ rows: MasterRow[], ready: boolean, feed: string | null, error: string | null }}
 */
export const catalogue = $state({ rows: [], ready: false, feed: null, error: null });
const loader = createCatalogueLoader(catalogue, ask);

/**
 * AN ABSENT FEED IS REFUSED HERE, NOT GUARDED FOR AT EVERY CALL SITE.
 *
 * The line below sent `?feed=${feed ?? ''}`, and the running API answers an
 * EMPTY feed parameter with a 200 and the default vendor's whole master —
 * measured: `/instruments.json?feed=` returns 161,232 bytes of Dhan. So a null
 * feed reaching this function would not fail; it would quietly stamp one
 * broker's instrument list as the answer for "no broker", which is the same
 * class of fault as a cleared feed picker still showing Dhan.
 *
 * The layout forwards a cleared selection so the retained catalogue and its
 * pending read are both revoked. The Markets re-read button explicitly forces
 * a current-feed refresh. Neither path asks the API with an absent feed.
 *
 * @param {string | null | undefined} feed
 * @param {boolean} [force] Re-read a held feed after an explicit refresh.
 */
export const loadCatalogue = (feed, force = false) => loader.load(feed, force);

/**
 * One probe for 1..4 characters; a filter over one bucket beyond that.
 *
 * @param {string} typed
 */
export function search(typed) {
  return loader.search(typed);
}
