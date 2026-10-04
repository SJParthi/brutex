// CE-81, D-1787: the run card drew a halted feed in the success colour, never
// showed `skipped`, and printed a negative or null-derived landed count.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { feedMeter, landedOf } from '../src/lib/run-card.js';

const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');

test('landed is computed only from two known totals, and never negative', () => {
  assert.deepEqual([landedOf(10, 99).kind, landedOf(10, 99).count], ['landed', 89]);
  assert.deepEqual([landedOf(5, 5).kind, landedOf(5, 5).count], ['landed', 0]);
  const fell = landedOf(1200, 200);
  assert.equal(fell.kind, 'dropped');
  assert.equal(fell.count, 1000);
  for (const [start, now] of [[null, 5], [5, null], [null, null], [undefined, 5], [-1, 5], [1.5, 5], ['5', 6]]) {
    const unknown = landedOf(start, now);
    assert.equal(unknown.kind, 'unknown', JSON.stringify([start, now]));
    assert.equal(unknown.count, null);
    assert.equal(unknown.lead, 'an unknown number of');
  }
});

test('a halted feed is never drawn as success, and its skipped legs are carried', () => {
  assert.deepEqual(feedMeter({ legs: 5, legsDone: 5, finished: true, skipped: 0 }), { up: true, halted: false, skipped: 0 });
  assert.deepEqual(feedMeter({ legs: 5, legsDone: 1, finished: true, skipped: 4 }), { up: false, halted: true, skipped: 4 });
  assert.deepEqual(feedMeter({ legs: 5, legsDone: 2, finished: false }), { up: false, halted: false, skipped: 0 });
  assert.equal(feedMeter({ legs: 3, legsDone: 3, credentialDead: true }).up, false);
  assert.equal(feedMeter({ legs: 0, legsDone: 0, finished: true }).up, false);
});

test('the card uses both readings and no longer subtracts the counters itself', () => {
  assert.doesNotMatch(page, /runState\.rowsNow - runState\.rowsAtStart/);
  assert.match(page, /\{@const landed = landedOf\(runState\.rowsAtStart, runState\.rowsNow\)\}/);
  assert.doesNotMatch(page, /class:up=\{f\.finished \|\|/);
  assert.match(page, /class:up=\{meter\.up\}/);
  assert.match(page, /\{n\(meter\.skipped\)\} leg\(s\) skipped/);
});
