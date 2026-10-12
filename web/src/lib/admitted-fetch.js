import { ask } from './ask.js';
import { fetchWithBusyRetry, waitForRetry } from '../../saved-backtest/requests.js';

/**
 * A READ THAT WAITS ITS TURN WHEN THE SERVER SAYS IT IS BUSY (DB-B51, WP-81).
 *
 * Ten `/db` tabs at once met 429 on 271 of 480 window seeks, and the bar grid
 * never asked again: the refusal became the page. The server now queues a short
 * read for a quarter second and sends `Retry-After` on every 429; this asks
 * again, at most twice, after the server's own delay — or 250 then 500 ms when
 * it names none — plus up to `JITTER_MS` of jitter, so ten tabs refused at one
 * instant do not all return at the next. The retry rules are
 * `fetchWithBusyRetry`'s (a generic 503 is never retried; a delay past two
 * seconds is not waited); only the jitter is added here.
 */
export const JITTER_MS = 250;

/**
 * @param {string} url
 * @param {{signal: AbortSignal}} options
 * @param {typeof ask} [request]
 * @param {{random?: () => number, wait?: (delayMs:number, signal:AbortSignal) => Promise<any>}} [seam] Test seam.
 * @returns {Promise<Response>}
 */
export function fetchAdmitted(url, { signal }, request = ask, seam = {}) {
  const random = seam.random ?? Math.random;
  const wait = seam.wait ?? waitForRetry;
  return fetchWithBusyRetry(url, { signal }, request, {
    wait: (delayMs, waiting) => wait(delayMs + Math.floor(random() * JITTER_MS), waiting)
  });
}
