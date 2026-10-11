import { ask } from './ask.js';

/* ======================================================================
   ONE PAGE OF THE BAR GRID, READ IN ONE REQUEST (D-3326)
   ----------------------------------------------------------------------
   This file used to read a page FILE BY FILE: the census said which month
   files held global rows N..N+50, and each was asked for its own slice with
   `from=to=<its month>`. Every defect of that shape was a defect of /db:

   * the offsets came from the CENSUS's row counts, so a month whose file had
     outgrown its census row (NIFTY 2019-11-28 12000 CE: census 60,070, file
     192,062) left most of its bars unreachable, and ascending and descending
     showed different bars (DB-G02, DB-G61);
   * several series were read one after another, never merged by time, so a
     CE+PE pair was stacked and "newest first" was false (DB-002, DB-G92);
   * each month was read alone, so the first bar of every month lost its
     change (DB-011, DB-G65).

   The server now does all of it: `key=` names every series, the merge and the
   paging are on the server, the total is the files' own, and the change of a
   page's first bar is measured against the stored bar before it wherever it
   lies. So this file asks once and checks what came back.
   ====================================================================== */

/** Attempts at a request the server's scan pool refused as saturated. */
export const SATURATED_TRIES = 4;

/**
 * A pause that ends early, and throws, when `signal` aborts.
 * @param {number} ms @param {AbortSignal} signal
 */
export function pause(ms, signal) {
  return new Promise((resolve, reject) => {
    signal.throwIfAborted();
    const t = setTimeout(resolve, ms);
    signal.addEventListener('abort', () => { clearTimeout(t); reject(signal.reason); }, { once: true });
  });
}

/**
 * ONE WINDOW REQUEST, CHECKED, WITH A BOUNDED RETRY WHEN THE SCAN POOL IS FULL.
 *
 * `/bars/window.json` runs a scan (a value sort, a time-of-day bound, an
 * extremes fold) in a pool of two and REFUSES the third with 429 rather than
 * queueing it. A burst of those once took 466 of 596 requests of one page
 * (DB-B49). With one request a page that burst is gone, and the rare 429 that
 * remains — another tab, the extremes fold beside a sort — is retried after a
 * jittered pause, at most `SATURATED_TRIES` times. Past that it is a failure
 * with the server's own sentence, never an empty page.
 *
 * The answer is checked before it is believed: a total that is not a whole
 * number, more rows than were asked for, or rows short of the total with no
 * fault named is a failure, not a page.
 *
 * @param {string} query the query string `windowQuery` built
 * @param {number} limit the rows asked for
 * @param {AbortSignal} signal
 * @param {typeof ask} [request]
 * @param {(ms: number, signal: AbortSignal) => Promise<unknown>} [wait]
 * @returns {Promise<any>} the parsed, checked answer
 */
export async function readWindowPage(query, limit, signal, request = ask, wait = pause) {
  for (let attempt = 1; ; attempt += 1) {
    signal.throwIfAborted();
    const response = await request(`/bars/window.json?${query}`, { signal });
    let body = null;
    try {
      body = await response.json();
    } catch {
      signal.throwIfAborted();
      throw new Error(`HTTP ${response.status}, and the body was not JSON`);
    }
    signal.throwIfAborted();
    if (response.status === 429 && attempt < SATURATED_TRIES) {
      await wait(250 * 2 ** (attempt - 1) + Math.floor(Math.random() * 250), signal);
      continue;
    }
    if (!response.ok) throw new Error(String(body?.error ?? `HTTP ${response.status}`));
    const offset = Number(body?.offset);
    if (!body || !Array.isArray(body.bars) || !Number.isSafeInteger(body.total) || body.total < 0 ||
        !Number.isSafeInteger(offset) || offset < 0 || body.bars.length > limit ||
        !Array.isArray(body.series_totals) ||
        (body.faults !== null && body.faults !== undefined && typeof body.faults !== 'string')) {
      throw new Error('The server did not return the bounded bar page that was asked for.');
    }
    const due = Math.min(limit, Math.max(0, body.total - offset));
    if (!body.faults && response.status === 200 && body.bars.length !== due) {
      throw new Error(`The bar page holds ${body.bars.length} row(s) where ${due} were due, and the server named no fault.`);
    }
    return body;
  }
}

