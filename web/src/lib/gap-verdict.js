/**
 * Completeness requires usable evidence as well as zero measured absences.
 * The optional fields keep older minute-audit responses readable.
 *
 * @typedef {{
 *   expected: number,
 *   lost_minutes: number,
 *   unmeasured_minutes: number,
 *   truncated: boolean,
 *   unreadable_records: number | null,
 *   invalid_timestamps?: number,
 *   summary?: string,
 *   absent_file: string | null,
 *   being_written?: string | null,
 *   evidence_error?: string | null
 * }} MonthEvidence
 * @typedef {{ word: string, kind: 'ok' | 'bad' | 'absent' | 'quiet' }} Verdict
 */

/**
 * File absence, damaged input and incomplete evidence precede arithmetic.
 * Known gap runs remain separate evidence and must still be shown by callers.
 * @param {MonthEvidence} month
 * @returns {Verdict}
 */
export function monthVerdict(month) {
  // A WRITER HELD THE MONTH (R3-08 repair 2, NEW-21): it was not read, so it
  // owes and loses nothing in this answer, and is neither "no file" nor whole.
  if (month.being_written != null) return { word: 'being written', kind: 'absent' };
  if (month.absent_file != null) return { word: 'no file', kind: 'absent' };
  if ((month.invalid_timestamps ?? 0) > 0) return { word: 'invalid stamps', kind: 'bad' };
  if (month.unreadable_records > 0) return { word: 'unreadable', kind: 'bad' };
  if (month.evidence_error != null) return { word: 'unverified', kind: 'absent' };
  if (month.truncated) return { word: 'incomplete', kind: 'absent' };
  if (month.unmeasured_minutes > 0) return { word: 'unmeasured', kind: 'absent' };
  if (month.lost_minutes > 0) return { word: 'short', kind: 'bad' };

  // Missing required fields are not zeroes. An empty error string is still an
  // error; only null or an omitted optional evidence_error means no error.
  if (
    month.absent_file !== null ||
    month.truncated !== false ||
    // NULL ONLY WHERE NOTHING WAS READ (R3-06): a month answered off its kept
    // summary read no record, so the server sends `null` rather than a `0` it
    // never counted. Anywhere else a null is a missing field, not a zero.
    !(
      month.unreadable_records === 0 ||
      (month.unreadable_records === null && month.summary === 'kept')
    ) ||
    (month.invalid_timestamps !== undefined && month.invalid_timestamps !== 0) ||
    month.unmeasured_minutes !== 0 ||
    month.lost_minutes !== 0 ||
    !Number.isSafeInteger(month.expected) ||
    month.expected < 0
  ) return { word: 'unverified', kind: 'absent' };

  if (month.expected === 0) return { word: 'nothing owed', kind: 'quiet' };
  return { word: 'whole', kind: 'ok' };
}

/**
 * The range has no aggregate damage counters, so inspect every month too.
 * Calendar staleness alone does not invalidate a fully covered historical span.
 * @param {{
 *   expected: number,
 *   lost_minutes: number,
 *   unmeasured_minutes: number,
 *   months: number,
 *   months_absent: number,
 *   truncated: boolean,
 *   calendar?: { covers_span: boolean, stale?: boolean, unreadable?: unknown },
 *   month: MonthEvidence[]
 * }} answer
 */
export function isWholeAudit(answer) {
  return (
    Number.isSafeInteger(answer.expected) &&
    answer.expected > 0 &&
    answer.lost_minutes === 0 &&
    answer.unmeasured_minutes === 0 &&
    answer.months_absent === 0 &&
    answer.truncated === false &&
    answer.calendar?.covers_span === true &&
    // A PEER THAT COULD NOT BE READ IS NOT A PEER THAT AGREED (CE-80, D-1787).
    // `calendar.unreadable` names every peer census or bar file the server
    // could not open; such a peer did not vote, so the calendar lacks what it
    // would have proved. An absent list is unknown, never empty.
    peersAllRead(answer.calendar) &&
    Array.isArray(answer.month) &&
    answer.month.length > 0 &&
    answer.month.length === answer.months &&
    answer.month.every((month) => {
      const { kind } = monthVerdict(month);
      return kind === 'ok' || kind === 'quiet';
    })
  );
}

/**
 * Whether every peer the calendar could have drawn on was read. True only for
 * an explicit empty `unreadable` list: a missing or malformed field is an
 * answer that did not say, and is not taken as "none".
 * @param {{ unreadable?: unknown } | undefined | null} calendar
 * @returns {boolean}
 */
export function peersAllRead(calendar) {
  return Array.isArray(calendar?.unreadable) && calendar.unreadable.length === 0;
}

/**
 * The peers named unreadable, as strings, or null when the field is absent or
 * malformed (which the page must say rather than show as none).
 * @param {{ unreadable?: unknown } | undefined | null} calendar
 * @returns {string[] | null}
 */
export function unreadablePeers(calendar) {
  const list = calendar?.unreadable;
  if (!Array.isArray(list) || !list.every((name) => typeof name === 'string')) return null;
  return list;
}

/**
 * The rung `/gaps.json` audits a stored series at: its own `1min` months when
 * the census holds any, else its `1s` months (D-3336 §4), else none.
 *
 * One second is FINER than a minute: the server folds it to minutes with
 * `pull::fold` and audits the seconds an index owes. GDFL holds nothing but
 * `1s` (D-2802), so a page that offered only `1min` could audit none of its
 * series (DB-073). A daily or other coarser rung cannot prove its source
 * minutes and is never offered.
 *
 * @param {Iterable<{ instrument: string, timeframe: string }>} rows the census rows
 * @param {string} instrument the census key asked for
 * @returns {'1min' | '1s' | null}
 */
export function auditRung(rows, instrument) {
  let second = false;
  for (const row of rows) {
    if (row.instrument !== instrument) continue;
    if (row.timeframe === '1min') return '1min';
    if (row.timeframe === '1s') second = true;
  }
  return second ? '1s' : null;
}

/**
 * How a month's gap answer was read (R3-06, D-4980): off the month's kept
 * summary, with no bar record read, or from its bars because no summary was
 * usable — and then why, and how many records it cost. A slow answer is
 * named, never silent; a server that does not say is "not stated", never
 * read as either.
 *
 * @param {{ summary?: string, summary_fallback?: string | null, bar_records_read?: number }} month
 * @returns {{ kind: 'kept' | 'built' | 'none' | 'unknown', word: string, why: string }}
 */
export function readFrom(month) {
  const records = typeof month.bar_records_read === 'number' ? month.bar_records_read : null;
  switch (month.summary) {
    case 'kept':
      return {
        kind: 'kept',
        word: 'summary',
        why:
          'read off the month summary; no bar record read, so a record damaged after the summary was written is not checked here'
      };
    case 'built':
      return {
        kind: 'built',
        word: records == null ? 'bars' : `bars · ${records.toLocaleString('en-IN')} records`,
        why: month.summary_fallback || 'no usable month summary; the server gave no reason'
      };
    case 'absent_file':
      return { kind: 'none', word: '—', why: 'the month file is absent; nothing was read' };
    default:
      return { kind: 'unknown', word: 'not stated', why: 'this server does not say how the month was read' };
  }
}
