import { test } from 'node:test';
import assert from 'node:assert/strict';

import { validateFrontierPayload } from '../src/lib/frontier-analytics.js';

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
const RULE_INTEGERS = Object.freeze([
  'min_win_rate_bp',
  'min_rr_bp',
  'min_ret_over_dd_bp',
  'min_trades',
  'min_assurance_bp',
  'max_mae_ppm',
  'top'
]);

/** @param {number} rank @param {boolean} admitted @returns {any} */
const row = (rank, admitted) => ({
  rank,
  direction: rank === 1 ? 'long' : 'short',
  mask_words: [String(rank), '0', '0', '0', '0', '0'],
  hits: 6,
  n: 5,
  mean_milli_paisa: 12_500,
  t_milli: 2_000,
  payoff_bp: 150,
  edge_wins: 4,
  priced: true,
  trades: 4,
  wins: admitted ? 3 : 1,
  losses: admitted ? 1 : 3,
  pessimistic: admitted ? 200 : -200,
  worst_trade: admitted ? -50 : -100,
  max_drawdown: admitted ? 50 : 300,
  min_win: admitted ? 150 : 100,
  win_rate_bp: admitted ? 7_500 : 2_500,
  reward_to_risk_bp: admitted ? 300 : 100,
  return_over_drawdown: admitted ? 400 : 0,
  avg_win: 100,
  avg_loss: -100,
  gross_win: admitted ? 300 : 100,
  gross_loss: admitted ? -100 : -300,
  meets: {
    win_rate: admitted,
    reward_to_risk: admitted,
    return_over_drawdown: admitted,
    trades: true,
    assurance: admitted,
    all: admitted,
    stop_unchecked: true,
    protective_exits_unchecked: true
  }
});

/** @returns {any} */
const validPayload = () => ({
  identity: 'ab'.repeat(32),
  rows: [row(1, true), row(2, false)],
  count: 2,
  admitted: 1,
  rules: {
    min_win_rate_bp: 5_000,
    min_rr_bp: 125,
    min_ret_over_dd_bp: 400,
    min_trades: 4,
    min_assurance_bp: 3_000,
    max_mae_ppm: 0,
    top: 25
  },
  refusal: null
});

test('one complete exact frontier admits every row without copying or reranking it', () => {
  const payload = validPayload();
  const result = validateFrontierPayload(payload);
  assert.equal(result.ok, true, result.why);
  assert.equal(result.rows, payload.rows);
  assert.equal(result.rules, payload.rules);
  assert.equal(result.admitted, 1);
  assert.equal(result.empty, false);
});

test('every bare Rust integer field refuses the whole frontier once JavaScript cannot represent it exactly', () => {
  const unsafe = Number.MAX_SAFE_INTEGER + 1;
  for (const field of ROW_INTEGERS) {
    const payload = validPayload();
    payload.rows[0][field] = unsafe;
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, false, field);
    assert.deepEqual(result.rows, [], `${field} leaked a valid-looking prefix`);
    assert.match(result.why, new RegExp(field));
  }
  for (const field of ['payoff_bp', 'reward_to_risk_bp', 'return_over_drawdown']) {
    const payload = validPayload();
    payload.rows[0][field] = unsafe;
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, false, field);
    assert.deepEqual(result.rows, [], `${field} leaked a valid-looking prefix`);
    assert.match(result.why, new RegExp(field));
  }
  for (const field of RULE_INTEGERS) {
    const payload = validPayload();
    payload.rules[field] = unsafe;
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, false, field);
    assert.deepEqual(result.rows, [], `${field} leaked a valid-looking prefix`);
    assert.match(result.why, new RegExp(field));
  }
  for (const field of ['count', 'admitted']) {
    const payload = validPayload();
    payload[field] = unsafe;
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, false, field);
    assert.deepEqual(result.rows, [], `${field} leaked a valid-looking prefix`);
    assert.match(result.why, new RegExp(field));
  }
});

test('nullable ratios preserve null but never admit an unsafe number', () => {
  const payload = validPayload();
  Object.assign(payload.rows[0], {
    wins: 4,
    losses: 0,
    pessimistic: 400,
    worst_trade: 0,
    max_drawdown: 0,
    min_win: 100,
    win_rate_bp: 10_000,
    avg_win: 100,
    avg_loss: 0,
    gross_win: 400,
    gross_loss: 0
  });
  payload.rows[0].reward_to_risk_bp = null;
  payload.rows[0].return_over_drawdown = null;
  const result = validateFrontierPayload(payload);
  assert.equal(result.ok, true, result.why);
});

