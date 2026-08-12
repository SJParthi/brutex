// THE /ingest PROGRESS METER'S NUMERATOR, over the request's own population.
//
// The bar read 100.0% while half the request was outstanding, because the fold
// counted every row in the window months and the denominator counted only the
// request's reach. This drives the fold directly.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { foldWindow, foldKey } from '../src/lib/fold.js';

/** The shared store's `byMonth` index, built the way `fold()` builds it. */
function index(cells) {
  const by = new Map();
  for (const cell of cells) {
    let bucket = by.get(cell.month);
    if (!bucket) by.set(cell.month, (bucket = { cells: 0, bars: 0, list: [] }));
    bucket.cells += 1;
    bucket.bars += cell.rows;
    bucket.list.push(cell);
  }
  return by;
}

/** The state the report reproduced: an equity backfill already in the month. */
function july() {
  const cells = [];
  for (let n = 0; n < 749; n += 1) {
    cells.push({
      instrument: `NSE-CASH-EQ${n}`,
      month: '2026-07',
      timeframe: '1min',
      rows: 8250
    });
  }
  cells.push({ instrument: 'NSE-INDEX-NIFTY', month: '2026-07', timeframe: '1min', rows: 4000 });
  return index(cells);
}

test('the ask sees only what the ask reaches', () => {
  const scope = {
    instruments: new Set(['NSE-INDEX-NIFTY', 'NSE-INDEX-BANKNIFTY']),
    rungs: new Set(['1min'])
  };
  const shot = foldWindow(july(), ['2026-07'], scope);

  // BEFORE: units was 750 against an `expectedUnits` of 2, `Math.min(1, …)`
  // pinned the meter at 100.0% and `unitsLeft` clamped to 0 — which removed the
  // ETA line too — with BANKNIFTY not yet fetched.
  assert.equal(shot.units, 1, 'one of the two indices has landed');
  assert.equal(shot.rows, 4000);
  assert.equal(shot.outside, 749, 'and the rest is counted, not silently dropped');
  assert.equal(shot.scoped, true);

  const expectedUnits = 2;
  const share = Math.min(1, shot.units / expectedUnits);
  assert.equal(share, 0.5, 'half the request is held, and the meter says half');
  assert.equal(Math.max(0, expectedUnits - shot.units), 1, 'and one is still outstanding');
});

test('the rung is part of the ask', () => {
  const by = index([
    { instrument: 'NSE-INDEX-NIFTY', month: '2026-07', timeframe: '1day', rows: 22 },
    { instrument: 'NSE-INDEX-NIFTY', month: '2026-07', timeframe: '1min', rows: 8250 }
  ]);
  const minute = foldWindow(by, ['2026-07'], {
    instruments: new Set(['NSE-INDEX-NIFTY']),
    rungs: new Set(['1min'])
  });
  assert.equal(minute.units, 1);
  assert.equal(minute.rows, 8250, 'a daily row is not progress on a minute request');
  assert.equal(minute.outside, 1);
});

test('no scope is every row, exactly as before', () => {
  const shot = foldWindow(july(), ['2026-07'], undefined);
  assert.equal(shot.units, 750);
  assert.equal(shot.outside, 0);
  assert.equal(shot.scoped, false);
});

test('a month the store does not hold contributes nothing and does not throw', () => {
  const shot = foldWindow(index([]), ['2026-07', '2026-08'], undefined);
  assert.deepEqual({ units: shot.units, rows: shot.rows, outside: shot.outside }, {
    units: 0,
    rows: 0,
    outside: 0
  });
});

test('the memo key carries the scope, so two scopes never share one answer', () => {
  const months = ['2026-07'];
  const all = foldKey(3, months, undefined);
  const swept = foldKey(3, months, {
    instruments: new Set(['NSE-INDEX-NIFTY']),
    rungs: new Set(['1min'])
  });
  const other = foldKey(3, months, {
    instruments: new Set(['NSE-INDEX-BANKNIFTY']),
    rungs: new Set(['1min'])
  });
  assert.notEqual(all, swept);
  assert.notEqual(swept, other);
  // ORDER-INDEPENDENT: the same set spelled in another order is the same key.
  assert.equal(
    foldKey(3, months, { instruments: new Set(['a', 'b']) }),
    foldKey(3, months, { instruments: new Set(['b', 'a']) })
  );
  // AND THE READ COUNTER STILL SEPARATES TWO READINGS OF ONE QUESTION.
  assert.notEqual(foldKey(4, months, undefined), all);
});