/**
 * EVERY KEPT ROW OF A SELECTION, IN TIME ORDER, for a sort the browser does
 * itself or a whole-query export — refused before any read past `budget`.
 *
 * `total` is the server's own count of the rows the query keeps; the read is
 * `ceil(total / page)` requests of `page` rows, sequential, so the cost is
 * stated before it is paid and is never a month file's whole JSON.
 *
 * @param {(offset: number, limit: number) => string} queryAt the query for one slice
 * @param {number} total @param {number} budget @param {number} page
 * @param {AbortSignal} signal
 * @param {typeof ask} [request]
 * @param {(read: number) => void} [progress]
 * @returns {Promise<{bars: any[], faults: string[]}>}
 */
export async function readAllRows(queryAt, total, budget, page, signal, request = ask, progress = () => {}) {
  if (!Number.isSafeInteger(total) || total < 0) throw new Error('The selection has no count to read against.');
  if (total > budget) {
    throw new Error(`over_budget: the query keeps ${total} rows and the browser reads at most ${budget} to order or write them`);
  }
  /** @type {any[]} */
  const bars = [];
  /** @type {string[]} */
  const faults = [];
  for (let offset = 0; offset < total; offset += page) {
    const body = await readWindowPage(queryAt(offset, page), page, signal, request);
    for (const b of body.bars) bars.push(b);
    if (body.faults) faults.push(body.faults);
    progress(bars.length);
    if (body.bars.length === 0) break;
  }
  return { bars, faults };
}

/**
 * WHAT A VALUE SORT READ INSTEAD OF SERVING FROM SORT INDEXES (D-3349,
 * DB-N14), as one sentence for the notes under the grid — `null` when every
 * kept row came from an index, or when the order is not a value sort.
 *
 * `/bars/window.json` orders a price, the volume or the open interest from
 * each month's `.srt` sort index; rows no index orders (a month without one
 * yet, rows appended after it, any order under a time-of-day bound) are READ,
 * under a cap, and the answer's `sort_index` names how many, in which months,
 * why, and the `cli srt-build` command that indexes them. Never a silent slow
 * sort: the page says what the server said.
 *
 * @param {any} answer a `/bars/window.json` body
 * @returns {string | null}
 */
export function sortIndexNote(answer) {
  const report = answer?.sort_index;
  if (!report || typeof report !== 'object') return null;
  const count = (/** @type {unknown} */ v) => (Number.isSafeInteger(v) && Number(v) > 0 ? Number(v) : 0);
  const text = (/** @type {number} */ n) => n.toLocaleString('en-IN');
  if (report.by_time === true) {
    return 'No row of this selection carries open interest (an index or a share has none), so ordering by it is time order.';
  }
  const read = count(report.scanned_rows);
  if (read === 0) return null;
  const why = Array.isArray(report.scanned_why) ? report.scanned_why.filter((w) => typeof w === 'string') : [];
  const months = Math.max(count(report.scanned_months), why.length);
  const more = months > why.length ? `; and ${text(months - why.length)} more month(s)` : '';
  const build = Array.isArray(report.build) ? report.build.filter((b) => typeof b === 'string') : [];
  const column = typeof report.column === 'string' ? report.column : 'this column';
  return `Ordered by ${column}: ${text(count(report.indexed_rows))} row(s) came from sort indexes and ` +
    `${text(read)} row(s) in ${text(months)} month(s) were read instead${why.length ? ` — ${why.join('; ')}${more}` : ''}.` +
    (build.length ? ` Index them with: ${build.join(' ; ')}` : '');
}
