// THE TWO /db COMPLETENESS DEFECTS, EACH WITH THE STATE THAT PRODUCED IT.
//
// `node --test web/tests/` — the runner is node's own, so this costs no
// dependency and no toolchain. It runs the same module `routes/db/+page.svelte`
// imports; nothing here is a copy of the page's arithmetic.
//
// Both cases below were REPRODUCED against the shipped page before the fix, by
// running its own expressions over these exact rows.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  denominators,
  denomKey,
  isSole,
  rollUpMonths,
  monthVerdict
} from '../src/lib/completeness.js';

/**
 * One decorated row, the way `deco` builds it, given the denominators.
 *
 * @param {import('../src/lib/completeness.js').Row & { instrument?: string }} row
 * @param {{ fullest: Map<string, number>, support: Map<string, number> }} d
 */
function deco(row, { fullest, support }) {
  const denom = fullest.get(denomKey(row.month, row.timeframe)) ?? row.rows;
  const sole = isSole(support, row);
  return {
    ...row,
    denom,
    sole,
    short: Math.max(0, denom - row.rows),
    pct: sole ? null : denom > 0 ? row.rows / denom : 1
  };
}

/**
 * @param {(import('../src/lib/completeness.js').Row & { instrument?: string })[]} rows
 */
function decorate(rows) {
  const d = denominators(rows);
  return rows.map((r) => deco(r, d));
}

/**
 * The one `find` result a test may use, and it is not the same as `find`.
 *
 * `Array.prototype.find` returns `T | undefined`, so every `assert.equal` on the
 * result below reads a property off a possibly-absent object. When the row IS
 * absent the test still fails — but with "cannot read property of undefined",
 * which names the assertion's own bug rather than the fixture's. This turns the
 * absence into its own failure, with the key that was missing in the message.
 *
 * @template T
 * @param {T[]} rows
 * @param {(row: T) => boolean} match
 * @param {string} what
 * @returns {T}
 */
function pick(rows, match, what) {
  const found = rows.find(match);
  assert.ok(found, `the fixture holds no ${what}`);
  return found;
}

test('a month held at two rungs reads the same whichever row arrives first', () => {
  // 2021-08 holding NIFTY@1day=23, NIFTY@1min=8,625 and BANKNIFTY@1min=8,625.
  // Every row IS the fullest at its own rung, so the month is genuinely
  // complete — `routes/+page.svelte` states outright that a month held at two
  // rungs is "a legitimate state of the store, not a fault".
  const daily = { instrument: 'NSE-INDEX-NIFTY', month: '2021-08', timeframe: '1day', rows: 23 };
  const minute = { instrument: 'NSE-INDEX-NIFTY', month: '2021-08', timeframe: '1min', rows: 8625 };
  const bank = { instrument: 'NSE-INDEX-BANKNIFTY', month: '2021-08', timeframe: '1min', rows: 8625 };

  const dailyFirst = rollUpMonths(decorate([daily, minute, bank]))[0];
  const minuteFirst = rollUpMonths(decorate([minute, bank, daily]))[0];

  // THE DEFECT: `fullest` was snapshot from the first row of the month and
  // multiplied by the row count, so the daily-first order gave 17,273 / (3 ×
  // 23) = 25,033.33% and the minute-first order gave 66.75%.
  assert.equal(dailyFirst.pct, minuteFirst.pct, 'the same month, two row orders, one answer');
  assert.equal(dailyFirst.pct, 1, 'and the answer is that the month is complete');
  assert.equal(dailyFirst.state, 'full');
  assert.equal(minuteFirst.state, 'full');
  assert.equal(dailyFirst.owed, 23 + 8625 + 8625, 'each row against its OWN rung');
  assert.equal(dailyFirst.missing, 0);
  // AND THE MONTH HOLDS TWO RUNGS, so there is no single "fullest" figure and
  // no single session size. `tf === false` is what the page draws a dash from.
  assert.equal(dailyFirst.tf, false);
  assert.equal(minuteFirst.tf, false);
});

