// THE OPERATOR'S RANKING, DRIVEN AS SHIPPED (P19-01, D-2560).
//
// `rankRows` orders the backtest page's "best of these" table by the
// operator's eleven weighted criteria. It is declared inside
// `routes/backtest/+page.svelte`, so nothing can import it.
// `rank-rows-unmeasurable.test.js` (D-2750) drives it, but only with the
// loss-ratio and reward-to-risk weights and no constant column, so inverting
// "less drawdown is better" (M45), scoring a constant column 0 instead of 0.5
// (M46) and every other criterion's direction stayed unpinned. These tests
// EXTRACT the page's own `rankRows`, `losingPct` and
// `lossRatio` through `svelte/compiler` and evaluate them -- no copy of the
// arithmetic lives here, so a mutation of the page is a mutation of what runs.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { declarations, pageSource } from './page-code-fixture.js';

/** @type {(rows: any[], w: Record<string, number>, n: number) => any[]} */
const rankRows = new Function(
  `${declarations(pageSource('backtest'), ['losingPct', 'lossRatio', 'rankRows'])}\nreturn rankRows;`
)();

const KEYS = Object.freeze([
  'drawdown', 'worstTrade', 'losingPct', 'losingTrades', 'lossRatio',
  'profit', 'winningTrades', 'winRate', 'rewardRisk', 'avgWin', 'avgLoss'
]);

/** Every weight zero except `key`. @param {string} key @param {number} [weight] */
const only = (key, weight = 1) => Object.fromEntries(KEYS.map((k) => [k, k === key ? weight : 0]));

/** Every weight one. */
const all = () => Object.fromEntries(KEYS.map((k) => [k, 1]));

/**
 * A priced row with every measurement at one value; tests override one field.
 * Generated shape only -- these are not market results.
 * @param {number} rank @param {Record<string, any>} [over] */
const row = (rank, over = {}) => ({
  rank,
  priced: true,
  max_drawdown: 100,
  worst_trade: -50,
  trades: 10,
  losses: 4,
  wins: 6,
  gross_win: 600,
  gross_loss: -200,
  pessimistic: 400,
  win_rate_bp: 6_000,
  reward_to_risk_bp: 200,
  avg_win: 100,
  avg_loss: -50,
  ...over
});

/** Every ordering of a small array. @template T @param {T[]} items @returns {T[][]} */
function permutations(items) {
  if (items.length <= 1) return [items.slice()];
  /** @type {T[][]} */
  const out = [];
  items.forEach((item, at) => {
    for (const rest of permutations([...items.slice(0, at), ...items.slice(at + 1)])) out.push([item, ...rest]);
  });
  return out;
}

// [weight key, the raw field it reads, three values, the value that must rank FIRST]
/** @type {[string, string, number[], number][]} */
const DIRECTIONS = [
  ['drawdown', 'max_drawdown', [100, 200, 300], 100], // less is better
  ['worstTrade', 'worst_trade', [-300, -200, -100], -100], // nearer zero is better
  ['losingPct', 'losses', [2, 4, 8], 2], // trades held at 10, so fewer losses = smaller share
  ['losingTrades', 'losses', [2, 4, 8], 2], // less is better
  ['lossRatio', 'gross_loss', [-100, -200, -300], -100], // gross_win held, so smaller ratio
  ['profit', 'pessimistic', [100, 200, 300], 300], // more is better
  ['winningTrades', 'wins', [2, 4, 8], 8], // more is better
  ['winRate', 'win_rate_bp', [1_000, 5_000, 9_000], 9_000], // more is better
  ['rewardRisk', 'reward_to_risk_bp', [100, 200, 300], 300], // more is better
  ['avgWin', 'avg_win', [10, 20, 30], 30], // more is better
  ['avgLoss', 'avg_loss', [-30, -20, -10], -10] // nearer zero is better
];

test('less_drawdown_ranks_higher, for every ordering of the input (M45, M47)', () => {
  for (const order of permutations([100, 200, 300])) {
    const rows = order.map((max_drawdown, at) => row(at + 1, { max_drawdown }));
    const ranked = rankRows(rows, only('drawdown'), 10);
    assert.deepEqual(ranked.map((r) => r.max_drawdown), [100, 200, 300], JSON.stringify(order));
    assert.deepEqual(ranked.map((r) => r.score), [1, 0.5, 0], JSON.stringify(order));
  }
});

test('every one of the eleven criteria points the way its slider says, alone and in every input order', () => {
  for (const [key, field, values, best] of DIRECTIONS) {
    for (const order of permutations(values)) {
      const rows = order.map((value, at) => row(at + 1, { [field]: value }));
      const ranked = rankRows(rows, only(key), 10);
      assert.equal(ranked[0][field], best, `${key}: ${JSON.stringify(order)}`);
      assert.equal(ranked[0].score, 1, `${key}: the best of the set scores the full weight`);
      assert.equal(ranked[2].score, 0, `${key}: the worst of the set scores nothing`);
      // The weight is a plain multiplier, so doubling it doubles every score.
      const doubled = rankRows(rows, only(key, 2), 10);
      assert.deepEqual(doubled.map((r) => r.score), ranked.map((r) => 2 * r.score), key);
    }
  }
});

