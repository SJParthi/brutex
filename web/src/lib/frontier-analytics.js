import { decodeMaskWords } from './mask.js';

const ROW_INTEGERS = Object.freeze([
  'rank',
  'hits',
  'n',
  'mean_milli_paisa',
  't_milli',
  'edge_wins',
  'trades',
  'wins',
  'losses',
  'pessimistic',
  'worst_trade',
  'max_drawdown',
  'min_win',
  'win_rate_bp',
  'avg_win',
  'avg_loss',
  'gross_win',
  'gross_loss'
]);
// `payoff_bp` IS NULLABLE (p5num-5, D-2568). `/frontier.json` sends `null` for
// the payoff of a row that never gave anything back (`Edge::payoff_bp` returns
// `i64::MAX`, which no JavaScript number holds exactly, so a bare 2^63-1 used to
// refuse the WHOLE frontier here) and for a row of fewer than two observations
// (a refusal the store holds as 0, which read as a measured zero). Nothing in
// the browser ranks on it; it is validated, never computed with.
const NULLABLE_ROW_INTEGERS = Object.freeze([
  'payoff_bp',
  'reward_to_risk_bp',
  'return_over_drawdown'
]);
const RULE_INTEGERS = Object.freeze([
  'min_win_rate_bp',
  'min_rr_bp',
  'min_ret_over_dd_bp',
  'min_trades',
  'min_assurance_bp',
  'max_mae_ppm',
  'top'
]);
// Signed thresholds: `Rules::avg_payoff_holds` drops its rule at `<= 0`, so a
// negative floor is a legal stored value and is not refused here.
const SIGNED_RULE_INTEGERS = Object.freeze(['min_avg_rr_bp', 'min_fill_headroom_bp']);
// Names the server's `admission` lists may carry. Only the SHAPE is checked;
// which rules exist and which `all` conjoins is the server's answer, read
// from `cli::frontier::VERDICT_CHECKED` and `VERDICT_UNCHECKED` (D-1810).
const RULE_NAME = /^[a-z][a-z0-9_]*$/;
const I64_MIN = -(1n << 63n);
const I64_MAX = (1n << 63n) - 1n;

/** @param {bigint} value */
const clampI64 = (value) => (value < I64_MIN ? I64_MIN : value > I64_MAX ? I64_MAX : value);

/** @param {number} value */
const integer = (value) => BigInt(value);

/** @param {number} value @param {number|bigint} factor */
const saturatingMultiply = (value, factor) => clampI64(integer(value) * BigInt(factor));

/** @param {number} value */
const saturatingNegate = (value) => clampI64(-integer(value));

/** @param {bigint} numerator @param {number|bigint} denominator */
const divide = (numerator, denominator) => numerator / BigInt(denominator);

/**
 * Compare a browser-safe wire integer with the exact i64 Rust would derive.
 * `i64::MAX` is the API's explicit `null` for an undefined denominator.
 *
 * @param {number|null} wire
 * @param {bigint} expected
 */
const ratioMatches = (wire, expected) =>
  expected === I64_MAX ? wire === null : wire !== null && integer(wire) === expected;

/** @param {string} why */
const refused = (why) => ({
  ok: false,
  rows: [],
  rules: null,
  admission: null,
  admitted: 0,
  empty: false,
  why
});

/** @param {number} left @param {number} right */
const exactAdd = (left, right) => {
  const sum = left + right;
  return Number.isSafeInteger(sum) ? sum : null;
};

/**
 * The envelope's `admission` member, shape-checked: two arrays of distinct
 * rule names, disjoint, the checked one non-empty, neither naming a member
 * `meets` already uses. Returns the refusal reason as a string.
 *
 * @param {unknown} input
 * @returns {string | {checked: string[], unchecked: string[], members: Set<string>}}
 */
