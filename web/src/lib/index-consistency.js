import { ask } from './ask.js';
import { detailRefusal } from './detail-refusal.js';
import { createPageRequests } from './page-requests.js';

const U64 = (1n << 64n) - 1n, I64 = (1n << 63n) - 1n;
const INDEXES = ['NSE-NIFTY', 'NSE-BANKNIFTY'];
const STATES = ['passed', 'failed', 'unmeasured', 'refused', 'not_applicable', 'not_assessed'];
const INSTITUTIONAL = ['admitted', 'rejected', 'unmeasured', 'refused'];
const SUMMARY = ['calendar_days', 'eligible_days', 'observed_days', 'winning_days', 'losing_days',
  'zero_days', 'no_trade_days', 'missing_days', 'unmeasured_days', 'excluded_days', 'closed_days',
  'weekend_sessions', 'trades', 'pessimistic_paisa', 'longest_losing_streak', 'complete_weeks',
  'passing_weeks', 'failing_weeks', 'short_weeks', 'partial_weeks', 'unmeasured_weeks'];
const WEEK = ['index', 'monday', 'kind', 'state', 'weekday_days', 'eligible_days', 'observed_days',
  'winning_days', 'losing_days', 'zero_days', 'no_trade_days', 'missing_days', 'unmeasured_days',
  'excluded_days', 'closed_weekdays', 'weekend_sessions', 'trades', 'pessimistic_paisa'];
// These stable native reason labels are display text, not a second evaluator.
const REASONS = [
  ['winning_day_ratio_below_policy_minimum', 'Fewer winning days than the policy requires.'],
  ['complete_week_below_policy_minimum_wins', 'A complete five-session week had fewer winning days than the policy requires.'],
  ['complete_week_above_policy_maximum_losses', 'A complete five-session week had more losing days than the policy allows.'],
  ['losing_day_streak_above_policy_maximum', 'More losing days occurred in a row than the policy allows, without a winning day resetting the streak.'],
  ['no_complete_five_session_week', 'No complete five-session week was measured.'],
  ['no_eligible_session', 'No eligible trading day was available to measure.'],
  ['missing_expected_session', 'An expected trading day is missing its saved observation.'],
  ['unmeasured_calendar', 'The saved calendar cannot establish every required day.'],
  ['invalid_requested_span', 'The requested date range is invalid.'],
  ['invalid_session_order_or_span', 'Saved days repeat, are out of order, or fall outside the requested range.'],
  ['session_on_closed_or_excluded_day', 'An observation exists on a closed or excluded day.'],
  ['invalid_session_count_or_return', 'A saved day has inconsistent trade counts or profit and loss.'],
  ['exact_arithmetic_overflow', 'The exact trade count or money total exceeded its supported range.'],
  ['week_output_allocation_refused', 'The weekly evidence could not be retained within available resources.']
];
/** @param {any} value */
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
/** @param {any} value */
const uint = value => typeof value === 'string' && value.length <= 20 && value.trim() === value && /^(0|[1-9][0-9]*)$/.test(value) && BigInt(value) <= U64;
/** @param {any} value */
const sint = value => typeof value === 'string' && value.length <= 20 && value.trim() === value && /^(0|-?[1-9][0-9]*)$/.test(value) && BigInt(value) >= -I64 - 1n && BigInt(value) <= I64;
/** @param {any} value */
const hex = value => typeof value === 'string' && value.length === 64 && /^[0-9a-f]{64}$/.test(value) && value !== '0'.repeat(64);
/** @param {any} value @param {string[]} names */
const keys = (value, names) => object(value) && Object.keys(value).length === names.length && names.every(name => Object.hasOwn(value, name));
/** @param {any} value */
const link = value => keys(value, ['identity', 'completion']) && hex(value.identity) && hex(value.completion);
/** @param {any} left @param {any} right */
const sameLink = (left, right) => link(left) && link(right) && left.identity === right.identity && left.completion === right.completion;
/** @param {any} row @param {string[]} names */
const sum = (row, names) => names.reduce((total, name) => total + BigInt(row[name]), 0n);

