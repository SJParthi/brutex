/**
 * The instrument index, and the reason the whole universe ships to the browser.
 *
 * O(1) PER KEYSTROKE. The surface is bounded at ~800 instruments by decision,
 * so the entire searchable set is one response at load. After that every
 * keystroke is ONE Map probe on a 1..4-character prefix bucket — not a scan of
 * 800, and not a request. A request per character is ~800 requests to type one
 * symbol and makes keystroke latency a function of the network.
 *
 * Beyond four characters the bucket is already small and filtering it costs
 * less than the memory a deeper index would. That is the only place cost is not
 * constant, and it is bounded by the largest 4-prefix bucket rather than by the
 * universe.
 *
 * Building the index is O(n) ONCE, at load. That is inherent — every row has to
 * be seen to be indexed — and it is paid a single time for the session.
 */

// A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
// ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
// threaded through every call site.
import { ask } from '$lib/ask.js';
// THE ARITHMETIC LIVES IN `$lib/prefix.js` so a test can drive it. This module
// imports `$lib/ask.js`, which node cannot resolve, so the O(1) claim in the
// opening comment was unreachable from `node --test` even in principle.
import { build, probe } from '$lib/prefix.js';

// `feed` IS NOT BOOKKEEPING — IT IS THE GATE. /ingest:1449 and /db:1013 both
// derive `catalogueIsThisFeed = catalogue.ready && catalogue.feed === feeds.active`
// and refuse to count a single name unless it holds, because counting one
// broker's master under another broker's name is exactly the §4 failure they
// exist to prevent. The field was never DECLARED and never WRITTEN, so that
// comparison was `undefined === 'dhan'` — false forever — and every instrument
// picker on both pages rendered "0 shown of 0" behind a refusal that could not
// be satisfied by any action the operator took.
export const catalogue = $state({ rows: [], ready: false, feed: null, error: null });
let byPrefix = new Map();
// The feed whose answer is IN FLIGHT. Switching feed twice quickly can land the
// responses out of order; without this the last to RETURN wins instead of the
// last one ASKED, and the page answers confidently for the wrong broker.
let inFlight = null;

export async function loadCatalogue(feed) {
  inFlight = feed;
  try {
    // THE FEED IS PART OF THE QUESTION. The two brokers do not list the same
    // instruments, so "every instrument" is a different set per feed and the
    // index has to be rebuilt when the selection changes.
    const r = await ask(`/instruments.json?feed=${encodeURIComponent(feed ?? '')}`);
    if (!r.ok) {
      // THE SERVER ALREADY SAID WHY, AND THIS THREW IT AWAY.
      //
      // D-0124 added `x-brutex-master-state` and `x-brutex-master-note` to this
      // exact response so a page could tell a vendor whose master was never
      // READ from one whose master lists nothing — two states that produce the
      // same empty screen and need opposite actions. This line reported
      // `HTTP 503` and dropped both.
      //
      // Measured: selecting a broker whose instrument file is not on disk drew
      // "/instruments.json could not be read, so no name is counted here:
      // Error: HTTP 503" — a status code, about a file, with no mention of the
      // file. The note says which one and what to do about it.
      const note = r.headers.get('x-brutex-master-note');
      const state = r.headers.get('x-brutex-master-state');
      if (note) throw new Error(state ? `${state} — ${note}` : note);
      throw new Error(`HTTP ${r.status}`);
    }
    const rows = await r.json();
    const next = build(rows);
    // A response for a feed the operator has already left is DISCARDED, never
    // stamped.
    if (inFlight !== feed) return;
    byPrefix = next;
    catalogue.rows = rows;
    catalogue.feed = feed;
    catalogue.ready = true;
    catalogue.error = null;
  } catch (why) {
    if (inFlight !== feed) return;
    // The stamp is CLEARED on failure. Leaving the previous feed's name on a
    // failed read is the stale-value shape §4 bans — the page would go on
    // counting the old master and say nothing was wrong.
    catalogue.feed = null;
    catalogue.ready = false;
    catalogue.error = String(why);
  }
}

/** One probe for 1..4 characters; a filter over one bucket beyond that. */
export function search(typed) {
  return probe(byPrefix, catalogue.rows, typed);
}
