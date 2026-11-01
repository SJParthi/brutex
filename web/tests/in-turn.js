// BASE-REDS-1023 repair 6 (D-7152 addendum 13, L-BRED23-19): how a test waits and how it times, on a loaded machine.
// Not a test file: `npm test` runs tests/*.test.js, and tests/in-turn.test.js proves every decision here with
// invented figures.
//
// A test that waited on the clock lost its race whenever the CPU went elsewhere. Check r6 of this part found two of
// four whole web runs red on one tip, each by a different test: 'the watch asked 1 time(s) in 40 ms at a 5 ms period'
// (a sleep that assumed how many periods fit in it) and 'the long-query filter grew 3.49x' (two sides of a ratio
// measured one after the other, so a stall lands on one side only). Neither is a defect of what the test tests, and
// neither is cured by a looser bound. So there are two tools here, and both keep the bound the test already had.

import { hrtime } from 'node:process';

/**
 * Waits for an ORDER, never for an amount of time. Asks `ready` once per timer tick, at most `ticks` times, and answers
 * whether it came true. A tick is a real timer turn, so a watch or a poller on a timer of its own moves between two
 * asks, and a read's stream moves too. How many ticks a chain of events takes does not depend on how busy the CPU is;
 * a stall only makes each tick later. The bound is in ticks: an order that never comes ends the wait with `false`, by
 * name in the test's own assertion, never a hang.
 *
 * @param {() => boolean} ready
 * @param {number} [ticks]
 * @returns {Promise<boolean>}
 */
export async function untilTicks(ready, ticks = 20_000) {
  for (let tick = 0; tick < ticks && !ready(); tick += 1) {
    await new Promise((done) => setTimeout(done, 1));
  }
  return ready();
}

/**
 * The nanoseconds one call of `op` costs, over `reps` calls after one warm call.
 *
 * @param {number} reps
 * @param {() => unknown} op
 * @returns {number}
 */
export function costOnce(reps, op) {
  op();
  const t0 = hrtime.bigint();
  for (let i = 0; i < reps; i += 1) op();
  return Number(hrtime.bigint() - t0) / reps;
}

/**
 * The decision, over any source of samples. Each round takes one sample of EACH side, in turn, so the two sides of a
 * ratio are measured in the same window; each side keeps its minimum, because interference only adds time, so a
 * side's minimum is the operation and not the scheduler. From round `atLeast` on, `holds(first, second)` over the two
 * minima ends the rounds. `atMost` rounds with no hold answer `held: false` with the minima, and the caller's
 * assertion says them.
 *
 * The bound is never moved. A search that walks a hundredfold catalogue costs a hundredfold in EVERY sample, so its
 * minimum does too, and no number of rounds brings it under a 3x bound. `atLeast` is there for the other direction: a
 * cheap side stalled in its first sample must not read as a hold on one round.
 *
 * @param {(round: number) => [number, number]} sample
 * @param {(first: number, second: number) => boolean} holds
 * @param {{atLeast?: number, atMost?: number}} [limits]
 * @returns {{held: boolean, first: number, second: number, rounds: number}}
 */
export function minimaInTurn(sample, holds, { atLeast = 5, atMost = 120 } = {}) {
  let first = Infinity;
  let second = Infinity;
  for (let round = 0; round < atMost; round += 1) {
    const [a, b] = sample(round);
    first = Math.min(first, a);
    second = Math.min(second, b);
    if (round + 1 >= atLeast && holds(first, second)) return { held: true, first, second, rounds: round + 1 };
  }
  return { held: false, first, second, rounds: atMost };
}

/**
 * `minimaInTurn` over two operations timed by `costOnce`, `reps` calls a sample.
 *
 * @param {number} reps
 * @param {() => unknown} firstOp
 * @param {() => unknown} secondOp
 * @param {(first: number, second: number) => boolean} holds
 * @param {{atLeast?: number, atMost?: number}} [limits]
 * @returns {{held: boolean, first: number, second: number, rounds: number}}
 */
export function costsInTurn(reps, firstOp, secondOp, holds, limits) {
  return minimaInTurn(() => [costOnce(reps, firstOp), costOnce(reps, secondOp)], holds, limits);
}
