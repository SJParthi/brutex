/** Display helpers for rows already accepted by the shared receipt validators.
 * These explain saved evidence; they never calculate a new research decision.
 */
const U64 = (1n << 64n) - 1n, I64 = (1n << 63n) - 1n;
const MISSING = 'Not recorded', INVALID = 'Invalid saved value';
/** @param {unknown} value @param {boolean} [signed] */
function integer(value, signed = false) {
  if (typeof value !== 'string' || value.length > 20 ||
      !(signed ? /^(0|-?[1-9]\d*)$/ : /^(0|[1-9]\d*)$/).test(value)) return null;
  const n = BigInt(value);
  return n >= (signed ? -I64 - 1n : 0n) && n <= (signed ? I64 : U64) ? n : null;
}
/** @param {unknown} value */
const unavailable = value => value === null || value === undefined ? MISSING : INVALID;
/** @param {unknown} value */
export function formatSavedCount(value) {
  const n = integer(value);
  return n === null ? unavailable(value) : n.toLocaleString('en-IN');
}
/** Exact integer paisa to rupees; no Number conversion or rounding. @param {unknown} value */
export function formatPaisa(value) {
  const n = integer(value, true);
  if (n === null) return unavailable(value);
  const magnitude = n < 0n ? -n : n;
  return `${n < 0n ? '−' : ''}₹${(magnitude / 100n).toLocaleString('en-IN')}.${String(magnitude % 100n).padStart(2, '0')}`;
}
/** @param {bigint} n @param {bigint} scale @param {number} places */
function decimal(n, scale, places) {
  const magnitude = n < 0n ? -n : n;
  const fraction = String(magnitude % scale).padStart(places, '0').replace(/0+$/, '');
  return `${n < 0n ? '−' : ''}${magnitude / scale}${fraction ? '.' + fraction : ''}`;
}
/** A ppm is one millionth: 10,000 ppm = 1%. @param {unknown} value */
export function formatPpmPercent(value) {
  const n = integer(value, true);
  return n === null ? unavailable(value) : decimal(n, 10_000n, 4) + '%';
}
/** @param {unknown} value */
export function formatPpmRatio(value) {
  const n = integer(value, true);
  return n === null ? unavailable(value) : decimal(n, 1_000_000n, 6) + '×';
}

/** @param {unknown} status */
export function decisionExplanation(status) {
  switch (status) {
    case 'admitted': return { label: 'Passed saved research checks', detail: 'Every required check passed for this saved setting. This is not permission to trade or a profit guarantee.' };
    case 'rejected': return { label: 'Measured, but failed checks', detail: 'The required evidence was measured, and at least one saved policy limit was not met.' };
    case 'refused': return { label: 'Required evidence refused', detail: 'At least one required input was refused upstream. Simulated trades may still be recorded, but they cannot establish a pass.' };
    case 'unmeasured': return { label: 'Required evidence missing', detail: 'At least one required input was not measured. Missing evidence is not a zero result.' };
    default: return { label: 'Decision not recorded', detail: 'No recognized saved decision is available.' };
  }
}

export const SETTING_EXPLANATION = 'One tested combination of instrument, entry rule, direction and exit rules. Its number is an address in the saved results, not its rank.';
export const RETURN_EXPLANATION = 'Sum of simulated returns for one unit per trade, before costs. It is not an account balance, portfolio return or return percentage. The same market opportunity can appear in many settings.';
export const VWAP_EXPLANATION = 'VWAP is the session’s volume-weighted average of each bar’s (high + low + close) / 3, including the current bar. It resets each IST session and needs two positive-volume bars. This is an OHLCV approximation, not a reconstruction of individual trades.';
export const UNKNOWN_EXPLANATION = 'Unknown means the rule could not be evaluated. NOT keeps Unknown as Unknown; it does not create an entry. Spot-index VWAP remains unavailable; cash-stock VWAP uses that stock’s own observed volume.';