/** The approved fixed policy is reported by the native server. An absent
 * descriptor is not evidence that an older server applies these checks.
 * @param {any} value */
export function validateIndexConsistencyPolicy(value) {
  const fields = ['schema_version', 'policy_digest', 'instruments', 'minimum_winning_day_numerator',
    'minimum_winning_day_denominator', 'minimum_week_winning_days', 'maximum_week_losing_days',
    'maximum_losing_day_streak', 'pnl_basis', 'zero_days_reset_streak', 'costs_included', 'evaluated_scope'];
  if (!keys(value, fields) || value.schema_version !== 1 || !hex(value.policy_digest) ||
      !Array.isArray(value.instruments) || value.instruments.length !== 2 || !INDEXES.every((name, index) => value.instruments[index] === name) ||
      // THRESHOLDS ARE VALIDATED AS SHAPE, NEVER AS VALUES.
      //
      // These five read '3','5','3','2','2' — V1's numbers, hardcoded here. The
      // server now serves V3 on this page, so this function threw on every
      // load, IndexStopLaunch caught it into `phase:'failed'`, and the operator
      // could not launch a sweep at all. Nothing caught that: no Rust gate reads
      // `web/`, and `cargo test -p cli --lib` compiles neither `api` nor this.
      //
      // Re-asserting the numbers here is the same defect `CLAUDE.md` §5 names
      // for the condition table: two copies of one fact, correct the day it is
      // written and silently wrong the first time the other copy moves. The
      // `policy_digest` above already authenticates the exact policy — it is
      // `blake3` over the version word and all five thresholds — so a changed
      // threshold is a changed digest and there is nothing left for a literal
      // to add. What the browser must still refuse is a MALFORMED field.
      !uint(value.minimum_winning_day_numerator) || !uint(value.minimum_winning_day_denominator) ||
      !uint(value.minimum_week_winning_days) || !uint(value.maximum_week_losing_days) ||
      !uint(value.maximum_losing_day_streak) ||
      value.pnl_basis !== 'pessimistic_gross_paisa' || value.zero_days_reset_streak !== false || value.costs_included !== false ||
      value.evaluated_scope !== 'training_and_later_with_calendar_gap_check') {
    throw new Error('The reported index day/week policy does not match the approved rule.');
  }
  return value;
}

/** @param {any} summary */
function validateSummary(summary) {
  if (!keys(summary, SUMMARY) || !SUMMARY.every(name => (name === 'pessimistic_paisa' ? sint : uint)(summary[name]))) {
    throw new Error('Index consistency counters must retain exact integer counts and paisa.');
  }
  if (sum(summary, ['winning_days', 'losing_days', 'zero_days', 'no_trade_days']) !== BigInt(summary.observed_days) ||
      sum(summary, ['observed_days', 'missing_days']) !== BigInt(summary.eligible_days) ||
      sum(summary, ['eligible_days', 'unmeasured_days', 'excluded_days', 'closed_days']) !== BigInt(summary.calendar_days) ||
      BigInt(summary.weekend_sessions) > BigInt(summary.eligible_days) || BigInt(summary.longest_losing_streak) > BigInt(summary.losing_days) ||
      sum(summary, ['passing_weeks', 'failing_weeks']) > BigInt(summary.complete_weeks) ||
      BigInt(summary.trades) < BigInt(summary.observed_days) - BigInt(summary.no_trade_days)) {
    throw new Error('Index consistency day and week totals do not reconcile.');
  }
}

