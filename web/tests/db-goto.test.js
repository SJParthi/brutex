// GO TO AN INSTANT ON /db (DB-005, R3-03).
//
// `goto.js` reads what the box typed and turns the server's answer into a
// landing. The server's `at=` rule is mirrored below from `bars.rs`
// (`at_index` and the offset it implies) so the landing is driven against the
// same arithmetic the route runs; `web/tests/db-goto.test.js` reads that Rust
// for the two lines it mirrors. The bars are an invented one-second contract
// with prices nobody could take for real; no vendor row is quoted.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parseGoto, gotoAsk, landing, GOTO_MAX } from '../src/lib/db/goto.js';
import { istInstant, pageOfIndex, windowQuery, seriesPlan } from '../src/lib/db/bars-plan.js';
import { dbSource } from './database-page-fixture.js';

const DAY = '2019-03-15';
const at = (/** @type {string} */ t) => /** @type {number} */ (istInstant(DAY, t));

/** Seconds of 10:00:00–10:04:59 that hold a bar: irregular, as a thin contract trades. */
const SECONDS = [];
for (let m = 0; m < 5; m += 1) for (const s of [3, 11, 17, 29, 41, 58]) SECONDS.push(at(`10:0${m}:${String(s).padStart(2, '0')}`));
const BARS = SECONDS.map((t) => ({ t, o: 9911, h: 9922, l: 9900, c: 9915 }));

/**
 * `/bars/window.json` with `at=` and no offset, as `bars.rs` answers it:
 * `at_index` is, ascending, the rows strictly before `at`; descending, the
 * total less the rows at or before it. The offset is that index, clamped to
 * the last row and rounded down to the page.
 * @param {number} instant @param {boolean} desc @param {number} limit
 */
function serve(instant, desc, limit) {
  const total = BARS.length;
  const before = BARS.filter((b) => b.t < (desc ? instant + 1 : instant)).length;
  const index = desc ? total - before : before;
  const last = Math.min(index, total - 1);
  const offset = last - (last % limit);
  const order = desc ? [...BARS].reverse() : BARS;
  return { total, offset, at_index: index, bars: order.slice(offset, offset + limit) };
}

const when = (/** @type {number} */ t) => {
  const d = new Date((t + 19_800) * 1000);
  return `${d.toISOString().slice(0, 10)} ${d.toISOString().slice(11, 19)}`;
};

test('the mirrored at= rule is the route’s own', () => {
  const rust = readFileSync(new URL('../../crates/api/src/bars.rs', import.meta.url), 'utf8');
  assert.match(rust, /let probe = if desc \{ at\.saturating_add\(1\) \} else \{ at \};/);
  assert.match(rust, /Ok\(if desc \{\s*total\.saturating_sub\(before\)\s*\} else \{\s*before\s*\}\)/);
  assert.match(rust, /usize::try_from\(last\)\.map_or\(0, \|last\| last - last % limit\.max\(1\)\)/);
});

