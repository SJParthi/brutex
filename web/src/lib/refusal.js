// The api's own reason for a refusal, read by name instead of dropped.
//
// CE-83, D-1789. Four readers printed only the HTTP status (`/store.json`,
// `/calendar.json`, `/logs.json`, and `/indexmap.json`, which read `error`
// while the unknown-feed answer sends `refused`). The server had already
// written the fix into the body; the page showed a number instead.
//
// THREE KEYS ARE ON THE WIRE, and this reads all three in one order:
//
//   error     most routes (`{"error": ...}`)
//   refused   `no_such_feed_json`, the unknown-feed 400 that /store, /calendar,
//             /indexmap, /instruments, /universes and /verify share
//   refusal   `/masters/status.json` and the masters refresh
//
// AND ONE NESTED SHAPE: `/backtest/run.json` answers an attempt it cannot read
// (503) or a malformed `?attempt=` (400) with
// `{"running":{"status":"unknown","why":…}}` (`crates/api/src/sweeprun.rs`
// `browser_attempt_unknown`, `unknown_status`), and none of the three keys
// above is set. Its `why` is read only when the status is `unknown`: a running
// or finished payload's `why` describes the run, not a refusal (W6, D-3216).
//
// A body that names none of them is not invented into one: the sentence then
// says the route named no reason, which is itself the fact worth showing.

const KEYS = ['error', 'refused', 'refusal'];

/**
 * The named reason in a parsed refusal body, or null when it names none.
 * @param {unknown} body
 * @returns {string | null}
 */
export function refusalOf(body) {
  if (body === null || typeof body !== 'object' || Array.isArray(body)) return null;
  const record = /** @type {Record<string, unknown>} */ (body);
  for (const key of KEYS) {
    const value = record[key];
    if (typeof value === 'string' && value.trim() !== '') return value.trim();
  }
  const running = record.running;
  if (running !== null && typeof running === 'object' && !Array.isArray(running)) {
    const { status, why } = /** @type {Record<string, unknown>} */ (running);
    if (status === 'unknown' && typeof why === 'string' && why.trim() !== '') return why.trim();
  }
  return null;
}

/** Longest stretch of a non-JSON body quoted back. */
const QUOTE_LIMIT = 500;

/**
 * Read a refusal response's body for its reason. Never throws: a body that
 * cannot be read or parsed yields null (or the raw text, when it is short
 * plain text) rather than a guess.
 * @param {Response} response
 * @returns {Promise<string | null>}
 */
export async function reasonOf(response) {
  let text = '';
  try {
    text = await response.text();
  } catch {
    return null;
  }
  if (text.trim() === '') return null;
  try {
    return refusalOf(JSON.parse(text));
  } catch {
    const plain = text.trim();
    return plain.length <= QUOTE_LIMIT ? plain : `${plain.slice(0, QUOTE_LIMIT)}…`;
  }
}

/**
 * The whole sentence for a refused read: the route, the status, and the
 * server's own reason, or a statement that it named none.
 * @param {string} route e.g. `/calendar.json`
 * @param {number} status
 * @param {string | null} reason
 * @returns {string}
 */
export function refusalSentence(route, status, reason) {
  return reason
    ? `${route} answered HTTP ${status}: ${reason}`
    : `${route} answered HTTP ${status} and named no reason`;
}

/**
 * `reasonOf` and `refusalSentence` together, for a reader that has the
 * response in hand.
 * @param {string} route
 * @param {Response} response
 * @returns {Promise<string>}
 */
export async function refusalFrom(route, response) {
  return refusalSentence(route, response.status, await reasonOf(response));
}

// THE STAMPED HALF: `/instruments.json` and `/store.json` put their reason in
// HEADERS, because their body is a JSON array in every state (D-0124).
//
// `/instruments.json` answers 503 when the census will not load OR the feed's
// master will not decode, and stamps both (`crates/api/src/server.rs`,
// `instruments_json`); `/store.json` answers 503 for an unreadable census, and
// a HEAD of it has no body at all. Three readers printed the status alone, and
// the catalogue loader printed the MASTER's sentence whatever had failed, so an
// unreadable census read "read — <feed>: master read; …": a master that was
// fine, blamed, and the census note that named the failure dropped (W2,
// D-3212). The master half is named only when the master did not read.

const CENSUS_STATE = 'x-brutex-census-state';
const CENSUS_NOTE = 'x-brutex-census-note';
const MASTER_STATE = 'x-brutex-master-state';
const MASTER_NOTE = 'x-brutex-master-note';

/**
 * Why a census- or master-stamped answer is not a measurement, read from its
 * headers, or null when neither stamp names a failure.
 * @param {Headers | null | undefined} headers
 * @returns {string | null}
 */
export function headerRefusal(headers) {
  const read = (/** @type {string} */ name) => (headers?.get?.(name) ?? '').trim();
  const parts = [];
  const master = read(MASTER_STATE);
  if (master !== '' && master !== 'read') {
    const note = read(MASTER_NOTE);
    parts.push(note ? `${master} — ${note}` : `the instrument master is ${master} and the response carried no master note`);
  }
  if (read(CENSUS_STATE) === 'unreadable') {
    const note = read(CENSUS_NOTE);
    parts.push(note ? `the store census is unreadable: ${note}` : 'the store census is unreadable and the response carried no census note');
  }
  return parts.length > 0 ? parts.join('; ') : null;
}

/**
 * The sentence for a refused stamped read: the headers' reason first, then the
 * body's (an unknown feed's 400 names itself in `refused`), else none named.
 * @param {string} route
 * @param {Response} response
 * @returns {Promise<string>}
 */
export async function headerRefusalFrom(route, response) {
  return refusalSentence(route, response.status, headerRefusal(response.headers) ?? (await reasonOf(response)));
}