/** @param {unknown} rung */
export function timeframeExplanation(rung) {
  const match = typeof rung === 'string' ? /^(1|2|3|5|10|15|30|60)min$/.exec(rung) : null;
  return match ? `${match[1]}-minute candles for the entry rule. Saved execution uses separate 1-minute bars.` : 'Signal timeframe not recorded.';
}

/**
 * Names come from Rust vocabulary metadata supplied by the caller, never a
 * copied bit-number table. Only the two named VWAP rules get special prose.
 * @param {unknown} expression
 * @param {{i?:number|string, index?:number|string, bit?:number|string, name:string}[]} [vocabulary]
 */
export function signalExplanation(expression, vocabulary = []) {
  if (typeof expression !== 'string' || !expression) return MISSING;
  const names = new Map(vocabulary.map(row => [String(row.i ?? row.index ?? row.bit), row.name]));
  const simple = /^\s*(!)?\s*\(?\s*(\d+)\s*\)?\s*$/.exec(expression);
  if (simple) {
    const name = names.get(simple[2]);
    if (name === 'close_above_vwap') return simple[1] ? 'Close at or below the session VWAP, when VWAP is available' : 'Close above the session VWAP';
    if (name === 'close_below_vwap') return simple[1] ? 'Close at or above the session VWAP, when VWAP is available' : 'Close below the session VWAP';
  }
  return expression.replace(/\d+|[!&|]/g, token => {
    if (token === '!') return 'NOT ';
    if (token === '&') return ' AND ';
    if (token === '|') return ' OR ';
    return names.get(token)?.replaceAll('_', ' ') ?? `condition ${token} (name not loaded)`;
  }).replace(/\s+/g, ' ').trim();
}

/** @param {any} coordinate @param {any} grid */
export function exitExplanation(coordinate, grid) {
  const levels = coordinate?.levels;
  /** @param {string} name */
  const level = name => levels && Object.hasOwn(levels, name)
    ? levels[name] === null ? 'Off' : formatPpmPercent(levels[name]) : MISSING;
  const horizon = integer(grid?.horizon_bars), seconds = integer(grid?.execution_seconds);
  const duration = horizon !== null && horizon > 0n && seconds !== null && seconds > 0n ? horizon * seconds : null;
  const time = duration === null ? MISSING : duration % 60n === 0n ? `${duration / 60n} minutes` : `${duration} seconds`;
  const arm = level('ttp_arm_ppm'), trail = level('ttp_trail_ppm');
  return [
    { label: 'Fixed stop loss', value: level('stop_ppm'), detail: 'Loss-distance threshold from entry.' },
    { label: 'Fixed profit target', value: level('target_ppm'), detail: 'Profit-distance threshold from entry. Off does not mean a zero-distance target.' },
    { label: 'Trailing stop', value: level('tsl_ppm'), detail: 'Active from entry; follows the best price and exits after the saved give-back distance.' },
    { label: 'Trailing profit', value: arm === 'Off' && trail === 'Off' ? 'Off' : arm === MISSING || trail === MISSING ? MISSING : `Activate at ${arm}; trail by ${trail}`, detail: 'Starts only after the activation level is reached, then follows the best price.' },
    { label: 'Time exit', value: time, detail: 'Measured from the execution entry timestamp, or earlier at the same-session 15:10 IST deadline. Bar-open timestamps must be before 15:10.' }
  ];
}

/** @param {any} value */
function periodFacts(value) {
  const cell = value?.cell;
  return {
    trades: formatSavedCount(cell ? cell.trades : value?.trades),
    wins: formatSavedCount(cell ? cell.wins : value?.wins),
    pessimistic: formatPaisa(cell ? cell.pessimistic : value?.return_paisa),
    optimistic: formatPaisa(cell?.optimistic),
    supportSessions: formatSavedCount(value?.support_sessions),
    hits: formatSavedCount(value?.truth?.hits), unknown: formatSavedCount(value?.truth?.unknown)
  };
}
/** Original/later coordinate rows, or training statistics plus an absent later row.
 * @param {any} training @param {any} later */
