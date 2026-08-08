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
    feeds.active = feeds.all[0]?.wire ?? null;
  } catch (why) {
    // LOUD, NOT SILENT. A feed list that quietly fails leaves a picker with no
    // options and no reason, which is the shape CLAUDE.md section 4 bans.
    feeds.error = String(why);
  }
}
