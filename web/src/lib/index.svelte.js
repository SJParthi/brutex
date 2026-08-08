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
const MAX_PREFIX = 4;

export const catalogue = $state({ rows: [], ready: false, error: null });
let byPrefix = new Map();

export async function loadCatalogue() {
  try {
    const r = await fetch('/instruments.json');
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    const rows = await r.json();
    const next = new Map();
    for (const row of rows) {
      const s = row.symbol;
      if (typeof s !== 'string') continue;
      for (let n = 1; n <= Math.min(MAX_PREFIX, s.length); n += 1) {
        const key = s.slice(0, n);
        let bucket = next.get(key);
        if (!bucket) next.set(key, (bucket = []));
        bucket.push(row);
      }
    }
    byPrefix = next;
    catalogue.rows = rows;
    catalogue.ready = true;
  } catch (why) {
    catalogue.error = String(why);
  }
}

/** One probe for 1..4 characters; a filter over one bucket beyond that. */
export function search(typed) {
  const q = (typed ?? '').trim().toUpperCase();
  if (!q) return catalogue.rows;
  if (q.length <= MAX_PREFIX) return byPrefix.get(q) ?? [];
  return (byPrefix.get(q.slice(0, MAX_PREFIX)) ?? []).filter((r) => r.symbol.startsWith(q));
}
