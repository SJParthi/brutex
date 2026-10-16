// /ingest'S EXCHANGE CALENDAR READ IS TICKETED — CE-71 (D-2732).
//
// Drives `createCalendarLoader` from `$lib/calendar-owed.js` with the page's
// own `createPageRequests`, and a transport that IGNORES abort, so the only
// thing keeping a late answer out is the ticket.

import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import { createCalendarLoader } from '../src/lib/calendar-owed.js';
import { createPageRequests } from '../src/lib/page-requests.js';

/** A calendar body with one traded day, so `first` names whose it is. */
const body = (/** @type {number} */ day) => ({ days: [{ day, owed: 375 }], firstDay: day, lastDay: day });

function harness() {
  /** @type {Map<string, (value: Response) => void>} */
  const answer = new Map();
  /** @type {any[]} */
  const applied = [];
  const loader = createCalendarLoader({
    request: (url) =>
      new Promise((resolve) => {
        answer.set(new URL(url, 'http://x').searchParams.get('feed') ?? '', resolve);
      }),
    apply: (c) => applied.push(c),
    requests: createPageRequests()
  });
  const reply = (/** @type {string} */ feed, /** @type {number} */ day) =>
    answer.get(feed)?.(new Response(JSON.stringify(body(day)), { status: 200 }));
  return { loader, applied, reply, answer };
}

const settle = () => new Promise((r) => setTimeout(r, 10));

test('a slower answer for the previous feed never replaces the current feed\'s calendar', async () => {
  const { loader, applied, reply, answer } = harness();
  void loader.load('A');
  await settle();
  void loader.load('B');
  await settle();
  reply('A', 19_000); // A's read was revoked; the transport ignored the abort.
  await settle();
  assert.ok(answer.has('B'), 'B is read once A is out of the way');
  reply('B', 20_000);
  await settle();
  const last = applied[applied.length - 1];
  assert.equal(last.first, '2024-10-04', 'B\'s calendar stands');
  assert.ok(
    applied.every((c) => c.first !== '2022-01-08'),
    'A\'s calendar was never applied after the switch'
  );
});

test('a feed change clears the calendar to "not loaded" at once, and no feed says so', async () => {
  const { loader, applied, reply } = harness();
  void loader.load('A');
  await settle();
  reply('A', 19_000);
  await settle();
  assert.equal(applied[applied.length - 1].first, '2022-01-08');
  void loader.load('B');
  assert.equal(applied[applied.length - 1].owed.size, 0, 'A\'s calendar does not stand under B');
  assert.equal(applied[applied.length - 1].why, '');
  void loader.load(null);
  assert.equal(applied[applied.length - 1].owed.size, 0);
  assert.equal(applied[applied.length - 1].why, 'no feed is selected');
  reply('B', 20_000);
  await settle();
  assert.equal(applied[applied.length - 1].why, 'no feed is selected', 'B\'s late answer is refused');
});

test('/ingest loads its calendar through the ticketed loader', () => {
  const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /createCalendarLoader\(/);
  assert.match(page, /calendarLoader\.load\(feed\)/);
  assert.doesNotMatch(page, /async function loadCalendar/);
});
