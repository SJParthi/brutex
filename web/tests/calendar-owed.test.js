import { test } from 'node:test';
import assert from 'node:assert/strict';

import { foldMinuteOwed } from '../src/lib/calendar-owed.js';

const DAY_MS = 86_400_000;

/** @param {string} iso @param {number} n */
function addDay(iso, n) {
  return new Date(Date.parse(`${iso}T00:00:00Z`) + n * DAY_MS).toISOString().slice(0, 10);
}

test('an open exchange day with an unknown index count withholds the whole index month', () => {
  const exchange = new Map([
    ['2021-02-24', 220],
    ['2021-02-25', 375]
  ]);
  const index = new Map([
    ['2021-02-24', null],
    ['2021-02-25', 375]
  ]);

  const folded = foldMinuteOwed('2021-02-24', '2021-02-25', exchange, index, addDay);
  assert.equal(folded.exchange.get('2021-02'), 595, 'the market timetable remains visible');
  assert.equal(folded.index.get('2021-02'), 375, 'known index days remain inspectable');
  assert.ok(
    folded.indexUnknown.has('2021-02'),
    'but the partial sum is forbidden as a month denominator'
  );
});

test('an older payload without indexOwed degrades to unknown instead of copying exchange owed', () => {
  const folded = foldMinuteOwed(
    '2024-01-02',
    '2024-01-02',
    new Map([['2024-01-02', 375]]),
    new Map(),
    addDay
  );
  assert.equal(folded.exchange.get('2024-01'), 375);
  assert.equal(folded.index.get('2024-01'), undefined);
  assert.ok(folded.indexUnknown.has('2024-01'));
});

test('a session whose exchange length is itself unknown is not promoted to an index claim', () => {
  const folded = foldMinuteOwed(
    '2024-11-01',
    '2024-11-01',
    new Map([['2024-11-01', null]]),
    new Map([['2024-11-01', null]]),
    addDay
  );
  assert.equal(folded.exchange.size, 0);
  assert.equal(folded.index.size, 0);
  assert.equal(folded.indexUnknown.size, 0, 'no known exchange count was partially folded');
});