test('go_to_a_minute_lands_on_its_first_bar_in_both_directions', () => {
  const asked = /** @type {any} */ (parseGoto('10:02', DAY));
  assert.deepEqual(asked, { at: at('10:02:00'), label: `${DAY} 10:02:00`, minute: true });
  // BEFORE R3-03, newest first asked `at` in the grid's own direction and the
  // server's rule there is "at or before": no bar opens at 10:02:00, so it
  // marked 10:01:58 — the minute before.
  assert.equal(serve(asked.at, true, 1).bars[0].t, at('10:01:58'), 'the defect, as the route answers it');
  // NOW ONE QUESTION, ASCENDING, WHATEVER THE DIRECTION.
  assert.deepEqual(gotoAsk(asked.at), { sort: 'ts', desc: false, at: asked.at, limit: 1 });
  const answer = serve(asked.at, false, 1);
  for (const desc of [false, true]) {
    for (const size of [8, 25, 7]) {
      const land = /** @type {any} */ (landing(answer, { ...asked, desc, size, when }));
      assert.equal(land.bar.t, at('10:02:03'), `the minute's first bar, ${desc ? 'newest' : 'oldest'} first`);
      const order = desc ? [...BARS].reverse() : BARS;
      assert.equal(order[land.index].t, at('10:02:03'), 'its row in the grid’s own order');
      assert.equal(land.page, pageOfIndex(land.index, size));
      const page = order.slice((land.page - 1) * size, land.page * size);
      assert.ok(page.some((b) => b.t === at('10:02:03')), 'and the page opened holds it');
      assert.match(land.why, /^The first bar of 2019-03-15 10:02 opens at 2019-03-15 10:02:03 IST, 3 second\(s\) into the minute — row \d+ of 30\.$/);
    }
  }
  // THE SECOND TIME FOR THE SAME MINUTE lands the same, and the grid marks the
  // row anew: the mark is cleared at the press and set again once the rows
  // are drawn, so landing on the row already marked still shows a landing.
  assert.deepEqual(landing(answer, { ...asked, desc: true, size: 8, when }), landing(answer, { ...asked, desc: true, size: 8, when }));
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  const script = grid.slice(0, grid.indexOf('</script>'));
  assert.match(script, /async function jumpToInstant\(at, label, key = '', minute = false\) \{\s*db\.landedRk = '';/);
  assert.match(script, /setTimeout\(\(\) => \{\s*db\.landedRk = row\.rk;/);
  assert.match(script, /windowQuery\(feed, p, \{ \.\.\.bounds, \.\.\.gotoAsk\(at\) \}\)/);
  assert.match(script, /anchorRow = land\.index;\s*db\.page = land\.page;/);
  // The anchor waits for the rows of the page it names.
  assert.match(script, /if \(want < 0 \|\| barState\.loading \|\| !box \|\| rows\.length === 0 \|\| pageShown !== pageNow\) return;/);
  // An instant to the second that holds a bar lands on it and says nothing.
  const exact = /** @type {any} */ (parseGoto(`${DAY} 10:03:29`, ''));
  assert.deepEqual(landing(serve(exact.at, false, 1), { ...exact, desc: true, size: 8, when }), {
    page: pageOfIndex(30 - 1 - 3 * 6 - 3, 8), index: 30 - 1 - (3 * 6 + 3), bar: BARS[3 * 6 + 3], why: ''
  });
  // A second with no bar names the next bar and how far.
  const sec = /** @type {any} */ (parseGoto('10:03:30', DAY));
  assert.match(/** @type {any} */ (landing(serve(sec.at, false, 1), { ...sec, desc: false, size: 8, when })).why,
    /^No bar opens at 2019-03-15 10:03:30\. Landed on the first bar after it, 2019-03-15 10:03:41 IST, 11 second\(s\) later — row 23 of 30\.$/);
  // A silent minute lands on the next bar after it and says so.
  const quiet = /** @type {any} */ (parseGoto('09:59', DAY));
  assert.match(/** @type {any} */ (landing(serve(quiet.at, false, 1), { ...quiet, desc: true, size: 8, when })).why,
    /^No bar opens in the minute 2019-03-15 09:59\. Landed on the first bar after it, 2019-03-15 10:00:03 IST, 63 second\(s\) later — row 30 of 30\.$/);
  // Past the end: the newest bar, said.
  const late = /** @type {any} */ (parseGoto('11:00', DAY));
  const past = /** @type {any} */ (landing(serve(late.at, false, 1), { ...late, desc: true, size: 8, when }));
  assert.deepEqual([past.index, past.page, past.bar.t], [0, 1, at('10:04:58')]);
  assert.match(past.why, /^No bar lies at or after 2019-03-15 11:00:00 in this query: its newest bar is 2019-03-15 10:04:58 IST\. Landed on it\.$/);
  assert.equal(/** @type {any} */ (landing(serve(late.at, false, 1), { ...late, desc: false, size: 8, when })).index, 29);
  // Nothing kept, or an answer that names no place, moves nothing.
  assert.deepEqual(landing({ total: 0, bars: [] }, { ...late, desc: true, size: 8, when }), { why: 'No bar is kept by this query, so there is nothing to land on.' });
  assert.match(/** @type {any} */ (landing({ total: 5, at_index: null, bars: [] }, { ...late, desc: true, size: 8, when })).why, /did not say where/);
  assert.match(/** @type {any} */ (landing(null, { ...late, desc: true, size: 8, when })).why, /No bar is kept/);
  assert.match(/** @type {any} */ (landing({ total: 5, at_index: 2, bars: 'x' }, { ...late, desc: true, size: 8, when })).why, /did not say where/);
  // The query the grid sends: ascending, the instant, no offset.
  const plan = seriesPlan([{ instrument: 'NSE-FNO-NIFTY-2019-03-28-1160000-CE', month: '2019-03', timeframe: '1s', rows: 30 }]);
  const q = new URLSearchParams(windowQuery('gdfl', plan, gotoAsk(asked.at)));
  assert.deepEqual([q.get('dir'), q.get('at'), q.get('offset'), q.get('limit')], ['asc', String(asked.at), null, '1']);
});

test('Go to takes a time alone, exact seconds, and refuses 1969, 2099 and nonsense in words', () => {
  const held = { lo: '2018-09-03', hi: '2026-09-30', what: 'this selection' };
  assert.deepEqual(parseGoto('10:02', DAY, held), { at: at('10:02:00'), label: `${DAY} 10:02:00`, minute: true });
  assert.deepEqual(parseGoto('9:15', DAY, held), { at: at('09:15:00'), label: `${DAY} 09:15:00`, minute: true });
  assert.deepEqual(parseGoto('9:15:07', DAY, held), { at: at('09:15:07'), label: `${DAY} 09:15:07`, minute: false });
  assert.deepEqual(parseGoto(' 2019-03-15T10:02:07 ', '', held), { at: at('10:02:07'), label: `${DAY} 10:02:07`, minute: false });
  const why = (/** @type {string} */ text, day = DAY) => /** @type {any} */ (parseGoto(text, day, held)).why ?? '';
  assert.match(why(''), /^Type an instant: a day and a time, 2019-03-15 10:02, or a time alone, 10:02, for the day on screen\.$/);
  assert.match(why('abc'), /^"abc" is not an instant\./);
  assert.match(why('1969-12-31 23:59:59'), /^31 Dec 1969 is before this selection's first day, 03 Sep 2018: no bar of it lies that early\. Pick a day from 03 Sep 2018 to 30 Sep 2026\.$/);
  assert.match(why('2099-01-01 10:00:00'), /^01 Jan 2099 is after this selection's last day, 30 Sep 2026: no bar of it lies that late\. Pick a day from 03 Sep 2018 to 30 Sep 2026\.$/);
  assert.match(why('2019-02-30 10:00:00'), /is not a real IST instant/);
  assert.match(why('25:61:61'), /is not a real IST instant/);
  assert.match(why('10:02', ''), /names no day and no bar is on screen/);
  const long = why('1'.repeat(6000));
  assert.match(long, /^Go to read 6,000 characters, and an instant is at most 32/);
  assert.ok(long.length < 200, 'never echoed');
  assert.equal(GOTO_MAX, 32);
  // A date window bounds it the same way, in its own name.
  assert.match(/** @type {any} */ (parseGoto('2019-03-14 10:00', DAY, { lo: '2019-03-15', hi: '2019-03-15', what: 'the date window' })).why,
    /^14 Mar 2019 is before the date window's first day, 15 Mar 2019/);
  // One end alone still bounds; no span is no bound.
  assert.match(/** @type {any} */ (parseGoto('2019-03-14 10:00', DAY, { lo: '2019-03-15' })).why, /Pick a day from 15 Mar 2019 to 15 Mar 2019\./);
  assert.match(/** @type {any} */ (parseGoto('2019-03-16 10:00', DAY, { hi: '2019-03-15' })).why, /Pick a day from 15 Mar 2019 to 15 Mar 2019\./);
  assert.equal(/** @type {any} */ (parseGoto('1969-12-31 23:59:59', DAY)).at, -19_801, 'with no span given, no bound');
  assert.equal(/** @type {any} */ (parseGoto(undefined, DAY)).why.startsWith('Type an instant'), true);
  // The grid reads the box and the picker through this, bounded by the window or the days held.
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  assert.match(grid, /const got = parseGoto\(text, dayOnScreen\(\), gotoSpan\(\)\);/);
  assert.match(grid, /const got = parseGoto\(String\(at \?\? ''\), dayOnScreen\(\), gotoSpan\(\)\);/);
  assert.match(grid, /: \{ lo: db\.heldDays\?\.lo \|\| '', hi: db\.heldDays\?\.hi \|\| '', what: 'this selection' \};/);
});

test('Go to at every boundary of what it reads, and every sentence it says, exactly', () => {
  const span = { lo: '2019-03-15', hi: '2019-03-15' };
  const why = (/** @type {unknown} */ text, day = DAY, o = {}) => /** @type {any} */ (parseGoto(text, day, o)).why;
  // Length: 32 characters are read, 33 are refused by their length.
  assert.equal(why('x'.repeat(33)), 'Go to read 33 characters, and an instant is at most 32: type a day and a time, 2019-03-15 10:02:07, or a time alone.');
  assert.equal(why('x'.repeat(32)), `"${'x'.repeat(32)}" is not an instant. Type a day and a time, 2019-03-15 10:02, or a time alone, 10:02, for the day on screen.`);
  // Anchored at both ends: nothing before the day, nothing after the time.
  assert.equal(why('x2019-03-15 10:02'), '"x2019-03-15 10:02" is not an instant. Type a day and a time, 2019-03-15 10:02, or a time alone, 10:02, for the day on screen.');
  assert.match(why('2019-03-15 10:02x'), /is not an instant/);
  assert.match(why('x10:02'), /is not an instant/);
  assert.match(why('10:02x'), /is not an instant/);
  assert.deepEqual(parseGoto('2019-03-15   10:02', ''), { at: at('10:02:00'), label: `${DAY} 10:02:00`, minute: true }, 'any run of spaces between day and time');
  // The span's own ends are inside it; the default name is "this selection".
  assert.equal(/** @type {any} */ (parseGoto('2019-03-15 10:02', '', span)).at, at('10:02:00'));
  assert.equal(why('2019-03-14 10:02', '', span), "14 Mar 2019 is before this selection's first day, 15 Mar 2019: no bar of it lies that early. Pick a day from 15 Mar 2019 to 15 Mar 2019.");
  assert.equal(why('2019-03-16 10:02', '', span), "16 Mar 2019 is after this selection's last day, 15 Mar 2019: no bar of it lies that late. Pick a day from 15 Mar 2019 to 15 Mar 2019.");
  // A place that is not a whole non-negative number moves nothing, even with a bar.
  const bar = { t: at('10:02:03') };
  const o = { at: at('10:02:00'), label: `${DAY} 10:02:00`, minute: true, desc: true, size: 8, when };
  for (const bad of [-1, 1.5, '3', undefined, Number.NaN]) {
    assert.deepEqual(landing({ total: 30, at_index: bad, bars: [bar] }, o), { why: 'The server did not say where that instant lies, so Go to did not move the grid.' }, String(bad));
  }
  // The minute's sixtieth second is the next minute: said as a silent minute.
  const next = { t: at('10:03:00') };
  assert.equal(/** @type {any} */ (landing({ total: 30, at_index: 12, bars: [next] }, o)).why,
    'No bar opens in the minute 2019-03-15 10:02. Landed on the first bar after it, 2019-03-15 10:03:00 IST, 60 second(s) later — row 18 of 30.');
  assert.equal(/** @type {any} */ (landing({ total: 30, at_index: 12, bars: [{ t: at('10:02:59') }] }, o)).why,
    'The first bar of 2019-03-15 10:02 opens at 2019-03-15 10:02:59 IST, 59 second(s) into the minute — row 18 of 30.');
});
