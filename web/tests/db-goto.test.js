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
import { parse } from 'svelte/compiler';
import { parseGoto, gotoAsk, landing, GOTO_MAX, gotoTurns, NO_LANDING } from '../src/lib/db/goto.js';
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
  assert.match(script, /async function jumpToInstant\(at, label, key = '', minute = false\) \{\s*(?:\/\*[\s\S]*?\*\/\s*)?const ticket = gotoTurn\.ask\(setKey\);\s*jumping = false;\s*db\.landedRk = '';/);
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

/**
 * THE GO TO MINUTE PICKER, AS THE PAGE WIRES IT (R3-03 repair 1). Its
 * `selected` and `onchange` are cut out of `DbControls.svelte`, and the
 * grid's `jumpToTime` out of `DbBarsGrid.svelte`, and run over a plain `db`.
 * The picker is single, so its rows are radios, and a radio already checked
 * fires no `change` when it is clicked: `pick` refuses exactly that, which is
 * what the browser does. A `jumpToInstant` stub stands for the read.
 *
 * @param {{dayRung?: boolean}} [o]
 */
function minutePicker(o = {}) {
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  /** @type {any} */
  let picker = null;
  /** @param {any} node */
  const walk = (node) => {
    if (picker || !node || typeof node !== 'object') return;
    if (node.type === 'Component' && node.name === 'Picker') {
      const on = node.attributes.find((/** @type {any} */ a) => a.name === 'onchange');
      if (on && controls.slice(on.value.expression.start, on.value.expression.end).includes('jumpToTime')) {
        picker = node;
        return;
      }
    }
    for (const [key, value] of Object.entries(node)) {
      if (key === 'parent' || key === 'loc' || key === 'metadata') continue;
      if (Array.isArray(value)) value.forEach(walk);
      else if (value && typeof value === 'object') walk(value);
    }
  };
  walk(parse(controls, { modern: true }).fragment);
  if (!picker) throw new Error('the Go to minute picker moved out of DbControls.svelte');
  const attr = (/** @type {string} */ name) => {
    const a = picker.attributes.find((/** @type {any} */ x) => x.name === name);
    return controls.slice(a.value.expression.start, a.value.expression.end);
  };
  const selected = new Function('db', `return (${attr('selected')});`);
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  /** @type {any} */
  let fn = null;
  /** @param {any} node */
  const find = (node) => {
    if (fn || !node || typeof node !== 'object') return;
    if (node.type === 'FunctionDeclaration' && node.id?.name === 'jumpToTime') {
      fn = node;
      return;
    }
    for (const [key, value] of Object.entries(node)) {
      if (key === 'parent' || key === 'loc' || key === 'metadata') continue;
      if (Array.isArray(value)) value.forEach(find);
      else if (value && typeof value === 'object') find(value);
    }
  };
  find(parse(grid, { modern: true }));
  if (!fn) throw new Error('jumpToTime moved out of DbBarsGrid.svelte');
  /** @type {{at: number, label: string, minute: boolean}[]} */
  const reads = [];
  /* `goTime` is watched: each time it is cleared, how many renders had run and
     how many reads had come back. A render is the `tick` the grid awaits,
     flushed one turn later as Svelte does; a read comes back two turns after it
     is asked. */
  let renders = 0;
  let landed = 0;
  /** How many times a refusal stopped whatever Go to was out. */
  let stops = 0;
  /** @type {number[][]} */
  const clearedAfter = [];
  let goTime = '';
  const db = /** @type {any} */ ({
    get goTime() { return goTime; },
    set goTime(v) { if (v === '' && goTime !== '') clearedAfter.push([renders, landed]); goTime = v; },
    page: 1, timeframe: o.dayRung ? '1day' : '1s', jumpToTime: null
  });
  const tick = () => new Promise((done) => setTimeout(() => { renders += 1; done(undefined); }, 0));
  const jumpToInstant = async (/** @type {number} */ instant, /** @type {string} */ label, _key = '', minute = false) => {
    reads.push({ at: instant, label, minute });
    for (let turn = 0; turn < 2; turn += 1) await new Promise((done) => setTimeout(done, 0));
    landed += 1;
    db.page = 9;
  };
  const made = new Function('db', 'parseGoto', 'dayOnScreen', 'gotoSpan', 'jumpToInstant', 'tick', 'dayRung', 'stopGoto',
    `let jumpWhy = ''; ${grid.slice(fn.start, fn.end)}; return { jumpToTime, why: () => jumpWhy };`)(
    db, parseGoto, () => DAY, () => ({ lo: DAY, hi: DAY, what: 'this selection' }), jumpToInstant, tick, Boolean(o.dayRung), () => { stops += 1; });
  db.jumpToTime = made.jumpToTime;
  const onchange = new Function('db', `return (${attr('onchange')});`)(db);
  /** A click on the minute's radio: `false` when it was already checked, so no change fired. */
  const pick = (/** @type {string} */ key) => {
    if (selected(db).has(key)) return false;
    onchange(new Set([key]));
    return true;
  };
  const settle = async () => {
    for (let turn = 0; turn < 6; turn += 1) await new Promise((done) => setTimeout(done, 0));
  };
  return { db, reads, pick, settle, clearedAfter, why: made.why, summary: attr('summary'), stops: () => stops };
}

test('the_minute_picker_lands_again_on_the_minute_it_already_landed_on', async () => {
  const p = minutePicker();
  assert.equal(p.pick('10:02'), true);
  await p.settle();
  assert.deepEqual(p.reads, [{ at: at('10:02:00'), label: `${DAY} 10:02:00`, minute: true }]);
  assert.equal(p.db.page, 9);
  // BEFORE THE REPAIR: `db.goTime` kept '10:02', the picker's radio for 10:02
  // stayed checked, and a click on it fired nothing — no read, no mark, the
  // grid left where the First page button put it.
  p.db.page = 1;
  assert.equal(p.pick('10:02'), true, 'after First page, the same minute is offered again');
  await p.settle();
  assert.equal(p.reads.length, 2, 'and it is asked again: one at= read');
  assert.deepEqual(p.reads[1], p.reads[0]);
  assert.equal(p.db.page, 9, 'and the grid is back on its page');
  // On the same page too: the read is asked and the row marked again.
  assert.equal(p.pick('10:02'), true);
  await p.settle();
  assert.equal(p.reads.length, 3);
  // A different minute, then the first one again.
  assert.equal(p.pick('10:03'), true);
  await p.settle();
  assert.equal(p.pick('10:02'), true);
  await p.settle();
  assert.deepEqual(p.reads.map((r) => r.at), [at('10:02:00'), at('10:02:00'), at('10:02:00'), at('10:03:00'), at('10:02:00')]);
  assert.equal(p.db.goTime, '', 'a Go to is spent once it is used: the picker holds no minute');
  assert.equal(p.summary, "db.goTime || 'Pick a minute'", 'so its summary asks for the next one');
  // Each clear comes after the render that drew the pick: cleared in the same
  // turn, Svelte would see no change and leave the browser's tick on the radio.
  // And only once its read is back: until then the summary names the minute
  // the grid is moving to.
  assert.deepEqual(p.clearedAfter, [[1, 1], [2, 2], [3, 3], [4, 4], [5, 5]]);
});

test('a minute the picker refuses is offered again too', async () => {
  // A day rung has no minute to choose: refused in words, and the radio is
  // not left checked, so the next click is read and said again.
  const p = minutePicker({ dayRung: true });
  assert.equal(p.pick('10:02'), true);
  await p.settle();
  assert.equal(p.reads.length, 0, 'nothing is asked of the server');
  assert.equal(p.why(), 'Every bar at 1day opens at the same minute, so there is no minute to choose between them.');
  assert.equal(p.db.goTime, '');
  assert.equal(p.pick('10:02'), true, 'the same minute is offered again');
  await p.settle();
  assert.equal(p.reads.length, 0);
  assert.deepEqual(p.clearedAfter, [[1, 0], [2, 0]], 'each refusal is cleared after its own render');
  assert.equal(p.stops(), 2, 'and each refusal stops whatever Go to was still out, so it cannot land after the words');
});

test('an end of the span the grid does not know is no end, and a number typed is read as its text', () => {
  // The grid's span gives '' for an end or a name it does not have
  // (`gotoSpan`: `db.heldDays?.hi || ''`). Before the repair that read
  // "Pick a day from 15 Mar 2019 to ." and "after 's last day".
  assert.equal(/** @type {any} */ (parseGoto('2019-03-14 10:00', DAY, { lo: DAY, hi: '', what: 'this selection' })).why,
    "14 Mar 2019 is before this selection's first day, 15 Mar 2019: no bar of it lies that early. Pick a day from 15 Mar 2019 to 15 Mar 2019.");
  assert.equal(/** @type {any} */ (parseGoto('2019-03-16 10:00', DAY, { lo: '', hi: DAY, what: '' })).why,
    "16 Mar 2019 is after this selection's last day, 15 Mar 2019: no bar of it lies that late. Pick a day from 15 Mar 2019 to 15 Mar 2019.");
  assert.equal(/** @type {any} */ (parseGoto('2019-03-14 10:00', DAY, { lo: DAY, hi: '2019-03-20', what: '' })).why,
    "14 Mar 2019 is before this selection's first day, 15 Mar 2019: no bar of it lies that early. Pick a day from 15 Mar 2019 to 20 Mar 2019.");
  // A number is read as what it prints: 0 is not an instant, not an empty box.
  assert.equal(/** @type {any} */ (parseGoto(0, DAY)).why,
    '"0" is not an instant. Type a day and a time, 2019-03-15 10:02, or a time alone, 10:02, for the day on screen.');
  assert.equal(/** @type {any} */ (parseGoto(null, DAY)).why.startsWith('Type an instant'), true);
});

test('a_go_to_lands_only_for_the_question_still_on_screen', () => {
  // `gotoTurns` alone: the latest Go to, for the row set it was asked of.
  const turns = gotoTurns();
  const a = turns.ask('set-1');
  assert.equal(turns.current(a, 'set-1'), true);
  assert.equal(turns.current(a, 'set-2'), false, 'another row set is not this answer’s');
  const b = turns.ask('set-1');
  assert.equal(a.signal.aborted, true, 'a later Go to stops the read before it');
  assert.equal(b.signal.aborted, false);
  assert.deepEqual([turns.mine(a), turns.current(a, 'set-1'), turns.mine(b), turns.current(b, 'set-1')], [false, false, true, true]);
  turns.drop();
  assert.equal(b.signal.aborted, true, 'a moved row set stops the Go to that is out');
  assert.deepEqual([turns.mine(b), turns.current(b, 'set-1')], [false, false]);
  turns.drop();
  const c = turns.ask('set-2');
  assert.equal(turns.current(c, 'set-2'), true, 'and the next Go to is asked as ever');
});

/**
 * THE GRID'S OWN GO TO, CUT OUT OF `DbBarsGrid.svelte` (R3-03 repair 2):
 * `gotoTurn`, `stopGoto`, `jumpToInstant` and `dropGoto` run over a plain
 * scope whose reads the test answers by hand, late or never.
 */
function gridGoto() {
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  /** @type {Record<string, any>} */
  const found = {};
  /** @param {any} node */
  const walk = (node) => {
    if (!node || typeof node !== 'object') return;
    if (node.type === 'FunctionDeclaration' && ['jumpToInstant', 'dropGoto'].includes(node.id?.name)) found[node.id.name] = node;
    if (node.type === 'VariableDeclaration' && ['gotoTurn', 'stopGoto', 'dayOnScreen'].includes(node.declarations?.[0]?.id?.name)) found[node.declarations[0].id.name] = node;
    for (const [key, value] of Object.entries(node)) {
      if (key === 'parent' || key === 'loc' || key === 'metadata') continue;
      if (Array.isArray(value)) value.forEach(walk);
      else if (value && typeof value === 'object') walk(value);
    }
  };
  walk(parse(grid, { modern: true }));
  for (const name of ['gotoTurn', 'stopGoto', 'dayOnScreen', 'jumpToInstant', 'dropGoto']) {
    if (!found[name]) throw new Error(`${name} is not in DbBarsGrid.svelte`);
  }
  const src = (/** @type {string} */ name) => grid.slice(found[name].start, found[name].end);
  /** @type {{at: number, signal: AbortSignal, resolve: (a: any) => void, reject: (e: any) => void}[]} */
  const reads = [];
  const scope = /** @type {any} */ ({
    db: { landedRk: 'earlier', page: 1, pageSize: 10, toDay: '', fromDay: '' }, barPage: [],
    jumpWhy: 'earlier', jumping: false, pinned: false, anchorRk: '', anchorRow: -1, barSortKey: 'ts', barDesc: true,
    setKey: 'set-1', plan: { keys: ['NSE-FNO-ALPHA-2019-03-28-1160000-CE'], why: null }, feeds: { active: 'gdfl' },
    barState: { answer: null, loading: true, error: null, set: '' }, pageTotal: 0,
    sortNow: { key: 'ts', desc: true }, bounds: {},
    readWindowPage: (/** @type {string} */ q, /** @type {number} */ _n, /** @type {AbortSignal} */ signal) =>
      new Promise((resolve, reject) => reads.push({ at: Number(new URLSearchParams(q).get('at')), signal, resolve, reject })),
    windowQuery: (/** @type {string} */ _feed, /** @type {any} */ _p, /** @type {any} */ o) => `at=${o.at}`,
    gotoAsk, landing, NO_LANDING, gotoTurns,
    dayLabel: () => '2019-03-15', barTimeLabel: () => '10:02:03', tick: () => Promise.resolve(), rowKey: (/** @type {string} */ k, /** @type {number} */ t) => `${k}@${t}`
  });
  const made = new Function('scope', `with (scope) { ${src('gotoTurn')} ${src('stopGoto')} ${src('dayOnScreen')} return { jumpToInstant: ${src('jumpToInstant')}, dropGoto: ${src('dropGoto')}, dayOnScreen }; }`)(scope);
  /** An answer `at_index` places in a 300-row query. @param {number} t @param {number} i */
  const answer = (t, i) => ({ total: 300, at_index: i, bars: [{ t }] });
  return { scope, reads, answer, go: (/** @type {number} */ t, key = '') => made.jumpToInstant(t, `instant ${t}`, key), dropGoto: made.dropGoto, dayOnScreen: made.dayOnScreen };
}

test('a late Go to answer lands nowhere: a later Go to, a moved row set, a failure of a stopped one', { timeout: 20_000 }, async () => {
  // TWO QUICK PICKS, THE FIRST ANSWER LATE (the second agent's attack: 10:02
  // held 900 ms, then 10:04). Before the repair the grid ended on 10:02.
  let g = gridGoto();
  const first = g.go(1000);
  const second = g.go(2000);
  assert.equal(g.reads.length, 2);
  assert.equal(g.reads[0].signal.aborted, true, 'the first read is stopped');
  g.reads[1].resolve(g.answer(2000, 200));
  await second;
  assert.equal(g.scope.db.page, 10, 'the later pick lands: row 100 of 300 newest first, page 10');
  assert.equal(g.scope.jumping, false);
  g.reads[0].resolve(g.answer(1000, 10)); // an answer that ignored its stop and came back after
  await first;
  assert.equal(g.scope.db.page, 10, 'and the earlier answer, late, moves nothing');
  assert.equal(g.scope.anchorRow, 99);
  assert.equal(g.scope.jumpWhy, '', 'the later instant is a bar of its own, so nothing is said, and the earlier one said nothing either');

  // A STOPPED READ'S FAILURE SAYS NOTHING; THE LATEST ONE'S IS SAID.
  g = gridGoto();
  const stopped = g.go(1000);
  const latest = g.go(2000);
  g.reads[0].reject(new Error('The operation was aborted.'));
  await stopped;
  assert.equal(g.scope.jumpWhy, '', 'no word for a Go to the reader replaced');
  assert.equal(g.scope.jumping, true, 'and the later one is still finding its page');
  g.reads[1].reject(new Error('Failed to fetch'));
  await latest;
  assert.equal(g.scope.jumpWhy, 'Go to could not be answered: Failed to fetch');
  assert.equal(g.scope.jumping, false);

  // THE ROW SET MOVES WHILE THE ANSWER IS OUT (a Side change 150 ms after a
  // Go to): before the effect has even run, the answer is not this screen's.
  g = gridGoto();
  let out = g.go(1000);
  g.scope.setKey = 'set-2';
  g.reads[0].resolve(g.answer(1000, 10));
  await out;
  assert.deepEqual([g.scope.db.page, g.scope.anchorRow, g.scope.pinned, g.scope.jumpWhy], [1, -1, false, ''], 'nothing lands on the new set');
  // And once it has: the read is stopped, and the line and the mark go too.
  g = gridGoto();
  out = g.go(1000);
  assert.equal(g.scope.jumping, true);
  g.scope.setKey = 'set-2';
  g.dropGoto();
  assert.equal(g.reads[0].signal.aborted, true);
  g.reads[0].resolve(g.answer(1000, 10));
  await out;
  assert.deepEqual([g.scope.db.page, g.scope.jumping, g.scope.jumpWhy, g.scope.db.landedRk, g.scope.anchorRow, g.scope.pinned], [1, false, '', '', -1, false]);
  // A landing already made on the old set is not carried onto the new one.
  g = gridGoto();
  out = g.go(1000);
  g.reads[0].resolve(g.answer(1000, 10));
  await out;
  assert.deepEqual([g.scope.db.page, g.scope.pinned, g.scope.anchorRow], [29, true, 289]);
  g.scope.db.landedRk = 'row 290';
  g.scope.setKey = 'set-2';
  g.dropGoto();
  assert.deepEqual([g.scope.pinned, g.scope.anchorRow, g.scope.anchorRk, g.scope.jumpWhy, g.scope.db.landedRk], [false, -1, '', '', '']);

  // NOTHING TO ASK OF: no feed, no series, or a plan the page refused. Each
  // is said, nothing is read, and the Go to before it is stopped.
  for (const [what, set, says] of /** @type {[string, (sc: any) => void, string][]} */ ([
    ['no feed', (sc) => { sc.feeds = { active: null }; }, 'No selection is read, so there is nothing to go to.'],
    ['no series', (sc) => { sc.plan = { keys: [], why: null }; }, 'No selection is read, so there is nothing to go to.'],
    ['a refused plan', (sc) => { sc.plan = { keys: ['NSE-FNO-ALPHA-2019-03-28-1160000-CE'], why: 'the plan names no rung' }; }, 'the plan names no rung']
  ])) {
    g = gridGoto();
    const before = g.go(1000);
    set(g.scope);
    await g.go(2000);
    assert.deepEqual([g.reads.length, g.scope.jumpWhy, g.scope.jumping], [1, says, false], what);
    assert.equal(g.reads[0].signal.aborted, true, `${what}: the Go to before it is stopped`);
    g.reads[0].resolve(g.answer(1000, 10));
    await before;
    assert.equal(g.scope.db.page, 1, `${what}: and lands nowhere`);
  }

  // A PLAN REFUSED WITH NO WORDS still says why nothing is read.
  g = gridGoto();
  g.scope.plan = { keys: [], why: '' };
  await g.go(1000);
  assert.equal(g.scope.jumpWhy, 'No selection is read, so there is nothing to go to.');

  // UNDER A VALUE SORT a Go to puts the grid back in time order, newest
  // first, and lands there; the line says so.
  g = gridGoto();
  g.scope.sortNow = { key: 'c', desc: false };
  g.scope.barSortKey = 'c';
  g.scope.barDesc = false;
  out = g.go(2000);
  g.reads[0].resolve(g.answer(2000, 200));
  await out;
  assert.deepEqual([g.scope.barSortKey, g.scope.barDesc, g.scope.db.page, g.scope.anchorRow], ['ts', true, 10, 99]);
  assert.match(g.scope.jumpWhy, /^The grid is back in time order, which is the order an instant has a place in.$/);
  // A re-sort's render that a later Go to overtakes lands nothing either.
  g = gridGoto();
  g.scope.sortNow = { key: 'c', desc: false };
  g.scope.tick = () => new Promise((done) => setTimeout(done, 5));
  out = g.go(2000);
  g.reads[0].resolve(g.answer(2000, 200));
  await Promise.resolve();
  await Promise.resolve();
  const overtaking = g.go(3000);
  await out;
  assert.deepEqual([g.scope.db.page, g.scope.anchorRow, g.scope.pinned], [1, -1, false], 'the Go to overtaken during its re-sort lands nowhere');
  g.reads[1].resolve(g.answer(3000, 0));
  await overtaking;
  assert.equal(g.scope.db.page, 30, 'the later one lands: row 300 of 300 newest first');

  // A SERIES NAMED (an extreme): its row is meant only when the bar is at
  // the instant itself.
  g = gridGoto();
  out = g.go(2000, 'NSE-INDEX-ALPHA');
  g.reads[0].resolve(g.answer(2000, 200));
  await out;
  assert.equal(g.scope.anchorRk, 'NSE-INDEX-ALPHA@2000');
  g = gridGoto();
  out = g.go(2000, 'NSE-INDEX-ALPHA');
  g.reads[0].resolve(g.answer(2007, 200));
  await out;
  assert.equal(g.scope.anchorRk, '', 'a later bar is not that series’ bar at the instant');
  g = gridGoto();
  out = g.go(2000);
  g.reads[0].resolve(g.answer(2000, 200));
  await out;
  assert.equal(g.scope.anchorRk, '', 'and with no series named, none is meant');

  // THE DAY A TIME ALONE IS READ ON: the first row on screen, else the
  // window's last day, else its first.
  g = gridGoto();
  assert.equal(g.dayOnScreen(), '');
  g.scope.db.fromDay = '2019-03-01';
  assert.equal(g.dayOnScreen(), '2019-03-01');
  g.scope.db.toDay = '2019-03-15';
  assert.equal(g.dayOnScreen(), '2019-03-15');
  g.scope.db.fromDay = '';
  assert.equal(g.dayOnScreen(), '2019-03-15');
  g.scope.barPage = [{ day: '2019-03-08' }];
  assert.equal(g.dayOnScreen(), '2019-03-08');

  // AN ANSWER ALREADY KNOWN TO HOLD NO BAR IS NOT ASKED AGAIN.
  g = gridGoto();
  g.scope.barState = { answer: { total: 0 }, loading: false, error: null, set: 'set-1' };
  await g.go(1000);
  assert.equal(g.reads.length, 0, 'no at= read');
  assert.equal(g.scope.jumpWhy, NO_LANDING);
  assert.equal(g.scope.jumping, false);
  // Known for ANOTHER set, or still loading, or failed: asked.
  for (const barState of [
    { answer: { total: 0 }, loading: false, error: null, set: 'set-0' },
    { answer: { total: 0 }, loading: true, error: null, set: 'set-1' },
    { answer: { total: 0 }, loading: false, error: 'refused', set: 'set-1' },
    { answer: null, loading: false, error: null, set: 'set-1' }
  ]) {
    g = gridGoto();
    g.scope.barState = barState;
    void g.go(1000);
    assert.equal(g.reads.length, 1, JSON.stringify(barState));
  }
  g = gridGoto();
  g.scope.barState = { answer: { total: 4 }, loading: false, error: null, set: 'set-1' };
  g.scope.pageTotal = 4;
  void g.go(1000);
  assert.equal(g.reads.length, 1, 'a query that keeps bars is asked');
  // THE GRID CALLS `dropGoto` WHEN THE ROW SET MOVES, and never on its first
  // reading: a remount keeps the line under the strip.
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  assert.match(grid, /\$effect\(\(\) => \{\s*const set = setKey;\s*untrack\(\(\) => \{\s*const before = gotoSet;\s*gotoSet = set;\s*if \(before !== null && before !== set\) dropGoto\(\);\s*\}\);\s*\}\);/);
});
