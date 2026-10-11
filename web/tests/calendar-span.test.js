// WP-61 (DB-B42): /ingest reads the calendar of record over the record's whole
// span, so a day past the feed's last held bar is the record's own kind and
// never an assumed session. The calendar below is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createCalendarLoader, epochDayOfIso, readCalendar } from '../src/lib/calendar-owed.js';

const from = epochDayOfIso('2026-09-28');
// Mon 28 Sep .. Sun 04 Oct 2026: four sessions, a closed Friday (02 Oct), a weekend.
const widened = {
  status: 'ok', span: 'record', from, to: from + 6, firstDay: from, lastDay: from + 6,
  days: 'RRRR...', held: '-------', minuteOwed: { regular: 375 }, special: [],
  heldSpan: { from: '2018-09-03', to: '2026-09-24' }
};

test('the record span is asked for by name, and a closed day past the held span is not a session', async () => {
  /** @type {string[]} */
  const asked = [];
  const request = async (/** @type {string} */ url) => {
    asked.push(url);
    return Response.json(widened);
  };
  const signal = new AbortController().signal;
  const calendar = await readCalendar('gdfl', request, signal, async () => {}, 'record');
  assert.deepEqual(asked, ['/calendar.json?feed=gdfl&span=record']);
  assert.equal(calendar.first, '2026-09-28');
  assert.equal(calendar.last, '2026-10-04');
  assert.equal(calendar.owed.get('2026-10-01'), 375, 'Thursday 01 Oct is a session of the record');
  assert.equal(calendar.owed.has('2026-10-02'), false, 'Friday 02 Oct is closed in the record: not a session');
  assert.equal(calendar.owed.has('2026-10-03'), false);
  assert.equal(calendar.owed.size, 4);
  // THE DEFAULT IS UNCHANGED: a caller that names no span asks for none.
  asked.length = 0;
  await readCalendar('gdfl', request, signal, async () => {});
  assert.deepEqual(asked, ['/calendar.json?feed=gdfl']);
});

test('the loader passes its span through, and /ingest asks for the record', async () => {
  /** @type {string[]} */
  const asked = [];
  /** @type {any[]} */
  const applied = [];
  const loader = createCalendarLoader({
    request: async (/** @type {string} */ url) => {
      asked.push(url);
      return Response.json(widened);
    },
    apply: (calendar) => applied.push(calendar),
    requests: {
      run: (work) => work({ signal: new AbortController().signal, current: () => true }),
      cancel: () => {}
    },
    span: 'record'
  });
  await loader.load('zerodha');
  assert.deepEqual(asked, ['/calendar.json?feed=zerodha&span=record']);
  assert.equal(applied.at(-1).owed.size, 4);
  const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /createCalendarLoader\(\{\s*request,\s*requests: calendarRequests,\s*span: 'record',/);
});