/** @param {any} period @param {boolean} applicable */
function validatePeriod(period, applicable) {
  if (!keys(period, ['state', 'first_day', 'last_day', 'reasons_bits', 'reasons', 'first_issue_day', 'days_count', 'weeks_count', 'summary']) ||
      !STATES.slice(0, -1).includes(period.state) || !sint(period.first_day) || !sint(period.last_day) ||
      !uint(period.reasons_bits) || BigInt(period.reasons_bits) >= (1n << BigInt(REASONS.length)) ||
      !(period.first_issue_day === null || sint(period.first_issue_day)) || !uint(period.days_count) || !uint(period.weeks_count)) {
    throw new Error('A saved consistency period has invalid dates, counts or reasons.');
  }
  const bits = BigInt(period.reasons_bits);
  const reasons = REASONS.filter((_, index) => bits & (1n << BigInt(index))).map(([name]) => name);
  if (!Array.isArray(period.reasons) || period.reasons.length !== reasons.length || !reasons.every((name, index) => period.reasons[index] === name)) {
    throw new Error('Index-policy reason labels do not match their saved reason bits.');
  }
  const refused = bits & (64n | 256n | 512n | 1024n | 2048n | 4096n | 8192n);
  const unmeasured = bits & (16n | 32n | 128n);
  const expectedState = !applicable ? 'not_applicable' : refused ? 'refused' : unmeasured ? 'unmeasured' : bits ? 'failed' : 'passed';
  if (period.state !== expectedState || applicable && !(bits & 256n) && BigInt(period.first_day) > BigInt(period.last_day)) {
    throw new Error('Missing, failed or inapplicable index evidence cannot be shown as passed.');
  }
  validateSummary(period.summary);
  const s = period.summary;
  if (sum(s, ['complete_weeks', 'short_weeks', 'partial_weeks', 'unmeasured_weeks']) !== BigInt(period.weeks_count) ||
      period.state === 'passed' && (s.eligible_days === '0' || s.complete_weeks === '0' || s.missing_days !== '0' || s.unmeasured_days !== '0' ||
        BigInt(s.winning_days) * 5n < BigInt(s.eligible_days) * 3n || BigInt(s.longest_losing_streak) > 2n || s.failing_weeks !== '0' ||
        s.passing_weeks !== s.complete_weeks || s.observed_days !== period.days_count)) {
    throw new Error('The saved index-policy verdict contradicts its measured totals.');
  }
}

/** Validate an additive saved receipt against its exact qualification, setting
 * and relevant institutional verdict. Missing historical fields stay unassessed.
 * @param {any} value
 * @param {{qualification:any,setting_index:string,instrument:string,institutional:string}} context */