function checkAdmission(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return 'is not an object.';
  }
  const body = /** @type {Record<string, unknown>} */ (input);
  if (Object.keys(body).length !== 2 || !Array.isArray(body.checked) || !Array.isArray(body.unchecked)) {
    return 'must carry exactly the checked and unchecked arrays.';
  }
  const checked = /** @type {unknown[]} */ (body.checked);
  const unchecked = /** @type {unknown[]} */ (body.unchecked);
  if (checked.length === 0) return 'names no checked rule, so `all` would be vacuous.';
  /** @type {Set<string>} */
  const members = new Set(['all']);
  for (const name of checked) {
    if (typeof name !== 'string' || !RULE_NAME.test(name) || name.endsWith('_unchecked') || members.has(name)) {
      return `checked name ${JSON.stringify(name)} is malformed, reserved or repeated.`;
    }
    members.add(name);
  }
  for (const name of unchecked) {
    if (typeof name !== 'string' || !RULE_NAME.test(name) || members.has(name) || members.has(`${name}_unchecked`)) {
      return `unchecked name ${JSON.stringify(name)} is malformed, reserved or repeated.`;
    }
    members.add(`${name}_unchecked`);
  }
  return {
    checked: /** @type {string[]} */ (checked),
    unchecked: /** @type {string[]} */ (unchecked),
    members
  };
}

/**
 * Admit one complete `/frontier.json` payload before it reaches ranking.
 *
 * Rust's `u64` and `i64` are wider than JavaScript's exact integer range.
 * JSON parsing has already rounded an unsafe bare number by the time this runs,
 * so any non-safe integer refuses the whole answer. Mask words are strings and
 * cross the existing exact `BigInt` decoder. No valid-looking prefix survives.
 *
 * @param {unknown} input
 * @param {string|undefined} expectedIdentity
 * WHICH RULES A ROW MEETS IS THE SERVER'S ANSWER, NOT THIS FILE'S (D-1810).
 * `meets` is checked against the envelope's `admission` lists: every listed
 * rule present and boolean, every unanswered rule flagged `true`, no other
 * member, and `all` exactly the conjunction of the listed rules. No threshold
 * is compared here and no rule statistic is recomputed, so a rule added in
 * `cli` cannot make this verifier refuse a valid answer, and a rule this file
 * never heard of cannot be silently dropped from `all`.
 *
 * @returns {{ok: boolean, rows: any[], rules: any, admission: any, admitted: number, empty: boolean, why: string}}
 */
