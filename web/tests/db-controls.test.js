// R3-02 (DB-074, DB-041, DB-G63, DB-020, DB-025, D-3338): the /db control
// strip's lists and words — the time lists, the calendar's month flags, the
// Universe and Instrument wording on a feed with no master, the shortfall
// figure, the stranded-rung banner, the months held outside the window, and
// the whole sentence on a clipped note.
//
// The lists and sentences are pure functions in `notes-controls.js`, driven
// here with no compiler; each test also reads the component or state source
// that must call them, so a test fails on the code before R3-02 (the function
// is absent, or the old wiring is still in place) and passes after. Every
// census row, calendar answer and day record below is invented.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dbSource } from './database-page-fixture.js';
import { parse, compileModule } from 'svelte/compiler';
import {
  specialsOf,
  specialsIn,
  specialMinutes,
  haltsOf,
  cutWindows,
  regularDaysIn,
  timeListRows,
  timeListWhy,
  haltedOf,
  reopeningOf,
  monthDays,
  monthNeedsRead,
  flagText,
  MONTH_FLAGS_KEPT,
  instrumentListWhy,
  everythingWhy,
  tierLeavesOut,
  tierLeavesOutText,
  tradeDriven,
  spotShortfalls,
  fullestSpot,
  shortfallsAgainst,
  shortfallText,
  hiddenMonths,
  hiddenText,
  timeframeTitle,
  strandedEffect,
  sideStranded,
  wholeTitle,
  tokensOf,
  censusKeyOf,
  prefixOf,
  UNIVERSES,
  EVERYTHING,
  LEGACY_BLIND,
  SEGMENT_KEYS,
  SEG_VIEW,
  VIEWS,
  dayWindowApplies
} from '../src/lib/db/notes-controls.js';
import { sessionGrid, SESSION_OPEN_MINUTE, calendarOf } from '../src/lib/db/selection-model.js';
import { bandOf } from '../src/lib/db/dayflags.js';
import { tfCmp, tfNote } from '../src/lib/db/notes-labels.js';

const fmt = (/** @type {number} */ n) => n.toLocaleString('en-IN');

/** A `/calendar.json` answer's `special` list as `calendar_record.rs` writes it (invented days' shapes, the recorded windows). */
const CALENDAR = {
  from: Math.round(Date.UTC(2021, 1, 22) / 86_400_000),
  // 22 Feb 2021 (Mon) .. 7 Nov 2024 is too long to write; the day string is
  // built below from a rule, with the special days marked P.
  special: [
    { day: 0, date: '2021-02-24', code: 'P', kind: 'systems outage', windows: [{ from: '09:15', to: '11:39' }, { from: '15:45', to: '16:59' }] },
    { day: 0, date: '2023-11-12', code: 'P', kind: 'Muhurat', windows: [{ from: '18:15', to: '19:14' }] },
    { day: 0, date: '2024-11-01', code: 'P', kind: 'Muhurat', windows: [{ from: '18:00', to: '18:59' }] },
    { day: 0, date: 'not a day', code: 'P', kind: 'Muhurat', windows: [{ from: '18:00', to: '18:59' }] },
    { day: 0, date: '2024-11-02', code: 'P', kind: '', windows: [{ from: '25:00', to: '18:59' }] },
    { day: 0, date: '2024-11-03', code: 'P', windows: 'none' }
  ]
};

/** A dense day string from 22 Feb 2021: weekends closed, the special days P, every other day R. */
function dayString(/** @type {string} */ to) {
  const specials = new Set(['2021-02-24', '2023-11-12', '2024-11-01']);
  let out = '';
  for (let d = CALENDAR.from; ; d += 1) {
    const iso = new Date(d * 86_400_000).toISOString().slice(0, 10);
    const dow = new Date(d * 86_400_000).getUTCDay();
    out += specials.has(iso) ? 'P' : dow === 0 || dow === 6 ? '.' : 'R';
    if (iso === to) return out;
  }
}

