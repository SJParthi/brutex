/**
 * Completeness requires usable evidence as well as zero measured absences.
 * The optional fields keep older minute-audit responses readable.
 *
 * @typedef {{
 *   expected: number,
 *   lost_minutes: number,
 *   unmeasured_minutes: number,
 *   truncated: boolean,
 *   unreadable_records: number,
 *   invalid_timestamps?: number,
 *   absent_file: string | null,
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
    month.unreadable_records !== 0 ||
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
 *   calendar?: { covers_span: boolean, stale?: boolean },
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
    Array.isArray(answer.month) &&
    answer.month.length > 0 &&
    answer.month.length === answer.months &&
    answer.month.every((month) => {
      const { kind } = monthVerdict(month);
      return kind === 'ok' || kind === 'quiet';
    })
  );
}
