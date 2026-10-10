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

// ======================================================================
// THE LEDGER /db JUDGES COVERAGE WITH (WP-54, D-3336)
// ======================================================================

import {
  owedLedger,
  ledgerSpan,
  ledgerDay,
  monthSpan,
  monthsBetween,
  epochDayOfIso,
  calendarUrl,
  readLedger
} from '../src/lib/calendar-owed.js';

/** A synthetic `/calendar.json` body in the server's shape. */
function calBody(/** @type {Record<string, any>} */ over = {}) {
  return {
    status: 'ok',
    feed: 'zerodha',
    rung: '1min',
    from: epochDayOfIso('2024-03-01'),
    days: 'RP.RRRRR..',
    held: '1001111100',
    heldBy: ['NIFTY'],
    target: { kind: 'series', exchange: 'NSE', segment: 'INDEX', symbol: 'NIFTY' },
    owed: { rung: '1min', regular: 375, casEligible: 360 },
    special: [{ day: epochDayOfIso('2024-03-02'), code: 'P', owed: 105, indexOwed: 105 }],
    halts: [],
    ...over
  };
}

test('a ledger answers any span in two reads: sessions, owed per kind, held days', () => {
  const L = owedLedger(calBody());
  assert.equal(L.series?.symbol, 'NIFTY');
  const all = ledgerSpan(L, L.from, L.to);
  assert.equal(all.sessions, 7);
  assert.equal(all.index, 6 * 375 + 105);
  assert.equal(all.specials, 1);
  assert.equal(all.heldSessions, 6, 'the drill day is not held');
  // A sub-span, and the days outside the calendar are counted apart.
  const tail = ledgerSpan(L, epochDayOfIso('2024-03-08'), epochDayOfIso('2024-03-15'));
  assert.equal(tail.asked, 8);
  assert.equal(tail.inside, 3);
  assert.equal(tail.outside, 5);
  assert.equal(tail.sessions, 1);
  const none = ledgerSpan(L, epochDayOfIso('2025-01-01'), epochDayOfIso('2025-01-31'));
  assert.equal(none.inside, 0);
  assert.equal(none.sessions, 0);
  assert.equal(none.outside, 31);
  assert.deepEqual(ledgerDay(L, epochDayOfIso('2024-03-02')), {
    day: epochDayOfIso('2024-03-02'),
    code: 'P',
    held: 0,
    special: { day: epochDayOfIso('2024-03-02'), code: 'P', owed: 105, indexOwed: 105 },
    halt: null
  });
  assert.equal(ledgerDay(L, L.to + 1), null);
});

test('an auction day owes the eligible count to an eligible share only, and a special day with no count is unknown', () => {
  const L = owedLedger(calBody({ days: 'XXP', held: null, special: [{ day: epochDayOfIso('2024-03-03'), code: 'P', owed: 220, indexOwed: null }] }));
  const s = ledgerSpan(L, L.from, L.to);
  assert.equal(s.cas, 360 + 360 + 220);
  assert.equal(s.share, 375 + 375 + 220);
  assert.equal(s.index, 375 + 375);
  assert.equal(s.unknownIndex, 1);
  assert.equal(s.unknownShare, 0);
  assert.equal(s.auction, 2);
  assert.equal(L.series?.segment, 'INDEX');
});

test('a malformed calendar is refused by name, never folded into an empty one', () => {
  assert.throws(() => owedLedger({ status: 'withheld', reason: 'no NIFTY file', days: '' }), /no day string \(no NIFTY file\)/);
  assert.throws(() => owedLedger(calBody({ from: 'x' })), /no day string/);
  assert.throws(() => owedLedger(calBody({ held: '101' })), /held string is 3 days against 10/);
});

test('month spans, month lists and calendar URLs', () => {
  assert.deepEqual(monthSpan('2024-02'), { first: epochDayOfIso('2024-02-01'), last: epochDayOfIso('2024-02-29') });
  assert.deepEqual(monthsBetween('2023-11', '2024-02'), ['2023-11', '2023-12', '2024-01', '2024-02']);
  assert.deepEqual(monthsBetween('2024-03', '2024-02'), []);
  assert.equal(calendarUrl('gdfl', '1s'), '/calendar.json?feed=gdfl&rung=1s');
  assert.equal(
    calendarUrl('zerodha', '1min', { symbol: 'BAJAJ-AUTO', segment: 'CASH' }),
    '/calendar.json?feed=zerodha&rung=1min&symbol=BAJAJ-AUTO&segment=CASH'
  );
});

test('readLedger names a refusal, a calendar still building and a withheld one; it never throws but on abort', async () => {
  const signal = new AbortController().signal;
  const json = (/** @type {number} */ status, /** @type {any} */ b) =>
    new Response(JSON.stringify(b), { status, headers: { 'content-type': 'application/json' } });
  const ok = await readLedger('/calendar.json?feed=zerodha', async () => json(200, calBody()), signal);
  assert.equal(ok.why, '');
  assert.equal(ok.ledger?.regular, 375);
  const refused = await readLedger('/c', async () => json(400, { refused: 'no such feed: dhanx' }), signal);
  assert.equal(refused.ledger, null);
  assert.match(refused.why, /no such feed: dhanx/);
  const withheld = await readLedger('/c', async () => json(200, { status: 'withheld', reason: 'GDFL holds no 1min' }), signal);
  assert.match(withheld.why, /withheld: GDFL holds no 1min/);
  let asked = 0;
  const building = await readLedger(
    '/c',
    async () => (++asked, json(202, { status: 'building', reason: 'census read', retry_after_ms: 50 })),
    signal,
    async () => {}
  );
  assert.ok(asked > 1, 'a 202 is asked again');
  assert.match(building.why, /still building this calendar: census read/);
  const broken = await readLedger('/c', async () => json(200, { status: 'ok', days: 7 }), signal);
  assert.match(broken.why, /could not be read \(\/calendar.json carried no day string\)/);
});