test('a_constant_column_contributes_one_half, per criterion and summed (M46)', () => {
  for (const key of KEYS) {
    const ranked = rankRows([row(1), row(2), row(3)], only(key), 10);
    assert.deepEqual(ranked.map((r) => r.score), [0.5, 0.5, 0.5], key);
  }
  const summed = rankRows([row(1), row(2)], all(), 10);
  assert.deepEqual(summed.map((r) => r.score), [5.5, 5.5]);
  // One priced row is a set of one: every range is a point, every term 0.5.
  assert.deepEqual(rankRows([row(7)], all(), 10).map((r) => r.score), [5.5]);
});

test('ties_break_on_server_rank, whatever order the rows arrive in', () => {
  for (const order of permutations([1, 2, 3, 4])) {
    const ranked = rankRows(order.map((rank) => row(rank)), all(), 10);
    assert.deepEqual(ranked.map((r) => r.rank), [1, 2, 3, 4], JSON.stringify(order));
  }
  // Zero weights make every score 0, and the order is still total.
  const silent = rankRows([row(3), row(1), row(2)], only('drawdown', 0), 10);
  assert.deepEqual(silent.map((r) => [r.rank, r.score]), [[1, 0], [2, 0], [3, 0]]);
});

// The never-lost `reward_to_risk_bp: null` and the never-won loss ratio are
// D-2750's rules and `rank-rows-unmeasurable.test.js`'s subject; this file pins
// the GENERIC rule they are exceptions to, on a criterion with no exception.
test('an unmeasurable value is excluded from the range and scores one half, never a measured zero', () => {
  const rows = [
    row(1, { avg_win: null }),
    row(2, { avg_win: 100 }),
    row(3, { avg_win: 300 })
  ];
  const ranked = rankRows(rows, only('avgWin'), 10);
  // Were null coerced to 0, the range would be 0..300 and 100 would score 1/3.
  assert.deepEqual(ranked.map((r) => [r.rank, r.score]), [[3, 1], [1, 0.5], [2, 0]]);
  // Every value unmeasurable: no range at all, every row one half.
  const none = rankRows([row(1, { avg_win: null }), row(2, { avg_win: null })], only('avgWin'), 10);
  assert.deepEqual(none.map((r) => r.score), [0.5, 0.5]);
  for (const bad of [undefined, NaN]) {
    const odd = rankRows([row(1, { avg_win: bad }), row(2, { avg_win: 100 }), row(3, { avg_win: 300 })], only('avgWin'), 10);
    assert.deepEqual(odd.map((r) => [r.rank, r.score]), [[3, 1], [1, 0.5], [2, 0]], String(bad));
  }
  // An unbounded value is not unmeasured (D-2750): +Infinity tops the range
  // (ties the best measured row, broken on rank), -Infinity floors it.
  const top = rankRows([row(1, { avg_win: Infinity }), row(2, { avg_win: 100 }), row(3, { avg_win: 300 })], only('avgWin'), 10);
  assert.deepEqual(top.map((r) => [r.rank, r.score]), [[1, 1], [3, 1], [2, 0]]);
  const bottom = rankRows([row(1, { avg_win: -Infinity }), row(2, { avg_win: 100 }), row(3, { avg_win: 300 })], only('avgWin'), 10);
  assert.deepEqual(bottom.map((r) => [r.rank, r.score]), [[3, 1], [1, 0], [2, 0]]);
});

test('a row with no trades is wholly losing on the losing-share criterion', () => {
  const ranked = rankRows([row(1, { trades: 0, losses: 0 }), row(2, { trades: 10, losses: 5 })], only('losingPct'), 10);
  assert.deepEqual(ranked.map((r) => [r.rank, r.score]), [[2, 1], [1, 0]]);
});

test('only priced rows are ranked, the count bounds the answer, and the input is not mutated', () => {
  assert.deepEqual(rankRows([], all(), 10), []);
  assert.deepEqual(rankRows([row(1, { priced: false })], all(), 10), []);
  const rows = [row(1, { priced: false, max_drawdown: 0 }), row(2, { max_drawdown: 300 }), row(3, { max_drawdown: 100 })];
  const ranked = rankRows(rows, only('drawdown'), 10);
  assert.deepEqual(ranked.map((r) => r.rank), [3, 2], 'the unpriced row neither ranks nor sets the range');
  assert.deepEqual(ranked.map((r) => r.score), [1, 0]);
  for (const [n, expected] of /** @type {[number, number[]][]} */ ([[0, []], [1, [3]], [2, [3, 2]], [3, [3, 2]]])) {
    assert.deepEqual(rankRows(rows, only('drawdown'), n).map((r) => r.rank), expected, `n=${n}`);
  }
  for (const r of rows) assert.equal(Object.hasOwn(r, 'score'), false, 'scores are written to copies');
});
