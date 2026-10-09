// WHICH CONDITION IS THE RESTATEMENT (P19-02, D-2561).
//
// `impliedConditions` dims the pivot levels a tighter level in the same
// direction already implies, and the backtest page prints
// "{names.length - implied.size} of {names.length} carry the setup" beside
// them. No test imported the module, so flipping its direction rule dimmed the
// INFORMATIVE condition while printing the same count -- a wrong answer that
// looks exactly like a right one. These drive the shipped module.
//
// The rule, from the module: on the ladder s5 < s4 < s3 < s2 < s1 < bc < tc <
// r1 < … < r5, `above` keeps the HIGHEST level (above s2 implies above s3), and
// `below` keeps the LOWEST (below s2 implies below s1). The finding's own
// sketch said "below s1+s2 (s2 implied)"; that reverses the inequality, and the
// module is right: price below s2 is necessarily below the higher s1.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  impliedConditions,
  splitCondition,
  groupConditions,
  conditionShape
} from '../src/lib/condition-groups.js';

// The order is a fact about the levels, written out here as the module writes
// it, so the exhaustive test below checks the rule against an independent copy
// of the ladder rather than against the function's own lookup.
const LADDER = ['s5', 's4', 's3', 's2', 's1', 'bc', 'tc', 'r1', 'r2', 'r3', 'r4', 'r5'];

/** @param {'above'|'below'} side @param {string} level */
const band = (side, level) => `close_${side}_pivot_${level}_band`;

/** @param {Set<string>} set */
const sorted = (set) => [...set].sort();

test('above s2 and above s3: s3 is the restatement', () => {
  assert.deepEqual(sorted(impliedConditions([band('above', 's2'), band('above', 's3')])), [band('above', 's3')]);
  assert.deepEqual(sorted(impliedConditions([band('above', 's3'), band('above', 's2')])), [band('above', 's3')]);
});

test('below s1 and below s2: s1 is the restatement, because below s2 is the tighter claim', () => {
  assert.deepEqual(sorted(impliedConditions([band('below', 's1'), band('below', 's2')])), [band('below', 's1')]);
  assert.deepEqual(sorted(impliedConditions([band('below', 's2'), band('below', 's1')])), [band('below', 's1')]);
});

test('opposite sides are a band and neither is implied', () => {
  assert.equal(impliedConditions([band('above', 's2'), band('below', 's1')]).size, 0);
  assert.equal(impliedConditions([band('above', 'r1'), band('below', 'r5')]).size, 0);
  // Two directions, two ladders: each direction keeps its own tightest level.
  const mixed = impliedConditions([
    band('above', 's2'), band('above', 's4'), band('below', 'r2'), band('below', 'r4')
  ]);
  assert.deepEqual(sorted(mixed), [band('above', 's4'), band('below', 'r4')].sort());
});

test('the `_band` suffix is a rendering detail: plain and band names share one ladder', () => {
  assert.deepEqual(splitCondition('close_above_pivot_s2_band'), { family: 'pivot', detail: 'above s2' });
  assert.deepEqual(splitCondition('close_above_pivot_s2'), { family: 'pivot', detail: 'above s2' });
  assert.deepEqual(
    sorted(impliedConditions(['close_above_pivot_s2', 'close_above_pivot_s3_band'])),
    ['close_above_pivot_s3_band']
  );
});

test('families that are not an ordered ladder imply nothing', () => {
  for (const names of [
    ['close_above_ema20', 'close_above_ema50'],
    ['close_above_pdl', 'close_above_pdh'],
    ['close_above_supertrend', 'close_below_supertrend'],
    ['close_near_pivot_s2_band', 'close_near_pivot_s3_band'],
    ['close_above_pivot_pp_band', 'close_above_pivot_s3_band'],
    ['pat_doji', 'inside_bar']
  ]) {
    assert.equal(impliedConditions(names).size, 0, JSON.stringify(names));
  }
});

test('malformed and degenerate input implies nothing and does not throw', () => {
  for (const names of /** @type {any[]} */ ([undefined, null, 'close_above_pivot_s2_band', 7, {}, []])) {
    assert.equal(impliedConditions(names).size, 0, String(names));
  }
  // One level alone, and the same name twice, restate nothing.
  assert.equal(impliedConditions([band('above', 's2')]).size, 0);
  assert.equal(impliedConditions([band('above', 's2'), band('above', 's2')]).size, 0);
  // A repeated LOWER level is one implied name, not two.
  assert.deepEqual(
    sorted(impliedConditions([band('above', 's3'), band('above', 's2'), band('above', 's3')])),
    [band('above', 's3')]
  );
  assert.deepEqual(splitCondition(''), { family: '?', detail: '' });
  assert.deepEqual(splitCondition(/** @type {any} */ (null)), { family: '?', detail: '' });
});

test('every subset of the ladder, on each side, keeps exactly its tightest level', () => {
  for (const side of /** @type {('above'|'below')[]} */ (['above', 'below'])) {
    for (let mask = 1; mask < 1 << LADDER.length; mask += 1) {
      const levels = LADDER.filter((_, at) => mask & (1 << at));
      const tightest = side === 'above' ? levels[levels.length - 1] : levels[0];
      const expected = levels.filter((level) => level !== tightest).map((level) => band(side, level)).sort();
      // Reversed input order must not change the answer.
      for (const order of [levels, [...levels].reverse()]) {
        const implied = impliedConditions(order.map((level) => band(side, level)));
        assert.deepEqual(sorted(implied), expected, `${side} ${order.join(',')}`);
      }
    }
  }
});

test('grouping keeps every condition in first-seen order and counts families', () => {
  const names = ['close_above_pdl', band('above', 's2'), 'close_below_supertrend', band('above', 's3')];
  assert.deepEqual(groupConditions(names), [
    { family: 'pdl', details: ['above'] },
    { family: 'pivot', details: ['above s2', 'above s3'] },
    { family: 'supertrend', details: ['below'] }
  ]);
  assert.deepEqual(conditionShape(names), { conditions: 4, families: 3 });
  assert.deepEqual(conditionShape(/** @type {any} */ (null)), { conditions: 0, families: 0 });
});