test('canonical masks and directions are required before any ranking row is published', () => {
  /** @type {Array<(payload: any) => void>} */
  const mutations = [
    (payload) => {
      payload.rows[1].direction = 'undirected';
    },
    (payload) => {
      payload.rows[1].mask_words[0] = '01';
    },
    (payload) => {
      payload.rows[1].mask_words.pop();
    }
  ];
  for (const mutate of mutations) {
    const payload = validPayload();
    mutate(payload);
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, false);
    assert.deepEqual(result.rows, []);
  }
});

test('envelope and row arithmetic contradictions refuse the entire answer', () => {
  /** @type {Array<[string, (payload: any) => void]>} */
  const cases = [
    ['count', (payload) => (payload.count = 1)],
    ['admitted', (payload) => (payload.admitted = 2)],
    ['rank', (payload) => (payload.rows[1].rank = 1)],
    ['evidence', (payload) => (payload.rows[0].edge_wins = 6)],
    ['losses', (payload) => (payload.rows[0].losses = 2)],
    ['priced', (payload) => (payload.rows[0].priced = false)],
    ['gross', (payload) => (payload.rows[0].gross_loss = -99)],
    ['verdict', (payload) => (payload.rows[0].meets.all = false)]
  ];
  for (const [name, mutate] of cases) {
    const payload = validPayload();
    mutate(payload);
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, false, name);
    assert.deepEqual(result.rows, [], `${name} leaked a valid-looking prefix`);
  }
});

test('raw cell totals, every derived figure, and every rule verdict must agree exactly', () => {
  /** @type {Array<[string, (payload: any) => void]>} */
  const cases = [
    ['win rate', (payload) => (payload.rows[0].win_rate_bp += 1)],
    ['reward', (payload) => (payload.rows[0].reward_to_risk_bp += 1)],
    ['drawdown return', (payload) => (payload.rows[0].return_over_drawdown += 1)],
    ['average win', (payload) => (payload.rows[0].avg_win += 1)],
    ['average loss', (payload) => (payload.rows[0].avg_loss -= 1)],
    ['individual verdict', (payload) => (payload.rows[0].meets.assurance = false)],
    ['unchecked stop', (payload) => (payload.rows[0].meets.stop_unchecked = false)],
    // A row claiming the PROTECTIVE-EXIT rule was checked is refused for the
    // same reason as the stop: `frontier::Row` drops `stop`, `target`, `tsl`
    // and `ttp`, so nothing downstream can answer that rule from this payload.
    // A server asserting otherwise is asserting a measurement it did not take.
    [
      'unchecked protective exits',
      (payload) => (payload.rows[0].meets.protective_exits_unchecked = false)
    ]
  ];
  for (const [name, mutate] of cases) {
    const payload = validPayload();
    mutate(payload);
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, false, name);
    assert.deepEqual(result.rows, []);
  }
});

test('the body identity is canonical and bound to the requested run', () => {
  const missing = validPayload();
  delete missing.identity;
  assert.match(validateFrontierPayload(missing).why, /canonical run identity/);

  const payload = validPayload();
  assert.equal(validateFrontierPayload(payload, payload.identity).ok, true);
  assert.match(validateFrontierPayload(payload, 'ef'.repeat(32)).why, /answered for run/);
});

test('partial server reads are refusals, while both explicit empty states stay honest', () => {
  const partial = validPayload();
  partial.refusal = 'sealed row two is corrupt';
  const refused = validateFrontierPayload(partial);
  assert.equal(refused.ok, false);
  assert.deepEqual(refused.rows, []);

  const committed = validateFrontierPayload({
    identity: 'ab'.repeat(32),
    rows: [],
    count: 0,
    admitted: 0,
    rules: null,
    refusal: null
  });
  assert.equal(committed.ok, true, committed.why);
  assert.equal(committed.empty, true);

  const absent = validateFrontierPayload({
    identity: 'ab'.repeat(32),
    rows: [],
    count: 0,
    refusal: 'frontier.bin does not exist'
  });
  assert.equal(absent.ok, true, absent.why);
  assert.equal(absent.empty, true);
  assert.equal(absent.why, 'frontier.bin does not exist');
});

