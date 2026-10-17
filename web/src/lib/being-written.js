/**
 * A MONTH BEING WRITTEN IS ASKED AGAIN, BY ITSELF (R3-08, NEW-21).
 *
 * A read that meets an import worker, a month repair or an index build on
 * its month waits briefly on the server, then answers 503 with
 * `"being_written": true` and `Retry-After` — never "unreadable". Every page
 * read that can meet one goes through here, so there is one rule for asking
 * again, not one per page: after the server's own delay (one second when it
 * names none, never more than two, jittered), at most
 * `BEING_WRITTEN_TRIES` requests in all. Past that the last answer is
 * returned as it is, and its sentence — which names who holds the month —
 * is the failure the caller shows. A 503 without the field (a pool that
 * could not join its task, a census that does not read) is not a "come
 * back" and is returned at once.
 */

/** Requests in all, the first included, for one read that meets a writer. */
export const BEING_WRITTEN_TRIES = 4;

/**
 * Waits `ms`, or rejects at once when `signal` is aborted.
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
 * Whether an answer says "being written, ask again".
 * @param {{ status: number }} response @param {any} body the parsed body
 */
export function isBeingWritten(response, body) {
  return response?.status === 503 && body?.being_written === true;
}

/**
 * The pause before asking again: the server's `Retry-After` in seconds, at
 * most two, else one second; plus up to 250 ms of jitter so many cells that
 * met one writer do not all ask in the same instant.
 * @param {{ headers?: { get(name: string): string | null } }} response
 * @param {() => number} [random]
 */
export function askAgainDelay(response, random = Math.random) {
  const named = Number(response?.headers?.get?.('Retry-After'));
  const seconds = Number.isFinite(named) && named > 0 ? Math.min(named, 2) : 1;
  return seconds * 1000 + Math.floor(random() * 250);
}

/**
 * One GET, asked again while the server says its month is being written.
 *
 * @param {(url: string, options: { signal: AbortSignal }) => Promise<any>} request
 * @param {string} url
 * @param {AbortSignal} signal
 * @param {(ms: number, signal: AbortSignal) => Promise<unknown>} [wait]
 * @returns {Promise<{ res: any, body: any }>} the last answer and its parsed body
 */
export async function askWhileBeingWritten(request, url, signal, wait = pause) {
  for (let attempt = 1; ; attempt += 1) {
    signal.throwIfAborted();
    const res = await request(url, { signal });
    const body = await res.json();
    if (!isBeingWritten(res, body) || attempt >= BEING_WRITTEN_TRIES) return { res, body };
    await wait(askAgainDelay(res), signal);
  }
}
