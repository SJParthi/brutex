/**
 * THE SHAPE OF A SERIES, IN A CONSTANT NUMBER OF READS.
 *
 * # Why this is a module rather than a closure on the page
 *
 * It lived inside `routes/db/+page.svelte`, which `node --test` cannot import,
 * so the O(1) claim made for it in `docs/07-o1-architecture.md` was a sentence
 * and nothing else. That document is explicit about what a sentence is worth:
 * layer 12 was `✓` with the words "a fixed row cap with paging, never
 * O(universe)" while the fold behind the cap measured 3.569 ms at 2,787
 * instruments and 124.916 ms at 50,000 — "the row cap is what hid it". A bound
 * nothing re-measures is a bound nobody is keeping.
 *
 * `$lib/place.js` was carved out for the same reason and says so in its own
 * header. This is that carve, and `spark.test.js` is the driver that could not
 * exist before it.
 *
 * # The bound, stated exactly
 *
 * `sampleSeries` performs **at most `take` index reads of `rows`**, whatever
 * `rows.length` is. Not "about", not "amortised": the loop runs `take` times
 * and each iteration reads one element by computed index. There is no scan, no
 * `filter`, no `map` over the input and no sort of it.
 *
 * That is law 4 of `docs/07-o1-architecture.md` — "arithmetic beats lookup, if
 * the address can be computed never search for it" — applied to a chart.
 *
 * WHAT IT COSTS TO GET THAT. `lo` and `hi` are the extremes OF THE SAMPLES and
 * not of the series, because a true minimum needs every element and that is the
 * O(n) walk this exists to avoid. A spike between two samples is invisible
 * here. The caller must say so on the control; `/db` does, in those words.
 * `CLAUDE.md` §3 rule 6: never claim a measurement you did not take.
 */

/**
 * @typedef {{ x: number, v: number }} Sample
 *   `x` is 0..1 across the drawn width; `v` is the value at that sample.
 */

/**
 * @typedef {{
 *   pts: Sample[], lo: number, hi: number, flat: boolean,
 *   up: boolean, reads: number, of: number
 * }} Sampled
 *   `reads` is the number of index reads performed — returned so a test can
 *   assert the bound rather than trust this comment. `of` is the input length,
 *   so a caller can print "N of M" honestly.
 */

/**
 * Sample a series at evenly spaced indices.
 *
 * @param {ArrayLike<unknown>} rows
 * @param {(row: unknown) => number | null} value
 *   Pulls the number out of one row, or `null` where the row carries none. A
 *   row that yields `null` is SKIPPED rather than treated as zero — a missing
 *   close is not a close of nothing, and plotting it at the axis would draw a
 *   crash that never happened.
 * @param {number} take how many samples to draw. Clamped to `rows.length`.
 * @param {boolean} [newestFirst] `true` when `rows` runs newest → oldest, as a
 *   grid sorted descending does. The samples are reversed so the drawn line
 *   always runs earliest → latest: a chart drawn backwards reads as a fall
 *   where the series rose, which is worse than no chart.
 * @returns {Sampled | null} `null` when fewer than two samples carry a value —
 *   a line needs two points, and one point drawn as a line is an invention.
 */
export function sampleSeries(rows, value, take, newestFirst = false) {
  const n = rows?.length ?? 0;
  if (n < 2 || take < 2) return null;
  const want = Math.min(n, Math.floor(take));

  /** @type {Sample[]} */
  const pts = [];
  let lo = Infinity;
  let hi = -Infinity;
  let reads = 0;

  for (let i = 0; i < want; i++) {
    /* THE ADDRESS IS COMPUTED. This is the whole bound: `i` walks 0..want and
       the index is arithmetic on it, so `n` never enters the iteration count. */
    const at = ((i * n) / want) | 0;
    reads += 1;
    const v = value(rows[at]);
    if (v === null || !Number.isFinite(v)) continue;
    if (v < lo) lo = v;
    if (v > hi) hi = v;
    pts.push({ x: 0, v });
  }
  if (pts.length < 2) return null;

  if (newestFirst) pts.reverse();
  /* `x` IS ASSIGNED AFTER THE ORDER IS FINAL, not during the read. Assigning it
     from `i` and then reversing leaves every point carrying the x of its
     mirror, which draws the series backwards while every value is right — the
     hardest kind of wrong to see. */
  const last = pts.length - 1;
  for (let i = 0; i <= last; i++) pts[i].x = i / last;

  return {
    pts,
    lo,
    hi,
    /* A SERIES WITH NO SPREAD IS FLAT, AND SAYING SO IS NOT THE SAME AS SAYING
       IT IS EMPTY. The caller draws it down the middle rather than dividing by
       a zero span. The newest minutes of this store really are flat, so this
       is a state the page reaches on load and not an edge case. */
    flat: hi === lo,
    up: pts[last].v >= pts[0].v,
    reads,
    of: n
  };
}

/**
 * A sampled series as an SVG path, in a `0 0 width height` viewBox.
 *
 * SEPARATE FROM THE SAMPLING because the geometry is what a renderer wants and
 * the samples are what a test wants. Keeping them in one function meant the
 * bound above could only be checked by parsing a path string.
 *
 * @param {Sampled} s
 * @param {number} [width]
 * @param {number} [height]
 * @param {number} [pad] top and bottom breathing room, in viewBox units, so a
 *   value at the extreme is not clipped by half its stroke.
 * @returns {{ line: string, area: string }}
 */
export function sparkPath(s, width = 1000, height = 100, pad = 4) {
  const span = s.hi - s.lo;
  const usable = height - pad * 2;
  const y = (/** @type {number} */ v) =>
    s.flat ? height / 2 : height - pad - ((v - s.lo) / span) * usable;
  let line = '';
  for (let i = 0; i < s.pts.length; i++) {
    const p = s.pts[i];
    line += `${i ? 'L' : 'M'}${(p.x * width).toFixed(1)} ${y(p.v).toFixed(1)}`;
  }
  return { line, area: `${line}L${width} ${height}L0 ${height}Z` };
}
