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

/**
 * `request(url, options)`, asked again while the server says its month is
 * being written — for a reader that keeps the RESPONSE: the last one comes
 * back with its body unread, so a caller that checks `response.ok` and quotes
 * `refusalFrom` keeps doing exactly that, and its last failure is the
 * server's sentence naming who holds the month. Only a 503 is looked at, its
 * body read from a clone; any other answer is returned as it came.
 *
 * The /db chart's folded rungs (`/sources/bars.json`), the day flags
 * (`/dayflags.json`) and the sources pages read through here (R3-08, RA-117):
 * beside a writer-held month each used to show the first 503 — or, for the
 * day flags, every day "unreadable" — and never ask again.
 *
 * @param {(url: string, options: any) => Promise<any>} request `ask`, or a counting wrapper of it
 * @param {string} url
 * @param {{ signal?: AbortSignal } & Record<string, any>} [options] passed to every request
 * @param {(ms: number, signal: AbortSignal) => Promise<unknown>} [wait]
 * @returns {Promise<any>} the last response, its body unread
 */
export async function askAgainWhileWritten(request, url, options = {}, wait = pause) {
  const signal = options.signal ?? new AbortController().signal;
  for (let attempt = 1; ; attempt += 1) {
    signal.throwIfAborted();
    const res = await request(url, options);
    if (res?.status !== 503 || attempt >= BEING_WRITTEN_TRIES) return res;
    let body = null;
    try {
      body = await res.clone().json();
    } catch {
      return res;
    }
    if (!isBeingWritten(res, body)) return res;
    await wait(askAgainDelay(res), signal);
  }
}
