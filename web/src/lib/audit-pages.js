// Merging `/audit.json` pages by ORDINAL, not by position.
//
// P1-06-03, D-2663. The server pages from the newest end of an append-only
// journal: page p covers ordinals [N - (p+1)*per, N - p*per) for the total N
// AT THE MOMENT IT ANSWERED. The page refreshed page 0 on every poll, kept the
// older pages it had fetched, and concatenated the two. While a pull appended
// g records, page 0 slid up by g and the g ordinals just below it were on no
// page held -- a silent hole -- and an older page fetched after growth page 0
// had not seen yet overlapped it, so one ordinal appeared twice and was used
// twice as a keyed-each key.
//
// The ordinal is the record's absolute index and never changes, so it is the
// only safe key: rows are de-duplicated by it, ordered by it, every missing
// range between the newest and the oldest held is reported as a gap, and the
// next page to read is the one that holds the newest missing ordinal.

/**
 * @typedef {{ ordinal: number }} Ordered
 * @typedef {{ from: number, to: number }} Gap  inclusive, from <= to
 */

/**
 * The page index that holds `ordinal` when the journal holds `total`
 * records, or null when no page does.
 *
 * @param {number} ordinal
 * @param {number} total
 * @param {number} perPage
 * @returns {number | null}
 */
export function pageFor(ordinal, total, perPage) {
  if (![ordinal, total, perPage].every(Number.isSafeInteger)) return null;
  if (perPage <= 0 || ordinal < 0 || ordinal >= total) return null;
  return Math.floor((total - 1 - ordinal) / perPage);
}

/**
 * De-duplicate by ordinal, newest first, and name every hole.
 *
 * @template {Ordered} T
 * @param {T[]} head the latest page 0
 * @param {T[]} older every older row already fetched
 * @returns {{ runs: T[], gaps: Gap[] }}
 */
export function mergePages(head, older) {
  /** @type {Map<number, T>} */
  const byOrdinal = new Map();
  // Page 0 first: when one ordinal was read twice, the newest read wins.
  for (const row of [...head, ...older]) {
    if (!Number.isSafeInteger(row?.ordinal)) continue;
    if (!byOrdinal.has(row.ordinal)) byOrdinal.set(row.ordinal, row);
  }
  const runs = [...byOrdinal.values()].sort((a, b) => b.ordinal - a.ordinal);
  /** @type {Gap[]} */
  const gaps = [];
  for (let i = 1; i < runs.length; i += 1) {
    const newer = runs[i - 1].ordinal;
    const next = runs[i].ordinal;
    if (newer - next > 1) gaps.push({ from: next + 1, to: newer - 1 });
  }
  return { runs, gaps };
}

/**
 * Which ordinal the next "load" must fetch: the newest missing one when a gap
 * exists, otherwise the one just below the oldest held. Null when nothing
 * older exists.
 *
 * @param {{ runs: Ordered[], gaps: Gap[] }} merged
 * @returns {number | null}
 */
export function nextOrdinal(merged) {
  if (merged.gaps.length > 0) return merged.gaps[0].to;
  const oldest = merged.runs.at(-1);
  if (oldest === undefined || oldest.ordinal <= 0) return null;
  return oldest.ordinal - 1;
}

/**
 * One older page's answer, as rows to merge or an error to show.
 *
 * OBSV-10, D-3209. `journal_block` answers 200 with `runs: []` and a
 * `runs_error` when its page read fails. Taken as an empty page, the failure
 * was never shown and still spent a page of `MAX_PAGES`. A named error wins
 * over any rows beside it; a body with no `runs` list is an error too, never
 * an empty page.
 *
 * @param {any} body
 * @returns {{ runs: any[], error: string | null }}
 */
export function olderPageOf(body) {
  const why = typeof body?.runs_error === 'string' ? body.runs_error.trim() : '';
  if (why !== '') return { runs: [], error: why };
  if (!Array.isArray(body?.runs)) return { runs: [], error: 'the older page carried no runs list' };
  return { runs: body.runs, error: null };
}

// F3, D-3219. `/audit.json` sends `store.generation` as null when no census is
// held (`crates/api/src/audit_json.rs`: absent or unreadable). The page held it
// as `?? 0`, so a census that became readable while the page was open read as
// every one of its commits "measured" between two answers. Zero is a
// generation; unknown is not zero, and no difference is taken across it.

/**
 * The manifest generation `/audit.json` reported, or null when it reported none.
 * @param {unknown} store
 * @returns {number | null}
 */
export function generationOf(store) {
  const g = store !== null && typeof store === 'object' ? /** @type {Record<string, unknown>} */ (store).generation : undefined;
  return typeof g === 'number' && Number.isSafeInteger(g) && g >= 0 ? g : null;
}

/**
 * Commits between two generations, or null when either is unknown.
 * @param {number | null} from
 * @param {number | null} to
 * @returns {number | null}
 */
export function generationStep(from, to) {
  return from === null || to === null ? null : to - from;
}
