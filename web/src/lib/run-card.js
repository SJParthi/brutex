// The /ingest run card's two derived readings: bars landed, and each feed's bar.
//
// CE-81, D-1787.
//
// LANDED. The card printed `rowsNow - rowsAtStart` with no guard. `rows_now`
// drops a feed whose census is unreadable and a degraded reload counts an
// older generation, so the difference can go NEGATIVE ("-1,200 bar(s)
// landed"), while the api's own summary saturates the same subtraction. A
// counter that is `null` (the total is unknown) is unknown here too: no delta
// is computed from it, and the card says "an unknown number of".
//
// THE FEED BAR. `class:up={f.finished || legsDone >= legs}` drew a halted
// feed -- `finished` with `skipped = legs - legsDone` after a permanent or
// credential refusal -- in the success colour at 1/5, and `skipped` was
// rendered nowhere. Success is every leg done and none skipped.

/**
 * @param {unknown} value
 * @returns {value is number}
 */
function count(value) {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

/**
 * @param {unknown} rowsAtStart `/pull/run.json`'s `rowsAtStart` (nullable)
 * @param {unknown} rowsNow `/pull/run.json`'s `rowsNow` (nullable)
 * @returns {{ kind: 'unknown', count: null, lead: string, why: string }
 *   | { kind: 'landed', count: number, lead: '', why: string }
 *   | { kind: 'dropped', count: number, lead: '', why: string }}
 */
export function landedOf(rowsAtStart, rowsNow) {
  if (!count(rowsAtStart) || !count(rowsNow)) {
    return {
      kind: 'unknown',
      count: null,
      lead: 'an unknown number of',
      why: 'The store total at the start or now is unknown, so no landed count is computed from it.'
    };
  }
  if (rowsNow >= rowsAtStart) {
    return { kind: 'landed', count: rowsNow - rowsAtStart, lead: '', why: 'Rows in the store now, less rows at the start of this run.' };
  }
  return {
    kind: 'dropped',
    count: rowsAtStart - rowsNow,
    lead: '',
    why:
      'The store total FELL during this run: a feed whose census became unreadable is not counted, ' +
      'or a damaged manifest reloaded an older generation. No landed count is claimed from a total that went down.'
  };
}

/**
 * @param {{ legs?: number, legsDone?: number, finished?: boolean, skipped?: number, credentialDead?: boolean }} feed
 * @returns {{ up: boolean, halted: boolean, skipped: number }}
 */
export function feedMeter(feed) {
  const skipped = count(feed.skipped) ? feed.skipped : 0;
  const legs = count(feed.legs) ? feed.legs : 0;
  const done = count(feed.legsDone) ? feed.legsDone : 0;
  const halted = skipped > 0 || feed.credentialDead === true;
  return { up: legs > 0 && done >= legs && !halted, halted, skipped };
}
