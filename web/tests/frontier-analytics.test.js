import { test } from 'node:test';
import assert from 'node:assert/strict';

import { validateFrontierPayload } from '../src/lib/frontier-analytics.js';

const ROW_INTEGERS = Object.freeze([
  'rank',
  'hits',
  'n',
  'mean_milli_paisa',
  't_milli',
  'payoff_bp',
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
  for (const field of ['reward_to_risk_bp', 'return_over_drawdown']) {
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
