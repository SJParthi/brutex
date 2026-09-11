import { ask } from '../src/lib/ask.js';

const MAX_ATTEMPTS = 3, MAX_WAIT_MS = 2_000;
const DEFAULT_WAITS_MS = [250, 500];

/** A retry wait owns and releases its timer and abort listener.
 * @param {number} delayMs @param {AbortSignal} signal */
export function waitForRetry(delayMs, signal) {
  signal.throwIfAborted();
  return new Promise((resolve, reject) => {
    const abort = () => { clearTimeout(timer); signal.removeEventListener('abort', abort); reject(signal.reason); };
    const timer = setTimeout(() => { signal.removeEventListener('abort', abort); resolve(undefined); }, delayMs);
    signal.addEventListener('abort', abort, { once: true });
  });
}

/** Return a valid server-requested minimum delay, including delays too long to
 * admit. A caller must never clamp a server minimum and retry early.
 * @param {Response} response @param {number} now */
function retryAfter(response, now) {
  const raw = response.headers?.get('retry-after');
  if (typeof raw !== 'string') return null;
  // An unreadable supplied minimum cannot safely be replaced by a shorter
  // guessed delay. Return the final response instead of retrying it early.
  if (raw.length > 128) return Infinity;
  const value = raw.trim();
  if (/^\d+$/.test(value)) {
    const millis = BigInt(value) * 1_000n;
    return millis > BigInt(MAX_WAIT_MS) ? Infinity : Number(millis);
  }
  // HTTP-date in its current IMF-fixdate form; permissive Date.parse by itself
  // would also accept unrelated strings such as a bare year or month.
  if (!/^(Mon|Tue|Wed|Thu|Fri|Sat|Sun), \d{2} (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) \d{4} \d{2}:\d{2}:\d{2} GMT$/.test(value)) return Infinity;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) && new Date(timestamp).toUTCString() === value
    ? Math.max(0, timestamp - now) : Infinity;
}

/**
 * At most three reads, only after explicit server backpressure. A generic 503
 * (changed receipt, missing evidence, byte ceiling, etc.) is never retried.
 * No body is consumed or replaced: the final response goes to its usual strict
 * receipt validator. The caller makes each retry visible through `onBusy`.
 *
 * @param {string} url
 * @param {{signal:AbortSignal,onBusy?:(info:{attempt:number,nextAttempt:number,delayMs:number,status:number})=>void}} options
 * @param {typeof ask} [request]
 * @param {{wait?:(delayMs:number,signal:AbortSignal)=>Promise<any>,now?:()=>number}} [clock] Test seam.
 * @returns {Promise<Response>}
 */
export async function fetchWithBusyRetry(url, { signal, onBusy }, request = ask, clock = {}) {
  const wait = clock.wait ?? waitForRetry, now = clock.now ?? Date.now;
  for (let attempt = 1; ; attempt += 1) {
    signal.throwIfAborted();
    const response = await request(url, { signal, cache: 'no-store' });
    signal.throwIfAborted();
    if (attempt === MAX_ATTEMPTS || ![429, 503].includes(response.status)) return response;
    const requested = retryAfter(response, now());
    if (response.status === 503 && requested === null) return response;
    const delayMs = requested ?? DEFAULT_WAITS_MS[attempt - 1];
    if (delayMs > MAX_WAIT_MS) return response;
    onBusy?.({ attempt, nextAttempt: attempt + 1, delayMs, status: response.status });
    signal.throwIfAborted();
    await wait(delayMs, signal);
    signal.throwIfAborted();
  }
}
