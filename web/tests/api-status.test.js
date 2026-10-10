// THE HEADER'S API BUTTON SETTLES ON ITS OWN (DB-N01).
//
// Measured on the live page, 9 Oct 2026: "Checking API…" for at least 37 s,
// title "Checked 01 Jan 1970, 05:30", settled only by a click. `watchVisible`
// runs its first poll only on a visible page, and the never-set stamp was
// rendered as a date.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import { firstCheck, apiTitle } from '../src/lib/api-status.js';
import { watchVisible } from '../src/lib/page-requests.js';

const flush = () => new Promise((resolve) => setTimeout(resolve, 0));
const noTimers = { schedule: () => 0, cancel: () => {} };

test('THE REGRESSION: on a page that is not visible at load the poll alone never checks, and the first check does', async () => {
  let calls = 0;
  const probe = async () => {
    calls += 1;
  };
  const options = { ...noTimers, visible: () => false, listen: () => () => {} };
  const stop = watchVisible(probe, 8000, options);
  await flush();
  assert.equal(calls, 0, 'watchVisible alone ran nothing on a hidden page — the defect');
  const stopFirst = firstCheck(probe, options);
  await flush();
  assert.equal(calls, 1);
  stop();
  stopFirst();
});

test('on a visible page the first check adds nothing to the poll', async () => {
  let calls = 0;
  const probe = async () => {
    calls += 1;
  };
  const options = { ...noTimers, visible: () => true, listen: () => () => {} };
  const stop = watchVisible(probe, 8000, options);
  const stopFirst = firstCheck(probe, options);
  await flush();
  assert.equal(calls, 1);
  stop();
  stopFirst();
});

test('a first check stopped before it answers cannot publish', async () => {
  /** @type {boolean[]} */
  const published = [];
  /** @type {() => void} */
  let release = () => {};
  const gate = new Promise((resolve) => {
    release = () => resolve(undefined);
  });
  const stop = firstCheck(
    async (ticket) => {
      await gate;
      published.push(ticket.current());
    },
    { ...noTimers, visible: () => false }
  );
  stop();
  release();
  await flush();
  assert.deepEqual(published, [false]);
});

test('a stamp that was never set is not rendered as a date', () => {
  const stamp = (/** @type {number} */ ms) => `stamp(${ms})`;
  assert.equal(apiTitle({ state: 'checking', why: null, at: 0 }, stamp), 'Round trip to /feeds.json, not answered yet. Click to check now.');
  assert.equal(apiTitle({ state: 'up', why: null, at: 1_791_600_000_000 }, stamp), 'Round trip to /feeds.json. Checked stamp(1791600000000). Click to re-check.');
  assert.equal(apiTitle({ state: 'down', why: 'HTTP 502', at: 5 }, stamp), 'Probing /feeds.json failed: HTTP 502');
});

test('the layout runs the first check beside the poll and titles the button through apiTitle', () => {
  const layout = readFileSync(new URL('../src/routes/+layout.svelte', import.meta.url), 'utf8');
  assert.match(layout, /const first = firstCheck\(probe, options\);/);
  assert.match(layout, /return \(\) => \{ watching\(\); first\(\); refreshProbe = \(\) => \{\}; \};/);
  assert.match(layout, /title=\{apiTitle\(api, stampLabel\)\}/);
});