export function periodComparison(training, later) {
  return { training: periodFacts(training), later: periodFacts(later), note: RETURN_EXPLANATION };
}

// Each row maps a stable API check name to its observed field and policy field.
// Threshold numbers are always taken from the validated saved policy.
/** @type {Record<string, [string, string, string | null, 'min'|'max'|'require'|'complete']>} */
const CHECKS = {
  support: ['Matching signal bars', 'support_hits', 'min_support_hits', 'min'],
  session_independence: ['Separate sessions with signals', 'independent_sessions', 'min_independent_sessions', 'min'],
  trades: ['Number of trades', 'trades', 'min_trades', 'min'],
  max_mae: ['Largest adverse move during a trade', 'max_mae_paisa', 'max_mae_paisa', 'max'],
  worst_reward_risk: ['Worst reward-to-risk ratio', 'worst_reward_risk_ppm', 'min_worst_reward_risk_ppm', 'min'],
  win_rate: ['Winning-trade rate', 'win_rate_ppm', 'min_win_rate_ppm', 'min'],
  wilson_win_rate: ['Conservative lower bound on win rate', 'wilson_win_rate_ppm', 'min_wilson_win_rate_ppm', 'min'],
  return_drawdown: ['Return relative to drawdown', 'return_drawdown_ppm', 'min_return_drawdown_ppm', 'min'],
  weakest_period: ['Return in the weakest recorded period', 'weakest_period_return_paisa', 'min_weakest_period_return_paisa', 'min'],
  pbo: ['Estimated backtest overfitting', 'pbo_ppm', 'max_pbo_ppm', 'max'],
  fwer: ['Search-adjusted statistical p-value', 'fwer_p_value_ppm', 'max_fwer_p_value_ppm', 'max'],
  spa: ['Superior-predictive-ability p-value', 'spa_p_value_ppm', 'max_spa_p_value_ppm', 'max'],
  decided_folds: ['Completed later validation windows', 'decided_folds', 'min_decided_folds', 'min'],
  ambiguous_fills: ['Trades with ambiguous candle ordering', 'ambiguous_fill_rate_ppm', 'max_ambiguous_fill_rate_ppm', 'max'],
  gap_affected: ['Trades affected by price gaps', 'gap_affected_rate_ppm', 'max_gap_affected_rate_ppm', 'max'],
  session_concentration: ['Concentration in one trading session', 'session_concentration_ppm', 'max_session_concentration_ppm', 'max'],
  largest_trade_profit_share: ['Profit concentrated in one trade', 'largest_trade_profit_share_ppm', 'max_largest_trade_profit_share_ppm', 'max'],
  execution_completeness: ['Required execution and exit evidence', 'execution_complete', null, 'complete'],
  data_completeness: ['Required source data', 'data_complete', null, 'complete'],
  calendar_completeness: ['Required trading-calendar evidence', 'calendar_complete', null, 'complete'],
  population_completeness: ['Complete declared set of tested settings', 'population_complete', null, 'complete'],
  drawdown: ['Largest fall from a cumulative-return peak', 'drawdown_paisa', 'max_drawdown_paisa', 'max'],
  worst_trade_loss: ['Largest single-trade loss', 'worst_trade_loss_paisa', 'max_worst_trade_loss_paisa', 'max'],
  losing_trade_rate: ['Losing-trade rate', 'losing_trade_rate_ppm', 'max_losing_trade_rate_ppm', 'max'],
  losing_trades: ['Number of losing trades', 'losing_trades', 'max_losing_trades', 'max'],
  pessimistic_profit: ['Pessimistic total return', 'pessimistic_profit_paisa', 'min_pessimistic_profit_paisa', 'min'],
  winning_trades: ['Number of winning trades', 'winning_trades', 'min_winning_trades', 'min'],
  average_win: ['Average winning-trade return', 'average_win_paisa', 'min_average_win_paisa', 'min'],
  average_loss: ['Average losing-trade loss', 'average_loss_paisa', 'max_average_loss_paisa', 'max'],
  profit_factor: ['Gross wins divided by gross losses', 'profit_factor_ppm', 'min_profit_factor_ppm', 'min'],
  consecutive_losing_streak: ['Longest losing-trade streak', 'consecutive_losing_streak', 'max_consecutive_losing_streak', 'max'],
  consecutive_winning_streak: ['Longest winning-trade streak', 'consecutive_winning_streak', 'min_consecutive_winning_streak', 'min'],
  bootstrap_draws: ['Statistical resampling draws', 'bootstrap_draws', 'min_bootstrap_draws', 'min'],
  bootstrap_strategies: ['Settings included in the statistical comparison', 'bootstrap_strategies', 'min_bootstrap_strategies', 'min'],
  bootstrap_periods: ['Aligned sessions in the statistical comparison', 'bootstrap_periods', 'min_bootstrap_periods', 'min'],
  pbo_contributing_folds: ['Usable overfitting-validation splits', 'pbo_contributing_folds', 'min_pbo_contributing_folds', 'min'],
  pbo_unrankable_folds: ['Unrankable overfitting-validation splits', 'pbo_unrankable_folds', 'max_pbo_unrankable_folds', 'max'],
  profitable_oos_folds: ['Later windows with positive pessimistic return', 'profitable_oos_folds', 'min_profitable_oos_folds', 'min'],
  oos_pessimistic_return: ['Validated later-period pessimistic return', 'oos_pessimistic_return_paisa', 'min_oos_pessimistic_return_paisa', 'min'],
  white_reality_p_value: ['White Reality Check p-value', 'white_reality_p_value_ppm', 'max_white_reality_p_value_ppm', 'max'],
  white_reality_decision: ['White test found evidence beyond its null hypothesis', 'white_reality_decision', 'require_white_reality_rejection', 'require'],
  romano_wolf_p_value: ['Romano–Wolf adjusted p-value', 'romano_wolf_p_value_ppm', 'max_romano_wolf_p_value_ppm', 'max'],
  romano_wolf_decision: ['Romano–Wolf test found evidence beyond its null hypothesis', 'romano_wolf_decision', 'require_romano_wolf_rejection', 'require'],
  full_precision_statistics_completeness: ['Complete full-precision statistics', 'full_precision_statistics_complete', null, 'complete']
};
const RATIOS = new Set(['worst_reward_risk_ppm', 'return_drawdown_ppm', 'profit_factor_ppm']);
/** @param {string} field @param {unknown} value */
function units(field, value) {
  return field.endsWith('_paisa') ? formatPaisa(value) : RATIOS.has(field) ? formatPpmRatio(value)
    : field.endsWith('_ppm') ? formatPpmPercent(value) : formatSavedCount(value);
}
/**
 * Caller supplies the policy only after receipt validation. Incomplete policy
 * shapes are still rejected here; there are no policy defaults in this module.
 * @param {any} check @param {any[]} values @param {any} [validatedPolicy]
 */