export function validateFrontierPayload(input, expectedIdentity = undefined) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return refused('/frontier.json did not return an object.');
  }
  const body = /** @type {Record<string, any>} */ (input);
  if (typeof body.identity !== 'string' || !/^[0-9a-f]{64}$/.test(body.identity)) {
    return refused('/frontier.json did not carry one canonical run identity.');
  }
  if (expectedIdentity !== undefined && body.identity !== expectedIdentity) {
    return refused(
      `/frontier.json answered for run ${body.identity}, not requested run ${String(expectedIdentity)}.`
    );
  }
  if (!Array.isArray(body.rows)) {
    return refused('/frontier.json `rows` is not an array.');
  }
  if (!Number.isSafeInteger(body.count) || body.count < 0) {
    return refused('/frontier.json `count` is not an exact non-negative integer.');
  }
  if (body.count !== body.rows.length) {
    return refused(
      `/frontier.json count says ${String(body.count)} but carries ${body.rows.length} rows.`
    );
  }

  const explicitAbsence =
    body.rows.length === 0 &&
    typeof body.refusal === 'string' &&
    body.refusal.length > 0 &&
    (body.rules === undefined || body.rules === null) &&
    (body.admitted === undefined || body.admitted === 0);
  if (body.refusal !== null && !explicitAbsence) {
    return refused('/frontier.json is partial or refused; no ranked prefix is published.');
  }
  if (explicitAbsence) {
    return {
      ok: true,
      rows: [],
      rules: null,
      admission: null,
      admitted: 0,
      empty: true,
      why: body.refusal
    };
  }

  if (!Number.isSafeInteger(body.admitted) || body.admitted < 0) {
    return refused('/frontier.json `admitted` is not an exact non-negative integer.');
  }
  if (body.admitted > body.count) {
    return refused('/frontier.json `admitted` exceeds `count`.');
  }
  if (body.rows.length === 0) {
    if (body.rules !== null || body.admitted !== 0) {
      return refused('/frontier.json empty rows require null rules and zero admissions.');
    }
    return { ok: true, rows: [], rules: null, admission: null, admitted: 0, empty: true, why: '' };
  }

  if (!body.rules || typeof body.rules !== 'object' || Array.isArray(body.rules)) {
    return refused('/frontier.json non-empty rows require their exact rules object.');
  }
  for (const field of RULE_INTEGERS) {
    if (!Number.isSafeInteger(body.rules[field]) || body.rules[field] < 0) {
      return refused(`/frontier.json rules.${field} is not an exact non-negative integer.`);
    }
  }
  for (const field of SIGNED_RULE_INTEGERS) {
    if (!Number.isSafeInteger(body.rules[field])) {
      return refused(`/frontier.json rules.${field} is not an exact integer.`);
    }
  }
  const admission = checkAdmission(body.admission);
  if (typeof admission === 'string') {
    return refused(`/frontier.json admission ${admission}`);
  }
  if (body.rules.top === 0) {
    return refused('/frontier.json rules.top cannot be zero for a non-empty frontier.');
  }
  if (body.count > body.rules.top) {
    return refused('/frontier.json carries more rows than rules.top permits.');
  }

  const seenRanks = new Set();
  let admitted = 0;
  for (let at = 0; at < body.rows.length; at += 1) {
    const row = body.rows[at];
    if (!row || typeof row !== 'object' || Array.isArray(row)) {
      return refused(`/frontier.json rows[${at}] is not an object.`);
    }
    for (const field of ROW_INTEGERS) {
      if (!Number.isSafeInteger(row[field])) {
        return refused(`/frontier.json rows[${at}].${field} is not an exact integer.`);
      }
    }
    for (const field of NULLABLE_ROW_INTEGERS) {
      if (row[field] !== null && !Number.isSafeInteger(row[field])) {
        return refused(`/frontier.json rows[${at}].${field} is neither null nor an exact integer.`);
      }
    }
    if (row.direction !== 'long' && row.direction !== 'short') {
      return refused(`/frontier.json rows[${at}].direction is not long or short.`);
    }
    const mask = decodeMaskWords(row.mask_words);
    if (!mask.ok) {
      return refused(`/frontier.json rows[${at}].mask_words is not canonical: ${mask.why}`);
    }
    if (typeof row.priced !== 'boolean') {
      return refused(`/frontier.json rows[${at}].priced is not boolean.`);
    }
    if (!row.meets || typeof row.meets !== 'object' || Array.isArray(row.meets)) {
      return refused(`/frontier.json rows[${at}].meets is not an object.`);
    }
    const keys = Object.keys(row.meets);
    if (
      keys.length !== admission.members.size ||
      keys.some((key) => !admission.members.has(key))
    ) {
      return refused(
        `/frontier.json rows[${at}].meets does not carry exactly the served admission members.`
      );
    }
    for (const key of keys) {
      if (typeof row.meets[key] !== 'boolean') {
        return refused(`/frontier.json rows[${at}].meets.${key} is not boolean.`);
      }
    }

    for (const field of [
      'hits',
      'n',
      'edge_wins',
      'trades',
      'wins',
      'losses',
      'max_drawdown',
      'min_win',
      'win_rate_bp'
    ]) {
      if (row[field] < 0) {
        return refused(`/frontier.json rows[${at}].${field} cannot be negative.`);
      }
    }
    if (row.rank !== at + 1 || seenRanks.has(row.rank)) {
      return refused(
        `/frontier.json row rank ${String(row.rank)} is duplicate or not the canonical wire rank ${String(at + 1)}.`
      );
    }
    seenRanks.add(row.rank);
    if (row.edge_wins > row.n || row.n > row.hits) {
      return refused(`/frontier.json rows[${at}] does not satisfy edge_wins <= n <= hits.`);
    }
    // One observation is not a distribution, so the payoff is a refusal and the
    // route sends `null` for it (p5num-5, D-2568). A number there would be the
    // refusal read back as a measurement.
    if (row.n < 2 && row.payoff_bp !== null) {
      return refused(`/frontier.json rows[${at}].payoff_bp must be null below two observations.`);
    }
    if (row.wins > row.trades || row.losses !== row.trades - row.wins) {
      return refused(`/frontier.json rows[${at}] does not satisfy wins + losses = trades.`);
    }
    if (row.win_rate_bp > 10_000) {
      return refused(`/frontier.json rows[${at}].win_rate_bp exceeds 10,000.`);
    }
    if (row.priced !== (row.trades > 0)) {
      return refused(`/frontier.json rows[${at}].priced disagrees with whether a cell traded.`);
    }
    if (
      row.gross_win < 0 ||
      row.gross_loss > 0 ||
      row.worst_trade > 0 ||
      row.min_win < 0 ||
      row.max_drawdown < 0
    ) {
      return refused(`/frontier.json rows[${at}] contradicts the signed cell-money domains.`);
    }
    if (
      (row.wins === 0 && (row.gross_win !== 0 || row.min_win !== 0)) ||
      (row.wins > 0 && (row.gross_win <= 0 || row.min_win <= 0)) ||
      (row.losses === 0 && (row.gross_loss !== 0 || row.worst_trade !== 0))
    ) {
      return refused(`/frontier.json rows[${at}] contradicts its win/loss extrema and gross totals.`);
    }
    const total = exactAdd(row.gross_win, row.gross_loss);
    if (total === null || total !== row.pessimistic) {
      return refused(`/frontier.json rows[${at}] gross results do not exactly sum to pessimistic.`);
    }

    const expectedWinRate =
      row.trades === 0 ? 0n : divide(saturatingMultiply(row.wins, 10_000), row.trades);
    const worstLoss = saturatingNegate(row.worst_trade);
    const expectedReward =
      worstLoss <= 0n ? I64_MAX : divide(saturatingMultiply(row.min_win, 100), worstLoss);
    const expectedReturn =
      row.pessimistic <= 0
        ? 0n
        : row.max_drawdown <= 0
          ? I64_MAX
          : divide(saturatingMultiply(row.pessimistic, 100), row.max_drawdown);
    const expectedAvgWin = row.wins === 0 ? 0n : integer(row.gross_win) / integer(row.wins);
    const expectedAvgLoss =
      row.losses === 0 ? 0n : integer(row.gross_loss) / integer(row.losses);
    if (
      integer(row.win_rate_bp) !== expectedWinRate ||
      !ratioMatches(row.reward_to_risk_bp, expectedReward) ||
      !ratioMatches(row.return_over_drawdown, expectedReturn) ||
      integer(row.avg_win) !== expectedAvgWin ||
      integer(row.avg_loss) !== expectedAvgLoss
    ) {
      return refused(`/frontier.json rows[${at}] carries derived cell figures that do not match its raw totals.`);
    }

    const conjunction = admission.checked.every((/** @type {string} */ name) => row.meets[name]);
    if (row.meets.all !== conjunction) {
      return refused(
        `/frontier.json rows[${at}].meets.all is not the conjunction of the served checked rules.`
      );
    }
    if (admission.unchecked.some((/** @type {string} */ name) => row.meets[`${name}_unchecked`] !== true)) {
      return refused(`/frontier.json rows[${at}] passes a rule the server says it cannot answer.`);
    }
    if (!row.priced && row.meets.all) {
      return refused(`/frontier.json rows[${at}] is admitted without ever being priced.`);
    }
    if (row.meets.all) admitted += 1;
  }

  if (admitted !== body.admitted) {
    return refused(
      `/frontier.json admitted says ${String(body.admitted)} but ${admitted} rows meet every checked rule.`
    );
  }
  return {
    ok: true,
    rows: body.rows,
    rules: body.rules,
    admission: { checked: admission.checked, unchecked: admission.unchecked },
    admitted,
    empty: false,
    why: ''
  };
}