export function validateIndexConsistency(value, context) {
  if (value === undefined) return null;
  if (!keys(value, ['schema_version', 'state', 'policy_digest', 'receipt', 'qualification', 'setting_index',
    'combined_qualifies', 'evaluation', 'days_count', 'weeks_count', 'evaluated_scope', 'periods']) || value.schema_version !== 1 || !STATES.includes(value.state) ||
    !sameLink(value.qualification, context.qualification) || !uint(value.setting_index) || value.setting_index !== context.setting_index ||
    !INSTITUTIONAL.includes(context.institutional) || !uint(value.days_count) || !uint(value.weeks_count)) {
    throw new Error('The index day/week receipt does not match this exact setting and qualification.');
  }
  if (value.state === 'not_assessed') {
    if (value.policy_digest !== null || value.receipt !== null || value.evaluation !== null || value.days_count !== '0' ||
        value.weeks_count !== '0' || value.combined_qualifies !== null || value.evaluated_scope !== null || value.periods !== null) throw new Error('Older evidence cannot acquire a new index-policy verdict.');
    return value;
  }
  const expectedCombined = context.institutional === 'admitted' && ['passed', 'not_applicable'].includes(value.state);
  const e = value.evaluation;
  if (!hex(value.policy_digest) || !link(value.receipt) || value.combined_qualifies !== expectedCombined ||
      !keys(e, ['instrument', 'first_day', 'last_day', 'reasons_bits', 'reasons', 'first_issue_day',
        'calendar_digest', 'sessions_digest', 'weeks_digest', 'summary']) || e.instrument !== context.instrument ||
      !['calendar_digest', 'sessions_digest', 'weeks_digest'].every(name => hex(e[name])) ||
      value.evaluated_scope !== 'training_and_later_with_calendar_gap_check' || !keys(value.periods, ['training', 'later', 'full'])) {
    throw new Error('Index-policy identity, source facts or combined qualification is inconsistent.');
  }
  const applicable = INDEXES.includes(e.instrument);
  for (const period of Object.values(value.periods)) validatePeriod(period, applicable);
  const periodStates = Object.values(value.periods).map((/** @type {any} */ p) => p.state);
  const expectedState = ['refused', 'unmeasured', 'failed', 'passed', 'not_applicable'].find(state => periodStates.includes(state));
  const full = value.periods.full;
  if (value.state !== expectedState || full.days_count !== value.days_count || full.weeks_count !== value.weeks_count ||
      BigInt(value.periods.training.days_count) + BigInt(value.periods.later.days_count) !== BigInt(full.days_count) ||
      !['first_day', 'last_day', 'reasons_bits', 'reasons', 'first_issue_day'].every(name => JSON.stringify(e[name]) === JSON.stringify(full[name])) ||
      !SUMMARY.every(name => e.summary?.[name] === full.summary[name])) {
    throw new Error('The full-span summary or combined period decision changed its saved scope.');
  }
  return value;
}

/** The search correction changes institutional admission, never the immutable
 * saved daily evidence. Compare decoded facts without depending on JSON key order.
 * @param {any} left @param {any} right @returns {boolean} */
function sameFacts(left, right) {
  if (left === right) return true;
  if (Array.isArray(left) && Array.isArray(right)) return left.length === right.length && left.every((value, index) => sameFacts(value, right[index]));
  return object(left) && object(right) && Object.keys(left).length === Object.keys(right).length &&
    Object.keys(left).every(key => Object.hasOwn(right, key) && sameFacts(left[key], right[key]));
}

/** @param {any} original @param {any} comparison @param {any} context @param {number} version */
export function validateSearchIndexConsistency(original, comparison, context, version) {
  const value = validateIndexConsistency(comparison, context);
  if (version === 4 ? !value || value.state === 'not_assessed' : value && value.state !== 'not_assessed') {
    throw new Error('This search version cannot acquire or omit the required saved index-policy assessment.');
  }
  if (original === undefined && comparison === undefined) return null;
  if (!object(original) || !object(comparison)) throw new Error('The search comparison lost its original index-policy evidence.');
  const { combined_qualifies: _originalVerdict, ...originalFacts } = original;
  const { combined_qualifies: _searchVerdict, ...comparisonFacts } = comparison;
  if (!sameFacts(originalFacts, comparisonFacts)) throw new Error('The search comparison changed the original saved day/week evidence.');
  return value;
}

/** @param {any} body @param {any} row @param {string} [institutional] */
export function indexConsistencyContext(body, row, institutional = row.status) {
  return { qualification: { identity: body.identity, completion: body.completion }, setting_index: row.index,
    instrument: row.statistics.source.instrument, institutional };
}

/** @param {string} state */
export function indexConsistencyLabel(state) {
  const labels = { passed: 'Passed', failed: 'Failed', unmeasured: 'Not enough evidence', refused: 'Evidence refused',
    not_applicable: 'Not applicable', not_assessed: 'Not assessed under this rule' };
  return labels[/** @type {keyof typeof labels} */ (state)] ?? 'Unavailable';
}
/** @param {any} value */
export function indexConsistencyReasons(value) {
  if (!value || value.state === 'not_assessed') return ['This older result has no saved assessment under the new index day/week rule.'];
  if (value.state === 'not_applicable') return ['This additional rule applies only to NIFTY and BANKNIFTY. The existing institutional result is unchanged.'];
  const reasons = new Set(Object.values(value.periods).flatMap((/** @type {any} */ period) => period.reasons));
  return [...reasons].map(name => REASONS.find(([key]) => key === name)?.[1] ?? String(name));
}

