// p14num-2: the web leaderboard scored an unmeasurable ratio as the midpoint.
// A row that never won (unbounded loss ratio) beat measured rows on "less
// losing ratio", and a row that never lost (reward_to_risk_bp null, the
// server's i64::MAX sentinel) sat mid-board while cli::ranked sorts it last.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';

const source = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
const body = /** @type {any} */ (parse(source)).instance.content.body;
/** @param {string} name */
function decl(name) {
  const found = body.find((/** @type {any} */ n) =>
    (n.type === 'FunctionDeclaration' && n.id?.name === name) ||
    (n.type === 'VariableDeclaration' && n.declarations.some((/** @type {any} */ d) => d.id?.name === name)));
  assert.ok(found, `backtest: actual ${name} is required`);
  return source.slice(found.start, found.end);
}
const rankRows = new Function(`${decl('losingPct')}\n${decl('lossRatio')}\n${decl('rankRows')}\nreturn rankRows;`)();
const zero = { drawdown: 0, worstTrade: 0, losingPct: 0, losingTrades: 0, lossRatio: 0, profit: 0, winningTrades: 0, winRate: 0, rewardRisk: 0, avgWin: 0, avgLoss: 0 };
const row = (/** @type {number} */ rank, /** @type {any} */ extra) => ({ rank, priced: true, trades: 10, losses: 5, wins: 5, ...extra });
const order = (/** @type {any[]} */ rows, /** @type {any} */ w) => rankRows(rows, { ...zero, ...w }, rows.length).map((/** @type {any} */ r) => r.label);

test('a row that never won ranks last on "less losing ratio", not in the middle', () => {
  const rows = [
    row(1, { label: 'ratio 0.2', gross_win: 1000, gross_loss: -200 }),
    row(2, { label: 'ratio 0.9', gross_win: 1000, gross_loss: -900 }),
    row(3, { label: 'never won', gross_win: 0, gross_loss: -1000 })
  ];
  assert.deepEqual(order(rows, { lossRatio: 1 }), ['ratio 0.2', 'ratio 0.9', 'never won']);
  // It scores what the worst measured row scores (0), and the server rank
  // breaks the tie; before the fix it scored the midpoint, 0.5.
  const scored = rankRows(rows, { ...zero, lossRatio: 1 }, 3);
  assert.equal(scored.find((/** @type {any} */ r) => r.label === 'never won').score, 0);
});

test('a row that never lost ranks last on "higher win:loss ratio", as cli::ranked does', () => {
  const rows = [
    row(1, { label: 'rr 3.00x', reward_to_risk_bp: 300 }),
    row(2, { label: 'rr 1.00x', reward_to_risk_bp: 100 }),
    row(3, { label: 'unbeaten', reward_to_risk_bp: null })
  ];
  assert.deepEqual(order(rows, { rewardRisk: 1 }), ['rr 3.00x', 'rr 1.00x', 'unbeaten']);
  assert.equal(rankRows(rows, { ...zero, rewardRisk: 1 }, 3).find((/** @type {any} */ r) => r.label === 'unbeaten').score, 0);
  const server = readFileSync(new URL('../../crates/cli/src/lib.rs', import.meta.url), 'utf8');
  assert.match(server, /if ratio == i64::MAX \{ i64::MIN \} else \{ ratio \}/, 'the server rule this mirrors');
});

test('a ratio with no movement on either side still has no reading', () => {
  const rows = [
    row(1, { label: 'a', gross_win: 1000, gross_loss: -200 }),
    row(2, { label: 'flat', gross_win: 0, gross_loss: 0 }),
    row(3, { label: 'b', gross_win: 1000, gross_loss: -900 })
  ];
  const scored = rankRows(rows, { ...zero, lossRatio: 1 }, 3);
  assert.equal(scored.find((/** @type {any} */ r) => r.label === 'flat').score, 0.5);
});
