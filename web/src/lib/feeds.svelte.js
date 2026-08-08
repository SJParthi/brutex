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
export const feeds = $state({ all: [], active: null, error: null });

export async function loadFeeds() {
  try {
    const r = await fetch('/feeds.json');
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
    // request, and it answers the only question worth asking on load: which of
    // these can show me something right now.
    const held = await Promise.all(
      feeds.all.map((f) =>
        fetch(`/store.json?feed=${encodeURIComponent(f.wire)}`)
          .then((r) => (r.ok ? r.json() : []))
          .then((d) => ({ wire: f.wire, ready: f.ready, bars: d.reduce((a, r) => a + r.rows, 0) }))
          .catch(() => ({ wire: f.wire, ready: f.ready, bars: 0 }))
      )
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