test('a month whose only row is short is not full — it is not comparable', () => {
  // The store holds 2026-07 1min = 8,250 (complete) and 2026-08 1min = 1 bar:
  // the month a backfill is landing in, or the first month ever pulled.
  const rows = [
    { instrument: 'NSE-INDEX-NIFTY', month: '2026-07', timeframe: '1min', rows: 8250 },
    { instrument: 'NSE-INDEX-BANKNIFTY', month: '2026-07', timeframe: '1min', rows: 8250 },
    { instrument: 'NSE-INDEX-NIFTY', month: '2026-08', timeframe: '1min', rows: 1 }
  ];
  const decorated = decorate(rows);
  const august = pick(decorated, (r) => r.month === '2026-08', 'row for 2026-08');

  // THE DEFECT: denom = its own 1 row, short = 0, state = 'full', pct = 100%.
  assert.equal(august.sole, true);
  assert.equal(august.short, 0, 'it is still not short of anything — that is the point');
  assert.equal(august.pct, null, 'and there is no ratio to print');

  const cards = rollUpMonths(decorated);
  const augustCard = pick(cards, (m) => m.month === '2026-08', 'card for 2026-08');
  assert.equal(augustCard.state, 'sole', 'not `full`');
  assert.equal(augustCard.pct, null, 'not 100.00%');
  assert.equal(augustCard.full, 0, 'and it is counted as complete nowhere');
  assert.equal(augustCard.unverified, 1);

  // THE CORROBORATED MONTH IS UNAFFECTED.
  const july = pick(cards, (m) => m.month === '2026-07', 'card for 2026-07');
  assert.equal(july.state, 'full');
  assert.equal(july.pct, 1);
  assert.equal(july.full, 2);
});

test('a store holding one bar does not report itself complete', () => {
  const [only] = decorate([
    { instrument: 'NSE-INDEX-NIFTY', month: '2026-08', timeframe: '1min', rows: 1 }
  ]);
  assert.equal(only.sole, true);
  assert.equal(only.pct, null);
  const [card] = rollUpMonths([only]);
  assert.equal(card.state, 'sole');
  assert.equal(card.pct, null);
  assert.equal(card.full, 0);
});

test('two instruments in one month at one rung are still judged against each other', () => {
  // The control case: this was already caught before the fix and must stay
  // caught. 8,250 against 1 is a hole, not an unknown.
  const decorated = decorate([
    { instrument: 'NSE-INDEX-NIFTY', month: '2026-08', timeframe: '1min', rows: 8250 },
    { instrument: 'NSE-INDEX-BANKNIFTY', month: '2026-08', timeframe: '1min', rows: 1 }
  ]);
  const short = pick(decorated, (r) => r.instrument === 'NSE-INDEX-BANKNIFTY', 'BANKNIFTY row');
  assert.equal(short.sole, false);
  assert.equal(short.short, 8249);
  // `pct` IS `number | null` AND THE NULL IS THE OTHER TEST'S SUBJECT. A `<`
  // against null coerces to 0 and passes, so the comparison alone would hold
  // for the very state — a sole row with nothing to compare against — that this
  // case exists to distinguish from a hole. The type is asserted first.
  assert.equal(typeof short.pct, 'number', 'a corroborated row HAS a ratio');
  assert.ok(Number(short.pct) < 0.001);
  const [card] = rollUpMonths(decorated);
  assert.equal(card.state, 'gap');
  assert.equal(card.unverified, 0);
});

test('a month verdict over an empty set claims nothing', () => {
  // Unreachable from the roll-up — a card exists because a row made it — and
  // written down anyway: no row is no evidence, which is `sole` and a dash,
  // never `full` and 100%.
  assert.deepEqual(monthVerdict({ n: 0, bars: 0, owed: 0, missing: 0, unverified: 0 }), {
    pct: null,
    state: 'sole'
  });
  assert.deepEqual(rollUpMonths([]), []);
  assert.deepEqual(rollUpMonths(undefined), []);
});
