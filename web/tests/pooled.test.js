// THE POOL THAT DECIDES HOW MANY REQUESTS `/terminal` HAS OUTSTANDING.
//
// `$lib/pooled.js` says in its own header why it is runeless `.js` rather than
// `.svelte.js`: so `node --test` can drive it, because "a concurrency pool that
// nothing can test is a concurrency pool nobody should trust". This file is
// what turns that sentence from a justification into a fact.
//
// Three properties carry real weight, and only one of them is about speed:
//
//   ORDER. Results come back in the order of `items`, never the order they
//   finished. `/terminal` indexes the answers straight against the rows it
//   drew, so a pool that reordered would put one instrument's price on another
//   instrument's line — a wrong number wearing a right number's shape, which is
//   the class of defect this repository is most careful about.
//
//   THE CAP. At most `limit` outstanding. The number is 6 because that is what
//   a browser grants one origin over HTTP/1.1; exceeding it does not create
//   more parallelism, it creates a queue inside the browser that nothing here
//   can see or report on.
//
//   NO DEADLOCK. No argument — an empty list, a zero limit, a negative one —
//   may produce a pool that never drains.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { pooled, IN_FLIGHT } from '../src/lib/pooled.js';

/** Resolves after `ms`, so a job can be made to finish out of turn. */
const after = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

test('results are in the order of `items`, not the order they finished', async () => {
  // The FIRST item is the SLOWEST, so completion order is the exact reverse of
  // input order. Anything that returns work in finishing order fails here.
  const out = await pooled([30, 20, 10, 0], 4, async (ms) => {
    await after(ms);
    return ms;
  });
  assert.deepEqual(out, [30, 20, 10, 0]);
});

test('at most `limit` jobs are outstanding at any moment', async () => {
  let outstanding = 0;
  let peak = 0;
  await pooled(Array.from({ length: 20 }, (_, i) => i), 3, async (i) => {
    outstanding += 1;
    peak = Math.max(peak, outstanding);
    await after(1);
    outstanding -= 1;
    return i;
  });
  // THE ASSERTION IS `<=`, NOT `===`. A pool that never reached 3 would also be
  // wrong, so the second line pins the floor separately rather than letting a
  // pool that ran everything serially pass a ceiling check.
  assert.ok(peak <= 3, `peak was ${peak}, which is above the limit of 3`);
  assert.equal(peak, 3);
});

test('every item is visited exactly once, and the pool drains', async () => {
  const seen = [];
  const out = await pooled(['a', 'b', 'c', 'd', 'e'], 2, async (item) => {
    seen.push(item);
    return item.toUpperCase();
  });
  assert.deepEqual(out, ['A', 'B', 'C', 'D', 'E']);
  assert.deepEqual(seen.slice().sort(), ['a', 'b', 'c', 'd', 'e']);
});

test('the job receives the item AND its index', async () => {
  const out = await pooled(['x', 'y', 'z'], 2, async (item, index) => `${index}:${item}`);
  assert.deepEqual(out, ['0:x', '1:y', '2:z']);
});

test('an empty list resolves to an empty array without starting a worker', async () => {
  let started = 0;
  const out = await pooled([], IN_FLIGHT, async () => {
    started += 1;
    return 1;
  });
  assert.deepEqual(out, []);
  assert.equal(started, 0);
});

test('a limit below one is raised to one rather than deadlocking', async () => {
  // A pool of zero workers would never drain, and the promise would never
  // settle — a hang, which is the failure mode a timeout catches long after the
  // reader has concluded the page is broken.
  for (const limit of [0, -5]) {
    const out = await pooled([1, 2, 3], limit, async (n) => n * 2);
    assert.deepEqual(out, [2, 4, 6], `limit ${limit} did not drain correctly`);
  }
});

test('a limit above the item count does not over-spawn', async () => {
  let peak = 0;
  let outstanding = 0;
  await pooled([1, 2], 50, async (n) => {
    outstanding += 1;
    peak = Math.max(peak, outstanding);
    await after(1);
    outstanding -= 1;
    return n;
  });
  assert.equal(peak, 2);
});

test('a job that throws rejects the call, which is why callers return failures as values', async () => {
  // NOT A DESIGN COMPLAINT, A PINNED CONTRACT. The pool does not swallow
  // errors, so `$lib/terminal.svelte.js`'s `quote()` catches its own and
  // returns a Quote carrying `why` instead. This test is what stops someone
  // "fixing" the pool to swallow, which would hide a broken series behind a
  // blank cell rather than a named one.
  await assert.rejects(
    () =>
      pooled([1, 2, 3], 2, async (n) => {
        if (n === 2) throw new Error('job 2 failed');
        return n;
      }),
    /job 2 failed/
  );
});

test('IN_FLIGHT is six, the browser’s per-origin ceiling over HTTP/1.1', () => {
  assert.equal(IN_FLIGHT, 6);
});