test('malformed schema shapes cannot masquerade as a zero-row frontier', () => {
  for (const input of [
    null,
    [],
    {},
    { rows: [], count: 0, admitted: 1, rules: null, refusal: null },
    { rows: [], count: 0, admitted: 0, rules: {}, refusal: null }
  ]) {
    const result = validateFrontierPayload(input);
    assert.equal(result.ok, false);
    assert.deepEqual(result.rows, []);
  }
});

// ---------------------------------------------------------------------------
// P19-03 (D-2562): the rules a mutation could delete with the suite green.
// ---------------------------------------------------------------------------

/**
 * One priced row of `wins` winning and `trades - wins` losing round trips,
 * every derived figure computed the way Rust derives it, and every rule but
 * assurance set to pass -- so `meets.all` is exactly `meets.assurance`.
 * Each win is +100 paisa and each loss -50; generated shape, not market data.
 *
 * @param {number} wins @param {number} trades @param {boolean} assured
 * @returns {any}
 */
function cell(wins, trades, assured) {
  const losses = trades - wins;
  const gross_win = 100 * wins;
  const gross_loss = losses > 0 ? -50 * losses : 0;
  const pessimistic = gross_win + gross_loss;
  const max_drawdown = losses > 0 ? 50 : 0;
  return {
    rank: 1,
    direction: 'long',
    mask_words: ['1', '0', '0', '0', '0', '0'],
    hits: trades,
    n: trades,
    mean_milli_paisa: 1_000,
    t_milli: 2_000,
    // One observation is a payoff refusal, which the route sends as null.
    payoff_bp: trades < 2 ? null : 200,
    edge_wins: wins,
    priced: true,
    trades,
    wins,
    losses,
    pessimistic,
    worst_trade: losses > 0 ? -50 : 0,
    max_drawdown,
    min_win: wins > 0 ? 100 : 0,
    win_rate_bp: Math.floor((wins * 10_000) / trades),
    reward_to_risk_bp: losses > 0 ? (wins > 0 ? 200 : 0) : null,
    return_over_drawdown: pessimistic <= 0 ? 0 : max_drawdown <= 0 ? null : (pessimistic * 100) / max_drawdown,
    avg_win: wins > 0 ? 100 : 0,
    avg_loss: losses > 0 ? -50 : 0,
    gross_win,
    gross_loss,
    meets: {
      win_rate: true,
      reward_to_risk: true,
      return_over_drawdown: true,
      trades: true,
      assurance: assured,
      all: assured,
      stop_unchecked: true,
      protective_exits_unchecked: true
    }
  };
}

/** @param {any} row @param {number} minAssurance @returns {any} */
const single = (row, minAssurance) => ({
  identity: 'cd'.repeat(32),
  rows: [row],
  count: 1,
  admitted: row.meets.all ? 1 : 0,
  rules: {
    min_win_rate_bp: 0,
    min_rr_bp: 0,
    min_ret_over_dd_bp: 0,
    min_trades: 0,
    min_assurance_bp: minAssurance,
    max_mae_ppm: 0,
    top: 25
  },
  refusal: null
});

// (wins, trades, the Wilson lower bound in basis points). The bound is
// `runner::grid::Cell::assurance_bp`: z = 1.959964, truncated toward zero after
// a clamp to [0, 10_000]. Computed in IEEE double with the SAME operation order
// as that function (and as `frontier-analytics.js`), and pinned on the Rust
// side by `the_wilson_table_the_browser_copy_is_checked_against` in
// `crates/runner/src/grid.rs`, so the two copies are held to ONE table.
//
// Chosen at the edges that kill a drifted copy:
//   * z = 1.96 instead of 1.959964 moves 3/3 and 4/4 by one basis point;
//   * Math.round instead of Math.trunc moves 1/4, 7/9, 2/3, 19/20, 1000/1000;
//   * z = 1.645 moves every non-zero row by hundreds.
/** @type {[number, number, number][]} */
const WILSON = [
  [0, 1, 0],
  [1, 1, 2065],
  [1, 2, 945],
  [2, 3, 2076],
  [3, 3, 4385],
  [1, 4, 455],
  [3, 4, 3006],
  [4, 4, 5101],
  [7, 9, 4525],
  [19, 20, 7638],
  [50, 100, 4038],
  [99, 100, 9455],
  [100, 100, 9630],
  [1000, 1000, 9961]
];