export function checkExplanation(check, values = [], validatedPolicy = null) {
  const spec = CHECKS[check?.name];
  const state = check?.state;
  const outcome = ({ passed: 'Passed', failed: 'Failed measured limit', refused: 'Evidence refused', unmeasured: 'Not measured' })[/** @type {'passed'|'failed'|'refused'|'unmeasured'} */ (state)] ?? 'Outcome not recorded';
  if (!spec) return { label: typeof check?.name === 'string' ? check.name.replaceAll('_', ' ') : 'Unknown check', outcome, observed: MISSING, required: MISSING, detail: 'No supported explanation is available for this saved check.' };
  const [label, field, policyName, comparison] = spec;
  const matches = values.filter(value => value?.name === field), value = matches.length === 1 ? matches[0] : null;
  const observed = value?.state === 'measured' ? units(field, value.value) : ({
    complete: 'Complete', incomplete: 'Incomplete', refused: 'Evidence refused', unmeasured: 'Not measured',
    'rejected-null': 'Evidence found beyond the null hypothesis', 'did-not-reject': 'Required statistical evidence was not found'
  })[/** @type {'complete'|'incomplete'|'refused'|'unmeasured'|'rejected-null'|'did-not-reject'} */(value?.state)] ?? MISSING;
  const policy = validatedPolicy?.values;
  const policyComplete = /^[0-9a-f]{64}$/.test(validatedPolicy?.digest ?? '') && Array.isArray(policy) && policy.length === 39 &&
    new Set(policy.map(row => row?.name)).size === 39 && policy.every(row => typeof row?.name === 'string' &&
      (typeof row.value === 'boolean' || integer(row.value, true) !== null || integer(row.value) !== null));
  const threshold = policyComplete && policyName ? policy.find(row => row.name === policyName) : null;
  const required = comparison === 'complete' ? 'Complete evidence' : !threshold ? 'Policy limit not loaded'
    : comparison === 'require' ? threshold.value === true ? 'Statistical null rejection required' : threshold.value === false ? 'Null rejection not required by this saved policy' : INVALID
    : `${comparison === 'min' ? 'At least' : 'At most'} ${units(field, threshold.value)}`;
  const detail = state === 'refused' ? 'The required input was refused upstream. Read the saved execution refusal before interpreting its trade observations.'
    : state === 'unmeasured' ? 'This required measurement is absent. Other saved trade observations do not fill this gap.'
    : ['fwer', 'spa', 'white_reality_p_value', 'romano_wolf_p_value'].includes(check.name) ? 'A statistical test measure; this is not the probability that a trade will win.'
    : 'The outcome is the saved verdict; this explanation does not recalculate or relax it.';
  return { label, outcome, observed, required, detail };
}

