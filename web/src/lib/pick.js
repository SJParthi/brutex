/**
 * WHICH FEED A PAGE OPENS ON — the rule, with no runes in it, so
 * `web/tests/pick.test.js` can drive it under `node --test`.
 *
 * Extracted from `feeds.svelte.js`, which imports `$lib/ask.js` and cannot be
 * reached from a test runner. The rule it carries is not a preference; it is a
 * bug fix, and `feeds.svelte.js` records the bug in its own words:
 *
 *   "first ready feed" opened every page on Dhan — ready because a broker's
 *   credential proves entitlement, and holding nothing because it had never
 *   pulled. Groww sat one dropdown away with 8,922 bars and the operator saw an
 *   empty DB, an empty chart and "With data 0", which reads as a broken build
 *   rather than as an unselected feed.
 *
 * So: THE STORE DECIDES THE DEFAULT AND THE CREDENTIAL IS THE FALLBACK. It
 * answers the only question worth asking on load — which of these can show me
 * something right now.
 */

/**
 * The feed to open on, or `null` when there are none.
 *
 * @param {Array<{wire: string, ready?: boolean, bars?: number}>} held
 *        one entry per feed, carrying what the store actually holds for it.
 * @returns {string | null} the `wire` name, never a display name.
 */
export function pickDefaultFeed(held) {
  if (!Array.isArray(held) || held.length === 0) return null;
  const best =
    held.filter((f) => (f.bars ?? 0) > 0).sort((a, b) => (b.bars ?? 0) - (a.bars ?? 0))[0] ??
    held.find((f) => f.ready) ??
    held[0];
  return best?.wire ?? null;
}
