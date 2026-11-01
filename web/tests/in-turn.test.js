// BASE-REDS-1023 repair 6 (D-7152 addendum 13, BRED-49): the two tools a timing test on a loaded machine uses, each
// decision proved with invented figures, so no line here depends on how busy the machine is. The one real measure
// below compares a hundredfold on purpose: nothing a scheduler does brings it under the bound.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { costOnce, costsInTurn, minimaInTurn, said, untilTicks } from './in-turn.js';

/** @param {Array<[number, number]>} figures */
const replay = (figures) => (/** @type {number} */ round) => figures[Math.min(round, figures.length - 1)];
const under3 = (/** @type {number} */ big, /** @type {number} */ small) => big / small < 3;

test('a stall on one side of a ratio lands on one sample, and the minimum is the operation', () => {
  // Check r6's own failure in figures: a first read of 3.49x, a stall on the large side only. Measured one side after
  // the other it is the whole answer; here the next sample of that side stands.
  const stalled = minimaInTurn(replay([[349, 100], [349, 100], [120, 100], [150, 100], [130, 100]]), under3);
  assert.deepEqual(stalled, { held: true, first: 120, second: 100, rounds: 5 });
});

test('a side that costs a hundredfold in every sample never holds, however many rounds it is given', () => {
  // The bound is the test's, unmoved. A search that walks the catalogue is this, and it stays red.
  /** @type {Array<[number, number]>} */
  const walks = [];
  for (let i = 0; i < 40; i += 1) walks.push([10_000 + i, 100 + (i % 7)]);
  const answer = minimaInTurn(replay(walks), under3, { atLeast: 5, atMost: 40 });
  assert.deepEqual(answer, { held: false, first: 10_000, second: 100, rounds: 40 });
  assert.ok(answer.first / answer.second >= 100, 'and the caller can say the ratio it measured');
});

test('a cheap side stalled in its first sample is no hold on one round', () => {
  // [100, 90] reads 1.1x. The rule of `atLeast` is what keeps that from ending the rounds before the cheap side's
  // real cost, 1, is seen. One round would have called a hundredfold a hold.
  const figures = replay([[100, 90], [100, 1], [100, 1], [100, 1], [100, 1]]);
  assert.equal(minimaInTurn(figures, under3, { atLeast: 1 }).held, true, 'what one round says');
  assert.deepEqual(minimaInTurn(figures, under3, { atLeast: 5, atMost: 5 }), { held: false, first: 100, second: 1, rounds: 5 });
});

test('the rounds end at the first that holds past atLeast, and never pass atMost', () => {
  /** @type {number[]} */
  const asked = [];
  const holding = minimaInTurn((round) => { asked.push(round); return [1, 1]; }, under3, { atLeast: 3, atMost: 9 });
  assert.deepEqual(asked, [0, 1, 2], 'held from round 1, and still three rounds');
  assert.equal(holding.rounds, 3);
  asked.length = 0;
  const never = minimaInTurn((round) => { asked.push(round); return [9, 1]; }, under3, { atLeast: 2, atMost: 4 });
  assert.deepEqual(asked, [0, 1, 2, 3], 'four rounds, no fifth');
  assert.equal(never.held, false);
  assert.equal(never.rounds, 4);
});

test('a greater-than holds the same way: the crowded side stays dearer than the spread one over the minima', () => {
  const dearer = (/** @type {number} */ crowded, /** @type {number} */ spread) => crowded > spread;
  // The first reads upside down, as a stalled spread side would; the rounds go on until the minima say what is so.
  const answer = minimaInTurn(replay([[500, 900], [500, 900], [500, 900], [500, 900], [500, 20]]), dearer);
  assert.deepEqual(answer, { held: true, first: 500, second: 20, rounds: 5 });
});

test('what a measure read is said with both minima, the ratio and the rounds', () => {
  assert.equal(said('a probe', { held: true, first: 30, second: 20, rounds: 5 }), 'a probe: 30.0 ns against 20.0 ns, 1.50x, the minimum of each side over 5 rounds in turn');
});

test('costOnce warms once, then times exactly the reps it was asked for', () => {
  let calls = 0;
  const ns = costOnce(10, () => { calls += 1; });
  assert.equal(calls, 11);
  assert.ok(Number.isFinite(ns) && ns >= 0, `${ns}`);
});

test('the two sides are timed in turn, each sample of one between two of the other', () => {
  /** @type {string[]} */
  const order = [];
  const answer = costsInTurn(1, () => order.push('first'), () => order.push('second'), () => true, { atLeast: 2, atMost: 2 });
  assert.equal(answer.rounds, 2);
  // Each sample is one warm call and one timed call.
  assert.deepEqual(order, ['first', 'first', 'second', 'second', 'first', 'first', 'second', 'second']);
});

test('a real walk over a hundredfold list is measured past the bound, through the same code the ratio tests use', () => {
  const big = Array.from({ length: 40_000 }, (_, i) => i);
  const small = big.slice(0, 400);
  /** @param {number[]} rows */
  const walk = (rows) => rows.filter((n) => n === -1).length;
  const answer = costsInTurn(20, () => walk(big), () => walk(small), under3, { atLeast: 3, atMost: 6 });
  assert.equal(answer.held, false, `a walk over 100x the rows read ${(answer.first / answer.second).toFixed(2)}x`);
});

test('a wait for an order ends as soon as the order holds, whatever the clock says', async () => {
  let seen = 0;
  const met = await untilTicks(() => { seen += 1; return seen >= 4; });
  assert.equal(met, true);
  assert.equal(seen, 5, 'asked per tick, and once more for the answer it returns');
  assert.equal(await untilTicks(() => true), true, 'an order that already holds needs no tick');
});

test('a wait for an order that never comes ends at its bound, says so, and never hangs', async () => {
  let asked = 0;
  assert.equal(await untilTicks(() => { asked += 1; return false; }, 3), false);
  assert.equal(asked, 4, 'three ticks, and the answer');
});

test('a wait moves a poller on a timer of its own: the ticks are real timer turns', async () => {
  let polled = 0;
  /** @type {ReturnType<typeof setTimeout>} */
  let timer;
  const poll = () => { polled += 1; timer = setTimeout(poll, 5); };
  timer = setTimeout(poll, 5);
  try {
    assert.equal(await untilTicks(() => polled >= 3), true);
  } finally {
    clearTimeout(timer);
  }
});
