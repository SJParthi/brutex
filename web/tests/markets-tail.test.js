// WP-61 (DB-041, DB-080): /markets reads a month file whole only while it is
// small enough to; past the bound it reads the newest bars, and says so.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { MONTH_READ_MAX, TAIL_PAGE, TAIL_PAGES, tailPlan } from '../src/lib/markets-tail.js';

test('a one-minute month is read whole; a one-second month is bounded, by size and not by name', () => {
  const minute = tailPlan([{ month: '2026-08', rows: 8_250 }, { month: '2026-09', rows: 8_625 }]);
  assert.deepEqual([minute.bounded, minute.from, minute.to, minute.pages, minute.biggest], [false, '2026-08', '2026-09', 0, 8_625]);
  const second = tailPlan([{ month: '2026-09', rows: 382_496 }]);
  assert.deepEqual([second.bounded, second.from, second.to, second.pages], [true, '2026-09', '2026-09', TAIL_PAGES]);
  assert.match(second.why, /3,82,496 bars/);
  assert.match(second.why, new RegExp(`newest\\s+${(TAIL_PAGE * TAIL_PAGES).toLocaleString('en-IN')} bars at most`));
  // The exact edge: the bound itself is still read whole.
  assert.equal(tailPlan([{ month: '2026-09', rows: MONTH_READ_MAX }]).bounded, false);
  assert.equal(tailPlan([{ month: '2026-09', rows: MONTH_READ_MAX + 1 }]).bounded, true);
});

test('empty, unordered, duplicate and malformed month lists', () => {
  assert.deepEqual(tailPlan([]), { bounded: false, from: '', to: '', pages: 0, biggest: 0, why: '' });
  assert.equal(tailPlan(/** @type {any} */ (null)).bounded, false);
  // ONE BIG MONTH AMONG SMALL ONES BOUNDS THE WHOLE READ, and the span is the
  // months' own whatever order they arrive in.
  const mixed = tailPlan([{ month: '2026-09', rows: 10 }, { month: '2026-07', rows: 400_000 }, { month: '2026-08', rows: 10 }, { month: '2026-08', rows: 10 }]);
  assert.deepEqual([mixed.bounded, mixed.from, mixed.to], [true, '2026-07', '2026-09']);
  assert.match(mixed.why, /2026-07 to 2026-09/);
  // A count that is absent, negative, fractional or text is not a size: it is
  // never read as a big month, and a row with no month is dropped.
  const bad = tailPlan(/** @type {any} */ ([
    { month: '2026-09', rows: -5 }, { month: '2026-09', rows: 1.5 }, { month: '2026-09', rows: '999999' },
    { month: '2026-09' }, { month: 'September', rows: 999_999 }, null
  ]));
  assert.deepEqual([bad.bounded, bad.biggest, bad.from, bad.to], [false, 0, '2026-09', '2026-09']);
});

test('the page reads through the plan and names what it drew', () => {
  const page = readFileSync(new URL('../src/routes/markets/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /tailPlan\(monthsAtBase\.filter\(/);
  assert.match(page, /if \(plan\.bounded\) \{\s*const got = await readTail\(row, plan, feed, rung, ticket\.signal\)/);
  assert.match(page, /readWindowPage\(q\.toString\(\), TAIL_PAGE, signal\)/);
  assert.match(page, /data-markets-tail=/);
});
