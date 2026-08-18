/**
 * ONE FETCH THAT CAN END — the arithmetic-free half, with no runes in it, so
 * `web/tests/ask.test.js` can drive it under `node --test`.
 *
 * # The failure this exists to remove
 *
 * `AbortSignal.timeout` appeared ZERO times in `web/src`. Every one of the
 * twelve `fetch` call sites in this app would wait for as long as the other end
 * held the socket open, and the only `AbortController` in the tree is an
 * operator-pressed cancel on `/ingest`. So a Rust process that accepted the
 * connection and then wedged -- a lock held, a disk stalled, a debugger paused
 * -- left `/db` reading "Reading…" and `/autopilot` reading "Working…" forever,
 * indistinguishable from slow, with no way to tell which it was.
 *
 * That is the fallback that hides a failure, in the one shape CLAUDE.md
 * section 4 names outright: it neither degrades loudly nor refuses. A spinner
 * that never resolves is a claim that the request is still coming.
 *
 * # Why a wrapper rather than a signal at each site
 *
 * Twelve sites, and each already has a `catch` that formats a sentence for the
 * operator. Handing each one an `AbortSignal` would time the request out and
 * then print `AbortError: signal is aborted without reason`, or in newer
 * runtimes `TimeoutError: The operation was aborted due to timeout` -- true,
 * and useless to somebody deciding whether the server is wedged or the network
 * is slow. `ask` throws an `Error` whose message is the sentence those catches
 * should print, so the call sites change by one word and gain a real answer.
 *
 * # The ceiling
 *
 * 15 seconds. Every route this app calls is a local read against a store on
 * the same machine -- no vendor request is ever made from the browser -- and
 * the slowest measured answer in the tree is a census fold over ~94k rows.
 * A ceiling below that would refuse work that is genuinely progressing; one
 * far above it is a spinner with extra steps. `/pull/spot` passes its own
 * larger ceiling, because a pull is the one route whose work is not local.
 */

/** The default ceiling for a local read, in milliseconds. */
export const ASK_MS = 15_000;

/**
 * `fetch`, with an end.
 *
 * Identical to `fetch` in every respect except that it cannot outlive `ms`,
 * and that the error it throws on timeout says what happened and what it means
 * rather than naming the mechanism that stopped it.
 *
 * A caller that passes its own `signal` keeps it: `AbortSignal.any` makes the
 * request answer to both, so an operator-pressed cancel still cancels and the
 * ceiling still applies. Dropping one for the other would silently disable a
 * control somebody is pressing.
 *
 * @param {string} url
 * @param {RequestInit & { ms?: number }} [options] `ms` overrides the ceiling.
 * @returns {Promise<Response>}
 */
export async function ask(url, options = {}) {
  const { ms = ASK_MS, signal, ...rest } = options;
  const ceiling = AbortSignal.timeout(ms);
  const merged = signal ? AbortSignal.any([signal, ceiling]) : ceiling;
  try {
    return await fetch(url, { ...rest, signal: merged });
  } catch (error) {
    // The caller's own abort is NOT a timeout and must not be reported as one.
    // An operator who pressed Cancel is owed the word they pressed.
    if (signal?.aborted) throw error;
    if (ceiling.aborted) {
      throw new Error(
        `No answer from ${url} within ${Math.round(ms / 1000)} s, so the request was ` +
          `given up rather than left waiting. The server accepted the connection and ` +
          `did not finish: it is wedged or the store is locked, which is a different ` +
          `fact from being slow.`
      );
    }
    throw error;
  }
}
