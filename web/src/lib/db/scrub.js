/**
 * WHAT THE FULL-BLOCK SCRUB FOUND IN THE SERIES /db IS SHOWING (DB-B74,
 * DB-B65; WP-81).
 *
 * A block that rotted in the middle of a month was refused only to whoever
 * read that exact block: the census still counted its bars, coverage was
 * unchanged and no page knew. `cli scrub-blocks` reads every block and keeps
 * what would not verify; `/scrub.json` answers the findings that still stand
 * for the series a page names. This turns that answer into the lines /db
 * prints under its grid — and says, rather than staying blank, when no scrub
 * of the feed has ever run: "nothing found" and "never looked" are different.
 */

/** The most series one `/scrub.json` request names (the server's `MAX_KEYS`). */
export const SCRUB_MAX_KEYS = 64;

/** How long /db waits before it asks again while the server is still indexing
 * its findings ledger (`truncated`), and how often at most. The server reads
 * 16 MiB of ledger per request, so this covers a ledger of about 1.9 GB; past
 * it the note says so and stops asking. */
export const SCRUB_REASK_MS = 1000;
export const SCRUB_REASK_MOST = 120;

/** The most busy months the note's head names; the rest are counted. */
export const SCRUB_BUSY_NAMED = 6;

/**
 * Whether /db asks `/scrub.json` again by itself, and after how long: only
 * while the answer says its index is not caught up, and at most
 * `SCRUB_REASK_MOST` times. An answer that is whole, or not an answer, is
 * never asked again.
 * @param {any} body the parsed answer @param {number} tries asks made so far
 * @returns {number | null} milliseconds to wait, or `null` to stop
 */
export function scrubAskAgainMs(body, tries) {
  if (!body || body.recorded !== true || body.truncated !== true) return null;
  return Number.isFinite(tries) && tries < SCRUB_REASK_MOST ? SCRUB_REASK_MS : null;
}

/**
 * The `/scrub.json` URL for a selection, or `null` when there is nothing to
 * ask: no feed, no series, or more than one rung.
 * @param {string | null | undefined} feed
 * @param {{keys: string[], timeframe: string} | null | undefined} plan
 * @returns {string | null}
 */
export function scrubUrl(feed, plan) {
  if (!feed || !plan || !plan.timeframe || !Array.isArray(plan.keys) || plan.keys.length === 0) return null;
  const q = new URLSearchParams({ feed, timeframe: plan.timeframe });
  for (const key of plan.keys.slice(0, SCRUB_MAX_KEYS)) q.append('key', key);
  return `/scrub.json?${q}`;
}

/** One finding as a sentence.
 * @param {string} key @param {any} f @returns {string} */
export function findingText(key, f) {
  const where = `${key} ${f?.month ?? '?'}`;
  const pass = Number(f?.pass) > 0 ? `pass ${f.pass}` : 'an earlier pass';
  // KEPT, NOT FOUND AGAIN: a later pass met the month busy and did not read
  // it, so the finding stands on the word of the pass that last read it.
  const kept = f?.kept_pass === null || f?.kept_pass === undefined ? ''
    : `; kept, not read again: ${Number(f.kept_pass) > 0 ? `pass ${f.kept_pass}` : 'a later pass'} met this month busy`;
  switch (f?.kind) {
    case 'bad':
      return `${where}: block ${f.block} (records ${f.first}–${f.last}) does not match its stored checksum — found by scrub ${pass}${kept}`;
    case 'unverifiable':
      return `${where}: cannot be verified (${f.why || 'no reason recorded'}) — scrub ${pass}${kept}`;
    case 'misfiled':
      return `${where}: filed under another name (${f.why || 'no reason recorded'}) — scrub ${pass}${kept}`;
    case 'missing':
      return `${where}: the census holds this month and its file is gone — scrub ${pass}${kept}`;
    default:
      return `${where}: ${String(f?.kind ?? 'an unnamed finding')} — scrub ${pass}${kept}`;
  }
}

/**
 * The months of the answer's series that a standing pass met busy — a writer
 * held them when the pass reached them — and DID NOT READ, as one sentence;
 * `''` when there is none. A pass does not vouch for a month it did not read,
 * and what earlier passes found there is kept (the server's `Index::keep`).
 * @param {any[] | null | undefined} series the answer's `series` @returns {{count: number, text: string}}
 */
export function busyText(series) {
  /** @type {string[]} */
  const named = [];
  let count = 0;
  for (const s of Array.isArray(series) ? series : []) {
    const busy = Array.isArray(s?.busy) ? s.busy : [];
    for (const b of busy) {
      count += 1;
      if (named.length < SCRUB_BUSY_NAMED) {
        named.push(`${String(s.key)} ${b?.month ?? '?'} (${Number(b?.pass) > 0 ? `pass ${b.pass}` : 'a pass that cannot be dated'})`);
      }
    }
    count += Number(s?.busy_more) || 0;
  }
  if (count === 0) return { count, text: '' };
  const rest = count - named.length;
  return {
    count,
    text: ` ${count} month(s) not read: busy (a writer held them when the pass reached them), earlier findings kept — ${named.join(', ')}${rest > 0 ? ` and ${rest} more` : ''}.`
  };
}

/**
 * The note for one `/scrub.json` body.
 *
 * `tone` is `warn` when something stands against the selection, `muted`
 * otherwise. `lines` are the findings, one sentence each; `head` always says
 * which pass, whether it is finished, and what that means for this selection.
 *
 * @param {any} body the parsed answer
 * @param {number} asked how many series the selection holds
 * @returns {{tone: 'warn' | 'muted', head: string, lines: string[]}}
 */
export function scrubNote(body, asked) {
  const feed = String(body?.feed ?? 'this feed');
  if (!body || body.recorded !== true) {
    return {
      tone: 'muted',
      head: `Blocks never scrubbed for ${feed}: a block damaged in the middle of a month is found only when that block is read. ${body?.why ? String(body.why) : ''}`.trim(),
      lines: []
    };
  }
  const series = Array.isArray(body.series) ? body.series : [];
  /** @type {string[]} */
  const lines = [];
  let more = 0;
  for (const s of series) {
    for (const f of Array.isArray(s?.findings) ? s.findings : []) lines.push(findingText(String(s.key), f));
    more += Number(s?.more) || 0;
  }
  const state = body.state === 'complete' ? 'complete' : 'in progress';
  const unasked = Math.max(0, asked - series.length);
  const scope = unasked > 0 ? ` (${series.length} of ${asked} series asked; narrow the selection to ask the rest)` : '';
  const partial = body.truncated ? ' The findings ledger is still being indexed after a restart; findings may be missing until it has caught up, so ask again.' : '';
  const busy = busyText(series);
  if (lines.length === 0 && more === 0) {
    return {
      // A PASS THAT DID NOT READ A MONTH OF THE SELECTION IS NOT A CLEAN ONE.
      tone: busy.count > 0 ? 'warn' : 'muted',
      head: `Scrub pass ${body.pass} (${state}) names no damaged block in ${busy.count > 0 ? 'the months it read of ' : ''}this selection${scope}.` +
        busy.text + (state === 'complete' ? '' : ' Months it has not reached yet are not vouched for.') + partial,
      lines: []
    };
  }
  return {
    tone: 'warn',
    head: `Scrub pass ${body.pass} (${state}): ${lines.length + more} finding(s) stand against this selection${scope}.` +
      (more > 0 ? ` ${more} more are not listed.` : '') + busy.text + partial,
    lines
  };
}