/** @param {any} value */
export function indexCombinedLabel(value) {
  if (!value || value.combined_qualifies === null) return 'Not assessed under both checks';
  if (value.state === 'not_applicable') return value.combined_qualifies ? 'Existing institutional checks passed; index rule does not apply' : 'Existing institutional checks did not pass; index rule does not apply';
  return value.combined_qualifies ? 'Both checks passed' : 'Does not qualify under both checks';
}

/** @param {any} value @param {string} kind @param {string} offset @param {number} limit @param {string} period */
function pageSelection(value, kind, offset, limit, period) {
  if (!value || !link(value.receipt) || !link(value.qualification) || !uint(value.setting_index) ||
      !['index-days', 'index-weeks'].includes(kind) || !['training', 'later', 'full'].includes(period) || !object(value.periods?.[period]) ||
      !uint(offset) || !Number.isInteger(limit) || limit < 1 || limit > 256) {
    throw new Error('Choose an exact saved index assessment and a bounded day or week page.');
  }
  return { identity: value.qualification.identity, completion: value.qualification.completion,
    setting: value.setting_index, kind, offset, limit, period };
}

/** @param {any} row */
function validateWeek(row) {
  if (!keys(row, WEEK) || !WEEK.filter(name => !['monday', 'pessimistic_paisa', 'kind', 'state'].includes(name)).every(name => uint(row[name])) ||
      !sint(row.monday) || !sint(row.pessimistic_paisa) || BigInt(row.monday) < -3n || (BigInt(row.monday) + 3n) % 7n !== 0n ||
      BigInt(row.weekday_days) > 5n || BigInt(row.weekend_sessions) > 2n ||
      sum(row, ['winning_days', 'losing_days', 'zero_days', 'no_trade_days']) !== BigInt(row.observed_days) ||
      sum(row, ['observed_days', 'missing_days']) !== BigInt(row.eligible_days) ||
      sum(row, ['eligible_days', 'unmeasured_days', 'excluded_days', 'closed_weekdays']) !== BigInt(row.weekday_days) ||
      BigInt(row.trades) < BigInt(row.observed_days) - BigInt(row.no_trade_days)) throw new Error('A saved week has invalid or inconsistent day counts.');
  const kind = BigInt(row.weekday_days) < 5n ? 'partial' : BigInt(row.unmeasured_days) > 0n ? 'unmeasured' : BigInt(row.eligible_days) < 5n ? 'short' : 'complete';
  const state = BigInt(row.missing_days) > 0n ? 'refused' : BigInt(row.unmeasured_days) > 0n ? 'unmeasured' : kind !== 'complete' ? 'not_applicable' :
    BigInt(row.winning_days) >= 3n && BigInt(row.losing_days) <= 2n ? 'passed' : 'failed';
  if (row.kind !== kind || row.state !== state) throw new Error('A partial, missing or failing week cannot be relabelled as passed.');
}

/** One pinned, bounded page. The browser checks transport consistency; the
 * native receipt remains the authority for the saved calendar and evaluation.
 * @param {any} value @param {string} kind @param {string} [offset]
 * @param {number} [limit] @param {(url:string)=>Promise<any>} [request] @param {string} [period] @param {string} [model] */
