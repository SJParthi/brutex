// What `/autopilot.json`'s `journal_error` says about the durable record.
//
// P1-06-01, D-2661. The server added `journal_error` because "a path is not a
// proof": `journal` names where the file is, and every page read that as
// evidence the runs were being recorded while the append's own answer was
// thrown away (crates/api/src/autopilot.rs, `Status::journal_error`). The page
// then threw the field away too, and went on calling the journal "the durable
// record" while appends were failing. This module is the one reader of it.
//
// The server always emits the field as a JSON string, empty when the last
// append succeeded. Three answers, and only the first is good news:
//
//   ok       the field is present and empty: the last append was written.
//   failed   the field is present and non-empty: the append failed, and the
//            text is the server's own reason. Shown verbatim.
//   unknown  the field is absent or not a string: this server did not say,
//            and an absent answer rendered as "recorded" is the fallback
//            CLAUDE.md section 4 bans.

/**
 * @typedef {{ state: 'ok' } | { state: 'failed', why: string } | { state: 'unknown', why: string }} JournalStatus
 */

/**
 * @param {unknown} value the raw `journal_error` field of `/autopilot.json`
 * @returns {JournalStatus}
 */
export function readJournalError(value) {
  if (typeof value !== 'string') {
    return {
      state: 'unknown',
      why:
        value === undefined
          ? 'the payload carries no `journal_error`, so whether the last run reached the journal is unknown'
          : '`journal_error` is not a string, so whether the last run reached the journal is unknown'
    };
  }
  if (value.trim() === '') return { state: 'ok' };
  return { state: 'failed', why: value };
}

/**
 * May the page call the journal "the durable record"? Only when the server
 * said the last append was written.
 *
 * @param {JournalStatus} status
 * @returns {boolean}
 */
export function journalIsRecord(status) {
  return status.state === 'ok';
}

/**
 * The banner text for a journal that is not known to be recording, or null.
 *
 * @param {JournalStatus} status
 * @param {string} path the journal path the page would otherwise name
 * @returns {string | null}
 */
export function journalBanner(status, path) {
  if (status.state === 'failed') {
    return `The last run was NOT written to the journal at ${path}: ${status.why}. Until an append succeeds, what this page shows is the only record of it.`;
  }
  if (status.state === 'unknown') {
    return `Whether the last run was written to the journal at ${path} is unknown: ${status.why}.`;
  }
  return null;
}
