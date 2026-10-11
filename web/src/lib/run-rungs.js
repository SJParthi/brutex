/**
 * THE RUNGS A RUN'S CHART MAY SWITCH TO, FROM FACETS (DB-B58, WP-81).
 *
 * `/backtest` learned one underlying's rungs by downloading the WHOLE expanded
 * census of the run's feed — 108,787,752 bytes for GDFL and 65,671,854 for
 * Zerodha, uncached, per run opened — and folding every row to keep one
 * instrument's. `/census/facets.json?feed&segment&underlying` answers exactly
 * that instrument's months, each with its rows per rung: a few kilobytes.
 *
 * A run's underlying is a spot index (`INDEX`) or a share's cash series
 * (`CASH`); both are asked and the answers folded, since a name lives in one
 * of them and the other answers an empty list.
 */

/** The segments a swept underlying lives in. */
export const RUN_SEGMENTS = Object.freeze(['INDEX', 'CASH']);

/** The facets URL for one segment of a run's underlying.
 * @param {string} feed @param {string} segment @param {string} underlying @returns {string} */
export function facetsUrl(feed, segment, underlying) {
  const q = new URLSearchParams({ feed, segment, underlying });
  return `/census/facets.json?${q}`;
}

/** Months held per rung, folded over facets answers: a month counts for a
 * rung when it holds at least one row at that rung.
 * @param {any[]} answers parsed `/census/facets.json` bodies
 * @returns {Map<string, number>} */
export function rungMonths(answers) {
  /** @type {Map<string, number>} */
  const months = new Map();
  for (const answer of answers) {
    for (const month of Array.isArray(answer?.months) ? answer.months : []) {
      const rows = month && typeof month.rows === 'object' && month.rows !== null ? month.rows : {};
      for (const [rung, held] of Object.entries(rows)) {
        if (Number(held) > 0) months.set(rung, (months.get(rung) ?? 0) + 1);
      }
    }
  }
  return months;
}