export async function fetchIndexConsistencyPage(value, kind, offset = '0', limit = 16, request = ask, period = 'full', model = 'qualification') {
  if (!['qualification','index-stop-qualification'].includes(model)) throw new Error('Choose the exact saved qualification model.');
  const selected = pageSelection(value, kind, offset, limit, period);
  const query = new URLSearchParams({ ...selected, limit: String(limit) });
  const response = await request((model === 'qualification' ? '/boolean-qualification.json?' : '/index-stop-qualification.json?') + query);
  if (!response?.ok) throw new Error(await detailRefusal(response, 'The saved day/week page is unavailable.'));
  const body = await response.json();
  if (!keys(body, ['schema_version', 'status', 'model', 'kind', 'identity', 'completion', 'setting', 'receipt', 'period', 'total', 'offset', 'limit', 'rows', 'refusal']) ||
      body.schema_version !== 1 || body.status !== 'saved' || body.model !== model || body.refusal !== null ||
      !Object.entries(selected).every(([name, expected]) => body[name] === expected) || !sameLink(body.receipt, value.receipt) ||
      !uint(body.total) || body.total !== value.periods[period][kind === 'index-days' ? 'days_count' : 'weeks_count'] || !Array.isArray(body.rows)) {
    throw new Error('The saved day/week page does not match its exact setting and receipt.');
  }
  const start = BigInt(offset), total = BigInt(body.total), available = total - start;
  if (start > total || BigInt(body.rows.length) !== (available < BigInt(limit) ? available : BigInt(limit))) {
    throw new Error('The saved day/week page skips, truncates or repeats records.');
  }
  let previous = /** @type {bigint|null} */ (null);
  for (const [index, row] of body.rows.entries()) {
    if (!object(row) || row.index !== String(start + BigInt(index))) throw new Error('Saved day/week order changed.');
    if (kind === 'index-days') {
      if (!keys(row, ['index', 'day', 'pessimistic_paisa', 'trades']) || !sint(row.day) || !sint(row.pessimistic_paisa) || !uint(row.trades) ||
          row.trades === '0' && row.pessimistic_paisa !== '0') throw new Error('A saved day has inconsistent trades or money totals.');
    } else validateWeek(row);
    const day = BigInt(kind === 'index-days' ? row.day : row.monday);
    if (previous !== null && (day <= previous || kind === 'index-weeks' && day !== previous + 7n)) throw new Error('Saved days or weeks repeat or change order.');
    previous = day;
  }
  const next = start + BigInt(body.rows.length);
  return { ...body, next: next < total ? String(next) : null };
}

/** No polling or full history download: a selection owns one read plus one
 * replaceable pending read. Late successes/errors cannot replace newer data.
 * @param {(state:any)=>void} publish @param {(url:string, options?:RequestInit)=>Promise<any>} [request] @param {string|(()=>string)} [model] */
export function createIndexConsistencyPages(publish, request = ask, model = 'qualification') {
  const reads = createPageRequests();
  let disposed = false;
  return {
    /** @param {any} value @param {string} [kind] @param {string} [offset] @param {string} [period] */
    async open(value, kind = 'index-weeks', offset = '0', period = 'full') {
      if (disposed) return;
      reads.cancel();
      let captured;
      try { pageSelection(value, kind, offset, 16, period); captured = structuredClone(value); }
      catch (error) { publish({ phase: 'failed', body: null, kind, offset, period, why: error instanceof Error ? error.message : String(error) }); return; }
      const selectedModel = typeof model === 'function' ? model() : model;
      const base = { value: captured, kind, offset, period };
      publish({ ...base, phase: 'loading', body: null, why: '' });
      await reads.run(async ticket => {
        try {
          const body = await fetchIndexConsistencyPage(captured, kind, offset, 16,
            url => request(url, { cache: 'no-store', signal: ticket.signal }), period, selectedModel);
          if (ticket.current()) publish({ ...base, phase: 'ready', body, why: '' });
        } catch (error) { if (ticket.current()) publish({ ...base, phase: 'failed', body: null, why: error instanceof Error ? error.message : String(error) }); }
      });
    },
    close() { reads.cancel(); if (!disposed) publish({ phase: 'idle', body: null, why: '' }); },
    dispose() { disposed = true; reads.dispose(); }
  };
}
