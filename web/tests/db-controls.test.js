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
import {
  specialsOf,
  specialsIn,
  specialMinutes,
  regularDaysIn,
  timeListRows,
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
  shortfallText,
  hiddenMonths,
  hiddenText,
  strandedEffect,
  wholeTitle
} from '../src/lib/db/notes-controls.js';
import { sessionGrid, SESSION_OPEN_MINUTE, calendarOf } from '../src/lib/db/selection-model.js';
import { bandOf } from '../src/lib/db/dayflags.js';
import { tfCmp } from '../src/lib/db/notes-labels.js';

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
  assert.deepEqual(specialMinutes([{ date: '2024-11-01', kind: 'x', windows: [{ from: 1082, to: 1094 }] }], 5, SESSION_OPEN_MINUTE).map((m) => m.key), ['18:05', '18:10']);
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
  assert.match(state, /const specialsHere = \$derived\(specialsIn\(specialsOf\(\{ special: calSpecial \}\), daysInView\.lo, daysInView\.hi\)\);/);
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
  assert.match(state, /while \(next\.size > MONTH_FLAGS_KEPT\) next\.delete/);
  const day = dbSource('../src/lib/DayField.svelte');
  assert.ok(day.includes("if (open && pad === 'day' && /^\\d{4}-\\d{2}$/.test(shown)) onshow?.(shown);"));
  assert.match(day, /\{#if flagged && flagWhy && pad === 'day' && flagWhy\(shown\)\}/);
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
  assert.deepEqual(by.get('SMALLIDX')?.get('1s'), { months: 1, short: 47, trade: false }, 'short of the fullest INDEX, not of every series');
  assert.deepEqual(by.get('SMALLIDX')?.get('1min'), { months: 1, short: 0, trade: false }, "a share's pre-open minute does not make an index short");
  assert.deepEqual(by.get('OTHER')?.get('1min'), { months: 1, short: 30, trade: false }, 'a share is short of the fullest share');
  assert.deepEqual(by.get('SHARE')?.get('1s'), { months: 1, short: 0, trade: true }, 'a share at 1s is trade-driven');
  assert.equal(by.get('BIGIDX')?.size, 2, 'the contract month is not folded into its underlying');
  assert.equal(shortfallText(by.get('SMALLIDX'), 'INDEX', tfCmp, fmt), ' (1s: 47 bar(s) short of the fullest INDEX series of each month)');
  assert.equal(shortfallText(by.get('SHARE'), 'CASH', tfCmp, fmt), ' (1s: trade-driven, a bar only where a trade printed, so no fixed bar count)');
  assert.equal(shortfallText(by.get('BIGIDX'), 'INDEX', tfCmp, fmt), '');
  assert.equal(shortfallText(undefined, 'INDEX', tfCmp, fmt), '');
  // Several rungs short: listed in ladder order, never summed into one figure.
  const two = spotShortfalls(census, [row('NSE-INDEX-SMALLIDX', '2019-06', '1min', 7_000), row('NSE-INDEX-SMALLIDX', '2019-06', '1s', 400_000)]);
  assert.equal(shortfallText(two.get('SMALLIDX'), 'INDEX', tfCmp, fmt), ' (1s: 5,647 bar(s) short of the fullest INDEX series of each month; 1min: 875 bar(s) short of the fullest INDEX series of each month)');
  // A month no other series shares measures against itself.
  assert.equal(spotShortfalls([], [row('NSE-INDEX-LONE', '2020-01', '1min', 10)]).get('LONE')?.get('1min')?.short, 0);
  assert.equal(tradeDriven({ segment: 'CASH', timeframe: '1s' }), true);
  assert.equal(tradeDriven({ kind: 'FNO', timeframe: '1min' }), true);
  assert.equal(tradeDriven({ timeframe: '1min' }), false);
  assert.equal(tradeDriven({ kind: 'INDEX', timeframe: '1s', expiry: '2020-01-30' }), true);

  // WIRED: the Instrument row's title says the per-rung, per-segment figure — not the census's pooled `short`.
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.match(controls, /spot month\(s\)\$\{shortfallText\(r\.shortBy, r\.seg, tfCmp, fmt\)\}/);
  assert.doesNotMatch(controls, /bar\(s\) short of the fullest series\)/);
  assert.match(dbSource('../src/lib/db/selection.svelte.js'), /const shortfalls = \$derived\(spotShortfalls\(db\.deco, universed\)\);/);
});

test('a stranded rung says "no row can match" only when no row does (DB-020)', () => {
  assert.equal(strandedEffect(0), 'so no row can match.');
  // A stale expiry under Spot, over a full grid.
  assert.equal(strandedEffect(43_942_544, 0, fmt), "so it narrows nothing on screen: the table's 4,39,42,544 row(s) do not carry this rung.");
  // Two strikes picked, one off this expiry's chain: the other still matches.
  assert.equal(strandedEffect(8, 1, fmt), 'so it matches nothing; the table shows the 1 other value(s) still chosen on this rung.');
  assert.equal(strandedEffect(5, 0), "so it narrows nothing on screen: the table's 5 row(s) do not carry this rung.");
  const state = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(state, /effect: strandedEffect\(st\.empty \? 0 : matched\.length, st\.kept \?\? 0, fmt\)/);
  assert.match(state, /kept: strikePick\.size - gone\.length,/);
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.match(controls, /which the rungs above it no longer offer —\s*\{st\.effect\}/);
  assert.doesNotMatch(controls, /no longer offer —\s*so no row can match\./);
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
  // WIRED: both lists carry the count; a rung held only outside the window is listed and refused.
  const state = dbSource('../src/lib/db/selection.svelte.js');
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
  assert.match(controls, /:global\(\.strip\) \.field > \.note \{[\s\S]*?line-clamp: 2;/);
});
