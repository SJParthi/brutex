/**
 * /db — GO TO AN INSTANT, AS ARITHMETIC (DB-005, R3-03)
 *
 * What the Go to box and the minute picker typed, as an instant or a refusal
 * in words; the one request that finds it; and where it lands. Moved out of
 * `bars-plan.js` and `DbBarsGrid.svelte` so the grid's later packages do not
 * queue on it.
 *
 * A PLAIN MODULE: pure functions only, so `node --test` imports it with no
 * compiler, and its imports are relative.
 *
 * Owner: R3-03.
 */
import { istInstant, pageOfIndex } from './bars-plan.js';
import { dayText, quote } from './input-check.js';
import { fmt } from './notes-labels.js';

/**
 * The most characters the box is read for. `2019-03-20 12:33:38` is
 * nineteen; anything past this is refused by its length and never echoed.
 */
export const GOTO_MAX = 32;

/** The example a refusal offers, on the rung's own precision. @param {boolean} seconds */
const example = (seconds) => (seconds ? '2019-03-15 10:02:07' : '2019-03-15 10:02');

/**
 * WHAT THE GO-TO BOX TYPED, as an instant (DB-005, R3-03).
 *
 * `YYYY-MM-DD HH:MM[:SS]` names the day; a time alone, `H:MM[:SS]`, takes
 * `day`, the day the reader is looking at. Seconds are accepted on every
 * rung. A time typed WITHOUT seconds names a MINUTE, and `minute` says so:
 * Go to then lands on that minute's first bar.
 *
 * Refused in words: an empty box, more than `GOTO_MAX` characters (named by
 * length, never echoed), a day that does not exist, a time past 23:59:59, and
 * an instant outside `span` — the days the selection holds, or the date
 * window that narrows it — so 1969 and 2099 are said to be outside rather
 * than turned into the first or last page without a word.
 *
 * @param {unknown} text what was typed
 * @param {string} day the day on screen, `YYYY-MM-DD`, or `''`
 * @param {{lo?: string, hi?: string, what?: string}} [span] the days an instant may name
 * @returns {{at: number, label: string, minute: boolean} | {why: string}}
 */
export function parseGoto(text, day, span = {}) {
  const raw = String(text ?? '');
  if (raw.length > GOTO_MAX) {
    return { why: `Go to read ${fmt(raw.length)} characters, and an instant is at most ${GOTO_MAX}: type a day and a time, ${example(true)}, or a time alone.` };
  }
  const s = raw.trim().replace('T', ' ');
  if (s === '') return { why: `Type an instant: a day and a time, ${example(false)}, or a time alone, 10:02, for the day on screen.` };
  const both = s.match(/^(\d{4}-\d{2}-\d{2})\s+(\d{1,2}:\d{2}(?::\d{2})?)$/);
  const only = s.match(/^(\d{1,2}:\d{2}(?::\d{2})?)$/);
  const d = both ? both[1] : day;
  const typedTime = both ? both[2] : only ? only[1] : '';
  if (!typedTime) {
    return { why: `${quote(s)} is not an instant. Type a day and a time, ${example(false)}, or a time alone, 10:02, for the day on screen.` };
  }
  const t = typedTime.length === 4 || typedTime.length === 7 ? `0${typedTime}` : typedTime;
  if (!d) return { why: `${quote(s)} names no day and no bar is on screen to take one from. Type the day too: 2019-03-15 ${t}.` };
  const at = istInstant(d, t);
  if (at === null) return { why: `${quote(s)} is not a real IST instant: the day must exist and the time be 00:00:00 to 23:59:59.` };
  const what = span.what ?? 'this selection';
  if (span.lo && d < span.lo) {
    return { why: `${dayText(d)} is before ${what}'s first day, ${dayText(span.lo)}: no bar of it lies that early. Pick a day from ${dayText(span.lo)} to ${dayText(span.hi ?? span.lo)}.` };
  }
  if (span.hi && d > span.hi) {
    return { why: `${dayText(d)} is after ${what}'s last day, ${dayText(span.hi)}: no bar of it lies that late. Pick a day from ${dayText(span.lo ?? span.hi)} to ${dayText(span.hi)}.` };
  }
  const minute = t.length === 5;
  return { at, label: `${d} ${minute ? `${t}:00` : t}`, minute };
}

/**
 * THE ONE QUESTION A GO TO ASKS: the instant, in ASCENDING time order,
 * whatever the grid's direction.
 *
 * The server's `at=` answers the first bar at or after an instant ascending,
 * and the first at or BEFORE it descending. Newest first, that second rule
 * landed on the bar before the minute asked for whenever no bar opened on its
 * first second — 10:01:59 for 10:02 on a one-second contract (the 11 Oct
 * re-check). Asked ascending, the answer is the minute's first bar in both
 * directions, and its place newest first is `total - 1 -` its place oldest
 * first. One request, one rank per series on the server.
 *
 * @param {number} at unix seconds
 * @returns {{sort: string, desc: boolean, at: number, limit: number}}
 */
export const gotoAsk = (at) => ({ sort: 'ts', desc: false, at, limit: 1 });

/**
 * @typedef {{page: number, index: number, bar: any, why: string}} Landing
 * `index` the 0-based row in the grid's own order; `why` what is said under
 * the strip (`''` when the landing is exactly what was asked).
 */

/**
 * WHERE A GO TO LANDS, from the answer to `gotoAsk`.
 *
 * The first bar at or after the instant, in either direction: its row in the
 * grid's order, its page at `size` rows, and a sentence whenever the bar is
 * not the instant itself — the minute's first bar, the next bar after a
 * silent minute, or the newest bar when the query ends before the instant.
 *
 * @param {any} answer the `/bars/window.json` answer
 * @param {{at: number, label: string, minute: boolean, desc: boolean, size: number, when: (t: number) => string}} o
 *   `when` prints a bar's instant
 * @returns {Landing | {why: string}}
 */
export function landing(answer, o) {
  const total = Number(answer?.total);
  if (!Number.isSafeInteger(total) || total <= 0) return { why: 'No bar is kept by this query, so there is nothing to land on.' };
  const ascIndex = Number(answer.at_index);
  const bar = Array.isArray(answer.bars) ? answer.bars[0] : undefined;
  if (!Number.isSafeInteger(ascIndex) || ascIndex < 0 || !bar) {
    return { why: 'The server did not say where that instant lies, so Go to did not move the grid.' };
  }
  if (ascIndex >= total) {
    /* PAST THE END: the answer's row is the newest bar of the query. */
    const index = o.desc ? 0 : total - 1;
    return {
      page: pageOfIndex(index, o.size),
      index,
      bar,
      why: `No bar lies at or after ${o.label} in this query: its newest bar is ${o.when(bar.t)} IST. Landed on it.`
    };
  }
  const index = o.desc ? total - 1 - ascIndex : ascIndex;
  const page = pageOfIndex(index, o.size);
  const row = `row ${fmt(index + 1)} of ${fmt(total)}`;
  if (bar.t === o.at) return { page, index, bar, why: '' };
  const gap = bar.t - o.at;
  if (o.minute && gap < 60) {
    return { page, index, bar, why: `The first bar of ${o.label.slice(0, 16)} opens at ${o.when(bar.t)} IST, ${fmt(gap)} second(s) into the minute — ${row}.` };
  }
  return {
    page,
    index,
    bar,
    why: `No bar opens ${o.minute ? 'in the minute' : 'at'} ${o.minute ? o.label.slice(0, 16) : o.label}. Landed on the first bar after it, ${o.when(bar.t)} IST, ${fmt(gap)} second(s) later — ${row}.`
  };
}