test('the browser Wilson bound matches the Rust table and flips meets.assurance exactly at the rule', () => {
  for (const [wins, trades, bound] of WILSON) {
    for (const rule of [bound - 1, bound, bound + 1]) {
      if (rule < 0) continue;
      const truth = bound >= rule;
      const honest = validateFrontierPayload(single(cell(wins, trades, truth), rule));
      assert.equal(honest.ok, true, `${wins}/${trades} at rule ${rule}: ${honest.why}`);
      assert.equal(honest.admitted, truth ? 1 : 0);
      const lying = validateFrontierPayload(single(cell(wins, trades, !truth), rule));
      assert.equal(lying.ok, false, `${wins}/${trades} at rule ${rule} accepted a wrong assurance verdict`);
      assert.match(lying.why, /meets does not match/);
      assert.deepEqual(lying.rows, []);
    }
  }
});

test('a frontier carrying more rows than rules.top is refused, and exactly top is not', () => {
  for (const [top, ok] of /** @type {[number, boolean][]} */ ([[1, false], [2, true], [3, true]])) {
    const payload = validPayload();
    payload.rules.top = top;
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, ok, `top ${top}: ${result.why}`);
    if (!ok) {
      assert.match(result.why, /more rows than rules\.top/);
      assert.deepEqual(result.rows, []);
    }
  }
});

test('ranks must be exactly 1..count in wire order: skipped, shifted, reversed and repeated ranks all refuse', () => {
  for (const ranks of [[2, 3], [1, 3], [0, 1], [2, 1], [1, 1], [-1, 0]]) {
    const payload = validPayload();
    payload.rows[0].rank = ranks[0];
    payload.rows[1].rank = ranks[1];
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, false, JSON.stringify(ranks));
    assert.match(result.why, /rank/, JSON.stringify(ranks));
    assert.deepEqual(result.rows, []);
  }
});

test('a ratio Rust cannot divide must arrive as null, and a ratio it can must not', () => {
  /** @returns {any} */
  const unbeaten = () => {
    const payload = validPayload();
    Object.assign(payload.rows[0], {
      wins: 4, losses: 0, pessimistic: 400, worst_trade: 0, max_drawdown: 0, min_win: 100,
      win_rate_bp: 10_000, avg_win: 100, avg_loss: 0, gross_win: 400, gross_loss: 0,
      reward_to_risk_bp: null, return_over_drawdown: null
    });
    return payload;
  };
  assert.equal(validateFrontierPayload(unbeaten()).ok, true);
  for (const [field, value] of /** @type {[string, number][]} */ ([
    ['reward_to_risk_bp', 300], ['reward_to_risk_bp', 0],
    ['return_over_drawdown', 400], ['return_over_drawdown', 0]
  ])) {
    const payload = unbeaten();
    payload.rows[0][field] = value;
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, false, `${field} ${value} where Rust sends null`);
    assert.deepEqual(result.rows, []);
  }
  for (const field of ['reward_to_risk_bp', 'return_over_drawdown']) {
    const payload = validPayload();
    payload.rows[0][field] = null;
    assert.equal(validateFrontierPayload(payload).ok, false, `${field} null where Rust sends a number`);
  }
});

// ---------------------------------------------------------------------------
// p5num-5 (D-2568): `payoff_bp` is nullable, and null below two observations.
// ---------------------------------------------------------------------------

test('an unbounded or refused payoff arrives as null and the frontier is still admitted', () => {
  const payload = validPayload();
  payload.rows[0].payoff_bp = null;
  const result = validateFrontierPayload(payload);
  assert.equal(result.ok, true, result.why);
  assert.equal(result.rows[0].payoff_bp, null);
  // The bare i64::MAX the route used to send is not an exact JavaScript number.
  const raw = validPayload();
  raw.rows[0].payoff_bp = JSON.parse('9223372036854775807');
  assert.equal(validateFrontierPayload(raw).ok, false);
});

test('below two observations a payoff number is a refusal read as a measurement, and is refused', () => {
  for (const [n, payoff, ok] of /** @type {[number, number|null, boolean][]} */ ([
    [0, null, true], [1, null, true], [0, 0, false], [1, 0, false], [1, 150, false],
    [2, 0, true], [2, null, true], [2, 150, true]
  ])) {
    const payload = validPayload();
    Object.assign(payload.rows[0], { n, edge_wins: Math.min(n, payload.rows[0].edge_wins), payoff_bp: payoff });
    const result = validateFrontierPayload(payload);
    assert.equal(result.ok, ok, `n ${n} payoff ${String(payoff)}: ${result.why}`);
    if (!ok) assert.match(result.why, /payoff_bp must be null below two observations/);
  }
});
