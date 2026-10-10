// /calendar.json SINCE WP-36 (DB-N19): a calendar the server is still building
// is answered `202` with `"status": "building"` and a retry hint, instead of
// holding the request for the whole build (6.55 s for GDFL NIFTY after a
// restart). The page asks again at the hint, shows the calendar once it is
// whole, and never shows a building answer as a calendar with no sessions.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';

import { CALENDAR_BUILDING_TRIES, askCalendar, readCalendar } from '../src/lib/calendar-owed.js';
import { calendarOf } from '../src/lib/db/selection-model.js';

// 2021-02-22 (Mon) .. 2021-02-28 (Sun), invented counts.
const FROM = 18_680;
const whole = {
  status: 'ok',
  from: FROM,
  to: FROM + 6,
  firstDay: FROM,
  lastDay: FROM + 6,
  days: 'RRRRR..',
  held: '1111100',
  minuteOwed: { rung: '1min', regular: 375, casEligible: 360 },
  special: [],
  withheld: [],
  derivedFrom: ['BANKNIFTY', 'NIFTY'],
  disagreements: []
};

const building = {
  status: 'building',
  feed: 'gdfl',
  rung: null,
  symbol: '',
  phase: 'counting',
  by: 'start-up',
  months_counted: 57,
  months_owed: 194,
  retry_after_ms: 250,
  reason: 'This calendar is being built off the request path (start-up build): it has counted 57 of the 194 month file(s) it reads so far.'
};

/** A transport answering `answers` in turn, and the URLs it was asked. */
function transport(/** @type {Array<() => Response>} */ answers) {
  /** @type {string[]} */
  const asked = [];
  let next = 0;
  return {
    asked,
    request: async (/** @type {string} */ url) => {
      asked.push(url);
      const answer = answers[Math.min(next, answers.length - 1)];
      next += 1;
      return answer();
    }
  };
}

test('a building calendar is asked for again at the server\'s hint until it is whole', async () => {
  const { asked, request } = transport([
    () => Response.json(building, { status: 202 }),
    () => Response.json({ ...building, months_counted: 190 }, { status: 202 }),
    () => Response.json(whole)
  ]);
  /** @type {number[]} */
  const waits = [];
  const calendar = await readCalendar('gdfl', request, new AbortController().signal, async (ms) => {
    waits.push(ms);
  });
  assert.equal(asked.length, 3, 'asked until the whole answer');
  assert.ok(asked.every((url) => url === '/calendar.json?feed=gdfl'));
  assert.deepEqual(waits, [250, 250], 'each wait is the server\'s hint');
  assert.equal(calendar.first, '2021-02-22');
  assert.equal(calendar.owed.size, 5);
  assert.equal(calendar.why, '');
});

test('a calendar still building after every try is empty and says so, never "no sessions"', async () => {
  const { asked, request } = transport([() => Response.json(building, { status: 202 })]);
  const calendar = await readCalendar('gdfl', request, new AbortController().signal, async () => {});
  assert.equal(asked.length, CALENDAR_BUILDING_TRIES);
  assert.equal(calendar.owed.size, 0);
  assert.match(calendar.why, /still building this calendar/);
  assert.match(calendar.why, /57 of the 194 month file/);
  assert.doesNotMatch(calendar.why, /holds no bars/);
});

test('a hint below 50 ms or missing is read as 250 ms', async () => {
  const { request } = transport([
    () => Response.json({ ...building, retry_after_ms: 0 }, { status: 202 }),
    () => Response.json({ ...building, retry_after_ms: undefined }, { status: 202 }),
    () => Response.json(whole)
  ]);
  /** @type {number[]} */
  const waits = [];
  await readCalendar('gdfl', request, new AbortController().signal, async (ms) => {
    waits.push(ms);
  });
  assert.deepEqual(waits, [250, 250]);
});

test('an abort during the wait ends the read as a failure, not a calendar', async () => {
  const { asked, request } = transport([() => Response.json(building, { status: 202 })]);
  const control = new AbortController();
  const reading = readCalendar('gdfl', request, control.signal);
  await new Promise((resolve) => setTimeout(resolve, 20));
  control.abort(new Error('the feed changed'));
  const calendar = await reading;
  assert.equal(asked.length, 1, 'asked once, then waited, then stopped');
  assert.equal(calendar.owed.size, 0);
  assert.match(calendar.why, /the calendar request failed \(the feed changed\)/);
});