/** Explain the displayed later qualification without losing either period's
 * upstream execution refusal. Saved check states retain their priority over
 * measured failures; no new verdict is inferred from trade counts or returns.
 * @param {any} comparison @param {any} fact @param {any} [validatedPolicy]
 */
export function resultReason(comparison, fact, validatedPolicy = null) {
  /** @param {any} period @returns {string[]} */
  const refusals = period => Array.isArray(period?.execution_refusals) &&
    period.execution_refusals.every((/** @type {any} */ value) => typeof value === 'string' && value.trim().length > 0)
    ? period.execution_refusals : [];
  const training = refusals(fact?.training), later = refusals(fact?.later);
  if (training.length && training.length === later.length && training.every(value => later.includes(value))) {
    return 'Training and later: ' + training.join('; ');
  }
  const periods = [];
  if (training.length) periods.push('Training: ' + training.join('; '));
  if (later.length) periods.push('Later: ' + later.join('; '));
  if (periods.length) return periods.join('. ');

  const priority = ['refused', 'unmeasured', 'failed'];
  const checks = (Array.isArray(comparison?.checks) ? comparison.checks : [])
    .filter((/** @type {any} */ check) => priority.includes(check?.state))
    .sort((/** @type {any} */ a, /** @type {any} */ b) => priority.indexOf(a.state) - priority.indexOf(b.state));
  if (!checks.length) return decisionExplanation(comparison?.status).detail;
  const values = Array.isArray(comparison?.values) ? comparison.values : [];
  return checks.slice(0, 2).map((/** @type {any} */ check) => {
    const explained = checkExplanation(check, values, validatedPolicy);
    return `${explained.outcome}: ${explained.label}`;
  }).join('; ') + (checks.length > 2 ? `; ${checks.length - 2} more ${checks.length === 3 ? 'check' : 'checks'}` : '');
}