test('the_time_lists_offer_every_traded_minute_of_the_days_in_view', () => {
  const specials = specialsOf(CALENDAR);
  // An entry this reader cannot read is left out, never guessed; the rest are sorted by day.
  assert.deepEqual(specials.map((s) => s.date), ['2021-02-24', '2023-11-12', '2024-11-01']);
  assert.deepEqual(specials[0].windows, [{ from: 555, to: 699 }, { from: 945, to: 1019 }]);
  assert.deepEqual(specialsOf({}), []);
  assert.deepEqual(specialsOf({ special: [{ date: '2024-11-05', windows: [{ from: '18:00', to: '18:59' }] }] })[0].kind, 'special session');
  // Out of order and repeated, the list comes back by day; a window with a missing end is left out.
  const unordered = specialsOf({
    special: [
      { date: '2024-11-05', kind: 'b', windows: [{ from: '18:00', to: '18:59' }] },
      { date: '2023-11-12', kind: 'a', windows: [{ from: '18:15', to: '19:14' }, { to: '18:00' }] },
      { date: '2024-11-05', kind: 'c', windows: [{ from: '18:00', to: '18:59' }] }
    ]
  });
  assert.deepEqual(unordered.map((d) => d.kind), ['a', 'b', 'c']);
  assert.equal(unordered[0].windows.length, 1);
  const cal = /** @type {{from: number, days: string}} */ (calendarOf({ from: CALENDAR.from, days: dayString('2024-11-08') }));

  /** The three lists for a window, built exactly as `selection.svelte.js` builds them. @param {string} lo @param {string} hi @param {number} secs @param {string[]} [loaded] */
  const lists = (lo, hi, secs = 60, loaded = []) => {
    const held = new Set(loaded);
    const step = Math.max(1, Math.round(Math.max(secs, 60) / 60));
    const here = specialsIn(specials, lo, hi);
    const extra = specialMinutes(here, step, SESSION_OPEN_MINUTE).map((m) => m.key);
    const grid = sessionGrid(secs, [...held, ...extra], { venue: 'index', lo, hi });
    return timeListRows(grid, { specials: here, regular: regularDaysIn(cal, lo, hi), held, step, open: SESSION_OPEN_MINUTE });
  };

  // THE DEFECT: the session grid alone (what the lists were) has no evening minute.
  assert.equal(sessionGrid(60, [], { venue: 'index', lo: '2024-11-01', hi: '2024-11-01' }).some((r) => r.key === '18:00'), false);

  // Muhurat 01 Nov 2024 alone: the hour that traded and nothing else — no 09:15 to land a bound on nothing.
  const muhurat = lists('2024-11-01', '2024-11-01');
  assert.equal(muhurat.length, 60);
  assert.equal(muhurat[0].key, '18:00');
  assert.equal(muhurat[59].key, '18:59');
  assert.equal(muhurat[0].detail, 'special session · 01 Nov 2024');
  assert.match(String(muhurat[0].why), /^Muhurat, 01 Nov 2024: bars open 18:00–18:59 \(the exchange calendar of record, \/calendar\.json\)\. Listed because a day in view traded at this minute\.$/);
  // A loaded bar at another minute of a special-only window stays listed.
  assert.ok(lists('2024-11-01', '2024-11-01', 60, ['18:30', '17:59']).some((r) => r.key === '17:59'));

  // A week holding the Muhurat Friday and four regular days: the 375 regular minutes AND the evening hour.
  const week = lists('2024-10-28', '2024-11-01');
  assert.equal(week.length, 375 + 60);
  assert.equal(week[0].key, '09:15');
  assert.equal(week[0].detail, 'regular session');
  assert.equal(week.find((r) => r.key === '18:15')?.detail, 'special session · 01 Nov 2024');

  // 24 Feb 2021 alone: its two windows — 09:15–11:39 and the 15:45–16:59 resumption — and no 12:00.
  const outage = lists('2021-02-24', '2021-02-24');
  assert.equal(outage.length, 145 + 75);
  assert.equal(outage.some((r) => r.key === '12:00'), false);
  assert.equal(outage.find((r) => r.key === '16:59')?.detail, 'special session · 24 Feb 2021');
  assert.equal(outage.find((r) => r.key === '09:15')?.detail, 'special session · 24 Feb 2021');
  // The same day beside a regular day: 09:15 is a regular minute again, 16:00 still the outage's.
  const around = lists('2021-02-23', '2021-02-24');
  assert.equal(around.find((r) => r.key === '09:15')?.detail, 'regular session');
  assert.equal(around.find((r) => r.key === '16:00')?.detail, 'special session · 24 Feb 2021');

  // Two Muhurats in view share 18:15–18:59, and the row says both days.
  const two = lists('2023-11-12', '2024-11-01');
  const shared = /** @type {any} */ (two.find((r) => r.key === '18:30'));
  assert.equal(shared.detail, 'special session · 2 days in view');
  assert.match(shared.why, /Muhurat, 12 Nov 2023: bars open 18:15–19:14; Muhurat, 01 Nov 2024: bars open 18:00–18:59/);

  // On a 5min rung only the grid's own minutes (09:15 + 5k) are listed for a special window.
  const five = lists('2024-11-01', '2024-11-01', 300);
  assert.deepEqual(five.map((r) => r.key).slice(0, 3), ['18:00', '18:05', '18:10']);
  assert.equal(five.length, 12);
  // An off-grid window (18:02–18:14) fills the 18:00 bucket too: the fold stamps a bar at its bucket's START (DB-074 repair).
  assert.deepEqual(specialMinutes([{ date: '2024-11-01', kind: 'x', windows: [{ from: 1082, to: 1094 }], halt: false }], 5, SESSION_OPEN_MINUTE).map((m) => m.key), ['18:00', '18:05', '18:10']);
  // The 01 Nov 2024 Muhurat hour on the derived rungs the re-check named, against /calendar.json's owed.
  assert.deepEqual(lists('2024-11-01', '2024-11-01', 3600).map((r) => r.key), ['17:15', '18:15'], '60min: owed 2');
  assert.deepEqual(lists('2024-11-01', '2024-11-01', 1800).map((r) => r.key), ['17:45', '18:15', '18:45'], '30min: owed 3');
  const two60 = lists('2024-11-01', '2024-11-01', 120).map((r) => r.key);
  assert.equal(two60.length, 31, '2min: owed 31');
  assert.equal(two60[0], '17:59');
  assert.equal(two60[30], '18:59');
  // The 17:15 bar holds 18:00–18:14, so its reason does not say the day traded AT 17:15.
  assert.match(String(lists('2024-11-01', '2024-11-01', 3600)[0].why), /Listed because a day in view traded inside the 60-minute bar stamped here \(a bar carries the first minute of its bucket on the 09:15 grid, pull::fold\)\.$/);
  // The smallest derived rung says it too (the re-check's surviving mutant, `step > 1` read as `step > 2`):
  // the 17:59 bar of 2min holds 18:00 alone, so it did not trade AT 17:59.
  const twoWhy = String(lists('2024-11-01', '2024-11-01', 120)[0].why);
  assert.ok(twoWhy.endsWith('traded inside the 2-minute bar stamped here (a bar carries the first minute of its bucket on the 09:15 grid, pull::fold).'), twoWhy);
  // And a 1s or 1min row keeps "at this minute": its bar is the minute.
  assert.ok(String(lists('2024-11-01', '2024-11-01', 1)[0].why).endsWith('Listed because a day in view traded at this minute.'));
  // More than three special days on one minute: three are named, the rest counted.
  const many = timeListRows(sessionGrid(60, ['18:30'], {}), {
    specials: ['2019-01-01', '2019-01-02', '2019-01-03', '2019-01-04'].map((date) => ({ date, kind: 'Muhurat', windows: [{ from: 1110, to: 1110 }] })),
    regular: null,
    held: [],
    step: 1,
    open: SESSION_OPEN_MINUTE
  });
  assert.match(String(many.find((r) => r.key === '18:30')?.why), /; and 1 more \(the exchange/);

  // The regular-day count: O(1) per question after one pass, unknown never zero.
  assert.equal(regularDaysIn(cal, '2024-10-28', '2024-11-01'), 4);
  assert.equal(regularDaysIn(cal, '2024-11-01', '2024-11-01'), 0);
  assert.equal(regularDaysIn(cal, '2024-11-02', '2024-11-03'), 0, 'a weekend holds no regular session');
  assert.equal(regularDaysIn(cal, '2024-11-04', '2024-11-01'), 0, 'a backwards span holds no day');
  // An impossible day is unknown, not rolled into the next month (repair round 2).
  assert.equal(regularDaysIn(cal, '2024-11-01', '2024-11-31'), null, '31 Nov is not a day');
  assert.equal(regularDaysIn(cal, '2024-02-30', '2024-11-01'), null, '30 Feb is not a day');
  assert.equal(regularDaysIn(cal, '2024-13-01', ''), null);
  assert.equal(regularDaysIn(cal, '2024-11-1', ''), null, 'a day that is not YYYY-MM-DD');
  assert.equal(regularDaysIn(cal, '2020-01-01', '2021-03-01'), null, 'a day before the calendar is unknown');
  assert.equal(regularDaysIn(cal, '2024-11-01', '2030-01-01'), null, 'a day after it is unknown');
  assert.equal(regularDaysIn(cal, '', ''), cal.days.split('').filter((c) => c === 'R').length);
  assert.equal(regularDaysIn(null, '', ''), null);
  assert.equal(regularDaysIn({ from: 0, days: '' }, '', ''), null);
  assert.equal(regularDaysIn({ from: CALENDAR.from, days: 'RR-RR' }, '', ''), null, "a '-' day in view is unknown");
  assert.equal(regularDaysIn({ from: CALENDAR.from, days: 'RXUH.P' }, '', ''), 4);
  // Unknown regular days keep the regular grid.
  assert.equal(timeListRows(sessionGrid(60, [], {}), { specials: [], regular: null, held: [], step: 1, open: SESSION_OPEN_MINUTE }).length, 375);

  // WIRED: the three lists read `sessionRows`, which builds through these functions from the calendar answer already read.
  const state = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(state, /db\.calSpecial = Array\.isArray\(body\?\.special\) \? body\.special : \[\];/);
  assert.match(state, /db\.calHalts = Array\.isArray\(body\?\.halts\) \? body\.halts : \[\];/);
  assert.match(state, /const specialsHere = \$derived\(specialsIn\(specialsOf\(\{ special: calSpecial, halts: calHalts \}\), daysInView\.lo, daysInView\.hi\)\);/);
  assert.match(state, /for \(const m of specialMinutes\(specialsHere, gridStep, SESSION_OPEN_MINUTE\)\) held\.add\(m\.key\);/);
  assert.match(state, /timeListRows\(rungGrid, \{\s*specials: specialsHere,\s*regular: regularDaysIn\(calendar, daysInView\.lo, daysInView\.hi\),/);
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.equal(controls.match(/rows=\{\[\{ key: '', name: 'Any', detail: 'no bound' \}, \.\.\.db\.sessionRows\]\}/g)?.length, 2, 'From time and To time');
  assert.match(controls, /label="minutes"\s*disabled=\{db\.sessionRows\.length === 0\}\s*summary=\{db\.goTime \|\| 'Pick a minute'\}\s*rows=\{db\.sessionRows\}/, 'Go to');
});

test('the_calendar_marks_refused_days_outside_the_window', () => {
  // One request per month a grid shows, none for a month the day band already read.
  assert.deepEqual(monthDays('2024-02'), { from: '2024-02-01', to: '2024-02-29' });
  assert.deepEqual(monthDays('2023-02'), { from: '2023-02-01', to: '2023-02-28' });
  assert.deepEqual(monthDays('1900-02'), { from: '1900-02-01', to: '1900-02-28' });
  assert.deepEqual(monthDays('2000-02'), { from: '2000-02-01', to: '2000-02-29' });
  assert.equal(monthDays('2024-13'), null);
  assert.equal(monthDays('2024-00'), null);
  assert.equal(monthDays('March'), null);
  assert.equal(monthDays(/** @type {any} */ (undefined)), null);
  const band = { from: '2019-03-15', to: '2019-03-15', clipped: false };
  assert.equal(monthNeedsRead('2019-03', band), true, 'a window of one day leaves the rest of its month unread');
  assert.equal(monthNeedsRead('2019-02', band), true, 'a month outside the window is read');
  assert.equal(monthNeedsRead('2019-03', { from: '2019-02-20', to: '2019-04-02' }), false, 'a month the band covers costs no request');
  assert.equal(monthNeedsRead('2019-03', null), true);
  assert.equal(monthNeedsRead('not a month', null), false);
  assert.ok(MONTH_FLAGS_KEPT > 0 && MONTH_FLAGS_KEPT <= 60, 'the cache is bounded');

  // A month's answer folds to the same marks the band gives: a refused day outside the window is marked.
  const read = bandOf({
    days: [
      { day: '2019-02-26', instrument: 'NSE-INDEX-SAMPLE', calendar: { code: 'R' }, import: { outcome: 'refused' }, held: { bars: 0 }, says: ['refused at import: every file failed its checksum'] },
      { day: '2019-02-27', instrument: 'NSE-INDEX-SAMPLE', calendar: { code: 'R' }, import: { outcome: 'done' }, held: { bars: 22500 }, says: [] },
      { day: '2019-02-28', instrument: 'NSE-INDEX-SAMPLE', calendar: { code: 'R' }, import: { outcome: 'incomplete', refusals: { this_count: 1, other_count: 0 } }, held: { bars: 100 }, says: ['incomplete: line 7'] }
    ]
  });
  const marks = new Map(read.cells.filter((c) => c.flagged).map((c) => [c.day, flagText(c)]));
  assert.equal(marks.get('2019-02-26'), 'refused at import — refused at import: every file failed its checksum');
  assert.equal(marks.get('2019-02-28'), 'incomplete at import — incomplete: line 7');
  assert.equal(marks.has('2019-02-27'), false, 'an ordinary day is unmarked');
  assert.equal(flagText({ word: 'closed', says: [] }), 'closed');

  // WIRED: the day fields mark from the band OR the month's own read, tell the
  // state which month they show, and say why a month is unmarked.
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.match(controls, /const dayMark = \(iso\) => db\.dayFlagged\?\.\(iso\) \?\? db\.monthFlagged\(iso\);/);
  assert.equal(controls.match(/flagged=\{dayMark\}\s*onshow=\{db\.showMonth\}\s*flagWhy=\{db\.monthFlagWhy\}/g)?.length, 2);
  assert.doesNotMatch(controls, /flagged=\{db\.dayFlagged\}/, 'the band alone marked only the window');
  const state = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(state, /if \(!monthNeedsRead\(ym, band\)\) return;/);
  assert.match(state, /const r = await ask\(dayflagsUrl\(feed, inst, days\.from, days\.to\), \{ cache: 'no-store', signal: ctl\.signal \}\);/);
  // Bounded: beyond MONTH_FLAGS_KEPT the oldest ANSWERED month goes, and a read still out goes only with its
  // read aborted (repair round 3; driven in `a_month_whose_read_is_out_is_never_dropped_or_asked_for_twice`).
  assert.match(state, /if \(next\.size > MONTH_FLAGS_KEPT\) \{\s*const gone = \[\.\.\.next\]\.find\(\(\[, e\]\) => !e\.reading\)\?\.\[0\] \?\? /);
  assert.match(state, /monthReads\.get\(gone\)\?\.abort\(\);\s*next\.delete\(gone\);/);
  // A feed, instrument or census change aborts the reads of the OLD scope only (round 1 aborted every
  // read in flight, the new scope's included); an abort is not cached as a refusal, and a finished read
  // never drops a newer read of the same key. The behaviour is driven in the test after this one.
  assert.match(state, /\$effect\(\(\) => \{\s*const scope = monthScope\(\);\s*untrack\(\(\) => \{\s*let next = null;\s*for \(const \[key, ctl\] of monthReads\) \{\s*if \(key\.startsWith\(scope\)\) continue;\s*ctl\.abort\(\);\s*monthReads\.delete\(key\);/);
  assert.match(state, /if \(ctl\.signal\.aborted\) return;\s*putMonth\(key, \{ flags: null, why: `\/dayflags\.json could not be read:/);
  assert.match(state, /if \(monthReads\.get\(key\) === ctl\) monthReads\.delete\(key\);/);
  // Every part of the cache key is one the abort watches, and the scope ends in `|`, so no scope prefixes another.
  assert.match(state, /const monthScope = \(\) => `\$\{feeds\.active \?\? ''\}\|\$\{db\.dayBandInstrument \?\? ''\}\|\$\{db\.fetchedAt \?\? ''\}\|`;/);
  assert.match(state, /const monthKey = \(ym\) => `\$\{monthScope\(\)\}\$\{ym\}`;/);
  const day = dbSource('../src/lib/DayField.svelte');
  assert.ok(day.includes("if (open && pad === 'day' && /^\\d{4}-\\d{2}$/.test(shown)) onshow?.(shown);"));
  assert.match(day, /\{#if flagged && flagWhy && pad === 'day' && flagWhy\(shown\)\}/);
});

/**
 * THE MONTH-FLAG CACHE AS THE PAGE RUNS IT (DB-025, repair round 2). The
 * block of `selectionState` from `let monthFlags` to the end of
 * `monthFlagWhy` is cut out of the file that holds it, as the page fixture
 * cuts the seeds, and compiled as a rune module beside the day field's own
 * `onshow` effect line, so Svelte's client runtime runs both effects exactly
 * as on the page. `dayFirst` puts the day field's effect before the block's,
 * so the test holds whichever of the two effects runs first. Every day record
 * is invented.
 *
 * @param {boolean} dayFirst
 */
async function monthFlagPage(dayFirst) {
  const state = dbSource('../src/lib/db/selection.svelte.js');
  const wrapped = '<script module>' + state + '</script>';
  const program = /** @type {any} */ (parse(wrapped)).module?.content;
  const factory = program.body.find((/** @type {any} */ n) => n.type === 'ExportNamedDeclaration' && n.declaration?.id?.name === 'selectionState')?.declaration;
  const body = factory?.body?.body ?? [];
  const first = body.find((/** @type {any} */ n) => n.type === 'VariableDeclaration' && n.declarations[0]?.id?.name === 'monthFlags');
  const last = body.find((/** @type {any} */ n) => n.type === 'FunctionDeclaration' && n.id?.name === 'monthFlagWhy');
  if (!first || !last) throw new Error('The month-flag seam changed.');
  const block = wrapped.slice(first.start, last.end);
  const line = "if (open && pad === 'day' && /^\\d{4}-\\d{2}$/.test(shown)) onshow?.(shown);";
  assert.ok(dbSource('../src/lib/DayField.svelte').includes(line), "the day field's onshow effect line");
  const lib = (/** @type {string} */ p) => new URL(`../src/lib/${p}`, import.meta.url).href;
  const field = `$effect(() => { const { open, pad, shown } = view; const onshow = showMonth; ${line} });`;
  const source = `import { untrack } from 'svelte';
import { monthDays, monthNeedsRead, MONTH_FLAGS_KEPT, flagText } from '${lib('db/notes-controls.js')}';
import { bandOf, dayflagsUrl } from '${lib('db/dayflags.js')}';
import { refusalFrom } from '${lib('refusal.js')}';
export function mount(init, ask) {
  const feeds = $state({ active: init.feed });
  const db = $state(init.db);
  const view = $state({ open: true, pad: 'day', shown: init.shown });
  const activeFeed = null;
  /** @type {any} */
  let api = null;
  const stop = $effect.root(() => {
    ${dayFirst ? field : ''}
    ${block}
    ${dayFirst ? '' : field}
    api = { monthFlagged, monthFlagWhy };
  });
  return { feeds, db, view, stop, api };
}`;
  const { js } = compileModule(source, { filename: 'month-flags.svelte.js', generate: 'client' });
  const code = js.code.replace(/from (["'])([^"']+)\1/g, (_m, q, p) => `from ${q}${p === 'svelte' ? SVELTE_CLIENT : import.meta.resolve(p)}${q}`);
  return import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${Math.random()}`);
}

/** The client entry of Svelte (node resolves `svelte` to its server entry, whose flushSync does nothing). */
const SVELTE_CLIENT = import.meta.resolve('svelte').replace(/index-server\.js$/, 'index-client.js');
const settle = async () => { for (let i = 0; i < 20; i++) await new Promise((done) => setImmediate(done)); };

test('a_shown_month_is_read_again_when_the_census_read_moves', async () => {
  const { flushSync } = await import(SVELTE_CLIENT);
  const REFUSED = { day: '2019-02-26', instrument: 'NSE-INDEX-SAMPLE', calendar: { code: 'R' }, import: { outcome: 'refused' }, held: { bars: 0 }, says: ['refused at import: every file failed its checksum'] };
  const MARK = 'refused at import — refused at import: every file failed its checksum';
  const READING = 'Reading this month’s day flags from /dayflags.json…';
  for (const dayFirst of [false, true]) {
    const at = dayFirst ? "the day field's effect first" : "the block's effects first";
    /** @type {Array<{url: string, signal: AbortSignal, answer: () => void, refuse: () => void}>} */
    const asked = [];
    /** @param {string} url @param {{signal: AbortSignal}} init */
    const ask = (url, init) => new Promise((resolve, reject) => {
      init.signal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
      asked.push({ url, signal: init.signal, answer: () => resolve(Response.json({ days: [REFUSED] })),
        refuse: () => resolve(Response.json({ error: 'the day flags are not built' }, { status: 503 })) });
    });
    const mod = await monthFlagPage(dayFirst);
    const page = mod.mount({ feed: 'gdfl', shown: '2019-02', db: { dayBandInstrument: 'NSE-INDEX-SAMPLE', fetchedAt: 1, dayBand: null, dayBandLoading: false, dayBandSpan: null } }, ask);
    const { monthFlagged, monthFlagWhy } = page.api;
    flushSync();
    assert.equal(asked.length, 1, `${at}: the shown month is asked for once`);
    assert.match(asked[0].url, /^\/dayflags\.json\?feed=gdfl&instrument=NSE-INDEX-SAMPLE&from=2019-02-01&to=2019-02-28/);
    assert.equal(monthFlagWhy('2019-02'), READING, `${at}: Reading while the read is out`);
    asked[0].answer();
    await settle();
    flushSync();
    assert.equal(monthFlagged('2019-02-26'), MARK, `${at}: the refused day is marked`);
    assert.equal(monthFlagWhy('2019-02'), '', `${at}: a read month says nothing under the grid`);

    // THE DEFECT: the whole-census read lands while the grid is open (db.fetchedAt moves). The month is
    // asked for again under its new key, says Reading only while that read is out, and is marked again.
    page.db.fetchedAt = 2;
    flushSync();
    assert.equal(asked.length, 2, `${at}: a census read landing asks for the shown month again`);
    assert.equal(asked[1].url, asked[0].url, `${at}: the same month`);
    assert.equal(monthFlagWhy('2019-02'), READING, `${at}: Reading while that read is out`);
    asked[1].answer();
    await settle();
    flushSync();
    assert.equal(monthFlagged('2019-02-26'), MARK, `${at}: marked again after the census read`);
    assert.equal(monthFlagWhy('2019-02'), '', `${at}: and nothing left saying Reading`);

    // Two census reads in a row while a month's read is out: the old scope's read is aborted, the new one
    // is not, and the month ends marked with nothing in flight. 3 then 32: the scope ends in '|', so the
    // old key (…|3|2019-02) is not taken for one of the new scope (…|32|).
    page.db.fetchedAt = 3;
    flushSync();
    page.db.fetchedAt = 32;
    flushSync();
    assert.equal(asked.length, 4, `${at}: one read per census read`);
    assert.equal(asked[2].signal.aborted, true, `${at}: the old scope's read is aborted`);
    assert.equal(asked[3].signal.aborted, false, `${at}: the current scope's read is not`);
    assert.equal(monthFlagWhy('2019-02'), READING);
    asked[3].answer();
    await settle();
    flushSync();
    assert.equal(monthFlagged('2019-02-26'), MARK, `${at}: marked after two census reads`);
    assert.equal(monthFlagWhy('2019-02'), '');

    // A scope left while its read is out: the read is aborted, and on return the month is asked for again,
    // never stuck at Reading; a scope returned to whose month was read costs nothing.
    page.db.dayBandInstrument = 'NSE-INDEX-OTHER';
    flushSync();
    assert.equal(asked.length, 5, `${at}: the other instrument's month is asked for`);
    page.db.dayBandInstrument = 'NSE-INDEX-SAMPLE';
    flushSync();
    assert.equal(asked.length, 5, `${at}: a month already read under the scope it returns to costs nothing`);
    assert.equal(asked[4].signal.aborted, true, `${at}: the scope left has its read aborted`);
    assert.equal(monthFlagged('2019-02-26'), MARK);
    assert.equal(monthFlagWhy('2019-02'), '');
    page.db.dayBandInstrument = 'NSE-INDEX-OTHER';
    flushSync();
    assert.equal(asked.length, 6, `${at}: the scope whose read was aborted is asked for again on return`);
    assert.equal(monthFlagWhy('2019-02'), READING, `${at}: Reading while that read is out`);
    asked[5].answer();
    await settle();
    flushSync();
    assert.equal(monthFlagged('2019-02-26'), MARK);
    assert.equal(monthFlagWhy('2019-02'), '');
    page.db.dayBandInstrument = 'NSE-INDEX-SAMPLE';
    flushSync();
    assert.equal(asked.length, 6);

    // Closing and reopening the grid asks nothing; paging to another month asks once for it.
    page.view.open = false;
    flushSync();
    page.view.open = true;
    flushSync();
    assert.equal(asked.length, 6, `${at}: a reopen asks nothing`);
    page.view.shown = '2019-01';
    flushSync();
    assert.equal(asked.length, 7, `${at}: another month is one read`);
    assert.match(asked[6].url, /from=2019-01-01&to=2019-01-31/);
    asked[6].answer();
    await settle();

    // An instrument change asks for the shown month under the new instrument; a refusal is said under
    // the grid in the server's words, never as Reading.
    page.db.dayBandInstrument = 'NSE-INDEX-OTHER';
    flushSync();
    assert.equal(asked.length, 8, `${at}: an instrument change asks again`);
    assert.match(asked[7].url, /instrument=NSE-INDEX-OTHER&from=2019-01-01&to=2019-01-31/);
    asked[7].refuse();
    await settle();
    flushSync();
    assert.equal(monthFlagWhy('2019-01'), 'No day of this month is marked: /dayflags.json answered HTTP 503: the day flags are not built.');
    assert.equal(monthFlagged('2019-01-15'), null);
    // A refused month is asked again when it is shown again — its grid reopened — and only then (repair round
    // 3: a refusal stayed until the feed, instrument or census read moved). A refusal asks nothing by itself.
    await settle();
    flushSync();
    assert.equal(asked.length, 8, `${at}: a refusal is not asked again on its own`);
    page.view.open = false;
    flushSync();
    page.view.open = true;
    flushSync();
    assert.equal(asked.length, 9, `${at}: a refused month shown again is asked once more`);
    assert.equal(asked[8].url, asked[7].url);
    assert.equal(monthFlagWhy('2019-01'), READING, `${at}: Reading while that read is out`);
    asked[8].refuse();
    await settle();
    flushSync();
    assert.equal(monthFlagWhy('2019-01'), 'No day of this month is marked: /dayflags.json answered HTTP 503: the day flags are not built.');

    // No feed: nothing is asked, and the grid never says Reading with nothing in flight.
    page.feeds.active = null;
    flushSync();
    assert.equal(asked.length, 9, `${at}: no feed, no read`);
    assert.equal(monthFlagWhy('2019-01'), 'No day of this month is marked yet: its day flags are not on hand, and no read of them is out.');
    // A month the day band covers is not asked for and says nothing.
    page.feeds.active = 'gdfl';
    page.db.dayBand = {};
    page.db.dayBandSpan = { from: '2018-12-01', to: '2019-01-31' };
    flushSync();
    assert.equal(asked.length, 9, `${at}: a month the band covers costs no request`);
    assert.equal(monthFlagWhy('2019-01'), '');
    page.view.shown = '2018-12';
    flushSync();
    assert.equal(asked.length, 9, `${at}: paging to a month the band covers asks nothing`);
    assert.equal(monthFlagWhy('2018-12'), '');
    // The page going aborts a read still out.
    page.view.shown = '2019-03';
    flushSync();
    assert.equal(asked.length, 10);
    assert.equal(asked[9].signal.aborted, false);
    page.stop();
    assert.equal(asked[9].signal.aborted, true, `${at}: the page going aborts the read still out`);
  }
});

test('a_month_whose_read_is_out_is_never_dropped_or_asked_for_twice', async () => {
  // REPAIR ROUND 3. The 48-month cache dropped its oldest entry whatever it was, so a month whose read was
  // still out lost its `reading` entry once 48 other months were shown: shown again, it said "its day flags
  // have not been asked for" with its read in flight, and only a second guard kept it from being asked for
  // twice. Now the oldest ANSWERED month goes; a read still out goes only when every kept month is one, and
  // then with its read aborted. Every day record is invented.
  const { flushSync } = await import(SVELTE_CLIENT);
  const REFUSED = { day: '2019-02-26', instrument: 'NSE-INDEX-SAMPLE', calendar: { code: 'R' }, import: { outcome: 'refused' }, held: { bars: 0 }, says: ['refused at import: every file failed its checksum'] };
  const MARK = 'refused at import — refused at import: every file failed its checksum';
  const READING = 'Reading this month’s day flags from /dayflags.json…';
  const NONE = 'No day of this month is marked yet: its day flags are not on hand, and no read of them is out.';
  /** 48 months other than Feb 2019: Jan 2015 to Dec 2018. */
  const others = Array.from({ length: MONTH_FLAGS_KEPT }, (_, i) => `${2015 + Math.floor(i / 12)}-${String((i % 12) + 1).padStart(2, '0')}`);
  assert.equal(new Set([...others, '2019-02']).size, MONTH_FLAGS_KEPT + 1);
  const fresh = async () => {
    /** @type {Array<{url: string, signal: AbortSignal, answer: () => void}>} */
    const asked = [];
    /** @param {string} url @param {{signal: AbortSignal}} init */
    const ask = (url, init) => new Promise((resolve, reject) => {
      init.signal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
      asked.push({ url, signal: init.signal, answer: () => resolve(Response.json({ days: url.includes('from=2019-02-01') ? [REFUSED] : [] })) });
    });
    const page = (await monthFlagPage(false)).mount({ feed: 'gdfl', shown: '2019-02', db: { dayBandInstrument: 'NSE-INDEX-SAMPLE', fetchedAt: 1, dayBand: null, dayBandLoading: false, dayBandSpan: null } }, ask);
    flushSync();
    return { asked, page };
  };

  // 1. Feb 2019's read stays out while 48 other months are shown and answered: the oldest ANSWERED month
  //    goes, Feb 2019 keeps saying Reading, and showing it again asks nothing.
  {
    const { asked, page } = await fresh();
    const { monthFlagged, monthFlagWhy } = page.api;
    assert.equal(asked.length, 1);
    for (const ym of others) {
      page.view.shown = ym;
      flushSync();
      asked[asked.length - 1].answer();
      await settle();
      flushSync();
    }
    assert.equal(asked.length, MONTH_FLAGS_KEPT + 1, 'one read per month shown');
    assert.equal(asked[0].signal.aborted, false, 'the read still out is not aborted while answered months can go');
    assert.equal(monthFlagWhy('2019-02'), READING, 'a month whose read is out says Reading, never "not on hand"');
    assert.equal(monthFlagWhy(others[0]), NONE, 'the oldest answered month went to keep the bound');
    assert.equal(monthFlagWhy(others[1]), '', 'the next one is kept');
    page.view.shown = '2019-02';
    flushSync();
    assert.equal(asked.length, MONTH_FLAGS_KEPT + 1, 'a month whose read is out is not asked for twice');
    assert.equal(monthFlagWhy('2019-02'), READING);
    asked[0].answer();
    await settle();
    flushSync();
    assert.equal(monthFlagged('2019-02-26'), MARK, 'its first read marks it');
    assert.equal(monthFlagWhy('2019-02'), '');
    page.view.shown = others[0];
    flushSync();
    assert.equal(asked.length, MONTH_FLAGS_KEPT + 2, 'the month that went is read again when shown');
    assert.equal(monthFlagWhy(others[0]), READING);
    assert.equal(monthFlagWhy(others[1]), NONE, 'and the oldest answered month goes for it');
    assert.equal(monthFlagWhy('2019-02'), '', 'never the month just answered');
    page.stop();
  }

  // 2. No read answers: the 49th month shown drops the oldest read with its read ABORTED, so nothing
  //    says Reading without a read out and nothing says "not on hand" with one out.
  {
    const { asked, page } = await fresh();
    const { monthFlagged, monthFlagWhy } = page.api;
    for (const ym of others) {
      page.view.shown = ym;
      flushSync();
    }
    assert.equal(asked.length, MONTH_FLAGS_KEPT + 1);
    assert.equal(asked[0].signal.aborted, true, 'every kept month was a read out: the oldest read is aborted with its entry');
    assert.equal(asked[1].signal.aborted, false, 'and only that one');
    await settle();
    assert.equal(monthFlagWhy('2019-02'), NONE, 'its read is not out, and it says so');
    assert.equal(monthFlagWhy(others[0]), READING);
    assert.equal(monthFlagWhy(others[MONTH_FLAGS_KEPT - 1]), READING);
    page.view.shown = '2019-02';
    flushSync();
    assert.equal(asked.length, MONTH_FLAGS_KEPT + 2, 'a month whose read was aborted is asked for again');
    assert.equal(asked[1].signal.aborted, true, 'the next oldest read goes for it');
    assert.equal(monthFlagWhy(others[0]), NONE);
    assert.equal(monthFlagWhy('2019-02'), READING);
    asked[MONTH_FLAGS_KEPT + 1].answer();
    await settle();
    flushSync();
    assert.equal(monthFlagged('2019-02-26'), MARK, 'the new read marks it; the aborted one left nothing behind');
    assert.equal(monthFlagWhy('2019-02'), '');
    page.view.shown = others[1];
    flushSync();
    assert.equal(asked.length, MONTH_FLAGS_KEPT + 2, 'a month whose read is still out is not asked for twice');
    page.stop();
  }
});

test('no_master_wording_on_a_feed_without_a_master', () => {
  // GDFL publishes no master: neither the list's tooltip nor Everything's row says "this feed's master".
  for (const said of [instrumentListWhy(true, 'GDFL'), everythingWhy(true, 'GDFL')]) {
    assert.doesNotMatch(said, /master (lists|no longer)/);
    assert.doesNotMatch(said, /this feed's master|feed’s master/);
  }
  assert.match(instrumentListWhy(true, 'GDFL'), /GDFL stores — it publishes no instrument master, so this list is its census \(stored, no master\)/);
  assert.match(everythingWhy(true, 'GDFL'), /including the ones no core::universe tier lists/);
  // A feed that has one says so by name.
  assert.match(instrumentListWhy(false, 'Zerodha'), /^Every name Zerodha's instrument master lists/);
  assert.match(everythingWhy(false, 'Zerodha'), /Zerodha's master no longer lists/);

  // The members a tier leaves out are named, sorted, then counted.
  const held = ['RELIANCE', 'DHFL', 'ALBK', 'NIFTY', 'TATAMOTORS', 'Z1', 'Z2', 'Z3', 'Z4', 'Z5', 'Z6'];
  const fno = new Set(['RELIANCE', 'NIFTY']);
  const left = /** @type {{count: number, names: string[]}} */ (tierLeavesOut(held, fno));
  assert.equal(left.count, 9);
  assert.deepEqual(left.names, ['ALBK', 'DHFL', 'TATAMOTORS', 'Z1', 'Z2', 'Z3', 'Z4', 'Z5']);
  assert.equal(tierLeavesOutText('F&O underlyings', left, fmt), 'F&O underlyings leaves out 9 stored name(s): ALBK, DHFL, TATAMOTORS, Z1, Z2, Z3, Z4, Z5 and 1 more — held, outside this tier, and listed under Everything.');
  assert.equal(tierLeavesOutText('NIFTY 50', /** @type {any} */ (tierLeavesOut(['NIFTY'], fno)), fmt), '');
  assert.equal(tierLeavesOut(held, null), null, 'Everything joins nothing and leaves nothing out');
  assert.equal(tierLeavesOutText('x', null, fmt), '');
  assert.equal(tierLeavesOutText('NIFTY 50', /** @type {any} */ (tierLeavesOut(['B', 'A'], new Set())), fmt), 'NIFTY 50 leaves out 2 stored name(s): A, B — held, outside this tier, and listed under Everything.');

  // WIRED on the page: no literal "this feed's master" sentence is left on the Universe or Instrument rung.
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.doesNotMatch(controls, /title="Every name this feed's master lists/);
  assert.doesNotMatch(controls, /whose instrument this feed\\u2019s master no longer lists/);
  assert.match(controls, /title=\{instrumentListWhy\(db\.noMaster, db\.feedName\)\}/);
  assert.match(controls, /why: u\.key \? undefined : everythingWhy\(db\.noMaster, db\.feedName\)/);
  assert.match(controls, /\{tierLeavesOutText\(db\.chosenUniverse\.label, db\.leftOut, fmt\)\}/);
  const state = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(state, /const leftOut = \$derived\(tierLeavesOut\(heldUnderlyings, universeNames\)\);/);
  assert.match(state, /\? `\$\{feedDisplay\(feeds\.active\)\} has no instrument master: its list is its census/);
  assert.doesNotMatch(state, /no count on this page is built from another feed's master/);
});

test('the census key and the membership tokens are read off a master row as before', () => {
  // Pre-existing readers in the touched module, driven so the module is covered whole.
  assert.deepEqual(tokensOf({ universes: ['index', 'fno'] }), ['index', 'fno']);
  assert.deepEqual(tokensOf({ universe: 'index+fno' }), ['index', 'fno']);
  assert.deepEqual(tokensOf({}), []);
  assert.deepEqual(tokensOf(null), []);
  assert.equal(censusKeyOf(/** @type {any} */ ({ exchange: 'NSE', segment: 'INDEX', symbol: 'NIFTY-50' })), 'NSE-INDEX-NIFTY-50');
});

test('the shortfall is per rung, against the same segment, and never said of a trade-driven series (DB-G63)', () => {
  /** @param {string} instrument @param {string} month @param {string} timeframe @param {number} rows @param {string | null} [expiry] */
  const row = (instrument, month, timeframe, rows, expiry = null) => {
    const [, kind, ...rest] = instrument.split('-');
    return { instrument, kind, underlying: rest[0], month, timeframe, rows, expiry };
  };
  const census = [
    // Indices at 1s: the fullest index is 405,647; a share at 1s holds far fewer, and is trade-driven.
    row('NSE-INDEX-BIGIDX', '2019-06', '1s', 405_647),
    row('NSE-INDEX-SMALLIDX', '2019-06', '1s', 405_600),
    row('NSE-CASH-SHARE', '2019-06', '1s', 90_000),
    // At 1min a share printed a pre-open minute the indices never hold: it must not make every index short.
    row('NSE-CASH-SHARE', '2019-06', '1min', 7_900),
    row('NSE-CASH-OTHER', '2019-06', '1min', 7_870),
    row('NSE-INDEX-BIGIDX', '2019-06', '1min', 7_875),
    row('NSE-INDEX-SMALLIDX', '2019-06', '1min', 7_875),
    // An option month, measured against nothing.
    row('NSE-FNO-BIGIDX', '2019-06', '1s', 11, '2023-12-28')
  ];
  const by = spotShortfalls(census, census);
  assert.deepEqual(by.get('SMALLIDX')?.get('1s'), { months: 1, short: 47, trade: false, seg: 'INDEX' }, 'short of the fullest INDEX, not of every series');
  assert.deepEqual(by.get('SMALLIDX')?.get('1min'), { months: 1, short: 0, trade: false, seg: 'INDEX' }, "a share's pre-open minute does not make an index short");
  assert.deepEqual(by.get('OTHER')?.get('1min'), { months: 1, short: 30, trade: false, seg: 'CASH' }, 'a share is short of the fullest share');
  assert.deepEqual(by.get('SHARE')?.get('1s'), { months: 1, short: 0, trade: true, seg: 'CASH' }, 'a share at 1s is trade-driven');
  assert.equal(by.get('BIGIDX')?.size, 2, 'the contract month is not folded into its underlying');
  assert.equal(shortfallText(by.get('SMALLIDX'), tfCmp, fmt), ' (1s: 47 bar(s) short over 1 month(s), against the fullest INDEX series of each)');
  assert.equal(shortfallText(by.get('SHARE'), tfCmp, fmt), ' (1s: trade-driven, a bar only where a trade printed, so no fixed bar count)');
  assert.equal(shortfallText(by.get('BIGIDX'), tfCmp, fmt), '');
  assert.equal(shortfallText(undefined, tfCmp, fmt), '');
  // Several rungs short: listed in ladder order, never summed into one figure.
  const two = spotShortfalls(census, [row('NSE-INDEX-SMALLIDX', '2019-06', '1min', 7_000), row('NSE-INDEX-SMALLIDX', '2019-06', '1s', 400_000)]);
  assert.equal(shortfallText(two.get('SMALLIDX'), tfCmp, fmt), ' (1s: 5,647 bar(s) short over 1 month(s), against the fullest INDEX series of each; 1min: 875 bar(s) short over 1 month(s), against the fullest INDEX series of each)');
  // A SUM, SAID AS ONE (Mac re-check, 17 Oct): Zerodha NIFTY 1min read "78 bar(s) short of the fullest
  // INDEX series of each month" for 78 bars across 105 months — the months are named beside the sum.
  assert.doesNotMatch(shortfallText(two.get('SMALLIDX'), tfCmp, fmt), /of each month/);
  assert.equal(
    shortfallText(new Map([['1min', { months: 1_200, short: 5, trade: false, seg: 'INDEX' }]]), tfCmp, fmt),
    ' (1min: 5 bar(s) short over 1,200 month(s), against the fullest INDEX series of each)'
  );
  const months2 = spotShortfalls(
    [row('NSE-INDEX-BIGIDX', '2019-06', '1min', 7_875), row('NSE-INDEX-BIGIDX', '2019-07', '1min', 8_250)],
    [row('NSE-INDEX-SMALLIDX', '2019-06', '1min', 7_800), row('NSE-INDEX-SMALLIDX', '2019-07', '1min', 8_247)]
  );
  assert.equal(shortfallText(months2.get('SMALLIDX'), tfCmp, fmt), ' (1min: 78 bar(s) short over 2 month(s), against the fullest INDEX series of each)');
  // A month no other series shares measures against itself.
  assert.equal(spotShortfalls([], [row('NSE-INDEX-LONE', '2020-01', '1min', 10)]).get('LONE')?.get('1min')?.short, 0);
  assert.equal(tradeDriven({ segment: 'CASH', timeframe: '1s' }), true);
  assert.equal(tradeDriven({ kind: 'FNO', timeframe: '1min' }), true);
  assert.equal(tradeDriven({ timeframe: '1min' }), false);
  assert.equal(tradeDriven({ kind: 'INDEX', timeframe: '1s', expiry: '2020-01-30' }), true);

  // WIRED: the Instrument row's title says the per-rung, per-segment figure — not the census's pooled `short`.
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.match(controls, /spot instrument-month\(s\)\$\{shortfallText\(r\.shortBy, tfCmp, fmt\)\}/);
  // "945 spot month(s)" for Zerodha NIFTY counted 105 months at nine rungs: the count is instrument-months.
  assert.doesNotMatch(controls, /\)\} spot month\(s\)/);
  assert.doesNotMatch(controls, /bar\(s\) short of the fullest series\)/);
  // The fullest series are derived from the census alone; a universe or window change folds only the universe (repair round 1).
  const sel = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(sel, /const fullest = \$derived\(fullestSpot\(db\.deco\)\);\s*const shortfalls = \$derived\(shortfallsAgainst\(fullest, universed\)\);/);
  assert.doesNotMatch(sel, /spotShortfalls\(db\.deco/);
  // The split is the same answer as the one call.
  const f = fullestSpot(census);
  assert.deepEqual(shortfallsAgainst(f, census), spotShortfalls(census, census));
  assert.deepEqual([...f.entries()].sort(), [...fullestSpot([...census].reverse()).entries()].sort(), 'order of the census does not matter');
  assert.equal(fullestSpot([{ underlying: 'X', month: '2020-01', timeframe: '1min', kind: 'FNO', rows: 9, expiry: '2020-01-30' }]).size, 0, 'a contract is no spot series');
});

test('a stranded rung says "no row can match" only when no row does (DB-020)', () => {
  assert.equal(strandedEffect(0), 'so no row can match.');
  // A stale expiry under Spot, over a full grid.
  // The number is INSTRUMENT-MONTHS (`matched.length`), and the sentence says so: on the Bars
  // view the table's rows are bars, and "the table's 105 row(s)" over 4,39,42,544 bars was false.
  assert.equal(strandedEffect(105, 0, fmt), "so it narrows nothing on screen: 105 instrument-month(s) still match the selection.");
  assert.doesNotMatch(strandedEffect(1_234, 0, fmt), /table's|row\(s\)/);
  assert.equal(strandedEffect(1_234, 0, fmt), 'so it narrows nothing on screen: 1,234 instrument-month(s) still match the selection.');
  assert.equal(strandedEffect(1, 0, fmt), 'so it narrows nothing on screen: 1 instrument-month(s) still match the selection.');
  // Two strikes picked, one off this expiry's chain: the other still matches.
  assert.equal(strandedEffect(8, 1, fmt), 'so it matches nothing; the table shows the 1 other value(s) still chosen on this rung.');
  assert.equal(strandedEffect(5, 0), "so it narrows nothing on screen: 5 instrument-month(s) still match the selection.");
  const state = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(state, /effect: strandedEffect\(st\.empty \? 0 : matched\.length, st\.kept \?\? 0, fmt\)/);
  assert.match(state, /kept: strikePick\.size - gone\.length,/);
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.match(controls, /which the rungs above it no longer offer —\s*\{st\.effect\}/);
  assert.doesNotMatch(controls, /no longer offer —\s*so no row can match\./);
});

test('a leftover strike does not strand a side the expiry offers (DB-020, control 110)', () => {
  // NIFTY 02 Jan 2020 (weekly): the chain holds 12,200 CE and PE; 8,500 is a
  // strike left over from another expiry. Invented rows, the real shapes.
  const rows = [
    { side: 'CE', strike: 1_220_000 },
    { side: 'PE', strike: 1_220_000 },
    { side: 'PE', strike: 1_210_000 }
  ];
  const chain = new Set(rows.map((r) => String(r.strike)));
  // The rule before: the side counted under the WHOLE strike pick, so 8,500
  // emptied it and PE was named stranded. Kept here as the rule refused.
  const oldStranded = (/** @type {string} */ side, /** @type {Set<string>} */ pick) =>
    !rows.filter((r) => pick.size === 0 || pick.has(String(r.strike))).some((r) => r.side === side);
  assert.equal(oldStranded('PE', new Set(['850000'])), true);
  // Now: a strike off the chain does not count, so PE stands.
  assert.equal(sideStranded('PE', rows, new Set(['850000']), chain), false);
  // 8,500 and 12,200 picked: 12,200 is on the chain and carries PE.
  assert.equal(sideStranded('PE', rows, new Set(['850000', '1220000']), chain), false);
  // A strike on the chain that carries no PE does strand PE: the rungs above really exclude it.
  const callsOnly = [{ side: 'CE', strike: 1_230_000 }, ...rows];
  assert.equal(sideStranded('PE', callsOnly, new Set(['1230000']), new Set(callsOnly.map((r) => String(r.strike)))), true);
  // No side picked is never stranded; a side no row carries is.
  assert.equal(sideStranded('', rows, new Set(), chain), false);
  assert.equal(sideStranded('XX', rows, new Set(), chain), true);
  assert.equal(sideStranded('PE', [], new Set(), new Set()), true);
  // A row with no strike counts only while no live strike is picked.
  assert.equal(sideStranded('PE', [{ side: 'PE', strike: null }], new Set(), new Set()), false);
  assert.equal(sideStranded('PE', [{ side: 'PE', strike: null }], new Set(['1']), new Set(['1'])), true);
  // WIRED: `stranded` asks `sideStranded` with the chain, not `sideOffered` under the whole pick.
  const state = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(state, /if \(sideStranded\(side, expiryGated, strikePick, here\)\) \{/);
  assert.doesNotMatch(state, /if \(side && !sideOffered\.includes\(side\)\)/);
});

test('the Timeframe and Segment lists say the months held outside the window, with their count (D-3338)', () => {
  const out = [
    { underlying: 'NIFTY', timeframe: '1day', seg: 'spot' },
    { underlying: 'NIFTY', timeframe: '1day', seg: 'spot' },
    { underlying: 'NIFTY', timeframe: '1min', seg: 'spot' },
    { underlying: 'NIFTY', timeframe: '1day', seg: 'futures' },
    { underlying: 'BANKNIFTY', timeframe: '1day', seg: 'spot' }
  ];
  const segOf = (/** @type {any} */ r) => r.seg;
  const h = hiddenMonths(out, 'NIFTY', segOf, 'spot');
  assert.deepEqual([...h.bySegment], [['spot', 3], ['futures', 1]]);
  assert.deepEqual([...h.byRung], [['1day', 2], ['1min', 1]], 'the rung counts are the picked segment’s');
  assert.deepEqual([...hiddenMonths(out, 'NIFTY', segOf, '').byRung], [['1day', 3], ['1min', 1]]);
  assert.equal(hiddenMonths(out, '', segOf, 'spot').bySegment.size, 0);
  assert.equal(hiddenText(36, { from: '2018-01-01', to: '2026-09-30' }, fmt), ' 36 more month(s) are held outside your window (2018-01-01 to 2026-09-30) and hidden — "Show them, labelled" under the dates brings them in.');
  assert.equal(hiddenText(2, null, fmt), ' 2 more month(s) are held outside your window and hidden — "Show them, labelled" under the dates brings them in.');
  assert.equal(hiddenText(0, null, fmt), '');
  // The Timeframe row's hover text closes the rung note as a sentence before the hidden clause (repair round 1).
  assert.equal(timeframeTitle(tfNote('1day'), 36, { from: '2018-01-01', to: '2026-09-30' }, fmt), `${tfNote('1day')}. 36 more month(s) are held outside your window (2018-01-01 to 2026-09-30) and hidden — "Show them, labelled" under the dates brings them in.`);
  assert.equal(timeframeTitle('60s bars · 375/session', 0, null, fmt), '60s bars · 375/session.');
  // WIRED: both lists carry the count; a rung held only outside the window is listed and refused.
  const state = dbSource('../src/lib/db/selection.svelte.js');
  assert.equal(state.match(/title: timeframeTitle\(tfNote\(tf\), off, win, fmt\)/g)?.length, 2, 'the held and the hidden-only rows');
  assert.match(state, /detail: `\$\{c > 0 \? `\$\{fmt\(c\)\} held` : 'nothing stored'\}\$\{off > 0 \? ` · \$\{fmt\(off\)\} hidden` : ''\}`/);
  assert.match(state, /why: `every \$\{tf\} month of this selection is outside your window`/);
  assert.match(state, /oow \? \{ bySegment: new Map\(\), byRung: new Map\(\) \} : hiddenMonths\(windowSplit\.out, filter, segOf, kind\)/);
  assert.match(dbSource('../src/routes/db/DbControls.svelte'), /\.\.\.db\.timeframeRows/);
});

test('a note cut to two lines carries the whole sentence as hover text', () => {
  const node = { textContent: '  No bar length —\n   nothing stored  at this rung ', title: '' };
  const handle = wholeTitle(node);
  assert.equal(node.title, 'No bar length — nothing stored at this rung');
  handle.destroy();
  // With an observer, a change to the text re-titles the node.
  /** @type {any[]} */
  const seen = [];
  const original = /** @type {any} */ (globalThis).MutationObserver;
  /** @type {any} */ (globalThis).MutationObserver = class {
    /** @param {() => void} fn */
    constructor(fn) {
      this.fn = fn;
      seen.push(this);
    }
    /** @param {any} target @param {any} options */
    observe(target, options) {
      this.target = target;
      this.options = options;
    }
    disconnect() {
      this.off = true;
    }
  };
  try {
    const live = { textContent: 'first', title: '' };
    const h2 = wholeTitle(live);
    assert.deepEqual(seen[0].options, { childList: true, characterData: true, subtree: true });
    live.textContent = 'second   sentence';
    seen[0].fn();
    assert.equal(live.title, 'second sentence');
    h2.destroy();
    assert.equal(seen[0].off, true);
    const empty = { textContent: null, title: 'x' };
    wholeTitle(empty);
    assert.equal(empty.title, '');
  } finally {
    /** @type {any} */ (globalThis).MutationObserver = original;
  }
  // WIRED: every clamped note on the strip that has no title of its own takes the action.
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  const notes = controls.match(/<span class="note(?:[^>{]|\{[^}]*\})*>/g) ?? [];
  assert.ok(notes.length >= 8);
  for (const tag of notes) assert.match(tag, /use:wholeTitle|title=/, tag);
  // The strip's other two files draw `.note`s under the same clamp: the Spot rule's note (DbMoneyness,
  // found clipped with no title at 1600 x 1000 on NIFTY 28 Mar 2019 options, 17 Oct) and the search's.
  for (const file of ['../src/routes/db/DbMoneyness.svelte', '../src/routes/db/DbSearch.svelte']) {
    const tags = dbSource(file).match(/<span class="note(?:[^>{]|\{[^}]*\})*>/g) ?? [];
    assert.ok(tags.length >= 1, file);
    for (const tag of tags) assert.match(tag, /use:wholeTitle|title=/, `${file}: ${tag}`);
  }
  assert.match(controls, /:global\(\.strip\) \.field > \.note \{[\s\S]*?line-clamp: 2;/);
});

test('the fixed vocabulary of the strips is exactly this', () => {
  // The universes, in the owner's order, with the token each joins on.
  assert.deepEqual(
    UNIVERSES.map((u) => [u.key, u.label, u.token]),
    [
      ['n50', 'NIFTY 50', 'n50'],
      ['n100', 'NIFTY 100', 'n100'],
      ['n200', 'NIFTY 200', 'n200'],
      ['n500', 'NIFTY 500', 'n500'],
      ['ntm', 'NIFTY Total Market', 'ntm'],
      ['fno', 'F&O underlyings', 'fno'],
      ['index', 'NSE indices', 'index'],
      ['', 'Everything', '']
    ]
  );
  assert.equal(UNIVERSES.at(-1), EVERYTHING);
  assert.deepEqual([...LEGACY_BLIND], ['n50', 'n100', 'n200', 'n500']);
  assert.deepEqual(SEGMENT_KEYS, ['INDEX', 'CASH', 'FNO']);
  assert.deepEqual(
    SEG_VIEW.map((s) => [s.key, s.name, s.note, s.always]),
    [
      ['spot', 'Spot', 'continuous series — no expiry', true],
      ['futures', 'Expired futures', 'futures that settled before today, with their final session inside the archive', true],
      ['options', 'Expired options', 'options that settled before today, with their final session inside the archive', true],
      ['futures-cut', 'Futures settled after the archive ended', "settled before today but after the archive's measured last day, so their last sessions are not imported", false],
      ['options-cut', 'Options settled after the archive ended', "settled before today but after the archive's measured last day, so their last sessions are not imported", false],
      ['futures-live', 'Unexpired futures', 'expiring today or later — not settled', false],
      ['options-live', 'Unexpired options', 'expiring today or later — not settled', false]
    ]
  );
  assert.deepEqual(
    VIEWS.map((v) => [v.key, v.label, v.sub, v.showing]),
    [
      ['census', 'Coverage', 'what you hold, per instrument-month', 'one row per INSTRUMENT-MONTH, counted from /store.json'],
      ['bars', 'Bars', 'the stored values, one row per bar', 'Stored candles, one row at a time, shown in pages']
    ]
  );
  assert.match(VIEWS[0].title, /^What the store HOLDS\. One row per instrument-month, off \/store\.json/);
  assert.match(VIEWS[1].title, /^Inspect saved candle prices and volume/);
  assert.equal(dayWindowApplies, true);
});

test('the edges of the special list, the day count and the month span', () => {
  // A window's ends must be whole HH:MM inside a day, in order; a bad entry is dropped, never guessed.
  const read = specialsOf({
    special: [
      null,
      { date: 'x2024-11-01', windows: [{ from: '18:00', to: '18:59' }] },
      { date: '2024-11-01x', windows: [{ from: '18:00', to: '18:59' }] },
      { date: 20241101, windows: [{ from: '18:00', to: '18:59' }] },
      { date: '2024-11-04', kind: '', windows: [{ from: '18:00', to: '18:00' }] },
      { date: '2024-11-05', kind: 5, windows: [null, { from: '118:00', to: '18:59' }, { from: '18:00', to: '18:59x' }, { from: '24:00', to: '24:30' }, { from: '00:00', to: 'x' }, { from: '19:00', to: '18:00' }, { from: '18:30', to: '18:45' }] }
    ]
  });
  assert.deepEqual(read, [
    { date: '2024-11-04', kind: 'special session', windows: [{ from: 1080, to: 1080 }], halt: false, halted: [], reopening: [], suspected: false },
    { date: '2024-11-05', kind: 'special session', windows: [{ from: 1110, to: 1125 }], halt: false, halted: [], reopening: [], suspected: false }
  ]);
  assert.deepEqual(specialsOf(null), []);

  // The regular-day count, at every edge.
  const cal = { from: 0, days: 'RR.RRP-R' };
  assert.equal(prefixOf(cal), prefixOf(cal), 'built once per calendar and kept');
  assert.deepEqual([...prefixOf(cal).regular], [0, 1, 2, 2, 3, 4, 4, 4, 5]);
  assert.equal(regularDaysIn(cal, '1970-01-01', '1970-01-01'), 1, 'one day');
  assert.equal(regularDaysIn(cal, '1970-01-05', '1970-01-04'), 0, 'backwards by one day');
  assert.equal(regularDaysIn(cal, '1970-01-06', '1970-01-03'), 0, 'backwards over regular days');
  assert.equal(regularDaysIn(cal, '1970-01-01', '1970-01-06'), 4);
  assert.equal(regularDaysIn(cal, '1970-01-08', '1970-01-08'), 1, "after a '-' day, from the day after it");
  assert.equal(regularDaysIn(cal, '1970-01-07', '1970-01-08'), null, "a '-' day at the span's start");
  assert.equal(regularDaysIn(cal, '1970-01-01', '1970-01-07'), null, "a '-' day at the span's end");
  assert.equal(regularDaysIn(cal, '1970-01-01', '1970-01-09'), null, 'one day past the end');
  assert.equal(regularDaysIn(cal, 'garbage', ''), null, 'a day that is not YYYY-MM-DD');
  assert.equal(regularDaysIn(/** @type {any} */ ({ from: 0, days: 123 }), '', ''), null);

  // The month span at its edges.
  assert.deepEqual(monthDays('2024-01'), { from: '2024-01-01', to: '2024-01-31' });
  assert.deepEqual(monthDays('2024-12'), { from: '2024-12-01', to: '2024-12-31' });
  assert.equal(monthDays('x2024-03'), null);
  assert.equal(monthDays('2024-03x'), null);
  assert.equal(monthNeedsRead('2019-03', { from: '2019-03-01', to: '2019-03-31' }), false, 'a band of exactly the month');
  assert.equal(monthNeedsRead('2019-03', { from: '2019-03-01', to: '2019-03-15' }), true, 'a band ending mid-month');
  assert.equal(monthNeedsRead('2019-03', { from: '2019-03-02', to: '2019-03-31' }), true, 'a band starting after the 1st');
  assert.equal(flagText({ word: 'refused at import', says: ['a', 'b'] }), 'refused at import — a; b');
});

test('the edges of the special minutes and the list rows', () => {
  // A 60min rung's buckets start on 09:15 + 60k, so an 18:00–19:59 window fills three: 17:15 (its 18:00–18:14), 18:15 and 19:15.
  assert.deepEqual(specialMinutes([{ date: 'd', kind: 'k', windows: [{ from: 1080, to: 1199 }], halt: false }], 60, SESSION_OPEN_MINUTE).map((m) => m.key), ['17:15', '18:15', '19:15']);
  // A window on the grid starts at its own minute; one a minute past it starts a step earlier.
  assert.deepEqual(specialMinutes([{ date: 'd', kind: 'k', windows: [{ from: 1095, to: 1095 }], halt: false }], 60, SESSION_OPEN_MINUTE).map((m) => m.key), ['18:15']);
  assert.deepEqual(specialMinutes([{ date: 'd', kind: 'k', windows: [{ from: 1096, to: 1096 }], halt: false }], 60, SESSION_OPEN_MINUTE).map((m) => m.key), ['18:15']);
  assert.deepEqual(specialMinutes([{ date: 'd', kind: 'k', windows: [{ from: 1094, to: 1094 }], halt: false }], 60, SESSION_OPEN_MINUTE).map((m) => m.key), ['17:15']);
  // A day or wider rung has no intraday minute (its bar is stamped at IST midnight).
  assert.deepEqual(specialMinutes([{ date: 'd', kind: 'k', windows: [{ from: 1080, to: 1199 }], halt: false }], 1440, SESSION_OPEN_MINUTE), []);
  assert.deepEqual(specialMinutes([{ date: 'd', kind: 'k', windows: [{ from: 1080, to: 1199 }], halt: false }], 1439, SESSION_OPEN_MINUTE).map((m) => m.key), ['09:15']);
  // A bucket that began the day before carries that day's clock, never a negative one.
  assert.deepEqual(specialMinutes([{ date: 'd', kind: 'k', windows: [{ from: 5, to: 20 }], halt: false }], 60, SESSION_OPEN_MINUTE).map((m) => m.key), ['00:15', '23:15']);
  // Overlapping windows of one day name that day once; minutes come back in clock order whatever the input order.
  const a = { date: '2024-11-01', kind: 'a', windows: [{ from: 1095, to: 1096 }] };
  const b = { date: '2024-11-02', kind: 'b', windows: [{ from: 1080, to: 1083 }, { from: 1082, to: 1084 }] };
  const minutes = specialMinutes([a, b], 1, SESSION_OPEN_MINUTE);
  assert.deepEqual(minutes.map((m) => m.key), ['18:00', '18:01', '18:02', '18:03', '18:04', '18:15', '18:16']);
  assert.deepEqual(minutes.find((m) => m.key === '18:03')?.days, [b]);
  // The row's reason names each window, joined, and three days with no remainder.
  const outage = { date: '2021-02-24', kind: 'systems outage', windows: [{ from: 555, to: 699 }, { from: 945, to: 1019 }] };
  const rows = timeListRows(sessionGrid(60, ['16:00'], {}), { specials: [outage], regular: 0, held: [], step: 1, open: SESSION_OPEN_MINUTE });
  assert.equal(rows.find((r) => r.key === '16:00')?.why, 'systems outage, 24 Feb 2021: bars open 09:15–11:39 and 15:45–16:59 (the exchange calendar of record, /calendar.json). Listed because a day in view traded at this minute.');
  const day = (/** @type {string} */ date) => ({ date, kind: 'Muhurat', windows: [{ from: 1110, to: 1110 }] });
  const three = timeListRows(sessionGrid(60, ['18:30'], {}), { specials: [day('2019-01-01'), day('2019-01-02'), day('2019-01-03')], regular: null, held: [], step: 1, open: SESSION_OPEN_MINUTE });
  assert.equal(three.find((r) => r.key === '18:30')?.why, 'Muhurat, 01 Jan 2019: bars open 18:30–18:30; Muhurat, 02 Jan 2019: bars open 18:30–18:30; Muhurat, 03 Jan 2019: bars open 18:30–18:30 (the exchange calendar of record, /calendar.json). Listed because a day in view traded at this minute.');
  const four = timeListRows(sessionGrid(60, ['18:30'], {}), { specials: [day('2019-01-01'), day('2019-01-02'), day('2019-01-03'), day('2019-01-04')], regular: null, held: [], step: 1, open: SESSION_OPEN_MINUTE });
  assert.equal(four.find((r) => r.key === '18:30')?.why, 'Muhurat, 01 Jan 2019: bars open 18:30–18:30; Muhurat, 02 Jan 2019: bars open 18:30–18:30; Muhurat, 03 Jan 2019: bars open 18:30–18:30; and 1 more (the exchange calendar of record, /calendar.json). Listed because a day in view traded at this minute.');
  // A special minute that is a dated-close regular minute (it carries a reason) is the special day's.
  const late = timeListRows(sessionGrid(60, [], { venue: 'derivative', lo: '2026-08-03', hi: '2026-09-24' }), { specials: [{ date: '2026-08-08', kind: 'k', windows: [{ from: 935, to: 935 }] }], regular: 5, held: [], step: 1, open: SESSION_OPEN_MINUTE });
  assert.equal(late.find((r) => r.key === '15:35')?.detail, 'special session · 08 Aug 2026');
  assert.equal(late.find((r) => r.key === '15:00')?.detail, 'regular session');
});

/**
 * THE CALENDAR OF RECORD AS /calendar.json SERVES IT (feed=zerodha, every
 * intraday rung, read from a shadow of the store on 18 Oct 2026 for the DB-074
 * repair): each special or halt day's code, kind and windows, its `halts`
 * list, and the bars `owed` per rung (`calendar_record.rs` `owed_at`, which
 * folds the day's owed minutes through `pull::fold`). Exchange calendar facts
 * and this repository's own fold counts — no vendor row.
 */
const RECORD = {
  special: [
    ['2018-11-07', 'P', 'Muhurat session', [['17:30', '18:29']]],
    ['2019-10-27', 'P', 'Muhurat session', [['18:15', '19:14']]],
    ['2020-02-01', 'P', 'Budget weekend session', [['09:15', '15:29']]],
    ['2020-03-13', 'H', 'suspected halt', [['09:15', '15:29']]],
    ['2020-03-23', 'H', 'suspected halt', [['09:15', '15:29']]],
    ['2020-11-14', 'P', 'Muhurat session', [['18:15', '19:14']]],
    ['2021-02-24', 'P', 'systems outage', [['09:15', '11:39'], ['15:45', '16:59']]],
    ['2021-11-04', 'P', 'Muhurat session', [['18:15', '19:14']]],
    ['2022-10-24', 'P', 'Muhurat session', [['18:15', '19:14']]],
    ['2023-11-12', 'P', 'Muhurat session', [['18:15', '19:14']]],
    ['2024-01-20', 'P', 'weekend full session', [['09:15', '15:29']]],
    ['2024-03-02', 'P', 'disaster-recovery drill', [['09:15', '09:59'], ['11:30', '12:29']]],
    ['2024-05-18', 'P', 'disaster-recovery drill', [['09:15', '09:59'], ['11:30', '12:29']]],
    ['2024-11-01', 'P', 'Muhurat session', [['18:00', '18:59']]],
    ['2025-02-01', 'P', 'Budget weekend session', [['09:15', '15:29']]],
    ['2025-10-21', 'P', 'Muhurat session', [['13:45', '14:44']]],
    ['2026-02-01', 'P', 'Budget weekend session', [['09:15', '15:29']]]
  ].map(([date, code, kind, windows]) => ({ date, code, kind, windows: /** @type {string[][]} */ (windows).map(([from, to]) => ({ from, to })) })),
  halts: [
    // `pull::calendar::HALTS` as `/calendar.json` serves it: the halted minutes, the reopening call auction and
    // whether the record calls the halt suspected (D-3320; the outage day is SEBI-sourced).
    { date: '2020-03-13', halted: { from: '09:22', to: '10:20' }, reopening: { from: '10:06', to: '10:20' }, suspected: true },
    { date: '2020-03-23', halted: { from: '09:59', to: '10:56' }, reopening: { from: '10:42', to: '10:56' }, suspected: true },
    { date: '2021-02-24', halted: { from: '11:40', to: '15:44' }, reopening: { from: '15:30', to: '15:44' }, suspected: false }
  ],
  /** minutes per bar -> owed, in the order of `special` */
  owed: {
    1: [60, 60, 375, 316, 317, 60, 220, 60, 60, 60, 375, 105, 105, 60, 375, 60, 375],
    2: [31, 30, 188, 159, 159, 30, 111, 30, 30, 30, 188, 54, 54, 31, 188, 30, 188],
    3: [20, 20, 125, 106, 106, 20, 74, 20, 20, 20, 125, 35, 35, 20, 125, 20, 125],
    5: [12, 12, 75, 64, 64, 12, 44, 12, 12, 12, 75, 21, 21, 12, 75, 12, 75],
    10: [7, 6, 38, 33, 33, 6, 23, 6, 6, 6, 38, 12, 12, 7, 38, 6, 38],
    15: [4, 4, 25, 22, 22, 4, 15, 4, 4, 4, 25, 7, 7, 4, 25, 4, 25],
    30: [3, 2, 13, 12, 12, 2, 8, 2, 2, 2, 13, 5, 5, 3, 13, 2, 13],
    60: [2, 1, 7, 7, 7, 1, 5, 1, 1, 1, 7, 3, 3, 2, 7, 2, 7]
  }
};

test('every_special_day_lists_as_many_bars_as_the_calendar_owes', () => {
  const days = specialsOf(RECORD);
  assert.equal(days.length, 17);
  let pairs = 0;
  let halted1 = 0;
  for (const [step, owed] of Object.entries(RECORD.owed)) {
    const st = Number(step);
    days.forEach((d, i) => {
      // The day alone in view, as the page lists it: a halt day is one regular session in the day string.
      const keys = specialMinutes([d], st, SESSION_OPEN_MINUTE).map((m) => m.key);
      const rows = timeListRows(sessionGrid(st * 60, keys, { venue: 'index', lo: d.date, hi: d.date }), {
        specials: [d],
        regular: d.halt ? 1 : 0,
        held: [],
        step: st,
        open: SESSION_OPEN_MINUTE
      });
      assert.equal(keys.length, owed[i], `${st}min ${d.date}: the special minutes against owed`);
      // A halt day also lists the buckets only its halt holds, said as halted (neither owed nor missing,
      // D-3336 §1; repair round 2): counted here by an independent fold of the halted minutes.
      const traded = new Set();
      for (const w of d.windows) for (let m = w.from; m <= w.to; m += 1) traded.add(Math.floor((m - SESSION_OPEN_MINUTE) / st));
      const haltOnly = new Set();
      for (const h of d.halted ?? []) for (let m = h.from; m <= h.to; m += 1) {
        const b = Math.floor((m - SESSION_OPEN_MINUTE) / st);
        if (!traded.has(b)) haltOnly.add(b);
      }
      assert.equal(rows.length, owed[i] + haltOnly.size, `${st}min ${d.date}: the From time list against owed, plus the halt's own buckets`);
      assert.equal(rows.filter((r) => /^(halted|reopening auction) · /.test(r.detail)).length, haltOnly.size, `${st}min ${d.date}: each of those said as halted, or as its reopening auction`);
      assert.equal(rows.some((r) => /Nothing traded/.test(String(r.why))), false, `${st}min ${d.date}: a halted minute is never said to have traded nothing`);
      if (d.halted?.length && st === 1) halted1 += haltOnly.size;
      pairs += 1;
    });
  }
  assert.equal(pairs, 136, 'every special day at every intraday rung the calendar answers for');
  assert.equal(halted1, 59 + 58, '1min: 13 Mar 2020 halted 09:22–10:20 and 23 Mar 2020 09:59–10:56, all listed');

  // The halt is cut out of the day's own windows and kept as halted; the list says which minutes traded and
  // which were halted, and offers the halted ones as halted (both stores hold bars there; repair round 2).
  const halt = /** @type {any} */ (days.find((d) => d.date === '2020-03-13'));
  assert.deepEqual(halt.windows, [{ from: 555, to: 561 }, { from: 621, to: 929 }]);
  assert.deepEqual(halt.halted, [{ from: 562, to: 620 }]);
  assert.deepEqual(halt.reopening, [{ from: 606, to: 620 }], 'the reopening call auction, 10:06–10:20');
  assert.equal(halt.suspected, true);
  assert.equal(halt.halt, true);
  const haltRows = timeListRows(sessionGrid(60, [], { venue: 'index', lo: halt.date, hi: halt.date }), { specials: [halt], regular: 1, held: [], step: 1, open: SESSION_OPEN_MINUTE });
  assert.equal(haltRows.length, 375, '13 Mar 2020 alone: 316 traded minutes and 59 halted ones');
  assert.equal(haltRows[0].detail, 'halt day · 13 Mar 2020');
  const DAY13 = 'suspected halt, 13 Mar 2020: traded 09:15–09:21 and 10:21–15:29, halted 09:22–10:20 with the reopening call auction at 10:06–10:20 (the exchange calendar of record, /calendar.json).';
  assert.equal(haltRows[0].why, `${DAY13} Listed because a day in view traded at this minute.`);
  // REPAIR ROUND 3: a halted minute is said as halted, never "Nothing traded" — the record's own evidence has
  // shares outside the F&O list and stock options printing at 09:22 on 13 Mar 2020, and a halt labels, it never
  // removes (pull::calendar::Halt). The halt is a suspected one, said as the record says it (D-3320), and the
  // reopening call auction's minutes say so.
  const TAIL = 'A halted minute is neither owed nor missing (D-3336 §1), and a print inside a halt is still a bar (pull::calendar); listed so a bar the store holds here can still be reached.';
  const DOUBT = ' — the record calls it a suspected halt, UNVERIFIED (D-3320): no NSE circular is recorded.';
  const stopped = haltRows.filter((r) => r.detail === 'halted · 13 Mar 2020');
  assert.equal(stopped.length, 44);
  assert.equal(stopped[0].key, '09:22');
  assert.equal(stopped[43].key, '10:05');
  assert.equal(stopped[0].why, `${DAY13} Halted at this minute${DOUBT} ${TAIL}`);
  const auction = haltRows.filter((r) => r.detail === 'reopening auction · 13 Mar 2020');
  assert.deepEqual(auction.map((r) => r.key), Array.from({ length: 15 }, (_, i) => `10:${String(6 + i).padStart(2, '0')}`), '10:06–10:20');
  assert.equal(auction[0].why, `${DAY13} Halted at this minute, in the reopening call auction of 13 Mar 2020${DOUBT} ${TAIL}`);
  assert.equal(haltRows.some((r) => /Nothing traded/.test(String(r.why))), false);
  assert.equal(haltRows.find((r) => r.key === '10:21')?.detail, 'halt day · 13 Mar 2020', 'the first minute after the halt traded');
  // On a derived rung a bucket holding only halted minutes is said as halted, in its own bar's words.
  const halt2 = timeListRows(sessionGrid(120, [], { venue: 'index', lo: halt.date, hi: halt.date }), { specials: [halt], regular: 1, held: [], step: 2, open: SESSION_OPEN_MINUTE });
  const h923 = /** @type {any} */ (halt2.find((r) => r.key === '09:23'));
  assert.equal(h923.detail, 'halted · 13 Mar 2020');
  assert.ok(h923.why.includes('Halted inside the 2-minute bar stamped here (a bar carries the first minute of its bucket on the 09:15 grid, pull::fold) — the record calls it'), h923.why);
  assert.equal(halt2.find((r) => r.key === '10:05')?.detail, 'reopening auction · 13 Mar 2020', '10:05–10:06 holds an auction minute');
  assert.equal(halt2.find((r) => r.key === '10:03')?.detail, 'halted · 13 Mar 2020');
  assert.equal(halt2.find((r) => r.key === '09:21')?.detail, 'halt day · 13 Mar 2020', 'a bucket with a traded minute (09:21) is said as traded');
  // Both halt days in view: a minute both halted names both, counted; 10:10 is 13 Mar's auction but only
  // 23 Mar's halt, so it is said as halted and names the one day whose auction it was.
  const both = timeListRows(sessionGrid(60, [], {}), { specials: days.filter((d) => d.halt), regular: 2, held: [], step: 1, open: SESSION_OPEN_MINUTE });
  assert.equal(both.find((r) => r.key === '10:00')?.detail, 'halted · 2 days in view');
  const m1010 = /** @type {any} */ (both.find((r) => r.key === '10:10'));
  assert.equal(m1010.detail, 'halted · 2 days in view');
  assert.match(m1010.why, /Halted at this minute, in the reopening call auction of 13 Mar 2020 — the record calls it a suspected halt/);
  assert.equal(both.length, 375);
  // Two days whose auctions share a minute: said as the auction, with both days named.
  const twin = { ...halt, date: '2020-03-12' };
  const twins = timeListRows(sessionGrid(60, [], {}), { specials: [twin, halt], regular: 2, held: [], step: 1, open: SESSION_OPEN_MINUTE });
  assert.equal(twins.find((r) => r.key === '10:10')?.detail, 'reopening auction · 2 days in view');
  assert.match(String(twins.find((r) => r.key === '10:10')?.why), /in the reopening call auction of 12 Mar 2020 and 13 Mar 2020 — /);
  // A halt the record does not call suspected says no doubt it does not hold.
  const sure = timeListRows(sessionGrid(60, [], {}), { specials: [{ ...halt, suspected: false, reopening: [] }], regular: 1, held: [], step: 1, open: SESSION_OPEN_MINUTE });
  assert.equal(sure.find((r) => r.key === '09:30')?.why, 'suspected halt, 13 Mar 2020: traded 09:15–09:21 and 10:21–15:29, halted 09:22–10:20 (the exchange calendar of record, /calendar.json). Halted at this minute. ' + TAIL);
  assert.equal(sure.find((r) => r.key === '10:10')?.detail, 'halted · 13 Mar 2020', 'no auction recorded, none said');
  // A loaded bar inside the halt (a print the halt did not stop) is still listed.
  assert.ok(timeListRows(sessionGrid(60, ['09:22'], {}), { specials: [halt], regular: 1, held: ['09:22'], step: 1, open: SESSION_OPEN_MINUTE }).some((r) => r.key === '09:22'));
  // Beside a regular day the halted minutes are that day's regular minutes again.
  assert.equal(timeListRows(sessionGrid(60, [], {}), { specials: [halt], regular: 2, held: [], step: 1, open: SESSION_OPEN_MINUTE }).length, 375);
  // The outage day's windows already stop where its halt starts: the cut changes nothing.
  assert.deepEqual(days.find((d) => d.date === '2021-02-24')?.windows, [{ from: 555, to: 699 }, { from: 945, to: 1019 }]);
  assert.equal(days.find((d) => d.date === '2021-02-24')?.halt, false);
  // Without the halts list the halt day keeps its whole session (more minutes, never fewer).
  assert.equal(specialsOf({ special: RECORD.special }).find((d) => d.date === '2020-03-13')?.windows.length, 1);

  // An independent fold: the distinct buckets floor((m - open) / step) every minute of every window falls in.
  let seed = 7;
  const next = (/** @type {number} */ n) => {
    seed = (seed * 1_103_515_245 + 12_345) % 2_147_483_648;
    return seed % n;
  };
  for (let trial = 0; trial < 400; trial += 1) {
    const st = [1, 2, 3, 5, 10, 15, 30, 60][next(8)];
    const a = 540 + next(700);
    const b = a + next(120);
    const c = b + 2 + next(90);
    const windows = next(2) ? [{ from: a, to: b }] : [{ from: a, to: b }, { from: c, to: c + next(60) }];
    const buckets = new Set();
    for (const w of windows) for (let m = w.from; m <= w.to; m += 1) buckets.add(Math.floor((m - SESSION_OPEN_MINUTE) / st));
    assert.equal(specialMinutes([{ date: 'd', kind: 'k', windows, halt: false }], st, SESSION_OPEN_MINUTE).length, buckets.size, `${st}min ${JSON.stringify(windows)}`);
  }
});

test('a disabled time list says why on its button', () => {
  const base = { rows: 0, rungSecs: 60, regular: 0, specials: [], lo: '2024-11-02', hi: '2024-11-03' };
  assert.equal(timeListWhy({ ...base, rows: 1 }), '', 'a list holding a minute carries no refusal');
  assert.equal(timeListWhy(base), 'The days in view (02 Nov 2024 to 03 Nov 2024) hold no trading session in the exchange calendar of record (/calendar.json) and no loaded bar, so there is no minute to pick — move the dates onto a trading day.');
  assert.equal(timeListWhy({ ...base, lo: '' }), 'The days in view hold no trading session in the exchange calendar of record (/calendar.json) and no loaded bar, so there is no minute to pick — move the dates onto a trading day.');
  assert.equal(timeListWhy({ ...base, hi: '' }), timeListWhy({ ...base, lo: '' }));
  for (const rungSecs of [0, -60, NaN, undefined, 'x']) assert.match(timeListWhy({ ...base, rungSecs }), /^No timeframe is chosen, so there is no bar length to list minutes on/);
  // A chosen timeframe the page does not know is named, not said to be none (tf=60MIN; repair round 2).
  assert.equal(timeListWhy({ ...base, rung: '60MIN', rungSecs: undefined }), 'Timeframe "60MIN" is not a timeframe this page knows, so there is no bar length to list minutes on — pick one under Timeframe.');
  assert.equal(timeListWhy({ ...base, rung: '', rungSecs: undefined }), timeListWhy({ ...base, rungSecs: undefined }), 'every rung, no single one chosen');
  assert.match(timeListWhy({ ...base, rung: 'Z'.repeat(6000), rungSecs: undefined }), /^Timeframe "Z{24}"… \(6,000 characters\) is not a timeframe this page knows/);
  assert.equal(timeListWhy({ ...base, rung: '60MIN', rungSecs: 3600, rows: 1 }), '', 'a known rung holding a minute says nothing');
  // A halt day whose every minute halted: no session beyond the halted minutes.
  assert.match(timeListWhy({ ...base, regular: 1, specials: [{ date: '2024-11-02', kind: 'h', windows: [], halt: true }] }), /\(\/calendar\.json\) beyond halted minutes and no loaded bar/);
  // A case it cannot name is said to be one, by name.
  assert.equal(timeListWhy({ ...base, regular: 2 }), 'UNEXPLAINED: the time lists hold no minute for the days in view (02 Nov 2024 to 03 Nov 2024), although the calendar counts 2 regular session(s) there — a defect to report.');
  assert.match(timeListWhy({ ...base, regular: null }), /counts an unknown number of regular session\(s\)/);
  // WIRED on all three lists, from the same counts the lists are built with.
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.equal(controls.match(/title=\{db\.timeListTitle \|\| null\}\s*label="minutes"\s*disabled=\{db\.sessionRows\.length === 0\}/g)?.length, 3, 'From time, To time and Go to');
  const state = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(state, /const timeListTitle = \$derived\(\s*timeListWhy\(\{\s*rows: sessionRows\.length,\s*rung: timeframe,\s*rungSecs,\s*regular: regularDaysIn\(calendar, daysInView\.lo, daysInView\.hi\),\s*specials: specialsHere,/);
});

test('the edges of the halts list and the window cut', () => {
  // A halt this reader cannot read is left out, never guessed.
  const by = haltsOf([
    null,
    { date: 'x', halted: { from: '09:00', to: '09:10' } },
    { date: '2020-03-13', halted: { from: '09:22' } },
    { date: '2020-03-13', halted: { from: '10:20', to: '09:22' } },
    { date: '2020-03-13', halted: { from: '9:22', to: '10:20' } },
    { date: '2020-03-13', halted: { from: '09:22', to: '10:20' } },
    { date: '2020-03-13', halted: { from: '12:00', to: '12:00' } },
    { date: '2020-03-23' }
  ]);
  assert.deepEqual([...by.entries()], [['2020-03-13', [{ from: 562, to: 620, reopening: null, suspected: false }, { from: 720, to: 720, reopening: null, suspected: false }]]]);
  // The reopening auction and the suspected flag are read as the record writes them, and nothing else (repair round 3).
  const flags = haltsOf([
    { date: '2020-03-13', halted: { from: '09:22', to: '10:20' }, reopening: { from: '10:06', to: '10:20' }, suspected: true },
    { date: '2020-03-14', halted: { from: '09:22', to: '10:20' }, reopening: { from: '10:20', to: '10:06' }, suspected: 'true' },
    { date: '2020-03-15', halted: { from: '09:22', to: '10:20' }, reopening: { from: '10:06' }, suspected: 1 },
    { date: '2020-03-16', halted: { from: '09:22', to: '10:20' }, reopening: { to: '10:20' } },
    { date: '2020-03-17', halted: { from: '09:22', to: '10:20' }, reopening: { from: '10:20', to: '10:20' } },
    { date: '2020-03-18', halted: { from: '09:22', to: '10:20' }, reopening: { from: '00:00' } }
  ]);
  assert.deepEqual([...flags.values()].map((l) => l[0]), [
    { from: 562, to: 620, reopening: { from: 606, to: 620 }, suspected: true },
    { from: 562, to: 620, reopening: null, suspected: false },
    { from: 562, to: 620, reopening: null, suspected: false },
    { from: 562, to: 620, reopening: null, suspected: false },
    { from: 562, to: 620, reopening: { from: 620, to: 620 }, suspected: false },
    { from: 562, to: 620, reopening: null, suspected: false }
  ]);
  // The auction is kept only inside the session, like the halt; a suspected halt outside it suspects nothing.
  const early = specialsOf({ special: [{ date: '2020-03-13', code: 'H', kind: 'h', windows: [{ from: '09:15', to: '15:29' }] }], halts: [{ date: '2020-03-13', halted: { from: '09:00', to: '09:20' }, reopening: { from: '09:05', to: '09:16' }, suspected: true }] });
  assert.deepEqual([early[0].halted, early[0].reopening, early[0].suspected], [[{ from: 555, to: 560 }], [{ from: 555, to: 556 }], true]);
  const before = specialsOf({ special: [{ date: '2020-03-13', code: 'H', kind: 'h', windows: [{ from: '09:15', to: '15:29' }] }], halts: [{ date: '2020-03-13', halted: { from: '09:00', to: '09:10' }, reopening: { from: '09:05', to: '09:10' }, suspected: true }] });
  assert.deepEqual([before[0].halted, before[0].reopening, before[0].suspected], [[], [], false]);
  const aside = specialsOf({ special: [{ date: '2020-03-13', code: 'H', kind: 'h', windows: [{ from: '09:15', to: '15:29' }] }], halts: [{ date: '2020-03-13', halted: { from: '09:22', to: '10:20' }, reopening: { from: '10:30', to: '10:40' } }] });
  assert.deepEqual(aside[0].reopening, [], 'an auction outside its own halt is not inside the halted minutes');
  const last = specialsOf({ special: [{ date: '2020-03-13', code: 'H', kind: 'h', windows: [{ from: '09:15', to: '15:29' }] }], halts: [{ date: '2020-03-13', halted: { from: '09:22', to: '10:20' }, reopening: { from: '10:20', to: '10:20' } }] });
  assert.deepEqual(last[0].reopening, [{ from: 620, to: 620 }], 'a one-minute auction is kept');
  // A halt with no auction recorded, over two windows, keeps its part inside each.
  const across = specialsOf({ special: [{ date: '2021-02-24', code: 'P', kind: 'o', windows: [{ from: '09:15', to: '11:39' }, { from: '15:45', to: '16:59' }] }], halts: [{ date: '2021-02-24', halted: { from: '11:00', to: '16:00' } }] });
  assert.deepEqual([across[0].halted, across[0].reopening], [[{ from: 660, to: 699 }, { from: 945, to: 960 }], []]);
  assert.deepEqual(reopeningOf({ date: 'd', kind: 'k', windows: [] }), [], 'absent reads as none');
  assert.equal(haltsOf(undefined).size, 0);
  assert.equal(haltsOf('09:22').size, 0);
  // Two halts on one day both come out.
  const twice = specialsOf({ special: [{ date: '2020-03-13', code: 'H', kind: 'h', windows: [{ from: '09:15', to: '15:29' }] }], halts: [{ date: '2020-03-13', halted: { from: '09:22', to: '10:20' } }, { date: '2020-03-13', halted: { from: '12:00', to: '12:00' } }] });
  assert.deepEqual(twice[0].windows, [{ from: 555, to: 561 }, { from: 621, to: 719 }, { from: 721, to: 929 }]);
  assert.deepEqual(twice[0].halted, [{ from: 562, to: 620 }, { from: 720, to: 720 }], 'what is cut is kept, as halted');
  // A halt reaching outside the session keeps only its part inside it; one wholly outside keeps nothing.
  const out = specialsOf({ special: [{ date: '2021-02-24', code: 'P', kind: 'o', windows: [{ from: '09:15', to: '11:39' }, { from: '15:45', to: '16:59' }] }], halts: [{ date: '2021-02-24', halted: { from: '11:40', to: '15:44' } }] });
  assert.deepEqual(out[0].halted, [], 'a halt between two windows is not inside the session');
  const edge = specialsOf({ special: [{ date: '2020-03-13', code: 'H', kind: 'h', windows: [{ from: '09:15', to: '15:29' }] }], halts: [{ date: '2020-03-13', halted: { from: '09:00', to: '09:15' } }, { date: '2020-03-13', halted: { from: '15:29', to: '15:40' } }] });
  assert.deepEqual(edge[0].halted, [{ from: 555, to: 555 }, { from: 929, to: 929 }], 'touching either end of the session');
  assert.deepEqual(edge[0].windows, [{ from: 556, to: 928 }]);
  // A halt over the whole session leaves the day with no window, still counted as a halt day.
  const all = specialsOf({ special: [{ date: '2020-03-13', code: 'H', kind: 'h', windows: [{ from: '09:15', to: '15:29' }] }], halts: [{ date: '2020-03-13', halted: { from: '09:00', to: '16:00' } }] });
  assert.deepEqual(all, [{ date: '2020-03-13', kind: 'h', windows: [], halt: true, halted: [{ from: 555, to: 929 }], reopening: [], suspected: false }]);
  // A halt over the whole session: every minute is offered, each said as halted, none as traded.
  const allRows = timeListRows(sessionGrid(60, [], {}), { specials: all, regular: 1, held: [], step: 1, open: SESSION_OPEN_MINUTE });
  assert.equal(allRows.length, 375);
  assert.equal(allRows.filter((r) => r.detail === 'halted · 13 Mar 2020').length, 375);
  assert.equal(allRows.some((r) => /Listed because a day in view traded/.test(String(r.why))), false);
  // Without halted ranges a day whose windows are empty offers nothing.
  assert.equal(timeListRows(sessionGrid(60, [], {}), { specials: [{ ...all[0], halted: [] }], regular: 1, held: [], step: 1, open: SESSION_OPEN_MINUTE }).length, 0, 'nothing traded or halted, nothing offered');
  assert.deepEqual(haltedOf({ date: 'd', kind: 'k', windows: [] }), [], 'absent reads as none');
  // The cut at each edge: before, after, touching either end, inside, covering.
  const w = [{ from: 100, to: 200 }];
  assert.deepEqual(cutWindows(w, { from: 50, to: 99 }), w);
  assert.deepEqual(cutWindows(w, { from: 201, to: 300 }), w);
  assert.deepEqual(cutWindows(w, { from: 50, to: 100 }), [{ from: 101, to: 200 }]);
  assert.deepEqual(cutWindows(w, { from: 200, to: 300 }), [{ from: 100, to: 199 }]);
  assert.deepEqual(cutWindows(w, { from: 150, to: 150 }), [{ from: 100, to: 149 }, { from: 151, to: 200 }]);
  assert.deepEqual(cutWindows(w, { from: 100, to: 200 }), []);
  assert.deepEqual(cutWindows([], { from: 100, to: 200 }), []);
  // An unknown regular count keeps the regular grid whatever halt days are in view.
  assert.equal(timeListRows(sessionGrid(60, [], {}), { specials: [{ date: 'd', kind: 'h', windows: [{ from: 555, to: 556 }], halt: true }], regular: null, held: [], step: 1, open: SESSION_OPEN_MINUTE }).length, 375, 'unknown keeps the regular grid');
});

test('the edges of the shortfall fold and the hidden clause', () => {
  /** @param {string} underlying @param {string} kind @param {string} month @param {string} timeframe @param {number} rows @param {any} [expiry] */
  const row = (underlying, kind, month, timeframe, rows, expiry = null) => ({ underlying, kind, month, timeframe, rows, expiry });
  const census = [
    row('A', 'INDEX', '2020-01', '1min', 7_000),
    // The fuller index comes LATER, and a spot row may carry no expiry field at all.
    row('B', 'INDEX', '2020-01', '1min', 7_500, undefined),
    // A contract is never the fullest, even one mis-segmented as CASH.
    row('C', 'CASH', '2020-01', '1min', 9_999, '2020-01-30'),
    row('D', 'CASH', '2020-01', '1min', 7_400),
    row('A', 'INDEX', '2020-02', '1min', 6_000),
    row('B', 'INDEX', '2020-02', '1min', 6_100),
    // Two shares at 1s: the trade-driven rung is never short, whatever the fullest.
    row('S', 'CASH', '2020-01', '1s', 100),
    row('T', 'CASH', '2020-01', '1s', 90_000),
    // A contract at a rung its underlying's spot lacks is not folded into the underlying.
    row('A', 'FNO', '2020-01', '1day', 20, '2020-01-30')
  ];
  const by = spotShortfalls(census, census);
  assert.deepEqual(by.get('A')?.get('1min'), { months: 2, short: 600, trade: false, seg: 'INDEX' }, 'two months, against the later fuller index');
  assert.equal(by.get('A')?.has('1day'), false);
  assert.deepEqual(by.get('B')?.get('1min'), { months: 2, short: 0, trade: false, seg: 'INDEX' });
  assert.deepEqual(by.get('D')?.get('1min'), { months: 1, short: 0, trade: false, seg: 'CASH' }, 'the CASH contract is not the fullest share');
  assert.deepEqual(by.get('S')?.get('1s'), { months: 1, short: 0, trade: true, seg: 'CASH' });
  assert.equal(by.has('C'), false);
  // A row with no segment field is folded under an empty segment, said as nothing.
  assert.equal(spotShortfalls([], [{ underlying: 'X', month: '2020-01', timeframe: '1min', rows: 1, expiry: null }]).get('X')?.get('1min')?.seg, '');
  assert.equal(hiddenText(3, { from: '2018-01-01' }, fmt), ' 3 more month(s) are held outside your window and hidden — "Show them, labelled" under the dates brings them in.');
});