test('a refusal is still a refusal, read once', async () => {
  const { asked, request } = transport([
    () => Response.json({ error: 'calendar derivation not admitted (Saturated): retry' }, { status: 429 })
  ]);
  const calendar = await readCalendar('gdfl', request, new AbortController().signal, async () => {});
  assert.equal(asked.length, 1);
  assert.equal(calendar.owed.size, 0);
  assert.match(calendar.why, /Saturated/);
});

test('askCalendar hands back a whole calendar after the building answers, and a refusal unread', async () => {
  const { asked, request } = transport([
    () => Response.json(building, { status: 202 }),
    () => Response.json(whole)
  ]);
  const settled = await askCalendar('/calendar.json?feed=gdfl&rung=1s', request, new AbortController().signal, async () => {});
  assert.equal(asked.length, 2);
  assert.equal(settled.response.status, 200);
  assert.deepEqual(settled.body, whole);

  const refused = transport([() => Response.json({ error: 'no such feed' }, { status: 400 })]);
  const answer = await askCalendar('/calendar.json?feed=x', refused.request, new AbortController().signal, async () => {});
  assert.equal(answer.response.status, 400);
  assert.equal(answer.body, null, 'a refusal is not a calendar body');
  assert.deepEqual(await answer.response.json(), { error: 'no such feed' }, 'and its reason is left to read');
});

/**
 * /db's calendar-of-record read (WP-50's date fields), cut out of
 * `selection.svelte.js` exactly as the page runs it: the callback handed to
 * `calReads.run`, run over a plain `db` and a ticket.
 *
 * @param {(url: string, init: RequestInit) => Promise<Response>} ask
 */
async function dbCalendarRead(ask) {
  const wrapped = '<script module>' +
    readFileSync(new URL('../src/lib/db/selection.svelte.js', import.meta.url), 'utf8') + '</script>';
  /** @type {any} */
  let callback = null;
  /** @param {any} node */
  const walk = (node) => {
    if (callback || !node || typeof node !== 'object') return;
    if (node.type === 'CallExpression' && node.callee?.type === 'MemberExpression' &&
      node.callee.object?.name === 'calReads' && node.callee.property?.name === 'run') {
      callback = node.arguments[0];
      return;
    }
    for (const [key, value] of Object.entries(node)) {
      if (key === 'parent' || key === 'loc' || key === 'metadata') continue;
      if (Array.isArray(value)) value.forEach(walk);
      else if (value && typeof value === 'object') walk(value);
    }
  };
  walk(parse(wrapped).module?.content);
  if (!callback) throw new Error('the /db calendar read moved out of calReads.run');
  const run = new Function('feed', 'rung', 'db', 'ask', 'askCalendar', 'calendarOf',
    `return (${wrapped.slice(callback.start, callback.end)});`);
  const db = { calendar: /** @type {any} */ (undefined), calWhy: 'unset' };
  const quick = (/** @type {string} */ url, /** @type {any} */ request, /** @type {AbortSignal} */ signal) =>
    askCalendar(url, request, signal, async () => {});
  await run('gdfl', '1s', db, ask, quick, calendarOf)({
    signal: new AbortController().signal,
    current: () => true
  });
  return db;
}

test('/db reads a building calendar of record again and shows it once whole (DB-N19)', async () => {
  const { asked, request } = transport([
    () => Response.json(building, { status: 202 }),
    () => Response.json(whole)
  ]);
  const db = await dbCalendarRead(request);
  assert.deepEqual(asked, ['/calendar.json?feed=gdfl&rung=1s', '/calendar.json?feed=gdfl&rung=1s']);
  assert.equal(db.calWhy, '');
  assert.deepEqual(db.calendar, calendarOf(whole));
});

test('/db names a calendar still building after every try as building, never "no day string"', async () => {
  const { asked, request } = transport([() => Response.json(building, { status: 202 })]);
  const db = await dbCalendarRead(request);
  assert.equal(asked.length, CALENDAR_BUILDING_TRIES);
  assert.equal(db.calendar, null);
  assert.match(db.calWhy, /^\/calendar\.json is still being built \(This calendar is being built/);
  assert.doesNotMatch(db.calWhy, /no day string/);
});

test('/db still names a refusal by its HTTP status, read once', async () => {
  const { asked, request } = transport([() => Response.json({ error: 'busy' }, { status: 429 })]);
  const db = await dbCalendarRead(request);
  assert.equal(asked.length, 1);
  assert.equal(db.calendar, null);
  assert.equal(db.calWhy, '/calendar.json answered HTTP 429');
});
