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
