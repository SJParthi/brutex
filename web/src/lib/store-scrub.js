/**
 * THE SCRUB, READ — `/verify.json` checks one feed's census against the bar
 * files it counts (sobs-10, D-4454).
 *
 * Every other completeness surface on `/db` answers from the census, which is
 * one file validated only against itself. The scrub is the one check that
 * opens the bar files, and until this panel existed no page asked for it: its
 * answer lived only in a reply nobody requested. The server now also writes
 * every scrub to the event log as `api.verify` "scrub", so a result read here
 * is also on `/logs` after the tab is closed.
 *
 * Pure: no runes and no DOM, so `web/tests/store-scrub.test.js` drives it
 * under `node --test`.
 *
 * Nothing here invents a verdict. A reply that is not the shape the server
 * writes is refused by name, and a refused question is drawn as refused, never
 * as a clean store: `verified` is false whenever nothing could be checked, and
 * this reader keeps that distinction rather than collapsing it.
 */

/** How long a scrub may take before the page stops waiting, in milliseconds.
 * A scrub opens one bar file per entry the counter holds, so it is the one
 * local read on this page that grows with the store. */
export const SCRUB_MS = 120_000;

/** The counts the reply carries, in the order the panel draws them. */
export const COUNTS = Object.freeze([
  ['seen', 'Entries examined'],
  ['agreed', 'Agree with their file'],
  ['missing', 'Counted, but the file is absent'],
  ['rows', 'Row count differs from the file'],
  ['bounds', 'First or last bar differs from the file'],
  ['unreadable', 'File could not be read'],
]);

/** @param {string} feed The feed's wire name. */
export function scrubUrl(feed) {
  return '/verify.json?feed=' + encodeURIComponent(feed);
}

/** @param {unknown} value */
const isCount = (value) => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

/**
 * @typedef {{
 *   verdict: 'verified' | 'disagrees' | 'empty' | 'refused' | 'unanswered',
 *   say: string,
 *   feed: string,
 *   counts: { key: string, label: string, n: number }[],
 *   findings: string[],
 *   undrawn: number,
 * }} Scrub
 */

/**
 * One reply, read. Throws when the body is not a shape the server writes.
 * @param {number} status
 * @param {any} body
 * @returns {Scrub}
 */
export function readScrub(status, body) {
  const unanswered = (/** @type {string} */ say) => ({
    verdict: /** @type {const} */ ('unanswered'),
    say,
    feed: typeof body?.feed === 'string' ? body.feed : '',
    counts: [],
    findings: [],
    undrawn: 0,
  });
  if (body === null || typeof body !== 'object' || Array.isArray(body)) {
    throw new Error(`the scrub answered ${status} with a body that is not an object`);
  }
  // NOT ADMITTED: every scrub slot was taken, or the task could not be joined.
  if (typeof body.error === 'string') return unanswered(body.error);
  // NOT A FEED: the question was refused before any file was opened.
  if (!('verified' in body)) {
    if (typeof body.refused === 'string') return unanswered(body.refused);
    throw new Error(`the scrub answered ${status} without a verdict`);
  }
  if (typeof body.verified !== 'boolean' || typeof body.say !== 'string') {
    throw new Error(`the scrub answered ${status} with a verdict this page cannot read`);
  }
  if (!Array.isArray(body.findings) || !body.findings.every((/** @type {unknown} */ f) => typeof f === 'string')) {
    throw new Error(`the scrub answered ${status} with findings this page cannot read`);
  }
  const missing = COUNTS.map(([key]) => key).filter((key) => !isCount(body[key]));
  if (missing.length || !isCount(body.undrawn)) {
    throw new Error(
      `the scrub answered ${status} without a whole count for ${[...missing, ...(isCount(body.undrawn) ? [] : ['undrawn'])].join(', ')}`
    );
  }
  const refused = body.refused !== null && body.refused !== undefined;
  /** @type {Scrub['verdict']} */
  const verdict = refused
    ? 'refused'
    : body.verified
      ? 'verified'
      : body.seen === 0
        ? 'empty'
        : 'disagrees';
  return {
    verdict,
    say: body.say,
    feed: typeof body.feed === 'string' ? body.feed : '',
    counts: COUNTS.map(([key, label]) => ({ key, label, n: body[key] })),
    findings: body.findings,
    undrawn: body.undrawn,
  };
}

/**
 * Ask for one scrub and read the reply.
 * @param {string} feed
 * @param {(url: string) => Promise<Response>} fetcher
 * @returns {Promise<Scrub>}
 */
export async function fetchScrub(feed, fetcher) {
  const response = await fetcher(scrubUrl(feed));
  let body;
  try {
    body = await response.json();
  } catch {
    throw new Error(`the scrub answered ${response.status} with a body that is not JSON`);
  }
  return readScrub(response.status, body);
}

/** The heading a verdict is drawn under. @param {Scrub['verdict']} verdict */
export function verdictHeading(verdict) {
  switch (verdict) {
    case 'verified':
      return 'The counter agrees with every file it counts';
    case 'disagrees':
      return 'The counter disagrees with its files';
    case 'empty':
      return 'Nothing to check: the counter holds no entry';
    case 'refused':
      return 'Not checked: the counter could not be read';
    default:
      return 'The scrub was not run';
  }
}
