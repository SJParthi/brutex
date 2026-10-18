// AH-04 (g-ui-dom-vs-json-4, D-5880): what the /audit header and its "Last
// commit" cell say, folded out of the page so a test drives the words.
import { whole } from './money.js';

/**
 * THE HEADER COUNTS THIS FEED'S RUNS, NOT THE WHOLE JOURNAL'S RECORDS.
 *
 * It printed `journal.records` — every record the pull journal holds, for
 * every vendor — and called them "runs": `42,17,222 runs` on the GDFL console
 * while `journal.feed_scope` said GDFL holds 0 records in 0 runs. A record is
 * not a run (a run that refused 409 members wrote 410 records), and another
 * feed's records are not this feed's. The header now says this feed's runs
 * and records, from `feed_scope`, and keeps the whole journal's count in the
 * title, named as all feeds'.
 *
 * @param {any} journal `/audit.json`'s `journal` block
 * @returns {{ text: string, title: string }}
 */
export function journalHeader(journal) {
  const total = Number.isSafeInteger(journal?.records) ? journal.records : null;
  const all = total === null ? 'an unstated number of' : whole(total);
  const scope = journal?.feed_scope;
  const feed = typeof scope?.feed === 'string' ? scope.feed : 'this feed';
  if (!scope || typeof scope !== 'object') {
    return {
      text: `${feed}: runs not counted`,
      title: `This answer carries no feed_scope, so this feed's runs are not counted. The pull journal holds ${all} records over every feed.`
    };
  }
  if (typeof scope.error === 'string') {
    return {
      text: `${feed}: runs not counted`,
      title: `This feed's runs could not be counted: ${scope.error}. The pull journal holds ${all} records over every feed.`
    };
  }
  const runs = Number.isSafeInteger(scope.runs) ? scope.runs : null;
  const records = Number.isSafeInteger(scope.records) ? scope.records : null;
  const unattributed = Number.isSafeInteger(scope.unattributed_records) ? scope.unattributed_records : 0;
  return {
    text: `${feed}: ${runs === null ? '—' : whole(runs)} runs · ${records === null ? '—' : whole(records)} records`,
    title:
      `The pull journal's runs and records for ${feed} (journal.feed_scope). ` +
      `The journal holds ${all} records over every feed` +
      (unattributed > 0 ? `, ${whole(unattributed)} of them unattributed to any feed.` : '.')
  };
}

/**
 * The "Last commit" cell's second line: the IST stamp and what it stands on,
 * in `committed_from`'s words.
 *
 * `committed_at` is the newest CURRENT census file's mtime (one stat per
 * segment of a migrated vendor, or the one file); the label said "manifest
 * mtime" while it stat'ed a directory. A file that would not stat makes the
 * stamp a lower bound and says so; a stamp later than the server's own `at`
 * is shown as it is, named as ahead of the clock.
 *
 * @param {any} store `/audit.json`'s `store` block
 * @param {number | null} now the answer's own `at`
 * @param {(secs: number) => string} ist epoch seconds as an IST stamp
 * @returns {string}
 */
export function commitLine(store, now, ist) {
  const from = store?.committed_from;
  const at = Number.isSafeInteger(store?.committed_at) ? store.committed_at : null;
  if (at === null) {
    return typeof from?.note === 'string' ? `unknown: ${from.note}` : 'unknown: the census file has no readable mtime';
  }
  const basis =
    from?.basis === 'newest census file'
      ? `newest of ${whole(Array.isArray(from.files) ? from.files.filter((/** @type {any} */ f) => f?.state === 'read').length : 0)} current census file(s)`
      : from?.basis === 'manifest file'
        ? 'the census file'
        : 'census file mtime';
  const parts = [`${ist(at)} IST · ${basis}`];
  if (from?.complete === false) parts.push(`AT LEAST this recent: ${from.note ?? 'a census file would not stat'}`);
  if (now !== null && Number.isFinite(now) && at > now) {
    parts.push(`${whole(at - now)} s AHEAD of the server's clock (the file's mtime, shown as it is)`);
  }
  return parts.join(' · ');
}

/**
 * Seconds since the last commit, or `null` when it is unknown OR later than
 * the server's own clock — a negative age is not "just wrote".
 *
 * @param {any} store
 * @param {number | null | undefined} now
 */
export function sinceCommit(store, now) {
  const at = Number.isSafeInteger(store?.committed_at) ? store.committed_at : null;
  if (at === null || typeof now !== 'number' || !Number.isFinite(now)) return null;
  return now >= at ? now - at : null;
}
