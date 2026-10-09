// /calendar.json SINCE D-3336: the calendar of record as a dense day string.
//
// The server answers `days` as one code per epoch day from `from`; /ingest's
// minute denominators are read from `minuteOwed` and `special[].minuteOwed`.
// The older array answer is still read, so a page and a server one release
// apart agree.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { denseOwed, readCalendar } from '../src/lib/calendar-owed.js';

// 2021-02-22 (Mon) .. 2021-02-28 (Sun): the outage day is 2021-02-24.
const FROM = 18_680;
const dense = {
  status: 'ok',
  from: FROM,
  to: FROM + 6,
  firstDay: FROM,
  lastDay: FROM + 6,
  days: 'RRPRR..',
  held: '2202200',
  minuteOwed: { rung: '1min', regular: 375, casEligible: 360 },
  special: [{ day: FROM + 2, code: 'P', minuteOwed: 220, minuteIndexOwed: null }],
  withheld: [],
  derivedFrom: ['BANKNIFTY', 'NIFTY'],
  disagreements: []
};

test('a dense answer gives every session its minutes and omits closed days', () => {
  const { owed, indexOwed } = denseOwed(dense);
  assert.equal(owed.size, 5, 'five sessions, the weekend omitted');
  assert.equal(owed.get('2021-02-22'), 375);
  assert.equal(owed.get('2021-02-24'), 220, 'a special session is sized by the server');
  assert.equal(indexOwed.get('2021-02-24'), null, 'its unproved index count stays unknown');
  assert.equal(indexOwed.get('2021-02-25'), 375);
  assert.equal(owed.has('2021-02-27'), false, 'a closed day is not a session');
});

test('an unsized or outside day is never sized by the page', () => {
  const { owed } = denseOwed({ ...dense, days: 'RU-....', special: [] });
  assert.equal(owed.get('2021-02-23'), null, 'U is a session of unknown length');
  assert.equal(owed.has('2021-02-24'), false, '- is outside the calendar, not a session');
  assert.equal(denseOwed({ days: 'RR' }).owed.size, 0, 'no `from`, nothing read');
  assert.equal(denseOwed({ days: [] }).owed.size, 0, 'the older array is not this reader');
});

test('readCalendar reads the dense answer and a withheld answer names its reason', async () => {
  const signal = new AbortController().signal;
  const calendar = await readCalendar('zerodha', async () => Response.json(dense), signal);
  assert.equal(calendar.first, '2021-02-22');
  assert.equal(calendar.last, '2021-02-28');
  assert.equal(calendar.owed.get('2021-02-24'), 220);
  assert.deepEqual(calendar.from, ['BANKNIFTY', 'NIFTY']);
  assert.equal(calendar.why, '');
  const withheld = await readCalendar(
    'gdfl',
    async () =>
      Response.json({
        status: 'withheld',
        reason: 'gdfl holds no 1day bar for BANKNIFTY or NIFTY',
        days: '',
        withheld: []
      }),
    signal
  );
  assert.equal(withheld.owed.size, 0);
  assert.equal(withheld.why, 'gdfl holds no 1day bar for BANKNIFTY or NIFTY');
});
