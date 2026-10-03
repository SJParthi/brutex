import { test } from 'node:test';
import assert from 'node:assert/strict';

import { foldMinuteOwed, withheldDays } from '../src/lib/calendar-owed.js';

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

/** @param {number} day */
function isoOf(day) {
  return new Date(day * DAY_MS).toISOString().slice(0, 10);
}

test('withheld runs become every day they name, inclusive, and their months (D-1507)', () => {
  // 2024-02-01 is epoch day 19754; a leap February is 29 days.
  const { days, months } = withheldDays([{ from: 19754, to: 19782 }, { from: 19800, to: 19800 }], isoOf);
  assert.equal(days.size, 30);
  assert.ok(days.has('2024-02-01') && days.has('2024-02-29'), 'both ends are withheld');
  assert.ok(!days.has('2024-01-31') && !days.has('2024-03-01'), 'and nothing past them');
  assert.ok(days.has(isoOf(19800)), 'a one-day run is one day');
  assert.deepEqual([...months].sort(), ['2024-02', isoOf(19800).slice(0, 7)].sort());
});

test('an older API without withheld withholds nothing, and an empty list is the same', () => {
  for (const runs of [undefined, null, []]) {
    const { days, months } = withheldDays(runs, isoOf);
    assert.equal(days.size, 0);
    assert.equal(months.size, 0);
  }
});

test('a malformed withheld run is refused, never dropped back into holidays', () => {
  for (const runs of [
    'nope',
    [{ from: 10 }],
    [{ from: 10, to: 9 }],
    [{ from: 1.5, to: 2 }],
    [{ from: '10', to: '11' }],
    [null]
  ]) {
    assert.throws(() => withheldDays(runs, isoOf), /withheld/, JSON.stringify(runs));
  }
});
